use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::path::PathBuf;
use std::sync::Once;

use crate::ffi::*;
use crate::utils::logger::LogLevel;

static INIT: Once = Once::new();

fn setup() {
    INIT.call_once(|| {
        // Initialize test environment
    });
}

fn create_test_appimage() -> PathBuf {
    let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data");
    test_dir.join("test.appimage")
}

#[test]
fn test_logging() {
    setup();

    // Test setting log level
    assert_eq!(appimage_set_log_level(LogLevel::Debug as c_int), 0);
    assert_eq!(appimage_set_log_level(LogLevel::Info as c_int), 0);
    assert_eq!(appimage_set_log_level(LogLevel::Warning as c_int), 0);
    assert_eq!(appimage_set_log_level(LogLevel::Error as c_int), 0);
    assert_ne!(appimage_set_log_level(100), 0); // Invalid level

    // Test log callback
    static mut LOG_CALLED: bool = false;
    extern "C" fn test_callback(_level: c_int, message: *const c_char) {
        unsafe {
            LOG_CALLED = true;
            let message = CStr::from_ptr(message).to_str().unwrap();
            assert!(!message.is_empty());
        }
    }

    assert_eq!(appimage_set_log_callback(test_callback), 0);
    // Trigger a log message
    let _ = appimage_new(std::ptr::null());
    unsafe {
        assert!(LOG_CALLED);
    }
}

#[test]
fn test_appimage_creation() {
    setup();

    // Test with null path
    let appimage = appimage_new(std::ptr::null());
    assert!(appimage.is_null());

    // Test with invalid path
    let invalid_path = CString::new("/nonexistent/path").unwrap();
    let appimage = appimage_new(invalid_path.as_ptr());
    assert!(appimage.is_null());

    // Test with valid path
    let test_path = create_test_appimage();
    let path = CString::new(test_path.to_str().unwrap()).unwrap();
    let appimage = appimage_new(path.as_ptr());
    assert!(!appimage.is_null());

    // Clean up
    appimage_free(appimage);
}

#[test]
fn test_appimage_format() {
    setup();

    // Create test AppImage
    let test_path = create_test_appimage();
    let path = CString::new(test_path.to_str().unwrap()).unwrap();
    let appimage = appimage_new(path.as_ptr());
    assert!(!appimage.is_null());

    // Test format detection
    let format = appimage_get_format(appimage);
    assert!(format == 1 || format == 2); // Type 1 or Type 2

    // Test with null handle
    let format = appimage_get_format(std::ptr::null());
    assert_ne!(format, 0);

    // Clean up
    appimage_free(appimage);
}

#[test]
fn test_appimage_extraction() {
    setup();

    // Create test AppImage
    let test_path = create_test_appimage();
    let path = CString::new(test_path.to_str().unwrap()).unwrap();
    let appimage = appimage_new(path.as_ptr());
    assert!(!appimage.is_null());

    // Create temporary directory for extraction
    let temp_dir = tempfile::tempdir().unwrap();
    let source = CString::new(".DirIcon").unwrap();
    let target = CString::new(temp_dir.path().join("icon").to_str().unwrap()).unwrap();

    // Test extraction
    let result = appimage_extract_file(appimage, source.as_ptr(), target.as_ptr());
    assert_eq!(result, 0);

    // Test with invalid source
    let invalid_source = CString::new("nonexistent").unwrap();
    let result = appimage_extract_file(appimage, invalid_source.as_ptr(), target.as_ptr());
    assert_ne!(result, 0);

    // Test with null handle
    let result = appimage_extract_file(std::ptr::null(), source.as_ptr(), target.as_ptr());
    assert_ne!(result, 0);

    // Clean up
    appimage_free(appimage);
}

#[test]
fn test_appimage_size() {
    setup();

    // Create test AppImage
    let test_path = create_test_appimage();
    let path = CString::new(test_path.to_str().unwrap()).unwrap();
    let appimage = appimage_new(path.as_ptr());
    assert!(!appimage.is_null());

    // Test size calculation
    let size = appimage_get_size(appimage);
    assert!(size > 0);

    // Test with null handle
    let size = appimage_get_size(std::ptr::null());
    assert_eq!(size, 0);

    // Clean up
    appimage_free(appimage);
}

#[test]
fn test_appimage_md5() {
    setup();

    // Create test AppImage
    let test_path = create_test_appimage();
    let path = CString::new(test_path.to_str().unwrap()).unwrap();
    let appimage = appimage_new(path.as_ptr());
    assert!(!appimage.is_null());

    // Test MD5 calculation
    let mut hash = [0u8; 33]; // MD5 hex string + null terminator
    let result = appimage_get_md5(appimage, hash.as_mut_ptr() as *mut c_char, hash.len() as c_int);
    assert_eq!(result, 0);

    let hash_str = unsafe { CStr::from_ptr(hash.as_ptr() as *const c_char) }.to_str().unwrap();
    assert_eq!(hash_str.len(), 32); // MD5 hex string length

    // Test with null handle
    let result = appimage_get_md5(std::ptr::null(), hash.as_mut_ptr() as *mut c_char, hash.len() as c_int);
    assert_ne!(result, 0);

    // Clean up
    appimage_free(appimage);
}

#[test]
fn test_appimage_integration() {
    setup();

    // Create test AppImage
    let test_path = create_test_appimage();
    let path = CString::new(test_path.to_str().unwrap()).unwrap();
    let appimage = appimage_new(path.as_ptr());
    assert!(!appimage.is_null());

    // Test integration
    let result = appimage_integrate(appimage);
    assert_eq!(result, 0);

    // Test is_integrated
    let result = appimage_is_integrated(appimage);
    assert_eq!(result, 1);

    // Test unintegration
    let result = appimage_unintegrate(appimage);
    assert_eq!(result, 0);

    // Test is_integrated after unintegration
    let result = appimage_is_integrated(appimage);
    assert_eq!(result, 0);

    // Test with null handle
    let result = appimage_integrate(std::ptr::null());
    assert_ne!(result, 0);

    // Clean up
    appimage_free(appimage);
}

#[test]
fn test_error_handling() {
    setup();

    // Trigger an error
    let _ = appimage_new(std::ptr::null());

    // Check error message
    let error = unsafe { CStr::from_ptr(appimage_get_last_error()) }.to_str().unwrap();
    assert!(!error.is_empty());
} 