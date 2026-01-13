//! Rule-based complexity classification.
//!
//! This module provides fast complexity classification using hand-crafted rules.
//! Can be replaced with an ML classifier in the future.

// Feature computation uses numeric conversions between index types (usize) and
// normalized values (f32/u16). Precision loss is acceptable for page statistics.
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]

use super::features::{PageFeatures, TextBlock};
use ndarray::Array3;

/// Document page complexity level.
///
/// Determines which layout detection method to use:
/// - `Simple`: Fast heuristic-based layout (~1ms)
/// - `Moderate`: Distilled lightweight model (~10ms) (future)
/// - `Complex`: Full RT-DETR model (~60ms)
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Complexity {
    /// Simple documents: single column, clear hierarchy, no tables/figures.
    ///
    /// Characteristics:
    /// - Single column layout
    /// - Clear heading hierarchy (varying font sizes)
    /// - Standard margins
    /// - No complex structures
    ///
    /// Best handled by heuristic layout detection (~1ms).
    /// Expected accuracy: 70-90% depending on document type.
    Simple,

    /// Moderate complexity: multi-column or has clear tables/figures.
    ///
    /// Characteristics:
    /// - Multi-column but regular spacing
    /// - Some figures/tables with clear boundaries
    /// - Standard academic/business layouts
    ///
    /// Best handled by distilled model (~10ms) [not yet implemented].
    /// Falls back to RT-DETR until distilled model is available.
    Moderate,

    /// Complex documents: irregular layout, overlapping elements.
    ///
    /// Characteristics:
    /// - Irregular layouts
    /// - Overlapping elements
    /// - Dense tables/figures
    /// - Magazine/newspaper style
    ///
    /// Requires full RT-DETR model for accurate layout detection (~60ms).
    /// Default value (for safety - won't miss elements).
    #[default]
    Complex,
}

impl std::fmt::Display for Complexity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Simple => write!(f, "simple"),
            Self::Moderate => write!(f, "moderate"),
            Self::Complex => write!(f, "complex"),
        }
    }
}

impl std::str::FromStr for Complexity {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "simple" => Ok(Self::Simple),
            "moderate" | "medium" => Ok(Self::Moderate),
            "complex" | "hard" => Ok(Self::Complex),
            _ => Err(format!(
                "Unknown complexity '{s}'. Expected: simple, moderate, complex"
            )),
        }
    }
}

/// Rule-based complexity estimator.
///
/// Uses hand-crafted rules based on page features to classify complexity.
/// This provides a fast, interpretable baseline that can be improved with ML.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ComplexityEstimator {
    /// Threshold for considering page as multi-column
    pub multi_column_threshold: f32,
    /// Threshold for considering table presence significant
    pub table_threshold: f32,
    /// Threshold for considering figure presence significant
    pub figure_threshold: f32,
    /// Minimum text blocks for meaningful analysis
    pub min_text_blocks: usize,
}

impl ComplexityEstimator {
    /// Create a new complexity estimator with default thresholds.
    #[inline]
    #[must_use = "returns a new ComplexityEstimator instance"]
    pub const fn new() -> Self {
        Self {
            multi_column_threshold: 1.5,
            table_threshold: 0.3,
            figure_threshold: 0.3,
            min_text_blocks: 3,
        }
    }

    /// Estimate page complexity from image and text blocks.
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
    /// Tuple of (complexity classification, extracted features).
    /// Features are returned for debugging and potential ML training.
    #[must_use = "returns the complexity classification and features"]
    pub fn estimate(
        &self,
        image: &Array3<u8>,
        text_blocks: &[TextBlock],
        page_width: f32,
        page_height: f32,
    ) -> (Complexity, PageFeatures) {
        // Fast path: check for obviously complex documents before full feature extraction
        if let Some(complexity) = self.is_obviously_complex(text_blocks, page_width) {
            // Return default features with quick early exit
            return (complexity, PageFeatures::default());
        }

        // Extract full features for borderline cases
        let features = PageFeatures::extract(image, text_blocks, page_width, page_height);

        // Classify based on features
        let complexity = self.classify(&features);

        (complexity, features)
    }

    /// Quick check for obviously complex documents.
    ///
    /// This avoids expensive feature extraction for documents that are
    /// clearly complex based on simple heuristics. Very conservative to
    /// avoid false positives.
    ///
    /// # Returns
    ///
    /// * `Some(Complexity::Complex)` if document is obviously complex
    /// * `None` if full feature extraction is needed
    #[inline]
    fn is_obviously_complex(
        &self,
        text_blocks: &[TextBlock],
        page_width: f32,
    ) -> Option<Complexity> {
        // Rule 1: Too few blocks to analyze - can't determine structure
        if text_blocks.len() < self.min_text_blocks {
            return Some(Complexity::Complex);
        }

        // Rule 2: Multi-column detection (quick histogram check)
        // Only for documents with many blocks (>50) to avoid false positives
        if text_blocks.len() > 50 {
            let mut left_bins = [0u16; 10];
            for block in text_blocks {
                let bin = ((block.bbox.0 / page_width) * 10.0).floor() as usize;
                if bin < 10 {
                    left_bins[bin] = left_bins[bin].saturating_add(1);
                }
            }

            // Count significant columns (bins with >15% of blocks)
            let threshold = (text_blocks.len() / 7).max(5) as u16;
            let column_count = left_bins.iter().filter(|&&c| c >= threshold).count();

            // More than 3 columns detected -> definitely Complex (newspapers, etc.)
            if column_count > 3 {
                return Some(Complexity::Complex);
            }
        }

        // Rule 3: Very large number of small blocks (likely dense table)
        // Only for documents with many uniform-width blocks
        if text_blocks.len() > 100 {
            let widths: Vec<f32> = text_blocks
                .iter()
                .map(super::features::TextBlock::width)
                .collect();
            let mean_width = widths.iter().sum::<f32>() / widths.len() as f32;

            // Count blocks with width close to mean (table cells are uniform)
            let uniform_count = widths
                .iter()
                .filter(|&&w| (w - mean_width).abs() < mean_width * 0.2)
                .count();

            // If >70% blocks have similar width and many small blocks -> dense table
            if uniform_count > text_blocks.len() * 70 / 100 && mean_width < page_width * 0.15 {
                return Some(Complexity::Complex);
            }
        }

        // Not obviously complex - need full feature extraction
        None
    }

    /// Classify complexity based on extracted features.
    ///
    /// # Decision Tree
    ///
    /// ```text
    /// 1. If few text blocks (<3) → Complex (can't determine structure)
    /// 2. If high table likelihood (>0.3) → Complex (tables need ML)
    /// 3. If high figure likelihood (>0.3) AND irregular layout → Complex
    /// 4. If multi-column (>1.5) with low regularity (<0.7) → Complex
    /// 5. If multi-column (>1.5) with good regularity (≥0.7) → Moderate
    /// 6. If figure likelihood moderate (>0.2) → Moderate
    /// 7. Otherwise → Simple
    /// ```
    #[must_use = "returns the complexity classification"]
    pub fn classify(&self, features: &PageFeatures) -> Complexity {
        // Rule 1: Too few text blocks to analyze reliably
        if features.text_block_count < self.min_text_blocks {
            return Complexity::Complex;
        }

        // Rule 2: High table likelihood requires ML for accurate detection
        if features.table_likelihood > self.table_threshold {
            return Complexity::Complex;
        }

        // Rule 3: High figure likelihood with irregular layout
        if features.figure_likelihood > self.figure_threshold
            && features.horizontal_alignment_score < 0.7
        {
            return Complexity::Complex;
        }

        // Rule 4: Multi-column with irregular spacing
        if features.estimated_columns > self.multi_column_threshold
            && features.column_regularity < 0.7
        {
            return Complexity::Complex;
        }

        // Rule 5: Multi-column but regular (e.g., academic papers)
        if features.estimated_columns > self.multi_column_threshold {
            return Complexity::Moderate;
        }

        // Rule 6: Moderate figure likelihood (single image in document)
        if features.figure_likelihood > 0.2 {
            return Complexity::Moderate;
        }

        // Rule 7: Single column, well-structured document
        // Check for good alignment and regular spacing
        if features.horizontal_alignment_score > 0.6 && features.vertical_spacing_regularity > 0.5 {
            return Complexity::Simple;
        }

        // Default to moderate for edge cases
        Complexity::Moderate
    }

    /// Quick check if page is definitely simple (conservative).
    ///
    /// Returns `true` only if page is very clearly simple.
    /// Use this for fast-path optimization without full feature extraction.
    #[must_use = "returns whether the page is definitely simple"]
    pub fn is_definitely_simple(&self, text_blocks: &[TextBlock], page_width: f32) -> bool {
        // Need enough blocks to be confident
        if text_blocks.len() < 5 {
            return false;
        }

        // Check for single column: all blocks should have similar left margins
        let left_margins: Vec<f32> = text_blocks.iter().map(|b| b.bbox.0).collect();
        let mean_left = left_margins.iter().sum::<f32>() / left_margins.len() as f32;
        let left_variance = left_margins
            .iter()
            .map(|&m| (m - mean_left).powi(2))
            .sum::<f32>()
            / left_margins.len() as f32;

        // Very low variance = single column
        let is_single_column = left_variance.sqrt() < page_width * 0.1;

        // Check for uniform-ish block widths (no small table cells)
        let widths: Vec<f32> = text_blocks
            .iter()
            .map(super::features::TextBlock::width)
            .collect();
        let mean_width = widths.iter().sum::<f32>() / widths.len() as f32;
        let width_variance = widths
            .iter()
            .map(|&w| (w - mean_width).powi(2))
            .sum::<f32>()
            / widths.len() as f32;

        // Moderate width variance = probably not a table
        let no_table_signals = width_variance.sqrt() < mean_width * 0.5;

        is_single_column && no_table_signals
    }
}

/// Statistics about complexity classification results.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct ComplexityStats {
    /// Number of pages classified as simple
    pub simple_count: usize,
    /// Number of pages classified as moderate
    pub moderate_count: usize,
    /// Number of pages classified as complex
    pub complex_count: usize,
}

impl ComplexityStats {
    /// Record a classification result.
    #[inline]
    pub fn record(&mut self, complexity: Complexity) {
        match complexity {
            Complexity::Simple => self.simple_count += 1,
            Complexity::Moderate => self.moderate_count += 1,
            Complexity::Complex => self.complex_count += 1,
        }
    }

    /// Total number of pages classified.
    #[inline]
    #[must_use = "returns the total count of classified pages"]
    pub const fn total(&self) -> usize {
        self.simple_count + self.moderate_count + self.complex_count
    }

    /// Percentage of pages that could use fast path (simple + moderate).
    #[inline]
    #[must_use = "returns the fast path percentage"]
    pub fn fast_path_percentage(&self) -> f32 {
        let total = self.total();
        if total == 0 {
            return 0.0;
        }
        (self.simple_count + self.moderate_count) as f32 / total as f32 * 100.0
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
    fn test_simple_document() {
        let estimator = ComplexityEstimator::new();
        let image = Array3::<u8>::zeros((100, 100, 3));

        // Simple single-column document with headers and paragraphs
        let blocks = vec![
            create_text_block(50.0, 50.0, 500.0, 30.0, 20.0, "Document Title"),
            create_text_block(
                50.0,
                100.0,
                500.0,
                100.0,
                12.0,
                "This is a paragraph of text that spans the full width.",
            ),
            create_text_block(50.0, 220.0, 500.0, 20.0, 14.0, "Section Header"),
            create_text_block(
                50.0,
                260.0,
                500.0,
                100.0,
                12.0,
                "Another paragraph of text.",
            ),
            create_text_block(50.0, 380.0, 500.0, 100.0, 12.0, "And one more paragraph."),
        ];

        let (complexity, features) = estimator.estimate(&image, &blocks, 612.0, 792.0);

        println!("Features: {features:?}");
        println!("Complexity: {complexity}");

        // Should be simple (single column, no tables/figures)
        assert!(matches!(
            complexity,
            Complexity::Simple | Complexity::Moderate
        ));
        assert!(features.estimated_columns < 1.5);
    }

    #[test]
    fn test_two_column_academic() {
        let estimator = ComplexityEstimator::new();
        let image = Array3::<u8>::zeros((100, 100, 3));

        // Two-column academic paper layout
        let blocks = vec![
            // Left column
            create_text_block(50.0, 100.0, 250.0, 100.0, 10.0, "Left column text 1"),
            create_text_block(50.0, 220.0, 250.0, 100.0, 10.0, "Left column text 2"),
            create_text_block(50.0, 340.0, 250.0, 100.0, 10.0, "Left column text 3"),
            // Right column
            create_text_block(320.0, 100.0, 250.0, 100.0, 10.0, "Right column text 1"),
            create_text_block(320.0, 220.0, 250.0, 100.0, 10.0, "Right column text 2"),
            create_text_block(320.0, 340.0, 250.0, 100.0, 10.0, "Right column text 3"),
        ];

        let (complexity, features) = estimator.estimate(&image, &blocks, 612.0, 792.0);

        println!("Features: {features:?}");
        println!("Complexity: {complexity}");

        // Should be moderate (multi-column but regular)
        assert!(features.estimated_columns >= 1.5);
        // Regular spacing should give moderate or complex, not simple
        assert!(!matches!(complexity, Complexity::Simple));
    }

    #[test]
    fn test_complex_with_table_signals() {
        let estimator = ComplexityEstimator::new();
        let image = Array3::<u8>::zeros((100, 100, 3));

        // Grid-like layout suggesting table
        let blocks = vec![
            // Row 1
            create_text_block(50.0, 100.0, 80.0, 20.0, 10.0, "Header 1"),
            create_text_block(150.0, 100.0, 80.0, 20.0, 10.0, "Header 2"),
            create_text_block(250.0, 100.0, 80.0, 20.0, 10.0, "Header 3"),
            // Row 2
            create_text_block(50.0, 130.0, 80.0, 20.0, 10.0, "Value 1"),
            create_text_block(150.0, 130.0, 80.0, 20.0, 10.0, "Value 2"),
            create_text_block(250.0, 130.0, 80.0, 20.0, 10.0, "Value 3"),
            // Row 3
            create_text_block(50.0, 160.0, 80.0, 20.0, 10.0, "Value 4"),
            create_text_block(150.0, 160.0, 80.0, 20.0, 10.0, "Value 5"),
            create_text_block(250.0, 160.0, 80.0, 20.0, 10.0, "Value 6"),
        ];

        let (complexity, features) = estimator.estimate(&image, &blocks, 612.0, 792.0);

        println!("Features: {features:?}");
        println!("Complexity: {complexity}");

        // Should detect table-like patterns
        assert!(features.table_likelihood > 0.1);
    }

    #[test]
    fn test_few_blocks_defaults_complex() {
        let estimator = ComplexityEstimator::new();
        let image = Array3::<u8>::zeros((100, 100, 3));

        // Very few blocks - can't determine structure
        let blocks = vec![
            create_text_block(50.0, 50.0, 500.0, 30.0, 20.0, "Title"),
            create_text_block(50.0, 100.0, 500.0, 100.0, 12.0, "Some text"),
        ];

        let (complexity, _features) = estimator.estimate(&image, &blocks, 612.0, 792.0);

        // Should default to complex when we can't determine structure
        assert!(matches!(complexity, Complexity::Complex));
    }

    #[test]
    fn test_is_definitely_simple() {
        let estimator = ComplexityEstimator::new();

        // Clear single-column document
        let blocks = vec![
            create_text_block(50.0, 50.0, 500.0, 30.0, 20.0, "Title"),
            create_text_block(50.0, 100.0, 500.0, 100.0, 12.0, "Para 1"),
            create_text_block(50.0, 220.0, 500.0, 100.0, 12.0, "Para 2"),
            create_text_block(50.0, 340.0, 500.0, 100.0, 12.0, "Para 3"),
            create_text_block(50.0, 460.0, 500.0, 100.0, 12.0, "Para 4"),
        ];

        assert!(estimator.is_definitely_simple(&blocks, 612.0));
    }

    #[test]
    fn test_complexity_stats() {
        let mut stats = ComplexityStats::default();

        stats.record(Complexity::Simple);
        stats.record(Complexity::Simple);
        stats.record(Complexity::Moderate);
        stats.record(Complexity::Complex);

        assert_eq!(stats.simple_count, 2);
        assert_eq!(stats.moderate_count, 1);
        assert_eq!(stats.complex_count, 1);
        assert_eq!(stats.total(), 4);
        assert!((stats.fast_path_percentage() - 75.0).abs() < 0.01);
    }

    #[test]
    fn test_complexity_from_str() {
        // Canonical forms
        assert_eq!("simple".parse::<Complexity>().unwrap(), Complexity::Simple);
        assert_eq!(
            "moderate".parse::<Complexity>().unwrap(),
            Complexity::Moderate
        );
        assert_eq!(
            "complex".parse::<Complexity>().unwrap(),
            Complexity::Complex
        );

        // Case insensitive
        assert_eq!("SIMPLE".parse::<Complexity>().unwrap(), Complexity::Simple);
        assert_eq!(
            "Complex".parse::<Complexity>().unwrap(),
            Complexity::Complex
        );

        // Aliases
        assert_eq!(
            "medium".parse::<Complexity>().unwrap(),
            Complexity::Moderate
        );
        assert_eq!("hard".parse::<Complexity>().unwrap(), Complexity::Complex);

        // Invalid
        assert!("invalid".parse::<Complexity>().is_err());
        assert!("".parse::<Complexity>().is_err());
    }

    #[test]
    fn test_complexity_roundtrip() {
        for complexity in [
            Complexity::Simple,
            Complexity::Moderate,
            Complexity::Complex,
        ] {
            let s = complexity.to_string();
            let parsed: Complexity = s.parse().unwrap();
            assert_eq!(parsed, complexity);
        }
    }
}
