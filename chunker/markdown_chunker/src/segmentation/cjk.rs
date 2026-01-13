// src/segmentation/cjk.rs

/// Check if text contains CJK characters
pub fn has_cjk(text: &str) -> bool {
    text.chars().any(is_cjk_char)
}

/// Check if a single character is CJK
#[inline]
fn is_cjk_char(c: char) -> bool {
    let code = c as u32;
    (0x4E00..=0x9FFF).contains(&code) || // CJK Unified Ideographs
    (0x3040..=0x309F).contains(&code) || // Hiragana
    (0x30A0..=0x30FF).contains(&code) || // Katakana
    (0xAC00..=0xD7AF).contains(&code)    // Hangul
}

/// Check if text contains Arabic characters
pub fn has_arabic(text: &str) -> bool {
    text.chars().any(is_arabic_char)
}

/// Check if a single character is Arabic
#[inline]
fn is_arabic_char(c: char) -> bool {
    let code = c as u32;
    (0x0600..=0x06FF).contains(&code) || // Arabic
    (0x0750..=0x077F).contains(&code) || // Arabic Supplement
    (0x08A0..=0x08FF).contains(&code) || // Arabic Extended-A
    (0xFB50..=0xFDFF).contains(&code) || // Arabic Presentation Forms-A
    (0xFE70..=0xFEFF).contains(&code)    // Arabic Presentation Forms-B
}

/// Detect CJK language with single-pass optimization
pub fn detect_cjk_language(text: &str) -> Option<&'static str> {
    let mut has_hiragana = false;
    let mut has_katakana = false;
    let mut has_hangul = false;
    let mut has_cjk_ideograph = false;

    // Single pass through characters
    for c in text.chars() {
        let code = c as u32;

        if !has_hiragana && (0x3040..=0x309F).contains(&code) {
            has_hiragana = true;
        }
        if !has_katakana && (0x30A0..=0x30FF).contains(&code) {
            has_katakana = true;
        }
        if !has_hangul && (0xAC00..=0xD7AF).contains(&code) {
            has_hangul = true;
        }
        if !has_cjk_ideograph && (0x4E00..=0x9FFF).contains(&code) {
            has_cjk_ideograph = true;
        }

        // Early exit if we found definitive markers
        if has_hiragana || has_katakana {
            return Some("ja");
        }
        if has_hangul {
            return Some("ko");
        }
    }

    if has_cjk_ideograph {
        Some("zh")
    } else {
        None
    }
}
