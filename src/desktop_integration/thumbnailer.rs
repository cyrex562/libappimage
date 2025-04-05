use std::path::{Path, PathBuf};
use std::fs;
use crate::error::DesktopIntegrationError;
use crate::icon_handle::IconHandle;
use crate::path_utils::hash_path;
use crate::resources_extractor::ResourcesExtractor;
use crate::AppImage;
use crate::desktop_integration::desktop_entry::DesktopEntry;

/// Thumbnails generator for AppImage files
/// 
/// Follows the Thumbnail Managing Standard by FreeDesktop
/// https://specifications.freedesktop.org/thumbnail-spec/0.8.0/index.html
pub struct Thumbnailer {
    xdg_cache_home: PathBuf,
}

impl Thumbnailer {
    const THUMBNAIL_FILE_EXTENSION: &'static str = ".png";
    const NORMAL_THUMBNAILS_PREFIX: &'static str = "thumbnails/normal";
    const LARGE_THUMBNAIL_PREFIX: &'static str = "thumbnails/large";

    /// Creates a Thumbnailer that will create and remove thumbnails at the user XDG_CACHE_HOME dir.
    pub fn new() -> Result<Self, DesktopIntegrationError> {
        let xdg_cache_home = dirs::cache_dir()
            .ok_or_else(|| DesktopIntegrationError::NotFound("XDG_CACHE_HOME not found".to_string()))?;

        Self::with_xdg_cache_home(xdg_cache_home)
    }

    /// Creates a Thumbnailer that will create and remove thumbnails at the specified directory.
    pub fn with_xdg_cache_home(xdg_cache_home: impl AsRef<Path>) -> Result<Self, DesktopIntegrationError> {
        let xdg_cache_home = xdg_cache_home.as_ref().to_path_buf();
        
        if xdg_cache_home.as_os_str().is_empty() {
            return Err(DesktopIntegrationError::InvalidParameter(
                "Invalid XDG_CACHE_HOME".to_string()
            ));
        }

        Ok(Self { xdg_cache_home })
    }

    /// Generate thumbnails for an AppImage file
    /// 
    /// # Arguments
    /// * `app_image` - The AppImage to generate thumbnails for
    /// 
    /// # Returns
    /// * `Result<(), DesktopIntegrationError>` - Success or error
    pub fn generate_thumbnails(&self, app_image: &AppImage) -> Result<(), DesktopIntegrationError> {
        let extractor = ResourcesExtractor::new(app_image.clone());

        // Get the application main icon
        let app_icon = self.get_app_icon_name(&extractor)?;

        // Generate canonical path MD5
        let canonical_path_md5 = hash_path(app_image.get_path())
            .map_err(|e| DesktopIntegrationError::InvalidParameter(e.to_string()))?;

        // Get icon paths
        let app_icons = extractor.get_icon_file_paths(&app_icon);

        // Generate normal size thumbnail
        if let Ok(icon_data) = self.get_icon_data(&extractor, &app_icons, "128x128") {
            self.generate_normal_size_thumbnail(&canonical_path_md5, &icon_data)?;
        }

        // Generate large size thumbnail
        if let Ok(icon_data) = self.get_icon_data(&extractor, &app_icons, "256x256") {
            self.generate_large_size_thumbnail(&canonical_path_md5, &icon_data)?;
        }

        Ok(())
    }

    /// Remove thumbnails for the given AppImage
    /// 
    /// Will find and remove every thumbnail related to the file pointed by the AppImage path.
    /// The files will be identified following the rules described in the Full FreeDesktop Thumbnails spec.
    pub fn remove_thumbnails(&self, app_image_path: &Path) -> Result<(), DesktopIntegrationError> {
        let canonical_path_md5 = hash_path(app_image_path)
            .map_err(|e| DesktopIntegrationError::InvalidParameter(e.to_string()))?;
        
        let normal_thumbnail_path = self.get_normal_thumbnail_path(&canonical_path_md5);
        let large_thumbnail_path = self.get_large_thumbnail_path(&canonical_path_md5);
        
        if normal_thumbnail_path.exists() {
            fs::remove_file(&normal_thumbnail_path)?;
        }
        
        if large_thumbnail_path.exists() {
            fs::remove_file(&large_thumbnail_path)?;
        }
        
        Ok(())
    }

    fn get_normal_thumbnail_path(&self, canonical_path_md5: &str) -> PathBuf {
        self.xdg_cache_home
            .join(Self::NORMAL_THUMBNAILS_PREFIX)
            .join(format!("{}{}", canonical_path_md5, Self::THUMBNAIL_FILE_EXTENSION))
    }

    fn get_large_thumbnail_path(&self, canonical_path_md5: &str) -> PathBuf {
        self.xdg_cache_home
            .join(Self::LARGE_THUMBNAIL_PREFIX)
            .join(format!("{}{}", canonical_path_md5, Self::THUMBNAIL_FILE_EXTENSION))
    }

    fn get_app_icon_name(&self, extractor: &ResourcesExtractor) -> Result<String, DesktopIntegrationError> {
        let desktop_entry_path = extractor.get_desktop_entry_path()
            .ok_or_else(|| DesktopIntegrationError::NotFound("Desktop entry not found".to_string()))?;
        
        let desktop_entry_data = extractor.extract_text(&desktop_entry_path)?;
        let desktop_entry = DesktopEntry::parse(&desktop_entry_data)?;
        
        let icon = desktop_entry.get("Desktop Entry/Icon");
        if icon.is_empty() {
            return Err(DesktopIntegrationError::NotFound("Icon field not found in desktop entry".to_string()));
        }
        Ok(icon.to_string())
    }

    fn get_icon_path(&self, app_icons: &[String], size: &str) -> Result<String, DesktopIntegrationError> {
        app_icons.iter()
            .find(|path| path.contains(size))
            .cloned()
            .ok_or_else(|| DesktopIntegrationError::NotFound(
                format!("No icon found with size {}", size)
            ))
    }

    fn get_icon_data(&self, extractor: &ResourcesExtractor, app_icons: &[String], size: &str) -> Result<Vec<u8>, DesktopIntegrationError> {
        let icon_path = self.get_icon_path(app_icons, size)?;
        extractor.extract(&icon_path)
    }

    fn generate_normal_size_thumbnail(&self, canonical_path_md5: &str, icon_data: &[u8]) -> Result<(), DesktopIntegrationError> {
        let normal_thumbnail_path = self.get_normal_thumbnail_path(canonical_path_md5);
        
        // Ensure parent directory exists
        if let Some(parent) = normal_thumbnail_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        match IconHandle::from_data(icon_data) {
            Ok(mut icon_handle) => {
                icon_handle.set_size(128);
                icon_handle.save(&normal_thumbnail_path, Some("png"))?;
            }
            Err(e) => {
                log::warn!("Unable to resize the application icon into a 128x128 image: \"{}\". It will be written unchanged.", e);
                fs::write(&normal_thumbnail_path, icon_data)?;
            }
        }
        
        Ok(())
    }

    fn generate_large_size_thumbnail(&self, canonical_path_md5: &str, icon_data: &[u8]) -> Result<(), DesktopIntegrationError> {
        let large_thumbnail_path = self.get_large_thumbnail_path(canonical_path_md5);
        
        // Ensure parent directory exists
        if let Some(parent) = large_thumbnail_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        match IconHandle::from_data(icon_data) {
            Ok(mut icon_handle) => {
                icon_handle.set_size(256);
                icon_handle.save(&large_thumbnail_path, Some("png"))?;
            }
            Err(e) => {
                log::warn!("Unable to resize the application icon into a 256x256 image: \"{}\". It will be written unchanged.", e);
                fs::write(&large_thumbnail_path, icon_data)?;
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_thumbnailer_creation() {
        let temp_dir = tempdir().unwrap();
        let thumbnailer = Thumbnailer::with_xdg_cache_home(temp_dir.path()).unwrap();
        assert_eq!(thumbnailer.xdg_cache_home, temp_dir.path());
    }

    #[test]
    fn test_invalid_xdg_cache_home() {
        let result = Thumbnailer::with_xdg_cache_home("");
        assert!(matches!(result, Err(DesktopIntegrationError::InvalidParameter(_))));
    }

    #[test]
    fn test_thumbnail_paths() {
        let temp_dir = tempdir().unwrap();
        let thumbnailer = Thumbnailer::with_xdg_cache_home(temp_dir.path()).unwrap();
        
        let canonical_path_md5 = "test123";
        let normal_path = thumbnailer.get_normal_thumbnail_path(canonical_path_md5);
        let large_path = thumbnailer.get_large_thumbnail_path(canonical_path_md5);
        
        assert!(normal_path.to_string_lossy().contains("thumbnails/normal"));
        assert!(large_path.to_string_lossy().contains("thumbnails/large"));
    }
} 