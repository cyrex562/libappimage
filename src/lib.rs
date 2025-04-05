//! A static library for AppImage handling
//!
//! This library provides functionality for working with AppImage files.

mod appimage;
pub mod appimage_handler;
mod config;
mod desktop_integration;
mod elf;
mod error;
pub mod ffi;
mod format;
pub mod handlers;
pub mod legacy;
mod payload;
mod payload_iterator;
mod payload_types;
mod progress;
mod squashfs_tools;
mod streambuf;
mod traversal;
pub mod utils;

pub use appimage::AppImage;
pub use appimage_handler::{AppImageHandler, TraverseCallback};
pub use config::{features, version};
pub use error::{
    AppImageError, AppImageResult, IntoAppImageError, IntoAppImageIoError, IntoAppImageStringError,
};
pub use error::{DesktopEntryEditError, DesktopIntegrationError};
pub use format::{AppImageFormat, FormatError};
pub use handlers::type1::Type1Handler;
pub use handlers::type2::Type2Handler;
pub use handlers::{create_handler, Handler};
pub use payload::PayloadIStream;
pub use payload_iterator::PayloadIterator;
pub use payload_types::PayloadEntryType;
pub use streambuf::{StreambufType1, StreambufType2};
pub use traversal::{Traversal, TraversalType1, TraversalType2};
pub use utils::*;

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
