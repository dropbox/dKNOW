//! # Layout Predictor - Document Layout Detection Model
//!
//! This module implements the layout detection model for identifying document elements
//! such as text blocks, tables, pictures, headers, formulas, and code blocks.
//!
//! ## Architecture
//!
//! The layout predictor uses **RT-DETR** (Real-Time Detection Transformer) architecture:
//! - **Backbone:** ResNet-50 (produces feature maps C2, C3, C4, C5)
//! - **Encoder:** Hybrid FPN+PAN encoder (multi-scale feature fusion)
//! - **Decoder:** 6-layer Transformer decoder with deformable attention
//! - **Heads:** Classification head (element types) + Bbox regression head (coordinates)
//!
//! ## Supported Backends
//!
//! The model supports two inference backends:
//!
//! ### 1. ONNX Backend (Default, Production)
//! - **Module:** Private `onnx` module (re-exported as [`LayoutPredictorModel`])
//! - **Runtime:** ONNX Runtime (ort crate)
//! - **Performance:** ⚡⚡⚡ Fast (2.88x faster than `PyTorch`)
//! - **Devices:** CPU, CUDA
//! - **Use case:** Production inference (opaque, optimized)
//!
//! ### 2. `PyTorch` Backend (Debug Only)
//! - **Module:** `pytorch_backend` (requires `pytorch` feature)
//! - **Runtime:** `PyTorch` (tch-rs crate)
//! - **Performance:** ⚡⚡ Medium
//! - **Devices:** CPU, CUDA, MPS (Apple Silicon)
//! - **Use case:** Debugging, intermediate output inspection
//! - **Note:** Not recommended for production (slower, more complex)
//!
//! ## Detected Element Types
//!
//! The model detects the following document element types:
//! - **Text:** Regular paragraph text
//! - **Title:** Document title
//! - **`SectionHeader`:** Section headings
//! - **Table:** Tables (structure analyzed by `TableFormer`)
//! - **Picture:** Images, diagrams, charts
//! - **Caption:** Figure/table captions
//! - **Formula:** Mathematical equations
//! - **Code:** Code blocks
//! - **Footnote:** Footnotes
//! - **PageHeader/PageFooter:** Headers/footers
//! - **`ListItem`:** Bulleted/numbered lists
//! - **Checkbox:** Form checkboxes
//!
//! ## Usage Example
//!
//! ```ignore
//! // NOTE: Requires model files from ds4sd/docling-models
//! use docling_pdf_ml::models::layout_predictor::LayoutPredictorModel;
//! use docling_pdf_ml::preprocessing::layout::layout_preprocess;
//! use std::path::Path;
//!
//! // Load model (ONNX backend, default)
//! let model = LayoutPredictorModel::load(
//!     Path::new("path/to/model.onnx"),
//!     docling_pdf_ml::Device::Cpu
//! )?;
//!
//! // Preprocess page image (requires ndarray input)
//! let page_image = ndarray::Array3::<u8>::zeros((792, 612, 3));
//! let preprocessed = layout_preprocess(&page_image);
//!
//! // Run inference using Pipeline::process_page (recommended API)
//! // See Pipeline documentation for complete usage
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Backend Selection
//!
//! By default, the ONNX backend is used. To use the `PyTorch` backend for debugging:
//!
//! ```ignore
//! // NOTE: Requires `pytorch` feature and model weights
//! #[cfg(feature = "pytorch")]
//! use docling_pdf_ml::models::layout_predictor::pytorch_backend::ResNetBackbone;
//!
//! // Load PyTorch backbone for debugging (requires pytorch feature)
//! let backbone = ResNetBackbone::new("path/to/weights.pt", tch::Device::Cpu)?;
//! let features = backbone.forward(&preprocessed)?;
//! // Inspect intermediate features C2, C3, C4, C5
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Performance
//!
//! Layout detection is the **primary bottleneck** in the pipeline:
//! - Accounts for **98.9%** of total processing time (non-OCR documents)
//! - **ONNX backend:** ~60 ms/page on MPS (Apple M1)
//! - **`PyTorch` backend:** ~170 ms/page on MPS (debugging only)
//! - **GPU (CUDA):** Expected 3-6x speedup vs CPU
//!
//! ## Model Source
//!
//! The model is downloaded from `HuggingFace`:
//! - **Repository:** `ds4sd/docling-models`
//! - **Model:** RT-DETR layout detection (ONNX and `PyTorch` versions)
//! - **Training:** Trained on diverse document corpus (papers, forms, books, etc.)

mod onnx;

// DocLayout-YOLO layout detection (GPU-only - 2.5x SLOWER on CPU, 20x faster with GPU)
pub mod doclayout_yolo;

// pytorch_backend requires PyTorch (tch-rs) for model inspection/debugging
#[cfg(feature = "pytorch")]
pub mod pytorch_backend;

// CoreML backend for Apple Neural Engine acceleration (macOS only)
// Achieves 71.5ms inference (7.2x faster than ONNX CPU)
#[cfg(feature = "coreml")]
pub mod coreml_backend;

// Re-export main types from ONNX module for backward compatibility
pub use onnx::{InferenceBackend, LayoutPredictorModel};

// Re-export DocLayout-YOLO for convenience
pub use doclayout_yolo::DocLayoutYolo;

// Re-export CoreML backend when available
#[cfg(feature = "coreml")]
pub use coreml_backend::DocLayoutYoloCoreML;
