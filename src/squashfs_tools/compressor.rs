use std::io::{self, Write};
use std::fmt;
use super::error::{SquashError, SquashResult as Result};

/// Compression types supported by SquashFS
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CompressionType {
    None,
    Gzip,
    Lzma,
    Lzo,
    Xz,
    Lz4,
    Zstd,
}

impl CompressionType {
    pub fn id(&self) -> i32 {
        match self {
            CompressionType::Gzip => 1,
            CompressionType::Lzma => 2,
            CompressionType::Lzo => 3,
            CompressionType::Lz4 => 4,
            CompressionType::Xz => 5,
            CompressionType::Zstd => 6,
            CompressionType::None => 0,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            CompressionType::Gzip => "gzip",
            CompressionType::Lzma => "lzma",
            CompressionType::Lzo => "lzo",
            CompressionType::Lz4 => "lz4",
            CompressionType::Xz => "xz",
            CompressionType::Zstd => "zstd",
            CompressionType::None => "none",
        }
    }
}

#[derive(Debug)]
pub enum CompressorError {
    InitializationError(String),
    CompressionError(String),
    DecompressionError(String),
    OptionsError(String),
    IOError(io::Error),
}

impl fmt::Display for CompressorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompressorError::InitializationError(msg) => write!(f, "Initialization error: {}", msg),
            CompressorError::CompressionError(msg) => write!(f, "Compression error: {}", msg),
            CompressorError::DecompressionError(msg) => write!(f, "Decompression error: {}", msg),
            CompressorError::OptionsError(msg) => write!(f, "Options error: {}", msg),
            CompressorError::IOError(e) => write!(f, "IO error: {}", e),
        }
    }
}

impl std::error::Error for CompressorError {}

pub type CompressorResult<T> = std::result::Result<T, CompressorError>;

/// Compressor interface
pub trait Compressor: Send + Sync {
    /// Get the name of the compressor
    fn name(&self) -> &str;

    /// Get the compression type
    fn compression_type(&self) -> CompressionType;

    /// Check if this compressor is supported
    fn supported(&self) -> bool;

    /// Initialize the compressor
    fn init(&mut self) -> Result<()>;

    /// Compress data
    fn compress(&self, data: &[u8], block_size: usize) -> Result<Vec<u8>>;

    /// Decompress data
    fn decompress(&self, data: &[u8], expected_size: usize) -> Result<Vec<u8>>;

    /// Get the default block size for this compressor
    fn default_block_size(&self) -> usize;
}

/// Default compressor implementation
#[derive(Debug, Default)]
pub struct DefaultCompressor {
    compression_type: CompressionType,
}

impl DefaultCompressor {
    pub fn new(compression_type: CompressionType) -> Self {
        Self {
            compression_type,
        }
    }
}

impl Compressor for DefaultCompressor {
    fn name(&self) -> &str {
        match self.compression_type {
            CompressionType::None => "none",
            CompressionType::Gzip => "gzip",
            CompressionType::Lzma => "lzma",
            CompressionType::Lzo => "lzo",
            CompressionType::Xz => "xz",
            CompressionType::Lz4 => "lz4",
            CompressionType::Zstd => "zstd",
        }
    }

    fn compression_type(&self) -> CompressionType {
        self.compression_type
    }

    fn supported(&self) -> bool {
        self.compression_type != CompressionType::None
    }

    fn init(&mut self) -> Result<()> {
        Ok(())
    }

    fn compress(&self, data: &[u8], block_size: usize) -> Result<Vec<u8>> {
        match self.compression_type {
            CompressionType::None => Ok(data.to_vec()),
            _ => Err(SquashError::Compression("Compression not implemented".to_string())),
        }
    }

    fn decompress(&self, data: &[u8], expected_size: usize) -> Result<Vec<u8>> {
        match self.compression_type {
            CompressionType::None => Ok(data.to_vec()),
            _ => Err(SquashError::Compression("Decompression not implemented".to_string())),
        }
    }

    fn default_block_size(&self) -> usize {
        128 * 1024 // 128KB default block size
    }
}

/// Create a compressor based on type
pub fn create_compressor(compression_type: CompressionType) -> Box<dyn Compressor> {
    Box::new(DefaultCompressor::new(compression_type))
}

/// Get compression type from ID
pub fn get_compression_type(id: u16) -> CompressionType {
    match id {
        0 => CompressionType::None,
        1 => CompressionType::Gzip,
        2 => CompressionType::Lzma,
        3 => CompressionType::Lzo,
        4 => CompressionType::Xz,
        5 => CompressionType::Lz4,
        6 => CompressionType::Zstd,
        _ => CompressionType::None,
    }
}

/// Get compression ID from type
pub fn get_compression_id(compression_type: CompressionType) -> u16 {
    match compression_type {
        CompressionType::None => 0,
        CompressionType::Gzip => 1,
        CompressionType::Lzma => 2,
        CompressionType::Lzo => 3,
        CompressionType::Xz => 4,
        CompressionType::Lz4 => 5,
        CompressionType::Zstd => 6,
    }
}

pub trait CompressorStream: Send + Sync {
    fn compress(&mut self, dest: &mut [u8], src: &[u8], size: usize, block_size: usize) -> CompressorResult<usize>;
    fn uncompress(&mut self, dest: &mut [u8], src: &[u8], size: usize, block_size: usize) -> CompressorResult<usize>;
}

pub struct CompressorManager {
    compressors: Vec<Box<dyn Compressor>>,
}

impl CompressorManager {
    pub fn new() -> Self {
        let mut compressors = Vec::new();
        
        // Add compressors in order of preference
        compressors.push(Box::new(GzipCompressor));
        compressors.push(Box::new(LzoCompressor));
        compressors.push(Box::new(Lz4Compressor));
        compressors.push(Box::new(XzCompressor));
        compressors.push(Box::new(ZstdCompressor));
        compressors.push(Box::new(LzmaCompressor));
        compressors.push(Box::new(UnknownCompressor));
        
        CompressorManager { compressors }
    }

    pub fn lookup_compressor(&self, name: &str) -> &dyn Compressor {
        self.compressors.iter()
            .find(|c| c.name() == name)
            .map(|c| c.as_ref())
            .unwrap_or_else(|| self.compressors.last().unwrap().as_ref())
    }

    pub fn lookup_compressor_id(&self, id: i32) -> &dyn Compressor {
        self.compressors.iter()
            .find(|c| c.compression_type().id() == id)
            .map(|c| c.as_ref())
            .unwrap_or_else(|| self.compressors.last().unwrap().as_ref())
    }

    pub fn valid_compressor(&self, name: &str) -> bool {
        self.lookup_compressor(name).supported()
    }

    pub fn display_compressor_usage(&self, stream: &mut dyn Write, def_comp: &str, cols: usize) -> CompressorResult<()> {
        writeln!(stream, "\nCompressors available and compressor specific options:\n")?;
        
        for compressor in &self.compressors {
            if compressor.supported() {
                let is_default = compressor.name() == def_comp;
                let default_str = if is_default { " (default)" } else { "" };
                
                writeln!(stream, "\t{}{}", compressor.name(), default_str)?;
                compressor.usage(stream, cols)?;
            }
        }
        
        Ok(())
    }

    pub fn print_selected_comp_options(&self, stream: &mut dyn Write, comp: &dyn Compressor, prog_name: &str) -> CompressorResult<()> {
        let cols = 80; // TODO: Get actual terminal width
        writeln!(stream, "{}: selected compressor \"{}\". Options supported: {}", 
            prog_name, comp.name(), if comp.usage(stream, cols).is_ok() { "" } else { "none" })?;
        Ok(())
    }

    pub fn print_comp_options(&self, stream: &mut dyn Write, cols: usize, comp_name: &str, prog_name: &str) -> CompressorResult<()> {
        if comp_name == "all" {
            self.display_compressor_usage(stream, "gzip", cols)?;
            return Ok(());
        }

        if let Some(comp) = self.compressors.iter().find(|c| c.supported() && c.name() == comp_name) {
            writeln!(stream, "{}: compressor \"{}\". Options supported: {}", 
                prog_name, comp.name(), if comp.usage(stream, cols).is_ok() { "" } else { "none" })?;
        }
        
        Ok(())
    }
}

// Placeholder implementations for each compressor
struct GzipCompressor;
struct LzmaCompressor;
struct LzoCompressor;
struct Lz4Compressor;
struct XzCompressor;
struct ZstdCompressor;
struct UnknownCompressor;

// Implement Compressor trait for each compressor type
impl Compressor for GzipCompressor {
    fn name(&self) -> &str { "gzip" }
    fn compression_type(&self) -> CompressionType { CompressionType::Gzip }
    fn supported(&self) -> bool { true }
    
    fn init(&mut self) -> Result<()> {
        Err(CompressorError::InitializationError("Not implemented".to_string()))
    }
    
    fn compress(&self, _data: &[u8], _block_size: usize) -> Result<Vec<u8>> {
        Err(CompressorError::CompressionError("Not implemented".to_string()))
    }
    
    fn decompress(&self, _data: &[u8], _expected_size: usize) -> Result<Vec<u8>> {
        Err(CompressorError::DecompressionError("Not implemented".to_string()))
    }
    
    fn default_block_size(&self) -> usize {
        128 * 1024 // 128KB default block size
    }
}

// Similar implementations for other compressors...
// (LzmaCompressor, LzoCompressor, etc.)

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compressor_lookup() {
        let manager = CompressorManager::new();
        
        let gzip = manager.lookup_compressor("gzip");
        assert_eq!(gzip.name(), "gzip");
        assert!(gzip.supported());
        
        let unknown = manager.lookup_compressor("unknown");
        assert_eq!(unknown.name(), "unknown");
        assert!(!unknown.supported());
    }

    #[test]
    fn test_compression_type() {
        assert_eq!(CompressionType::Gzip.id(), 1);
        assert_eq!(CompressionType::Gzip.name(), "gzip");
    }
} 