use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use std::ffi::CString;
use std::os::unix::fs::MetadataExt;
use regex::Regex;
use crate::squashfs_tools::error::{Error, Result};
use crate::squashfs_tools::symbolic_mode::{ModeData, parse_mode, execute_mode};

/// TAR file type constants
const TAR_NORMAL1: u8 = b'0';
const TAR_NORMAL2: u8 = b'\0';
const TAR_HARD: u8 = b'1';
const TAR_SYM: u8 = b'2';
const TAR_CHAR: u8 = b'3';
const TAR_BLOCK: u8 = b'4';
const TAR_DIR: u8 = b'5';
const TAR_FIFO: u8 = b'6';
const TAR_NORMAL3: u8 = b'7';
const TAR_GXHDR: u8 = b'g';
const TAR_XHDR: u8 = b'x';
const GNUTAR_LONG_NAME: u8 = b'L';
const GNUTAR_LONG_LINK: u8 = b'K';
const GNUTAR_SPARSE: u8 = b'S';
const SOLARIS_XHDR: u8 = b'X';

/// TAR magic constants
const V7_MAGIC: &[u8] = b"\0\0\0\0\0\0\0";
const GNU_MAGIC: &[u8] = b"ustar  ";
const USTAR_MAGIC: &[u8] = b"ustar\00000";

/// Encoding types for extended attributes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum XattrEncoding {
    Base64,
    Binary,
}

/// Represents a sparse file map entry
#[derive(Debug, Clone)]
pub struct FileMap {
    pub offset: i64,
    pub number: i64,
}

/// Represents an extended attribute
#[derive(Debug)]
pub struct XattrList {
    pub full_name: String,
    pub name: String,
    pub value: Vec<u8>,
    pub vsize: usize,
    pub type_: i32,
}

/// Represents a TAR file entry
#[derive(Debug)]
pub struct TarFile {
    pub realsize: i64,
    pub buf: std::fs::Metadata,
    pub file: Option<Box<dyn Read>>,
    pub xattr_list: Vec<XattrList>,
    pub map: Option<Vec<FileMap>>,
    pub pathname: String,
    pub link: Option<String>,
    pub uname: Option<String>,
    pub gname: Option<String>,
    pub xattrs: usize,
    pub map_entries: usize,
    pub have_size: bool,
    pub have_uid: bool,
    pub have_gid: bool,
    pub have_mtime: bool,
    pub sparse_pax: u8,
}

impl TarFile {
    pub fn new() -> Self {
        Self {
            realsize: 0,
            buf: std::fs::metadata(".").unwrap(), // Temporary, will be replaced
            file: None,
            xattr_list: Vec::new(),
            map: None,
            pathname: String::new(),
            link: None,
            uname: None,
            gname: None,
            xattrs: 0,
            map_entries: 0,
            have_size: false,
            have_uid: false,
            have_gid: false,
            have_mtime: false,
            sparse_pax: 0,
        }
    }
}

/// Read extended attributes from a TAR file
pub fn read_tar_xattr(name: &str, value: &[u8], size: usize, encoding: XattrEncoding, file: &mut TarFile) -> Result<()> {
    // Check for duplicate xattrs
    for xattr in &file.xattr_list {
        if xattr.full_name == name {
            return Ok(());
        }
    }

    // Handle encoding
    let data = match encoding {
        XattrEncoding::Base64 => {
            base64::decode(value)
                .map_err(|_| Error::InvalidXattr("Invalid base64 value".to_string()))?
        }
        XattrEncoding::Binary => value.to_vec(),
    };

    // Create new xattr
    let mut xattr = XattrList {
        full_name: name.to_string(),
        name: String::new(),
        value: data,
        vsize: size,
        type_: -1,
    };

    // Get xattr prefix
    xattr.type_ = xattr_get_prefix(&xattr)?;
    if xattr.type_ == -1 {
        return Err(Error::InvalidXattr(format!("Unrecognised tar xattr prefix {}", name)));
    }

    file.xattr_list.push(xattr);
    file.xattrs += 1;
    Ok(())
}

/// Get the prefix type for an extended attribute
pub fn xattr_get_prefix(xattr: &XattrList) -> Result<i32> {
    let prefix = xattr.full_name.split_once('.')
        .map(|(p, _)| p)
        .unwrap_or(&xattr.full_name);

    match prefix {
        "security" => Ok(0),
        "system" => Ok(1),
        "trusted" => Ok(2),
        "user" => Ok(3),
        _ => Ok(-1),
    }
}

/// Read extended attributes from a TAR file for an inode
pub fn read_xattrs_from_tarfile(inode: &std::fs::Metadata) -> Result<Vec<XattrList>> {
    // This would be implemented to read xattrs from the TAR file
    // For now, return empty vector
    Ok(Vec::new())
}

/// Free extended attributes from a TAR file
pub fn free_tar_xattrs(file: &mut TarFile) {
    file.xattr_list.clear();
    file.xattrs = 0;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xattr_encoding() {
        let mut file = TarFile::new();
        let value = b"SGVsbG8gV29ybGQ="; // "Hello World" in base64
        
        read_tar_xattr("user.test", value, value.len(), XattrEncoding::Base64, &mut file).unwrap();
        assert_eq!(file.xattrs, 1);
        assert_eq!(file.xattr_list[0].value, b"Hello World");
    }

    #[test]
    fn test_xattr_prefix() {
        let xattr = XattrList {
            full_name: "user.test".to_string(),
            name: "test".to_string(),
            value: Vec::new(),
            vsize: 0,
            type_: -1,
        };

        assert_eq!(xattr_get_prefix(&xattr).unwrap(), 3); // user prefix
    }

    #[test]
    fn test_duplicate_xattr() {
        let mut file = TarFile::new();
        let value = b"test";
        
        read_tar_xattr("user.test", value, value.len(), XattrEncoding::Binary, &mut file).unwrap();
        read_tar_xattr("user.test", value, value.len(), XattrEncoding::Binary, &mut file).unwrap();
        
        assert_eq!(file.xattrs, 1); // Should not add duplicate
    }
} 