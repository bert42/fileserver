use common::FileServerError;
use nix::unistd::{setgid, setuid, getuid, getgid, User, Group};
use tracing::{info, warn, error};

pub struct PrivilegeManager;

impl PrivilegeManager {
    pub fn new() -> Self {
        Self
    }

    pub fn drop_privileges(&self, username: Option<&str>, groupname: Option<&str>) -> Result<(), FileServerError> {
        // Only attempt privilege dropping if running as root
        if !getuid().is_root() {
            if username.is_some() || groupname.is_some() {
                warn!("User/group specified in config but not running as root - ignoring privilege drop");
            }
            return Ok(());
        }

        info!("Running as root, attempting to drop privileges");

        // Drop group privileges first
        if let Some(group_name) = groupname {
            let group = Group::from_name(group_name)
                .map_err(|e| FileServerError::ConfigError(format!("Failed to lookup group: {}", e)))?
                .ok_or_else(|| FileServerError::ConfigError(format!("Group '{}' not found", group_name)))?;
            
            setgid(group.gid)
                .map_err(|e| FileServerError::ConfigError(format!("Failed to set group ID: {}", e)))?;
            
            info!("Successfully changed group to: {} (gid: {})", group_name, group.gid);
        }

        // Drop user privileges
        if let Some(user_name) = username {
            let user = User::from_name(user_name)
                .map_err(|e| FileServerError::ConfigError(format!("Failed to lookup user: {}", e)))?
                .ok_or_else(|| FileServerError::ConfigError(format!("User '{}' not found", user_name)))?;
            
            setuid(user.uid)
                .map_err(|e| FileServerError::ConfigError(format!("Failed to set user ID: {}", e)))?;
            
            info!("Successfully changed user to: {} (uid: {})", user_name, user.uid);
        }

        // Verify we're no longer running as root
        if getuid().is_root() {
            error!("Still running as root after privilege drop attempt");
            return Err(FileServerError::ConfigError(
                "Failed to drop root privileges - still running as root".to_string()
            ));
        }

        info!("Privilege drop successful - now running as uid: {}, gid: {}", getuid(), getgid());
        Ok(())
    }

    pub fn validate_user_group(&self, username: Option<&str>, groupname: Option<&str>) -> Result<(), FileServerError> {
        // If running as root, validate that specified users/groups exist
        if getuid().is_root() {
            if let Some(user_name) = username {
                User::from_name(user_name)
                    .map_err(|e| FileServerError::ConfigError(format!("Failed to lookup user: {}", e)))?
                    .ok_or_else(|| FileServerError::ConfigError(format!("User '{}' not found", user_name)))?;
            }

            if let Some(group_name) = groupname {
                Group::from_name(group_name)
                    .map_err(|e| FileServerError::ConfigError(format!("Failed to lookup group: {}", e)))?
                    .ok_or_else(|| FileServerError::ConfigError(format!("Group '{}' not found", group_name)))?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_privilege_manager_creation() {
        let manager = PrivilegeManager::new();
        assert!(std::mem::size_of_val(&manager) == 0); // Zero-sized struct
    }

    #[test]
    fn test_validate_nonexistent_user() {
        let manager = PrivilegeManager::new();
        
        // This should not fail when not running as root
        let result = manager.validate_user_group(Some("nonexistent_user_12345"), None);
        
        if getuid().is_root() {
            // If running as root, this should fail
            assert!(result.is_err());
        } else {
            // If not running as root, validation should pass
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_drop_privileges_not_root() {
        let manager = PrivilegeManager::new();
        
        // This should not fail when not running as root
        let result = manager.drop_privileges(Some("nobody"), Some("nogroup"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_existing_user() {
        let manager = PrivilegeManager::new();
        
        // Test with a user that should exist on most systems
        let result = manager.validate_user_group(Some("root"), Some("root"));
        
        if getuid().is_root() {
            // If running as root, this should succeed
            assert!(result.is_ok());
        } else {
            // If not running as root, validation should pass (no validation performed)
            assert!(result.is_ok());
        }
    }
}