//! Microsoft Visio (.vsdx) format support - Pure Rust Implementation
//!
//! VSDX is Office Open XML format (similar to DOCX).
//! Architecture: VSDX → ZIP → XML → `DocItems` (no Python!)
//!
//! Structure:
//! - visio/pages/page*.xml contains shape text
//! - Each `<Shape>` element may contain `<Text>` with diagram labels
//!
//! Generates `DocItems` directly following correct architecture.

use anyhow::{Context, Result};
use docling_core::content::DocItem;
use docling_core::document::{Document, DocumentMetadata};
use docling_core::format::InputFormat;
use quick_xml::events::Event;
use quick_xml::Reader;
use std::fmt::Write;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use zip::ZipArchive;

/// Backend for Microsoft Visio files
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct VisioBackend;

/// A shape with its text content and position
#[derive(Debug, Clone, PartialEq)]
struct VisioShape {
    /// Shape ID (used for referencing in connections)
    id: Option<String>,
    /// Text content extracted from `<Text>` element
    text: String,
    /// X position (horizontal placement)
    pin_x: Option<f64>,
    /// Y position (vertical placement, used for sorting top-to-bottom)
    pin_y: Option<f64>,
    /// Shape width
    width: Option<f64>,
    /// Shape height
    height: Option<f64>,
    /// Shape type (e.g., "Shape", "Group")
    shape_type: Option<String>,
    /// Master shape reference (template ID)
    master: Option<String>,
    /// Page number (1-indexed, for multi-page diagrams)
    page_num: usize,
}

/// A connection between two shapes (e.g., arrows, lines)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisioConnection {
    /// Connector shape ID (the arrow/line itself)
    from_sheet: String,
    /// Target shape ID (where the connector points to)
    to_sheet: String,
    /// Connection point on source (e.g., "`BeginX`", "`EndX`")
    from_cell: String,
    /// Connection point on target (reserved for layout export)
    _to_cell: String,
}

/// State for parsing a Visio shape from XML
#[derive(Debug, Default)]
struct ShapeParseState {
    in_shape: bool,
    in_text: bool,
    text: String,
    id: Option<String>,
    pin_x: Option<f64>,
    pin_y: Option<f64>,
    width: Option<f64>,
    height: Option<f64>,
    shape_type: Option<String>,
    master: Option<String>,
}

impl ShapeParseState {
    const fn new() -> Self {
        Self {
            in_shape: false,
            in_text: false,
            text: String::new(),
            id: None,
            pin_x: None,
            pin_y: None,
            width: None,
            height: None,
            shape_type: None,
            master: None,
        }
    }

    fn reset(&mut self) {
        self.in_shape = false;
        self.in_text = false;
        self.text.clear();
        self.id = None;
        self.pin_x = None;
        self.pin_y = None;
        self.width = None;
        self.height = None;
        self.shape_type = None;
        self.master = None;
    }

    fn to_shape(&self, page_num: usize) -> Option<VisioShape> {
        let trimmed = self.text.trim();
        if trimmed.is_empty() {
            return None;
        }
        Some(VisioShape {
            id: self.id.clone(),
            text: trimmed.to_string(),
            pin_x: self.pin_x,
            pin_y: self.pin_y,
            width: self.width,
            height: self.height,
            shape_type: self.shape_type.clone(),
            master: self.master.clone(),
            page_num,
        })
    }
}

impl VisioBackend {
    /// Create a new Visio backend
    #[inline]
    #[must_use = "creates Visio backend instance"]
    pub const fn new() -> Self {
        Self
    }

    /// Create provenance from shape position
    fn create_shape_provenance(
        shape: &VisioShape,
        page_num: usize,
    ) -> Vec<docling_core::content::ProvenanceItem> {
        if let (Some(x), Some(y), Some(w), Some(h)) =
            (shape.pin_x, shape.pin_y, shape.width, shape.height)
        {
            let left = x - (w / 2.0);
            let right = x + (w / 2.0);
            let top = y + (h / 2.0);
            let bottom = y - (h / 2.0);

            vec![docling_core::content::ProvenanceItem {
                page_no: page_num,
                bbox: docling_core::content::BoundingBox {
                    l: left,
                    t: top,
                    r: right,
                    b: bottom,
                    coord_origin: docling_core::content::CoordOrigin::BottomLeft,
                },
                charspan: None,
            }]
        } else {
            Vec::new()
        }
    }

    /// Build display text with shape metadata as HTML comment
    fn build_shape_display_text(shape: &VisioShape) -> String {
        let mut display_text = shape.text.clone();
        if shape.id.is_some()
            || shape.shape_type.is_some()
            || shape.master.is_some()
            || shape.pin_x.is_some()
        {
            let mut metadata_parts = Vec::new();
            if let Some(id) = &shape.id {
                metadata_parts.push(format!("ID: {id}"));
            }
            if let Some(shape_type) = &shape.shape_type {
                metadata_parts.push(format!("Type: {shape_type}"));
            }
            if let Some(master) = &shape.master {
                metadata_parts.push(format!("Master: {master}"));
            }
            if let (Some(x), Some(y)) = (shape.pin_x, shape.pin_y) {
                metadata_parts.push(format!("Position: ({x:.2}, {y:.2})"));
            }
            if let (Some(w), Some(h)) = (shape.width, shape.height) {
                metadata_parts.push(format!("Size: {w:.2}x{h:.2}"));
            }
            if !metadata_parts.is_empty() {
                let _ = write!(display_text, " <!-- {} -->", metadata_parts.join(", "));
            }
        }
        display_text
    }

    /// Create a Text [`DocItem`] from a shape
    fn create_text_doc_item(
        shape: &VisioShape,
        text_counter: usize,
        page_num: usize,
        parent_ref: Option<docling_core::content::ItemRef>,
    ) -> DocItem {
        let prov = Self::create_shape_provenance(shape, page_num);
        let display_text = Self::build_shape_display_text(shape);

        DocItem::Text {
            self_ref: format!("#/texts/{text_counter}"),
            parent: parent_ref,
            children: Vec::new(),
            content_layer: "body".to_string(),
            prov,
            orig: shape.text.clone(),
            text: display_text,
            formatting: None,
            hyperlink: None,
        }
    }

    /// Build connector map from connections
    fn build_connector_map(
        connections: &[VisioConnection],
    ) -> std::collections::HashMap<String, (Vec<String>, Vec<String>)> {
        let mut connector_map: std::collections::HashMap<String, (Vec<String>, Vec<String>)> =
            std::collections::HashMap::new();

        for conn in connections {
            // Determine if this is a Begin or End connection for the connector (FromSheet)
            if conn.from_cell.contains("Begin") {
                // Connector's Begin point connects to ToSheet
                connector_map
                    .entry(conn.from_sheet.clone())
                    .or_insert_with(|| (Vec::new(), Vec::new()))
                    .0
                    .push(conn.to_sheet.clone());
            } else if conn.from_cell.contains("End") {
                // Connector's End point connects to ToSheet
                connector_map
                    .entry(conn.from_sheet.clone())
                    .or_insert_with(|| (Vec::new(), Vec::new()))
                    .1
                    .push(conn.to_sheet.clone());
            }
        }

        connector_map
    }

    /// Generate connection lines from connector map
    fn generate_connection_lines(
        connector_map: &std::collections::HashMap<String, (Vec<String>, Vec<String>)>,
        shape_id_to_text: &std::collections::HashMap<String, String>,
    ) -> Vec<String> {
        let mut connection_lines = Vec::new();
        let mut processed_pairs = std::collections::HashSet::new();

        for (connector_id, (from_shapes, to_shapes)) in connector_map {
            // Skip if connector is actually a shape with text (not a pure connector)
            let connector_has_text = shape_id_to_text
                .get(connector_id)
                .is_some_and(|t| !t.is_empty());

            for from_id in from_shapes {
                for to_id in to_shapes {
                    // Get shape names
                    let from_name = shape_id_to_text
                        .get(from_id)
                        .map(std::string::String::as_str)
                        .filter(|s| !s.is_empty())
                        .unwrap_or(from_id.as_str());
                    let to_name = shape_id_to_text
                        .get(to_id)
                        .map(std::string::String::as_str)
                        .filter(|s| !s.is_empty())
                        .unwrap_or(to_id.as_str());

                    // Create unique key (sorted to avoid duplicates)
                    let pair_key = if from_id < to_id {
                        format!("{from_id}→{to_id}")
                    } else {
                        format!("{to_id}→{from_id}")
                    };

                    // Only add if not already processed (insert returns true if key was new)
                    if processed_pairs.insert(pair_key) {
                        let connection_text = if connector_has_text {
                            // If connector has text (e.g., labeled edge), include it
                            // SAFETY: connector_has_text is only true if shape_id_to_text.get(connector_id)
                            // returned Some with non-empty text (checked on line 254-256 above)
                            let connector_label = shape_id_to_text.get(connector_id).unwrap();
                            format!("[{from_name}] -[{connector_label}]→ [{to_name}]")
                        } else {
                            // Pure connector (arrow/line)
                            format!("[{from_name}] → [{to_name}]")
                        };

                        connection_lines.push(connection_text);
                    }
                }
            }
        }

        connection_lines
    }

    /// Parse Cell attributes (N and V) from an element
    fn parse_cell_attrs(
        e: &quick_xml::events::BytesStart<'_>,
        reader: &Reader<&[u8]>,
    ) -> (Option<String>, Option<String>) {
        let mut cell_name = None;
        let mut cell_value = None;

        for attr in e.attributes().flatten() {
            match attr.key.as_ref() {
                b"N" => {
                    if let Ok(val) = attr.decode_and_unescape_value(reader) {
                        cell_name = Some(val.to_string());
                    }
                }
                b"V" => {
                    if let Ok(val) = attr.decode_and_unescape_value(reader) {
                        cell_value = Some(val.to_string());
                    }
                }
                _ => {}
            }
        }
        (cell_name, cell_value)
    }

    /// Apply Cell value to shape state
    fn apply_cell_value(state: &mut ShapeParseState, name: &str, value: &str) {
        match name {
            "PinX" => state.pin_x = value.parse::<f64>().ok(),
            "PinY" => state.pin_y = value.parse::<f64>().ok(),
            "Width" => state.width = value.parse::<f64>().ok(),
            "Height" => state.height = value.parse::<f64>().ok(),
            _ => {}
        }
    }

    /// Parse Connect element attributes (`FromSheet`, `ToSheet`, `FromCell`, `ToCell`)
    fn parse_connect_attrs(
        e: &quick_xml::events::BytesStart<'_>,
        reader: &Reader<&[u8]>,
    ) -> Option<VisioConnection> {
        let mut from_sheet = None;
        let mut to_sheet = None;
        let mut from_cell = None;
        let mut to_cell = None;

        for attr in e.attributes().flatten() {
            match attr.key.as_ref() {
                b"FromSheet" => {
                    if let Ok(val) = attr.decode_and_unescape_value(reader) {
                        from_sheet = Some(val.to_string());
                    }
                }
                b"ToSheet" => {
                    if let Ok(val) = attr.decode_and_unescape_value(reader) {
                        to_sheet = Some(val.to_string());
                    }
                }
                b"FromCell" => {
                    if let Ok(val) = attr.decode_and_unescape_value(reader) {
                        from_cell = Some(val.to_string());
                    }
                }
                b"ToCell" => {
                    if let Ok(val) = attr.decode_and_unescape_value(reader) {
                        to_cell = Some(val.to_string());
                    }
                }
                _ => {}
            }
        }

        // Create connection if all required fields present
        match (from_sheet, to_sheet, from_cell, to_cell) {
            (Some(from), Some(to), Some(from_c), Some(to_c)) => Some(VisioConnection {
                from_sheet: from,
                to_sheet: to,
                from_cell: from_c,
                _to_cell: to_c,
            }),
            _ => None,
        }
    }

    /// Parse Shape element attributes
    fn parse_shape_attrs(
        e: &quick_xml::events::BytesStart<'_>,
        reader: &Reader<&[u8]>,
        state: &mut ShapeParseState,
    ) {
        for attr in e.attributes().flatten() {
            match attr.key.as_ref() {
                b"ID" => {
                    if let Ok(val) = attr.decode_and_unescape_value(reader) {
                        state.id = Some(val.to_string());
                    }
                }
                b"Type" => {
                    if let Ok(val) = attr.decode_and_unescape_value(reader) {
                        state.shape_type = Some(val.to_string());
                    }
                }
                b"Master" => {
                    if let Ok(val) = attr.decode_and_unescape_value(reader) {
                        state.master = Some(val.to_string());
                    }
                }
                _ => {}
            }
        }
    }

    /// Extract shapes with text from page XML
    fn extract_shapes_from_xml(xml_content: &str, page_num: usize) -> Vec<VisioShape> {
        let mut reader = Reader::from_str(xml_content);
        reader.trim_text(true);

        let mut shapes = Vec::new();
        let mut state = ShapeParseState::new();
        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => {
                    let name = e.name();
                    if name.as_ref() == b"Shape" {
                        state.reset();
                        state.in_shape = true;
                        Self::parse_shape_attrs(e, &reader, &mut state);
                    } else if name.as_ref() == b"Text" && state.in_shape {
                        state.in_text = true;
                        state.text.clear();
                    } else if name.as_ref() == b"Cell" && state.in_shape {
                        let (cell_name, cell_value) = Self::parse_cell_attrs(e, &reader);
                        if let (Some(n), Some(v)) = (cell_name, cell_value) {
                            Self::apply_cell_value(&mut state, &n, &v);
                        }
                    }
                }
                Ok(Event::Text(e)) => {
                    if state.in_text {
                        if let Ok(text) = e.unescape() {
                            let trimmed = text.trim();
                            if !trimmed.is_empty() {
                                if !state.text.is_empty() {
                                    state.text.push(' ');
                                }
                                state.text.push_str(trimmed);
                            }
                        }
                    }
                }
                Ok(Event::Empty(ref e)) => {
                    if e.name().as_ref() == b"Cell" && state.in_shape {
                        let (cell_name, cell_value) = Self::parse_cell_attrs(e, &reader);
                        if let (Some(n), Some(v)) = (cell_name, cell_value) {
                            Self::apply_cell_value(&mut state, &n, &v);
                        }
                    }
                }
                Ok(Event::End(ref e)) => {
                    let name = e.name();
                    if name.as_ref() == b"Shape" && state.in_shape {
                        if let Some(shape) = state.to_shape(page_num) {
                            shapes.push(shape);
                        }
                        state.reset();
                    } else if name.as_ref() == b"Text" && state.in_text {
                        state.in_text = false;
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    log::warn!("XML error at position {}: {}", reader.buffer_position(), e);
                    break;
                }
                _ => {}
            }
            buf.clear();
        }

        Self::sort_shapes_by_position(&mut shapes);
        shapes
    }

    /// Sort shapes by Y position (top to bottom, Visio Y increases upward)
    fn sort_shapes_by_position(shapes: &mut [VisioShape]) {
        shapes.sort_by(|a, b| match (b.pin_y, a.pin_y) {
            (Some(y1), Some(y2)) => y1.partial_cmp(&y2).unwrap_or(std::cmp::Ordering::Equal),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        });
    }

    /// Extract connections (connectors between shapes) from page XML
    fn extract_connections_from_xml(xml_content: &str) -> Vec<VisioConnection> {
        let mut reader = Reader::from_str(xml_content);
        reader.trim_text(true);

        let mut connections = Vec::new();
        let mut in_connects = false;

        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => {
                    let name = e.name();
                    if name.as_ref() == b"Connects" {
                        in_connects = true;
                    } else if name.as_ref() == b"Connect" && in_connects {
                        if let Some(conn) = Self::parse_connect_attrs(e, &reader) {
                            connections.push(conn);
                        }
                    }
                }
                Ok(Event::Empty(ref e)) => {
                    // Handle self-closing <Connect /> elements
                    if e.name().as_ref() == b"Connect" && in_connects {
                        if let Some(conn) = Self::parse_connect_attrs(e, &reader) {
                            connections.push(conn);
                        }
                    }
                }
                Ok(Event::End(ref e)) => {
                    let name = e.name();
                    if name.as_ref() == b"Connects" && in_connects {
                        in_connects = false;
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    log::warn!("XML error at position {}: {}", reader.buffer_position(), e);
                    break;
                }
                _ => {}
            }
            buf.clear();
        }

        connections
    }

    /// Parse Visio diagram to Document (legacy markdown format)
    ///
    /// Converts via `LibreOffice` to PDF, then parses PDF
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or if parsing fails.
    #[must_use = "this function returns a parsed document that should be processed"]
    #[allow(clippy::too_many_lines)] // Complex Visio parsing - keeping together for clarity
    pub fn parse(&self, input_path: &Path) -> Result<Document> {
        // Open ZIP archive
        let file = File::open(input_path)
            .with_context(|| format!("Failed to open Visio file: {}", input_path.display()))?;

        let mut archive = ZipArchive::new(BufReader::new(file))
            .context("Failed to read Visio file as ZIP archive")?;

        let mut all_shapes = Vec::new();
        let mut all_connections = Vec::new();
        let mut page_counter = 0;

        // Extract shapes and connections from XML files
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let name = file.name().to_string();

            // Process page files (these contain the actual diagram content)
            if (name.starts_with("visio/pages/page") || name == "visio/document.xml")
                && std::path::Path::new(&name)
                    .extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("xml"))
            {
                page_counter += 1;
                let mut xml_content = String::new();
                file.read_to_string(&mut xml_content)?;
                let shapes = Self::extract_shapes_from_xml(&xml_content, page_counter);
                let connections = Self::extract_connections_from_xml(&xml_content);
                all_shapes.extend(shapes);
                all_connections.extend(connections);
            }
        }

        // Generate DocItems with page hierarchy
        let mut doc_items = Vec::new();
        let max_page = all_shapes.iter().map(|s| s.page_num).max().unwrap_or(1);

        // Group shapes by page number
        let mut shapes_by_page: std::collections::HashMap<usize, Vec<&VisioShape>> =
            std::collections::HashMap::new();
        for shape in &all_shapes {
            shapes_by_page
                .entry(shape.page_num)
                .or_default()
                .push(shape);
        }

        // Track global item counter for unique self_ref IDs
        let mut text_counter = 0;

        // Create DocItems for each page
        for page_num in 1..=max_page {
            let page_shapes = shapes_by_page
                .get(&page_num)
                .map_or(&[][..], std::vec::Vec::as_slice);

            if page_shapes.is_empty() {
                continue;
            }

            // Multi-page: create section headers, single-page: just text items
            if max_page > 1 {
                let section_ref = format!("#/sections/{}", page_num - 1);
                let mut section_children = Vec::new();

                for shape in page_shapes {
                    let text_ref = format!("#/texts/{text_counter}");
                    section_children.push(docling_core::content::ItemRef { ref_path: text_ref });
                    let parent_ref = Some(docling_core::content::ItemRef {
                        ref_path: section_ref.clone(),
                    });
                    doc_items.push(Self::create_text_doc_item(
                        shape,
                        text_counter,
                        page_num,
                        parent_ref,
                    ));
                    text_counter += 1;
                }

                // Insert section header before its children
                doc_items.insert(
                    doc_items.len() - section_children.len(),
                    DocItem::SectionHeader {
                        self_ref: section_ref,
                        parent: None,
                        children: section_children,
                        content_layer: "body".to_string(),
                        prov: Vec::new(),
                        orig: format!("Page {page_num}"),
                        text: format!("Page {page_num}"),
                        level: 1,
                        formatting: None,
                        hyperlink: None,
                    },
                );
            } else {
                // Single-page: no sections
                for shape in page_shapes {
                    doc_items.push(Self::create_text_doc_item(shape, text_counter, 1, None));
                    text_counter += 1;
                }
            }
        }

        // Build shape ID to text map for connection links
        let mut shape_id_to_text = std::collections::HashMap::new();
        for shape in &all_shapes {
            if let Some(id) = &shape.id {
                shape_id_to_text.insert(id.clone(), shape.text.clone());
            }
        }

        // Generate connection lines using helper methods
        let connector_map = Self::build_connector_map(&all_connections);
        let connection_lines = Self::generate_connection_lines(&connector_map, &shape_id_to_text);

        // Group shapes by page for hierarchical markdown
        let max_page = all_shapes.iter().map(|s| s.page_num).max().unwrap_or(1);
        let mut markdown_parts = Vec::new();

        if max_page > 1 {
            // Multi-page diagram: show page structure
            for page in 1..=max_page {
                let page_shapes: Vec<_> =
                    all_shapes.iter().filter(|s| s.page_num == page).collect();

                if !page_shapes.is_empty() {
                    markdown_parts.push(format!("# Page {page}\n\n"));
                    let page_text = page_shapes
                        .iter()
                        .map(|s| s.text.as_str())
                        .collect::<Vec<_>>()
                        .join("\n\n");
                    markdown_parts.push(page_text);
                    markdown_parts.push("\n\n".to_string());
                }
            }
        } else {
            // Single-page diagram: no page headers
            markdown_parts.push(
                all_shapes
                    .iter()
                    .map(|s| s.text.as_str())
                    .collect::<Vec<_>>()
                    .join("\n\n"),
            );
        }

        // Add connections section if any exist
        if !connection_lines.is_empty() {
            markdown_parts.push("\n\n## Connections\n\n".to_string());
            markdown_parts.push(connection_lines.join("\n"));
        }

        let markdown = markdown_parts.join("");

        // Create metadata
        let title = input_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Visio Diagram")
            .to_string();

        let metadata = DocumentMetadata {
            title: Some(title),
            author: None,
            subject: None,
            created: None,
            modified: None,
            num_pages: None,
            num_characters: markdown.len(),
            language: None,
            exif: None,
        };

        // Build Document with DocItems
        Ok(Document {
            markdown,
            format: InputFormat::Vsdx,
            content_blocks: Some(doc_items),
            metadata,
            docling_document: None,
        })
    }

    /// Get the backend name
    #[inline]
    #[must_use = "returns backend name string"]
    pub const fn name(&self) -> &'static str {
        "Visio"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as IoWrite;
    use tempfile::NamedTempFile;
    use zip::write::FileOptions;
    use zip::ZipWriter;

    // Helper function to create a test VSDX file
    fn create_test_vsdx(page_xml: &str) -> NamedTempFile {
        let temp_file = NamedTempFile::new().unwrap();
        let file = temp_file.reopen().unwrap();
        let mut zip = ZipWriter::new(file);

        // Create visio/pages/page1.xml
        let options: FileOptions<()> = FileOptions::default();
        zip.start_file("visio/pages/page1.xml", options).unwrap();
        zip.write_all(page_xml.as_bytes()).unwrap();

        zip.finish().unwrap();
        temp_file
    }

    #[test]
    fn test_visio_backend_creation() {
        let backend = VisioBackend::new();
        assert_eq!(backend.name(), "Visio");
    }

    #[test]
    #[allow(
        clippy::default_constructed_unit_structs,
        reason = "testing Default trait impl"
    )]
    fn test_visio_backend_default_equals_new() {
        // Verify derived Default produces same result as new()
        assert_eq!(VisioBackend::default(), VisioBackend::new());
    }

    #[test]
    fn test_visio_backend_default() {
        let backend = VisioBackend {};
        assert_eq!(backend.name(), "Visio");
    }

    #[test]
    fn test_extract_single_shape_with_text() {
        let _backend = VisioBackend::new();
        let xml = r#"<?xml version="1.0"?>
            <PageContents>
                <Shapes>
                    <Shape>
                        <Text>Hello World</Text>
                    </Shape>
                </Shapes>
            </PageContents>"#;

        let shapes = VisioBackend::extract_shapes_from_xml(xml, 1);
        assert_eq!(shapes.len(), 1);
        assert_eq!(shapes[0].text, "Hello World");
        // Note: PinY extraction from Cell elements is not implemented in current parser
        // (self-closing Cell tags are ignored by Event::Empty handler)
    }

    #[test]
    fn test_extract_multiple_shapes() {
        let _backend = VisioBackend::new();
        let xml = r#"<?xml version="1.0"?>
            <PageContents>
                <Shapes>
                    <Shape>
                        <Text>Shape 1</Text>
                    </Shape>
                    <Shape>
                        <Text>Shape 2</Text>
                    </Shape>
                    <Shape>
                        <Text>Shape 3</Text>
                    </Shape>
                </Shapes>
            </PageContents>"#;

        let shapes = VisioBackend::extract_shapes_from_xml(xml, 1);
        assert_eq!(shapes.len(), 3);
        // Without PinY, shapes appear in document order
        assert_eq!(shapes[0].text, "Shape 1");
        assert_eq!(shapes[1].text, "Shape 2");
        assert_eq!(shapes[2].text, "Shape 3");
    }

    #[test]
    fn test_extract_shape_without_piny() {
        let _backend = VisioBackend::new();
        let xml = r#"<?xml version="1.0"?>
            <PageContents>
                <Shapes>
                    <Shape>
                        <Text>No Position</Text>
                    </Shape>
                </Shapes>
            </PageContents>"#;

        let shapes = VisioBackend::extract_shapes_from_xml(xml, 1);
        assert_eq!(shapes.len(), 1);
        assert_eq!(shapes[0].text, "No Position");
        assert_eq!(shapes[0].pin_y, None);
    }

    #[test]
    fn test_extract_shape_with_empty_text() {
        let _backend = VisioBackend::new();
        let xml = r#"<?xml version="1.0"?>
            <PageContents>
                <Shapes>
                    <Shape>
                        <Text>   </Text>
                    </Shape>
                </Shapes>
            </PageContents>"#;

        let shapes = VisioBackend::extract_shapes_from_xml(xml, 1);
        // Empty text shapes should be filtered out
        assert_eq!(shapes.len(), 0);
    }

    #[test]
    fn test_extract_shape_with_multiline_text() {
        let _backend = VisioBackend::new();
        let xml = r#"<?xml version="1.0"?>
            <PageContents>
                <Shapes>
                    <Shape>
                        <Text>Line 1
Line 2
Line 3</Text>
                    </Shape>
                </Shapes>
            </PageContents>"#;

        let shapes = VisioBackend::extract_shapes_from_xml(xml, 1);
        assert_eq!(shapes.len(), 1);
        assert!(shapes[0].text.contains("Line 1"));
        assert!(shapes[0].text.contains("Line 2"));
        assert!(shapes[0].text.contains("Line 3"));
    }

    #[test]
    fn test_extract_shape_with_nested_cp_elements() {
        let _backend = VisioBackend::new();
        let xml = r#"<?xml version="1.0"?>
            <PageContents>
                <Shapes>
                    <Shape>
                        <Text>Before<cp IX='0'/>After</Text>
                    </Shape>
                </Shapes>
            </PageContents>"#;

        let shapes = VisioBackend::extract_shapes_from_xml(xml, 1);
        assert_eq!(shapes.len(), 1);
        // cp elements should be ignored, text should concatenate
        assert!(shapes[0].text.contains("Before"));
        assert!(shapes[0].text.contains("After"));
    }

    #[test]
    fn test_extract_shapes_mixed_piny_presence() {
        let _backend = VisioBackend::new();
        let xml = r#"<?xml version="1.0"?>
            <PageContents>
                <Shapes>
                    <Shape>
                        <Text>First Shape</Text>
                    </Shape>
                    <Shape>
                        <Text>Second Shape</Text>
                    </Shape>
                </Shapes>
            </PageContents>"#;

        let shapes = VisioBackend::extract_shapes_from_xml(xml, 1);
        assert_eq!(shapes.len(), 2);
        // Without PinY data, shapes appear in document order
        assert_eq!(shapes[0].text, "First Shape");
        assert_eq!(shapes[1].text, "Second Shape");
    }

    #[test]
    fn test_extract_shapes_with_unicode() {
        let _backend = VisioBackend::new();
        let xml = r#"<?xml version="1.0"?>
            <PageContents>
                <Shapes>
                    <Shape>
                        <Text>Hello 世界 🌍</Text>
                    </Shape>
                </Shapes>
            </PageContents>"#;

        let shapes = VisioBackend::extract_shapes_from_xml(xml, 1);
        assert_eq!(shapes.len(), 1);
        assert_eq!(shapes[0].text, "Hello 世界 🌍");
    }

    #[test]
    fn test_extract_shapes_with_xml_entities() {
        let _backend = VisioBackend::new();
        let xml = r#"<?xml version="1.0"?>
            <PageContents>
                <Shapes>
                    <Shape>
                        <Text>&lt;test&gt; &amp; &quot;quoted&quot;</Text>
                    </Shape>
                </Shapes>
            </PageContents>"#;

        let shapes = VisioBackend::extract_shapes_from_xml(xml, 1);
        assert_eq!(shapes.len(), 1);
        assert_eq!(shapes[0].text, "<test> & \"quoted\"");
    }

    #[test]
    fn test_extract_shapes_no_shapes_element() {
        let _backend = VisioBackend::new();
        let xml = r#"<?xml version="1.0"?>
            <PageContents>
            </PageContents>"#;

        let shapes = VisioBackend::extract_shapes_from_xml(xml, 1);
        assert_eq!(shapes.len(), 0);
    }

    #[test]
    fn test_parse_creates_docitems() {
        let xml = r#"<?xml version="1.0"?>
            <PageContents>
                <Shapes>
                    <Shape>
                        <Text>Test Shape</Text>
                    </Shape>
                </Shapes>
            </PageContents>"#;

        let temp_file = create_test_vsdx(xml);
        let backend = VisioBackend::new();
        let doc = backend.parse(temp_file.path()).unwrap();

        // Should have DocItems
        assert!(doc.content_blocks.is_some());
        let items = doc.content_blocks.unwrap();
        assert_eq!(items.len(), 1);

        // Check DocItem structure
        match &items[0] {
            DocItem::Text { text, self_ref, .. } => {
                assert_eq!(text, "Test Shape");
                assert_eq!(self_ref, "#/texts/0");
            }
            _ => panic!("Expected Text DocItem"),
        }
    }

    #[test]
    fn test_parse_creates_markdown() {
        let xml = r#"<?xml version="1.0"?>
            <PageContents>
                <Shapes>
                    <Shape>
                        <Text>Shape 1</Text>
                    </Shape>
                    <Shape>
                        <Text>Shape 2</Text>
                    </Shape>
                </Shapes>
            </PageContents>"#;

        let temp_file = create_test_vsdx(xml);
        let backend = VisioBackend::new();
        let doc = backend.parse(temp_file.path()).unwrap();

        // Check markdown generation
        assert!(doc.markdown.contains("Shape 1"));
        assert!(doc.markdown.contains("Shape 2"));
        assert!(doc.markdown.contains("\n\n")); // Shapes separated by double newline
    }

    #[test]
    fn test_parse_metadata() {
        let xml = r#"<?xml version="1.0"?>
            <PageContents>
                <Shapes>
                    <Shape>
                        <Text>Test</Text>
                    </Shape>
                </Shapes>
            </PageContents>"#;

        let temp_file = create_test_vsdx(xml);
        let backend = VisioBackend::new();
        let doc = backend.parse(temp_file.path()).unwrap();

        // Check metadata
        assert!(doc.metadata.title.is_some());
        assert_eq!(doc.metadata.num_characters, doc.markdown.len());
        assert_eq!(doc.format, InputFormat::Vsdx);
    }

    #[test]
    fn test_parse_empty_diagram() {
        let xml = r#"<?xml version="1.0"?>
            <PageContents>
                <Shapes>
                </Shapes>
            </PageContents>"#;

        let temp_file = create_test_vsdx(xml);
        let backend = VisioBackend::new();
        let doc = backend.parse(temp_file.path()).unwrap();

        // Empty diagram should have None content_blocks (no shapes)
        // Actually, it will have Some(vec![]) based on the implementation
        assert!(doc.content_blocks.is_some());
        assert_eq!(doc.content_blocks.unwrap().len(), 0);
        assert_eq!(doc.markdown, "");
    }

    #[test]
    fn test_parse_invalid_zip() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"Not a ZIP file").unwrap();
        temp_file.flush().unwrap();

        let backend = VisioBackend::new();
        let result = backend.parse(temp_file.path());

        // Should error on invalid ZIP
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_nonexistent_file() {
        let backend = VisioBackend::new();
        let result = backend.parse(Path::new("/nonexistent/file.vsdx"));

        // Should error on nonexistent file
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_large_diagram_many_shapes() {
        let mut xml = r#"<?xml version="1.0"?>
            <PageContents>
                <Shapes>"#
            .to_string();

        // Add 100 shapes
        for i in 0..100 {
            let _ = write!(
                xml,
                r#"
                    <Shape>
                        <Text>Shape {i}</Text>
                        <Cell N="PinY" V="{i}.0"/>
                    </Shape>"#
            );
        }

        xml.push_str(
            r"
                </Shapes>
            </PageContents>",
        );

        let temp_file = create_test_vsdx(&xml);
        let backend = VisioBackend::new();
        let doc = backend.parse(temp_file.path()).unwrap();

        // Should handle 100 shapes
        assert!(doc.content_blocks.is_some());
        assert_eq!(doc.content_blocks.unwrap().len(), 100);
    }

    #[test]
    fn test_parse_shape_with_long_text() {
        let long_text = "A".repeat(10000);
        let xml = format!(
            r#"<?xml version="1.0"?>
            <PageContents>
                <Shapes>
                    <Shape>
                        <Text>{long_text}</Text>
                    </Shape>
                </Shapes>
            </PageContents>"#
        );

        let temp_file = create_test_vsdx(&xml);
        let backend = VisioBackend::new();
        let doc = backend.parse(temp_file.path()).unwrap();

        // Should handle very long text
        assert!(doc.content_blocks.is_some());
        let items = doc.content_blocks.unwrap();
        assert_eq!(items.len(), 1);

        match &items[0] {
            DocItem::Text { text, .. } => {
                assert_eq!(text.len(), 10000);
            }
            _ => panic!("Expected Text DocItem"),
        }
    }

    #[test]
    fn test_visio_multiple_pages() {
        // Test VSDX with multiple page files
        let temp_file = NamedTempFile::new().unwrap();
        let file = temp_file.reopen().unwrap();
        let mut zip = ZipWriter::new(file);
        let options: FileOptions<()> = FileOptions::default();

        // Page 1
        let page1_xml = r#"<?xml version="1.0"?>
            <PageContents>
                <Shapes>
                    <Shape>
                        <Text>Page 1 Shape</Text>
                    </Shape>
                </Shapes>
            </PageContents>"#;
        zip.start_file("visio/pages/page1.xml", options).unwrap();
        zip.write_all(page1_xml.as_bytes()).unwrap();

        // Page 2
        let page2_xml = r#"<?xml version="1.0"?>
            <PageContents>
                <Shapes>
                    <Shape>
                        <Text>Page 2 Shape</Text>
                    </Shape>
                </Shapes>
            </PageContents>"#;
        zip.start_file("visio/pages/page2.xml", options).unwrap();
        zip.write_all(page2_xml.as_bytes()).unwrap();

        zip.finish().unwrap();

        let backend = VisioBackend::new();
        let doc = backend.parse(temp_file.path()).unwrap();

        // Should extract shapes from both pages with section headers
        assert!(doc.content_blocks.is_some());
        let items = doc.content_blocks.unwrap();
        // Multi-page: 2 SectionHeader items + 2 Text items = 4 total
        assert_eq!(items.len(), 4);

        // Verify structure: should have 2 sections with 1 text each
        let section_count = items
            .iter()
            .filter(|item| matches!(item, DocItem::SectionHeader { .. }))
            .count();
        let text_count = items
            .iter()
            .filter(|item| matches!(item, DocItem::Text { .. }))
            .count();
        assert_eq!(section_count, 2);
        assert_eq!(text_count, 2);

        // Check markdown contains both shapes and page headers
        assert!(doc.markdown.contains("Page 1"));
        assert!(doc.markdown.contains("Page 2"));
        assert!(doc.markdown.contains("Page 1 Shape"));
        assert!(doc.markdown.contains("Page 2 Shape"));
    }

    #[test]
    fn test_visio_whitespace_only_shapes() {
        let _backend = VisioBackend::new();
        let xml = r#"<?xml version="1.0"?>
            <PageContents>
                <Shapes>
                    <Shape>
                        <Text>Valid Text</Text>
                    </Shape>
                    <Shape>
                        <Text>
                        </Text>
                    </Shape>
                    <Shape>
                        <Text>		</Text>
                    </Shape>
                </Shapes>
            </PageContents>"#;

        let shapes = VisioBackend::extract_shapes_from_xml(xml, 1);
        // Only the valid text shape should remain
        assert_eq!(shapes.len(), 1);
        assert_eq!(shapes[0].text, "Valid Text");
    }

    #[test]
    fn test_visio_piny_sorting_with_negatives() {
        let _backend = VisioBackend::new();
        let xml = r#"<?xml version="1.0"?>
            <PageContents>
                <Shapes>
                    <Shape>
                        <Cell N="PinY" V="5.0"/>
                        <Text>Bottom</Text>
                    </Shape>
                    <Shape>
                        <Cell N="PinY" V="10.0"/>
                        <Text>Top</Text>
                    </Shape>
                    <Shape>
                        <Cell N="PinY" V="-2.0"/>
                        <Text>Very Bottom</Text>
                    </Shape>
                    <Shape>
                        <Cell N="PinY" V="7.5"/>
                        <Text>Middle</Text>
                    </Shape>
                </Shapes>
            </PageContents>"#;

        let shapes = VisioBackend::extract_shapes_from_xml(xml, 1);
        assert_eq!(shapes.len(), 4);

        // Visio Y coordinates increase upward, so sort descending
        // Expected order: 10.0 (Top), 7.5 (Middle), 5.0 (Bottom), -2.0 (Very Bottom)
        assert_eq!(shapes[0].text, "Top");
        assert_eq!(shapes[1].text, "Middle");
        assert_eq!(shapes[2].text, "Bottom");
        assert_eq!(shapes[3].text, "Very Bottom");
    }

    #[test]
    fn test_visio_cell_attributes_special_chars() {
        let _backend = VisioBackend::new();
        let xml = r#"<?xml version="1.0"?>
            <PageContents>
                <Shapes>
                    <Shape>
                        <Cell N="PinY" V="3.7521"/>
                        <Cell N="PinX" V="2.71828"/>
                        <Cell N="Width" V="1.0"/>
                        <Text>Shape with &lt;special&gt; &amp; chars</Text>
                    </Shape>
                </Shapes>
            </PageContents>"#;

        let shapes = VisioBackend::extract_shapes_from_xml(xml, 1);
        assert_eq!(shapes.len(), 1);
        assert_eq!(shapes[0].text, "Shape with <special> & chars");
        assert_eq!(shapes[0].pin_y, Some(3.7521));
    }

    #[test]
    fn test_visio_extract_shape_metadata() {
        // Test extraction of shape metadata (ID, Type, Master, position, size)
        let _backend = VisioBackend::new();
        let xml = r#"<?xml version="1.0"?>
            <PageContents>
                <Shapes>
                    <Shape ID="5" Type="Shape" Master="4">
                        <Cell N="PinX" V="3.5"/>
                        <Cell N="PinY" V="5.2"/>
                        <Cell N="Width" V="2.0"/>
                        <Cell N="Height" V="1.5"/>
                        <Text>Test Shape</Text>
                    </Shape>
                </Shapes>
            </PageContents>"#;

        let shapes = VisioBackend::extract_shapes_from_xml(xml, 1);
        assert_eq!(shapes.len(), 1);

        let shape = &shapes[0];
        assert_eq!(shape.text, "Test Shape");
        assert_eq!(shape.id, Some("5".to_string()));
        assert_eq!(shape.shape_type, Some("Shape".to_string()));
        assert_eq!(shape.master, Some("4".to_string()));
        assert_eq!(shape.pin_x, Some(3.5));
        assert_eq!(shape.pin_y, Some(5.2));
        assert_eq!(shape.width, Some(2.0));
        assert_eq!(shape.height, Some(1.5));
    }

    #[test]
    fn test_visio_docitems_with_metadata() {
        // Test that DocItems include provenance (bounding box) and metadata comments
        let xml = r#"<?xml version="1.0"?>
            <PageContents>
                <Shapes>
                    <Shape ID="1" Type="Shape">
                        <Cell N="PinX" V="4.0"/>
                        <Cell N="PinY" V="6.0"/>
                        <Cell N="Width" V="2.0"/>
                        <Cell N="Height" V="1.0"/>
                        <Text>Shape with Metadata</Text>
                    </Shape>
                </Shapes>
            </PageContents>"#;

        let temp_file = create_test_vsdx(xml);
        let backend = VisioBackend::new();
        let doc = backend.parse(temp_file.path()).unwrap();

        assert!(doc.content_blocks.is_some());
        let items = doc.content_blocks.unwrap();
        assert_eq!(items.len(), 1);

        match &items[0] {
            DocItem::Text {
                text, orig, prov, ..
            } => {
                // Original text should not have metadata comments
                assert_eq!(orig, "Shape with Metadata");

                // Display text should include metadata as HTML comment
                assert!(text.contains("Shape with Metadata"));
                assert!(text.contains("<!-- ID: 1"));
                assert!(text.contains("Type: Shape"));
                assert!(text.contains("Position: (4.00, 6.00)"));
                assert!(text.contains("Size: 2.00x1.00"));

                // Provenance should contain bounding box
                assert_eq!(prov.len(), 1);
                let bbox = &prov[0].bbox;

                // PinX=4, Width=2 → left=3, right=5
                // PinY=6, Height=1 → bottom=5.5, top=6.5
                assert!((bbox.l - 3.0).abs() < 0.001);
                assert!((bbox.r - 5.0).abs() < 0.001);
                assert!((bbox.b - 5.5).abs() < 0.001);
                assert!((bbox.t - 6.5).abs() < 0.001);
            }
            _ => panic!("Expected Text DocItem"),
        }
    }

    #[test]
    fn test_visio_shapes_without_metadata() {
        // Test shapes that have text but no position/size metadata
        let xml = r#"<?xml version="1.0"?>
            <PageContents>
                <Shapes>
                    <Shape>
                        <Text>Simple Shape</Text>
                    </Shape>
                </Shapes>
            </PageContents>"#;

        let temp_file = create_test_vsdx(xml);
        let backend = VisioBackend::new();
        let doc = backend.parse(temp_file.path()).unwrap();

        assert!(doc.content_blocks.is_some());
        let items = doc.content_blocks.unwrap();
        assert_eq!(items.len(), 1);

        match &items[0] {
            DocItem::Text { text, prov, .. } => {
                // Without position/size, should not have HTML comment
                assert_eq!(text, "Simple Shape");

                // Without full metadata, provenance should be empty
                assert_eq!(prov.len(), 0);
            }
            _ => panic!("Expected Text DocItem"),
        }
    }

    #[test]
    fn test_visio_extremely_long_text_5000_chars() {
        // Test with even longer text to stress-test buffer handling
        let long_text = "Visio shape with extremely long text content. ".repeat(100); // ~4600 chars
        let xml = format!(
            r#"<?xml version="1.0"?>
            <PageContents>
                <Shapes>
                    <Shape>
                        <Text>{long_text}</Text>
                    </Shape>
                </Shapes>
            </PageContents>"#
        );

        let temp_file = create_test_vsdx(&xml);
        let backend = VisioBackend::new();
        let doc = backend.parse(temp_file.path()).unwrap();

        // Should handle very long text without truncation
        assert!(doc.content_blocks.is_some());
        let items = doc.content_blocks.unwrap();
        assert_eq!(items.len(), 1);

        match &items[0] {
            DocItem::Text { text, .. } => {
                assert!(text.len() > 4500);
                assert!(text.contains("Visio shape with extremely long text content."));
            }
            _ => panic!("Expected Text DocItem"),
        }
    }

    #[test]
    #[allow(clippy::used_underscore_binding)]
    fn test_visio_extract_connections() {
        // Test extraction of connections from <Connects> elements
        let _backend = VisioBackend::new();
        let xml = r#"<?xml version="1.0"?>
            <PageContents>
                <Shapes>
                    <Shape ID="1"><Text>Shape A</Text></Shape>
                    <Shape ID="2"><Text>Shape B</Text></Shape>
                    <Shape ID="3"><Text>Connector</Text></Shape>
                </Shapes>
                <Connects>
                    <Connect FromSheet="3" FromCell="BeginX" ToSheet="1" ToCell="PinX"/>
                    <Connect FromSheet="3" FromCell="EndX" ToSheet="2" ToCell="PinX"/>
                </Connects>
            </PageContents>"#;

        let connections = VisioBackend::extract_connections_from_xml(xml);
        assert_eq!(connections.len(), 2);

        assert_eq!(connections[0].from_sheet, "3");
        assert_eq!(connections[0].to_sheet, "1");
        assert_eq!(connections[0].from_cell, "BeginX");
        assert_eq!(connections[0]._to_cell, "PinX");

        assert_eq!(connections[1].from_sheet, "3");
        assert_eq!(connections[1].to_sheet, "2");
        assert_eq!(connections[1].from_cell, "EndX");
        assert_eq!(connections[1]._to_cell, "PinX");
    }

    #[test]
    fn test_visio_connections_in_markdown() {
        // Test that connections appear in markdown output
        let xml = r#"<?xml version="1.0"?>
            <PageContents>
                <Shapes>
                    <Shape ID="1"><Text>Start</Text></Shape>
                    <Shape ID="2"><Text>End</Text></Shape>
                    <Shape ID="3"><Text>Arrow</Text></Shape>
                </Shapes>
                <Connects>
                    <Connect FromSheet="3" FromCell="BeginX" ToSheet="1" ToCell="PinX"/>
                    <Connect FromSheet="3" FromCell="EndX" ToSheet="2" ToCell="PinX"/>
                </Connects>
            </PageContents>"#;

        let temp_file = create_test_vsdx(xml);
        let backend = VisioBackend::new();
        let doc = backend.parse(temp_file.path()).unwrap();

        // Check markdown contains shapes
        assert!(doc.markdown.contains("Start"));
        assert!(doc.markdown.contains("End"));
        assert!(doc.markdown.contains("Arrow"));

        // Check markdown contains connections section
        assert!(doc.markdown.contains("## Connections"));
        // Should show connection from connector shape to other shapes
        assert!(doc.markdown.contains('[') && doc.markdown.contains(']'));
        assert!(doc.markdown.contains("→") || doc.markdown.contains("↔"));
    }

    #[test]
    fn test_visio_no_connections() {
        // Test diagram without connections
        let xml = r#"<?xml version="1.0"?>
            <PageContents>
                <Shapes>
                    <Shape ID="1"><Text>Shape A</Text></Shape>
                    <Shape ID="2"><Text>Shape B</Text></Shape>
                </Shapes>
            </PageContents>"#;

        let temp_file = create_test_vsdx(xml);
        let backend = VisioBackend::new();
        let doc = backend.parse(temp_file.path()).unwrap();

        // Without connections, should not have Connections section
        assert!(!doc.markdown.contains("## Connections"));
    }

    #[test]
    fn test_visio_self_closing_connect_elements() {
        // Test self-closing <Connect/> elements (common in VSDX)
        let _backend = VisioBackend::new();
        let xml = r#"<?xml version="1.0"?>
            <PageContents>
                <Shapes>
                    <Shape ID="1"><Text>A</Text></Shape>
                    <Shape ID="2"><Text>B</Text></Shape>
                </Shapes>
                <Connects>
                    <Connect FromSheet="1" FromCell="BeginX" ToSheet="2" ToCell="PinX"/>
                </Connects>
            </PageContents>"#;

        let connections = VisioBackend::extract_connections_from_xml(xml);
        assert_eq!(connections.len(), 1);
        assert_eq!(connections[0].from_sheet, "1");
        assert_eq!(connections[0].to_sheet, "2");
    }

    #[test]
    fn test_visio_multiple_connections_same_shape() {
        // Test shape with multiple outgoing connections (e.g., decision node)
        let xml = r#"<?xml version="1.0"?>
            <PageContents>
                <Shapes>
                    <Shape ID="1"><Text>Decision</Text></Shape>
                    <Shape ID="2"><Text>Yes Path</Text></Shape>
                    <Shape ID="3"><Text>No Path</Text></Shape>
                    <Shape ID="4"><Text>Arrow1</Text></Shape>
                    <Shape ID="5"><Text>Arrow2</Text></Shape>
                </Shapes>
                <Connects>
                    <Connect FromSheet="4" FromCell="BeginX" ToSheet="1" ToCell="PinX"/>
                    <Connect FromSheet="4" FromCell="EndX" ToSheet="2" ToCell="PinX"/>
                    <Connect FromSheet="5" FromCell="BeginX" ToSheet="1" ToCell="PinX"/>
                    <Connect FromSheet="5" FromCell="EndX" ToSheet="3" ToCell="PinX"/>
                </Connects>
            </PageContents>"#;

        let temp_file = create_test_vsdx(xml);
        let backend = VisioBackend::new();
        let doc = backend.parse(temp_file.path()).unwrap();

        // Should have connections section
        assert!(doc.markdown.contains("## Connections"));
        // Should show multiple connections
        let connection_count =
            doc.markdown.matches("→").count() + doc.markdown.matches("↔").count();
        assert!(connection_count >= 2, "Expected at least 2 connections");
    }

    #[test]
    fn test_visio_multi_page_hierarchy() {
        // Test multi-page diagram with page hierarchy
        let temp_file = NamedTempFile::new().unwrap();
        let file = temp_file.reopen().unwrap();
        let mut zip = ZipWriter::new(file);
        let options: FileOptions<()> = FileOptions::default();

        // Page 1
        let page1_xml = r#"<?xml version="1.0"?>
            <PageContents>
                <Shapes>
                    <Shape ID="1"><Text>Page 1 - Shape A</Text></Shape>
                    <Shape ID="2"><Text>Page 1 - Shape B</Text></Shape>
                </Shapes>
            </PageContents>"#;
        zip.start_file("visio/pages/page1.xml", options).unwrap();
        zip.write_all(page1_xml.as_bytes()).unwrap();

        // Page 2
        let page2_xml = r#"<?xml version="1.0"?>
            <PageContents>
                <Shapes>
                    <Shape ID="3"><Text>Page 2 - Shape C</Text></Shape>
                    <Shape ID="4"><Text>Page 2 - Shape D</Text></Shape>
                </Shapes>
            </PageContents>"#;
        zip.start_file("visio/pages/page2.xml", options).unwrap();
        zip.write_all(page2_xml.as_bytes()).unwrap();

        zip.finish().unwrap();

        let backend = VisioBackend::new();
        let doc = backend.parse(temp_file.path()).unwrap();

        // Should have page headers in markdown
        assert!(doc.markdown.contains("# Page 1"));
        assert!(doc.markdown.contains("# Page 2"));

        // Check shapes appear under correct pages
        assert!(doc.markdown.contains("Page 1 - Shape A"));
        assert!(doc.markdown.contains("Page 1 - Shape B"));
        assert!(doc.markdown.contains("Page 2 - Shape C"));
        assert!(doc.markdown.contains("Page 2 - Shape D"));

        // Verify page order (Page 1 before Page 2)
        let page1_pos = doc.markdown.find("# Page 1").unwrap();
        let page2_pos = doc.markdown.find("# Page 2").unwrap();
        assert!(page1_pos < page2_pos, "Page 1 should appear before Page 2");
    }
}
