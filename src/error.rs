use crate::format::FormatError;
use crate::icon_handle::IconHandleError;
use std::ffi::{FromBytesWithNulError, NulError};
use std::fmt;
use std::io::{self, Write};
use std::str::Utf8Error;
use thiserror::Error;

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
    UnsupportedVersion { major: u16, minor: u16 },

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

/// Error type for desktop entry editing operations
#[derive(Debug, thiserror::Error)]
pub enum DesktopEntryEditError {
    /// File operation error
    #[error("File error: {0}")]
    File(#[from] std::io::Error),

    /// Format error in desktop entry
    #[error("Format error: {0}")]
    Format(String),

    /// Missing required field
    #[error("Missing field: {0}")]
    MissingField(String),

    /// Invalid field value
    #[error("Invalid value: {0}")]
    InvalidValue(String),

    /// Operation failed
    #[error("Operation failed: {0}")]
    OperationFailed(String),
}

impl From<DesktopEntryEditError> for DesktopIntegrationError {
    fn from(error: DesktopEntryEditError) -> Self {
        DesktopIntegrationError::DesktopIntegration(error.to_string())
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

    #[test]
    fn test_desktop_entry_edit_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let edit_err = DesktopEntryEditError::File(io_err);
        assert!(matches!(edit_err, DesktopEntryEditError::File(_)));

        let format_err = DesktopEntryEditError::Format("Invalid format".to_string());
        assert!(matches!(format_err, DesktopEntryEditError::Format(_)));

        let missing_err = DesktopEntryEditError::MissingField("Name".to_string());
        assert!(matches!(
            missing_err,
            DesktopEntryEditError::MissingField(_)
        ));

        // Test error conversion
        let integration_err: DesktopIntegrationError = format_err.into();
        assert!(matches!(
            integration_err,
            DesktopIntegrationError::DesktopIntegration(_)
        ));
    }
}
