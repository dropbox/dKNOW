//! # Table Structure Model (ONNX) - Microsoft Table Transformer
//!
//! This module implements table structure recognition using Microsoft's Table Transformer
//! model via ONNX Runtime. This is an alternative to the PyTorch-based IBM `TableFormer`
//! that avoids libtorch crashes.
//!
//! Note: Infrastructure code - some helpers ported from Python not yet wired up.
#![allow(dead_code)]
// Table dimensions use i64 from ONNX tensors but usize for Rust indexing.
// Values are always non-negative (table row/column counts).
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_precision_loss)]
//!
//! ## Architecture Differences
//!
//! **Microsoft Table Transformer (DETR-based):**
//! - Single forward pass (no autoregressive decoding)
//! - Outputs bounding boxes for rows, columns, headers
//! - Labels: table column, table row, table column header, etc.
//! - ONNX compatible (no beam search)
//!
//! **IBM `TableFormer` (Original):**
//! - Autoregressive decoder with beam search
//! - Outputs cell-by-cell tag sequence (fcel, ecel, nl, etc.)
//! - NOT ONNX compatible
//!
//! ## Post-processing Strategy
//!
//! Since Microsoft model detects rows/columns instead of cells:
//! 1. Filter detections by confidence threshold
//! 2. Apply NMS (Non-Maximum Suppression)
//! 3. Separate rows vs columns based on label
//! 4. Compute cell grid from row/column intersections
//! 5. Match OCR text to detected cells

use crate::error::{DoclingError, Result};
use crate::pipeline::{BoundingBox, Cluster, CoordOrigin, DocItemLabel, TableCell, TableElement};

/// `IoU` threshold for suppressing overlapping *row* intervals.
///
/// Rows can be tightly packed; using a higher threshold avoids incorrectly suppressing
/// distinct adjacent rows (e.g., when a cell contains two stacked lines of text).
const ROW_INTERVAL_MERGE_IOU_THRESHOLD: f32 = 0.6;

/// `IoU` threshold for suppressing overlapping *column* intervals.
///
/// Columns tend to have cleaner separation; a lower threshold is effective at removing
/// duplicate detections without collapsing true distinct columns.
const COL_INTERVAL_MERGE_IOU_THRESHOLD: f32 = 0.3;

/// OCR metadata for table cell text (F72: Table-OCR linkage)
///
/// Contains information about whether text came from OCR and the associated confidence scores.
struct OcrTextMatch {
    /// Concatenated text from matching OCR cells
    text: String,
    /// Whether any text came from OCR (true if any matching cell had `from_ocr=true`)
    from_ocr: bool,
    /// Average confidence of matching OCR cells (None if no OCR cells matched)
    confidence: Option<f32>,
}
use ndarray::Array4;
use ort::session::{builder::GraphOptimizationLevel, Session};
use std::path::Path;

/// Microsoft Table Transformer labels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TableLabel {
    Table = 0,
    TableColumn = 1,
    TableRow = 2,
    TableColumnHeader = 3,
    TableProjectedRowHeader = 4,
    TableSpanningCell = 5,
    NoObject = 6,
}

impl std::fmt::Display for TableLabel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Table => write!(f, "table"),
            Self::TableColumn => write!(f, "table column"),
            Self::TableRow => write!(f, "table row"),
            Self::TableColumnHeader => write!(f, "table column header"),
            Self::TableProjectedRowHeader => write!(f, "table projected row header"),
            Self::TableSpanningCell => write!(f, "table spanning cell"),
            Self::NoObject => write!(f, "no object"),
        }
    }
}

impl std::str::FromStr for TableLabel {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().replace(['-', '_'], " ").as_str() {
            "table" => Ok(Self::Table),
            "table column" | "tablecolumn" | "column" => Ok(Self::TableColumn),
            "table row" | "tablerow" | "row" => Ok(Self::TableRow),
            "table column header" | "tablecolumnheader" | "column header" | "header" => {
                Ok(Self::TableColumnHeader)
            }
            "table projected row header"
            | "tableprojectedrowheader"
            | "projected row header"
            | "row header" => Ok(Self::TableProjectedRowHeader),
            "table spanning cell" | "tablespanningcell" | "spanning cell" | "span" => {
                Ok(Self::TableSpanningCell)
            }
            "no object" | "noobject" | "none" | "background" => Ok(Self::NoObject),
            _ => Err(format!(
                "Unknown table label '{s}'. Expected: table, table column, table row, \
                 table column header, table projected row header, table spanning cell, no object"
            )),
        }
    }
}

impl TryFrom<usize> for TableLabel {
    type Error = ();

    fn try_from(value: usize) -> std::result::Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Table),
            1 => Ok(Self::TableColumn),
            2 => Ok(Self::TableRow),
            3 => Ok(Self::TableColumnHeader),
            4 => Ok(Self::TableProjectedRowHeader),
            5 => Ok(Self::TableSpanningCell),
            6 => Ok(Self::NoObject),
            _ => Err(()),
        }
    }
}

/// A single detection from the table transformer
#[derive(Debug, Clone, PartialEq)]
pub struct Detection {
    /// Class label
    pub label: TableLabel,
    /// Confidence score
    pub score: f32,
    /// Bounding box (cx, cy, w, h) normalized [0, 1]
    pub bbox: [f32; 4],
}

/// Table Structure Model using ONNX Runtime
///
/// Loads Microsoft's Table Transformer model and performs inference.
pub struct TableStructureModelOnnx {
    session: Session,
    /// Confidence threshold for filtering detections
    pub confidence_threshold: f32,
    /// NMS `IoU` threshold
    pub nms_threshold: f32,
}

impl std::fmt::Debug for TableStructureModelOnnx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TableStructureModelOnnx")
            .field("session", &"<Session>")
            .field("confidence_threshold", &self.confidence_threshold)
            .field("nms_threshold", &self.nms_threshold)
            .finish()
    }
}

impl TableStructureModelOnnx {
    /// Load the Table Transformer ONNX model
    ///
    /// # Arguments
    /// * `model_path` - Path to `table_structure_model.onnx`
    ///
    /// # Example
    /// ```no_run
    /// use docling_pdf_ml::models::table_structure_onnx::TableStructureModelOnnx;
    ///
    /// let model = TableStructureModelOnnx::load("path/to/model.onnx")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn load<P: AsRef<Path>>(model_path: P) -> Result<Self> {
        let model_path = model_path.as_ref();

        if !model_path.exists() {
            return Err(DoclingError::ModelLoadError {
                model_name: "TableStructureOnnx".to_string(),
                source: Box::new(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Model file not found: {}", model_path.display()),
                )),
            });
        }

        let num_threads = num_cpus::get();
        let session = Session::builder()
            .map_err(|e| DoclingError::ModelLoadError {
                model_name: "TableStructureOnnx".to_string(),
                source: Box::new(e),
            })?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| DoclingError::ModelLoadError {
                model_name: "TableStructureOnnx".to_string(),
                source: Box::new(e),
            })?
            .with_intra_threads(num_threads)
            .map_err(|e| DoclingError::ModelLoadError {
                model_name: "TableStructureOnnx".to_string(),
                source: Box::new(e),
            })?
            .commit_from_file(model_path)
            .map_err(|e| DoclingError::ModelLoadError {
                model_name: "TableStructureOnnx".to_string(),
                source: Box::new(e),
            })?;

        log::debug!(
            "Loaded TableStructureOnnx model from: {}",
            model_path.display()
        );

        Ok(Self {
            session,
            confidence_threshold: 0.5,
            nms_threshold: 0.5,
        })
    }

    /// Run inference on a preprocessed table image
    ///
    /// # Arguments
    /// * `image` - Preprocessed image tensor [1, 3, H, W] normalized
    ///
    /// # Returns
    /// Vector of detected rows, columns, and headers
    #[must_use = "table structure prediction returns results that should be processed"]
    pub fn predict(&mut self, image: &Array4<f32>) -> Result<Vec<Detection>> {
        // Convert ndarray to ort Value
        let shape = image.shape().to_vec();
        let data = image
            .as_slice()
            .ok_or_else(|| DoclingError::PreprocessingError {
                reason: "Image array is not contiguous".to_string(),
            })?;

        let input_value = ort::value::Value::from_array((shape.as_slice(), data.to_vec()))
            .map_err(|e| DoclingError::InferenceError {
                model_name: "TableStructureOnnx".to_string(),
                source: Box::new(e),
            })?;

        let outputs = self
            .session
            .run(ort::inputs!["pixel_values" => input_value])
            .map_err(|e| DoclingError::InferenceError {
                model_name: "TableStructureOnnx".to_string(),
                source: Box::new(e),
            })?;

        // Extract outputs
        // logits: [1, 125, 7] - class probabilities
        // pred_boxes: [1, 125, 4] - bounding boxes (cx, cy, w, h)
        let (logits_shape, logits_data) =
            outputs[0]
                .try_extract_tensor::<f32>()
                .map_err(|e| DoclingError::InferenceError {
                    model_name: "TableStructureOnnx".to_string(),
                    source: Box::new(e),
                })?;
        let (_boxes_shape, boxes_data) =
            outputs[1]
                .try_extract_tensor::<f32>()
                .map_err(|e| DoclingError::InferenceError {
                    model_name: "TableStructureOnnx".to_string(),
                    source: Box::new(e),
                })?;

        // Process predictions
        let mut detections = Vec::new();

        // Shape: [1, num_queries, num_classes] for logits
        // Shape: [1, num_queries, 4] for boxes
        let num_queries = logits_shape[1] as usize;
        let num_classes = logits_shape[2] as usize;

        for i in 0..num_queries {
            // Get class probabilities (softmax already applied in model)
            let mut max_score = f32::NEG_INFINITY;
            let mut max_class = 0usize;

            for c in 0..num_classes {
                // Flat index: batch * (queries * classes) + query * classes + class
                let idx = i * num_classes + c;
                let score = logits_data[idx];
                if score > max_score {
                    max_score = score;
                    max_class = c;
                }
            }

            // Skip "no object" class and low confidence
            if max_class == 6 || max_score < self.confidence_threshold {
                continue;
            }

            let label = TableLabel::try_from(max_class).unwrap_or(TableLabel::NoObject);

            // Skip table detections (we're analyzing table structure, not detecting tables)
            if label == TableLabel::Table {
                continue;
            }

            // Boxes flat index: batch * (queries * 4) + query * 4 + coord
            let box_base = i * 4;
            let bbox = [
                boxes_data[box_base],
                boxes_data[box_base + 1],
                boxes_data[box_base + 2],
                boxes_data[box_base + 3],
            ];

            detections.push(Detection {
                label,
                score: max_score,
                bbox,
            });
        }

        // Drop outputs before calling apply_nms to release borrow on session
        drop(outputs);

        // Apply NMS
        let detections = self.apply_nms(detections);

        Ok(detections)
    }

    /// Apply Non-Maximum Suppression to filter overlapping detections
    fn apply_nms(&self, mut detections: Vec<Detection>) -> Vec<Detection> {
        // Sort by confidence score (descending)
        detections.sort_by(|a, b| b.score.total_cmp(&a.score));

        let mut keep = vec![true; detections.len()];

        for i in 0..detections.len() {
            if !keep[i] {
                continue;
            }

            for j in (i + 1)..detections.len() {
                if !keep[j] {
                    continue;
                }

                // Only compare same label type
                if detections[i].label != detections[j].label {
                    continue;
                }

                let iou = compute_iou(&detections[i].bbox, &detections[j].bbox);
                if iou > self.nms_threshold {
                    keep[j] = false;
                }
            }
        }

        detections
            .into_iter()
            .zip(keep)
            .filter_map(|(d, k)| k.then_some(d))
            .collect()
    }

    /// Convert detections to `TableElement`
    ///
    /// This reconstructs the table cell grid from detected rows and columns.
    ///
    /// # Arguments
    /// * `detections` - Filtered detections from `predict()`
    /// * `table_bbox` - Original table bounding box in page coordinates
    /// * `cluster_id` - Table cluster ID
    /// * `ocr_cells` - OCR text cells for text matching
    #[must_use = "table element is created but not used"]
    #[allow(clippy::too_many_lines)]
    pub fn detections_to_table_element(
        &self,
        detections: &[Detection],
        table_bbox: &BoundingBox,
        cluster_id: usize,
        ocr_cells: &[crate::pipeline::SimpleTextCell],
    ) -> TableElement {
        // Separate rows and columns
        let rows: Vec<_> = detections
            .iter()
            .filter(|d| d.label == TableLabel::TableRow)
            .collect();
        let columns: Vec<_> = detections
            .iter()
            .filter(|d| d.label == TableLabel::TableColumn)
            .collect();
        let header_count = detections
            .iter()
            .filter(|d| d.label == TableLabel::TableColumnHeader)
            .count();

        log::debug!(
            "[ONNX Table] Raw detected {} rows, {} columns, {} headers",
            rows.len(),
            columns.len(),
            header_count
        );

        // Convert detections to boundaries
        // Rows: (y_top, y_bottom) - note that rows span the full width, we use y coords
        let row_boundaries: Vec<_> = rows
            .iter()
            .map(|d| (d.bbox[1] - d.bbox[3] / 2.0, d.bbox[1] + d.bbox[3] / 2.0))
            .collect();

        // Columns: (x_left, x_right) - note columns span full height, we use x coords
        let col_boundaries: Vec<_> = columns
            .iter()
            .map(|d| (d.bbox[0] - d.bbox[2] / 2.0, d.bbox[0] + d.bbox[2] / 2.0))
            .collect();

        // Merge overlapping intervals using proper 1D NMS
        // This handles cases where same row/column is detected multiple times
        let merged_rows = merge_intervals_nms(&row_boundaries, ROW_INTERVAL_MERGE_IOU_THRESHOLD);
        let merged_cols = merge_intervals_nms(&col_boundaries, COL_INTERVAL_MERGE_IOU_THRESHOLD);

        log::debug!(
            "[ONNX Table] After NMS: {} rows, {} cols",
            merged_rows.len(),
            merged_cols.len()
        );

        // Sort merged rows by y coordinate (top to bottom)
        let mut sorted_rows = merged_rows;
        sorted_rows.sort_by(|a, b| a.0.total_cmp(&b.0));

        // Sort merged columns by x coordinate (left to right)
        let mut sorted_cols = merged_cols;
        sorted_cols.sort_by(|a, b| a.0.total_cmp(&b.0));

        let table_w = table_bbox.r - table_bbox.l;
        let table_h = table_bbox.b - table_bbox.t;

        // If any OCR cells inside the table bbox are not covered by detected rows,
        // infer additional rows from OCR vertical positions (helps recover missed
        // bottom rows).
        if table_w > 0.0 && table_h > 0.0 && !sorted_rows.is_empty() {
            let mut table_ocr_cells: Vec<&crate::pipeline::SimpleTextCell> = Vec::new();
            for cell in ocr_cells {
                let bbox = cell.bbox();
                let center_x = (bbox.l + bbox.r) / 2.0;
                let center_y = (bbox.t + bbox.b) / 2.0;
                if center_x >= table_bbox.l
                    && center_x <= table_bbox.r
                    && center_y >= table_bbox.t
                    && center_y <= table_bbox.b
                {
                    table_ocr_cells.push(cell);
                }
            }

            let uncovered: Vec<&crate::pipeline::SimpleTextCell> = table_ocr_cells
                .iter()
                .copied()
                .filter(|cell| {
                    let bbox = cell.bbox();
                    let center_y = (bbox.t + bbox.b) / 2.0;
                    let y = (center_y - table_bbox.t) / table_h;
                    !sorted_rows.iter().any(|(t, b)| y >= *t && y <= *b)
                })
                .collect();

            if !uncovered.is_empty() {
                let inferred = infer_rows_from_ocr(&uncovered, table_bbox);
                if !inferred.is_empty() {
                    sorted_rows.extend(inferred);
                    sorted_rows.sort_by(|a, b| a.0.total_cmp(&b.0));
                }
            }
        }

        let num_rows = sorted_rows.len().max(1);
        let num_cols = sorted_cols.len().max(1);

        log::debug!("    After merging: {num_rows} rows, {num_cols} columns");

        // Build cell grid from row/column intersections
        let mut table_cells = Vec::new();

        for (row_idx, row) in sorted_rows.iter().enumerate() {
            for (col_idx, col) in sorted_cols.iter().enumerate() {
                // Cell bbox is intersection of row and column
                let cell_bbox = BoundingBox {
                    l: col.0.mul_add(table_w, table_bbox.l),
                    t: row.0.mul_add(table_h, table_bbox.t),
                    r: col.1.mul_add(table_w, table_bbox.l),
                    b: row.1.mul_add(table_h, table_bbox.t),
                    coord_origin: CoordOrigin::TopLeft,
                };

                // Find matching OCR text (F72: returns OCR metadata)
                let ocr_match = find_matching_ocr_text(&cell_bbox, ocr_cells);

                table_cells.push(TableCell {
                    text: ocr_match.text,
                    bbox: cell_bbox,
                    row_span: 1,
                    col_span: 1,
                    start_row_offset_idx: row_idx,
                    end_row_offset_idx: row_idx + 1,
                    start_col_offset_idx: col_idx,
                    end_col_offset_idx: col_idx + 1,
                    // ONNX table structure doesn't detect header cells
                    // Would require: Model upgrade or heuristic (first row detection)
                    column_header: false,
                    row_header: false,
                    // F72: Include from_ocr and confidence from OCR match
                    from_ocr: ocr_match.from_ocr,
                    confidence: ocr_match.confidence,
                });
            }
        }

        // Generate OTSL sequence (for compatibility)
        let mut otsl_seq = Vec::new();
        for row_idx in 0..num_rows {
            for col_idx in 0..num_cols {
                let cell_idx = row_idx * num_cols + col_idx;
                if cell_idx < table_cells.len() && !table_cells[cell_idx].text.is_empty() {
                    otsl_seq.push("fcel".to_string()); // filled cell
                } else {
                    otsl_seq.push("ecel".to_string()); // empty cell
                }
            }
            otsl_seq.push("nl".to_string()); // newline
        }

        TableElement {
            label: DocItemLabel::Table,
            id: cluster_id,
            page_no: 0,
            text: None,
            cluster: Cluster {
                id: cluster_id,
                label: DocItemLabel::Table,
                bbox: *table_bbox,
                confidence: 1.0,
                cells: vec![],
                children: vec![],
            },
            otsl_seq,
            num_rows,
            num_cols,
            table_cells,
            captions: Vec::new(),
            footnotes: Vec::new(),
        }
    }
}

/// Merge overlapping intervals using 1D NMS (Non-Maximum Suppression)
///
/// This is like NMS for bounding boxes but for 1D intervals.
/// Intervals with `IoU` > threshold are merged together.
///
/// # Arguments
/// * `intervals` - List of (start, end) intervals
/// * `iou_threshold` - Intervals with `IoU` above this are merged
fn merge_intervals_nms(intervals: &[(f32, f32)], iou_threshold: f32) -> Vec<(f32, f32)> {
    if intervals.is_empty() {
        return vec![];
    }

    // Sort intervals by size (larger first - we prefer larger detections)
    let mut indexed: Vec<_> = intervals.iter().enumerate().collect();
    indexed.sort_by(|a, b| {
        let size_a = a.1 .1 - a.1 .0;
        let size_b = b.1 .1 - b.1 .0;
        size_b.total_cmp(&size_a)
    });

    let mut keep = vec![true; intervals.len()];
    let mut result = Vec::new();

    for i in 0..indexed.len() {
        let idx_i = indexed[i].0;
        if !keep[idx_i] {
            continue;
        }

        let int_i = intervals[idx_i];
        result.push(int_i);

        // Suppress overlapping intervals
        for (idx_j, _) in indexed.iter().skip(i + 1) {
            if !keep[*idx_j] {
                continue;
            }

            let int_j = intervals[*idx_j];
            let iou = compute_interval_iou(int_i, int_j);
            if iou > iou_threshold {
                keep[*idx_j] = false;
            }
        }
    }

    result
}

fn infer_rows_from_ocr(
    ocr_cells: &[&crate::pipeline::SimpleTextCell],
    table_bbox: &BoundingBox,
) -> Vec<(f32, f32)> {
    if ocr_cells.is_empty() {
        return vec![];
    }

    let table_h = table_bbox.b - table_bbox.t;
    if table_h <= 0.0 {
        return vec![];
    }

    // Sort cells by vertical center (top to bottom), normalized to table height.
    let mut cells: Vec<(f32, f32, f32)> = ocr_cells
        .iter()
        .map(|cell| {
            let bbox = cell.bbox();
            let center_y = (bbox.t + bbox.b) / 2.0;
            let y_center = (center_y - table_bbox.t) / table_h;
            let y_top = (bbox.t - table_bbox.t) / table_h;
            let y_bottom = (bbox.b - table_bbox.t) / table_h;
            (y_center, y_top, y_bottom)
        })
        .collect();
    cells.sort_by(|a, b| a.0.total_cmp(&b.0));

    // Grouping threshold based on median cell height.
    let mut heights: Vec<f32> = cells.iter().map(|(_, t, b)| (b - t).abs()).collect();
    heights.sort_by(f32::total_cmp);
    let median_h = heights.get(heights.len() / 2).copied().unwrap_or(0.02);
    let group_thresh = (median_h * 1.5).max(0.02);

    let mut intervals: Vec<(f32, f32)> = Vec::new();
    let mut current_top = cells[0].1;
    let mut current_bottom = cells[0].2;
    let mut last_center = cells[0].0;

    for (y_center, y_top, y_bottom) in cells.into_iter().skip(1) {
        if (y_center - last_center) > group_thresh {
            intervals.push((current_top.clamp(0.0, 1.0), current_bottom.clamp(0.0, 1.0)));
            current_top = y_top;
            current_bottom = y_bottom;
        } else {
            current_top = current_top.min(y_top);
            current_bottom = current_bottom.max(y_bottom);
        }
        last_center = y_center;
    }

    intervals.push((current_top.clamp(0.0, 1.0), current_bottom.clamp(0.0, 1.0)));
    intervals
}

/// Infer column boundaries from OCR cell positions
///
/// This is a fallback when the ONNX model doesn't produce reliable column detections.
/// It clusters OCR cells by their x-center position and creates column boundaries.
fn infer_columns_from_ocr(
    ocr_cells: &[crate::pipeline::SimpleTextCell],
    table_bbox: &BoundingBox,
) -> Vec<(f32, f32)> {
    if ocr_cells.is_empty() {
        return vec![];
    }

    let table_w = table_bbox.r - table_bbox.l;
    if table_w <= 0.0 {
        return vec![];
    }

    // Get x-centers of all OCR cells (normalized to table width)
    let mut x_centers: Vec<f32> = ocr_cells
        .iter()
        .map(|c| {
            let bbox = c.bbox();
            let center = (bbox.l + bbox.r) / 2.0;
            (center - table_bbox.l) / table_w
        })
        .collect();

    x_centers.sort_by(f32::total_cmp);

    // Find gaps between OCR cells to identify column breaks
    // Use a 3% threshold - balance between too few and too many columns
    let gap_threshold = 0.03;
    let mut col_breaks = vec![0.0]; // Start of table

    for i in 1..x_centers.len() {
        let gap = x_centers[i] - x_centers[i - 1];
        if gap > gap_threshold {
            // Found a gap - create column break at midpoint
            col_breaks.push((x_centers[i - 1] + x_centers[i]) / 2.0);
        }
    }
    col_breaks.push(1.0); // End of table

    // Convert breaks to column intervals
    let mut columns = Vec::new();
    for i in 0..col_breaks.len() - 1 {
        columns.push((col_breaks[i], col_breaks[i + 1]));
    }

    columns
}

/// Compute `IoU` (Intersection over Union) for 1D intervals
fn compute_interval_iou(a: (f32, f32), b: (f32, f32)) -> f32 {
    let inter_start = a.0.max(b.0);
    let inter_end = a.1.min(b.1);
    let intersection = (inter_end - inter_start).max(0.0);

    let len_a = a.1 - a.0;
    let len_b = b.1 - b.0;
    let union = len_a + len_b - intersection;

    if union > 0.0 {
        intersection / union
    } else {
        0.0
    }
}

/// Compute `IoU` (Intersection over Union) between two boxes
/// Boxes are in (cx, cy, w, h) format, normalized [0, 1]
fn compute_iou(box1: &[f32; 4], box2: &[f32; 4]) -> f32 {
    // Convert to (x1, y1, x2, y2)
    let (x1_1, y1_1, x2_1, y2_1) = (
        box1[0] - box1[2] / 2.0,
        box1[1] - box1[3] / 2.0,
        box1[0] + box1[2] / 2.0,
        box1[1] + box1[3] / 2.0,
    );
    let (x1_2, y1_2, x2_2, y2_2) = (
        box2[0] - box2[2] / 2.0,
        box2[1] - box2[3] / 2.0,
        box2[0] + box2[2] / 2.0,
        box2[1] + box2[3] / 2.0,
    );

    // Intersection
    let x1_i = x1_1.max(x1_2);
    let y1_i = y1_1.max(y1_2);
    let x2_i = x2_1.min(x2_2);
    let y2_i = y2_1.min(y2_2);

    let inter_w = (x2_i - x1_i).max(0.0);
    let inter_h = (y2_i - y1_i).max(0.0);
    let intersection = inter_w * inter_h;

    // Union
    let area1 = box1[2] * box1[3];
    let area2 = box2[2] * box2[3];
    let union = area1 + area2 - intersection;

    if union > 0.0 {
        intersection / union
    } else {
        0.0
    }
}

/// Find OCR cells that overlap with a table cell bbox
/// Uses overlap ratio to assign text only to the cell with best overlap
///
/// F72: Now returns OCR metadata (`from_ocr`, confidence) along with text.
fn find_matching_ocr_text(
    cell_bbox: &BoundingBox,
    ocr_cells: &[crate::pipeline::SimpleTextCell],
) -> OcrTextMatch {
    let mut matching: Vec<&crate::pipeline::SimpleTextCell> = Vec::new();

    for ocr_cell in ocr_cells {
        let ocr_bbox = ocr_cell.bbox();

        // Calculate overlap ratio - what fraction of OCR cell center is in this table cell
        let ocr_center_x = (ocr_bbox.l + ocr_bbox.r) / 2.0;
        let ocr_center_y = (ocr_bbox.t + ocr_bbox.b) / 2.0;

        // Check if the center of the OCR cell is within the table cell
        if ocr_center_x >= cell_bbox.l
            && ocr_center_x <= cell_bbox.r
            && ocr_center_y >= cell_bbox.t
            && ocr_center_y <= cell_bbox.b
        {
            matching.push(ocr_cell);
        }
    }

    // F72: Calculate OCR metadata
    let from_ocr = matching.iter().any(|cell| cell.from_ocr);

    // Calculate average confidence from OCR cells
    // SimpleTextCell.confidence is f32 (not Option), so we always have values
    let confidence = if matching.is_empty() {
        None
    } else {
        let total: f32 = matching.iter().map(|cell| cell.confidence).sum();
        Some(total / matching.len() as f32)
    };

    let matching_texts: Vec<&str> = matching.iter().map(|cell| cell.text.as_str()).collect();

    OcrTextMatch {
        text: matching_texts.join(" "),
        from_ocr,
        confidence,
    }
}

/// Compute what fraction of `bbox_a` is inside `bbox_b`
fn compute_bbox_overlap_ratio(bbox_a: &BoundingBox, bbox_b: &BoundingBox) -> f32 {
    // Calculate intersection
    let inter_l = bbox_a.l.max(bbox_b.l);
    let inter_t = bbox_a.t.max(bbox_b.t);
    let inter_r = bbox_a.r.min(bbox_b.r);
    let inter_b = bbox_a.b.min(bbox_b.b);

    let inter_w = (inter_r - inter_l).max(0.0);
    let inter_h = (inter_b - inter_t).max(0.0);
    let intersection = inter_w * inter_h;

    // Area of bbox_a
    let area_a = (bbox_a.r - bbox_a.l) * (bbox_a.b - bbox_a.t);

    if area_a > 0.0 {
        intersection / area_a
    } else {
        0.0
    }
}

/// Check if two bounding boxes overlap
fn bboxes_overlap(bbox1: &BoundingBox, bbox2: &BoundingBox) -> bool {
    !(bbox1.r < bbox2.l || bbox1.l > bbox2.r || bbox1.b < bbox2.t || bbox1.t > bbox2.b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_iou_same_box() {
        let box1 = [0.5, 0.5, 0.2, 0.2];
        let iou = compute_iou(&box1, &box1);
        assert!((iou - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_compute_iou_no_overlap() {
        let box1 = [0.2, 0.2, 0.2, 0.2];
        let box2 = [0.8, 0.8, 0.2, 0.2];
        let iou = compute_iou(&box1, &box2);
        assert!(iou < 1e-5);
    }

    #[test]
    fn test_compute_iou_partial_overlap() {
        let box1 = [0.5, 0.5, 0.4, 0.4];
        let box2 = [0.6, 0.5, 0.4, 0.4];
        let iou = compute_iou(&box1, &box2);
        assert!(iou > 0.0 && iou < 1.0);
    }

    #[test]
    fn test_table_label_from_usize() {
        assert_eq!(TableLabel::try_from(0), Ok(TableLabel::Table));
        assert_eq!(TableLabel::try_from(1), Ok(TableLabel::TableColumn));
        assert_eq!(TableLabel::try_from(2), Ok(TableLabel::TableRow));
        assert_eq!(TableLabel::try_from(7), Err(()));
    }

    #[test]
    fn test_table_label_display() {
        assert_eq!(TableLabel::Table.to_string(), "table");
        assert_eq!(TableLabel::TableColumn.to_string(), "table column");
        assert_eq!(TableLabel::TableRow.to_string(), "table row");
        assert_eq!(
            TableLabel::TableColumnHeader.to_string(),
            "table column header"
        );
        assert_eq!(
            TableLabel::TableProjectedRowHeader.to_string(),
            "table projected row header"
        );
        assert_eq!(
            TableLabel::TableSpanningCell.to_string(),
            "table spanning cell"
        );
        assert_eq!(TableLabel::NoObject.to_string(), "no object");
    }

    #[test]
    fn test_table_label_from_str() {
        // Exact matches
        assert_eq!("table".parse::<TableLabel>().unwrap(), TableLabel::Table);
        assert_eq!(
            "table column".parse::<TableLabel>().unwrap(),
            TableLabel::TableColumn
        );
        assert_eq!(
            "table row".parse::<TableLabel>().unwrap(),
            TableLabel::TableRow
        );
        assert_eq!(
            "table column header".parse::<TableLabel>().unwrap(),
            TableLabel::TableColumnHeader
        );
        assert_eq!(
            "table projected row header".parse::<TableLabel>().unwrap(),
            TableLabel::TableProjectedRowHeader
        );
        assert_eq!(
            "table spanning cell".parse::<TableLabel>().unwrap(),
            TableLabel::TableSpanningCell
        );
        assert_eq!(
            "no object".parse::<TableLabel>().unwrap(),
            TableLabel::NoObject
        );

        // Short aliases
        assert_eq!(
            "column".parse::<TableLabel>().unwrap(),
            TableLabel::TableColumn
        );
        assert_eq!("row".parse::<TableLabel>().unwrap(), TableLabel::TableRow);
        assert_eq!(
            "header".parse::<TableLabel>().unwrap(),
            TableLabel::TableColumnHeader
        );
        assert_eq!(
            "row header".parse::<TableLabel>().unwrap(),
            TableLabel::TableProjectedRowHeader
        );
        assert_eq!(
            "span".parse::<TableLabel>().unwrap(),
            TableLabel::TableSpanningCell
        );
        assert_eq!("none".parse::<TableLabel>().unwrap(), TableLabel::NoObject);
        assert_eq!(
            "background".parse::<TableLabel>().unwrap(),
            TableLabel::NoObject
        );

        // Case insensitive
        assert_eq!("TABLE".parse::<TableLabel>().unwrap(), TableLabel::Table);
        assert_eq!(
            "Table Column".parse::<TableLabel>().unwrap(),
            TableLabel::TableColumn
        );

        // Underscore/hyphen variants
        assert_eq!(
            "table_column".parse::<TableLabel>().unwrap(),
            TableLabel::TableColumn
        );
        assert_eq!(
            "table-row".parse::<TableLabel>().unwrap(),
            TableLabel::TableRow
        );

        // Invalid
        assert!("invalid".parse::<TableLabel>().is_err());
    }

    #[test]
    fn test_table_label_roundtrip() {
        let labels = [
            TableLabel::Table,
            TableLabel::TableColumn,
            TableLabel::TableRow,
            TableLabel::TableColumnHeader,
            TableLabel::TableProjectedRowHeader,
            TableLabel::TableSpanningCell,
            TableLabel::NoObject,
        ];
        for label in labels {
            let s = label.to_string();
            let parsed: TableLabel = s.parse().unwrap();
            assert_eq!(parsed, label, "Roundtrip failed for {label:?}");
        }
    }
}
