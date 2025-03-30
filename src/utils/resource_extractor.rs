use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::{self, Read, Write};
use crate::utils::payload_entries_cache::PayloadEntriesCache;

/// Error type for resource extraction operations
#[derive(Debug, thiserror::Error)]
pub enum ResourceError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Entry not found: {0}")]
    EntryNotFound(String),
    #[error("Invalid entry type: {0}")]
    InvalidEntryType(String),
}

/// Result type for resource extraction operations
pub type Result<T> = std::result::Result<T, ResourceError>;

/// A struct for extracting resources from an AppImage
pub struct ResourceExtractor {
    cache: PayloadEntriesCache,
}

impl ResourceExtractor {
    /// Create a new ResourceExtractor with the given payload entries cache
    pub fn new(cache: PayloadEntriesCache) -> Self {
        Self { cache }
    }

    /// Extract a single entry into memory
    /// 
    /// # Arguments
    /// 
    /// * `entry_path` - The path of the entry to extract
    /// 
    /// # Returns
    /// 
    /// The contents of the entry as a byte vector
    pub fn extract(&self, entry_path: &str) -> Result<Vec<u8>> {
        let entry_type = self.cache.get_entry_type(entry_path)
            .ok_or_else(|| ResourceError::EntryNotFound(entry_path.to_string()))?;

        if !entry_type.is_file() {
            return Err(ResourceError::InvalidEntryType(format!(
                "Entry {} is not a file",
                entry_path
            )));
        }

        let mut file = File::open(entry_path)?;
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)?;

        Ok(contents)
    }

    /// Extract multiple entries into memory
    /// 
    /// # Arguments
    /// 
    /// * `entry_paths` - The paths of the entries to extract
    /// 
    /// # Returns
    /// 
    /// A map of entry paths to their contents
    pub fn extract_multiple(&self, entry_paths: &[&str]) -> Result<std::collections::HashMap<String, Vec<u8>>> {
        let mut result = std::collections::HashMap::new();

        for &path in entry_paths {
            match self.extract(path) {
                Ok(contents) => {
                    result.insert(path.to_string(), contents);
                }
                Err(e) => {
                    // Log error but continue with other entries
                    eprintln!("Failed to extract {}: {}", path, e);
                }
            }
        }

        Ok(result)
    }

    /// Extract an entry to a specific path
    /// 
    /// # Arguments
    /// 
    /// * `entry_path` - The path of the entry to extract
    /// * `target_path` - The path where the entry should be extracted to
    /// 
    /// # Returns
    /// 
    /// The path where the entry was extracted
    pub fn extract_to<P: AsRef<Path>>(&self, entry_path: &str, target_path: P) -> Result<PathBuf> {
        let entry_type = self.cache.get_entry_type(entry_path)
            .ok_or_else(|| ResourceError::EntryNotFound(entry_path.to_string()))?;

        if !entry_type.is_file() {
            return Err(ResourceError::InvalidEntryType(format!(
                "Entry {} is not a file",
                entry_path
            )));
        }

        let target_path = target_path.as_ref();
        
        // Create parent directories if they don't exist
        if let Some(parent) = target_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Copy the file
        let mut source = File::open(entry_path)?;
        let mut target = File::create(target_path)?;
        io::copy(&mut source, &mut target)?;

        Ok(target_path.to_path_buf())
    }

    /// Extract a text file into memory
    /// 
    /// # Arguments
    /// 
    /// * `entry_path` - The path of the text file to extract
    /// 
    /// # Returns
    /// 
    /// The contents of the text file as a string
    pub fn extract_text(&self, entry_path: &str) -> Result<String> {
        let contents = self.extract(entry_path)?;
        Ok(String::from_utf8_lossy(&contents).into_owned())
    }

    /// Get the main desktop entry path
    /// 
    /// # Returns
    /// 
    /// The path to the main desktop entry file
    pub fn get_main_desktop_entry_path(&self) -> Option<String> {
        // Look for .desktop files in the AppDir
        for entry in self.cache.get_all_entry_paths() {
            if entry.ends_with(".desktop") {
                return Some(entry);
            }
        }
        None
    }

    /// Get the icon path
    /// 
    /// # Returns
    /// 
    /// The path to the icon file
    pub fn get_icon_path(&self) -> Option<String> {
        // Look for icon files in the AppDir
        for entry in self.cache.get_all_entry_paths() {
            if is_icon_path(&entry) {
                return Some(entry);
            }
        }
        None
    }

    /// Get the MIME type package path
    /// 
    /// # Returns
    /// 
    /// The path to the MIME type package file
    pub fn get_mime_type_package_path(&self) -> Option<String> {
        // Look for MIME type package files in the AppDir
        for entry in self.cache.get_all_entry_paths() {
            if is_mime_file(&entry) {
                return Some(entry);
            }
        }
        None
    }
}

/// Check if a path is an icon file
fn is_icon_path(path: &str) -> bool {
    path.ends_with(".png") || path.ends_with(".svg") || path.ends_with(".ico")
}

/// Check if a path is a MIME type file
fn is_mime_file(path: &str) -> bool {
    path.ends_with(".xml") && path.contains("mime")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;

    fn create_test_file<P: AsRef<Path>>(path: P, contents: &[u8]) -> io::Result<()> {
        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = File::create(path)?;
        file.write_all(contents)?;
        Ok(())
    }

    #[test]
    fn test_extract() -> Result<()> {
        let dir = tempdir()?;
        let test_file = dir.path().join("test.txt");
        let contents = b"Hello, World!";
        create_test_file(&test_file, contents)?;

        let cache = PayloadEntriesCache::new(dir.path())?;
        let extractor = ResourceExtractor::new(cache);

        let extracted = extractor.extract(test_file.to_str().unwrap())?;
        assert_eq!(&extracted, contents);

        Ok(())
    }

    #[test]
    fn test_extract_multiple() -> Result<()> {
        let dir = tempdir()?;
        let files = [
            ("file1.txt", b"Hello"),
            ("file2.txt", b"World"),
        ];

        for (name, contents) in &files {
            create_test_file(dir.path().join(name), contents)?;
        }

        let cache = PayloadEntriesCache::new(dir.path())?;
        let extractor = ResourceExtractor::new(cache);

        let paths: Vec<&str> = files.iter().map(|(name, _)| name).collect();
        let extracted = extractor.extract_multiple(&paths)?;

        assert_eq!(extracted.len(), 2);
        assert_eq!(extracted["file1.txt"], b"Hello");
        assert_eq!(extracted["file2.txt"], b"World");

        Ok(())
    }

    #[test]
    fn test_extract_to() -> Result<()> {
        let dir = tempdir()?;
        let source_file = dir.path().join("source.txt");
        let contents = b"Hello, World!";
        create_test_file(&source_file, contents)?;

        let cache = PayloadEntriesCache::new(dir.path())?;
        let extractor = ResourceExtractor::new(cache);

        let target_dir = tempdir()?;
        let target_file = target_dir.path().join("target.txt");
        
        let extracted_path = extractor.extract_to(
            source_file.to_str().unwrap(),
            &target_file
        )?;

        assert_eq!(extracted_path, target_file);
        assert_eq!(
            fs::read_to_string(&target_file)?,
            String::from_utf8_lossy(contents)
        );

        Ok(())
    }

    #[test]
    fn test_extract_text() -> Result<()> {
        let dir = tempdir()?;
        let test_file = dir.path().join("test.txt");
        let contents = "Hello, World!";
        create_test_file(&test_file, contents.as_bytes())?;

        let cache = PayloadEntriesCache::new(dir.path())?;
        let extractor = ResourceExtractor::new(cache);

        let extracted = extractor.extract_text(test_file.to_str().unwrap())?;
        assert_eq!(&extracted, contents);

        Ok(())
    }
} 