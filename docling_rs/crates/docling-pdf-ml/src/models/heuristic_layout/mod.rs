//! # Heuristic Layout Detection - Fast Rule-Based Layout Analysis
//!
//! This module provides fast layout detection using heuristics instead of ML models.
//! Designed for simple documents where ML is overkill (~1ms vs ~60ms).
//!
//! ## Detected Element Types
//!
//! - **`title`:** Large text at document start
//! - **`section_header`:** Larger-than-average text blocks
//! - **`text`:** Regular paragraph text
//! - **`list_item`:** Text starting with bullets/numbers
//! - **`page_header`:** Text in top margin area
//! - **`page_footer`:** Text in bottom margin area
//!
//! ## Usage
//!
//! ```ignore
//! use docling_pdf_ml::models::heuristic_layout::HeuristicLayoutDetector;
//! use docling_pdf_ml::baseline::LayoutCluster;
//!
//! let detector = HeuristicLayoutDetector::new();
//! let clusters = detector.detect(&text_blocks, page_width, page_height);
//! ```
//!
//! ## Limitations
//!
//! - Cannot detect tables (no grid analysis)
//! - Cannot detect figures (no image regions)
//! - Cannot detect formulas or code blocks
//! - Accuracy varies by document type (70-95%)

mod detector;

pub use detector::HeuristicLayoutDetector;
