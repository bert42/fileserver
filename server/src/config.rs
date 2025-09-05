use common::FileServerError;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::path::PathBuf;
use ipnet::IpNet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub server: ServerSettings,
    pub directories: Vec<DirectoryConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerSettings {
    pub port: u16,
    pub allowed_ips: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryConfig {
    pub name: String,
    pub path: String,
    pub permissions: String,
}

impl ServerConfig {
    pub fn load_from_file(path: &str) -> Result<Self, FileServerError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| FileServerError::ConfigError(format!("Failed to read config file: {}", e)))?;
        
        let config: ServerConfig = toml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), FileServerError> {
        if self.server.port == 0 {
            return Err(FileServerError::ConfigError("Port cannot be 0".to_string()));
        }

        for ip_str in &self.server.allowed_ips {
            if !Self::is_valid_ip_or_cidr(ip_str) {
                return Err(FileServerError::ConfigError(
                    format!("Invalid IP address or CIDR: {}", ip_str)
                ));
            }
        }

        for dir in &self.directories {
            let path = PathBuf::from(&dir.path);
            if !path.exists() {
                return Err(FileServerError::ConfigError(
                    format!("Directory does not exist: {}", dir.path)
                ));
            }

            match dir.permissions.as_str() {
                "read-only" | "read-write" => {},
                _ => return Err(FileServerError::ConfigError(
                    format!("Invalid permissions '{}'. Must be 'read-only' or 'read-write'", dir.permissions)
                )),
            }
        }

        Ok(())
    }

    fn is_valid_ip_or_cidr(ip_str: &str) -> bool {
        if let Ok(_) = ip_str.parse::<IpAddr>() {
            return true;
        }
        
        if let Ok(_) = ip_str.parse::<IpNet>() {
            return true;
        }
        
        false
    }

    pub fn is_ip_allowed(&self, client_ip: &IpAddr) -> bool {
        for allowed in &self.server.allowed_ips {
            if let Ok(ip) = allowed.parse::<IpAddr>() {
                if ip == *client_ip {
                    return true;
                }
            } else if let Ok(net) = allowed.parse::<IpNet>() {
                if net.contains(client_ip) {
                    return true;
                }
            }
        }
        false
    }

    pub fn get_directory(&self, name: &str) -> Option<&DirectoryConfig> {
        self.directories.iter().find(|d| d.name == name)
    }
}