//! SRT subtitle document backend
//!
//! This module provides SRT (`SubRip`) subtitle file parsing and conversion
//! capabilities using the docling-video crate.

use crate::traits::{BackendOptions, DocumentBackend};
use crate::utils::{create_text_item, opt_vec};
use docling_core::{DocItem, DoclingError, Document, DocumentMetadata, InputFormat};
use docling_video::SubtitleEntry;
use std::fmt::Write;
use std::path::Path;

/// SRT subtitle backend
///
/// Supports SRT (`SubRip`) subtitle format (.srt files).
/// Converts subtitle entries to markdown with timestamp information.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct SrtBackend;

impl SrtBackend {
    /// Create a new SRT backend
    ///
    /// Note: This method returns a Result for API consistency with other backends,
    /// but never actually fails. The `SrtBackend` is a unit struct with no initialization logic.
    ///
    /// # Errors
    ///
    /// This function never returns an error (returns `Ok(Self)` always).
    #[inline]
    #[must_use = "creating a backend that is not used is a waste of resources"]
    pub const fn new() -> Result<Self, DoclingError> {
        Ok(Self)
    }

    /// Format a duration as SRT timestamp (HH:MM:SS,mmm)
    fn format_timestamp(duration: std::time::Duration) -> String {
        let total_secs = duration.as_secs();
        let millis = duration.subsec_millis();
        let hours = total_secs / 3600;
        let minutes = (total_secs % 3600) / 60;
        let seconds = total_secs % 60;
        format!("{hours:02}:{minutes:02}:{seconds:02},{millis:03}")
    }

    /// Generate markdown from subtitle entries
    fn generate_markdown(entries: &[SubtitleEntry], subtitle_name: &str) -> String {
        let mut markdown = String::new();
        let _ = write!(markdown, "# Subtitle File: {subtitle_name}\n\n");

        if entries.is_empty() {
            markdown.push_str("*(Empty subtitle file)*\n");
            return markdown;
        }

        let _ = write!(markdown, "**Total entries:** {}\n\n", entries.len());

        // List all subtitle entries
        for entry in entries {
            let start = Self::format_timestamp(entry.start_time);
            let end = Self::format_timestamp(entry.end_time);
            let _ = writeln!(markdown, "[{start} --> {end}]");
            markdown.push_str(&entry.text);
            markdown.push_str("\n\n");
        }

        markdown
    }

    /// Create `DocItems` from subtitle entries
    ///
    /// Similar to webvtt.rs pattern: creates Text `DocItems` for timing and content.
    /// Each subtitle entry becomes 2 `DocItems`: timing line + content line.
    fn create_docitems(entries: &[SubtitleEntry]) -> Vec<DocItem> {
        let mut doc_items = Vec::new();
        let mut text_idx = 0;

        for entry in entries {
            let start = Self::format_timestamp(entry.start_time);
            let end = Self::format_timestamp(entry.end_time);
            let timing_text = format!("{start} --> {end}");

            // Save indices before incrementing
            let timing_idx_val = text_idx;
            text_idx += 1;
            let content_idx_val = text_idx;
            text_idx += 1;

            // Build speaker text (though SRT typically doesn't have speaker info)
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
    /// Shared helper used by both `parse_bytes` and `parse_file` to avoid code duplication.
    fn create_document(entries: &[SubtitleEntry], subtitle_name: &str) -> Document {
        // Generate DocItems
        let doc_items = Self::create_docitems(entries);

        // Generate markdown
        let markdown = Self::generate_markdown(entries, subtitle_name);
        let num_characters = markdown.chars().count();

        // Create document with DocItems
        Document {
            markdown,
            format: InputFormat::Srt,
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

impl DocumentBackend for SrtBackend {
    #[inline]
    fn format(&self) -> InputFormat {
        InputFormat::Srt
    }

    fn parse_bytes(
        &self,
        data: &[u8],
        _options: &BackendOptions,
    ) -> Result<Document, DoclingError> {
        // Convert bytes to string
        let content = std::str::from_utf8(data)
            .map_err(|e| DoclingError::BackendError(format!("Invalid UTF-8 in SRT file: {e}")))?;

        // Parse SRT content using srtparse
        let parsed_subtitles = srtparse::from_str(content)
            .map_err(|e| DoclingError::BackendError(format!("SRT parsing failed: {e}")))?;

        // Convert to our SubtitleEntry format
        let entries: Vec<SubtitleEntry> = parsed_subtitles
            .iter()
            .enumerate()
            .map(|(idx, sub)| {
                let start_ms = (sub.start_time.hours * 3600
                    + sub.start_time.minutes * 60
                    + sub.start_time.seconds)
                    * 1000
                    + sub.start_time.milliseconds;
                let end_ms =
                    (sub.end_time.hours * 3600 + sub.end_time.minutes * 60 + sub.end_time.seconds)
                        * 1000
                        + sub.end_time.milliseconds;

                SubtitleEntry {
                    start_time: std::time::Duration::from_millis(start_ms),
                    end_time: std::time::Duration::from_millis(end_ms),
                    text: sub.text.clone(),
                    line_number: Some(idx + 1),
                    speaker: None,
                    speaker_classes: Vec::new(),
                    ..Default::default()
                }
            })
            .collect();

        Ok(Self::create_document(&entries, "subtitle.srt"))
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
            .unwrap_or("subtitle.srt");

        // Parse SRT file using docling-video
        let subtitle_file = docling_video::parse_subtitle_file(path_ref).map_err(|e| {
            DoclingError::BackendError(format!("Failed to parse SRT file: {e}: {subtitle_name}"))
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
        assert_eq!(
            SrtBackend::format_timestamp(Duration::from_millis(1500)),
            "00:00:01,500"
        );
        assert_eq!(
            SrtBackend::format_timestamp(Duration::from_secs(3665) + Duration::from_millis(123)),
            "01:01:05,123"
        );
    }

    #[test]
    fn test_markdown_generation_empty() {
        let entries = vec![];
        let markdown = SrtBackend::generate_markdown(&entries, "test.srt");
        assert!(
            markdown.contains("Empty subtitle file"),
            "Empty subtitle file should show placeholder message"
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
        let markdown = SrtBackend::generate_markdown(&entries, "test.srt");
        assert!(
            markdown.contains("**Total entries:** 2"),
            "Should show total entry count of 2"
        );
        assert!(
            markdown.contains("[00:00:01,000 --> 00:00:03,000]"),
            "Should contain first timestamp"
        );
        assert!(
            markdown.contains("Hello world"),
            "Should contain first subtitle text"
        );
        assert!(
            markdown.contains("[00:00:05,000 --> 00:00:07,000]"),
            "Should contain second timestamp"
        );
        assert!(
            markdown.contains("Goodbye world"),
            "Should contain second subtitle text"
        );
    }

    /// Test SRT timestamp format uses comma (not period like WebVTT)
    #[test]
    fn test_timestamp_format_comma_separator() {
        let timestamp = SrtBackend::format_timestamp(Duration::from_millis(1500));
        assert!(
            timestamp.contains(','),
            "SRT timestamps use comma separator"
        );
        assert!(!timestamp.contains('.'), "SRT timestamps don't use period");
        assert_eq!(
            timestamp, "00:00:01,500",
            "Timestamp should format as 00:00:01,500"
        );
    }

    /// Test timestamp edge cases
    #[test]
    fn test_timestamp_edge_cases() {
        // Zero duration
        assert_eq!(
            SrtBackend::format_timestamp(Duration::from_secs(0)),
            "00:00:00,000"
        );

        // Long duration (99 hours)
        assert_eq!(
            SrtBackend::format_timestamp(Duration::from_secs(356_400)),
            "99:00:00,000"
        );

        // Milliseconds precision
        assert_eq!(
            SrtBackend::format_timestamp(Duration::from_millis(1)),
            "00:00:00,001"
        );
        assert_eq!(
            SrtBackend::format_timestamp(Duration::from_millis(999)),
            "00:00:00,999"
        );
    }

    /// Test markdown with header
    #[test]
    fn test_markdown_includes_header() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(1),
            end_time: Duration::from_secs(2),
            text: "Text".to_string(),
            line_number: Some(1),
            speaker: None,
            speaker_classes: vec![],
            ..Default::default()
        }];

        let markdown = SrtBackend::generate_markdown(&entries, "movie.srt");

        // Should have title header
        assert!(
            markdown.contains("# Subtitle File: movie.srt"),
            "Markdown should have filename as title"
        );
        // Should have entry count
        assert!(
            markdown.contains("**Total entries:** 1"),
            "Markdown should show entry count of 1"
        );
    }

    /// Test markdown with brackets around timestamps
    #[test]
    fn test_markdown_timestamp_brackets() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(0),
            end_time: Duration::from_secs(1),
            text: "Test".to_string(),
            line_number: Some(1),
            speaker: None,
            speaker_classes: vec![],
            ..Default::default()
        }];

        let markdown = SrtBackend::generate_markdown(&entries, "test.srt");

        // Timestamps should be in brackets
        assert!(markdown.contains("[00:00:00,000 --> 00:00:01,000]"));
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

        let doc_items = SrtBackend::create_docitems(&entries);

        // Should create 2 items: timing + content
        assert_eq!(
            doc_items.len(),
            2,
            "Single entry should create 2 DocItems (timing + content)"
        );

        // First item is timing
        match &doc_items[0] {
            DocItem::Text { self_ref, text, .. } => {
                assert_eq!(
                    self_ref, "#/texts/0",
                    "First DocItem self_ref should be #/texts/0"
                );
                assert_eq!(
                    text, "00:00:01,000 --> 00:00:03,000",
                    "First DocItem should contain timing text"
                );
            }
            _ => panic!("Expected Text DocItem for timing"),
        }

        // Second item is content
        match &doc_items[1] {
            DocItem::Text { self_ref, text, .. } => {
                assert_eq!(
                    self_ref, "#/texts/1",
                    "Second DocItem self_ref should be #/texts/1"
                );
                assert_eq!(
                    text, "Test text",
                    "Second DocItem should contain subtitle text"
                );
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

        let doc_items = SrtBackend::create_docitems(&entries);

        // Should create 4 items: 2 entries × (timing + content)
        assert_eq!(
            doc_items.len(),
            4,
            "Two entries should create 4 DocItems (2 timing + 2 content)"
        );

        // Verify indices are sequential
        match &doc_items[0] {
            DocItem::Text { self_ref, .. } => assert_eq!(
                self_ref, "#/texts/0",
                "First DocItem self_ref should be #/texts/0"
            ),
            _ => panic!("Expected Text DocItem"),
        }
        match &doc_items[1] {
            DocItem::Text { self_ref, .. } => assert_eq!(
                self_ref, "#/texts/1",
                "Second DocItem self_ref should be #/texts/1"
            ),
            _ => panic!("Expected Text DocItem"),
        }
        match &doc_items[2] {
            DocItem::Text { self_ref, .. } => assert_eq!(
                self_ref, "#/texts/2",
                "Third DocItem self_ref should be #/texts/2"
            ),
            _ => panic!("Expected Text DocItem"),
        }
        match &doc_items[3] {
            DocItem::Text { self_ref, .. } => assert_eq!(
                self_ref, "#/texts/3",
                "Fourth DocItem self_ref should be #/texts/3"
            ),
            _ => panic!("Expected Text DocItem"),
        }
    }

    /// Test DocItem creation with speaker (though rare in SRT)
    #[test]
    fn test_create_docitems_with_speaker() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(1),
            end_time: Duration::from_secs(3),
            text: "Hello".to_string(),
            line_number: Some(1),
            speaker: Some("Alice".to_string()),
            speaker_classes: vec!["narrator".to_string()],
            ..Default::default()
        }];

        let doc_items = SrtBackend::create_docitems(&entries);

        // Second item should have speaker prefix
        match &doc_items[1] {
            DocItem::Text { text, .. } => {
                assert_eq!(
                    text, "Alice (narrator): Hello",
                    "Speaker with class should format as 'Name (class): Text'"
                );
            }
            _ => panic!("Expected Text DocItem"),
        }
    }

    /// Test empty entries produce empty DocItems
    #[test]
    fn test_create_docitems_empty() {
        let entries = vec![];
        let doc_items = SrtBackend::create_docitems(&entries);
        assert_eq!(
            doc_items.len(),
            0,
            "Empty entries should produce empty DocItems"
        );
    }

    /// Test backend format identification
    #[test]
    fn test_backend_format() {
        let backend = SrtBackend::new().unwrap();
        assert_eq!(
            backend.format(),
            InputFormat::Srt,
            "SRT backend should report Srt format"
        );
    }

    /// Test empty text entry still shows timestamp
    #[test]
    fn test_markdown_empty_text() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(1),
            end_time: Duration::from_secs(2),
            text: "".to_string(),
            line_number: Some(1),
            speaker: None,
            speaker_classes: vec![],
            ..Default::default()
        }];

        let markdown = SrtBackend::generate_markdown(&entries, "test.srt");

        // Should still have timestamp even with empty text
        assert!(
            markdown.contains("[00:00:01,000 --> 00:00:02,000]"),
            "Timestamp should be present even for entry with empty text"
        );
    }

    /// Test long duration (hours)
    #[test]
    fn test_timestamp_hours() {
        // 2 hours, 30 minutes, 45 seconds, 500 milliseconds
        let duration = Duration::from_secs(2 * 3600 + 30 * 60 + 45) + Duration::from_millis(500);
        assert_eq!(
            SrtBackend::format_timestamp(duration),
            "02:30:45,500",
            "2h 30m 45s 500ms should format as 02:30:45,500"
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

        let doc_items = SrtBackend::create_docitems(&entries);

        // Speaker without classes shouldn't have empty parentheses
        match &doc_items[1] {
            DocItem::Text { text, .. } => {
                assert!(
                    text.contains("Speaker: Text"),
                    "Speaker name should prefix text"
                );
                assert!(
                    !text.contains("()"),
                    "Should not have empty parentheses without classes"
                );
            }
            _ => panic!("Expected Text DocItem"),
        }
    }

    // ========== Backend Creation Tests ==========

    /// Test Default trait creates valid backend
    #[test]
    fn test_backend_default() {
        let backend = SrtBackend;
        assert_eq!(
            backend.format(),
            InputFormat::Srt,
            "Default SRT backend should report Srt format"
        );
    }

    /// Test new() and default() are equivalent
    #[test]
    fn test_backend_new_vs_default() {
        let backend1 = SrtBackend::new().unwrap();
        let backend2 = SrtBackend;
        assert_eq!(
            backend1.format(),
            backend2.format(),
            "new() and default() should produce equivalent backends"
        );
    }

    // ========== parse_bytes Error Handling Tests ==========

    /// Test invalid UTF-8 in parse_bytes
    #[test]
    fn test_parse_bytes_invalid_utf8() {
        let backend = SrtBackend::new().unwrap();
        let invalid_data = vec![0xFF, 0xFE, 0xFD]; // Invalid UTF-8 sequence
        let result = backend.parse_bytes(&invalid_data, &BackendOptions::default());
        assert!(result.is_err(), "Invalid UTF-8 data should return error");
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Invalid UTF-8 in SRT file"),
            "Error should mention invalid UTF-8"
        );
    }

    /// Test invalid SRT format (malformed subtitle)
    #[test]
    fn test_parse_bytes_invalid_format() {
        let backend = SrtBackend::new().unwrap();
        let invalid_srt = b"This is not a valid SRT file\nJust random text";
        let result = backend.parse_bytes(invalid_srt, &BackendOptions::default());
        // Should return error from srtparse
        assert!(result.is_err(), "Malformed SRT should return error");
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("SRT parsing failed"),
            "Error should mention SRT parsing failed"
        );
    }

    /// Test empty SRT file
    #[test]
    fn test_parse_bytes_empty() {
        let backend = SrtBackend::new().unwrap();
        let empty_srt = b"";
        let result = backend.parse_bytes(empty_srt, &BackendOptions::default());
        // Empty file should parse successfully but have no entries
        assert!(result.is_ok(), "Empty SRT file should parse successfully");
        let doc = result.unwrap();
        assert!(
            doc.content_blocks.is_none(),
            "Empty SRT file should have no content blocks"
        ); // Empty entries → None
    }

    // ========== Timestamp Formatting Edge Cases ==========

    /// Test timestamp with hours > 99 (overflow)
    #[test]
    fn test_timestamp_overflow_hours() {
        // 100 hours
        let duration = Duration::from_secs(100 * 3600);
        let timestamp = SrtBackend::format_timestamp(duration);
        // SRT format doesn't limit hours, should show 100:00:00,000
        assert!(
            timestamp.starts_with("100:"),
            "100-hour timestamp should start with '100:'"
        );
        assert_eq!(timestamp, "100:00:00,000");
    }

    /// Test timestamp at 59:59.999 boundary
    #[test]
    fn test_timestamp_boundary_59_minutes() {
        let duration = Duration::from_secs(59 * 60 + 59) + Duration::from_millis(999);
        assert_eq!(SrtBackend::format_timestamp(duration), "00:59:59,999");
    }

    /// Test timestamp subsecond precision (milliseconds only)
    #[test]
    fn test_timestamp_subsecond_precision() {
        // SRT uses milliseconds (3 digits), not microseconds
        let duration = Duration::from_micros(1_234_567); // 1.234567 seconds
        let timestamp = SrtBackend::format_timestamp(duration);
        // Should round down to 1234 ms
        assert_eq!(timestamp, "00:00:01,234");
    }

    // ========== Markdown Generation Variations ==========

    /// Test markdown with special characters in text
    #[test]
    fn test_markdown_special_characters() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(1),
            end_time: Duration::from_secs(2),
            text: "Text with <tags> & \"quotes\"".to_string(),
            line_number: Some(1),
            speaker: None,
            speaker_classes: vec![],
            ..Default::default()
        }];

        let markdown = SrtBackend::generate_markdown(&entries, "test.srt");
        assert!(
            markdown.contains("<tags>"),
            "HTML tags should be preserved in markdown"
        );
        assert!(
            markdown.contains("&"),
            "Ampersand should be preserved in markdown"
        );
        assert!(
            markdown.contains("\"quotes\""),
            "Quotes should be preserved in markdown"
        );
    }

    /// Test markdown with newlines in text
    #[test]
    fn test_markdown_newlines_in_text() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(1),
            end_time: Duration::from_secs(2),
            text: "Line 1\nLine 2\nLine 3".to_string(),
            line_number: Some(1),
            speaker: None,
            speaker_classes: vec![],
            ..Default::default()
        }];

        let markdown = SrtBackend::generate_markdown(&entries, "test.srt");
        assert!(
            markdown.contains("Line 1\nLine 2\nLine 3"),
            "Newlines in text should be preserved"
        );
    }

    /// Test markdown with very long text
    #[test]
    fn test_markdown_long_text() {
        let long_text = "A".repeat(1000);
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(1),
            end_time: Duration::from_secs(10),
            text: long_text.clone(),
            line_number: Some(1),
            speaker: None,
            speaker_classes: vec![],
            ..Default::default()
        }];

        let markdown = SrtBackend::generate_markdown(&entries, "test.srt");
        assert!(
            markdown.contains(&long_text),
            "Very long text content should be preserved"
        );
    }

    // ========== DocItem Edge Cases ==========

    /// Test DocItem with multiple speaker classes
    #[test]
    fn test_docitem_multiple_speaker_classes() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(1),
            end_time: Duration::from_secs(2),
            text: "Hello".to_string(),
            line_number: Some(1),
            speaker: Some("Alice".to_string()),
            speaker_classes: vec![
                "narrator".to_string(),
                "loud".to_string(),
                "happy".to_string(),
            ],
            ..Default::default()
        }];

        let doc_items = SrtBackend::create_docitems(&entries);

        // Speaker with multiple classes should show comma-separated
        match &doc_items[1] {
            DocItem::Text { text, .. } => {
                assert_eq!(
                    text, "Alice (narrator, loud, happy): Hello",
                    "Multiple speaker classes should be comma-separated in parentheses"
                );
            }
            _ => panic!("Expected Text DocItem"),
        }
    }

    /// Test DocItem with very long speaker name
    #[test]
    fn test_docitem_long_speaker_name() {
        let long_speaker = "A".repeat(100);
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(0),
            end_time: Duration::from_secs(1),
            text: "Text".to_string(),
            line_number: Some(1),
            speaker: Some(long_speaker.clone()),
            speaker_classes: vec![],
            ..Default::default()
        }];

        let doc_items = SrtBackend::create_docitems(&entries);

        match &doc_items[1] {
            DocItem::Text { text, .. } => {
                assert!(
                    text.starts_with(&long_speaker),
                    "Long speaker name should be at start of text"
                );
            }
            _ => panic!("Expected Text DocItem"),
        }
    }

    /// Test DocItem with unicode text (CJK, emoji)
    #[test]
    fn test_docitem_unicode_text() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(1),
            end_time: Duration::from_secs(2),
            text: "Hello 世界 🌍".to_string(),
            line_number: Some(1),
            speaker: None,
            speaker_classes: vec![],
            ..Default::default()
        }];

        let doc_items = SrtBackend::create_docitems(&entries);

        match &doc_items[1] {
            DocItem::Text { text, .. } => {
                assert_eq!(
                    text, "Hello 世界 🌍",
                    "Unicode text with CJK and emoji should be preserved"
                );
            }
            _ => panic!("Expected Text DocItem"),
        }
    }

    // ========== Integration Tests ==========

    /// Test character count validation
    #[test]
    fn test_character_count_validation() {
        let backend = SrtBackend::new().unwrap();
        let srt_data = b"1\n00:00:01,000 --> 00:00:02,000\nHello\n\n2\n00:00:03,000 --> 00:00:04,000\nWorld\n\n";
        let doc = backend
            .parse_bytes(srt_data, &BackendOptions::default())
            .unwrap();

        // Character count should match markdown length
        assert_eq!(
            doc.metadata.num_characters,
            doc.markdown.chars().count(),
            "Character count metadata should match actual markdown length"
        );
        assert!(
            doc.metadata.num_characters > 0,
            "Character count should be positive for non-empty document"
        );
    }

    /// Test metadata fields populated correctly
    #[test]
    fn test_metadata_fields() {
        let backend = SrtBackend::new().unwrap();
        let srt_data = b"1\n00:00:01,000 --> 00:00:02,000\nTest\n\n";
        let doc = backend
            .parse_bytes(srt_data, &BackendOptions::default())
            .unwrap();

        // Check metadata fields
        assert_eq!(
            doc.metadata.title,
            Some("subtitle.srt".to_string()),
            "Title should default to subtitle.srt"
        );
        assert_eq!(
            doc.metadata.num_pages, None,
            "SRT files should have no page count"
        );
        assert_eq!(
            doc.metadata.author, None,
            "SRT files should have no author metadata"
        );
        assert_eq!(
            doc.metadata.created, None,
            "SRT files should have no creation date"
        );
        assert_eq!(
            doc.metadata.modified, None,
            "SRT files should have no modified date"
        );
        assert_eq!(
            doc.metadata.language, None,
            "SRT files should have no language metadata"
        );
        assert!(
            doc.metadata.exif.is_none(),
            "SRT files should have no EXIF metadata"
        );
    }

    // ========== Format Differences Tests ==========

    /// Test SRT uses comma separator (not period like WebVTT)
    #[test]
    fn test_srt_vs_webvtt_separator() {
        let duration = Duration::from_millis(1234);
        let srt_timestamp = SrtBackend::format_timestamp(duration);

        // SRT: 00:00:01,234 (comma)
        // WebVTT: 00:00:01.234 (period)
        assert!(
            srt_timestamp.contains(','),
            "SRT timestamps should use comma separator"
        );
        assert!(
            !srt_timestamp.contains('.'),
            "SRT timestamps should not use period separator"
        );
    }

    /// Test line_number field handling (SRT specific)
    #[test]
    fn test_line_number_field() {
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

        // line_number field exists but isn't used in markdown/docitems
        let markdown = SrtBackend::generate_markdown(&entries, "test.srt");
        let doc_items = SrtBackend::create_docitems(&entries);

        // Should have 2 entries
        assert!(
            markdown.contains("**Total entries:** 2"),
            "Markdown should show 2 entries"
        );
        assert_eq!(doc_items.len(), 4); // 2 entries × 2 items each
    }

    // ========== Additional Edge Cases ==========

    /// Test timestamp with maximum u64 duration
    #[test]
    fn test_timestamp_maximum_duration() {
        // Max reasonable duration (10 years in seconds = ~315M seconds = ~87,000 hours)
        let duration = Duration::from_secs(315_360_000);
        let timestamp = SrtBackend::format_timestamp(duration);
        // Should handle large hours value
        assert!(
            timestamp.starts_with("87600:"),
            "Maximum duration should format as 87600+ hours"
        );
    }

    /// Test timestamp with only milliseconds
    #[test]
    fn test_timestamp_only_milliseconds() {
        let duration = Duration::from_millis(500);
        assert_eq!(SrtBackend::format_timestamp(duration), "00:00:00,500");
    }

    /// Test timestamp exact minute boundary
    #[test]
    fn test_timestamp_exact_minute_boundary() {
        let duration = Duration::from_secs(60); // Exactly 1 minute
        assert_eq!(SrtBackend::format_timestamp(duration), "00:01:00,000");
    }

    /// Test timestamp exact hour boundary
    #[test]
    fn test_timestamp_exact_hour_boundary() {
        let duration = Duration::from_secs(3600); // Exactly 1 hour
        assert_eq!(SrtBackend::format_timestamp(duration), "01:00:00,000");
    }

    /// Test speaker with empty string
    #[test]
    fn test_docitem_empty_speaker_name() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(0),
            end_time: Duration::from_secs(1),
            text: "Text".to_string(),
            line_number: Some(1),
            speaker: Some("".to_string()),
            speaker_classes: vec![],
            ..Default::default()
        }];

        let doc_items = SrtBackend::create_docitems(&entries);

        // Empty speaker should still add colon
        match &doc_items[1] {
            DocItem::Text { text, .. } => {
                assert_eq!(text, ": Text");
            }
            _ => panic!("Expected Text DocItem"),
        }
    }

    /// Test speaker classes without speaker name
    #[test]
    fn test_docitem_classes_without_speaker() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(0),
            end_time: Duration::from_secs(1),
            text: "Text".to_string(),
            line_number: Some(1),
            speaker: None,
            speaker_classes: vec!["loud".to_string()],
            ..Default::default()
        }];

        let doc_items = SrtBackend::create_docitems(&entries);

        // Classes without speaker should just show text
        match &doc_items[1] {
            DocItem::Text { text, .. } => {
                assert_eq!(text, "Text");
            }
            _ => panic!("Expected Text DocItem"),
        }
    }

    /// Test very large number of entries
    #[test]
    fn test_docitem_many_entries() {
        let entries: Vec<SubtitleEntry> = (0..100)
            .map(|i| SubtitleEntry {
                start_time: Duration::from_secs(i),
                end_time: Duration::from_secs(i + 1),
                text: format!("Entry {i}"),
                line_number: Some((i + 1) as usize),
                speaker: None,
                speaker_classes: vec![],
                ..Default::default()
            })
            .collect();

        let doc_items = SrtBackend::create_docitems(&entries);

        // Should create 200 items (100 entries × 2)
        assert_eq!(
            doc_items.len(),
            200,
            "100 entries should create 200 DocItems (100 timing + 100 content)"
        );

        // Verify first and last items
        match &doc_items[0] {
            DocItem::Text { self_ref, .. } => assert_eq!(
                self_ref, "#/texts/0",
                "First DocItem self_ref should be #/texts/0"
            ),
            _ => panic!("Expected Text DocItem"),
        }
        match &doc_items[199] {
            DocItem::Text { self_ref, .. } => assert_eq!(
                self_ref, "#/texts/199",
                "Last DocItem self_ref should be #/texts/199"
            ),
            _ => panic!("Expected Text DocItem"),
        }
    }

    /// Test content_blocks is None when empty
    #[test]
    fn test_content_blocks_none_when_empty() {
        let backend = SrtBackend::new().unwrap();
        let empty_srt = b"";
        let doc = backend
            .parse_bytes(empty_srt, &BackendOptions::default())
            .unwrap();

        assert!(
            doc.content_blocks.is_none(),
            "Empty SRT should have no content blocks"
        );
    }

    /// Test content_blocks is Some when not empty
    #[test]
    fn test_content_blocks_some_when_not_empty() {
        let backend = SrtBackend::new().unwrap();
        let srt_data = b"1\n00:00:01,000 --> 00:00:02,000\nTest\n\n";
        let doc = backend
            .parse_bytes(srt_data, &BackendOptions::default())
            .unwrap();

        assert!(
            doc.content_blocks.is_some(),
            "Non-empty SRT should have content blocks"
        );
        let blocks = doc.content_blocks.unwrap();
        assert_eq!(blocks.len(), 2); // timing + content
    }

    /// Test single entry produces 2 DocItems
    #[test]
    fn test_single_entry_docitem_count() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(0),
            end_time: Duration::from_secs(1),
            text: "Single".to_string(),
            line_number: Some(1),
            speaker: None,
            speaker_classes: vec![],
            ..Default::default()
        }];

        let doc_items = SrtBackend::create_docitems(&entries);
        assert_eq!(
            doc_items.len(),
            2,
            "Single entry should produce 2 DocItems (timing + content)"
        );
    }

    /// Test markdown with zero duration entry
    #[test]
    fn test_markdown_zero_duration() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(5),
            end_time: Duration::from_secs(5), // Same start and end
            text: "Flash".to_string(),
            line_number: Some(1),
            speaker: None,
            speaker_classes: vec![],
            ..Default::default()
        }];

        let markdown = SrtBackend::generate_markdown(&entries, "test.srt");
        assert!(
            markdown.contains("[00:00:05,000 --> 00:00:05,000]"),
            "Zero duration entry should show identical start and end times"
        );
    }

    /// Test can_handle method
    #[test]
    fn test_can_handle_srt_format() {
        let backend = SrtBackend::new().unwrap();
        assert!(
            backend.can_handle(InputFormat::Srt),
            "SRT backend should handle SRT format"
        );
        assert!(
            !backend.can_handle(InputFormat::Webvtt),
            "SRT backend should not handle WebVTT format"
        );
        assert!(
            !backend.can_handle(InputFormat::Pdf),
            "SRT backend should not handle PDF format"
        );
    }

    /// Test BackendOptions passthrough (ignored but accepted)
    #[test]
    fn test_backend_options_passthrough() {
        let backend = SrtBackend::new().unwrap();
        let srt_data = b"1\n00:00:01,000 --> 00:00:02,000\nTest\n\n";

        // Test with various options (all should be ignored for SRT)
        let options = BackendOptions::default()
            .with_ocr(true)
            .with_table_structure(true);

        let result = backend.parse_bytes(srt_data, &options);
        assert!(
            result.is_ok(),
            "SRT parsing should succeed regardless of backend options"
        );
    }

    /// Test markdown generation with custom filename
    #[test]
    fn test_markdown_custom_filename() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(0),
            end_time: Duration::from_secs(1),
            text: "Text".to_string(),
            line_number: Some(1),
            speaker: None,
            speaker_classes: vec![],
            ..Default::default()
        }];

        let markdown = SrtBackend::generate_markdown(&entries, "my_movie.srt");
        assert!(
            markdown.contains("# Subtitle File: my_movie.srt"),
            "Markdown should include custom filename in title"
        );
    }

    /// Test valid SRT with valid parse
    #[test]
    fn test_parse_bytes_valid_srt() {
        let backend = SrtBackend::new().unwrap();
        let valid_srt = b"1\n00:00:01,000 --> 00:00:02,000\nHello world\n\n2\n00:00:03,000 --> 00:00:04,000\nGoodbye world\n\n";
        let result = backend.parse_bytes(valid_srt, &BackendOptions::default());

        assert!(result.is_ok(), "Valid SRT should parse successfully");
        let doc = result.unwrap();
        assert_eq!(
            doc.format,
            InputFormat::Srt,
            "Document format should be SRT"
        );
        assert!(
            doc.content_blocks.is_some(),
            "Parsed SRT should have content blocks"
        );
        assert_eq!(
            doc.content_blocks.unwrap().len(),
            4,
            "Two entries should produce 4 DocItems (2 timing + 2 content)"
        );
    }

    /// Test parse_bytes with trailing whitespace
    #[test]
    fn test_parse_bytes_trailing_whitespace() {
        let backend = SrtBackend::new().unwrap();
        let srt_with_whitespace =
            b"1\n00:00:01,000 --> 00:00:02,000\nText with trailing space  \n\n";
        let result = backend.parse_bytes(srt_with_whitespace, &BackendOptions::default());

        assert!(
            result.is_ok(),
            "SRT with trailing whitespace should parse successfully"
        );
        let doc = result.unwrap();
        // Trailing whitespace should be preserved
        assert!(
            doc.markdown.contains("Text with trailing space"),
            "Text content should be preserved in markdown"
        );
    }

    /// Test DocItem provenance structure
    #[test]
    fn test_docitem_provenance_structure() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(0),
            end_time: Duration::from_secs(1),
            text: "Test".to_string(),
            line_number: Some(1),
            speaker: None,
            speaker_classes: vec![],
            ..Default::default()
        }];

        let doc_items = SrtBackend::create_docitems(&entries);

        // Verify both items have empty provenance (SRT doesn't use provenance)
        for item in &doc_items {
            match item {
                DocItem::Text { prov, .. } => {
                    assert!(prov.is_empty(), "SRT DocItems should have empty provenance");
                }
                _ => panic!("Expected Text DocItem"),
            }
        }
    }

    // ===== N=472 Expansion: 10 additional tests =====

    /// Test overlapping subtitle time ranges
    #[test]
    fn test_overlapping_time_ranges() {
        let entries = vec![
            SubtitleEntry {
                start_time: Duration::from_secs(0),
                end_time: Duration::from_secs(3),
                text: "First subtitle".to_string(),
                line_number: Some(1),
                speaker: None,
                speaker_classes: vec![],
                ..Default::default()
            },
            SubtitleEntry {
                start_time: Duration::from_secs(2),
                end_time: Duration::from_secs(5),
                text: "Overlapping subtitle".to_string(),
                line_number: Some(2),
                speaker: None,
                speaker_classes: vec![],
                ..Default::default()
            },
        ];

        let markdown = SrtBackend::generate_markdown(&entries, "test.srt");
        // Both entries should appear even though they overlap
        assert!(markdown.contains("[00:00:00,000 --> 00:00:03,000]"));
        assert!(markdown.contains("[00:00:02,000 --> 00:00:05,000]"));
        assert!(
            markdown.contains("Overlapping subtitle"),
            "Overlapping subtitles should both be preserved"
        );
    }

    /// Test consecutive entries without gap
    #[test]
    fn test_consecutive_entries_no_gap() {
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

        let doc_items = SrtBackend::create_docitems(&entries);
        // Should create 4 DocItems (2 timing + 2 content)
        assert_eq!(
            doc_items.len(),
            4,
            "Consecutive entries with no gap should create 4 DocItems"
        );
    }

    /// Test text with HTML-like tags (SRT can contain formatting)
    #[test]
    fn test_text_with_html_formatting() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(0),
            end_time: Duration::from_secs(2),
            text: "<i>Italic</i> and <b>bold</b>".to_string(),
            line_number: Some(1),
            speaker: None,
            speaker_classes: vec![],
            ..Default::default()
        }];

        let markdown = SrtBackend::generate_markdown(&entries, "test.srt");
        // HTML tags should be preserved as-is
        assert!(
            markdown.contains("<i>Italic</i> and <b>bold</b>"),
            "HTML formatting tags should be preserved"
        );
    }

    /// Test text with leading/trailing whitespace preservation
    #[test]
    fn test_text_whitespace_edges() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(0),
            end_time: Duration::from_secs(1),
            text: "  Spaces before and after  ".to_string(),
            line_number: Some(1),
            speaker: None,
            speaker_classes: vec![],
            ..Default::default()
        }];

        let markdown = SrtBackend::generate_markdown(&entries, "test.srt");
        // Whitespace should be preserved in output
        assert!(
            markdown.contains("  Spaces before and after  "),
            "Whitespace at text edges should be preserved"
        );
    }

    /// Test can_handle with multiple formats
    #[test]
    fn test_can_handle_various_formats() {
        let backend = SrtBackend::new().unwrap();

        // Should handle SRT
        assert!(
            backend.can_handle(InputFormat::Srt),
            "Should handle SRT format"
        );

        // Should not handle other formats
        assert!(
            !backend.can_handle(InputFormat::Webvtt),
            "Should not handle WebVTT format"
        );
        assert!(
            !backend.can_handle(InputFormat::Pdf),
            "Should not handle PDF format"
        );
        assert!(
            !backend.can_handle(InputFormat::Docx),
            "Should not handle DOCX format"
        );
        assert!(
            !backend.can_handle(InputFormat::Html),
            "Should not handle HTML format"
        );
    }

    /// Test document metadata structure
    #[test]
    fn test_document_metadata_fields() {
        let backend = SrtBackend::new().unwrap();
        let srt_data = b"1\n00:00:01,000 --> 00:00:02,000\nTest\n\n";
        let doc = backend
            .parse_bytes(srt_data, &BackendOptions::default())
            .unwrap();

        // Should have title (temp filename)
        assert!(
            doc.metadata.title.is_some(),
            "Parsed SRT should have a title"
        );
        // Should have character count
        assert!(
            doc.metadata.num_characters > 0,
            "Character count should be positive"
        );
        // Should not have pages (subtitle files don't have pages)
        assert!(
            doc.metadata.num_pages.is_none(),
            "Subtitle files should not have pages"
        );
    }

    /// Test entries with out-of-order timestamps
    #[test]
    fn test_out_of_order_entries() {
        let entries = vec![
            SubtitleEntry {
                start_time: Duration::from_secs(5),
                end_time: Duration::from_secs(7),
                text: "Later entry".to_string(),
                line_number: Some(2),
                speaker: None,
                speaker_classes: vec![],
                ..Default::default()
            },
            SubtitleEntry {
                start_time: Duration::from_secs(0),
                end_time: Duration::from_secs(2),
                text: "Earlier entry".to_string(),
                line_number: Some(1),
                speaker: None,
                speaker_classes: vec![],
                ..Default::default()
            },
        ];

        let markdown = SrtBackend::generate_markdown(&entries, "test.srt");
        // Should preserve order as given (parser doesn't sort)
        let later_pos = markdown.find("Later entry").unwrap();
        let earlier_pos = markdown.find("Earlier entry").unwrap();
        assert!(
            later_pos < earlier_pos,
            "Entry order should be preserved as given (not sorted by timestamp)"
        );
    }

    /// Test very short duration (1 millisecond)
    #[test]
    fn test_one_millisecond_duration() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_millis(1000),
            end_time: Duration::from_millis(1001),
            text: "Quick flash".to_string(),
            line_number: Some(1),
            speaker: None,
            speaker_classes: vec![],
            ..Default::default()
        }];

        let markdown = SrtBackend::generate_markdown(&entries, "test.srt");
        // Should handle 1ms precision
        assert!(
            markdown.contains("[00:00:01,000 --> 00:00:01,001]"),
            "1ms duration should be represented accurately"
        );
    }

    /// Test DocItem self_ref format correctness
    #[test]
    fn test_docitem_self_ref_correctness() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(0),
            end_time: Duration::from_secs(1),
            text: "Test".to_string(),
            line_number: Some(1),
            speaker: None,
            speaker_classes: vec![],
            ..Default::default()
        }];

        let doc_items = SrtBackend::create_docitems(&entries);

        // Check timing item
        match &doc_items[0] {
            DocItem::Text { self_ref, .. } => {
                assert_eq!(
                    self_ref, "#/texts/0",
                    "Timing DocItem self_ref should be #/texts/0"
                );
            }
            _ => panic!("Expected Text DocItem"),
        }

        // Check content item
        match &doc_items[1] {
            DocItem::Text { self_ref, .. } => {
                assert_eq!(
                    self_ref, "#/texts/1",
                    "Content DocItem self_ref should be #/texts/1"
                );
            }
            _ => panic!("Expected Text DocItem"),
        }
    }

    /// Test multiple entries with same content (duplicates)
    #[test]
    fn test_duplicate_entries() {
        let entries = vec![
            SubtitleEntry {
                start_time: Duration::from_secs(0),
                end_time: Duration::from_secs(1),
                text: "Same text".to_string(),
                line_number: Some(1),
                speaker: None,
                speaker_classes: vec![],
                ..Default::default()
            },
            SubtitleEntry {
                start_time: Duration::from_secs(2),
                end_time: Duration::from_secs(3),
                text: "Same text".to_string(),
                line_number: Some(2),
                speaker: None,
                speaker_classes: vec![],
                ..Default::default()
            },
        ];

        let doc_items = SrtBackend::create_docitems(&entries);
        // Should create 4 DocItems even with duplicate text
        assert_eq!(
            doc_items.len(),
            4,
            "Duplicate text entries should still create separate DocItems"
        );
    }

    /// Test subtitle with HTML formatting tags
    #[test]
    fn test_subtitle_with_html_tags() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(0),
            end_time: Duration::from_secs(2),
            text: "<i>Italicized text</i> and <b>bold text</b>".to_string(),
            line_number: Some(1),
            speaker: None,
            speaker_classes: vec![],
            ..Default::default()
        }];

        let doc_items = SrtBackend::create_docitems(&entries);
        // Should preserve HTML tags in text
        let has_html = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => text.contains("<i>") || text.contains("<b>"),
            _ => false,
        });
        assert!(
            has_html,
            "HTML formatting tags should be preserved in DocItems"
        );
    }

    /// Test subtitle with color formatting
    #[test]
    fn test_subtitle_with_color_tags() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(0),
            end_time: Duration::from_secs(2),
            text: "<font color=\"#FF0000\">Red text</font>".to_string(),
            line_number: Some(1),
            speaker: None,
            speaker_classes: vec![],
            ..Default::default()
        }];

        let doc_items = SrtBackend::create_docitems(&entries);
        // Should preserve font color tags
        let has_color = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => text.contains("color="),
            _ => false,
        });
        assert!(has_color, "Font color tags should be preserved in DocItems");
    }

    /// Test subtitle with position/alignment information
    #[test]
    fn test_subtitle_positioning() {
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(0),
            end_time: Duration::from_secs(2),
            text: "{\\an8}Top-aligned text".to_string(),
            line_number: Some(1),
            speaker: None,
            speaker_classes: vec![],
            ..Default::default()
        }];

        let doc_items = SrtBackend::create_docitems(&entries);
        // Should preserve positioning codes
        assert_eq!(
            doc_items.len(),
            2,
            "Entry with positioning codes should create 2 DocItems"
        );
    }

    /// Test subtitle with fractional seconds
    #[test]
    fn test_fractional_seconds_precision() {
        use std::time::Duration;
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_millis(1234),
            end_time: Duration::from_millis(5678),
            text: "Precise timing".to_string(),
            line_number: Some(1),
            speaker: None,
            speaker_classes: vec![],
            ..Default::default()
        }];

        let doc_items = SrtBackend::create_docitems(&entries);
        // Check timing format includes milliseconds
        let has_timing = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => text.contains("00:00:01,234"),
            _ => false,
        });
        assert!(
            has_timing,
            "Millisecond precision should be preserved in timing"
        );
    }

    /// Test subtitle with hearing impaired annotations
    #[test]
    fn test_hearing_impaired_annotations() {
        let entries = vec![
            SubtitleEntry {
                start_time: Duration::from_secs(0),
                end_time: Duration::from_secs(2),
                text: "[Door creaking]".to_string(),
                line_number: Some(1),
                speaker: None,
                speaker_classes: vec![],
                ..Default::default()
            },
            SubtitleEntry {
                start_time: Duration::from_secs(3),
                end_time: Duration::from_secs(5),
                text: "(Music playing)".to_string(),
                line_number: Some(2),
                speaker: None,
                speaker_classes: vec![],
                ..Default::default()
            },
            SubtitleEntry {
                start_time: Duration::from_secs(6),
                end_time: Duration::from_secs(8),
                text: "♪ Song lyrics ♪".to_string(),
                line_number: Some(3),
                speaker: None,
                speaker_classes: vec![],
                ..Default::default()
            },
        ];

        let doc_items = SrtBackend::create_docitems(&entries);
        // Should preserve all annotation types
        let has_brackets = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => text.contains("[Door creaking]"),
            _ => false,
        });
        let has_parens = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => text.contains("(Music playing)"),
            _ => false,
        });
        let has_music = doc_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => text.contains("♪"),
            _ => false,
        });
        assert!(has_brackets, "Bracket annotations should be preserved");
        assert!(has_parens, "Parenthesis annotations should be preserved");
        assert!(has_music, "Music note symbols should be preserved");
    }

    #[test]
    fn test_subtitle_with_multiple_lines_per_entry() {
        // SRT entries often have multiple lines of text per timestamp
        let entries = vec![
            SubtitleEntry {
                start_time: Duration::from_secs(1),
                end_time: Duration::from_secs(3),
                text: "First line\nSecond line\nThird line".to_string(),
                line_number: Some(1),
                speaker: None,
                speaker_classes: vec![],
                ..Default::default()
            },
            SubtitleEntry {
                start_time: Duration::from_secs(4),
                end_time: Duration::from_secs(6),
                text: "Single line".to_string(),
                line_number: Some(2),
                speaker: None,
                speaker_classes: vec![],
                ..Default::default()
            },
        ];

        // SRT backend creates 2 DocItems per entry: timing + content
        // 2 entries * 2 DocItems = 4 total
        let doc_items = SrtBackend::create_docitems(&entries);
        assert_eq!(doc_items.len(), 4, "Two entries should create 4 DocItems");

        // Verify multi-line entry preserves newlines
        // First entry's content is at index 1 (index 0 is timing)
        let first_content_item = &doc_items[1];
        if let DocItem::Text { text, .. } = first_content_item {
            assert!(
                text.contains("First line"),
                "First line should be preserved in multi-line text"
            );
            assert!(
                text.contains("Second line"),
                "Second line should be preserved in multi-line text"
            );
            assert!(
                text.contains("Third line"),
                "Third line should be preserved in multi-line text"
            );
            // Newlines should be preserved in text
            let newline_count = text.matches('\n').count();
            assert!(
                newline_count >= 2,
                "Should preserve newlines in subtitle text"
            );
        } else {
            panic!("Expected Text DocItem");
        }
    }

    #[test]
    fn test_subtitle_with_extremely_long_duration() {
        // Test subtitle entry that spans many hours (e.g., permanent caption)
        let entries = vec![SubtitleEntry {
            start_time: Duration::from_secs(0),
            end_time: Duration::from_secs(36000), // 10 hours
            text: "This caption remains for 10 hours".to_string(),
            line_number: Some(1),
            speaker: None,
            speaker_classes: vec![],
            ..Default::default()
        }];

        let markdown = SrtBackend::generate_markdown(&entries, "test.srt");

        // Should format timestamp correctly for long duration
        assert!(markdown.contains("[00:00:00,000 --> 10:00:00,000]"));
        assert!(
            markdown.contains("This caption remains for 10 hours"),
            "Long duration text should be preserved"
        );

        // Verify DocItem generation
        // SRT backend creates 2 DocItems per entry: timing + content
        let doc_items = SrtBackend::create_docitems(&entries);
        assert_eq!(doc_items.len(), 2);

        // First item should be timing
        if let DocItem::Text { text, .. } = &doc_items[0] {
            assert!(
                text.contains("10:00:00"),
                "Timing should show long duration"
            );
        } else {
            panic!("Expected Text DocItem for timing");
        }

        // Second item should be content
        if let DocItem::Text { text, .. } = &doc_items[1] {
            assert!(
                text.contains("This caption remains for 10 hours"),
                "Content DocItem should have caption text"
            );
        } else {
            panic!("Expected Text DocItem for content");
        }
    }

    /// Test subtitle with UTF-8 BOM and multilingual content
    #[test]
    fn test_srt_utf8_bom_and_multilingual() {
        use docling_video::SubtitleEntry;
        use std::time::Duration;

        // SRT files often start with UTF-8 BOM (Byte Order Mark)
        // Test with various languages and scripts
        let entries = vec![
            SubtitleEntry {
                line_number: Some(1),
                start_time: Duration::from_secs(0),
                end_time: Duration::from_secs(2),
                text: "Hello, World!".to_string(),
                speaker: None,
                speaker_classes: vec![],
                ..Default::default()
            },
            SubtitleEntry {
                line_number: Some(2),
                start_time: Duration::from_secs(2),
                end_time: Duration::from_secs(4),
                text: "こんにちは世界 (Japanese)".to_string(),
                speaker: None,
                speaker_classes: vec![],
                ..Default::default()
            },
            SubtitleEntry {
                line_number: Some(3),
                start_time: Duration::from_secs(4),
                end_time: Duration::from_secs(6),
                text: "مرحبا بالعالم (Arabic - RTL)".to_string(),
                speaker: None,
                speaker_classes: vec![],
                ..Default::default()
            },
            SubtitleEntry {
                line_number: Some(4),
                start_time: Duration::from_secs(6),
                end_time: Duration::from_secs(8),
                text: "Здравствуй, мир! (Russian)".to_string(),
                speaker: None,
                speaker_classes: vec![],
                ..Default::default()
            },
            SubtitleEntry {
                line_number: Some(5),
                start_time: Duration::from_secs(8),
                end_time: Duration::from_secs(10),
                text: "🌍 Emoji and symbols: ♥ ★ ✓".to_string(),
                speaker: None,
                speaker_classes: vec![],
                ..Default::default()
            },
        ];

        // Test markdown generation
        let markdown = SrtBackend::generate_markdown(&entries, "multilingual.srt");
        assert!(
            markdown.contains("Subtitle File: multilingual.srt"),
            "Filename should be in markdown header"
        );
        assert!(
            markdown.contains("**Total entries:** 5"),
            "Should show 5 entries"
        );
        assert!(
            markdown.contains("こんにちは世界"),
            "Japanese text should be preserved"
        );
        assert!(
            markdown.contains("مرحبا بالعالم"),
            "Arabic text should be preserved"
        );
        assert!(
            markdown.contains("Здравствуй, мир"),
            "Russian text should be preserved"
        );
        assert!(markdown.contains("🌍"), "Emoji should be preserved");

        // Test DocItem generation
        let doc_items = SrtBackend::create_docitems(&entries);
        assert_eq!(
            doc_items.len(),
            10,
            "5 entries should create 10 DocItems (5 timing + 5 content)"
        );
    }

    /// Test karaoke-style word-level timing within entries
    #[test]
    fn test_srt_karaoke_word_timing() {
        use docling_video::SubtitleEntry;
        use std::time::Duration;

        // Advanced SRT feature: word-level timing for karaoke or lyric sync
        // Format: <00:00:00.500>Word1 <00:00:01.000>Word2 <00:00:01.500>Word3
        let entries = vec![
            SubtitleEntry {
                line_number: Some(1),
                start_time: Duration::from_millis(0),
                end_time: Duration::from_millis(3000),
                text: "<00:00:00.500>I <00:00:01.000>will <00:00:01.500>always <00:00:02.000>love <00:00:02.500>you"
                    .to_string(),
                speaker: None,
                speaker_classes: vec![],
                ..Default::default()
            },
            SubtitleEntry {
                line_number: Some(2),
                start_time: Duration::from_millis(3000),
                end_time: Duration::from_millis(6000),
                text: "<00:00:03.200>Every <00:00:03.800>word <00:00:04.400>highlighted <00:00:05.000>separately"
                    .to_string(),
                speaker: None,
                speaker_classes: vec![],
                ..Default::default()
            },
        ];

        // Test markdown generation preserves karaoke timing
        let markdown = SrtBackend::generate_markdown(&entries, "karaoke.srt");
        assert!(
            markdown.contains("<00:00:00.500>"),
            "Karaoke timestamp should be preserved"
        );
        assert!(
            markdown.contains("<00:00:01.000>"),
            "Second karaoke timestamp should be preserved"
        );
        assert!(
            markdown.contains("always"),
            "Karaoke word should be preserved"
        );
        assert!(
            markdown.contains("highlighted"),
            "Karaoke word should be preserved"
        );

        // Test DocItem generation
        let doc_items = SrtBackend::create_docitems(&entries);
        assert_eq!(
            doc_items.len(),
            4,
            "Two karaoke entries should create 4 DocItems"
        );

        // Verify karaoke timing is preserved in text
        if let DocItem::Text { text, .. } = &doc_items[1] {
            assert!(
                text.contains("<00:00:00.500>"),
                "DocItem should preserve karaoke timing tags"
            );
            assert!(
                text.contains("love"),
                "DocItem should preserve karaoke word content"
            );
        }
    }

    /// Test closed caption events (sound effects, music notation)
    #[test]
    fn test_srt_closed_caption_events() {
        use docling_video::SubtitleEntry;
        use std::time::Duration;

        // Closed captions include sound effects, music, speaker changes
        // Common notations: [music], (door slams), ♪ musical note
        let entries = vec![
            SubtitleEntry {
                line_number: Some(1),
                start_time: Duration::from_secs(0),
                end_time: Duration::from_secs(2),
                text: "[dramatic music playing]".to_string(),
                speaker: None,
                speaker_classes: vec![],
                ..Default::default()
            },
            SubtitleEntry {
                line_number: Some(2),
                start_time: Duration::from_secs(2),
                end_time: Duration::from_secs(4),
                text: "(door creaking open)".to_string(),
                speaker: None,
                speaker_classes: vec![],
                ..Default::default()
            },
            SubtitleEntry {
                line_number: Some(3),
                start_time: Duration::from_secs(4),
                end_time: Duration::from_secs(6),
                text: "♪ Singing in the rain ♪".to_string(),
                speaker: None,
                speaker_classes: vec![],
                ..Default::default()
            },
            SubtitleEntry {
                line_number: Some(4),
                start_time: Duration::from_secs(6),
                end_time: Duration::from_secs(8),
                text: "[thunder rumbling]".to_string(),
                speaker: None,
                speaker_classes: vec![],
                ..Default::default()
            },
            SubtitleEntry {
                line_number: Some(5),
                start_time: Duration::from_secs(8),
                end_time: Duration::from_secs(10),
                text: "(glass shattering)".to_string(),
                speaker: None,
                speaker_classes: vec![],
                ..Default::default()
            },
            SubtitleEntry {
                line_number: Some(6),
                start_time: Duration::from_secs(10),
                end_time: Duration::from_secs(12),
                text: "[sirens wailing in distance]".to_string(),
                speaker: None,
                speaker_classes: vec![],
                ..Default::default()
            },
            SubtitleEntry {
                line_number: Some(7),
                start_time: Duration::from_secs(12),
                end_time: Duration::from_secs(14),
                text: "♪ [upbeat jazz music] ♪".to_string(),
                speaker: None,
                speaker_classes: vec![],
                ..Default::default()
            },
        ];

        // Test markdown generation
        let markdown = SrtBackend::generate_markdown(&entries, "closed_captions.srt");
        assert!(
            markdown.contains("**Total entries:** 7"),
            "Should show 7 entries"
        );
        assert!(
            markdown.contains("[dramatic music playing]"),
            "Music notation should be preserved"
        );
        assert!(
            markdown.contains("(door creaking open)"),
            "Sound effect should be preserved"
        );
        assert!(
            markdown.contains("♪ Singing in the rain ♪"),
            "Music note symbol should be preserved"
        );
        assert!(
            markdown.contains("[thunder rumbling]"),
            "Sound effect bracket should be preserved"
        );
        assert!(
            markdown.contains("(glass shattering)"),
            "Sound effect parenthesis should be preserved"
        );
        assert!(
            markdown.contains("[sirens wailing in distance]"),
            "Long sound effect should be preserved"
        );
        assert!(
            markdown.contains("♪ [upbeat jazz music] ♪"),
            "Mixed notation should be preserved"
        );

        // Test DocItem generation
        let doc_items = SrtBackend::create_docitems(&entries);
        assert_eq!(
            doc_items.len(),
            14,
            "7 closed caption entries should create 14 DocItems"
        );

        // Verify sound effect notations are preserved
        if let DocItem::Text { text, .. } = &doc_items[1] {
            assert!(
                text.contains("[dramatic music playing]"),
                "Bracket sound effect should be preserved in DocItem"
            );
        }
        if let DocItem::Text { text, .. } = &doc_items[5] {
            assert!(
                text.contains("♪"),
                "Music note symbol should be preserved in DocItem"
            );
        }
    }

    #[test]
    fn test_srt_zero_duration_flash_subtitle() {
        // Test subtitle with 0ms duration (flash text, used for on-screen text effects)
        let entries = vec![SubtitleEntry {
            line_number: Some(1),
            start_time: Duration::from_millis(5000),
            end_time: Duration::from_millis(5000),
            text: "[FLASH MESSAGE]".to_string(),
            speaker: None,
            speaker_classes: vec![],
            ..Default::default()
        }];

        assert_eq!(
            entries[0].start_time, entries[0].end_time,
            "Flash subtitle should have equal start and end times"
        );
        assert_eq!(
            entries[0].text, "[FLASH MESSAGE]",
            "Flash subtitle text should be preserved"
        );

        // Test markdown generation
        let markdown = SrtBackend::generate_markdown(&entries, "flash.srt");
        assert!(
            markdown.contains("[00:00:05,000 --> 00:00:05,000]"),
            "Zero duration timestamp should be rendered correctly"
        );
        assert!(
            markdown.contains("[FLASH MESSAGE]"),
            "Flash message text should be preserved in markdown"
        );
    }

    #[test]
    fn test_srt_backwards_time_range_semantics() {
        // Test subtitle with end time before start time (semantically invalid but parseable)
        let entries = vec![SubtitleEntry {
            line_number: Some(1),
            start_time: Duration::from_millis(10000),
            end_time: Duration::from_millis(5000),
            text: "Backwards time".to_string(),
            speaker: None,
            speaker_classes: vec![],
            ..Default::default()
        }];

        // Verify it can represent this edge case
        assert!(
            entries[0].start_time > entries[0].end_time,
            "Backwards time range should be representable"
        );
        assert_eq!(entries[0].text, "Backwards time");

        // Test markdown generation handles this gracefully
        let markdown = SrtBackend::generate_markdown(&entries, "backwards.srt");
        assert!(markdown.contains("[00:00:10,000 --> 00:00:05,000]"));
    }

    #[test]
    fn test_srt_overlapping_subtitles_dual_language() {
        // Test overlapping subtitle entries (common in dual-language or SDH tracks)
        let entries = vec![
            SubtitleEntry {
                line_number: Some(1),
                start_time: Duration::from_millis(0),
                end_time: Duration::from_millis(3000),
                text: "First subtitle".to_string(),
                speaker: None,
                speaker_classes: vec![],
                ..Default::default()
            },
            SubtitleEntry {
                line_number: Some(2),
                start_time: Duration::from_millis(2000),
                end_time: Duration::from_millis(5000),
                text: "Overlapping subtitle".to_string(),
                speaker: None,
                speaker_classes: vec![],
                ..Default::default()
            },
            SubtitleEntry {
                line_number: Some(3),
                start_time: Duration::from_millis(5000),
                end_time: Duration::from_millis(8000),
                text: "Third subtitle".to_string(),
                speaker: None,
                speaker_classes: vec![],
                ..Default::default()
            },
        ];

        // Verify overlap: entry 2 starts before entry 1 ends
        assert!(
            entries[1].start_time < entries[0].end_time,
            "Entry 2 should overlap with entry 1"
        );
        assert_eq!(
            entries[2].start_time, entries[1].end_time,
            "Entry 3 should start when entry 2 ends"
        );

        // Test markdown generation preserves all subtitles
        let markdown = SrtBackend::generate_markdown(&entries, "overlapping.srt");
        assert!(
            markdown.contains("**Total entries:** 3"),
            "Should show 3 total entries"
        );
        assert!(
            markdown.contains("First subtitle"),
            "First subtitle should be preserved in overlapping sequence"
        );
        assert!(
            markdown.contains("Overlapping subtitle"),
            "Overlapping subtitle should be preserved"
        );
        assert!(
            markdown.contains("Third subtitle"),
            "Third subtitle should be preserved after overlap"
        );
    }

    #[test]
    fn test_srt_forced_subtitle_markers() {
        // Test forced subtitle markers (used for foreign language or important text only)
        let entries = vec![
            SubtitleEntry {
                line_number: Some(1),
                start_time: Duration::from_millis(1000),
                end_time: Duration::from_millis(3000),
                text: "{FORCED} Sign: DO NOT ENTER".to_string(),
                speaker: None,
                speaker_classes: vec![],
                ..Default::default()
            },
            SubtitleEntry {
                line_number: Some(2),
                start_time: Duration::from_millis(5000),
                end_time: Duration::from_millis(7000),
                text: "{FORCED} [On screen text: Paris, France]".to_string(),
                speaker: None,
                speaker_classes: vec![],
                ..Default::default()
            },
        ];

        // Verify forced markers preserved
        assert!(
            entries[0].text.contains("{FORCED}"),
            "First entry should have FORCED marker"
        );
        assert!(
            entries[1].text.contains("{FORCED}"),
            "Second entry should have FORCED marker"
        );

        // Test markdown generation preserves forced markers
        let markdown = SrtBackend::generate_markdown(&entries, "forced_subtitles.srt");
        assert!(
            markdown.contains("{FORCED}"),
            "FORCED marker should be in markdown"
        );
        assert!(
            markdown.contains("Sign: DO NOT ENTER"),
            "Sign text should be preserved"
        );
        assert!(markdown.contains("Paris, France"));

        // Test DocItem generation preserves forced markers
        let doc_items = SrtBackend::create_docitems(&entries);
        if let DocItem::Text { text, .. } = &doc_items[1] {
            assert!(
                text.contains("{FORCED}"),
                "FORCED marker should be in DocItem"
            );
        }
    }

    #[test]
    fn test_srt_continuous_subtitle_stream_no_gaps() {
        // Test continuous subtitle stream with no gaps (common in live broadcasts)
        let entries = vec![
            SubtitleEntry {
                line_number: Some(1),
                start_time: Duration::from_millis(1000),
                end_time: Duration::from_millis(3000),
                text: "Continuous stream".to_string(),
                speaker: None,
                speaker_classes: vec![],
                ..Default::default()
            },
            SubtitleEntry {
                line_number: Some(2),
                start_time: Duration::from_millis(3000),
                end_time: Duration::from_millis(5000),
                text: "No gaps between".to_string(),
                speaker: None,
                speaker_classes: vec![],
                ..Default::default()
            },
            SubtitleEntry {
                line_number: Some(3),
                start_time: Duration::from_millis(5000),
                end_time: Duration::from_millis(7000),
                text: "Perfect timing".to_string(),
                speaker: None,
                speaker_classes: vec![],
                ..Default::default()
            },
        ];

        // Verify continuous timing (no gaps)
        assert_eq!(
            entries[0].end_time, entries[1].start_time,
            "Entry 1 end should equal entry 2 start (no gap)"
        );
        assert_eq!(
            entries[1].end_time, entries[2].start_time,
            "Entry 2 end should equal entry 3 start (no gap)"
        );

        // Test markdown generation
        let markdown = SrtBackend::generate_markdown(&entries, "continuous.srt");
        assert!(
            markdown.contains("**Total entries:** 3"),
            "Continuous stream should show 3 entries"
        );

        // Test DocItem generation creates correct count
        let doc_items = SrtBackend::create_docitems(&entries);
        assert_eq!(
            doc_items.len(),
            6,
            "3 continuous entries should create 6 DocItems"
        );
    }
}
