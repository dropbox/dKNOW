// Stage 08: Overlap Resolution - infrastructure for pipeline
// Note: Some helpers ported from Python not yet wired up.
#![allow(dead_code)]
// Intentional ML conversions: cluster indices, bounding box coordinates
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
// Pipeline stage functions take Vec ownership for data flow semantics
#![allow(clippy::needless_pass_by_value)]

use super::types::{BBox, ClusterWithCells, ClustersWithCells, TextCell};
use rustc_hash::FxHashMap; // N=373: 2-4x faster than std::HashMap for integer keys
/// Stage 08: Overlap Resolution
///
/// Resolves overlapping clusters by merging them using Union-Find algorithm.
///
/// Algorithm (from ~/docling/docling/utils/layout_postprocessor.py:487-542):
/// 1. Separate clusters into 3 types: regular, picture, wrapper
/// 2. For each type, use Union-Find to group overlapping clusters
/// 3. Within each group, select the "best" cluster based on rules
/// 4. Merge cells from other clusters into the best cluster
/// 5. Deduplicate and sort cells
///
/// Input: `ClustersWithCells` (clusters with adjusted bboxes from Stage 07)
/// Output: `ClustersWithCells` (no overlapping clusters)
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Configuration for Stage 08 (Overlap Resolution)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Stage08Config {
    /// `IoU` threshold for considering overlap
    pub overlap_threshold: f64,
    /// Containment threshold for overlap
    pub containment_threshold: f64,

    // Parameters for selecting best cluster from overlapping groups
    pub regular_area_threshold: f64,
    pub regular_conf_threshold: f64,
    pub picture_area_threshold: f64,
    pub picture_conf_threshold: f64,
    pub wrapper_area_threshold: f64,
    pub wrapper_conf_threshold: f64,

    // Label-specific merge rule parameters
    /// Area similarity threshold for `LIST_ITEM` vs `TEXT` rule (default: 0.2 = 20%)
    pub list_item_area_similarity_threshold: f64,
    /// Containment threshold for CODE rule (default: 0.8 = 80%)
    pub code_containment_threshold: f64,
}

impl Default for Stage08Config {
    #[inline]
    fn default() -> Self {
        Self {
            overlap_threshold: 0.8,
            containment_threshold: 0.8,
            regular_area_threshold: 1.3,
            regular_conf_threshold: 0.05,
            picture_area_threshold: 2.0,
            picture_conf_threshold: 0.3,
            wrapper_area_threshold: 2.0,
            wrapper_conf_threshold: 0.2,
            list_item_area_similarity_threshold: 0.2,
            code_containment_threshold: 0.8,
        }
    }
}

/// Union-Find data structure for grouping elements
///
/// From ~/docling/docling/utils/layout_postprocessor.py:16-46
struct UnionFind {
    parent: FxHashMap<usize, usize>,
    rank: FxHashMap<usize, usize>,
}

impl UnionFind {
    /// Initialize with a collection of element IDs
    fn new(elements: &[usize]) -> Self {
        let parent = elements.iter().map(|&e| (e, e)).collect();
        let rank = elements.iter().map(|&e| (e, 0)).collect();
        Self { parent, rank }
    }

    /// Find root of element with path compression
    fn find(&mut self, x: usize) -> usize {
        if self.parent[&x] != x {
            let root = self.find(self.parent[&x]);
            self.parent.insert(x, root);
        }
        self.parent[&x]
    }

    /// Union two elements by rank
    fn union(&mut self, x: usize, y: usize) {
        let root_x = self.find(x);
        let root_y = self.find(y);

        if root_x == root_y {
            return;
        }

        // Union by rank
        let rank_x = self.rank[&root_x];
        let rank_y = self.rank[&root_y];

        match rank_x.cmp(&rank_y) {
            std::cmp::Ordering::Greater => {
                self.parent.insert(root_y, root_x);
            }
            std::cmp::Ordering::Less => {
                self.parent.insert(root_x, root_y);
            }
            std::cmp::Ordering::Equal => {
                self.parent.insert(root_y, root_x);
                self.rank.insert(root_x, rank_x + 1);
            }
        }
    }

    /// Get groups as {root: [elements]}
    fn get_groups(&mut self) -> FxHashMap<usize, Vec<usize>> {
        let mut groups: FxHashMap<usize, Vec<usize>> = FxHashMap::default();

        let elements: Vec<usize> = self.parent.keys().copied().collect();
        for elem in elements {
            let root = self.find(elem);
            groups.entry(root).or_default().push(elem);
        }

        groups
    }
}

/// Stage 08: Resolve overlapping clusters by merging them
///
/// Algorithm:
/// 1. Classify clusters into types (regular, picture, wrapper)
/// 2. For each type, find overlapping clusters using Union-Find
/// 3. Merge overlapping groups, selecting best cluster
/// 4. Deduplicate and sort cells
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Stage08OverlapResolver {
    config: Stage08Config,
}

impl Stage08OverlapResolver {
    // Wrapper types (special clusters that can contain other clusters)
    const WRAPPER_TYPES: &'static [&'static str] =
        &["form", "key_value_region", "table", "document_index"];

    // Picture type
    const PICTURE_TYPE: &'static str = "picture";

    /// Create a new Stage 08 overlap resolver with default config
    #[inline]
    #[must_use = "returns a new Stage08OverlapResolver instance"]
    pub fn new() -> Self {
        Self {
            config: Stage08Config::default(),
        }
    }

    /// Create a new Stage 08 overlap resolver with custom config
    #[inline]
    #[must_use = "returns a new Stage08OverlapResolver with custom config"]
    pub const fn with_config(config: Stage08Config) -> Self {
        Self { config }
    }

    /// Process clusters and resolve overlaps
    ///
    /// Args:
    ///     input: Clusters with adjusted bboxes
    ///
    /// Returns:
    ///     Clusters with overlaps resolved
    #[must_use = "returns the processed clusters with overlaps resolved"]
    pub fn process(&self, input: ClustersWithCells) -> ClustersWithCells {
        let clusters = input.clusters;

        // Classify clusters by type
        let mut regular_clusters = Vec::new();
        let mut picture_clusters = Vec::new();
        let mut wrapper_clusters = Vec::new();

        for cluster in clusters {
            // Normalize label: lowercase and replace hyphens/spaces with underscores
            let label = Self::normalize_label(&cluster.label);

            if Self::WRAPPER_TYPES.contains(&label.as_str()) {
                wrapper_clusters.push(cluster);
            } else if label == Self::PICTURE_TYPE {
                picture_clusters.push(cluster);
            } else {
                regular_clusters.push(cluster);
            }
        }

        // Resolve overlaps for each type
        let resolved_regular =
            self.remove_overlapping_clusters(regular_clusters, ClusterType::Regular);

        let resolved_picture =
            self.remove_overlapping_clusters(picture_clusters, ClusterType::Picture);

        let resolved_wrapper =
            self.remove_overlapping_clusters(wrapper_clusters, ClusterType::Wrapper);

        // Combine all resolved clusters
        let mut all_clusters = Vec::new();
        all_clusters.extend(resolved_regular);
        all_clusters.extend(resolved_picture);
        all_clusters.extend(resolved_wrapper);

        // Normalize cluster labels (replace hyphens/spaces with underscores)
        for cluster in &mut all_clusters {
            cluster.label = Self::normalize_label(&cluster.label);
        }

        ClustersWithCells {
            clusters: all_clusters,
        }
    }

    /// Normalize label: lowercase and replace hyphens/spaces with underscores
    fn normalize_label(label: &str) -> String {
        label.to_lowercase().replace(['-', ' '], "_")
    }

    /// Remove overlapping clusters of a specific type
    ///
    /// From ~/docling/docling/utils/layout_postprocessor.py:487-542
    fn remove_overlapping_clusters(
        &self,
        clusters: Vec<ClusterWithCells>,
        cluster_type: ClusterType,
    ) -> Vec<ClusterWithCells> {
        if clusters.is_empty() {
            return Vec::new();
        }

        // Get parameters for this cluster type
        let params = match cluster_type {
            ClusterType::Regular => ClusterParams {
                area_threshold: self.config.regular_area_threshold,
                conf_threshold: self.config.regular_conf_threshold,
            },
            ClusterType::Picture => ClusterParams {
                area_threshold: self.config.picture_area_threshold,
                conf_threshold: self.config.picture_conf_threshold,
            },
            ClusterType::Wrapper => ClusterParams {
                area_threshold: self.config.wrapper_area_threshold,
                conf_threshold: self.config.wrapper_conf_threshold,
            },
        };

        // Map of currently valid clusters
        let cluster_ids: Vec<usize> = clusters.iter().map(|c| c.id).collect();
        let valid_clusters: FxHashMap<usize, ClusterWithCells> =
            clusters.into_iter().map(|c| (c.id, c)).collect();

        let mut uf = UnionFind::new(&cluster_ids);

        // Find overlapping pairs and union them
        // N=607: Merge clusters if they overlap, REGARDLESS of label
        // From ~/docling/docling/utils/layout_postprocessor.py:487-542 _remove_overlapping_clusters()
        // Official Docling does NOT check label equality - only overlap!
        // Reference repo was wrong (had "same label" check that doesn't exist in official Docling)
        for cluster_id in &cluster_ids {
            for other_id in &cluster_ids {
                if cluster_id == other_id {
                    continue;
                }

                let cluster = &valid_clusters[cluster_id];
                let other = &valid_clusters[other_id];

                // N=607: REMOVED "only merge same label" check - official Docling merges all overlapping clusters
                // This fixes cluster 21 (list_item) merging with cluster 10 (text)

                if self.check_overlap(&cluster.bbox, &other.bbox) {
                    uf.union(*cluster_id, *other_id);
                }
                // N=2281: DISABLED - Too aggressive, merges everything into 1 DocItem per page
                // Root cause is NOT in overlap resolver - it's earlier in the pipeline
                // Python produces 53 semantic units, Rust produces 115 geometric fragments
                // Need to investigate cluster generation, not post-processing
            }
        }

        // Process each group
        let mut result = Vec::new();
        for group in uf.get_groups().values() {
            if group.len() == 1 {
                // No overlap, keep cluster as-is
                result.push(valid_clusters[&group[0]].clone());
                continue;
            }

            // Multiple overlapping clusters - select best and merge cells
            // Sort by ID to ensure deterministic order (matches Python behavior)
            let mut sorted_group = group.clone();
            sorted_group.sort_unstable();
            let group_clusters: Vec<&ClusterWithCells> =
                sorted_group.iter().map(|id| &valid_clusters[id]).collect();

            let best_id = self.select_best_cluster_from_group(&group_clusters, &params);
            let mut best = valid_clusters[&best_id].clone();

            // Merge cells from all clusters in group into best cluster
            for cluster_id in group {
                if *cluster_id != best_id {
                    best.cells.extend(valid_clusters[cluster_id].cells.clone());
                }
            }

            // Deduplicate and sort cells
            best.cells = Self::deduplicate_cells(best.cells);
            best.cells = Self::sort_cells(best.cells);
            result.push(best);
        }

        result
    }

    /// Check if two bboxes overlap sufficiently
    ///
    /// From ~/docling/docling/utils/layout_postprocessor.py:85-104
    #[inline]
    fn check_overlap(&self, bbox1: &BBox, bbox2: &BBox) -> bool {
        if bbox1.area() <= 0.0 || bbox2.area() <= 0.0 {
            return false;
        }

        let iou = bbox1.iou(bbox2);
        let containment1 = bbox1.intersection_over_self(bbox2);
        let containment2 = bbox2.intersection_over_self(bbox1);

        iou > self.config.overlap_threshold
            || containment1 > self.config.containment_threshold
            || containment2 > self.config.containment_threshold
    }

    /// Select best cluster from a group of overlapping clusters
    ///
    /// From ~/docling/docling/utils/layout_postprocessor.py:454-486
    fn select_best_cluster_from_group(
        &self,
        group_clusters: &[&ClusterWithCells],
        params: &ClusterParams,
    ) -> usize {
        let mut current_best: Option<&ClusterWithCells> = None;

        for candidate in group_clusters {
            let mut should_select = true;

            for other in group_clusters {
                if other.id == candidate.id {
                    continue;
                }

                if !self.should_prefer_cluster(candidate, other, params) {
                    should_select = false;
                    break;
                }
            }

            if should_select {
                match current_best {
                    None => current_best = Some(candidate),
                    Some(best) => {
                        // If both clusters pass rules, prefer the larger one
                        // unless confidence differs significantly
                        if candidate.bbox.area() > best.bbox.area()
                            && best.confidence - candidate.confidence <= params.conf_threshold
                        {
                            current_best = Some(candidate);
                        }
                    }
                }
            }
        }

        current_best.map_or(group_clusters[0].id, |c| c.id)
    }

    /// Determine if candidate should be preferred over other cluster
    ///
    /// Returns True if candidate should be preferred, False otherwise.
    ///
    /// From ~/docling/docling/utils/layout_postprocessor.py:418-452
    fn should_prefer_cluster(
        &self,
        candidate: &ClusterWithCells,
        other: &ClusterWithCells,
        params: &ClusterParams,
    ) -> bool {
        // Normalize labels
        let candidate_label = Self::normalize_label(&candidate.label);
        let other_label = Self::normalize_label(&other.label);

        // Rule 1: LIST_ITEM vs TEXT
        if candidate_label == "list_item" && other_label == "text" {
            // Check if areas are similar (within threshold of each other)
            let area_ratio = if other.bbox.area() > 0.0 {
                candidate.bbox.area() / other.bbox.area()
            } else {
                0.0
            };
            let area_similarity =
                (1.0 - area_ratio).abs() < self.config.list_item_area_similarity_threshold;
            if area_similarity {
                return true;
            }
        }

        // Rule 2: CODE vs others
        if candidate_label == "code" {
            // Calculate how much of other is contained within CODE cluster
            let containment = other.bbox.intersection_over_self(&candidate.bbox);
            if containment > self.config.code_containment_threshold {
                // other is contained within CODE cluster beyond threshold
                return true;
            }
        }

        // If no label-based rules matched, fall back to area/confidence thresholds
        let area_ratio = if other.bbox.area() > 0.0 {
            candidate.bbox.area() / other.bbox.area()
        } else {
            0.0
        };
        let conf_diff = other.confidence - candidate.confidence;

        if area_ratio <= params.area_threshold && conf_diff > params.conf_threshold {
            return false;
        }

        true // Default to keeping candidate if no rules triggered rejection
    }

    /// Ensure each cell appears only once, maintaining order of first appearance
    ///
    /// From ~/docling/docling/utils/layout_postprocessor.py:574-582
    fn deduplicate_cells(cells: Vec<TextCell>) -> Vec<TextCell> {
        let mut seen_ids = HashSet::new();
        let mut unique_cells = Vec::new();

        for cell in cells {
            // Use (text, bbox) as unique identifier
            // Convert floats to strings with fixed precision for hashing
            let cell_id = format!(
                "{}|{:.6}|{:.6}|{:.6}|{:.6}",
                cell.text, cell.bbox.l, cell.bbox.t, cell.bbox.r, cell.bbox.b
            );

            if seen_ids.insert(cell_id) {
                unique_cells.push(cell);
            }
        }

        unique_cells
    }

    /// Sort cells in native reading order
    ///
    /// From ~/docling/docling/utils/layout_postprocessor.py:653-655
    fn sort_cells(mut cells: Vec<TextCell>) -> Vec<TextCell> {
        // Sort by y-coordinate (top to bottom), then x-coordinate (left to right)
        cells.sort_by(|a, b| {
            a.bbox
                .t
                .partial_cmp(&b.bbox.t)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    a.bbox
                        .l
                        .partial_cmp(&b.bbox.l)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        });
        cells
    }

    /// Check if label represents a text element
    /// N=2281: Helper for adjacency merging logic
    #[inline]
    fn is_text_element(label: &str) -> bool {
        matches!(
            label,
            "text"
                | "section_header"
                | "caption"
                | "footnote"
                | "list_item"
                | "code"
                | "page_header"
                | "page_footer"
                | "checkbox_selected"
                | "checkbox_unselected"
                | "formula"
                | "Section-header"
                | "Caption"
                | "Footnote"
                | "List-item"
                | "Code"
                | "Page-header"
                | "Page-footer"
                | "Formula"
        )
    }
}

/// Cluster type for classification
enum ClusterType {
    Regular,
    Picture,
    Wrapper,
}

/// Parameters for selecting best cluster from group
struct ClusterParams {
    area_threshold: f64,
    conf_threshold: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_union_find() {
        let elements = vec![0, 1, 2, 3, 4];
        let mut uf = UnionFind::new(&elements);

        // Union 0 and 1
        uf.union(0, 1);
        assert_eq!(uf.find(0), uf.find(1));

        // Union 2 and 3
        uf.union(2, 3);
        assert_eq!(uf.find(2), uf.find(3));

        // Union 0 and 2 (merges two groups)
        uf.union(0, 2);
        assert_eq!(uf.find(0), uf.find(2));
        assert_eq!(uf.find(1), uf.find(3));

        // 4 should be alone
        assert_ne!(uf.find(4), uf.find(0));

        // Get groups
        let groups = uf.get_groups();
        assert_eq!(groups.len(), 2); // Two groups: {0,1,2,3} and {4}
    }

    #[test]
    fn test_check_overlap_iou() {
        let resolver = Stage08OverlapResolver::new();

        // Two overlapping boxes (50% overlap)
        let bbox1 = BBox::new(0.0, 0.0, 10.0, 10.0); // Area = 100
        let bbox2 = BBox::new(5.0, 0.0, 15.0, 10.0); // Area = 100
                                                     // Intersection = 50, Union = 150, IoU = 0.333

        // Should not overlap (IoU 0.333 < 0.8 threshold, containment 0.5 < 0.8)
        assert!(!resolver.check_overlap(&bbox1, &bbox2));
    }

    #[test]
    fn test_check_overlap_containment() {
        let resolver = Stage08OverlapResolver::new();

        // One box contains another (100% containment)
        let bbox1 = BBox::new(0.0, 0.0, 10.0, 10.0); // Area = 100
        let bbox2 = BBox::new(2.0, 2.0, 8.0, 8.0); // Area = 36, 100% inside bbox1
                                                   // Containment of bbox2 in bbox1 = 1.0 > 0.8

        // Should overlap (containment 1.0 > 0.8 threshold)
        assert!(resolver.check_overlap(&bbox1, &bbox2));
        assert!(resolver.check_overlap(&bbox2, &bbox1));
    }

    #[test]
    fn test_deduplicate_cells() {
        let cells = vec![
            TextCell {
                text: "Hello".to_string(),
                bbox: BBox::new(0.0, 0.0, 10.0, 10.0),
                confidence: Some(1.0),
                is_bold: false,
                is_italic: false,
            },
            TextCell {
                text: "Hello".to_string(),
                bbox: BBox::new(0.0, 0.0, 10.0, 10.0),
                confidence: Some(1.0),
                is_bold: false,
                is_italic: false,
            },
            TextCell {
                text: "World".to_string(),
                bbox: BBox::new(10.0, 10.0, 20.0, 20.0),
                confidence: Some(1.0),
                is_bold: false,
                is_italic: false,
            },
        ];

        let unique = Stage08OverlapResolver::deduplicate_cells(cells);
        assert_eq!(unique.len(), 2); // "Hello" should appear once
        assert_eq!(unique[0].text, "Hello");
        assert_eq!(unique[1].text, "World");
    }

    #[test]
    fn test_sort_cells() {
        let cells = vec![
            TextCell {
                text: "Bottom".to_string(),
                bbox: BBox::new(0.0, 20.0, 10.0, 30.0),
                confidence: Some(1.0),
                is_bold: false,
                is_italic: false,
            },
            TextCell {
                text: "Top Right".to_string(),
                bbox: BBox::new(10.0, 0.0, 20.0, 10.0),
                confidence: Some(1.0),
                is_bold: false,
                is_italic: false,
            },
            TextCell {
                text: "Top Left".to_string(),
                bbox: BBox::new(0.0, 0.0, 10.0, 10.0),
                confidence: Some(1.0),
                is_bold: false,
                is_italic: false,
            },
        ];

        let sorted = Stage08OverlapResolver::sort_cells(cells);
        assert_eq!(sorted[0].text, "Top Left"); // y=0, x=0
        assert_eq!(sorted[1].text, "Top Right"); // y=0, x=10
        assert_eq!(sorted[2].text, "Bottom"); // y=20
    }
}
