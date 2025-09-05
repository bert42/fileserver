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
}