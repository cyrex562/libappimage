use std::path::Path;
use std::collections::{HashMap, HashSet};
use crate::AppImage;
use crate::error::AppImageError;
use crate::desktop_integration::error::DesktopIntegrationError;
use crate::desktop_integration::desktop_entry::DesktopEntry;

/// Allows to identify and extract the resources (files) required to integrate an AppImage into the
/// desktop environment in an effective way.
///
/// Using the `PayloadIterator::read` method on symlinks is not reliable as it's not supported on
/// AppImages of type 1 (blame on `libarchive`). To overcome this limitation two iterations over the
/// AppImage will be performed. One to resolve all the links entries and other to actually extract
/// the resources.
pub struct ResourcesExtractor {
    app_image: AppImage,
    entries_cache: PayloadEntriesCache,
}

impl ResourcesExtractor {
    /// Create a new ResourcesExtractor for the given AppImage
    pub fn new(app_image: AppImage) -> Self {
        Self {
            entries_cache: PayloadEntriesCache::new(&app_image),
            app_image,
        }
    }

    /// Read an entry into memory, if the entry is a link it will be resolved.
    pub fn extract(&self, path: &str) -> Result<Vec<u8>, DesktopIntegrationError> {
        let mut result = HashMap::new();
        result.insert(path.to_string(), Vec::new());
        
        self.extract_to(&result)?;
        
        Ok(result.remove(path).unwrap_or_default())
    }

    /// Read each entry into memory, if the entry is a link it will be resolved.
    pub fn extract_multiple(&self, paths: &[String]) -> Result<HashMap<String, Vec<u8>>, DesktopIntegrationError> {
        let mut result = HashMap::new();
        for path in paths {
            result.insert(path.clone(), Vec::new());
        }
        
        self.extract_to(&result)?;
        
        Ok(result)
    }

    /// Extract entries listed in 'first' member of the targets map to the 'second' member
    /// of the targets map. Will resolve links to regular files.
    pub fn extract_to(&self, targets: &HashMap<String, PathBuf>) -> Result<(), DesktopIntegrationError> {
        // Resolve links to ensure proper extraction
        let mut real_targets = HashMap::new();
        
        for (source, target) in targets {
            if self.entries_cache.get_entry_type(source) == PayloadEntryType::Link {
                let real_target = self.entries_cache.get_entry_link_target(source)?;
                real_targets.insert(real_target, target.clone());
            } else {
                real_targets.insert(source.clone(), target.clone());
            }
        }

        // Iterate over all file paths in the AppImage
        for entry in self.app_image.files() {
            let path = entry.path();
            
            // Check if we have to extract the file
            if let Some(target_path) = real_targets.get(path) {
                // Ensure parent directory exists
                if let Some(parent) = target_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                
                // Extract the file
                let mut file = std::fs::File::create(target_path)?;
                entry.read_to(&mut file)?;
            }
        }
        
        Ok(())
    }

    /// Read an entry into a String, if the entry is a link it will be resolved.
    /// Should only be used in text files.
    pub fn extract_text(&self, path: &str) -> Result<String, DesktopIntegrationError> {
        let data = self.extract(path)?;
        String::from_utf8(data)
            .map_err(|e| DesktopIntegrationError::InvalidParameter(
                format!("Invalid UTF-8 in text file: {}", e)
            ))
    }

    /// Get the path to the main desktop entry of the AppImage
    pub fn get_desktop_entry_path(&self) -> Option<String> {
        for path in self.entries_cache.get_entries_paths() {
            if Self::is_main_desktop_file(&path) {
                return Some(path);
            }
        }
        None
    }

    /// Get icon file paths for the given icon name
    /// 
    /// Icons are expected to be located in "usr/share/icons/" according to the FreeDesktop
    /// Icon Theme Specification. This method look for entries in that path whose file name
    /// matches to the iconName
    pub fn get_icon_file_paths(&self, icon_name: &str) -> Vec<String> {
        self.entries_cache.get_entries_paths()
            .into_iter()
            .filter(|path| Self::is_icon_file(path) && path.contains(icon_name))
            .collect()
    }

    /// Get MIME type package paths
    /// 
    /// MIME-Type packages are XML files located in usr/share/mime/packages according to the
    /// Shared MIME-info Database specification.
    pub fn get_mime_type_packages_paths(&self) -> Vec<String> {
        self.entries_cache.get_entries_paths()
            .into_iter()
            .filter(|path| Self::is_mime_file(path))
            .collect()
    }

    fn is_icon_file(file_name: &str) -> bool {
        file_name.contains("usr/share/icons")
    }

    fn is_main_desktop_file(file_name: &str) -> bool {
        file_name.ends_with(".desktop") && !file_name.contains('/')
    }

    fn is_mime_file(file_name: &str) -> bool {
        const PREFIX: &str = "usr/share/mime/packages/";
        const SUFFIX: &str = ".xml";
        
        file_name.starts_with(PREFIX) && 
        file_name.ends_with(SUFFIX) && 
        file_name.len() > PREFIX.len() + SUFFIX.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_resources_extractor_creation() {
        let temp_dir = tempdir().unwrap();
        let app_image_path = temp_dir.path().join("test.AppImage");
        
        let mut file = File::create(&app_image_path).unwrap();
        file.write_all(b"dummy content").unwrap();

        let app_image = AppImage::new(&app_image_path).unwrap();
        let extractor = ResourcesExtractor::new(app_image);
        assert!(extractor.get_desktop_entry_path().is_none());
    }

    #[test]
    fn test_icon_path_detection() {
        assert!(ResourcesExtractor::is_icon_file("usr/share/icons/hicolor/128x128/apps/test.png"));
        assert!(!ResourcesExtractor::is_icon_file("usr/share/applications/test.desktop"));
    }

    #[test]
    fn test_mime_file_detection() {
        assert!(ResourcesExtractor::is_mime_file("usr/share/mime/packages/test.xml"));
        assert!(!ResourcesExtractor::is_mime_file("usr/share/mime/packages/test.txt"));
    }
} 