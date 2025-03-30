use std::io::{self, Write};
use xz2::stream::{Action, Check, LzmaOptions, Stream};
use crate::compressor::{Compressor, CompressorError};

/// Size of LZMA properties
const LZMA_PROPS_SIZE: usize = 5;
/// Size of uncompressed size field
const LZMA_UNCOMP_SIZE: usize = 8;
/// Size of LZMA header (properties + uncompressed size)
const LZMA_HEADER_SIZE: usize = LZMA_PROPS_SIZE + LZMA_UNCOMP_SIZE;
/// Default compression level
const LZMA_OPTIONS: u32 = 5;
/// Memory limit for decompression (32MB)
const MEMLIMIT: u64 = 32 * 1024 * 1024;

/// LZMA XZ compressor implementation
pub struct LzmaXzCompressor {
    /// Compression level (0-9)
    level: u32,
    /// Dictionary size
    dict_size: u32,
}

impl LzmaXzCompressor {
    /// Create a new LZMA XZ compressor with default settings
    pub fn new() -> Self {
        Self {
            level: LZMA_OPTIONS,
            dict_size: 32,
        }
    }

    /// Display usage information
    pub fn display_usage(&self, writer: &mut impl Write) -> io::Result<()> {
        writeln!(writer, "\t  (no options) (deprecated - no kernel support)")
    }
}

impl Compressor for LzmaXzCompressor {
    fn compress(&self, input: &[u8], output: &mut [u8]) -> Result<usize, CompressorError> {
        if output.len() < LZMA_HEADER_SIZE {
            return Err(CompressorError::CompressionError("Output buffer too small".to_string()));
        }

        // Create LZMA options
        let options = LzmaOptions::new_preset(self.level as u32, Check::None)
            .map_err(|e| CompressorError::CompressionError(e.to_string()))?;

        // Create stream
        let mut stream = Stream::new_lzma_encoder(options)
            .map_err(|e| CompressorError::CompressionError(e.to_string()))?;

        // Set up input/output buffers
        let mut input_pos = 0;
        let mut output_pos = LZMA_HEADER_SIZE;
        let mut action = Action::Run;

        // Compress data
        while input_pos < input.len() && output_pos < output.len() {
            let input_remaining = input.len() - input_pos;
            let output_remaining = output.len() - output_pos;

            stream.set_input(&input[input_pos..]);
            stream.set_output(&mut output[output_pos..]);

            let status = stream.process(action)
                .map_err(|e| CompressorError::CompressionError(e.to_string()))?;

            input_pos += stream.input_processed();
            output_pos += stream.output_processed();

            if status.is_done() {
                break;
            }

            if output_pos >= output.len() {
                return Ok(0); // Output buffer overflow
            }

            action = Action::Run;
        }

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

        Ok(output_pos)
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

        // Create stream with memory limit
        let mut stream = Stream::new_lzma_decoder(MEMLIMIT)
            .map_err(|e| CompressorError::DecompressionError(e.to_string()))?;

        // Set up input/output buffers
        let mut input_pos = 0;
        let mut output_pos = 0;
        let mut action = Action::Run;

        // Decompress data
        while input_pos < input.len() && output_pos < output.len() {
            let input_remaining = input.len() - input_pos;
            let output_remaining = output.len() - output_pos;

            stream.set_input(&input[input_pos..]);
            stream.set_output(&mut output[output_pos..]);

            let status = stream.process(action)
                .map_err(|e| CompressorError::DecompressionError(e.to_string()))?;

            input_pos += stream.input_processed();
            output_pos += stream.output_processed();

            if status.is_done() {
                break;
            }

            action = Action::Run;
        }

        if output_pos != uncompressed_size as usize {
            return Err(CompressorError::DecompressionError("Size mismatch".to_string()));
        }

        Ok(output_pos)
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
    fn test_lzma_xz_compression() {
        let compressor = LzmaXzCompressor::new();
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
    fn test_lzma_xz_header() {
        let compressor = LzmaXzCompressor::new();
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