use std::io::{self, Write};
use std::path::Path;
use regex::Regex;
use xattr::FileExt;
use crate::squashfs_tools::error::{Error, Result};
use crate::squashfs_tools::unsquashfs_error::{info, error, exit_unsquash, exit_unsquash_ignore, exit_unsquash_strict};
use crate::squashfs_tools::xattr::{XattrList, get_xattr, free_xattr};
use crate::squashfs_tools::squashfs::{sBlk, SQUASHFS_INVALID_XATTR, SQUASHFS_INVALID_BLK};
use crate::squashfs_tools::constants::{SQUASHFS_XATTR_PREFIX_MASK, SQUASHFS_XATTR_USER};

/// Maximum number of "no space" errors to print before suppressing
const NOSPACE_MAX: u32 = 10;

/// Global state for xattr handling
#[derive(Default)]
pub struct XattrState {
    /// Whether to ignore xattr errors
    pub ignore_errors: bool,
    /// Whether to be strict about xattr errors
    pub strict_errors: bool,
    /// Whether xattrs are supported by the filesystem
    pub xattrs_supported: bool,
    /// Whether we're running as root
    pub root_process: bool,
    /// Number of "no space" errors encountered
    pub nospace_error: u32,
    /// Whether we've shown the non-superuser error
    pub nonsuper_error: bool,
    /// Regex for excluding xattrs
    pub xattr_exclude_regex: Option<Regex>,
    /// Regex for including xattrs
    pub xattr_include_regex: Option<Regex>,
}

impl XattrState {
    pub fn new() -> Self {
        Self {
            ignore_errors: false,
            strict_errors: false,
            xattrs_supported: true,
            root_process: false,
            nospace_error: 0,
            nonsuper_error: false,
            xattr_exclude_regex: None,
            xattr_include_regex: None,
        }
    }
}

/// Check if a file has extended attributes
pub fn has_xattrs(xattr: u32) -> bool {
    xattr != SQUASHFS_INVALID_XATTR && 
    sBlk.s.xattr_id_table_start != SQUASHFS_INVALID_BLK
}

/// Write extended attributes to a file
pub fn write_xattr(pathname: &Path, xattr: u32, state: &mut XattrState) -> Result<bool> {
    if !state.xattrs_supported || !has_xattrs(xattr) {
        return Ok(true);
    }

    if xattr >= sBlk.xattr_ids {
        exit_unsquash(&format!("File system corrupted - xattr index in inode too large (xattr: {})", xattr));
    }

    let (xattr_list, count, failed) = get_xattr(xattr)?;
    if xattr_list.is_none() && !failed {
        return Err(Error::XattrError("Failed to get xattr list".to_string()));
    }

    if failed {
        exit_unsquash_strict(&format!("write_xattr: Failed to read one or more xattrs for {}", pathname.display()));
    }

    let mut success = true;
    let xattr_list = xattr_list.unwrap();

    for i in 0..count {
        let xattr = &xattr_list[i];
        let prefix = xattr.type_ & SQUASHFS_XATTR_PREFIX_MASK;

        // Check exclude/include regexes
        if let Some(exclude_regex) = &state.xattr_exclude_regex {
            if exclude_regex.is_match(xattr.full_name) {
                continue;
            }
        }

        if let Some(include_regex) = &state.xattr_include_regex {
            if !include_regex.is_match(xattr.full_name) {
                continue;
            }
        }

        if state.root_process || prefix == SQUASHFS_XATTR_USER {
            match xattr::set(pathname, xattr.full_name, xattr.value) {
                Ok(_) => continue,
                Err(e) => {
                    match e.kind() {
                        io::ErrorKind::Unsupported => {
                            error(&format!("write_xattr: failed to write xattr {} for file {} because extended attributes are not supported by the destination filesystem", 
                                xattr.full_name, pathname.display()));
                            error("Ignoring xattrs in filesystem\n");
                            exit_unsquash_strict("To avoid this error message, specify -no-xattrs\n");
                            state.xattrs_supported = false;
                            success = false;
                        }
                        io::ErrorKind::StorageFull | io::ErrorKind::QuotaExceeded => {
                            if state.nospace_error < NOSPACE_MAX {
                                exit_unsquash_ignore(&format!("write_xattr: failed to write xattr {} for file {} because no extended attribute space remaining (per file or filesystem limit)", 
                                    xattr.full_name, pathname.display()));
                                state.nospace_error += 1;
                                if state.nospace_error == NOSPACE_MAX {
                                    error!("{} of these errors printed, further error messages of this type are suppressed!\n", NOSPACE_MAX);
                                }
                            }
                            success = false;
                        }
                        _ => {
                            exit_unsquash_ignore(&format!("write_xattr: failed to write xattr {} for file {} because {}", 
                                xattr.full_name, pathname.display(), e));
                            success = false;
                        }
                    }
                }
            }
        } else if !state.nonsuper_error {
            error(&format!("write_xattr: could not write xattr {} for file {} because you're not superuser!", 
                xattr.full_name, pathname.display()));
            exit_unsquash_strict("write_xattr: to avoid this error message, either specify -xattrs-include '^user.', -no-xattrs, or run as superuser!\n");
            error("Further error messages of this type are suppressed!\n");
            state.nonsuper_error = true;
            success = false;
        }
    }

    free_xattr(xattr_list, count);
    Ok(success)
}

/// Print extended attributes to a file descriptor
pub fn print_xattr(pathname: &Path, xattr: u32, writer: &mut dyn Write, state: &XattrState) -> Result<()> {
    if !has_xattrs(xattr) {
        return Ok(());
    }

    if xattr >= sBlk.xattr_ids {
        exit_unsquash(&format!("File system corrupted - xattr index in inode too large (xattr: {})", xattr));
    }

    let (xattr_list, count, failed) = get_xattr(xattr)?;
    if xattr_list.is_none() && !failed {
        return Err(Error::XattrError("Failed to get xattr list".to_string()));
    }

    if failed {
        exit_unsquash_strict(&format!("write_xattr: Failed to read one or more xattrs for {}", pathname.display()));
    }

    let xattr_list = xattr_list.unwrap();

    for i in 0..count {
        let xattr = &xattr_list[i];

        // Check exclude/include regexes
        if let Some(exclude_regex) = &state.xattr_exclude_regex {
            if exclude_regex.is_match(xattr.full_name) {
                continue;
            }
        }

        if let Some(include_regex) = &state.xattr_include_regex {
            if !include_regex.is_match(xattr.full_name) {
                continue;
            }
        }

        writeln!(writer, "{} x ", pathname.display())?;
        print_xattr_name_value(xattr, writer)?;
    }

    free_xattr(xattr_list, count);
    Ok(())
}

/// Print a single xattr name and value
fn print_xattr_name_value(xattr: &XattrList, writer: &mut dyn Write) -> Result<()> {
    let value = xattr.value;
    let mut printable = true;
    let mut count = 0;

    // Count printable characters and calculate total size
    for &byte in value.iter() {
        if byte < 32 || byte > 126 {
            printable = false;
            count += 4;
        } else if byte == b'\\' {
            count += 4;
        } else {
            count += 1;
        }
    }

    let value = if !printable {
        let mut new = Vec::with_capacity(count + 2);
        new.extend_from_slice(b"0t");
        let mut i = 0;
        while i < value.len() {
            let byte = value[i];
            if byte < 32 || byte > 126 || byte == b'\\' {
                new.extend_from_slice(format!("\\{:03o}", byte).as_bytes());
                i += 1;
            } else {
                new.push(byte);
                i += 1;
            }
        }
        new
    } else {
        value.to_vec()
    };

    write!(writer, "{}=", xattr.full_name)?;
    writer.write_all(&value)?;
    writeln!(writer)?;

    Ok(())
}

/// Create a regex for xattr filtering
pub fn xattr_regex(pattern: &str, option: &str) -> Result<Regex> {
    Regex::new(pattern).map_err(|e| {
        Error::XattrError(format!("invalid regex {} in xattrs-{} option, because {}", pattern, option, e))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn test_has_xattrs() {
        assert!(!has_xattrs(SQUASHFS_INVALID_XATTR));
        assert!(!has_xattrs(0));
    }

    #[test]
    fn test_xattr_regex() {
        let regex = xattr_regex("^user.", "include").unwrap();
        assert!(regex.is_match("user.test"));
        assert!(!regex.is_match("system.test"));
    }

    #[test]
    fn test_print_xattr_name_value() {
        let mut output = Vec::new();
        let xattr = XattrList {
            full_name: "test".to_string(),
            type_: 0,
            value: vec![b't', b'e', b's', b't'],
        };
        print_xattr_name_value(&xattr, &mut output).unwrap();
        assert_eq!(output, b"test=test\n");
    }

    #[test]
    fn test_print_xattr_name_value_non_printable() {
        let mut output = Vec::new();
        let xattr = XattrList {
            full_name: "test".to_string(),
            type_: 0,
            value: vec![0, b't', b'e', b's', b't'],
        };
        print_xattr_name_value(&xattr, &mut output).unwrap();
        assert_eq!(output, b"test=0t\\000test\n");
    }

    #[test]
    fn test_write_xattr() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test");
        File::create(&file_path).unwrap();
        
        let mut state = XattrState::new();
        state.root_process = true;
        
        // This test just ensures the function doesn't panic
        write_xattr(&file_path, 0, &mut state).unwrap();
    }
} 