use std::path::Path;
use crate::error::{AppImageError, AppImageResult};
use crate::payload_types::PayloadEntryType;

/// Cache for AppImage payload entries
pub struct PayloadEntriesCache {
    entries: std::collections::HashMap<String, (PayloadEntryType, String)>,
}

impl PayloadEntriesCache {
    /// Create a new PayloadEntriesCache for the given AppImage
    pub fn new<P: AsRef<Path>>(_path: P) -> AppImageResult<Self> {
        Ok(Self {
            entries: std::collections::HashMap::new(),
        })
    }

    /// Get the type of an entry
    pub fn get_entry_type(&self, entry: &str) -> Option<PayloadEntryType> {
        self.entries.get(entry).map(|(entry_type, _)| *entry_type)
    }

    /// Get the link target of an entry
    pub fn get_entry_link_target(&self, entry: &str) -> Option<&str> {
        self.entries.get(entry).map(|(_, target)| target.as_str())
    }

    /// Get all entry paths
    pub fn get_all_entry_paths(&self) -> Vec<String> {
        self.entries.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_payload_entries_cache_creation() {
        let temp_dir = tempdir().unwrap();
        let app_image_path = temp_dir.path().join("test.AppImage");
        
        let mut file = File::create(&app_image_path).unwrap();
        file.write_all(b"dummy content").unwrap();

        let cache = PayloadEntriesCache::new(&app_image_path).unwrap();
        assert!(!cache.get_all_entry_paths().is_empty());
    }

    #[test]
    fn test_entry_type_detection() {
        let temp_dir = tempdir().unwrap();
        let app_image_path = temp_dir.path().join("test.AppImage");
        
        let mut file = File::create(&app_image_path).unwrap();
        file.write_all(b"dummy content").unwrap();

        let cache = PayloadEntriesCache::new(&app_image_path).unwrap();
        
        for path in cache.get_all_entry_paths() {
            assert!(matches!(cache.get_entry_type(&path), Some(PayloadEntryType::File)));
        }
    }
} 