//! Apple Numbers format support
//!
//! Numbers files (.numbers) are ZIP archives containing XML files.
//!
//! Structure:
//! - index.xml - Main document structure (Numbers '09 XML format)
//! - QuickLook/Thumbnail.jpg - Thumbnail preview
//! - Data/ - Embedded media files
//!
//! NOTE: This implementation supports Numbers '09 XML format.

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

/// Backend for Apple Numbers files
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct NumbersBackend;

impl NumbersBackend {
    /// Create a new Numbers backend instance
    #[inline]
    #[must_use = "creates a backend instance that should be used for parsing"]
    pub const fn new() -> Self {
        Self
    }

    /// Parse Numbers file and generate `DocItems` directly
    ///
    /// Parses Numbers '09 XML format and creates `DoclingDocument` structure
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

    /// Extract index.xml from Numbers ZIP archive
    fn extract_index_xml(input_path: &Path) -> Result<String> {
        let file = File::open(input_path)
            .with_context(|| format!("Failed to open Numbers file: {}", input_path.display()))?;

        let mut archive =
            ZipArchive::new(file).context("Failed to read Numbers file as ZIP archive")?;

        // Find index.xml
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            if file.name() == "index.xml" {
                let mut content = String::new();
                file.read_to_string(&mut content)?;
                return Ok(content);
            }
        }

        anyhow::bail!("Invalid Numbers file: missing index.xml")
    }

    /// Parse Numbers XML and generate `DocItems`
    #[allow(clippy::too_many_lines)] // Complex iWork XML parsing - keeping together for clarity
    fn parse_xml(xml_content: &str, input_path: &Path) -> Result<DoclingDocument> {
        let file_name = input_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown.numbers")
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
        let mut in_table = false;
        let mut in_row = false;
        let mut in_cell = false;
        let mut current_sheet_name = String::new();
        let mut current_table_name = String::new();
        let mut table_rows: Vec<Vec<String>> = Vec::new();
        let mut current_row: Vec<String> = Vec::new();
        let mut current_cell_text = String::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e) | Event::Empty(ref e)) => {
                    match e.name().as_ref() {
                        b"ls:sheet" => {
                            // Get sheet name attribute
                            current_sheet_name = e
                                .attributes()
                                .filter_map(std::result::Result::ok)
                                .find(|attr| attr.key.as_ref() == b"ls:name")
                                .and_then(|attr| String::from_utf8(attr.value.to_vec()).ok())
                                .unwrap_or_else(|| "Sheet".to_string());
                        }
                        b"ls:table" => {
                            in_table = true;
                            table_rows.clear();
                            // Get table name attribute
                            current_table_name = e
                                .attributes()
                                .filter_map(std::result::Result::ok)
                                .find(|attr| attr.key.as_ref() == b"ls:name")
                                .and_then(|attr| String::from_utf8(attr.value.to_vec()).ok())
                                .unwrap_or_else(|| "Table".to_string());
                        }
                        b"ls:row" => {
                            if in_table {
                                in_row = true;
                                current_row.clear();
                            }
                        }
                        b"ls:cell" => {
                            if in_row {
                                in_cell = true;
                                current_cell_text.clear();
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Event::End(ref e)) => {
                    match e.name().as_ref() {
                        b"ls:table" => {
                            if in_table && !table_rows.is_empty() {
                                // Create sheet heading
                                let heading = if !current_sheet_name.is_empty()
                                    && !current_table_name.is_empty()
                                {
                                    format!("{current_sheet_name} - {current_table_name}")
                                } else if !current_table_name.is_empty() {
                                    current_table_name.clone()
                                } else {
                                    "Table".to_string()
                                };

                                let heading_ref = format!("#/texts/{text_idx}");
                                body_children.push(ItemRef::new(&heading_ref));
                                text_items.push(DocItem::SectionHeader {
                                    self_ref: heading_ref,
                                    parent: Some(ItemRef::new("#")),
                                    children: vec![],
                                    content_layer: "body".to_string(),
                                    prov: vec![ProvenanceItem {
                                        page_no: 1,
                                        bbox: default_bbox,
                                        charspan: None,
                                    }],
                                    orig: heading.clone(),
                                    text: heading,
                                    level: 2,
                                    formatting: None,
                                    hyperlink: None,
                                });
                                text_idx += 1;

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
                                            confidence: None,
                                            bbox: None,
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
                        b"ls:row" => {
                            if in_row && !current_row.is_empty() {
                                table_rows.push(current_row.clone());
                                current_row.clear();
                            }
                            in_row = false;
                        }
                        b"ls:cell" => {
                            if in_cell {
                                current_row.push(current_cell_text.clone());
                                current_cell_text.clear();
                            }
                            in_cell = false;
                        }
                        _ => {}
                    }
                }
                Ok(Event::Text(e)) => {
                    if in_cell {
                        let text = e.unescape().unwrap_or_default().trim().to_string();
                        if !text.is_empty() {
                            if !current_cell_text.is_empty() {
                                current_cell_text.push(' ');
                            }
                            current_cell_text.push_str(&text);
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

        // Add title if no content
        if text_items.is_empty() && table_items.is_empty() {
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
                mimetype: "application/x-iwork-numbers-sffnumbers".to_string(),
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

    /// Extract `QuickLook` preview PDF from Numbers file
    ///
    /// Returns the raw PDF bytes that can be parsed by a PDF backend.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be opened (I/O error), if the content is not
    /// a valid ZIP archive, or if `QuickLook/Preview.pdf` is missing from the archive.
    #[must_use = "this function returns PDF data that should be processed"]
    pub fn extract_preview_pdf(&self, input_path: &Path) -> Result<Vec<u8>> {
        crate::common::extract_quicklook_pdf(input_path, "Numbers")
    }

    /// Get the backend name
    #[inline]
    #[must_use = "returns the backend identifier string"]
    pub const fn name(&self) -> &'static str {
        "Numbers"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    /// Helper to create a minimal Numbers file for testing
    fn create_test_numbers_file(index_xml: &str) -> NamedTempFile {
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
    fn test_numbers_backend_creation() {
        let backend = NumbersBackend::new();
        assert_eq!(backend.name(), "Numbers");
    }

    #[test]
    fn test_numbers_backend_default() {
        let backend = NumbersBackend;
        assert_eq!(backend.name(), "Numbers");
    }

    #[test]
    #[allow(
        clippy::default_constructed_unit_structs,
        reason = "testing Default trait impl"
    )]
    fn test_numbers_backend_default_equals_new() {
        // Verify derived Default produces same result as new()
        assert_eq!(NumbersBackend::default(), NumbersBackend::new());
    }

    #[test]
    fn test_numbers_simple_table() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<ls:document xmlns:ls="http://developer.apple.com/namespaces/ls">
    <ls:sheet ls:name="Sheet1">
        <ls:table ls:name="Table 1">
            <ls:row>
                <ls:cell>A1</ls:cell>
                <ls:cell>B1</ls:cell>
            </ls:row>
            <ls:row>
                <ls:cell>A2</ls:cell>
                <ls:cell>B2</ls:cell>
            </ls:row>
        </ls:table>
    </ls:sheet>
</ls:document>"#;

        let temp_file = create_test_numbers_file(xml);
        let backend = NumbersBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        // Should have 1 section header + 1 table
        assert_eq!(result.texts.len(), 1);
        assert_eq!(result.tables.len(), 1);

        // Check heading
        if let DocItem::SectionHeader { text, level, .. } = &result.texts[0] {
            assert_eq!(text, "Sheet1 - Table 1");
            assert_eq!(*level, 2);
        } else {
            panic!("Expected SectionHeader DocItem");
        }

        // Check table
        if let DocItem::Table { data, .. } = &result.tables[0] {
            assert_eq!(data.num_rows, 2);
            assert_eq!(data.num_cols, 2);
            assert_eq!(data.grid[0][0].text, "A1");
            assert_eq!(data.grid[0][1].text, "B1");
            assert_eq!(data.grid[1][0].text, "A2");
            assert_eq!(data.grid[1][1].text, "B2");
        } else {
            panic!("Expected Table DocItem");
        }
    }

    #[test]
    fn test_numbers_multiple_sheets() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<ls:document xmlns:ls="http://developer.apple.com/namespaces/ls">
    <ls:sheet ls:name="Sheet1">
        <ls:table ls:name="Table 1">
            <ls:row><ls:cell>Data1</ls:cell></ls:row>
        </ls:table>
    </ls:sheet>
    <ls:sheet ls:name="Sheet2">
        <ls:table ls:name="Table 2">
            <ls:row><ls:cell>Data2</ls:cell></ls:row>
        </ls:table>
    </ls:sheet>
</ls:document>"#;

        let temp_file = create_test_numbers_file(xml);
        let backend = NumbersBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        assert_eq!(result.texts.len(), 2); // 2 headings
        assert_eq!(result.tables.len(), 2); // 2 tables

        if let DocItem::SectionHeader { text, .. } = &result.texts[0] {
            assert_eq!(text, "Sheet1 - Table 1");
        } else {
            panic!("Expected SectionHeader DocItem");
        }

        if let DocItem::SectionHeader { text, .. } = &result.texts[1] {
            assert_eq!(text, "Sheet2 - Table 2");
        } else {
            panic!("Expected SectionHeader DocItem");
        }
    }

    #[test]
    fn test_numbers_empty_cells() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<ls:document xmlns:ls="http://developer.apple.com/namespaces/ls">
    <ls:sheet ls:name="Sheet1">
        <ls:table ls:name="Table 1">
            <ls:row>
                <ls:cell>A</ls:cell>
                <ls:cell></ls:cell>
            </ls:row>
        </ls:table>
    </ls:sheet>
</ls:document>"#;

        let temp_file = create_test_numbers_file(xml);
        let backend = NumbersBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        if let DocItem::Table { data, .. } = &result.tables[0] {
            assert_eq!(data.grid[0][0].text, "A");
            assert_eq!(data.grid[0][1].text, "");
        } else {
            panic!("Expected Table DocItem");
        }
    }

    #[test]
    fn test_numbers_no_table_name() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<ls:document xmlns:ls="http://developer.apple.com/namespaces/ls">
    <ls:sheet ls:name="Sheet1">
        <ls:table>
            <ls:row><ls:cell>Data</ls:cell></ls:row>
        </ls:table>
    </ls:sheet>
</ls:document>"#;

        let temp_file = create_test_numbers_file(xml);
        let backend = NumbersBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        if let DocItem::SectionHeader { text, .. } = &result.texts[0] {
            assert_eq!(text, "Sheet1 - Table");
        } else {
            panic!("Expected SectionHeader DocItem");
        }
    }

    #[test]
    fn test_numbers_no_sheet_name() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<ls:document xmlns:ls="http://developer.apple.com/namespaces/ls">
    <ls:sheet>
        <ls:table ls:name="Table 1">
            <ls:row><ls:cell>Data</ls:cell></ls:row>
        </ls:table>
    </ls:sheet>
</ls:document>"#;

        let temp_file = create_test_numbers_file(xml);
        let backend = NumbersBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        if let DocItem::SectionHeader { text, .. } = &result.texts[0] {
            // When no sheet name attribute, defaults to "Sheet"
            assert_eq!(text, "Sheet - Table 1");
        } else {
            panic!("Expected SectionHeader DocItem");
        }
    }

    #[test]
    fn test_numbers_empty_document() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<ls:document xmlns:ls="http://developer.apple.com/namespaces/ls">
</ls:document>"#;

        let temp_file = create_test_numbers_file(xml);
        let backend = NumbersBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        // Empty document should create a title
        assert_eq!(result.texts.len(), 1);
        assert_eq!(result.tables.len(), 0);

        if let DocItem::Title { .. } = &result.texts[0] {
            // Success
        } else {
            panic!("Expected Title DocItem for empty document");
        }
    }

    #[test]
    fn test_numbers_missing_index_xml() {
        let temp_file = NamedTempFile::new().unwrap();
        let file = temp_file.reopen().unwrap();

        let mut zip = ZipWriter::new(file);
        zip.start_file("other.xml", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"<doc/>").unwrap();
        zip.finish().unwrap();

        let backend = NumbersBackend::new();
        let result = backend.parse(temp_file.path());

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("missing index.xml"));
    }

    #[test]
    fn test_numbers_invalid_zip() {
        let temp_file = NamedTempFile::new().unwrap();
        std::fs::write(temp_file.path(), b"Not a ZIP file").unwrap();

        let backend = NumbersBackend::new();
        let result = backend.parse(temp_file.path());

        assert!(result.is_err());
    }

    #[test]
    fn test_numbers_malformed_xml() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<ls:document xmlns:ls="http://developer.apple.com/namespaces/ls">
    <ls:unclosed-tag>
</ls:document>"#;

        let temp_file = create_test_numbers_file(xml);
        let backend = NumbersBackend::new();
        let result = backend.parse(temp_file.path());

        assert!(result.is_err());
    }

    #[test]
    fn test_numbers_unicode_content() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<ls:document xmlns:ls="http://developer.apple.com/namespaces/ls">
    <ls:sheet ls:name="Sheet1">
        <ls:table ls:name="Table 1">
            <ls:row><ls:cell>Hello 世界 🌍</ls:cell></ls:row>
        </ls:table>
    </ls:sheet>
</ls:document>"#;

        let temp_file = create_test_numbers_file(xml);
        let backend = NumbersBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        if let DocItem::Table { data, .. } = &result.tables[0] {
            assert_eq!(data.grid[0][0].text, "Hello 世界 🌍");
        } else {
            panic!("Expected Table DocItem");
        }
    }

    #[test]
    fn test_numbers_document_metadata() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<ls:document xmlns:ls="http://developer.apple.com/namespaces/ls">
    <ls:sheet ls:name="Sheet1">
        <ls:table ls:name="Table 1">
            <ls:row><ls:cell>Data</ls:cell></ls:row>
        </ls:table>
    </ls:sheet>
</ls:document>"#;

        let temp_file = create_test_numbers_file(xml);
        let backend = NumbersBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        assert_eq!(result.schema_name, "DoclingDocument");
        assert_eq!(result.version, "1.7.0");
        assert_eq!(
            result.origin.mimetype,
            "application/x-iwork-numbers-sffnumbers"
        );
        assert_eq!(result.pages.len(), 1);
        assert_eq!(result.pages.get("1").unwrap().page_no, 1);
    }

    #[test]
    fn test_numbers_body_structure() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<ls:document xmlns:ls="http://developer.apple.com/namespaces/ls">
    <ls:sheet ls:name="Sheet1">
        <ls:table ls:name="Table 1">
            <ls:row><ls:cell>Data</ls:cell></ls:row>
        </ls:table>
    </ls:sheet>
</ls:document>"#;

        let temp_file = create_test_numbers_file(xml);
        let backend = NumbersBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        assert_eq!(result.body.self_ref, "#");
        assert_eq!(result.body.name, "body");
        assert_eq!(result.body.label, "body");
        assert_eq!(result.body.content_layer, "body");
        assert_eq!(result.body.children.len(), 2); // heading + table
    }

    #[test]
    fn test_numbers_table_cell_spans() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<ls:document xmlns:ls="http://developer.apple.com/namespaces/ls">
    <ls:sheet ls:name="Sheet1">
        <ls:table ls:name="Table 1">
            <ls:row>
                <ls:cell>A</ls:cell>
                <ls:cell>B</ls:cell>
            </ls:row>
        </ls:table>
    </ls:sheet>
</ls:document>"#;

        let temp_file = create_test_numbers_file(xml);
        let backend = NumbersBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        if let DocItem::Table { data, .. } = &result.tables[0] {
            assert_eq!(data.grid[0][0].row_span, Some(1));
            assert_eq!(data.grid[0][0].col_span, Some(1));
        } else {
            panic!("Expected Table DocItem");
        }
    }

    #[test]
    fn test_numbers_large_table() {
        let mut xml = String::from(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<ls:document xmlns:ls="http://developer.apple.com/namespaces/ls">
    <ls:sheet ls:name="Sheet1">
        <ls:table ls:name="Table 1">"#,
        );

        // Create 10x10 table
        for row in 0..10 {
            xml.push_str("<ls:row>");
            for col in 0..10 {
                xml.push_str(&format!("<ls:cell>R{row}C{col}</ls:cell>"));
            }
            xml.push_str("</ls:row>");
        }

        xml.push_str(
            "</ls:table>
    </ls:sheet>
</ls:document>",
        );

        let temp_file = create_test_numbers_file(&xml);
        let backend = NumbersBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

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

    #[test]
    fn test_numbers_multiple_tables_same_sheet() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<ls:document xmlns:ls="http://developer.apple.com/namespaces/ls">
    <ls:sheet ls:name="Sheet1">
        <ls:table ls:name="Table 1">
            <ls:row><ls:cell>T1</ls:cell></ls:row>
        </ls:table>
        <ls:table ls:name="Table 2">
            <ls:row><ls:cell>T2</ls:cell></ls:row>
        </ls:table>
    </ls:sheet>
</ls:document>"#;

        let temp_file = create_test_numbers_file(xml);
        let backend = NumbersBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        assert_eq!(result.texts.len(), 2); // 2 headings
        assert_eq!(result.tables.len(), 2); // 2 tables

        if let DocItem::SectionHeader { text, .. } = &result.texts[0] {
            assert_eq!(text, "Sheet1 - Table 1");
        } else {
            panic!("Expected first heading");
        }

        if let DocItem::SectionHeader { text, .. } = &result.texts[1] {
            assert_eq!(text, "Sheet1 - Table 2");
        } else {
            panic!("Expected second heading");
        }
    }

    #[test]
    fn test_numbers_empty_rows_ignored() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<ls:document xmlns:ls="http://developer.apple.com/namespaces/ls">
    <ls:sheet ls:name="Sheet1">
        <ls:table ls:name="Table 1">
            <ls:row></ls:row>
            <ls:row><ls:cell>Data</ls:cell></ls:row>
            <ls:row></ls:row>
        </ls:table>
    </ls:sheet>
</ls:document>"#;

        let temp_file = create_test_numbers_file(xml);
        let backend = NumbersBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        if let DocItem::Table { data, .. } = &result.tables[0] {
            // Empty rows should be ignored
            assert_eq!(data.num_rows, 1);
            assert_eq!(data.grid[0][0].text, "Data");
        } else {
            panic!("Expected Table DocItem");
        }
    }

    #[test]
    fn test_numbers_whitespace_handling() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<ls:document xmlns:ls="http://developer.apple.com/namespaces/ls">
    <ls:sheet ls:name="Sheet1">
        <ls:table ls:name="Table 1">
            <ls:row><ls:cell>  Extra   spaces  </ls:cell></ls:row>
        </ls:table>
    </ls:sheet>
</ls:document>"#;

        let temp_file = create_test_numbers_file(xml);
        let backend = NumbersBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        if let DocItem::Table { data, .. } = &result.tables[0] {
            // XML trimming should handle excess whitespace
            assert!(data.grid[0][0].text.contains("Extra"));
            assert!(data.grid[0][0].text.contains("spaces"));
        } else {
            panic!("Expected Table DocItem");
        }
    }

    #[test]
    fn test_numbers_provenance_info() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<ls:document xmlns:ls="http://developer.apple.com/namespaces/ls">
    <ls:sheet ls:name="Sheet1">
        <ls:table ls:name="Table 1">
            <ls:row><ls:cell>Data</ls:cell></ls:row>
        </ls:table>
    </ls:sheet>
</ls:document>"#;

        let temp_file = create_test_numbers_file(xml);
        let backend = NumbersBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        if let DocItem::SectionHeader { prov, .. } = &result.texts[0] {
            assert_eq!(prov.len(), 1);
            assert_eq!(prov[0].page_no, 1);
            assert!(prov[0].bbox.width() > 0.0);
            assert!(prov[0].bbox.height() > 0.0);
        } else {
            panic!("Expected SectionHeader DocItem");
        }

        if let DocItem::Table { prov, .. } = &result.tables[0] {
            assert_eq!(prov.len(), 1);
            assert_eq!(prov[0].page_no, 1);
        } else {
            panic!("Expected Table DocItem");
        }
    }

    #[test]
    fn test_numbers_very_long_cell_content() {
        let long_text = "A".repeat(5000);
        let xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<ls:document xmlns:ls="http://developer.apple.com/namespaces/ls">
    <ls:sheet ls:name="Sheet1">
        <ls:table ls:name="Table 1">
            <ls:row><ls:cell>{long_text}</ls:cell></ls:row>
        </ls:table>
    </ls:sheet>
</ls:document>"#
        );

        let temp_file = create_test_numbers_file(&xml);
        let backend = NumbersBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        if let DocItem::Table { data, .. } = &result.tables[0] {
            assert_eq!(data.grid[0][0].text.len(), 5000);
            assert_eq!(data.grid[0][0].text, long_text);
        } else {
            panic!("Expected Table DocItem");
        }
    }

    #[test]
    fn test_numbers_special_xml_characters() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<ls:document xmlns:ls="http://developer.apple.com/namespaces/ls">
    <ls:sheet ls:name="Sheet1">
        <ls:table ls:name="Table 1">
            <ls:row><ls:cell>&lt;tag&gt; &amp; "quotes"</ls:cell></ls:row>
        </ls:table>
    </ls:sheet>
</ls:document>"#;

        let temp_file = create_test_numbers_file(xml);
        let backend = NumbersBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        if let DocItem::Table { data, .. } = &result.tables[0] {
            // XML entities should be decoded
            assert_eq!(data.grid[0][0].text, "<tag> & \"quotes\"");
        } else {
            panic!("Expected Table DocItem");
        }
    }

    #[test]
    fn test_numbers_typed_cells() {
        // Test sf:string and sf:number elements (real Numbers format)
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<ls:document xmlns:ls="http://developer.apple.com/namespaces/ls" xmlns:sf="http://developer.apple.com/namespaces/sf">
    <ls:sheet ls:name="Data">
        <ls:table ls:name="Sales">
            <ls:row>
                <ls:cell><sf:string>Product</sf:string></ls:cell>
                <ls:cell><sf:string>Revenue</sf:string></ls:cell>
            </ls:row>
            <ls:row>
                <ls:cell><sf:string>Widget</sf:string></ls:cell>
                <ls:cell><sf:number>125000</sf:number></ls:cell>
            </ls:row>
            <ls:row>
                <ls:cell><sf:string>Gadget</sf:string></ls:cell>
                <ls:cell><sf:number>98500</sf:number></ls:cell>
            </ls:row>
        </ls:table>
    </ls:sheet>
</ls:document>"#;

        let temp_file = create_test_numbers_file(xml);
        let backend = NumbersBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        assert_eq!(result.texts.len(), 1); // 1 heading
        assert_eq!(result.tables.len(), 1); // 1 table

        // Check table content
        if let DocItem::Table { data, .. } = &result.tables[0] {
            assert_eq!(data.num_rows, 3);
            assert_eq!(data.num_cols, 2);

            // Header row
            assert_eq!(data.grid[0][0].text, "Product");
            assert_eq!(data.grid[0][1].text, "Revenue");

            // Data rows - numbers should be extracted as text
            assert_eq!(data.grid[1][0].text, "Widget");
            assert_eq!(data.grid[1][1].text, "125000");
            assert_eq!(data.grid[2][0].text, "Gadget");
            assert_eq!(data.grid[2][1].text, "98500");
        } else {
            panic!("Expected Table DocItem");
        }
    }

    #[test]
    fn test_numbers_various_cell_types() {
        // Test various sf: cell types (date, bool, duration, etc.)
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<ls:document xmlns:ls="http://developer.apple.com/namespaces/ls" xmlns:sf="http://developer.apple.com/namespaces/sf">
    <ls:sheet ls:name="Types">
        <ls:table ls:name="Data Types">
            <ls:row>
                <ls:cell><sf:string>Name</sf:string></ls:cell>
                <ls:cell><sf:number>42</sf:number></ls:cell>
                <ls:cell><sf:date>2025-01-15</sf:date></ls:cell>
                <ls:cell><sf:bool>true</sf:bool></ls:cell>
            </ls:row>
        </ls:table>
    </ls:sheet>
</ls:document>"#;

        let temp_file = create_test_numbers_file(xml);
        let backend = NumbersBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        if let DocItem::Table { data, .. } = &result.tables[0] {
            assert_eq!(data.num_rows, 1);
            assert_eq!(data.num_cols, 4);

            // All cell types should extract their text content
            assert_eq!(data.grid[0][0].text, "Name");
            assert_eq!(data.grid[0][1].text, "42");
            assert_eq!(data.grid[0][2].text, "2025-01-15");
            assert_eq!(data.grid[0][3].text, "true");
        } else {
            panic!("Expected Table DocItem");
        }
    }

    #[test]
    fn test_numbers_single_column_table() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<ls:document xmlns:ls="http://developer.apple.com/namespaces/ls">
    <ls:sheet ls:name="Sheet1">
        <ls:table ls:name="Table 1">
            <ls:row><ls:cell>Row1</ls:cell></ls:row>
            <ls:row><ls:cell>Row2</ls:cell></ls:row>
            <ls:row><ls:cell>Row3</ls:cell></ls:row>
        </ls:table>
    </ls:sheet>
</ls:document>"#;

        let temp_file = create_test_numbers_file(xml);
        let backend = NumbersBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        if let DocItem::Table { data, .. } = &result.tables[0] {
            assert_eq!(data.num_rows, 3);
            assert_eq!(data.num_cols, 1);
            assert_eq!(data.grid[0][0].text, "Row1");
            assert_eq!(data.grid[1][0].text, "Row2");
            assert_eq!(data.grid[2][0].text, "Row3");
        } else {
            panic!("Expected Table DocItem");
        }
    }

    #[test]
    fn test_numbers_single_row_table() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<ls:document xmlns:ls="http://developer.apple.com/namespaces/ls">
    <ls:sheet ls:name="Sheet1">
        <ls:table ls:name="Table 1">
            <ls:row>
                <ls:cell>Col1</ls:cell>
                <ls:cell>Col2</ls:cell>
                <ls:cell>Col3</ls:cell>
            </ls:row>
        </ls:table>
    </ls:sheet>
</ls:document>"#;

        let temp_file = create_test_numbers_file(xml);
        let backend = NumbersBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        if let DocItem::Table { data, .. } = &result.tables[0] {
            assert_eq!(data.num_rows, 1);
            assert_eq!(data.num_cols, 3);
            assert_eq!(data.grid[0][0].text, "Col1");
            assert_eq!(data.grid[0][1].text, "Col2");
            assert_eq!(data.grid[0][2].text, "Col3");
        } else {
            panic!("Expected Table DocItem");
        }
    }

    #[test]
    fn test_numbers_very_long_names() {
        let long_sheet_name = "S".repeat(200);
        let long_table_name = "T".repeat(200);
        let xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<ls:document xmlns:ls="http://developer.apple.com/namespaces/ls">
    <ls:sheet ls:name="{long_sheet_name}">
        <ls:table ls:name="{long_table_name}">
            <ls:row><ls:cell>Data</ls:cell></ls:row>
        </ls:table>
    </ls:sheet>
</ls:document>"#
        );

        let temp_file = create_test_numbers_file(&xml);
        let backend = NumbersBackend::new();
        let result = backend.parse(temp_file.path()).unwrap();

        if let DocItem::SectionHeader { text, .. } = &result.texts[0] {
            assert!(text.len() > 400); // Both names + separator
            assert!(text.contains(&long_sheet_name));
            assert!(text.contains(&long_table_name));
        } else {
            panic!("Expected SectionHeader DocItem");
        }
    }
}
