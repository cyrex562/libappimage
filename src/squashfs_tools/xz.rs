use std::io::{self, Write};
use xz2::stream::{Action, Check, LzmaOptions, Stream};
use crate::compressor::{Compressor, CompressorError};
use crate::error::{SquashError, Result};
use std::collections::HashMap;

/// Memory limit for decompression (32MB)
const MEMLIMIT: u64 = 32 * 1024 * 1024;

/// Structure representing a BCJ filter
#[derive(Debug, Clone)]
struct BcjFilter {
    name: &'static str,
    id: u64,
    selected: bool,
}

/// Structure representing an XZ filter
#[derive(Debug)]
struct Filter {
    buffer: Vec<u8>,
    filters: Vec<lzma_sys::lzma_filter>,
    length: usize,
}

/// Structure representing an XZ stream
#[derive(Debug)]
struct XzStream {
    filters: Vec<Filter>,
    dictionary_size: usize,
    opt: LzmaOptions,
}

/// Structure for compression options
#[derive(Debug, Clone)]
struct CompOpts {
    dictionary_size: u32,
    flags: u32,
}

/// XZ compressor implementation
pub struct XzCompressor {
    /// Dictionary size
    dictionary_size: usize,
    /// Dictionary size as percentage of block size
    dictionary_percent: f32,
    /// Number of filters
    filter_count: usize,
    /// Selected BCJ filters
    bcj_filters: Vec<BcjFilter>,
    /// Compression preset (0-9)
    preset: u32,
    /// Literal context bits
    lc: i32,
    /// Literal position bits
    lp: i32,
    /// Position bits
    pb: i32,
}

impl XzCompressor {
    /// Create a new XZ compressor with default settings
    pub fn new() -> Self {
        let bcj_filters = vec![
            BcjFilter { name: "x86", id: lzma_sys::LZMA_FILTER_X86, selected: false },
            BcjFilter { name: "powerpc", id: lzma_sys::LZMA_FILTER_POWERPC, selected: false },
            BcjFilter { name: "ia64", id: lzma_sys::LZMA_FILTER_IA64, selected: false },
            BcjFilter { name: "arm", id: lzma_sys::LZMA_FILTER_ARM, selected: false },
            BcjFilter { name: "armthumb", id: lzma_sys::LZMA_FILTER_ARMTHUMB, selected: false },
            BcjFilter { name: "sparc", id: lzma_sys::LZMA_FILTER_SPARC, selected: false },
            BcjFilter { name: "arm64", id: lzma_sys::LZMA_FILTER_ARM64, selected: false },
            BcjFilter { name: "riscv", id: lzma_sys::LZMA_FILTER_RISCV, selected: false },
        ];

        Self {
            dictionary_size: 0,
            dictionary_percent: 0.0,
            filter_count: 1,
            bcj_filters: bcj_filters,
            preset: 6, // LZMA_PRESET_DEFAULT
            lc: -1,
            lp: -1,
            pb: -1,
        }
    }

    /// Parse XZ compressor options
    pub fn parse_options(&mut self, args: &[String]) -> Result<usize> {
        if args.is_empty() {
            return Ok(0);
        }

        match args[0].as_str() {
            "-Xbcj" => {
                if args.len() < 2 {
                    return Err(SquashError::CompressorError("Missing filter argument".to_string()));
                }

                let filters: Vec<&str> = args[1].split(',').collect();
                for filter_name in filters {
                    if let Some(filter) = self.bcj_filters.iter_mut().find(|f| f.name == filter_name) {
                        if !filter.selected {
                            filter.selected = true;
                            self.filter_count += 1;
                        }
                    } else {
                        return Err(SquashError::CompressorError(format!("Unrecognized filter: {}", filter_name)));
                    }
                }
                Ok(1)
            }
            "-Xdict-size" => {
                if args.len() < 2 {
                    return Err(SquashError::CompressorError("Missing dictionary size argument".to_string()));
                }

                let size_str = &args[1];
                if size_str.ends_with('%') {
                    let percent = size_str[..size_str.len()-1].parse::<f32>()
                        .map_err(|e| SquashError::CompressorError(format!("Invalid percentage: {}", e)))?;
                    if percent <= 0.0 || percent > 100.0 {
                        return Err(SquashError::CompressorError("Dictionary size percentage must be between 0 and 100".to_string()));
                    }
                    self.dictionary_percent = percent;
                    self.dictionary_size = 0;
                } else {
                    let mut size = size_str.parse::<f32>()
                        .map_err(|e| SquashError::CompressorError(format!("Invalid size: {}", e)))?;
                    
                    if size.fract() != 0.0 {
                        return Err(SquashError::CompressorError("Dictionary size cannot be fractional unless a percentage".to_string()));
                    }

                    let mut size = size as usize;
                    if size_str.ends_with('k') || size_str.ends_with('K') {
                        size *= 1024;
                    } else if size_str.ends_with('m') || size_str.ends_with('M') {
                        size *= 1024 * 1024;
                    }

                    self.dictionary_size = size;
                    self.dictionary_percent = 0.0;
                }
                Ok(1)
            }
            "-Xpreset" => {
                if args.len() < 2 {
                    return Err(SquashError::CompressorError("Missing preset level argument".to_string()));
                }

                let preset = args[1].parse::<u32>()
                    .map_err(|e| SquashError::CompressorError(format!("Invalid preset level: {}", e)))?;
                if preset > 9 {
                    return Err(SquashError::CompressorError("Preset level must be between 0 and 9".to_string()));
                }
                self.preset = preset;
                Ok(1)
            }
            "-Xe" => {
                self.preset |= lzma_sys::LZMA_PRESET_EXTREME;
                Ok(0)
            }
            "-Xlc" => {
                if args.len() < 2 {
                    return Err(SquashError::CompressorError("Missing LC value argument".to_string()));
                }

                let lc = args[1].parse::<i32>()
                    .map_err(|e| SquashError::CompressorError(format!("Invalid LC value: {}", e)))?;
                if lc < lzma_sys::LZMA_LCLP_MIN || lc > lzma_sys::LZMA_LCLP_MAX {
                    return Err(SquashError::CompressorError("LC value out of range".to_string()));
                }
                self.lc = lc;
                Ok(1)
            }
            "-Xlp" => {
                if args.len() < 2 {
                    return Err(SquashError::CompressorError("Missing LP value argument".to_string()));
                }

                let lp = args[1].parse::<i32>()
                    .map_err(|e| SquashError::CompressorError(format!("Invalid LP value: {}", e)))?;
                if lp < lzma_sys::LZMA_LCLP_MIN || lp > lzma_sys::LZMA_LCLP_MAX {
                    return Err(SquashError::CompressorError("LP value out of range".to_string()));
                }
                self.lp = lp;
                Ok(1)
            }
            "-Xpb" => {
                if args.len() < 2 {
                    return Err(SquashError::CompressorError("Missing PB value argument".to_string()));
                }

                let pb = args[1].parse::<i32>()
                    .map_err(|e| SquashError::CompressorError(format!("Invalid PB value: {}", e)))?;
                if pb < lzma_sys::LZMA_PB_MIN || pb > lzma_sys::LZMA_PB_MAX {
                    return Err(SquashError::CompressorError("PB value out of range".to_string()));
                }
                self.pb = pb;
                Ok(1)
            }
            _ => Ok(0),
        }
    }

    /// Post-process options after all arguments are parsed
    pub fn post_process_options(&mut self, block_size: usize) -> Result<()> {
        if self.dictionary_size > 0 || self.dictionary_percent > 0.0 {
            let dict_size = if self.dictionary_size > 0 {
                self.dictionary_size
            } else {
                (block_size as f32 * self.dictionary_percent / 100.0) as usize
            };

            if dict_size > block_size {
                return Err(SquashError::CompressorError("Dictionary size cannot be larger than block size".to_string()));
            }

            if dict_size < 8192 {
                return Err(SquashError::CompressorError("Dictionary size must be at least 8192 bytes".to_string()));
            }

            // Check if dictionary size is valid (2^n or 2^n+2^(n+1))
            let n = dict_size.trailing_zeros();
            let valid_size = dict_size == (1 << n) || dict_size == ((1 << n) + (1 << (n + 1)));
            if !valid_size {
                return Err(SquashError::CompressorError(
                    "Dictionary size must be storable in xz header as either 2^n or as 2^n+2^(n+1)".to_string()
                ));
            }

            self.dictionary_size = dict_size;
        } else {
            self.dictionary_size = block_size;
        }

        Ok(())
    }

    /// Display usage information
    pub fn display_usage(&self, writer: &mut impl Write) -> io::Result<()> {
        writeln!(writer, "\t  -Xbcj filter1,filter2,...,filterN")?;
        writeln!(writer, "\t\tCompress using filter1,filter2,...,filterN in turn (in addition to no filter), and choose the best compression. Available filters: x86, arm, armthumb, arm64, powerpc, sparc, ia64, riscv")?;
        writeln!(writer, "\t  -Xdict-size <dict-size>")?;
        writeln!(writer, "\t\tUse <dict-size> as the XZ dictionary size. The dictionary size can be specified as a percentage of the block size, or as an absolute value. The dictionary size must be less than or equal to the block size and 8192 bytes or larger. It must also be storable in the xz header as either 2^n or as 2^n+2^(n+1). Example dict-sizes are 75%, 50%, 37.5%, 25%, or 32K, 16K, 8K etc.")?;
        writeln!(writer, "\t  -Xpreset <preset-level>")?;
        writeln!(writer, "\t\tUse <preset-value> as the custom preset to use on compress. <preset-level> should be 0 .. 9 (default 6)")?;
        writeln!(writer, "\t  -Xe")?;
        writeln!(writer, "\t\tEnable additional compression settings by passing the EXTREME flag to the compression flags.")?;
        writeln!(writer, "\t  -Xlc <value>")?;
        writeln!(writer, "\t  -Xlp <value>")?;
        writeln!(writer, "\t  -Xpb <value>")?;
        Ok(())
    }
}

impl Compressor for XzCompressor {
    fn compress(&self, input: &[u8], output: &mut [u8]) -> Result<usize, CompressorError> {
        let mut stream = XzStream {
            filters: Vec::new(),
            dictionary_size: self.dictionary_size,
            opt: LzmaOptions::new_preset(self.preset, Check::Crc32)
                .map_err(|e| CompressorError::CompressionError(e.to_string()))?,
        };

        // Initialize filters
        for filter in &self.bcj_filters {
            if filter.selected {
                let mut xz_filter = Filter {
                    buffer: vec![0; output.len()],
                    filters: vec![
                        lzma_sys::lzma_filter {
                            id: filter.id,
                            options: std::ptr::null_mut(),
                        },
                        lzma_sys::lzma_filter {
                            id: lzma_sys::LZMA_FILTER_LZMA2,
                            options: &stream.opt as *const _ as *mut _,
                        },
                        lzma_sys::lzma_filter {
                            id: lzma_sys::LZMA_VLI_UNKNOWN,
                            options: std::ptr::null_mut(),
                        },
                    ],
                    length: 0,
                };

                // Set filter-specific options
                match filter.id {
                    lzma_sys::LZMA_FILTER_ARMTHUMB | lzma_sys::LZMA_FILTER_RISCV => {
                        stream.opt.lp = 1;
                    }
                    lzma_sys::LZMA_FILTER_POWERPC | lzma_sys::LZMA_FILTER_ARM |
                    lzma_sys::LZMA_FILTER_SPARC | lzma_sys::LZMA_FILTER_ARM64 => {
                        stream.opt.lp = 2;
                        stream.opt.lc = 2;
                    }
                    lzma_sys::LZMA_FILTER_IA64 => {
                        stream.opt.pb = 4;
                        stream.opt.lp = 4;
                        stream.opt.lc = 0;
                    }
                    _ => {}
                }

                // Override with user-specified values
                if self.lc >= 0 {
                    stream.opt.lc = self.lc;
                }
                if self.lp >= 0 {
                    stream.opt.lp = self.lp;
                }
                if self.pb >= 0 {
                    stream.opt.pb = self.pb;
                }

                stream.filters.push(xz_filter);
            }
        }

        // Add default LZMA2 filter if no BCJ filters are selected
        if stream.filters.is_empty() {
            stream.filters.push(Filter {
                buffer: vec![0; output.len()],
                filters: vec![
                    lzma_sys::lzma_filter {
                        id: lzma_sys::LZMA_FILTER_LZMA2,
                        options: &stream.opt as *const _ as *mut _,
                    },
                    lzma_sys::lzma_filter {
                        id: lzma_sys::LZMA_VLI_UNKNOWN,
                        options: std::ptr::null_mut(),
                    },
                ],
                length: 0,
            });
        }

        // Try each filter and select the best compression
        let mut best_length = usize::MAX;
        let mut best_filter = None;

        for filter in &mut stream.filters {
            let mut stream = Stream::new_encoder(&filter.filters, Check::Crc32)
                .map_err(|e| CompressorError::CompressionError(e.to_string()))?;

            let mut input_pos = 0;
            let mut output_pos = 0;
            let mut action = Action::Run;

            while input_pos < input.len() && output_pos < output.len() {
                stream.set_input(&input[input_pos..]);
                stream.set_output(&mut filter.buffer[output_pos..]);

                let status = stream.process(action)
                    .map_err(|e| CompressorError::CompressionError(e.to_string()))?;

                input_pos += stream.input_processed();
                output_pos += stream.output_processed();

                if status.is_done() {
                    break;
                }

                action = Action::Run;
            }

            if output_pos < best_length {
                best_length = output_pos;
                best_filter = Some(filter);
            }
        }

        // Copy best result to output
        if let Some(filter) = best_filter {
            output[..best_length].copy_from_slice(&filter.buffer[..best_length]);
            Ok(best_length)
        } else {
            Ok(0) // Output buffer overflow
        }
    }

    fn decompress(&self, input: &[u8], output: &mut [u8]) -> Result<usize, CompressorError> {
        let mut stream = Stream::new_decoder(MEMLIMIT)
            .map_err(|e| CompressorError::DecompressionError(e.to_string()))?;

        let mut input_pos = 0;
        let mut output_pos = 0;
        let mut action = Action::Run;

        while input_pos < input.len() && output_pos < output.len() {
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

        Ok(output_pos)
    }

    fn name(&self) -> &str {
        "xz"
    }

    fn supported(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xz_compression() {
        let compressor = XzCompressor::new();
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
    fn test_xz_options() {
        let mut compressor = XzCompressor::new();
        
        // Test -Xbcj option
        let args = vec!["-Xbcj".to_string(), "x86,arm".to_string()];
        let parsed = compressor.parse_options(&args).unwrap();
        assert_eq!(parsed, 1);
        assert_eq!(compressor.filter_count, 3); // 1 default + 2 selected filters
        
        // Test -Xdict-size option
        let args = vec!["-Xdict-size".to_string(), "8192".to_string()];
        let parsed = compressor.parse_options(&args).unwrap();
        assert_eq!(parsed, 1);
        assert_eq!(compressor.dictionary_size, 8192);
        
        // Test -Xpreset option
        let args = vec!["-Xpreset".to_string(), "9".to_string()];
        let parsed = compressor.parse_options(&args).unwrap();
        assert_eq!(parsed, 1);
        assert_eq!(compressor.preset, 9);
    }
} 