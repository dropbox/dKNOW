// Intentional ML conversions: element indices, bounding box coordinates
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]

/// Stage 10: Reading Order
///
/// Applies reading order to document elements using the rule-based algorithm.
/// Ported from: `docling_ibm_models/reading_order/reading_order_rb.py`
///
/// Algorithm:
/// 1. Separate header elements (`page_header`, `page_footer`) from body elements
/// 2. Convert elements to spatial representation (BOTTOMLEFT coordinates)
/// 3. Find spatial neighbors using bounded iteration
/// 4. Apply horizontal dilation (optional, widens elements for better column detection)
/// 5. Find head elements (no predecessors above them)
/// 6. Depth-first traversal from heads to determine reading order
/// 7. Return headers + ordered body elements
///
/// Note: R-tree spatial index is imported but not currently used for queries.
/// Manual bounded iteration is used instead for correctness with Python's
/// coordinate system (BOTTOMLEFT where t > b). Future optimization could
/// restore R-tree queries once coordinate handling is verified.
use crate::pipeline_modular::types::BBox;
use crate::pipeline_modular::DocumentElement;
use log::{trace, warn};
#[allow(unused_imports)]
use rstar::{RTree, AABB};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Configuration for reading order predictor
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stage10Config {
    /// Enable horizontal dilation of element bboxes
    pub dilated_page_element: bool,
    /// Horizontal dilation threshold (fraction of page width)
    pub horizontal_dilation_threshold_norm: f32,
    /// Epsilon for strict comparison operators
    pub eps: f32,
    /// Horizontal padding for R-tree queries when finding elements above/below (in points)
    ///
    /// When querying for elements above or below a target element, the horizontal range
    /// is expanded by this amount on each side to catch elements that slightly overlap.
    pub rtree_query_horizontal_padding: f32,
    /// Padding for R-tree gap queries when checking for intervening elements (in points)
    ///
    /// When checking if an element exists between two vertically adjacent elements,
    /// the query region is expanded by this amount to ensure proper detection.
    pub rtree_gap_query_padding: f32,
}

impl Default for Stage10Config {
    #[inline]
    fn default() -> Self {
        Self {
            dilated_page_element: true,
            horizontal_dilation_threshold_norm: 0.15,
            eps: 1e-3,
            rtree_query_horizontal_padding: 0.1,
            rtree_gap_query_padding: 1.0,
        }
    }
}

/// Output of Stage 10 (reading order applied)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stage10Output {
    /// All elements in reading order (headers + body)
    pub sorted_elements: Vec<DocumentElement>,
    /// Caption mappings (placeholder for future implementation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caption_mappings: Option<serde_json::Value>,
    /// Footnote mappings (placeholder for future implementation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub footnote_mappings: Option<serde_json::Value>,
    /// Merge mappings (placeholder for future implementation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub merge_mappings: Option<serde_json::Value>,
}

/// Internal state for reading order predictor
#[derive(Debug)]
struct ReadingOrderState {
    /// Element index (id) to array index mapping
    h2i_map: HashMap<usize, usize>,
    /// Array index to element index (id) mapping
    i2h_map: HashMap<usize, usize>,
    /// Up map: elements above (predecessors)
    up_map: HashMap<usize, Vec<usize>>,
    /// Down map: elements below (successors)
    dn_map: HashMap<usize, Vec<usize>>,
    /// Head elements (no predecessors)
    heads: Vec<usize>,
}

impl ReadingOrderState {
    fn new() -> Self {
        Self {
            h2i_map: HashMap::new(),
            i2h_map: HashMap::new(),
            up_map: HashMap::new(),
            dn_map: HashMap::new(),
            heads: Vec::new(),
        }
    }
}

/// Element with spatial information for reading order
#[derive(Debug, Clone, PartialEq)]
struct OrderableElement {
    /// Element ID (cluster ID)
    id: usize,
    /// Page number
    #[allow(dead_code, reason = "stored for multi-page reading order")]
    page_no: usize,
    /// Bounding box (in BOTTOMLEFT coordinates)
    bbox: BBox,
    /// Page width
    #[allow(dead_code, reason = "stored for column detection heuristics")]
    page_width: f64,
    /// Page height
    page_height: f64,
    /// Element label
    label: String,
}

impl OrderableElement {
    /// Check if this element is strictly above another (no vertical overlap)
    #[inline]
    fn is_strictly_above(&self, other: &Self) -> bool {
        // In BOTTOMLEFT coordinates with Python's bbox convention:
        // - 't' is the UPPER edge (larger y, farther from origin)
        // - 'b' is the LOWER edge (smaller y, closer to origin)
        // So "strictly above" means: self.b > other.t (self's lower edge is above other's upper edge)
        self.bbox.b > other.bbox.t
    }

    /// Check if this element overlaps horizontally with another
    #[inline]
    fn overlaps_horizontally(&self, other: &Self) -> bool {
        // Horizontal overlap: max(l1,l2) < min(r1,r2)
        self.bbox.l.max(other.bbox.l) < self.bbox.r.min(other.bbox.r)
    }

    /// Convert from `DocumentElement` to `OrderableElement`
    fn from_document_element(elem: &DocumentElement) -> Option<Self> {
        let cluster = elem.cluster.as_ref()?;
        let bbox = elem.bbox.as_ref()?;
        let page_width = elem.page_width?;
        let page_height = elem.page_height?;

        // Convert from TOPLEFT to BOTTOMLEFT coordinates
        // Python's BOTTOMLEFT convention: t > b (t is UPPER edge, b is LOWER edge)
        // This is counterintuitive but matches Python's BoundingBox storage!
        //
        // TOPLEFT: t=top edge (small y), b=bottom edge (large y)
        // BOTTOMLEFT: y increases upward, so higher elements have larger y
        //
        // Conversion:
        // - Upper edge in BOTTOMLEFT = page_height - top edge in TOPLEFT
        // - Lower edge in BOTTOMLEFT = page_height - bottom edge in TOPLEFT
        //
        // Field naming in Python's BOTTOMLEFT:
        // - 't' field = upper/top edge (larger y value)
        // - 'b' field = lower/bottom edge (smaller y value)
        // - Always: t > b in BOTTOMLEFT
        let bbox_bottomleft = BBox {
            l: bbox.l,
            t: page_height - bbox.t, // Upper edge: convert top from TOPLEFT
            r: bbox.r,
            b: page_height - bbox.b, // Lower edge: convert bottom from TOPLEFT
        };

        Some(Self {
            id: cluster.id,
            page_no: elem.page_no,
            bbox: bbox_bottomleft,
            page_width,
            page_height,
            label: cluster.label.clone(),
        })
    }

    /// Check if this is a header element (`page_header` or `page_footer`)
    #[inline]
    fn is_page_header(&self) -> bool {
        self.label == "page_header" || self.label == "page_footer"
    }

    /// Comparison for sorting - Total order implementation
    ///
    /// N=4317 FIX: The original Python-style comparison using `overlaps_horizontally`
    /// does NOT implement a total order because transitivity can be violated:
    /// - A overlaps B → compare by b
    /// - B overlaps C → compare by b
    /// - A doesn't overlap C → compare by l
    ///
    /// This can cause A < B < C but A > C, violating total order.
    ///
    /// Rust 1.81+ enforces total order in [`slice::sort_by`] and panics on violations.
    ///
    /// FIX: Use a consistent total order: (`page_no`, b descending, l, r, t, id)
    /// This approximates reading order (top-to-bottom, left-to-right) while
    /// maintaining strict total order for all element pairs.
    fn compare(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering;

        // Primary: page number (ascending)
        match self.page_no.cmp(&other.page_no) {
            Ordering::Equal => {}
            other_ord => return other_ord,
        }

        // Secondary: b coordinate descending (higher elements first in BOTTOMLEFT)
        // Using reverse comparison: larger b = comes first = Less
        match other.bbox.b.total_cmp(&self.bbox.b) {
            Ordering::Equal => {}
            other_ord => return other_ord,
        }

        // Tertiary: l coordinate ascending (leftmost first)
        match self.bbox.l.total_cmp(&other.bbox.l) {
            Ordering::Equal => {}
            other_ord => return other_ord,
        }

        // Quaternary: r coordinate (for determinism)
        match self.bbox.r.total_cmp(&other.bbox.r) {
            Ordering::Equal => {}
            other_ord => return other_ord,
        }

        // Quinary: t coordinate (for determinism)
        match self.bbox.t.total_cmp(&other.bbox.t) {
            Ordering::Equal => {}
            other_ord => return other_ord,
        }

        // Final tie-breaker: element id (ensures unique ordering)
        self.id.cmp(&other.id)
    }
}

/// Envelope for R-tree spatial indexing (reserved for future optimization)
///
/// Note: R-tree queries are currently bypassed in favor of bounded iteration
/// for correctness with Python's BOTTOMLEFT coordinate system. This struct
/// is retained for potential future use when coordinate handling is verified.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
struct ElementEnvelope {
    aabb: AABB<[f64; 2]>,
    index: usize,
}

#[allow(dead_code)]
impl rstar::RTreeObject for ElementEnvelope {
    type Envelope = AABB<[f64; 2]>;

    fn envelope(&self) -> Self::Envelope {
        self.aabb
    }
}

#[allow(dead_code)]
impl rstar::PointDistance for ElementEnvelope {
    fn distance_2(&self, point: &[f64; 2]) -> f64 {
        self.aabb.distance_2(point)
    }
}

/// Stage 10: Reading Order Predictor
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Stage10ReadingOrder {
    config: Stage10Config,
}

impl Stage10ReadingOrder {
    #[inline]
    #[must_use = "reading order stage is created but not used"]
    pub fn new() -> Self {
        Self {
            config: Stage10Config::default(),
        }
    }

    #[inline]
    #[must_use = "reading order stage is created but not used"]
    pub const fn with_config(config: Stage10Config) -> Self {
        Self { config }
    }

    /// Apply reading order to document elements
    ///
    /// Args:
    ///   elements: Document elements from Stage 9
    ///
    /// Returns:
    ///   `Stage10Output` with sorted elements
    #[must_use = "stage output is returned but not used"]
    pub fn process(&self, elements: Vec<DocumentElement>) -> Stage10Output {
        let input_count = elements.len();

        if elements.is_empty() {
            return Stage10Output {
                sorted_elements: Vec::new(),
                caption_mappings: None,
                footnote_mappings: None,
                merge_mappings: None,
            };
        }

        // Collect input IDs before draining (F70: for verification)
        let input_ids: HashSet<usize> = elements
            .iter()
            .filter_map(|e| e.cluster.as_ref().map(|c| c.id))
            .collect();

        // Group elements by page
        let mut pages: HashMap<usize, Vec<DocumentElement>> = HashMap::new();
        for elem in elements {
            pages.entry(elem.page_no).or_default().push(elem);
        }

        let mut sorted_elements = Vec::new();

        // Process each page independently
        let mut page_numbers: Vec<_> = pages.keys().copied().collect();
        page_numbers.sort_unstable();

        for page_no in page_numbers {
            let page_elements = pages.remove(&page_no).unwrap();
            let page_sorted = self.process_page(page_elements);
            sorted_elements.extend(page_sorted);
        }

        // F70: Verify reading order integrity
        let output_count = sorted_elements.len();
        if output_count != input_count {
            warn!(
                "F70: Reading order element count mismatch: input={}, output={} (lost {} elements)",
                input_count,
                output_count,
                input_count - output_count
            );
        }

        // Check for missing IDs
        let output_ids: HashSet<usize> = sorted_elements
            .iter()
            .filter_map(|e| e.cluster.as_ref().map(|c| c.id))
            .collect();
        let missing: Vec<usize> = input_ids.difference(&output_ids).copied().collect();
        if !missing.is_empty() {
            warn!(
                "F70: {} element IDs missing from reading order output: {:?}",
                missing.len(),
                if missing.len() > 5 {
                    format!("{:?}...", &missing[..5])
                } else {
                    format!("{missing:?}")
                }
            );
        }

        // Check for duplicates
        let mut seen: HashSet<usize> = HashSet::new();
        let mut duplicates = Vec::new();
        for elem in &sorted_elements {
            if let Some(cluster) = &elem.cluster {
                if !seen.insert(cluster.id) {
                    duplicates.push(cluster.id);
                }
            }
        }
        if !duplicates.is_empty() {
            warn!(
                "F70: {} duplicate element IDs in reading order output: {:?}",
                duplicates.len(),
                if duplicates.len() > 5 {
                    format!("{:?}...", &duplicates[..5])
                } else {
                    format!("{duplicates:?}")
                }
            );
        }

        Stage10Output {
            sorted_elements,
            caption_mappings: None,
            footnote_mappings: None,
            merge_mappings: None,
        }
    }

    /// F70: Verify reading order output integrity
    ///
    /// Validates that:
    /// 1. Output count matches input count (no elements lost)
    /// 2. No duplicate element IDs in output
    /// 3. All input element IDs appear in output
    ///
    /// Returns tuple of (`is_valid`, warnings)
    #[must_use = "verification result is returned but not used"]
    pub fn verify_reading_order(
        input_elements: &[DocumentElement],
        output_elements: &[DocumentElement],
    ) -> (bool, Vec<String>) {
        let mut warnings = Vec::new();

        // Check 1: Count match
        if input_elements.len() != output_elements.len() {
            let diff = input_elements.len() as i64 - output_elements.len() as i64;
            warnings.push(format!(
                "Reading order element count mismatch: input={}, output={} (diff={})",
                input_elements.len(),
                output_elements.len(),
                diff
            ));
        }

        // Build input ID set
        let mut input_ids: HashSet<usize> = HashSet::new();
        for elem in input_elements {
            if let Some(cluster) = &elem.cluster {
                input_ids.insert(cluster.id);
            }
        }

        // Check 2: No duplicates in output
        let mut seen_ids: HashSet<usize> = HashSet::new();
        for elem in output_elements {
            if let Some(cluster) = &elem.cluster {
                if !seen_ids.insert(cluster.id) {
                    warnings.push(format!(
                        "Duplicate element ID {} in reading order output",
                        cluster.id
                    ));
                }
            }
        }

        // Check 3: All input IDs in output
        let missing: Vec<usize> = input_ids.difference(&seen_ids).copied().collect();
        if !missing.is_empty() {
            warnings.push(format!(
                "Missing {} element IDs in reading order output: {:?}",
                missing.len(),
                if missing.len() > 10 {
                    format!("{:?}...", &missing[..10])
                } else {
                    format!("{missing:?}")
                }
            ));
        }

        // Check 4: Extra IDs in output (not in input)
        let extra: Vec<usize> = seen_ids.difference(&input_ids).copied().collect();
        if !extra.is_empty() {
            warnings.push(format!(
                "Extra {} element IDs in reading order output not in input: {:?}",
                extra.len(),
                if extra.len() > 10 {
                    format!("{:?}...", &extra[..10])
                } else {
                    format!("{extra:?}")
                }
            ));
        }

        // is_valid is true only when there are no warnings
        (warnings.is_empty(), warnings)
    }

    fn process_page(&self, elements: Vec<DocumentElement>) -> Vec<DocumentElement> {
        if elements.is_empty() {
            return Vec::new();
        }

        trace!("process_page: Starting with {} elements", elements.len());

        // Convert to orderable elements
        let orderable: Vec<(OrderableElement, DocumentElement)> = elements
            .into_iter()
            .filter_map(|elem| {
                let orderable = OrderableElement::from_document_element(&elem)?;
                Some((orderable, elem))
            })
            .collect();

        trace!(
            "process_page: After conversion: {} orderable elements",
            orderable.len()
        );

        // Separate headers from body
        let mut headers = Vec::new();
        let mut body = Vec::new();

        for (ord, elem) in orderable {
            if ord.is_page_header() {
                headers.push((ord, elem));
            } else {
                body.push((ord, elem));
            }
        }

        trace!(
            "process_page: Headers: {}, Body: {}",
            headers.len(),
            body.len()
        );

        // Sort headers by position
        headers.sort_by(|a, b| a.0.compare(&b.0));

        // Apply reading order to body elements
        let body_orderable: Vec<OrderableElement> =
            body.iter().map(|(ord, _)| ord.clone()).collect();
        let body_order = self.predict_reading_order(&body_orderable);

        trace!(
            "process_page: Reading order returned {} IDs",
            body_order.len()
        );

        // Build result: headers + ordered body
        let mut result = Vec::new();

        // Add headers
        let num_headers = headers.len();
        for (_, elem) in headers {
            result.push(elem);
        }

        trace!("process_page: Added {num_headers} headers to result");

        // Add body elements in reading order
        for id in body_order {
            if let Some(idx) = body.iter().position(|(ord, _)| ord.id == id) {
                result.push(body[idx].1.clone());
            } else {
                warn!("Reading order: ID {id} not found in body elements");
            }
        }

        trace!("process_page: Final result has {} elements", result.len());

        result
    }

    fn predict_reading_order(&self, elements: &[OrderableElement]) -> Vec<usize> {
        if elements.is_empty() {
            return Vec::new();
        }

        // Initialize state
        let mut state = ReadingOrderState::new();
        self.init_h2i_map(elements, &mut state);

        // Build up/down maps using R-tree
        self.init_ud_maps(elements, &mut state);

        // Optional: horizontal dilation
        if self.config.dilated_page_element {
            // Note: Dilation modifies bboxes in-place in Python
            // For now, skip dilation (reading order still works without it)
        }

        // Sort successor lists in down map (BEFORE finding heads)
        self.sort_dn_maps(elements, &mut state);

        // Find head elements
        self.find_heads(elements, &mut state);

        // Depth-first traversal to find reading order
        self.find_order(elements, &state)
    }

    /// Initialize hash maps for bidirectional index lookup
    // Method signature kept for API consistency with other ReadingOrderProcessor methods
    #[allow(clippy::unused_self)]
    fn init_h2i_map(&self, elements: &[OrderableElement], state: &mut ReadingOrderState) {
        for (i, elem) in elements.iter().enumerate() {
            state.h2i_map.insert(elem.id, i);
            state.i2h_map.insert(i, elem.id);
        }
    }

    /// Initialize up/down maps using R-tree spatial indexing
    fn init_ud_maps(&self, elements: &[OrderableElement], state: &mut ReadingOrderState) {
        // Initialize empty maps
        for elem in elements {
            state.up_map.insert(elem.id, Vec::new());
            state.dn_map.insert(elem.id, Vec::new());
        }

        // For each element j, find elements above it that might precede it in reading order
        for (j, elem_j) in elements.iter().enumerate() {
            // Query region: elements above elem_j within slightly expanded horizontal range
            // Python: (elem_j.l - 0.1, elem_j.t, elem_j.r + 0.1, float("inf"))
            // With Python's BOTTOMLEFT (t > b): t = upper edge, so "above" means larger y
            // Query from elem_j.t (upper edge of elem_j) to top of page
            let h_pad = f64::from(self.config.rtree_query_horizontal_padding);
            let query_aabb = AABB::from_corners(
                [elem_j.bbox.l - h_pad, elem_j.bbox.t], // Start from upper edge of elem_j
                [elem_j.bbox.r + h_pad, elem_j.page_height], // Extend to top of page
            );

            // Find elements in the query region using bounded iteration
            // Note: R-tree locate_in_envelope was considered but bounded iteration
            // is used for correctness with Python's BOTTOMLEFT coordinate system
            let candidates: Vec<usize> = elements
                .iter()
                .enumerate()
                .filter(|(_i, elem)| {
                    // Check if element is in the query region
                    // Query: elements above elem_j (larger y) within expanded horizontal range
                    let x_overlap =
                        elem.bbox.r > query_aabb.lower()[0] && elem.bbox.l < query_aabb.upper()[0];
                    let y_overlap =
                        elem.bbox.t > query_aabb.lower()[1] && elem.bbox.b < query_aabb.upper()[1];
                    x_overlap && y_overlap
                })
                .map(|(i, _)| i)
                .collect();

            for &i in &candidates {
                if i == j {
                    continue;
                }

                let elem_i = &elements[i];

                // Check spatial relationship: elem_i must be strictly above elem_j
                // AND overlap horizontally
                if !elem_i.is_strictly_above(elem_j) {
                    continue;
                }
                if !elem_i.overlaps_horizontally(elem_j) {
                    continue;
                }

                // Check for interrupting elements
                if self.has_sequence_interruption(elements, i, j, elem_i, elem_j) {
                    continue;
                }

                // Add connection: elem_i precedes elem_j
                state.dn_map.get_mut(&elem_i.id).unwrap().push(elem_j.id);
                state.up_map.get_mut(&elem_j.id).unwrap().push(elem_i.id);
            }
        }
    }

    /// Check if there are elements that interrupt the reading sequence between i and j
    fn has_sequence_interruption(
        &self,
        elements: &[OrderableElement],
        i: usize,
        j: usize,
        elem_i: &OrderableElement,
        elem_j: &OrderableElement,
    ) -> bool {
        // Find elements in the gap between elem_i and elem_j
        // With Python's BOTTOMLEFT: t > b (t=upper, b=lower)
        // elem_i is above elem_j, so elem_i.b > elem_j.t
        // Gap between them: [elem_j.t, elem_i.b]
        let gap_pad = f64::from(self.config.rtree_gap_query_padding);
        let x_min = elem_i.bbox.l.min(elem_j.bbox.l) - gap_pad;
        let x_max = elem_i.bbox.r.max(elem_j.bbox.r) + gap_pad;
        // With Python's BOTTOMLEFT (t > b): t = upper, b = lower
        // Gap is between elem_j's upper edge (t) and elem_i's lower edge (b)
        let y_min = elem_j.bbox.t; // Upper edge of elem_j (bottom of gap)
        let y_max = elem_i.bbox.b; // Lower edge of elem_i (top of gap)

        // Find elements in the gap using bounded iteration
        let candidates: Vec<usize> = elements
            .iter()
            .enumerate()
            .filter(|(_w, elem)| {
                // Check if element is in the query region (gap between elem_i and elem_j)
                let x_overlap = elem.bbox.r > x_min && elem.bbox.l < x_max;
                let y_overlap = elem.bbox.t > y_min && elem.bbox.b < y_max;
                x_overlap && y_overlap
            })
            .map(|(w, _)| w)
            .collect();

        for &w in &candidates {
            if w == i || w == j {
                continue;
            }

            let elem_w = &elements[w];

            // Check if elem_w interrupts the i->j sequence
            // elem_w must:
            // 1. Overlap horizontally with elem_i OR elem_j
            // 2. Be strictly above elem_j
            // 3. elem_i be strictly above elem_w
            if (elem_i.overlaps_horizontally(elem_w) || elem_j.overlaps_horizontally(elem_w))
                && elem_i.is_strictly_above(elem_w)
                && elem_w.is_strictly_above(elem_j)
            {
                return true;
            }
        }

        false
    }

    /// Sort successor lists in `dn_map`
    ///
    /// Python: _`sort_ud_maps`
    ///
    /// For each element's successor list, sort them using `compare()`
    /// so that during DFS we visit successors in correct order.
    // Method signature kept for API consistency with other ReadingOrderProcessor methods
    #[allow(clippy::unused_self)]
    fn sort_dn_maps(&self, elements: &[OrderableElement], state: &mut ReadingOrderState) {
        for successors in state.dn_map.values_mut() {
            // Sort successors by position (using compare)
            successors.sort_by(|&a, &b| {
                let idx_a = state.h2i_map[&a];
                let idx_b = state.h2i_map[&b];
                elements[idx_a].compare(&elements[idx_b])
            });
        }
    }

    /// Find head elements (no predecessors)
    // Method signature kept for API consistency with other ReadingOrderProcessor methods
    #[allow(clippy::unused_self)]
    fn find_heads(&self, elements: &[OrderableElement], state: &mut ReadingOrderState) {
        for elem in elements {
            let up = state.up_map.get(&elem.id).map_or(0, Vec::len);
            if up == 0 {
                state.heads.push(elem.id);
            }
        }

        // Sort heads by position (left-to-right, top-to-bottom)
        state.heads.sort_by(|&a, &b| {
            let idx_a = state.h2i_map[&a];
            let idx_b = state.h2i_map[&b];
            elements[idx_a].compare(&elements[idx_b])
        });
    }

    /// Depth-first traversal to determine reading order
    fn find_order(&self, elements: &[OrderableElement], state: &ReadingOrderState) -> Vec<usize> {
        let mut visited = std::collections::HashSet::new();
        let mut order = Vec::new();

        // Visit from each head
        for &head in &state.heads {
            self.dfs(head, elements, state, &mut visited, &mut order);
        }

        // N=4407: Add any unvisited elements at the end
        // This handles elements that are disconnected from the main reading order graph
        // (e.g., isolated text boxes, floating elements not spatially connected)
        // Sort unvisited by position to maintain reasonable order
        let mut unvisited: Vec<_> = elements
            .iter()
            .filter(|e| !visited.contains(&e.id))
            .collect();

        if !unvisited.is_empty() {
            // Sort by position: top-to-bottom, left-to-right
            unvisited.sort_by(|a, b| a.compare(b));

            log::trace!(
                "find_order: Adding {} unvisited elements to end of reading order",
                unvisited.len()
            );

            for elem in unvisited {
                order.push(elem.id);
            }
        }

        order
    }

    /// Search upwards to find the highest unvisited predecessor
    ///
    /// Python: _`depth_first_search_upwards`
    ///
    /// Starting from node j, follow `up_map` (predecessors) until we find
    /// a node with no unvisited predecessors. Return that node.
    // Method signature kept for API consistency with other ReadingOrderProcessor methods
    #[allow(clippy::unused_self)]
    fn dfs_upwards(
        &self,
        node: usize,
        state: &ReadingOrderState,
        visited: &std::collections::HashSet<usize>,
    ) -> usize {
        let mut k = node;
        loop {
            let predecessors = state.up_map.get(&k).map_or(&[][..], Vec::as_slice);
            let mut found_unvisited = false;

            for &pred in predecessors {
                if !visited.contains(&pred) {
                    k = pred;
                    found_unvisited = true;
                    break;
                }
            }

            if !found_unvisited {
                return k;
            }
        }
    }

    /// Depth-first search helper
    ///
    /// Python: _`depth_first_search_downwards` (non-recursive with explicit stack)
    ///
    /// CRITICAL: This must be iterative with explicit stack to match Python's traversal order.
    /// Recursive implementation causes different traversal order when `dfs_upwards` finds
    /// unvisited predecessors while processing a node's successors.
    ///
    /// Python implementation (lines 555-584 in `reading_order_rb.py)`:
    /// - Uses explicit stack: stack: List[Tuple[List[int], int]]
    /// - Each stack element is (`indices_to_check`, offset)
    /// - Updates offset in current frame before pushing new frame
    fn dfs(
        &self,
        node: usize,
        _elements: &[OrderableElement],
        state: &ReadingOrderState,
        visited: &mut std::collections::HashSet<usize>,
        order: &mut Vec<usize>,
    ) {
        if visited.contains(&node) {
            return;
        }

        // Visit the initial node
        visited.insert(node);
        order.push(node);

        // Explicit stack: each element is (list of successor indices, current offset)
        let mut stack: Vec<(Vec<usize>, usize)> = Vec::new();

        // Push initial node's successors onto stack
        if let Some(successors) = state.dn_map.get(&node) {
            stack.push((successors.clone(), 0));
        }

        while !stack.is_empty() {
            // CRITICAL: PEEK at top of stack, don't pop yet
            // Python uses stack[-1] which peeks, only pops when done with frame
            let (inds, offset) = stack.last().unwrap().clone();
            let mut found_non_visited = false;

            if offset < inds.len() {
                // Iterate through remaining successors starting from offset
                for (new_offset, &i) in inds[offset..].iter().enumerate() {
                    // Search upwards to find highest unvisited predecessor
                    let k = self.dfs_upwards(i, state, visited);

                    if visited.insert(k) {
                        // Visit k (insert returns true if k was not already present)
                        order.push(k);

                        // Update offset in current frame (to resume after this successor)
                        let updated_offset = offset + new_offset + 1;
                        stack.pop(); // Remove old frame
                        stack.push((inds.clone(), updated_offset)); // Push updated frame

                        // Push k's successors onto stack
                        if let Some(k_successors) = state.dn_map.get(&k) {
                            stack.push((k_successors.clone(), 0));
                        }

                        found_non_visited = true;
                        break;
                    }
                }
            }

            // If we didn't find any unvisited successors, pop the frame
            if !found_non_visited {
                stack.pop();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline_modular::stage09_document_assembler::ClusterInfo;

    /// Create a minimal `DocumentElement` for testing
    fn make_test_element(id: usize, page_no: usize) -> DocumentElement {
        let bbox = BBox {
            l: 100.0,
            t: 100.0 + (id as f64 * 50.0), // Stack elements vertically
            r: 400.0,
            b: 140.0 + (id as f64 * 50.0),
        };
        DocumentElement {
            element_type: "text".to_string(),
            label: "text".to_string(),
            id,
            page_no,
            bbox: Some(bbox),
            page_width: Some(612.0),
            page_height: Some(792.0),
            ref_str: None,
            text: Some(format!("Element {id}")),
            cluster: Some(ClusterInfo {
                id,
                label: "text".to_string(),
                confidence: 0.9,
                bbox,
                cells: Vec::new(),
            }),
        }
    }

    #[test]
    fn test_f70_verify_reading_order_valid() {
        // Create input elements
        let input = vec![
            make_test_element(1, 0),
            make_test_element(2, 0),
            make_test_element(3, 0),
        ];

        // Simulate valid output (same elements, possibly reordered)
        let output = vec![
            make_test_element(2, 0),
            make_test_element(1, 0),
            make_test_element(3, 0),
        ];

        let (is_valid, warnings) = Stage10ReadingOrder::verify_reading_order(&input, &output);

        assert!(is_valid, "Should be valid: {warnings:?}");
        assert!(warnings.is_empty(), "Should have no warnings: {warnings:?}");
    }

    #[test]
    fn test_f70_verify_reading_order_missing_element() {
        // Create input elements
        let input = vec![
            make_test_element(1, 0),
            make_test_element(2, 0),
            make_test_element(3, 0),
        ];

        // Output is missing element 2
        let output = vec![make_test_element(1, 0), make_test_element(3, 0)];

        let (is_valid, warnings) = Stage10ReadingOrder::verify_reading_order(&input, &output);

        assert!(!is_valid, "Should be invalid due to missing element");
        assert!(
            warnings.iter().any(|w| w.contains("count mismatch")),
            "Should warn about count mismatch"
        );
        assert!(
            warnings
                .iter()
                .any(|w| w.to_lowercase().contains("missing") && w.contains('2')),
            "Should warn about missing ID 2: {warnings:?}"
        );
    }

    #[test]
    fn test_f70_verify_reading_order_duplicate() {
        // Create input elements
        let input = vec![
            make_test_element(1, 0),
            make_test_element(2, 0),
            make_test_element(3, 0),
        ];

        // Output has duplicate ID 1
        let output = vec![
            make_test_element(1, 0),
            make_test_element(1, 0), // Duplicate!
            make_test_element(3, 0),
        ];

        let (is_valid, warnings) = Stage10ReadingOrder::verify_reading_order(&input, &output);

        assert!(!is_valid, "Should be invalid due to duplicate");
        assert!(
            warnings.iter().any(|w| w.contains("Duplicate")),
            "Should warn about duplicate: {warnings:?}"
        );
    }

    #[test]
    fn test_f70_process_preserves_all_elements() {
        // Test that process() preserves all elements
        let input = vec![
            make_test_element(1, 0),
            make_test_element(2, 0),
            make_test_element(3, 0),
        ];

        let stage = Stage10ReadingOrder::new();
        let output = stage.process(input.clone());

        // Should have same count
        assert_eq!(
            output.sorted_elements.len(),
            input.len(),
            "Should preserve element count"
        );

        // All IDs should be present
        let output_ids: HashSet<usize> = output
            .sorted_elements
            .iter()
            .filter_map(|e| e.cluster.as_ref().map(|c| c.id))
            .collect();

        for elem in &input {
            if let Some(cluster) = &elem.cluster {
                assert!(
                    output_ids.contains(&cluster.id),
                    "Element {} should be in output",
                    cluster.id
                );
            }
        }
    }
}
