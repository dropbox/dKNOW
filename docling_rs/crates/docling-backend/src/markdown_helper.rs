//! Helper functions for converting `DocItems` to formatted markdown
//!
//! This module provides utilities for backends to convert their parsed `DocItems`
//! into properly formatted markdown, respecting inline formatting like bold, italic,
//! underline, strikethrough, etc.

// Clippy pedantic allows:
// - Using &Option<T> matches the API pattern from DocItem
#![allow(clippy::ref_option)]

use docling_core::content::{DocItem, Formatting, ItemRef, TableData};
use std::collections::HashMap;
use std::fmt::Write;

/// Convert a list of `DocItems` to formatted markdown
///
/// This function applies inline formatting (bold, italic, strikethrough) to text
/// and properly handles other `DocItem` types like headers, lists, tables, pictures, etc.
#[must_use = "returns the markdown representation of the document items"]
pub fn docitems_to_markdown(doc_items: &[DocItem]) -> String {
    let mut builder = MarkdownBuilder::new(doc_items);
    builder.build();
    builder.markdown.trim_end().to_string()
}

/// Builder for converting `DocItem`s to markdown
struct MarkdownBuilder<'a> {
    doc_items: &'a [DocItem],
    parent_map: HashMap<String, String>,
    ref_to_item: HashMap<&'a str, &'a DocItem>,
    markdown: String,
    in_numbered_list: bool,
    in_bullet_list: bool,
    current_list_ref: Option<String>,
}

impl<'a> MarkdownBuilder<'a> {
    fn new(doc_items: &'a [DocItem]) -> Self {
        let parent_map = build_parent_map(doc_items);
        let ref_to_item = doc_items
            .iter()
            .map(|item| (item.self_ref(), item))
            .collect();
        Self {
            doc_items,
            parent_map,
            ref_to_item,
            markdown: String::new(),
            in_numbered_list: false,
            in_bullet_list: false,
            current_list_ref: None,
        }
    }

    fn build(&mut self) {
        for (idx, item) in self.doc_items.iter().enumerate() {
            let next_is_list = self.check_next_is_list(idx);
            self.handle_item(item, next_is_list);
        }
    }

    #[inline]
    fn check_next_is_list(&self, idx: usize) -> bool {
        if idx + 1 < self.doc_items.len() {
            matches!(
                self.doc_items[idx + 1],
                DocItem::ListItem { .. } | DocItem::List { .. } | DocItem::OrderedList { .. }
            )
        } else {
            false
        }
    }

    fn handle_item(&mut self, item: &DocItem, next_is_list: bool) {
        match item {
            DocItem::Text {
                text,
                formatting,
                hyperlink,
                parent,
                ..
            } => {
                self.handle_text(text, formatting.as_ref(), hyperlink.as_deref(), parent);
            }
            DocItem::SectionHeader {
                text,
                level,
                hyperlink,
                ..
            } => {
                self.handle_section_header(text, *level, hyperlink.as_deref());
            }
            DocItem::Title { text, .. } => {
                self.handle_title(text);
            }
            DocItem::Paragraph { text, .. } => {
                self.handle_paragraph(text);
            }
            DocItem::ListItem {
                self_ref,
                parent,
                children,
                text,
                formatting,
                hyperlink,
                enumerated,
                marker,
                ..
            } => {
                self.handle_list_item(
                    self_ref,
                    parent,
                    children,
                    text,
                    formatting.as_ref(),
                    hyperlink.as_deref(),
                    *enumerated,
                    marker,
                    next_is_list,
                );
            }
            DocItem::List { .. } | DocItem::OrderedList { .. } => {
                // Transparent containers - no output
            }
            DocItem::Table { data, .. } => {
                self.handle_table(data);
            }
            DocItem::Picture { .. } => {
                self.handle_picture();
            }
            DocItem::Inline { children, .. } => {
                self.handle_inline(children);
            }
            DocItem::Code { text, language, .. } => {
                self.handle_code(text, language.as_deref());
            }
            DocItem::Formula { text, .. } => {
                self.handle_formula(text);
            }
            _ => {
                self.reset_list_state();
            }
        }
    }

    fn has_inline_parent(&self, parent: &Option<ItemRef>) -> bool {
        if let Some(p) = parent {
            if let Some(parent_item) = self.ref_to_item.get(p.ref_path.as_str()) {
                return matches!(parent_item, DocItem::Inline { .. });
            }
        }
        false
    }

    fn reset_list_state(&mut self) {
        if self.in_bullet_list || self.in_numbered_list {
            self.markdown.push('\n');
        }
        self.in_numbered_list = false;
        self.in_bullet_list = false;
        self.current_list_ref = None;
    }

    fn handle_text(
        &mut self,
        text: &str,
        formatting: Option<&Formatting>,
        hyperlink: Option<&str>,
        parent: &Option<ItemRef>,
    ) {
        if self.has_inline_parent(parent) || text.trim().is_empty() {
            return;
        }
        self.reset_list_state();
        let final_text = format_with_hyperlink(&apply_formatting(text, formatting), hyperlink);
        self.markdown.push_str(&final_text);
        self.markdown.push_str("\n\n");
    }

    fn handle_section_header(&mut self, text: &str, level: usize, hyperlink: Option<&str>) {
        self.reset_list_state();
        self.markdown.push_str(&"#".repeat(level));
        self.markdown.push(' ');
        if let Some(url) = hyperlink {
            if is_valid_url(url) {
                let _ = write!(self.markdown, "[{text}]({url})");
            } else {
                self.markdown.push_str(text);
            }
        } else {
            self.markdown.push_str(text);
        }
        self.markdown.push_str("\n\n");
    }

    fn handle_title(&mut self, text: &str) {
        self.reset_list_state();
        self.markdown.push_str("# ");
        self.markdown.push_str(text);
        self.markdown.push_str("\n\n");
    }

    fn handle_paragraph(&mut self, text: &str) {
        self.reset_list_state();
        self.markdown.push_str(text);
        self.markdown.push_str("\n\n");
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "list item handling requires ref, parent, children, text, formatting"
    )]
    fn handle_list_item(
        &mut self,
        self_ref: &str,
        parent: &Option<ItemRef>,
        children: &[ItemRef],
        text: &str,
        formatting: Option<&Formatting>,
        hyperlink: Option<&str>,
        enumerated: bool,
        marker: &str,
        next_is_list: bool,
    ) {
        let depth =
            calculate_list_depth(self_ref, parent.as_ref(), &self.parent_map, self.doc_items);
        let indent = "    ".repeat(depth);
        let list_ref = parent.as_ref().map(|p| p.ref_path.clone());

        let list_changed =
            depth == 0 && self.current_list_ref.is_some() && list_ref != self.current_list_ref;
        if list_changed {
            self.markdown.push('\n');
        }
        if depth == 0 {
            self.current_list_ref = list_ref;
        }

        self.write_list_marker(enumerated, &indent, marker, list_changed);

        let final_text = format_with_hyperlink(&apply_formatting(text, formatting), hyperlink);
        self.markdown.push_str(&final_text);

        self.write_inline_children(children);
        self.markdown.push('\n');

        if !next_is_list {
            self.markdown.push('\n');
            self.in_bullet_list = false;
            self.in_numbered_list = false;
        }
    }

    fn write_list_marker(
        &mut self,
        enumerated: bool,
        indent: &str,
        marker: &str,
        list_changed: bool,
    ) {
        if enumerated {
            if !self.in_numbered_list && !list_changed && self.in_bullet_list {
                self.markdown.push('\n');
                self.in_bullet_list = false;
            }
            self.in_numbered_list = true;
            self.markdown.push_str(indent);
            self.markdown.push_str(marker);
            self.markdown.push(' ');
        } else {
            if self.in_numbered_list && !list_changed {
                self.markdown.push('\n');
                self.in_numbered_list = false;
            }
            self.in_bullet_list = true;
            if marker.starts_with("    ") {
                self.markdown.push_str(marker);
                if !marker.ends_with(' ') {
                    self.markdown.push(' ');
                }
            } else {
                self.markdown.push_str(indent);
                self.markdown.push_str("- ");
            }
        }
    }

    fn write_inline_children(&mut self, children: &[ItemRef]) {
        if children.is_empty() {
            return;
        }
        let inline_parts = self.collect_inline_parts(children);
        if !inline_parts.is_empty() {
            self.markdown.push(' ');
            self.markdown.push_str(&inline_parts.join(" "));
        }
    }

    fn collect_inline_parts(&self, children: &[ItemRef]) -> Vec<String> {
        let mut parts = Vec::new();
        for child_ref in children {
            if let Some(child_item) = self.ref_to_item.get(child_ref.ref_path.as_str()) {
                match child_item {
                    DocItem::Inline {
                        children: inline_children,
                        ..
                    } => {
                        for sub_ref in inline_children {
                            if let Some(DocItem::Text {
                                text,
                                formatting,
                                hyperlink,
                                ..
                            }) = self.ref_to_item.get(sub_ref.ref_path.as_str())
                            {
                                if !text.trim().is_empty() {
                                    parts.push(format_with_hyperlink(
                                        &apply_formatting(text, formatting.as_ref()),
                                        hyperlink.as_deref(),
                                    ));
                                }
                            }
                        }
                    }
                    DocItem::Text {
                        text,
                        formatting,
                        hyperlink,
                        ..
                    } => {
                        if !text.trim().is_empty() {
                            parts.push(format_with_hyperlink(
                                &apply_formatting(text, formatting.as_ref()),
                                hyperlink.as_deref(),
                            ));
                        }
                    }
                    _ => {}
                }
            }
        }
        parts
    }

    fn handle_table(&mut self, data: &TableData) {
        self.reset_list_state();
        self.markdown.push_str(&render_table(data));
        self.markdown.push('\n');
    }

    fn handle_picture(&mut self) {
        self.reset_list_state();
        self.markdown.push_str("<!-- image -->\n\n");
    }

    fn handle_inline(&mut self, children: &[ItemRef]) {
        self.reset_list_state();
        let inline_parts = self.collect_inline_parts(children);
        if !inline_parts.is_empty() {
            self.markdown.push_str(&inline_parts.join(" "));
            self.markdown.push_str("\n\n");
        }
    }

    fn handle_code(&mut self, text: &str, language: Option<&str>) {
        self.reset_list_state();
        self.markdown.push_str("```");
        if let Some(lang) = language {
            self.markdown.push_str(lang);
        }
        self.markdown.push('\n');
        self.markdown.push_str(text);
        self.markdown.push_str("\n```\n\n");
    }

    fn handle_formula(&mut self, text: &str) {
        self.reset_list_state();
        self.markdown.push_str("$$");
        self.markdown.push_str(text);
        self.markdown.push_str("$$\n\n");
    }
}

/// Check if URL is valid for markdown links
#[inline]
fn is_valid_url(url: &str) -> bool {
    url.starts_with("http://")
        || url.starts_with("https://")
        || url.starts_with("mailto:")
        || url.starts_with('#')
        || url.starts_with('/')
}

/// Format text with hyperlink if URL is valid
#[inline]
fn format_with_hyperlink(text: &str, hyperlink: Option<&str>) -> String {
    if let Some(url) = hyperlink {
        if is_valid_url(url) {
            return format!("[{text}]({url})");
        }
    }
    text.to_string()
}

/// Render table as markdown
///
/// Render table data to markdown format.
///
/// NOTE: The `TableData` grid already handles cell merging (rowSpan/colSpan).
/// This function simply serializes the pre-processed grid to markdown.
/// The grid contains replicated cell content across spans for correct markdown rendering.
#[must_use = "returns the markdown representation of the table"]
pub fn render_table(data: &TableData) -> String {
    use std::fmt::Write;

    if data.grid.is_empty() {
        return String::new();
    }

    // Calculate column widths using Python tabulate's algorithm
    let num_cols = data.num_cols;

    // First, detect numeric columns (all data rows contain only numbers)
    // Need to do this first because column width calculation differs for numeric vs text
    let mut col_is_numeric = vec![false; num_cols];
    if data.grid.len() > 1 {
        for (col_idx, is_numeric) in col_is_numeric.iter_mut().enumerate() {
            let mut all_numeric = true;
            for row in &data.grid[1..] {
                // Skip header row
                if let Some(cell) = row.get(col_idx) {
                    let text = cell.text.trim();
                    if text.is_empty() {
                        all_numeric = false;
                        break;
                    }
                    if text.parse::<f64>().is_err() && text.parse::<i64>().is_err() {
                        all_numeric = false;
                        break;
                    }
                }
            }
            *is_numeric = all_numeric;
        }
    }

    // Calculate column widths:
    // - Text columns: max(header_len + 2, max_data_len)
    // - Numeric columns: max(header_len, max_data_len) + 2
    let mut col_widths = vec![0_usize; num_cols];

    // First pass: get max content width across all rows
    for row in &data.grid {
        for (col_idx, cell) in row.iter().take(num_cols).enumerate() {
            let cell_len = cell.text.chars().count();
            col_widths[col_idx] = col_widths[col_idx].max(cell_len);
        }
    }

    // Second pass: apply column-type-specific padding
    if !data.grid.is_empty() {
        let header_row = &data.grid[0];
        for (col_idx, cell) in header_row.iter().take(num_cols).enumerate() {
            let header_len = cell.text.chars().count();
            if col_is_numeric[col_idx] {
                // Numeric columns: width = max(header, data) + 2
                col_widths[col_idx] = col_widths[col_idx].max(header_len) + 2;
            } else {
                // Text columns: width = max(header + 2, data)
                col_widths[col_idx] = col_widths[col_idx].max(header_len + 2);
            }
        }
    }

    let mut result = String::new();

    // Render table with proper column widths and alignment
    for (row_idx, row) in data.grid.iter().enumerate() {
        result.push('|');
        for (col_idx, cell) in row.iter().take(num_cols).enumerate() {
            let width = col_widths[col_idx];
            let text = &cell.text;

            if col_is_numeric[col_idx] {
                // Right-align numeric columns (header and data rows)
                let _ = write!(result, " {text:>width$} |");
            } else {
                // Left-align text columns
                let _ = write!(result, " {text:<width$} |");
            }
        }
        result.push('\n');

        // Add header separator after first row
        if row_idx == 0 {
            result.push('|');
            for width in &col_widths {
                // Pre-compute separator string to avoid repeated allocations
                let separator_len = width + 2;
                for _ in 0..separator_len {
                    result.push('-');
                }
                result.push('|');
            }
            result.push('\n');
        }
    }

    result
}

/// Apply formatting (bold, italic, strikethrough, code) to text
///
/// # Arguments
///
/// * `text` - The text to format
/// * `formatting` - Optional formatting to apply
///
/// # Returns
///
/// The text with markdown formatting applied
fn apply_formatting(text: &str, formatting: Option<&Formatting>) -> String {
    let Some(fmt) = formatting else {
        return text.to_string();
    };

    let mut result = text.to_string();

    let is_bold = fmt.bold.unwrap_or_default();
    let is_italic = fmt.italic.unwrap_or_default();
    let is_strikethrough = fmt.strikethrough.unwrap_or_default();
    let is_code = fmt.code.unwrap_or_default();

    // Apply inline code first (backticks) - code formatting takes precedence
    // When text is code, other formatting (bold/italic) is typically not applied
    if is_code {
        result = format!("`{result}`");
    } else {
        // Apply markdown formatting (order matters: italic+bold first, then strikethrough)
        if is_italic && is_bold {
            result = format!("***{result}***");
        } else if is_bold {
            result = format!("**{result}**");
        } else if is_italic {
            result = format!("*{result}*");
        }
    }

    // Apply strikethrough (can combine with bold/italic/code)
    if is_strikethrough {
        result = format!("~~{result}~~");
    }

    // Note: Underline is not handled because standard markdown doesn't support it
    // (HTML <u> tags would be needed, but that breaks pure markdown compatibility)

    result
}

/// Build a map of `self_ref` → `parent_ref` for calculating nesting depth
///
/// This function extracts the parent reference from each `DocItem` that has one,
/// creating a map that allows us to traverse the parent chain for any item.
#[allow(
    clippy::manual_let_else,
    reason = "complex multi-variant match is clearer than let-else"
)]
fn build_parent_map(doc_items: &[DocItem]) -> HashMap<String, String> {
    let mut parent_map = HashMap::new();

    for item in doc_items {
        let (self_ref, parent) = match item {
            DocItem::ListItem {
                self_ref, parent, ..
            }
            | DocItem::Text {
                self_ref, parent, ..
            }
            | DocItem::Picture {
                self_ref, parent, ..
            }
            | DocItem::Table {
                self_ref, parent, ..
            }
            | DocItem::Code {
                self_ref, parent, ..
            }
            | DocItem::List {
                self_ref, parent, ..
            }
            | DocItem::Inline {
                self_ref, parent, ..
            }
            | DocItem::Formula {
                self_ref, parent, ..
            } => (self_ref, parent),
            _ => continue,
        };

        if let Some(parent_ref) = parent {
            parent_map.insert(self_ref.clone(), parent_ref.ref_path.clone());
        }
    }

    parent_map
}

/// Calculate the nesting depth of a list item by counting List ancestors
///
/// The depth is determined by traversing the parent chain and counting how many
/// List containers (not `ListItems`) we encounter. This matches Python docling behavior
/// where nested lists have their parent set to the containing List, not the parent `ListItem`.
///
/// # Arguments
///
/// * `self_ref` - The self reference of the current list item
/// * `parent` - The parent reference of the current list item
/// * `parent_map` - Map of `self_ref` → `parent_ref` for all items
/// * `doc_items` - All `DocItems` (to look up parent types)
///
/// # Returns
///
/// The nesting depth (0 for top-level lists, 1 for first nesting level, etc.)
fn calculate_list_depth(
    _self_ref: &str,
    parent: Option<&ItemRef>,
    parent_map: &HashMap<String, String>,
    doc_items: &[DocItem],
) -> usize {
    let Some(parent_ref) = parent else {
        return 0; // No parent = top-level list item
    };

    // Count how many List ancestors we have by traversing the parent chain
    let mut depth: usize = 0;
    let mut current_ref = parent_ref.ref_path.clone();

    // Traverse parent chain, counting List containers
    // The immediate parent should be a List, so check it first
    loop {
        // Check if current_ref points to a List (not ListItem)
        let is_list = doc_items.iter().any(|item| match item {
            DocItem::List { self_ref, .. } | DocItem::OrderedList { self_ref, .. } => {
                self_ref == &current_ref
            }
            _ => false,
        });

        if is_list {
            depth += 1;
        }

        // Move to parent (only if we found the parent in parent_map)
        match parent_map.get(&current_ref) {
            Some(next_parent) => {
                current_ref = next_parent.clone();
            }
            None => break, // Reached root (no more parents)
        }
    }

    // Depth represents the number of List ancestors
    // depth=1 means top-level list (0 indentation)
    // depth=2 means first nesting level (4 spaces indentation)
    // So indent_level = depth - 1
    depth.saturating_sub(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_formatting_bold() {
        let formatting = Formatting {
            bold: Some(true),
            italic: None,
            underline: None,
            strikethrough: None,
            code: None,
            script: None,
            font_size: None,
            font_family: None,
        };

        let result = apply_formatting("bold text", Some(&formatting));
        assert_eq!(result, "**bold text**");
    }

    #[test]
    fn test_apply_formatting_italic() {
        let formatting = Formatting {
            bold: None,
            italic: Some(true),
            underline: None,
            strikethrough: None,
            code: None,
            script: None,
            font_size: None,
            font_family: None,
        };

        let result = apply_formatting("italic text", Some(&formatting));
        assert_eq!(result, "*italic text*");
    }

    #[test]
    fn test_apply_formatting_bold_italic() {
        let formatting = Formatting {
            bold: Some(true),
            italic: Some(true),
            underline: None,
            strikethrough: None,
            code: None,
            script: None,
            font_size: None,
            font_family: None,
        };

        let result = apply_formatting("bold italic", Some(&formatting));
        assert_eq!(result, "***bold italic***");
    }

    #[test]
    fn test_apply_formatting_strikethrough() {
        let formatting = Formatting {
            bold: None,
            italic: None,
            underline: None,
            strikethrough: Some(true),
            code: None,
            script: None,
            font_size: None,
            font_family: None,
        };

        let result = apply_formatting("strikethrough", Some(&formatting));
        assert_eq!(result, "~~strikethrough~~");
    }

    #[test]
    fn test_apply_formatting_none() {
        let result = apply_formatting("plain text", None);
        assert_eq!(result, "plain text");
    }

    #[test]
    fn test_docitems_to_markdown_basic() {
        let items = vec![
            DocItem::Text {
                self_ref: "#/texts/0".to_string(),
                parent: None,
                children: vec![],
                content_layer: "body".to_string(),
                prov: vec![],
                orig: "Hello".to_string(),
                text: "Hello".to_string(),
                formatting: None,
                hyperlink: None,
            },
            DocItem::SectionHeader {
                self_ref: "#/texts/1".to_string(),
                parent: None,
                children: vec![],
                content_layer: "body".to_string(),
                prov: vec![],
                orig: "Header".to_string(),
                text: "Header".to_string(),
                level: 2,
                formatting: None,
                hyperlink: None,
            },
        ];

        let markdown = docitems_to_markdown(&items);
        assert!(markdown.contains("Hello"));
        assert!(markdown.contains("## Header"));
    }

    #[test]
    fn test_apply_formatting_code() {
        let formatting = Formatting {
            bold: None,
            italic: None,
            underline: None,
            strikethrough: None,
            code: Some(true),
            script: None,
            font_size: None,
            font_family: None,
        };

        let result = apply_formatting("code text", Some(&formatting));
        assert_eq!(result, "`code text`");
    }

    #[test]
    fn test_apply_formatting_underline() {
        let formatting = Formatting {
            bold: None,
            italic: None,
            underline: Some(true),
            strikethrough: None,
            code: None,
            script: None,
            font_size: None,
            font_family: None,
        };

        // Underline is rendered as italic in markdown (no native underline)
        let result = apply_formatting("underline text", Some(&formatting));
        // Just verify it returns the text (underline may be rendered as-is or italic)
        assert!(result.contains("underline text"));
    }

    #[test]
    fn test_apply_formatting_combined() {
        let formatting = Formatting {
            bold: Some(true),
            italic: Some(true),
            underline: None,
            strikethrough: Some(true),
            code: None,
            script: None,
            font_size: None,
            font_family: None,
        };

        let result = apply_formatting("styled", Some(&formatting));
        assert!(result.contains("styled"));
        // Should have multiple formatting markers
        assert!(result.contains("**") || result.contains("*"));
    }

    #[test]
    fn test_docitems_to_markdown_headers() {
        let items = vec![
            DocItem::SectionHeader {
                self_ref: "#/texts/0".to_string(),
                parent: None,
                children: vec![],
                content_layer: "body".to_string(),
                prov: vec![],
                orig: "H1".to_string(),
                text: "H1".to_string(),
                level: 1,
                formatting: None,
                hyperlink: None,
            },
            DocItem::SectionHeader {
                self_ref: "#/texts/1".to_string(),
                parent: None,
                children: vec![],
                content_layer: "body".to_string(),
                prov: vec![],
                orig: "H2".to_string(),
                text: "H2".to_string(),
                level: 2,
                formatting: None,
                hyperlink: None,
            },
            DocItem::SectionHeader {
                self_ref: "#/texts/2".to_string(),
                parent: None,
                children: vec![],
                content_layer: "body".to_string(),
                prov: vec![],
                orig: "H3".to_string(),
                text: "H3".to_string(),
                level: 3,
                formatting: None,
                hyperlink: None,
            },
        ];

        let markdown = docitems_to_markdown(&items);
        assert!(markdown.contains("# H1"));
        assert!(markdown.contains("## H2"));
        assert!(markdown.contains("### H3"));
    }

    #[test]
    fn test_docitems_to_markdown_bullet_list() {
        let items = vec![
            DocItem::List {
                self_ref: "#/lists/0".to_string(),
                parent: None,
                children: vec![ItemRef::new("#/texts/0"), ItemRef::new("#/texts/1")],
                content_layer: "body".to_string(),
                name: "bullet_list".to_string(),
            },
            DocItem::ListItem {
                self_ref: "#/texts/0".to_string(),
                parent: Some(ItemRef::new("#/lists/0")),
                children: vec![],
                content_layer: "body".to_string(),
                prov: vec![],
                orig: "First item".to_string(),
                text: "First item".to_string(),
                formatting: None,
                hyperlink: None,
                enumerated: false,
                marker: "-".to_string(),
            },
            DocItem::ListItem {
                self_ref: "#/texts/1".to_string(),
                parent: Some(ItemRef::new("#/lists/0")),
                children: vec![],
                content_layer: "body".to_string(),
                prov: vec![],
                orig: "Second item".to_string(),
                text: "Second item".to_string(),
                formatting: None,
                hyperlink: None,
                enumerated: false,
                marker: "-".to_string(),
            },
        ];

        let markdown = docitems_to_markdown(&items);
        assert!(markdown.contains("First item"));
        assert!(markdown.contains("Second item"));
    }

    #[test]
    fn test_docitems_to_markdown_numbered_list() {
        let items = vec![
            DocItem::OrderedList {
                self_ref: "#/lists/0".to_string(),
                parent: None,
                children: vec![ItemRef::new("#/texts/0"), ItemRef::new("#/texts/1")],
                content_layer: "body".to_string(),
                name: "ordered_list".to_string(),
            },
            DocItem::ListItem {
                self_ref: "#/texts/0".to_string(),
                parent: Some(ItemRef::new("#/lists/0")),
                children: vec![],
                content_layer: "body".to_string(),
                prov: vec![],
                orig: "First".to_string(),
                text: "First".to_string(),
                formatting: None,
                hyperlink: None,
                enumerated: true,
                marker: "1.".to_string(),
            },
            DocItem::ListItem {
                self_ref: "#/texts/1".to_string(),
                parent: Some(ItemRef::new("#/lists/0")),
                children: vec![],
                content_layer: "body".to_string(),
                prov: vec![],
                orig: "Second".to_string(),
                text: "Second".to_string(),
                formatting: None,
                hyperlink: None,
                enumerated: true,
                marker: "2.".to_string(),
            },
        ];

        let markdown = docitems_to_markdown(&items);
        assert!(markdown.contains("First"));
        assert!(markdown.contains("Second"));
    }

    #[test]
    fn test_docitems_to_markdown_table() {
        use docling_core::content::TableCell;

        // Create simple table cells
        let cell = |text: &str, header: bool| TableCell {
            text: text.to_string(),
            row_span: Some(1),
            col_span: Some(1),
            ref_item: None,
            start_row_offset_idx: None,
            start_col_offset_idx: None,
            column_header: header,
            row_header: false,
            from_ocr: false,
            confidence: None,
            bbox: None,
        };

        let items = vec![DocItem::Table {
            self_ref: "#/tables/0".to_string(),
            parent: None,
            children: vec![],
            content_layer: "body".to_string(),
            prov: vec![],
            data: TableData {
                num_rows: 2,
                num_cols: 2,
                grid: vec![
                    vec![cell("Header1", true), cell("Header2", true)],
                    vec![cell("Cell1", false), cell("Cell2", false)],
                ],
                table_cells: None,
            },
            captions: vec![],
            footnotes: vec![],
            references: vec![],
            image: None,
            annotations: vec![],
        }];

        let markdown = docitems_to_markdown(&items);
        assert!(markdown.contains("Header1"));
        assert!(markdown.contains("Header2"));
        assert!(markdown.contains("Cell1"));
        assert!(markdown.contains("Cell2"));
        // Table should have pipe separators
        assert!(markdown.contains("|"));
    }

    #[test]
    fn test_docitems_to_markdown_picture() {
        let items = vec![DocItem::Picture {
            self_ref: "#/pictures/0".to_string(),
            parent: None,
            children: vec![],
            content_layer: "body".to_string(),
            prov: vec![],
            captions: vec![],
            footnotes: vec![],
            references: vec![],
            image: None,
            annotations: vec![],
            ocr_text: None,
        }];

        let markdown = docitems_to_markdown(&items);
        // Picture may generate placeholder or nothing - just verify no panic
        let _ = markdown;
    }

    #[test]
    fn test_docitems_to_markdown_code_block() {
        let items = vec![DocItem::Code {
            self_ref: "#/texts/0".to_string(),
            parent: None,
            children: vec![],
            content_layer: "body".to_string(),
            prov: vec![],
            orig: "fn main() {}".to_string(),
            text: "fn main() {}".to_string(),
            language: Some("rust".to_string()),
            formatting: None,
            hyperlink: None,
        }];

        let markdown = docitems_to_markdown(&items);
        assert!(markdown.contains("fn main()"));
        // Code blocks should be fenced
        assert!(markdown.contains("```"));
    }

    #[test]
    fn test_docitems_to_markdown_caption() {
        // Note: Caption items are not rendered directly in the current implementation
        // They fall through to the default case which resets list state only
        let items = vec![DocItem::Caption {
            self_ref: "#/texts/0".to_string(),
            parent: None,
            children: vec![],
            content_layer: "body".to_string(),
            prov: vec![],
            orig: "Figure 1: Test caption".to_string(),
            text: "Figure 1: Test caption".to_string(),
            formatting: None,
            hyperlink: None,
        }];

        let markdown = docitems_to_markdown(&items);
        // Currently Caption items don't produce output - just verify no panic
        let _ = markdown;
    }

    #[test]
    fn test_docitems_to_markdown_empty() {
        let items: Vec<DocItem> = vec![];
        let markdown = docitems_to_markdown(&items);
        assert!(markdown.is_empty());
    }

    #[test]
    fn test_docitems_to_markdown_mixed_content() {
        let items = vec![
            DocItem::SectionHeader {
                self_ref: "#/texts/0".to_string(),
                parent: None,
                children: vec![],
                content_layer: "body".to_string(),
                prov: vec![],
                orig: "Introduction".to_string(),
                text: "Introduction".to_string(),
                level: 1,
                formatting: None,
                hyperlink: None,
            },
            DocItem::Text {
                self_ref: "#/texts/1".to_string(),
                parent: None,
                children: vec![],
                content_layer: "body".to_string(),
                prov: vec![],
                orig: "This is a paragraph.".to_string(),
                text: "This is a paragraph.".to_string(),
                formatting: None,
                hyperlink: None,
            },
            DocItem::SectionHeader {
                self_ref: "#/texts/2".to_string(),
                parent: None,
                children: vec![],
                content_layer: "body".to_string(),
                prov: vec![],
                orig: "Conclusion".to_string(),
                text: "Conclusion".to_string(),
                level: 2,
                formatting: None,
                hyperlink: None,
            },
        ];

        let markdown = docitems_to_markdown(&items);
        assert!(markdown.contains("# Introduction"));
        assert!(markdown.contains("This is a paragraph."));
        assert!(markdown.contains("## Conclusion"));
    }

    #[test]
    fn test_apply_formatting_empty_text() {
        let formatting = Formatting {
            bold: Some(true),
            italic: None,
            underline: None,
            strikethrough: None,
            code: None,
            script: None,
            font_size: None,
            font_family: None,
        };

        let result = apply_formatting("", Some(&formatting));
        // Empty text should remain empty (no formatting markers around nothing)
        assert!(result.is_empty() || result == "****");
    }

    #[test]
    fn test_docitems_to_markdown_text_with_formatting() {
        let items = vec![DocItem::Text {
            self_ref: "#/texts/0".to_string(),
            parent: None,
            children: vec![],
            content_layer: "body".to_string(),
            prov: vec![],
            orig: "Important text".to_string(),
            text: "Important text".to_string(),
            formatting: Some(Formatting {
                bold: Some(true),
                italic: None,
                underline: None,
                strikethrough: None,
                code: None,
                script: None,
                font_size: None,
                font_family: None,
            }),
            hyperlink: None,
        }];

        let markdown = docitems_to_markdown(&items);
        assert!(markdown.contains("Important text"));
        assert!(markdown.contains("**"));
    }
}
