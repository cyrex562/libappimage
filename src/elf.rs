use std::io::{self, Read};
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ElfError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Invalid ELF format: {0}")]
    InvalidFormat(String),
    #[error("Unsupported ELF class: {0}")]
    UnsupportedClass(u8),
    #[error("Unsupported ELF data: {0}")]
    UnsupportedData(u8),
}

#[derive(Debug)]
pub struct ElfFile {
    path: String,
    class: u8,
    data: u8,
    size: u64,
}

impl ElfFile {
    /// Create a new ElfFile instance for the given path
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, ElfError> {
        let path = path.as_ref().to_string_lossy().into_owned();
        let mut file = std::fs::File::open(&path)?;
        let mut buffer = [0u8; 16];
        file.read_exact(&mut buffer)?;

        // Check ELF signature
        if buffer[0] != 0x7F || buffer[1] != b'E' || buffer[2] != b'L' || buffer[3] != b'F' {
            return Err(ElfError::InvalidFormat("Not an ELF file".to_string()));
        }

        // Get ELF class (32-bit or 64-bit)
        let class = buffer[4];
        if class != 1 && class != 2 {
            return Err(ElfError::UnsupportedClass(class));
        }

        // Get ELF data encoding (little-endian or big-endian)
        let data = buffer[5];
        if data != 1 && data != 2 {
            return Err(ElfError::UnsupportedData(data));
        }

        // Get ELF header version
        let version = buffer[6];
        if version != 1 {
            return Err(ElfError::InvalidFormat(format!("Unsupported ELF version: {}", version)));
        }

        // Get ELF size based on class
        let size = if class == 1 {
            // 32-bit ELF
            let mut size = [0u8; 4];
            file.read_exact(&mut size)?;
            u32::from_le_bytes(size) as u64
        } else {
            // 64-bit ELF
            let mut size = [0u8; 8];
            file.read_exact(&mut size)?;
            u64::from_le_bytes(size)
        };

        Ok(Self {
            path,
            class,
            data,
            size,
        })
    }

    /// Get the size of the ELF file
    pub fn get_size(&self) -> u64 {
        self.size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_elf_file() {
        let path = "test.AppImage";
        let elf = ElfFile::new(path).unwrap();
        assert!(elf.get_size() > 0);
    }
} 