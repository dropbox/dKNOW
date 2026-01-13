//! Apple Pages format support
//!
//! Pages files (.pages) are ZIP archives containing XML files.
//!
//! Structure:
//! - index.xml - Main document structure (Pages '09 XML format)
//! - QuickLook/Thumbnail.jpg - Thumbnail preview
//! - Data/ - Embedded media files
//!
//! NOTE: This implementation supports Pages '09 XML format.
//! Modern Pages '13+ files use IWA (iWork Archive) format which is protobuf-based.

use anyhow::{Context, Result};
use docling_core::{
    content::{
        CoordOrigin, DocItem, ItemRef, ProvenanceItem, TableCell, TableData, US_LETTER_HEIGHT,
        US_LETTER_WIDTH,
    },
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

/// Backend for Apple Pages files
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct PagesBackend;

impl PagesBackend {
    /// Create a new Pages backend instance
    #[inline]
    #[must_use = "creates a backend instance that should be used for parsing"]
    pub const fn new() -> Self {
        Self
    }

    /// Parse Pages file and generate `DocItems` directly
    ///
    /// Parses Pages '09 XML format and creates `DoclingDocument` structure
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be opened (I/O error), if the content is not
    /// a valid ZIP archive, if index.xml is missing, or if the XML is malformed.
    #[must_use = "this function returns a parsed document that should be processed"]
    pub fn parse(&self, input_path: &Path) -> Result<DoclingDocument> {
        // Extract index.xml from ZIP archive
        let xml_content = Self::extract_index_xml(input_path)?;

        // Parse XML and build DocItems
        Self::parse_xml(&xml_content, input_path)
    }

    /// Extract index.xml from Pages ZIP archive
    fn extract_index_xml(input_path: &Path) -> Result<String> {
        let file = File::open(input_path)
            .with_context(|| format!("Failed to open Pages file: {}", input_path.display()))?;

        let mut archive =
            ZipArchive::new(file).context("Failed to read Pages file as ZIP archive")?;

        // Find index.xml
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            if file.name() == "index.xml" {
                let mut content = String::new();
                file.read_to_string(&mut content)?;
                return Ok(content);
            }
        }

        anyhow::bail!("Invalid Pages file: missing index.xml")
    }

    /// Parse Pages XML and generate `DocItems`
    #[allow(clippy::too_many_lines)] // Complex iWork XML parsing - keeping together for clarity
    fn parse_xml(xml_content: &str, input_path: &Path) -> Result<DoclingDocument> {
        let file_name = input_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown.pages")
            .to_string();

        let mut reader = Reader::from_str(xml_content);
        reader.trim_text(true);

        let mut text_items = Vec::new();
        let mut table_items = Vec::new();
        let mut body_children = Vec::new();
        let mut text_idx = 0;
        let mut table_idx = 0;

        // Default bounding box (US Letter page)
        let default_bbox = BoundingBox::new(
            0.0,
            0.0,
            US_LETTER_WIDTH,
            US_LETTER_HEIGHT,
            CoordOrigin::BottomLeft,
        );

        let mut buf = Vec::new();
        let mut current_text = String::new();
        let mut in_text_storage = false;
        let mut in_paragraph = false;
        let mut in_table = false;
        let mut table_rows: Vec<Vec<String>> = Vec::new();
        let mut current_row: Vec<String> = Vec::new();
        let mut current_cell = String::new();
        let mut paragraph_style = String::new();
        let mut document_title = String::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => {
                    match e.name().as_ref() {
                        b"sl:title" => {
                            // Read document title
                            if let Ok(Event::Text(txt)) = reader.read_event_into(&mut buf) {
                                document_title = txt.unescape().unwrap_or_default().to_string();
                            }
                        }
                        b"sf:text-storage" => {
                            in_text_storage = true;
                        }
                        b"sf:p" => {
                            in_paragraph = true;
                            current_text.clear();
                            // Check for style attribute
                            paragraph_style = e
                                .attributes()
                                .filter_map(std::result::Result::ok)
                                .find(|attr| attr.key.as_ref() == b"sf:style")
                                .and_then(|attr| String::from_utf8(attr.value.to_vec()).ok())
                                .unwrap_or_default();
                        }
                        b"sf:text" => {
                            // Simple text element
                            if let Ok(Event::Text(txt)) = reader.read_event_into(&mut buf) {
                                current_text = txt.unescape().unwrap_or_default().to_string();
                            }
                        }
                        b"sf:table" => {
                            in_table = true;
                            table_rows.clear();
                        }
                        b"sf:row" => {
                            if in_table {
                                current_row.clear();
                            }
                        }
                        b"sf:cell" => {
                            current_cell.clear();
                        }
                        b"sf:list-item" => {
                            // List items are treated as text paragraphs
                            in_paragraph = true;
                            current_text.clear();
                        }
                        _ => {}
                    }
                }
                Ok(Event::End(ref e)) => {
                    match e.name().as_ref() {
                        b"sf:text-storage" => {
                            // If we have text that wasn't part of a paragraph, save it now
                            if in_text_storage && !in_paragraph && !current_text.is_empty() {
                                let item_ref = format!("#/texts/{text_idx}");
                                body_children.push(ItemRef::new(&item_ref));

                                let doc_item = DocItem::Paragraph {
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
                                };

                                text_items.push(doc_item);
                                text_idx += 1;
                                current_text.clear();
                            }
                            in_text_storage = false;
                        }
                        b"sf:p" => {
                            if in_paragraph && !current_text.is_empty() {
                                // Create DocItem based on style
                                let item_ref = format!("#/texts/{text_idx}");
                                body_children.push(ItemRef::new(&item_ref));

                                let doc_item = if paragraph_style.contains("heading1") {
                                    DocItem::SectionHeader {
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
                                        level: 1,
                                        formatting: None,
                                        hyperlink: None,
                                    }
                                } else if paragraph_style.contains("heading2") {
                                    DocItem::SectionHeader {
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
                                        level: 2,
                                        formatting: None,
                                        hyperlink: None,
                                    }
                                } else if paragraph_style.contains("heading") {
                                    DocItem::SectionHeader {
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
                                        level: 3,
                                        formatting: None,
                                        hyperlink: None,
                                    }
                                } else {
                                    DocItem::Text {
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
                                };

                                text_items.push(doc_item);
                                text_idx += 1;
                                current_text.clear();
                            }
                            in_paragraph = false;
                            paragraph_style.clear();
                        }
                        b"sf:cell" => {
                            if in_table {
                                current_row.push(current_cell.clone());
                                current_cell.clear();
                            }
                        }
                        b"sf:row" => {
                            if in_table && !current_row.is_empty() {
                                table_rows.push(current_row.clone());
                                current_row.clear();
                            }
                        }
                        b"sf:table" => {
                            if in_table && !table_rows.is_empty() {
                                // Create table DocItem
                                let mut grid = Vec::new();
                                for row in &table_rows {
                                    let mut grid_row = Vec::new();
                                    for cell_text in row {
                                        grid_row.push(TableCell {
                                            text: cell_text.clone(),
                                            row_span: Some(1),
                                            col_span: Some(1),
                                            ref_item: None,
                                            start_row_offset_idx: None,
                                            start_col_offset_idx: None,
                                            column_header: false,
                                            row_header: false,
                                            from_ocr: false,
                                            bbox: None,
                                            confidence: None,
                                        });
                                    }
                                    grid.push(grid_row);
                                }

                                let table_data = TableData {
                                    num_rows: table_rows.len(),
                                    num_cols: table_rows.first().map_or(0, std::vec::Vec::len),
                                    grid,
                                    table_cells: None,
                                };

                                let table_ref = format!("#/tables/{table_idx}");
                                body_children.push(ItemRef::new(&table_ref));
                                table_items.push(DocItem::Table {
                                    self_ref: table_ref,
                                    parent: Some(ItemRef::new("#")),
                                    children: vec![],
                                    content_layer: "body".to_string(),
                                    prov: vec![ProvenanceItem {
                                        page_no: 1,
                                        bbox: default_bbox,
                                        charspan: None,
                                    }],
                                    data: table_data,
                                    captions: vec![],
                                    footnotes: vec![],
                                    references: vec![],
                                    image: None,
                                    annotations: vec![],
                                });
                                table_idx += 1;
                                table_rows.clear();
                            }
                            in_table = false;
                        }
                        b"sf:list-item" => {
                            if in_paragraph && !current_text.is_empty() {
                                // Create list item as text
                                let item_ref = format!("#/texts/{text_idx}");
                                body_children.push(ItemRef::new(&item_ref));
                                text_items.push(DocItem::Text {
                                    self_ref: item_ref,
                                    parent: Some(ItemRef::new("#")),
                                    children: vec![],
                                    content_layer: "body".to_string(),
                                    prov: vec![ProvenanceItem {
                                        page_no: 1,
                                        bbox: default_bbox,
                                        charspan: None,
                                    }],
                                    orig: format!("• {current_text}"),
                                    text: format!("• {current_text}"),
                                    formatting: None,
                                    hyperlink: None,
                                });
                                text_idx += 1;
                                current_text.clear();
                            }
                            in_paragraph = false;
                        }
                        _ => {}
                    }
                }
                Ok(Event::Text(e)) => {
                    let text = e.unescape().unwrap_or_default().trim().to_string();
                    if !text.is_empty() {
                        if in_paragraph || in_text_storage {
                            if !current_text.is_empty() {
                                current_text.push(' ');
                            }
                            current_text.push_str(&text);
                        } else if in_table {
                            current_cell.push_str(&text);
                        }
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

        // Add document title if found
        if !document_title.is_empty() {
            let title_ref = format!("#/texts/{text_idx}");
            // Insert title at the beginning
            body_children.insert(0, ItemRef::new(&title_ref));
            text_items.insert(
                0,
                DocItem::Title {
                    self_ref: title_ref,
                    parent: Some(ItemRef::new("#")),
                    children: vec![],
                    content_layer: "body".to_string(),
                    prov: vec![ProvenanceItem {
                        page_no: 1,
                        bbox: default_bbox,
                        charspan: None,
                    }],
                    orig: document_title.clone(),
                    text: document_title,
                    formatting: None,
                    hyperlink: None,
                },
            );
        }

        // Add simple text if found at document level
        if !current_text.is_empty() && !in_paragraph {
            let text_ref = format!("#/texts/{text_idx}");
            body_children.push(ItemRef::new(&text_ref));
            text_items.push(DocItem::Text {
                self_ref: text_ref,
                parent: Some(ItemRef::new("#")),
                children: vec![],
                content_layer: "body".to_string(),
                prov: vec![ProvenanceItem {
                    page_no: 1,
                    bbox: default_bbox,
                    charspan: None,
                }],
                orig: current_text.clone(),
                text: current_text,
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

        // Create pages map
        let mut pages = HashMap::new();
        pages.insert(
            "1".to_string(),
            PageInfo {
                page_no: 1,
                size: PageSize {
                    width: US_LETTER_WIDTH,
                    height: US_LETTER_HEIGHT,
                },
            },
        );

        Ok(DoclingDocument {
            schema_name: "DoclingDocument".to_string(),
            version: "1.7.0".to_string(),
            name: file_name,
            origin: Origin {
                filename: input_path.to_string_lossy().to_string(),
                mimetype: "application/x-iwork-pages-sffpages".to_string(),
                binary_hash: 0,
            },
            body,
            furniture: None,
            texts: text_items,
            tables: table_items,
            groups: vec![],
            pictures: vec![],
            key_value_items: vec![],
            form_items: vec![],
            pages,
        })
    }

    /// Extract `QuickLook` preview PDF from Pages file
    ///
    /// Returns the raw PDF bytes that can be parsed by a PDF backend.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be opened (I/O error), if the content is not
    /// a valid ZIP archive, or if `QuickLook/Preview.pdf` is missing from the archive.
    #[must_use = "this function returns PDF data that should be processed"]
    pub fn extract_preview_pdf(&self, input_path: &Path) -> Result<Vec<u8>> {
        // Open ZIP archive
        let file = File::open(input_path)
            .with_context(|| format!("Failed to open Pages file: {}", input_path.display()))?;

        let mut archive =
            ZipArchive::new(file).context("Failed to read Pages file as ZIP archive")?;

        // Check for QuickLook/Preview.pdf (all Pages files have this)
        let has_preview = (0..archive.len()).any(|i| {
            archive
                .by_index(i)
                .is_ok_and(|file| file.name() == "QuickLook/Preview.pdf")
        });

        if !has_preview {
            anyhow::bail!("Invalid Pages file: missing QuickLook/Preview.pdf");
        }

        // Extract Preview.pdf
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            if file.name() == "QuickLook/Preview.pdf" {
                let mut pdf_content = Vec::new();
                file.read_to_end(&mut pdf_content)?;
                return Ok(pdf_content);
            }
        }

        anyhow::bail!("Failed to extract preview PDF from Pages file")
    }

    /// Get the backend name
    #[inline]
    #[must_use = "returns the backend identifier string"]
    pub const fn name(&self) -> &'static str {
        "Pages"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    /// Helper to create a minimal Pages file for testing
    fn create_test_pages_file(index_xml: &str) -> NamedTempFile {
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
    fn test_pages_backend_creation() {
        let backend = PagesBackend::new();
        assert_eq!(backend.name(), "Pages");
    }

    #[test]
    fn test_pages_backend_default() {
        let backend = PagesBackend;
        assert_eq!(backend.name(), "Pages");
    }

    #[test]
    #[allow(
        clippy::default_constructed_unit_structs,
        reason = "testing Default trait impl"
    )]
    fn test_pages_backend_default_equals_new() {
        // Verify derived Default produces same result as new()
        assert_eq!(PagesBackend::default(), PagesBackend::new());
    }

    #[test]
    fn test_pages_simple_text() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<sl:document xmlns:sl="http://developer.apple.com/namespaces/sl" xmlns:sf="http://developer.apple.com/namespaces/sf">
    <sf:text-storage>
        <sf:p><sf:text>Hello, World!</sf:text></sf:p>
    </sf:text-storage>
</sl:document>"#;

        let temp_file = create_test_pages_file(xml);
        let backend = PagesBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        assert_eq!(result.texts.len(), 1);
        if let DocItem::Text { text, .. } = &result.texts[0] {
            assert_eq!(text, "Hello, World!");
        } else {
            panic!("Expected Text DocItem");
        }
    }

    #[test]
    fn test_pages_multiple_paragraphs() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<sl:document xmlns:sl="http://developer.apple.com/namespaces/sl" xmlns:sf="http://developer.apple.com/namespaces/sf">
    <sf:text-storage>
        <sf:p><sf:text>First paragraph</sf:text></sf:p>
        <sf:p><sf:text>Second paragraph</sf:text></sf:p>
        <sf:p><sf:text>Third paragraph</sf:text></sf:p>
    </sf:text-storage>
</sl:document>"#;

        let temp_file = create_test_pages_file(xml);
        let backend = PagesBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        assert_eq!(result.texts.len(), 3);
        for (i, expected) in ["First paragraph", "Second paragraph", "Third paragraph"]
            .iter()
            .enumerate()
        {
            if let DocItem::Text { text, .. } = &result.texts[i] {
                assert_eq!(text, expected);
            } else {
                panic!("Expected Text DocItem at index {i}");
            }
        }
    }

    #[test]
    fn test_pages_heading1() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<sl:document xmlns:sl="http://developer.apple.com/namespaces/sl" xmlns:sf="http://developer.apple.com/namespaces/sf">
    <sf:text-storage>
        <sf:p sf:style="heading1"><sf:text>Main Heading</sf:text></sf:p>
        <sf:p><sf:text>Body text</sf:text></sf:p>
    </sf:text-storage>
</sl:document>"#;

        let temp_file = create_test_pages_file(xml);
        let backend = PagesBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        assert_eq!(result.texts.len(), 2);
        if let DocItem::SectionHeader { text, level, .. } = &result.texts[0] {
            assert_eq!(text, "Main Heading");
            assert_eq!(*level, 1);
        } else {
            panic!("Expected SectionHeader DocItem");
        }
    }

    #[test]
    fn test_pages_heading2() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<sl:document xmlns:sl="http://developer.apple.com/namespaces/sl" xmlns:sf="http://developer.apple.com/namespaces/sf">
    <sf:text-storage>
        <sf:p sf:style="heading2"><sf:text>Subheading</sf:text></sf:p>
    </sf:text-storage>
</sl:document>"#;

        let temp_file = create_test_pages_file(xml);
        let backend = PagesBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        assert_eq!(result.texts.len(), 1);
        if let DocItem::SectionHeader { text, level, .. } = &result.texts[0] {
            assert_eq!(text, "Subheading");
            assert_eq!(*level, 2);
        } else {
            panic!("Expected SectionHeader DocItem");
        }
    }

    #[test]
    fn test_pages_generic_heading() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<sl:document xmlns:sl="http://developer.apple.com/namespaces/sl" xmlns:sf="http://developer.apple.com/namespaces/sf">
    <sf:text-storage>
        <sf:p sf:style="heading"><sf:text>Generic Heading</sf:text></sf:p>
    </sf:text-storage>
</sl:document>"#;

        let temp_file = create_test_pages_file(xml);
        let backend = PagesBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        assert_eq!(result.texts.len(), 1);
        if let DocItem::SectionHeader { text, level, .. } = &result.texts[0] {
            assert_eq!(text, "Generic Heading");
            assert_eq!(*level, 3);
        } else {
            panic!("Expected SectionHeader DocItem");
        }
    }

    #[test]
    fn test_pages_list_items() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<sl:document xmlns:sl="http://developer.apple.com/namespaces/sl" xmlns:sf="http://developer.apple.com/namespaces/sf">
    <sf:text-storage>
        <sf:list-item><sf:text>First item</sf:text></sf:list-item>
        <sf:list-item><sf:text>Second item</sf:text></sf:list-item>
    </sf:text-storage>
</sl:document>"#;

        let temp_file = create_test_pages_file(xml);
        let backend = PagesBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        assert_eq!(result.texts.len(), 2);
        if let DocItem::Text { text, .. } = &result.texts[0] {
            assert_eq!(text, "• First item");
        } else {
            panic!("Expected Text DocItem for list item");
        }
        if let DocItem::Text { text, .. } = &result.texts[1] {
            assert_eq!(text, "• Second item");
        } else {
            panic!("Expected Text DocItem for list item");
        }
    }

    #[test]
    fn test_pages_simple_table() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<sl:document xmlns:sl="http://developer.apple.com/namespaces/sl" xmlns:sf="http://developer.apple.com/namespaces/sf">
    <sf:table>
        <sf:row>
            <sf:cell>A1</sf:cell>
            <sf:cell>B1</sf:cell>
        </sf:row>
        <sf:row>
            <sf:cell>A2</sf:cell>
            <sf:cell>B2</sf:cell>
        </sf:row>
    </sf:table>
</sl:document>"#;

        let temp_file = create_test_pages_file(xml);
        let backend = PagesBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        assert_eq!(result.tables.len(), 1);
        if let DocItem::Table { data, .. } = &result.tables[0] {
            assert_eq!(data.num_rows, 2);
            assert_eq!(data.num_cols, 2);
            assert_eq!(data.grid.len(), 2);
            assert_eq!(data.grid[0][0].text, "A1");
            assert_eq!(data.grid[0][1].text, "B1");
            assert_eq!(data.grid[1][0].text, "A2");
            assert_eq!(data.grid[1][1].text, "B2");
        } else {
            panic!("Expected Table DocItem");
        }
    }

    #[test]
    fn test_pages_table_with_empty_cells() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<sl:document xmlns:sl="http://developer.apple.com/namespaces/sl" xmlns:sf="http://developer.apple.com/namespaces/sf">
    <sf:table>
        <sf:row>
            <sf:cell>Data</sf:cell>
            <sf:cell></sf:cell>
        </sf:row>
    </sf:table>
</sl:document>"#;

        let temp_file = create_test_pages_file(xml);
        let backend = PagesBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        assert_eq!(result.tables.len(), 1);
        if let DocItem::Table { data, .. } = &result.tables[0] {
            assert_eq!(data.num_rows, 1);
            assert_eq!(data.num_cols, 2);
            assert_eq!(data.grid[0][0].text, "Data");
            assert_eq!(data.grid[0][1].text, "");
        } else {
            panic!("Expected Table DocItem");
        }
    }

    #[test]
    fn test_pages_document_title() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<sl:document xmlns:sl="http://developer.apple.com/namespaces/sl" xmlns:sf="http://developer.apple.com/namespaces/sf">
    <sl:title>My Document</sl:title>
</sl:document>"#;

        let temp_file = create_test_pages_file(xml);
        let backend = PagesBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        assert_eq!(result.texts.len(), 1);
        if let DocItem::Title { text, .. } = &result.texts[0] {
            assert_eq!(text, "My Document");
        } else {
            panic!("Expected Title DocItem");
        }
    }

    #[test]
    fn test_pages_mixed_content() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<sl:document xmlns:sl="http://developer.apple.com/namespaces/sl" xmlns:sf="http://developer.apple.com/namespaces/sf">
    <sf:text-storage>
        <sf:p sf:style="heading1"><sf:text>Title</sf:text></sf:p>
        <sf:p><sf:text>First paragraph</sf:text></sf:p>
        <sf:list-item><sf:text>List item</sf:text></sf:list-item>
    </sf:text-storage>
    <sf:table>
        <sf:row>
            <sf:cell>Cell</sf:cell>
        </sf:row>
    </sf:table>
</sl:document>"#;

        let temp_file = create_test_pages_file(xml);
        let backend = PagesBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        assert_eq!(result.texts.len(), 3);
        assert_eq!(result.tables.len(), 1);
        assert_eq!(result.body.children.len(), 4); // 3 texts + 1 table
    }

    #[test]
    fn test_pages_empty_paragraphs_ignored() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<sl:document xmlns:sl="http://developer.apple.com/namespaces/sl" xmlns:sf="http://developer.apple.com/namespaces/sf">
    <sf:text-storage>
        <sf:p><sf:text></sf:text></sf:p>
        <sf:p><sf:text>Real content</sf:text></sf:p>
        <sf:p><sf:text></sf:text></sf:p>
    </sf:text-storage>
</sl:document>"#;

        let temp_file = create_test_pages_file(xml);
        let backend = PagesBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        // Only 1 text item (empty paragraphs are ignored)
        assert_eq!(result.texts.len(), 1);
        if let DocItem::Text { text, .. } = &result.texts[0] {
            assert_eq!(text, "Real content");
        } else {
            panic!("Expected Text DocItem");
        }
    }

    #[test]
    fn test_pages_missing_index_xml() {
        let temp_file = NamedTempFile::new().unwrap();
        let file = temp_file.reopen().unwrap();

        let mut zip = ZipWriter::new(file);
        zip.start_file("other.xml", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"<doc/>").unwrap();
        zip.finish().unwrap();

        let backend = PagesBackend::new();
        let result = backend.parse(temp_file.path());

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("missing index.xml"));
    }

    #[test]
    fn test_pages_invalid_zip() {
        let temp_file = NamedTempFile::new().unwrap();
        std::fs::write(temp_file.path(), b"Not a ZIP file").unwrap();

        let backend = PagesBackend::new();
        let result = backend.parse(temp_file.path());

        assert!(result.is_err());
    }

    #[test]
    fn test_pages_malformed_xml() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<sl:document xmlns:sl="http://developer.apple.com/namespaces/sl">
    <sf:unclosed-tag>
</sl:document>"#;

        let temp_file = create_test_pages_file(xml);
        let backend = PagesBackend::new();
        let result = backend.parse(temp_file.path());

        assert!(result.is_err());
    }

    #[test]
    fn test_pages_unicode_content() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<sl:document xmlns:sl="http://developer.apple.com/namespaces/sl" xmlns:sf="http://developer.apple.com/namespaces/sf">
    <sf:text-storage>
        <sf:p><sf:text>Hello 世界 🌍</sf:text></sf:p>
    </sf:text-storage>
</sl:document>"#;

        let temp_file = create_test_pages_file(xml);
        let backend = PagesBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        assert_eq!(result.texts.len(), 1);
        if let DocItem::Text { text, .. } = &result.texts[0] {
            assert_eq!(text, "Hello 世界 🌍");
        } else {
            panic!("Expected Text DocItem");
        }
    }

    #[test]
    fn test_pages_whitespace_handling() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<sl:document xmlns:sl="http://developer.apple.com/namespaces/sl" xmlns:sf="http://developer.apple.com/namespaces/sf">
    <sf:text-storage>
        <sf:p><sf:text>  Extra   spaces  </sf:text></sf:p>
    </sf:text-storage>
</sl:document>"#;

        let temp_file = create_test_pages_file(xml);
        let backend = PagesBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        assert_eq!(result.texts.len(), 1);
        if let DocItem::Text { text, .. } = &result.texts[0] {
            // XML trimming should handle excess whitespace
            assert!(text.contains("Extra"));
            assert!(text.contains("spaces"));
        } else {
            panic!("Expected Text DocItem");
        }
    }

    #[test]
    fn test_pages_document_metadata() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<sl:document xmlns:sl="http://developer.apple.com/namespaces/sl" xmlns:sf="http://developer.apple.com/namespaces/sf">
    <sf:text-storage>
        <sf:p><sf:text>Content</sf:text></sf:p>
    </sf:text-storage>
</sl:document>"#;

        let temp_file = create_test_pages_file(xml);
        let backend = PagesBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        assert_eq!(result.schema_name, "DoclingDocument");
        assert_eq!(result.version, "1.7.0");
        assert_eq!(result.origin.mimetype, "application/x-iwork-pages-sffpages");
        assert_eq!(result.pages.len(), 1);
        assert_eq!(result.pages.get("1").unwrap().page_no, 1);
    }

    #[test]
    fn test_pages_table_cell_spans() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<sl:document xmlns:sl="http://developer.apple.com/namespaces/sl" xmlns:sf="http://developer.apple.com/namespaces/sf">
    <sf:table>
        <sf:row>
            <sf:cell>A</sf:cell>
            <sf:cell>B</sf:cell>
        </sf:row>
    </sf:table>
</sl:document>"#;

        let temp_file = create_test_pages_file(xml);
        let backend = PagesBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        if let DocItem::Table { data, .. } = &result.tables[0] {
            // Default row_span and col_span should be Some(1)
            assert_eq!(data.grid[0][0].row_span, Some(1));
            assert_eq!(data.grid[0][0].col_span, Some(1));
        } else {
            panic!("Expected Table DocItem");
        }
    }

    #[test]
    fn test_pages_body_structure() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<sl:document xmlns:sl="http://developer.apple.com/namespaces/sl" xmlns:sf="http://developer.apple.com/namespaces/sf">
    <sf:text-storage>
        <sf:p><sf:text>Text</sf:text></sf:p>
    </sf:text-storage>
</sl:document>"#;

        let temp_file = create_test_pages_file(xml);
        let backend = PagesBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        assert_eq!(result.body.self_ref, "#");
        assert_eq!(result.body.name, "body");
        assert_eq!(result.body.label, "body");
        assert_eq!(result.body.content_layer, "body");
        assert_eq!(result.body.children.len(), 1);
        assert_eq!(result.body.children[0].ref_path, "#/texts/0");
    }

    #[test]
    fn test_pages_provenance_info() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<sl:document xmlns:sl="http://developer.apple.com/namespaces/sl" xmlns:sf="http://developer.apple.com/namespaces/sf">
    <sf:text-storage>
        <sf:p><sf:text>Text</sf:text></sf:p>
    </sf:text-storage>
</sl:document>"#;

        let temp_file = create_test_pages_file(xml);
        let backend = PagesBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        if let DocItem::Text { prov, .. } = &result.texts[0] {
            assert_eq!(prov.len(), 1);
            assert_eq!(prov[0].page_no, 1);
            assert!(prov[0].bbox.width() > 0.0);
            assert!(prov[0].bbox.height() > 0.0);
        } else {
            panic!("Expected Text DocItem");
        }
    }

    #[test]
    fn test_pages_empty_document() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<sl:document xmlns:sl="http://developer.apple.com/namespaces/sl" xmlns:sf="http://developer.apple.com/namespaces/sf">
</sl:document>"#;

        let temp_file = create_test_pages_file(xml);
        let backend = PagesBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        // Empty document should still parse successfully
        assert_eq!(result.texts.len(), 0);
        assert_eq!(result.tables.len(), 0);
        assert_eq!(result.body.children.len(), 0);
    }

    #[test]
    fn test_pages_multiple_text_in_paragraph() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<sl:document xmlns:sl="http://developer.apple.com/namespaces/sl" xmlns:sf="http://developer.apple.com/namespaces/sf">
    <sf:text-storage>
        <sf:p>First Second Third</sf:p>
    </sf:text-storage>
</sl:document>"#;

        let temp_file = create_test_pages_file(xml);
        let backend = PagesBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        // Multiple text nodes in one paragraph should be combined
        assert_eq!(result.texts.len(), 1);
        if let DocItem::Text { text, .. } = &result.texts[0] {
            assert!(text.contains("First"));
            assert!(text.contains("Second"));
            assert!(text.contains("Third"));
        } else {
            panic!("Expected Text DocItem");
        }
    }

    #[test]
    fn test_pages_large_table() {
        let mut xml = String::from(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<sl:document xmlns:sl="http://developer.apple.com/namespaces/sl" xmlns:sf="http://developer.apple.com/namespaces/sf">
    <sf:table>"#,
        );

        // Create 10x10 table
        for row in 0..10 {
            xml.push_str("<sf:row>");
            for col in 0..10 {
                xml.push_str(&format!("<sf:cell>R{row}C{col}</sf:cell>"));
            }
            xml.push_str("</sf:row>");
        }

        xml.push_str("</sf:table></sl:document>");

        let temp_file = create_test_pages_file(&xml);
        let backend = PagesBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        assert_eq!(result.tables.len(), 1);
        if let DocItem::Table { data, .. } = &result.tables[0] {
            assert_eq!(data.num_rows, 10);
            assert_eq!(data.num_cols, 10);
            assert_eq!(data.grid.len(), 10);
            assert_eq!(data.grid[0].len(), 10);
            assert_eq!(data.grid[0][0].text, "R0C0");
            assert_eq!(data.grid[9][9].text, "R9C9");
        } else {
            panic!("Expected Table DocItem");
        }
    }
}
