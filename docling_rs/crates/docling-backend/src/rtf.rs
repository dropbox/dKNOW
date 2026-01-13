//! RTF backend for docling
//!
//! This backend converts Rich Text Format (.rtf) files to docling's document model.

use crate::traits::{BackendOptions, DocumentBackend};
use crate::utils::{create_text_item, opt_vec};
use docling_core::{DocItem, DoclingError, Document, DocumentMetadata, Formatting, InputFormat};
use docling_legacy::{rtf_to_markdown_raw, RtfParser, StyleBlock};
use std::path::Path;

/// Text run with consistent formatting
///
/// Represents a span of text with specific formatting properties.
/// Similar to DOCX, PPTX, HTML, ODT implementations.
#[derive(Debug, Clone, PartialEq)]
struct TextRun {
    pub text: String,
    pub formatting: Option<Formatting>,
}

/// RTF backend
///
/// Converts Rich Text Format (.rtf) files to docling's document model.
/// Supports text extraction and basic formatting.
///
/// ## Features
///
/// - Parse RTF files using rtf-parser (RTF 1.9 specification)
/// - Extract plain text with paragraph breaks
/// - UTF-16 unicode support
/// - Font and color table parsing
/// - Markdown-formatted output
///
/// ## Example
///
/// ```no_run
/// use docling_backend::RtfBackend;
/// use docling_backend::DocumentBackend;
///
/// let backend = RtfBackend::new();
/// let result = backend.parse_file("document.rtf", &Default::default())?;
/// println!("Document: {:?}", result.metadata.title);
/// # Ok::<(), docling_core::error::DoclingError>(())
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct RtfBackend;

impl RtfBackend {
    /// Create a new RTF backend instance
    #[inline]
    #[must_use = "creates a new RTF backend instance"]
    pub const fn new() -> Self {
        Self
    }

    /// Create `DocItems` from RTF `StyleBlocks` with formatting extraction and paragraph separation
    ///
    /// Converts rtf-parser's `StyleBlock` structure (with Painter formatting info)
    /// into docling `DocItems`, extracting inline formatting (bold, italic, underline, etc.).
    /// Also ensures that paragraphs are properly separated into individual `DocItems`.
    ///
    /// # Arguments
    ///
    /// * `style_blocks` - The parsed `StyleBlocks` from rtf-parser (each has text + Painter)
    /// * `markdown` - The markdown output with paragraph breaks (used to detect paragraph boundaries)
    ///
    /// # Returns
    ///
    /// A vector of `DocItem::Text` entries with extracted formatting and proper paragraph separation.
    fn create_docitems_from_style_blocks(
        style_blocks: &[StyleBlock],
        markdown: &str,
    ) -> Vec<DocItem> {
        // Convert StyleBlocks to TextRuns
        let mut runs: Vec<TextRun> = Vec::new();

        for block in style_blocks {
            if block.text.is_empty() {
                continue;
            }

            let formatting = Self::extract_formatting(block);
            runs.push(TextRun {
                text: block.text.clone(),
                formatting,
            });
        }

        // Group consecutive runs with identical formatting
        let grouped_runs = Self::group_runs_by_formatting(runs);

        // Split runs by paragraph breaks
        // The markdown output has \n\n for paragraph breaks, so we use that to identify where to split
        let paragraphs: Vec<&str> = markdown.split("\n\n").collect();

        // If we have multiple paragraphs in markdown but single run, split the run
        let runs_to_process = if paragraphs.len() > 1 && grouped_runs.len() == 1 {
            // Split the single run's text by periods followed by capital letters (paragraph heuristic)
            // or use the paragraph structure from markdown
            Self::split_run_into_paragraphs(&grouped_runs[0], &paragraphs)
        } else {
            grouped_runs
        };

        // Convert grouped runs to DocItems
        let mut doc_items = Vec::new();
        let mut text_idx = 0;

        for run in runs_to_process {
            if run.text.trim().is_empty() {
                continue;
            }

            let parent_refs = vec![];

            doc_items.push(create_text_item(text_idx, run.text, parent_refs));

            // Update formatting on the last DocItem
            if let Some(fmt) = run.formatting {
                if let Some(DocItem::Text { formatting, .. }) = doc_items.last_mut() {
                    *formatting = Some(fmt);
                }
            }

            text_idx += 1;
        }

        doc_items
    }

    /// Split a single `TextRun` into multiple runs based on paragraph structure
    ///
    /// When rtf-parser gives us a single `StyleBlock` with concatenated paragraphs,
    /// but markdown correctly shows paragraph breaks, we need to split the text.
    ///
    /// # Arguments
    ///
    /// * `run` - The `TextRun` to split
    /// * `paragraphs` - Paragraph text from markdown (split by \n\n)
    ///
    /// # Returns
    ///
    /// A vector of `TextRuns`, one per paragraph
    fn split_run_into_paragraphs(run: &TextRun, paragraphs: &[&str]) -> Vec<TextRun> {
        let mut result = Vec::new();
        let text = &run.text;

        // Try to match paragraph boundaries in the concatenated text
        // Strategy: Find where each markdown paragraph appears in the concatenated text
        let mut current_pos = 0;

        for para in paragraphs {
            let para_trimmed = para.trim();
            if para_trimmed.is_empty() {
                continue;
            }

            // Find this paragraph's text in the concatenated string
            if let Some(pos) = text[current_pos..].find(para_trimmed) {
                let absolute_pos = current_pos + pos;
                let end_pos = absolute_pos + para_trimmed.len();

                // Extract this paragraph
                let paragraph_text = text[absolute_pos..end_pos].to_string();
                result.push(TextRun {
                    text: paragraph_text,
                    formatting: run.formatting.clone(),
                });

                current_pos = end_pos;
            }
        }

        // If we couldn't split (edge case), return the original run
        if result.is_empty() {
            result.push(run.clone());
        }

        result
    }

    /// Extract formatting from RTF `StyleBlock`'s Painter
    ///
    /// The rtf-parser crate provides formatting via StyleBlock.painter fields:
    /// - painter.bold: bool
    /// - painter.italic: bool
    /// - painter.underline: bool
    /// - painter.strike: bool
    /// - painter.superscript: bool
    /// - painter.subscript: bool
    ///
    /// # Arguments
    ///
    /// * `block` - The `StyleBlock` containing text and Painter formatting
    ///
    /// # Returns
    ///
    /// Some(Formatting) if any formatting is present, None otherwise.
    fn extract_formatting(block: &StyleBlock) -> Option<Formatting> {
        let painter = &block.painter;

        let has_formatting = painter.bold
            || painter.italic
            || painter.underline
            || painter.strike
            || painter.superscript
            || painter.subscript;

        if !has_formatting {
            return None;
        }

        Some(Formatting {
            bold: painter.bold.then_some(true),
            italic: painter.italic.then_some(true),
            underline: painter.underline.then_some(true),
            strikethrough: painter.strike.then_some(true),
            code: None, // RTF doesn't have inline code concept
            script: if painter.superscript {
                Some("super".to_string())
            } else if painter.subscript {
                Some("sub".to_string())
            } else {
                None
            },
            font_family: None, // RTF backend doesn't extract font family yet
            font_size: None,   // RTF backend doesn't extract font size yet
        })
    }

    /// Group consecutive text runs with identical formatting
    ///
    /// Merges adjacent `TextRuns` that have the same formatting properties.
    /// This reduces `DocItem` count and matches the pattern used in other backends.
    ///
    /// # Arguments
    ///
    /// * `runs` - Vector of `TextRuns` to group
    ///
    /// # Returns
    ///
    /// A new vector with consecutive runs merged where formatting matches.
    fn group_runs_by_formatting(runs: Vec<TextRun>) -> Vec<TextRun> {
        if runs.is_empty() {
            return Vec::new();
        }

        let mut grouped = Vec::new();
        let mut iter = runs.into_iter();
        // SAFETY: We checked runs.is_empty() above
        let mut current_run = iter.next().unwrap();

        for run in iter {
            if Self::formatting_matches(current_run.formatting.as_ref(), run.formatting.as_ref()) {
                // Same formatting - merge text
                current_run.text.push_str(&run.text);
            } else {
                // Different formatting - start new run
                grouped.push(current_run);
                current_run = run;
            }
        }

        // Push final run
        grouped.push(current_run);
        grouped
    }

    /// Check if two formatting options are equivalent
    fn formatting_matches(fmt1: Option<&Formatting>, fmt2: Option<&Formatting>) -> bool {
        match (fmt1, fmt2) {
            (None, None) => true,
            (Some(f1), Some(f2)) => {
                f1.bold == f2.bold
                    && f1.italic == f2.italic
                    && f1.underline == f2.underline
                    && f1.strikethrough == f2.strikethrough
                    && f1.script == f2.script
            }
            _ => false,
        }
    }

    /// Legacy method: Create `DocItems` from RTF markdown output
    ///
    /// Used for fallback and testing. Converts markdown to simple `DocItems` without formatting.
    /// This method is kept for backward compatibility with existing tests.
    ///
    /// # Arguments
    ///
    /// * `markdown` - The markdown string generated from the RTF document
    ///
    /// # Returns
    ///
    /// A vector of `DocItem::Text` entries representing the document structure.
    #[cfg(test)]
    fn create_docitems(markdown: &str) -> Vec<DocItem> {
        let mut doc_items = Vec::new();
        let mut text_idx = 0;

        // Split into paragraphs (double newline separated)
        // RTF documents often have paragraph breaks as double newlines in markdown
        for paragraph in markdown.split("\n\n") {
            let trimmed = paragraph.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Create a Text DocItem for each paragraph using helper
            doc_items.push(create_text_item(text_idx, trimmed.to_string(), vec![]));
            text_idx += 1;
        }

        doc_items
    }
}

impl DocumentBackend for RtfBackend {
    #[inline]
    fn format(&self) -> InputFormat {
        InputFormat::Rtf
    }

    fn parse_bytes(
        &self,
        data: &[u8],
        _options: &BackendOptions,
    ) -> Result<Document, DoclingError> {
        // Convert bytes to string
        let content = std::str::from_utf8(data)
            .map_err(|e| DoclingError::BackendError(format!("Invalid UTF-8 in RTF file: {e}")))?;

        // Parse RTF document
        let rtf_doc = RtfParser::parse_str(content)
            .map_err(|e| DoclingError::BackendError(format!("Failed to parse RTF: {e}")))?;

        // Convert to markdown using docling_legacy (with raw RTF for paragraph detection)
        let markdown = rtf_to_markdown_raw(&rtf_doc, Some(content));

        // Create DocItems from StyleBlocks (preserves formatting: bold, italic, underline, etc.)
        // Uses rtf-parser's StyleBlock.painter fields for accurate formatting extraction
        // Pass markdown to enable paragraph separation
        let doc_items = Self::create_docitems_from_style_blocks(&rtf_doc.body, &markdown);
        let num_characters = markdown.chars().count();

        // Extract metadata (RTF spec allows info group, but rtf-parser may not expose it)
        // For now, we use None for metadata fields
        let title = None;
        let author = None;
        let created = None;
        let modified = None;

        // Populate content_blocks if we have DocItems
        let content_blocks = opt_vec(doc_items);

        // Create document
        Ok(Document {
            markdown,
            format: InputFormat::Rtf,
            metadata: DocumentMetadata {
                num_pages: None,
                num_characters,
                title,
                author,
                created,
                modified,
                language: None,
                subject: None,
                exif: None,
            },
            content_blocks,
            docling_document: None,
        })
    }

    fn parse_file<P: AsRef<Path>>(
        &self,
        path: P,
        options: &BackendOptions,
    ) -> Result<Document, DoclingError> {
        let path_ref = path.as_ref();
        let filename = path_ref.display().to_string();

        // Helper to add filename context to errors
        let add_context = |err: DoclingError| -> DoclingError {
            match err {
                DoclingError::BackendError(msg) => {
                    DoclingError::BackendError(format!("{msg}: {filename}"))
                }
                other => other,
            }
        };

        let data = std::fs::read(path_ref).map_err(DoclingError::IoError)?;
        self.parse_bytes(&data, options).map_err(add_context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rtf_backend_creation() {
        let backend = RtfBackend::new();
        assert_eq!(backend.format(), InputFormat::Rtf);
    }

    #[test]
    fn test_parse_simple_rtf() {
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Times New Roman;}}
\f0\fs60 Hello, World!
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert_eq!(
            doc.format,
            InputFormat::Rtf,
            "Document format should be Rtf"
        );
        assert!(
            doc.markdown.contains("Hello, World!"),
            "Markdown should contain 'Hello, World!' text"
        );
        assert!(
            doc.metadata.num_characters > 0,
            "Character count should be positive for RTF with content"
        );
    }

    #[test]
    fn test_parse_rtf_with_paragraphs() {
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Times New Roman;}}
\f0\fs24 First paragraph.\par
\par
Second paragraph.\par
\par
Third paragraph.
}";

        // First parse RTF to see StyleBlocks
        let rtf_doc = RtfParser::parse_str(rtf).unwrap();
        println!("Number of StyleBlocks: {}", rtf_doc.body.len());
        for (i, block) in rtf_doc.body.iter().enumerate() {
            println!(
                "Block {}: text={:?}, len={}",
                i,
                block.text,
                block.text.len()
            );
        }

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        println!("\nRTF markdown output:\n{:?}", doc.markdown);
        println!("RTF markdown rendered:\n{}", doc.markdown);

        // Check DocItems are separated by paragraphs
        if let Some(ref content_blocks) = doc.content_blocks {
            println!("Number of DocItems: {}", content_blocks.len());
            for (i, item) in content_blocks.iter().enumerate() {
                if let DocItem::Text { text, .. } = item {
                    println!("DocItem {i}: {text:?}");
                }
            }
        }

        assert!(
            doc.markdown.contains("First paragraph"),
            "Markdown should contain 'First paragraph'"
        );
        assert!(
            doc.markdown.contains("Second paragraph"),
            "Markdown should contain 'Second paragraph'"
        );
        assert!(
            doc.markdown.contains("Third paragraph"),
            "Markdown should contain 'Third paragraph'"
        );
    }

    #[test]
    fn test_parse_empty_rtf() {
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Times New Roman;}}
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert_eq!(
            doc.format,
            InputFormat::Rtf,
            "Document format should be Rtf"
        );
        assert!(
            doc.markdown.is_empty() || doc.markdown.trim().is_empty(),
            "Empty RTF should produce empty or whitespace-only markdown"
        );
    }

    #[test]
    fn test_parse_invalid_rtf() {
        let invalid_rtf = b"This is not RTF content";

        let backend = RtfBackend::new();
        let result = backend.parse_bytes(invalid_rtf, &BackendOptions::default());

        assert!(
            result.is_err(),
            "Parsing invalid RTF content should return an error"
        );
    }

    // ============================================================================
    // CATEGORY 1: Metadata Tests (3 tests)
    // ============================================================================

    #[test]
    fn test_rtf_metadata_character_count() {
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Times New Roman;}}
\f0\fs24 This is a test document with multiple words.
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Character count should match markdown length
        assert_eq!(
            doc.metadata.num_characters,
            doc.markdown.chars().count(),
            "num_characters should match actual markdown character count"
        );
        assert!(
            doc.metadata.num_characters > 0,
            "Character count should be positive for RTF with content"
        );
    }

    #[test]
    fn test_rtf_metadata_empty_document() {
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Times New Roman;}}
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Empty RTF should have zero or minimal character count
        assert!(
            doc.metadata.num_characters == 0 || doc.markdown.trim().is_empty(),
            "Empty RTF should have zero characters or empty markdown"
        );
    }

    #[test]
    fn test_rtf_metadata_no_optional_fields() {
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Times New Roman;}}
\f0\fs24 Test content
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        // RTF backend doesn't extract metadata (parser limitation)
        assert!(
            doc.metadata.title.is_none(),
            "RTF parser does not extract title metadata"
        );
        assert!(
            doc.metadata.author.is_none(),
            "RTF parser does not extract author metadata"
        );
        assert!(
            doc.metadata.created.is_none(),
            "RTF parser does not extract created date metadata"
        );
        assert!(
            doc.metadata.modified.is_none(),
            "RTF parser does not extract modified date metadata"
        );
        assert!(
            doc.metadata.num_pages.is_none(),
            "RTF parser does not extract page count metadata"
        );
    }

    // ============================================================================
    // CATEGORY 2: DocItem Generation Tests (3 tests)
    // ============================================================================

    #[test]
    fn test_rtf_docitem_single_paragraph() {
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Times New Roman;}}
\f0\fs24 This is a single paragraph of text.
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        let items = doc.content_blocks.expect("Should have DocItems");
        assert_eq!(
            items.len(),
            1,
            "Single paragraph RTF should produce exactly one DocItem"
        );

        match &items[0] {
            DocItem::Text { text, .. } => {
                assert!(
                    text.contains("single paragraph"),
                    "DocItem text should contain 'single paragraph'"
                );
            }
            _ => panic!("Expected Text DocItem"),
        }
    }

    #[test]
    fn test_rtf_docitem_multiple_paragraphs() {
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Times New Roman;}}
\f0\fs24 First paragraph.\par
\par
Second paragraph.\par
\par
Third paragraph.
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        let items = doc.content_blocks.expect("Should have DocItems");
        // RTF backend splits on \n\n, so we expect multiple Text DocItems
        assert!(
            !items.is_empty(),
            "Multi-paragraph RTF should produce DocItems"
        );

        // All items should be Text DocItems
        for (i, item) in items.iter().enumerate() {
            assert!(
                matches!(item, DocItem::Text { .. }),
                "DocItem {i} should be Text variant"
            );
        }
    }

    #[test]
    fn test_rtf_docitem_empty_document() {
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Times New Roman;}}
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Empty RTF should have no DocItems
        assert!(
            doc.content_blocks.is_none(),
            "Empty RTF should have no DocItems"
        );
    }

    // ============================================================================
    // CATEGORY 3: Format-Specific Features (5 tests)
    // ============================================================================

    #[test]
    fn test_rtf_paragraph_break_parsing() {
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Times New Roman;}}
\f0\fs24 Paragraph one.\par
\par
Paragraph two.
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        // \par should create paragraph breaks in markdown
        assert!(
            doc.markdown.contains("Paragraph one"),
            "Markdown should contain 'Paragraph one'"
        );
        assert!(
            doc.markdown.contains("Paragraph two"),
            "Markdown should contain 'Paragraph two'"
        );
    }

    #[test]
    fn test_rtf_font_table_handling() {
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Arial;}{\f1 Times New Roman;}{\f2 Courier New;}}
\f0 Arial text\par
\f1 Times text\par
\f2 Courier text
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Parser should handle multiple fonts
        assert!(
            doc.markdown.contains("Arial text"),
            "Markdown should contain 'Arial text' from font 0"
        );
        assert!(
            doc.markdown.contains("Times text"),
            "Markdown should contain 'Times text' from font 1"
        );
        assert!(
            doc.markdown.contains("Courier text"),
            "Markdown should contain 'Courier text' from font 2"
        );
    }

    #[test]
    fn test_rtf_font_size_variations() {
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Times New Roman;}}
\f0\fs16 Small text\par
\fs24 Normal text\par
\fs36 Large text
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Font sizes should not break parsing
        assert!(
            doc.markdown.contains("Small text"),
            "Markdown should contain 'Small text' (fs16)"
        );
        assert!(
            doc.markdown.contains("Normal text"),
            "Markdown should contain 'Normal text' (fs24)"
        );
        assert!(
            doc.markdown.contains("Large text"),
            "Markdown should contain 'Large text' (fs36)"
        );
    }

    #[test]
    fn test_rtf_whitespace_preservation() {
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Times New Roman;}}
\f0\fs24 Text   with   multiple   spaces
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Markdown should contain the text (whitespace handling varies by parser)
        assert!(
            doc.markdown.contains("Text"),
            "Markdown should contain 'Text'"
        );
        assert!(
            doc.markdown.contains("multiple"),
            "Markdown should contain 'multiple'"
        );
        assert!(
            doc.markdown.contains("spaces"),
            "Markdown should contain 'spaces'"
        );
    }

    #[test]
    fn test_rtf_create_docitems_helper() {
        let markdown = "First paragraph\n\nSecond paragraph\n\nThird paragraph";

        let items = RtfBackend::create_docitems(markdown);

        assert_eq!(
            items.len(),
            3,
            "Three paragraphs should create three DocItems"
        );

        // All should be Text DocItems
        for (i, item) in items.iter().enumerate() {
            match item {
                DocItem::Text { .. } => {
                    // Text DocItem verified
                }
                _ => panic!("Expected Text DocItem at index {i}"),
            }
        }
    }

    // ============================================================================
    // CATEGORY 4: Edge Cases (3 tests)
    // ============================================================================

    #[test]
    fn test_rtf_only_whitespace() {
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Times New Roman;}}
\f0\fs24    \par
   \par

}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Whitespace-only content should produce minimal/no DocItems
        // (behavior depends on parser's whitespace handling)
        let item_count = doc.content_blocks.as_ref().map_or(0, Vec::len);
        assert!(
            item_count == 0 || doc.markdown.trim().is_empty(),
            "Whitespace-only RTF should produce no DocItems or empty markdown"
        );
    }

    #[test]
    fn test_rtf_special_characters() {
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Times New Roman;}}
\f0\fs24 Special characters: \u233? \u8364? \u169?
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Parser should handle RTF unicode escapes
        assert!(
            doc.markdown.contains("Special characters"),
            "Markdown should contain 'Special characters'"
        );
        assert!(
            doc.metadata.num_characters > 0,
            "Character count should be positive for RTF with content"
        );
    }

    #[test]
    fn test_rtf_invalid_utf8() {
        let invalid_bytes = vec![0xFF, 0xFE, 0x7B, 0x5C, 0x72, 0x74, 0x66, 0x31, 0xFF, 0xFF];

        let backend = RtfBackend::new();
        let result = backend.parse_bytes(&invalid_bytes, &BackendOptions::default());

        // Should fail gracefully with UTF-8 error
        assert!(result.is_err(), "Invalid UTF-8 data should return an error");
        if let Err(DoclingError::BackendError(msg)) = result {
            assert!(
                msg.contains("Invalid UTF-8") || msg.contains("UTF"),
                "Error message should mention UTF-8: {msg}"
            );
        } else {
            panic!("Expected BackendError for invalid UTF-8");
        }
    }

    // ============================================================================
    // CATEGORY 5: Text Formatting Features (5 tests)
    // ============================================================================

    #[test]
    fn test_rtf_bold_text() {
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Times New Roman;}}
\f0\fs24 Normal text \b Bold text\b0  Normal again
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Parser should extract text content (bold markers may or may not be in markdown)
        assert!(doc.markdown.contains("Normal text"));
        assert!(doc.markdown.contains("Bold text"));
    }

    #[test]
    fn test_rtf_italic_text() {
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Times New Roman;}}
\f0\fs24 Normal text \i Italic text\i0  Normal again
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert!(doc.markdown.contains("Normal text"));
        assert!(doc.markdown.contains("Italic text"));
    }

    #[test]
    fn test_rtf_underline_text() {
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Times New Roman;}}
\f0\fs24 Normal text \ul Underlined text\ul0  Normal again
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert!(doc.markdown.contains("Normal text"));
        assert!(doc.markdown.contains("Underlined text"));
    }

    #[test]
    fn test_rtf_combined_formatting() {
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Times New Roman;}}
\f0\fs24 \b\i Bold and italic\b0\i0  text
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert!(doc.markdown.contains("Bold and italic"));
        assert!(doc.markdown.contains("text"));
    }

    #[test]
    fn test_rtf_strikethrough_text() {
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Times New Roman;}}
\f0\fs24 Normal \strike strikethrough\strike0  text
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert!(doc.markdown.contains("Normal"));
        assert!(doc.markdown.contains("strikethrough"));
        assert!(doc.markdown.contains("text"));
    }

    // ============================================================================
    // CATEGORY 6: Document Structure (5 tests)
    // ============================================================================

    #[test]
    fn test_rtf_docitem_indices() {
        let markdown = "Para 1\n\nPara 2\n\nPara 3";
        let items = RtfBackend::create_docitems(markdown);

        assert_eq!(items.len(), 3);

        // Verify indices are sequential
        for (i, item) in items.iter().enumerate() {
            let self_ref = match item {
                DocItem::Text { self_ref, .. } => self_ref,
                _ => panic!("Expected Text DocItem"),
            };

            let index_str = self_ref.trim_start_matches("#/texts/");
            let index: usize = index_str.parse().expect("Invalid index in self_ref");
            assert_eq!(
                index, i,
                "DocItem at position {i} has self_ref index {index}"
            );
        }
    }

    #[test]
    fn test_rtf_docitem_content_layer() {
        let markdown = "Test paragraph";
        let items = RtfBackend::create_docitems(markdown);

        assert_eq!(items.len(), 1);

        match &items[0] {
            DocItem::Text { content_layer, .. } => {
                assert_eq!(content_layer, "body");
            }
            _ => panic!("Expected Text DocItem"),
        }
    }

    #[test]
    fn test_rtf_docitem_self_ref_format() {
        let markdown = "Test";
        let items = RtfBackend::create_docitems(markdown);

        assert_eq!(items.len(), 1);

        match &items[0] {
            DocItem::Text { self_ref, .. } => {
                assert_eq!(self_ref, "#/texts/0");
            }
            _ => panic!("Expected Text DocItem"),
        }
    }

    #[test]
    fn test_rtf_large_document() {
        let mut rtf = String::from(
            r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Times New Roman;}}
\f0\fs24 ",
        );

        // Create 10 paragraphs
        for i in 1..=10 {
            rtf.push_str(&format!("Paragraph {i}\\par\n\\par\n"));
        }
        rtf.push('}');

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Should have DocItems (count depends on parser's paragraph handling)
        // Just verify the document parses and contains all paragraphs
        assert!(doc.markdown.contains("Paragraph 1"));
        assert!(doc.markdown.contains("Paragraph 10"));
        assert!(doc.metadata.num_characters > 0);

        // Verify we have some DocItems
        if let Some(items) = doc.content_blocks {
            assert!(!items.is_empty(), "Should have at least some DocItems");
        }
    }

    #[test]
    fn test_rtf_very_long_paragraph() {
        let mut rtf = String::from(
            r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Times New Roman;}}
\f0\fs24 ",
        );

        // Create a very long paragraph (1000 words)
        for i in 1..=1000 {
            rtf.push_str(&format!("word{i} "));
        }
        rtf.push('}');

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Should parse without error
        assert!(doc.metadata.num_characters > 5000);
        assert!(doc.markdown.contains("word1"));
        assert!(doc.markdown.contains("word1000"));
    }

    // ============================================================================
    // CATEGORY 7: Color and Tables (3 tests)
    // ============================================================================

    #[test]
    fn test_rtf_color_table() {
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Times New Roman;}}
{\colortbl ;\red255\green0\blue0;\red0\green255\blue0;\red0\green0\blue255;}
\f0\fs24 \cf1 Red text \cf2 Green text \cf3 Blue text\cf0  Normal
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Parser should handle color tables
        assert!(doc.markdown.contains("Red text"));
        assert!(doc.markdown.contains("Green text"));
        assert!(doc.markdown.contains("Blue text"));
        assert!(doc.markdown.contains("Normal"));
    }

    #[test]
    fn test_rtf_table_basic() {
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Times New Roman;}}
\f0\fs24
\trowd\cellx2000\cellx4000
\intbl Cell 1\cell Cell 2\cell\row
\trowd\cellx2000\cellx4000
\intbl Cell 3\cell Cell 4\cell\row
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Parser should handle basic tables
        assert!(doc.markdown.contains("Cell 1") || doc.markdown.contains("Cell"));
    }

    #[test]
    fn test_rtf_background_color() {
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Times New Roman;}}
{\colortbl ;\red255\green255\blue0;}
\f0\fs24 \cb1 Highlighted text\cb0  Normal text
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert!(doc.markdown.contains("Highlighted text"));
        assert!(doc.markdown.contains("Normal text"));
    }

    // ============================================================================
    // CATEGORY 8: List and Bullets (4 tests)
    // ============================================================================

    #[test]
    fn test_rtf_bulleted_list() {
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Times New Roman;}}
\f0\fs24
\pard{\pntext\bullet\tab}Item 1\par
{\pntext\bullet\tab}Item 2\par
{\pntext\bullet\tab}Item 3\par
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert!(doc.markdown.contains("Item 1"));
        assert!(doc.markdown.contains("Item 2"));
        assert!(doc.markdown.contains("Item 3"));
    }

    #[test]
    fn test_rtf_numbered_list() {
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Times New Roman;}}
\f0\fs24
\pard{\pntext 1.\tab}First item\par
{\pntext 2.\tab}Second item\par
{\pntext 3.\tab}Third item\par
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert!(doc.markdown.contains("First item"));
        assert!(doc.markdown.contains("Second item"));
        assert!(doc.markdown.contains("Third item"));
    }

    #[test]
    fn test_rtf_indentation() {
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Times New Roman;}}
\f0\fs24
\pard\li720 Indented paragraph\par
\pard\li0 Normal paragraph\par
\pard\li1440 Double indented paragraph\par
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert!(doc.markdown.contains("Indented paragraph"));
        assert!(doc.markdown.contains("Normal paragraph"));
        assert!(doc.markdown.contains("Double indented paragraph"));
    }

    #[test]
    fn test_rtf_alignment() {
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Times New Roman;}}
\f0\fs24
\qc Centered text\par
\ql Left aligned text\par
\qr Right aligned text\par
\qj Justified text\par
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert!(doc.markdown.contains("Centered text"));
        assert!(doc.markdown.contains("Left aligned text"));
        assert!(doc.markdown.contains("Right aligned text"));
        assert!(doc.markdown.contains("Justified text"));
    }

    // ============================================================================
    // CATEGORY 9: Special Content (3 tests)
    // ============================================================================

    #[test]
    fn test_rtf_hyperlink() {
        let rtf = r#"{\rtf1\ansi\deff0 {\fonttbl {\f0 Times New Roman;}}
\f0\fs24 Visit {\field{\*\fldinst{HYPERLINK "http://example.com"}}{\fldrslt Example Site}}
}"#;

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Parser may or may not preserve hyperlink structure
        assert!(doc.markdown.contains("Visit") || doc.markdown.contains("Example"));
    }

    #[test]
    fn test_rtf_newline_characters() {
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Times New Roman;}}
\f0\fs24 Line 1\line Line 2\line Line 3
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert!(doc.markdown.contains("Line 1"));
        assert!(doc.markdown.contains("Line 2"));
        assert!(doc.markdown.contains("Line 3"));
    }

    #[test]
    fn test_rtf_tab_characters() {
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Times New Roman;}}
\f0\fs24 Column1\tab Column2\tab Column3
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert!(doc.markdown.contains("Column1"));
        assert!(doc.markdown.contains("Column2"));
        assert!(doc.markdown.contains("Column3"));
    }

    // ============================================================================
    // CATEGORY 10: Character Encoding Variations (4 tests)
    // ============================================================================

    #[test]
    fn test_rtf_mac_encoding() {
        let rtf = r"{\rtf1\mac\deff0 {\fonttbl {\f0 Times New Roman;}}
\f0\fs24 Mac encoded text
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert!(doc.markdown.contains("Mac encoded text"));
        assert_eq!(doc.format, InputFormat::Rtf);
    }

    #[test]
    fn test_rtf_pc_encoding() {
        let rtf = r"{\rtf1\pc\deff0 {\fonttbl {\f0 Times New Roman;}}
\f0\fs24 PC encoded text
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert!(doc.markdown.contains("PC encoded text"));
        assert_eq!(doc.format, InputFormat::Rtf);
    }

    #[test]
    fn test_rtf_pca_encoding() {
        let rtf = r"{\rtf1\pca\deff0 {\fonttbl {\f0 Times New Roman;}}
\f0\fs24 PCA encoded text
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert!(doc.markdown.contains("PCA encoded text"));
        assert_eq!(doc.format, InputFormat::Rtf);
    }

    #[test]
    fn test_rtf_unicode_character_escapes() {
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Times New Roman;}}
\f0\fs24 Unicode: \u8364? \u163? \u165?
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Parser should handle unicode escapes
        assert!(doc.markdown.contains("Unicode"));
        assert!(doc.metadata.num_characters > 0);
    }

    // ============================================================================
    // CATEGORY 11: Nested and Complex Formatting (4 tests)
    // ============================================================================

    #[test]
    fn test_rtf_deeply_nested_groups() {
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Times New Roman;}}
\f0\fs24 {Outer {Middle {Inner text} middle} outer}
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert!(doc.markdown.contains("Inner text"));
        assert!(doc.metadata.num_characters > 0);
    }

    #[test]
    fn test_rtf_multiple_font_switches() {
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Arial;}{\f1 Times New Roman;}{\f2 Courier New;}}
\f0 Arial\f1 Times\f2 Courier\f0 Arial again\f1 Times again
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Parser should handle rapid font switching
        assert!(doc.markdown.contains("Arial"));
        assert!(doc.markdown.contains("Times"));
        assert!(doc.markdown.contains("Courier"));
    }

    #[test]
    fn test_rtf_nested_formatting_bold_italic() {
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Times New Roman;}}
\f0\fs24 {\b {\i Nested bold and italic} still bold} normal
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        println!("Markdown output: {:?}", doc.markdown);

        // Verify DocItems have correct text content (even if markdown doesn't format it)
        let items = doc.content_blocks.expect("Should have DocItems");
        let all_text: String = items
            .iter()
            .filter_map(|item| match item {
                DocItem::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" ");

        assert!(
            all_text.contains("Nested bold and italic"),
            "DocItems should contain text"
        );
        assert!(
            all_text.contains("still bold"),
            "DocItems should contain text"
        );
        assert!(all_text.contains("normal"), "DocItems should contain text");
    }

    #[test]
    fn test_rtf_complex_paragraph_structure() {
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Times New Roman;}}
\f0\fs24
\pard\qc\b Centered Bold Title\b0\par
\pard\ql Normal paragraph with \i italic\i0  and \ul underline\ul0  text.\par
\pard\qr\fs18 Right aligned small footer\par
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert!(doc.markdown.contains("Centered Bold Title"));
        assert!(doc.markdown.contains("Normal paragraph"));
        assert!(doc.markdown.contains("Right aligned small footer"));
    }

    // ============================================================================
    // CATEGORY 12: create_docitems() Edge Cases (5 tests)
    // ============================================================================

    #[test]
    fn test_create_docitems_empty_string() {
        let items = RtfBackend::create_docitems("");
        assert_eq!(items.len(), 0, "Empty string should produce no DocItems");
    }

    #[test]
    fn test_create_docitems_only_newlines() {
        let items = RtfBackend::create_docitems("\n\n\n\n");
        assert_eq!(items.len(), 0, "Only newlines should produce no DocItems");
    }

    #[test]
    fn test_create_docitems_mixed_empty_paragraphs() {
        let markdown = "First\n\n\n\nSecond\n\n\n\n\n\nThird";
        let items = RtfBackend::create_docitems(markdown);

        assert_eq!(items.len(), 3);

        match &items[0] {
            DocItem::Text { text, .. } => assert_eq!(text, "First"),
            _ => panic!("Expected Text DocItem"),
        }
        match &items[1] {
            DocItem::Text { text, .. } => assert_eq!(text, "Second"),
            _ => panic!("Expected Text DocItem"),
        }
        match &items[2] {
            DocItem::Text { text, .. } => assert_eq!(text, "Third"),
            _ => panic!("Expected Text DocItem"),
        }
    }

    #[test]
    fn test_create_docitems_whitespace_only_paragraphs() {
        let markdown = "Para1\n\n   \n\nPara2\n\n\t\t\n\nPara3";
        let items = RtfBackend::create_docitems(markdown);

        // Whitespace-only paragraphs should be skipped
        assert_eq!(items.len(), 3);

        for item in items.iter() {
            match item {
                DocItem::Text { text, .. } => {
                    assert!(text.contains("Para"));
                }
                _ => panic!("Expected Text DocItem"),
            }
        }
    }

    #[test]
    fn test_create_docitems_very_long_single_paragraph() {
        let long_text = "A".repeat(100000);
        let items = RtfBackend::create_docitems(&long_text);

        assert_eq!(items.len(), 1);

        match &items[0] {
            DocItem::Text { text, .. } => {
                assert_eq!(text.len(), 100000);
            }
            _ => panic!("Expected Text DocItem"),
        }
    }

    // ============================================================================
    // CATEGORY 13: Escape Sequences and Special Characters (4 tests)
    // ============================================================================

    #[test]
    fn test_rtf_escaped_braces() {
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Times New Roman;}}
\f0\fs24 Text with \{ and \} braces
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Parser should handle escaped braces
        assert!(doc.markdown.contains("Text with"));
        assert!(doc.metadata.num_characters > 0);
    }

    #[test]
    fn test_rtf_escaped_backslash() {
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Times New Roman;}}
\f0\fs24 Text with \\ backslash
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert!(doc.markdown.contains("Text with"));
        assert!(doc.metadata.num_characters > 0);
    }

    #[test]
    fn test_rtf_hex_escape() {
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Times New Roman;}}
\f0\fs24 Hex escape: \'41 \'42 \'43
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Hex escapes (\'41 = 'A', \'42 = 'B', \'43 = 'C')
        assert!(doc.markdown.contains("Hex escape"));
        assert!(doc.metadata.num_characters > 0);
    }

    #[test]
    fn test_rtf_optional_hyphen() {
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Times New Roman;}}
\f0\fs24 This is a long\-word that can break
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Optional hyphen (\-) - parser behavior varies
        // Just verify document parses and contains text
        assert!(doc.markdown.contains("long") || doc.markdown.contains("This"));
        assert!(doc.metadata.num_characters > 0);
    }

    // ============================================================================
    // CATEGORY 14: Error Handling and Malformed Input (3 tests)
    // ============================================================================

    #[test]
    fn test_rtf_missing_closing_brace() {
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Times New Roman;}}
\f0\fs24 Text without closing brace";

        let backend = RtfBackend::new();
        let result = backend.parse_bytes(rtf.as_bytes(), &BackendOptions::default());

        // Parser should handle or reject malformed RTF
        // Result could be Ok (lenient parser) or Err (strict parser)
        // Just verify it doesn't panic
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_rtf_unknown_control_word() {
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Times New Roman;}}
\f0\fs24 \unknowncontrol Text with unknown control
}";

        let backend = RtfBackend::new();
        let result = backend.parse_bytes(rtf.as_bytes(), &BackendOptions::default());

        // Unknown control words should be ignored or handled gracefully
        if let Ok(doc) = result {
            assert!(doc.markdown.contains("Text with unknown control"));
        }
    }

    #[test]
    fn test_rtf_empty_font_table() {
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl}
\f0\fs24 Text with empty font table
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert!(
            doc.markdown.contains("Text with empty font table") || doc.markdown.contains("Text")
        );
    }

    // ============================================================================
    // CATEGORY 15: International and Unicode Text (1 test)
    // ============================================================================

    #[test]
    fn test_rtf_international_unicode_text() {
        // Test RTF with international characters and Unicode escapes
        // Common in real-world documents from international users
        let rtf = r"{\rtf1\ansi\ansicpg1252\deff0 {\fonttbl {\f0 Arial;}}
\f0\fs24 International characters:\par
French: \'e9 (é) Caf\'e9\par
German: \'fc (ü) Gr\'fc\'dfen\par
Spanish: \'f1 (ñ) Espa\'f1ol\par
Greek: \u945? (alpha) \u946? (beta)\par
Chinese: \u20013? (中) \u22269? (国)\par
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Should handle international characters
        assert!(doc.markdown.contains("International") || doc.markdown.contains("French"));

        // Should generate DocItems
        assert!(doc.content_blocks.is_some());
        let items = doc.content_blocks.unwrap();
        assert!(!items.is_empty(), "Should have at least one DocItem");

        // Verify DocItems were created (may be one or more depending on parser behavior)
        let text_items: Vec<_> = items
            .iter()
            .filter(|item| matches!(item, DocItem::Text { .. }))
            .collect();
        assert!(
            !text_items.is_empty(),
            "Should have at least one Text DocItem"
        );

        // Should have reasonable character count (international characters present)
        assert!(
            doc.metadata.num_characters >= 20,
            "Document should have content"
        );
    }

    // ============================================================================
    // CATEGORY 16: Additional Edge Cases (N=534) (5 tests)
    // ============================================================================

    #[test]
    fn test_rtf_with_tabs_and_indentation() {
        // Test RTF with tab characters and indentation control words
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Arial;}}
\f0\fs24 First level\par
\tab Second level (tab)\par
\tab\tab Third level (two tabs)\par
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert!(doc.markdown.contains("First level"));
        assert!(doc.content_blocks.is_some());
        assert!(!doc.content_blocks.as_ref().unwrap().is_empty());
    }

    #[test]
    fn test_rtf_with_hyperlinks() {
        // Test RTF with hyperlink field codes ({\field{\*\fldinst HYPERLINK ...}})
        let rtf = r#"{\rtf1\ansi\deff0 {\fonttbl {\f0 Arial;}}
\f0\fs24 Visit {\field{\*\fldinst HYPERLINK "http://example.com"}{\fldrslt Example}} for more info.\par
}"#;

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Should extract hyperlink display text or URL
        assert!(
            doc.markdown.contains("Visit")
                || doc.markdown.contains("Example")
                || doc.markdown.contains("example.com")
        );
        assert!(doc.content_blocks.is_some());
    }

    #[test]
    fn test_rtf_with_special_characters_curly_braces() {
        // Test RTF with literal curly braces (must be escaped as \{ and \})
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Arial;}}
\f0\fs24 Use \{curly braces\} in code.\par
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Should correctly handle escaped braces
        assert!(doc.markdown.len() > 5);
        assert!(doc.content_blocks.is_some());
    }

    #[test]
    fn test_rtf_with_very_long_single_line() {
        // Test RTF with a very long line (no paragraph breaks)
        let long_text = "Word ".repeat(500); // 2500 chars
        let rtf = format!(
            r"{{\rtf1\ansi\deff0 {{\fonttbl {{\f0 Arial;}}}}
\f0\fs24 {long_text}\par
}}"
        );

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Should handle long lines without truncation or errors
        assert!(doc.markdown.len() > 1000);
        assert!(doc.content_blocks.is_some());
        assert!(!doc.content_blocks.as_ref().unwrap().is_empty());
    }

    #[test]
    fn test_rtf_with_multiple_font_sizes() {
        // Test RTF with multiple font size changes (\fs20, \fs24, \fs32)
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Arial;}}
\f0\fs20 Small text.\par
\fs24 Normal text.\par
\fs32 Large text.\par
\fs16 Very small text.\par
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Should extract all text regardless of font size
        assert!(doc.markdown.contains("Small") || doc.markdown.len() > 20);
        assert!(doc.content_blocks.is_some());

        let items = doc.content_blocks.as_ref().unwrap();
        assert!(
            !items.is_empty(),
            "Should generate DocItems for multi-size text"
        );
    }

    #[test]
    fn test_rtf_with_color_table() {
        // Test RTF with color table (\colortbl) and colored text (\cf1, \cf2)
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Arial;}}
{\colortbl ;\red255\green0\blue0;\red0\green255\blue0;\red0\green0\blue255;}
\f0\fs24 Normal text.\par
\cf1 Red text.\par
\cf2 Green text.\par
\cf3 Blue text.\par
\cf0 Back to normal.\par
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Should extract all text regardless of color
        assert!(doc.markdown.len() > 20);
        assert!(doc.content_blocks.is_some());

        let items = doc.content_blocks.as_ref().unwrap();
        assert!(
            !items.is_empty(),
            "Should generate DocItems for colored text"
        );
    }

    #[test]
    fn test_rtf_with_bullets_and_numbering() {
        // Test RTF with bullet lists and numbered lists
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Arial;}}
\f0\fs24 Shopping list:\par
\pard\li720\fi-360\bullet\tab Apples\par
\bullet\tab Oranges\par
\bullet\tab Bananas\par
\pard\li0 Numbered steps:\par
\pard\li720\fi-360 1.\tab First step\par
2.\tab Second step\par
3.\tab Third step\par
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Should extract list items
        assert!(doc.markdown.len() > 30);
        assert!(doc.content_blocks.is_some());

        let items = doc.content_blocks.as_ref().unwrap();
        assert!(!items.is_empty(), "Should generate DocItems for lists");
    }

    #[test]
    fn test_rtf_with_headers_and_footers() {
        // Test RTF with header and footer sections
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Arial;}}
{\header Header content\par}
{\footer Footer content\par}
\f0\fs24 Main body text.\par
More content here.\par
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Should extract main body text (headers/footers may be ignored or included)
        assert!(doc.markdown.len() > 10);
        assert!(doc.content_blocks.is_some());

        let items = doc.content_blocks.as_ref().unwrap();
        assert!(
            !items.is_empty(),
            "Should generate DocItems for document with headers/footers"
        );
    }

    #[test]
    fn test_rtf_with_tables() {
        // Test RTF with table structure (\trowd, \cell, \row)
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Arial;}}
\f0\fs24 Document with table:\par
\trowd\cellx2000\cellx4000\cellx6000
\intbl Cell 1\cell Cell 2\cell Cell 3\cell\row
\intbl Data 1\cell Data 2\cell Data 3\cell\row
\pard Normal text after table.\par
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Should extract table content
        assert!(doc.markdown.len() > 15);
        assert!(doc.content_blocks.is_some());

        let items = doc.content_blocks.as_ref().unwrap();
        assert!(
            !items.is_empty(),
            "Should generate DocItems for document with tables"
        );
    }

    #[test]
    fn test_rtf_with_page_breaks() {
        // Test RTF with explicit page breaks (\page)
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Arial;}}
\f0\fs24 Content on page 1.\par
This is still page 1.\par
\page
Content on page 2.\par
More content on page 2.\par
\page
Content on page 3.\par
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Should extract content from all pages
        assert!(doc.markdown.len() > 30);
        assert!(doc.content_blocks.is_some());

        let items = doc.content_blocks.as_ref().unwrap();
        assert!(
            !items.is_empty(),
            "Should generate DocItems for multi-page document"
        );

        // Check that content from different pages is preserved
        let markdown_lower = doc.markdown.to_lowercase();
        assert!(
            markdown_lower.contains("page 1")
                || markdown_lower.contains("page 2")
                || markdown_lower.contains("page 3")
                || doc.markdown.len() > 30,
            "Should preserve content across page breaks"
        );
    }

    // ========== ADVANCED REAL-WORLD RTF TESTS ==========

    /// Test RTF with embedded image (common in Word documents)
    #[test]
    fn test_rtf_with_embedded_image() {
        // RTF with embedded PNG image data ({\pict ...} group)
        // Common in Word documents, presentations, reports
        let rtf = r"{\rtf1\ansi\deff0
{\fonttbl{\f0 Times New Roman;}}
{\pict\pngblip\picw100\pich100 89504e470d0a1a0a}
\par Image caption: Company Logo
\par This document contains an embedded image.
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Text content should be extracted even if image parsing is not implemented
        assert!(doc.markdown.contains("Image caption") || doc.markdown.contains("document"));
        assert!(doc.content_blocks.is_some());
        // Should not crash on image data
        assert!(!doc.markdown.is_empty());
    }

    /// Test RTF with footnotes and endnotes (academic/legal documents)
    #[test]
    fn test_rtf_with_footnotes_and_endnotes() {
        // RTF with footnotes ({\footnote ...}) - common in academic papers, legal briefs
        let rtf = r"{\rtf1\ansi\deff0
{\fonttbl{\f0 Times New Roman;}}
Main text with citation{\footnote See Smith v. Jones, 123 F.3d 456 (2020)}.
\par More text with another footnote{\footnote According to the 2019 Annual Report, page 42}.
\par Final paragraph with endnote{\endnote This is an endnote at document end}.
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Main text should be extracted
        assert!(doc.markdown.contains("Main text") || doc.markdown.contains("citation"));
        assert!(doc.content_blocks.is_some());
        // Footnote content may or may not be extracted (depends on implementation)
        // Test should pass either way
        assert!(doc.markdown.len() > 20);
    }

    /// Test RTF with bidirectional text (Arabic/Hebrew with RTL support)
    #[test]
    fn test_rtf_with_bidirectional_text() {
        // RTF with right-to-left paragraph direction (Arabic/Hebrew)
        // Common in international documents, Middle Eastern business documents
        let rtf = r"{\rtf1\ansi\deff0
{\fonttbl{\f0 Arial;}{\f1 Arial Unicode MS;}}
English paragraph (LTR).
\par\rtlpar\f1 \u1605?\u1585?\u1581?\u1576?\u1575? (Arabic: Hello in RTL)
\par\ltrpar\f0 Back to English (LTR).
\par Mixed: English and \rtlch\f1\u1593?\u1585?\u1576?\u1610? \ltrch\f0 combined.
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Text should be extracted (Unicode handling)
        assert!(doc.markdown.contains("English") || doc.markdown.len() > 20);
        assert!(doc.content_blocks.is_some());
        // Bidirectional control words should not crash parser
        assert!(!doc.markdown.is_empty());
    }

    /// Test RTF with field codes (auto-updating dates, page numbers)
    #[test]
    fn test_rtf_with_field_codes() {
        // RTF field codes ({\field ...}) - common in templates, forms
        // Auto-updating dates, page numbers, TOC entries
        let rtf = r#"{\rtf1\ansi\deff0
{\fonttbl{\f0 Arial;}}
Document created: {\field{\*\fldinst DATE \@ "MMMM d, yyyy"}{\fldrslt January 15, 2024}}
\par Page {\field{\*\fldinst PAGE}{\fldrslt 1}} of {\field{\*\fldinst NUMPAGES}{\fldrslt 10}}
\par File path: {\field{\*\fldinst FILENAME \* MERGEFORMAT}{\fldrslt report.docx}}
\par Author: {\field{\*\fldinst AUTHOR \* MERGEFORMAT}{\fldrslt John Doe}}
}"#;

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Field result text should be extracted
        assert!(
            doc.markdown.contains("January")
                || doc.markdown.contains("2024")
                || doc.markdown.contains("Document created")
        );
        assert!(doc.content_blocks.is_some());
        // Field instructions should not break parser
        assert!(doc.markdown.len() > 20);
    }

    /// Test RTF with bookmarks and cross-references (long documents)
    #[test]
    fn test_rtf_with_bookmarks_and_cross_references() {
        // RTF bookmarks ({\*\bkmkstart ...}) - internal navigation in contracts, manuals
        let rtf = r"{\rtf1\ansi\deff0
{\fonttbl{\f0 Arial;}}
{\*\bkmkstart section1}Section 1: Introduction{\*\bkmkend section1}
\par This section introduces the topic.
\par {\*\bkmkstart section2}Section 2: Details{\*\bkmkend section2}
\par For more information, see {\field{\*\fldinst REF section1}{\fldrslt Section 1}}.
\par Cross-reference to {\field{\*\fldinst REF section2 \* MERGEFORMAT}{\fldrslt Section 2}}.
\par {\*\bkmkstart conclusion}Conclusion{\*\bkmkend conclusion}
\par Final remarks referencing {\field{\*\fldinst PAGEREF section1}{\fldrslt page 1}}.
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Section text should be extracted
        assert!(
            doc.markdown.contains("Section 1")
                || doc.markdown.contains("Introduction")
                || doc.markdown.contains("Details")
        );
        assert!(doc.content_blocks.is_some());
        // Bookmarks and cross-references should not break parser
        assert!(doc.markdown.len() > 40);
    }

    // ============================================================================
    // CATEGORY 17: Formatting Extraction Tests (N=1165) (5 tests)
    // ============================================================================

    /// Test RTF formatting extraction - bold text
    #[test]
    fn test_rtf_formatting_extraction_bold() {
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Times New Roman;}}
\f0\fs24 Normal text \b bold text\b0  normal again
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Should have DocItems
        let items = doc.content_blocks.expect("Should have DocItems");
        assert!(!items.is_empty(), "Should generate at least one DocItem");

        // Check for bold formatting in at least one DocItem
        let has_bold = items.iter().any(|item| match item {
            DocItem::Text { formatting, .. } => {
                formatting.as_ref().and_then(|f| f.bold).unwrap_or(false)
            }
            _ => false,
        });

        assert!(
            has_bold,
            "Should have at least one DocItem with bold formatting"
        );
    }

    /// Test RTF formatting extraction - italic text
    #[test]
    fn test_rtf_formatting_extraction_italic() {
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Times New Roman;}}
\f0\fs24 Normal text \i italic text\i0  normal again
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        let items = doc.content_blocks.expect("Should have DocItems");
        assert!(!items.is_empty());

        // Check for italic formatting
        let has_italic = items.iter().any(|item| match item {
            DocItem::Text { formatting, .. } => {
                formatting.as_ref().and_then(|f| f.italic).unwrap_or(false)
            }
            _ => false,
        });

        assert!(
            has_italic,
            "Should have at least one DocItem with italic formatting"
        );
    }

    /// Test RTF formatting extraction - underline text
    #[test]
    fn test_rtf_formatting_extraction_underline() {
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Times New Roman;}}
\f0\fs24 Normal text \ul underlined text\ul0  normal again
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        let items = doc.content_blocks.expect("Should have DocItems");
        assert!(!items.is_empty());

        // Check for underline formatting
        let has_underline = items.iter().any(|item| match item {
            DocItem::Text { formatting, .. } => formatting
                .as_ref()
                .and_then(|f| f.underline)
                .unwrap_or(false),
            _ => false,
        });

        assert!(
            has_underline,
            "Should have at least one DocItem with underline formatting"
        );
    }

    /// Test RTF formatting extraction - strikethrough text
    #[test]
    fn test_rtf_formatting_extraction_strikethrough() {
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Times New Roman;}}
\f0\fs24 Normal text \strike strikethrough text\strike0  normal again
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        let items = doc.content_blocks.expect("Should have DocItems");
        assert!(!items.is_empty());

        // Check for strikethrough formatting
        let has_strikethrough = items.iter().any(|item| match item {
            DocItem::Text { formatting, .. } => formatting
                .as_ref()
                .and_then(|f| f.strikethrough)
                .unwrap_or(false),
            _ => false,
        });

        assert!(
            has_strikethrough,
            "Should have at least one DocItem with strikethrough formatting"
        );
    }

    /// Test RTF formatting extraction - combined formatting
    #[test]
    fn test_rtf_formatting_extraction_combined() {
        let rtf = r"{\rtf1\ansi\deff0 {\fonttbl {\f0 Times New Roman;}}
\f0\fs24 Normal \b\i bold and italic\b0\i0  normal \ul\b underline and bold\ul0\b0  normal again
}";

        let backend = RtfBackend::new();
        let doc = backend
            .parse_bytes(rtf.as_bytes(), &BackendOptions::default())
            .unwrap();

        let items = doc.content_blocks.expect("Should have DocItems");
        assert!(!items.is_empty(), "Should generate DocItems");

        // Check that we have multiple formatted text runs
        // Combined formatting should create separate DocItems for different formatting combinations
        let formatted_items: Vec<_> = items
            .iter()
            .filter(|item| match item {
                DocItem::Text { formatting, .. } => formatting.is_some(),
                _ => false,
            })
            .collect();

        assert!(
            !formatted_items.is_empty(),
            "Should have at least one DocItem with formatting"
        );

        // Check for at least one item with multiple formatting properties
        let has_combined_formatting = items.iter().any(|item| match item {
            DocItem::Text {
                formatting: Some(fmt),
                ..
            } => {
                let prop_count = [fmt.bold, fmt.italic, fmt.underline, fmt.strikethrough]
                    .into_iter()
                    .flatten()
                    .count();
                prop_count >= 2
            }
            _ => false,
        });

        assert!(
            has_combined_formatting,
            "Should have at least one DocItem with combined formatting (2+ properties)"
        );
    }
}
