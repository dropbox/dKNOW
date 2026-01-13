//! # docling-py - Python Interoperability (Deprecated)
//!
//! **Status: DEPRECATED** - Python support has been removed from docling-rs.
//!
//! This crate previously provided Python interoperability via `PyO3` for accessing
//! Python docling's ML models. As of 2024, all ML functionality has been ported
//! to native Rust + C++ and Python is no longer required.
//!
//! ## Migration
//!
//! If you were using Python interop, migrate to the native Rust implementations:
//!
//! | Previous (Python) | Current (Rust) |
//! |-------------------|----------------|
//! | Python docling ML models | `docling-pdf-ml` (`PyTorch` C++ via tch-rs) |
//! | Python OCR | `docling-ocr` (ONNX Runtime) |
//! | Python `DocumentConverter` | `docling-backend::DocumentConverter` |
//! | Hybrid serializer | No longer needed (pure Rust) |
//!
//! ## Architecture
//!
//! The current architecture is 100% Rust + C++ (via FFI):
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                    docling-rs (100% Rust + C++ FFI)                         │
//! │  ❌ NO Python  ❌ NO PyO3  ❌ NO subprocess calls  ✅ Native performance    │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Why Python Was Removed
//!
//! - **Performance**: Native Rust/C++ is faster than Python subprocess calls
//! - **Deployment**: No Python runtime required in production
//! - **Reliability**: No Python version conflicts or dependency issues
//! - **Simplicity**: Single binary distribution
//!
//! ## See Also
//!
//! - [`docling-pdf-ml`](../docling_pdf_ml/index.html) - Native ML-based PDF processing
//! - [`docling-backend`](../docling_backend/index.html) - Document conversion (40+ formats)
//! - [`docling-ocr`](../docling_ocr/index.html) - Native OCR (`PaddleOCR` models)

// Python support has been removed. This crate is a placeholder for backwards compatibility.
/// Placeholder function to prevent empty crate warnings.
///
/// Python support has been removed from this project.
/// This crate exists for backwards compatibility only.
pub const fn placeholder() {}
