//! # Cascade Layout Predictor - Adaptive Layout Detection
//!
//! This module provides an adaptive cascade architecture that routes pages to
//! different layout detection methods based on document complexity:
//!
//! ```text
//! Page → Complexity Estimator (~1ms)
//!   ├─ Simple   → Heuristic layout (~1ms, 70-90% accuracy)
//!   ├─ Moderate → ML model (~60ms, 90% accuracy)
//!   └─ Complex  → ML model (~60ms, 90% accuracy)
//! ```
//!
//! ## Performance Benefits
//!
//! For documents with many simple pages (books, reports), cascade routing can
//! reduce average layout detection time from ~60ms to ~10ms per page (6x speedup).
//!
//! ## Usage
//!
//! ```ignore
//! use docling_pdf_ml::models::cascade_layout::{CascadeLayoutPredictor, CascadeMode};
//!
//! let predictor = CascadeLayoutPredictor::new(
//!     layout_predictor,    // Existing LayoutPredictorModel
//!     CascadeMode::Auto,   // Auto-detect complexity
//! );
//!
//! // Single page inference
//! let clusters = predictor.infer(&page_image, &text_blocks, page_width, page_height)?;
//!
//! // Batch inference
//! let results = predictor.infer_batch(&images, &text_blocks_per_page, widths, heights)?;
//! ```

mod predictor;

pub use predictor::{CascadeLayoutPredictor, CascadeMode, CascadeStats};
