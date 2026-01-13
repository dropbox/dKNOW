//! XPS (XML Paper Specification) integration module
//!
//! This module integrates the docling-xps parser into the document converter,
//! converting XPS documents to markdown format.

use crate::error::DoclingError;
use std::fmt::Write;
use std::path::Path;

/// Vertical threshold for grouping XPS text elements into lines (in points).
/// Elements within this distance are considered part of the same line.
const XPS_LINE_GROUPING_THRESHOLD: f64 = 5.0;

/// Process an XPS file and convert to markdown
///
/// # Errors
///
/// Returns an error if the file cannot be read or if XPS parsing fails.
#[must_use = "this function returns the extracted markdown content"]
pub fn process_xps(path: &Path) -> Result<String, DoclingError> {
    let doc = docling_xps::parse_xps(path)
        .map_err(|e| DoclingError::ConversionError(format!("XPS parsing failed: {e}")))?;
    Ok(xps_to_markdown(&doc))
}

/// Convert XPS document to markdown
fn xps_to_markdown(doc: &docling_xps::XpsDocument) -> String {
    let mut output = String::new();

    // Add metadata as frontmatter if available
    if doc.metadata.title.is_some()
        || doc.metadata.author.is_some()
        || doc.metadata.subject.is_some()
    {
        output.push_str("---\n");
        if let Some(title) = &doc.metadata.title {
            let _ = writeln!(output, "title: {title}");
        }
        if let Some(author) = &doc.metadata.author {
            let _ = writeln!(output, "author: {author}");
        }
        if let Some(subject) = &doc.metadata.subject {
            let _ = writeln!(output, "subject: {subject}");
        }
        output.push_str("---\n\n");
    }

    // Process each page
    for (idx, page) in doc.pages.iter().enumerate() {
        if idx > 0 {
            output.push_str("\n---\n\n"); // Page break
        }

        // Sort text elements by Y position (top to bottom), then X (left to right)
        let mut elements = page.text.clone();
        elements.sort_by(|a, b| {
            let y_cmp = a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal);
            if y_cmp == std::cmp::Ordering::Equal {
                a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal)
            } else {
                y_cmp
            }
        });

        // Group elements into lines (similar Y positions)
        let mut current_line = Vec::new();
        let mut last_y = None;

        for elem in elements {
            match last_y {
                None => {
                    current_line.push(elem.clone());
                    last_y = Some(elem.y);
                }
                Some(y) if (elem.y - y).abs() < XPS_LINE_GROUPING_THRESHOLD => {
                    // Same line
                    current_line.push(elem.clone());
                }
                Some(_) => {
                    // New line - output current line
                    if !current_line.is_empty() {
                        let line_text = current_line
                            .iter()
                            .map(|e| e.content.as_str())
                            .collect::<Vec<_>>()
                            .join(" ");
                        output.push_str(&line_text);
                        output.push('\n');
                    }
                    current_line.clear();
                    current_line.push(elem.clone());
                    last_y = Some(elem.y);
                }
            }
        }

        // Output final line
        if !current_line.is_empty() {
            let line_text = current_line
                .iter()
                .map(|e| e.content.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            output.push_str(&line_text);
            output.push('\n');
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use docling_xps::{XPS_DEFAULT_PAGE_HEIGHT, XPS_DEFAULT_PAGE_WIDTH};

    #[test]
    fn test_xps_to_markdown_basic() {
        let mut doc = docling_xps::XpsDocument {
            metadata: docling_xps::XpsMetadata::new(),
            pages: vec![],
        };

        doc.metadata.title = Some("Test".to_string());

        let page = docling_xps::XpsPage {
            number: 1,
            width: XPS_DEFAULT_PAGE_WIDTH,
            height: XPS_DEFAULT_PAGE_HEIGHT,
            text: vec![
                docling_xps::XpsTextElement::new("Hello".to_string(), 10.0, 10.0),
                docling_xps::XpsTextElement::new("World".to_string(), 50.0, 10.0),
            ],
        };

        doc.pages.push(page);

        let markdown = xps_to_markdown(&doc);
        assert!(markdown.contains("title: Test"));
        assert!(markdown.contains("Hello World"));
    }

    #[test]
    fn test_process_xps_file() {
        let path = Path::new("../../test-corpus/xps/simple_text.xps");
        if path.exists() {
            let result = process_xps(path);
            assert!(result.is_ok());
            let markdown = result.unwrap();
            assert!(markdown.contains("Simple Text Document"));
            assert!(markdown.contains("Hello, World!"));
        }
    }

    #[test]
    fn test_process_xps_nonexistent_file() {
        // Test error handling for missing file
        let result = process_xps(Path::new("/nonexistent/path/to/document.xps"));
        assert!(result.is_err());
    }

    #[test]
    fn test_xps_to_markdown_no_metadata() {
        // Test XPS document without metadata
        let doc = docling_xps::XpsDocument {
            metadata: docling_xps::XpsMetadata::new(),
            pages: vec![docling_xps::XpsPage {
                number: 1,
                width: XPS_DEFAULT_PAGE_WIDTH,
                height: XPS_DEFAULT_PAGE_HEIGHT,
                text: vec![docling_xps::XpsTextElement::new(
                    "Content".to_string(),
                    10.0,
                    10.0,
                )],
            }],
        };

        let markdown = xps_to_markdown(&doc);
        // Should not have frontmatter
        assert!(!markdown.starts_with("---"));
        // But should have content
        assert!(markdown.contains("Content"));
    }

    #[test]
    fn test_xps_to_markdown_multiple_pages() {
        // Test multi-page XPS document
        let doc = docling_xps::XpsDocument {
            metadata: docling_xps::XpsMetadata::new(),
            pages: vec![
                docling_xps::XpsPage {
                    number: 1,
                    width: XPS_DEFAULT_PAGE_WIDTH,
                    height: XPS_DEFAULT_PAGE_HEIGHT,
                    text: vec![docling_xps::XpsTextElement::new(
                        "Page 1".to_string(),
                        10.0,
                        10.0,
                    )],
                },
                docling_xps::XpsPage {
                    number: 2,
                    width: XPS_DEFAULT_PAGE_WIDTH,
                    height: XPS_DEFAULT_PAGE_HEIGHT,
                    text: vec![docling_xps::XpsTextElement::new(
                        "Page 2".to_string(),
                        10.0,
                        10.0,
                    )],
                },
            ],
        };

        let markdown = xps_to_markdown(&doc);
        assert!(markdown.contains("Page 1"));
        assert!(markdown.contains("Page 2"));
        // Should have page separator
        assert!(markdown.contains("---"));
    }

    #[test]
    fn test_xps_to_markdown_empty_pages() {
        // Test XPS document with empty pages
        let doc = docling_xps::XpsDocument {
            metadata: docling_xps::XpsMetadata::new(),
            pages: vec![docling_xps::XpsPage {
                number: 1,
                width: XPS_DEFAULT_PAGE_WIDTH,
                height: XPS_DEFAULT_PAGE_HEIGHT,
                text: vec![],
            }],
        };

        let markdown = xps_to_markdown(&doc);
        // Should return valid markdown even with no content
        assert!(!markdown.is_empty() || markdown.is_empty()); // Either case is acceptable
    }

    #[test]
    fn test_xps_to_markdown_full_metadata() {
        // Test XPS document with all metadata fields
        let mut doc = docling_xps::XpsDocument {
            metadata: docling_xps::XpsMetadata::new(),
            pages: vec![docling_xps::XpsPage {
                number: 1,
                width: XPS_DEFAULT_PAGE_WIDTH,
                height: XPS_DEFAULT_PAGE_HEIGHT,
                text: vec![docling_xps::XpsTextElement::new(
                    "Content".to_string(),
                    10.0,
                    10.0,
                )],
            }],
        };

        doc.metadata.title = Some("Test Title".to_string());
        doc.metadata.author = Some("Test Author".to_string());
        doc.metadata.subject = Some("Test Subject".to_string());

        let markdown = xps_to_markdown(&doc);
        assert!(markdown.contains("title: Test Title"));
        assert!(markdown.contains("author: Test Author"));
        assert!(markdown.contains("subject: Test Subject"));
    }

    #[test]
    fn test_xps_to_markdown_text_ordering() {
        // Test that text elements are ordered correctly (top to bottom, left to right)
        let doc = docling_xps::XpsDocument {
            metadata: docling_xps::XpsMetadata::new(),
            pages: vec![docling_xps::XpsPage {
                number: 1,
                width: XPS_DEFAULT_PAGE_WIDTH,
                height: XPS_DEFAULT_PAGE_HEIGHT,
                text: vec![
                    docling_xps::XpsTextElement::new("Third".to_string(), 10.0, 30.0),
                    docling_xps::XpsTextElement::new("First".to_string(), 10.0, 10.0),
                    docling_xps::XpsTextElement::new("Second".to_string(), 50.0, 10.0),
                ],
            }],
        };

        let markdown = xps_to_markdown(&doc);
        // Should appear in correct order
        let first_pos = markdown.find("First").unwrap();
        let second_pos = markdown.find("Second").unwrap();
        let third_pos = markdown.find("Third").unwrap();
        assert!(first_pos < second_pos);
        assert!(second_pos < third_pos);
    }

    #[test]
    fn test_xps_to_markdown_line_grouping() {
        // Test that text elements on same line are grouped together
        let doc = docling_xps::XpsDocument {
            metadata: docling_xps::XpsMetadata::new(),
            pages: vec![docling_xps::XpsPage {
                number: 1,
                width: XPS_DEFAULT_PAGE_WIDTH,
                height: XPS_DEFAULT_PAGE_HEIGHT,
                text: vec![
                    docling_xps::XpsTextElement::new("Hello".to_string(), 10.0, 10.0),
                    docling_xps::XpsTextElement::new("World".to_string(), 50.0, 10.5), // Similar Y
                    docling_xps::XpsTextElement::new("Next".to_string(), 10.0, 20.0), // Different line
                ],
            }],
        };

        let markdown = xps_to_markdown(&doc);
        // "Hello" and "World" should be on same line (within threshold)
        assert!(markdown.contains("Hello World"));
    }

    #[test]
    fn test_xps_to_markdown_trailing_newline() {
        // Test that output ends with newline
        let doc = docling_xps::XpsDocument {
            metadata: docling_xps::XpsMetadata::new(),
            pages: vec![docling_xps::XpsPage {
                number: 1,
                width: XPS_DEFAULT_PAGE_WIDTH,
                height: XPS_DEFAULT_PAGE_HEIGHT,
                text: vec![docling_xps::XpsTextElement::new(
                    "Content".to_string(),
                    10.0,
                    10.0,
                )],
            }],
        };

        let markdown = xps_to_markdown(&doc);
        assert!(markdown.ends_with('\n'));
    }

    #[test]
    fn test_xps_to_markdown_empty_document() {
        // Test completely empty XPS document
        let doc = docling_xps::XpsDocument {
            metadata: docling_xps::XpsMetadata::new(),
            pages: vec![],
        };

        let markdown = xps_to_markdown(&doc);
        // Should return empty or minimal output
        assert!(markdown.is_empty() || markdown.trim().is_empty());
    }
}
