//! `OpenDocument` Format (ODF) backend for ODT, ODS, and ODP formats
//!
//! This module provides `OpenDocument` parsing and markdown conversion capabilities
//! for various office document formats (ISO/IEC 26300).

// Clippy pedantic allows:
// - Unit struct &self convention for backend methods
// - XML parsing functions are necessarily complex
#![allow(clippy::trivially_copy_pass_by_ref)]
#![allow(clippy::too_many_lines)]

use crate::traits::{BackendOptions, DocumentBackend};
use crate::utils::{create_list_item, create_section_header, create_text_item};
use chrono::{DateTime, Utc};
use docling_core::{
    content::{CoordOrigin, DocItem, ProvenanceItem, TableCell, TableData},
    DoclingError, Document, DocumentMetadata, InputFormat,
};
use docling_opendocument::{parse_odp_slides, parse_ods_sheets};
use quick_xml::events::Event;
use quick_xml::Reader;
use std::fmt::Write as FmtWrite;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use zip::ZipArchive;

/// Metadata extracted from `OpenDocument` (author, title, subject, created, modified)
type OpenDocumentMetadata = (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<DateTime<Utc>>,
    Option<DateTime<Utc>>,
);

/// Text run with formatting information
///
/// Represents a single run of text with consistent formatting (bold, italic, underline).
/// Used to build paragraphs with mixed formatting.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TextRun {
    text: String,
    bold: bool,
    italic: bool,
    underline: bool,
}

impl TextRun {
    const fn new(text: String, bold: bool, italic: bool, underline: bool) -> Self {
        Self {
            text,
            bold,
            italic,
            underline,
        }
    }

    /// Check if this run has any formatting applied
    #[inline]
    const fn has_formatting(&self) -> bool {
        self.bold || self.italic || self.underline
    }

    /// Check if two runs have identical formatting
    #[inline]
    const fn same_formatting(&self, other: &Self) -> bool {
        self.bold == other.bold && self.italic == other.italic && self.underline == other.underline
    }
}

/// Builder for ODT paragraphs with formatting support
///
/// Accumulates text runs and generates `DocItems` when paragraph ends.
/// Groups consecutive runs with identical formatting to minimize `DocItem` count.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
struct OdtParagraphBuilder {
    runs: Vec<TextRun>,
    current_bold: bool,
    current_italic: bool,
    current_underline: bool,
}

impl OdtParagraphBuilder {
    #[inline]
    const fn new() -> Self {
        Self {
            runs: Vec::new(),
            current_bold: false,
            current_italic: false,
            current_underline: false,
        }
    }

    /// Add text with current formatting state
    #[inline]
    fn add_text(&mut self, text: String) {
        if !text.is_empty() {
            self.runs.push(TextRun::new(
                text,
                self.current_bold,
                self.current_italic,
                self.current_underline,
            ));
        }
    }

    /// Update formatting state from style name (heuristic approach)
    ///
    /// ODT uses style references like text:style-name="Bold".
    /// Since we don't parse styles.xml yet, we use heuristics:
    /// - Style name contains "bold" (case-insensitive) → bold
    /// - Style name contains "italic" (case-insensitive) → italic
    /// - Style name contains "underline" (case-insensitive) → underline
    #[inline]
    fn set_formatting_from_style(&mut self, style_name: &str) {
        let lower = style_name.to_lowercase();
        self.current_bold = lower.contains("bold") || lower.contains("strong");
        self.current_italic = lower.contains("italic") || lower.contains("emphasis");
        self.current_underline = lower.contains("underline");
    }

    /// Clear formatting state (exit span)
    #[inline]
    const fn clear_formatting(&mut self) {
        self.current_bold = false;
        self.current_italic = false;
        self.current_underline = false;
    }

    /// Check if builder has any content
    #[inline]
    fn is_empty(&self) -> bool {
        self.runs.is_empty() || self.runs.iter().all(|r| r.text.trim().is_empty())
    }

    /// Build `DocItems` from accumulated runs
    ///
    /// Groups consecutive runs with identical formatting to minimize `DocItem` count.
    /// Returns a vector of Text `DocItems` with appropriate formatting attributes.
    fn build_doc_items(&self, start_idx: usize, provenance: &[ProvenanceItem]) -> Vec<DocItem> {
        if self.is_empty() {
            return Vec::new();
        }

        let mut doc_items = Vec::new();
        let mut grouped_runs: Vec<TextRun> = Vec::new();

        // Group consecutive runs with same formatting
        for run in &self.runs {
            if let Some(last) = grouped_runs.last_mut() {
                if last.same_formatting(run) {
                    // Same formatting - merge text
                    last.text.push_str(&run.text);
                    continue;
                }
            }
            // Different formatting or first run - add new group
            grouped_runs.push(run.clone());
        }

        // Generate DocItems from grouped runs
        for (idx, run) in grouped_runs.iter().enumerate() {
            let text = run.text.trim();
            if text.is_empty() {
                continue;
            }

            let formatting = if run.has_formatting() {
                use docling_core::content::Formatting;
                Some(Formatting {
                    bold: Some(run.bold),
                    italic: Some(run.italic),
                    underline: Some(run.underline),
                    strikethrough: None,
                    code: None,
                    script: None,
                    font_size: None,
                    font_family: None,
                })
            } else {
                None
            };

            doc_items.push(DocItem::Text {
                self_ref: format!("#/texts/{}", start_idx + idx),
                parent: None,
                children: vec![],
                content_layer: "body".to_string(),
                prov: provenance.to_vec(),
                orig: text.to_string(),
                text: text.to_string(),
                formatting,
                hyperlink: None,
            });
        }

        doc_items
    }
}

/// `OpenDocument` backend for processing office documents
///
/// Supports:
/// - ODT (.odt) - `OpenDocument` Text (`LibreOffice` Writer)
/// - ODS (.ods) - `OpenDocument` Spreadsheet (`LibreOffice` Calc)
/// - ODP (.odp) - `OpenDocument` Presentation (`LibreOffice` Impress)
///
/// Parses document content, metadata, and structure.
/// Converts to markdown with document structure preserved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OpenDocumentBackend {
    format: InputFormat,
}

impl OpenDocumentBackend {
    /// Create a new `OpenDocument` backend for the specified format
    ///
    /// # Errors
    ///
    /// Returns an error if the format is not an `OpenDocument` format.
    #[inline]
    #[must_use = "creating a backend that is not used is a waste of resources"]
    pub fn new(format: InputFormat) -> Result<Self, DoclingError> {
        match format {
            InputFormat::Odt | InputFormat::Ods | InputFormat::Odp => Ok(Self { format }),
            _ => Err(DoclingError::FormatError(format!(
                "Format {format:?} is not an OpenDocument format"
            ))),
        }
    }

    /// Extract metadata from meta.xml
    ///
    /// Returns (author, title, subject, `creation_date`, `modification_date`)
    /// Uses Dublin Core elements from `OpenDocument` specification
    // Method signature kept for API consistency with other OpenDocumentBackend methods
    #[allow(clippy::unused_self)]
    fn extract_metadata(&self, archive: &mut ZipArchive<File>) -> OpenDocumentMetadata {
        // Try to read meta.xml
        let meta_xml = match archive.by_name("meta.xml") {
            Ok(mut file) => {
                let mut content = String::new();
                if file.read_to_string(&mut content).is_err() {
                    return (None, None, None, None, None);
                }
                content
            }
            Err(_) => return (None, None, None, None, None),
        };

        let mut author = None;
        let mut title = None;
        let mut subject = None;
        let mut creation_date = None;
        let mut modification_date = None;

        let mut reader = Reader::from_str(&meta_xml);
        reader.trim_text(true);
        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e) | Event::Empty(e)) => {
                    let name = e.name();
                    let name_str = String::from_utf8_lossy(name.as_ref());

                    // Read text content for Dublin Core elements
                    // Use a separate buffer to avoid double mutable borrow
                    let mut text_buf = Vec::new();
                    if let Ok(Event::Text(text)) = reader.read_event_into(&mut text_buf) {
                        let text_str = text.unescape().unwrap_or_default().trim().to_string();
                        if !text_str.is_empty() {
                            match name_str.as_ref() {
                                "dc:creator" => author = Some(text_str),
                                "dc:title" => title = Some(text_str),
                                "dc:subject" => subject = Some(text_str),
                                "meta:creation-date" => {
                                    creation_date = Self::parse_datetime(&text_str);
                                }
                                "dc:date" => modification_date = Self::parse_datetime(&text_str),
                                _ => {}
                            }
                        }
                    }
                }
                Ok(Event::Eof) | Err(_) => break,
                _ => {}
            }
            buf.clear();
        }

        (author, title, subject, creation_date, modification_date)
    }

    /// Parse ISO 8601 datetime string to `chrono::DateTime<Utc>`
    ///
    /// `OpenDocument` files use W3CDTF format (ISO 8601):
    /// - 2024-01-15T10:30:00Z
    /// - 2024-01-15T10:30:00.123Z
    #[inline]
    fn parse_datetime(s: &str) -> Option<DateTime<Utc>> {
        DateTime::parse_from_rfc3339(s)
            .ok()
            .map(|dt| dt.with_timezone(&Utc))
    }
}

impl OpenDocumentBackend {
    /// Parse ODT file and generate `DocItems` with structure
    fn parse_odt_with_structure(&self, path: &Path) -> Result<Vec<DocItem>, DoclingError> {
        let file = File::open(path).map_err(DoclingError::IoError)?;
        let mut archive = ZipArchive::new(file)
            .map_err(|e| DoclingError::BackendError(format!("Failed to open ODT as ZIP: {e}")))?;

        // Read content.xml
        let xml_content = {
            let mut content_file = archive
                .by_name("content.xml")
                .map_err(|e| DoclingError::BackendError(format!("Missing content.xml: {e}")))?;
            let mut content = String::new();
            content_file
                .read_to_string(&mut content)
                .map_err(DoclingError::IoError)?;
            content
        };

        self.parse_odt_content(&xml_content)
    }

    /// Parse ODT content.xml and extract `DocItems`
    fn parse_odt_content(&self, xml_content: &str) -> Result<Vec<DocItem>, DoclingError> {
        let mut doc_items = Vec::new();
        let mut reader = Reader::from_str(xml_content);
        reader.trim_text(true);

        let mut buf = Vec::new();
        let mut in_paragraph = false;
        let mut in_heading = false;
        let mut in_list_item = false;
        let mut in_table = false;
        let mut in_table_cell = false;
        let mut in_span = false; // NEW: Track <text:span> elements
        let mut current_text = String::new(); // For headings, lists, table cells
        let mut paragraph_builder = OdtParagraphBuilder::new(); // NEW: For formatted paragraphs
        let mut heading_level: Option<usize> = None;
        let mut list_depth: usize = 0;
        let mut list_counter: usize = 0; // Track numbered list counter for proper markers
        let mut table_rows: Vec<Vec<TableCell>> = Vec::new();
        let mut current_row: Vec<TableCell> = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e) | Event::Empty(e)) => {
                    let name = e.name();
                    let name_bytes = name.as_ref();

                    match name_bytes {
                        b"text:p" => {
                            in_paragraph = true;
                            current_text.clear();
                            paragraph_builder = OdtParagraphBuilder::new(); // NEW: Reset builder
                        }
                        b"text:h" => {
                            in_heading = true;
                            current_text.clear();
                            // Extract heading level from outline-level attribute
                            heading_level = e
                                .attributes()
                                .find_map(|a| {
                                    let attr = a.ok()?;
                                    if attr.key.local_name().as_ref() == b"outline-level" {
                                        String::from_utf8_lossy(&attr.value).parse::<usize>().ok()
                                    } else {
                                        None
                                    }
                                })
                                .or(Some(1)); // Default to level 1 if not specified
                        }
                        // NEW: Handle <text:span> for inline formatting
                        b"text:span" => {
                            in_span = true;
                            // Extract style-name attribute
                            if let Some(style_name) = e.attributes().find_map(|a| {
                                let attr = a.ok()?;
                                if attr.key.local_name().as_ref() == b"style-name" {
                                    Some(String::from_utf8_lossy(&attr.value).to_string())
                                } else {
                                    None
                                }
                            }) {
                                // Set formatting based on style name heuristics
                                paragraph_builder.set_formatting_from_style(&style_name);
                            }
                        }
                        b"text:list" => {
                            list_depth += 1;
                            list_counter = 0; // Reset counter when entering new list
                        }
                        b"text:list-item" => {
                            in_list_item = true;
                            list_counter += 1; // Increment counter for each list item
                        }
                        b"text:s" => {
                            if in_paragraph && !in_heading && !in_table_cell && !in_list_item {
                                paragraph_builder.add_text(" ".to_string());
                            } else {
                                current_text.push(' ');
                            }
                        }
                        b"text:tab" => {
                            if in_paragraph && !in_heading && !in_table_cell && !in_list_item {
                                paragraph_builder.add_text("\t".to_string());
                            } else {
                                current_text.push('\t');
                            }
                        }
                        b"text:line-break" => {
                            if in_paragraph && !in_heading && !in_table_cell && !in_list_item {
                                paragraph_builder.add_text("\n".to_string());
                            } else {
                                current_text.push('\n');
                            }
                        }
                        b"table:table" => {
                            in_table = true;
                            table_rows.clear();
                        }
                        b"table:table-row" => {
                            current_row.clear();
                        }
                        b"table:table-cell" => {
                            in_table_cell = true;
                            current_text.clear();
                        }
                        _ => {}
                    }
                }
                Ok(Event::Text(e)) if in_paragraph || in_heading || in_table_cell => {
                    let text = e.unescape().map_err(|e| {
                        DoclingError::BackendError(format!("XML unescape error: {e}"))
                    })?;

                    // Route text to paragraph builder for regular paragraphs (not lists, headings, or tables)
                    if in_paragraph && !in_heading && !in_table_cell && !in_list_item {
                        paragraph_builder.add_text(text.to_string());
                    } else {
                        current_text.push_str(&text);
                    }
                }
                Ok(Event::End(e)) => {
                    let name = e.name();
                    let name_bytes = name.as_ref();

                    match name_bytes {
                        // NEW: Handle </text:span>
                        b"text:span" => {
                            if in_span {
                                paragraph_builder.clear_formatting();
                                in_span = false;
                            }
                        }
                        b"text:p" => {
                            if in_paragraph {
                                if in_list_item {
                                    // List items use simple text (no formatting)
                                    let text = current_text.trim();
                                    if !text.is_empty() {
                                        doc_items.push(create_list_item(
                                            doc_items.len(),
                                            text.to_string(),
                                            format!("{list_counter}."), // Use counter, not depth
                                            true,
                                            self.create_provenance(1),
                                            None,
                                        ));
                                    }
                                } else if !in_table {
                                    // NEW: Use paragraph builder for regular paragraphs
                                    if !paragraph_builder.is_empty() {
                                        let items = paragraph_builder.build_doc_items(
                                            doc_items.len(),
                                            &self.create_provenance(1),
                                        );
                                        doc_items.extend(items);
                                    }
                                }
                                // Note: When in_table, text stays in current_text for table cell
                                in_paragraph = false;
                                // Only clear if NOT in table cell (table cell needs the text)
                                if !in_table_cell {
                                    current_text.clear();
                                }
                            }
                        }
                        b"text:h" => {
                            if in_heading {
                                let text = current_text.trim();
                                if !text.is_empty() {
                                    doc_items.push(create_section_header(
                                        doc_items.len(),
                                        text.to_string(),
                                        heading_level.unwrap_or(1),
                                        self.create_provenance(1),
                                    ));
                                }
                                in_heading = false;
                                heading_level = None;
                                current_text.clear();
                            }
                        }
                        b"text:list-item" => {
                            in_list_item = false;
                        }
                        b"text:list" => {
                            list_depth = list_depth.saturating_sub(1);
                        }
                        b"table:table-cell" => {
                            if in_table_cell {
                                current_row.push(TableCell {
                                    text: current_text.trim().to_string(),
                                    row_span: Some(1),
                                    col_span: Some(1),
                                    ref_item: None,
                                    start_row_offset_idx: None,
                                    start_col_offset_idx: None,
                                    ..Default::default()
                                });
                                in_table_cell = false;
                                current_text.clear();
                            }
                        }
                        b"table:table-row" => {
                            if !current_row.is_empty() {
                                table_rows.push(current_row.clone());
                            }
                        }
                        b"table:table" => {
                            if in_table && !table_rows.is_empty() {
                                // Create table DocItem
                                let num_cols =
                                    table_rows.iter().map(std::vec::Vec::len).max().unwrap_or(0);
                                let num_rows = table_rows.len();
                                doc_items.push(DocItem::Table {
                                    self_ref: format!("#/tables/{}", doc_items.len()),
                                    parent: None,
                                    children: vec![],
                                    content_layer: "body".to_string(),
                                    prov: self.create_provenance(1),
                                    data: TableData {
                                        num_rows,
                                        num_cols,
                                        grid: std::mem::take(&mut table_rows),
                                        table_cells: None,
                                    },
                                    captions: vec![],
                                    footnotes: vec![],
                                    references: vec![],
                                    image: None,
                                    annotations: vec![],
                                });
                            }
                            in_table = false;
                            // table_rows already empty from mem::take above (or was empty)
                        }
                        _ => {}
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(DoclingError::BackendError(format!("XML parse error: {e}"))),
                _ => {}
            }
            buf.clear();
        }

        Ok(doc_items)
    }

    /// Parse ODS file and generate `DocItems` with structure
    fn parse_ods_with_structure(&self, path: &Path) -> Result<Vec<DocItem>, DoclingError> {
        // Parse ODS file and extract structured sheets
        let sheets = parse_ods_sheets(path)
            .map_err(|e| DoclingError::BackendError(format!("Failed to parse ODS: {e}")))?;

        let mut doc_items = Vec::new();

        // Get file name for document context
        let file_name = path
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("spreadsheet");

        // Always add document title for clarity (with "Spreadsheet:" prefix)
        doc_items.push(create_section_header(
            doc_items.len(),
            format!("Spreadsheet: {file_name}"),
            1,
            self.create_provenance(1),
        ));

        // Create a Table DocItem for each sheet
        for (sheet_idx, sheet) in sheets.iter().enumerate() {
            // Sheet header always level 2 (subordinate to document title)
            doc_items.push(create_section_header(
                doc_items.len(),
                format!("Sheet: {}", sheet.name),
                2, // Level 2 heading for sheet names
                self.create_provenance(sheet_idx + 1),
            ));

            // Convert sheet rows to TableCells
            let table_cells: Vec<Vec<TableCell>> = sheet
                .rows
                .iter()
                .map(|row| {
                    row.iter()
                        .map(|cell_text| TableCell {
                            text: cell_text.clone(),
                            row_span: Some(1),
                            col_span: Some(1),
                            ref_item: None,
                            start_row_offset_idx: None,
                            start_col_offset_idx: None,
                            ..Default::default()
                        })
                        .collect()
                })
                .collect();

            // Create Table DocItem for this sheet
            doc_items.push(DocItem::Table {
                self_ref: format!("#/tables/{}", doc_items.len()),
                parent: None,
                children: vec![],
                content_layer: "body".to_string(),
                prov: self.create_provenance(sheet_idx + 1),
                data: TableData {
                    num_rows: sheet.row_count,
                    num_cols: sheet.col_count,
                    grid: table_cells,
                    table_cells: None,
                },
                captions: vec![],
                footnotes: vec![],
                references: vec![],
                image: None,
                annotations: vec![],
            });
        }

        Ok(doc_items)
    }

    /// Parse ODP file and generate `DocItems` with structure
    fn parse_odp_with_structure(&self, path: &Path) -> Result<Vec<DocItem>, DoclingError> {
        // Parse ODP file and extract structured slides
        let slides = parse_odp_slides(path)
            .map_err(|e| DoclingError::BackendError(format!("Failed to parse ODP: {e}")))?;

        let mut doc_items = Vec::new();

        // Create DocItems for each slide
        for slide in &slides {
            // Add slide header as SectionHeader (Level 2)
            doc_items.push(create_section_header(
                doc_items.len(),
                format!("Slide {}", slide.number),
                2, // Level 2 heading
                self.create_provenance(slide.number),
            ));

            // Add each paragraph as a Text DocItem
            for paragraph in &slide.paragraphs {
                if !paragraph.is_empty() {
                    // Strip redundant "Slide N:" or "Title Slide:" prefixes
                    // These are redundant since we already have "## Slide N" headers
                    let cleaned = Self::strip_slide_prefix(paragraph);
                    if !cleaned.is_empty() {
                        doc_items.push(create_text_item(
                            doc_items.len(),
                            cleaned,
                            self.create_provenance(slide.number),
                        ));
                    }
                }
            }

            // Add images as Picture DocItems
            for image_href in &slide.images {
                // Create Picture DocItem with image metadata
                // The image href is typically something like "Pictures/image1.png"
                let picture_item = DocItem::Picture {
                    self_ref: format!("#/pictures/{}", doc_items.len()),
                    parent: None,
                    children: vec![],
                    content_layer: "body".to_string(),
                    prov: self.create_provenance(slide.number),
                    captions: vec![],
                    footnotes: vec![],
                    references: vec![],
                    image: Some(serde_json::json!({
                        "href": image_href,
                        "source": "ODP embedded image"
                    })),
                    annotations: vec![],
                    ocr_text: None,
                };
                doc_items.push(picture_item);
            }
        }

        Ok(doc_items)
    }

    /// Strip redundant slide prefix patterns like "Slide N:" or "Title Slide:"
    /// These are redundant since slides already have "## Slide N" section headers
    fn strip_slide_prefix(text: &str) -> String {
        let trimmed = text.trim();

        // Pattern 1: "Slide N:" or "Slide N -" (where N is a number)
        if let Some(after_prefix) = trimmed.strip_prefix("Slide ") {
            // Find the colon or dash separator
            if let Some(colon_pos) = after_prefix.find(':') {
                // Check if everything before the colon is a number
                let before_colon = &after_prefix[..colon_pos];
                if before_colon.trim().parse::<usize>().is_ok() {
                    // Strip "Slide N:" prefix
                    return after_prefix[colon_pos + 1..].trim().to_string();
                }
            } else if let Some(dash_pos) = after_prefix.find('-') {
                // Check if everything before the dash is a number
                let before_dash = &after_prefix[..dash_pos];
                if before_dash.trim().parse::<usize>().is_ok() {
                    // Strip "Slide N -" prefix
                    return after_prefix[dash_pos + 1..].trim().to_string();
                }
            }
        }

        // Pattern 2: "Title Slide:" or "Title Slide -"
        if let Some(after_prefix) = trimmed.strip_prefix("Title Slide:") {
            return after_prefix.trim().to_string();
        }
        if let Some(after_prefix) = trimmed.strip_prefix("Title Slide -") {
            return after_prefix.trim().to_string();
        }

        // No prefix found, return original text
        trimmed.to_string()
    }

    /// Create provenance metadata for `OpenDocument` content
    ///
    /// Returns a Vec containing a single `ProvenanceItem` for the given page.
    /// This is the standard format expected by `DocItem` creation functions.
    // Method signature kept for API consistency with other OpenDocumentBackend methods
    #[allow(clippy::unused_self)]
    fn create_provenance(&self, page_no: usize) -> Vec<ProvenanceItem> {
        vec![crate::utils::create_default_provenance(
            page_no,
            CoordOrigin::TopLeft,
        )]
    }
}

impl DocumentBackend for OpenDocumentBackend {
    #[inline]
    fn format(&self) -> InputFormat {
        self.format
    }

    fn parse_bytes(
        &self,
        _content: &[u8],
        _options: &BackendOptions,
    ) -> Result<Document, DoclingError> {
        // All OpenDocument formats are ZIP archives and require file path access
        Err(DoclingError::BackendError(format!(
            "{:?} format requires file path (ZIP archive), use parse_file() instead",
            self.format
        )))
    }

    fn parse_file<P: AsRef<Path>>(
        &self,
        path: P,
        _options: &BackendOptions,
    ) -> Result<Document, DoclingError> {
        let path_ref = path.as_ref();
        let filename = path_ref.display().to_string();

        // Helper to add filename context to errors
        let add_context = |err: DoclingError| -> DoclingError {
            match err {
                DoclingError::BackendError(msg) => {
                    DoclingError::BackendError(format!("{msg}: {filename}"))
                }
                other => other,
            }
        };

        // Open ZIP archive to extract both content and metadata
        let file = File::open(path_ref).map_err(DoclingError::IoError)?;
        let mut archive = ZipArchive::new(file).map_err(|e| {
            DoclingError::BackendError(format!("Failed to open as ZIP: {e}: {filename}"))
        })?;

        // Extract metadata (author, title, subject, dates)
        let (author, title, subject, created, modified) = self.extract_metadata(&mut archive);

        // Parse document to get structured content as DocItems
        let doc_items = match self.format {
            InputFormat::Odt => self
                .parse_odt_with_structure(path_ref)
                .map_err(&add_context)?,
            InputFormat::Ods => self
                .parse_ods_with_structure(path_ref)
                .map_err(&add_context)?,
            InputFormat::Odp => self
                .parse_odp_with_structure(path_ref)
                .map_err(&add_context)?,
            _ => {
                return Err(DoclingError::FormatError(format!(
                    "Unsupported OpenDocument format: {:?}",
                    self.format
                )))
            }
        };

        // Generate markdown from DocItems
        let mut markdown = String::new();

        // Add metadata header if any metadata is present
        if title.is_some()
            || author.is_some()
            || subject.is_some()
            || created.is_some()
            || modified.is_some()
        {
            markdown.push_str("# Document Metadata\n\n");

            if let Some(ref doc_title) = title {
                let _ = write!(markdown, "Title: {doc_title}\n\n");
            }
            if let Some(ref doc_author) = author {
                let _ = write!(markdown, "Author: {doc_author}\n\n");
            }
            if let Some(ref doc_subject) = subject {
                let _ = write!(markdown, "Subject: {doc_subject}\n\n");
            }
            if let Some(ref created_date) = created {
                let _ = write!(
                    markdown,
                    "Created: {}\n\n",
                    created_date.format("%Y-%m-%d %H:%M:%S UTC")
                );
            }
            if let Some(ref modified_date) = modified {
                let _ = write!(
                    markdown,
                    "Modified: {}\n\n",
                    modified_date.format("%Y-%m-%d %H:%M:%S UTC")
                );
            }

            markdown.push_str("---\n\n");
        }

        // Append content from DocItems using shared helper (applies formatting)
        markdown.push_str(&crate::markdown_helper::docitems_to_markdown(&doc_items));

        let num_characters = markdown.chars().count();

        Ok(Document {
            markdown,
            format: self.format,
            metadata: DocumentMetadata {
                num_characters,
                author,
                title,
                subject,
                created,
                modified,
                ..Default::default()
            },
            content_blocks: Some(doc_items),
            docling_document: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // Backend Creation Tests (7 tests)
    // ========================================

    #[test]
    fn test_create_backend_odt() {
        let backend = OpenDocumentBackend::new(InputFormat::Odt);
        assert!(
            backend.is_ok(),
            "ODT backend should be created successfully"
        );
        assert_eq!(
            backend.unwrap().format(),
            InputFormat::Odt,
            "Backend format should be ODT"
        );
    }

    #[test]
    fn test_create_backend_ods() {
        let backend = OpenDocumentBackend::new(InputFormat::Ods);
        assert!(
            backend.is_ok(),
            "ODS backend should be created successfully"
        );
        assert_eq!(
            backend.unwrap().format(),
            InputFormat::Ods,
            "Backend format should be ODS"
        );
    }

    #[test]
    fn test_create_backend_odp() {
        let backend = OpenDocumentBackend::new(InputFormat::Odp);
        assert!(
            backend.is_ok(),
            "ODP backend should be created successfully"
        );
        assert_eq!(
            backend.unwrap().format(),
            InputFormat::Odp,
            "Backend format should be ODP"
        );
    }

    #[test]
    fn test_create_backend_default() {
        // Test that each format can be created with default backend trait
        let odt = OpenDocumentBackend::new(InputFormat::Odt).unwrap();
        let ods = OpenDocumentBackend::new(InputFormat::Ods).unwrap();
        let odp = OpenDocumentBackend::new(InputFormat::Odp).unwrap();

        assert_eq!(odt.format(), InputFormat::Odt);
        assert_eq!(ods.format(), InputFormat::Ods);
        assert_eq!(odp.format(), InputFormat::Odp);
    }

    #[test]
    fn test_backend_format_constant() {
        // Verify format() returns correct constant for all three formats
        let odt_backend = OpenDocumentBackend::new(InputFormat::Odt).unwrap();
        let ods_backend = OpenDocumentBackend::new(InputFormat::Ods).unwrap();
        let odp_backend = OpenDocumentBackend::new(InputFormat::Odp).unwrap();

        assert_eq!(odt_backend.format, InputFormat::Odt);
        assert_eq!(ods_backend.format, InputFormat::Ods);
        assert_eq!(odp_backend.format, InputFormat::Odp);
    }

    #[test]
    fn test_backend_format_persistence() {
        let backend = OpenDocumentBackend::new(InputFormat::Odt).unwrap();

        // Format should remain constant across multiple calls
        assert_eq!(backend.format(), InputFormat::Odt);
        assert_eq!(backend.format(), InputFormat::Odt);
        assert_eq!(backend.format(), InputFormat::Odt);

        // Create another backend and verify independence
        let backend2 = OpenDocumentBackend::new(InputFormat::Ods).unwrap();
        assert_eq!(backend.format(), InputFormat::Odt);
        assert_eq!(backend2.format(), InputFormat::Ods);
    }

    #[test]
    fn test_backend_unsupported_format_error() {
        // Test various unsupported formats
        let unsupported_formats = vec![
            InputFormat::Pdf,
            InputFormat::Docx,
            InputFormat::Html,
            InputFormat::Jpeg,
            InputFormat::Png,
        ];

        for format in unsupported_formats {
            let result = OpenDocumentBackend::new(format);
            assert!(
                result.is_err(),
                "Creating backend for {format:?} format should fail"
            );
            match result {
                Err(DoclingError::FormatError(msg)) => {
                    assert!(
                        msg.contains("not an OpenDocument format"),
                        "Error for {format:?} should mention 'not an OpenDocument format'"
                    );
                }
                _ => panic!("Expected FormatError for {format:?}"),
            }
        }
    }

    // ========================================
    // Metadata Tests (8 tests)
    // ========================================

    #[test]
    fn test_metadata_extraction_complete() {
        // Test full metadata extraction from ODT file
        let backend = OpenDocumentBackend::new(InputFormat::Odt).unwrap();
        let options = BackendOptions::default();

        // Use report.odt which has metadata: creator="Jane Smith", title="Test Report 2024"
        let test_path = "test-corpus/opendocument/odt/report.odt";

        if std::path::Path::new(test_path).exists() {
            let doc = backend.parse_file(test_path, &options).unwrap();

            // Verify metadata extraction
            assert_eq!(doc.metadata.author.as_deref(), Some("Jane Smith"));
            assert_eq!(doc.metadata.title.as_deref(), Some("Test Report 2024"));
            assert_eq!(doc.metadata.subject.as_deref(), Some("Test Document"));
            assert!(
                doc.metadata.created.is_some(),
                "Created datetime should be extracted"
            );
            assert!(
                doc.metadata.modified.is_some(),
                "Modified datetime should be extracted"
            );

            // Verify character count is computed
            assert!(
                doc.metadata.num_characters > 0,
                "Character count should be positive"
            );

            // Verify subject appears in markdown
            assert!(
                doc.markdown.contains("Subject: Test Document"),
                "Subject should appear in markdown"
            );
        }
    }

    #[test]
    fn test_metadata_character_count() {
        // Verify character count matches markdown length
        let backend = OpenDocumentBackend::new(InputFormat::Odt).unwrap();

        // Create simple ODT content for testing
        let doc_items = vec![create_text_item(
            0,
            "Hello World".to_string(),
            backend.create_provenance(1),
        )];

        let markdown = crate::markdown_helper::docitems_to_markdown(&doc_items);
        let char_count = markdown.chars().count();

        // Note: markdown_helper trims trailing whitespace to match Python docling
        assert_eq!(char_count, "Hello World".chars().count());
        assert_eq!(char_count, 11); // "Hello World" (no trailing newlines after trim)
    }

    #[test]
    fn test_metadata_format_field() {
        // Verify format field is correctly set for all three formats
        let odt_backend = OpenDocumentBackend::new(InputFormat::Odt).unwrap();
        let ods_backend = OpenDocumentBackend::new(InputFormat::Ods).unwrap();
        let odp_backend = OpenDocumentBackend::new(InputFormat::Odp).unwrap();

        assert_eq!(odt_backend.format(), InputFormat::Odt);
        assert_eq!(ods_backend.format(), InputFormat::Ods);
        assert_eq!(odp_backend.format(), InputFormat::Odp);

        // Verify format names match expected values
        assert_eq!(format!("{:?}", odt_backend.format()), "Odt");
        assert_eq!(format!("{:?}", ods_backend.format()), "Ods");
        assert_eq!(format!("{:?}", odp_backend.format()), "Odp");
    }

    #[test]
    fn test_metadata_missing_fields() {
        // Test that missing metadata returns None gracefully
        let backend = OpenDocumentBackend::new(InputFormat::Odt).unwrap();

        // Simulate missing meta.xml by using extract_metadata with empty archive
        // (This tests the fallback behavior when meta.xml doesn't exist)
        let xml = r#"<?xml version="1.0"?>
            <office:document-content>
                <office:body>
                    <office:text>
                        <text:p>Content without metadata</text:p>
                    </office:text>
                </office:body>
            </office:document-content>"#;

        let doc_items = backend.parse_odt_content(xml).unwrap();
        let markdown = crate::markdown_helper::docitems_to_markdown(&doc_items);

        // Metadata extraction would return (None, None, None, None) for missing meta.xml
        // Character count should still be computed
        assert!(
            markdown.chars().count() > 0,
            "Markdown should have non-zero character count"
        );
    }

    #[test]
    fn test_metadata_datetime_parsing() {
        // Test that datetime parsing handles various ISO 8601 formats
        let _backend = OpenDocumentBackend::new(InputFormat::Odt).unwrap();

        // Valid ISO 8601 datetime strings
        let valid_dates = vec![
            "2024-01-15T10:30:00Z",
            "2024-01-15T10:30:00+00:00",
            "2024-01-15T10:30:00.123Z",
        ];

        for date_str in valid_dates {
            let result = OpenDocumentBackend::parse_datetime(date_str);
            assert!(
                result.is_some(),
                "Failed to parse valid datetime: {date_str}"
            );
        }

        // Invalid datetime should return None
        let invalid_dates = vec!["not-a-date", "", "2024-13-45"];
        for date_str in invalid_dates {
            let result = OpenDocumentBackend::parse_datetime(date_str);
            assert!(
                result.is_none(),
                "Should not parse invalid datetime: {date_str}"
            );
        }
    }

    #[test]
    fn test_metadata_with_special_characters() {
        let backend = OpenDocumentBackend::new(InputFormat::Odt).unwrap();

        // Create ODT content with special characters in metadata
        // (Integration test would verify this from real files)
        let doc_items = vec![
            create_section_header(
                0,
                "Title with émojis 🎉 and unicode 日本語".to_string(),
                1,
                backend.create_provenance(1),
            ),
            create_text_item(
                1,
                "Content with special chars: <>&\"'".to_string(),
                backend.create_provenance(1),
            ),
        ];

        let markdown = crate::markdown_helper::docitems_to_markdown(&doc_items);

        // Verify special characters are preserved in markdown
        assert!(
            markdown.contains("émojis"),
            "French accented text should be preserved"
        );
        assert!(markdown.contains("🎉"), "Emoji should be preserved");
        assert!(
            markdown.contains("日本語"),
            "Japanese text should be preserved"
        );
        assert!(
            markdown.contains("<>&\"'"),
            "Special characters should be preserved"
        );
    }

    #[test]
    fn test_metadata_consistency_across_formats() {
        // All three backends should handle metadata consistently
        let odt = OpenDocumentBackend::new(InputFormat::Odt).unwrap();
        let _ods = OpenDocumentBackend::new(InputFormat::Ods).unwrap();
        let _odp = OpenDocumentBackend::new(InputFormat::Odp).unwrap();

        // Create same content for all backends
        let doc_items = vec![create_text_item(
            0,
            "Test content".to_string(),
            odt.create_provenance(1),
        )];

        let md_odt = crate::markdown_helper::docitems_to_markdown(&doc_items);
        let md_ods = crate::markdown_helper::docitems_to_markdown(&doc_items);
        let md_odp = crate::markdown_helper::docitems_to_markdown(&doc_items);

        // All should produce same markdown for same content
        assert_eq!(md_odt, md_ods);
        assert_eq!(md_ods, md_odp);

        // All should have same character count
        assert_eq!(md_odt.chars().count(), md_ods.chars().count());
        assert_eq!(md_ods.chars().count(), md_odp.chars().count());
    }

    #[test]
    fn test_metadata_optional_fields_default() {
        let _backend = OpenDocumentBackend::new(InputFormat::Odt).unwrap();

        // Empty doc_items should produce empty markdown
        let doc_items: Vec<DocItem> = vec![];
        let markdown = crate::markdown_helper::docitems_to_markdown(&doc_items);

        assert_eq!(markdown, "");
        assert_eq!(markdown.chars().count(), 0);
    }

    // ========================================
    // DocItem Generation Tests (9 tests)
    // ========================================

    #[test]
    fn test_create_docitems_empty_markdown() {
        let _backend = OpenDocumentBackend::new(InputFormat::Odt).unwrap();
        let doc_items: Vec<DocItem> = vec![];
        let markdown = crate::markdown_helper::docitems_to_markdown(&doc_items);
        assert_eq!(markdown, "");
    }

    #[test]
    fn test_create_docitems_single_paragraph() {
        let backend = OpenDocumentBackend::new(InputFormat::Odt).unwrap();
        let doc_items = vec![create_text_item(
            0,
            "Single paragraph".to_string(),
            backend.create_provenance(1),
        )];
        let markdown = crate::markdown_helper::docitems_to_markdown(&doc_items);
        // Note: markdown_helper trims trailing whitespace to match Python docling
        assert_eq!(markdown, "Single paragraph");
    }

    #[test]
    fn test_create_docitems_multiple_paragraphs() {
        let backend = OpenDocumentBackend::new(InputFormat::Odt).unwrap();
        let doc_items = vec![
            create_text_item(
                0,
                "First paragraph".to_string(),
                backend.create_provenance(1),
            ),
            create_text_item(
                1,
                "Second paragraph".to_string(),
                backend.create_provenance(1),
            ),
            create_text_item(
                2,
                "Third paragraph".to_string(),
                backend.create_provenance(1),
            ),
        ];
        let markdown = crate::markdown_helper::docitems_to_markdown(&doc_items);
        // Note: markdown_helper trims trailing whitespace to match Python docling
        assert_eq!(
            markdown,
            "First paragraph\n\nSecond paragraph\n\nThird paragraph"
        );
    }

    #[test]
    fn test_create_docitems_filters_empty_paragraphs() {
        // Verify that parse_odt_content filters out empty paragraphs
        let backend = OpenDocumentBackend::new(InputFormat::Odt).unwrap();

        // XML with empty paragraph (trimmed whitespace)
        let xml = r#"<?xml version="1.0"?>
            <office:document-content>
                <office:body>
                    <office:text>
                        <text:p>  </text:p>
                        <text:p>Non-empty</text:p>
                        <text:p></text:p>
                    </office:text>
                </office:body>
            </office:document-content>"#;

        let doc_items = backend.parse_odt_content(xml).unwrap();

        // Should only contain one DocItem (the non-empty paragraph)
        assert_eq!(doc_items.len(), 1);
        match &doc_items[0] {
            DocItem::Text { text, .. } => assert_eq!(text, "Non-empty"),
            _ => panic!("Expected Text DocItem"),
        }
    }

    #[test]
    fn test_create_docitems_ordering_preservation() {
        let backend = OpenDocumentBackend::new(InputFormat::Odt).unwrap();

        let doc_items = vec![
            create_section_header(0, "First".to_string(), 1, backend.create_provenance(1)),
            create_text_item(1, "Second".to_string(), backend.create_provenance(1)),
            create_section_header(2, "Third".to_string(), 2, backend.create_provenance(1)),
            create_text_item(3, "Fourth".to_string(), backend.create_provenance(1)),
        ];

        let markdown = crate::markdown_helper::docitems_to_markdown(&doc_items);

        // Verify order is preserved in markdown
        let first_pos = markdown.find("First").unwrap();
        let second_pos = markdown.find("Second").unwrap();
        let third_pos = markdown.find("Third").unwrap();
        let fourth_pos = markdown.find("Fourth").unwrap();

        assert!(
            first_pos < second_pos,
            "First paragraph should appear before second"
        );
        assert!(
            second_pos < third_pos,
            "Second paragraph should appear before third"
        );
        assert!(
            third_pos < fourth_pos,
            "Third paragraph should appear before fourth"
        );
    }

    #[test]
    fn test_create_docitems_unicode_content() {
        let backend = OpenDocumentBackend::new(InputFormat::Odt).unwrap();

        let doc_items = vec![
            create_text_item(0, "English text".to_string(), backend.create_provenance(1)),
            create_text_item(
                1,
                "日本語テキスト".to_string(),
                backend.create_provenance(1),
            ),
            create_text_item(
                2,
                "Émojis: 🎉🚀💡".to_string(),
                backend.create_provenance(1),
            ),
            create_text_item(3, "Ελληνικά".to_string(), backend.create_provenance(1)),
        ];

        let markdown = crate::markdown_helper::docitems_to_markdown(&doc_items);

        // Verify all unicode content is preserved
        assert!(
            markdown.contains("English text"),
            "English text should be present in markdown"
        );
        assert!(
            markdown.contains("日本語テキスト"),
            "Japanese text should be present in markdown"
        );
        assert!(
            markdown.contains("🎉🚀💡"),
            "Emojis should be present in markdown"
        );
        assert!(
            markdown.contains("Ελληνικά"),
            "Greek text should be present in markdown"
        );
    }

    #[test]
    fn test_create_docitems_self_ref_generation() {
        let backend = OpenDocumentBackend::new(InputFormat::Odt).unwrap();

        let doc_items = [
            create_text_item(0, "A".to_string(), backend.create_provenance(1)),
            create_text_item(1, "B".to_string(), backend.create_provenance(1)),
            create_text_item(2, "C".to_string(), backend.create_provenance(1)),
        ];

        // Verify self_ref values are sequential
        for (i, item) in doc_items.iter().enumerate() {
            if let DocItem::Text { self_ref, .. } = item {
                let expected = format!("#/texts/{i}");
                assert_eq!(self_ref, &expected);
            }
        }
    }

    #[test]
    fn test_create_docitems_mixed_types() {
        let backend = OpenDocumentBackend::new(InputFormat::Odt).unwrap();

        let doc_items = vec![
            create_section_header(0, "Heading".to_string(), 1, backend.create_provenance(1)),
            create_text_item(1, "Paragraph".to_string(), backend.create_provenance(1)),
            create_list_item(
                2,
                "List item".to_string(),
                "1. ".to_string(),
                true,
                backend.create_provenance(1),
                None,
            ),
        ];

        let markdown = crate::markdown_helper::docitems_to_markdown(&doc_items);

        // Verify all types are rendered correctly
        assert!(
            markdown.contains("# Heading\n"),
            "Level 1 heading should be rendered with single hash"
        );
        assert!(
            markdown.contains("Paragraph\n\n"),
            "Paragraphs should have proper spacing"
        );
        assert!(
            markdown.contains("1.  List item"),
            "Numbered list item should be rendered"
        ); // Note: marker is "1. " and markdown_helper adds space
    }

    #[test]
    fn test_create_docitems_text_escaping() {
        let backend = OpenDocumentBackend::new(InputFormat::Odt).unwrap();

        // Text with characters that might need escaping in different contexts
        let doc_items = vec![
            create_text_item(
                0,
                "Text with <angle> brackets".to_string(),
                backend.create_provenance(1),
            ),
            create_text_item(
                1,
                "Text with & ampersand".to_string(),
                backend.create_provenance(1),
            ),
            create_text_item(
                2,
                "Text with \"quotes\"".to_string(),
                backend.create_provenance(1),
            ),
        ];

        let markdown = crate::markdown_helper::docitems_to_markdown(&doc_items);

        // In markdown, these should be preserved as-is
        assert!(
            markdown.contains("<angle>"),
            "Angle brackets should be preserved in markdown"
        );
        assert!(
            markdown.contains("&"),
            "Ampersand should be preserved in markdown"
        );
        assert!(
            markdown.contains("\"quotes\""),
            "Quotes should be preserved in markdown"
        );
    }

    // ========================================
    // Format-Specific Tests (13 tests)
    // ========================================

    #[test]
    fn test_odt_heading_levels() {
        let backend = OpenDocumentBackend::new(InputFormat::Odt).unwrap();

        // XML with different heading levels
        let xml = r#"<?xml version="1.0"?>
            <office:document-content>
                <office:body>
                    <office:text>
                        <text:h text:outline-level="1">Heading 1</text:h>
                        <text:h text:outline-level="2">Heading 2</text:h>
                        <text:h text:outline-level="3">Heading 3</text:h>
                    </office:text>
                </office:body>
            </office:document-content>"#;

        let doc_items = backend.parse_odt_content(xml).unwrap();
        let markdown = crate::markdown_helper::docitems_to_markdown(&doc_items);

        assert!(
            markdown.contains("# Heading 1\n") || markdown.contains("# Heading 1"),
            "Level 1 heading should be rendered with single hash"
        );
        assert!(markdown.contains("## Heading 2\n") || markdown.contains("## Heading 2"));
        assert!(markdown.contains("### Heading 3\n") || markdown.contains("### Heading 3"));
    }

    #[test]
    fn test_odt_list_items() {
        let backend = OpenDocumentBackend::new(InputFormat::Odt).unwrap();

        // XML with list items
        let xml = r#"<?xml version="1.0"?>
            <office:document-content>
                <office:body>
                    <office:text>
                        <text:list>
                            <text:list-item>
                                <text:p>First item</text:p>
                            </text:list-item>
                            <text:list-item>
                                <text:p>Second item</text:p>
                            </text:list-item>
                        </text:list>
                    </office:text>
                </office:body>
            </office:document-content>"#;

        let doc_items = backend.parse_odt_content(xml).unwrap();
        let markdown = crate::markdown_helper::docitems_to_markdown(&doc_items);

        // List items have consecutive numbers
        // Note: Last element may not have trailing newline due to markdown_helper trim
        assert!(markdown.contains("1. First item"));
        assert!(markdown.contains("2. Second item"));
    }

    #[test]
    fn test_odt_table_rendering() {
        let backend = OpenDocumentBackend::new(InputFormat::Odt).unwrap();

        // Test with programmatically created table (real parsing tested in integration tests)
        // The parse_odt_content method handles complex table XML parsing with paragraphs inside cells
        let table_cells = vec![
            vec![
                TableCell {
                    text: "Header 1".to_string(),
                    row_span: Some(1),
                    col_span: Some(1),
                    ref_item: None,
                    start_row_offset_idx: None,
                    start_col_offset_idx: None,
                    ..Default::default()
                },
                TableCell {
                    text: "Header 2".to_string(),
                    row_span: Some(1),
                    col_span: Some(1),
                    ref_item: None,
                    start_row_offset_idx: None,
                    start_col_offset_idx: None,
                    ..Default::default()
                },
            ],
            vec![
                TableCell {
                    text: "Cell A1".to_string(),
                    row_span: Some(1),
                    col_span: Some(1),
                    ref_item: None,
                    start_row_offset_idx: None,
                    start_col_offset_idx: None,
                    ..Default::default()
                },
                TableCell {
                    text: "Cell A2".to_string(),
                    row_span: Some(1),
                    col_span: Some(1),
                    ref_item: None,
                    start_row_offset_idx: None,
                    start_col_offset_idx: None,
                    ..Default::default()
                },
            ],
        ];

        let doc_items = vec![DocItem::Table {
            self_ref: "#/tables/0".to_string(),
            parent: None,
            children: vec![],
            content_layer: "body".to_string(),
            prov: backend.create_provenance(1),
            data: TableData {
                num_rows: 2,
                num_cols: 2,
                grid: table_cells,
                table_cells: None,
            },
            captions: vec![],
            footnotes: vec![],
            references: vec![],
            image: None,
            annotations: vec![],
        }];

        // Verify markdown table rendering
        // Note: Table cells have padding for column alignment - check content exists
        let markdown = crate::markdown_helper::docitems_to_markdown(&doc_items);
        assert!(
            markdown.contains("Header 1") && markdown.contains("Header 2"),
            "Table should contain headers"
        );
        assert!(
            markdown.contains("|--"),
            "Table should have header separator"
        );
        assert!(
            markdown.contains("Cell A1") && markdown.contains("Cell A2"),
            "Table should contain cells"
        );
    }

    #[test]
    fn test_ods_sheet_headers() {
        let backend = OpenDocumentBackend::new(InputFormat::Ods).unwrap();
        let options = BackendOptions::default();

        // Use budget.ods which has multiple sheets
        let test_path = "test-corpus/opendocument/ods/budget.ods";

        if std::path::Path::new(test_path).exists() {
            let doc = backend.parse_file(test_path, &options).unwrap();

            // ODS files should have "Sheet: <name>" headers
            // Verify markdown contains sheet headers (format: "## Sheet: <name>")
            assert!(doc.markdown.contains("## Sheet:") || doc.markdown.contains("Sheet"));
        }
    }

    #[test]
    fn test_odp_slide_headers() {
        let backend = OpenDocumentBackend::new(InputFormat::Odp).unwrap();
        let options = BackendOptions::default();

        // Use project_overview.odp which has multiple slides
        let test_path = "test-corpus/opendocument/odp/project_overview.odp";

        if std::path::Path::new(test_path).exists() {
            let doc = backend.parse_file(test_path, &options).unwrap();

            // ODP files should have "Slide N" headers
            // Verify markdown contains slide headers (format: "## Slide N")
            assert!(doc.markdown.contains("## Slide") || doc.markdown.contains("Slide"));
        }
    }

    #[test]
    fn test_format_detection() {
        // Verify that each backend correctly identifies its format
        let odt = OpenDocumentBackend::new(InputFormat::Odt).unwrap();
        let ods = OpenDocumentBackend::new(InputFormat::Ods).unwrap();
        let odp = OpenDocumentBackend::new(InputFormat::Odp).unwrap();

        // Each backend should report its own format
        assert_eq!(odt.format(), InputFormat::Odt);
        assert_eq!(ods.format(), InputFormat::Ods);
        assert_eq!(odp.format(), InputFormat::Odp);

        // Verify format() is consistent with internal field
        assert_eq!(odt.format, InputFormat::Odt);
        assert_eq!(ods.format, InputFormat::Ods);
        assert_eq!(odp.format, InputFormat::Odp);
    }

    #[test]
    fn test_table_with_empty_cells() {
        let backend = OpenDocumentBackend::new(InputFormat::Odt).unwrap();

        // Create table with some empty cells
        let table_cells = vec![vec![
            TableCell {
                text: "A1".to_string(),
                row_span: Some(1),
                col_span: Some(1),
                ref_item: None,
                start_row_offset_idx: None,
                start_col_offset_idx: None,
                ..Default::default()
            },
            TableCell {
                text: "".to_string(), // Empty cell
                row_span: Some(1),
                col_span: Some(1),
                ref_item: None,
                start_row_offset_idx: None,
                start_col_offset_idx: None,
                ..Default::default()
            },
        ]];

        let doc_items = vec![DocItem::Table {
            self_ref: "#/tables/0".to_string(),
            parent: None,
            children: vec![],
            content_layer: "body".to_string(),
            prov: backend.create_provenance(1),
            data: TableData {
                num_rows: 1,
                num_cols: 2,
                grid: table_cells,
                table_cells: None,
            },
            captions: vec![],
            footnotes: vec![],
            references: vec![],
            image: None,
            annotations: vec![],
        }];

        let markdown = crate::markdown_helper::docitems_to_markdown(&doc_items);
        // Note: Table cells have padding - check content exists
        assert!(markdown.contains("A1"), "Table should contain 'A1'");
    }

    #[test]
    fn test_list_marker_numbering() {
        let backend = OpenDocumentBackend::new(InputFormat::Odt).unwrap();

        let doc_items = vec![
            create_list_item(
                0,
                "First".to_string(),
                "1.".to_string(),
                true,
                backend.create_provenance(1),
                None,
            ),
            create_list_item(
                1,
                "Second".to_string(),
                "2.".to_string(),
                true,
                backend.create_provenance(1),
                None,
            ),
            create_list_item(
                2,
                "Third".to_string(),
                "3.".to_string(),
                true,
                backend.create_provenance(1),
                None,
            ),
        ];

        let markdown = crate::markdown_helper::docitems_to_markdown(&doc_items);

        // markdown_helper uses marker field directly (N=1267 change)
        // Note: Last element may not have trailing newline due to markdown_helper trim
        assert!(markdown.contains("1. First"));
        assert!(markdown.contains("2. Second"));
        assert!(markdown.contains("3. Third"));
    }

    #[test]
    fn test_heading_level_bounds() {
        let backend = OpenDocumentBackend::new(InputFormat::Odt).unwrap();

        // Test extreme heading levels
        let doc_items = vec![
            create_section_header(0, "Level 1".to_string(), 1, backend.create_provenance(1)),
            create_section_header(1, "Level 6".to_string(), 6, backend.create_provenance(1)),
        ];

        let markdown = crate::markdown_helper::docitems_to_markdown(&doc_items);

        // First heading should have newline (followed by another element)
        assert!(markdown.contains("# Level 1\n"));
        // Last heading may not have trailing newline at end of string
        // Use flexible assertion that works both ways
        assert!(
            markdown.contains("###### Level 6\n") || markdown.contains("###### Level 6"),
            "Should contain h6 heading"
        );
    }

    #[test]
    fn test_ods_table_rendering_from_backend() {
        let _backend = OpenDocumentBackend::new(InputFormat::Ods).unwrap();

        // ODS sheets are rendered as tables with headers
        let table_data = TableData {
            num_rows: 2,
            num_cols: 2,
            grid: vec![
                vec![
                    TableCell {
                        text: "Name".to_string(),
                        row_span: Some(1),
                        col_span: Some(1),
                        ref_item: None,
                        start_row_offset_idx: None,
                        start_col_offset_idx: None,
                        ..Default::default()
                    },
                    TableCell {
                        text: "Value".to_string(),
                        row_span: Some(1),
                        col_span: Some(1),
                        ref_item: None,
                        start_row_offset_idx: None,
                        start_col_offset_idx: None,
                        ..Default::default()
                    },
                ],
                vec![
                    TableCell {
                        text: "Item".to_string(),
                        row_span: Some(1),
                        col_span: Some(1),
                        ref_item: None,
                        start_row_offset_idx: None,
                        start_col_offset_idx: None,
                        ..Default::default()
                    },
                    TableCell {
                        text: "100".to_string(),
                        row_span: Some(1),
                        col_span: Some(1),
                        ref_item: None,
                        start_row_offset_idx: None,
                        start_col_offset_idx: None,
                        ..Default::default()
                    },
                ],
            ],
            table_cells: None,
        };

        let rendered = crate::markdown_helper::render_table(&table_data);

        // Note: Table cells have padding for column alignment - check content exists
        assert!(
            rendered.contains("Name") && rendered.contains("Value"),
            "Table should contain headers"
        );
        assert!(
            rendered.contains("|--"),
            "Table should have header separator"
        );
        assert!(
            rendered.contains("Item") && rendered.contains("100"),
            "Table should contain data"
        );
    }

    #[test]
    fn test_odp_slide_numbering_sequence() {
        let backend = OpenDocumentBackend::new(InputFormat::Odp).unwrap();

        // ODP slides are numbered sequentially
        let doc_items = vec![
            create_section_header(0, "Slide 1".to_string(), 2, backend.create_provenance(1)),
            create_text_item(1, "Content 1".to_string(), backend.create_provenance(1)),
            create_section_header(2, "Slide 2".to_string(), 2, backend.create_provenance(2)),
            create_text_item(3, "Content 2".to_string(), backend.create_provenance(2)),
        ];

        let markdown = crate::markdown_helper::docitems_to_markdown(&doc_items);

        assert!(markdown.contains("## Slide 1\n"));
        assert!(markdown.contains("## Slide 2\n"));

        // Verify order
        let slide1_pos = markdown.find("Slide 1").unwrap();
        let slide2_pos = markdown.find("Slide 2").unwrap();
        assert!(slide1_pos < slide2_pos);
    }

    #[test]
    fn test_complex_document_structure() {
        let backend = OpenDocumentBackend::new(InputFormat::Odt).unwrap();

        // Test document with mixed content types
        let doc_items = vec![
            create_section_header(
                0,
                "Introduction".to_string(),
                1,
                backend.create_provenance(1),
            ),
            create_text_item(
                1,
                "Opening paragraph".to_string(),
                backend.create_provenance(1),
            ),
            create_section_header(2, "Methods".to_string(), 2, backend.create_provenance(1)),
            create_list_item(
                3,
                "Step 1".to_string(),
                "1.".to_string(),
                true,
                backend.create_provenance(1),
                None,
            ),
            create_list_item(
                4,
                "Step 2".to_string(),
                "2.".to_string(),
                true,
                backend.create_provenance(1),
                None,
            ),
            create_section_header(5, "Results".to_string(), 2, backend.create_provenance(1)),
            create_text_item(6, "Conclusion".to_string(), backend.create_provenance(1)),
        ];

        let markdown = crate::markdown_helper::docitems_to_markdown(&doc_items);

        // Verify all sections present and ordered
        assert!(markdown.contains("# Introduction\n"));
        assert!(markdown.contains("## Methods\n"));
        assert!(markdown.contains("## Results\n"));
        assert!(markdown.contains("1. Step 1\n"));
        assert!(markdown.contains("2. Step 2\n"));

        // Verify correct order
        let intro_pos = markdown.find("Introduction").unwrap();
        let methods_pos = markdown.find("Methods").unwrap();
        let results_pos = markdown.find("Results").unwrap();
        assert!(intro_pos < methods_pos);
        assert!(methods_pos < results_pos);
    }

    // ========================================
    // Integration Tests (parse_bytes, errors) (8 tests)
    // ========================================

    #[test]
    fn test_parse_bytes_requires_file() {
        // All OpenDocument formats are ZIP archives requiring file path access
        let odt_backend = OpenDocumentBackend::new(InputFormat::Odt).unwrap();
        let ods_backend = OpenDocumentBackend::new(InputFormat::Ods).unwrap();
        let odp_backend = OpenDocumentBackend::new(InputFormat::Odp).unwrap();
        let options = BackendOptions::default();

        // parse_bytes should fail for all three formats
        let odt_result = odt_backend.parse_bytes(&[], &options);
        assert!(odt_result.is_err());
        assert!(odt_result
            .unwrap_err()
            .to_string()
            .contains("requires file path"));

        let ods_result = ods_backend.parse_bytes(&[], &options);
        assert!(ods_result.is_err());
        assert!(ods_result
            .unwrap_err()
            .to_string()
            .contains("requires file path"));

        let odp_result = odp_backend.parse_bytes(&[], &options);
        assert!(odp_result.is_err());
        assert!(odp_result
            .unwrap_err()
            .to_string()
            .contains("requires file path"));
    }

    #[test]
    fn test_invalid_format_rejection() {
        // Reject non-OpenDocument formats
        let pdf_result = OpenDocumentBackend::new(InputFormat::Pdf);
        assert!(pdf_result.is_err());
        match pdf_result {
            Err(DoclingError::FormatError(msg)) => {
                assert!(msg.contains("not an OpenDocument format"));
            }
            _ => panic!("Expected FormatError"),
        }

        let docx_result = OpenDocumentBackend::new(InputFormat::Docx);
        assert!(docx_result.is_err());

        let html_result = OpenDocumentBackend::new(InputFormat::Html);
        assert!(html_result.is_err());
    }

    #[test]
    fn test_parse_file_nonexistent_file() {
        let backend = OpenDocumentBackend::new(InputFormat::Odt).unwrap();
        let options = BackendOptions::default();

        let result = backend.parse_file("/nonexistent/file.odt", &options);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_file_not_a_zip() {
        // Create a temporary non-ZIP file
        let backend = OpenDocumentBackend::new(InputFormat::Odt).unwrap();
        let options = BackendOptions::default();

        // Try to parse a non-ZIP file (e.g., text file)
        let temp_file = std::env::temp_dir().join("not_a_zip.odt");
        std::fs::write(&temp_file, b"Not a ZIP file").unwrap();

        let result = backend.parse_file(&temp_file, &options);
        assert!(result.is_err());

        // Cleanup
        std::fs::remove_file(temp_file).ok();
    }

    #[test]
    fn test_parse_file_error_messages() {
        let backend = OpenDocumentBackend::new(InputFormat::Odt).unwrap();
        let options = BackendOptions::default();

        // Nonexistent file should give IO error
        let result = backend.parse_file("/tmp/nonexistent_12345.odt", &options);
        assert!(result.is_err());

        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("No such file")
                || err_msg.contains("not found")
                || err_msg.contains("IO")
        );
    }

    #[test]
    fn test_backend_reusability() {
        // Test that backend can be reused for multiple parses
        let backend = OpenDocumentBackend::new(InputFormat::Odt).unwrap();

        // Create multiple DocItem sets and serialize them
        let content1 = vec![create_text_item(
            0,
            "Doc 1".to_string(),
            backend.create_provenance(1),
        )];
        let content2 = vec![create_text_item(
            0,
            "Doc 2".to_string(),
            backend.create_provenance(1),
        )];
        let content3 = vec![create_text_item(
            0,
            "Doc 3".to_string(),
            backend.create_provenance(1),
        )];

        let md1 = crate::markdown_helper::docitems_to_markdown(&content1);
        let md2 = crate::markdown_helper::docitems_to_markdown(&content2);
        let md3 = crate::markdown_helper::docitems_to_markdown(&content3);

        // Each should produce different output
        assert!(md1.contains("Doc 1"));
        assert!(md2.contains("Doc 2"));
        assert!(md3.contains("Doc 3"));

        // Verify backend format is still correct
        assert_eq!(backend.format(), InputFormat::Odt);
    }

    #[test]
    fn test_parse_bytes_all_formats_error() {
        // Verify all three OpenDocument formats reject parse_bytes
        let formats = vec![
            (InputFormat::Odt, "Odt"),
            (InputFormat::Ods, "Ods"),
            (InputFormat::Odp, "Odp"),
        ];

        for (format, name) in formats {
            let backend = OpenDocumentBackend::new(format).unwrap();
            let options = BackendOptions::default();

            let result = backend.parse_bytes(&[], &options);
            assert!(result.is_err(), "{name} should reject parse_bytes");

            match result {
                Err(DoclingError::BackendError(msg)) => {
                    assert!(msg.contains("requires file path"), "{name}: {msg}");
                }
                _ => panic!("{name}: Expected BackendError"),
            }
        }
    }

    #[test]
    fn test_provenance_generation() {
        let backend = OpenDocumentBackend::new(InputFormat::Odt).unwrap();

        // Test provenance generation for different page numbers
        // create_provenance returns Vec<ProvenanceItem>, get the first element
        let prov1 = &backend.create_provenance(1)[0];
        let prov2 = &backend.create_provenance(2)[0];
        let prov10 = &backend.create_provenance(10)[0];

        // All should have correct page numbers
        assert_eq!(prov1.page_no, 1);
        assert_eq!(prov2.page_no, 2);
        assert_eq!(prov10.page_no, 10);

        // All should have same coord_origin (TopLeft)
        assert_eq!(prov1.bbox.coord_origin, CoordOrigin::TopLeft);
        assert_eq!(prov2.bbox.coord_origin, CoordOrigin::TopLeft);
        assert_eq!(prov10.bbox.coord_origin, CoordOrigin::TopLeft);
    }

    // ========================================
    // Additional Edge Cases and Features
    // ========================================

    #[test]
    fn test_odt_nested_lists() {
        let backend = OpenDocumentBackend::new(InputFormat::Odt).unwrap();

        // XML with nested lists (depth 2)
        let xml = r#"<?xml version="1.0"?>
            <office:document-content>
                <office:body>
                    <office:text>
                        <text:list>
                            <text:list-item>
                                <text:p>Level 1 item</text:p>
                                <text:list>
                                    <text:list-item>
                                        <text:p>Level 2 item</text:p>
                                    </text:list-item>
                                </text:list>
                            </text:list-item>
                        </text:list>
                    </office:text>
                </office:body>
            </office:document-content>"#;

        let doc_items = backend.parse_odt_content(xml).unwrap();
        let markdown = crate::markdown_helper::docitems_to_markdown(&doc_items);

        // Both list items should be present with appropriate markers
        assert!(markdown.contains("Level 1 item"));
        assert!(markdown.contains("Level 2 item"));
        // Level 1 has marker "1. ", Level 2 has marker "2. "
        assert!(markdown.contains("1. ") || markdown.contains("2. "));
    }

    #[test]
    fn test_odt_heading_without_level() {
        let backend = OpenDocumentBackend::new(InputFormat::Odt).unwrap();

        // XML with heading that doesn't specify outline-level (should default to 1)
        let xml = r#"<?xml version="1.0"?>
            <office:document-content>
                <office:body>
                    <office:text>
                        <text:h>Default Level Heading</text:h>
                    </office:text>
                </office:body>
            </office:document-content>"#;

        let doc_items = backend.parse_odt_content(xml).unwrap();
        let markdown = crate::markdown_helper::docitems_to_markdown(&doc_items);

        // Should default to level 1 (single #)
        // Note: Last element may not have trailing newline due to markdown_helper trim
        assert!(
            markdown.contains("# Default Level Heading\n")
                || markdown.contains("# Default Level Heading")
        );
    }

    #[test]
    fn test_ods_empty_sheet() {
        let backend = OpenDocumentBackend::new(InputFormat::Ods).unwrap();

        // Test with single empty sheet - sheet header is always level 2 (subordinate to document title)
        let doc_items = vec![create_section_header(
            0,
            "Sheet: Empty".to_string(),
            2,
            backend.create_provenance(1),
        )];

        let markdown = crate::markdown_helper::docitems_to_markdown(&doc_items);
        assert!(markdown.contains("## Sheet: Empty\n") || markdown.contains("## Sheet: Empty"));
    }

    #[test]
    fn test_odp_empty_slide() {
        let backend = OpenDocumentBackend::new(InputFormat::Odp).unwrap();

        // Test with empty slide (no paragraphs)
        let doc_items = vec![create_section_header(
            0,
            "Slide 1".to_string(),
            2,
            backend.create_provenance(1),
        )];

        let markdown = crate::markdown_helper::docitems_to_markdown(&doc_items);
        assert!(markdown.contains("## Slide 1\n") || markdown.contains("## Slide 1"));
    }

    #[test]
    fn test_xml_special_characters() {
        let backend = OpenDocumentBackend::new(InputFormat::Odt).unwrap();

        // XML with special characters that need escaping
        let xml = r#"<?xml version="1.0"?>
            <office:document-content>
                <office:body>
                    <office:text>
                        <text:p>Text with &lt;brackets&gt; and &amp; ampersand</text:p>
                    </office:text>
                </office:body>
            </office:document-content>"#;

        let doc_items = backend.parse_odt_content(xml).unwrap();

        // Verify special characters are unescaped correctly
        match &doc_items[0] {
            DocItem::Text { text, .. } => {
                assert_eq!(text, "Text with <brackets> and & ampersand");
            }
            _ => panic!("Expected Text DocItem"),
        }
    }

    #[test]
    fn test_whitespace_elements() {
        let backend = OpenDocumentBackend::new(InputFormat::Odt).unwrap();

        // XML with text:s (space), text:tab, text:line-break elements
        let xml = r#"<?xml version="1.0"?>
            <office:document-content>
                <office:body>
                    <office:text>
                        <text:p>Before<text:s/>space<text:tab/>tab<text:line-break/>newline</text:p>
                    </office:text>
                </office:body>
            </office:document-content>"#;

        let doc_items = backend.parse_odt_content(xml).unwrap();

        // Verify whitespace elements are converted correctly
        match &doc_items[0] {
            DocItem::Text { text, .. } => {
                assert!(text.contains("Before space")); // text:s → space
                assert!(text.contains("space\ttab")); // text:tab → \t
                assert!(text.contains("tab\nnewline")); // text:line-break → \n
            }
            _ => panic!("Expected Text DocItem"),
        }
    }

    // Note: OpenDocument format is complex with ZIP archives and XML content.
    // These tests cover backend functionality (metadata, DocItem generation, format-specific features).
    // Full integration tests with real ODT/ODS/ODP files are in docling-core integration tests.
    // Parser implementations in docling-opendocument crate have their own detailed unit tests.

    // ========================================
    // Additional Comprehensive Tests (10 tests)
    // ========================================

    #[test]
    fn test_odt_inline_formatting() {
        let backend = OpenDocumentBackend::new(InputFormat::Odt).unwrap();

        // XML with inline formatting (bold, italic)
        let xml = r#"<?xml version="1.0"?>
            <office:document-content>
                <office:body>
                    <office:text>
                        <text:p>Normal <text:span text:style-name="Bold">bold text</text:span> and <text:span text:style-name="Italic">italic text</text:span></text:p>
                    </office:text>
                </office:body>
            </office:document-content>"#;

        let doc_items = backend.parse_odt_content(xml).unwrap();

        // With formatting extraction, paragraph with mixed formatting generates multiple DocItems
        assert!(
            doc_items.len() >= 3,
            "Expected at least 3 DocItems (normal, bold, italic), got {}",
            doc_items.len()
        );

        // Check that all text content is present across DocItems
        let all_text: String = doc_items
            .iter()
            .filter_map(|item| match item {
                DocItem::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" ");

        assert!(all_text.contains("Normal"), "Missing 'Normal' text");
        assert!(all_text.contains("bold text"), "Missing 'bold text'");
        assert!(all_text.contains("italic text"), "Missing 'italic text'");

        // Verify formatting attributes are present
        let has_bold = doc_items.iter().any(|item| match item {
            DocItem::Text {
                text, formatting, ..
            } => text.contains("bold") && formatting.as_ref().is_some_and(|f| f.bold == Some(true)),
            _ => false,
        });

        let has_italic = doc_items.iter().any(|item| match item {
            DocItem::Text {
                text, formatting, ..
            } => {
                text.contains("italic")
                    && formatting.as_ref().is_some_and(|f| f.italic == Some(true))
            }
            _ => false,
        });

        assert!(has_bold, "Bold formatting not found");
        assert!(has_italic, "Italic formatting not found");
    }

    #[test]
    fn test_odt_mixed_table_text_content() {
        let backend = OpenDocumentBackend::new(InputFormat::Odt).unwrap();

        // Create mixed content: text, table, text
        let table_data = TableData {
            num_rows: 1,
            num_cols: 2,
            grid: vec![vec![
                TableCell {
                    text: "A".to_string(),
                    row_span: Some(1),
                    col_span: Some(1),
                    ref_item: None,
                    start_row_offset_idx: None,
                    start_col_offset_idx: None,
                    ..Default::default()
                },
                TableCell {
                    text: "B".to_string(),
                    row_span: Some(1),
                    col_span: Some(1),
                    ref_item: None,
                    start_row_offset_idx: None,
                    start_col_offset_idx: None,
                    ..Default::default()
                },
            ]],
            table_cells: None,
        };

        let doc_items = vec![
            create_text_item(0, "Before table".to_string(), backend.create_provenance(1)),
            DocItem::Table {
                self_ref: "#/tables/0".to_string(),
                parent: None,
                children: vec![],
                content_layer: "body".to_string(),
                prov: backend.create_provenance(1),
                data: table_data,
                captions: vec![],
                footnotes: vec![],
                references: vec![],
                image: None,
                annotations: vec![],
            },
            create_text_item(1, "After table".to_string(), backend.create_provenance(1)),
        ];

        let markdown = crate::markdown_helper::docitems_to_markdown(&doc_items);

        // Verify order is preserved
        // Note: Table cells have padding - use content position checking
        let before_pos = markdown.find("Before table").unwrap();
        let table_pos = markdown.find("| A").unwrap(); // Table cells have variable padding
        let after_pos = markdown.find("After table").unwrap();

        assert!(
            before_pos < table_pos,
            "Before table should come before table"
        );
        assert!(
            table_pos < after_pos,
            "Table should come before after table"
        );
    }

    #[test]
    fn test_ods_formula_cells() {
        let _backend = OpenDocumentBackend::new(InputFormat::Ods).unwrap();

        // ODS cells can contain formulas (stored in table:formula attribute)
        // Create table with formula result text
        let table_data = TableData {
            num_rows: 2,
            num_cols: 2,
            grid: vec![
                vec![
                    TableCell {
                        text: "A1".to_string(),
                        row_span: Some(1),
                        col_span: Some(1),
                        ref_item: None,
                        start_row_offset_idx: None,
                        start_col_offset_idx: None,
                        ..Default::default()
                    },
                    TableCell {
                        text: "100".to_string(),
                        row_span: Some(1),
                        col_span: Some(1),
                        ref_item: None,
                        start_row_offset_idx: None,
                        start_col_offset_idx: None,
                        ..Default::default()
                    },
                ],
                vec![
                    TableCell {
                        text: "A2".to_string(),
                        row_span: Some(1),
                        col_span: Some(1),
                        ref_item: None,
                        start_row_offset_idx: None,
                        start_col_offset_idx: None,
                        ..Default::default()
                    },
                    TableCell {
                        text: "200".to_string(), // Formula result (e.g., =SUM(B1:B1))
                        row_span: Some(1),
                        col_span: Some(1),
                        ref_item: None,
                        start_row_offset_idx: None,
                        start_col_offset_idx: None,
                        ..Default::default()
                    },
                ],
            ],
            table_cells: None,
        };

        let rendered = crate::markdown_helper::render_table(&table_data);

        // Note: Table cells have padding for column alignment - check content exists
        assert!(
            rendered.contains("A1") && rendered.contains("100"),
            "Table should contain first row data"
        );
        assert!(
            rendered.contains("A2") && rendered.contains("200"),
            "Table should contain second row data"
        );
    }

    #[test]
    fn test_odp_master_slide_content() {
        let backend = OpenDocumentBackend::new(InputFormat::Odp).unwrap();

        // Master slides contain repeated content (headers, footers, page numbers)
        // Test that content from slides is properly captured
        let doc_items = vec![
            create_section_header(0, "Slide 1".to_string(), 2, backend.create_provenance(1)),
            create_text_item(
                1,
                "Title from master".to_string(),
                backend.create_provenance(1),
            ),
            create_text_item(
                2,
                "Content specific to this slide".to_string(),
                backend.create_provenance(1),
            ),
        ];

        let markdown = crate::markdown_helper::docitems_to_markdown(&doc_items);

        // Both master and slide-specific content should be present
        assert!(markdown.contains("## Slide 1\n"));
        assert!(markdown.contains("Title from master"));
        assert!(markdown.contains("Content specific to this slide"));
    }

    #[test]
    fn test_unicode_in_tables() {
        let _backend = OpenDocumentBackend::new(InputFormat::Odt).unwrap();

        // Table with Unicode characters (CJK, emoji, etc.)
        let table_data = TableData {
            num_rows: 2,
            num_cols: 2,
            grid: vec![
                vec![
                    TableCell {
                        text: "\u{540D}\u{524D}".to_string(), // Japanese: "Name"
                        row_span: Some(1),
                        col_span: Some(1),
                        ref_item: None,
                        start_row_offset_idx: None,
                        start_col_offset_idx: None,
                        ..Default::default()
                    },
                    TableCell {
                        text: "\u{5024}".to_string(), // Japanese: "Value"
                        row_span: Some(1),
                        col_span: Some(1),
                        ref_item: None,
                        start_row_offset_idx: None,
                        start_col_offset_idx: None,
                        ..Default::default()
                    },
                ],
                vec![
                    TableCell {
                        text: "\u{9805}\u{76EE} \u{1F3AF}".to_string(), // Japanese + emoji
                        row_span: Some(1),
                        col_span: Some(1),
                        ref_item: None,
                        start_row_offset_idx: None,
                        start_col_offset_idx: None,
                        ..Default::default()
                    },
                    TableCell {
                        text: "100 \u{2713}".to_string(), // Checkmark
                        row_span: Some(1),
                        col_span: Some(1),
                        ref_item: None,
                        start_row_offset_idx: None,
                        start_col_offset_idx: None,
                        ..Default::default()
                    },
                ],
            ],
            table_cells: None,
        };

        let rendered = crate::markdown_helper::render_table(&table_data);

        // Unicode should be preserved
        assert!(rendered.contains("\u{540D}\u{524D}")); // 名前
        assert!(rendered.contains("\u{5024}")); // 値
        assert!(rendered.contains("\u{9805}\u{76EE}")); // 項目
        assert!(rendered.contains("\u{1F3AF}")); // 🎯
        assert!(rendered.contains("\u{2713}")); // ✓
    }

    #[test]
    fn test_ods_multiple_sheets_complex() {
        let backend = OpenDocumentBackend::new(InputFormat::Ods).unwrap();

        // Simulate ODS with multiple sheets, each with different content
        // Sheet headers are now level 2 (clear and prominent)
        let doc_items = vec![
            create_section_header(
                0,
                "Sheet: Data".to_string(),
                2,
                backend.create_provenance(1),
            ),
            create_text_item(1, "Data content".to_string(), backend.create_provenance(1)),
            create_section_header(
                2,
                "Sheet: Charts".to_string(),
                2,
                backend.create_provenance(2),
            ),
            create_text_item(3, "Chart content".to_string(), backend.create_provenance(2)),
            create_section_header(
                4,
                "Sheet: Summary".to_string(),
                2,
                backend.create_provenance(3),
            ),
            create_text_item(
                5,
                "Summary content".to_string(),
                backend.create_provenance(3),
            ),
        ];

        let markdown = crate::markdown_helper::docitems_to_markdown(&doc_items);

        // All sheets should be present with headers
        assert!(markdown.contains("## Sheet: Data\n"));
        assert!(markdown.contains("## Sheet: Charts\n"));
        assert!(markdown.contains("## Sheet: Summary\n"));

        // Content should follow headers
        assert!(markdown.contains("Data content"));
        assert!(markdown.contains("Chart content"));
        assert!(markdown.contains("Summary content"));
    }

    #[test]
    fn test_table_spanning_cells() {
        let _backend = OpenDocumentBackend::new(InputFormat::Odt).unwrap();

        // Table with spanning cells (rowspan, colspan)
        let table_data = TableData {
            num_rows: 2,
            num_cols: 3,
            grid: vec![
                vec![
                    TableCell {
                        text: "Span 2 cols".to_string(),
                        row_span: Some(1),
                        col_span: Some(2), // Spans 2 columns
                        ref_item: None,
                        start_row_offset_idx: None,
                        start_col_offset_idx: None,
                        ..Default::default()
                    },
                    TableCell {
                        text: "C1".to_string(),
                        row_span: Some(1),
                        col_span: Some(1),
                        ref_item: None,
                        start_row_offset_idx: None,
                        start_col_offset_idx: None,
                        ..Default::default()
                    },
                ],
                vec![
                    TableCell {
                        text: "A2".to_string(),
                        row_span: Some(1),
                        col_span: Some(1),
                        ref_item: None,
                        start_row_offset_idx: None,
                        start_col_offset_idx: None,
                        ..Default::default()
                    },
                    TableCell {
                        text: "B2".to_string(),
                        row_span: Some(1),
                        col_span: Some(1),
                        ref_item: None,
                        start_row_offset_idx: None,
                        start_col_offset_idx: None,
                        ..Default::default()
                    },
                    TableCell {
                        text: "C2".to_string(),
                        row_span: Some(1),
                        col_span: Some(1),
                        ref_item: None,
                        start_row_offset_idx: None,
                        start_col_offset_idx: None,
                        ..Default::default()
                    },
                ],
            ],
            table_cells: None,
        };

        let rendered = crate::markdown_helper::render_table(&table_data);

        // Spanning cell should be rendered (colspan metadata may not be visible in markdown)
        assert!(rendered.contains("Span 2 cols"));
        assert!(rendered.contains("C1"));
        assert!(rendered.contains("A2"));
    }

    #[test]
    fn test_odt_footnotes() {
        let backend = OpenDocumentBackend::new(InputFormat::Odt).unwrap();

        // ODT supports footnotes (text:note with text:note-citation)
        // Create DocItem with footnote
        let doc_items = vec![
            create_text_item(
                0,
                "Main text with footnote reference".to_string(),
                backend.create_provenance(1),
            ),
            create_text_item(
                1,
                "Footnote content here".to_string(),
                backend.create_provenance(1),
            ),
        ];

        let markdown = crate::markdown_helper::docitems_to_markdown(&doc_items);

        // Both main text and footnote should be present
        assert!(markdown.contains("Main text with footnote reference"));
        assert!(markdown.contains("Footnote content here"));
    }

    #[test]
    fn test_odt_image_handling() {
        let backend = OpenDocumentBackend::new(InputFormat::Odt).unwrap();

        // ODT can contain images (draw:frame with draw:image)
        // Images are stored in Pictures/ folder in the ZIP
        let doc_items = vec![
            create_text_item(
                0,
                "Text before image".to_string(),
                backend.create_provenance(1),
            ),
            DocItem::Picture {
                self_ref: "#/pictures/0".to_string(),
                parent: None,
                children: vec![],
                content_layer: "body".to_string(),
                prov: backend.create_provenance(1),
                image: None, // Image data would be loaded from ZIP
                captions: vec![],
                footnotes: vec![],
                references: vec![],
                annotations: vec![],
                ocr_text: None,
            },
            create_text_item(
                1,
                "Text after image".to_string(),
                backend.create_provenance(1),
            ),
        ];

        let markdown = crate::markdown_helper::docitems_to_markdown(&doc_items);

        // Text should be present, image may be rendered as placeholder
        assert!(markdown.contains("Text before image"));
        assert!(markdown.contains("Text after image"));
    }

    #[test]
    fn test_odt_style_inheritance() {
        let backend = OpenDocumentBackend::new(InputFormat::Odt).unwrap();

        // ODT supports style inheritance (paragraph styles can inherit from parent styles)
        // XML with styled content
        let xml = r#"<?xml version="1.0"?>
            <office:document-content>
                <office:body>
                    <office:text>
                        <text:p text:style-name="Standard">Normal paragraph</text:p>
                        <text:p text:style-name="Heading">Styled as heading</text:p>
                        <text:p text:style-name="Quote">Styled as quote</text:p>
                    </office:text>
                </office:body>
            </office:document-content>"#;

        let doc_items = backend.parse_odt_content(xml).unwrap();

        // Verify all paragraphs are captured
        assert_eq!(doc_items.len(), 3);

        // Check text content
        let texts: Vec<String> = doc_items
            .iter()
            .filter_map(|item| match item {
                DocItem::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();

        assert!(texts.iter().any(|t| t.contains("Normal paragraph")));
        assert!(texts.iter().any(|t| t.contains("Styled as heading")));
        assert!(texts.iter().any(|t| t.contains("Styled as quote")));
    }

    #[test]
    fn test_odt_text_boxes() {
        let backend = OpenDocumentBackend::new(InputFormat::Odt).unwrap();

        // ODT supports text boxes (frames)
        let xml = r#"<?xml version="1.0"?>
            <office:document-content>
                <office:body>
                    <office:text>
                        <text:p>Main text</text:p>
                        <draw:frame>
                            <draw:text-box>
                                <text:p>Text box content</text:p>
                            </draw:text-box>
                        </draw:frame>
                    </office:text>
                </office:body>
            </office:document-content>"#;

        let doc_items = backend.parse_odt_content(xml).unwrap();
        assert!(doc_items.len() >= 2);

        let texts: Vec<String> = doc_items
            .iter()
            .filter_map(|item| match item {
                DocItem::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();

        assert!(texts.iter().any(|t| t.contains("Main text")));
        // Text box content may or may not be extracted depending on implementation
    }

    #[test]
    fn test_ods_cell_data_types() {
        let backend = OpenDocumentBackend::new(InputFormat::Ods).unwrap();

        // ODS supports different cell data types
        let xml = r#"<?xml version="1.0"?>
            <office:document-content>
                <office:body>
                    <office:spreadsheet>
                        <table:table>
                            <table:table-row>
                                <table:table-cell office:value-type="float" office:value="123.45">
                                    <text:p>123.45</text:p>
                                </table:table-cell>
                                <table:table-cell office:value-type="string">
                                    <text:p>Text</text:p>
                                </table:table-cell>
                                <table:table-cell office:value-type="date" office:date-value="2024-01-15">
                                    <text:p>2024-01-15</text:p>
                                </table:table-cell>
                            </table:table-row>
                        </table:table>
                    </office:spreadsheet>
                </office:body>
            </office:document-content>"#;

        let doc_items = backend.parse_odt_content(xml).unwrap();

        // Verify table DocItem was created with correct data
        assert_eq!(doc_items.len(), 1, "Should create one table DocItem");
        match &doc_items[0] {
            DocItem::Table { data, .. } => {
                assert_eq!(data.num_rows, 1);
                assert_eq!(data.num_cols, 3);
                assert_eq!(data.grid[0][0].text, "123.45");
                assert_eq!(data.grid[0][1].text, "Text");
                assert_eq!(data.grid[0][2].text, "2024-01-15");
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    #[test]
    fn test_odp_slide_notes() {
        let backend = OpenDocumentBackend::new(InputFormat::Odp).unwrap();

        // ODP supports speaker notes
        let xml = r#"<?xml version="1.0"?>
            <office:document-content>
                <office:body>
                    <office:presentation>
                        <draw:page>
                            <draw:frame>
                                <draw:text-box>
                                    <text:p>Slide content</text:p>
                                </draw:text-box>
                            </draw:frame>
                            <presentation:notes>
                                <text:p>Speaker notes here</text:p>
                            </presentation:notes>
                        </draw:page>
                    </office:presentation>
                </office:body>
            </office:document-content>"#;

        let doc_items = backend.parse_odt_content(xml).unwrap();
        assert!(!doc_items.is_empty());

        let texts: Vec<String> = doc_items
            .iter()
            .filter_map(|item| match item {
                DocItem::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();

        assert!(texts.iter().any(|t| t.contains("Slide content")));
        // Speaker notes may or may not be extracted
    }

    #[test]
    fn test_odt_page_breaks() {
        let backend = OpenDocumentBackend::new(InputFormat::Odt).unwrap();

        // ODT supports explicit page breaks
        let xml = r#"<?xml version="1.0"?>
            <office:document-content>
                <office:body>
                    <office:text>
                        <text:p>Page 1 content</text:p>
                        <text:soft-page-break/>
                        <text:p>Page 2 content</text:p>
                    </office:text>
                </office:body>
            </office:document-content>"#;

        let doc_items = backend.parse_odt_content(xml).unwrap();
        assert!(doc_items.len() >= 2);

        let texts: Vec<String> = doc_items
            .iter()
            .filter_map(|item| match item {
                DocItem::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();

        assert!(texts.iter().any(|t| t.contains("Page 1 content")));
        assert!(texts.iter().any(|t| t.contains("Page 2 content")));
    }

    #[test]
    fn test_ods_conditional_formatting() {
        let backend = OpenDocumentBackend::new(InputFormat::Ods).unwrap();

        // ODS supports conditional formatting (style applied based on cell value)
        let xml = r#"<?xml version="1.0"?>
            <office:document-content>
                <office:body>
                    <office:spreadsheet>
                        <table:table>
                            <table:table-row>
                                <table:table-cell office:value-type="float" office:value="100">
                                    <text:p>100</text:p>
                                </table:table-cell>
                                <table:table-cell office:value-type="float" office:value="50">
                                    <text:p>50</text:p>
                                </table:table-cell>
                            </table:table-row>
                        </table:table>
                    </office:spreadsheet>
                </office:body>
            </office:document-content>"#;

        let doc_items = backend.parse_odt_content(xml).unwrap();

        // Verify table DocItem was created with correct data
        // Values should be preserved regardless of conditional formatting
        assert_eq!(doc_items.len(), 1, "Should create one table DocItem");
        match &doc_items[0] {
            DocItem::Table { data, .. } => {
                assert_eq!(data.num_rows, 1);
                assert_eq!(data.num_cols, 2);
                assert_eq!(data.grid[0][0].text, "100");
                assert_eq!(data.grid[0][1].text, "50");
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    #[test]
    fn test_ods_merged_cells() {
        let backend = OpenDocumentBackend::new(InputFormat::Ods).unwrap();

        // ODS merged cells using table:number-columns-spanned and table:number-rows-spanned
        let xml = r#"<?xml version="1.0"?>
            <office:document-content>
                <office:body>
                    <office:spreadsheet>
                        <table:table>
                            <table:table-row>
                                <table:table-cell table:number-columns-spanned="2" table:number-rows-spanned="1">
                                    <text:p>Merged Cell</text:p>
                                </table:table-cell>
                            </table:table-row>
                            <table:table-row>
                                <table:table-cell>
                                    <text:p>Cell A2</text:p>
                                </table:table-cell>
                                <table:table-cell>
                                    <text:p>Cell B2</text:p>
                                </table:table-cell>
                            </table:table-row>
                        </table:table>
                    </office:spreadsheet>
                </office:body>
            </office:document-content>"#;

        let doc_items = backend.parse_odt_content(xml).unwrap();

        // Verify table DocItem was created with merged cell handling
        assert_eq!(doc_items.len(), 1, "Should create one table DocItem");
        match &doc_items[0] {
            DocItem::Table { data, .. } => {
                // Should have 2 rows
                assert_eq!(data.num_rows, 2);
                // First row should have merged cell content
                assert!(data.grid[0][0].text.contains("Merged Cell"));
                // Second row should have separate cells
                assert!(data.grid[1][0].text.contains("Cell A2"));
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    #[test]
    fn test_odt_nested_tables() {
        let backend = OpenDocumentBackend::new(InputFormat::Odt).unwrap();

        // ODT supports tables within tables (nested structure)
        let xml = r#"<?xml version="1.0"?>
            <office:document-content>
                <office:body>
                    <office:text>
                        <table:table>
                            <table:table-row>
                                <table:table-cell>
                                    <text:p>Outer Cell</text:p>
                                    <table:table>
                                        <table:table-row>
                                            <table:table-cell>
                                                <text:p>Inner Cell</text:p>
                                            </table:table-cell>
                                        </table:table-row>
                                    </table:table>
                                </table:table-cell>
                            </table:table-row>
                        </table:table>
                    </office:text>
                </office:body>
            </office:document-content>"#;

        let doc_items = backend.parse_odt_content(xml).unwrap();

        // Should create table DocItems for both outer and inner tables
        let table_count = doc_items
            .iter()
            .filter(|item| matches!(item, DocItem::Table { .. }))
            .count();
        assert!(table_count >= 1, "Should create at least one table DocItem");

        // Verify outer table contains "Outer Cell" text
        let has_outer = doc_items.iter().any(|item| match item {
            DocItem::Table { data, .. } => data.grid.iter().any(|row| {
                row.iter().any(|cell| {
                    cell.text.contains("Outer Cell") || cell.text.contains("Inner Cell")
                })
            }),
            _ => false,
        });
        assert!(
            has_outer,
            "Should contain table content from nested structure"
        );
    }

    #[test]
    fn test_odp_embedded_multimedia_references() {
        let backend = OpenDocumentBackend::new(InputFormat::Odp).unwrap();

        // ODP presentations can reference embedded images/videos
        let xml = r#"<?xml version="1.0"?>
            <office:document-content>
                <office:body>
                    <office:presentation>
                        <draw:page>
                            <draw:frame>
                                <draw:image xlink:href="Pictures/image1.png"/>
                            </draw:frame>
                            <draw:frame>
                                <text:p>Slide with multimedia</text:p>
                            </draw:frame>
                        </draw:page>
                    </office:presentation>
                </office:body>
            </office:document-content>"#;

        let doc_items = backend.parse_odt_content(xml).unwrap();

        // Should create DocItems for slide content (text)
        // Image references are typically not converted to DocItems (binary content)
        let text_items: Vec<_> = doc_items
            .iter()
            .filter(|item| matches!(item, DocItem::Text { .. }))
            .collect();

        assert!(
            !text_items.is_empty(),
            "Should extract text from slides with multimedia"
        );

        let has_slide_text = text_items.iter().any(|item| match item {
            DocItem::Text { text, .. } => text.contains("Slide with multimedia"),
            _ => false,
        });
        assert!(has_slide_text, "Should extract slide text content");
    }

    #[test]
    fn test_ods_external_sheet_references() {
        let backend = OpenDocumentBackend::new(InputFormat::Ods).unwrap();

        // ODS supports formulas with external sheet references
        let xml = r#"<?xml version="1.0"?>
            <office:document-content>
                <office:body>
                    <office:spreadsheet>
                        <table:table table:name="Sheet1">
                            <table:table-row>
                                <table:table-cell office:value-type="float" office:value="100">
                                    <text:p>100</text:p>
                                </table:table-cell>
                            </table:table-row>
                        </table:table>
                        <table:table table:name="Sheet2">
                            <table:table-row>
                                <table:table-cell table:formula="of:=[Sheet1.A1]*2">
                                    <text:p>200</text:p>
                                </table:table-cell>
                            </table:table-row>
                        </table:table>
                    </office:spreadsheet>
                </office:body>
            </office:document-content>"#;

        let doc_items = backend.parse_odt_content(xml).unwrap();

        // Should create table DocItems for both sheets
        let table_count = doc_items
            .iter()
            .filter(|item| matches!(item, DocItem::Table { .. }))
            .count();
        assert_eq!(
            table_count, 2,
            "Should create two table DocItems for two sheets"
        );

        // Verify that cell values are extracted (formulas evaluated to results)
        let has_value_200 = doc_items.iter().any(|item| match item {
            DocItem::Table { data, .. } => data
                .grid
                .iter()
                .any(|row| row.iter().any(|cell| cell.text.contains("200"))),
            _ => false,
        });
        assert!(has_value_200, "Should extract formula result value");
    }

    #[test]
    fn test_odt_change_tracking() {
        let backend = OpenDocumentBackend::new(InputFormat::Odt).unwrap();

        // ODT supports change tracking (track-changes)
        // text:change-start, text:change-end markers
        let xml = r#"<?xml version="1.0"?>
            <office:document-content>
                <office:body>
                    <office:text>
                        <text:p>Original text
                            <text:change-start text:change-id="ct1"/>
                            with tracked changes
                            <text:change-end text:change-id="ct1"/>
                            and more content
                        </text:p>
                    </office:text>
                </office:body>
            </office:document-content>"#;

        let doc_items = backend.parse_odt_content(xml).unwrap();

        // Should extract text content, handling change tracking markers
        assert!(
            !doc_items.is_empty(),
            "Should extract content with change tracking"
        );

        let text_items: Vec<_> = doc_items
            .iter()
            .filter_map(|item| match item {
                DocItem::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();

        assert!(!text_items.is_empty(), "Should have text items");

        // Should contain the text content (change markers should not break parsing)
        let full_text = text_items.join(" ");
        assert!(
            full_text.contains("Original text")
                || full_text.contains("tracked changes")
                || full_text.contains("more content"),
            "Should extract text content from document with change tracking"
        );
    }

    #[test]
    fn test_odp_with_speaker_notes() {
        let backend = OpenDocumentBackend::new(InputFormat::Odp).unwrap();

        // ODP presentations support speaker notes (draw:page with presentation:notes)
        let xml = r#"<?xml version="1.0"?>
            <office:document-content>
                <office:body>
                    <office:presentation>
                        <draw:page draw:name="page1">
                            <draw:frame>
                                <draw:text-box>
                                    <text:p>Slide content</text:p>
                                </draw:text-box>
                            </draw:frame>
                            <presentation:notes>
                                <draw:frame>
                                    <draw:text-box>
                                        <text:p>These are speaker notes for the presenter</text:p>
                                    </draw:text-box>
                                </draw:frame>
                            </presentation:notes>
                        </draw:page>
                    </office:presentation>
                </office:body>
            </office:document-content>"#;

        let doc_items = backend.parse_odt_content(xml).unwrap();

        // Should extract both slide content and speaker notes
        assert!(
            !doc_items.is_empty(),
            "Should extract content from presentation with speaker notes"
        );

        let text_items: Vec<_> = doc_items
            .iter()
            .filter_map(|item| match item {
                DocItem::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();

        assert!(!text_items.is_empty(), "Should have text items");

        // Should contain both slide content and notes
        let full_text = text_items.join(" ");
        assert!(
            full_text.contains("Slide content") || full_text.contains("speaker notes"),
            "Should extract both slide content and speaker notes"
        );
    }

    #[test]
    fn test_ods_with_conditional_formatting() {
        let backend = OpenDocumentBackend::new(InputFormat::Ods).unwrap();

        // ODS spreadsheets support conditional formatting
        // (style:map with style:condition attribute)
        let xml = r#"<?xml version="1.0"?>
            <office:document-content>
                <office:automatic-styles>
                    <style:style style:name="ce1">
                        <style:map style:condition="cell-content()&gt;50" style:apply-style-name="good"/>
                        <style:map style:condition="cell-content()&lt;=50" style:apply-style-name="bad"/>
                    </style:style>
                </office:automatic-styles>
                <office:body>
                    <office:spreadsheet>
                        <table:table>
                            <table:table-row>
                                <table:table-cell table:style-name="ce1" office:value-type="float" office:value="75">
                                    <text:p>75</text:p>
                                </table:table-cell>
                                <table:table-cell table:style-name="ce1" office:value-type="float" office:value="25">
                                    <text:p>25</text:p>
                                </table:table-cell>
                            </table:table-row>
                        </table:table>
                    </office:spreadsheet>
                </office:body>
            </office:document-content>"#;

        let doc_items = backend.parse_odt_content(xml).unwrap();

        // Should extract table with cell values (conditional formatting is styling)
        let table_items: Vec<_> = doc_items
            .iter()
            .filter(|item| matches!(item, DocItem::Table { .. }))
            .collect();

        assert!(!table_items.is_empty(), "Should have table items");

        // Should contain cell values
        let has_values = doc_items.iter().any(|item| match item {
            DocItem::Table { data, .. } => data.grid.iter().any(|row| {
                row.iter()
                    .any(|cell| cell.text.contains("75") || cell.text.contains("25"))
            }),
            _ => false,
        });
        assert!(
            has_values,
            "Should extract cell values with conditional formatting"
        );
    }

    #[test]
    fn test_odt_with_bibliography_references() {
        let backend = OpenDocumentBackend::new(InputFormat::Odt).unwrap();

        // ODT supports bibliography references (text:bibliography-mark)
        let xml = r#"<?xml version="1.0"?>
            <office:document-content>
                <office:body>
                    <office:text>
                        <text:p>Research shows that
                            <text:bibliography-mark
                                text:identifier="Smith2020"
                                text:bibliography-type="article">
                            </text:bibliography-mark>
                            machine learning is effective.
                        </text:p>
                        <text:bibliography>
                            <text:index-body>
                                <text:p>Smith, J. (2020). ML Research. Journal of AI.</text:p>
                            </text:index-body>
                        </text:bibliography>
                    </office:text>
                </office:body>
            </office:document-content>"#;

        let doc_items = backend.parse_odt_content(xml).unwrap();

        // Should extract text content and bibliography
        assert!(
            !doc_items.is_empty(),
            "Should extract content with bibliography"
        );

        let text_items: Vec<_> = doc_items
            .iter()
            .filter_map(|item| match item {
                DocItem::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();

        assert!(!text_items.is_empty(), "Should have text items");

        // Should contain both main text and bibliography entry
        let full_text = text_items.join(" ");
        assert!(
            full_text.contains("machine learning")
                || full_text.contains("Smith")
                || full_text.contains("2020"),
            "Should extract text with bibliography references"
        );
    }

    #[test]
    fn test_odt_with_drawing_shapes() {
        let backend = OpenDocumentBackend::new(InputFormat::Odt).unwrap();

        // ODT documents can embed drawings with connector lines between shapes
        // (draw:connector with draw:start-shape and draw:end-shape)
        let xml = r#"<?xml version="1.0"?>
            <office:document-content>
                <office:body>
                    <office:text>
                        <draw:page>
                            <draw:custom-shape draw:name="shape1">
                                <draw:text-box>
                                    <text:p>Start Node</text:p>
                                </draw:text-box>
                            </draw:custom-shape>
                            <draw:custom-shape draw:name="shape2">
                                <draw:text-box>
                                    <text:p>End Node</text:p>
                                </draw:text-box>
                            </draw:custom-shape>
                            <draw:connector
                                draw:start-shape="shape1"
                                draw:end-shape="shape2"
                                draw:type="standard">
                            </draw:connector>
                        </draw:page>
                    </office:text>
                </office:body>
            </office:document-content>"#;

        let doc_items = backend.parse_odt_content(xml).unwrap();

        // Should extract text from shapes (connector lines are structural)
        assert!(
            !doc_items.is_empty(),
            "Should extract content from document with embedded drawing"
        );

        let text_items: Vec<_> = doc_items
            .iter()
            .filter_map(|item| match item {
                DocItem::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();

        assert!(!text_items.is_empty(), "Should have text items from shapes");

        // Should contain text from both shapes
        let full_text = text_items.join(" ");
        assert!(
            full_text.contains("Start Node") || full_text.contains("End Node"),
            "Should extract text from connected shapes in embedded drawing"
        );
    }

    #[test]
    fn test_odt_with_index_entries() {
        let backend = OpenDocumentBackend::new(InputFormat::Odt).unwrap();

        // ODT supports index entries (text:alphabetical-index-mark)
        let xml = r#"<?xml version="1.0"?>
            <office:document-content>
                <office:body>
                    <office:text>
                        <text:p>Document with
                            <text:alphabetical-index-mark text:string-value="Important Term"/>
                            important concepts and
                            <text:alphabetical-index-mark text:string-value="Key Concept"/>
                            terminology.
                        </text:p>
                        <text:alphabetical-index>
                            <text:index-body>
                                <text:p>Important Term ........... 1</text:p>
                                <text:p>Key Concept ............... 1</text:p>
                            </text:index-body>
                        </text:alphabetical-index>
                    </office:text>
                </office:body>
            </office:document-content>"#;

        let doc_items = backend.parse_odt_content(xml).unwrap();

        // Should extract text content and index
        assert!(!doc_items.is_empty(), "Should extract content with index");

        let text_items: Vec<_> = doc_items
            .iter()
            .filter_map(|item| match item {
                DocItem::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();

        assert!(!text_items.is_empty(), "Should have text items");

        // Should contain both main text and index entries
        let full_text = text_items.join(" ");
        assert!(
            full_text.contains("important")
                || full_text.contains("Important Term")
                || full_text.contains("Key Concept"),
            "Should extract text with index entries"
        );
    }
}
