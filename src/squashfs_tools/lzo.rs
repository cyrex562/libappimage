use std::io::{self, Write};
use lzo::{Lzo1x1, Lzo1x11, Lzo1x12, Lzo1x15, Lzo1x999};
use crate::compressor::{Compressor, CompressorError};
use crate::endian::swap_le32;

/// Compression algorithm identifiers
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LzoAlgorithm {
    Lzo1x1,
    Lzo1x11,
    Lzo1x12,
    Lzo1x15,
    Lzo1x999,
}

impl LzoAlgorithm {
    /// Get the name of the algorithm
    pub fn name(&self) -> &'static str {
        match self {
            Self::Lzo1x1 => "lzo1x_1",
            Self::Lzo1x11 => "lzo1x_1_11",
            Self::Lzo1x12 => "lzo1x_1_12",
            Self::Lzo1x15 => "lzo1x_1_15",
            Self::Lzo1x999 => "lzo1x_999",
        }
    }

    /// Get the memory size required for compression
    pub fn memory_size(&self) -> usize {
        match self {
            Self::Lzo1x1 => Lzo1x1::memory_size(),
            Self::Lzo1x11 => Lzo1x11::memory_size(),
            Self::Lzo1x12 => Lzo1x12::memory_size(),
            Self::Lzo1x15 => Lzo1x15::memory_size(),
            Self::Lzo1x999 => Lzo1x999::memory_size(),
        }
    }
}

/// Compression options structure
#[derive(Debug, Clone)]
pub struct LzoCompOpts {
    /// Selected compression algorithm
    pub algorithm: LzoAlgorithm,
    /// Compression level (1-9, only used for LZO1X_999)
    pub compression_level: u32,
}

impl LzoCompOpts {
    /// Create default compression options
    pub fn new() -> Self {
        Self {
            algorithm: LzoAlgorithm::Lzo1x999,
            compression_level: 8, // SQUASHFS_LZO1X_999_COMP_DEFAULT
        }
    }

    /// Swap endianness of the options
    pub fn swap_endianness(&mut self) {
        self.algorithm = unsafe { std::mem::transmute(swap_le32(self.algorithm as u32)) };
        self.compression_level = swap_le32(self.compression_level);
    }
}

/// LZO compressor implementation
pub struct LzoCompressor {
    /// Compression options
    opts: LzoCompOpts,
    /// Workspace for compression
    workspace: Vec<u8>,
    /// Buffer for compression output
    buffer: Vec<u8>,
}

impl LzoCompressor {
    /// Create a new LZO compressor with default settings
    pub fn new(block_size: usize) -> Self {
        let opts = LzoCompOpts::new();
        let workspace_size = opts.algorithm.memory_size();
        let buffer_size = block_size + (block_size / 16) + 64 + 3; // LZO_MAX_EXPANSION

        Self {
            opts,
            workspace: vec![0; workspace_size],
            buffer: vec![0; buffer_size],
        }
    }

    /// Parse command line options
    pub fn parse_options(&mut self, args: &[String]) -> Result<usize, String> {
        if args.is_empty() {
            return Err("Missing option argument".to_string());
        }

        match args[0].as_str() {
            "-Xalgorithm" => {
                if args.len() < 2 {
                    return Err("Missing algorithm name".to_string());
                }

                let algorithm = match args[1].as_str() {
                    "lzo1x_1" => LzoAlgorithm::Lzo1x1,
                    "lzo1x_1_11" => LzoAlgorithm::Lzo1x11,
                    "lzo1x_1_12" => LzoAlgorithm::Lzo1x12,
                    "lzo1x_1_15" => LzoAlgorithm::Lzo1x15,
                    "lzo1x_999" => LzoAlgorithm::Lzo1x999,
                    _ => return Err("Unrecognized algorithm".to_string()),
                };

                self.opts.algorithm = algorithm;
                Ok(1)
            }
            "-Xcompression-level" => {
                if args.len() < 2 {
                    return Err("Missing compression level".to_string());
                }

                let level = args[1].parse::<u32>()
                    .map_err(|_| "Invalid compression level".to_string())?;

                if level < 1 || level > 9 {
                    return Err("Compression level must be between 1 and 9".to_string());
                }

                if self.opts.algorithm != LzoAlgorithm::Lzo1x999 {
                    return Err("Compression level only applies to lzo1x_999 algorithm".to_string());
                }

                self.opts.compression_level = level;
                Ok(1)
            }
            _ => Err("Unrecognized option".to_string()),
        }
    }

    /// Display usage information
    pub fn display_usage(&self, writer: &mut impl Write) -> io::Result<()> {
        writeln!(writer, "\t  -Xalgorithm <algorithm>")?;
        writeln!(writer, "\t\tWhere <algorithm> is one of:")?;
        
        for algorithm in [
            LzoAlgorithm::Lzo1x1,
            LzoAlgorithm::Lzo1x11,
            LzoAlgorithm::Lzo1x12,
            LzoAlgorithm::Lzo1x15,
            LzoAlgorithm::Lzo1x999,
        ] {
            writeln!(writer, "\t\t\t{}{}", 
                algorithm.name(),
                if algorithm == LzoAlgorithm::Lzo1x999 { " (default)" } else { "" }
            )?;
        }

        writeln!(writer, "\t  -Xcompression-level <compression-level>")?;
        writeln!(writer, "\t\t<compression-level> should be 1 .. 9 (default 8). Only applies to lzo1x_999 algorithm")?;
        Ok(())
    }
}

impl Compressor for LzoCompressor {
    fn compress(&self, input: &[u8], output: &mut [u8]) -> Result<usize, CompressorError> {
        let mut compressed_size = 0;

        // Compress using selected algorithm
        match self.opts.algorithm {
            LzoAlgorithm::Lzo1x1 => {
                Lzo1x1::compress(input, &mut self.buffer, &mut compressed_size, &self.workspace)
                    .map_err(|e| CompressorError::CompressionError(e.to_string()))?;
            }
            LzoAlgorithm::Lzo1x11 => {
                Lzo1x11::compress(input, &mut self.buffer, &mut compressed_size, &self.workspace)
                    .map_err(|e| CompressorError::CompressionError(e.to_string()))?;
            }
            LzoAlgorithm::Lzo1x12 => {
                Lzo1x12::compress(input, &mut self.buffer, &mut compressed_size, &self.workspace)
                    .map_err(|e| CompressorError::CompressionError(e.to_string()))?;
            }
            LzoAlgorithm::Lzo1x15 => {
                Lzo1x15::compress(input, &mut self.buffer, &mut compressed_size, &self.workspace)
                    .map_err(|e| CompressorError::CompressionError(e.to_string()))?;
            }
            LzoAlgorithm::Lzo1x999 => {
                Lzo1x999::compress_level(input, &mut self.buffer, &mut compressed_size, 
                    &self.workspace, self.opts.compression_level)
                    .map_err(|e| CompressorError::CompressionError(e.to_string()))?;
            }
        }

        // Check if compression was successful
        if compressed_size == 0 {
            return Ok(0); // Output buffer overflow
        }

        // Copy compressed data to output
        output[..compressed_size].copy_from_slice(&self.buffer[..compressed_size]);
        Ok(compressed_size)
    }

    fn decompress(&self, input: &[u8], output: &mut [u8]) -> Result<usize, CompressorError> {
        let mut decompressed_size = output.len();
        
        // Decompress using selected algorithm
        match self.opts.algorithm {
            LzoAlgorithm::Lzo1x1 => {
                Lzo1x1::decompress_safe(input, output, &mut decompressed_size)
                    .map_err(|e| CompressorError::DecompressionError(e.to_string()))?;
            }
            LzoAlgorithm::Lzo1x11 => {
                Lzo1x11::decompress_safe(input, output, &mut decompressed_size)
                    .map_err(|e| CompressorError::DecompressionError(e.to_string()))?;
            }
            LzoAlgorithm::Lzo1x12 => {
                Lzo1x12::decompress_safe(input, output, &mut decompressed_size)
                    .map_err(|e| CompressorError::DecompressionError(e.to_string()))?;
            }
            LzoAlgorithm::Lzo1x15 => {
                Lzo1x15::decompress_safe(input, output, &mut decompressed_size)
                    .map_err(|e| CompressorError::DecompressionError(e.to_string()))?;
            }
            LzoAlgorithm::Lzo1x999 => {
                Lzo1x999::decompress_safe(input, output, &mut decompressed_size)
                    .map_err(|e| CompressorError::DecompressionError(e.to_string()))?;
            }
        }

        Ok(decompressed_size)
    }

    fn name(&self) -> &str {
        "lzo"
    }

    fn supported(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lzo_compression() {
        let compressor = LzoCompressor::new(1024);
        let input = b"Hello, World!";
        let mut output = vec![0; input.len() * 2];
        
        let compressed_size = compressor.compress(input, &mut output).unwrap();
        assert!(compressed_size > 0);
        
        let mut decompressed = vec![0; input.len()];
        let decompressed_size = compressor.decompress(&output[..compressed_size], &mut decompressed).unwrap();
        assert_eq!(decompressed_size, input.len());
        assert_eq!(&decompressed[..decompressed_size], input);
    }

    #[test]
    fn test_lzo_algorithms() {
        let input = b"Test data";
        let mut output = vec![0; input.len() * 2];
        let mut decompressed = vec![0; input.len()];

        for algorithm in [
            LzoAlgorithm::Lzo1x1,
            LzoAlgorithm::Lzo1x11,
            LzoAlgorithm::Lzo1x12,
            LzoAlgorithm::Lzo1x15,
            LzoAlgorithm::Lzo1x999,
        ] {
            let mut compressor = LzoCompressor::new(1024);
            compressor.opts.algorithm = algorithm;

            let compressed_size = compressor.compress(input, &mut output).unwrap();
            assert!(compressed_size > 0);

            let decompressed_size = compressor.decompress(&output[..compressed_size], &mut decompressed).unwrap();
            assert_eq!(decompressed_size, input.len());
            assert_eq!(&decompressed[..decompressed_size], input);
        }
    }

    #[test]
    fn test_lzo_options() {
        let mut compressor = LzoCompressor::new(1024);
        
        // Test algorithm selection
        assert_eq!(compressor.parse_options(&["-Xalgorithm".to_string(), "lzo1x_999".to_string()]).unwrap(), 1);
        assert_eq!(compressor.opts.algorithm, LzoAlgorithm::Lzo1x999);
        
        // Test compression level
        assert_eq!(compressor.parse_options(&["-Xcompression-level".to_string(), "5".to_string()]).unwrap(), 1);
        assert_eq!(compressor.opts.compression_level, 5);
        
        // Test invalid options
        assert!(compressor.parse_options(&["-Xalgorithm".to_string()]).is_err());
        assert!(compressor.parse_options(&["-Xcompression-level".to_string()]).is_err());
        assert!(compressor.parse_options(&["-Xcompression-level".to_string(), "10".to_string()]).is_err());
    }
} 