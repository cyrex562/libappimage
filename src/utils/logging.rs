use crate::utils::logger::{LogLevel, LogCallback, set_logger_callback};

/// Re-export the log level enum for public use
pub use crate::utils::logger::LogLevel;

/// Type alias for the logging callback function
pub type LogCallback = crate::utils::logger::LogCallback;

/// Set a custom logging function.
/// Allows to capture the libappimage log messages.
///
/// # Arguments
///
/// * `callback` - The logging function callback to be used for logging messages
///
/// # Example
///
/// ```
/// use libappimage::utils::logging::{set_logger_callback, LogCallback, LogLevel};
///
/// let callback: LogCallback = Box::new(|level: LogLevel, message: &str| {
///     println!("[{}] {}", level, message);
/// });
///
/// set_logger_callback(callback);
/// ```
pub fn set_logger_callback(callback: LogCallback) {
    crate::utils::logger::set_logger_callback(callback);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[test]
    fn test_set_logger_callback() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();
        
        let callback: LogCallback = Box::new(move |_level: LogLevel, _message: &str| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });

        set_logger_callback(callback);
        
        // Use the logger to verify the callback is working
        crate::utils::logger::Logger::debug("test");
        crate::utils::logger::Logger::info("test");
        crate::utils::logger::Logger::warning("test");
        crate::utils::logger::Logger::error("test");

        assert_eq!(counter.load(Ordering::SeqCst), 4);
    }
} 