use std::io::{self, Read, BufRead};

/// A wrapper around std::io::Read to allow reading files contained inside an AppImage.
/// 
/// This struct provides functionality similar to the C++ PayloadIStream class,
/// but using Rust's more idiomatic I/O traits.
pub struct PayloadIStream {
    inner: Box<dyn Read>,
}

impl PayloadIStream {
    /// Creates a new PayloadIStream with the given reader
    pub(crate) fn new(reader: Box<dyn Read>) -> Self {
        Self { inner: reader }
    }
}

impl Read for PayloadIStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}

impl BufRead for PayloadIStream {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        // Since we're using Box<dyn Read>, we can't implement BufRead directly
        // This is a limitation of the current design
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "PayloadIStream does not support BufRead operations",
        ))
    }

    fn consume(&mut self, amt: usize) {
        // No-op since we can't implement BufRead
    }
}

// Prevent cloning and copying
impl Clone for PayloadIStream {
    fn clone(&self) -> Self {
        panic!("PayloadIStream cannot be cloned")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_read() {
        let data = b"Hello, World!";
        let reader = Box::new(Cursor::new(data));
        let mut stream = PayloadIStream::new(reader);
        
        let mut buffer = [0u8; 13];
        assert_eq!(stream.read(&mut buffer).unwrap(), 13);
        assert_eq!(&buffer, data);
    }
} 