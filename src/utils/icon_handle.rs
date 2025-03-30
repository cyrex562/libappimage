use std::path::Path;
use std::error::Error;
use std::fmt;
use std::fs;
use std::io::Read;
use crate::utils::icon_handle_backend::IconHandleBackend;

/// Error type for icon handling operations
#[derive(Debug)]
pub enum IconHandleError {
    /// IO error occurred
    Io(std::io::Error),
    /// Backend error (cairo/rsvg)
    Backend(String),
    /// Unsupported image format
    UnsupportedFormat(String),
    /// Invalid icon data
    InvalidData(String),
}

impl fmt::Display for IconHandleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IconHandleError::Io(e) => write!(f, "IO error: {}", e),
            IconHandleError::Backend(e) => write!(f, "Backend error: {}", e),
            IconHandleError::UnsupportedFormat(e) => write!(f, "Unsupported format: {}", e),
            IconHandleError::InvalidData(e) => write!(f, "Invalid data: {}", e),
        }
    }
}

impl Error for IconHandleError {}

impl From<std::io::Error> for IconHandleError {
    fn from(err: std::io::Error) -> Self {
        IconHandleError::Io(err)
    }
}

/// Provide the image manipulation functions required by libappimage, nothing more.
/// Currently are supported two image formats: png and svg. Those formats are the
/// ones recommended for creating icons at the FreeDesktop Icon Theme Specification.
/// See: https://standards.freedesktop.org/icon-theme-spec/icon-theme-spec-latest.html
///
/// This implementation uses libcairo and librsvg as backend. Those libraries are
/// dynamically loaded at runtime so they are not required for building (or linking) the
/// binaries.
pub struct IconHandle {
    /// The icon data
    data: Vec<u8>,
    /// The current size of the icon
    size: i32,
    /// The format of the icon ("png" or "svg")
    format: String,
    /// The backend for image manipulation
    backend: IconHandleBackend,
}

impl IconHandle {
    /// Create a IconHandle instance from data
    /// 
    /// # Arguments
    /// * `data` - The icon data as bytes
    /// 
    /// # Returns
    /// * `Result<Self, IconHandleError>` - The IconHandle instance or an error
    pub fn from_data(data: &[u8]) -> Result<Self, IconHandleError> {
        // Detect format from data
        let format = Self::detect_format(data)?;
        
        // Create backend
        let backend = IconHandleBackend::new()
            .map_err(|e| IconHandleError::Backend(e.to_string()))?;
        
        Ok(Self {
            data: data.to_vec(),
            size: 0, // Will be set when loaded
            format,
            backend,
        })
    }

    /// Create an IconHandle from a file
    /// 
    /// # Arguments
    /// * `path` - Path to the icon file
    /// 
    /// # Returns
    /// * `Result<Self, IconHandleError>` - The IconHandle instance or an error
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, IconHandleError> {
        let mut file = fs::File::open(path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        
        Self::from_data(&data)
    }

    /// Save the icon to a file
    /// 
    /// # Arguments
    /// * `path` - Target path to save the icon
    /// * `format` - Output format (defaults to "png")
    /// 
    /// # Returns
    /// * `Result<(), IconHandleError>` - Success or error
    pub fn save<P: AsRef<Path>>(&self, path: P, format: Option<&str>) -> Result<(), IconHandleError> {
        let path = path.as_ref();
        
        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Use the specified format or the original format
        let format = format.unwrap_or(&self.format);
        
        // Save using the backend
        self.backend.save_icon(&self.data, path, format, self.size)
            .map_err(|e| IconHandleError::Backend(e.to_string()))
    }

    /// Get the current size of the icon
    /// 
    /// # Returns
    /// * `i32` - The size of the icon
    pub fn size(&self) -> i32 {
        self.size
    }

    /// Set a new size for the icon
    /// 
    /// # Arguments
    /// * `size` - The new size
    pub fn set_size(&mut self, size: i32) {
        self.size = size;
    }

    /// Get the format of the icon
    /// 
    /// # Returns
    /// * `&str` - The format ("png" or "svg")
    pub fn format(&self) -> &str {
        &self.format
    }

    // Private helper methods

    fn detect_format(data: &[u8]) -> Result<String, IconHandleError> {
        // Check for PNG signature
        if data.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
            return Ok("png".to_string());
        }
        
        // Check for SVG signature
        if data.starts_with(b"<?xml") || data.starts_with(b"<svg") {
            return Ok("svg".to_string());
        }
        
        Err(IconHandleError::UnsupportedFormat(
            "Unsupported image format. Only PNG and SVG are supported.".to_string()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_icon_handle_creation() {
        let temp_dir = tempdir().unwrap();
        let icon_path = temp_dir.path().join("test.png");
        
        // Create a dummy PNG file
        let mut file = File::create(&icon_path).unwrap();
        file.write_all(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]).unwrap();

        let icon = IconHandle::from_file(&icon_path).unwrap();
        assert_eq!(icon.format(), "png");
    }

    #[test]
    fn test_invalid_icon() {
        let temp_dir = tempdir().unwrap();
        let icon_path = temp_dir.path().join("test.txt");
        
        // Create an invalid file
        let mut file = File::create(&icon_path).unwrap();
        file.write_all(b"invalid content").unwrap();

        assert!(IconHandle::from_file(&icon_path).is_err());
    }
} 