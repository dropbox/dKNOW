//! # docling-parse - Document Parsing Infrastructure
//!
//! This crate is a placeholder in the docling-rs architecture, reserved for
//! future shared parsing utilities. Currently, parsing functionality is provided
//! by dedicated crates:
//!
//! ## Architecture Overview
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                           docling-rs Parsing Stack                          │
//! └─────────────────────────────────────────────────────────────────────────────┘
//!
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                           docling-backend                                   │
//! │   High-level document conversion (40+ format backends: PDF, DOCX, HTML...)  │
//! └─────────────────────────────────────────────────────────────────────────────┘
//!                                      │
//!              ┌───────────────────────┼───────────────────────┐
//!              ▼                       ▼                       ▼
//! ┌───────────────────────┐ ┌───────────────────────┐ ┌───────────────────────┐
//! │   docling-pdf-ml      │ │   docling-parse-rs    │ │  Format-specific      │
//! │   ML-based PDF        │ │   Safe Rust wrapper   │ │  crates (email,       │
//! │   layout detection    │ │   for C++ library     │ │  calendar, CAD, etc.) │
//! └───────────────────────┘ └───────────────────────┘ └───────────────────────┘
//!              │                       │
//!              │              ┌────────┘
//!              ▼              ▼
//! ┌───────────────────────────────────────────────────────────────────────────┐
//! │                          docling-parse-sys                                │
//! │                    Raw FFI bindings to C++ library                        │
//! └───────────────────────────────────────────────────────────────────────────┘
//!                                      │
//!                                      ▼
//! ┌───────────────────────────────────────────────────────────────────────────┐
//! │                       docling-parse C++ Library                           │
//! │                 (High-performance native PDF parsing)                     │
//! └───────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Related Crates
//!
//! | Crate | Purpose |
//! |-------|---------|
//! | [`docling-parse-rs`](../docling_parse_rs/index.html) | Safe Rust wrapper for C++ PDF parsing |
//! | [`docling-parse-sys`](../docling_parse_sys/index.html) | Raw FFI bindings to C++ library |
//! | [`docling-pdf-ml`](../docling_pdf_ml/index.html) | ML-based layout detection and table extraction |
//! | [`docling-backend`](../docling_backend/index.html) | High-level format backends (40+ formats) |
//!
//! ## Usage
//!
//! For most use cases, use one of the higher-level crates:
//!
//! ```rust,ignore
//! // For PDF with ML-based layout detection:
//! use docling_pdf_ml::Pipeline;
//!
//! // For multi-format document conversion:
//! use docling_backend::DocumentConverter;
//!
//! // For direct C++ PDF parsing:
//! use docling_parse_rs::DoclingParser;
//! ```
//!
//! ## Future Plans
//!
//! This crate may eventually contain:
//! - Shared text extraction utilities
//! - XML/HTML parsing helpers
//! - Character encoding detection
//! - Common parsing primitives used across multiple backends

// Placeholder - parsing is implemented in dedicated crates
/// Placeholder function to prevent empty crate warnings.
///
/// This crate is reserved for shared parsing utilities.
/// Actual parsing functionality is implemented in dedicated format crates.
pub const fn placeholder() {}
