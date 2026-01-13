//! Legacy document format parsing for docling
//!
//! This crate provides parsers for legacy document formats:
//! - RTF (Rich Text Format) - Implemented
//! - DOC (Microsoft Word 97-2003 binary format) - Implemented (via textutil on macOS)
//! - `WordPerfect` (.wpd) - Implemented (via libwpd/wpd2text)
//! - WPS (Kingsoft Writer) - Implemented (via `LibreOffice` conversion)
//!
//! ## Dependencies
//!
//! Some formats require external tools:
//! - **DOC**: macOS only (uses built-in `textutil`)
//! - **WPD**: Requires `libwpd` (`brew install libwpd` on macOS, `apt install libwpd-tools` on Linux)
//! - **WPS**: Requires `LibreOffice` (`soffice` command must be in PATH)
//!
//! ## Examples
//!
//! ### Parse RTF to Markdown
//!
//! ```rust
//! use docling_legacy::RtfParser;
//!
//! let rtf = r"{\rtf1\ansi Hello, World!}";
//! let doc = RtfParser::parse_str(rtf).unwrap();
//! let markdown = docling_legacy::rtf_to_markdown(&doc);
//! assert!(markdown.contains("Hello, World!"));
//! ```
//!
//! ### Extract Text from `WordPerfect`
//!
//! ```rust,no_run
//! use docling_legacy::WpdBackend;
//! use std::path::Path;
//!
//! let text = WpdBackend::extract_text(Path::new("document.wpd"))?;
//! println!("Extracted: {}", text);
//! # Ok::<(), anyhow::Error>(())
//! ```

/// DOC (Microsoft Word 97-2003) format backend
pub mod doc;
/// RTF (Rich Text Format) parsing module
pub mod rtf;
/// WPD (WordPerfect Document) format backend
pub mod wpd;
/// WPS (Kingsoft Writer) format backend
pub mod wps;

pub use doc::DocBackend;
pub use rtf::{to_markdown as rtf_to_markdown, to_markdown_raw as rtf_to_markdown_raw, RtfParser};
pub use wpd::WpdBackend;
pub use wps::WpsBackend;

// Re-export rtf-parser types for use in backends
pub use rtf_parser::StyleBlock;
