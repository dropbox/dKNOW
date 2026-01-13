//! XPS document parser
//!
//! Parses XPS files (ZIP archives containing XML documents)

use crate::error::{Result, XpsError};
use crate::metadata::XpsMetadata;
use crate::page::{XpsPage, XpsTextElement};
use crate::{XPS_DEFAULT_PAGE_HEIGHT, XPS_DEFAULT_PAGE_WIDTH};
use quick_xml::events::Event;
use quick_xml::Reader;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use zip::ZipArchive;

/// Parsed XPS document
#[derive(Debug, Clone, Default, PartialEq)]
pub struct XpsDocument {
    /// Document metadata
    pub metadata: XpsMetadata,

    /// Document pages
    pub pages: Vec<XpsPage>,
}

/// Parse XPS file from path
///
/// # Errors
///
/// Returns an error if:
/// - The file cannot be opened (`XpsError::Io`)
/// - The file is not a valid ZIP archive (`XpsError::Zip`)
/// - XML parsing fails for document structure (`XpsError::Xml`)
#[must_use = "parsing produces a result that should be handled"]
pub fn parse_xps(path: &Path) -> Result<XpsDocument> {
    let file = File::open(path)?;
    let mut archive = ZipArchive::new(file)?;

    // Extract metadata
    let metadata = extract_metadata(&mut archive)?;

    // Find document structure
    let page_paths = find_page_paths(&mut archive)?;

    // Parse each page
    let mut pages = Vec::new();
    for (idx, page_path) in page_paths.iter().enumerate() {
        match parse_page(&mut archive, page_path, idx + 1) {
            Ok(page) => pages.push(page),
            Err(e) => {
                log::warn!("Failed to parse page {}: {}", idx + 1, e);
                // Continue with other pages
            }
        }
    }

    Ok(XpsDocument { metadata, pages })
}

/// Extract metadata from docProps/core.xml
fn extract_metadata(archive: &mut ZipArchive<File>) -> Result<XpsMetadata> {
    let mut metadata = XpsMetadata::new();

    // Try to read core properties
    let content = match archive.by_name("docProps/core.xml") {
        Ok(mut file) => {
            let mut s = String::new();
            file.read_to_string(&mut s)?;
            s
        }
        Err(_) => return Ok(metadata), // No metadata file, return empty
    };

    // Parse XML to extract metadata fields
    let mut reader = Reader::from_str(&content);
    reader.trim_text(true);

    let mut buf = Vec::new();
    let mut current_tag = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e) | Event::Empty(e)) => {
                current_tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
            }
            Ok(Event::Text(e)) => {
                let text = e.unescape().unwrap_or_default().to_string();
                match current_tag.as_str() {
                    "dc:title" | "title" => metadata.title = Some(text),
                    "dc:creator" | "creator" => metadata.author = Some(text),
                    "dc:subject" | "subject" => metadata.subject = Some(text),
                    "dc:description" | "description" => metadata.description = Some(text),
                    "cp:keywords" | "keywords" => metadata.keywords = Some(text),
                    "dcterms:created" | "created" => metadata.created = Some(text),
                    "dcterms:modified" | "modified" => metadata.modified = Some(text),
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                log::warn!("XML parse error in metadata: {e}");
                break;
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(metadata)
}

/// Find paths to all page files (*.fpage)
fn find_page_paths(archive: &mut ZipArchive<File>) -> Result<Vec<String>> {
    let mut paths = Vec::new();

    // Enumerate all files and find *.fpage files
    for i in 0..archive.len() {
        let file = archive.by_index(i)?;
        let name = file.name().to_string();
        if std::path::Path::new(&name)
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("fpage"))
        {
            paths.push(name);
        }
    }

    // Sort paths to maintain page order
    paths.sort();

    if paths.is_empty() {
        return Err(XpsError::InvalidStructure(
            "No .fpage files found in XPS archive".to_string(),
        ));
    }

    Ok(paths)
}

/// Parse a single page file (.fpage)
fn parse_page(
    archive: &mut ZipArchive<File>,
    page_path: &str,
    page_number: usize,
) -> Result<XpsPage> {
    let mut file = archive.by_name(page_path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    drop(file);

    let mut page = XpsPage {
        number: page_number,
        width: XPS_DEFAULT_PAGE_WIDTH,
        height: XPS_DEFAULT_PAGE_HEIGHT,
        text: Vec::new(),
    };

    // Parse XML to extract text and layout
    let mut reader = Reader::from_str(&content);
    reader.trim_text(true);

    let mut buf = Vec::new();
    let mut current_x = 0.0;
    let mut current_y = 0.0;
    let mut current_font_size = None;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e) | Event::Empty(e)) => {
                let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();

                match tag_name.as_str() {
                    "FixedPage" => {
                        // Extract page dimensions
                        for attr in e.attributes().flatten() {
                            let key = String::from_utf8_lossy(attr.key.as_ref());
                            let value = String::from_utf8_lossy(&attr.value);

                            match key.as_ref() {
                                "Width" => {
                                    if let Ok(w) = value.parse::<f64>() {
                                        page.width = w;
                                    }
                                }
                                "Height" => {
                                    if let Ok(h) = value.parse::<f64>() {
                                        page.height = h;
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    "Glyphs" => {
                        // Extract position and font info
                        for attr in e.attributes().flatten() {
                            let key = String::from_utf8_lossy(attr.key.as_ref());
                            let value = String::from_utf8_lossy(&attr.value);

                            match key.as_ref() {
                                "OriginX" => {
                                    current_x = value.parse::<f64>().unwrap_or(0.0);
                                }
                                "OriginY" => {
                                    current_y = value.parse::<f64>().unwrap_or(0.0);
                                }
                                "FontRenderingEmSize" => {
                                    current_font_size = value.parse::<f64>().ok();
                                }
                                "UnicodeString" => {
                                    // Text content as attribute
                                    let text = value.to_string();
                                    if !text.is_empty() {
                                        let mut elem =
                                            XpsTextElement::new(text, current_x, current_y);
                                        elem.font_size = current_font_size;
                                        page.text.push(elem);
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::End(_e)) => {
                // End tags don't require processing for XPS
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                log::warn!("XML parse error in page {page_number}: {e}");
                break;
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(page)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{XPS_DEFAULT_PAGE_HEIGHT, XPS_DEFAULT_PAGE_WIDTH};

    #[test]
    fn test_page_structure() {
        let page = XpsPage {
            number: 1,
            width: XPS_DEFAULT_PAGE_WIDTH,
            height: XPS_DEFAULT_PAGE_HEIGHT,
            text: vec![XpsTextElement::new("Hello".to_string(), 10.0, 20.0)],
        };

        assert_eq!(page.number, 1);
        assert_eq!(page.text.len(), 1);
        assert_eq!(page.text[0].content, "Hello");
    }

    #[test]
    fn test_metadata() {
        let mut meta = XpsMetadata::new();
        meta.title = Some("Test Document".to_string());
        meta.author = Some("Test Author".to_string());

        assert_eq!(meta.title, Some("Test Document".to_string()));
        assert_eq!(meta.author, Some("Test Author".to_string()));
    }

    #[test]
    fn test_parse_simple_xps() {
        let path = Path::new("../../test-corpus/xps/simple_text.xps");
        if path.exists() {
            let doc = parse_xps(path).expect("Failed to parse simple XPS file");

            assert_eq!(doc.metadata.title, Some("Simple Text Document".to_string()));
            assert_eq!(doc.metadata.author, Some("Test Author".to_string()));
            assert_eq!(doc.pages.len(), 1);
            assert_eq!(doc.pages[0].text.len(), 2);
            assert_eq!(doc.pages[0].text[0].content, "Hello, World!");
        }
    }

    #[test]
    fn test_parse_multi_page_xps() {
        let path = Path::new("../../test-corpus/xps/multi_page.xps");
        if path.exists() {
            let doc = parse_xps(path).expect("Failed to parse multi-page XPS file");

            assert_eq!(doc.metadata.title, Some("Multi-Page Document".to_string()));
            assert_eq!(doc.pages.len(), 3);
            assert!(doc.pages[0].text[0].content.contains("Page 1"));
            assert!(doc.pages[1].text[0].content.contains("Page 2"));
            assert!(doc.pages[2].text[0].content.contains("Page 3"));
        }
    }
}
