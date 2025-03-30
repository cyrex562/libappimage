use std::collections::HashMap;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use crate::error::{SquashError, Result};
use crate::fs::{SquashfsSuperBlock, SquashfsInode};
use crate::alloc::{safe_malloc, safe_free};
use crate::read::read_block;

const READAHEAD_SIZE: usize = 8192;
const READAHEAD_ALLOC: usize = 0x100000;
const READAHEAD_INDEX_MASK: u64 = 0xfffff;
const READAHEAD_OFFSET_MASK: u64 = (READAHEAD_SIZE - 1) as u64;

#[derive(Debug)]
pub struct Readahead {
    start: i64,
    size: usize,
    next: Option<Box<Readahead>>,
    src: *mut u8,
    data: [u8; 0], // Flexible array member
}

#[derive(Debug)]
pub struct ReadEntry {
    pub dir_ent: Arc<DirEnt>,
    pub file_count: u32,
}

#[derive(Debug)]
pub struct Reader {
    pub id: usize,
    pub size: usize,
    pub type_: String,
    pub pathname: Option<String>,
    pub buffer: Arc<Mutex<Buffer>>,
}

#[derive(Debug)]
pub struct Buffer {
    data: Vec<u8>,
    size: usize,
    capacity: usize,
}

#[derive(Debug)]
pub struct DirEnt {
    pub name: String,
    pub inode: Arc<InodeInfo>,
    pub next: Option<Arc<DirEnt>>,
    pub dir: Option<Arc<DirInfo>>,
    pub nonstandard_pathname: Option<String>,
}

#[derive(Debug)]
pub struct InodeInfo {
    pub buf: std::fs::Metadata,
    pub noD: bool,
    pub noF: bool,
    pub no_fragments: bool,
    pub always_use_fragments: bool,
    pub scanned: bool,
    pub root_entry: bool,
    pub pseudo: Option<Arc<PseudoFile>>,
}

#[derive(Debug)]
pub struct DirInfo {
    pub pathname: String,
    pub list: Option<Arc<DirEnt>>,
}

#[derive(Debug)]
pub struct PseudoFile {
    pub filename: String,
    pub fd: i32,
    pub current: i64,
    pub start: i64,
    pub data: Option<Arc<PseudoData>>,
}

#[derive(Debug)]
pub struct PseudoData {
    pub length: i64,
    pub offset: i64,
    pub file: Arc<PseudoFile>,
}

pub struct ReaderManager {
    readers: Vec<Reader>,
    reader_threads: Vec<thread::JoinHandle<()>>,
    readahead_table: Vec<Option<Box<Readahead>>>,
    block_array: Vec<Option<ReadEntry>>,
    fragment_array: Vec<Option<ReadEntry>>,
    file_count: u32,
    block_count: u32,
    fragment_count: u32,
    total_rblocks: usize,
    total_rmbytes: usize,
    total_wblocks: usize,
    total_wmbytes: usize,
    sleep_time: u32,
    single_threaded: bool,
    fragment_threads: usize,
    block_threads: usize,
}

impl ReaderManager {
    pub fn new() -> Self {
        Self {
            readers: Vec::new(),
            reader_threads: Vec::new(),
            readahead_table: vec![None; READAHEAD_ALLOC],
            block_array: Vec::new(),
            fragment_array: Vec::new(),
            file_count: 0,
            block_count: 0,
            fragment_count: 0,
            total_rblocks: 0,
            total_rmbytes: 0,
            total_wblocks: 0,
            total_wmbytes: 0,
            sleep_time: 0,
            single_threaded: true,
            fragment_threads: 0,
            block_threads: 1,
        }
    }

    pub fn set_read_frag_threads(&mut self, fragments: usize) {
        self.fragment_threads = fragments;
        self.reader_threads = self.fragment_threads + self.block_threads;
    }

    pub fn set_read_block_threads(&mut self, blocks: usize) {
        self.block_threads = blocks;
        self.reader_threads = self.fragment_threads + self.block_threads;
    }

    pub fn set_single_threaded(&mut self) {
        self.single_threaded = true;
        self.reader_threads = 1;
        self.block_threads = 1;
        self.fragment_threads = 0;
    }

    pub fn get_reader_num(&self) -> usize {
        self.reader_threads
    }

    pub fn set_sleep_time(&mut self, time: u32) {
        self.sleep_time = time;
    }

    pub fn check_min_memory(&mut self, rmbytes: usize, wmbytes: usize, block_log: u32) -> Result<()> {
        let rblocks = rmbytes << (20 - block_log);
        let wblocks = wmbytes << (20 - block_log);
        let per_rthread = rblocks / self.reader_threads;
        let total_fwthread = num_cpus::get() * self.fragment_threads;
        let per_wthread = (wblocks - total_fwthread) / self.block_threads;

        if per_wthread < num_cpus::get() || per_rthread < 4 { // 4 is BLOCKS_MIN
            let twblocks = total_fwthread + num_cpus::get() * self.block_threads;
            let twmbytes = if twblocks >> (20 - block_log) > 0 { twblocks >> (20 - block_log) } else { 1 };
            let twmin_mem = twmbytes * 8; // SQUASHFS_BWRITEQ_MEM
            let trblocks = 4 * self.reader_threads; // 4 is BLOCKS_MIN
            let trmbytes = if trblocks >> (20 - block_log) > 0 { trblocks >> (20 - block_log) } else { 1 };
            let trmin_mem = trmbytes * 8; // SQUASHFS_READQ_MEM
            let reader_only = twmin_mem <= trmin_mem;
            let min_mem = if reader_only { trmin_mem } else { twmin_mem };

            return Err(SquashError::Other(format!(
                "Insufficient memory for specified options! Please increase memory to {} Mbytes (-mem option)",
                min_mem
            )));
        }

        self.total_rblocks = rblocks;
        self.total_rmbytes = rmbytes;
        self.total_wblocks = wblocks;
        self.total_wmbytes = wmbytes;

        Ok(())
    }

    fn readahead_index(start: i64) -> usize {
        ((start >> 13) & READAHEAD_INDEX_MASK) as usize
    }

    fn readahead_offset(start: i64) -> usize {
        (start & READAHEAD_OFFSET_MASK) as usize
    }

    fn remove_readahead(&mut self, index: usize, prev: Option<&mut Box<Readahead>>, new: &mut Box<Readahead>) {
        if let Some(prev) = prev {
            prev.next = new.next.take();
        } else {
            self.readahead_table[index] = new.next.take();
        }
    }

    fn add_readahead(&mut self, new: Box<Readahead>) {
        let index = Self::readahead_index(new.start);
        let mut current = self.readahead_table[index].take();
        new.next = current;
        self.readahead_table[index] = Some(new);
    }

    fn get_readahead(&mut self, file: &mut PseudoFile, current: i64, file_buffer: &mut Buffer, size: usize) -> io::Result<usize> {
        if self.readahead_table.is_empty() {
            return Ok(0);
        }

        let mut count = size;
        let mut dest = file_buffer.data.as_mut_ptr();

        while size > 0 {
            let index = Self::readahead_index(current);
            let mut buffer = self.readahead_table[index].take();
            let mut prev = None;

            while let Some(mut buf) = buffer {
                if buf.start <= current && buf.start + buf.size as i64 > current {
                    let offset = Self::readahead_offset(current);
                    let buffer_offset = Self::readahead_offset(buf.start);

                    if offset == buffer_offset && size >= buf.size {
                        unsafe {
                            std::ptr::copy_nonoverlapping(buf.src, dest, buf.size);
                        }
                        dest = dest.add(buf.size);
                        current += buf.size as i64;
                        size -= buf.size;

                        self.remove_readahead(index, prev.as_deref_mut(), &mut buf);
                        break;
                    } else if offset == buffer_offset {
                        unsafe {
                            std::ptr::copy_nonoverlapping(buf.src, dest, size);
                        }
                        buf.start += size as i64;
                        buf.src = buf.src.add(size);
                        buf.size -= size;

                        self.remove_readahead(index, prev.as_deref_mut(), &mut buf);
                        self.add_readahead(buf);
                        return Ok(count);
                    } else if buffer_offset + buf.size <= offset + size {
                        let bytes = buffer_offset + buf.size - offset;
                        unsafe {
                            std::ptr::copy_nonoverlapping(buf.src.add(offset - buffer_offset), dest, bytes);
                        }
                        buf.size -= bytes;
                        dest = dest.add(bytes);
                        size -= bytes;
                        current += bytes as i64;
                        break;
                    } else {
                        let left_size = offset - buffer_offset;
                        let right_size = buf.size - (offset + size);

                        unsafe {
                            std::ptr::copy_nonoverlapping(buf.src.add(offset - buffer_offset), dest, size);
                        }

                        let mut left = Box::new(Readahead {
                            start: buf.start,
                            size: left_size,
                            next: None,
                            src: buf.src,
                            data: [],
                        });

                        let mut right = Box::new(Readahead {
                            start: current + size as i64,
                            size: right_size,
                            next: None,
                            src: buf.src.add(offset + size),
                            data: [],
                        });

                        self.remove_readahead(index, prev.as_deref_mut(), &mut buf);
                        self.add_readahead(left);
                        self.add_readahead(right);
                        return Ok(count);
                    }
                }
                prev = Some(&mut buf);
                buffer = buf.next;
            }

            if buffer.is_none() {
                return Ok(0);
            }
        }

        Ok(count)
    }

    fn do_readahead(&mut self, file: &mut PseudoFile, current: i64, file_buffer: &mut Buffer, size: usize) -> io::Result<usize> {
        let readahead = current - file.current;

        if self.readahead_table.is_empty() {
            self.readahead_table = vec![None; READAHEAD_ALLOC];
        }

        let mut readahead_remaining = readahead;
        while readahead_remaining > 0 {
            let offset = Self::readahead_offset(file.current);
            let bytes = (READAHEAD_SIZE - offset).min(readahead_remaining as usize);
            let mut buffer = Box::new(Readahead {
                start: file.current,
                size: bytes,
                next: None,
                src: std::ptr::null_mut(),
                data: [],
            });

            let res = unsafe {
                std::fs::File::open(&file.filename)?
                    .read_exact(std::slice::from_raw_parts_mut(buffer.data.as_mut_ptr(), bytes))?
            };

            if res.is_err() {
                return Err(res.unwrap_err());
            }

            buffer.src = buffer.data.as_mut_ptr();
            self.add_readahead(buffer);

            file.current += bytes as i64;
            readahead_remaining -= bytes as i64;
        }

        let res = unsafe {
            std::fs::File::open(&file.filename)?
                .read_exact(std::slice::from_raw_parts_mut(file_buffer.data.as_mut_ptr(), size))?
        };

        if res.is_ok() {
            file.current += size as i64;
        }

        res
    }

    fn read_data(&mut self, file: &mut PseudoFile, current: i64, file_buffer: &mut Buffer, size: usize) -> io::Result<usize> {
        if file.fd != -1 {
            if current != file.current {
                std::fs::File::open(&file.filename)?
                    .seek(SeekFrom::Start((current + file.start) as u64))?;
                file.current = current;
            }

            let res = unsafe {
                std::fs::File::open(&file.filename)?
                    .read_exact(std::slice::from_raw_parts_mut(file_buffer.data.as_mut_ptr(), size))?
            };

            if res.is_ok() {
                file.current += size as i64;
            }

            return res;
        }

        if current == file.current {
            let res = self.do_readahead(file, current, file_buffer, size);
            if res.is_ok() {
                file.current += size as i64;
            }
            return res;
        } else if current < file.current {
            return self.get_readahead(file, current, file_buffer, size);
        } else {
            return self.do_readahead(file, current, file_buffer, size);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reader_manager_creation() {
        let manager = ReaderManager::new();
        assert_eq!(manager.reader_threads, 0);
        assert_eq!(manager.block_threads, 1);
        assert_eq!(manager.fragment_threads, 0);
        assert!(manager.single_threaded);
    }

    #[test]
    fn test_readahead_index() {
        assert_eq!(ReaderManager::readahead_index(0), 0);
        assert_eq!(ReaderManager::readahead_index(8192), 1);
        assert_eq!(ReaderManager::readahead_index(16384), 2);
    }

    #[test]
    fn test_readahead_offset() {
        assert_eq!(ReaderManager::readahead_offset(0), 0);
        assert_eq!(ReaderManager::readahead_offset(8192), 0);
        assert_eq!(ReaderManager::readahead_offset(8193), 1);
    }
} 