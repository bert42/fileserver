use crate::client::FileServerClient;
use common::{FileServerError, FileEntry, FileMetadata, HealthStatus};

pub struct FileOperations {
    client: FileServerClient,
}

impl FileOperations {
    pub fn new(client: FileServerClient) -> Self {
        Self { client }
    }

    pub async fn connect(&mut self) -> Result<(), FileServerError> {
        let response = self.client.authenticate().await?;
        
        if response.success {
            println!("✓ Connected to server successfully");
            println!("  Message: {}", response.message);
            println!("  Available directories:");
            for dir in response.available_directories {
                println!("    - {}", dir);
            }
        } else {
            return Err(FileServerError::ConnectionFailed(response.message));
        }
        
        Ok(())
    }

    pub async fn health_check(&mut self) -> Result<HealthStatus, FileServerError> {
        let status = self.client.health_check().await?;
        
        println!("Server Health Check:");
        println!("  Status: {}", if status.healthy { "Healthy" } else { "Unhealthy" });
        println!("  Uptime: {} seconds", status.uptime_seconds);
        println!("  Version: {}", status.version);
        println!("  Message: {}", status.message);
        
        Ok(status)
    }

    pub async fn stat(&mut self, path: &str) -> Result<FileMetadata, FileServerError> {
        let metadata = self.client.stat(path).await?;
        
        println!("File Information for '{}':", path);
        println!("  Name: {}", metadata.name);
        println!("  Size: {} bytes", metadata.size);
        println!("  Type: {}", if metadata.is_directory { "Directory" } else { "File" });
        println!("  Permissions: {}", metadata.permissions);
        
        let modified = std::time::UNIX_EPOCH + std::time::Duration::from_secs(metadata.modified_time as u64);
        let created = std::time::UNIX_EPOCH + std::time::Duration::from_secs(metadata.created_time as u64);
        
        let modified_datetime = chrono::DateTime::<chrono::Utc>::from(modified);
        let created_datetime = chrono::DateTime::<chrono::Utc>::from(created);
        
        println!("  Modified: {}", modified_datetime.format("%Y-%m-%d %H:%M:%S UTC"));
        println!("  Created: {}", created_datetime.format("%Y-%m-%d %H:%M:%S UTC"));
        
        Ok(metadata)
    }

    pub async fn list(&mut self, path: &str) -> Result<Vec<FileEntry>, FileServerError> {
        let entries = self.client.list(path).await?;
        
        println!("Directory listing for '{}':", path);
        println!("{:<30} {:<10} {:<15} {}", "Name", "Type", "Size", "Modified");
        println!("{}", "-".repeat(70));
        
        for entry in &entries {
            let file_type = if entry.is_directory { "Directory" } else { "File" };
            let size = if entry.is_directory {
                "-".to_string()
            } else {
                format!("{} bytes", entry.size)
            };
            
            let modified = std::time::UNIX_EPOCH + std::time::Duration::from_secs(entry.modified_time as u64);
            let datetime = chrono::DateTime::<chrono::Utc>::from(modified);
            let modified_str = datetime.format("%Y-%m-%d %H:%M").to_string();
            
            println!("{:<30} {:<10} {:<15} {}", 
                entry.name, 
                file_type, 
                size,
                modified_str
            );
        }
        
        Ok(entries)
    }

    pub async fn read(&mut self, path: &str) -> Result<Vec<u8>, FileServerError> {
        let data = self.client.read(path).await?;
        
        println!("Read {} bytes from '{}'", data.len(), path);
        
        Ok(data)
    }

    pub async fn read_text(&mut self, path: &str) -> Result<String, FileServerError> {
        let data = self.client.read(path).await?;
        
        match String::from_utf8(data) {
            Ok(text) => {
                println!("File content ({})", path);
                println!("{}", "-".repeat(50));
                println!("{}", text);
                Ok(text)
            }
            Err(_) => {
                println!("File '{}' contains binary data", path);
                Err(FileServerError::InvalidPath("File contains binary data".to_string()))
            }
        }
    }

    pub async fn write(&mut self, path: &str, content: &str) -> Result<(), FileServerError> {
        let response = self.client.write_text(path, content).await?;
        
        if response.success {
            println!("✓ Successfully wrote {} bytes to '{}'", response.bytes_written, path);
            println!("  Message: {}", response.message);
        } else {
            return Err(FileServerError::IoError(
                std::io::Error::new(std::io::ErrorKind::Other, response.message)
            ));
        }
        
        Ok(())
    }

    pub async fn write_file(&mut self, path: &str, file_path: &str) -> Result<(), FileServerError> {
        let content = std::fs::read_to_string(file_path)
            .map_err(|e| FileServerError::IoError(e))?;
        
        self.write(path, &content).await
    }

    pub async fn delete(&mut self, path: &str) -> Result<(), FileServerError> {
        let response = self.client.delete(path).await?;
        
        if response.success {
            println!("✓ Successfully deleted '{}'", path);
            println!("  Message: {}", response.message);
        } else {
            return Err(FileServerError::IoError(
                std::io::Error::new(std::io::ErrorKind::Other, response.message)
            ));
        }
        
        Ok(())
    }
}