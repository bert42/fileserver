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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::net::IpAddr;

    #[test]
    fn test_valid_config_parsing() {
        let config_content = r#"
[server]
port = 8080
allowed_ips = ["127.0.0.1", "192.168.1.0/24"]

[[directories]]
name = "test_dir"
path = "/tmp"
permissions = "read-only"
        "#;

        let config: ServerConfig = toml::from_str(config_content).unwrap();
        
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.server.allowed_ips, vec!["127.0.0.1", "192.168.1.0/24"]);
        assert_eq!(config.directories.len(), 1);
        assert_eq!(config.directories[0].name, "test_dir");
        assert_eq!(config.directories[0].path, "/tmp");
        assert_eq!(config.directories[0].permissions, "read-only");
    }

    #[test]
    fn test_ip_validation() {
        let config = ServerConfig {
            server: ServerSettings {
                port: 8080,
                allowed_ips: vec!["127.0.0.1".to_string(), "192.168.1.0/24".to_string()],
            },
            directories: vec![],
        };

        // Test localhost
        let localhost: IpAddr = "127.0.0.1".parse().unwrap();
        assert!(config.is_ip_allowed(&localhost));

        // Test IP in CIDR range
        let ip_in_range: IpAddr = "192.168.1.100".parse().unwrap();
        assert!(config.is_ip_allowed(&ip_in_range));

        // Test IP not in range
        let ip_not_allowed: IpAddr = "10.0.0.1".parse().unwrap();
        assert!(!config.is_ip_allowed(&ip_not_allowed));
    }

    #[test]
    fn test_directory_lookup() {
        let config = ServerConfig {
            server: ServerSettings {
                port: 8080,
                allowed_ips: vec!["127.0.0.1".to_string()],
            },
            directories: vec![
                DirectoryConfig {
                    name: "docs".to_string(),
                    path: "/tmp/docs".to_string(),
                    permissions: "read-only".to_string(),
                },
                DirectoryConfig {
                    name: "workspace".to_string(),
                    path: "/tmp/workspace".to_string(),
                    permissions: "read-write".to_string(),
                },
            ],
        };

        assert!(config.get_directory("docs").is_some());
        assert!(config.get_directory("workspace").is_some());
        assert!(config.get_directory("nonexistent").is_none());
        
        let docs_dir = config.get_directory("docs").unwrap();
        assert_eq!(docs_dir.permissions, "read-only");
    }

    #[test]
    fn test_config_validation_invalid_port() {
        let config = ServerConfig {
            server: ServerSettings {
                port: 0,
                allowed_ips: vec!["127.0.0.1".to_string()],
            },
            directories: vec![],
        };

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Port cannot be 0"));
    }

    #[test]
    fn test_config_validation_invalid_permissions() {
        // Create a temporary directory for testing
        let temp_dir = std::env::temp_dir().join("fileserver_test");
        fs::create_dir_all(&temp_dir).unwrap();

        let config = ServerConfig {
            server: ServerSettings {
                port: 8080,
                allowed_ips: vec!["127.0.0.1".to_string()],
            },
            directories: vec![DirectoryConfig {
                name: "test".to_string(),
                path: temp_dir.to_string_lossy().to_string(),
                permissions: "invalid".to_string(),
            }],
        };

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid permissions"));

        // Clean up
        fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_is_valid_ip_or_cidr() {
        assert!(ServerConfig::is_valid_ip_or_cidr("127.0.0.1"));
        assert!(ServerConfig::is_valid_ip_or_cidr("192.168.1.0/24"));
        assert!(ServerConfig::is_valid_ip_or_cidr("::1"));
        assert!(!ServerConfig::is_valid_ip_or_cidr("invalid_ip"));
        assert!(!ServerConfig::is_valid_ip_or_cidr("256.256.256.256"));
    }
}