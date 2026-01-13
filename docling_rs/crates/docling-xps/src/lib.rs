//! # docling-xps
//!
//! XPS (XML Paper Specification) document parser for docling-rs.
//!
//! This crate provides parsing support for XPS and Open XPS (OXPS) documents,
//! Microsoft's XML-based fixed document format similar to PDF.
//!
//! ## Supported Formats
//!
//! | Format | Extension | Description |
//! |--------|-----------|-------------|
//! | XPS | `.xps` | XML Paper Specification (original) |
//! | OXPS | `.oxps` | Open XPS (ISO 29500-compliant variant) |
//!
//! ## What is XPS?
//!
//! XPS (XML Paper Specification) is a fixed-layout document format developed by
//! Microsoft, similar in purpose to PDF. Key characteristics:
//!
//! - **Fixed layout**: Pages render identically on any device
//! - **XML-based**: Document structure stored as XML files
//! - **ZIP container**: XPS files are ZIP archives with specific structure
//! - **Print fidelity**: Designed for high-quality printing
//! - **Native Windows**: Built-in support in Windows Vista and later
//!
//! XPS is commonly used for:
//! - Printing and print preview on Windows
//! - Document archival and exchange
//! - Publishing workflows with Windows applications
//!
//! ## Quick Start
//!
//! ### Parse an XPS Document
//!
//! ```rust,no_run
//! use docling_xps::parse_xps;
//! use std::path::Path;
//!
//! let doc = parse_xps(Path::new("report.xps"))?;
//!
//! // Access metadata
//! if let Some(title) = &doc.metadata.title {
//!     println!("Title: {}", title);
//! }
//! if let Some(author) = &doc.metadata.author {
//!     println!("Author: {}", author);
//! }
//!
//! // Iterate through pages
//! for page in &doc.pages {
//!     println!("Page {}: {}x{} units",
//!         page.number, page.width, page.height);
//!
//!     // Extract text elements
//!     for text in &page.text {
//!         println!("  [{:.0},{:.0}] {}",
//!             text.x, text.y, text.content);
//!     }
//! }
//! # Ok::<(), docling_xps::XpsError>(())
//! ```
//!
//! ### Extract All Text Content
//!
//! ```rust,no_run
//! use docling_xps::parse_xps;
//! use std::path::Path;
//!
//! let doc = parse_xps(Path::new("document.xps"))?;
//!
//! // Collect all text in reading order
//! let mut full_text = String::new();
//! for page in &doc.pages {
//!     for text in &page.text {
//!         full_text.push_str(&text.content);
//!         full_text.push(' ');
//!     }
//!     full_text.push('\n');
//! }
//!
//! println!("{}", full_text);
//! # Ok::<(), docling_xps::XpsError>(())
//! ```
//!
//! ## Document Structure
//!
//! ### `XpsDocument`
//!
//! The top-level container for an XPS document:
//!
//! | Field | Type | Description |
//! |-------|------|-------------|
//! | `metadata` | `XpsMetadata` | Document properties |
//! | `pages` | `Vec<XpsPage>` | Document pages |
//!
//! ### `XpsMetadata`
//!
//! Document metadata extracted from `docProps/core.xml`:
//!
//! | Field | Type | Description |
//! |-------|------|-------------|
//! | `title` | `Option<String>` | Document title |
//! | `author` | `Option<String>` | Document author(s) |
//! | `subject` | `Option<String>` | Document subject |
//! | `creator` | `Option<String>` | Creating application |
//! | `keywords` | `Option<String>` | Document keywords |
//! | `description` | `Option<String>` | Document description |
//! | `created` | `Option<String>` | Creation date (ISO 8601) |
//! | `modified` | `Option<String>` | Last modified date |
//!
//! ### `XpsPage`
//!
//! A single page in the document:
//!
//! | Field | Type | Description |
//! |-------|------|-------------|
//! | `number` | `usize` | Page number (1-indexed) |
//! | `width` | `f64` | Width in XPS units (1/96 inch) |
//! | `height` | `f64` | Height in XPS units (1/96 inch) |
//! | `text` | `Vec<XpsTextElement>` | Text elements on page |
//!
//! ### `XpsTextElement`
//!
//! A text run on a page with position:
//!
//! | Field | Type | Description |
//! |-------|------|-------------|
//! | `content` | `String` | Text content |
//! | `x` | `f64` | X position (left edge) |
//! | `y` | `f64` | Y position (top edge) |
//! | `font_size` | `Option<f64>` | Font size if available |
//!
//! ## XPS File Structure
//!
//! An XPS file is a ZIP archive with this structure:
//!
//! ```text
//! document.xps/
//! ├── [Content_Types].xml        # MIME type mappings
//! ├── _rels/
//! │   └── .rels                  # Root relationships
//! ├── FixedDocSeq.fdseq          # Document sequence
//! ├── Documents/
//! │   └── 1/
//! │       ├── FixedDoc.fdoc      # Document structure
//! │       └── Pages/
//! │           ├── 1.fpage        # Page content (XML)
//! │           ├── 2.fpage
//! │           └── ...
//! ├── Resources/
//! │   ├── Fonts/                 # Embedded fonts (.odttf)
//! │   └── Images/                # Embedded images
//! └── docProps/
//!     └── core.xml               # Document metadata
//! ```
//!
//! ## XPS Units
//!
//! XPS uses a unit of 1/96 inch (same as Windows DIPs):
//!
//! - Letter page: 816 x 1056 units (8.5" x 11")
//! - A4 page: 793.7 x 1122.5 units (210mm x 297mm)
//!
//! Convert to inches: `value / 96.0`
//! Convert to points: `value / 96.0 * 72.0`
//!
//! ## Use Cases
//!
//! - **Text extraction**: Extract text from Windows print outputs
//! - **Document conversion**: Convert XPS to other formats
//! - **Print preview**: Access print-ready content
//! - **Windows integration**: Process XPS files from Windows apps
//!
//! ## Limitations
//!
//! - **Text only**: Images and vector graphics are not extracted
//! - **Reading order**: Text elements may not be in reading order
//! - **Formatting**: Font styles and colors not preserved
//! - **Complex layouts**: Multi-column layouts may need reordering
//!
//! ## Error Handling
//!
//! ```rust,no_run
//! use docling_xps::{parse_xps, XpsError};
//! use std::path::Path;
//!
//! match parse_xps(Path::new("document.xps")) {
//!     Ok(doc) => println!("Parsed {} pages", doc.pages.len()),
//!     Err(XpsError::Io(e)) => println!("File error: {}", e),
//!     Err(XpsError::Zip(e)) => println!("Archive error: {}", e),
//!     Err(XpsError::Xml(e)) => println!("XML parse error: {}", e),
//!     Err(e) => println!("Error: {}", e),
//! }
//! ```

pub mod error;
pub mod metadata;
pub mod page;
pub mod parser;

pub use error::{Result, XpsError};
pub use metadata::XpsMetadata;
pub use page::{XpsPage, XpsTextElement};
pub use parser::{parse_xps, XpsDocument};

// ============================================================================
// XPS Standard Constants
// ============================================================================

/// XPS units per inch (Windows DIPs - Device Independent Pixels).
///
/// XPS documents use 1/96 inch units (the same as Windows DIPs), meaning
/// 96 units equals 1 inch. This is the standard screen DPI in Windows.
pub const XPS_UNITS_PER_INCH: f64 = 96.0;

/// Default XPS page width in XPS units (US Letter: 8.5 inches × 96 DPI = 816).
///
/// This is the standard US Letter paper width when no explicit page dimensions
/// are specified in the XPS document.
pub const XPS_DEFAULT_PAGE_WIDTH: f64 = 816.0;

/// Default XPS page height in XPS units (US Letter: 11 inches × 96 DPI = 1056).
///
/// This is the standard US Letter paper height when no explicit page dimensions
/// are specified in the XPS document.
pub const XPS_DEFAULT_PAGE_HEIGHT: f64 = 1056.0;

#[cfg(test)]
mod tests {
    #[test]
    fn test_basic() {
        // Basic module test
        assert_eq!(2 + 2, 4);
    }
}
