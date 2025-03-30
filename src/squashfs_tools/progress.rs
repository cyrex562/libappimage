use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::fmt::Arguments;
use crate::error::SquashError;
use std::fmt;

/// Structure to manage progress bar state
#[derive(Debug)]
pub struct ProgressBar {
    display_enabled: bool,
    temp_disabled: bool,
    percent_mode: bool,
    need_newline: bool,
    rotate: usize,
    cur_uncompressed: u64,
    estimated_uncompressed: u64,
    columns: usize,
    progress_mutex: Arc<Mutex<()>>,
    size_mutex: Arc<Mutex<()>>,
}

impl ProgressBar {
    /// Create a new progress bar
    pub fn new() -> Result<Self, SquashError> {
        let columns = Self::get_terminal_width()?;
        Ok(Self {
            display_enabled: false,
            temp_disabled: false,
            percent_mode: false,
            need_newline: false,
            rotate: 0,
            cur_uncompressed: 0,
            estimated_uncompressed: 0,
            columns,
            progress_mutex: Arc::new(Mutex::new(())),
            size_mutex: Arc::new(Mutex::new(())),
        })
    }

    /// Get the terminal width
    fn get_terminal_width() -> Result<usize, SquashError> {
        use libc::{winsize, TIOCGWINSZ, STDOUT_FILENO};
        use std::os::unix::io::AsRawFd;

        let stdout = io::stdout();
        let fd = stdout.as_raw_fd();
        let mut winsize: winsize = unsafe { std::mem::zeroed() };

        unsafe {
            if libc::ioctl(fd, TIOCGWINSZ, &mut winsize) == -1 {
                if libc::isatty(STDOUT_FILENO) != 0 {
                    eprintln!("TIOCGWINSZ ioctl failed, defaulting to 80 columns");
                }
                Ok(80)
            } else {
                Ok(winsize.ws_col as usize)
            }
        }
    }

    /// Increment the progress counter
    pub fn increment(&mut self) {
        self.cur_uncompressed = self.cur_uncompressed.saturating_add(1);
    }

    /// Decrement the progress counter
    pub fn decrement(&mut self, count: u64) {
        self.cur_uncompressed = self.cur_uncompressed.saturating_sub(count);
    }

    /// Set the total size for progress calculation
    pub fn set_size(&mut self, count: u64) -> Result<(), SquashError> {
        let _lock = self.size_mutex.lock()
            .map_err(|e| SquashError::Other(format!("Failed to acquire size mutex: {}", e)))?;
        self.estimated_uncompressed = self.estimated_uncompressed.saturating_add(count);
        Ok(())
    }

    /// Enable the progress bar
    pub fn enable(&mut self) -> Result<(), SquashError> {
        let _lock = self.progress_mutex.lock()
            .map_err(|e| SquashError::Other(format!("Failed to acquire progress mutex: {}", e)))?;
        if self.display_enabled {
            self.update_display()?;
            self.need_newline = true;
        }
        self.temp_disabled = false;
        Ok(())
    }

    /// Disable the progress bar
    pub fn disable(&mut self) -> Result<(), SquashError> {
        let _lock = self.progress_mutex.lock()
            .map_err(|e| SquashError::Other(format!("Failed to acquire progress mutex: {}", e)))?;
        if self.need_newline {
            print!("\n");
            self.need_newline = false;
        }
        self.temp_disabled = true;
        Ok(())
    }

    /// Set the progress bar state
    pub fn set_state(&mut self, enabled: bool) -> Result<(), SquashError> {
        let _lock = self.progress_mutex.lock()
            .map_err(|e| SquashError::Other(format!("Failed to acquire progress mutex: {}", e)))?;
        
        if self.display_enabled != enabled {
            if self.display_enabled && !self.temp_disabled {
                self.update_display()?;
                print!("\n");
                self.need_newline = false;
            }
            self.display_enabled = enabled;
        }
        Ok(())
    }

    /// Set percentage-only mode
    pub fn set_percentage_mode(&mut self) {
        self.percent_mode = true;
    }

    /// Update the progress display
    fn update_display(&mut self) -> Result<(), SquashError> {
        if self.percent_mode {
            self.display_percentage()?;
        } else {
            self.display_progress_bar()?;
        }
        Ok(())
    }

    /// Display the progress bar
    fn display_progress_bar(&mut self) -> Result<(), SquashError> {
        let rotate_chars = ['|', '/', '-', '\\'];
        let max_digits = if self.estimated_uncompressed == 0 {
            1
        } else {
            (self.estimated_uncompressed as f64).log10().floor() as usize + 1
        };

        let used = if self.estimated_uncompressed == 0 {
            13
        } else {
            max_digits * 2 + 11
        };

        let hashes = if self.estimated_uncompressed == 0 {
            0
        } else {
            (self.cur_uncompressed * (self.columns - used) as u64) / self.estimated_uncompressed
        } as usize;

        let spaces = self.columns - used - hashes;
        let percentage = if self.estimated_uncompressed == 0 {
            100
        } else {
            (self.cur_uncompressed * 100) / self.estimated_uncompressed
        } as usize;

        if self.cur_uncompressed > self.estimated_uncompressed || self.columns < used {
            return Ok(());
        }

        print!("\r[");
        for _ in 0..hashes {
            print!("=");
        }
        print!("{}", rotate_chars[self.rotate]);
        for _ in 0..spaces {
            print!(" ");
        }
        print!("] {:>width$}/{:>width$} {:>3}%",
            self.cur_uncompressed,
            self.estimated_uncompressed,
            percentage,
            width = max_digits
        );
        io::stdout().flush()?;
        Ok(())
    }

    /// Display percentage only
    fn display_percentage(&mut self) -> Result<(), SquashError> {
        let percentage = if self.estimated_uncompressed == 0 {
            100
        } else {
            (self.cur_uncompressed * 100) / self.estimated_uncompressed
        } as usize;

        println!("{}", percentage);
        io::stdout().flush()?;
        Ok(())
    }

    /// Print an error message
    pub fn error(&mut self, args: Arguments) -> Result<(), SquashError> {
        let _lock = self.progress_mutex.lock()
            .map_err(|e| SquashError::Other(format!("Failed to acquire progress mutex: {}", e)))?;

        if self.need_newline {
            print!("\n");
            self.need_newline = false;
        }

        print!("{}", args);
        io::stdout().flush()?;
        Ok(())
    }

    /// Print an info message
    pub fn info(&mut self, args: Arguments) -> Result<(), SquashError> {
        let _lock = self.progress_mutex.lock()
            .map_err(|e| SquashError::Other(format!("Failed to acquire progress mutex: {}", e)))?;

        if self.need_newline {
            print!("\n");
            self.need_newline = false;
        }

        print!("{}", args);
        io::stdout().flush()?;
        Ok(())
    }

    /// Start the progress bar thread
    pub fn start_thread(&mut self) -> Result<(), SquashError> {
        let progress_mutex = self.progress_mutex.clone();
        let progress_bar = Arc::new(Mutex::new(self));

        thread::spawn(move || {
            let sleep_duration = Duration::from_millis(250);
            loop {
                thread::sleep(sleep_duration);
                if let Ok(mut progress) = progress_bar.lock() {
                    progress.rotate = (progress.rotate + 1) % 4;
                    if progress.display_enabled && !progress.temp_disabled {
                        if let Err(e) = progress.update_display() {
                            eprintln!("Failed to update progress display: {}", e);
                        }
                        progress.need_newline = true;
                    }
                }
            }
        });

        Ok(())
    }
}

/// Progress bar interface for displaying progress and errors
pub trait ProgressBarTrait {
    fn error(&mut self, fmt: fmt::Arguments<'_>) -> io::Result<()>;
    fn info(&mut self, fmt: fmt::Arguments<'_>) -> io::Result<()>;
    fn disable(&mut self);
    fn enable(&mut self);
}

/// Default implementation using stdout/stderr
pub struct DefaultProgressBar {
    stdout: io::Stdout,
    stderr: io::Stderr,
    enabled: bool,
}

impl DefaultProgressBar {
    pub fn new() -> Self {
        DefaultProgressBar {
            stdout: io::stdout(),
            stderr: io::stderr(),
            enabled: true,
        }
    }
}

impl ProgressBarTrait for DefaultProgressBar {
    fn error(&mut self, fmt: fmt::Arguments<'_>) -> io::Result<()> {
        if self.enabled {
            writeln!(self.stderr, "{}", fmt)
        } else {
            Ok(())
        }
    }

    fn info(&mut self, fmt: fmt::Arguments<'_>) -> io::Result<()> {
        if self.enabled {
            writeln!(self.stdout, "{}", fmt)
        } else {
            Ok(())
        }
    }

    fn disable(&mut self) {
        self.enabled = false;
    }

    fn enable(&mut self) {
        self.enabled = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_bar_creation() {
        let progress = ProgressBar::new().unwrap();
        assert!(!progress.display_enabled);
        assert!(!progress.temp_disabled);
        assert!(!progress.percent_mode);
        assert_eq!(progress.cur_uncompressed, 0);
        assert_eq!(progress.estimated_uncompressed, 0);
    }

    #[test]
    fn test_progress_bar_increment() {
        let mut progress = ProgressBar::new().unwrap();
        progress.increment();
        assert_eq!(progress.cur_uncompressed, 1);
    }

    #[test]
    fn test_progress_bar_decrement() {
        let mut progress = ProgressBar::new().unwrap();
        progress.cur_uncompressed = 10;
        progress.decrement(3);
        assert_eq!(progress.cur_uncompressed, 7);
    }

    #[test]
    fn test_progress_bar_set_size() {
        let mut progress = ProgressBar::new().unwrap();
        progress.set_size(100).unwrap();
        assert_eq!(progress.estimated_uncompressed, 100);
    }
} 