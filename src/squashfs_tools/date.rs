use std::process::{Command, Stdio};
use std::io::{self, Read};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug)]
pub enum DateError {
    CommandError(String),
    ParseError(String),
    InvalidTimestamp(String),
    IOError(io::Error),
}

impl std::fmt::Display for DateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DateError::CommandError(msg) => write!(f, "Command error: {}", msg),
            DateError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            DateError::InvalidTimestamp(msg) => write!(f, "Invalid timestamp: {}", msg),
            DateError::IOError(e) => write!(f, "IO error: {}", e),
        }
    }
}

impl std::error::Error for DateError {}

pub type DateResult<T> = Result<T, DateError>;

/// Executes the date command to parse a date string and returns the Unix timestamp
/// 
/// # Arguments
/// 
/// * `date_string` - The date string to parse
/// 
/// # Returns
/// 
/// * `DateResult<u32>` - The Unix timestamp as a u32, or an error if parsing failed
/// 
/// # Examples
/// 
/// ```
/// use squashfs_tools::date::exec_date;
/// 
/// let timestamp = exec_date("2024-01-01").unwrap();
/// assert!(timestamp > 0);
/// ```
pub fn exec_date(date_string: &str) -> DateResult<u32> {
    let mut child = Command::new("/usr/bin/date")
        .args(&["-d", date_string, "+%s"])
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| DateError::CommandError(format!("Failed to spawn date command: {}", e)))?;

    let mut output = String::new();
    child.stdout
        .as_mut()
        .ok_or_else(|| DateError::CommandError("Failed to get stdout".to_string()))?
        .read_to_string(&mut output)
        .map_err(DateError::IOError)?;

    let status = child.wait()
        .map_err(|e| DateError::CommandError(format!("Failed to wait for date command: {}", e)))?;

    if !status.success() {
        return Err(DateError::CommandError("Date command failed".to_string()));
    }

    // Remove trailing newline
    let output = output.trim();

    // Parse the timestamp
    let timestamp: i64 = output.parse()
        .map_err(|e| DateError::ParseError(format!("Failed to parse timestamp: {}", e)))?;

    // Validate timestamp range
    if timestamp < 0 {
        return Err(DateError::InvalidTimestamp(
            "Dates should be on or after the epoch of 1970-01-01 00:00 UTC".to_string()
        ));
    }

    if timestamp > u32::MAX as i64 {
        return Err(DateError::InvalidTimestamp(
            "Dates should be before 2106-02-07 06:28:16 UTC".to_string()
        ));
    }

    Ok(timestamp as u32)
}

/// Gets the current Unix timestamp
/// 
/// # Returns
/// 
/// * `DateResult<u32>` - The current Unix timestamp as a u32, or an error if getting the time failed
pub fn get_current_timestamp() -> DateResult<u32> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| DateError::CommandError(format!("Failed to get current time: {}", e)))
        .map(|duration| duration.as_secs() as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exec_date() {
        let timestamp = exec_date("2024-01-01").unwrap();
        assert!(timestamp > 0);
    }

    #[test]
    fn test_exec_date_invalid() {
        assert!(exec_date("invalid date").is_err());
    }

    #[test]
    fn test_exec_date_past() {
        assert!(exec_date("1969-12-31").is_err());
    }

    #[test]
    fn test_exec_date_future() {
        assert!(exec_date("2106-02-07 06:28:17").is_err());
    }

    #[test]
    fn test_get_current_timestamp() {
        let timestamp = get_current_timestamp().unwrap();
        assert!(timestamp > 0);
    }
} 