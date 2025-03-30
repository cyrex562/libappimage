use std::collections::HashMap;
use std::fs;
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use libc::{dev_t, ino_t};
use crate::error::{SquashError, Result};
use crate::fs::{DirInfo, DirEnt, InodeInfo, FileInfo};
use crate::progress::ProgressBar;

const MAX_LINE: usize = 16384;
const PRIORITY_RANGE: i32 = 32768;

#[derive(Debug)]
pub struct PriorityEntry {
    pub dir: Arc<DirEnt>,
    pub next: Option<Arc<PriorityEntry>>,
}

#[derive(Debug)]
pub struct SortInfo {
    pub st_dev: dev_t,
    pub st_ino: ino_t,
    pub priority: i32,
    pub next: Option<Arc<SortInfo>>,
}

pub struct SortManager {
    sort_info_list: Vec<Option<Arc<SortInfo>>>,
    priority_list: Vec<Option<Arc<PriorityEntry>>>,
    mkisofs_style: Option<bool>,
    hardlnk_count: i64,
}

impl SortManager {
    pub fn new() -> Self {
        Self {
            sort_info_list: vec![None; 65536],
            priority_list: vec![None; 65536],
            mkisofs_style: None,
            hardlnk_count: 0,
        }
    }

    fn add_priority_list(&mut self, dir: Arc<DirEnt>, priority: i32) {
        let priority = priority + PRIORITY_RANGE;
        let new_entry = Arc::new(PriorityEntry {
            dir,
            next: self.priority_list[priority as usize].take(),
        });
        self.priority_list[priority as usize] = Some(new_entry);
    }

    fn get_priority(&self, filename: &str, buf: &fs::Metadata, priority: i32) -> i32 {
        let hash = (buf.ino() & 0xffff) as usize;
        let mut current = self.sort_info_list[hash].as_ref();

        while let Some(s) = current {
            if s.st_dev == buf.dev() && s.st_ino == buf.ino() {
                return s.priority;
            }
            current = s.next.as_ref();
        }

        priority
    }

    fn add_sort_list(&mut self, path: &str, priority: i32, source: usize, source_path: &[String]) -> Result<bool> {
        let path = if path.ends_with("/*") {
            &path[..path.len() - 2]
        } else {
            path
        };

        if path.starts_with('/') || path.starts_with("./") || path.starts_with("../") || self.mkisofs_style == Some(true) {
            match fs::symlink_metadata(path) {
                Ok(buf) => {
                    let hash = (buf.ino() & 0xffff) as usize;
                    let sort_info = Arc::new(SortInfo {
                        st_dev: buf.dev() as dev_t,
                        st_ino: buf.ino() as ino_t,
                        priority,
                        next: self.sort_info_list[hash].take(),
                    });
                    self.sort_info_list[hash] = Some(sort_info);
                    return Ok(true);
                }
                Err(e) => {
                    if e.kind() != io::ErrorKind::NotFound {
                        return Err(SquashError::Other(format!("Failed to stat sortlist entry: {}", e)));
                    }
                }
            }
        }

        let mut found_count = 0;
        for source_path in source_path.iter() {
            let filename = PathBuf::from(source_path).join(path);
            match fs::symlink_metadata(&filename) {
                Ok(buf) => {
                    let hash = (buf.ino() & 0xffff) as usize;
                    let sort_info = Arc::new(SortInfo {
                        st_dev: buf.dev() as dev_t,
                        st_ino: buf.ino() as ino_t,
                        priority,
                        next: self.sort_info_list[hash].take(),
                    });
                    self.sort_info_list[hash] = Some(sort_info);
                    found_count += 1;
                }
                Err(e) => {
                    if e.kind() != io::ErrorKind::NotFound {
                        return Err(SquashError::Other(format!("Failed to stat sortlist entry: {}", e)));
                    }
                }
            }
        }

        if found_count == 0 && self.mkisofs_style.is_none() {
            if fs::symlink_metadata(path).is_ok() {
                eprintln!("WARNING: Mkisofs style sortlist detected! This is supported but please");
                eprintln!("convert to mksquashfs style sortlist! A sortlist entry should be");
                eprintln!("either absolute (starting with '/') start with './' or '../' (taken to be");
                eprintln!("relative to $PWD), otherwise it is assumed the entry is relative to one");
                eprintln!("of the source directories.");
                self.mkisofs_style = Some(true);
                return self.add_sort_list(path, priority, source, source_path);
            }
        }

        self.mkisofs_style = Some(false);

        if found_count == 1 {
            Ok(true)
        } else if found_count > 1 {
            Err(SquashError::Other(format!(
                "Ambiguous sortlist entry \"{}\"\nIt maps to more than one source entry! Please use an absolute path.",
                path
            )))
        } else {
            Ok(true) // Historical behavior: ignore missing entries
        }
    }

    pub fn generate_file_priorities(&mut self, dir: &DirInfo, priority: i32, buf: &fs::Metadata) {
        let priority = self.get_priority(&dir.pathname, buf, priority);
        let mut current = dir.list.as_ref();

        while let Some(dir_ent) = current {
            if !dir_ent.inode.root_entry {
                match dir_ent.inode.buf.file_type() {
                    fs::FileType::File => {
                        self.add_priority_list(
                            dir_ent.clone(),
                            self.get_priority(&dir_ent.name, &dir_ent.inode.buf, priority),
                        );
                    }
                    fs::FileType::Directory => {
                        self.generate_file_priorities(&dir_ent.dir.as_ref().unwrap(), priority, &dir_ent.inode.buf);
                    }
                    _ => {}
                }
            }
            current = dir_ent.next.as_ref();
        }
    }

    pub fn read_sort_file(&mut self, filename: &str, source: usize, source_path: &[String]) -> Result<bool> {
        let file = fs::File::open(filename)?;
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line?;
            if line.len() > MAX_LINE {
                return Err(SquashError::Other(format!(
                    "Line too long when reading sort file \"{}\", larger than {} bytes",
                    filename, MAX_LINE
                )));
            }

            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let mut parts = line.splitn(2, char::is_whitespace);
            let filename = parts.next().unwrap_or_default();
            let priority_str = parts.next().unwrap_or_default();

            if filename.is_empty() {
                continue;
            }

            let priority: i32 = priority_str.parse().map_err(|e| {
                SquashError::Other(format!(
                    "Invalid priority in sort file \"{}\": {}",
                    filename, e
                ))
            })?;

            if priority < -32768 || priority > 32767 {
                return Err(SquashError::Other(format!(
                    "Sort file \"{}\", entry \"{}\" has priority outside range of -32767:32768",
                    filename, line
                )));
            }

            if !parts.next().is_none() {
                return Err(SquashError::Other(format!(
                    "Sort file \"{}\", trailing characters after priority in entry \"{}\"",
                    filename, line
                )));
            }

            self.add_sort_list(filename, priority, source, source_path)?;
        }

        Ok(true)
    }

    pub fn sort_files_and_write(&mut self, dir: &DirInfo, progress_bar: &ProgressBar) -> Result<()> {
        for i in (0..65536).rev() {
            let mut current = self.priority_list[i].take();
            while let Some(entry) = current {
                let priority = i as i32 - PRIORITY_RANGE;
                if entry.dir.inode.inode == 0 { // SQUASHFS_INVALID_BLK
                    let (file, duplicate_file) = self.write_file(&entry.dir)?;
                    let inode = self.create_inode(
                        None,
                        &entry.dir,
                        "SQUASHFS_FILE_TYPE",
                        file.file_size,
                        file.start,
                        file.blocks,
                        file.block_list,
                        file.fragment,
                        None,
                        file.sparse,
                    )?;

                    if !self.duplicate_checking {
                        self.free_fragment(file.fragment);
                        self.free(file.block_list);
                    }

                    println!(
                        "file {}, uncompressed size {} bytes {}\n",
                        entry.dir.name,
                        entry.dir.inode.buf.len(),
                        if duplicate_file { "DUPLICATE" } else { "" }
                    );

                    entry.dir.inode.inode = inode;
                    entry.dir.inode.type_ = "SQUASHFS_FILE_TYPE".to_string();
                    self.hardlnk_count -= 1;
                } else {
                    println!(
                        "file {}, uncompressed size {} bytes LINK\n",
                        entry.dir.name,
                        entry.dir.inode.buf.len()
                    );
                }
                current = entry.next;
            }
        }
        Ok(())
    }

    // Helper functions that need to be implemented
    fn write_file(&self, dir_ent: &DirEnt) -> Result<(FileInfo, bool)> {
        // TODO: Implement write_file
        unimplemented!()
    }

    fn create_inode(
        &self,
        parent: Option<&DirEnt>,
        dir_ent: &DirEnt,
        type_: &str,
        file_size: u64,
        start: u64,
        blocks: u32,
        block_list: Vec<u64>,
        fragment: Option<u32>,
        xattr: Option<u32>,
        sparse: bool,
    ) -> Result<u64> {
        // TODO: Implement create_inode
        unimplemented!()
    }

    fn free_fragment(&self, fragment: Option<u32>) {
        // TODO: Implement free_fragment
        unimplemented!()
    }

    fn free(&self, block_list: Vec<u64>) {
        // TODO: Implement free
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sort_manager_creation() {
        let manager = SortManager::new();
        assert_eq!(manager.sort_info_list.len(), 65536);
        assert_eq!(manager.priority_list.len(), 65536);
        assert!(manager.mkisofs_style.is_none());
        assert_eq!(manager.hardlnk_count, 0);
    }

    #[test]
    fn test_priority_range() {
        let mut manager = SortManager::new();
        let dir = Arc::new(DirEnt {
            name: "test".to_string(),
            inode: Arc::new(InodeInfo {
                buf: fs::metadata(".").unwrap(),
                root_entry: false,
                inode: 0,
                type_: "".to_string(),
                dir: None,
            }),
            next: None,
            dir: None,
        });

        manager.add_priority_list(dir.clone(), 0);
        assert!(manager.priority_list[32768].is_some());
    }
} 