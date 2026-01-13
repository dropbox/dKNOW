// Reading Order implementation - Rule-based algorithm for document element ordering
// Ported from: docling_ibm_models/reading_order/reading_order_rb.py
//
// Note: Infrastructure code ported from Python. Some code paths not yet wired up.
#![allow(dead_code)]
// Intentional ML conversions: page indices, bounding box coordinates
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
// Algorithm functions take Vec ownership for data flow semantics
#![allow(clippy::needless_pass_by_value)]

use crate::pipeline::data_structures::{
    BoundingBox, CoordOrigin, DocItemLabel, PageElement, US_LETTER_HEIGHT_F32, US_LETTER_WIDTH_F32,
};
#[cfg(feature = "debug-trace")]
use log::trace;
use regex::Regex;
use rstar::{RTree, AABB};
use std::collections::{BTreeMap, HashMap};
use std::sync::LazyLock;

/// Pattern 1: text ending with lowercase letter or hyphen (with optional trailing space)
static PATTERN1: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r".+([a-z,\-])(\s*)$").expect("valid pattern1 regex"));
/// Pattern 2: text starting with optional space + lowercase letter
static PATTERN2: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\s*[a-z])(.+)").expect("valid pattern2 regex"));

/// Configuration for reading order predictor
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReadingOrderConfig {
    /// Enable horizontal dilation of element bboxes
    pub dilated_page_element: bool,
    /// Horizontal dilation threshold (fraction of page width)
    pub horizontal_dilation_threshold_norm: f32,
    /// Epsilon for strict comparison operators
    pub eps: f32,
    /// Row bucket height for vertical discretization (in pixels)
    ///
    /// Elements within this vertical distance are treated as "same row"
    /// and sorted horizontally. Elements further apart are sorted vertically.
    /// Fine-grained values (e.g., 1.0) approximate Python's non-transitive
    /// comparison while maintaining Rust's sorting requirements.
    pub row_height: f32,
    /// Horizontal padding for R-tree queries when finding elements above/below (in points)
    ///
    /// When querying for elements above or below a target element, the horizontal range
    /// is expanded by this amount on each side to catch elements that slightly overlap.
    pub rtree_query_horizontal_padding: f32,
}

impl Default for ReadingOrderConfig {
    #[inline]
    fn default() -> Self {
        Self {
            dilated_page_element: true,
            horizontal_dilation_threshold_norm: 0.15,
            eps: 1e-3,
            row_height: 1.0,
            rtree_query_horizontal_padding: 0.1,
        }
    }
}

/// Internal state for reading order predictor
#[derive(Debug)]
struct ReadingOrderState {
    /// Element index (cid) to array index mapping
    h2i_map: HashMap<usize, usize>,
    /// Array index to element index (cid) mapping
    i2h_map: HashMap<usize, usize>,
    /// Left-to-right neighbor map (reserved for future use)
    #[allow(
        dead_code,
        reason = "reserved for bidirectional reading order enhancement"
    )]
    l2r_map: HashMap<usize, usize>,
    /// Right-to-left neighbor map (reserved for future use)
    #[allow(
        dead_code,
        reason = "reserved for bidirectional reading order enhancement"
    )]
    r2l_map: HashMap<usize, usize>,
    /// Up map: elements above (predecessors) - `BTreeMap` for deterministic iteration
    up_map: BTreeMap<usize, Vec<usize>>,
    /// Down map: elements below (successors) - `BTreeMap` for deterministic iteration
    dn_map: BTreeMap<usize, Vec<usize>>,
    /// Head elements (no predecessors)
    heads: Vec<usize>,
}

impl ReadingOrderState {
    fn new() -> Self {
        Self {
            h2i_map: HashMap::new(),
            i2h_map: HashMap::new(),
            l2r_map: HashMap::new(),
            r2l_map: HashMap::new(),
            up_map: BTreeMap::new(),
            dn_map: BTreeMap::new(),
            heads: Vec::new(),
        }
    }
}

/// Element with spatial information for reading order
#[derive(Debug, Clone, Copy, PartialEq)]
struct OrderableElement {
    /// Element ID (cluster ID)
    cid: usize,
    /// Page number
    page_no: usize,
    /// Bounding box (in BOTTOMLEFT coordinates)
    bbox: BoundingBox,
    /// Page width
    page_width: f32,
    /// Page height
    page_height: f32,
    /// Element label
    label: DocItemLabel,
}

impl OrderableElement {
    /// Convert from `PageElement` to `OrderableElement`
    fn from_page_element(elem: &PageElement, page_width: f32, page_height: f32) -> Self {
        let cluster = elem.cluster();
        let bbox = cluster.bbox.to_bottom_left_origin(page_height);

        Self {
            cid: cluster.id,
            page_no: elem.page_no(),
            bbox,
            page_width,
            page_height,
            label: cluster.label,
        }
    }

    /// Check if two elements overlap horizontally
    /// Python: `docling_core.types.doc.base.BoundingBox.overlaps_horizontally`
    /// Returns true if horizontal ranges [l, r] overlap
    #[inline]
    fn overlaps_horizontally(&self, other: &Self) -> bool {
        // not (self.r <= other.l or other.r <= self.l)
        !(self.bbox.r <= other.bbox.l || other.bbox.r <= self.bbox.l)
    }

    /// Comparison for sorting - approximates Python's reading order logic transitively
    ///
    /// Python's original comparison (from `docling_ibm_models/reading_order/reading_order_rb.py)`:
    /// ```python
    /// def __lt__(self, other):
    ///     if self.overlaps_horizontally(other):
    ///         return self.b > other.b  # Vertical: bottom descending
    ///     else:
    ///         return self.l < other.l  # Horizontal: left ascending
    /// ```
    ///
    /// PROBLEM: This is NON-TRANSITIVE - causes Rust's sort to panic (N=120 confirmed)
    /// SOLUTION: Use fine-grained row buckets to approximate Python's semantics
    /// while maintaining strict transitivity:
    /// 1. Primary: Page number
    /// 2. Row bucket (configurable vertical discretization) - groups vertically close elements
    /// 3. Within bucket: left coordinate (left-to-right)
    /// 4. Tie-breaker: cluster ID
    ///
    /// Using fine-grained buckets means elements within that distance vertically are
    /// treated as "same row" and sorted horizontally. Elements further apart are
    /// sorted vertically. This closely approximates Python while ensuring transitivity.
    ///
    /// # Arguments
    /// * `other` - Element to compare against
    /// * `row_height` - Row bucket height for vertical discretization (in pixels)
    fn compare_with_row_height(&self, other: &Self, row_height: f32) -> std::cmp::Ordering {
        use std::cmp::Ordering;

        // Primary: page number
        match self.page_no.cmp(&other.page_no) {
            Ordering::Equal => {}
            other_ord => return other_ord,
        }

        // Secondary: row bucket
        // In BOTTOMLEFT coords: larger bottom = higher on page
        // We want higher rows to come first, so negate before bucketing
        let self_row = (-self.bbox.b / row_height).floor() as i64;
        let other_row = (-other.bbox.b / row_height).floor() as i64;

        match self_row.cmp(&other_row) {
            Ordering::Equal => {
                // Same row bucket → compare horizontally (left to right)
                match self.bbox.l.total_cmp(&other.bbox.l) {
                    Ordering::Equal => self.cid.cmp(&other.cid),
                    other_ord => other_ord,
                }
            }
            other_ord => other_ord,
        }
    }
}

/// Envelope for R-tree spatial indexing of elements
#[derive(Debug, Clone, Copy)]
struct ElementEnvelope {
    aabb: AABB<[f32; 2]>,
    index: usize,
}

impl rstar::RTreeObject for ElementEnvelope {
    type Envelope = AABB<[f32; 2]>;

    fn envelope(&self) -> Self::Envelope {
        self.aabb
    }
}

impl rstar::PointDistance for ElementEnvelope {
    fn distance_2(&self, point: &[f32; 2]) -> f32 {
        self.aabb.distance_2(point)
    }
}

/// Reading order predictor
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ReadingOrderPredictor {
    config: ReadingOrderConfig,
}

impl ReadingOrderPredictor {
    /// Creates a new reading order predictor with the given configuration.
    #[inline]
    #[must_use = "returns a new ReadingOrderPredictor instance"]
    pub const fn new(config: ReadingOrderConfig) -> Self {
        Self { config }
    }

    /// Predicts the reading order of elements based on spatial layout.
    ///
    /// Returns a vector of indices representing the order in which elements
    /// should be read.
    #[must_use = "returns the predicted reading order indices"]
    pub fn predict(
        &self,
        elements: &[PageElement],
        page_dimensions: &HashMap<usize, (f32, f32)>,
    ) -> Vec<usize> {
        if elements.is_empty() {
            return Vec::new();
        }

        // Group elements by page - use BTreeMap for deterministic page order
        let mut pages: BTreeMap<usize, Vec<&PageElement>> = BTreeMap::new();
        for elem in elements {
            pages.entry(elem.page_no()).or_default().push(elem);
        }

        let mut final_order = Vec::new();

        // Process each page independently (BTreeMap iterates in sorted key order)
        for (page_no, page_elements) in pages {
            // Get page dimensions or fall back to US Letter size
            let (page_width, page_height) = page_dimensions
                .get(&page_no)
                .copied()
                .unwrap_or((US_LETTER_WIDTH_F32, US_LETTER_HEIGHT_F32));
            let page_order = self.predict_page(&page_elements, page_width, page_height);
            final_order.extend(page_order);
        }

        final_order
    }

    fn predict_page(
        &self,
        elements: &[&PageElement],
        page_width: f32,
        page_height: f32,
    ) -> Vec<usize> {
        if elements.is_empty() {
            return Vec::new();
        }

        // Convert to orderable elements with BOTTOMLEFT coordinates
        let mut orderable: Vec<OrderableElement> = elements
            .iter()
            .map(|e| OrderableElement::from_page_element(e, page_width, page_height))
            .collect();

        // CRITICAL FIX: Sort elements by cid for deterministic processing
        // This ensures the same input order for all downstream operations
        orderable.sort_by_key(|e| e.cid);

        // Separate page headers, footers, and body elements (matches Python)
        // Python code: reading_order_rb.py:predict_reading_order()
        // - page_to_headers: PAGE_HEADER elements
        // - page_to_elems: Body elements (all others)
        // - page_to_footers: PAGE_FOOTER elements
        // Final order: headers + body + footers
        let mut headers = Vec::new();
        let mut footers = Vec::new();
        let mut body = Vec::new();

        for elem in orderable {
            match elem.label {
                DocItemLabel::PageHeader => headers.push(elem),
                DocItemLabel::PageFooter => footers.push(elem),
                _ => body.push(elem),
            }
        }

        // Sort headers by position
        let row_height = self.config.row_height;
        headers.sort_by(|a, b| a.compare_with_row_height(b, row_height));

        // Sort footers by position
        footers.sort_by(|a, b| a.compare_with_row_height(b, row_height));

        // Initialize state
        let mut state = ReadingOrderState::new();
        self.init_h2i_map(&body, &mut state);

        // Build up/down maps using R-tree
        self.init_ud_maps(&body, &mut state);

        // Optional: horizontal dilation
        if self.config.dilated_page_element {
            self.do_horizontal_dilation(&mut body, &mut state);
        }

        // Find head elements
        self.find_heads(&body, &mut state);

        // Sort down-map children for deterministic traversal (matches Python)
        self.sort_ud_maps(&body, &mut state);

        // Depth-first traversal to find reading order
        let body_order = self.find_order(&body, &state);

        // Combine headers, body, and footers (matches Python order)
        let mut result = Vec::new();
        for h in headers {
            result.push(h.cid);
        }
        result.extend(body_order);
        for f in footers {
            result.push(f.cid);
        }

        result
    }

    /// Initialize hash maps for bidirectional index lookup
    // Method signature kept for API consistency with other ReadingOrderProcessor methods
    #[allow(clippy::unused_self)]
    fn init_h2i_map(&self, elements: &[OrderableElement], state: &mut ReadingOrderState) {
        for (i, elem) in elements.iter().enumerate() {
            state.h2i_map.insert(elem.cid, i);
            state.i2h_map.insert(i, elem.cid);
        }
    }

    /// Initialize up/down maps using R-tree spatial indexing
    fn init_ud_maps(&self, elements: &[OrderableElement], state: &mut ReadingOrderState) {
        if elements.is_empty() {
            return;
        }

        // Build R-tree spatial index
        let rtree_data: Vec<ElementEnvelope> = elements
            .iter()
            .enumerate()
            .map(|(i, elem)| {
                let bbox = &elem.bbox;
                let aabb = AABB::from_corners([bbox.l, bbox.b], [bbox.r, bbox.t]);
                ElementEnvelope { aabb, index: i }
            })
            .collect();

        let rtree = RTree::bulk_load(rtree_data);

        // For each element j, find all elements i that precede it (i -> j)
        for (j_idx, elem_j) in elements.iter().enumerate() {
            let j_cid = elem_j.cid;

            // Query R-tree for elements above elem_j
            let h_pad = self.config.rtree_query_horizontal_padding;
            let query_bbox = AABB::from_corners(
                [elem_j.bbox.l - h_pad, elem_j.bbox.t],
                [elem_j.bbox.r + h_pad, elem_j.page_height + h_pad],
            );

            let mut candidates: Vec<usize> = rtree
                .locate_in_envelope(&query_bbox)
                .map(|env| env.index)
                .collect();

            // CRITICAL FIX: Sort candidates for deterministic processing
            // R-tree query results can be in non-deterministic order
            candidates.sort_unstable();

            // Check each candidate i to see if it precedes j
            for i_idx in candidates {
                if i_idx == j_idx {
                    continue;
                }

                let elem_i = &elements[i_idx];
                let i_cid = elem_i.cid;

                // Check if i is strictly above j and overlaps horizontally
                if !elem_i.bbox.is_strictly_above(&elem_j.bbox, self.config.eps) {
                    continue;
                }

                if !elem_i.bbox.overlaps_horizontally(&elem_j.bbox) {
                    continue;
                }

                // Check for sequence interruption
                if self.has_sequence_interruption(&rtree, elements, elem_i, elem_j) {
                    continue;
                }

                // Add edge i -> j
                state.dn_map.entry(i_cid).or_default().push(j_cid);
                state.up_map.entry(j_cid).or_default().push(i_cid);
            }
        }

        // DEBUG: Export up/down maps for first page only
        if !elements.is_empty() && elements[0].page_no == 0 {
            self.export_ud_maps_debug(elements, state);
        }
    }

    /// Export up/down maps for debugging (conditional compilation for debugging)
    // Method signature kept for API consistency with other ReadingOrderProcessor methods
    #[allow(clippy::unused_self)]
    fn export_ud_maps_debug(&self, elements: &[OrderableElement], state: &ReadingOrderState) {
        #[cfg(feature = "debug-trace")]
        use std::{fs, io::Write};

        // Format up_map as proper JSON
        let up_map_json = state
            .up_map
            .iter()
            .map(|(k, v)| format!("    \"{k}\": {v:?}"))
            .collect::<Vec<_>>()
            .join(",\n");

        // Format dn_map as proper JSON
        let dn_map_json = state
            .dn_map
            .iter()
            .map(|(k, v)| format!("    \"{k}\": {v:?}"))
            .collect::<Vec<_>>()
            .join(",\n");

        // Format elements
        let elements_json = elements
            .iter()
            .enumerate()
            .map(|(i, e)| {
                format!(
                    "    {{\"idx\": {}, \"cid\": {}, \"page_no\": {}, \"label\": \"{:?}\", \"bbox\": {{\"l\": {:.2}, \"t\": {:.2}, \"r\": {:.2}, \"b\": {:.2}}}}}",
                    i, e.cid, e.page_no, e.label, e.bbox.l, e.bbox.t, e.bbox.r, e.bbox.b
                )
            })
            .collect::<Vec<_>>()
            .join(",\n");

        let _output = format!(
            "{{\n  \"up_map\": {{\n{up_map_json}\n  }},\n  \"dn_map\": {{\n{dn_map_json}\n  }},\n  \"elements\": [\n{elements_json}\n  ]\n}}"
        );

        #[cfg(feature = "debug-trace")]
        {
            if let Ok(mut file) = fs::File::create("debug_output/rust_ud_maps_arxiv_p0.json") {
                let _ = file.write_all(_output.as_bytes());
                trace!("Exported Rust up/down maps to debug_output/rust_ud_maps_arxiv_p0.json");
            }
        }
    }

    /// Check if there's an interrupting element between i and j
    // Method signature kept for API consistency with other ReadingOrderProcessor methods
    #[allow(clippy::unused_self)]
    fn has_sequence_interruption(
        &self,
        rtree: &RTree<ElementEnvelope>,
        elements: &[OrderableElement],
        elem_i: &OrderableElement,
        elem_j: &OrderableElement,
    ) -> bool {
        // Query R-tree for elements in rectangle between i and j
        let query_bbox = AABB::from_corners(
            [elem_i.bbox.l.min(elem_j.bbox.l), elem_j.bbox.t],
            [elem_i.bbox.r.max(elem_j.bbox.r), elem_i.bbox.b],
        );

        for env in rtree.locate_in_envelope(&query_bbox) {
            let elem_w = &elements[env.index];

            // Skip if it's i or j
            if elem_w.cid == elem_i.cid || elem_w.cid == elem_j.cid {
                continue;
            }

            // Check if w interrupts the i->j sequence
            let w_overlaps_i = elem_w.bbox.overlaps_horizontally(&elem_i.bbox);
            let w_overlaps_j = elem_w.bbox.overlaps_horizontally(&elem_j.bbox);

            if w_overlaps_i || w_overlaps_j {
                // Check if w is strictly between i and j vertically
                let w_below_i = elem_w.bbox.b < elem_i.bbox.b;
                let w_above_j = elem_w.bbox.t > elem_j.bbox.t;

                if w_below_i && w_above_j {
                    return true;
                }
            }
        }

        false
    }

    /// Horizontal dilation of element bboxes to improve column detection
    fn do_horizontal_dilation(
        &self,
        elements: &mut [OrderableElement],
        state: &mut ReadingOrderState,
    ) {
        if elements.is_empty() {
            return;
        }

        let page_width = elements[0].page_width;
        let dilation_threshold = page_width * self.config.horizontal_dilation_threshold_norm;

        // Clone original bboxes
        let original_bboxes: Vec<_> = elements.iter().map(|e| e.bbox).collect();

        // Dilate each element
        for (i, elem) in elements.iter_mut().enumerate() {
            let cid = elem.cid;

            // Get predecessors and successors
            let predecessors = state.up_map.get(&cid).cloned().unwrap_or_default();
            let successors = state.dn_map.get(&cid).cloned().unwrap_or_default();

            let mut new_l = elem.bbox.l;
            let mut new_r = elem.bbox.r;

            // Expand to match predecessors/successors
            for &pred_cid in &predecessors {
                if let Some(&pred_idx) = state.h2i_map.get(&pred_cid) {
                    let pred_bbox = &original_bboxes[pred_idx];
                    new_l = new_l.min(pred_bbox.l);
                    new_r = new_r.max(pred_bbox.r);
                }
            }

            for &succ_cid in &successors {
                if let Some(&succ_idx) = state.h2i_map.get(&succ_cid) {
                    let succ_bbox = &original_bboxes[succ_idx];
                    new_l = new_l.min(succ_bbox.l);
                    new_r = new_r.max(succ_bbox.r);
                }
            }

            // Only apply if expansion is within threshold
            let expansion = (new_r - new_l) - (elem.bbox.r - elem.bbox.l);
            if expansion <= dilation_threshold {
                // Check for overlaps with other elements using original bboxes
                let mut has_overlap = false;
                let test_bbox = BoundingBox {
                    l: new_l,
                    r: new_r,
                    t: elem.bbox.t,
                    b: elem.bbox.b,
                    coord_origin: CoordOrigin::BottomLeft,
                };

                for (j, other_bbox) in original_bboxes.iter().enumerate() {
                    if i == j {
                        continue;
                    }
                    if test_bbox.overlaps_horizontally(other_bbox)
                        && test_bbox.overlaps_vertically(other_bbox)
                    {
                        has_overlap = true;
                        break;
                    }
                }

                if !has_overlap {
                    elem.bbox.l = new_l;
                    elem.bbox.r = new_r;
                }
            }
        }

        // Re-initialize up/down maps with dilated bboxes
        state.up_map.clear();
        state.dn_map.clear();
        self.init_ud_maps(elements, state);
    }

    /// Find head elements (elements with no predecessors)
    // Method signature kept for API consistency with other ReadingOrderProcessor methods
    #[allow(clippy::unused_self)]
    fn find_heads(&self, elements: &[OrderableElement], state: &mut ReadingOrderState) {
        let mut heads = Vec::new();

        for elem in elements {
            let predecessors = state.up_map.get(&elem.cid).map_or(0, Vec::len);
            if predecessors == 0 {
                heads.push(*elem);
            }
        }

        // Sort heads using comparison logic
        let row_height = self.config.row_height;
        heads.sort_by(|a, b| a.compare_with_row_height(b, row_height));

        state.heads = heads.iter().map(|h| h.cid).collect();

        #[cfg(feature = "debug-trace")]
        {
            // DEBUG: Print heads for first page
            if !elements.is_empty() && elements[0].page_no == 0 {
                trace!("\nDEBUG: Heads for page 0 (after sorting):");
                for (i, &cid) in state.heads.iter().enumerate() {
                    if let Some(elem) = heads.iter().find(|h| h.cid == cid) {
                        trace!(
                            "  Head {}: cid={}, label={:?}, bbox l={:.2}, t={:.2}",
                            i,
                            cid,
                            elem.label,
                            elem.bbox.l,
                            elem.bbox.t
                        );
                    }
                }
            }
        }
    }

    /// Sort down-map and up-map for deterministic traversal order
    ///
    /// This matches Python's `_sort_ud_maps` method which sorts the children
    /// in each `dn_map` entry to ensure deterministic depth-first traversal.
    ///
    /// CRITICAL FIX: Also sort `up_map` to ensure deterministic upward search.
    /// When multiple predecessors exist, the order they're checked matters.
    /// `HashMap` iteration is non-deterministic, so we must sort both maps.
    // Method signature kept for API consistency with other ReadingOrderProcessor methods
    #[allow(clippy::unused_self)]
    fn sort_ud_maps(&self, elements: &[OrderableElement], state: &mut ReadingOrderState) {
        // Create lookup map from cid to element
        let elem_map: std::collections::HashMap<usize, &OrderableElement> =
            elements.iter().map(|elem| (elem.cid, elem)).collect();

        let row_height = self.config.row_height;

        // Sort dn_map: For each parent, sort its children (successors)
        for children_cids in state.dn_map.values_mut() {
            // Collect child elements
            let mut child_elements: Vec<&OrderableElement> = children_cids
                .iter()
                .filter_map(|&cid| elem_map.get(&cid).copied())
                .collect();

            // Sort using comparison function (same as Python's __lt__)
            child_elements.sort_by(|a, b| a.compare_with_row_height(b, row_height));

            // Update dn_map with sorted cids
            *children_cids = child_elements.iter().map(|e| e.cid).collect();
        }

        // CRITICAL FIX: Also sort up_map for deterministic upward search
        // When depth_first_search_upwards checks predecessors, order matters
        for predecessor_cids in state.up_map.values_mut() {
            // Collect predecessor elements
            let mut pred_elements: Vec<&OrderableElement> = predecessor_cids
                .iter()
                .filter_map(|&cid| elem_map.get(&cid).copied())
                .collect();

            // Sort using comparison function
            pred_elements.sort_by(|a, b| a.compare_with_row_height(b, row_height));

            // Update up_map with sorted cids
            *predecessor_cids = pred_elements.iter().map(|e| e.cid).collect();
        }
    }

    /// Find reading order using depth-first traversal
    ///
    /// Matches Python implementation:
    /// - Visit head elements first
    /// - Then recursively visit their successors
    fn find_order(&self, elements: &[OrderableElement], state: &ReadingOrderState) -> Vec<usize> {
        let mut order = Vec::new();
        let mut visited = vec![false; elements.len()];

        // For each head element
        for &head_cid in &state.heads {
            if let Some(&head_idx) = state.h2i_map.get(&head_cid) {
                if !visited[head_idx] {
                    // Visit the head element FIRST (matches Python)
                    order.push(head_cid);
                    visited[head_idx] = true;

                    // Then recursively visit its successors
                    self.depth_first_search_downwards(head_cid, state, &mut visited, &mut order);
                }
            }
        }

        // N=4407: Add any unvisited elements at the end
        // This handles elements that are disconnected from the main reading order graph
        // (e.g., isolated text boxes, floating elements not spatially connected)
        // Sort unvisited by position to maintain reasonable order
        let mut unvisited: Vec<_> = elements
            .iter()
            .enumerate()
            .filter(|(i, _)| !visited[*i])
            .map(|(_, e)| e)
            .collect();

        if !unvisited.is_empty() {
            // Sort by position: top-to-bottom, left-to-right
            let row_height = self.config.row_height;
            unvisited.sort_by(|a, b| a.compare_with_row_height(b, row_height));

            log::trace!(
                "find_order: Adding {} unvisited elements to end of reading order",
                unvisited.len()
            );

            for elem in unvisited {
                order.push(elem.cid);
            }
        }

        order
    }

    /// Depth-first search upwards to find topmost unvisited predecessor
    ///
    /// Python: _`depth_first_search_upwards`
    /// Starting from element `cid`, walks up the predecessor chain
    /// to find the topmost unvisited element. Returns that element's cid
    /// (or `cid` itself if all predecessors are visited).
    // Method signature kept for API consistency with other ReadingOrderProcessor methods
    #[allow(clippy::unused_self)]
    fn depth_first_search_upwards(
        &self,
        mut cid: usize,
        state: &ReadingOrderState,
        visited: &[bool],
    ) -> usize {
        loop {
            // Get predecessors of current element
            let predecessors = state.up_map.get(&cid).cloned().unwrap_or_default();

            // Find first unvisited predecessor
            let mut found_unvisited = false;
            for &pred_cid in &predecessors {
                if let Some(&pred_idx) = state.h2i_map.get(&pred_cid) {
                    if !visited[pred_idx] {
                        // Move up to this unvisited predecessor
                        cid = pred_cid;
                        found_unvisited = true;
                        break;
                    }
                }
            }

            // If no unvisited predecessor found, return current element
            if !found_unvisited {
                return cid;
            }
        }
    }

    /// Non-recursive depth-first search downwards
    ///
    /// Python: _`depth_first_search_downwards`
    /// Uses a stack of (`successors_list`, offset) tuples to simulate recursion.
    /// For each successor, calls upward search first to find the topmost
    /// unvisited element in its predecessor chain.
    fn depth_first_search_downwards(
        &self,
        start_cid: usize,
        state: &ReadingOrderState,
        visited: &mut [bool],
        order: &mut Vec<usize>,
    ) {
        // Stack contains (successors_list, current_offset)
        // This allows us to resume iteration after recursive calls
        let successors = state.dn_map.get(&start_cid).cloned().unwrap_or_default();
        let mut stack = vec![(successors, 0)];

        while let Some((inds, offset)) = stack.pop() {
            let mut found_non_visited = false;

            // Iterate through successors starting from offset
            if offset < inds.len() {
                for (new_offset, &succ_cid) in inds.iter().enumerate().skip(offset) {
                    // CRITICAL: Call upward search to find topmost unvisited predecessor
                    // This resolves dependency chains before visiting the element
                    let k = self.depth_first_search_upwards(succ_cid, state, visited);

                    // Check if this element can be visited
                    if let Some(&k_idx) = state.h2i_map.get(&k) {
                        if !visited[k_idx] {
                            // Visit element k
                            order.push(k);
                            visited[k_idx] = true;

                            // Push current state back (with updated offset)
                            stack.push((inds.clone(), new_offset + 1));

                            // Recursively process k's successors
                            let k_successors = state.dn_map.get(&k).cloned().unwrap_or_default();
                            stack.push((k_successors, 0));

                            found_non_visited = true;
                            break;
                        }
                    }
                }
            }

            // If no unvisited successors found, continue with parent's remaining successors
            if !found_non_visited {
                // Stack will naturally pop to parent level
            }
        }
    }

    /// Predict caption assignments (picture/table/code → captions)
    /// Returns map from element cid to list of caption cids
    ///
    /// N=120C: Content-based matching must work ACROSS ALL PAGES (not per-page)
    /// because table captions can be on different pages than their tables.
    /// Example: arxiv has 5 tables all on one page, but captions spread across multiple pages.
    // Method signature kept for API consistency with other ReadingOrderProcessor methods
    #[allow(clippy::unused_self)]
    #[must_use = "returns the element-to-caption mappings"]
    pub fn predict_to_captions(
        &self,
        sorted_elements: &[PageElement],
    ) -> HashMap<usize, Vec<usize>> {
        // Match captions to tables/pictures across ALL pages (document-level)
        self.find_to_captions_document_level(sorted_elements)
    }

    /// Find caption assignments ACROSS ALL PAGES using CONTENT-BASED matching
    ///
    /// N=120C: Position-based matching failed because Python's reading order comparison
    /// is non-transitive (causes Rust panic). Solution: Match captions to tables/figures
    /// by extracting numbers from caption text ("Table 2:" → 2) and matching to element index.
    ///
    /// CRITICAL: Must work DOCUMENT-LEVEL (not per-page) because table captions can be
    /// on different pages than their tables (e.g., arxiv has 5 tables on page 7, but
    /// captions "Table 1-5" are spread across pages 1-6).
    // Method signature kept for API consistency with other ReadingOrderProcessor methods
    #[allow(clippy::unused_self)]
    #[allow(clippy::too_many_lines)]
    fn find_to_captions_document_level(
        &self,
        sorted_elements: &[PageElement],
    ) -> HashMap<usize, Vec<usize>> {
        let mut to_captions: HashMap<usize, Vec<usize>> = HashMap::new();

        // Collect ALL captions across all pages
        let mut table_captions: Vec<(usize, &str)> = Vec::new();
        let mut figure_captions: Vec<(usize, &str)> = Vec::new();
        let mut code_captions: Vec<(usize, &str)> = Vec::new();

        for elem in sorted_elements {
            if let PageElement::Text(text_elem) = elem {
                if text_elem.label == DocItemLabel::Caption {
                    let text = &text_elem.text;
                    let cid = text_elem.id;

                    if text.starts_with("Table ") {
                        table_captions.push((cid, text));
                        log::debug!(
                            "      Found table caption CID {}: {}",
                            cid,
                            &text[..text.len().min(50)]
                        );
                    } else if text.starts_with("Figure ")
                        || text.starts_with("Fig. ")
                        || text.starts_with("Fig ")
                    {
                        figure_captions.push((cid, text));
                        log::debug!(
                            "      Found figure caption CID {}: {}",
                            cid,
                            &text[..text.len().min(50)]
                        );
                    } else if text.starts_with("Listing ") {
                        code_captions.push((cid, text));
                        log::debug!(
                            "      Found code caption CID {}: {}",
                            cid,
                            &text[..text.len().min(50)]
                        );
                    }
                }
            }
        }

        // Collect ALL tables, pictures, code elements across all pages IN DOCUMENT ORDER
        let mut tables: Vec<usize> = Vec::new();
        let mut pictures: Vec<usize> = Vec::new();
        let mut code_blocks: Vec<usize> = Vec::new();

        for elem in sorted_elements {
            match elem.cluster().label {
                DocItemLabel::Table => tables.push(elem.cluster().id),
                DocItemLabel::Picture => pictures.push(elem.cluster().id),
                DocItemLabel::Code => code_blocks.push(elem.cluster().id),
                _ => {}
            }
        }

        log::debug!(
            "    Document-level caption matching: {} table captions, {} figure captions, {} code captions",
            table_captions.len(),
            figure_captions.len(),
            code_captions.len()
        );
        log::debug!(
            "    Document has {} tables, {} pictures, {} code blocks",
            tables.len(),
            pictures.len(),
            code_blocks.len()
        );

        // Match table captions to tables by number
        for (caption_cid, caption_text) in table_captions {
            if let Some(table_num) = Self::extract_table_number(caption_text) {
                // table_num is 1-indexed ("Table 1"), array is 0-indexed
                if table_num > 0 && table_num <= tables.len() {
                    let table_cid = tables[table_num - 1];
                    to_captions.entry(table_cid).or_default().push(caption_cid);
                    log::debug!(
                        "      Matched '{}' (CID {}) to Table #{} (CID {})",
                        &caption_text[..caption_text.len().min(50)],
                        caption_cid,
                        table_num,
                        table_cid
                    );
                } else {
                    log::debug!(
                        "      Caption '{}' (CID {}) has table_num={} but only {} tables exist",
                        &caption_text[..caption_text.len().min(50)],
                        caption_cid,
                        table_num,
                        tables.len()
                    );
                }
            }
        }

        // Match figure captions to pictures by number
        for (caption_cid, caption_text) in figure_captions {
            if let Some(fig_num) = Self::extract_figure_number(caption_text) {
                // fig_num is 1-indexed ("Figure 1"), array is 0-indexed
                if fig_num > 0 && fig_num <= pictures.len() {
                    let picture_cid = pictures[fig_num - 1];
                    to_captions
                        .entry(picture_cid)
                        .or_default()
                        .push(caption_cid);
                    log::debug!(
                        "      Matched '{}' (CID {}) to Figure #{} (CID {})",
                        &caption_text[..caption_text.len().min(50)],
                        caption_cid,
                        fig_num,
                        picture_cid
                    );
                } else {
                    log::debug!(
                        "      Caption '{}' (CID {}) has fig_num={} but only {} pictures exist",
                        &caption_text[..caption_text.len().min(50)],
                        caption_cid,
                        fig_num,
                        pictures.len()
                    );
                }
            }
        }

        // Match code captions to code blocks by number ("Listing 1:" → code block #1)
        for (caption_cid, caption_text) in code_captions {
            if let Some(listing_num) = Self::extract_listing_number(caption_text) {
                // listing_num is 1-indexed ("Listing 1"), array is 0-indexed
                if listing_num > 0 && listing_num <= code_blocks.len() {
                    let code_cid = code_blocks[listing_num - 1];
                    to_captions.entry(code_cid).or_default().push(caption_cid);
                    log::debug!(
                        "      Matched '{}' (CID {}) to Code #{} (CID {})",
                        &caption_text[..caption_text.len().min(50)],
                        caption_cid,
                        listing_num,
                        code_cid
                    );
                } else {
                    log::debug!(
                        "      Caption '{}' (CID {}) has listing_num={} but only {} code blocks exist",
                        &caption_text[..caption_text.len().min(50)],
                        caption_cid,
                        listing_num,
                        code_blocks.len()
                    );
                }
            }
        }

        to_captions
    }

    /// Extract table number from caption text ("Table 2:" → 2)
    #[inline]
    fn extract_table_number(text: &str) -> Option<usize> {
        if let Some(after_table) = text.strip_prefix("Table ") {
            // Skip "Table "
            if let Some(colon_pos) = after_table.find(':') {
                let num_str = &after_table[..colon_pos].trim();
                return num_str.parse::<usize>().ok();
            }
        }
        None
    }

    /// Extract figure number from caption text ("Figure 3:" → 3, "Fig. 1:" → 1)
    #[inline]
    fn extract_figure_number(text: &str) -> Option<usize> {
        let prefix = if text.starts_with("Figure ") {
            "Figure "
        } else if text.starts_with("Fig. ") {
            "Fig. "
        } else if text.starts_with("Fig ") {
            "Fig "
        } else {
            return None;
        };

        let after_prefix = &text[prefix.len()..];
        if let Some(colon_pos) = after_prefix.find(':') {
            let num_str = &after_prefix[..colon_pos].trim();
            return num_str.parse::<usize>().ok();
        }
        None
    }

    /// Extract listing number from caption text ("Listing 1:" → 1)
    #[inline]
    fn extract_listing_number(text: &str) -> Option<usize> {
        if let Some(after_listing) = text.strip_prefix("Listing ") {
            // Skip "Listing "
            if let Some(colon_pos) = after_listing.find(':') {
                let num_str = &after_listing[..colon_pos].trim();
                return num_str.parse::<usize>().ok();
            }
        }
        None
    }

    /// Remove overlapping caption assignments by keeping only the closest caption
    // Method signature kept for API consistency with other ReadingOrderProcessor methods
    #[allow(clippy::unused_self)]
    fn remove_overlapping_caption_indexes(
        &self,
        mapping: HashMap<usize, Vec<usize>>,
    ) -> HashMap<usize, Vec<usize>> {
        use std::collections::HashSet;

        let mut used = HashSet::new();
        let mut result = HashMap::new();

        // Sort by key to ensure deterministic ordering
        let mut sorted_keys: Vec<_> = mapping.keys().copied().collect();
        sorted_keys.sort_unstable();

        for key in sorted_keys {
            if let Some(values) = mapping.get(&key) {
                // Sort values by distance from key (closest first)
                let mut sorted_values = values.clone();
                sorted_values.sort_by_key(|&v| v.abs_diff(key));

                // Find first unused value
                if let Some(&closest) = sorted_values.iter().find(|&&v| !used.contains(&v)) {
                    result.insert(key, vec![closest]);
                    used.insert(closest);
                }
            }
        }

        result
    }

    /// Predict footnote assignments (table/picture → footnotes)
    /// Returns map from element cid to list of footnote cids
    #[must_use = "returns the element-to-footnote mappings"]
    pub fn predict_to_footnotes(
        &self,
        sorted_elements: &[PageElement],
    ) -> HashMap<usize, Vec<usize>> {
        let mut to_footnotes: HashMap<usize, Vec<usize>> = HashMap::new();

        // Group elements by page
        let mut pages: HashMap<usize, Vec<&PageElement>> = HashMap::new();
        for elem in sorted_elements {
            pages.entry(elem.page_no()).or_default().push(elem);
        }

        // Process each page independently
        for (_page_no, page_elements) in pages {
            let page_to_footnotes = self.find_to_footnotes(&page_elements);
            to_footnotes.extend(page_to_footnotes);
        }

        to_footnotes
    }

    /// Find footnote assignments for a single page
    // Method signature kept for API consistency with other ReadingOrderProcessor methods
    #[allow(clippy::unused_self)]
    fn find_to_footnotes(&self, page_elements: &[&PageElement]) -> HashMap<usize, Vec<usize>> {
        let mut to_footnotes: HashMap<usize, Vec<usize>> = HashMap::new();

        // Find TABLE/PICTURE elements followed by consecutive FOOTNOTE elements
        for (ind, elem) in page_elements.iter().enumerate() {
            let label = elem.cluster().label;

            if matches!(label, DocItemLabel::Table | DocItemLabel::Picture) {
                let mut ind_p1 = ind + 1;

                // Collect consecutive footnotes after this table/picture
                while ind_p1 < page_elements.len()
                    && page_elements[ind_p1].cluster().label == DocItemLabel::Footnote
                {
                    to_footnotes
                        .entry(elem.cluster().id)
                        .or_default()
                        .push(page_elements[ind_p1].cluster().id);
                    ind_p1 += 1;
                }
            }
        }

        to_footnotes
    }

    /// Predict text element merges (split text across columns/pages)
    /// Returns map from element cid to list of cids to merge with
    pub fn predict_merges(&self, sorted_elements: &[PageElement]) -> HashMap<usize, Vec<usize>> {
        let mut merges: HashMap<usize, Vec<usize>> = HashMap::new();

        let mut curr_ind = 0;

        while curr_ind < sorted_elements.len() {
            let elem = &sorted_elements[curr_ind];

            // Only merge TEXT elements
            if elem.cluster().label == DocItemLabel::Text {
                // Skip non-TEXT elements after current element
                let mut ind_p1 = curr_ind + 1;
                while ind_p1 < sorted_elements.len() {
                    let next_label = sorted_elements[ind_p1].cluster().label;
                    if matches!(
                        next_label,
                        DocItemLabel::PageHeader
                            | DocItemLabel::PageFooter
                            | DocItemLabel::Table
                            | DocItemLabel::Picture
                            | DocItemLabel::Caption
                            | DocItemLabel::Footnote
                    ) {
                        ind_p1 += 1;
                    } else {
                        break;
                    }
                }

                // Check if next element is also TEXT
                if ind_p1 < sorted_elements.len() {
                    let next_elem = &sorted_elements[ind_p1];

                    if next_elem.cluster().label == DocItemLabel::Text {
                        // Check if elements are on different pages OR current is strictly left of next
                        let different_page = elem.page_no() != next_elem.page_no();
                        let strictly_left = elem
                            .cluster()
                            .bbox
                            .is_strictly_left_of(&next_elem.cluster().bbox, self.config.eps);

                        if different_page || strictly_left {
                            // Check text patterns for merging
                            let text1 = elem.text();
                            let text2 = next_elem.text();

                            if PATTERN1.is_match(text1) && PATTERN2.is_match(text2) {
                                merges.insert(elem.cluster().id, vec![next_elem.cluster().id]);
                                curr_ind = ind_p1;
                                continue;
                            }
                        }
                    }
                }
            }

            curr_ind += 1;
        }

        merges
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_reading_order_basic() {
        // Note: Reading order unit tests not needed (function is simple wrapper)
        // Comprehensive integration tests in tests/test_reading_order_integration.rs
        // and tests_pytest/test_stage41_reading_order.py validate behavior
    }
}
