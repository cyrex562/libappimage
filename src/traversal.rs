use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use thiserror::Error;
use backhand::{FilesystemReader, NodeHeader};
use crate::streambuf::{StreambufType1, StreambufType2};
use crate::payload::PayloadIStream;
use crate::error::{AppImageError, AppImageResult};
use crate::payload_types::PayloadEntryType;

#[derive(Error, Debug)]
pub enum TraversalError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Archive error: {0}")]
    Archive(String),
    #[error("File system error: {0}")]
    FileSystem(String),
    #[error("SquashFS error: {0}")]
    SquashFs(String),
    #[error("AppImage error: {0}")]
    AppImage(String),
}

/// Trait for traversing files in an AppImage payload
pub trait Traversal {
    /// Get the next entry in the traversal
    fn next(&mut self) -> AppImageResult<bool>;
    
    /// Get the type of the current entry
    fn get_type(&self) -> PayloadEntryType;
    
    /// Get the name of the current entry
    fn get_name(&self) -> &str;
    
    /// Get the size of the current entry
    fn get_size(&self) -> u64;
    
    /// Get the mode of the current entry
    fn get_mode(&self) -> u32;
    
    /// Get the mtime of the current entry
    fn get_mtime(&self) -> u64;
    
    /// Get the target of the current entry (for symlinks)
    fn get_target(&self) -> &str;
    
    /// Reset the traversal to the beginning
    fn reset(&mut self);
}

impl Iterator for Box<dyn Traversal> {
    type Item = AppImageResult<String>;

    fn next(&mut self) -> Option<Self::Item> {
        match (**self).next() {
            Ok(true) => Some(Ok(self.get_name().to_string())),
            Ok(false) => None,
            Err(e) => Some(Err(e)),
        }
    }
}

/// Implementation of Traversal for Type 1 AppImages
pub struct TraversalType1 {
    /// The file path of the AppImage
    path: String,
    /// The current position in the file
    position: u64,
    /// The size of the payload
    payload_size: u64,
    /// The current entry type
    entry_type: PayloadEntryType,
    /// The current entry name
    entry_name: String,
    /// The current entry size
    entry_size: u64,
    /// The current entry mode
    entry_mode: u32,
    /// The current entry mtime
    entry_mtime: u64,
    /// The current entry target (for symlinks)
    entry_target: String,
}

impl TraversalType1 {
    /// Create a new TraversalType1 for the given AppImage file
    pub fn new<P: AsRef<Path>>(path: P) -> AppImageResult<Self> {
        let path = path.as_ref().to_string_lossy().into_owned();
        let mut file = std::fs::File::open(&path)?;
        
        // Get file size
        let file_size = file.metadata()?.len();
        
        // Calculate payload size (file size minus ELF header)
        let payload_size = file_size - Self::get_elf_size(&mut file)?;
        
        Ok(Self {
            path,
            position: 0,
            payload_size,
            entry_type: PayloadEntryType::Unknown,
            entry_name: String::new(),
            entry_size: 0,
            entry_mode: 0,
            entry_mtime: 0,
            entry_target: String::new(),
        })
    }

    /// Get the size of the ELF header
    fn get_elf_size(file: &mut std::fs::File) -> AppImageResult<u64> {
        let mut buffer = [0u8; 4];
        file.read_exact(&mut buffer)?;
        
        // Check for ELF signature
        if buffer != [0x7F, b'E', b'L', b'F'] {
            return Err(AppImageError::InvalidFormat("Not an ELF file".to_string()));
        }
        
        // Read ELF header size
        file.seek(SeekFrom::Start(0x28))?;
        let mut size = [0u8; 8];
        file.read_exact(&mut size)?;
        
        Ok(u64::from_le_bytes(size))
    }
}

impl Traversal for TraversalType1 {
    fn next(&mut self) -> AppImageResult<bool> {
        let mut file = std::fs::File::open(&self.path)?;
        
        // Seek to payload start + current position
        file.seek(SeekFrom::Start(self.position))?;
        
        // Read entry header
        let mut header = [0u8; 8];
        file.read_exact(&mut header)?;
        
        // Check for end of payload
        if header == [0; 8] {
            return Ok(false);
        }
        
        // Parse entry header
        self.entry_type = PayloadEntryType::from_mode(u32::from_le_bytes(header[0..4].try_into().unwrap()));
        self.entry_size = u64::from_le_bytes(header[4..8].try_into().unwrap());
        
        // Read entry name
        let mut name_len = [0u8; 2];
        file.read_exact(&mut name_len)?;
        let name_len = u16::from_le_bytes(name_len) as usize;
        
        let mut name = vec![0u8; name_len];
        file.read_exact(&mut name)?;
        self.entry_name = String::from_utf8_lossy(&name).to_string();
        
        // Read entry mode
        let mut mode = [0u8; 4];
        file.read_exact(&mut mode)?;
        self.entry_mode = u32::from_le_bytes(mode);
        
        // Read entry mtime
        let mut mtime = [0u8; 8];
        file.read_exact(&mut mtime)?;
        self.entry_mtime = u64::from_le_bytes(mtime);
        
        // Read symlink target if applicable
        if self.entry_type.is_symlink() {
            let mut target_len = [0u8; 2];
            file.read_exact(&mut target_len)?;
            let target_len = u16::from_le_bytes(target_len) as usize;
            
            let mut target = vec![0u8; target_len];
            file.read_exact(&mut target)?;
            self.entry_target = String::from_utf8_lossy(&target).to_string();
        }
        
        // Update position for next entry
        self.position += 8 + 2 + name_len as u64 + 4 + 8;
        if self.entry_type.is_symlink() {
            self.position += 2 + self.entry_target.len() as u64;
        }
        
        Ok(true)
    }

    fn get_type(&self) -> PayloadEntryType {
        self.entry_type
    }

    fn get_name(&self) -> &str {
        &self.entry_name
    }

    fn get_size(&self) -> u64 {
        self.entry_size
    }

    fn get_mode(&self) -> u32 {
        self.entry_mode
    }

    fn get_mtime(&self) -> u64 {
        self.entry_mtime
    }

    fn get_target(&self) -> &str {
        &self.entry_target
    }

    fn reset(&mut self) {
        self.position = 0;
        self.entry_type = PayloadEntryType::Unknown;
        self.entry_name.clear();
        self.entry_size = 0;
        self.entry_mode = 0;
        self.entry_mtime = 0;
        self.entry_target.clear();
    }
}

/// Provides an implementation of the traversal trait for type 2 AppImages.
/// It's based on squashfuse and is READONLY, ONE WAY, SINGLE PASS.
pub struct TraversalType2 {
    /// The file path of the AppImage
    path: String,
    /// The current position in the file
    position: u64,
    /// The size of the payload
    payload_size: u64,
    /// The current entry type
    entry_type: PayloadEntryType,
    /// The current entry name
    entry_name: String,
    /// The current entry size
    entry_size: u64,
    /// The current entry mode
    entry_mode: u32,
    /// The current entry mtime
    entry_mtime: u64,
    /// The current entry target (for symlinks)
    entry_target: String,
}

impl TraversalType2 {
    /// Create a new TraversalType2 for the given AppImage file
    pub fn new<P: AsRef<Path>>(path: P) -> AppImageResult<Self> {
        let path = path.as_ref().to_string_lossy().into_owned();
        let mut file = std::fs::File::open(&path)?;
        
        // Get file size
        let file_size = file.metadata()?.len();
        
        // Calculate payload size (file size minus ELF header)
        let payload_size = file_size - Self::get_elf_size(&mut file)?;
        
        Ok(Self {
            path,
            position: 0,
            payload_size,
            entry_type: PayloadEntryType::Unknown,
            entry_name: String::new(),
            entry_size: 0,
            entry_mode: 0,
            entry_mtime: 0,
            entry_target: String::new(),
        })
    }

    /// Get the size of the ELF header
    fn get_elf_size(file: &mut std::fs::File) -> AppImageResult<u64> {
        let mut buffer = [0u8; 4];
        file.read_exact(&mut buffer)?;
        
        // Check for ELF signature
        if buffer != [0x7F, b'E', b'L', b'F'] {
            return Err(AppImageError::InvalidFormat("Not an ELF file".to_string()));
        }
        
        // Read ELF header size
        file.seek(SeekFrom::Start(0x28))?;
        let mut size = [0u8; 8];
        file.read_exact(&mut size)?;
        
        Ok(u64::from_le_bytes(size))
    }
}

impl Traversal for TraversalType2 {
    fn next(&mut self) -> AppImageResult<bool> {
        let mut file = std::fs::File::open(&self.path)?;
        
        // Seek to payload start + current position
        file.seek(SeekFrom::Start(self.position))?;
        
        // Read entry header
        let mut header = [0u8; 8];
        file.read_exact(&mut header)?;
        
        // Check for end of payload
        if header == [0; 8] {
            return Ok(false);
        }
        
        // Parse entry header
        self.entry_type = PayloadEntryType::from_mode(u32::from_le_bytes(header[0..4].try_into().unwrap()));
        self.entry_size = u64::from_le_bytes(header[4..8].try_into().unwrap());
        
        // Read entry name
        let mut name_len = [0u8; 2];
        file.read_exact(&mut name_len)?;
        let name_len = u16::from_le_bytes(name_len) as usize;
        
        let mut name = vec![0u8; name_len];
        file.read_exact(&mut name)?;
        self.entry_name = String::from_utf8_lossy(&name).to_string();
        
        // Read entry mode
        let mut mode = [0u8; 4];
        file.read_exact(&mut mode)?;
        self.entry_mode = u32::from_le_bytes(mode);
        
        // Read entry mtime
        let mut mtime = [0u8; 8];
        file.read_exact(&mut mtime)?;
        self.entry_mtime = u64::from_le_bytes(mtime);
        
        // Read symlink target if applicable
        if self.entry_type.is_symlink() {
            let mut target_len = [0u8; 2];
            file.read_exact(&mut target_len)?;
            let target_len = u16::from_le_bytes(target_len) as usize;
            
            let mut target = vec![0u8; target_len];
            file.read_exact(&mut target)?;
            self.entry_target = String::from_utf8_lossy(&target).to_string();
        }
        
        // Update position for next entry
        self.position += 8 + 2 + name_len as u64 + 4 + 8;
        if self.entry_type.is_symlink() {
            self.position += 2 + self.entry_target.len() as u64;
        }
        
        Ok(true)
    }

    fn get_type(&self) -> PayloadEntryType {
        self.entry_type
    }

    fn get_name(&self) -> &str {
        &self.entry_name
    }

    fn get_size(&self) -> u64 {
        self.entry_size
    }

    fn get_mode(&self) -> u32 {
        self.entry_mode
    }

    fn get_mtime(&self) -> u64 {
        self.entry_mtime
    }

    fn get_target(&self) -> &str {
        &self.entry_target
    }

    fn reset(&mut self) {
        self.position = 0;
        self.entry_type = PayloadEntryType::Unknown;
        self.entry_name.clear();
        self.entry_size = 0;
        self.entry_mode = 0;
        self.entry_mtime = 0;
        self.entry_target.clear();
    }
}

impl Drop for TraversalType2 {
    fn drop(&mut self) {
        // Resources will be cleaned up automatically when dropped
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_traversal_type1() {
        let path = "test.AppImage";
        let mut traversal = TraversalType1::new(path).unwrap();
        
        // Test first entry
        assert!(traversal.next().unwrap());
        assert_eq!(traversal.get_type(), PayloadEntryType::File);
        assert_eq!(traversal.get_name(), "test.txt");
        assert_eq!(traversal.get_size(), 1024);
        assert_eq!(traversal.get_mode(), 0o644);
        assert_eq!(traversal.get_mtime(), 1234567890);
        
        // Test symlink entry
        assert!(traversal.next().unwrap());
        assert_eq!(traversal.get_type(), PayloadEntryType::Symlink);
        assert_eq!(traversal.get_name(), "link.txt");
        assert_eq!(traversal.get_target(), "test.txt");
        
        // Test end of payload
        assert!(!traversal.next().unwrap());
        
        // Test reset
        traversal.reset();
        assert!(traversal.next().unwrap());
        assert_eq!(traversal.get_name(), "test.txt");
    }

    #[test]
    fn test_traversal_type2() {
        let path = "test.AppImage";
        let mut traversal = TraversalType2::new(path).unwrap();
        
        // Test first entry
        assert!(traversal.next().unwrap());
        assert_eq!(traversal.get_type(), PayloadEntryType::File);
        assert_eq!(traversal.get_name(), "test.txt");
        assert_eq!(traversal.get_size(), 1024);
        assert_eq!(traversal.get_mode(), 0o644);
        assert_eq!(traversal.get_mtime(), 1234567890);
        
        // Test symlink entry
        assert!(traversal.next().unwrap());
        assert_eq!(traversal.get_type(), PayloadEntryType::Symlink);
        assert_eq!(traversal.get_name(), "link.txt");
        assert_eq!(traversal.get_target(), "test.txt");
        
        // Test end of payload
        assert!(!traversal.next().unwrap());
        
        // Test reset
        traversal.reset();
        assert!(traversal.next().unwrap());
        assert_eq!(traversal.get_name(), "test.txt");
    }
} 