// tests/test_token_counter.rs

use markdown_chunker::token_counter::TokenCounter;

#[test]
fn test_english_token_estimation() {
    let text = "This is a test."; // 15 chars
    let estimate = TokenCounter::estimate(text);
    // ~4 chars per token, so 15/4 = 3
    assert_eq!(estimate, 3);
}

#[test]
fn test_japanese_token_estimation() {
    let text = "これは日本語です"; // 8 chars
    let estimate = TokenCounter::estimate(text);
    // ~2 chars per token, so 8/2 = 4
    assert_eq!(estimate, 4);
}

#[test]
fn test_chinese_token_estimation() {
    let text = "这是中文"; // 4 chars
    let estimate = TokenCounter::estimate(text);
    // ~2 chars per token, so 4/2 = 2
    assert_eq!(estimate, 2);
}

#[test]
fn test_mixed_content() {
    let text = "Hello 世界"; // 5 English + 1 space + 2 CJK = 8 chars
    let estimate = TokenCounter::estimate(text);
    // Improved: CJK chars (2) = 2/2 = 1 token, non-CJK (6) = 6/4 = 1 token
    // Total: 1 + 1 = 2 tokens (more accurate for mixed content)
    assert_eq!(estimate, 2);
}

#[test]
fn test_arabic_token_estimation() {
    let text = "مرحبا بالعالم"; // "Hello world" in Arabic, ~13 chars
    let estimate = TokenCounter::estimate(text);
    // Arabic: ~5 chars per token, so 13/5 = 2
    assert_eq!(estimate, 2);
}

#[test]
fn test_ascii_fast_path_correctness() {
    // Verify ASCII fast path gives same result as character counting
    let text = "Hello world this is a test";
    let estimate = TokenCounter::estimate(text);

    // ASCII: 26 chars (len() == chars().count() for ASCII)
    // 26 / 4 = 6
    assert_eq!(estimate, 6);

    // Verify consistency with longer ASCII text
    let long_ascii = "a".repeat(400);
    assert_eq!(TokenCounter::estimate(&long_ascii), 100);
}

#[test]
fn test_non_ascii_uses_char_count() {
    // Text with non-ASCII but non-CJK/Arabic (e.g., accented characters)
    let text = "Café résumé naïve"; // 17 characters, but more bytes due to accents
    let estimate = TokenCounter::estimate(text);

    // Should use char count (17), not byte count
    // 17 chars / 4 = 4 tokens
    assert_eq!(estimate, 4);
}
