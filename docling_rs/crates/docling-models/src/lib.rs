//! # docling-models - Machine Learning Models
//!
//! This crate is reserved for ML model management and inference abstractions.
//! Currently, ML functionality is implemented in the `docling-pdf-ml` crate.
//!
//! ## Architecture Overview
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                          ML Model Architecture                              │
//! └─────────────────────────────────────────────────────────────────────────────┘
//!
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                           docling-pdf-ml                                    │
//! │              (ML pipeline for PDF processing)                               │
//! └─────────────────────────────────────────────────────────────────────────────┘
//!                                      │
//!              ┌───────────────────────┼───────────────────────┐
//!              ▼                       ▼                       ▼
//! ┌───────────────────────┐ ┌───────────────────────┐ ┌───────────────────────┐
//! │   Layout Detection    │ │   Table Structure     │ │   Reading Order       │
//! │   (RT-DETR model)     │ │   (TableFormer)       │ │   (Graph-based)       │
//! └───────────────────────┘ └───────────────────────┘ └───────────────────────┘
//!              │                       │                       │
//!              └───────────────────────┼───────────────────────┘
//!                                      ▼
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                        Inference Backend                                    │
//! │   PyTorch C++ (tch-rs) │ ONNX Runtime (ort) │ CoreML (Apple Silicon)       │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Available Models
//!
//! | Model | Purpose | Backend | Size |
//! |-------|---------|---------|------|
//! | RT-DETR Layout | Document structure detection | PyTorch/ONNX | ~50MB |
//! | `RapidOCR` | Text recognition | ONNX | ~10MB |
//! | `TableFormer` | Table structure | `PyTorch` | ~40MB |
//! | Reading Order | Reading sequence | `PyTorch` | ~30MB |
//!
//! ## Current Implementation
//!
//! ML inference is implemented in:
//!
//! | Crate | Purpose |
//! |-------|---------|
//! | [`docling-pdf-ml`](../docling_pdf_ml/index.html) | Complete ML pipeline for PDFs |
//! | [`docling-ocr`](../docling_ocr/index.html) | OCR using `PaddleOCR` models |
//!
//! ## Usage
//!
//! For ML-based PDF processing:
//!
//! ```rust,ignore
//! use docling_pdf_ml::{Pipeline, PipelineConfigBuilder, Device};
//!
//! // Create pipeline with ML models
//! let config = PipelineConfigBuilder::complete()
//!     .device(Device::Cpu)
//!     .build()?;
//! let mut pipeline = Pipeline::new(config)?;
//!
//! // Process a page
//! let result = pipeline.process_page(0, &image, width, height, None)?;
//! ```
//!
//! ## Future Plans
//!
//! This crate may eventually provide:
//! - Unified model registry and download
//! - Model version management
//! - Cross-backend inference abstraction
//! - Model quantization utilities
//! - Custom model integration

// ML functionality is currently in docling-pdf-ml and docling-ocr
/// Placeholder function to prevent empty crate warnings.
///
/// This crate is reserved for future ML model utilities.
/// Actual ML functionality is currently implemented in:
/// - `docling-pdf-ml` - PDF-specific ML models (layout, OCR, tables)
/// - `docling-ocr` - OCR functionality
pub const fn placeholder() {}
