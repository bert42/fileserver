use common::FileServerError;
use nix::unistd::{setgid, setuid, getuid, getgid, User, Group, Uid, Gid};
use tracing::{info, warn, error};

pub struct PrivilegeManager;

impl PrivilegeManager {
    pub fn new() -> Self {
        Self
    }

    /// Parse user string as either name or numeric UID
    fn parse_user(&self, user_str: &str) -> Result<(User, String), FileServerError> {
        // Try parsing as numeric UID first
        if let Ok(uid_num) = user_str.parse::<u32>() {
            let uid = Uid::from_raw(uid_num);
            let user = User::from_uid(uid)
                .map_err(|e| FileServerError::ConfigError(format!("Failed to lookup user by UID {}: {}", uid_num, e)))?
                .ok_or_else(|| FileServerError::ConfigError(format!("User with UID {} not found", uid_num)))?;
            let display = format!("{} (UID: {})", user.name, uid_num);
            Ok((user, display))
        } else {
            // Parse as username
            let user = User::from_name(user_str)
                .map_err(|e| FileServerError::ConfigError(format!("Failed to lookup user by name '{}': {}", user_str, e)))?
                .ok_or_else(|| FileServerError::ConfigError(format!("User '{}' not found", user_str)))?;
            let display = format!("{} (UID: {})", user.name, user.uid);
            Ok((user, display))
        }
    }

    /// Parse group string as either name or numeric GID
    fn parse_group(&self, group_str: &str) -> Result<(Group, String), FileServerError> {
        // Try parsing as numeric GID first
        if let Ok(gid_num) = group_str.parse::<u32>() {
            let gid = Gid::from_raw(gid_num);
            let group = Group::from_gid(gid)
                .map_err(|e| FileServerError::ConfigError(format!("Failed to lookup group by GID {}: {}", gid_num, e)))?
                .ok_or_else(|| FileServerError::ConfigError(format!("Group with GID {} not found", gid_num)))?;
            let display = format!("{} (GID: {})", group.name, gid_num);
            Ok((group, display))
        } else {
            // Parse as group name
            let group = Group::from_name(group_str)
                .map_err(|e| FileServerError::ConfigError(format!("Failed to lookup group by name '{}': {}", group_str, e)))?
                .ok_or_else(|| FileServerError::ConfigError(format!("Group '{}' not found", group_str)))?;
            let display = format!("{} (GID: {})", group.name, group.gid);
            Ok((group, display))
        }
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
        if let Some(group_str) = groupname {
            let (group, group_display) = self.parse_group(group_str)?;
            
            setgid(group.gid)
                .map_err(|e| FileServerError::ConfigError(format!("Failed to set group ID: {}", e)))?;
            
            info!("Successfully changed group to: {}", group_display);
        }

        // Drop user privileges
        if let Some(user_str) = username {
            let (user, user_display) = self.parse_user(user_str)?;
            
            setuid(user.uid)
                .map_err(|e| FileServerError::ConfigError(format!("Failed to set user ID: {}", e)))?;
            
            info!("Successfully changed user to: {}", user_display);
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
            if let Some(user_str) = username {
                let (_user, user_display) = self.parse_user(user_str)?;
                info!("Validated user: {}", user_display);
            }

            if let Some(group_str) = groupname {
                let (_group, group_display) = self.parse_group(group_str)?;
                info!("Validated group: {}", group_display);
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

    #[test]
    fn test_parse_user_by_name() {
        let manager = PrivilegeManager::new();
        
        // This test only works if we can lookup system users
        if let Ok((user, display)) = manager.parse_user("root") {
            assert_eq!(user.name, "root");
            assert!(display.contains("root"));
            assert!(display.contains("UID:"));
        }
    }

    #[test]
    fn test_parse_user_by_uid() {
        let manager = PrivilegeManager::new();
        
        // Test with UID 0 (root) - should exist on all Unix systems
        if let Ok((user, display)) = manager.parse_user("0") {
            assert_eq!(user.uid, Uid::from_raw(0));
            assert!(display.contains("UID: 0"));
        }
    }

    #[test]
    fn test_parse_group_by_name() {
        let manager = PrivilegeManager::new();
        
        // Test with a group that should exist on most systems
        if let Ok((_group, display)) = manager.parse_group("root") {
            assert!(display.contains("root"));
            assert!(display.contains("GID:"));
        }
    }

    #[test]
    fn test_parse_group_by_gid() {
        let manager = PrivilegeManager::new();
        
        // Test with GID 0 (root group) - should exist on all Unix systems
        if let Ok((group, display)) = manager.parse_group("0") {
            assert_eq!(group.gid, Gid::from_raw(0));
            assert!(display.contains("GID: 0"));
        }
    }

    #[test]
    fn test_parse_nonexistent_numeric_user() {
        let manager = PrivilegeManager::new();
        
        // Use a very high UID that shouldn't exist
        let result = manager.parse_user("999999");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("User with UID 999999 not found"));
    }

    #[test]
    fn test_parse_nonexistent_numeric_group() {
        let manager = PrivilegeManager::new();
        
        // Use a very high GID that shouldn't exist
        let result = manager.parse_group("999999");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Group with GID 999999 not found"));
    }

    #[test]
    fn test_validate_numeric_ids() {
        let manager = PrivilegeManager::new();
        
        // This test works regardless of whether we're running as root
        // because validation logic handles both cases
        let result = manager.validate_user_group(Some("0"), Some("0"));
        
        if getuid().is_root() {
            // If running as root, should validate successfully
            assert!(result.is_ok());
        } else {
            // If not running as root, validation should pass (no validation performed)
            assert!(result.is_ok());
        }
    }
}