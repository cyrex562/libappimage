use std::path::Path;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use crate::error::{AppImageError, AppImageResult};
use crate::handlers::Handler;
use crate::squashfs_tools::reader::{Reader, ReaderManager, DirEnt, InodeInfo};
use crate::squashfs_tools::read::{read_block, read_super, read_filesystem};
use crate::squashfs_tools::fs::SquashFsFileType;

/// Type 1 AppImage handler implementation
pub struct Type1Handler {
    path: String,
    archive: Option<Reader>,
}

impl Type1Handler {
    /// Create a new Type 1 handler
    pub fn new(path: &Path) -> AppImageResult<Self> {
        Ok(Self {
            path: path.to_string_lossy().to_string(),
            archive: None,
        })
    }

    /// Open the AppImage archive
    pub fn open(&mut self) -> AppImageResult<()> {
        self.archive = Some(Reader::open(&self.path)?);
        Ok(())
    }

    /// Close the AppImage archive
    pub fn close(&mut self) -> AppImageResult<()> {
        self.archive = None;
        Ok(())
    }

    /// Get the file name from an archive entry
    fn get_file_name(path: &str) -> AppImageResult<String> {
        Ok(path.trim_start_matches("./").to_string())
    }

    /// Extract a file from the archive
    fn extract_file(&self, path: &str, target: impl AsRef<Path>) -> AppImageResult<()> {
        if let Some(parent) = target.as_ref().parent() {
            fs::create_dir_all(parent)?;
        }

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o644)
            .open(target)?;

        let archive = self.archive.as_ref().ok_or(AppImageError::NotOpen)?;
        let node = archive.get(path)?.ok_or_else(|| AppImageError::FileNotFound(path.to_string()))?;
        let mut reader = node.as_file()?;
        std::io::copy(&mut reader, &mut file)?;
        Ok(())
    }

    /// Read a file from the archive into a buffer
    fn read_file_into_buf(&self, path: &str) -> AppImageResult<Vec<u8>> {
        let archive = self.archive.as_ref().ok_or(AppImageError::NotOpen)?;
        let node = archive.get(path)?.ok_or_else(|| AppImageError::FileNotFound(path.to_string()))?;
        let mut reader = node.as_file()?;
        let mut buffer = Vec::new();
        reader.read_to_end(&mut buffer)?;
        Ok(buffer)
    }

    /// Get the link target from an archive entry
    fn get_file_link(&self, path: &str) -> AppImageResult<Option<String>> {
        let archive = self.archive.as_ref().ok_or(AppImageError::NotOpen)?;
        if let Ok(Some(node)) = archive.get(path) {
            if let Ok(SquashFsFileType::Symlink) = node.get_type() {
                Ok(Some(node.get_symlink()?.to_string_lossy().into_owned()))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    fn exists(&self, path: &str) -> bool {
        if let Some(archive) = &self.archive {
            archive.get(path).is_ok()
        } else {
            false
        }
    }
}

impl Handler for Type1Handler {
    fn get_type(&self) -> i32 {
        1
    }

    fn set_type(&mut self, _handler_type: i32) {
        // Type 1 handler is always type 1, so this is a no-op
    }

    fn get_file_name(&self, entry: &str) -> AppImageResult<String> {
        Self::get_file_name(entry)
    }

    fn extract_file(&mut self, path: &str, target: &Path) -> AppImageResult<()> {
        self.open()?;
        self.extract_file(path, target)
    }

    fn read_file_into_buf(&mut self, path: &str) -> AppImageResult<Vec<u8>> {
        self.open()?;
        self.read_file_into_buf(path)
    }

    fn get_file_link(&mut self, entry: &str) -> AppImageResult<Option<String>> {
        self.open()?;
        self.get_file_link(entry)
    }

    fn traverse(&mut self, mut callback: Box<dyn FnMut(&str, &[u8], &mut dyn std::any::Any) -> AppImageResult<()>>) -> AppImageResult<()> {
        self.open()?;
        let archive = self.archive.as_ref().ok_or(AppImageError::NotOpen)?;
        let root = archive.get("/")?.ok_or_else(|| AppImageError::FileNotFound("/".to_string()))?;
        let dir = root.as_dir()?;
        for entry in dir {
            let entry = entry?;
            if let Ok(SquashFsFileType::File) = entry.get_type() {
                let mut buf = Vec::new();
                entry.read_to_end(&mut buf)?;
                let path = entry.path().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
                let mut any = ();
                callback(&path, &buf, &mut any)?;
            }
        }
        Ok(())
    }
}

impl Drop for Type1Handler {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_type1_handler() -> AppImageResult<()> {
        let temp_dir = tempdir()?;
        let test_dir = temp_dir.path().join("test_dir");
        fs::create_dir(&test_dir)?;

        let test_file = test_dir.join("test.txt");
        let test_content = "Hello, World!";
        fs::write(&test_file, test_content)?;

        let squashfs_file = temp_dir.path().join("test.squashfs");
        let status = std::process::Command::new("mksquashfs")
            .arg(&test_dir)
            .arg(&squashfs_file)
            .status()?;
        assert!(status.success());

        let mut handler = Type1Handler::new(&squashfs_file)?;
        handler.open()?;

        assert_eq!(handler.get_type(), 1);
        assert_eq!(handler.get_file_name("test.txt")?, "test.txt");

        let buf = handler.read_file_into_buf("test.txt")?;
        assert_eq!(String::from_utf8(buf).unwrap(), test_content);

        let extracted_file = temp_dir.path().join("extracted.txt");
        handler.extract_file("test.txt", &extracted_file)?;
        assert_eq!(fs::read_to_string(extracted_file)?, test_content);

        let mut found = false;
        handler.traverse(Box::new(move |path, contents, _| {
            if path == "test.txt" {
                assert_eq!(String::from_utf8(contents.to_vec()).unwrap(), test_content);
                found = true;
            }
            Ok(())
        }))?;
        assert!(found);

        Ok(())
    }
} 