use common::{FileServerError, FileMetadata, FileEntry};
use std::path::Path;
use tokio::fs as async_fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt, AsyncSeekExt};

pub struct FileHandler;

impl FileHandler {
    pub fn new() -> Self {
        Self
    }

    pub async fn stat(&self, full_path: &Path) -> Result<FileMetadata, FileServerError> {
        let metadata = async_fs::metadata(full_path).await?;
        
        let name = full_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        let modified_time = metadata.modified()?
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let created_time = metadata.created()
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        Ok(FileMetadata {
            name,
            size: metadata.len(),
            is_directory: metadata.is_dir(),
            permissions: if metadata.is_dir() { "dir".to_string() } else { "file".to_string() },
            modified_time,
            created_time,
        })
    }

    pub async fn list_directory(&self, full_path: &Path) -> Result<Vec<FileEntry>, FileServerError> {
        if !full_path.is_dir() {
            return Err(FileServerError::InvalidPath("Path is not a directory".to_string()));
        }

        let mut entries = Vec::new();
        let mut dir = async_fs::read_dir(full_path).await?;

        while let Some(entry) = dir.next_entry().await? {
            let metadata = entry.metadata().await?;
            let name = entry.file_name().to_string_lossy().to_string();

            let modified_time = metadata.modified()?
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;

            entries.push(FileEntry {
                name,
                is_directory: metadata.is_dir(),
                size: metadata.len(),
                modified_time,
                permissions: if metadata.is_dir() { "dir".to_string() } else { "file".to_string() },
            });
        }

        entries.sort_by(|a, b| {
            match (a.is_directory, b.is_directory) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            }
        });

        Ok(entries)
    }

    pub async fn read_file(&self, full_path: &Path, offset: Option<u64>, length: Option<u64>) -> Result<Vec<u8>, FileServerError> {
        if !full_path.is_file() {
            return Err(FileServerError::InvalidPath("Path is not a file".to_string()));
        }

        let mut file = async_fs::File::open(full_path).await?;
        
        let file_size = file.metadata().await?.len();
        let start = offset.unwrap_or(0);
        let end = length.map(|len| start + len).unwrap_or(file_size);

        if start >= file_size {
            return Ok(Vec::new());
        }

        let actual_end = end.min(file_size);
        let bytes_to_read = (actual_end - start) as usize;

        let mut buffer = vec![0u8; bytes_to_read];
        file.seek(std::io::SeekFrom::Start(start)).await?;
        file.read_exact(&mut buffer).await?;

        Ok(buffer)
    }

    pub async fn write_file(&self, full_path: &Path, data: &[u8], offset: Option<u64>) -> Result<u64, FileServerError> {
        if let Some(parent) = full_path.parent() {
            async_fs::create_dir_all(parent).await?;
        }

        let mut file = if offset.is_some() && full_path.exists() {
            async_fs::OpenOptions::new()
                .write(true)
                .open(full_path).await?
        } else {
            async_fs::File::create(full_path).await?
        };

        if let Some(pos) = offset {
            file.seek(std::io::SeekFrom::Start(pos)).await?;
        }

        file.write_all(data).await?;
        file.sync_all().await?;

        Ok(data.len() as u64)
    }

    pub async fn delete_file(&self, full_path: &Path) -> Result<(), FileServerError> {
        if !full_path.exists() {
            return Err(FileServerError::FileNotFound(
                full_path.to_string_lossy().to_string()
            ));
        }

        if full_path.is_file() {
            async_fs::remove_file(full_path).await?;
        } else if full_path.is_dir() {
            async_fs::remove_dir_all(full_path).await?;
        } else {
            return Err(FileServerError::InvalidPath(
                "Path is neither a file nor a directory".to_string()
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tokio::fs as async_fs;
    use uuid::Uuid;

    async fn create_test_environment() -> std::path::PathBuf {
        let test_dir = std::env::temp_dir().join(format!("fileserver_handler_test_{}", Uuid::now_v7()));
        if test_dir.exists() {
            async_fs::remove_dir_all(&test_dir).await.ok();
        }
        async_fs::create_dir_all(&test_dir).await.unwrap();

        // Create test files and directories
        let test_file = test_dir.join("test_file.txt");
        let test_subdir = test_dir.join("subdir");
        let subdir_file = test_subdir.join("nested_file.txt");

        async_fs::write(&test_file, "Hello, World!").await.unwrap();
        async_fs::create_dir_all(&test_subdir).await.unwrap();
        async_fs::write(&subdir_file, "Nested content").await.unwrap();

        test_dir
    }

    async fn cleanup_test_environment(test_dir: &Path) {
        fs::remove_dir_all(test_dir).ok();
    }

    #[tokio::test]
    async fn test_stat_file() {
        let test_dir = create_test_environment().await;
        let handler = FileHandler::new();
        let test_file = test_dir.join("test_file.txt");

        let result = handler.stat(&test_file).await;
        assert!(result.is_ok());

        let metadata = result.unwrap();
        assert_eq!(metadata.name, "test_file.txt");
        assert!(!metadata.is_directory);
        assert_eq!(metadata.size, 13); // "Hello, World!" length
        assert!(metadata.modified_time > 0);
        assert!(metadata.created_time > 0);

        cleanup_test_environment(&test_dir).await;
    }

    #[tokio::test]
    async fn test_stat_directory() {
        let test_dir = create_test_environment().await;
        let handler = FileHandler::new();
        let test_subdir = test_dir.join("subdir");

        let result = handler.stat(&test_subdir).await;
        assert!(result.is_ok());

        let metadata = result.unwrap();
        assert_eq!(metadata.name, "subdir");
        assert!(metadata.is_directory);

        cleanup_test_environment(&test_dir).await;
    }

    #[tokio::test]
    async fn test_stat_nonexistent_file() {
        let test_dir = create_test_environment().await;
        let handler = FileHandler::new();
        let nonexistent = test_dir.join("nonexistent.txt");

        let result = handler.stat(&nonexistent).await;
        assert!(result.is_err());

        cleanup_test_environment(&test_dir).await;
    }

    #[tokio::test]
    async fn test_list_directory() {
        let test_dir = create_test_environment().await;
        let handler = FileHandler::new();

        let result = handler.list_directory(&test_dir).await;
        assert!(result.is_ok());

        let entries = result.unwrap();
        assert_eq!(entries.len(), 2); // test_file.txt and subdir

        // Check directory comes first (sorted)
        assert!(entries[0].is_directory);
        assert_eq!(entries[0].name, "subdir");

        // Check file comes second
        assert!(!entries[1].is_directory);
        assert_eq!(entries[1].name, "test_file.txt");
        assert_eq!(entries[1].size, 13);

        cleanup_test_environment(&test_dir).await;
    }

    #[tokio::test]
    async fn test_list_nonexistent_directory() {
        let test_dir = create_test_environment().await;
        let handler = FileHandler::new();
        let nonexistent = test_dir.join("nonexistent");

        let result = handler.list_directory(&nonexistent).await;
        assert!(result.is_err());

        cleanup_test_environment(&test_dir).await;
    }

    #[tokio::test]
    async fn test_read_file() {
        let test_dir = create_test_environment().await;
        let handler = FileHandler::new();
        let test_file = test_dir.join("test_file.txt");

        let result = handler.read_file(&test_file, None, None).await;
        assert!(result.is_ok());

        let content = result.unwrap();
        assert_eq!(content, b"Hello, World!");

        cleanup_test_environment(&test_dir).await;
    }

    #[tokio::test]
    async fn test_read_file_with_offset_and_length() {
        let test_dir = create_test_environment().await;
        let handler = FileHandler::new();
        let test_file = test_dir.join("test_file.txt");

        // Read "World" from "Hello, World!"
        let result = handler.read_file(&test_file, Some(7), Some(5)).await;
        assert!(result.is_ok());

        let content = result.unwrap();
        assert_eq!(content, b"World");

        cleanup_test_environment(&test_dir).await;
    }

    #[tokio::test]
    async fn test_write_file() {
        let test_dir = create_test_environment().await;
        let handler = FileHandler::new();
        let new_file = test_dir.join("new_file.txt");

        let data = b"New file content";
        let result = handler.write_file(&new_file, data, None).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), data.len() as u64);

        // Verify file was written
        let written_content = fs::read(&new_file).unwrap();
        assert_eq!(written_content, data);

        cleanup_test_environment(&test_dir).await;
    }

    #[tokio::test]
    async fn test_write_file_with_offset() {
        let test_dir = create_test_environment().await;
        let handler = FileHandler::new();
        let test_file = test_dir.join("test_file.txt");

        // Write "RUST" at offset 7 (replacing "World")
        let data = b"RUST";
        let result = handler.write_file(&test_file, data, Some(7)).await;
        assert!(result.is_ok());

        // Verify the content
        let content = fs::read(&test_file).unwrap();
        let content_str = String::from_utf8(content).unwrap();
        assert!(content_str.contains("Hello, RUST"));

        cleanup_test_environment(&test_dir).await;
    }

    #[tokio::test]
    async fn test_delete_file() {
        let test_dir = create_test_environment().await;
        let handler = FileHandler::new();
        let test_file = test_dir.join("test_file.txt");

        // Ensure file exists before deletion
        assert!(test_file.exists());

        let result = handler.delete_file(&test_file).await;
        assert!(result.is_ok());

        // Verify file was deleted
        assert!(!test_file.exists());

        cleanup_test_environment(&test_dir).await;
    }

    #[tokio::test]
    async fn test_delete_directory() {
        let test_dir = create_test_environment().await;
        let handler = FileHandler::new();
        let test_subdir = test_dir.join("subdir");

        // Ensure directory exists before deletion
        assert!(test_subdir.exists());

        let result = handler.delete_file(&test_subdir).await;
        assert!(result.is_ok());

        // Verify directory was deleted
        assert!(!test_subdir.exists());

        cleanup_test_environment(&test_dir).await;
    }

    #[tokio::test]
    async fn test_delete_nonexistent_file() {
        let test_dir = create_test_environment().await;
        let handler = FileHandler::new();
        let nonexistent = test_dir.join("nonexistent.txt");

        let result = handler.delete_file(&nonexistent).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("File not found"));

        cleanup_test_environment(&test_dir).await;
    }
}