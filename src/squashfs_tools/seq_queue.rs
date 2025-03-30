use std::sync::{Arc, Mutex, Condvar};
use std::collections::HashMap;
use super::FileBuffer;

const HASH_SIZE: usize = 65536;

fn calculate_hash(n: i64) -> usize {
    (n & 0xffff) as usize
}

pub struct SeqQueue {
    version: Arc<Mutex<u16>>,
    fragment_count: Arc<Mutex<i32>>,
    block_count: Arc<Mutex<i32>>,
    sequence: Arc<Mutex<i64>>,
    file_count: Arc<Mutex<i64>>,
    block: Arc<Mutex<i64>>,
    hash_table: Arc<Mutex<HashMap<usize, Vec<FileBuffer>>>>,
    mutex: Arc<Mutex<()>>,
    wait: Arc<Condvar>,
}

impl SeqQueue {
    pub fn new() -> Self {
        SeqQueue {
            version: Arc::new(Mutex::new(0)),
            fragment_count: Arc::new(Mutex::new(0)),
            block_count: Arc::new(Mutex::new(0)),
            sequence: Arc::new(Mutex::new(0)),
            file_count: Arc::new(Mutex::new(0)),
            block: Arc::new(Mutex::new(0)),
            hash_table: Arc::new(Mutex::new(HashMap::new())),
            mutex: Arc::new(Mutex::new(())),
            wait: Arc::new(Condvar::new()),
        }
    }

    pub fn flush(&self) {
        let _guard = self.mutex.lock().unwrap();
        let mut hash_table = self.hash_table.lock().unwrap();
        hash_table.clear();
        
        *self.fragment_count.lock().unwrap() = 0;
        *self.block_count.lock().unwrap() = 0;
    }

    pub fn main_queue_put(&self, entry: FileBuffer) {
        let _guard = self.mutex.lock().unwrap();
        let hash = calculate_hash(entry.file_count);
        
        let mut hash_table = self.hash_table.lock().unwrap();
        hash_table.entry(hash).or_insert_with(Vec::new).push(entry.clone());
        
        if entry.fragment {
            *self.fragment_count.lock().unwrap() += 1;
        } else {
            *self.block_count.lock().unwrap() += 1;
        }

        let file_count = *self.file_count.lock().unwrap();
        let block = *self.block.lock().unwrap();
        let version = *self.version.lock().unwrap();

        if entry.file_count == file_count && entry.block == block && entry.version == version {
            self.wait.notify_one();
        }
    }

    pub fn main_queue_get(&self) -> FileBuffer {
        let _guard = self.mutex.lock().unwrap();
        
        loop {
            let file_count = *self.file_count.lock().unwrap();
            let block = *self.block.lock().unwrap();
            let version = *self.version.lock().unwrap();
            let hash = calculate_hash(file_count);
            
            let hash_table = self.hash_table.lock().unwrap();
            if let Some(entries) = hash_table.get(&hash) {
                if let Some(pos) = entries.iter().position(|e| 
                    e.file_count == file_count && 
                    e.block == block && 
                    e.version == version
                ) {
                    let mut entries = entries.clone();
                    let entry = entries.remove(pos);
                    
                    if entry.fragment {
                        *self.fragment_count.lock().unwrap() -= 1;
                    } else {
                        *self.block_count.lock().unwrap() -= 1;
                    }

                    match entry.next_state {
                        super::NextState::NextVersion => {
                            *self.version.lock().unwrap() += 1;
                            *self.block.lock().unwrap() = 0;
                        }
                        super::NextState::NextBlock => {
                            *self.block.lock().unwrap() += 1;
                        }
                        super::NextState::NextFile => {
                            *self.version.lock().unwrap() = 0;
                            *self.block.lock().unwrap() = 0;
                            *self.file_count.lock().unwrap() += 1;
                        }
                    }

                    return entry;
                }
            }

            self.wait.wait(_guard).unwrap();
        }
    }

    pub fn fragment_queue_put(&self, entry: FileBuffer) {
        let _guard = self.mutex.lock().unwrap();
        let hash = calculate_hash(entry.sequence);
        
        let mut hash_table = self.hash_table.lock().unwrap();
        hash_table.entry(hash).or_insert_with(Vec::new).push(entry.clone());
        
        if entry.fragment {
            *self.fragment_count.lock().unwrap() += 1;
        } else {
            *self.block_count.lock().unwrap() += 1;
        }

        let sequence = *self.sequence.lock().unwrap();
        if entry.sequence == sequence {
            self.wait.notify_one();
        }
    }

    pub fn fragment_queue_get(&self) -> FileBuffer {
        let _guard = self.mutex.lock().unwrap();
        
        loop {
            let sequence = *self.sequence.lock().unwrap();
            let hash = calculate_hash(sequence);
            
            let hash_table = self.hash_table.lock().unwrap();
            if let Some(entries) = hash_table.get(&hash) {
                if let Some(pos) = entries.iter().position(|e| e.sequence == sequence) {
                    let mut entries = entries.clone();
                    let entry = entries.remove(pos);
                    
                    if entry.fragment {
                        *self.fragment_count.lock().unwrap() -= 1;
                    } else {
                        *self.block_count.lock().unwrap() -= 1;
                    }

                    *self.sequence.lock().unwrap() += 1;
                    return entry;
                }
            }

            self.wait.wait(_guard).unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_main_queue() {
        let queue = SeqQueue::new();
        
        let mut entry = FileBuffer::new(1024);
        entry.file_count = 1;
        entry.block = 0;
        entry.version = 0;
        entry.fragment = false;
        
        queue.main_queue_put(entry.clone());
        let result = queue.main_queue_get();
        
        assert_eq!(result.file_count, 1);
        assert_eq!(result.block, 0);
        assert_eq!(result.version, 0);
    }

    #[test]
    fn test_fragment_queue() {
        let queue = SeqQueue::new();
        
        let mut entry = FileBuffer::new(1024);
        entry.sequence = 1;
        entry.fragment = true;
        
        queue.fragment_queue_put(entry.clone());
        let result = queue.fragment_queue_get();
        
        assert_eq!(result.sequence, 1);
        assert!(result.fragment);
    }
} 