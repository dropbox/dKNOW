//! `DoclingDocument` export - Convert Rust pipeline output to `DoclingDocument` JSON format
//!
//! This module converts Rust `PageElement` structures to the official `DoclingDocument`
//! v1.8.0 JSON schema format for validation against Python Docling baselines.
//!
//! Note: Infrastructure code ported from Python. Some code paths not yet wired up.
#![allow(dead_code)]
// Intentional ML conversions: page indices, bounding box coordinates
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]

use crate::convert::detect_header_level;
use crate::docling_document::{
    BoundingBox as DocBBox, ContentLayer, CoordOrigin as DocCoordOrigin, DocItemLabel as DocLabel,
    DoclingDocument, DocumentOrigin, GroupItem, GroupLabel, NodeItemBase, PageItem, PictureItem,
    ProvenanceItem, RefItem, TableCell as DocTableCell, TableData, TableItem, TextItem,
    DOCLING_DOCUMENT_SCHEMA_NAME, DOCLING_DOCUMENT_SCHEMA_VERSION,
};
use crate::pipeline::data_structures::TextCell;
use crate::pipeline::{
    BoundingBox, DocItemLabel, FigureElement, PageElement, Size as PageSize, TableCell,
    TableElement, TextElement,
};
use std::collections::{HashMap, HashSet};

/// Build the set of caption and footnote CIDs that should be skipped in main iteration
/// (they are added immediately after their parent element instead)
fn build_skippable_cids(pages: &[crate::pipeline::Page]) -> HashSet<(usize, usize)> {
    let mut skippable_cids = HashSet::new();

    for page in pages {
        let Some(assembled) = &page.assembled else {
            continue;
        };

        for element in &assembled.elements {
            let (caption_cids, footnote_cids) = match element {
                PageElement::Text(e) => (&e.captions, &e.footnotes),
                PageElement::Table(e) => (&e.captions, &e.footnotes),
                PageElement::Figure(e) => (&e.captions, &e.footnotes),
                PageElement::Container(_) => continue,
            };

            for &cid in caption_cids {
                skippable_cids.insert((page.page_no, cid));
            }
            for &cid in footnote_cids {
                skippable_cids.insert((page.page_no, cid));
            }
        }
    }

    skippable_cids
}

/// N=4393: Build map of table/figure bounding boxes per page
/// Used to skip text elements that are contained within tables/figures
/// (prevents duplication when both individual text elements AND the table are detected)
/// N=4430: UNUSED - the filter was removed because it caused 65% content loss
#[allow(dead_code)]
fn build_container_bboxes(pages: &[crate::pipeline::Page]) -> HashMap<usize, Vec<BoundingBox>> {
    let mut container_bboxes: HashMap<usize, Vec<BoundingBox>> = HashMap::new();

    for page in pages {
        let Some(assembled) = &page.assembled else {
            continue;
        };

        let page_bboxes = container_bboxes.entry(page.page_no).or_default();

        for element in &assembled.elements {
            match element {
                PageElement::Table(table_elem) => {
                    page_bboxes.push(table_elem.cluster.bbox);
                }
                PageElement::Figure(fig_elem) => {
                    page_bboxes.push(fig_elem.cluster.bbox);
                }
                PageElement::Text(_) | PageElement::Container(_) => {}
            }
        }
    }

    container_bboxes
}

/// N=4422: Build map of FIGURE-ONLY bounding boxes per page
/// Used to skip tables that are contained within figures
/// (prevents duplicate table extraction from figure images)
/// N=4431: UNUSED - filter removed per CLEANUP SPRINT (filters legitimate tables)
#[allow(dead_code)]
fn build_figure_bboxes(pages: &[crate::pipeline::Page]) -> HashMap<usize, Vec<BoundingBox>> {
    let mut figure_bboxes: HashMap<usize, Vec<BoundingBox>> = HashMap::new();

    for page in pages {
        let Some(assembled) = &page.assembled else {
            continue;
        };

        let page_bboxes = figure_bboxes.entry(page.page_no).or_default();

        for element in &assembled.elements {
            if let PageElement::Figure(fig_elem) = element {
                page_bboxes.push(fig_elem.cluster.bbox);
            }
        }
    }

    figure_bboxes
}

/// N=4422: Check if a table is contained within any figure bbox
/// Uses 80% containment threshold (same as text containment check)
/// This filters out spurious tables extracted from figure images
/// N=4431: UNUSED - filter removed per CLEANUP SPRINT (filters legitimate tables)
#[allow(dead_code)]
fn is_table_inside_figure(table_bbox: &BoundingBox, figure_bboxes: &[BoundingBox]) -> bool {
    const CONTAINMENT_THRESHOLD: f32 = 0.8;

    for figure_bbox in figure_bboxes {
        if table_bbox.intersection_over_self(figure_bbox) > CONTAINMENT_THRESHOLD {
            return true;
        }
    }

    false
}

/// N=4393: Check if a text element is contained within any table/figure bbox
/// Uses 80% containment threshold (same as overlap resolver)
/// N=4430: UNUSED - the filter was removed because it caused 65% content loss
#[allow(dead_code)]
fn is_text_inside_container(text_bbox: &BoundingBox, container_bboxes: &[BoundingBox]) -> bool {
    const CONTAINMENT_THRESHOLD: f32 = 0.8;

    for container_bbox in container_bboxes {
        if text_bbox.intersection_over_self(container_bbox) > CONTAINMENT_THRESHOLD {
            return true;
        }
    }

    false
}

fn bbox_to_top_left_origin(bbox: BoundingBox, page_height: f32) -> BoundingBox {
    match bbox.coord_origin {
        crate::pipeline::CoordOrigin::TopLeft => bbox,
        crate::pipeline::CoordOrigin::BottomLeft => BoundingBox {
            l: bbox.l,
            t: page_height - bbox.t,
            r: bbox.r,
            b: page_height - bbox.b,
            coord_origin: crate::pipeline::CoordOrigin::TopLeft,
        },
    }
}

fn group_text_cells_into_lines<'a>(
    cells: &'a [TextCell],
    page_height: f32,
) -> Vec<Vec<&'a TextCell>> {
    #[derive(Debug, Clone)]
    struct CellGeom<'a> {
        cell: &'a TextCell,
        center_y: f32,
        left_x: f32,
        height: f32,
    }

    #[derive(Debug)]
    struct Line<'a> {
        cells: Vec<CellGeom<'a>>,
        avg_center_y: f32,
    }

    if cells.is_empty() {
        return Vec::new();
    }

    let mut geoms: Vec<CellGeom<'a>> = cells
        .iter()
        .map(|cell| {
            let bbox = bbox_to_top_left_origin(cell.rect.to_bbox(), page_height);
            let height = (bbox.b - bbox.t).abs();
            let center_y = (bbox.t + bbox.b) / 2.0;
            CellGeom {
                cell,
                center_y,
                left_x: bbox.l,
                height,
            }
        })
        .collect();

    let mut heights: Vec<f32> = geoms
        .iter()
        .map(|g| g.height)
        .filter(|h| *h > 0.0)
        .collect();
    heights.sort_by(f32::total_cmp);
    let median_height = heights.get(heights.len() / 2).copied().unwrap_or(0.0);
    let line_threshold = (median_height * 0.75).max(2.0);

    geoms.sort_by(|a, b| {
        a.center_y
            .total_cmp(&b.center_y)
            .then_with(|| a.left_x.total_cmp(&b.left_x))
            .then_with(|| a.cell.index.cmp(&b.cell.index))
    });

    let mut lines: Vec<Line<'a>> = Vec::new();

    for geom in geoms {
        match lines.last_mut() {
            None => {
                lines.push(Line {
                    avg_center_y: geom.center_y,
                    cells: vec![geom],
                });
            }
            Some(line) => {
                if (geom.center_y - line.avg_center_y).abs() <= line_threshold {
                    let prev_count = line.cells.len() as f32;
                    line.avg_center_y =
                        (line.avg_center_y * prev_count + geom.center_y) / (prev_count + 1.0);
                    line.cells.push(geom);
                } else {
                    lines.push(Line {
                        avg_center_y: geom.center_y,
                        cells: vec![geom],
                    });
                }
            }
        }
    }

    lines
        .into_iter()
        .map(|mut line| {
            line.cells.sort_by(|a, b| {
                a.left_x
                    .total_cmp(&b.left_x)
                    .then_with(|| a.center_y.total_cmp(&b.center_y))
                    .then_with(|| a.cell.index.cmp(&b.cell.index))
            });
            line.cells.into_iter().map(|g| g.cell).collect()
        })
        .collect()
}

/// State for collecting document items during export
struct ExportCollector {
    texts: Vec<TextItem>,
    tables: Vec<TableItem>,
    pictures: Vec<PictureItem>,
    cid_to_ref: HashMap<(usize, usize), String>,
}

impl ExportCollector {
    fn new() -> Self {
        Self {
            texts: Vec::new(),
            tables: Vec::new(),
            pictures: Vec::new(),
            cid_to_ref: HashMap::new(),
        }
    }

    /// Add captions as child text items after a parent element
    fn add_captions(
        &mut self,
        caption_cids: &[usize],
        parent_ref: &str,
        page_no: usize,
        page_height: f32,
        cid_to_element: &HashMap<usize, &PageElement>,
    ) -> Vec<RefItem> {
        let mut child_refs = Vec::new();

        for &caption_cid in caption_cids {
            let caption_key = (page_no, caption_cid);
            if let Some(PageElement::Text(caption_text_elem)) = cid_to_element.get(&caption_cid) {
                let caption_idx = self.texts.len();
                let caption_self_ref = format!("#/texts/{caption_idx}");
                self.cid_to_ref
                    .insert(caption_key, caption_self_ref.clone());

                let mut caption_item = convert_text_element(
                    caption_text_elem,
                    &caption_self_ref,
                    page_no,
                    page_height,
                );
                caption_item.base.parent = Some(RefItem {
                    cref: parent_ref.to_string(),
                });
                self.texts.push(caption_item);

                child_refs.push(RefItem {
                    cref: caption_self_ref,
                });
            }
        }

        child_refs
    }

    /// Process a text element and its captions
    fn process_text_element(
        &mut self,
        text_elem: &TextElement,
        page_no: usize,
        page_height: f32,
        cid_to_element: &HashMap<usize, &PageElement>,
    ) {
        let text_idx = self.texts.len();
        let self_ref = format!("#/texts/{text_idx}");
        let key = (page_no, text_elem.cluster.id);
        self.cid_to_ref.insert(key, self_ref.clone());

        self.texts.push(convert_text_element(
            text_elem,
            &self_ref,
            page_no,
            page_height,
        ));

        // Add captions immediately after parent
        let caption_refs = self.add_captions(
            &text_elem.captions,
            &self_ref,
            page_no,
            page_height,
            cid_to_element,
        );

        // Update parent's children
        self.texts[text_idx].base.children.extend(caption_refs);
    }

    /// Process a table element and its captions
    fn process_table_element(
        &mut self,
        table_elem: &TableElement,
        page_no: usize,
        page_height: f32,
        cid_to_element: &HashMap<usize, &PageElement>,
    ) {
        let table_idx = self.tables.len();
        let self_ref = format!("#/tables/{table_idx}");
        let key = (page_no, table_elem.cluster.id);
        self.cid_to_ref.insert(key, self_ref.clone());

        self.tables.push(convert_table_element(
            table_elem,
            &self_ref,
            page_no,
            page_height,
        ));

        // Add captions immediately after parent
        let caption_refs = self.add_captions(
            &table_elem.captions,
            &self_ref,
            page_no,
            page_height,
            cid_to_element,
        );

        // Update parent's children and captions field
        for caption_ref in caption_refs {
            self.tables[table_idx]
                .base
                .children
                .push(caption_ref.clone());
            self.tables[table_idx].captions.push(caption_ref);
        }
    }

    /// Process a figure element, its captions, and text cells
    fn process_figure_element(
        &mut self,
        fig_elem: &FigureElement,
        page_no: usize,
        page_height: f32,
        cid_to_element: &HashMap<usize, &PageElement>,
    ) {
        let picture_idx = self.pictures.len();
        let self_ref = format!("#/pictures/{picture_idx}");
        let key = (page_no, fig_elem.cluster.id);
        self.cid_to_ref.insert(key, self_ref.clone());

        self.pictures.push(convert_figure_element(
            fig_elem,
            &self_ref,
            page_no,
            page_height,
        ));

        // Add captions immediately after parent
        let caption_refs = self.add_captions(
            &fig_elem.captions,
            &self_ref,
            page_no,
            page_height,
            cid_to_element,
        );

        // Update parent's children and captions field
        for caption_ref in caption_refs {
            self.pictures[picture_idx]
                .base
                .children
                .push(caption_ref.clone());
            self.pictures[picture_idx].captions.push(caption_ref);
        }

        // Collect OCR text from cells inside the figure (charts, diagrams, graphs, etc.).
        // N=4401: Improve ordering by grouping cells into lines and sorting left-to-right.
        let mut ocr_lines: Vec<String> = Vec::new();

        for line in group_text_cells_into_lines(&fig_elem.cluster.cells, page_height) {
            let mut line_parts: Vec<String> = Vec::new();

            // Add text cells inside the figure as child text items
            for cell in line {
                let cell_idx = self.texts.len();
                let cell_self_ref = format!("#/texts/{cell_idx}");

                let mut cell_text_item =
                    convert_text_cell_to_text_item(cell, &cell_self_ref, page_no, page_height);
                cell_text_item.base.parent = Some(RefItem {
                    cref: self_ref.clone(),
                });

                // Collect non-empty text for OCR content
                let normalized = cell.text.split_whitespace().collect::<Vec<_>>().join(" ");
                if !normalized.is_empty() {
                    line_parts.push(normalized);
                }

                self.texts.push(cell_text_item);

                self.pictures[picture_idx].base.children.push(RefItem {
                    cref: cell_self_ref,
                });
            }

            let line_text = line_parts.join(" ");
            if !line_text.is_empty() {
                ocr_lines.push(line_text);
            }
        }

        // Set the collected OCR text on the picture
        // Join with double newlines to create separate paragraphs (matches Python behavior)
        if !ocr_lines.is_empty() {
            self.pictures[picture_idx].ocr_text = Some(ocr_lines.join("\n\n"));
        }
    }
}

/// Check if an element should be excluded from body (page headers/footers)
const fn is_furniture_element(element: &PageElement) -> bool {
    if let PageElement::Text(text_elem) = element {
        matches!(
            text_elem.label,
            DocItemLabel::PageHeader | DocItemLabel::PageFooter
        )
    } else {
        false
    }
}

/// Build body children for a page, iterating in reading order
fn build_body_children_for_page(
    page: &crate::pipeline::Page,
    reading_order: &[usize],
    caption_cids: &HashSet<(usize, usize)>,
    cid_to_ref: &HashMap<(usize, usize), String>,
) -> Vec<RefItem> {
    let mut body_children = Vec::new();

    let Some(assembled) = &page.assembled else {
        return body_children;
    };

    // Create CID-to-element map for this page
    let cid_to_element: HashMap<usize, &PageElement> = assembled
        .elements
        .iter()
        .map(|e| (e.cluster().id, e))
        .collect();

    // Determine iteration order
    let cids_in_order: Vec<usize> = if reading_order.is_empty() {
        assembled.elements.iter().map(|e| e.cluster().id).collect()
    } else {
        reading_order.to_vec()
    };

    for cid in cids_in_order {
        let key = (page.page_no, cid);

        // Skip captions/footnotes (they have non-body parents)
        if caption_cids.contains(&key) {
            continue;
        }

        // Skip page headers/footers (they go in furniture)
        if let Some(element) = cid_to_element.get(&cid) {
            if is_furniture_element(element) {
                continue;
            }
        }

        if let Some(element_ref) = cid_to_ref.get(&key) {
            body_children.push(RefItem {
                cref: element_ref.clone(),
            });
        }
    }

    body_children
}

/// Convert pipeline `DocItemLabel` to `docling_document` `DocItemLabel`
const fn convert_label(label: DocItemLabel) -> DocLabel {
    match label {
        DocItemLabel::Text => DocLabel::Text,
        DocItemLabel::SectionHeader => DocLabel::SectionHeader,
        DocItemLabel::PageHeader => DocLabel::PageHeader,
        DocItemLabel::PageFooter => DocLabel::PageFooter,
        DocItemLabel::Title => DocLabel::Title,
        DocItemLabel::Caption => DocLabel::Caption,
        DocItemLabel::Footnote => DocLabel::Footnote,
        DocItemLabel::Table => DocLabel::Table,
        DocItemLabel::Figure | DocItemLabel::Picture => DocLabel::Picture,
        DocItemLabel::Formula => DocLabel::Formula,
        DocItemLabel::ListItem => DocLabel::ListItem,
        DocItemLabel::Code => DocLabel::Code,
        DocItemLabel::CheckboxSelected => DocLabel::CheckboxSelected,
        DocItemLabel::CheckboxUnselected => DocLabel::CheckboxUnselected,
        DocItemLabel::Form => DocLabel::Form,
        DocItemLabel::KeyValueRegion => DocLabel::KeyValueRegion,
        DocItemLabel::DocumentIndex => DocLabel::DocumentIndex,
    }
}

/// Convert `Vec<Page>` to `DoclingDocument` (multi-page support)
///
/// # Arguments
/// * `pages` - Pages with assembled elements
/// * `page_reading_orders` - Per-page reading order (CIDs in order), one `Vec<usize>` per page
/// * `filename` - PDF filename for document metadata
///
/// # Reading Order
/// Elements must be exported in reading order to match Python Docling's behavior.
/// The `page_reading_orders` parameter specifies the CID order for each page.
///
/// # Panics
///
/// Panics if any page does not have a size set (`page.size` is `None`).
/// All pages should have their size set during PDF rendering.
#[must_use = "returns the converted DoclingDocument"]
#[allow(clippy::too_many_lines)]
pub fn to_docling_document_multi(
    pages: &[crate::pipeline::Page],
    page_reading_orders: &[Vec<usize>],
    filename: &str,
) -> DoclingDocument {
    let mut collector = ExportCollector::new();
    let mut pages_map = HashMap::new();

    // Pre-pass: Build skippable_cids set (captions, footnotes)
    let skippable_cids = build_skippable_cids(pages);

    // N=4393: Pre-pass: Build container bboxes per page (tables, figures)
    // Used to skip text elements that are contained within tables/figures
    // N=4430: container_bboxes no longer used - filter was removed
    // let container_bboxes = build_container_bboxes(pages);

    // N=4422: Pre-pass: Build figure-only bboxes per page
    // Used to skip tables that are contained within figures (extracted from figure images)
    // N=4431: REMOVED - filter caused loss of legitimate tables
    // let figure_bboxes = build_figure_bboxes(pages);

    // First pass: Create all items and build CID-to-ref map
    // Iterate in READING ORDER, skip captions/footnotes (added after parent)
    for (page_idx, page) in pages.iter().enumerate() {
        let Some(assembled) = &page.assembled else {
            continue;
        };

        let page_size = page.size.expect("Page must have size");
        let page_height = page_size.height;

        // Get reading order for this page (fallback to empty for assembly order)
        let empty_order = Vec::new();
        let reading_order = page_reading_orders.get(page_idx).unwrap_or(&empty_order);

        // Create CID-to-element map for this page
        let cid_to_element: HashMap<usize, &PageElement> = assembled
            .elements
            .iter()
            .map(|e| (e.cluster().id, e))
            .collect();

        // Determine element iteration order
        let elements_in_order: Vec<&PageElement> = if reading_order.is_empty() {
            assembled.elements.iter().collect()
        } else {
            reading_order
                .iter()
                .filter_map(|cid| cid_to_element.get(cid).copied())
                .collect()
        };

        // N=4393: Container bbox filtering was REMOVED (N=4430)
        // N=4422: Figure bbox filtering was REMOVED (N=4431)
        // See MANAGER_DIRECTIVE_2026-01-06_CLEANUP_SPRINT.md
        // If duplication occurs, fix it in Stage08 overlap resolver, not export.

        for element in elements_in_order {
            let key = (page.page_no, element.cluster().id);

            // Skip captions/footnotes (will be added after their parent)
            if skippable_cids.contains(&key) {
                continue;
            }

            match element {
                PageElement::Text(text_elem) => {
                    // N=4430: REMOVED is_text_inside_container filter (was N=4393)
                    // The filter was WRONG - it removed 65% of legitimate content!
                    // Evidence: 2203.01017v2.pdf page 14 went from 511 text items to 0.
                    // If duplication occurs, fix it in Stage08 overlap resolver, not export.
                    //
                    // See MANAGER_DIRECTIVE_2026-01-06_CLEANUP_SPRINT.md

                    collector.process_text_element(
                        text_elem,
                        page.page_no,
                        page_height,
                        &cid_to_element,
                    );
                }
                PageElement::Table(table_elem) => {
                    // N=4431: REMOVED is_table_inside_figure filter (was N=4422)
                    // The filter removed legitimate tables.
                    // If duplicates occur, fix in Stage08 overlap resolver, not export.
                    // See MANAGER_DIRECTIVE_2026-01-06_CLEANUP_SPRINT.md

                    collector.process_table_element(
                        table_elem,
                        page.page_no,
                        page_height,
                        &cid_to_element,
                    );
                }
                PageElement::Figure(fig_elem) => {
                    collector.process_figure_element(
                        fig_elem,
                        page.page_no,
                        page_height,
                        &cid_to_element,
                    );
                }
                PageElement::Container(_) => {
                    // Container elements not exported (reserved for future hierarchical structure)
                }
            }
        }

        // Add page to pages map
        pages_map.insert(
            (page.page_no + 1).to_string(),
            PageItem {
                page_no: (page.page_no + 1) as i32,
                size: crate::docling_document::Size {
                    width: f64::from(page_size.width),
                    height: f64::from(page_size.height),
                },
            },
        );
    }

    // Second pass: Build body.children (excludes captions/footnotes/furniture)
    let mut body_children = Vec::new();
    for (page_idx, page) in pages.iter().enumerate() {
        let empty_order = Vec::new();
        let reading_order = page_reading_orders.get(page_idx).unwrap_or(&empty_order);

        let page_children = build_body_children_for_page(
            page,
            reading_order,
            &skippable_cids,
            &collector.cid_to_ref,
        );

        body_children.extend(page_children);
    }

    // N=4407: Third pass - add any elements in cid_to_ref that weren't added to body_children
    // This catches elements that might have been missed due to reading order issues
    let body_refs: std::collections::HashSet<_> = body_children.iter().map(|r| &r.cref).collect();
    let mut missing_refs: Vec<_> = collector
        .cid_to_ref
        .iter()
        .filter(|(key, ref_path)| {
            // Skip captions/footnotes (they have non-body parents)
            if skippable_cids.contains(key) {
                return false;
            }
            // Skip if already added
            if body_refs.contains(ref_path) {
                return false;
            }
            // N=4408: Skip furniture items (they go in furniture, not body)
            // Check content_layer of the actual item in collector.texts
            if ref_path.starts_with("#/texts/") {
                if let Some(idx_str) = ref_path.strip_prefix("#/texts/") {
                    if let Ok(idx) = idx_str.parse::<usize>() {
                        if let Some(text_item) = collector.texts.get(idx) {
                            if text_item.base.content_layer == ContentLayer::Furniture {
                                return false; // Skip furniture items
                            }
                        }
                    }
                }
            }
            true
        })
        .map(|(key, ref_path)| (*key, ref_path.clone()))
        .collect();

    if !missing_refs.is_empty() {
        // Sort by page number, then by CID for deterministic order
        missing_refs.sort_by_key(|(key, _)| *key);

        log::debug!(
            "N=4407: Adding {} missing elements to body.children",
            missing_refs.len()
        );

        for (_, ref_path) in missing_refs {
            body_children.push(RefItem { cref: ref_path });
        }
    }

    // Create body GroupItem
    let body = GroupItem {
        base: NodeItemBase {
            self_ref: "#/body".to_string(),
            parent: None,
            children: body_children,
            content_layer: ContentLayer::Body,
        },
        name: "body".to_string(),
        label: GroupLabel::Unspecified,
    };

    // Create document origin
    let origin = DocumentOrigin {
        mimetype: "application/pdf".to_string(),
        binary_hash: 0, // Hash computation not implemented (would need PDF file access)
        filename: filename.to_string(),
        uri: None,
    };

    DoclingDocument {
        schema_name: DOCLING_DOCUMENT_SCHEMA_NAME.to_string(),
        version: DOCLING_DOCUMENT_SCHEMA_VERSION.to_string(),
        name: filename.to_string(),
        origin: Some(origin),
        body,
        furniture: None,
        groups: Vec::new(),
        texts: collector.texts,
        tables: collector.tables,
        pictures: collector.pictures,
        key_value_items: Vec::new(),
        form_items: Vec::new(),
        pages: pages_map,
        num_pages: pages.len() as i32,
        markdown: None,
    }
}

/// Convert single `AssembledUnit` to `DoclingDocument` (legacy single-page API)
#[must_use = "returns the converted DoclingDocument"]
pub fn to_docling_document(
    elements: &[PageElement],
    page_no: usize,
    page_size: PageSize,
    filename: &str,
) -> DoclingDocument {
    // Create a single Page for legacy API
    let page = crate::pipeline::Page {
        page_no,
        size: Some(page_size),
        predictions: crate::pipeline::PagePredictions::default(),
        assembled: Some(crate::pipeline::AssembledUnit {
            elements: elements.to_vec(),
            body: elements.to_vec(),
            headers: Vec::new(),
        }),
    };

    // Legacy API: use assembly order (no reading order specified)
    let reading_order = Vec::new();
    to_docling_document_multi(&[page], &[reading_order], filename)
}

/// Detect list item marker and enumeration from text content
///
/// Returns (`marker`, `enumerated`, `text_without_marker`) tuple:
/// - `marker`: The extracted marker (e.g., "1."), or None if not detected
/// - `enumerated`: true if numbered list, false if bulleted
/// - `text_without_marker`: Text with marker prefix removed
///
/// N=4414: Python docling v2.58.0 behavior for lists:
/// - Numbered lists (1., 2., etc.): Extract marker to `marker` field, remove from `text`
/// - Bullet lists (∞, •, etc.): Do NOT extract marker, keep bullet in `text`, leave `marker` empty
///   This matches the groundtruth JSON where bullet list items have marker="" but text contains "∞ ..."
fn detect_list_marker(text: &str) -> (Option<String>, bool, String) {
    let trimmed = text.trim_start();

    // Check for numbered markers like "1.", "2.", "10."
    // For numbered lists, Python DOES extract the marker and remove it from text
    if let Some(dot_pos) = trimmed.find('.') {
        let prefix = &trimmed[..dot_pos];
        if !prefix.is_empty()
            && prefix.chars().all(|c| c.is_ascii_digit())
            && trimmed.len() > dot_pos + 1
        {
            let marker = format!("{prefix}.");
            let rest = trimmed[dot_pos + 1..].trim_start().to_string();
            return (Some(marker), true, rest);
        }
    }

    // N=4414: For bullet markers (∞, •, etc.), Python does NOT extract them
    // The bullet stays in text and marker field is empty
    // Check if it starts with a bullet character to set enumerated=false
    if let Some(first_char) = trimmed.chars().next() {
        if matches!(
            first_char,
            '∞' | '•' | '‣' | '◦' | '▪' | '‐' | '–' | '—' | '⁃' | '◘' | '○' | '●'
        ) {
            // Return marker as empty (None), enumerated=false, and keep text as-is with bullet
            return (None, false, text.to_string());
        }
    }

    // No marker detected - return original text
    (None, false, text.to_string())
}

/// Convert Rust `TextElement` to `DoclingDocument` `TextItem`
fn convert_text_element(
    elem: &TextElement,
    self_ref: &str,
    page_no: usize,
    page_height: f32,
) -> TextItem {
    // Convert provenance
    let prov = vec![ProvenanceItem {
        page_no: (page_no + 1) as i32, // 1-indexed
        bbox: convert_bbox(&elem.cluster.bbox, page_height),
        charspan: (0, elem.orig.len()), // Charspan based on original unsanitized text
    }];

    // Determine level for headers using text-based heuristics
    // N=4355: Use detect_header_level to infer H1/H2/H3 from numbering patterns
    let level = match elem.label {
        DocItemLabel::SectionHeader | DocItemLabel::Title => {
            Some(detect_header_level(&elem.text, elem.label, page_no) as i32)
        }
        _ => None,
    };

    // Determine content layer based on label (page headers/footers are furniture)
    let (content_layer, parent_ref) = match elem.label {
        DocItemLabel::PageHeader | DocItemLabel::PageFooter => {
            (ContentLayer::Furniture, "#/furniture".to_string())
        }
        _ => (ContentLayer::Body, "#/body".to_string()),
    };

    // Detect list marker and enumeration for ListItem elements
    // For Formula: text is empty (formula not decoded), orig contains OCR text
    let (marker, enumerated, text) = if elem.label == DocItemLabel::ListItem {
        let (marker, enumerated, text_without_marker) = detect_list_marker(&elem.text);
        (marker, Some(enumerated), text_without_marker)
    } else if elem.label == DocItemLabel::Formula {
        // Formula text is empty - we don't decode formulas to LaTeX/MathML
        // orig field contains the OCR text for reference
        (None, None, String::new())
    } else {
        (None, None, elem.text.clone())
    };

    // N=4414: Python docling v2.58.0 doesn't apply bold/italic formatting to list items
    // The expected JSON shows formatting: null for all list items
    let (is_bold, is_italic) = if elem.label == DocItemLabel::ListItem {
        (false, false)
    } else {
        // N=4378: Preserve bold/italic formatting from TextElement for non-list items
        (elem.is_bold, elem.is_italic)
    };

    TextItem {
        base: NodeItemBase {
            self_ref: self_ref.to_string(),
            parent: Some(RefItem { cref: parent_ref }),
            children: Vec::new(),
            content_layer,
        },
        label: convert_label(elem.label),
        orig: elem.orig.clone(), // Original unsanitized text
        text,                    // Text (with marker removed for ListItems)
        prov,
        level,
        enumerated,
        marker,
        code_language: None,
        is_bold,
        is_italic,
    }
}

/// Convert Rust `TableElement` to `DoclingDocument` `TableItem`
fn convert_table_element(
    elem: &TableElement,
    self_ref: &str,
    page_no: usize,
    page_height: f32,
) -> TableItem {
    // Convert provenance
    let prov = vec![ProvenanceItem {
        page_no: (page_no + 1) as i32, // 1-indexed
        bbox: convert_bbox(&elem.cluster.bbox, page_height),
        charspan: (0, elem.text.as_ref().map_or(0, String::len)), // NOTE: Charspan approximated as (0, text_len)
                                                                  // Rationale: Actual character positions in source PDF not tracked through pipeline.
                                                                  // Would require: OCR/text extraction to record byte offsets in original file.
    }];

    // Convert table cells
    let table_cells = elem
        .table_cells
        .iter()
        .map(|cell| convert_table_cell(cell, page_height))
        .collect();

    TableItem {
        base: NodeItemBase {
            self_ref: self_ref.to_string(),
            parent: Some(RefItem {
                cref: "#/body".to_string(),
            }),
            children: Vec::new(),
            content_layer: ContentLayer::Body,
        },
        label: DocLabel::Table,
        data: TableData {
            num_rows: elem.num_rows as i32,
            num_cols: elem.num_cols as i32,
            table_cells,
        },
        prov,
        captions: Vec::new(), // Initialized empty; populated by process_table_element() after conversion
    }
}

/// Convert Rust `TableCell` to `DoclingDocument` `TableCell`
fn convert_table_cell(cell: &TableCell, page_height: f32) -> DocTableCell {
    DocTableCell {
        text: cell.text.clone(),
        row_span: cell.row_span as i32,
        col_span: cell.col_span as i32,
        start_row_offset_idx: cell.start_row_offset_idx as i32,
        end_row_offset_idx: cell.end_row_offset_idx as i32,
        start_col_offset_idx: cell.start_col_offset_idx as i32,
        end_col_offset_idx: cell.end_col_offset_idx as i32,
        // CRITICAL FIX (Issue T6/F32): Use header flags from TableFormer output
        // Python: docling_ibm_models/tableformer classifies cells via ched/rhed tags
        column_header: cell.column_header,
        row_header: cell.row_header,
        bbox: Some(convert_bbox(&cell.bbox, page_height)),
        // Issue #18 FIX: Propagate OCR metadata from pipeline
        from_ocr: cell.from_ocr,
        confidence: cell.confidence,
    }
}

/// Convert Rust `FigureElement` to `DoclingDocument` `PictureItem`
fn convert_figure_element(
    elem: &FigureElement,
    self_ref: &str,
    page_no: usize,
    page_height: f32,
) -> PictureItem {
    // Convert provenance
    let prov = vec![ProvenanceItem {
        page_no: (page_no + 1) as i32, // 1-indexed
        bbox: convert_bbox(&elem.cluster.bbox, page_height),
        charspan: (0, 0), // Pictures don't have text
    }];

    PictureItem {
        base: NodeItemBase {
            self_ref: self_ref.to_string(),
            parent: Some(RefItem {
                cref: "#/body".to_string(),
            }),
            children: Vec::new(),
            content_layer: ContentLayer::Body,
        },
        label: convert_label(elem.label),
        prov,
        captions: Vec::new(), // Initialized empty; populated by process_figure_element() after conversion
        annotations: Vec::new(),
        ocr_text: None, // Populated by process_figure_element() with collected OCR cells
    }
}

/// Convert Rust `TextCell` to `DoclingDocument` `TextItem`
///
/// Creates a text item from a text cell (used for cells inside figures/tables).
/// These are the individual OCR or programmatic text fragments.
fn convert_text_cell_to_text_item(
    cell: &crate::pipeline::TextCell,
    self_ref: &str,
    page_no: usize,
    page_height: f32,
) -> TextItem {
    // Convert BoundingRectangle to BoundingBox (axis-aligned)
    let bbox = cell.rect.to_bbox();

    // Convert provenance
    let prov = vec![ProvenanceItem {
        page_no: (page_no + 1) as i32, // 1-indexed
        bbox: convert_bbox(&bbox, page_height),
        charspan: (0, cell.text.len()),
    }];

    TextItem {
        base: NodeItemBase {
            self_ref: self_ref.to_string(),
            parent: None, // Will be set by caller
            children: Vec::new(),
            content_layer: ContentLayer::Body,
        },
        label: DocLabel::Text, // Text cells are always labeled as "text"
        orig: cell.text.clone(),
        text: cell.text.clone(),
        prov,
        level: None,
        enumerated: None,
        marker: None,
        code_language: None,
        // N=4378: Preserve bold/italic from TextCell
        is_bold: cell.is_bold,
        is_italic: cell.is_italic,
    }
}

/// Convert `TableCell` (from `TableFormer`) to `DoclingDocument` `TextItem`
///
/// Creates a text item from a `TableFormer` table cell (structured cell with row/col info).
/// These cells have text content extracted from the table and include structural metadata.
fn convert_table_cell_to_text_item(
    cell: &TableCell,
    self_ref: &str,
    page_no: usize,
    page_height: f32,
) -> TextItem {
    // Convert provenance
    let prov = vec![ProvenanceItem {
        page_no: (page_no + 1) as i32, // 1-indexed
        bbox: convert_bbox(&cell.bbox, page_height),
        charspan: (0, cell.text.len()),
    }];

    TextItem {
        base: NodeItemBase {
            self_ref: self_ref.to_string(),
            parent: None, // Will be set by caller
            children: Vec::new(),
            content_layer: ContentLayer::Body,
        },
        label: DocLabel::Text, // Table cells are labeled as "text"
        orig: cell.text.clone(),
        text: cell.text.clone(),
        prov,
        level: None,
        enumerated: None,
        marker: None,
        code_language: None,
        // N=4378: Table cells don't currently have bold/italic info from TableFormer
        is_bold: false,
        is_italic: false,
    }
}

/// Convert Rust `BoundingBox` to `DoclingDocument` `BoundingBox` (BOTTOMLEFT origin)
fn convert_bbox(bbox: &BoundingBox, page_height: f32) -> DocBBox {
    // Convert to BOTTOMLEFT origin if needed
    let converted = bbox.to_bottom_left_origin(page_height);

    DocBBox {
        l: f64::from(converted.l),
        t: f64::from(converted.t),
        r: f64::from(converted.r),
        b: f64::from(converted.b),
        coord_origin: DocCoordOrigin::Bottomleft,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::{BoundingRectangle, Cluster, CoordOrigin};

    fn rect(l: f32, t: f32, r: f32, b: f32, coord_origin: CoordOrigin) -> BoundingRectangle {
        BoundingRectangle {
            r_x0: l,
            r_y0: t,
            r_x1: r,
            r_y1: t,
            r_x2: r,
            r_y2: b,
            r_x3: l,
            r_y3: b,
            coord_origin,
        }
    }

    #[test]
    fn test_bbox_conversion() {
        let bbox = BoundingBox {
            l: 100.0,
            t: 50.0,
            r: 200.0,
            b: 150.0,
            coord_origin: CoordOrigin::TopLeft,
        };
        let page_height = 800.0;

        let doc_bbox = convert_bbox(&bbox, page_height);

        // TOPLEFT (l=100, t=50, r=200, b=150) with height 800
        // -> BOTTOMLEFT: l=100, t=800-50=750, r=200, b=800-150=650
        // Note: In BOTTOMLEFT, t > b (top is further from bottom origin)
        assert_eq!(doc_bbox.l, 100.0);
        assert_eq!(doc_bbox.t, 750.0);
        assert_eq!(doc_bbox.r, 200.0);
        assert_eq!(doc_bbox.b, 650.0);
        assert_eq!(doc_bbox.coord_origin, DocCoordOrigin::Bottomleft);
    }

    #[test]
    fn test_group_text_cells_into_lines_top_left_origin() {
        let page_height = 100.0;

        let cells = vec![
            TextCell {
                index: 0,
                text: "world".to_string(),
                rect: rect(60.0, 10.0, 100.0, 20.0, CoordOrigin::TopLeft),
                confidence: None,
                from_ocr: true,
                is_bold: false,
                is_italic: false,
            },
            TextCell {
                index: 1,
                text: "Hello".to_string(),
                rect: rect(10.0, 10.0, 50.0, 20.0, CoordOrigin::TopLeft),
                confidence: None,
                from_ocr: true,
                is_bold: false,
                is_italic: false,
            },
            TextCell {
                index: 2,
                text: "line".to_string(),
                rect: rect(70.0, 40.0, 100.0, 50.0, CoordOrigin::TopLeft),
                confidence: None,
                from_ocr: true,
                is_bold: false,
                is_italic: false,
            },
            TextCell {
                index: 3,
                text: "Second".to_string(),
                rect: rect(10.0, 40.0, 60.0, 50.0, CoordOrigin::TopLeft),
                confidence: None,
                from_ocr: true,
                is_bold: false,
                is_italic: false,
            },
        ];

        let lines = group_text_cells_into_lines(&cells, page_height);
        let line_texts: Vec<String> = lines
            .into_iter()
            .map(|line| {
                line.into_iter()
                    .map(|c| c.text.as_str())
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect();

        assert_eq!(line_texts, vec!["Hello world", "Second line"]);
    }

    #[test]
    fn test_group_text_cells_into_lines_bottom_left_origin() {
        let page_height = 100.0;

        // Bottom-left origin: higher y is closer to top of page.
        // These should end up ordered as: "Hello world" then "Second line".
        let cells = vec![
            TextCell {
                index: 0,
                text: "Second".to_string(),
                rect: rect(10.0, 60.0, 60.0, 50.0, CoordOrigin::BottomLeft),
                confidence: None,
                from_ocr: true,
                is_bold: false,
                is_italic: false,
            },
            TextCell {
                index: 1,
                text: "line".to_string(),
                rect: rect(70.0, 60.0, 100.0, 50.0, CoordOrigin::BottomLeft),
                confidence: None,
                from_ocr: true,
                is_bold: false,
                is_italic: false,
            },
            TextCell {
                index: 2,
                text: "world".to_string(),
                rect: rect(60.0, 90.0, 100.0, 80.0, CoordOrigin::BottomLeft),
                confidence: None,
                from_ocr: true,
                is_bold: false,
                is_italic: false,
            },
            TextCell {
                index: 3,
                text: "Hello".to_string(),
                rect: rect(10.0, 90.0, 50.0, 80.0, CoordOrigin::BottomLeft),
                confidence: None,
                from_ocr: true,
                is_bold: false,
                is_italic: false,
            },
        ];

        let lines = group_text_cells_into_lines(&cells, page_height);
        let line_texts: Vec<String> = lines
            .into_iter()
            .map(|line| {
                line.into_iter()
                    .map(|c| c.text.as_str())
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect();

        assert_eq!(line_texts, vec!["Hello world", "Second line"]);
    }

    #[test]
    fn test_text_element_conversion() {
        let text_elem = TextElement {
            label: DocItemLabel::Text,
            id: 0,
            page_no: 0,
            text: "Test text".to_string(),
            orig: "Test text".to_string(),
            cluster: Cluster {
                id: 0,
                label: DocItemLabel::Text,
                bbox: BoundingBox {
                    l: 100.0,
                    t: 50.0,
                    r: 200.0,
                    b: 150.0,
                    coord_origin: CoordOrigin::TopLeft,
                },
                confidence: 0.99,
                cells: Vec::new(),
                children: Vec::new(),
            },
            captions: Vec::new(),
            footnotes: Vec::new(),
            is_bold: false,
            is_italic: false,
        };

        let text_item = convert_text_element(&text_elem, "#/texts/0", 0, 800.0);

        assert_eq!(text_item.text, "Test text");
        assert_eq!(text_item.orig, "Test text");
        assert_eq!(text_item.label, DocLabel::Text);
        assert_eq!(text_item.base.self_ref, "#/texts/0");
        assert_eq!(text_item.prov.len(), 1);
        assert_eq!(text_item.prov[0].page_no, 1); // 1-indexed
        assert_eq!(text_item.prov[0].charspan, (0, 9)); // charspan matches orig length
    }

    #[test]
    fn test_charspan_uses_orig_length() {
        // Test that charspan uses orig.len(), not text.len()
        // This is important when text is sanitized differently from orig
        let text_elem = TextElement {
            label: DocItemLabel::Text,
            id: 0,
            page_no: 0,
            text: "sanitized".to_string(),
            orig: "original unsanitized".to_string(), // Different length
            cluster: Cluster {
                id: 0,
                label: DocItemLabel::Text,
                bbox: BoundingBox {
                    l: 100.0,
                    t: 50.0,
                    r: 200.0,
                    b: 150.0,
                    coord_origin: CoordOrigin::TopLeft,
                },
                confidence: 0.99,
                cells: Vec::new(),
                children: Vec::new(),
            },
            captions: Vec::new(),
            footnotes: Vec::new(),
            is_bold: false,
            is_italic: false,
        };

        let text_item = convert_text_element(&text_elem, "#/texts/0", 0, 800.0);

        assert_eq!(text_item.text, "sanitized");
        assert_eq!(text_item.orig, "original unsanitized");
        // Charspan should be [0, 20] (orig.len()), NOT [0, 9] (text.len())
        assert_eq!(text_item.prov[0].charspan, (0, 20));
    }

    // N=4393: Tests for text-inside-container containment logic
    #[test]
    fn test_is_text_inside_container_fully_contained() {
        // Text box fully contained within table box (100% containment)
        let text_bbox = BoundingBox {
            l: 100.0,
            t: 100.0,
            r: 200.0,
            b: 150.0,
            coord_origin: CoordOrigin::TopLeft,
        };
        let table_bbox = BoundingBox {
            l: 50.0,
            t: 50.0,
            r: 300.0,
            b: 300.0,
            coord_origin: CoordOrigin::TopLeft,
        };

        assert!(is_text_inside_container(&text_bbox, &[table_bbox]));
    }

    #[test]
    fn test_is_text_inside_container_not_contained() {
        // Text box completely outside table box (0% containment)
        let text_bbox = BoundingBox {
            l: 400.0,
            t: 100.0,
            r: 500.0,
            b: 150.0,
            coord_origin: CoordOrigin::TopLeft,
        };
        let table_bbox = BoundingBox {
            l: 50.0,
            t: 50.0,
            r: 300.0,
            b: 300.0,
            coord_origin: CoordOrigin::TopLeft,
        };

        assert!(!is_text_inside_container(&text_bbox, &[table_bbox]));
    }

    #[test]
    fn test_is_text_inside_container_partially_contained() {
        // Text box 50% overlapping table (under 80% threshold)
        let text_bbox = BoundingBox {
            l: 250.0,
            t: 100.0,
            r: 350.0,
            b: 150.0,
            coord_origin: CoordOrigin::TopLeft,
        };
        let table_bbox = BoundingBox {
            l: 50.0,
            t: 50.0,
            r: 300.0,
            b: 300.0,
            coord_origin: CoordOrigin::TopLeft,
        };
        // Text bbox area: 100x50 = 5000
        // Intersection: (250-300) x (100-150) = 50x50 = 2500
        // Containment: 2500/5000 = 0.5 < 0.8 threshold

        assert!(!is_text_inside_container(&text_bbox, &[table_bbox]));
    }

    #[test]
    fn test_is_text_inside_container_above_threshold() {
        // Text box 90% inside table (above 80% threshold)
        let text_bbox = BoundingBox {
            l: 100.0,
            t: 100.0,
            r: 200.0,
            b: 150.0,
            coord_origin: CoordOrigin::TopLeft,
        };
        let table_bbox = BoundingBox {
            l: 50.0,
            t: 50.0,
            r: 300.0,
            b: 145.0, // Cuts off 5px at bottom of text
            coord_origin: CoordOrigin::TopLeft,
        };
        // Text bbox area: 100x50 = 5000
        // Intersection: 100x45 = 4500
        // Containment: 4500/5000 = 0.9 > 0.8 threshold

        assert!(is_text_inside_container(&text_bbox, &[table_bbox]));
    }

    // N=4422: Tests for table-inside-figure containment logic
    #[test]
    fn test_is_table_inside_figure_fully_contained() {
        // Table box fully contained within figure box (100% containment)
        let table_bbox = BoundingBox {
            l: 184.0,
            t: 347.0,
            r: 436.0,
            b: 291.0,
            coord_origin: CoordOrigin::BottomLeft,
        };
        let figure_bbox = BoundingBox {
            l: 162.0,
            t: 348.0,
            r: 451.0,
            b: 129.0,
            coord_origin: CoordOrigin::BottomLeft,
        };

        assert!(is_table_inside_figure(&table_bbox, &[figure_bbox]));
    }

    #[test]
    fn test_is_table_inside_figure_not_contained() {
        // Table box completely outside figure box (0% containment)
        let table_bbox = BoundingBox {
            l: 500.0,
            t: 100.0,
            r: 600.0,
            b: 50.0,
            coord_origin: CoordOrigin::BottomLeft,
        };
        let figure_bbox = BoundingBox {
            l: 50.0,
            t: 300.0,
            r: 200.0,
            b: 100.0,
            coord_origin: CoordOrigin::BottomLeft,
        };

        assert!(!is_table_inside_figure(&table_bbox, &[figure_bbox]));
    }

    #[test]
    fn test_is_table_inside_figure_partially_contained() {
        // Table box 50% overlapping figure (under 80% threshold)
        let table_bbox = BoundingBox {
            l: 150.0,
            t: 200.0,
            r: 250.0,
            b: 100.0,
            coord_origin: CoordOrigin::BottomLeft,
        };
        let figure_bbox = BoundingBox {
            l: 50.0,
            t: 200.0,
            r: 200.0,
            b: 100.0,
            coord_origin: CoordOrigin::BottomLeft,
        };
        // Table bbox area: 100x100 = 10000
        // Intersection: (150-200) x (100-200) = 50x100 = 5000
        // Containment: 5000/10000 = 0.5 < 0.8 threshold

        assert!(!is_table_inside_figure(&table_bbox, &[figure_bbox]));
    }
}
