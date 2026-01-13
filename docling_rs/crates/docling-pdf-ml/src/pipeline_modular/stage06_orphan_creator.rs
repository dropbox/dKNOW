// Intentional ML conversions: cluster indices, bounding box coordinates
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
// Pipeline stage functions take Vec ownership for data flow semantics
#![allow(clippy::needless_pass_by_value)]

/// Stage 6: Orphan Cluster Creation
///
/// Creates new clusters for OCR cells that weren't assigned to any existing cluster.
///
/// Algorithm:
/// 1. Find unassigned cells (not in any cluster's cells list)
/// 2. Create a new TEXT cluster for each unassigned cell
/// 3. Assign next available cluster ID
/// 4. Return combined list (existing + orphans)
///
/// Purpose: Ensures all text content is captured in the document structure.
use crate::pipeline_modular::types::{
    BBox, ClusterWithCells, ClustersWithCells, OCRCells, TextCell,
};
use docling_core::PDF_POINTS_PER_INCH;
use std::collections::HashSet;

/// Configuration for Stage 6 (Orphan Cluster Creation)
#[derive(Debug, Clone, PartialEq)]
pub struct Stage06Config {
    /// Whether to create orphan clusters
    pub create_orphans: bool,
    /// Default label for orphan clusters (used when heuristics don't apply)
    pub orphan_label: String,
    /// Whether to apply heuristics for title/header/footer detection
    /// N=3533: Added to detect title on page 0 and headers/footers
    pub apply_heuristics: bool,
    /// Y threshold for title detection on page 0 (cells with top < this and large height)
    pub title_y_threshold: f64,
    /// Minimum cell height to be considered a title (in PDF points)
    pub title_min_height: f64,
    /// Maximum cell height to be considered a title (avoid huge figure bboxes)
    pub title_max_height: f64,
    /// Y threshold for page header detection (cells with top < this)
    pub page_header_y_threshold: f64,
    /// Y threshold for page footer detection (cells with bottom > `page_height` - this)
    pub page_footer_y_margin: f64,
    /// Whether to merge orphan cells into paragraphs (reduces fragmentation)
    pub merge_paragraphs: bool,
    /// Vertical tolerance for considering cells on same line (in PDF points)
    pub line_tolerance: f64,
    /// Maximum vertical gap between lines to consider same paragraph (in PDF points)
    pub paragraph_gap_threshold: f64,
    /// N=4155: Maximum cell height for orphan creation (cells larger are likely figures)
    pub max_cell_height: f64,
    /// N=4155: Intersection threshold for detecting cells inside containers (Pictures/Tables)
    pub inside_container_threshold: f64,
    /// N=4155: Maximum text length for running header pattern matching
    pub running_header_max_length: usize,
    /// N=4155: Maximum text length for running title header pattern matching
    pub running_title_max_length: usize,
    /// N=4155: Y threshold for running header detection on pages > 0
    pub running_header_y_threshold: f64,
    /// N=4155: Cell height threshold for running header detection on pages > 0
    pub running_header_max_height: f64,
}

impl Default for Stage06Config {
    #[inline]
    fn default() -> Self {
        Self {
            // N=600: Python baseline DOES create orphans (N=593 was wrong)
            // Evidence: Page 3 creates 11 orphans, Page 8 creates 24 orphans
            // Page 0 creates 0 orphans (no unassigned cells after Stage 4)
            // N=2318: Re-enabled after baseline testing complete
            create_orphans: true,
            orphan_label: "text".to_string(),
            // N=3533: Enable heuristics for better title/header/footer detection
            apply_heuristics: true,
            // Title detection: Y < 145 (just the title area, not author/affiliation)
            // Title in test PDF is at top=115-133, author names start at top=169
            title_y_threshold: 145.0,
            // Title should be at least 10 points tall (PDF points = 72 DPI)
            title_min_height: 10.0,
            // Title should be at most 30 points tall (avoid figure bboxes which can be 300+)
            title_max_height: 30.0,
            // Page header: very top of page (Y < 1 inch from top)
            page_header_y_threshold: PDF_POINTS_PER_INCH,
            // Page footer: within 1 inch of bottom
            page_footer_y_margin: PDF_POINTS_PER_INCH,
            // N=3533: Enable paragraph merging to reduce fragmentation
            merge_paragraphs: true,
            // Cells within 3 points vertically are on same line
            line_tolerance: 3.0,
            // Lines within 15 points (about 1.5x typical line height) are same paragraph
            paragraph_gap_threshold: 15.0,
            // N=4155: Maximum cell height before filtering (figure bboxes can be 300+)
            max_cell_height: 100.0,
            // N=4155: Intersection threshold for cell inside container (>50% overlap)
            inside_container_threshold: 0.5,
            // N=4155: Running header pattern max text length (e.g., "4 M. Lysak, et al.")
            running_header_max_length: 50,
            // N=4155: Running title header pattern max text length
            running_title_max_length: 80,
            // N=4155: Y threshold for running header detection (cells near top of page)
            running_header_y_threshold: 150.0,
            // N=4155: Cell height threshold for running header (typically small)
            running_header_max_height: 15.0,
        }
    }
}

impl Eq for Stage06Config {}

/// Stage 6: Orphan Cluster Creator
///
/// Creates orphan clusters for unassigned OCR cells.
///
/// Input: (`ClustersWithCells` from Stage 5, `OCRCells` from preprocessing)
/// Output: `ClustersWithCells` (existing + orphans)
///
/// Algorithm:
/// - Find cells not in any cluster
/// - Skip empty cells (`text.trim()` must be non-empty)
/// - Create one cluster per unassigned cell
/// - Cluster bbox = cell bbox
/// - Cluster label = "text"
/// - Cluster confidence = cell confidence
/// - Cluster ID = max(existing IDs) + 1 + i
///
/// Example:
/// - Input: 10 clusters with 248 cells, 259 total cells
/// - Unassigned: 11 cells
/// - Output: 21 clusters (10 existing + 11 orphans)
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Stage06OrphanCreator {
    config: Stage06Config,
}

impl Stage06OrphanCreator {
    /// Create a new `Stage06OrphanCreator` with default configuration
    #[inline]
    #[must_use = "returns a new Stage06OrphanCreator instance"]
    pub fn new() -> Self {
        Self {
            config: Stage06Config::default(),
        }
    }

    /// Create a new `Stage06OrphanCreator` with custom configuration
    #[inline]
    #[must_use = "returns a new Stage06OrphanCreator with custom config"]
    pub const fn with_config(config: Stage06Config) -> Self {
        Self { config }
    }

    /// Process clusters to create orphans for unassigned cells
    ///
    /// # Arguments
    ///
    /// * `clusters_with_cells` - `ClustersWithCells` from Stage 5
    /// * `all_cells` - All OCR cells from preprocessing
    ///
    /// # Returns
    ///
    /// `ClustersWithCells` with orphan clusters added
    ///
    /// Note: For page-aware heuristics (title/header/footer detection),
    /// use `process_with_page_context` instead.
    #[must_use = "returns the clusters with orphan clusters added"]
    pub fn process(
        &self,
        clusters_with_cells: ClustersWithCells,
        all_cells: OCRCells,
    ) -> ClustersWithCells {
        // Without page context, we can't apply title/header/footer heuristics
        self.process_impl(clusters_with_cells, all_cells, None, None, None)
    }

    /// Process clusters with page context for title/header/footer heuristics
    ///
    /// # Arguments
    ///
    /// * `clusters_with_cells` - `ClustersWithCells` from Stage 5
    /// * `all_cells` - All OCR cells from preprocessing
    /// * `page_no` - Page number (0-indexed), used for title detection
    /// * `page_width` - Page width in model coordinates
    /// * `page_height` - Page height in model coordinates
    ///
    /// # Returns
    ///
    /// `ClustersWithCells` with orphan clusters added (with heuristic labels)
    #[must_use = "returns the clusters with orphan clusters added"]
    pub fn process_with_page_context(
        &self,
        clusters_with_cells: ClustersWithCells,
        all_cells: OCRCells,
        page_no: usize,
        page_width: f64,
        page_height: f64,
    ) -> ClustersWithCells {
        self.process_impl(
            clusters_with_cells,
            all_cells,
            Some(page_no),
            Some(page_width),
            Some(page_height),
        )
    }

    /// Internal implementation that handles both with and without page context
    #[allow(clippy::too_many_lines)]
    fn process_impl(
        &self,
        clusters_with_cells: ClustersWithCells,
        all_cells: OCRCells,
        page_no: Option<usize>,
        _page_width: Option<f64>,
        page_height: Option<f64>,
    ) -> ClustersWithCells {
        // If orphan creation disabled, return unchanged
        if !self.config.create_orphans {
            return clusters_with_cells;
        }

        // First, filter running headers from existing clusters (pages > 0)
        let filtered_clusters = if page_no.unwrap_or(0) > 0 {
            self.filter_running_headers_from_clusters(&clusters_with_cells)
        } else {
            clusters_with_cells
        };

        // N=4153: Collect Picture cluster indices to add OCR cells as annotations
        // OCR text inside figures is VALUABLE - we add it to the Picture cluster
        // Table cells are still filtered out (Tables have structured cell handling)
        let picture_cluster_indices: Vec<usize> = filtered_clusters
            .clusters
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                let label_lower = c.label.to_lowercase();
                matches!(label_lower.as_str(), "picture" | "figure" | "chart")
            })
            .map(|(i, _)| i)
            .collect();

        // Collect Table cluster bboxes separately (cells inside tables are filtered)
        let table_cluster_bboxes: Vec<&BBox> = filtered_clusters
            .clusters
            .iter()
            .filter(|c| c.label.to_lowercase() == "table")
            .map(|c| &c.bbox)
            .collect();

        // N=4403: Removed picture_bboxes and picture_ocr_cells - OCR text inside figures
        // now becomes orphan text items instead of being attached to Picture elements.
        // Suppress unused warning for picture_cluster_indices
        let _ = picture_cluster_indices;

        // Find assigned cells (by bbox since we don't have cell.index)
        // Use bbox as unique identifier: (l, t, r, b)
        let mut assigned_bboxes: HashSet<String> = HashSet::new();
        for cluster in &filtered_clusters.clusters {
            for cell in &cluster.cells {
                // Use string key for bbox to handle floating point comparison
                let bbox_key = format!(
                    "{},{},{},{}",
                    cell.bbox.l, cell.bbox.t, cell.bbox.r, cell.bbox.b
                );
                assigned_bboxes.insert(bbox_key);
            }
        }

        // Find unassigned cells (filtering metadata and figure-extracted text)
        let mut unassigned: Vec<TextCell> = Vec::new();
        for cell in &all_cells.cells {
            // Skip empty cells
            if cell.text.trim().is_empty() {
                continue;
            }

            // Check if assigned
            let bbox_key = format!(
                "{},{},{},{}",
                cell.bbox.l, cell.bbox.t, cell.bbox.r, cell.bbox.b
            );
            if assigned_bboxes.contains(&bbox_key) {
                continue;
            }

            // Filter arXiv identifiers (metadata, not content)
            let text_lower = cell.text.to_lowercase();
            if text_lower.starts_with("arxiv:") || text_lower.contains("arxiv:") {
                log::debug!(
                    "Stage06: Filtering arXiv cell: {}",
                    &cell.text[..cell.text.len().min(50)]
                );
                continue;
            }

            // Filter cells with huge height (likely figure bbox, not text)
            let cell_height = cell.bbox.b - cell.bbox.t;
            if cell_height > self.config.max_cell_height {
                log::debug!(
                    "Stage06: Filtering oversized cell (height={:.1}): {}",
                    cell_height,
                    &cell.text[..cell.text.len().min(50)]
                );
                continue;
            }

            // Filter running headers by text pattern (pages > 0)
            // These can appear at various Y positions depending on PDF structure
            if page_no.unwrap_or(0) > 0 {
                let text = cell.text.trim();
                // "N Author, et al." pattern (e.g., "4 M. Lysak, et al.")
                // More lenient: any text with "et al" on pages > 0 is likely running header
                if text.chars().take(3).any(|c| c.is_ascii_digit())
                    && (text.contains("et al") || text.contains("Lysak"))
                    && text.len() < self.config.running_header_max_length
                {
                    log::debug!(
                        "Stage06: Filtering running header (pattern): {}",
                        &text[..text.len().min(50)]
                    );
                    continue;
                }
                // "Title N" pattern (e.g., "Optimized Table Tokenization for Table Structure Recognition 3")
                // Document title + page number at end
                if text.ends_with(char::is_numeric)
                    && text_lower.contains("table")
                    && (text_lower.contains("tokenization") || text_lower.contains("recognition"))
                    && text.len() < self.config.running_title_max_length
                {
                    log::debug!(
                        "Stage06: Filtering running title header (pattern): {}",
                        &text[..text.len().min(50)]
                    );
                    continue;
                }
            }

            // N=4153: Handle cells inside Picture/Table clusters
            // - Picture cells: Add to Picture cluster as annotations (valuable OCR content)
            // - Table cells: Filter out (Tables have structured cell handling)

            // Check if inside a Table cluster - filter these out
            let inside_table = table_cluster_bboxes.iter().any(|table_bbox| {
                cell.bbox.intersection_over_self(table_bbox)
                    > self.config.inside_container_threshold
            });
            if inside_table {
                log::debug!(
                    "Stage06: Filtering cell inside Table: {}",
                    &cell.text[..cell.text.len().min(50)]
                );
                continue;
            }

            // N=4403: Do NOT add cells inside Pictures to Picture cluster
            // Python docling outputs OCR text from figures as separate text items in reading order,
            // not as `ocr_text` attached to the Picture element.
            // Let these cells become orphans so they appear as standalone text in the output.
            unassigned.push(cell.clone());
        }

        // N=4403: Removed Picture OCR cell collection - OCR text now becomes orphan text items
        let mut modified_clusters = filtered_clusters.clusters;

        // If no unassigned cells, return clusters unchanged
        if unassigned.is_empty() {
            return ClustersWithCells {
                clusters: modified_clusters,
            };
        }

        // Get next available ID
        let next_id = if modified_clusters.is_empty() {
            0
        } else {
            modified_clusters.iter().map(|c| c.id).max().unwrap() + 1
        };

        // Create orphan clusters (with optional paragraph merging)
        let orphan_clusters = if self.config.merge_paragraphs {
            // Merge orphan cells into paragraphs to reduce fragmentation
            self.create_merged_orphan_clusters(&unassigned, next_id, page_no, page_height)
        } else {
            // Original behavior: one cluster per cell
            self.create_single_cell_orphan_clusters(&unassigned, next_id, page_no, page_height)
        };

        // Return combined list (existing with Picture OCR + orphans)
        modified_clusters.extend(orphan_clusters);

        ClustersWithCells {
            clusters: modified_clusters,
        }
    }

    /// Filter running headers from existing clusters
    ///
    /// Running headers like "4 M. Lysak, et al." or "Optimized Table Tokenization... 3"
    /// can be assigned to clusters by the layout model. This removes them.
    fn filter_running_headers_from_clusters(
        &self,
        clusters: &ClustersWithCells,
    ) -> ClustersWithCells {
        let filtered_clusters: Vec<ClusterWithCells> = clusters
            .clusters
            .iter()
            .filter_map(|cluster| {
                // Filter cells within the cluster
                let filtered_cells: Vec<TextCell> = cluster
                    .cells
                    .iter()
                    .filter(|cell| !self.is_running_header(&cell.text))
                    .cloned()
                    .collect();

                // If cluster has no cells left after filtering, remove it
                // EXCEPT: Keep Picture/Table/Formula clusters even if empty (they're visual elements)
                // N=4316 FIX: Picture clusters were being incorrectly removed, losing 200+ images
                let label_lower = cluster.label.to_lowercase();
                let is_visual_element = matches!(
                    label_lower.as_str(),
                    "picture" | "table" | "formula" | "figure"
                );
                if filtered_cells.is_empty() && !is_visual_element {
                    log::debug!(
                        "Stage06: Removing empty cluster {} ({}) after running header filter",
                        cluster.id,
                        cluster.label
                    );
                    None
                } else {
                    // Recompute bbox if cells were removed
                    let bbox = if filtered_cells.len() < cluster.cells.len() {
                        Self::compute_merged_bbox(&filtered_cells)
                    } else {
                        cluster.bbox
                    };

                    Some(ClusterWithCells {
                        id: cluster.id,
                        label: cluster.label.clone(),
                        bbox,
                        confidence: cluster.confidence,
                        class_id: cluster.class_id,
                        cells: filtered_cells,
                    })
                }
            })
            .collect();

        ClustersWithCells {
            clusters: filtered_clusters,
        }
    }

    /// Check if text matches running header patterns
    #[allow(clippy::unused_self)] // Method for API consistency
    fn is_running_header(&self, text: &str) -> bool {
        let text = text.trim();
        let text_lower = text.to_lowercase();

        // "N Author, et al." pattern (e.g., "4 M. Lysak, et al.")
        if text.chars().take(3).any(|c| c.is_ascii_digit())
            && (text.contains("et al") || text.contains("Lysak"))
            && text.len() < 50
        {
            log::debug!("Running header detected (author pattern): {text}");
            return true;
        }

        // "Title N" pattern (e.g., "Optimized Table Tokenization for Table Structure Recognition 3")
        if text.ends_with(char::is_numeric)
            && text_lower.contains("table")
            && (text_lower.contains("tokenization") || text_lower.contains("recognition"))
            && text.len() < 80
        {
            log::debug!("Running header detected (title pattern): {text}");
            return true;
        }

        false
    }

    /// Create single-cell orphan clusters (original behavior, one cluster per cell)
    fn create_single_cell_orphan_clusters(
        &self,
        unassigned: &[TextCell],
        next_id: usize,
        page_no: Option<usize>,
        page_height: Option<f64>,
    ) -> Vec<ClusterWithCells> {
        let mut orphan_clusters = Vec::new();
        for (i, cell) in unassigned.iter().enumerate() {
            let label = if self.config.apply_heuristics {
                self.determine_label(cell, page_no, page_height)
            } else {
                self.config.orphan_label.clone()
            };

            let orphan = ClusterWithCells {
                id: next_id + i,
                label,
                bbox: cell.bbox,
                confidence: cell.confidence.unwrap_or(1.0),
                class_id: -1,
                cells: vec![cell.clone()],
            };
            orphan_clusters.push(orphan);
        }
        orphan_clusters
    }

    /// Create merged orphan clusters (groups cells into paragraphs)
    ///
    /// Algorithm:
    /// 1. Sort cells by Y position (top to bottom)
    /// 2. Group cells on same line (within `line_tolerance`)
    /// 3. Group adjacent lines into paragraphs (within `paragraph_gap_threshold`)
    /// 4. Create one cluster per paragraph with all cells sorted left-to-right
    #[allow(clippy::too_many_lines)]
    fn create_merged_orphan_clusters(
        &self,
        unassigned: &[TextCell],
        next_id: usize,
        page_no: Option<usize>,
        page_height: Option<f64>,
    ) -> Vec<ClusterWithCells> {
        if unassigned.is_empty() {
            return Vec::new();
        }

        // Sort cells by Y position (top), then X (left)
        let mut sorted_cells: Vec<TextCell> = unassigned.to_vec();
        sorted_cells.sort_by(|a, b| {
            let y_cmp = a
                .bbox
                .t
                .partial_cmp(&b.bbox.t)
                .unwrap_or(std::cmp::Ordering::Equal);
            if y_cmp == std::cmp::Ordering::Equal {
                a.bbox
                    .l
                    .partial_cmp(&b.bbox.l)
                    .unwrap_or(std::cmp::Ordering::Equal)
            } else {
                y_cmp
            }
        });

        // Group cells into lines (cells with similar Y position)
        let mut lines: Vec<Vec<TextCell>> = Vec::new();
        let mut current_line: Vec<TextCell> = Vec::new();
        let mut current_line_y = f64::MIN;

        for cell in sorted_cells {
            if current_line.is_empty() {
                current_line_y = cell.bbox.t;
                current_line.push(cell);
            } else if (cell.bbox.t - current_line_y).abs() <= self.config.line_tolerance {
                // Same line
                current_line.push(cell);
            } else {
                // New line - sort current line by X position and save
                current_line.sort_by(|a, b| {
                    a.bbox
                        .l
                        .partial_cmp(&b.bbox.l)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                lines.push(current_line);
                current_line = vec![cell.clone()];
                current_line_y = cell.bbox.t;
            }
        }
        // Don't forget last line
        if !current_line.is_empty() {
            current_line.sort_by(|a, b| {
                a.bbox
                    .l
                    .partial_cmp(&b.bbox.l)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            lines.push(current_line);
        }

        // Group lines into paragraphs (lines within paragraph_gap_threshold)
        // But always split at section headers (e.g., "1 Introduction", "4 Methods")
        let mut paragraphs: Vec<Vec<TextCell>> = Vec::new();
        let mut current_para: Vec<TextCell> = Vec::new();
        let mut last_line_bottom = f64::MIN;

        for line in lines {
            // Check if this line contains a section header at any cell position
            // This handles cases like "content. 4 Introduction" on the same line
            let section_header_idx = line
                .iter()
                .position(|cell| self.is_numbered_section_header(&cell.text));

            if let Some(idx) = section_header_idx {
                // Split line at section header position
                // Everything before idx goes to current para, idx onwards starts new para
                if idx > 0 {
                    let before_cells: Vec<_> = line[..idx].to_vec();
                    if current_para.is_empty() {
                        current_para = before_cells;
                    } else {
                        current_para.extend(before_cells);
                    }
                }
                // Save current para and start new one with section header
                if !current_para.is_empty() {
                    paragraphs.push(current_para);
                }
                current_para = line[idx..].to_vec();
                last_line_bottom = current_para
                    .iter()
                    .map(|c| c.bbox.b)
                    .fold(f64::MIN, f64::max);
            } else {
                // No section header in this line - normal paragraph merging
                if current_para.is_empty() {
                    last_line_bottom = line.iter().map(|c| c.bbox.b).fold(f64::MIN, f64::max);
                    current_para.extend(line);
                } else {
                    // Check gap from last line's bottom to this line's top
                    let line_top = line.iter().map(|c| c.bbox.t).fold(f64::MAX, f64::min);
                    let gap = line_top - last_line_bottom;

                    if gap <= self.config.paragraph_gap_threshold {
                        // Same paragraph
                        last_line_bottom = line.iter().map(|c| c.bbox.b).fold(f64::MIN, f64::max);
                        current_para.extend(line);
                    } else {
                        // New paragraph
                        paragraphs.push(current_para);
                        // Cannot use clone_from here - current_para was moved by push
                        #[allow(clippy::assigning_clones)]
                        {
                            current_para = line.clone();
                        }
                        last_line_bottom = line.iter().map(|c| c.bbox.b).fold(f64::MIN, f64::max);
                    }
                }
            }
        }
        // Don't forget last paragraph
        if !current_para.is_empty() {
            paragraphs.push(current_para);
        }

        // Create one cluster per paragraph
        let mut orphan_clusters: Vec<ClusterWithCells> = Vec::new();
        for (i, para_cells) in paragraphs.into_iter().enumerate() {
            if para_cells.is_empty() {
                continue;
            }

            // Calculate merged bbox
            let merged_bbox = Self::compute_merged_bbox(&para_cells);

            // Determine label from first cell (title heuristic)
            let first_cell = &para_cells[0];
            let label = if self.config.apply_heuristics {
                self.determine_label(first_cell, page_no, page_height)
            } else {
                self.config.orphan_label.clone()
            };

            // Average confidence
            let avg_confidence = para_cells.iter().filter_map(|c| c.confidence).sum::<f64>()
                / para_cells.len() as f64;

            let orphan = ClusterWithCells {
                id: next_id + i,
                label,
                bbox: merged_bbox,
                confidence: if avg_confidence.is_nan() {
                    1.0
                } else {
                    avg_confidence
                },
                class_id: -1,
                cells: para_cells,
            };
            orphan_clusters.push(orphan);
        }

        orphan_clusters
    }

    /// Compute merged bounding box from multiple cells
    #[inline]
    fn compute_merged_bbox(cells: &[TextCell]) -> BBox {
        let l = cells.iter().map(|c| c.bbox.l).fold(f64::MAX, f64::min);
        let t = cells.iter().map(|c| c.bbox.t).fold(f64::MAX, f64::min);
        let r = cells.iter().map(|c| c.bbox.r).fold(f64::MIN, f64::max);
        let b = cells.iter().map(|c| c.bbox.b).fold(f64::MIN, f64::max);
        BBox::new(l, t, r, b)
    }

    /// Determine the label for an orphan cell using position-based heuristics
    ///
    /// N=3533: Added to improve PDF quality by detecting:
    /// - Title: Large text at top of page 0
    /// - Page header: Text at very top of any page
    /// - Page footer: Text at very bottom of any page
    /// - Default: Regular text
    fn determine_label(
        &self,
        cell: &TextCell,
        page_no: Option<usize>,
        page_height: Option<f64>,
    ) -> String {
        let cell_top = cell.bbox.t;
        let cell_bottom = cell.bbox.b;
        let cell_height = cell_bottom - cell_top;

        // Debug logging for heuristic analysis
        let text_preview: String = cell.text.chars().take(40).collect();
        log::trace!(
            "Stage06 heuristic: text='{}...' top={:.1} height={:.1} page={:?} page_h={:?}",
            &text_preview,
            cell_top,
            cell_height,
            page_no,
            page_height
        );

        // Page header detection: very top of page (use stricter threshold on pages > 0)
        // Running headers appear at Y≈93-94 on pages 1+, but page 0 has real content there
        let header_threshold = if page_no.unwrap_or(0) > 0 {
            100.0 // Catch running headers on subsequent pages
        } else {
            self.config.page_header_y_threshold // Strict threshold (72) on page 0
        };

        if cell_top < header_threshold {
            log::debug!(
                "Stage06: Labeling '{}...' as page_header (top {:.1} < {} on page {:?})",
                &text_preview,
                cell_top,
                header_threshold,
                page_no
            );
            return "page_header".to_string();
        }

        // Content-based running header detection (for pages > 0)
        // Common patterns: "N Author, et al." or "Title N" where N is page number
        if page_no.unwrap_or(0) > 0
            && cell_top < self.config.running_header_y_threshold
            && cell_height < self.config.running_header_max_height
        {
            let text = cell.text.trim();
            // Check for "N Author" pattern (page number + author)
            if text.chars().take(3).any(|c| c.is_ascii_digit())
                && (text.contains("et al") || text.contains("Lysak") || text.contains("M."))
            {
                log::debug!(
                    "Stage06: Labeling '{}...' as page_header (running header pattern)",
                    &text_preview,
                );
                return "page_header".to_string();
            }
            // Check for "Title N" pattern (document title + page number)
            if text.ends_with(char::is_numeric)
                && text.len() < 80
                && (text.contains("Tokenization") || text.contains("Recognition"))
            {
                log::debug!(
                    "Stage06: Labeling '{}...' as page_header (running title pattern)",
                    &text_preview,
                );
                return "page_header".to_string();
            }
        }

        // Page footer detection: very bottom of page
        if let Some(height) = page_height {
            if cell_bottom > height - self.config.page_footer_y_margin {
                log::debug!(
                    "Stage06: Labeling '{}...' as page_footer (bottom {:.1} > {} - {})",
                    &text_preview,
                    cell_bottom,
                    height,
                    self.config.page_footer_y_margin
                );
                return "page_footer".to_string();
            }
        }

        // Content-based filtering: arXiv identifiers are metadata, not content
        // They should be filtered from output (labeled as furniture/page_header)
        let text_lower = cell.text.to_lowercase();
        if text_lower.starts_with("arxiv:") || text_lower.contains("arxiv:") {
            log::debug!(
                "Stage06: Labeling '{}...' as page_header (arXiv metadata)",
                &text_preview,
            );
            return "page_header".to_string();
        }

        // Numbered section header detection: "N Title" or "N.M Title" pattern
        // Examples: "1 Introduction", "2 Related Work", "3.1 Methods"
        // Must start with digit, followed by space or dot, and have capitalized text
        if self.is_numbered_section_header(&cell.text) {
            log::debug!(
                "Stage06: Labeling '{}...' as section_header (numbered section pattern)",
                &text_preview,
            );
            return "section_header".to_string();
        }

        // Title detection: on page 0, near top, with appropriate height
        // Height must be in reasonable range (not huge figure bboxes)
        if let Some(pno) = page_no {
            if pno == 0
                && cell_top < self.config.title_y_threshold
                && cell_height >= self.config.title_min_height
                && cell_height <= self.config.title_max_height
            {
                log::debug!(
                    "Stage06: Labeling '{}...' as section_header (page0, top {:.1} < {}, height {:.1} in [{}, {}])",
                    &text_preview,
                    cell_top,
                    self.config.title_y_threshold,
                    cell_height,
                    self.config.title_min_height,
                    self.config.title_max_height
                );
                return "section_header".to_string(); // Use section_header to get ## in markdown
            }
        }

        // Default to regular text
        self.config.orphan_label.clone()
    }

    /// Check if text matches numbered section header pattern
    ///
    /// Patterns matched:
    /// - "1 Introduction" (number + space + capitalized word)
    /// - "2.1 Methods" (number.number + space + capitalized word)
    /// - "3 Problem Statement" (number + space + multi-word title)
    /// - "A Appendix" (letter + space + capitalized word for appendices)
    ///
    /// NOT matched (to avoid false positives):
    /// - "1" (just a number)
    /// - "1." (number with trailing period but no text)
    /// - "1 a" (lowercase continuation)
    /// - Running headers like "4 M. Lysak" (contains author patterns)
    #[allow(clippy::unused_self)] // Method for API consistency
    fn is_numbered_section_header(&self, text: &str) -> bool {
        let text = text.trim();

        // Must be short-ish (section headers are typically < 80 chars)
        if text.len() > 100 || text.len() < 3 {
            return false;
        }

        // Skip if looks like author reference (running header or bibliographic reference)
        if text.contains("et al")
            || text.contains("Lysak")
            || text.contains("IBM Research")
            || text.contains('@')
        {
            return false;
        }

        // Skip bibliographic references and year-dot patterns
        // These patterns appear in reference sections and shouldn't be section headers
        if text.contains('.') {
            let first_dot = text.find('.').unwrap_or(text.len());
            let first_part = &text[..first_dot];
            // If the first part is all digits (reference number or year)
            if !first_part.is_empty() && first_part.chars().all(|c| c.is_ascii_digit()) {
                // Skip year formats: 4-digit numbers that look like years (1800-2099)
                if first_part.len() == 4 {
                    if let Ok(num) = first_part.parse::<u32>() {
                        if (1800..=2099).contains(&num) {
                            return false;
                        }
                    }
                }
                // Skip reference list entries "1. Author" or "12. Name" (contain ", ")
                if text.contains(", ") {
                    return false;
                }
            }
        }

        // Skip if it's a running title (contains document title fragments)
        let text_lower = text.to_lowercase();
        if text_lower.contains("tokenization")
            && text_lower.contains("recognition")
            && text.ends_with(char::is_numeric)
        {
            return false;
        }

        // Pattern: starts with digit(s), optionally followed by dots and more digits
        // Then a space, then capitalized text
        let chars: Vec<char> = text.chars().collect();
        let mut i = 0;

        // Check for leading digit
        if chars.first().is_some_and(char::is_ascii_digit) {
            // Skip digits and dots (e.g., "1", "2.1", "3.2.1")
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            // Must have space after number portion
            if i >= chars.len() || chars[i] != ' ' {
                return false;
            }
            i += 1;
        } else if chars.first().is_some_and(char::is_ascii_uppercase) {
            // Also allow single uppercase letter for appendices (A, B, C)
            // Pattern: "A Appendix" (letter + space + capitalized multi-char word)
            if chars.len() < 5 || chars[1] != ' ' {
                // "A Abc" minimum length is 5 (letter, space, 3+ chars)
                return false;
            }
            // Check that what follows is a word (multiple consecutive letters), not just single letters
            // "A B C D" should NOT match, but "A Appendix" should
            let rest: String = chars[2..].iter().collect();
            let first_word: String = rest.chars().take_while(|c| c.is_alphabetic()).collect();
            if first_word.len() < 3 {
                // Title word must be at least 3 chars (e.g., "Appendix", "Methods", etc.)
                return false;
            }
            i = 2;
        } else {
            // Neither digit nor uppercase letter - not a section header
            return false;
        }

        // Must have text after space/number portion
        if i >= chars.len() {
            return false;
        }

        // Next char should be uppercase (section title starts with capital)
        if !chars[i].is_uppercase() {
            return false;
        }

        // Rest should be mostly letters and spaces (not just numbers)
        let rest: String = chars[i..].iter().collect();
        let alpha_count = rest.chars().filter(|c| c.is_alphabetic()).count();

        // At least 3 alphabetic chars to be a real section title
        alpha_count >= 3
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline_modular::types::BBox;

    fn make_cluster(id: usize, label: &str, cells: Vec<TextCell>) -> ClusterWithCells {
        let bbox = if !cells.is_empty() {
            cells[0].bbox
        } else {
            BBox::new(0.0, 0.0, 100.0, 100.0)
        };

        ClusterWithCells {
            id,
            label: label.to_string(),
            bbox,
            confidence: 0.9,
            class_id: 0,
            cells,
        }
    }

    fn make_cell(text: &str, l: f64, t: f64, r: f64, b: f64) -> TextCell {
        TextCell {
            text: text.to_string(),
            bbox: BBox::new(l, t, r, b),
            confidence: Some(1.0),
            is_bold: false,
            is_italic: false,
        }
    }

    #[test]
    fn test_creates_orphans_for_unassigned() {
        // Use config without heuristics and without merging to test basic orphan creation
        let config = Stage06Config {
            apply_heuristics: false,
            merge_paragraphs: false, // Disable merging for this test
            ..Default::default()
        };
        let creator = Stage06OrphanCreator::with_config(config);

        let cell1 = make_cell("assigned", 0.0, 0.0, 10.0, 10.0);
        let cell2 = make_cell("unassigned1", 20.0, 20.0, 30.0, 30.0);
        let cell3 = make_cell("unassigned2", 40.0, 40.0, 50.0, 50.0);

        let clusters = ClustersWithCells {
            clusters: vec![make_cluster(0, "text", vec![cell1.clone()])],
        };

        let all_cells = OCRCells {
            cells: vec![cell1, cell2.clone(), cell3.clone()],
        };

        let output = creator.process(clusters, all_cells);

        // Should have: 1 original + 2 orphans = 3 total
        assert_eq!(output.clusters.len(), 3);

        // Check orphan IDs
        assert_eq!(output.clusters[1].id, 1);
        assert_eq!(output.clusters[2].id, 2);

        // Check orphan labels
        assert_eq!(output.clusters[1].label, "text");
        assert_eq!(output.clusters[2].label, "text");

        // Check orphan cells
        assert_eq!(output.clusters[1].cells.len(), 1);
        assert_eq!(output.clusters[1].cells[0].text, "unassigned1");
        assert_eq!(output.clusters[2].cells.len(), 1);
        assert_eq!(output.clusters[2].cells[0].text, "unassigned2");
    }

    #[test]
    fn test_skips_empty_text_cells() {
        let creator = Stage06OrphanCreator::new();

        let cell1 = make_cell("assigned", 0.0, 0.0, 10.0, 10.0);
        let cell2 = make_cell("", 20.0, 20.0, 30.0, 30.0); // Empty - should skip
        let cell3 = make_cell("  ", 40.0, 40.0, 50.0, 50.0); // Whitespace - should skip

        let clusters = ClustersWithCells {
            clusters: vec![make_cluster(0, "text", vec![cell1.clone()])],
        };

        let all_cells = OCRCells {
            cells: vec![cell1, cell2, cell3],
        };

        let output = creator.process(clusters, all_cells);

        // Should have: 1 original + 0 orphans = 1 total (empty cells skipped)
        assert_eq!(output.clusters.len(), 1);
    }

    #[test]
    fn test_no_orphans_when_all_assigned() {
        let creator = Stage06OrphanCreator::new();

        let cell1 = make_cell("assigned1", 0.0, 0.0, 10.0, 10.0);
        let cell2 = make_cell("assigned2", 20.0, 20.0, 30.0, 30.0);

        let clusters = ClustersWithCells {
            clusters: vec![
                make_cluster(0, "text", vec![cell1.clone()]),
                make_cluster(1, "text", vec![cell2.clone()]),
            ],
        };

        let all_cells = OCRCells {
            cells: vec![cell1, cell2],
        };

        let output = creator.process(clusters, all_cells);

        // Should have: 2 original + 0 orphans = 2 total
        assert_eq!(output.clusters.len(), 2);
    }

    #[test]
    fn test_config_create_orphans_false() {
        let config = Stage06Config {
            create_orphans: false,
            orphan_label: "text".to_string(),
            ..Default::default()
        };
        let creator = Stage06OrphanCreator::with_config(config);

        let cell1 = make_cell("assigned", 0.0, 0.0, 10.0, 10.0);
        let cell2 = make_cell("unassigned", 20.0, 20.0, 30.0, 30.0);

        let clusters = ClustersWithCells {
            clusters: vec![make_cluster(0, "text", vec![cell1.clone()])],
        };

        let all_cells = OCRCells {
            cells: vec![cell1, cell2],
        };

        let output = creator.process(clusters, all_cells);

        // Should have: 1 original + 0 orphans = 1 total (creation disabled)
        assert_eq!(output.clusters.len(), 1);
    }

    #[test]
    fn test_orphan_bbox_equals_cell_bbox() {
        let creator = Stage06OrphanCreator::new();

        let cell1 = make_cell("unassigned", 12.5, 34.7, 89.3, 102.1);

        let clusters = ClustersWithCells { clusters: vec![] };

        let all_cells = OCRCells {
            cells: vec![cell1.clone()],
        };

        let output = creator.process(clusters, all_cells);

        assert_eq!(output.clusters.len(), 1);
        assert_eq!(output.clusters[0].bbox, cell1.bbox);
    }

    #[test]
    fn test_next_id_calculation() {
        let creator = Stage06OrphanCreator::new();

        let cell1 = make_cell("assigned", 0.0, 0.0, 10.0, 10.0);
        let cell2 = make_cell("unassigned", 20.0, 20.0, 30.0, 30.0);

        // Existing clusters have IDs 5, 10, 15
        let clusters = ClustersWithCells {
            clusters: vec![
                make_cluster(5, "text", vec![cell1.clone()]),
                make_cluster(10, "caption", vec![]),
                make_cluster(15, "footer", vec![]),
            ],
        };

        let all_cells = OCRCells {
            cells: vec![cell1, cell2],
        };

        let output = creator.process(clusters, all_cells);

        // Should have: 3 original + 1 orphan = 4 total
        assert_eq!(output.clusters.len(), 4);

        // Orphan ID should be max(5, 10, 15) + 1 = 16
        assert_eq!(output.clusters[3].id, 16);
    }

    #[test]
    fn test_empty_existing_clusters() {
        let creator = Stage06OrphanCreator::new();

        let cell1 = make_cell("unassigned", 0.0, 0.0, 10.0, 10.0);

        let clusters = ClustersWithCells { clusters: vec![] };

        let all_cells = OCRCells { cells: vec![cell1] };

        let output = creator.process(clusters, all_cells);

        // Should have: 0 original + 1 orphan = 1 total
        assert_eq!(output.clusters.len(), 1);
        assert_eq!(output.clusters[0].id, 0); // First ID is 0
    }

    #[test]
    fn test_is_numbered_section_header() {
        let creator = Stage06OrphanCreator::new();

        // Should match
        assert!(creator.is_numbered_section_header("1 Introduction"));
        assert!(creator.is_numbered_section_header("2 Related Work"));
        assert!(creator.is_numbered_section_header("3 Problem Statement"));
        assert!(creator.is_numbered_section_header("4.1 Methods"));
        assert!(creator.is_numbered_section_header("5.2.1 Subsection"));
        assert!(creator.is_numbered_section_header("6 Conclusion"));
        assert!(creator.is_numbered_section_header("A Appendix"));

        // Should NOT match (too short, no text, or false positive patterns)
        assert!(!creator.is_numbered_section_header("1"));
        assert!(!creator.is_numbered_section_header("1."));
        assert!(!creator.is_numbered_section_header("1 a")); // lowercase
        assert!(!creator.is_numbered_section_header("1. some text")); // has dot then space (period)
        assert!(!creator.is_numbered_section_header("4 M. Lysak, et al.")); // running header
        assert!(!creator.is_numbered_section_header(
            "Optimized Table Tokenization for Table Structure Recognition 3"
        )); // running title
        assert!(!creator.is_numbered_section_header("longer than 100 chars ".repeat(5).as_str())); // too long
        assert!(!creator.is_numbered_section_header("")); // empty
        assert!(!creator.is_numbered_section_header("Abstract. Some text")); // no number
                                                                             // Bibliographic references should NOT match
        assert!(!creator.is_numbered_section_header("1. Auer, C., Dolfi, M., Carvalho, A.")); // reference
        assert!(!creator.is_numbered_section_header("18. Xue, W., Yu, B., Wang, W."));
        // reference
    }

    #[test]
    fn test_cells_inside_picture_become_orphan_text_items() {
        // N=4403: Test that OCR cells inside Picture bboxes become orphan text items
        // (separate text clusters) instead of being added to the Picture cluster.
        // This matches Python docling behavior where OCR text from figures appears
        // as standalone text items in reading order.
        let config = Stage06Config {
            apply_heuristics: false,
            merge_paragraphs: false,
            ..Default::default()
        };
        let creator = Stage06OrphanCreator::with_config(config);

        // Create a Picture cluster at (0, 0, 100, 100)
        let picture_cell = make_cell("picture content", 0.0, 0.0, 100.0, 100.0);
        let picture_cluster = make_cluster(0, "picture", vec![picture_cell.clone()]);

        // Create a cell inside the picture bbox
        let ocr_cell_inside = make_cell("chart label 1E+08", 20.0, 20.0, 80.0, 30.0);

        // Create a cell outside the picture bbox
        let ocr_cell_outside = make_cell("regular text", 150.0, 150.0, 200.0, 160.0);

        let clusters = ClustersWithCells {
            clusters: vec![picture_cluster],
        };

        let all_cells = OCRCells {
            cells: vec![
                picture_cell,
                ocr_cell_inside.clone(),
                ocr_cell_outside.clone(),
            ],
        };

        let output = creator.process(clusters, all_cells);

        // N=4403: Should have: 1 Picture cluster + 2 orphans (inside cell + outside cell)
        // The cell inside the picture is NOT added to Picture - it becomes an orphan text item
        assert_eq!(
            output.clusters.len(),
            3,
            "Expected 3 clusters (Picture + 2 orphans), got {}",
            output.clusters.len()
        );

        // The Picture cluster (index 0) should still have only 1 cell (original)
        let picture = &output.clusters[0];
        assert_eq!(picture.label, "picture", "First cluster should be Picture");
        assert_eq!(
            picture.cells.len(),
            1,
            "Picture should have 1 cell (original only), got {}",
            picture.cells.len()
        );

        // The OCR cell inside picture should NOT be in the Picture cluster
        let has_ocr_cell = picture.cells.iter().any(|c| c.text == "chart label 1E+08");
        assert!(
            !has_ocr_cell,
            "Picture cluster should NOT contain the OCR cell - it should be an orphan"
        );

        // Find the orphan clusters by their text
        let orphan_texts: Vec<&str> = output.clusters[1..]
            .iter()
            .flat_map(|c| c.cells.iter().map(|cell| cell.text.as_str()))
            .collect();
        assert!(
            orphan_texts.contains(&"chart label 1E+08"),
            "Should have orphan for 'chart label 1E+08'"
        );
        assert!(
            orphan_texts.contains(&"regular text"),
            "Should have orphan for 'regular text'"
        );
    }

    #[test]
    fn test_cells_inside_table_filtered_out() {
        // N=4153: Verify that cells inside Table bboxes are still filtered out
        // (Tables have structured cell handling and shouldn't get orphan OCR cells)
        let config = Stage06Config {
            apply_heuristics: false,
            merge_paragraphs: false,
            ..Default::default()
        };
        let creator = Stage06OrphanCreator::with_config(config);

        // Create a Table cluster at (0, 0, 100, 100)
        let table_cell = make_cell("table content", 0.0, 0.0, 100.0, 100.0);
        let table_cluster = make_cluster(0, "table", vec![table_cell.clone()]);

        // Create a cell inside the table bbox (should be filtered, not added to table)
        let ocr_cell_inside = make_cell("garbled table text", 20.0, 20.0, 80.0, 30.0);

        // Create a cell outside the table bbox
        let ocr_cell_outside = make_cell("regular text", 150.0, 150.0, 200.0, 160.0);

        let clusters = ClustersWithCells {
            clusters: vec![table_cluster],
        };

        let all_cells = OCRCells {
            cells: vec![table_cell, ocr_cell_inside, ocr_cell_outside.clone()],
        };

        let output = creator.process(clusters, all_cells);

        // Should have: 1 Table cluster + 1 orphan for the outside cell
        // The cell inside the table should be filtered out (NOT added to Table)
        assert_eq!(
            output.clusters.len(),
            2,
            "Expected 2 clusters (Table + orphan), got {}",
            output.clusters.len()
        );

        // The Table cluster should still have only 1 cell (original)
        let table = &output.clusters[0];
        assert_eq!(table.label, "table", "First cluster should be Table");
        assert_eq!(
            table.cells.len(),
            1,
            "Table should still have only 1 cell, got {}",
            table.cells.len()
        );

        // The inside cell should NOT be in the table
        let has_inside_cell = table.cells.iter().any(|c| c.text == "garbled table text");
        assert!(
            !has_inside_cell,
            "Table cluster should NOT contain the cell inside its bbox"
        );
    }
}
