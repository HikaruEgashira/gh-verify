use super::*;

// --- short_sha boundary mutations ---

#[test]
fn short_sha_exactly_7_chars() {
    // Kills: >= 7 → > 7
    assert_eq!(short_sha("abcdefg"), "abcdefg");
}

#[test]
fn short_sha_empty_string() {
    assert_eq!(short_sha(""), "");
}
