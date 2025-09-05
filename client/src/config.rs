use common::FileServerError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    pub server: ServerSettings,
    pub client: ClientSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerSettings {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientSettings {
    pub timeout_seconds: u64,
    pub retry_attempts: u32,
}

impl ClientConfig {
    pub fn load_from_file(path: &str) -> Result<Self, FileServerError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| FileServerError::ConfigError(format!("Failed to read config file: {}", e)))?;
        
        let config: ClientConfig = toml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), FileServerError> {
        if self.server.host.is_empty() {
            return Err(FileServerError::ConfigError("Server host cannot be empty".to_string()));
        }

        if self.server.port == 0 {
            return Err(FileServerError::ConfigError("Server port cannot be 0".to_string()));
        }

        if self.client.timeout_seconds == 0 {
            return Err(FileServerError::ConfigError("Timeout cannot be 0".to_string()));
        }

        if self.client.retry_attempts == 0 {
            return Err(FileServerError::ConfigError("Retry attempts cannot be 0".to_string()));
        }

        Ok(())
    }

    pub fn server_address(&self) -> String {
        format!("http://{}:{}", self.server.host, self.server.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_client_config_parsing() {
        let config_content = r#"
[server]
host = "localhost"
port = 9090

[client]
timeout_seconds = 60
retry_attempts = 5
        "#;

        let config: ClientConfig = toml::from_str(config_content).unwrap();
        
        assert_eq!(config.server.host, "localhost");
        assert_eq!(config.server.port, 9090);
        assert_eq!(config.client.timeout_seconds, 60);
        assert_eq!(config.client.retry_attempts, 5);
    }

    #[test]
    fn test_server_address_generation() {
        let config = ClientConfig {
            server: ServerSettings {
                host: "192.168.1.100".to_string(),
                port: 8080,
            },
            client: ClientSettings {
                timeout_seconds: 30,
                retry_attempts: 3,
            },
        };

        assert_eq!(config.server_address(), "http://192.168.1.100:8080");
    }

    #[test]
    fn test_config_validation_valid() {
        let config = ClientConfig {
            server: ServerSettings {
                host: "localhost".to_string(),
                port: 8080,
            },
            client: ClientSettings {
                timeout_seconds: 30,
                retry_attempts: 3,
            },
        };

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_empty_host() {
        let config = ClientConfig {
            server: ServerSettings {
                host: "".to_string(),
                port: 8080,
            },
            client: ClientSettings {
                timeout_seconds: 30,
                retry_attempts: 3,
            },
        };

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Server host cannot be empty"));
    }

    #[test]
    fn test_config_validation_zero_port() {
        let config = ClientConfig {
            server: ServerSettings {
                host: "localhost".to_string(),
                port: 0,
            },
            client: ClientSettings {
                timeout_seconds: 30,
                retry_attempts: 3,
            },
        };

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Server port cannot be 0"));
    }

    #[test]
    fn test_config_validation_zero_timeout() {
        let config = ClientConfig {
            server: ServerSettings {
                host: "localhost".to_string(),
                port: 8080,
            },
            client: ClientSettings {
                timeout_seconds: 0,
                retry_attempts: 3,
            },
        };

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Timeout cannot be 0"));
    }

    #[test]
    fn test_config_validation_zero_retries() {
        let config = ClientConfig {
            server: ServerSettings {
                host: "localhost".to_string(),
                port: 8080,
            },
            client: ClientSettings {
                timeout_seconds: 30,
                retry_attempts: 0,
            },
        };

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Retry attempts cannot be 0"));
    }

    #[test]
    fn test_config_load_from_file() {
        // Create a temporary config file
        let temp_dir = std::env::temp_dir();
        let config_file = temp_dir.join("test_client_config.toml");
        
        let config_content = r#"
[server]
host = "testhost"
port = 12345

[client]
timeout_seconds = 120
retry_attempts = 10
        "#;

        std::fs::write(&config_file, config_content).unwrap();

        let config = ClientConfig::load_from_file(config_file.to_str().unwrap()).unwrap();
        
        assert_eq!(config.server.host, "testhost");
        assert_eq!(config.server.port, 12345);
        assert_eq!(config.client.timeout_seconds, 120);
        assert_eq!(config.client.retry_attempts, 10);

        // Clean up
        std::fs::remove_file(&config_file).ok();
    }

    #[test]
    fn test_config_load_nonexistent_file() {
        let result = ClientConfig::load_from_file("nonexistent_config.toml");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Failed to read config file"));
    }
}