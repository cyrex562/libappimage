use std::sync::{Arc, Mutex, Condvar};
use std::sync::atomic::{AtomicI32, Ordering};
use super::FileBuffer;

pub struct ReadQueueThread {
    size: usize,
    buffer: Vec<Arc<Mutex<Option<FileBuffer>>>>,
    read_pos: Arc<AtomicI32>,
    write_pos: Arc<AtomicI32>,
    full: Arc<Condvar>,
}

impl ReadQueueThread {
    pub fn new(size: usize) -> Self {
        ReadQueueThread {
            size: size + 1,
            buffer: (0..size + 1).map(|_| Arc::new(Mutex::new(None))).collect(),
            read_pos: Arc::new(AtomicI32::new(0)),
            write_pos: Arc::new(AtomicI32::new(0)),
            full: Arc::new(Condvar::new()),
        }
    }
}

pub struct ReadQueue {
    threads: usize,
    count: Arc<AtomicI32>,
    mutex: Arc<Mutex<()>>,
    empty: Arc<Condvar>,
    thread: Vec<ReadQueueThread>,
}

impl ReadQueue {
    pub fn new() -> Self {
        ReadQueue {
            threads: 0,
            count: Arc::new(AtomicI32::new(0)),
            mutex: Arc::new(Mutex::new(())),
            empty: Arc::new(Condvar::new()),
            thread: Vec::new(),
        }
    }

    pub fn set(&mut self, threads: usize, size: usize) {
        let _guard = self.mutex.lock().unwrap();
        self.threads = threads;
        self.thread = (0..threads)
            .map(|_| ReadQueueThread::new(size))
            .collect();
    }

    pub fn put(&self, id: usize, buffer: FileBuffer) {
        let _guard = self.mutex.lock().unwrap();
        let thread = &self.thread[id];
        
        while (thread.write_pos.load(Ordering::SeqCst) + 1) % thread.size as i32 == thread.read_pos.load(Ordering::SeqCst) {
            thread.full.wait(_guard).unwrap();
        }

        let write_pos = thread.write_pos.load(Ordering::SeqCst) as usize;
        *thread.buffer[write_pos].lock().unwrap() = Some(buffer);
        thread.write_pos.store((write_pos + 1) as i32 % thread.size as i32, Ordering::SeqCst);
        
        self.count.fetch_add(1, Ordering::SeqCst);
        self.empty.notify_one();
    }

    pub fn get(&self) -> FileBuffer {
        let _guard = self.mutex.lock().unwrap();
        
        loop {
            let mut buffer = None;
            let mut id = 0;
            let mut empty = true;

            for (i, thread) in self.thread.iter().enumerate() {
                if thread.read_pos.load(Ordering::SeqCst) == thread.write_pos.load(Ordering::SeqCst) {
                    continue;
                }

                let read_pos = thread.read_pos.load(Ordering::SeqCst) as usize;
                if let Some(Some(current)) = thread.buffer[read_pos].lock().unwrap().as_ref() {
                    if buffer.is_none() || earlier_buffer(current, &buffer.unwrap()) {
                        buffer = Some(current.clone());
                        id = i;
                        empty = false;
                    }
                }
            }

            if empty {
                self.empty.wait(_guard).unwrap();
            } else {
                let thread = &self.thread[id];
                let read_pos = thread.read_pos.load(Ordering::SeqCst) as usize;
                *thread.buffer[read_pos].lock().unwrap() = None;
                thread.read_pos.store((read_pos + 1) as i32 % thread.size as i32, Ordering::SeqCst);
                
                self.count.fetch_sub(1, Ordering::SeqCst);
                thread.full.notify_one();
                
                return buffer.unwrap();
            }
        }
    }

    pub fn flush(&self) {
        let _guard = self.mutex.lock().unwrap();
        for thread in &self.thread {
            thread.read_pos.store(thread.write_pos.load(Ordering::SeqCst), Ordering::SeqCst);
        }
    }
}

fn earlier_buffer(new: &FileBuffer, old: &FileBuffer) -> bool {
    if old.file_count == new.file_count {
        if old.version == new.version {
            new.block < old.block
        } else {
            new.version < old.version
        }
    } else {
        new.file_count < old.file_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_queue() {
        let mut queue = ReadQueue::new();
        queue.set(2, 2);
        
        let mut buffer = FileBuffer::new(1024);
        buffer.file_count = 1;
        buffer.block = 0;
        buffer.version = 0;
        
        queue.put(0, buffer.clone());
        let result = queue.get();
        
        assert_eq!(result.file_count, 1);
        assert_eq!(result.block, 0);
        assert_eq!(result.version, 0);
    }

    #[test]
    fn test_read_queue_multiple_threads() {
        let mut queue = ReadQueue::new();
        queue.set(2, 2);
        
        let mut buffer1 = FileBuffer::new(1024);
        buffer1.file_count = 1;
        buffer1.block = 0;
        buffer1.version = 0;
        
        let mut buffer2 = FileBuffer::new(1024);
        buffer2.file_count = 1;
        buffer2.block = 1;
        buffer2.version = 0;
        
        queue.put(0, buffer1);
        queue.put(1, buffer2);
        
        let result1 = queue.get();
        let result2 = queue.get();
        
        assert_eq!(result1.block, 0);
        assert_eq!(result2.block, 1);
    }
} 