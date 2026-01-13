//! # docling-ebook
//!
//! E-book format parsers for docling-rs.
//!
//! This crate provides parsing support for popular e-book formats, extracting
//! text content, metadata, and structure for document processing workflows.
//!
//! ## Supported Formats
//!
//! | Format | Extension | Description |
//! |--------|-----------|-------------|
//! | EPUB | `.epub` | Electronic Publication (IDPF/W3C standard) |
//! | FB2 | `.fb2`, `.fb2.zip` | `FictionBook` (Russian/European format) |
//! | MOBI | `.mobi`, `.prc`, `.azw` | Mobipocket/Amazon Kindle |
//!
//! ## Quick Start
//!
//! ### Parse an EPUB File
//!
//! ```rust,no_run
//! use docling_ebook::parse_epub;
//!
//! let book = parse_epub("novel.epub")?;
//!
//! // Access metadata
//! println!("Title: {}", book.metadata.title.unwrap_or_default());
//! println!("Authors: {:?}", book.metadata.creators);
//! println!("Language: {:?}", book.metadata.language);
//!
//! // Read table of contents
//! for entry in &book.toc {
//!     println!("  - {}", entry.label);
//! }
//!
//! // Extract chapter content
//! for chapter in &book.chapters {
//!     println!("Chapter: {:?}", chapter.title);
//!     println!("{}", &chapter.content[..100.min(chapter.content.len())]);
//! }
//! # Ok::<(), docling_ebook::EbookError>(())
//! ```
//!
//! ### Parse an FB2 File
//!
//! ```rust,no_run
//! use docling_ebook::parse_fb2;
//!
//! let book = parse_fb2("russian_novel.fb2")?;
//!
//! // FB2 includes body title separate from metadata
//! if let Some(title) = &book.body_title {
//!     println!("Body title: {}", title);
//! }
//!
//! // Access genre/subjects
//! for subject in &book.metadata.subjects {
//!     println!("Genre: {}", subject);
//! }
//! # Ok::<(), docling_ebook::EbookError>(())
//! ```
//!
//! ### Parse a MOBI File
//!
//! ```rust,no_run
//! use docling_ebook::parse_mobi;
//! use std::fs;
//!
//! // MOBI parser takes bytes, not a path
//! let bytes = fs::read("kindle_book.mobi")?;
//! let book = parse_mobi(&bytes)?;
//!
//! println!("Title: {:?}", book.metadata.title);
//! println!("Chapters: {}", book.chapters.len());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Common Structure
//!
//! All formats are parsed into the same `ParsedEbook` structure:
//!
//! ### `ParsedEbook`
//!
//! | Field | Type | Description |
//! |-------|------|-------------|
//! | `metadata` | `EbookMetadata` | Book metadata |
//! | `body_title` | `Option<String>` | Title page content (FB2) |
//! | `chapters` | `Vec<Chapter>` | Chapters in reading order |
//! | `toc` | `Vec<TocEntry>` | Table of contents |
//! | `page_list` | `Vec<PageTarget>` | Page markers (EPUB) |
//!
//! ### `EbookMetadata`
//!
//! | Field | Type | Description |
//! |-------|------|-------------|
//! | `title` | `Option<String>` | Book title |
//! | `creators` | `Vec<String>` | Authors |
//! | `language` | `Option<String>` | Language code (e.g., "en") |
//! | `identifier` | `Option<String>` | ISBN, UUID, etc. |
//! | `publisher` | `Option<String>` | Publisher name |
//! | `date` | `Option<String>` | Publication date |
//! | `description` | `Option<String>` | Book summary |
//! | `subjects` | `Vec<String>` | Categories/genres |
//! | `rights` | `Option<String>` | Copyright info |
//! | `contributors` | `Vec<String>` | Editors, illustrators |
//!
//! ### Chapter
//!
//! | Field | Type | Description |
//! |-------|------|-------------|
//! | `title` | `Option<String>` | Chapter title |
//! | `content` | `String` | Chapter text (HTML stripped) |
//! | `href` | `String` | Source file path |
//! | `spine_order` | `usize` | Reading order position |
//!
//! ### `TocEntry`
//!
//! | Field | Type | Description |
//! |-------|------|-------------|
//! | `label` | `String` | TOC entry title |
//! | `href` | `String` | Target content file |
//! | `play_order` | `Option<usize>` | Reading sequence |
//! | `children` | `Vec<TocEntry>` | Nested entries |
//!
//! ## Format Details
//!
//! ### EPUB
//!
//! EPUB is the most widely used e-book format:
//! - ZIP archive with XHTML content files
//! - OPF package file with metadata
//! - NCX or Navigation Document for TOC
//! - Supports EPUB 2 and EPUB 3
//!
//! ### FB2 (`FictionBook`)
//!
//! Popular in Russia and Eastern Europe:
//! - Single XML file (or ZIP-compressed)
//! - Inline binary images (base64)
//! - Rich genre and annotation metadata
//! - Structured sections with titles
//!
//! ### MOBI (Mobipocket)
//!
//! Amazon Kindle's original format:
//! - `PalmDOC` compression
//! - MOBI header with metadata
//! - PDB container format
//! - HTML content
//!
//! ## Utility Functions
//!
//! ### HTML to Text Conversion
//!
//! ```rust
//! use docling_ebook::html_to_text;
//!
//! let html = "<p>Hello <b>world</b>!</p>";
//! let text = html_to_text(html);
//! assert_eq!(text.trim(), "Hello world!");
//! ```
//!
//! ## Use Cases
//!
//! - **Text extraction**: Extract content for search indexing
//! - **Format conversion**: Convert between e-book formats
//! - **Metadata cataloging**: Build book catalogs
//! - **Content analysis**: Analyze book structure and length
//! - **Accessibility**: Convert to accessible formats
//!
//! ## Error Handling
//!
//! ```rust,no_run
//! use docling_ebook::{parse_epub, EbookError};
//!
//! match parse_epub("book.epub") {
//!     Ok(book) => println!("Parsed {} chapters", book.chapters.len()),
//!     Err(EbookError::IoError(e)) => println!("File error: {}", e),
//!     Err(EbookError::ZipError(e)) => println!("Archive error: {}", e),
//!     Err(EbookError::XmlParse(e)) => println!("XML error: {}", e),
//!     Err(e) => println!("Error: {}", e),
//! }
//! ```

/// EPUB format parser (Electronic Publication)
pub mod epub;
/// Error types for e-book parsing
pub mod error;
/// FB2 format parser (`FictionBook`)
pub mod fb2;
/// MOBI format parser (Mobipocket/Kindle)
pub mod mobi;
/// Common types for parsed e-book content
pub mod types;

// Re-export commonly used items
pub use epub::{html_to_text, parse_epub};
pub use error::{EbookError, Result};
pub use fb2::parse_fb2;
pub use mobi::parse_mobi;
pub use types::{Chapter, EbookMetadata, ParsedEbook, TocEntry};
