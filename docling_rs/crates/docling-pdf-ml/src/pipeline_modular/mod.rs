//! # Modular Pipeline - Assembly Substages (Stages 3.0-3.5)
//!
//! This module provides fine-grained, independently testable stages for document assembly.
//! It is used internally by [`crate::pipeline::Pipeline`] to execute stages 3.0-3.5.
//!
//! ## Design Philosophy
//!
//! The modular pipeline breaks down the assembly process into discrete stages with:
//! - **Clear boundaries:** Each stage has well-defined input/output types
//! - **Independent testing:** Each stage can be validated in isolation
//! - **Type safety:** Different types for each processing stage (LabeledCluster, ClusterWithCells, etc.)
//! - **Baseline validation:** Each stage can be compared against Python docling baselines
//!
//! This mirrors the Python `docling_modular` architecture where each stage is a separate
//! transformation with explicit contracts.
//!
//! ## Pipeline Stages
//!
//! ### Stage 3.0: Cell Assignment ([`Stage04CellAssigner`])
//! - **Input:** Layout clusters (from Stage 1.8), OCR text cells (from Stage 0.3.1)
//! - **Process:** Assigns text cells to clusters using spatial containment
//! - **Output:** Clusters with assigned cells
//! - **Test:** `test_stage30_cell_assignment` (96/96 passing)
//!
//! ### Stage 3.1: Empty Removal ([`Stage05EmptyRemover`])
//! - **Input:** Clusters with cells
//! - **Process:** Filters out clusters without content (except tables/formulas)
//! - **Output:** Non-empty clusters only
//! - **Test:** `test_stage31_empty_removal` (96/96 passing)
//!
//! ### Stage 3.2: Orphan Creation ([`Stage06OrphanCreator`])
//! - **Input:** Clusters + unassigned text cells
//! - **Process:** Creates new TEXT clusters for unassigned cells (conf=1.0)
//! - **Output:** Original clusters + orphan clusters
//! - **Test:** `test_stage32_orphan_creation` (96/96 passing)
//!
//! ### Stage 3.3: BBox Adjustment ([`Stage07BboxAdjuster`])
//! - **Input:** Clusters with cells
//! - **Process:** Expands cluster bounding boxes to encompass all cell bounds
//! - **Output:** Clusters with adjusted bounding boxes
//! - **Test:** `test_stage33_bbox_adjust` (96/96 passing)
//! - **Note:** This stage iterates with Stage 3.4 until convergence
//!
//! ### Stage 3.4: Overlap Resolution ([`Stage08OverlapResolver`])
//! - **Input:** Clusters with adjusted bboxes
//! - **Process:** Merges overlapping clusters using Union-Find algorithm
//! - **Output:** Merged, non-overlapping clusters
//! - **Test:** `test_stage34_overlap_resolve` (96/96 passing)
//! - **Note:** Iterates with Stage 3.3 until no more overlaps (typically 1-2 iterations)
//!
//! ### Stage 3.5: Assembly ([`Stage09DocumentAssembler`])
//! - **Input:** Final merged clusters
//! - **Process:** Converts clusters to document elements (Text, Table, Picture, etc.)
//! - **Output:** Page elements ready for reading order
//! - **Test:** `test_stage35_assembly` (237/237 passing)
//!
//! ### Stage 4.1: Reading Order ([`Stage10ReadingOrder`])
//! - **Input:** Document elements
//! - **Process:** Determines spatial reading order
//! - **Output:** Ordered page elements
//! - **Note:** This implementation exists for modular testing but is NOT used by
//!   `Pipeline::process_page()`. The main pipeline uses `pipeline::reading_order::ReadingOrderPredictor`
//!   instead, which provides additional features (caption assignment, footnotes, merges).
//!   See N=155 report line 132-158 for rationale.
//!
//! ## Type System
//!
//! The modular pipeline uses a different type system than the main pipeline:
//! - **`BBox`**: f64 precision (vs `BoundingBox`: f32)
//! - **`TextCell`**: Simpler structure (vs `pipeline::TextCell`)
//! - **Stage-specific types**: `LabeledCluster`, `ClusterWithCells`, etc.
//!
//! This separation enables:
//! - Higher precision for intermediate computations (f64 vs f32)
//! - Optimized serialization for cross-language testing
//! - Clear boundaries between processing stages
//!
//! The main pipeline executor converts between type systems at stage boundaries
//! (see `pipeline/executor.rs:1044-1083`).
//!
//! ## Integration with Main Pipeline
//!
//! The main pipeline calls this module via [`ModularPipeline::process_stages_4_to_8()`]:
//! ```ignore
//! // pipeline/executor.rs:1035
//! let assembled = self.modular_pipeline.process_stages_4_to_8(
//!     clusters,
//!     textline_cells,
//!     page_width,
//!     page_height
//! )?;
//! ```
//!
//! The orchestrator chains stages 3.0-3.5 sequentially, with the 3.3-3.4 loop
//! iterating until convergence (no more overlaps).
//!
//! ## Testing
//!
//! Each stage is validated via:
//! - **Unit tests:** Rust integration tests (`tests/test_orchestrator_integration.rs`)
//! - **Pytest:** Python-Rust baseline comparison (`tests_pytest/test_stage03_assembly_pipeline.py`)
//! - **End-to-end:** Full orchestrator test (26/26 pages passing)
//!
//! ## Usage
//!
//! This module is **internal** and should not be used directly by library consumers.
//! Use [`crate::pipeline::Pipeline`] instead. This module is exposed for:
//! - Integration testing (direct stage-by-stage validation)
//! - Debugging (inspect intermediate stage outputs)
//! - Development (validate against Python baselines)

pub mod orchestrator;
pub mod stage04_cell_assigner;
pub mod stage05_empty_remover;
pub mod stage06_orphan_creator;
pub mod stage07_bbox_adjuster;
pub mod stage08_overlap_resolver;
pub mod stage09_document_assembler;
pub mod stage10_reading_order;
pub mod types;

pub use orchestrator::ModularPipeline;
pub use stage04_cell_assigner::{Stage04CellAssigner, Stage04Config};
pub use stage05_empty_remover::{Stage05Config, Stage05EmptyRemover};
pub use stage06_orphan_creator::{Stage06Config, Stage06OrphanCreator};
pub use stage07_bbox_adjuster::{Stage07BboxAdjuster, Stage07Config};
pub use stage08_overlap_resolver::{Stage08Config, Stage08OverlapResolver};
pub use stage09_document_assembler::{DocumentElement, Stage09Config, Stage09DocumentAssembler};
pub use stage10_reading_order::{Stage10Config, Stage10Output, Stage10ReadingOrder};
pub use types::*;
