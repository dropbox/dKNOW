//! `OpenDocument` Text (ODT) format parser
//!
//! Parses .odt files (`OpenDocument` Text format used by `LibreOffice` Writer).
//!
//! ## Format Structure
//! ODT files are ZIP archives containing:
//! - `content.xml` - Main document content
//! - `styles.xml` - Document styles
//! - `meta.xml` - Document metadata
//! - `META-INF/manifest.xml` - File manifest

use crate::error::Result;
use crate::xml::extract_file_as_string;
use quick_xml::events::Event;
use quick_xml::Reader;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use zip::ZipArchive;

/// Parsed ODT document content
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct OdtDocument {
    /// Document text content (paragraphs, headings, etc.)
    pub text: String,
    /// Document title (from metadata)
    pub title: Option<String>,
    /// Document author (from metadata)
    pub author: Option<String>,
    /// Number of paragraphs
    pub paragraph_count: usize,
    /// Number of tables
    pub table_count: usize,
}

impl OdtDocument {
    /// Create a new empty ODT document
    #[inline]
    #[must_use = "creates empty ODT document"]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add text content
    #[inline]
    pub fn add_text(&mut self, text: &str) {
        if !self.text.is_empty() && !self.text.ends_with('\n') {
            self.text.push('\n');
        }
        self.text.push_str(text);
    }

    /// Add a newline
    #[inline]
    pub fn add_newline(&mut self) {
        self.text.push('\n');
    }
}

/// Parse ODT file from a path
///
/// # Errors
///
/// Returns an error if the file cannot be opened (I/O error) or if the ODT content
/// is invalid (not a valid ZIP archive, missing content.xml, or malformed XML).
#[must_use = "this function returns a parsed ODT document that should be processed"]
pub fn parse_odt_file<P: AsRef<Path>>(path: P) -> Result<OdtDocument> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    parse_odt_reader(reader)
}

/// Parse ODT from a reader
///
/// # Errors
///
/// Returns an error if the reader content is not a valid ZIP archive, if content.xml
/// is missing, or if the XML content is malformed.
#[must_use = "this function returns a parsed ODT document that should be processed"]
pub fn parse_odt_reader<R: Read + std::io::Seek>(reader: R) -> Result<OdtDocument> {
    let mut archive = ZipArchive::new(reader)?;
    let mut doc = OdtDocument::new();

    // Parse metadata
    if let Ok(meta_xml) = extract_file_as_string(&mut archive, "meta.xml") {
        parse_metadata(&meta_xml, &mut doc)?;
    }

    // Parse main content
    let content_xml = extract_file_as_string(&mut archive, "content.xml")?;
    parse_content(&content_xml, &mut doc)?;

    Ok(doc)
}

/// Parse metadata from meta.xml
fn parse_metadata(xml_content: &str, doc: &mut OdtDocument) -> Result<()> {
    let mut reader = Reader::from_str(xml_content);
    reader.trim_text(true);

    let mut buf = Vec::new();
    let mut in_title = false;
    let mut in_initial_creator = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = e.name();
                match name.local_name().as_ref() {
                    b"title" => in_title = true,
                    b"initial-creator" => in_initial_creator = true,
                    _ => {}
                }
            }
            Ok(Event::Text(e)) if in_title || in_initial_creator => {
                let text = e.unescape()?.into_owned();
                if in_title {
                    doc.title = Some(text);
                } else if in_initial_creator {
                    doc.author = Some(text);
                }
            }
            Ok(Event::End(e)) => {
                let name = e.name();
                match name.local_name().as_ref() {
                    b"title" => in_title = false,
                    b"initial-creator" => in_initial_creator = false,
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(e.into()),
            _ => {}
        }
        buf.clear();
    }

    Ok(())
}

/// Parse document content from content.xml
#[allow(clippy::too_many_lines)] // Complex XML parsing - keeping together for clarity
fn parse_content(xml_content: &str, doc: &mut OdtDocument) -> Result<()> {
    let mut reader = Reader::from_str(xml_content);
    reader.trim_text(true);

    let mut buf = Vec::new();
    let mut in_paragraph = false;
    let mut in_heading = false;
    let mut in_table_cell = false;
    let mut current_text = String::new();
    let mut list_depth = 0;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e) | Event::Empty(e)) => {
                let name = e.name();
                let name_bytes = name.as_ref();

                // Match full qualified names (namespace:localname)
                match name_bytes {
                    // Text namespace elements
                    b"text:p" => {
                        in_paragraph = true;
                        current_text.clear();
                    }
                    b"text:h" => {
                        in_heading = true;
                        current_text.clear();
                    }
                    // Note: text:list-item content handled via nested paragraphs (fall through to default)
                    b"text:list" => {
                        list_depth += 1;
                    }
                    b"text:s" => {
                        // Space
                        current_text.push(' ');
                    }
                    b"text:tab" => {
                        current_text.push('\t');
                    }
                    b"text:line-break" => {
                        current_text.push('\n');
                    }
                    // Table namespace elements
                    b"table:table" => {
                        doc.table_count += 1;
                        if !doc.text.is_empty() {
                            doc.add_newline();
                        }
                    }
                    b"table:table-cell" => {
                        in_table_cell = true;
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(e)) if in_paragraph || in_heading || in_table_cell => {
                let text = e.unescape()?.into_owned();
                current_text.push_str(&text);
            }
            Ok(Event::End(e)) => {
                let name = e.name();
                let name_bytes = name.as_ref();

                match name_bytes {
                    // Text namespace elements
                    b"text:p" => {
                        if in_paragraph {
                            let trimmed = current_text.trim();
                            if !trimmed.is_empty() {
                                // Add list marker if in list
                                if list_depth > 0 {
                                    doc.add_text(&format!(
                                        "{}• {}",
                                        "  ".repeat(list_depth - 1),
                                        trimmed
                                    ));
                                } else {
                                    doc.add_text(trimmed);
                                }
                                doc.paragraph_count += 1;
                            }
                            in_paragraph = false;
                            current_text.clear();
                        }
                    }
                    b"text:h" => {
                        if in_heading {
                            let trimmed = current_text.trim();
                            if !trimmed.is_empty() {
                                doc.add_newline();
                                doc.add_text(&format!("# {trimmed}"));
                                doc.add_newline();
                                doc.paragraph_count += 1;
                            }
                            in_heading = false;
                            current_text.clear();
                        }
                    }
                    // Note: text:list-item end tag needs no action (fall through to default)
                    b"text:list" => {
                        list_depth = list_depth.saturating_sub(1);
                    }
                    // Table namespace elements
                    b"table:table" => {
                        doc.add_newline();
                    }
                    b"table:table-cell" => {
                        if in_table_cell {
                            let trimmed = current_text.trim();
                            if !trimmed.is_empty() {
                                doc.add_text(trimmed);
                                doc.add_text(" | ");
                            }
                            in_table_cell = false;
                            current_text.clear();
                        }
                    }
                    b"table:table-row" => {
                        // End of table row
                        if !doc.text.is_empty() && !doc.text.ends_with('\n') {
                            doc.add_newline();
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(e.into()),
            _ => {}
        }
        buf.clear();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_odt_document_creation() {
        let mut doc = OdtDocument::new();
        assert_eq!(doc.text, "");
        doc.add_text("Hello");
        assert_eq!(doc.text, "Hello");
        doc.add_text("World");
        assert_eq!(doc.text, "Hello\nWorld");
    }

    #[test]
    fn test_parse_metadata() {
        let xml = r#"<?xml version="1.0"?>
        <office:document-meta>
            <office:meta>
                <dc:title>Test Document</dc:title>
                <meta:initial-creator>Test Author</meta:initial-creator>
            </office:meta>
        </office:document-meta>"#;

        let mut doc = OdtDocument::new();
        parse_metadata(xml, &mut doc).unwrap();
        assert_eq!(doc.title, Some("Test Document".to_string()));
        assert_eq!(doc.author, Some("Test Author".to_string()));
    }

    #[test]
    fn test_parse_simple_paragraph() {
        let xml = r#"<?xml version="1.0"?>
        <office:document-content>
            <office:body>
                <office:text>
                    <text:p>Hello World</text:p>
                </office:text>
            </office:body>
        </office:document-content>"#;

        let mut doc = OdtDocument::new();
        parse_content(xml, &mut doc).unwrap();
        assert!(doc.text.contains("Hello World"));
        assert_eq!(doc.paragraph_count, 1);
    }

    #[test]
    fn test_parse_heading() {
        let xml = r#"<?xml version="1.0"?>
        <office:document-content>
            <office:body>
                <office:text>
                    <text:h text:outline-level="1">Chapter 1</text:h>
                </office:text>
            </office:body>
        </office:document-content>"#;

        let mut doc = OdtDocument::new();
        parse_content(xml, &mut doc).unwrap();
        assert!(doc.text.contains("# Chapter 1"));
    }
}
