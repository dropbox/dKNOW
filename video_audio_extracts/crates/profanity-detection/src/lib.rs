//! Profanity Detection Module
//!
//! Detects profane language in text (typically from transcription output).
//! Uses a comprehensive word list to identify profanity with configurable
//! severity levels and context extraction.

pub mod plugin;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use tracing::{debug, info};

/// Profanity detection errors
#[derive(Debug, Error)]
pub enum ProfanityError {
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Processing error: {0}")]
    ProcessingError(String),
}

pub type Result<T> = std::result::Result<T, ProfanityError>;

/// Severity level of profanity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Mild profanity (damn, hell, crap)
    Mild,
    /// Moderate profanity (ass, bitch)
    Moderate,
    /// Strong profanity (f-word, explicit sexual terms)
    Strong,
    /// Severe profanity (slurs, hate speech)
    Severe,
}

/// A detected profane word with context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfanityMatch {
    /// The profane word detected
    pub word: String,
    /// Severity level
    pub severity: Severity,
    /// Start time in seconds (if from transcription)
    pub start: Option<f64>,
    /// End time in seconds (if from transcription)
    pub end: Option<f64>,
    /// Context text around the profane word
    pub context: String,
}

/// Configuration for profanity detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfanityConfig {
    /// Minimum severity level to detect (filters below this level)
    pub min_severity: Severity,
    /// Whether to match word boundaries (recommended: true)
    pub word_boundaries: bool,
    /// Number of context words before/after profanity
    pub context_words: usize,
}

impl Default for ProfanityConfig {
    fn default() -> Self {
        Self {
            min_severity: Severity::Mild,
            word_boundaries: true,
            context_words: 3,
        }
    }
}

/// Profanity detector
pub struct ProfanityDetector {
    config: ProfanityConfig,
    word_list: HashMap<String, Severity>,
}

impl ProfanityDetector {
    /// Create a new profanity detector with default configuration
    pub fn new() -> Self {
        Self::with_config(ProfanityConfig::default())
    }

    /// Create a profanity detector with custom configuration
    pub fn with_config(config: ProfanityConfig) -> Self {
        let word_list = Self::build_word_list();
        info!(
            "Profanity detector initialized with {} word patterns, min_severity={:?}",
            word_list.len(),
            config.min_severity
        );

        Self { config, word_list }
    }

    /// Build comprehensive profanity word list
    /// Returns map of normalized_word -> severity for O(1) lookups
    fn build_word_list() -> HashMap<String, Severity> {
        // Pre-allocate capacity for all profanity words (14 mild + 17 moderate + 20 strong + 7 severe = 58 total)
        let mut words = HashMap::with_capacity(58);

        // Mild profanity
        for word in &[
            "damn", "dammit", "dang", "hell", "crap", "crappy", "suck", "sucks", "piss", "pissed",
            "pissing", "ass", "asses", "asshole", "assholes",
        ] {
            words.insert(word.to_string(), Severity::Mild);
        }

        // Moderate profanity
        for word in &[
            "bitch",
            "bitches",
            "bitching",
            "bastard",
            "bastards",
            "shit",
            "shits",
            "shitty",
            "bullshit",
            "horseshit",
            "dipshit",
            "jackass",
            "dumbass",
            "dick",
            "dicks",
            "dickhead",
            "cock",
            "cocks",
        ] {
            words.insert(word.to_string(), Severity::Moderate);
        }

        // Strong profanity (explicit sexual/anatomical terms)
        for word in &[
            "fuck",
            "fucking",
            "fucked",
            "fucker",
            "fucks",
            "motherfucker",
            "motherfucking",
            "pussy",
            "pussies",
            "cunt",
            "cunts",
            "whore",
            "whores",
            "slut",
            "sluts",
            "tits",
            "titties",
            "penis",
            "vagina",
        ] {
            words.insert(word.to_string(), Severity::Strong);
        }

        // Severe profanity (slurs, hate speech - representative examples)
        // Note: This is a minimal set for demonstration. Production systems
        // would use more comprehensive lists from established sources.
        for word in &[
            "fag", "faggot", "dyke", "retard", "retarded", "spic", "chink",
        ] {
            words.insert(word.to_string(), Severity::Severe);
        }

        words
    }

    /// Detect profanity in plain text
    pub fn detect_in_text(&self, text: &str) -> Vec<ProfanityMatch> {
        let normalized = text.to_lowercase();
        let word_count = normalized.split_whitespace().count();
        let mut words: Vec<&str> = Vec::with_capacity(word_count);
        words.extend(normalized.split_whitespace());
        // Pre-allocate capacity for expected matches (typically small, use words count as upper bound)
        let mut matches = Vec::with_capacity(words.len().min(8));

        for (i, word) in words.iter().enumerate() {
            // Strip punctuation for matching
            let alnum_count = word.chars().filter(|c| c.is_alphanumeric()).count();
            let mut clean_word: String = String::with_capacity(alnum_count);
            clean_word.extend(word.chars().filter(|c| c.is_alphanumeric()));

            // O(1) lookup in HashMap
            if let Some(&severity) = self.word_list.get(&clean_word) {
                // Filter by minimum severity
                if self.severity_level(severity) >= self.severity_level(self.config.min_severity) {
                    // Extract context
                    let context = self.extract_context(&words, i);

                    debug!(
                        "Detected profanity: '{}' (severity={:?}) in context: '{}'",
                        &clean_word, severity, &context
                    );

                    matches.push(ProfanityMatch {
                        word: clean_word,
                        severity,
                        start: None,
                        end: None,
                        context,
                    });
                }
            }
        }

        matches
    }

    /// Detect profanity in transcription segments
    pub fn detect_in_segments(
        &self,
        segments: &[(f64, f64, String)], // (start, end, text)
    ) -> Vec<ProfanityMatch> {
        // Pre-allocate capacity for expected total matches (typically few matches per segment)
        let mut all_matches = Vec::with_capacity(segments.len() * 2);

        for (start, end, text) in segments {
            let mut matches = self.detect_in_text(text);

            // Add timing information
            for m in &mut matches {
                m.start = Some(*start);
                m.end = Some(*end);
            }

            all_matches.extend(matches);
        }

        all_matches
    }

    /// Extract context words around the profanity
    fn extract_context(&self, words: &[&str], index: usize) -> String {
        let start = index.saturating_sub(self.config.context_words);
        let end = (index + 1 + self.config.context_words).min(words.len());

        words[start..end].join(" ")
    }

    /// Convert severity to numeric level for comparison
    fn severity_level(&self, severity: Severity) -> u8 {
        match severity {
            Severity::Mild => 1,
            Severity::Moderate => 2,
            Severity::Strong => 3,
            Severity::Severe => 4,
        }
    }

    /// Get current configuration
    pub fn config(&self) -> &ProfanityConfig {
        &self.config
    }
}

impl Default for ProfanityDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mild_profanity_detection() {
        let detector = ProfanityDetector::new();
        let text = "Oh damn, that really sucks!";
        let matches = detector.detect_in_text(text);

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].word, "damn");
        assert_eq!(matches[0].severity, Severity::Mild);
        assert_eq!(matches[1].word, "sucks");
    }

    #[test]
    fn test_moderate_profanity_detection() {
        let detector = ProfanityDetector::new();
        let text = "This is bullshit and you know it.";
        let matches = detector.detect_in_text(text);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].word, "bullshit");
        assert_eq!(matches[0].severity, Severity::Moderate);
    }

    #[test]
    fn test_strong_profanity_detection() {
        let detector = ProfanityDetector::new();
        let text = "What the fuck is going on here?";
        let matches = detector.detect_in_text(text);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].word, "fuck");
        assert_eq!(matches[0].severity, Severity::Strong);
    }

    #[test]
    fn test_severity_filtering() {
        let config = ProfanityConfig {
            min_severity: Severity::Strong,
            ..Default::default()
        };

        let detector = ProfanityDetector::with_config(config);
        let text = "Damn, this shit is fucking broken!";
        let matches = detector.detect_in_text(text);

        // Should only detect "fucking" (Strong), not "damn" (Mild) or "shit" (Moderate)
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].word, "fucking");
        assert_eq!(matches[0].severity, Severity::Strong);
    }

    #[test]
    fn test_context_extraction() {
        let detector = ProfanityDetector::new();
        let text = "I think this is a really damn good idea overall.";
        let matches = detector.detect_in_text(text);

        assert_eq!(matches.len(), 1);
        assert!(matches[0].context.contains("really damn good"));
    }

    #[test]
    fn test_punctuation_handling() {
        let detector = ProfanityDetector::new();
        let text = "What the hell?! This is crap...";
        let matches = detector.detect_in_text(text);

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].word, "hell");
        assert_eq!(matches[1].word, "crap");
    }

    #[test]
    fn test_case_insensitive() {
        let detector = ProfanityDetector::new();
        let text = "DAMN, Damn, damn!";
        let matches = detector.detect_in_text(text);

        assert_eq!(matches.len(), 3);
        for m in matches {
            assert_eq!(m.word, "damn");
        }
    }

    #[test]
    fn test_no_profanity() {
        let detector = ProfanityDetector::new();
        let text = "This is a perfectly clean sentence.";
        let matches = detector.detect_in_text(text);

        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_detect_in_segments() {
        let detector = ProfanityDetector::new();
        let segments = vec![
            (0.0, 2.5, "Hello there, how are you?".to_string()),
            (2.5, 5.0, "Oh damn, that really sucks!".to_string()),
            (5.0, 7.5, "This is bullshit.".to_string()),
        ];

        let matches = detector.detect_in_segments(&segments);

        assert_eq!(matches.len(), 3);
        assert_eq!(matches[0].start, Some(2.5));
        assert_eq!(matches[0].end, Some(5.0));
        assert_eq!(matches[0].word, "damn");
    }
}
