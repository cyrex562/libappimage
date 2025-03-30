use std::io::{self, Read, BufRead};
use std::vec::Vec;
use thiserror::Error;
use libarchive::archive::{Archive, ArchiveRead};
use squashfuse::{SquashFs, Inode};

#[derive(Error, Debug)]
pub enum StreambufError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Archive error: {0}")]
    Archive(String),
    #[error("SquashFS error: {0}")]
    SquashFs(String),
}

/// Provides a buffered reader implementation for reading type 1 AppImages
/// by means of libarchive.
pub struct StreambufType1 {
    archive: ArchiveRead,
    buffer: Vec<u8>,
    buffer_pos: usize,
    buffer_len: usize,
}

impl StreambufType1 {
    /// Create a new StreambufType1 from an archive with the specified buffer size
    pub fn new(archive: ArchiveRead, buffer_size: usize) -> Self {
        Self {
            archive,
            buffer: vec![0; buffer_size],
            buffer_pos: 0,
            buffer_len: 0,
        }
    }

    /// Read more data from the archive into the buffer
    fn fill_buffer(&mut self) -> io::Result<()> {
        let bytes_read = self.archive.read_data(&mut self.buffer)?;
        self.buffer_pos = 0;
        self.buffer_len = bytes_read;
        Ok(())
    }
}

impl Read for StreambufType1 {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut total_read = 0;
        
        while total_read < buf.len() {
            // If we've consumed all data in the buffer, try to read more
            if self.buffer_pos >= self.buffer_len {
                self.fill_buffer()?;
                if self.buffer_len == 0 {
                    break; // EOF
                }
            }

            // Calculate how many bytes we can copy
            let available = self.buffer_len - self.buffer_pos;
            let to_copy = std::cmp::min(available, buf.len() - total_read);
            
            // Copy data from buffer to output
            buf[total_read..total_read + to_copy]
                .copy_from_slice(&self.buffer[self.buffer_pos..self.buffer_pos + to_copy]);
            
            self.buffer_pos += to_copy;
            total_read += to_copy;
        }

        Ok(total_read)
    }
}

impl BufRead for StreambufType1 {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        if self.buffer_pos >= self.buffer_len {
            self.fill_buffer()?;
        }
        Ok(&self.buffer[self.buffer_pos..self.buffer_len])
    }

    fn consume(&mut self, amt: usize) {
        self.buffer_pos = std::cmp::min(self.buffer_pos + amt, self.buffer_len);
    }
}

// Prevent cloning and copying
impl Clone for StreambufType1 {
    fn clone(&self) -> Self {
        panic!("StreambufType1 cannot be cloned")
    }
}

/// Provides a buffered reader implementation for reading type 2 AppImages
/// by means of squashfuse.
pub struct StreambufType2 {
    fs: SquashFs,
    inode: Inode,
    buffer: Vec<u8>,
    bytes_already_read: u64,
}

impl StreambufType2 {
    /// Create a new StreambufType2 for reading the file pointed by inode at fs
    /// with the specified buffer size
    pub fn new(fs: SquashFs, inode: Inode, buffer_size: usize) -> Self {
        Self {
            fs,
            inode,
            buffer: vec![0; buffer_size],
            bytes_already_read: 0,
        }
    }

    /// Read more data from the SquashFS file into the buffer
    fn fill_buffer(&mut self) -> io::Result<()> {
        // Check if we've reached the end of the file
        if self.bytes_already_read >= self.inode.file_size() {
            return Ok(());
        }

        // Calculate how many bytes to read
        let remaining = self.inode.file_size() - self.bytes_already_read;
        let to_read = std::cmp::min(remaining, self.buffer.len() as u64) as usize;

        // Read the data
        let bytes_read = self.fs.read_range(
            &self.inode,
            self.bytes_already_read,
            &mut self.buffer[..to_read],
        ).map_err(|e| StreambufError::SquashFs(e.to_string()))?;

        self.bytes_already_read += bytes_read as u64;
        Ok(())
    }
}

impl Read for StreambufType2 {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut total_read = 0;
        
        while total_read < buf.len() {
            // If we've consumed all data in the buffer, try to read more
            if self.bytes_already_read >= self.inode.file_size() {
                break; // EOF
            }

            // Read more data if needed
            self.fill_buffer()?;

            // Calculate how many bytes we can copy
            let available = (self.bytes_already_read - (self.bytes_already_read - self.buffer.len() as u64)) as usize;
            let to_copy = std::cmp::min(available, buf.len() - total_read);
            
            // Copy data from buffer to output
            buf[total_read..total_read + to_copy]
                .copy_from_slice(&self.buffer[..to_copy]);
            
            total_read += to_copy;
        }

        Ok(total_read)
    }
}

impl BufRead for StreambufType2 {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        if self.bytes_already_read < self.inode.file_size() {
            self.fill_buffer()?;
        }
        Ok(&self.buffer[..(self.bytes_already_read - (self.bytes_already_read - self.buffer.len() as u64)) as usize])
    }

    fn consume(&mut self, amt: usize) {
        self.bytes_already_read = std::cmp::min(
            self.bytes_already_read + amt as u64,
            self.inode.file_size()
        );
    }
}

// Prevent cloning and copying
impl Clone for StreambufType2 {
    fn clone(&self) -> Self {
        panic!("StreambufType2 cannot be cloned")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_read() {
        let data = b"Hello, World!";
        let archive = ArchiveRead::new(Cursor::new(data));
        let mut stream = StreambufType1::new(archive, 4);
        
        let mut buffer = [0u8; 13];
        assert_eq!(stream.read(&mut buffer).unwrap(), 13);
        assert_eq!(&buffer, data);
    }
} 