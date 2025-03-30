mod type1;
mod type2;

pub use type1::Type1Handler;
pub use type2::Type2Handler;

use std::any::Any;
use std::path::Path;
use crate::error::AppImageResult;
use crate::format::AppImageFormat;

/// Trait for AppImage handlers
pub trait Handler {
    /// Get the handler type
    fn get_type(&self) -> i32;

    /// Set the handler type
    fn set_type(&mut self, handler_type: i32);

    /// Get the file name for an entry
    fn get_file_name(&self, entry: &str) -> AppImageResult<String>;

    /// Extract a file from the AppImage
    fn extract_file(&mut self, path: &str, target: &Path) -> AppImageResult<()>;

    /// Read a file into a buffer
    fn read_file_into_buf(&mut self, path: &str) -> AppImageResult<Vec<u8>>;

    /// Get the link target for an entry
    fn get_file_link(&mut self, entry: &str) -> AppImageResult<Option<String>>;

    /// Traverse all entries in the AppImage
    fn traverse(&mut self, callback: Box<dyn FnMut(&str, &[u8], &mut dyn std::any::Any) -> AppImageResult<()>>) -> AppImageResult<()>;
}

/// Create a new handler for the given AppImage path
pub fn create_handler(path: &Path) -> AppImageResult<Box<dyn Handler>> {
    let format = AppImageFormat::from_file(path)?;
    match format {
        AppImageFormat::Type1 => Ok(Box::new(Type1Handler::new(path)?)),
        AppImageFormat::Type2 => Ok(Box::new(Type2Handler::new(path)?)),
        AppImageFormat::Invalid => Err(crate::error::AppImageError::InvalidFormat("Invalid AppImage format".to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    #[test]
    fn test_handler() {
        let test_content = b"Hello, World!";
        let appimage_path = "test.AppImage";
        let target_path = "test.txt";

        // Create a test file
        let mut file = fs::File::create(appimage_path).unwrap();
        file.write_all(test_content).unwrap();

        // Test handler creation
        let mut handler = create_handler(Path::new(appimage_path)).unwrap();

        // Test traversal
        handler.traverse(Box::new(|path, contents, _| {
            assert_eq!(path, "test.txt");
            assert_eq!(contents, test_content);
            Ok(())
        })).unwrap();

        // Test file extraction
        handler.extract_file("test.txt", Path::new(target_path)).unwrap();
        assert_eq!(fs::read(target_path).unwrap(), test_content);

        // Test file reading
        let contents = handler.read_file_into_buf("test.txt").unwrap();
        assert_eq!(contents, test_content);

        // Clean up
        fs::remove_file(appimage_path).unwrap();
        fs::remove_file(target_path).unwrap();
    }
} 