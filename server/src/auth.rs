use crate::config::ServerConfig;
use common::FileServerError;
use std::net::IpAddr;
use tonic::{Request, Status};

pub struct AuthService {
    pub config: ServerConfig,
}

impl AuthService {
    pub fn new(config: ServerConfig) -> Self {
        Self { config }
    }

    pub fn authorize_connection(&self, request: &Request<()>) -> Result<(), Status> {
        let client_ip = self.extract_client_ip(request)?;
        
        if !self.config.is_ip_allowed(&client_ip) {
            return Err(Status::permission_denied(
                format!("IP address {} is not allowed to connect", client_ip)
            ));
        }
        
        Ok(())
    }

    pub fn check_directory_access(&self, dir_name: &str, operation: &str) -> Result<String, FileServerError> {
        let directory = self.config.get_directory(dir_name)
            .ok_or_else(|| FileServerError::PermissionDenied(
                format!("Directory '{}' not found", dir_name)
            ))?;

        match (operation, directory.permissions.as_str()) {
            ("read", "read-only") | ("read", "read-write") => Ok(directory.path.clone()),
            ("write", "read-write") => Ok(directory.path.clone()),
            ("write", "read-only") => Err(FileServerError::PermissionDenied(
                format!("Write operation not allowed on read-only directory '{}'", dir_name)
            )),
            _ => Err(FileServerError::PermissionDenied(
                format!("Invalid operation '{}' on directory '{}'", operation, dir_name)
            )),
        }
    }

    fn extract_client_ip(&self, request: &Request<()>) -> Result<IpAddr, Status> {
        let remote_addr = request.remote_addr();
        
        match remote_addr {
            Some(addr) => Ok(addr.ip()),
            None => {
                // Fallback to localhost for local development when remote_addr is not available
                Ok("127.0.0.1".parse().unwrap())
            }
        }
    }

    pub fn validate_path(&self, path: &str) -> Result<(), FileServerError> {
        if path.contains("..") {
            return Err(FileServerError::InvalidPath(
                "Path traversal not allowed".to_string()
            ));
        }

        if path.starts_with('/') || path.starts_with('\\') {
            return Err(FileServerError::InvalidPath(
                "Absolute paths not allowed".to_string()
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ServerConfig, ServerSettings, DirectoryConfig};
    use std::fs;

    fn create_test_config() -> ServerConfig {
        // Create temporary directories for testing with unique names
        let uuid = uuid::Uuid::now_v7();
        let temp_dir = std::env::temp_dir().join(format!("fileserver_auth_test_{}", uuid));
        let docs_dir = temp_dir.join("docs");
        let workspace_dir = temp_dir.join("workspace");
        
        fs::create_dir_all(&docs_dir).unwrap();
        fs::create_dir_all(&workspace_dir).unwrap();

        ServerConfig {
            server: ServerSettings {
                port: 8080,
                allowed_ips: vec!["127.0.0.1".to_string(), "192.168.1.0/24".to_string()],
                user: None,
                group: None,
            },
            directories: vec![
                DirectoryConfig {
                    name: "docs".to_string(),
                    path: docs_dir.to_string_lossy().to_string(),
                    permissions: "read-only".to_string(),
                },
                DirectoryConfig {
                    name: "workspace".to_string(),
                    path: workspace_dir.to_string_lossy().to_string(),
                    permissions: "read-write".to_string(),
                },
            ],
        }
    }

    fn cleanup_test_dirs(config: &ServerConfig) {
        if let Some(dir_config) = config.directories.first() {
            if let Some(parent) = std::path::Path::new(&dir_config.path).parent() {
                if let Some(grandparent) = parent.parent() {
                    fs::remove_dir_all(grandparent).ok();
                }
            }
        }
    }

    #[test]
    fn test_directory_access_read_operations() {
        let config = create_test_config();
        let auth = AuthService::new(config.clone());

        // Test read access to read-only directory
        let result = auth.check_directory_access("docs", "read");
        assert!(result.is_ok());

        // Test read access to read-write directory
        let result = auth.check_directory_access("workspace", "read");
        assert!(result.is_ok());

        cleanup_test_dirs(&config);
    }

    #[test]
    fn test_directory_access_write_operations() {
        let config = create_test_config();
        let auth = AuthService::new(config.clone());

        // Test write access to read-only directory (should fail)
        let result = auth.check_directory_access("docs", "write");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Write operation not allowed"));

        // Test write access to read-write directory (should succeed)
        let result = auth.check_directory_access("workspace", "write");
        assert!(result.is_ok());

        cleanup_test_dirs(&config);
    }

    #[test]
    fn test_directory_access_nonexistent_directory() {
        let config = create_test_config();
        let auth = AuthService::new(config.clone());

        let result = auth.check_directory_access("nonexistent", "read");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Directory 'nonexistent' not found"));

        cleanup_test_dirs(&config);
    }

    #[test]
    fn test_path_validation() {
        let config = create_test_config();
        let auth = AuthService::new(config.clone());

        // Valid paths
        assert!(auth.validate_path("file.txt").is_ok());
        assert!(auth.validate_path("subdir/file.txt").is_ok());
        assert!(auth.validate_path("").is_ok());

        // Invalid paths with directory traversal
        assert!(auth.validate_path("../file.txt").is_err());
        assert!(auth.validate_path("dir/../file.txt").is_err());
        assert!(auth.validate_path("../../etc/passwd").is_err());

        // Invalid absolute paths
        assert!(auth.validate_path("/etc/passwd").is_err());
        assert!(auth.validate_path("\\Windows\\System32").is_err());

        cleanup_test_dirs(&config);
    }

    #[test]
    fn test_path_validation_error_messages() {
        let config = create_test_config();
        let auth = AuthService::new(config.clone());

        let result = auth.validate_path("../file.txt");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Path traversal not allowed"));

        let result = auth.validate_path("/etc/passwd");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Absolute paths not allowed"));

        cleanup_test_dirs(&config);
    }
}