//! docling-microsoft-extended - Extended Microsoft format support for docling
//!
//! This crate provides parsers for extended Microsoft formats beyond the
//! core Office suite (Word, Excel, PowerPoint):
//!
//! - **Visio** (`.vsdx`) - Diagram and flowchart files (Office Open XML)
//! - **Publisher** (`.pub`) - Desktop publishing files (via `LibreOffice`)
//! - **Project** (`.mpp`) - Project management files
//! - **`OneNote`** (`.one`) - Note-taking files
//! - **Access** (`.mdb`, `.accdb`) - Database files (out of scope - see note)
//!
//! ## Examples
//!
//! Parse a Visio diagram:
//!
//! ```rust,ignore
//! use docling_microsoft_extended::VisioBackend;
//! use std::path::Path;
//!
//! let backend = VisioBackend::new();
//! let doc = backend.parse(Path::new("flowchart.vsdx"))?;
//!
//! // Access extracted markdown
//! println!("Markdown: {}", doc.markdown);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Convert a Publisher file (requires LibreOffice):
//!
//! ```rust,ignore
//! use docling_microsoft_extended::PublisherBackend;
//! use std::path::Path;
//!
//! let backend = PublisherBackend::new();
//!
//! // Check if LibreOffice is available
//! if backend.is_libreoffice_available() {
//!     let doc = backend.parse(Path::new("brochure.pub"))?;
//!     println!("Markdown: {}", doc.markdown);
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Format Details
//!
//! ### Visio (.vsdx)
//! VSDX files use Office Open XML format (similar to DOCX). They are ZIP
//! archives containing XML files with shape and connection data:
//! - `visio/pages/page*.xml` - Shape text and positions
//! - Shapes are sorted by position (top-to-bottom, left-to-right)
//! - Connections between shapes are preserved
//!
//! ### Publisher (.pub)
//! Publisher files use a proprietary binary format that requires
//! `LibreOffice` for conversion. The backend converts to PDF/ODT first,
//! then extracts content.
//!
//! ### Access (.mdb, .accdb)
//! **Note:** Database formats are out of scope for docling. Access files
//! contain complex relational data that requires SQL query capabilities.
//! Use specialized database tools like `mdbtools` for these formats.
//!
//! ## Architecture
//!
//! Most parsers follow the pure Rust architecture:
//! ```text
//! Office file → ZIP extraction → XML parsing → DocItems → DoclingDocument
//! ```
//!
//! Publisher files require `LibreOffice` due to the proprietary format.

pub mod access;
pub mod onenote;
pub mod project;
pub mod publisher;
pub mod visio;

pub use access::AccessBackend;
pub use onenote::OneNoteBackend;
pub use project::ProjectBackend;
pub use publisher::PublisherBackend;
pub use visio::VisioBackend;
