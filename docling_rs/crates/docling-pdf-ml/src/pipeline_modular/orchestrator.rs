/// Modular Pipeline Orchestrator
///
/// Coordinates all pipeline stages with proper iteration for bbox adjustment + overlap resolution.
///
/// The monolithic Python implementation (~/docling/docling/utils/layout_postprocessor.py:302-309)
/// uses 3 iterations of:
///   - _`adjust_cluster_bboxes` (Stage 07)
///   - _`remove_overlapping_clusters` (Stage 08)
///
/// This orchestrator replicates that behavior using modular stages.
use crate::pipeline_modular::{
    stage04_cell_assigner::Stage04Config,
    stage06_orphan_creator::Stage06Config,
    types::{ClustersWithCells, LabeledClusters, OCRCells},
    DocumentElement, Stage04CellAssigner, Stage05EmptyRemover, Stage06OrphanCreator,
    Stage07BboxAdjuster, Stage08OverlapResolver, Stage09DocumentAssembler, Stage10Output,
    Stage10ReadingOrder,
};

/// Modular pipeline orchestrator that coordinates stages 04-10 with iteration
#[derive(Debug, Clone)]
pub struct ModularPipeline {
    stage04: Stage04CellAssigner,
    stage05: Stage05EmptyRemover,
    stage06: Stage06OrphanCreator,
    stage07: Stage07BboxAdjuster,
    stage08: Stage08OverlapResolver,
    stage09: Stage09DocumentAssembler,
    stage10: Stage10ReadingOrder,
    max_iterations: usize,
    /// Debug output directory for saving intermediate stage outputs
    debug_output_dir: Option<std::path::PathBuf>,
}

impl ModularPipeline {
    /// Create a new pipeline with default configurations
    #[inline]
    #[must_use = "pipeline is created but not used"]
    pub fn new() -> Self {
        Self {
            stage04: Stage04CellAssigner::new(),
            stage05: Stage05EmptyRemover::new(),
            stage06: Stage06OrphanCreator::new(),
            stage07: Stage07BboxAdjuster::new(),
            stage08: Stage08OverlapResolver::new(),
            stage09: Stage09DocumentAssembler::new(),
            stage10: Stage10ReadingOrder::new(),
            max_iterations: 3,
            debug_output_dir: None,
        }
    }

    /// Create a new pipeline with custom max iterations
    #[inline]
    #[must_use = "pipeline is created but not used"]
    pub fn with_max_iterations(max_iterations: usize) -> Self {
        Self {
            stage04: Stage04CellAssigner::new(),
            stage05: Stage05EmptyRemover::new(),
            stage06: Stage06OrphanCreator::new(),
            stage07: Stage07BboxAdjuster::new(),
            stage08: Stage08OverlapResolver::new(),
            stage09: Stage09DocumentAssembler::new(),
            stage10: Stage10ReadingOrder::new(),
            max_iterations,
            debug_output_dir: None,
        }
    }

    /// Create a new pipeline with debug output enabled
    #[inline]
    #[must_use = "pipeline is created but not used"]
    pub fn with_debug_output(debug_dir: std::path::PathBuf) -> Self {
        Self {
            stage04: Stage04CellAssigner::new(),
            stage05: Stage05EmptyRemover::new(),
            stage06: Stage06OrphanCreator::new(),
            stage07: Stage07BboxAdjuster::new(),
            stage08: Stage08OverlapResolver::new(),
            stage09: Stage09DocumentAssembler::new(),
            stage10: Stage10ReadingOrder::new(),
            max_iterations: 3,
            debug_output_dir: Some(debug_dir),
        }
    }

    /// Create a new pipeline configured for OCR mode
    ///
    /// N=4404: In OCR mode, we want each OCR cell to become its own text item
    /// (matching Python docling behavior). This is achieved by:
    /// 1. Stage04: Skip assigning cells to TEXT clusters (makes them orphans)
    /// 2. Stage06: Disable paragraph merging (each orphan is its own item)
    #[inline]
    #[must_use = "pipeline is created but not used"]
    pub fn for_ocr_mode() -> Self {
        let stage04_config = Stage04Config {
            skip_text_clusters: true,
            ..Default::default()
        };
        let stage06_config = Stage06Config {
            merge_paragraphs: false,
            ..Default::default()
        };
        Self {
            stage04: Stage04CellAssigner::with_config(stage04_config),
            stage05: Stage05EmptyRemover::new(),
            stage06: Stage06OrphanCreator::with_config(stage06_config),
            stage07: Stage07BboxAdjuster::new(),
            stage08: Stage08OverlapResolver::new(),
            stage09: Stage09DocumentAssembler::new(),
            stage10: Stage10ReadingOrder::new(),
            max_iterations: 3,
            debug_output_dir: None,
        }
    }

    /// Process stages 4-8 of the pipeline (without page context)
    ///
    /// Args:
    ///   `stage3_clusters`: Labeled clusters from Stage 3 (HF postprocessing)
    ///   `ocr_cells`: OCR text cells
    ///
    /// Returns:
    ///   `ClustersWithCells` after all processing stages
    ///
    /// Note: For page-aware heuristics (title/header/footer detection),
    /// use `process_stages_4_to_8_with_page_context` instead.
    #[must_use = "processed clusters are returned but not used"]
    pub fn process_stages_4_to_8(
        &self,
        stage3_clusters: LabeledClusters,
        ocr_cells: OCRCells,
    ) -> ClustersWithCells {
        self.process_stages_4_to_8_impl(stage3_clusters, ocr_cells, None, None, None)
    }

    /// Process stages 4-8 of the pipeline with page context for heuristics
    ///
    /// Args:
    ///   `stage3_clusters`: Labeled clusters from Stage 3 (HF postprocessing)
    ///   `ocr_cells`: OCR text cells
    ///   `page_no`: Page number (0-indexed), used for title detection
    ///   `page_width`: Page width in model coordinates
    ///   `page_height`: Page height in model coordinates
    ///
    /// Returns:
    ///   `ClustersWithCells` after all processing stages (with heuristic labels)
    #[must_use = "processed clusters are returned but not used"]
    pub fn process_stages_4_to_8_with_page_context(
        &self,
        stage3_clusters: LabeledClusters,
        ocr_cells: OCRCells,
        page_no: usize,
        page_width: f64,
        page_height: f64,
    ) -> ClustersWithCells {
        self.process_stages_4_to_8_impl(
            stage3_clusters,
            ocr_cells,
            Some(page_no),
            Some(page_width),
            Some(page_height),
        )
    }

    /// Internal implementation for stages 4-8
    fn process_stages_4_to_8_impl(
        &self,
        stage3_clusters: LabeledClusters,
        ocr_cells: OCRCells,
        page_no: Option<usize>,
        page_width: Option<f64>,
        page_height: Option<f64>,
    ) -> ClustersWithCells {
        // Stage 4: Cell Assignment
        log::debug!(
            "Stage 04: Assigning cells to {} clusters...",
            stage3_clusters.clusters.len()
        );
        let clusters_with_cells = self.stage04.process(stage3_clusters, ocr_cells.clone());
        let num_with_cells = clusters_with_cells
            .clusters
            .iter()
            .filter(|c| !c.cells.is_empty())
            .count();

        // N=595: Diagnostic - show which cluster IDs have cells (for comparison with Python)
        let ids_with_cells: Vec<usize> = clusters_with_cells
            .clusters
            .iter()
            .enumerate()
            .filter(|(_, c)| !c.cells.is_empty())
            .map(|(i, _)| i)
            .collect();
        log::debug!(
            "  -> {} total clusters, {} have cells, {} empty",
            clusters_with_cells.clusters.len(),
            num_with_cells,
            clusters_with_cells.clusters.len() - num_with_cells
        );
        log::debug!("  -> Cluster IDs with cells: {ids_with_cells:?}");

        // Stage 5: Empty Cluster Removal (keeps special types even if empty)
        log::debug!("Stage 05: Removing empty clusters...");
        let removed_before = clusters_with_cells.clusters.len();
        let non_empty = self.stage05.process(clusters_with_cells);
        let removed = removed_before - non_empty.clusters.len();
        let kept_special = non_empty
            .clusters
            .iter()
            .filter(|c| c.cells.is_empty())
            .count();
        log::debug!(
            "  -> {} clusters kept ({} removed), {} empty special types kept",
            non_empty.clusters.len(),
            removed,
            kept_special
        );

        // Stage 6: Orphan Cluster Creation (needs original OCR cells)
        // N=3533: Use page context for title/header/footer heuristics if available
        log::debug!("Stage 06: Creating orphan clusters...");
        let orphans_before = non_empty.clusters.len();
        let with_orphans =
            if let (Some(pno), Some(pw), Some(ph)) = (page_no, page_width, page_height) {
                self.stage06
                    .process_with_page_context(non_empty, ocr_cells, pno, pw, ph)
            } else {
                self.stage06.process(non_empty, ocr_cells)
            };
        let orphans_created = with_orphans.clusters.len().saturating_sub(orphans_before);
        log::debug!(
            "  -> {} clusters ({} orphans created)",
            with_orphans.clusters.len(),
            orphans_created
        );

        // Stages 7+8: Iterative refinement (bbox adjustment + overlap resolution)
        // This matches layout_postprocessor.py:302-309
        log::debug!(
            "Stages 07+08: Iterative refinement (max {} iterations)...",
            self.max_iterations
        );

        let mut clusters = with_orphans;
        let mut prev_count = clusters.clusters.len() + 1;

        for iteration in 0..self.max_iterations {
            // Check convergence
            if prev_count == clusters.clusters.len() {
                log::debug!("  -> Converged at iteration {iteration} (count unchanged)");
                // Save convergence info
                if let Some(ref debug_dir) = self.debug_output_dir {
                    self.save_convergence_info(debug_dir, iteration, clusters.clusters.len(), true);
                }
                break;
            }

            prev_count = clusters.clusters.len();

            // Stage 07: Bbox Adjustment (takes ownership)
            let adjusted = self.stage07.process(clusters);
            log::debug!(
                "  Iteration {} - Stage 07: {} clusters",
                iteration + 1,
                adjusted.clusters.len()
            );

            // Save per-iteration Stage 7 output
            if let Some(ref debug_dir) = self.debug_output_dir {
                self.save_clusters(
                    debug_dir,
                    &format!("stage7_iteration{}_adjusted", iteration + 1),
                    &adjusted,
                );
            }

            // Stage 08: Overlap Resolution (takes ownership)
            let resolved = self.stage08.process(adjusted);
            let clusters_removed = prev_count - resolved.clusters.len();
            log::debug!(
                "  Iteration {} - Stage 08: {} clusters ({} overlaps removed)",
                iteration + 1,
                resolved.clusters.len(),
                clusters_removed
            );

            // Save per-iteration Stage 8 output
            if let Some(ref debug_dir) = self.debug_output_dir {
                self.save_clusters(
                    debug_dir,
                    &format!("stage8_iteration{}_resolved", iteration + 1),
                    &resolved,
                );
            }

            // Prepare for next iteration
            clusters = resolved;
        }

        // Return clusters after loop converges (matches Python: layout_postprocessor.py:310)
        clusters
    }

    /// Process stages 4-9 of the pipeline (includes document assembly)
    ///
    /// Args:
    ///   `stage3_clusters`: Labeled clusters from Stage 3 (HF postprocessing)
    ///   `ocr_cells`: OCR text cells
    ///   `page_no`: Page number
    ///   `page_width`: Page width in pixels
    ///   `page_height`: Page height in pixels
    ///
    /// Returns:
    ///   `Vec<DocumentElement>` - assembled document elements
    #[must_use = "document elements are returned but not used"]
    pub fn process_stages_4_to_9(
        &self,
        stage3_clusters: LabeledClusters,
        ocr_cells: OCRCells,
        page_no: usize,
        page_width: f64,
        page_height: f64,
    ) -> Vec<DocumentElement> {
        // Run stages 4-8 with page context for title/header/footer heuristics
        let resolved_clusters = self.process_stages_4_to_8_with_page_context(
            stage3_clusters,
            ocr_cells,
            page_no,
            page_width,
            page_height,
        );

        // Stage 9: Document Assembly
        log::debug!(
            "Stage 09: Assembling {} clusters into document elements...",
            resolved_clusters.clusters.len()
        );
        let elements =
            self.stage09
                .process(resolved_clusters.clusters, page_no, page_width, page_height);
        log::debug!("  -> {} document elements", elements.len());

        elements
    }

    /// Process stages 4-10 of the pipeline (includes reading order)
    ///
    /// Args:
    ///   `stage3_clusters`: Labeled clusters from Stage 3 (HF postprocessing)
    ///   `ocr_cells`: OCR text cells
    ///   `page_no`: Page number
    ///   `page_width`: Page width in pixels
    ///   `page_height`: Page height in pixels
    ///
    /// Returns:
    ///   `Stage10Output` - sorted document elements with reading order applied
    #[must_use = "pipeline output is returned but not used"]
    pub fn process_stages_4_to_10(
        &self,
        stage3_clusters: LabeledClusters,
        ocr_cells: OCRCells,
        page_no: usize,
        page_width: f64,
        page_height: f64,
    ) -> Stage10Output {
        // Run stages 4-9
        let elements = self.process_stages_4_to_9(
            stage3_clusters,
            ocr_cells,
            page_no,
            page_width,
            page_height,
        );

        // Stage 10: Reading Order
        log::debug!(
            "Stage 10: Applying reading order to {} elements...",
            elements.len()
        );
        let output = self.stage10.process(elements);
        log::debug!("  -> {} sorted elements", output.sorted_elements.len());

        output
    }

    /// Debug method: Run only Stage 04 and return output for comparison
    /// N=315: Added to help debug cell assignment discrepancy
    #[must_use = "debug output is returned but not used"]
    pub fn debug_stage04_output(
        &self,
        stage3_clusters: LabeledClusters,
        ocr_cells: OCRCells,
    ) -> Option<ClustersWithCells> {
        Some(self.stage04.process(stage3_clusters, ocr_cells))
    }

    /// Save `ClustersWithCells` to JSON file (for debugging/validation)
    // Method signature kept for API consistency with other PipelineOrchestrator methods
    #[allow(clippy::unused_self)]
    fn save_clusters(
        &self,
        debug_dir: &std::path::Path,
        filename: &str,
        clusters_with_cells: &ClustersWithCells,
    ) {
        use serde_json;
        use std::fs;

        let output_dir = debug_dir;
        if let Err(e) = fs::create_dir_all(output_dir) {
            log::error!("Failed to create debug directory: {e}");
            return;
        }

        let output_path = output_dir.join(format!("{filename}.json"));

        // ClustersWithCells should already be serializable via Serialize derive
        // Just serialize it directly
        match serde_json::to_string_pretty(&clusters_with_cells.clusters) {
            Ok(json_str) => {
                if let Err(e) = fs::write(&output_path, json_str) {
                    log::error!("Failed to save {}: {}", output_path.display(), e);
                }
            }
            Err(e) => {
                log::error!("Failed to serialize {filename}: {e}");
            }
        }
    }

    /// Save convergence info to JSON file
    fn save_convergence_info(
        &self,
        debug_dir: &std::path::Path,
        iteration: usize,
        final_count: usize,
        converged_early: bool,
    ) {
        use serde_json;
        use std::fs;

        let output_path = debug_dir.join("loop_convergence_info.json");

        let info = serde_json::json!({
            "converged_at_iteration": iteration,
            "final_cluster_count": final_count,
            "converged_early": converged_early,
            "max_iterations": self.max_iterations
        });

        if let Err(e) = fs::write(&output_path, serde_json::to_string_pretty(&info).unwrap()) {
            log::error!("Failed to save convergence info: {e}");
        }
    }
}

impl Default for ModularPipeline {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline_modular::types::{BBox, LabeledCluster, TextCell};

    #[test]
    fn test_orchestrator_basic() {
        // Create a simple test case with 1 cluster and 1 cell
        let clusters = LabeledClusters {
            clusters: vec![LabeledCluster {
                id: 0,
                label: "text".to_string(),
                bbox: BBox::new(10.0, 10.0, 100.0, 50.0),
                confidence: 0.9,
                class_id: 1,
            }],
        };

        let cells = OCRCells {
            cells: vec![TextCell {
                text: "Hello World".to_string(),
                bbox: BBox::new(15.0, 15.0, 95.0, 45.0),
                confidence: Some(0.95),
                is_bold: false,
                is_italic: false,
            }],
        };

        let pipeline = ModularPipeline::new();
        let result = pipeline.process_stages_4_to_8(clusters, cells);

        // Should have at least 1 cluster (either the original or an orphan)
        assert!(
            !result.clusters.is_empty(),
            "Pipeline should produce at least one cluster"
        );
    }
}
