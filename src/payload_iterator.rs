use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;
use crate::error::{AppImageError, AppImageResult};
use crate::payload_types::PayloadEntryType;

/// Iterator for traversing files in an AppImage payload
pub struct PayloadIterator {
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

impl PayloadIterator {
    /// Create a new PayloadIterator for the given AppImage file
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

    /// Read the next entry from the payload
    pub fn next(&mut self) -> AppImageResult<bool> {
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

    /// Get the current entry type
    pub fn get_type(&self) -> PayloadEntryType {
        self.entry_type
    }

    /// Get the current entry name
    pub fn get_name(&self) -> &str {
        &self.entry_name
    }

    /// Get the current entry size
    pub fn get_size(&self) -> u64 {
        self.entry_size
    }

    /// Get the current entry mode
    pub fn get_mode(&self) -> u32 {
        self.entry_mode
    }

    /// Get the current entry mtime
    pub fn get_mtime(&self) -> u64 {
        self.entry_mtime
    }

    /// Get the current entry target (for symlinks)
    pub fn get_target(&self) -> &str {
        &self.entry_target
    }

    /// Reset the iterator to the beginning of the payload
    pub fn reset(&mut self) {
        self.position = 0;
        self.entry_type = PayloadEntryType::Unknown;
        self.entry_name.clear();
        self.entry_size = 0;
        self.entry_mode = 0;
        self.entry_mtime = 0;
        self.entry_target.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_payload_iterator() {
        let path = "test.AppImage";
        let mut iterator = PayloadIterator::new(path).unwrap();
        
        // Test first entry
        assert!(iterator.next().unwrap());
        assert_eq!(iterator.get_type(), PayloadEntryType::File);
        assert_eq!(iterator.get_name(), "test.txt");
        assert_eq!(iterator.get_size(), 1024);
        assert_eq!(iterator.get_mode(), 0o644);
        assert_eq!(iterator.get_mtime(), 1234567890);
        
        // Test symlink entry
        assert!(iterator.next().unwrap());
        assert_eq!(iterator.get_type(), PayloadEntryType::Symlink);
        assert_eq!(iterator.get_name(), "link.txt");
        assert_eq!(iterator.get_target(), "test.txt");
        
        // Test end of payload
        assert!(!iterator.next().unwrap());
        
        // Test reset
        iterator.reset();
        assert!(iterator.next().unwrap());
        assert_eq!(iterator.get_name(), "test.txt");
    }
} 