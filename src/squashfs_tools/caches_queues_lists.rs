use std::sync::{Arc, Mutex, Condvar};
use std::sync::atomic::{AtomicI32, Ordering};
use std::collections::HashMap;
use std::ptr;

const HASH_SIZE: usize = 65536;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NextState {
    NextBlock,
    NextFile,
    NextVersion,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CacheType {
    QueueCache,
    GenCache,
}

#[derive(Debug)]
pub struct FileBuffer {
    pub index: i64,
    pub sequence: i64,
    pub file_size: i64,
    pub block: i64,
    pub size: i32,
    pub c_byte: i32,
    pub checksum: u16,
    pub version: u16,
    pub thread: u16,
    pub used: bool,
    pub fragment: bool,
    pub error: bool,
    pub locked: bool,
    pub wait_on_unlock: bool,
    pub no_d: bool,
    pub duplicate: bool,
    pub next_state: NextState,
    pub cache_type: CacheType,
    pub hashed: bool,
    pub data: Vec<u8>,
}

impl FileBuffer {
    pub fn new(size: usize) -> Self {
        FileBuffer {
            index: 0,
            sequence: 0,
            file_size: 0,
            block: 0,
            size: 0,
            c_byte: 0,
            checksum: 0,
            version: 0,
            thread: 0,
            used: false,
            fragment: false,
            error: false,
            locked: false,
            wait_on_unlock: false,
            no_d: false,
            duplicate: false,
            next_state: NextState::NextBlock,
            cache_type: CacheType::GenCache,
            hashed: false,
            data: vec![0; size],
        }
    }
}

pub trait Queue {
    fn put(&self, data: Box<dyn std::any::Any>) -> Result<(), QueueError>;
    fn get(&self) -> Result<Box<dyn std::any::Any>, QueueError>;
    fn empty(&self) -> bool;
    fn flush(&self);
}

#[derive(Debug)]
pub enum QueueError {
    Full,
    Empty,
    LockError,
    ThreadError,
}

pub struct CircularQueue {
    size: usize,
    read_pos: Arc<AtomicI32>,
    write_pos: Arc<AtomicI32>,
    data: Vec<Arc<Mutex<Option<Box<dyn std::any::Any>>>>>,
    empty: Arc<Condvar>,
    full: Arc<Condvar>,
    mutex: Arc<Mutex<()>>,
}

impl CircularQueue {
    pub fn new(size: usize) -> Self {
        CircularQueue {
            size: size + 1,
            read_pos: Arc::new(AtomicI32::new(0)),
            write_pos: Arc::new(AtomicI32::new(0)),
            data: (0..size + 1).map(|_| Arc::new(Mutex::new(None))).collect(),
            empty: Arc::new(Condvar::new()),
            full: Arc::new(Condvar::new()),
            mutex: Arc::new(Mutex::new(())),
        }
    }
}

impl Queue for CircularQueue {
    fn put(&self, data: Box<dyn std::any::Any>) -> Result<(), QueueError> {
        let _guard = self.mutex.lock().map_err(|_| QueueError::LockError)?;
        
        while (self.write_pos.load(Ordering::SeqCst) + 1) % self.size as i32 == self.read_pos.load(Ordering::SeqCst) {
            self.full.wait(_guard).map_err(|_| QueueError::LockError)?;
        }

        let write_pos = self.write_pos.load(Ordering::SeqCst) as usize;
        *self.data[write_pos].lock().map_err(|_| QueueError::LockError)? = Some(data);
        self.write_pos.store((write_pos + 1) as i32 % self.size as i32, Ordering::SeqCst);
        
        self.empty.notify_one();
        Ok(())
    }

    fn get(&self) -> Result<Box<dyn std::any::Any>, QueueError> {
        let _guard = self.mutex.lock().map_err(|_| QueueError::LockError)?;
        
        while self.read_pos.load(Ordering::SeqCst) == self.write_pos.load(Ordering::SeqCst) {
            self.empty.wait(_guard).map_err(|_| QueueError::LockError)?;
        }

        let read_pos = self.read_pos.load(Ordering::SeqCst) as usize;
        let data = self.data[read_pos].lock().map_err(|_| QueueError::LockError)?
            .take()
            .ok_or(QueueError::Empty)?;
            
        self.read_pos.store((read_pos + 1) as i32 % self.size as i32, Ordering::SeqCst);
        
        self.full.notify_one();
        Ok(data)
    }

    fn empty(&self) -> bool {
        self.read_pos.load(Ordering::SeqCst) == self.write_pos.load(Ordering::SeqCst)
    }

    fn flush(&self) {
        let _guard = self.mutex.lock().unwrap();
        self.read_pos.store(self.write_pos.load(Ordering::SeqCst), Ordering::SeqCst);
    }
}

pub struct Cache {
    max_buffers: usize,
    count: Arc<AtomicI32>,
    buffer_size: usize,
    noshrink_lookup: bool,
    first_freelist: bool,
    used: Arc<AtomicI32>,
    free_list: Arc<Mutex<Vec<FileBuffer>>>,
    hash_table: Arc<Mutex<HashMap<i64, FileBuffer>>>,
    wait_for_free: Arc<Condvar>,
    wait_for_unlock: Arc<Condvar>,
    mutex: Arc<Mutex<()>>,
}

impl Cache {
    pub fn new(buffer_size: usize, max_buffers: usize, noshrink_lookup: bool, first_freelist: bool) -> Self {
        Cache {
            max_buffers,
            count: Arc::new(AtomicI32::new(0)),
            buffer_size,
            noshrink_lookup,
            first_freelist,
            used: Arc::new(AtomicI32::new(0)),
            free_list: Arc::new(Mutex::new(Vec::new())),
            hash_table: Arc::new(Mutex::new(HashMap::new())),
            wait_for_free: Arc::new(Condvar::new()),
            wait_for_unlock: Arc::new(Condvar::new()),
            mutex: Arc::new(Mutex::new(())),
        }
    }

    pub fn lookup(&self, index: i64) -> Option<FileBuffer> {
        let _guard = self.mutex.lock().unwrap();
        let hash_table = self.hash_table.lock().unwrap();
        
        if let Some(mut buffer) = hash_table.get(&index).cloned() {
            if !buffer.used {
                let mut free_list = self.free_list.lock().unwrap();
                if let Some(pos) = free_list.iter().position(|x| x.index == buffer.index) {
                    free_list.remove(pos);
                }
                self.used.fetch_add(1, Ordering::SeqCst);
            }
            buffer.used = true;
            Some(buffer)
        } else {
            None
        }
    }

    pub fn get(&self, index: i64) -> FileBuffer {
        let _guard = self.mutex.lock().unwrap();
        
        loop {
            if self.noshrink_lookup {
                if self.first_freelist {
                    if let Some(buffer) = self.free_list.lock().unwrap().pop() {
                        self.used.fetch_add(1, Ordering::SeqCst);
                        return buffer;
                    }
                }
                if self.count.load(Ordering::SeqCst) < self.max_buffers as i32 {
                    let buffer = FileBuffer::new(self.buffer_size);
                    self.count.fetch_add(1, Ordering::SeqCst);
                    self.used.fetch_add(1, Ordering::SeqCst);
                    return buffer;
                }
            } else {
                if self.count.load(Ordering::SeqCst) < self.max_buffers as i32 {
                    let buffer = FileBuffer::new(self.buffer_size);
                    self.count.fetch_add(1, Ordering::SeqCst);
                    self.used.fetch_add(1, Ordering::SeqCst);
                    return buffer;
                }
            }
            
            self.wait_for_free.wait(_guard).unwrap();
        }
    }

    pub fn put(&self, mut buffer: FileBuffer) {
        let _guard = self.mutex.lock().unwrap();
        
        buffer.used = false;
        if buffer.used {
            if self.noshrink_lookup {
                self.free_list.lock().unwrap().push(buffer);
                self.used.fetch_sub(1, Ordering::SeqCst);
            } else {
                self.count.fetch_sub(1, Ordering::SeqCst);
            }
        }
        
        self.wait_for_free.notify_one();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circular_queue() {
        let queue = CircularQueue::new(2);
        
        // Test put and get
        queue.put(Box::new(42)).unwrap();
        let value = queue.get().unwrap();
        assert_eq!(value.downcast_ref::<i32>().unwrap(), &42);
        
        // Test empty
        assert!(queue.empty());
        
        // Test full
        queue.put(Box::new(1)).unwrap();
        queue.put(Box::new(2)).unwrap();
        assert!(queue.put(Box::new(3)).is_err());
    }

    #[test]
    fn test_cache() {
        let cache = Cache::new(1024, 2, true, true);
        
        // Test get and put
        let mut buffer = cache.get(1);
        buffer.index = 1;
        cache.put(buffer);
        
        // Test lookup
        let buffer = cache.lookup(1).unwrap();
        assert_eq!(buffer.index, 1);
    }
} 