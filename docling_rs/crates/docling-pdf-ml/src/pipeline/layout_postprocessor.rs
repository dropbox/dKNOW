//! Layout post-processing stages 3-6
//!
//! These stages process layout clusters after initial filtering (stages 1-2):
//! - Stage 3: Assign text cells to clusters (20% overlap threshold)
//! - Stage 4: Remove empty clusters (clusters with no cells)
//! - Stage 5: Create orphan clusters (unassigned cells → TEXT clusters)
//! - Stage 6: Iterative refinement (bbox adjustment + overlap removal)
//!
//! Reference: ~/`docling/docling/utils/layout_postprocessor.py`
//!
//! Note: Some code in this module is infrastructure ported from Python that will be
//! wired up in future pipeline iterations. Allow `dead_code` for staged infrastructure.
#![allow(dead_code)]
// Intentional ML conversions: bounding box coordinates
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
// Pipeline stage functions take Vec ownership for data flow semantics
#![allow(clippy::needless_pass_by_value)]

use crate::pipeline::{BoundingBox, Cluster, DocItemLabel, SimpleTextCell, TextCell};
use log::trace;
use rstar::{RTree, AABB};
use std::collections::{HashMap, HashSet};
use std::time::Instant;

/// Union-Find (Disjoint Set Union) data structure with path compression and union by rank
///
/// Used in Stage 6 to group overlapping clusters into connected components.
/// Reference: ~/`docling/docling/utils/layout_postprocessor.py` (lines 16-46)
struct UnionFind {
    parent: HashMap<usize, usize>,
    rank: HashMap<usize, usize>,
}

impl UnionFind {
    /// Create a new `UnionFind` with given elements
    fn new(elements: impl Iterator<Item = usize>) -> Self {
        let elements_vec: Vec<usize> = elements.collect();
        let parent = elements_vec.iter().map(|&e| (e, e)).collect();
        let rank = elements_vec.iter().map(|&e| (e, 0)).collect();
        Self { parent, rank }
    }

    /// Find the root of element x with path compression
    fn find(&mut self, x: usize) -> usize {
        if self.parent[&x] != x {
            // Path compression: update parent to root
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

        // Union by rank: attach smaller tree under larger tree
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

    /// Get groups as `HashMap<root, Vec<elements>>`
    fn get_groups(&mut self) -> HashMap<usize, Vec<usize>> {
        let mut groups: HashMap<usize, Vec<usize>> = HashMap::new();
        let elements: Vec<usize> = self.parent.keys().copied().collect();

        for elem in elements {
            let root = self.find(elem);
            groups.entry(root).or_default().push(elem);
        }

        groups
    }
}

/// Interval for 1D range queries
#[derive(Debug, Clone, Copy, PartialEq)]
struct Interval {
    min_val: f32,
    max_val: f32,
    id: usize,
}

impl Eq for Interval {}

impl PartialOrd for Interval {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Interval {
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.min_val.total_cmp(&other.min_val)
    }
}

/// `IntervalTree` for efficient 1D range queries
///
/// Used in Stage 6 for spatial indexing along x and y axes.
/// Reference: ~/`docling/docling/utils/layout_postprocessor.py` (lines 107-152)
struct IntervalTree {
    intervals: Vec<Interval>,
}

impl IntervalTree {
    const fn new() -> Self {
        Self {
            intervals: Vec::new(),
        }
    }

    /// Insert an interval with binary insertion to maintain sorted order
    fn insert(&mut self, min_val: f32, max_val: f32, id: usize) {
        let interval = Interval {
            min_val,
            max_val,
            id,
        };
        let pos = self
            .intervals
            .binary_search(&interval)
            .unwrap_or_else(|e| e);
        self.intervals.insert(pos, interval);
    }

    /// Find all intervals containing the given point
    #[allow(dead_code, reason = "kept for potential future spatial queries")]
    fn find_containing(&self, point: f32) -> HashSet<usize> {
        let mut result = HashSet::new();

        // Binary search to find insertion point
        let pos = self
            .intervals
            .binary_search_by(|interval| interval.min_val.total_cmp(&point))
            .unwrap_or_else(|e| e);

        // Check intervals starting before point (reverse iteration)
        for interval in self.intervals[..pos].iter().rev() {
            if interval.min_val <= point && point <= interval.max_val {
                result.insert(interval.id);
            } else {
                break;
            }
        }

        // Check intervals starting at/after point (forward iteration)
        for interval in &self.intervals[pos..] {
            if point <= interval.max_val {
                if interval.min_val <= point {
                    result.insert(interval.id);
                }
            } else {
                break;
            }
        }

        result
    }
}

/// Envelope for R-tree spatial indexing
#[derive(Debug, Clone, Copy)]
struct ClusterEnvelope {
    aabb: AABB<[f32; 2]>,
    id: usize,
}

impl rstar::RTreeObject for ClusterEnvelope {
    type Envelope = AABB<[f32; 2]>;

    fn envelope(&self) -> Self::Envelope {
        self.aabb
    }
}

impl rstar::PointDistance for ClusterEnvelope {
    fn distance_2(&self, point: &[f32; 2]) -> f32 {
        self.aabb.distance_2(point)
    }
}

/// `SpatialClusterIndex` for efficient overlap detection
///
/// Uses R-tree for 2D spatial queries and interval trees for 1D queries.
/// Reference: ~/`docling/docling/utils/layout_postprocessor.py` (lines 49-105)
struct SpatialClusterIndex {
    spatial_index: RTree<ClusterEnvelope>,
    #[allow(dead_code, reason = "kept for potential future 1D spatial queries")]
    x_intervals: IntervalTree,
    #[allow(dead_code, reason = "kept for potential future 1D spatial queries")]
    y_intervals: IntervalTree,
    #[allow(dead_code, reason = "kept for potential future cluster lookup by ID")]
    clusters_by_id: HashMap<usize, BoundingBox>,
}

impl SpatialClusterIndex {
    /// Build spatial index from clusters
    fn new(clusters: &[Cluster]) -> Self {
        let mut spatial_index_data = Vec::new();
        let mut x_intervals = IntervalTree::new();
        let mut y_intervals = IntervalTree::new();
        let mut clusters_by_id = HashMap::new();

        for cluster in clusters {
            let bbox = &cluster.bbox;
            let id = cluster.id;

            // Store bbox
            clusters_by_id.insert(id, *bbox);

            // Add to R-tree
            let aabb = AABB::from_corners([bbox.l, bbox.t], [bbox.r, bbox.b]);
            spatial_index_data.push(ClusterEnvelope { aabb, id });

            // Add to interval trees
            x_intervals.insert(bbox.l, bbox.r, id);
            y_intervals.insert(bbox.t, bbox.b, id);
        }

        let spatial_index = RTree::bulk_load(spatial_index_data);

        Self {
            spatial_index,
            x_intervals,
            y_intervals,
            clusters_by_id,
        }
    }

    /// Find candidate clusters that might overlap with given bbox
    ///
    /// Returns cluster IDs that spatially intersect with bbox.
    fn find_candidates(&self, bbox: &BoundingBox) -> HashSet<usize> {
        let aabb = AABB::from_corners([bbox.l, bbox.t], [bbox.r, bbox.b]);
        self.spatial_index
            .locate_in_envelope_intersecting(&aabb)
            .map(|envelope| envelope.id)
            .collect()
    }

    /// Check if two bboxes overlap sufficiently
    ///
    /// Overlap detected if ANY of these conditions are true:
    /// - `IoU` > `overlap_threshold` (0.8)
    /// - containment1 > `containment_threshold` (0.8)
    /// - containment2 > `containment_threshold` (0.8)
    ///
    /// Reference: ~/`docling/docling/utils/layout_postprocessor.py` (lines 85-104)
    fn check_overlap(
        bbox1: &BoundingBox,
        bbox2: &BoundingBox,
        overlap_threshold: f32,
        containment_threshold: f32,
    ) -> bool {
        if bbox1.area() <= 0.0 || bbox2.area() <= 0.0 {
            return false;
        }

        let iou = bbox1.intersection_over_union(bbox2);
        let containment1 = bbox1.intersection_over_self(bbox2);
        let containment2 = bbox2.intersection_over_self(bbox1);

        iou > overlap_threshold
            || containment1 > containment_threshold
            || containment2 > containment_threshold
    }
}

/// Overlap parameters for different cluster types
///
/// Reference: ~/`docling/docling/utils/layout_postprocessor.py` (lines 158-162)
#[derive(Debug, Clone, Copy)]
struct OverlapParams {
    /// Area threshold for best cluster selection (default: 1.3 for regular, 2.0 for picture/wrapper)
    area_threshold: f32,
    /// Confidence threshold for best cluster selection (default: 0.05 for regular)
    conf_threshold: f32,
    /// `IoU` threshold for overlap detection (default: 0.8)
    overlap_threshold: f32,
    /// Containment threshold for overlap detection and preference rules (default: 0.8)
    containment_threshold: f32,
    /// Max horizontal gap as multiplier of avg height for adjacency merge (default: 1.5)
    horizontal_gap_multiplier: f32,
    /// Vertical alignment tolerance as multiplier of avg height (default: 0.3)
    vertical_align_multiplier: f32,
    /// Minimum vertical overlap ratio for adjacency merge (default: 0.5)
    vertical_overlap_threshold: f32,
    /// Area similarity threshold for `LIST_ITEM` vs `TEXT` preference rule (default: 0.2)
    area_similarity_threshold: f32,
}

impl OverlapParams {
    const REGULAR: Self = Self {
        area_threshold: 1.3,
        conf_threshold: 0.05,
        overlap_threshold: 0.8,
        containment_threshold: 0.8,
        horizontal_gap_multiplier: 1.5,
        vertical_align_multiplier: 0.3,
        vertical_overlap_threshold: 0.5,
        area_similarity_threshold: 0.2,
    };

    const PICTURE: Self = Self {
        area_threshold: 2.0,
        conf_threshold: 0.3,
        overlap_threshold: 0.8,
        containment_threshold: 0.8,
        // Not used for picture clusters (not text elements)
        horizontal_gap_multiplier: 0.0,
        vertical_align_multiplier: 0.0,
        vertical_overlap_threshold: 0.0,
        area_similarity_threshold: 0.2,
    };

    const WRAPPER: Self = Self {
        area_threshold: 2.0,
        conf_threshold: 0.2,
        overlap_threshold: 0.8,
        containment_threshold: 0.8,
        // Not used for wrapper clusters (not text elements)
        horizontal_gap_multiplier: 0.0,
        vertical_align_multiplier: 0.0,
        vertical_overlap_threshold: 0.0,
        area_similarity_threshold: 0.2,
    };
}

/// Post-processor configuration
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PostProcessorConfig {
    /// Minimum overlap ratio for cell assignment (default: 0.2 = 20%)
    pub min_cell_overlap: f32,
    /// Whether to keep empty clusters (clusters with no cells)
    pub keep_empty_clusters: bool,
    /// Whether to skip cell assignment
    pub skip_cell_assignment: bool,
    /// Containment threshold for child assignment to special clusters (default: 0.8)
    pub child_containment_threshold: f32,
    /// Confidence threshold for high-precision labels (default: 0.45)
    /// Used for: `SectionHeader`, `Title`, `Code`, `CheckboxSelected`, `CheckboxUnselected`, `Form`, `KeyValueRegion`, `DocumentIndex`
    pub high_precision_threshold: f32,
    /// Confidence threshold for standard labels (default: 0.5)
    /// Used for: `Caption`, `Footnote`, `Formula`, `ListItem`, `PageFooter`, `PageHeader`, `Picture`, `Table`, `Text`, `Figure`
    pub standard_threshold: f32,
}

impl Default for PostProcessorConfig {
    #[inline]
    fn default() -> Self {
        Self {
            min_cell_overlap: 0.2,
            keep_empty_clusters: false,
            skip_cell_assignment: false,
            child_containment_threshold: 0.8,
            high_precision_threshold: 0.45,
            standard_threshold: 0.5,
        }
    }
}

/// Output of Stage 3.1 (after empty removal)
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Stage31Output {
    pub regular_clusters: Vec<Cluster>,
    pub special_clusters: Vec<Cluster>,
}

/// Output of Stage 3.2 (after orphan creation)
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Stage32Output {
    pub regular_clusters: Vec<Cluster>,
    pub special_clusters: Vec<Cluster>,
    pub orphan_count: usize,
}

/// Output of Stage 3.3 (after bbox adjustment)
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Stage33Output {
    pub regular_clusters: Vec<Cluster>,
    pub special_clusters: Vec<Cluster>,
}

/// Layout post-processor
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct LayoutPostProcessor {
    config: PostProcessorConfig,
}

/// Helper to run a timed operation with micro-profiling
fn timed_operation<T, F: FnOnce() -> T>(micro_profile: bool, label: &str, f: F) -> T {
    if micro_profile {
        let t_start = Instant::now();
        let result = f();
        log::warn!(
            "[MICRO] {}: {:.3} ms",
            label,
            t_start.elapsed().as_secs_f64() * 1000.0
        );
        result
    } else {
        f()
    }
}

/// Get minimum cell index for a cluster (for sorting)
fn min_cell_index(cluster: &Cluster) -> usize {
    if cluster.cells.is_empty() {
        usize::MAX
    } else {
        cluster
            .cells
            .iter()
            .map(|cell| cell.index)
            .min()
            .unwrap_or(usize::MAX)
    }
}

impl LayoutPostProcessor {
    /// Create new post-processor with configuration
    #[must_use = "returns a new LayoutPostProcessor instance"]
    pub const fn new(config: PostProcessorConfig) -> Self {
        Self { config }
    }

    /// Create new post-processor with default configuration
    #[must_use = "returns a new LayoutPostProcessor with default config"]
    pub fn new_default() -> Self {
        Self::new(PostProcessorConfig::default())
    }

    /// Sort clusters by reading order (min cell index, then bbox position)
    fn sort_clusters_by_reading_order(clusters: &mut [Cluster]) {
        clusters.sort_by(|a, b| {
            let a_min_idx = min_cell_index(a);
            let b_min_idx = min_cell_index(b);

            // Primary: sort by minimum cell index
            // Tie-breaker 1: sort by bbox.t (top position)
            // Tie-breaker 2: sort by bbox.l (left position)
            a_min_idx
                .cmp(&b_min_idx)
                .then_with(|| {
                    a.bbox
                        .t
                        .partial_cmp(&b.bbox.t)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .then_with(|| {
                    a.bbox
                        .l
                        .partial_cmp(&b.bbox.l)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        });
    }

    /// Debug log cluster info before sorting (for `code_and_formula` test)
    fn debug_log_clusters_before_sort(clusters: &[Cluster]) {
        if clusters.len() == 8 && clusters.iter().any(|c| c.id == 6 || c.id == 7) {
            log::debug!("[POSTPROC DEBUG] Before sorting:");
            for c in clusters {
                log::debug!(
                    "  Cluster ID={}: min_cell_idx={}, bbox.t={:.3}, bbox.l={:.3}, label={:?}",
                    c.id,
                    min_cell_index(c),
                    c.bbox.t,
                    c.bbox.l,
                    c.label
                );
            }
        }
    }

    /// Debug log cluster IDs after sorting
    fn debug_log_clusters_after_sort(clusters: &[Cluster]) {
        if clusters.len() == 8 && clusters.iter().any(|c| c.id == 6 || c.id == 7) {
            log::debug!("[POSTPROC DEBUG] After sorting:");
            log::debug!(
                "  Cluster IDs: {:?}",
                clusters.iter().map(|c| c.id).collect::<Vec<_>>()
            );
        }
    }

    /// Debug log special cluster children
    fn debug_log_special_children(special_clusters: &[Cluster]) {
        for special in special_clusters {
            if !special.children.is_empty() {
                let child_ids: Vec<usize> = special.children.iter().map(|c| c.id).collect();
                log::debug!(
                    "    Special cluster ID={}, Label={:?} has {} children: {:?}",
                    special.id,
                    special.label,
                    special.children.len(),
                    child_ids
                );
            }
        }
    }

    /// N=4373: NO LONGER remapping Title→SectionHeader
    ///
    /// Previously converted all Title labels to SectionHeader, which caused all
    /// headers to render as H2. Now we preserve Title labels so that
    /// `detect_header_level` can return level 0 for titles (rendering as H1).
    fn apply_label_remapping(clusters: Vec<Cluster>) -> Vec<Cluster> {
        // N=4373: Keep Title labels - they should render as H1
        // detect_header_level() handles: Title→level 0 (H1), SectionHeader→level 1+ (H2+)
        clusters
    }

    /// Filter clusters by confidence threshold
    fn filter_by_confidence(&self, clusters: Vec<Cluster>) -> Vec<Cluster> {
        clusters
            .into_iter()
            .filter(|c| c.confidence >= self.get_confidence_threshold(c.label))
            .collect()
    }

    /// Get confidence threshold for a label
    /// Reference: ~/docling/docling/utils/layout_postprocessor.py:172-188
    fn get_confidence_threshold(&self, label: DocItemLabel) -> f32 {
        // N=3613: Use DEBUG_LOW_THRESHOLD to see all detections
        if std::env::var("DEBUG_LOW_THRESHOLD").is_ok() {
            return 0.01; // Very low threshold for debugging
        }

        match label {
            // Higher precision labels use high_precision_threshold (default: 0.45)
            DocItemLabel::SectionHeader
            | DocItemLabel::Title
            | DocItemLabel::Code
            | DocItemLabel::CheckboxSelected
            | DocItemLabel::CheckboxUnselected
            | DocItemLabel::Form
            | DocItemLabel::KeyValueRegion
            | DocItemLabel::DocumentIndex => self.config.high_precision_threshold,
            // Standard labels use standard_threshold (default: 0.5)
            DocItemLabel::Caption
            | DocItemLabel::Footnote
            | DocItemLabel::Formula
            | DocItemLabel::ListItem
            | DocItemLabel::PageFooter
            | DocItemLabel::PageHeader
            | DocItemLabel::Picture
            | DocItemLabel::Table
            | DocItemLabel::Text
            | DocItemLabel::Figure => self.config.standard_threshold,
        }
    }

    /// Run ONLY Stage 3.0 (cell assignment)
    ///
    /// Returns: (`regular_clusters`, `special_clusters`) after cell assignment
    /// Includes empty clusters (not filtered yet)
    #[must_use = "returns stage output with assigned clusters"]
    pub fn run_stage30_only(
        &self,
        clusters: Vec<Cluster>,
        cells: Vec<SimpleTextCell>,
    ) -> Stage31Output {
        // Apply label remapping
        let all_clusters: Vec<Cluster> = clusters
            .into_iter()
            .map(|mut c| {
                if c.label == DocItemLabel::Title {
                    c.label = DocItemLabel::SectionHeader;
                }
                c
            })
            .collect();

        // Split into regular and special
        let (regular_clusters, special_clusters) = Self::split_regular_special(all_clusters);

        // Confidence filter
        let regular_filtered: Vec<Cluster> = regular_clusters
            .into_iter()
            .filter(|c| c.confidence >= self.get_confidence_threshold(c.label))
            .collect();

        let special_filtered: Vec<Cluster> = special_clusters
            .into_iter()
            .filter(|c| c.confidence >= self.get_confidence_threshold(c.label))
            .collect();

        // Assign cells
        let mut all_filtered = regular_filtered.clone();
        all_filtered.extend(special_filtered);

        self.assign_cells_to_clusters(&mut all_filtered, &cells);

        // Deduplicate cells (happens in assign_cells_to_clusters but need to ensure)
        for cluster in &mut all_filtered {
            cluster.cells = Self::deduplicate_cells(std::mem::take(&mut cluster.cells));
        }

        // Split back (INCLUDE EMPTY CLUSTERS - no filtering yet)
        let num_regular = regular_filtered.len();
        let regular_clusters = all_filtered[..num_regular].to_vec();
        let special_clusters = all_filtered[num_regular..].to_vec();

        Stage31Output {
            regular_clusters,
            special_clusters,
        }
    }

    /// Run Stage 3.4 only (one iteration of overlap resolution) on regular clusters
    #[must_use = "returns merged clusters after overlap resolution"]
    pub fn run_stage34_one_iter(clusters: Vec<Cluster>) -> Vec<Cluster> {
        Self::remove_overlapping_clusters(clusters, OverlapParams::REGULAR)
    }

    /// Run Stages 3.0-3.4 (one iteration of overlap resolution)
    #[must_use = "returns processed clusters after all stages"]
    pub fn run_stage30_31_32_33_34_one_iter(
        &self,
        clusters: Vec<Cluster>,
        cells: Vec<SimpleTextCell>,
    ) -> Vec<Cluster> {
        // Run Stages 3.0-3.3
        let stage33 = self.run_stage30_31_32_33(clusters, cells);

        // Stage 3.4: One iteration of overlap resolution (regular clusters only)
        let merged_regular =
            Self::remove_overlapping_clusters(stage33.regular_clusters, OverlapParams::REGULAR);

        // Combine regular and special clusters
        // CRITICAL: Must return BOTH regular and special clusters
        // Special clusters (Picture, Table, Form, KeyValueRegion) are NOT merged in stage 3.4
        let mut all_clusters = merged_regular;
        all_clusters.extend(stage33.special_clusters);

        all_clusters
    }

    /// Run Stages 3.0-3.3 (through bbox adjustment, one iteration only)
    #[must_use = "returns stage output with adjusted bboxes"]
    pub fn run_stage30_31_32_33(
        &self,
        clusters: Vec<Cluster>,
        cells: Vec<SimpleTextCell>,
    ) -> Stage33Output {
        // Run Stages 3.0-3.2
        let stage32 = self.run_stage30_31_32(clusters, cells);

        // Stage 3.3: Adjust bboxes for BOTH regular and special clusters
        let adjusted_regular = Self::adjust_cluster_bboxes(stage32.regular_clusters);
        let adjusted_special = Self::adjust_cluster_bboxes(stage32.special_clusters);

        Stage33Output {
            regular_clusters: adjusted_regular,
            special_clusters: adjusted_special,
        }
    }

    /// Run Stages 3.0-3.2 (cell assignment + empty removal + orphan creation)
    #[must_use = "returns stage output with orphan clusters created"]
    pub fn run_stage30_31_32(
        &self,
        clusters: Vec<Cluster>,
        cells: Vec<SimpleTextCell>,
    ) -> Stage32Output {
        // Calculate max input ID for orphan creation BEFORE moving clusters
        let max_input_id = clusters.iter().map(|c| c.id).max().unwrap_or(0);

        // Run Stage 3.0 + 3.1
        let stage31 = self.run_stage30_and_31_only(clusters, cells.clone());

        // Stage 3.2: Create orphan clusters
        let orphan_clusters = self.create_orphan_clusters_with_special(
            &stage31.regular_clusters,
            &stage31.special_clusters,
            &cells,
            max_input_id,
        );

        let orphan_count = orphan_clusters.len();
        let mut regular_with_orphans = stage31.regular_clusters;
        regular_with_orphans.extend(orphan_clusters);

        Stage32Output {
            regular_clusters: regular_with_orphans,
            special_clusters: stage31.special_clusters,
            orphan_count,
        }
    }

    /// Run ONLY Stage 3.0 + 3.1 (cell assignment + empty removal)
    ///
    /// Returns: (`regular_clusters`, `special_clusters`) after empty removal
    #[must_use = "returns stage output with empty clusters removed"]
    pub fn run_stage30_and_31_only(
        &self,
        clusters: Vec<Cluster>,
        cells: Vec<SimpleTextCell>,
    ) -> Stage31Output {
        // Apply label remapping
        let all_clusters: Vec<Cluster> = clusters
            .into_iter()
            .map(|mut c| {
                if c.label == DocItemLabel::Title {
                    c.label = DocItemLabel::SectionHeader;
                }
                c
            })
            .collect();

        // Split into regular and special
        let (regular_clusters, special_clusters) = Self::split_regular_special(all_clusters);

        // Confidence filter
        let regular_filtered: Vec<Cluster> = regular_clusters
            .into_iter()
            .filter(|c| c.confidence >= self.get_confidence_threshold(c.label))
            .collect();

        let special_filtered: Vec<Cluster> = special_clusters
            .into_iter()
            .filter(|c| c.confidence >= self.get_confidence_threshold(c.label))
            .collect();

        // Assign cells
        let mut all_filtered = regular_filtered.clone();
        all_filtered.extend(special_filtered);

        self.assign_cells_to_clusters(&mut all_filtered, &cells);

        // Split back
        let num_regular = regular_filtered.len();
        let regular_clusters = all_filtered[..num_regular].to_vec();
        let special_clusters = all_filtered[num_regular..].to_vec();

        // Stage 3.1: Remove empty clusters from regular only
        let regular_non_empty: Vec<Cluster> = regular_clusters
            .into_iter()
            .filter(|c| !c.cells.is_empty() || c.label == DocItemLabel::Formula)
            .collect();

        Stage31Output {
            regular_clusters: regular_non_empty,
            special_clusters,
        }
    }

    /// Process layout clusters (stages 1-6)
    ///
    /// # Arguments
    /// * `clusters` - Layout clusters from layout predictor
    /// * `cells` - Text cells from OCR/programmatic extraction
    ///
    /// # Returns
    /// Processed clusters with cells assigned
    #[must_use = "returns processed clusters that should be used"]
    pub fn process(&self, clusters: Vec<Cluster>, cells: Vec<SimpleTextCell>) -> Vec<Cluster> {
        trace!(
            "[N=2281 PROCESS] LayoutPostProcessor::process() called with {} clusters",
            clusters.len()
        );
        let micro_profile = std::env::var("MICRO_PROFILE").is_ok();

        if self.config.skip_cell_assignment {
            trace!("[N=2281 PROCESS] Skipping cell assignment!");
            return clusters;
        }

        // Calculate max cluster ID from ALL input clusters (before filtering)
        let max_input_id = clusters.iter().map(|c| c.id).max().unwrap_or(0);

        // STEP 1: Apply label remapping: TITLE → SECTION_HEADER
        let all_clusters = timed_operation(micro_profile, "Label remapping", || {
            Self::apply_label_remapping(clusters)
        });

        // STEP 2: Split into regular and special clusters
        let (regular_clusters, special_clusters) =
            timed_operation(micro_profile, "Split regular/special", || {
                Self::split_regular_special(all_clusters)
            });
        log::debug!(
            "    Split: {} regular, {} special clusters",
            regular_clusters.len(),
            special_clusters.len()
        );

        // STEP 3: Confidence filter before cell assignment (matches Python!)
        let (regular_filtered, special_filtered) =
            timed_operation(micro_profile, "Confidence filtering", || {
                (
                    self.filter_by_confidence(regular_clusters),
                    self.filter_by_confidence(special_clusters),
                )
            });
        log::debug!(
            "    After confidence filter: {} regular, {} special",
            regular_filtered.len(),
            special_filtered.len()
        );

        // STEP 4: Assign cells to REGULAR clusters ONLY
        let mut regular_filtered = regular_filtered;
        timed_operation(micro_profile, "Assign cells", || {
            self.assign_cells_to_clusters(&mut regular_filtered, &cells);
        });

        let total_cells_assigned: usize = regular_filtered.iter().map(|c| c.cells.len()).sum();
        log::debug!(
            "    Stage 3.0 complete: Assigned {} cells to {} regular clusters",
            total_cells_assigned,
            regular_filtered.len()
        );

        // STEP 5: Process regular clusters (empty removal, orphan creation, refinement)
        let regular_count = regular_filtered.len();
        let mut regular_clusters =
            timed_operation(micro_profile, "Process regular clusters", || {
                self.process_regular_clusters(
                    regular_filtered,
                    &special_filtered,
                    &cells,
                    regular_count,
                    max_input_id,
                )
            });

        // STEP 6: Process special clusters (Picture, Table, Form, etc.)
        let special_clusters = timed_operation(micro_profile, "Process special clusters", || {
            self.process_special_clusters(special_filtered, &regular_clusters)
        });
        log::debug!(
            "    Special: After processing: {} special clusters",
            special_clusters.len()
        );

        // STEP 7: Remove regular clusters that are children of special clusters
        let contained_ids: HashSet<usize> = special_clusters
            .iter()
            .flat_map(|s| s.children.iter().map(|c| c.id))
            .collect();
        Self::debug_log_special_children(&special_clusters);
        regular_clusters.retain(|c| !contained_ids.contains(&c.id));
        log::debug!(
            "    Removed {} regular clusters (children of special): {:?}",
            contained_ids.len(),
            contained_ids
        );

        // DEBUG: Check labels before combine
        log::debug!("[LABEL DEBUG] Regular clusters before combining:");
        for c in regular_clusters.iter().take(15) {
            log::debug!("  ID={}, label={:?}", c.id, c.label);
        }

        // STEP 8: Combine regular and special clusters
        regular_clusters.extend(special_clusters);

        // DEBUG: Check labels after combine
        log::debug!("[LABEL DEBUG] All clusters after combining:");
        for c in regular_clusters.iter().take(15) {
            log::debug!("  ID={}, label={:?}", c.id, c.label);
        }

        // STEP 9: Sort by reading order (min cell index, then bbox position)
        Self::debug_log_clusters_before_sort(&regular_clusters);
        Self::sort_clusters_by_reading_order(&mut regular_clusters);
        Self::debug_log_clusters_after_sort(&regular_clusters);

        regular_clusters
    }

    /// Split clusters into regular and special types
    ///
    /// Special types: Picture, Table, Form, `KeyValueRegion`
    /// Regular types: everything else
    ///
    /// Reference: ~/`docling/docling/utils/layout_postprocessor.py` (lines 207-210)
    /// Note: Python includes `DocumentIndex` in special types, but it's not in Rust enum yet
    fn split_regular_special(clusters: Vec<Cluster>) -> (Vec<Cluster>, Vec<Cluster>) {
        // Python: SPECIAL_TYPES = WRAPPER_TYPES + {PICTURE}
        // WRAPPER_TYPES = {FORM, KEY_VALUE_REGION, TABLE, DOCUMENT_INDEX}
        // Reference: ~/docling/docling/utils/layout_postprocessor.py:166-171
        let (special, regular): (Vec<Cluster>, Vec<Cluster>) =
            clusters.into_iter().partition(|c| {
                matches!(
                    c.label,
                    DocItemLabel::Picture
                        | DocItemLabel::Table
                        | DocItemLabel::Form
                        | DocItemLabel::KeyValueRegion
                        | DocItemLabel::DocumentIndex
                )
            });
        (regular, special)
    }

    /// Process regular clusters with iterative refinement
    ///
    /// NOTE: Cell assignment is done BEFORE splitting in `process()`, so clusters
    /// already have cells assigned here.
    ///
    /// Reference: ~/`docling/docling/utils/layout_postprocessor.py` (lines 256-311)
    fn process_regular_clusters(
        &self,
        mut clusters: Vec<Cluster>,
        special_clusters: &[Cluster],
        cells: &[SimpleTextCell],
        initial_count: usize,
        max_input_id: usize,
    ) -> Vec<Cluster> {
        let micro_profile = std::env::var("MICRO_PROFILE").is_ok();

        // Cell assignment already done in process() - just log
        let cells_assigned: usize = clusters.iter().map(|c| c.cells.len()).sum();
        log::debug!(
            "    Regular: {} cells already assigned to {} regular clusters",
            cells_assigned,
            clusters.len()
        );

        // Stage 4: Remove empty clusters
        if !self.config.keep_empty_clusters {
            let before_ids: Vec<usize> = clusters.iter().map(|c| c.id).collect();
            clusters = timed_operation(micro_profile, "  Remove empty clusters", || {
                self.remove_empty_clusters(clusters)
            });
            let after_ids: Vec<usize> = clusters.iter().map(|c| c.id).collect();
            log::debug!(
                "    Stage 4: Removed empty regular clusters: {} → {} clusters",
                initial_count,
                clusters.len()
            );

            // Log removed IDs
            let removed_ids: Vec<usize> = before_ids
                .iter()
                .filter(|id| !after_ids.contains(id))
                .copied()
                .collect();
            log::debug!(
                "    Stage 4: Removed {} regular cluster IDs (empty): {:?}",
                removed_ids.len(),
                removed_ids
            );
        }

        // Stage 5: Create orphan clusters (unassigned cells → TEXT clusters)
        let orphan_clusters = timed_operation(micro_profile, "  Create orphan clusters", || {
            self.create_orphan_clusters_with_special(
                &clusters,
                special_clusters,
                cells,
                max_input_id,
            )
        });
        log::debug!(
            "    Stage 5: Created {} orphan clusters",
            orphan_clusters.len()
        );
        clusters.extend(orphan_clusters);

        // Stage 6: Iterative refinement (bbox adjustment + overlap removal)
        clusters = timed_operation(micro_profile, "  Iterative refinement", || {
            self.iterative_refinement(clusters)
        });
        log::debug!(
            "    Stage 6: Iterative refinement complete: {} regular clusters",
            clusters.len()
        );

        clusters
    }

    /// Process special clusters (Picture, Table, Form, `KeyValueRegion`, `DocumentIndex`)
    ///
    /// Reference: ~/`docling/docling/utils/layout_postprocessor.py` (lines 313-381)
    // Method signature kept for API consistency with other LayoutPostprocessor methods
    #[allow(clippy::unused_self)]
    #[allow(clippy::too_many_lines)]
    fn process_special_clusters(
        &self,
        special_clusters: Vec<Cluster>,
        regular_clusters: &[Cluster],
    ) -> Vec<Cluster> {
        // DEBUG: Show special clusters BEFORE confidence filtering
        let total_cells_before = special_clusters
            .iter()
            .map(|c| c.cells.len())
            .sum::<usize>();
        log::debug!(
            "    Special: BEFORE confidence filtering: {} clusters, {} cells",
            special_clusters.len(),
            total_cells_before
        );
        for c in &special_clusters {
            if !c.cells.is_empty() {
                log::debug!(
                    "      ID={}, label={:?}, conf={:.4}, cells={}, threshold={:.2}",
                    c.id,
                    c.label,
                    c.confidence,
                    c.cells.len(),
                    self.get_confidence_threshold(c.label)
                );
            }
        }

        let special_clusters: Vec<Cluster> = special_clusters
            .into_iter()
            .filter(|c| c.confidence >= self.get_confidence_threshold(c.label))
            .collect();

        let total_cells_after = special_clusters
            .iter()
            .map(|c| c.cells.len())
            .sum::<usize>();
        log::debug!(
            "    Special: After confidence filtering: {} special clusters, {} cells (lost {} cells)",
            special_clusters.len(),
            total_cells_after,
            total_cells_before - total_cells_after
        );

        // Handle cross-type overlaps (KVR vs TABLE)
        // Python: ~/docling/docling/utils/layout_postprocessor.py:383-409
        // NOTE: This Python function checks special clusters against regular TABLEs
        // In edinet page 0, both TABLE and KVR are special clusters, so this won't affect them
        // Skipping for now - overlap resolution will handle it

        // Filter out full-page pictures (>90% of page area)
        // Python: lines 322-334
        // NOTE: Page size check skipped for now - implement if needed

        // CRITICAL: Assign children to special clusters BEFORE overlap removal
        // Python: lines 336-365 (child assignment)
        // Then: lines 367-377 (overlap removal with adjusted bboxes)
        // This matters because KVR bbox gets expanded to fit children, affecting merge decisions
        let mut special_clusters_with_children = Vec::new();
        for mut special in special_clusters {
            let mut contained = Vec::new();
            for regular in regular_clusters {
                let containment = regular.bbox.intersection_over_self(&special.bbox);
                if containment > self.config.child_containment_threshold {
                    contained.push(regular.clone());
                }
            }

            if !contained.is_empty() {
                // Sort by cell index
                contained.sort_by(|a, b| {
                    let a_min = a.cells.iter().map(|c| c.index).min().unwrap_or(usize::MAX);
                    let b_min = b.cells.iter().map(|c| c.index).min().unwrap_or(usize::MAX);
                    a_min
                        .cmp(&b_min)
                        .then_with(|| {
                            a.bbox
                                .t
                                .partial_cmp(&b.bbox.t)
                                .unwrap_or(std::cmp::Ordering::Equal)
                        })
                        .then_with(|| {
                            a.bbox
                                .l
                                .partial_cmp(&b.bbox.l)
                                .unwrap_or(std::cmp::Ordering::Equal)
                        })
                });
                special.children.clone_from(&contained);

                // Adjust bbox for FORM and KEY_VALUE_REGION (NOT Table or Picture)
                if matches!(
                    special.label,
                    DocItemLabel::Form | DocItemLabel::KeyValueRegion
                ) && !contained.is_empty()
                {
                    let min_l = contained
                        .iter()
                        .map(|c| c.bbox.l)
                        .min_by(f32::total_cmp)
                        .unwrap();
                    let min_t = contained
                        .iter()
                        .map(|c| c.bbox.t)
                        .min_by(f32::total_cmp)
                        .unwrap();
                    let max_r = contained
                        .iter()
                        .map(|c| c.bbox.r)
                        .max_by(f32::total_cmp)
                        .unwrap();
                    let max_b = contained
                        .iter()
                        .map(|c| c.bbox.b)
                        .max_by(f32::total_cmp)
                        .unwrap();
                    special.bbox = BoundingBox {
                        l: min_l,
                        t: min_t,
                        r: max_r,
                        b: max_b,
                        coord_origin: special.bbox.coord_origin,
                    };
                }

                // Collect cells from children
                let all_cells: Vec<TextCell> =
                    contained.iter().flat_map(|c| c.cells.clone()).collect();
                special.cells = Self::deduplicate_cells(all_cells);
                special.cells.sort_by_key(|c| c.index);
            }

            special_clusters_with_children.push(special);
        }

        // NOW split into picture and wrapper for overlap removal
        // Overlap removal uses the ADJUSTED bboxes (KVR expanded to children)
        let (picture_clusters, wrapper_clusters): (Vec<Cluster>, Vec<Cluster>) =
            special_clusters_with_children
                .into_iter()
                .partition(|c| c.label == DocItemLabel::Picture);

        // Debug: Show picture clusters BEFORE overlap removal
        log::debug!("    Special: Picture clusters before overlap removal:");
        for pic in &picture_clusters {
            log::debug!(
                "      Picture ID={}, conf={:.4}, cells={}, bbox=({:.1},{:.1},{:.1},{:.1})",
                pic.id,
                pic.confidence,
                pic.cells.len(),
                pic.bbox.l,
                pic.bbox.t,
                pic.bbox.r,
                pic.bbox.b
            );
        }

        // CRITICAL: Do overlap removal FIRST, which merges cells from overlapping pictures
        // Python does NOT remove empty pictures - they stay in output even with 0 cells
        // Reference: ~/docling/docling/utils/layout_postprocessor.py:367-377
        // Python simply returns picture_clusters + wrapper_clusters with no empty filtering
        let picture_clusters =
            Self::remove_overlapping_clusters(picture_clusters, OverlapParams::PICTURE);
        log::debug!(
            "    Special: After Picture overlap removal: {} Picture clusters, {} cells",
            picture_clusters.len(),
            picture_clusters
                .iter()
                .map(|c| c.cells.len())
                .sum::<usize>()
        );

        // Debug: print KVR bboxes BEFORE overlap removal
        for w in &wrapper_clusters {
            if w.label == DocItemLabel::KeyValueRegion {
                log::debug!(
                    "      [BEFORE overlap removal] KVR ID={}, bbox={:?}, cells={}",
                    w.id,
                    w.bbox,
                    w.cells.len()
                );
            }
        }

        // Debug: count KVRs BEFORE overlap removal
        let kvr_count_before = wrapper_clusters
            .iter()
            .filter(|c| c.label == DocItemLabel::KeyValueRegion)
            .count();
        log::debug!("    Special: Before Wrapper overlap removal: {kvr_count_before} KVRs");

        // Process Wrapper clusters (Table, Form, etc.) with WRAPPER overlap params
        let wrapper_clusters =
            Self::remove_overlapping_clusters(wrapper_clusters, OverlapParams::WRAPPER);
        log::debug!(
            "    Special: After Wrapper overlap removal: {} Wrapper clusters",
            wrapper_clusters.len()
        );

        // Debug: count KVRs AFTER overlap removal
        let kvr_count_after = wrapper_clusters
            .iter()
            .filter(|c| c.label == DocItemLabel::KeyValueRegion)
            .count();
        log::debug!("    Special: After Wrapper overlap removal: {kvr_count_after} KVRs");

        // Debug: print KVR bboxes AFTER overlap removal
        for w in &wrapper_clusters {
            if w.label == DocItemLabel::KeyValueRegion {
                log::debug!(
                    "      [AFTER overlap removal] KVR ID={}, bbox={:?}, cells={}",
                    w.id,
                    w.bbox,
                    w.cells.len()
                );
            }
        }

        // Combine picture and wrapper clusters (now with children and adjusted bboxes)
        let mut all_special = picture_clusters;
        all_special.extend(wrapper_clusters);

        // Note: Children were already assigned BEFORE overlap removal
        // Cells and bboxes are already correct
        // No need for duplicate child assignment here

        all_special
    }

    /// Stage 3: Assign cells to best overlapping cluster
    ///
    /// For each cell, find the cluster with the highest overlap ratio.
    /// If overlap >= `min_overlap` (20%), assign cell to that cluster.
    pub fn assign_cells_to_clusters(&self, clusters: &mut [Cluster], cells: &[SimpleTextCell]) {
        #[cfg(feature = "debug-trace")]
        let debug_picture_clusters = std::env::var("DEBUG_CELL_ASSIGNMENT").is_ok();

        #[cfg(not(feature = "debug-trace"))]
        let debug_picture_clusters = false;

        #[cfg(feature = "debug-trace")]
        {
            if debug_picture_clusters {
                let cluster_ids: Vec<usize> = clusters.iter().map(|c| c.id).collect();
                log::debug!(
                    "[Rust] Cluster iteration order ({} clusters): {:?}",
                    clusters.len(),
                    &cluster_ids[..20.min(cluster_ids.len())]
                );
            }
        }

        // Clear existing cells
        for cluster in &mut *clusters {
            cluster.cells.clear();
        }

        // Assign each cell to best overlapping cluster
        for (cell_idx, cell) in cells.iter().enumerate() {
            let cell_bbox = cell.bbox();

            if cell.text.trim().is_empty() {
                continue;
            }

            if cell_bbox.area() <= 0.0 {
                continue;
            }

            let mut best_overlap = self.config.min_cell_overlap;
            let mut best_cluster_idx: Option<usize> = None;

            if debug_picture_clusters && cell_idx < 100 {
                let preview = cell.text.chars().take(40).collect::<String>();
                log::debug!(
                    "[Rust] Cell[{}] '{}' bbox=({:.1},{:.1},{:.1},{:.1}) area={:.1}",
                    cell_idx,
                    preview,
                    cell_bbox.l,
                    cell_bbox.t,
                    cell_bbox.r,
                    cell_bbox.b,
                    cell_bbox.area()
                );
            }

            for (idx, cluster) in clusters.iter().enumerate() {
                let overlap_ratio = cell_bbox.intersection_over_self(&cluster.bbox);

                if debug_picture_clusters && cell_idx < 20 && overlap_ratio > 0.05 {
                    log::debug!(
                        "  vs Cluster[{}] ID={} label={:?}: overlap={:.6}",
                        idx,
                        cluster.id,
                        cluster.label,
                        overlap_ratio
                    );
                }

                // Use > to match Python behavior (first max wins, not last)
                // Python: overlap_ratio > best_overlap
                // When there are ties (multiple clusters with same overlap), Python assigns to the FIRST one
                if overlap_ratio > best_overlap {
                    best_overlap = overlap_ratio;
                    best_cluster_idx = Some(idx);
                }
            }

            if let Some(idx) = best_cluster_idx {
                if debug_picture_clusters && cell_idx < 100 {
                    log::debug!(
                        "  → ASSIGNED to Cluster ID={} label={:?} (overlap={:.6})",
                        clusters[idx].id,
                        clusters[idx].label,
                        best_overlap
                    );
                }
                // Convert SimpleTextCell to TextCell and add to cluster
                clusters[idx].cells.push(cell.to_text_cell());
            } else if debug_picture_clusters && cell_idx < 20 {
                log::debug!(
                    "  → NOT ASSIGNED (best_overlap={:.6} < min={:.2})",
                    best_overlap,
                    self.config.min_cell_overlap
                );
            }
        }

        // Deduplicate cells in each cluster after assignment (matches Python line 614)
        for cluster in &mut *clusters {
            cluster.cells = Self::deduplicate_cells(std::mem::take(&mut cluster.cells));
        }

        // Debug: Show picture cluster cell counts and check ID=22
        if debug_picture_clusters {
            log::debug!("[Rust] Picture cluster cell counts after assignment:");
            for cluster in clusters.iter() {
                if cluster.label == DocItemLabel::Picture {
                    log::debug!("  Cluster ID={}: {} cells", cluster.id, cluster.cells.len());
                }
                if cluster.id == 22 {
                    log::debug!(
                        "[Rust] Cluster ID=22: label={:?}, {} cells",
                        cluster.label,
                        cluster.cells.len()
                    );
                }
            }
        }
    }

    /// Stage 4: Remove clusters with no cells
    ///
    /// Keep only clusters that have cells OR are FORMULA label (formulas may not have cells)
    // Method signature kept for API consistency with other LayoutPostprocessor methods
    #[allow(clippy::unused_self)]
    fn remove_empty_clusters(&self, clusters: Vec<Cluster>) -> Vec<Cluster> {
        clusters
            .into_iter()
            .filter(|cluster| !cluster.cells.is_empty() || cluster.label == DocItemLabel::Formula)
            .collect()
    }

    /// Stage 5: Create orphan clusters for unassigned cells (accounting for special clusters)
    ///
    /// Find cells not assigned to any cluster (regular OR special) and create TEXT clusters for them.
    // Method signature kept for API consistency with other LayoutPostprocessor methods
    #[allow(clippy::unused_self)]
    fn create_orphan_clusters_with_special(
        &self,
        regular_clusters: &[Cluster],
        special_clusters: &[Cluster],
        cells: &[SimpleTextCell],
        max_input_id: usize,
    ) -> Vec<Cluster> {
        // Find assigned cell indices from BOTH regular and special clusters
        let mut assigned_indices: HashSet<usize> = regular_clusters
            .iter()
            .flat_map(|cluster| cluster.cells.iter().map(|cell| cell.index))
            .collect();

        // Add cells from special clusters
        for cluster in special_clusters {
            for cell in &cluster.cells {
                assigned_indices.insert(cell.index);
            }
        }

        // Find unassigned cells
        let mut orphan_clusters = Vec::new();
        // Use max from original input clusters (passed in), not filtered clusters
        // Python behavior: orphan IDs start from max of ALL input IDs, not just survivors
        let mut next_cluster_id = max_input_id + 1;

        for cell in cells {
            if cell.text.trim().is_empty() {
                continue;
            }

            if !assigned_indices.contains(&cell.index) {
                // Log the unassigned cell
                log::debug!(
                    "    Unassigned cell[{}]: \"{}\", bbox=({:.1},{:.1},{:.1},{:.1})",
                    cell.index,
                    cell.text.chars().take(40).collect::<String>(),
                    cell.rect.l,
                    cell.rect.t,
                    cell.rect.r,
                    cell.rect.b
                );

                // Create TEXT cluster for orphan cell
                let orphan_cluster = Cluster {
                    id: next_cluster_id,
                    label: DocItemLabel::Text,
                    bbox: *cell.bbox(),
                    confidence: cell.confidence,
                    cells: vec![cell.to_text_cell()],
                    children: vec![],
                };
                orphan_clusters.push(orphan_cluster);
                next_cluster_id += 1;
            }
        }

        orphan_clusters
    }

    /// Stage 6: Iterative refinement
    ///
    /// Performs iterative bbox adjustment and overlap removal (max 3 iterations).
    /// Reference: ~/`docling/docling/utils/layout_postprocessor.py` (lines 302-311)
    // Method signature kept for API consistency with other LayoutPostprocessor methods
    #[allow(clippy::unused_self)]
    fn iterative_refinement(&self, mut clusters: Vec<Cluster>) -> Vec<Cluster> {
        let micro_profile = std::env::var("MICRO_PROFILE").is_ok();
        let mut prev_count = clusters.len() + 1;

        for _iteration in 0..3 {
            // Early termination if no changes
            if prev_count == clusters.len() {
                break;
            }
            prev_count = clusters.len();

            // Phase 1: Adjust cluster bboxes to contain cells
            clusters = timed_operation(micro_profile, "    Adjust bboxes", || {
                Self::adjust_cluster_bboxes(clusters)
            });

            // Phase 2: Remove overlapping clusters
            clusters = timed_operation(micro_profile, "    Remove overlaps", || {
                Self::remove_overlapping_clusters(clusters, OverlapParams::REGULAR)
            });
        }

        clusters
    }

    /// Adjust cluster bounding boxes to contain their cells
    ///
    /// Reference: ~/`docling/docling/utils/layout_postprocessor.py` (lines 627-651)
    fn adjust_cluster_bboxes(mut clusters: Vec<Cluster>) -> Vec<Cluster> {
        for cluster in &mut clusters {
            if cluster.cells.is_empty() {
                continue;
            }

            // Calculate bbox containing all cells
            let cell_bboxes: Vec<BoundingBox> =
                cluster.cells.iter().map(|c| c.rect.to_bbox()).collect();
            let cells_l = cell_bboxes
                .iter()
                .map(|b| b.l)
                .fold(f32::INFINITY, f32::min);
            let cells_t = cell_bboxes
                .iter()
                .map(|b| b.t)
                .fold(f32::INFINITY, f32::min);
            let cells_r = cell_bboxes
                .iter()
                .map(|b| b.r)
                .fold(f32::NEG_INFINITY, f32::max);
            let cells_b = cell_bboxes
                .iter()
                .map(|b| b.b)
                .fold(f32::NEG_INFINITY, f32::max);

            let cells_bbox = BoundingBox {
                l: cells_l,
                t: cells_t,
                r: cells_r,
                b: cells_b,
                coord_origin: cluster.bbox.coord_origin,
            };

            // Special handling for TABLE clusters: union of original and cells bbox
            if cluster.label == DocItemLabel::Table {
                cluster.bbox = BoundingBox {
                    l: cluster.bbox.l.min(cells_bbox.l),
                    t: cluster.bbox.t.min(cells_bbox.t),
                    r: cluster.bbox.r.max(cells_bbox.r),
                    b: cluster.bbox.b.max(cells_bbox.b),
                    coord_origin: cluster.bbox.coord_origin,
                };
            } else {
                // For other clusters: replace with cells bbox
                cluster.bbox = cells_bbox;
            }
        }

        clusters
    }

    /// Remove overlapping clusters using Union-Find algorithm
    ///
    /// Reference: ~/`docling/docling/utils/layout_postprocessor.py` (lines 487-542)
    fn remove_overlapping_clusters(clusters: Vec<Cluster>, params: OverlapParams) -> Vec<Cluster> {
        trace!(
            "[N=2281 START] remove_overlapping_clusters called with {} clusters",
            clusters.len()
        );

        if clusters.is_empty() {
            return vec![];
        }

        // Build spatial index
        let spatial_index = SpatialClusterIndex::new(&clusters);

        // Map clusters by ID for fast lookup
        let clusters_by_id: HashMap<usize, &Cluster> = clusters.iter().map(|c| (c.id, c)).collect();

        // Initialize UnionFind
        let mut uf = UnionFind::new(clusters.iter().map(|c| c.id));

        // Phase A: Build union-find groups by detecting overlaps
        let mut adjacency_checks = 0;
        let mut adjacency_merges = 0;

        for cluster in &clusters {
            let candidates = spatial_index.find_candidates(&cluster.bbox);

            for other_id in candidates {
                if other_id == cluster.id {
                    continue; // Skip self
                }

                if let Some(other) = clusters_by_id.get(&other_id) {
                    // Sub-phase A1: Check overlap-based merging
                    if SpatialClusterIndex::check_overlap(
                        &cluster.bbox,
                        &other.bbox,
                        params.overlap_threshold,
                        params.containment_threshold,
                    ) {
                        uf.union(cluster.id, other_id);
                    }
                    // Sub-phase A2: Check horizontal adjacency for text clusters
                    // Fix for N=2280: Merge adjacent text boxes to prevent fragmentation
                    // Example: "Pre" + "-" + "Digital Era" should be ONE DocItem
                    // Only applies when adjacency thresholds are configured (> 0)
                    // N=4417: Exclude page_header/page_footer from adjacency merge
                    // Python keeps page furniture elements separate (title, page number)
                    else if params.horizontal_gap_multiplier > 0.0
                        && cluster.label.is_text_element()
                        && other.label.is_text_element()
                        && !cluster.label.is_page_header()
                        && !other.label.is_page_header()
                    {
                        adjacency_checks += 1;

                        let gap = other.bbox.l - cluster.bbox.r; // horizontal gap
                        let cluster_height = (cluster.bbox.b - cluster.bbox.t).abs();
                        let other_height = (other.bbox.b - other.bbox.t).abs();
                        let avg_height = (cluster_height + other_height) / 2.0;

                        // Merge if:
                        // 1. Horizontally adjacent (gap <= multiplier * average height)
                        // 2. Vertically aligned (y-overlap > threshold)
                        if gap > 0.0 && gap <= avg_height * params.horizontal_gap_multiplier {
                            // Check vertical alignment
                            let vertical_overlap = cluster.bbox.intersection_area(&other.bbox)
                                / cluster.bbox.area().min(other.bbox.area());
                            let v_align = (cluster.bbox.t - other.bbox.t).abs()
                                < avg_height * params.vertical_align_multiplier;

                            if vertical_overlap > params.vertical_overlap_threshold || v_align {
                                adjacency_merges += 1;
                                uf.union(cluster.id, other_id);
                            }
                        }
                    }
                }
            }
        }

        trace!("[N=2281] Adjacency checks: {adjacency_checks}, merges: {adjacency_merges}");

        // Phase B: Select best cluster from each group and merge cells
        let mut result = Vec::new();
        let groups = uf.get_groups();

        for group in groups.values() {
            if group.len() == 1 {
                // Singleton group - keep as-is
                let cluster = clusters_by_id[&group[0]].clone();
                result.push(cluster);
                continue;
            }

            // Multi-cluster group - select best and merge cells
            // CRITICAL (N=219): Sort group by cluster ID to match Python's iteration order
            // Python iterates clusters in the order they appear in Stage 3 output (by score descending)
            // Cluster IDs are assigned sequentially in Stage 3, so sorting by ID preserves this order
            //
            // NOTE (N=219): This fix is INCOMPLETE. Sorting by ID doesn't fully match Python's behavior.
            // Issue: When UnionFind creates groups via union() calls, the order of elements within
            // each group depends on the order of union() calls, not just the initial cluster order.
            // Python's dict maintains insertion order, so the group order reflects union() call order.
            //
            // Example: For clusters [18, 21, 24] with overlaps:
            //   - If union(18, 24) then union(21, 18), group might be [18, 24, 21]
            //   - Sorting gives [18, 21, 24], but Python might have [21, 18, 24]
            //
            // Impact: 2/11 pages fail (arxiv pages 1 and 7) with label mismatches on duplicate bboxes.
            // Both pages have ML model outputs with identical bboxes but different labels.
            // Python selects Text, Rust selects PageHeader (first in sorted order).
            //
            // Next steps:
            //   1. Add instrumentation to log union() call order
            //   2. Modify UnionFind to track insertion order within groups
            //   3. OR: Change tie-breaking rule (e.g., prefer lower confidence when areas equal)
            let mut group_sorted = group.clone();
            group_sorted.sort_unstable();

            let group_clusters: Vec<&Cluster> =
                group_sorted.iter().map(|id| clusters_by_id[id]).collect();

            let mut best = Self::select_best_cluster_from_group(&group_clusters, params).clone();

            // Merge cells from all clusters in group
            for cluster in &group_clusters {
                if cluster.id != best.id {
                    best.cells.extend(cluster.cells.clone());
                }
            }

            // Deduplicate and sort cells
            best.cells = Self::deduplicate_cells(best.cells);
            best.cells.sort_by_key(|c| c.index);

            result.push(best);
        }

        result
    }

    /// Check if candidate should replace current best based on area/confidence
    fn should_replace_best(
        candidate: &Cluster,
        best: &Cluster,
        params: OverlapParams,
    ) -> Option<&'static str> {
        let cand_area = candidate.bbox.area();
        let best_area = best.bbox.area();
        let conf_diff = best.confidence - candidate.confidence;
        let is_near_identical = (cand_area - best_area).abs() < 1.0;

        // Rule 1: Larger area wins if confidence difference is small
        if cand_area > best_area && conf_diff <= params.conf_threshold {
            return Some("larger area");
        }

        // Rule 2: Tie-breaker for near-identical areas
        if is_near_identical {
            // Text preferred over PageHeader (N=222)
            if candidate.label == DocItemLabel::Text && best.label == DocItemLabel::PageHeader {
                return Some("Text over PageHeader tie-breaker");
            }
        }

        None
    }

    /// Select best cluster from a group of overlapping clusters
    ///
    /// Reference: ~/`docling/docling/utils/layout_postprocessor.py` (lines 454-485)
    fn select_best_cluster_from_group<'a>(
        group_clusters: &[&'a Cluster],
        params: OverlapParams,
    ) -> &'a Cluster {
        let has_kvr = group_clusters
            .iter()
            .any(|c| c.label == DocItemLabel::KeyValueRegion);
        let verbose = has_kvr && group_clusters.len() > 1;

        // Debug: log group info
        if verbose {
            log::debug!(
                "      [DEBUG] Selecting from group of {} clusters (contains KVR):",
                group_clusters.len()
            );
            for c in group_clusters {
                log::debug!(
                    "        ID={}, label={:?}, area={:.1}, conf={:.6}",
                    c.id,
                    c.label,
                    c.bbox.area(),
                    c.confidence
                );
            }
        }

        let mut current_best: Option<&Cluster> = None;

        for candidate in group_clusters {
            // Check if candidate beats all others
            let should_select = group_clusters
                .iter()
                .filter(|other| other.id != candidate.id)
                .all(|other| Self::should_prefer_cluster(candidate, other, params));

            if verbose {
                log::debug!(
                    "        ID={}: should_select={}",
                    candidate.id,
                    should_select
                );
            }

            if !should_select {
                continue;
            }

            match current_best {
                None => {
                    current_best = Some(candidate);
                    if verbose {
                        log::debug!("          → Set as current_best (first)");
                    }
                }
                Some(best) => {
                    if let Some(reason) = Self::should_replace_best(candidate, best, params) {
                        if verbose {
                            log::debug!("          → Replace current_best ({reason})");
                        }
                        current_best = Some(candidate);
                    } else if verbose {
                        log::debug!("          → Keep current_best ID={}", best.id);
                    }
                }
            }
        }

        let result = current_best.unwrap_or(group_clusters[0]);
        if verbose {
            log::debug!(
                "      [DEBUG] Winner: ID={}, label={:?}, area={:.1}, conf={:.6}",
                result.id,
                result.label,
                result.bbox.area(),
                result.confidence
            );
        }
        result
    }

    /// Determine if candidate cluster should be preferred over other cluster
    ///
    /// Reference: ~/`docling/docling/utils/layout_postprocessor.py` (lines 418-452)
    fn should_prefer_cluster(candidate: &Cluster, other: &Cluster, params: OverlapParams) -> bool {
        // Rule 1: LIST_ITEM vs TEXT
        if candidate.label == DocItemLabel::ListItem && other.label == DocItemLabel::Text {
            let area_ratio = candidate.bbox.area() / other.bbox.area();
            let area_similarity = (1.0 - area_ratio).abs() < params.area_similarity_threshold;
            if area_similarity {
                return true; // Prefer LIST_ITEM over TEXT if similar size
            }
        }

        // Rule 2: CODE vs others
        if candidate.label == DocItemLabel::Code {
            let containment = other.bbox.intersection_over_self(&candidate.bbox);
            if containment > params.containment_threshold {
                return true; // Prefer CODE if it contains other
            }
        }

        // Fallback: Area/confidence threshold
        let area_ratio = candidate.bbox.area() / other.bbox.area();
        let conf_diff = other.confidence - candidate.confidence;

        if area_ratio <= params.area_threshold && conf_diff > params.conf_threshold {
            return false; // Reject candidate if smaller AND much lower confidence
        }

        true // Default to keeping candidate
    }

    /// Deduplicate cells by index (first occurrence wins)
    fn deduplicate_cells(cells: Vec<crate::pipeline::TextCell>) -> Vec<crate::pipeline::TextCell> {
        let mut seen_indices = HashSet::new();
        let mut unique_cells = Vec::new();

        for cell in cells {
            if seen_indices.insert(cell.index) {
                unique_cells.push(cell);
            }
        }

        unique_cells
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::CoordOrigin;

    #[test]
    fn test_post_processor_creation() {
        let _pp = LayoutPostProcessor::new_default();
    }

    #[test]
    fn test_cell_assignment() {
        let mut clusters = vec![Cluster {
            id: 0,
            label: DocItemLabel::Text,
            bbox: BoundingBox {
                l: 0.0,
                t: 0.0,
                r: 100.0,
                b: 50.0,
                coord_origin: CoordOrigin::TopLeft,
            },
            confidence: 0.9,
            cells: vec![],
            children: vec![],
        }];

        let cells = vec![SimpleTextCell {
            index: 0,
            text: "Test".to_string(),
            rect: BoundingBox {
                l: 10.0,
                t: 10.0,
                r: 90.0,
                b: 40.0,
                coord_origin: CoordOrigin::TopLeft,
            },
            confidence: 1.0,
            from_ocr: false,
            is_bold: false,
            is_italic: false,
        }];

        let pp = LayoutPostProcessor::new_default();
        pp.assign_cells_to_clusters(&mut clusters, &cells);

        assert_eq!(clusters[0].cells.len(), 1);
        assert_eq!(clusters[0].cells[0].text, "Test");
    }
}
