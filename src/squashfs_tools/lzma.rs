use std::io::{self, Write};
use lzma_rs::{LzmaWriter, LzmaReader};
use crate::compressor::{Compressor, CompressorError};

/// Size of LZMA properties
const LZMA_PROPS_SIZE: usize = 5;
/// Size of LZMA header (properties + 8 bytes for uncompressed size)
const LZMA_HEADER_SIZE: usize = LZMA_PROPS_SIZE + 8;

/// LZMA compressor implementation
pub struct LzmaCompressor {
    /// Compression level (0-9)
    level: u8,
    /// Dictionary size (power of 2)
    dict_size: u32,
}

impl LzmaCompressor {
    /// Create a new LZMA compressor with default settings
    pub fn new() -> Self {
        Self {
            level: 5,
            dict_size: 32,
        }
    }

    /// Display usage information
    pub fn display_usage(&self, writer: &mut impl Write) -> io::Result<()> {
        writeln!(writer, "\t  (no options) (deprecated - no kernel support)")
    }
}

impl Compressor for LzmaCompressor {
    fn compress(&self, input: &[u8], output: &mut [u8]) -> Result<usize, CompressorError> {
        if output.len() < LZMA_HEADER_SIZE {
            return Err(CompressorError::CompressionError("Output buffer too small".to_string()));
        }

        // Create LZMA writer with properties
        let mut writer = LzmaWriter::new_encoder(&mut output[LZMA_HEADER_SIZE..], self.level)
            .map_err(|e| CompressorError::CompressionError(e.to_string()))?;

        // Write input data
        writer.write_all(input)
            .map_err(|e| CompressorError::CompressionError(e.to_string()))?;

        // Finish compression
        let compressed_size = writer.finish()
            .map_err(|e| CompressorError::CompressionError(e.to_string()))?;

        // Write uncompressed size in little-endian format
        let size = input.len() as u64;
        output[LZMA_PROPS_SIZE] = (size & 0xFF) as u8;
        output[LZMA_PROPS_SIZE + 1] = ((size >> 8) & 0xFF) as u8;
        output[LZMA_PROPS_SIZE + 2] = ((size >> 16) & 0xFF) as u8;
        output[LZMA_PROPS_SIZE + 3] = ((size >> 24) & 0xFF) as u8;
        output[LZMA_PROPS_SIZE + 4] = 0;
        output[LZMA_PROPS_SIZE + 5] = 0;
        output[LZMA_PROPS_SIZE + 6] = 0;
        output[LZMA_PROPS_SIZE + 7] = 0;

        Ok(compressed_size + LZMA_HEADER_SIZE)
    }

    fn decompress(&self, input: &[u8], output: &mut [u8]) -> Result<usize, CompressorError> {
        if input.len() < LZMA_HEADER_SIZE {
            return Err(CompressorError::DecompressionError("Input buffer too small".to_string()));
        }

        // Read uncompressed size from header
        let uncompressed_size = (input[LZMA_PROPS_SIZE] as u64) |
            ((input[LZMA_PROPS_SIZE + 1] as u64) << 8) |
            ((input[LZMA_PROPS_SIZE + 2] as u64) << 16) |
            ((input[LZMA_PROPS_SIZE + 3] as u64) << 24);

        if uncompressed_size > output.len() as u64 {
            return Err(CompressorError::DecompressionError("Output buffer too small".to_string()));
        }

        // Create LZMA reader
        let mut reader = LzmaReader::new_decoder(&input[LZMA_HEADER_SIZE..])
            .map_err(|e| CompressorError::DecompressionError(e.to_string()))?;

        // Read decompressed data
        let mut bytes_read = 0;
        loop {
            match reader.read(&mut output[bytes_read..]) {
                Ok(0) => break,
                Ok(n) => bytes_read += n,
                Err(e) => return Err(CompressorError::DecompressionError(e.to_string())),
            }
        }

        Ok(bytes_read)
    }

    fn name(&self) -> &str {
        "lzma"
    }

    fn supported(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lzma_compression() {
        let compressor = LzmaCompressor::new();
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
    fn test_lzma_header() {
        let compressor = LzmaCompressor::new();
        let input = b"Test data";
        let mut output = vec![0; input.len() * 2];
        
        let compressed_size = compressor.compress(input, &mut output).unwrap();
        
        // Verify header size
        assert!(compressed_size >= LZMA_HEADER_SIZE);
        
        // Verify uncompressed size in header
        let size = (output[LZMA_PROPS_SIZE] as u64) |
            ((output[LZMA_PROPS_SIZE + 1] as u64) << 8) |
            ((output[LZMA_PROPS_SIZE + 2] as u64) << 16) |
            ((output[LZMA_PROPS_SIZE + 3] as u64) << 24);
        assert_eq!(size, input.len() as u64);
    }
} 