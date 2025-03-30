use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use libappimage::utils::{Logger, LogLevel};

#[test]
fn test_instance() {
    let logger1 = Logger::get_instance();
    let logger2 = Logger::get_instance();
    assert!(std::ptr::eq(logger1, logger2));
}

#[test]
fn test_set_callback() {
    let logger = Logger::get_instance();
    let level_set = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let message_set = Arc::new(std::sync::Mutex::new(String::new()));
    
    let level_set_clone = level_set.clone();
    let message_set_clone = message_set.clone();
    
    logger.set_callback(Box::new(move |level: LogLevel, message: &str| {
        level_set_clone.store(level as usize, Ordering::SeqCst);
        *message_set_clone.lock().unwrap() = message.to_string();
    }));

    logger.log(LogLevel::Error, "Hello");

    assert_eq!(level_set.load(Ordering::SeqCst), LogLevel::Error as usize);
    assert_eq!(*message_set.lock().unwrap(), "Hello");
} 