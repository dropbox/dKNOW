//! `WordPerfect` (.wpd) document parsing
//!
//! This module provides parsing for Corel `WordPerfect` documents (.wpd files).
//!
//! ## Implementation Strategy
//!
//! Uses `wpd2text` from libwpd to extract plain text from `WordPerfect` documents.
//! This provides fast text extraction without implementing the complex WPD binary format.
//!
//! ### Requirements
//!
//! - **libwpd** must be installed:
//!   - macOS: `brew install libwpd`
//!   - Linux: `apt install libwpd-tools` or equivalent
//!   - Windows: Not currently supported
//!
//! ### Supported Formats
//!
//! - `WordPerfect` 5.x (.wpd, .wp5)
//! - `WordPerfect` 6.x/7.x/8.x/9.x/10.x (.wpd)
//! - `WordPerfect` X3/X4/X5/X6/X7 (.wpd)
//!
//! ## Usage
//!
//! ```rust,no_run
//! use docling_legacy::wpd::WpdBackend;
//! use std::path::Path;
//!
//! let wpd_path = Path::new("document.wpd");
//! let text = WpdBackend::extract_text(wpd_path)?;
//! println!("Extracted text:\n{}", text);
//! # Ok::<(), anyhow::Error>(())
//! ```

pub mod parser;

pub use parser::WpdBackend;
