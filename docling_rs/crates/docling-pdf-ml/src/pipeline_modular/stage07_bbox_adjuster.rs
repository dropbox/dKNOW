// Intentional ML conversions: bounding box coordinates
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
// ZST struct methods use &self for API consistency
#![allow(clippy::trivially_copy_pass_by_ref)]

use super::types::{BBox, ClusterWithCells, ClustersWithCells, TextCell};
/// Stage 07: Bbox Adjuster
///
/// Adjusts cluster bounding boxes to contain their assigned cells.
///
/// Input: `ClustersWithCells` (from Stage 06 - orphan creation)
/// Output: `ClustersWithCells` (same structure, adjusted bboxes)
///
/// Algorithm (from Python baseline ~/docling/docling/utils/layout_postprocessor.py:627-651):
///   For each cluster:
///     If cluster has no cells:
///       Skip (keep original bbox)
///     Else:
///       Calculate `cells_bbox` = union of all cell bboxes
///       If cluster.label == TABLE:
///         # Take union of current bbox and cells bbox
///         cluster.bbox = union(cluster.bbox, `cells_bbox`)
///       Else:
///         # Replace bbox with cells bbox
///         cluster.bbox = `cells_bbox`
///
/// Deterministic: Yes (same cell positions → same bbox adjustments)
/// Edge cases: Empty clusters keep original bbox (from layout model)
/// Special handling: TABLE clusters union with cells (preserves table borders)
use serde::{Deserialize, Serialize};

/// Configuration for Stage 07 (Bbox Adjuster)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Stage07Config {
    // No configuration parameters - algorithm is fixed
}

/// Stage 07: Adjust cluster bounding boxes to contain their assigned cells.
///
/// Input: `ClustersWithCells` (clusters after orphan creation)
/// Output: `ClustersWithCells` (same clusters, adjusted bboxes)
///
/// Logic:
///   - For non-empty clusters: bbox = cells bbox (except tables)
///   - For TABLE clusters: bbox = union(cluster bbox, cells bbox)
///   - For empty clusters: bbox unchanged
///
/// Deterministic: Yes (same cell positions → same bbox adjustments)
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Stage07BboxAdjuster {
    #[allow(dead_code, reason = "config stored for future configurability")]
    config: Stage07Config,
}

impl Stage07BboxAdjuster {
    /// Create a new Stage 07 bbox adjuster with default config
    #[inline]
    #[must_use = "returns a new Stage07BboxAdjuster instance"]
    pub fn new() -> Self {
        Self {
            config: Stage07Config::default(),
        }
    }

    /// Create a new Stage 07 bbox adjuster with custom config
    #[inline]
    #[must_use = "returns a new Stage07BboxAdjuster with custom config"]
    pub const fn with_config(config: Stage07Config) -> Self {
        Self { config }
    }

    /// Process clusters and adjust their bboxes based on assigned cells
    ///
    /// Args:
    ///     input: `ClustersWithCells` from Stage 06 (with orphan clusters)
    ///
    /// Returns:
    ///     `ClustersWithCells` with adjusted bboxes
    #[must_use = "returns the processed clusters with adjusted bboxes"]
    pub fn process(&self, input: ClustersWithCells) -> ClustersWithCells {
        let adjusted_clusters = input
            .clusters
            .into_iter()
            .map(|cluster| self.adjust_cluster_bbox(cluster))
            .collect();

        ClustersWithCells {
            clusters: adjusted_clusters,
        }
    }

    /// Adjust a single cluster's bbox based on its assigned cells
    fn adjust_cluster_bbox(&self, mut cluster: ClusterWithCells) -> ClusterWithCells {
        // Skip empty clusters (keep original bbox)
        if cluster.cells.is_empty() {
            return cluster;
        }

        // Calculate bbox that contains all cells
        let cells_bbox = self.calculate_cells_bbox(&cluster.cells);

        // Adjust cluster bbox
        if cluster.label.to_lowercase() == "table" {
            // TABLE: Take union of current bbox and cells bbox
            cluster.bbox = BBox::new(
                cluster.bbox.l.min(cells_bbox.l),
                cluster.bbox.t.min(cells_bbox.t),
                cluster.bbox.r.max(cells_bbox.r),
                cluster.bbox.b.max(cells_bbox.b),
            );
        } else {
            // Non-TABLE: Replace with cells bbox
            cluster.bbox = cells_bbox;
        }

        cluster
    }

    /// Calculate bounding box that contains all cells
    ///
    /// Args:
    ///     cells: List of `TextCell` objects
    ///
    /// Returns:
    ///     `BBox` that contains all cells
    // Method signature kept for API consistency with other BBoxAdjuster methods
    #[allow(clippy::unused_self)]
    fn calculate_cells_bbox(&self, cells: &[TextCell]) -> BBox {
        // Extract min/max coordinates from all cells
        let mut l = f64::INFINITY;
        let mut t = f64::INFINITY;
        let mut r = f64::NEG_INFINITY;
        let mut b = f64::NEG_INFINITY;

        for cell in cells {
            l = l.min(cell.bbox.l);
            t = t.min(cell.bbox.t);
            r = r.max(cell.bbox.r);
            b = b.max(cell.bbox.b);
        }

        BBox::new(l, t, r, b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_cluster_unchanged() {
        let adjuster = Stage07BboxAdjuster::new();

        let cluster = ClusterWithCells {
            id: 0,
            label: "text".to_string(),
            bbox: BBox::new(10.0, 10.0, 50.0, 50.0),
            confidence: 0.9,
            class_id: 1,
            cells: vec![],
        };

        let input = ClustersWithCells {
            clusters: vec![cluster],
        };

        let output = adjuster.process(input);

        assert_eq!(output.clusters.len(), 1);
        assert_eq!(output.clusters[0].bbox.l, 10.0);
        assert_eq!(output.clusters[0].bbox.t, 10.0);
        assert_eq!(output.clusters[0].bbox.r, 50.0);
        assert_eq!(output.clusters[0].bbox.b, 50.0);
    }

    #[test]
    fn test_non_table_bbox_replaced() {
        let adjuster = Stage07BboxAdjuster::new();

        let cluster = ClusterWithCells {
            id: 0,
            label: "text".to_string(),
            bbox: BBox::new(10.0, 10.0, 50.0, 50.0),
            confidence: 0.9,
            class_id: 1,
            cells: vec![
                TextCell {
                    text: "Hello".to_string(),
                    bbox: BBox::new(15.0, 15.0, 30.0, 25.0),
                    confidence: Some(1.0),
                    is_bold: false,
                    is_italic: false,
                },
                TextCell {
                    text: "World".to_string(),
                    bbox: BBox::new(15.0, 26.0, 30.0, 36.0),
                    confidence: Some(1.0),
                    is_bold: false,
                    is_italic: false,
                },
            ],
        };

        let input = ClustersWithCells {
            clusters: vec![cluster],
        };

        let output = adjuster.process(input);

        assert_eq!(output.clusters.len(), 1);
        // Bbox should be replaced with cells bbox (15, 15, 30, 36)
        assert_eq!(output.clusters[0].bbox.l, 15.0);
        assert_eq!(output.clusters[0].bbox.t, 15.0);
        assert_eq!(output.clusters[0].bbox.r, 30.0);
        assert_eq!(output.clusters[0].bbox.b, 36.0);
    }

    #[test]
    fn test_table_bbox_union() {
        let adjuster = Stage07BboxAdjuster::new();

        let cluster = ClusterWithCells {
            id: 0,
            label: "table".to_string(),
            bbox: BBox::new(10.0, 10.0, 50.0, 50.0),
            confidence: 0.9,
            class_id: 2,
            cells: vec![
                TextCell {
                    text: "Cell 1".to_string(),
                    bbox: BBox::new(15.0, 15.0, 30.0, 25.0),
                    confidence: Some(1.0),
                    is_bold: false,
                    is_italic: false,
                },
                TextCell {
                    text: "Cell 2".to_string(),
                    bbox: BBox::new(35.0, 35.0, 60.0, 45.0), // Extends beyond original bbox
                    confidence: Some(1.0),
                    is_bold: false,
                    is_italic: false,
                },
            ],
        };

        let input = ClustersWithCells {
            clusters: vec![cluster.clone()],
        };

        let output = adjuster.process(input);

        assert_eq!(output.clusters.len(), 1);
        // Bbox should be union of original (10, 10, 50, 50) and cells (15, 15, 60, 45)
        // = (10, 10, 60, 50)
        assert_eq!(output.clusters[0].bbox.l, 10.0);
        assert_eq!(output.clusters[0].bbox.t, 10.0);
        assert_eq!(output.clusters[0].bbox.r, 60.0);
        assert_eq!(output.clusters[0].bbox.b, 50.0);
    }

    #[test]
    fn test_calculate_cells_bbox() {
        let adjuster = Stage07BboxAdjuster::new();

        let cells = vec![
            TextCell {
                text: "A".to_string(),
                bbox: BBox::new(10.0, 20.0, 30.0, 40.0),
                confidence: Some(1.0),
                is_bold: false,
                is_italic: false,
            },
            TextCell {
                text: "B".to_string(),
                bbox: BBox::new(15.0, 15.0, 25.0, 35.0),
                confidence: Some(1.0),
                is_bold: false,
                is_italic: false,
            },
            TextCell {
                text: "C".to_string(),
                bbox: BBox::new(20.0, 25.0, 40.0, 45.0),
                confidence: Some(1.0),
                is_bold: false,
                is_italic: false,
            },
        ];

        let bbox = adjuster.calculate_cells_bbox(&cells);

        // Min l: 10.0, Min t: 15.0, Max r: 40.0, Max b: 45.0
        assert_eq!(bbox.l, 10.0);
        assert_eq!(bbox.t, 15.0);
        assert_eq!(bbox.r, 40.0);
        assert_eq!(bbox.b, 45.0);
    }

    #[test]
    fn test_multiple_clusters() {
        let adjuster = Stage07BboxAdjuster::new();

        let input = ClustersWithCells {
            clusters: vec![
                ClusterWithCells {
                    id: 0,
                    label: "text".to_string(),
                    bbox: BBox::new(10.0, 10.0, 50.0, 50.0),
                    confidence: 0.9,
                    class_id: 1,
                    cells: vec![TextCell {
                        text: "A".to_string(),
                        bbox: BBox::new(15.0, 15.0, 30.0, 25.0),
                        confidence: Some(1.0),
                        is_bold: false,
                        is_italic: false,
                    }],
                },
                ClusterWithCells {
                    id: 1,
                    label: "table".to_string(),
                    bbox: BBox::new(60.0, 60.0, 100.0, 100.0),
                    confidence: 0.95,
                    class_id: 2,
                    cells: vec![TextCell {
                        text: "B".to_string(),
                        bbox: BBox::new(65.0, 65.0, 80.0, 75.0),
                        confidence: Some(1.0),
                        is_bold: false,
                        is_italic: false,
                    }],
                },
                ClusterWithCells {
                    id: 2,
                    label: "caption".to_string(),
                    bbox: BBox::new(110.0, 110.0, 150.0, 150.0),
                    confidence: 0.85,
                    class_id: 3,
                    cells: vec![], // Empty
                },
            ],
        };

        let output = adjuster.process(input);

        assert_eq!(output.clusters.len(), 3);

        // Cluster 0 (text): bbox replaced with cells
        assert_eq!(output.clusters[0].bbox.l, 15.0);
        assert_eq!(output.clusters[0].bbox.t, 15.0);
        assert_eq!(output.clusters[0].bbox.r, 30.0);
        assert_eq!(output.clusters[0].bbox.b, 25.0);

        // Cluster 1 (table): bbox union with cells
        assert_eq!(output.clusters[1].bbox.l, 60.0);
        assert_eq!(output.clusters[1].bbox.t, 60.0);
        assert_eq!(output.clusters[1].bbox.r, 100.0);
        assert_eq!(output.clusters[1].bbox.b, 100.0);

        // Cluster 2 (empty): bbox unchanged
        assert_eq!(output.clusters[2].bbox.l, 110.0);
        assert_eq!(output.clusters[2].bbox.t, 110.0);
        assert_eq!(output.clusters[2].bbox.r, 150.0);
        assert_eq!(output.clusters[2].bbox.b, 150.0);
    }
}
