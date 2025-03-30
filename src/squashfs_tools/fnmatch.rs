use std::ffi::c_int;

/// Constants for fnmatch flags
pub const FNM_NOESCAPE: c_int = 0x01;
pub const FNM_PATHNAME: c_int = 0x02;
pub const FNM_PERIOD: c_int = 0x04;
pub const FNM_LEADING_DIR: c_int = 0x08;
pub const FNM_CASEFOLD: c_int = 0x10;
pub const FNM_EXTMATCH: c_int = 0x20;

/// Return values for fnmatch
pub const FNM_NOMATCH: c_int = 1;
pub const FNM_MATCH: c_int = 0;

/// Error types for fnmatch operations
#[derive(Debug)]
pub enum FnmatchError {
    InvalidPattern(String),
    InvalidFlags(String),
    Other(String),
}

impl std::fmt::Display for FnmatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FnmatchError::InvalidPattern(msg) => write!(f, "Invalid pattern: {}", msg),
            FnmatchError::InvalidFlags(msg) => write!(f, "Invalid flags: {}", msg),
            FnmatchError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for FnmatchError {}

pub type FnmatchResult<T> = Result<T, FnmatchError>;

/// Match a filename against a pattern
/// 
/// # Arguments
/// 
/// * `pattern` - The pattern to match against
/// * `string` - The string to match
/// * `flags` - Match flags (FNM_* constants)
/// 
/// # Returns
/// 
/// * `FnmatchResult<c_int>` - FNM_MATCH (0) if matched, FNM_NOMATCH (1) if not matched
pub fn fnmatch(pattern: &str, string: &str, flags: c_int) -> FnmatchResult<c_int> {
    // Validate flags
    if (flags & !(FNM_NOESCAPE | FNM_PATHNAME | FNM_PERIOD | FNM_LEADING_DIR | FNM_CASEFOLD | FNM_EXTMATCH)) != 0 {
        return Err(FnmatchError::InvalidFlags("Invalid flags specified".to_string()));
    }

    // Convert pattern to regex if needed
    let regex_pattern = if (flags & FNM_EXTMATCH) != 0 {
        convert_pattern_to_regex(pattern)?
    } else {
        pattern.to_string()
    };

    // Perform the match
    let matched = if (flags & FNM_CASEFOLD) != 0 {
        string.to_lowercase().contains(&regex_pattern.to_lowercase())
    } else {
        string.contains(&regex_pattern)
    };

    Ok(if matched { FNM_MATCH } else { FNM_NOMATCH })
}

/// Convert a fnmatch pattern to a regex pattern
fn convert_pattern_to_regex(pattern: &str) -> FnmatchResult<String> {
    let mut regex = String::new();
    let mut chars = pattern.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '*' => regex.push_str(".*"),
            '?' => regex.push('.'),
            '[' => {
                regex.push('[');
                while let Some(&c) = chars.peek() {
                    if c == ']' {
                        chars.next();
                        regex.push(']');
                        break;
                    }
                    regex.push(c);
                    chars.next();
                }
            },
            '\\' => {
                if let Some(c) = chars.next() {
                    regex.push('\\');
                    regex.push(c);
                } else {
                    return Err(FnmatchError::InvalidPattern("Invalid escape sequence".to_string()));
                }
            },
            c => regex.push(c),
        }
    }

    Ok(regex)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_matching() {
        assert_eq!(fnmatch("*.txt", "test.txt", 0).unwrap(), FNM_MATCH);
        assert_eq!(fnmatch("*.txt", "test.doc", 0).unwrap(), FNM_NOMATCH);
    }

    #[test]
    fn test_case_folding() {
        assert_eq!(fnmatch("*.TXT", "test.txt", FNM_CASEFOLD).unwrap(), FNM_MATCH);
        assert_eq!(fnmatch("*.txt", "test.TXT", FNM_CASEFOLD).unwrap(), FNM_MATCH);
    }

    #[test]
    fn test_pathname_matching() {
        assert_eq!(fnmatch("*.txt", "dir/test.txt", FNM_PATHNAME).unwrap(), FNM_MATCH);
        assert_eq!(fnmatch("*.txt", "dir/subdir/test.txt", FNM_PATHNAME).unwrap(), FNM_MATCH);
    }

    #[test]
    fn test_invalid_flags() {
        assert!(fnmatch("*.txt", "test.txt", 0x100).is_err());
    }

    #[test]
    fn test_pattern_conversion() {
        let pattern = "test[0-9]*.txt";
        let regex = convert_pattern_to_regex(pattern).unwrap();
        assert_eq!(regex, "test[0-9].*.txt");
    }
} 