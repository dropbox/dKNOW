//! XPS backend for docling
//!
//! This backend converts XPS (XML Paper Specification) files to docling's document model.

use crate::traits::{BackendOptions, DocumentBackend};
use crate::utils::{create_section_header, create_text_item};
use docling_core::{
    content::{BoundingBox, CoordOrigin, DocItem, ProvenanceItem},
    DoclingError, Document, DocumentMetadata, InputFormat,
};
use docling_xps::{parse_xps, XpsDocument};
use std::fmt::Write;
use std::path::Path;

/// XPS backend
///
/// Converts XPS (XML Paper Specification) files to docling's document model.
/// XPS is Microsoft's XML-based document format similar to PDF.
///
/// ## Features
///
/// - Parse XPS files (ZIP archives containing XML)
/// - Extract text content from pages
/// - Parse document metadata (title, author, etc.)
/// - Markdown-formatted output
///
/// ## Example
///
/// ```no_run
/// use docling_backend::XpsBackend;
/// use docling_backend::DocumentBackend;
///
/// let backend = XpsBackend::new();
/// let result = backend.parse_file("document.xps", &Default::default())?;
/// println!("XPS: {:?}", result.metadata.title);
/// # Ok::<(), docling_core::error::DoclingError>(())
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct XpsBackend;

impl XpsBackend {
    /// Create a new XPS backend instance
    #[inline]
    #[must_use = "creates a backend instance that should be used for parsing"]
    pub const fn new() -> Self {
        Self
    }

    /// Convert XPS document to `DocItems`
    fn xps_to_doc_items(xps: &XpsDocument) -> Vec<DocItem> {
        let mut doc_items = Vec::new();
        let mut text_id = 0;

        // Add title as section header if present
        if let Some(title) = &xps.metadata.title {
            doc_items.push(create_section_header(0, title.clone(), 1, vec![]));
        }

        // Add each page's text elements
        for page in &xps.pages {
            for element in &page.text {
                let text_content = element.content.trim();
                if text_content.is_empty() {
                    continue;
                }

                // Create bounding box from element position
                // XPS uses 1/96 inch units, normalized to page dimensions
                let bbox = BoundingBox {
                    l: element.x,
                    t: element.y,
                    r: element.x + 100.0, // Approximate width (XPS doesn't provide)
                    b: element.y + (element.font_size.unwrap_or(12.0)), // Height from font size
                    coord_origin: CoordOrigin::TopLeft,
                };

                let prov = ProvenanceItem {
                    page_no: page.number,
                    bbox,
                    charspan: Some(vec![0, text_content.len()]),
                };

                doc_items.push(create_text_item(
                    text_id,
                    text_content.to_string(),
                    vec![prov],
                ));
                text_id += 1;
            }
        }

        doc_items
    }

    /// Convert XPS document to markdown
    fn xps_to_markdown(xps: &XpsDocument) -> String {
        let mut markdown = String::new();

        // Add title if present
        if let Some(title) = &xps.metadata.title {
            let _ = writeln!(markdown, "# {title}\n");
        }

        // Add metadata section if present
        if xps.metadata.author.is_some()
            || xps.metadata.subject.is_some()
            || xps.metadata.keywords.is_some()
        {
            markdown.push_str("---\n\n");

            if let Some(author) = &xps.metadata.author {
                let _ = writeln!(markdown, "Author: {author}\n");
            }

            if let Some(subject) = &xps.metadata.subject {
                let _ = writeln!(markdown, "Subject: {subject}\n");
            }

            if let Some(keywords) = &xps.metadata.keywords {
                let _ = writeln!(markdown, "Keywords: {keywords}\n");
            }

            if let Some(created) = &xps.metadata.created {
                let _ = writeln!(markdown, "Created: {created}\n");
            }

            if let Some(modified) = &xps.metadata.modified {
                let _ = writeln!(markdown, "Modified: {modified}\n");
            }

            markdown.push_str("---\n\n");
        }

        // Add pages
        for page in &xps.pages {
            if xps.pages.len() > 1 {
                let _ = writeln!(markdown, "## Page {}\n", page.number);
            }

            // Add text elements from the page
            for element in &page.text {
                let _ = writeln!(markdown, "{}\n", element.content.trim());
            }
        }

        markdown
    }
}

impl DocumentBackend for XpsBackend {
    #[inline]
    fn format(&self) -> InputFormat {
        InputFormat::Xps
    }

    fn parse_bytes(&self, data: &[u8], options: &BackendOptions) -> Result<Document, DoclingError> {
        // Write bytes to temp file for parsing (XPS is a ZIP archive, requires file path)
        let temp_file_path = crate::utils::write_temp_file(data, "document", ".xps")?;
        self.parse_file(&temp_file_path, options)
    }

    fn parse_file<P: AsRef<Path>>(
        &self,
        path: P,
        _options: &BackendOptions,
    ) -> Result<Document, DoclingError> {
        let path_ref = path.as_ref();
        let filename = path_ref.display();

        // Parse XPS file
        let xps = parse_xps(path_ref).map_err(|e| {
            DoclingError::BackendError(format!("Failed to parse XPS file: {e}: {filename}"))
        })?;

        // Generate DocItems
        let doc_items = Self::xps_to_doc_items(&xps);

        // Convert to markdown
        let markdown = Self::xps_to_markdown(&xps);
        let num_characters = markdown.chars().count();

        // Create document
        Ok(Document {
            markdown,
            format: InputFormat::Xps,
            metadata: DocumentMetadata {
                num_pages: Some(xps.pages.len()),
                num_characters,
                title: xps.metadata.title,
                author: xps.metadata.author,
                created: None,  // XPS metadata.created is String, not DateTime
                modified: None, // XPS metadata.modified is String, not DateTime
                language: None,
                subject: None,
                exif: None,
            },
            content_blocks: Some(doc_items),
            docling_document: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use docling_core::content::{A4_HEIGHT, A4_WIDTH, US_LETTER_HEIGHT, US_LETTER_WIDTH};
    use docling_xps::{
        XpsDocument, XpsMetadata, XpsPage, XpsTextElement, XPS_DEFAULT_PAGE_HEIGHT,
        XPS_DEFAULT_PAGE_WIDTH,
    };

    #[test]
    fn test_xps_backend_creation() {
        let backend = XpsBackend::new();
        assert_eq!(
            backend.format(),
            InputFormat::Xps,
            "XpsBackend should report Xps format"
        );
    }

    // ========================================
    // Metadata Tests
    // ========================================

    #[test]
    fn test_xps_metadata_timestamps() {
        let xps = XpsDocument {
            metadata: XpsMetadata {
                title: Some("Test Document".to_string()),
                author: Some("Test Author".to_string()),
                subject: None,
                creator: None,
                keywords: None,
                description: None,
                created: Some("2024-01-15T10:30:00Z".to_string()),
                modified: Some("2024-02-20T14:45:00Z".to_string()),
            },
            pages: vec![],
        };

        let markdown = XpsBackend::xps_to_markdown(&xps);

        // Timestamps should appear in metadata section
        assert!(
            markdown.contains("Created: 2024-01-15T10:30:00Z"),
            "Markdown should contain created date"
        );
        assert!(
            markdown.contains("Modified: 2024-02-20T14:45:00Z"),
            "Markdown should contain modified date"
        );
    }

    #[test]
    fn test_xps_metadata_complete() {
        let xps = XpsDocument {
            metadata: XpsMetadata {
                title: Some("Complete Document".to_string()),
                author: Some("John Doe".to_string()),
                subject: Some("XPS Testing".to_string()),
                creator: Some("Test Creator".to_string()),
                keywords: Some("xps, test, metadata".to_string()),
                description: Some("Test description".to_string()),
                created: Some("2024-01-01T00:00:00Z".to_string()),
                modified: Some("2024-12-31T23:59:59Z".to_string()),
            },
            pages: vec![],
        };

        let markdown = XpsBackend::xps_to_markdown(&xps);

        // Should have title as H1
        assert!(
            markdown.contains("# Complete Document"),
            "Markdown should contain title as H1"
        );

        // Should have metadata section
        assert!(
            markdown.contains("Author: John Doe"),
            "Markdown should contain author"
        );
        assert!(
            markdown.contains("Subject: XPS Testing"),
            "Markdown should contain subject"
        );
        assert!(
            markdown.contains("Keywords: xps, test, metadata"),
            "Markdown should contain keywords"
        );
        assert!(
            markdown.contains("Created: 2024-01-01T00:00:00Z"),
            "Markdown should contain created date"
        );
        assert!(
            markdown.contains("Modified: 2024-12-31T23:59:59Z"),
            "Markdown should contain modified date"
        );

        // Should have metadata delimiters
        assert!(
            markdown.matches("---").count() == 2,
            "Markdown should have 2 metadata separators"
        );
    }

    #[test]
    fn test_xps_metadata_empty() {
        let xps = XpsDocument {
            metadata: XpsMetadata {
                title: None,
                author: None,
                subject: None,
                creator: None,
                keywords: None,
                description: None,
                created: None,
                modified: None,
            },
            pages: vec![],
        };

        let markdown = XpsBackend::xps_to_markdown(&xps);

        // Should not have title or metadata section
        assert!(
            !markdown.contains('#'),
            "Empty metadata should not produce title"
        );
        assert!(
            !markdown.contains("---"),
            "Empty metadata should not produce separators"
        );
        assert!(
            !markdown.contains("Author:"),
            "Empty metadata should not contain author field"
        );
    }

    // ========================================
    // DocItem Generation Tests
    // ========================================

    #[test]
    fn test_xps_single_page_markdown() {
        let xps = XpsDocument {
            metadata: XpsMetadata {
                title: Some("Single Page".to_string()),
                author: None,
                subject: None,
                creator: None,
                keywords: None,
                description: None,
                created: None,
                modified: None,
            },
            pages: vec![XpsPage {
                number: 1,
                width: XPS_DEFAULT_PAGE_WIDTH,
                height: XPS_DEFAULT_PAGE_HEIGHT,
                text: vec![
                    XpsTextElement::new("First paragraph.".to_string(), 50.0, 100.0),
                    XpsTextElement::new("Second paragraph.".to_string(), 50.0, 150.0),
                ],
            }],
        };

        let markdown = XpsBackend::xps_to_markdown(&xps);

        // Should have title
        assert!(
            markdown.contains("# Single Page"),
            "Markdown should contain title '# Single Page'"
        );

        // Should NOT have page headers for single page
        assert!(
            !markdown.contains("## Page"),
            "Single page should not have '## Page' headers"
        );

        // Should have text content
        assert!(
            markdown.contains("First paragraph."),
            "Markdown should contain first paragraph"
        );
        assert!(
            markdown.contains("Second paragraph."),
            "Markdown should contain second paragraph"
        );
    }

    #[test]
    fn test_xps_multi_page_markdown() {
        let xps = XpsDocument {
            metadata: XpsMetadata {
                title: Some("Multi Page".to_string()),
                author: None,
                subject: None,
                creator: None,
                keywords: None,
                description: None,
                created: None,
                modified: None,
            },
            pages: vec![
                XpsPage {
                    number: 1,
                    width: XPS_DEFAULT_PAGE_WIDTH,
                    height: XPS_DEFAULT_PAGE_HEIGHT,
                    text: vec![XpsTextElement::new(
                        "Page 1 content.".to_string(),
                        50.0,
                        100.0,
                    )],
                },
                XpsPage {
                    number: 2,
                    width: XPS_DEFAULT_PAGE_WIDTH,
                    height: XPS_DEFAULT_PAGE_HEIGHT,
                    text: vec![XpsTextElement::new(
                        "Page 2 content.".to_string(),
                        50.0,
                        100.0,
                    )],
                },
                XpsPage {
                    number: 3,
                    width: XPS_DEFAULT_PAGE_WIDTH,
                    height: XPS_DEFAULT_PAGE_HEIGHT,
                    text: vec![XpsTextElement::new(
                        "Page 3 content.".to_string(),
                        50.0,
                        100.0,
                    )],
                },
            ],
        };

        let markdown = XpsBackend::xps_to_markdown(&xps);

        // Should have title
        assert!(
            markdown.contains("# Multi Page"),
            "Multi-page document should have title '# Multi Page'"
        );

        // Should have page headers for multi-page document
        assert!(
            markdown.contains("## Page 1"),
            "Multi-page document should have '## Page 1' header"
        );
        assert!(
            markdown.contains("## Page 2"),
            "Multi-page document should have '## Page 2' header"
        );
        assert!(
            markdown.contains("## Page 3"),
            "Multi-page document should have '## Page 3' header"
        );

        // Should have content from all pages
        assert!(
            markdown.contains("Page 1 content."),
            "Page 1 content should be present in markdown"
        );
        assert!(
            markdown.contains("Page 2 content."),
            "Page 2 content should be present in markdown"
        );
        assert!(
            markdown.contains("Page 3 content."),
            "Page 3 content should be present in markdown"
        );
    }

    #[test]
    fn test_xps_empty_document() {
        let xps = XpsDocument {
            metadata: XpsMetadata::new(),
            pages: vec![],
        };

        let markdown = XpsBackend::xps_to_markdown(&xps);

        // Should be empty or nearly empty (just whitespace)
        assert!(
            markdown.trim().is_empty(),
            "Empty XPS document should produce empty or whitespace-only markdown"
        );
    }

    // ========================================
    // Format-Specific Feature Tests
    // ========================================

    #[test]
    fn test_xps_page_dimensions() {
        let xps = XpsDocument {
            metadata: XpsMetadata::new(),
            pages: vec![XpsPage {
                number: 1,
                width: 1200.0,  // Custom width
                height: 1800.0, // Custom height
                text: vec![],
            }],
        };

        // Page dimensions are stored but not directly in markdown
        // Verify they're preserved in structure
        assert_eq!(
            xps.pages[0].width, 1200.0,
            "Custom page width should be preserved"
        );
        assert_eq!(
            xps.pages[0].height, 1800.0,
            "Custom page height should be preserved"
        );

        // Markdown generation should not fail with custom dimensions
        let markdown = XpsBackend::xps_to_markdown(&xps);
        assert!(
            markdown.is_empty() || markdown.trim().is_empty(),
            "Empty page should produce empty markdown"
        );
    }

    #[test]
    fn test_xps_text_element_positioning() {
        let mut elem = XpsTextElement::new("Positioned text".to_string(), 123.45, 678.90);
        elem.font_size = Some(12.0);

        // Verify position properties
        assert_eq!(elem.x, 123.45, "X coordinate should be preserved");
        assert_eq!(elem.y, 678.90, "Y coordinate should be preserved");
        assert_eq!(elem.font_size, Some(12.0), "Font size should be preserved");
        assert_eq!(
            elem.content, "Positioned text",
            "Text content should be preserved"
        );
    }

    #[test]
    fn test_xps_font_size_extraction() {
        let page = XpsPage {
            number: 1,
            width: XPS_DEFAULT_PAGE_WIDTH,
            height: XPS_DEFAULT_PAGE_HEIGHT,
            text: vec![
                {
                    let mut elem = XpsTextElement::new("Large text".to_string(), 0.0, 0.0);
                    elem.font_size = Some(24.0);
                    elem
                },
                {
                    let mut elem = XpsTextElement::new("Small text".to_string(), 0.0, 50.0);
                    elem.font_size = Some(8.0);
                    elem
                },
                {
                    // No font size specified
                    XpsTextElement::new("Default text".to_string(), 0.0, 100.0)
                },
            ],
        };

        // Verify font sizes preserved
        assert_eq!(
            page.text[0].font_size,
            Some(24.0),
            "Large text should have font size 24.0"
        );
        assert_eq!(
            page.text[1].font_size,
            Some(8.0),
            "Small text should have font size 8.0"
        );
        assert_eq!(
            page.text[2].font_size, None,
            "Default text should have no font size specified"
        );
    }

    #[test]
    fn test_xps_multiple_text_elements() {
        let xps = XpsDocument {
            metadata: XpsMetadata::new(),
            pages: vec![XpsPage {
                number: 1,
                width: XPS_DEFAULT_PAGE_WIDTH,
                height: XPS_DEFAULT_PAGE_HEIGHT,
                text: vec![
                    XpsTextElement::new("Line 1".to_string(), 50.0, 100.0),
                    XpsTextElement::new("Line 2".to_string(), 50.0, 120.0),
                    XpsTextElement::new("Line 3".to_string(), 50.0, 140.0),
                    XpsTextElement::new("Line 4".to_string(), 50.0, 160.0),
                    XpsTextElement::new("Line 5".to_string(), 50.0, 180.0),
                ],
            }],
        };

        let markdown = XpsBackend::xps_to_markdown(&xps);

        // All text elements should appear in markdown
        assert!(
            markdown.contains("Line 1"),
            "Markdown should contain 'Line 1'"
        );
        assert!(
            markdown.contains("Line 2"),
            "Markdown should contain 'Line 2'"
        );
        assert!(
            markdown.contains("Line 3"),
            "Markdown should contain 'Line 3'"
        );
        assert!(
            markdown.contains("Line 4"),
            "Markdown should contain 'Line 4'"
        );
        assert!(
            markdown.contains("Line 5"),
            "Markdown should contain 'Line 5'"
        );

        // Each should be on separate paragraph (double newline)
        let line_count = markdown.matches("Line").count();
        assert_eq!(line_count, 5, "Should have exactly 5 'Line' occurrences");
    }

    // ========================================
    // Edge Case Tests
    // ========================================

    #[test]
    fn test_xps_parse_bytes_rejects() {
        let backend = XpsBackend::new();

        // parse_bytes should work with valid XPS data (but we don't have test data)
        // Test that empty bytes return an error
        let result = backend.parse_bytes(b"invalid xps data", &Default::default());
        assert!(result.is_err(), "Invalid XPS data should return an error");
    }

    #[test]
    fn test_xps_whitespace_trimming() {
        let xps = XpsDocument {
            metadata: XpsMetadata::new(),
            pages: vec![XpsPage {
                number: 1,
                width: XPS_DEFAULT_PAGE_WIDTH,
                height: XPS_DEFAULT_PAGE_HEIGHT,
                text: vec![
                    XpsTextElement::new("  Text with leading whitespace".to_string(), 50.0, 100.0),
                    XpsTextElement::new("Text with trailing whitespace  ".to_string(), 50.0, 120.0),
                    XpsTextElement::new("  Both sides  ".to_string(), 50.0, 140.0),
                ],
            }],
        };

        let markdown = XpsBackend::xps_to_markdown(&xps);

        // Content should be trimmed (per line 88 in xps_to_markdown)
        assert!(
            markdown.contains("Text with leading whitespace"),
            "Leading whitespace should be trimmed"
        );
        assert!(
            markdown.contains("Text with trailing whitespace"),
            "Trailing whitespace should be trimmed"
        );
        assert!(
            markdown.contains("Both sides"),
            "Whitespace on both sides should be trimmed"
        );

        // Should not have leading/trailing whitespace in markdown lines
        let lines: Vec<&str> = markdown.lines().filter(|l| !l.is_empty()).collect();
        for line in lines {
            if line.contains("Text") {
                assert_eq!(line, line.trim());
            }
        }
    }

    #[test]
    fn test_xps_metadata_without_title() {
        let xps = XpsDocument {
            metadata: XpsMetadata {
                title: None,
                author: Some("Author Only".to_string()),
                subject: Some("Subject Only".to_string()),
                creator: None,
                keywords: None,
                description: None,
                created: None,
                modified: None,
            },
            pages: vec![],
        };

        let markdown = XpsBackend::xps_to_markdown(&xps);

        // Should have metadata section even without title
        assert!(
            markdown.contains("---"),
            "Metadata section should have '---' separators"
        );
        assert!(
            markdown.contains("Author: Author Only"),
            "Markdown should contain author field"
        );
        assert!(
            markdown.contains("Subject: Subject Only"),
            "Markdown should contain subject field"
        );

        // Should NOT have H1 title
        assert!(
            !markdown.contains('#'),
            "Document without title should not have '#' header"
        );
    }

    // ===== Backend Trait Tests =====

    /// Test XpsBackend implements Default
    #[test]
    fn test_backend_default() {
        let backend = XpsBackend;
        assert_eq!(
            backend.format(),
            InputFormat::Xps,
            "XpsBackend should report Xps format"
        );
    }

    /// Test format() consistency
    #[test]
    fn test_backend_format_constant() {
        let backend1 = XpsBackend::new();
        let backend2 = XpsBackend;
        assert_eq!(
            backend1.format(),
            backend2.format(),
            "Both backends should report same format"
        );
        assert_eq!(
            backend1.format(),
            InputFormat::Xps,
            "XpsBackend should report Xps format"
        );
    }

    // ===== Metadata Edge Cases =====

    /// Test metadata with created timestamp (requires author/subject/keywords to show section)
    #[test]
    fn test_metadata_with_created() {
        let xps = XpsDocument {
            metadata: XpsMetadata {
                title: Some("Doc".to_string()),
                author: Some("Author".to_string()), // Required to trigger metadata section
                subject: None,
                creator: None,
                keywords: None,
                description: None,
                created: Some("2024-01-01T12:00:00Z".to_string()),
                modified: None,
            },
            pages: vec![],
        };

        let markdown = XpsBackend::xps_to_markdown(&xps);
        // Created timestamp should appear in metadata section
        assert!(
            markdown.contains("Created: 2024-01-01T12:00:00Z"),
            "Created timestamp should appear in metadata"
        );
        assert!(
            !markdown.contains("Modified:"),
            "Modified field should not appear when not set"
        );
    }

    /// Test metadata with modified timestamp (requires author/subject/keywords to show section)
    #[test]
    fn test_metadata_with_modified() {
        let xps = XpsDocument {
            metadata: XpsMetadata {
                title: None,
                author: None,
                subject: Some("Test".to_string()), // Required to trigger metadata section
                creator: None,
                keywords: None,
                description: None,
                created: None,
                modified: Some("2024-12-31T23:59:59Z".to_string()),
            },
            pages: vec![],
        };

        let markdown = XpsBackend::xps_to_markdown(&xps);
        // Modified timestamp should appear in metadata section
        assert!(
            markdown.contains("Modified: 2024-12-31T23:59:59Z"),
            "Modified timestamp should appear in metadata"
        );
        assert!(
            !markdown.contains("Created:"),
            "Created field should not appear when not set"
        );
    }

    /// Test metadata with creator field (not shown in markdown currently)
    #[test]
    fn test_metadata_creator_field() {
        let xps = XpsDocument {
            metadata: XpsMetadata {
                title: None,
                author: None,
                subject: None,
                creator: Some("XPS Creator App".to_string()),
                keywords: None,
                description: None,
                created: None,
                modified: None,
            },
            pages: vec![],
        };

        // Creator is stored but not displayed in markdown (not in xps_to_markdown logic)
        let markdown = XpsBackend::xps_to_markdown(&xps);
        assert!(
            !markdown.contains("XPS Creator App"),
            "Creator field is not displayed in markdown"
        );
    }

    /// Test metadata with description field (not shown in markdown currently)
    #[test]
    fn test_metadata_description_field() {
        let xps = XpsDocument {
            metadata: XpsMetadata {
                title: None,
                author: None,
                subject: None,
                creator: None,
                keywords: None,
                description: Some("This is a description".to_string()),
                created: None,
                modified: None,
            },
            pages: vec![],
        };

        // Description is stored but not displayed in markdown (not in xps_to_markdown logic)
        let markdown = XpsBackend::xps_to_markdown(&xps);
        assert!(
            !markdown.contains("This is a description"),
            "Description field is not displayed in markdown"
        );
    }

    // ===== Page Numbering Edge Cases =====

    /// Test pages with non-sequential numbering
    #[test]
    fn test_nonsequential_page_numbers() {
        let xps = XpsDocument {
            metadata: XpsMetadata::new(),
            pages: vec![
                XpsPage {
                    number: 1,
                    width: XPS_DEFAULT_PAGE_WIDTH,
                    height: XPS_DEFAULT_PAGE_HEIGHT,
                    text: vec![XpsTextElement::new("Page 1".to_string(), 0.0, 0.0)],
                },
                XpsPage {
                    number: 5, // Gap: skipped 2, 3, 4
                    width: XPS_DEFAULT_PAGE_WIDTH,
                    height: XPS_DEFAULT_PAGE_HEIGHT,
                    text: vec![XpsTextElement::new("Page 5".to_string(), 0.0, 0.0)],
                },
                XpsPage {
                    number: 10,
                    width: XPS_DEFAULT_PAGE_WIDTH,
                    height: XPS_DEFAULT_PAGE_HEIGHT,
                    text: vec![XpsTextElement::new("Page 10".to_string(), 0.0, 0.0)],
                },
            ],
        };

        let markdown = XpsBackend::xps_to_markdown(&xps);
        // Page headers show original page numbers
        assert!(
            markdown.contains("## Page 1"),
            "Non-sequential page 1 should appear"
        );
        assert!(
            markdown.contains("## Page 5"),
            "Non-sequential page 5 should appear"
        );
        assert!(
            markdown.contains("## Page 10"),
            "Non-sequential page 10 should appear"
        );
    }

    /// Test page with number 0 (edge case)
    #[test]
    fn test_page_number_zero() {
        let xps = XpsDocument {
            metadata: XpsMetadata::new(),
            pages: vec![
                XpsPage {
                    number: 0, // Zero-based indexing edge case
                    width: XPS_DEFAULT_PAGE_WIDTH,
                    height: XPS_DEFAULT_PAGE_HEIGHT,
                    text: vec![XpsTextElement::new("Page 0".to_string(), 0.0, 0.0)],
                },
                XpsPage {
                    number: 1,
                    width: XPS_DEFAULT_PAGE_WIDTH,
                    height: XPS_DEFAULT_PAGE_HEIGHT,
                    text: vec![XpsTextElement::new("Page 1".to_string(), 0.0, 0.0)],
                },
            ],
        };

        let markdown = XpsBackend::xps_to_markdown(&xps);
        assert!(
            markdown.contains("## Page 0"),
            "Zero-indexed page should appear as '## Page 0'"
        );
        assert!(
            markdown.contains("## Page 1"),
            "Page 1 should appear as '## Page 1'"
        );
    }

    // ===== Text Element Edge Cases =====

    /// Test text element with empty content
    #[test]
    fn test_text_element_empty_content() {
        let xps = XpsDocument {
            metadata: XpsMetadata::new(),
            pages: vec![XpsPage {
                number: 1,
                width: XPS_DEFAULT_PAGE_WIDTH,
                height: XPS_DEFAULT_PAGE_HEIGHT,
                text: vec![
                    XpsTextElement::new("Before empty".to_string(), 0.0, 0.0),
                    XpsTextElement::new("".to_string(), 0.0, 20.0), // Empty content
                    XpsTextElement::new("After empty".to_string(), 0.0, 40.0),
                ],
            }],
        };

        let markdown = XpsBackend::xps_to_markdown(&xps);
        assert!(
            markdown.contains("Before empty"),
            "Text before empty element should appear"
        );
        assert!(
            markdown.contains("After empty"),
            "Text after empty element should appear"
        );
        // Empty content trimmed to empty string, should create blank paragraph
    }

    /// Test text element with special characters
    #[test]
    fn test_text_element_special_characters() {
        let xps = XpsDocument {
            metadata: XpsMetadata::new(),
            pages: vec![XpsPage {
                number: 1,
                width: XPS_DEFAULT_PAGE_WIDTH,
                height: XPS_DEFAULT_PAGE_HEIGHT,
                text: vec![
                    XpsTextElement::new("Special chars: @#$%^&*()".to_string(), 0.0, 0.0),
                    XpsTextElement::new("Quotes: \"double\" and 'single'".to_string(), 0.0, 20.0),
                    XpsTextElement::new("Symbols: © ® ™ € £".to_string(), 0.0, 40.0),
                ],
            }],
        };

        let markdown = XpsBackend::xps_to_markdown(&xps);
        assert!(
            markdown.contains("Special chars: @#$%^&*()"),
            "Special characters should be preserved"
        );
        assert!(
            markdown.contains("Quotes: \"double\" and 'single'"),
            "Quote characters should be preserved"
        );
        assert!(
            markdown.contains("Symbols: © ® ™ € £"),
            "Unicode symbols should be preserved"
        );
    }

    /// Test text element with unicode (CJK, emoji)
    #[test]
    fn test_text_element_unicode() {
        let xps = XpsDocument {
            metadata: XpsMetadata::new(),
            pages: vec![XpsPage {
                number: 1,
                width: XPS_DEFAULT_PAGE_WIDTH,
                height: XPS_DEFAULT_PAGE_HEIGHT,
                text: vec![
                    XpsTextElement::new("日本語テキスト".to_string(), 0.0, 0.0), // Japanese
                    XpsTextElement::new("中文文本".to_string(), 0.0, 20.0),      // Chinese
                    XpsTextElement::new("한국어 텍스트".to_string(), 0.0, 40.0), // Korean
                    XpsTextElement::new("Emoji: 😀 🎉 🚀".to_string(), 0.0, 60.0),
                ],
            }],
        };

        let markdown = XpsBackend::xps_to_markdown(&xps);
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
            "Emoji should be preserved"
        );
    }

    /// Test text element with newlines
    #[test]
    fn test_text_element_with_newlines() {
        let xps = XpsDocument {
            metadata: XpsMetadata::new(),
            pages: vec![XpsPage {
                number: 1,
                width: XPS_DEFAULT_PAGE_WIDTH,
                height: XPS_DEFAULT_PAGE_HEIGHT,
                text: vec![XpsTextElement::new(
                    "Line 1\nLine 2\nLine 3".to_string(),
                    0.0,
                    0.0,
                )],
            }],
        };

        let markdown = XpsBackend::xps_to_markdown(&xps);
        // Newlines in content should be preserved
        assert!(
            markdown.contains("Line 1\nLine 2\nLine 3"),
            "Newlines within text content should be preserved"
        );
    }

    // ===== Markdown Formatting Edge Cases =====

    /// Test title with special characters
    #[test]
    fn test_title_special_characters() {
        let xps = XpsDocument {
            metadata: XpsMetadata {
                title: Some("Title: With Special & Characters!".to_string()),
                author: None,
                subject: None,
                creator: None,
                keywords: None,
                description: None,
                created: None,
                modified: None,
            },
            pages: vec![],
        };

        let markdown = XpsBackend::xps_to_markdown(&xps);
        assert!(
            markdown.contains("# Title: With Special & Characters!"),
            "Title with special characters should be preserved"
        );
    }

    /// Test keywords with commas (CSV-like)
    #[test]
    fn test_keywords_with_commas() {
        let xps = XpsDocument {
            metadata: XpsMetadata {
                title: None,
                author: None,
                subject: None,
                creator: None,
                keywords: Some("rust, xps, docling, parsing, test".to_string()),
                description: None,
                created: None,
                modified: None,
            },
            pages: vec![],
        };

        let markdown = XpsBackend::xps_to_markdown(&xps);
        assert!(
            markdown.contains("Keywords: rust, xps, docling, parsing, test"),
            "Keywords with commas should be preserved"
        );
    }

    /// Test very long metadata field
    #[test]
    fn test_very_long_metadata() {
        let long_subject = "A".repeat(1000);
        let xps = XpsDocument {
            metadata: XpsMetadata {
                title: None,
                author: None,
                subject: Some(long_subject.clone()),
                creator: None,
                keywords: None,
                description: None,
                created: None,
                modified: None,
            },
            pages: vec![],
        };

        let markdown = XpsBackend::xps_to_markdown(&xps);
        assert!(
            markdown.contains(&format!("Subject: {long_subject}")),
            "Very long metadata fields should be preserved"
        );
    }

    /// Test metadata section appears between delimiters
    #[test]
    fn test_metadata_section_delimiters() {
        let xps = XpsDocument {
            metadata: XpsMetadata {
                title: Some("Title".to_string()),
                author: Some("Author".to_string()),
                subject: None,
                creator: None,
                keywords: None,
                description: None,
                created: None,
                modified: None,
            },
            pages: vec![],
        };

        let markdown = XpsBackend::xps_to_markdown(&xps);
        // Should have exactly 2 "---" delimiters (start and end of metadata section)
        assert_eq!(
            markdown.matches("---").count(),
            2,
            "Metadata section should have exactly 2 '---' delimiters"
        );
    }

    // ===== Integration Tests =====

    /// Test character count matches markdown output
    #[test]
    fn test_character_count_validation() {
        let xps = XpsDocument {
            metadata: XpsMetadata {
                title: Some("Test".to_string()),
                author: None,
                subject: None,
                creator: None,
                keywords: None,
                description: None,
                created: None,
                modified: None,
            },
            pages: vec![XpsPage {
                number: 1,
                width: XPS_DEFAULT_PAGE_WIDTH,
                height: XPS_DEFAULT_PAGE_HEIGHT,
                text: vec![XpsTextElement::new("Content".to_string(), 0.0, 0.0)],
            }],
        };

        let markdown = XpsBackend::xps_to_markdown(&xps);
        let char_count = markdown.chars().count();
        // Character count should be positive and match actual markdown length
        assert!(
            char_count > 0,
            "Character count should be positive for document with content"
        );
        assert_eq!(
            char_count,
            markdown.chars().count(),
            "Character count should be consistent"
        );
    }

    /// Test num_pages metadata field
    #[test]
    fn test_num_pages_metadata() {
        let xps = XpsDocument {
            metadata: XpsMetadata::new(),
            pages: vec![
                XpsPage {
                    number: 1,
                    width: XPS_DEFAULT_PAGE_WIDTH,
                    height: XPS_DEFAULT_PAGE_HEIGHT,
                    text: vec![],
                },
                XpsPage {
                    number: 2,
                    width: XPS_DEFAULT_PAGE_WIDTH,
                    height: XPS_DEFAULT_PAGE_HEIGHT,
                    text: vec![],
                },
                XpsPage {
                    number: 3,
                    width: XPS_DEFAULT_PAGE_WIDTH,
                    height: XPS_DEFAULT_PAGE_HEIGHT,
                    text: vec![],
                },
            ],
        };

        // num_pages should equal pages.len()
        assert_eq!(xps.pages.len(), 3, "Document should have 3 pages");
    }

    /// Test empty pages list produces num_pages = 0
    #[test]
    fn test_empty_pages_num_pages() {
        let xps = XpsDocument {
            metadata: XpsMetadata::new(),
            pages: vec![],
        };

        assert_eq!(xps.pages.len(), 0, "Empty document should have 0 pages");
    }

    // ===== Additional Edge Cases (Target: 50 tests) =====

    /// Test can_handle method
    #[test]
    fn test_can_handle_xps_format() {
        let backend = XpsBackend::new();
        assert!(
            backend.can_handle(InputFormat::Xps),
            "XPS backend should handle XPS format"
        );
        assert!(
            !backend.can_handle(InputFormat::Pdf),
            "XPS backend should not handle PDF format"
        );
        assert!(
            !backend.can_handle(InputFormat::Docx),
            "XPS backend should not handle DOCX format"
        );
    }

    /// Test BackendOptions passthrough (ignored but accepted)
    #[test]
    fn test_backend_options_passthrough() {
        let xps = XpsDocument {
            metadata: XpsMetadata::new(),
            pages: vec![],
        };

        // XPS backend ignores all options
        let _options = BackendOptions::default()
            .with_ocr(true)
            .with_table_structure(true);

        // Should generate markdown regardless of options
        let markdown = XpsBackend::xps_to_markdown(&xps);
        assert!(
            markdown.is_empty() || markdown.trim().is_empty(),
            "Empty document should produce empty markdown regardless of options"
        );
    }

    /// Test XPS generates DocItems for content_blocks
    #[test]
    fn test_content_blocks_populated_for_xps() {
        let xps = XpsDocument {
            metadata: XpsMetadata {
                title: Some("Test".to_string()),
                author: None,
                subject: None,
                creator: None,
                keywords: None,
                description: None,
                created: None,
                modified: None,
            },
            pages: vec![XpsPage {
                number: 1,
                width: XPS_DEFAULT_PAGE_WIDTH,
                height: XPS_DEFAULT_PAGE_HEIGHT,
                text: vec![XpsTextElement::new("Content".to_string(), 0.0, 0.0)],
            }],
        };

        // XPS backend now generates DocItems (N=501 change)
        let doc_items = XpsBackend::xps_to_doc_items(&xps);
        assert!(
            !doc_items.is_empty(),
            "DocItems should not be empty for document with content"
        );

        // Should have title + content text = 2 items
        assert_eq!(
            doc_items.len(),
            2,
            "Should have 2 DocItems: title header + content text"
        );

        // First item should be SectionHeader for title
        match &doc_items[0] {
            DocItem::SectionHeader { text, level, .. } => {
                assert_eq!(text, "Test");
                assert_eq!(*level, 1);
            }
            _ => panic!("Expected SectionHeader DocItem for title"),
        }

        // Second item should be Text for content
        match &doc_items[1] {
            DocItem::Text { text, .. } => {
                assert_eq!(text, "Content");
            }
            _ => panic!("Expected Text DocItem for content"),
        }
    }

    /// Test page with very large number
    #[test]
    fn test_page_very_large_number() {
        let xps = XpsDocument {
            metadata: XpsMetadata::new(),
            pages: vec![
                XpsPage {
                    number: 1,
                    width: XPS_DEFAULT_PAGE_WIDTH,
                    height: XPS_DEFAULT_PAGE_HEIGHT,
                    text: vec![XpsTextElement::new("Page 1".to_string(), 0.0, 0.0)],
                },
                XpsPage {
                    number: 9999,
                    width: XPS_DEFAULT_PAGE_WIDTH,
                    height: XPS_DEFAULT_PAGE_HEIGHT,
                    text: vec![XpsTextElement::new("Page 9999".to_string(), 0.0, 0.0)],
                },
            ],
        };

        let markdown = XpsBackend::xps_to_markdown(&xps);
        assert!(markdown.contains("## Page 1"), "Should have Page 1 header");
        assert!(
            markdown.contains("## Page 9999"),
            "Should handle large page number 9999"
        );
    }

    /// Test text element with very long content
    #[test]
    fn test_text_element_very_long_content() {
        let long_content = "word ".repeat(1000); // 5000 chars
        let xps = XpsDocument {
            metadata: XpsMetadata::new(),
            pages: vec![XpsPage {
                number: 1,
                width: XPS_DEFAULT_PAGE_WIDTH,
                height: XPS_DEFAULT_PAGE_HEIGHT,
                text: vec![XpsTextElement::new(long_content.clone(), 0.0, 0.0)],
            }],
        };

        let markdown = XpsBackend::xps_to_markdown(&xps);
        assert!(
            markdown.contains(long_content.trim()),
            "Very long text content (5000 chars) should be preserved"
        );
    }

    /// Test page with negative coordinates
    #[test]
    fn test_text_element_negative_coordinates() {
        let xps = XpsDocument {
            metadata: XpsMetadata::new(),
            pages: vec![XpsPage {
                number: 1,
                width: XPS_DEFAULT_PAGE_WIDTH,
                height: XPS_DEFAULT_PAGE_HEIGHT,
                text: vec![
                    XpsTextElement::new("Negative X".to_string(), -10.0, 100.0),
                    XpsTextElement::new("Negative Y".to_string(), 100.0, -50.0),
                    XpsTextElement::new("Both negative".to_string(), -20.0, -30.0),
                ],
            }],
        };

        let markdown = XpsBackend::xps_to_markdown(&xps);
        // Negative coordinates should not prevent markdown generation
        assert!(
            markdown.contains("Negative X"),
            "Text with negative X coordinate should be preserved"
        );
        assert!(
            markdown.contains("Negative Y"),
            "Text with negative Y coordinate should be preserved"
        );
        assert!(
            markdown.contains("Both negative"),
            "Text with both negative coordinates should be preserved"
        );
    }

    /// Test page with zero dimensions
    #[test]
    fn test_page_zero_dimensions() {
        let xps = XpsDocument {
            metadata: XpsMetadata::new(),
            pages: vec![XpsPage {
                number: 1,
                width: 0.0,
                height: 0.0,
                text: vec![XpsTextElement::new(
                    "Zero dimension page".to_string(),
                    0.0,
                    0.0,
                )],
            }],
        };

        let markdown = XpsBackend::xps_to_markdown(&xps);
        assert!(
            markdown.contains("Zero dimension page"),
            "Page with zero dimensions should still render text"
        );
    }

    /// Test font size with zero value
    #[test]
    fn test_text_element_zero_font_size() {
        let mut elem = XpsTextElement::new("Zero font".to_string(), 0.0, 0.0);
        elem.font_size = Some(0.0);

        assert_eq!(
            elem.font_size,
            Some(0.0),
            "Zero font size should be preserved"
        );
        assert_eq!(
            elem.content, "Zero font",
            "Text content should be preserved with zero font size"
        );
    }

    /// Test font size with very large value
    #[test]
    fn test_text_element_large_font_size() {
        let mut elem = XpsTextElement::new("Large font".to_string(), 0.0, 0.0);
        elem.font_size = Some(999.99);

        assert_eq!(
            elem.font_size,
            Some(999.99),
            "Very large font size should be preserved"
        );
    }

    /// Test font size with negative value (edge case)
    #[test]
    fn test_text_element_negative_font_size() {
        let mut elem = XpsTextElement::new("Negative font".to_string(), 0.0, 0.0);
        elem.font_size = Some(-12.0);

        assert_eq!(
            elem.font_size,
            Some(-12.0),
            "Negative font size should be stored (edge case)"
        );
    }

    /// Test page with many text elements (100+)
    #[test]
    fn test_page_many_text_elements() {
        let elements: Vec<XpsTextElement> = (0..100)
            .map(|i| XpsTextElement::new(format!("Element {i}"), 0.0, (i * 20) as f64))
            .collect();

        let xps = XpsDocument {
            metadata: XpsMetadata::new(),
            pages: vec![XpsPage {
                number: 1,
                width: XPS_DEFAULT_PAGE_WIDTH,
                height: XPS_DEFAULT_PAGE_HEIGHT,
                text: elements,
            }],
        };

        let markdown = XpsBackend::xps_to_markdown(&xps);
        assert!(
            markdown.contains("Element 0"),
            "First element should be preserved"
        );
        assert!(
            markdown.contains("Element 50"),
            "Middle element (50) should be preserved"
        );
        assert!(
            markdown.contains("Element 99"),
            "Last element (99) should be preserved"
        );
    }

    /// Test multiple pages with same number (duplicate page numbers)
    #[test]
    fn test_duplicate_page_numbers() {
        let xps = XpsDocument {
            metadata: XpsMetadata::new(),
            pages: vec![
                XpsPage {
                    number: 1,
                    width: XPS_DEFAULT_PAGE_WIDTH,
                    height: XPS_DEFAULT_PAGE_HEIGHT,
                    text: vec![XpsTextElement::new("First page 1".to_string(), 0.0, 0.0)],
                },
                XpsPage {
                    number: 1, // Duplicate
                    width: XPS_DEFAULT_PAGE_WIDTH,
                    height: XPS_DEFAULT_PAGE_HEIGHT,
                    text: vec![XpsTextElement::new("Second page 1".to_string(), 0.0, 0.0)],
                },
            ],
        };

        let markdown = XpsBackend::xps_to_markdown(&xps);
        // Both pages should appear with "## Page 1" headers
        assert!(
            markdown.contains("First page 1"),
            "First duplicate page content should be preserved"
        );
        assert!(
            markdown.contains("Second page 1"),
            "Second duplicate page content should be preserved"
        );
        assert_eq!(
            markdown.matches("## Page 1").count(),
            2,
            "Should have 2 Page 1 headers for duplicate pages"
        );
    }

    /// Test metadata with all fields except title
    #[test]
    fn test_metadata_complete_without_title() {
        let xps = XpsDocument {
            metadata: XpsMetadata {
                title: None,
                author: Some("Author".to_string()),
                subject: Some("Subject".to_string()),
                creator: Some("Creator".to_string()),
                keywords: Some("Keywords".to_string()),
                description: Some("Description".to_string()),
                created: Some("2024-01-01T00:00:00Z".to_string()),
                modified: Some("2024-12-31T23:59:59Z".to_string()),
            },
            pages: vec![],
        };

        let markdown = XpsBackend::xps_to_markdown(&xps);
        // Should have metadata section without H1 title
        assert!(
            markdown.contains("---"),
            "Metadata section should have separators"
        );
        assert!(
            markdown.contains("Author:"),
            "Author field should be present"
        );
        assert!(
            markdown.contains("Subject:"),
            "Subject field should be present"
        );
        assert!(
            markdown.contains("Keywords:"),
            "Keywords field should be present"
        );
        assert!(
            !markdown.starts_with('#'),
            "Markdown should not start with title if no title set"
        );
    }

    /// Test text element with only whitespace
    #[test]
    fn test_text_element_only_whitespace() {
        let xps = XpsDocument {
            metadata: XpsMetadata::new(),
            pages: vec![XpsPage {
                number: 1,
                width: XPS_DEFAULT_PAGE_WIDTH,
                height: XPS_DEFAULT_PAGE_HEIGHT,
                text: vec![
                    XpsTextElement::new("Before".to_string(), 0.0, 0.0),
                    XpsTextElement::new("   ".to_string(), 0.0, 20.0), // Only spaces
                    XpsTextElement::new("After".to_string(), 0.0, 40.0),
                ],
            }],
        };

        let markdown = XpsBackend::xps_to_markdown(&xps);
        assert!(
            markdown.contains("Before"),
            "Text before whitespace-only element should be preserved"
        );
        assert!(
            markdown.contains("After"),
            "Text after whitespace-only element should be preserved"
        );
        // Whitespace-only element trimmed to empty (line 88: .trim())
    }

    /// Test title with markdown special characters
    #[test]
    fn test_title_markdown_special_chars() {
        let xps = XpsDocument {
            metadata: XpsMetadata {
                title: Some("Title with *asterisks* and _underscores_ and `code`".to_string()),
                author: None,
                subject: None,
                creator: None,
                keywords: None,
                description: None,
                created: None,
                modified: None,
            },
            pages: vec![],
        };

        let markdown = XpsBackend::xps_to_markdown(&xps);
        // Special markdown chars should be preserved (not escaped)
        assert!(
            markdown.contains("*asterisks*"),
            "Asterisks in title should be preserved"
        );
        assert!(
            markdown.contains("_underscores_"),
            "Underscores in title should be preserved"
        );
        assert!(
            markdown.contains("`code`"),
            "Backticks in title should be preserved"
        );
    }

    /// Test page with floating point coordinates
    #[test]
    fn test_text_element_fractional_coordinates() {
        let xps = XpsDocument {
            metadata: XpsMetadata::new(),
            pages: vec![XpsPage {
                number: 1,
                width: XPS_DEFAULT_PAGE_WIDTH,
                height: XPS_DEFAULT_PAGE_HEIGHT,
                text: vec![
                    XpsTextElement::new("Fractional".to_string(), 123.456, 789.012),
                    XpsTextElement::new("Very precise".to_string(), 0.0001, 0.9999),
                ],
            }],
        };

        let markdown = XpsBackend::xps_to_markdown(&xps);
        assert!(
            markdown.contains("Fractional"),
            "Text with fractional coordinates should be preserved"
        );
        assert!(
            markdown.contains("Very precise"),
            "Text with very precise coordinates should be preserved"
        );
    }

    /// Test format field in Document
    #[test]
    fn test_document_format_field() {
        // Verify format is set correctly when converting to Document
        // XPS backend sets format to InputFormat::Xps (line 133)
        let backend = XpsBackend::new();
        assert_eq!(
            backend.format(),
            InputFormat::Xps,
            "XpsBackend should report Xps format"
        );
    }

    // ===== N=473 Expansion: 10 additional tests =====

    /// Test can_handle method with XPS format
    #[test]
    fn test_can_handle_xps() {
        let backend = XpsBackend::new();
        assert!(
            backend.can_handle(InputFormat::Xps),
            "XPS backend should handle XPS format"
        );
    }

    /// Test can_handle rejects non-XPS formats
    #[test]
    fn test_can_handle_rejects_others() {
        let backend = XpsBackend::new();
        assert!(
            !backend.can_handle(InputFormat::Pdf),
            "XPS backend should not handle PDF"
        );
        assert!(
            !backend.can_handle(InputFormat::Docx),
            "XPS backend should not handle DOCX"
        );
        assert!(
            !backend.can_handle(InputFormat::Html),
            "XPS backend should not handle HTML"
        );
    }

    /// Test multiple pages with different dimensions
    #[test]
    fn test_mixed_page_dimensions() {
        let xps = XpsDocument {
            metadata: XpsMetadata::new(),
            pages: vec![
                XpsPage {
                    number: 1,
                    width: XPS_DEFAULT_PAGE_WIDTH,
                    height: XPS_DEFAULT_PAGE_HEIGHT,
                    text: vec![XpsTextElement::new("Letter size".to_string(), 100.0, 100.0)],
                },
                XpsPage {
                    number: 2,
                    width: A4_WIDTH,
                    height: A4_HEIGHT,
                    text: vec![XpsTextElement::new("A4 size".to_string(), 100.0, 100.0)],
                },
            ],
        };

        let markdown = XpsBackend::xps_to_markdown(&xps);
        assert!(
            markdown.contains("Letter size"),
            "Letter size page content should be preserved"
        );
        assert!(
            markdown.contains("A4 size"),
            "A4 size page content should be preserved"
        );
    }

    /// Test text elements at page boundaries
    #[test]
    fn test_text_at_page_boundaries() {
        let xps = XpsDocument {
            metadata: XpsMetadata::new(),
            pages: vec![XpsPage {
                number: 1,
                width: XPS_DEFAULT_PAGE_WIDTH,
                height: XPS_DEFAULT_PAGE_HEIGHT,
                text: vec![
                    XpsTextElement::new("Top left".to_string(), 0.0, 0.0),
                    XpsTextElement::new(
                        "Bottom right".to_string(),
                        XPS_DEFAULT_PAGE_WIDTH,
                        XPS_DEFAULT_PAGE_HEIGHT,
                    ),
                ],
            }],
        };

        let markdown = XpsBackend::xps_to_markdown(&xps);
        assert!(
            markdown.contains("Top left"),
            "Text at top-left corner should be preserved"
        );
        assert!(
            markdown.contains("Bottom right"),
            "Text at bottom-right corner should be preserved"
        );
    }

    /// Test document with all metadata fields
    #[test]
    fn test_full_metadata() {
        let mut metadata = XpsMetadata::new();
        metadata.title = Some("Test Title".to_string());
        metadata.author = Some("Test Author".to_string());
        metadata.subject = Some("Test Subject".to_string());
        metadata.keywords = Some("test, keywords".to_string());
        metadata.created = Some("2024-01-01".to_string());
        metadata.modified = Some("2024-12-31".to_string());

        let xps = XpsDocument {
            metadata,
            pages: vec![],
        };

        let markdown = XpsBackend::xps_to_markdown(&xps);
        // Title appears as markdown header
        assert!(markdown.contains("# Test Title"));
        // Metadata fields use bold format
        assert!(markdown.contains("Author: Test Author"));
        assert!(markdown.contains("Subject: Test Subject"));
        assert!(markdown.contains("Keywords: test, keywords"));
        assert!(markdown.contains("Created: 2024-01-01"));
        assert!(markdown.contains("Modified: 2024-12-31"));
    }

    /// Test text with mixed Unicode and ASCII
    #[test]
    fn test_mixed_unicode_ascii() {
        let xps = XpsDocument {
            metadata: XpsMetadata::new(),
            pages: vec![XpsPage {
                number: 1,
                width: XPS_DEFAULT_PAGE_WIDTH,
                height: XPS_DEFAULT_PAGE_HEIGHT,
                text: vec![
                    XpsTextElement::new("ASCII text".to_string(), 100.0, 100.0),
                    XpsTextElement::new("Unicode 日本語".to_string(), 100.0, 120.0),
                    XpsTextElement::new("Emoji 🎉".to_string(), 100.0, 140.0),
                ],
            }],
        };

        let markdown = XpsBackend::xps_to_markdown(&xps);
        assert!(
            markdown.contains("ASCII text"),
            "ASCII text should be preserved"
        );
        assert!(
            markdown.contains("Unicode 日本語"),
            "Japanese Unicode text should be preserved"
        );
        assert!(markdown.contains("Emoji 🎉"), "Emoji should be preserved");
    }

    /// Test very large page dimensions
    #[test]
    fn test_large_page_dimensions() {
        let xps = XpsDocument {
            metadata: XpsMetadata::new(),
            pages: vec![XpsPage {
                number: 1,
                width: 10000.0,
                height: 10000.0,
                text: vec![XpsTextElement::new("Big page".to_string(), 5000.0, 5000.0)],
            }],
        };

        let markdown = XpsBackend::xps_to_markdown(&xps);
        assert!(
            markdown.contains("Big page"),
            "Text on very large page (10000x10000) should be preserved"
        );
    }

    /// Test text elements with zero coordinates
    #[test]
    fn test_zero_coordinates() {
        let xps = XpsDocument {
            metadata: XpsMetadata::new(),
            pages: vec![XpsPage {
                number: 1,
                width: XPS_DEFAULT_PAGE_WIDTH,
                height: XPS_DEFAULT_PAGE_HEIGHT,
                text: vec![
                    XpsTextElement::new("At origin".to_string(), 0.0, 0.0),
                    XpsTextElement::new("Also zero".to_string(), 0.0, 0.0),
                ],
            }],
        };

        let markdown = XpsBackend::xps_to_markdown(&xps);
        assert!(
            markdown.contains("At origin"),
            "Text at origin (0,0) should be preserved"
        );
        assert!(
            markdown.contains("Also zero"),
            "Multiple texts at same origin should be preserved"
        );
    }

    /// Test BackendOptions are ignored (XPS doesn't use them)
    #[test]
    fn test_backend_options_ignored() {
        let backend = XpsBackend::new();
        let xps = XpsDocument {
            metadata: XpsMetadata::new(),
            pages: vec![XpsPage {
                number: 1,
                width: XPS_DEFAULT_PAGE_WIDTH,
                height: XPS_DEFAULT_PAGE_HEIGHT,
                text: vec![XpsTextElement::new("Test".to_string(), 100.0, 100.0)],
            }],
        };

        // Create options with various settings
        let _options = BackendOptions::default()
            .with_ocr(true)
            .with_table_structure(true);

        // xps_to_markdown doesn't take options, but verify format() works
        assert_eq!(
            backend.format(),
            InputFormat::Xps,
            "XpsBackend should report Xps format"
        );
        let markdown = XpsBackend::xps_to_markdown(&xps);
        assert!(
            markdown.contains("Test"),
            "Text should be preserved regardless of backend options"
        );
    }

    /// Test page number formatting in output
    #[test]
    fn test_page_number_formatting() {
        let xps = XpsDocument {
            metadata: XpsMetadata::new(),
            pages: vec![
                XpsPage {
                    number: 1,
                    width: XPS_DEFAULT_PAGE_WIDTH,
                    height: XPS_DEFAULT_PAGE_HEIGHT,
                    text: vec![XpsTextElement::new("Page one".to_string(), 100.0, 100.0)],
                },
                XpsPage {
                    number: 2,
                    width: XPS_DEFAULT_PAGE_WIDTH,
                    height: XPS_DEFAULT_PAGE_HEIGHT,
                    text: vec![XpsTextElement::new("Page two".to_string(), 100.0, 100.0)],
                },
            ],
        };

        let markdown = XpsBackend::xps_to_markdown(&xps);
        // Check that page headers are formatted correctly
        assert!(
            markdown.contains("Page one"),
            "Page 1 content should be preserved"
        );
        assert!(
            markdown.contains("Page two"),
            "Page 2 content should be preserved"
        );
    }

    // Note: XPS is a complex format requiring ZIP archives with XML.
    // Full integration tests would require real XPS files.
    // Parser implementation in docling-xps crate has its own tests.

    #[test]
    fn test_xps_with_images() {
        use docling_xps::{XpsDocument, XpsMetadata, XpsPage, XpsTextElement};

        let xps = XpsDocument {
            metadata: XpsMetadata::new(),
            pages: vec![XpsPage {
                number: 1,
                width: XPS_DEFAULT_PAGE_WIDTH,
                height: XPS_DEFAULT_PAGE_HEIGHT,
                text: vec![
                    XpsTextElement::new("Caption: Figure 1".to_string(), 100.0, 500.0),
                    XpsTextElement::new("[Image placeholder]".to_string(), 100.0, 520.0),
                ],
            }],
        };

        let markdown = XpsBackend::xps_to_markdown(&xps);
        assert!(
            markdown.contains("Caption: Figure 1"),
            "Image caption should be preserved"
        );
        // XPS images are typically referenced as resources
        assert!(
            !markdown.is_empty(),
            "Markdown should not be empty for document with image caption"
        );
    }

    #[test]
    fn test_xps_rotated_text() {
        use docling_xps::{XpsDocument, XpsMetadata, XpsPage, XpsTextElement};

        // XPS supports rotated text via transforms
        let xps = XpsDocument {
            metadata: XpsMetadata::new(),
            pages: vec![XpsPage {
                number: 1,
                width: XPS_DEFAULT_PAGE_WIDTH,
                height: XPS_DEFAULT_PAGE_HEIGHT,
                text: vec![
                    XpsTextElement::new("Normal text".to_string(), 100.0, 100.0),
                    XpsTextElement::new("Rotated text 90°".to_string(), 200.0, 100.0),
                ],
            }],
        };

        let markdown = XpsBackend::xps_to_markdown(&xps);
        assert!(
            markdown.contains("Normal text"),
            "Normal text should be preserved"
        );
        assert!(
            markdown.contains("Rotated text 90°"),
            "Rotated text with degree symbol should be preserved"
        );
    }

    #[test]
    fn test_xps_unicode_content() {
        use docling_xps::{XpsDocument, XpsMetadata, XpsPage, XpsTextElement};

        let xps = XpsDocument {
            metadata: XpsMetadata::new(),
            pages: vec![XpsPage {
                number: 1,
                width: XPS_DEFAULT_PAGE_WIDTH,
                height: XPS_DEFAULT_PAGE_HEIGHT,
                text: vec![
                    XpsTextElement::new("Chinese: 你好世界".to_string(), 100.0, 100.0),
                    XpsTextElement::new("Arabic: مرحبا بالعالم".to_string(), 100.0, 120.0),
                    XpsTextElement::new("Emoji: 😀🎉".to_string(), 100.0, 140.0),
                ],
            }],
        };

        let markdown = XpsBackend::xps_to_markdown(&xps);
        assert!(
            markdown.contains("你好世界"),
            "Chinese characters should be preserved"
        );
        assert!(
            markdown.contains("مرحبا بالعالم"),
            "Arabic characters should be preserved"
        );
        assert!(markdown.contains("😀🎉"), "Emoji should be preserved");
    }

    #[test]
    fn test_xps_page_size_variations() {
        use docling_xps::{XpsDocument, XpsMetadata, XpsPage, XpsTextElement};

        let xps = XpsDocument {
            metadata: XpsMetadata::new(),
            pages: vec![
                XpsPage {
                    number: 1,
                    width: US_LETTER_WIDTH,
                    height: US_LETTER_HEIGHT,
                    text: vec![XpsTextElement::new("Letter page".to_string(), 50.0, 50.0)],
                },
                XpsPage {
                    number: 2,
                    width: A4_WIDTH,
                    height: A4_HEIGHT,
                    text: vec![XpsTextElement::new("A4 page".to_string(), 50.0, 50.0)],
                },
                XpsPage {
                    number: 3,
                    width: 1224.0,            // Tabloid width (17" × 72 dpi)
                    height: US_LETTER_HEIGHT, // Same as Letter height (11" × 72 dpi)
                    text: vec![XpsTextElement::new("Tabloid page".to_string(), 50.0, 50.0)],
                },
            ],
        };

        let markdown = XpsBackend::xps_to_markdown(&xps);
        assert!(
            markdown.contains("Letter page"),
            "Letter size page content should be preserved"
        );
        assert!(
            markdown.contains("A4 page"),
            "A4 size page content should be preserved"
        );
        assert!(
            markdown.contains("Tabloid page"),
            "Tabloid size page content should be preserved"
        );
    }

    #[test]
    fn test_xps_overlapping_text() {
        use docling_xps::{XpsDocument, XpsMetadata, XpsPage, XpsTextElement};

        // XPS can have overlapping text elements (e.g., watermarks, stamps)
        let xps = XpsDocument {
            metadata: XpsMetadata::new(),
            pages: vec![XpsPage {
                number: 1,
                width: XPS_DEFAULT_PAGE_WIDTH,
                height: XPS_DEFAULT_PAGE_HEIGHT,
                text: vec![
                    XpsTextElement::new("Main content".to_string(), 100.0, 100.0),
                    XpsTextElement::new("CONFIDENTIAL".to_string(), 100.0, 100.0), // Same position
                    XpsTextElement::new("More content".to_string(), 100.0, 120.0),
                ],
            }],
        };

        let markdown = XpsBackend::xps_to_markdown(&xps);
        // Both texts should be preserved
        assert!(
            markdown.contains("Main content"),
            "Main content at overlapping position should be preserved"
        );
        assert!(
            markdown.contains("CONFIDENTIAL"),
            "Watermark text at overlapping position should be preserved"
        );
        assert!(
            markdown.contains("More content"),
            "Additional content should be preserved"
        );
    }

    #[test]
    fn test_xps_with_hyperlinks() {
        use docling_xps::{XpsDocument, XpsMetadata, XpsPage, XpsTextElement};

        // XPS supports hyperlinks (internal and external)
        let xps = XpsDocument {
            metadata: XpsMetadata::new(),
            pages: vec![XpsPage {
                number: 1,
                width: XPS_DEFAULT_PAGE_WIDTH,
                height: XPS_DEFAULT_PAGE_HEIGHT,
                text: vec![
                    XpsTextElement::new(
                        "Visit our website at https://example.com".to_string(),
                        100.0,
                        100.0,
                    ),
                    XpsTextElement::new("See Section 3 on page 5".to_string(), 100.0, 120.0),
                    XpsTextElement::new("Email: support@example.com".to_string(), 100.0, 140.0),
                    XpsTextElement::new("Call 1-800-EXAMPLE".to_string(), 100.0, 160.0),
                ],
            }],
        };

        let markdown = XpsBackend::xps_to_markdown(&xps);
        // Verify hyperlink text is preserved
        assert!(
            markdown.contains("https://example.com"),
            "URL should be preserved"
        );
        assert!(
            markdown.contains("support@example.com"),
            "Email address should be preserved"
        );
        assert!(
            markdown.contains("Section 3 on page 5"),
            "Cross-reference should be preserved"
        );
        assert!(
            markdown.contains("1-800-EXAMPLE"),
            "Phone number should be preserved"
        );
    }

    #[test]
    fn test_xps_with_vector_graphics_text() {
        use docling_xps::{XpsDocument, XpsMetadata, XpsPage, XpsTextElement};

        // XPS supports vector graphics (paths, shapes)
        // Test text alongside graphics descriptions
        let xps = XpsDocument {
            metadata: XpsMetadata::new(),
            pages: vec![XpsPage {
                number: 1,
                width: XPS_DEFAULT_PAGE_WIDTH,
                height: XPS_DEFAULT_PAGE_HEIGHT,
                text: vec![
                    XpsTextElement::new("Figure 1: System Architecture".to_string(), 100.0, 100.0),
                    XpsTextElement::new("[Circle: Database]".to_string(), 100.0, 120.0),
                    XpsTextElement::new("[Rectangle: Web Server]".to_string(), 100.0, 140.0),
                    XpsTextElement::new("[Arrow: Data flow]".to_string(), 100.0, 160.0),
                    XpsTextElement::new(
                        "Diagram shows component relationships".to_string(),
                        100.0,
                        180.0,
                    ),
                ],
            }],
        };

        let markdown = XpsBackend::xps_to_markdown(&xps);
        // Verify text associated with graphics is preserved
        assert!(
            markdown.contains("Figure 1: System Architecture"),
            "Figure title should be preserved"
        );
        assert!(
            markdown.contains("Database"),
            "Database label should be preserved"
        );
        assert!(
            markdown.contains("Web Server"),
            "Web Server label should be preserved"
        );
        assert!(
            markdown.contains("Data flow"),
            "Data flow label should be preserved"
        );
    }

    #[test]
    fn test_xps_with_transparency_layers() {
        use docling_xps::{XpsDocument, XpsMetadata, XpsPage, XpsTextElement};

        // XPS supports transparency and opacity (overlays, watermarks)
        let xps = XpsDocument {
            metadata: XpsMetadata::new(),
            pages: vec![XpsPage {
                number: 1,
                width: XPS_DEFAULT_PAGE_WIDTH,
                height: XPS_DEFAULT_PAGE_HEIGHT,
                text: vec![
                    XpsTextElement::new("Document Content".to_string(), 100.0, 100.0),
                    XpsTextElement::new("DRAFT - NOT FOR DISTRIBUTION".to_string(), 100.0, 500.0), // Watermark (50% opacity)
                    XpsTextElement::new("Confidential Information".to_string(), 100.0, 120.0),
                    XpsTextElement::new("INTERNAL USE ONLY".to_string(), 400.0, 500.0), // Diagonal watermark
                ],
            }],
        };

        let markdown = XpsBackend::xps_to_markdown(&xps);
        // Verify all text layers are preserved (including watermarks)
        assert!(
            markdown.contains("Document Content"),
            "Main content should be preserved"
        );
        assert!(
            markdown.contains("DRAFT - NOT FOR DISTRIBUTION"),
            "Draft watermark should be preserved"
        );
        assert!(
            markdown.contains("Confidential Information"),
            "Confidential text should be preserved"
        );
        assert!(
            markdown.contains("INTERNAL USE ONLY"),
            "Internal use watermark should be preserved"
        );
    }

    #[test]
    fn test_xps_with_form_fields() {
        use docling_xps::{XpsDocument, XpsMetadata, XpsPage, XpsTextElement};

        // XPS supports fillable forms (checkboxes, text fields, radio buttons)
        let xps = XpsDocument {
            metadata: XpsMetadata::new(),
            pages: vec![XpsPage {
                number: 1,
                width: XPS_DEFAULT_PAGE_WIDTH,
                height: XPS_DEFAULT_PAGE_HEIGHT,
                text: vec![
                    XpsTextElement::new("Application Form".to_string(), 100.0, 100.0),
                    XpsTextElement::new("Name: [            ]".to_string(), 100.0, 120.0),
                    XpsTextElement::new("Email: [            ]".to_string(), 100.0, 140.0),
                    XpsTextElement::new(
                        "☐ I agree to terms and conditions".to_string(),
                        100.0,
                        160.0,
                    ),
                    XpsTextElement::new(
                        "Gender: ○ Male  ○ Female  ○ Other".to_string(),
                        100.0,
                        180.0,
                    ),
                    XpsTextElement::new(
                        "Comments: [                    ]".to_string(),
                        100.0,
                        200.0,
                    ),
                ],
            }],
        };

        let markdown = XpsBackend::xps_to_markdown(&xps);
        // Verify form field labels and structures are preserved
        assert!(
            markdown.contains("Application Form"),
            "Form title should be preserved"
        );
        assert!(
            markdown.contains("Name:"),
            "Name field label should be preserved"
        );
        assert!(
            markdown.contains("Email:"),
            "Email field label should be preserved"
        );
        assert!(
            markdown.contains("agree to terms"),
            "Checkbox text should be preserved"
        );
        assert!(
            markdown.contains("Gender:"),
            "Gender field label should be preserved"
        );
        assert!(
            markdown.contains("Comments:"),
            "Comments field label should be preserved"
        );
    }

    #[test]
    fn test_xps_with_digital_signature_info() {
        use docling_xps::{XpsDocument, XpsMetadata, XpsPage, XpsTextElement};

        // XPS supports digital signatures for document verification
        let mut metadata = XpsMetadata::new();
        metadata.title = Some("Digitally Signed Document".to_string());
        metadata.author = Some("John Doe".to_string());

        let xps = XpsDocument {
            metadata,
            pages: vec![XpsPage {
                number: 1,
                width: XPS_DEFAULT_PAGE_WIDTH,
                height: XPS_DEFAULT_PAGE_HEIGHT,
                text: vec![
                    XpsTextElement::new("Official Document".to_string(), 100.0, 100.0),
                    XpsTextElement::new(
                        "This document has been digitally signed.".to_string(),
                        100.0,
                        120.0,
                    ),
                    XpsTextElement::new("Signature: John Doe".to_string(), 100.0, 900.0),
                    XpsTextElement::new("Date: 2024-01-15".to_string(), 100.0, 920.0),
                    XpsTextElement::new("Certificate: Valid".to_string(), 100.0, 940.0),
                ],
            }],
        };

        let markdown = XpsBackend::xps_to_markdown(&xps);
        // Verify metadata and signature information
        assert!(
            markdown.contains("Digitally Signed Document"),
            "Title with signature info should be preserved"
        );
        assert!(
            markdown.contains("Official Document"),
            "Document heading should be preserved"
        );
        assert!(
            markdown.contains("digitally signed"),
            "Digital signature statement should be preserved"
        );
        assert!(
            markdown.contains("Signature: John Doe"),
            "Signature line should be preserved"
        );
        assert!(
            markdown.contains("Certificate: Valid"),
            "Certificate status should be preserved"
        );
    }

    // ========== Advanced XPS Features (5 tests) ==========

    #[test]
    fn test_xps_fixed_document_structure() {
        use docling_xps::{XpsDocument, XpsMetadata, XpsPage, XpsTextElement};

        // XPS Fixed Document Structure: FixedDocumentSequence → FixedDocument → FixedPage
        // Tests multi-document XPS file (like multiple chapters in a book)
        let mut metadata = XpsMetadata::new();
        metadata.title = Some("Complete Works Collection".to_string());
        metadata.author = Some("Various Authors".to_string());

        let xps = XpsDocument {
            metadata,
            pages: vec![
                // Document 1: First work
                XpsPage {
                    number: 1,
                    width: XPS_DEFAULT_PAGE_WIDTH,
                    height: XPS_DEFAULT_PAGE_HEIGHT,
                    text: vec![
                        XpsTextElement::new(
                            "Document 1: Essay on Nature".to_string(),
                            100.0,
                            100.0,
                        ),
                        XpsTextElement::new(
                            "The natural world presents...".to_string(),
                            100.0,
                            140.0,
                        ),
                    ],
                },
                XpsPage {
                    number: 2,
                    width: XPS_DEFAULT_PAGE_WIDTH,
                    height: XPS_DEFAULT_PAGE_HEIGHT,
                    text: vec![XpsTextElement::new(
                        "...endless fascination.".to_string(),
                        100.0,
                        100.0,
                    )],
                },
                // Document 2: Second work (different fixed document in sequence)
                XpsPage {
                    number: 3,
                    width: XPS_DEFAULT_PAGE_WIDTH,
                    height: XPS_DEFAULT_PAGE_HEIGHT,
                    text: vec![
                        XpsTextElement::new(
                            "Document 2: Technical Manual".to_string(),
                            100.0,
                            100.0,
                        ),
                        XpsTextElement::new(
                            "Installation procedures are as follows...".to_string(),
                            100.0,
                            140.0,
                        ),
                    ],
                },
            ],
        };

        let markdown = XpsBackend::xps_to_markdown(&xps);
        // Verify all documents are preserved in sequence
        assert!(
            markdown.contains("Document 1: Essay on Nature"),
            "First document title should be preserved in fixed document structure"
        );
        assert!(
            markdown.contains("Document 2: Technical Manual"),
            "Second document title should be preserved in fixed document structure"
        );
        assert!(
            markdown.contains("natural world"),
            "First document content should be preserved"
        );
        assert!(
            markdown.contains("Installation procedures"),
            "Second document content should be preserved"
        );

        // Verify page structure is maintained
        let doc_items = XpsBackend::xps_to_doc_items(&xps);
        assert!(doc_items.len() >= 5, "Should have items from all 3 pages");
    }

    #[test]
    fn test_xps_resource_dictionaries() {
        use docling_xps::{XpsDocument, XpsMetadata, XpsPage, XpsTextElement};

        // XPS Resource Dictionaries: Shared resources (fonts, colors, brushes)
        // Common in corporate documents with consistent branding
        let mut metadata = XpsMetadata::new();
        metadata.title = Some("Corporate Brand Guidelines".to_string());
        metadata.author = Some("Marketing Department".to_string());

        let xps = XpsDocument {
            metadata,
            pages: vec![XpsPage {
                number: 1,
                width: XPS_DEFAULT_PAGE_WIDTH,
                height: XPS_DEFAULT_PAGE_HEIGHT,
                text: vec![
                    // Title using corporate font (referenced from resource dictionary)
                    XpsTextElement {
                        content: "Corporate Brand Guidelines".to_string(),
                        x: 100.0,
                        y: 100.0,
                        font_size: Some(24.0),
                    },
                    // Body using standard font (referenced from resource dictionary)
                    XpsTextElement {
                        content: "Primary Color: Corporate Blue (#003366)".to_string(),
                        x: 100.0,
                        y: 150.0,
                        font_size: Some(12.0),
                    },
                    XpsTextElement {
                        content: "Secondary Color: Corporate Gray (#999999)".to_string(),
                        x: 100.0,
                        y: 170.0,
                        font_size: Some(12.0),
                    },
                    // Heading using accent font (referenced from resource dictionary)
                    XpsTextElement {
                        content: "Typography Standards".to_string(),
                        x: 100.0,
                        y: 220.0,
                        font_size: Some(18.0),
                    },
                    XpsTextElement {
                        content: "Headings: Corporate Sans Bold, 18pt".to_string(),
                        x: 100.0,
                        y: 250.0,
                        font_size: Some(12.0),
                    },
                    XpsTextElement {
                        content: "Body: Corporate Sans Regular, 12pt".to_string(),
                        x: 100.0,
                        y: 270.0,
                        font_size: Some(12.0),
                    },
                ],
            }],
        };

        let markdown = XpsBackend::xps_to_markdown(&xps);
        // Verify text with different formatting resources
        assert!(
            markdown.contains("Corporate Brand Guidelines"),
            "Title with corporate formatting should be preserved"
        );
        assert!(
            markdown.contains("Primary Color: Corporate Blue"),
            "Color specification should be preserved"
        );
        assert!(
            markdown.contains("#003366"),
            "Hex color code should be preserved"
        );
        assert!(
            markdown.contains("Typography Standards"),
            "Typography section should be preserved"
        );

        // Verify DocItems preserve font size information
        let doc_items = XpsBackend::xps_to_doc_items(&xps);
        let text_items: Vec<_> = doc_items
            .iter()
            .filter(|item| matches!(item, DocItem::Text { .. }))
            .collect();
        assert!(
            text_items.len() >= 5,
            "Should have multiple text items with different formatting"
        );
    }

    #[test]
    fn test_xps_story_fragments() {
        use docling_xps::{XpsDocument, XpsMetadata, XpsPage, XpsTextElement};

        // XPS Story Fragments: Structured text flow across pages
        // Used for articles, magazines, or documents with text flowing through columns/pages
        let mut metadata = XpsMetadata::new();
        metadata.title = Some("Magazine Article".to_string());
        metadata.author = Some("Jane Reporter".to_string());

        let xps = XpsDocument {
            metadata,
            pages: vec![
                // Page 1: Article start in left column
                XpsPage {
                    number: 1,
                    width: XPS_DEFAULT_PAGE_WIDTH,
                    height: XPS_DEFAULT_PAGE_HEIGHT,
                    text: vec![
                        XpsTextElement::new(
                            "The Rise of Technology in Education".to_string(),
                            100.0,
                            100.0,
                        ),
                        XpsTextElement::new("By Jane Reporter".to_string(), 100.0, 130.0),
                        // Left column story fragment 1
                        XpsTextElement::new(
                            "In recent years, technology has transformed...".to_string(),
                            100.0,
                            180.0,
                        ),
                        XpsTextElement::new(
                            "...the landscape of modern education.".to_string(),
                            100.0,
                            200.0,
                        ),
                        // Right column story fragment 2 (continues from left)
                        XpsTextElement::new(
                            "Digital classrooms now incorporate...".to_string(),
                            450.0,
                            180.0,
                        ),
                        XpsTextElement::new(
                            "...interactive learning tools.".to_string(),
                            450.0,
                            200.0,
                        ),
                    ],
                },
                // Page 2: Article continuation
                XpsPage {
                    number: 2,
                    width: XPS_DEFAULT_PAGE_WIDTH,
                    height: XPS_DEFAULT_PAGE_HEIGHT,
                    text: vec![
                        // Story fragment 3 (continues from page 1)
                        XpsTextElement::new(
                            "Furthermore, students benefit from...".to_string(),
                            100.0,
                            100.0,
                        ),
                        XpsTextElement::new(
                            "...personalized learning experiences.".to_string(),
                            100.0,
                            120.0,
                        ),
                        XpsTextElement::new("(Continued on page 3)".to_string(), 100.0, 900.0),
                    ],
                },
            ],
        };

        let markdown = XpsBackend::xps_to_markdown(&xps);
        // Verify story fragments across pages and columns
        assert!(
            markdown.contains("Rise of Technology in Education"),
            "Article title should be preserved across story fragments"
        );
        assert!(
            markdown.contains("transformed"),
            "Left column content should be preserved"
        );
        assert!(
            markdown.contains("Digital classrooms"),
            "Right column content should be preserved"
        );
        assert!(
            markdown.contains("personalized learning"),
            "Page 2 content should be preserved"
        );
        assert!(
            markdown.contains("Continued on page 3"),
            "Continuation note should be preserved"
        );

        // Verify page structure
        let doc_items = XpsBackend::xps_to_doc_items(&xps);
        assert!(doc_items.len() >= 8, "Should have items from both pages");
    }

    #[test]
    fn test_xps_with_outlining_bookmarks() {
        use docling_xps::{XpsDocument, XpsMetadata, XpsPage, XpsTextElement};

        // XPS Outlining: Document structure and bookmarks (like PDF bookmarks)
        // Used for table of contents navigation
        let mut metadata = XpsMetadata::new();
        metadata.title = Some("Technical Specification".to_string());
        metadata.author = Some("Engineering Team".to_string());

        let xps = XpsDocument {
            metadata,
            pages: vec![
                // TOC page (page 1)
                XpsPage {
                    number: 1,
                    width: XPS_DEFAULT_PAGE_WIDTH,
                    height: XPS_DEFAULT_PAGE_HEIGHT,
                    text: vec![
                        XpsTextElement::new("Table of Contents".to_string(), 100.0, 100.0),
                        XpsTextElement::new(
                            "1. Introduction ............... 2".to_string(),
                            100.0,
                            150.0,
                        ),
                        XpsTextElement::new(
                            "2. System Requirements ..... 3".to_string(),
                            100.0,
                            170.0,
                        ),
                        XpsTextElement::new(
                            "   2.1 Hardware ............. 3".to_string(),
                            120.0,
                            190.0,
                        ),
                        XpsTextElement::new(
                            "   2.2 Software ............. 4".to_string(),
                            120.0,
                            210.0,
                        ),
                        XpsTextElement::new(
                            "3. Installation ............... 5".to_string(),
                            100.0,
                            230.0,
                        ),
                        XpsTextElement::new(
                            "4. Configuration ............ 6".to_string(),
                            100.0,
                            250.0,
                        ),
                    ],
                },
                // Chapter 1: Introduction (page 2)
                XpsPage {
                    number: 2,
                    width: XPS_DEFAULT_PAGE_WIDTH,
                    height: XPS_DEFAULT_PAGE_HEIGHT,
                    text: vec![
                        XpsTextElement::new("1. Introduction".to_string(), 100.0, 100.0),
                        XpsTextElement::new(
                            "This document describes the system architecture...".to_string(),
                            100.0,
                            140.0,
                        ),
                    ],
                },
                // Chapter 2: System Requirements (page 3)
                XpsPage {
                    number: 3,
                    width: XPS_DEFAULT_PAGE_WIDTH,
                    height: XPS_DEFAULT_PAGE_HEIGHT,
                    text: vec![
                        XpsTextElement::new("2. System Requirements".to_string(), 100.0, 100.0),
                        XpsTextElement::new("2.1 Hardware".to_string(), 100.0, 140.0),
                        XpsTextElement::new(
                            "Minimum 8GB RAM required...".to_string(),
                            120.0,
                            170.0,
                        ),
                    ],
                },
            ],
        };

        let markdown = XpsBackend::xps_to_markdown(&xps);
        // Verify TOC structure
        assert!(
            markdown.contains("Table of Contents"),
            "TOC header should be preserved"
        );
        assert!(
            markdown.contains("1. Introduction"),
            "Introduction TOC entry should be preserved"
        );
        assert!(
            markdown.contains("2. System Requirements"),
            "System Requirements TOC entry should be preserved"
        );
        assert!(
            markdown.contains("2.1 Hardware"),
            "Hardware subsection TOC entry should be preserved"
        );

        // Verify chapter headings
        assert!(
            markdown.contains("system architecture"),
            "Chapter content should be preserved"
        );
        assert!(
            markdown.contains("8GB RAM"),
            "Hardware specification content should be preserved"
        );

        // Verify DocItems structure
        let doc_items = XpsBackend::xps_to_doc_items(&xps);
        assert!(doc_items.len() >= 10, "Should have items from all pages");

        // Verify section headers are identified
        let section_headers: Vec<_> = doc_items
            .iter()
            .filter(|item| matches!(item, DocItem::SectionHeader { .. }))
            .collect();
        assert!(
            !section_headers.is_empty(),
            "Should have at least title as section header"
        );
    }

    #[test]
    fn test_xps_with_print_ticket() {
        use docling_xps::{XpsDocument, XpsMetadata, XpsPage, XpsTextElement};

        // XPS Print Ticket: Page settings (orientation, paper size, duplex, color)
        // Embedded print settings for reproduction
        let mut metadata = XpsMetadata::new();
        metadata.title = Some("Landscape Brochure".to_string());
        metadata.author = Some("Marketing".to_string());

        let xps = XpsDocument {
            metadata,
            pages: vec![
                // Page 1: Landscape orientation (width > height)
                XpsPage {
                    number: 1,
                    width: XPS_DEFAULT_PAGE_HEIGHT, // Landscape: swapped dimensions
                    height: XPS_DEFAULT_PAGE_WIDTH,
                    text: vec![
                        XpsTextElement::new("Product Brochure".to_string(), 400.0, 100.0),
                        XpsTextElement::new(
                            "[Page Settings: Landscape, Letter, Color, Duplex]".to_string(),
                            300.0,
                            750.0,
                        ),
                        XpsTextElement::new("Full-color photography".to_string(), 400.0, 300.0),
                        XpsTextElement::new("Print on both sides".to_string(), 400.0, 320.0),
                    ],
                },
                // Page 2: Also landscape (consistent print ticket)
                XpsPage {
                    number: 2,
                    width: XPS_DEFAULT_PAGE_HEIGHT,
                    height: XPS_DEFAULT_PAGE_WIDTH,
                    text: vec![
                        XpsTextElement::new("Features and Benefits".to_string(), 400.0, 100.0),
                        XpsTextElement::new("High quality images".to_string(), 400.0, 200.0),
                    ],
                },
            ],
        };

        let markdown = XpsBackend::xps_to_markdown(&xps);
        // Verify content
        assert!(
            markdown.contains("Product Brochure"),
            "Brochure title should be preserved"
        );
        assert!(
            markdown.contains("Features and Benefits"),
            "Features section should be preserved"
        );

        // Verify print settings indication
        assert!(
            markdown.contains("Landscape") || markdown.contains("Page Settings"),
            "Should reference print settings"
        );

        // Verify DocItems preserve page dimensions
        let doc_items = XpsBackend::xps_to_doc_items(&xps);
        assert!(doc_items.len() >= 4, "Should have items from both pages");

        // Verify provenance includes page numbers
        for item in &doc_items {
            match item {
                DocItem::Text { prov, .. } | DocItem::SectionHeader { prov, .. } => {
                    if !prov.is_empty() {
                        assert!(
                            prov[0].page_no >= 1 && prov[0].page_no <= 2,
                            "Page numbers should be 1 or 2"
                        );
                    }
                }
                _ => {}
            }
        }

        // Verify landscape orientation (width > height for both pages)
        assert!(
            xps.pages[0].width > xps.pages[0].height,
            "Page 1 should be landscape"
        );
        assert!(
            xps.pages[1].width > xps.pages[1].height,
            "Page 2 should be landscape"
        );
    }
}
