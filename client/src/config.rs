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