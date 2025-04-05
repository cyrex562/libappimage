use crate::elf::ElfFile;
use crate::error::{AppImageError, AppImageResult};
use crate::format::AppImageFormat;
use crate::handlers::type1::Type1Handler;
use crate::handlers::type2::Type2Handler;
use crate::handlers::Handler;
use crate::traversal::{Traversal, TraversalType1, TraversalType2};
use std::path::{Path, PathBuf};

/// Represents an existing AppImage file. Provides readonly methods to
/// access the AppImage information and contained files.
#[derive(Clone)]
pub struct AppImage {
    path: PathBuf,
    format: AppImageFormat,
}

impl AppImage {
    /// Open the AppImage at the given path
    pub fn new<P: AsRef<Path>>(path: P) -> AppImageResult<Self> {
        let path = path.as_ref().to_path_buf();
        let format = AppImageFormat::from_file(&path)?;

        if !format.is_valid() {
            return Err(AppImageError::InvalidFormat(format!(
                "Unknown AppImage format: {}",
                path.display()
            )));
        }

        Ok(Self { path, format })
    }

    /// Get the AppImage file path
    pub fn get_path(&self) -> &Path {
        &self.path
    }

    /// Get the AppImage format
    pub fn get_format(&self) -> AppImageFormat {
        self.format
    }

    /// Calculate the offset in the AppImage file where the payload filesystem is located
    pub fn get_payload_offset(&self) -> AppImageResult<i64> {
        let elf = ElfFile::new(&self.path).map_err(|e| AppImageError::Elf(e.to_string()))?;

        Ok(elf.get_size() as i64)
    }

    /// Get a traversal iterator for the files contained inside the AppImage
    pub fn files(&self) -> AppImageResult<Box<dyn Traversal>> {
        match self.format {
            AppImageFormat::Type1 => Ok(Box::new(TraversalType1::new(&self.path)?)),
            AppImageFormat::Type2 => Ok(Box::new(TraversalType2::new(&self.path)?)),
            AppImageFormat::Invalid => Err(AppImageError::InvalidFormat(
                "Invalid AppImage format".to_string(),
            )),
        }
    }

    /// Read a file from the AppImage
    pub fn read_file(&self, path: &str) -> AppImageResult<Vec<u8>> {
        let mut handler = self.get_handler()?;
        handler.read_file_into_buf(path)
    }

    /// Extract a file from the AppImage to a target path
    pub fn extract_file<P: AsRef<Path>>(&self, path: &str, target: P) -> AppImageResult<()> {
        let mut handler = self.get_handler()?;
        handler.extract_file(path, target.as_ref())
    }

    /// Get the size of the AppImage file
    pub fn size(&self) -> AppImageResult<u64> {
        std::fs::metadata(&self.path)
            .map(|m| m.len())
            .map_err(AppImageError::Io)
    }

    /// Get the appropriate handler for this AppImage
    fn get_handler(&self) -> AppImageResult<Box<dyn Handler>> {
        match self.format {
            AppImageFormat::Type1 => Ok(Box::new(Type1Handler::new(&self.path)?)),
            AppImageFormat::Type2 => Ok(Box::new(Type2Handler::new(&self.path)?)),
            AppImageFormat::Invalid => Err(AppImageError::InvalidFormat(
                "Invalid AppImage format".to_string(),
            )),
        }
    }
}

impl PartialEq for AppImage {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path && self.format == other.format
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_appimage_creation() -> AppImageResult<()> {
        let temp_dir = tempdir()?;
        let appimage_path = temp_dir.path().join("test.AppImage");

        // Create a test AppImage
        let mut file = fs::File::create(&appimage_path)?;
        file.write_all(b"Hello, World!")?;

        // Create AppImage instance
        let appimage = AppImage::new(&appimage_path)?;
        assert_eq!(appimage.get_path(), appimage_path);
        assert_eq!(appimage.get_format(), AppImageFormat::Type1);

        Ok(())
    }

    #[test]
    fn test_file_operations() -> AppImageResult<()> {
        let temp_dir = tempdir()?;
        let appimage_path = temp_dir.path().join("test.AppImage");

        // Create a test AppImage
        let mut file = fs::File::create(&appimage_path)?;
        file.write_all(b"Hello, World!")?;

        // Test file operations
        let appimage = AppImage::new(&appimage_path)?;

        // Test file reading
        let content = appimage.read_file("test.txt")?;
        assert_eq!(content, b"Hello, World!");

        // Test file extraction
        let extracted_file = temp_dir.path().join("extracted.txt");
        appimage.extract_file("test.txt", &extracted_file)?;
        assert_eq!(fs::read_to_string(extracted_file)?, "Hello, World!");

        // Test file metadata
        assert!(appimage.size()? > 0);

        Ok(())
    }
}
