use std::io::{self, Write};
use zstd::stream::{Encoder, Decoder};
use crate::compressor::{Compressor, CompressorError};
use crate::error::{SquashError, Result};
use crate::endian_compat::EndianCompat;

/// Default compression level for ZSTD
const ZSTD_DEFAULT_COMPRESSION_LEVEL: i32 = 15;

/// Structure for ZSTD compression options
#[derive(Debug, Clone)]
pub struct ZstdCompOpts {
    /// Compression level (-22 to 22)
    pub compression_level: i32,
}

impl EndianCompat for ZstdCompOpts {
    fn swap_endianness(&mut self) {
        self.compression_level = self.compression_level.to_le();
    }
}

/// ZSTD compressor implementation
pub struct ZstdCompressor {
    /// Compression level
    compression_level: i32,
}

impl ZstdCompressor {
    /// Create a new ZSTD compressor with default settings
    pub fn new() -> Self {
        Self {
            compression_level: ZSTD_DEFAULT_COMPRESSION_LEVEL,
        }
    }

    /// Parse ZSTD compressor options
    pub fn parse_options(&mut self, args: &[String]) -> Result<usize> {
        if args.is_empty() {
            return Ok(0);
        }

        match args[0].as_str() {
            "-Xcompression-level" => {
                if args.len() < 2 {
                    return Err(SquashError::CompressorError("Missing compression level argument".to_string()));
                }

                let level = args[1].parse::<i32>()
                    .map_err(|e| SquashError::CompressorError(format!("Invalid compression level: {}", e)))?;

                if level == 0 {
                    return Err(SquashError::CompressorError(format!(
                        "Invalid compression level, should be {} <= n <= -1 or 1 <= n <= {}",
                        zstd::min_c_level(),
                        zstd::max_c_level()
                    )));
                }

                if level < 0 && level < zstd::min_c_level() {
                    return Err(SquashError::CompressorError(format!(
                        "Invalid compression level, should be {} <= n <= -1",
                        zstd::min_c_level()
                    )));
                }

                if level > 0 && level > zstd::max_c_level() {
                    return Err(SquashError::CompressorError(format!(
                        "Invalid compression level, should be 1 <= n <= {}",
                        zstd::max_c_level()
                    )));
                }

                self.compression_level = level;
                Ok(1)
            }
            _ => Ok(0),
        }
    }

    /// Dump compression options for storage in filesystem
    pub fn dump_options(&self, block_size: usize) -> Option<(ZstdCompOpts, usize)> {
        if self.compression_level == ZSTD_DEFAULT_COMPRESSION_LEVEL {
            None
        } else {
            Some((ZstdCompOpts {
                compression_level: self.compression_level,
            }, std::mem::size_of::<ZstdCompOpts>()))
        }
    }

    /// Extract compression options from filesystem
    pub fn extract_options(&mut self, block_size: usize, buffer: &[u8], size: usize) -> Result<()> {
        if size == 0 {
            self.compression_level = ZSTD_DEFAULT_COMPRESSION_LEVEL;
            return Ok(());
        }

        if size < std::mem::size_of::<ZstdCompOpts>() {
            return Err(SquashError::CompressorError("Invalid compression options size".to_string()));
        }

        let mut opts = unsafe {
            std::ptr::read_unaligned(buffer.as_ptr() as *const ZstdCompOpts)
        };
        opts.swap_endianness();

        if opts.compression_level == 0 ||
           opts.compression_level < zstd::min_c_level() ||
           opts.compression_level > zstd::max_c_level() {
            return Err(SquashError::CompressorError("Invalid compression level in options".to_string()));
        }

        self.compression_level = opts.compression_level;
        Ok(())
    }

    /// Display compression options
    pub fn display_options(&self, buffer: &[u8], size: usize) -> Result<()> {
        if size < std::mem::size_of::<ZstdCompOpts>() {
            return Err(SquashError::CompressorError("Invalid compression options size".to_string()));
        }

        let mut opts = unsafe {
            std::ptr::read_unaligned(buffer.as_ptr() as *const ZstdCompOpts)
        };
        opts.swap_endianness();

        if opts.compression_level == 0 ||
           opts.compression_level < zstd::min_c_level() ||
           opts.compression_level > zstd::max_c_level() {
            return Err(SquashError::CompressorError("Invalid compression level in options".to_string()));
        }

        println!("\tcompression-level {}", opts.compression_level);
        Ok(())
    }

    /// Display usage information
    pub fn display_usage(&self, writer: &mut impl Write) -> io::Result<()> {
        writeln!(writer, "\t  -Xcompression-level <compression-level>")?;
        writeln!(writer, "\t\t<compression-level> should be {} .. -1 or 1 .. {} (default {}). Negative compression levels correspond to the zstd --fast option.",
            zstd::min_c_level(),
            zstd::max_c_level(),
            ZSTD_DEFAULT_COMPRESSION_LEVEL
        )?;
        Ok(())
    }
}

impl Compressor for ZstdCompressor {
    fn compress(&self, input: &[u8], output: &mut [u8]) -> Result<usize, CompressorError> {
        let mut encoder = Encoder::new(output, self.compression_level)
            .map_err(|e| CompressorError::CompressionError(e.to_string()))?;

        encoder.write_all(input)
            .map_err(|e| CompressorError::CompressionError(e.to_string()))?;

        encoder.finish()
            .map_err(|e| CompressorError::CompressionError(e.to_string()))
    }

    fn decompress(&self, input: &[u8], output: &mut [u8]) -> Result<usize, CompressorError> {
        let mut decoder = Decoder::new(input)
            .map_err(|e| CompressorError::DecompressionError(e.to_string()))?;

        let mut bytes_read = 0;
        loop {
            let n = decoder.read(&mut output[bytes_read..])
                .map_err(|e| CompressorError::DecompressionError(e.to_string()))?;
            if n == 0 {
                break;
            }
            bytes_read += n;
        }

        Ok(bytes_read)
    }

    fn name(&self) -> &str {
        "zstd"
    }

    fn supported(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zstd_compression() {
        let compressor = ZstdCompressor::new();
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
    fn test_zstd_options() {
        let mut compressor = ZstdCompressor::new();
        
        // Test valid compression level
        let args = vec!["-Xcompression-level".to_string(), "3".to_string()];
        let parsed = compressor.parse_options(&args).unwrap();
        assert_eq!(parsed, 1);
        assert_eq!(compressor.compression_level, 3);
        
        // Test invalid compression level
        let args = vec!["-Xcompression-level".to_string(), "0".to_string()];
        assert!(compressor.parse_options(&args).is_err());
        
        // Test negative compression level
        let args = vec!["-Xcompression-level".to_string(), "-5".to_string()];
        let parsed = compressor.parse_options(&args).unwrap();
        assert_eq!(parsed, 1);
        assert_eq!(compressor.compression_level, -5);
    }

    #[test]
    fn test_zstd_options_serialization() {
        let mut compressor = ZstdCompressor::new();
        compressor.compression_level = 5;
        
        let (opts, size) = compressor.dump_options(4096).unwrap();
        assert_eq!(size, std::mem::size_of::<ZstdCompOpts>());
        assert_eq!(opts.compression_level, 5);
        
        let mut buffer = vec![0u8; size];
        unsafe {
            std::ptr::write_unaligned(buffer.as_mut_ptr() as *mut ZstdCompOpts, opts);
        }
        
        let mut new_compressor = ZstdCompressor::new();
        new_compressor.extract_options(4096, &buffer, size).unwrap();
        assert_eq!(new_compressor.compression_level, 5);
    }
} 