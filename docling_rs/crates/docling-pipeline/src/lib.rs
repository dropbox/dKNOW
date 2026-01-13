//! # docling-pipeline - Document Processing Pipeline
//!
//! This crate is reserved for the high-level document processing pipeline
//! orchestration layer. Currently, pipeline functionality is implemented
//! in other crates:
//!
//! ## Architecture Overview
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                        Document Processing Pipeline                         │
//! └─────────────────────────────────────────────────────────────────────────────┘
//!
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                       Input Stage (Format Detection)                        │
//! │   Format detection → Backend selection → Content extraction                 │
//! └─────────────────────────────────────────────────────────────────────────────┘
//!                                      │
//!                                      ▼
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                       Analysis Stage (ML Processing)                        │
//! │   Layout detection → Table extraction → OCR → Reading order                 │
//! └─────────────────────────────────────────────────────────────────────────────┘
//!                                      │
//!                                      ▼
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                       Output Stage (Serialization)                          │
//! │   DocItems → Markdown/HTML/JSON → File output                               │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Current Implementation
//!
//! Pipeline functionality is currently implemented across:
//!
//! | Crate | Stage | Purpose |
//! |-------|-------|---------|
//! | [`docling-backend`](../docling_backend/index.html) | Input | Format detection, backend dispatch |
//! | [`docling-pdf-ml`](../docling_pdf_ml/index.html) | Analysis | ML-based layout detection for PDFs |
//! | [`docling-core`](../docling_core/index.html) | Output | Serialization to Markdown/HTML/JSON |
//!
//! ## Usage
//!
//! For document conversion, use the `DocumentConverter` in `docling-backend`:
//!
//! ```rust,ignore
//! use docling_backend::DocumentConverter;
//!
//! let converter = DocumentConverter::new()?;
//! let result = converter.convert("document.pdf")?;
//! println!("{}", result.document.to_markdown());
//! ```
//!
//! ## Future Plans
//!
//! This crate may eventually provide:
//! - Unified pipeline configuration
//! - Streaming batch processing
//! - Progress callbacks and monitoring
//! - Resource management and pooling
//! - Distributed processing support

// Pipeline functionality is currently in docling-backend
/// Placeholder function to prevent empty crate warnings.
///
/// This crate is reserved for future document processing pipeline utilities.
/// Actual pipeline functionality is currently implemented in `docling-backend`.
pub const fn placeholder() {}
