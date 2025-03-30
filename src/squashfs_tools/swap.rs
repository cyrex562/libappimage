use std::mem;
use crate::squashfs_tools::endian::{self, is_big_endian};
use crate::squashfs_tools::fs::*;
use byteorder::{ByteOrder, LittleEndian, BigEndian};

/// Trait for types that can be swapped between endianness
pub trait EndianSwap {
    /// Swap the endianness of this value
    fn swap(&self) -> Self;
    
    /// Swap the endianness of this value in-place
    fn swap_in_place(&mut self);
    
    /// Swap a slice of values between endianness
    fn swap_slice(slice: &[Self]) -> Vec<Self> where Self: Sized;
    
    /// Swap a slice of values in-place between endianness
    fn swap_slice_in_place(slice: &mut [Self]) where Self: Sized;
}

impl EndianSwap for u16 {
    fn swap(&self) -> Self {
        if is_big_endian() {
            endian::inswap_le16(*self)
        } else {
            *self
        }
    }
    
    fn swap_in_place(&mut self) {
        if is_big_endian() {
            *self = endian::inswap_le16(*self);
        }
    }
    
    fn swap_slice(slice: &[Self]) -> Vec<Self> {
        let mut result = slice.to_vec();
        Self::swap_slice_in_place(&mut result);
        result
    }
    
    fn swap_slice_in_place(slice: &mut [Self]) {
        if is_big_endian() {
            endian::inswap_le16_num(slice);
        }
    }
}

impl EndianSwap for u32 {
    fn swap(&self) -> Self {
        if is_big_endian() {
            endian::inswap_le32(*self)
        } else {
            *self
        }
    }
    
    fn swap_in_place(&mut self) {
        if is_big_endian() {
            *self = endian::inswap_le32(*self);
        }
    }
    
    fn swap_slice(slice: &[Self]) -> Vec<Self> {
        let mut result = slice.to_vec();
        Self::swap_slice_in_place(&mut result);
        result
    }
    
    fn swap_slice_in_place(slice: &mut [Self]) {
        if is_big_endian() {
            endian::inswap_le32_num(slice);
        }
    }
}

impl EndianSwap for i64 {
    fn swap(&self) -> Self {
        if is_big_endian() {
            endian::inswap_le64(*self)
        } else {
            *self
        }
    }
    
    fn swap_in_place(&mut self) {
        if is_big_endian() {
            *self = endian::inswap_le64(*self);
        }
    }
    
    fn swap_slice(slice: &[Self]) -> Vec<Self> {
        let mut result = slice.to_vec();
        Self::swap_slice_in_place(&mut result);
        result
    }
    
    fn swap_slice_in_place(slice: &mut [Self]) {
        if is_big_endian() {
            endian::inswap_le64_num(slice);
        }
    }
}

// Implement EndianSwap for SquashFS structures
impl EndianSwap for SquashFsSuperBlock {
    fn swap(&self) -> Self {
        if is_big_endian() {
            let mut swapped = *self;
            swapped.inodes = self.inodes.swap();
            swapped.mkfs_time = self.mkfs_time.swap();
            swapped.block_size = self.block_size.swap();
            swapped.fragments = self.fragments.swap();
            swapped.compression = self.compression.swap();
            swapped.block_log = self.block_log.swap();
            swapped.flags = self.flags.swap();
            swapped.no_ids = self.no_ids.swap();
            swapped.major = self.major.swap();
            swapped.minor = self.minor.swap();
            swapped.root_inode = self.root_inode.swap();
            swapped.bytes_used = self.bytes_used.swap();
            swapped.id_table_start = self.id_table_start.swap();
            swapped.xattr_id_table_start = self.xattr_id_table_start.swap();
            swapped.inode_table_start = self.inode_table_start.swap();
            swapped.directory_table_start = self.directory_table_start.swap();
            swapped.fragment_table_start = self.fragment_table_start.swap();
            swapped.lookup_table_start = self.lookup_table_start.swap();
            swapped
        } else {
            *self
        }
    }
    
    fn swap_in_place(&mut self) {
        if is_big_endian() {
            self.inodes.swap_in_place();
            self.mkfs_time.swap_in_place();
            self.block_size.swap_in_place();
            self.fragments.swap_in_place();
            self.compression.swap_in_place();
            self.block_log.swap_in_place();
            self.flags.swap_in_place();
            self.no_ids.swap_in_place();
            self.major.swap_in_place();
            self.minor.swap_in_place();
            self.root_inode.swap_in_place();
            self.bytes_used.swap_in_place();
            self.id_table_start.swap_in_place();
            self.xattr_id_table_start.swap_in_place();
            self.inode_table_start.swap_in_place();
            self.directory_table_start.swap_in_place();
            self.fragment_table_start.swap_in_place();
            self.lookup_table_start.swap_in_place();
        }
    }
    
    fn swap_slice(slice: &[Self]) -> Vec<Self> {
        let mut result = slice.to_vec();
        Self::swap_slice_in_place(&mut result);
        result
    }
    
    fn swap_slice_in_place(slice: &mut [Self]) {
        if is_big_endian() {
            for item in slice.iter_mut() {
                item.swap_in_place();
            }
        }
    }
}

// Helper functions for common operations
pub fn swap_value<T: EndianSwap>(value: T) -> T {
    value.swap()
}

pub fn swap_slice<T: EndianSwap>(slice: &[T]) -> Vec<T> {
    T::swap_slice(slice)
}

pub fn swap_value_into<T: EndianSwap>(value: &mut T) {
    value.swap_in_place();
}

pub fn swap_slice_into<T: EndianSwap>(slice: &mut [T]) {
    T::swap_slice_in_place(slice);
}

// Implement EndianSwap for SquashFS structures
impl EndianSwap for SquashFsDirIndex {
    fn swap_endianness(&mut self) {
        self.index.swap_endianness();
        self.start_block.swap_endianness();
        self.size.swap_endianness();
    }
}

impl EndianSwap for SquashFsBaseInodeHeader {
    fn swap_endianness(&mut self) {
        self.inode_type.swap_endianness();
        self.mode.swap_endianness();
        self.uid.swap_endianness();
        self.guid.swap_endianness();
        self.mtime.swap_endianness();
        self.inode_number.swap_endianness();
    }
}

impl EndianSwap for SquashFsIpcInodeHeader {
    fn swap_endianness(&mut self) {
        self.base.swap_endianness();
        self.nlink.swap_endianness();
    }
}

impl EndianSwap for SquashFsLIpcInodeHeader {
    fn swap_endianness(&mut self) {
        self.base.swap_endianness();
        self.xattr.swap_endianness();
    }
}

impl EndianSwap for SquashFsDevInodeHeader {
    fn swap_endianness(&mut self) {
        self.base.swap_endianness();
        self.nlink.swap_endianness();
        self.rdev.swap_endianness();
    }
}

impl EndianSwap for SquashFsLDevInodeHeader {
    fn swap_endianness(&mut self) {
        self.base.swap_endianness();
        self.xattr.swap_endianness();
    }
}

impl EndianSwap for SquashFsSymlinkInodeHeader {
    fn swap_endianness(&mut self) {
        self.base.swap_endianness();
        self.nlink.swap_endianness();
        self.symlink_size.swap_endianness();
    }
}

impl EndianSwap for SquashFsRegInodeHeader {
    fn swap_endianness(&mut self) {
        self.base.swap_endianness();
        self.start_block.swap_endianness();
        self.fragment.swap_endianness();
        self.offset.swap_endianness();
        self.file_size.swap_endianness();
    }
}

impl EndianSwap for SquashFsLRegInodeHeader {
    fn swap_endianness(&mut self) {
        self.base.swap_endianness();
        self.start_block.swap_endianness();
        self.file_size.swap_endianness();
        self.sparse.swap_endianness();
        self.nlink.swap_endianness();
        self.fragment.swap_endianness();
        self.offset.swap_endianness();
        self.xattr.swap_endianness();
    }
}

impl EndianSwap for SquashFsDirInodeHeader {
    fn swap_endianness(&mut self) {
        self.base.swap_endianness();
        self.start_block.swap_endianness();
        self.nlink.swap_endianness();
        self.file_size.swap_endianness();
        self.offset.swap_endianness();
        self.parent_inode.swap_endianness();
    }
}

impl EndianSwap for SquashFsLDirInodeHeader {
    fn swap_endianness(&mut self) {
        self.base.swap_endianness();
        self.nlink.swap_endianness();
        self.file_size.swap_endianness();
        self.start_block.swap_endianness();
        self.parent_inode.swap_endianness();
        self.i_count.swap_endianness();
        self.offset.swap_endianness();
        self.xattr.swap_endianness();
    }
}

impl EndianSwap for SquashFsDirEntry {
    fn swap_endianness(&mut self) {
        self.offset.swap_endianness();
        self.inode_number.swap_endianness();
        self.type_.swap_endianness();
        self.size.swap_endianness();
    }
}

impl EndianSwap for SquashFsDirHeader {
    fn swap_endianness(&mut self) {
        self.count.swap_endianness();
        self.start_block.swap_endianness();
        self.inode_number.swap_endianness();
    }
}

impl EndianSwap for SquashFsFragmentEntry {
    fn swap_endianness(&mut self) {
        self.start_block.swap_endianness();
        self.size.swap_endianness();
        self.unused.swap_endianness();
    }
}

impl EndianSwap for SquashFsXattrEntry {
    fn swap_endianness(&mut self) {
        self.type_.swap_endianness();
        self.size.swap_endianness();
    }
}

impl EndianSwap for SquashFsXattrVal {
    fn swap_endianness(&mut self) {
        self.vsize.swap_endianness();
    }
}

impl EndianSwap for SquashFsXattrId {
    fn swap_endianness(&mut self) {
        self.xattr.swap_endianness();
        self.count.swap_endianness();
        self.size.swap_endianness();
    }
}

impl EndianSwap for SquashFsXattrTable {
    fn swap_endianness(&mut self) {
        self.xattr_table_start.swap_endianness();
        self.xattr_ids.swap_endianness();
        self.unused.swap_endianness();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_type_swapping() {
        let value: u16 = 0x1234;
        assert_eq!(value.swap(), if is_big_endian() { 0x3412 } else { 0x1234 });
        
        let value: u32 = 0x12345678;
        assert_eq!(value.swap(), if is_big_endian() { 0x78563412 } else { 0x12345678 });
        
        let value: i64 = 0x1234567890ABCDEF;
        assert_eq!(value.swap(), if is_big_endian() { 0xEFCDAB9078563412 } else { 0x1234567890ABCDEF });
    }

    #[test]
    fn test_slice_swapping() {
        let slice: &[u16] = &[0x1234, 0x5678];
        let swapped = u16::swap_slice(slice);
        assert_eq!(swapped, if is_big_endian() { vec![0x3412, 0x7856] } else { vec![0x1234, 0x5678] });
        
        let mut slice = vec![0x1234, 0x5678];
        u16::swap_slice_in_place(&mut slice);
        assert_eq!(slice, if is_big_endian() { vec![0x3412, 0x7856] } else { vec![0x1234, 0x5678] });
    }

    #[test]
    fn test_superblock_swapping() {
        let mut superblock = SquashFsSuperBlock {
            s_magic: SQUASHFS_MAGIC,
            inodes: 100,
            mkfs_time: 1234567890,
            block_size: 4096,
            fragments: 10,
            compression: CompressionType::Gzip as u16,
            block_log: 12,
            flags: SquashFsFlags::new(0),
            no_ids: 1,
            major: 4,
            minor: 0,
            root_inode: 1,
            bytes_used: 1000000,
            id_table_start: 1000,
            xattr_id_table_start: 0,
            inode_table_start: 2000,
            directory_table_start: 3000,
            fragment_table_start: 4000,
            lookup_table_start: 0,
        };
        
        let swapped = superblock.swap();
        assert_eq!(swapped.s_magic, SQUASHFS_MAGIC);
        assert_eq!(swapped.inodes, if is_big_endian() { 100.swap() } else { 100 });
        assert_eq!(swapped.mkfs_time, if is_big_endian() { 1234567890.swap() } else { 1234567890 });
        
        superblock.swap_in_place();
        assert_eq!(superblock.inodes, if is_big_endian() { 100.swap() } else { 100 });
        assert_eq!(superblock.mkfs_time, if is_big_endian() { 1234567890.swap() } else { 1234567890 });
    }

    #[test]
    fn test_helper_functions() {
        let value: u16 = 0x1234;
        assert_eq!(swap_value(value), if is_big_endian() { 0x3412 } else { 0x1234 });
        
        let mut value = 0x1234;
        swap_value_into(&mut value);
        assert_eq!(value, if is_big_endian() { 0x3412 } else { 0x1234 });
        
        let slice: &[u16] = &[0x1234, 0x5678];
        assert_eq!(swap_slice(slice), if is_big_endian() { vec![0x3412, 0x7856] } else { vec![0x1234, 0x5678] });
        
        let mut slice = vec![0x1234, 0x5678];
        swap_slice_into(&mut slice);
        assert_eq!(slice, if is_big_endian() { vec![0x3412, 0x7856] } else { vec![0x1234, 0x5678] });
    }
} 