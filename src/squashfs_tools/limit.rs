use std::sync::Once;
use libc::{rlimit, RLIMIT_NOFILE, RLIM_INFINITY};
use crate::error::SquashError;

/// Margin of file descriptors to leave free for system use
pub const OPEN_FILE_MARGIN: i32 = 10;

/// Get the maximum number of files that can be opened
/// 
/// This function returns the system's file descriptor limit minus a margin
/// for system use. The result is cached after the first call.
/// 
/// Returns:
/// - A positive number indicating the maximum number of files that can be opened
/// - -1 if there is no limit (RLIM_INFINITY)
/// - 1 if there is an error or no margin available
pub fn file_limit() -> i32 {
    static mut MAX_FILES: i32 = -2;
    static INIT: Once = Once::new();

    unsafe {
        INIT.call_once(|| {
            let mut rlim = rlimit {
                rlim_cur: 0,
                rlim_max: 0,
            };

            let res = libc::getrlimit(RLIMIT_NOFILE, &mut rlim);
            if res == -1 {
                eprintln!("Failed to get open file limit! Defaulting to 1");
                MAX_FILES = 1;
            } else if rlim.rlim_cur != RLIM_INFINITY {
                if rlim.rlim_cur <= OPEN_FILE_MARGIN as u64 {
                    // No margin available, use minimum possible
                    MAX_FILES = 1;
                } else {
                    MAX_FILES = (rlim.rlim_cur - OPEN_FILE_MARGIN as u64) as i32;
                }
            } else {
                MAX_FILES = -1;
            }
        });

        MAX_FILES
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_limit() {
        let limit = file_limit();
        assert!(limit >= 1 || limit == -1);
        
        // Test that the limit is cached
        let limit2 = file_limit();
        assert_eq!(limit, limit2);
    }
} 