//! docling-apple - Apple iWork format support for docling
//!
//! This crate provides parsers for Apple iWork formats:
//! - **Pages** (`.pages`) - Word processing documents
//! - **Numbers** (`.numbers`) - Spreadsheet documents
//! - **Keynote** (`.key`) - Presentation documents
//!
//! ## Format Details
//!
//! Apple iWork files are ZIP archives containing XML files:
//! - `index.xml` - Main document structure (iWork '09 XML format)
//! - `QuickLook/Thumbnail.jpg` - Preview thumbnail
//! - `Data/` - Embedded media files
//!
//! **Note:** This implementation supports the iWork '09 XML format.
//! Modern iWork '13+ files use the IWA (iWork Archive) format which
//! is protobuf-based and requires additional handling.
//!
//! ## Examples
//!
//! Parse a Pages document:
//!
//! ```rust,ignore
//! use docling_apple::PagesBackend;
//! use std::path::Path;
//!
//! let backend = PagesBackend::new();
//! let doc = backend.parse(Path::new("document.pages"))?;
//!
//! // Access document name
//! println!("Document: {}", doc.name);
//!
//! // Iterate over text items (DocItem enum)
//! for item in &doc.texts {
//!     println!("Item: {:?}", item);
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Parse a Numbers spreadsheet:
//!
//! ```rust,ignore
//! use docling_apple::NumbersBackend;
//! use std::path::Path;
//!
//! let backend = NumbersBackend::new();
//! let doc = backend.parse(Path::new("spreadsheet.numbers"))?;
//!
//! // Access extracted tables
//! println!("Tables found: {}", doc.tables.len());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Parse a Keynote presentation:
//!
//! ```rust,ignore
//! use docling_apple::KeynoteBackend;
//! use std::path::Path;
//!
//! let backend = KeynoteBackend::new();
//! let doc = backend.parse(Path::new("presentation.key"))?;
//!
//! // Access slide content
//! println!("Texts found: {}", doc.texts.len());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Architecture
//!
//! All parsers follow the pure Rust architecture:
//! ```text
//! iWork file → ZIP extraction → XML parsing → DocItems → DoclingDocument
//! ```
//!
//! No external dependencies like Python or `LibreOffice` are required.

pub mod common;
pub mod keynote;
pub mod numbers;
pub mod pages;

pub use keynote::KeynoteBackend;
pub use numbers::NumbersBackend;
pub use pages::PagesBackend;
