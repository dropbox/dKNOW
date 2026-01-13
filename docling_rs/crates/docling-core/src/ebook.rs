/// E-book format backend for docling-core
///
/// Processes EPUB, FB2, MOBI, and other e-book formats into markdown documents
use std::fmt::Write;
use std::path::Path;

use docling_ebook::{html_to_text, parse_epub, parse_fb2, parse_mobi, ParsedEbook};

use crate::error::{DoclingError, Result};

/// Process an EPUB file into markdown
///
/// # Errors
/// Returns an error if the file cannot be parsed as valid EPUB.
#[must_use = "this function returns the extracted markdown content"]
pub fn process_epub<P: AsRef<Path>>(path: P) -> Result<String> {
    // Parse EPUB using docling-ebook
    let parsed = parse_epub(path.as_ref())
        .map_err(|e| DoclingError::ConversionError(format!("Failed to parse EPUB: {e}")))?;

    // Convert to markdown
    Ok(epub_to_markdown(&parsed))
}

/// Process a FB2 file into markdown
///
/// # Errors
/// Returns an error if the file cannot be parsed as valid FB2.
#[must_use = "this function returns the extracted markdown content"]
pub fn process_fb2<P: AsRef<Path>>(path: P) -> Result<String> {
    // Parse FB2 using docling-ebook
    let parsed = parse_fb2(path.as_ref())
        .map_err(|e| DoclingError::ConversionError(format!("Failed to parse FB2: {e}")))?;

    // Convert to markdown (reuse epub_to_markdown, works for all e-books)
    Ok(epub_to_markdown(&parsed))
}

/// Process a MOBI file into markdown
///
/// # Errors
/// Returns an error if the file cannot be read or parsed as valid MOBI.
#[must_use = "this function returns the extracted markdown content"]
pub fn process_mobi<P: AsRef<Path>>(path: P) -> Result<String> {
    // Read MOBI file bytes
    let bytes = std::fs::read(path.as_ref())?;

    // Parse MOBI using docling-ebook
    let parsed = parse_mobi(&bytes)
        .map_err(|e| DoclingError::ConversionError(format!("Failed to parse MOBI: {e}")))?;

    // Convert to markdown (reuse epub_to_markdown, works for all e-books)
    Ok(epub_to_markdown(&parsed))
}

/// Convert `ParsedEbook` to markdown format
fn epub_to_markdown(ebook: &ParsedEbook) -> String {
    let mut markdown = String::new();

    // Title
    if let Some(title) = &ebook.metadata.title {
        let _ = writeln!(markdown, "# {title}\n");
    } else {
        markdown.push_str("# Untitled Book\n\n");
    }

    // Authors
    if !ebook.metadata.creators.is_empty() {
        let _ = writeln!(
            markdown,
            "**Authors:** {}\n",
            ebook.metadata.creators.join(", ")
        );
    }

    // Metadata section - track if we write any metadata by checking length
    let metadata_start_len = markdown.len();

    if let Some(publisher) = &ebook.metadata.publisher {
        let _ = writeln!(markdown, "**Publisher:** {publisher}");
    }

    if let Some(date) = &ebook.metadata.date {
        let _ = writeln!(markdown, "**Date:** {date}");
    }

    if let Some(language) = &ebook.metadata.language {
        let _ = writeln!(markdown, "**Language:** {language}");
    }

    if !ebook.metadata.subjects.is_empty() {
        let _ = writeln!(
            markdown,
            "**Subjects:** {}",
            ebook.metadata.subjects.join(", ")
        );
    }

    if markdown.len() > metadata_start_len {
        markdown.push('\n');
    }

    // Description
    if let Some(description) = &ebook.metadata.description {
        markdown.push_str("## Description\n\n");
        markdown.push_str(description);
        markdown.push_str("\n\n");
    }

    // Table of Contents
    if !ebook.toc.is_empty() {
        markdown.push_str("## Table of Contents\n\n");
        for (i, entry) in ebook.toc.iter().enumerate() {
            let _ = writeln!(markdown, "{}. {}", i + 1, entry.label);
        }
        markdown.push_str("\n---\n\n");
    }

    // Chapters
    for chapter in &ebook.chapters {
        // Chapter title
        if let Some(title) = &chapter.title {
            let _ = writeln!(markdown, "## {title}\n");
        } else {
            let _ = writeln!(markdown, "## Chapter {}\n", chapter.spine_order + 1);
        }

        // Convert HTML content to plain text
        let text = html_to_text(&chapter.content);

        // Clean up excessive newlines
        let cleaned = text
            .lines()
            .map(str::trim_end)
            .collect::<Vec<_>>()
            .join("\n");

        markdown.push_str(&cleaned);
        markdown.push_str("\n\n---\n\n");
    }

    // Appendix with additional metadata
    markdown.push_str("## Appendix\n\n");

    if let Some(identifier) = &ebook.metadata.identifier {
        let _ = writeln!(markdown, "**Identifier:** {identifier}");
    }

    if let Some(rights) = &ebook.metadata.rights {
        let _ = writeln!(markdown, "**Rights:** {rights}");
    }

    if !ebook.metadata.contributors.is_empty() {
        let _ = writeln!(
            markdown,
            "**Contributors:** {}",
            ebook.metadata.contributors.join(", ")
        );
    }

    markdown
}

#[cfg(test)]
mod tests {
    use super::*;
    use docling_ebook::types::{Chapter, EbookMetadata, ParsedEbook, TocEntry};

    #[test]
    fn test_epub_to_markdown_basic() {
        let mut metadata = EbookMetadata::new();
        metadata.title = Some("Test Book".to_string());
        metadata.creators = vec!["Test Author".to_string()];

        let mut ebook = ParsedEbook::new(metadata);

        let chapter = Chapter {
            title: Some("Chapter One".to_string()),
            content: "<p>Test content</p>".to_string(),
            href: "ch1.xhtml".to_string(),
            spine_order: 0,
        };

        ebook.chapters.push(chapter);

        let markdown = epub_to_markdown(&ebook);

        assert!(markdown.contains("# Test Book"));
        assert!(markdown.contains("**Authors:** Test Author"));
        assert!(markdown.contains("## Chapter One"));
        assert!(markdown.contains("Test content"));
    }

    #[test]
    fn test_epub_to_markdown_with_toc() {
        let mut metadata = EbookMetadata::new();
        metadata.title = Some("Book with TOC".to_string());

        let mut ebook = ParsedEbook::new(metadata);

        ebook.toc.push(TocEntry::new(
            "Introduction".to_string(),
            "intro.xhtml".to_string(),
        ));

        ebook.toc.push(TocEntry::new(
            "Chapter 1".to_string(),
            "ch1.xhtml".to_string(),
        ));

        let markdown = epub_to_markdown(&ebook);

        assert!(markdown.contains("## Table of Contents"));
        assert!(markdown.contains("1. Introduction"));
        assert!(markdown.contains("2. Chapter 1"));
    }

    #[test]
    fn test_epub_to_markdown_with_metadata() {
        let mut metadata = EbookMetadata::new();
        metadata.title = Some("Detailed Book".to_string());
        metadata.creators = vec!["Author Name".to_string()];
        metadata.publisher = Some("Test Publisher".to_string());
        metadata.date = Some("2023-01-01".to_string());
        metadata.language = Some("en".to_string());
        metadata.description = Some("A test book description".to_string());
        metadata.identifier = Some("ISBN 123-456-789".to_string());

        let ebook = ParsedEbook::new(metadata);

        let markdown = epub_to_markdown(&ebook);

        assert!(markdown.contains("# Detailed Book"));
        assert!(markdown.contains("**Authors:** Author Name"));
        assert!(markdown.contains("**Publisher:** Test Publisher"));
        assert!(markdown.contains("**Date:** 2023-01-01"));
        assert!(markdown.contains("**Language:** en"));
        assert!(markdown.contains("## Description"));
        assert!(markdown.contains("A test book description"));
        assert!(markdown.contains("**Identifier:** ISBN 123-456-789"));
    }

    #[test]
    fn test_epub_to_markdown_untitled_book() {
        // Book without title should use "Untitled Book"
        let metadata = EbookMetadata::new();
        let ebook = ParsedEbook::new(metadata);

        let markdown = epub_to_markdown(&ebook);

        assert!(markdown.contains("# Untitled Book"));
        assert!(markdown.contains("## Appendix"));
    }

    #[test]
    fn test_epub_to_markdown_no_chapters() {
        // Book without chapters should still generate valid markdown
        let mut metadata = EbookMetadata::new();
        metadata.title = Some("Empty Book".to_string());

        let ebook = ParsedEbook::new(metadata);

        let markdown = epub_to_markdown(&ebook);

        assert!(markdown.contains("# Empty Book"));
        assert!(markdown.contains("## Appendix"));
        assert!(!markdown.contains("## Chapter"));
    }

    #[test]
    fn test_epub_to_markdown_multiple_authors() {
        // Book with multiple authors
        let mut metadata = EbookMetadata::new();
        metadata.title = Some("Collaborative Book".to_string());
        metadata.creators = vec![
            "First Author".to_string(),
            "Second Author".to_string(),
            "Third Author".to_string(),
        ];

        let ebook = ParsedEbook::new(metadata);

        let markdown = epub_to_markdown(&ebook);

        assert!(markdown.contains("**Authors:** First Author, Second Author, Third Author"));
    }

    #[test]
    fn test_epub_to_markdown_with_subjects() {
        // Book with subject tags
        let mut metadata = EbookMetadata::new();
        metadata.title = Some("Tagged Book".to_string());
        metadata.subjects = vec![
            "Fiction".to_string(),
            "Science Fiction".to_string(),
            "Adventure".to_string(),
        ];

        let ebook = ParsedEbook::new(metadata);

        let markdown = epub_to_markdown(&ebook);

        assert!(markdown.contains("**Subjects:** Fiction, Science Fiction, Adventure"));
    }

    #[test]
    fn test_epub_to_markdown_with_contributors() {
        // Book with contributors in appendix
        let mut metadata = EbookMetadata::new();
        metadata.title = Some("Book with Contributors".to_string());
        metadata.contributors = vec!["Editor One".to_string(), "Translator Two".to_string()];

        let ebook = ParsedEbook::new(metadata);

        let markdown = epub_to_markdown(&ebook);

        assert!(markdown.contains("**Contributors:** Editor One, Translator Two"));
    }

    #[test]
    fn test_epub_to_markdown_with_rights() {
        // Book with rights information
        let mut metadata = EbookMetadata::new();
        metadata.title = Some("Rights Book".to_string());
        metadata.rights = Some("Copyright 2023 Test Publisher".to_string());

        let ebook = ParsedEbook::new(metadata);

        let markdown = epub_to_markdown(&ebook);

        assert!(markdown.contains("**Rights:** Copyright 2023 Test Publisher"));
    }

    #[test]
    fn test_epub_to_markdown_chapter_without_title() {
        // Chapter without title should use default numbering
        let mut metadata = EbookMetadata::new();
        metadata.title = Some("Test Book".to_string());

        let mut ebook = ParsedEbook::new(metadata);

        let chapter = Chapter {
            title: None,
            content: "<p>Chapter content</p>".to_string(),
            href: "ch1.xhtml".to_string(),
            spine_order: 0,
        };

        ebook.chapters.push(chapter);

        let markdown = epub_to_markdown(&ebook);

        assert!(markdown.contains("## Chapter 1"));
        assert!(markdown.contains("Chapter content"));
    }

    #[test]
    fn test_epub_to_markdown_multiple_chapters() {
        // Book with multiple chapters
        let mut metadata = EbookMetadata::new();
        metadata.title = Some("Multi-Chapter Book".to_string());

        let mut ebook = ParsedEbook::new(metadata);

        for i in 0..3 {
            let chapter = Chapter {
                title: Some(format!("Chapter {}", i + 1)),
                content: format!("<p>Content for chapter {}</p>", i + 1),
                href: format!("ch{}.xhtml", i + 1),
                spine_order: i,
            };
            ebook.chapters.push(chapter);
        }

        let markdown = epub_to_markdown(&ebook);

        assert!(markdown.contains("## Chapter 1"));
        assert!(markdown.contains("## Chapter 2"));
        assert!(markdown.contains("## Chapter 3"));
        assert!(markdown.contains("Content for chapter 1"));
        assert!(markdown.contains("Content for chapter 2"));
        assert!(markdown.contains("Content for chapter 3"));
    }

    #[test]
    fn test_epub_to_markdown_html_cleaning() {
        // Test HTML to text conversion and whitespace cleaning
        let mut metadata = EbookMetadata::new();
        metadata.title = Some("HTML Test Book".to_string());

        let mut ebook = ParsedEbook::new(metadata);

        let chapter = Chapter {
            title: Some("HTML Chapter".to_string()),
            content: "<p>First paragraph.</p>\n<p>Second paragraph.</p>".to_string(),
            href: "ch1.xhtml".to_string(),
            spine_order: 0,
        };

        ebook.chapters.push(chapter);

        let markdown = epub_to_markdown(&ebook);

        assert!(markdown.contains("First paragraph"));
        assert!(markdown.contains("Second paragraph"));
    }

    #[test]
    fn test_epub_to_markdown_complete_metadata() {
        // Test all metadata fields together
        let mut metadata = EbookMetadata::new();
        metadata.title = Some("Complete Book".to_string());
        metadata.creators = vec!["Author One".to_string(), "Author Two".to_string()];
        metadata.contributors = vec!["Editor".to_string()];
        metadata.publisher = Some("Test Publisher".to_string());
        metadata.date = Some("2023-12-01".to_string());
        metadata.language = Some("en-US".to_string());
        metadata.subjects = vec!["Technology".to_string(), "Programming".to_string()];
        metadata.description = Some("A complete test book".to_string());
        metadata.identifier = Some("ISBN 978-0-123456-78-9".to_string());
        metadata.rights = Some("All rights reserved".to_string());

        let mut ebook = ParsedEbook::new(metadata);

        let chapter = Chapter {
            title: Some("Test Chapter".to_string()),
            content: "<p>Test content</p>".to_string(),
            href: "test.xhtml".to_string(),
            spine_order: 0,
        };
        ebook.chapters.push(chapter);

        let markdown = epub_to_markdown(&ebook);

        // Check all fields are present
        assert!(markdown.contains("# Complete Book"));
        assert!(markdown.contains("**Authors:** Author One, Author Two"));
        assert!(markdown.contains("**Publisher:** Test Publisher"));
        assert!(markdown.contains("**Date:** 2023-12-01"));
        assert!(markdown.contains("**Language:** en-US"));
        assert!(markdown.contains("**Subjects:** Technology, Programming"));
        assert!(markdown.contains("## Description"));
        assert!(markdown.contains("A complete test book"));
        assert!(markdown.contains("## Test Chapter"));
        assert!(markdown.contains("Test content"));
        assert!(markdown.contains("**Identifier:** ISBN 978-0-123456-78-9"));
        assert!(markdown.contains("**Rights:** All rights reserved"));
        assert!(markdown.contains("**Contributors:** Editor"));
    }

    #[test]
    fn test_process_epub_nonexistent_file() {
        // Test error handling for missing file
        let result = process_epub("/nonexistent/path/to/book.epub");
        assert!(result.is_err());
    }

    #[test]
    fn test_process_fb2_nonexistent_file() {
        // Test error handling for missing FB2 file
        let result = process_fb2("/nonexistent/path/to/book.fb2");
        assert!(result.is_err());
    }

    #[test]
    fn test_process_mobi_nonexistent_file() {
        // Test error handling for missing MOBI file
        let result = process_mobi("/nonexistent/path/to/book.mobi");
        assert!(result.is_err());
    }
}
