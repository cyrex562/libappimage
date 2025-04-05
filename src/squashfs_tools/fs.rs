use crate::error::SquashError;
use std::convert::TryFrom;
use std::mem;

// Constants
pub const SQUASHFS_CACHED_FRAGMENTS: usize = 8; // CONFIG_SQUASHFS_FRAGMENT_CACHE_SIZE
pub const SQUASHFS_MAJOR: u16 = 4;
pub const SQUASHFS_MINOR: u16 = 0;
pub const SQUASHFS_MAGIC: u32 = 0x73717368;
pub const SQUASHFS_MAGIC_SWAP: u32 = 0x68737173;
pub const SQUASHFS_START: u32 = 0;
pub const SQUASHFS_METADATA_SIZE: usize = 8192;
pub const SQUASHFS_METADATA_LOG: u32 = 13;
pub const SQUASHFS_FILE_SIZE: usize = 131072;
pub const SQUASHFS_FILE_MAX_SIZE: usize = 1048576;
pub const SQUASHFS_FILE_MAX_LOG: u32 = 20;

pub const SQUASHFS_IDS: u32 = 65536;
pub const SQUASHFS_NAME_LEN: usize = 256;
pub const SQUASHFS_DIR_COUNT: u32 = 256;
pub const SQUASHFS_SYMLINK_MAX: u32 = 65535;

pub const SQUASHFS_INVALID: i64 = 0xffffffffffff;
pub const SQUASHFS_INVALID_FRAG: u32 = 0xffffffff;
pub const SQUASHFS_INVALID_XATTR: u32 = 0xffffffff;
pub const SQUASHFS_INVALID_BLK: i64 = -1;
pub const SQUASHFS_USED_BLK: i64 = -2;

// Metadata and block size constants
// Filesystem flags
#[derive(Debug, Clone, Copy)]
pub struct SquashFsFlags(u16);

impl SquashFsFlags {
    pub const NOI: u16 = 0;
    pub const NOD: u16 = 1;
    pub const CHECK: u16 = 2;
    pub const NOF: u16 = 3;
    pub const NO_FRAG: u16 = 4;
    pub const ALWAYS_FRAG: u16 = 5;
    pub const DUPLICATE: u16 = 6;
    pub const EXPORT: u16 = 7;
    pub const NOX: u16 = 8;
    pub const NO_XATTR: u16 = 9;
    pub const COMP_OPT: u16 = 10;
    pub const NOID: u16 = 11;

    pub fn new(flags: u16) -> Self {
        Self(flags)
    }

    pub fn uncompressed_inodes(&self) -> bool {
        (self.0 >> Self::NOI) & 1 == 1
    }

    pub fn uncompressed_data(&self) -> bool {
        (self.0 >> Self::NOD) & 1 == 1
    }

    pub fn uncompressed_fragments(&self) -> bool {
        (self.0 >> Self::NOF) & 1 == 1
    }

    pub fn no_fragments(&self) -> bool {
        (self.0 >> Self::NO_FRAG) & 1 == 1
    }

    pub fn always_fragments(&self) -> bool {
        (self.0 >> Self::ALWAYS_FRAG) & 1 == 1
    }

    pub fn duplicates(&self) -> bool {
        (self.0 >> Self::DUPLICATE) & 1 == 1
    }

    pub fn exportable(&self) -> bool {
        (self.0 >> Self::EXPORT) & 1 == 1
    }

    pub fn uncompressed_xattrs(&self) -> bool {
        (self.0 >> Self::NOX) & 1 == 1
    }

    pub fn no_xattrs(&self) -> bool {
        (self.0 >> Self::NO_XATTR) & 1 == 1
    }

    pub fn comp_opts(&self) -> bool {
        (self.0 >> Self::COMP_OPT) & 1 == 1
    }

    pub fn uncompressed_ids(&self) -> bool {
        (self.0 >> Self::NOID) & 1 == 1
    }
}

// File types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SquashFsFileType {
    Dir = 1,
    File = 2,
    Symlink = 3,
    BlkDev = 4,
    ChrDev = 5,
    Fifo = 6,
    Socket = 7,
    LDir = 8,
    LReg = 9,
    LSymlink = 10,
    LBlkDev = 11,
    LChrDev = 12,
    LFifo = 13,
    LSocket = 14,
}

impl TryFrom<u16> for SquashFsFileType {
    type Error = &'static str;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Dir),
            2 => Ok(Self::File),
            3 => Ok(Self::Symlink),
            4 => Ok(Self::BlkDev),
            5 => Ok(Self::ChrDev),
            6 => Ok(Self::Fifo),
            7 => Ok(Self::Socket),
            8 => Ok(Self::LDir),
            9 => Ok(Self::LReg),
            10 => Ok(Self::LSymlink),
            11 => Ok(Self::LBlkDev),
            12 => Ok(Self::LChrDev),
            13 => Ok(Self::LFifo),
            14 => Ok(Self::LSocket),
            _ => Err("Invalid SquashFS file type"),
        }
    }
}

// Compression types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CompressionType {
    Zlib = 1,
    Lzma = 2,
    Lzo = 3,
    Xz = 4,
    Lz4 = 5,
    Zstd = 6,
}

impl TryFrom<u16> for CompressionType {
    type Error = &'static str;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Zlib),
            2 => Ok(Self::Lzma),
            3 => Ok(Self::Lzo),
            4 => Ok(Self::Xz),
            5 => Ok(Self::Lz4),
            6 => Ok(Self::Zstd),
            _ => Err("Invalid compression type"),
        }
    }
}

// On-disk structures
#[repr(C)]
#[derive(Debug)]
pub struct SquashFsSuperBlock {
    pub magic: u32,
    pub inodes: u32,
    pub mkfs_time: u32,
    pub block_size: u32,
    pub fragments: u32,
    pub compression: u16,
    pub block_log: u16,
    pub flags: u16,
    pub no_ids: u16,
    pub s_major: u16,
    pub s_minor: u16,
    pub root_inode: i64,
    pub bytes_used: i64,
    pub id_table_start: i64,
    pub xattr_id_table_start: i64,
    pub inode_table_start: i64,
    pub directory_table_start: i64,
    pub fragment_table_start: i64,
    pub lookup_table_start: i64,
}

#[repr(C)]
#[derive(Debug)]
pub struct SquashFsDirIndex {
    pub index: u32,
    pub start_block: u32,
    pub size: u32,
    pub name: [u8; 0], // Flexible array member
}

#[repr(C)]
#[derive(Debug)]
pub struct SquashFsBaseInodeHeader {
    pub inode_type: u16,
    pub mode: u16,
    pub uid: u16,
    pub guid: u16,
    pub mtime: u32,
    pub inode_number: u32,
}

#[repr(C)]
#[derive(Debug)]
pub struct SquashFsIpcInodeHeader {
    pub base: SquashFsBaseInodeHeader,
    pub nlink: u32,
}

#[repr(C)]
#[derive(Debug)]
pub struct SquashFsLIpcInodeHeader {
    pub base: SquashFsIpcInodeHeader,
    pub xattr: u32,
}

#[repr(C)]
#[derive(Debug)]
pub struct SquashFsDevInodeHeader {
    pub base: SquashFsBaseInodeHeader,
    pub nlink: u32,
    pub rdev: u32,
}

#[repr(C)]
#[derive(Debug)]
pub struct SquashFsLDevInodeHeader {
    pub base: SquashFsDevInodeHeader,
    pub xattr: u32,
}

#[repr(C)]
#[derive(Debug)]
pub struct SquashFsSymlinkInodeHeader {
    pub base: SquashFsBaseInodeHeader,
    pub nlink: u32,
    pub symlink_size: u32,
    pub symlink: [u8; 0], // Flexible array member
}

#[repr(C)]
#[derive(Debug)]
pub struct SquashFsRegInodeHeader {
    pub base: SquashFsBaseInodeHeader,
    pub start_block: u32,
    pub fragment: u32,
    pub offset: u32,
    pub file_size: u32,
    pub block_list: [u32; 0], // Flexible array member
}

#[repr(C)]
#[derive(Debug)]
pub struct SquashFsLRegInodeHeader {
    pub base: SquashFsBaseInodeHeader,
    pub start_block: i64,
    pub file_size: i64,
    pub sparse: i64,
    pub nlink: u32,
    pub fragment: u32,
    pub offset: u32,
    pub xattr: u32,
    pub block_list: [u32; 0], // Flexible array member
}

#[repr(C)]
#[derive(Debug)]
pub struct SquashFsDirInodeHeader {
    pub base: SquashFsBaseInodeHeader,
    pub start_block: u32,
    pub nlink: u32,
    pub file_size: u16,
    pub offset: u16,
    pub parent_inode: u32,
}

#[repr(C)]
#[derive(Debug)]
pub struct SquashFsLDirInodeHeader {
    pub base: SquashFsBaseInodeHeader,
    pub nlink: u32,
    pub file_size: u32,
    pub start_block: u32,
    pub parent_inode: u32,
    pub i_count: u16,
    pub offset: u16,
    pub xattr: u32,
    pub index: [SquashFsDirIndex; 0], // Flexible array member
}

#[repr(C)]
#[derive(Debug)]
pub struct SquashFsDirEntry {
    pub offset: u16,
    pub inode_number: i16,
    pub type_: u16,
    pub size: u16,
    pub name: [u8; 0], // Flexible array member
}

#[repr(C)]
#[derive(Debug)]
pub struct SquashFsDirHeader {
    pub count: u32,
    pub start_block: u32,
    pub inode_number: u32,
}

#[repr(C)]
#[derive(Debug)]
pub struct SquashFsFragmentEntry {
    pub start_block: i64,
    pub size: u32,
    pub unused: u32,
}

#[repr(C)]
#[derive(Debug)]
pub struct SquashFsXattrEntry {
    pub type_: u16,
    pub size: u16,
}

#[repr(C)]
#[derive(Debug)]
pub struct SquashFsXattrVal {
    pub vsize: u32,
}

#[repr(C)]
#[derive(Debug)]
pub struct SquashFsXattrId {
    pub xattr: i64,
    pub count: u32,
    pub size: u32,
}

#[repr(C)]
#[derive(Debug)]
pub struct SquashFsXattrTable {
    pub xattr_table_start: i64,
    pub xattr_ids: u32,
    pub unused: u32,
}

// Helper functions
pub fn squashfs_mk_vfs_inode(a: u32, b: u32) -> u32 {
    ((a << 8) + (b >> 2) + 1)
}

pub fn squashfs_mode(mode: u16) -> u16 {
    mode & 0xfff
}

pub fn squashfs_compressed_size(b: u32) -> u32 {
    if (b & !0x8000) != 0 {
        b & !0x8000
    } else {
        0x8000
    }
}

pub fn squashfs_compressed(b: u32) -> bool {
    (b & 0x8000) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_squashfs_flags() {
        let flags = SquashFsFlags::new(0b000000000001);
        assert!(flags.uncompressed_inodes());
        assert!(!flags.uncompressed_data());
        assert!(!flags.uncompressed_fragments());
    }

    #[test]
    fn test_file_type_conversion() {
        assert_eq!(
            SquashFsFileType::try_from(1).unwrap(),
            SquashFsFileType::Dir
        );
        assert_eq!(
            SquashFsFileType::try_from(2).unwrap(),
            SquashFsFileType::File
        );
        assert!(SquashFsFileType::try_from(15).is_err());
    }

    #[test]
    fn test_compression_type_conversion() {
        assert_eq!(CompressionType::try_from(1).unwrap(), CompressionType::Zlib);
        assert_eq!(CompressionType::try_from(6).unwrap(), CompressionType::Zstd);
        assert!(CompressionType::try_from(7).is_err());
    }

    #[test]
    fn test_helper_functions() {
        assert_eq!(squashfs_mk_vfs_inode(1, 2), 258);
        assert_eq!(squashfs_mode(0x1fff), 0xfff);
        assert_eq!(squashfs_compressed_size(0x8001), 1);
        assert!(squashfs_compressed(0x8001));
        assert!(!squashfs_compressed(0x0001));
    }
}

impl SquashFsSuperBlock {
    pub fn is_valid(&self) -> bool {
        self.magic == 0x73717368 // SQUASHFS_MAGIC
    }

    pub fn is_big_endian(&self) -> bool {
        self.magic == 0x68737173 // SQUASHFS_MAGIC_SWAP
    }

    pub fn is_version_supported(&self) -> bool {
        self.s_major == 4 && self.s_minor <= 0
    }
}

impl SquashFsDirEntry {
    pub fn name(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.name.as_ptr(), self.size as usize) }
    }
}

impl SquashFsXattrEntry {
    pub fn value(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.value.as_ptr(), self.size as usize) }
    }
}

pub type SquashFsInode = u64;

pub fn squashfs_mkinode(start_block: u32, offset: u16) -> SquashFsInode {
    ((start_block as u64) << 16) | (offset as u64)
}

pub fn squashfs_inode_offset(inode: SquashFsInode) -> u16 {
    (inode & 0xFFFF) as u16
}

pub fn squashfs_inode_block(inode: SquashFsInode) -> u32 {
    (inode >> 16) as u32
}
