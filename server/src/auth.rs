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