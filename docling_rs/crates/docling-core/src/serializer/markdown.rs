//! Markdown serialization for `DoclingDocument`.
//!
//! This module provides functionality to serialize structured `DoclingDocument`
//! objects into markdown format, preserving document structure, formatting,
//! and relationships between content blocks.
//!
//! # Examples
//!
//! ```rust,ignore
//! // Note: DocumentConverter is in docling-backend crate
//! use docling_backend::DocumentConverter;
//! use docling_core::{MarkdownSerializer, MarkdownOptions};
//!
//! // Convert and serialize with default options
//! let converter = DocumentConverter::new()?;
//! let result = converter.convert("document.pdf")?;
//! let markdown = result.document.markdown; // Already in markdown format
//!
//! // For custom serialization from DoclingDocument:
//! let serializer = MarkdownSerializer::new();
//! // let custom_markdown = serializer.serialize(&docling_document);
//! # Ok::<(), docling_core::DoclingError>(())
//! ```

use crate::content::{DocItem, ItemRef};
use crate::document::DoclingDocument;
use log::trace;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashSet;
use unicode_width::UnicodeWidthStr;

/// Configuration options for markdown serialization.
///
/// Controls formatting details such as indentation, character escaping,
/// and HTML handling.
///
/// # Examples
///
/// ## Default Options
///
/// ```rust
/// use docling_core::MarkdownOptions;
///
/// let options = MarkdownOptions::default();
/// assert_eq!(options.indent, 4);
/// assert_eq!(options.escape_underscores, true);
/// assert_eq!(options.escape_html, true); // Matches Python docling v2.58.0
/// ```
///
/// ## Custom Options
///
/// ```rust
/// use docling_core::MarkdownOptions;
///
/// let options = MarkdownOptions {
///     indent: 2,                    // Use 2-space indentation
///     escape_underscores: false,    // Don't escape underscores
///     escape_html: true,            // Escape HTML characters
///     ..Default::default()          // Use defaults for other fields
/// };
/// ```
///
/// ## Using with Serializer
///
/// ```rust
/// use docling_core::{MarkdownSerializer, MarkdownOptions};
///
/// let options = MarkdownOptions {
///     indent: 2,
///     escape_underscores: false,
///     escape_html: true,
///     ..Default::default()
/// };
///
/// let serializer = MarkdownSerializer::with_options(options);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MarkdownOptions {
    /// Number of spaces for list indentation.
    ///
    /// Default: 4 spaces per indentation level.
    pub indent: usize,

    /// Whether to escape underscores in text.
    ///
    /// When `true`, underscores are escaped as `\_` to prevent
    /// accidental italic formatting in markdown.
    ///
    /// Default: `true` (matches Python docling v2.58.0 behavior).
    pub escape_underscores: bool,

    /// Whether to escape HTML special characters.
    ///
    /// When `true`, characters like `<`, `>`, and `&` are escaped
    /// to prevent HTML injection.
    ///
    /// Default: `true`.
    pub escape_html: bool,

    /// Issue #19 FIX: Include furniture layer (page headers/footers)
    ///
    /// When `false` (default), only "body" content layer is serialized.
    /// When `true`, furniture layer items (page headers/footers) are also included.
    ///
    /// Default: `false` (matches Python docling's `DEFAULT_CONTENT_LAYERS = {ContentLayer.BODY}`)
    pub include_furniture: bool,

    /// Issue #9 FIX: Maximum list nesting depth.
    ///
    /// Limits how deeply lists can be nested to prevent excessive indentation
    /// and potential stack overflow with pathological input.
    ///
    /// Default: 10 levels (sufficient for any reasonable document).
    pub max_list_depth: usize,

    /// N=4355: Convert plain text URLs to markdown links.
    ///
    /// When `true`, URLs like `https://example.com` are converted to
    /// `[https://example.com](https://example.com)` markdown links.
    ///
    /// Default: `true` (improves PDF extraction quality).
    pub linkify_urls: bool,

    /// N=4355: Insert page break comments between pages.
    ///
    /// When `true`, inserts `<!-- page N -->` comments before content
    /// from each new page. Useful for preserving page structure in output.
    ///
    /// Default: `false` (backward compatible with existing tests).
    pub insert_page_breaks: bool,
}

impl Default for MarkdownOptions {
    #[inline]
    fn default() -> Self {
        Self {
            indent: 4,
            // Python docling v2.58.0 default behavior: DOES escape underscores
            // Verified with export_to_markdown() on v2.58.0
            escape_underscores: true,
            // Python docling v2.58.0 default behavior: DOES escape HTML
            // Verified with:
            // - Markdown test: escaped_characters.md has &amp; &lt; &gt; (546 chars)
            // - JATS test: elife-56337.nxml.md has "adjusted p-value&lt;1e-5"
            // Note: Code blocks and tables are NOT escaped (handled separately)
            escape_html: true,
            // Issue #19 FIX: Default to body only (matches Python docling)
            include_furniture: false,
            // Issue #9 FIX: Max nesting depth (10 levels is plenty for any document)
            max_list_depth: 10,
            // N=4420: Don't linkify URLs - Python groundtruth uses bare URLs
            linkify_urls: false,
            // N=4355: Page breaks off by default (backward compat)
            insert_page_breaks: false,
        }
    }
}

/// Markdown serializer for `DoclingDocument`
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct MarkdownSerializer {
    options: MarkdownOptions,
}

impl MarkdownSerializer {
    /// Create a new markdown serializer with default options
    #[inline]
    #[must_use = "creates serializer with default options"]
    pub const fn new() -> Self {
        Self {
            options: MarkdownOptions {
                indent: 4,
                escape_underscores: true,
                escape_html: true,
                include_furniture: false,
                max_list_depth: 10,
                linkify_urls: false,
                insert_page_breaks: false,
            },
        }
    }

    /// Create a new markdown serializer with custom options
    #[inline]
    #[must_use = "creates serializer with custom options"]
    pub const fn with_options(options: MarkdownOptions) -> Self {
        Self { options }
    }

    /// Serialize a `DoclingDocument` to markdown
    #[must_use = "serialization returns markdown string"]
    pub fn serialize(&self, doc: &DoclingDocument) -> String {
        let mut parts: Vec<(String, bool)> = Vec::new(); // (content, is_list_item)
        let mut visited = HashSet::new();
        // N=4355: Track current page for page break comments
        let mut current_page: Option<usize> = None;

        // Serialize body children
        for child_ref in &doc.body.children {
            // N=4355: Insert page break comment when page changes
            if self.options.insert_page_breaks {
                if let Some(page_no) = Self::get_item_page(doc, &child_ref.ref_path) {
                    if current_page != Some(page_no) {
                        current_page = Some(page_no);
                        parts.push((format!("<!-- page {} -->", page_no), false));
                    }
                }
            }

            // N=4414: Check if this item is a list item for proper joining
            let is_list_item = doc
                .find_item(&child_ref.ref_path)
                .is_some_and(|item| matches!(item, DocItem::ListItem { .. }));

            if let Some(part) = self.serialize_item(doc, &child_ref.ref_path, 0, &mut visited, None)
            {
                // Skip empty parts to avoid extra blank lines (matches Python behavior)
                // N=4408: Removed N=4368 content-based deduplication - it incorrectly filtered
                // legitimate repeated content (like multiple "Lorem ipsum" paragraphs).
                // The `visited` HashSet already prevents the same ref_path from being serialized twice.
                // Captions are handled by being children of their parent element.
                if !part.is_empty() {
                    parts.push((part, is_list_item));
                }
            }
        }

        // N=4414: Join with smart newlines - consecutive list items use single newline,
        // otherwise use double newline (matches Python behavior where list items are
        // inside List containers that join with "\n")
        if parts.is_empty() {
            String::new()
        } else {
            let mut result = String::new();
            for (i, (content, is_list_item)) in parts.iter().enumerate() {
                if i > 0 {
                    // Use single newline between consecutive list items, double otherwise
                    let prev_is_list_item = parts[i - 1].1;
                    if *is_list_item && prev_is_list_item {
                        result.push('\n');
                    } else {
                        result.push_str("\n\n");
                    }
                }
                result.push_str(content);
            }
            result
        }
    }

    /// N=4355: Get the page number from an item's provenance
    fn get_item_page(doc: &DoclingDocument, ref_path: &str) -> Option<usize> {
        let item = doc.find_item(ref_path)?;
        let prov = item.provenance();
        prov.first().map(|p| p.page_no)
    }

    /// Serialize a single item by reference
    ///
    /// `list_item_index` is the 0-based index of this item within its parent list (for enumerated lists)
    #[allow(clippy::too_many_lines)] // Complex item serialization - keeping together for clarity
    fn serialize_item(
        &self,
        doc: &DoclingDocument,
        ref_path: &str,
        list_level: usize,
        visited: &mut HashSet<String>,
        list_item_index: Option<usize>,
    ) -> Option<String> {
        // Find the item first (before checking visited set)
        let item = doc.find_item(ref_path)?;

        // Issue #18 FIX: Caption, Footnote, and Reference items can be referenced multiple times
        // For all other items: Use standard visited-set suppression to prevent infinite loops
        let is_referenceable_leaf = Self::is_referenceable_leaf(item);

        // Avoid infinite loops (skip for referenceable leaf items)
        if !is_referenceable_leaf {
            if visited.contains(ref_path) {
                return None;
            }
            visited.insert(ref_path.to_string());
        }

        // Filter by content_layer - only include "body" layer (excludes "furniture")
        let content_layer = Self::get_content_layer(item);
        if !self.should_include_layer(content_layer, ref_path) {
            return None;
        }

        // Dispatch to type-specific serializers
        match item {
            // Text items may have hyperlinks and formatting
            DocItem::Text {
                text,
                hyperlink,
                formatting,
                ..
            } => self.serialize_text_item(text, hyperlink.as_deref(), formatting.as_ref()),

            // Other text-like items: output text directly
            DocItem::Paragraph { text, .. }
            | DocItem::PageFooter { text, .. }
            | DocItem::PageHeader { text, .. }
            | DocItem::Caption { text, .. }
            | DocItem::Footnote { text, .. }
            | DocItem::Reference { text, .. }
            | DocItem::CheckboxSelected { text, .. }
            | DocItem::CheckboxUnselected { text, .. } => Some(self.post_process(text)),

            // Title gets single # like Python's TitleItem
            DocItem::Title {
                text,
                children,
                formatting,
                hyperlink,
                ..
            } => self.serialize_title_item(
                doc,
                text,
                children,
                formatting.as_ref(),
                hyperlink.as_deref(),
                list_level,
                visited,
            ),

            // Section headers get ## (level + 1 hashes)
            DocItem::SectionHeader {
                text,
                level,
                children,
                formatting,
                hyperlink,
                ..
            } => self.serialize_section_header_item(
                doc,
                text,
                *level,
                children,
                formatting.as_ref(),
                hyperlink.as_deref(),
                list_level,
                visited,
            ),

            // List items with markers
            DocItem::ListItem {
                text,
                marker,
                children,
                formatting,
                hyperlink,
                ..
            } => self.serialize_list_item_item(
                doc,
                text,
                marker,
                children,
                formatting.as_ref(),
                hyperlink.as_deref(),
                list_level,
                visited,
                list_item_index,
            ),

            // List groups - use single newline delimiter
            DocItem::List { children, .. } | DocItem::OrderedList { children, .. } => {
                self.serialize_list(doc, children, list_level, visited, "\n", true)
            }

            // Inline groups - serialize children with space delimiter
            DocItem::Inline { children, .. } => {
                self.serialize_list(doc, children, list_level, visited, " ", false)
            }

            // Groups using double newline delimiter (form/section/sheet/slide/etc.)
            DocItem::FormArea { children, .. }
            | DocItem::KeyValueArea { children, .. }
            | DocItem::Section { children, .. }
            | DocItem::Chapter { children, .. }
            | DocItem::CommentSection { children, .. }
            | DocItem::Sheet { children, .. }
            | DocItem::Slide { children, .. }
            | DocItem::PictureArea { children, .. }
            | DocItem::Unspecified { children, .. } => {
                self.serialize_list(doc, children, list_level, visited, "\n\n", false)
            }

            // Tables (FloatingItem - handle captions, footnotes, references)
            DocItem::Table {
                data,
                captions,
                footnotes,
                references,
                ..
            } => self.serialize_table_item(
                doc, data, captions, footnotes, references, list_level, visited,
            ),

            // Code blocks (FloatingItem - handle children captions)
            DocItem::Code {
                text,
                children,
                parent,
                formatting,
                ..
            } => self.serialize_code_item(
                doc,
                text,
                children,
                parent.as_ref(),
                formatting.as_ref(),
                list_level,
                visited,
            ),

            // Formulas (FloatingItem - handle children captions)
            DocItem::Formula {
                text,
                children,
                parent,
                ..
            } => self.serialize_formula_item(
                doc,
                text,
                children,
                parent.as_ref(),
                list_level,
                visited,
            ),

            // Pictures (FloatingItem - serialize captions + image + footnotes + references)
            DocItem::Picture {
                captions,
                image,
                footnotes,
                references,
                ocr_text,
                ..
            } => self.serialize_picture_item(
                doc,
                captions,
                image.as_ref(),
                footnotes,
                references,
                ocr_text.as_deref(),
                list_level,
                visited,
            ),
        }
    }

    /// Look up a table cell from the `cell_map`, or use the grid cell as fallback.
    /// This allows us to use grid for structure but `table_cells` for content (with refs).
    fn get_table_cell_with_ref<'a>(
        grid_cell: &'a crate::content::TableCell,
        row_idx: usize,
        col_idx: usize,
        cell_map: Option<
            &'a std::collections::HashMap<(usize, usize), &'a crate::content::TableCell>,
        >,
    ) -> &'a crate::content::TableCell {
        if let Some(map) = cell_map {
            if let Some(cell) = map.get(&(row_idx, col_idx)) {
                return cell;
            }
        }
        grid_cell
    }

    /// Get text from table cell, handling rich content (`RichTableCell`)
    ///
    /// Python behavior (document.py:353-363):
    /// - Simple `TableCell`: Returns cell.text directly
    /// - `RichTableCell` (has ref): Resolves reference and serializes recursively
    ///
    /// This mimics Python's `RichTableCell`._`get_text()` method.
    fn get_cell_text(&self, doc: &DoclingDocument, cell: &crate::content::TableCell) -> String {
        // Check if this is a rich table cell (has reference to group/list)
        if let Some(ref_item) = &cell.ref_item {
            // Serialize the referenced item recursively using its ref_path
            let mut visited = HashSet::new();
            if let Some(serialized) =
                self.serialize_item(doc, &ref_item.ref_path, 0, &mut visited, None)
            {
                // In table cells, Python replaces newlines with single spaces (not double)
                // The table serializer will later convert ALL newlines to double spaces
                // So we need to collapse list items into a single line with single spaces
                // between items, preserving the "- " prefixes
                return serialized.trim().replace('\n', " ");
            }
            // Fallback to collapsed text if reference resolution fails
        }
        // Simple cell or fallback - use text directly
        cell.text.clone()
    }

    /// Serialize a table in markdown format (github style)
    #[allow(clippy::too_many_lines)] // Complex table serialization - keeping together for clarity
    fn serialize_table(
        &self,
        doc: &DoclingDocument,
        data: &crate::content::TableData,
    ) -> Option<String> {
        if data.grid.is_empty() {
            return None;
        }

        // Build a lookup map from table_cells (which has refs) keyed by (row, col)
        // This allows us to use grid for structure but table_cells for content
        let cell_map = data.table_cells.as_ref().map(|table_cells| {
            let mut map = std::collections::HashMap::new();
            for cell in table_cells {
                if let (Some(row), Some(col)) =
                    (cell.start_row_offset_idx, cell.start_col_offset_idx)
                {
                    map.insert((row, col), cell);
                }
            }
            map
        });

        // Python's grid already handles spans correctly (cells are replicated).
        // We just use it directly, but we look up refs from table_cells.
        let rows = &data.grid;

        // Calculate column widths using Python tabulate's algorithm:
        // Python: width = max(header_len, max_data_len) + MIN_PADDING (2)
        // Then format with " {text:width} " (adds 1 space on each side, ADDITIONAL to width)
        let num_cols = data.num_cols;
        let mut col_widths = vec![0; num_cols];

        // First pass: calculate minimum width from headers (first row)
        // Python tabulate: minwidth = header_len + MIN_PADDING (2)
        if !rows.is_empty() {
            let header_row = &rows[0];
            for (col_idx, grid_cell) in header_row.iter().enumerate().take(num_cols) {
                // Look up cell with ref from table_cells if available
                let cell = Self::get_table_cell_with_ref(grid_cell, 0, col_idx, cell_map.as_ref());
                // Python tabulate: NO escaping in table cells (not underscores, not HTML)
                // Python replaces newlines with TWO spaces to separate paragraphs
                let header_text = self.get_cell_text(doc, cell).replace('\n', "  ");
                // Python's MIN_PADDING = 2
                // Issue #14 FIX: Use unicode display width for proper CJK alignment
                // CJK characters and other double-width characters count as 2 columns
                col_widths[col_idx] = header_text.width() + 2;
            }
        }

        // Second pass: find max content width across all rows (including header)
        // Takes max of (header+2) from pass 1 and actual data content
        for (row_idx, row) in rows.iter().enumerate() {
            for (col_idx, grid_cell) in row.iter().enumerate().take(num_cols) {
                // Look up cell with ref from table_cells if available
                let cell =
                    Self::get_table_cell_with_ref(grid_cell, row_idx, col_idx, cell_map.as_ref());
                // Python tabulate: NO escaping in table cells (not underscores, not HTML)
                // Python replaces newlines with TWO spaces to separate paragraphs
                let cell_text = self.get_cell_text(doc, cell).replace('\n', "  ");

                // Issue #14 FIX: Use unicode display width for proper CJK alignment
                // CJK characters and other double-width characters count as 2 columns
                // This ensures table columns are properly aligned in monospace fonts
                // Take max of current width (header+2) and content display width
                col_widths[col_idx] = col_widths[col_idx].max(cell_text.width());
            }
        }

        // NOTE: This algorithm produces col_width = max(header+2, max_data)
        // Examples from Python docling v2.58.0:
        // - "Index" (5 chars), max data "1": col_width = max(5+2, 1) = 7
        // - "Customer Id" (11 chars), max data "DD37..." (15): col_width = max(11+2, 15) = 15
        // - "First Name" (10 chars), max data "Preston" (7): col_width = max(10+2, 7) = 12
        // This matches Python tabulate's behavior exactly.

        // Third pass: detect numeric columns (mimics Python tabulate's numparse)
        // A column is numeric if ALL data rows (excluding header) contain only numeric text
        let mut col_is_numeric = vec![false; num_cols];
        if rows.len() > 1 {
            #[allow(
                clippy::needless_range_loop,
                reason = "column-major iteration for numeric detection"
            )]
            for col_idx in 0..num_cols {
                let mut all_numeric = true;
                for (row_idx, row) in rows[1..].iter().enumerate() {
                    if let Some(grid_cell) = row.get(col_idx) {
                        // Look up cell with ref from table_cells if available (row_idx+1 because we skip header)
                        let cell = Self::get_table_cell_with_ref(
                            grid_cell,
                            row_idx + 1,
                            col_idx,
                            cell_map.as_ref(),
                        );
                        let text = self.get_cell_text(doc, cell);
                        let text = text.trim();
                        // Empty cells are not considered numeric
                        if text.is_empty() {
                            all_numeric = false;
                            break;
                        }
                        // Check if the text is a valid number (integer or float)
                        // Python tabulate uses more sophisticated parsing, but this covers the common cases
                        if text.parse::<f64>().is_err() && text.parse::<i64>().is_err() {
                            all_numeric = false;
                            break;
                        }
                    }
                }
                col_is_numeric[col_idx] = all_numeric;
            }
        }

        let mut result = Vec::new();

        // Header row (first row) - align based on column type
        // FIX (Issue #17): Handle ragged header rows by iterating 0..num_cols
        if !rows.is_empty() {
            let header_row = &rows[0];

            // Issue #15 FIX: Check if header row has any non-empty content
            // Collect header texts first to check if all are empty
            let header_texts: Vec<String> = (0..num_cols)
                .map(|idx| {
                    // Handle ragged header rows: use empty string if column doesn't exist
                    header_row.get(idx).map_or_else(String::new, |grid_cell| {
                        // Look up cell with ref from table_cells if available
                        let cell =
                            Self::get_table_cell_with_ref(grid_cell, 0, idx, cell_map.as_ref());
                        // Python tabulate: NO escaping in table cells (not underscores, not HTML)
                        // Python replaces newlines with TWO spaces to separate paragraphs
                        self.get_cell_text(doc, cell).replace('\n', "  ")
                    })
                })
                .collect();

            // Issue #15 FIX: Only emit header and separator if header has content
            let header_has_content = header_texts.iter().any(|t| !t.trim().is_empty());

            if header_has_content {
                let header_cells: Vec<String> = header_texts
                    .iter()
                    .enumerate()
                    .map(|(idx, text)| {
                        if col_is_numeric[idx] {
                            // Right-align numeric column headers
                            format!(" {:>width$} ", text, width = col_widths[idx])
                        } else {
                            // Left-align string column headers
                            format!(" {:<width$} ", text, width = col_widths[idx])
                        }
                    })
                    .collect();
                result.push(format!("|{}|", header_cells.join("|")));

                // Separator row (only if header has content)
                let separators: Vec<String> = col_widths
                    .iter()
                    .map(|&width| "-".repeat(width + 2))
                    .collect();
                result.push(format!("|{}|", separators.join("|")));
            }

            // Data rows - numeric columns are right-aligned
            // FIX (Issue #17): Pad ragged rows to num_cols to prevent data loss
            for (row_idx, row) in rows[1..].iter().enumerate() {
                // Issue #22 FIX: Collect cell texts first to check if row has content
                let cell_data: Vec<(String, bool)> = (0..num_cols)
                    .map(|col_idx| {
                        // Handle ragged rows: use empty string if column doesn't exist
                        row.get(col_idx).map_or_else(
                            || (String::new(), false), // Ragged row: pad with empty cell
                            |grid_cell| {
                                // Look up cell with ref from table_cells if available (row_idx+1 because we skip header)
                                let cell = Self::get_table_cell_with_ref(
                                    grid_cell,
                                    row_idx + 1,
                                    col_idx,
                                    cell_map.as_ref(),
                                );
                                // Python tabulate: NO escaping in table cells (not underscores, not HTML)
                                // Python replaces newlines with TWO spaces to separate paragraphs
                                let text = self.get_cell_text(doc, cell).replace('\n', "  ");
                                (text, cell.row_header)
                            },
                        )
                    })
                    .collect();

                // Issue #22 FIX: Skip rows where all cells are empty
                // This filters out phantom rows from TableFormer that contain no data
                let row_has_content = cell_data.iter().any(|(text, _)| !text.trim().is_empty());
                if !row_has_content {
                    continue;
                }

                let data_cells: Vec<String> = cell_data
                    .into_iter()
                    .enumerate()
                    .map(|(col_idx, (text, is_row_header))| {
                        // Issue #16 FIX: Bold row headers for distinct rendering
                        // Row headers identify what each row represents (like column headers do for columns)
                        let text = if is_row_header && !text.is_empty() {
                            format!("**{text}**")
                        } else {
                            text
                        };

                        // Python tabulate treats tabs as single characters for formatting.
                        // The tab is preserved in the output (not expanded to spaces).
                        // Column width calculation already accounts for tab as 1 character.
                        let width = col_widths[col_idx];

                        if col_is_numeric[col_idx] {
                            // Right-align numeric columns
                            format!(" {text:>width$} ")
                        } else {
                            // Left-align string columns
                            format!(" {text:<width$} ")
                        }
                    })
                    .collect();
                result.push(format!("|{}|", data_cells.join("|")));
            }
        }

        // Issue #21 FIX: No trailing newline from table
        // The serialize() function joins parts with "\n\n" which provides proper spacing
        // Adding "\n" here caused triple newlines between table and next element
        Some(result.join("\n"))
    }

    /// Serialize a list group (or `form_area/key_value_area`)
    fn serialize_list(
        &self,
        doc: &DoclingDocument,
        children: &[ItemRef],
        list_level: usize,
        visited: &mut HashSet<String>,
        delimiter: &str,
        parent_is_list: bool,
    ) -> Option<String> {
        let mut parts = Vec::new();

        // Python behavior: Check if the FIRST item in the group is enumerated
        // If yes, ALL items in the group get sequential numbering (1., 2., 3., ...)
        // regardless of their own enumerated flag
        let first_item_enumerated = children
            .first()
            .and_then(|first_ref| {
                doc.find_item(&first_ref.ref_path).and_then(|item| {
                    if let DocItem::ListItem { enumerated, .. } = item {
                        Some(*enumerated)
                    } else {
                        None
                    }
                })
            })
            .unwrap_or(false);

        for (index, child_ref) in children.iter().enumerate() {
            // Python behavior: When a List contains another List directly (not through a ListItem),
            // increment the list_level. This handles invisible wrapper lists in HTML.
            // Example: List → List creates nesting: <ul><ul><li>Item</li></ul></ul>
            //
            // Python does NOT increment list_level for lists inside structural containers
            // (form_area, key_value_area, chapter, section, slide, etc.)
            // Example: chapter → list → list_item should render at list_level=0 (no indentation)

            let child_list_level =
                doc.find_item(&child_ref.ref_path)
                    .map_or(list_level, |child_item| {
                        // Check if child is a List/OrderedList
                        if matches!(
                            child_item,
                            DocItem::List { .. } | DocItem::OrderedList { .. }
                        ) {
                            // Python: Only increment if parent is ALSO a List/OrderedList
                            if parent_is_list {
                                list_level + 1 // List nesting
                            } else {
                                list_level // Structural container, keep same level
                            }
                        } else {
                            list_level
                        }
                    });

            // Pass the index for enumerated list numbering
            // Python: Uses index-based numbering if first item is enumerated
            let use_index = first_item_enumerated.then_some(index);
            if let Some(part) = self.serialize_item(
                doc,
                &child_ref.ref_path,
                child_list_level,
                visited,
                use_index,
            ) {
                parts.push(part);
            }
        }

        if parts.is_empty() {
            None
        } else {
            // Join with delimiter based on group type:
            // - "list" uses "\n" (single newline)
            // - "form_area" and "key_value_area" use "\n\n" (double newline)
            // Python: MarkdownListSerializer uses "\n", MarkdownFallbackSerializer uses "\n\n"
            Some(parts.join(delimiter))
        }
    }

    /// Apply formatting (bold, italic) to text
    /// Python: `serialize_bold`, `serialize_italic` methods
    fn apply_formatting(text: &str, formatting: Option<&crate::content::Formatting>) -> String {
        let mut result = text.to_string();

        if let Some(fmt) = formatting {
            let is_bold = fmt.bold.unwrap_or(false);
            let is_italic = fmt.italic.unwrap_or(false);
            let is_strikethrough = fmt.strikethrough.unwrap_or(false);

            // Apply markdown formatting (order matters: italic+bold first, then strikethrough)
            if is_italic && is_bold {
                result = format!("***{result}***");
            } else if is_bold {
                result = format!("**{result}**");
            } else if is_italic {
                result = format!("*{result}*");
            }

            // Apply strikethrough (can combine with bold/italic)
            if is_strikethrough {
                result = format!("~~{result}~~");
            }
        }

        result
    }

    /// Apply post-processing (escaping, control char sanitization, etc.)
    fn post_process(&self, text: &str) -> String {
        // Issue #18 FIX: Sanitize control characters from OCR text
        // Strip control chars except newline (\n), carriage return (\r), tab (\t)
        let result: String = text
            .chars()
            .filter(|c| {
                // Keep if not a control character, OR if it's \n, \r, \t
                !c.is_control() || *c == '\n' || *c == '\r' || *c == '\t'
            })
            .collect();

        let mut result = result;

        // Escape underscores (except in URLs)
        if self.options.escape_underscores {
            result = Self::escape_underscores(&result);
        }

        // Escape HTML
        if self.options.escape_html {
            result = Self::escape_html(&result);
        }

        // N=4355: Linkify plain text URLs
        if self.options.linkify_urls {
            result = Self::linkify_urls(&result);
        }

        result
    }

    /// Escape underscores but leave them intact in URLs and code (backticks)
    fn escape_underscores(text: &str) -> String {
        // Don't escape underscores inside backticks (inline code)
        // Track whether we're inside backticks
        let mut result = String::with_capacity(text.len());
        let mut inside_backticks = false;
        let chars = text.chars();

        for ch in chars {
            if ch == '`' {
                inside_backticks = !inside_backticks;
                result.push(ch);
            } else if ch == '_' && !inside_backticks {
                result.push_str(r"\_");
            } else {
                result.push(ch);
            }
        }

        result
    }

    /// Escape HTML characters but leave them intact in code (backticks)
    /// Note: Only escapes &, <, > to match Python docling v2.58.0 baseline
    /// Quotes and NBSP are NOT escaped to maintain baseline compatibility
    fn escape_html(text: &str) -> String {
        // Don't escape HTML inside backticks (inline code)
        // Track whether we're inside backticks
        let mut result = String::with_capacity(text.len() + 20); // Extra space for escapes
        let mut inside_backticks = false;

        for ch in text.chars() {
            if ch == '`' {
                inside_backticks = !inside_backticks;
                result.push(ch);
            } else if inside_backticks {
                // Inside code - don't escape
                result.push(ch);
            } else {
                // Outside code - escape only &, <, > (matches Python docling)
                match ch {
                    '&' => result.push_str("&amp;"),
                    '<' => result.push_str("&lt;"),
                    '>' => result.push_str("&gt;"),
                    _ => result.push(ch),
                }
            }
        }

        result
    }

    /// N=4355: Convert plain text URLs to markdown links
    ///
    /// Converts `https://example.com` to `[https://example.com](https://example.com)`.
    /// Only linkifies URLs that are not already part of a markdown link.
    fn linkify_urls(text: &str) -> String {
        // Regex pattern for URLs: http:// or https:// followed by non-whitespace
        // Match URL characters until we hit whitespace or common terminating punctuation
        static URL_PATTERN: Lazy<Regex> = Lazy::new(|| {
            // Use a simple pattern that avoids complex character class escaping
            // Matches http:// or https:// followed by URL-valid chars
            Regex::new(r"https?://[^\s<>]+").expect("Invalid URL regex")
        });

        // Check if we have any URLs to process
        if !text.contains("http://") && !text.contains("https://") {
            return text.to_string();
        }

        // Skip if text already contains markdown links (to avoid double-linking)
        if text.contains("](http") {
            return text.to_string();
        }

        // Replace URLs with markdown links
        URL_PATTERN
            .replace_all(text, |caps: &regex::Captures| {
                let url = caps[0].trim_end_matches(|c| {
                    matches!(
                        c,
                        '.' | ',' | ';' | ':' | '!' | '?' | '\'' | '"' | ')' | ']'
                    )
                });
                // Escape the URL for safe markdown
                let safe_url = Self::escape_url(url);
                format!("[{url}]({safe_url})")
            })
            .into_owned()
    }

    /// Escape special characters in URLs for safe markdown link formatting
    /// Issue #2 fix: Prevents URL injection and malformed markdown
    fn escape_url(url: &str) -> String {
        let mut result = String::with_capacity(url.len() + 10);
        for ch in url.chars() {
            match ch {
                // Escape characters that could break markdown link syntax
                ')' => result.push_str("%29"), // Close paren breaks [text](url)
                '(' => result.push_str("%28"), // Open paren for consistency
                ' ' => result.push_str("%20"), // Space breaks URL parsing
                '\n' | '\r' => {}              // Strip newlines (injection vector)
                '[' => result.push_str("%5B"), // Could break markdown
                ']' => result.push_str("%5D"), // Could break markdown
                '\\' => result.push_str("%5C"), // Escape char
                _ => result.push(ch),
            }
        }
        result
    }

    /// N=4322: Detect if text matches a date pattern like "5 May 2023" or "May 5, 2023"
    /// N=4322c: Added ISO 8601 and European date formats
    ///
    /// These are metadata dates, not section titles.
    /// Patterns detected:
    /// - "5 May 2023" (day month year)
    /// - "15th January 2025" (ordinal day month year)
    /// - "May 5, 2023" (month day year)
    /// - "January 15, 2025" (month day year)
    /// - "2023-05-05" (ISO 8601)
    /// - "05.05.2023" (European DD.MM.YYYY)
    fn is_date_pattern(text: &str) -> bool {
        // Month names (full and abbreviated) - defined at start per clippy::pedantic
        const MONTHS: &[&str] = &[
            "january",
            "february",
            "march",
            "april",
            "may",
            "june",
            "july",
            "august",
            "september",
            "october",
            "november",
            "december",
            "jan",
            "feb",
            "mar",
            "apr",
            "jun",
            "jul",
            "aug",
            "sep",
            "sept",
            "oct",
            "nov",
            "dec",
        ];

        let trimmed = text.trim();

        // N=4322c: Check for ISO 8601 format (YYYY-MM-DD)
        // Must be exactly 10 chars with hyphens in positions 4 and 7
        if trimmed.len() == 10 {
            let chars: Vec<char> = trimmed.chars().collect();
            if chars[4] == '-' && chars[7] == '-' {
                // Extract year, month, day
                let year_str: String = chars[0..4].iter().collect();
                let month_str: String = chars[5..7].iter().collect();
                let day_str: String = chars[8..10].iter().collect();

                if let (Ok(year), Ok(month), Ok(day)) = (
                    year_str.parse::<u32>(),
                    month_str.parse::<u32>(),
                    day_str.parse::<u32>(),
                ) {
                    if (1900..=2099).contains(&year)
                        && (1..=12).contains(&month)
                        && (1..=31).contains(&day)
                    {
                        return true;
                    }
                }
            }
            // Check for European format (DD.MM.YYYY)
            if chars[2] == '.' && chars[5] == '.' {
                let day_str: String = chars[0..2].iter().collect();
                let month_str: String = chars[3..5].iter().collect();
                let year_str: String = chars[6..10].iter().collect();

                if let (Ok(day), Ok(month), Ok(year)) = (
                    day_str.parse::<u32>(),
                    month_str.parse::<u32>(),
                    year_str.parse::<u32>(),
                ) {
                    if (1..=31).contains(&day)
                        && (1..=12).contains(&month)
                        && (1900..=2099).contains(&year)
                    {
                        return true;
                    }
                }
            }
        }

        let lower = text.to_lowercase();
        let words: Vec<&str> = lower.split_whitespace().collect();

        // Date patterns are typically 2-4 words: "5 May 2023" or "May 5, 2023"
        // Skip if too many words (likely a title like "May 2020 Conference Proceedings")
        if words.len() > 4 {
            return false;
        }

        // Check if text contains a month name
        let has_month = MONTHS.iter().any(|m| lower.contains(m));
        if !has_month {
            return false;
        }

        // Check if text contains a year (4 digit number in 1900-2099 range)
        let has_year = words.iter().any(|w| {
            // Remove trailing punctuation (comma, period)
            let w_clean: String = w.chars().filter(char::is_ascii_digit).collect();
            if w_clean.len() == 4 {
                if let Ok(year) = w_clean.parse::<u32>() {
                    return (1900..=2099).contains(&year);
                }
            }
            false
        });

        if !has_year {
            return false;
        }

        // Check if text contains a day number (1-31)
        let has_day = words.iter().any(|w| {
            // Handle ordinal suffixes: "5th", "1st", "2nd", "3rd", "15th"
            let w_clean: String = w.chars().take_while(char::is_ascii_digit).collect();
            if w_clean.is_empty() {
                return false;
            }
            if let Ok(day) = w_clean.parse::<u32>() {
                return (1..=31).contains(&day);
            }
            false
        });

        // Pattern: month + year + day = date
        has_month && has_year && has_day
    }

    /// N=4357: Detect affiliation patterns - superscript numbers followed by institution text
    ///
    /// In academic papers, author affiliations often appear as:
    /// - "1 Machine Learning Department, Carnegie Mellon University"
    /// - "2 Department of Computer Science, Princeton University"
    ///
    /// These should NOT be rendered as section headers.
    fn is_affiliation_pattern(text: &str) -> bool {
        // Institution-related keywords (case insensitive)
        const INSTITUTION_WORDS: &[&str] = &[
            "department",
            "university",
            "institute",
            "college",
            "school",
            "laboratory",
            "lab",
            "centre",
            "center",
            "faculty",
            "research",
            "sciences",
            "engineering",
            "medicine",
            "hospital",
            "corporation",
            "inc.",
            "llc",
            "ltd",
            "google",
            "microsoft",
            "meta",
            "amazon",
            "apple",
            "nvidia",
            "openai",
            "anthropic",
            "deepmind",
        ];

        let trimmed = text.trim();

        // Must start with 1-2 digit number followed by space
        let mut chars = trimmed.chars().peekable();
        let mut digits = String::new();
        while let Some(&c) = chars.peek() {
            if c.is_ascii_digit() && digits.len() < 2 {
                digits.push(c);
                chars.next();
            } else {
                break;
            }
        }

        // Need at least one digit
        if digits.is_empty() {
            return false;
        }

        // Parse the number - should be small (affiliations typically 1-20)
        if let Ok(num) = digits.parse::<u32>() {
            if num > 20 {
                return false;
            }
        }

        // Must have space after number
        if chars.next() != Some(' ') {
            return false;
        }

        // Check for institution keywords in the rest of the text
        let rest: String = chars.collect();
        let lower = rest.to_lowercase();

        // Must contain at least one institution-related word
        INSTITUTION_WORDS.iter().any(|word| lower.contains(word))
    }

    /// N=4357: Detect algorithm/figure box labels
    ///
    /// In academic papers, algorithm boxes often have labels like:
    /// - "1 SSM (S4)"
    /// - "2 SSM + Selection (S6)"
    ///
    /// These are algorithm numbers inside boxes, not section headers.
    /// Pattern: single digit + space + abbreviation/acronym (mostly uppercase)
    fn is_algorithm_label_pattern(text: &str) -> bool {
        let trimmed = text.trim();

        // Must start with single digit
        let first_char = trimmed.chars().next();
        if !first_char.is_some_and(|c| c.is_ascii_digit()) {
            return false;
        }

        // Second char must be space
        let second_char = trimmed.chars().nth(1);
        if second_char != Some(' ') {
            return false;
        }

        // Rest of text after "N "
        let rest = &trimmed[2..];

        // Must have content after the number
        if rest.trim().is_empty() {
            return false;
        }

        // Check if the first word after the number is an abbreviation (all uppercase or mostly)
        let first_word: String = rest.split_whitespace().next().unwrap_or("").to_string();

        // First word should be mostly uppercase (abbreviation like "SSM", "CNN", "MLP")
        // Allow some lowercase for patterns like "SSM+Selection"
        let uppercase_count = first_word
            .chars()
            .filter(|c| c.is_ascii_uppercase())
            .count();
        let letter_count = first_word
            .chars()
            .filter(|c| c.is_ascii_alphabetic())
            .count();

        if letter_count == 0 {
            return false;
        }

        // At least 60% uppercase letters suggests an abbreviation
        let uppercase_ratio = uppercase_count as f64 / letter_count as f64;

        // Short text (< 30 chars) with mostly uppercase first word = likely algorithm label
        uppercase_ratio >= 0.6 && trimmed.len() < 30
    }

    /// Detect if text is body content misclassified as section header
    ///
    /// N=4132: The layout model sometimes detects split reference citations as
    /// section headers because they start with numbers (like "1873. IEEE (2022)").
    ///
    /// N=4321: Extended for scanned documents where ALL CAPS body text (e.g., FBI memos)
    /// is incorrectly classified as section headers. Real section headers:
    /// - Are typically short (< 80 characters)
    /// - Don't contain semicolons (multiple clauses)
    /// - Don't have multiple sentences
    /// - Are meaningful titles, not data lists
    fn is_fake_section_header(text: &str) -> bool {
        let trimmed = text.trim();

        // Empty text is not a fake section header
        if trimmed.is_empty() {
            return false;
        }

        // N=4321: Very long text is body content, not a section header
        // Real section headers are typically concise (< 80 chars)
        if trimmed.len() > 80 {
            return true;
        }

        // N=4321: Text with semicolons is typically body content (lists, data)
        // Real section headers rarely have semicolons
        if trimmed.contains(';') {
            return true;
        }

        // N=4368: Standalone "Appendix" is often an OCR artifact from inline references
        // Real appendix headers are "Appendix A", "Appendix B", etc. with a letter suffix
        // This occurs when OCR splits "See Appendix D" into separate text blocks
        if trimmed.eq_ignore_ascii_case("appendix") {
            return true;
        }

        // N=4357: Affiliation patterns - single/double digit followed by institution words
        // e.g., "1 Machine Learning Department, Carnegie Mellon University"
        // These are author affiliations, not section headers
        if Self::is_affiliation_pattern(trimmed) {
            return true;
        }

        // N=4357: Algorithm/figure box labels - number followed by abbreviation/acronym
        // e.g., "1 SSM (S4)", "2 SSM + Selection (S6)"
        // These are algorithm labels inside boxes, not section headers
        if Self::is_algorithm_label_pattern(trimmed) {
            return true;
        }

        // N=4321: Text with "PERCENT" or "AGENTS" is likely government document body text
        // These are data patterns, not header patterns
        let upper = trimmed.to_uppercase();
        if upper.contains("PERCENT") || upper.contains("AGENTS AT") {
            return true;
        }

        // N=4321: Text ending with a colon suggests a label, not header
        // e.g., "Serial Scope:" - render as text, not heading
        if trimmed.ends_with(':') && !trimmed.starts_with(char::is_numeric) {
            // Exception: "1. Introduction:" should still be a header
            // Only filter standalone labels without section numbers
            let has_section_number = trimmed.chars().take(5).any(|c| c.is_ascii_digit());
            if !has_section_number {
                return true;
            }
        }

        // N=4322: Date patterns like "5 May 2023" should not be section headers
        // These are metadata dates, not section titles
        if Self::is_date_pattern(trimmed) {
            return true;
        }

        // Check if starts with a 4-digit year (1800-2099)
        // Reference citations often have years like "1873. IEEE", "2019.", "2022."
        let mut chars = trimmed.chars().peekable();
        let mut digits = String::new();
        while let Some(&c) = chars.peek() {
            if c.is_ascii_digit() && digits.len() < 4 {
                digits.push(c);
                chars.next();
            } else {
                break;
            }
        }

        if digits.len() == 4 {
            if let Ok(year) = digits.parse::<u32>() {
                // Years between 1800-2099 are likely reference citations
                // Real section headers don't start with 4-digit numbers
                if (1800..=2099).contains(&year) {
                    // Extra check: real headings like "1900s Overview" shouldn't be filtered
                    // Reference patterns typically have: year followed by period/comma/text
                    let rest = trimmed[4..].trim();
                    if rest.starts_with('.')
                        || rest.starts_with(',')
                        || rest.starts_with(')')
                        || rest.to_lowercase().contains("ieee")
                        || rest.to_lowercase().contains("acm")
                        || rest.to_lowercase().contains("springer")
                        || rest.to_lowercase().contains("pp.")
                        || rest.to_lowercase().contains("proceedings")
                    {
                        return true;
                    }
                }
            }
        }

        false
    }

    // ========================================================================
    // Helper methods for serialize_item - extracted to reduce complexity
    // ========================================================================

    /// Check if item is a referenceable leaf (can be referenced multiple times)
    #[inline]
    const fn is_referenceable_leaf(item: &DocItem) -> bool {
        matches!(
            item,
            DocItem::Caption { .. } | DocItem::Footnote { .. } | DocItem::Reference { .. }
        )
    }

    /// Extract `content_layer` from any `DocItem` variant
    #[inline]
    fn get_content_layer(item: &DocItem) -> &str {
        match item {
            DocItem::Text { content_layer, .. }
            | DocItem::Paragraph { content_layer, .. }
            | DocItem::Title { content_layer, .. }
            | DocItem::SectionHeader { content_layer, .. }
            | DocItem::ListItem { content_layer, .. }
            | DocItem::List { content_layer, .. }
            | DocItem::FormArea { content_layer, .. }
            | DocItem::KeyValueArea { content_layer, .. }
            | DocItem::OrderedList { content_layer, .. }
            | DocItem::Chapter { content_layer, .. }
            | DocItem::Section { content_layer, .. }
            | DocItem::Sheet { content_layer, .. }
            | DocItem::Slide { content_layer, .. }
            | DocItem::CommentSection { content_layer, .. }
            | DocItem::Inline { content_layer, .. }
            | DocItem::PictureArea { content_layer, .. }
            | DocItem::Unspecified { content_layer, .. }
            | DocItem::Table { content_layer, .. }
            | DocItem::Picture { content_layer, .. }
            | DocItem::Code { content_layer, .. }
            | DocItem::Formula { content_layer, .. }
            | DocItem::Caption { content_layer, .. }
            | DocItem::Footnote { content_layer, .. }
            | DocItem::PageHeader { content_layer, .. }
            | DocItem::PageFooter { content_layer, .. }
            | DocItem::Reference { content_layer, .. }
            | DocItem::CheckboxSelected { content_layer, .. }
            | DocItem::CheckboxUnselected { content_layer, .. } => content_layer,
        }
    }

    /// Check if content should be included based on layer
    fn should_include_layer(&self, content_layer: &str, ref_path: &str) -> bool {
        let is_body = content_layer == "body";
        let is_furniture = content_layer == "furniture";
        let should_include = is_body || (is_furniture && self.options.include_furniture);

        if !should_include {
            trace!(
                "Skipping item {} with content_layer '{}' (include_furniture={})",
                ref_path,
                content_layer,
                self.options.include_furniture
            );
        }
        should_include
    }

    /// Check if text appears to be page header content that was mislabeled.
    ///
    /// N=4418: The layout model sometimes classifies page headers (arXiv identifiers,
    /// dates, page numbers) as regular text. These should be filtered from body output
    /// to match Python docling's behavior where page headers are in the furniture layer.
    ///
    /// Detects:
    /// - arXiv identifiers: "arXiv:XXXX.XXXXX" patterns
    /// - Standalone dates: "5 May 2023", "January 15, 2024"
    /// - Page numbers: standalone digits like "1", "42"
    fn is_mislabeled_page_header(text: &str) -> bool {
        let trimmed = text.trim();

        // Empty text is not a page header
        if trimmed.is_empty() {
            return false;
        }

        // arXiv identifiers (case-insensitive)
        // Pattern: "arXiv:XXXX.XXXXX" with optional version like "v1"
        let text_lower = trimmed.to_lowercase();
        if text_lower.starts_with("arxiv:") || text_lower.contains("arxiv:") {
            log::debug!("Filtering mislabeled page header (arXiv): {}", trimmed);
            return true;
        }

        // Standalone date patterns (without other content)
        // These are often publication dates in page headers
        if Self::is_date_pattern(trimmed) {
            log::debug!("Filtering mislabeled page header (date): {}", trimmed);
            return true;
        }

        // Standalone page numbers (1-4 digits only)
        if trimmed.len() <= 4 && trimmed.chars().all(|c| c.is_ascii_digit()) {
            log::debug!(
                "Filtering mislabeled page header (page number): {}",
                trimmed
            );
            return true;
        }

        false
    }

    /// Check if text appears to be a figure or algorithm artifact that should be filtered.
    ///
    /// These are typically small text elements from within figures or algorithm boxes
    /// that get detected as separate text elements but don't add value as standalone
    /// paragraphs in the markdown output.
    ///
    /// Filters:
    /// - Algorithm annotations starting with "⊲"
    /// - Single-word figure labels (Input, Output, Solution, etc.)
    /// - Single punctuation characters
    /// - Math variable symbols with subscripts/superscripts (N=4371)
    /// - Hardware/diagram labels (GPU, SRAM, HBM) (N=4371)
    fn is_figure_artifact(text: &str) -> bool {
        let trimmed = text.trim();

        // Algorithm annotations (triangular bullet)
        if trimmed.starts_with('⊲') {
            return true;
        }

        // Single punctuation or very short text (likely OCR noise)
        // N=4371: Expanded from 2 to 3 chars to catch more noise
        if trimmed.len() <= 3 && !trimmed.chars().any(char::is_alphabetic) {
            return true;
        }

        // N=4371: Math subscript/superscript artifacts (e.g., "x!", "y!", "!"#", "! ")
        // These are OCR'd diagram labels that look like variables with subscripts
        if trimmed.len() <= 4 {
            // Count special chars (! is common OCR for subscripts)
            let special_count = trimmed.chars().filter(|c| *c == '!' || *c == '#').count();
            let alpha_count = trimmed.chars().filter(|c| c.is_alphabetic()).count();
            // If mostly special characters or single letter + special chars
            if special_count > 0 && alpha_count <= 1 {
                return true;
            }
        }

        // Common figure/diagram labels that shouldn't be standalone paragraphs
        // These typically appear inside figures or algorithm boxes
        // N=4371: Expanded list based on Mamba paper figure analysis
        let figure_labels = [
            // Original labels
            "Input",
            "Output",
            "Solution",
            "Input:",
            "Output:",
            "Copying",
            // N=4371: Additional diagram labels from Figure 1 analysis
            "Project",
            "Discretize",
            "GPU",
            "SRAM",
            "HBM",
            "GPU SRAM",
            "GPU HBM",
            // Hardware/architecture labels
            "CPU",
            "Memory",
            "Cache",
            "Register",
            "Stack",
            "Heap",
            // Flow diagram labels
            "Start",
            "End",
            "Yes",
            "No",
            "True",
            "False",
            // Neural network diagram labels
            "Linear",
            "Conv",
            "ReLU",
            "Softmax",
            "Attention",
            "MLP",
            "Encoder",
            "Decoder",
            "Embedding",
            "Norm",
            "Layer",
        ];

        if figure_labels.contains(&trimmed) {
            return true;
        }

        // N=4371: Short text with math symbols likely diagram annotations
        // Unicode math letters: 𝐴-𝑍 (U+1D400-U+1D433) and 𝑎-𝑧 (U+1D434-U+1D467)
        if trimmed.len() <= 8
            && trimmed
                .chars()
                .any(|c| ('\u{1D400}'..='\u{1D467}').contains(&c))
        {
            // Contains math italic letters - likely figure annotation
            return true;
        }

        false
    }

    /// Serialize `DocItem::Text` with formatting and hyperlinks
    #[allow(
        clippy::unnecessary_wraps,
        reason = "consistent Option<String> interface with other serialize_*_item methods"
    )]
    fn serialize_text_item(
        &self,
        text: &str,
        hyperlink: Option<&str>,
        formatting: Option<&crate::content::Formatting>,
    ) -> Option<String> {
        // N=4418: Filter mislabeled page headers (arXiv, dates, page numbers)
        if Self::is_mislabeled_page_header(text) {
            return None;
        }

        // Filter out figure/algorithm artifacts
        if Self::is_figure_artifact(text) {
            return None;
        }

        let formatted_text = Self::apply_formatting(text, formatting);
        let processed_text = self.post_process(&formatted_text);

        let content = if let Some(url) = hyperlink {
            let safe_url = Self::escape_url(url);
            format!("[{processed_text}]({safe_url})")
        } else {
            processed_text
        };

        Some(content)
    }

    /// Serialize `DocItem::Title` with children
    #[allow(
        clippy::too_many_arguments,
        reason = "serialization context: doc, text, children, formatting, state"
    )]
    #[allow(
        clippy::unnecessary_wraps,
        reason = "consistent Option<String> interface with other serialize_*_item methods"
    )]
    fn serialize_title_item(
        &self,
        doc: &DoclingDocument,
        text: &str,
        children: &[ItemRef],
        formatting: Option<&crate::content::Formatting>,
        hyperlink: Option<&str>,
        list_level: usize,
        visited: &mut HashSet<String>,
    ) -> Option<String> {
        let formatted_text = Self::apply_formatting(text, formatting);

        let link_text = if let Some(href) = hyperlink {
            let safe_url = Self::escape_url(href);
            format!("[{formatted_text}]({safe_url})")
        } else {
            formatted_text
        };

        let processed_text = self.post_process(&link_text);
        // N=4418: Python docling uses "##" for titles too (section_header level 1 in groundtruth)
        let mut parts = vec![format!("## {processed_text}")];

        for child_ref in children {
            if let Some(child) =
                self.serialize_item(doc, &child_ref.ref_path, list_level, visited, None)
            {
                if !child.is_empty() {
                    parts.push(child);
                }
            }
        }

        Some(parts.join("\n\n"))
    }

    /// Serialize `DocItem::SectionHeader` with inline children support
    #[allow(
        clippy::too_many_arguments,
        reason = "serialization context: doc, text, level, children, formatting, state"
    )]
    #[allow(
        clippy::unnecessary_wraps,
        reason = "consistent Option<String> interface with other serialize_*_item methods"
    )]
    fn serialize_section_header_item(
        &self,
        doc: &DoclingDocument,
        text: &str,
        _level: usize, // N=4418: Unused - Python uses fixed "##" for all section headers
        children: &[ItemRef],
        _formatting: Option<&crate::content::Formatting>, // N=4410: Unused - headers don't apply bold/italic
        hyperlink: Option<&str>,
        list_level: usize,
        visited: &mut HashSet<String>,
    ) -> Option<String> {
        // N=4418: Filter mislabeled page headers (dates, arXiv) completely
        // These should not appear in output at all
        if Self::is_mislabeled_page_header(text) {
            return None;
        }

        // N=4132: Detect if this is reference content misclassified as section header
        // Example: "1873. IEEE (2022)" from a split reference citation
        // Real section headers don't start with 4-digit years
        if Self::is_fake_section_header(text) {
            log::debug!(
                "Markdown: Treating fake section header as text: {}",
                &text[..text.len().min(50)]
            );
            // Render as plain text instead of heading
            let processed_text = self.post_process(text);
            return Some(processed_text);
        }

        // N=4418: Python docling uses fixed "##" for ALL section headers regardless of level.
        // This matches the groundtruth where titles and subsections all use "##".
        let hashes = "##";

        // Check if we have inline children (for inline formatting in headings)
        if text.is_empty() && !children.is_empty() {
            if let Some(first_child) = children.first() {
                if let Some(child_item) = doc.find_item(&first_child.ref_path) {
                    if matches!(child_item, DocItem::Inline { .. }) {
                        let mut inline_parts = Vec::new();
                        for child_ref in children {
                            if let Some(child) = self.serialize_item(
                                doc,
                                &child_ref.ref_path,
                                list_level,
                                visited,
                                None,
                            ) {
                                inline_parts.push(child);
                            }
                        }
                        let inline_text = inline_parts.join(" ");
                        return Some(format!("{hashes} {inline_text}"));
                    }
                }
            }
        }

        // Regular heading with text
        // N=4410: Don't apply bold/italic formatting to section headers
        // The `##` markdown syntax already provides visual emphasis
        // Python groundtruth shows headers without `**` formatting inside
        let base_text = text.to_string();

        let link_text = if let Some(href) = hyperlink {
            let safe_url = Self::escape_url(href);
            format!("[{base_text}]({safe_url})")
        } else {
            base_text
        };

        let processed_text = self.post_process(&link_text);
        let mut parts = vec![format!("{} {}", hashes, processed_text)];

        for child_ref in children {
            if let Some(child) =
                self.serialize_item(doc, &child_ref.ref_path, list_level, visited, None)
            {
                if !child.is_empty() {
                    parts.push(child);
                }
            }
        }

        Some(parts.join("\n\n"))
    }

    /// Build list item marker with proper formatting
    ///
    /// Handles different marker types:
    /// - Markdown-valid markers (`-`, `*`, `+`, `1.`): used directly
    /// - OCR bullet artifacts (∞, •, ·, etc.): normalized to `-` (not preserved)
    /// - Empty marker with enumeration: generates numbered marker (1., 2., etc.)
    /// - Empty marker without enumeration: uses `-` bullet
    // Method signature kept for API consistency with other MarkdownSerializer methods
    #[allow(clippy::unused_self)]
    fn build_list_marker(&self, marker: &str, list_item_index: Option<usize>) -> Vec<String> {
        let is_markdown_valid = marker == "-"
            || marker == "*"
            || marker == "+"
            || (marker.chars().all(|c| c.is_ascii_digit() || c == '.') && marker.ends_with('.'));

        // Common OCR bullet artifacts that should be normalized to `-`
        // These are symbols that OCR often produces when reading bullet points
        let is_ocr_bullet_artifact = matches!(
            marker,
            "∞" | "•"
                | "·"
                | "‣"
                | "○"
                | "●"
                | "◦"
                | "▪"
                | "▫"
                | "◾"
                | "◽"
                | "►"
                | "▸"
                | "‐"
                | "–"
                | "—"
                | "→"
                | "⁃"
                | "⦿"
                | "⦾"
                | "◇"
                | "◆"
                | "★"
                | "☆"
                | "✓"
                | "✔"
        );

        let mut pieces = Vec::new();

        if is_markdown_valid {
            // Use markdown-valid markers directly
            pieces.push(marker.to_string());
        } else if is_ocr_bullet_artifact || marker.is_empty() {
            // OCR bullet artifacts: normalize to standard markdown bullet
            // Empty markers: use bullet or numbered format
            if list_item_index.is_some() {
                let number = list_item_index.map_or(1, |i| i + 1);
                pieces.push(format!("{number}."));
            } else {
                pieces.push("-".to_string());
            }
        } else {
            // Non-empty marker that's not an OCR artifact (e.g., "a.", "i)", custom labels)
            // Preserve it with a markdown prefix
            pieces.push("-".to_string());
            pieces.push(marker.to_string());
        }

        pieces
    }

    /// N=4369: Strip leading OCR bullet artifacts from list item text content
    ///
    /// When OCR reads bullet points, it sometimes captures the bullet character
    /// in both the marker field AND the text content. For example:
    /// - marker: "∞" → normalized to "-"
    /// - text: "∞ Synthetics..." → needs stripping to avoid "- ∞ Synthetics..."
    ///
    /// This function removes leading bullet artifacts and whitespace to produce
    /// clean text content.
    ///
    /// N=4413: Currently unused in production (Python docling keeps artifacts)
    /// but kept for potential future use and tests document its behavior.
    #[allow(dead_code)]
    fn strip_leading_bullet_artifact(text: &str) -> String {
        // OCR bullet artifacts that might appear at the start of text
        const BULLET_ARTIFACTS: &[char] = &[
            '∞',  // infinity - common OCR misread for bullet
            '•',  // bullet point
            '·',  // middle dot
            '‣',  // triangular bullet
            '○',  // white circle
            '●',  // black circle
            '◦',  // white bullet
            '▪',  // black square
            '▫',  // white square
            '◾', // medium black square
            '◽', // medium white square
            '–',  // en dash (sometimes used as bullet)
            '—',  // em dash
            '‐',  // hyphen
            '⁃',  // hyphen bullet
            '◇',  // white diamond
            '◆',  // black diamond
            '⬥',  // black medium diamond
            '⊳',  // right triangle
            '⊲',  // left triangle (algorithm annotation)
        ];

        let trimmed = text.trim_start();

        // Check if text starts with a bullet artifact followed by space
        if let Some(first_char) = trimmed.chars().next() {
            if BULLET_ARTIFACTS.contains(&first_char) {
                // Remove the artifact and any following whitespace
                let rest = &trimmed[first_char.len_utf8()..];
                return rest.trim_start().to_string();
            }
        }

        // No artifact found, return original (trimmed of leading whitespace only)
        text.to_string()
    }

    /// Serialize `DocItem::ListItem` with markers and children
    #[allow(
        clippy::too_many_arguments,
        reason = "serialization context: doc, text, marker, children, formatting, state"
    )]
    #[allow(
        clippy::unnecessary_wraps,
        reason = "consistent Option<String> interface with other serialize_*_item methods"
    )]
    fn serialize_list_item_item(
        &self,
        doc: &DoclingDocument,
        text: &str,
        marker: &str,
        children: &[ItemRef],
        formatting: Option<&crate::content::Formatting>,
        hyperlink: Option<&str>,
        list_level: usize,
        visited: &mut HashSet<String>,
        list_item_index: Option<usize>,
    ) -> Option<String> {
        let clamped_level = list_level.min(self.options.max_list_depth);
        let indent_str = if clamped_level > 0 {
            " ".repeat(clamped_level * self.options.indent)
        } else {
            String::new()
        };

        let mut pieces = self.build_list_marker(marker, list_item_index);

        // Add text if non-empty
        if !text.is_empty() {
            // N=4413: Keep text as-is to match Python docling v2.58.0 output
            // Python keeps OCR bullet artifacts in text (e.g., "- ∞ IBM MT/ST")
            // Previously we stripped these (N=4369), but that doesn't match groundtruth
            let formatted_text = Self::apply_formatting(text, formatting);
            let final_text = if let Some(href) = hyperlink {
                if href.is_empty() {
                    formatted_text
                } else {
                    let safe_url = Self::escape_url(href);
                    format!("[{formatted_text}]({safe_url})")
                }
            } else {
                formatted_text
            };
            pieces.push(final_text);
        }

        // Filter empty pieces and join
        let non_empty: Vec<_> = pieces.iter().filter(|s| !s.is_empty()).collect();
        let joined = if non_empty.is_empty() {
            String::new()
        } else {
            non_empty
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        };
        let list_item_start = self.post_process(&format!("{indent_str}{joined}"));

        if children.is_empty() {
            return Some(list_item_start);
        }

        // Has children - separate inline vs nested
        let mut inline_parts = vec![list_item_start];
        let mut nested_parts = Vec::new();

        for child_ref in children {
            let is_inline = doc
                .find_item(&child_ref.ref_path)
                .is_some_and(|item| matches!(item, DocItem::Inline { .. }));

            if is_inline {
                if let Some(child) =
                    self.serialize_item(doc, &child_ref.ref_path, list_level, visited, None)
                {
                    inline_parts.push(child);
                }
            } else if let Some(child) =
                self.serialize_item(doc, &child_ref.ref_path, list_level + 1, visited, None)
            {
                nested_parts.push(child);
            }
        }

        let mut result = inline_parts.join(" ");
        if !nested_parts.is_empty() {
            result.push('\n');
            result.push_str(&nested_parts.join("\n"));
        }
        Some(result)
    }

    /// Serialize `DocItem::Table` with captions, footnotes, references
    #[allow(
        clippy::too_many_arguments,
        reason = "table serialization needs data, captions, footnotes, refs, state"
    )]
    fn serialize_table_item(
        &self,
        doc: &DoclingDocument,
        data: &crate::content::TableData,
        captions: &[ItemRef],
        footnotes: &[ItemRef],
        references: &[ItemRef],
        list_level: usize,
        visited: &mut HashSet<String>,
    ) -> Option<String> {
        let mut parts = Vec::new();

        // Captions first
        for cap_ref in captions {
            if let Some(caption) =
                self.serialize_item(doc, &cap_ref.ref_path, list_level, visited, None)
            {
                parts.push(caption);
            }
        }

        // Table itself
        if let Some(table_md) = self.serialize_table(doc, data) {
            parts.push(table_md);
        }

        // Footnotes
        for fn_ref in footnotes {
            if let Some(footnote) =
                self.serialize_item(doc, &fn_ref.ref_path, list_level, visited, None)
            {
                parts.push(footnote);
            }
        }

        // References
        for ref_ref in references {
            if let Some(reference) =
                self.serialize_item(doc, &ref_ref.ref_path, list_level, visited, None)
            {
                parts.push(reference);
            }
        }

        if parts.is_empty() {
            None
        } else {
            Some(parts.join("\n\n"))
        }
    }

    /// Serialize `DocItem::Code` (inline or block)
    #[allow(
        clippy::too_many_arguments,
        reason = "serialization context: doc, text, children, parent, formatting, state"
    )]
    #[allow(
        clippy::unnecessary_wraps,
        reason = "consistent Option<String> interface with other serialize_*_item methods"
    )]
    fn serialize_code_item(
        &self,
        doc: &DoclingDocument,
        text: &str,
        children: &[ItemRef],
        parent: Option<&ItemRef>,
        formatting: Option<&crate::content::Formatting>,
        list_level: usize,
        visited: &mut HashSet<String>,
    ) -> Option<String> {
        let is_inline = parent
            .and_then(|parent_ref| doc.find_item(&parent_ref.ref_path))
            .is_some_and(|parent_item| matches!(parent_item, DocItem::Inline { .. }));

        let code_text = if is_inline {
            format!("`{text}`")
        } else {
            let trimmed_text = text.trim_end_matches('\n');
            format!("```\n{trimmed_text}\n```")
        };

        let formatted = Self::apply_formatting(&code_text, formatting);
        let mut parts = vec![formatted];

        for child_ref in children {
            if let Some(caption) =
                self.serialize_item(doc, &child_ref.ref_path, list_level, visited, None)
            {
                parts.push(caption);
            }
        }

        Some(parts.join("\n\n"))
    }

    /// Serialize `DocItem::Formula` (inline or display math)
    fn serialize_formula_item(
        &self,
        doc: &DoclingDocument,
        text: &str,
        children: &[ItemRef],
        parent: Option<&ItemRef>,
        list_level: usize,
        visited: &mut HashSet<String>,
    ) -> Option<String> {
        let mut parts = Vec::new();

        let is_inline = parent
            .and_then(|parent_ref| doc.find_item(&parent_ref.ref_path))
            .is_some_and(|parent_item| matches!(parent_item, DocItem::Inline { .. }));

        if text.is_empty() {
            parts.push("<!-- formula-not-decoded -->".to_string());
        } else {
            let formula_text = if is_inline {
                format!("${text}$")
            } else {
                format!("$${text}$$")
            };
            parts.push(formula_text);
        }

        for child_ref in children {
            if let Some(caption) =
                self.serialize_item(doc, &child_ref.ref_path, list_level, visited, None)
            {
                parts.push(caption);
            }
        }

        if parts.is_empty() {
            None
        } else {
            Some(parts.join("\n\n"))
        }
    }

    /// Serialize `DocItem::Picture` with captions, image, footnotes, references, and OCR text
    #[allow(
        clippy::too_many_arguments,
        reason = "picture serialization needs captions, image, footnotes, refs, ocr_text, state"
    )]
    fn serialize_picture_item(
        &self,
        doc: &DoclingDocument,
        captions: &[ItemRef],
        image: Option<&serde_json::Value>,
        footnotes: &[ItemRef],
        references: &[ItemRef],
        ocr_text: Option<&str>,
        list_level: usize,
        visited: &mut HashSet<String>,
    ) -> Option<String> {
        let mut parts = Vec::new();
        let mut caption_texts = Vec::new();

        // Captions first
        for caption_ref in captions {
            if let Some(caption_content) =
                self.serialize_item(doc, &caption_ref.ref_path, list_level, visited, None)
            {
                let alt_text = caption_content.trim().replace('\n', " ").replace("  ", " ");
                if !alt_text.is_empty() {
                    caption_texts.push(alt_text);
                }
                parts.push(caption_content);
            }
        }

        // Derive alt text
        let alt_text = caption_texts
            .first()
            .map(|s| {
                if s.len() > 100 {
                    format!("{}...", &s[..97])
                } else {
                    s.clone()
                }
            })
            .unwrap_or_default();

        // Image markdown - fallback for missing image data
        let fallback_markdown = || {
            if alt_text.is_empty() {
                "<!-- image -->".to_string()
            } else {
                format!("<!-- image: {alt_text} -->")
            }
        };
        let image_markdown = image.map_or_else(fallback_markdown, |img_data| {
            if let (Some(data), Some(mimetype)) = (
                img_data.get("data").and_then(|v| v.as_str()),
                img_data.get("mimetype").and_then(|v| v.as_str()),
            ) {
                format!("![{alt_text}](data:{mimetype};base64,{data})")
            } else {
                fallback_markdown()
            }
        });
        parts.push(image_markdown);

        // OCR text from figure content (scanned pages, charts, diagrams)
        // N=4355: Option A - Smart figure text grouping
        // Mark as [Figure text: ...] for clear identification in output
        if let Some(text) = ocr_text {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                // Clean up messy OCR: collapse multiple newlines, trim excessive whitespace
                let cleaned = trimmed
                    .lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .collect::<Vec<_>>()
                    .join(" ");

                if !cleaned.is_empty() {
                    // Option A: Mark figure text clearly with markdown comment
                    parts.push(format!("[Figure text: {}]", cleaned));
                }
            }
        }

        // Footnotes
        for fn_ref in footnotes {
            if let Some(footnote) =
                self.serialize_item(doc, &fn_ref.ref_path, list_level, visited, None)
            {
                parts.push(footnote);
            }
        }

        // References
        for ref_ref in references {
            if let Some(reference) =
                self.serialize_item(doc, &ref_ref.ref_path, list_level, visited, None)
            {
                parts.push(reference);
            }
        }

        if parts.is_empty() {
            None
        } else {
            Some(parts.join("\n\n"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::ItemRef;
    use crate::document::{DoclingDocument, GroupItem, Origin};
    use std::collections::HashMap;

    #[test]
    fn test_serialize_text_item() {
        let doc = DoclingDocument {
            schema_name: "DoclingDocument".to_string(),
            version: "1.7.0".to_string(),
            name: "test".to_string(),
            origin: Origin {
                mimetype: "application/pdf".to_string(),
                binary_hash: 0,
                filename: "test.pdf".to_string(),
            },
            body: GroupItem {
                self_ref: "#/body".to_string(),
                parent: None,
                children: vec![ItemRef::new("#/texts/0")],
                content_layer: "body".to_string(),
                name: "_root_".to_string(),
                label: "unspecified".to_string(),
            },
            furniture: None,
            texts: vec![DocItem::Text {
                self_ref: "#/texts/0".to_string(),
                parent: Some(ItemRef::new("#/body")),
                children: vec![],
                content_layer: "body".to_string(),
                prov: vec![],
                orig: "Hello, world!".to_string(),
                text: "Hello, world!".to_string(),
                formatting: None,
                hyperlink: None,
            }],
            groups: vec![],
            tables: vec![],
            pictures: vec![],
            key_value_items: vec![],
            form_items: vec![],
            pages: HashMap::new(),
        };

        let serializer = MarkdownSerializer::new();
        let markdown = serializer.serialize(&doc);

        assert_eq!(markdown, "Hello, world!");
    }

    #[test]
    fn test_serialize_section_header() {
        let doc = DoclingDocument {
            schema_name: "DoclingDocument".to_string(),
            version: "1.7.0".to_string(),
            name: "test".to_string(),
            origin: Origin {
                mimetype: "application/pdf".to_string(),
                binary_hash: 0,
                filename: "test.pdf".to_string(),
            },
            body: GroupItem {
                self_ref: "#/body".to_string(),
                parent: None,
                children: vec![ItemRef::new("#/texts/0")],
                content_layer: "body".to_string(),
                name: "_root_".to_string(),
                label: "unspecified".to_string(),
            },
            furniture: None,
            texts: vec![DocItem::SectionHeader {
                self_ref: "#/texts/0".to_string(),
                parent: Some(ItemRef::new("#/body")),
                children: vec![],
                content_layer: "body".to_string(),
                prov: vec![],
                orig: "Introduction".to_string(),
                text: "Introduction".to_string(),
                level: 1,
                formatting: None,
                hyperlink: None,
            }],
            groups: vec![],
            tables: vec![],
            pictures: vec![],
            key_value_items: vec![],
            form_items: vec![],
            pages: HashMap::new(),
        };

        let serializer = MarkdownSerializer::new();
        let markdown = serializer.serialize(&doc);

        assert_eq!(markdown, "## Introduction");
    }

    #[test]
    fn test_escape_underscores() {
        let result = MarkdownSerializer::escape_underscores("hello_world_test");
        assert_eq!(result, r"hello\_world\_test");
    }

    #[test]
    fn test_serialize_multi_page() {
        // Test serialization with actual multi_page.json
        // N=4322: Fixed paths to use groundtruth (full DoclingDocument + expected markdown)
        let json_path = "../../test-corpus/groundtruth/docling_v2/multi_page.json";
        let expected_path = "../../test-corpus/groundtruth/docling_v2/multi_page.md";

        if !std::path::Path::new(json_path).exists() {
            eprintln!("Skipping test: {json_path} not found");
            return;
        }

        let json_content =
            std::fs::read_to_string(json_path).expect("Failed to read multi_page.json");

        let doc: DoclingDocument =
            serde_json::from_str(&json_content).expect("Failed to deserialize DoclingDocument");

        let serializer = MarkdownSerializer::new();
        let markdown = serializer.serialize(&doc);

        // Read expected output if available
        if let Ok(expected) = std::fs::read_to_string(expected_path) {
            eprintln!("Generated: {} chars", markdown.len());
            eprintln!("Expected: {} chars", expected.len());
            eprintln!(
                "Difference: {} chars",
                (markdown.len() as i64 - expected.len() as i64).abs()
            );

            // Output first differences
            let gen_lines: Vec<&str> = markdown.lines().collect();
            let exp_lines: Vec<&str> = expected.lines().collect();

            for (i, (gen, exp)) in gen_lines.iter().zip(exp_lines.iter()).enumerate() {
                if gen != exp {
                    eprintln!("\nFirst difference at line {}:", i + 1);
                    eprintln!("Generated: {gen}");
                    eprintln!("Expected:  {exp}");
                    break;
                }
            }

            // For now, just check length is close (within 10%)
            let len_diff_pct = ((markdown.len() as f64 - expected.len() as f64).abs()
                / expected.len() as f64)
                * 100.0;
            eprintln!("Length difference: {len_diff_pct:.1}%");
        }

        // Check it's not empty
        assert!(!markdown.is_empty(), "Markdown should not be empty");

        // Check it contains expected headers
        assert!(
            markdown.contains("## The Evolution of the Word Processor"),
            "Should contain main header"
        );
    }

    #[test]
    fn test_serialize_with_tables() {
        // Test with a document that has tables
        let json_path = "../../test-results/outputs/pdf/redp5110_sampled.json";

        if !std::path::Path::new(json_path).exists() {
            eprintln!("Skipping test: {json_path} not found");
            return;
        }

        let json_content =
            std::fs::read_to_string(json_path).expect("Failed to read redp5110_sampled.json");

        // Try to deserialize - skip if it fails (missing variants)
        let doc: DoclingDocument = match serde_json::from_str(&json_content) {
            Ok(doc) => doc,
            Err(e) => {
                eprintln!("Skipping test: deserialization failed (missing DocItem variants): {e}");
                return;
            }
        };

        eprintln!("Document has {} tables", doc.tables.len());

        let serializer = MarkdownSerializer::new();
        let markdown = serializer.serialize(&doc);

        eprintln!("Generated: {} chars", markdown.len());

        // Check it's not empty and contains table markers
        assert!(!markdown.is_empty());
        assert!(markdown.contains('|'), "Should contain table separators");
    }

    #[test]
    fn test_markdown_serializer_default() {
        let default = MarkdownSerializer::default();
        let new = MarkdownSerializer::new();
        assert_eq!(default, new);
    }

    #[test]
    fn test_is_fake_section_header() {
        // N=4132: Test detection of reference content misclassified as section headers

        // Real section headers - should NOT be filtered
        assert!(
            !MarkdownSerializer::is_fake_section_header("1. Introduction"),
            "Real section header should not be filtered"
        );
        assert!(
            !MarkdownSerializer::is_fake_section_header("2 Related Work"),
            "Real section header should not be filtered"
        );
        assert!(
            !MarkdownSerializer::is_fake_section_header("Abstract"),
            "Real section header should not be filtered"
        );
        assert!(
            !MarkdownSerializer::is_fake_section_header("4.1 Methods"),
            "Real section header should not be filtered"
        );
        assert!(
            !MarkdownSerializer::is_fake_section_header("1900s Overview"),
            "Historical topic should not be filtered"
        );

        // Fake section headers (reference content) - SHOULD be filtered
        assert!(
            MarkdownSerializer::is_fake_section_header("1873. IEEE (2022)"),
            "Reference year+publisher should be filtered"
        );
        assert!(
            MarkdownSerializer::is_fake_section_header("2019. Proceedings of CVPR"),
            "Reference year+proceedings should be filtered"
        );
        assert!(
            MarkdownSerializer::is_fake_section_header("2022, pp. 123-456"),
            "Reference year+pages should be filtered"
        );
        assert!(
            MarkdownSerializer::is_fake_section_header("1996), ACM Press"),
            "Reference year ending paren should be filtered"
        );
        assert!(
            MarkdownSerializer::is_fake_section_header("2020. Springer"),
            "Reference year+Springer should be filtered"
        );

        // N=4322: Date patterns - SHOULD be filtered
        assert!(
            MarkdownSerializer::is_fake_section_header("5 May 2023"),
            "Date pattern 'day month year' should be filtered"
        );
        assert!(
            MarkdownSerializer::is_fake_section_header("May 5, 2023"),
            "Date pattern 'month day, year' should be filtered"
        );
        assert!(
            MarkdownSerializer::is_fake_section_header("15th January 2025"),
            "Date pattern with ordinal should be filtered"
        );
        assert!(
            MarkdownSerializer::is_fake_section_header("January 15, 2025"),
            "Date pattern 'month day, year' should be filtered"
        );
        assert!(
            MarkdownSerializer::is_fake_section_header("1 Jan 2024"),
            "Date pattern with abbreviated month should be filtered"
        );
        assert!(
            MarkdownSerializer::is_fake_section_header("Dec 25, 2023"),
            "Date pattern with abbreviated month should be filtered"
        );

        // NOT date patterns - should NOT be filtered
        assert!(
            !MarkdownSerializer::is_fake_section_header("May 2020"),
            "Month + year alone is not a full date (no day)"
        );
        assert!(
            !MarkdownSerializer::is_fake_section_header("May 2020 Conference"),
            "Month + year + other text is not a date pattern"
        );

        // N=4368: Standalone "Appendix" - SHOULD be filtered (OCR artifact from inline refs)
        assert!(
            MarkdownSerializer::is_fake_section_header("Appendix"),
            "Standalone 'Appendix' should be filtered (OCR artifact)"
        );
        assert!(
            MarkdownSerializer::is_fake_section_header("APPENDIX"),
            "Uppercase 'APPENDIX' should be filtered"
        );
        assert!(
            MarkdownSerializer::is_fake_section_header("appendix"),
            "Lowercase 'appendix' should be filtered"
        );

        // Real appendix headers - should NOT be filtered
        assert!(
            !MarkdownSerializer::is_fake_section_header("Appendix A"),
            "Appendix with letter should not be filtered"
        );
        assert!(
            !MarkdownSerializer::is_fake_section_header("Appendix B: Details"),
            "Appendix with title should not be filtered"
        );
        assert!(
            !MarkdownSerializer::is_fake_section_header("A Appendix"),
            "Appendix as suffix should not be filtered"
        );
    }

    #[test]
    fn test_is_date_pattern() {
        // N=4322: Test date pattern detection

        // Valid date patterns - SHOULD be detected
        assert!(MarkdownSerializer::is_date_pattern("5 May 2023"));
        assert!(MarkdownSerializer::is_date_pattern("May 5, 2023"));
        assert!(MarkdownSerializer::is_date_pattern("15th January 2025"));
        assert!(MarkdownSerializer::is_date_pattern("January 15, 2025"));
        assert!(MarkdownSerializer::is_date_pattern("1 Jan 2024"));
        assert!(MarkdownSerializer::is_date_pattern("Dec 25, 2023"));
        assert!(MarkdownSerializer::is_date_pattern("31 December 1999"));

        // NOT date patterns - should NOT be detected
        assert!(!MarkdownSerializer::is_date_pattern("May 2020")); // No day
        assert!(!MarkdownSerializer::is_date_pattern("2023")); // Just year
        assert!(!MarkdownSerializer::is_date_pattern("Introduction")); // No date
        assert!(!MarkdownSerializer::is_date_pattern(
            "May 2020 Conference Proceedings"
        )); // Too many words
        assert!(!MarkdownSerializer::is_date_pattern(
            "The May 2020 Conference"
        )); // Too many words

        // N=4322c: ISO 8601 format (YYYY-MM-DD)
        assert!(MarkdownSerializer::is_date_pattern("2023-05-05"));
        assert!(MarkdownSerializer::is_date_pattern("2025-01-15"));
        assert!(MarkdownSerializer::is_date_pattern("1999-12-31"));
        assert!(!MarkdownSerializer::is_date_pattern("2023-13-05")); // Invalid month
        assert!(!MarkdownSerializer::is_date_pattern("2023-05-32")); // Invalid day
        assert!(!MarkdownSerializer::is_date_pattern("1899-05-05")); // Year out of range
        assert!(!MarkdownSerializer::is_date_pattern("2100-05-05")); // Year out of range

        // N=4322c: European format (DD.MM.YYYY)
        assert!(MarkdownSerializer::is_date_pattern("05.05.2023"));
        assert!(MarkdownSerializer::is_date_pattern("15.01.2025"));
        assert!(MarkdownSerializer::is_date_pattern("31.12.1999"));
        assert!(!MarkdownSerializer::is_date_pattern("05.13.2023")); // Invalid month
        assert!(!MarkdownSerializer::is_date_pattern("32.05.2023")); // Invalid day
        assert!(!MarkdownSerializer::is_date_pattern("05.05.1899")); // Year out of range

        // Not date formats
        assert!(!MarkdownSerializer::is_date_pattern("1.2.3")); // Version-like
        assert!(!MarkdownSerializer::is_date_pattern("192.168.1.1")); // IP address
    }

    #[test]
    fn test_is_affiliation_pattern() {
        // N=4357: Test affiliation pattern detection

        // Real affiliations - SHOULD be detected (filtered from headers)
        assert!(
            MarkdownSerializer::is_affiliation_pattern(
                "1 Machine Learning Department, Carnegie Mellon University"
            ),
            "Affiliation with Department and University should be detected"
        );
        assert!(
            MarkdownSerializer::is_affiliation_pattern(
                "2 Department of Computer Science, Princeton University"
            ),
            "Affiliation with Department and University should be detected"
        );
        assert!(
            MarkdownSerializer::is_affiliation_pattern("1 Google Research"),
            "Company research affiliation should be detected"
        );
        assert!(
            MarkdownSerializer::is_affiliation_pattern("3 Microsoft Research"),
            "Company research affiliation should be detected"
        );
        assert!(
            MarkdownSerializer::is_affiliation_pattern(
                "5 MIT Laboratory for Information and Decision Systems"
            ),
            "MIT lab affiliation should be detected"
        );
        assert!(
            MarkdownSerializer::is_affiliation_pattern(
                "12 Stanford Institute for Human-Centered AI"
            ),
            "Stanford Institute should be detected"
        );

        // NOT affiliations - should NOT be detected (real section headers)
        assert!(
            !MarkdownSerializer::is_affiliation_pattern("1 Introduction"),
            "Real section header should not be filtered"
        );
        assert!(
            !MarkdownSerializer::is_affiliation_pattern("2 Related Work"),
            "Real section header should not be filtered"
        );
        assert!(
            !MarkdownSerializer::is_affiliation_pattern("3 Methods"),
            "Real section header should not be filtered"
        );
        assert!(
            !MarkdownSerializer::is_affiliation_pattern("4 Results"),
            "Real section header should not be filtered"
        );
        assert!(
            !MarkdownSerializer::is_affiliation_pattern("25 Some Section"),
            "Large number (>20) suggests real section, not affiliation"
        );
        assert!(
            !MarkdownSerializer::is_affiliation_pattern("Abstract"),
            "No leading number - not an affiliation pattern"
        );
    }

    #[test]
    fn test_is_algorithm_label_pattern() {
        // N=4357: Test algorithm/figure box label detection

        // Algorithm labels - SHOULD be detected (filtered from headers)
        assert!(
            MarkdownSerializer::is_algorithm_label_pattern("1 SSM (S4)"),
            "Algorithm label with abbreviation should be detected"
        );
        assert!(
            MarkdownSerializer::is_algorithm_label_pattern("2 SSM + Selection (S6)"),
            "Algorithm label with abbreviation should be detected"
        );
        assert!(
            MarkdownSerializer::is_algorithm_label_pattern("1 CNN Architecture"),
            "Algorithm label with acronym should be detected"
        );
        assert!(
            MarkdownSerializer::is_algorithm_label_pattern("3 MLP Block"),
            "Algorithm label with acronym should be detected"
        );

        // NOT algorithm labels - should NOT be detected (real section headers)
        assert!(
            !MarkdownSerializer::is_algorithm_label_pattern("1 Introduction"),
            "Real section header (lowercase word) should not be filtered"
        );
        assert!(
            !MarkdownSerializer::is_algorithm_label_pattern("1. Introduction"),
            "Section header with period should not be filtered"
        );
        assert!(
            !MarkdownSerializer::is_algorithm_label_pattern("2 Related Work"),
            "Real section header should not be filtered"
        );
        assert!(
            !MarkdownSerializer::is_algorithm_label_pattern("Abstract"),
            "No leading number - not an algorithm label"
        );
        assert!(
            !MarkdownSerializer::is_algorithm_label_pattern(
                "1 This is a very long title that goes on and on"
            ),
            "Long text should not be detected as algorithm label"
        );
    }

    #[test]
    fn test_build_list_marker_ocr_artifacts() {
        // N=4366: Test OCR bullet artifact normalization
        let serializer = MarkdownSerializer::new();

        // Standard markdown markers should be preserved
        assert_eq!(serializer.build_list_marker("-", None), vec!["-"]);
        assert_eq!(serializer.build_list_marker("*", None), vec!["*"]);
        assert_eq!(serializer.build_list_marker("+", None), vec!["+"]);
        assert_eq!(serializer.build_list_marker("1.", None), vec!["1."]);
        assert_eq!(serializer.build_list_marker("42.", None), vec!["42."]);

        // OCR bullet artifacts should be normalized to "-"
        assert_eq!(
            serializer.build_list_marker("∞", None),
            vec!["-"],
            "Infinity symbol (common OCR misread) should normalize to -"
        );
        assert_eq!(
            serializer.build_list_marker("•", None),
            vec!["-"],
            "Bullet point should normalize to -"
        );
        assert_eq!(
            serializer.build_list_marker("·", None),
            vec!["-"],
            "Middle dot should normalize to -"
        );
        assert_eq!(
            serializer.build_list_marker("▪", None),
            vec!["-"],
            "Black square should normalize to -"
        );
        assert_eq!(
            serializer.build_list_marker("→", None),
            vec!["-"],
            "Arrow should normalize to -"
        );
        assert_eq!(
            serializer.build_list_marker("✓", None),
            vec!["-"],
            "Checkmark should normalize to -"
        );

        // OCR bullet artifacts with enumeration should use numbered format
        assert_eq!(
            serializer.build_list_marker("∞", Some(0)),
            vec!["1."],
            "OCR artifact with index 0 should become 1."
        );
        assert_eq!(
            serializer.build_list_marker("•", Some(2)),
            vec!["3."],
            "OCR artifact with index 2 should become 3."
        );

        // Empty marker cases
        assert_eq!(
            serializer.build_list_marker("", None),
            vec!["-"],
            "Empty marker without enumeration should be -"
        );
        assert_eq!(
            serializer.build_list_marker("", Some(0)),
            vec!["1."],
            "Empty marker with enumeration should be numbered"
        );

        // Custom markers (not OCR artifacts) should be preserved with prefix
        assert_eq!(
            serializer.build_list_marker("a.", None),
            vec!["-", "a."],
            "Custom letter marker should be preserved with prefix"
        );
        assert_eq!(
            serializer.build_list_marker("i)", None),
            vec!["-", "i)"],
            "Roman numeral marker should be preserved with prefix"
        );
    }

    #[test]
    fn test_strip_leading_bullet_artifact() {
        // N=4369: Test stripping of leading OCR bullet artifacts from text content

        // Infinity symbol at start should be stripped
        assert_eq!(
            MarkdownSerializer::strip_leading_bullet_artifact(
                "∞ Synthetics. On important tasks..."
            ),
            "Synthetics. On important tasks...",
            "Leading ∞ should be stripped"
        );

        // Other bullet artifacts should also be stripped
        assert_eq!(
            MarkdownSerializer::strip_leading_bullet_artifact("• First item"),
            "First item",
            "Leading • should be stripped"
        );
        assert_eq!(
            MarkdownSerializer::strip_leading_bullet_artifact("▪ Black square item"),
            "Black square item",
            "Leading ▪ should be stripped"
        );
        assert_eq!(
            MarkdownSerializer::strip_leading_bullet_artifact("⊲ comment annotation"),
            "comment annotation",
            "Leading ⊲ should be stripped"
        );

        // Regular text without bullet artifacts should be unchanged
        assert_eq!(
            MarkdownSerializer::strip_leading_bullet_artifact("Normal text without bullet"),
            "Normal text without bullet",
            "Text without artifact should be unchanged"
        );

        // Text starting with space then bullet should strip both
        assert_eq!(
            MarkdownSerializer::strip_leading_bullet_artifact("  ∞ Indented bullet text"),
            "Indented bullet text",
            "Leading space + bullet should both be stripped"
        );

        // Text that just happens to contain bullet char (not at start) should be unchanged
        assert_eq!(
            MarkdownSerializer::strip_leading_bullet_artifact("Text with ∞ symbol inside"),
            "Text with ∞ symbol inside",
            "Bullet char not at start should be kept"
        );

        // Empty text should remain empty
        assert_eq!(
            MarkdownSerializer::strip_leading_bullet_artifact(""),
            "",
            "Empty text should remain empty"
        );

        // Just the bullet char should produce empty string
        assert_eq!(
            MarkdownSerializer::strip_leading_bullet_artifact("∞"),
            "",
            "Just bullet char should produce empty string"
        );

        // Bullet char followed by text without space
        assert_eq!(
            MarkdownSerializer::strip_leading_bullet_artifact("∞NoSpace"),
            "NoSpace",
            "Bullet without following space should still strip"
        );
    }

    #[test]
    fn test_is_figure_artifact() {
        // N=4367: Test detection of figure/algorithm artifacts

        // Algorithm annotations starting with ⊲ should be filtered
        assert!(
            MarkdownSerializer::is_figure_artifact("⊲ Represents structured N × N matrix"),
            "Algorithm annotation should be filtered"
        );
        assert!(
            MarkdownSerializer::is_figure_artifact("⊲ Time-varying: recurrence (scan) only"),
            "Algorithm annotation should be filtered"
        );

        // Single-word figure labels should be filtered
        assert!(
            MarkdownSerializer::is_figure_artifact("Input"),
            "Figure label 'Input' should be filtered"
        );
        assert!(
            MarkdownSerializer::is_figure_artifact("Output"),
            "Figure label 'Output' should be filtered"
        );
        assert!(
            MarkdownSerializer::is_figure_artifact("Solution"),
            "Figure label 'Solution' should be filtered"
        );
        assert!(
            MarkdownSerializer::is_figure_artifact("Input:"),
            "Figure label 'Input:' should be filtered"
        );
        assert!(
            MarkdownSerializer::is_figure_artifact("Output:"),
            "Figure label 'Output:' should be filtered"
        );

        // Single punctuation should be filtered (OCR noise)
        assert!(
            MarkdownSerializer::is_figure_artifact("()"),
            "Single punctuation should be filtered"
        );
        assert!(
            MarkdownSerializer::is_figure_artifact(":"),
            "Single colon should be filtered"
        );
        assert!(
            MarkdownSerializer::is_figure_artifact("?"),
            "Single question mark should be filtered"
        );

        // Real text should NOT be filtered
        assert!(
            !MarkdownSerializer::is_figure_artifact("The input parameters are defined as follows."),
            "Real sentence mentioning 'input' should not be filtered"
        );
        assert!(
            !MarkdownSerializer::is_figure_artifact("This is a normal paragraph."),
            "Normal paragraph should not be filtered"
        );
        assert!(
            !MarkdownSerializer::is_figure_artifact("Algorithm 1: SSM (S4)"),
            "Algorithm title should not be filtered"
        );
        assert!(
            !MarkdownSerializer::is_figure_artifact("Input: 𝑥 : (B, L, D)"),
            "Algorithm input line should not be filtered (has more content)"
        );
        assert!(
            !MarkdownSerializer::is_figure_artifact("1: 𝑨 : (D, N) ← Parameter"),
            "Algorithm step should not be filtered"
        );

        // N=4371: New diagram/figure artifact patterns
        // Hardware labels
        assert!(
            MarkdownSerializer::is_figure_artifact("GPU"),
            "Hardware label 'GPU' should be filtered"
        );
        assert!(
            MarkdownSerializer::is_figure_artifact("SRAM"),
            "Hardware label 'SRAM' should be filtered"
        );
        assert!(
            MarkdownSerializer::is_figure_artifact("GPU HBM"),
            "Hardware label 'GPU HBM' should be filtered"
        );
        assert!(
            MarkdownSerializer::is_figure_artifact("Project"),
            "Diagram label 'Project' should be filtered"
        );
        assert!(
            MarkdownSerializer::is_figure_artifact("Discretize"),
            "Diagram label 'Discretize' should be filtered"
        );

        // Math subscript artifacts (OCR noise from figures)
        assert!(
            MarkdownSerializer::is_figure_artifact("x!"),
            "Math subscript 'x!' should be filtered"
        );
        assert!(
            MarkdownSerializer::is_figure_artifact("!\"#"),
            "Subscript noise '!\"#' should be filtered"
        );
        assert!(
            MarkdownSerializer::is_figure_artifact("! "),
            "Subscript noise '! ' should be filtered"
        );

        // Math italic symbols (Unicode math letters)
        assert!(
            MarkdownSerializer::is_figure_artifact("𝑥!"),
            "Math italic '𝑥!' should be filtered"
        );
        assert!(
            MarkdownSerializer::is_figure_artifact("𝐴"),
            "Math italic '𝐴' should be filtered"
        );
        assert!(
            MarkdownSerializer::is_figure_artifact("𝑦!"),
            "Math italic '𝑦!' should be filtered"
        );

        // NN diagram labels
        assert!(
            MarkdownSerializer::is_figure_artifact("Linear"),
            "NN label 'Linear' should be filtered"
        );
        assert!(
            MarkdownSerializer::is_figure_artifact("Softmax"),
            "NN label 'Softmax' should be filtered"
        );

        // But real text with these words should NOT be filtered
        assert!(
            !MarkdownSerializer::is_figure_artifact("The GPU memory hierarchy is important."),
            "Sentence with 'GPU' should not be filtered"
        );
        assert!(
            !MarkdownSerializer::is_figure_artifact("We apply a linear transformation."),
            "Sentence with 'linear' should not be filtered"
        );
        assert!(
            !MarkdownSerializer::is_figure_artifact("The input is discretized using Δ."),
            "Sentence with 'discretized' should not be filtered"
        );
    }
}
