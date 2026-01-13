//! Feature extraction for page complexity estimation.
//!
//! Extracts numerical features from page images and text blocks that can be used
//! to classify document complexity.

// Feature extraction uses numeric conversions between index types (usize) and
// normalized values (f32). Precision loss is acceptable for image statistics.
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]

use ndarray::Array3;

/// Text block with position and content information.
///
/// This is a lightweight representation of text extracted from PDF via pdfium.
#[derive(Debug, Clone, PartialEq)]
pub struct TextBlock {
    /// Bounding box: (left, top, right, bottom) in points
    pub bbox: (f32, f32, f32, f32),
    /// Estimated font size (height of text)
    pub font_size: f32,
    /// Text content (first line or preview)
    pub text: String,
    /// Character count in block
    pub char_count: usize,
}

impl TextBlock {
    /// Create a new text block.
    #[inline]
    #[must_use = "returns a new TextBlock instance"]
    pub fn new(bbox: (f32, f32, f32, f32), font_size: f32, text: String) -> Self {
        let char_count = text.chars().count();
        Self {
            bbox,
            font_size,
            text,
            char_count,
        }
    }

    /// Width of the bounding box.
    #[inline]
    #[must_use = "returns the width of the bounding box"]
    pub fn width(&self) -> f32 {
        self.bbox.2 - self.bbox.0
    }

    /// Height of the bounding box.
    #[inline]
    #[must_use = "returns the height of the bounding box"]
    pub fn height(&self) -> f32 {
        self.bbox.3 - self.bbox.1
    }

    /// Center X coordinate.
    #[inline]
    #[must_use = "returns the center X coordinate"]
    pub fn center_x(&self) -> f32 {
        (self.bbox.0 + self.bbox.2) / 2.0
    }

    /// Center Y coordinate.
    #[inline]
    #[must_use = "returns the center Y coordinate"]
    pub fn center_y(&self) -> f32 {
        (self.bbox.1 + self.bbox.3) / 2.0
    }
}

/// Features extracted from a document page for complexity estimation.
///
/// All features are normalized to [0, 1] range where applicable.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageFeatures {
    // Layout structure features
    /// Estimated number of columns (1.0 = single column, 2.0+ = multi-column)
    pub estimated_columns: f32,
    /// Column regularity score (1.0 = perfectly aligned, 0.0 = irregular)
    pub column_regularity: f32,

    // Text distribution features
    /// Number of text blocks on page
    pub text_block_count: usize,
    /// Variance in font sizes (0.0 = uniform, 1.0 = highly varied)
    pub font_size_variance: f32,
    /// Ratio of largest to smallest font size
    pub font_size_ratio: f32,

    // Content density features
    /// Text coverage ratio (text area / page area)
    pub text_coverage: f32,
    /// Whitespace distribution score (0.0 = packed, 1.0 = sparse)
    pub whitespace_score: f32,

    // Special element indicators
    /// Likelihood of tables (based on grid-like patterns)
    pub table_likelihood: f32,
    /// Likelihood of figures (based on large non-text regions)
    pub figure_likelihood: f32,
    /// Presence of bullet/number list patterns
    pub list_pattern_score: f32,

    // Structural features
    /// Count of potential headers (large font blocks)
    pub header_count: usize,
    /// Regularity of vertical spacing between blocks
    pub vertical_spacing_regularity: f32,
    /// Horizontal alignment consistency
    pub horizontal_alignment_score: f32,

    // Form element indicators (require RT-DETR 17-class model)
    /// Whether form elements were detected (checkboxes, key-value pairs, etc.)
    /// When true, RT-DETR should be used instead of YOLO for form class coverage.
    pub has_form_elements: bool,
}

impl Default for PageFeatures {
    #[inline]
    fn default() -> Self {
        Self {
            // Sensible defaults for empty/unknown pages
            estimated_columns: 1.0, // Assume single column
            column_regularity: 1.0, // Assume regular
            text_block_count: 0,
            font_size_variance: 0.0,
            font_size_ratio: 0.0,
            text_coverage: 0.0,
            whitespace_score: 1.0, // Empty = full whitespace
            table_likelihood: 0.0,
            figure_likelihood: 0.0,
            list_pattern_score: 0.0,
            header_count: 0,
            vertical_spacing_regularity: 1.0, // Assume regular
            horizontal_alignment_score: 1.0,  // Assume aligned
            has_form_elements: false,         // Assume no forms
        }
    }
}

impl PageFeatures {
    /// Extract features from page image and text blocks.
    ///
    /// # Arguments
    ///
    /// * `image` - Page image as RGB array (`HxWx3`)
    /// * `text_blocks` - Text blocks extracted from PDF
    /// * `page_width` - Page width in points
    /// * `page_height` - Page height in points
    ///
    /// # Returns
    ///
    /// Extracted features for complexity estimation.
    #[must_use = "returns the extracted page features"]
    pub fn extract(
        _image: &Array3<u8>,
        text_blocks: &[TextBlock],
        page_width: f32,
        page_height: f32,
    ) -> Self {
        if text_blocks.is_empty() {
            return Self::default();
        }

        let page_area = page_width * page_height;

        // Extract font sizes
        let font_sizes: Vec<f32> = text_blocks.iter().map(|b| b.font_size).collect();
        let min_font = font_sizes.iter().copied().fold(f32::MAX, f32::min);
        let max_font = font_sizes.iter().copied().fold(f32::MIN, f32::max);
        let mean_font = font_sizes.iter().sum::<f32>() / font_sizes.len() as f32;

        // Font size variance (normalized)
        let font_size_variance = if font_sizes.len() > 1 && mean_font > 0.0 {
            let variance = font_sizes
                .iter()
                .map(|&s| (s - mean_font).powi(2))
                .sum::<f32>()
                / font_sizes.len() as f32;
            (variance.sqrt() / mean_font).min(1.0)
        } else {
            0.0
        };

        let font_size_ratio = if min_font > 0.0 {
            (max_font / min_font).min(5.0) / 5.0
        } else {
            0.0
        };

        // Text coverage
        let total_text_area: f32 = text_blocks.iter().map(|b| b.width() * b.height()).sum();
        let text_coverage = (total_text_area / page_area).min(1.0);

        // Column estimation using X-coordinate clustering
        let (estimated_columns, column_regularity) =
            Self::estimate_columns(text_blocks, page_width);

        // Header count (blocks with font size significantly larger than average)
        let header_count = text_blocks
            .iter()
            .filter(|b| b.font_size > mean_font * 1.3)
            .count();

        // List pattern detection
        let list_pattern_score = Self::detect_list_patterns(text_blocks);

        // Table likelihood (grid-like arrangement)
        let table_likelihood = Self::estimate_table_likelihood(text_blocks, page_width);

        // Figure likelihood (large gaps in text coverage)
        let figure_likelihood =
            Self::estimate_figure_likelihood(text_blocks, page_width, page_height);

        // Vertical spacing regularity
        let vertical_spacing_regularity = Self::calculate_vertical_spacing_regularity(text_blocks);

        // Horizontal alignment score
        let horizontal_alignment_score =
            Self::calculate_horizontal_alignment(text_blocks, page_width);

        // Whitespace score (inverse of text coverage, weighted)
        let whitespace_score = 1.0 - text_coverage.sqrt();

        // Form element detection (checkboxes, key-value pairs, code blocks)
        // These elements require RT-DETR's 17-class model instead of YOLO's 11 classes
        let has_form_elements = Self::detect_form_elements(text_blocks, page_width);

        Self {
            estimated_columns,
            column_regularity,
            text_block_count: text_blocks.len(),
            font_size_variance,
            font_size_ratio,
            text_coverage,
            whitespace_score,
            table_likelihood,
            figure_likelihood,
            list_pattern_score,
            header_count,
            vertical_spacing_regularity,
            horizontal_alignment_score,
            has_form_elements,
        }
    }

    /// Detect potential form elements that require RT-DETR's extended class coverage.
    ///
    /// Heuristics:
    /// - Small square blocks near text (checkboxes)
    /// - Aligned key:value patterns
    /// - Monospace-like text patterns (code)
    fn detect_form_elements(text_blocks: &[TextBlock], page_width: f32) -> bool {
        if text_blocks.len() < 3 {
            return false;
        }

        // 1. Check for checkbox patterns: small square-ish blocks
        let mut potential_checkboxes = 0;
        for block in text_blocks {
            let w = block.width();
            let h = block.height();
            // Checkbox-like: small, roughly square
            if w < page_width * 0.05 && h > 0.0 && (w / h - 1.0).abs() < 0.5 && w < 30.0 {
                potential_checkboxes += 1;
            }
        }
        if potential_checkboxes >= 2 {
            return true;
        }

        // 2. Check for key:value patterns (colon followed by text on same line)
        let mut kv_patterns = 0;
        for block in text_blocks {
            // Simple heuristic: text contains colon not at end
            if block.text.contains(':') && !block.text.ends_with(':') {
                let colon_pos = block.text.find(':').unwrap();
                // Has content on both sides of colon
                if colon_pos > 0 && colon_pos < block.text.len() - 1 {
                    kv_patterns += 1;
                }
            }
        }
        // Forms typically have multiple key:value pairs
        if kv_patterns >= 3 {
            return true;
        }

        // 3. Check for code-like patterns (consistent indentation, special chars)
        let mut code_like = 0;
        for block in text_blocks {
            // Code indicators: braces, brackets, semicolons, =>, etc.
            let code_patterns = ["{", "}", "[", "]", ";", "=>", "->", "::"];
            let has_code_pattern = code_patterns.iter().any(|c| block.text.contains(c));
            if has_code_pattern {
                code_like += 1;
            }
        }
        if code_like >= 3 {
            return true;
        }

        false
    }

    /// Estimate number of columns and their regularity.
    fn estimate_columns(text_blocks: &[TextBlock], page_width: f32) -> (f32, f32) {
        if text_blocks.is_empty() {
            return (1.0, 1.0);
        }

        // Collect left edges of text blocks
        let left_edges: Vec<f32> = text_blocks.iter().map(|b| b.bbox.0).collect();

        // Simple histogram-based column detection
        let bin_width = page_width / 10.0;
        let mut bins = [0usize; 10];

        for &edge in &left_edges {
            let bin = ((edge / page_width) * 10.0).floor() as usize;
            if bin < 10 {
                bins[bin] += 1;
            }
        }

        // Count significant bins (peaks)
        let threshold = text_blocks.len() / 5; // At least 20% of blocks
        let significant_bins: Vec<usize> = bins
            .iter()
            .enumerate()
            .filter(|(_, &count)| count >= threshold.max(2))
            .map(|(i, _)| i)
            .collect();

        let num_columns = significant_bins.len().max(1) as f32;

        // Column regularity: how evenly distributed are the peaks
        let regularity = if significant_bins.len() > 1 {
            // Calculate expected spacing vs actual spacing
            let expected_spacing = 10.0 / num_columns;
            let actual_spacings: Vec<f32> = significant_bins
                .windows(2)
                .map(|w| (w[1] - w[0]) as f32)
                .collect();

            if actual_spacings.is_empty() {
                1.0
            } else {
                let mean_spacing =
                    actual_spacings.iter().sum::<f32>() / actual_spacings.len() as f32;
                let regularity_score = 1.0 - (mean_spacing - expected_spacing).abs() / 10.0;
                regularity_score.clamp(0.0, 1.0)
            }
        } else {
            1.0 // Single column is perfectly regular
        };

        (num_columns, regularity)
    }

    /// Detect list patterns (bullets, numbers).
    fn detect_list_patterns(text_blocks: &[TextBlock]) -> f32 {
        if text_blocks.is_empty() {
            return 0.0;
        }

        let list_starters = text_blocks
            .iter()
            .filter(|b| {
                let text = b.text.trim();
                // Check for common list patterns
                text.starts_with('-')
                    || text.starts_with('•')
                    || text.starts_with('*')
                    || text.starts_with("1.")
                    || text.starts_with("1)")
                    || text.starts_with('a')
                        && text.chars().nth(1).is_some_and(|c| c == '.' || c == ')')
            })
            .count();

        (list_starters as f32 / text_blocks.len() as f32).min(1.0)
    }

    /// Estimate likelihood of tables based on grid patterns.
    fn estimate_table_likelihood(text_blocks: &[TextBlock], page_width: f32) -> f32 {
        if text_blocks.len() < 4 {
            return 0.0;
        }

        // Look for grid-like alignment patterns
        // Key insight: tables have MULTIPLE distinct left edge positions
        // Single-column documents have ONE left edge position (not a table!)
        let x_tolerance = page_width * 0.02; // 2% tolerance
        let left_edges: Vec<f32> = text_blocks.iter().map(|b| b.bbox.0).collect();

        // Count distinct left edge positions
        let mut distinct_positions: Vec<f32> = Vec::new();
        for &edge in &left_edges {
            let is_new = !distinct_positions
                .iter()
                .any(|&pos| (pos - edge).abs() < x_tolerance);
            if is_new {
                distinct_positions.push(edge);
            }
        }

        // If only 1-2 distinct left positions, it's NOT a table (single/double column)
        if distinct_positions.len() < 3 {
            return 0.0;
        }

        // Count blocks at each distinct position
        let mut alignment_counts: Vec<usize> = distinct_positions
            .iter()
            .map(|&pos| {
                left_edges
                    .iter()
                    .filter(|&&e| (e - pos).abs() < x_tolerance)
                    .count()
            })
            .collect();
        alignment_counts.sort_unstable();
        alignment_counts.reverse();

        // For a table, multiple columns should have similar block counts
        // (e.g., 5 rows = each column has ~5 blocks)
        let top_two_sum = alignment_counts.iter().take(2).sum::<usize>();
        let alignment_score = (top_two_sum as f32 / text_blocks.len() as f32 / 2.0).min(1.0);

        // Check for small, uniform blocks (typical of table cells)
        let mean_width =
            text_blocks.iter().map(TextBlock::width).sum::<f32>() / text_blocks.len() as f32;
        let small_blocks = text_blocks
            .iter()
            .filter(|b| b.width() < mean_width * 0.5)
            .count();
        let small_block_ratio = small_blocks as f32 / text_blocks.len() as f32;

        // Combine signals (require both grid pattern AND small cells)
        (alignment_score * 0.5 + small_block_ratio * 0.5).min(1.0)
    }

    /// Estimate likelihood of figures based on layout gaps.
    fn estimate_figure_likelihood(
        text_blocks: &[TextBlock],
        page_width: f32,
        page_height: f32,
    ) -> f32 {
        if text_blocks.len() < 2 {
            return 0.0;
        }

        // Sort blocks by Y position
        let mut sorted_blocks = text_blocks.to_vec();
        sorted_blocks.sort_by(|a, b| a.bbox.1.partial_cmp(&b.bbox.1).unwrap());

        // Look for large vertical gaps between blocks
        let large_gap_threshold = page_height * 0.1; // 10% of page height
        let mut large_gaps = 0;

        for window in sorted_blocks.windows(2) {
            let gap = window[1].bbox.1 - window[0].bbox.3;
            if gap > large_gap_threshold {
                large_gaps += 1;
            }
        }

        // Also check for horizontal gaps (figure might be beside text)
        let mut horizontal_gaps = 0;
        for block in text_blocks {
            // If block doesn't span page width and there's space beside it
            let coverage = block.width() / page_width;
            if coverage < 0.5 {
                horizontal_gaps += 1;
            }
        }

        let vertical_score = (large_gaps as f32 / text_blocks.len() as f32 * 2.0).min(1.0);
        let horizontal_score = (horizontal_gaps as f32 / text_blocks.len() as f32 / 2.0).min(1.0);

        (vertical_score + horizontal_score) / 2.0
    }

    /// Calculate regularity of vertical spacing between blocks.
    fn calculate_vertical_spacing_regularity(text_blocks: &[TextBlock]) -> f32 {
        if text_blocks.len() < 3 {
            return 1.0; // Default to regular for few blocks
        }

        // Sort by Y position
        let mut sorted_blocks = text_blocks.to_vec();
        sorted_blocks.sort_by(|a, b| a.bbox.1.partial_cmp(&b.bbox.1).unwrap());

        // Calculate vertical gaps
        let gaps: Vec<f32> = sorted_blocks
            .windows(2)
            .map(|w| (w[1].bbox.1 - w[0].bbox.3).max(0.0))
            .filter(|&gap| gap > 0.0) // Only positive gaps
            .collect();

        if gaps.is_empty() {
            return 1.0;
        }

        let mean_gap = gaps.iter().sum::<f32>() / gaps.len() as f32;
        if mean_gap < 0.001 {
            return 1.0;
        }

        // Calculate coefficient of variation (std / mean)
        let variance =
            gaps.iter().map(|&g| (g - mean_gap).powi(2)).sum::<f32>() / gaps.len() as f32;
        let cv = variance.sqrt() / mean_gap;

        // Convert CV to regularity score (lower CV = more regular)
        (1.0 - cv.min(2.0) / 2.0).max(0.0)
    }

    /// Calculate horizontal alignment consistency.
    fn calculate_horizontal_alignment(text_blocks: &[TextBlock], page_width: f32) -> f32 {
        if text_blocks.len() < 2 {
            return 1.0;
        }

        // Calculate left margin consistency
        let left_margins: Vec<f32> = text_blocks.iter().map(|b| b.bbox.0).collect();
        let mean_left = left_margins.iter().sum::<f32>() / left_margins.len() as f32;
        let left_variance = left_margins
            .iter()
            .map(|&m| (m - mean_left).powi(2))
            .sum::<f32>()
            / left_margins.len() as f32;

        // Normalize by page width
        let normalized_variance = left_variance.sqrt() / page_width;

        // Lower variance = better alignment
        normalized_variance.min(0.5).mul_add(-2.0, 1.0).max(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::Array3;

    fn create_text_block(x: f32, y: f32, w: f32, h: f32, font_size: f32, text: &str) -> TextBlock {
        TextBlock::new((x, y, x + w, y + h), font_size, text.to_string())
    }

    #[test]
    fn test_empty_page_features() {
        let image = Array3::<u8>::zeros((100, 100, 3));
        let features = PageFeatures::extract(&image, &[], 612.0, 792.0);
        assert_eq!(features.text_block_count, 0);
        assert_eq!(features.estimated_columns, 1.0);
    }

    #[test]
    fn test_single_column_detection() {
        let image = Array3::<u8>::zeros((100, 100, 3));
        let blocks = vec![
            create_text_block(50.0, 50.0, 500.0, 20.0, 12.0, "Title"),
            create_text_block(50.0, 100.0, 500.0, 100.0, 10.0, "Paragraph 1"),
            create_text_block(50.0, 220.0, 500.0, 100.0, 10.0, "Paragraph 2"),
            create_text_block(50.0, 340.0, 500.0, 100.0, 10.0, "Paragraph 3"),
        ];
        let features = PageFeatures::extract(&image, &blocks, 612.0, 792.0);

        // Should detect single column
        assert!(features.estimated_columns < 1.5);
        assert!(features.column_regularity > 0.8);
    }

    #[test]
    fn test_two_column_detection() {
        let image = Array3::<u8>::zeros((100, 100, 3));
        // Create blocks in two columns
        let blocks = vec![
            // Left column
            create_text_block(50.0, 100.0, 200.0, 100.0, 10.0, "Left 1"),
            create_text_block(50.0, 220.0, 200.0, 100.0, 10.0, "Left 2"),
            create_text_block(50.0, 340.0, 200.0, 100.0, 10.0, "Left 3"),
            // Right column
            create_text_block(350.0, 100.0, 200.0, 100.0, 10.0, "Right 1"),
            create_text_block(350.0, 220.0, 200.0, 100.0, 10.0, "Right 2"),
            create_text_block(350.0, 340.0, 200.0, 100.0, 10.0, "Right 3"),
        ];
        let features = PageFeatures::extract(&image, &blocks, 612.0, 792.0);

        // Should detect two columns
        assert!(features.estimated_columns >= 1.5);
    }

    #[test]
    fn test_list_pattern_detection() {
        let image = Array3::<u8>::zeros((100, 100, 3));
        let blocks = vec![
            create_text_block(50.0, 100.0, 500.0, 20.0, 10.0, "- Item 1"),
            create_text_block(50.0, 130.0, 500.0, 20.0, 10.0, "- Item 2"),
            create_text_block(50.0, 160.0, 500.0, 20.0, 10.0, "- Item 3"),
            create_text_block(50.0, 190.0, 500.0, 20.0, 10.0, "Normal text"),
        ];
        let features = PageFeatures::extract(&image, &blocks, 612.0, 792.0);

        // Should detect list patterns
        assert!(features.list_pattern_score > 0.5);
    }

    #[test]
    fn test_header_detection() {
        let image = Array3::<u8>::zeros((100, 100, 3));
        let blocks = vec![
            create_text_block(50.0, 50.0, 500.0, 30.0, 24.0, "Big Title"),
            create_text_block(50.0, 100.0, 500.0, 20.0, 16.0, "Section Header"),
            create_text_block(50.0, 150.0, 500.0, 100.0, 10.0, "Normal paragraph text"),
            create_text_block(50.0, 280.0, 500.0, 100.0, 10.0, "Another paragraph"),
        ];
        let features = PageFeatures::extract(&image, &blocks, 612.0, 792.0);

        // Should detect headers (blocks with larger font)
        assert!(features.header_count >= 1);
        assert!(features.font_size_ratio > 0.3); // Significant font size variation
    }
}
