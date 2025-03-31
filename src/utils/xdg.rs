use crate::{AppImageError, AppImageResult};
use std::env;
use std::path::{Path, PathBuf};

/// Get the user's home directory
pub fn user_home() -> AppImageResult<PathBuf> {
    env::var_os("HOME").map(PathBuf::from).ok_or_else(|| {
        AppImageError::EnvironmentError("HOME environment variable not set".to_string())
    })
}

/// Get the XDG config home directory
pub fn xdg_config_home() -> AppImageResult<PathBuf> {
    env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| user_home().map(|home| home.join(".config")).ok())
        .ok_or_else(|| {
            AppImageError::EnvironmentError("Could not determine XDG config home".to_string())
        })
}

/// Get the XDG data home directory
pub fn xdg_data_home() -> AppImageResult<PathBuf> {
    env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| user_home().map(|home| home.join(".local/share")).ok())
        .ok_or_else(|| {
            AppImageError::EnvironmentError("Could not determine XDG data home".to_string())
        })
}

/// Get the XDG cache home directory
pub fn xdg_cache_home() -> AppImageResult<PathBuf> {
    env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| user_home().map(|home| home.join(".cache")).ok())
        .ok_or_else(|| {
            AppImageError::EnvironmentError("Could not determine XDG cache home".to_string())
        })
}

/// Get the XDG runtime directory
pub fn xdg_runtime_dir() -> AppImageResult<PathBuf> {
    env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .ok_or_else(|| {
            AppImageError::EnvironmentError(
                "XDG_RUNTIME_DIR environment variable not set".to_string(),
            )
        })
}

/// Get the XDG data directories
pub fn xdg_data_dirs() -> AppImageResult<Vec<PathBuf>> {
    env::var_os("XDG_DATA_DIRS")
        .map(|dirs| {
            dirs.to_str()
                .unwrap_or(":/usr/local/share:/usr/share")
                .split(':')
                .map(PathBuf::from)
                .collect()
        })
        .ok_or_else(|| {
            AppImageError::EnvironmentError("Could not determine XDG data directories".to_string())
        })
}

/// Get the XDG config directories
pub fn xdg_config_dirs() -> AppImageResult<Vec<PathBuf>> {
    env::var_os("XDG_CONFIG_DIRS")
        .map(|dirs| {
            dirs.to_str()
                .unwrap_or(":/etc/xdg")
                .split(':')
                .map(PathBuf::from)
                .collect()
        })
        .ok_or_else(|| {
            AppImageError::EnvironmentError(
                "Could not determine XDG config directories".to_string(),
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_home() {
        assert!(user_home().is_ok());
    }

    #[test]
    fn test_xdg_config_home() {
        assert!(xdg_config_home().is_ok());
    }

    #[test]
    fn test_xdg_data_home() {
        assert!(xdg_data_home().is_ok());
    }

    #[test]
    fn test_xdg_cache_home() {
        assert!(xdg_cache_home().is_ok());
    }

    #[test]
    fn test_xdg_data_dirs() {
        assert!(xdg_data_dirs().is_ok());
    }

    #[test]
    fn test_xdg_config_dirs() {
        assert!(xdg_config_dirs().is_ok());
    }
}
