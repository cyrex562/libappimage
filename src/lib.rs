//! A static library for AppImage handling
//! 
//! This library provides functionality for working with AppImage files.

mod payload;
mod streambuf;
mod traversal;
mod appimage;
mod elf;
mod format;
mod error;
mod payload_types;
mod payload_iterator;
mod desktop_integration;
mod config;
mod squashfs_tools;
pub mod utils;
pub mod appimage_handler;
pub mod ffi;
pub mod legacy;
pub mod handlers;

pub use payload::PayloadIStream;
pub use streambuf::{StreambufType1, StreambufType2};
pub use traversal::{Traversal, TraversalType1, TraversalType2};
pub use appimage::AppImage;
pub use format::{AppImageFormat, FormatError};
pub use error::{AppImageError, AppImageResult, IntoAppImageError, IntoAppImageIoError, IntoAppImageStringError};
pub use payload_types::PayloadEntryType;
pub use payload_iterator::PayloadIterator;
pub use desktop_integration::error::{DesktopIntegrationError, DesktopEntryEditError};
pub use utils::*;
pub use appimage_handler::{AppImageHandler, TraverseCallback};
pub use config::{version, features};
pub use handlers::{Handler, create_handler, Type1Handler, Type2Handler};

#[no_mangle]
pub extern "C" fn appimage_version() -> *const i8 {
    format!("{}\0", version::VERSION).as_ptr() as *const i8
}

#[no_mangle]
pub extern "C" fn appimage_init() -> i32 {
    0 // Success
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        unsafe {
            let version = std::ffi::CStr::from_ptr(appimage_version());
            assert_eq!(version.to_str().unwrap(), version::VERSION);
        }
    }
} 