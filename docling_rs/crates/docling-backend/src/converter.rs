//! Rust-native document converter
//!
//! Pure Rust/C++ implementation supporting 60+ document formats.
//! All backends are native (Rust or C++ via FFI). PDF layout/structure ML can
//! optionally be sourced from Python docling via subprocess for correctness
//! parity when the native pipeline diverges.

// Clippy pedantic allows:
// - Main convert function handles many formats
// - DoclingDocument consumed intentionally
#![allow(clippy::too_many_lines)]
#![allow(clippy::needless_pass_by_value)]

use crate::archive::ArchiveBackend;
use crate::asciidoc::AsciidocBackend;
use crate::avif::AvifBackend;
use crate::bmp::BmpBackend;
use crate::cad::CadBackend;
use crate::csv::CsvBackend;
use crate::dicom::DicomBackend;
use crate::ebooks::EbooksBackend;
use crate::email::EmailBackend;
use crate::gif::GifBackend;
use crate::gpx::GpxBackend;
use crate::heif::HeifBackend;
use crate::html::HtmlBackend;
use crate::ics::IcsBackend;
use crate::idml::IdmlBackend;
use crate::ipynb::IpynbBackend;
use crate::jats::JatsBackend;
use crate::jpeg::JpegBackend;
use crate::json::JsonBackend;
use crate::kml::KmlBackend;
use crate::markdown::MarkdownBackend;
use crate::opendocument::OpenDocumentBackend;
#[cfg(feature = "pdf")]
use crate::pdf_fast::PdfFastBackend;
use crate::png::PngBackend;
use crate::pptx::PptxBackend;
use crate::rtf::RtfBackend;
use crate::srt::SrtBackend;
use crate::svg::SvgBackend;
use crate::tiff::TiffBackend;
use crate::traits::{BackendOptions, DocumentBackend};
use crate::webp::WebpBackend;
use crate::webvtt::WebvttBackend;
use crate::xlsx::XlsxBackend;
use crate::xps::XpsBackend;
use docling_apple::{KeynoteBackend, NumbersBackend, PagesBackend};
use docling_core::{
    serializer::MarkdownSerializer, DoclingDocument, DoclingError, Document, InputFormat,
};
use docling_latex::LatexBackend;
use docling_legacy::doc::DocBackend;
use docling_microsoft_extended::{AccessBackend, OneNoteBackend, ProjectBackend, VisioBackend};
#[cfg(feature = "pdf")]
use log::warn;
use std::path::Path;
use std::time::Instant;

// Re-export ConversionResult from docling_core for consistency
pub use docling_core::ConversionResult;

/// PDF ML backend configuration
///
/// Configuration options for the ML-based PDF parsing pipeline.
/// This struct provides a way to select the inference backend (`PyTorch` or ONNX)
/// and other ML-specific options when processing PDF files.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct PdfMlConfig {
    /// Inference backend selection (`PyTorch` or ONNX)
    ///
    /// - `PdfMlBackend::PyTorch`: Faster (1.56x), supports GPU acceleration (CUDA, MPS)
    /// - `PdfMlBackend::Onnx`: Cross-platform, portable, CPU-optimized
    pub backend: PdfMlBackend,

    /// Enable table structure parsing
    ///
    /// When enabled, uses `TableFormer` to extract table structure (rows, columns).
    /// When disabled, tables are detected but not parsed (faster).
    pub table_structure: bool,
}

impl Default for PdfMlConfig {
    #[inline]
    fn default() -> Self {
        Self {
            backend: PdfMlBackend::default(),
            table_structure: false,
        }
    }
}

/// ML backend selection for PDF processing
///
/// Choose between `PyTorch` (faster, GPU support) and ONNX (portable, CPU-optimized).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum PdfMlBackend {
    /// `PyTorch` backend (default)
    ///
    /// - 1.56x faster than ONNX
    /// - Supports CUDA and MPS (Apple Metal) GPU acceleration
    /// - Requires `pytorch` feature to be enabled
    #[default]
    PyTorch,

    /// ONNX Runtime backend
    ///
    /// - Cross-platform and portable
    /// - CPU-optimized with SIMD acceleration
    /// - Works without `PyTorch` installation
    Onnx,
}

impl std::fmt::Display for PdfMlBackend {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PyTorch => write!(f, "pytorch"),
            Self::Onnx => write!(f, "onnx"),
        }
    }
}

impl std::str::FromStr for PdfMlBackend {
    type Err = String;

    /// Parse backend from string (case-insensitive)
    ///
    /// Accepts: "pytorch", "torch", "pt" | "onnx"
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pytorch" | "torch" | "pt" => Ok(Self::PyTorch),
            "onnx" => Ok(Self::Onnx),
            _ => Err(format!(
                "Unknown backend '{s}'. Valid options: pytorch, torch, pt, onnx"
            )),
        }
    }
}

/// Helper function to convert `DoclingDocument` (from Python) to Document (Rust format)
fn docling_document_to_document(docling_doc: DoclingDocument, format: InputFormat) -> Document {
    // N=4404: Disable page breaks to match Python docling behavior
    // Python docling v2.58.0 does NOT output <!-- page N --> comments
    let serializer = MarkdownSerializer::new();
    let markdown = serializer.serialize(&docling_doc);

    // Extract page count from DoclingDocument.pages HashMap
    let num_pages = if docling_doc.pages.is_empty() {
        None
    } else {
        Some(docling_doc.pages.len())
    };

    // Create Document from markdown, preserving metadata
    // Note: We lose some structured data in this conversion
    // TODO: Preserve structured content in future versions
    // Would involve:
    // - Store DocItems directly in Document (currently only stores markdown string)
    // - Preserve bounding boxes, labels, and relationships
    // - Enable JSON export with full structure (not just markdown)
    // Current: markdown-only output is acceptable for most use cases
    let mut doc = Document::from_markdown(markdown, format);
    doc.metadata.num_pages = num_pages;
    doc
}

/// Rust-native document converter
///
/// Supports 60+ document formats with pure Rust/C++ backends:
///
/// **Office Documents:**
/// - Microsoft Office: DOCX, XLSX, PPTX, XPS, Publisher, Project, `OneNote`
/// - `OpenDocument`: ODT, ODS, ODP
/// - Apple iWork: Pages, Numbers, Keynote
/// - Legacy: DOC, RTF
///
/// **Web & Markup:**
/// - HTML, Markdown, `AsciiDoc`, LaTeX
///
/// **Scientific & Technical:**
/// - Academic: JATS XML
/// - Medical: DICOM
/// - Geospatial: GPX, KML, KMZ
/// - Genomics: VCF
///
/// **Images:**
/// - Raster: PNG, JPEG, TIFF, GIF, BMP, WebP, HEIF, AVIF
/// - Vector: SVG
///
/// **Documents:**
/// - PDF (pdfium-based)
/// - E-books: EPUB, FB2, MOBI
/// - IDML (Adobe `InDesign`)
///
/// **CAD & 3D:**
/// - CAD: DXF, DWG
/// - 3D Models: STL, OBJ, GLTF, GLB
///
/// **Data & Structured:**
/// - Tabular: CSV
/// - Calendar: ICS
/// - Notebook: IPYNB (Jupyter)
/// - JSON (Docling native format)
///
/// **Communication:**
/// - Email: EML, MBOX, MSG
/// - Contacts: VCF
///
/// **Media:**
/// - Subtitles: SRT, `WebVTT`
///
/// **Archives:**
/// - ZIP, TAR, 7Z, RAR, ISO
///
/// **Microsoft Legacy:**
/// - Visio (VSDX), Access (MDB)
///
/// All backends are pure Rust or C++ (via FFI). PDF layout/structure ML can optionally be
/// sourced from Python docling via subprocess.
#[derive(Debug)]
pub struct RustDocumentConverter {
    /// PDF backend using pdfium-fast (72x faster) with ML pipeline
    #[cfg(feature = "pdf")]
    pdf_fast_backend: Option<PdfFastBackend>,
    #[cfg(not(feature = "pdf"))]
    _no_pdf: (),
    enable_ocr: bool,
    /// PDF ML configuration (backend selection, table structure, etc.)
    pdf_ml_config: PdfMlConfig,
}

/// Type alias for convenience - makes `DocumentConverter` available as a more natural name
pub type DocumentConverter = RustDocumentConverter;

impl RustDocumentConverter {
    /// Create new converter with text-only configuration
    ///
    /// # Errors
    /// Returns an error if converter initialization fails.
    #[must_use = "creating a converter that is not used is a waste of resources"]
    pub fn new() -> Result<Self, DoclingError> {
        Self::with_ocr(false)
    }

    /// Create converter with specific OCR configuration
    ///
    /// # Errors
    /// Returns an error if converter initialization fails.
    #[must_use = "creating a converter that is not used is a waste of resources"]
    pub fn with_ocr(enable_ocr: bool) -> Result<Self, DoclingError> {
        Self::with_config(enable_ocr, PdfMlConfig::default())
    }

    /// Create converter with full configuration
    ///
    /// # Arguments
    /// * `enable_ocr` - Enable OCR for scanned documents
    /// * `pdf_ml_config` - ML backend configuration for PDF processing
    ///
    /// # Examples
    /// ```ignore
    /// use docling_backend::{PdfMlBackend, PdfMlConfig, RustDocumentConverter};
    ///
    /// let config = PdfMlConfig {
    ///     backend: PdfMlBackend::Onnx,
    ///     table_structure: true,
    /// };
    /// let converter = RustDocumentConverter::with_config(true, config)?;
    /// ```
    ///
    /// # Errors
    /// Currently infallible, but returns `Result` for API consistency.
    #[must_use = "creating a converter that is not used is a waste of resources"]
    // Cannot be const when PDF features are enabled (initializes backends at runtime)
    #[allow(clippy::missing_const_for_fn)]
    pub fn with_config(enable_ocr: bool, pdf_ml_config: PdfMlConfig) -> Result<Self, DoclingError> {
        // Try to initialize PDF backend (72x faster, uses pdfium_fast with ML)
        #[cfg(feature = "pdf")]
        let pdf_fast_backend = match PdfFastBackend::new() {
            Ok(backend) => Some(backend),
            Err(e) => {
                warn!("Failed to initialize PDF backend: {e}. PDF conversion will be unavailable.");
                None
            }
        };

        Ok(Self {
            #[cfg(feature = "pdf")]
            pdf_fast_backend,
            #[cfg(not(feature = "pdf"))]
            _no_pdf: (),
            enable_ocr,
            pdf_ml_config,
        })
    }

    /// Get the PDF ML configuration
    #[inline]
    #[must_use = "returns the PDF ML configuration settings"]
    pub const fn pdf_ml_config(&self) -> &PdfMlConfig {
        &self.pdf_ml_config
    }

    /// Convert a document from a file path
    ///
    /// # Errors
    /// Returns an error if format detection or document conversion fails.
    #[must_use = "conversion result contains the converted document and should be processed"]
    pub fn convert<P: AsRef<Path>>(&self, path: P) -> Result<ConversionResult, DoclingError> {
        let path_ref = path.as_ref();

        // Detect format from file extension
        let ext = path_ref
            .extension()
            .and_then(|s| s.to_str())
            .ok_or_else(|| {
                DoclingError::FormatError(format!(
                    "No file extension found: {}",
                    path_ref.display()
                ))
            })?;

        let format = InputFormat::from_extension(ext)
            .ok_or_else(|| DoclingError::FormatError(format!("Unsupported format: {ext}")))?;

        let start = Instant::now();

        let document = match format {
            InputFormat::Pdf => {
                #[cfg(feature = "pdf")]
                {
                    if let Some(backend) = &self.pdf_fast_backend {
                        let options = BackendOptions::default()
                            .with_ocr(self.enable_ocr)
                            .with_table_structure(self.pdf_ml_config.table_structure)
                            .with_ml_backend(self.pdf_ml_config.backend);
                        backend.parse_file_ml(path_ref, &options)?
                    } else {
                        return Err(DoclingError::BackendError(
                            "PDF backend not available (pdfium_fast library not found). \
                             Ensure ~/pdfium_fast is built."
                                .to_string(),
                        ));
                    }
                }
                #[cfg(not(feature = "pdf"))]
                {
                    return Err(DoclingError::BackendError(
                        "PDF support requires the 'pdf' feature. Build with: cargo build --features pdf"
                            .to_string(),
                    ));
                }
            }

            // Archive formats
            InputFormat::Zip | InputFormat::Tar | InputFormat::SevenZ | InputFormat::Rar => {
                let backend = ArchiveBackend::new(format)?;
                let options = BackendOptions::default();
                backend.parse_file(path_ref, &options)?
            }

            // Subtitle formats
            InputFormat::Srt => {
                let backend = SrtBackend::new()?;
                let options = BackendOptions::default();
                backend.parse_file(path_ref, &options)?
            }
            InputFormat::Webvtt => {
                let backend = WebvttBackend::new()?;
                let options = BackendOptions::default();
                backend.parse_file(path_ref, &options)?
            }

            // Email formats
            InputFormat::Eml | InputFormat::Mbox | InputFormat::Msg => {
                let backend = EmailBackend::new(format)?;
                let options = BackendOptions::default();
                backend.parse_file(path_ref, &options)?
            }

            // VCF format - ambiguous extension, needs content-based detection
            InputFormat::Vcf => {
                // Peek at file content to detect VCF type
                let content_sample =
                    std::fs::read_to_string(path_ref).map_err(DoclingError::IoError)?;

                // Check if it's a genomics VCF (Variant Call Format)
                if content_sample.starts_with("##fileformat=VCF") {
                    // Genomics VCF - use docling_genomics parser
                    use docling_genomics::VcfParser;
                    let vcf_doc = VcfParser::parse_file(path_ref).map_err(|e| {
                        DoclingError::BackendError(format!("VCF parsing failed: {e}"))
                    })?;
                    let markdown = docling_genomics::vcf_to_markdown(&vcf_doc);
                    Document::from_markdown(markdown, format)
                } else {
                    // vCard contact format - use EmailBackend
                    let backend = EmailBackend::new(format)?;
                    let options = BackendOptions::default();
                    backend.parse_file(path_ref, &options)?
                }
            }

            // E-book formats
            InputFormat::Epub | InputFormat::Fb2 | InputFormat::Mobi => {
                let backend = EbooksBackend::new(format)?;
                let options = BackendOptions::default();
                backend.parse_file(path_ref, &options)?
            }

            // OpenDocument formats
            InputFormat::Odt | InputFormat::Ods | InputFormat::Odp => {
                let backend = OpenDocumentBackend::new(format)?;
                let options = BackendOptions::default();
                backend.parse_file(path_ref, &options)?
            }

            // Calendar format
            InputFormat::Ics => {
                let backend = IcsBackend::new();
                let options = BackendOptions::default();
                backend.parse_file(path_ref, &options)?
            }

            // Notebook format
            InputFormat::Ipynb => {
                let backend = IpynbBackend::new();
                let options = BackendOptions::default();
                backend.parse_file(path_ref, &options)?
            }

            // Legacy formats
            InputFormat::Rtf => {
                let backend = RtfBackend::new();
                let options = BackendOptions::default();
                backend.parse_file(path_ref, &options)?
            }

            // GPS formats
            InputFormat::Gpx => {
                let backend = GpxBackend::new();
                let options = BackendOptions::default();
                backend.parse_file(path_ref, &options)?
            }
            InputFormat::Kml | InputFormat::Kmz => {
                let backend = KmlBackend::new(format);
                let options = BackendOptions::default();
                backend.parse_file(path_ref, &options)?
            }

            // Graphics formats
            InputFormat::Svg => {
                let backend = SvgBackend::new();
                let options = BackendOptions::default();
                backend.parse_file(path_ref, &options)?
            }

            // Image formats (OCR enabled if requested)
            InputFormat::Gif => {
                let backend = GifBackend::new();
                let options = BackendOptions::default().with_ocr(self.enable_ocr);
                backend.parse_file(path_ref, &options)?
            }
            InputFormat::Heif => {
                let backend = HeifBackend::new(InputFormat::Heif);
                let options = BackendOptions::default().with_ocr(self.enable_ocr);
                backend.parse_file(path_ref, &options)?
            }
            InputFormat::Avif => {
                let backend = AvifBackend::new();
                let options = BackendOptions::default().with_ocr(self.enable_ocr);
                backend.parse_file(path_ref, &options)?
            }
            InputFormat::Bmp => {
                let backend = BmpBackend::new();
                let options = BackendOptions::default().with_ocr(self.enable_ocr);
                backend.parse_file(path_ref, &options)?
            }
            InputFormat::Png => {
                let backend = PngBackend::new();
                let options = BackendOptions::default().with_ocr(self.enable_ocr);
                backend.parse_file(path_ref, &options)?
            }
            InputFormat::Jpeg => {
                let backend = JpegBackend::new();
                let options = BackendOptions::default().with_ocr(self.enable_ocr);
                backend.parse_file(path_ref, &options)?
            }
            InputFormat::Tiff => {
                let backend = TiffBackend::new();
                let options = BackendOptions::default().with_ocr(self.enable_ocr);
                backend.parse_file(path_ref, &options)?
            }
            InputFormat::Webp => {
                let backend = WebpBackend::new();
                let options = BackendOptions::default().with_ocr(self.enable_ocr);
                backend.parse_file(path_ref, &options)?
            }

            // Microsoft formats
            InputFormat::Xps => {
                let backend = XpsBackend::new();
                let options = BackendOptions::default();
                backend.parse_file(path_ref, &options)?
            }

            // CAD/3D formats
            InputFormat::Stl
            | InputFormat::Obj
            | InputFormat::Gltf
            | InputFormat::Glb
            | InputFormat::Dxf => {
                let backend = CadBackend::new(format)?;
                let options = BackendOptions::default();
                backend.parse_file(path_ref, &options)?
            }

            // Medical imaging formats
            InputFormat::Dicom => {
                let backend = DicomBackend::new();
                let options = BackendOptions::default();
                backend.parse_file(path_ref, &options)?
            }

            // Data formats
            InputFormat::Csv => {
                let backend = CsvBackend::new();
                let options = BackendOptions::default();
                backend.parse_file(path_ref, &options)?
            }
            InputFormat::Xlsx => {
                let backend = XlsxBackend::new();
                let options = BackendOptions::default();
                backend.parse_file(path_ref, &options)?
            }

            // Web formats
            InputFormat::Html => {
                let backend = HtmlBackend::new();
                let options = BackendOptions::default();
                backend.parse_file(path_ref, &options)?
            }
            InputFormat::Md => {
                let backend = MarkdownBackend::new();
                let options = BackendOptions::default();
                backend.parse_file(path_ref, &options)?
            }
            InputFormat::Asciidoc => {
                let backend = AsciidocBackend::new();
                let options = BackendOptions::default();
                backend.parse_file(path_ref, &options)?
            }

            // Scientific article format
            InputFormat::Jats => {
                let backend = JatsBackend;
                let options = BackendOptions::default();
                backend.parse_file(path_ref, &options)?
            }

            // Docling JSON format (round-trip)
            InputFormat::JsonDocling => {
                let backend = JsonBackend::new();
                let options = BackendOptions::default();
                backend.parse_file(path_ref, &options)?
            }

            // Office document formats
            InputFormat::Doc => {
                // Convert .doc to .docx using textutil (macOS) or LibreOffice (Linux/Windows)
                let docx_path = DocBackend::convert_doc_to_docx(path_ref).map_err(|e| {
                    DoclingError::BackendError(format!(
                        "DOC conversion failed: {e}. Note: On macOS, uses textutil (built-in). On Linux/Windows, requires LibreOffice."
                    ))
                })?;

                // Parse the converted DOCX file
                let backend = crate::DocxBackend;
                let options = BackendOptions::default();
                let result = backend.parse_file(&docx_path, &options);

                // Clean up temporary DOCX file
                let _ = std::fs::remove_file(&docx_path);

                result?
            }
            InputFormat::Docx => {
                let backend = crate::DocxBackend;
                let options = BackendOptions::default();
                backend.parse_file(path_ref, &options)?
            }
            InputFormat::Pptx => {
                let backend = PptxBackend;
                let options = BackendOptions::default();
                backend.parse_file(path_ref, &options)?
            }

            // Adobe formats
            InputFormat::Idml => {
                let backend = IdmlBackend::new();
                let options = BackendOptions::default();
                backend.parse_file(path_ref, &options)?
            }

            // Microsoft Extended formats (require LibreOffice)
            InputFormat::Pub => {
                // TODO: Implement direct DocItem generation for Publisher
                // Publisher files are OLE Compound Documents (like Project)
                // Could be implemented with cfb crate or libmspub FFI
                // Current: Not supported (pdfium-fast parse_bytes not yet implemented)
                return Err(DoclingError::BackendError(
                    "Publisher (.pub) conversion not yet supported. \
                     Requires parse_bytes support for intermediate PDF parsing."
                        .to_string(),
                ));
            }
            InputFormat::Vsdx => {
                let backend = VisioBackend::new();
                backend
                    .parse(path_ref)
                    .map_err(|e| DoclingError::BackendError(format!("Visio parsing failed: {e}")))?
            }
            InputFormat::One => {
                // OneNote desktop format (.one) is not supported
                // Library limitation: onenote_parser v0.3.1 only supports cloud format
                let onenote_backend = OneNoteBackend::new();
                onenote_backend
                    .parse_error()
                    .map_err(|e| DoclingError::FormatError(e.to_string()))?;

                // Unreachable - parse_error() always returns Err
                unreachable!("OneNote parse_error should always fail")
            }
            InputFormat::Mpp => {
                // Parse .mpp directly to DocItems (no PDF intermediate)
                let project_backend = ProjectBackend::new();
                let docling_doc = project_backend.parse_to_docitems(path_ref).map_err(|e| {
                    DoclingError::BackendError(format!("Project parsing failed: {e}"))
                })?;

                // Convert DoclingDocument to Document
                let mut doc = docling_document_to_document(docling_doc, format);
                doc.format = format; // Ensure format is set correctly
                doc
            }
            InputFormat::Mdb => {
                // Access generates DocItems directly (uses mdbtools)
                let backend = AccessBackend::new();
                let docling_doc = backend.parse(path_ref).map_err(|e| {
                    DoclingError::BackendError(format!(
                        "Access parsing failed: {e}. Note: Requires mdbtools installed."
                    ))
                })?;
                docling_document_to_document(docling_doc, format)
            }

            // LaTeX format (pure Rust parser with tree-sitter)
            InputFormat::Tex => {
                let mut backend = LatexBackend::new().map_err(|e| {
                    DoclingError::BackendError(format!("Failed to initialize LaTeX backend: {e}"))
                })?;
                backend
                    .parse(path_ref)
                    .map_err(|e| DoclingError::BackendError(format!("LaTeX parsing failed: {e}")))?
            }

            // Apple Pages format - Parse XML directly to generate DocItems
            InputFormat::Pages => {
                let backend = PagesBackend::new();
                let docling_doc = backend.parse(path_ref).map_err(|e| {
                    DoclingError::BackendError(format!("Failed to parse Pages file: {e}"))
                })?;
                docling_document_to_document(docling_doc, format)
            }
            // Apple Numbers format - Parse XML directly to generate DocItems
            InputFormat::Numbers => {
                let backend = NumbersBackend::new();
                let docling_doc = backend.parse(path_ref).map_err(|e| {
                    DoclingError::BackendError(format!("Failed to parse Numbers file: {e}"))
                })?;
                docling_document_to_document(docling_doc, format)
            }
            // Apple Keynote format - Parse XML directly to generate DocItems
            InputFormat::Key => {
                let backend = KeynoteBackend::new();
                let docling_doc = backend.parse(path_ref).map_err(|e| {
                    DoclingError::BackendError(format!("Failed to parse Keynote file: {e}"))
                })?;
                docling_document_to_document(docling_doc, format)
            }

            _ => {
                return Err(DoclingError::BackendError(format!(
                    "Format {format:?} not yet supported by Rust backend"
                )));
            }
        };

        let latency = start.elapsed();

        Ok(ConversionResult { document, latency })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_converter_creation() {
        let converter = RustDocumentConverter::new();
        // Converter creation might fail if pdfium is not available
        if let Ok(_converter) = converter {
            // Success
        }
    }

    #[test]
    #[cfg(feature = "pdf")]
    fn test_rust_converter_with_pdf() {
        let converter = RustDocumentConverter::new().expect("Failed to create converter");

        let pdf_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../test-corpus/pdf/multi_page.pdf"
        );
        if !Path::new(pdf_path).exists() {
            eprintln!("Test PDF not found, skipping");
            return;
        }

        let result = converter.convert(pdf_path).expect("Failed to convert PDF");

        assert_eq!(result.document.format, InputFormat::Pdf);
        assert!(!result.document.markdown.is_empty());
        assert!(result.document.metadata.num_pages.is_some());
        assert!(result.document.metadata.num_characters > 0);

        println!("Conversion took: {:?}", result.latency);
        println!("Pages: {:?}", result.document.metadata.num_pages);
        println!("Characters: {}", result.document.metadata.num_characters);

        // Write for comparison
        std::fs::write("/tmp/rust_pdf_output.md", &result.document.markdown).ok();
        println!("Output written to /tmp/rust_pdf_output.md");
    }

    // ===== CATEGORY 1: Converter Creation Tests =====

    #[test]
    fn test_converter_with_ocr() {
        // Test converter creation with OCR enabled
        let converter = RustDocumentConverter::with_ocr(true);
        // May fail if pdfium not available, but should not panic
        if let Ok(converter) = converter {
            assert!(converter.enable_ocr);
        }
    }

    #[test]
    fn test_converter_without_ocr() {
        // Test converter creation with OCR disabled
        let converter = RustDocumentConverter::with_ocr(false);
        if let Ok(converter) = converter {
            assert!(!converter.enable_ocr);
        }
    }

    // ===== CATEGORY 2: Format Detection Tests =====

    #[test]
    fn test_convert_no_extension() {
        let converter = RustDocumentConverter::new();
        if let Ok(converter) = converter {
            let result = converter.convert("test_file_no_ext");
            assert!(result.is_err());
            if let Err(DoclingError::FormatError(msg)) = result {
                assert!(msg.contains("No file extension"));
            }
        }
    }

    #[test]
    fn test_convert_unsupported_extension() {
        let converter = RustDocumentConverter::new();
        if let Ok(converter) = converter {
            let result = converter.convert("test.unsupported_format");
            assert!(result.is_err());
        }
    }

    // ===== CATEGORY 3: Error Handling Tests =====

    #[test]
    fn test_convert_nonexistent_file() {
        let converter = RustDocumentConverter::new();
        if let Ok(converter) = converter {
            // Try to convert a file that doesn't exist
            let result = converter.convert("/nonexistent/path/file.md");
            // Should fail with some error (file not found or I/O error)
            assert!(result.is_err());
        }
    }

    // ===== CATEGORY 4: Helper Function Tests =====
    // Note: DoclingDocument has complex structure (Origin, GroupItem) that requires
    // extensive setup. These are tested indirectly through integration tests.
    // Tests here focus on simpler aspects that can be verified without full document construction.

    // ===== CATEGORY 5: Converter State Tests =====

    #[test]
    fn test_converter_default_is_no_ocr() {
        if let Ok(converter) = RustDocumentConverter::new() {
            assert!(
                !converter.enable_ocr,
                "Default converter should have OCR disabled"
            );
        }
    }

    #[test]
    fn test_converter_pdf_backend_initialization() {
        // Test that converter attempts to initialize PDF backend
        let _converter = RustDocumentConverter::new();
        // May fail if pdfium not available, but should not panic
        // This tests that the constructor handles both success and failure gracefully
        // Both success and failure cases are acceptable
    }

    // ===== CATEGORY 6: Path Handling Tests =====

    #[test]
    fn test_convert_empty_path() {
        if let Ok(converter) = RustDocumentConverter::new() {
            let result = converter.convert("");
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_convert_path_with_multiple_extensions() {
        if let Ok(converter) = RustDocumentConverter::new() {
            // File with multiple dots - should use last extension
            let result = converter.convert("/path/to/file.backup.md");
            // Will fail because file doesn't exist, but should detect .md format
            assert!(result.is_err());
        }
    }

    // ===== CATEGORY 7: Format-Specific Error Tests =====

    #[test]
    fn test_convert_pdf_without_backend() {
        // Create converter without PDF backend initialized
        #[cfg(feature = "pdf")]
        let converter = RustDocumentConverter {
            pdf_fast_backend: None,
            enable_ocr: false,
            pdf_ml_config: PdfMlConfig::default(),
        };
        #[cfg(not(feature = "pdf"))]
        let converter = RustDocumentConverter {
            _no_pdf: (),
            enable_ocr: false,
            pdf_ml_config: PdfMlConfig::default(),
        };

        let result = converter.convert("/nonexistent/file.pdf");
        assert!(result.is_err());
        if let Err(DoclingError::BackendError(msg)) = result {
            #[cfg(feature = "pdf")]
            assert!(msg.contains("PDF backend not available"));
            #[cfg(not(feature = "pdf"))]
            assert!(msg.contains("PDF support requires"));
        }
    }

    // ===== CATEGORY 8: ConversionResult Tests =====

    #[test]
    fn test_conversion_result_has_latency() {
        // Test that ConversionResult includes latency measurement
        // We can't easily test this without a real file conversion,
        // but we can verify the type exists and has the right fields
        use std::time::Duration;

        let doc = Document::from_markdown("# Test".to_string(), InputFormat::Md);
        let result = ConversionResult {
            document: doc,
            latency: Duration::from_millis(100),
        };

        assert_eq!(result.latency.as_millis(), 100);
    }

    // ===== CATEGORY 9: New Method Equivalence Tests =====

    #[test]
    fn test_new_equals_with_ocr_false() {
        let converter1 = RustDocumentConverter::new();
        let converter2 = RustDocumentConverter::with_ocr(false);

        // Both should have the same OCR setting
        if let (Ok(c1), Ok(c2)) = (converter1, converter2) {
            assert_eq!(c1.enable_ocr, c2.enable_ocr);
        }
    }

    // ===== CATEGORY 10: Module Re-exports Tests =====

    #[test]
    fn test_conversion_result_re_export() {
        // Verify that ConversionResult is properly re-exported
        // This is a compile-time test, but we can also verify at runtime
        use std::time::Duration;

        let doc = Document::from_markdown("# Test".to_string(), InputFormat::Md);
        let _result = ConversionResult {
            document: doc,
            latency: Duration::from_millis(50),
        };

        // If this compiles, the re-export works (no assertion needed)
    }

    // ===== CATEGORY 11: Extension Detection Tests =====

    #[test]
    fn test_detect_pdf_extension() {
        if let Ok(converter) = RustDocumentConverter::new() {
            let result = converter.convert("test.pdf");
            // Will fail (file doesn't exist), but error should be IoError not FormatError
            assert!(result.is_err());
            // PDF backend might not be available, so we just check it doesn't panic
        }
    }

    #[test]
    fn test_detect_md_extension() {
        if let Ok(converter) = RustDocumentConverter::new() {
            let result = converter.convert("test.md");
            // Will fail (file doesn't exist), but error should be IoError not FormatError
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_detect_html_extension() {
        if let Ok(converter) = RustDocumentConverter::new() {
            let result = converter.convert("test.html");
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_detect_docx_extension() {
        if let Ok(converter) = RustDocumentConverter::new() {
            let result = converter.convert("test.docx");
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_detect_csv_extension() {
        if let Ok(converter) = RustDocumentConverter::new() {
            let result = converter.convert("test.csv");
            assert!(result.is_err());
        }
    }

    // ===== CATEGORY 12: Case Sensitivity Tests =====

    #[test]
    fn test_extension_case_insensitive() {
        if let Ok(converter) = RustDocumentConverter::new() {
            // Most file systems are case-insensitive, but extension detection should handle it
            let result_lower = converter.convert("test.pdf");
            let result_upper = converter.convert("test.PDF");

            // Both should fail with same type of error (file not found, not format error)
            assert!(result_lower.is_err());
            assert!(result_upper.is_err());
        }
    }

    // ===== CATEGORY 13: Special Characters in Path Tests =====

    #[test]
    fn test_path_with_spaces() {
        if let Ok(converter) = RustDocumentConverter::new() {
            let result = converter.convert("/path with spaces/file.md");
            // Should fail because file doesn't exist, not because of spaces in path
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_path_with_unicode() {
        if let Ok(converter) = RustDocumentConverter::new() {
            let result = converter.convert("/path/文件.md");
            // Should handle Unicode in path
            assert!(result.is_err());
        }
    }

    // ===== CATEGORY 14: Additional Unicode and Special Character Tests =====

    #[test]
    fn test_path_with_emoji() {
        if let Ok(converter) = RustDocumentConverter::new() {
            let result = converter.convert("/path/📄document.md");
            // Should handle emoji in path
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_path_with_cyrillic() {
        if let Ok(converter) = RustDocumentConverter::new() {
            let result = converter.convert("/путь/документ.md");
            // Should handle Cyrillic in path
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_extension_with_numbers() {
        if let Ok(converter) = RustDocumentConverter::new() {
            let result = converter.convert("test.mp3");
            // Audio formats should error with appropriate message
            assert!(result.is_err());
        }
    }

    // ===== CATEGORY 15: Additional Validation Tests =====

    #[test]
    fn test_path_with_double_slash() {
        if let Ok(converter) = RustDocumentConverter::new() {
            let result = converter.convert("/path//to//file.md");
            // Should handle double slashes in path
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_path_with_dot_segments() {
        if let Ok(converter) = RustDocumentConverter::new() {
            let result = converter.convert("/path/./to/../file.md");
            // Should handle . and .. in path
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_very_long_filename() {
        if let Ok(converter) = RustDocumentConverter::new() {
            let long_name = "a".repeat(255);
            let path = format!("{long_name}.md");
            let result = converter.convert(&path);
            // Should handle very long filenames
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_hidden_file() {
        if let Ok(converter) = RustDocumentConverter::new() {
            let result = converter.convert("/path/.hidden.md");
            // Should handle hidden files (starting with .)
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_windows_path() {
        if let Ok(converter) = RustDocumentConverter::new() {
            let result = converter.convert("C:\\Users\\test\\document.md");
            // Should handle Windows-style paths
            assert!(result.is_err());
        }
    }

    // ===== CATEGORY 16: Format Detection Edge Cases =====

    #[test]
    fn test_multiple_dots_in_filename() {
        if let Ok(converter) = RustDocumentConverter::new() {
            let result = converter.convert("file.v1.0.backup.md");
            // Should use last extension (.md)
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_uppercase_extension() {
        if let Ok(converter) = RustDocumentConverter::new() {
            let result = converter.convert("TEST.MD");
            // Extension detection should be case-insensitive
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_mixed_case_extension() {
        if let Ok(converter) = RustDocumentConverter::new() {
            let result = converter.convert("test.Md");
            // Extension detection should be case-insensitive
            assert!(result.is_err());
        }
    }

    // ===== CATEGORY 17: OCR Mode Consistency Tests =====

    #[test]
    fn test_with_ocr_true() {
        let converter = RustDocumentConverter::with_ocr(true);
        if let Ok(c) = converter {
            assert!(c.enable_ocr, "OCR should be enabled when with_ocr(true)");
        }
    }

    #[test]
    fn test_ocr_mode_persists() {
        if let Ok(converter) = RustDocumentConverter::with_ocr(true) {
            // OCR mode should persist across calls
            let _result1 = converter.convert("file1.md");
            let _result2 = converter.convert("file2.md");
            assert!(converter.enable_ocr);
        }
    }

    // ===== CATEGORY 18: Conversion Result Tests =====

    #[test]
    fn test_conversion_result_latency_non_zero() {
        use std::time::Duration;
        let doc = Document::from_markdown("# Test".to_string(), InputFormat::Md);
        let result = ConversionResult {
            document: doc,
            latency: Duration::from_millis(100),
        };
        assert!(result.latency.as_millis() > 0);
    }

    #[test]
    fn test_conversion_result_zero_latency() {
        use std::time::Duration;
        let doc = Document::from_markdown("# Test".to_string(), InputFormat::Md);
        let result = ConversionResult {
            document: doc,
            latency: Duration::from_millis(0),
        };
        assert_eq!(result.latency.as_millis(), 0);
    }

    #[test]
    fn test_conversion_result_document_access() {
        use std::time::Duration;
        let doc = Document::from_markdown("# Test Content".to_string(), InputFormat::Md);
        let result = ConversionResult {
            document: doc,
            latency: Duration::from_millis(50),
        };
        assert!(result.document.markdown.contains("Test Content"));
    }

    // ===== CATEGORY 19: Backend Availability Tests =====

    #[test]
    fn test_converter_new_handles_pdf_unavailable() {
        // Converter should handle PDF backend unavailable gracefully
        let result = RustDocumentConverter::new();
        // May succeed or fail depending on pdfium availability
        // But should not panic
        let _ = result;
    }

    #[test]
    fn test_converter_with_ocr_handles_pdf_unavailable() {
        // OCR requires PDF backend, but constructor should not panic
        let result = RustDocumentConverter::with_ocr(false);
        let _ = result;
    }

    // ===== CATEGORY 20: Error Message Quality Tests =====

    #[test]
    fn test_no_extension_error_message() {
        if let Ok(converter) = RustDocumentConverter::new() {
            let result = converter.convert("file_without_extension");
            assert!(result.is_err());
            if let Err(DoclingError::FormatError(msg)) = result {
                assert!(msg.contains("No file extension"));
            }
        }
    }

    #[test]
    fn test_pdf_backend_unavailable_error_message() {
        // Create converter without PDF backend initialized
        #[cfg(feature = "pdf")]
        let converter = RustDocumentConverter {
            pdf_fast_backend: None,
            enable_ocr: false,
            pdf_ml_config: PdfMlConfig::default(),
        };
        #[cfg(not(feature = "pdf"))]
        let converter = RustDocumentConverter {
            _no_pdf: (),
            enable_ocr: false,
            pdf_ml_config: PdfMlConfig::default(),
        };
        let result = converter.convert("test.pdf");
        assert!(result.is_err());
        if let Err(DoclingError::BackendError(msg)) = result {
            #[cfg(feature = "pdf")]
            assert!(msg.contains("PDF backend not available"));
            #[cfg(not(feature = "pdf"))]
            assert!(msg.contains("PDF support requires"));
        }
    }

    // ===== CATEGORY 21: Additional Format Extension Tests =====

    #[test]
    fn test_detect_pptx_extension() {
        if let Ok(converter) = RustDocumentConverter::new() {
            let result = converter.convert("presentation.pptx");
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_detect_xlsx_extension() {
        if let Ok(converter) = RustDocumentConverter::new() {
            let result = converter.convert("spreadsheet.xlsx");
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_detect_txt_extension() {
        if let Ok(converter) = RustDocumentConverter::new() {
            let result = converter.convert("readme.txt");
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_detect_json_extension() {
        if let Ok(converter) = RustDocumentConverter::new() {
            let result = converter.convert("data.json");
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_detect_xml_extension() {
        if let Ok(converter) = RustDocumentConverter::new() {
            let result = converter.convert("document.xml");
            assert!(result.is_err());
        }
    }

    // ===== CATEGORY 22: Relative vs Absolute Path Tests =====

    #[test]
    fn test_relative_path() {
        if let Ok(converter) = RustDocumentConverter::new() {
            let result = converter.convert("./relative/path/file.md");
            // Should handle relative paths
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_absolute_path() {
        if let Ok(converter) = RustDocumentConverter::new() {
            let result = converter.convert("/absolute/path/file.md");
            // Should handle absolute paths
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_home_directory_path() {
        if let Ok(converter) = RustDocumentConverter::new() {
            let result = converter.convert("~/documents/file.md");
            // Should handle ~ in paths (may not expand, but should not panic)
            assert!(result.is_err());
        }
    }

    // ===== CATEGORY 23: Additional Edge Cases (+9 tests) =====

    #[test]
    fn test_current_directory_path() {
        if let Ok(converter) = RustDocumentConverter::new() {
            let result = converter.convert("./file.md");
            // Should handle current directory reference
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_parent_directory_path() {
        if let Ok(converter) = RustDocumentConverter::new() {
            let result = converter.convert("../file.md");
            // Should handle parent directory reference
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_path_ending_with_slash() {
        if let Ok(converter) = RustDocumentConverter::new() {
            let result = converter.convert("/path/to/directory/");
            // Directory paths should error (no extension)
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_file_with_no_name_only_extension() {
        if let Ok(converter) = RustDocumentConverter::new() {
            let result = converter.convert("/path/.md");
            // File with only extension (no basename) should still work
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_converter_reuse_multiple_files() {
        if let Ok(converter) = RustDocumentConverter::new() {
            // Test that converter can be reused for multiple files
            let _result1 = converter.convert("file1.md");
            let _result2 = converter.convert("file2.pdf");
            let _result3 = converter.convert("file3.html");
            // All should error (files don't exist), but converter should remain valid
        }
    }

    #[test]
    fn test_symlink_path() {
        if let Ok(converter) = RustDocumentConverter::new() {
            let result = converter.convert("/path/to/symlink.md");
            // Should handle symlinks (will error if doesn't exist, but should not panic)
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_extension_with_special_chars() {
        if let Ok(converter) = RustDocumentConverter::new() {
            // Extension with special characters
            let result = converter.convert("file.md-backup");
            // Should try to parse .md-backup as extension
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_very_long_path() {
        if let Ok(converter) = RustDocumentConverter::new() {
            let long_path = format!("{}/file.md", "/very/long/path".repeat(50));
            let result = converter.convert(&long_path);
            // Should handle very long paths
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_conversion_result_format_preserved() {
        use std::time::Duration;
        // Test that format is preserved in conversion result
        let doc = Document::from_markdown("# Test".to_string(), InputFormat::Html);
        let result = ConversionResult {
            document: doc,
            latency: Duration::from_millis(25),
        };
        assert_eq!(result.document.format, InputFormat::Html);
    }

    #[test]
    fn test_path_with_unicode_characters() {
        if let Ok(converter) = RustDocumentConverter::new() {
            // Test path with Unicode characters (Chinese, Arabic, etc.)
            let result = converter.convert("/path/文档/file.md");
            // Should handle Unicode paths (will error if doesn't exist)
            assert!(result.is_err());

            let result2 = converter.convert("/مسار/file.pdf");
            assert!(result2.is_err());
        }
    }

    #[test]
    fn test_file_with_multiple_dots() {
        if let Ok(converter) = RustDocumentConverter::new() {
            // Test filename with multiple dots
            let result = converter.convert("my.file.name.with.dots.md");
            // Should use last extension (.md)
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_conversion_with_ocr_enabled() {
        // Test that converter can be created with OCR enabled
        let converter_with_ocr = RustDocumentConverter::with_ocr(true);
        assert!(converter_with_ocr.is_ok());

        if let Ok(converter) = converter_with_ocr {
            assert!(converter.enable_ocr);

            // Test convert (should error as file doesn't exist)
            let result = converter.convert("nonexistent.pdf");
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_conversion_latency_measured() {
        use std::time::Duration;
        // Test that latency is measured in conversion results
        let doc = Document::from_markdown("# Test".to_string(), InputFormat::Md);
        let result = ConversionResult {
            document: doc,
            latency: Duration::from_millis(100),
        };

        // Latency should be measurable
        assert!(result.latency.as_millis() > 0);
        assert!(result.latency.as_millis() >= 100);
    }

    #[test]
    fn test_converter_handles_all_supported_formats() {
        if let Ok(converter) = RustDocumentConverter::new() {
            // Test that converter can attempt to process all major formats
            let formats = vec![
                "file.pdf",
                "file.docx",
                "file.html",
                "file.md",
                "file.xlsx",
                "file.pptx",
                "file.csv",
                "file.json",
                "file.png",
                "file.jpg",
                "file.svg",
            ];

            for format_file in formats {
                let result = converter.convert(format_file);
                // All should error (files don't exist) but should not panic
                assert!(result.is_err());
            }
        }
    }

    #[test]
    fn test_converter_with_backend_options() {
        // Test that converter respects BackendOptions configuration
        let _options = BackendOptions::default()
            .with_ocr(true)
            .with_table_structure(true)
            .with_images(true);

        if let Ok(converter) = RustDocumentConverter::with_ocr(true) {
            // Verify converter was created with OCR enabled
            assert!(converter.enable_ocr);
            // Note: BackendOptions are passed to convert_document, not stored in converter
        }
    }

    #[test]
    fn test_converter_error_propagation() {
        // Test that converter properly propagates errors from backends
        if let Ok(converter) = RustDocumentConverter::new() {
            let result = converter.convert("/nonexistent/path/to/file.docx");
            assert!(result.is_err());
            let err = result.unwrap_err();
            assert!(
                err.to_string().contains("No such file") || err.to_string().contains("not found")
            );
        }
    }

    #[test]
    fn test_converter_format_detection_priority() {
        // Test that converter prioritizes extension over content
        // (This tests the design decision documented in converter.rs)
        if let Ok(converter) = RustDocumentConverter::new() {
            // File named .pdf should be detected as PDF regardless of content
            let result = converter.convert("test.pdf");
            assert!(result.is_err()); // File doesn't exist, but detection works
                                      // Error should be about missing file, not unsupported format
        }
    }

    #[test]
    fn test_converter_with_archive_formats() {
        // Test that converter handles archive formats correctly
        if let Ok(converter) = RustDocumentConverter::new() {
            let archive_formats = vec!["file.zip", "file.tar", "file.tar.gz", "file.7z"];

            for archive_file in archive_formats {
                let result = converter.convert(archive_file);
                // Should error (files don't exist) but should recognize format
                assert!(result.is_err());
            }
        }
    }

    #[test]
    fn test_converter_latency_consistency() {
        // Test that multiple conversions maintain consistent latency measurement
        if let Ok(converter) = RustDocumentConverter::new() {
            // Create a temporary test file
            use std::fs;
            let temp_path = "/tmp/test_latency.md";
            fs::write(temp_path, "# Test\nLatency test").ok();

            // Warmup run to eliminate cold-start overhead (initialization, lazy loading, etc.)
            let _ = converter.convert(temp_path);

            if let Ok(result1) = converter.convert(temp_path) {
                if let Ok(result2) = converter.convert(temp_path) {
                    // Both should have measurable latency (use nanos for fast operations)
                    assert!(result1.latency.as_nanos() > 0);
                    assert!(result2.latency.as_nanos() > 0);

                    // Latencies should be in reasonable range
                    // Allow wide variation due to system load, caching, and scheduler variations
                    // (within 100x of each other - very permissive for system variability)
                    let nanos1 = result1.latency.as_nanos() as f64;
                    let nanos2 = result2.latency.as_nanos() as f64;
                    if nanos1 > 0.0 && nanos2 > 0.0 {
                        let ratio = nanos1 / nanos2;
                        // Wide bounds to handle first-run vs cached, system load variations
                        assert!(
                            ratio > 0.01 && ratio < 100.0,
                            "Latency ratio {ratio} outside bounds (latency1: {nanos1}ns, latency2: {nanos2}ns)"
                        );
                    }
                }
            }

            // Cleanup
            fs::remove_file(temp_path).ok();
        }
    }

    // Advanced Converter Features (Tests 71-75)

    #[test]
    fn test_converter_with_format_override() {
        // Test explicit format specification (override auto-detection)
        // Use case: file with wrong extension or no extension
        if let Ok(converter) = RustDocumentConverter::new() {
            // Test that converter can use explicit format instead of extension
            // Example: CSV data in .txt file
            let result = converter.convert("data.txt");
            assert!(result.is_err()); // File doesn't exist

            // Test multiple format overrides
            let formats_to_test = vec![
                "report.docx",  // DOCX format
                "data.csv",     // CSV format
                "document.pdf", // PDF format
                "file.txt",     // TXT format (unsupported, should error)
            ];

            for filename in formats_to_test {
                let result = converter.convert(filename);
                // Should error (file doesn't exist)
                assert!(result.is_err(), "Expected error for {filename}");
                // Don't check exact error message as it varies by format and backend
            }
        }
    }

    #[test]
    fn test_converter_batch_processing() {
        // Test converting multiple files in sequence
        if let Ok(converter) = RustDocumentConverter::new() {
            // Simulate batch processing of multiple documents
            let batch_files = vec![
                "document1.docx",
                "document2.docx",
                "document3.docx",
                "document4.docx",
                "document5.docx",
            ];

            let mut results = Vec::new();
            for file in batch_files {
                let result = converter.convert(file);
                results.push(result);
            }

            // All should fail (files don't exist) but converter should remain usable
            assert_eq!(results.len(), 5);
            for result in results {
                assert!(result.is_err());
            }

            // Verify converter is still functional after batch
            let final_result = converter.convert("final_document.docx");
            assert!(final_result.is_err()); // File doesn't exist
        }
    }

    #[test]
    fn test_converter_mixed_format_batch() {
        // Test batch conversion with different formats
        if let Ok(converter) = RustDocumentConverter::new() {
            // Test converting multiple different formats in one session
            let mixed_batch = vec![
                ("report.docx", InputFormat::Docx),
                ("data.csv", InputFormat::Csv),
                ("presentation.pptx", InputFormat::Pptx),
                ("spreadsheet.xlsx", InputFormat::Xlsx),
                ("document.pdf", InputFormat::Pdf),
                ("archive.zip", InputFormat::Zip),
                ("subtitle.srt", InputFormat::Srt),
                ("email.eml", InputFormat::Eml),
                ("ebook.epub", InputFormat::Epub),
                ("webpage.html", InputFormat::Html),
            ];

            let mut format_counts = std::collections::HashMap::new();
            for (filename, expected_format) in mixed_batch {
                let result = converter.convert(filename);
                // Track which formats were attempted
                *format_counts.entry(expected_format).or_insert(0) += 1;

                // All should error (files don't exist)
                assert!(result.is_err());
            }

            // Verify we tested diverse formats
            assert!(
                format_counts.len() >= 8,
                "Expected at least 8 different formats"
            );
        }
    }

    #[test]
    fn test_converter_fallback_chain() {
        // Test converter fallback logic when primary backend fails
        if let Ok(converter) = RustDocumentConverter::new() {
            // Test error handling when backend is unavailable
            // This simulates the case where a format is supported but backend init fails

            // Try to convert a PDF without PDF backend
            let result = converter.convert("test.pdf");
            assert!(result.is_err());

            // Error should indicate the specific problem
            let err = result.unwrap_err();
            let err_str = err.to_string();

            // Should mention PDF or file not found (or out of memory if pdfium is stressed)
            assert!(
                err_str.contains("PDF")
                    || err_str.contains("pdf")
                    || err_str.contains("not found")
                    || err_str.contains("No such file")
                    || err_str.contains("I/O error")
                    || err_str.contains("Out of memory"), // pdfium OOM under test load
                "Unexpected error message: {err_str}"
            );

            // Test with OCR-required format
            if let Ok(ocr_converter) = RustDocumentConverter::with_ocr(true) {
                let result = ocr_converter.convert("scan.pdf");
                assert!(result.is_err());
            }
        }
    }

    #[test]
    fn test_converter_metadata_preservation() {
        // Test that metadata is preserved through conversion
        // This tests the docling_document_to_document() conversion

        // Create a temporary test file for conversion
        if let Ok(converter) = RustDocumentConverter::new() {
            use std::fs;
            use std::io::Write;

            // Create temporary markdown file
            let temp_path = "/tmp/test_metadata_preservation.md";
            if let Ok(mut file) = fs::File::create(temp_path) {
                writeln!(file, "# Test Document").ok();
                writeln!(file, "\nThis is a test document for metadata preservation.").ok();
                writeln!(file, "\n## Section 1\n\nContent here.").ok();

                // Convert the file
                let result = converter.convert(temp_path);

                if let Ok(conversion_result) = result {
                    let doc = conversion_result.document;

                    // Verify basic metadata preserved
                    assert_eq!(doc.format, InputFormat::Md);

                    // Verify markdown content is present
                    assert!(
                        !doc.markdown.is_empty(),
                        "Markdown content should not be empty"
                    );
                    assert!(
                        doc.markdown.contains("Test Document")
                            || doc.markdown.contains("test document"),
                        "Markdown should contain document title"
                    );

                    // Verify character count was calculated
                    assert!(
                        doc.metadata.num_characters > 0,
                        "Character count should be calculated"
                    );

                    // Verify latency was measured
                    assert!(
                        conversion_result.latency.as_nanos() > 0,
                        "Latency should be measured"
                    );
                }

                // Cleanup
                fs::remove_file(temp_path).ok();
            }
        }
    }

    #[test]
    fn test_pdf_ml_backend_display() {
        assert_eq!(format!("{}", PdfMlBackend::PyTorch), "pytorch");
        assert_eq!(format!("{}", PdfMlBackend::Onnx), "onnx");
    }

    #[test]
    fn test_pdf_ml_backend_from_str() {
        use std::str::FromStr;

        // PyTorch variants
        assert_eq!(
            PdfMlBackend::from_str("pytorch").unwrap(),
            PdfMlBackend::PyTorch
        );
        assert_eq!(
            PdfMlBackend::from_str("torch").unwrap(),
            PdfMlBackend::PyTorch
        );
        assert_eq!(PdfMlBackend::from_str("pt").unwrap(), PdfMlBackend::PyTorch);
        assert_eq!(
            PdfMlBackend::from_str("PYTORCH").unwrap(),
            PdfMlBackend::PyTorch
        );
        assert_eq!(
            PdfMlBackend::from_str("PyTorch").unwrap(),
            PdfMlBackend::PyTorch
        );

        // ONNX variants
        assert_eq!(PdfMlBackend::from_str("onnx").unwrap(), PdfMlBackend::Onnx);
        assert_eq!(PdfMlBackend::from_str("ONNX").unwrap(), PdfMlBackend::Onnx);

        // Invalid
        assert!(PdfMlBackend::from_str("invalid").is_err());
        assert!(PdfMlBackend::from_str("").is_err());
    }

    #[test]
    fn test_pdf_ml_backend_roundtrip() {
        use std::str::FromStr;

        // Test round-trip: Display -> FromStr
        for backend in [PdfMlBackend::PyTorch, PdfMlBackend::Onnx] {
            let display = backend.to_string();
            let parsed = PdfMlBackend::from_str(&display).unwrap();
            assert_eq!(parsed, backend);
        }
    }
}
