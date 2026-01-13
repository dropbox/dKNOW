// tests/test_cjk.rs

use markdown_chunker::segmentation::{cjk, unicode::SentenceSegmenter};

#[test]
fn test_cjk_detection() {
    assert!(cjk::has_cjk("これは日本語です"));
    assert!(cjk::has_cjk("这是中文"));
    assert!(cjk::has_cjk("한국어입니다"));
    assert!(!cjk::has_cjk("English"));
}

#[test]
fn test_language_detection() {
    assert_eq!(cjk::detect_cjk_language("これは日本語です"), Some("ja"));
    assert_eq!(cjk::detect_cjk_language("这是中文"), Some("zh"));
    assert_eq!(cjk::detect_cjk_language("한국어입니다"), Some("ko"));
    assert_eq!(cjk::detect_cjk_language("English text"), None);
}

#[test]
fn test_japanese_sentences() {
    let text = "これは文です。これは別の文です。";
    let sentences = SentenceSegmenter::split_japanese(text);
    assert_eq!(sentences.len(), 2);
    assert_eq!(sentences[0], "これは文です");
    assert_eq!(sentences[1], "これは別の文です");
}

#[test]
fn test_chinese_sentences() {
    let text = "这是第一句。这是第二句！";
    let sentences = SentenceSegmenter::split_chinese(text);
    assert_eq!(sentences.len(), 2);
}

#[test]
fn test_universal_segmentation() {
    let text = "This is sentence one. This is sentence two.";
    let sentences = SentenceSegmenter::split_universal(text);
    assert!(sentences.len() >= 2);
}

#[test]
fn test_arabic_detection() {
    assert!(cjk::has_arabic("مرحبا بالعالم"));
    assert!(cjk::has_arabic("Hello مرحبا")); // Mixed
    assert!(!cjk::has_arabic("Hello world"));
}

#[test]
fn test_arabic_sentences() {
    let text = "هذه جملة. هذه جملة أخرى؟"; // "This is a sentence. This is another sentence?"
    let sentences = SentenceSegmenter::split_arabic(text);

    assert!(sentences.len() >= 2);
    assert!(!sentences[0].is_empty());
    assert!(!sentences[1].is_empty());
}
