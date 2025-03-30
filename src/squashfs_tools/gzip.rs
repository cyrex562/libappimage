use std::ffi::c_int;
use std::io::{self, Write};
use flate2::write::{DeflateEncoder, DeflateDecoder};
use flate2::Compression;
use flate2::read::DeflateDecoder as DeflateReader;
use super::endian::{is_big_endian, swap_le16, swap_le32, inswap_le16, inswap_le32};

/// Default compression level
pub const GZIP_DEFAULT_COMPRESSION_LEVEL: i32 = 9;
/// Default window size
pub const GZIP_DEFAULT_WINDOW_SIZE: i32 = 15;

/// Compression strategies
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CompressionStrategy {
    Default,
    Filtered,
    HuffmanOnly,
    RunLengthEncoded,
    Fixed,
}

impl CompressionStrategy {
    fn name(&self) -> &'static str {
        match self {
            CompressionStrategy::Default => "default",
            CompressionStrategy::Filtered => "filtered",
            CompressionStrategy::HuffmanOnly => "huffman_only",
            CompressionStrategy::RunLengthEncoded => "run_length_encoded",
            CompressionStrategy::Fixed => "fixed",
        }
    }

    fn value(&self) -> i32 {
        match self {
            CompressionStrategy::Default => 0,
            CompressionStrategy::Filtered => 1,
            CompressionStrategy::HuffmanOnly => 2,
            CompressionStrategy::RunLengthEncoded => 3,
            CompressionStrategy::Fixed => 4,
        }
    }
}

/// Gzip compression options structure matching the C version
#[repr(C)]
#[derive(Debug, Clone)]
pub struct GzipCompOpts {
    pub compression_level: i32,
    pub window_size: i16,
    pub strategy: i16,
}

impl GzipCompOpts {
    /// Create new compression options with default values
    pub fn new() -> Self {
        GzipCompOpts {
            compression_level: GZIP_DEFAULT_COMPRESSION_LEVEL,
            window_size: GZIP_DEFAULT_WINDOW_SIZE as i16,
            strategy: CompressionStrategy::Default.value() as i16,
        }
    }

    /// Swap endianness of the options if on big-endian system
    pub fn swap_endianness(&mut self) {
        if is_big_endian() {
            self.compression_level = inswap_le32(self.compression_level as u32) as i32;
            self.window_size = inswap_le16(self.window_size as u16) as i16;
            self.strategy = inswap_le16(self.strategy as u16) as i16;
        }
    }
}

/// Gzip strategy structure matching the C version
#[derive(Debug)]
pub struct GzipStrategy {
    pub strategy: i32,
    pub length: i32,
    pub buffer: Vec<u8>,
}

/// Gzip stream structure matching the C version
#[derive(Debug)]
pub struct GzipStream {
    pub strategies: Vec<GzipStrategy>,
}

impl GzipStream {
    pub fn new(block_size: usize, strategies: Vec<CompressionStrategy>) -> Self {
        let mut gzip_strategies = Vec::new();
        
        for (i, strategy) in strategies.into_iter().enumerate() {
            let mut buffer = if i == 0 {
                Vec::with_capacity(block_size)
            } else {
                Vec::with_capacity(block_size)
            };
            
            gzip_strategies.push(GzipStrategy {
                strategy: strategy.value(),
                length: 0,
                buffer,
            });
        }

        GzipStream {
            strategies: gzip_strategies,
        }
    }
}

/// Error types for gzip operations
#[derive(Debug)]
pub enum GzipError {
    CompressionError(String),
    DecompressionError(String),
    InvalidOptions(String),
    IOError(io::Error),
    BufferOverflow,
}

impl std::fmt::Display for GzipError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GzipError::CompressionError(msg) => write!(f, "Compression error: {}", msg),
            GzipError::DecompressionError(msg) => write!(f, "Decompression error: {}", msg),
            GzipError::InvalidOptions(msg) => write!(f, "Invalid options: {}", msg),
            GzipError::IOError(e) => write!(f, "IO error: {}", e),
            GzipError::BufferOverflow => write!(f, "Buffer overflow"),
        }
    }
}

impl std::error::Error for GzipError {}

pub type GzipResult<T> = Result<T, GzipError>;

/// Gzip compressor implementation
pub struct GzipCompressor {
    options: GzipCompOpts,
    stream: Option<GzipStream>,
}

impl GzipCompressor {
    pub fn new() -> Self {
        GzipCompressor {
            options: GzipCompOpts::new(),
            stream: None,
        }
    }

    pub fn with_options(options: GzipCompOpts) -> Self {
        GzipCompressor {
            options,
            stream: None,
        }
    }

    /// Initialize the compressor with a block size
    pub fn init(&mut self, block_size: usize) -> GzipResult<()> {
        let strategies = vec![CompressionStrategy::Default];
        self.stream = Some(GzipStream::new(block_size, strategies));
        Ok(())
    }

    /// Parse command line options for gzip compression
    pub fn parse_options(&mut self, args: &[String]) -> GzipResult<usize> {
        if args.is_empty() {
            return Ok(0);
        }

        match args[0].as_str() {
            "-Xcompression-level" => {
                if args.len() < 2 {
                    return Err(GzipError::InvalidOptions(
                        "Missing compression level".to_string(),
                    ));
                }

                let level = args[1].parse::<i32>().map_err(|e| {
                    GzipError::InvalidOptions(format!("Invalid compression level: {}", e))
                })?;

                if level < 1 || level > 9 {
                    return Err(GzipError::InvalidOptions(
                        "Compression level must be between 1 and 9".to_string(),
                    ));
                }

                self.options.compression_level = level;
                Ok(1)
            }
            "-Xwindow-size" => {
                if args.len() < 2 {
                    return Err(GzipError::InvalidOptions("Missing window size".to_string()));
                }

                let size = args[1].parse::<i32>().map_err(|e| {
                    GzipError::InvalidOptions(format!("Invalid window size: {}", e))
                })?;

                if size < 8 || size > 15 {
                    return Err(GzipError::InvalidOptions(
                        "Window size must be between 8 and 15".to_string(),
                    ));
                }

                self.options.window_size = size as i16;
                Ok(1)
            }
            "-Xstrategy" => {
                if args.len() < 2 {
                    return Err(GzipError::InvalidOptions("Missing strategies".to_string()));
                }

                let strategy = match args[1].as_str() {
                    "default" => CompressionStrategy::Default,
                    "filtered" => CompressionStrategy::Filtered,
                    "huffman_only" => CompressionStrategy::HuffmanOnly,
                    "run_length_encoded" => CompressionStrategy::RunLengthEncoded,
                    "fixed" => CompressionStrategy::Fixed,
                    _ => {
                        return Err(GzipError::InvalidOptions(format!(
                            "Unrecognized strategy: {}",
                            args[1]
                        )))
                    }
                };

                self.options.strategy = strategy.value() as i16;
                Ok(1)
            }
            _ => Ok(0),
        }
    }

    /// Compress data using the configured options
    pub fn compress(&mut self, input: &[u8], output: &mut [u8]) -> GzipResult<usize> {
        let stream = self.stream.as_mut().ok_or_else(|| {
            GzipError::CompressionError("Compressor not initialized".to_string())
        })?;

        let mut encoder = DeflateEncoder::new(
            Vec::new(),
            Compression::new(self.options.compression_level as u32),
        );

        encoder.write_all(input).map_err(GzipError::IOError)?;
        let compressed = encoder.finish().map_err(|e| {
            GzipError::CompressionError(format!("Failed to finish compression: {}", e))
        })?;

        if compressed.len() > output.len() {
            return Err(GzipError::BufferOverflow);
        }

        output[..compressed.len()].copy_from_slice(&compressed);
        Ok(compressed.len())
    }

    /// Decompress data
    pub fn decompress(&self, input: &[u8], output: &mut [u8]) -> GzipResult<usize> {
        let mut decoder = DeflateDecoder::new(Vec::new());
        decoder.write_all(input).map_err(GzipError::IOError)?;
        let decompressed = decoder.finish().map_err(|e| {
            GzipError::DecompressionError(format!("Failed to finish decompression: {}", e))
        })?;

        if decompressed.len() > output.len() {
            return Err(GzipError::BufferOverflow);
        }

        output[..decompressed.len()].copy_from_slice(&decompressed);
        Ok(decompressed.len())
    }

    /// Display compression options
    pub fn display_options(&self, stream: &mut dyn Write) -> io::Result<()> {
        writeln!(stream, "\tcompression-level {}", self.options.compression_level)?;
        writeln!(stream, "\twindow-size {}", self.options.window_size)?;
        
        let strategy = match self.options.strategy {
            0 => "default",
            1 => "filtered",
            2 => "huffman_only",
            3 => "run_length_encoded",
            4 => "fixed",
            _ => "unknown",
        };
        
        writeln!(stream, "\tStrategy selected: {}", strategy)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compression_decompression() {
        let mut compressor = GzipCompressor::new();
        compressor.init(1024).unwrap();
        
        let input = b"Hello, World!";
        let mut compressed = vec![0; input.len() * 2];
        let mut decompressed = vec![0; input.len()];

        let compressed_len = compressor.compress(input, &mut compressed).unwrap();
        let decompressed_len = compressor
            .decompress(&compressed[..compressed_len], &mut decompressed)
            .unwrap();

        assert_eq!(decompressed_len, input.len());
        assert_eq!(&decompressed[..decompressed_len], input);
    }

    #[test]
    fn test_options_parsing() {
        let mut compressor = GzipCompressor::new();
        
        let args = vec![
            "-Xcompression-level".to_string(),
            "9".to_string(),
            "-Xwindow-size".to_string(),
            "15".to_string(),
            "-Xstrategy".to_string(),
            "filtered".to_string(),
        ];

        let mut i = 0;
        while i < args.len() {
            let consumed = compressor.parse_options(&args[i..]).unwrap();
            i += consumed + 1;
        }

        assert_eq!(compressor.options.compression_level, 9);
        assert_eq!(compressor.options.window_size, 15);
        assert_eq!(compressor.options.strategy, CompressionStrategy::Filtered.value() as i16);
    }

    #[test]
    fn test_invalid_options() {
        let mut compressor = GzipCompressor::new();
        
        assert!(compressor.parse_options(&["-Xcompression-level".to_string()]).is_err());
        assert!(compressor.parse_options(&["-Xcompression-level".to_string(), "10".to_string()]).is_err());
        assert!(compressor.parse_options(&["-Xwindow-size".to_string(), "16".to_string()]).is_err());
        assert!(compressor.parse_options(&["-Xstrategy".to_string(), "invalid".to_string()]).is_err());
    }

    #[test]
    fn test_endianness_swapping() {
        let mut opts = GzipCompOpts::new();
        opts.swap_endianness();
        
        if is_big_endian() {
            assert_eq!(opts.compression_level, inswap_le32(GZIP_DEFAULT_COMPRESSION_LEVEL as u32) as i32);
            assert_eq!(opts.window_size, inswap_le16(GZIP_DEFAULT_WINDOW_SIZE as u16) as i16);
            assert_eq!(opts.strategy, inswap_le16(CompressionStrategy::Default.value() as u16) as i16);
        } else {
            assert_eq!(opts.compression_level, GZIP_DEFAULT_COMPRESSION_LEVEL);
            assert_eq!(opts.window_size, GZIP_DEFAULT_WINDOW_SIZE as i16);
            assert_eq!(opts.strategy, CompressionStrategy::Default.value() as i16);
        }
    }
} 