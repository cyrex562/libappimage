use std::path::{Path, PathBuf};
use std::fs;
use crate::path_utils::hash_path;
use crate::resources_extractor::ResourcesExtractor;
use crate::AppImage;
use crate::error::DesktopIntegrationError;
use crate::desktop_integration::integrator::Integrator;
use crate::desktop_integration::constants::VENDOR_PREFIX;
use crate::desktop_integration::desktop_entry::DesktopEntry;

/// Manages the integration and disintegration of AppImages in the system
pub struct IntegrationManager {
    xdg_data_home: PathBuf,
    #[cfg(feature = "thumbnailer")]
    thumbnailer: crate::desktop_integration::thumbnailer::Thumbnailer,
}

impl IntegrationManager {
    /// Create a new IntegrationManager using the system's XDG_DATA_HOME
    pub fn new() -> Result<Self, DesktopIntegrationError> {
        let xdg_data_home = dirs::data_dir()
            .ok_or_else(|| DesktopIntegrationError::NotFound("XDG_DATA_HOME not found".to_string()))?;

        Self::with_xdg_data_home(xdg_data_home)
    }

    /// Create a new IntegrationManager with a custom XDG_DATA_HOME
    pub fn with_xdg_data_home(xdg_data_home: impl AsRef<Path>) -> Result<Self, DesktopIntegrationError> {
        let xdg_data_home = xdg_data_home.as_ref().to_path_buf();
        
        if xdg_data_home.as_os_str().is_empty() || !xdg_data_home.is_dir() {
            return Err(DesktopIntegrationError::InvalidParameter(
                format!("Invalid XDG_DATA_HOME: {}", xdg_data_home.display())
            ));
        }

        Ok(Self {
            xdg_data_home,
            #[cfg(feature = "thumbnailer")]
            thumbnailer: crate::desktop_integration::thumbnailer::Thumbnailer::new()?,
        })
    }

    /// Register an AppImage in the system
    pub fn register_app_image(&self, app_image: &AppImage) -> Result<(), DesktopIntegrationError> {
        // Try to integrate the AppImage
        let result = {
            let integrator = Integrator::new(app_image.clone(), &self.xdg_data_home)?;
            integrator.integrate()
        };

        // If integration fails, clean up any partially created files
        if result.is_err() {
            self.unregister_app_image(app_image.get_path())?;
        }

        result
    }

    /// Unregister an AppImage from the system
    pub fn unregister_app_image(&self, app_image_path: &Path) -> Result<(), DesktopIntegrationError> {
        let app_image_id = self.generate_app_image_id(app_image_path);

        // Remove desktop entries
        self.remove_matching_files(&self.xdg_data_home.join("applications"), &app_image_id)?;

        // Remove icons
        self.remove_matching_files(&self.xdg_data_home.join("icons"), &app_image_id)?;

        // Remove mime type packages
        self.remove_matching_files(&self.xdg_data_home.join("mime"), &app_image_id)?;

        #[cfg(feature = "thumbnailer")]
        self.thumbnailer.remove_thumbnails(app_image_path)?;

        Ok(())
    }

    /// Check if an AppImage is registered in the system
    pub fn is_registered_app_image(&self, app_image_path: &Path) -> Result<bool, DesktopIntegrationError> {
        let app_image_id = self.generate_app_image_id(app_image_path);
        let apps_path = self.xdg_data_home.join("applications");

        if !apps_path.exists() {
            return Ok(false);
        }

        for entry in fs::read_dir(&apps_path)? {
            let entry = entry?;
            if entry.file_type()?.is_file() && entry.file_name().to_string_lossy().contains(&app_image_id) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Check if an AppImage should be registered
    pub fn should_register_app_image(&self, app_image: &AppImage) -> Result<bool, DesktopIntegrationError> {
        let extractor = ResourcesExtractor::new(app_image.clone());
        let desktop_entry_path = extractor.get_desktop_entry_path()
            .ok_or_else(|| DesktopIntegrationError::NotFound("Desktop entry not found".to_string()))?;

        let desktop_entry_data = extractor.extract_text(&desktop_entry_path)?;
        let desktop_entry = DesktopEntry::parse(&desktop_entry_data)?;

        // Check X-AppImage-Integrate
        let integrate = desktop_entry.get("Desktop Entry/X-AppImage-Integrate");
        if !integrate.is_empty() && !integrate.parse::<bool>().unwrap_or(true) {
            return Ok(false);
        }

        // Check Terminal
        let terminal = desktop_entry.get("Desktop Entry/Terminal");
        if !terminal.is_empty() && terminal.parse::<bool>().unwrap_or(false) {
            return Ok(false);
        }

        Ok(true)
    }

    #[cfg(feature = "thumbnailer")]
    /// Generate thumbnails for an AppImage
    pub fn generate_thumbnails(&self, app_image: &AppImage) -> Result<(), DesktopIntegrationError> {
        self.thumbnailer.generate_thumbnails(app_image)
    }

    #[cfg(feature = "thumbnailer")]
    /// Remove thumbnails for an AppImage
    pub fn remove_thumbnails(&self, app_image_path: &Path) -> Result<(), DesktopIntegrationError> {
        self.thumbnailer.remove_thumbnails(app_image_path)
    }

    fn generate_app_image_id(&self, app_image_path: &Path) -> String {
        format!("{}_{}", VENDOR_PREFIX, hash_path(app_image_path).unwrap_or_default())
    }

    fn remove_matching_files(&self, dir: &Path, hint: &str) -> Result<(), DesktopIntegrationError> {
        if !dir.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && path.to_string_lossy().contains(hint) {
                fs::remove_file(path)?;
            } else if path.is_dir() {
                self.remove_matching_files(&path, hint)?;
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
    fn test_integration_manager_creation() {
        let temp_dir = tempdir().unwrap();
        let manager = IntegrationManager::with_xdg_data_home(temp_dir.path()).unwrap();
        assert_eq!(manager.xdg_data_home, temp_dir.path());
    }

    #[test]
    fn test_invalid_xdg_data_home() {
        let result = IntegrationManager::with_xdg_data_home("");
        assert!(matches!(result, Err(DesktopIntegrationError::InvalidParameter(_))));
    }

    #[test]
    fn test_app_image_id_generation() {
        let temp_dir = tempdir().unwrap();
        let app_image_path = temp_dir.path().join("test.AppImage");
        
        let mut file = File::create(&app_image_path).unwrap();
        file.write_all(b"dummy content").unwrap();

        let manager = IntegrationManager::with_xdg_data_home(temp_dir.path()).unwrap();
        let id = manager.generate_app_image_id(&app_image_path);
        assert!(id.starts_with(VENDOR_PREFIX));
    }
} 