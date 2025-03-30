use std::sync::Once;
use std::sync::Mutex;
use std::sync::Arc;
use std::io::{self, Write};

/// Log levels supported by the logger
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    /// Debug level for detailed information
    Debug,
    /// Info level for general information
    Info,
    /// Warning level for potential issues
    Warning,
    /// Error level for serious problems
    Error,
}

/// Type alias for the logging callback function
pub type LogCallback = Box<dyn Fn(LogLevel, &str) + Send + Sync>;

/// A singleton logger that provides global logging functionality
pub struct Logger {
    callback: Arc<Mutex<LogCallback>>,
}

impl Logger {
    /// Get the singleton instance of the logger
    pub fn get_instance() -> &'static Logger {
        static INSTANCE: Once = Once::new();
        static mut LOGGER: Option<Logger> = None;

        unsafe {
            INSTANCE.call_once(|| {
                LOGGER = Some(Logger::new());
            });
            LOGGER.as_ref().unwrap()
        }
    }

    /// Create a new logger instance with default callback
    fn new() -> Self {
        let default_callback = Box::new(|level: LogLevel, message: &str| {
            let mut stderr = io::stderr();
            let prefix = match level {
                LogLevel::Debug => "DEBUG: ",
                LogLevel::Info => "INFO: ",
                LogLevel::Warning => "WARNING: ",
                LogLevel::Error => "ERROR: ",
            };
            let _ = write!(stderr, "{}{}\n", prefix, message);
        });

        Logger {
            callback: Arc::new(Mutex::new(default_callback)),
        }
    }

    /// Set a custom logging callback
    pub fn set_callback(&self, callback: LogCallback) {
        let mut guard = self.callback.lock().unwrap();
        *guard = callback;
    }

    /// Log a message with the specified level
    pub fn log(&self, level: LogLevel, message: &str) {
        let callback = self.callback.lock().unwrap();
        callback(level, message);
    }

    /// Log a debug message
    pub fn debug(message: &str) {
        Self::get_instance().log(LogLevel::Debug, message);
    }

    /// Log an info message
    pub fn info(message: &str) {
        Self::get_instance().log(LogLevel::Info, message);
    }

    /// Log a warning message
    pub fn warning(message: &str) {
        Self::get_instance().log(LogLevel::Warning, message);
    }

    /// Log an error message
    pub fn error(message: &str) {
        Self::get_instance().log(LogLevel::Error, message);
    }
}

/// Set a custom logging callback globally
pub fn set_logger_callback(callback: LogCallback) {
    Logger::get_instance().set_callback(callback);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[test]
    fn test_logger_singleton() {
        let logger1 = Logger::get_instance();
        let logger2 = Logger::get_instance();
        assert!(std::ptr::eq(logger1, logger2));
    }

    #[test]
    fn test_logger_callback() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();
        
        let callback = Box::new(move |_level: LogLevel, _message: &str| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });

        set_logger_callback(callback);
        
        Logger::debug("test");
        Logger::info("test");
        Logger::warning("test");
        Logger::error("test");

        assert_eq!(counter.load(Ordering::SeqCst), 4);
    }

    #[test]
    fn test_logger_levels() {
        let mut received_levels = Vec::new();
        let callback = Box::new(move |level: LogLevel, _message: &str| {
            received_levels.push(level);
        });

        set_logger_callback(callback);
        
        Logger::debug("test");
        Logger::info("test");
        Logger::warning("test");
        Logger::error("test");

        assert_eq!(received_levels, vec![
            LogLevel::Debug,
            LogLevel::Info,
            LogLevel::Warning,
            LogLevel::Error,
        ]);
    }
} 