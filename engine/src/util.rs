/// Truncate a string to at most `max_chars` characters (char-safe, never
/// splits a UTF-8 sequence). Used everywhere we slice user/model output for
/// previews — byte-index slicing panics on multi-byte characters.
pub fn truncate_chars(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        s.chars().take(max_chars).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_ascii() {
        assert_eq!(truncate_chars("hello world", 5), "hello");
    }

    #[test]
    fn truncate_short_string_untouched() {
        assert_eq!(truncate_chars("hi", 10), "hi");
    }

    #[test]
    fn truncate_multibyte_does_not_panic() {
        // kaomoji / box-drawing chars are 3 bytes in UTF-8; slicing at a byte
        // offset would panic, but truncate_chars must not.
        let s = "(╯°□°)╯︵ ┻━┻ 123456789";
        let out = truncate_chars(s, 5);
        assert_eq!(out.chars().count(), 5);
        assert!(s.starts_with(&out));
    }

    #[test]
    fn truncate_exact_boundary() {
        let s = "abcd";
        assert_eq!(truncate_chars(s, 4), "abcd");
        assert_eq!(truncate_chars(s, 5), "abcd");
    }
}
