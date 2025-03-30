use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use super::progress::ProgressBar;
use super::signals::{SignalHandler, SignalType};
use super::thread::ThreadManager;

/// Directory entry structure
#[derive(Debug)]
pub struct DirEntry {
    pub name: String,
    pub subpath: String,
}

/// Info manager for handling filesystem information display
pub struct InfoManager {
    current_entry: Arc<Mutex<Option<DirEntry>>>,
    thread_manager: ThreadManager,
    progress_bar: Arc<Mutex<Option<Box<dyn ProgressBar>>>>,
}

impl InfoManager {
    /// Create a new info manager
    pub fn new() -> Self {
        InfoManager {
            current_entry: Arc::new(Mutex::new(None)),
            thread_manager: ThreadManager::new(),
            progress_bar: Arc::new(Mutex::new(None)),
        }
    }

    /// Disable info display
    pub fn disable_info(&self) {
        let mut entry = self.current_entry.lock().unwrap();
        *entry = None;
    }

    /// Update the current directory entry
    pub fn update_info(&self, dir_entry: DirEntry) {
        let mut entry = self.current_entry.lock().unwrap();
        *entry = Some(dir_entry);
    }

    /// Print the current filename
    fn print_filename(&self) {
        let entry = self.current_entry.lock().unwrap();
        if let Some(dir_entry) = entry.as_ref() {
            if !dir_entry.subpath.is_empty() {
                println!("{}/{}", dir_entry.subpath, dir_entry.name);
            } else {
                println!("/{}", dir_entry.name);
            }
        }
    }

    /// Dump the current state of queues and caches
    fn dump_state(&self) {
        // TODO: Implement queue and cache dumping
        println!("Queues, caches and threads status dump");
        println!("======================================");
        
        // Disable progress bar during dump
        if let Some(progress_bar) = self.progress_bar.lock().unwrap().as_mut() {
            progress_bar.disable();
        }

        // Dump various queues and caches
        // This will be implemented when the queue and cache modules are available
        
        // Re-enable progress bar after dump
        if let Some(progress_bar) = self.progress_bar.lock().unwrap().as_mut() {
            progress_bar.enable();
        }
    }

    /// Initialize the info manager
    pub fn init(&self) {
        let current_entry = self.current_entry.clone();
        let progress_bar = self.progress_bar.clone();
        
        let info_thread = thread::spawn(move || {
            let mut waiting = false;
            let signal_handler = SignalHandler::new();

            loop {
                match signal_handler.wait_for_signal() {
                    SignalType::Quit if !waiting => {
                        // Print current filename
                        let entry = current_entry.lock().unwrap();
                        if entry.is_some() {
                            if let Some(progress_bar) = progress_bar.lock().unwrap().as_mut() {
                                if let Some(dir_entry) = entry.as_ref() {
                                    if !dir_entry.subpath.is_empty() {
                                        progress_bar.info(&format!("{}/{}", dir_entry.subpath, dir_entry.name));
                                    } else {
                                        progress_bar.info(&format!("/{}", dir_entry.name));
                                    }
                                }
                            }
                        }

                        // Set one second interval period
                        waiting = true;
                        thread::sleep(Duration::from_secs(1));
                    }
                    SignalType::Quit | SignalType::Hup => {
                        // Dump state
                        println!("Dumping state...");
                        // TODO: Implement state dumping
                        waiting = false;
                    }
                    _ => {}
                }
            }
        });

        self.thread_manager.register_thread("info", info_thread);
    }

    /// Set the progress bar for info display
    pub fn set_progress_bar(&self, progress_bar: Box<dyn ProgressBar>) {
        let mut pb = self.progress_bar.lock().unwrap();
        *pb = Some(progress_bar);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::progressbar::DefaultProgressBar;

    #[test]
    fn test_info_manager() {
        let manager = InfoManager::new();
        let progress_bar = Box::new(DefaultProgressBar::new());
        manager.set_progress_bar(progress_bar);

        // Test updating info
        let dir_entry = DirEntry {
            name: "test.txt".to_string(),
            subpath: "test/dir".to_string(),
        };
        manager.update_info(dir_entry);

        // Test disabling info
        manager.disable_info();

        // Initialize the manager
        manager.init();
    }
} 