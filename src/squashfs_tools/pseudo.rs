use std::collections::HashMap;
use std::ffi::{CString, OsString};
use std::os::unix::ffi::OsStringExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use libc::{mode_t, uid_t, gid_t, dev_t, time_t};
use crate::error::SquashError;
use crate::xattr::Xattr;

/// File types for pseudo files
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PseudoFileType {
    Other,
    Process,
    Data,
}

/// Pseudo file types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PseudoType {
    Directory,
    Regular,
    Block,
    Character,
    Fifo,
    Socket,
    Symlink,
}

/// Statistics for pseudo files
#[derive(Debug, Clone)]
pub struct PseudoStat {
    pub mode: mode_t,
    pub uid: uid_t,
    pub gid: gid_t,
    pub major: u32,
    pub minor: u32,
    pub mtime: time_t,
    pub ino: i32,
}

/// Pseudo file data
#[derive(Debug)]
pub struct PseudoData {
    pub file: Arc<PseudoFile>,
    pub offset: i64,
    pub length: i64,
    pub sparse: bool,
}

/// Pseudo file information
#[derive(Debug)]
pub struct PseudoFile {
    pub filename: String,
    pub start: i64,
    pub current: i64,
    pub fd: i32,
}

/// Pseudo device information
#[derive(Debug)]
pub struct PseudoDev {
    pub type_: PseudoType,
    pub pseudo_type: PseudoFileType,
    pub stat: PseudoStat,
    pub data: Option<PseudoData>,
    pub command: Option<String>,
    pub symlink: Option<String>,
    pub linkname: Option<String>,
}

/// Pseudo entry in the filesystem
#[derive(Debug)]
pub struct PseudoEntry {
    pub name: String,
    pub pathname: String,
    pub pseudo: Option<Box<Pseudo>>,
    pub dev: Option<PseudoDev>,
    pub xattr: Option<Vec<Xattr>>,
    pub next: Option<Box<PseudoEntry>>,
}

/// Pseudo filesystem structure
#[derive(Debug)]
pub struct Pseudo {
    pub names: i32,
    pub current: Option<Box<PseudoEntry>>,
    pub head: Option<Box<PseudoEntry>>,
}

impl Pseudo {
    /// Create a new pseudo filesystem
    pub fn new() -> Self {
        Self {
            names: 0,
            current: None,
            head: None,
        }
    }

    /// Add a pseudo device to the filesystem
    pub fn add_pseudo(&mut self, dev: PseudoDev, target: &str, alltarget: &str) -> Result<(), SquashError> {
        let mut path = PathBuf::from(target);
        let mut components: Vec<String> = path.components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect();

        if components.is_empty() {
            components.push("/".to_string());
        }

        let mut current = self;
        let mut current_path = String::new();

        for (i, component) in components.iter().enumerate() {
            current_path = if i == 0 {
                component.clone()
            } else {
                format!("{}/{}", current_path, component)
            };

            let entry = current.find_or_create_entry(component, &current_path)?;

            if i == components.len() - 1 {
                // At leaf component
                if let Some(ref mut existing_dev) = entry.dev {
                    if !existing_dev.eq(&dev) {
                        return Err(SquashError::Other(format!(
                            "{} already exists as a different pseudo definition",
                            alltarget
                        )));
                    }
                } else {
                    entry.dev = Some(dev);
                }
            } else {
                // Create or get subdirectory
                if entry.pseudo.is_none() {
                    entry.pseudo = Some(Box::new(Pseudo::new()));
                }
                current = entry.pseudo.as_mut().unwrap();
            }
        }

        Ok(())
    }

    /// Find or create a pseudo entry
    fn find_or_create_entry(&mut self, name: &str, pathname: &str) -> Result<&mut PseudoEntry, SquashError> {
        let mut current = &mut self.head;
        let mut prev = None;

        while let Some(ref mut entry) = current {
            match entry.name.cmp(name) {
                std::cmp::Ordering::Equal => {
                    return Ok(entry);
                }
                std::cmp::Ordering::Greater => {
                    break;
                }
                std::cmp::Ordering::Less => {
                    prev = Some(entry);
                    current = &mut entry.next;
                }
            }
        }

        let new_entry = Box::new(PseudoEntry {
            name: name.to_string(),
            pathname: pathname.to_string(),
            pseudo: None,
            dev: None,
            xattr: None,
            next: current.take(),
        });

        if let Some(prev) = prev {
            prev.next = Some(new_entry);
        } else {
            self.head = Some(new_entry);
        }

        self.names += 1;
        Ok(self.head.as_mut().unwrap())
    }

    /// Add extended attributes to a pseudo entry
    pub fn add_xattr(&mut self, xattr: Xattr, target: &str, alltarget: &str) -> Result<(), SquashError> {
        let mut path = PathBuf::from(target);
        let mut components: Vec<String> = path.components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect();

        if components.is_empty() {
            components.push("/".to_string());
        }

        let mut current = self;
        let mut current_path = String::new();

        for (i, component) in components.iter().enumerate() {
            current_path = if i == 0 {
                component.clone()
            } else {
                format!("{}/{}", current_path, component)
            };

            let entry = current.find_or_create_entry(component, &current_path)?;

            if i == components.len() - 1 {
                // At leaf component
                if let Some(ref mut xattrs) = entry.xattr {
                    xattrs.push(xattr);
                } else {
                    entry.xattr = Some(vec![xattr]);
                }
            } else {
                // Create or get subdirectory
                if entry.pseudo.is_none() {
                    entry.pseudo = Some(Box::new(Pseudo::new()));
                }
                current = entry.pseudo.as_mut().unwrap();
            }
        }

        Ok(())
    }

    /// Get a subdirectory by name
    pub fn subdir(&self, filename: &str) -> Option<&Pseudo> {
        self.head.as_ref().and_then(|entry| {
            if entry.name == filename {
                entry.pseudo.as_deref()
            } else {
                None
            }
        })
    }

    /// Read the next directory entry
    pub fn readdir(&mut self) -> Option<&PseudoEntry> {
        if let Some(ref mut current) = self.current {
            self.current = current.next.take();
            Some(current)
        } else {
            self.current = self.head.clone();
            self.current.as_deref()
        }
    }
}

impl PseudoDev {
    /// Create a new pseudo device
    pub fn new(type_: PseudoType, mode: mode_t, uid: uid_t, gid: gid_t) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as time_t;

        Self {
            type_,
            pseudo_type: PseudoFileType::Other,
            stat: PseudoStat {
                mode,
                uid,
                gid,
                major: 0,
                minor: 0,
                mtime: now,
                ino: 0, // Will be set when added to filesystem
            },
            data: None,
            command: None,
            symlink: None,
            linkname: None,
        }
    }

    /// Create a new directory pseudo device
    pub fn new_directory(mode: mode_t, uid: uid_t, gid: gid_t) -> Self {
        Self::new(PseudoType::Directory, mode | libc::S_IFDIR, uid, gid)
    }

    /// Create a new regular file pseudo device
    pub fn new_regular(mode: mode_t, uid: uid_t, gid: gid_t) -> Self {
        Self::new(PseudoType::Regular, mode | libc::S_IFREG, uid, gid)
    }

    /// Create a new symlink pseudo device
    pub fn new_symlink(target: String, uid: uid_t, gid: gid_t) -> Self {
        Self {
            type_: PseudoType::Symlink,
            pseudo_type: PseudoFileType::Other,
            stat: PseudoStat {
                mode: libc::S_IFLNK | 0o777,
                uid,
                gid,
                major: 0,
                minor: 0,
                mtime: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as time_t,
                ino: 0,
            },
            data: None,
            command: None,
            symlink: Some(target),
            linkname: None,
        }
    }

    /// Create a new process pseudo device
    pub fn new_process(command: String, mode: mode_t, uid: uid_t, gid: gid_t) -> Self {
        Self {
            type_: PseudoType::Regular,
            pseudo_type: PseudoFileType::Process,
            stat: PseudoStat {
                mode: mode | libc::S_IFREG,
                uid,
                gid,
                major: 0,
                minor: 0,
                mtime: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as time_t,
                ino: 0,
            },
            data: None,
            command: Some(command),
            symlink: None,
            linkname: None,
        }
    }

    /// Create a new data pseudo device
    pub fn new_data(file: Arc<PseudoFile>, offset: i64, length: i64, sparse: bool) -> Self {
        Self {
            type_: PseudoType::Regular,
            pseudo_type: PseudoFileType::Data,
            stat: PseudoStat {
                mode: libc::S_IFREG | 0o644,
                uid: 0,
                gid: 0,
                major: 0,
                minor: 0,
                mtime: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as time_t,
                ino: 0,
            },
            data: Some(PseudoData {
                file,
                offset,
                length,
                sparse,
            }),
            command: None,
            symlink: None,
            linkname: None,
        }
    }

    /// Compare two pseudo devices for equality
    fn eq(&self, other: &PseudoDev) -> bool {
        self.type_ == other.type_ &&
        self.pseudo_type == other.pseudo_type &&
        self.stat.mode == other.stat.mode &&
        self.stat.uid == other.stat.uid &&
        self.stat.gid == other.stat.gid &&
        self.stat.major == other.stat.major &&
        self.stat.minor == other.stat.minor &&
        self.command == other.command &&
        self.symlink == other.symlink &&
        self.linkname == other.linkname
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pseudo_creation() {
        let pseudo = Pseudo::new();
        assert_eq!(pseudo.names, 0);
        assert!(pseudo.current.is_none());
        assert!(pseudo.head.is_none());
    }

    #[test]
    fn test_pseudo_add_entry() {
        let mut pseudo = Pseudo::new();
        let dev = PseudoDev::new_directory(0o755, 0, 0);
        pseudo.add_pseudo(dev, "/test", "/test").unwrap();
        
        assert_eq!(pseudo.names, 1);
        assert!(pseudo.head.is_some());
        assert_eq!(pseudo.head.as_ref().unwrap().name, "test");
    }

    #[test]
    fn test_pseudo_add_nested() {
        let mut pseudo = Pseudo::new();
        let dev = PseudoDev::new_directory(0o755, 0, 0);
        pseudo.add_pseudo(dev, "/test/dir", "/test/dir").unwrap();
        
        assert_eq!(pseudo.names, 1);
        assert!(pseudo.head.is_some());
        assert_eq!(pseudo.head.as_ref().unwrap().name, "test");
        assert!(pseudo.head.as_ref().unwrap().pseudo.is_some());
        assert_eq!(pseudo.head.as_ref().unwrap().pseudo.as_ref().unwrap().names, 1);
    }
} 