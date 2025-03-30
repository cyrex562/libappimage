use std::path::Path;

/// Sanitizes strings for use in file paths.
/// 
/// This struct provides methods to sanitize strings by replacing unsafe characters
/// with safe alternatives.
pub struct StringSanitizer<'a> {
    input: &'a str,
}

impl<'a> StringSanitizer<'a> {
    /// Creates a new StringSanitizer with the given input string.
    pub fn new(input: &'a str) -> Self {
        Self { input }
    }

    /// Sanitizes the input string for use in file paths.
    /// 
    /// This method replaces unsafe characters with underscores:
    /// - Unsafe characters (/, $, §, %, &, etc.) are replaced with _
    /// - Spaces are replaced with _
    /// - Safe characters (a-z, A-Z, 0-9, -, _, .) are kept as is
    /// 
    /// # Returns
    /// 
    /// The sanitized string
    pub fn sanitize_for_path(&self) -> String {
        if self.input.is_empty() {
            return String::new();
        }

        self.input.chars().map(|c| {
            match c {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' => c,
                _ => '_',
            }
        }).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_for_path_with_empty_path() {
        let sanitizer = StringSanitizer::new("");
        let actual = sanitizer.sanitize_for_path();
        assert!(actual.is_empty());
    }

    #[test]
    fn test_sanitize_for_path_with_already_safe_string() {
        let already_safe_string = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_.";
        let sanitizer = StringSanitizer::new(already_safe_string);
        let actual = sanitizer.sanitize_for_path();
        assert_eq!(actual, already_safe_string);
    }

    #[test]
    fn test_sanitize_for_path_with_unsafe_path() {
        let unsafe_string = "/../$§%&testabcdefg";
        let expected = "_..______testabcdefg";
        let sanitizer = StringSanitizer::new(unsafe_string);
        let actual = sanitizer.sanitize_for_path();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_sanitize_for_path_with_spaces() {
        let unsafe_string = "test string abcdefg hijklmnop ";
        let expected = "test_string_abcdefg_hijklmnop_";
        let sanitizer = StringSanitizer::new(unsafe_string);
        let actual = sanitizer.sanitize_for_path();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_sanitize_for_path_with_special_characters() {
        let unsafe_string = "test@#$%^&*()_+{}|:\"<>?`~[]\\;',./";
        let expected = "test_____________________________";
        let sanitizer = StringSanitizer::new(unsafe_string);
        let actual = sanitizer.sanitize_for_path();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_sanitize_for_path_with_unicode() {
        let unsafe_string = "test🎉🌟✨";
        let expected = "test___";
        let sanitizer = StringSanitizer::new(unsafe_string);
        let actual = sanitizer.sanitize_for_path();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_sanitize_for_path_with_mixed_characters() {
        let unsafe_string = "Test123!@#$%^&*()_+{}|:\"<>?`~[]\\;',./";
        let expected = "Test123_____________________________";
        let sanitizer = StringSanitizer::new(unsafe_string);
        let actual = sanitizer.sanitize_for_path();
        assert_eq!(actual, expected);
    }
} 