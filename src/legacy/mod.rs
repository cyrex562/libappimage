use std::path::Path;
use std::os::raw::{c_char, c_int};
use crate::{
    AppImage, AppImageError, AppImageResult,
    utils::{elf_file::ElfFile, digest::type2_digest_md5},
    desktop_integration::IntegrationManager,
    ffi::{catch_all, appimage_get_type, appimage_is_terminal_app},
};

/// Calculate the size of an ELF file on disk based on the information in its header
#[deprecated(note = "Use ElfFile::get_size instead")]
pub fn get_elf_size(path: impl AsRef<Path>) -> AppImageResult<i64> {
    let elf = ElfFile::new(path)?;
    Ok(elf.get_size())
}

/// Check if a Type 1 AppImage is a terminal application
#[deprecated(note = "Use AppImage::is_terminal_app instead")]
pub fn type1_is_terminal_app(path: impl AsRef<Path>) -> AppImageResult<bool> {
    let app_image = crate::AppImage::new(path)?;
    Ok(app_image.is_terminal_app()?)
}

/// Check if a Type 2 AppImage is a terminal application
#[deprecated(note = "Use AppImage::is_terminal_app instead")]
pub fn type2_is_terminal_app(path: impl AsRef<Path>) -> AppImageResult<bool> {
    type1_is_terminal_app(path)
}

#[cfg(feature = "desktop-integration")]
mod desktop_integration {
    use super::*;
    use crate::desktop_integration::IntegrationManager;

    /// Check if a Type 1 AppImage should not be integrated
    #[deprecated(note = "Use AppImage::shall_not_be_integrated instead")]
    pub fn type1_shall_not_be_integrated(path: impl AsRef<Path>) -> AppImageResult<bool> {
        let app_image = crate::AppImage::new(path)?;
        let manager = IntegrationManager::new();
        Ok(manager.shall_not_be_integrated(&app_image)?)
    }

    /// Check if a Type 2 AppImage should not be integrated
    #[deprecated(note = "Use AppImage::shall_not_be_integrated instead")]
    pub fn type2_shall_not_be_integrated(path: impl AsRef<Path>) -> AppImageResult<bool> {
        type1_shall_not_be_integrated(path)
    }

    /// Register a Type 1 AppImage in the system
    #[deprecated(note = "Use AppImage::register_in_system instead")]
    pub fn type1_register_in_system(path: impl AsRef<Path>, verbose: bool) -> AppImageResult<()> {
        let app_image = crate::AppImage::new(path)?;
        let manager = IntegrationManager::new();
        
        manager.register_app_image(&app_image)?;
        
        #[cfg(feature = "thumbnailer")]
        {
            manager.generate_thumbnails(&app_image)?;
        }
        
        Ok(())
    }

    /// Register a Type 2 AppImage in the system
    #[deprecated(note = "Use AppImage::register_in_system instead")]
    pub fn type2_register_in_system(path: impl AsRef<Path>, verbose: bool) -> AppImageResult<()> {
        type1_register_in_system(path, verbose)
    }
}

/// FFI module for legacy C bindings
pub mod ffi {
    use super::*;
    use crate::desktop_integration::IntegrationManager;

    /// Get the ELF size of an AppImage
    #[no_mangle]
    pub extern "C" fn appimage_get_elf_size(fname: *const c_char) -> i64 {
        if fname.is_null() {
            return 0;
        }

        catch_all(|| {
            let path = unsafe { std::ffi::CStr::from_ptr(fname).to_str()? };
            let app_image = AppImage::new(path)?;
            Ok(app_image.get_payload_offset())
        })
    }

    /// Check if a Type 1 AppImage is a terminal application
    #[no_mangle]
    pub extern "C" fn appimage_type1_is_terminal_app(path: *const c_char) -> c_int {
        appimage_is_terminal_app(path)
    }

    /// Check if a Type 2 AppImage is a terminal application
    #[no_mangle]
    pub extern "C" fn appimage_type2_is_terminal_app(path: *const c_char) -> c_int {
        appimage_is_terminal_app(path)
    }

    #[cfg(feature = "desktop-integration")]
    pub mod desktop_integration {
        use super::*;

        /// Register a Type 1 AppImage in the system
        #[no_mangle]
        pub extern "C" fn appimage_type1_register_in_system(path: *const c_char, verbose: bool) -> bool {
            if path.is_null() {
                return false;
            }

            catch_all(|| {
                let path = unsafe { std::ffi::CStr::from_ptr(path).to_str()? };
                let app_image = AppImage::new(path)?;
                let manager = IntegrationManager::new();
                
                manager.register_app_image(&app_image)?;
                
                #[cfg(feature = "thumbnailer")]
                {
                    manager.generate_thumbnails(&app_image)?;
                }
                
                Ok(true)
            })
        }

        /// Register a Type 2 AppImage in the system
        #[no_mangle]
        pub extern "C" fn appimage_type2_register_in_system(path: *const c_char, verbose: bool) -> bool {
            appimage_type1_register_in_system(path, verbose)
        }

        /// Check if a Type 1 AppImage should not be integrated
        #[no_mangle]
        pub extern "C" fn appimage_type1_shall_not_be_integrated(path: *const c_char) -> c_int {
            if path.is_null() {
                return -1;
            }

            catch_all(|| {
                let path = unsafe { std::ffi::CStr::from_ptr(path).to_str()? };
                let app_image = AppImage::new(path)?;
                let manager = IntegrationManager::new();
                
                if manager.shall_not_be_integrated(&app_image)? {
                    Ok(1)
                } else {
                    Ok(0)
                }
            })
        }

        /// Check if a Type 2 AppImage should not be integrated
        #[no_mangle]
        pub extern "C" fn appimage_type2_shall_not_be_integrated(path: *const c_char) -> c_int {
            appimage_type1_shall_not_be_integrated(path)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

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

    #[cfg(feature = "desktop-integration")]
    #[test]
    fn test_shall_not_be_integrated() {
        let dir = tempdir().unwrap();
        let appimage_path = dir.path().join("test.AppImage");
        
        // Create a test AppImage
        let mut file = fs::File::create(&appimage_path).unwrap();
        file.write_all(b"Hello, World!").unwrap();
        
        // Check if it should not be integrated
        let shall_not = type1_shall_not_be_integrated(&appimage_path).unwrap();
        assert!(!shall_not);
    }

    #[cfg(feature = "desktop-integration")]
    #[test]
    fn test_register_in_system() {
        let dir = tempdir().unwrap();
        let appimage_path = dir.path().join("test.AppImage");
        
        // Create a test AppImage
        let mut file = fs::File::create(&appimage_path).unwrap();
        file.write_all(b"Hello, World!").unwrap();
        
        // Register the AppImage
        type1_register_in_system(&appimage_path, false).unwrap();
    }
} 