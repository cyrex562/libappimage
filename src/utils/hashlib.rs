use std::io::{self, Read};
use std::fmt;
use md5::{Md5, Digest};

/// C++ wrapper around the bare C hashing algorithms implementations
pub mod hashlib {
    use super::*;

    /// Convenience function to compute md5 sums from a Read implementation
    /// 
    /// # Arguments
    /// * `data` - Any type implementing Read trait
    /// 
    /// # Returns
    /// * `Result<Vec<u8>, io::Error>` - MD5 digest as bytes on success, IO error otherwise
    pub fn md5<R: Read>(mut data: R) -> Result<Vec<u8>, io::Error> {
        let mut hasher = Md5::new();
        let mut buffer = [0; 4096];
        
        loop {
            let bytes_read = data.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        Ok(hasher.finalize().to_vec())
    }

    /// Convenience function to compute md5 sums from a string
    /// 
    /// # Arguments
    /// * `data` - String to compute MD5 hash for
    /// 
    /// # Returns
    /// * `Vec<u8>` - MD5 digest as bytes
    pub fn md5_string(data: &str) -> Vec<u8> {
        let mut hasher = Md5::new();
        hasher.update(data.as_bytes());
        hasher.finalize().to_vec()
    }

    /// Generates an hexadecimal representation of the values in the digest
    /// 
    /// # Arguments
    /// * `digest` - MD5 digest as bytes
    /// 
    /// # Returns
    /// * `String` - Hexadecimal representation of the digest
    pub fn to_hex(digest: &[u8]) -> String {
        digest.iter()
            .map(|b| format!("{:02x}", b))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_md5_string() {
        let input = "Hello, World!";
        let hash = hashlib::md5_string(input);
        let hex = hashlib::to_hex(&hash);
        assert_eq!(hex, "65a8e27d8879283831b6bd4031b7ba1a");
    }

    #[test]
    fn test_md5_read() {
        let input = "Hello, World!";
        let cursor = Cursor::new(input);
        let hash = hashlib::md5(cursor).unwrap();
        let hex = hashlib::to_hex(&hash);
        assert_eq!(hex, "65a8e27d8879283831b6bd4031b7ba1a");
    }

    #[test]
    fn test_to_hex() {
        let input = vec![0x12, 0x34, 0x56, 0x78];
        let hex = hashlib::to_hex(&input);
        assert_eq!(hex, "12345678");
    }
} 