use std::fmt;
use std::io::{self, Read};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FormatError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Invalid format: {0}")]
    Invalid(String),
}

/// Represents the different formats of AppImage files
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppImageFormat {
    /// Type 1 AppImage format (based on ISO9660)
    Type1,
    /// Type 2 AppImage format (based on SquashFS)
    Type2,
    /// Invalid or unknown format
    Invalid,
}

impl AppImageFormat {
    /// Inspect the magic bytes of the file to guess the AppImage format
    pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self, FormatError> {
        let path = path.as_ref();
        let mut file = std::fs::File::open(path)?;
        let mut buffer = [0u8; 32774]; // Need enough space for ISO9660 signature check
        file.read_exact(&mut buffer)?;

        // Check for ELF signature
        if buffer[0] != 0x7F || buffer[1] != b'E' || buffer[2] != b'L' || buffer[3] != b'F' {
            return Ok(AppImageFormat::Invalid);
        }

        // Check for AppImage type 1 signature
        if buffer[8] == 0x41 && buffer[9] == 0x49 && buffer[10] == 0x01 {
            return Ok(AppImageFormat::Type1);
        }

        // Check for AppImage type 2 signature
        if buffer[8] == 0x41 && buffer[9] == 0x49 && buffer[10] == 0x02 {
            return Ok(AppImageFormat::Type2);
        }

        // Check for ISO9660 signature (Type 1 without magic bytes)
        if buffer[32769] == 0x43 && buffer[32770] == 0x44 && buffer[32771] == 0x30 && buffer[32772] == 0x30 && buffer[32773] == 0x31 {
            eprintln!("WARNING: {} seems to be a Type 1 AppImage without magic bytes.", path.display());
            return Ok(AppImageFormat::Type1);
        }

        Ok(AppImageFormat::Invalid)
    }

    /// Check if the format is valid
    pub fn is_valid(&self) -> bool {
        matches!(self, AppImageFormat::Type1 | AppImageFormat::Type2)
    }

    /// Get the format as a string
    pub fn as_str(&self) -> &'static str {
        match self {
            AppImageFormat::Type1 => "Type 1",
            AppImageFormat::Type2 => "Type 2",
            AppImageFormat::Invalid => "Invalid",
        }
    }

    /// Get the format as a number
    pub fn as_number(&self) -> Option<u8> {
        match self {
            AppImageFormat::Type1 => Some(1),
            AppImageFormat::Type2 => Some(2),
            AppImageFormat::Invalid => None,
        }
    }
}

impl fmt::Display for AppImageFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<u8> for AppImageFormat {
    fn from(value: u8) -> Self {
        match value {
            1 => AppImageFormat::Type1,
            2 => AppImageFormat::Type2,
            _ => AppImageFormat::Invalid,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_detection() {
        let path = "test.AppImage";
        let format = AppImageFormat::from_file(path).unwrap();
        assert_eq!(format, AppImageFormat::Type2);
        assert!(format.is_valid());
        assert_eq!(format.as_str(), "Type 2");
        assert_eq!(format.as_number(), Some(2));
    }

    #[test]
    fn test_format_conversion() {
        assert_eq!(AppImageFormat::from(1), AppImageFormat::Type1);
        assert_eq!(AppImageFormat::from(2), AppImageFormat::Type2);
        assert_eq!(AppImageFormat::from(3), AppImageFormat::Invalid);
    }

    #[test]
    fn test_format_display() {
        assert_eq!(AppImageFormat::Type1.to_string(), "Type 1");
        assert_eq!(AppImageFormat::Type2.to_string(), "Type 2");
        assert_eq!(AppImageFormat::Invalid.to_string(), "Invalid");
    }
} 