// Core functionality
mod alloc;
pub mod error;
pub mod fs;
pub mod memory;
pub mod read;
pub mod reader;
pub mod swap;

// Compression support
pub mod compressor;
pub mod gzip;
pub mod lz4;
pub mod lzma;
pub mod lzma_xz;
pub mod lzo;
pub mod xz;
pub mod zstd;

// File system operations
pub mod fragments;
pub mod pseudo;
pub mod xattr;
pub mod xattr_system;

// Tools and utilities
pub mod action;
pub mod action_eval;
pub mod action_impl;
pub mod atomic_swap;
pub mod caches_queues_lists;
pub mod compat;
pub mod date;
pub mod endian;
pub mod fnmatch;
pub mod help;
pub mod info;
pub mod limit;
pub mod merge_sort;
pub mod mksquashfs;
pub mod pager;
pub mod processors;
pub mod progress;
pub mod read_queue;
pub mod restore;
pub mod seq_queue;
pub mod signals;
pub mod sort;
pub mod symbolic_mode;
pub mod tar;
pub mod thread;
pub mod time_compat;
pub mod unsquashfs;
pub mod unsquashfs_error;
pub mod unsquashfs_help;
pub mod unsquashfs_info;
pub mod unsquashfs_xattr;

// Re-export commonly used types and functions
pub use alloc::*;
pub use compressor::{Compressor, CompressorError};

// Compression implementations
pub use gzip::GzipCompressor;
pub use lz4::Lz4Compressor;
pub use lzma::LzmaCompressor;
pub use lzma_xz::LzmaXzCompressor;
pub use lzo::LzoCompressor;
pub use xz::XzCompressor;
pub use zstd::ZstdCompressor;

// File system operations
pub use fragments::*;
pub use pseudo::*;
pub use xattr::*;
pub use xattr_system::*;

// Tools and utilities
pub use action::*;
pub use action_eval::*;
pub use action_impl::*;
pub use compat::*;
pub use endian::*;
pub use fnmatch::*;
pub use help::*;
pub use info::*;
pub use limit::*;
pub use merge_sort::*;
pub use mksquashfs::*;
pub use pager::*;
pub use processors::*;
pub use progress::*;
pub use read_queue::*;
pub use restore::*;
pub use seq_queue::*;
pub use signals::*;
pub use sort::*;
pub use symbolic_mode::*;
pub use tar::*;
pub use thread::*;
pub use time_compat::*;
pub use unsquashfs::*;
pub use unsquashfs_error::*;
pub use unsquashfs_help::*;
pub use unsquashfs_info::*;
pub use unsquashfs_xattr::*;

// Constants and types
pub const SQUASHFS_MAGIC: u32 = 0x73717368;
pub const SQUASHFS_START: u32 = 0x73717368;
pub const SQUASHFS_MAGIC_SWAP: u32 = 0x68737173;

/// SquashFS filesystem version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SquashFsVersion {
    /// Version 1.0
    V1_0,
    /// Version 2.0
    V2_0,
    /// Version 3.0
    V3_0,
    /// Version 4.0
    V4_0,
}

/// SquashFS compression type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionType {
    /// No compression
    None,
    /// GZIP compression
    Gzip,
    /// LZMA compression
    Lzma,
    /// LZO compression
    Lzo,
    /// XZ compression
    Xz,
    /// LZ4 compression
    Lz4,
    /// ZSTD compression
    Zstd,
}

/// SquashFS filesystem options
#[derive(Debug, Clone)]
pub struct SquashFsOptions {
    /// Block size
    pub block_size: usize,
    /// Compression type
    pub compression_type: CompressionType,
    /// Filesystem version
    pub version: SquashFsVersion,
    /// Whether to use extended attributes
    pub use_xattrs: bool,
    /// Whether to use fragments
    pub use_fragments: bool,
    /// Whether to use export table
    pub use_export_table: bool,
    /// Whether to use original inode numbers
    pub use_original_inode: bool,
}

impl Default for SquashFsOptions {
    fn default() -> Self {
        Self {
            block_size: 128 * 1024, // 128KB
            compression_type: CompressionType::Gzip,
            version: SquashFsVersion::V4_0,
            use_xattrs: true,
            use_fragments: true,
            use_export_table: false,
            use_original_inode: false,
        }
    }
}

/// SquashFS filesystem builder
pub struct SquashFsBuilder {
    options: SquashFsOptions,
}

impl SquashFsBuilder {
    /// Create a new SquashFS builder with default options
    pub fn new() -> Self {
        Self {
            options: SquashFsOptions::default(),
        }
    }

    /// Set the block size
    pub fn block_size(mut self, size: usize) -> Self {
        self.options.block_size = size;
        self
    }

    /// Set the compression type
    pub fn compression_type(mut self, compression: CompressionType) -> Self {
        self.options.compression_type = compression;
        self
    }

    /// Set the filesystem version
    pub fn version(mut self, version: SquashFsVersion) -> Self {
        self.options.version = version;
        self
    }

    /// Enable/disable extended attributes
    pub fn use_xattrs(mut self, use_xattrs: bool) -> Self {
        self.options.use_xattrs = use_xattrs;
        self
    }

    /// Enable/disable fragments
    pub fn use_fragments(mut self, use_fragments: bool) -> Self {
        self.options.use_fragments = use_fragments;
        self
    }

    /// Enable/disable export table
    pub fn use_export_table(mut self, use_export_table: bool) -> Self {
        self.options.use_export_table = use_export_table;
        self
    }

    /// Enable/disable original inode numbers
    pub fn use_original_inode(mut self, use_original_inode: bool) -> Self {
        self.options.use_original_inode = use_original_inode;
        self
    }

    /// Build the SquashFS filesystem
    pub fn build(self) -> Result<SquashFs> {
        SquashFs::new(self.options)
    }
}

/// SquashFS filesystem
pub struct SquashFs {
    options: SquashFsOptions,
    compressor: Box<dyn Compressor>,
}

impl SquashFs {
    /// Create a new SquashFS filesystem with the given options
    pub fn new(options: SquashFsOptions) -> Result<Self> {
        let compressor = match options.compression_type {
            CompressionType::None => Box::new(NoCompressor),
            CompressionType::Gzip => Box::new(GzipCompressor::new()),
            CompressionType::Lzma => Box::new(LzmaCompressor::new()),
            CompressionType::Lzo => Box::new(LzoCompressor::new()),
            CompressionType::Xz => Box::new(XzCompressor::new()),
            CompressionType::Lz4 => Box::new(Lz4Compressor::new()),
            CompressionType::Zstd => Box::new(ZstdCompressor::new()),
        };

        Ok(Self {
            options,
            compressor,
        })
    }

    /// Create a new SquashFS builder
    pub fn builder() -> SquashFsBuilder {
        SquashFsBuilder::new()
    }

    /// Get the filesystem options
    pub fn options(&self) -> &SquashFsOptions {
        &self.options
    }

    /// Get the compressor
    pub fn compressor(&self) -> &Box<dyn Compressor> {
        &self.compressor
    }
}

/// No compression implementation
struct NoCompressor;

impl Compressor for NoCompressor {
    fn compress(&self, input: &[u8], output: &mut [u8]) -> Result<usize, CompressorError> {
        if output.len() < input.len() {
            return Err(CompressorError::CompressionError("Output buffer too small".to_string()));
        }
        output[..input.len()].copy_from_slice(input);
        Ok(input.len())
    }

    fn decompress(&self, input: &[u8], output: &mut [u8]) -> Result<usize, CompressorError> {
        if output.len() < input.len() {
            return Err(CompressorError::DecompressionError("Output buffer too small".to_string()));
        }
        output[..input.len()].copy_from_slice(input);
        Ok(input.len())
    }

    fn name(&self) -> &str {
        "none"
    }

    fn supported(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_squashfs_builder() {
        let fs = SquashFs::builder()
            .block_size(256 * 1024)
            .compression_type(CompressionType::Zstd)
            .version(SquashFsVersion::V4_0)
            .use_xattrs(true)
            .use_fragments(true)
            .use_export_table(false)
            .use_original_inode(false)
            .build()
            .unwrap();

        assert_eq!(fs.options().block_size, 256 * 1024);
        assert_eq!(fs.options().compression_type, CompressionType::Zstd);
        assert_eq!(fs.options().version, SquashFsVersion::V4_0);
        assert!(fs.options().use_xattrs);
        assert!(fs.options().use_fragments);
        assert!(!fs.options().use_export_table);
        assert!(!fs.options().use_original_inode);
    }

    #[test]
    fn test_no_compression() {
        let compressor = NoCompressor;
        let input = b"Hello, World!";
        let mut output = vec![0; input.len() * 2];
        
        let compressed_size = compressor.compress(input, &mut output).unwrap();
        assert_eq!(compressed_size, input.len());
        assert_eq!(&output[..compressed_size], input);
        
        let mut decompressed = vec![0; input.len()];
        let decompressed_size = compressor.decompress(&output[..compressed_size], &mut decompressed).unwrap();
        assert_eq!(decompressed_size, input.len());
        assert_eq!(&decompressed[..decompressed_size], input);
    }
}
