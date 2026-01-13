//! Document format backends for `docling_rs`
//!
//! This crate provides the core document conversion backends that parse various file
//! formats and convert them into structured [`Document`] types with [`DocItem`]s.
//!
//! # Overview
//!
//! The `docling-backend` crate is the heart of the document extraction system. It contains
//! 30+ backends supporting 60+ document formats that implement the [`DocumentBackend`] trait,
//! enabling consistent document processing across diverse file formats.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                         DocumentConverter                          │
//! │  (orchestrates backends, handles format detection, manages output) │
//! └─────────────────────────────────────────────────────────────────────┘
//!                                    │
//!                                    ▼
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                       DocumentBackend Trait                         │
//! │  fn convert(&self, input: &[u8]) -> Result<Document>               │
//! │  fn supported_formats(&self) -> Vec<InputFormat>                   │
//! └─────────────────────────────────────────────────────────────────────┘
//!                                    │
//!        ┌───────────────────────────┼───────────────────────────┐
//!        ▼                           ▼                           ▼
//! ┌─────────────┐             ┌─────────────┐             ┌─────────────┐
//! │ PdfBackend  │             │ DocxBackend │             │ HtmlBackend │
//! │ (pdfium +   │             │ (zip + xml) │             │ (scraper)   │
//! │  ML models) │             │             │             │             │
//! └─────────────┘             └─────────────┘             └─────────────┘
//! ```
//!
//! # Supported Formats
//!
//! ## Document Formats
//! | Format | Backend | Description |
//! |--------|---------|-------------|
//! | PDF | `PdfBackend` | Portable Document Format with ML-based layout detection |
//! | DOCX | [`DocxBackend`] | Microsoft Word Open XML documents |
//! | PPTX | [`PptxBackend`] | Microsoft `PowerPoint` presentations |
//! | XLSX | [`XlsxBackend`] | Microsoft Excel spreadsheets |
//! | RTF | [`RtfBackend`] | Rich Text Format documents |
//! | ODT/ODS/ODP | [`OpenDocumentBackend`] | `OpenDocument` format family |
//!
//! ## Web Formats
//! | Format | Backend | Description |
//! |--------|---------|-------------|
//! | HTML | [`HtmlBackend`] | HTML documents with semantic extraction |
//! | Markdown | [`MarkdownBackend`] | `CommonMark` and GFM markdown |
//! | `AsciiDoc` | [`AsciidocBackend`] | `AsciiDoc` technical documents |
//! | JSON | [`JsonBackend`] | Structured JSON data |
//! | CSV | [`CsvBackend`] | Comma-separated values |
//!
//! ## Scientific Formats
//! | Format | Backend | Description |
//! |--------|---------|-------------|
//! | JATS | [`JatsBackend`] | Journal Article Tag Suite (`PubMed` XML) |
//! | DICOM | [`DicomBackend`] | Medical imaging metadata |
//! | LaTeX | Separate crate | LaTeX/TeX documents |
//!
//! ## Image Formats
//! | Format | Backend | Description |
//! |--------|---------|-------------|
//! | PNG | [`PngBackend`] | PNG images with EXIF metadata |
//! | JPEG | [`JpegBackend`] | JPEG images with EXIF metadata |
//! | TIFF | [`TiffBackend`] | TIFF images (multi-page support) |
//! | WebP | [`WebpBackend`] | WebP images |
//! | BMP | [`BmpBackend`] | Bitmap images |
//! | HEIF/HEIC | [`HeifBackend`] | High Efficiency Image Format |
//! | AVIF | [`AvifBackend`] | AV1 Image Format |
//! | GIF | [`GifBackend`] | GIF images (first frame) |
//! | SVG | [`SvgBackend`] | Scalable Vector Graphics |
//!
//! ## Specialized Formats
//! | Format | Backend | Description |
//! |--------|---------|-------------|
//! | Archives | [`ArchiveBackend`] | ZIP, TAR, 7Z, RAR |
//! | Ebooks | [`EbooksBackend`] | EPUB, MOBI, AZW3 |
//! | Email | [`EmailBackend`] | EML, MBOX, VCF |
//! | Subtitles | [`SrtBackend`], [`WebvttBackend`] | SRT, `WebVTT` |
//! | Notebooks | [`IpynbBackend`] | Jupyter notebooks |
//! | GPS | [`GpxBackend`], [`KmlBackend`] | GPX, KML/KMZ |
//! | Calendar | [`IcsBackend`] | iCalendar files |
//! | CAD | [`CadBackend`] | DXF, STL, OBJ, PLY, 3MF, STEP |
//!
//! # Usage
//!
//! ## Using `DocumentConverter` (Recommended)
//!
//! The [`DocumentConverter`] is the primary interface for document conversion:
//!
//! ```ignore
//! use docling_backend::DocumentConverter;
//!
//! // Create converter with default options
//! let converter = DocumentConverter::new()?;
//!
//! // Convert a file
//! let result = converter.convert("document.pdf")?;
//!
//! // Access document content
//! println!("Title: {:?}", result.document.metadata.title);
//! println!("Pages: {:?}", result.document.metadata.num_pages);
//!
//! // Get markdown output
//! let markdown = &result.document.markdown;
//! # Ok::<(), docling_core::DoclingError>(())
//! ```
//!
//! ## Using Individual Backends
//!
//! For fine-grained control, use backends directly:
//!
//! ```ignore
//! use docling_backend::{HtmlBackend, traits::DocumentBackend};
//! use std::fs;
//!
//! let backend = HtmlBackend::default();
//! let data = fs::read("document.html")?;
//! let document = backend.convert(&data, Default::default())?;
//!
//! // Access DocItems
//! if let Some(items) = &document.content_blocks {
//!     for item in items {
//!         println!("{:?}: {:?}", item.label, item.text);
//!     }
//! }
//! # Ok::<(), docling_core::DoclingError>(())
//! ```
//!
//! # `DocItem` Types
//!
//! All backends produce [`DocItem`]s with semantic labels:
//!
//! - `Title` - Document titles
//! - `SectionHeader` - Section headings (h1-h6)
//! - `Text` / `Paragraph` - Body text
//! - `ListItem` - Bulleted/numbered list items
//! - `Table` - Tabular data
//! - `Picture` / `Figure` - Images and figures
//! - `Caption` - Figure/table captions
//! - `Code` - Code blocks
//! - `Formula` - Mathematical formulas
//! - `Footnote` - Footnotes and references
//! - `PageHeader` / `PageFooter` - Headers and footers
//!
//! # Performance
//!
//! - **PDF**: ~150ms/page with ML layout detection (`PyTorch` backend)
//! - **Office formats**: ~10-50ms per file
//! - **Images**: ~5-20ms per image (metadata extraction)
//! - **Web formats**: ~1-10ms per file
//!
//! # Feature Flags
//!
//! - `default` - Core backends (HTML, Markdown, images)
//! - `pdf-ml` - PDF with ML layout detection (requires libtorch)
//! - `ocr` - OCR for images and scanned PDFs
//! - `all` - All backends enabled
//!
//! [`Document`]: docling_core::Document
//! [`DocItem`]: docling_core::DocItem
//! [`DocumentBackend`]: traits::DocumentBackend

pub mod archive;
pub mod asciidoc;
pub mod avif;
pub mod bmp;
pub mod cad;
pub mod converter;
pub mod csv;
pub mod dicom;
pub mod docitem_completeness_tests;
pub mod docx;
pub mod docx_numbering;
pub mod ebooks;
pub mod email;
pub mod exif_utils;
pub mod gif;
pub mod gpx;
pub mod heif;
pub mod html;
pub mod ics;
pub mod idml;
pub mod ipynb;
pub mod jats;
pub mod jpeg;
pub mod json;
pub mod kml;
pub mod markdown;
pub mod markdown_helper;
pub mod opendocument;
// Shared PDF constants
pub mod pdf_constants;
#[cfg(feature = "pdf")]
pub mod pdf_fast;
#[cfg(feature = "pdf")]
pub mod pdfium_adapter;
pub mod png;
pub mod pptx;
pub mod rtf;
pub mod srt;
pub mod svg;
pub mod tiff;
pub mod traits;
pub mod utils;
pub mod webp;
pub mod webvtt;
pub mod xlsx;
pub mod xps;

pub use archive::ArchiveBackend;
pub use asciidoc::AsciidocBackend;
pub use avif::AvifBackend;
pub use bmp::BmpBackend;
pub use cad::CadBackend;
pub use converter::*;
pub use csv::CsvBackend;
pub use dicom::DicomBackend;
pub use docx::DocxBackend;
pub use ebooks::EbooksBackend;
pub use email::EmailBackend;
pub use gif::GifBackend;
pub use gpx::GpxBackend;
pub use heif::HeifBackend;
pub use html::HtmlBackend;
pub use ics::IcsBackend;
pub use idml::IdmlBackend;
pub use ipynb::IpynbBackend;
pub use jats::JatsBackend;
pub use jpeg::JpegBackend;
pub use json::JsonBackend;
pub use kml::KmlBackend;
pub use markdown::MarkdownBackend;
pub use opendocument::OpenDocumentBackend;
#[cfg(feature = "pdf")]
pub use pdf_fast::PdfFastBackend;
pub use png::PngBackend;
pub use pptx::PptxBackend;
pub use rtf::RtfBackend;
pub use srt::SrtBackend;
pub use svg::SvgBackend;
pub use tiff::TiffBackend;
pub use traits::*;
pub use webp::WebpBackend;
pub use webvtt::WebvttBackend;
pub use xlsx::XlsxBackend;
pub use xps::XpsBackend;
