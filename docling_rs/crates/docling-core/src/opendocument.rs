/// `OpenDocument` Format backend for docling-core
///
/// Processes ODT, ODS, and ODP files into markdown documents
use std::fmt::Write;
use std::path::Path;

use docling_opendocument::{
    parse_odp_file, parse_ods_file, parse_odt_file, OdpDocument, OdsDocument, OdtDocument,
};

use crate::error::{DoclingError, Result};

/// Process an ODT (`OpenDocument` Text) file into markdown
///
/// # Errors
/// Returns an error if the file cannot be parsed as valid ODT.
#[must_use = "this function returns the extracted markdown content"]
pub fn process_odt<P: AsRef<Path>>(path: P) -> Result<String> {
    // Parse ODT using docling-opendocument
    let parsed = parse_odt_file(path.as_ref())
        .map_err(|e| DoclingError::ConversionError(format!("Failed to parse ODT: {e}")))?;

    // Convert to markdown
    Ok(odt_to_markdown(&parsed))
}

/// Process an ODS (`OpenDocument` Spreadsheet) file into markdown
///
/// # Errors
/// Returns an error if the file cannot be parsed as valid ODS.
#[must_use = "this function returns the extracted markdown content"]
pub fn process_ods<P: AsRef<Path>>(path: P) -> Result<String> {
    // Parse ODS using docling-opendocument
    let parsed = parse_ods_file(path.as_ref())
        .map_err(|e| DoclingError::ConversionError(format!("Failed to parse ODS: {e}")))?;

    // Convert to markdown
    Ok(ods_to_markdown(&parsed))
}

/// Process an ODP (`OpenDocument` Presentation) file into markdown
///
/// # Errors
/// Returns an error if the file cannot be parsed as valid ODP.
#[must_use = "this function returns the extracted markdown content"]
pub fn process_odp<P: AsRef<Path>>(path: P) -> Result<String> {
    // Parse ODP using docling-opendocument
    let parsed = parse_odp_file(path.as_ref())
        .map_err(|e| DoclingError::ConversionError(format!("Failed to parse ODP: {e}")))?;

    // Convert to markdown
    Ok(odp_to_markdown(&parsed))
}

/// Convert `OdtDocument` to markdown format
fn odt_to_markdown(doc: &OdtDocument) -> String {
    let mut markdown = String::new();

    // Title from metadata
    if let Some(title) = &doc.title {
        let _ = writeln!(markdown, "# {title}\n");
    } else {
        markdown.push_str("# OpenDocument Text\n\n");
    }

    // Metadata section - track if we write any metadata by checking length
    let metadata_start_len = markdown.len();

    if let Some(author) = &doc.author {
        let _ = writeln!(markdown, "**Author:** {author}");
    }

    if markdown.len() > metadata_start_len {
        markdown.push('\n');
    }

    // Main content (already formatted with markdown-style headings and lists)
    markdown.push_str(&doc.text);

    // Ensure single trailing newline
    if !markdown.ends_with('\n') {
        markdown.push('\n');
    }

    markdown
}

/// Convert `OdsDocument` to markdown format
fn ods_to_markdown(doc: &OdsDocument) -> String {
    let mut markdown = String::new();

    // Title
    markdown.push_str("# OpenDocument Spreadsheet\n\n");

    // Sheet count
    if !doc.sheet_names.is_empty() {
        let _ = writeln!(markdown, "**Sheets:** {}\n", doc.sheet_names.len());
    }

    // Main content (already formatted with sheet names and tables)
    markdown.push_str(&doc.text);

    // Ensure single trailing newline
    if !markdown.ends_with('\n') {
        markdown.push('\n');
    }

    markdown
}

/// Convert `OdpDocument` to markdown format
fn odp_to_markdown(doc: &OdpDocument) -> String {
    let mut markdown = String::new();

    // Title from metadata
    if let Some(title) = &doc.title {
        let _ = writeln!(markdown, "# {title}\n");
    } else {
        markdown.push_str("# OpenDocument Presentation\n\n");
    }

    // Metadata section - track if we write any metadata by checking length
    let metadata_start_len = markdown.len();

    if let Some(author) = &doc.author {
        let _ = writeln!(markdown, "**Author:** {author}");
    }

    if doc.slide_count > 0 {
        let _ = writeln!(markdown, "**Slides:** {}", doc.slide_count);
    }

    if markdown.len() > metadata_start_len {
        markdown.push('\n');
    }

    // Main content (already formatted with slide separators and content)
    markdown.push_str(&doc.text);

    // Ensure single trailing newline
    if !markdown.ends_with('\n') {
        markdown.push('\n');
    }

    markdown
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Full integration tests will be in integration_tests.rs
    // These are just basic unit tests for the markdown conversion functions

    #[test]
    fn test_odt_to_markdown_basic() {
        let doc = OdtDocument {
            title: Some("Test Document".to_string()),
            author: Some("Test Author".to_string()),
            paragraph_count: 1,
            table_count: 0,
            text: "# Heading\n\nParagraph text.\n\n".to_string(),
        };

        let markdown = odt_to_markdown(&doc);

        assert!(markdown.contains("# Test Document"));
        assert!(markdown.contains("**Author:** Test Author"));
        // Note: Subject field is not part of OdtDocument structure
        assert!(markdown.contains("# Heading"));
        assert!(markdown.contains("Paragraph text."));
    }

    #[test]
    fn test_odt_to_markdown_no_metadata() {
        let doc = OdtDocument {
            title: None,
            author: None,
            paragraph_count: 1,
            table_count: 0,
            text: "Just some text.\n".to_string(),
        };

        let markdown = odt_to_markdown(&doc);

        assert!(markdown.contains("# OpenDocument Text"));
        assert!(markdown.contains("Just some text."));
        assert!(!markdown.contains("**Author:**"));
    }

    #[test]
    fn test_ods_to_markdown_basic() {
        let doc = OdsDocument {
            sheet_names: vec!["Sheet1".to_string(), "Sheet2".to_string()],
            text: "## Sheet: Sheet1\n\nA | B\n--- | ---\n1 | 2\n\n".to_string(),
            sheet_count: 2,
            cell_count: 2,
            row_count: 1,
        };

        let markdown = ods_to_markdown(&doc);

        assert!(markdown.contains("# OpenDocument Spreadsheet"));
        assert!(markdown.contains("**Sheets:** 2"));
        assert!(markdown.contains("## Sheet: Sheet1"));
    }

    #[test]
    fn test_odp_to_markdown_basic() {
        let doc = OdpDocument {
            title: Some("Test Presentation".to_string()),
            author: Some("Presenter".to_string()),
            slide_count: 3,
            text: "## Slide 1\n\nSlide content\n\n".to_string(),
            slide_titles: vec!["Slide 1".to_string()],
            slide_names: vec!["Slide 1".to_string()],
            slide_metadata: vec![],
        };

        let markdown = odp_to_markdown(&doc);

        assert!(markdown.contains("# Test Presentation"));
        assert!(markdown.contains("**Author:** Presenter"));
        assert!(markdown.contains("**Slides:** 3"));
        assert!(markdown.contains("## Slide 1"));
    }

    #[test]
    fn test_odp_to_markdown_no_metadata() {
        let doc = OdpDocument {
            title: None,
            author: None,
            slide_count: 0,
            text: "## Slide 1\n\nContent\n".to_string(),
            slide_titles: vec![],
            slide_names: vec![],
            slide_metadata: vec![],
        };

        let markdown = odp_to_markdown(&doc);

        assert!(markdown.contains("# OpenDocument Presentation"));
        assert!(markdown.contains("## Slide 1"));
        assert!(!markdown.contains("**Author:**"));
    }

    #[test]
    fn test_odt_to_markdown_trailing_newline() {
        // Test that markdown always ends with exactly one newline
        let doc_with_newline = OdtDocument {
            title: None,
            author: None,
            paragraph_count: 0,
            table_count: 0,
            text: "Text with newline\n".to_string(),
        };

        let doc_without_newline = OdtDocument {
            title: None,
            author: None,
            paragraph_count: 0,
            table_count: 0,
            text: "Text without newline".to_string(),
        };

        let md1 = odt_to_markdown(&doc_with_newline);
        let md2 = odt_to_markdown(&doc_without_newline);

        assert!(md1.ends_with('\n'));
        assert!(md2.ends_with('\n'));
        // Ensure single newline, not multiple
        assert!(!md1.ends_with("\n\n"));
        assert!(!md2.ends_with("\n\n"));
    }

    #[test]
    fn test_ods_to_markdown_empty_sheets() {
        let doc = OdsDocument {
            sheet_names: vec![],
            text: "".to_string(),
            sheet_count: 0,
            cell_count: 0,
            row_count: 0,
        };

        let markdown = ods_to_markdown(&doc);

        assert!(markdown.contains("# OpenDocument Spreadsheet"));
        // Should not contain "**Sheets:**" line when empty
        assert!(!markdown.contains("**Sheets:** 0"));
    }

    #[test]
    fn test_ods_to_markdown_multiple_sheets() {
        let doc = OdsDocument {
            sheet_names: vec![
                "Financial".to_string(),
                "Budget".to_string(),
                "Forecast".to_string(),
            ],
            text: "## Sheet: Financial\n\n## Sheet: Budget\n\n## Sheet: Forecast\n\n".to_string(),
            sheet_count: 3,
            cell_count: 100,
            row_count: 20,
        };

        let markdown = ods_to_markdown(&doc);

        assert!(markdown.contains("**Sheets:** 3"));
        assert!(markdown.contains("## Sheet: Financial"));
        assert!(markdown.contains("## Sheet: Budget"));
        assert!(markdown.contains("## Sheet: Forecast"));
    }

    #[test]
    fn test_odp_to_markdown_multiple_slides() {
        let doc = OdpDocument {
            title: Some("Q4 Review".to_string()),
            author: Some("Team Lead".to_string()),
            slide_count: 5,
            text: "## Slide 1\n\nIntro\n\n## Slide 2\n\nData\n\n".to_string(),
            slide_titles: vec![
                "Introduction".to_string(),
                "Data Analysis".to_string(),
                "Conclusions".to_string(),
                "Q&A".to_string(),
                "Thank You".to_string(),
            ],
            slide_names: vec![
                "Slide 1".to_string(),
                "Slide 2".to_string(),
                "Slide 3".to_string(),
                "Slide 4".to_string(),
                "Slide 5".to_string(),
            ],
            slide_metadata: vec![],
        };

        let markdown = odp_to_markdown(&doc);

        assert!(markdown.contains("# Q4 Review"));
        assert!(markdown.contains("**Author:** Team Lead"));
        assert!(markdown.contains("**Slides:** 5"));
        assert!(markdown.contains("## Slide 1"));
        assert!(markdown.contains("## Slide 2"));
    }

    #[test]
    fn test_odt_to_markdown_with_tables() {
        let doc = OdtDocument {
            title: Some("Report".to_string()),
            author: None,
            paragraph_count: 5,
            table_count: 2,
            text: "Introduction\n\n| Header |\n|--------|\n| Cell   |\n\nConclusion\n".to_string(),
        };

        let markdown = odt_to_markdown(&doc);

        assert!(markdown.contains("# Report"));
        assert!(markdown.contains("Introduction"));
        assert!(markdown.contains("| Header |"));
        assert!(markdown.contains("Conclusion"));
    }

    #[test]
    fn test_odt_to_markdown_special_characters() {
        let doc = OdtDocument {
            title: Some("Test & Document".to_string()),
            author: Some("Author <email@example.com>".to_string()),
            paragraph_count: 1,
            table_count: 0,
            text: "Text with *asterisks* and _underscores_.\n".to_string(),
        };

        let markdown = odt_to_markdown(&doc);

        // Markdown conversion should preserve special characters
        assert!(markdown.contains("Test & Document"));
        assert!(markdown.contains("Author <email@example.com>"));
        assert!(markdown.contains("*asterisks*"));
        assert!(markdown.contains("_underscores_"));
    }

    #[test]
    fn test_ods_to_markdown_trailing_newline() {
        // Test that spreadsheet markdown always ends with exactly one newline
        let doc = OdsDocument {
            sheet_names: vec!["Sheet1".to_string()],
            text: "Data".to_string(), // No trailing newline
            sheet_count: 1,
            cell_count: 10,
            row_count: 2,
        };

        let markdown = ods_to_markdown(&doc);

        assert!(markdown.ends_with('\n'));
        assert!(!markdown.ends_with("\n\n"));
    }

    #[test]
    fn test_odp_to_markdown_zero_slides() {
        // Edge case: presentation with 0 slides
        let doc = OdpDocument {
            title: Some("Empty".to_string()),
            author: None,
            slide_count: 0,
            text: "".to_string(),
            slide_titles: vec![],
            slide_names: vec![],
            slide_metadata: vec![],
        };

        let markdown = odp_to_markdown(&doc);

        assert!(markdown.contains("# Empty"));
        // Should not show "**Slides:** 0" when count is zero
        assert!(!markdown.contains("**Slides:** 0"));
    }

    #[test]
    fn test_odt_to_markdown_metadata_formatting() {
        // Test that metadata is properly formatted with blank line after
        let doc = OdtDocument {
            title: Some("Document".to_string()),
            author: Some("Writer".to_string()),
            paragraph_count: 1,
            table_count: 0,
            text: "Content starts here.\n".to_string(),
        };

        let markdown = odt_to_markdown(&doc);

        // Check that there's a blank line between metadata and content
        assert!(markdown.contains("**Author:** Writer\n\nContent starts here"));
    }
}
