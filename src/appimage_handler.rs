use std::path::{Path, PathBuf};
use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::io::{self, Read, Write};
use crate::utils::payload_entries_cache::PayloadEntriesCache;
use crate::error::{AppImageError, AppImageResult};

/// Type for the traverse callback function
pub type TraverseCallback = Box<dyn FnMut(&AppImageHandler, &str, &mut dyn std::any::Any)>;

/// A handler for AppImage operations
pub struct AppImageHandler {
    path: PathBuf,
    cache: PayloadEntriesCache,
    is_open: bool,
    handler_type: i32,
}

impl AppImageHandler {
    /// Create a new AppImage handler
    /// 
    /// # Arguments
    /// 
    /// * `path` - The path to the AppImage file
    /// 
    /// # Returns
    /// 
    /// A new AppImage handler instance
    pub fn new<P: AsRef<Path>>(path: P) -> AppImageResult<Self> {
        let path = path.as_ref().to_path_buf();
        let cache = PayloadEntriesCache::new(&path)?;
        
        Ok(Self {
            path,
            cache,
            is_open: true,
            handler_type: 0, // Default type
        })
    }

    /// Get the file name for an entry
    /// 
    /// # Arguments
    /// 
    /// * `entry` - The entry to get the file name for
    /// 
    /// # Returns
    /// 
    /// The file name as a C-compatible string
    pub fn get_file_name(&self, entry: &str) -> AppImageResult<CString> {
        if !self.is_open {
            return Err(AppImageError::InvalidState("Handler is not open".to_string()));
        }

        Ok(CString::new(entry)?)
    }

    /// Extract a file from the AppImage
    /// 
    /// # Arguments
    /// 
    /// * `entry` - The entry to extract
    /// * `target` - The target path to extract to
    pub fn extract_file<P: AsRef<Path>>(&self, entry: &str, target: P) -> AppImageResult<()> {
        if !self.is_open {
            return Err(AppImageError::InvalidState("Handler is not open".to_string()));
        }

        let entry_type = self.cache.get_entry_type(entry)
            .ok_or_else(|| AppImageError::NotFound(entry.to_string()))?;

        if !entry_type.is_file() {
            return Err(AppImageError::NotAFile);
        }

        let target_path = target.as_ref();
        
        // Create parent directories if they don't exist
        if let Some(parent) = target_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Copy the file
        let mut source = std::fs::File::open(entry)?;
        let mut target = std::fs::File::create(target_path)?;
        io::copy(&mut source, &mut target)?;

        Ok(())
    }

    /// Read a file into a new buffer
    /// 
    /// # Arguments
    /// 
    /// * `entry` - The entry to read
    /// 
    /// # Returns
    /// 
    /// The contents of the file as a byte vector
    pub fn read_file_into_new_buffer(&self, entry: &str) -> AppImageResult<Vec<u8>> {
        if !self.is_open {
            return Err(AppImageError::InvalidState("Handler is not open".to_string()));
        }

        let entry_type = self.cache.get_entry_type(entry)
            .ok_or_else(|| AppImageError::NotFound(entry.to_string()))?;

        if !entry_type.is_file() {
            return Err(AppImageError::NotAFile);
        }

        let mut file = std::fs::File::open(entry)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;

        Ok(buffer)
    }

    /// Get the link target for an entry
    /// 
    /// # Arguments
    /// 
    /// * `entry` - The entry to get the link target for
    /// 
    /// # Returns
    /// 
    /// The link target as a C-compatible string
    pub fn get_file_link(&self, entry: &str) -> AppImageResult<CString> {
        if !self.is_open {
            return Err(AppImageError::InvalidState("Handler is not open".to_string()));
        }

        let link_target = self.cache.get_entry_link_target(entry)
            .ok_or_else(|| AppImageError::NotFound(entry.to_string()))?;

        Ok(CString::new(link_target)?)
    }

    /// Traverse all entries in the AppImage
    /// 
    /// # Arguments
    /// 
    /// * `callback` - The callback function to call for each entry
    /// * `user_data` - User data to pass to the callback
    pub fn traverse<T: std::any::Any>(&self, mut callback: TraverseCallback, user_data: &mut T) -> AppImageResult<()> {
        if !self.is_open {
            return Err(AppImageError::InvalidState("Handler is not open".to_string()));
        }

        for entry in self.cache.get_all_entry_paths() {
            callback(self, &entry, user_data);
        }

        Ok(())
    }

    /// Check if the handler is valid
    /// 
    /// # Returns
    /// 
    /// Whether the handler is valid
    pub fn is_valid(&self) -> bool {
        self.is_open && self.path.exists()
    }

    /// Get the handler type
    /// 
    /// # Returns
    /// 
    /// The handler type
    pub fn get_type(&self) -> i32 {
        self.handler_type
    }

    /// Set the handler type
    /// 
    /// # Arguments
    /// 
    /// * `handler_type` - The handler type to set
    pub fn set_type(&mut self, handler_type: i32) {
        self.handler_type = handler_type;
    }
}

/// Create the base directory for a target path
/// 
/// # Arguments
/// 
/// * `target` - The target path to create the base directory for
pub fn mk_base_dir<P: AsRef<Path>>(target: P) -> AppImageResult<()> {
    if let Some(parent) = target.as_ref().parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;

    fn create_test_appimage(dir: &Path) -> io::Result<()> {
        // Create a simple test AppImage structure
        let files = [
            ("file1.txt", "Hello"),
            ("file2.txt", "World"),
            ("link1.txt", "file1.txt"),
        ];

        for (name, contents) in &files {
            let path = dir.join(name);
            if name.ends_with(".txt") && !name.starts_with("link") {
                fs::write(path, contents)?;
            } else if name.starts_with("link") {
                std::os::unix::fs::symlink(contents, path)?;
            }
        }

        Ok(())
    }

    #[test]
    fn test_handler_creation() -> AppImageResult<()> {
        let dir = tempdir()?;
        create_test_appimage(dir.path())?;

        let handler = AppImageHandler::new(dir.path())?;
        assert!(handler.is_valid());

        Ok(())
    }

    #[test]
    fn test_file_operations() -> AppImageResult<()> {
        let dir = tempdir()?;
        create_test_appimage(dir.path())?;

        let handler = AppImageHandler::new(dir.path())?;

        // Test get_file_name
        let file_name = handler.get_file_name("file1.txt")?;
        assert_eq!(file_name.to_str().unwrap(), "file1.txt");

        // Test read_file_into_new_buffer
        let contents = handler.read_file_into_new_buffer("file1.txt")?;
        assert_eq!(String::from_utf8_lossy(&contents), "Hello");

        // Test get_file_link
        let link_target = handler.get_file_link("link1.txt")?;
        assert_eq!(link_target.to_str().unwrap(), "file1.txt");

        Ok(())
    }

    #[test]
    fn test_extract_file() -> AppImageResult<()> {
        let dir = tempdir()?;
        create_test_appimage(dir.path())?;

        let handler = AppImageHandler::new(dir.path())?;
        let target_dir = tempdir()?;
        let target_file = target_dir.path().join("extracted.txt");

        handler.extract_file("file1.txt", &target_file)?;
        assert_eq!(
            fs::read_to_string(&target_file)?,
            "Hello"
        );

        Ok(())
    }

    #[test]
    fn test_traverse() -> AppImageResult<()> {
        let dir = tempdir()?;
        create_test_appimage(dir.path())?;

        let handler = AppImageHandler::new(dir.path())?;
        let mut entries = Vec::new();

        handler.traverse(
            Box::new(|handler: &AppImageHandler, entry: &str, user_data: &mut dyn std::any::Any| {
                if let Some(entries) = user_data.downcast_mut::<Vec<String>>() {
                    entries.push(entry.to_string());
                }
            }),
            &mut entries
        )?;

        assert!(entries.contains(&"file1.txt".to_string()));
        assert!(entries.contains(&"file2.txt".to_string()));
        assert!(entries.contains(&"link1.txt".to_string()));

        Ok(())
    }
} 