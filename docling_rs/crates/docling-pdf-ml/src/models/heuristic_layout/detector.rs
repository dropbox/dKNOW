//! Heuristic-based layout detection for simple documents.

// Cluster IDs use usize internally but i32 for output format compatibility.
// Values are always within i32 range for practical document sizes.
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_possible_wrap)]

use crate::baseline::{BBox, LayoutCluster};
use crate::models::complexity_estimator::TextBlock;

/// Heuristic layout detector for simple documents.
///
/// Uses rule-based analysis of text blocks to identify document structure.
/// Much faster than ML-based detection (~1ms vs ~60ms).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeuristicLayoutDetector {
    /// Minimum font size ratio for headers (`header_size` / `mean_size`)
    pub header_ratio: f32,
    /// Minimum font size ratio for titles
    pub title_ratio: f32,
    /// Margin threshold as fraction of page dimension
    pub margin_threshold: f32,
    /// Minimum vertical gap between paragraphs (points)
    pub paragraph_gap: f32,
}

impl Default for HeuristicLayoutDetector {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl HeuristicLayoutDetector {
    /// Create a new heuristic layout detector with default settings.
    #[inline]
    #[must_use = "returns a new detector instance"]
    pub const fn new() -> Self {
        Self {
            header_ratio: 1.15,     // 15% larger than mean = header
            title_ratio: 1.3,       // 30% larger than mean = title
            margin_threshold: 0.05, // 5% of page = margin area
            paragraph_gap: 12.0,    // 12pt gap between paragraphs
        }
    }

    /// Detect layout elements from text blocks.
    ///
    /// # Arguments
    ///
    /// * `text_blocks` - Text blocks extracted from PDF
    /// * `page_width` - Page width in points
    /// * `page_height` - Page height in points
    ///
    /// # Returns
    ///
    /// Vector of layout clusters compatible with ML model output.
    #[must_use = "returns the detected layout clusters"]
    pub fn detect(
        &self,
        text_blocks: &[TextBlock],
        page_width: f32,
        page_height: f32,
    ) -> Vec<LayoutCluster> {
        if text_blocks.is_empty() {
            return Vec::new();
        }

        // Define margin boundaries first (needed for base font calculation)
        let top_margin = page_height * self.margin_threshold;
        let bottom_margin = page_height * (1.0 - self.margin_threshold);

        // Calculate font statistics for BODY text only (exclude headers/footers)
        // This prevents small margin text from skewing the baseline
        let body_fonts: Vec<f32> = text_blocks
            .iter()
            .filter(|b| b.bbox.3 >= top_margin && b.bbox.1 <= bottom_margin)
            .map(|b| b.font_size)
            .collect();

        // If no body text, use all blocks
        let font_sizes: Vec<f32> = if body_fonts.is_empty() {
            text_blocks.iter().map(|b| b.font_size).collect()
        } else {
            body_fonts
        };

        // Use min body font as baseline (body text is typically smallest in main content)
        let min_font = font_sizes.iter().copied().fold(f32::MAX, f32::min);
        let max_font = font_sizes.iter().copied().fold(f32::MIN, f32::max);
        let base_font = min_font;
        let left_margin = page_width * self.margin_threshold;
        let right_margin = page_width * (1.0 - self.margin_threshold);

        // Track if we've seen a title (first large heading)
        let mut title_found = false;

        let mut clusters = Vec::with_capacity(text_blocks.len());

        for (idx, block) in text_blocks.iter().enumerate() {
            let label = self.classify_block(
                block,
                base_font,
                max_font,
                top_margin,
                bottom_margin,
                left_margin,
                right_margin,
                &mut title_found,
            );

            clusters.push(LayoutCluster {
                id: idx as i32,
                label,
                confidence: self.estimate_confidence(block, base_font),
                bbox: BBox {
                    l: f64::from(block.bbox.0),
                    t: f64::from(block.bbox.1),
                    r: f64::from(block.bbox.2),
                    b: f64::from(block.bbox.3),
                },
            });
        }

        // Apply reading order (already sorted by position in most PDFs)
        self.apply_reading_order(&mut clusters);

        clusters
    }

    /// Classify a single text block.
    #[allow(
        clippy::too_many_arguments,
        reason = "layout classification requires multiple page-level metrics"
    )]
    fn classify_block(
        &self,
        block: &TextBlock,
        base_font: f32,
        max_font: f32,
        top_margin: f32,
        bottom_margin: f32,
        _left_margin: f32,
        _right_margin: f32,
        title_found: &mut bool,
    ) -> String {
        let text = block.text.trim();

        // 1. Check for page header (in top margin)
        if block.bbox.3 < top_margin {
            return "page_header".to_string();
        }

        // 2. Check for page footer (in bottom margin)
        if block.bbox.1 > bottom_margin {
            return "page_footer".to_string();
        }

        // 3. Check for list items (starts with bullet or number)
        if self.is_list_item(text) {
            return "list_item".to_string();
        }

        // 4. Check for title (largest font, first occurrence)
        // Title: largest font AND significantly larger than body text
        if !*title_found
            && block.font_size >= max_font * 0.95
            && block.font_size > base_font * self.title_ratio
        {
            *title_found = true;
            return "title".to_string();
        }

        // 5. Check for section header (larger than body text but not title)
        if block.font_size > base_font * self.header_ratio {
            return "section_header".to_string();
        }

        // 6. Default to text
        "text".to_string()
    }

    /// Check if text starts with a list pattern.
    // Method signature kept for API consistency with other HeuristicLayoutDetector methods
    #[allow(clippy::unused_self)]
    #[inline]
    fn is_list_item(&self, text: &str) -> bool {
        let first_char = text.chars().next();
        match first_char {
            Some('-' | '•' | '*' | '○' | '▪' | '►') => true,
            Some(c) if c.is_ascii_digit() => {
                // Check for "1." or "1)" pattern
                text.chars()
                    .nth(1)
                    .is_some_and(|second| second == '.' || second == ')')
            }
            Some(c) if c.is_ascii_lowercase() && text.len() >= 2 => {
                // Check for "a." or "a)" pattern
                text.chars()
                    .nth(1)
                    .is_some_and(|second| second == '.' || second == ')')
            }
            _ => false,
        }
    }

    /// Estimate confidence based on how clearly the classification matches.
    // Method signature kept for API consistency with other HeuristicLayoutDetector methods
    #[allow(clippy::unused_self)]
    fn estimate_confidence(&self, block: &TextBlock, base_font: f32) -> f64 {
        // Higher confidence for more distinct font sizes (compared to body text)
        let font_ratio = block.font_size / base_font;

        if font_ratio > 1.5 {
            // Clear header/title (50%+ larger than body)
            0.95
        } else if font_ratio > 1.2 {
            // Likely a header (20-50% larger)
            0.85
        } else {
            // Similar to body text - default confidence
            0.75
        }
    }

    /// Apply reading order to clusters (top-to-bottom, left-to-right).
    // Method signature kept for API consistency with other HeuristicLayoutDetector methods
    #[allow(clippy::unused_self)]
    fn apply_reading_order(&self, clusters: &mut [LayoutCluster]) {
        // Sort by Y position first, then X position for same-line elements
        clusters.sort_by(|a, b| {
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

        // Update IDs to reflect reading order
        for (idx, cluster) in clusters.iter_mut().enumerate() {
            cluster.id = idx as i32;
        }
    }

    /// Group text blocks into paragraphs based on vertical gaps.
    ///
    /// This can be used to merge multiple text blocks into single paragraph clusters.
    #[must_use = "returns the grouped paragraph indices"]
    pub fn group_paragraphs(&self, text_blocks: &[TextBlock]) -> Vec<Vec<usize>> {
        if text_blocks.is_empty() {
            return Vec::new();
        }

        // Sort blocks by Y position
        let mut indices: Vec<usize> = (0..text_blocks.len()).collect();
        indices.sort_by(|&a, &b| {
            text_blocks[a]
                .bbox
                .1
                .partial_cmp(&text_blocks[b].bbox.1)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut paragraphs: Vec<Vec<usize>> = Vec::new();
        let mut current_para: Vec<usize> = vec![indices[0]];

        for &idx in indices.iter().skip(1) {
            let prev_idx = *current_para.last().unwrap();
            let prev_block = &text_blocks[prev_idx];
            let curr_block = &text_blocks[idx];

            // Calculate gap between blocks
            let gap = curr_block.bbox.1 - prev_block.bbox.3;

            // Check if blocks have similar left alignment (same paragraph)
            let alignment_diff = (curr_block.bbox.0 - prev_block.bbox.0).abs();
            let is_aligned = alignment_diff < 20.0; // 20pt tolerance

            if gap < self.paragraph_gap && is_aligned {
                // Same paragraph
                current_para.push(idx);
            } else {
                // New paragraph
                paragraphs.push(current_para);
                current_para = vec![idx];
            }
        }

        paragraphs.push(current_para);
        paragraphs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_text_block(x: f32, y: f32, w: f32, h: f32, font_size: f32, text: &str) -> TextBlock {
        TextBlock::new((x, y, x + w, y + h), font_size, text.to_string())
    }

    #[test]
    fn test_empty_detection() {
        let detector = HeuristicLayoutDetector::new();
        let clusters = detector.detect(&[], 612.0, 792.0);
        assert!(clusters.is_empty());
    }

    #[test]
    fn test_title_detection() {
        let detector = HeuristicLayoutDetector::new();
        let blocks = vec![
            create_text_block(50.0, 50.0, 500.0, 40.0, 24.0, "Document Title"),
            create_text_block(
                50.0,
                120.0,
                500.0,
                100.0,
                12.0,
                "Normal paragraph text here.",
            ),
        ];

        let clusters = detector.detect(&blocks, 612.0, 792.0);

        assert_eq!(clusters.len(), 2);
        assert_eq!(clusters[0].label, "title");
        assert_eq!(clusters[1].label, "text");
    }

    #[test]
    fn test_section_header_detection() {
        let detector = HeuristicLayoutDetector::new();
        let blocks = vec![
            create_text_block(50.0, 50.0, 500.0, 40.0, 24.0, "Title"),
            create_text_block(50.0, 120.0, 500.0, 100.0, 12.0, "Paragraph 1"),
            create_text_block(50.0, 250.0, 500.0, 30.0, 16.0, "Section Header"),
            create_text_block(50.0, 300.0, 500.0, 100.0, 12.0, "Paragraph 2"),
        ];

        let clusters = detector.detect(&blocks, 612.0, 792.0);

        assert_eq!(clusters.len(), 4);
        assert_eq!(clusters[0].label, "title");
        assert_eq!(clusters[1].label, "text");
        assert_eq!(clusters[2].label, "section_header");
        assert_eq!(clusters[3].label, "text");
    }

    #[test]
    fn test_list_detection() {
        let detector = HeuristicLayoutDetector::new();
        let blocks = vec![
            create_text_block(50.0, 100.0, 500.0, 20.0, 12.0, "- First item"),
            create_text_block(50.0, 130.0, 500.0, 20.0, 12.0, "• Second item"),
            create_text_block(50.0, 160.0, 500.0, 20.0, 12.0, "1. Third item"),
            create_text_block(50.0, 190.0, 500.0, 20.0, 12.0, "a) Fourth item"),
            create_text_block(50.0, 220.0, 500.0, 20.0, 12.0, "Normal text"),
        ];

        let clusters = detector.detect(&blocks, 612.0, 792.0);

        assert_eq!(clusters.len(), 5);
        assert_eq!(clusters[0].label, "list_item");
        assert_eq!(clusters[1].label, "list_item");
        assert_eq!(clusters[2].label, "list_item");
        assert_eq!(clusters[3].label, "list_item");
        assert_eq!(clusters[4].label, "text");
    }

    #[test]
    fn test_page_header_footer() {
        let detector = HeuristicLayoutDetector::new();
        let page_height = 792.0;
        let blocks = vec![
            // Page header (top 5% = 39.6 points)
            create_text_block(50.0, 10.0, 200.0, 15.0, 10.0, "Page 1"),
            // Normal content
            create_text_block(50.0, 100.0, 500.0, 100.0, 12.0, "Content"),
            // Page footer (bottom 5% = starts at 752.4)
            create_text_block(50.0, 770.0, 200.0, 15.0, 10.0, "Footer"),
        ];

        let clusters = detector.detect(&blocks, 612.0, page_height);

        assert_eq!(clusters.len(), 3);
        assert_eq!(clusters[0].label, "page_header");
        assert_eq!(clusters[1].label, "text");
        assert_eq!(clusters[2].label, "page_footer");
    }

    #[test]
    fn test_reading_order() {
        let detector = HeuristicLayoutDetector::new();
        let blocks = vec![
            // Second in visual order (lower on page)
            create_text_block(50.0, 200.0, 500.0, 50.0, 12.0, "Second paragraph"),
            // First in visual order (higher on page)
            create_text_block(50.0, 100.0, 500.0, 50.0, 12.0, "First paragraph"),
        ];

        let clusters = detector.detect(&blocks, 612.0, 792.0);

        // Should be reordered by Y position
        assert_eq!(clusters[0].id, 0);
        assert_eq!(clusters[1].id, 1);
        assert!(clusters[0].bbox.t < clusters[1].bbox.t);
    }

    #[test]
    fn test_paragraph_grouping() {
        let detector = HeuristicLayoutDetector::new();
        let blocks = vec![
            // Paragraph 1 (two lines close together)
            create_text_block(50.0, 100.0, 500.0, 12.0, 12.0, "Line 1 of para 1"),
            create_text_block(50.0, 114.0, 500.0, 12.0, 12.0, "Line 2 of para 1"),
            // Gap
            // Paragraph 2
            create_text_block(50.0, 150.0, 500.0, 12.0, 12.0, "Line 1 of para 2"),
            create_text_block(50.0, 164.0, 500.0, 12.0, 12.0, "Line 2 of para 2"),
        ];

        let paragraphs = detector.group_paragraphs(&blocks);

        // Should have 2 paragraphs
        assert_eq!(paragraphs.len(), 2);
        assert_eq!(paragraphs[0].len(), 2);
        assert_eq!(paragraphs[1].len(), 2);
    }

    #[test]
    fn test_confidence_levels() {
        let detector = HeuristicLayoutDetector::new();
        let blocks = vec![
            create_text_block(50.0, 50.0, 500.0, 40.0, 24.0, "Large Title"),
            create_text_block(50.0, 120.0, 500.0, 100.0, 12.0, "Normal text"),
        ];

        let clusters = detector.detect(&blocks, 612.0, 792.0);

        // Title should have higher confidence (distinct font)
        assert!(clusters[0].confidence > clusters[1].confidence);
    }
}
