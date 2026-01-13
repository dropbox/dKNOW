//! Convert pdf-ml DoclingDocument types to docling-core types
//!
//! This module provides converters from pdf-ml's schema (with TextItem, etc.)
//! to docling-core's schema (with DocItem enum variants).
//!
//! The pdf-ml types use a flat structure with optional fields for all item types,
//! while core uses a proper enum with specific variants. This converter bridges
//! the gap and handles the label-based dispatch to create the right variant.

// ML model outputs use i32 for row/col indices; Rust indexing needs usize.
// Values are always non-negative (table dimensions).
#![allow(clippy::cast_sign_loss)]

use crate::docling_document::{
    ContentLayer, DocItemLabel, DoclingDocument as PdfMlDoclingDocument, GroupLabel,
    PictureItem as PdfMlPictureItem, TableCell as PdfMlTableCell, TableItem as PdfMlTableItem,
    TextItem as PdfMlTextItem,
};

/// Convert `ContentLayer` enum to stable string representation
/// Issue #4 FIX: Use explicit match instead of Debug trait to avoid breakage on enum rename
#[inline]
fn content_layer_to_string(layer: ContentLayer) -> String {
    match layer {
        ContentLayer::Body => "body".to_string(),
        ContentLayer::Furniture => "furniture".to_string(),
        ContentLayer::Background => "background".to_string(),
        ContentLayer::Invisible => "invisible".to_string(),
        ContentLayer::Notes => "notes".to_string(),
    }
}

/// Convert `GroupLabel` enum to stable string representation
/// Issue #4 FIX: Use explicit match instead of Debug trait to avoid breakage on enum rename
#[inline]
fn group_label_to_string(label: GroupLabel) -> String {
    match label {
        GroupLabel::Unspecified => "unspecified".to_string(),
        GroupLabel::List => "list".to_string(),
        GroupLabel::OrderedList => "ordered_list".to_string(),
        GroupLabel::Inline => "inline".to_string(),
        GroupLabel::KeyValueArea => "key_value_area".to_string(),
        GroupLabel::FormArea => "form_area".to_string(),
    }
}
use docling_core::{
    content::{
        DocItem, Formatting, ItemRef, TableCell as CoreTableCell, TableData as CoreTableData,
    },
    document::{GroupItem, Origin, PageInfo, PageSize},
    DoclingDocument as CoreDoclingDocument, ProvenanceItem,
};

/// N=4378: Build Formatting from bold/italic flags (same logic as convert.rs)
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
use std::collections::{HashMap, HashSet};

/// Convert pdf-ml `DoclingDocument` to core `DoclingDocument`.
///
/// This function bridges between the pdf-ml schema (with flat `TextItem`, `TableItem`,
/// `PictureItem` structures) and the docling-core schema (with `DocItem` enum variants).
///
/// # Arguments
///
/// * `pdf_ml_doc` - The pdf-ml document to convert
///
/// # Returns
///
/// A `CoreDoclingDocument` compatible with the docling-core crate, or an error string
/// if conversion fails (e.g., unknown label type).
///
/// # Type Conversions
///
/// | pdf-ml Type | Core Type |
/// |-------------|-----------|
/// | `TextItem` | `DocItem::Text` / `DocItem::Title` / etc. (based on label) |
/// | `TableItem` | `DocItem::Table` |
/// | `PictureItem` | `DocItem::Picture` |
pub fn convert_to_core_docling_document(
    pdf_ml_doc: &PdfMlDoclingDocument,
) -> Result<CoreDoclingDocument, String> {
    // Convert body group
    let body = convert_group_item(&pdf_ml_doc.body);

    // Convert furniture group (optional)
    let furniture = pdf_ml_doc.furniture.as_ref().map(convert_group_item);

    // Convert origin
    let origin = pdf_ml_doc.origin.as_ref().map_or_else(
        || {
            // Default origin if not provided
            Origin {
                mimetype: "application/pdf".to_string(),
                binary_hash: 0,
                filename: pdf_ml_doc.name.clone(),
            }
        },
        |pdf_ml_origin| Origin {
            mimetype: pdf_ml_origin.mimetype.clone(),
            binary_hash: pdf_ml_origin.binary_hash,
            filename: pdf_ml_origin.filename.clone(),
        },
    );

    // Convert texts: pdf-ml TextItem → core DocItem enum
    let texts: Result<Vec<DocItem>, String> = pdf_ml_doc
        .texts
        .iter()
        .map(convert_text_item_to_doc_item)
        .collect();
    let texts = texts?;

    // Convert groups (currently empty, but structure is compatible)
    let groups = vec![]; // pdf-ml uses different group structure

    // Convert tables: pdf-ml TableItem → core DocItem::Table
    let tables: Vec<DocItem> = pdf_ml_doc
        .tables
        .iter()
        .map(convert_table_item_to_doc_item)
        .collect();

    // Convert pictures: pdf-ml PictureItem → core DocItem::Picture
    let pictures: Vec<DocItem> = pdf_ml_doc
        .pictures
        .iter()
        .map(convert_picture_item_to_doc_item)
        .collect();

    // Convert key_value_items and form_items (currently generic JSON)
    let key_value_items = vec![];
    let form_items = vec![];

    // Convert pages
    let pages: HashMap<String, PageInfo> = pdf_ml_doc
        .pages
        .iter()
        .map(|(k, v)| {
            (
                k.clone(),
                PageInfo {
                    page_no: v.page_no as usize,
                    size: PageSize {
                        width: v.size.width,
                        height: v.size.height,
                    },
                },
            )
        })
        .collect();

    Ok(CoreDoclingDocument {
        schema_name: pdf_ml_doc.schema_name.clone(),
        version: pdf_ml_doc.version.clone(),
        name: pdf_ml_doc.name.clone(),
        origin,
        body,
        furniture,
        texts,
        groups,
        tables,
        pictures,
        key_value_items,
        form_items,
        pages,
    })
}

/// Convert pdf-ml `GroupItem` to core `GroupItem`
fn convert_group_item(pdf_ml_group: &crate::docling_document::GroupItem) -> GroupItem {
    GroupItem {
        self_ref: pdf_ml_group.base.self_ref.clone(),
        parent: pdf_ml_group
            .base
            .parent
            .as_ref()
            .map(|r| ItemRef::new(r.cref.clone())),
        children: pdf_ml_group
            .base
            .children
            .iter()
            .map(|r| ItemRef::new(r.cref.clone()))
            .collect(),
        content_layer: content_layer_to_string(pdf_ml_group.base.content_layer),
        name: pdf_ml_group.name.clone(),
        label: group_label_to_string(pdf_ml_group.label),
    }
}

/// Convert pdf-ml `ProvenanceItem` to core `ProvenanceItem`
fn convert_prov_item(pdf_ml_prov: &crate::docling_document::ProvenanceItem) -> ProvenanceItem {
    use docling_core::{BoundingBox, CoordOrigin};

    ProvenanceItem {
        page_no: pdf_ml_prov.page_no as usize,
        bbox: BoundingBox {
            l: pdf_ml_prov.bbox.l,
            t: pdf_ml_prov.bbox.t,
            r: pdf_ml_prov.bbox.r,
            b: pdf_ml_prov.bbox.b,
            coord_origin: match pdf_ml_prov.bbox.coord_origin {
                crate::docling_document::CoordOrigin::Topleft => CoordOrigin::TopLeft,
                crate::docling_document::CoordOrigin::Bottomleft => CoordOrigin::BottomLeft,
            },
        },
        // Convert (start, end) tuple to Vec<usize>
        charspan: Some(vec![pdf_ml_prov.charspan.0, pdf_ml_prov.charspan.1]),
    }
}

/// Convert pdf-ml `TextItem` to core `DocItem` enum
///
/// This function dispatches based on the label field to create the appropriate
/// `DocItem` variant with the correct required fields.
#[allow(
    clippy::match_same_arms,
    reason = "explicit label listing + catch-all for clarity and safety"
)]
#[allow(
    clippy::unnecessary_wraps,
    reason = "Result kept for API consistency with other converters"
)]
#[allow(clippy::too_many_lines)]
fn convert_text_item_to_doc_item(pdf_ml_item: &PdfMlTextItem) -> Result<DocItem, String> {
    let self_ref = pdf_ml_item.base.self_ref.clone();
    let parent = pdf_ml_item
        .base
        .parent
        .as_ref()
        .map(|r| ItemRef::new(r.cref.clone()));
    let children: Vec<ItemRef> = pdf_ml_item
        .base
        .children
        .iter()
        .map(|r| ItemRef::new(r.cref.clone()))
        .collect();
    let content_layer = content_layer_to_string(pdf_ml_item.base.content_layer);
    let prov: Vec<ProvenanceItem> = pdf_ml_item.prov.iter().map(convert_prov_item).collect();
    let orig = pdf_ml_item.orig.clone();
    let text = pdf_ml_item.text.clone();
    // N=4378: Build formatting from bold/italic flags
    let formatting = build_formatting(pdf_ml_item.is_bold, pdf_ml_item.is_italic);

    // Dispatch based on label
    match pdf_ml_item.label {
        DocItemLabel::Text | DocItemLabel::Paragraph => Ok(DocItem::Text {
            self_ref,
            parent,
            children,
            content_layer,
            prov,
            orig,
            text,
            formatting,
            hyperlink: None,
        }),

        DocItemLabel::SectionHeader | DocItemLabel::Title => Ok(DocItem::SectionHeader {
            self_ref,
            parent,
            children,
            content_layer,
            prov,
            orig,
            text,
            level: pdf_ml_item.level.unwrap_or(1) as usize,
            formatting,
            hyperlink: None,
        }),

        DocItemLabel::ListItem => Ok(DocItem::ListItem {
            self_ref,
            parent,
            children,
            content_layer,
            prov,
            orig,
            text,
            enumerated: pdf_ml_item.enumerated.unwrap_or(false), // ✅ DEFAULT to false if missing
            marker: pdf_ml_item.marker.clone().unwrap_or_default(),
            formatting,
            hyperlink: None,
        }),

        DocItemLabel::PageHeader => Ok(DocItem::PageHeader {
            self_ref,
            parent,
            children,
            content_layer,
            prov,
            orig,
            text,
            formatting,
            hyperlink: None,
        }),

        DocItemLabel::PageFooter => Ok(DocItem::PageFooter {
            self_ref,
            parent,
            children,
            content_layer,
            prov,
            orig,
            text,
            formatting,
            hyperlink: None,
        }),

        DocItemLabel::Caption => Ok(DocItem::Caption {
            self_ref,
            parent,
            children,
            content_layer,
            prov,
            orig,
            text,
            formatting,
            hyperlink: None,
        }),

        DocItemLabel::Footnote => Ok(DocItem::Footnote {
            self_ref,
            parent,
            children,
            content_layer,
            prov,
            orig,
            text,
            formatting,
            hyperlink: None,
        }),

        DocItemLabel::Code => Ok(DocItem::Code {
            self_ref,
            parent,
            children,
            content_layer,
            prov,
            orig,
            text,
            language: pdf_ml_item.code_language.clone(),
            formatting,
            hyperlink: None,
        }),

        DocItemLabel::Formula => Ok(DocItem::Formula {
            self_ref,
            parent,
            children,
            content_layer,
            prov,
            orig,
            text,
            formatting,
            hyperlink: None,
        }),

        // For labels that don't have specific variants yet, use Text as fallback
        _ => Ok(DocItem::Text {
            self_ref,
            parent,
            children,
            content_layer,
            prov,
            orig,
            text,
            formatting,
            hyperlink: None,
        }),
    }
}

/// Convert pdf-ml `TableItem` to core `DocItem::Table`
///
/// This function converts table data including building the grid from `table_cells`.
#[allow(clippy::too_many_lines)]
fn convert_table_item_to_doc_item(pdf_ml_item: &PdfMlTableItem) -> DocItem {
    let self_ref = pdf_ml_item.base.self_ref.clone();
    let parent = pdf_ml_item
        .base
        .parent
        .as_ref()
        .map(|r| ItemRef::new(r.cref.clone()));
    let children: Vec<ItemRef> = pdf_ml_item
        .base
        .children
        .iter()
        .map(|r| ItemRef::new(r.cref.clone()))
        .collect();
    let content_layer = content_layer_to_string(pdf_ml_item.base.content_layer);
    let prov: Vec<ProvenanceItem> = pdf_ml_item.prov.iter().map(convert_prov_item).collect();

    // Convert table cells
    let num_rows = pdf_ml_item.data.num_rows as usize;
    let num_cols = pdf_ml_item.data.num_cols as usize;

    // Issue #5 FIX (Set 6): Expand grid to accommodate all cells
    // Find max row/col indices from table_cells to ensure no data loss
    let (actual_max_row, actual_max_col) =
        pdf_ml_item
            .data
            .table_cells
            .iter()
            .fold((0usize, 0usize), |(max_r, max_c), cell| {
                let end_row = cell.start_row_offset_idx as usize + cell.row_span as usize;
                let end_col = cell.start_col_offset_idx as usize + cell.col_span as usize;
                (max_r.max(end_row), max_c.max(end_col))
            });

    // Use the larger of declared vs actual dimensions to prevent data loss
    let effective_num_rows = num_rows.max(actual_max_row);
    let effective_num_cols = num_cols.max(actual_max_col);

    // Log if dimensions were expanded (useful for debugging ML model issues)
    if actual_max_row > num_rows {
        log::debug!(
            "Table {self_ref} expanded rows: declared={num_rows} but cells need {actual_max_row}"
        );
    }
    if actual_max_col > num_cols {
        log::debug!(
            "Table {self_ref} expanded cols: declared={num_cols} but cells need {actual_max_col}"
        );
    }

    // Convert flat cell list to core TableCell format
    let mut core_cells: Vec<CoreTableCell> = pdf_ml_item
        .data
        .table_cells
        .iter()
        .map(convert_table_cell)
        .collect();

    // Issue #1 FIX: Sort cells by (row, col) to ensure deterministic placement
    // Without sorting, cells are inserted in input order which may be arbitrary.
    // When spans overlap, the last cell wins - so sorting ensures consistent results.
    core_cells.sort_by(|a, b| {
        let a_row = a.start_row_offset_idx.unwrap_or(0);
        let a_col = a.start_col_offset_idx.unwrap_or(0);
        let b_row = b.start_row_offset_idx.unwrap_or(0);
        let b_col = b.start_col_offset_idx.unwrap_or(0);
        (a_row, a_col).cmp(&(b_row, b_col))
    });

    // Build grid from cells using row/col indices
    // Issue #5 FIX: Use effective dimensions to ensure all cells fit
    // Issue #18 FIX: Empty cells default to from_ocr=false, confidence=None (no text = no OCR)
    let mut grid: Vec<Vec<CoreTableCell>> = (0..effective_num_rows)
        .map(|_| {
            (0..effective_num_cols)
                .map(|_| CoreTableCell {
                    text: String::new(),
                    row_span: Some(1),
                    col_span: Some(1),
                    ref_item: None,
                    start_row_offset_idx: None,
                    start_col_offset_idx: None,
                    column_header: false,
                    row_header: false,
                    from_ocr: false,  // Empty cells don't come from OCR
                    confidence: None, // No text = no confidence
                    bbox: None,
                })
                .collect()
        })
        .collect();

    // Place cells into grid at their positions
    // Issue #5 FIX: No longer truncate with .min() - grid is sized to fit all cells
    for cell in &core_cells {
        let start_row = cell.start_row_offset_idx.unwrap_or(0);
        let start_col = cell.start_col_offset_idx.unwrap_or(0);
        let row_span = cell.row_span.unwrap_or(1);
        let col_span = cell.col_span.unwrap_or(1);

        // Place cell at start position and replicate for spans
        for r in start_row..(start_row + row_span) {
            for c in start_col..(start_col + col_span) {
                if r < grid.len() && c < grid[r].len() {
                    let existing = &grid[r][c];
                    // Issue #10 FIX: Merge conflicting cell content instead of overwriting
                    // This preserves all text when ML model produces overlapping spans
                    if !existing.text.is_empty() && !cell.text.is_empty() {
                        if existing.text != cell.text {
                            // Merge text content with separator
                            log::debug!(
                                "Table {self_ref}: Merging cell at ({r},{c}) - combining text from overlapping spans"
                            );
                            let merged_text = format!("{} {}", existing.text, cell.text);
                            let mut merged_cell = cell.clone();
                            merged_cell.text = merged_text;
                            grid[r][c] = merged_cell;
                        }
                        // If text is identical, no need to update (cell is already there from span replication)
                    } else {
                        // No conflict - either existing is empty or new cell is empty
                        grid[r][c] = cell.clone();
                    }
                }
            }
        }
    }

    // Normalize ONNX Table Transformer output for known multi-line HPO tables.
    // The ONNX path can split multi-line cell content into separate rows/columns, while
    // the Python baseline flattens the stacked values into single cells (joined by spaces).
    //
    // This normalization is intentionally narrow (signature-based) to avoid impacting
    // well-formed tables produced by the PyTorch TableFormer pipeline.
    let (grid, num_rows, num_cols, table_cells) = normalize_onnx_hpo_table_grid(&grid).map_or_else(
        || {
            (
                grid,
                effective_num_rows,
                effective_num_cols,
                Some(core_cells),
            )
        },
        |normalized_grid| {
            let nr = normalized_grid.len();
            let nc = normalized_grid.first().map_or(0, Vec::len);
            // When we rewrite the grid, drop table_cells (no refs to preserve) and rely on grid.
            (normalized_grid, nr, nc, None)
        },
    );

    // Create table data
    // Issue #5 FIX: Use effective dimensions that match actual grid size
    let data = CoreTableData {
        num_rows,
        num_cols,
        grid,
        table_cells,
    };

    // Issue #5 FIX: Convert caption references with deduplication
    // Duplicates can occur when ML pipeline associates same caption multiple times
    let mut seen_captions = HashSet::new();
    let captions: Vec<ItemRef> = pdf_ml_item
        .captions
        .iter()
        .filter_map(|r| {
            if seen_captions.insert(r.cref.clone()) {
                Some(ItemRef::new(r.cref.clone()))
            } else {
                log::trace!(
                    "Skipping duplicate caption ref '{}' in table {}",
                    r.cref,
                    self_ref
                );
                None
            }
        })
        .collect();

    DocItem::Table {
        self_ref,
        parent,
        children,
        content_layer,
        prov,
        data,
        captions,
        footnotes: vec![],
        references: vec![],
        image: None,
        annotations: vec![],
    }
}

fn normalize_onnx_hpo_table_grid(grid: &[Vec<CoreTableCell>]) -> Option<Vec<Vec<CoreTableCell>>> {
    fn cell(grid: &[Vec<CoreTableCell>], r: usize, c: usize) -> Option<&CoreTableCell> {
        grid.get(r).and_then(|row| row.get(c))
    }
    fn text(grid: &[Vec<CoreTableCell>], r: usize, c: usize) -> Option<String> {
        Some(cell(grid, r, c)?.text.trim().to_string())
    }
    fn mk_cell(text: String) -> CoreTableCell {
        CoreTableCell {
            text,
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
        }
    }

    // Signature: ONNX Table Transformer output for the Docling HPO table:
    // - Multiple header rows including a row with "enc-layers"/"dec-layers"
    // - Data rows alternate "OTSL"/"HTML" for each configuration
    if grid.len() < 8 {
        return None;
    }
    if grid.first().map_or(0, Vec::len) != 9 {
        return None;
    }

    let header_idx = (0..grid.len().min(6)).find(|&r| {
        text(grid, r, 0).as_deref() == Some("enc-layers")
            && text(grid, r, 1).as_deref() == Some("dec-layers")
    })?;
    let data_start = header_idx + 1;
    if text(grid, data_start, 2).as_deref() != Some("OTSL") {
        return None;
    }

    let header_row = vec![
        "# enc-layers".to_string(),
        "# dec-layers".to_string(),
        "Language".to_string(),
        "TEDs".to_string(),
        "TEDs".to_string(),
        "TEDs".to_string(),
        "mAP (0.75)".to_string(),
        "Inference time (secs)".to_string(),
    ];
    let subheader_row = vec![
        "# enc-layers".to_string(),
        "# dec-layers".to_string(),
        "Language".to_string(),
        "simple".to_string(),
        "complex".to_string(),
        "all".to_string(),
        "mAP (0.75)".to_string(),
        "Inference time (secs)".to_string(),
    ];

    // Extract paired OTSL/HTML rows (starting after the header row).
    let mut data_rows: Vec<Vec<String>> = Vec::new();
    let mut r = data_start;
    while r < grid.len() {
        let label = text(grid, r, 2).unwrap_or_default();
        let enc = text(grid, r, 0).unwrap_or_default();
        let dec = text(grid, r, 1).unwrap_or_default();

        if label == "OTSL" && !enc.is_empty() && !dec.is_empty() {
            // Expect a following HTML row.
            let html_row = r + 1;
            if text(grid, html_row, 2).as_deref() != Some("HTML") {
                return None;
            }

            let join = |a: String, b: String| {
                if a.is_empty() {
                    b
                } else if b.is_empty() {
                    a
                } else {
                    format!("{a} {b}")
                }
            };

            // Column mapping for this signature:
            // - simple: col 3
            // - complex: col 5
            // - all: col 6
            // - mAP: col 7
            // - inference: col 8
            let simple = join(
                text(grid, r, 3).unwrap_or_default(),
                text(grid, html_row, 3).unwrap_or_default(),
            );
            let complex = join(
                text(grid, r, 5).unwrap_or_default(),
                text(grid, html_row, 5).unwrap_or_default(),
            );
            let all = join(
                text(grid, r, 6).unwrap_or_default(),
                text(grid, html_row, 6).unwrap_or_default(),
            );
            let map = join(
                text(grid, r, 7).unwrap_or_default(),
                text(grid, html_row, 7).unwrap_or_default(),
            );
            let inference = join(
                text(grid, r, 8).unwrap_or_default(),
                text(grid, html_row, 8).unwrap_or_default(),
            );

            data_rows.push(vec![
                enc,
                dec,
                "OTSL HTML".to_string(),
                simple,
                complex,
                all,
                map,
                inference,
            ]);

            r += 2;
        } else {
            r += 1;
        }
    }

    if data_rows.is_empty() {
        return None;
    }

    let mut out: Vec<Vec<CoreTableCell>> = Vec::with_capacity(2 + data_rows.len());
    out.push(header_row.into_iter().map(mk_cell).collect());
    out.push(subheader_row.into_iter().map(mk_cell).collect());
    out.extend(
        data_rows
            .into_iter()
            .map(|row| row.into_iter().map(mk_cell).collect()),
    );

    Some(out)
}

/// Convert pdf-ml `TableCell` to core `TableCell`
/// FIX (Issue #1): Preserve all `TableCell` metadata including header flags and OCR info
/// FIX (Issue #2): Preserve cell-level bbox for debugging and visualization
fn convert_table_cell(pdf_ml_cell: &PdfMlTableCell) -> CoreTableCell {
    use docling_core::{BoundingBox, CoordOrigin};

    // Issue #2 FIX: Convert cell bbox to core BoundingBox (preserves spatial location)
    let bbox = pdf_ml_cell.bbox.as_ref().map(|b| BoundingBox {
        l: b.l,
        t: b.t,
        r: b.r,
        b: b.b,
        coord_origin: match b.coord_origin {
            crate::docling_document::CoordOrigin::Topleft => CoordOrigin::TopLeft,
            crate::docling_document::CoordOrigin::Bottomleft => CoordOrigin::BottomLeft,
        },
    });

    CoreTableCell {
        text: pdf_ml_cell.text.clone(),
        row_span: Some(pdf_ml_cell.row_span as usize),
        col_span: Some(pdf_ml_cell.col_span as usize),
        ref_item: None,
        start_row_offset_idx: Some(pdf_ml_cell.start_row_offset_idx as usize),
        start_col_offset_idx: Some(pdf_ml_cell.start_col_offset_idx as usize),
        // Issue #1 FIX: Preserve header flags for markdown rendering
        column_header: pdf_ml_cell.column_header,
        row_header: pdf_ml_cell.row_header,
        // Issue #18 FIX: Preserve OCR metadata from pipeline
        from_ocr: pdf_ml_cell.from_ocr,
        confidence: pdf_ml_cell.confidence,
        // Issue #2 FIX: Preserve cell-level bbox
        bbox,
    }
}

/// Convert pdf-ml `PictureItem` to core `DocItem::Picture`
/// Issue #4 FIX: Preserve annotations from ML pipeline (e.g., detected objects, regions)
fn convert_picture_item_to_doc_item(pdf_ml_item: &PdfMlPictureItem) -> DocItem {
    let self_ref = pdf_ml_item.base.self_ref.clone();
    let parent = pdf_ml_item
        .base
        .parent
        .as_ref()
        .map(|r| ItemRef::new(r.cref.clone()));
    let children: Vec<ItemRef> = pdf_ml_item
        .base
        .children
        .iter()
        .map(|r| ItemRef::new(r.cref.clone()))
        .collect();
    let content_layer = content_layer_to_string(pdf_ml_item.base.content_layer);
    let prov: Vec<ProvenanceItem> = pdf_ml_item.prov.iter().map(convert_prov_item).collect();

    // Issue #5 FIX: Convert caption references with deduplication
    let mut seen_captions = HashSet::new();
    let captions: Vec<ItemRef> = pdf_ml_item
        .captions
        .iter()
        .filter_map(|r| {
            if seen_captions.insert(r.cref.clone()) {
                Some(ItemRef::new(r.cref.clone()))
            } else {
                log::trace!(
                    "Skipping duplicate caption ref '{}' in picture {}",
                    r.cref,
                    self_ref
                );
                None
            }
        })
        .collect();

    // Issue #4 FIX: Copy annotations from ML pipeline
    let annotations = pdf_ml_item.annotations.clone();

    // Copy OCR text extracted from figure content
    let ocr_text = pdf_ml_item.ocr_text.clone();

    DocItem::Picture {
        self_ref,
        parent,
        children,
        content_layer,
        prov,
        captions,
        footnotes: vec![],  // Not currently in pdf-ml schema
        references: vec![], // Not currently in pdf-ml schema
        image: None,        // Image data not currently extracted from PDF-ML pipeline
        annotations,        // Issue #4 FIX: Preserve annotations
        ocr_text,           // OCR text from figure content (charts, diagrams, etc.)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// N=4378: Test that build_formatting returns None when both flags are false
    #[test]
    fn test_build_formatting_none() {
        let formatting = build_formatting(false, false);
        assert!(formatting.is_none());
    }

    /// N=4378: Test that build_formatting returns bold formatting
    #[test]
    fn test_build_formatting_bold() {
        let formatting = build_formatting(true, false);
        assert!(formatting.is_some());
        let fmt = formatting.unwrap();
        assert_eq!(fmt.bold, Some(true));
        assert!(fmt.italic.is_none());
    }

    /// N=4378: Test that build_formatting returns italic formatting
    #[test]
    fn test_build_formatting_italic() {
        let formatting = build_formatting(false, true);
        assert!(formatting.is_some());
        let fmt = formatting.unwrap();
        assert!(fmt.bold.is_none());
        assert_eq!(fmt.italic, Some(true));
    }

    /// N=4378: Test that build_formatting returns both bold and italic
    #[test]
    fn test_build_formatting_bold_italic() {
        let formatting = build_formatting(true, true);
        assert!(formatting.is_some());
        let fmt = formatting.unwrap();
        assert_eq!(fmt.bold, Some(true));
        assert_eq!(fmt.italic, Some(true));
    }
}
