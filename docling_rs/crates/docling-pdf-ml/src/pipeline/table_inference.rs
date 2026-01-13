use crate::models::table_structure::TableStructureModel;
use crate::pipeline::{
    BoundingBox, Cluster, CoordOrigin, DocItemLabel, SimpleTextCell, TableElement,
    TableStructurePrediction,
};
use crate::preprocessing::tableformer::TABLEFORMER_INPUT_SIZE;
use ndarray::{s, Array3, ArrayView3};
use std::collections::HashMap;
use tch::Tensor;

/// OCR metadata for table cell text (F72: Table-OCR linkage)
///
/// Contains information about whether text came from OCR and the associated confidence scores.
struct OcrTextMatch {
    /// Concatenated text from matching OCR cells
    text: String,
    /// Whether any text came from OCR (true if any matching cell had from_ocr=true)
    from_ocr: bool,
    /// Average confidence of matching OCR cells (None if no OCR cells matched)
    confidence: Option<f32>,
}

/// Default inference scale factor for TableFormer (matches Python: 2.0 for 144 DPI)
/// This constant is kept for reference; actual value should come from PipelineConfig
pub const DEFAULT_TABLE_SCALE: f32 = 2.0;

/// Issue #8 FIX: Minimum cell size in points (filters out noise/artifacts)
/// Cells smaller than 1x1 point are likely detection noise
/// NOTE: This default value is kept for reference. The actual value is passed via
/// PipelineConfig.min_cell_size_points and can be configured per-pipeline.
#[allow(dead_code)]
const MIN_CELL_SIZE_POINTS_DEFAULT: f32 = 1.0;

/// Issue #10 FIX: Minimum confidence threshold for cell detection
/// Cells with confidence below this threshold are filtered out
/// 0.5 = 50% confidence threshold (conservative, keeps most cells)
/// NOTE: This default value is kept for reference. The actual value is passed via
/// PipelineConfig.min_cell_confidence and can be configured per-pipeline.
#[allow(dead_code)]
const MIN_CELL_CONFIDENCE_DEFAULT: f32 = 0.5;

/// TableFormer normalization mean values (from tm_config.json)
///
/// These are per-channel (RGB) mean values used for image normalization
/// before TableFormer inference. Computed as: (pixel - 255*mean) / std
/// Values from Python docling's TableFormer model configuration.
const TABLEFORMER_NORM_MEAN: [f32; 3] = [0.942_478_5, 0.942_546_7, 0.942_926_1];

/// TableFormer normalization std values (from tm_config.json)
///
/// These are per-channel (RGB) standard deviation values used for
/// image normalization before TableFormer inference.
const TABLEFORMER_NORM_STD: [f32; 3] = [0.179_109_56, 0.179_404_03, 0.179_316_63];

/// Run TableFormer inference on table clusters
///
/// # Arguments
/// * `model` - TableFormer model
/// * `page_image` - Full page image (HWC, f32, [0-255])
/// * `page_width` - Page width in points
/// * `page_height` - Page height in points
/// * `table_clusters` - Table clusters from layout detection
/// * `ocr_cells` - OCR text cells for text matching
/// * `page_no` - Page number (0-indexed)
/// * `table_scale` - Scale factor for table inference (default 2.0 for 144 DPI)
/// * `min_cell_size_points` - Minimum cell size in points (filters noise)
/// * `min_cell_confidence` - Minimum confidence threshold for cell detection
///
/// # Returns
/// TableStructurePrediction with table_map populated
#[allow(clippy::too_many_arguments)]
pub fn run_table_inference(
    model: &TableStructureModel,
    page_image: &ArrayView3<f32>,
    page_width: f32,
    page_height: f32,
    table_clusters: &[Cluster],
    ocr_cells: &[SimpleTextCell],
    page_no: usize,
    table_scale: f32,
    min_cell_size_points: f32,
    min_cell_confidence: f32,
) -> Result<TableStructurePrediction, Box<dyn std::error::Error>> {
    let mut table_map: HashMap<usize, TableElement> = HashMap::new();

    // Issue #8 FIX: Warn if OCR cells are empty (all table cells will have empty text)
    if ocr_cells.is_empty() && !table_clusters.is_empty() {
        log::warn!(
            "Page {}: No OCR cells provided for {} table(s) - table cells will have empty text",
            page_no,
            table_clusters.len()
        );
    }

    for cluster in table_clusters {
        log::debug!(
            "    Processing table cluster ID={} at bbox={:?}",
            cluster.id,
            cluster.bbox
        );

        // Step 1: Crop table region from page image
        let table_bbox_scaled = scale_bbox(&cluster.bbox, table_scale);
        log::debug!(
            "      Scaled bbox: l={:.1}, t={:.1}, r={:.1}, b={:.1}",
            table_bbox_scaled.l,
            table_bbox_scaled.t,
            table_bbox_scaled.r,
            table_bbox_scaled.b
        );
        let cropped = crop_table_region(
            page_image,
            &table_bbox_scaled,
            page_width * table_scale,
            page_height * table_scale,
        )?;
        log::debug!("      Cropped table region: {:?}", cropped.dim());

        // Step 2: Preprocess to 448x448 for TableFormer
        log::debug!("      Preprocessing to 448x448...");
        let preprocessed = preprocess_for_tableformer(&cropped)?;
        log::debug!("      Preprocessed: {:?}", preprocessed.size());

        // Step 3: Run TableFormer inference
        let (tag_sequence, class_logits, coordinates) = model.predict(&preprocessed);

        // Step 4: Parse output to TableElement
        let table_element = parse_tableformer_output(
            tag_sequence,
            class_logits,
            coordinates,
            &cluster.bbox,
            table_scale,
            cluster.id,
            ocr_cells,
            page_no,
            min_cell_size_points,
            min_cell_confidence,
        )?;

        log::debug!(
            "      ✓ Table: {} rows, {} cols, {} cells",
            table_element.num_rows,
            table_element.num_cols,
            table_element.table_cells.len()
        );

        table_map.insert(cluster.id, table_element);
    }

    Ok(TableStructurePrediction { table_map })
}

/// Scale a bounding box by a factor
fn scale_bbox(bbox: &BoundingBox, scale: f32) -> BoundingBox {
    BoundingBox {
        l: (bbox.l * scale).round(),
        t: (bbox.t * scale).round(),
        r: (bbox.r * scale).round(),
        b: (bbox.b * scale).round(),
        coord_origin: bbox.coord_origin,
    }
}

/// Crop table region from page image
///
/// Python reference: docling/models/table_structure_model.py:208-212
/// - Page image is scaled by 2.0 (144 DPI)
/// - Table bbox is rounded and scaled
/// - Region is cropped from scaled page image
///
/// # Arguments
/// * `page_image` - Full page image (HWC, f32, [0-255])
/// * `table_bbox` - Table bounding box (already scaled)
/// * `page_width_scaled` - Scaled page width
/// * `page_height_scaled` - Scaled page height
///
/// # Returns
/// Cropped table region (HWC, f32, [0-255])
fn crop_table_region(
    page_image: &ArrayView3<f32>,
    table_bbox: &BoundingBox,
    page_width_scaled: f32,
    page_height_scaled: f32,
) -> Result<Array3<f32>, Box<dyn std::error::Error>> {
    let (img_h, img_w, _channels) = page_image.dim();

    // Convert bbox coordinates (points) to pixel coordinates
    // Image coordinates: (0,0) is top-left
    // Bbox uses TopLeft origin, so direct mapping
    let scale_x = img_w as f32 / page_width_scaled;
    let scale_y = img_h as f32 / page_height_scaled;

    let x0 = (table_bbox.l * scale_x).round().max(0.0) as usize;
    let y0 = (table_bbox.t * scale_y).round().max(0.0) as usize;
    let x1 = (table_bbox.r * scale_x).round().max(0.0).min(img_w as f32) as usize;
    let y1 = (table_bbox.b * scale_y).round().max(0.0).min(img_h as f32) as usize;

    // CRITICAL FIX (Issue T9): Guard against inverted or zero-sized bbox
    // This can happen with malformed PDF bbox data
    let x0 = x0.min(img_w.saturating_sub(1));
    let y0 = y0.min(img_h.saturating_sub(1));
    let x1 = x1.max(x0 + 1).min(img_w); // Ensure at least 1 pixel width
    let y1 = y1.max(y0 + 1).min(img_h); // Ensure at least 1 pixel height

    // Crop region
    let cropped = page_image.slice(s![y0..y1, x0..x1, ..]).to_owned();

    Ok(cropped)
}

/// Preprocess cropped table image for TableFormer
///
/// Python reference:
/// - docling/models/table_structure_model.py lines 208-212 + TFPredictor._prepare_image
/// - docling_ibm_models/tableformer/data_management/functional.py:38-53 (normalize)
/// - docling_ibm_models/tableformer/data_management/functional.py:58-97 (resize)
///
/// Steps:
/// 1. Normalize: (pixel - 255*mean) / std  [CRITICAL: Mean is multiplied by 255!]
/// 2. Resize to 448x448 using bilinear interpolation
/// 3. Transpose: [H, W, C] → [C, W, H]  [CRITICAL: W and H are swapped!]
/// 4. Convert to tensor and divide by 255
///
/// # Arguments
/// * `cropped` - Cropped table region (HWC, f32, [0-255])
///
/// # Returns
/// Preprocessed tensor [1, 3, 448, 448] (NCHW, f32, normalized)
fn preprocess_for_tableformer(cropped: &Array3<f32>) -> Result<Tensor, Box<dyn std::error::Error>> {
    let (h, w, c) = cropped.dim();
    log::debug!("        Input dimensions: H={}, W={}, C={}", h, w, c);

    // Step 1: Normalize using TableFormer's mean/std
    // Python: (img - 255*mean) / std
    // Values from TABLEFORMER_NORM_MEAN and TABLEFORMER_NORM_STD constants
    let mean = TABLEFORMER_NORM_MEAN;
    let std = TABLEFORMER_NORM_STD;

    let mut normalized = cropped.clone();
    for c_idx in 0..3 {
        let mut channel = normalized.slice_mut(s![.., .., c_idx]);
        let mean_val = 255.0 * mean[c_idx];
        let std_val = std[c_idx];
        channel.mapv_inplace(|pixel| (pixel - mean_val) / std_val);
    }
    log::debug!(
        "        After normalize: min={:.2}, max={:.2}",
        normalized.iter().copied().fold(f32::INFINITY, f32::min),
        normalized.iter().copied().fold(f32::NEG_INFINITY, f32::max)
    );

    // Step 2: Resize to 448x448 using PyTorch's bilinear interpolation
    // (matches Python's OpenCV cv2.resize with INTER_LINEAR)
    // Convert ndarray → Tensor for resize operation
    let normalized_flat: Vec<f32> = normalized.iter().copied().collect();
    let normalized_tensor =
        Tensor::f_from_slice(&normalized_flat)?.reshape([1, h as i64, w as i64, c as i64]); // [1, H, W, C]

    // Permute to NCHW: [1, C, H, W] for interpolation
    let normalized_nchw = normalized_tensor.permute([0, 3, 1, 2]);

    // Resize to 448x448 using bilinear interpolation
    let target_size = TABLEFORMER_INPUT_SIZE as i64;
    let resized = normalized_nchw.upsample_bilinear2d(
        [target_size, target_size],
        false, // align_corners
        None,  // scales_h
        None,  // scales_w
    );
    log::debug!("        After resize: {}x{}", target_size, target_size);

    // Step 3: Python's transpose(2,1,0) AFTER resize
    // Python: img.transpose(2, 1, 0) converts [H, W, C] → [C, W, H]
    // This SWAPS width and height! (NOT standard PyTorch format)
    // Current shape: [1, C, H, W]
    // Target shape: [1, C, W, H]
    // Permute: [0, 1, 3, 2] swaps the last two dimensions
    let transposed = resized.permute([0, 1, 3, 2]); // [1, C, H, W] → [1, C, W, H]
    log::debug!("        After transpose(W/H swap): {:?}", transposed.size());

    // Step 4: Divide by 255
    let normalized_final = transposed / 255.0;
    log::debug!("        Final shape: {:?}", normalized_final.size());

    // Get min/max for debugging
    let min_val = normalized_final.min().double_value(&[]);
    let max_val = normalized_final.max().double_value(&[]);
    log::debug!("        Final range: [{:.6}, {:.6}]", min_val, max_val);

    Ok(normalized_final)
}

/// Find OCR cells that are contained within a table cell bbox
///
/// Python reference: docling_ibm_models/tableformer/data_management/tf_predictor.py:812-841
/// Python uses cell_matcher.match_cells() which does bbox intersection matching
///
/// CRITICAL FIX (Issue 7): Sort OCR cells by position (top-to-bottom, left-to-right)
/// before concatenating text, ensuring consistent and correct reading order.
///
/// F72: Now returns OCR metadata (from_ocr, confidence) along with text.
///
/// # Arguments
/// * `table_cell_bbox` - Table cell bounding box
/// * `ocr_cells` - OCR text cells to search
///
/// # Returns
/// OcrTextMatch containing concatenated text, from_ocr flag, and average confidence
fn find_matching_ocr_text(
    table_cell_bbox: &BoundingBox,
    ocr_cells: &[SimpleTextCell],
) -> OcrTextMatch {
    // Collect matching OCR cells with their bboxes for sorting
    let mut matching: Vec<(&SimpleTextCell, &BoundingBox)> = Vec::new();

    for ocr_cell in ocr_cells {
        let ocr_bbox = ocr_cell.bbox();

        // Check if OCR cell overlaps with table cell
        // Use intersection-based matching (Python: cell_matcher uses IoU/intersection)
        if bboxes_overlap(table_cell_bbox, ocr_bbox) {
            matching.push((ocr_cell, ocr_bbox));
        }
    }

    // Sort by reading order: top-to-bottom, then left-to-right
    // Python: cell_matcher sorts by (row, col) position
    // We use bbox coordinates: primary sort by top (t), secondary by left (l)
    matching.sort_by(|a, b| {
        let t_cmp =
            a.1.t
                .partial_cmp(&b.1.t)
                .unwrap_or(std::cmp::Ordering::Equal);
        if t_cmp == std::cmp::Ordering::Equal {
            a.1.l
                .partial_cmp(&b.1.l)
                .unwrap_or(std::cmp::Ordering::Equal)
        } else {
            t_cmp
        }
    });

    // Collect texts in sorted order
    let matching_texts: Vec<&str> = matching
        .iter()
        .map(|(cell, _)| cell.text.as_str())
        .collect();

    // F72: Calculate OCR metadata
    let from_ocr = matching.iter().any(|(cell, _)| cell.from_ocr);

    // Calculate average confidence from OCR cells
    // SimpleTextCell.confidence is f32 (not Option), so we always have values
    let confidence = if matching.is_empty() {
        None
    } else {
        let total: f32 = matching.iter().map(|(cell, _)| cell.confidence).sum();
        Some(total / matching.len() as f32)
    };

    // Join texts with space
    OcrTextMatch {
        text: matching_texts.join(" "),
        from_ocr,
        confidence,
    }
}

/// Check if two bounding boxes have strict overlap (not just touching edges)
///
/// Issue #7 FIX: Use strict overlap (not touching edges) to prevent adjacent
/// OCR boxes from bleeding text into neighboring cells. Previously, boxes that
/// merely touched at their edges (r==l or b==t) were considered overlapping,
/// which caused text to incorrectly appear in multiple cells.
///
/// # Arguments
/// * `bbox1` - First bounding box
/// * `bbox2` - Second bounding box
///
/// # Returns
/// true if bboxes have actual overlap area (> 0), not just touching edges
fn bboxes_overlap(bbox1: &BoundingBox, bbox2: &BoundingBox) -> bool {
    // No overlap if one bbox is completely to the left/right/above/below the other
    // Using strict inequality: touching edges (r<=l, b<=t) do NOT count as overlap
    // Issue #7 FIX: Changed from < to <= to exclude touching edges
    !(bbox1.r <= bbox2.l || bbox1.l >= bbox2.r || bbox1.b <= bbox2.t || bbox1.t >= bbox2.b)
}

/// Calculate colspan by counting consecutive "lcel" or "xcel" tokens to the right
/// Python: docling_ibm_models/tableformer/otsl.py:182-183 (otsl_check_right)
/// Issue #9 FIX: Added bounds check to prevent panic on malformed input
fn calculate_colspan(otsl_grid: &[Vec<String>], row: usize, col: usize) -> usize {
    // Issue #9 FIX: Bounds check - return 1 if row is out of bounds
    let row_tags = match otsl_grid.get(row) {
        Some(tags) => tags,
        None => return 1,
    };

    let mut colspan = 1; // Start with 1 (the cell itself)

    // Count consecutive lcel or xcel tokens to the right
    for tag_str in row_tags.iter().skip(col + 1) {
        let tag = tag_str.as_str();
        if tag == "lcel" || tag == "xcel" {
            colspan += 1;
        } else {
            break;
        }
    }
    colspan
}

/// Calculate rowspan by counting consecutive "ucel" or "xcel" tokens below
/// Python: docling_ibm_models/tableformer/otsl.py:190-191 (otsl_check_down)
fn calculate_rowspan(otsl_grid: &[Vec<String>], row: usize, col: usize) -> usize {
    let mut rowspan = 1; // Start with 1 (the cell itself)

    // Count consecutive ucel or xcel tokens below
    for row_tags in otsl_grid.iter().skip(row + 1) {
        if col < row_tags.len() {
            let tag = row_tags[col].as_str();
            if tag == "ucel" || tag == "xcel" {
                rowspan += 1;
            } else {
                break;
            }
        } else {
            break;
        }
    }
    rowspan
}

/// Parse TableFormer output into TableElement
///
/// Python reference: docling_ibm_models/tableformer/data_management/tf_predictor.py:705-843
/// Python reference: docling/models/table_structure_model.py:250-294
///
/// # Arguments
/// * `tag_sequence` - Tag sequence from TableFormer (e.g., [start, fcel, ecel, ..., end])
/// * `class_logits` - Class predictions [N, 3]
/// * `coordinates` - Bbox coordinates [N, 4] (cx, cy, w, h) normalized [0, 1]
/// * `table_bbox` - Table cluster bbox (unscaled, in page coordinates)
/// * `scale` - Scale factor used for preprocessing
/// * `cluster_id` - Table cluster ID
/// * `ocr_cells` - OCR text cells for text matching
/// * `page_no` - Page number (0-indexed)
///
/// # Returns
/// TableElement with cells, num_rows, num_cols
#[allow(clippy::too_many_arguments)]
fn parse_tableformer_output(
    tag_sequence: Vec<i64>,
    class_logits: Tensor,
    coordinates: Tensor,
    table_bbox: &BoundingBox,
    _scale: f32, // Currently unused, kept for future coordinate scaling
    cluster_id: usize,
    ocr_cells: &[SimpleTextCell],
    page_no: usize,
    min_cell_size_points: f32,
    min_cell_confidence: f32,
) -> Result<TableElement, Box<dyn std::error::Error>> {
    // Tag vocabulary (from components.rs:1104)
    const TAG_FCEL: i64 = 5; // filled cell
    const TAG_ECEL: i64 = 4; // empty cell
    const TAG_CHED: i64 = 10; // column header
    const TAG_RHED: i64 = 11; // row header
    const TAG_SROW: i64 = 12; // special row
    const TAG_NL: i64 = 9; // newline

    // Cell tags (Python: tf_predictor.py:758-770)
    let cell_tags = [TAG_FCEL, TAG_ECEL, TAG_CHED, TAG_RHED, TAG_SROW];

    // Issue #6 FIX: Count cell tags to validate against coordinate count
    let cell_tag_count = tag_sequence
        .iter()
        .filter(|t| cell_tags.contains(t))
        .count();
    let num_coords = coordinates.size()[0] as usize;

    // Issue #6 FIX: Validate coordinate/tag count match and warn on mismatch
    if cell_tag_count != num_coords {
        log::warn!(
            "Page {}, table {}: Coordinate/tag count mismatch - {} cell tags but {} coordinates. \
             Cells may be truncated or missing coordinates.",
            page_no,
            cluster_id,
            cell_tag_count,
            num_coords
        );
    }

    log::debug!("      Parsing TableFormer output:");
    log::debug!("        Tag sequence length: {}", tag_sequence.len());
    log::debug!("        Cell tags in sequence: {}", cell_tag_count);
    log::debug!("        Num cell bboxes: {}", class_logits.size()[0]);

    // Step 1: Convert tag sequence to OTSL string sequence
    // Python: tf_predictor.py:782 - prediction["rs_seq"] = self._get_html_tags(pred_tag_seq)
    let tag_names = vec![
        "<pad>", "<unk>", "<start>", "<end>", "ecel", "fcel", "lcel", "ucel", "xcel", "nl", "ched",
        "rhed", "srow",
    ];

    log::debug!("        Tag sequence length: {}", tag_sequence.len());
    log::debug!(
        "        First 20 tags: {:?}",
        &tag_sequence[..20.min(tag_sequence.len())]
    );
    log::debug!(
        "        Last 20 tags: {:?}",
        &tag_sequence[tag_sequence.len().saturating_sub(20)..]
    );

    let otsl_seq: Vec<String> = tag_sequence
        .iter()
        .filter(|&&tag| (0..13).contains(&tag) && tag != 2 && tag != 3) // Skip <start> and <end>
        .map(|&tag| tag_names[tag as usize].to_string())
        .collect();
    log::debug!("        OTSL sequence length: {}", otsl_seq.len());

    // Step 2: Count rows and cols AND build a 2D grid for span calculation
    // Python: tf_predictor.py:570-571
    // CRITICAL FIX (Issue 1): num_rows = nl_count + 1 (rows = newlines + 1)
    // A table with 3 rows has 2 newlines between them
    let nl_count = otsl_seq.iter().filter(|s| s.as_str() == "nl").count();
    let num_rows = if nl_count > 0 || !otsl_seq.is_empty() {
        nl_count + 1 // Always add 1 for the last row (which has no trailing nl)
    } else {
        0 // Empty table
    };

    // CRITICAL FIX (Issue 2): Count max columns across ALL rows
    // Previous code only looked at first row, which breaks for variable-width tables
    let num_cols: usize = {
        let mut max_cols: usize = 0;
        let mut current_col: usize = 0;
        for tag in &otsl_seq {
            if tag.as_str() == "nl" {
                max_cols = max_cols.max(current_col);
                current_col = 0;
            } else {
                current_col += 1;
            }
        }
        // Don't forget the last row (no trailing nl)
        max_cols.max(current_col)
    };

    // Build 2D grid of OTSL tags for span calculation (ISSUE 5 FIX)
    // Python: docling_ibm_models/tableformer/otsl.py uses 2D grid to compute spans
    let mut otsl_grid: Vec<Vec<String>> = Vec::new();
    let mut current_row_tags: Vec<String> = Vec::new();
    for tag in &otsl_seq {
        if tag.as_str() == "nl" {
            otsl_grid.push(current_row_tags);
            current_row_tags = Vec::new();
        } else {
            current_row_tags.push(tag.clone());
        }
    }
    // Don't forget the last row (no trailing nl)
    if !current_row_tags.is_empty() {
        otsl_grid.push(current_row_tags);
    }

    log::debug!(
        "        Detected structure: {} rows x {} cols",
        num_rows,
        num_cols
    );

    // Step 3: Extract cell coordinates and convert to page coordinates
    // Python: tf_predictor.py:758 - bbox_pred = u.box_cxcywh_to_xyxy(outputs_coord)
    let coords_cpu = coordinates.to_device(tch::Device::Cpu);
    let num_cells = coords_cpu.size()[0] as usize;

    // Issue #10 FIX: Extract cell confidence from class_logits
    // class_logits has shape [N, 3] where 3 = number of cell classes (empty, filled, header)
    // Apply softmax to get probabilities, then take max as confidence
    let logits_cpu = class_logits.to_device(tch::Device::Cpu);
    let cell_confidences: Vec<f32> = if logits_cpu.numel() > 0 {
        let softmax_probs = logits_cpu.softmax(-1, tch::Kind::Float);
        // Get max probability for each cell as confidence
        let (max_probs, _max_indices) = softmax_probs.max_dim(-1, false);
        (0..num_cells)
            .map(|i| max_probs.double_value(&[i as i64]) as f32)
            .collect()
    } else {
        vec![1.0; num_cells] // Default to full confidence if no logits
    };

    let mut table_cells = Vec::new();
    let mut current_row = 0;
    let mut current_col = 0;
    let mut cell_idx = 0; // Index into coordinates tensor

    // Step 4: Iterate through tag sequence and build cells
    // Python: tf_predictor.py:524-567 - Process tf_responses to assign row/col indices
    for tag in &tag_sequence {
        // Check if this is a cell token
        if cell_tags.contains(tag) {
            if cell_idx >= num_cells {
                // Safety check - should not happen
                log::debug!(
                    "        WARNING: cell_idx {} exceeds num_cells {}",
                    cell_idx,
                    num_cells
                );
                break;
            }

            // Extract normalized coordinates (cx, cy, w, h) [0, 1]
            let cx = coords_cpu.double_value(&[cell_idx as i64, 0]) as f32;
            let cy = coords_cpu.double_value(&[cell_idx as i64, 1]) as f32;
            let w = coords_cpu.double_value(&[cell_idx as i64, 2]) as f32;
            let h = coords_cpu.double_value(&[cell_idx as i64, 3]) as f32;

            // Issue #9 FIX: Validate against NaN/inf in TableFormer outputs
            // Invalid coordinates can poison assembly and markdown generation
            if !cx.is_finite() || !cy.is_finite() || !w.is_finite() || !h.is_finite() {
                log::warn!(
                    "Page {}, table {}, cell {}: Invalid coordinates (NaN/inf) detected - \
                     cx={}, cy={}, w={}, h={}. Skipping cell.",
                    page_no,
                    cluster_id,
                    cell_idx,
                    cx,
                    cy,
                    w,
                    h
                );
                cell_idx += 1;
                current_col += 1;
                continue;
            }

            // Issue #10 FIX: Filter cells with low confidence
            let cell_confidence = cell_confidences.get(cell_idx).copied().unwrap_or(1.0);
            if cell_confidence < min_cell_confidence {
                log::trace!(
                    "Skipping low-confidence cell at ({},{}) with confidence {:.2} (min: {})",
                    current_row,
                    current_col,
                    cell_confidence,
                    min_cell_confidence
                );
                cell_idx += 1;
                current_col += 1;
                continue;
            }

            // Convert (cx, cy, w, h) normalized to (l, t, r, b) in table coordinates
            // Python: tf_predictor.py:758 - box_cxcywh_to_xyxy
            let l_norm = cx - w / 2.0;
            let t_norm = cy - h / 2.0;
            let r_norm = cx + w / 2.0;
            let b_norm = cy + h / 2.0;

            // Scale to table bbox (not page) - table_bbox is the crop region
            // Python: docling/models/table_structure_model.py:257-268
            let table_w = table_bbox.r - table_bbox.l;
            let table_h = table_bbox.b - table_bbox.t;

            let bbox = BoundingBox {
                l: table_bbox.l + l_norm * table_w,
                t: table_bbox.t + t_norm * table_h,
                r: table_bbox.l + r_norm * table_w,
                b: table_bbox.t + b_norm * table_h,
                coord_origin: CoordOrigin::TopLeft,
            };

            // Issue #8 FIX: Filter out tiny cell detections (likely noise/artifacts)
            let cell_width = bbox.r - bbox.l;
            let cell_height = bbox.b - bbox.t;
            if cell_width < min_cell_size_points || cell_height < min_cell_size_points {
                log::trace!(
                    "Skipping tiny cell at ({},{}) with size {:.2}x{:.2} points (min: {})",
                    current_row,
                    current_col,
                    cell_width,
                    cell_height,
                    min_cell_size_points
                );
                current_col += 1;
                continue;
            }

            // Determine cell type from tag
            // Python: docling_ibm_models/tableformer/data_management/tf_predictor.py:786-799
            // TAG_CHED = column header, TAG_RHED = row header
            let (column_header, row_header) = match *tag {
                TAG_CHED => (true, false),
                TAG_RHED => (false, true),
                _ => (false, false),
            };

            // Match OCR text cells to table cell bbox (F72: returns OCR metadata)
            let ocr_match = find_matching_ocr_text(&bbox, ocr_cells);

            // CRITICAL FIX (Issue 5): Calculate spans from OTSL grid
            // Python: docling_ibm_models/tableformer/otsl.py uses lcel/ucel/xcel tokens
            let (col_span, row_span) = if current_row < otsl_grid.len()
                && current_col < otsl_grid.get(current_row).map_or(0, |r| r.len())
            {
                let cs = calculate_colspan(&otsl_grid, current_row, current_col);
                let rs = calculate_rowspan(&otsl_grid, current_row, current_col);
                (cs, rs)
            } else {
                (1, 1)
            };

            // CRITICAL FIX (Issue T11): Bounds check on cell assignment
            // Clamp indices to valid table dimensions to prevent overflow
            let start_row = current_row.min(num_rows.saturating_sub(1));
            let end_row = (current_row + row_span).min(num_rows);
            let start_col = current_col.min(num_cols.saturating_sub(1));
            let end_col = (current_col + col_span).min(num_cols);

            // Create TableCell with proper span values and header flags
            // CRITICAL FIX (Issue T6): Emit column_header and row_header flags
            // F72: Include from_ocr and confidence from OCR match
            let cell = crate::pipeline::TableCell {
                text: ocr_match.text,
                bbox,
                row_span: end_row.saturating_sub(start_row),
                col_span: end_col.saturating_sub(start_col),
                start_row_offset_idx: start_row,
                end_row_offset_idx: end_row,
                start_col_offset_idx: start_col,
                end_col_offset_idx: end_col,
                column_header,
                row_header,
                from_ocr: ocr_match.from_ocr,
                confidence: ocr_match.confidence,
            };

            table_cells.push(cell);
            cell_idx += 1;
            current_col += 1;
        } else if *tag == TAG_NL {
            // Newline - move to next row
            current_row += 1;
            current_col = 0;
        }
    }

    log::debug!("        Created {} cells", table_cells.len());

    // Post-process: Split cells with multiple space-separated values into adjacent empty cells
    let table_cells =
        crate::pipeline::data_structures::postprocess_table_cells(table_cells, num_rows, num_cols);

    Ok(TableElement {
        label: DocItemLabel::Table,
        id: cluster_id,
        page_no, // CRITICAL FIX (Issue 3): Use actual page number instead of hardcoded 0
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
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::data_structures::TableCell;

    #[test]
    fn test_scale_bbox() {
        let bbox = BoundingBox {
            l: 100.0,
            t: 200.0,
            r: 300.0,
            b: 400.0,
            coord_origin: CoordOrigin::TopLeft,
        };

        let scaled = scale_bbox(&bbox, 2.0);

        assert_eq!(scaled.l, 200.0);
        assert_eq!(scaled.t, 400.0);
        assert_eq!(scaled.r, 600.0);
        assert_eq!(scaled.b, 800.0);
    }

    #[test]
    fn test_crop_table_region() {
        // Create 100x100 test image
        let img = Array3::<f32>::zeros((100, 100, 3));

        let bbox = BoundingBox {
            l: 10.0,
            t: 20.0,
            r: 50.0,
            b: 60.0,
            coord_origin: CoordOrigin::TopLeft,
        };

        let cropped = crop_table_region(&img.view(), &bbox, 100.0, 100.0).unwrap();

        assert_eq!(cropped.dim(), (40, 40, 3)); // 60-20=40, 50-10=40
    }

    #[test]
    fn test_calculate_colspan_simple() {
        // Simple 2x3 table with no spans:
        // fcel fcel fcel nl
        // fcel fcel fcel
        let grid = vec![
            vec!["fcel".to_string(), "fcel".to_string(), "fcel".to_string()],
            vec!["fcel".to_string(), "fcel".to_string(), "fcel".to_string()],
        ];

        // All cells should have colspan=1
        assert_eq!(calculate_colspan(&grid, 0, 0), 1);
        assert_eq!(calculate_colspan(&grid, 0, 1), 1);
        assert_eq!(calculate_colspan(&grid, 0, 2), 1);
    }

    #[test]
    fn test_calculate_colspan_with_merge() {
        // Table with colspan=2 in first row:
        // fcel lcel fcel nl
        // fcel fcel fcel
        let grid = vec![
            vec!["fcel".to_string(), "lcel".to_string(), "fcel".to_string()],
            vec!["fcel".to_string(), "fcel".to_string(), "fcel".to_string()],
        ];

        // First cell spans 2 columns
        assert_eq!(calculate_colspan(&grid, 0, 0), 2);
        // Third cell has no span
        assert_eq!(calculate_colspan(&grid, 0, 2), 1);
    }

    #[test]
    fn test_calculate_rowspan_simple() {
        // Simple 2x2 table with no spans
        let grid = vec![
            vec!["fcel".to_string(), "fcel".to_string()],
            vec!["fcel".to_string(), "fcel".to_string()],
        ];

        // All cells should have rowspan=1
        assert_eq!(calculate_rowspan(&grid, 0, 0), 1);
        assert_eq!(calculate_rowspan(&grid, 1, 0), 1);
    }

    #[test]
    fn test_calculate_rowspan_with_merge() {
        // Table with rowspan=2 in first column:
        // fcel fcel nl
        // ucel fcel
        let grid = vec![
            vec!["fcel".to_string(), "fcel".to_string()],
            vec!["ucel".to_string(), "fcel".to_string()],
        ];

        // First cell spans 2 rows
        assert_eq!(calculate_rowspan(&grid, 0, 0), 2);
        // Second column has no span
        assert_eq!(calculate_rowspan(&grid, 0, 1), 1);
    }

    #[test]
    fn test_calculate_span_with_xcel() {
        // Table with 2x2 merged cell (both colspan and rowspan):
        // fcel xcel nl
        // xcel xcel
        let grid = vec![
            vec!["fcel".to_string(), "xcel".to_string()],
            vec!["xcel".to_string(), "xcel".to_string()],
        ];

        // First cell spans 2 columns and 2 rows
        assert_eq!(calculate_colspan(&grid, 0, 0), 2);
        assert_eq!(calculate_rowspan(&grid, 0, 0), 2);
    }

    /// Helper function to create a table cell for tests
    fn make_test_cell(text: &str, row: usize, col: usize, bbox: BoundingBox) -> TableCell {
        TableCell {
            text: text.to_string(),
            row_span: 1,
            col_span: 1,
            start_row_offset_idx: row,
            start_col_offset_idx: col,
            end_row_offset_idx: row + 1,
            end_col_offset_idx: col + 1,
            column_header: false,
            row_header: false,
            from_ocr: false,
            confidence: Some(1.0),
            bbox,
        }
    }

    #[test]
    fn test_postprocess_split_merged_cells() {
        // Simulate Table 3 from Mamba paper, Row 3:
        // Col 0: "Mamba-130M", Col 1: "NeoX", Cols 2-4: empty,
        // Col 5: "10.56 16.07 44.3 35.3 64.5 48.0 24.3 51.9 44.7",
        // Cols 6-10: empty
        let default_bbox = BoundingBox {
            l: 0.0,
            t: 0.0,
            r: 10.0,
            b: 10.0,
            coord_origin: CoordOrigin::TopLeft,
        };

        let cells = vec![
            // Row 3 cells
            make_test_cell("Mamba-130M", 3, 0, default_bbox),
            make_test_cell("NeoX", 3, 1, default_bbox),
            // Empty cells 2, 3, 4 (left of merged values)
            make_test_cell("", 3, 2, default_bbox),
            make_test_cell("", 3, 3, default_bbox),
            make_test_cell("", 3, 4, default_bbox),
            // Cell with merged values at col 5
            make_test_cell(
                "10.56 16.07 44.3 35.3 64.5 48.0 24.3 51.9 44.7",
                3,
                5,
                default_bbox,
            ),
            // Empty cells 6-10 (right of merged values)
            make_test_cell("", 3, 6, default_bbox),
            make_test_cell("", 3, 7, default_bbox),
            make_test_cell("", 3, 8, default_bbox),
            make_test_cell("", 3, 9, default_bbox),
            make_test_cell("", 3, 10, default_bbox),
        ];

        let result = crate::pipeline::data_structures::postprocess_table_cells(cells, 10, 11);

        // Check that values were redistributed
        // Expected: cols 2,3,4 get first 3 values, col 5 gets 4th, cols 6-10 get rest
        let row3_cells: Vec<_> = result
            .iter()
            .filter(|c| c.start_row_offset_idx == 3)
            .collect();

        // Find cell at col 2
        let col2 = row3_cells
            .iter()
            .find(|c| c.start_col_offset_idx == 2)
            .unwrap();
        assert_eq!(col2.text, "10.56", "Col 2 should get first value");

        let col3 = row3_cells
            .iter()
            .find(|c| c.start_col_offset_idx == 3)
            .unwrap();
        assert_eq!(col3.text, "16.07", "Col 3 should get second value");

        let col4 = row3_cells
            .iter()
            .find(|c| c.start_col_offset_idx == 4)
            .unwrap();
        assert_eq!(col4.text, "44.3", "Col 4 should get third value");

        let col5 = row3_cells
            .iter()
            .find(|c| c.start_col_offset_idx == 5)
            .unwrap();
        assert_eq!(col5.text, "35.3", "Col 5 should get fourth value");

        let col6 = row3_cells
            .iter()
            .find(|c| c.start_col_offset_idx == 6)
            .unwrap();
        assert_eq!(col6.text, "64.5", "Col 6 should get fifth value");

        let col10 = row3_cells
            .iter()
            .find(|c| c.start_col_offset_idx == 10)
            .unwrap();
        assert_eq!(col10.text, "44.7", "Col 10 should get ninth value");
    }

    #[test]
    fn test_postprocess_split_text_plus_numbers() {
        // Simulate Pythia row from Table 3 Mamba paper:
        // Cell has "Pythia-160M NeoX 29.64 38.10 33.0" merged into one cell
        // Expected: Split into text tokens + numeric tokens
        let default_bbox = BoundingBox {
            l: 0.0,
            t: 0.0,
            r: 10.0,
            b: 10.0,
            coord_origin: CoordOrigin::TopLeft,
        };

        let cells = vec![
            // Cell with merged text+numbers at col 0
            make_test_cell("Pythia-160M NeoX 29.64 38.10 33.0", 5, 0, default_bbox),
            // Empty cells 1-4 (right of merged values)
            make_test_cell("", 5, 1, default_bbox),
            make_test_cell("", 5, 2, default_bbox),
            make_test_cell("", 5, 3, default_bbox),
            make_test_cell("", 5, 4, default_bbox),
        ];

        let result = crate::pipeline::data_structures::postprocess_table_cells(cells, 10, 5);

        let row5_cells: Vec<_> = result
            .iter()
            .filter(|c| c.start_row_offset_idx == 5)
            .collect();

        // Verify: "Pythia-160M" | "NeoX" | "29.64" | "38.10" | "33.0"
        let col0 = row5_cells
            .iter()
            .find(|c| c.start_col_offset_idx == 0)
            .unwrap();
        assert_eq!(col0.text, "Pythia-160M", "Col 0 should get model name");

        let col1 = row5_cells
            .iter()
            .find(|c| c.start_col_offset_idx == 1)
            .unwrap();
        assert_eq!(col1.text, "NeoX", "Col 1 should get tokenizer");

        let col2 = row5_cells
            .iter()
            .find(|c| c.start_col_offset_idx == 2)
            .unwrap();
        assert_eq!(col2.text, "29.64", "Col 2 should get first number");

        let col3 = row5_cells
            .iter()
            .find(|c| c.start_col_offset_idx == 3)
            .unwrap();
        assert_eq!(col3.text, "38.10", "Col 3 should get second number");

        let col4 = row5_cells
            .iter()
            .find(|c| c.start_col_offset_idx == 4)
            .unwrap();
        assert_eq!(col4.text, "33.0", "Col 4 should get third number");
    }
}
