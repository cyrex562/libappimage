use std::path::{Path, PathBuf};
use crate::utils::hashlib;

/// Error type for path operations
#[derive(Debug, thiserror::Error)]
pub enum PathError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type for path operations
pub type Result<T> = std::result::Result<T, PathError>;

/// Prepends 'file://' to a local path string if required.
/// 
/// # Arguments
/// 
/// * `path` - The path to convert to URI format
/// 
/// # Returns
/// 
/// The path with 'file://' prefix if it wasn't already present
/// 
/// # Example
/// 
/// ```
/// use libappimage::utils::path_utils::path_to_uri;
/// 
/// assert_eq!(path_to_uri("/path/to/file"), "file:///path/to/file");
/// assert_eq!(path_to_uri("file:///path/to/file"), "file:///path/to/file");
/// ```
pub fn path_to_uri<P: AsRef<Path>>(path: P) -> String {
    let path_str = path.as_ref().to_string_lossy();
    if !path_str.starts_with("file://") {
        format!("file://{}", path_str)
    } else {
        path_str.into_owned()
    }
}

/// Provides a MD5 hash that identifies a file given its path.
/// 
/// Implementation of the thumbnail filename hash function available at:
/// https://specifications.freedesktop.org/thumbnail-spec/thumbnail-spec-latest.html#THUMBSAVE
/// 
/// It may be used to identify files that are related to a given AppImage at a given location.
/// 
/// # Arguments
/// 
/// * `path` - The path to hash
/// 
/// # Returns
/// 
/// The MD5 hash of the path as a hex string, or an empty string if the path is empty or invalid
/// 
/// # Example
/// 
/// ```
/// use libappimage::utils::path_utils::hash_path;
/// use std::path::Path;
/// 
/// let path = Path::new("/path/to/file");
/// let hash = hash_path(path).unwrap();
/// assert!(!hash.is_empty());
/// ```
pub fn hash_path<P: AsRef<Path>>(path: P) -> Result<String> {
    let path = path.as_ref();
    
    if path.as_os_str().is_empty() {
        return Ok(String::new());
    }

    let canonical_path = path.canonicalize()?;
    
    if canonical_path.as_os_str().is_empty() {
        return Ok(String::new());
    }

    let uri = path_to_uri(&canonical_path);
    let md5_raw = hashlib::md5(&uri);
    let md5_str = hashlib::to_hex(&md5_raw);

    Ok(md5_str)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn test_path_to_uri() {
        assert_eq!(path_to_uri("/path/to/file"), "file:///path/to/file");
        assert_eq!(path_to_uri("file:///path/to/file"), "file:///path/to/file");
        assert_eq!(path_to_uri(""), "file://");
        assert_eq!(path_to_uri("file://"), "file://");
    }

    #[test]
    fn test_hash_path() -> Result<()> {
        // Create a temporary directory and file for testing
        let dir = tempdir()?;
        let file_path = dir.path().join("test.txt");
        File::create(&file_path)?;

        // Test with a valid path
        let hash = hash_path(&file_path)?;
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 32); // MD5 hash is 32 characters in hex

        // Test with an empty path
        let empty_hash = hash_path("")?;
        assert!(empty_hash.is_empty());

        // Test with a non-existent path
        let non_existent = dir.path().join("non_existent.txt");
        assert!(hash_path(&non_existent).is_err());

        Ok(())
    }

    #[test]
    fn test_hash_path_consistency() -> Result<()> {
        // Test that the same path always produces the same hash
        let path = "/path/to/file";
        let hash1 = hash_path(path)?;
        let hash2 = hash_path(path)?;
        assert_eq!(hash1, hash2);

        // Test that different paths produce different hashes
        let hash3 = hash_path("/path/to/different")?;
        assert_ne!(hash1, hash3);

        Ok(())
    }
} 