// src/segmentation/unicode.rs

use unicode_segmentation::UnicodeSegmentation;

pub struct SentenceSegmenter;

impl SentenceSegmenter {
    /// Universal (works for all languages)
    pub fn split_universal(text: &str) -> Vec<String> {
        text.unicode_sentences().map(str::to_string).collect()
    }

    /// Language-specific (better accuracy)
    pub fn split_with_language(text: &str, language: &str) -> Vec<String> {
        match language {
            "ja" => Self::split_japanese(text),
            "zh" | "zh-CN" | "zh-TW" => Self::split_chinese(text),
            "ar" | "ar-SA" | "ar-EG" => Self::split_arabic(text),
            _ => Self::split_universal(text),
        }
    }

    pub fn split_japanese(text: &str) -> Vec<String> {
        text.split(['。', '！', '？'])
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }

    pub fn split_chinese(text: &str) -> Vec<String> {
        text.split(['。', '！', '？', '；'])
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }

    pub fn split_arabic(text: &str) -> Vec<String> {
        // Arabic uses period (.), question mark (؟), and exclamation mark (!) for sentences
        text.split(['.', '؟', '!', '،'])
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }
}
