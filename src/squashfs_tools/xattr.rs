use std::path::Path;
use std::ffi::CString;
use std::os::unix::fs::MetadataExt;
use crate::error::{ErrorState, SquashError};
use std::collections::HashMap;
use crate::fs::{SquashfsSuperBlock, SquashfsXattrEntry};
use crate::alloc::{safe_malloc, safe_free};
use crate::read::read_block;
use std::ffi::{CString, CStr};
use regex::Regex;
use crate::squashfs_tools::error::{Error, Result};
use crate::squashfs_tools::squashfs_fs::{SquashfsXattrTable, SquashfsXattrId};
use crate::squashfs_tools::squashfs_swap::{swap_xattr_table, swap_xattr_id};
use crate::squashfs_tools::unsquashfs_error::{error, error_start, error_exit};

/// Structure representing an extended attribute
#[derive(Debug, Clone)]
pub struct Xattr {
    /// Name of the extended attribute
    pub name: String,
    /// Value of the extended attribute
    pub value: Vec<u8>,
}

/// Structure representing a list of extended attributes
#[derive(Debug, Default)]
pub struct XattrList {
    /// List of extended attributes
    pub attrs: Vec<Xattr>,
}

impl XattrList {
    /// Create a new empty xattr list
    pub fn new() -> Self {
        Self {
            attrs: Vec::new(),
        }
    }

    /// Add an extended attribute to the list
    pub fn add(&mut self, name: String, value: Vec<u8>) {
        self.attrs.push(Xattr { name, value });
    }
}

/// Read extended attributes from a file in the system
/// 
/// This function reads all extended attributes from a file and returns them in a list.
/// The function is only available if extended attributes are supported by the system.
/// 
/// # Arguments
/// * `filename` - Path to the file to read xattrs from
/// 
/// # Returns
/// * `Result<XattrList, SquashError>` - List of extended attributes or error
#[cfg(target_os = "linux")]
pub fn read_xattrs_from_system(filename: &Path) -> Result<XattrList, SquashError> {
    use std::fs::File;
    use std::io::Read;
    use std::os::unix::fs::OpenOptionsExt;
    use libc::{c_char, c_int, c_ulong, c_void, size_t};

    let mut xattrs = XattrList::new();
    let mut error_state = ErrorState::default();

    // Open file with O_RDONLY flag
    let file = File::options()
        .read(true)
        .custom_flags(libc::O_RDONLY)
        .open(filename)
        .map_err(|e| SquashError::IOError(e))?;

    // Get file descriptor
    let fd = file.as_raw_fd();

    // First call to get required buffer size
    let mut size: size_t = 0;
    unsafe {
        let ret = libc::listxattr(
            filename.as_os_str().as_ptr() as *const c_char,
            std::ptr::null_mut(),
            0,
        );
        if ret < 0 {
            return Err(SquashError::Other(format!("Failed to get xattr list size: {}", std::io::Error::last_os_error())));
        }
        size = ret as size_t;
    }

    if size == 0 {
        return Ok(xattrs);
    }

    // Allocate buffer for xattr names
    let mut names = vec![0u8; size];
    let mut ret: c_int;

    // Get list of xattr names
    unsafe {
        ret = libc::listxattr(
            filename.as_os_str().as_ptr() as *const c_char,
            names.as_mut_ptr() as *mut c_char,
            size,
        );
    }

    if ret < 0 {
        return Err(SquashError::Other(format!("Failed to list xattrs: {}", std::io::Error::last_os_error())));
    }

    // Process each xattr name
    let mut pos = 0;
    while pos < size {
        let name = unsafe {
            let name_ptr = names.as_ptr().add(pos) as *const c_char;
            CString::from_raw(name_ptr as *mut c_char)
        };

        let name_str = name.to_str().map_err(|e| {
            SquashError::Other(format!("Invalid xattr name encoding: {}", e))
        })?;

        // Get value size
        let mut value_size: size_t = 0;
        unsafe {
            let ret = libc::getxattr(
                filename.as_os_str().as_ptr() as *const c_char,
                name.as_ptr(),
                std::ptr::null_mut(),
                0,
            );
            if ret < 0 {
                return Err(SquashError::Other(format!("Failed to get xattr value size: {}", std::io::Error::last_os_error())));
            }
            value_size = ret as size_t;
        }

        if value_size > 0 {
            // Allocate buffer for value
            let mut value = vec![0u8; value_size];

            // Get xattr value
            unsafe {
                ret = libc::getxattr(
                    filename.as_os_str().as_ptr() as *const c_char,
                    name.as_ptr(),
                    value.as_mut_ptr() as *mut c_void,
                    value_size,
                );
            }

            if ret < 0 {
                return Err(SquashError::Other(format!("Failed to get xattr value: {}", std::io::Error::last_os_error())));
            }

            xattrs.add(name_str.to_string(), value);
        }

        pos += name.as_bytes().len() + 1;
    }

    Ok(xattrs)
}

/// Stub implementation for systems without xattr support
#[cfg(not(target_os = "linux"))]
pub fn read_xattrs_from_system(_filename: &Path) -> Result<XattrList, SquashError> {
    Ok(XattrList::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_xattr_list() {
        let mut list = XattrList::new();
        assert!(list.attrs.is_empty());

        list.add("test.name".to_string(), vec![1, 2, 3]);
        assert_eq!(list.attrs.len(), 1);
        assert_eq!(list.attrs[0].name, "test.name");
        assert_eq!(list.attrs[0].value, vec![1, 2, 3]);
    }

    #[test]
    fn test_read_xattrs() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"test content").unwrap();

        let result = read_xattrs_from_system(file.path());
        assert!(result.is_ok());
        
        let xattrs = result.unwrap();
        // Note: Actual xattr presence depends on system and file permissions
        // We just verify the function doesn't crash
    }
}

const SQUASHFS_METADATA_SIZE: usize = 8192;
const SQUASHFS_XATTR_USER: u8 = 0;
const SQUASHFS_XATTR_TRUSTED: u8 = 1;
const SQUASHFS_XATTR_SECURITY: u8 = 2;
const XATTR_PREFIX_MASK: u8 = 0x0F;
const SQUASHFS_XATTR_VALUE_OOL: u8 = 0x80;

#[derive(Debug)]
pub struct XattrTable {
    xattr_ids: Vec<SquashfsXattrId>,
    xattrs: Vec<u8>,
    xattr_table_start: i64,
    xattr_table_length: usize,
    hash_table: HashMap<i64, i64>,
}

#[repr(C)]
#[derive(Debug)]
struct SquashfsXattrId {
    xattr: u64,
    count: u32,
}

#[derive(Debug)]
struct Prefix {
    prefix: &'static str,
    type_: i32,
}

const PREFIX_TABLE: &[Prefix] = &[
    Prefix { prefix: "user.", type_: SQUASHFS_XATTR_USER as i32 },
    Prefix { prefix: "trusted.", type_: SQUASHFS_XATTR_TRUSTED as i32 },
    Prefix { prefix: "security.", type_: SQUASHFS_XATTR_SECURITY as i32 },
    Prefix { prefix: "", type_: -1 },
];

impl XattrTable {
    pub fn new() -> Self {
        Self {
            xattr_ids: Vec::new(),
            xattrs: Vec::new(),
            xattr_table_start: 0,
            xattr_table_length: 0,
            hash_table: HashMap::new(),
        }
    }

    fn save_xattr_block(&mut self, start: i64, offset: i64) -> Result<()> {
        self.hash_table.insert(start, offset);
        Ok(())
    }

    fn get_xattr_block(&self, start: i64) -> Option<i64> {
        self.hash_table.get(&start).copied()
    }

    fn read_xattr_entry(&self, entry: &SquashfsXattrEntry, name: &[u8]) -> Result<XattrList> {
        let type_ = entry.type_ & XATTR_PREFIX_MASK;
        let prefix = PREFIX_TABLE.iter()
            .find(|p| p.type_ == type_ as i32)
            .ok_or_else(|| SquashError::InvalidXattrs)?;

        let full_name = format!("{}{}", prefix.prefix, String::from_utf8_lossy(name));
        let name = full_name[prefix.prefix.len()..].to_string();

        Ok(XattrList {
            full_name,
            name,
            value: Vec::new(), // Will be filled in later
            size: entry.size as usize,
            vsize: 0, // Will be filled in later
            type_: entry.type_,
        })
    }

    pub fn read_xattrs_from_disk<R: std::io::Read + std::io::Seek>(
        &mut self,
        reader: &mut R,
        s_blk: &SquashfsSuperBlock,
        sanity_only: bool,
        table_start: Option<&mut i64>,
    ) -> Result<u32> {
        let mut id_table = [0u8; std::mem::size_of::<SquashfsXattrTable>()];
        reader.seek(std::io::SeekFrom::Start(s_blk.xattr_id_table_start as u64))?;
        reader.read_exact(&mut id_table)?;

        let id_table = unsafe {
            std::ptr::read_unaligned(id_table.as_ptr() as *const SquashfsXattrTable)
        };

        let ids = id_table.xattr_ids;
        if ids == 0 {
            return Err(SquashError::InvalidXattrs);
        }

        self.xattr_table_start = id_table.xattr_table_start;
        let index_bytes = (ids as usize * 16 + 8191) & !8191;
        let indexes = (ids as usize * 16 + 8191) / 8192;

        if index_bytes != (s_blk.bytes_used - (s_blk.xattr_id_table_start + std::mem::size_of::<SquashfsXattrTable>())) as usize {
            return Err(SquashError::InvalidXattrs);
        }

        if let Some(start) = table_start {
            *start = id_table.xattr_table_start;
        }

        if sanity_only {
            return Ok(ids);
        }

        // Read the index table
        let mut index = vec![0i64; indexes];
        reader.seek(std::io::SeekFrom::Start((s_blk.xattr_id_table_start + std::mem::size_of::<SquashfsXattrTable>()) as u64))?;
        reader.read_exact(unsafe { std::slice::from_raw_parts_mut(index.as_mut_ptr() as *mut u8, index_bytes) })?;

        // Read and decompress the xattr id table
        let bytes = (ids as usize * 16 + 8191) & !8191;
        self.xattr_ids = vec![SquashfsXattrId { xattr: 0, count: 0 }; ids as usize];

        for i in 0..indexes {
            let expected = if (i + 1) != indexes { SQUASHFS_METADATA_SIZE } else { bytes & (SQUASHFS_METADATA_SIZE - 1) };
            let offset = i * SQUASHFS_METADATA_SIZE;
            let mut block = vec![0u8; expected];
            
            if read_block(reader, index[i], None, Some(expected), &mut block)? == 0 {
                return Err(SquashError::InvalidXattrs);
            }

            unsafe {
                std::ptr::copy_nonoverlapping(
                    block.as_ptr(),
                    self.xattr_ids.as_mut_ptr().add(i * SQUASHFS_METADATA_SIZE / 16) as *mut u8,
                    expected,
                );
            }
        }

        // Read and decompress the xattr metadata
        let mut start = self.xattr_table_start;
        let end = index[0];
        let mut i = 0;

        while start < end {
            self.xattrs.resize((i + 1) * SQUASHFS_METADATA_SIZE, 0);
            self.save_xattr_block(start, (i * SQUASHFS_METADATA_SIZE) as i64)?;

            let mut block = vec![0u8; SQUASHFS_METADATA_SIZE];
            let length = read_block(reader, start, Some(&mut start), None, &mut block)?;
            
            if length == 0 {
                return Err(SquashError::InvalidXattrs);
            }

            if start != end && length != SQUASHFS_METADATA_SIZE {
                return Err(SquashError::InvalidXattrs);
            }

            unsafe {
                std::ptr::copy_nonoverlapping(
                    block.as_ptr(),
                    self.xattrs.as_mut_ptr().add(i * SQUASHFS_METADATA_SIZE),
                    length,
                );
            }

            self.xattr_table_length += length;
            i += 1;
        }

        Ok(ids)
    }

    pub fn get_xattr(&self, i: usize, count: &mut u32, failed: &mut bool) -> Result<Vec<XattrList>> {
        if i >= self.xattr_ids.len() {
            return Err(SquashError::InvalidXattrs);
        }

        let xattr_id = &self.xattr_ids[i];
        if xattr_id.count == 0 {
            *failed = true;
            *count = 0;
            return Ok(Vec::new());
        }

        *failed = false;
        let mut xattr_list = Vec::new();
        let mut xptr_offset = self.get_xattr_block(xattr_id.xattr as i64)
            .ok_or_else(|| SquashError::InvalidXattrs)?;
        
        xptr_offset += (xattr_id.xattr & 0xFFFF) as i64;
        if xptr_offset as usize + (xattr_id.xattr >> 16) as usize > self.xattr_table_length {
            return Err(SquashError::InvalidXattrs);
        }

        let mut xptr = &self.xattrs[xptr_offset as usize..];
        let mut j = 0;

        for _ in 0..xattr_id.count {
            if xptr.len() < std::mem::size_of::<SquashfsXattrEntry>() {
                return Err(SquashError::InvalidXattrs);
            }

            let entry = unsafe {
                std::ptr::read_unaligned(xptr.as_ptr() as *const SquashfsXattrEntry)
            };

            xptr = &xptr[std::mem::size_of::<SquashfsXattrEntry>()..];
            
            if xptr.len() < entry.size as usize {
                return Err(SquashError::InvalidXattrs);
            }

            let mut xattr = self.read_xattr_entry(&entry, &xptr[..entry.size as usize])?;
            xptr = &xptr[entry.size as usize..];

            if xptr.len() < std::mem::size_of::<SquashfsXattrVal>() {
                return Err(SquashError::InvalidXattrs);
            }

            let val = unsafe {
                std::ptr::read_unaligned(xptr.as_ptr() as *const SquashfsXattrVal)
            };

            xptr = &xptr[std::mem::size_of::<SquashfsXattrVal>()..];

            if entry.type_ & SQUASHFS_XATTR_VALUE_OOL != 0 {
                if xptr.len() < std::mem::size_of::<i64>() {
                    return Err(SquashError::InvalidXattrs);
                }

                let xattr = unsafe {
                    std::ptr::read_unaligned(xptr.as_ptr() as *const i64)
                };

                xptr = &xptr[std::mem::size_of::<i64>()..];

                let start = (xattr >> 16) as i64 + self.xattr_table_start;
                let offset = (xattr & 0xFFFF) as i64;
                let ool_xptr_offset = self.get_xattr_block(start)
                    .ok_or_else(|| SquashError::InvalidXattrs)?;
                
                let ool_xptr = &self.xattrs[ool_xptr_offset as usize + offset as usize..];
                let ool_val = unsafe {
                    std::ptr::read_unaligned(ool_xptr.as_ptr() as *const SquashfsXattrVal)
                };

                xattr.value = ool_xptr[std::mem::size_of::<SquashfsXattrVal>()..][..ool_val.vsize as usize].to_vec();
            } else {
                if xptr.len() < val.vsize as usize {
                    return Err(SquashError::InvalidXattrs);
                }

                xattr.value = xptr[..val.vsize as usize].to_vec();
                xptr = &xptr[val.vsize as usize..];
            }

            xattr.vsize = val.vsize as usize;
            xattr_list.push(xattr);
            j += 1;
        }

        *count = j;
        Ok(xattr_list)
    }
}

#[repr(C)]
#[derive(Debug)]
struct SquashfsXattrVal {
    vsize: u32,
}

pub fn free_xattr(xattr_list: &[XattrList]) {
    // No need to free in Rust as memory is managed automatically
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xattr_table_creation() {
        let table = XattrTable::new();
        assert!(table.xattr_ids.is_empty());
        assert!(table.xattrs.is_empty());
        assert_eq!(table.xattr_table_start, 0);
        assert_eq!(table.xattr_table_length, 0);
        assert!(table.hash_table.is_empty());
    }

    #[test]
    fn test_xattr_block_hash() {
        let mut table = XattrTable::new();
        table.save_xattr_block(100, 200).unwrap();
        assert_eq!(table.get_xattr_block(100), Some(200));
        assert_eq!(table.get_xattr_block(101), None);
    }
}

/// Constants for xattr handling
pub const XATTR_VALUE_OOL: i32 = 0x100;
pub const XATTR_PREFIX_MASK: i32 = 0xff;
pub const XATTR_VALUE_OOL_SIZE: usize = std::mem::size_of::<i64>();
pub const XATTR_INLINE_MAX: usize = 128;
pub const XATTR_TARGET_MAX: usize = 65536;

/// Format prefixes for xattr values
pub const PREFIX_BASE64_0S: u16 = 0x3000 + 0x53;
pub const PREFIX_BASE64_0s: u16 = 0x3000 + 0x73;
pub const PREFIX_BINARY_0B: u16 = 0x3000 + 0x42;
pub const PREFIX_BINARY_0b: u16 = 0x3000 + 0x62;
pub const PREFIX_HEX_0X: u16 = 0x3000 + 0x58;
pub const PREFIX_HEX_0x: u16 = 0x3000 + 0x78;
pub const PREFIX_TEXT_0T: u16 = 0x3000 + 0x54;
pub const PREFIX_TEXT_0t: u16 = 0x3000 + 0x74;

/// Structure for xattr prefix information
#[derive(Debug, Clone)]
pub struct Prefix {
    pub prefix: &'static str,
    pub type_: i32,
}

/// Structure for xattr list entries
#[derive(Debug, Clone)]
pub struct XattrList {
    pub name: String,
    pub full_name: String,
    pub size: usize,
    pub vsize: usize,
    pub value: Vec<u8>,
    pub type_: i32,
    pub ool_value: i64,
    pub vchecksum: u16,
    pub vnext: Option<Box<XattrList>>,
}

impl XattrList {
    pub fn new(name: &str) -> Result<Self> {
        let type_ = xattr_get_type(name);
        let (full_name, name, size) = if type_ != -1 {
            let prefix = &prefix_table[type_ as usize];
            let name = &name[prefix.prefix.len()..];
            (name.to_string(), name.to_string(), name.len())
        } else {
            (name.to_string(), name.to_string(), name.len())
        };

        Ok(Self {
            name,
            full_name,
            size,
            vsize: 0,
            value: Vec::new(),
            type_,
            ool_value: -1,
            vchecksum: 0,
            vnext: None,
        })
    }
}

/// Structure for duplicate ID entries
#[derive(Debug)]
pub struct DuplId {
    pub xattr_list: Vec<XattrList>,
    pub xattrs: usize,
    pub xattr_id: i32,
    pub next: Option<Box<DuplId>>,
}

/// Structure for xattr add entries
#[derive(Debug)]
pub struct XattrAdd {
    pub name: String,
    pub value: Vec<u8>,
    pub vsize: usize,
    pub type_: i32,
    pub next: Option<Box<XattrAdd>>,
}

/// Global state for xattr handling
pub struct XattrState {
    pub xattr_table: Vec<u8>,
    pub xattr_size: usize,
    pub data_cache: Vec<u8>,
    pub cache_bytes: usize,
    pub cache_size: usize,
    pub xattr_id_table: Vec<SquashfsXattrId>,
    pub xattr_ids: usize,
    pub dupl_value: HashMap<u16, Vec<XattrList>>,
    pub dupl_id: HashMap<u16, Vec<DuplId>>,
    pub xattr_add_list: Option<Box<XattrAdd>>,
    pub xattr_add_count: usize,
}

impl XattrState {
    pub fn new() -> Self {
        Self {
            xattr_table: Vec::new(),
            xattr_size: 0,
            data_cache: Vec::new(),
            cache_bytes: 0,
            cache_size: 0,
            xattr_id_table: Vec::new(),
            xattr_ids: 0,
            dupl_value: HashMap::new(),
            dupl_id: HashMap::new(),
            xattr_add_list: None,
            xattr_add_count: 0,
        }
    }
}

/// Get the type of an xattr based on its prefix
fn xattr_get_type(name: &str) -> i32 {
    for (i, prefix) in prefix_table.iter().enumerate() {
        if name.starts_with(prefix.prefix) {
            return i as i32;
        }
    }
    -1
}

/// Parse a base64 encoded string
pub fn base64_decode(source: &str) -> Result<Vec<u8>> {
    let mut dest = Vec::new();
    let mut bit_pos = 0;
    let mut output = 0;
    let mut source = source.as_bytes();

    // Handle padding
    if source.len() % 4 == 0 {
        if source.len() >= 2 {
            if source[source.len() - 2] == b'=' && source[source.len() - 1] == b'=' {
                source = &source[..source.len() - 2];
            } else if source[source.len() - 1] == b'=' {
                source = &source[..source.len() - 1];
            }
        }
    }

    // Calculate output size
    let count = source.len() * 3 / 4;
    dest.reserve(count);

    for &byte in source {
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return Err(Error::XattrError("Invalid base64 character".to_string())),
        };

        if bit_pos == 24 {
            dest.push((output >> 16) as u8);
            dest.push(((output >> 8) & 0xff) as u8);
            dest.push((output & 0xff) as u8);
            bit_pos = 0;
            output = 0;
        }

        output = (output << 6) | value as i32;
        bit_pos += 6;
    }

    output = output << (24 - bit_pos);

    if bit_pos == 6 {
        return Err(Error::XattrError("Invalid base64 padding".to_string()));
    }

    if bit_pos >= 12 {
        dest.push((output >> 16) as u8);
    }
    if bit_pos >= 18 {
        dest.push(((output >> 8) & 0xff) as u8);
    }
    if bit_pos == 24 {
        dest.push((output & 0xff) as u8);
    }

    Ok(dest)
}

/// Parse a hex encoded string
fn hex_decode(source: &str) -> Result<Vec<u8>> {
    if source.len() % 2 != 0 {
        return Err(Error::XattrError("Invalid hex string length".to_string()));
    }

    let mut dest = Vec::with_capacity(source.len() / 2);
    let mut chars = source.chars();

    while let (Some(c1), Some(c2)) = (chars.next(), chars.next()) {
        let digit1 = match c1 {
            '0'..='9' => c1 as u8 - b'0',
            'A'..='F' => c1 as u8 - b'A' + 10,
            'a'..='f' => c1 as u8 - b'a' + 10,
            _ => return Err(Error::XattrError("Invalid hex character".to_string())),
        };

        let digit2 = match c2 {
            '0'..='9' => c2 as u8 - b'0',
            'A'..='F' => c2 as u8 - b'A' + 10,
            'a'..='f' => c2 as u8 - b'a' + 10,
            _ => return Err(Error::XattrError("Invalid hex character".to_string())),
        };

        dest.push((digit1 << 4) | digit2);
    }

    Ok(dest)
}

/// Parse a text encoded string with octal escapes
fn text_decode(source: &str) -> Result<Vec<u8>> {
    let mut dest = Vec::new();
    let mut chars = source.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            let mut octal = String::with_capacity(3);
            for _ in 0..3 {
                if let Some(&c) = chars.peek() {
                    if c.is_ascii_digit() && c < '8' {
                        octal.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }

            if octal.len() == 3 {
                let value = u8::from_str_radix(&octal, 8)
                    .map_err(|e| Error::XattrError(format!("Invalid octal value: {}", e)))?;
                dest.push(value);
            } else {
                return Err(Error::XattrError("Invalid octal escape".to_string()));
            }
        } else {
            dest.push(c as u8);
        }
    }

    Ok(dest)
}

/// Parse an xattr string
pub fn xattr_parse(str: &str, pre: &str, option: &str) -> Result<XattrAdd> {
    let parts: Vec<&str> = str.splitn(2, '=').collect();
    if parts.len() != 2 {
        return Err(Error::XattrError(format!(
            "{}invalid argument \"{}\" in {} option, because no `=` found",
            pre, str, option
        )));
    }

    let (name, value) = (parts[0], parts[1]);
    if name.is_empty() {
        return Err(Error::XattrError(format!(
            "{}invalid argument \"{}\" in {} option, because xattr name is empty",
            pre, str, option
        )));
    }

    if value.is_empty() {
        return Err(Error::XattrError(format!(
            "{}invalid argument \"{}\" in {} option, because xattr value is empty",
            pre, str, option
        )));
    }

    let type_ = xattr_get_type(name);
    if type_ == -1 {
        return Err(Error::XattrError(format!(
            "{}{}: unrecognised xattr prefix in {}",
            pre, option, name
        )));
    }

    let (value, vsize) = if value.len() >= 2 {
        let prefix = (value.as_bytes()[0] as u16) << 8 | value.as_bytes()[1] as u16;
        match prefix {
            PREFIX_BASE64_0S | PREFIX_BASE64_0s => {
                let value = &value[2..];
                if value.is_empty() {
                    return Err(Error::XattrError(format!(
                        "{}invalid argument {} in {} option, because xattr value is empty after format prefix 0S or 0s",
                        pre, str, option
                    )));
                }
                (base64_decode(value)?, value.len())
            }
            PREFIX_HEX_0X | PREFIX_HEX_0x => {
                let value = &value[2..];
                if value.is_empty() {
                    return Err(Error::XattrError(format!(
                        "{}invalid argument {} in {} option, because xattr value is empty after format prefix 0X or 0x",
                        pre, str, option
                    )));
                }
                (hex_decode(value)?, value.len())
            }
            PREFIX_TEXT_0T | PREFIX_TEXT_0t => {
                let value = &value[2..];
                if value.is_empty() {
                    return Err(Error::XattrError(format!(
                        "{}invalid argument {} in {} option, because xattr value is empty after format prefix 0T or 0t",
                        pre, str, option
                    )));
                }
                (text_decode(value)?, value.len())
            }
            PREFIX_BINARY_0B | PREFIX_BINARY_0b => {
                let value = &value[2..];
                if value.is_empty() {
                    return Err(Error::XattrError(format!(
                        "{}invalid argument {} in {} option, because xattr value is empty after format prefix 0B or 0b",
                        pre, str, option
                    )));
                }
                (value.as_bytes().to_vec(), value.len())
            }
            _ => (value.as_bytes().to_vec(), value.len()),
        }
    } else {
        (value.as_bytes().to_vec(), value.len())
    };

    Ok(XattrAdd {
        name: name.to_string(),
        value,
        vsize,
        type_,
        next: None,
    })
}

/// Add an xattr to the list
pub fn xattrs_add(state: &mut XattrState, str: &str) -> Result<()> {
    let entry = xattr_parse(str, "FATAL ERROR: ", "xattrs-add")?;
    let mut new_entry = Box::new(entry);
    new_entry.next = state.xattr_add_list.take();
    state.xattr_add_list = Some(new_entry);
    state.xattr_add_count += 1;
    Ok(())
}

/// Get the number of xattrs
pub fn add_xattrs(state: &XattrState) -> usize {
    state.xattr_add_count
}

/// Sort the xattr add list
pub fn sort_xattr_add_list(state: &mut XattrState) {
    // TODO: Implement merge sort for xattr add list
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xattr_parse() {
        let entry = xattr_parse("user.test=value", "", "test").unwrap();
        assert_eq!(entry.name, "user.test");
        assert_eq!(entry.value, b"value");
        assert_eq!(entry.vsize, 5);
        assert_eq!(entry.type_, 0);
    }

    #[test]
    fn test_base64_decode() {
        let value = base64_decode("SGVsbG8=").unwrap();
        assert_eq!(value, b"Hello");
    }

    #[test]
    fn test_hex_decode() {
        let value = hex_decode("48656c6c6f").unwrap();
        assert_eq!(value, b"Hello");
    }

    #[test]
    fn test_text_decode() {
        let value = text_decode("Hello\\040World").unwrap();
        assert_eq!(value, b"Hello World");
    }

    #[test]
    fn test_xattrs_add() {
        let mut state = XattrState::new();
        xattrs_add(&mut state, "user.test=value").unwrap();
        assert_eq!(add_xattrs(&state), 1);
    }
} 