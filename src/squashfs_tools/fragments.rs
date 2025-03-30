use std::io::{self, Read, Write};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::path::Path;
use crate::error::SquashError;
use crate::squashfs_tools::compressor::Compressor;
use crate::squashfs_tools::caches_queues_lists::{Cache, Queue};

/// Represents a fragment in the filesystem
#[derive(Debug, Clone)]
pub struct Fragment {
    pub index: usize,
    pub offset: usize,
    pub size: usize,
    pub start_block: u64,
    pub checksum: u16,
}

/// Represents a file buffer with fragment information
#[derive(Debug)]
pub struct FileBuffer {
    pub data: Vec<u8>,
    pub size: usize,
    pub file_size: u64,
    pub checksum: u16,
    pub fragment: Option<Fragment>,
    pub duplicate: bool,
    pub dupl_start: Option<Arc<FileInfo>>,
    pub cache: Option<Arc<Cache>>,
}

/// Represents file information for fragment processing
#[derive(Debug)]
pub struct FileInfo {
    pub fragment: Fragment,
    pub fragment_checksum: u16,
    pub have_frag_checksum: bool,
    pub frag_next: Option<Arc<FileInfo>>,
}

/// Structure to manage fragment processing
#[derive(Debug)]
pub struct FragmentProcessor {
    fragment_cache: Arc<Cache>,
    reserve_cache: Arc<Cache>,
    writer_cache: Arc<Cache>,
    dupl_frag: HashMap<u64, Arc<FileInfo>>,
    dup_mutex: Arc<Mutex<()>>,
    fragment_mutex: Arc<Mutex<()>>,
    compressor: Arc<dyn Compressor>,
    block_size: usize,
    sparse_files: bool,
    start_offset: u64,
}

impl FragmentProcessor {
    /// Create a new fragment processor
    pub fn new(
        compressor: Arc<dyn Compressor>,
        block_size: usize,
        sparse_files: bool,
        start_offset: u64,
    ) -> Self {
        Self {
            fragment_cache: Arc::new(Cache::new(block_size, 1024, false, false)),
            reserve_cache: Arc::new(Cache::new(block_size, 64, false, false)),
            writer_cache: Arc::new(Cache::new(block_size, 256, false, false)),
            dupl_frag: HashMap::new(),
            dup_mutex: Arc::new(Mutex::new(())),
            fragment_mutex: Arc::new(Mutex::new(())),
            compressor,
            block_size,
            sparse_files,
            start_offset,
        }
    }

    /// Calculate checksum and check for sparseness
    fn checksum_sparse(&self, buffer: &mut FileBuffer) -> bool {
        let mut chksum: u16 = 0;
        let mut sparse = true;

        for &byte in &buffer.data {
            chksum = (chksum & 1)
                .then(|| (chksum >> 1) | 0x8000)
                .unwrap_or(chksum >> 1);
            
            if byte != 0 {
                sparse = false;
                chksum = chksum.wrapping_add(byte as u16);
            }
        }

        buffer.checksum = chksum;
        sparse
    }

    /// Read from the filesystem
    fn read_filesystem(&self, fd: &mut impl Read, byte: u64, bytes: usize, buff: &mut [u8]) -> Result<(), SquashError> {
        let offset = self.start_offset + byte;
        // Seek to offset and read bytes
        // Implementation depends on your filesystem interface
        Ok(())
    }

    /// Get a fragment from cache or filesystem
    fn get_fragment(&self, fragment: &Fragment, data_buffer: &mut [u8], fd: &mut impl Read) -> Result<FileBuffer, SquashError> {
        let _dup_lock = self.dup_mutex.lock().map_err(|e| SquashError::Other(format!("Failed to acquire dup mutex: {}", e)))?;
        
        // Try to get from fragment cache
        if let Some(buffer) = self.fragment_cache.lookup(fragment.index) {
            return Ok(buffer);
        }

        // Try to get from reserve cache
        if let Some(buffer) = self.reserve_cache.lookup(fragment.index) {
            return Ok(buffer);
        }

        // Get new buffer from fragment cache
        let mut buffer = self.fragment_cache.get(fragment.index)
            .unwrap_or_else(|| self.reserve_cache.get(fragment.index)
                .expect("No space in reserve cache"));

        // Get compressed data
        let compressed_buffer = self.writer_cache.lookup(fragment.index);
        let size = fragment.size;
        let compressed = fragment.size & 0x80000000 != 0;
        let start_block = fragment.start_block;

        if compressed {
            let data = if let Some(compressed) = compressed_buffer {
                &compressed.data
            } else {
                self.read_filesystem(fd, start_block, size, data_buffer)?;
                data_buffer
            };

            let mut error = 0;
            let decompressed = self.compressor.uncompress(data, size, self.block_size, &mut error)
                .map_err(|e| SquashError::Other(format!("Decompression failed: {}", e)))?;
            
            buffer.data = decompressed;
        } else if let Some(compressed) = compressed_buffer {
            buffer.data = compressed.data.clone();
        } else {
            self.read_filesystem(fd, start_block, size, &mut buffer.data)?;
        }

        Ok(buffer)
    }

    /// Process fragments in a separate thread
    pub fn process_fragments(&self, destination_file: &Path) -> Result<(), SquashError> {
        let mut fd = std::fs::File::open(destination_file)
            .map_err(|e| SquashError::Other(format!("Failed to open destination file: {}", e)))?;

        let mut data_buffer = vec![0u8; self.block_size];

        loop {
            let mut file_buffer = self.to_process_frag.pop()
                .ok_or_else(|| SquashError::Other("No more fragments to process".to_string()))?;

            // Check for sparseness
            let sparse = self.checksum_sparse(&mut file_buffer);
            if self.sparse_files && sparse {
                file_buffer.fragment = None;
            }

            // Skip if this is part of a larger file
            if file_buffer.file_size != file_buffer.size as u64 {
                self.to_main.push(file_buffer);
                continue;
            }

            // Check for duplicates
            let file_size = file_buffer.file_size;
            let dupl_ptr = self.dupl_frag.get(&file_size).cloned();

            file_buffer.dupl_start = dupl_ptr.clone();
            file_buffer.duplicate = false;

            if let Some(dupl) = dupl_ptr {
                let mut current = dupl;
                while let Some(next) = &current.frag_next {
                    if file_size != current.fragment.size as u64 {
                        current = next.clone();
                        continue;
                    }

                    let checksum = if current.have_frag_checksum {
                        current.fragment_checksum
                    } else {
                        let mut cksum = 0;
                        let buffer = self.get_fragment_cksum(&current, &mut data_buffer, &mut fd, &mut cksum)?;
                        cksum
                    };

                    if checksum == file_buffer.checksum {
                        let buffer = self.get_fragment(&current.fragment, &mut data_buffer, &mut fd)?;
                        let offset = current.fragment.offset;
                        let size = current.fragment.size;

                        if file_buffer.data == buffer.data[offset..offset + size] {
                            let mut dup = file_buffer.clone();
                            dup.dupl_start = Some(current.clone());
                            dup.duplicate = true;
                            dup.cache = None;
                            file_buffer = dup;
                            break;
                        }
                    }

                    current = next.clone();
                }
            }

            self.to_main.push(file_buffer);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checksum_sparse() {
        let processor = FragmentProcessor::new(
            Arc::new(Compressor::new()),
            4096,
            true,
            0,
        );

        let mut buffer = FileBuffer {
            data: vec![0; 100],
            size: 100,
            file_size: 100,
            checksum: 0,
            fragment: None,
            duplicate: false,
            dupl_start: None,
            cache: None,
        };

        assert!(processor.checksum_sparse(&mut buffer));
    }
} 