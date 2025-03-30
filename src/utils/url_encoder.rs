/// Provides a minimal implementation of the Uniform Resource Identifiers (RFC 2396)
/// See: https://tools.ietf.org/html/rfc2396
pub struct UrlEncoder;

impl UrlEncoder {
    /// Characters that don't need to be encoded according to RFC 2396
    const UNRESERVED_CHARS: &'static [char] = &[
        'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm',
        'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
        'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M',
        'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
        '0', '1', '2', '3', '4', '5', '6', '7', '8', '9',
        '-', '_', '.', '~', '/'
    ];

    /// Escape characters in the given value according to RFC 2396
    /// 
    /// # Arguments
    /// 
    /// * `value` - The string to encode
    /// 
    /// # Returns
    /// 
    /// The URL-encoded string
    /// 
    /// # Example
    /// 
    /// ```
    /// use libappimage::utils::UrlEncoder;
    /// 
    /// assert_eq!(
    ///     UrlEncoder::encode("Hello, World!"),
    ///     "Hello%2C%20World%21"
    /// );
    /// ```
    pub fn encode(value: &str) -> String {
        let mut result = String::with_capacity(value.len() * 3); // Worst case: every char is encoded

        for c in value.chars() {
            if Self::UNRESERVED_CHARS.contains(&c) {
                result.push(c);
            } else {
                // Percent-encode the character
                result.push('%');
                result.push_str(&format!("{:02X}", c as u8));
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_basic() {
        assert_eq!(UrlEncoder::encode("Hello, World!"), "Hello%2C%20World%21");
        assert_eq!(UrlEncoder::encode("file:///path/to/file"), "file%3A%2F%2F%2Fpath%2Fto%2Ffile");
    }

    #[test]
    fn test_encode_special_chars() {
        assert_eq!(UrlEncoder::encode("space space"), "space%20space");
        assert_eq!(UrlEncoder::encode("tab\ttab"), "tab%09tab");
        assert_eq!(UrlEncoder::encode("newline\nnewline"), "newline%0Anewline");
    }

    #[test]
    fn test_encode_unicode() {
        assert_eq!(UrlEncoder::encode("你好"), "%E4%BD%A0%E5%A5%BD");
        assert_eq!(UrlEncoder::encode("éèêë"), "%C3%A9%C3%A8%C3%AA%C3%AB");
    }

    #[test]
    fn test_encode_unreserved_chars() {
        // Test that unreserved characters are not encoded
        for &c in UrlEncoder::UNRESERVED_CHARS {
            assert_eq!(UrlEncoder::encode(&c.to_string()), c.to_string());
        }
    }

    #[test]
    fn test_encode_empty() {
        assert_eq!(UrlEncoder::encode(""), "");
    }

    #[test]
    fn test_encode_mixed() {
        assert_eq!(
            UrlEncoder::encode("Hello, 世界! (World)"),
            "Hello%2C%20%E4%B8%96%E7%95%8C%21%20%28World%29"
        );
    }
} 