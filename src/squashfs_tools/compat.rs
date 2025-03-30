use std::mem;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use crate::error::{SquashError, SquashResult as Result};

// Constants
pub const SQUASHFS_CHECK: u8 = 2;
pub const SQUASHFS_UIDS: u16 = 256;
pub const SQUASHFS_GUIDS: u16 = 255;
pub const SQUASHFS_TYPES: u8 = 5;
pub const SQUASHFS_IPC_TYPE: u8 = 0;
pub const SQUASHFS_METADATA_SIZE: usize = 8192;

// Helper macro for bit operations
macro_rules! SQUASHFS_BIT {
    ($flags:expr, $bit:expr) => {
        (($flags) & (1 << ($bit))) != 0
    };
}

// Helper macro for checking data flags
macro_rules! SQUASHFS_CHECK_DATA {
    ($flags:expr) => {
        SQUASHFS_BIT!($flags, SQUASHFS_CHECK)
    };
}

// Version 3.x structures
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SquashfsSuperBlock3 {
    pub s_magic: u32,
    pub inodes: u32,
    pub bytes_used_2: u32,
    pub uid_start_2: u32,
    pub guid_start_2: u32,
    pub inode_table_start_2: u32,
    pub directory_table_start_2: u32,
    pub s_major: u16,
    pub s_minor: u16,
    pub block_size_1: u16,
    pub block_log: u16,
    pub flags: u8,
    pub no_uids: u8,
    pub no_guids: u8,
    pub mkfs_time: i32,
    pub root_inode: u64,
    pub block_size: u32,
    pub fragments: u32,
    pub fragment_table_start_2: u32,
    pub bytes_used: i64,
    pub uid_start: i64,
    pub guid_start: i64,
    pub inode_table_start: i64,
    pub directory_table_start: i64,
    pub fragment_table_start: i64,
    pub lookup_table_start: i64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SquashfsDirIndex3 {
    pub index: u32,
    pub start_block: u32,
    pub size: u8,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SquashfsBaseInodeHeader3 {
    pub inode_type: u8,
    pub mode: u16,
    pub uid: u8,
    pub guid: u8,
    pub mtime: i32,
    pub inode_number: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SquashfsIpcInodeHeader3 {
    pub base: SquashfsBaseInodeHeader3,
    pub nlink: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SquashfsDevInodeHeader3 {
    pub base: SquashfsBaseInodeHeader3,
    pub nlink: u32,
    pub rdev: u16,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SquashfsSymlinkInodeHeader3 {
    pub base: SquashfsBaseInodeHeader3,
    pub nlink: u32,
    pub symlink_size: u16,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SquashfsRegInodeHeader3 {
    pub base: SquashfsBaseInodeHeader3,
    pub start_block: u64,
    pub fragment: u32,
    pub offset: u32,
    pub file_size: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SquashfsLregInodeHeader3 {
    pub base: SquashfsBaseInodeHeader3,
    pub nlink: u32,
    pub start_block: u64,
    pub fragment: u32,
    pub offset: u32,
    pub file_size: i64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SquashfsDirInodeHeader3 {
    pub base: SquashfsBaseInodeHeader3,
    pub nlink: u32,
    pub file_size: u32,
    pub offset: u32,
    pub start_block: u32,
    pub parent_inode: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SquashfsLdirInodeHeader3 {
    pub base: SquashfsBaseInodeHeader3,
    pub nlink: u32,
    pub start_block: u32,
    pub fragment: u32,
    pub offset: u32,
    pub file_size: i64,
    pub i_count: u16,
    pub parent_inode: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SquashfsDirEntry3 {
    pub offset: u16,
    pub type_: u8,
    pub size: u8,
    pub inode_number: i16,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SquashfsDirHeader3 {
    pub count: u8,
    pub start_block: u32,
    pub inode_number: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SquashfsFragmentEntry3 {
    pub start_block: i64,
    pub size: u32,
    pub pending: u32,
}

// Version 2.x structures
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SquashfsDirIndex2 {
    pub index: u32,
    pub start_block: u32,
    pub size: u8,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SquashfsBaseInodeHeader2 {
    pub inode_type: u8,
    pub mode: u16,
    pub uid: u8,
    pub guid: u8,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SquashfsIpcInodeHeader2 {
    pub base: SquashfsBaseInodeHeader2,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SquashfsDevInodeHeader2 {
    pub base: SquashfsBaseInodeHeader2,
    pub rdev: u16,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SquashfsSymlinkInodeHeader2 {
    pub base: SquashfsBaseInodeHeader2,
    pub symlink_size: u16,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SquashfsRegInodeHeader2 {
    pub base: SquashfsBaseInodeHeader2,
    pub mtime: i32,
    pub start_block: u32,
    pub fragment: u32,
    pub offset: u32,
    pub file_size: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SquashfsDirInodeHeader2 {
    pub base: SquashfsBaseInodeHeader2,
    pub file_size: u32,
    pub offset: u32,
    pub mtime: i32,
    pub start_block: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SquashfsLdirInodeHeader2 {
    pub base: SquashfsBaseInodeHeader2,
    pub file_size: u32,
    pub offset: u32,
    pub mtime: i32,
    pub start_block: u32,
    pub i_count: u16,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SquashfsDirHeader2 {
    pub count: u8,
    pub start_block: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SquashfsDirEntry2 {
    pub offset: u16,
    pub type_: u8,
    pub size: u8,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SquashfsFragmentEntry2 {
    pub start_block: u32,
    pub size: u32,
}

// Helper functions for endianness conversion
pub fn swap_super_block_3(s: &mut SquashfsSuperBlock3) {
    s.s_magic = s.s_magic.to_le();
    s.inodes = s.inodes.to_le();
    s.bytes_used_2 = s.bytes_used_2.to_le();
    s.uid_start_2 = s.uid_start_2.to_le();
    s.guid_start_2 = s.guid_start_2.to_le();
    s.inode_table_start_2 = s.inode_table_start_2.to_le();
    s.directory_table_start_2 = s.directory_table_start_2.to_le();
    s.s_major = s.s_major.to_le();
    s.s_minor = s.s_minor.to_le();
    s.block_size_1 = s.block_size_1.to_le();
    s.block_log = s.block_log.to_le();
    s.mkfs_time = s.mkfs_time.to_le();
    s.root_inode = s.root_inode.to_le();
    s.block_size = s.block_size.to_le();
    s.fragments = s.fragments.to_le();
    s.fragment_table_start_2 = s.fragment_table_start_2.to_le();
    s.bytes_used = s.bytes_used.to_le();
    s.uid_start = s.uid_start.to_le();
    s.guid_start = s.guid_start.to_le();
    s.inode_table_start = s.inode_table_start.to_le();
    s.directory_table_start = s.directory_table_start.to_le();
    s.fragment_table_start = s.fragment_table_start.to_le();
    s.lookup_table_start = s.lookup_table_start.to_le();
}

// Helper functions for calculating sizes and offsets
pub fn squashfs_fragment_bytes_3(fragments: u32) -> usize {
    fragments as usize * mem::size_of::<SquashfsFragmentEntry3>()
}

pub fn squashfs_fragment_index_3(fragments: u32) -> usize {
    squashfs_fragment_bytes_3(fragments) / SQUASHFS_METADATA_SIZE
}

pub fn squashfs_fragment_index_offset_3(fragments: u32) -> usize {
    squashfs_fragment_bytes_3(fragments) % SQUASHFS_METADATA_SIZE
}

pub fn squashfs_fragment_indexes_3(fragments: u32) -> usize {
    (squashfs_fragment_bytes_3(fragments) + SQUASHFS_METADATA_SIZE - 1) / SQUASHFS_METADATA_SIZE
}

pub fn squashfs_fragment_index_bytes_3(fragments: u32) -> usize {
    squashfs_fragment_indexes_3(fragments) * mem::size_of::<i64>()
}

// Helper functions for lookup table calculations
pub fn squashfs_lookup_bytes_3(inodes: u32) -> usize {
    inodes as usize * mem::size_of::<u64>()
}

pub fn squashfs_lookup_block_3(inodes: u32) -> usize {
    squashfs_lookup_bytes_3(inodes) / SQUASHFS_METADATA_SIZE
}

pub fn squashfs_lookup_block_offset_3(inodes: u32) -> usize {
    squashfs_lookup_bytes_3(inodes) % SQUASHFS_METADATA_SIZE
}

pub fn squashfs_lookup_blocks_3(inodes: u32) -> usize {
    (squashfs_lookup_bytes_3(inodes) + SQUASHFS_METADATA_SIZE - 1) / SQUASHFS_METADATA_SIZE
}

pub fn squashfs_lookup_block_bytes_3(inodes: u32) -> usize {
    squashfs_lookup_blocks_3(inodes) * mem::size_of::<i64>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_super_block_swap() {
        let mut sb = SquashfsSuperBlock3 {
            s_magic: 0x12345678,
            inodes: 0x87654321,
            bytes_used_2: 0x11223344,
            uid_start_2: 0x44332211,
            guid_start_2: 0x55667788,
            inode_table_start_2: 0x88776655,
            directory_table_start_2: 0x99aabbcc,
            s_major: 0xccdd,
            s_minor: 0xeeff,
            block_size_1: 0x1122,
            block_log: 0x3344,
            flags: 0x55,
            no_uids: 0x66,
            no_guids: 0x77,
            mkfs_time: 0x8899aabb,
            root_inode: 0x1122334455667788,
            block_size: 0x99aabbcc,
            fragments: 0xddffeeff,
            fragment_table_start_2: 0x11223344,
            bytes_used: 0x5566778899aabbcc,
            uid_start: 0x1122334455667788,
            guid_start: 0x99aabbccddeeff00,
            inode_table_start: 0x1122334455667788,
            directory_table_start: 0x99aabbccddeeff00,
            fragment_table_start: 0x1122334455667788,
            lookup_table_start: 0x99aabbccddeeff00,
        };

        swap_super_block_3(&mut sb);
        assert_eq!(sb.s_magic, 0x78563412);
        assert_eq!(sb.inodes, 0x21436587);
    }

    #[test]
    fn test_fragment_calculations() {
        let fragments = 100;
        assert_eq!(squashfs_fragment_bytes_3(fragments), 1200);
        assert_eq!(squashfs_fragment_index_3(fragments), 0);
        assert_eq!(squashfs_fragment_index_offset_3(fragments), 1200);
    }

    #[test]
    fn test_lookup_calculations() {
        let inodes = 100;
        assert_eq!(squashfs_lookup_bytes_3(inodes), 800);
        assert_eq!(squashfs_lookup_block_3(inodes), 0);
        assert_eq!(squashfs_lookup_block_offset_3(inodes), 800);
    }
} 