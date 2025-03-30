use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

/// Error type for MagicBytesChecker operations
#[derive(Debug, thiserror::Error)]
pub enum MagicBytesError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
}

/// Result type for MagicBytesChecker operations
pub type Result<T> = std::result::Result<T, MagicBytesError>;

/// Allows the verification of magic bytes in a given file.
pub struct MagicBytesChecker {
    file: File,
}

impl MagicBytesChecker {
    /// Create a new MagicBytesChecker for the given file path
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path)?;
        Ok(Self { file })
    }

    /// Check if the file has an ISO 9660 signature
    /// 
    /// Signature: 43 44 30 30 31 = "CD001"
    /// This signature usually occurs at byte offset 32769 (0x8001),
    /// 34817 (0x8801), or 36865 (0x9001).
    pub fn has_iso9660_signature(&mut self) -> Result<bool> {
        let positions = [32769, 34817, 36865];
        let signature = b"CD001";

        for &pos in &positions {
            if self.has_signature_at(signature, pos)? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Check if the file has an ELF signature
    /// 
    /// Checks for magic hex 0x7f 0x45 0x4c 0x46 at offset 0
    pub fn has_elf_signature(&mut self) -> Result<bool> {
        let signature = [0x7f, 0x45, 0x4c, 0x46];
        self.has_signature_at(&signature, 0)
    }

    /// Check if the file has an AppImage Type 1 signature
    /// 
    /// Checks for magic hex 0x414901 at offset 8
    pub fn has_appimage_type1_signature(&mut self) -> Result<bool> {
        let signature = [0x41, 0x49, 0x01];
        self.has_signature_at(&signature, 8)
    }

    /// Check if the file has an AppImage Type 2 signature
    /// 
    /// Checks for magic hex 0x414902 at offset 8
    pub fn has_appimage_type2_signature(&mut self) -> Result<bool> {
        let signature = [0x41, 0x49, 0x02];
        self.has_signature_at(&signature, 8)
    }

    /// Verify if the input matches at the given offset with the signature
    fn has_signature_at(&mut self, signature: &[u8], offset: u64) -> Result<bool> {
        // Move to the right offset in the file
        self.file.seek(SeekFrom::Start(offset))?;

        // Read the same number of bytes as the signature
        let mut buffer = vec![0u8; signature.len()];
        self.file.read_exact(&mut buffer)?;

        // Compare the bytes
        Ok(buffer == signature)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_file(content: &[u8]) -> io::Result<NamedTempFile> {
        let mut file = NamedTempFile::new()?;
        file.write_all(content)?;
        Ok(file)
    }

    #[test]
    fn test_iso9660_signature() -> Result<()> {
        let mut content = vec![0u8; 36866];
        content[32769..32774].copy_from_slice(b"CD001");
        
        let file = create_test_file(&content)?;
        let mut checker = MagicBytesChecker::new(file.path())?;
        
        assert!(checker.has_iso9660_signature()?);
        Ok(())
    }

    #[test]
    fn test_elf_signature() -> Result<()> {
        let mut content = vec![0u8; 4];
        content[0..4].copy_from_slice(&[0x7f, 0x45, 0x4c, 0x46]);
        
        let file = create_test_file(&content)?;
        let mut checker = MagicBytesChecker::new(file.path())?;
        
        assert!(checker.has_elf_signature()?);
        Ok(())
    }

    #[test]
    fn test_appimage_type1_signature() -> Result<()> {
        let mut content = vec![0u8; 11];
        content[8..11].copy_from_slice(&[0x41, 0x49, 0x01]);
        
        let file = create_test_file(&content)?;
        let mut checker = MagicBytesChecker::new(file.path())?;
        
        assert!(checker.has_appimage_type1_signature()?);
        Ok(())
    }

    #[test]
    fn test_appimage_type2_signature() -> Result<()> {
        let mut content = vec![0u8; 11];
        content[8..11].copy_from_slice(&[0x41, 0x49, 0x02]);
        
        let file = create_test_file(&content)?;
        let mut checker = MagicBytesChecker::new(file.path())?;
        
        assert!(checker.has_appimage_type2_signature()?);
        Ok(())
    }

    #[test]
    fn test_invalid_signatures() -> Result<()> {
        let content = vec![0u8; 36866];
        let file = create_test_file(&content)?;
        let mut checker = MagicBytesChecker::new(file.path())?;
        
        assert!(!checker.has_iso9660_signature()?);
        assert!(!checker.has_elf_signature()?);
        assert!(!checker.has_appimage_type1_signature()?);
        assert!(!checker.has_appimage_type2_signature()?);
        Ok(())
    }
} 