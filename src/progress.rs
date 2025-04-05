use std::fmt;
use std::io::{self, Write};

/// Progress bar interface for displaying progress and errors
pub trait ProgressBar {
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

impl ProgressBar for DefaultProgressBar {
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

/// Global progress bar instance
static mut PROGRESS_BAR: Option<Box<dyn ProgressBar>> = None;

/// Initialize the progress bar
pub fn init_progress_bar(bar: Box<dyn ProgressBar>) {
    unsafe {
        PROGRESS_BAR = Some(bar);
    }
}

/// Get the current progress bar instance
fn get_progress_bar() -> &'static mut Box<dyn ProgressBar> {
    unsafe { PROGRESS_BAR.as_mut().expect("Progress bar not initialized") }
}

/// Print an error message
#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {
        if let Err(e) = get_progress_bar().error(format_args!($($arg)*)) {
            eprintln!("Failed to write error message: {}", e);
        }
    };
}

/// Print an info message
#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        if let Err(e) = get_progress_bar().info(format_args!($($arg)*)) {
            eprintln!("Failed to write info message: {}", e);
        }
    };
}

/// Print a trace message (only if SQUASHFS_TRACE is enabled)
#[cfg(feature = "trace")]
#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => {
        info!("squashfs: {}", format_args!($($arg)*));
    };
}

#[cfg(not(feature = "trace"))]
#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => {};
}

/// Handle memory errors
#[macro_export]
macro_rules! mem_error {
    ($func:expr) => {
        error!("FATAL ERROR: Out of memory ({})", $func);
        exit_squashfs();
    };
}