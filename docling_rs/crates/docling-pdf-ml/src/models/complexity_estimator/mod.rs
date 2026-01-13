//! # Complexity Estimator - Document Page Complexity Classification
//!
//! This module provides fast page complexity estimation to route documents
//! through an efficient cascade architecture:
//!
//! ```text
//! Page → Complexity Estimator (~1ms)
//!   ├─ Simple   → Heuristic layout (1ms, 70-90% accuracy)
//!   ├─ Moderate → Distilled model (10ms, 85% accuracy)  (future)
//!   └─ Complex  → RT-DETR (60ms, 90% accuracy)
//! ```
//!
//! ## Complexity Classes
//!
//! - **Simple:** Single column, clear hierarchy, no tables/figures
//! - **Moderate:** Multi-column or has tables/figures with clear boundaries
//! - **Complex:** Irregular layout, overlapping elements, dense content
//!
//! ## Usage
//!
//! ```ignore
//! use docling_pdf_ml::models::complexity_estimator::{ComplexityEstimator, Complexity};
//!
//! let estimator = ComplexityEstimator::new();
//! let (complexity, features) = estimator.estimate(&page_image, &text_blocks);
//!
//! match complexity {
//!     Complexity::Simple => { /* use heuristics */ }
//!     Complexity::Moderate => { /* use distilled model */ }
//!     Complexity::Complex => { /* use full RT-DETR */ }
//! }
//! ```

mod classifier;
mod features;

pub use classifier::{Complexity, ComplexityEstimator, ComplexityStats};
pub use features::{PageFeatures, TextBlock};
