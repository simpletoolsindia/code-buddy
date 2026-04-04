//! General utility functions for Code Buddy.

/// Redact a sensitive string for safe logging.
/// Preserves the first 4 and last 4 characters to aid in debugging,
/// replacing the middle with asterisks.
///
/// # Examples
/// ```
/// use code_buddy_utils::redact;
/// assert_eq!(redact("sk-1234567890abcdef"), "sk-1****cdef");
/// assert_eq!(redact("short"), "****");
/// assert_eq!(redact(""), "****");
/// ```
#[must_use]
pub fn redact(secret: &str) -> String {
    let len = secret.len();
    if len <= 8 {
        return "****".to_string();
    }
    let prefix = &secret[..4];
    let suffix = &secret[len - 4..];
    format!("{prefix}****{suffix}")
}

/// Truncate a string to a maximum byte length, appending `...` if truncated.
#[must_use]
pub fn truncate(s: &str, max_bytes: usize) -> String {
    if s.len() <= max_bytes {
        return s.to_string();
    }
    // Find safe UTF-8 boundary
    let boundary = s
        .char_indices()
        .rev()
        .find(|(i, _)| *i <= max_bytes.saturating_sub(3))
        .map_or(0, |(i, _)| i);
    format!("{}...", &s[..boundary])
}

/// Strip ANSI escape codes from a string.
#[must_use]
pub fn strip_ansi(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            // Skip until 'm' (end of ANSI sequence)
            for c in chars.by_ref() {
                if c == 'm' {
                    break;
                }
            }
        } else {
            result.push(ch);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redact_short_string() {
        assert_eq!(redact("short"), "****");
        assert_eq!(redact(""), "****");
    }

    #[test]
    fn redact_long_string() {
        let result = redact("sk-1234567890abcdef");
        assert!(result.contains("****"));
        assert!(result.starts_with("sk-1"));
        assert!(result.ends_with("cdef"));
    }

    #[test]
    fn truncate_short() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn truncate_long() {
        let result = truncate("hello world", 8);
        assert!(result.ends_with("..."));
        assert!(result.len() <= 11);
    }
}
