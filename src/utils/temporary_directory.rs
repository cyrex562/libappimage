use std::path::{Path, PathBuf};
use std::env;
use std::io;
use std::fs;
use tempfile::Builder;

/// A temporary directory that is automatically cleaned up when dropped.
/// 
/// This struct provides a safe way to create and manage temporary directories.
/// The directory is automatically removed when the struct is dropped.
/// 
/// # Examples
/// 
/// ```
/// use libappimage::utils::TemporaryDirectory;
/// 
/// let temp_dir = TemporaryDirectory::new("test").unwrap();
/// println!("Created temporary directory at: {}", temp_dir.path().display());
/// // Directory is automatically cleaned up when temp_dir is dropped
/// ```
pub struct TemporaryDirectory {
    path: PathBuf,
}

impl TemporaryDirectory {
    /// Creates a new temporary directory with an optional prefix.
    /// 
    /// The base directory may be changed using the $TEMPDIR environment variable.
    /// If $TEMPDIR is not set, the system's temporary directory is used.
    /// 
    /// # Arguments
    /// 
    /// * `prefix` - Optional prefix to add to the temporary directory's name
    /// 
    /// # Returns
    /// 
    /// * `Ok(TemporaryDirectory)` - The created temporary directory
    /// * `Err(io::Error)` - If the directory could not be created
    pub fn new(prefix: &str) -> io::Result<Self> {
        let mut builder = Builder::new();
        
        // Set prefix if provided
        if !prefix.is_empty() {
            builder.prefix(prefix);
        }
        
        // Set temp directory from environment if available
        if let Ok(temp_dir) = env::var("TEMPDIR") {
            builder.tempdir_in(temp_dir)
        } else {
            builder.tempdir_in(env::temp_dir())
        }.map(|temp_dir| Self {
            path: temp_dir.path().to_path_buf()
        })
    }

    /// Returns the path to the temporary directory.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TemporaryDirectory {
    fn drop(&mut self) {
        // Ignore errors during cleanup
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_create_temp_dir() {
        let temp_dir = TemporaryDirectory::new("test").unwrap();
        assert!(temp_dir.path().exists());
        assert!(fs::metadata(temp_dir.path()).unwrap().is_dir());
    }

    #[test]
    fn test_create_temp_dir_with_prefix() {
        let temp_dir = TemporaryDirectory::new("test-prefix").unwrap();
        assert!(temp_dir.path().exists());
        assert!(temp_dir.path().to_string_lossy().contains("test-prefix"));
    }

    #[test]
    fn test_temp_dir_cleanup() {
        let path = {
            let temp_dir = TemporaryDirectory::new("cleanup-test").unwrap();
            temp_dir.path().to_path_buf()
        };
        
        assert!(!path.exists());
    }

    #[test]
    fn test_temp_dir_custom_location() {
        let custom_temp = env::temp_dir().join("custom-temp");
        fs::create_dir_all(&custom_temp).unwrap();
        
        env::set_var("TEMPDIR", custom_temp.to_str().unwrap());
        let temp_dir = TemporaryDirectory::new("custom").unwrap();
        
        assert!(temp_dir.path().starts_with(&custom_temp));
        
        // Cleanup
        env::remove_var("TEMPDIR");
        let _ = fs::remove_dir_all(&custom_temp);
    }
} 