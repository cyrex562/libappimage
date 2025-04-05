use std::path::Path;
use crate::error::AppImageResult;
use crate::desktop_integration::desktop_entry::{DesktopEntry, DesktopEntryExecValue, DesktopEntryStringsValue};
use crate::string_sanitizer::StringSanitizer;

/// Editor for modifying desktop entries from AppImages
pub struct DesktopEntryEditor {
    /// The AppImage file path
    app_image_path: String,
    /// The AppImage version
    app_image_version: String,
    /// The vendor prefix (usually AppImage path md5 sum)
    vendor_prefix: String,
    /// The unique identifier (usually AppImage path md5 sum)
    identifier: String,
}

impl DesktopEntryEditor {
    /// Create a new DesktopEntryEditor
    pub fn new() -> Self {
        Self {
            app_image_path: String::new(),
            app_image_version: String::new(),
            vendor_prefix: "appimagekit".to_string(),
            identifier: String::new(),
        }
    }

    /// Set the AppImage file path
    pub fn set_app_image_path<P: AsRef<Path>>(&mut self, path: P) {
        self.app_image_path = path.as_ref().to_string_lossy().into_owned();
    }

    /// Set the AppImage version
    pub fn set_app_image_version(&mut self, version: &str) {
        self.app_image_version = version.to_string();
    }

    /// Set the vendor prefix
    pub fn set_vendor_prefix(&mut self, prefix: &str) {
        self.vendor_prefix = prefix.to_string();
    }

    /// Set the unique identifier
    pub fn set_identifier(&mut self, id: &str) {
        self.identifier = id.to_string();
    }

    /// Edit the desktop entry according to the set parameters
    pub fn edit(&self, entry: &mut DesktopEntry) -> AppImageResult<()> {
        if !entry.exists("Desktop Entry/Exec") {
            return Err(crate::ffi::ErrorCode::Validation("Missing Desktop Entry".to_string()).into());
        }

        self.set_exec_paths(entry)?;
        self.set_icons(entry)?;
        self.append_version_to_name(entry)?;

        // Set identifier
        entry.set("Desktop Entry/X-AppImage-Identifier", &self.identifier);

        Ok(())
    }

    /// Set Exec and TryExec entries in the Desktop Entry and Desktop Action groups
    fn set_exec_paths(&self, entry: &mut DesktopEntry) -> AppImageResult<()> {
        // Edit Desktop Entry/Exec
        let mut exec_value = DesktopEntryExecValue::parse(entry.get("Desktop Entry/Exec"))?;
        exec_value[0] = self.app_image_path.clone();
        entry.set("Desktop Entry/Exec", &exec_value.to_string());

        // Edit TryExec
        entry.set("Desktop Entry/TryExec", &self.app_image_path);

        // Modify actions Exec entry
        let actions = DesktopEntryStringsValue::parse(entry.get("Desktop Entry/Actions"))?;
        for action in actions.iter() {
            let key_path = format!("Desktop Action {}/Exec", action);
            let mut action_exec = DesktopEntryExecValue::parse(entry.get(&key_path))?;
            action_exec[0] = self.app_image_path.clone();
            entry.set(&key_path, &action_exec.to_string());
        }

        Ok(())
    }

    /// Set Icon entries in the Desktop Entry and Desktop Action groups
    fn set_icons(&self, entry: &mut DesktopEntry) -> AppImageResult<()> {
        if self.identifier.is_empty() {
            return Err(crate::ffi::ErrorCode::Validation("Missing AppImage UUID".to_string()).into());
        }

        // Get all icon paths first
        let paths: Vec<String> = entry.paths()
            .iter()
            .filter(|path| path.contains("/Icon"))
            .cloned()
            .collect();

        // Process each path
        for path in paths {
            // Get the icon name
            let icon_name = entry.get(&path).to_string();
            let sanitized_icon = StringSanitizer::new(&icon_name).sanitize_for_path();
            let new_icon = format!("{}_{}_{}", self.vendor_prefix, self.identifier, sanitized_icon);
            
            // Save old icon value
            let old_key = path.replace("/Icon", "/X-AppImage-Old-Icon");
            entry.set(&old_key, &icon_name);
            
            // Set new icon value
            entry.set(&path, &new_icon);
        }

        Ok(())
    }

    /// Append version to Name entries in the Desktop Entry group
    fn append_version_to_name(&self, entry: &mut DesktopEntry) -> AppImageResult<()> {
        // Set version from external source if available
        if !self.app_image_version.is_empty() {
            entry.set("Desktop Entry/X-AppImage-Version", &self.app_image_version);
        }

        // Get version from entry if not set externally
        let version = if !self.app_image_version.is_empty() {
            &self.app_image_version
        } else if entry.exists("Desktop Entry/X-AppImage-Version") {
            entry.get("Desktop Entry/X-AppImage-Version")
        } else {
            return Ok(());
        };

        // Find name entries and collect modifications
        let mut modifications: Vec<(String, String, String, String)> = Vec::new();
        
        for path in entry.paths().iter().filter(|path| path.contains("Desktop Entry/Name")) {
            let name = entry.get(path).to_string();
            
            // Skip if version is already part of the name
            if name.contains(version) {
                continue;
            }

            let old_key = path.replace("/Name", "/X-AppImage-Old-Name");
            let new_name = format!("{} ({})", name, version);
            modifications.push((old_key, path.clone(), name, new_name));
        }

        // Apply modifications
        for (old_key, path, name, new_name) in modifications {
            entry.set(&old_key, &name);
            entry.set(&path, &new_name);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_desktop_entry_editor() {
        let mut editor = DesktopEntryEditor::new();
        editor.set_app_image_path("/path/to/app.AppImage");
        editor.set_app_image_version("1.0.0");
        editor.set_identifier("test-id");
        editor.set_vendor_prefix("test-vendor");

        let mut entry = DesktopEntry::new();
        entry.set("Desktop Entry/Exec", "old-exec");
        entry.set("Desktop Entry/TryExec", "old-tryexec");
        entry.set("Desktop Entry/Icon", "old-icon");
        entry.set("Desktop Entry/Name", "Test App");
        entry.set("Desktop Entry/Actions", "action1;action2");
        entry.set("Desktop Action action1/Exec", "old-action1-exec");
        entry.set("Desktop Action action2/Exec", "old-action2-exec");

        editor.edit(&mut entry).unwrap();

        assert_eq!(entry.get("Desktop Entry/Exec"), "/path/to/app.AppImage");
        assert_eq!(entry.get("Desktop Entry/TryExec"), "/path/to/app.AppImage");
        assert_eq!(entry.get("Desktop Entry/Icon"), "test-vendor_test-id_old-icon");
        assert_eq!(entry.get("Desktop Entry/Name"), "Test App (1.0.0)");
        assert_eq!(entry.get("Desktop Action action1/Exec"), "/path/to/app.AppImage");
        assert_eq!(entry.get("Desktop Action action2/Exec"), "/path/to/app.AppImage");
        assert_eq!(entry.get("Desktop Entry/X-AppImage-Identifier"), "test-id");
    }
} 