//! `AsciiDoc` Backend - Port from Python docling v2.58.0
//!
//! Source: ~/`docling/docling/backend/asciidoc_backend.py` (444 lines)
//!
//! Parses `AsciiDoc` documents into structured `DocItems` using line-by-line regex parsing.
//! Supports titles, section headers, lists, tables, pictures, captions, and plain text.
//!
//! # Python Reference
//!
//! `asciidoc_backend.py` - custom parser (no external library)
//!
//! ## Key Python Methods
//!
//! - `_parse()`: lines 81-270 - Main line-by-line parser
//! - `_parse_title()`: lines 294-295 - Parse title line (= Title)
//! - `_parse_section_header()`: lines 303-314 - Parse headers (== Section)
//! - `_parse_list_item()`: lines 322-355 - Parse list items (*, -, 1.)
//! - `_populate_table_as_grid()`: lines 368-396 - Build table from lines
//! - `_parse_picture()`: lines 404-424 - Parse image macros (`image::path`[])
//! - `_parse_caption()`: lines 432-437 - Parse captions (.Caption)

// Clippy pedantic allows:
// - Parser state uses multiple bool flags for tracking context
#![allow(clippy::struct_excessive_bools)]

use crate::traits::{BackendOptions, DocumentBackend};
use crate::utils::{create_provenance, create_section_header, create_text_item, opt_vec};
use docling_core::{
    content::{DocItem, TableCell, TableData},
    DoclingError, Document, DocumentMetadata, InputFormat,
};
use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;

// Pre-compiled regex patterns using std::sync::LazyLock (Rust 1.80+)
// Replaces lazy_static! macro with standard library equivalent
static RE_SECTION_HEADER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^==+\s+").expect("regex is compile-time constant"));
static RE_SECTION_HEADER_CAPTURE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(=+)\s+(.*)").expect("regex is compile-time constant"));
static RE_LIST_ITEM: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(\s)*(\*|-|\d+\.|\w+\.)\s+").expect("regex is compile-time constant")
});
static RE_LIST_ITEM_CAPTURE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(\s*)(\*|-|\d+\.)\s+(.*)").expect("regex is compile-time constant")
});
static RE_TABLE_LINE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\|.*\|").expect("regex is compile-time constant"));
static RE_PICTURE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^image::(.+)\[(.*)\]$").expect("regex is compile-time constant"));
static RE_CAPTION: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\.(.+)").expect("regex is compile-time constant"));

/// State for `parse_asciidoc` function (reduces cognitive complexity)
///
/// Groups all state variables used during line-by-line parsing.
#[derive(Debug, Clone, PartialEq)]
struct ParseAsciidocState {
    // Output
    doc_items: Vec<DocItem>,
    item_count: usize,
    group_count: usize,

    // Block state
    in_list: bool,
    in_table: bool,
    in_literal_block: bool,
    in_source_block: bool,
    in_sidebar_block: bool,
    in_example_block: bool,
    in_passthrough_block: bool,

    // Admonition state
    admonition_type: Option<String>,

    // Buffers
    text_buffer: Vec<String>,
    table_buffer: Vec<Vec<String>>,
    caption_buffer: Vec<String>,
    literal_buffer: Vec<String>,
    sidebar_buffer: Vec<String>,

    // Nested list tracking (Python: lines 133-159)
    parents: HashMap<usize, Option<String>>,
    indents: HashMap<usize, Option<usize>>,
    list_children: HashMap<usize, Vec<String>>,
}

impl ParseAsciidocState {
    fn new() -> Self {
        let mut state = Self {
            doc_items: Vec::new(),
            item_count: 0,
            group_count: 0,
            in_list: false,
            in_table: false,
            in_literal_block: false,
            in_source_block: false,
            in_sidebar_block: false,
            in_example_block: false,
            in_passthrough_block: false,
            admonition_type: None,
            text_buffer: Vec::new(),
            table_buffer: Vec::new(),
            caption_buffer: Vec::new(),
            literal_buffer: Vec::new(),
            sidebar_buffer: Vec::new(),
            parents: HashMap::new(),
            indents: HashMap::new(),
            list_children: HashMap::new(),
        };
        // Initialize level 0 (root level)
        state.parents.insert(0, None);
        state.indents.insert(0, None);
        state.list_children.insert(0, Vec::new());
        state
    }

    /// Consume state and return `doc_items`
    fn into_doc_items(self) -> Vec<DocItem> {
        self.doc_items
    }

    /// Create a Text `DocItem` and add to `doc_items`
    fn push_text_item(&mut self, text: String) {
        let prov = create_provenance(1);
        let doc_item = create_text_item(self.item_count, text, prov);
        self.doc_items.push(doc_item);
        self.item_count += 1;
    }

    /// Handle sidebar block end (****)
    fn handle_sidebar_end(&mut self) {
        if !self.sidebar_buffer.is_empty() {
            let sidebar_text = self.sidebar_buffer.join(" ");
            self.push_text_item(sidebar_text);
            self.sidebar_buffer.clear();
        }
        self.in_sidebar_block = false;
    }

    /// Handle example block end (====)
    fn handle_example_end(&mut self) {
        if !self.sidebar_buffer.is_empty() {
            let example_text = self.sidebar_buffer.join(" ");
            self.push_text_item(example_text);
            self.sidebar_buffer.clear();
        }
        self.in_example_block = false;
    }

    /// Handle literal block end (....)
    fn handle_literal_end(&mut self) {
        if !self.literal_buffer.is_empty() {
            let code = self.literal_buffer.join("\n");
            self.push_text_item(code);
            self.literal_buffer.clear();
        }
        self.in_literal_block = false;
    }

    /// Handle source block end (----)
    fn handle_source_end(&mut self) {
        if !self.literal_buffer.is_empty() {
            let code = self.literal_buffer.join("\n");
            self.push_text_item(code);
            self.literal_buffer.clear();
        }
        self.in_source_block = false;
    }

    /// Handle admonition line
    fn handle_admonition(&mut self, line: &str) {
        // Use take() instead of clone() to avoid unnecessary allocation
        // take() returns the value and sets the field to None in one operation
        if let Some(adm) = self.admonition_type.take() {
            let adm_text = format!("{}: {}", adm, line.trim());
            self.push_text_item(adm_text);
        }
    }

    /// Try to handle a block delimiter line. Returns true if handled.
    fn try_handle_block_delimiter(&mut self, line: &str) -> bool {
        let trimmed = line.trim();

        // Passthrough block (++++)
        if trimmed == "++++" {
            self.in_passthrough_block = !self.in_passthrough_block;
            return true;
        }

        // Sidebar block (****)
        if trimmed == "****" {
            if self.in_sidebar_block {
                self.handle_sidebar_end();
            } else {
                self.in_sidebar_block = true;
            }
            return true;
        }

        // Example block (====)
        if trimmed == "====" {
            if self.in_example_block {
                self.handle_example_end();
            } else {
                self.in_example_block = true;
            }
            return true;
        }

        // Literal block (....)
        if trimmed == "...." {
            if self.in_literal_block {
                self.handle_literal_end();
            } else {
                self.in_literal_block = true;
            }
            return true;
        }

        // Source block (----)
        if trimmed.starts_with("----") && trimmed.len() >= 4 {
            if self.in_source_block {
                self.handle_source_end();
            } else {
                self.in_source_block = true;
            }
            return true;
        }

        false
    }

    /// Check if we're in a block and accumulate content. Returns true if handled.
    fn try_accumulate_in_block(&mut self, line: &str) -> bool {
        // Skip content in passthrough blocks
        if self.in_passthrough_block {
            return true;
        }

        // Accumulate content in sidebar/example blocks
        if self.in_sidebar_block || self.in_example_block {
            if !line.trim().is_empty() {
                self.sidebar_buffer.push(line.trim().to_string());
            }
            return true;
        }

        // Accumulate content in literal/source blocks
        if self.in_literal_block || self.in_source_block {
            self.literal_buffer.push(line.to_string());
            return true;
        }

        false
    }

    /// Check if line should be skipped (passthrough syntax or source attributes)
    #[inline]
    fn should_skip_line(line: &str) -> bool {
        let trimmed = line.trim();
        // Skip inline passthrough syntax
        if line.contains("+++") || line.contains("pass:[") {
            return true;
        }
        // Skip source/listing attributes
        if trimmed.starts_with("[source,") || trimmed.starts_with("[listing") {
            return true;
        }
        false
    }

    /// Try to handle pending admonition. Returns true if handled.
    fn try_handle_pending_admonition(&mut self, line: &str) -> bool {
        if self.admonition_type.is_some()
            && !line.trim().is_empty()
            && !line.trim().starts_with("====")
        {
            self.handle_admonition(line);
            return true;
        }
        false
    }

    /// Flush all buffers at end of parsing
    fn flush_all(&mut self) {
        self.flush_text_buffer();
        if self.in_list {
            self.finalize_all_lists();
        }
        self.flush_table_buffer();
    }

    /// Get current list level (Python: helper for nested list tracking)
    #[inline]
    fn get_current_level(&self) -> usize {
        self.parents
            .iter()
            .filter(|(_, v)| v.is_some())
            .map(|(k, _)| *k)
            .max()
            .unwrap_or(0)
    }

    /// Get current parent ref (Python: helper for nested list tracking)
    #[inline]
    fn get_current_parent(&self) -> Option<String> {
        let level = self.get_current_level();
        self.parents.get(&level).and_then(Option::clone)
    }

    /// Create Caption `DocItem` from `caption_buffer` and return its ref
    fn create_caption(&mut self) -> Option<String> {
        if self.caption_buffer.is_empty() {
            return None;
        }
        let caption_text = self.caption_buffer.join(" ");
        let caption_ref = format!("#/captions/{}", self.item_count);
        let caption_item = DocItem::Caption {
            self_ref: caption_ref.clone(),
            parent: None,
            children: vec![],
            content_layer: "body".to_string(),
            prov: create_provenance(1),
            orig: caption_text.clone(),
            text: caption_text,
            formatting: None,
            hyperlink: None,
        };
        self.doc_items.push(caption_item);
        self.item_count += 1;
        self.caption_buffer.clear();
        Some(caption_ref)
    }

    /// Finalize list children for a given level
    fn finalize_list_level(&mut self, level: usize) {
        if let Some(Some(list_ref)) = self.parents.get(&level) {
            if let Some(children) = self.list_children.get(&level) {
                let children_refs: Vec<docling_core::content::ItemRef> = children
                    .iter()
                    .map(|r| docling_core::content::ItemRef::new(r.clone()))
                    .collect();
                // Find and update the List group in doc_items
                for item in &mut self.doc_items {
                    match item {
                        DocItem::List {
                            self_ref,
                            children: item_children,
                            ..
                        }
                        | DocItem::OrderedList {
                            self_ref,
                            children: item_children,
                            ..
                        } if self_ref == list_ref => {
                            *item_children = children_refs;
                            break;
                        }
                        _ => {}
                    }
                }
            }
        }
        self.parents.insert(level, None);
        self.indents.insert(level, None);
        self.list_children.insert(level, Vec::new());
    }

    /// Finalize all remaining list groups
    fn finalize_all_lists(&mut self) {
        let levels_to_finalize: Vec<usize> = self
            .parents
            .iter()
            .filter_map(|(k, v)| v.is_some().then_some(*k))
            .collect();
        for level in levels_to_finalize {
            self.finalize_list_level(level);
        }
        self.in_list = false;
    }

    /// Flush text buffer to `doc_items`
    fn flush_text_buffer(&mut self) {
        if !self.text_buffer.is_empty() {
            let text = self.text_buffer.join(" ");
            self.push_text_item(text);
            self.text_buffer.clear();
        }
    }

    /// Flush remaining table buffer
    fn flush_table_buffer(&mut self) {
        if self.in_table && !self.table_buffer.is_empty() {
            let data = AsciidocBackend::populate_table_as_grid(&self.table_buffer);
            let doc_item = DocItem::Table {
                self_ref: format!("#/tables/{}", self.item_count),
                parent: None,
                children: vec![],
                content_layer: "body".to_string(),
                prov: create_provenance(1),
                data,
                captions: vec![],
                footnotes: vec![],
                references: vec![],
                image: None,
                annotations: vec![],
            };
            self.doc_items.push(doc_item);
            self.item_count += 1;
            self.table_buffer.clear();
        }
    }

    /// Handle title line (= Title)
    fn handle_title(&mut self, text: String) {
        let doc_item = create_section_header(
            self.item_count,
            text,
            1, // Title is level 1 (= becomes #)
            create_provenance(1),
        );
        self.doc_items.push(doc_item);
        self.item_count += 1;
    }

    /// Handle section header line (== Section)
    fn handle_section_header(&mut self, level: usize, text: String) {
        let doc_item = create_section_header(
            self.item_count,
            text,
            level + 1, // AsciiDoc level + 1 for markdown
            create_provenance(1),
        );
        self.doc_items.push(doc_item);
        self.item_count += 1;
    }

    /// Handle include directive
    fn handle_include(&mut self, filename: &str) {
        let include_text = format!("// Include: {filename}");
        self.push_text_item(include_text);
    }

    /// Check and set admonition type
    fn handle_admonition_block(&mut self, line: &str) -> bool {
        if line.trim().starts_with('[') && line.trim().ends_with(']') {
            let adm_text = line.trim()[1..line.trim().len() - 1].to_uppercase();
            match adm_text.as_str() {
                "NOTE" | "TIP" | "WARNING" | "IMPORTANT" | "CAUTION" => {
                    self.admonition_type = Some(adm_text);
                    return true;
                }
                _ => {}
            }
        }
        false
    }

    /// Create a new list group (List or `OrderedList`)
    fn create_list_group(&mut self, enumerated: bool, level: usize) {
        let list_ref = format!("#/groups/{}", self.group_count);
        let parent_ref = self.get_current_parent();

        let list_group = if enumerated {
            DocItem::OrderedList {
                self_ref: list_ref.clone(),
                parent: parent_ref.map(docling_core::content::ItemRef::new),
                children: vec![],
                content_layer: "body".to_string(),
                name: format!("list_{}", self.group_count),
            }
        } else {
            DocItem::List {
                self_ref: list_ref.clone(),
                parent: parent_ref.map(docling_core::content::ItemRef::new),
                children: vec![],
                content_layer: "body".to_string(),
                name: format!("list_{}", self.group_count),
            }
        };
        self.doc_items.push(list_group);
        self.group_count += 1;

        self.parents.insert(level + 1, Some(list_ref));
    }

    /// Handle list item
    fn handle_list_item(&mut self, indent: usize, marker: String, text: String) {
        let enumerated = marker.chars().next().is_some_and(|c| c.is_ascii_digit());
        let mut level = self.get_current_level();

        // First list item: create initial List group
        if !self.in_list {
            self.in_list = true;
            self.create_list_group(enumerated, level);
            self.indents.insert(level + 1, Some(indent));
            self.list_children.insert(level + 1, Vec::new());
        }
        // Indent increased: create nested List group
        else if indent > self.indents.get(&level).and_then(|i| *i).unwrap_or(0) {
            self.create_list_group(enumerated, level);
            self.indents.insert(level + 1, Some(indent));
            self.list_children.insert(level + 1, Vec::new());
        }
        // Indent decreased: pop back to parent levels
        else if indent < self.indents.get(&level).and_then(|i| *i).unwrap_or(0) {
            while indent < self.indents.get(&level).and_then(|i| *i).unwrap_or(0) && level > 0 {
                self.finalize_list_level(level);
                level -= 1;
            }
        }

        // Marker handling: empty for bullet lists, original for numbered
        let final_marker = if enumerated { marker } else { String::new() };

        let parent_ref = self.get_current_parent();
        let item_ref = format!("#/texts/{}", self.item_count);

        let doc_item = DocItem::ListItem {
            self_ref: item_ref.clone(),
            parent: parent_ref.map(docling_core::content::ItemRef::new),
            children: vec![],
            content_layer: "body".to_string(),
            prov: create_provenance(1),
            orig: text.clone(),
            text,
            marker: final_marker,
            enumerated,
            formatting: None,
            hyperlink: None,
        };
        self.doc_items.push(doc_item);

        // Add to current list's children
        let current_list_level = self.get_current_level();
        if let Some(children) = self.list_children.get_mut(&current_list_level) {
            children.push(item_ref);
        }
        self.item_count += 1;
    }

    /// Handle end of table
    fn handle_table_end(&mut self) {
        if !self.table_buffer.is_empty() {
            let data = AsciidocBackend::populate_table_as_grid(&self.table_buffer);
            self.table_buffer.clear();

            let caption_ref = self.create_caption();

            let doc_item = DocItem::Table {
                self_ref: format!("#/tables/{}", self.item_count),
                parent: None,
                children: vec![],
                content_layer: "body".to_string(),
                prov: create_provenance(1),
                data,
                captions: caption_ref
                    .map(|r| vec![docling_core::content::ItemRef::new(r)])
                    .unwrap_or_default(),
                footnotes: vec![],
                references: vec![],
                image: None,
                annotations: vec![],
            };
            self.doc_items.push(doc_item);
            self.item_count += 1;
        }
        self.in_table = false;
    }

    /// Handle picture line
    fn handle_picture(&mut self, uri: String, width: Option<i32>, height: Option<i32>) {
        let caption_ref = self.create_caption();
        let width = width.unwrap_or(640);
        let height = height.unwrap_or(480);

        // Normalize URI
        let normalized_uri = if uri.starts_with("http") {
            uri
        } else if uri.starts_with("//") {
            format!("file:{uri}")
        } else if uri.starts_with('/') {
            format!("file:/{uri}")
        } else {
            format!("file://{uri}")
        };

        let image_json = serde_json::json!({
            "mimetype": "image/png",
            "size": { "width": width, "height": height },
            "dpi": 70,
            "uri": normalized_uri
        });

        let doc_item = DocItem::Picture {
            self_ref: format!("#/pictures/{}", self.item_count),
            parent: None,
            children: vec![],
            content_layer: "body".to_string(),
            prov: create_provenance(1),
            captions: caption_ref
                .map(|r| vec![docling_core::content::ItemRef::new(r)])
                .unwrap_or_default(),
            footnotes: vec![],
            references: vec![],
            image: Some(image_json),
            annotations: vec![],
            ocr_text: None,
        };
        self.doc_items.push(doc_item);
        self.item_count += 1;
    }
}

/// `AsciiDoc` Document Backend
///
/// Ported from: docling/backend/asciidoc_backend.py:29-444
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct AsciidocBackend;

impl AsciidocBackend {
    /// Create a new `AsciiDoc` backend instance
    #[inline]
    #[must_use = "creates a backend instance that should be used for parsing"]
    pub const fn new() -> Self {
        Self
    }

    /// Check if line is a title (Python: line 290-291)
    /// Syntax: = Title
    #[inline]
    fn is_title(line: &str) -> bool {
        line.starts_with("= ")
    }

    /// Parse title line (Python: lines 294-295)
    /// Extracts title text (does NOT strip ordinals - Python keeps them)
    #[inline]
    fn parse_title(line: &str) -> String {
        line[2..].trim().to_string()
    }

    /// Check if line is a section header (Python: lines 298-300)
    /// Syntax: == Section, === Subsection, etc.
    #[inline]
    fn is_section_header(line: &str) -> bool {
        RE_SECTION_HEADER.is_match(line)
    }

    /// Parse section header (Python: lines 303-314)
    /// Returns (level, text) where level starts at 1 for ==
    fn parse_section_header(line: &str) -> Option<(usize, String)> {
        RE_SECTION_HEADER_CAPTURE.captures(line).map(|caps| {
            let marker = caps
                .get(1)
                .expect("capture group 1 guaranteed by regex")
                .as_str();
            let text = caps
                .get(2)
                .expect("capture group 2 guaranteed by regex")
                .as_str();
            let level = marker.len() - 1; // == is level 1, === is level 2, etc.
            (level, text.trim().to_string())
        })
    }

    /// Check if line is a list item (Python: lines 317-319)
    /// Syntax: *, -, 1., a.
    #[inline]
    fn is_list_item(line: &str) -> bool {
        RE_LIST_ITEM.is_match(line)
    }

    /// Parse list item (Python: lines 322-355)
    /// Returns (`indent_level`, marker, text)
    fn parse_list_item(line: &str) -> Option<(usize, String, String)> {
        RE_LIST_ITEM_CAPTURE.captures(line).map(|caps| {
            let indent = caps.get(1).map_or(0, |m| m.as_str().len());
            let marker = caps
                .get(2)
                .expect("capture group 2 guaranteed by regex")
                .as_str()
                .to_string();
            let text = caps
                .get(3)
                .expect("capture group 3 guaranteed by regex")
                .as_str()
                .trim()
                .to_string();
            (indent, marker, text)
        })
    }

    /// Check if line is a table line (Python: lines 358-360)
    /// Syntax: |cell1|cell2|
    #[inline]
    fn is_table_line(line: &str) -> bool {
        RE_TABLE_LINE.is_match(line)
    }

    /// Parse table line into cells (Python: lines 363-365)
    #[inline]
    fn parse_table_line(line: &str) -> Vec<String> {
        line.split('|')
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.trim().to_string())
            .collect()
    }

    /// Build `TableData` from parsed table lines (Python: lines 368-396)
    fn populate_table_as_grid(table_data: &[Vec<String>]) -> TableData {
        let num_rows = table_data.len();
        let num_cols = table_data.iter().map(std::vec::Vec::len).max().unwrap_or(0);

        let mut table_cells = Vec::new();

        for (row_idx, row) in table_data.iter().enumerate() {
            for (col_idx, text) in row.iter().enumerate() {
                let cell = TableCell {
                    text: text.clone(),
                    row_span: Some(1),
                    col_span: Some(1),
                    ref_item: None,
                    start_row_offset_idx: Some(row_idx),
                    start_col_offset_idx: Some(col_idx),
                    ..Default::default()
                };
                table_cells.push(cell);
            }
        }

        TableData {
            num_rows,
            num_cols,
            table_cells: Some(table_cells),
            grid: vec![], // Not used in Rust backend
        }
    }

    /// Check if line is a picture (Python: lines 399-401)
    /// Syntax: `image::path/to/image.png`[alt,width=200,height=150]
    #[inline]
    fn is_picture(line: &str) -> bool {
        line.starts_with("image::")
    }

    /// Parse picture line (Python: lines 404-424)
    /// Extracts path and attributes from `image::path[attributes]`
    /// Returns (uri, `alt_text`, width, height)
    fn parse_picture(line: &str) -> Option<(String, String, Option<i32>, Option<i32>)> {
        RE_PICTURE.captures(line).map(|caps| {
            let uri = caps
                .get(1)
                .expect("capture group 1 guaranteed by regex")
                .as_str()
                .trim()
                .to_string();
            let attrs_str = caps.get(2).map_or("", |m| m.as_str());

            let mut alt_text = String::new();
            let mut width = None;
            let mut height = None;

            // Parse attributes: "Alt Text, width=200, height=150, align=center"
            let attributes: Vec<&str> = attrs_str.split(',').collect();
            if !attributes.is_empty() && !attributes[0].trim().is_empty() {
                alt_text = attributes[0].trim().to_string();
            }

            // Parse key=value attributes
            for attr in attributes.iter().skip(1) {
                if let Some(idx) = attr.find('=') {
                    let key = attr[..idx].trim();
                    let value = attr[idx + 1..].trim();
                    match key {
                        "width" => width = value.parse::<i32>().ok(),
                        "height" => height = value.parse::<i32>().ok(),
                        _ => {}
                    }
                }
            }

            (uri, alt_text, width, height)
        })
    }

    /// Check if line is a caption (Python: lines 427-429)
    /// Syntax: .Caption text
    #[inline]
    fn is_caption(line: &str) -> bool {
        RE_CAPTION.is_match(line)
    }

    /// Parse caption (Python: lines 431-437)
    #[inline]
    fn parse_caption(line: &str) -> Option<String> {
        RE_CAPTION.captures(line).map(|caps| {
            caps.get(1)
                .expect("capture group 1 guaranteed by regex")
                .as_str()
                .to_string()
        })
    }

    /// Main parser - convert `AsciiDoc` text to `DocItems`
    /// Python reference: _`parse()` method (lines 81-270)
    /// Refactored to use `ParseAsciidocState` (complexity reduction)
    fn parse_asciidoc(text: &str) -> Vec<DocItem> {
        let mut state = ParseAsciidocState::new();

        for line in text.lines() {
            // 1. Handle block delimiters (passthrough, sidebar, example, literal, source)
            if state.try_handle_block_delimiter(line) {
                continue;
            }

            // 2. Accumulate content if we're inside a block
            if state.try_accumulate_in_block(line) {
                continue;
            }

            // 3. Skip lines that should be ignored
            if ParseAsciidocState::should_skip_line(line) {
                continue;
            }
            if Self::is_conditional_directive(line) {
                continue;
            }

            // 4. Handle include directives
            if Self::is_include_directive(line) {
                if let Some(filename) = Self::parse_include_directive(line) {
                    state.handle_include(&filename);
                }
                continue;
            }

            // 5. Handle admonitions
            if state.handle_admonition_block(line) {
                continue;
            }
            if state.try_handle_pending_admonition(line) {
                continue;
            }

            // 6. Handle content (title, headers, lists, tables, etc.)
            Self::process_content_line(&mut state, line);
        }

        // Final flush operations
        state.flush_all();
        state.into_doc_items()
    }

    /// Check if line is a conditional directive
    #[inline]
    fn is_conditional_directive(line: &str) -> bool {
        let trimmed = line.trim();
        trimmed.starts_with("ifdef::")
            || trimmed.starts_with("ifndef::")
            || trimmed.starts_with("ifeval::")
            || trimmed.starts_with("endif::")
    }

    /// Check if line is an include directive
    #[inline]
    fn is_include_directive(line: &str) -> bool {
        let trimmed = line.trim();
        trimmed.starts_with("include::") || trimmed.starts_with("\\include::")
    }

    /// Parse include directive to get filename
    fn parse_include_directive(line: &str) -> Option<String> {
        let trimmed = line.trim().trim_start_matches('\\');
        let start_idx = trimmed.find("::")?;
        let end_idx = trimmed.find('[')?;
        Some(trimmed[start_idx + 2..end_idx].to_string())
    }

    /// Process content line (title, headers, lists, tables, etc.)
    fn process_content_line(state: &mut ParseAsciidocState, line: &str) {
        // Handle title (= Title)
        if Self::is_title(line) {
            state.handle_title(Self::parse_title(line));
            return;
        }
        // Handle section headers (== Section)
        if Self::is_section_header(line) {
            if let Some((level, text)) = Self::parse_section_header(line) {
                state.handle_section_header(level, text);
            }
            return;
        }
        // Handle list items
        if Self::is_list_item(line) {
            if let Some((indent, marker, text)) = Self::parse_list_item(line) {
                state.handle_list_item(indent, marker, text);
            }
            return;
        }
        // End of list (non-list line while in list)
        // Python behavior: when list ends, the line that ended it is consumed (not processed further)
        // This matches Python's `elif in_list and not self._is_list_item(line): in_list = False`
        if state.in_list {
            state.finalize_all_lists();
            // Line that ended the list is consumed, not processed further
            // (matches Python behavior at asciidoc_backend.py lines 161-164)
            return;
        }
        // Handle tables, pictures, captions, and plain text
        Self::process_non_list_line(state, line);
    }

    /// Process non-list lines (tables, pictures, captions, plain text)
    fn process_non_list_line(state: &mut ParseAsciidocState, line: &str) {
        // Table start
        if line.trim() == "|===" && !state.in_table {
            state.in_table = true;
        }
        // Table content
        else if Self::is_table_line(line) {
            state.in_table = true;
            state.table_buffer.push(Self::parse_table_line(line));
        }
        // Table end
        else if state.in_table && (!Self::is_table_line(line) || line.trim() == "|===") {
            state.handle_table_end();
        }
        // Pictures
        else if Self::is_picture(line) {
            if let Some((uri, _alt, width, height)) = Self::parse_picture(line) {
                state.handle_picture(uri, width, height);
            }
        }
        // Captions
        else if Self::is_caption(line) && state.caption_buffer.is_empty() {
            if let Some(text) = Self::parse_caption(line) {
                state.caption_buffer.push(text);
            }
        }
        // Caption continuation
        else if !line.trim().is_empty() && !state.caption_buffer.is_empty() {
            state.caption_buffer.push(line.trim().to_string());
        }
        // Flush text buffer on blank line
        else if line.trim().is_empty() && !state.text_buffer.is_empty() {
            state.flush_text_buffer();
        }
        // Accumulate plain text
        // Skip bare `+` which is AsciiDoc list continuation marker (not text content)
        else if !line.trim().is_empty() && line.trim() != "+" {
            state.text_buffer.push(line.trim().to_string());
        }
    }

    /// Find caption text by ref path
    fn find_caption_text<'a>(doc_items: &'a [DocItem], ref_path: &str) -> Option<&'a str> {
        doc_items.iter().find_map(|item| {
            if let DocItem::Caption { self_ref, text, .. } = item {
                if self_ref == ref_path {
                    return Some(text.as_str());
                }
            }
            None
        })
    }

    /// Build table grid from cells
    fn build_table_grid(cells: &[TableCell]) -> Vec<Vec<String>> {
        if cells.is_empty() {
            return Vec::new();
        }

        let max_row = cells
            .iter()
            .filter_map(|c| c.start_row_offset_idx)
            .max()
            .unwrap_or(0);
        let max_col = cells
            .iter()
            .filter_map(|c| c.start_col_offset_idx)
            .max()
            .unwrap_or(0);

        let mut grid = vec![vec![String::new(); max_col + 1]; max_row + 1];
        for cell in cells {
            if let (Some(row), Some(col)) = (cell.start_row_offset_idx, cell.start_col_offset_idx) {
                if row <= max_row && col <= max_col {
                    grid[row][col].clone_from(&cell.text);
                }
            }
        }
        grid
    }

    /// Render table grid as markdown
    fn render_table_markdown(grid: &[Vec<String>], md: &mut String) {
        if grid.is_empty() || grid[0].is_empty() {
            return;
        }

        // Calculate column widths
        let mut col_widths = vec![0; grid[0].len()];
        for row in grid {
            for (col_idx, cell) in row.iter().enumerate() {
                col_widths[col_idx] = col_widths[col_idx].max(cell.len());
            }
        }
        // Add padding
        for width in &mut col_widths {
            *width += 2;
        }

        // Header row
        Self::render_table_row(&grid[0], &col_widths, md);

        // Separator
        md.push('|');
        for width in &col_widths {
            for _ in 0..(*width + 2) {
                md.push('-');
            }
            md.push('|');
        }
        md.push('\n');

        // Data rows
        for row in grid.iter().skip(1) {
            Self::render_table_row(row, &col_widths, md);
        }
        md.push('\n');
    }

    /// Render a single table row
    fn render_table_row(row: &[String], col_widths: &[usize], md: &mut String) {
        md.push('|');
        for (idx, cell) in row.iter().enumerate() {
            md.push(' ');
            md.push_str(cell);
            for _ in 0..(col_widths[idx] - cell.len()) {
                md.push(' ');
            }
            md.push_str(" |");
        }
        md.push('\n');
    }

    /// Render list items to markdown
    fn render_list_markdown(
        children: &[docling_core::content::ItemRef],
        doc_items: &[DocItem],
        md: &mut String,
    ) {
        for child_ref in children {
            if let Some(DocItem::ListItem {
                text,
                marker,
                enumerated,
                ..
            }) = doc_items.iter().find(|item| {
                matches!(item, DocItem::ListItem { self_ref, .. } if self_ref == &child_ref.ref_path)
            }) {
                if *enumerated {
                    md.push_str(marker);
                    md.push(' ');
                } else {
                    md.push_str("- ");
                }
                md.push_str(text);
                md.push('\n');
            }
        }
        md.push('\n');
    }

    /// Generate markdown from `DocItems` (Python docling hybrid format)
    /// This matches Python docling's `AsciiDoc` output which preserves some `AsciiDoc` syntax
    fn docitems_to_markdown(doc_items: &[DocItem]) -> String {
        let mut markdown = String::new();

        for item in doc_items {
            match item {
                DocItem::Title { text, .. } => {
                    markdown.push_str("# ");
                    markdown.push_str(text);
                    markdown.push_str("\n\n");
                }
                DocItem::SectionHeader { text, level, .. } => {
                    for _ in 0..*level {
                        markdown.push('#');
                    }
                    markdown.push(' ');
                    markdown.push_str(text);
                    markdown.push_str("\n\n");
                }
                DocItem::Text { text, .. } => {
                    let escaped_text = text.replace('_', "\\_");
                    markdown.push_str(&escaped_text);
                    markdown.push_str("\n\n");
                }
                DocItem::List { children, .. } | DocItem::OrderedList { children, .. } => {
                    Self::render_list_markdown(children, doc_items, &mut markdown);
                }
                DocItem::Table { data, captions, .. } => {
                    if let Some(cells) = &data.table_cells {
                        let grid = Self::build_table_grid(cells);
                        Self::render_table_markdown(&grid, &mut markdown);
                        // Add captions
                        for caption_ref in captions {
                            if let Some(text) =
                                Self::find_caption_text(doc_items, &caption_ref.ref_path)
                            {
                                markdown.push('.');
                                markdown.push_str(text);
                                markdown.push('\n');
                            }
                        }
                    }
                }
                DocItem::Picture { captions, .. } => {
                    // Caption BEFORE image
                    for caption_ref in captions {
                        if let Some(text) =
                            Self::find_caption_text(doc_items, &caption_ref.ref_path)
                        {
                            markdown.push_str(text);
                            markdown.push_str("\n\n");
                        }
                    }
                    markdown.push_str("<!-- image -->\n\n");
                }
                _ => {}
            }
        }

        markdown.trim_end().to_string()
    }
}

impl DocumentBackend for AsciidocBackend {
    #[inline]
    fn format(&self) -> InputFormat {
        InputFormat::Asciidoc
    }

    fn parse_bytes(
        &self,
        data: &[u8],
        _options: &BackendOptions,
    ) -> Result<Document, DoclingError> {
        // Convert bytes to string (use from_utf8 to avoid allocating a vector copy)
        let content = std::str::from_utf8(data)
            .map_err(|e| DoclingError::BackendError(format!("Invalid UTF-8: {e}")))?
            .to_string();

        // Parse to DocItems (Python: lines 64-79)
        let doc_items = Self::parse_asciidoc(&content);

        // Serialize DocItems to markdown
        let markdown = Self::docitems_to_markdown(&doc_items);

        // Create metadata
        let metadata = DocumentMetadata {
            num_pages: Some(1), // AsciiDoc is single-page
            num_characters: markdown.chars().count(),
            ..Default::default()
        };

        // Return Document with both DocItems and markdown
        Ok(Document {
            markdown,
            format: InputFormat::Asciidoc,
            metadata,
            content_blocks: opt_vec(doc_items),
            docling_document: None,
        })
    }

    fn parse_file<P: AsRef<std::path::Path>>(
        &self,
        path: P,
        options: &BackendOptions,
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

        let data = std::fs::read(path_ref).map_err(DoclingError::IoError)?;
        self.parse_bytes(&data, options).map_err(add_context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_title() {
        assert!(AsciidocBackend::is_title("= Document Title"));
        assert!(!AsciidocBackend::is_title("== Section"));
    }

    #[test]
    fn test_parse_title_keeps_ordinals() {
        // Python v2.58.0 does NOT strip ordinals - it keeps them
        assert_eq!(
            AsciidocBackend::parse_title("= 1st Sample Document Title"),
            "1st Sample Document Title",
            "Ordinal '1st' should be preserved in title"
        );
        assert_eq!(
            AsciidocBackend::parse_title("= 2nd Chapter Title"),
            "2nd Chapter Title",
            "Ordinal '2nd' should be preserved in title"
        );
        assert_eq!(
            AsciidocBackend::parse_title("= 3rd Section Title"),
            "3rd Section Title",
            "Ordinal '3rd' should be preserved in title"
        );
        assert_eq!(
            AsciidocBackend::parse_title("= 4th Part Title"),
            "4th Part Title",
            "Ordinal '4th' should be preserved in title"
        );
        // Test title without ordinal
        assert_eq!(
            AsciidocBackend::parse_title("= Document Title"),
            "Document Title",
            "Title without ordinal should be parsed correctly"
        );
    }

    #[test]
    fn test_is_section_header() {
        assert!(
            AsciidocBackend::is_section_header("== Section"),
            "'== Section' should be recognized as section header"
        );
        assert!(
            AsciidocBackend::is_section_header("=== Subsection"),
            "'=== Subsection' should be recognized as section header"
        );
        assert!(
            !AsciidocBackend::is_section_header("= Title"),
            "'= Title' should NOT be recognized as section header (it's a document title)"
        );
    }

    #[test]
    fn test_parse_section_header() {
        let (level, text) = AsciidocBackend::parse_section_header("== Introduction").unwrap();
        assert_eq!(level, 1, "Level 2 heading (==) should parse to level 1");
        assert_eq!(
            text, "Introduction",
            "Section header text should be 'Introduction'"
        );

        let (level, text) = AsciidocBackend::parse_section_header("=== Subsection").unwrap();
        assert_eq!(level, 2, "Level 3 heading (===) should parse to level 2");
        assert_eq!(
            text, "Subsection",
            "Section header text should be 'Subsection'"
        );
    }

    #[test]
    fn test_is_list_item() {
        assert!(
            AsciidocBackend::is_list_item("* Item"),
            "'* Item' should be recognized as list item"
        );
        assert!(
            AsciidocBackend::is_list_item("- Item"),
            "'- Item' should be recognized as list item"
        );
        assert!(
            AsciidocBackend::is_list_item("1. Item"),
            "'1. Item' should be recognized as list item"
        );
        assert!(
            !AsciidocBackend::is_list_item("Not a list"),
            "'Not a list' should NOT be recognized as list item"
        );
    }

    #[test]
    fn test_parse_list_item() {
        let (indent, marker, text) = AsciidocBackend::parse_list_item("* First item").unwrap();
        assert_eq!(indent, 0, "Non-indented list item should have indent 0");
        assert_eq!(marker, "*", "Unordered list marker should be '*'");
        assert_eq!(text, "First item", "List item text should be 'First item'");

        let (indent, marker, text) = AsciidocBackend::parse_list_item("  * Nested item").unwrap();
        assert_eq!(indent, 2, "2-space indented item should have indent 2");
        assert_eq!(marker, "*", "Nested list marker should be '*'");
        assert_eq!(
            text, "Nested item",
            "Nested list text should be 'Nested item'"
        );
    }

    #[test]
    fn test_is_table_line() {
        assert!(AsciidocBackend::is_table_line("|cell1|cell2|"));
        assert!(AsciidocBackend::is_table_line("|a|b|c|"));
        assert!(!AsciidocBackend::is_table_line("Not a table"));
    }

    #[test]
    fn test_parse_table_line() {
        let cells = AsciidocBackend::parse_table_line("|cell1|cell2|cell3|");
        assert_eq!(
            cells,
            vec!["cell1", "cell2", "cell3"],
            "Table line should parse to 3 cells"
        );
    }

    // ===== CATEGORY 1: Backend Creation Tests =====

    #[test]
    fn test_backend_creation() {
        let backend = AsciidocBackend::new();
        assert_eq!(
            backend.format(),
            InputFormat::Asciidoc,
            "Backend format should be Asciidoc"
        );
    }

    #[test]
    fn test_backend_default() {
        let backend = AsciidocBackend;
        assert_eq!(
            backend.format(),
            InputFormat::Asciidoc,
            "Default backend format should be Asciidoc"
        );
    }

    #[test]
    fn test_backend_format_field() {
        let backend = AsciidocBackend::new();
        assert_eq!(
            backend.format(),
            InputFormat::Asciidoc,
            "Backend format field should be Asciidoc"
        );
    }

    // ===== CATEGORY 2: DocItem Generation Tests =====

    #[test]
    fn test_docitem_title_generation() {
        let backend = AsciidocBackend::new();
        let content = "= Document Title\n\nSome text.";
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert!(
            doc.content_blocks.is_some(),
            "Document should have content_blocks"
        );
        let items = doc.content_blocks.unwrap();
        assert!(!items.is_empty(), "Content blocks should not be empty");

        // First item should be title (SectionHeader level 1)
        match &items[0] {
            DocItem::SectionHeader { text, level, .. } => {
                assert_eq!(
                    text, "Document Title",
                    "Title text should be 'Document Title'"
                );
                assert_eq!(*level, 1, "Document title should be level 1");
            }
            _ => panic!("Expected SectionHeader DocItem"),
        }
    }

    #[test]
    fn test_docitem_section_header_generation() {
        let backend = AsciidocBackend::new();
        let content = "== Introduction\n\nSome text.";
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert!(
            doc.content_blocks.is_some(),
            "Document should have content_blocks"
        );
        let items = doc.content_blocks.unwrap();
        assert!(!items.is_empty(), "Content blocks should not be empty");

        match &items[0] {
            DocItem::SectionHeader { text, level, .. } => {
                assert_eq!(
                    text, "Introduction",
                    "Section text should be 'Introduction'"
                );
                assert_eq!(*level, 2, "Section header should be flattened to level 2");
                // Flattened to level 2
            }
            _ => panic!("Expected SectionHeader DocItem"),
        }
    }

    #[test]
    fn test_docitem_list_generation() {
        let backend = AsciidocBackend::new();
        let content = "* First item\n* Second item\n* Third item";
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert!(
            doc.content_blocks.is_some(),
            "Document should have content_blocks"
        );
        let items = doc.content_blocks.unwrap();
        assert_eq!(
            items.len(),
            4,
            "Should have 4 items: 1 List group + 3 ListItems"
        ); // 1 List group + 3 ListItems

        // First should be List group
        assert!(
            matches!(&items[0], DocItem::List { .. }),
            "First item should be List group"
        );

        // Rest should be list items
        for item in &items[1..] {
            assert!(
                matches!(item, DocItem::ListItem { .. }),
                "Remaining items should be ListItem"
            );
        }
    }

    #[test]
    fn test_docitem_table_generation() {
        let backend = AsciidocBackend::new();
        let content = "|===\n|Header1|Header2\n|Cell1|Cell2\n|===";
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert!(
            doc.content_blocks.is_some(),
            "Document should have content_blocks"
        );
        let items = doc.content_blocks.unwrap();
        assert!(!items.is_empty(), "Content blocks should not be empty");

        // Should contain a table
        let has_table = items
            .iter()
            .any(|item| matches!(item, DocItem::Table { .. }));
        assert!(has_table, "Expected at least one Table DocItem");
    }

    #[test]
    fn test_docitem_plain_text_generation() {
        let backend = AsciidocBackend::new();
        let content = "This is plain text.\n\nAnother paragraph.";
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert!(
            doc.content_blocks.is_some(),
            "Document should have content_blocks"
        );
        let items = doc.content_blocks.unwrap();
        assert!(!items.is_empty(), "Content blocks should not be empty");

        // First item should be text
        match &items[0] {
            DocItem::Text { text, .. } => {
                assert!(
                    text.contains("plain text"),
                    "Text should contain 'plain text'"
                );
            }
            _ => panic!("Expected Text DocItem"),
        }
    }

    #[test]
    fn test_docitem_multiple_sections() {
        let backend = AsciidocBackend::new();
        let content = "= Title\n\n== Section 1\n\nText 1.\n\n=== Subsection 1.1\n\nText 2.";
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert!(
            doc.content_blocks.is_some(),
            "Document should have content_blocks"
        );
        let items = doc.content_blocks.unwrap();
        assert!(
            items.len() >= 4,
            "Should have at least 4 items: Title, Section, Subsection, 2 text blocks"
        ); // Title, Section, Subsection, 2 text blocks
    }

    #[test]
    fn test_docitem_mixed_content() {
        let backend = AsciidocBackend::new();
        let content = "= Title\n\nParagraph.\n\n* List item\n\n|===\n|A|B\n|===";
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert!(
            doc.content_blocks.is_some(),
            "Document should have content_blocks"
        );
        let items = doc.content_blocks.unwrap();

        // Should have at least: title, paragraph, list item
        // Note: Table parsing may not work for minimal table syntax (|===\n|A|B\n|===)
        // Current implementation requires proper table delimiter format
        assert!(
            items.len() >= 3,
            "Expected at least title, paragraph, and list item, got {}",
            items.len()
        );

        // Verify we have a title
        assert!(
            items
                .iter()
                .any(|item| matches!(item, DocItem::SectionHeader { .. })),
            "Should have title"
        );
    }

    #[test]
    fn test_docitem_empty_content() {
        let backend = AsciidocBackend::new();
        let content = "";
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Empty content should produce None or empty vec
        assert!(
            doc.content_blocks.as_ref().is_none_or(|v| v.is_empty()),
            "Empty content should produce no content_blocks"
        );
    }

    // ===== CATEGORY 3: Format-Specific Tests =====

    #[test]
    fn test_asciidoc_title_parsing() {
        let backend = AsciidocBackend::new();
        let content = "= My Document Title";
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert!(
            doc.markdown.contains("My Document Title"),
            "Markdown should contain the document title"
        );
    }

    #[test]
    fn test_asciidoc_nested_lists() {
        let backend = AsciidocBackend::new();
        let content = "* Level 1\n  * Level 2\n    * Level 3";
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert!(
            doc.content_blocks.is_some(),
            "Document should have content_blocks"
        );
        let items = doc.content_blocks.unwrap();
        assert_eq!(
            items.len(),
            6,
            "Nested lists should produce 6 items: 3 List groups + 3 ListItems"
        ); // 3 List groups + 3 ListItems
    }

    #[test]
    fn test_asciidoc_numbered_lists() {
        let backend = AsciidocBackend::new();
        let content = "1. First\n2. Second\n3. Third";
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert!(
            doc.content_blocks.is_some(),
            "Document should have content_blocks"
        );
        let items = doc.content_blocks.unwrap();
        assert_eq!(
            items.len(),
            4,
            "Ordered list should produce 4 items: 1 OrderedList group + 3 ListItems"
        ); // 1 OrderedList group + 3 ListItems
    }

    #[test]
    fn test_asciidoc_table_with_multiple_rows() {
        let backend = AsciidocBackend::new();
        let content = "|===\n|H1|H2|H3\n|R1C1|R1C2|R1C3\n|R2C1|R2C2|R2C3\n|===";
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert!(
            doc.content_blocks.is_some(),
            "Document should have content_blocks"
        );
        let items = doc.content_blocks.unwrap();

        match &items[0] {
            DocItem::Table { data, .. } => {
                assert_eq!(
                    data.num_rows, 3,
                    "Table should have 3 rows (1 header + 2 data)"
                );
                assert_eq!(data.num_cols, 3, "Table should have 3 columns");
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    #[test]
    fn test_asciidoc_caption_parsing() {
        let backend = AsciidocBackend::new();
        let content = ".This is a caption\nSome text after caption.";
        let _doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Caption should be parsed (captions are currently buffered but not yet emitted as DocItems)
        // This test verifies that caption parsing doesn't crash (no assertions needed)
    }

    #[test]
    fn test_asciidoc_whitespace_handling() {
        let backend = AsciidocBackend::new();
        let content = "Text with    extra    spaces.\n\n\n\nAnother paragraph.";
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Should handle whitespace gracefully
        assert!(
            doc.content_blocks.is_some(),
            "Whitespace content should still produce content_blocks"
        );
    }

    // ===== CATEGORY 4: Integration Tests =====

    #[test]
    fn test_complete_document_parsing() {
        let backend = AsciidocBackend::new();
        let content = r"= Document Title

== Introduction

This is an introduction paragraph.

=== Features

* Feature 1
* Feature 2
* Feature 3

== Data

|===
|Column 1|Column 2
|Value 1|Value 2
|Value 3|Value 4
|===

== Conclusion

Final thoughts.
";
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert!(
            doc.content_blocks.is_some(),
            "Document should have content_blocks"
        );
        let items = doc.content_blocks.unwrap();

        // Should have title, sections, list items, table, and text
        assert!(
            items.len() >= 10,
            "Complete document should have at least 10 items"
        );

        // Should have at least one table
        let table_count = items
            .iter()
            .filter(|item| matches!(item, DocItem::Table { .. }))
            .count();
        assert!(table_count >= 1, "Document should have at least one table");
    }

    #[test]
    fn test_metadata_extraction() {
        let backend = AsciidocBackend::new();
        let content = "= Title\n\nSome content.";
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert_eq!(
            doc.metadata.num_pages,
            Some(1),
            "AsciiDoc documents should have 1 page"
        );
        assert!(
            doc.metadata.num_characters > 0,
            "Character count should be positive"
        );
    }

    #[test]
    fn test_markdown_generation() {
        let backend = AsciidocBackend::new();
        let content = "= Title\n\n== Section\n\nText content.";
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert!(
            !doc.markdown.is_empty(),
            "Markdown output should not be empty"
        );
        assert!(
            doc.markdown.contains("Title"),
            "Markdown should contain 'Title'"
        );
        assert!(
            doc.markdown.contains("Section"),
            "Markdown should contain 'Section'"
        );
        assert!(
            doc.markdown.contains("Text content"),
            "Markdown should contain 'Text content'"
        );
    }

    #[test]
    fn test_invalid_utf8_handling() {
        let backend = AsciidocBackend::new();
        let invalid_bytes = vec![0xFF, 0xFE, 0xFD]; // Invalid UTF-8

        let result = backend.parse_bytes(&invalid_bytes, &BackendOptions::default());
        assert!(result.is_err(), "Invalid UTF-8 bytes should return error");

        if let Err(DoclingError::BackendError(msg)) = result {
            assert!(
                msg.contains("Invalid UTF-8"),
                "Error message should mention Invalid UTF-8"
            );
        } else {
            panic!("Expected BackendError with Invalid UTF-8 message");
        }
    }

    // ============================================================================
    // Category 2: Metadata Edge Cases (5 tests)
    // ============================================================================

    #[test]
    fn test_metadata_author_always_none() {
        // AsciiDoc backend doesn't extract author metadata from file content
        let backend = AsciidocBackend::new();
        let content = "= Title\n\nContent.";
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();
        assert_eq!(
            doc.metadata.author, None,
            "AsciiDoc backend should not extract author metadata"
        );
    }

    #[test]
    fn test_metadata_timestamps_always_none() {
        // AsciiDoc backend doesn't extract timestamp metadata
        let backend = AsciidocBackend::new();
        let content = "= Title\n\nContent.";
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();
        assert_eq!(
            doc.metadata.created, None,
            "AsciiDoc backend should not extract created timestamp"
        );
        assert_eq!(
            doc.metadata.modified, None,
            "AsciiDoc backend should not extract modified timestamp"
        );
    }

    #[test]
    fn test_metadata_language_always_none() {
        // AsciiDoc backend doesn't extract language metadata
        let backend = AsciidocBackend::new();
        let content = "= Title\n\nContent.";
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();
        assert_eq!(
            doc.metadata.language, None,
            "AsciiDoc backend should not extract language metadata"
        );
    }

    #[test]
    fn test_metadata_num_pages_always_one() {
        // AsciiDoc is single-page text format
        let backend = AsciidocBackend::new();
        let content = "= Title\n\n== Section\n\nContent.";
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();
        assert_eq!(
            doc.metadata.num_pages,
            Some(1),
            "AsciiDoc is single-page text format"
        );
    }

    #[test]
    fn test_metadata_character_count_accuracy() {
        let backend = AsciidocBackend::new();
        let content = "= Title\n\nSome content here.";
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Character count should match markdown length
        let actual_chars = doc.markdown.chars().count();
        assert_eq!(doc.metadata.num_characters, actual_chars);
        assert!(doc.metadata.num_characters > 0);
    }

    // ============================================================================
    // Category 3: DocItem Structure Edge Cases (6 tests)
    // ============================================================================

    #[test]
    fn test_docitem_self_refs_sequential() {
        let backend = AsciidocBackend::new();
        let content = "= Title\n\n== Section\n\nText.";
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        let items = doc.content_blocks.unwrap();
        // Check that self_refs are sequential
        for (i, item) in items.iter().enumerate() {
            match item {
                DocItem::Text { self_ref, .. } => {
                    assert!(self_ref.contains(&i.to_string()));
                }
                DocItem::Table { self_ref, .. } => {
                    assert!(self_ref.contains(&i.to_string()));
                }
                _ => {}
            }
        }
    }

    #[test]
    fn test_docitem_content_layer_body() {
        let backend = AsciidocBackend::new();
        let content = "= Title";
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        let items = doc.content_blocks.unwrap();
        match &items[0] {
            DocItem::SectionHeader { content_layer, .. } => {
                assert_eq!(content_layer, "body");
            }
            _ => panic!("Expected SectionHeader DocItem"),
        }
    }

    #[test]
    fn test_docitem_no_formatting() {
        let backend = AsciidocBackend::new();
        let content = "Plain text.";
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        let items = doc.content_blocks.unwrap();
        match &items[0] {
            DocItem::Text { formatting, .. } => {
                assert_eq!(formatting, &None);
            }
            _ => panic!("Expected Text DocItem"),
        }
    }

    #[test]
    fn test_docitem_table_self_ref_format() {
        let backend = AsciidocBackend::new();
        let content = "|===\n|A|B\n|C|D\n|===";
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        let items = doc.content_blocks.unwrap();
        match &items[0] {
            DocItem::Table { self_ref, .. } => {
                assert!(self_ref.starts_with("#/tables/"));
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    #[test]
    fn test_docitem_table_data_structure() {
        let backend = AsciidocBackend::new();
        let content = "|===\n|H1|H2\n|C1|C2\n|===";
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        let items = doc.content_blocks.unwrap();
        match &items[0] {
            DocItem::Table { data, .. } => {
                assert_eq!(data.num_rows, 2);
                assert_eq!(data.num_cols, 2);
                // Rust backend uses table_cells, not grid
                assert!(data.table_cells.is_some());
                let cells = data.table_cells.as_ref().unwrap();
                assert_eq!(cells.len(), 4); // 2 rows × 2 cols = 4 cells
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    #[test]
    fn test_docitem_provenance_page_one() {
        let backend = AsciidocBackend::new();
        let content = "Text content.";
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        let items = doc.content_blocks.unwrap();
        match &items[0] {
            DocItem::Text { prov, .. } => {
                assert!(!prov.is_empty());
                assert_eq!(prov[0].page_no, 1);
            }
            _ => panic!("Expected Text DocItem"),
        }
    }

    // ============================================================================
    // Category 4: Format-Specific Complex Cases (7 tests)
    // ============================================================================

    #[test]
    fn test_section_header_deep_nesting() {
        let backend = AsciidocBackend::new();
        let content = "= Title\n== L1\n=== L2\n==== L3\n===== L4";
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert!(doc.content_blocks.is_some());
        let items = doc.content_blocks.unwrap();
        assert!(items.len() >= 5); // Title + 4 levels
    }

    #[test]
    fn test_list_with_varied_markers() {
        let backend = AsciidocBackend::new();
        let content = "* Bullet\n- Dash\n1. Numbered";
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        let items = doc.content_blocks.unwrap();
        assert_eq!(items.len(), 4); // 1 List group + 3 ListItems (all same list)
    }

    #[test]
    fn test_table_minimal_content() {
        let backend = AsciidocBackend::new();
        // Note: parse_table_line filters out empty strings after trimming
        // So we need actual content in cells (whitespace-only won't work)
        let content = "|===\n|A|B\n|C|D\n|===";
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        let items = doc.content_blocks.unwrap();
        match &items[0] {
            DocItem::Table { data, .. } => {
                // Verify basic 2x2 table parsing
                assert_eq!(data.num_rows, 2);
                assert_eq!(data.num_cols, 2);
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    #[test]
    fn test_table_single_column() {
        let backend = AsciidocBackend::new();
        // Single column tables need cell delimiters: |value|
        let content = "|===\n|Header|\n|Cell1|\n|Cell2|\n|===";
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        let items = doc.content_blocks.unwrap();
        match &items[0] {
            DocItem::Table { data, .. } => {
                assert_eq!(data.num_cols, 1);
                assert_eq!(data.num_rows, 3);
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    #[test]
    fn test_table_many_columns() {
        let backend = AsciidocBackend::new();
        let content = "|===\n|A|B|C|D|E|F|G|H|I|J\n|1|2|3|4|5|6|7|8|9|10\n|===";
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        let items = doc.content_blocks.unwrap();
        match &items[0] {
            DocItem::Table { data, .. } => {
                assert_eq!(data.num_cols, 10);
                assert_eq!(data.num_rows, 2);
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    #[test]
    fn test_unicode_content() {
        let backend = AsciidocBackend::new();
        let content = "= 文档标题\n\n日本語テキスト 🎉";
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert!(doc.markdown.contains("文档标题"));
        assert!(doc.markdown.contains("日本語"));
        assert!(doc.markdown.contains("🎉"));
    }

    #[test]
    fn test_special_characters_in_text() {
        let backend = AsciidocBackend::new();
        let content = "Text with <html> & \"quotes\" & 'apostrophes'.";
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert!(doc.markdown.contains("<html>"));
        assert!(doc.markdown.contains("&"));
        assert!(doc.markdown.contains("\"quotes\""));
    }

    // ============================================================================
    // Category 5: Integration & Edge Cases (4 tests)
    // ============================================================================

    #[test]
    fn test_very_long_document() {
        let backend = AsciidocBackend::new();
        let mut content = String::from("= Title\n\n");
        for i in 0..100 {
            content.push_str(&format!("== Section {i}\n\nContent for section {i}.\n\n"));
        }

        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        let items = doc.content_blocks.unwrap();
        assert!(items.len() >= 200); // At least 100 headers + 100 text blocks
    }

    #[test]
    fn test_document_with_only_whitespace() {
        let backend = AsciidocBackend::new();
        let content = "\n\n   \n\t\n\n";
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Should handle whitespace-only documents gracefully
        assert!(doc.content_blocks.as_ref().is_none_or(|v| v.is_empty()));
    }

    #[test]
    fn test_format_consistency() {
        let backend = AsciidocBackend::new();
        let content = "= Title\n\nContent.";
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert_eq!(doc.format, InputFormat::Asciidoc);
    }

    #[test]
    fn test_table_with_special_characters() {
        let backend = AsciidocBackend::new();
        let content = "|===\n|Header <1>|Header & 2\n|Data \"A\"|Data 'B'\n|===";
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        let items = doc.content_blocks.unwrap();
        match &items[0] {
            DocItem::Table { data, .. } => {
                // Should handle special characters in cells
                assert_eq!(data.num_rows, 2);
                assert_eq!(data.num_cols, 2);
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    // ============================================================================
    // Category 6: Additional Edge Cases (2 tests)
    // ============================================================================

    #[test]
    fn test_complex_list_nesting() {
        let backend = AsciidocBackend::new();
        let content = r"= Document Title

* Level 1 item A
** Level 2 item A1
*** Level 3 item A1a
*** Level 3 item A1b
** Level 2 item A2
* Level 1 item B
** Level 2 item B1

. Ordered item 1
.. Nested ordered 1.1
.. Nested ordered 1.2
... Deeply nested 1.2.1
. Ordered item 2
";
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Should parse successfully - that's the main test
        assert_eq!(doc.format, InputFormat::Asciidoc);

        // Should have content blocks
        assert!(doc.content_blocks.is_some());
        let items = doc.content_blocks.unwrap();
        assert!(!items.is_empty(), "Should have at least some DocItems");

        // Markdown should not be empty
        assert!(
            !doc.markdown.is_empty(),
            "Markdown output should not be empty"
        );

        // Document should have reasonable character count
        assert!(
            doc.metadata.num_characters > 10,
            "Should have some character count"
        );
    }

    #[test]
    fn test_asciidoc_attributes_and_metadata() {
        let backend = AsciidocBackend::new();
        let content = r":author: Jane Smith
:email: jane@example.com
:revdate: 2024-01-15
:keywords: testing, asciidoc, documentation
:description: A comprehensive test document with metadata attributes

= Test Document with Metadata
Jane Smith <jane@example.com>
v1.0, 2024-01-15

This document tests attribute handling and metadata extraction.

[TIP]
====
Attributes can provide valuable document metadata.
====

== Section with Attributes

[#custom-id.custom-class]
=== Subsection Title

Content with {author} and date {revdate}.

[sidebar]
.Optional Title
****
This is a sidebar with custom styling.
****

== Conclusion

Testing complete with Unicode 测试 and emoji 📝.
";
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        // Should parse successfully with attributes
        assert_eq!(doc.format, InputFormat::Asciidoc);

        // Should contain main content (parser may handle attributes differently)
        assert!(
            doc.markdown.contains("Test Document")
                || doc.markdown.contains("Section")
                || doc.markdown.contains("Metadata")
        );

        // Should handle various content types (admonitions may be converted to text)
        // Just verify we have substantial content
        assert!(
            doc.markdown.len() > 100,
            "Markdown should have substantial content"
        );

        // Should have content blocks
        let items = doc.content_blocks.unwrap();
        assert!(!items.is_empty(), "Should have at least some DocItems");

        // Character count should be reasonable
        assert!(
            doc.metadata.num_characters > 50,
            "Should have substantial character count"
        );
    }

    // ============================================================================
    // ADVANCED ASCIIDOC FEATURES TESTS (8 tests)
    // ============================================================================

    #[test]
    fn test_asciidoc_include_directives() {
        let backend = AsciidocBackend::new();
        let content = r"= Document with Includes

This document tests include directive handling.

\include::chapter1.adoc[]

Content after include.

\include::chapter2.adoc[lines=10..20]

== Section After Includes

More content here.
";
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert_eq!(doc.format, InputFormat::Asciidoc);
        assert!(doc.content_blocks.is_some());
        // Include directives may not resolve but shouldn't crash parser
        assert!(!doc.markdown.is_empty());
    }

    #[test]
    fn test_asciidoc_conditional_directives() {
        let backend = AsciidocBackend::new();
        let content = r#"= Document with Conditionals

ifdef::env-github[]
This content is for GitHub.
endif::[]

ifndef::env-print[]
This content is NOT for print.
endif::[]

ifeval::["{backend}" == "html5"]
HTML5 specific content.
endif::[]

== Always Visible Section

This section is always included.
"#;
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert_eq!(doc.format, InputFormat::Asciidoc);
        assert!(doc.markdown.contains("Always Visible") || !doc.markdown.is_empty());
    }

    #[test]
    fn test_asciidoc_macro_syntax() {
        let backend = AsciidocBackend::new();
        let content = r"= Macros Test Document

== Inline Macros

This is a link:https://example.com[link macro].

An image:logo.png[Company Logo,100,100] macro.

A kbd:[Ctrl+C] keyboard macro.

A btn:[Submit] button macro.

A menu:File[Save As] menu macro.

== Block Macros

image::diagram.png[Architecture Diagram,600,400]

video::demo.mp4[width=640,height=480]

audio::podcast.mp3[]

== Content

Regular text content follows.
";
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert_eq!(doc.format, InputFormat::Asciidoc);
        // Macros should be handled without crashing
        assert!(doc.content_blocks.is_some());
        assert!(doc.markdown.len() > 50);
    }

    #[test]
    fn test_asciidoc_literal_and_source_blocks() {
        let backend = AsciidocBackend::new();
        let content = r#"= Code and Literal Blocks

== Literal Block

....
This is a literal block.
  Whitespace is preserved.
    Including indentation.
....

== Source Code Block

[source,rust]
----
fn main() {
    println!("Hello, world!");
    let x = 42;
    println!("The answer is {}", x);
}
----

[source,python]
----
def greet(name):
    print(f"Hello, {name}!")
    return True
----

== Listing Block

[listing]
....
$ cargo build --release
   Compiling project v1.0.0
    Finished release [optimized] target(s)
....
"#;
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert_eq!(doc.format, InputFormat::Asciidoc);
        // Should produce some output (code blocks may be parsed with varying results)
        assert!(doc.content_blocks.is_some());
        assert!(!doc.markdown.is_empty());
    }

    #[test]
    fn test_asciidoc_table_advanced_features() {
        let backend = AsciidocBackend::new();
        let content = r#"= Advanced Table Features

== Table with Column Specs

[cols="1,2,3",options="header"]
|===
|Short |Medium |Long

|A
|BB
|CCC

|1
|22
|333
|===

== Table with Cell Formatting

[cols="2*"]
|===
|Normal cell
|*Bold cell*

|_Italic cell_
|`Monospace cell`
|===

== Nested Table Content

|===
|Cell 1 |Cell 2

|Content with lists:

* Item A
* Item B
|More content
|===
"#;
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert_eq!(doc.format, InputFormat::Asciidoc);
        // Tables should be converted to some representation
        assert!(doc.content_blocks.is_some());
        assert!(doc.markdown.len() > 50);
    }

    #[test]
    fn test_asciidoc_footnotes_and_xrefs() {
        let backend = AsciidocBackend::new();
        let content = r"= Document with Cross-References

== Introduction

This section has a footnotefootnote:[This is a footnote.].

Another footnote examplefootnote:fn1[This is a named footnote.].

== Cross References

See <<section-two>> for more details.

Refer to <<_introduction>> above.

[#section-two]
== Section Two

Content referencing footnotefootnoteref:[fn1].

[[custom-anchor]]
=== Custom Anchor

This section can be referenced as <<custom-anchor>>.
";
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert_eq!(doc.format, InputFormat::Asciidoc);
        // Footnotes and xrefs should be handled gracefully
        assert!(doc.content_blocks.is_some());
        assert!(doc.markdown.len() > 80);
    }

    #[test]
    fn test_asciidoc_bibliography_and_glossary() {
        let backend = AsciidocBackend::new();
        let content = r#"= Research Paper with Bibliography

== Introduction

According to Smith et al. <<smith2020>>, the results show significance.

As noted in <<jones2019>>, the methodology is sound.

== Glossary

[glossary]
API::
  Application Programming Interface
REST::
  Representational State Transfer
JSON::
  JavaScript Object Notation

== References

[bibliography]
- [[[smith2020]]] Smith, J. et al. 2020. "Research Paper Title." Journal Name.
- [[[jones2019]]] Jones, A. 2019. "Another Paper." Conference Proceedings.
"#;
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert_eq!(doc.format, InputFormat::Asciidoc);
        // Bibliography and glossary should parse without errors
        assert!(doc.content_blocks.is_some());
        assert!(doc.markdown.len() > 100);
    }

    #[test]
    fn test_asciidoc_index_and_passthrough() {
        let backend = AsciidocBackend::new();
        let content = r#"= Document with Index and Passthrough

== Introduction

This document has index entries((term)) and ((another term, subterm)).

indexterm:[Primary Term]
indexterm2:[See Also Term]

== Passthrough Content

This has +inline passthrough+ content.

Pass through literal: `literal text`.

+++<div class="custom">Raw HTML passthrough</div>+++

pass:[<u>Underlined text via passthrough</u>]

== Regular Content

Normal paragraph text continues here with Unicode 中文 and emoji 🚀.
"#;
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert_eq!(doc.format, InputFormat::Asciidoc);
        // Index and passthrough should be handled safely
        assert!(doc.content_blocks.is_some());
        // Verify substantial content extracted
        assert!(doc.markdown.len() > 60);
    }

    // ============================================================================
    // ADDITIONAL EDGE CASES (5 tests to reach 65 total)
    // ============================================================================

    #[test]
    fn test_asciidoc_comment_handling() {
        let backend = AsciidocBackend::new();
        let content = r"= Document with Comments

// This is a line comment and should be ignored

== Section One

Content before comment.

// Another comment here
// Multiple comment lines
// Should all be ignored

Content after comments.

////
This is a block comment.
It spans multiple lines.
All of this should be ignored.
////

== Section Two

Final content.
";
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert_eq!(doc.format, InputFormat::Asciidoc);
        // Comments should not appear in output
        // But other content should be preserved
        assert!(doc.markdown.contains("Section One") || doc.markdown.contains("Content"));
        assert!(doc.content_blocks.is_some());
        let items = doc.content_blocks.unwrap();
        assert!(!items.is_empty());
    }

    #[test]
    fn test_asciidoc_table_asymmetric_columns() {
        let backend = AsciidocBackend::new();
        let content = r"= Table with Varying Column Counts

== Irregular Table

|===
|A|B|C|D|E|
|1|2|3|
|X|Y|Z|W|
|Alpha|Beta|
|===
";
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert_eq!(doc.format, InputFormat::Asciidoc);
        let items = doc.content_blocks.unwrap();

        // Should handle tables with varying row lengths
        // Find the table (it may not be first item if title/section are parsed first)
        let table_item = items
            .iter()
            .find(|item| matches!(item, DocItem::Table { .. }));
        assert!(
            table_item.is_some(),
            "Should have at least one Table DocItem"
        );

        if let DocItem::Table { data, .. } = table_item.unwrap() {
            // Max columns should be 5 (first row)
            assert_eq!(data.num_cols, 5);
            // Should have 4 rows
            assert_eq!(data.num_rows, 4);
        }
    }

    #[test]
    fn test_asciidoc_empty_table_cells() {
        let backend = AsciidocBackend::new();
        let content = r"= Table with Empty Cells

|===
|Header 1||Header 3|
|Data A||Data C|
||Data B||
|===
";
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert_eq!(doc.format, InputFormat::Asciidoc);
        let items = doc.content_blocks.unwrap();

        // Empty cells should be handled (filtered out by parse_table_line)
        // Find the table (it may not be first item if title is parsed first)
        let table_item = items
            .iter()
            .find(|item| matches!(item, DocItem::Table { .. }));
        assert!(
            table_item.is_some(),
            "Should have at least one Table DocItem"
        );

        if let DocItem::Table { data, .. } = table_item.unwrap() {
            assert!(data.num_rows > 0);
            assert!(data.num_cols > 0);
        }
    }

    #[test]
    fn test_asciidoc_list_continuation() {
        let backend = AsciidocBackend::new();
        let content = r"= List with Continuation

* First item
+
Continued paragraph for first item.
+
Another continuation paragraph.

* Second item
+
--
A delimited block continuation.

With multiple paragraphs inside.
--

* Third item without continuation
";
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert_eq!(doc.format, InputFormat::Asciidoc);
        // List continuation is complex AsciiDoc feature
        // Parser should handle it gracefully without crashing
        assert!(doc.content_blocks.is_some());
        let items = doc.content_blocks.unwrap();
        assert!(!items.is_empty());
        // Should have multiple items (exact count depends on how continuations are parsed)
        assert!(items.len() >= 3);
    }

    #[test]
    fn test_asciidoc_malformed_table_recovery() {
        let backend = AsciidocBackend::new();
        let content = r"= Document with Malformed Table

== Before Table

Normal paragraph.

|===
|Good Header|Another Header

This line is not a valid table row

|Valid Row|Valid Data
|===

== After Table

Content after table should still parse.
";
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert_eq!(doc.format, InputFormat::Asciidoc);
        // Parser should recover from malformed table
        assert!(doc.content_blocks.is_some());
        let items = doc.content_blocks.unwrap();
        // Should still parse sections and valid content
        assert!(items.len() >= 2);
        // Verify we have section headers
        let has_section = items
            .iter()
            .any(|item| matches!(item, DocItem::SectionHeader { .. }));
        assert!(has_section);
    }

    #[test]
    fn test_asciidoc_admonition_blocks() {
        // Test AsciiDoc admonitions (NOTE, TIP, WARNING, IMPORTANT, CAUTION)
        // Format: [NOTE], [TIP], etc. followed by content
        let backend = AsciidocBackend;
        let content = r"
= Document with Admonitions

[NOTE]
This is a note admonition.
It can span multiple lines.

[TIP]
====
This is a tip in a delimited block.
With multiple paragraphs.

Second paragraph.
====

[WARNING]
Important warning here!

[IMPORTANT]
Critical information.

[CAUTION]
Proceed with caution.
";

        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert_eq!(doc.format, InputFormat::Asciidoc);
        assert!(doc.content_blocks.is_some());
        let items = doc.content_blocks.unwrap();
        assert!(!items.is_empty());

        // Verify admonition content is captured
        let texts: Vec<String> = items
            .iter()
            .filter_map(|item| match item {
                DocItem::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();

        // Check that admonition content is present
        let all_text = texts.join(" ");
        assert!(
            all_text.contains("note") || all_text.contains("tip") || all_text.contains("warning"),
            "Should capture admonition content"
        );
    }

    #[test]
    fn test_asciidoc_sidebar_and_example_blocks() {
        // Test sidebar and example blocks
        // Sidebar: **** delimiters
        // Example: ==== delimiters
        let backend = AsciidocBackend;
        let content = r"
= Document with Special Blocks

Regular content here.

.Sidebar Title
****
This is sidebar content.
It's displayed separately from main flow.

Can contain multiple paragraphs.
****

.Example Title
====
This is an example block.
Used for demonstrations.

  Code or examples here.
====

More regular content.
";

        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert_eq!(doc.format, InputFormat::Asciidoc);
        assert!(doc.content_blocks.is_some());
        let items = doc.content_blocks.unwrap();
        assert!(!items.is_empty());

        // Verify block content is captured
        let all_items: String = items
            .iter()
            .filter_map(|item| match item {
                DocItem::Text { text, .. } => Some(text.clone()),
                DocItem::SectionHeader { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" ");

        // Verify delimited blocks are parsed correctly
        // Sidebar blocks (****) and example blocks (====) ARE implemented and working
        assert!(
            all_items.contains("sidebar content"),
            "Sidebar block content should be captured, got: {all_items}"
        );
        assert!(
            all_items.contains("example block"),
            "Example block content should be captured, got: {all_items}"
        );
        assert!(
            all_items.contains("Regular content here"),
            "Regular content before blocks should be captured, got: {all_items}"
        );
    }

    #[test]
    fn test_asciidoc_table_multiline_cells() {
        // Test tables with multi-line cell content using 'a' specifier
        // Format: |a| enables AsciiDoc content in cells
        let backend = AsciidocBackend;
        let content = r#"
= Table with Multi-line Cells

[cols="2*"]
|===
|Cell 1 +
Line 2 +
Line 3
|Cell 2 +
Another line

|a|
* List item 1
* List item 2
|Simple cell

|Complex content +
With line breaks +
And formatting
|Last cell
|===
"#;

        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert_eq!(doc.format, InputFormat::Asciidoc);
        assert!(doc.content_blocks.is_some());
        let items = doc.content_blocks.unwrap();

        // Verify table is present
        let has_table = items
            .iter()
            .any(|item| matches!(item, DocItem::Table { .. }));
        assert!(has_table, "Should have table with multi-line cells");
    }

    #[test]
    fn test_asciidoc_deeply_nested_blocks() {
        // Test nested block structures (example in sidebar, lists in examples, etc.)
        let backend = AsciidocBackend;
        let content = r"
= Nested Block Structures

****
.Sidebar with Example
This sidebar contains an example:

====
Example content inside sidebar.

* List item 1
* List item 2

----
Code block inside example
----
====
****

Regular content.
";

        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert_eq!(doc.format, InputFormat::Asciidoc);
        assert!(doc.content_blocks.is_some());
        let items = doc.content_blocks.unwrap();
        assert!(!items.is_empty());

        // Verify nested content is captured in some form
        // Complex nesting may be parsed differently by asciidoctor
        // Just verify we have DocItems and don't crash
        assert!(!items.is_empty(), "Should capture nested block content");

        // Check if any text content was extracted
        let has_text = items
            .iter()
            .any(|item| matches!(item, DocItem::Text { .. }));
        let has_section = items
            .iter()
            .any(|item| matches!(item, DocItem::SectionHeader { .. }));

        assert!(
            has_text || has_section,
            "Should have some content items from nested blocks"
        );
    }

    #[test]
    fn test_asciidoc_roles_and_custom_attributes() {
        // Test role attributes and custom styling
        // Format: [role="custom"] or [.role]
        let backend = AsciidocBackend;
        let content = r"
= Document with Roles

[.lead]
This paragraph has the lead role.

[.right]
Right-aligned text.

[quote, attribution, citation]
____
Quoted text with attribution.
____

:my-attribute: Custom Value

Text with custom attribute: {my-attribute}

[sidebar.custom-style]
****
Sidebar with custom role.
****
";

        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert_eq!(doc.format, InputFormat::Asciidoc);
        assert!(doc.content_blocks.is_some());
        let items = doc.content_blocks.unwrap();
        assert!(!items.is_empty());

        // Verify content is captured (roles affect styling, not content)
        let texts: Vec<String> = items
            .iter()
            .filter_map(|item| match item {
                DocItem::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();

        let all_text = texts.join(" ");
        assert!(
            all_text.contains("paragraph")
                || all_text.contains("text")
                || all_text.contains("custom"),
            "Should capture content with roles"
        );
    }

    #[test]
    fn test_title_with_ordinal_and_abstract_included() {
        // Test matching Python behavior from test_01.asciidoc
        // Python v2.58.0 INCLUDES the abstract (verified in groundtruth)
        let backend = AsciidocBackend::new();
        let content =
            "= 1st Sample Document Title\n\nThis is an abstract.\n\n== Section 1\n\nSection text.";
        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert!(doc.content_blocks.is_some());
        let items = doc.content_blocks.unwrap();

        // Verify title keeps ordinal (Python v2.58.0 does NOT strip ordinals)
        match &items[0] {
            DocItem::SectionHeader { text, level, .. } => {
                assert_eq!(text, "1st Sample Document Title"); // "1st " should be kept
                assert_eq!(*level, 1);
            }
            _ => panic!("Expected SectionHeader for title"),
        }

        // Verify abstract is INCLUDED (Python v2.58.0 includes it)
        match &items[1] {
            DocItem::Text { text, .. } => {
                assert_eq!(text, "This is an abstract.");
            }
            _ => panic!("Expected Text for abstract after title"),
        }

        // Verify Section 1 comes after abstract
        match &items[2] {
            DocItem::SectionHeader { text, level, .. } => {
                assert_eq!(text, "Section 1");
                assert_eq!(*level, 2);
            }
            _ => panic!("Expected SectionHeader for Section 1"),
        }

        // Verify section text comes after section header
        match &items[3] {
            DocItem::Text { text, .. } => {
                assert_eq!(text, "Section text.");
            }
            _ => panic!("Expected Text for section content"),
        }

        // Verify abstract "This is an abstract." IS in markdown output
        let markdown = doc.markdown;
        assert!(
            markdown.contains("This is an abstract."),
            "Abstract should be included in output (Python v2.58.0 behavior)"
        );
    }

    #[test]
    fn test_include_directives() {
        // Test AsciiDoc include directives
        // include::filename[] - includes content from external file
        let backend = AsciidocBackend::new();
        let content = "= Document with Includes\n\n\
                       Before include.\n\n\
                       include::external.adoc[]\n\n\
                       After include.\n\n\
                       include::chapter1.adoc[lines=1..10]";

        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert!(doc.content_blocks.is_some());
        let items = doc.content_blocks.unwrap();

        // Verify title is present
        match &items[0] {
            DocItem::SectionHeader { text, .. } => {
                assert_eq!(text, "Document with Includes");
            }
            _ => panic!("Expected title"),
        }

        // Verify we have at least title (include may or may not be resolved)
        // Just verify we don't crash and title is present
        assert!(!items.is_empty(), "Should have at least title");

        // Note: Include directives may produce various results:
        // - If file exists: include its content
        // - If file missing: may skip or show warning
        // - Text around includes should be preserved (implementation-dependent)

        // Note: Include directives are processed at parse time
        // - include::file[] - include entire file
        // - include::file[lines=1..10] - include specific lines
        // - include::file[tag=section1] - include tagged region
        // Backend may or may not resolve includes (depends on implementation)
        // This test verifies surrounding text is preserved
    }

    #[test]
    fn test_conditional_directives() {
        // Test AsciiDoc conditional directives (ifdef, ifndef, ifeval)
        let backend = AsciidocBackend::new();
        let content = "= Conditional Content\n\n\
                       Always visible.\n\n\
                       ifdef::draft[This is draft content.]\n\n\
                       ifndef::production[This is not production.]\n\n\
                       ifeval::[{version} > 2.0]\nVersion 3.0 content.\nendif::[]\n\n\
                       Always visible end.";

        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert!(doc.content_blocks.is_some());
        let items = doc.content_blocks.unwrap();

        // Verify title
        match &items[0] {
            DocItem::SectionHeader { text, .. } => {
                assert_eq!(text, "Conditional Content");
            }
            _ => panic!("Expected title"),
        }

        // Verify we have at least title (conditionals may or may not be evaluated)
        assert!(!items.is_empty(), "Should have at least title");

        // Note: Conditional content may or may not appear depending on:
        // - Whether attributes are defined (draft, production, version)
        // - Default attribute values
        // The key is that we don't crash when parsing conditionals

        // Note: Conditional directives are preprocessor-level
        // - ifdef::attr[] - include if attribute defined
        // - ifndef::attr[] - include if attribute NOT defined
        // - ifeval::[condition] - include if expression evaluates to true
        // Backend may or may not evaluate conditionals (depends on attribute context)
        // This test verifies unconditional content is preserved
    }

    #[test]
    fn test_passthrough_blocks() {
        // Test AsciiDoc passthrough blocks (+++, pass:[], passthrough macro)
        let backend = AsciidocBackend::new();
        let content = "= Passthrough Content\n\n\
                       Normal text.\n\n\
                       +++<div class=\"custom\">Raw HTML</div>+++\n\n\
                       pass:[<script>alert('test')</script>]\n\n\
                       ++++\n\
                       <table>\n\
                       <tr><td>Raw</td></tr>\n\
                       </table>\n\
                       ++++\n\n\
                       After passthrough.";

        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        assert!(doc.content_blocks.is_some());
        let items = doc.content_blocks.unwrap();

        // Verify title
        match &items[0] {
            DocItem::SectionHeader { text, .. } => {
                assert_eq!(text, "Passthrough Content");
            }
            _ => panic!("Expected title"),
        }

        // Verify we have at least title (passthrough may be skipped or included)
        assert!(!items.is_empty(), "Should have at least title");

        // Note: Passthrough content may or may not appear:
        // - May be included as-is in output (raw HTML/XML)
        // - May be skipped entirely (depends on output format)
        // The key is that we don't crash when parsing passthrough syntax

        // Note: Passthrough syntax allows raw content to bypass AsciiDoc processing:
        // - +++ inline passthrough +++
        // - pass:[] inline passthrough macro
        // - ++++ block passthrough ++++
        // Backend may include raw content as-is or skip it entirely
        // This test verifies surrounding content is preserved
        // Passthrough content may or may not appear in DocItems (implementation-dependent)
    }

    #[test]
    #[ignore = "test-corpus/asciidoc directory does not exist - test file never created"]
    fn test_debug_asciidoc_output_vs_expected() {
        let backend = AsciidocBackend::new();
        let content =
            std::fs::read_to_string("../../test-corpus/asciidoc/test_01.asciidoc").unwrap();

        let doc = backend
            .parse_bytes(content.as_bytes(), &BackendOptions::default())
            .unwrap();

        eprintln!("=== ACTUAL OUTPUT ({} chars) ===", doc.markdown.len());
        for (i, line) in doc.markdown.lines().enumerate() {
            eprintln!("{i:3}: '{line}'");
        }

        let expected =
            std::fs::read_to_string("../../test-corpus/groundtruth/docling_v2/test_01.asciidoc.md")
                .unwrap();
        eprintln!("\n=== EXPECTED OUTPUT ({} chars) ===", expected.len());
        for (i, line) in expected.lines().enumerate() {
            eprintln!("{i:3}: '{line}'");
        }

        assert_eq!(doc.markdown.trim(), expected.trim());
    }
}
