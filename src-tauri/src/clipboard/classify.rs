/// A clip counts as a link when the whole text is a single http(s) URL.
pub fn is_link(text: &str) -> bool {
    let t = text.trim();
    (t.starts_with("http://") || t.starts_with("https://"))
        && t.len() > 8
        && !t.contains(char::is_whitespace)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_single_urls() {
        assert!(is_link("https://example.com/a?b=c"));
        assert!(is_link("  http://localhost:1420  "));
    }

    #[test]
    fn rejects_non_links() {
        assert!(!is_link("hello https://example.com"));
        assert!(!is_link("https://"));
        assert!(!is_link("example.com"));
        assert!(!is_link("https://a.com b"));
    }
}
