//! Type conversions from PDF ML types to docling-core DocItem types
//!
//! This module provides functions to convert internal PDF ML data structures
//! (from the `types` module) into the standardized docling-core types used
//! for serialization and output.
//!
//! # Examples
//!
//! ```rust,ignore
//! use docling_pdf_ml::convert::*;
//! use docling_pdf_ml::types::*;
//!
//! let cluster = Cluster {
//!     id: 0,
//!     label: DocItemLabel::Title,
//!     bbox: BoundingBox { /* ... */ },
//!     confidence: 0.95,
//!     cells: vec![],
//!     children: vec![],
//! };
//!
//! let doc_item = cluster_to_doc_item(&cluster, "body", 0);
//! ```

use crate::pipeline::data_structures::{
    BoundingBox as MlBoundingBox, Cluster, CoordOrigin as MlCoordOrigin,
    DocItemLabel as MlDocItemLabel, FigureElement, PageElement, TableElement, TextElement,
};
use docling_core::{BoundingBox, CoordOrigin, DocItem, Formatting, ProvenanceItem};
use once_cell::sync::Lazy;
use regex::Regex;
use std::fmt::Write;

/// Detect header level from text content and label
///
/// Uses the following heuristics:
/// 1. Title label → level 0 (H1 in markdown)
/// 2. "Abstract" or similar top-level sections → level 1 (H2)
/// 3. Single number like "1 Introduction" → level 1 (H2)
/// 4. Any multi-part number like "1.1 Methods" → level 1 (H2)
/// 5. Any deeper numbering like "1.1.1 Details" → level 1 (H2)
/// 6. Default → level 1 (H2)
///
/// # Arguments
/// * `text` - The header text content
/// * `label` - The ML-detected label (Title vs SectionHeader)
/// * `page_no` - Page number (0-indexed) for position-based heuristics
///
/// # Returns
/// Header level (0 = H1, 1 = H2, 2 = H3, etc.)
#[must_use = "returns the detected header level"]
pub fn detect_header_level(text: &str, label: MlDocItemLabel, page_no: usize) -> usize {
    // Title label is always H1
    if label == MlDocItemLabel::Title {
        return 0;
    }

    // Top-level sections that should be H2 (level 1)
    static TOP_LEVEL_SECTIONS: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"(?i)^(abstract|introduction|conclusion|references|bibliography|acknowledgments?|appendix|related\s+work|background|discussion|results|methods?|methodology|experimental?\s+setup|evaluation)$")
            .expect("Invalid top-level sections regex")
    });

    // Numbered section patterns
    // Level 1 (H2): "1 Introduction", "1. Methods", "A Background"
    static LEVEL_1_PATTERN: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"^([0-9]+|[A-Z])\.?\s+\S").expect("Invalid level 1 pattern regex")
    });

    // Level 2 (H3): "1.1 Subsection", "1.2 Another"
    static LEVEL_2_PATTERN: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"^[0-9]+\.[0-9]+\.?\s+\S").expect("Invalid level 2 pattern regex")
    });

    // Level 3 (H4): "1.1.1 Sub-subsection"
    static LEVEL_3_PATTERN: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"^[0-9]+\.[0-9]+\.[0-9]+\.?\s+\S").expect("Invalid level 3 pattern regex")
    });

    // Level 4 (H5): "1.1.1.1 Deep section"
    static LEVEL_4_PATTERN: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+\.?\s+\S")
            .expect("Invalid level 4 pattern regex")
    });

    let trimmed = text.trim();

    // Check numbering patterns.
    //
    // Python baseline renders numbered section headers as H2 regardless of dot depth:
    // - "1 Introduction" -> ## ...
    // - "5.1 Hyper Parameter Optimization" -> ## ...
    // - "3.2.2 Built-in global variables" -> ## ...
    //
    // Preserve deeper nesting for non-PDF formats elsewhere; this function is PDF-specific.
    if LEVEL_4_PATTERN.is_match(trimmed)
        || LEVEL_3_PATTERN.is_match(trimmed)
        || LEVEL_2_PATTERN.is_match(trimmed)
        || LEVEL_1_PATTERN.is_match(trimmed)
    {
        return 1;
    }

    // Check for top-level unnumbered sections (like "Abstract")
    if TOP_LEVEL_SECTIONS.is_match(trimmed) {
        return 1;
    }

    // Position heuristic: first header on page 0 might be title
    // (This is a fallback - if it's the first section header and on page 0,
    // it might be a paper title that the model missed)
    if page_no == 0 {
        // Could potentially return 0 (H1) here for first header,
        // but safer to default to H2 unless we have more context
        return 1;
    }

    // Default to level 1 (H2)
    1
}

/// Convert PDF ML `CoordOrigin` to docling-core `CoordOrigin`
#[inline]
#[must_use = "returns the docling-core coordinate origin"]
pub const fn convert_coord_origin(origin: MlCoordOrigin) -> CoordOrigin {
    match origin {
        MlCoordOrigin::TopLeft => CoordOrigin::TopLeft,
        MlCoordOrigin::BottomLeft => CoordOrigin::BottomLeft,
    }
}

/// Convert PDF ML `BBox` to docling-core `BoundingBox`
#[inline]
#[must_use = "returns the docling-core bounding box"]
pub fn convert_bbox(bbox: &MlBoundingBox) -> BoundingBox {
    BoundingBox {
        l: f64::from(bbox.l),
        t: f64::from(bbox.t),
        r: f64::from(bbox.r),
        b: f64::from(bbox.b),
        coord_origin: convert_coord_origin(bbox.coord_origin),
    }
}

/// N=4373: Build Formatting from bold/italic flags
///
/// Returns `Some(Formatting)` if either bold or italic is true, `None` otherwise.
#[inline]
fn build_formatting(is_bold: bool, is_italic: bool) -> Option<Formatting> {
    if !is_bold && !is_italic {
        return None;
    }
    Some(Formatting {
        bold: is_bold.then_some(true),
        italic: is_italic.then_some(true),
        underline: None,
        strikethrough: None,
        code: None,
        script: None,
        font_size: None,
        font_family: None,
    })
}

/// Convert `TextElement` to docling-core `DocItem`
///
/// `TextElements` include Text, `SectionHeader`, Title, Caption, Footnote, `ListItem`, Code, etc.
#[must_use = "returns a DocItem for the text element"]
#[allow(
    clippy::match_same_arms,
    reason = "explicit label listing + catch-all for clarity and safety"
)]
#[allow(clippy::too_many_lines)]
pub fn text_element_to_doc_item(element: &TextElement, _page_num: usize) -> DocItem {
    let self_ref = format!("doc_{}_{}", element.page_no, element.id);
    let content_layer = "body".to_string();
    let formatting = build_formatting(element.is_bold, element.is_italic);

    // Create provenance with bounding box
    let prov = vec![ProvenanceItem {
        page_no: element.page_no,
        bbox: convert_bbox(&element.cluster.bbox),
        charspan: None,
    }];

    match element.label {
        MlDocItemLabel::Text
        | MlDocItemLabel::Caption
        | MlDocItemLabel::Footnote
        | MlDocItemLabel::Formula => DocItem::Text {
            self_ref,
            parent: None,
            children: vec![],
            content_layer,
            prov,
            orig: element.orig.clone(),
            text: element.text.clone(),
            formatting, // N=4373: Pass bold/italic formatting
            hyperlink: None,
        },
        MlDocItemLabel::SectionHeader | MlDocItemLabel::Title => DocItem::SectionHeader {
            self_ref,
            parent: None,
            children: vec![],
            content_layer,
            prov,
            orig: element.orig.clone(),
            text: element.text.clone(),
            level: detect_header_level(&element.text, element.label, element.page_no),
            formatting, // N=4373: Pass bold/italic formatting
            hyperlink: None,
        },
        MlDocItemLabel::ListItem => DocItem::ListItem {
            self_ref,
            parent: None,
            children: vec![],
            content_layer,
            prov,
            orig: element.orig.clone(),
            text: element.text.clone(),
            enumerated: false, // Default, should be determined by analysis
            marker: "-".to_string(),
            formatting, // N=4373: Pass bold/italic formatting
            hyperlink: None,
        },
        MlDocItemLabel::Code => DocItem::Code {
            self_ref,
            parent: None,
            children: vec![],
            content_layer,
            prov,
            orig: element.orig.clone(),
            text: element.text.clone(),
            language: None,
            formatting, // N=4373: Pass bold/italic formatting
            hyperlink: None,
        },
        MlDocItemLabel::PageHeader => DocItem::PageHeader {
            self_ref,
            parent: None,
            children: vec![],
            content_layer,
            prov,
            orig: element.orig.clone(),
            text: element.text.clone(),
            formatting, // N=4373: Pass bold/italic formatting
            hyperlink: None,
        },
        MlDocItemLabel::PageFooter => DocItem::PageFooter {
            self_ref,
            parent: None,
            children: vec![],
            content_layer,
            prov,
            orig: element.orig.clone(),
            text: element.text.clone(),
            formatting, // N=4373: Pass bold/italic formatting
            hyperlink: None,
        },
        MlDocItemLabel::CheckboxSelected => DocItem::Text {
            self_ref,
            parent: None,
            children: vec![],
            content_layer,
            prov,
            orig: "[x]".to_string(),
            text: "[x]".to_string(),
            formatting, // N=4373: Pass bold/italic formatting (unlikely for checkboxes)
            hyperlink: None,
        },
        MlDocItemLabel::CheckboxUnselected => DocItem::Text {
            self_ref,
            parent: None,
            children: vec![],
            content_layer,
            prov,
            orig: "[ ]".to_string(),
            text: "[ ]".to_string(),
            formatting, // N=4373: Pass bold/italic formatting (unlikely for checkboxes)
            hyperlink: None,
        },
        _ => DocItem::Text {
            // Fallback for any unhandled labels
            self_ref,
            parent: None,
            children: vec![],
            content_layer,
            prov,
            orig: element.orig.clone(),
            text: element.text.clone(),
            formatting, // N=4373: Pass bold/italic formatting
            hyperlink: None,
        },
    }
}

/// Convert `TableElement` to docling-core `DocItem`
///
/// Tables are converted to the Table variant with structured grid data
#[must_use = "returns a DocItem for the table element"]
pub fn table_element_to_doc_item(element: &TableElement, _page_num: usize) -> DocItem {
    let self_ref = format!("doc_{}_{}", element.page_no, element.id);
    let content_layer = "body".to_string();

    // Create provenance with bounding box
    let prov = vec![ProvenanceItem {
        page_no: element.page_no,
        bbox: convert_bbox(&element.cluster.bbox),
        charspan: None,
    }];

    // Convert table cells
    // Issue #1 FIX: Preserve header flags and OCR metadata
    // Issue #2 FIX: Preserve cell-level bbox
    let cells: Vec<docling_core::TableCell> = element
        .table_cells
        .iter()
        .map(|cell| docling_core::TableCell {
            text: cell.text.clone(),
            row_span: Some(cell.row_span),
            col_span: Some(cell.col_span),
            ref_item: None,
            start_row_offset_idx: Some(cell.start_row_offset_idx),
            start_col_offset_idx: Some(cell.start_col_offset_idx),
            column_header: cell.column_header,
            row_header: cell.row_header,
            from_ocr: cell.from_ocr,
            confidence: None,
            bbox: Some(convert_bbox(&cell.bbox)),
        })
        .collect();

    // Build grid from cells - MarkdownSerializer requires non-empty grid!
    // Grid is row-major: grid[row_idx][col_idx]
    let num_rows = element.num_rows;
    let num_cols = element.num_cols;

    // Initialize grid with empty cells
    let mut grid: Vec<Vec<docling_core::TableCell>> = Vec::with_capacity(num_rows);
    for _ in 0..num_rows {
        let row: Vec<docling_core::TableCell> = (0..num_cols)
            .map(|_| docling_core::TableCell {
                text: String::new(),
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
            })
            .collect();
        grid.push(row);
    }

    // Place cells into grid at their positions
    // Handle spans by replicating cells (Python does this too)
    for cell in &cells {
        let start_row = cell.start_row_offset_idx.unwrap_or(0);
        let start_col = cell.start_col_offset_idx.unwrap_or(0);
        let row_span = cell.row_span.unwrap_or(1);
        let col_span = cell.col_span.unwrap_or(1);

        // Place cell at start position and replicate for spans
        for r in start_row..(start_row + row_span).min(num_rows) {
            for c in start_col..(start_col + col_span).min(num_cols) {
                if r < grid.len() && c < grid[r].len() {
                    grid[r][c] = cell.clone();
                }
            }
        }
    }

    // Create table data structure with populated grid
    let data = docling_core::TableData {
        grid,
        num_rows,
        num_cols,
        table_cells: Some(cells),
    };

    DocItem::Table {
        self_ref,
        parent: None,
        children: vec![],
        content_layer,
        prov,
        data,
        captions: vec![],
        footnotes: vec![],
        references: vec![],
        image: None,
        annotations: vec![],
    }
}

/// Convert `FigureElement` to docling-core `DocItem`
///
/// Figures/Pictures are converted to the Picture variant with optional children for embedded text
#[must_use = "returns a DocItem for the figure element"]
pub fn figure_element_to_doc_item(element: &FigureElement, _page_num: usize) -> DocItem {
    let self_ref = format!("doc_{}_{}", element.page_no, element.id);
    let content_layer = "body".to_string();

    // Create provenance with bounding box
    let prov = vec![ProvenanceItem {
        page_no: element.page_no,
        bbox: convert_bbox(&element.cluster.bbox),
        charspan: None,
    }];

    DocItem::Picture {
        self_ref,
        parent: None,
        children: vec![],
        content_layer,
        prov,
        captions: vec![],
        footnotes: vec![],
        references: vec![],
        image: None,
        annotations: vec![],
        ocr_text: None, // Will be populated by orphan cell processing
    }
}

/// Convert `PageElement` to docling-core `DocItem`
///
/// This is the main conversion function for assembled pipeline output
#[inline]
#[must_use = "returns a DocItem for the page element"]
pub fn page_element_to_doc_item(element: &PageElement, page_num: usize) -> DocItem {
    match element {
        PageElement::Text(text_elem) => text_element_to_doc_item(text_elem, page_num),
        PageElement::Table(table_elem) => table_element_to_doc_item(table_elem, page_num),
        PageElement::Figure(fig_elem) => figure_element_to_doc_item(fig_elem, page_num),
        PageElement::Container(_container_elem) => {
            // Container elements are currently treated as text
            // In future, may need hierarchical document structure support
            DocItem::Text {
                self_ref: format!("doc_{}_{}", page_num, 0),
                parent: None,
                children: vec![],
                content_layer: "body".to_string(),
                prov: vec![],
                orig: String::new(),
                text: String::new(),
                formatting: None,
                hyperlink: None,
            }
        }
    }
}

/// Convert Page with `AssembledUnit` to `Vec<DocItem>`
///
/// This converts a fully assembled page (after pipeline processing) to `DocItems`.
/// Respects reading order from the assembled unit.
#[must_use = "returns DocItems for all elements on the page"]
pub fn page_to_doc_items(page: &crate::pipeline::data_structures::Page) -> Vec<DocItem> {
    let Some(assembled) = &page.assembled else {
        return vec![];
    };

    assembled
        .elements
        .iter()
        .map(|element| page_element_to_doc_item(element, page.page_no))
        .collect()
}

/// Convert multiple pages to `Vec<DocItem>`
///
/// Combines `DocItems` from multiple pages into a single flat list
#[must_use = "this function returns DocItems that should be processed"]
pub fn pages_to_doc_items(pages: &[crate::pipeline::data_structures::Page]) -> Vec<DocItem> {
    pages.iter().flat_map(page_to_doc_items).collect()
}

/// Convert Cluster to docling-core `DocItem`
///
/// This is a lower-level conversion for direct cluster output.
/// For assembled pipeline output, use `text_element_to_doc_item` or `table_element_to_doc_item`.
#[must_use = "returns a DocItem for the cluster"]
pub fn cluster_to_doc_item(cluster: &Cluster, content_layer: &str, page_num: usize) -> DocItem {
    let self_ref = format!("cluster_{}", cluster.id);

    // Extract text from cells
    let text: String = cluster
        .cells
        .iter()
        .map(|cell| cell.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");

    // Create provenance with bounding box
    let prov = vec![ProvenanceItem {
        page_no: page_num,
        bbox: convert_bbox(&cluster.bbox),
        charspan: None,
    }];

    // Map label to appropriate DocItem variant
    match cluster.label {
        MlDocItemLabel::SectionHeader | MlDocItemLabel::Title => DocItem::SectionHeader {
            self_ref,
            parent: None,
            children: vec![],
            content_layer: content_layer.to_string(),
            prov,
            orig: text.clone(),
            text: text.clone(),
            level: detect_header_level(&text, cluster.label, page_num),
            formatting: None,
            hyperlink: None,
        },
        MlDocItemLabel::Table | MlDocItemLabel::DocumentIndex => DocItem::Table {
            self_ref,
            parent: None,
            children: vec![],
            content_layer: content_layer.to_string(),
            prov,
            data: docling_core::TableData {
                grid: vec![],
                num_rows: 0,
                num_cols: 0,
                table_cells: None,
            },
            captions: vec![],
            footnotes: vec![],
            references: vec![],
            image: None,
            annotations: vec![],
        },
        MlDocItemLabel::Picture | MlDocItemLabel::Figure => {
            // Extract OCR text from figure content (charts, diagrams, etc.)
            // This preserves valuable figure content that Python discards
            let ocr_text = if text.trim().is_empty() {
                None
            } else {
                Some(text.trim().to_string())
            };

            DocItem::Picture {
                self_ref,
                parent: None,
                children: vec![],
                content_layer: content_layer.to_string(),
                prov,
                captions: vec![],
                footnotes: vec![],
                references: vec![],
                image: None,
                annotations: vec![],
                ocr_text,
            }
        }
        // Text and other labels mapped to Text
        _ => DocItem::Text {
            self_ref,
            parent: None,
            children: vec![],
            content_layer: content_layer.to_string(),
            prov,
            orig: text.clone(),
            text,
            formatting: None,
            hyperlink: None,
        },
    }
}

/// Convert a list of Clusters to `Vec<DocItem>`
#[inline]
#[must_use = "returns DocItems for all clusters"]
pub fn clusters_to_doc_items(
    clusters: &[Cluster],
    content_layer: &str,
    page_num: usize,
) -> Vec<DocItem> {
    clusters
        .iter()
        .map(|cluster| cluster_to_doc_item(cluster, content_layer, page_num))
        .collect()
}

/// Render `TableData` to markdown table format
///
/// Converts a table grid to markdown with proper column alignment
#[must_use = "returns the markdown representation of the table"]
fn render_table_markdown(data: &docling_core::TableData) -> String {
    if data.grid.is_empty() || data.num_cols == 0 {
        return String::new();
    }

    // Calculate column widths
    let num_cols = data.num_cols;
    let mut col_widths = vec![3usize; num_cols]; // Minimum width of 3

    for row in &data.grid {
        for (col_idx, cell) in row.iter().take(num_cols).enumerate() {
            let cell_len = cell.text.chars().count();
            col_widths[col_idx] = col_widths[col_idx].max(cell_len);
        }
    }

    let mut result = String::new();

    // Render rows
    for (row_idx, row) in data.grid.iter().enumerate() {
        result.push('|');
        for (col_idx, cell) in row.iter().take(num_cols).enumerate() {
            let width = col_widths[col_idx];
            let text = &cell.text;
            let _ = write!(result, " {text:width$} |");
        }
        // Fill missing columns
        for &width in col_widths.iter().skip(row.len()) {
            let _ = write!(result, " {:width$} |", "", width = width);
        }
        result.push('\n');

        // Add separator after header row
        if row_idx == 0 {
            result.push('|');
            for &width in &col_widths {
                result.push_str(&"-".repeat(width + 2));
                result.push('|');
            }
            result.push('\n');
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

/// Detect if text is body content misclassified as section header
///
/// N=4152: The layout model sometimes detects split reference citations as
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

    // N=4321: Text with "PERCENT" or "AGENTS" is likely government document body text
    // These are data patterns, not header patterns
    let upper = trimmed.to_uppercase();
    if upper.contains("PERCENT") || upper.contains("AGENTS AT") {
        return true;
    }

    // N=4321: Text ending with a colon followed by whitespace suggests a label, not header
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
    if is_date_pattern(trimmed) {
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

/// Export `DocItems` to markdown format
///
/// This is a simple markdown export for testing. Production code should use
/// docling-core's `MarkdownSerializer` with full `DoclingDocument` structure.
#[must_use = "returns the markdown representation of the DocItems"]
pub fn export_to_markdown(doc_items: &[DocItem]) -> String {
    let mut output = String::new();

    for item in doc_items {
        match item {
            DocItem::Text { text, .. } => {
                output.push_str(text);
                output.push_str("\n\n");
            }
            DocItem::SectionHeader { text, level, .. } => {
                // N=4152: Detect reference content misclassified as section header
                if is_fake_section_header(text) {
                    // Render as plain text instead of heading
                    output.push_str(text);
                    output.push_str("\n\n");
                } else {
                    let prefix = "#".repeat(*level);
                    let _ = writeln!(output, "{prefix} {text}\n");
                }
            }
            DocItem::Table { data, .. } => {
                // Render table as markdown
                output.push_str(&render_table_markdown(data));
                output.push_str("\n\n");
            }
            DocItem::Picture { ocr_text, .. } => {
                // Render Picture with standard marker (matches Python docling)
                output.push_str("<!-- image -->\n\n");

                // Render OCR text from figure content as actual paragraph content
                // Python docling renders OCR text as visible content, not comments
                if let Some(text) = ocr_text {
                    let trimmed = text.trim();
                    if !trimmed.is_empty() {
                        output.push_str(trimmed);
                        output.push_str("\n\n");
                    }
                }
            }
            DocItem::ListItem { text, marker, .. } => {
                let _ = writeln!(output, "{marker} {text}");
            }
            DocItem::Code { text, language, .. } => {
                if let Some(lang) = language {
                    let _ = writeln!(output, "```{lang}\n{text}\n```\n");
                } else {
                    let _ = writeln!(output, "```\n{text}\n```\n");
                }
            }
            DocItem::PageHeader { text, .. } => {
                let _ = writeln!(output, "--- Header: {text} ---\n");
            }
            DocItem::PageFooter { text, .. } => {
                let _ = writeln!(output, "--- Footer: {text} ---\n");
            }
            _ => {
                // Other variants - basic text extraction
                if let Some(text) = item.text() {
                    output.push_str(text);
                    output.push_str("\n\n");
                }
            }
        }
    }

    // N=4373: Linkify plain URLs in output
    linkify_urls(&output)
}

/// N=4373: Convert plain URLs to markdown links
///
/// Converts `https://example.com` to `[https://example.com](https://example.com)`.
/// Only linkifies URLs that are not already part of a markdown link.
fn linkify_urls(text: &str) -> String {
    static URL_PATTERN: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"https?://[^\s<>]+").expect("Invalid URL regex"));

    // Check if we have any URLs to process
    if !text.contains("http://") && !text.contains("https://") {
        return text.to_string();
    }

    // Skip if text already contains markdown links (to avoid double-linking)
    if text.contains("](http") {
        return text.to_string();
    }

    // Replace URLs with markdown links, preserving trailing punctuation
    URL_PATTERN
        .replace_all(text, |caps: &regex::Captures| {
            let full_match = &caps[0];
            let url = full_match.trim_end_matches(|c| {
                matches!(
                    c,
                    '.' | ',' | ';' | ':' | '!' | '?' | '\'' | '"' | ')' | ']'
                )
            });
            // Preserve any trailing punctuation that was trimmed
            let trailing = &full_match[url.len()..];
            format!("[{url}]({url}){trailing}")
        })
        .into_owned()
}

/// Export `DocItems` to JSON format
///
/// Serializes `DocItems` to JSON for structured output and validation
#[must_use = "this function returns JSON data that should be used or written"]
pub fn export_to_json(doc_items: &[DocItem]) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(doc_items)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::data_structures::{
        BoundingBox as MlBoundingBox, CoordOrigin as MlCoordOrigin, DocItemLabel as MlDocItemLabel,
    };

    #[test]
    fn test_convert_coord_origin() {
        assert_eq!(
            convert_coord_origin(MlCoordOrigin::TopLeft),
            CoordOrigin::TopLeft
        );
        assert_eq!(
            convert_coord_origin(MlCoordOrigin::BottomLeft),
            CoordOrigin::BottomLeft
        );
    }

    #[test]
    fn test_convert_bbox() {
        let ml_bbox = MlBoundingBox {
            l: 10.0,
            t: 20.0,
            r: 100.0,
            b: 50.0,
            coord_origin: MlCoordOrigin::TopLeft,
        };

        let core_bbox = convert_bbox(&ml_bbox);

        assert_eq!(core_bbox.l, 10.0);
        assert_eq!(core_bbox.t, 20.0);
        assert_eq!(core_bbox.r, 100.0);
        assert_eq!(core_bbox.b, 50.0);
        assert_eq!(core_bbox.coord_origin, CoordOrigin::TopLeft);
    }

    #[test]
    fn test_cluster_to_doc_item_text() {
        let cluster = Cluster {
            id: 1,
            label: MlDocItemLabel::Text,
            bbox: MlBoundingBox {
                l: 0.0,
                t: 0.0,
                r: 100.0,
                b: 50.0,
                coord_origin: MlCoordOrigin::TopLeft,
            },
            confidence: 0.95,
            cells: vec![],
            children: vec![],
        };

        let doc_item = cluster_to_doc_item(&cluster, "body", 0);

        match doc_item {
            DocItem::Text { self_ref, text, .. } => {
                assert_eq!(self_ref, "cluster_1");
                assert_eq!(text, ""); // No cells, so empty text
            }
            _ => panic!("Expected DocItem::Text"),
        }
    }

    #[test]
    fn test_cluster_to_doc_item_section_header() {
        let cluster = Cluster {
            id: 2,
            label: MlDocItemLabel::SectionHeader,
            bbox: MlBoundingBox {
                l: 0.0,
                t: 0.0,
                r: 200.0,
                b: 30.0,
                coord_origin: MlCoordOrigin::TopLeft,
            },
            confidence: 0.98,
            cells: vec![],
            children: vec![],
        };

        let doc_item = cluster_to_doc_item(&cluster, "body", 0);

        match doc_item {
            DocItem::SectionHeader {
                self_ref, level, ..
            } => {
                assert_eq!(self_ref, "cluster_2");
                assert_eq!(level, 1);
            }
            _ => panic!("Expected DocItem::SectionHeader"),
        }
    }

    #[test]
    fn test_cluster_to_doc_item_table() {
        let cluster = Cluster {
            id: 3,
            label: MlDocItemLabel::Table,
            bbox: MlBoundingBox {
                l: 0.0,
                t: 0.0,
                r: 400.0,
                b: 200.0,
                coord_origin: MlCoordOrigin::TopLeft,
            },
            confidence: 0.92,
            cells: vec![],
            children: vec![],
        };

        let doc_item = cluster_to_doc_item(&cluster, "body", 0);

        match doc_item {
            DocItem::Table { self_ref, data, .. } => {
                assert_eq!(self_ref, "cluster_3");
                assert_eq!(data.num_rows, 0);
                assert_eq!(data.num_cols, 0);
            }
            _ => panic!("Expected DocItem::Table"),
        }
    }

    #[test]
    fn test_page_element_to_doc_item_text() {
        let text_elem = TextElement {
            label: MlDocItemLabel::Text,
            id: 0,
            page_no: 0,
            text: "Test text".to_string(),
            orig: "Test text".to_string(),
            cluster: Cluster {
                id: 0,
                label: MlDocItemLabel::Text,
                bbox: MlBoundingBox {
                    l: 0.0,
                    t: 0.0,
                    r: 100.0,
                    b: 50.0,
                    coord_origin: MlCoordOrigin::TopLeft,
                },
                confidence: 0.95,
                cells: vec![],
                children: vec![],
            },
            captions: vec![],
            footnotes: vec![],
            is_bold: false,
            is_italic: false,
        };

        let doc_item = page_element_to_doc_item(&PageElement::Text(text_elem), 0);

        match doc_item {
            DocItem::Text { text, .. } => {
                assert_eq!(text, "Test text");
            }
            _ => panic!("Expected DocItem::Text"),
        }
    }

    #[test]
    fn test_export_to_markdown_basic() {
        let doc_items = vec![
            DocItem::SectionHeader {
                self_ref: "h1".to_string(),
                parent: None,
                children: vec![],
                content_layer: "body".to_string(),
                prov: vec![],
                orig: "Title".to_string(),
                text: "Title".to_string(),
                level: 1,
                formatting: None,
                hyperlink: None,
            },
            DocItem::Text {
                self_ref: "p1".to_string(),
                parent: None,
                children: vec![],
                content_layer: "body".to_string(),
                prov: vec![],
                orig: "Paragraph text".to_string(),
                text: "Paragraph text".to_string(),
                formatting: None,
                hyperlink: None,
            },
        ];

        let markdown = export_to_markdown(&doc_items);

        assert!(
            markdown.contains("# Title"),
            "Markdown should contain heading"
        );
        assert!(
            markdown.contains("Paragraph text"),
            "Markdown should contain paragraph text"
        );
    }

    #[test]
    fn test_export_to_json() {
        let doc_items = vec![DocItem::Text {
            self_ref: "t1".to_string(),
            parent: None,
            children: vec![],
            content_layer: "body".to_string(),
            prov: vec![],
            orig: "Test".to_string(),
            text: "Test".to_string(),
            formatting: None,
            hyperlink: None,
        }];

        let json = export_to_json(&doc_items).expect("JSON serialization failed");

        assert!(json.contains("Test"), "JSON should contain text content");
        assert!(json.contains("text"), "JSON should contain text item type");
    }

    #[test]
    fn test_is_fake_section_header() {
        // N=4152: Test detection of reference content misclassified as section headers

        // Real section headers - should NOT be filtered
        assert!(
            !is_fake_section_header("1. Introduction"),
            "Real section header should not be filtered"
        );
        assert!(
            !is_fake_section_header("2 Related Work"),
            "Real section header should not be filtered"
        );
        assert!(
            !is_fake_section_header("Abstract"),
            "Real section header should not be filtered"
        );

        // Fake section headers (reference content) - SHOULD be filtered
        assert!(
            is_fake_section_header("1873. IEEE (2022)"),
            "Reference year+publisher should be filtered"
        );
        assert!(
            is_fake_section_header("2019. Proceedings of CVPR"),
            "Reference year+proceedings should be filtered"
        );
        assert!(
            is_fake_section_header("2022, pp. 123-456"),
            "Reference year+pages should be filtered"
        );

        // N=4321: Test detection of scanned document body text misclassified as section headers

        // Long text is body content, not header
        assert!(
            is_fake_section_header(
                "SENATE SELECT COMMITTEE ON INTELLIGENCE ACTIVITIES SENATOR FRANK CHURCH, CHAIRMAN"
            ),
            "Long text (>80 chars) should be filtered"
        );

        // Text with semicolons is data/lists, not headers
        assert!(
            is_fake_section_header("8 AGENTS AT 5 PERCENT; 2 AGENTS AT 10 PERCENT"),
            "Text with semicolons should be filtered"
        );

        // Government document patterns
        assert!(
            is_fake_section_header("40 PERCENT OF SUPERVISORY TIME"),
            "Text with PERCENT should be filtered"
        );
        assert!(
            is_fake_section_header("23 AGENTS AT 5 PERCENT"),
            "Text with AGENTS AT should be filtered"
        );

        // Labels ending with colon
        assert!(
            is_fake_section_header("Serial Scope:"),
            "Labels ending with colon should be filtered"
        );
        assert!(
            is_fake_section_header("ATTN:"),
            "Labels ending with colon should be filtered"
        );

        // But section headers with colons should be kept
        assert!(
            !is_fake_section_header("1. Introduction:"),
            "Section header with number should not be filtered"
        );
        assert!(
            !is_fake_section_header("3.2 Methods:"),
            "Section header with number should not be filtered"
        );

        // Short text without patterns should be kept as headers
        assert!(
            !is_fake_section_header("DIRECTOR, FBI"),
            "Short title-like text should not be filtered"
        );
        assert!(
            !is_fake_section_header("FROM"),
            "Short text should not be filtered"
        );
    }

    #[test]
    fn test_export_to_markdown_filters_fake_section_headers() {
        // N=4152: Test that fake section headers (reference citations) are rendered as text
        let doc_items = vec![
            DocItem::SectionHeader {
                self_ref: "h1".to_string(),
                parent: None,
                children: vec![],
                content_layer: "body".to_string(),
                prov: vec![],
                orig: "1 Introduction".to_string(),
                text: "1 Introduction".to_string(),
                level: 2,
                formatting: None,
                hyperlink: None,
            },
            DocItem::SectionHeader {
                self_ref: "fake".to_string(),
                parent: None,
                children: vec![],
                content_layer: "body".to_string(),
                prov: vec![],
                orig: "1873. IEEE (2022)".to_string(),
                text: "1873. IEEE (2022)".to_string(),
                level: 2,
                formatting: None,
                hyperlink: None,
            },
        ];

        let markdown = export_to_markdown(&doc_items);

        // Real section header should be rendered as heading
        assert!(
            markdown.contains("## 1 Introduction"),
            "Real section header should be a heading, got: {markdown}"
        );

        // Fake section header should NOT be a heading
        assert!(
            !markdown.contains("## 1873."),
            "Fake section header should not be a heading, got: {markdown}"
        );

        // But it should still be in the output as text
        assert!(
            markdown.contains("1873. IEEE (2022)"),
            "Fake section header content should still be present as text, got: {markdown}"
        );
    }

    #[test]
    fn test_export_to_markdown_picture_with_ocr_text() {
        // Test that Picture OCR content is rendered as structured annotation
        let doc_items = vec![DocItem::Picture {
            self_ref: "pic1".to_string(),
            parent: None,
            children: vec![],
            content_layer: "body".to_string(),
            prov: vec![ProvenanceItem {
                page_no: 2,
                bbox: BoundingBox {
                    l: 0.0,
                    t: 0.0,
                    r: 100.0,
                    b: 100.0,
                    coord_origin: CoordOrigin::TopLeft,
                },
                charspan: None,
            }],
            captions: vec![],
            footnotes: vec![],
            references: vec![],
            image: None,
            annotations: vec![],
            ocr_text: Some("1E+08 1E+06 HTML colspan".to_string()),
        }];

        let markdown = export_to_markdown(&doc_items);

        // Should contain image comment (matches Python docling format)
        assert!(
            markdown.contains("<!-- image -->"),
            "Picture should include image marker, got: {markdown}"
        );

        // Should contain the actual OCR text as content (Python docling renders OCR as visible text)
        assert!(
            markdown.contains("1E+08 1E+06 HTML colspan"),
            "Picture should include OCR text content, got: {markdown}"
        );
    }

    #[test]
    fn test_export_to_markdown_picture_without_ocr_text() {
        // Test that Picture without OCR text renders cleanly
        let doc_items = vec![DocItem::Picture {
            self_ref: "pic1".to_string(),
            parent: None,
            children: vec![],
            content_layer: "body".to_string(),
            prov: vec![ProvenanceItem {
                page_no: 0,
                bbox: BoundingBox {
                    l: 0.0,
                    t: 0.0,
                    r: 100.0,
                    b: 100.0,
                    coord_origin: CoordOrigin::TopLeft,
                },
                charspan: None,
            }],
            captions: vec![],
            footnotes: vec![],
            references: vec![],
            image: None,
            annotations: vec![],
            ocr_text: None,
        }];

        let markdown = export_to_markdown(&doc_items);

        // Should contain image comment (matches Python docling format)
        assert!(
            markdown.contains("<!-- image -->"),
            "Picture should include image marker, got: {markdown}"
        );
    }

    #[test]
    fn test_detect_header_level_title() {
        // Title label should always be H1 (level 0)
        assert_eq!(
            detect_header_level(
                "Mamba: Linear-Time Sequence Modeling",
                MlDocItemLabel::Title,
                0
            ),
            0
        );
        assert_eq!(
            detect_header_level("Any Title Text", MlDocItemLabel::Title, 5),
            0
        );
    }

    #[test]
    fn test_detect_header_level_top_level_sections() {
        // Top-level section names should be H2 (level 1)
        assert_eq!(
            detect_header_level("Abstract", MlDocItemLabel::SectionHeader, 0),
            1
        );
        assert_eq!(
            detect_header_level("Introduction", MlDocItemLabel::SectionHeader, 0),
            1
        );
        assert_eq!(
            detect_header_level("CONCLUSION", MlDocItemLabel::SectionHeader, 5),
            1
        );
        assert_eq!(
            detect_header_level("Related Work", MlDocItemLabel::SectionHeader, 1),
            1
        );
        assert_eq!(
            detect_header_level("References", MlDocItemLabel::SectionHeader, 10),
            1
        );
    }

    #[test]
    fn test_detect_header_level_numbered_sections() {
        // Single-level numbering (1, 2, 3, A, B, C) should be H2 (level 1)
        assert_eq!(
            detect_header_level("1 Introduction", MlDocItemLabel::SectionHeader, 0),
            1
        );
        assert_eq!(
            detect_header_level("2. Methods", MlDocItemLabel::SectionHeader, 1),
            1
        );
        assert_eq!(
            detect_header_level("A Background", MlDocItemLabel::SectionHeader, 5),
            1
        );

        // Two-level numbering (1.1, 2.3) should still be H2 (level 1) in PDF baseline
        assert_eq!(
            detect_header_level("1.1 State Space Models", MlDocItemLabel::SectionHeader, 1),
            1
        );
        assert_eq!(
            detect_header_level("3.2. Experimental Setup", MlDocItemLabel::SectionHeader, 3),
            1
        );

        // Three-level numbering (1.1.1) should still be H2 (level 1) in PDF baseline
        assert_eq!(
            detect_header_level("1.1.1 Sub-subsection", MlDocItemLabel::SectionHeader, 2),
            1
        );
        assert_eq!(
            detect_header_level(
                "2.3.1. Implementation Details",
                MlDocItemLabel::SectionHeader,
                4
            ),
            1
        );

        // Four-level numbering (1.1.1.1) should still be H2 (level 1) in PDF baseline
        assert_eq!(
            detect_header_level(
                "1.1.1.1 Very Deep Section",
                MlDocItemLabel::SectionHeader,
                5
            ),
            1
        );
    }

    #[test]
    fn test_detect_header_level_default() {
        // Unknown patterns should default to H2 (level 1)
        assert_eq!(
            detect_header_level("Some Random Header", MlDocItemLabel::SectionHeader, 0),
            1
        );
        assert_eq!(
            detect_header_level("Custom Section Name", MlDocItemLabel::SectionHeader, 3),
            1
        );
    }

    #[test]
    fn test_linkify_urls_basic() {
        // Basic URL should be linkified
        let input = "Visit https://example.com for more info";
        let output = linkify_urls(input);
        assert_eq!(
            output,
            "Visit [https://example.com](https://example.com) for more info"
        );
    }

    #[test]
    fn test_linkify_urls_http() {
        // HTTP URL should also be linkified
        let input = "Check http://old-site.org";
        let output = linkify_urls(input);
        assert_eq!(output, "Check [http://old-site.org](http://old-site.org)");
    }

    #[test]
    fn test_linkify_urls_multiple() {
        // Multiple URLs should all be linkified
        let input = "Links: https://a.com and https://b.com";
        let output = linkify_urls(input);
        assert!(output.contains("[https://a.com](https://a.com)"));
        assert!(output.contains("[https://b.com](https://b.com)"));
    }

    #[test]
    fn test_linkify_urls_no_double_link() {
        // Already-linked URLs should not be double-linked
        let input = "[https://example.com](https://example.com) is linked";
        let output = linkify_urls(input);
        assert_eq!(output, input, "Already-linked URL should not be modified");
    }

    #[test]
    fn test_linkify_urls_no_urls() {
        // Text without URLs should be unchanged
        let input = "This is plain text without any links";
        let output = linkify_urls(input);
        assert_eq!(output, input);
    }

    #[test]
    fn test_linkify_urls_trailing_punctuation() {
        // URLs with trailing punctuation should trim it
        let input = "See https://example.com.";
        let output = linkify_urls(input);
        assert_eq!(output, "See [https://example.com](https://example.com).");
    }

    #[test]
    fn test_export_to_markdown_with_urls() {
        // Test that export_to_markdown linkifies URLs in text content
        let doc_items = vec![DocItem::Text {
            self_ref: "t1".to_string(),
            parent: None,
            children: vec![],
            content_layer: "body".to_string(),
            prov: vec![],
            orig: "Visit https://arxiv.org for papers".to_string(),
            text: "Visit https://arxiv.org for papers".to_string(),
            formatting: None,
            hyperlink: None,
        }];

        let markdown = export_to_markdown(&doc_items);
        assert!(
            markdown.contains("[https://arxiv.org](https://arxiv.org)"),
            "URLs should be linkified in markdown output, got: {markdown}"
        );
    }
}
