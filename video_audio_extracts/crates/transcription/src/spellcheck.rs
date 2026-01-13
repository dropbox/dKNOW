//! Post-processing spell correction for transcribed text
//!
//! Focuses on correcting proper nouns (brands, companies, products, names)
//! that are commonly mis-transcribed by speech recognition systems.

use strsim::jaro_winkler;
use tracing::debug;

/// Proper noun dictionary for spell correction
///
/// This dictionary contains commonly mis-transcribed proper nouns
/// with their correct spellings. The system uses fuzzy matching
/// to detect and correct these terms.
#[derive(Debug, Clone)]
pub struct ProperNounDictionary {
    /// List of correct proper noun spellings
    entries: Vec<String>,
}

impl Default for ProperNounDictionary {
    fn default() -> Self {
        Self::new()
    }
}

impl ProperNounDictionary {
    /// Create a new dictionary with default entries
    #[must_use]
    pub fn new() -> Self {
        let entries = vec![
            // Audio/Media brands
            "LibriVox".to_string(),
            "Spotify".to_string(),
            "YouTube".to_string(),
            "SoundCloud".to_string(),
            "iTunes".to_string(),

            // Tech companies
            "Google".to_string(),
            "Microsoft".to_string(),
            "Apple".to_string(),
            "Amazon".to_string(),
            "Facebook".to_string(),
            "Meta".to_string(),
            "Tesla".to_string(),
            "Netflix".to_string(),

            // Common proper nouns
            "Shakespeare".to_string(),
            "LinkedIn".to_string(),
            "GitHub".to_string(),
            "Instagram".to_string(),
            "Twitter".to_string(),
            "WhatsApp".to_string(),
            "TikTok".to_string(),
            "Dropbox".to_string(),

            // Products
            "iPhone".to_string(),
            "iPad".to_string(),
            "MacBook".to_string(),
            "PlayStation".to_string(),
            "Xbox".to_string(),
            "Android".to_string(),

            // More audio/book related
            "Audible".to_string(),
            "Kindle".to_string(),
            "Goodreads".to_string(),
        ];

        Self { entries }
    }

    /// Add a custom entry to the dictionary
    pub fn add_entry(&mut self, entry: String) {
        if !self.entries.contains(&entry) {
            self.entries.push(entry);
        }
    }

    /// Find the best matching correction for a word
    ///
    /// Returns the corrected word if a good match is found (similarity >= threshold),
    /// otherwise returns None.
    ///
    /// # Arguments
    /// * `word` - The word to check/correct
    /// * `threshold` - Minimum similarity score (0.0-1.0) to accept correction
    pub fn find_correction(&self, word: &str, threshold: f64) -> Option<String> {
        // Skip very short words or words that are already in dictionary
        if word.len() < 3 {
            return None;
        }

        // Case-insensitive exact match
        if self.entries.iter().any(|e| e.eq_ignore_ascii_case(word)) {
            return None; // Already correct
        }

        let mut best_match: Option<(String, f64)> = None;

        for entry in &self.entries {
            // Use Jaro-Winkler similarity (good for proper nouns)
            let similarity = jaro_winkler(&word.to_lowercase(), &entry.to_lowercase());

            if similarity >= threshold {
                if let Some((_, best_score)) = &best_match {
                    if similarity > *best_score {
                        best_match = Some((entry.clone(), similarity));
                    }
                } else {
                    best_match = Some((entry.clone(), similarity));
                }
            }
        }

        if let Some((correction, score)) = best_match {
            debug!(
                "Spell correction: '{}' -> '{}' (similarity: {:.3})",
                word, correction, score
            );
            Some(correction)
        } else {
            None
        }
    }

    /// Correct a full text by checking each word
    ///
    /// Preserves punctuation and whitespace. Only corrects words
    /// that have high similarity to dictionary entries.
    ///
    /// # Arguments
    /// * `text` - The text to correct
    /// * `threshold` - Minimum similarity score (0.0-1.0) to accept correction
    #[must_use]
    pub fn correct_text(&self, text: &str, threshold: f64) -> String {
        let mut result = String::with_capacity(text.len());
        let mut current_word = String::new();

        for ch in text.chars() {
            if ch.is_alphanumeric() {
                current_word.push(ch);
            } else {
                // End of word - check for correction
                if !current_word.is_empty() {
                    if let Some(correction) = self.find_correction(&current_word, threshold) {
                        result.push_str(&correction);
                    } else {
                        result.push_str(&current_word);
                    }
                    current_word.clear();
                }
                result.push(ch);
            }
        }

        // Handle last word if text doesn't end with punctuation
        if !current_word.is_empty() {
            if let Some(correction) = self.find_correction(&current_word, threshold) {
                result.push_str(&correction);
            } else {
                result.push_str(&current_word);
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match_no_correction() {
        let dict = ProperNounDictionary::new();

        // Exact matches should return None (already correct)
        assert_eq!(dict.find_correction("LibriVox", 0.9), None);
        assert_eq!(dict.find_correction("librivox", 0.9), None); // Case insensitive
        assert_eq!(dict.find_correction("LIBRIVOX", 0.9), None);
    }

    #[test]
    fn test_similar_word_correction() {
        let dict = ProperNounDictionary::new();

        // Common misspellings
        assert_eq!(
            dict.find_correction("Libravox", 0.85),
            Some("LibriVox".to_string())
        );

        assert_eq!(
            dict.find_correction("Youtub", 0.85),
            Some("YouTube".to_string())
        );

        assert_eq!(
            dict.find_correction("Shakspeare", 0.85),
            Some("Shakespeare".to_string())
        );
    }

    #[test]
    fn test_correct_full_text() {
        let dict = ProperNounDictionary::new();

        let input = "This is a Libravox recording. All Libravox recordings are free.";
        let expected = "This is a LibriVox recording. All LibriVox recordings are free.";

        assert_eq!(dict.correct_text(input, 0.85), expected);
    }

    #[test]
    fn test_preserve_punctuation() {
        let dict = ProperNounDictionary::new();

        let input = "Visit Youtub, Gogle, and Facbook today!";
        let output = dict.correct_text(input, 0.85);

        assert!(output.contains("YouTube"));
        assert!(output.contains("Google"));
        assert!(output.contains("Facebook"));
        assert!(output.contains("!"));
    }

    #[test]
    fn test_no_false_positives() {
        let dict = ProperNounDictionary::new();

        // Words that shouldn't be corrected
        let input = "The quick brown fox jumps over the lazy dog.";
        let output = dict.correct_text(input, 0.9);

        assert_eq!(input, output); // Should remain unchanged
    }

    #[test]
    fn test_threshold_filtering() {
        let dict = ProperNounDictionary::new();

        // With high threshold, low-similarity words shouldn't be corrected
        assert_eq!(dict.find_correction("Xyz", 0.95), None);

        // But with lower threshold, close matches work
        assert!(dict.find_correction("Libravox", 0.80).is_some());
    }
}
