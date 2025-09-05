use crate::config::ClientConfig;
use common::{file_service_client::FileServiceClient, *};
use std::time::Duration;
use tokio_stream::StreamExt;
use tonic::transport::Channel;
use tonic::Request;

pub struct FileServerClient {
    client: FileServiceClient<Channel>,
    client_id: String,
}

impl FileServerClient {
    pub async fn new(config: ClientConfig, client_id: String) -> Result<Self, FileServerError> {
        let endpoint = config.server_address();
        let channel = Channel::from_shared(endpoint)
            .map_err(|e| FileServerError::ConnectionFailed(e.to_string()))?
            .timeout(Duration::from_secs(config.client.timeout_seconds))
            .connect()
            .await
            .map_err(|e| FileServerError::ConnectionFailed(e.to_string()))?;

        let client = FileServiceClient::new(channel);

        Ok(Self {
            client,
            client_id,
        })
    }

    pub async fn authenticate(&mut self) -> Result<ConnectResponse, FileServerError> {
        let request = Request::new(ConnectRequest {
            client_id: self.client_id.clone(),
        });

        let response = self.client.authenticate(request).await?;
        Ok(response.into_inner())
    }

    pub async fn health_check(&mut self) -> Result<HealthStatus, FileServerError> {
        let request = Request::new(Empty {});
        let response = self.client.health_check(request).await?;
        Ok(response.into_inner())
    }

    pub async fn stat(&mut self, path: &str) -> Result<FileMetadata, FileServerError> {
        let request = Request::new(StatRequest {
            path: path.to_string(),
        });

        let response = self.client.stat(request).await?;
        Ok(response.into_inner())
    }

    pub async fn list(&mut self, path: &str) -> Result<Vec<FileEntry>, FileServerError> {
        let request = Request::new(ListRequest {
            path: path.to_string(),
        });

        let response = self.client.list(request).await?;
        Ok(response.into_inner().entries)
    }

    pub async fn read(&mut self, path: &str) -> Result<Vec<u8>, FileServerError> {
        let request = Request::new(ReadRequest {
            path: path.to_string(),
            offset: None,
            length: None,
        });

        let mut stream = self.client.read(request).await?.into_inner();
        let mut data = Vec::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            data.extend_from_slice(&chunk.data);
            
            if chunk.is_last {
                break;
            }
        }

        Ok(data)
    }


    pub async fn write(&mut self, path: &str, data: &[u8]) -> Result<WriteResponse, FileServerError> {
        let chunk_size = 64 * 1024; // 64KB chunks
        let chunks: Vec<_> = data
            .chunks(chunk_size)
            .enumerate()
            .map(|(i, chunk)| {
                let is_last = (i + 1) * chunk_size >= data.len();
                DataChunk {
                    path: path.to_string(),
                    data: chunk.to_vec(),
                    offset: (i * chunk_size) as u64,
                    is_last,
                }
            })
            .collect();

        let stream = tokio_stream::iter(chunks);
        let request = Request::new(stream);

        let response = self.client.write(request).await?;
        Ok(response.into_inner())
    }

    pub async fn write_text(&mut self, path: &str, text: &str) -> Result<WriteResponse, FileServerError> {
        self.write(path, text.as_bytes()).await
    }

    pub async fn delete(&mut self, path: &str) -> Result<DeleteResponse, FileServerError> {
        let request = Request::new(DeleteRequest {
            path: path.to_string(),
        });

        let response = self.client.delete(request).await?;
        Ok(response.into_inner())
    }
}