use std::path::Path;
use std::ffi::CString;
use std::os::unix::fs::MetadataExt;
use std::sync::Arc;
use std::sync::Mutex;
use std::collections::HashMap;
use crate::error::{ErrorState, SquashError};
use crate::xattr::{Xattr, XattrList};

/// Structure representing directory information
#[derive(Debug)]
pub struct DirInfo {
    /// Pathname of the directory
    pub pathname: String,
    /// Subpath within the filesystem
    pub subpath: Option<String>,
    /// Number of entries in the directory
    pub count: u32,
    /// Number of subdirectories
    pub directory_count: u32,
    /// Depth in the directory tree
    pub depth: u32,
    /// Whether this directory is excluded
    pub excluded: bool,
    /// Whether this is a large directory
    pub dir_is_ldir: bool,
    /// Directory entries
    pub entries: Vec<DirEntry>,
}

/// Structure representing a directory entry
#[derive(Debug)]
pub struct DirEntry {
    /// Name of the entry
    pub name: String,
    /// Source name (for hard links)
    pub source_name: Option<String>,
    /// Non-standard pathname
    pub nonstandard_pathname: Option<String>,
    /// Inode information
    pub inode: Arc<InodeInfo>,
    /// Parent directory
    pub dir: Arc<DirInfo>,
    /// Directory containing this entry
    pub our_dir: Arc<DirInfo>,
    /// Next entry in the directory
    pub next: Option<Arc<DirEntry>>,
    /// Next entry for reader thread
    pub reader_next: Option<Arc<DirEntry>>,
}

/// Structure representing inode information
#[derive(Debug)]
pub struct InodeInfo {
    /// File metadata
    pub metadata: std::fs::Metadata,
    /// Next inode in hash chain
    pub next: Option<Arc<InodeInfo>>,
    /// Pseudo device information
    pub pseudo: Option<Arc<PseudoDev>>,
    /// Tar file information
    pub tar_file: Option<Arc<TarFile>>,
    /// Extended attributes
    pub xattr: Option<XattrList>,
    /// SquashFS inode number
    pub inode: u64,
    /// Inode number
    pub inode_number: u32,
    /// Number of hard links
    pub nlink: u32,
    /// Whether this is a dummy root directory
    pub dummy_root_dir: bool,
    /// File type
    pub file_type: FileType,
    /// Whether this is a root entry
    pub root_entry: bool,
    /// Whether to not use fragments
    pub no_fragments: bool,
    /// Whether to always use fragments
    pub always_use_fragments: bool,
    /// Whether to not use deduplication
    pub no_dedup: bool,
    /// Whether to not use fragments
    pub no_frag: bool,
    /// Whether this is from a tar file
    pub tarfile: bool,
    /// Whether this has been read
    pub read: bool,
    /// Whether this has been scanned
    pub scanned: bool,
    /// Symlink target (if applicable)
    pub symlink: Option<String>,
}

/// Structure representing file information
#[derive(Debug)]
pub struct FileInfo {
    /// Total file size
    pub file_size: i64,
    /// Size in bytes
    pub bytes: i64,
    /// Starting block
    pub start: i64,
    /// Number of sparse blocks
    pub sparse: i64,
    /// List of blocks
    pub block_list: Vec<u32>,
    /// Next fragment
    pub frag_next: Option<Arc<FileInfo>>,
    /// Next block
    pub block_next: Option<Arc<FileInfo>>,
    /// Fragment information
    pub fragment: Option<Fragment>,
    /// Duplicate information
    pub dup: Option<DupInfo>,
    /// Number of blocks
    pub blocks: u32,
    /// File checksum
    pub checksum: u16,
    /// Fragment checksum
    pub fragment_checksum: u16,
    /// Whether fragment checksum is present
    pub have_frag_checksum: bool,
    /// Whether file checksum is present
    pub have_checksum: bool,
}

/// Structure representing fragment information
#[derive(Debug)]
pub struct Fragment {
    /// Fragment index
    pub index: u32,
    /// Offset within fragment
    pub offset: i32,
    /// Fragment size
    pub size: i32,
}

/// Structure representing duplicate information
#[derive(Debug)]
pub struct DupInfo {
    /// Original file
    pub file: Arc<FileInfo>,
    /// Fragment information
    pub frag: Option<Arc<FileInfo>>,
    /// Next duplicate
    pub next: Option<Arc<DupInfo>>,
}

/// Enum representing file types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FileType {
    Regular,
    Directory,
    Symlink,
    CharacterDevice,
    BlockDevice,
    Fifo,
    Socket,
}

/// Structure representing a pseudo device
#[derive(Debug)]
pub struct PseudoDev {
    /// Device metadata
    pub metadata: std::fs::Metadata,
    /// Device type
    pub dev_type: FileType,
}

/// Structure representing a tar file
#[derive(Debug)]
pub struct TarFile {
    /// File information
    pub file: Option<Arc<FileInfo>>,
    /// Next tar file
    pub next: Option<Arc<TarFile>>,
}

impl DirInfo {
    /// Create a new directory info structure
    pub fn new(pathname: String, subpath: Option<String>, depth: u32) -> Self {
        Self {
            pathname,
            subpath,
            count: 0,
            directory_count: 0,
            depth,
            excluded: false,
            dir_is_ldir: false,
            entries: Vec::new(),
        }
    }

    /// Add a directory entry
    pub fn add_entry(&mut self, entry: DirEntry) {
        self.count += 1;
        if matches!(entry.inode.file_type, FileType::Directory) {
            self.directory_count += 1;
        }
        self.entries.push(entry);
    }
}

impl InodeInfo {
    /// Create a new inode info structure
    pub fn new(metadata: std::fs::Metadata, file_type: FileType) -> Self {
        Self {
            metadata,
            next: None,
            pseudo: None,
            tar_file: None,
            xattr: None,
            inode: 0,
            inode_number: 0,
            nlink: 1,
            dummy_root_dir: false,
            file_type,
            root_entry: false,
            no_fragments: false,
            always_use_fragments: false,
            no_dedup: false,
            no_frag: false,
            tarfile: false,
            read: false,
            scanned: false,
            symlink: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    

    #[test]
    fn test_dir_info() {
        let mut dir = DirInfo::new("test".to_string(), None, 0);
        assert_eq!(dir.count, 0);
        assert_eq!(dir.directory_count, 0);

        let metadata = std::fs::metadata(".").unwrap();
        let inode = Arc::new(InodeInfo::new(metadata, FileType::Directory));
        let dir_entry = DirEntry {
            name: "test".to_string(),
            source_name: None,
            nonstandard_pathname: None,
            inode: inode.clone(),
            dir: Arc::new(DirInfo::new("parent".to_string(), None, 0)),
            our_dir: Arc::new(dir.clone()),
            next: None,
            reader_next: None,
        };

        dir.add_entry(dir_entry);
        assert_eq!(dir.count, 1);
        assert_eq!(dir.directory_count, 1);
    }
} 