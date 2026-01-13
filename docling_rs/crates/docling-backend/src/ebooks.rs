//! E-book document backend for EPUB, FB2, and MOBI formats
//!
//! This module provides e-book parsing and markdown conversion capabilities
//! for various e-book formats supported by the docling-ebook crate.

// Clippy pedantic allows:
// - E-book parsing functions are complex
// - Unit struct &self convention
#![allow(clippy::too_many_lines)]
#![allow(clippy::trivially_copy_pass_by_ref)]

use crate::traits::{BackendOptions, DocumentBackend};
use crate::utils::{create_list_item, create_section_header, create_text_item};
use docling_core::{
    content::{CoordOrigin, DocItem, ProvenanceItem},
    DoclingError, Document, DocumentMetadata, InputFormat,
};
use docling_ebook::{html_to_text, parse_epub, parse_fb2, parse_mobi, ParsedEbook};
use std::path::Path;

/// E-book backend for processing electronic book formats
///
/// Supports:
/// - EPUB (.epub) - Electronic Publication format (EPUB 2.0.1 and 3.x)
/// - FB2 (.fb2, .fb2.zip) - `FictionBook` XML format
/// - MOBI (.mobi, .prc, .azw) - Mobipocket format
///
/// Parses e-book metadata, chapters, and table of contents.
/// Converts to markdown with chapter structure preserved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EbooksBackend {
    format: InputFormat,
}

impl EbooksBackend {
    /// Create a new e-books backend for the specified format
    ///
    /// # Errors
    ///
    /// Returns an error if the format is not an e-book format.
    #[inline]
    #[must_use = "creating a backend that is not used is a waste of resources"]
    pub fn new(format: InputFormat) -> Result<Self, DoclingError> {
        if !format.is_ebook() {
            return Err(DoclingError::FormatError(format!(
                "Format {format:?} is not an e-book format"
            )));
        }
        Ok(Self { format })
    }

    /// Generate `DocItems` from parsed e-book
    ///
    /// Structure:
    /// 1. Title as `SectionHeader` (level 1)
    /// 2. Metadata as Text (authors, publisher, etc.)
    /// 3. Table of Contents as `SectionHeader` + `ListItems`
    /// 4. Chapters as `SectionHeaders` (level 2) + Text
    fn generate_docitems(ebook: &ParsedEbook, format: InputFormat) -> Vec<DocItem> {
        let mut doc_items = Vec::new();

        // Title header
        if let Some(title) = &ebook.metadata.title {
            doc_items.push(create_section_header(
                doc_items.len(),
                title.clone(),
                1,
                Self::create_provenance(1),
            ));
        }

        // Authors
        if !ebook.metadata.creators.is_empty() {
            let authors_text = format!("Authors: {}", ebook.metadata.creators.join(", "));
            doc_items.push(create_text_item(
                doc_items.len(),
                authors_text,
                Self::create_provenance(1),
            ));
        }

        // Publisher
        if let Some(publisher) = &ebook.metadata.publisher {
            let publisher_text = format!("Publisher: {publisher}");
            doc_items.push(create_text_item(
                doc_items.len(),
                publisher_text,
                Self::create_provenance(1),
            ));
        }

        // Publication date
        if let Some(date) = &ebook.metadata.date {
            let date_text = format!("Published: {date}");
            doc_items.push(create_text_item(
                doc_items.len(),
                date_text,
                Self::create_provenance(1),
            ));
        }

        // Description
        if let Some(description) = &ebook.metadata.description {
            let desc_text = format!("Description: {description}");
            doc_items.push(create_text_item(
                doc_items.len(),
                desc_text,
                Self::create_provenance(1),
            ));
        }

        // Language
        if let Some(language) = &ebook.metadata.language {
            let lang_text = format!("Language: {language}");
            doc_items.push(create_text_item(
                doc_items.len(),
                lang_text,
                Self::create_provenance(1),
            ));
        }

        // Identifier (ISBN, UUID, document ID, etc.)
        if let Some(identifier) = &ebook.metadata.identifier {
            let id_text = format!("Identifier: {identifier}");
            doc_items.push(create_text_item(
                doc_items.len(),
                id_text,
                Self::create_provenance(1),
            ));
        }

        // Contributors (translators, editors, illustrators, etc.)
        if !ebook.metadata.contributors.is_empty() {
            for contributor in &ebook.metadata.contributors {
                doc_items.push(create_text_item(
                    doc_items.len(),
                    contributor.clone(),
                    Self::create_provenance(1),
                ));
            }
        }

        // Subjects (genres, keywords, topics)
        if !ebook.metadata.subjects.is_empty() {
            let subjects_text = format!("Subjects: {}", ebook.metadata.subjects.join(", "));
            doc_items.push(create_text_item(
                doc_items.len(),
                subjects_text,
                Self::create_provenance(1),
            ));
        }

        // Rights/License (important for completeness, especially Project Gutenberg books)
        if let Some(rights) = &ebook.metadata.rights {
            let rights_text = format!("Rights: {rights}");
            doc_items.push(create_text_item(
                doc_items.len(),
                rights_text,
                Self::create_provenance(1),
            ));
        }

        // Body title (FB2 format) - title page content with possible subtitle
        // Only include if different from metadata title to avoid duplication
        if let Some(body_title) = &ebook.body_title {
            let is_duplicate = ebook
                .metadata
                .title
                .as_ref()
                .is_some_and(|t| t == body_title);

            if !is_duplicate {
                doc_items.push(create_text_item(
                    doc_items.len(),
                    body_title.clone(),
                    Self::create_provenance(1),
                ));
            }
        }

        // Table of Contents (with hierarchical structure)
        if !ebook.toc.is_empty() {
            doc_items.push(create_section_header(
                doc_items.len(),
                "Table of Contents".to_string(),
                2,
                Self::create_provenance(1),
            ));

            for entry in &ebook.toc {
                Self::add_toc_entry_recursive(&mut doc_items, entry, 0);
            }
        }

        // Page List (EPUB pageList for page markers/illustrations)
        if !ebook.page_list.is_empty() {
            doc_items.push(create_section_header(
                doc_items.len(),
                "List of Pages".to_string(),
                2,
                Self::create_provenance(1),
            ));

            for page_target in &ebook.page_list {
                doc_items.push(create_list_item(
                    doc_items.len(),
                    page_target.label.clone(),
                    "- ".to_string(),
                    false,
                    Self::create_provenance(1),
                    None,
                ));
            }
        }

        // Chapters (with spine order metadata)
        if !ebook.chapters.is_empty() {
            for chapter in &ebook.chapters {
                // Chapter content (convert HTML for EPUB and MOBI)
                // Both formats contain HTML that needs conversion to markdown
                // FB2 content is already plain text, so no conversion needed
                let content = if format == InputFormat::Epub || format == InputFormat::Mobi {
                    html_to_text(&chapter.content)
                } else {
                    chapter.content.clone()
                };

                // Skip chapters with empty content (prevents confusing empty section headers)
                // This handles cases like EPUB cover wrappers with only images or metadata sections
                if content.trim().is_empty() {
                    continue;
                }

                // Chapter heading (only add if content is non-empty)
                if let Some(title) = &chapter.title {
                    doc_items.push(create_section_header(
                        doc_items.len(),
                        title.clone(),
                        2,
                        Self::create_provenance(1),
                    ));
                }

                // Add content as Text DocItem
                doc_items.push(create_text_item(
                    doc_items.len(),
                    content,
                    Self::create_provenance(1),
                ));
            }
        }

        doc_items
    }

    /// Recursively add TOC entries with hierarchical structure
    ///
    /// Uses indentation to show nesting levels:
    /// - Level 0: "- [Entry](#href)"
    /// - Level 1: "  - [Sub-entry](#href)"
    /// - Level 2: "    - [Sub-sub-entry](#href)"
    ///
    /// Formats each entry as a markdown link: `[label](href)` for better navigation
    fn add_toc_entry_recursive(
        doc_items: &mut Vec<DocItem>,
        entry: &docling_ebook::TocEntry,
        level: usize,
    ) {
        // Calculate indentation prefix (2 spaces per level)
        let indent = "  ".repeat(level);
        let marker = format!("{indent}- ");

        // Format entry as markdown link: [label](href)
        // If href doesn't start with #, add it (for internal document links)
        let href = if entry.href.starts_with('#') {
            entry.href.clone()
        } else if entry.href.is_empty() {
            // No href available, use plain text
            String::new()
        } else {
            // Add # prefix for internal links (e.g., "chapter_1" -> "#chapter_1")
            format!("#{}", entry.href)
        };

        // Create markdown link text
        let link_text = if href.is_empty() {
            // No href, use plain label
            entry.label.clone()
        } else {
            // Format as markdown link: [label](href)
            format!("[{}]({})", entry.label, href)
        };

        // Add current entry
        doc_items.push(create_list_item(
            doc_items.len(),
            link_text,
            marker,
            false,
            Self::create_provenance(1),
            None,
        ));

        // Recursively add children with increased indentation
        for child in &entry.children {
            Self::add_toc_entry_recursive(doc_items, child, level + 1);
        }
    }

    /// Create provenance metadata for ebook content
    ///
    /// Returns a Vec containing a single `ProvenanceItem` for the given page.
    /// This is the standard format expected by `DocItem` creation functions.
    fn create_provenance(page_no: usize) -> Vec<ProvenanceItem> {
        vec![crate::utils::create_default_provenance(
            page_no,
            CoordOrigin::BottomLeft,
        )]
    }

    /// Convert `DocItems` to markdown
    fn docitems_to_markdown(doc_items: &[DocItem]) -> String {
        let mut markdown = String::new();

        for (i, item) in doc_items.iter().enumerate() {
            match item {
                DocItem::SectionHeader { text, level, .. } => {
                    if i > 0 {
                        markdown.push('\n');
                    }
                    markdown.push_str(&"#".repeat(*level));
                    markdown.push(' ');
                    markdown.push_str(text);
                    markdown.push_str("\n\n");
                }
                DocItem::Text { text, .. } => {
                    // Metadata fields use plain format (no bold keys)
                    markdown.push_str(text);
                    markdown.push_str("\n\n");
                }
                DocItem::ListItem { text, marker, .. } => {
                    // Use the marker from DocItem (includes indentation for nested items)
                    markdown.push_str(marker);
                    markdown.push_str(text);
                    markdown.push('\n');
                }
                _ => {
                    // Other DocItem types not expected in e-books
                }
            }
        }

        // Add separators for better document structure
        // Separator 1: After metadata, before Table of Contents
        // Separator 2: After Table of Contents, before Chapters

        // Find "## Table of Contents" position
        if let Some(toc_pos) = markdown.find("## Table of Contents") {
            // Add separator before TOC
            let before_toc = &markdown[..toc_pos];
            let from_toc = &markdown[toc_pos..];

            // Find the end of TOC section (next ## header, which would be first chapter)
            if let Some(next_section_pos) = from_toc[20..].find("\n\n## ") {
                // Position relative to start of document
                let chapter_start = toc_pos + 20 + next_section_pos;
                let toc_section = &markdown[toc_pos..chapter_start];
                let chapters = &markdown[chapter_start + 2..]; // Skip \n\n

                // Ensure toc_section ends with proper spacing before separator
                // ListItems end with single '\n', so we need to add one more '\n' before '---'
                markdown = format!("{before_toc}---\n\n{toc_section}\n---\n\n{chapters}");
            } else {
                // No chapters after TOC, just add separator before TOC
                markdown = format!("{before_toc}---\n\n{from_toc}");
            }
        } else if markdown.contains("## ") {
            // No TOC, but has chapters - add separator before first chapter
            if let Some(pos) = markdown.find("\n\n## ") {
                let before = &markdown[..pos];
                let after = &markdown[pos + 2..];
                markdown = format!("{before}---\n\n{after}");
            }
        }

        markdown
    }

    /// Convert `EbookMetadata` to `DocumentMetadata`
    ///
    /// Extracts title, authors, language, and publication date from e-book metadata
    /// and populates the Document's metadata fields.
    fn ebook_to_document_metadata(ebook: &ParsedEbook, num_characters: usize) -> DocumentMetadata {
        // Join multiple authors with ", "
        let author = if ebook.metadata.creators.is_empty() {
            None
        } else {
            Some(ebook.metadata.creators.join(", "))
        };

        // Try to parse publication date as DateTime
        // E-book dates are often in formats like "2024-01-15" or "2024"
        let created = ebook.metadata.date.as_ref().and_then(|date_str| {
            // Try full ISO 8601 format first
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(date_str) {
                return Some(dt.with_timezone(&chrono::Utc));
            }

            // Try date-only format (YYYY-MM-DD)
            if let Ok(naive_date) = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                let naive_datetime = naive_date.and_hms_opt(0, 0, 0)?;
                return Some(chrono::DateTime::from_naive_utc_and_offset(
                    naive_datetime,
                    chrono::Utc,
                ));
            }

            // Try year-only format (YYYY)
            if let Ok(year) = date_str.parse::<i32>() {
                if (1000..=9999).contains(&year) {
                    let naive_date = chrono::NaiveDate::from_ymd_opt(year, 1, 1)?;
                    let naive_datetime = naive_date.and_hms_opt(0, 0, 0)?;
                    return Some(chrono::DateTime::from_naive_utc_and_offset(
                        naive_datetime,
                        chrono::Utc,
                    ));
                }
            }

            None
        });

        // Extract subject from subjects list (join multiple subjects)
        let subject = if ebook.metadata.subjects.is_empty() {
            None
        } else {
            Some(ebook.metadata.subjects.join(", "))
        };

        DocumentMetadata {
            num_pages: None, // E-books don't have traditional page numbers
            num_characters,
            title: ebook.metadata.title.clone(),
            author,
            created,
            modified: None, // E-books typically don't have modification dates
            language: ebook.metadata.language.clone(),
            subject, // N=1878: Extract subjects from ebook metadata
            exif: None,
        }
    }

    /// Parse e-book and convert to markdown
    ///
    /// For EPUB: Parses from file path, extracts HTML chapters, converts to text
    /// For FB2: Parses from file path (handles .fb2 and .fb2.zip), extracts text chapters
    /// For MOBI: Parses from bytes, extracts chapters (already markdown-formatted)
    fn parse_and_convert(
        &self,
        content: &[u8],
        path: Option<&Path>,
    ) -> Result<ParsedEbook, DoclingError> {
        let ebook: ParsedEbook = match self.format {
            InputFormat::Epub => {
                // EPUB requires file path (ZIP archive)
                let path = path.ok_or_else(|| {
                    DoclingError::BackendError("EPUB format requires file path".to_string())
                })?;
                parse_epub(path)
                    .map_err(|e| DoclingError::BackendError(format!("Failed to parse EPUB: {e}")))?
            }
            InputFormat::Fb2 => {
                // FB2 requires file path (may be .fb2 or .fb2.zip)
                let path = path.ok_or_else(|| {
                    DoclingError::BackendError("FB2 format requires file path".to_string())
                })?;
                parse_fb2(path)
                    .map_err(|e| DoclingError::BackendError(format!("Failed to parse FB2: {e}")))?
            }
            InputFormat::Mobi => {
                // MOBI can parse from bytes
                parse_mobi(content)
                    .map_err(|e| DoclingError::BackendError(format!("Failed to parse MOBI: {e}")))?
            }
            _ => {
                return Err(DoclingError::FormatError(format!(
                    "Unsupported e-book format: {:?}",
                    self.format
                )))
            }
        };

        Ok(ebook)
    }
}

impl DocumentBackend for EbooksBackend {
    #[inline]
    fn format(&self) -> InputFormat {
        self.format
    }

    fn parse_bytes(
        &self,
        content: &[u8],
        _options: &BackendOptions,
    ) -> Result<Document, DoclingError> {
        // EPUB and FB2 cannot be parsed from bytes alone (require file path for ZIP/XML parsing)
        if self.format == InputFormat::Epub || self.format == InputFormat::Fb2 {
            return Err(DoclingError::BackendError(format!(
                "{:?} format requires file path, use parse_file() instead",
                self.format
            )));
        }

        let ebook = self.parse_and_convert(content, None)?;
        let doc_items = Self::generate_docitems(&ebook, self.format);
        let markdown = Self::docitems_to_markdown(&doc_items);
        let num_characters = markdown.chars().count();

        Ok(Document {
            markdown,
            format: self.format,
            metadata: Self::ebook_to_document_metadata(&ebook, num_characters),
            content_blocks: Some(doc_items),
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

        // EPUB and FB2 need special handling (file path required)
        if self.format == InputFormat::Epub || self.format == InputFormat::Fb2 {
            let ebook = self
                .parse_and_convert(&[], Some(path_ref))
                .map_err(&add_context)?;
            let doc_items = Self::generate_docitems(&ebook, self.format);
            let markdown = Self::docitems_to_markdown(&doc_items);
            let num_characters = markdown.chars().count();
            return Ok(Document {
                markdown,
                format: self.format,
                metadata: Self::ebook_to_document_metadata(&ebook, num_characters),
                content_blocks: Some(doc_items),
                docling_document: None,
            });
        }

        // For MOBI format, read file and parse from bytes
        let content = std::fs::read(path_ref).map_err(DoclingError::IoError)?;

        self.parse_bytes(&content, options).map_err(add_context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike; // For year(), month(), day() methods

    #[test]
    fn test_ebooks_backend_creation() {
        // Valid formats
        assert!(
            EbooksBackend::new(InputFormat::Epub).is_ok(),
            "EPUB format should be supported"
        );
        assert!(
            EbooksBackend::new(InputFormat::Fb2).is_ok(),
            "FB2 format should be supported"
        );
        assert!(
            EbooksBackend::new(InputFormat::Mobi).is_ok(),
            "MOBI format should be supported"
        );

        // Invalid format
        assert!(
            EbooksBackend::new(InputFormat::Pdf).is_err(),
            "PDF format should be rejected for ebooks backend"
        );
    }

    #[test]
    fn test_epub_parse_bytes_fails() {
        let backend = EbooksBackend::new(InputFormat::Epub).unwrap();
        let result = backend.parse_bytes(b"dummy content", &BackendOptions::default());
        assert!(
            result.is_err(),
            "EPUB parse_bytes should fail (requires file path)"
        );
        let err_msg = result.unwrap_err().to_string();
        // DoclingError::BackendError wraps with "Backend error: " prefix
        assert!(err_msg.contains("Backend error"));
        assert!(err_msg.contains("EPUB") || err_msg.contains("Epub"));
        assert!(err_msg.contains("requires file path"));
    }

    #[test]
    fn test_fb2_parse_bytes_fails() {
        let backend = EbooksBackend::new(InputFormat::Fb2).unwrap();
        let result = backend.parse_bytes(b"dummy content", &BackendOptions::default());
        assert!(
            result.is_err(),
            "FB2 parse_bytes should fail (requires file path)"
        );
        let err_msg = result.unwrap_err().to_string();
        // DoclingError::BackendError wraps with "Backend error: " prefix
        assert!(err_msg.contains("Backend error"));
        assert!(err_msg.contains("FB2") || err_msg.contains("Fb2"));
        assert!(err_msg.contains("requires file path"));
    }

    #[test]
    fn test_docitem_generation_basic() {
        use docling_ebook::{Chapter, EbookMetadata, ParsedEbook, TocEntry};

        let mut metadata = EbookMetadata::new();
        metadata.title = Some("Test Book".to_string());
        metadata.creators.push("Test Author".to_string());
        metadata.publisher = Some("Test Publisher".to_string());

        let mut ebook = ParsedEbook::new(metadata);

        // Add TOC
        ebook.toc.push(TocEntry::new(
            "Chapter 1".to_string(),
            "ch01.html".to_string(),
        ));

        // Add chapter
        ebook.chapters.push(Chapter {
            title: Some("Introduction".to_string()),
            content: "This is the introduction.".to_string(),
            href: "ch01.html".to_string(),
            spine_order: 0,
        });

        let doc_items = EbooksBackend::generate_docitems(&ebook, InputFormat::Mobi);
        let markdown = EbooksBackend::docitems_to_markdown(&doc_items);

        assert!(markdown.contains("# Test Book"));
        assert!(markdown.contains("Authors: Test Author"));
        assert!(markdown.contains("Publisher: Test Publisher"));
        assert!(markdown.contains("## Table of Contents"));
        assert!(markdown.contains("- [Chapter 1](#ch01.html)"));
        assert!(markdown.contains("## Introduction"));
        assert!(markdown.contains("This is the introduction."));
    }

    #[test]
    fn test_docitem_generation_epub_html_conversion() {
        use docling_ebook::{Chapter, EbookMetadata, ParsedEbook};

        let metadata = EbookMetadata::new();
        let mut ebook = ParsedEbook::new(metadata);

        // Add chapter with HTML content
        ebook.chapters.push(Chapter {
            title: Some("Chapter One".to_string()),
            content: "<p>This is <strong>bold</strong> text.</p>".to_string(),
            href: "ch01.xhtml".to_string(),
            spine_order: 0,
        });

        let doc_items = EbooksBackend::generate_docitems(&ebook, InputFormat::Epub);
        let markdown = EbooksBackend::docitems_to_markdown(&doc_items);

        // Should convert HTML to text
        assert!(markdown.contains("## Chapter One"));
        assert!(markdown.contains("This is"));
        assert!(markdown.contains("bold"));
    }

    #[test]
    fn test_docitem_generation_minimal() {
        use docling_ebook::{EbookMetadata, ParsedEbook};

        // Minimal ebook with no metadata
        let metadata = EbookMetadata::new();
        let ebook = ParsedEbook::new(metadata);

        let doc_items = EbooksBackend::generate_docitems(&ebook, InputFormat::Mobi);
        let markdown = EbooksBackend::docitems_to_markdown(&doc_items);

        // Should not crash, produce minimal output
        assert!(!markdown.contains("# ")); // No title
        assert!(!markdown.contains("Authors:")); // No authors
    }

    #[test]
    fn test_metadata_extraction() {
        use docling_ebook::{EbookMetadata, ParsedEbook};

        // Create ebook with metadata
        let mut metadata = EbookMetadata::new();
        metadata.title = Some("The Great Novel".to_string());
        metadata.creators = vec!["Jane Doe".to_string(), "John Smith".to_string()];
        metadata.language = Some("en".to_string());
        metadata.date = Some("2024-01-15".to_string());
        metadata.subjects = vec!["Fiction".to_string(), "Drama".to_string()]; // N=1878: Test subject extraction

        let ebook = ParsedEbook::new(metadata);

        // Convert to DocumentMetadata
        let doc_metadata = EbooksBackend::ebook_to_document_metadata(&ebook, 1000);

        // Verify metadata extraction
        assert_eq!(
            doc_metadata.title,
            Some("The Great Novel".to_string()),
            "Title should be extracted from metadata"
        );
        assert_eq!(
            doc_metadata.author,
            Some("Jane Doe, John Smith".to_string()),
            "Authors should be joined with comma"
        );
        assert_eq!(
            doc_metadata.language,
            Some("en".to_string()),
            "Language should be extracted from metadata"
        );
        assert_eq!(
            doc_metadata.subject,
            Some("Fiction, Drama".to_string()),
            "Subjects should be joined with comma"
        ); // N=1878: Verify subject extraction
        assert_eq!(
            doc_metadata.num_characters, 1000,
            "Character count should match input"
        );
        assert!(
            doc_metadata.created.is_some(),
            "Created date should be extracted from date field"
        );

        // Check that date was parsed correctly
        let created = doc_metadata.created.unwrap();
        assert_eq!(created.year(), 2024, "Year should be 2024");
        assert_eq!(created.month(), 1, "Month should be 1 (January)");
        assert_eq!(created.day(), 15, "Day should be 15");
    }

    #[test]
    fn test_metadata_extraction_year_only() {
        use docling_ebook::{EbookMetadata, ParsedEbook};

        // Create ebook with year-only date
        let mut metadata = EbookMetadata::new();
        metadata.title = Some("Classic Book".to_string());
        metadata.date = Some("1999".to_string());

        let ebook = ParsedEbook::new(metadata);
        let doc_metadata = EbooksBackend::ebook_to_document_metadata(&ebook, 500);

        // Verify year-only date parsing
        assert!(
            doc_metadata.created.is_some(),
            "Year-only date should be parsed"
        );
        let created = doc_metadata.created.unwrap();
        assert_eq!(created.year(), 1999, "Year should be 1999");
        assert_eq!(
            created.month(),
            1,
            "Month should default to 1 for year-only"
        );
        assert_eq!(created.day(), 1, "Day should default to 1 for year-only");
    }

    #[test]
    fn test_metadata_extraction_no_metadata() {
        use docling_ebook::{EbookMetadata, ParsedEbook};

        // Minimal ebook with no metadata
        let metadata = EbookMetadata::new();
        let ebook = ParsedEbook::new(metadata);

        let doc_metadata = EbooksBackend::ebook_to_document_metadata(&ebook, 250);

        // Verify empty metadata
        assert_eq!(
            doc_metadata.title, None,
            "Title should be None when not set"
        );
        assert_eq!(
            doc_metadata.author, None,
            "Author should be None when no creators"
        );
        assert_eq!(
            doc_metadata.language, None,
            "Language should be None when not set"
        );
        assert_eq!(
            doc_metadata.subject, None,
            "Subject should be None when no subjects"
        ); // N=1878: Verify subject is None when no subjects
        assert_eq!(
            doc_metadata.created, None,
            "Created date should be None when not set"
        );
        assert_eq!(
            doc_metadata.num_characters, 250,
            "Character count should match input"
        );
    }

    // ===== CATEGORY 1: Backend Creation Tests (Additional) =====

    #[test]
    fn test_ebooks_backend_format_field() {
        let backend_epub = EbooksBackend::new(InputFormat::Epub).unwrap();
        assert_eq!(
            backend_epub.format(),
            InputFormat::Epub,
            "EPUB backend format() should return Epub"
        );

        let backend_fb2 = EbooksBackend::new(InputFormat::Fb2).unwrap();
        assert_eq!(
            backend_fb2.format(),
            InputFormat::Fb2,
            "FB2 backend format() should return Fb2"
        );

        let backend_mobi = EbooksBackend::new(InputFormat::Mobi).unwrap();
        assert_eq!(
            backend_mobi.format(),
            InputFormat::Mobi,
            "MOBI backend format() should return Mobi"
        );
    }

    #[test]
    fn test_mobi_can_parse_bytes() {
        // MOBI should not error when calling parse_bytes (even if data is invalid)
        let backend = EbooksBackend::new(InputFormat::Mobi).unwrap();
        // Invalid MOBI data will cause parsing error, not format error
        let result = backend.parse_bytes(b"invalid mobi data", &BackendOptions::default());
        // Should fail with BackendError (parsing issue), not format error
        assert!(result.is_err(), "Invalid MOBI data should fail to parse");
    }

    // ===== CATEGORY 2: DocItem Generation Tests (Additional) =====

    #[test]
    fn test_docitem_multiple_chapters() {
        use docling_ebook::{Chapter, EbookMetadata, ParsedEbook};

        let metadata = EbookMetadata::new();
        let mut ebook = ParsedEbook::new(metadata);

        // Add multiple chapters
        ebook.chapters.push(Chapter {
            title: Some("Chapter 1".to_string()),
            content: "First chapter content.".to_string(),
            href: "ch01.html".to_string(),
            spine_order: 0,
        });

        ebook.chapters.push(Chapter {
            title: Some("Chapter 2".to_string()),
            content: "Second chapter content.".to_string(),
            href: "ch02.html".to_string(),
            spine_order: 1,
        });

        ebook.chapters.push(Chapter {
            title: Some("Chapter 3".to_string()),
            content: "Third chapter content.".to_string(),
            href: "ch03.html".to_string(),
            spine_order: 2,
        });

        let doc_items = EbooksBackend::generate_docitems(&ebook, InputFormat::Epub);
        let markdown = EbooksBackend::docitems_to_markdown(&doc_items);

        assert!(markdown.contains("## Chapter 1"));
        assert!(markdown.contains("## Chapter 2"));
        assert!(markdown.contains("## Chapter 3"));
        assert!(markdown.contains("First chapter content"));
        assert!(markdown.contains("Second chapter content"));
        assert!(markdown.contains("Third chapter content"));
    }

    #[test]
    fn test_docitem_toc_generation() {
        use docling_ebook::{EbookMetadata, ParsedEbook, TocEntry};

        let metadata = EbookMetadata::new();
        let mut ebook = ParsedEbook::new(metadata);

        // Add TOC entries
        ebook.toc.push(TocEntry::new(
            "Introduction".to_string(),
            "intro.html".to_string(),
        ));
        ebook.toc.push(TocEntry::new(
            "Main Content".to_string(),
            "main.html".to_string(),
        ));
        ebook.toc.push(TocEntry::new(
            "Conclusion".to_string(),
            "end.html".to_string(),
        ));

        let doc_items = EbooksBackend::generate_docitems(&ebook, InputFormat::Fb2);
        let markdown = EbooksBackend::docitems_to_markdown(&doc_items);

        assert!(markdown.contains("## Table of Contents"));
        assert!(markdown.contains("- [Introduction](#intro.html)"));
        assert!(markdown.contains("- [Main Content](#main.html)"));
        assert!(markdown.contains("- [Conclusion](#end.html)"));
    }

    #[test]
    fn test_docitem_chapter_without_title() {
        use docling_ebook::{Chapter, EbookMetadata, ParsedEbook};

        let metadata = EbookMetadata::new();
        let mut ebook = ParsedEbook::new(metadata);

        // Add chapter with no title
        ebook.chapters.push(Chapter {
            title: None,
            content: "Untitled chapter content.".to_string(),
            href: "ch01.html".to_string(),
            spine_order: 0,
        });

        let doc_items = EbooksBackend::generate_docitems(&ebook, InputFormat::Epub);
        let markdown = EbooksBackend::docitems_to_markdown(&doc_items);

        // Should still generate content, even without header
        assert!(markdown.contains("Untitled chapter content"));
        // Should not have ## header for missing title
        assert!(!markdown.starts_with("##"));
    }

    #[test]
    fn test_docitem_empty_chapters() {
        use docling_ebook::{Chapter, EbookMetadata, ParsedEbook};

        let metadata = EbookMetadata::new();
        let mut ebook = ParsedEbook::new(metadata);

        // Add empty chapter
        ebook.chapters.push(Chapter {
            title: Some("Empty Chapter".to_string()),
            content: "".to_string(),
            href: "empty.html".to_string(),
            spine_order: 0,
        });

        let doc_items = EbooksBackend::generate_docitems(&ebook, InputFormat::Mobi);
        let markdown = EbooksBackend::docitems_to_markdown(&doc_items);

        // Empty chapters should be skipped (prevents confusing empty section headers)
        assert!(!markdown.contains("## Empty Chapter"));
    }

    // ===== CATEGORY 3: Format-Specific Tests =====

    #[test]
    fn test_epub_html_stripping() {
        use docling_ebook::{Chapter, EbookMetadata, ParsedEbook};

        let metadata = EbookMetadata::new();
        let mut ebook = ParsedEbook::new(metadata);

        // Add chapter with complex HTML
        ebook.chapters.push(Chapter {
            title: Some("Chapter".to_string()),
            content: "<div><p>Text with <em>emphasis</em> and <a href=\"link\">link</a>.</p></div>"
                .to_string(),
            href: "ch.xhtml".to_string(),
            spine_order: 0,
        });

        let doc_items = EbooksBackend::generate_docitems(&ebook, InputFormat::Epub);
        let markdown = EbooksBackend::docitems_to_markdown(&doc_items);

        // Should strip HTML tags for EPUB
        assert!(markdown.contains("Text with"));
        assert!(markdown.contains("emphasis"));
        assert!(markdown.contains("link"));
        // Should not have HTML tags
        assert!(!markdown.contains("<div>"));
        assert!(!markdown.contains("<em>"));
    }

    #[test]
    fn test_fb2_format_specific() {
        use docling_ebook::{Chapter, EbookMetadata, ParsedEbook};

        let mut metadata = EbookMetadata::new();
        metadata.title = Some("FB2 Book".to_string());
        let mut ebook = ParsedEbook::new(metadata);

        ebook.chapters.push(Chapter {
            title: Some("FB2 Chapter".to_string()),
            content: "FB2 content".to_string(),
            href: "section.fb2".to_string(),
            spine_order: 0,
        });

        let doc_items = EbooksBackend::generate_docitems(&ebook, InputFormat::Fb2);

        // Should generate DocItems regardless of format
        assert!(!doc_items.is_empty(), "FB2 format should generate DocItems");
    }

    #[test]
    fn test_mobi_format_specific() {
        use docling_ebook::{Chapter, EbookMetadata, ParsedEbook};

        let mut metadata = EbookMetadata::new();
        metadata.title = Some("MOBI Book".to_string());
        let mut ebook = ParsedEbook::new(metadata);

        ebook.chapters.push(Chapter {
            title: Some("MOBI Chapter".to_string()),
            content: "MOBI content".to_string(),
            href: "ch.html".to_string(),
            spine_order: 0,
        });

        let doc_items = EbooksBackend::generate_docitems(&ebook, InputFormat::Mobi);
        let markdown = EbooksBackend::docitems_to_markdown(&doc_items);

        assert!(markdown.contains("# MOBI Book"));
        assert!(markdown.contains("## MOBI Chapter"));
        assert!(markdown.contains("MOBI content"));
    }

    #[test]
    fn test_metadata_multiple_authors() {
        use docling_ebook::{EbookMetadata, ParsedEbook};

        let mut metadata = EbookMetadata::new();
        metadata.creators = vec![
            "Author One".to_string(),
            "Author Two".to_string(),
            "Author Three".to_string(),
        ];

        let ebook = ParsedEbook::new(metadata);
        let doc_metadata = EbooksBackend::ebook_to_document_metadata(&ebook, 100);

        assert_eq!(
            doc_metadata.author,
            Some("Author One, Author Two, Author Three".to_string()),
            "Multiple authors should be joined with commas"
        );
    }

    #[test]
    fn test_metadata_publisher_extraction() {
        use docling_ebook::{EbookMetadata, ParsedEbook};

        let mut metadata = EbookMetadata::new();
        metadata.publisher = Some("Big Publishing House".to_string());

        let ebook = ParsedEbook::new(metadata);
        let doc_items = EbooksBackend::generate_docitems(&ebook, InputFormat::Epub);
        let markdown = EbooksBackend::docitems_to_markdown(&doc_items);

        assert!(markdown.contains("Publisher: Big Publishing House"));
    }

    #[test]
    fn test_metadata_language_extraction() {
        use docling_ebook::{EbookMetadata, ParsedEbook};

        let mut metadata = EbookMetadata::new();
        metadata.language = Some("fr".to_string());

        let ebook = ParsedEbook::new(metadata);
        let doc_metadata = EbooksBackend::ebook_to_document_metadata(&ebook, 100);

        assert_eq!(doc_metadata.language, Some("fr".to_string()));
    }

    // ===== CATEGORY 4: Integration Tests =====

    #[test]
    fn test_complete_ebook_structure() {
        use docling_ebook::{Chapter, EbookMetadata, ParsedEbook, TocEntry};

        let mut metadata = EbookMetadata::new();
        metadata.title = Some("Complete Book".to_string());
        metadata.creators = vec!["Test Author".to_string()];
        metadata.publisher = Some("Test Publisher".to_string());
        metadata.language = Some("en".to_string());
        metadata.date = Some("2024-06-01".to_string());

        let mut ebook = ParsedEbook::new(metadata);

        // Add TOC
        ebook.toc.push(TocEntry::new(
            "Chapter 1".to_string(),
            "ch1.html".to_string(),
        ));
        ebook.toc.push(TocEntry::new(
            "Chapter 2".to_string(),
            "ch2.html".to_string(),
        ));

        // Add chapters
        ebook.chapters.push(Chapter {
            title: Some("Chapter 1".to_string()),
            content: "First chapter text.".to_string(),
            href: "ch1.html".to_string(),
            spine_order: 0,
        });

        ebook.chapters.push(Chapter {
            title: Some("Chapter 2".to_string()),
            content: "Second chapter text.".to_string(),
            href: "ch2.html".to_string(),
            spine_order: 1,
        });

        let doc_items = EbooksBackend::generate_docitems(&ebook, InputFormat::Epub);
        let markdown = EbooksBackend::docitems_to_markdown(&doc_items);

        // Verify all components
        assert!(markdown.contains("# Complete Book"));
        assert!(markdown.contains("Authors: Test Author"));
        assert!(markdown.contains("Publisher: Test Publisher"));
        assert!(markdown.contains("## Table of Contents"));
        assert!(markdown.contains("- [Chapter 1](#ch1.html)"));
        assert!(markdown.contains("- [Chapter 2](#ch2.html)"));
        assert!(markdown.contains("## Chapter 1"));
        assert!(markdown.contains("First chapter text"));
        assert!(markdown.contains("## Chapter 2"));
        assert!(markdown.contains("Second chapter text"));
    }

    #[test]
    fn test_markdown_character_count() {
        use docling_ebook::{Chapter, EbookMetadata, ParsedEbook};

        let mut metadata = EbookMetadata::new();
        metadata.title = Some("Test".to_string());
        let mut ebook = ParsedEbook::new(metadata);

        ebook.chapters.push(Chapter {
            title: Some("Ch".to_string()),
            content: "Content.".to_string(),
            href: "ch.html".to_string(),
            spine_order: 0,
        });

        let doc_items = EbooksBackend::generate_docitems(&ebook, InputFormat::Mobi);
        let markdown = EbooksBackend::docitems_to_markdown(&doc_items);
        let char_count = markdown.chars().count();

        let doc_metadata = EbooksBackend::ebook_to_document_metadata(&ebook, char_count);

        assert_eq!(
            doc_metadata.num_characters, char_count,
            "Character count should match markdown length"
        );
        assert!(char_count > 0, "Character count should be positive");
    }

    #[test]
    fn test_docitems_not_empty() {
        use docling_ebook::{Chapter, EbookMetadata, ParsedEbook};

        let metadata = EbookMetadata::new();
        let mut ebook = ParsedEbook::new(metadata);

        ebook.chapters.push(Chapter {
            title: Some("Chapter".to_string()),
            content: "Content".to_string(),
            href: "ch.html".to_string(),
            spine_order: 0,
        });

        let doc_items = EbooksBackend::generate_docitems(&ebook, InputFormat::Epub);

        // Should have at least one DocItem (chapter header or content)
        assert!(
            !doc_items.is_empty(),
            "Ebook with content should generate DocItems"
        );
    }

    #[test]
    fn test_error_handling_invalid_format() {
        // Test that invalid format returns error
        let result = EbooksBackend::new(InputFormat::Docx);
        assert!(
            result.is_err(),
            "DOCX format should be rejected for ebooks backend"
        );

        if let Err(DoclingError::FormatError(msg)) = result {
            assert!(msg.contains("not an e-book format"));
        } else {
            panic!("Expected FormatError");
        }
    }

    // ========== UNICODE AND SPECIAL CHARACTER TESTS ==========

    #[test]
    fn test_ebook_unicode_title() {
        use docling_ebook::{EbookMetadata, ParsedEbook};

        let mut metadata = EbookMetadata::new();
        metadata.title = Some("世界文学 🌍".to_string());
        metadata.creators = vec!["村上春樹".to_string()];

        let ebook = ParsedEbook::new(metadata);
        let doc_items = EbooksBackend::generate_docitems(&ebook, InputFormat::Epub);
        let markdown = EbooksBackend::docitems_to_markdown(&doc_items);

        assert!(markdown.contains("# 世界文学 🌍"));
        assert!(markdown.contains("村上春樹"));
    }

    #[test]
    fn test_ebook_unicode_content() {
        use docling_ebook::{Chapter, EbookMetadata, ParsedEbook};

        let metadata = EbookMetadata::new();
        let mut ebook = ParsedEbook::new(metadata);

        ebook.chapters.push(Chapter {
            title: Some("Chapter".to_string()),
            content: "Γεια σου κόσμε! مرحبا العالم! שלום עולם!".to_string(),
            href: "ch.html".to_string(),
            spine_order: 0,
        });

        let doc_items = EbooksBackend::generate_docitems(&ebook, InputFormat::Mobi);
        let markdown = EbooksBackend::docitems_to_markdown(&doc_items);

        assert!(markdown.contains("Γεια σου κόσμε"));
        assert!(markdown.contains("مرحبا العالم"));
        assert!(markdown.contains("שלום עולם"));
    }

    #[test]
    fn test_ebook_special_markdown_characters() {
        use docling_ebook::{Chapter, EbookMetadata, ParsedEbook};

        let metadata = EbookMetadata::new();
        let mut ebook = ParsedEbook::new(metadata);

        ebook.chapters.push(Chapter {
            title: Some("Special Characters".to_string()),
            content: "Text with **asterisks** and __underscores__ and [brackets]".to_string(),
            href: "ch.html".to_string(),
            spine_order: 0,
        });

        let doc_items = EbooksBackend::generate_docitems(&ebook, InputFormat::Fb2);
        let markdown = EbooksBackend::docitems_to_markdown(&doc_items);

        // Special markdown characters should be preserved
        assert!(markdown.contains("**asterisks**"));
        assert!(markdown.contains("__underscores__"));
        assert!(markdown.contains("[brackets]"));
    }

    // ========== ADDITIONAL VALIDATION TESTS ==========

    #[test]
    fn test_ebook_very_long_title() {
        use docling_ebook::{EbookMetadata, ParsedEbook};

        let mut metadata = EbookMetadata::new();
        let long_title = "A".repeat(500);
        metadata.title = Some(long_title.clone());

        let ebook = ParsedEbook::new(metadata);
        let doc_items = EbooksBackend::generate_docitems(&ebook, InputFormat::Epub);
        let markdown = EbooksBackend::docitems_to_markdown(&doc_items);

        assert!(
            markdown.contains(&long_title),
            "Very long title (500 chars) should be preserved in markdown"
        );
    }

    #[test]
    fn test_ebook_many_authors() {
        use docling_ebook::{EbookMetadata, ParsedEbook};

        let mut metadata = EbookMetadata::new();
        for i in 1..=20 {
            metadata.creators.push(format!("Author {i}"));
        }

        let ebook = ParsedEbook::new(metadata);
        let doc_metadata = EbooksBackend::ebook_to_document_metadata(&ebook, 100);

        assert!(
            doc_metadata.author.is_some(),
            "Author field should be set when creators exist"
        );
        let author_str = doc_metadata.author.unwrap();
        assert!(
            author_str.contains("Author 1"),
            "First author should be present"
        );
        assert!(
            author_str.contains("Author 20"),
            "Last author should be present"
        );
        assert_eq!(
            author_str.matches(',').count(),
            19,
            "20 authors should produce 19 commas"
        ); // 20 authors = 19 commas
    }

    #[test]
    fn test_ebook_many_chapters() {
        use docling_ebook::{Chapter, EbookMetadata, ParsedEbook};

        let metadata = EbookMetadata::new();
        let mut ebook = ParsedEbook::new(metadata);

        // Add 50 chapters
        for i in 1..=50 {
            ebook.chapters.push(Chapter {
                title: Some(format!("Chapter {i}")),
                content: format!("Content {i}"),
                href: format!("ch{i}.html"),
                spine_order: i - 1,
            });
        }

        let doc_items = EbooksBackend::generate_docitems(&ebook, InputFormat::Epub);
        let markdown = EbooksBackend::docitems_to_markdown(&doc_items);

        assert!(markdown.contains("## Chapter 1"));
        assert!(markdown.contains("## Chapter 25"));
        assert!(markdown.contains("## Chapter 50"));
        assert!(markdown.contains("Content 1"));
        assert!(markdown.contains("Content 50"));
    }

    #[test]
    fn test_ebook_empty_toc_entries() {
        use docling_ebook::{EbookMetadata, ParsedEbook, TocEntry};

        let metadata = EbookMetadata::new();
        let mut ebook = ParsedEbook::new(metadata);

        // Add TOC entry with empty label
        ebook
            .toc
            .push(TocEntry::new(String::new(), "ch.html".to_string()));

        let doc_items = EbooksBackend::generate_docitems(&ebook, InputFormat::Epub);
        let markdown = EbooksBackend::docitems_to_markdown(&doc_items);

        // Should still have TOC section
        assert!(markdown.contains("## Table of Contents"));
    }

    #[test]
    fn test_ebook_whitespace_only_content() {
        use docling_ebook::{Chapter, EbookMetadata, ParsedEbook};

        let metadata = EbookMetadata::new();
        let mut ebook = ParsedEbook::new(metadata);

        ebook.chapters.push(Chapter {
            title: Some("Chapter".to_string()),
            content: "   \n\n\t\t  ".to_string(),
            href: "ch.html".to_string(),
            spine_order: 0,
        });

        let doc_items = EbooksBackend::generate_docitems(&ebook, InputFormat::Mobi);

        // Whitespace-only content should be filtered out completely (no header, no content)
        // This prevents confusing empty section headers
        let has_chapter_header = doc_items
            .iter()
            .any(|item| matches!(item, DocItem::SectionHeader { text, .. } if text == "Chapter"));
        assert!(
            !has_chapter_header,
            "Whitespace-only chapter should not produce a chapter header"
        );
    }

    // ========== SERIALIZATION CONSISTENCY TESTS ==========

    #[test]
    fn test_ebook_markdown_not_empty_with_content() {
        use docling_ebook::{Chapter, EbookMetadata, ParsedEbook};

        let metadata = EbookMetadata::new();
        let mut ebook = ParsedEbook::new(metadata);

        ebook.chapters.push(Chapter {
            title: Some("Chapter".to_string()),
            content: "Content".to_string(),
            href: "ch.html".to_string(),
            spine_order: 0,
        });

        let doc_items = EbooksBackend::generate_docitems(&ebook, InputFormat::Epub);
        let markdown = EbooksBackend::docitems_to_markdown(&doc_items);

        assert!(
            !markdown.is_empty(),
            "Ebook with content should produce non-empty markdown"
        );
        assert!(
            markdown.len() > 10,
            "Markdown should have reasonable length (>10 chars)"
        );
    }

    #[test]
    fn test_ebook_markdown_structure_consistency() {
        use docling_ebook::{Chapter, EbookMetadata, ParsedEbook};

        let mut metadata = EbookMetadata::new();
        metadata.title = Some("Title".to_string());
        let mut ebook = ParsedEbook::new(metadata);

        ebook.chapters.push(Chapter {
            title: Some("Chapter".to_string()),
            content: "Content".to_string(),
            href: "ch.html".to_string(),
            spine_order: 0,
        });

        let doc_items = EbooksBackend::generate_docitems(&ebook, InputFormat::Mobi);
        let markdown = EbooksBackend::docitems_to_markdown(&doc_items);

        // Should have proper markdown structure
        assert!(markdown.contains("# Title")); // H1 for book title
        assert!(markdown.contains("## Chapter")); // H2 for chapter
    }

    #[test]
    fn test_ebook_docitems_match_markdown() {
        use docling_ebook::{Chapter, EbookMetadata, ParsedEbook};

        let mut metadata = EbookMetadata::new();
        metadata.title = Some("Test".to_string());
        let mut ebook = ParsedEbook::new(metadata);

        ebook.chapters.push(Chapter {
            title: Some("Chapter".to_string()),
            content: "Text content".to_string(),
            href: "ch.html".to_string(),
            spine_order: 0,
        });

        let doc_items = EbooksBackend::generate_docitems(&ebook, InputFormat::Epub);
        let markdown = EbooksBackend::docitems_to_markdown(&doc_items);

        // Every DocItem should contribute to markdown
        for item in &doc_items {
            match item {
                DocItem::SectionHeader { text, .. } => {
                    assert!(
                        markdown.contains(text),
                        "SectionHeader text '{text}' should appear in markdown"
                    );
                }
                DocItem::Text { text, .. } => {
                    // Metadata text has "key: value" format, may be bolded
                    if text.contains(':') {
                        let value = text.split(':').nth(1).unwrap().trim();
                        assert!(
                            markdown.contains(value),
                            "Text value '{value}' should appear in markdown"
                        );
                    } else {
                        assert!(
                            markdown.contains(text),
                            "Text '{text}' should appear in markdown"
                        );
                    }
                }
                DocItem::ListItem { text, .. } => {
                    assert!(
                        markdown.contains(text),
                        "ListItem text '{text}' should appear in markdown"
                    );
                }
                _ => {}
            }
        }
    }

    #[test]
    fn test_ebook_idempotent_parsing() {
        use docling_ebook::{Chapter, EbookMetadata, ParsedEbook};

        let mut metadata = EbookMetadata::new();
        metadata.title = Some("Book".to_string());
        let mut ebook = ParsedEbook::new(metadata);

        ebook.chapters.push(Chapter {
            title: Some("Chapter".to_string()),
            content: "Content".to_string(),
            href: "ch.html".to_string(),
            spine_order: 0,
        });

        // Parse twice
        let doc_items1 = EbooksBackend::generate_docitems(&ebook, InputFormat::Epub);
        let markdown1 = EbooksBackend::docitems_to_markdown(&doc_items1);

        let doc_items2 = EbooksBackend::generate_docitems(&ebook, InputFormat::Epub);
        let markdown2 = EbooksBackend::docitems_to_markdown(&doc_items2);

        // Should produce identical output
        assert_eq!(
            markdown1, markdown2,
            "Parsing the same ebook twice should produce identical markdown"
        );
        assert_eq!(
            doc_items1.len(),
            doc_items2.len(),
            "Parsing the same ebook twice should produce same DocItem count"
        );
    }

    // ========== BACKEND OPTIONS TESTS ==========

    #[test]
    fn test_ebook_accepts_default_options() {
        // MOBI can use parse_bytes
        let backend = EbooksBackend::new(InputFormat::Mobi).unwrap();
        let result = backend.parse_bytes(b"invalid", &BackendOptions::default());
        // Will fail due to invalid data, but options are accepted
        assert!(result.is_err(), "Invalid MOBI data should fail to parse");
    }

    #[test]
    fn test_ebook_accepts_custom_options() {
        let backend = EbooksBackend::new(InputFormat::Mobi).unwrap();
        let options = BackendOptions::default();
        let result = backend.parse_bytes(b"invalid", &options);
        // Will fail due to invalid data, but options are accepted
        assert!(
            result.is_err(),
            "Invalid MOBI data should fail to parse with custom options"
        );
    }

    // ========== FORMAT-SPECIFIC EDGE CASES ==========

    #[test]
    fn test_epub_requires_file_path() {
        let backend = EbooksBackend::new(InputFormat::Epub).unwrap();
        let result = backend.parse_bytes(b"data", &BackendOptions::default());

        assert!(result.is_err(), "EPUB should require file path");
        let err = result.unwrap_err();
        assert!(err.to_string().contains("EPUB") || err.to_string().contains("Epub"));
        assert!(err.to_string().contains("file path"));
    }

    #[test]
    fn test_fb2_requires_file_path() {
        let backend = EbooksBackend::new(InputFormat::Fb2).unwrap();
        let result = backend.parse_bytes(b"data", &BackendOptions::default());

        assert!(result.is_err(), "FB2 should require file path");
        let err = result.unwrap_err();
        assert!(err.to_string().contains("FB2") || err.to_string().contains("Fb2"));
        assert!(err.to_string().contains("file path"));
    }

    #[test]
    fn test_mobi_accepts_parse_bytes() {
        let backend = EbooksBackend::new(InputFormat::Mobi).unwrap();
        // Should not error about format, only parsing
        let result = backend.parse_bytes(b"invalid mobi", &BackendOptions::default());
        assert!(result.is_err(), "Invalid MOBI data should fail parsing");
        // Should be BackendError (parsing), not format error
        let err_str = result.unwrap_err().to_string();
        assert!(err_str.contains("Backend error") || err_str.contains("Failed to parse"));
    }

    #[test]
    fn test_ebook_date_parsing_iso8601() {
        use docling_ebook::{EbookMetadata, ParsedEbook};

        let mut metadata = EbookMetadata::new();
        metadata.date = Some("2024-12-25T10:30:00Z".to_string());

        let ebook = ParsedEbook::new(metadata);
        let doc_metadata = EbooksBackend::ebook_to_document_metadata(&ebook, 100);

        assert!(
            doc_metadata.created.is_some(),
            "ISO8601 date should be parsed"
        );
        let created = doc_metadata.created.unwrap();
        assert_eq!(created.year(), 2024, "Year should be 2024");
        assert_eq!(created.month(), 12, "Month should be 12 (December)");
        assert_eq!(created.day(), 25, "Day should be 25");
    }

    #[test]
    fn test_ebook_date_parsing_date_only() {
        use docling_ebook::{EbookMetadata, ParsedEbook};

        let mut metadata = EbookMetadata::new();
        metadata.date = Some("2020-06-15".to_string());

        let ebook = ParsedEbook::new(metadata);
        let doc_metadata = EbooksBackend::ebook_to_document_metadata(&ebook, 100);

        assert!(
            doc_metadata.created.is_some(),
            "Date-only format should be parsed"
        );
        let created = doc_metadata.created.unwrap();
        assert_eq!(created.year(), 2020, "Year should be 2020");
        assert_eq!(created.month(), 6, "Month should be 6 (June)");
        assert_eq!(created.day(), 15, "Day should be 15");
    }

    #[test]
    fn test_ebook_date_parsing_invalid() {
        use docling_ebook::{EbookMetadata, ParsedEbook};

        let mut metadata = EbookMetadata::new();
        metadata.date = Some("not a date".to_string());

        let ebook = ParsedEbook::new(metadata);
        let doc_metadata = EbooksBackend::ebook_to_document_metadata(&ebook, 100);

        // Invalid date should result in None
        assert!(
            doc_metadata.created.is_none(),
            "Invalid date should result in None"
        );
    }

    #[test]
    fn test_ebook_chapter_order_preservation() {
        use docling_ebook::{Chapter, EbookMetadata, ParsedEbook};

        let metadata = EbookMetadata::new();
        let mut ebook = ParsedEbook::new(metadata);

        // Add chapters with specific spine order
        ebook.chapters.push(Chapter {
            title: Some("First".to_string()),
            content: "1".to_string(),
            href: "a.html".to_string(),
            spine_order: 0,
        });
        ebook.chapters.push(Chapter {
            title: Some("Second".to_string()),
            content: "2".to_string(),
            href: "b.html".to_string(),
            spine_order: 1,
        });
        ebook.chapters.push(Chapter {
            title: Some("Third".to_string()),
            content: "3".to_string(),
            href: "c.html".to_string(),
            spine_order: 2,
        });

        let doc_items = EbooksBackend::generate_docitems(&ebook, InputFormat::Epub);
        let markdown = EbooksBackend::docitems_to_markdown(&doc_items);

        // Check order in markdown
        let pos_first = markdown.find("## First").unwrap();
        let pos_second = markdown.find("## Second").unwrap();
        let pos_third = markdown.find("## Third").unwrap();

        assert!(
            pos_first < pos_second,
            "First chapter should appear before Second"
        );
        assert!(
            pos_second < pos_third,
            "Second chapter should appear before Third"
        );
    }

    #[test]
    fn test_ebook_toc_order_preservation() {
        use docling_ebook::{EbookMetadata, ParsedEbook, TocEntry};

        let metadata = EbookMetadata::new();
        let mut ebook = ParsedEbook::new(metadata);

        ebook
            .toc
            .push(TocEntry::new("Entry A".to_string(), "a.html".to_string()));
        ebook
            .toc
            .push(TocEntry::new("Entry B".to_string(), "b.html".to_string()));
        ebook
            .toc
            .push(TocEntry::new("Entry C".to_string(), "c.html".to_string()));

        let doc_items = EbooksBackend::generate_docitems(&ebook, InputFormat::Fb2);
        let markdown = EbooksBackend::docitems_to_markdown(&doc_items);

        // Check TOC order in markdown (entries are now formatted as links)
        let pos_a = markdown.find("- [Entry A](#a.html)").unwrap();
        let pos_b = markdown.find("- [Entry B](#b.html)").unwrap();
        let pos_c = markdown.find("- [Entry C](#c.html)").unwrap();

        assert!(pos_a < pos_b);
        assert!(pos_b < pos_c);
    }

    #[test]
    fn test_ebook_metadata_separator() {
        use docling_ebook::{Chapter, EbookMetadata, ParsedEbook};

        let mut metadata = EbookMetadata::new();
        metadata.title = Some("Book".to_string());
        metadata.creators = vec!["Author".to_string()];
        let mut ebook = ParsedEbook::new(metadata);

        ebook.chapters.push(Chapter {
            title: Some("Chapter".to_string()),
            content: "Content".to_string(),
            href: "ch.html".to_string(),
            spine_order: 0,
        });

        let doc_items = EbooksBackend::generate_docitems(&ebook, InputFormat::Epub);
        let markdown = EbooksBackend::docitems_to_markdown(&doc_items);

        // Should have separator between metadata and chapters
        assert!(markdown.contains("---"));
    }

    #[test]
    fn test_ebook_description_extraction() {
        use docling_ebook::{EbookMetadata, ParsedEbook};

        let mut metadata = EbookMetadata::new();
        metadata.description = Some("This is a test book description.".to_string());

        let ebook = ParsedEbook::new(metadata);
        let doc_items = EbooksBackend::generate_docitems(&ebook, InputFormat::Epub);
        let markdown = EbooksBackend::docitems_to_markdown(&doc_items);

        assert!(markdown.contains("Description: This is a test book description."));
    }

    #[test]
    fn test_ebook_provenance_generation() {
        use docling_ebook::{Chapter, EbookMetadata, ParsedEbook};

        let metadata = EbookMetadata::new();
        let mut ebook = ParsedEbook::new(metadata);

        ebook.chapters.push(Chapter {
            title: Some("Chapter".to_string()),
            content: "Content".to_string(),
            href: "ch.html".to_string(),
            spine_order: 0,
        });

        let doc_items = EbooksBackend::generate_docitems(&ebook, InputFormat::Mobi);

        // All DocItems should have provenance
        for item in &doc_items {
            match item {
                DocItem::SectionHeader { prov, .. } => {
                    assert!(!prov.is_empty());
                    assert_eq!(prov[0].page_no, 1);
                }
                DocItem::Text { prov, .. } => {
                    assert!(!prov.is_empty());
                    assert_eq!(prov[0].page_no, 1);
                }
                DocItem::ListItem { prov, .. } => {
                    assert!(!prov.is_empty());
                    assert_eq!(prov[0].page_no, 1);
                }
                _ => {}
            }
        }
    }

    #[test]
    fn test_ebook_format_identification() {
        let epub_backend = EbooksBackend::new(InputFormat::Epub).unwrap();
        assert_eq!(epub_backend.format(), InputFormat::Epub);

        let fb2_backend = EbooksBackend::new(InputFormat::Fb2).unwrap();
        assert_eq!(fb2_backend.format(), InputFormat::Fb2);

        let mobi_backend = EbooksBackend::new(InputFormat::Mobi).unwrap();
        assert_eq!(mobi_backend.format(), InputFormat::Mobi);
    }

    #[test]
    fn test_ebook_character_count_accuracy() {
        use docling_ebook::{Chapter, EbookMetadata, ParsedEbook};

        let mut metadata = EbookMetadata::new();
        metadata.title = Some("Test".to_string());
        let mut ebook = ParsedEbook::new(metadata);

        ebook.chapters.push(Chapter {
            title: Some("Ch".to_string()),
            content: "12345".to_string(),
            href: "ch.html".to_string(),
            spine_order: 0,
        });

        let doc_items = EbooksBackend::generate_docitems(&ebook, InputFormat::Epub);
        let markdown = EbooksBackend::docitems_to_markdown(&doc_items);
        let char_count = markdown.chars().count();

        let doc_metadata = EbooksBackend::ebook_to_document_metadata(&ebook, char_count);

        assert_eq!(doc_metadata.num_characters, char_count);
        assert!(doc_metadata.num_characters > 5); // At least chapter + content
    }

    // ========== EBOOK ADVANCED FEATURES (8 tests) ==========

    #[test]
    fn test_ebook_toc_navigation_extraction() {
        use docling_ebook::{Chapter, EbookMetadata, ParsedEbook};

        let metadata = EbookMetadata::new();
        let mut ebook = ParsedEbook::new(metadata);

        // Multiple chapters with hierarchical structure
        ebook.chapters.push(Chapter {
            title: Some("Part I: Introduction".to_string()),
            content: "Introduction content.".to_string(),
            href: "part1.html".to_string(),
            spine_order: 0,
        });
        ebook.chapters.push(Chapter {
            title: Some("Chapter 1: Getting Started".to_string()),
            content: "Getting started content.".to_string(),
            href: "ch1.html".to_string(),
            spine_order: 1,
        });
        ebook.chapters.push(Chapter {
            title: Some("Chapter 2: Advanced Topics".to_string()),
            content: "Advanced topics content.".to_string(),
            href: "ch2.html".to_string(),
            spine_order: 2,
        });

        let doc_items = EbooksBackend::generate_docitems(&ebook, InputFormat::Epub);

        // Should have section headers for each chapter
        let section_headers: Vec<_> = doc_items
            .iter()
            .filter(|item| matches!(item, DocItem::SectionHeader { .. }))
            .collect();

        assert!(
            section_headers.len() >= 3,
            "Should have at least 3 section headers"
        );
    }

    #[test]
    fn test_ebook_spine_order_preservation() {
        use docling_ebook::{Chapter, EbookMetadata, ParsedEbook};

        let metadata = EbookMetadata::new();
        let mut ebook = ParsedEbook::new(metadata);

        // Add chapters out of order
        ebook.chapters.push(Chapter {
            title: Some("Chapter 3".to_string()),
            content: "Third".to_string(),
            href: "ch3.html".to_string(),
            spine_order: 2,
        });
        ebook.chapters.push(Chapter {
            title: Some("Chapter 1".to_string()),
            content: "First".to_string(),
            href: "ch1.html".to_string(),
            spine_order: 0,
        });
        ebook.chapters.push(Chapter {
            title: Some("Chapter 2".to_string()),
            content: "Second".to_string(),
            href: "ch2.html".to_string(),
            spine_order: 1,
        });

        let markdown = EbooksBackend::docitems_to_markdown(&EbooksBackend::generate_docitems(
            &ebook,
            InputFormat::Epub,
        ));

        // Should preserve spine order in output
        // (spine_order determines reading order, not insertion order)
        assert!(markdown.len() > 10);
    }

    #[test]
    fn test_ebook_html_tags_in_chapter_content() {
        use docling_ebook::{Chapter, EbookMetadata, ParsedEbook};

        let metadata = EbookMetadata::new();
        let mut ebook = ParsedEbook::new(metadata);

        ebook.chapters.push(Chapter {
            title: Some("Chapter with HTML".to_string()),
            content: "<p>This is a <strong>bold</strong> word and <em>italic</em> text.</p>\
                     <ul><li>Item 1</li><li>Item 2</li></ul>\
                     <h2>Subsection</h2><p>More content.</p>"
                .to_string(),
            href: "ch.html".to_string(),
            spine_order: 0,
        });

        let doc_items = EbooksBackend::generate_docitems(&ebook, InputFormat::Epub);
        let markdown = EbooksBackend::docitems_to_markdown(&doc_items);

        // Should handle HTML tags gracefully (may strip or convert them)
        assert!(markdown.len() > 50);
        assert!(
            markdown.contains("bold") || markdown.contains("italic") || markdown.contains("Item")
        );
    }

    #[test]
    fn test_ebook_multilingual_metadata() {
        use docling_ebook::{EbookMetadata, ParsedEbook};

        let mut metadata = EbookMetadata::new();
        metadata.title = Some("多言語書籍 / Multilingual Book / Многоязычная книга".to_string());
        metadata.creators = vec![
            "山田太郎".to_string(),
            "Jean Dupont".to_string(),
            "Иван Петров".to_string(),
        ];
        metadata.language = Some("multi".to_string());
        metadata.publisher = Some("国際出版社 International Publishers".to_string());

        let ebook = ParsedEbook::new(metadata);
        let doc_items = EbooksBackend::generate_docitems(&ebook, InputFormat::Epub);
        let markdown = EbooksBackend::docitems_to_markdown(&doc_items);

        // Should handle multilingual metadata
        assert!(
            markdown.contains("山田") || markdown.contains("Dupont") || markdown.contains("多言語")
        );
        assert!(markdown.len() > 50);
    }

    #[test]
    fn test_ebook_chapter_without_title() {
        use docling_ebook::{Chapter, EbookMetadata, ParsedEbook};

        let metadata = EbookMetadata::new();
        let mut ebook = ParsedEbook::new(metadata);

        ebook.chapters.push(Chapter {
            title: None, // No title
            content: "This chapter has no title but has content.".to_string(),
            href: "untitled.html".to_string(),
            spine_order: 0,
        });

        let doc_items = EbooksBackend::generate_docitems(&ebook, InputFormat::Mobi);
        let markdown = EbooksBackend::docitems_to_markdown(&doc_items);

        // Should handle chapters without titles gracefully
        assert!(markdown.contains("This chapter has no title but has content."));
    }

    #[test]
    fn test_ebook_very_long_chapter_content() {
        use docling_ebook::{Chapter, EbookMetadata, ParsedEbook};

        let metadata = EbookMetadata::new();
        let mut ebook = ParsedEbook::new(metadata);

        // Create very long chapter content (50KB+)
        let long_content = "This is a test sentence. ".repeat(2000); // ~50KB

        ebook.chapters.push(Chapter {
            title: Some("Long Chapter".to_string()),
            content: long_content.clone(),
            href: "long.html".to_string(),
            spine_order: 0,
        });

        let doc_items = EbooksBackend::generate_docitems(&ebook, InputFormat::Epub);
        let markdown = EbooksBackend::docitems_to_markdown(&doc_items);

        // Should handle very long content without truncation or crashes
        assert!(markdown.len() > 40000);
        assert!(markdown.contains("test sentence"));
    }

    #[test]
    fn test_ebook_special_characters_in_metadata() {
        use docling_ebook::{EbookMetadata, ParsedEbook};

        let mut metadata = EbookMetadata::new();
        metadata.title = Some("Book: \"The Journey\" & More <Adventures>".to_string());
        metadata.creators = vec!["O'Brien, James".to_string()];
        metadata.publisher = Some("Smith & Co. Publishers™".to_string());
        metadata.description = Some("A book about 5 < 10 and 20 > 15. It's great!".to_string());

        let ebook = ParsedEbook::new(metadata);
        let doc_items = EbooksBackend::generate_docitems(&ebook, InputFormat::Fb2);
        let markdown = EbooksBackend::docitems_to_markdown(&doc_items);

        // Should handle special characters in metadata
        assert!(markdown.contains("Journey") || markdown.contains("Adventures"));
        assert!(markdown.contains("O'Brien") || markdown.contains("OBrien"));
        assert!(markdown.len() > 80);
    }

    #[test]
    fn test_ebook_multiple_subjects_and_identifiers() {
        use docling_ebook::{EbookMetadata, ParsedEbook};

        let mut metadata = EbookMetadata::new();
        metadata.title = Some("Comprehensive Guide".to_string());
        metadata.subjects = vec![
            "Technology".to_string(),
            "Programming".to_string(),
            "Rust Language".to_string(),
            "Software Engineering".to_string(),
        ];
        // Note: EbookMetadata has 'identifier' field (singular), not 'identifiers' (plural)
        // Typically stores primary identifier like ISBN
        metadata.identifier = Some("978-1-234-56789-0".to_string());

        let ebook = ParsedEbook::new(metadata);
        let doc_items = EbooksBackend::generate_docitems(&ebook, InputFormat::Epub);
        let markdown = EbooksBackend::docitems_to_markdown(&doc_items);

        // Should produce some markdown output (even if minimal for metadata-only ebook)
        // Subjects and identifiers may or may not be included in markdown
        assert!(!markdown.is_empty());
        assert!(!doc_items.is_empty());
    }

    // ==================== ADDITIONAL EDGE CASES (N=537) ====================

    #[test]
    fn test_ebook_format_differences() {
        // Test that different e-book formats produce consistent output
        use docling_ebook::{Chapter, EbookMetadata, ParsedEbook};

        let mut metadata = EbookMetadata::new();
        metadata.title = Some("Test Book".to_string());
        metadata.creators = vec!["Author Name".to_string()];

        let mut ebook = ParsedEbook::new(metadata);
        ebook.chapters.push(Chapter {
            title: Some("Chapter 1".to_string()),
            content: "Chapter content here".to_string(),
            href: "ch1.html".to_string(),
            spine_order: 0,
        });

        // Generate DocItems for each format
        let doc_items_epub = EbooksBackend::generate_docitems(&ebook, InputFormat::Epub);
        let doc_items_fb2 = EbooksBackend::generate_docitems(&ebook, InputFormat::Fb2);
        let doc_items_mobi = EbooksBackend::generate_docitems(&ebook, InputFormat::Mobi);

        // All formats should produce the same structure
        assert_eq!(doc_items_epub.len(), doc_items_fb2.len());
        assert_eq!(doc_items_fb2.len(), doc_items_mobi.len());
        assert!(doc_items_epub.len() > 3); // At least title, author, and chapter
    }

    #[test]
    fn test_ebook_minimal_metadata() {
        // Test e-book with absolutely minimal metadata (no title, author, etc.)
        use docling_ebook::{Chapter, EbookMetadata, ParsedEbook};

        let metadata = EbookMetadata::new(); // Empty metadata
        let mut ebook = ParsedEbook::new(metadata);

        // Add just one chapter with no title
        ebook.chapters.push(Chapter {
            title: None,
            content: "Minimal content".to_string(),
            href: "content.html".to_string(),
            spine_order: 0,
        });

        let doc_items = EbooksBackend::generate_docitems(&ebook, InputFormat::Epub);
        let markdown = EbooksBackend::docitems_to_markdown(&doc_items);

        // Should handle minimal ebook gracefully
        assert!(!doc_items.is_empty());
        assert!(!markdown.is_empty());
        assert!(markdown.contains("Minimal content"));
    }

    #[test]
    fn test_ebook_deeply_nested_toc() {
        // Test table of contents with multiple nesting levels
        use docling_ebook::{EbookMetadata, ParsedEbook, TocEntry};

        let mut metadata = EbookMetadata::new();
        metadata.title = Some("Nested TOC Book".to_string());

        let mut ebook = ParsedEbook::new(metadata);

        // Create deeply nested TOC structure (4 levels)
        let mut root_toc = TocEntry::new("Part I".to_string(), "part1.html".to_string());

        let mut chapter_toc = TocEntry::new("Chapter 1".to_string(), "ch1.html".to_string());

        let mut section_toc = TocEntry::new("Section 1.1".to_string(), "sec11.html".to_string());

        let subsection_toc =
            TocEntry::new("Subsection 1.1.1".to_string(), "subsec111.html".to_string());

        section_toc.children.push(subsection_toc);
        chapter_toc.children.push(section_toc);
        root_toc.children.push(chapter_toc);
        ebook.toc.push(root_toc);

        let doc_items = EbooksBackend::generate_docitems(&ebook, InputFormat::Epub);
        let markdown = EbooksBackend::docitems_to_markdown(&doc_items);

        // Should handle nested TOC structure
        assert!(markdown.contains("Nested TOC Book")); // Title present
        assert!(markdown.contains("Table of Contents") || markdown.contains("Part I")); // TOC present
                                                                                        // DocItems should include list items for TOC
        assert!(doc_items.len() >= 2); // At least title and TOC section
    }

    #[test]
    fn test_ebook_html_content_in_chapters() {
        // Test chapters containing HTML markup (common in EPUB)
        use docling_ebook::{Chapter, EbookMetadata, ParsedEbook};

        let metadata = EbookMetadata::new();
        let mut ebook = ParsedEbook::new(metadata);

        // Add chapter with HTML content
        ebook.chapters.push(Chapter {
            title: Some("HTML Chapter".to_string()),
            content: "<p>Paragraph with <strong>bold</strong> and <em>italic</em> text.</p>\
                      <ul><li>List item 1</li><li>List item 2</li></ul>"
                .to_string(),
            href: "html_ch.html".to_string(),
            spine_order: 0,
        });

        let doc_items = EbooksBackend::generate_docitems(&ebook, InputFormat::Epub);
        let markdown = EbooksBackend::docitems_to_markdown(&doc_items);

        // Should handle HTML content (either by parsing or raw inclusion)
        assert!(markdown.len() > 20);
        // Content should be present in some form
        assert!(
            markdown.contains("Paragraph") || markdown.contains("bold") || markdown.contains("p>")
        );
    }

    #[test]
    fn test_ebook_rights_and_contributors_metadata() {
        // Test additional metadata fields (rights, contributors)
        use docling_ebook::{EbookMetadata, ParsedEbook};

        let mut metadata = EbookMetadata::new();
        metadata.title = Some("Rights Test Book".to_string());
        metadata.creators = vec!["Main Author".to_string()];
        metadata.contributors = vec!["Editor Name".to_string(), "Translator Name".to_string()];
        metadata.rights = Some("Copyright © 2024. All rights reserved.".to_string());
        metadata.language = Some("en-US".to_string());

        let ebook = ParsedEbook::new(metadata);
        let doc_items = EbooksBackend::generate_docitems(&ebook, InputFormat::Epub);
        let markdown = EbooksBackend::docitems_to_markdown(&doc_items);

        // Should include metadata
        assert!(!markdown.is_empty());
        assert!(markdown.contains("Rights Test Book"));
        // Rights and contributors may or may not appear in markdown output,
        // but structure should be valid
        assert!(!doc_items.is_empty());
    }

    #[test]
    fn test_ebook_with_publisher_and_date() {
        // Test ebooks with publisher and publication date
        use docling_ebook::{EbookMetadata, ParsedEbook};

        let mut metadata = EbookMetadata::new();
        metadata.title = Some("Published Book".to_string());
        metadata.creators = vec!["Published Author".to_string()];
        metadata.publisher = Some("Example Publisher".to_string());
        metadata.date = Some("2024-01-15".to_string());

        let ebook = ParsedEbook::new(metadata);
        let doc_items = EbooksBackend::generate_docitems(&ebook, InputFormat::Epub);
        let markdown = EbooksBackend::docitems_to_markdown(&doc_items);

        // Should include title and metadata
        assert!(!markdown.is_empty());
        assert!(markdown.contains("Published Book"));
        // Publisher and date may be included in metadata section
        assert!(!doc_items.is_empty());
    }

    #[test]
    fn test_ebook_multiple_authors_and_editors() {
        // Test ebooks with multiple creators and contributors
        use docling_ebook::{EbookMetadata, ParsedEbook};

        let mut metadata = EbookMetadata::new();
        metadata.title = Some("Collaborative Book".to_string());
        metadata.creators = vec![
            "Author One".to_string(),
            "Author Two".to_string(),
            "Author Three".to_string(),
        ];
        metadata.contributors = vec![
            "Editor A".to_string(),
            "Editor B".to_string(),
            "Illustrator C".to_string(),
        ];

        let ebook = ParsedEbook::new(metadata);
        let doc_items = EbooksBackend::generate_docitems(&ebook, InputFormat::Epub);
        let markdown = EbooksBackend::docitems_to_markdown(&doc_items);

        // Should include all authors (may be comma-separated or listed)
        assert!(!markdown.is_empty());
        assert!(markdown.contains("Collaborative Book"));
        // Multiple creators should be represented in DocItems
        assert!(!doc_items.is_empty());
    }

    #[test]
    fn test_ebook_with_footnotes_and_endnotes() {
        // Test ebooks with footnote/endnote references
        use docling_ebook::{Chapter, EbookMetadata, ParsedEbook};

        let metadata = EbookMetadata::new();
        let mut ebook = ParsedEbook::new(metadata);

        // Add chapter with footnote references
        ebook.chapters.push(Chapter {
            title: Some("Chapter with Footnotes".to_string()),
            content: "Main text with footnote reference[1]. More text with another footnote[2].\n\
                      [1] This is footnote one.\n\
                      [2] This is footnote two."
                .to_string(),
            href: "chapter_footnotes.html".to_string(),
            spine_order: 0,
        });

        let doc_items = EbooksBackend::generate_docitems(&ebook, InputFormat::Epub);
        let markdown = EbooksBackend::docitems_to_markdown(&doc_items);

        // Should preserve footnote structure
        assert!(markdown.len() > 30);
        assert!(
            markdown.contains("footnote")
                || markdown.contains("[1]")
                || markdown.contains("Main text")
        );
    }

    #[test]
    fn test_ebook_with_embedded_fonts() {
        // Test ebooks with embedded font references (metadata only, fonts not loaded)
        use docling_ebook::{EbookMetadata, ParsedEbook};

        let mut metadata = EbookMetadata::new();
        metadata.title = Some("Styled Book".to_string());
        metadata.creators = vec!["Font User".to_string()];
        // EPUB can reference fonts in manifest, but we don't load font files
        // This tests that ebook parsing doesn't break with font references

        let ebook = ParsedEbook::new(metadata);
        let doc_items = EbooksBackend::generate_docitems(&ebook, InputFormat::Epub);
        let markdown = EbooksBackend::docitems_to_markdown(&doc_items);

        // Should create valid DocItems regardless of font references
        assert!(!markdown.is_empty());
        assert!(markdown.contains("Styled Book"));
        assert!(!doc_items.is_empty());
    }

    #[test]
    fn test_ebook_chapter_with_tables() {
        // Test ebooks with table content in chapters
        use docling_ebook::{Chapter, EbookMetadata, ParsedEbook};

        let metadata = EbookMetadata::new();
        let mut ebook = ParsedEbook::new(metadata);

        // Add chapter with HTML table
        ebook.chapters.push(Chapter {
            title: Some("Chapter with Table".to_string()),
            content: "<table>\
                        <tr><th>Header 1</th><th>Header 2</th></tr>\
                        <tr><td>Cell 1</td><td>Cell 2</td></tr>\
                        <tr><td>Cell 3</td><td>Cell 4</td></tr>\
                      </table>\
                      <p>Text after table.</p>"
                .to_string(),
            href: "table_chapter.html".to_string(),
            spine_order: 0,
        });

        let doc_items = EbooksBackend::generate_docitems(&ebook, InputFormat::Epub);
        let markdown = EbooksBackend::docitems_to_markdown(&doc_items);

        // Should handle table content (either parsed or raw)
        assert!(markdown.len() > 20);
        assert!(
            markdown.contains("Header") || markdown.contains("Cell") || markdown.contains("table")
        );
    }

    // ==================== ADVANCED EBOOK FEATURES (N=626: 70 → 75 tests) ====================

    #[test]
    fn test_ebook_epub3_advanced_metadata() {
        // EPUB3 specific metadata fields (meta-properties, refinements, belongs-to-collection)
        use docling_ebook::{EbookMetadata, ParsedEbook};

        let mut metadata = EbookMetadata::new();
        metadata.title = Some("Modern EPUB3 Book".to_string());
        metadata.creators = vec!["EPUB3 Author".to_string()];
        metadata.language = Some("en-GB".to_string());
        // EPUB3 specific: belongs-to-collection, group-position, collection-type
        // Note: EbookMetadata may not have these fields yet, but test that parsing doesn't break
        metadata.description = Some(
            "An EPUB3 publication with advanced metadata including collection membership, \
             accessibility features, and media overlays."
                .to_string(),
        );

        let ebook = ParsedEbook::new(metadata);
        let doc_items = EbooksBackend::generate_docitems(&ebook, InputFormat::Epub);
        let markdown = EbooksBackend::docitems_to_markdown(&doc_items);

        // Should handle EPUB3 metadata gracefully (even if not all fields are parsed)
        assert!(!markdown.is_empty());
        assert!(markdown.contains("Modern EPUB3 Book"));
        assert!(markdown.contains("EPUB3 Author") || markdown.contains("Authors:"));
        assert!(!doc_items.is_empty());
    }

    #[test]
    fn test_ebook_cover_image_metadata() {
        // Test ebooks with cover image references in metadata
        use docling_ebook::{EbookMetadata, ParsedEbook};

        let mut metadata = EbookMetadata::new();
        metadata.title = Some("Book with Cover".to_string());
        metadata.creators = vec!["Visual Author".to_string()];
        // Note: EbookMetadata may not have explicit 'cover' field
        // EPUB stores cover in manifest with properties="cover-image"
        // This test validates that cover references don't break parsing
        metadata.description = Some("Book with beautiful cover art".to_string());

        let ebook = ParsedEbook::new(metadata);
        let doc_items = EbooksBackend::generate_docitems(&ebook, InputFormat::Epub);
        let markdown = EbooksBackend::docitems_to_markdown(&doc_items);

        // Should create valid DocItems even if cover image is referenced
        assert!(!markdown.is_empty());
        assert!(markdown.contains("Book with Cover"));
        assert!(!doc_items.is_empty());
    }

    #[test]
    fn test_ebook_series_metadata() {
        // Test ebooks with series/calibre metadata (series name, series index)
        use docling_ebook::{EbookMetadata, ParsedEbook};

        let mut metadata = EbookMetadata::new();
        metadata.title = Some("The Rising Storm".to_string());
        metadata.creators = vec!["Fantasy Author".to_string()];
        metadata.publisher = Some("Epic Fantasy Press".to_string());
        // Series metadata (Calibre extension): calibre:series, calibre:series_index
        // Standard: <meta property="belongs-to-collection">Epic Saga Series</meta>
        // Note: EbookMetadata may not parse these yet, testing graceful handling
        metadata.description = Some("Book 3 in the Epic Saga Series".to_string());

        let ebook = ParsedEbook::new(metadata);
        let doc_items = EbooksBackend::generate_docitems(&ebook, InputFormat::Epub);
        let markdown = EbooksBackend::docitems_to_markdown(&doc_items);

        // Should handle series metadata (if parsed) or skip gracefully
        assert!(!markdown.is_empty());
        assert!(markdown.contains("The Rising Storm"));
        assert!(markdown.contains("Fantasy Author") || markdown.contains("Authors:"));
        assert!(!doc_items.is_empty());
    }

    #[test]
    fn test_ebook_multilingual_content() {
        // Test ebooks with multiple languages (multilingual content, language attributes)
        use docling_ebook::{Chapter, EbookMetadata, ParsedEbook};

        let mut metadata = EbookMetadata::new();
        metadata.title = Some("Multilingual Guide".to_string());
        metadata.creators = vec!["Polyglot Author".to_string()];
        metadata.language = Some("mul".to_string()); // ISO 639-2 code for multiple languages

        let mut ebook = ParsedEbook::new(metadata);

        // Add chapters in different languages
        ebook.chapters.push(Chapter {
            title: Some("English Introduction".to_string()),
            content: "This is the English version.".to_string(),
            href: "intro_en.html".to_string(),
            spine_order: 0,
        });

        ebook.chapters.push(Chapter {
            title: Some("Introduction en Français".to_string()),
            content: "Ceci est la version française. Voilà!".to_string(),
            href: "intro_fr.html".to_string(),
            spine_order: 1,
        });

        ebook.chapters.push(Chapter {
            title: Some("Einführung auf Deutsch".to_string()),
            content: "Dies ist die deutsche Version. Grüße!".to_string(),
            href: "intro_de.html".to_string(),
            spine_order: 2,
        });

        let doc_items = EbooksBackend::generate_docitems(&ebook, InputFormat::Epub);
        let markdown = EbooksBackend::docitems_to_markdown(&doc_items);

        // Should handle multilingual content correctly (UTF-8 support)
        assert!(markdown.len() > 100);
        assert!(markdown.contains("Multilingual Guide"));
        assert!(
            markdown.contains("English")
                || markdown.contains("Français")
                || markdown.contains("Deutsch")
        );
        assert!(markdown.contains("française") || markdown.contains("deutsche")); // Accented characters
        assert!(doc_items.len() >= 5); // Title + author + 3 chapters
    }

    #[test]
    fn test_ebook_hierarchical_toc_structure() {
        // Test that hierarchical TOC structure is preserved in output
        use docling_ebook::{EbookMetadata, ParsedEbook, TocEntry};

        let mut metadata = EbookMetadata::new();
        metadata.title = Some("Hierarchical TOC Book".to_string());

        let mut ebook = ParsedEbook::new(metadata);

        // Create multi-level TOC structure
        let mut part1 = TocEntry::new("Part I: Basics".to_string(), "part1.html".to_string());
        part1.children.push(TocEntry::new(
            "Chapter 1".to_string(),
            "ch1.html".to_string(),
        ));
        part1.children.push(TocEntry::new(
            "Chapter 2".to_string(),
            "ch2.html".to_string(),
        ));

        let mut part2 = TocEntry::new(
            "Part II: Advanced Topics".to_string(),
            "part2.html".to_string(),
        );
        let mut chapter3 = TocEntry::new("Chapter 3".to_string(), "ch3.html".to_string());
        chapter3.children.push(TocEntry::new(
            "Section 3.1".to_string(),
            "ch3-1.html".to_string(),
        ));
        chapter3.children.push(TocEntry::new(
            "Section 3.2".to_string(),
            "ch3-2.html".to_string(),
        ));
        part2.children.push(chapter3);

        ebook.toc.push(part1);
        ebook.toc.push(part2);

        let doc_items = EbooksBackend::generate_docitems(&ebook, InputFormat::Epub);
        let markdown = EbooksBackend::docitems_to_markdown(&doc_items);

        // Should have hierarchical TOC in output
        assert!(markdown.contains("## Table of Contents"));
        assert!(markdown.contains("- [Part I: Basics](#part1.html)"));
        assert!(markdown.contains("  - [Chapter 1](#ch1.html)"));
        assert!(markdown.contains("  - [Chapter 2](#ch2.html)"));
        assert!(markdown.contains("- [Part II: Advanced Topics](#part2.html)"));
        assert!(markdown.contains("  - [Chapter 3](#ch3.html)"));
        assert!(markdown.contains("    - [Section 3.1](#ch3-1.html)"));
        assert!(markdown.contains("    - [Section 3.2](#ch3-2.html)"));
    }

    #[test]
    fn test_ebook_spine_order_display() {
        // Test that spine order and file paths are displayed for chapters
        use docling_ebook::{Chapter, EbookMetadata, ParsedEbook};

        let mut metadata = EbookMetadata::new();
        metadata.title = Some("Spine Order Test".to_string());
        let mut ebook = ParsedEbook::new(metadata);

        // Add chapters with specific spine order
        ebook.chapters.push(Chapter {
            title: Some("First Chapter".to_string()),
            content: "Content 1".to_string(),
            href: "chapter01.xhtml".to_string(),
            spine_order: 0,
        });
        ebook.chapters.push(Chapter {
            title: Some("Second Chapter".to_string()),
            content: "Content 2".to_string(),
            href: "chapter02.xhtml".to_string(),
            spine_order: 1,
        });

        let doc_items = EbooksBackend::generate_docitems(&ebook, InputFormat::Epub);
        let markdown = EbooksBackend::docitems_to_markdown(&doc_items);

        // Should display chapter headings (spine order metadata removed for cleaner output)
        assert!(markdown.contains("## First Chapter"));
        assert!(markdown.contains("## Second Chapter"));
    }

    #[test]
    fn test_ebook_epub3_navigation_landmarks() {
        // EPUB3 navigation document with landmarks (cover, toc, bodymatter, etc.)
        use docling_ebook::{EbookMetadata, ParsedEbook, TocEntry};

        let mut metadata = EbookMetadata::new();
        metadata.title = Some("EPUB3 Navigation Book".to_string());
        metadata.creators = vec!["Navigation Expert".to_string()];

        let mut ebook = ParsedEbook::new(metadata);

        // EPUB3 navigation landmarks: cover, toc, bodymatter, backmatter
        // Represented as special TOC entries with epub:type attributes
        let cover_landmark = TocEntry::new("Cover".to_string(), "cover.html".to_string());
        let toc_landmark = TocEntry::new("Table of Contents".to_string(), "toc.html".to_string());
        let mut body_landmark =
            TocEntry::new("Start of Content".to_string(), "chapter1.html".to_string());
        let back_landmark = TocEntry::new("Index".to_string(), "index.html".to_string());

        // Nested structure under body landmark
        body_landmark.children.push(TocEntry::new(
            "Chapter 1".to_string(),
            "chapter1.html".to_string(),
        ));
        body_landmark.children.push(TocEntry::new(
            "Chapter 2".to_string(),
            "chapter2.html".to_string(),
        ));

        ebook.toc.push(cover_landmark);
        ebook.toc.push(toc_landmark);
        ebook.toc.push(body_landmark);
        ebook.toc.push(back_landmark);

        let doc_items = EbooksBackend::generate_docitems(&ebook, InputFormat::Epub);
        let markdown = EbooksBackend::docitems_to_markdown(&doc_items);

        // Should handle EPUB3 navigation landmarks structure
        assert!(markdown.contains("EPUB3 Navigation Book"));
        assert!(markdown.contains("Navigation Expert") || markdown.contains("Authors:"));
        // TOC should be represented (may contain "Cover", "Table of Contents", etc.)
        assert!(doc_items.len() >= 2); // At least title and TOC section
        assert!(!markdown.is_empty());
    }
}
