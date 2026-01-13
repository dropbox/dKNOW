/// Stage 5: Empty Cluster Removal
///
/// Removes clusters with no assigned cells, except special clusters which are always kept.
///
/// Algorithm:
/// 1. Filter clusters: Keep if (has cells) OR (is special type)
/// 2. Return filtered list
///
/// Special cases:
/// - Formula clusters: kept even if empty (for downstream processing)
/// - Empty pictures/tables/forms: REMOVED (added back in Stage 9 special cluster processing)
use crate::pipeline_modular::types::{ClusterWithCells, ClustersWithCells};

/// Check if a cluster should be kept even if empty
///
/// N=99 FIX: Keep FORMULA and TABLE clusters if empty.
/// - Formula clusters: Kept for downstream `CodeFormula` processing
/// - Table clusters: Kept for `TableFormer` processing (visual structure detection)
///   Empty tables (no OCR text) may still have valid visual table structure detectable by `TableFormer`
///
/// N=120D FIX (REVERTED in N=122, RE-ADDED in N=2398):
///
/// HISTORY:
/// - N=120D: Added picture to fix JFK (scanned PDFs with pictures containing no text)
/// - N=122: REVERTED because arxiv had 7 pictures (should be 6)
/// - N=2398: RE-ADDED after discovering Python's ACTUAL behavior
///
/// CORRECT UNDERSTANDING (N=2398):
/// - Python's _`process_regular_clusters()` removes empty clusters (line 277-282)
/// - Python's _`process_special_clusters()` does NOT remove empty clusters
/// - Pictures are SPECIAL clusters, so empty pictures are KEPT in Python
/// - The N=122 "7 pictures" issue was caused by a DIFFERENT bug (overlap removal?)
/// - Reference: ~/docling/docling/utils/layout_postprocessor.py:310-381
///
/// CURRENT STATUS (N=2398):
/// - arxiv 2305.03393v1: Has 6 images in Python groundtruth
/// - Rust was outputting 5 images (missing Figure 2, which has 0 cells)
/// - Adding "picture" to `should_keep_if_empty()` fixes this
///
/// N=606 INCOMPLETE: Only kept formulas, assumed empty tables would be "added back in Stage 9"
/// But Stage 9 never implemented this logic, causing table loss (edinet: 20 vs 25 tables)
fn should_keep_if_empty(label: &str) -> bool {
    matches!(
        label.to_lowercase().as_str(),
        "formula" | "table" | "picture" // N=2398: "picture" re-added - Python keeps empty pictures (special cluster processing)
    )
}

/// Configuration for Stage 5 (Empty Cluster Removal)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Stage05Config {
    /// Keep FORMULA clusters even if empty (N=606: only formulas, not pictures/tables/forms)
    pub keep_special: bool,
}

impl Default for Stage05Config {
    #[inline]
    fn default() -> Self {
        Self { keep_special: true }
    }
}

/// Stage 5: Empty Cluster Remover
///
/// Removes empty clusters (no cells) from the layout.
///
/// Input: `ClustersWithCells` (from Stage 4)
/// Output: `ClustersWithCells` (filtered)
///
/// Algorithm:
/// - Remove clusters with empty cells list
/// - Exception: Keep formula clusters even if empty (for downstream processing)
/// - Preserves cluster order and IDs
///
/// Example:
/// - Input: 98 clusters (30 with cells, 68 empty)
/// - Output: 30 clusters (all with cells, plus any empty formula clusters)
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Stage05EmptyRemover {
    config: Stage05Config,
}

impl Stage05EmptyRemover {
    /// Create a new `Stage05EmptyRemover` with default configuration
    #[inline]
    #[must_use = "empty remover stage is created but not used"]
    pub fn new() -> Self {
        Self {
            config: Stage05Config::default(),
        }
    }

    /// Create a new `Stage05EmptyRemover` with custom configuration
    #[inline]
    #[must_use = "empty remover stage is created but not used"]
    pub const fn with_config(config: Stage05Config) -> Self {
        Self { config }
    }

    /// Process clusters to remove empty ones (except formulas)
    ///
    /// # Arguments
    ///
    /// * `input` - `ClustersWithCells` from Stage 4
    ///
    /// # Returns
    ///
    /// `ClustersWithCells` with empty clusters removed (except formulas)
    #[must_use = "processed clusters are returned but not used"]
    pub fn process(&self, input: ClustersWithCells) -> ClustersWithCells {
        let filtered: Vec<ClusterWithCells> = input
            .clusters
            .into_iter()
            .filter(|cluster| {
                // Keep if has cells
                if !cluster.cells.is_empty() {
                    return true;
                }

                // N=606: Only keep FORMULA clusters if empty
                // Python's Stage 5 removes empty pictures/tables/forms - they're added back in Stage 9
                if self.config.keep_special && should_keep_if_empty(&cluster.label) {
                    return true;
                }

                // Otherwise, remove empty clusters (regular clusters with no cells)
                false
            })
            .collect();

        ClustersWithCells { clusters: filtered }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline_modular::types::{BBox, TextCell};

    fn make_cluster(id: usize, label: &str, has_cells: bool) -> ClusterWithCells {
        let cells = if has_cells {
            vec![TextCell {
                text: "test".to_string(),
                bbox: BBox::new(0.0, 0.0, 10.0, 10.0),
                confidence: Some(1.0),
                is_bold: false,
                is_italic: false,
            }]
        } else {
            vec![]
        };

        ClusterWithCells {
            id,
            label: label.to_string(),
            bbox: BBox::new(0.0, 0.0, 100.0, 100.0),
            confidence: 0.9,
            class_id: 0,
            cells,
        }
    }

    #[test]
    fn test_removes_empty_clusters() {
        let remover = Stage05EmptyRemover::new();

        let input = ClustersWithCells {
            clusters: vec![
                make_cluster(0, "text", true),  // has cells -> keep
                make_cluster(1, "text", false), // empty -> remove
                make_cluster(2, "text", true),  // has cells -> keep
            ],
        };

        let output = remover.process(input);

        assert_eq!(output.clusters.len(), 2);
        assert_eq!(output.clusters[0].id, 0);
        assert_eq!(output.clusters[1].id, 2);
    }

    #[test]
    fn test_keeps_empty_formula() {
        let remover = Stage05EmptyRemover::new();

        let input = ClustersWithCells {
            clusters: vec![
                make_cluster(0, "text", false),    // empty text -> remove
                make_cluster(1, "formula", false), // empty formula -> KEEP
                make_cluster(2, "text", true),     // has cells -> keep
            ],
        };

        let output = remover.process(input);

        assert_eq!(output.clusters.len(), 2);
        assert_eq!(output.clusters[0].id, 1);
        assert_eq!(output.clusters[0].label, "formula");
        assert_eq!(output.clusters[1].id, 2);
    }

    #[test]
    fn test_keeps_formula_with_cells() {
        let remover = Stage05EmptyRemover::new();

        let input = ClustersWithCells {
            clusters: vec![
                make_cluster(0, "formula", true), // formula with cells -> keep
            ],
        };

        let output = remover.process(input);

        assert_eq!(output.clusters.len(), 1);
        assert_eq!(output.clusters[0].id, 0);
        assert_eq!(output.clusters[0].label, "formula");
        assert!(
            !output.clusters[0].cells.is_empty(),
            "Formula cluster should retain its cells"
        );
    }

    #[test]
    fn test_preserves_cluster_order() {
        let remover = Stage05EmptyRemover::new();

        let input = ClustersWithCells {
            clusters: vec![
                make_cluster(5, "text", true),
                make_cluster(10, "text", false),
                make_cluster(15, "text", true),
                make_cluster(20, "text", false),
                make_cluster(25, "text", true),
            ],
        };

        let output = remover.process(input);

        assert_eq!(output.clusters.len(), 3);
        assert_eq!(output.clusters[0].id, 5);
        assert_eq!(output.clusters[1].id, 15);
        assert_eq!(output.clusters[2].id, 25);
    }

    #[test]
    fn test_config_keep_special_false() {
        let config = Stage05Config {
            keep_special: false,
        };
        let remover = Stage05EmptyRemover::with_config(config);

        let input = ClustersWithCells {
            clusters: vec![
                make_cluster(0, "text", false),    // empty text -> remove
                make_cluster(1, "formula", false), // empty formula -> REMOVE (config.keep_special = false)
                make_cluster(2, "picture", false), // empty picture -> REMOVE (config.keep_special = false)
                make_cluster(3, "text", true),     // has cells -> keep
            ],
        };

        let output = remover.process(input);

        assert_eq!(output.clusters.len(), 1);
        assert_eq!(output.clusters[0].id, 3);
    }

    #[test]
    fn test_keeps_empty_picture() {
        // N=2398: Empty pictures are KEPT (Python behavior)
        // Python's _process_special_clusters() does NOT remove empty clusters
        // Pictures are special clusters, so empty pictures survive
        let remover = Stage05EmptyRemover::new();

        let input = ClustersWithCells {
            clusters: vec![
                make_cluster(0, "text", false),    // empty text -> remove
                make_cluster(1, "picture", false), // empty picture -> KEEP (N=2398)
                make_cluster(2, "text", true),     // has cells -> keep
            ],
        };

        let output = remover.process(input);

        assert_eq!(output.clusters.len(), 2);
        assert_eq!(output.clusters[0].id, 1); // Empty picture kept
        assert_eq!(output.clusters[1].id, 2); // Text with cells kept
    }

    #[test]
    fn test_all_empty_no_formulas() {
        let remover = Stage05EmptyRemover::new();

        let input = ClustersWithCells {
            clusters: vec![
                make_cluster(0, "text", false),
                make_cluster(1, "caption", false),
                make_cluster(2, "section_header", false),
            ],
        };

        let output = remover.process(input);

        assert_eq!(output.clusters.len(), 0);
    }

    #[test]
    fn test_all_non_empty() {
        let remover = Stage05EmptyRemover::new();

        let input = ClustersWithCells {
            clusters: vec![
                make_cluster(0, "text", true),
                make_cluster(1, "caption", true),
                make_cluster(2, "section_header", true),
            ],
        };

        let output = remover.process(input);

        assert_eq!(output.clusters.len(), 3);
    }

    #[test]
    fn test_default_trait() {
        // Verify Default trait produces same result as new()
        let default_remover = Stage05EmptyRemover::default();
        let new_remover = Stage05EmptyRemover::new();
        assert_eq!(default_remover, new_remover);

        // Default config should keep special clusters
        let default_config = Stage05Config::default();
        assert!(default_config.keep_special);
    }
}
