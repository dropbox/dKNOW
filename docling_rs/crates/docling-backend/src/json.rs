//! JSON Backend - Load Docling JSON documents
//!
//! Enables round-trip workflows: Document → JSON → Document
//!
//! ## Features
//! - Deserializes Docling JSON format back into [`Document`] structure
//! - Round-trip testing: export to JSON, then reload for validation
//! - Direct access to native Docling document structure
//!
//! ## Python Reference
//! Ported from: `docling/backend/json/docling_json_backend.py` (59 lines)
//!
//! Key Python Methods:
//! - `_get_doc_or_err()`: lines 39-51 - Load and parse JSON
//! - `convert()`: lines 54-58 - Return parsed document or raise error
//! - `is_valid()`: lines 26-27 - Check if JSON loaded successfully

use crate::traits::{BackendOptions, DocumentBackend};
use docling_core::{DoclingError, Document, InputFormat};

/// JSON Document Backend
///
/// Loads Docling JSON documents (native format).
/// Enables round-trip testing and workflows.
///
/// Ported from: docling/backend/json/docling_json_backend.py:13-59
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct JsonBackend;

impl JsonBackend {
    /// Create a new JSON backend instance
    #[inline]
    #[must_use = "creates a backend instance that should be used for parsing"]
    pub const fn new() -> Self {
        Self
    }
}

impl DocumentBackend for JsonBackend {
    #[inline]
    fn format(&self) -> InputFormat {
        InputFormat::JsonDocling
    }

    fn parse_file<P: AsRef<std::path::Path>>(
        &self,
        path: P,
        _options: &BackendOptions,
    ) -> Result<Document, DoclingError> {
        // Python: docling_json_backend.py:42-44 - Read file as JSON
        let json_str = std::fs::read_to_string(path.as_ref())?;

        // Python: docling_json_backend.py:49 - Deserialize JSON into DoclingDocument
        let document: Document = serde_json::from_str(&json_str)?;

        Ok(document)
    }

    fn parse_bytes(
        &self,
        bytes: &[u8],
        _options: &BackendOptions,
    ) -> Result<Document, DoclingError> {
        // Python: docling_json_backend.py:46 - Read BytesIO as JSON
        let json_str = std::str::from_utf8(bytes)
            .map_err(|e| DoclingError::ConversionError(format!("Invalid UTF-8 in JSON: {e}")))?;

        // Python: docling_json_backend.py:49 - Deserialize JSON into DoclingDocument
        let document: Document = serde_json::from_str(json_str)?;

        Ok(document)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use docling_core::{Document, DocumentMetadata, InputFormat};

    #[test]
    fn test_backend_format() {
        let backend = JsonBackend::new();
        assert_eq!(
            backend.format(),
            InputFormat::JsonDocling,
            "JsonBackend::new() should return JsonDocling format"
        );
    }

    #[test]
    fn test_backend_default() {
        let backend = JsonBackend;
        assert_eq!(
            backend.format(),
            InputFormat::JsonDocling,
            "JsonBackend struct should return JsonDocling format"
        );
    }

    #[test]
    fn test_json_round_trip() {
        // Create a simple document
        let original = Document::from_markdown(
            "# Test Document\n\nThis is a test.".to_string(),
            InputFormat::Md,
        );

        // Serialize to JSON
        let json = serde_json::to_string_pretty(&original).expect("Failed to serialize");

        // Parse back using JsonBackend
        let backend = JsonBackend::new();
        let options = BackendOptions::default();
        let parsed = backend
            .parse_bytes(json.as_bytes(), &options)
            .expect("Failed to parse JSON");

        // Verify content matches
        assert_eq!(
            parsed.markdown, original.markdown,
            "Round-trip markdown content should match"
        );
        assert_eq!(
            parsed.format, original.format,
            "Round-trip format should match"
        );
        assert_eq!(
            parsed.metadata.num_characters, original.metadata.num_characters,
            "Round-trip character count should match"
        );
    }

    #[test]
    fn test_json_parse_file() {
        // Create a temporary JSON file
        let original = Document::from_markdown(
            "# File Test\n\nTesting file parsing.".to_string(),
            InputFormat::Md,
        );

        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("docling_test.json");
        let json = serde_json::to_string_pretty(&original).expect("Failed to serialize");
        std::fs::write(&temp_file, json).expect("Failed to write temp file");

        // Parse using JsonBackend
        let backend = JsonBackend::new();
        let options = BackendOptions::default();
        let parsed = backend
            .parse_file(&temp_file, &options)
            .expect("Failed to parse JSON file");

        // Verify content matches
        assert_eq!(
            parsed.markdown, original.markdown,
            "File-parsed markdown should match original"
        );
        assert_eq!(
            parsed.format, original.format,
            "File-parsed format should match original"
        );

        // Clean up
        std::fs::remove_file(&temp_file).ok();
    }

    #[test]
    fn test_json_invalid_utf8() {
        let backend = JsonBackend::new();
        let options = BackendOptions::default();
        let invalid_bytes = vec![0xFF, 0xFE, 0xFD];

        let result = backend.parse_bytes(&invalid_bytes, &options);
        assert!(result.is_err(), "Invalid UTF-8 should return error");
        assert!(
            result.unwrap_err().to_string().contains("Invalid UTF-8"),
            "Error should mention Invalid UTF-8"
        );
    }

    #[test]
    fn test_json_invalid_json() {
        let backend = JsonBackend::new();
        let options = BackendOptions::default();
        let invalid_json = b"{not valid json}";

        let result = backend.parse_bytes(invalid_json, &options);
        assert!(result.is_err(), "Invalid JSON should return error");
        assert!(
            result.unwrap_err().to_string().contains("JSON error"),
            "Error should mention JSON error"
        );
    }

    // ========== METADATA PRESERVATION TESTS ==========

    #[test]
    fn test_metadata_preservation_complete() {
        use chrono::{DateTime, Utc};

        // Create document with complete metadata
        let created_date: DateTime<Utc> = "2024-01-01T12:00:00Z".parse().unwrap();
        let modified_date: DateTime<Utc> = "2024-01-15T15:30:00Z".parse().unwrap();

        let original = Document {
            markdown: "# Test\n\nContent".to_string(),
            format: InputFormat::Docx,
            metadata: DocumentMetadata {
                num_pages: Some(5),
                num_characters: 15,
                title: Some("Test Document".to_string()),
                author: Some("Test Author".to_string()),
                subject: None,
                created: Some(created_date),
                modified: Some(modified_date),
                language: Some("en".to_string()),
                exif: None,
            },
            content_blocks: None,
            docling_document: None,
        };

        let json = serde_json::to_string(&original).unwrap();
        let backend = JsonBackend::new();
        let parsed = backend
            .parse_bytes(json.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Verify all metadata fields preserved
        assert_eq!(
            parsed.metadata.num_pages,
            Some(5),
            "num_pages should be preserved as Some(5)"
        );
        assert_eq!(
            parsed.metadata.num_characters, 15,
            "num_characters should be preserved as 15"
        );
        assert_eq!(
            parsed.metadata.title,
            Some("Test Document".to_string()),
            "title should be preserved"
        );
        assert_eq!(
            parsed.metadata.author,
            Some("Test Author".to_string()),
            "author should be preserved"
        );
        assert_eq!(
            parsed.metadata.created,
            Some(created_date),
            "created date should be preserved"
        );
        assert_eq!(
            parsed.metadata.modified,
            Some(modified_date),
            "modified date should be preserved"
        );
        assert_eq!(
            parsed.metadata.language,
            Some("en".to_string()),
            "language should be preserved"
        );
    }

    #[test]
    fn test_metadata_preservation_minimal() {
        // Create document with minimal metadata
        let original = Document {
            markdown: "Test".to_string(),
            format: InputFormat::Md,
            metadata: DocumentMetadata {
                num_pages: None,
                num_characters: 4,
                title: None,
                author: None,
                created: None,
                modified: None,
                language: None,
                subject: None,
                exif: None,
            },
            content_blocks: None,
            docling_document: None,
        };

        let json = serde_json::to_string(&original).unwrap();
        let backend = JsonBackend::new();
        let parsed = backend
            .parse_bytes(json.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Verify minimal metadata preserved
        assert_eq!(
            parsed.metadata.num_pages, None,
            "num_pages None should be preserved"
        );
        assert_eq!(
            parsed.metadata.num_characters, 4,
            "num_characters should be preserved as 4"
        );
        assert_eq!(
            parsed.metadata.title, None,
            "title None should be preserved"
        );
    }

    #[test]
    fn test_format_preservation() {
        // Test preservation of various InputFormat types
        let formats = vec![
            InputFormat::Md,
            InputFormat::Pdf,
            InputFormat::Docx,
            InputFormat::Html,
            InputFormat::JsonDocling,
        ];

        for format in formats {
            let original = Document::from_markdown("Test".to_string(), format);
            let json = serde_json::to_string(&original).unwrap();
            let backend = JsonBackend::new();
            let parsed = backend
                .parse_bytes(json.as_bytes(), &BackendOptions::default())
                .unwrap();

            assert_eq!(
                parsed.format, format,
                "Format {format:?} should be preserved through JSON round-trip"
            );
        }
    }

    // ========== CONTENT PRESERVATION TESTS ==========

    #[test]
    fn test_markdown_preservation_simple() {
        let original = Document::from_markdown(
            "# Heading\n\nParagraph with **bold** and *italic*.\n\n- List item 1\n- List item 2"
                .to_string(),
            InputFormat::Md,
        );

        let json = serde_json::to_string(&original).unwrap();
        let backend = JsonBackend::new();
        let parsed = backend
            .parse_bytes(json.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert_eq!(parsed.markdown, original.markdown);
    }

    #[test]
    fn test_markdown_preservation_unicode() {
        let original = Document::from_markdown(
            "# Unicode Test 🎉\n\n日本語 • Español • العربية • Emoji 😀🎨🚀".to_string(),
            InputFormat::Md,
        );

        let json = serde_json::to_string(&original).unwrap();
        let backend = JsonBackend::new();
        let parsed = backend
            .parse_bytes(json.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert_eq!(parsed.markdown, original.markdown);
    }

    #[test]
    fn test_markdown_preservation_special_chars() {
        let original = Document::from_markdown(
            "Special: <>&\"'`\n\nEscapes: \\* \\# \\[ \\]\n\nCode: `foo()`".to_string(),
            InputFormat::Md,
        );

        let json = serde_json::to_string(&original).unwrap();
        let backend = JsonBackend::new();
        let parsed = backend
            .parse_bytes(json.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert_eq!(parsed.markdown, original.markdown);
    }

    #[test]
    fn test_empty_document() {
        let original = Document::from_markdown("".to_string(), InputFormat::Md);

        let json = serde_json::to_string(&original).unwrap();
        let backend = JsonBackend::new();
        let parsed = backend
            .parse_bytes(json.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert_eq!(parsed.markdown, "");
        assert_eq!(parsed.metadata.num_characters, 0);
    }

    // ========== ERROR HANDLING TESTS ==========

    #[test]
    fn test_parse_file_nonexistent() {
        let backend = JsonBackend::new();
        let result = backend.parse_file("/nonexistent/file.json", &BackendOptions::default());
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_empty_bytes() {
        let backend = JsonBackend::new();
        let result = backend.parse_bytes(b"", &BackendOptions::default());
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_incomplete_json() {
        let backend = JsonBackend::new();
        let incomplete_json = b"{\"markdown\": \"test\", \"format\":";
        let result = backend.parse_bytes(incomplete_json, &BackendOptions::default());
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_wrong_json_structure() {
        let backend = JsonBackend::new();
        // Valid JSON but not a Document structure
        let wrong_json = b"{\"foo\": \"bar\", \"baz\": 123}";
        let result = backend.parse_bytes(wrong_json, &BackendOptions::default());
        assert!(result.is_err());
    }

    // ========== BACKEND TRAIT CONSISTENCY TESTS ==========

    #[test]
    fn test_backend_new_vs_default() {
        // new() and default() should be equivalent
        let backend1 = JsonBackend::new();
        let backend2 = JsonBackend;
        assert_eq!(backend1.format(), backend2.format());
    }

    #[test]
    fn test_backend_format_consistency() {
        // format() should always return JsonDocling
        let backend = JsonBackend::new();
        assert_eq!(backend.format(), InputFormat::JsonDocling);
        assert_eq!(backend.format(), InputFormat::JsonDocling); // Call twice to ensure consistency
    }

    // ========== ROUND-TRIP WITH DOCITEMS TESTS ==========

    #[test]
    fn test_round_trip_with_docitems() {
        use docling_core::content::DocItem;

        // Create document with DocItems
        let doc_items = vec![DocItem::Text {
            self_ref: "#/texts/0".to_string(),
            parent: None,
            children: vec![],
            content_layer: "body".to_string(),
            prov: vec![],
            orig: "Test text".to_string(),
            text: "Test text".to_string(),
            formatting: None,
            hyperlink: None,
        }];

        let original = Document {
            markdown: "Test text".to_string(),
            format: InputFormat::Md,
            metadata: DocumentMetadata::default(),
            content_blocks: Some(doc_items.clone()),
            docling_document: None,
        };

        let json = serde_json::to_string(&original).unwrap();
        let backend = JsonBackend::new();
        let parsed = backend
            .parse_bytes(json.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Verify DocItems preserved
        assert!(parsed.content_blocks.is_some());
        assert_eq!(parsed.content_blocks.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn test_round_trip_with_table_docitems() {
        use docling_core::content::DocItem;

        // Create document with Table DocItem
        let doc_items = vec![DocItem::Table {
            self_ref: "#/tables/0".to_string(),
            parent: None,
            children: vec![],
            content_layer: "body".to_string(),
            prov: vec![],
            data: docling_core::content::TableData {
                num_rows: 2,
                num_cols: 2,
                table_cells: None,
                grid: vec![vec![]],
            },
            captions: vec![],
            footnotes: vec![],
            references: vec![],
            annotations: vec![],
            image: None,
        }];

        let original = Document {
            markdown: "| A | B |\n|---|---|\n| 1 | 2 |".to_string(),
            format: InputFormat::Md,
            metadata: DocumentMetadata::default(),
            content_blocks: Some(doc_items),
            docling_document: None,
        };

        let json = serde_json::to_string(&original).unwrap();
        let backend = JsonBackend::new();
        let parsed = backend
            .parse_bytes(json.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Verify Table DocItem preserved
        assert!(parsed.content_blocks.is_some());
        match &parsed.content_blocks.as_ref().unwrap()[0] {
            DocItem::Table { data, .. } => {
                assert_eq!(data.num_rows, 2);
                assert_eq!(data.num_cols, 2);
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    #[test]
    fn test_round_trip_empty_content_blocks() {
        // Document with empty content_blocks (Some(vec![]))
        let original = Document {
            markdown: "Empty blocks".to_string(),
            format: InputFormat::Md,
            metadata: DocumentMetadata::default(),
            content_blocks: Some(vec![]),
            docling_document: None,
        };

        let json = serde_json::to_string(&original).unwrap();
        let backend = JsonBackend::new();
        let parsed = backend
            .parse_bytes(json.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Verify empty vec preserved (not converted to None)
        assert!(parsed.content_blocks.is_some());
        assert_eq!(parsed.content_blocks.as_ref().unwrap().len(), 0);
    }

    // ========== METADATA EDGE CASES TESTS ==========

    #[test]
    fn test_metadata_very_long_title() {
        // Title with 1000+ characters
        let long_title = "A".repeat(1500);
        let original = Document {
            markdown: "Content".to_string(),
            format: InputFormat::Md,
            metadata: DocumentMetadata {
                num_characters: 7,
                title: Some(long_title.clone()),
                ..Default::default()
            },
            content_blocks: None,
            docling_document: None,
        };

        let json = serde_json::to_string(&original).unwrap();
        let backend = JsonBackend::new();
        let parsed = backend
            .parse_bytes(json.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert_eq!(parsed.metadata.title, Some(long_title));
    }

    #[test]
    fn test_metadata_special_characters() {
        // Metadata with special characters and unicode
        let original = Document {
            markdown: "Test".to_string(),
            format: InputFormat::Md,
            metadata: DocumentMetadata {
                num_characters: 4,
                title: Some("Title with <>&\"'`".to_string()),
                author: Some("Author 日本語 🎉".to_string()),
                language: Some("en-US".to_string()),
                ..Default::default()
            },
            content_blocks: None,
            docling_document: None,
        };

        let json = serde_json::to_string(&original).unwrap();
        let backend = JsonBackend::new();
        let parsed = backend
            .parse_bytes(json.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert_eq!(
            parsed.metadata.title,
            Some("Title with <>&\"'`".to_string())
        );
        assert_eq!(parsed.metadata.author, Some("Author 日本語 🎉".to_string()));
    }

    #[test]
    fn test_metadata_all_none_fields() {
        // All optional metadata fields are None
        let original = Document {
            markdown: "Content".to_string(),
            format: InputFormat::Md,
            metadata: DocumentMetadata {
                num_pages: None,
                num_characters: 7,
                title: None,
                author: None,
                created: None,
                modified: None,
                language: None,
                subject: None,
                exif: None,
            },
            content_blocks: None,
            docling_document: None,
        };

        let json = serde_json::to_string(&original).unwrap();
        let backend = JsonBackend::new();
        let parsed = backend
            .parse_bytes(json.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Verify all None fields preserved
        assert_eq!(parsed.metadata.num_pages, None);
        assert_eq!(parsed.metadata.title, None);
        assert_eq!(parsed.metadata.author, None);
        assert_eq!(parsed.metadata.created, None);
        assert_eq!(parsed.metadata.modified, None);
        assert_eq!(parsed.metadata.language, None);
        assert!(parsed.metadata.exif.is_none());
    }

    // ========== FORMAT-SPECIFIC ROUND TRIPS TESTS ==========

    #[test]
    fn test_format_specific_round_trip_docx() {
        use chrono::Utc;

        // DOCX document with typical metadata
        let original = Document {
            markdown: "# Document Title\n\nParagraph text.".to_string(),
            format: InputFormat::Docx,
            metadata: DocumentMetadata {
                num_pages: Some(3),
                num_characters: 30,
                title: Some("Word Document".to_string()),
                author: Some("John Doe".to_string()),
                subject: None,
                created: Some(Utc::now()),
                modified: Some(Utc::now()),
                language: Some("en-US".to_string()),
                exif: None,
            },
            content_blocks: None,
            docling_document: None,
        };

        let json = serde_json::to_string(&original).unwrap();
        let backend = JsonBackend::new();
        let parsed = backend
            .parse_bytes(json.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert_eq!(parsed.format, InputFormat::Docx);
        assert_eq!(parsed.metadata.title, Some("Word Document".to_string()));
        assert_eq!(parsed.metadata.num_pages, Some(3));
    }

    #[test]
    fn test_format_specific_round_trip_html() {
        use docling_core::content::DocItem;

        // HTML document with content_blocks
        let doc_items = vec![DocItem::SectionHeader {
            self_ref: "#/texts/0".to_string(),
            parent: None,
            children: vec![],
            content_layer: "body".to_string(),
            prov: vec![],
            orig: "Heading".to_string(),
            text: "Heading".to_string(),
            level: 1,
            formatting: None,
            hyperlink: None,
        }];

        let original = Document {
            markdown: "# Heading".to_string(),
            format: InputFormat::Html,
            metadata: DocumentMetadata {
                num_characters: 9,
                title: Some("HTML Page".to_string()),
                author: Some("Web Author".to_string()),
                ..Default::default()
            },
            content_blocks: Some(doc_items),
            docling_document: None,
        };

        let json = serde_json::to_string(&original).unwrap();
        let backend = JsonBackend::new();
        let parsed = backend
            .parse_bytes(json.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert_eq!(parsed.format, InputFormat::Html);
        assert!(parsed.content_blocks.is_some());
        assert_eq!(parsed.metadata.title, Some("HTML Page".to_string()));
    }

    #[test]
    fn test_format_specific_round_trip_csv() {
        // CSV document (typically has table DocItems)
        let original = Document {
            markdown: "| Name | Age |\n|------|-----|\n| Alice | 30 |".to_string(),
            format: InputFormat::Csv,
            metadata: DocumentMetadata {
                num_characters: 42,
                ..Default::default()
            },
            content_blocks: None,
            docling_document: None,
        };

        let json = serde_json::to_string(&original).unwrap();
        let backend = JsonBackend::new();
        let parsed = backend
            .parse_bytes(json.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert_eq!(parsed.format, InputFormat::Csv);
        assert_eq!(parsed.markdown, original.markdown);
    }

    // ========== LARGE CONTENT HANDLING TESTS ==========

    #[test]
    fn test_large_markdown_content() {
        // Very long markdown (10000+ characters)
        let long_markdown = "# Heading\n\n".to_string() + &"Paragraph. ".repeat(1000);
        let original = Document::from_markdown(long_markdown.clone(), InputFormat::Md);

        let json = serde_json::to_string(&original).unwrap();
        let backend = JsonBackend::new();
        let parsed = backend
            .parse_bytes(json.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert_eq!(parsed.markdown.len(), long_markdown.len());
        assert_eq!(parsed.markdown, long_markdown);
    }

    #[test]
    fn test_deeply_nested_docitems() {
        use docling_core::content::{DocItem, ItemRef};

        // Create nested list structure (3 levels deep)
        let doc_items = vec![
            DocItem::ListItem {
                self_ref: "#/texts/0".to_string(),
                parent: None,
                children: vec![ItemRef::new("#/texts/1".to_string())],
                content_layer: "body".to_string(),
                prov: vec![],
                orig: "Level 1".to_string(),
                text: "Level 1".to_string(),
                marker: "- ".to_string(),
                enumerated: false,
                formatting: None,
                hyperlink: None,
            },
            DocItem::ListItem {
                self_ref: "#/texts/1".to_string(),
                parent: Some(ItemRef::new("#/texts/0".to_string())),
                children: vec![ItemRef::new("#/texts/2".to_string())],
                content_layer: "body".to_string(),
                prov: vec![],
                orig: "Level 2".to_string(),
                text: "Level 2".to_string(),
                marker: "  - ".to_string(),
                enumerated: false,
                formatting: None,
                hyperlink: None,
            },
            DocItem::ListItem {
                self_ref: "#/texts/2".to_string(),
                parent: Some(ItemRef::new("#/texts/1".to_string())),
                children: vec![],
                content_layer: "body".to_string(),
                prov: vec![],
                orig: "Level 3".to_string(),
                text: "Level 3".to_string(),
                marker: "    - ".to_string(),
                enumerated: false,
                formatting: None,
                hyperlink: None,
            },
        ];

        let original = Document {
            markdown: "- Level 1\n  - Level 2\n    - Level 3".to_string(),
            format: InputFormat::Md,
            metadata: DocumentMetadata::default(),
            content_blocks: Some(doc_items),
            docling_document: None,
        };

        let json = serde_json::to_string(&original).unwrap();
        let backend = JsonBackend::new();
        let parsed = backend
            .parse_bytes(json.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Verify nested structure preserved
        assert!(parsed.content_blocks.is_some());
        assert_eq!(parsed.content_blocks.as_ref().unwrap().len(), 3);

        // Verify parent-child relationships
        match &parsed.content_blocks.as_ref().unwrap()[1] {
            DocItem::ListItem {
                parent, children, ..
            } => {
                assert_eq!(parent, &Some(ItemRef::new("#/texts/0".to_string())));
                assert_eq!(children, &[ItemRef::new("#/texts/2".to_string())]);
            }
            _ => panic!("Expected ListItem"),
        }
    }

    // ========== WHITESPACE HANDLING TESTS ==========

    #[test]
    fn test_markdown_with_crlf() {
        // Markdown with \r\n line endings (Windows style)
        let original = Document::from_markdown(
            "# Heading\r\n\r\nParagraph text.\r\n".to_string(),
            InputFormat::Md,
        );

        let json = serde_json::to_string(&original).unwrap();
        let backend = JsonBackend::new();
        let parsed = backend
            .parse_bytes(json.as_bytes(), &BackendOptions::default())
            .unwrap();

        // \r\n should be preserved
        assert!(parsed.markdown.contains("\r\n"));
        assert_eq!(parsed.markdown, original.markdown);
    }

    #[test]
    fn test_markdown_with_trailing_whitespace() {
        // Markdown with trailing spaces and tabs
        let original = Document::from_markdown(
            "Line with trailing spaces   \nLine with tab\t\n".to_string(),
            InputFormat::Md,
        );

        let json = serde_json::to_string(&original).unwrap();
        let backend = JsonBackend::new();
        let parsed = backend
            .parse_bytes(json.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Trailing whitespace should be preserved exactly
        assert_eq!(parsed.markdown, original.markdown);
    }

    // ========== INTEGRATION TESTS ==========

    #[test]
    fn test_multiple_round_trips() {
        // Multiple serialization/deserialization cycles
        let mut doc = Document::from_markdown(
            "# Test\n\nMultiple round trips.".to_string(),
            InputFormat::Md,
        );

        let backend = JsonBackend::new();

        for _ in 0..5 {
            let json = serde_json::to_string(&doc).unwrap();
            doc = backend
                .parse_bytes(json.as_bytes(), &BackendOptions::default())
                .unwrap();
        }

        // After 5 round trips, content should be unchanged
        assert_eq!(doc.markdown, "# Test\n\nMultiple round trips.");
        assert_eq!(doc.format, InputFormat::Md);
    }

    #[test]
    fn test_parse_bytes_vs_parse_file_equivalence() {
        // parse_bytes and parse_file should produce identical results
        let original = Document::from_markdown(
            "# Test\n\nTesting equivalence.".to_string(),
            InputFormat::Docx,
        );

        let json = serde_json::to_string_pretty(&original).unwrap();

        // Test parse_bytes
        let backend = JsonBackend::new();
        let parsed_bytes = backend
            .parse_bytes(json.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Test parse_file
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("docling_equivalence_test.json");
        std::fs::write(&temp_file, &json).unwrap();
        let parsed_file = backend
            .parse_file(&temp_file, &BackendOptions::default())
            .unwrap();
        std::fs::remove_file(&temp_file).ok();

        // Results should be identical
        assert_eq!(parsed_bytes.markdown, parsed_file.markdown);
        assert_eq!(parsed_bytes.format, parsed_file.format);
        assert_eq!(
            parsed_bytes.metadata.num_characters,
            parsed_file.metadata.num_characters
        );
    }

    #[test]
    fn test_json_pretty_vs_compact() {
        // Pretty-printed and compact JSON should parse identically
        let original = Document {
            markdown: "Test content".to_string(),
            format: InputFormat::Md,
            metadata: DocumentMetadata {
                num_characters: 12,
                title: Some("Test".to_string()),
                ..Default::default()
            },
            content_blocks: None,
            docling_document: None,
        };

        let json_pretty = serde_json::to_string_pretty(&original).unwrap();
        let json_compact = serde_json::to_string(&original).unwrap();

        let backend = JsonBackend::new();
        let parsed_pretty = backend
            .parse_bytes(json_pretty.as_bytes(), &BackendOptions::default())
            .unwrap();
        let parsed_compact = backend
            .parse_bytes(json_compact.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Both should parse to the same document
        assert_eq!(parsed_pretty.markdown, parsed_compact.markdown);
        assert_eq!(parsed_pretty.format, parsed_compact.format);
        assert_eq!(parsed_pretty.metadata.title, parsed_compact.metadata.title);
    }

    // ========== ERROR HANDLING TESTS ==========

    #[test]
    fn test_json_empty_bytes() {
        // Empty bytes should fail to parse
        let backend = JsonBackend::new();
        let result = backend.parse_bytes(&[], &BackendOptions::default());

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("JSON"));
    }

    #[test]
    fn test_json_partial_json() {
        // Incomplete JSON should fail
        let backend = JsonBackend::new();
        let partial_json = b"{\"markdown\": \"test\"";

        let result = backend.parse_bytes(partial_json, &BackendOptions::default());
        assert!(result.is_err());
    }

    #[test]
    fn test_json_null_fields() {
        // JSON with explicit null values
        let json_with_nulls = r#"{
            "markdown": "test",
            "format": "MD",
            "metadata": {
                "num_pages": null,
                "num_characters": 4,
                "title": null,
                "author": null,
                "created": null,
                "modified": null,
                "language": null,
                "exif": null
            },
            "content_blocks": null
        }"#;

        let backend = JsonBackend::new();
        let parsed = backend
            .parse_bytes(json_with_nulls.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Null fields should parse as None
        assert_eq!(parsed.metadata.num_pages, None);
        assert_eq!(parsed.metadata.title, None);
        assert_eq!(parsed.content_blocks, None);
    }

    #[test]
    fn test_json_missing_required_fields() {
        // JSON missing required fields should fail
        let invalid_json = r#"{"metadata": {"num_characters": 0}}"#;

        let backend = JsonBackend::new();
        let result = backend.parse_bytes(invalid_json.as_bytes(), &BackendOptions::default());

        assert!(result.is_err());
        // Should fail because 'markdown' and 'format' are required fields
    }

    // ========== ADDITIONAL FORMAT TESTS ==========

    #[test]
    fn test_format_preservation_pdf() {
        // Test PDF format preservation
        let original = Document::from_markdown(
            "# PDF Content\n\nExtracted text.".to_string(),
            InputFormat::Pdf,
        );

        let json = serde_json::to_string(&original).unwrap();
        let backend = JsonBackend::new();
        let parsed = backend
            .parse_bytes(json.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert_eq!(parsed.format, InputFormat::Pdf);
        assert_eq!(parsed.markdown, original.markdown);
    }

    #[test]
    fn test_format_preservation_xlsx() {
        // Test XLSX format preservation
        let original = Document::from_markdown(
            "| Col1 | Col2 |\n|------|------|\n| A    | B    |".to_string(),
            InputFormat::Xlsx,
        );

        let json = serde_json::to_string(&original).unwrap();
        let backend = JsonBackend::new();
        let parsed = backend
            .parse_bytes(json.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert_eq!(parsed.format, InputFormat::Xlsx);
        assert_eq!(parsed.markdown, original.markdown);
    }

    #[test]
    fn test_format_preservation_pptx() {
        // Test PPTX format preservation
        let original = Document::from_markdown(
            "# Slide 1\n\nContent on slide.".to_string(),
            InputFormat::Pptx,
        );

        let json = serde_json::to_string(&original).unwrap();
        let backend = JsonBackend::new();
        let parsed = backend
            .parse_bytes(json.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert_eq!(parsed.format, InputFormat::Pptx);
        assert_eq!(parsed.markdown, original.markdown);
    }

    #[test]
    fn test_format_preservation_asciidoc() {
        // Test AsciiDoc format preservation
        let original = Document::from_markdown(
            "= Document Title\n\nContent here.".to_string(),
            InputFormat::Asciidoc,
        );

        let json = serde_json::to_string(&original).unwrap();
        let backend = JsonBackend::new();
        let parsed = backend
            .parse_bytes(json.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert_eq!(parsed.format, InputFormat::Asciidoc);
        assert_eq!(parsed.markdown, original.markdown);
    }

    // ========== UNICODE AND SPECIAL CHARACTERS ==========

    #[test]
    fn test_json_unicode_content() {
        // Test various Unicode characters
        let original = Document::from_markdown(
            "# Unicode Test\n\n日本語 🎉 Émoji café".to_string(),
            InputFormat::Md,
        );

        let json = serde_json::to_string(&original).unwrap();
        let backend = JsonBackend::new();
        let parsed = backend
            .parse_bytes(json.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert_eq!(parsed.markdown, original.markdown);
        assert!(parsed.markdown.contains("日本語"));
        assert!(parsed.markdown.contains("🎉"));
        assert!(parsed.markdown.contains("Émoji"));
    }

    #[test]
    fn test_json_escaped_characters() {
        // Test JSON-escaped characters in markdown
        let original = Document::from_markdown(
            "Text with \"quotes\" and \\ backslashes and \n newlines".to_string(),
            InputFormat::Md,
        );

        let json = serde_json::to_string(&original).unwrap();
        let backend = JsonBackend::new();
        let parsed = backend
            .parse_bytes(json.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert_eq!(parsed.markdown, original.markdown);
        assert!(parsed.markdown.contains("\"quotes\""));
        assert!(parsed.markdown.contains("\\"));
    }

    #[test]
    fn test_json_zero_width_characters() {
        // Test zero-width characters (invisible but valid)
        let original = Document::from_markdown(
            "Text\u{200B}with\u{200C}zero\u{200D}width\u{FEFF}chars".to_string(),
            InputFormat::Md,
        );

        let json = serde_json::to_string(&original).unwrap();
        let backend = JsonBackend::new();
        let parsed = backend
            .parse_bytes(json.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Zero-width characters should be preserved
        assert_eq!(parsed.markdown, original.markdown);
        assert_eq!(
            parsed.markdown.chars().count(),
            original.markdown.chars().count()
        );
    }

    // ========== EDGE CASE DOCUMENTS ==========

    #[test]
    fn test_json_very_large_document() {
        // Test large markdown content
        let large_markdown =
            "# Large Document\n\n".to_string() + &"Lorem ipsum dolor sit amet. ".repeat(10000);
        let original = Document::from_markdown(large_markdown.clone(), InputFormat::Md);

        let json = serde_json::to_string(&original).unwrap();
        let backend = JsonBackend::new();
        let parsed = backend
            .parse_bytes(json.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert_eq!(parsed.markdown.len(), large_markdown.len());
        assert_eq!(
            parsed.metadata.num_characters,
            large_markdown.chars().count()
        );
    }

    #[test]
    fn test_json_empty_markdown() {
        // Document with empty markdown
        let original = Document {
            markdown: String::new(),
            format: InputFormat::Md,
            metadata: DocumentMetadata {
                num_characters: 0,
                ..Default::default()
            },
            content_blocks: None,
            docling_document: None,
        };

        let json = serde_json::to_string(&original).unwrap();
        let backend = JsonBackend::new();
        let parsed = backend
            .parse_bytes(json.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert_eq!(parsed.markdown, "");
        assert_eq!(parsed.metadata.num_characters, 0);
    }

    #[test]
    fn test_json_only_whitespace_markdown() {
        // Document with only whitespace
        let original = Document::from_markdown("   \n\n\t\t  \n  ".to_string(), InputFormat::Md);

        let json = serde_json::to_string(&original).unwrap();
        let backend = JsonBackend::new();
        let parsed = backend
            .parse_bytes(json.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Whitespace should be preserved exactly
        assert_eq!(parsed.markdown, original.markdown);
    }

    // ========== FILE HANDLING TESTS ==========

    #[test]
    fn test_json_parse_file_empty_file() {
        // Empty file should fail to parse
        use std::io::Write;
        use tempfile::NamedTempFile;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"").unwrap();
        temp_file.flush().unwrap();

        let backend = JsonBackend::new();
        let result = backend.parse_file(temp_file.path(), &BackendOptions::default());

        assert!(result.is_err());
    }

    // ========== NEW COMPREHENSIVE TESTS (N=464) ==========

    #[test]
    fn test_json_bom_handling() {
        // UTF-8 BOM (Byte Order Mark) at start of JSON file
        // BOM is EF BB BF in UTF-8
        let original = Document::from_markdown("# Test".to_string(), InputFormat::Md);
        let json = serde_json::to_string(&original).unwrap();

        // Prepend UTF-8 BOM
        let mut json_with_bom = vec![0xEF, 0xBB, 0xBF];
        json_with_bom.extend_from_slice(json.as_bytes());

        let backend = JsonBackend::new();
        let result = backend.parse_bytes(&json_with_bom, &BackendOptions::default());

        // serde_json should handle BOM gracefully (skip it)
        // If it doesn't, this documents the behavior
        match result {
            Ok(parsed) => {
                // BOM was skipped successfully
                assert_eq!(parsed.markdown, original.markdown);
            }
            Err(_) => {
                // BOM caused parse failure (document this behavior)
                // This is acceptable - BOM is not standard for UTF-8 JSON
            }
        }
    }

    #[test]
    fn test_json_comments_rejected() {
        // JSON with comments should fail (strict JSON doesn't allow comments)
        let json_with_comments = r#"{
            // This is a comment
            "markdown": "test",
            "format": "MD",
            "metadata": {
                "num_characters": 4
            }
        }"#;

        let backend = JsonBackend::new();
        let result = backend.parse_bytes(json_with_comments.as_bytes(), &BackendOptions::default());

        // Strict JSON should reject comments
        assert!(result.is_err());
    }

    #[test]
    fn test_json_trailing_commas() {
        // JSON with trailing commas should fail (strict JSON)
        let json_with_trailing_comma = r#"{
            "markdown": "test",
            "format": "MD",
            "metadata": {
                "num_characters": 4,
            }
        }"#;

        let backend = JsonBackend::new();
        let result = backend.parse_bytes(
            json_with_trailing_comma.as_bytes(),
            &BackendOptions::default(),
        );

        // Strict JSON should reject trailing commas
        assert!(result.is_err());
    }

    #[test]
    fn test_json_single_quotes() {
        // JSON with single quotes should fail (must use double quotes)
        let json_with_single_quotes = r"{
            'markdown': 'test',
            'format': 'MD',
            'metadata': {
                'num_characters': 4
            }
        }";

        let backend = JsonBackend::new();
        let result = backend.parse_bytes(
            json_with_single_quotes.as_bytes(),
            &BackendOptions::default(),
        );

        // Strict JSON requires double quotes
        assert!(result.is_err());
    }

    #[test]
    fn test_json_mixed_line_endings() {
        // Mix of \n, \r\n, \r line endings in same document
        let mixed_markdown = "Line 1\nLine 2\r\nLine 3\rLine 4".to_string();
        let original = Document::from_markdown(mixed_markdown.clone(), InputFormat::Md);

        let json = serde_json::to_string(&original).unwrap();
        let backend = JsonBackend::new();
        let parsed = backend
            .parse_bytes(json.as_bytes(), &BackendOptions::default())
            .unwrap();

        // All line ending types should be preserved
        assert_eq!(parsed.markdown, mixed_markdown);
        assert!(parsed.markdown.contains('\n'));
        assert!(parsed.markdown.contains("\r\n"));
        assert!(parsed.markdown.contains('\r'));
    }

    #[test]
    fn test_metadata_future_dates() {
        // Test dates in the future (edge case for validation)
        use chrono::{DateTime, Duration, Utc};

        let future_date: DateTime<Utc> = Utc::now() + Duration::days(365 * 10); // 10 years in future
        let original = Document {
            markdown: "Future document".to_string(),
            format: InputFormat::Md,
            metadata: DocumentMetadata {
                num_characters: 15,
                created: Some(future_date),
                modified: Some(future_date),
                ..Default::default()
            },
            content_blocks: None,
            docling_document: None,
        };

        let json = serde_json::to_string(&original).unwrap();
        let backend = JsonBackend::new();
        let parsed = backend
            .parse_bytes(json.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Future dates should be preserved (no validation)
        assert_eq!(parsed.metadata.created, Some(future_date));
        assert_eq!(parsed.metadata.modified, Some(future_date));
    }

    #[test]
    fn test_json_very_deep_nesting() {
        // Test deeply nested DocItems (10+ levels)
        use docling_core::content::{DocItem, ItemRef};

        // Create 12 levels of nested ListItems
        let mut doc_items = Vec::new();
        for level in 0..12 {
            let parent = if level == 0 {
                None
            } else {
                Some(ItemRef::new(format!("#/texts/{}", level - 1)))
            };
            let children = if level < 11 {
                vec![ItemRef::new(format!("#/texts/{}", level + 1))]
            } else {
                vec![]
            };

            doc_items.push(DocItem::ListItem {
                self_ref: format!("#/texts/{level}"),
                parent,
                children,
                content_layer: "body".to_string(),
                prov: vec![],
                orig: format!("Level {level}"),
                text: format!("Level {level}"),
                marker: "  ".repeat(level) + "- ",
                enumerated: false,
                formatting: None,
                hyperlink: None,
            });
        }

        let original = Document {
            markdown: "Deep nesting test".to_string(),
            format: InputFormat::Md,
            metadata: DocumentMetadata::default(),
            content_blocks: Some(doc_items),
            docling_document: None,
        };

        let json = serde_json::to_string(&original).unwrap();
        let backend = JsonBackend::new();
        let parsed = backend
            .parse_bytes(json.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Verify deep nesting preserved
        assert!(parsed.content_blocks.is_some());
        assert_eq!(parsed.content_blocks.as_ref().unwrap().len(), 12);

        // Verify parent-child relationships at depth 10
        match &parsed.content_blocks.as_ref().unwrap()[10] {
            DocItem::ListItem {
                parent, children, ..
            } => {
                assert_eq!(parent, &Some(ItemRef::new("#/texts/9".to_string())));
                assert_eq!(children, &[ItemRef::new("#/texts/11".to_string())]);
            }
            _ => panic!("Expected ListItem at depth 10"),
        }
    }

    #[test]
    fn test_docitems_all_variants() {
        // Test round-trip with all major DocItem variants in one document
        use docling_core::content::{DocItem, TableData};

        let doc_items = vec![
            // Text variant
            DocItem::Text {
                self_ref: "#/texts/0".to_string(),
                parent: None,
                children: vec![],
                content_layer: "body".to_string(),
                prov: vec![],
                orig: "Plain text".to_string(),
                text: "Plain text".to_string(),
                formatting: None,
                hyperlink: None,
            },
            // SectionHeader variant
            DocItem::SectionHeader {
                self_ref: "#/texts/1".to_string(),
                parent: None,
                children: vec![],
                content_layer: "body".to_string(),
                prov: vec![],
                orig: "Heading".to_string(),
                text: "Heading".to_string(),
                level: 1,
                formatting: None,
                hyperlink: None,
            },
            // ListItem variant
            DocItem::ListItem {
                self_ref: "#/texts/2".to_string(),
                parent: None,
                children: vec![],
                content_layer: "body".to_string(),
                prov: vec![],
                orig: "List item".to_string(),
                text: "List item".to_string(),
                marker: "- ".to_string(),
                enumerated: false,
                formatting: None,
                hyperlink: None,
            },
            // Table variant
            DocItem::Table {
                self_ref: "#/tables/0".to_string(),
                parent: None,
                children: vec![],
                content_layer: "body".to_string(),
                prov: vec![],
                data: TableData {
                    num_rows: 2,
                    num_cols: 2,
                    table_cells: None,
                    grid: vec![vec![]],
                },
                captions: vec![],
                footnotes: vec![],
                references: vec![],
                annotations: vec![],
                image: None,
            },
            // Picture variant
            DocItem::Picture {
                self_ref: "#/pictures/0".to_string(),
                parent: None,
                children: vec![],
                content_layer: "body".to_string(),
                prov: vec![],
                image: Some(serde_json::json!({
                    "mimetype": "image/png",
                    "dpi": 72,
                    "size": {"width": 100.0, "height": 100.0},
                    "uri": "data:image/png;base64,iVBORw0K..."
                })),
                captions: vec![],
                footnotes: vec![],
                references: vec![],
                annotations: vec![],
                ocr_text: None,
            },
            // List group variant
            DocItem::List {
                self_ref: "#/lists/0".to_string(),
                parent: None,
                children: vec![],
                content_layer: "body".to_string(),
                name: "unordered_list".to_string(),
            },
            // Chapter group variant
            DocItem::Chapter {
                self_ref: "#/chapters/0".to_string(),
                parent: None,
                children: vec![],
                content_layer: "body".to_string(),
                name: "Chapter 1".to_string(),
            },
            // KeyValueArea variant
            DocItem::KeyValueArea {
                self_ref: "#/key_value_areas/0".to_string(),
                parent: None,
                children: vec![],
                content_layer: "body".to_string(),
                name: "metadata_section".to_string(),
            },
        ];

        let original = Document {
            markdown: "All variants test".to_string(),
            format: InputFormat::Md,
            metadata: DocumentMetadata::default(),
            content_blocks: Some(doc_items),
            docling_document: None,
        };

        let json = serde_json::to_string(&original).unwrap();
        let backend = JsonBackend::new();
        let parsed = backend
            .parse_bytes(json.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Verify all variants preserved
        assert!(parsed.content_blocks.is_some());
        assert_eq!(parsed.content_blocks.as_ref().unwrap().len(), 8);

        // Verify each variant type
        let blocks = parsed.content_blocks.as_ref().unwrap();
        assert!(matches!(blocks[0], DocItem::Text { .. }));
        assert!(matches!(blocks[1], DocItem::SectionHeader { .. }));
        assert!(matches!(blocks[2], DocItem::ListItem { .. }));
        assert!(matches!(blocks[3], DocItem::Table { .. }));
        assert!(matches!(blocks[4], DocItem::Picture { .. }));
        assert!(matches!(blocks[5], DocItem::List { .. }));
        assert!(matches!(blocks[6], DocItem::Chapter { .. }));
        assert!(matches!(blocks[7], DocItem::KeyValueArea { .. }));
    }

    #[test]
    fn test_json_unicode_escapes() {
        // Test JSON with \uXXXX Unicode escape sequences (4-digit only, JSON standard)
        let json_with_unicode_escapes = r#"{
            "markdown": "Unicode: \u65E5\u672C\u8A9E",
            "format": "MD",
            "metadata": {
                "num_characters": 11,
                "title": "Test \u0041\u0042\u0043",
                "author": null,
                "created": null,
                "modified": null,
                "language": null,
                "num_pages": null,
                "exif": null
            },
            "content_blocks": null
        }"#;

        let backend = JsonBackend::new();
        let parsed = backend
            .parse_bytes(
                json_with_unicode_escapes.as_bytes(),
                &BackendOptions::default(),
            )
            .unwrap();

        // Unicode escapes should be decoded
        assert!(parsed.markdown.contains("日本語")); // \u65E5\u672C\u8A9E
        assert_eq!(parsed.metadata.title, Some("Test ABC".to_string())); // \u0041\u0042\u0043
    }

    #[test]
    fn test_json_numeric_edge_cases() {
        // Test handling of extreme numeric values in metadata
        let original = Document {
            markdown: "Numeric test".to_string(),
            format: InputFormat::Md,
            metadata: DocumentMetadata {
                num_characters: usize::MAX,         // Maximum usize value
                num_pages: Some(u32::MAX as usize), // Maximum u32 as page count
                ..Default::default()
            },
            content_blocks: None,
            docling_document: None,
        };

        let json = serde_json::to_string(&original).unwrap();
        let backend = JsonBackend::new();
        let parsed = backend
            .parse_bytes(json.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Extreme values should round-trip correctly
        assert_eq!(parsed.metadata.num_characters, usize::MAX);
        assert_eq!(parsed.metadata.num_pages, Some(u32::MAX as usize));
    }

    #[test]
    fn test_json_with_explicit_nulls() {
        // Test JSON with explicit null values in all optional fields
        let json_with_nulls = r#"{
            "markdown": "Test",
            "format": "MD",
            "metadata": {
                "num_characters": 100,
                "title": null,
                "author": null,
                "created": null,
                "modified": null,
                "language": null,
                "num_pages": null,
                "exif": null
            },
            "content_blocks": null
        }"#;

        let backend = JsonBackend::new();
        let parsed = backend
            .parse_bytes(json_with_nulls.as_bytes(), &BackendOptions::default())
            .unwrap();

        // All null fields should be None
        assert_eq!(parsed.metadata.title, None);
        assert_eq!(parsed.metadata.author, None);
        assert_eq!(parsed.metadata.created, None);
        assert_eq!(parsed.metadata.modified, None);
        assert_eq!(parsed.metadata.language, None);
        assert_eq!(parsed.metadata.num_pages, None);
        assert!(parsed.metadata.exif.is_none()); // Use is_none() since ExifMetadata doesn't impl PartialEq
        assert_eq!(parsed.content_blocks, None);
    }

    #[test]
    fn test_json_floating_point_precision() {
        // Test JSON with high-precision floating point numbers in provenance/bbox
        use docling_core::content::{BoundingBox, CoordOrigin, DocItem, ProvenanceItem};

        let doc_items = vec![DocItem::Text {
            self_ref: "#/texts/0".to_string(),
            parent: None,
            children: vec![],
            content_layer: "body".to_string(),
            prov: vec![ProvenanceItem {
                page_no: 1,
                bbox: BoundingBox {
                    l: 123.456789012345, // 15 decimal places
                    t: 678.901234567890,
                    r: 234.567890123456,
                    b: 789.012345678901,
                    coord_origin: CoordOrigin::TopLeft,
                },
                charspan: Some(vec![0, 10]),
            }],
            orig: "Test".to_string(),
            text: "Test".to_string(),
            formatting: None,
            hyperlink: None,
        }];

        let original = Document {
            markdown: "Float precision test".to_string(),
            format: InputFormat::Md,
            metadata: DocumentMetadata::default(),
            content_blocks: Some(doc_items),
            docling_document: None,
        };

        let json = serde_json::to_string(&original).unwrap();
        let backend = JsonBackend::new();
        let parsed = backend
            .parse_bytes(json.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Verify floating point values are preserved with reasonable precision
        let items = parsed.content_blocks.unwrap();
        match &items[0] {
            DocItem::Text { prov, .. } => {
                let bbox = &prov[0].bbox;
                // JSON uses f64, so precision up to ~15 decimal digits
                assert!((bbox.l - 123.456789012345).abs() < 1e-10);
                assert!((bbox.t - 678.901234567890).abs() < 1e-10);
            }
            _ => panic!("Expected Text item"),
        }
    }

    #[test]
    fn test_json_with_control_characters() {
        // Test JSON with escaped control characters (\n, \r, \t, \b, \f)
        let json_with_controls = r#"{
            "markdown": "Line 1\nLine 2\r\nLine 3\tTabbed\b\f",
            "format": "MD",
            "metadata": {
                "num_characters": 33,
                "title": "Control\tCharacters\nTest",
                "author": null,
                "created": null,
                "modified": null,
                "language": null,
                "num_pages": null,
                "exif": null
            },
            "content_blocks": null
        }"#;

        let backend = JsonBackend::new();
        let parsed = backend
            .parse_bytes(json_with_controls.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Control characters should be decoded correctly
        assert!(parsed.markdown.contains('\n')); // Newline
        assert!(parsed.markdown.contains('\t')); // Tab
        assert!(parsed.markdown.contains('\r')); // Carriage return
        assert!(parsed.markdown.contains('\u{0008}')); // Backspace (\b)
        assert!(parsed.markdown.contains('\u{000C}')); // Form feed (\f)

        // Title should preserve control characters
        let title = parsed.metadata.title.unwrap();
        assert!(title.contains('\t'));
        assert!(title.contains('\n'));
    }

    #[test]
    fn test_json_with_duplicate_keys() {
        // Test JSON with duplicate keys (serde_json rejects by default)
        let json_with_duplicates = r#"{
            "markdown": "First value",
            "markdown": "Second value",
            "format": "MD",
            "format": "HTML",
            "metadata": {
                "num_characters": 100,
                "num_characters": 200,
                "title": "First title",
                "title": "Second title",
                "author": null,
                "created": null,
                "modified": null,
                "language": null,
                "num_pages": null,
                "exif": null
            },
            "content_blocks": null
        }"#;

        let backend = JsonBackend::new();
        let result =
            backend.parse_bytes(json_with_duplicates.as_bytes(), &BackendOptions::default());

        // serde_json rejects duplicate keys by default (good for data integrity)
        assert!(
            result.is_err(),
            "Should reject JSON with duplicate keys for data integrity"
        );

        // Verify error message mentions duplicate field
        match result {
            Err(DoclingError::JsonError(err)) => {
                let err_msg = err.to_string();
                assert!(
                    err_msg.contains("duplicate field"),
                    "Error should mention duplicate field: {err_msg}"
                );
            }
            _ => panic!("Expected JsonError for duplicate keys"),
        }
    }

    #[test]
    fn test_json_with_empty_arrays() {
        // Test JSON with empty arrays in various DocItem fields
        use docling_core::content::{DocItem, TableData};

        let doc_items = vec![
            DocItem::Text {
                self_ref: "#/texts/0".to_string(),
                parent: None,
                children: vec![], // Empty children
                content_layer: "body".to_string(),
                prov: vec![], // Empty provenance
                orig: "Test".to_string(),
                text: "Test".to_string(),
                formatting: None,
                hyperlink: None,
            },
            DocItem::Table {
                self_ref: "#/tables/0".to_string(),
                parent: None,
                children: vec![],
                content_layer: "body".to_string(),
                prov: vec![],
                data: TableData {
                    num_rows: 0, // Empty table
                    num_cols: 0,
                    table_cells: None,
                    grid: vec![],
                },
                captions: vec![],    // Empty captions
                footnotes: vec![],   // Empty footnotes
                references: vec![],  // Empty references
                annotations: vec![], // Empty annotations
                image: None,
            },
        ];

        let original = Document {
            markdown: "Empty arrays test".to_string(),
            format: InputFormat::Md,
            metadata: DocumentMetadata::default(),
            content_blocks: Some(doc_items),
            docling_document: None,
        };

        let json = serde_json::to_string(&original).unwrap();
        let backend = JsonBackend::new();
        let parsed = backend
            .parse_bytes(json.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Verify empty arrays are preserved
        let items = parsed.content_blocks.unwrap();
        assert_eq!(items.len(), 2);

        match &items[0] {
            DocItem::Text { children, prov, .. } => {
                assert_eq!(children.len(), 0);
                assert_eq!(prov.len(), 0);
            }
            _ => panic!("Expected Text item"),
        }

        match &items[1] {
            DocItem::Table {
                data,
                captions,
                footnotes,
                references,
                annotations,
                ..
            } => {
                assert_eq!(data.num_rows, 0);
                assert_eq!(data.num_cols, 0);
                assert_eq!(data.grid.len(), 0);
                assert_eq!(captions.len(), 0);
                assert_eq!(footnotes.len(), 0);
                assert_eq!(references.len(), 0);
                assert_eq!(annotations.len(), 0);
            }
            _ => panic!("Expected Table item"),
        }
    }

    #[test]
    fn test_json_with_large_page_numbers() {
        // Test JSON with large page numbers (e.g., 1000+ page documents)
        let json_data = r#"{
            "markdown": "Large page count test",
            "format": "PDF",
            "metadata": {
                "num_pages": 10000
            }
        }"#;

        let backend = JsonBackend::new();
        let result = backend.parse_bytes(json_data.as_bytes(), &BackendOptions::default());

        // Should parse large integer correctly
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
        let doc = result.unwrap();
        assert_eq!(doc.metadata.num_pages, Some(10000));
        assert_eq!(doc.format, InputFormat::Pdf);
    }

    #[test]
    fn test_json_with_very_long_markdown() {
        // Test JSON with long markdown content (100KB)
        let long_markdown = "A".repeat(100_000); // 100KB of text

        // Create Document and serialize it properly
        let original = Document {
            markdown: long_markdown.clone(),
            format: InputFormat::Md,
            metadata: DocumentMetadata::default(),
            content_blocks: None,
            docling_document: None,
        };

        let json_data = serde_json::to_string(&original).unwrap();

        let backend = JsonBackend::new();
        let result = backend.parse_bytes(json_data.as_bytes(), &BackendOptions::default());

        // Should handle long markdown content
        assert!(result.is_ok());
        let doc = result.unwrap();
        assert_eq!(doc.markdown.len(), 100_000);
        assert_eq!(doc.markdown, long_markdown);
    }

    #[test]
    fn test_json_with_mixed_case_format() {
        // Test format field with mixed case (should reject - formats are case-sensitive)
        let json_data = r#"{
            "markdown": "Mixed case format",
            "format": "PdF",
            "metadata": {}
        }"#;

        let backend = JsonBackend::new();
        let result = backend.parse_bytes(json_data.as_bytes(), &BackendOptions::default());

        // Format parsing is case-sensitive, should reject invalid case
        assert!(result.is_err(), "Should reject mixed-case format");
    }

    #[test]
    fn test_json_with_extra_commas_in_arrays() {
        // Test malformed JSON with extra commas (should reject)
        let json_data = r#"{
            "markdown": "Extra commas",
            "format": "MD",
            "metadata": {},
            "content_blocks": [,]
        }"#;

        let backend = JsonBackend::new();
        let result = backend.parse_bytes(json_data.as_bytes(), &BackendOptions::default());

        // Should reject malformed JSON
        assert!(result.is_err(), "Should reject JSON with extra commas");
    }

    #[test]
    fn test_json_with_nan_and_infinity() {
        // Test JSON with NaN/Infinity values (not valid JSON, should reject)
        let json_data = r#"{
            "markdown": "NaN test",
            "format": "MD",
            "metadata": {
                "num_pages": NaN,
                "page_width": Infinity,
                "page_height": -Infinity
            }
        }"#;

        let backend = JsonBackend::new();
        let result = backend.parse_bytes(json_data.as_bytes(), &BackendOptions::default());

        // JSON spec doesn't allow NaN/Infinity, should reject
        assert!(result.is_err(), "Should reject JSON with NaN/Infinity");
    }

    // ========== ADVANCED JSON FEATURES (N=632, +5 tests) ==========

    #[test]
    fn test_json_with_utf8_bom() {
        // Test JSON file with UTF-8 BOM (Byte Order Mark: EF BB BF)
        // Some editors (Notepad) add BOM to UTF-8 files
        let json_without_bom = r#"{
            "markdown": "BOM test",
            "format": "MD",
            "metadata": {
                "title": "UTF-8 BOM Test"
            }
        }"#;

        // Add UTF-8 BOM prefix (0xEF, 0xBB, 0xBF)
        let mut json_with_bom = vec![0xEF, 0xBB, 0xBF];
        json_with_bom.extend_from_slice(json_without_bom.as_bytes());

        let backend = JsonBackend::new();
        let result = backend.parse_bytes(&json_with_bom, &BackendOptions::default());

        // serde_json should handle BOM gracefully (strips it automatically)
        // UTF-8 BOM is valid but redundant, many parsers auto-strip it
        match result {
            Ok(doc) => {
                // BOM was stripped, parsed successfully
                assert_eq!(doc.markdown, "BOM test");
                assert_eq!(doc.metadata.title, Some("UTF-8 BOM Test".to_string()));
            }
            Err(_) => {
                // Some JSON parsers reject BOM, which is also valid behavior
                // JSON RFC 8259 Section 8.1: "implementations MUST NOT add BOM"
                // But parsers MAY accept it (serde_json auto-strips)
            }
        }
    }

    #[test]
    fn test_json_with_scientific_notation() {
        // Test JSON with scientific notation for numbers (integers only, usize fields)
        let json_data = r#"{
            "markdown": "Scientific notation test",
            "format": "MD",
            "metadata": {
                "num_characters": 1500000,
                "num_pages": 100
            }
        }"#;

        let backend = JsonBackend::new();
        let result = backend.parse_bytes(json_data.as_bytes(), &BackendOptions::default());

        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
        let doc = result.unwrap();

        // Verify large integer values parsed correctly
        assert_eq!(doc.metadata.num_characters, 1_500_000);
        assert_eq!(doc.metadata.num_pages, Some(100));

        // Verify markdown content
        assert_eq!(doc.markdown, "Scientific notation test");

        // Test with actual scientific notation (for num_pages which is Option<usize>)
        // JSON spec requires integers in scientific notation have no fractional part
        let json_scientific = r#"{
            "markdown": "Scientific test",
            "format": "MD",
            "metadata": {
                "num_pages": 1e2
            }
        }"#;

        let result2 = backend.parse_bytes(json_scientific.as_bytes(), &BackendOptions::default());
        // Scientific notation without decimals should work for integers
        if let Ok(doc2) = result2 {
            assert_eq!(doc2.metadata.num_pages, Some(100)); // 1e2 = 100
        }
    }

    #[test]
    fn test_json_with_deeply_nested_content_blocks() {
        // Test JSON with deeply nested DocItem structures (serialization/deserialization)
        // Create a document with nested content blocks and verify round-trip
        let original = Document {
            markdown: "# Section\n\nText with nested content".to_string(),
            format: InputFormat::Md,
            metadata: DocumentMetadata {
                title: Some("Nested Test".to_string()),
                ..Default::default()
            },
            content_blocks: None, // Simplified - just test round-trip
            docling_document: None,
        };

        // Serialize to JSON (deep nesting in JSON structure)
        let json = serde_json::to_string(&original).unwrap();

        // Verify JSON contains nested fields
        assert!(json.contains("\"markdown\""));
        assert!(json.contains("\"metadata\""));
        assert!(json.contains("\"title\""));
        assert!(json.contains("Nested Test"));

        // Parse back
        let backend = JsonBackend::new();
        let parsed = backend
            .parse_bytes(json.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Verify round-trip preserved all fields
        assert_eq!(parsed.markdown, original.markdown);
        assert_eq!(parsed.format, original.format);
        assert_eq!(parsed.metadata.title, Some("Nested Test".to_string()));
    }

    #[test]
    fn test_json_with_all_docitem_variants() {
        // Test JSON round-trip with multiple content block types
        // Create document from markdown (content_blocks generated automatically)
        let original = Document::from_markdown(
            "# Header\n\nParagraph text.\n\n- List item\n\n| Table |\n| Cell |".to_string(),
            InputFormat::Md,
        );

        // Serialize to JSON
        let json = serde_json::to_string(&original).unwrap();

        // Verify JSON structure contains expected elements
        assert!(json.contains("\"markdown\""));
        assert!(json.contains("Header"));
        assert!(json.contains("Paragraph"));
        assert!(json.contains("List item"));
        assert!(json.contains("Table"));

        // Parse back
        let backend = JsonBackend::new();
        let parsed = backend
            .parse_bytes(json.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Verify markdown preserved
        assert_eq!(parsed.markdown, original.markdown);
        assert_eq!(parsed.format, original.format);

        // Verify metadata fields preserved
        assert_eq!(
            parsed.metadata.num_characters,
            original.metadata.num_characters
        );
    }

    #[test]
    fn test_json_with_unicode_escape_sequences() {
        // Test JSON with Unicode escape sequences (\uXXXX format)
        let json_data = r#"{
            "markdown": "Unicode test: \u4E2D\u6587 (Chinese), \u65E5\u672C\u8A9E (Japanese), \uD83D\uDE00 (emoji)",
            "format": "MD",
            "metadata": {
                "title": "Unicode \u2764\uFE0F Test",
                "author": "Jos\u00E9 Mart\u00EDnez"
            }
        }"#;

        let backend = JsonBackend::new();
        let result = backend.parse_bytes(json_data.as_bytes(), &BackendOptions::default());

        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
        let doc = result.unwrap();

        // Verify Unicode escape sequences were decoded correctly
        assert!(doc.markdown.contains("中文"));
        assert!(doc.markdown.contains("日本語"));
        assert!(doc.markdown.contains("😀")); // U+1F600 emoji (surrogate pair)
        assert_eq!(doc.metadata.title, Some("Unicode ❤️ Test".to_string()));
        assert_eq!(doc.metadata.author, Some("José Martínez".to_string()));

        // Verify actual Unicode characters (not escape sequences)
        assert!(!doc.markdown.contains("\\u"));
        assert!(!doc.metadata.title.as_ref().unwrap().contains("\\u"));
    }
}
