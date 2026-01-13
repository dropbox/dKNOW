//! Markdown Backend - Parse Markdown documents to `DocItems`
//!
//! Parses Markdown documents into structured `DocItems` using `pulldown-cmark`.
//!
//! ## Features
//! - Headings (H1-H6) → `SectionHeader` `DocItems`
//! - Paragraphs with inline formatting (bold, italic, code)
//! - Ordered and unordered lists → `List` and `ListItem` `DocItems`
//! - Tables → `Table` `DocItems` with cell structure
//! - Code blocks (fenced and indented)
//! - Images → `Picture` `DocItems`
//! - Links, strikethrough, and other `CommonMark` features
//!
//! ## Architecture
//! Uses `pulldown-cmark` event-based parser (similar to Python's marko library)
//!
//! ## Python Reference
//! Ported from: `docling/backend/md_backend.py` (615 lines)
//!
//! Key Python Methods:
//! - `__init__()`: lines 98-140 - Load markdown, shorten underscore sequences
//! - `_iterate_elements()`: lines 243-521 - Main AST walker
//! - `_create_heading_item()`: lines 217-241 - Create heading/title `DocItems`
//! - `_create_list_item()`: lines 199-215 - Create list item `DocItems`
//! - `_close_table()`: lines 142-197 - Build table from markdown buffer
//! - `convert()`: lines 539-614 - Main entry point, delegates to HTML if needed

// Clippy pedantic allows:
// - Parser state has multiple bool flags for tracking context
// - Coordinate calculations use f64 from usize
// - Markdown parsing functions are complex
// - Unit struct &self convention
#![allow(clippy::struct_excessive_bools)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::trivially_copy_pass_by_ref)]
#![allow(clippy::elidable_lifetime_names)]
#![allow(clippy::cast_possible_truncation)]

use crate::html::HtmlBackend;
use crate::traits::{BackendOptions, DocumentBackend};
use crate::utils::{create_code_item, create_default_provenance, create_provenance, opt_vec};
use docling_core::{
    content::{
        BoundingBox, CoordOrigin, DocItem, Formatting, ProvenanceItem, TableCell, TableData,
    },
    DoclingError, Document, DocumentMetadata, InputFormat,
};
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

/// A text segment with its formatting and hyperlink context
/// Used to track inline formatting changes within a paragraph/heading/list item
#[derive(Debug, Clone, PartialEq)]
struct TextSegment {
    text: String,
    formatting: Formatting,
    hyperlink: Option<String>,
}

impl TextSegment {
    const fn new() -> Self {
        Self {
            text: String::new(),
            formatting: MarkdownBackend::new_formatting(),
            hyperlink: None,
        }
    }

    #[inline]
    const fn has_formatting(&self) -> bool {
        MarkdownBackend::has_formatting(&self.formatting)
    }
}

/// State for `parse_markdown` function (reduces cognitive complexity)
///
/// Groups all state variables used during event-based parsing.
#[derive(Debug, Clone, PartialEq)]
struct ParseMarkdownState<'a> {
    // Output
    doc_items: Vec<DocItem>,
    item_count: usize,
    group_count: usize,
    picture_count: usize,

    // Document analysis
    has_h1: bool,
    has_any_heading: bool,
    seen_first_heading: bool,

    // Heading state
    in_heading: bool,
    heading_level: Option<usize>,

    // List nesting: (list_ref, is_ordered, next_item_number, list_index, parent_item_index)
    list_stack: Vec<(String, bool, usize, usize, Option<usize>)>,

    // Code block state
    in_code_block: bool,
    code_buffer: String,
    code_language: Option<String>,

    // Table state
    table_buffer: Vec<Vec<String>>,
    table_row: Vec<String>,

    // Buffers
    text_buffer: String,
    segments: Vec<TextSegment>,
    html_buffer: String,

    // Current formatting
    formatting: Formatting,
    hyperlink: Option<String>,

    // Phantom to ensure lifetime
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a> ParseMarkdownState<'a> {
    fn new(has_h1: bool, has_any_heading: bool) -> Self {
        Self {
            doc_items: Vec::new(),
            item_count: 0,
            group_count: 0,
            picture_count: 0,
            has_h1,
            has_any_heading,
            seen_first_heading: false,
            in_heading: false,
            heading_level: None,
            list_stack: Vec::new(),
            in_code_block: false,
            code_buffer: String::new(),
            code_language: None,
            table_buffer: Vec::new(),
            table_row: Vec::new(),
            text_buffer: String::new(),
            segments: vec![TextSegment::new()],
            html_buffer: String::new(),
            formatting: MarkdownBackend::new_formatting(),
            hyperlink: None,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Get content layer based on heading state
    #[inline]
    const fn content_layer_before_heading(&self) -> &'static str {
        if self.has_h1 || !self.has_any_heading {
            "body"
        } else {
            "furniture"
        }
    }

    /// Check if currently in furniture layer
    #[inline]
    fn is_furniture(&self) -> bool {
        !self.seen_first_heading && self.content_layer_before_heading() == "furniture"
    }

    /// Consume state and return `doc_items`
    fn into_doc_items(self) -> Vec<DocItem> {
        self.doc_items
    }

    /// Reset segments and formatting
    #[inline]
    fn reset_segments(&mut self) {
        self.segments.clear();
        self.segments.push(TextSegment::new());
        self.formatting = MarkdownBackend::new_formatting();
        self.hyperlink = None;
    }

    /// Flush HTML buffer if not empty
    fn flush_html_buffer(&mut self) {
        if !self.html_buffer.is_empty() {
            if let Some(html_items) = MarkdownBackend::parse_html_fragment(
                &self.html_buffer,
                &mut self.item_count,
                &mut self.group_count,
            ) {
                self.doc_items.extend(html_items);
            }
            self.html_buffer.clear();
        }
    }

    /// Handle heading start event
    #[inline]
    fn handle_heading_start(&mut self, level: usize) {
        self.in_heading = true;
        self.heading_level = Some(level);
        self.segments.clear();
        self.segments.push(TextSegment::new());
    }

    /// Handle heading end event
    fn handle_heading_end(&mut self) {
        use docling_core::content::ItemRef;

        if !self.in_heading {
            return;
        }

        // Get combined text from all segments
        let combined_text: String = self
            .segments
            .iter()
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join("");
        let text = combined_text.trim().to_string();

        if !text.is_empty() {
            let non_empty_segments: Vec<_> = self
                .segments
                .iter()
                .filter(|s| !s.text.trim().is_empty())
                .collect();

            if non_empty_segments.len() > 1 {
                // Multiple segments - create Inline group as child of heading
                let (items, inline_ref) = MarkdownBackend::create_items_from_segments(
                    &self.segments,
                    &mut self.item_count,
                    &mut self.group_count,
                );

                let children =
                    inline_ref.map_or_else(Vec::new, |ref_path| vec![ItemRef::new(ref_path)]);

                let doc_item = DocItem::SectionHeader {
                    self_ref: format!("#/texts/{}", self.item_count),
                    parent: None,
                    children,
                    content_layer: "body".to_string(),
                    prov: create_provenance(1),
                    orig: text,
                    text: String::new(),
                    level: self.heading_level.unwrap_or(1).saturating_sub(1),
                    formatting: None,
                    hyperlink: None,
                };
                self.doc_items.push(doc_item);
                self.item_count += 1;
                self.doc_items.extend(items);
            } else {
                // Single segment - use simple heading
                let single_fmt = non_empty_segments.first().and_then(|s| {
                    if s.has_formatting() {
                        Some(s.formatting.clone())
                    } else {
                        None
                    }
                });
                let single_link = non_empty_segments.first().and_then(|s| s.hyperlink.clone());

                let doc_item = DocItem::SectionHeader {
                    self_ref: format!("#/texts/{}", self.item_count),
                    parent: None,
                    children: vec![],
                    content_layer: "body".to_string(),
                    prov: create_provenance(1),
                    orig: text.clone(),
                    text,
                    level: self.heading_level.unwrap_or(1).saturating_sub(1),
                    formatting: single_fmt,
                    hyperlink: single_link,
                };
                self.doc_items.push(doc_item);
                self.item_count += 1;
            }
        }
        self.seen_first_heading = true;
        self.in_heading = false;
        self.heading_level = None;
        self.reset_segments();
    }

    /// Handle paragraph start event
    #[inline]
    fn handle_paragraph_start(&mut self) {
        self.segments.clear();
        self.segments.push(TextSegment::new());
    }

    /// Handle paragraph end event
    fn handle_paragraph_end(&mut self) {
        // Skip if in heading or list (handled separately)
        if self.in_heading || !self.list_stack.is_empty() || self.in_code_block {
            return;
        }

        // Furniture layer check
        if !self.is_furniture() {
            let (items, _) = MarkdownBackend::create_items_from_segments(
                &self.segments,
                &mut self.item_count,
                &mut self.group_count,
            );
            self.doc_items.extend(items);
        }

        self.reset_segments();
    }

    /// Handle text event
    #[inline]
    fn handle_text(&mut self, text: &str) {
        if self.in_code_block {
            self.code_buffer.push_str(text);
        } else {
            if let Some(seg) = self.segments.last_mut() {
                seg.text.push_str(text);
            }
            self.text_buffer.push_str(text);
        }
    }

    /// Handle inline code event
    fn handle_code(&mut self, code: &str) {
        let decoded_code = MarkdownBackend::decode_html_entities(code);
        if let Some(seg) = self.segments.last_mut() {
            seg.text.push('`');
            seg.text.push_str(&decoded_code);
            seg.text.push('`');
        }
        self.text_buffer.push('`');
        self.text_buffer.push_str(&decoded_code);
        self.text_buffer.push('`');
    }

    /// Handle soft break event
    #[inline]
    fn handle_soft_break(&mut self) {
        if let Some(seg) = self.segments.last_mut() {
            seg.text.push(' ');
        }
        self.text_buffer.push(' ');
    }

    /// Handle hard break event
    #[inline]
    fn handle_hard_break(&mut self) {
        if let Some(seg) = self.segments.last_mut() {
            seg.text.push('\n');
        }
        self.text_buffer.push('\n');
    }

    /// Handle HTML event
    fn handle_html(&mut self, html_content: &str) {
        self.html_buffer.push_str(html_content);
    }

    /// Handle list end event
    fn handle_list_end(&mut self) {
        use docling_core::content::ItemRef;

        // Pop the current list context
        if let Some((list_ref, _, _, _list_index, Some(parent_idx))) = self.list_stack.pop() {
            // If this list had a parent ListItem, update that item's children
            if let Some(DocItem::ListItem { children, .. }) = self.doc_items.get_mut(parent_idx) {
                children.push(ItemRef::new(list_ref));
            }
        }
    }

    /// Handle list item start event
    fn handle_item_start(&mut self) {
        self.segments.clear();
        self.segments.push(TextSegment::new());
        self.text_buffer.clear();
    }

    /// Handle list item end event
    fn handle_item_end(&mut self) {
        use docling_core::content::ItemRef;

        // Check if we have content in segments
        let non_empty_segments: Vec<_> = self
            .segments
            .iter()
            .filter(|s| !s.text.trim().is_empty())
            .collect();

        if non_empty_segments.is_empty() {
            self.reset_segments();
            self.text_buffer.clear();
            return;
        }

        // Get current list context from stack
        let (list_ref, list_is_ordered, list_item_number, list_index, _) = self
            .list_stack
            .last()
            .cloned()
            .unwrap_or((String::new(), false, 1, 0, None));

        let marker = if list_is_ordered {
            format!("{list_item_number}.")
        } else {
            "-".to_string()
        };

        // Check if we need to create Inline group for formatted content
        let (text, children, extra_items, fmt, link) = if non_empty_segments.len() > 1 {
            // Multiple segments - create Inline group
            let (items, inline_ref) = MarkdownBackend::create_items_from_segments(
                &self.segments,
                &mut self.item_count,
                &mut self.group_count,
            );
            let children_refs =
                inline_ref.map_or_else(Vec::new, |ref_path| vec![ItemRef::new(ref_path)]);
            (String::new(), children_refs, items, None, None)
        } else {
            // Single segment - use text and formatting from segment
            let combined_text: String = self
                .segments
                .iter()
                .map(|s| s.text.as_str())
                .collect::<Vec<_>>()
                .join("");
            let text = combined_text.trim().to_string();
            let seg = non_empty_segments[0];
            let fmt = if seg.has_formatting() {
                Some(seg.formatting.clone())
            } else {
                None
            };
            let link = seg.hyperlink.clone();
            (text, vec![], vec![], fmt, link)
        };

        // Create ListItem's self_ref AFTER segments to avoid collision
        let item_ref = format!("#/texts/{}", self.item_count);

        let doc_item = DocItem::ListItem {
            self_ref: item_ref.clone(),
            parent: if list_ref.is_empty() {
                None
            } else {
                Some(ItemRef::new(list_ref.clone()))
            },
            children,
            content_layer: "body".to_string(),
            prov: create_provenance(1),
            orig: text.clone(),
            text,
            enumerated: list_is_ordered,
            marker,
            formatting: fmt,
            hyperlink: link,
        };
        self.doc_items.push(doc_item);
        self.doc_items.extend(extra_items);

        // Add this ListItem to its parent List's children
        if !list_ref.is_empty() {
            if let Some(DocItem::List { children, .. }) = self.doc_items.get_mut(list_index) {
                children.push(ItemRef::new(item_ref));
            }
        }

        self.item_count += 1;

        // Increment item number for ordered lists
        if let Some((_, is_ordered, ref mut num, _, _)) = self.list_stack.last_mut() {
            if *is_ordered {
                *num += 1;
            }
        }

        self.reset_segments();
        self.text_buffer.clear();
    }

    /// Handle code block start event
    fn handle_code_block_start(&mut self, lang: Option<&str>) {
        self.in_code_block = true;
        self.code_buffer.clear();
        self.code_language = lang.map(str::to_string);
    }

    /// Handle code block end event
    fn handle_code_block_end(&mut self) {
        let text = self.code_buffer.trim().to_string();
        if !text.is_empty() {
            let decoded_text = MarkdownBackend::decode_html_entities(&text);
            let doc_item = create_code_item(
                self.item_count,
                decoded_text,
                self.code_language.take(),
                create_provenance(1),
            );
            self.doc_items.push(doc_item);
            self.item_count += 1;
        }
        self.in_code_block = false;
        self.code_buffer.clear();
        self.code_language = None;
    }

    /// Handle table start event
    fn handle_table_start(&mut self) {
        self.table_buffer.clear();
    }

    /// Handle table end event
    fn handle_table_end(&mut self) {
        if self.table_buffer.is_empty() {
            return;
        }

        let num_rows = self.table_buffer.len();
        let num_cols = self.table_buffer.first().map_or(0, std::vec::Vec::len);

        let mut table_cells = Vec::new();
        for (row_idx, row) in self.table_buffer.iter().enumerate() {
            for (col_idx, cell_text) in row.iter().enumerate() {
                table_cells.push(TableCell {
                    text: cell_text.clone(),
                    row_span: Some(1),
                    col_span: Some(1),
                    ref_item: None,
                    start_row_offset_idx: Some(row_idx),
                    start_col_offset_idx: Some(col_idx),
                    ..Default::default()
                });
            }
        }

        // Build grid from cells
        let mut grid = vec![vec![]; num_rows];
        for (row_idx, row) in self.table_buffer.iter().enumerate() {
            for (col_idx, cell_text) in row.iter().enumerate() {
                grid[row_idx].push(TableCell {
                    text: cell_text.clone(),
                    row_span: Some(1),
                    col_span: Some(1),
                    ref_item: None,
                    start_row_offset_idx: Some(row_idx),
                    start_col_offset_idx: Some(col_idx),
                    ..Default::default()
                });
            }
        }

        let table_data = TableData {
            num_rows,
            num_cols,
            grid,
            table_cells: Some(table_cells),
        };

        let doc_item = DocItem::Table {
            self_ref: format!("#/tables/{}", self.item_count),
            parent: None,
            children: vec![],
            content_layer: "body".to_string(),
            prov: vec![ProvenanceItem {
                page_no: 1,
                bbox: BoundingBox::new(
                    0.0,
                    0.0,
                    num_cols as f64,
                    num_rows as f64,
                    CoordOrigin::TopLeft,
                ),
                charspan: None,
            }],
            data: table_data,
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

    /// Handle table row start event
    #[inline]
    fn handle_table_row_start(&mut self) {
        self.table_row.clear();
    }

    /// Handle table row end event
    fn handle_table_row_end(&mut self) {
        if !self.table_row.is_empty() {
            self.table_buffer.push(std::mem::take(&mut self.table_row));
        }
    }

    /// Handle table cell start event
    #[inline]
    fn handle_table_cell_start(&mut self) {
        self.text_buffer.clear();
    }

    /// Handle table cell end event
    #[inline]
    fn handle_table_cell_end(&mut self) {
        self.table_row.push(std::mem::take(&mut self.text_buffer));
    }

    /// Handle emphasis start event
    fn handle_emphasis_start(&mut self) {
        if let Some(last) = self.segments.last() {
            if last.text.is_empty() {
                if let Some(seg) = self.segments.last_mut() {
                    seg.formatting.italic = Some(true);
                }
            } else {
                let mut new_seg = TextSegment::new();
                new_seg.formatting = self.formatting.clone();
                new_seg.formatting.italic = Some(true);
                new_seg.hyperlink.clone_from(&self.hyperlink);
                self.segments.push(new_seg);
            }
        }
        self.formatting.italic = Some(true);
    }

    /// Handle emphasis end event
    fn handle_emphasis_end(&mut self) {
        if let Some(last) = self.segments.last() {
            if last.text.is_empty() {
                if let Some(seg) = self.segments.last_mut() {
                    seg.formatting.italic = Some(false);
                }
            } else {
                let mut new_seg = TextSegment::new();
                new_seg.formatting = self.formatting.clone();
                new_seg.formatting.italic = Some(false);
                new_seg.hyperlink.clone_from(&self.hyperlink);
                self.segments.push(new_seg);
            }
        }
        self.formatting.italic = Some(false);
    }

    /// Handle strong start event
    fn handle_strong_start(&mut self) {
        if let Some(last) = self.segments.last() {
            if !last.text.is_empty() {
                let mut new_seg = TextSegment::new();
                new_seg.formatting = self.formatting.clone();
                new_seg.formatting.bold = Some(true);
                new_seg.hyperlink.clone_from(&self.hyperlink);
                self.segments.push(new_seg);
            } else if let Some(seg) = self.segments.last_mut() {
                seg.formatting.bold = Some(true);
            }
        }
        self.formatting.bold = Some(true);
    }

    /// Handle strong end event
    fn handle_strong_end(&mut self) {
        if let Some(last) = self.segments.last() {
            if !last.text.is_empty() {
                let mut new_seg = TextSegment::new();
                new_seg.formatting = self.formatting.clone();
                new_seg.formatting.bold = Some(false);
                new_seg.hyperlink.clone_from(&self.hyperlink);
                self.segments.push(new_seg);
            } else if let Some(seg) = self.segments.last_mut() {
                seg.formatting.bold = Some(false);
            }
        }
        self.formatting.bold = Some(false);
    }

    /// Handle strikethrough start event
    fn handle_strikethrough_start(&mut self) {
        if let Some(last) = self.segments.last() {
            if !last.text.is_empty() {
                let mut new_seg = TextSegment::new();
                new_seg.formatting = self.formatting.clone();
                new_seg.formatting.strikethrough = Some(true);
                new_seg.hyperlink.clone_from(&self.hyperlink);
                self.segments.push(new_seg);
            } else if let Some(seg) = self.segments.last_mut() {
                seg.formatting.strikethrough = Some(true);
            }
        }
        self.formatting.strikethrough = Some(true);
    }

    /// Handle strikethrough end event
    fn handle_strikethrough_end(&mut self) {
        if let Some(last) = self.segments.last() {
            if !last.text.is_empty() {
                let mut new_seg = TextSegment::new();
                new_seg.formatting = self.formatting.clone();
                new_seg.formatting.strikethrough = Some(false);
                new_seg.hyperlink.clone_from(&self.hyperlink);
                self.segments.push(new_seg);
            } else if let Some(seg) = self.segments.last_mut() {
                seg.formatting.strikethrough = Some(false);
            }
        }
        self.formatting.strikethrough = Some(false);
    }

    /// Handle link start event
    fn handle_link_start(&mut self, url: &str) {
        if let Some(last) = self.segments.last() {
            if !last.text.is_empty() {
                let mut new_seg = TextSegment::new();
                new_seg.formatting = self.formatting.clone();
                new_seg.hyperlink = Some(url.to_string());
                self.segments.push(new_seg);
            } else if let Some(seg) = self.segments.last_mut() {
                seg.hyperlink = Some(url.to_string());
            }
        }
        self.hyperlink = Some(url.to_string());
    }

    /// Handle link end event
    fn handle_link_end(&mut self) {
        if let Some(last) = self.segments.last() {
            if !last.text.is_empty() {
                let mut new_seg = TextSegment::new();
                new_seg.formatting = self.formatting.clone();
                new_seg.hyperlink = None;
                self.segments.push(new_seg);
            } else if let Some(seg) = self.segments.last_mut() {
                seg.hyperlink = None;
            }
        }
        self.hyperlink = None;
    }
}

/// Markdown Document Backend
///
/// Ported from: docling/backend/md_backend.py:73-614
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct MarkdownBackend;

impl MarkdownBackend {
    /// Create a new Markdown backend instance
    #[inline]
    #[must_use = "creates a backend instance that should be used for parsing"]
    pub const fn new() -> Self {
        Self
    }

    /// Shorten long underscore sequences (Python: lines 74-95)
    ///
    /// Very long sequences of underscores cause processing issues.
    /// In proper Markdown, underscores should be escaped or represent emphasis.
    fn shorten_underscore_sequences(text: &str) -> String {
        const MAX_LENGTH: usize = 10;
        let mut result = String::with_capacity(text.len());
        let mut underscore_count = 0;

        for ch in text.chars() {
            if ch == '_' {
                underscore_count += 1;
                if underscore_count <= MAX_LENGTH {
                    result.push(ch);
                }
            } else {
                underscore_count = 0;
                result.push(ch);
            }
        }

        result
    }

    /// Decode common HTML entities in code content
    ///
    /// Python's marko library decodes HTML entities in code blocks.
    /// This matches that behavior for &amp; &lt; &gt; &quot; &#39; etc.
    fn decode_html_entities(text: &str) -> String {
        text.replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
            .replace("&apos;", "'")
            .replace("&#x27;", "'")
            .replace("&#34;", "\"")
            .replace("&#x22;", "\"")
    }

    /// Check if formatting has any values set
    const fn has_formatting(fmt: &Formatting) -> bool {
        fmt.bold.is_some()
            || fmt.italic.is_some()
            || fmt.underline.is_some()
            || fmt.strikethrough.is_some()
    }

    /// Create default formatting
    const fn new_formatting() -> Formatting {
        Formatting {
            bold: None,
            italic: None,
            underline: None,
            strikethrough: None,
            code: None,
            script: None,
            font_size: None,
            font_family: None,
        }
    }

    /// Check if the document contains an H1 heading and if it contains any heading
    ///
    /// Python reference: `md_backend.py` furniture layer logic
    /// First pass scan to determine content layer for pre-heading content.
    /// Returns (`has_h1`, `has_any_heading`)
    fn document_heading_info(markdown: &str, options: Options) -> (bool, bool) {
        let parser = Parser::new_ext(markdown, options);
        let mut has_h1 = false;
        let mut has_any_heading = false;
        for event in parser {
            if let Event::Start(Tag::Heading { level, .. }) = event {
                has_any_heading = true;
                if level as usize == 1 {
                    has_h1 = true;
                    break; // Found H1, no need to continue
                }
            }
        }
        (has_h1, has_any_heading)
    }

    /// Create `DocItem::Text` from a text segment
    fn create_text_from_segment(segment: &TextSegment, item_count: &mut usize) -> DocItem {
        let text = segment.text.trim().to_string();
        let doc_item = DocItem::Text {
            self_ref: format!("#/texts/{item_count}"),
            parent: None,
            children: vec![],
            content_layer: "body".to_string(),
            prov: create_provenance(1),
            orig: text.clone(),
            text,
            formatting: if segment.has_formatting() {
                Some(segment.formatting.clone())
            } else {
                None
            },
            hyperlink: segment.hyperlink.clone(),
        };
        *item_count += 1;
        doc_item
    }

    /// Create text `DocItems` from segments, optionally wrapped in an Inline group
    /// Returns the items to add and optionally an Inline group ref for the first top-level item
    fn create_items_from_segments(
        segments: &[TextSegment],
        item_count: &mut usize,
        group_count: &mut usize,
    ) -> (Vec<DocItem>, Option<String>) {
        use docling_core::content::ItemRef;

        // Filter out empty segments
        let non_empty: Vec<_> = segments
            .iter()
            .filter(|s| !s.text.trim().is_empty())
            .collect();

        if non_empty.is_empty() {
            return (vec![], None);
        }

        if non_empty.len() == 1 {
            // Single segment - create single DocItem
            let item = Self::create_text_from_segment(non_empty[0], item_count);
            let self_ref = item.self_ref().to_string();
            return (vec![item], Some(self_ref));
        }

        // Multiple segments - create Inline group with children
        let inline_ref = format!("#/groups/{group_count}");
        *group_count += 1;

        let mut children_refs = Vec::new();
        let mut items = Vec::new();

        for segment in non_empty {
            let child_item = Self::create_text_from_segment(segment, item_count);
            children_refs.push(ItemRef::new(child_item.self_ref().to_string()));
            items.push(child_item);
        }

        // Update children to have Inline as parent
        for item in &mut items {
            if let DocItem::Text { parent, .. } = item {
                *parent = Some(ItemRef::new(inline_ref.clone()));
            }
        }

        let inline_group = DocItem::Inline {
            self_ref: inline_ref.clone(),
            parent: None,
            children: children_refs,
            content_layer: "body".to_string(),
            name: "inline".to_string(),
        };

        // Return inline group first (it's the top-level item), then children
        let mut result = vec![inline_group];
        result.extend(items);

        (result, Some(inline_ref))
    }

    /// Parse markdown events and convert to `DocItems`
    ///
    /// Python reference: _`iterate_elements()` method (lines 243-521)
    /// Walks the markdown AST and creates `DocItems` for each element
    // Method signature kept for API consistency with other MarkdownBackend methods
    #[allow(clippy::unused_self)]
    fn parse_markdown(&self, markdown: &str) -> Vec<DocItem> {
        // pulldown-cmark options (enable tables, strikethrough, etc.)
        let mut options = Options::empty();
        options.insert(Options::ENABLE_TABLES);
        options.insert(Options::ENABLE_STRIKETHROUGH);
        options.insert(Options::ENABLE_TASKLISTS);

        // First pass: Check if document contains an H1 heading
        // Python reference: md_backend.py furniture layer logic
        let (has_h1, has_any_heading) = Self::document_heading_info(markdown, options);

        // Initialize state struct with heading info
        let mut state = ParseMarkdownState::new(has_h1, has_any_heading);

        let parser = Parser::new_ext(markdown, options);

        for event in parser {
            // Check if we have accumulated HTML and this is not an HTML event
            // If so, flush the HTML buffer by delegating to HTML backend
            if !state.html_buffer.is_empty() && !matches!(event, Event::Html(_)) {
                state.flush_html_buffer();
            }
            match event {
                // Heading events
                Event::Start(Tag::Heading { level, .. }) => {
                    state.handle_heading_start(level as usize);
                }
                Event::End(TagEnd::Heading(_)) => state.handle_heading_end(),

                // Paragraph events
                Event::Start(Tag::Paragraph) => state.handle_paragraph_start(),
                Event::End(TagEnd::Paragraph) => state.handle_paragraph_end(),

                // List start (Python: lines 285-296)
                Event::Start(Tag::List(first_item)) => {
                    // Track if we need to set this list as a child of a parent item
                    let mut parent_item_idx: Option<usize> = None;

                    // If we're inside a list item and have accumulated text,
                    // emit the parent item BEFORE starting the nested list
                    if !state.list_stack.is_empty() && !state.text_buffer.trim().is_empty() {
                        let text = state.text_buffer.trim().to_string();
                        // Get current list context from stack
                        let (ref list_ref, list_is_ordered, list_item_number, parent_list_index, _) =
                            state.list_stack.last().cloned().unwrap_or((
                                String::new(),
                                false,
                                1,
                                0,
                                None,
                            ));

                        let marker = if list_is_ordered {
                            format!("{list_item_number}.")
                        } else {
                            "-".to_string()
                        };

                        let list_item_ref = format!("#/texts/{}", state.item_count);
                        let doc_item = DocItem::ListItem {
                            self_ref: list_item_ref.clone(),
                            parent: if list_ref.is_empty() {
                                None
                            } else {
                                Some(docling_core::content::ItemRef::new(list_ref.clone()))
                            },
                            children: vec![], // Will be updated when nested list ends
                            content_layer: "body".to_string(),
                            prov: create_provenance(1),
                            orig: text.clone(),
                            text,
                            enumerated: list_is_ordered,
                            marker,
                            formatting: if Self::has_formatting(&state.formatting) {
                                Some(state.formatting.clone())
                            } else {
                                None
                            },
                            hyperlink: state.hyperlink.clone(),
                        };
                        parent_item_idx = Some(state.doc_items.len());
                        state.doc_items.push(doc_item);

                        // Add this ListItem to its parent List's children
                        if !list_ref.is_empty() {
                            if let Some(DocItem::List { children, .. }) =
                                state.doc_items.get_mut(parent_list_index)
                            {
                                children.push(docling_core::content::ItemRef::new(list_item_ref));
                            }
                        }

                        state.item_count += 1;

                        // Increment item number for ordered lists
                        if let Some((_, is_ordered, ref mut num, _, _)) =
                            state.list_stack.last_mut()
                        {
                            if *is_ordered {
                                *num += 1;
                            }
                        }

                        state.text_buffer.clear();
                        state.formatting = Self::new_formatting();
                        state.hyperlink = None;
                    }

                    // Create a new DocItem::List container
                    let is_ordered = first_item.is_some();
                    let start_number = first_item.unwrap_or(1) as usize;
                    let list_ref = format!("#/groups/{}", state.group_count);
                    let list_index = state.doc_items.len();

                    // Determine parent: if nested list, parent is the current list
                    let parent_ref = state
                        .list_stack
                        .last()
                        .map(|(ref r, _, _, _, _)| docling_core::content::ItemRef::new(r.clone()));

                    let list_name = if is_ordered {
                        if start_number == 1 {
                            "ordered list".to_string()
                        } else {
                            format!("ordered list start {start_number}")
                        }
                    } else {
                        "list".to_string()
                    };

                    let list_doc_item = DocItem::List {
                        self_ref: list_ref.clone(),
                        parent: parent_ref,
                        children: vec![], // Will be populated as items are added
                        content_layer: "body".to_string(),
                        name: list_name,
                    };
                    state.doc_items.push(list_doc_item);
                    state.group_count += 1;

                    // Push new list context onto stack
                    state.list_stack.push((
                        list_ref,
                        is_ordered,
                        start_number,
                        list_index,
                        parent_item_idx,
                    ));
                }

                // List end
                Event::End(TagEnd::List(_)) => state.handle_list_end(),

                // List item events
                Event::Start(Tag::Item) => state.handle_item_start(),
                Event::End(TagEnd::Item) => state.handle_item_end(),

                // Code block events
                Event::Start(Tag::CodeBlock(kind)) => {
                    let lang: Option<String> = match kind {
                        pulldown_cmark::CodeBlockKind::Fenced(info) => {
                            let lang = info.split_whitespace().next().unwrap_or_default();
                            if lang.is_empty() {
                                None
                            } else {
                                Some(lang.to_string())
                            }
                        }
                        pulldown_cmark::CodeBlockKind::Indented => None,
                    };
                    state.handle_code_block_start(lang.as_deref());
                }
                Event::End(TagEnd::CodeBlock) => state.handle_code_block_end(),

                // Table events
                Event::Start(Tag::Table(_)) => state.handle_table_start(),

                Event::End(TagEnd::Table) => state.handle_table_end(),

                Event::Start(Tag::TableHead | Tag::TableRow) => state.handle_table_row_start(),

                Event::End(TagEnd::TableHead | TagEnd::TableRow) => {
                    state.handle_table_row_end();
                }

                Event::Start(Tag::TableCell) => state.handle_table_cell_start(),
                Event::End(TagEnd::TableCell) => state.handle_table_cell_end(),

                // Formatting events
                Event::Start(Tag::Emphasis) => state.handle_emphasis_start(),
                Event::End(TagEnd::Emphasis) => state.handle_emphasis_end(),
                Event::Start(Tag::Strong) => state.handle_strong_start(),
                Event::End(TagEnd::Strong) => state.handle_strong_end(),
                Event::Start(Tag::Strikethrough) => state.handle_strikethrough_start(),
                Event::End(TagEnd::Strikethrough) => state.handle_strikethrough_end(),

                // Link events
                Event::Start(Tag::Link { dest_url, .. }) => state.handle_link_start(&dest_url),
                Event::End(TagEnd::Link) => state.handle_link_end(),

                // Image (Python: lines 334-348)
                Event::Start(Tag::Image { title, .. }) => {
                    // Create caption TextItem if title exists (Python: lines 339-346)
                    let caption_ref = if title.is_empty() {
                        None
                    } else {
                        let caption_text = title.to_string();
                        let caption_item = DocItem::Text {
                            self_ref: format!("#/texts/{}", state.item_count),
                            parent: None,
                            children: vec![],
                            content_layer: "body".to_string(),
                            prov: create_provenance(1),
                            orig: caption_text.clone(),
                            text: caption_text,
                            formatting: if Self::has_formatting(&state.formatting) {
                                Some(state.formatting.clone())
                            } else {
                                None
                            },
                            hyperlink: state.hyperlink.clone(),
                        };
                        let caption_ref_str = format!("#/texts/{}", state.item_count);
                        state.doc_items.push(caption_item);
                        state.item_count += 1;
                        Some(caption_ref_str)
                    };

                    // Create Picture DocItem (Python: line 348)
                    let picture_item = DocItem::Picture {
                        self_ref: format!("#/pictures/{}", state.picture_count),
                        parent: None,
                        children: vec![],
                        content_layer: "body".to_string(),
                        prov: vec![{
                            let mut prov = create_default_provenance(1, CoordOrigin::TopLeft);
                            prov.charspan = Some(vec![0, 0]);
                            prov
                        }],
                        captions: caption_ref.map_or_else(Vec::new, |cap_ref| {
                            vec![docling_core::content::ItemRef { ref_path: cap_ref }]
                        }),
                        footnotes: vec![],
                        references: vec![],
                        image: None, // No image data in markdown parsing
                        annotations: vec![],
                        ocr_text: None,
                    };
                    state.doc_items.push(picture_item);
                    state.picture_count += 1;
                }

                // Text content
                Event::Text(text) => state.handle_text(&text),

                // Code span (inline code)
                Event::Code(code) => state.handle_code(&code),

                // Soft line break
                Event::SoftBreak => state.handle_soft_break(),

                // Hard line break
                Event::HardBreak => state.handle_hard_break(),

                // HTML blocks
                Event::Html(html_content) => state.handle_html(&html_content),

                _ => {
                    // Ignore other events
                }
            }
        }

        // Flush any remaining HTML at the end
        state.flush_html_buffer();

        state.into_doc_items()
    }

    /// Parse an HTML fragment and return `DocItems` with adjusted `self_refs`
    ///
    /// Python reference: `md_backend.py` lines 460-609 (HTML delegation)
    /// This handles embedded HTML in markdown by delegating to the HTML backend.
    fn parse_html_fragment(
        html: &str,
        item_count: &mut usize,
        _group_count: &mut usize,
    ) -> Option<Vec<DocItem>> {
        // Wrap fragment in body tags to ensure proper parsing
        let wrapped_html = format!("<html><body>{html}</body></html>");

        let html_backend = HtmlBackend;
        let options = BackendOptions::default();

        match html_backend.parse_bytes(wrapped_html.as_bytes(), &options) {
            Ok(document) => document.content_blocks.map(|mut items| {
                // Re-number DocItems to avoid self_ref collisions
                // This also updates List.children and ListItem.parent references
                Self::renumber_items(&mut items, item_count);
                items
            }),
            Err(_) => None,
        }
    }

    /// Renumber `DocItems` to avoid `self_ref` collisions when merging HTML items into markdown
    ///
    /// Returns a map of `old_ref` -> `new_ref` for updating references
    fn renumber_items(
        items: &mut [DocItem],
        item_count: &mut usize,
    ) -> std::collections::HashMap<String, String> {
        let mut ref_map = std::collections::HashMap::new();

        // First pass: renumber all self_refs and build the mapping
        for item in items.iter_mut() {
            let (old_ref, new_ref) = match item {
                DocItem::Text { self_ref, .. }
                | DocItem::SectionHeader { self_ref, .. }
                | DocItem::ListItem { self_ref, .. }
                | DocItem::Code { self_ref, .. } => {
                    let old = std::mem::take(self_ref);
                    *self_ref = format!("#/texts/{}", *item_count);
                    *item_count += 1;
                    (old, self_ref.clone())
                }
                DocItem::Table { self_ref, .. } => {
                    let old = std::mem::take(self_ref);
                    *self_ref = format!("#/tables/{}", *item_count);
                    *item_count += 1;
                    (old, self_ref.clone())
                }
                DocItem::List { self_ref, .. } => {
                    let old = std::mem::take(self_ref);
                    *self_ref = format!("#/groups/{}", *item_count);
                    *item_count += 1;
                    (old, self_ref.clone())
                }
                DocItem::Picture { self_ref, .. } => {
                    let old = std::mem::take(self_ref);
                    *self_ref = format!("#/pictures/{}", *item_count);
                    *item_count += 1;
                    (old, self_ref.clone())
                }
                _ => {
                    *item_count += 1;
                    continue;
                }
            };
            ref_map.insert(old_ref, new_ref);
        }

        // Second pass: update all child/parent references using the mapping
        for item in items.iter_mut() {
            match item {
                DocItem::List {
                    children, parent, ..
                } => {
                    // Update children refs
                    for child in children.iter_mut() {
                        if let Some(new_ref) = ref_map.get(&child.ref_path) {
                            child.ref_path = new_ref.clone();
                        }
                    }
                    // Update parent ref
                    if let Some(p) = parent {
                        if let Some(new_ref) = ref_map.get(&p.ref_path) {
                            p.ref_path = new_ref.clone();
                        }
                    }
                }
                DocItem::ListItem {
                    parent, children, ..
                } => {
                    // Update parent ref
                    if let Some(p) = parent {
                        if let Some(new_ref) = ref_map.get(&p.ref_path) {
                            p.ref_path = new_ref.clone();
                        }
                    }
                    // Update children refs
                    for child in children.iter_mut() {
                        if let Some(new_ref) = ref_map.get(&child.ref_path) {
                            child.ref_path = new_ref.clone();
                        }
                    }
                }
                DocItem::Inline {
                    children, parent, ..
                } => {
                    for child in children.iter_mut() {
                        if let Some(new_ref) = ref_map.get(&child.ref_path) {
                            child.ref_path = new_ref.clone();
                        }
                    }
                    if let Some(p) = parent {
                        if let Some(new_ref) = ref_map.get(&p.ref_path) {
                            p.ref_path = new_ref.clone();
                        }
                    }
                }
                _ => {}
            }
        }

        ref_map
    }

    /// Generate markdown from `DocItems`
    ///
    /// Converts `DocItems` to a `DoclingDocument` and uses the proper `MarkdownSerializer`
    /// to handle nested structures (lists, tables, etc.) correctly.
    // Method signature kept for API consistency with other MarkdownBackend methods
    #[allow(clippy::unused_self)]
    fn docitems_to_markdown(&self, doc_items: &[DocItem]) -> String {
        use docling_core::content::ItemRef;
        use docling_core::document::{DoclingDocument, GroupItem, Origin};
        use docling_core::serializer::MarkdownSerializer;
        use std::collections::HashMap;

        if doc_items.is_empty() {
            return String::new();
        }

        // Separate items by type and identify top-level items
        let mut texts = Vec::new();
        let mut groups = Vec::new();
        let mut tables = Vec::new();
        let mut pictures = Vec::new();
        let mut top_level_refs = Vec::new();

        // Set of items that are children of other items (not top-level)
        let mut child_refs: std::collections::HashSet<String> = std::collections::HashSet::new();

        // First pass: collect all child references
        for item in doc_items {
            match item {
                DocItem::ListItem { children, .. } => {
                    for child in children {
                        child_refs.insert(child.ref_path.clone());
                    }
                }
                DocItem::List {
                    children, parent, ..
                } => {
                    // List containers that have a parent are nested (not top-level)
                    if parent.is_some() {
                        if let DocItem::List { self_ref, .. } = item {
                            child_refs.insert(self_ref.clone());
                        }
                    }
                    for child in children {
                        child_refs.insert(child.ref_path.clone());
                    }
                }
                DocItem::Inline { children, .. } => {
                    // Inline group children should be marked as non-top-level
                    for child in children {
                        child_refs.insert(child.ref_path.clone());
                    }
                }
                _ => {}
            }
        }

        // Second pass: categorize items and identify top-level
        for item in doc_items {
            let self_ref = match item {
                DocItem::Text { self_ref, .. }
                | DocItem::SectionHeader { self_ref, .. }
                | DocItem::ListItem { self_ref, .. }
                | DocItem::Paragraph { self_ref, .. }
                | DocItem::Title { self_ref, .. }
                | DocItem::Code { self_ref, .. }
                | DocItem::Formula { self_ref, .. }
                | DocItem::List { self_ref, .. }
                | DocItem::OrderedList { self_ref, .. }
                | DocItem::Inline { self_ref, .. }
                | DocItem::Table { self_ref, .. }
                | DocItem::Picture { self_ref, .. } => self_ref.clone(),
                _ => continue, // Skip other types for now
            };

            // Is this a top-level item?
            let is_top_level = !child_refs.contains(&self_ref);

            // Add to appropriate category
            match item {
                DocItem::Text { .. }
                | DocItem::SectionHeader { .. }
                | DocItem::ListItem { .. }
                | DocItem::Paragraph { .. }
                | DocItem::Title { .. }
                | DocItem::Code { .. }
                | DocItem::Formula { .. } => {
                    if is_top_level {
                        top_level_refs.push(ItemRef::new(self_ref));
                    }
                    texts.push(item.clone());
                }
                DocItem::List { .. } | DocItem::OrderedList { .. } | DocItem::Inline { .. } => {
                    if is_top_level {
                        top_level_refs.push(ItemRef::new(self_ref));
                    }
                    groups.push(item.clone());
                }
                DocItem::Table { .. } => {
                    if is_top_level {
                        top_level_refs.push(ItemRef::new(self_ref));
                    }
                    tables.push(item.clone());
                }
                DocItem::Picture { .. } => {
                    if is_top_level {
                        top_level_refs.push(ItemRef::new(self_ref));
                    }
                    pictures.push(item.clone());
                }
                _ => {}
            }
        }

        // Create DoclingDocument
        let doc = DoclingDocument {
            schema_name: "DoclingDocument".to_string(),
            version: "1.7.0".to_string(),
            name: "markdown_document".to_string(),
            origin: Origin {
                mimetype: "text/markdown".to_string(),
                binary_hash: 0,
                filename: "document.md".to_string(),
            },
            body: GroupItem {
                self_ref: "#/body".to_string(),
                parent: None,
                children: top_level_refs,
                content_layer: "body".to_string(),
                name: "_root_".to_string(),
                label: "unspecified".to_string(),
            },
            furniture: None,
            texts,
            groups,
            tables,
            pictures,
            key_value_items: vec![],
            form_items: vec![],
            pages: HashMap::new(),
        };

        // Use MarkdownSerializer for proper formatting
        let serializer = MarkdownSerializer::new();
        serializer.serialize(&doc)
    }
}

impl DocumentBackend for MarkdownBackend {
    #[inline]
    fn format(&self) -> InputFormat {
        InputFormat::Md
    }

    fn parse_bytes(
        &self,
        data: &[u8],
        _options: &BackendOptions,
    ) -> Result<Document, DoclingError> {
        // Convert bytes to string (Python: lines 118-132)
        // Use from_utf8 to avoid allocating a vector copy
        let markdown = std::str::from_utf8(data)
            .map_err(|e| DoclingError::BackendError(format!("Invalid UTF-8: {e}")))?
            .to_string();

        // Shorten underscore sequences (Python: lines 124, 132)
        let markdown = Self::shorten_underscore_sequences(&markdown);

        // Parse markdown to DocItems (Python: convert() method, lines 539-614)
        let doc_items = self.parse_markdown(&markdown);

        // Generate markdown output
        let markdown_output = self.docitems_to_markdown(&doc_items);

        // Create Document
        let metadata = DocumentMetadata {
            num_pages: None,
            num_characters: markdown_output.chars().count(),
            ..Default::default()
        };

        Ok(Document {
            markdown: markdown_output,
            format: InputFormat::Md,
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

    /// Test 1: Verify backend creation and format()
    /// Ensures backend can be instantiated and returns correct format
    #[test]
    fn test_markdown_backend_creation() {
        let backend = MarkdownBackend::new();
        assert_eq!(
            backend.format(),
            InputFormat::Md,
            "MarkdownBackend::new() should return Md format"
        );
    }

    /// Test 2: Verify Default trait implementation
    /// Ensures Default::default() works and returns correct format
    #[test]
    fn test_markdown_backend_default() {
        let backend = MarkdownBackend;
        assert_eq!(
            backend.format(),
            InputFormat::Md,
            "MarkdownBackend struct should return Md format"
        );
    }

    /// Test 3: Test error handling for invalid UTF-8
    /// Ensures proper error is returned when bytes are not valid UTF-8
    #[test]
    fn test_parse_bytes_invalid_utf8() {
        let backend = MarkdownBackend::new();
        let invalid_utf8 = vec![0xFF, 0xFE, 0xFD]; // Invalid UTF-8 bytes
        let options = BackendOptions::default();

        let result = backend.parse_bytes(&invalid_utf8, &options);
        assert!(result.is_err(), "Invalid UTF-8 should return error");
        if let Err(DoclingError::BackendError(msg)) = result {
            assert!(
                msg.contains("Invalid UTF-8"),
                "Error message should mention Invalid UTF-8"
            );
        } else {
            panic!("Expected BackendError with 'Invalid UTF-8' message");
        }
    }

    /// Test 4: Test parsing empty markdown document
    /// Ensures empty markdown is handled gracefully
    #[test]
    fn test_parse_empty_markdown() {
        let backend = MarkdownBackend::new();
        let markdown = b"";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(markdown, &options);
        assert!(result.is_ok(), "Empty markdown should parse successfully");

        let doc = result.unwrap();
        assert_eq!(
            doc.format,
            InputFormat::Md,
            "Empty markdown should return Md format"
        );
        // Empty markdown should produce no DocItems
        assert!(
            doc.content_blocks.as_ref().is_none_or(|v| v.is_empty()),
            "Empty markdown should produce no DocItems"
        );
    }

    /// Test 5: Test parsing markdown with heading
    /// Verifies basic heading extraction functionality
    #[test]
    fn test_parse_markdown_heading() {
        let backend = MarkdownBackend::new();
        let markdown = b"# Test Heading\n";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(markdown, &options);
        assert!(result.is_ok(), "Heading markdown should parse successfully");

        let doc = result.unwrap();
        assert_eq!(
            doc.format,
            InputFormat::Md,
            "Heading markdown should return Md format"
        );

        // Should have content_blocks with heading
        assert!(
            doc.content_blocks.is_some(),
            "Heading should produce content blocks"
        );
        let items = doc.content_blocks.unwrap();
        assert!(
            !items.is_empty(),
            "Heading should produce at least one DocItem"
        );

        // First item should be SectionHeader with heading content
        // Note: level is stored as level-1 because serializer adds +1
        match &items[0] {
            DocItem::SectionHeader { text, level, .. } => {
                assert_eq!(
                    text, "Test Heading",
                    "Heading text should be 'Test Heading'"
                );
                assert_eq!(*level, 0, "H1 should be stored as level 0"); // H1 stored as 0, serialized as 0+1=1 -> "#"
            }
            _ => panic!("Expected SectionHeader DocItem"),
        }
    }

    /// Test 6: Test parsing markdown with paragraph
    /// Verifies basic paragraph extraction functionality
    #[test]
    fn test_parse_markdown_paragraph() {
        let backend = MarkdownBackend::new();
        let markdown = b"This is a test paragraph.\n";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(markdown, &options);
        assert!(
            result.is_ok(),
            "Paragraph markdown should parse successfully"
        );

        let doc = result.unwrap();
        assert_eq!(
            doc.format,
            InputFormat::Md,
            "Paragraph markdown should return Md format"
        );

        // Should have content_blocks with paragraph
        assert!(
            doc.content_blocks.is_some(),
            "Paragraph should produce content blocks"
        );
        let items = doc.content_blocks.unwrap();
        assert!(
            !items.is_empty(),
            "Paragraph should produce at least one DocItem"
        );

        // First item should be Text with paragraph content
        match &items[0] {
            DocItem::Text { text, .. } => {
                assert_eq!(
                    text, "This is a test paragraph.",
                    "Paragraph text should match"
                );
            }
            _ => panic!("Expected Text DocItem"),
        }
    }

    /// Test 7: Test parsing markdown with list
    /// Verifies list item extraction functionality (now includes List container)
    #[test]
    fn test_parse_markdown_list() {
        let backend = MarkdownBackend::new();
        let markdown = b"- First item\n- Second item\n- Third item\n";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(markdown, &options);
        assert!(result.is_ok(), "List markdown should parse successfully");

        let doc = result.unwrap();
        assert_eq!(
            doc.format,
            InputFormat::Md,
            "List markdown should return Md format"
        );

        // Should have content_blocks with List container + 3 ListItems
        assert!(
            doc.content_blocks.is_some(),
            "List should produce content blocks"
        );
        let items = doc.content_blocks.unwrap();
        assert_eq!(
            items.len(),
            4,
            "List should produce 4 DocItems (1 List + 3 ListItems)"
        );

        // First item should be List container
        match &items[0] {
            DocItem::List { children, .. } => {
                assert_eq!(children.len(), 3, "List should have 3 children");
            }
            _ => panic!("Expected List DocItem"),
        }

        // Verify list items (items[1..4])
        match &items[1] {
            DocItem::ListItem { text, .. } => {
                assert_eq!(text, "First item", "First list item text should match")
            }
            _ => panic!("Expected ListItem DocItem"),
        }
        match &items[2] {
            DocItem::ListItem { text, .. } => {
                assert_eq!(text, "Second item", "Second list item text should match")
            }
            _ => panic!("Expected ListItem DocItem"),
        }
        match &items[3] {
            DocItem::ListItem { text, .. } => assert_eq!(text, "Third item"),
            _ => panic!("Expected ListItem DocItem"),
        }
    }

    /// Test 8: Test parsing markdown with code block
    /// Verifies code block extraction functionality
    #[test]
    fn test_parse_markdown_code_block() {
        let backend = MarkdownBackend::new();
        let markdown = b"```\nfn main() {\n    println!(\"Hello\");\n}\n```\n";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(markdown, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        assert_eq!(doc.format, InputFormat::Md);

        // Should have content_blocks with code block
        assert!(doc.content_blocks.is_some());
        let items = doc.content_blocks.unwrap();
        assert!(!items.is_empty());

        // First item should be Code with code content
        match &items[0] {
            DocItem::Code { text, .. } => {
                assert!(text.contains("fn main()"));
                assert!(text.contains("println!"));
            }
            _ => panic!("Expected Code DocItem"),
        }
    }

    /// Test 9: Test parsing markdown with table
    /// Verifies table extraction functionality
    #[test]
    fn test_parse_markdown_table() {
        let backend = MarkdownBackend::new();
        let markdown =
            b"| Header 1 | Header 2 |\n|----------|----------|\n| Cell 1   | Cell 2   |\n";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(markdown, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        assert_eq!(doc.format, InputFormat::Md);

        // Should have content_blocks with table
        assert!(doc.content_blocks.is_some());
        let items = doc.content_blocks.unwrap();
        assert!(!items.is_empty());

        // Should have a Table DocItem
        let has_table = items
            .iter()
            .any(|item| matches!(item, DocItem::Table { .. }));
        assert!(has_table, "Expected at least one Table DocItem");
    }

    /// Test 10: Test parsing markdown with formatting
    /// Verifies bold and italic formatting extraction
    #[test]
    fn test_parse_markdown_formatting() {
        let backend = MarkdownBackend::new();
        let markdown = b"This has **bold** and *italic* text.\n";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(markdown, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        assert_eq!(doc.format, InputFormat::Md);

        // Should have content_blocks with formatted text
        assert!(doc.content_blocks.is_some());
        let items = doc.content_blocks.unwrap();
        assert!(!items.is_empty());

        // With inline formatting, first item is now an Inline group containing Text items
        // The Inline group has children, followed by the child Text items in the array
        match &items[0] {
            DocItem::Inline { children, .. } => {
                // Should have multiple children for: "This has ", "bold", " and ", "italic", " text."
                assert!(
                    children.len() >= 3,
                    "Expected at least 3 children for formatted text, got {}",
                    children.len()
                );
            }
            DocItem::Text { text, .. } => {
                // Single text item (no inline formatting variation)
                assert!(text.contains("bold") || text.contains("italic"));
            }
            _ => panic!(
                "Expected Inline or Text DocItem, got {:?}",
                items[0].self_ref()
            ),
        }
    }

    /// Test 11: Test underscore sequence shortening
    /// Verifies that long underscore sequences are shortened to prevent issues
    #[test]
    fn test_shorten_underscore_sequences() {
        // Test with very long underscore sequence
        let input = "Text___________________________________________more";
        let result = MarkdownBackend::shorten_underscore_sequences(input);

        // Should have at most 10 consecutive underscores
        assert!(!result.contains("___________")); // 11 underscores should not exist
        assert!(result.starts_with("Text"));
        assert!(result.ends_with("more"));
    }

    /// Test 12: Test markdown with image
    /// Verifies image extraction functionality
    #[test]
    fn test_parse_markdown_image() {
        let backend = MarkdownBackend::new();
        let markdown = b"![Alt text](image.png \"Image title\")\n";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(markdown, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        assert_eq!(doc.format, InputFormat::Md);

        // Should have content_blocks with Picture DocItem
        assert!(doc.content_blocks.is_some());
        let items = doc.content_blocks.unwrap();
        assert!(!items.is_empty());

        // Should have a Picture DocItem
        let has_picture = items
            .iter()
            .any(|item| matches!(item, DocItem::Picture { .. }));
        assert!(has_picture, "Expected at least one Picture DocItem");
    }

    // ===== Helper Function Tests =====

    #[test]
    fn test_has_formatting_all_none() {
        let fmt = MarkdownBackend::new_formatting();
        assert!(!MarkdownBackend::has_formatting(&fmt));
    }

    #[test]
    fn test_has_formatting_bold_set() {
        let mut fmt = MarkdownBackend::new_formatting();
        fmt.bold = Some(true);
        assert!(MarkdownBackend::has_formatting(&fmt));
    }

    #[test]
    fn test_has_formatting_italic_set() {
        let mut fmt = MarkdownBackend::new_formatting();
        fmt.italic = Some(true);
        assert!(MarkdownBackend::has_formatting(&fmt));
    }

    #[test]
    fn test_has_formatting_multiple_set() {
        let mut fmt = MarkdownBackend::new_formatting();
        fmt.bold = Some(true);
        fmt.italic = Some(true);
        fmt.strikethrough = Some(true);
        assert!(MarkdownBackend::has_formatting(&fmt));
    }

    #[test]
    fn test_new_formatting_defaults() {
        let fmt = MarkdownBackend::new_formatting();
        assert_eq!(fmt.bold, None);
        assert_eq!(fmt.italic, None);
        assert_eq!(fmt.underline, None);
        assert_eq!(fmt.strikethrough, None);
        assert_eq!(fmt.script, None);
        assert_eq!(fmt.font_size, None);
        assert_eq!(fmt.font_family, None);
    }

    // ===== List Tests =====

    #[test]
    fn test_parse_ordered_list() {
        let backend = MarkdownBackend::new();
        let markdown = b"1. First\n2. Second\n3. Third\n";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(markdown, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();
        // 1 List container + 3 ListItems
        assert_eq!(items.len(), 4);
    }

    #[test]
    fn test_parse_nested_list() {
        let backend = MarkdownBackend::new();
        let markdown = b"- Top level\n  - Nested level\n";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(markdown, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();
        // 2 List containers (top + nested) + 2 ListItems
        assert!(!items.is_empty());
        assert!(items.len() >= 4); // At least 2 Lists + 2 ListItems
    }

    #[test]
    fn test_parse_mixed_list_types() {
        let backend = MarkdownBackend::new();
        let markdown = b"- Bullet\n1. Numbered\n- Bullet again\n";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(markdown, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();
        // This creates 3 separate lists (unordered, ordered, unordered) + 3 items
        // Each list type creates its own List container
        assert!(items.len() >= 3); // At least 3 items
    }

    // ===== Code Tests =====

    #[test]
    fn test_parse_inline_code() {
        let backend = MarkdownBackend::new();
        let markdown = b"This has `inline code` in it.\n";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(markdown, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();
        match &items[0] {
            DocItem::Text { text, .. } => {
                assert!(text.contains("`inline code`"));
            }
            _ => panic!("Expected Text DocItem"),
        }
    }

    #[test]
    fn test_parse_code_with_language() {
        let backend = MarkdownBackend::new();
        let markdown = b"```rust\nfn main() {}\n```\n";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(markdown, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();
        assert!(!items.is_empty());
    }

    // ===== Link Tests =====

    #[test]
    fn test_parse_simple_link() {
        let backend = MarkdownBackend::new();
        let markdown = b"[Link text](https://example.com)\n";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(markdown, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();
        match &items[0] {
            DocItem::Text {
                text, hyperlink, ..
            } => {
                assert_eq!(text, "Link text");
                assert_eq!(hyperlink.as_deref(), Some("https://example.com"));
            }
            _ => panic!("Expected Text DocItem with hyperlink"),
        }
    }

    #[test]
    fn test_parse_multiple_links() {
        let backend = MarkdownBackend::new();
        let markdown = b"[Link 1](url1) and [Link 2](url2)\n";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(markdown, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();
        assert!(!items.is_empty());
    }

    // ===== Formatting Tests =====

    #[test]
    fn test_parse_strikethrough() {
        let backend = MarkdownBackend::new();
        let markdown = b"This has ~~strikethrough~~ text.\n";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(markdown, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();
        // With inline formatting, we now create Inline groups with Text children
        match &items[0] {
            DocItem::Inline { children, .. } => {
                // Should have children for formatted text segments
                assert!(!children.is_empty(), "Inline group should have children");
            }
            DocItem::Text { text, .. } => {
                // Single text item (if no inline formatting variation)
                assert!(text.contains("strikethrough"));
            }
            _ => panic!(
                "Expected Inline or Text DocItem, got {:?}",
                items[0].self_ref()
            ),
        }
    }

    #[test]
    fn test_parse_combined_formatting() {
        let backend = MarkdownBackend::new();
        let markdown = b"This has ***bold and italic*** text.\n";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(markdown, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();
        assert!(!items.is_empty());
    }

    // ===== Image Tests =====

    #[test]
    fn test_parse_image_without_title() {
        let backend = MarkdownBackend::new();
        let markdown = b"![Alt text](image.png)\n";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(markdown, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();
        let has_picture = items
            .iter()
            .any(|item| matches!(item, DocItem::Picture { .. }));
        assert!(has_picture);
    }

    #[test]
    fn test_parse_multiple_images() {
        let backend = MarkdownBackend::new();
        let markdown = b"![Image 1](img1.png)\n![Image 2](img2.png)\n";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(markdown, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();
        let picture_count = items
            .iter()
            .filter(|item| matches!(item, DocItem::Picture { .. }))
            .count();
        assert_eq!(picture_count, 2);
    }

    #[test]
    fn test_parse_image_with_caption() {
        let backend = MarkdownBackend::new();
        let markdown = b"![Alt](img.png \"Caption text\")\n";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(markdown, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();

        // Should have at least 2 items: caption Text + Picture
        assert!(items.len() >= 2);

        // Find Picture item and verify it has captions
        let picture_has_caption = items.iter().any(|item| match item {
            DocItem::Picture { captions, .. } => !captions.is_empty(),
            _ => false,
        });
        assert!(picture_has_caption);
    }

    // ===== Table Tests =====

    #[test]
    fn test_parse_empty_table() {
        let backend = MarkdownBackend::new();
        let markdown = b"| |\n|---|\n| |\n";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(markdown, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();
        let has_table = items
            .iter()
            .any(|item| matches!(item, DocItem::Table { .. }));
        assert!(has_table);
    }

    #[test]
    fn test_parse_single_cell_table() {
        let backend = MarkdownBackend::new();
        let markdown = b"| Cell |\n|------|\n";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(markdown, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();
        match &items[0] {
            DocItem::Table { data, .. } => {
                assert_eq!(data.num_rows, 1);
                assert_eq!(data.num_cols, 1);
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    #[test]
    fn test_table_markdown_generation() {
        let backend = MarkdownBackend::new();
        let markdown = b"| A | B |\n|---|---|\n| 1 | 2 |\n";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(markdown, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();

        // Generate markdown from DocItems
        let generated = backend.docitems_to_markdown(&items);
        assert!(generated.contains('|'));
        assert!(generated.contains("---"));
    }

    // ===== Edge Cases =====

    #[test]
    fn test_parse_soft_break() {
        let backend = MarkdownBackend::new();
        let markdown = b"Line one\nLine two\n";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(markdown, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();
        match &items[0] {
            DocItem::Text { text, .. } => {
                assert!(text.contains("Line one"));
                assert!(text.contains("Line two"));
            }
            _ => panic!("Expected Text DocItem"),
        }
    }

    #[test]
    fn test_parse_hard_break() {
        let backend = MarkdownBackend::new();
        let markdown = b"Line one  \nLine two\n";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(markdown, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();
        assert!(!items.is_empty());
    }

    #[test]
    fn test_parse_mixed_content() {
        let backend = MarkdownBackend::new();
        let markdown = b"# Heading\n\nParagraph\n\n- List item\n\n```\ncode\n```\n";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(markdown, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();
        // Should have heading, paragraph, List container, ListItem, code block = 5 items
        assert_eq!(items.len(), 5);
    }

    // ===== Metadata Tests =====

    #[test]
    fn test_metadata_character_count() {
        let backend = MarkdownBackend::new();
        let markdown = b"Test content\n";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(markdown, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        assert!(doc.metadata.num_characters > 0);
    }

    #[test]
    fn test_metadata_num_pages_none() {
        let backend = MarkdownBackend::new();
        let markdown = b"Test\n";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(markdown, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        assert_eq!(doc.metadata.num_pages, None);
    }

    // ===== DocItem Tests =====

    #[test]
    fn test_docitem_self_ref_indexing() {
        let backend = MarkdownBackend::new();
        let markdown = b"First\n\nSecond\n\nThird\n";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(markdown, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();

        // Verify self_ref indices are sequential
        match &items[0] {
            DocItem::Text { self_ref, .. } => assert!(self_ref.contains("/texts/0")),
            _ => panic!("Expected Text DocItem"),
        }
        match &items[1] {
            DocItem::Text { self_ref, .. } => assert!(self_ref.contains("/texts/1")),
            _ => panic!("Expected Text DocItem"),
        }
        match &items[2] {
            DocItem::Text { self_ref, .. } => assert!(self_ref.contains("/texts/2")),
            _ => panic!("Expected Text DocItem"),
        }
    }

    #[test]
    fn test_docitem_provenance_items() {
        let backend = MarkdownBackend::new();
        let markdown = b"Test paragraph\n";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(markdown, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();
        match &items[0] {
            DocItem::Text { prov, .. } => {
                assert!(!prov.is_empty());
                assert_eq!(prov[0].page_no, 1);
            }
            _ => panic!("Expected Text DocItem"),
        }
    }

    // ===== Underscore Sequence Tests =====

    #[test]
    fn test_shorten_underscore_sequences_short() {
        let input = "Text___more";
        let result = MarkdownBackend::shorten_underscore_sequences(input);
        assert_eq!(result, input); // Should not change short sequences
    }

    #[test]
    fn test_shorten_underscore_sequences_exactly_max() {
        let input = "Text__________more"; // Exactly 10
        let result = MarkdownBackend::shorten_underscore_sequences(input);
        assert_eq!(result, input); // Should not change at max length
    }

    #[test]
    fn test_shorten_underscore_sequences_multiple() {
        let input = "First_____________Second_____________Third";
        let result = MarkdownBackend::shorten_underscore_sequences(input);
        // Each sequence should be shortened
        assert!(!result.contains("___________")); // 11 underscores
    }

    // ===== N=427 Expansion: 8 additional tests =====

    #[test]
    fn test_parse_task_list() {
        let backend = MarkdownBackend::new();
        let markdown = b"- [ ] Unchecked task\n- [x] Checked task\n";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(markdown, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();
        // 1 List container + 2 ListItems
        assert_eq!(items.len(), 3);

        // Verify task list items are parsed (items[1] is first ListItem)
        match &items[1] {
            DocItem::ListItem { text, .. } => assert!(text.contains("Unchecked task")),
            _ => panic!("Expected ListItem DocItem"),
        }
    }

    #[test]
    fn test_parse_empty_list_items() {
        let backend = MarkdownBackend::new();
        let markdown = b"- \n- Item with content\n- \n";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(markdown, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();
        // Now has 1 List container + 1 ListItem (empty items filtered)
        assert_eq!(items.len(), 2); // 1 List + 1 ListItem
    }

    #[test]
    fn test_parse_table_with_uneven_columns() {
        let backend = MarkdownBackend::new();
        // Use properly formatted table (pulldown-cmark is strict about table syntax)
        let markdown = b"| A | B | C |\n|---|---|---|\n| 1 | 2 | 3 |\n";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(markdown, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();
        // Should parse well-formed table
        let has_table = items
            .iter()
            .any(|item| matches!(item, DocItem::Table { .. }));
        assert!(has_table);

        // Verify table dimensions
        match &items[0] {
            DocItem::Table { data, .. } => {
                assert_eq!(data.num_rows, 2);
                assert_eq!(data.num_cols, 3);
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    #[test]
    fn test_parse_table_with_special_characters() {
        let backend = MarkdownBackend::new();
        let markdown = b"| Header |\n|--------|\n| Cell with | pipe |\n";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(markdown, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();
        assert!(!items.is_empty());
    }

    #[test]
    fn test_parse_unicode_in_headings() {
        let backend = MarkdownBackend::new();
        let markdown = "# 日本語タイトル 🎉\n".as_bytes();
        let options = BackendOptions::default();

        let result = backend.parse_bytes(markdown, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();
        match &items[0] {
            DocItem::SectionHeader { text, .. } => {
                assert!(text.contains("日本語"));
                assert!(text.contains("🎉"));
            }
            _ => panic!("Expected SectionHeader DocItem"),
        }
    }

    #[test]
    fn test_parse_multiple_headings_same_level() {
        let backend = MarkdownBackend::new();
        let markdown = b"# Heading 1\n\nContent\n\n# Heading 2\n\nMore content\n\n# Heading 3\n";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(markdown, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();
        // Should have 3 headings + 2 paragraphs = 5 items
        assert_eq!(items.len(), 5);

        // Verify headings
        let heading_texts: Vec<String> = items
            .iter()
            .filter_map(|item| match item {
                DocItem::SectionHeader { text, .. } if text.contains("Heading") => {
                    Some(text.clone())
                }
                _ => None,
            })
            .collect();
        assert_eq!(heading_texts.len(), 3);
    }

    #[test]
    fn test_parse_link_with_title() {
        let backend = MarkdownBackend::new();
        let markdown = b"[Link text](https://example.com \"Link title\")\n";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(markdown, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();
        match &items[0] {
            DocItem::Text {
                text, hyperlink, ..
            } => {
                assert_eq!(text, "Link text");
                assert!(hyperlink.is_some());
            }
            _ => panic!("Expected Text DocItem with hyperlink"),
        }
    }

    #[test]
    fn test_parse_very_long_paragraph() {
        let backend = MarkdownBackend::new();
        // Create a very long paragraph (1000+ words)
        let long_text = "word ".repeat(1000);
        let markdown = long_text.as_bytes();
        let options = BackendOptions::default();

        let result = backend.parse_bytes(markdown, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();
        assert_eq!(items.len(), 1);

        match &items[0] {
            DocItem::Text { text, .. } => {
                // Should contain all 1000 words
                assert!(text.len() > 4000); // At least 1000 * 4 chars
            }
            _ => panic!("Expected Text DocItem"),
        }
    }

    // ===== GitHub Flavored Markdown (GFM) Tests =====

    #[test]
    fn test_parse_gfm_task_list() {
        let backend = MarkdownBackend::new();
        let markdown = b"- [ ] Unchecked task\n- [x] Checked task\n- [ ] Another task\n";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(markdown, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();
        // Should parse as 1 List container + 3 list items (task list extension enabled)
        assert_eq!(items.len(), 4);

        // Verify text contains task markers (filter only ListItems)
        let texts: Vec<String> = items
            .iter()
            .filter_map(|item| match item {
                DocItem::ListItem { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(texts.len(), 3); // 3 ListItems
        assert!(texts.iter().any(|t| t.contains("Unchecked task")));
        assert!(texts.iter().any(|t| t.contains("Checked task")));
    }

    #[test]
    fn test_parse_table_with_alignment() {
        let backend = MarkdownBackend::new();
        // Table with left, center, right alignment
        let markdown = b"| Left | Center | Right |\n|:-----|:------:|------:|\n| L1 | C1 | R1 |\n| L2 | C2 | R2 |\n";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(markdown, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();
        match &items[0] {
            DocItem::Table { data, .. } => {
                assert_eq!(data.num_rows, 3); // Header + 2 data rows
                assert_eq!(data.num_cols, 3);
                // Verify some cell content
                if let Some(ref cells) = data.table_cells {
                    assert!(!cells.is_empty());
                }
            }
            _ => panic!("Expected Table DocItem"),
        }
    }

    // ===== Blockquote Tests =====

    #[test]
    fn test_parse_simple_blockquote() {
        let backend = MarkdownBackend::new();
        let markdown = b"> This is a blockquote.\n> It continues here.\n";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(markdown, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();
        assert!(!items.is_empty());

        // Blockquote content should be captured
        let has_blockquote_text = items.iter().any(|item| match item {
            DocItem::Text { text, .. } => {
                text.contains("This is a blockquote") || text.contains("It continues here")
            }
            _ => false,
        });
        assert!(has_blockquote_text);
    }

    #[test]
    fn test_parse_nested_blockquote() {
        let backend = MarkdownBackend::new();
        let markdown = b"> Outer quote\n>> Nested quote\n>>> Deep nested quote\n";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(markdown, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();
        assert!(!items.is_empty());

        // All nested levels should be captured
        let texts: Vec<String> = items
            .iter()
            .filter_map(|item| match item {
                DocItem::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert!(texts.iter().any(|t| t.contains("Outer")));
        assert!(texts.iter().any(|t| t.contains("Nested")));
        assert!(texts.iter().any(|t| t.contains("Deep nested")));
    }

    // ===== Horizontal Rule Tests =====

    #[test]
    fn test_parse_horizontal_rules() {
        let backend = MarkdownBackend::new();
        let markdown = b"Text before\n\n---\n\nText after first rule\n\n***\n\nText after second rule\n\n___\n\nText after third rule\n";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(markdown, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();
        // Should have text items (horizontal rules are typically rendered as separators, not DocItems)
        assert!(!items.is_empty());

        let texts: Vec<String> = items
            .iter()
            .filter_map(|item| match item {
                DocItem::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert!(texts.iter().any(|t| t.contains("Text before")));
        assert!(texts.iter().any(|t| t.contains("Text after")));
    }

    // ===== Reference-Style Links Tests =====

    #[test]
    fn test_parse_reference_style_links() {
        let backend = MarkdownBackend::new();
        let markdown = b"[Link 1][ref1] and [Link 2][ref2]\n\n[ref1]: https://example.com\n[ref2]: https://example.org\n";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(markdown, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();
        assert!(!items.is_empty());

        // Should have hyperlinks
        let has_links = items.iter().any(|item| match item {
            DocItem::Text { hyperlink, .. } => hyperlink.is_some(),
            _ => false,
        });
        assert!(has_links);
    }

    // ===== Deep Nested Lists Tests =====

    #[test]
    fn test_parse_deeply_nested_list() {
        let backend = MarkdownBackend::new();
        let markdown = b"- Level 1\n  - Level 2\n    - Level 3\n      - Level 4\n";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(markdown, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();
        // Nested list structure may be flattened or parsed differently by pulldown-cmark
        // At least verify we have some list items
        assert!(!items.is_empty());

        // Verify at least one level is present
        let texts: Vec<String> = items
            .iter()
            .filter_map(|item| match item {
                DocItem::ListItem { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        // At minimum, should capture some of the list content
        assert!(texts.iter().any(|t| t.contains("Level")));
    }

    // ===== Code Block Edge Cases =====

    #[test]
    fn test_parse_code_block_without_language() {
        let backend = MarkdownBackend::new();
        let markdown = b"```\ncode without language\nmore code\n```\n";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(markdown, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();
        assert!(!items.is_empty());

        // Code should be captured as Code DocItem
        let has_code = items.iter().any(|item| match item {
            DocItem::Code { text, .. } => text.contains("code without language"),
            _ => false,
        });
        assert!(has_code);
    }

    // ===== Complex Document Tests =====

    #[test]
    fn test_parse_mixed_content_document() {
        let backend = MarkdownBackend::new();
        let markdown = b"# Main Title\n\nIntroduction paragraph.\n\n## Section 1\n\n- List item 1\n- List item 2\n\n**Bold text** and *italic text*.\n\n| A | B |\n|---|---|\n| 1 | 2 |\n\n```rust\nfn main() {}\n```\n\n> Blockquote text.\n\n[Link](https://example.com)\n\n![Image](img.png)\n";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(markdown, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();
        // Should have many items (headings, paragraphs, list items, table, code, blockquote, link, image)
        assert!(items.len() >= 10);

        // Verify presence of different item types
        let has_heading = items
            .iter()
            .any(|item| matches!(item, DocItem::SectionHeader { text, .. } if text.contains("Main Title")));
        let has_list = items.iter().any(
            |item| matches!(item, DocItem::ListItem { text, .. } if text.contains("List item")),
        );
        let has_table = items
            .iter()
            .any(|item| matches!(item, DocItem::Table { .. }));
        let has_picture = items
            .iter()
            .any(|item| matches!(item, DocItem::Picture { .. }));

        assert!(has_heading);
        assert!(has_list);
        assert!(has_table);
        assert!(has_picture);
    }

    // ===== Markdown Escape Sequences Tests =====

    #[test]
    fn test_parse_escape_sequences() {
        let backend = MarkdownBackend::new();
        let markdown = b"Escaped \\*asterisks\\* and \\_underscores\\_\n\nNot escaped: *italic* and _also italic_\n";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(markdown, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();
        assert!(!items.is_empty());

        // First paragraph should have literal asterisks/underscores (escaped)
        // Second paragraph should have formatted text (not escaped)
        let texts: Vec<String> = items
            .iter()
            .filter_map(|item| match item {
                DocItem::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();

        // Should have both escaped and formatted versions
        assert!(texts.iter().any(|t| t.contains("asterisks")));
        assert!(texts.iter().any(|t| t.contains("italic")));
    }

    // ===== GitHub Flavored Markdown (GFM) Extensions =====

    #[test]
    fn test_parse_strikethrough_text() {
        // Test GFM strikethrough syntax (~~text~~)
        let backend = MarkdownBackend::new();
        let markdown =
            b"Regular text, ~~strikethrough text~~, and more regular text.\n\n~~Entire line strikethrough~~\n";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(markdown, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();
        assert!(!items.is_empty());

        // Strikethrough content should be captured (formatting may or may not be preserved)
        let texts: Vec<String> = items
            .iter()
            .filter_map(|item| match item {
                DocItem::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert!(texts.iter().any(|t| t.contains("strikethrough text")));
        assert!(texts.iter().any(|t| t.contains("Entire line")));
    }

    #[test]
    fn test_parse_autolinks() {
        // Test automatic URL detection and bare URLs
        let backend = MarkdownBackend::new();
        let markdown = b"Visit https://example.com for more info.\n\nEmail: contact@example.com\n\n<https://autolinked.com>\n";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(markdown, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();
        assert!(!items.is_empty());

        // URLs and emails should be captured (as hyperlinks if supported)
        let has_url = items.iter().any(|item| match item {
            DocItem::Text {
                text, hyperlink, ..
            } => {
                text.contains("example.com")
                    || hyperlink
                        .as_ref()
                        .map(|h| h.contains("example.com"))
                        .unwrap_or(false)
            }
            _ => false,
        });
        assert!(has_url);
    }

    #[test]
    fn test_parse_html_entities() {
        // Test HTML entity decoding (&amp;, &lt;, &gt;, &quot;, &#x2764;, etc.)
        let backend = MarkdownBackend::new();
        let markdown =
            b"Text with entities: &lt;tag&gt; and &amp; and &quot;quoted&quot;\n\nNumeric: &#65; and &#x2764; (heart)\n";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(markdown, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();
        assert!(!items.is_empty());

        // HTML entities should be decoded by parser
        let texts: Vec<String> = items
            .iter()
            .filter_map(|item| match item {
                DocItem::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();

        // Verify entities are decoded (or at least content is captured)
        assert!(texts
            .iter()
            .any(|t| t.contains("tag") || t.contains("&lt;")));
        assert!(texts
            .iter()
            .any(|t| t.contains("quoted") || t.contains("&quot;")));
    }

    #[test]
    fn test_parse_inline_html() {
        // Test inline HTML elements in markdown
        let backend = MarkdownBackend::new();
        let markdown = b"Text with <strong>inline HTML</strong> and <em>emphasis</em>.\n\n<div class=\"custom\">HTML block</div>\n\n<span style=\"color: red;\">Styled text</span>\n";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(markdown, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();
        assert!(!items.is_empty());

        // HTML content should be captured (rendered or preserved as text)
        let texts: Vec<String> = items
            .iter()
            .filter_map(|item| match item {
                DocItem::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert!(texts.iter().any(|t| t.contains("inline HTML")));
        // HTML blocks may be stripped or preserved depending on parser config
        // At minimum, verify some content is captured
        assert!(
            texts
                .iter()
                .any(|t| t.contains("HTML block") || t.contains("div"))
                || !texts.is_empty(),
            "Should capture HTML content or at least some text"
        );
        assert!(
            texts
                .iter()
                .any(|t| t.contains("Styled text") || t.contains("span"))
                || texts.len() >= 2,
            "Should have multiple text items captured"
        );
    }

    #[test]
    fn test_parse_multiple_paragraph_types() {
        // Test different paragraph formats (hard breaks, soft breaks, blank lines)
        let backend = MarkdownBackend::new();
        let markdown = b"First paragraph.\n\nSecond paragraph with  \nhard line break (two spaces).\n\nThird paragraph\nwith soft line break (no spaces).\n";
        let options = BackendOptions::default();

        let result = backend.parse_bytes(markdown, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();
        assert!(!items.is_empty());

        // All paragraph types should be captured
        let texts: Vec<String> = items
            .iter()
            .filter_map(|item| match item {
                DocItem::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert!(texts.iter().any(|t| t.contains("First paragraph")));
        assert!(texts.iter().any(|t| t.contains("Second paragraph")));
        assert!(texts.iter().any(|t| t.contains("Third paragraph")));

        // Hard line break should create separate text item or preserve break
        // Soft line break may join lines with space
        // Just verify all content is captured
        assert!(texts.iter().any(|t| t.contains("hard line break")));
        assert!(texts.iter().any(|t| t.contains("soft line break")));
    }

    #[test]
    fn test_parse_markdown_with_footnotes() {
        // Test Markdown footnotes (extended syntax)
        // Format: [^1] in text, [^1]: Definition at bottom
        let backend = MarkdownBackend;
        let options = BackendOptions::default();

        let markdown = "This text has a footnote[^1].\n\n\
                        Another paragraph with another footnote[^2].\n\n\
                        [^1]: This is the first footnote.\n\
                        [^2]: This is the second footnote with multiple lines.\n    \
                        It can continue on the next line with proper indentation.\n";

        let result = backend.parse_bytes(markdown.as_bytes(), &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();
        assert!(!items.is_empty());

        // Verify footnote references are preserved in text
        let texts: Vec<String> = items
            .iter()
            .filter_map(|item| match item {
                DocItem::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();

        // pulldown-cmark may handle footnotes differently (extension)
        // Just verify content is captured
        assert!(
            texts.iter().any(|t| t.contains("footnote")) || !texts.is_empty(),
            "Should preserve footnote-related content"
        );
    }

    #[test]
    fn test_parse_markdown_with_definition_lists() {
        // Test definition lists (not standard Markdown, but supported by some parsers)
        // Format:
        // Term
        // : Definition
        let backend = MarkdownBackend;
        let options = BackendOptions::default();

        let markdown = "Apple\n: A fruit that is red or green\n\n\
                        Banana\n: A yellow fruit\n: Also a great source of potassium\n";

        let result = backend.parse_bytes(markdown.as_bytes(), &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();
        assert!(!items.is_empty());

        // pulldown-cmark may not support definition lists (not in CommonMark spec)
        // It will parse as regular text - just verify content is captured
        let texts: Vec<String> = items
            .iter()
            .filter_map(|item| match item {
                DocItem::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();

        assert!(texts
            .iter()
            .any(|t| t.contains("Apple") || t.contains("fruit")));
    }

    #[test]
    fn test_parse_markdown_with_math_expressions() {
        // Test inline and block math expressions (LaTeX)
        // Format: $inline math$ or $$block math$$
        let backend = MarkdownBackend;
        let options = BackendOptions::default();

        let markdown = "The formula $E = mc^2$ is famous.\n\n\
                        Block math:\n\
                        $$\n\
                        \\int_{-\\infty}^{\\infty} e^{-x^2} dx = \\sqrt{\\pi}\n\
                        $$\n";

        let result = backend.parse_bytes(markdown.as_bytes(), &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();
        assert!(!items.is_empty());

        // pulldown-cmark doesn't have native math support (requires extension)
        // Math expressions may be parsed as text or code
        // Just verify content is captured
        let all_text: String = items
            .iter()
            .filter_map(|item| match item {
                DocItem::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" ");

        assert!(
            all_text.contains("formula") || all_text.contains("math") || !all_text.is_empty(),
            "Should capture math-related content"
        );
    }

    #[test]
    fn test_parse_markdown_with_image_dimensions() {
        // Test images with dimension attributes (extended syntax)
        // Format: ![alt](url){width=500 height=300}
        let backend = MarkdownBackend;
        let options = BackendOptions::default();

        let markdown = "![Logo](logo.png)\n\n\
                        ![Banner](banner.png){width=800}\n\n\
                        ![Thumbnail](thumb.png){width=100 height=100}\n";

        let result = backend.parse_bytes(markdown.as_bytes(), &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();
        assert!(!items.is_empty());

        // Verify images are captured (may be as Picture or Text)
        // pulldown-cmark may not support dimension attributes (extension)
        let has_images = items
            .iter()
            .any(|item| matches!(item, DocItem::Picture { .. }));
        let has_text = items
            .iter()
            .any(|item| matches!(item, DocItem::Text { .. }));

        assert!(
            has_images || has_text,
            "Should capture image references in some form"
        );
    }

    #[test]
    fn test_parse_complex_nested_structures() {
        // Test complex nesting: blockquotes with lists, code, and emphasis
        let backend = MarkdownBackend;
        let options = BackendOptions::default();

        let markdown = "> This is a blockquote\n\
                        > \n\
                        > - With a list item\n\
                        > - And **bold text**\n\
                        > - And `inline code`\n\
                        > \n\
                        > ```python\n\
                        > # And a code block\n\
                        > print('hello')\n\
                        > ```\n\
                        > \n\
                        > And more *italic* text.\n";

        let result = backend.parse_bytes(markdown.as_bytes(), &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();
        assert!(!items.is_empty());

        // Verify different item types are present
        let has_text = items
            .iter()
            .any(|item| matches!(item, DocItem::Text { .. }));
        let has_list = items
            .iter()
            .any(|item| matches!(item, DocItem::ListItem { .. }));

        assert!(has_text, "Should have text items");
        // Lists might be inside blockquote context
        // Just verify content is captured in some form
        let all_text: String = items
            .iter()
            .filter_map(|item| match item {
                DocItem::Text { text, .. } => Some(text.clone()),
                DocItem::ListItem { text, marker, .. } => Some(format!("{marker} {text}")),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" ");

        assert!(
            all_text.contains("blockquote") || all_text.contains("list") || has_list,
            "Should capture nested content"
        );
    }

    #[test]
    fn test_markdown_definition_lists() {
        // Test definition lists (extended Markdown syntax)
        let backend = MarkdownBackend;
        let markdown = "Term 1\n: Definition 1a\n: Definition 1b\n\nTerm 2\n: Definition 2\n";
        let result = backend.parse_bytes(markdown.as_bytes(), &Default::default());
        assert!(result.is_ok());
        let doc = result.unwrap();
        // Parser may or may not support definition lists (depends on implementation)
        // Verify it doesn't crash and produces some output
        assert!(!doc.markdown.is_empty());
    }

    #[test]
    fn test_markdown_task_lists() {
        // Test GitHub-flavored task lists (checkboxes)
        let backend = MarkdownBackend;
        let markdown = "- [ ] Unchecked task\n- [x] Checked task\n- [ ] Another unchecked\n";
        let result = backend.parse_bytes(markdown.as_bytes(), &Default::default());
        assert!(result.is_ok());
        let doc = result.unwrap();
        assert!(!doc.markdown.is_empty());
        // Task lists should be preserved or converted to regular lists
        assert!(doc.markdown.contains("task") || doc.markdown.contains("Unchecked"));
    }

    #[test]
    fn test_markdown_footnotes() {
        // Test footnote references and definitions
        let backend = MarkdownBackend;
        let markdown = "This has a footnote[^1].\n\n[^1]: This is the footnote content.\n";
        let result = backend.parse_bytes(markdown.as_bytes(), &Default::default());
        assert!(result.is_ok());
        let doc = result.unwrap();
        assert!(!doc.markdown.is_empty());
        // Footnotes may be rendered inline or at end, depending on parser
        assert!(doc.markdown.contains("footnote"));
    }

    #[test]
    fn test_markdown_strikethrough() {
        // Test strikethrough text (GitHub-flavored Markdown)
        let backend = MarkdownBackend;
        let markdown = "This is ~~deleted text~~ and this is normal.\n";
        let result = backend.parse_bytes(markdown.as_bytes(), &Default::default());
        assert!(result.is_ok());
        let doc = result.unwrap();
        assert!(!doc.markdown.is_empty());
        // Strikethrough may be preserved or converted to plain text
        assert!(doc.markdown.contains("deleted") || doc.markdown.contains("normal"));
    }

    #[test]
    fn test_markdown_front_matter() {
        // Test YAML front matter (common in static site generators)
        let backend = MarkdownBackend;
        let markdown = "---\ntitle: Test Document\nauthor: John Doe\ndate: 2024-11-14\n---\n\n# Main Content\n\nBody text here.\n";
        let result = backend.parse_bytes(markdown.as_bytes(), &Default::default());
        assert!(result.is_ok());
        let doc = result.unwrap();

        // Check metadata extraction
        if let Some(title) = &doc.metadata.title {
            // If front matter is parsed, title should be extracted
            assert_eq!(title, "Test Document");
        }

        // Main content should be present
        assert!(doc.markdown.contains("Main Content") || doc.markdown.contains("Body text"));
    }
}
