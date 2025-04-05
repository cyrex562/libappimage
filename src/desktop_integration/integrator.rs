use std::path::{Path, PathBuf};
use std::fs;
use std::collections::HashMap;
use std::os::unix::fs::PermissionsExt;
use crate::icon_handle::IconHandle;
use crate::path_utils::hash_path;
use crate::resources_extractor::ResourcesExtractor;
use crate::string_sanitizer::StringSanitizer;
use crate::AppImage;
use crate::desktop_integration::editor::DesktopEntryEditor;
use crate::desktop_integration::desktop_entry::DesktopEntry;
use crate::desktop_integration::constants::VENDOR_PREFIX;
use crate::error::{DesktopIntegrationError, DesktopEntryEditError};

/// Integrator instances allow the integration and disintegration of AppImage with XDG compliant desktop environments.
pub struct Integrator {
    app_image: AppImage,
    xdg_data_home: PathBuf,
    app_image_id: String,
    resources_extractor: ResourcesExtractor,
    desktop_entry: DesktopEntry,
}

impl Integrator {
    /// Create an Integrator instance with a custom XDG_DATA_HOME.
    pub fn new(app_image: AppImage, xdg_data_home: impl AsRef<Path>) -> Result<Self, DesktopIntegrationError> {
        let xdg_data_home = xdg_data_home.as_ref().to_path_buf();
        
        if xdg_data_home.as_os_str().is_empty() {
            return Err(DesktopIntegrationError::InvalidParameter("Invalid XDG_DATA_HOME".into()));
        }

        let resources_extractor = ResourcesExtractor::new(app_image.clone());
        let app_image_id = hash_path(app_image.get_path())
            .map_err(|e| DesktopIntegrationError::InvalidParameter(e.to_string()))?;

        // Extract desktop entry
        let desktop_entry_path = resources_extractor.get_desktop_entry_path()
            .ok_or_else(|| DesktopIntegrationError::NotFound("Desktop entry not found".into()))?;
        
        let desktop_entry_data = resources_extractor.extract_text(&desktop_entry_path)
            .map_err(|e| DesktopIntegrationError::DesktopEntry(DesktopEntryEditError::File(std::io::Error::new(std::io::ErrorKind::Other, e))))?;
        
        let desktop_entry = DesktopEntry::parse(&desktop_entry_data)
            .map_err(|e| DesktopIntegrationError::DesktopEntry(DesktopEntryEditError::Format(e.to_string())))?;

        Ok(Self {
            app_image,
            xdg_data_home,
            app_image_id,
            resources_extractor,
            desktop_entry,
        })
    }

    /// Perform the AppImage integration into the Desktop Environment
    pub fn integrate(&self) -> Result<(), DesktopIntegrationError> {
        // Check if integration is allowed
        self.assert_it_should_be_integrated()?;

        // Deploy desktop entry
        self.deploy_desktop_entry()?;

        // Deploy icons
        self.deploy_icons()?;

        // Deploy mime type packages
        self.deploy_mime_type_packages()?;

        Ok(())
    }

    fn assert_it_should_be_integrated(&self) -> Result<(), DesktopIntegrationError> {
        let integrate = self.desktop_entry.get("Desktop Entry/X-AppImage-Integrate");
        if !integrate.is_empty() && !integrate.parse::<bool>().unwrap_or(true) {
            return Err(DesktopIntegrationError::NotSupported(
                "The AppImage explicitly requested to not be integrated".into()
            ));
        }

        let no_display = self.desktop_entry.get("Desktop Entry/NoDisplay");
        if !no_display.is_empty() && no_display.parse::<bool>().unwrap_or(false) {
            return Err(DesktopIntegrationError::NotSupported(
                "The AppImage explicitly requested to not be integrated".into()
            ));
        }

        Ok(())
    }

    fn deploy_desktop_entry(&self) -> Result<(), DesktopIntegrationError> {
        let desktop_entry_deploy_path = self.build_desktop_file_path()?;
        
        // Ensure parent directory exists
        if let Some(parent) = desktop_entry_deploy_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Update references to the deployed resources
        let mut edited_desktop_entry = self.desktop_entry.clone();
        self.edit_desktop_entry(&mut edited_desktop_entry)?;

        // Write file contents
        fs::write(&desktop_entry_deploy_path, edited_desktop_entry.to_string())?;

        // Make it executable
        let mut perms = fs::metadata(&desktop_entry_deploy_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&desktop_entry_deploy_path, perms)?;

        Ok(())
    }

    fn build_desktop_file_path(&self) -> Result<PathBuf, DesktopIntegrationError> {
        let name = self.desktop_entry.get("Desktop Entry/Name");
        if name.is_empty() {
            return Err(DesktopIntegrationError::NotFound("Desktop file does not contain Name entry".into()));
        }

        let sanitized_name = StringSanitizer::new(name).sanitize_for_path();
        let desktop_file_name = format!("{}_{}-{}.desktop", VENDOR_PREFIX, self.app_image_id, sanitized_name);
        
        Ok(self.xdg_data_home.join("applications").join(desktop_file_name))
    }

    fn edit_desktop_entry(&self, entry: &mut DesktopEntry) -> Result<(), DesktopIntegrationError> {
        let mut editor = DesktopEntryEditor::new();
        editor.set_app_image_path(self.app_image.get_path());
        editor.set_identifier(&self.app_image_id);
        editor.edit(entry)?;
        Ok(())
    }

    fn deploy_icons(&self) -> Result<(), DesktopIntegrationError> {
        const DIR_ICON_PATH: &str = ".DirIcon";
        const ICONS_DIR_PATH: &str = "usr/share/icons";

        let icon_name = self.desktop_entry.get("Desktop Entry/Icon");
        if icon_name.is_empty() {
            return Err(DesktopIntegrationError::NotFound("Missing icon field in the desktop entry".into()));
        }

        if icon_name.contains('/') {
            return Err(DesktopIntegrationError::InvalidParameter("Icon field contains path".into()));
        }

        let icon_paths = self.resources_extractor.get_icon_file_paths(&icon_name);

        if icon_paths.is_empty() {
            log::warn!("No icons found at \"{}\"", ICONS_DIR_PATH);

            match self.resources_extractor.extract(DIR_ICON_PATH) {
                Ok(dir_icon_data) => {
                    log::warn!("Using .DirIcon as default app icon");
                    self.deploy_application_icon(&icon_name, &dir_icon_data)?;
                }
                Err(e) => {
                    log::error!("{}", e);
                    log::error!("No icon was generated for: {}", self.app_image.get_path().display());
                }
            }
        } else {
            let mut icon_files_target_paths = HashMap::new();
            for path in icon_paths {
                icon_files_target_paths.insert(path.clone(), self.generate_deploy_path(Path::new(&path))?);
            }
            self.resources_extractor.extract_to(&icon_files_target_paths)?;
        }

        Ok(())
    }

    fn deploy_application_icon(&self, icon_name: &str, icon_data: &[u8]) -> Result<(), DesktopIntegrationError> {
        let icon = IconHandle::from_data(icon_data)?;
        let mut icon_path = PathBuf::from("icons/hicolor");

        let sanitized_name = StringSanitizer::new(icon_name).sanitize_for_path();
        let mut icon_name_builder = String::new();

        if icon.format() == "svg" {
            icon_name_builder.push_str(&sanitized_name);
            icon_name_builder.push_str(".svg");
            icon_path.push("scalable");
        } else {
            icon_name_builder.push_str(&sanitized_name);
            icon_name_builder.push_str(".png");
            icon_path.push(format!("{}x{}", icon.size(), icon.size()));
        }

        icon_path.push("apps");
        icon_path.push(format!("{}_{}_{}", VENDOR_PREFIX, self.app_image_id, icon_name_builder));

        let target_path = self.xdg_data_home.join(&icon_path);
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&target_path, icon_data)?;
        Ok(())
    }

    fn generate_deploy_path(&self, path: &Path) -> Result<PathBuf, DesktopIntegrationError> {
        let file_name = path.file_name()
            .ok_or_else(|| DesktopIntegrationError::InvalidParameter("Invalid file path".into()))?
            .to_str()
            .ok_or_else(|| DesktopIntegrationError::InvalidParameter("Invalid file name".into()))?;

        let sanitized_name = StringSanitizer::new(file_name).sanitize_for_path();
        let new_name = format!("{}_{}_{}", VENDOR_PREFIX, self.app_image_id, sanitized_name);

        Ok(self.xdg_data_home.join(path.with_file_name(new_name)))
    }

    fn deploy_mime_type_packages(&self) -> Result<(), DesktopIntegrationError> {
        // TODO: Implement mime type package deployment
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
    fn test_integrator_creation() {
        let temp_dir = tempdir().unwrap();
        let app_image_path = temp_dir.path().join("test.AppImage");
        
        // Create a dummy AppImage file
        let mut file = File::create(&app_image_path).unwrap();
        file.write_all(b"dummy content").unwrap();

        let app_image = AppImage::new(&app_image_path).unwrap();
        let integrator = Integrator::new(app_image, temp_dir.path()).unwrap();
        
        assert_eq!(integrator.xdg_data_home, temp_dir.path());
    }

    #[test]
    fn test_invalid_xdg_data_home() {
        let temp_dir = tempdir().unwrap();
        let app_image_path = temp_dir.path().join("test.AppImage");
        
        let mut file = File::create(&app_image_path).unwrap();
        file.write_all(b"dummy content").unwrap();

        let app_image = AppImage::new(&app_image_path).unwrap();
        let result = Integrator::new(app_image, "");
        
        assert!(matches!(result, Err(DesktopIntegrationError::InvalidParameter(_))));
    }
} 