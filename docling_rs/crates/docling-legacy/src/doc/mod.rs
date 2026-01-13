//! Microsoft Word 97-2003 (.doc) document parsing
//!
//! This module provides parsing for legacy Microsoft Word binary format (.doc files).
//!
//! ## Implementation Strategy
//!
//! The .doc format (MS-DOC) is a complex proprietary binary format based on OLE2/CFB
//! (Compound File Binary). Direct parsing would require 40-80+ hours of implementation
//! using the `cfb` crate as a foundation, followed by custom binary stream parsing.
//!
//! Instead, we use a **conversion-based approach** for faster implementation (4-6 hours):
//!
//! ### Conversion Approach
//!
//! 1. **macOS (Primary):** Use native `textutil` command to convert .doc → .docx
//!    - `/usr/bin/textutil -convert docx input.doc -output output.docx`
//!    - Preserves text, formatting, tables, structure
//!    - Fast (< 1 second per file)
//!    - Zero external dependencies (built into macOS)
//!
//! 2. **Linux/Windows (Future):** Use `LibreOffice` for cross-platform support
//!    - `soffice --headless --convert-to docx input.doc`
//!    - High quality conversion
//!    - Requires `LibreOffice` installation (500+ MB)
//!
//! ### Format Detection
//!
//! .doc files are detected by:
//! 1. File extension: `.doc`
//! 2. CFB signature: `D0 CF 11 E0 A1 B1 1A E1` (first 8 bytes)
//!
//! ### Error Handling
//!
//! - **macOS:** If textutil fails, return error with conversion details
//! - **Non-macOS:** Return error with instructions to:
//!   1. Use `LibreOffice` to convert .doc → .docx
//!   2. Install `LibreOffice` and enable feature flag (future)
//!
//! ## Usage
//!
//! ```rust,no_run
//! use docling_legacy::doc::DocBackend;
//! use std::path::Path;
//!
//! let doc_path = Path::new("document.doc");
//! let docx_path = DocBackend::convert_doc_to_docx(doc_path)?;
//! println!("Converted to: {}", docx_path.display());
//! # Ok::<(), anyhow::Error>(())
//! ```

pub mod parser;

pub use parser::DocBackend;
