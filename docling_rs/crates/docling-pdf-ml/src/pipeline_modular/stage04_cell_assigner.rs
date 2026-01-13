// Stage 4: Cell Assignment - infrastructure for pipeline
// Note: Some helpers ported from Python not yet wired up.
#![allow(dead_code)]
// Intentional ML conversions: cell indices, overlap calculations
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
// ZST struct methods use &self for API consistency
#![allow(clippy::trivially_copy_pass_by_ref)]

use log::trace;
use ordered_float::OrderedFloat;
/// Stage 4: Cell Assignment
///
/// Assigns OCR text cells to layout clusters based on spatial overlap.
///
/// Algorithm:
/// 1. Initialize empty cell lists for all clusters
/// 2. For each text cell (skip if empty):
///    - Find cluster with best `intersection_over_self` ratio > `min_overlap`
///    - Assign cell to best cluster
/// 3. Deduplicate cells in each cluster by bbox
///
/// Key detail: Uses `intersection_over_self` (`intersection_area` / `cell_area`), NOT `IoU`!
use std::collections::HashSet;

use crate::pipeline_modular::types::{
    BBox, ClusterWithCells, ClustersWithCells, LabeledClusters, OCRCells, TextCell,
};

/// Configuration for Stage 4 (Cell Assignment)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stage04Config {
    /// Minimum `intersection_over_self` ratio (default 0.2 = 20% of cell must overlap cluster)
    pub min_overlap: f64,
    /// N=4404: Skip assigning OCR cells to TEXT-type clusters (makes them orphans)
    /// When true, OCR cells only check for Picture/Table containment but don't get assigned
    /// to TEXT clusters. This matches Python docling's behavior where each OCR fragment
    /// becomes its own text item with its own bbox.
    pub skip_text_clusters: bool,
}

impl Default for Stage04Config {
    #[inline]
    fn default() -> Self {
        Self {
            min_overlap: 0.2,
            // N=4404: Default false for backward compatibility with native text PDFs
            skip_text_clusters: false,
        }
    }
}

/// Stage 4: Cell Assigner
///
/// Assigns text cells to clusters based on spatial overlap.
///
/// Input: (`LabeledClusters`, `OCRCells`)
/// Output: `ClustersWithCells`
///
/// Algorithm:
/// - Greedy assignment: Each cell assigned to ONE cluster (best match)
/// - Overlap metric: `intersection_over_self` = `intersection_area` / `cell_area`
/// - Threshold: `min_overlap` (default 0.2 = 20% of cell must overlap cluster)
/// - Empty cells skipped (`text.trim()` must be non-empty)
/// - Cells deduplicated by bbox after assignment
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Stage04CellAssigner {
    config: Stage04Config,
}

impl Stage04CellAssigner {
    /// Create a new cell assigner with default configuration
    #[inline]
    #[must_use = "returns a new Stage04CellAssigner instance"]
    pub fn new() -> Self {
        Self {
            config: Stage04Config::default(),
        }
    }

    /// Create a new cell assigner with custom configuration
    #[inline]
    #[must_use = "returns a new Stage04CellAssigner with custom config"]
    pub const fn with_config(config: Stage04Config) -> Self {
        Self { config }
    }

    /// Check if a cluster label is a "special type" that should NOT get cells assigned
    ///
    /// From Python docling/utils/layout_postprocessor.py:164-170:
    /// - `SPECIAL_TYPES` = {PICTURE, FORM, `KEY_VALUE_REGION`, TABLE, `DOCUMENT_INDEX`}
    /// - These clusters are processed separately and don't participate in cell assignment
    #[inline]
    fn is_special_type(label: &str) -> bool {
        matches!(
            label.to_lowercase().as_str(),
            "picture"
                | "form"
                | "key-value region"
                | "key_value_region"
                | "table"
                | "document_index"
        )
    }

    /// Log initial state for debugging
    fn log_initial_state(clusters: &[ClusterWithCells], cells: &OCRCells) {
        trace!("\n=== STAGE 4 CELL ASSIGNMENT START ===");
        trace!("Total clusters from Stage 3: {}", clusters.len());
        trace!("Total cells to assign: {}", cells.cells.len());

        trace!("\nFirst 5 cells:");
        for (idx, cell) in cells.cells.iter().take(5).enumerate() {
            trace!(
                "  Cell[{}] bbox=({:.1},{:.1})→({:.1},{:.1}), area={:.1}, text={:?}",
                idx,
                cell.bbox.l,
                cell.bbox.t,
                cell.bbox.r,
                cell.bbox.b,
                cell.bbox.area(),
                cell.text.chars().take(30).collect::<String>()
            );
        }

        for (idx, cluster) in clusters.iter().take(20).enumerate() {
            trace!(
                "  Cluster[{}] label={}, bbox=({:.1},{:.1})→({:.1},{:.1}), area={:.1}",
                idx,
                cluster.label,
                cluster.bbox.l,
                cluster.bbox.t,
                cluster.bbox.r,
                cluster.bbox.b,
                cluster.bbox.area()
            );
        }
        if clusters.len() > 20 {
            trace!("  ... ({} more clusters)", clusters.len() - 20);
        }
    }

    /// Log assignment results for debugging
    fn log_assignment_results(
        clusters: &[ClusterWithCells],
        debug_assignments: &std::collections::HashMap<usize, Vec<(String, f64)>>,
        debug_cluster_ids: &[usize],
    ) {
        trace!("\n=== STAGE 4 CELL ASSIGNMENT RESULTS ===");
        let mut clusters_with_cells = 0;
        let mut clusters_without_cells = 0;

        for (idx, cluster) in clusters.iter().enumerate() {
            let num_cells = cluster.cells.len();
            if num_cells > 0 {
                clusters_with_cells += 1;
                trace!(
                    "  ✓ Cluster[{}] label={} got {} cells",
                    idx,
                    cluster.label,
                    num_cells
                );
            } else {
                clusters_without_cells += 1;
                if clusters_without_cells <= 10 {
                    trace!(
                        "  ✗ Cluster[{}] label={} got 0 cells (bbox area={:.1})",
                        idx,
                        cluster.label,
                        cluster.bbox.area()
                    );
                }
            }
        }

        trace!(
            "Summary: {clusters_with_cells} clusters WITH cells, {clusters_without_cells} clusters WITHOUT cells"
        );

        for &cluster_id in debug_cluster_ids {
            if cluster_id < clusters.len() {
                let num_cells = clusters[cluster_id].cells.len();
                let overlaps = debug_assignments.get(&cluster_id);
                log::warn!(
                    "  -> Cluster {}: {} cells assigned, {} candidates had overlap",
                    cluster_id,
                    num_cells,
                    overlaps.map_or(0, Vec::len)
                );
                if let Some(overlaps) = overlaps {
                    for (cell_text, ratio) in overlaps.iter().take(3) {
                        log::warn!("     - '{cell_text}...' overlap={ratio:.4}");
                    }
                }
            }
        }
    }

    /// Find best cluster for a cell based on overlap ratio
    ///
    /// Note: Skips "special type" clusters (`Picture`, `Table`, `Form`, `KeyValueRegion`, `DocumentIndex`)
    /// as per Python docling behavior. These clusters are processed separately and don't
    /// participate in cell assignment. This ensures OCR text becomes separate `TextItem`s
    /// instead of being embedded in `Picture.ocr_text` (which serializes as HTML comments).
    fn find_best_cluster(
        &self,
        cell: &TextCell,
        clusters: &[ClusterWithCells],
        debug_cluster_ids: &[usize],
        debug_assignments: &mut std::collections::HashMap<usize, Vec<(String, f64)>>,
    ) -> Option<usize> {
        let mut best_overlap = self.config.min_overlap;
        let mut best_cluster_idx: Option<usize> = None;

        for (idx, cluster) in clusters.iter().enumerate() {
            // N=4318: Skip special type clusters (Picture, Table, Form, etc.)
            // These are processed separately and should not get cells assigned.
            // Without this check, OCR text on scanned PDFs ends up in Picture.ocr_text
            // and serializes as <!-- figure-ocr: ... --> instead of regular text.
            if Self::is_special_type(&cluster.label) {
                continue;
            }

            // N=4404: Skip TEXT-type clusters if configured (for OCR mode)
            // This makes OCR cells become orphans instead of being merged into TEXT clusters,
            // matching Python docling's behavior where each OCR fragment is its own text item.
            if self.config.skip_text_clusters && cluster.label.to_lowercase() == "text" {
                continue;
            }

            let overlap_ratio = cell.bbox.intersection_over_self(&cluster.bbox);

            if overlap_ratio > best_overlap {
                best_overlap = overlap_ratio;
                best_cluster_idx = Some(idx);
            }

            // Track debug assignments
            if debug_cluster_ids.contains(&idx) && overlap_ratio > 0.0 {
                let cell_preview = cell.text.chars().take(20).collect::<String>();
                debug_assignments
                    .entry(idx)
                    .or_default()
                    .push((cell_preview, overlap_ratio));
            }
        }

        best_cluster_idx
    }

    /// Process labeled clusters and OCR cells to produce clusters with assigned cells
    #[must_use = "returns clusters with assigned cells"]
    pub fn process(
        &self,
        labeled_clusters: LabeledClusters,
        ocr_cells: OCRCells,
    ) -> ClustersWithCells {
        // Initialize clusters with empty cell lists
        let mut clusters: Vec<ClusterWithCells> = labeled_clusters
            .clusters
            .into_iter()
            .map(|cluster| ClusterWithCells {
                id: cluster.id,
                label: cluster.label,
                bbox: cluster.bbox,
                confidence: cluster.confidence,
                class_id: cluster.class_id,
                cells: Vec::new(),
            })
            .collect();

        Self::log_initial_state(&clusters, &ocr_cells);

        // N=4403: Collect Picture cluster bboxes to exclude cells inside them
        // Cells inside pictures should NOT be assigned to ANY cluster (including Text clusters)
        // They will become orphan text items in Stage 6 instead
        // Note: Copy bboxes to avoid borrow conflicts when mutating clusters later
        let picture_bboxes: Vec<BBox> = clusters
            .iter()
            .filter(|c| Self::is_special_type(&c.label))
            .map(|c| c.bbox)
            .collect();

        let debug_cluster_ids = vec![4, 6, 11, 13, 22, 24, 27, 33, 40, 68, 83, 88];
        let mut debug_assignments: std::collections::HashMap<usize, Vec<(String, f64)>> =
            std::collections::HashMap::new();

        // Assign each cell to best overlapping cluster
        for cell in ocr_cells.cells {
            // Skip empty cells or cells with invalid area
            if cell.text.trim().is_empty() || cell.bbox.area() <= 0.0 {
                continue;
            }

            // N=4403: Skip cells that are inside Picture bboxes
            // These should become orphan text items, not be merged into body text
            let inside_picture = picture_bboxes.iter().any(|picture_bbox| {
                cell.bbox.intersection_over_self(picture_bbox) > self.config.min_overlap
            });
            if inside_picture {
                log::debug!(
                    "Stage04: Skipping cell inside Picture bbox: {}",
                    &cell.text[..cell.text.len().min(50)]
                );
                continue;
            }

            // Find and assign to best cluster
            if let Some(idx) =
                self.find_best_cluster(&cell, &clusters, &debug_cluster_ids, &mut debug_assignments)
            {
                clusters[idx].cells.push(cell);
            }
        }

        Self::log_assignment_results(&clusters, &debug_assignments, &debug_cluster_ids);

        // Deduplicate cells in each cluster
        for cluster in &mut clusters {
            cluster.cells = Self::deduplicate_cells(&cluster.cells);
        }

        ClustersWithCells { clusters }
    }

    /// Deduplicate cells by their bbox coordinates
    ///
    /// Since `TextCell` doesn't have an 'index' field, we deduplicate by bbox.
    /// Preserves first occurrence of each unique bbox.
    fn deduplicate_cells(cells: &[TextCell]) -> Vec<TextCell> {
        // N=374: Use OrderedFloat instead of format!() to avoid String allocations
        // OrderedFloat is a wrapper that makes f32/f64 hashable by treating NaN/Inf consistently
        type BBoxKey = (
            OrderedFloat<f64>,
            OrderedFloat<f64>,
            OrderedFloat<f64>,
            OrderedFloat<f64>,
        );

        let mut seen_bboxes: HashSet<BBoxKey> = HashSet::new();
        // N=373: Preallocate with input size (best case: no duplicates)
        let mut unique_cells = Vec::with_capacity(cells.len());

        for cell in cells {
            // Create a hashable representation of bbox using OrderedFloat
            // Zero String allocations - uses stack-allocated tuple
            let bbox_key = (
                OrderedFloat(cell.bbox.l),
                OrderedFloat(cell.bbox.t),
                OrderedFloat(cell.bbox.r),
                OrderedFloat(cell.bbox.b),
            );

            if seen_bboxes.insert(bbox_key) {
                unique_cells.push(cell.clone());
            }
        }

        unique_cells
    }

    /// Get stage name for logging
    #[inline]
    #[must_use = "returns the stage name for logging"]
    pub const fn stage_name(&self) -> &'static str {
        "Stage04_CellAssigner"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline_modular::types::BBox;

    fn create_test_cluster(
        id: usize,
        bbox: BBox,
    ) -> crate::pipeline_modular::types::LabeledCluster {
        crate::pipeline_modular::types::LabeledCluster {
            id,
            label: "text".to_string(),
            bbox,
            confidence: 0.9,
            class_id: 9,
        }
    }

    fn create_test_cell(text: &str, bbox: BBox) -> TextCell {
        TextCell {
            text: text.to_string(),
            bbox,
            confidence: Some(0.9),
            is_bold: false,
            is_italic: false,
        }
    }

    #[test]
    fn test_cell_assignment_basic() {
        // Create a cluster at (0, 0, 100, 100)
        let cluster = create_test_cluster(0, BBox::new(0.0, 0.0, 100.0, 100.0));
        let clusters = LabeledClusters {
            clusters: vec![cluster],
        };

        // Create a cell at (10, 10, 30, 30) - fully inside cluster
        let cell = create_test_cell("test", BBox::new(10.0, 10.0, 30.0, 30.0));
        let cells = OCRCells { cells: vec![cell] };

        // Process
        let assigner = Stage04CellAssigner::new();
        let result = assigner.process(clusters, cells);

        // Check result
        assert_eq!(result.clusters.len(), 1);
        assert_eq!(result.clusters[0].cells.len(), 1);
        assert_eq!(result.clusters[0].cells[0].text, "test");
    }

    #[test]
    fn test_cell_assignment_no_overlap() {
        // Create a cluster at (0, 0, 100, 100)
        let cluster = create_test_cluster(0, BBox::new(0.0, 0.0, 100.0, 100.0));
        let clusters = LabeledClusters {
            clusters: vec![cluster],
        };

        // Create a cell at (200, 200, 220, 220) - no overlap
        let cell = create_test_cell("test", BBox::new(200.0, 200.0, 220.0, 220.0));
        let cells = OCRCells { cells: vec![cell] };

        // Process
        let assigner = Stage04CellAssigner::new();
        let result = assigner.process(clusters, cells);

        // Check result - cell should not be assigned
        assert_eq!(result.clusters.len(), 1);
        assert_eq!(result.clusters[0].cells.len(), 0);
    }

    #[test]
    fn test_cell_assignment_partial_overlap() {
        // Create a cluster at (0, 0, 100, 100)
        let cluster = create_test_cluster(0, BBox::new(0.0, 0.0, 100.0, 100.0));
        let clusters = LabeledClusters {
            clusters: vec![cluster],
        };

        // Create a cell at (50, 50, 150, 150) - 25% overlap (50x50 intersection, 100x100 cell)
        let cell = create_test_cell("test", BBox::new(50.0, 50.0, 150.0, 150.0));
        let cells = OCRCells { cells: vec![cell] };

        // Process with default min_overlap = 0.2
        let assigner = Stage04CellAssigner::new();
        let result = assigner.process(clusters, cells);

        // Check result - cell should be assigned (25% > 20%)
        assert_eq!(result.clusters.len(), 1);
        assert_eq!(result.clusters[0].cells.len(), 1);
    }

    #[test]
    fn test_cell_assignment_empty_text_skipped() {
        let cluster = create_test_cluster(0, BBox::new(0.0, 0.0, 100.0, 100.0));
        let clusters = LabeledClusters {
            clusters: vec![cluster],
        };

        // Create cells with empty text
        let cells = OCRCells {
            cells: vec![
                create_test_cell("", BBox::new(10.0, 10.0, 30.0, 30.0)),
                create_test_cell("   ", BBox::new(40.0, 40.0, 60.0, 60.0)),
            ],
        };

        let assigner = Stage04CellAssigner::new();
        let result = assigner.process(clusters, cells);

        // Both cells should be skipped
        assert_eq!(result.clusters[0].cells.len(), 0);
    }

    #[test]
    fn test_cell_assignment_best_match() {
        // Create two clusters
        let cluster1 = create_test_cluster(0, BBox::new(0.0, 0.0, 100.0, 100.0));
        let cluster2 = create_test_cluster(1, BBox::new(50.0, 50.0, 150.0, 150.0));
        let clusters = LabeledClusters {
            clusters: vec![cluster1, cluster2],
        };

        // Create a cell that overlaps both clusters
        // Cell at (60, 60, 80, 80) - 20x20 area = 400
        // Overlap with cluster1: 20x20 = 400 (100% of cell)
        // Overlap with cluster2: 20x20 = 400 (100% of cell)
        // Both have same overlap, should assign to first (cluster1)
        let cell = create_test_cell("test", BBox::new(60.0, 60.0, 80.0, 80.0));
        let cells = OCRCells { cells: vec![cell] };

        let assigner = Stage04CellAssigner::new();
        let result = assigner.process(clusters, cells);

        // Cell should be assigned to cluster with highest overlap
        // If ties, first cluster wins
        assert_eq!(result.clusters[0].cells.len(), 1);
        assert_eq!(result.clusters[1].cells.len(), 0);
    }

    #[test]
    fn test_deduplication() {
        let cluster = create_test_cluster(0, BBox::new(0.0, 0.0, 100.0, 100.0));
        let clusters = LabeledClusters {
            clusters: vec![cluster],
        };

        // Create duplicate cells (same bbox)
        let bbox = BBox::new(10.0, 10.0, 30.0, 30.0);
        let cells = OCRCells {
            cells: vec![
                create_test_cell("text1", bbox),
                create_test_cell("text2", bbox), // Duplicate bbox
            ],
        };

        let assigner = Stage04CellAssigner::new();
        let result = assigner.process(clusters, cells);

        // Only one cell should remain (first occurrence)
        assert_eq!(result.clusters[0].cells.len(), 1);
        assert_eq!(result.clusters[0].cells[0].text, "text1");
    }
}
