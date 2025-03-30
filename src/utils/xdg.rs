use std::env;
use std::path::{Path, PathBuf};
use crate::AppImageError;

/// Get the user's home directory
pub fn user_home() -> AppImageError<PathBuf> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| AppImageError::EnvironmentError("HOME environment variable not set".to_string()))
}

/// Get the XDG config home directory
pub fn xdg_config_home() -> AppImageError<PathBuf> {
    env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            user_home().map(|home| home.join(".config")).ok()
        })
        .ok_or_else(|| AppImageError::EnvironmentError("Could not determine XDG config home".to_string()))
}

/// Get the XDG data home directory
pub fn xdg_data_home() -> AppImageError<PathBuf> {
    env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            user_home().map(|home| home.join(".local/share")).ok()
        })
        .ok_or_else(|| AppImageError::EnvironmentError("Could not determine XDG data home".to_string()))
}

/// Get the XDG cache home directory
pub fn xdg_cache_home() -> AppImageError<PathBuf> {
    env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            user_home().map(|home| home.join(".cache")).ok()
        })
        .ok_or_else(|| AppImageError::EnvironmentError("Could not determine XDG cache home".to_string()))
}

/// Get the XDG runtime directory
pub fn xdg_runtime_dir() -> AppImageError<PathBuf> {
    env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .ok_or_else(|| AppImageError::EnvironmentError("XDG_RUNTIME_DIR environment variable not set".to_string()))
}

/// Get the XDG data directories
pub fn xdg_data_dirs() -> AppImageError<Vec<PathBuf>> {
    let mut dirs = Vec::new();
    
    // Add XDG_DATA_HOME if set
    if let Ok(home) = xdg_data_home() {
        dirs.push(home);
    }
    
    // Add XDG_DATA_DIRS if set, otherwise use default
    if let Ok(data_dirs) = env::var("XDG_DATA_DIRS") {
        dirs.extend(
            data_dirs
                .split(':')
                .filter(|s| !s.is_empty())
                .map(PathBuf::from)
        );
    } else {
        dirs.push(PathBuf::from("/usr/local/share"));
        dirs.push(PathBuf::from("/usr/share"));
    }
    
    Ok(dirs)
}

/// Get the XDG config directories
pub fn xdg_config_dirs() -> AppImageError<Vec<PathBuf>> {
    let mut dirs = Vec::new();
    
    // Add XDG_CONFIG_HOME if set
    if let Ok(home) = xdg_config_home() {
        dirs.push(home);
    }
    
    // Add XDG_CONFIG_DIRS if set, otherwise use default
    if let Ok(config_dirs) = env::var("XDG_CONFIG_DIRS") {
        dirs.extend(
            config_dirs
                .split(':')
                .filter(|s| !s.is_empty())
                .map(PathBuf::from)
        );
    } else {
        dirs.push(PathBuf::from("/etc/xdg"));
    }
    
    Ok(dirs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_user_home() {
        let home = user_home().unwrap();
        assert!(home.exists());
        assert!(home.is_absolute());
    }

    #[test]
    fn test_xdg_config_home() {
        let config_home = xdg_config_home().unwrap();
        assert!(config_home.is_absolute());
        
        // Test with custom XDG_CONFIG_HOME
        env::set_var("XDG_CONFIG_HOME", "/tmp/test_config");
        let custom_config = xdg_config_home().unwrap();
        assert_eq!(custom_config, PathBuf::from("/tmp/test_config"));
        
        // Test without XDG_CONFIG_HOME
        env::remove_var("XDG_CONFIG_HOME");
        let default_config = xdg_config_home().unwrap();
        assert!(default_config.ends_with(".config"));
    }

    #[test]
    fn test_xdg_data_home() {
        let data_home = xdg_data_home().unwrap();
        assert!(data_home.is_absolute());
        
        // Test with custom XDG_DATA_HOME
        env::set_var("XDG_DATA_HOME", "/tmp/test_data");
        let custom_data = xdg_data_home().unwrap();
        assert_eq!(custom_data, PathBuf::from("/tmp/test_data"));
        
        // Test without XDG_DATA_HOME
        env::remove_var("XDG_DATA_HOME");
        let default_data = xdg_data_home().unwrap();
        assert!(default_data.ends_with(".local/share"));
    }

    #[test]
    fn test_xdg_cache_home() {
        let cache_home = xdg_cache_home().unwrap();
        assert!(cache_home.is_absolute());
        
        // Test with custom XDG_CACHE_HOME
        env::set_var("XDG_CACHE_HOME", "/tmp/test_cache");
        let custom_cache = xdg_cache_home().unwrap();
        assert_eq!(custom_cache, PathBuf::from("/tmp/test_cache"));
        
        // Test without XDG_CACHE_HOME
        env::remove_var("XDG_CACHE_HOME");
        let default_cache = xdg_cache_home().unwrap();
        assert!(default_cache.ends_with(".cache"));
    }

    #[test]
    fn test_xdg_data_dirs() {
        let dirs = xdg_data_dirs().unwrap();
        assert!(!dirs.is_empty());
        assert!(dirs.iter().all(|d| d.is_absolute()));
        
        // Test with custom XDG_DATA_DIRS
        env::set_var("XDG_DATA_DIRS", "/tmp/test1:/tmp/test2");
        let custom_dirs = xdg_data_dirs().unwrap();
        assert!(custom_dirs.contains(&PathBuf::from("/tmp/test1")));
        assert!(custom_dirs.contains(&PathBuf::from("/tmp/test2")));
    }

    #[test]
    fn test_xdg_config_dirs() {
        let dirs = xdg_config_dirs().unwrap();
        assert!(!dirs.is_empty());
        assert!(dirs.iter().all(|d| d.is_absolute()));
        
        // Test with custom XDG_CONFIG_DIRS
        env::set_var("XDG_CONFIG_DIRS", "/tmp/test1:/tmp/test2");
        let custom_dirs = xdg_config_dirs().unwrap();
        assert!(custom_dirs.contains(&PathBuf::from("/tmp/test1")));
        assert!(custom_dirs.contains(&PathBuf::from("/tmp/test2")));
    }
} 