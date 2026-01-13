//! `OpenDocument` Presentation (ODP) format parser
//!
//! Parses .odp files (`OpenDocument` Presentation format used by `LibreOffice` Impress).
//!
//! ## Format Structure
//! ODP files are ZIP archives containing:
//! - `content.xml` - Main presentation content (slides)
//! - `styles.xml` - Presentation styles
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

/// Represents a single slide with structured content
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct OdpSlide {
    /// Slide number (1-indexed)
    pub number: usize,
    /// Slide name from draw:name attribute (e.g., "Slide 1", "Introduction")
    pub name: Option<String>,
    /// Slide content as text paragraphs
    pub paragraphs: Vec<String>,
    /// Slide text content (all paragraphs concatenated)
    pub text: String,
    /// Image references (xlink:href paths to embedded images)
    pub images: Vec<String>,
    /// Slide transition type (e.g., "fade", "push", "wipe")
    pub transition_type: Option<String>,
    /// Slide transition speed (e.g., "slow", "medium", "fast")
    pub transition_speed: Option<String>,
    /// Slide display duration (e.g., "3s", "5s", "PT3S")
    pub duration: Option<String>,
}

/// Slide transition and timing metadata
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct SlideMetadata {
    /// Slide transition type (e.g., "fade", "push", "wipe")
    pub transition_type: Option<String>,
    /// Slide transition speed (e.g., "slow", "medium", "fast")
    pub transition_speed: Option<String>,
    /// Slide display duration (e.g., "3s", "5s", "PT3S")
    pub duration: Option<String>,
}

/// Parsed ODP presentation content
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct OdpDocument {
    /// Presentation text content (all slides concatenated)
    pub text: String,
    /// Presentation title (from metadata)
    pub title: Option<String>,
    /// Presentation author (from metadata)
    pub author: Option<String>,
    /// Number of slides
    pub slide_count: usize,
    /// Slide titles (if available)
    pub slide_titles: Vec<String>,
    /// Slide names from draw:name attribute (e.g., "Slide 1", "Introduction")
    pub slide_names: Vec<String>,
    /// Slide transition and timing metadata (one entry per slide)
    pub slide_metadata: Vec<SlideMetadata>,
}

impl OdpDocument {
    /// Create a new empty ODP document
    #[inline]
    #[must_use = "creates empty ODP document"]
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

    /// Start a new slide
    #[inline]
    pub fn start_slide(&mut self, slide_num: usize) {
        if slide_num > 1 {
            self.add_newline();
            self.add_newline();
        }
        self.add_text(&format!("## Slide {slide_num}"));
        self.add_newline();
        self.add_newline();
    }
}

/// Parse ODP file from a path
///
/// # Errors
///
/// Returns an error if the file cannot be opened (I/O error) or if the ODP content
/// is invalid (not a valid ZIP archive, missing content.xml, or malformed XML).
#[must_use = "this function returns a parsed ODP document that should be processed"]
pub fn parse_odp_file<P: AsRef<Path>>(path: P) -> Result<OdpDocument> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    parse_odp_reader(reader)
}

/// Parse ODP from a reader
///
/// # Errors
///
/// Returns an error if the reader content is not a valid ZIP archive, if content.xml
/// is missing, or if the XML content is malformed.
#[must_use = "this function returns a parsed ODP document that should be processed"]
pub fn parse_odp_reader<R: Read + std::io::Seek>(reader: R) -> Result<OdpDocument> {
    let mut archive = ZipArchive::new(reader)?;
    let mut doc = OdpDocument::new();

    // Parse metadata
    if let Ok(meta_xml) = extract_file_as_string(&mut archive, "meta.xml") {
        parse_metadata(&meta_xml, &mut doc)?;
    }

    // Parse main content (slides)
    let content_xml = extract_file_as_string(&mut archive, "content.xml")?;
    parse_content(&content_xml, &mut doc)?;

    Ok(doc)
}

/// Parse metadata from meta.xml
fn parse_metadata(xml_content: &str, doc: &mut OdpDocument) -> Result<()> {
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

/// Parse presentation content from content.xml
#[allow(clippy::too_many_lines)] // Complex XML parsing - keeping together for clarity
fn parse_content(xml_content: &str, doc: &mut OdpDocument) -> Result<()> {
    let mut reader = Reader::from_str(xml_content);
    reader.trim_text(true);

    let mut buf = Vec::new();
    let mut in_paragraph = false;
    let mut in_list = false;
    let mut current_text = String::new();
    let mut slide_num = 0;
    let mut list_depth = 0;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e) | Event::Empty(e)) => {
                let name = e.name();
                let name_bytes = name.as_ref();

                // Match full qualified names (namespace:localname)
                match name_bytes {
                    // Draw namespace elements (slides, text boxes)
                    b"draw:page" => {
                        // New slide
                        slide_num += 1;
                        doc.slide_count += 1;

                        // Extract slide metadata from attributes
                        let mut slide_name: Option<String> = None;
                        let mut metadata = SlideMetadata::default();

                        for attr in e.attributes().flatten() {
                            let key_local = attr.key.local_name();
                            let key_prefix =
                                attr.key.prefix().map(quick_xml::name::Prefix::into_inner);
                            let value = String::from_utf8_lossy(&attr.value).to_string();

                            match (key_prefix, key_local.as_ref()) {
                                // draw:name - Slide name
                                (Some(b"draw"), b"name") => {
                                    slide_name = Some(value);
                                }
                                // presentation:transition-type - Transition effect
                                (Some(b"presentation"), b"transition-type") => {
                                    metadata.transition_type = Some(value);
                                }
                                // presentation:transition-speed - Transition speed
                                (Some(b"presentation"), b"transition-speed") => {
                                    metadata.transition_speed = Some(value);
                                }
                                // presentation:duration - Slide duration (ODF format)
                                (Some(b"presentation"), b"duration") => {
                                    metadata.duration = Some(value);
                                }
                                // smil:dur - Duration in SMIL format (alternative)
                                (Some(b"smil"), b"dur") => {
                                    // Prefer presentation:duration if already set
                                    if metadata.duration.is_none() {
                                        metadata.duration = Some(value);
                                    }
                                }
                                _ => {}
                            }
                        }

                        // Store slide name
                        if let Some(name) = slide_name {
                            doc.slide_names.push(name);
                        }

                        // Store slide metadata
                        doc.slide_metadata.push(metadata);

                        doc.start_slide(slide_num);
                    }
                    // Note: draw:frame/draw:text-box content captured in paragraphs (fall through to default)
                    b"draw:image" => {
                        // Extract image metadata from xlink:href attribute
                        for attr in e.attributes().flatten() {
                            let key_local = attr.key.local_name();
                            let key_prefix =
                                attr.key.prefix().map(quick_xml::name::Prefix::into_inner);

                            if matches!(key_prefix, Some(b"xlink")) && key_local.as_ref() == b"href"
                            {
                                let href = String::from_utf8_lossy(&attr.value);
                                doc.add_text(&format!("![Image]({href})"));
                            }
                        }
                    }
                    // Text namespace elements
                    b"text:p" => {
                        in_paragraph = true;
                        current_text.clear();
                    }
                    b"text:list" => {
                        in_list = true;
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
                    _ => {}
                }
            }
            Ok(Event::Text(e)) if in_paragraph => {
                let text = e.unescape()?.into_owned();
                current_text.push_str(&text);
            }
            Ok(Event::End(e)) => {
                let name = e.name();
                let name_bytes = name.as_ref();

                match name_bytes {
                    // Note: draw:page/draw:frame/draw:text-box end tags need no action (fall through to default)
                    // Text namespace elements
                    b"text:p" => {
                        if in_paragraph {
                            let trimmed = current_text.trim();
                            if !trimmed.is_empty() {
                                // Add list marker if in list
                                if in_list && list_depth > 0 {
                                    doc.add_text(&format!(
                                        "{}• {}",
                                        "  ".repeat(list_depth - 1),
                                        trimmed
                                    ));
                                } else {
                                    doc.add_text(trimmed);
                                }
                            }
                            in_paragraph = false;
                            current_text.clear();
                        }
                    }
                    b"text:list" => {
                        list_depth = list_depth.saturating_sub(1);
                        if list_depth == 0 {
                            in_list = false;
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

/// Extract slides with structured content
/// Returns a vector of slides, each containing paragraphs
///
/// # Errors
///
/// Returns an error if the file cannot be opened (I/O error), if the content is not
/// a valid ZIP archive, if content.xml is missing, or if the XML is malformed.
#[must_use = "this function returns parsed ODP slides that should be processed"]
pub fn parse_odp_slides<P: AsRef<Path>>(path: P) -> Result<Vec<OdpSlide>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut archive = ZipArchive::new(reader)?;

    // Parse main content (slides)
    let content_xml = extract_file_as_string(&mut archive, "content.xml")?;
    parse_slides(&content_xml)
}

/// Parse slides from content.xml
#[allow(clippy::too_many_lines)] // Complex XML parsing - keeping together for clarity
fn parse_slides(xml_content: &str) -> Result<Vec<OdpSlide>> {
    let mut reader = Reader::from_str(xml_content);
    reader.trim_text(true);

    let mut buf = Vec::new();
    let mut slides = Vec::new();
    let mut in_paragraph = false;
    let mut in_list = false;
    let mut current_text = String::new();
    let mut current_slide_paragraphs: Vec<String> = Vec::new();
    let mut current_slide_images: Vec<String> = Vec::new();
    let mut current_slide_name: Option<String> = None;
    let mut slide_num = 0;
    let mut list_depth = 0;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e) | Event::Empty(e)) => {
                let name = e.name();
                let name_bytes = name.as_ref();

                match name_bytes {
                    b"draw:page" => {
                        // Save previous slide if exists
                        if slide_num > 0 && !current_slide_paragraphs.is_empty() {
                            let slide_text = current_slide_paragraphs.join("\n");
                            slides.push(OdpSlide {
                                number: slide_num,
                                name: current_slide_name.clone(),
                                paragraphs: current_slide_paragraphs.clone(),
                                text: slide_text,
                                images: current_slide_images.clone(),
                                transition_type: None,
                                transition_speed: None,
                                duration: None,
                            });
                        }

                        // Start new slide
                        slide_num += 1;
                        current_slide_paragraphs.clear();
                        current_slide_images.clear();

                        // Extract slide name from draw:name attribute
                        current_slide_name = e
                            .attributes()
                            .filter_map(std::result::Result::ok)
                            .find(|attr| {
                                attr.key.local_name().as_ref() == b"name"
                                    && matches!(
                                        attr.key.prefix().map(quick_xml::name::Prefix::into_inner),
                                        Some(b"draw")
                                    )
                            })
                            .and_then(|attr| String::from_utf8(attr.value.to_vec()).ok());
                    }
                    b"draw:image" => {
                        // Extract image metadata from xlink:href attribute
                        if let Some(href_attr) = e
                            .attributes()
                            .filter_map(std::result::Result::ok)
                            .find(|attr| {
                                attr.key.local_name().as_ref() == b"href"
                                    && matches!(
                                        attr.key.prefix().map(quick_xml::name::Prefix::into_inner),
                                        Some(b"xlink")
                                    )
                            })
                        {
                            let href = String::from_utf8_lossy(&href_attr.value);
                            current_slide_paragraphs.push(format!("![Image]({href})"));
                        }
                    }
                    b"text:p" => {
                        in_paragraph = true;
                        current_text.clear();
                    }
                    b"text:list" => {
                        in_list = true;
                        list_depth += 1;
                    }
                    b"text:s" => {
                        current_text.push(' ');
                    }
                    b"text:tab" => {
                        current_text.push('\t');
                    }
                    b"text:line-break" => {
                        current_text.push('\n');
                    }
                    // Note: b"draw:image" case is handled above at line 406
                    // Duplicate case removed to fix unreachable pattern warning
                    _ => {}
                }
            }
            Ok(Event::Text(e)) if in_paragraph => {
                let text = e.unescape()?.into_owned();
                current_text.push_str(&text);
            }
            Ok(Event::End(e)) => {
                let name = e.name();
                let name_bytes = name.as_ref();

                match name_bytes {
                    b"text:p" => {
                        if in_paragraph {
                            let trimmed = current_text.trim();
                            if !trimmed.is_empty() {
                                // Add list marker if in list
                                let paragraph = if in_list && list_depth > 0 {
                                    format!("{}• {}", "  ".repeat(list_depth - 1), trimmed)
                                } else {
                                    trimmed.to_string()
                                };
                                current_slide_paragraphs.push(paragraph);
                            }
                            in_paragraph = false;
                            current_text.clear();
                        }
                    }
                    b"text:list" => {
                        list_depth = list_depth.saturating_sub(1);
                        if list_depth == 0 {
                            in_list = false;
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => {
                // Save last slide if exists
                if slide_num > 0 && !current_slide_paragraphs.is_empty() {
                    let slide_text = current_slide_paragraphs.join("\n");
                    slides.push(OdpSlide {
                        number: slide_num,
                        name: current_slide_name,
                        paragraphs: current_slide_paragraphs,
                        text: slide_text,
                        images: current_slide_images,
                        transition_type: None,
                        transition_speed: None,
                        duration: None,
                    });
                }
                break;
            }
            Err(e) => return Err(e.into()),
            _ => {}
        }
        buf.clear();
    }

    Ok(slides)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_odp_document_creation() {
        let mut doc = OdpDocument::new();
        assert_eq!(doc.text, "");
        assert_eq!(doc.slide_count, 0);

        doc.start_slide(1);
        assert_eq!(doc.slide_count, 0); // start_slide doesn't increment count
        assert!(doc.text.contains("## Slide 1"));

        doc.add_text("Slide content");
        assert!(doc.text.contains("Slide content"));
    }

    #[test]
    fn test_parse_metadata() {
        let xml = r#"<?xml version="1.0"?>
        <office:document-meta>
            <office:meta>
                <dc:title>Test Presentation</dc:title>
                <meta:initial-creator>Test Author</meta:initial-creator>
            </office:meta>
        </office:document-meta>"#;

        let mut doc = OdpDocument::new();
        parse_metadata(xml, &mut doc).unwrap();
        assert_eq!(doc.title, Some("Test Presentation".to_string()));
        assert_eq!(doc.author, Some("Test Author".to_string()));
    }

    #[test]
    fn test_parse_simple_slide() {
        let xml = r#"<?xml version="1.0"?>
        <office:document-content>
            <office:body>
                <office:presentation>
                    <draw:page draw:name="page1">
                        <draw:frame>
                            <draw:text-box>
                                <text:p>Slide Title</text:p>
                            </draw:text-box>
                        </draw:frame>
                    </draw:page>
                </office:presentation>
            </office:body>
        </office:document-content>"#;

        let mut doc = OdpDocument::new();
        parse_content(xml, &mut doc).unwrap();
        assert_eq!(doc.slide_count, 1);
        assert!(doc.text.contains("Slide Title"));
    }

    #[test]
    fn test_parse_multiple_slides() {
        let xml = r#"<?xml version="1.0"?>
        <office:document-content>
            <office:body>
                <office:presentation>
                    <draw:page draw:name="page1">
                        <draw:text-box>
                            <text:p>Slide 1</text:p>
                        </draw:text-box>
                    </draw:page>
                    <draw:page draw:name="page2">
                        <draw:text-box>
                            <text:p>Slide 2</text:p>
                        </draw:text-box>
                    </draw:page>
                </office:presentation>
            </office:body>
        </office:document-content>"#;

        let mut doc = OdpDocument::new();
        parse_content(xml, &mut doc).unwrap();
        assert_eq!(doc.slide_count, 2);
        assert!(doc.text.contains("Slide 1"));
        assert!(doc.text.contains("Slide 2"));
    }

    #[test]
    fn test_parse_slide_names() {
        let xml = r#"<?xml version="1.0"?>
        <office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
                               xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0"
                               xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0">
            <office:body>
                <office:presentation>
                    <draw:page draw:name="Introduction">
                        <draw:frame><draw:text-box><text:p>Welcome</text:p></draw:text-box></draw:frame>
                    </draw:page>
                    <draw:page draw:name="Main Content">
                        <draw:frame><draw:text-box><text:p>Details</text:p></draw:text-box></draw:frame>
                    </draw:page>
                    <draw:page draw:name="Conclusion">
                        <draw:frame><draw:text-box><text:p>Thank you</text:p></draw:text-box></draw:frame>
                    </draw:page>
                </office:presentation>
            </office:body>
        </office:document-content>"#;

        let mut doc = OdpDocument::new();
        parse_content(xml, &mut doc).unwrap();

        assert_eq!(doc.slide_count, 3);
        assert_eq!(doc.slide_names.len(), 3);
        assert_eq!(doc.slide_names[0], "Introduction");
        assert_eq!(doc.slide_names[1], "Main Content");
        assert_eq!(doc.slide_names[2], "Conclusion");
    }

    #[test]
    fn test_parse_slide_transitions() {
        let xml = r#"<?xml version="1.0"?>
        <office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
                               xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0"
                               xmlns:presentation="urn:oasis:names:tc:opendocument:xmlns:presentation:1.0"
                               xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0">
            <office:body>
                <office:presentation>
                    <draw:page draw:name="Slide 1" presentation:transition-type="fade" presentation:transition-speed="fast">
                        <draw:frame><draw:text-box><text:p>First slide with fade transition</text:p></draw:text-box></draw:frame>
                    </draw:page>
                    <draw:page draw:name="Slide 2" presentation:transition-type="push" presentation:transition-speed="medium" presentation:duration="PT5S">
                        <draw:frame><draw:text-box><text:p>Second slide with push transition and 5s duration</text:p></draw:text-box></draw:frame>
                    </draw:page>
                    <draw:page draw:name="Slide 3">
                        <draw:frame><draw:text-box><text:p>Third slide with no transition metadata</text:p></draw:text-box></draw:frame>
                    </draw:page>
                </office:presentation>
            </office:body>
        </office:document-content>"#;

        let mut doc = OdpDocument::new();
        parse_content(xml, &mut doc).unwrap();

        // Verify slide count
        assert_eq!(doc.slide_count, 3);
        assert_eq!(doc.slide_metadata.len(), 3);

        // Verify first slide metadata (fade transition, fast speed)
        assert_eq!(
            doc.slide_metadata[0].transition_type.as_deref(),
            Some("fade")
        );
        assert_eq!(
            doc.slide_metadata[0].transition_speed.as_deref(),
            Some("fast")
        );
        assert_eq!(doc.slide_metadata[0].duration.as_ref(), None);

        // Verify second slide metadata (push transition, medium speed, 5s duration)
        assert_eq!(
            doc.slide_metadata[1].transition_type.as_deref(),
            Some("push")
        );
        assert_eq!(
            doc.slide_metadata[1].transition_speed.as_deref(),
            Some("medium")
        );
        assert_eq!(doc.slide_metadata[1].duration.as_deref(), Some("PT5S"));

        // Verify third slide metadata (no transition info)
        assert_eq!(doc.slide_metadata[2].transition_type.as_ref(), None);
        assert_eq!(doc.slide_metadata[2].transition_speed.as_ref(), None);
        assert_eq!(doc.slide_metadata[2].duration.as_ref(), None);
    }

    #[test]
    fn test_parse_slide_smil_duration() {
        let xml = r#"<?xml version="1.0"?>
        <office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
                               xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0"
                               xmlns:smil="urn:oasis:names:tc:opendocument:xmlns:smil-compatible:1.0"
                               xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0">
            <office:body>
                <office:presentation>
                    <draw:page draw:name="Slide 1" smil:dur="3s">
                        <draw:frame><draw:text-box><text:p>Slide with SMIL duration</text:p></draw:text-box></draw:frame>
                    </draw:page>
                </office:presentation>
            </office:body>
        </office:document-content>"#;

        let mut doc = OdpDocument::new();
        parse_content(xml, &mut doc).unwrap();

        // Verify SMIL duration is captured
        assert_eq!(doc.slide_count, 1);
        assert_eq!(doc.slide_metadata.len(), 1);
        assert_eq!(doc.slide_metadata[0].duration.as_deref(), Some("3s"));
    }

    #[test]
    fn test_parse_all_transition_types() {
        let xml = r#"<?xml version="1.0"?>
        <office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
                               xmlns:draw="urn:oasis:names:tc:opendocument:xmlns:drawing:1.0"
                               xmlns:presentation="urn:oasis:names:tc:opendocument:xmlns:presentation:1.0"
                               xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0">
            <office:body>
                <office:presentation>
                    <draw:page presentation:transition-type="fade">
                        <draw:frame><draw:text-box><text:p>Fade</text:p></draw:text-box></draw:frame>
                    </draw:page>
                    <draw:page presentation:transition-type="push">
                        <draw:frame><draw:text-box><text:p>Push</text:p></draw:text-box></draw:frame>
                    </draw:page>
                    <draw:page presentation:transition-type="wipe">
                        <draw:frame><draw:text-box><text:p>Wipe</text:p></draw:text-box></draw:frame>
                    </draw:page>
                    <draw:page presentation:transition-type="dissolve">
                        <draw:frame><draw:text-box><text:p>Dissolve</text:p></draw:text-box></draw:frame>
                    </draw:page>
                </office:presentation>
            </office:body>
        </office:document-content>"#;

        let mut doc = OdpDocument::new();
        parse_content(xml, &mut doc).unwrap();

        // Verify all transition types are captured
        assert_eq!(doc.slide_count, 4);
        assert_eq!(doc.slide_metadata.len(), 4);
        assert_eq!(
            doc.slide_metadata[0].transition_type.as_deref(),
            Some("fade")
        );
        assert_eq!(
            doc.slide_metadata[1].transition_type.as_deref(),
            Some("push")
        );
        assert_eq!(
            doc.slide_metadata[2].transition_type.as_deref(),
            Some("wipe")
        );
        assert_eq!(
            doc.slide_metadata[3].transition_type.as_deref(),
            Some("dissolve")
        );
    }
}
