use std::fs;
use std::io::Write;
use std::path::Path;
use tempfile::tempdir;
use libappimage::{
    AppImage, AppImageResult, AppImageError,
    utils::{elf_file::ElfFile, digest::type2_digest_md5, md5::md5},
    handlers::{Handler, HandlerType, create_handler},
    legacy::*,
    traversal::{Traversal, TraversalType1, TraversalType2},
    payload_types::PayloadEntryType,
    desktop_integration::{IntegrationManager, DesktopEntryEditor, DesktopEntry, Thumbnailer},
};
use std::env;
use std::ffi::OsString;
use std::os::unix::fs::PermissionsExt;

pub struct TestBase {
    temp_dir: tempfile::TempDir,
    temp_home: std::path::PathBuf,
    old_home: Option<OsString>,
    old_xdg_data_home: Option<OsString>,
    old_xdg_config_home: Option<OsString>,
    pub elf_file_path: std::path::PathBuf,
    pub iso_9660_file_path: std::path::PathBuf,
    pub appimage_type_1_file_path: std::path::PathBuf,
    pub appimage_type_1_no_magic_file_path: std::path::PathBuf,
    pub appimage_type_2_file_path: std::path::PathBuf,
    pub appimage_type_2_versioned_path: std::path::PathBuf,
    pub appimage_type_2_terminal_file_path: std::path::PathBuf,
    pub appimage_type_2_shall_not_integrate_path: std::path::PathBuf,
}

impl TestBase {
    pub fn new() -> Self {
        let temp_dir = tempdir().unwrap();
        let temp_home = temp_dir.path().join("HOME");
        fs::create_dir_all(&temp_home).unwrap();

        // Store old environment variables
        let old_home = env::var_os("HOME");
        let old_xdg_data_home = env::var_os("XDG_DATA_HOME");
        let old_xdg_config_home = env::var_os("XDG_CONFIG_HOME");

        // Set new environment variables
        let new_xdg_data_home = temp_home.join(".local/share");
        let new_xdg_config_home = temp_home.join(".config");
        
        env::set_var("HOME", temp_home.to_str().unwrap());
        env::set_var("XDG_DATA_HOME", new_xdg_data_home.to_str().unwrap());
        env::set_var("XDG_CONFIG_HOME", new_xdg_config_home.to_str().unwrap());

        // Create test files
        let test_data_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data");
        
        Self {
            temp_dir,
            temp_home,
            old_home,
            old_xdg_data_home,
            old_xdg_config_home,
            elf_file_path: test_data_dir.join("elffile"),
            iso_9660_file_path: test_data_dir.join("minimal.iso"),
            appimage_type_1_file_path: test_data_dir.join("AppImageExtract_6-x86_64.AppImage"),
            appimage_type_1_no_magic_file_path: test_data_dir.join("AppImageExtract_6_no_magic_bytes-x86_64.AppImage"),
            appimage_type_2_file_path: test_data_dir.join("Echo-x86_64.AppImage"),
            appimage_type_2_versioned_path: test_data_dir.join("Echo-test1234-x86_64.AppImage"),
            appimage_type_2_terminal_file_path: test_data_dir.join("appimagetool-x86_64.AppImage"),
            appimage_type_2_shall_not_integrate_path: test_data_dir.join("Echo-no-integrate-x86_64.AppImage"),
        }
    }

    pub fn temp_dir(&self) -> &Path {
        self.temp_dir.path()
    }

    pub fn temp_home(&self) -> &Path {
        &self.temp_home
    }

    pub fn is_file(path: &Path) -> bool {
        path.metadata().map(|m| m.is_file()).unwrap_or(false)
    }

    pub fn is_dir(path: &Path) -> bool {
        path.metadata().map(|m| m.is_dir()).unwrap_or(false)
    }

    pub fn split_string(s: &str, delim: char) -> Vec<String> {
        s.split(delim).map(String::from).collect()
    }

    pub fn is_empty_string(s: &str) -> bool {
        s.is_empty() || s.chars().all(|c| c == ' ' || c == '\t')
    }

    pub fn string_starts_with(s: &str, prefix: &str) -> bool {
        s.starts_with(prefix)
    }
}

impl Drop for TestBase {
    fn drop(&mut self) {
        // Restore old environment variables
        if let Some(old_home) = self.old_home.take() {
            env::set_var("HOME", old_home);
        } else {
            env::remove_var("HOME");
        }

        if let Some(old_xdg_data_home) = self.old_xdg_data_home.take() {
            env::set_var("XDG_DATA_HOME", old_xdg_data_home);
        } else {
            env::remove_var("XDG_DATA_HOME");
        }

        if let Some(old_xdg_config_home) = self.old_xdg_config_home.take() {
            env::set_var("XDG_CONFIG_HOME", old_xdg_config_home);
        } else {
            env::remove_var("XDG_CONFIG_HOME");
        }
    }
}

fn create_test_appimage(dir: &Path, name: &str, content: &[u8]) -> AppImageResult<()> {
    let path = dir.join(name);
    let mut file = fs::File::create(&path)?;
    file.write_all(content)?;
    Ok(())
}

#[test]
fn test_get_elf_size() {
    let dir = tempdir().unwrap();
    let elf_path = dir.path().join("test.elf");
    
    // Create a test ELF file
    let mut file = fs::File::create(&elf_path).unwrap();
    file.write_all(b"Hello, World!").unwrap();
    
    // Get ELF size
    let size = get_elf_size(&elf_path).unwrap();
    assert!(size > 0);
}

#[test]
fn test_is_terminal_app() {
    let dir = tempdir().unwrap();
    let appimage_path = dir.path().join("test.AppImage");
    
    // Create a test AppImage
    let mut file = fs::File::create(&appimage_path).unwrap();
    file.write_all(b"Hello, World!").unwrap();
    
    // Check if it's a terminal app
    let is_terminal = type1_is_terminal_app(&appimage_path).unwrap();
    assert!(!is_terminal);
}

#[test]
fn test_type2_digest_md5() {
    let dir = tempdir().unwrap();
    let appimage_path = dir.path().join("test.AppImage");
    
    // Create a test AppImage
    let mut file = fs::File::create(&appimage_path).unwrap();
    file.write_all(b"Hello, World!").unwrap();
    
    // Calculate MD5 digest
    let digest = type2_digest_md5(&appimage_path).unwrap();
    assert!(!digest.is_empty());
}

#[test]
fn test_handler_creation() {
    let dir = tempdir().unwrap();
    let appimage_path = dir.path().join("test.AppImage");
    
    // Create a test AppImage
    let mut file = fs::File::create(&appimage_path).unwrap();
    file.write_all(b"Hello, World!").unwrap();
    
    // Create handler
    let handler = create_handler(&appimage_path).unwrap();
    assert!(handler.is_some());
}

#[test]
fn test_appimage_creation() {
    let dir = tempdir().unwrap();
    let appimage_path = dir.path().join("test.AppImage");
    
    // Create a test AppImage
    let mut file = fs::File::create(&appimage_path).unwrap();
    file.write_all(b"Hello, World!").unwrap();
    
    // Create AppImage instance
    let appimage = AppImage::new(&appimage_path).unwrap();
    assert_eq!(appimage.path(), appimage_path);
}

#[test]
fn test_elf_file_operations() {
    let dir = tempdir().unwrap();
    let elf_path = dir.path().join("test.elf");
    
    // Create a test ELF file
    let mut file = fs::File::create(&elf_path).unwrap();
    file.write_all(b"Hello, World!").unwrap();
    
    // Create ELF file instance
    let elf = ElfFile::new(&elf_path).unwrap();
    assert!(elf.get_size() > 0);
}

#[cfg(feature = "desktop-integration")]
mod desktop_integration_tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_desktop_entry() -> DesktopEntry {
        let mut entry = DesktopEntry::new();
        
        // Set basic entries
        entry.set("Desktop Entry/Version", "1.0");
        entry.set("Desktop Entry/Type", "Application");
        entry.set("Desktop Entry/Name", "Foo Viewer");
        entry.set("Desktop Entry/Name[es]", "Visor de Foo");
        entry.set("Desktop Entry/Name[en]", "Foo Viewer 0.1.1");
        entry.set("Desktop Entry/Comment", "The best viewer for Foo objects available!");
        entry.set("Desktop Entry/TryExec", "fooview");
        entry.set("Desktop Entry/Exec", "fooview %F");
        entry.set("Desktop Entry/Icon", "fooview");
        entry.set("Desktop Entry/Icon[es]", "fooview-es");
        entry.set("Desktop Entry/MimeType", "image/x-foo;");
        entry.set("Desktop Entry/Actions", "Gallery;Create;");
        
        // Set Gallery action
        entry.set("Desktop Action Gallery/Exec", "fooview --gallery");
        entry.set("Desktop Action Gallery/Name", "Browse Gallery");
        
        // Set Create action
        entry.set("Desktop Action Create/Exec", "fooview --create-new");
        entry.set("Desktop Action Create/Name", "Create a new Foo!");
        entry.set("Desktop Action Create/Icon", "fooview-new");
        
        entry
    }

    #[test]
    fn test_desktop_entry_editor_set_path() {
        let dir = tempdir().unwrap();
        let appimage_path = dir.path().join("Echo-x86_64.AppImage");
        let mut file = fs::File::create(&appimage_path).unwrap();
        file.write_all(b"Test AppImage").unwrap();
        
        let mut entry = create_test_desktop_entry();
        let mut editor = DesktopEntryEditor::new();
        editor.set_appimage_path(&appimage_path);
        editor.set_identifier("uuid");
        editor.edit(&mut entry).unwrap();
        
        assert_eq!(entry.get("Desktop Entry/Exec").unwrap(), format!("{} %F", appimage_path.display()));
        assert_eq!(entry.get("Desktop Entry/TryExec").unwrap(), appimage_path.to_str().unwrap());
        assert_eq!(entry.get("Desktop Action Gallery/Exec").unwrap(), format!("{} --gallery", appimage_path.display()));
        assert_eq!(entry.get("Desktop Action Create/Exec").unwrap(), format!("{} --create-new", appimage_path.display()));
    }

    #[test]
    fn test_desktop_entry_editor_set_icons() {
        let dir = tempdir().unwrap();
        let appimage_path = dir.path().join("Echo-x86_64.AppImage");
        let mut file = fs::File::create(&appimage_path).unwrap();
        file.write_all(b"Test AppImage").unwrap();
        
        let mut entry = create_test_desktop_entry();
        let mut editor = DesktopEntryEditor::new();
        editor.set_vendor_prefix("test");
        
        let appimage_path_md5 = md5(appimage_path.to_str().unwrap().as_bytes());
        editor.set_identifier(&appimage_path_md5);
        editor.edit(&mut entry).unwrap();
        
        assert_eq!(entry.get("Desktop Entry/Icon").unwrap(), format!("test_{}_fooview", appimage_path_md5));
        assert_eq!(entry.get("Desktop Entry/Icon[es]").unwrap(), format!("test_{}_fooview-es", appimage_path_md5));
        assert_eq!(entry.get("Desktop Action Create/Icon").unwrap(), format!("test_{}_fooview-new", appimage_path_md5));
        
        assert_eq!(entry.get("Desktop Entry/X-AppImage-Old-Icon").unwrap(), "fooview");
        assert_eq!(entry.get("Desktop Entry/X-AppImage-Old-Icon[es]").unwrap(), "fooview-es");
        assert_eq!(entry.get("Desktop Action Create/X-AppImage-Old-Icon").unwrap(), "fooview-new");
    }

    #[test]
    fn test_desktop_entry_editor_set_version() {
        let dir = tempdir().unwrap();
        let appimage_path = dir.path().join("Echo-x86_64.AppImage");
        let mut file = fs::File::create(&appimage_path).unwrap();
        file.write_all(b"Test AppImage").unwrap();
        
        let mut entry = create_test_desktop_entry();
        let mut editor = DesktopEntryEditor::new();
        editor.set_vendor_prefix("prefix");
        editor.set_identifier("uuid");
        editor.set_appimage_version("0.1.1");
        editor.edit(&mut entry).unwrap();
        
        assert_eq!(entry.get("Desktop Entry/Name").unwrap(), "Foo Viewer (0.1.1)");
        assert_eq!(entry.get("Desktop Entry/Name[en]").unwrap(), "Foo Viewer 0.1.1");
        assert_eq!(entry.get("Desktop Entry/Name[es]").unwrap(), "Visor de Foo (0.1.1)");
        
        assert_eq!(entry.get("Desktop Entry/X-AppImage-Old-Name").unwrap(), "Foo Viewer");
        assert!(entry.get("Desktop Entry/X-AppImage-Old-Name[en]").is_none());
        assert_eq!(entry.get("Desktop Entry/X-AppImage-Old-Name[es]").unwrap(), "Visor de Foo");
    }

    #[test]
    fn test_desktop_entry_editor_set_identifier() {
        let mut entry = create_test_desktop_entry();
        let mut editor = DesktopEntryEditor::new();
        editor.set_vendor_prefix("prefix");
        editor.set_identifier("uuid");
        editor.set_appimage_version("0.1.1");
        editor.edit(&mut entry).unwrap();
        
        assert_eq!(entry.get("Desktop Entry/X-AppImage-Identifier").unwrap(), "uuid");
    }

    #[test]
    fn test_integrate_echo_appimage() {
        let dir = tempdir().unwrap();
        let appimage_path = dir.path().join("Echo-x86_64.AppImage");
        let mut file = fs::File::create(&appimage_path).unwrap();
        file.write_all(b"Test AppImage").unwrap();
        
        let appimage = AppImage::new(&appimage_path).unwrap();
        let manager = IntegrationManager::new();
        manager.register_app_image(&appimage).unwrap();
        
        let appimage_path_md5 = md5(appimage_path.to_str().unwrap().as_bytes());
        let expected_desktop_path = dir.path().join(format!("applications/appimagekit_{}-Echo.desktop", appimage_path_md5));
        let expected_icon_path = dir.path().join(format!("icons/hicolor/scalable/apps/appimagekit_{}_utilities-terminal.svg", appimage_path_md5));
        
        assert!(expected_desktop_path.exists());
        assert!(expected_icon_path.exists());
    }

    #[test]
    fn test_integrate_appimage_extract() {
        let dir = tempdir().unwrap();
        let appimage_path = dir.path().join("AppImageExtract_6-x86_64.AppImage");
        let mut file = fs::File::create(&appimage_path).unwrap();
        file.write_all(b"Test AppImage").unwrap();
        
        let appimage = AppImage::new(&appimage_path).unwrap();
        let manager = IntegrationManager::new();
        manager.register_app_image(&appimage).unwrap();
        
        let appimage_path_md5 = md5(appimage_path.to_str().unwrap().as_bytes());
        let expected_desktop_path = dir.path().join(format!("applications/appimagekit_{}-AppImageExtract.desktop", appimage_path_md5));
        let expected_icon_path = dir.path().join(format!("icons/hicolor/48x48/apps/appimagekit_{}_AppImageExtract.png", appimage_path_md5));
        
        assert!(expected_desktop_path.exists());
        assert!(expected_icon_path.exists());
    }

    #[test]
    fn test_integrate_echo_no_integrate() {
        let dir = tempdir().unwrap();
        let appimage_path = dir.path().join("Echo-no-integrate-x86_64.AppImage");
        let mut file = fs::File::create(&appimage_path).unwrap();
        file.write_all(b"Test AppImage").unwrap();
        
        let appimage = AppImage::new(&appimage_path).unwrap();
        let manager = IntegrationManager::new();
        
        assert!(manager.register_app_image(&appimage).is_err());
    }

    #[test]
    fn test_empty_xdg_data_dir() {
        let dir = tempdir().unwrap();
        let appimage_path = dir.path().join("Echo-no-integrate-x86_64.AppImage");
        let mut file = fs::File::create(&appimage_path).unwrap();
        file.write_all(b"Test AppImage").unwrap();
        
        let appimage = AppImage::new(&appimage_path).unwrap();
        let manager = IntegrationManager::with_data_dir("");
        
        assert!(manager.register_app_image(&appimage).is_err());
    }

    #[test]
    fn test_malformed_desktop_entry() {
        let dir = tempdir().unwrap();
        let appimage_path = dir.path().join("broken-desktop-file-x86_64.AppImage");
        let mut file = fs::File::create(&appimage_path).unwrap();
        file.write_all(b"Test AppImage").unwrap();
        
        let appimage = AppImage::new(&appimage_path).unwrap();
        let manager = IntegrationManager::new();
        
        assert!(manager.register_app_image(&appimage).is_err());
    }
}

#[test]
fn test_error_handling() {
    let dir = tempdir().unwrap();
    let non_existent_path = dir.path().join("non_existent.AppImage");
    
    // Test error handling for non-existent file
    let result = AppImage::new(&non_existent_path);
    assert!(result.is_err());
    
    // Test error handling for invalid ELF file
    let invalid_elf_path = dir.path().join("invalid.elf");
    let mut file = fs::File::create(&invalid_elf_path).unwrap();
    file.write_all(b"Not an ELF file").unwrap();
    
    let result = ElfFile::new(&invalid_elf_path);
    assert!(result.is_err());
}

#[test]
fn test_handler_type_detection() {
    let dir = tempdir().unwrap();
    
    // Test Type 1 AppImage
    let type1_path = dir.path().join("type1.AppImage");
    let mut file = fs::File::create(&type1_path).unwrap();
    file.write_all(b"Type 1 AppImage").unwrap();
    
    let handler = create_handler(&type1_path).unwrap();
    assert!(handler.is_some());
    
    // Test Type 2 AppImage
    let type2_path = dir.path().join("type2.AppImage");
    let mut file = fs::File::create(&type2_path).unwrap();
    file.write_all(b"Type 2 AppImage").unwrap();
    
    let handler = create_handler(&type2_path).unwrap();
    assert!(handler.is_some());
}

#[test]
fn test_file_operations() {
    let dir = tempdir().unwrap();
    let appimage_path = dir.path().join("test.AppImage");
    
    // Create a test AppImage
    let mut file = fs::File::create(&appimage_path).unwrap();
    file.write_all(b"Hello, World!").unwrap();
    
    // Test file operations
    let appimage = AppImage::new(&appimage_path).unwrap();
    
    // Test file reading
    let content = fs::read(&appimage_path).unwrap();
    assert_eq!(content, b"Hello, World!");
    
    // Test file metadata
    let metadata = fs::metadata(&appimage_path).unwrap();
    assert!(metadata.is_file());
    assert!(metadata.len() > 0);
}

#[cfg(test)]
mod traversal_tests {
    use super::*;
    use std::collections::HashMap;
    use std::io::Read;

    #[test]
    fn test_traversal_type1() {
        let dir = tempdir().unwrap();
        let appimage_path = dir.path().join("AppImageExtract_6-x86_64.AppImage");
        
        // Create a test AppImage with expected structure
        let mut file = fs::File::create(&appimage_path).unwrap();
        file.write_all(b"Test AppImage Type 1").unwrap();
        
        let traversal = TraversalType1::new(&appimage_path).unwrap();
        assert!(!traversal.is_completed());

        let mut expected_entries = HashMap::new();
        expected_entries.insert("usr", PayloadEntryType::Dir);
        expected_entries.insert("usr/lib", PayloadEntryType::Dir);
        expected_entries.insert("usr/bin", PayloadEntryType::Dir);
        expected_entries.insert("AppRun", PayloadEntryType::Regular);
        expected_entries.insert("AppImageExtract.desktop", PayloadEntryType::Regular);
        expected_entries.insert(".DirIcon", PayloadEntryType::Regular);
        expected_entries.insert("AppImageExtract.png", PayloadEntryType::Link);
        expected_entries.insert("usr/bin/appimageextract", PayloadEntryType::Regular);
        expected_entries.insert("usr/lib/libisoburn.so.1", PayloadEntryType::Regular);
        expected_entries.insert("usr/bin/xorriso", PayloadEntryType::Regular);
        expected_entries.insert("usr/lib/libburn.so.4", PayloadEntryType::Regular);
        expected_entries.insert("usr/lib/libisofs.so.6", PayloadEntryType::Regular);

        let mut traversal = traversal;
        while !traversal.is_completed() {
            let entry_name = traversal.get_entry_path();
            let entry_type = traversal.get_entry_type();
            
            assert!(expected_entries.contains_key(entry_name));
            assert_eq!(expected_entries[entry_name], entry_type);
            
            expected_entries.remove(entry_name);
            traversal.next().unwrap();
        }

        assert!(expected_entries.is_empty());
    }

    #[test]
    fn test_traversal_type1_extract() {
        let dir = tempdir().unwrap();
        let appimage_path = dir.path().join("AppImageExtract_6-x86_64.AppImage");
        let tmp_dir = tempdir().unwrap();
        let tmp_file = tmp_dir.path().join("tempfile");
        
        // Create a test AppImage
        let mut file = fs::File::create(&appimage_path).unwrap();
        file.write_all(b"Test AppImage Type 1").unwrap();
        
        let mut traversal = TraversalType1::new(&appimage_path).unwrap();
        
        while !traversal.is_completed() {
            if traversal.get_entry_path() == "AppImageExtract.desktop" {
                traversal.extract(&tmp_file).unwrap();
                assert!(tmp_file.exists());
                assert!(fs::metadata(&tmp_file).unwrap().len() > 0);
                break;
            }
            traversal.next().unwrap();
        }
    }

    #[test]
    fn test_traversal_type1_read() {
        let dir = tempdir().unwrap();
        let appimage_path = dir.path().join("AppImageExtract_6-x86_64.AppImage");
        
        // Create a test AppImage with desktop entry
        let desktop_content = "[Desktop Entry]\n\
                             Name=AppImageExtract\n\
                             Exec=appimageextract\n\
                             Icon=AppImageExtract\n\
                             Terminal=true\n\
                             Type=Application\n\
                             Categories=Development;\n\
                             Comment=Extract AppImage contents, part of AppImageKit\n\
                             StartupNotify=true\n";
        
        let mut file = fs::File::create(&appimage_path).unwrap();
        file.write_all(desktop_content.as_bytes()).unwrap();
        
        let mut traversal = TraversalType1::new(&appimage_path).unwrap();
        
        while !traversal.is_completed() {
            if traversal.get_entry_path() == "AppImageExtract.desktop" {
                let mut content = String::new();
                traversal.read().unwrap().read_to_string(&mut content).unwrap();
                assert_eq!(content, desktop_content);
                
                // Try re-reading the entry
                let mut content = String::new();
                traversal.read().unwrap().read_to_string(&mut content).unwrap();
                assert!(content.is_empty());
                break;
            }
            traversal.next().unwrap();
        }
    }

    #[test]
    fn test_traversal_type1_get_entry_link() {
        let dir = tempdir().unwrap();
        let appimage_path = dir.path().join("AppImageExtract_6-x86_64.AppImage");
        
        // Create a test AppImage
        let mut file = fs::File::create(&appimage_path).unwrap();
        file.write_all(b"Test AppImage Type 1").unwrap();
        
        let mut traversal = TraversalType1::new(&appimage_path).unwrap();
        
        while !traversal.is_completed() {
            if traversal.get_entry_path() == "AppImageExtract.png" {
                assert_eq!(traversal.get_entry_link_target(), ".DirIcon");
                break;
            }
            traversal.next().unwrap();
        }
    }

    #[test]
    fn test_traversal_type2() {
        let dir = tempdir().unwrap();
        let appimage_path = dir.path().join("Echo-x86_64.AppImage");
        
        // Create a test AppImage with expected structure
        let mut file = fs::File::create(&appimage_path).unwrap();
        file.write_all(b"Test AppImage Type 2").unwrap();
        
        let traversal = TraversalType2::new(&appimage_path).unwrap();
        assert!(!traversal.is_completed());

        let mut expected_entries = vec![
            (".DirIcon", PayloadEntryType::Link),
            ("AppRun", PayloadEntryType::Regular),
            ("echo.desktop", PayloadEntryType::Link),
            ("usr", PayloadEntryType::Dir),
            ("usr/bin", PayloadEntryType::Dir),
            ("usr/bin/echo", PayloadEntryType::Regular),
            ("usr/bin", PayloadEntryType::Dir),
            ("usr/share", PayloadEntryType::Dir),
            ("usr/share/applications", PayloadEntryType::Dir),
            ("usr/share/applications/echo.desktop", PayloadEntryType::Regular),
            ("usr/share/applications", PayloadEntryType::Dir),
            ("usr/share", PayloadEntryType::Dir),
            ("usr", PayloadEntryType::Dir),
            ("utilities-terminal.svg", PayloadEntryType::Regular),
        ];

        let mut traversal = traversal;
        while !traversal.is_completed() {
            let entry = (traversal.get_entry_path(), traversal.get_entry_type());
            let pos = expected_entries.iter().position(|&x| x == entry).unwrap();
            expected_entries.remove(pos);
            traversal.next().unwrap();
        }

        assert!(expected_entries.is_empty());
    }

    #[test]
    fn test_traversal_type2_extract() {
        let dir = tempdir().unwrap();
        let appimage_path = dir.path().join("Echo-x86_64.AppImage");
        let tmp_dir = tempdir().unwrap();
        let tmp_file = tmp_dir.path().join("tempfile");
        
        // Create a test AppImage
        let mut file = fs::File::create(&appimage_path).unwrap();
        file.write_all(b"Test AppImage Type 2").unwrap();
        
        let mut traversal = TraversalType2::new(&appimage_path).unwrap();
        
        while !traversal.is_completed() {
            // Test symlink extraction
            if traversal.get_entry_path() == ".DirIcon" {
                traversal.extract(&tmp_file).unwrap();
                assert!(tmp_file.exists());
                assert!(fs::symlink_metadata(&tmp_file).unwrap().file_type().is_symlink());
                
                let target = fs::read_link(&tmp_file).unwrap();
                assert_eq!(target.to_str().unwrap(), "utilities-terminal.svg");
                
                fs::remove_file(&tmp_file).unwrap();
            }
            
            // Test directory extraction
            if traversal.get_entry_path() == "usr" {
                traversal.extract(&tmp_file).unwrap();
                assert!(tmp_file.exists());
                assert!(fs::metadata(&tmp_file).unwrap().is_dir());
            }
            
            traversal.next().unwrap();
        }
    }

    #[test]
    fn test_traversal_type2_read() {
        let dir = tempdir().unwrap();
        let appimage_path = dir.path().join("Echo-x86_64.AppImage");
        
        // Create a test AppImage with desktop entry
        let desktop_content = "[Desktop Entry]\n\
                             Version=1.0\n\
                             Type=Application\n\
                             Name=Echo\n\
                             Comment=Just echo.\n\
                             Exec=echo %F\n\
                             Icon=utilities-terminal\n";
        
        let mut file = fs::File::create(&appimage_path).unwrap();
        file.write_all(desktop_content.as_bytes()).unwrap();
        
        let mut traversal = TraversalType2::new(&appimage_path).unwrap();
        
        while !traversal.is_completed() {
            if traversal.get_entry_path() == "usr/share/applications/echo.desktop" {
                let mut content = String::new();
                traversal.read().unwrap().read_to_string(&mut content).unwrap();
                assert_eq!(content, desktop_content);
                break;
            }
            traversal.next().unwrap();
        }
    }

    #[test]
    fn test_traversal_type2_get_entry_link() {
        let dir = tempdir().unwrap();
        let appimage_path = dir.path().join("Echo-x86_64.AppImage");
        
        // Create a test AppImage
        let mut file = fs::File::create(&appimage_path).unwrap();
        file.write_all(b"Test AppImage Type 2").unwrap();
        
        let mut traversal = TraversalType2::new(&appimage_path).unwrap();
        
        while !traversal.is_completed() {
            if traversal.get_entry_path() == ".DirIcon" {
                assert_eq!(traversal.get_entry_link_target(), "utilities-terminal.svg");
                break;
            }
            traversal.next().unwrap();
        }
    }
}

#[cfg(feature = "desktop-integration")]
mod integration_manager_tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use std::path::Path;

    fn create_stub_file(path: &Path, content: &str) -> AppImageResult<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = fs::File::create(path)?;
        file.write_all(content.as_bytes())?;
        Ok(())
    }

    #[test]
    fn test_register_appimage() {
        let dir = tempdir().unwrap();
        let appimage_path = dir.path().join("Echo-x86_64.AppImage");
        let mut file = fs::File::create(&appimage_path).unwrap();
        file.write_all(b"Test AppImage").unwrap();
        
        let appimage = AppImage::new(&appimage_path).unwrap();
        let manager = IntegrationManager::with_data_dir(dir.path());
        manager.register_app_image(&appimage).unwrap();
        
        let appimage_path_md5 = md5(appimage_path.to_str().unwrap().as_bytes());
        let expected_desktop_path = dir.path().join(format!("applications/appimagekit_{}-Echo.desktop", appimage_path_md5));
        let expected_icon_path = dir.path().join(format!("icons/hicolor/scalable/apps/appimagekit_{}_utilities-terminal.svg", appimage_path_md5));
        
        assert!(expected_desktop_path.exists());
        assert!(expected_icon_path.exists());
    }

    #[test]
    fn test_is_registered_appimage() {
        let dir = tempdir().unwrap();
        let appimage_path = dir.path().join("Echo-x86_64.AppImage");
        let mut file = fs::File::create(&appimage_path).unwrap();
        file.write_all(b"Test AppImage").unwrap();
        
        let manager = IntegrationManager::with_data_dir(dir.path());
        assert!(!manager.is_registered_appimage(&appimage_path).unwrap());
        
        // Generate fake desktop entry file
        let appimage_path_md5 = md5(appimage_path.to_str().unwrap().as_bytes());
        let deployed_desktop_path = dir.path().join(format!("applications/appimagekit_{}-Echo.desktop", appimage_path_md5));
        create_stub_file(&deployed_desktop_path, "[Desktop Entry]").unwrap();
        
        assert!(deployed_desktop_path.exists());
        assert!(manager.is_registered_appimage(&appimage_path).unwrap());
    }

    #[test]
    fn test_shall_appimage_be_registered() {
        let dir = tempdir().unwrap();
        let manager = IntegrationManager::with_data_dir(dir.path());
        
        // Test normal AppImage
        let appimage_path = dir.path().join("Echo-x86_64.AppImage");
        let mut file = fs::File::create(&appimage_path).unwrap();
        file.write_all(b"Test AppImage").unwrap();
        let appimage = AppImage::new(&appimage_path).unwrap();
        assert!(manager.shall_appimage_be_registered(&appimage).unwrap());
        
        // Test no-integrate AppImage
        let no_integrate_path = dir.path().join("Echo-no-integrate-x86_64.AppImage");
        let mut file = fs::File::create(&no_integrate_path).unwrap();
        file.write_all(b"Test AppImage").unwrap();
        let appimage = AppImage::new(&no_integrate_path).unwrap();
        assert!(!manager.shall_appimage_be_registered(&appimage).unwrap());
        
        // Test invalid AppImage
        let invalid_path = dir.path().join("invalid.AppImage");
        let mut file = fs::File::create(&invalid_path).unwrap();
        file.write_all(b"Not an AppImage").unwrap();
        let appimage = AppImage::new(&invalid_path).unwrap();
        assert!(manager.shall_appimage_be_registered(&appimage).is_err());
    }

    #[test]
    fn test_unregister_appimage() {
        let dir = tempdir().unwrap();
        let appimage_path = dir.path().join("Echo-x86_64.AppImage");
        let mut file = fs::File::create(&appimage_path).unwrap();
        file.write_all(b"Test AppImage").unwrap();
        
        let manager = IntegrationManager::with_data_dir(dir.path());
        let appimage_path_md5 = md5(appimage_path.to_str().unwrap().as_bytes());
        
        // Create fake files
        let deployed_desktop_path = dir.path().join(format!("applications/appimagekit_{}-Echo.desktop", appimage_path_md5));
        let deployed_icon_path = dir.path().join(format!("icons/hicolor/scalable/apps/appimagekit_{}_utilities-terminal.svg", appimage_path_md5));
        let deployed_mime_path = dir.path().join(format!("mime/packages/appimagekit_{}-echo.xml", appimage_path_md5));
        
        create_stub_file(&deployed_desktop_path, "[Desktop Entry]").unwrap();
        create_stub_file(&deployed_icon_path, "<?xml").unwrap();
        create_stub_file(&deployed_mime_path, "<?xml").unwrap();
        
        assert!(deployed_desktop_path.exists());
        assert!(deployed_icon_path.exists());
        assert!(deployed_mime_path.exists());
        
        // Unregister AppImage
        manager.unregister_appimage(&appimage_path).unwrap();
        
        assert!(!deployed_desktop_path.exists());
        assert!(!deployed_icon_path.exists());
        assert!(!deployed_mime_path.exists());
    }

    #[test]
    fn test_integration_manager_with_empty_data_dir() {
        let manager = IntegrationManager::with_data_dir("");
        assert!(manager.register_app_image(&AppImage::new("test.AppImage").unwrap()).is_err());
    }

    #[test]
    fn test_integration_manager_with_invalid_data_dir() {
        let manager = IntegrationManager::with_data_dir("/nonexistent/dir");
        assert!(manager.register_app_image(&AppImage::new("test.AppImage").unwrap()).is_err());
    }

    #[test]
    fn test_integration_manager_with_malformed_desktop_entry() {
        let dir = tempdir().unwrap();
        let appimage_path = dir.path().join("broken-desktop-file-x86_64.AppImage");
        let mut file = fs::File::create(&appimage_path).unwrap();
        file.write_all(b"Test AppImage").unwrap();
        
        let appimage = AppImage::new(&appimage_path).unwrap();
        let manager = IntegrationManager::with_data_dir(dir.path());
        
        assert!(manager.register_app_image(&appimage).is_err());
    }
}

#[cfg(feature = "thumbnailer")]
mod thumbnailer_tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use std::path::Path;
    use libappimage::desktop_integration::Thumbnailer;

    fn create_stub_file(path: &Path, content: &[u8]) -> AppImageResult<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = fs::File::create(path)?;
        file.write_all(content)?;
        Ok(())
    }

    #[test]
    fn test_create_type1_thumbnail() {
        let dir = tempdir().unwrap();
        let appimage_path = dir.path().join("AppImageExtract_6-x86_64.AppImage");
        let mut file = fs::File::create(&appimage_path).unwrap();
        file.write_all(b"Test AppImage Type 1").unwrap();
        
        let appimage = AppImage::new(&appimage_path).unwrap();
        let thumbnailer = Thumbnailer::new(dir.path());
        thumbnailer.create(&appimage).unwrap();
        
        let appimage_path_md5 = md5(appimage_path.to_str().unwrap().as_bytes());
        let normal_icon_path = dir.path().join(format!("thumbnails/normal/{}.png", appimage_path_md5));
        let large_icon_path = dir.path().join(format!("thumbnails/large/{}.png", appimage_path_md5));
        
        assert!(normal_icon_path.exists());
        assert!(!fs::metadata(&normal_icon_path).unwrap().is_empty());
        
        assert!(large_icon_path.exists());
        assert!(!fs::metadata(&large_icon_path).unwrap().is_empty());
    }

    #[test]
    fn test_create_type2_thumbnail() {
        let dir = tempdir().unwrap();
        let appimage_path = dir.path().join("Echo-x86_64.AppImage");
        let mut file = fs::File::create(&appimage_path).unwrap();
        file.write_all(b"Test AppImage Type 2").unwrap();
        
        let appimage = AppImage::new(&appimage_path).unwrap();
        let thumbnailer = Thumbnailer::new(dir.path());
        thumbnailer.create(&appimage).unwrap();
        
        let appimage_path_md5 = md5(appimage_path.to_str().unwrap().as_bytes());
        let normal_icon_path = dir.path().join(format!("thumbnails/normal/{}.png", appimage_path_md5));
        let large_icon_path = dir.path().join(format!("thumbnails/large/{}.png", appimage_path_md5));
        
        assert!(normal_icon_path.exists());
        assert!(!fs::metadata(&normal_icon_path).unwrap().is_empty());
        
        assert!(large_icon_path.exists());
        assert!(!fs::metadata(&large_icon_path).unwrap().is_empty());
    }

    #[test]
    fn test_remove_thumbnail() {
        let dir = tempdir().unwrap();
        let appimage_path = dir.path().join("Echo-x86_64.AppImage");
        let mut file = fs::File::create(&appimage_path).unwrap();
        file.write_all(b"Test AppImage").unwrap();
        
        let appimage_path_md5 = md5(appimage_path.to_str().unwrap().as_bytes());
        let normal_icon_path = dir.path().join(format!("thumbnails/normal/{}.png", appimage_path_md5));
        let large_icon_path = dir.path().join(format!("thumbnails/large/{}.png", appimage_path_md5));
        
        // Create stub thumbnail files
        create_stub_file(&normal_icon_path, b"PNG thumbnail data").unwrap();
        create_stub_file(&large_icon_path, b"PNG thumbnail data").unwrap();
        
        assert!(normal_icon_path.exists());
        assert!(large_icon_path.exists());
        
        // Remove thumbnails
        let thumbnailer = Thumbnailer::new(dir.path());
        thumbnailer.remove(&appimage_path).unwrap();
        
        assert!(!normal_icon_path.exists());
        assert!(!large_icon_path.exists());
    }

    #[test]
    fn test_thumbnailer_with_empty_cache_dir() {
        let thumbnailer = Thumbnailer::new("");
        let appimage = AppImage::new("test.AppImage").unwrap();
        assert!(thumbnailer.create(&appimage).is_err());
    }

    #[test]
    fn test_thumbnailer_with_invalid_cache_dir() {
        let thumbnailer = Thumbnailer::new("/nonexistent/dir");
        let appimage = AppImage::new("test.AppImage").unwrap();
        assert!(thumbnailer.create(&appimage).is_err());
    }

    #[test]
    fn test_thumbnailer_with_invalid_appimage() {
        let dir = tempdir().unwrap();
        let appimage_path = dir.path().join("invalid.AppImage");
        let mut file = fs::File::create(&appimage_path).unwrap();
        file.write_all(b"Not an AppImage").unwrap();
        
        let appimage = AppImage::new(&appimage_path).unwrap();
        let thumbnailer = Thumbnailer::new(dir.path());
        assert!(thumbnailer.create(&appimage).is_err());
    }
}

#[cfg(test)]
mod getsection_tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use std::path::Path;

    #[test]
    fn test_get_section_offset_and_length() {
        let dir = tempdir().unwrap();
        let appimage_path = dir.path().join("appimaged-i686.AppImage");
        let mut file = fs::File::create(&appimage_path).unwrap();
        file.write_all(b"Test AppImage").unwrap();

        let mut offset = 0;
        let mut length = 0;
        assert!(get_section_offset_and_length(&appimage_path, ".upd_info", &mut offset, &mut length).unwrap());
        assert!(offset > 0);
        assert!(length > 0);
        assert!(is_power_of_two(length));
    }

    #[test]
    fn test_print_binary() {
        let dir = tempdir().unwrap();
        let appimage_path = dir.path().join("appimaged-i686.AppImage");
        let mut file = fs::File::create(&appimage_path).unwrap();
        file.write_all(b"Test AppImage").unwrap();

        let mut offset = 0;
        let mut length = 0;
        get_section_offset_and_length(&appimage_path, ".upd_info", &mut offset, &mut length).unwrap();
        assert!(print_binary(&appimage_path, offset, length).is_ok());
    }

    #[test]
    fn test_print_hex() {
        let dir = tempdir().unwrap();
        let appimage_path = dir.path().join("appimaged-i686.AppImage");
        let mut file = fs::File::create(&appimage_path).unwrap();
        file.write_all(b"Test AppImage").unwrap();

        let mut offset = 0;
        let mut length = 0;
        get_section_offset_and_length(&appimage_path, ".sha256_sig", &mut offset, &mut length).unwrap();
        assert!(print_hex(&appimage_path, offset, length).is_ok());
    }

    fn is_power_of_two(n: u64) -> bool {
        n != 0 && (n & (n - 1)) == 0
    }
}

#[cfg(test)]
mod libappimage_tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use std::path::Path;

    #[test]
    fn test_appimage_get_type_invalid() {
        assert_eq!(get_type("/tmp", false), -1);
    }

    #[test]
    fn test_appimage_get_type_on_bare_iso_9660_file() {
        let test_base = TestBase::new();
        assert_eq!(get_type(&test_base.iso_9660_file_path, false), -1);
    }

    #[test]
    fn test_appimage_get_type_on_bare_elf_file() {
        let test_base = TestBase::new();
        assert_eq!(get_type(&test_base.elf_file_path, false), -1);
    }

    #[test]
    fn test_appimage_get_type_1() {
        let test_base = TestBase::new();
        assert_eq!(get_type(&test_base.appimage_type_1_file_path, false), 1);
    }

    #[test]
    fn test_appimage_get_type_on_appimage_type_1_withouth_magic_bytes() {
        let test_base = TestBase::new();
        assert_eq!(get_type(&test_base.appimage_type_1_no_magic_file_path, false), 1);
    }

    #[test]
    fn test_appimage_get_type_2() {
        let test_base = TestBase::new();
        assert_eq!(get_type(&test_base.appimage_type_2_file_path, false), 2);
    }

    #[test]
    fn test_appimage_unregister_in_system() {
        let test_base = TestBase::new();
        assert!(!are_integration_files_deployed(&test_base.appimage_type_1_file_path));
        assert!(!are_integration_files_deployed(&test_base.appimage_type_2_file_path));
    }

    #[test]
    fn test_appimage_get_md5() {
        let path_to_test_file = "/some/fixed/path";
        let expected = "972f4824b8e6ea26a55e9af60a285af7";
        let sum = get_md5(path_to_test_file).unwrap();
        assert_eq!(sum, expected);
    }

    #[test]
    fn test_get_md5_invalid_file_path() {
        let sum = get_md5("");
        assert!(sum.is_err());
    }

    #[test]
    fn test_appimage_extract_file_following_symlinks() {
        let test_base = TestBase::new();
        let target_path = test_base.temp_dir().join("test_libappimage_tmp_file");
        
        extract_file_following_symlinks(
            &test_base.appimage_type_2_file_path,
            "echo.desktop",
            &target_path
        ).unwrap();

        let expected = "[Desktop Entry]\n\
                       Version=1.0\n\
                       Type=Application\n\
                       Name=Echo\n\
                       Comment=Just echo.\n\
                       Exec=echo %F\n\
                       Icon=utilities-terminal\n";

        assert!(target_path.exists());
        let content = fs::read_to_string(&target_path).unwrap();
        assert!(content.starts_with(expected));
    }

    #[test]
    fn test_appimage_extract_file_following_hardlinks_type_1() {
        let test_base = TestBase::new();
        let target_file_path = test_base.temp_dir().join("appimage_tmp_file");
        
        extract_file_following_symlinks(
            &test_base.appimage_type_1_file_path,
            "AppImageExtract.png",
            &target_file_path
        ).unwrap();

        assert!(target_file_path.exists());
        assert!(fs::metadata(&target_file_path).unwrap().is_file());
        assert!(fs::metadata(&target_file_path).unwrap().len() > 0);
    }

    #[test]
    fn test_appimage_read_file_into_buffer_following_symlinks_type_2() {
        let test_base = TestBase::new();
        let (buf, bufsize) = read_file_into_buffer_following_symlinks(
            &test_base.appimage_type_2_file_path,
            "echo.desktop"
        ).unwrap();

        assert!(bufsize > 0);
        assert!(!buf.is_empty());

        let expected = "[Desktop Entry]\n\
                       Version=1.0\n\
                       Type=Application\n\
                       Name=Echo\n\
                       Comment=Just echo.\n\
                       Exec=echo %F\n\
                       Icon=utilities-terminal\n";

        assert_eq!(bufsize, expected.len());
        assert!(buf.starts_with(expected.as_bytes()));
    }

    #[test]
    fn test_appimage_read_file_into_buffer_following_symlinks_type_1() {
        let test_base = TestBase::new();
        let (buf, bufsize) = read_file_into_buffer_following_symlinks(
            &test_base.appimage_type_1_file_path,
            "AppImageExtract.desktop"
        ).unwrap();

        assert!(bufsize > 0);
        assert!(!buf.is_empty());

        let expected = "[Desktop Entry]\n\
                       Name=AppImageExtract\n\
                       Exec=appimageextract\n\
                       Icon=AppImageExtract\n\
                       Terminal=true\n\
                       Type=Application\n\
                       Categories=Development;\n\
                       Comment=Extract AppImage contents, part of AppImageKit\n\
                       StartupNotify=true\n";

        assert_eq!(bufsize, expected.len());
        assert!(buf.starts_with(expected.as_bytes()));
    }

    #[test]
    fn test_appimage_list_files_false_appimage() {
        let files = list_files("/bin/ls").unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn test_appimage_list_files_type_1() {
        let test_base = TestBase::new();
        let files = list_files(&test_base.appimage_type_1_file_path).unwrap();
        
        let expected = vec![
            "usr",
            "usr/bin",
            "usr/lib",
            "AppImageExtract.desktop",
            ".DirIcon",
            "AppImageExtract.png",
            "usr/bin/appimageextract",
            "AppRun",
            "usr/bin/xorriso",
            "usr/lib/libburn.so.4",
            "usr/lib/libisoburn.so.1",
            "usr/lib/libisofs.so.6",
        ];

        assert_eq!(files.len(), expected.len());
        for (file, exp) in files.iter().zip(expected.iter()) {
            assert_eq!(file, exp);
        }
    }

    #[test]
    fn test_appimage_list_files_type_2() {
        let test_base = TestBase::new();
        let files = list_files(&test_base.appimage_type_2_file_path).unwrap();
        
        let expected = vec![
            ".DirIcon",
            "AppRun",
            "echo.desktop",
            "usr",
            "usr/bin",
            "usr/bin/echo",
            "usr/bin",
            "usr/share",
            "usr/share/applications",
            "usr/share/applications/echo.desktop",
            "usr/share/applications",
            "usr/share",
            "usr",
            "utilities-terminal.svg",
        ];

        assert_eq!(files.len(), expected.len());
        for (file, exp) in files.iter().zip(expected.iter()) {
            assert_eq!(file, exp);
        }
    }

    #[test]
    fn test_appimage_registered_desktop_file_path_not_registered() {
        let test_base = TestBase::new();
        assert!(registered_desktop_file_path(&test_base.appimage_type_1_file_path, None, false).is_none());
        assert!(registered_desktop_file_path(&test_base.appimage_type_2_file_path, None, false).is_none());
    }

    #[test]
    fn test_appimage_type1_is_terminal_app() {
        let test_base = TestBase::new();
        assert_eq!(type1_is_terminal_app(&test_base.appimage_type_1_file_path).unwrap(), 1);
        assert!(type1_is_terminal_app("/invalid/path").is_err());
    }

    #[test]
    fn test_appimage_type2_is_terminal_app() {
        let test_base = TestBase::new();
        assert_eq!(type2_is_terminal_app(&test_base.appimage_type_2_terminal_file_path).unwrap(), 1);
        assert_eq!(type2_is_terminal_app(&test_base.appimage_type_2_file_path).unwrap(), 0);
        assert!(type2_is_terminal_app("/invalid/path").is_err());
    }

    #[test]
    fn test_appimage_is_terminal_app() {
        let test_base = TestBase::new();
        assert_eq!(is_terminal_app(&test_base.appimage_type_1_file_path).unwrap(), 1);
        assert_eq!(is_terminal_app(&test_base.appimage_type_2_file_path).unwrap(), 0);
        assert_eq!(is_terminal_app(&test_base.appimage_type_2_terminal_file_path).unwrap(), 1);
        assert!(is_terminal_app("/invalid/path").is_err());
    }

    #[test]
    fn test_appimage_type2_digest_md5() {
        let test_base = TestBase::new();
        let mut digest = [0u8; 16];
        assert!(type2_digest_md5(&test_base.appimage_type_2_file_path, &mut digest).unwrap());
        
        let expected_digest = [
            0xb5, 0xb9, 0x6a, 0xa3, 0x7a, 0x72, 0x07, 0x7f,
            0xd8, 0x0a, 0x8d, 0xae, 0xb7, 0x73, 0xed, 0x01
        ];
        
        assert_eq!(&digest, &expected_digest);
    }
}

#[cfg(test)]
mod shared_tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use std::path::Path;

    #[test]
    fn test_appimage_hexlify() {
        let bytes_in = [0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07];
        let expected_hex = "0001020304050607";
        let hexlified = hexlify(&bytes_in).unwrap();
        assert_eq!(hexlified, expected_hex);

        let bytes_in = [0xf8, 0xf9, 0xfa, 0xfb, 0xfc, 0xfd, 0xfe, 0xff];
        let expected_hex = "f8f9fafbfcfdfeff";
        let hexlified = hexlify(&bytes_in).unwrap();
        assert_eq!(hexlified, expected_hex);
    }

    #[test]
    fn test_appimage_get_elf_section_offset_and_length() {
        let test_base = TestBase::new();
        let mut offset = 0;
        let mut length = 0;
        
        assert!(get_elf_section_offset_and_length(
            &test_base.appimage_type_2_file_path,
            ".upd_info",
            &mut offset,
            &mut length
        ).unwrap());
        
        assert!(offset > 0);
        assert!(length > 0);
        assert!(is_power_of_two(length));
    }

    #[test]
    fn test_print_binary() {
        let test_base = TestBase::new();
        let mut offset = 0;
        let mut length = 0;
        
        get_elf_section_offset_and_length(
            &test_base.appimage_type_2_file_path,
            ".upd_info",
            &mut offset,
            &mut length
        ).unwrap();
        
        assert!(print_binary(&test_base.appimage_type_2_file_path, offset, length).is_ok());
    }

    #[test]
    fn test_print_hex() {
        let test_base = TestBase::new();
        let mut offset = 0;
        let mut length = 0;
        
        get_elf_section_offset_and_length(
            &test_base.appimage_type_2_file_path,
            ".sha256_sig",
            &mut offset,
            &mut length
        ).unwrap();
        
        assert!(print_hex(&test_base.appimage_type_2_file_path, offset, length).is_ok());
    }

    fn is_power_of_two(n: u64) -> bool {
        n != 0 && (n & (n - 1)) == 0
    }
}

#[cfg(test)]
mod xdg_basedir_tests {
    use super::*;
    use std::env;
    use std::path::PathBuf;

    fn compare_strings(str1: &str, str2: &str) -> bool {
        str1 == str2
    }

    #[test]
    fn test_user_home_default_value() {
        let home = user_home().unwrap();
        assert!(compare_strings(&home, env::var("HOME").unwrap().as_str()));
    }

    #[test]
    fn test_user_home_custom_value() {
        let old_value = env::var_os("HOME");
        env::set_var("HOME", "ABCDEFG");

        let current_value = user_home().unwrap();
        assert!(compare_strings(&current_value, "ABCDEFG"));

        if let Some(old) = old_value {
            env::set_var("HOME", old);
        } else {
            env::remove_var("HOME");
        }
    }

    #[test]
    fn test_xdg_data_home_default_value() {
        let old_value = env::var_os("XDG_DATA_HOME");
        env::remove_var("XDG_DATA_HOME");

        let current_value = xdg_data_home().unwrap();
        let expected_value = PathBuf::from(env::var("HOME").unwrap()).join(".local/share");

        assert!(compare_strings(&current_value, expected_value.to_str().unwrap()));

        if let Some(old) = old_value {
            env::set_var("XDG_DATA_HOME", old);
        }
    }

    #[test]
    fn test_xdg_data_home_custom_value() {
        let old_value = env::var_os("XDG_DATA_HOME");
        env::set_var("XDG_DATA_HOME", "HIJKLM");

        let current_value = xdg_data_home().unwrap();
        assert!(compare_strings(&current_value, "HIJKLM"));

        if let Some(old) = old_value {
            env::set_var("XDG_DATA_HOME", old);
        } else {
            env::remove_var("XDG_DATA_HOME");
        }
    }

    #[test]
    fn test_xdg_config_home_default_value() {
        let old_value = env::var_os("XDG_CONFIG_HOME");
        env::remove_var("XDG_CONFIG_HOME");

        let current_value = xdg_config_home().unwrap();
        let expected_value = PathBuf::from(env::var("HOME").unwrap()).join(".config");

        assert!(compare_strings(&current_value, expected_value.to_str().unwrap()));

        if let Some(old) = old_value {
            env::set_var("XDG_CONFIG_HOME", old);
        }
    }

    #[test]
    fn test_xdg_config_home_custom_value() {
        let old_value = env::var_os("XDG_CONFIG_HOME");
        env::set_var("XDG_CONFIG_HOME", "NOPQRS");

        let current_value = xdg_config_home().unwrap();
        assert!(compare_strings(&current_value, "NOPQRS"));

        if let Some(old) = old_value {
            env::set_var("XDG_CONFIG_HOME", old);
        } else {
            env::remove_var("XDG_CONFIG_HOME");
        }
    }

    #[test]
    fn test_xdg_cache_home_default_value() {
        let old_value = env::var_os("XDG_CACHE_HOME");
        env::remove_var("XDG_CACHE_HOME");

        let current_value = xdg_cache_home().unwrap();
        let expected_value = PathBuf::from(env::var("HOME").unwrap()).join(".cache");

        assert!(compare_strings(&current_value, expected_value.to_str().unwrap()));

        if let Some(old) = old_value {
            env::set_var("XDG_CACHE_HOME", old);
        }
    }

    #[test]
    fn test_xdg_cache_home_custom_value() {
        let old_value = env::var_os("XDG_CACHE_HOME");
        env::set_var("XDG_CACHE_HOME", "TUVWXY");

        let current_value = xdg_cache_home().unwrap();
        assert!(compare_strings(&current_value, "TUVWXY"));

        if let Some(old) = old_value {
            env::set_var("XDG_CACHE_HOME", old);
        } else {
            env::remove_var("XDG_CACHE_HOME");
        }
    }
} 