use std::io::{self, Write};
use lz4::{Encoder, Decoder};
use crate::compressor::{Compressor, CompressorError, CompressorOptions};
use crate::endian::{ByteOrder, swap_le32};

/// LZ4 compression options
#[derive(Debug, Clone, Copy)]
pub struct Lz4CompOpts {
    /// Stream format version
    pub version: u32,
    /// Compression flags
    pub flags: u32,
}

impl Lz4CompOpts {
    /// Create new LZ4 compression options
    pub fn new() -> Self {
        Self {
            version: LZ4_LEGACY,
            flags: 0,
        }
    }

    /// Swap endianness of the options
    pub fn swap_endianness(&mut self) {
        self.version = swap_le32(self.version);
        self.flags = swap_le32(self.flags);
    }
}

/// LZ4 compression flags
pub const LZ4_LEGACY: u32 = 1;
pub const LZ4_FLAGS_MASK: u32 = 1;
pub const LZ4_HC: u32 = 1;

/// LZ4 compressor implementation
pub struct Lz4Compressor {
    /// Whether to use high compression mode
    hc: bool,
}

impl Lz4Compressor {
    /// Create a new LZ4 compressor
    pub fn new() -> Self {
        Self {
            hc: false,
        }
    }

    /// Parse command line options
    pub fn parse_options(&mut self, args: &[String]) -> Result<usize, CompressorError> {
        if args.is_empty() {
            return Err(CompressorError::InvalidOption("No option provided".to_string()));
        }

        match args[0].as_str() {
            "-Xhc" => {
                self.hc = true;
                Ok(0)
            }
            _ => Err(CompressorError::InvalidOption(format!("Unknown option: {}", args[0])))
        }
    }

    /// Dump compression options
    pub fn dump_options(&self, _block_size: usize) -> Result<Lz4CompOpts, CompressorError> {
        let mut opts = Lz4CompOpts::new();
        opts.flags = if self.hc { LZ4_HC } else { 0 };
        opts.swap_endianness();
        Ok(opts)
    }

    /// Extract compression options
    pub fn extract_options(&mut self, _block_size: usize, buffer: &[u8]) -> Result<(), CompressorError> {
        if buffer.len() < std::mem::size_of::<Lz4CompOpts>() {
            return Err(CompressorError::InvalidOption("Buffer too small".to_string()));
        }

        let mut opts = unsafe { std::ptr::read(buffer.as_ptr() as *const Lz4CompOpts) };
        opts.swap_endianness();

        if opts.version != LZ4_LEGACY {
            return Err(CompressorError::InvalidOption("Unknown LZ4 version".to_string()));
        }

        if opts.flags & !LZ4_FLAGS_MASK != 0 {
            return Err(CompressorError::InvalidOption("Unknown LZ4 flags".to_string()));
        }

        self.hc = opts.flags & LZ4_HC != 0;
        Ok(())
    }

    /// Check compression options
    pub fn check_options(&self, _block_size: usize, buffer: &[u8]) -> Result<(), CompressorError> {
        if buffer.len() < std::mem::size_of::<Lz4CompOpts>() {
            return Err(CompressorError::InvalidOption("Buffer too small".to_string()));
        }

        let mut opts = unsafe { std::ptr::read(buffer.as_ptr() as *const Lz4CompOpts) };
        opts.swap_endianness();

        if opts.version != LZ4_LEGACY {
            return Err(CompressorError::InvalidOption("Unknown LZ4 version".to_string()));
        }

        Ok(())
    }

    /// Display compression options
    pub fn display_options(&self, buffer: &[u8], writer: &mut impl Write) -> Result<(), CompressorError> {
        if buffer.len() < std::mem::size_of::<Lz4CompOpts>() {
            return Err(CompressorError::InvalidOption("Buffer too small".to_string()));
        }

        let mut opts = unsafe { std::ptr::read(buffer.as_ptr() as *const Lz4CompOpts) };
        opts.swap_endianness();

        if opts.version != LZ4_LEGACY {
            return Err(CompressorError::InvalidOption("Unknown LZ4 version".to_string()));
        }

        if opts.flags & !LZ4_FLAGS_MASK != 0 {
            return Err(CompressorError::InvalidOption("Unknown LZ4 flags".to_string()));
        }

        if opts.flags & LZ4_HC != 0 {
            writeln!(writer, "\tHigh Compression option specified (-Xhc)")?;
        }

        Ok(())
    }
}

impl Compressor for Lz4Compressor {
    fn compress(&self, input: &[u8], output: &mut [u8]) -> Result<usize, CompressorError> {
        let mut encoder = if self.hc {
            Encoder::new(output)
        } else {
            Encoder::new(output)
        };

        encoder.write_all(input).map_err(|e| CompressorError::CompressionError(e.to_string()))?;
        encoder.finish().map_err(|e| CompressorError::CompressionError(e.to_string()))
    }

    fn decompress(&self, input: &[u8], output: &mut [u8]) -> Result<usize, CompressorError> {
        let mut decoder = Decoder::new(input).map_err(|e| CompressorError::DecompressionError(e.to_string()))?;
        let mut bytes_read = 0;
        
        loop {
            match decoder.read(&mut output[bytes_read..]) {
                Ok(0) => break,
                Ok(n) => bytes_read += n,
                Err(e) => return Err(CompressorError::DecompressionError(e.to_string())),
            }
        }

        Ok(bytes_read)
    }

    fn name(&self) -> &str {
        "lz4"
    }

    fn supported(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lz4_compression() {
        let compressor = Lz4Compressor::new();
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
    fn test_lz4_options() {
        let mut compressor = Lz4Compressor::new();
        
        // Test parsing options
        assert_eq!(compressor.parse_options(&["-Xhc".to_string()]).unwrap(), 0);
        assert!(compressor.hc);
        
        // Test invalid option
        assert!(compressor.parse_options(&["-Xinvalid".to_string()]).is_err());
    }
} 