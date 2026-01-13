// src/token_counter.rs

pub struct TokenCounter;

impl TokenCounter {
    /// Estimate token count for multilingual text
    /// Supports: English, CJK (Chinese/Japanese/Korean), Arabic, and other languages
    ///
    /// Performance: O(1) for pure ASCII/English, O(n) for multilingual content
    pub fn estimate(text: &str) -> usize {
        // Fast path for empty text
        if text.is_empty() {
            return 0;
        }

        // Ultra-fast path for pure ASCII/English (most common case)
        // This avoids expensive character iteration for English documents
        if text.is_ascii() {
            return text.len() / 4;
        }

        // Multilingual path: Single pass detection for non-ASCII content
        let mut char_count = 0;
        let mut cjk_count = 0;
        let mut arabic_count = 0;

        for c in text.chars() {
            char_count += 1;
            if is_cjk_char(c) {
                cjk_count += 1;
            } else if is_arabic_char(c) {
                arabic_count += 1;
            }
        }

        // Determine predominant script and calculate tokens
        if cjk_count > 0 {
            // Mixed CJK: CJK chars ~2 chars/token, others ~4 chars/token
            let non_cjk = char_count - cjk_count;
            (cjk_count / 2) + (non_cjk / 4)
        } else if arabic_count > char_count / 2 {
            // Predominantly Arabic: ~5 chars per token
            char_count / 5
        } else {
            // Default (Latin, Cyrillic, etc.): ~4 chars per token
            char_count / 4
        }
    }
}

#[inline]
fn is_cjk_char(c: char) -> bool {
    let code = c as u32;
    (0x4E00..=0x9FFF).contains(&code) || // CJK Unified Ideographs
    (0x3040..=0x309F).contains(&code) || // Hiragana
    (0x30A0..=0x30FF).contains(&code) || // Katakana
    (0xAC00..=0xD7AF).contains(&code)    // Hangul
}

#[inline]
fn is_arabic_char(c: char) -> bool {
    let code = c as u32;
    (0x0600..=0x06FF).contains(&code) || // Arabic
    (0x0750..=0x077F).contains(&code) || // Arabic Supplement
    (0x08A0..=0x08FF).contains(&code) || // Arabic Extended-A
    (0xFB50..=0xFDFF).contains(&code) || // Arabic Presentation Forms-A
    (0xFE70..=0xFEFF).contains(&code)    // Arabic Presentation Forms-B
}
