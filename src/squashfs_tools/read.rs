use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;
use crate::error::SquashError;
use crate::compressor::Compressor;
use crate::squashfs_fs::{SquashfsSuperBlock, SquashfsInodeHeader, SquashfsBaseInodeHeader};
use crate::squashfs_swap::{swap_super_block, swap_base_inode_header, swap_dir_inode_header, swap_ldir_inode_header};
use crate::alloc::{malloc, free};

const SQUASHFS_METADATA_SIZE: usize = 8192;
const SQUASHFS_MAGIC: u32 = 0x73717368;
const SQUASHFS_MAGIC_SWAP: u32 = 0x68737173;
const SQUASHFS_MAJOR: u16 = 4;
const SQUASHFS_MINOR: u16 = 0;

/// Read a block from the filesystem
pub fn read_block<R: Read + Seek>(
    reader: &mut R,
    start: i64,
    next: Option<&mut i64>,
    expected: Option<usize>,
    block: &mut [u8],
) -> Result<usize, SquashError> {
    let mut c_byte = [0u8; 2];
    reader.seek(SeekFrom::Start(start as u64))?;
    reader.read_exact(&mut c_byte)?;

    let c_byte = u16::from_le_bytes(c_byte);
    let compressed = (c_byte & 0x8000) != 0;
    let c_byte = (c_byte & 0x7FFF) as usize;

    let outlen = expected.unwrap_or(SQUASHFS_METADATA_SIZE);

    if c_byte > outlen {
        return Err(SquashError::InvalidBlockSize);
    }

    if compressed {
        let mut buffer = vec![0u8; c_byte];
        reader.read_exact(&mut buffer)?;

        // TODO: Implement decompression using the compressor
        // For now, just copy the data
        block[..c_byte].copy_from_slice(&buffer);
        if let Some(next) = next {
            *next = start + 2 + c_byte as i64;
        }
        Ok(c_byte)
    } else {
        reader.read_exact(&mut block[..c_byte])?;
        if let Some(next) = next {
            *next = start + 2 + c_byte as i64;
        }
        Ok(c_byte)
    }
}

/// Read the superblock from a SquashFS filesystem
pub fn read_super<R: Read + Seek>(
    reader: &mut R,
    source: &Path,
) -> Result<(SquashfsSuperBlock, Compressor), SquashError> {
    let mut s_blk = SquashfsSuperBlock::default();
    let mut buffer = [0u8; std::mem::size_of::<SquashfsSuperBlock>()];

    reader.seek(SeekFrom::Start(0))?;
    reader.read_exact(&mut buffer)?;

    unsafe {
        std::ptr::copy_nonoverlapping(
            buffer.as_ptr(),
            &mut s_blk as *mut _ as *mut u8,
            std::mem::size_of::<SquashfsSuperBlock>(),
        );
    }

    swap_super_block(&mut s_blk);

    if s_blk.s_magic != SQUASHFS_MAGIC {
        if s_blk.s_magic == SQUASHFS_MAGIC_SWAP {
            return Err(SquashError::UnsupportedBigEndian);
        }
        return Err(SquashError::InvalidMagic);
    }

    if s_blk.s_major != SQUASHFS_MAJOR || s_blk.s_minor > SQUASHFS_MINOR {
        return Err(SquashError::UnsupportedVersion {
            major: s_blk.s_major,
            minor: s_blk.s_minor,
        });
    }

    // TODO: Implement compressor lookup and initialization
    let compressor = Compressor::default();

    Ok((s_blk, compressor))
}

/// Read the filesystem
pub fn read_filesystem<R: Read + Seek>(
    root_name: Option<&str>,
    reader: &mut R,
    s_blk: &mut SquashfsSuperBlock,
) -> Result<(), SquashError> {
    // TODO: Implement full filesystem reading
    // This will involve:
    // 1. Reading the inode table
    // 2. Reading the directory table
    // 3. Reading the fragment table
    // 4. Reading the id table
    // 5. Reading the lookup table
    // 6. Reading extended attributes

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;

    #[test]
    fn test_read_super() {
        let mut file = File::open("test.squashfs").unwrap();
        let (s_blk, _) = read_super(&mut file, Path::new("test.squashfs")).unwrap();
        assert_eq!(s_blk.s_magic, SQUASHFS_MAGIC);
    }
} 