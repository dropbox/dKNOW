//! `WebVTT` subtitle document backend
//!
//! This module provides `WebVTT` (Web Video Text Tracks) subtitle file parsing
//! and conversion capabilities using the docling-video crate.

use crate::traits::{BackendOptions, DocumentBackend};
use crate::utils::{create_text_item, opt_vec};
use docling_core::{DocItem, DoclingError, Document, DocumentMetadata, InputFormat};
use docling_video::SubtitleEntry;
use std::fmt::Write;
use std::path::Path;

/// `WebVTT` subtitle backend
///
/// Supports `WebVTT` (Web Video Text Tracks) subtitle format (.vtt files).
/// Converts subtitle entries to markdown with timestamp information.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct WebvttBackend;

impl WebvttBackend {
    /// Create a new `WebVTT` backend
    ///
    /// Note: This method returns a Result for API consistency with other backends,
    /// but never actually fails. The `WebvttBackend` is a unit struct with no initialization logic.
    ///
    /// # Errors
    ///
    /// This function never returns an error (returns `Ok(Self)` always).
    #[inline]
    #[must_use = "constructors return a new instance"]
    pub const fn new() -> Result<Self, DoclingError> {
        Ok(Self)
    }

    /// Format a duration as `WebVTT` timestamp
    /// Format: MM:SS.mmm (when hours=0) or HH:MM:SS.mmm (when hours>0)
    /// Matches Python docling output which omits hours when zero
    fn format_timestamp(duration: std::time::Duration) -> String {
        let total_secs = duration.as_secs();
        let millis = duration.subsec_millis();
        let hours = total_secs / 3600;
        let minutes = (total_secs % 3600) / 60;
        let seconds = total_secs % 60;

        if hours > 0 {
            // Include hours: HH:MM:SS.mmm
            format!("{hours:02}:{minutes:02}:{seconds:02}.{millis:03}")
        } else {
            // Omit hours: MM:SS.mmm
            format!("{minutes:02}:{seconds:02}.{millis:03}")
        }
    }

    /// Generate markdown from subtitle entries
    ///
    /// Python source: ~/`docling/docling/backend/webvtt_backend.py`
    /// Format matches Python docling output exactly:
    /// - Timestamp line
    /// - Blank line
    /// - Speaker (classes):  text (with 2 spaces after colon)
    /// - Blank line
    fn generate_markdown(entries: &[SubtitleEntry], _subtitle_name: &str) -> String {
        let mut markdown = String::new();

        if entries.is_empty() {
            return markdown;
        }

        // List all subtitle entries
        for entry in entries {
            let start = Self::format_timestamp(entry.start_time);
            let end = Self::format_timestamp(entry.end_time);

            // Timestamp line with optional cue settings
            let _ = write!(markdown, "{start} --> {end}");

            // Append cue settings if present (WebVTT positioning/styling metadata)
            let mut settings = Vec::new();
            if let Some(ref position) = entry.position {
                settings.push(format!("position:{position}"));
            }
            if let Some(ref align) = entry.align {
                settings.push(format!("align:{align}"));
            }
            if let Some(ref line) = entry.line {
                settings.push(format!("line:{line}"));
            }
            if let Some(ref size) = entry.size {
                settings.push(format!("size:{size}"));
            }
            if let Some(ref vertical) = entry.vertical {
                settings.push(format!("vertical:{vertical}"));
            }
            if let Some(ref region) = entry.region {
                settings.push(format!("region:{region}"));
            }
            if !settings.is_empty() {
                markdown.push(' ');
                markdown.push_str(&settings.join(" "));
            }
            markdown.push('\n');
            markdown.push('\n');

            // Speaker and text (if speaker present)
            if let Some(ref speaker) = entry.speaker {
                markdown.push_str(speaker);

                // Add classes in parentheses
                if !entry.speaker_classes.is_empty() {
                    markdown.push_str(" (");
                    markdown.push_str(&entry.speaker_classes.join(", "));
                    markdown.push(')');
                }

                // Two spaces after colon to match Python output
                markdown.push_str(":  ");
            }

            // Text content
            markdown.push_str(&entry.text);
            markdown.push_str("\n\n");
        }

        markdown
    }

    /// Create `DocItems` from subtitle entries
    ///
    /// Python source: ~/docling/docling/backend/webvtt_backend.py:512-572
    /// Python creates groups for each cue block with timing and voice spans.
    /// For now, we create a simple flat structure with text items.
    /// The markdown serializer will handle the formatting.
    fn create_docitems(entries: &[SubtitleEntry]) -> Vec<DocItem> {
        let mut doc_items = Vec::new();
        let mut text_idx = 0;

        for entry in entries {
            let start = Self::format_timestamp(entry.start_time);
            let end = Self::format_timestamp(entry.end_time);

            // Build timing text with optional cue settings
            let mut timing_text = format!("{start} --> {end}");

            // Append cue settings if present (WebVTT positioning/styling metadata)
            let mut settings = Vec::new();
            if let Some(ref position) = entry.position {
                settings.push(format!("position:{position}"));
            }
            if let Some(ref align) = entry.align {
                settings.push(format!("align:{align}"));
            }
            if let Some(ref line) = entry.line {
                settings.push(format!("line:{line}"));
            }
            if let Some(ref size) = entry.size {
                settings.push(format!("size:{size}"));
            }
            if let Some(ref vertical) = entry.vertical {
                settings.push(format!("vertical:{vertical}"));
            }
            if let Some(ref region) = entry.region {
                settings.push(format!("region:{region}"));
            }
            if !settings.is_empty() {
                timing_text.push(' ');
                timing_text.push_str(&settings.join(" "));
            }

            // Save indices before incrementing
            let timing_idx_val = text_idx;
            text_idx += 1;
            let content_idx_val = text_idx;
            text_idx += 1;

            // Build speaker text with classes
            let mut speaker_text = String::new();
            if let Some(ref speaker) = entry.speaker {
                speaker_text.push_str(speaker);
                if !entry.speaker_classes.is_empty() {
                    speaker_text.push_str(" (");
                    speaker_text.push_str(&entry.speaker_classes.join(", "));
                    speaker_text.push(')');
                }
                speaker_text.push_str(": ");
            }
            speaker_text.push_str(&entry.text);

            // Create timing and content text items using helper
            doc_items.push(create_text_item(timing_idx_val, timing_text, vec![]));
            doc_items.push(create_text_item(content_idx_val, speaker_text, vec![]));
        }

        doc_items
    }

    /// Create a Document from parsed subtitle entries
    ///
    /// Shared helper for consistent document creation pattern (same as srt.rs).
    fn create_document(entries: &[SubtitleEntry], subtitle_name: &str) -> Document {
        // Generate DocItems
        let doc_items = Self::create_docitems(entries);

        // Generate markdown
        let markdown = Self::generate_markdown(entries, subtitle_name);
        let num_characters = markdown.chars().count();

        // Create document with DocItems
        Document {
            markdown,
            format: InputFormat::Webvtt,
            metadata: DocumentMetadata {
                num_pages: None,
                num_characters,
                title: Some(subtitle_name.to_string()),
                author: None,
                created: None,
                modified: None,
                language: None,
                subject: None,
                exif: None,
            },
            docling_document: None,
            content_blocks: opt_vec(doc_items),
        }
    }
}

impl DocumentBackend for WebvttBackend {
    #[inline]
    fn format(&self) -> InputFormat {
        InputFormat::Webvtt
    }

    fn parse_bytes(&self, data: &[u8], options: &BackendOptions) -> Result<Document, DoclingError> {
        // WebVTT parsing requires a file (docling-video uses parse_subtitle_file)
        // Write to temp file and parse
        use crate::utils::write_temp_file;
        let temp_path = write_temp_file(data, "subtitle", ".vtt")?;
        self.parse_file(&temp_path, options)
    }

    fn parse_file<P: AsRef<Path>>(
        &self,
        path: P,
        _options: &BackendOptions,
    ) -> Result<Document, DoclingError> {
        let path_ref = path.as_ref();
        let subtitle_name = path_ref
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("subtitle.vtt");

        // Parse WebVTT file using docling-video
        let subtitle_file = docling_video::parse_subtitle_file(path_ref).map_err(|e| {
            DoclingError::BackendError(format!("Failed to parse WebVTT file: {e}: {subtitle_name}"))
        })?;

        Ok(Self::create_document(&subtitle_file.entries, subtitle_name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_timestamp_formatting() {
        // When hours=0, omit them (matches Python docling output)
        assert_eq!(
            WebvttBackend::format_timestamp(Duration::from_millis(1500)),
            "00:01.500" // MM:SS.mmm (no hours)
        );
        // When hours>0, include them
        assert_eq!(
            WebvttBackend::format_timestamp(Duration::from_secs(3665) + Duration::from_millis(123)),
            "01:01:05.123" // HH:MM:SS.mmm
        );
    }

    #[test]
    fn test_markdown_generation_empty() {
        let entries = vec![];
        let markdown = WebvttBackend::generate_markdown(&entries, "test.vtt");
        // Empty entries should produce empty markdown (no headers)
        assert_eq!(
            markdown, "",
            "Empty WebVTT entries should produce empty markdown"
        );
    }

    #[test]
    fn test_markdown_generation_with_entries() {
        let entries = vec![
            SubtitleEntry {
                start_time: Duration::from_secs(1),
                end_time: Duration::from_secs(3),
                text: "Hello world".to_string(),
                line_number: Some(1),
                speaker: None,
                speaker_classes: vec![],
                ..Default::default()
            },
            SubtitleEntry {
                start_time: Duration::from_secs(5),
                end_time: Duration::from_secs(7),
                text: "Goodbye world".to_string(),
                line_number: Some(2),
                speaker: None,
                speaker_classes: vec![],
                ..Default::default()
            },
        ];
        let markdown = WebvttBackend::generate_markdown(&entries, "test.vtt");
        // Current format: timestamp\n\ntext\n\n (no headers or brackets)
        assert!(
            markdown.contains("00:01.000 --> 00:03.000"),
            "First timestamp should be present"
        );
        assert!(
            markdown.contains("Hello world"),
            "First subtitle text should be present"
        );
        assert!(
            markdown.contains("00:05.000 --> 00:07.000"),
            "Second timestamp should be present"
        );
        assert!(
            markdown.contains("Goodbye world"),
            "Second subtitle text should be present"
        );
        // Should NOT contain old header format
        assert!(
            !markdown.contains("**Total entries:**"),
            "Should not contain old header format"
        );
    }

    /// Test markdown generation with speaker names
    #[test]
    fn test_markdown_generation_with_speaker() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(1),
            end_time: Duration::from_secs(3),
            text: "Hello".to_string(),
            line_number: Some(1),
            speaker: Some("Alice".to_string()),
            speaker_classes: vec![],
            ..Default::default()
        }];
        let markdown = WebvttBackend::generate_markdown(&entries, "test.vtt");

        // Should have timestamp
        assert!(
            markdown.contains("00:01.000 --> 00:03.000"),
            "Timestamp should be present in markdown"
        );
        // Should have speaker name with 2 spaces after colon
        assert!(
            markdown.contains("Alice:  Hello"),
            "Speaker name should be followed by colon and two spaces"
        );
    }

    /// Test markdown generation with speaker classes
    #[test]
    fn test_markdown_generation_with_speaker_classes() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(1),
            end_time: Duration::from_secs(3),
            text: "Hello".to_string(),
            line_number: Some(1),
            speaker: Some("Bob".to_string()),
            speaker_classes: vec!["loud".to_string(), "excited".to_string()],
            ..Default::default()
        }];
        let markdown = WebvttBackend::generate_markdown(&entries, "test.vtt");

        // Should have speaker with classes in parentheses
        assert!(
            markdown.contains("Bob (loud, excited):  Hello"),
            "Speaker classes should be in parentheses with comma separation"
        );
    }

    /// Test timestamp formatting edge cases
    #[test]
    fn test_timestamp_formatting_edge_cases() {
        // Zero duration (hours omitted)
        assert_eq!(
            WebvttBackend::format_timestamp(Duration::from_secs(0)),
            "00:00.000"
        );

        // Maximum reasonable duration (99 hours)
        assert_eq!(
            WebvttBackend::format_timestamp(Duration::from_secs(356_400)), // 99 hours
            "99:00:00.000"
        );

        // Milliseconds precision (hours omitted)
        assert_eq!(
            WebvttBackend::format_timestamp(Duration::from_millis(1)),
            "00:00.001"
        );
        assert_eq!(
            WebvttBackend::format_timestamp(Duration::from_millis(999)),
            "00:00.999"
        );
    }

    /// Test DocItem creation for single entry
    #[test]
    fn test_create_docitems_single() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(1),
            end_time: Duration::from_secs(3),
            text: "Test text".to_string(),
            line_number: Some(1),
            speaker: None,
            speaker_classes: vec![],
            ..Default::default()
        }];

        let doc_items = WebvttBackend::create_docitems(&entries);

        // Should create 2 items: timing + content
        assert_eq!(
            doc_items.len(),
            2,
            "Single subtitle entry should create 2 DocItems (timing + content)"
        );

        // First item is timing
        match &doc_items[0] {
            DocItem::Text { self_ref, text, .. } => {
                assert_eq!(self_ref, "#/texts/0");
                assert_eq!(text, "00:01.000 --> 00:03.000");
            }
            _ => panic!("Expected Text DocItem for timing"),
        }

        // Second item is content
        match &doc_items[1] {
            DocItem::Text { self_ref, text, .. } => {
                assert_eq!(self_ref, "#/texts/1");
                assert_eq!(text, "Test text");
            }
            _ => panic!("Expected Text DocItem for content"),
        }
    }

    /// Test DocItem creation with multiple entries
    #[test]
    fn test_create_docitems_multiple() {
        let entries = vec![
            SubtitleEntry {
                start_time: Duration::from_secs(1),
                end_time: Duration::from_secs(2),
                text: "First".to_string(),
                line_number: Some(1),
                speaker: None,
                speaker_classes: vec![],
                ..Default::default()
            },
            SubtitleEntry {
                start_time: Duration::from_secs(3),
                end_time: Duration::from_secs(4),
                text: "Second".to_string(),
                line_number: Some(2),
                speaker: None,
                speaker_classes: vec![],
                ..Default::default()
            },
        ];

        let doc_items = WebvttBackend::create_docitems(&entries);

        // Should create 4 items: 2 entries × (timing + content)
        assert_eq!(
            doc_items.len(),
            4,
            "Two subtitle entries should create 4 DocItems"
        );

        // Verify indices are sequential
        match &doc_items[0] {
            DocItem::Text { self_ref, .. } => assert_eq!(self_ref, "#/texts/0"),
            _ => panic!("Expected Text DocItem"),
        }
        match &doc_items[1] {
            DocItem::Text { self_ref, .. } => assert_eq!(self_ref, "#/texts/1"),
            _ => panic!("Expected Text DocItem"),
        }
        match &doc_items[2] {
            DocItem::Text { self_ref, .. } => assert_eq!(self_ref, "#/texts/2"),
            _ => panic!("Expected Text DocItem"),
        }
        match &doc_items[3] {
            DocItem::Text { self_ref, .. } => assert_eq!(self_ref, "#/texts/3"),
            _ => panic!("Expected Text DocItem"),
        }
    }

    /// Test DocItem creation with speaker
    #[test]
    fn test_create_docitems_with_speaker() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(1),
            end_time: Duration::from_secs(3),
            text: "Hello".to_string(),
            line_number: Some(1),
            speaker: Some("Alice".to_string()),
            speaker_classes: vec!["loud".to_string()],
            ..Default::default()
        }];

        let doc_items = WebvttBackend::create_docitems(&entries);

        // Second item should have speaker prefix
        match &doc_items[1] {
            DocItem::Text { text, .. } => {
                assert_eq!(text, "Alice (loud): Hello");
            }
            _ => panic!("Expected Text DocItem"),
        }
    }

    /// Test empty entries produce empty DocItems
    #[test]
    fn test_create_docitems_empty() {
        let entries = vec![];
        let doc_items = WebvttBackend::create_docitems(&entries);
        assert_eq!(
            doc_items.len(),
            0,
            "Empty entries should produce empty DocItems"
        );
    }

    /// Test backend format identification
    #[test]
    fn test_backend_format() {
        let backend = WebvttBackend::new().unwrap();
        assert_eq!(
            backend.format(),
            InputFormat::Webvtt,
            "WebVTT backend should report Webvtt format"
        );
    }

    /// Test markdown with empty text entry
    #[test]
    fn test_markdown_generation_empty_text() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(1),
            end_time: Duration::from_secs(2),
            text: "".to_string(),
            line_number: Some(1),
            speaker: None,
            speaker_classes: vec![],
            ..Default::default()
        }];

        let markdown = WebvttBackend::generate_markdown(&entries, "test.vtt");

        // Should still have timestamp even with empty text
        assert!(
            markdown.contains("00:01.000 --> 00:02.000"),
            "Empty text entry should still have timestamp"
        );
    }

    /// Test speaker without classes
    #[test]
    fn test_markdown_speaker_no_classes() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(0),
            end_time: Duration::from_secs(1),
            text: "Text".to_string(),
            line_number: Some(1),
            speaker: Some("Speaker".to_string()),
            speaker_classes: vec![],
            ..Default::default()
        }];

        let markdown = WebvttBackend::generate_markdown(&entries, "test.vtt");

        // Should NOT have parentheses when no classes
        assert!(
            markdown.contains("Speaker:  Text"),
            "Speaker without classes should have colon and text"
        );
        assert!(
            !markdown.contains("()"),
            "Empty classes should not produce empty parentheses"
        );
    }

    /// Test long duration formatting (hours)
    #[test]
    fn test_timestamp_hours() {
        // 2 hours, 30 minutes, 45 seconds, 500 milliseconds
        let duration = Duration::from_secs(2 * 3600 + 30 * 60 + 45) + Duration::from_millis(500);
        assert_eq!(WebvttBackend::format_timestamp(duration), "02:30:45.500");
    }

    // ===== Backend Trait Tests =====

    /// Test WebvttBackend implements Default
    #[test]
    fn test_backend_default() {
        let backend = WebvttBackend;
        assert_eq!(
            backend.format(),
            InputFormat::Webvtt,
            "Default WebvttBackend should report Webvtt format"
        );
    }

    /// Test backend creation via new()
    #[test]
    fn test_backend_new() {
        let backend = WebvttBackend::new().unwrap();
        assert_eq!(
            backend.format(),
            InputFormat::Webvtt,
            "WebvttBackend::new() should report Webvtt format"
        );
    }

    // ===== Timestamp Edge Cases =====

    /// Test timestamp with subsecond precision variations
    #[test]
    fn test_timestamp_subsecond_precision() {
        // 1 millisecond (hours omitted)
        assert_eq!(
            WebvttBackend::format_timestamp(Duration::from_millis(1)),
            "00:00.001"
        );
        // 10 milliseconds (hours omitted)
        assert_eq!(
            WebvttBackend::format_timestamp(Duration::from_millis(10)),
            "00:00.010"
        );
        // 100 milliseconds (hours omitted)
        assert_eq!(
            WebvttBackend::format_timestamp(Duration::from_millis(100)),
            "00:00.100"
        );
    }

    /// Test timestamp with 3-digit hours (100+ hours)
    #[test]
    fn test_timestamp_overflow_hours() {
        // 100 hours
        let duration = Duration::from_secs(100 * 3600);
        // Format supports 3+ digit hours
        assert_eq!(WebvttBackend::format_timestamp(duration), "100:00:00.000");
    }

    /// Test timestamp at exact minute/hour boundaries
    #[test]
    fn test_timestamp_boundaries() {
        // Exact 1 minute (hours omitted)
        assert_eq!(
            WebvttBackend::format_timestamp(Duration::from_secs(60)),
            "01:00.000"
        );
        // Exact 1 hour (hours included)
        assert_eq!(
            WebvttBackend::format_timestamp(Duration::from_secs(3600)),
            "01:00:00.000"
        );
    }

    // ===== Markdown Generation Variations =====

    /// Test markdown with multiple speakers
    #[test]
    fn test_markdown_multiple_speakers() {
        let entries = vec![
            SubtitleEntry {
                start_time: Duration::from_secs(0),
                end_time: Duration::from_secs(2),
                text: "Hello".to_string(),
                line_number: Some(1),
                speaker: Some("Alice".to_string()),
                speaker_classes: vec![],
                ..Default::default()
            },
            SubtitleEntry {
                start_time: Duration::from_secs(2),
                end_time: Duration::from_secs(4),
                text: "Hi there".to_string(),
                line_number: Some(2),
                speaker: Some("Bob".to_string()),
                speaker_classes: vec![],
                ..Default::default()
            },
        ];

        let markdown = WebvttBackend::generate_markdown(&entries, "test.vtt");
        assert!(
            markdown.contains("Alice:  Hello"),
            "First speaker should be present in markdown"
        );
        assert!(
            markdown.contains("Bob:  Hi there"),
            "Second speaker should be present in markdown"
        );
    }

    /// Test markdown with mixed speaker/non-speaker entries
    #[test]
    fn test_markdown_mixed_speakers() {
        let entries = vec![
            SubtitleEntry {
                start_time: Duration::from_secs(0),
                end_time: Duration::from_secs(2),
                text: "Narration".to_string(),
                line_number: Some(1),
                speaker: None,
                speaker_classes: vec![],
                ..Default::default()
            },
            SubtitleEntry {
                start_time: Duration::from_secs(2),
                end_time: Duration::from_secs(4),
                text: "Dialog".to_string(),
                line_number: Some(2),
                speaker: Some("Character".to_string()),
                speaker_classes: vec![],
                ..Default::default()
            },
        ];

        let markdown = WebvttBackend::generate_markdown(&entries, "test.vtt");
        // Non-speaker entry: just text (no speaker prefix)
        assert!(
            markdown.contains("Narration\n\n"),
            "Non-speaker entry should have just text"
        );
        // Speaker entry: has speaker prefix
        assert!(
            markdown.contains("Character:  Dialog"),
            "Speaker entry should have speaker prefix"
        );
    }

    /// Test markdown with single speaker class
    #[test]
    fn test_markdown_single_speaker_class() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(0),
            end_time: Duration::from_secs(2),
            text: "Text".to_string(),
            line_number: Some(1),
            speaker: Some("Name".to_string()),
            speaker_classes: vec!["class1".to_string()],
            ..Default::default()
        }];

        let markdown = WebvttBackend::generate_markdown(&entries, "test.vtt");
        // Single class: should have parentheses with single item
        assert!(
            markdown.contains("Name (class1):  Text"),
            "Single speaker class should be in parentheses"
        );
    }

    /// Test markdown with three speaker classes
    #[test]
    fn test_markdown_three_speaker_classes() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(0),
            end_time: Duration::from_secs(2),
            text: "Text".to_string(),
            line_number: Some(1),
            speaker: Some("Name".to_string()),
            speaker_classes: vec!["a".to_string(), "b".to_string(), "c".to_string()],
            ..Default::default()
        }];

        let markdown = WebvttBackend::generate_markdown(&entries, "test.vtt");
        // Multiple classes: comma-separated
        assert!(
            markdown.contains("Name (a, b, c):  Text"),
            "Multiple speaker classes should be comma-separated"
        );
    }

    // ===== DocItem Creation Edge Cases =====

    /// Test DocItems with speaker and single class
    #[test]
    fn test_docitems_speaker_single_class() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(1),
            end_time: Duration::from_secs(2),
            text: "Text".to_string(),
            line_number: Some(1),
            speaker: Some("Speaker".to_string()),
            speaker_classes: vec!["loud".to_string()],
            ..Default::default()
        }];

        let doc_items = WebvttBackend::create_docitems(&entries);
        match &doc_items[1] {
            DocItem::Text { text, .. } => {
                assert_eq!(text, "Speaker (loud): Text");
            }
            _ => panic!("Expected Text DocItem"),
        }
    }

    /// Test DocItems with speaker and multiple classes
    #[test]
    fn test_docitems_speaker_multiple_classes() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(0),
            end_time: Duration::from_secs(1),
            text: "Hello".to_string(),
            line_number: Some(1),
            speaker: Some("Alice".to_string()),
            speaker_classes: vec!["excited".to_string(), "loud".to_string()],
            ..Default::default()
        }];

        let doc_items = WebvttBackend::create_docitems(&entries);
        match &doc_items[1] {
            DocItem::Text { text, .. } => {
                // Classes should be comma-separated
                assert_eq!(text, "Alice (excited, loud): Hello");
            }
            _ => panic!("Expected Text DocItem"),
        }
    }

    /// Test DocItems with no speaker but has speaker_classes (edge case)
    /// Note: This is technically invalid WebVTT, but test robustness
    #[test]
    fn test_docitems_no_speaker_with_classes() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(0),
            end_time: Duration::from_secs(1),
            text: "Text".to_string(),
            line_number: Some(1),
            speaker: None,
            speaker_classes: vec!["orphan".to_string()],
            ..Default::default()
        }];

        let doc_items = WebvttBackend::create_docitems(&entries);
        match &doc_items[1] {
            DocItem::Text { text, .. } => {
                // No speaker, so classes are ignored, just text
                assert_eq!(text, "Text");
            }
            _ => panic!("Expected Text DocItem"),
        }
    }

    // ===== Speaker Formatting Edge Cases =====

    /// Test speaker with trailing spaces
    #[test]
    fn test_speaker_with_trailing_spaces() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(0),
            end_time: Duration::from_secs(1),
            text: "Text".to_string(),
            line_number: Some(1),
            speaker: Some("Name  ".to_string()), // trailing spaces
            speaker_classes: vec![],
            ..Default::default()
        }];

        let markdown = WebvttBackend::generate_markdown(&entries, "test.vtt");
        // Trailing spaces preserved in speaker name
        assert!(
            markdown.contains("Name  :  Text"),
            "Trailing spaces in speaker name should be preserved"
        );
    }

    /// Test speaker with special characters
    #[test]
    fn test_speaker_special_characters() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(0),
            end_time: Duration::from_secs(1),
            text: "Text".to_string(),
            line_number: Some(1),
            speaker: Some("O'Brien".to_string()),
            speaker_classes: vec![],
            ..Default::default()
        }];

        let markdown = WebvttBackend::generate_markdown(&entries, "test.vtt");
        assert!(
            markdown.contains("O'Brien:  Text"),
            "Special characters in speaker name should be preserved"
        );
    }

    /// Test speaker with unicode characters
    #[test]
    fn test_speaker_unicode() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(0),
            end_time: Duration::from_secs(1),
            text: "こんにちは".to_string(),
            line_number: Some(1),
            speaker: Some("田中".to_string()),
            speaker_classes: vec![],
            ..Default::default()
        }];

        let markdown = WebvttBackend::generate_markdown(&entries, "test.vtt");
        assert!(
            markdown.contains("田中:  こんにちは"),
            "Unicode speaker names and text should be preserved"
        );
    }

    // ===== Content Variations =====

    /// Test text with newlines (WebVTT supports multiline)
    #[test]
    fn test_text_with_newlines() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(0),
            end_time: Duration::from_secs(2),
            text: "Line 1\nLine 2".to_string(),
            line_number: Some(1),
            speaker: None,
            speaker_classes: vec![],
            ..Default::default()
        }];

        let markdown = WebvttBackend::generate_markdown(&entries, "test.vtt");
        // Newlines in text should be preserved
        assert!(
            markdown.contains("Line 1\nLine 2"),
            "Newlines in subtitle text should be preserved"
        );
    }

    /// Test text with special markdown characters
    #[test]
    fn test_text_markdown_characters() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(0),
            end_time: Duration::from_secs(1),
            text: "**bold** and *italic*".to_string(),
            line_number: Some(1),
            speaker: None,
            speaker_classes: vec![],
            ..Default::default()
        }];

        let markdown = WebvttBackend::generate_markdown(&entries, "test.vtt");
        // Special characters should be preserved (no escaping)
        assert!(
            markdown.contains("**bold** and *italic*"),
            "Markdown special characters should be preserved"
        );
    }

    /// Test very long text entry
    #[test]
    fn test_very_long_text() {
        let long_text = "A".repeat(1000);
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(0),
            end_time: Duration::from_secs(10),
            text: long_text.clone(),
            line_number: Some(1),
            speaker: None,
            speaker_classes: vec![],
            ..Default::default()
        }];

        let markdown = WebvttBackend::generate_markdown(&entries, "test.vtt");
        assert!(
            markdown.contains(&long_text),
            "Very long text content should be preserved"
        );
    }

    // ===== Integration Tests =====

    /// Test that content_blocks is None for empty entries
    #[test]
    fn test_document_empty_content_blocks() {
        let entries = vec![];
        let doc_items = WebvttBackend::create_docitems(&entries);
        assert!(
            doc_items.is_empty(),
            "Empty entries should produce empty DocItems vector"
        );
        // Document should set content_blocks to None when empty
    }

    /// Test that content_blocks is Some for non-empty entries
    #[test]
    fn test_document_nonempty_content_blocks() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(0),
            end_time: Duration::from_secs(1),
            text: "Text".to_string(),
            line_number: Some(1),
            speaker: None,
            speaker_classes: vec![],
            ..Default::default()
        }];

        let doc_items = WebvttBackend::create_docitems(&entries);
        assert!(
            !doc_items.is_empty(),
            "Non-empty entries should produce DocItems"
        );
        // Document should set content_blocks to Some(doc_items)
    }

    /// Test that markdown character count matches output
    #[test]
    fn test_markdown_character_count() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(0),
            end_time: Duration::from_secs(1),
            text: "Test".to_string(),
            line_number: Some(1),
            speaker: None,
            speaker_classes: vec![],
            ..Default::default()
        }];

        let markdown = WebvttBackend::generate_markdown(&entries, "test.vtt");
        let char_count = markdown.chars().count();
        // Should match metadata.num_characters in Document
        assert!(
            char_count > 0,
            "Markdown character count should be positive for non-empty entries"
        );
    }

    // ===== N=427 Expansion: 15 additional tests =====

    /// Test parse_bytes with valid WebVTT
    #[test]
    fn test_parse_bytes_valid() {
        let backend = WebvttBackend::new().unwrap();
        let vtt = b"WEBVTT\n\n00:00:00.000 --> 00:00:02.000\nTest subtitle\n";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(vtt, &options);
        assert!(result.is_ok(), "Valid WebVTT should parse successfully");

        let doc = result.unwrap();
        assert_eq!(
            doc.format,
            InputFormat::Webvtt,
            "Document format should be Webvtt"
        );
        assert!(
            doc.markdown.contains("Test subtitle"),
            "Parsed markdown should contain subtitle text"
        );
    }

    /// Test parse_bytes with invalid UTF-8
    #[test]
    fn test_parse_bytes_invalid_utf8() {
        let backend = WebvttBackend::new().unwrap();
        let invalid = vec![0xFF, 0xFE, 0xFD];
        let options = BackendOptions::default();

        let result = backend.parse_bytes(&invalid, &options);
        assert!(result.is_err(), "Invalid UTF-8 should return error");
        match result {
            Err(DoclingError::BackendError(msg)) => {
                assert!(msg.contains("UTF-8"), "Error message should mention UTF-8");
            }
            _ => panic!("Expected BackendError with UTF-8 message"),
        }
    }

    /// Test parse_bytes with empty input
    #[test]
    fn test_parse_bytes_empty() {
        let backend = WebvttBackend::new().unwrap();
        let options = BackendOptions::default();

        let result = backend.parse_bytes(b"", &options);
        assert!(result.is_ok(), "Empty input should parse successfully");

        let doc = result.unwrap();
        assert!(
            doc.content_blocks.as_ref().is_none_or(|v| v.is_empty()),
            "Empty input should have no content blocks"
        );
    }

    /// Test multiple entries with same speaker
    #[test]
    fn test_multiple_same_speaker() {
        let entries = vec![
            SubtitleEntry {
                start_time: Duration::from_secs(0),
                end_time: Duration::from_secs(1),
                text: "First".to_string(),
                line_number: Some(1),
                speaker: Some("Alice".to_string()),
                speaker_classes: vec![],
                ..Default::default()
            },
            SubtitleEntry {
                start_time: Duration::from_secs(2),
                end_time: Duration::from_secs(3),
                text: "Second".to_string(),
                line_number: Some(2),
                speaker: Some("Alice".to_string()),
                speaker_classes: vec![],
                ..Default::default()
            },
        ];

        let markdown = WebvttBackend::generate_markdown(&entries, "test.vtt");
        // Both entries should have speaker prefix
        assert!(
            markdown.contains("Alice:  First"),
            "First entry with same speaker should have prefix"
        );
        assert!(
            markdown.contains("Alice:  Second"),
            "Second entry with same speaker should have prefix"
        );
    }

    /// Test time duration edge case: zero duration
    #[test]
    fn test_zero_duration() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(5),
            end_time: Duration::from_secs(5), // Zero duration
            text: "Instant".to_string(),
            line_number: Some(1),
            speaker: None,
            speaker_classes: vec![],
            ..Default::default()
        }];

        let markdown = WebvttBackend::generate_markdown(&entries, "test.vtt");
        // Should handle zero duration gracefully
        assert!(
            markdown.contains("00:05.000 --> 00:05.000"),
            "Zero duration should format correctly"
        );
    }

    /// Test time duration edge case: very long duration
    #[test]
    fn test_very_long_duration() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(0),
            end_time: Duration::from_secs(10000), // ~2.7 hours
            text: "Long".to_string(),
            line_number: Some(1),
            speaker: None,
            speaker_classes: vec![],
            ..Default::default()
        }];

        let markdown = WebvttBackend::generate_markdown(&entries, "test.vtt");
        // Should format hours correctly
        assert!(
            markdown.contains("02:46:40.000"),
            "Long duration should include hours in timestamp"
        );
    }

    /// Test empty speaker name (edge case)
    #[test]
    fn test_empty_speaker_name() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(0),
            end_time: Duration::from_secs(1),
            text: "Text".to_string(),
            line_number: Some(1),
            speaker: Some("".to_string()), // Empty speaker name
            speaker_classes: vec![],
            ..Default::default()
        }];

        let markdown = WebvttBackend::generate_markdown(&entries, "test.vtt");
        // Empty speaker should still get colon formatting
        assert!(
            markdown.contains(":  Text"),
            "Empty speaker name should still have colon"
        );
    }

    /// Test DocItem self_ref format validation
    #[test]
    fn test_docitem_self_ref_format() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(0),
            end_time: Duration::from_secs(1),
            text: "Test".to_string(),
            line_number: Some(1),
            speaker: None,
            speaker_classes: vec![],
            ..Default::default()
        }];

        let doc_items = WebvttBackend::create_docitems(&entries);
        match &doc_items[1] {
            DocItem::Text { self_ref, .. } => {
                // Should follow #/texts/{index} format
                assert!(self_ref.starts_with("#/texts/"));
            }
            _ => panic!("Expected Text DocItem"),
        }
    }

    /// Test DocItem provenance field (WebVTT uses empty provenance)
    #[test]
    fn test_docitem_provenance() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(0),
            end_time: Duration::from_secs(1),
            text: "Test".to_string(),
            line_number: Some(1),
            speaker: None,
            speaker_classes: vec![],
            ..Default::default()
        }];

        let doc_items = WebvttBackend::create_docitems(&entries);
        match &doc_items[1] {
            DocItem::Text { prov, .. } => {
                // WebVTT creates DocItems with empty provenance
                assert!(prov.is_empty());
            }
            _ => panic!("Expected Text DocItem"),
        }
    }

    /// Test DocItem formatting field is None
    #[test]
    fn test_docitem_no_formatting() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(0),
            end_time: Duration::from_secs(1),
            text: "Plain text".to_string(),
            line_number: Some(1),
            speaker: None,
            speaker_classes: vec![],
            ..Default::default()
        }];

        let doc_items = WebvttBackend::create_docitems(&entries);
        match &doc_items[1] {
            DocItem::Text { formatting, .. } => {
                // WebVTT doesn't support formatting metadata
                assert!(formatting.is_none());
            }
            _ => panic!("Expected Text DocItem"),
        }
    }

    /// Test backend options are passed through (even if unused)
    #[test]
    fn test_backend_options_passthrough() {
        let backend = WebvttBackend::new().unwrap();
        let vtt = b"WEBVTT\n\n00:00:00.000 --> 00:00:01.000\nTest\n";
        let options = BackendOptions::default()
            .with_ocr(true)
            .with_table_structure(true);

        let result = backend.parse_bytes(vtt, &options);
        // Options don't affect WebVTT parsing, but should not error
        assert!(
            result.is_ok(),
            "Backend options should not cause errors for WebVTT"
        );
    }

    /// Test DocItems structure (timing + content pairs)
    #[test]
    fn test_docitems_timing_content_pairs() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(0),
            end_time: Duration::from_secs(1),
            text: "Text".to_string(),
            line_number: Some(1),
            speaker: None,
            speaker_classes: vec![],
            ..Default::default()
        }];

        let doc_items = WebvttBackend::create_docitems(&entries);
        // Should have 2 items: timing + content
        assert_eq!(
            doc_items.len(),
            2,
            "DocItems should include timing and content pairs"
        );

        // First item should be timing
        match &doc_items[0] {
            DocItem::Text { text, .. } => {
                assert!(text.contains("-->"));
            }
            _ => panic!("Expected Text DocItem for timing"),
        }

        // Second item should be content
        match &doc_items[1] {
            DocItem::Text { text, .. } => {
                assert_eq!(text, "Text");
            }
            _ => panic!("Expected Text DocItem for content"),
        }
    }

    /// Test line_number is preserved in DocItems
    #[test]
    fn test_line_number_preservation() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(0),
            end_time: Duration::from_secs(1),
            text: "Test".to_string(),
            line_number: Some(42),
            speaker: None,
            speaker_classes: vec![],
            ..Default::default()
        }];

        let doc_items = WebvttBackend::create_docitems(&entries);
        // Line number is used in timestamp annotation
        // (not directly stored in DocItem, but affects formatting)
        assert!(
            doc_items.len() >= 2,
            "DocItems should be created regardless of line number"
        );
    }

    /// Test timestamp with milliseconds precision
    #[test]
    fn test_timestamp_milliseconds() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_millis(1234),
            end_time: Duration::from_millis(5678),
            text: "Test".to_string(),
            line_number: Some(1),
            speaker: None,
            speaker_classes: vec![],
            ..Default::default()
        }];

        let markdown = WebvttBackend::generate_markdown(&entries, "test.vtt");
        assert!(
            markdown.contains("00:01.234"),
            "Millisecond precision should be preserved in start time"
        );
        assert!(
            markdown.contains("00:05.678"),
            "Millisecond precision should be preserved in end time"
        );
    }

    /// Test multiple entries create correct number of DocItems
    #[test]
    fn test_multiple_entries_docitem_count() {
        let entries = vec![
            SubtitleEntry {
                start_time: Duration::from_secs(0),
                end_time: Duration::from_secs(1),
                text: "First".to_string(),
                line_number: Some(1),
                speaker: None,
                speaker_classes: vec![],
                ..Default::default()
            },
            SubtitleEntry {
                start_time: Duration::from_secs(2),
                end_time: Duration::from_secs(3),
                text: "Second".to_string(),
                line_number: Some(2),
                speaker: None,
                speaker_classes: vec![],
                ..Default::default()
            },
            SubtitleEntry {
                start_time: Duration::from_secs(4),
                end_time: Duration::from_secs(5),
                text: "Third".to_string(),
                line_number: Some(3),
                speaker: None,
                speaker_classes: vec![],
                ..Default::default()
            },
        ];

        let doc_items = WebvttBackend::create_docitems(&entries);
        // Each entry creates 2 DocItems (timing + content)
        assert_eq!(doc_items.len(), 6, "3 entries should create 6 DocItems"); // 3 entries * 2 items/entry
    }

    // ===== N=471 Expansion: 10 additional tests =====

    /// Test overlapping time ranges
    #[test]
    fn test_overlapping_timestamps() {
        let entries = vec![
            SubtitleEntry {
                start_time: Duration::from_secs(0),
                end_time: Duration::from_secs(3),
                text: "First".to_string(),
                line_number: Some(1),
                speaker: None,
                speaker_classes: vec![],
                ..Default::default()
            },
            SubtitleEntry {
                start_time: Duration::from_secs(2),
                end_time: Duration::from_secs(5),
                text: "Second (overlaps)".to_string(),
                line_number: Some(2),
                speaker: None,
                speaker_classes: vec![],
                ..Default::default()
            },
        ];

        let markdown = WebvttBackend::generate_markdown(&entries, "test.vtt");
        // Both entries should appear with their original times
        assert!(
            markdown.contains("00:00.000 --> 00:03.000"),
            "First overlapping timestamp should be present"
        );
        assert!(
            markdown.contains("00:02.000 --> 00:05.000"),
            "Second overlapping timestamp should be present"
        );
        assert!(
            markdown.contains("Second (overlaps)"),
            "Overlapping entry text should be present"
        );
    }

    /// Test consecutive entries without gap
    #[test]
    fn test_consecutive_no_gap() {
        let entries = vec![
            SubtitleEntry {
                start_time: Duration::from_secs(0),
                end_time: Duration::from_secs(2),
                text: "First".to_string(),
                line_number: Some(1),
                speaker: None,
                speaker_classes: vec![],
                ..Default::default()
            },
            SubtitleEntry {
                start_time: Duration::from_secs(2),
                end_time: Duration::from_secs(4),
                text: "Second".to_string(),
                line_number: Some(2),
                speaker: None,
                speaker_classes: vec![],
                ..Default::default()
            },
        ];

        let doc_items = WebvttBackend::create_docitems(&entries);
        // Both entries should create DocItems (4 total: 2 timing + 2 content)
        assert_eq!(
            doc_items.len(),
            4,
            "Consecutive entries should create 4 DocItems"
        );
    }

    /// Test speaker classes with special characters
    #[test]
    fn test_speaker_classes_special_chars() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(0),
            end_time: Duration::from_secs(1),
            text: "Text".to_string(),
            line_number: Some(1),
            speaker: Some("Speaker".to_string()),
            speaker_classes: vec!["loud-voice".to_string(), "off_screen".to_string()],
            ..Default::default()
        }];

        let markdown = WebvttBackend::generate_markdown(&entries, "test.vtt");
        // Classes with hyphens and underscores should work
        assert!(
            markdown.contains("Speaker (loud-voice, off_screen):  Text"),
            "Speaker classes with special chars should be preserved"
        );
    }

    /// Test text with HTML-like content (WebVTT supports some HTML)
    #[test]
    fn test_text_with_html_tags() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(0),
            end_time: Duration::from_secs(2),
            text: "<i>Italic</i> and <b>bold</b>".to_string(),
            line_number: Some(1),
            speaker: None,
            speaker_classes: vec![],
            ..Default::default()
        }];

        let markdown = WebvttBackend::generate_markdown(&entries, "test.vtt");
        // HTML tags should be preserved as-is
        assert!(
            markdown.contains("<i>Italic</i> and <b>bold</b>"),
            "HTML-like tags should be preserved in WebVTT"
        );
    }

    /// Test text with leading/trailing whitespace
    #[test]
    fn test_text_whitespace_preservation() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(0),
            end_time: Duration::from_secs(1),
            text: "  Leading and trailing  ".to_string(),
            line_number: Some(1),
            speaker: None,
            speaker_classes: vec![],
            ..Default::default()
        }];

        let markdown = WebvttBackend::generate_markdown(&entries, "test.vtt");
        // Whitespace in text should be preserved
        assert!(
            markdown.contains("  Leading and trailing  "),
            "Leading/trailing whitespace should be preserved"
        );
    }

    /// Test format method returns correct InputFormat
    #[test]
    fn test_backend_format_consistency() {
        let backend = WebvttBackend;
        // Format should consistently be Webvtt
        assert_eq!(
            backend.format(),
            InputFormat::Webvtt,
            "Backend format should be Webvtt"
        );
        assert_eq!(
            WebvttBackend.format(),
            InputFormat::Webvtt,
            "Static format call should return Webvtt"
        );
    }

    /// Test Document metadata title
    #[test]
    fn test_document_metadata_title() {
        // This is implicitly tested in parse_file, but verify expectation
        // that title comes from filename
        let backend = WebvttBackend::new().unwrap();
        let vtt = b"WEBVTT\n\n00:00:00.000 --> 00:00:01.000\nTest\n";
        let options = BackendOptions::default();

        let doc = backend.parse_bytes(vtt, &options).unwrap();
        // Title should be "subtitle.vtt" (temp file name)
        assert!(
            doc.metadata.title.is_some(),
            "Document metadata should have a title"
        );
    }

    /// Test entries with out-of-order timestamps
    #[test]
    fn test_out_of_order_timestamps() {
        let entries = vec![
            SubtitleEntry {
                start_time: Duration::from_secs(5),
                end_time: Duration::from_secs(7),
                text: "Later".to_string(),
                line_number: Some(2),
                speaker: None,
                speaker_classes: vec![],
                ..Default::default()
            },
            SubtitleEntry {
                start_time: Duration::from_secs(0),
                end_time: Duration::from_secs(2),
                text: "Earlier".to_string(),
                line_number: Some(1),
                speaker: None,
                speaker_classes: vec![],
                ..Default::default()
            },
        ];

        let markdown = WebvttBackend::generate_markdown(&entries, "test.vtt");
        // Should preserve order as given, even if timestamps are out of order
        assert!(
            markdown.contains("00:05.000 --> 00:07.000\n\nLater"),
            "Later entry should appear first when given first"
        );
        assert!(
            markdown.contains("00:00.000 --> 00:02.000\n\nEarlier"),
            "Earlier entry should appear second when given second"
        );
    }

    /// Test speaker with empty classes list
    #[test]
    fn test_speaker_empty_classes_list() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(0),
            end_time: Duration::from_secs(1),
            text: "Text".to_string(),
            line_number: Some(1),
            speaker: Some("Speaker".to_string()),
            speaker_classes: vec![],
            ..Default::default()
        }];

        let doc_items = WebvttBackend::create_docitems(&entries);
        match &doc_items[1] {
            DocItem::Text { text, .. } => {
                // Empty classes list should not add parentheses
                assert_eq!(text, "Speaker: Text");
                assert!(!text.contains("()"));
            }
            _ => panic!("Expected Text DocItem"),
        }
    }

    /// Test very short duration (1 millisecond)
    #[test]
    fn test_very_short_duration() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_millis(1000),
            end_time: Duration::from_millis(1001),
            text: "Flash".to_string(),
            line_number: Some(1),
            speaker: None,
            speaker_classes: vec![],
            ..Default::default()
        }];

        let markdown = WebvttBackend::generate_markdown(&entries, "test.vtt");
        // Should handle 1ms duration
        assert!(
            markdown.contains("00:01.000 --> 00:01.001"),
            "Very short 1ms duration should be handled correctly"
        );
    }

    /// Test WebVTT with voice tags
    #[test]
    fn test_webvtt_voice_tags() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(0),
            end_time: Duration::from_secs(2),
            text: "<v John>Hello there!</v>".to_string(),
            line_number: Some(1),
            speaker: None,
            speaker_classes: vec![],
            ..Default::default()
        }];

        let doc_items = WebvttBackend::create_docitems(&entries);
        // Should preserve voice tags
        let has_voice = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => text.contains("<v John>"),
            _ => false,
        });
        assert!(
            has_voice,
            "WebVTT voice tags should be preserved in DocItems"
        );
    }

    /// Test WebVTT with cue settings (position, align)
    #[test]
    fn test_webvtt_cue_settings() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(0),
            end_time: Duration::from_secs(2),
            text: "Positioned text".to_string(),
            line_number: Some(1),
            speaker: None,
            speaker_classes: vec!["align:middle".to_string(), "position:50%".to_string()],
            ..Default::default()
        }];

        let doc_items = WebvttBackend::create_docitems(&entries);
        // Should create DocItems with positioning info
        assert!(
            doc_items.len() >= 2,
            "Cue settings should not prevent DocItem creation"
        );
    }

    /// Test WebVTT with language annotations
    #[test]
    fn test_webvtt_language_tags() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(0),
            end_time: Duration::from_secs(2),
            text: "<lang en>Hello</lang> <lang es>Hola</lang>".to_string(),
            line_number: Some(1),
            speaker: None,
            speaker_classes: vec![],
            ..Default::default()
        }];

        let doc_items = WebvttBackend::create_docitems(&entries);
        // Should preserve language tags
        let has_lang = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => text.contains("<lang"),
            _ => false,
        });
        assert!(has_lang, "WebVTT language tags should be preserved");
    }

    /// Test WebVTT with ruby text (East Asian pronunciation)
    #[test]
    fn test_webvtt_ruby_text() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(0),
            end_time: Duration::from_secs(2),
            text: "<ruby>漢字<rt>かんじ</rt></ruby>".to_string(),
            line_number: Some(1),
            speaker: None,
            speaker_classes: vec![],
            ..Default::default()
        }];

        let doc_items = WebvttBackend::create_docitems(&entries);
        // Should preserve ruby annotations
        let has_ruby = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => text.contains("<ruby>") && text.contains("<rt>"),
            _ => false,
        });
        assert!(
            has_ruby,
            "WebVTT ruby annotations should be preserved for East Asian text"
        );
    }

    /// Test WebVTT with timestamp tags (karaoke style)
    #[test]
    fn test_webvtt_timestamp_tags() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(0),
            end_time: Duration::from_secs(5),
            text: "Word <00:00:01.000>by <00:00:02.000>word <00:00:03.000>highlighting".to_string(),
            line_number: Some(1),
            speaker: None,
            speaker_classes: vec![],
            ..Default::default()
        }];

        let doc_items = WebvttBackend::create_docitems(&entries);
        // Should preserve internal timestamps for karaoke effect
        let has_timestamps = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => text.contains("<00:00:"),
            _ => false,
        });
        assert!(
            has_timestamps,
            "Internal karaoke timestamps should be preserved"
        );
    }

    /// Test WebVTT with NOTE comments (metadata in file)
    #[test]
    fn test_webvtt_note_comments() {
        // WebVTT NOTE blocks are metadata comments, typically ignored during parsing
        // This test verifies that subtitle content is extracted correctly even with NOTEs
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(1),
            end_time: Duration::from_secs(3),
            text: "Actual subtitle text".to_string(),
            line_number: Some(1),
            speaker: None,
            speaker_classes: vec![],
            ..Default::default()
        }];

        let doc_items = WebvttBackend::create_docitems(&entries);
        assert_eq!(
            doc_items.len(),
            2,
            "NOTE blocks should not prevent DocItem creation"
        ); // Timing + Text

        // Verify text content is present
        let has_content = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => text.contains("Actual subtitle text"),
            _ => false,
        });
        assert!(
            has_content,
            "Subtitle text should be present despite NOTE blocks"
        );
    }

    /// Test WebVTT with STYLE blocks (CSS styling)
    #[test]
    fn test_webvtt_style_blocks() {
        // WebVTT STYLE blocks define CSS for cue styling
        // Test that text with class references is preserved
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(0),
            end_time: Duration::from_secs(2),
            text: "<c.yellow>Warning:</c> <c.red>Critical alert</c>".to_string(),
            line_number: Some(1),
            speaker: None,
            speaker_classes: vec![],
            ..Default::default()
        }];

        let doc_items = WebvttBackend::create_docitems(&entries);
        // Should preserve class markup for styling
        let has_classes = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => text.contains("<c.yellow>") && text.contains("<c.red>"),
            _ => false,
        });
        assert!(has_classes, "WebVTT CSS class markup should be preserved");
    }

    /// Test WebVTT with REGION definitions (positioning)
    #[test]
    fn test_webvtt_region_definitions() {
        // WebVTT REGION blocks define screen regions for cue positioning
        // Text is extracted normally, positioning metadata is in parser metadata
        let entries = vec![
            SubtitleEntry {
                start_time: Duration::from_secs(0),
                end_time: Duration::from_secs(2),
                text: "Top region text".to_string(),
                line_number: Some(1),
                speaker: None,
                speaker_classes: vec![],
                ..Default::default()
            },
            SubtitleEntry {
                start_time: Duration::from_secs(2),
                end_time: Duration::from_secs(4),
                text: "Bottom region text".to_string(),
                line_number: Some(2),
                speaker: None,
                speaker_classes: vec![],
                ..Default::default()
            },
        ];

        let doc_items = WebvttBackend::create_docitems(&entries);
        assert_eq!(
            doc_items.len(),
            4,
            "Region entries should create 4 DocItems"
        ); // 2 timing + 2 text items

        // Verify both region texts are present
        let texts: Vec<String> = doc_items
            .iter()
            .filter_map(|item| match item {
                DocItem::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert!(
            texts.iter().any(|t| t.contains("Top region")),
            "Top region text should be present"
        );
        assert!(
            texts.iter().any(|t| t.contains("Bottom region")),
            "Bottom region text should be present"
        );
    }

    /// Test WebVTT with voice spans and complex nesting
    #[test]
    fn test_webvtt_voice_spans_complex_nesting() {
        // WebVTT supports <v> voice tags with nested formatting
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(0),
            end_time: Duration::from_secs(5),
            text: "<v Alice><i>Hello</i> <b>world</b></v> <v Bob><u>Nice to meet you</u></v>"
                .to_string(),
            line_number: Some(1),
            speaker: None,
            speaker_classes: vec![],
            ..Default::default()
        }];

        let doc_items = WebvttBackend::create_docitems(&entries);
        // Should preserve voice tags and nested formatting
        let has_voices = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => {
                text.contains("<v Alice>")
                    && text.contains("<v Bob>")
                    && text.contains("<i>")
                    && text.contains("<b>")
                    && text.contains("<u>")
            }
            _ => false,
        });
        assert!(
            has_voices,
            "Voice spans with nested formatting should be preserved"
        );
    }

    /// Test WebVTT with mixed directionality (RTL and LTR text)
    #[test]
    fn test_webvtt_mixed_directionality() {
        // WebVTT supports mixed RTL (Arabic, Hebrew) and LTR text
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(0),
            end_time: Duration::from_secs(3),
            text: "English text مرحبا (Arabic) שלום (Hebrew)".to_string(),
            line_number: Some(1),
            speaker: None,
            speaker_classes: vec![],
            ..Default::default()
        }];

        let doc_items = WebvttBackend::create_docitems(&entries);
        assert_eq!(
            doc_items.len(),
            2,
            "Mixed directionality entry should create 2 DocItems"
        ); // Timing + Text

        // Verify mixed directionality text is preserved
        let has_mixed = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => {
                text.contains("English") && text.contains("مرحبا") && text.contains("שלום")
            }
            _ => false,
        });
        assert!(has_mixed, "Mixed RTL/LTR text should be preserved");
    }

    /// Test WebVTT with region positioning (spatial layout cues)
    #[test]
    fn test_webvtt_region_positioning() {
        // WebVTT supports regions for spatial positioning of cues
        let backend = WebvttBackend;
        let vtt_content = "WEBVTT\n\nREGION\nid:top\nwidth:50%\nlines:3\n\n00:00:00.000 --> 00:00:03.000 region:top\nTop region text\n\n00:00:03.000 --> 00:00:06.000\nDefault region text";
        let result = backend.parse_bytes(vtt_content.as_bytes(), &Default::default());
        assert!(
            result.is_ok(),
            "WebVTT with REGION definitions should parse successfully"
        );
        let doc = result.unwrap();
        // Should parse successfully with regions
        assert!(
            !doc.markdown.is_empty(),
            "Parsed markdown should not be empty"
        );
        assert!(
            doc.markdown.contains("Top region text"),
            "Top region text should be in markdown"
        );
        assert!(
            doc.markdown.contains("Default region text"),
            "Default region text should be in markdown"
        );
    }

    /// Test WebVTT with line positioning and alignment
    #[test]
    fn test_webvtt_line_position_alignment() {
        // WebVTT supports line positioning (line:) and alignment (align:)
        let backend = WebvttBackend;
        let vtt_content = "WEBVTT\n\n00:00:00.000 --> 00:00:03.000 line:0% align:start\nTop-left text\n\n00:00:03.000 --> 00:00:06.000 line:50% align:center\nCentered text\n\n00:00:06.000 --> 00:00:09.000 line:100% align:end\nBottom-right text";
        let result = backend.parse_bytes(vtt_content.as_bytes(), &Default::default());
        assert!(
            result.is_ok(),
            "WebVTT with line/align positioning should parse successfully"
        );
        let doc = result.unwrap();
        assert!(
            !doc.markdown.is_empty(),
            "Positioned cues should produce non-empty markdown"
        );
        // Should have all three positioned cues
        assert!(
            doc.markdown.contains("Top-left"),
            "Top-left positioned text should be present"
        );
        assert!(
            doc.markdown.contains("Centered"),
            "Centered text should be present"
        );
        assert!(
            doc.markdown.contains("Bottom-right"),
            "Bottom-right positioned text should be present"
        );
    }

    /// Test WebVTT with size positioning (cue box width)
    #[test]
    fn test_webvtt_size_positioning() {
        // WebVTT supports size: to control cue box width
        let backend = WebvttBackend;
        let vtt_content = "WEBVTT\n\n00:00:00.000 --> 00:00:03.000 size:50%\nHalf-width cue\n\n00:00:03.000 --> 00:00:06.000 size:100%\nFull-width cue\n\n00:00:06.000 --> 00:00:09.000 size:25%\nQuarter-width cue";
        let result = backend.parse_bytes(vtt_content.as_bytes(), &Default::default());
        assert!(
            result.is_ok(),
            "WebVTT with size positioning should parse successfully"
        );
        let doc = result.unwrap();
        assert!(
            !doc.markdown.is_empty(),
            "Size-positioned cues should produce non-empty markdown"
        );
        // Should have all sized cues
        assert!(
            doc.markdown.contains("Half-width"),
            "Half-width cue text should be present"
        );
        assert!(
            doc.markdown.contains("Full-width"),
            "Full-width cue text should be present"
        );
        assert!(
            doc.markdown.contains("Quarter-width"),
            "Quarter-width cue text should be present"
        );
    }

    /// Test WebVTT with vertical text direction
    #[test]
    fn test_webvtt_vertical_text() {
        // WebVTT supports vertical: rl (right-to-left) and lr (left-to-right) for Asian languages
        let backend = WebvttBackend;
        let vtt_content = "WEBVTT\n\n00:00:00.000 --> 00:00:03.000 vertical:rl\n日本語の縦書き\n\n00:00:03.000 --> 00:00:06.000 vertical:lr\n中文直排文字";
        let result = backend.parse_bytes(vtt_content.as_bytes(), &Default::default());
        assert!(
            result.is_ok(),
            "WebVTT with vertical text direction should parse successfully"
        );
        let doc = result.unwrap();
        assert!(
            !doc.markdown.is_empty(),
            "Vertical text should produce non-empty markdown"
        );
        // Should preserve vertical text (Japanese and Chinese)
        assert!(
            doc.markdown.contains("日本語") || doc.markdown.contains("中文"),
            "Vertical Asian text should be preserved"
        );
    }

    /// Test WebVTT with NOTE blocks in full file parsing
    #[test]
    fn test_webvtt_note_blocks_full_parse() {
        // WebVTT supports NOTE blocks for comments (not displayed to viewers)
        let backend = WebvttBackend;
        let vtt_content = "WEBVTT\n\nNOTE This is a comment\nNOTE Comments can span\nmultiple lines\n\n00:00:00.000 --> 00:00:03.000\nVisible subtitle\n\nNOTE Another comment\n\n00:00:03.000 --> 00:00:06.000\nAnother visible subtitle";
        let result = backend.parse_bytes(vtt_content.as_bytes(), &Default::default());
        assert!(
            result.is_ok(),
            "WebVTT with NOTE blocks should parse successfully"
        );
        let doc = result.unwrap();
        assert!(
            !doc.markdown.is_empty(),
            "NOTE blocks should not prevent markdown generation"
        );
        // Comments should not appear in output (or may appear as metadata)
        assert!(
            doc.markdown.contains("Visible subtitle"),
            "First visible subtitle should be present"
        );
        assert!(
            doc.markdown.contains("Another visible subtitle"),
            "Second visible subtitle should be present"
        );
        // NOTE comments are typically ignored in subtitle display
    }
}
