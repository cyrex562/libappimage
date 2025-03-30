use std::time::{SystemTime, UNIX_EPOCH};
use std::path::Path;
use crate::squashfs_tools::error::{Error, Result};

/// Represents a timestamp with nanosecond precision
#[derive(Debug, Clone, Copy)]
pub struct Timestamp {
    seconds: i64,
    nanoseconds: u32,
}

impl Timestamp {
    /// Create a new timestamp from seconds and nanoseconds
    pub fn new(seconds: i64, nanoseconds: u32) -> Self {
        Self {
            seconds,
            nanoseconds,
        }
    }

    /// Create a timestamp from a SystemTime
    pub fn from_system_time(time: SystemTime) -> Self {
        let duration = time.duration_since(UNIX_EPOCH).unwrap();
        Self {
            seconds: duration.as_secs() as i64,
            nanoseconds: duration.subsec_nanos(),
        }
    }

    /// Get the seconds component
    pub fn seconds(&self) -> i64 {
        self.seconds
    }

    /// Get the nanoseconds component
    pub fn nanoseconds(&self) -> u32 {
        self.nanoseconds
    }
}

/// Set the timestamp for a file or symlink
/// 
/// This function handles the platform-specific differences in setting timestamps.
/// On OpenBSD, it uses utimensat with AT_SYMLINK_NOFOLLOW.
/// On other systems, it uses lutimes.
pub fn set_timestamp<P: AsRef<Path>>(pathname: P, timestamp: Timestamp) -> Result<()> {
    #[cfg(target_os = "openbsd")]
    {
        use std::os::unix::fs::Utimensat;
        use std::os::unix::fs::AtFlags;
        
        let times = [
            std::os::unix::fs::Timespec {
                tv_sec: timestamp.seconds(),
                tv_nsec: timestamp.nanoseconds() as i64,
            },
            std::os::unix::fs::Timespec {
                tv_sec: timestamp.seconds(),
                tv_nsec: timestamp.nanoseconds() as i64,
            },
        ];

        std::fs::OpenOptions::new()
            .read(true)
            .open(pathname.as_ref())?
            .utimensat(AtFlags::AT_SYMLINK_NOFOLLOW, &times)?;
    }

    #[cfg(not(target_os = "openbsd"))]
    {
        use std::os::unix::fs::Lutimes;
        
        let times = [
            std::os::unix::fs::TimeVal {
                tv_sec: timestamp.seconds(),
                tv_usec: (timestamp.nanoseconds() / 1000) as i64,
            },
            std::os::unix::fs::TimeVal {
                tv_sec: timestamp.seconds(),
                tv_usec: (timestamp.nanoseconds() / 1000) as i64,
            },
        ];

        std::fs::OpenOptions::new()
            .read(true)
            .open(pathname.as_ref())?
            .lutimes(&times)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_timestamp_creation() {
        let timestamp = Timestamp::new(1234567890, 123456789);
        assert_eq!(timestamp.seconds(), 1234567890);
        assert_eq!(timestamp.nanoseconds(), 123456789);
    }

    #[test]
    fn test_set_timestamp() -> Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("test_file");
        
        // Create a test file
        File::create(&file_path)?;
        
        // Create a test timestamp
        let timestamp = Timestamp::new(1234567890, 0);
        
        // Set the timestamp
        set_timestamp(&file_path, timestamp)?;
        
        // Verify the timestamp was set (platform-specific)
        #[cfg(target_os = "linux")]
        {
            use std::os::unix::fs::MetadataExt;
            let metadata = file_path.metadata()?;
            assert_eq!(metadata.mtime(), timestamp.seconds());
            assert_eq!(metadata.mtime_nsec(), timestamp.nanoseconds());
        }
        
        Ok(())
    }

    #[test]
    fn test_symlink_timestamp() -> Result<()> {
        let dir = tempdir()?;
        let target_path = dir.path().join("target");
        let symlink_path = dir.path().join("symlink");
        
        // Create a target file and symlink
        File::create(&target_path)?;
        std::os::unix::fs::symlink(&target_path, &symlink_path)?;
        
        // Create a test timestamp
        let timestamp = Timestamp::new(1234567890, 0);
        
        // Set the timestamp on the symlink
        set_timestamp(&symlink_path, timestamp)?;
        
        Ok(())
    }
} 