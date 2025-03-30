use std::path::Path;
use std::ffi::{CString, CStr};
use std::os::unix::ffi::OsStrExt;
use libc::{self, c_void, size_t};
use regex::Regex;
use crate::squashfs_tools::error::{Error, Result};
use crate::squashfs_tools::xattr::{XattrList, XattrData};
use crate::squashfs_tools::unsquashfs_error::{error, error_start, error_exit};

/// Platform-specific xattr functions
#[cfg(target_os = "macos")]
mod platform {
    use libc::{self, c_void, size_t};
    use std::ffi::CString;

    pub unsafe fn lsetxattr(path: *const i8, name: *const i8, value: *const c_void, size: size_t, flags: i32) -> i32 {
        libc::setxattr(path, name, value, size, 0, flags | libc::XATTR_NOFOLLOW)
    }

    pub unsafe fn llistxattr(path: *const i8, list: *mut i8, size: size_t) -> i32 {
        libc::listxattr(path, list, size, libc::XATTR_NOFOLLOW)
    }

    pub unsafe fn lgetxattr(path: *const i8, name: *const i8, value: *mut c_void, size: size_t) -> i32 {
        libc::getxattr(path, name, value, size, 0, libc::XATTR_NOFOLLOW)
    }
}

#[cfg(not(target_os = "macos"))]
mod platform {
    use libc::{self, c_void, size_t};
    use std::ffi::CString;

    pub unsafe fn lsetxattr(path: *const i8, name: *const i8, value: *const c_void, size: size_t, flags: i32) -> i32 {
        libc::lsetxattr(path, name, value, size, flags)
    }

    pub unsafe fn llistxattr(path: *const i8, list: *mut i8, size: size_t) -> i32 {
        libc::llistxattr(path, list, size)
    }

    pub unsafe fn lgetxattr(path: *const i8, name: *const i8, value: *mut c_void, size: size_t) -> i32 {
        libc::lgetxattr(path, name, value, size)
    }
}

/// Read extended attributes from the system
pub fn read_xattrs_from_system(
    path: &Path,
    xattr_exclude_regex: Option<&Regex>,
    xattr_include_regex: Option<&Regex>,
    xattr_exc_list: Option<&[XattrData]>,
    xattr_inc_list: Option<&[XattrData]>,
) -> Result<Vec<XattrList>> {
    let path_c = CString::new(path.as_os_str().as_bytes())
        .map_err(|e| Error::XattrError(format!("Invalid path: {}", e)))?;

    // Get size of xattr list
    let size = unsafe {
        platform::llistxattr(path_c.as_ptr(), std::ptr::null_mut(), 0)
    };

    if size <= 0 {
        if size < 0 && std::io::Error::last_os_error().kind() != std::io::ErrorKind::Unsupported {
            error_start!("llistxattr for {} failed in read_attrs, because {}", 
                path.display(), std::io::Error::last_os_error());
            error_exit!(".  Ignoring\n");
        }
        return Ok(Vec::new());
    }

    // Allocate buffer for xattr names
    let mut xattr_names = vec![0u8; size as usize];
    let size = unsafe {
        platform::llistxattr(path_c.as_ptr(), xattr_names.as_mut_ptr() as *mut i8, size)
    };

    if size < 0 {
        if std::io::Error::last_os_error().kind() == std::io::ErrorKind::InvalidInput {
            // xattr list grew? Try again
            return read_xattrs_from_system(path, xattr_exclude_regex, xattr_include_regex, xattr_exc_list, xattr_inc_list);
        } else {
            error_start!("llistxattr for {} failed in read_attrs, because {}", 
                path.display(), std::io::Error::last_os_error());
            error_exit!(".  Ignoring\n");
            return Ok(Vec::new());
        }
    }

    let mut xattr_list = Vec::new();
    let mut p = xattr_names.as_ptr();

    while p < xattr_names.as_ptr().add(size as usize) {
        let name = unsafe {
            CStr::from_ptr(p as *const i8)
                .to_str()
                .map_err(|e| Error::XattrError(format!("Invalid xattr name: {}", e)))?
        };

        // Skip if excluded by regex
        if let Some(exclude_regex) = xattr_exclude_regex {
            if exclude_regex.is_match(name) {
                p = unsafe { p.add(name.len() + 1) };
                continue;
            }
        }

        // Skip if not included by regex
        if let Some(include_regex) = xattr_include_regex {
            if !include_regex.is_match(name) {
                p = unsafe { p.add(name.len() + 1) };
                continue;
            }
        }

        // Skip if excluded by action list
        if let Some(exc_list) = xattr_exc_list {
            if exc_list.iter().any(|x| x.matches(name)) {
                p = unsafe { p.add(name.len() + 1) };
                continue;
            }
        }

        // Skip if not included by action list
        if let Some(inc_list) = xattr_inc_list {
            if !inc_list.iter().any(|x| x.matches(name)) {
                p = unsafe { p.add(name.len() + 1) };
                continue;
            }
        }

        // Get xattr type
        let mut xattr = XattrList::new(name)?;
        if xattr.type_ == -1 {
            error!("Unrecognised xattr prefix {}\n", name);
            p = unsafe { p.add(name.len() + 1) };
            continue;
        }

        // Get xattr value
        let name_c = CString::new(xattr.full_name.as_bytes())
            .map_err(|e| Error::XattrError(format!("Invalid xattr name: {}", e)))?;

        let mut vsize = unsafe {
            platform::lgetxattr(path_c.as_ptr(), name_c.as_ptr(), std::ptr::null_mut(), 0)
        };

        if vsize < 0 {
            error_start!("lgetxattr failed for {} in read_attrs, because {}", 
                path.display(), std::io::Error::last_os_error());
            error_exit!(".  Ignoring\n");
            continue;
        }

        xattr.value = vec![0u8; vsize as usize];
        vsize = unsafe {
            platform::lgetxattr(
                path_c.as_ptr(),
                name_c.as_ptr(),
                xattr.value.as_mut_ptr() as *mut c_void,
                vsize,
            )
        };

        if vsize < 0 {
            if std::io::Error::last_os_error().kind() == std::io::ErrorKind::InvalidInput {
                // xattr grew? Try again
                continue;
            } else {
                error_start!("lgetxattr failed for {} in read_attrs, because {}", 
                    path.display(), std::io::Error::last_os_error());
                error_exit!(".  Ignoring\n");
                continue;
            }
        }

        xattr.vsize = vsize as usize;
        xattr_list.push(xattr);
        p = unsafe { p.add(name.len() + 1) };
    }

    Ok(xattr_list)
}

/// Write extended attributes to the system
pub fn write_xattr_to_system(path: &Path, xattr: &XattrList) -> Result<()> {
    let path_c = CString::new(path.as_os_str().as_bytes())
        .map_err(|e| Error::XattrError(format!("Invalid path: {}", e)))?;
    let name_c = CString::new(xattr.full_name.as_bytes())
        .map_err(|e| Error::XattrError(format!("Invalid xattr name: {}", e)))?;

    let result = unsafe {
        platform::lsetxattr(
            path_c.as_ptr(),
            name_c.as_ptr(),
            xattr.value.as_ptr() as *const c_void,
            xattr.vsize as size_t,
            0,
        )
    };

    if result < 0 {
        Err(Error::XattrError(format!(
            "Failed to write xattr {} to {}: {}",
            xattr.full_name,
            path.display(),
            std::io::Error::last_os_error()
        )))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs::File;

    #[test]
    fn test_read_write_xattrs() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test");
        File::create(&file_path).unwrap();

        // Test writing xattr
        let xattr = XattrList {
            full_name: "user.test".to_string(),
            type_: 0,
            value: b"test value".to_vec(),
            vsize: 10,
        };
        write_xattr_to_system(&file_path, &xattr).unwrap();

        // Test reading xattrs
        let xattrs = read_xattrs_from_system(
            &file_path,
            None,
            None,
            None,
            None,
        ).unwrap();
        assert!(!xattrs.is_empty());
        assert_eq!(xattrs[0].full_name, "user.test");
        assert_eq!(xattrs[0].value, b"test value");
    }

    #[test]
    fn test_xattr_regex_filtering() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test");
        File::create(&file_path).unwrap();

        // Write test xattrs
        let xattrs = vec![
            ("user.test1", b"value1"),
            ("user.test2", b"value2"),
            ("system.test", b"value3"),
        ];

        for (name, value) in xattrs {
            let xattr = XattrList {
                full_name: name.to_string(),
                type_: 0,
                value: value.to_vec(),
                vsize: value.len(),
            };
            write_xattr_to_system(&file_path, &xattr).unwrap();
        }

        // Test exclude regex
        let exclude_regex = Regex::new(r"^system\.").unwrap();
        let xattrs = read_xattrs_from_system(
            &file_path,
            Some(&exclude_regex),
            None,
            None,
            None,
        ).unwrap();
        assert_eq!(xattrs.len(), 2);
        assert!(xattrs.iter().all(|x| x.full_name.starts_with("user.")));

        // Test include regex
        let include_regex = Regex::new(r"^system\.").unwrap();
        let xattrs = read_xattrs_from_system(
            &file_path,
            None,
            Some(&include_regex),
            None,
            None,
        ).unwrap();
        assert_eq!(xattrs.len(), 1);
        assert!(xattrs[0].full_name.starts_with("system."));
    }
} 