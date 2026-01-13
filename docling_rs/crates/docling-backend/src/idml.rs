//! IDML backend for docling
//!
//! This backend converts Adobe `InDesign` IDML files to docling's document model.

use crate::traits::{BackendOptions, DocumentBackend};
use crate::utils::create_text_item;
use docling_adobe::{IdmlParser, IdmlSerializer};
use docling_core::{
    content::{BoundingBox, CoordOrigin, DocItem, ProvenanceItem},
    DoclingError, Document, DocumentMetadata, InputFormat,
};
use std::path::Path;

/// IDML backend
///
/// Converts IDML (`InDesign` Markup Language) files to docling's document model.
/// IDML is Adobe `InDesign`'s interchange format based on XML.
///
/// ## Features
///
/// - Extract stories (text content) from IDML packages
/// - Extract metadata (title, author, creation date)
/// - Convert to structured markdown
/// - Preserve paragraph structure
///
/// ## Example
///
/// ```no_run
/// use docling_backend::IdmlBackend;
/// use docling_backend::DocumentBackend;
///
/// let backend = IdmlBackend::new();
/// let result = backend.parse_file("document.idml", &Default::default())?;
/// println!("Document: {:?}", result.metadata.title);
/// # Ok::<(), docling_core::error::DoclingError>(())
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct IdmlBackend;

impl IdmlBackend {
    /// Create a new IDML backend instance
    #[inline]
    #[must_use = "creates a backend instance that should be used for parsing"]
    pub const fn new() -> Self {
        Self
    }

    /// Convert IDML document to `DocItems`
    fn idml_to_doc_items(idml: &docling_adobe::idml::types::IdmlDocument) -> Vec<DocItem> {
        let mut doc_items = Vec::new();
        let mut item_id = 0;

        // Process each story (text flow)
        for story in &idml.stories {
            for paragraph in &story.paragraphs {
                let text_content = paragraph.text.trim();
                if text_content.is_empty() {
                    continue;
                }

                // Check if paragraph has heading style (Heading1, Heading2, etc.)
                if let Some(style) = &paragraph.style {
                    if style.starts_with("Heading") {
                        // Extract heading level (Heading1 -> level 1, Heading2 -> level 2, etc.)
                        let level = style
                            .trim_start_matches("Heading")
                            .parse::<usize>()
                            .unwrap_or(1);

                        // Clamp to valid markdown heading levels (1-6)
                        let level = level.clamp(1, 6);

                        let self_ref = format!("#/texts/{item_id}");
                        let item = DocItem::SectionHeader {
                            self_ref,
                            parent: None,
                            children: vec![],
                            content_layer: "body".to_string(),
                            prov: vec![],
                            orig: text_content.to_string(),
                            text: text_content.to_string(),
                            level,
                            formatting: None,
                            hyperlink: None,
                        };
                        doc_items.push(item);
                        item_id += 1;
                        continue;
                    }
                }

                // Regular paragraph -> Text DocItem
                // IDML doesn't provide position info, so we use default bounding box
                let bbox = BoundingBox {
                    l: 0.0,
                    t: 0.0,
                    r: 1.0,
                    b: 1.0,
                    coord_origin: CoordOrigin::TopLeft,
                };

                let prov = ProvenanceItem {
                    page_no: 1, // IDML doesn't have page numbers (flows across pages)
                    bbox,
                    charspan: Some(vec![0, text_content.len()]),
                };

                doc_items.push(create_text_item(
                    item_id,
                    text_content.to_string(),
                    vec![prov],
                ));
                item_id += 1;
            }
        }

        doc_items
    }
}

impl DocumentBackend for IdmlBackend {
    #[inline]
    fn format(&self) -> InputFormat {
        InputFormat::Idml
    }

    fn parse_file<P: AsRef<Path>>(
        &self,
        path: P,
        _options: &BackendOptions,
    ) -> Result<Document, DoclingError> {
        let path_ref = path.as_ref();
        let filename = path_ref.display().to_string();

        // Parse IDML file
        let idml_doc = IdmlParser::parse_file(path_ref).map_err(|e| {
            DoclingError::BackendError(format!("Failed to parse IDML: {e}: {filename}"))
        })?;

        // Generate DocItems
        let doc_items = Self::idml_to_doc_items(&idml_doc);

        // Convert to markdown
        let markdown = IdmlSerializer::to_markdown(&idml_doc);
        let num_characters = markdown.chars().count();

        // Extract metadata
        let title = idml_doc.metadata.title.clone();
        let author = idml_doc.metadata.author;

        Ok(Document {
            markdown,
            format: InputFormat::Idml,
            metadata: DocumentMetadata {
                num_pages: None,
                num_characters,
                title,
                author,
                created: None,
                modified: None,
                language: None,
                subject: None,
                exif: None,
            },
            content_blocks: Some(doc_items),
            docling_document: None,
        })
    }

    fn parse_bytes(
        &self,
        _data: &[u8],
        _options: &BackendOptions,
    ) -> Result<Document, DoclingError> {
        Err(DoclingError::BackendError(
            "IDML format does not support parsing from bytes (requires ZIP package)".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use docling_adobe::idml::types::{IdmlDocument, Metadata, Paragraph, Story};

    #[test]
    fn test_idml_backend_creation() {
        let backend = IdmlBackend::new();
        assert_eq!(
            backend.format(),
            InputFormat::Idml,
            "IdmlBackend should report Idml format"
        );
    }

    #[test]
    fn test_idml_parse_bytes_not_supported() {
        let backend = IdmlBackend::new();
        let data = b"test data";
        let result = backend.parse_bytes(data, &BackendOptions::default());
        assert!(
            result.is_err(),
            "parse_bytes should return error for IDML (requires ZIP)"
        );
        if let Err(DoclingError::BackendError(msg)) = result {
            assert!(
                msg.contains("does not support parsing from bytes"),
                "Error message should mention byte parsing is not supported"
            );
        } else {
            panic!("Expected BackendError");
        }
    }

    // ========================================
    // Metadata Tests
    // ========================================

    #[test]
    fn test_idml_metadata_complete() {
        let metadata = Metadata {
            title: Some("Complete IDML Document".to_string()),
            author: Some("John Doe".to_string()),
        };

        let doc = IdmlDocument::with_metadata(metadata);
        let markdown = IdmlSerializer::to_markdown(&doc);

        // Should have YAML frontmatter
        assert!(
            markdown.starts_with("---\n"),
            "Frontmatter should start with YAML delimiter"
        );
        assert!(
            markdown.contains("title: Complete IDML Document"),
            "Frontmatter should contain document title"
        );
        assert!(
            markdown.contains("author: John Doe"),
            "Frontmatter should contain author name"
        );
        // Note: When document has no content, trim_end() removes trailing \n\n,
        // so second --- may not have preceding newline
        assert!(
            markdown.contains("---") && markdown.matches("---").count() == 2,
            "Frontmatter should have exactly two delimiters"
        );
    }

    #[test]
    fn test_idml_metadata_title_only() {
        let metadata = Metadata {
            title: Some("Title Only".to_string()),
            author: None,
        };

        let doc = IdmlDocument::with_metadata(metadata);
        let markdown = IdmlSerializer::to_markdown(&doc);

        // Should have frontmatter with title only
        assert!(
            markdown.contains("---"),
            "Frontmatter should have YAML delimiter"
        );
        assert!(
            markdown.contains("title: Title Only"),
            "Frontmatter should contain title"
        );
        assert!(
            !markdown.contains("author:"),
            "Frontmatter should not contain author when None"
        );
    }

    #[test]
    fn test_idml_metadata_empty() {
        let doc = IdmlDocument::new();
        let markdown = IdmlSerializer::to_markdown(&doc);

        // Should not have frontmatter delimiters
        assert!(
            !markdown.contains("---"),
            "Empty metadata should not generate frontmatter delimiter"
        );
        assert!(
            !markdown.contains("title:"),
            "Empty metadata should not contain title field"
        );
        assert!(
            !markdown.contains("author:"),
            "Empty metadata should not contain author field"
        );
    }

    // ========================================
    // DocItem Generation Tests
    // ========================================

    #[test]
    fn test_idml_single_story_markdown() {
        let mut doc = IdmlDocument::new();
        let mut story = Story::new("u1000".to_string());

        story.add_paragraph(Paragraph::new("First paragraph.".to_string()));
        story.add_paragraph(Paragraph::new("Second paragraph.".to_string()));
        story.add_paragraph(Paragraph::new("Third paragraph.".to_string()));

        doc.add_story(story);

        let markdown = IdmlSerializer::to_markdown(&doc);

        // Should contain all paragraphs separated by double newlines
        assert!(
            markdown.contains("First paragraph."),
            "First paragraph should be present"
        );
        assert!(
            markdown.contains("Second paragraph."),
            "Second paragraph should be present"
        );
        assert!(
            markdown.contains("Third paragraph."),
            "Third paragraph should be present"
        );

        // Count paragraph breaks (should be 3 paragraphs)
        let parts: Vec<&str> = markdown.split("\n\n").collect();
        assert_eq!(
            parts.len(),
            3,
            "Should have 3 paragraphs separated by double newlines"
        );
    }

    #[test]
    fn test_idml_multi_story_markdown() {
        let mut doc = IdmlDocument::new();

        // Story 1
        let mut story1 = Story::new("u1000".to_string());
        story1.add_paragraph(Paragraph::new("Story 1, Paragraph 1.".to_string()));
        story1.add_paragraph(Paragraph::new("Story 1, Paragraph 2.".to_string()));
        doc.add_story(story1);

        // Story 2
        let mut story2 = Story::new("u2000".to_string());
        story2.add_paragraph(Paragraph::new("Story 2, Paragraph 1.".to_string()));
        story2.add_paragraph(Paragraph::new("Story 2, Paragraph 2.".to_string()));
        doc.add_story(story2);

        let markdown = IdmlSerializer::to_markdown(&doc);

        // Should contain content from both stories
        assert!(
            markdown.contains("Story 1, Paragraph 1."),
            "Story 1 first paragraph should be present"
        );
        assert!(
            markdown.contains("Story 1, Paragraph 2."),
            "Story 1 second paragraph should be present"
        );
        assert!(
            markdown.contains("Story 2, Paragraph 1."),
            "Story 2 first paragraph should be present"
        );
        assert!(
            markdown.contains("Story 2, Paragraph 2."),
            "Story 2 second paragraph should be present"
        );

        // All paragraphs should be present
        let parts: Vec<&str> = markdown.split("\n\n").collect();
        assert_eq!(parts.len(), 4, "Should have 4 paragraphs from both stories");
    }

    #[test]
    fn test_idml_empty_document() {
        let doc = IdmlDocument::new();
        let markdown = IdmlSerializer::to_markdown(&doc);

        // Empty document should produce empty string
        assert!(
            markdown.is_empty(),
            "Empty IDML document should produce empty markdown"
        );
    }

    // ========================================
    // Format-Specific Feature Tests
    // ========================================

    #[test]
    fn test_idml_heading_styles() {
        let mut doc = IdmlDocument::new();
        let mut story = Story::new("u1000".to_string());

        story.add_paragraph(Paragraph::with_style(
            "Heading1".to_string(),
            "Main Heading".to_string(),
        ));
        story.add_paragraph(Paragraph::with_style(
            "Heading2".to_string(),
            "Subheading".to_string(),
        ));
        story.add_paragraph(Paragraph::with_style(
            "Heading3".to_string(),
            "Sub-subheading".to_string(),
        ));
        story.add_paragraph(Paragraph::new("Body text".to_string()));

        doc.add_story(story);

        let markdown = IdmlSerializer::to_markdown(&doc);

        // Heading styles should be converted to markdown headers
        assert!(
            markdown.contains("# Main Heading"),
            "Heading1 should convert to H1 markdown"
        );
        assert!(
            markdown.contains("## Subheading"),
            "Heading2 should convert to H2 markdown"
        );
        assert!(
            markdown.contains("### Sub-subheading"),
            "Heading3 should convert to H3 markdown"
        );
        assert!(
            markdown.contains("Body text"),
            "Body text should be present"
        );
    }

    #[test]
    fn test_idml_all_heading_levels() {
        let mut doc = IdmlDocument::new();
        let mut story = Story::new("u1000".to_string());

        story.add_paragraph(Paragraph::with_style(
            "Heading1".to_string(),
            "H1".to_string(),
        ));
        story.add_paragraph(Paragraph::with_style(
            "Heading2".to_string(),
            "H2".to_string(),
        ));
        story.add_paragraph(Paragraph::with_style(
            "Heading3".to_string(),
            "H3".to_string(),
        ));
        story.add_paragraph(Paragraph::with_style(
            "Heading4".to_string(),
            "H4".to_string(),
        ));
        story.add_paragraph(Paragraph::with_style(
            "Heading5".to_string(),
            "H5".to_string(),
        ));
        story.add_paragraph(Paragraph::with_style(
            "Heading6".to_string(),
            "H6".to_string(),
        ));

        doc.add_story(story);

        let markdown = IdmlSerializer::to_markdown(&doc);

        // All 6 heading levels should be preserved
        assert!(
            markdown.contains("# H1"),
            "Heading1 should convert to level 1 header"
        );
        assert!(
            markdown.contains("## H2"),
            "Heading2 should convert to level 2 header"
        );
        assert!(
            markdown.contains("### H3"),
            "Heading3 should convert to level 3 header"
        );
        assert!(
            markdown.contains("#### H4"),
            "Heading4 should convert to level 4 header"
        );
        assert!(
            markdown.contains("##### H5"),
            "Heading5 should convert to level 5 header"
        );
        assert!(
            markdown.contains("###### H6"),
            "Heading6 should convert to level 6 header"
        );
    }

    #[test]
    fn test_idml_story_ids() {
        let mut doc = IdmlDocument::new();

        let story1 = Story::new("u1000".to_string());
        let story2 = Story::new("u2000".to_string());
        let story3 = Story::new("u3000".to_string());

        // Story IDs follow InDesign's convention (u + number)
        assert_eq!(story1.id, "u1000", "Story 1 should have ID u1000");
        assert_eq!(story2.id, "u2000", "Story 2 should have ID u2000");
        assert_eq!(story3.id, "u3000", "Story 3 should have ID u3000");

        doc.add_story(story1);
        doc.add_story(story2);
        doc.add_story(story3);

        assert_eq!(doc.stories.len(), 3, "Document should contain 3 stories");
    }

    #[test]
    fn test_idml_paragraph_style_variants() {
        let mut doc = IdmlDocument::new();
        let mut story = Story::new("u1000".to_string());

        // Test unstyled paragraph
        story.add_paragraph(Paragraph::new("Unstyled".to_string()));

        // Test styled paragraphs
        story.add_paragraph(Paragraph::with_style(
            "BodyText".to_string(),
            "Body".to_string(),
        ));
        story.add_paragraph(Paragraph::with_style(
            "Quote".to_string(),
            "Quote".to_string(),
        ));

        doc.add_story(story);

        let markdown = IdmlSerializer::to_markdown(&doc);

        // Unstyled and unknown styles should appear as plain text
        assert!(
            markdown.contains("Unstyled"),
            "Unstyled paragraph should be present"
        );
        assert!(
            markdown.contains("Body"),
            "BodyText styled paragraph should be present"
        );
        assert!(
            markdown.contains("Quote"),
            "Quote styled paragraph should be present"
        );

        // Should NOT have markdown headers for non-heading styles
        let lines: Vec<&str> = markdown.lines().collect();
        let body_line = lines.iter().find(|&&l| l.contains("Body")).unwrap();
        assert!(
            !body_line.starts_with("#"),
            "Non-heading style should not produce markdown header"
        );
    }

    // ========================================
    // Edge Case Tests
    // ========================================

    #[test]
    fn test_idml_trailing_whitespace_trimmed() {
        let mut doc = IdmlDocument::new();
        let mut story = Story::new("u1000".to_string());

        story.add_paragraph(Paragraph::new("Content".to_string()));

        doc.add_story(story);

        let markdown = IdmlSerializer::to_markdown(&doc);

        // Markdown should not end with newlines (per line 44 in serializer.rs)
        assert!(
            !markdown.ends_with('\n'),
            "Markdown should not have trailing newline"
        );
        assert_eq!(
            markdown, "Content",
            "Single paragraph should produce just the text"
        );
    }

    #[test]
    fn test_idml_empty_story() {
        let mut doc = IdmlDocument::new();
        let story = Story::new("u1000".to_string());

        // Story with no paragraphs
        assert!(
            story.paragraphs.is_empty(),
            "Newly created story should have no paragraphs"
        );

        doc.add_story(story);

        let markdown = IdmlSerializer::to_markdown(&doc);

        // Empty story should produce empty output
        assert!(
            markdown.is_empty(),
            "Empty story should produce empty markdown"
        );
    }

    #[test]
    fn test_idml_mixed_content_and_metadata() {
        let metadata = Metadata {
            title: Some("Test Document".to_string()),
            author: Some("Test Author".to_string()),
        };

        let mut doc = IdmlDocument::with_metadata(metadata);
        let mut story = Story::new("u1000".to_string());

        story.add_paragraph(Paragraph::with_style(
            "Heading1".to_string(),
            "Introduction".to_string(),
        ));
        story.add_paragraph(Paragraph::new("This is the body content.".to_string()));

        doc.add_story(story);

        let markdown = IdmlSerializer::to_markdown(&doc);

        // Should have both frontmatter and content
        assert!(
            markdown.starts_with("---\n"),
            "Mixed content should start with frontmatter"
        );
        assert!(
            markdown.contains("title: Test Document"),
            "Frontmatter should contain title"
        );
        assert!(
            markdown.contains("# Introduction"),
            "Content should have heading"
        );
        assert!(
            markdown.contains("This is the body content."),
            "Content should have body text"
        );

        // Frontmatter should come before content
        let frontmatter_end = markdown.find("\n---\n").unwrap();
        let content_start = markdown.find("# Introduction").unwrap();
        assert!(
            frontmatter_end < content_start,
            "Frontmatter should precede content"
        );
    }

    // ===== Backend Trait Tests =====

    /// Test IdmlBackend implements Default
    #[test]
    fn test_backend_default() {
        let backend = IdmlBackend;
        assert_eq!(
            backend.format(),
            InputFormat::Idml,
            "Default backend should report Idml format"
        );
    }

    /// Test format() consistency
    #[test]
    fn test_backend_format_constant() {
        let backend1 = IdmlBackend::new();
        let backend2 = IdmlBackend;
        assert_eq!(
            backend1.format(),
            backend2.format(),
            "new() and default should produce same format"
        );
        assert_eq!(
            backend1.format(),
            InputFormat::Idml,
            "Format should always be Idml"
        );
    }

    // ===== Metadata Edge Cases =====

    /// Test metadata with author only (no title)
    #[test]
    fn test_metadata_author_only() {
        let metadata = Metadata {
            title: None,
            author: Some("Author Name".to_string()),
        };

        let doc = IdmlDocument::with_metadata(metadata);
        let markdown = IdmlSerializer::to_markdown(&doc);

        // Should have frontmatter with author only
        assert!(
            markdown.contains("---"),
            "Frontmatter delimiter should be present"
        );
        assert!(
            markdown.contains("author: Author Name"),
            "Author should be in frontmatter"
        );
        assert!(
            !markdown.contains("title:"),
            "Title should not be present when None"
        );
    }

    /// Test metadata with title containing special characters
    #[test]
    fn test_metadata_title_special_characters() {
        let metadata = Metadata {
            title: Some("Title: With Special & Characters!".to_string()),
            author: None,
        };

        let doc = IdmlDocument::with_metadata(metadata);
        let markdown = IdmlSerializer::to_markdown(&doc);

        // Special characters should be preserved (YAML can handle them)
        assert!(
            markdown.contains("title: Title: With Special & Characters!"),
            "Special characters in title should be preserved"
        );
    }

    /// Test metadata with very long fields
    #[test]
    fn test_metadata_very_long_fields() {
        let long_title = "A".repeat(500);
        let metadata = Metadata {
            title: Some(long_title.clone()),
            author: Some("Author".to_string()),
        };

        let doc = IdmlDocument::with_metadata(metadata);
        let markdown = IdmlSerializer::to_markdown(&doc);

        // Long title should be preserved
        assert!(
            markdown.contains(&format!("title: {long_title}")),
            "Very long title should be preserved in frontmatter"
        );
    }

    // ===== Story Structure Edge Cases =====

    /// Test story with single paragraph
    #[test]
    fn test_story_single_paragraph() {
        let mut doc = IdmlDocument::new();
        let mut story = Story::new("u1000".to_string());

        story.add_paragraph(Paragraph::new("Single paragraph.".to_string()));
        doc.add_story(story);

        let markdown = IdmlSerializer::to_markdown(&doc);

        // Should produce just the paragraph text (no trailing newlines per trim_end())
        assert_eq!(
            markdown, "Single paragraph.",
            "Single paragraph story should produce just the text"
        );
    }

    /// Test paragraph with empty content
    #[test]
    fn test_paragraph_empty_content() {
        let mut doc = IdmlDocument::new();
        let mut story = Story::new("u1000".to_string());

        story.add_paragraph(Paragraph::new("Before".to_string()));
        story.add_paragraph(Paragraph::new("".to_string())); // Empty
        story.add_paragraph(Paragraph::new("After".to_string()));

        doc.add_story(story);

        let markdown = IdmlSerializer::to_markdown(&doc);

        // Empty paragraph should still create a paragraph break
        assert!(
            markdown.contains("Before"),
            "Content before empty paragraph should be present"
        );
        assert!(
            markdown.contains("After"),
            "Content after empty paragraph should be present"
        );
    }

    /// Test paragraph order preservation
    #[test]
    fn test_paragraph_order_preservation() {
        let mut doc = IdmlDocument::new();
        let mut story = Story::new("u1000".to_string());

        story.add_paragraph(Paragraph::new("First".to_string()));
        story.add_paragraph(Paragraph::new("Second".to_string()));
        story.add_paragraph(Paragraph::new("Third".to_string()));
        story.add_paragraph(Paragraph::new("Fourth".to_string()));

        doc.add_story(story);

        let markdown = IdmlSerializer::to_markdown(&doc);

        // Verify order is preserved
        let first_pos = markdown.find("First").unwrap();
        let second_pos = markdown.find("Second").unwrap();
        let third_pos = markdown.find("Third").unwrap();
        let fourth_pos = markdown.find("Fourth").unwrap();

        assert!(first_pos < second_pos, "First should come before Second");
        assert!(second_pos < third_pos, "Second should come before Third");
        assert!(third_pos < fourth_pos, "Third should come before Fourth");
    }

    // ===== Heading Edge Cases =====

    /// Test heading style with number beyond 6 (edge case)
    #[test]
    fn test_heading_overflow() {
        let mut doc = IdmlDocument::new();
        let mut story = Story::new("u1000".to_string());

        // Heading7 is not valid markdown (max is ###### for H6)
        story.add_paragraph(Paragraph::with_style(
            "Heading7".to_string(),
            "H7 becomes plain text".to_string(),
        ));

        doc.add_story(story);

        let markdown = IdmlSerializer::to_markdown(&doc);

        // Heading7 should be treated as plain text (no markdown header)
        assert!(
            markdown.contains("H7 becomes plain text"),
            "Heading7 text should be present"
        );
        assert!(
            !markdown.contains("#######"),
            "7 hashes not valid markdown so should not appear"
        );
    }

    /// Test heading style with mixed case
    #[test]
    fn test_heading_mixed_case_style() {
        let mut doc = IdmlDocument::new();
        let mut story = Story::new("u1000".to_string());

        // Test if heading style matching is case-sensitive
        story.add_paragraph(Paragraph::with_style(
            "heading1".to_string(), // lowercase
            "Lowercase style".to_string(),
        ));
        story.add_paragraph(Paragraph::with_style(
            "HEADING2".to_string(), // uppercase
            "Uppercase style".to_string(),
        ));

        doc.add_story(story);

        let markdown = IdmlSerializer::to_markdown(&doc);

        // Depends on implementation: may or may not match case-insensitively
        // Just verify they appear (plain text fallback is acceptable)
        assert!(
            markdown.contains("Lowercase style"),
            "Lowercase heading style text should be present"
        );
        assert!(
            markdown.contains("Uppercase style"),
            "Uppercase heading style text should be present"
        );
    }

    /// Test heading with empty text
    #[test]
    fn test_heading_empty_text() {
        let mut doc = IdmlDocument::new();
        let mut story = Story::new("u1000".to_string());

        story.add_paragraph(Paragraph::with_style(
            "Heading1".to_string(),
            "".to_string(),
        ));

        doc.add_story(story);

        let markdown = IdmlSerializer::to_markdown(&doc);

        // Empty heading should produce "#" or "" depending on implementation
        // Just verify it doesn't crash
        let _ = markdown;
    }

    // ===== Paragraph Content Edge Cases =====

    /// Test paragraph with unicode text (CJK, emoji)
    #[test]
    fn test_paragraph_unicode() {
        let mut doc = IdmlDocument::new();
        let mut story = Story::new("u1000".to_string());

        story.add_paragraph(Paragraph::new("日本語テキスト".to_string())); // Japanese
        story.add_paragraph(Paragraph::new("中文文本".to_string())); // Chinese
        story.add_paragraph(Paragraph::new("한국어 텍스트".to_string())); // Korean
        story.add_paragraph(Paragraph::new("Emoji: 😀 🎉 🚀".to_string()));

        doc.add_story(story);

        let markdown = IdmlSerializer::to_markdown(&doc);

        assert!(
            markdown.contains("日本語テキスト"),
            "Japanese text should be preserved"
        );
        assert!(
            markdown.contains("中文文本"),
            "Chinese text should be preserved"
        );
        assert!(
            markdown.contains("한국어 텍스트"),
            "Korean text should be preserved"
        );
        assert!(
            markdown.contains("Emoji: 😀 🎉 🚀"),
            "Emoji characters should be preserved"
        );
    }

    /// Test paragraph with special characters
    #[test]
    fn test_paragraph_special_characters() {
        let mut doc = IdmlDocument::new();
        let mut story = Story::new("u1000".to_string());

        story.add_paragraph(Paragraph::new("Special: @#$%^&*()".to_string()));
        story.add_paragraph(Paragraph::new(
            "Quotes: \"double\" and 'single'".to_string(),
        ));
        story.add_paragraph(Paragraph::new("Symbols: © ® ™ € £".to_string()));

        doc.add_story(story);

        let markdown = IdmlSerializer::to_markdown(&doc);

        assert!(
            markdown.contains("Special: @#$%^&*()"),
            "Special punctuation should be preserved"
        );
        assert!(
            markdown.contains("Quotes: \"double\" and 'single'"),
            "Quote characters should be preserved"
        );
        assert!(
            markdown.contains("Symbols: © ® ™ € £"),
            "Currency and trademark symbols should be preserved"
        );
    }

    /// Test paragraph with newlines
    #[test]
    fn test_paragraph_with_newlines() {
        let mut doc = IdmlDocument::new();
        let mut story = Story::new("u1000".to_string());

        story.add_paragraph(Paragraph::new("Line 1\nLine 2\nLine 3".to_string()));

        doc.add_story(story);

        let markdown = IdmlSerializer::to_markdown(&doc);

        // Newlines within paragraph should be preserved
        assert!(
            markdown.contains("Line 1\nLine 2\nLine 3"),
            "Newlines within paragraph should be preserved"
        );
    }

    /// Test very long paragraph
    #[test]
    fn test_very_long_paragraph() {
        let long_text = "A".repeat(5000);
        let mut doc = IdmlDocument::new();
        let mut story = Story::new("u1000".to_string());

        story.add_paragraph(Paragraph::new(long_text.clone()));
        doc.add_story(story);

        let markdown = IdmlSerializer::to_markdown(&doc);

        // Long paragraph should be preserved
        assert!(
            markdown.contains(&long_text),
            "Very long paragraph text should be preserved"
        );
        assert_eq!(
            markdown.len(),
            long_text.len(),
            "Markdown length should match paragraph length"
        );
    }

    // ===== Integration Tests =====

    /// Test character count matches markdown output
    #[test]
    fn test_character_count_validation() {
        let mut doc = IdmlDocument::new();
        let mut story = Story::new("u1000".to_string());

        story.add_paragraph(Paragraph::new("Test content".to_string()));
        doc.add_story(story);

        let markdown = IdmlSerializer::to_markdown(&doc);
        let char_count = markdown.chars().count();

        // Character count should match markdown length (unicode-aware)
        assert_eq!(
            char_count,
            markdown.chars().count(),
            "Character count should be consistent"
        );
        assert!(
            char_count > 0,
            "Non-empty content should have positive character count"
        );
    }

    /// Test parse_bytes error message
    #[test]
    fn test_parse_bytes_error_message() {
        let backend = IdmlBackend::new();
        let result = backend.parse_bytes(b"test", &BackendOptions::default());

        assert!(result.is_err(), "parse_bytes should return error");
        match result {
            Err(DoclingError::BackendError(msg)) => {
                assert!(
                    msg.contains("ZIP package"),
                    "Error should mention ZIP package requirement"
                );
                assert!(
                    msg.contains("parsing from bytes"),
                    "Error should mention byte parsing limitation"
                );
            }
            _ => panic!("Expected BackendError"),
        }
    }

    /// Test metadata extraction from document
    #[test]
    fn test_metadata_extraction() {
        let metadata = Metadata {
            title: Some("Extracted Title".to_string()),
            author: Some("Extracted Author".to_string()),
        };

        let doc = IdmlDocument::with_metadata(metadata.clone());

        // Verify metadata is stored correctly
        assert_eq!(
            doc.metadata.title,
            Some("Extracted Title".to_string()),
            "Title should be extracted correctly"
        );
        assert_eq!(
            doc.metadata.author,
            Some("Extracted Author".to_string()),
            "Author should be extracted correctly"
        );
    }

    /// Test multiple stories with different IDs
    #[test]
    fn test_multiple_story_ids() {
        let mut doc = IdmlDocument::new();

        doc.add_story(Story::new("u1000".to_string()));
        doc.add_story(Story::new("u2000".to_string()));
        doc.add_story(Story::new("u12345".to_string())); // Non-standard ID

        assert_eq!(doc.stories.len(), 3, "Document should have 3 stories");
        assert_eq!(
            doc.stories[0].id, "u1000",
            "First story should have ID u1000"
        );
        assert_eq!(
            doc.stories[1].id, "u2000",
            "Second story should have ID u2000"
        );
        assert_eq!(
            doc.stories[2].id, "u12345",
            "Third story should have non-standard ID"
        );
    }

    // Note: IDML is a ZIP package format requiring real files.
    // Full integration tests with parse_file would require test IDML files.
    // These tests focus on the backend's markdown generation logic.

    // ===== Additional Format-Specific Tests =====

    /// Test heading with trailing whitespace
    #[test]
    fn test_heading_trailing_whitespace() {
        let mut doc = IdmlDocument::new();
        let mut story = Story::new("u1000".to_string());

        story.add_paragraph(Paragraph::with_style(
            "Heading1".to_string(),
            "Title with spaces   ".to_string(),
        ));

        doc.add_story(story);

        let markdown = IdmlSerializer::to_markdown(&doc);

        // Whitespace should be preserved (trimming is serializer's choice)
        assert!(
            markdown.contains("Title with spaces"),
            "Heading with trailing whitespace should have text preserved"
        );
    }

    /// Test very many paragraphs (stress test)
    #[test]
    fn test_many_paragraphs() {
        let mut doc = IdmlDocument::new();
        let mut story = Story::new("u1000".to_string());

        // Add 100 paragraphs
        for i in 0..100 {
            story.add_paragraph(Paragraph::new(format!("Paragraph {i}")));
        }

        doc.add_story(story);

        let markdown = IdmlSerializer::to_markdown(&doc);

        // Verify first and last paragraphs
        assert!(
            markdown.contains("Paragraph 0"),
            "First paragraph should be present"
        );
        assert!(
            markdown.contains("Paragraph 99"),
            "Last paragraph should be present"
        );

        // Verify all paragraphs present
        for i in 0..100 {
            assert!(
                markdown.contains(&format!("Paragraph {i}")),
                "Paragraph {i} should be present"
            );
        }
    }

    /// Test story with alternating headings and content
    #[test]
    fn test_alternating_headings_and_content() {
        let mut doc = IdmlDocument::new();
        let mut story = Story::new("u1000".to_string());

        story.add_paragraph(Paragraph::with_style(
            "Heading1".to_string(),
            "Section 1".to_string(),
        ));
        story.add_paragraph(Paragraph::new("Content 1".to_string()));
        story.add_paragraph(Paragraph::with_style(
            "Heading2".to_string(),
            "Section 2".to_string(),
        ));
        story.add_paragraph(Paragraph::new("Content 2".to_string()));

        doc.add_story(story);

        let markdown = IdmlSerializer::to_markdown(&doc);

        // Verify structure
        assert!(
            markdown.contains("# Section 1"),
            "First heading should be H1"
        );
        assert!(
            markdown.contains("Content 1"),
            "First content should be present"
        );
        assert!(
            markdown.contains("## Section 2"),
            "Second heading should be H2"
        );
        assert!(
            markdown.contains("Content 2"),
            "Second content should be present"
        );

        // Verify order
        let section1_pos = markdown.find("# Section 1").unwrap();
        let content1_pos = markdown.find("Content 1").unwrap();
        let section2_pos = markdown.find("## Section 2").unwrap();
        let content2_pos = markdown.find("Content 2").unwrap();

        assert!(
            section1_pos < content1_pos,
            "Section 1 heading should precede its content"
        );
        assert!(
            content1_pos < section2_pos,
            "Content 1 should precede Section 2 heading"
        );
        assert!(
            section2_pos < content2_pos,
            "Section 2 heading should precede its content"
        );
    }

    // ===== Additional Metadata Edge Cases =====

    /// Test metadata with empty strings (vs None)
    #[test]
    fn test_metadata_empty_strings() {
        let metadata = Metadata {
            title: Some("".to_string()),
            author: Some("".to_string()),
        };

        let doc = IdmlDocument::with_metadata(metadata);
        let markdown = IdmlSerializer::to_markdown(&doc);

        // Empty strings may or may not generate frontmatter (implementation choice)
        // Just verify it doesn't crash
        let _ = markdown;
    }

    /// Test metadata with unicode characters
    #[test]
    fn test_metadata_unicode() {
        let metadata = Metadata {
            title: Some("日本語タイトル 📚".to_string()),
            author: Some("作者名 🖊️".to_string()),
        };

        let doc = IdmlDocument::with_metadata(metadata);
        let markdown = IdmlSerializer::to_markdown(&doc);

        // Unicode should be preserved in frontmatter
        assert!(
            markdown.contains("日本語タイトル 📚"),
            "Japanese title with emoji should be preserved"
        );
        assert!(
            markdown.contains("作者名 🖊️"),
            "Japanese author name with emoji should be preserved"
        );
    }

    /// Test metadata with newlines (edge case)
    #[test]
    fn test_metadata_with_newlines() {
        let metadata = Metadata {
            title: Some("Multi\nLine\nTitle".to_string()),
            author: Some("Author\nName".to_string()),
        };

        let doc = IdmlDocument::with_metadata(metadata);
        let markdown = IdmlSerializer::to_markdown(&doc);

        // Newlines in metadata are valid (YAML handles multi-line)
        assert!(
            markdown.contains("Multi\nLine\nTitle"),
            "Multi-line title should be preserved"
        );
    }

    // ===== Additional Story Structure Tests =====

    /// Test many stories (stress test)
    #[test]
    fn test_many_stories() {
        let mut doc = IdmlDocument::new();

        // Add 50 stories
        for i in 0..50 {
            let mut story = Story::new(format!("u{}", i * 1000));
            story.add_paragraph(Paragraph::new(format!("Story {i} content")));
            doc.add_story(story);
        }

        let markdown = IdmlSerializer::to_markdown(&doc);

        // Verify all stories present
        for i in 0..50 {
            assert!(
                markdown.contains(&format!("Story {i} content")),
                "Story {i} should be present"
            );
        }
    }

    /// Test story order preservation across multiple stories
    #[test]
    fn test_story_order_preservation() {
        let mut doc = IdmlDocument::new();

        // Add stories with identifiable content
        let mut story1 = Story::new("u1000".to_string());
        story1.add_paragraph(Paragraph::new("First story".to_string()));
        doc.add_story(story1);

        let mut story2 = Story::new("u2000".to_string());
        story2.add_paragraph(Paragraph::new("Second story".to_string()));
        doc.add_story(story2);

        let mut story3 = Story::new("u3000".to_string());
        story3.add_paragraph(Paragraph::new("Third story".to_string()));
        doc.add_story(story3);

        let markdown = IdmlSerializer::to_markdown(&doc);

        // Verify order is preserved
        let first_pos = markdown.find("First story").unwrap();
        let second_pos = markdown.find("Second story").unwrap();
        let third_pos = markdown.find("Third story").unwrap();

        assert!(first_pos < second_pos, "First story should precede second");
        assert!(second_pos < third_pos, "Second story should precede third");
    }

    /// Test story with mixed empty and non-empty paragraphs
    #[test]
    fn test_story_mixed_empty_paragraphs() {
        let mut doc = IdmlDocument::new();
        let mut story = Story::new("u1000".to_string());

        story.add_paragraph(Paragraph::new("First".to_string()));
        story.add_paragraph(Paragraph::new("".to_string()));
        story.add_paragraph(Paragraph::new("Second".to_string()));
        story.add_paragraph(Paragraph::new("".to_string()));
        story.add_paragraph(Paragraph::new("Third".to_string()));

        doc.add_story(story);

        let markdown = IdmlSerializer::to_markdown(&doc);

        // Non-empty paragraphs should be present
        assert!(
            markdown.contains("First"),
            "First paragraph should be present"
        );
        assert!(
            markdown.contains("Second"),
            "Second paragraph should be present"
        );
        assert!(
            markdown.contains("Third"),
            "Third paragraph should be present"
        );
    }

    // ===== Additional Heading Tests =====

    /// Test heading styles case sensitivity
    #[test]
    fn test_heading_case_sensitivity() {
        let mut doc = IdmlDocument::new();
        let mut story = Story::new("u1000".to_string());

        // Test exact case match
        story.add_paragraph(Paragraph::with_style(
            "Heading1".to_string(),
            "Exact case".to_string(),
        ));

        doc.add_story(story);

        let markdown = IdmlSerializer::to_markdown(&doc);

        // Should match (case-sensitive "Heading1")
        assert!(
            markdown.contains("# Exact case"),
            "Case-sensitive Heading1 should produce markdown header"
        );
    }

    /// Test all non-heading styles remain plain text
    #[test]
    fn test_non_heading_styles_plain_text() {
        let mut doc = IdmlDocument::new();
        let mut story = Story::new("u1000".to_string());

        story.add_paragraph(Paragraph::with_style(
            "BodyText".to_string(),
            "Body".to_string(),
        ));
        story.add_paragraph(Paragraph::with_style(
            "Caption".to_string(),
            "Caption".to_string(),
        ));
        story.add_paragraph(Paragraph::with_style(
            "Footnote".to_string(),
            "Footnote".to_string(),
        ));

        doc.add_story(story);

        let markdown = IdmlSerializer::to_markdown(&doc);

        // None should have markdown headers
        assert!(
            !markdown.contains("# Body"),
            "BodyText style should not create header"
        );
        assert!(
            !markdown.contains("# Caption"),
            "Caption style should not create header"
        );
        assert!(
            !markdown.contains("# Footnote"),
            "Footnote style should not create header"
        );

        // Should appear as plain text
        assert!(markdown.contains("Body"), "Body text should be present");
        assert!(
            markdown.contains("Caption"),
            "Caption text should be present"
        );
        assert!(
            markdown.contains("Footnote"),
            "Footnote text should be present"
        );
    }

    // ===== Additional Content Edge Cases =====

    /// Test paragraph with only whitespace
    #[test]
    fn test_paragraph_only_whitespace() {
        let mut doc = IdmlDocument::new();
        let mut story = Story::new("u1000".to_string());

        story.add_paragraph(Paragraph::new("Before".to_string()));
        story.add_paragraph(Paragraph::new("   ".to_string())); // Only spaces
        story.add_paragraph(Paragraph::new("After".to_string()));

        doc.add_story(story);

        let markdown = IdmlSerializer::to_markdown(&doc);

        // Before and After should be present
        assert!(
            markdown.contains("Before"),
            "Content before whitespace-only paragraph should be present"
        );
        assert!(
            markdown.contains("After"),
            "Content after whitespace-only paragraph should be present"
        );
    }

    /// Test paragraph with tabs and mixed whitespace
    #[test]
    fn test_paragraph_mixed_whitespace() {
        let mut doc = IdmlDocument::new();
        let mut story = Story::new("u1000".to_string());

        story.add_paragraph(Paragraph::new("Text\twith\ttabs".to_string()));
        story.add_paragraph(Paragraph::new("Text  with   spaces".to_string()));

        doc.add_story(story);

        let markdown = IdmlSerializer::to_markdown(&doc);

        // Whitespace should be preserved
        assert!(
            markdown.contains("Text\twith\ttabs"),
            "Tab characters should be preserved"
        );
        assert!(
            markdown.contains("Text  with   spaces"),
            "Multiple spaces should be preserved"
        );
    }

    /// Test paragraph with markdown-like characters
    #[test]
    fn test_paragraph_markdown_characters() {
        let mut doc = IdmlDocument::new();
        let mut story = Story::new("u1000".to_string());

        story.add_paragraph(Paragraph::new("Text with *asterisks*".to_string()));
        story.add_paragraph(Paragraph::new("Text with _underscores_".to_string()));
        story.add_paragraph(Paragraph::new("Text with `backticks`".to_string()));
        story.add_paragraph(Paragraph::new("Text with [brackets]".to_string()));

        doc.add_story(story);

        let markdown = IdmlSerializer::to_markdown(&doc);

        // Markdown special chars should be preserved as-is
        assert!(
            markdown.contains("Text with *asterisks*"),
            "Asterisks should be preserved"
        );
        assert!(
            markdown.contains("Text with _underscores_"),
            "Underscores should be preserved"
        );
        assert!(
            markdown.contains("Text with `backticks`"),
            "Backticks should be preserved"
        );
        assert!(
            markdown.contains("Text with [brackets]"),
            "Brackets should be preserved"
        );
    }

    // ===== Additional Integration Tests =====

    /// Test BackendOptions ignored (IDML doesn't use options)
    #[test]
    fn test_backend_options_ignored() {
        let backend = IdmlBackend::new();
        let options = BackendOptions::default()
            .with_ocr(true)
            .with_table_structure(true);

        // parse_bytes should fail regardless of options
        let result = backend.parse_bytes(b"test", &options);
        assert!(
            result.is_err(),
            "parse_bytes should fail even with OCR and table options"
        );
    }

    /// Test format field in Document
    #[test]
    fn test_document_format_field() {
        let mut doc = IdmlDocument::new();
        let mut story = Story::new("u1000".to_string());
        story.add_paragraph(Paragraph::new("Content".to_string()));
        doc.add_story(story);

        // Simulate what parse_file would return
        let markdown = IdmlSerializer::to_markdown(&doc);
        let document = Document {
            markdown,
            format: InputFormat::Idml,
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

        assert_eq!(
            document.format,
            InputFormat::Idml,
            "Document format field should be Idml"
        );
    }

    /// Test DocItem generation from IDML
    #[test]
    fn test_idml_docitem_generation() {
        let mut doc = IdmlDocument::new();
        let mut story = Story::new("u1000".to_string());

        // Add a heading
        story.add_paragraph(Paragraph::with_style(
            "Heading1".to_string(),
            "Test Heading".to_string(),
        ));

        // Add regular text
        story.add_paragraph(Paragraph::new("Body paragraph.".to_string()));

        doc.add_story(story);

        // Generate DocItems
        let doc_items = IdmlBackend::idml_to_doc_items(&doc);

        // Should have 2 DocItems: SectionHeader + Text
        assert_eq!(
            doc_items.len(),
            2,
            "Should generate 2 DocItems (heading + text)"
        );

        // First item should be SectionHeader
        if let DocItem::SectionHeader {
            text,
            level,
            self_ref,
            ..
        } = &doc_items[0]
        {
            assert_eq!(text, "Test Heading");
            assert_eq!(*level, 1);
            assert_eq!(self_ref, "#/texts/0");
        } else {
            panic!("Expected SectionHeader, got {:?}", doc_items[0]);
        }

        // Second item should be Text
        if let DocItem::Text { text, self_ref, .. } = &doc_items[1] {
            assert_eq!(text, "Body paragraph.");
            assert_eq!(self_ref, "#/texts/1");
        } else {
            panic!("Expected Text, got {:?}", doc_items[1]);
        }
    }

    // ===== IDML Package Structure Tests =====

    /// Test story threading across multiple frames (common in magazines)
    #[test]
    fn test_story_threading_multi_frame() {
        let mut doc = IdmlDocument::new();
        let mut story = Story::new("u1000".to_string());

        // Story flows across 3 frames (pages 1-3)
        story.add_paragraph(Paragraph::new("Page 1 content flows".to_string()));
        story.add_paragraph(Paragraph::new("to page 2 content flows".to_string()));
        story.add_paragraph(Paragraph::new("to page 3 content".to_string()));

        doc.add_story(story);

        let markdown = IdmlSerializer::to_markdown(&doc);

        // All content should be in one continuous story
        assert!(
            markdown.contains("Page 1 content flows"),
            "First frame content should be present"
        );
        assert!(
            markdown.contains("to page 2 content flows"),
            "Second frame content should be present"
        );
        assert!(
            markdown.contains("to page 3 content"),
            "Third frame content should be present"
        );

        // Order should be preserved
        let page1_pos = markdown.find("Page 1").unwrap();
        let page2_pos = markdown.find("to page 2").unwrap();
        let page3_pos = markdown.find("to page 3").unwrap();
        assert!(
            page1_pos < page2_pos,
            "Page 1 content should precede page 2 in threaded story"
        );
        assert!(
            page2_pos < page3_pos,
            "Page 2 content should precede page 3 in threaded story"
        );
    }

    /// Test magazine layout with multiple independent stories
    #[test]
    fn test_magazine_layout_multiple_stories() {
        let mut doc = IdmlDocument::new();

        // Story 1: Main article
        let mut main_article = Story::new("u1000".to_string());
        main_article.add_paragraph(Paragraph::with_style(
            "Heading1".to_string(),
            "Feature Article".to_string(),
        ));
        main_article.add_paragraph(Paragraph::new(
            "Main article content spanning multiple pages.".to_string(),
        ));
        doc.add_story(main_article);

        // Story 2: Sidebar
        let mut sidebar = Story::new("u2000".to_string());
        sidebar.add_paragraph(Paragraph::with_style(
            "Heading2".to_string(),
            "Sidebar".to_string(),
        ));
        sidebar.add_paragraph(Paragraph::new("Sidebar content.".to_string()));
        doc.add_story(sidebar);

        // Story 3: Caption
        let mut caption = Story::new("u3000".to_string());
        caption.add_paragraph(Paragraph::new("Photo caption text.".to_string()));
        doc.add_story(caption);

        let markdown = IdmlSerializer::to_markdown(&doc);

        // All stories should be present
        assert!(
            markdown.contains("# Feature Article"),
            "Main article heading should be present"
        );
        assert!(
            markdown.contains("Main article content"),
            "Main article body should be present"
        );
        assert!(
            markdown.contains("## Sidebar"),
            "Sidebar heading should be present"
        );
        assert!(
            markdown.contains("Photo caption text"),
            "Caption story should be present"
        );
    }

    /// Test book layout with chapter structure
    #[test]
    fn test_book_layout_chapters() {
        let mut doc = IdmlDocument::new();

        // Chapter 1
        let mut chapter1 = Story::new("u1000".to_string());
        chapter1.add_paragraph(Paragraph::with_style(
            "Heading1".to_string(),
            "Chapter 1: Introduction".to_string(),
        ));
        chapter1.add_paragraph(Paragraph::new(
            "This is the first chapter content.".to_string(),
        ));
        doc.add_story(chapter1);

        // Chapter 2
        let mut chapter2 = Story::new("u2000".to_string());
        chapter2.add_paragraph(Paragraph::with_style(
            "Heading1".to_string(),
            "Chapter 2: Development".to_string(),
        ));
        chapter2.add_paragraph(Paragraph::new(
            "This is the second chapter content.".to_string(),
        ));
        doc.add_story(chapter2);

        let markdown = IdmlSerializer::to_markdown(&doc);

        // Both chapters should be present with headings
        assert!(
            markdown.contains("# Chapter 1: Introduction"),
            "Chapter 1 heading should be present"
        );
        assert!(
            markdown.contains("first chapter content"),
            "Chapter 1 body content should be present"
        );
        assert!(
            markdown.contains("# Chapter 2: Development"),
            "Chapter 2 heading should be present"
        );
        assert!(
            markdown.contains("second chapter content"),
            "Chapter 2 body content should be present"
        );
    }

    /// Test InDesign special typography characters
    #[test]
    fn test_indesign_special_characters() {
        let mut doc = IdmlDocument::new();
        let mut story = Story::new("u1000".to_string());

        // InDesign commonly uses these typographic characters
        story.add_paragraph(Paragraph::new("Em dash — separator".to_string()));
        story.add_paragraph(Paragraph::new("En dash – range".to_string()));
        story.add_paragraph(Paragraph::new("Ellipsis… continuation".to_string()));
        story.add_paragraph(Paragraph::new(
            "Quotations: \u{201C}smart quotes\u{201D}".to_string(),
        ));
        story.add_paragraph(Paragraph::new("Apostrophe: don\u{2019}t".to_string()));
        story.add_paragraph(Paragraph::new(
            "Non-breaking space:\u{00A0}here".to_string(),
        ));

        doc.add_story(story);

        let markdown = IdmlSerializer::to_markdown(&doc);

        // All special characters should be preserved
        assert!(
            markdown.contains("Em dash — separator"),
            "Em dash character should be preserved"
        );
        assert!(
            markdown.contains("En dash – range"),
            "En dash character should be preserved"
        );
        assert!(
            markdown.contains("Ellipsis… continuation"),
            "Ellipsis character should be preserved"
        );
        assert!(
            markdown.contains("\u{201C}smart quotes\u{201D}"),
            "Smart quote characters should be preserved"
        );
        assert!(
            markdown.contains("don\u{2019}t"),
            "Curly apostrophe should be preserved"
        );
        assert!(
            markdown.contains("\u{00A0}"),
            "Non-breaking space should be preserved"
        );
    }

    /// Test paragraph style inheritance (style names with variations)
    #[test]
    fn test_paragraph_style_variations() {
        let mut doc = IdmlDocument::new();
        let mut story = Story::new("u1000".to_string());

        // InDesign allows style name variations
        story.add_paragraph(Paragraph::with_style(
            "Heading1".to_string(),
            "Standard H1".to_string(),
        ));
        story.add_paragraph(Paragraph::with_style(
            "Heading1Bold".to_string(),
            "Bold variant".to_string(),
        ));
        story.add_paragraph(Paragraph::with_style(
            "ParagraphStyle1".to_string(),
            "Custom style 1".to_string(),
        ));
        story.add_paragraph(Paragraph::with_style(
            "ParagraphStyle2".to_string(),
            "Custom style 2".to_string(),
        ));

        doc.add_story(story);

        let markdown = IdmlSerializer::to_markdown(&doc);

        // Heading1 should be converted
        assert!(
            markdown.contains("# Standard H1"),
            "Exact Heading1 style should convert to H1 markdown"
        );

        // Non-standard styles should be plain text
        assert!(
            markdown.contains("Bold variant"),
            "Heading1Bold variant should appear as plain text"
        );
        assert!(
            markdown.contains("Custom style 1"),
            "ParagraphStyle1 should appear as plain text"
        );
        assert!(
            markdown.contains("Custom style 2"),
            "ParagraphStyle2 should appear as plain text"
        );
    }

    /// Test nested story structure (complex documents)
    #[test]
    fn test_nested_text_frame_content() {
        let mut doc = IdmlDocument::new();

        // Main text frame
        let mut main_text = Story::new("u1000".to_string());
        main_text.add_paragraph(Paragraph::new("Main body text.".to_string()));
        doc.add_story(main_text);

        // Anchored frame (text box within text)
        let mut anchored = Story::new("u1001".to_string());
        anchored.add_paragraph(Paragraph::new("Anchored text box.".to_string()));
        doc.add_story(anchored);

        // Footer frame
        let mut footer = Story::new("u1002".to_string());
        footer.add_paragraph(Paragraph::new("Footer content.".to_string()));
        doc.add_story(footer);

        let markdown = IdmlSerializer::to_markdown(&doc);

        // All frames should be represented
        assert!(
            markdown.contains("Main body text"),
            "Main text frame content should be present"
        );
        assert!(
            markdown.contains("Anchored text box"),
            "Anchored frame content should be present"
        );
        assert!(
            markdown.contains("Footer content"),
            "Footer frame content should be present"
        );
    }

    /// Test very long story with many paragraphs (performance test)
    #[test]
    fn test_very_long_story_performance() {
        let mut doc = IdmlDocument::new();
        let mut story = Story::new("u1000".to_string());

        // Simulate a book chapter with 1000 paragraphs
        for i in 0..1000 {
            story.add_paragraph(Paragraph::new(format!(
                "Paragraph {i} content for performance testing."
            )));
        }

        doc.add_story(story);

        let markdown = IdmlSerializer::to_markdown(&doc);

        // Verify first and last paragraphs
        assert!(
            markdown.contains("Paragraph 0 content"),
            "First paragraph should be present"
        );
        assert!(
            markdown.contains("Paragraph 999 content"),
            "Last paragraph (999) should be present"
        );

        // Verify approximate length (1000 paragraphs * ~50 chars each)
        assert!(
            markdown.len() > 40000,
            "Long story output should exceed 40000 characters"
        );
    }

    /// Test mixed content with all features (integration)
    #[test]
    fn test_mixed_content_comprehensive() {
        let metadata = Metadata {
            title: Some("Comprehensive IDML Test".to_string()),
            author: Some("Test Suite".to_string()),
        };

        let mut doc = IdmlDocument::with_metadata(metadata);
        let mut story = Story::new("u1000".to_string());

        // Title
        story.add_paragraph(Paragraph::with_style(
            "Heading1".to_string(),
            "Document Title".to_string(),
        ));

        // Subtitle
        story.add_paragraph(Paragraph::with_style(
            "Heading2".to_string(),
            "Subtitle".to_string(),
        ));

        // Body paragraphs
        story.add_paragraph(Paragraph::new("First body paragraph.".to_string()));
        story.add_paragraph(Paragraph::new("Second body paragraph.".to_string()));

        // Subheading
        story.add_paragraph(Paragraph::with_style(
            "Heading3".to_string(),
            "Section".to_string(),
        ));

        // More content
        story.add_paragraph(Paragraph::new("Section content.".to_string()));

        // Special characters
        story.add_paragraph(Paragraph::new(
            "Special: — – … \u{201C}quotes\u{201D}".to_string(),
        ));

        // Unicode
        story.add_paragraph(Paragraph::new("Unicode: 日本語 émoji 🎨".to_string()));

        doc.add_story(story);

        let markdown = IdmlSerializer::to_markdown(&doc);

        // Verify frontmatter
        assert!(
            markdown.contains("title: Comprehensive IDML Test"),
            "Frontmatter should contain title"
        );

        // Verify all content
        assert!(
            markdown.contains("# Document Title"),
            "H1 title should be present"
        );
        assert!(
            markdown.contains("## Subtitle"),
            "H2 subtitle should be present"
        );
        assert!(
            markdown.contains("### Section"),
            "H3 section heading should be present"
        );
        assert!(
            markdown.contains("First body paragraph"),
            "Body paragraph should be present"
        );
        assert!(
            markdown.contains("Special: — – …"),
            "Special typography characters should be preserved"
        );
        assert!(
            markdown.contains("日本語"),
            "Unicode Japanese text should be preserved"
        );
    }

    /// Test document with only master page content (edge case)
    #[test]
    fn test_master_page_only_content() {
        let mut doc = IdmlDocument::new();

        // Master page content (headers, footers, page numbers)
        let mut master_content = Story::new("u9999".to_string());
        master_content.add_paragraph(Paragraph::new("Header content".to_string()));
        master_content.add_paragraph(Paragraph::new("Page number: 1".to_string()));
        doc.add_story(master_content);

        let markdown = IdmlSerializer::to_markdown(&doc);

        // Master page content should still be extracted
        assert!(
            markdown.contains("Header content"),
            "Master page header content should be present"
        );
        assert!(
            markdown.contains("Page number: 1"),
            "Master page number content should be present"
        );
    }

    /// Test multiple headings in sequence (table of contents pattern)
    #[test]
    fn test_multiple_headings_sequence() {
        let mut doc = IdmlDocument::new();
        let mut story = Story::new("u1000".to_string());

        // Table of contents pattern: many headings with minimal content
        story.add_paragraph(Paragraph::with_style(
            "Heading1".to_string(),
            "Chapter 1".to_string(),
        ));
        story.add_paragraph(Paragraph::with_style(
            "Heading2".to_string(),
            "Section 1.1".to_string(),
        ));
        story.add_paragraph(Paragraph::with_style(
            "Heading2".to_string(),
            "Section 1.2".to_string(),
        ));
        story.add_paragraph(Paragraph::with_style(
            "Heading1".to_string(),
            "Chapter 2".to_string(),
        ));
        story.add_paragraph(Paragraph::with_style(
            "Heading2".to_string(),
            "Section 2.1".to_string(),
        ));

        doc.add_story(story);

        let markdown = IdmlSerializer::to_markdown(&doc);

        // All headings should be present with correct levels
        assert!(
            markdown.contains("# Chapter 1"),
            "Chapter 1 H1 heading should be present"
        );
        assert!(
            markdown.contains("## Section 1.1"),
            "Section 1.1 H2 heading should be present"
        );
        assert!(
            markdown.contains("## Section 1.2"),
            "Section 1.2 H2 heading should be present"
        );
        assert!(
            markdown.contains("# Chapter 2"),
            "Chapter 2 H1 heading should be present"
        );
        assert!(
            markdown.contains("## Section 2.1"),
            "Section 2.1 H2 heading should be present"
        );

        // Verify hierarchy is preserved
        let ch1 = markdown.find("# Chapter 1").unwrap();
        let s11 = markdown.find("## Section 1.1").unwrap();
        let s12 = markdown.find("## Section 1.2").unwrap();
        let ch2 = markdown.find("# Chapter 2").unwrap();
        let s21 = markdown.find("## Section 2.1").unwrap();

        assert!(ch1 < s11, "Chapter 1 should precede Section 1.1");
        assert!(s11 < s12, "Section 1.1 should precede Section 1.2");
        assert!(s12 < ch2, "Section 1.2 should precede Chapter 2");
        assert!(ch2 < s21, "Chapter 2 should precede Section 2.1");
    }

    #[test]
    fn test_character_styles_basic_text() {
        use docling_adobe::idml::types::{IdmlDocument, Paragraph, Story};

        let mut doc = IdmlDocument::new();
        let mut story = Story::new("u1000".to_string());

        // Paragraph with bold and italic markers in text
        let para1 = Paragraph::new("This is **bold** text.".to_string());
        story.add_paragraph(para1);

        // Paragraph with italic markers
        let para2 = Paragraph::new("This is *italic* text.".to_string());
        story.add_paragraph(para2);

        doc.add_story(story);

        let markdown = IdmlSerializer::to_markdown(&doc);
        // Should contain text with markdown formatting
        assert!(
            markdown.contains("bold"),
            "Bold marked text should be present"
        );
        assert!(
            markdown.contains("italic"),
            "Italic marked text should be present"
        );
    }

    #[test]
    fn test_text_frame_threading() {
        use docling_adobe::idml::types::{IdmlDocument, Paragraph, Story};

        let mut doc = IdmlDocument::new();

        // First threaded text frame
        let mut story1 = Story::new("u1000".to_string());
        story1.add_paragraph(Paragraph::new(
            "Start of threaded text that continues...".to_string(),
        ));
        doc.add_story(story1);

        // Second threaded text frame (continuation)
        let mut story2 = Story::new("u1001".to_string());
        story2.add_paragraph(Paragraph::new("...into the next frame.".to_string()));
        doc.add_story(story2);

        let markdown = IdmlSerializer::to_markdown(&doc);

        // Both text frames should be included
        assert!(
            markdown.contains("Start of threaded text"),
            "First threaded frame content should be present"
        );
        assert!(
            markdown.contains("into the next frame"),
            "Second threaded frame content should be present"
        );
    }

    #[test]
    fn test_master_spread_content() {
        use docling_adobe::idml::types::{IdmlDocument, Paragraph, Story};

        let mut doc = IdmlDocument::new();

        // Master spread usually contains headers/footers/page numbers
        let mut master_story = Story::new("uMaster".to_string());
        master_story.add_paragraph(Paragraph::new("Page Header".to_string()));
        master_story.add_paragraph(Paragraph::new("© 2024 Company".to_string()));
        doc.add_story(master_story);

        // Regular page content
        let mut story = Story::new("u1000".to_string());
        story.add_paragraph(Paragraph::new("Main content here.".to_string()));
        doc.add_story(story);

        let markdown = IdmlSerializer::to_markdown(&doc);

        // Master spread content should be included
        assert!(
            markdown.contains("Page Header"),
            "Master spread header should be present"
        );
        assert!(
            markdown.contains("© 2024 Company"),
            "Master spread copyright should be present"
        );
        assert!(
            markdown.contains("Main content here"),
            "Regular page content should be present"
        );
    }

    #[test]
    fn test_page_numbering_markers() {
        use docling_adobe::idml::types::{IdmlDocument, Paragraph, Story};

        let mut doc = IdmlDocument::new();
        let mut story = Story::new("u1000".to_string());

        // IDML uses special markers for page numbers, e.g., <Auto>
        story.add_paragraph(Paragraph::new("Page <Auto>".to_string()));
        story.add_paragraph(Paragraph::new("Content on this page.".to_string()));

        doc.add_story(story);

        let markdown = IdmlSerializer::to_markdown(&doc);

        // Page number markers should be preserved or converted
        assert!(
            markdown.contains("Content on this page"),
            "Page content text should be present"
        );
        // The <Auto> marker may be stripped or converted
    }

    /// Test hyperlinks and cross-references in content
    #[test]
    fn test_idml_hyperlinks_and_cross_references() {
        use docling_adobe::idml::types::{IdmlDocument, Paragraph, Story};

        let mut doc = IdmlDocument::new();
        let mut story = Story::new("u1000".to_string());

        // InDesign supports hyperlinks (external URLs)
        story.add_paragraph(Paragraph::new(
            "Visit our website at https://example.com for more information.".to_string(),
        ));

        // Cross-references to other parts of document (internal links)
        story.add_paragraph(Paragraph::with_style(
            "Heading1".to_string(),
            "Chapter 1: Introduction".to_string(),
        ));
        story.add_paragraph(Paragraph::new(
            "See Chapter 1: Introduction for details.".to_string(),
        ));

        // Email links
        story.add_paragraph(Paragraph::new(
            "Contact us at support@example.com.".to_string(),
        ));

        doc.add_story(story);

        let markdown = IdmlSerializer::to_markdown(&doc);

        // Verify hyperlinks are preserved
        assert!(
            markdown.contains("https://example.com"),
            "URL hyperlink should be preserved"
        );
        assert!(
            markdown.contains("support@example.com"),
            "Email link should be preserved"
        );
        assert!(
            markdown.contains("Chapter 1: Introduction"),
            "Cross-reference text should be preserved"
        );
    }

    /// Test index markers and glossary entries (book publishing feature)
    #[test]
    fn test_idml_index_markers_and_glossary() {
        use docling_adobe::idml::types::{IdmlDocument, Paragraph, Story};

        let mut doc = IdmlDocument::new();
        let mut story = Story::new("u1000".to_string());

        // Index markers are typically invisible in text but mark terms for indexing
        story.add_paragraph(Paragraph::new(
            "The photosynthesis process converts light energy.".to_string(),
        ));

        // Topic markers for glossary generation
        story.add_paragraph(Paragraph::new(
            "Chloroplast: organelle where photosynthesis occurs.".to_string(),
        ));

        // Multiple index levels (main term, subterm)
        story.add_paragraph(Paragraph::new(
            "Plants use chlorophyll for light absorption.".to_string(),
        ));

        doc.add_story(story);

        let markdown = IdmlSerializer::to_markdown(&doc);

        // Text content should be preserved (markers are typically stripped in output)
        assert!(
            markdown.contains("photosynthesis"),
            "Index term 'photosynthesis' should be preserved"
        );
        assert!(
            markdown.contains("Chloroplast"),
            "Glossary term 'Chloroplast' should be preserved"
        );
        assert!(
            markdown.contains("chlorophyll"),
            "Index term 'chlorophyll' should be preserved"
        );
    }

    /// Test footnotes and endnotes in document (academic/publishing feature)
    #[test]
    fn test_idml_footnotes_and_endnotes() {
        use docling_adobe::idml::types::{IdmlDocument, Paragraph, Story};

        let mut doc = IdmlDocument::new();
        let mut story = Story::new("u1000".to_string());

        // Main text with footnote reference
        story.add_paragraph(Paragraph::new(
            "The study found significant results.[1]".to_string(),
        ));

        // Footnote content (typically in separate story in IDML)
        story.add_paragraph(Paragraph::new(
            "[1] Smith et al., \"Research Paper\", 2023, p. 42.".to_string(),
        ));

        // Endnote reference
        story.add_paragraph(Paragraph::new("Further research is needed.[i]".to_string()));

        // Endnote content
        story.add_paragraph(Paragraph::new(
            "[i] For detailed methodology, see Appendix A.".to_string(),
        ));

        doc.add_story(story);

        let markdown = IdmlSerializer::to_markdown(&doc);

        // Verify footnote/endnote markers and content
        assert!(
            markdown.contains("[1]"),
            "Footnote marker [1] should be preserved"
        );
        assert!(
            markdown.contains("Smith et al."),
            "Footnote citation should be preserved"
        );
        assert!(
            markdown.contains("[i]"),
            "Endnote marker [i] should be preserved"
        );
        assert!(
            markdown.contains("Appendix A"),
            "Endnote reference should be preserved"
        );
    }

    /// Test tables embedded within stories (structured data in text flow)
    #[test]
    fn test_idml_tables_in_text_flow() {
        use docling_adobe::idml::types::{IdmlDocument, Paragraph, Story};

        let mut doc = IdmlDocument::new();
        let mut story = Story::new("u1000".to_string());

        // Text before table
        story.add_paragraph(Paragraph::new("Sales data for Q4:".to_string()));

        // Table content (simplified - real IDML has complex table XML)
        // In IDML, tables are typically in separate XML but referenced in story flow
        story.add_paragraph(Paragraph::new("Product | Units | Revenue".to_string()));
        story.add_paragraph(Paragraph::new("Widget A | 1000 | $50,000".to_string()));
        story.add_paragraph(Paragraph::new("Widget B | 1500 | $75,000".to_string()));
        story.add_paragraph(Paragraph::new("Total | 2500 | $125,000".to_string()));

        // Text after table
        story.add_paragraph(Paragraph::new(
            "The table shows strong performance in Widget B.".to_string(),
        ));

        doc.add_story(story);

        let markdown = IdmlSerializer::to_markdown(&doc);

        // Verify table content is preserved
        assert!(
            markdown.contains("Sales data"),
            "Table intro text should be present"
        );
        assert!(
            markdown.contains("Product"),
            "Table header should be present"
        );
        assert!(
            markdown.contains("Widget A"),
            "Table row data should be present"
        );
        assert!(
            markdown.contains("$125,000"),
            "Table total should be present"
        );
        assert!(
            markdown.contains("strong performance"),
            "Text after table should be present"
        );
    }

    /// Test bulleted and numbered lists (common formatting patterns)
    #[test]
    fn test_idml_bulleted_and_numbered_lists() {
        use docling_adobe::idml::types::{IdmlDocument, Paragraph, Story};

        let mut doc = IdmlDocument::new();
        let mut story = Story::new("u1000".to_string());

        // Bulleted list (InDesign uses special bullet characters)
        story.add_paragraph(Paragraph::new("Features:".to_string()));
        story.add_paragraph(Paragraph::new("• Easy to use".to_string()));
        story.add_paragraph(Paragraph::new("• Fast performance".to_string()));
        story.add_paragraph(Paragraph::new("• Cross-platform".to_string()));

        // Numbered list
        story.add_paragraph(Paragraph::new("Installation steps:".to_string()));
        story.add_paragraph(Paragraph::new("1. Download the installer".to_string()));
        story.add_paragraph(Paragraph::new("2. Run setup wizard".to_string()));
        story.add_paragraph(Paragraph::new("3. Configure preferences".to_string()));
        story.add_paragraph(Paragraph::new("4. Launch application".to_string()));

        // Nested list (sub-bullets)
        story.add_paragraph(Paragraph::new("Requirements:".to_string()));
        story.add_paragraph(Paragraph::new("• Hardware".to_string()));
        story.add_paragraph(Paragraph::new("  ◦ 8GB RAM minimum".to_string()));
        story.add_paragraph(Paragraph::new("  ◦ 256GB storage".to_string()));
        story.add_paragraph(Paragraph::new("• Software".to_string()));
        story.add_paragraph(Paragraph::new("  ◦ macOS 10.15+".to_string()));

        doc.add_story(story);

        let markdown = IdmlSerializer::to_markdown(&doc);

        // Verify list formatting is preserved
        assert!(
            markdown.contains("Features:"),
            "Bulleted list header should be present"
        );
        assert!(
            markdown.contains("Easy to use"),
            "Bulleted list item should be present"
        );
        assert!(
            markdown.contains("1. Download"),
            "Numbered list first item should be present"
        );
        assert!(
            markdown.contains("4. Launch"),
            "Numbered list fourth item should be present"
        );
        assert!(
            markdown.contains("Hardware"),
            "Nested list parent should be present"
        );
        assert!(
            markdown.contains("8GB RAM"),
            "Nested list child item should be present"
        );
    }

    /// Test master pages and page templates (common in multi-page layouts)
    #[test]
    fn test_idml_master_pages_and_templates() {
        use docling_adobe::idml::types::{IdmlDocument, Paragraph, Story};

        let mut doc = IdmlDocument::new();

        // Master page content (headers/footers typically defined in master pages)
        let mut master_story = Story::new("master_u1000".to_string());
        master_story.add_paragraph(Paragraph::new("Company Name | Page Header".to_string()));
        master_story.add_paragraph(Paragraph::new(
            "© 2024 All Rights Reserved | Footer".to_string(),
        ));
        doc.add_story(master_story);

        // Body content story
        let mut body_story = Story::new("body_u2000".to_string());
        body_story.add_paragraph(Paragraph::new(
            "This is the main content on the page.".to_string(),
        ));
        body_story.add_paragraph(Paragraph::new(
            "Master page elements appear on every page.".to_string(),
        ));
        doc.add_story(body_story);

        let markdown = IdmlSerializer::to_markdown(&doc);

        // Verify both master page and body content
        assert!(
            markdown.contains("Company Name"),
            "Master page company name should be present"
        );
        assert!(
            markdown.contains("Page Header"),
            "Master page header text should be present"
        );
        assert!(
            markdown.contains("main content"),
            "Body content should be present"
        );
        assert!(
            markdown.contains("All Rights Reserved"),
            "Master page footer should be present"
        );
    }

    /// Test character styles and inline formatting (bold, italic, underline)
    #[test]
    fn test_idml_character_styles_inline_formatting() {
        use docling_adobe::idml::types::{IdmlDocument, Paragraph, Story};

        let mut doc = IdmlDocument::new();
        let mut story = Story::new("u1000".to_string());

        // IDML supports rich character formatting via CharacterStyleRange
        // Simulating common markdown-like formatting
        story.add_paragraph(Paragraph::new(
            "This text has **bold**, *italic*, and _underlined_ formatting.".to_string(),
        ));
        story.add_paragraph(Paragraph::new(
            "You can also have ***bold italic*** and `monospace code`.".to_string(),
        ));
        story.add_paragraph(Paragraph::new(
            "Character styles in IDML: [Emphasis]emphasized text[/Emphasis].".to_string(),
        ));

        doc.add_story(story);

        let markdown = IdmlSerializer::to_markdown(&doc);

        // Verify formatting markers are preserved
        assert!(
            markdown.contains("bold"),
            "Bold text marker should be preserved"
        );
        assert!(
            markdown.contains("italic"),
            "Italic text marker should be preserved"
        );
        assert!(
            markdown.contains("underlined"),
            "Underlined text marker should be preserved"
        );
        assert!(
            markdown.contains("monospace"),
            "Monospace text marker should be preserved"
        );
        assert!(
            markdown.contains("emphasized"),
            "Emphasized text marker should be preserved"
        );
    }

    /// Test text wrapping around images and objects (common in magazine layouts)
    #[test]
    fn test_idml_text_wrap_around_objects() {
        use docling_adobe::idml::types::{IdmlDocument, Paragraph, Story};

        let mut doc = IdmlDocument::new();
        let mut story = Story::new("u1000".to_string());

        // Text before image
        story.add_paragraph(Paragraph::new(
            "This paragraph flows around an image.".to_string(),
        ));

        // Image placeholder (real IDML has Rectangle/Image elements with wrap settings)
        story.add_paragraph(Paragraph::new(
            "[IMAGE: product_photo.jpg | width=300px | wrap=right]".to_string(),
        ));

        // Text wrapping around image
        story.add_paragraph(Paragraph::new(
            "The text wraps around the image on the left side, creating a natural flow. "
                .to_string(),
        ));
        story.add_paragraph(Paragraph::new(
            "This is common in magazine and brochure layouts.".to_string(),
        ));

        // Text after image clears
        story.add_paragraph(Paragraph::new(
            "This paragraph appears below the image with normal flow.".to_string(),
        ));

        doc.add_story(story);

        let markdown = IdmlSerializer::to_markdown(&doc);

        // Verify content around image placeholder
        assert!(
            markdown.contains("flows around"),
            "Text before image should be present"
        );
        assert!(
            markdown.contains("product_photo.jpg"),
            "Image placeholder reference should be present"
        );
        assert!(
            markdown.contains("wraps around"),
            "Text wrapping around image should be present"
        );
        assert!(
            markdown.contains("below the image"),
            "Text after image should be present"
        );
    }

    /// Test conditional text for multi-variant publishing
    #[test]
    fn test_idml_conditional_text_variants() {
        use docling_adobe::idml::types::{IdmlDocument, Paragraph, Story};

        let mut doc = IdmlDocument::new();
        let mut story = Story::new("u1000".to_string());

        // Conditional text is used for generating multiple document variants
        // (e.g., print vs web, different language editions, etc.)
        story.add_paragraph(Paragraph::new("Welcome to our product guide.".to_string()));

        // Conditional content (typically has condition tags in IDML XML)
        story.add_paragraph(Paragraph::new(
            "[Condition:Print]For print customers: Call 1-800-SUPPORT[/Condition:Print]"
                .to_string(),
        ));
        story.add_paragraph(Paragraph::new(
            "[Condition:Web]For online customers: Visit support.example.com[/Condition:Web]"
                .to_string(),
        ));

        story.add_paragraph(Paragraph::new(
            "Thank you for choosing our product.".to_string(),
        ));

        doc.add_story(story);

        let markdown = IdmlSerializer::to_markdown(&doc);

        // Verify conditional text markers are present
        assert!(
            markdown.contains("Welcome to our product"),
            "Common intro text should be present"
        );
        assert!(
            markdown.contains("Print") || markdown.contains("1-800-SUPPORT"),
            "Print variant content should be present"
        );
        assert!(
            markdown.contains("Web") || markdown.contains("support.example.com"),
            "Web variant content should be present"
        );
        assert!(
            markdown.contains("Thank you"),
            "Common outro text should be present"
        );
    }

    /// Test anchored objects (inline images and frames)
    #[test]
    fn test_idml_anchored_objects() {
        use docling_adobe::idml::types::{IdmlDocument, Paragraph, Story};

        let mut doc = IdmlDocument::new();
        let mut story = Story::new("u1000".to_string());

        // Text with inline anchored objects
        story.add_paragraph(Paragraph::new(
            "InDesign supports anchored objects that flow with text.".to_string(),
        ));

        // Inline image (anchored to text position)
        story.add_paragraph(Paragraph::new(
            "Here is an inline icon [ANCHOR:icon_checkmark.svg] in the text.".to_string(),
        ));

        // Anchored frame with caption
        story.add_paragraph(Paragraph::new("[ANCHOR:Frame]".to_string()));
        story.add_paragraph(Paragraph::new(
            "Figure 1: Product diagram showing key components.".to_string(),
        ));
        story.add_paragraph(Paragraph::new("[/ANCHOR:Frame]".to_string()));

        // Text continues after anchored object
        story.add_paragraph(Paragraph::new(
            "The text continues normally after the anchored frame.".to_string(),
        ));

        doc.add_story(story);

        let markdown = IdmlSerializer::to_markdown(&doc);

        // Verify anchored objects are represented
        assert!(
            markdown.contains("anchored objects"),
            "Anchored objects description should be present"
        );
        assert!(
            markdown.contains("icon_checkmark.svg") || markdown.contains("inline icon"),
            "Inline anchored icon reference should be present"
        );
        assert!(
            markdown.contains("Figure 1"),
            "Anchored frame figure label should be present"
        );
        assert!(
            markdown.contains("Product diagram"),
            "Anchored frame caption should be present"
        );
        assert!(
            markdown.contains("continues normally"),
            "Text after anchored frame should be present"
        );
    }
}
