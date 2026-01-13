//! Kingsoft WPS Writer (.wps) document parsing
//!
//! This module provides parsing for Kingsoft WPS Writer documents (.wps files).
//!
//! ## Implementation Strategy
//!
//! Uses `LibreOffice` to convert WPS documents to DOCX format, then uses the
//! standard DOCX backend for parsing.
//!
//! ### Requirements
//!
//! - **`LibreOffice`** must be installed and accessible via `soffice` command
//!
//! ## Usage
//!
//! ```rust,no_run
//! use docling_legacy::wps::WpsBackend;
//! use std::path::Path;
//!
//! let wps_path = Path::new("document.wps");
//! let docx_bytes = WpsBackend::convert_to_docx(wps_path)?;
//! // Parse docx_bytes with DOCX backend
//! # Ok::<(), anyhow::Error>(())
//! ```

pub mod parser;

pub use parser::WpsBackend;
