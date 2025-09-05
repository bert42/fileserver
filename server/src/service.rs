use crate::auth::AuthService;
use crate::file_handler::FileHandler;
use common::*;
use std::path::Path;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::mpsc;
use tokio_stream::{wrappers::ReceiverStream, StreamExt};
use tonic::{Request, Response, Status, Streaming};

pub struct FileServiceImpl {
    auth: Arc<AuthService>,
    file_handler: Arc<FileHandler>,
    start_time: SystemTime,
}

impl FileServiceImpl {
    pub fn new(auth: AuthService) -> Self {
        Self {
            auth: Arc::new(auth),
            file_handler: Arc::new(FileHandler::new()),
            start_time: SystemTime::now(),
        }
    }

    fn parse_path(&self, path: &str) -> Result<(String, String), Status> {
        if path.is_empty() {
            return Err(Status::invalid_argument("Path cannot be empty"));
        }

        let parts: Vec<&str> = path.splitn(2, '/').collect();
        let directory_name = parts[0].to_string();
        let file_path = if parts.len() > 1 {
            parts[1].to_string()
        } else {
            String::new()
        };

        Ok((directory_name, file_path))
    }

    fn resolve_full_path(&self, directory_name: &str, file_path: &str, operation: &str) -> Result<std::path::PathBuf, Status> {
        self.auth.validate_path(file_path)
            .map_err(|e| Status::invalid_argument(e.to_string()))?;

        let base_path = self.auth.check_directory_access(directory_name, operation)
            .map_err(|e| Status::permission_denied(e.to_string()))?;

        let full_path = Path::new(&base_path).join(file_path);
        
        if !full_path.starts_with(&base_path) {
            return Err(Status::permission_denied("Path traversal attempt detected"));
        }

        Ok(full_path)
    }
}

#[tonic::async_trait]
impl file_service_server::FileService for FileServiceImpl {
    async fn authenticate(&self, request: Request<ConnectRequest>) -> Result<Response<ConnectResponse>, Status> {
        self.auth.authorize_connection(&Request::new(()))?;

        let req = request.into_inner();
        tracing::info!("Client {} connected", req.client_id);

        let auth = Arc::clone(&self.auth);
        let available_directories: Vec<String> = auth.config.directories
            .iter()
            .map(|d| d.name.clone())
            .collect();

        let response = ConnectResponse {
            success: true,
            message: "Connection established successfully".to_string(),
            available_directories,
        };

        Ok(Response::new(response))
    }

    async fn health_check(&self, _request: Request<Empty>) -> Result<Response<HealthStatus>, Status> {
        let uptime = self.start_time
            .elapsed()
            .unwrap_or_default()
            .as_secs() as i64;

        let response = HealthStatus {
            healthy: true,
            uptime_seconds: uptime,
            version: env!("CARGO_PKG_VERSION").to_string(),
            message: "Server is healthy".to_string(),
        };

        Ok(Response::new(response))
    }

    async fn stat(&self, request: Request<StatRequest>) -> Result<Response<FileMetadata>, Status> {
        let req = request.into_inner();
        let (directory_name, file_path) = self.parse_path(&req.path)?;
        let full_path = self.resolve_full_path(&directory_name, &file_path, "read")?;

        let metadata = self.file_handler.stat(&full_path).await
            .map_err(|e| Status::not_found(e.to_string()))?;

        Ok(Response::new(metadata))
    }

    async fn list(&self, request: Request<ListRequest>) -> Result<Response<ListResponse>, Status> {
        let req = request.into_inner();
        let (directory_name, file_path) = self.parse_path(&req.path)?;
        let full_path = self.resolve_full_path(&directory_name, &file_path, "read")?;

        let entries = self.file_handler.list_directory(&full_path).await
            .map_err(|e| Status::invalid_argument(e.to_string()))?;

        let response = ListResponse { entries };
        Ok(Response::new(response))
    }

    type ReadStream = ReceiverStream<Result<DataChunk, Status>>;

    async fn read(&self, request: Request<ReadRequest>) -> Result<Response<Self::ReadStream>, Status> {
        let req = request.into_inner();
        let (directory_name, file_path) = self.parse_path(&req.path)?;
        let full_path = self.resolve_full_path(&directory_name, &file_path, "read")?;

        let (tx, rx) = mpsc::channel(4);
        let file_handler = Arc::clone(&self.file_handler);
        let path_clone = req.path.clone();

        tokio::spawn(async move {
            const CHUNK_SIZE: usize = 64 * 1024; // 64KB chunks
            
            match file_handler.read_file(&full_path, req.offset, req.length).await {
                Ok(data) => {
                    let mut offset = req.offset.unwrap_or(0);
                    
                    for chunk in data.chunks(CHUNK_SIZE) {
                        let is_last = chunk.len() < CHUNK_SIZE;
                        let data_chunk = DataChunk {
                            path: path_clone.clone(),
                            data: chunk.to_vec(),
                            offset,
                            is_last,
                        };
                        
                        if tx.send(Ok(data_chunk)).await.is_err() {
                            break;
                        }
                        
                        offset += chunk.len() as u64;
                        
                        if is_last {
                            break;
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(Err(Status::internal(e.to_string()))).await;
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn write(&self, request: Request<Streaming<DataChunk>>) -> Result<Response<WriteResponse>, Status> {
        let mut stream = request.into_inner();
        let total_bytes;
        let mut current_path = String::new();
        let mut buffer = Vec::new();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            
            if current_path.is_empty() {
                current_path = chunk.path.clone();
            } else if current_path != chunk.path {
                return Err(Status::invalid_argument("All chunks must have the same path"));
            }

            buffer.extend_from_slice(&chunk.data);
            
            if chunk.is_last {
                break;
            }
        }

        if current_path.is_empty() {
            return Err(Status::invalid_argument("No data received"));
        }

        let (directory_name, file_path) = self.parse_path(&current_path)?;
        let full_path = self.resolve_full_path(&directory_name, &file_path, "write")?;

        total_bytes = self.file_handler.write_file(&full_path, &buffer, None).await
            .map_err(|e| Status::internal(e.to_string()))?;

        let response = WriteResponse {
            success: true,
            message: "File written successfully".to_string(),
            bytes_written: total_bytes,
        };

        Ok(Response::new(response))
    }
}