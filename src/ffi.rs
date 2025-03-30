use std::ffi::{CString, c_void};
use std::os::raw::{c_char, c_int, c_ulong};
use std::path::Path;
use std::ptr;

use crate::{
    AppImage, AppImageFormat, AppImageError,
    utils::{hashlib, path_utils},
    desktop_integration::IntegrationManager,
    error::{AppImageResult, IntoAppImageStringError},
    utils::logger::{Logger, LogLevel},
    handlers::{Handler, create_handler},
    desktop_integration::{DesktopIntegrationError},
};

/// FFI error codes
#[repr(C)]
#[derive(Copy, Clone)]
pub enum ErrorCode {
    /// Operation succeeded
    Success = 0,
    /// IO error occurred
    IoError = 1,
    /// Invalid format
    FormatError = 2,
    /// String conversion error
    StringConversionError = 3,
    /// Nul error
    NulError = 4,
    /// UTF-8 error
    Utf8Error = 5,
    /// Desktop entry edit error
    DesktopEntryEditError = 6,
    /// Desktop integration error
    DesktopIntegrationError = 7,
    /// SquashFS error
    SquashFsError = 8,
    /// Handler is not open
    NotOpen = 9,
    /// File not found
    FileNotFound = 10,
    /// Not a file
    NotAFile = 11,
}

impl From<AppImageError> for ErrorCode {
    fn from(error: AppImageError) -> Self {
        match error {
            AppImageError::Io(_) => ErrorCode::IoError,
            AppImageError::InvalidFormat(_) => ErrorCode::FormatError,
            AppImageError::Elf(_) => ErrorCode::IoError,
            AppImageError::FileSystem(_) => ErrorCode::IoError,
            AppImageError::Archive(_) => ErrorCode::IoError,
            AppImageError::SquashFs(_) => ErrorCode::SquashFsError,
            AppImageError::NotSupported(_) => ErrorCode::IoError,
            AppImageError::InvalidParameter(_) => ErrorCode::IoError,
            AppImageError::NotFound(_) => ErrorCode::FileNotFound,
            AppImageError::PermissionDenied(_) => ErrorCode::IoError,
            AppImageError::OperationFailed(_) => ErrorCode::IoError,
            AppImageError::StringConversion(_) |
            AppImageError::NulError(_) |
            AppImageError::FromBytesWithNul(_) => ErrorCode::StringConversionError,
            AppImageError::DesktopIntegration(_) => ErrorCode::DesktopIntegrationError,
            AppImageError::NotOpen => ErrorCode::NotOpen,
            AppImageError::FileNotFound(_) => ErrorCode::FileNotFound,
            AppImageError::NotAFile => ErrorCode::NotAFile,
            AppImageError::InvalidPath(_) => ErrorCode::IoError,
            AppImageError::InvalidData(_) => ErrorCode::IoError,
            AppImageError::InvalidHeader(_) => ErrorCode::IoError,
            AppImageError::InvalidFooter(_) => ErrorCode::IoError,
            AppImageError::InvalidMagic(_) => ErrorCode::IoError,
            AppImageError::AlreadyExists(_) => ErrorCode::IoError,
            AppImageError::InvalidState(_) => ErrorCode::IoError,
            AppImageError::InvalidOperation(_) => ErrorCode::IoError,
        }
    }
}

impl From<ErrorCode> for c_int {
    fn from(code: ErrorCode) -> Self {
        code as i32
    }
}

static mut LAST_ERROR: Option<String> = None;

/// Wrapper for C-style error handling
pub fn catch_all<T: Default>(f: impl FnOnce() -> Result<T, AppImageError>) -> T {
    match f() {
        Ok(value) => value,
        Err(e) => {
            eprintln!("Error: {}", e);
            unsafe {
                LAST_ERROR = Some(e.to_string());
            }
            T::default()
        }
    }
}

/// Check if a file is an AppImage. Returns the image type if it is, or -1 if it isn't
#[no_mangle]
pub extern "C" fn appimage_get_type(path: *const c_char, _verbose: bool) -> c_int {
    if path.is_null() {
        return -1;
    }

    catch_all(|| {
        let path = unsafe { std::ffi::CStr::from_ptr(path).to_str()? };
        let app_image = AppImage::new(path)?;
        Ok(app_image.get_format() as c_int)
    })
}

/// List all files in an AppImage
#[no_mangle]
pub extern "C" fn appimage_list_files(path: *const c_char) -> *mut *mut c_char {
    if path.is_null() {
        return std::ptr::null_mut();
    }

    catch_all(|| {
        let path = unsafe { std::ffi::CStr::from_ptr(path).to_str()? };
        let app_image = AppImage::new(path)?;
        let mut files = Vec::new();
        
        for file in app_image.files()? {
            files.push(CString::new(file)?.into_raw());
        }
        
        // Add null terminator
        files.push(std::ptr::null_mut());
        
        // Convert to raw pointer and forget the Vec to prevent deallocation
        let ptr = files.as_mut_ptr();
        std::mem::forget(files);
        Ok(ptr)
    })
}

/// Free a string list allocated by appimage_list_files
#[no_mangle]
pub extern "C" fn appimage_string_list_free(list: *mut *mut c_char) {
    if list.is_null() {
        return;
    }

    unsafe {
        let mut ptr = list;
        while !(*ptr).is_null() {
            CString::from_raw(*ptr);
            ptr = ptr.add(1);
        }
        let len = ptr.offset_from(list) as usize;
        Box::from_raw(std::slice::from_raw_parts_mut(list, len));
    }
}

/// Read a file from an AppImage into a buffer
#[no_mangle]
pub extern "C" fn appimage_read_file_into_buffer_following_symlinks(
    appimage_file_path: *const c_char,
    file_path: *const c_char,
    buffer: *mut *mut c_char,
    buf_size: *mut c_ulong,
) -> bool {
    if appimage_file_path.is_null() || file_path.is_null() || buffer.is_null() || buf_size.is_null() {
        return false;
    }

    catch_all(|| {
        let app_path = unsafe { std::ffi::CStr::from_ptr(appimage_file_path).to_str()? };
        let file_path = unsafe { std::ffi::CStr::from_ptr(file_path).to_str()? };
        
        let app_image = AppImage::new(app_path)?;
        let contents = app_image.read_file(file_path)?;
        
        unsafe {
            *buffer = libc::malloc(contents.len()) as *mut c_char;
            std::ptr::copy_nonoverlapping(contents.as_ptr(), *buffer as *mut u8, contents.len());
            *buf_size = contents.len() as c_ulong;
        }
        
        Ok(true)
    })
}

/// Extract a file from an AppImage
#[no_mangle]
pub extern "C" fn appimage_extract_file_following_symlinks(
    appimage_file_path: *const c_char,
    file_path: *const c_char,
    target_file_path: *const c_char,
) {
    if appimage_file_path.is_null() || file_path.is_null() || target_file_path.is_null() {
        return;
    }

    catch_all(|| {
        let app_path = unsafe { std::ffi::CStr::from_ptr(appimage_file_path).to_str()? };
        let file_path = unsafe { std::ffi::CStr::from_ptr(file_path).to_str()? };
        let target_path = unsafe { std::ffi::CStr::from_ptr(target_file_path).to_str()? };
        
        let app_image = AppImage::new(app_path)?;
        app_image.extract_file(file_path, target_path)?;
        
        Ok(())
    });
}

/// Get the MD5 hash of an AppImage
#[no_mangle]
pub extern "C" fn appimage_get_md5(appimage: *const c_void, hash: *mut c_char, hash_len: c_int) -> c_int {
    if appimage.is_null() || hash.is_null() || hash_len <= 0 {
        return ErrorCode::IoError.into();
    }

    let appimage = unsafe { &*(appimage as *const AppImage) };
    let hash_str = match appimage.get_md5() {
        Ok(h) => h,
        Err(e) => {
            unsafe { LAST_ERROR = Some(e.to_string()); }
            return ErrorCode::IoError.into();
        }
    };

    let c_hash = match CString::new(hash_str) {
        Ok(h) => h,
        Err(e) => {
            unsafe { LAST_ERROR = Some(e.to_string()); }
            return ErrorCode::StringConversionError.into();
        }
    };

    unsafe {
        let src = c_hash.as_bytes_with_nul();
        let dst = std::slice::from_raw_parts_mut(hash as *mut u8, hash_len as usize);
        let len = std::cmp::min(src.len(), dst.len());
        dst[..len].copy_from_slice(&src[..len]);
    }

    ErrorCode::Success.into()
}

/// Get the AppImage payload offset
#[no_mangle]
pub extern "C" fn appimage_get_payload_offset(appimage: *const c_void) -> i64 {
    if appimage.is_null() {
        return -1;
    }

    let appimage = unsafe { &*(appimage as *const AppImage) };
    match appimage.get_payload_offset() {
        Ok(offset) => offset,
        Err(e) => {
            unsafe { LAST_ERROR = Some(e.to_string()); }
            -1
        }
    }
}

#[cfg(feature = "desktop-integration")]
mod desktop_integration_ffi {
    use super::*;
    use crate::desktop_integration::IntegrationManager;

    /// Register an AppImage in the system
    #[no_mangle]
    pub extern "C" fn appimage_register_in_system(path: *const c_char, verbose: bool) -> c_int {
        if path.is_null() {
            return 1;
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
            
            Ok(0)
        })
    }

    /// Unregister an AppImage from the system
    #[no_mangle]
    pub extern "C" fn appimage_unregister_in_system(path: *const c_char, verbose: bool) -> c_int {
        if path.is_null() {
            return 1;
        }

        catch_all(|| {
            let path = unsafe { std::ffi::CStr::from_ptr(path).to_str()? };
            let manager = IntegrationManager::new();
            
            manager.unregister_app_image(path)?;
            
            #[cfg(feature = "thumbnailer")]
            {
                manager.remove_thumbnails(path)?;
            }
            
            Ok(0)
        })
    }

    /// Check if an AppImage is registered in the system
    #[no_mangle]
    pub extern "C" fn appimage_is_registered_in_system(path: *const c_char) -> bool {
        if path.is_null() {
            return false;
        }

        catch_all(|| {
            let path = unsafe { std::ffi::CStr::from_ptr(path).to_str()? };
            let manager = IntegrationManager::new();
            Ok(manager.is_registered_app_image(path)?)
        })
    }

    #[cfg(feature = "thumbnailer")]
    /// Create AppImage thumbnail
    #[no_mangle]
    pub extern "C" fn appimage_create_thumbnail(appimage_file_path: *const c_char, verbose: bool) -> bool {
        if appimage_file_path.is_null() {
            return false;
        }

        catch_all(|| {
            let path = unsafe { std::ffi::CStr::from_ptr(appimage_file_path).to_str()? };
            let app_image = AppImage::new(path)?;
            let manager = IntegrationManager::new();
            
            manager.generate_thumbnails(&app_image)?;
            Ok(true)
        })
    }
}

/// Convert a C string to a Path
fn c_str_to_path(path: *const c_char) -> AppImageResult<&'static Path> {
    if path.is_null() {
        return Err(AppImageError::InvalidParameter("Path is null".into()));
    }

    let c_str = unsafe { std::ffi::CStr::from_ptr(path) };
    let path_str = c_str.to_str().into_appimage_string_error()?;
    Ok(Path::new(path_str))
}

/// Convert a string to a C string
fn str_to_c_string(s: &str) -> AppImageResult<CString> {
    CString::new(s).into_appimage_string_error()
}

/// Set the log level
#[no_mangle]
pub extern "C" fn appimage_set_log_level(level: c_int) -> c_int {
    let log_level = match level {
        0 => LogLevel::Debug,
        1 => LogLevel::Info,
        2 => LogLevel::Warning,
        3 => LogLevel::Error,
        _ => return ErrorCode::IoError as c_int,
    };

    Logger::instance().set_level(log_level);
    ErrorCode::Success.into()
}

/// Set the log callback
#[no_mangle]
pub extern "C" fn appimage_set_log_callback(callback: extern "C" fn(level: c_int, message: *const c_char)) -> c_int {
    Logger::instance().set_callback(move |level, message| {
        let level_int = match level {
            LogLevel::Debug => 0,
            LogLevel::Info => 1,
            LogLevel::Warning => 2,
            LogLevel::Error => 3,
        };

        if let Ok(c_message) = str_to_c_string(&message) {
            callback(level_int, c_message.as_ptr());
        }
    });

    ErrorCode::Success as c_int
}

/// Get the last error message
#[no_mangle]
pub extern "C" fn appimage_get_error() -> *const c_char {
    if let Some(error) = get_last_error() {
        if let Ok(c_str) = CString::new(error) {
            return c_str.into_raw();
        }
    }
    std::ptr::null()
}

/// Free the last error message
#[no_mangle]
pub extern "C" fn appimage_free_error(error: *mut c_char) {
    if !error.is_null() {
        unsafe {
            let _ = CString::from_raw(error);
        }
    }
}

/// Get the error code for the last error
#[no_mangle]
pub extern "C" fn appimage_get_error_code() -> ErrorCode {
    if let Some(error) = get_last_error() {
        match error.as_str() {
            "IO error" => ErrorCode::IoError,
            "Format error" => ErrorCode::FormatError,
            "String conversion error" => ErrorCode::StringConversionError,
            "Nul error" => ErrorCode::NulError,
            "UTF-8 error" => ErrorCode::Utf8Error,
            "Desktop entry edit error" => ErrorCode::DesktopEntryEditError,
            "Desktop integration error" => ErrorCode::DesktopIntegrationError,
            "SquashFS error" => ErrorCode::SquashFsError,
            "Handler is not open" => ErrorCode::NotOpen,
            "File not found" => ErrorCode::FileNotFound,
            "Not a file" => ErrorCode::NotAFile,
            _ => ErrorCode::Success,
        }
    } else {
        ErrorCode::Success
    }
}

/// Set the last error message
fn set_last_error(error: AppImageError) {
    unsafe {
        LAST_ERROR = Some(error.to_string());
    }
}

/// Get the last error message
fn get_last_error() -> Option<String> {
    unsafe {
        LAST_ERROR.take()
    }
}

/// Helper macro to handle FFI results
macro_rules! ffi_try {
    ($expr:expr) => {
        match $expr {
            Ok(val) => val,
            Err(err) => {
                set_last_error(err);
                false
            }
        }
    };
}

/// Create a new AppImage instance
#[no_mangle]
pub extern "C" fn appimage_new(path: *const c_char) -> *mut c_void {
    let path = match c_str_to_path(path) {
        Ok(p) => p,
        Err(err) => {
            set_last_error(err);
            return ptr::null_mut();
        }
    };

    match AppImage::new(path) {
        Ok(appimage) => Box::into_raw(Box::new(appimage)) as *mut c_void,
        Err(err) => {
            set_last_error(err);
            ptr::null_mut()
        }
    }
}

/// Free an AppImage instance
#[no_mangle]
pub extern "C" fn appimage_free(appimage: *mut c_void) {
    if !appimage.is_null() {
        unsafe {
            drop(Box::from_raw(appimage as *mut AppImage));
        }
    }
}

/// Get the AppImage format
#[no_mangle]
pub extern "C" fn appimage_get_format(appimage: *const c_void) -> c_int {
    if appimage.is_null() {
        return ErrorCode::IoError as c_int;
    }

    let appimage = unsafe { &*(appimage as *const AppImage) };
    match appimage.format() {
        AppImageFormat::Type1 => 1,
        AppImageFormat::Type2 => 2,
        _ => 0,
    }
}

/// Extract a file from the AppImage
#[no_mangle]
pub extern "C" fn appimage_extract_file(
    appimage: *const c_void,
    source: *const c_char,
    target: *const c_char,
) -> bool {
    if appimage.is_null() {
        return false;
    }

    let appimage = unsafe { &*(appimage as *const AppImage) };
    let source = ffi_try!(c_str_to_path(source));
    let target = ffi_try!(c_str_to_path(target));

    ffi_try!(appimage.extract_file(source, target))
}

/// Get the AppImage size
#[no_mangle]
pub extern "C" fn appimage_get_size(appimage: *const c_void) -> c_ulong {
    if appimage.is_null() {
        set_last_error(AppImageError::InvalidParameter("AppImage is null".into()));
        return 0;
    }

    let appimage = unsafe { &*(appimage as *const AppImage) };
    match appimage.size() {
        Ok(size) => size as c_ulong,
        Err(err) => {
            set_last_error(err);
            0
        }
    }
}

/// Integrate the AppImage into the system
#[no_mangle]
pub extern "C" fn appimage_integrate(appimage: *const c_void) -> bool {
    if appimage.is_null() {
        return false;
    }

    let appimage = unsafe { &*(appimage as *const AppImage) };
    let manager = IntegrationManager::new();

    ffi_try!(manager.integrate(appimage))
}

/// Remove the AppImage integration from the system
#[no_mangle]
pub extern "C" fn appimage_unintegrate(appimage: *const c_void) -> bool {
    if appimage.is_null() {
        return false;
    }

    let appimage = unsafe { &*(appimage as *const AppImage) };
    let manager = IntegrationManager::new();

    ffi_try!(manager.unintegrate(appimage))
}

/// Check if the AppImage is integrated
#[no_mangle]
pub extern "C" fn appimage_is_integrated(appimage: *const c_void) -> bool {
    if appimage.is_null() {
        return false;
    }

    let appimage = unsafe { &*(appimage as *const AppImage) };
    let manager = IntegrationManager::new();

    ffi_try!(manager.is_integrated(appimage))
}

/// Get the AppImage path
#[no_mangle]
pub extern "C" fn appimage_get_path(appimage: *const c_void) -> *const c_char {
    if appimage.is_null() {
        return ptr::null();
    }

    let appimage = unsafe { &*(appimage as *const AppImage) };
    let path = appimage.get_path();
    let c_path = CString::new(path.to_string_lossy().into_owned())?;
    c_path.into_raw()
}

/// Get the AppImage files
#[no_mangle]
pub extern "C" fn appimage_get_files(appimage: *const c_void) -> *mut *mut c_char {
    if appimage.is_null() {
        return ptr::null_mut();
    }

    let appimage = unsafe { &*(appimage as *const AppImage) };
    let files = appimage.files()?;
    let mut file_list = Vec::new();
    for file in files {
        file_list.push(CString::new(file)?.into_raw());
    }
    Ok(Box::into_raw(Box::new(file_list.into_boxed_slice())) as *mut *mut c_char)
}

/// Free the AppImage files
#[no_mangle]
pub extern "C" fn appimage_free_files(files: *mut *mut c_char) {
    if !files.is_null() {
        unsafe {
            let mut len = 0;
            let mut current = files;
            while !(*current).is_null() {
                let _ = CString::from_raw(*current);
                current = current.add(1);
                len += 1;
            }
            Vec::from_raw_parts(files, len, len + 1);
        }
    }
}

/// Check if an AppImage is a terminal application
#[no_mangle]
pub extern "C" fn appimage_is_terminal_app(path: *const c_char) -> c_int {
    if path.is_null() {
        return -1;
    }

    catch_all(|| {
        let path = unsafe { std::ffi::CStr::from_ptr(path).to_str()? };
        let app_image = AppImage::new(path)?;
        Ok(app_image.is_terminal_app()? as c_int)
    })
}

// Add other FFI functions here using the same error handling pattern 