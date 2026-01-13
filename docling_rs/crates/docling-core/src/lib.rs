//! # Docling Core - Document Conversion Library
//!
//! Docling is a powerful document conversion library that extracts structured content
//! from various document formats including PDF, Microsoft Office, HTML, and more.
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! // Note: DocumentConverter is in docling-backend crate
//! use docling_backend::{DocumentConverter, Result};
//!
//! fn main() -> Result<()> {
//!     // Create a converter (text-only mode by default)
//!     let converter = DocumentConverter::new()?;
//!
//!     // Convert a PDF to markdown
//!     let result = converter.convert("document.pdf")?;
//!
//!     println!("Markdown output:\n{}", result.document.markdown);
//!     println!("Pages: {:?}", result.document.metadata.num_pages);
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Features
//!
//! - **60+ Document Formats**: PDF, DOCX, PPTX, XLSX, HTML, Markdown, images, and more
//! - **Pure Rust/C++ Implementation**: No Python runtime required for production use
//! - **OCR Support**: Optical character recognition for scanned documents (via pdfium)
//! - **Table Extraction**: Intelligent table detection and conversion with rich cell support
//! - **Structured Output**: JSON, YAML, or Markdown with preserved document structure
//! - **High Performance**: Native Rust implementation with C++ library integration via FFI
//!
//! ## Supported Formats (60+)
//!
//! | Category | Formats |
//! |----------|---------|
//! | **Office Documents** | DOCX, XLSX, PPTX, XPS, DOC, RTF, Publisher, Project, `OneNote` |
//! | **Apple iWork** | Pages, Numbers, Keynote |
//! | **`OpenDocument`** | ODT, ODS, ODP |
//! | **Web & Markup** | HTML, Markdown, `AsciiDoc`, LaTeX |
//! | **PDF** | PDF (pdfium-based, with optional OCR) |
//! | **Images** | PNG, JPEG, TIFF, GIF, BMP, WebP, HEIF, AVIF, SVG |
//! | **E-books** | EPUB, FB2, MOBI |
//! | **Scientific** | JATS XML, DICOM |
//! | **Geospatial** | GPX, KML, KMZ |
//! | **CAD & 3D** | DXF, DWG, STL, OBJ, GLTF, GLB |
//! | **Communication** | EML, MBOX, MSG, VCF, ICS |
//! | **Data** | CSV, JSON, IPYNB (Jupyter) |
//! | **Media** | SRT, `WebVTT` (subtitles) |
//! | **Archives** | ZIP, TAR, 7Z, RAR, ISO |
//! | **Other** | IDML (`InDesign`), Visio (VSDX), Access (MDB), Genomics (VCF) |
//!
//! ## Examples
//!
//! ### Basic Document Conversion
//!
//! ```rust,ignore
//! // Note: DocumentConverter is in docling-backend crate
//! use docling_backend::{DocumentConverter, Result};
//!
//! fn main() -> Result<()> {
//!     let converter = DocumentConverter::new()?;
//!     let result = converter.convert("example.pdf")?;
//!
//!     // Access markdown output
//!     println!("{}", result.document.to_markdown());
//!
//!     Ok(())
//! }
//! ```
//!
//! ### OCR for Scanned Documents
//!
//! ```rust,ignore
//! // Note: DocumentConverter is in docling-backend crate
//! use docling_backend::{DocumentConverter, Result};
//!
//! fn main() -> Result<()> {
//!     // Enable OCR for scanned PDFs and images
//!     let converter = DocumentConverter::with_ocr(true)?;
//!     let result = converter.convert("scanned_document.pdf")?;
//!
//!     println!("Extracted text: {}", result.document.markdown);
//!
//!     Ok(())
//! }
//! ```
//!
//! ### Structured Content Access
//!
//! ```rust,ignore
//! // Note: DocumentConverter is in docling-backend crate
//! use docling_backend::{DocumentConverter, Result};
//!
//! fn main() -> Result<()> {
//!     let converter = DocumentConverter::new()?;
//!     let result = converter.convert("report.pdf")?;
//!
//!     // Check if structured content is available
//!     if result.document.has_structured_content() {
//!         if let Some(blocks) = result.document.blocks() {
//!             println!("Found {} content blocks", blocks.len());
//!         }
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ### Batch Processing
//!
//! ```rust,ignore
//! // Note: DocumentConverter is in docling-backend crate
//! use docling_backend::{DocumentConverter, Result};
//! use std::path::Path;
//!
//! fn main() -> Result<()> {
//!     let converter = DocumentConverter::new()?;
//!
//!     let files = vec!["doc1.pdf", "doc2.docx", "doc3.html"];
//!
//!     for file in files {
//!         match converter.convert(file) {
//!             Ok(result) => {
//!                 println!("✓ {}: {} chars",
//!                     file,
//!                     result.document.metadata.num_characters
//!                 );
//!             }
//!             Err(e) => {
//!                 eprintln!("✗ {}: {}", file, e);
//!             }
//!         }
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ### Custom Serialization
//!
//! ```rust,ignore
//! // Note: DocumentConverter is in docling-backend crate
//! use docling_backend::DocumentConverter;
//! use docling_core::{MarkdownOptions, MarkdownSerializer, Result};
//!
//! fn main() -> Result<()> {
//!     let converter = DocumentConverter::new()?;
//!     let result = converter.convert("document.pdf")?;
//!
//!     // Customize markdown output
//!     let options = MarkdownOptions {
//!         indent: 2,
//!         escape_underscores: false,
//!         escape_html: true,
//!         ..Default::default()  // Use defaults for include_furniture, max_list_depth
//!     };
//!
//!     let serializer = MarkdownSerializer::with_options(options);
//!     // let markdown = serializer.serialize(&result.document);
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Module Organization
//!
//! - `DocumentConverter` (in `docling-backend` crate) - Main document conversion API
//! - [`document`] - Core document types and metadata
//! - [`content`] - Structured content representation
//! - [`serializer`] - Output format serializers (Markdown, JSON, YAML)
//! - [`mod@format`] - Input format detection
//! - [`error`] - Error types and handling
//!
//! ## Performance
//!
//! Docling is designed for high-throughput document processing:
//!
//! - **Rust Backend**: 5-10x faster than pure Python for supported formats
//! - **Streaming API**: Process large documents with minimal memory
//! - **Batch Mode**: Efficiently process hundreds of files
//! - **Multi-threading**: Parallel conversion support
//!
//! See the [performance guide](../../../docs/guides/performance.md) for optimization tips.
//!
//! ## Error Handling
//!
//! All conversion operations return a [`Result<T, DoclingError>`](error::DoclingError):
//!
//! ```rust,ignore
//! // Note: DocumentConverter is in docling-backend crate
//! use docling_backend::DocumentConverter;
//! use docling_core::{DoclingError, Result};
//!
//! fn convert_with_error_handling(path: &str) -> Result<String> {
//!     let converter = DocumentConverter::new()?;
//!
//!     match converter.convert(path) {
//!         Ok(result) => Ok(result.document.markdown),
//!         Err(DoclingError::FileNotFound(msg)) => {
//!             eprintln!("File not found: {}", msg);
//!             Err(DoclingError::FileNotFound(msg))
//!         }
//!         Err(DoclingError::UnsupportedFormat(msg)) => {
//!             eprintln!("Unsupported format: {}", msg);
//!             Err(DoclingError::UnsupportedFormat(msg))
//!         }
//!         Err(e) => {
//!             eprintln!("Conversion error: {}", e);
//!             Err(e)
//!         }
//!     }
//! }
//! ```
//!
//! ## See Also
//!
//! - [User Guide](../../../docs/USER_GUIDE.md) - Comprehensive usage documentation
//! - [Format Guide](../../../docs/formats/) - Format-specific documentation
//! - [Migration Guide](../../../docs/guides/migration.md) - Python to Rust migration
//! - [API Cookbook](../../../docs/API_COOKBOOK.md) - Common usage patterns

// Core modules
pub mod adobe;
// NOTE: apple module commented out due to circular dependency
// docling-apple now depends on docling-core (for DocItem types)
// Use docling-backend converter which imports both packages
// pub mod apple;
// archive module removed - functionality moved to docling-backend/archive.rs
// Use ArchiveBackend from docling-backend instead
pub mod audio;
pub mod cad;
pub mod calendar;
pub mod content;
// converter module removed with python-bridge
// Use RustDocumentConverter (alias: DocumentConverter) from docling-backend instead
pub mod dicom;
pub mod doc;
pub mod document;
/// E-book format support (EPUB, MOBI, FB2)
pub mod ebook;
/// Email format support (EML, MSG, MBOX, VCF)
pub mod email;
pub mod error;
pub mod format;
pub mod gps;
pub mod kml;
pub mod legacy;
pub mod notebook;
/// `OpenDocument` format support (ODT, ODS, ODP)
pub mod opendocument;
// performance module removed with python-bridge
// python-bridge REMOVED - Module archived to archive/python/python_bridge.rs.deprecated
// Use pure Rust ML implementation instead: docling-pdf-ml crate with pdf-ml feature
pub mod serializer;
pub mod svg;
pub mod types;
pub mod video;
pub mod xps;

// Re-exports for convenience
pub use content::*;
// converter re-export removed - use docling-backend::DocumentConverter instead
pub use document::*;
pub use error::*;
pub use format::*;
// performance and python_bridge re-exports removed with python-bridge feature
pub use serializer::*;
