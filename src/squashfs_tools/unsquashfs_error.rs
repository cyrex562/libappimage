use std::process;
use crate::squashfs_tools::error::{Error, Result};

/// Global state for error handling
pub struct ErrorState {
    /// Whether to ignore errors instead of exiting
    pub ignore_errors: bool,
    /// Whether to be strict about errors
    pub strict_errors: bool,
}

impl Default for ErrorState {
    fn default() -> Self {
        Self {
            ignore_errors: false,
            strict_errors: false,
        }
    }
}

/// Print an informational message
pub fn info(message: &str) {
    // TODO: Implement progress bar info display
    println!("{}", message);
}

/// Print an error message and exit if not ignoring errors
pub fn bad_error(message: &str) -> ! {
    eprintln!("FATAL ERROR: {}", message);
    process::exit(1);
}

/// Print an error message and exit if not ignoring errors
pub fn exit_unsquash(message: &str) -> ! {
    bad_error(message);
}

/// Print an error message and exit if not ignoring errors, respecting ignore_errors flag
pub fn exit_unsquash_ignore(message: &str, state: &ErrorState) {
    if state.ignore_errors {
        eprintln!("{}", message);
    } else {
        bad_error(message);
    }
}

/// Print an error message and exit if strict_errors is enabled
pub fn exit_unsquash_strict(message: &str, state: &ErrorState) {
    if !state.strict_errors {
        eprintln!("{}", message);
    } else {
        bad_error(message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_state_default() {
        let state = ErrorState::default();
        assert!(!state.ignore_errors);
        assert!(!state.strict_errors);
    }

    #[test]
    fn test_info() {
        // This test just ensures the function doesn't panic
        info("Test message");
    }

    #[test]
    fn test_exit_unsquash_ignore() {
        let mut state = ErrorState::default();
        
        // Test with ignore_errors = false
        state.ignore_errors = false;
        exit_unsquash_ignore("Test error", &state);
        
        // Test with ignore_errors = true
        state.ignore_errors = true;
        exit_unsquash_ignore("Test error", &state);
    }

    #[test]
    fn test_exit_unsquash_strict() {
        let mut state = ErrorState::default();
        
        // Test with strict_errors = false
        state.strict_errors = false;
        exit_unsquash_strict("Test error", &state);
        
        // Test with strict_errors = true
        state.strict_errors = true;
        exit_unsquash_strict("Test error", &state);
    }
} 