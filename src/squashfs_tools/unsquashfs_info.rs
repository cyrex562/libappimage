use std::sync::{Arc, Mutex};
use std::thread;
use signal_hook::{consts::signal::*, iterator::Signals};
use crate::squashfs_tools::error::{Error, Result};
use crate::squashfs_tools::unsquashfs_error::info;
use crate::squashfs_tools::progress::{disable_progress_bar, enable_progress_bar};
use crate::squashfs_tools::queue::{dump_queue, to_reader, to_inflate, to_writer};
use crate::squashfs_tools::cache::{dump_cache, data_cache, fragment_cache};

/// Thread-safe wrapper for the current pathname
#[derive(Default)]
struct InfoState {
    pathname: Option<String>,
}

impl InfoState {
    fn new() -> Self {
        Self {
            pathname: None,
        }
    }

    fn set_pathname(&mut self, name: Option<String>) {
        self.pathname = name;
    }

    fn get_pathname(&self) -> Option<&str> {
        self.pathname.as_deref()
    }
}

/// Global state for info handling
static INFO_STATE: Mutex<InfoState> = Mutex::new(InfoState::new());

/// Disable info display by clearing the current pathname
pub fn disable_info() {
    if let Ok(mut state) = INFO_STATE.lock() {
        state.set_pathname(None);
    }
}

/// Update the current pathname being processed
pub fn update_info(name: String) {
    if let Ok(mut state) = INFO_STATE.lock() {
        state.set_pathname(Some(name));
    }
}

/// Dump the current state of queues and caches
pub fn dump_state() {
    disable_progress_bar();

    println!("Queue and cache status dump");
    println!("===========================");

    println!("file buffer read queue (main thread -> reader thread)");
    dump_queue(to_reader);

    println!("file buffer decompress queue (reader thread -> inflate thread(s))");
    dump_queue(to_inflate);

    println!("file buffer write queue (main thread -> writer thread)");
    dump_queue(to_writer);

    println!("\nbuffer cache (uncompressed blocks and compressed blocks 'in flight')");
    dump_cache(data_cache);

    println!("fragment buffer cache (uncompressed frags and compressed frags 'in flight')");
    dump_cache(fragment_cache);

    enable_progress_bar();
}

/// Info thread function that handles signals
fn info_thread() -> Result<()> {
    let mut signals = Signals::new(&[SIGQUIT, SIGHUP])?;
    let mut waiting = false;

    for sig in signals.forever() {
        match sig {
            SIGQUIT if !waiting => {
                if let Ok(state) = INFO_STATE.lock() {
                    if let Some(pathname) = state.get_pathname() {
                        info(pathname);
                    }
                }
                waiting = true;
            }
            SIGQUIT | SIGHUP => {
                dump_state();
                waiting = false;
            }
            _ => unreachable!(),
        }
    }

    Ok(())
}

/// Initialize the info thread
pub fn init_info() -> Result<()> {
    let handle = thread::spawn(info_thread);
    
    // Store the thread handle for later cleanup if needed
    // Note: In a real implementation, you might want to store this handle
    // in a static variable or struct for proper cleanup
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_info_state() {
        let mut state = InfoState::new();
        assert!(state.get_pathname().is_none());
        
        state.set_pathname(Some("test".to_string()));
        assert_eq!(state.get_pathname(), Some("test"));
        
        state.set_pathname(None);
        assert!(state.get_pathname().is_none());
    }

    #[test]
    fn test_disable_info() {
        update_info("test".to_string());
        disable_info();
        if let Ok(state) = INFO_STATE.lock() {
            assert!(state.get_pathname().is_none());
        }
    }

    #[test]
    fn test_update_info() {
        update_info("test".to_string());
        if let Ok(state) = INFO_STATE.lock() {
            assert_eq!(state.get_pathname(), Some("test"));
        }
    }

    #[test]
    fn test_dump_state() {
        // This test just ensures the function doesn't panic
        dump_state();
    }
} 