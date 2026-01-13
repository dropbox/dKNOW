//! # ML Models - Document Understanding Models
//!
//! This module contains the machine learning models used in the PDF parsing pipeline.
//! All models support both CPU and GPU execution (PyTorch via tch-rs, ONNX via ort).
//!
//! ## Available Models
//!
//! ### Layout Detection Model
//! - **Module:** `layout_predictor`
//! - **Purpose:** Detects document layout elements (text, tables, pictures, headers, etc.)
//! - **Backend:** ONNX (production) or PyTorch (debugging)
//! - **Architecture:** RT-DETR (Real-Time Detection Transformer) with ResNet-50 backbone
//! - **Input:** 640×640 RGB image (preprocessed)
//! - **Output:** Bounding boxes with labels and confidence scores
//! - **Classes:** Text, Table, Picture, Caption, SectionHeader, Formula, Code, etc.
//!
//! ### Table Structure Model
//! - **Module:** `table_structure` (requires `pytorch` feature)
//! - **Purpose:** Analyzes table structure (rows, columns, cells, headers)
//! - **Backend:** PyTorch (tch-rs)
//! - **Architecture:** TableFormer (Transformer-based table understanding)
//! - **Input:** Table region image + text tokens from OCR
//! - **Output:** Cell coordinates, row/column assignments, header detection
//! - **Mode:** "accurate" (prioritizes quality over speed)
//!
//! ### Code/Formula Enrichment Model
//! - **Module:** `code_formula` (requires `pytorch` feature)
//! - **Purpose:** Enriches code blocks and mathematical formulas with descriptions
//! - **Backend:** PyTorch (tch-rs)
//! - **Architecture:** Idefics3 (Vision-Language Model from HuggingFace)
//! - **Input:** Code/formula region image + optional context
//! - **Output:** Natural language description or LaTeX representation
//! - **Optional:** Disabled by default (computationally expensive)
//!
//! ## Model Characteristics
//!
//! | Model | Backend | Input Size | Device Support | Relative Speed |
//! |-------|---------|------------|----------------|----------------|
//! | Layout (ONNX) | ONNX Runtime | 640×640 | CPU, CUDA | ⚡⚡⚡ Fast (default) |
//! | Layout (PyTorch) | tch-rs | 640×640 | CPU, CUDA, MPS | ⚡⚡ Medium (debug only) |
//! | TableFormer | tch-rs | Variable | CPU, CUDA, MPS | ⚡⚡ Medium |
//! | CodeFormula | tch-rs | Variable | CPU, CUDA, MPS | ⚡ Slow (optional) |
//!
//! ## Usage Example
//!
//! ```ignore
//! // NOTE: Requires model files and `pytorch` feature for table_structure
//! use docling_pdf_ml::models::layout_predictor::LayoutPredictorModel;
//! use std::path::Path;
//!
//! // Layout detection (ONNX backend, default)
//! let layout_model = LayoutPredictorModel::load(
//!     Path::new("path/to/model.onnx"),
//!     docling_pdf_ml::Device::Cpu
//! )?;
//!
//! // For complete document processing, use Pipeline (recommended API):
//! // let mut pipeline = Pipeline::with_defaults()?;
//! // let page = pipeline.process_page(0, &image, width, height, None)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Model Sources
//!
//! All models are downloaded from HuggingFace:
//! - **Layout:** `ds4sd/docling-models` (RT-DETR ONNX/PyTorch)
//! - **TableFormer:** `ds4sd/docling-models` (TableFormer PyTorch)
//! - **CodeFormula:** HuggingFace `Idefics3ForConditionalGeneration`
//!
//! ## Performance Considerations
//!
//! - **Layout detection** is the primary bottleneck (98.9% of pipeline time)
//! - **ONNX backend** is 2.88x faster than PyTorch for layout detection
//! - **GPU acceleration** (CUDA) provides 3-6x speedup over CPU
//! - **TableFormer** adds minimal overhead (~1-2% of total time)
//! - **CodeFormula** is optional and expensive (enable only when needed)

// code_formula requires PyTorch (tch-rs) for Idefics3 model
#[cfg(feature = "pytorch")]
pub mod code_formula;

pub mod layout_predictor;

// complexity_estimator provides fast page complexity classification
// Used to route pages through cascade architecture (heuristics → distilled → RT-DETR)
pub mod complexity_estimator;

// heuristic_layout provides fast rule-based layout detection for simple documents
// ~1ms vs ~60ms for ML-based detection (RT-DETR)
pub mod heuristic_layout;

// cascade_layout provides adaptive routing between heuristic and ML-based detection
// Routes simple pages to fast heuristics, complex pages to ML models
pub mod cascade_layout;

// table_structure requires PyTorch (tch-rs) for TableFormer model (IBM)
#[cfg(feature = "pytorch")]
pub mod table_structure;

// table_structure_onnx uses ONNX Runtime for Microsoft Table Transformer
// This is an alternative to PyTorch TableFormer when libtorch crashes
pub mod table_structure_onnx;
