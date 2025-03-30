use std::io::{self, Write};
use std::ffi::{NulError, FromBytesWithNulError};
use std::str::Utf8Error;
use std::fmt;
use thiserror::Error;
use crate::format::FormatError;
use crate::utils::IconHandleError;

/// Error types for SquashFS operations
#[derive(Error, Debug)]
pub enum SquashError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Invalid magic number")]
    InvalidMagic,

    #[error("Unsupported big-endian filesystem")]
    UnsupportedBigEndian,

    #[error("Unsupported version {major}.{minor}")]
    UnsupportedVersion {
        major: u16,
        minor: u16,
    },

    #[error("Invalid block size")]
    InvalidBlockSize,

    #[error("Corrupted filesystem: {0}")]
    Corrupted(String),

    #[error("Compression error: {0}")]
    Compression(String),

    #[error("Invalid inode type: {0}")]
    InvalidInodeType(u16),

    #[error("Invalid directory entry")]
    InvalidDirectoryEntry,

    #[error("Invalid fragment entry")]
    InvalidFragmentEntry,

    #[error("Invalid id table")]
    InvalidIdTable,

    #[error("Invalid lookup table")]
    InvalidLookupTable,

    #[error("Invalid extended attributes")]
    InvalidXattrs,

    #[error("Other error: {0}")]
    Other(String),
}

pub type SquashResult<T> = Result<T, SquashError>;

/// Progress bar interface for displaying progress and errors
pub trait ProgressBar {
    fn error(&mut self, fmt: fmt::Arguments<'_>) -> io::Result<()>;
    fn info(&mut self, fmt: fmt::Arguments<'_>) -> io::Result<()>;
    fn disable(&mut self);
    fn enable(&mut self);
}

/// Default implementation using stdout/stderr
pub struct DefaultProgressBar {
    stdout: io::Stdout,
    stderr: io::Stderr,
    enabled: bool,
}

impl DefaultProgressBar {
    pub fn new() -> Self {
        DefaultProgressBar {
            stdout: io::stdout(),
            stderr: io::stderr(),
            enabled: true,
        }
    }
}

impl ProgressBar for DefaultProgressBar {
    fn error(&mut self, fmt: fmt::Arguments<'_>) -> io::Result<()> {
        if self.enabled {
            writeln!(self.stderr, "{}", fmt)
        } else {
            Ok(())
        }
    }

    fn info(&mut self, fmt: fmt::Arguments<'_>) -> io::Result<()> {
        if self.enabled {
            writeln!(self.stdout, "{}", fmt)
        } else {
            Ok(())
        }
    }

    fn disable(&mut self) {
        self.enabled = false;
    }

    fn enable(&mut self) {
        self.enabled = true;
    }
}

/// Global progress bar instance
static mut PROGRESS_BAR: Option<Box<dyn ProgressBar>> = None;

/// Initialize the progress bar
pub fn init_progress_bar(bar: Box<dyn ProgressBar>) {
    unsafe {
        PROGRESS_BAR = Some(bar);
    }
}

/// Get the current progress bar instance
fn get_progress_bar() -> &'static mut Box<dyn ProgressBar> {
    unsafe {
        PROGRESS_BAR.as_mut().expect("Progress bar not initialized")
    }
}

/// Print an error message
#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {
        if let Err(e) = get_progress_bar().error(format_args!($($arg)*)) {
            eprintln!("Failed to write error message: {}", e);
        }
    };
}

/// Print an info message
#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        if let Err(e) = get_progress_bar().info(format_args!($($arg)*)) {
            eprintln!("Failed to write info message: {}", e);
        }
    };
}

/// Print a trace message (only if SQUASHFS_TRACE is enabled)
#[cfg(feature = "trace")]
#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => {
        info!("squashfs: {}", format_args!($($arg)*));
    };
}

#[cfg(not(feature = "trace"))]
#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => {};
}

/// Handle memory errors
#[macro_export]
macro_rules! mem_error {
    ($func:expr) => {
        error!("FATAL ERROR: Out of memory ({})", $func);
        exit_squashfs();
    };
}

/// Exit the program with an error
pub fn exit_squashfs() {
    std::process::exit(1);
}

/// Global state for error handling
pub struct ErrorState {
    /// Whether to exit on error
    pub exit_on_error: bool,
    /// Whether to display info
    pub display_info: bool,
    /// File to write info to
    pub info_file: Option<Box<dyn Write>>,
}

impl Default for ErrorState {
    fn default() -> Self {
        Self {
            exit_on_error: true,
            display_info: true,
            info_file: None,
        }
    }
}

impl ErrorState {
    /// Create a new error state
    pub fn new() -> Self {
        Self::default()
    }

    /// Display an info message
    pub fn info(&mut self, message: &str) {
        if self.display_info {
            if let Some(file) = &mut self.info_file {
                let _ = writeln!(file, "{}", message);
            } else {
                println!("{}", message);
            }
        }
    }

    /// Display an error message and handle based on exit_on_error setting
    pub fn error(&mut self, message: &str) {
        eprintln!("{}", message);

        if self.exit_on_error {
            eprintln!();
            self.exit();
        }
    }

    /// Display a fatal error message and exit
    pub fn fatal_error(&mut self, message: &str) {
        eprintln!("FATAL ERROR: {}", message);
        self.exit();
    }

    /// Prepare for exit and terminate the program
    pub fn exit(&mut self) {
        std::process::exit(1);
    }
}

/// Base error type for all AppImage-related errors
#[derive(Error, Debug)]
pub enum AppImageError {
    /// IO operation failed
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    /// Invalid AppImage format
    #[error("Invalid format: {0}")]
    InvalidFormat(String),

    /// ELF file operation failed
    #[error("ELF error: {0}")]
    Elf(String),

    /// File system operation failed
    #[error("File system error: {0}")]
    FileSystem(String),

    /// Archive operation failed
    #[error("Archive error: {0}")]
    Archive(String),

    /// SquashFS operation failed
    #[error("SquashFS error: {0}")]
    SquashFs(String),

    /// Operation not supported
    #[error("Operation not supported: {0}")]
    NotSupported(String),

    /// Invalid parameter
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    /// Resource not found
    #[error("Resource not found: {0}")]
    NotFound(String),

    /// Permission denied
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Operation failed
    #[error("Operation failed: {0}")]
    OperationFailed(String),

    /// String conversion error
    #[error("String conversion error: {0}")]
    StringConversion(String),

    /// Null terminator error
    #[error("Null terminator error: {0}")]
    NulError(String),

    /// Invalid null-terminated string
    #[error("Invalid null-terminated string: {0}")]
    FromBytesWithNul(#[from] FromBytesWithNulError),

    /// Desktop integration error
    #[error("Desktop integration error: {0}")]
    DesktopIntegration(String),

    /// Handler is not open
    #[error("Handler is not open")]
    NotOpen,

    /// File not found
    #[error("File not found: {0}")]
    FileNotFound(String),

    /// Not a file
    #[error("Not a file")]
    NotAFile,

    /// Invalid path
    #[error("Invalid path: {0}")]
    InvalidPath(String),

    /// Invalid data
    #[error("Invalid data: {0}")]
    InvalidData(String),

    /// Invalid header
    #[error("Invalid header: {0}")]
    InvalidHeader(String),

    /// Invalid footer
    #[error("Invalid footer: {0}")]
    InvalidFooter(String),

    /// Invalid magic
    #[error("Invalid magic: {0}")]
    InvalidMagic(String),

    /// Already exists
    #[error("Already exists: {0}")]
    AlreadyExists(String),

    /// Invalid state
    #[error("Invalid state: {0}")]
    InvalidState(String),

    /// Invalid operation
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
}

/// Result type for AppImage operations
pub type AppImageResult<T> = Result<T, AppImageError>;

/// Helper trait for converting other error types into AppImageError
pub trait IntoAppImageError<T> {
    /// Convert the error into an AppImageError
    fn into_appimage_error(self) -> AppImageResult<T>;
}

impl<T, E> IntoAppImageError<T> for Result<T, E>
where
    E: std::fmt::Display,
{
    fn into_appimage_error(self) -> AppImageResult<T> {
        self.map_err(|e| AppImageError::OperationFailed(e.to_string()))
    }
}

/// Helper trait for converting IO errors into AppImageError
pub trait IntoAppImageIoError<T> {
    /// Convert the IO error into an AppImageError
    fn into_appimage_io_error(self) -> AppImageResult<T>;
}

impl<T> IntoAppImageIoError<T> for Result<T, io::Error> {
    fn into_appimage_io_error(self) -> AppImageResult<T> {
        self.map_err(AppImageError::Io)
    }
}

/// Helper trait for converting string errors into AppImageError
pub trait IntoAppImageStringError<T> {
    /// Convert the string error into an AppImageError
    fn into_appimage_string_error(self) -> AppImageResult<T>;
}

impl<T> IntoAppImageStringError<T> for Result<T, Utf8Error> {
    fn into_appimage_string_error(self) -> AppImageResult<T> {
        self.map_err(|e| AppImageError::StringConversion(e.to_string()))
    }
}

impl<T> IntoAppImageStringError<T> for Result<T, NulError> {
    fn into_appimage_string_error(self) -> AppImageResult<T> {
        self.map_err(|e| AppImageError::NulError(e.to_string()))
    }
}

impl<T> IntoAppImageStringError<T> for Result<T, FromBytesWithNulError> {
    fn into_appimage_string_error(self) -> AppImageResult<T> {
        self.map_err(|e| AppImageError::FromBytesWithNul(e))
    }
}

impl<T> IntoAppImageStringError<T> for Result<T, String> {
    fn into_appimage_string_error(self) -> AppImageResult<T> {
        self.map_err(|e| AppImageError::OperationFailed(e))
    }
}

impl From<FormatError> for AppImageError {
    fn from(error: FormatError) -> Self {
        AppImageError::InvalidFormat(error.to_string())
    }
}

impl From<IconHandleError> for AppImageError {
    fn from(error: IconHandleError) -> Self {
        AppImageError::OperationFailed(error.to_string())
    }
}

impl From<AppImageError> for DesktopIntegrationError {
    fn from(error: AppImageError) -> Self {
        AppImageError::DesktopIntegration(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_conversion() {
        // Test IO error conversion
        let io_err = io::Error::new(io::ErrorKind::NotFound, "File not found");
        let app_err: AppImageError = io_err.into();
        assert!(matches!(app_err, AppImageError::Io(_)));

        // Test string error conversion
        let str_err = "Test error".to_string();
        let app_err = AppImageError::OperationFailed(str_err);
        assert!(matches!(app_err, AppImageError::OperationFailed(_)));

        // Test helper trait
        let result: Result<(), String> = Err("Test error".to_string());
        let app_result = result.into_appimage_string_error();
        assert!(matches!(app_result, Err(AppImageError::OperationFailed(_))));
    }

    #[test]
    fn test_error_display() {
        let err = AppImageError::InvalidFormat("Test format error".to_string());
        assert_eq!(err.to_string(), "Invalid format: Test format error");

        let err = AppImageError::NotFound("Test file".to_string());
        assert_eq!(err.to_string(), "Resource not found: Test file");
    }
}

/// Error type for desktop integration operations
#[derive(Debug, thiserror::Error)]
pub enum DesktopIntegrationError {
    /// IO operation failed
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    /// Desktop integration error
    #[error("Desktop integration error: {0}")]
    DesktopIntegration(String),

    /// Permission denied
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Operation not supported
    #[error("Operation not supported: {0}")]
    NotSupported(String),

    /// Invalid parameter
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    /// Resource not found
    #[error("Resource not found: {0}")]
    NotFound(String),

    /// Operation failed
    #[error("Operation failed: {0}")]
    OperationFailed(String),
}

impl From<IconHandleError> for DesktopIntegrationError {
    fn from(error: IconHandleError) -> Self {
        DesktopIntegrationError::OperationFailed(error.to_string())
    }
}

impl From<DesktopIntegrationError> for AppImageError {
    fn from(error: DesktopIntegrationError) -> Self {
        AppImageError::DesktopIntegration(error.to_string())
    }
} 