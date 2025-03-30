use std::fmt;
use crate::error::{AppImageError, AppImageResult};

/// Represents the type of entry in the AppImage payload
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayloadEntryType {
    /// Regular file
    File,
    /// Symbolic link
    Symlink,
    /// Directory
    Directory,
    /// Character device
    CharDevice,
    /// Block device
    BlockDevice,
    /// Named pipe (FIFO)
    Fifo,
    /// Socket
    Socket,
    /// Unknown entry type
    Unknown,
}

impl PayloadEntryType {
    /// Convert a mode_t value to a PayloadEntryType
    pub fn from_mode(mode: u32) -> Self {
        match mode & 0o170000 {
            0o100000 => Self::File,      // S_IFREG
            0o120000 => Self::Symlink,   // S_IFLNK
            0o040000 => Self::Directory, // S_IFDIR
            0o020000 => Self::CharDevice, // S_IFCHR
            0o060000 => Self::BlockDevice, // S_IFBLK
            0o010000 => Self::Fifo,      // S_IFIFO
            0o140000 => Self::Socket,    // S_IFSOCK
            _ => Self::Unknown,
        }
    }

    /// Convert a PayloadEntryType to a mode_t value
    pub fn to_mode(&self) -> u32 {
        match self {
            Self::File => 0o100000,      // S_IFREG
            Self::Symlink => 0o120000,   // S_IFLNK
            Self::Directory => 0o040000, // S_IFDIR
            Self::CharDevice => 0o020000, // S_IFCHR
            Self::BlockDevice => 0o060000, // S_IFBLK
            Self::Fifo => 0o010000,      // S_IFIFO
            Self::Socket => 0o140000,    // S_IFSOCK
            Self::Unknown => 0,
        }
    }

    /// Convert a string representation to a PayloadEntryType
    pub fn from_str(s: &str) -> AppImageResult<Self> {
        match s {
            "file" => Ok(Self::File),
            "symlink" => Ok(Self::Symlink),
            "directory" => Ok(Self::Directory),
            "char" => Ok(Self::CharDevice),
            "block" => Ok(Self::BlockDevice),
            "fifo" => Ok(Self::Fifo),
            "socket" => Ok(Self::Socket),
            _ => Err(AppImageError::InvalidParameter(format!(
                "Unknown entry type: {}",
                s
            ))),
        }
    }

    /// Check if the entry type is a regular file
    pub fn is_file(&self) -> bool {
        matches!(self, Self::File)
    }

    /// Check if the entry type is a symbolic link
    pub fn is_symlink(&self) -> bool {
        matches!(self, Self::Symlink)
    }

    /// Check if the entry type is a directory
    pub fn is_directory(&self) -> bool {
        matches!(self, Self::Directory)
    }

    /// Check if the entry type is a device (char or block)
    pub fn is_device(&self) -> bool {
        matches!(self, Self::CharDevice | Self::BlockDevice)
    }

    /// Check if the entry type is a special file (fifo or socket)
    pub fn is_special(&self) -> bool {
        matches!(self, Self::Fifo | Self::Socket)
    }
}

impl fmt::Display for PayloadEntryType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::File => write!(f, "file"),
            Self::Symlink => write!(f, "symlink"),
            Self::Directory => write!(f, "directory"),
            Self::CharDevice => write!(f, "char"),
            Self::BlockDevice => write!(f, "block"),
            Self::Fifo => write!(f, "fifo"),
            Self::Socket => write!(f, "socket"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_mode() {
        assert_eq!(PayloadEntryType::from_mode(0o100000), PayloadEntryType::File);
        assert_eq!(PayloadEntryType::from_mode(0o120000), PayloadEntryType::Symlink);
        assert_eq!(PayloadEntryType::from_mode(0o040000), PayloadEntryType::Directory);
        assert_eq!(PayloadEntryType::from_mode(0o020000), PayloadEntryType::CharDevice);
        assert_eq!(PayloadEntryType::from_mode(0o060000), PayloadEntryType::BlockDevice);
        assert_eq!(PayloadEntryType::from_mode(0o010000), PayloadEntryType::Fifo);
        assert_eq!(PayloadEntryType::from_mode(0o140000), PayloadEntryType::Socket);
        assert_eq!(PayloadEntryType::from_mode(0), PayloadEntryType::Unknown);
    }

    #[test]
    fn test_to_mode() {
        assert_eq!(PayloadEntryType::File.to_mode(), 0o100000);
        assert_eq!(PayloadEntryType::Symlink.to_mode(), 0o120000);
        assert_eq!(PayloadEntryType::Directory.to_mode(), 0o040000);
        assert_eq!(PayloadEntryType::CharDevice.to_mode(), 0o020000);
        assert_eq!(PayloadEntryType::BlockDevice.to_mode(), 0o060000);
        assert_eq!(PayloadEntryType::Fifo.to_mode(), 0o010000);
        assert_eq!(PayloadEntryType::Socket.to_mode(), 0o140000);
        assert_eq!(PayloadEntryType::Unknown.to_mode(), 0);
    }

    #[test]
    fn test_from_str() {
        assert_eq!(PayloadEntryType::from_str("file").unwrap(), PayloadEntryType::File);
        assert_eq!(PayloadEntryType::from_str("symlink").unwrap(), PayloadEntryType::Symlink);
        assert_eq!(PayloadEntryType::from_str("directory").unwrap(), PayloadEntryType::Directory);
        assert_eq!(PayloadEntryType::from_str("char").unwrap(), PayloadEntryType::CharDevice);
        assert_eq!(PayloadEntryType::from_str("block").unwrap(), PayloadEntryType::BlockDevice);
        assert_eq!(PayloadEntryType::from_str("fifo").unwrap(), PayloadEntryType::Fifo);
        assert_eq!(PayloadEntryType::from_str("socket").unwrap(), PayloadEntryType::Socket);
        assert!(PayloadEntryType::from_str("unknown").is_err());
    }

    #[test]
    fn test_predicates() {
        let file = PayloadEntryType::File;
        let symlink = PayloadEntryType::Symlink;
        let dir = PayloadEntryType::Directory;
        let char_dev = PayloadEntryType::CharDevice;
        let block_dev = PayloadEntryType::BlockDevice;
        let fifo = PayloadEntryType::Fifo;
        let socket = PayloadEntryType::Socket;

        assert!(file.is_file());
        assert!(symlink.is_symlink());
        assert!(dir.is_directory());
        assert!(char_dev.is_device());
        assert!(block_dev.is_device());
        assert!(fifo.is_special());
        assert!(socket.is_special());
    }

    #[test]
    fn test_display() {
        assert_eq!(PayloadEntryType::File.to_string(), "file");
        assert_eq!(PayloadEntryType::Symlink.to_string(), "symlink");
        assert_eq!(PayloadEntryType::Directory.to_string(), "directory");
        assert_eq!(PayloadEntryType::CharDevice.to_string(), "char");
        assert_eq!(PayloadEntryType::BlockDevice.to_string(), "block");
        assert_eq!(PayloadEntryType::Fifo.to_string(), "fifo");
        assert_eq!(PayloadEntryType::Socket.to_string(), "socket");
        assert_eq!(PayloadEntryType::Unknown.to_string(), "unknown");
    }
} 