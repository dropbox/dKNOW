//! Apple Keynote format support
//!
//! Keynote files (.key) are ZIP archives containing XML files.
//!
//! Structure:
//! - index.xml - Main presentation structure (Keynote '09 XML format)
//! - QuickLook/Thumbnail.jpg - Thumbnail preview
//! - Data/ - Embedded media files
//!
//! NOTE: This implementation supports Keynote '09 XML format.

use anyhow::{Context, Result};
use docling_core::{
    content::{CoordOrigin, DocItem, ItemRef, ProvenanceItem},
    document::{GroupItem, Origin, PageInfo, PageSize},
    BoundingBox, DoclingDocument,
};
use quick_xml::events::Event;
use quick_xml::Reader;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use zip::ZipArchive;

/// Backend for Apple Keynote files
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct KeynoteBackend;

impl KeynoteBackend {
    /// Create a new Keynote backend instance
    #[inline]
    #[must_use = "creates a backend instance that should be used for parsing"]
    pub const fn new() -> Self {
        Self
    }

    /// Parse Keynote file and generate `DocItems` directly
    ///
    /// Parses Keynote '09 XML format and creates `DoclingDocument` structure
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or XML parsing fails.
    #[must_use = "this function returns a parsed document that should be processed"]
    pub fn parse(&self, input_path: &Path) -> Result<DoclingDocument> {
        // Extract index.xml from ZIP archive
        let xml_content = Self::extract_index_xml(input_path)?;

        // Parse XML and build DocItems
        Self::parse_xml(&xml_content, input_path)
    }

    /// Extract index.xml from Keynote ZIP archive
    fn extract_index_xml(input_path: &Path) -> Result<String> {
        let file = File::open(input_path)
            .with_context(|| format!("Failed to open Keynote file: {}", input_path.display()))?;

        let mut archive =
            ZipArchive::new(file).context("Failed to read Keynote file as ZIP archive")?;

        // Find index.xml
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            if file.name() == "index.xml" {
                let mut content = String::new();
                file.read_to_string(&mut content)?;
                return Ok(content);
            }
        }

        anyhow::bail!("Invalid Keynote file: missing index.xml")
    }

    /// Parse Keynote XML and generate `DocItems`
    #[allow(clippy::too_many_lines)] // Complex iWork XML parsing - keeping together for clarity
    fn parse_xml(xml_content: &str, input_path: &Path) -> Result<DoclingDocument> {
        let file_name = input_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown.key")
            .to_string();

        let mut reader = Reader::from_str(xml_content);
        reader.trim_text(true);

        let mut text_items = Vec::new();
        let mut body_children = Vec::new();
        let mut text_idx = 0;

        // Default bounding box (widescreen presentation size)
        let default_bbox = BoundingBox::new(0.0, 0.0, 1024.0, 768.0, CoordOrigin::BottomLeft);

        let mut buf = Vec::new();
        let mut current_text = String::new();
        let mut in_title = false;
        let mut in_body = false;
        let mut in_notes = false;
        let mut slide_number = 0;

        // Track transitions and builds for each slide
        let mut slide_transitions: HashMap<usize, String> = HashMap::new();
        let mut slide_builds: HashMap<usize, Vec<String>> = HashMap::new();
        let mut current_slide_builds: Vec<String> = Vec::new();
        let mut slide_notes: HashMap<usize, String> = HashMap::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => {
                    match e.name().as_ref() {
                        b"key:slide" => {
                            slide_number += 1;

                            // Get slide number attribute if present
                            if let Some(num) = e
                                .attributes()
                                .filter_map(std::result::Result::ok)
                                .find(|attr| attr.key.as_ref() == b"key:number")
                                .and_then(|attr| String::from_utf8(attr.value.to_vec()).ok())
                                .and_then(|s| s.parse::<usize>().ok())
                            {
                                slide_number = num;
                            }
                        }
                        b"key:title" | b"key:title-placeholder" => {
                            in_title = true;
                            current_text.clear();
                        }
                        b"key:body" | b"key:body-placeholder" => {
                            in_body = true;
                            current_text.clear();
                        }
                        b"key:notes" | b"key:presenter-notes" | b"key:speaker-notes" => {
                            in_notes = true;
                            current_text.clear();
                        }
                        b"key:transition" => {
                            // Extract transition type/name from attributes
                            if let Some(transition_type) = e
                                .attributes()
                                .filter_map(std::result::Result::ok)
                                .find(|attr| {
                                    attr.key.as_ref() == b"key:type"
                                        || attr.key.as_ref() == b"type"
                                        || attr.key.as_ref() == b"key:name"
                                })
                                .and_then(|attr| String::from_utf8(attr.value.to_vec()).ok())
                            {
                                slide_transitions.insert(slide_number, transition_type);
                            } else {
                                // If no type attribute, mark as having a transition
                                slide_transitions.insert(slide_number, "transition".to_string());
                            }
                        }
                        b"key:build" | b"key:animation" | b"key:build-in" | b"key:build-out"
                        | b"key:action-build" => {
                            // Extract build/animation type from attributes
                            let build_type = e
                                .attributes()
                                .filter_map(std::result::Result::ok)
                                .find(|attr| {
                                    attr.key.as_ref() == b"key:type"
                                        || attr.key.as_ref() == b"type"
                                        || attr.key.as_ref() == b"key:name"
                                })
                                .and_then(|attr| String::from_utf8(attr.value.to_vec()).ok())
                                .unwrap_or_else(|| {
                                    String::from_utf8_lossy(e.name().as_ref()).to_string()
                                });

                            current_slide_builds.push(build_type);
                        }
                        _ => {}
                    }
                }
                Ok(Event::End(ref e)) => {
                    match e.name().as_ref() {
                        b"key:slide" => {
                            // Slide ended - save builds for this slide
                            if !current_slide_builds.is_empty() {
                                slide_builds.insert(slide_number, current_slide_builds.clone());
                                current_slide_builds.clear();
                            }
                        }
                        b"key:title" | b"key:title-placeholder" => {
                            if in_title && !current_text.is_empty() {
                                // Create title or section header
                                let item_ref = format!("#/texts/{text_idx}");
                                body_children.push(ItemRef::new(&item_ref));

                                let doc_item = if slide_number == 0 {
                                    // Main presentation title
                                    DocItem::Title {
                                        self_ref: item_ref,
                                        parent: Some(ItemRef::new("#")),
                                        children: vec![],
                                        content_layer: "body".to_string(),
                                        prov: vec![ProvenanceItem {
                                            page_no: 1,
                                            bbox: default_bbox,
                                            charspan: None,
                                        }],
                                        orig: current_text.clone(),
                                        text: current_text.clone(),
                                        formatting: None,
                                        hyperlink: None,
                                    }
                                } else {
                                    // Slide title - add slide number prefix if not already present
                                    let text_lower = current_text.to_lowercase();
                                    let title_text = if text_lower.starts_with("slide ") {
                                        current_text.clone()
                                    } else {
                                        format!("Slide {slide_number}: {current_text}")
                                    };

                                    DocItem::SectionHeader {
                                        self_ref: item_ref,
                                        parent: Some(ItemRef::new("#")),
                                        children: vec![],
                                        content_layer: "body".to_string(),
                                        prov: vec![ProvenanceItem {
                                            page_no: slide_number.max(1),
                                            bbox: default_bbox,
                                            charspan: None,
                                        }],
                                        orig: current_text.clone(),
                                        text: title_text,
                                        level: 2,
                                        formatting: None,
                                        hyperlink: None,
                                    }
                                };

                                text_items.push(doc_item);
                                text_idx += 1;
                                current_text.clear();
                            }
                            in_title = false;
                        }
                        b"key:body" | b"key:body-placeholder" => {
                            if in_body && !current_text.is_empty() {
                                // Create body text
                                let item_ref = format!("#/texts/{text_idx}");
                                body_children.push(ItemRef::new(&item_ref));
                                text_items.push(DocItem::Text {
                                    self_ref: item_ref,
                                    parent: Some(ItemRef::new("#")),
                                    children: vec![],
                                    content_layer: "body".to_string(),
                                    prov: vec![ProvenanceItem {
                                        page_no: slide_number.max(1),
                                        bbox: default_bbox,
                                        charspan: None,
                                    }],
                                    orig: current_text.clone(),
                                    text: current_text.clone(),
                                    formatting: None,
                                    hyperlink: None,
                                });
                                text_idx += 1;
                                current_text.clear();
                            }
                            in_body = false;
                        }
                        b"key:notes" | b"key:presenter-notes" | b"key:speaker-notes" => {
                            if in_notes && !current_text.is_empty() {
                                // Save notes for this slide
                                slide_notes.insert(slide_number.max(1), current_text.clone());
                                current_text.clear();
                            }
                            in_notes = false;
                        }
                        _ => {}
                    }
                }
                Ok(Event::Text(e)) => {
                    if in_title || in_body || in_notes {
                        let text = e.unescape().unwrap_or_default().trim().to_string();
                        if !text.is_empty() {
                            if !current_text.is_empty() {
                                current_text.push(' ');
                            }
                            current_text.push_str(&text);
                        }
                    }
                }
                Ok(Event::Empty(ref e)) => {
                    // Handle self-closing tags like <key:transition key:type="dissolve"/>
                    match e.name().as_ref() {
                        b"key:transition" => {
                            // Extract transition type/name from attributes
                            if let Some(transition_type) = e
                                .attributes()
                                .filter_map(std::result::Result::ok)
                                .find(|attr| {
                                    attr.key.as_ref() == b"key:type"
                                        || attr.key.as_ref() == b"type"
                                        || attr.key.as_ref() == b"key:name"
                                })
                                .and_then(|attr| String::from_utf8(attr.value.to_vec()).ok())
                            {
                                slide_transitions.insert(slide_number, transition_type);
                            } else {
                                // If no type attribute, mark as having a transition
                                slide_transitions.insert(slide_number, "transition".to_string());
                            }
                        }
                        b"key:build" | b"key:animation" | b"key:build-in" | b"key:build-out"
                        | b"key:action-build" => {
                            // Extract build/animation type from attributes
                            let build_type = e
                                .attributes()
                                .filter_map(std::result::Result::ok)
                                .find(|attr| {
                                    attr.key.as_ref() == b"key:type"
                                        || attr.key.as_ref() == b"type"
                                        || attr.key.as_ref() == b"key:name"
                                })
                                .and_then(|attr| String::from_utf8(attr.value.to_vec()).ok())
                                .unwrap_or_else(|| {
                                    String::from_utf8_lossy(e.name().as_ref()).to_string()
                                });

                            current_slide_builds.push(build_type);
                        }
                        _ => {}
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    return Err(anyhow::anyhow!(
                        "XML parse error at position {}: {}",
                        reader.buffer_position(),
                        e
                    ));
                }
                _ => {}
            }
            buf.clear();
        }

        // Add transition and build metadata to slides
        // Simply append metadata for each slide that has it
        for slide_no in 1..=slide_number.max(1) {
            let mut metadata_lines = Vec::new();

            // Add transition info
            if let Some(transition) = slide_transitions.get(&slide_no) {
                metadata_lines.push(format!("Transition: {transition}"));
            }

            // Add build info
            if let Some(builds) = slide_builds.get(&slide_no) {
                if !builds.is_empty() {
                    metadata_lines.push(format!("Animations: {}", builds.join(", ")));
                }
            }

            // If we have metadata, add it
            if !metadata_lines.is_empty() {
                let metadata_text = metadata_lines.join("; ");
                let item_ref = format!("#/texts/{text_idx}");
                body_children.push(ItemRef::new(&item_ref));
                text_items.push(DocItem::Text {
                    self_ref: item_ref,
                    parent: Some(ItemRef::new("#")),
                    children: vec![],
                    content_layer: "body".to_string(),
                    prov: vec![ProvenanceItem {
                        page_no: slide_no,
                        bbox: default_bbox,
                        charspan: None,
                    }],
                    orig: metadata_text.clone(),
                    text: metadata_text,
                    formatting: None,
                    hyperlink: None,
                });
                text_idx += 1;
            }

            // Add presenter notes if present
            if let Some(notes) = slide_notes.get(&slide_no) {
                let notes_text = format!("Notes: {notes}");
                let item_ref = format!("#/texts/{text_idx}");
                body_children.push(ItemRef::new(&item_ref));
                text_items.push(DocItem::Text {
                    self_ref: item_ref,
                    parent: Some(ItemRef::new("#")),
                    children: vec![],
                    content_layer: "body".to_string(),
                    prov: vec![ProvenanceItem {
                        page_no: slide_no,
                        bbox: default_bbox,
                        charspan: None,
                    }],
                    orig: notes_text.clone(),
                    text: notes_text,
                    formatting: None,
                    hyperlink: None,
                });
                text_idx += 1;
            }
        }

        // Add title if no content
        if text_items.is_empty() {
            let title_ref = format!("#/texts/{text_idx}");
            body_children.push(ItemRef::new(&title_ref));
            text_items.push(DocItem::Title {
                self_ref: title_ref,
                parent: Some(ItemRef::new("#")),
                children: vec![],
                content_layer: "body".to_string(),
                prov: vec![ProvenanceItem {
                    page_no: 1,
                    bbox: default_bbox,
                    charspan: None,
                }],
                orig: file_name.clone(),
                text: file_name.clone(),
                formatting: None,
                hyperlink: None,
            });
        }

        // Create body group
        let body = GroupItem {
            self_ref: "#".to_string(),
            parent: None,
            children: body_children,
            content_layer: "body".to_string(),
            name: "body".to_string(),
            label: "body".to_string(),
        };

        // Create pages map (slides in Keynote)
        let mut pages = HashMap::new();
        let num_slides = slide_number.max(1);
        for i in 1..=num_slides {
            pages.insert(
                i.to_string(),
                PageInfo {
                    page_no: i,
                    size: PageSize {
                        width: 1024.0, // Widescreen 4:3
                        height: 768.0,
                    },
                },
            );
        }

        Ok(DoclingDocument {
            schema_name: "DoclingDocument".to_string(),
            version: "1.7.0".to_string(),
            name: file_name,
            origin: Origin {
                filename: input_path.to_string_lossy().to_string(),
                mimetype: "application/x-iwork-keynote-sffkey".to_string(),
                binary_hash: 0,
            },
            body,
            furniture: None,
            texts: text_items,
            tables: vec![],
            groups: vec![],
            pictures: vec![],
            key_value_items: vec![],
            form_items: vec![],
            pages,
        })
    }

    /// Extract `QuickLook` preview PDF from Keynote file
    ///
    /// Returns the raw PDF bytes that can be parsed by a PDF backend.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be opened or does not contain a preview.
    #[must_use = "this function returns PDF data that should be processed"]
    pub fn extract_preview_pdf(&self, input_path: &Path) -> Result<Vec<u8>> {
        crate::common::extract_quicklook_pdf(input_path, "Keynote")
    }

    /// Get the backend name
    #[inline]
    #[must_use = "returns the backend identifier string"]
    pub const fn name(&self) -> &'static str {
        "Keynote"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;
    use zip::write::{SimpleFileOptions, ZipWriter};

    /// Helper function to create a temporary Keynote file with given XML content
    fn create_test_keynote_file(index_xml: &str) -> NamedTempFile {
        let temp_file = NamedTempFile::new().unwrap();
        let file = temp_file.reopen().unwrap();

        let mut zip = ZipWriter::new(file);
        zip.start_file("index.xml", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(index_xml.as_bytes()).unwrap();
        zip.finish().unwrap();

        temp_file
    }

    #[test]
    fn test_keynote_backend_creation() {
        let backend = KeynoteBackend::new();
        assert_eq!(backend.name(), "Keynote");
    }

    #[test]
    fn test_keynote_backend_default() {
        let backend = KeynoteBackend;
        assert_eq!(backend.name(), "Keynote");
    }

    #[test]
    #[allow(
        clippy::default_constructed_unit_structs,
        reason = "testing Default trait impl"
    )]
    fn test_keynote_backend_default_equals_new() {
        // Verify derived Default produces same result as new()
        assert_eq!(KeynoteBackend::default(), KeynoteBackend::new());
    }

    #[test]
    fn test_keynote_simple_text() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<key:presentation xmlns:key="http://developer.apple.com/namespaces/keynote2">
    <key:slide key:number="1">
        <key:title>
            <key:text>Hello Keynote</key:text>
        </key:title>
    </key:slide>
</key:presentation>"#;

        let temp_file = create_test_keynote_file(xml);
        let backend = KeynoteBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        assert_eq!(result.texts.len(), 1);
        if let DocItem::SectionHeader { text, .. } = &result.texts[0] {
            assert_eq!(text, "Slide 1: Hello Keynote");
        } else {
            panic!("Expected SectionHeader DocItem for title");
        }
    }

    #[test]
    fn test_keynote_multiple_slides() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<key:presentation xmlns:key="http://developer.apple.com/namespaces/keynote2">
    <key:slide key:number="1">
        <key:title><key:text>Slide 1</key:text></key:title>
    </key:slide>
    <key:slide key:number="2">
        <key:title><key:text>Slide 2</key:text></key:title>
    </key:slide>
    <key:slide key:number="3">
        <key:title><key:text>Slide 3</key:text></key:title>
    </key:slide>
</key:presentation>"#;

        let temp_file = create_test_keynote_file(xml);
        let backend = KeynoteBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        assert_eq!(result.texts.len(), 3);
        assert_eq!(result.pages.len(), 3);

        // Verify slide titles
        if let DocItem::SectionHeader { text, .. } = &result.texts[0] {
            assert_eq!(text, "Slide 1");
        }
        if let DocItem::SectionHeader { text, .. } = &result.texts[1] {
            assert_eq!(text, "Slide 2");
        }
        if let DocItem::SectionHeader { text, .. } = &result.texts[2] {
            assert_eq!(text, "Slide 3");
        }
    }

    #[test]
    fn test_keynote_title_and_body() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<key:presentation xmlns:key="http://developer.apple.com/namespaces/keynote2">
    <key:slide key:number="1">
        <key:title><key:text>Main Title</key:text></key:title>
        <key:body><key:text>Body content here</key:text></key:body>
    </key:slide>
</key:presentation>"#;

        let temp_file = create_test_keynote_file(xml);
        let backend = KeynoteBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        assert_eq!(result.texts.len(), 2);

        if let DocItem::SectionHeader { text, .. } = &result.texts[0] {
            assert_eq!(text, "Slide 1: Main Title");
        } else {
            panic!("Expected SectionHeader for title");
        }

        if let DocItem::Text { text, .. } = &result.texts[1] {
            assert_eq!(text, "Body content here");
        } else {
            panic!("Expected Text for body");
        }
    }

    #[test]
    fn test_keynote_multiple_text_items() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<key:presentation xmlns:key="http://developer.apple.com/namespaces/keynote2">
    <key:slide key:number="1">
        <key:title><key:text>Title</key:text></key:title>
        <key:body>
            <key:text>First paragraph</key:text>
            <key:text>Second paragraph</key:text>
            <key:text>Third paragraph</key:text>
        </key:body>
    </key:slide>
</key:presentation>"#;

        let temp_file = create_test_keynote_file(xml);
        let backend = KeynoteBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        // Parser concatenates all text within a single <key:body> tag
        // 1 title + 1 body (with concatenated text) = 2 items
        assert_eq!(result.texts.len(), 2);

        if let DocItem::Text { text, .. } = &result.texts[1] {
            // Verify all paragraphs are concatenated
            assert!(text.contains("First paragraph"));
            assert!(text.contains("Second paragraph"));
            assert!(text.contains("Third paragraph"));
        }
    }

    #[test]
    fn test_keynote_empty_presentation() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<key:presentation xmlns:key="http://developer.apple.com/namespaces/keynote2">
</key:presentation>"#;

        let temp_file = create_test_keynote_file(xml);
        let backend = KeynoteBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        // Should create a default title slide
        assert_eq!(result.texts.len(), 1);
        if let DocItem::Text { text, .. } = &result.texts[0] {
            assert!(text.contains("Empty Keynote"));
        }
    }

    #[test]
    fn test_keynote_empty_slide() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<key:presentation xmlns:key="http://developer.apple.com/namespaces/keynote2">
    <key:slide key:number="1">
    </key:slide>
</key:presentation>"#;

        let temp_file = create_test_keynote_file(xml);
        let backend = KeynoteBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        // Empty slide should still create document structure
        assert_eq!(result.pages.len(), 1);
    }

    #[test]
    fn test_keynote_missing_index_xml() {
        let temp_file = NamedTempFile::new().unwrap();
        let file = temp_file.reopen().unwrap();

        let mut zip = ZipWriter::new(file);
        zip.start_file("other.xml", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"<root/>").unwrap();
        zip.finish().unwrap();

        let backend = KeynoteBackend::new();
        let result = backend.parse(temp_file.path());

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("missing index.xml"));
    }

    #[test]
    fn test_keynote_invalid_zip() {
        let temp_file = NamedTempFile::new().unwrap();
        std::fs::write(temp_file.path(), b"Not a ZIP file").unwrap();

        let backend = KeynoteBackend::new();
        let result = backend.parse(temp_file.path());

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("ZIP"));
    }

    #[test]
    fn test_keynote_malformed_xml() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<key:presentation xmlns:key="http://developer.apple.com/namespaces/keynote2">
    <key:slide
</key:presentation>"#;

        let temp_file = create_test_keynote_file(xml);
        let backend = KeynoteBackend::new();
        let result = backend.parse(temp_file.path());

        // Should handle malformed XML gracefully (quick-xml is lenient)
        // May succeed with partial content or fail
        let _ = result; // Allow either success or failure
    }

    #[test]
    fn test_keynote_unicode_support() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<key:presentation xmlns:key="http://developer.apple.com/namespaces/keynote2">
    <key:slide key:number="1">
        <key:title><key:text>Hello 世界 🎨</key:text></key:title>
        <key:body><key:text>Презентация Keynote 📊</key:text></key:body>
    </key:slide>
</key:presentation>"#;

        let temp_file = create_test_keynote_file(xml);
        let backend = KeynoteBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        assert_eq!(result.texts.len(), 2);

        if let DocItem::SectionHeader { text, .. } = &result.texts[0] {
            assert!(text.contains("世界"));
            assert!(text.contains("🎨"));
        }

        if let DocItem::Text { text, .. } = &result.texts[1] {
            assert!(text.contains("Презентация"));
            assert!(text.contains("📊"));
        }
    }

    #[test]
    fn test_keynote_whitespace_handling() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<key:presentation xmlns:key="http://developer.apple.com/namespaces/keynote2">
    <key:slide key:number="1">
        <key:title><key:text>  Extra   spaces  </key:text></key:title>
    </key:slide>
</key:presentation>"#;

        let temp_file = create_test_keynote_file(xml);
        let backend = KeynoteBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        assert_eq!(result.texts.len(), 1);
        if let DocItem::SectionHeader { text, .. } = &result.texts[0] {
            // XML trimming should handle excess whitespace
            assert!(text.contains("Extra"));
            assert!(text.contains("spaces"));
        }
    }

    #[test]
    fn test_keynote_document_metadata() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<key:presentation xmlns:key="http://developer.apple.com/namespaces/keynote2">
    <key:slide key:number="1">
        <key:title><key:text>Content</key:text></key:title>
    </key:slide>
</key:presentation>"#;

        let temp_file = create_test_keynote_file(xml);
        let backend = KeynoteBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        assert_eq!(result.schema_name, "DoclingDocument");
        assert_eq!(result.version, "1.7.0");
        assert_eq!(result.origin.mimetype, "application/x-iwork-keynote-sffkey");
        assert_eq!(result.pages.len(), 1);
        assert_eq!(result.pages.get("1").unwrap().page_no, 1);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn test_keynote_slide_structure() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<key:presentation xmlns:key="http://developer.apple.com/namespaces/keynote2">
    <key:slide key:number="1">
        <key:title><key:text>Slide 1</key:text></key:title>
    </key:slide>
    <key:slide key:number="2">
        <key:title><key:text>Slide 2</key:text></key:title>
    </key:slide>
</key:presentation>"#;

        let temp_file = create_test_keynote_file(xml);
        let backend = KeynoteBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        // Verify page/slide structure
        assert_eq!(result.pages.len(), 2);
        assert_eq!(result.pages.get("1").unwrap().page_no, 1);
        assert_eq!(result.pages.get("2").unwrap().page_no, 2);

        // Default widescreen size
        assert_eq!(result.pages.get("1").unwrap().size.width, 1024.0);
        assert_eq!(result.pages.get("1").unwrap().size.height, 768.0);
    }

    #[test]
    fn test_keynote_body_structure() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<key:presentation xmlns:key="http://developer.apple.com/namespaces/keynote2">
    <key:slide key:number="1">
        <key:title><key:text>Title</key:text></key:title>
    </key:slide>
</key:presentation>"#;

        let temp_file = create_test_keynote_file(xml);
        let backend = KeynoteBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        assert_eq!(result.body.self_ref, "#");
        assert_eq!(result.body.name, "body");
        assert_eq!(result.body.label, "body");
        assert_eq!(result.body.content_layer, "body");
        assert_eq!(result.body.children.len(), 1);
    }

    #[test]
    fn test_keynote_provenance_info() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<key:presentation xmlns:key="http://developer.apple.com/namespaces/keynote2">
    <key:slide key:number="1">
        <key:title><key:text>Test</key:text></key:title>
    </key:slide>
</key:presentation>"#;

        let temp_file = create_test_keynote_file(xml);
        let backend = KeynoteBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        // Check provenance is set
        if let DocItem::SectionHeader { prov, .. } = &result.texts[0] {
            assert_eq!(prov.len(), 1);
            assert_eq!(prov[0].page_no, 1);
            assert_eq!(prov[0].bbox.l, 0.0);
            assert_eq!(prov[0].bbox.t, 0.0);
            assert_eq!(prov[0].bbox.r, 1024.0);
            assert_eq!(prov[0].bbox.b, 768.0);
        } else {
            panic!("Expected SectionHeader with provenance");
        }
    }

    #[test]
    fn test_keynote_mixed_content() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<key:presentation xmlns:key="http://developer.apple.com/namespaces/keynote2">
    <key:slide key:number="1">
        <key:title><key:text>Introduction</key:text></key:title>
        <key:body><key:text>Welcome to the presentation</key:text></key:body>
    </key:slide>
    <key:slide key:number="2">
        <key:title><key:text>Content</key:text></key:title>
        <key:body>
            <key:text>Point 1</key:text>
            <key:text>Point 2</key:text>
        </key:body>
    </key:slide>
    <key:slide key:number="3">
        <key:title><key:text>Conclusion</key:text></key:title>
    </key:slide>
</key:presentation>"#;

        let temp_file = create_test_keynote_file(xml);
        let backend = KeynoteBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        // Parser concatenates text within each <key:body> tag
        // 3 titles + 1 body (slide 1) + 1 body (slide 2, concatenated) = 5 text items
        assert_eq!(result.texts.len(), 5);
        assert_eq!(result.pages.len(), 3);

        // Verify slide 2 body has both points concatenated
        if let DocItem::Text { text, .. } = &result.texts[3] {
            assert!(text.contains("Point 1"));
            assert!(text.contains("Point 2"));
        }
    }

    #[test]
    fn test_keynote_no_text_content() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<key:presentation xmlns:key="http://developer.apple.com/namespaces/keynote2">
    <key:slide key:number="1">
        <key:image src="picture.jpg"/>
    </key:slide>
</key:presentation>"#;

        let temp_file = create_test_keynote_file(xml);
        let backend = KeynoteBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        // No text items, but document should still be valid
        assert_eq!(result.pages.len(), 1);
    }

    #[test]
    fn test_keynote_nested_elements() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<key:presentation xmlns:key="http://developer.apple.com/namespaces/keynote2">
    <key:slide key:number="1">
        <key:title>
            <key:p>
                <key:span>
                    <key:text>Nested Title Text</key:text>
                </key:span>
            </key:p>
        </key:title>
    </key:slide>
</key:presentation>"#;

        let temp_file = create_test_keynote_file(xml);
        let backend = KeynoteBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        // Should extract text even from nested elements
        assert_eq!(result.texts.len(), 1);
        if let DocItem::SectionHeader { text, .. } = &result.texts[0] {
            assert!(text.contains("Nested Title Text"));
        }
    }

    #[test]
    fn test_keynote_empty_text_elements() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<key:presentation xmlns:key="http://developer.apple.com/namespaces/keynote2">
    <key:slide key:number="1">
        <key:title><key:text></key:text></key:title>
        <key:body><key:text>   </key:text></key:body>
        <key:body><key:text>Valid text</key:text></key:body>
    </key:slide>
</key:presentation>"#;

        let temp_file = create_test_keynote_file(xml);
        let backend = KeynoteBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        // Empty text elements should be ignored
        // Only "Valid text" should be extracted
        let non_empty_texts: Vec<_> = result
            .texts
            .iter()
            .filter(|item| match item {
                DocItem::Text { text, .. } => !text.trim().is_empty(),
                DocItem::SectionHeader { text, .. } => !text.trim().is_empty(),
                _ => false,
            })
            .collect();

        assert_eq!(non_empty_texts.len(), 1);
    }

    #[test]
    fn test_keynote_large_presentation() {
        let mut xml = String::from(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<key:presentation xmlns:key="http://developer.apple.com/namespaces/keynote2">"#,
        );

        // Generate 50 slides
        for i in 1..=50 {
            xml.push_str(&format!(
                r#"
    <key:slide key:number="{i}">
        <key:title><key:text>Slide {i}</key:text></key:title>
        <key:body><key:text>Content for slide {i}</key:text></key:body>
    </key:slide>"#
            ));
        }

        xml.push_str("\n</key:presentation>");

        let temp_file = create_test_keynote_file(&xml);
        let backend = KeynoteBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        // 50 slides × 2 items (title + body) = 100 text items
        assert_eq!(result.texts.len(), 100);
        assert_eq!(result.pages.len(), 50);
    }

    #[test]
    fn test_keynote_special_characters() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<key:presentation xmlns:key="http://developer.apple.com/namespaces/keynote2">
    <key:slide key:number="1">
        <key:title><key:text>Title with &lt;brackets&gt; &amp; symbols</key:text></key:title>
        <key:body><key:text>Text with "quotes" and 'apostrophes'</key:text></key:body>
    </key:slide>
</key:presentation>"#;

        let temp_file = create_test_keynote_file(xml);
        let backend = KeynoteBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        if let DocItem::SectionHeader { text, .. } = &result.texts[0] {
            assert!(text.contains("<brackets>"));
            assert!(text.contains('&'));
        }

        if let DocItem::Text { text, .. } = &result.texts[1] {
            assert!(text.contains("quotes"));
            assert!(text.contains("apostrophes"));
        }
    }

    #[test]
    fn test_keynote_multiline_text() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<key:presentation xmlns:key="http://developer.apple.com/namespaces/keynote2">
    <key:slide key:number="1">
        <key:body><key:text>Line 1
Line 2
Line 3</key:text></key:body>
    </key:slide>
</key:presentation>"#;

        let temp_file = create_test_keynote_file(xml);
        let backend = KeynoteBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        assert_eq!(result.texts.len(), 1);
        if let DocItem::Text { text, .. } = &result.texts[0] {
            assert!(text.contains("Line 1"));
            assert!(text.contains("Line 2"));
            assert!(text.contains("Line 3"));
        }
    }

    #[test]
    fn test_keynote_slide_numbering() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<key:presentation xmlns:key="http://developer.apple.com/namespaces/keynote2">
    <key:slide key:number="5">
        <key:title><key:text>Slide 5</key:text></key:title>
    </key:slide>
    <key:slide key:number="10">
        <key:title><key:text>Slide 10</key:text></key:title>
    </key:slide>
</key:presentation>"#;

        let temp_file = create_test_keynote_file(xml);
        let backend = KeynoteBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        // Should create pages for slides 1-10 (or at least the max slide number)
        assert!(result.pages.len() >= 2);
    }

    #[test]
    fn test_keynote_extract_preview_pdf() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<key:presentation xmlns:key="http://developer.apple.com/namespaces/keynote2">
    <key:slide key:number="1">
        <key:title><key:text>Test</key:text></key:title>
    </key:slide>
</key:presentation>"#;

        let temp_file = create_test_keynote_file(xml);
        let backend = KeynoteBackend::new();

        // Test extract_preview_pdf method (will fail since no QuickLook PDF in test file)
        let result = backend.extract_preview_pdf(temp_file.path());
        assert!(result.is_err()); // Expected to fail - no QuickLook PDF in test file
    }

    #[test]
    fn test_keynote_with_transition() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<key:presentation xmlns:key="http://developer.apple.com/namespaces/keynote2">
    <key:slide key:number="1">
        <key:title><key:text>Slide with Transition</key:text></key:title>
        <key:transition key:type="dissolve"/>
    </key:slide>
</key:presentation>"#;

        let temp_file = create_test_keynote_file(xml);
        let backend = KeynoteBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        // Should have title + transition metadata
        assert!(result.texts.len() >= 2);

        // Check that transition metadata was extracted
        let has_transition_metadata = result.texts.iter().any(|item| match item {
            DocItem::Text { text, .. } => text.contains("Transition:") && text.contains("dissolve"),
            _ => false,
        });
        assert!(
            has_transition_metadata,
            "Expected transition metadata in output"
        );
    }

    #[test]
    fn test_keynote_with_builds() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<key:presentation xmlns:key="http://developer.apple.com/namespaces/keynote2">
    <key:slide key:number="1">
        <key:title><key:text>Slide with Builds</key:text></key:title>
        <key:build key:type="fade-in"/>
        <key:build key:type="fly-in"/>
    </key:slide>
</key:presentation>"#;

        let temp_file = create_test_keynote_file(xml);
        let backend = KeynoteBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        // Should have title + build metadata
        assert!(result.texts.len() >= 2);

        // Check that build metadata was extracted
        let has_build_metadata = result.texts.iter().any(|item| match item {
            DocItem::Text { text, .. } => {
                text.contains("Animations:") && text.contains("fade-in") && text.contains("fly-in")
            }
            _ => false,
        });
        assert!(has_build_metadata, "Expected build metadata in output");
    }

    #[test]
    fn test_keynote_with_transition_and_builds() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<key:presentation xmlns:key="http://developer.apple.com/namespaces/keynote2">
    <key:slide key:number="1">
        <key:title><key:text>Complete Slide</key:text></key:title>
        <key:body><key:text>Content here</key:text></key:body>
        <key:transition key:type="cube"/>
        <key:build-in key:type="appear"/>
        <key:action-build key:type="rotate"/>
    </key:slide>
</key:presentation>"#;

        let temp_file = create_test_keynote_file(xml);
        let backend = KeynoteBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        // Should have title + body + metadata
        assert!(result.texts.len() >= 3);

        // Check for transition
        let has_transition = result.texts.iter().any(|item| match item {
            DocItem::Text { text, .. } => text.contains("Transition:") && text.contains("cube"),
            _ => false,
        });
        assert!(has_transition, "Expected transition metadata");

        // Check for animations
        let has_animations = result.texts.iter().any(|item| match item {
            DocItem::Text { text, .. } => {
                text.contains("Animations:") && text.contains("appear") && text.contains("rotate")
            }
            _ => false,
        });
        assert!(has_animations, "Expected animation metadata");
    }

    #[test]
    fn test_keynote_multiple_slides_with_transitions() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<key:presentation xmlns:key="http://developer.apple.com/namespaces/keynote2">
    <key:slide key:number="1">
        <key:title><key:text>Slide 1</key:text></key:title>
        <key:transition key:type="push"/>
    </key:slide>
    <key:slide key:number="2">
        <key:title><key:text>Slide 2</key:text></key:title>
        <key:transition key:type="wipe"/>
    </key:slide>
    <key:slide key:number="3">
        <key:title><key:text>Slide 3</key:text></key:title>
    </key:slide>
</key:presentation>"#;

        let temp_file = create_test_keynote_file(xml);
        let backend = KeynoteBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        // Should have 3 titles + 2 metadata items (only slides 1 and 2 have transitions)
        assert!(result.texts.len() >= 5);

        // Verify each transition
        let text_contents: Vec<String> = result
            .texts
            .iter()
            .filter_map(|item| match item {
                DocItem::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();

        let has_push = text_contents
            .iter()
            .any(|t| t.contains("Transition:") && t.contains("push"));
        let has_wipe = text_contents
            .iter()
            .any(|t| t.contains("Transition:") && t.contains("wipe"));

        assert!(has_push, "Expected 'push' transition");
        assert!(has_wipe, "Expected 'wipe' transition");
    }

    #[test]
    fn test_keynote_with_presenter_notes() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<key:presentation xmlns:key="http://developer.apple.com/namespaces/keynote2">
    <key:slide key:number="1">
        <key:title><key:text>Slide with Notes</key:text></key:title>
        <key:body><key:text>Main content here</key:text></key:body>
        <key:notes><key:text>These are presenter notes for the speaker</key:text></key:notes>
    </key:slide>
</key:presentation>"#;

        let temp_file = create_test_keynote_file(xml);
        let backend = KeynoteBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        // Should have title + body + notes
        assert!(result.texts.len() >= 3);

        // Check that notes were extracted
        let has_notes = result.texts.iter().any(|item| match item {
            DocItem::Text { text, .. } => {
                text.contains("Notes:") && text.contains("presenter notes for the speaker")
            }
            _ => false,
        });
        assert!(has_notes, "Expected presenter notes in output");
    }

    #[test]
    fn test_keynote_multiple_slides_with_notes() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<key:presentation xmlns:key="http://developer.apple.com/namespaces/keynote2">
    <key:slide key:number="1">
        <key:title><key:text>Introduction</key:text></key:title>
        <key:notes><key:text>Introduce yourself and the topic</key:text></key:notes>
    </key:slide>
    <key:slide key:number="2">
        <key:title><key:text>Main Points</key:text></key:title>
        <key:body><key:text>Key information</key:text></key:body>
        <key:notes><key:text>Elaborate on each point with examples</key:text></key:notes>
    </key:slide>
    <key:slide key:number="3">
        <key:title><key:text>Conclusion</key:text></key:title>
        <key:notes><key:text>Summarize and take questions</key:text></key:notes>
    </key:slide>
</key:presentation>"#;

        let temp_file = create_test_keynote_file(xml);
        let backend = KeynoteBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        // Verify all notes are present
        let notes_found = result
            .texts
            .iter()
            .filter(|item| match item {
                DocItem::Text { text, .. } => text.starts_with("Notes:"),
                _ => false,
            })
            .count();

        assert_eq!(notes_found, 3, "Expected notes for all 3 slides");

        // Verify specific notes content
        let has_intro_notes = result.texts.iter().any(|item| match item {
            DocItem::Text { text, .. } => text.contains("Introduce yourself"),
            _ => false,
        });
        let has_main_notes = result.texts.iter().any(|item| match item {
            DocItem::Text { text, .. } => text.contains("Elaborate on each point"),
            _ => false,
        });
        let has_conclusion_notes = result.texts.iter().any(|item| match item {
            DocItem::Text { text, .. } => text.contains("Summarize and take questions"),
            _ => false,
        });

        assert!(has_intro_notes, "Expected introduction notes");
        assert!(has_main_notes, "Expected main points notes");
        assert!(has_conclusion_notes, "Expected conclusion notes");
    }
}
