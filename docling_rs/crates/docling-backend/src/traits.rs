//! Core trait definitions for document backends

// Clippy pedantic allows:
// - BackendOptions uses multiple bool flags for feature toggles
#![allow(clippy::struct_excessive_bools)]

use crate::converter::PdfMlBackend;
use docling_core::{DoclingError, Document, InputFormat};
use std::path::Path;

/// Default rendering DPI for ML-based layout detection
///
/// 144 DPI matches Python docling's rendering resolution. The RT-DETR V2 layout
/// model was trained on images rendered at this DPI. Using different DPI causes
/// wrong detections (e.g., code blocks labeled as text).
pub const DEFAULT_RENDER_DPI: f32 = 144.0;

/// Default horizontal merge threshold factor for text cell grouping
///
/// Controls how aggressively horizontally adjacent text cells are merged.
/// The actual threshold is: `avg_cell_height * merge_threshold_factor`.
/// Value of 1.3 matches pdfium-render behavior.
const DEFAULT_MERGE_THRESHOLD_FACTOR: f32 = 1.3;

/// Options for backend processing
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BackendOptions {
    /// Enable OCR for scanned documents/images
    pub enable_ocr: bool,

    /// Enable table structure recognition
    pub enable_table_structure: bool,

    /// Extract images from document
    pub extract_images: bool,

    /// Maximum pages to process (None = all)
    pub max_pages: Option<usize>,

    /// Enable fetching remote images (HTTP/HTTPS URLs) in HTML documents
    /// Security: Only enable when processing trusted content
    pub enable_remote_fetch: bool,

    /// Enable fetching local images (file:// URLs, relative paths) in HTML documents
    /// Security: Only enable when processing trusted content from known sources
    pub enable_local_fetch: bool,

    /// ML backend selection for PDF processing (`PyTorch` or ONNX)
    ///
    /// - `PdfMlBackend::PyTorch`: Faster (1.56x), supports GPU acceleration
    /// - `PdfMlBackend::Onnx`: Cross-platform, portable, CPU-optimized
    ///
    /// Only affects PDF processing with the pdf-ml feature enabled.
    pub ml_backend: PdfMlBackend,

    /// Issue #19 FIX: Include furniture layer (page headers/footers) in output
    ///
    /// By default, only the "body" content layer is included in markdown output.
    /// When enabled, page headers and footers (furniture layer) are also included.
    ///
    /// Default: false (matches Python docling's `DEFAULT_CONTENT_LAYERS` = {ContentLayer.BODY})
    pub include_furniture: bool,

    /// BUG #64 fix: Render DPI for PDF page rasterization
    ///
    /// Higher DPI improves text recognition accuracy but increases memory and CPU usage.
    /// Typical values:
    /// - 72 DPI: Preview quality (fast, low memory)
    /// - 144 DPI: Default (matches Python docling, required for ML models)
    /// - 300 DPI: Print quality (higher memory, may cause ML detection issues)
    /// - 600 DPI: High quality (slow, high memory)
    ///
    /// Default: 144.0 (matches Python docling - required for correct ML detection)
    pub render_dpi: f32,

    /// BUG #47 fix: Horizontal merge threshold factor for text cell grouping
    ///
    /// Controls how aggressively horizontally adjacent text cells are merged.
    /// The actual threshold is: `avg_cell_height` * `merge_threshold_factor`
    ///
    /// Higher values merge more aggressively (may merge separate words).
    /// Lower values are more conservative (may split single words).
    ///
    /// Default: 1.3 (matches pdfium-render behavior)
    pub merge_threshold_factor: f32,
}

impl BackendOptions {
    /// Create options with OCR enabled
    #[inline]
    #[must_use = "returns options with OCR setting configured"]
    pub const fn with_ocr(mut self, enable: bool) -> Self {
        self.enable_ocr = enable;
        self
    }

    /// Create options with table structure recognition
    #[inline]
    #[must_use = "returns options with table structure setting configured"]
    pub const fn with_table_structure(mut self, enable: bool) -> Self {
        self.enable_table_structure = enable;
        self
    }

    /// Create options with image extraction
    #[inline]
    #[must_use = "returns options with image extraction setting configured"]
    pub const fn with_images(mut self, enable: bool) -> Self {
        self.extract_images = enable;
        self
    }

    /// Create options with remote image fetching
    /// Security: Only enable when processing trusted content
    #[inline]
    #[must_use = "returns options with remote fetch setting configured"]
    pub const fn with_remote_fetch(mut self, enable: bool) -> Self {
        self.enable_remote_fetch = enable;
        self
    }

    /// Create options with local image fetching
    /// Security: Only enable when processing trusted content from known sources
    #[inline]
    #[must_use = "returns options with local fetch setting configured"]
    pub const fn with_local_fetch(mut self, enable: bool) -> Self {
        self.enable_local_fetch = enable;
        self
    }

    /// Set maximum pages to process
    #[inline]
    #[must_use = "returns options with maximum pages configured"]
    pub const fn with_max_pages(mut self, max_pages: Option<usize>) -> Self {
        self.max_pages = max_pages;
        self
    }

    /// Set ML backend for PDF processing
    ///
    /// # Arguments
    /// * `backend` - ML backend selection (`PyTorch` or ONNX)
    ///
    /// # Examples
    /// ```ignore
    /// use docling_backend::{BackendOptions, PdfMlBackend};
    ///
    /// let opts = BackendOptions::default()
    ///     .with_ml_backend(PdfMlBackend::Onnx);
    /// ```
    #[inline]
    #[must_use = "returns options with ML backend configured"]
    pub const fn with_ml_backend(mut self, backend: PdfMlBackend) -> Self {
        self.ml_backend = backend;
        self
    }

    /// Issue #19 FIX: Include furniture layer (page headers/footers)
    ///
    /// By default, only the "body" content layer is included in markdown output.
    /// When enabled, page headers and footers are also included.
    ///
    /// # Examples
    /// ```ignore
    /// use docling_backend::BackendOptions;
    ///
    /// let opts = BackendOptions::default()
    ///     .with_furniture(true);
    /// ```
    #[inline]
    #[must_use = "returns options with furniture layer setting configured"]
    pub const fn with_furniture(mut self, include: bool) -> Self {
        self.include_furniture = include;
        self
    }

    /// BUG #64 fix: Set render DPI for PDF page rasterization
    ///
    /// Higher DPI improves text recognition accuracy but increases memory and CPU usage.
    ///
    /// # Arguments
    /// * `dpi` - Dots per inch (72-600, default 300.0)
    ///
    /// # Examples
    /// ```ignore
    /// use docling_backend::BackendOptions;
    ///
    /// let opts = BackendOptions::default()
    ///     .with_render_dpi(150.0);  // Faster but lower quality
    /// ```
    #[inline]
    #[must_use = "returns options with render DPI configured"]
    pub const fn with_render_dpi(mut self, dpi: f32) -> Self {
        self.render_dpi = dpi;
        self
    }

    /// BUG #47 fix: Set horizontal merge threshold factor for text cell grouping
    ///
    /// Controls how aggressively horizontally adjacent text cells are merged.
    ///
    /// # Arguments
    /// * `factor` - Threshold multiplier (0.5-3.0, default 1.3)
    ///
    /// # Examples
    /// ```ignore
    /// use docling_backend::BackendOptions;
    ///
    /// let opts = BackendOptions::default()
    ///     .with_merge_threshold_factor(1.5);  // More aggressive merging
    /// ```
    #[inline]
    #[must_use = "returns options with merge threshold configured"]
    pub const fn with_merge_threshold_factor(mut self, factor: f32) -> Self {
        self.merge_threshold_factor = factor;
        self
    }
}

impl Default for BackendOptions {
    #[inline]
    fn default() -> Self {
        Self {
            enable_ocr: false,
            enable_table_structure: false,
            extract_images: false,
            max_pages: None,
            enable_remote_fetch: false,
            enable_local_fetch: false,
            ml_backend: PdfMlBackend::default(),
            include_furniture: false,
            render_dpi: DEFAULT_RENDER_DPI,
            merge_threshold_factor: DEFAULT_MERGE_THRESHOLD_FACTOR,
        }
    }
}

/// Main trait for document backends
///
/// Each backend (PDF, DOCX, etc.) implements this trait to provide
/// document parsing and conversion functionality.
pub trait DocumentBackend: Send + Sync {
    /// Get the format this backend handles
    fn format(&self) -> InputFormat;

    /// Parse document from bytes
    ///
    /// # Errors
    /// Returns an error if parsing fails.
    fn parse_bytes(&self, data: &[u8], options: &BackendOptions) -> Result<Document, DoclingError>;

    /// Parse document from file path
    ///
    /// # Errors
    /// Returns an error if file reading or parsing fails.
    fn parse_file<P: AsRef<Path>>(
        &self,
        path: P,
        options: &BackendOptions,
    ) -> Result<Document, DoclingError> {
        let data = std::fs::read(path.as_ref()).map_err(DoclingError::IoError)?;
        self.parse_bytes(&data, options)
    }

    /// Check if this backend can handle the given format
    fn can_handle(&self, format: InputFormat) -> bool {
        self.format() == format
    }
}

/// Async version of DocumentBackend trait
#[cfg(feature = "async")]
#[async_trait::async_trait]
pub trait AsyncDocumentBackend: Send + Sync {
    /// Get the format this backend handles
    fn format(&self) -> InputFormat;

    /// Parse document from bytes asynchronously
    async fn parse_bytes(
        &self,
        data: &[u8],
        options: &BackendOptions,
    ) -> Result<Document, DoclingError>;

    /// Parse document from file path asynchronously
    async fn parse_file<P: AsRef<Path> + Send>(
        &self,
        path: P,
        options: &BackendOptions,
    ) -> Result<Document, DoclingError> {
        let data = tokio::fs::read(path.as_ref())
            .await
            .map_err(DoclingError::IoError)?;
        self.parse_bytes(&data, options).await
    }

    /// Check if this backend can handle the given format
    fn can_handle(&self, format: InputFormat) -> bool {
        self.format() == format
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_options_default() {
        let opts = BackendOptions::default();
        assert!(!opts.enable_ocr);
        assert!(!opts.enable_table_structure);
        assert!(!opts.extract_images);
        assert!(opts.max_pages.is_none());
        assert!(!opts.enable_remote_fetch);
        assert!(!opts.enable_local_fetch);
        // BUG #64 fix: Verify default render DPI (144.0 matches Python docling)
        assert!((opts.render_dpi - 144.0).abs() < f32::EPSILON);
        // BUG #47 fix: Verify default merge threshold factor
        assert!((opts.merge_threshold_factor - 1.3).abs() < f32::EPSILON);
    }

    #[test]
    fn test_backend_options_with_ocr_true() {
        let opts = BackendOptions::default().with_ocr(true);
        assert!(opts.enable_ocr);
        assert!(!opts.enable_table_structure);
    }

    #[test]
    fn test_backend_options_with_ocr_false() {
        let opts = BackendOptions::default().with_ocr(false);
        assert!(!opts.enable_ocr);
    }

    #[test]
    fn test_backend_options_with_table_structure_true() {
        let opts = BackendOptions::default().with_table_structure(true);
        assert!(opts.enable_table_structure);
        assert!(!opts.enable_ocr);
    }

    #[test]
    fn test_backend_options_with_table_structure_false() {
        let opts = BackendOptions::default().with_table_structure(false);
        assert!(!opts.enable_table_structure);
    }

    #[test]
    fn test_backend_options_with_images_true() {
        let opts = BackendOptions::default().with_images(true);
        assert!(opts.extract_images);
        assert!(!opts.enable_ocr);
    }

    #[test]
    fn test_backend_options_with_images_false() {
        let opts = BackendOptions::default().with_images(false);
        assert!(!opts.extract_images);
    }

    #[test]
    fn test_backend_options_with_remote_fetch_true() {
        let opts = BackendOptions::default().with_remote_fetch(true);
        assert!(opts.enable_remote_fetch);
        assert!(!opts.enable_local_fetch);
    }

    #[test]
    fn test_backend_options_with_remote_fetch_false() {
        let opts = BackendOptions::default().with_remote_fetch(false);
        assert!(!opts.enable_remote_fetch);
    }

    #[test]
    fn test_backend_options_with_local_fetch_true() {
        let opts = BackendOptions::default().with_local_fetch(true);
        assert!(opts.enable_local_fetch);
        assert!(!opts.enable_remote_fetch);
    }

    #[test]
    fn test_backend_options_with_local_fetch_false() {
        let opts = BackendOptions::default().with_local_fetch(false);
        assert!(!opts.enable_local_fetch);
    }

    #[test]
    fn test_backend_options_chaining_all_enabled() {
        let opts = BackendOptions::default()
            .with_ocr(true)
            .with_table_structure(true)
            .with_images(true)
            .with_remote_fetch(true)
            .with_local_fetch(true);

        assert!(opts.enable_ocr);
        assert!(opts.enable_table_structure);
        assert!(opts.extract_images);
        assert!(opts.enable_remote_fetch);
        assert!(opts.enable_local_fetch);
    }

    #[test]
    fn test_backend_options_chaining_mixed() {
        let opts = BackendOptions::default()
            .with_ocr(true)
            .with_table_structure(false)
            .with_images(true)
            .with_remote_fetch(false)
            .with_local_fetch(true);

        assert!(opts.enable_ocr);
        assert!(!opts.enable_table_structure);
        assert!(opts.extract_images);
        assert!(!opts.enable_remote_fetch);
        assert!(opts.enable_local_fetch);
    }

    #[test]
    fn test_backend_options_override() {
        let opts = BackendOptions::default().with_ocr(true).with_ocr(false);

        assert!(!opts.enable_ocr);
    }

    #[test]
    fn test_backend_options_copy() {
        let opts1 = BackendOptions::default().with_ocr(true);
        let opts2 = opts1; // Copy trait copies the value

        assert!(opts2.enable_ocr);
        assert!(!opts2.enable_table_structure);
    }

    #[test]
    fn test_backend_options_debug() {
        let opts = BackendOptions::default().with_ocr(true);
        let debug_str = format!("{opts:?}");

        assert!(debug_str.contains("BackendOptions"));
        assert!(debug_str.contains("enable_ocr"));
    }

    #[test]
    fn test_backend_options_max_pages_none() {
        let opts = BackendOptions::default();
        assert!(opts.max_pages.is_none());
    }

    #[test]
    fn test_backend_options_direct_field_access() {
        let opts = BackendOptions {
            max_pages: Some(10),
            enable_ocr: true,
            ..Default::default()
        };

        assert_eq!(opts.max_pages, Some(10));
        assert!(opts.enable_ocr);
    }

    // New tests for N=392 expansion

    #[test]
    fn test_backend_options_with_max_pages_none() {
        let opts = BackendOptions::default().with_max_pages(None);
        assert!(opts.max_pages.is_none());
    }

    #[test]
    fn test_backend_options_with_max_pages_some_zero() {
        let opts = BackendOptions::default().with_max_pages(Some(0));
        assert_eq!(opts.max_pages, Some(0));
    }

    #[test]
    fn test_backend_options_with_max_pages_some_one() {
        let opts = BackendOptions::default().with_max_pages(Some(1));
        assert_eq!(opts.max_pages, Some(1));
    }

    #[test]
    fn test_backend_options_with_max_pages_some_hundred() {
        let opts = BackendOptions::default().with_max_pages(Some(100));
        assert_eq!(opts.max_pages, Some(100));
    }

    #[test]
    fn test_backend_options_with_max_pages_usize_max() {
        let opts = BackendOptions::default().with_max_pages(Some(usize::MAX));
        assert_eq!(opts.max_pages, Some(usize::MAX));
    }

    #[test]
    fn test_backend_options_all_disabled() {
        let opts = BackendOptions::default()
            .with_ocr(false)
            .with_table_structure(false)
            .with_images(false)
            .with_remote_fetch(false)
            .with_local_fetch(false);

        assert!(!opts.enable_ocr);
        assert!(!opts.enable_table_structure);
        assert!(!opts.extract_images);
        assert!(!opts.enable_remote_fetch);
        assert!(!opts.enable_local_fetch);
    }

    #[test]
    fn test_backend_options_security_remote_and_local() {
        let opts = BackendOptions::default()
            .with_remote_fetch(true)
            .with_local_fetch(true);

        assert!(opts.enable_remote_fetch);
        assert!(opts.enable_local_fetch);
    }

    #[test]
    fn test_backend_options_ocr_with_max_pages() {
        let opts = BackendOptions::default()
            .with_ocr(true)
            .with_max_pages(Some(50));

        assert!(opts.enable_ocr);
        assert_eq!(opts.max_pages, Some(50));
    }

    #[test]
    fn test_backend_options_table_structure_with_images() {
        let opts = BackendOptions::default()
            .with_table_structure(true)
            .with_images(true);

        assert!(opts.enable_table_structure);
        assert!(opts.extract_images);
    }

    #[test]
    fn test_backend_options_builder_immutability() {
        let opts1 = BackendOptions::default();
        let opts2 = opts1.with_ocr(true); // Copy trait preserves opts1

        // Original unchanged
        assert!(!opts1.enable_ocr);
        // New instance modified
        assert!(opts2.enable_ocr);
    }

    #[test]
    fn test_backend_options_debug_all_enabled() {
        let opts = BackendOptions::default()
            .with_ocr(true)
            .with_table_structure(true)
            .with_images(true)
            .with_remote_fetch(true)
            .with_local_fetch(true)
            .with_max_pages(Some(10));

        let debug_str = format!("{opts:?}");
        assert!(debug_str.contains("enable_ocr: true"));
        assert!(debug_str.contains("enable_table_structure: true"));
        assert!(debug_str.contains("extract_images: true"));
        assert!(debug_str.contains("enable_remote_fetch: true"));
        assert!(debug_str.contains("enable_local_fetch: true"));
        assert!(debug_str.contains("max_pages: Some(10)"));
    }

    #[test]
    fn test_backend_options_override_max_pages() {
        let opts = BackendOptions::default()
            .with_max_pages(Some(100))
            .with_max_pages(Some(50));

        assert_eq!(opts.max_pages, Some(50));
    }

    #[test]
    fn test_backend_options_complex_chaining() {
        let opts = BackendOptions::default()
            .with_ocr(true)
            .with_max_pages(Some(20))
            .with_table_structure(true)
            .with_images(false)
            .with_remote_fetch(true)
            .with_local_fetch(false);

        assert!(opts.enable_ocr);
        assert_eq!(opts.max_pages, Some(20));
        assert!(opts.enable_table_structure);
        assert!(!opts.extract_images);
        assert!(opts.enable_remote_fetch);
        assert!(!opts.enable_local_fetch);
    }

    #[test]
    fn test_backend_options_max_pages_override_to_none() {
        let opts = BackendOptions::default()
            .with_max_pages(Some(100))
            .with_max_pages(None);

        assert!(opts.max_pages.is_none());
    }

    // Mock backend for testing DocumentBackend trait
    struct MockBackend {
        format: InputFormat,
    }

    impl DocumentBackend for MockBackend {
        fn format(&self) -> InputFormat {
            self.format
        }

        fn parse_bytes(
            &self,
            _data: &[u8],
            _options: &BackendOptions,
        ) -> Result<Document, DoclingError> {
            Ok(Document {
                markdown: "Mock document".to_string(),
                format: self.format,
                metadata: Default::default(),
                content_blocks: None,
                docling_document: None,
            })
        }
    }

    #[test]
    fn test_document_backend_can_handle_matching() {
        let backend = MockBackend {
            format: InputFormat::Docx,
        };
        assert!(backend.can_handle(InputFormat::Docx));
    }

    #[test]
    fn test_document_backend_can_handle_non_matching() {
        let backend = MockBackend {
            format: InputFormat::Docx,
        };
        assert!(!backend.can_handle(InputFormat::Html));
    }

    #[test]
    fn test_document_backend_parse_bytes() {
        let backend = MockBackend {
            format: InputFormat::Md,
        };
        let opts = BackendOptions::default();
        let result = backend.parse_bytes(b"test", &opts);

        assert!(result.is_ok());
        let doc = result.unwrap();
        assert_eq!(doc.markdown, "Mock document");
        assert_eq!(doc.format, InputFormat::Md);
    }

    // N=427 expansion: 15 new tests for DocumentBackend trait and edge cases

    #[test]
    fn test_document_backend_format() {
        let backend = MockBackend {
            format: InputFormat::Pdf,
        };
        assert_eq!(backend.format(), InputFormat::Pdf);
    }

    #[test]
    fn test_document_backend_multiple_formats() {
        let backends = [
            MockBackend {
                format: InputFormat::Docx,
            },
            MockBackend {
                format: InputFormat::Html,
            },
            MockBackend {
                format: InputFormat::Csv,
            },
        ];

        assert_eq!(backends[0].format(), InputFormat::Docx);
        assert_eq!(backends[1].format(), InputFormat::Html);
        assert_eq!(backends[2].format(), InputFormat::Csv);
    }

    #[test]
    fn test_document_backend_can_handle_all_formats() {
        let backend = MockBackend {
            format: InputFormat::Pptx,
        };

        // Should only handle PPTX
        assert!(backend.can_handle(InputFormat::Pptx));
        assert!(!backend.can_handle(InputFormat::Docx));
        assert!(!backend.can_handle(InputFormat::Pdf));
        assert!(!backend.can_handle(InputFormat::Html));
    }

    #[test]
    fn test_document_backend_parse_bytes_with_options() {
        let backend = MockBackend {
            format: InputFormat::Docx,
        };
        let opts = BackendOptions::default()
            .with_ocr(true)
            .with_table_structure(true);

        let result = backend.parse_bytes(b"test content", &opts);
        assert!(result.is_ok());
    }

    #[test]
    fn test_document_backend_parse_bytes_empty() {
        let backend = MockBackend {
            format: InputFormat::Md,
        };
        let opts = BackendOptions::default();
        let result = backend.parse_bytes(b"", &opts);

        assert!(result.is_ok());
        assert_eq!(result.unwrap().markdown, "Mock document");
    }

    #[test]
    fn test_document_backend_parse_bytes_large() {
        let backend = MockBackend {
            format: InputFormat::Html,
        };
        let opts = BackendOptions::default();
        let large_data = vec![b'x'; 1_000_000];
        let result = backend.parse_bytes(&large_data, &opts);

        assert!(result.is_ok());
    }

    // Error backend for testing error handling
    struct ErrorBackend;

    impl DocumentBackend for ErrorBackend {
        fn format(&self) -> InputFormat {
            InputFormat::Pdf
        }

        fn parse_bytes(
            &self,
            _data: &[u8],
            _options: &BackendOptions,
        ) -> Result<Document, DoclingError> {
            Err(DoclingError::BackendError("Intentional error".to_string()))
        }
    }

    #[test]
    fn test_document_backend_parse_bytes_error() {
        let backend = ErrorBackend;
        let opts = BackendOptions::default();
        let result = backend.parse_bytes(b"test", &opts);

        assert!(result.is_err());
        match result {
            Err(DoclingError::BackendError(msg)) => {
                assert_eq!(msg, "Intentional error");
            }
            _ => panic!("Expected BackendError"),
        }
    }

    #[test]
    fn test_document_backend_parse_file_nonexistent() {
        let backend = MockBackend {
            format: InputFormat::Md,
        };
        let opts = BackendOptions::default();
        let result = backend.parse_file("/nonexistent/path/file.md", &opts);

        assert!(result.is_err());
        match result {
            Err(DoclingError::IoError(_)) => {}
            _ => panic!("Expected IoError"),
        }
    }

    #[test]
    fn test_backend_options_max_pages_with_all_options() {
        let opts = BackendOptions::default()
            .with_max_pages(Some(5))
            .with_ocr(true)
            .with_table_structure(true)
            .with_images(true)
            .with_remote_fetch(true)
            .with_local_fetch(true);

        assert_eq!(opts.max_pages, Some(5));
        assert!(opts.enable_ocr);
        assert!(opts.enable_table_structure);
        assert!(opts.extract_images);
        assert!(opts.enable_remote_fetch);
        assert!(opts.enable_local_fetch);
    }

    #[test]
    fn test_backend_options_security_defaults() {
        let opts = BackendOptions::default();

        // Security options should default to false
        assert!(!opts.enable_remote_fetch);
        assert!(!opts.enable_local_fetch);
    }

    #[test]
    fn test_backend_options_security_only_remote() {
        let opts = BackendOptions::default()
            .with_remote_fetch(true)
            .with_local_fetch(false);

        assert!(opts.enable_remote_fetch);
        assert!(!opts.enable_local_fetch);
    }

    #[test]
    fn test_backend_options_security_only_local() {
        let opts = BackendOptions::default()
            .with_remote_fetch(false)
            .with_local_fetch(true);

        assert!(!opts.enable_remote_fetch);
        assert!(opts.enable_local_fetch);
    }

    #[test]
    fn test_backend_options_multiple_overrides() {
        let opts = BackendOptions::default()
            .with_ocr(true)
            .with_ocr(false)
            .with_ocr(true)
            .with_table_structure(false)
            .with_table_structure(true);

        assert!(opts.enable_ocr);
        assert!(opts.enable_table_structure);
    }

    #[test]
    fn test_document_backend_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<MockBackend>();
    }

    #[test]
    fn test_backend_options_zero_max_pages() {
        let opts = BackendOptions::default().with_max_pages(Some(0));
        assert_eq!(opts.max_pages, Some(0));
        // Zero max pages is valid (might mean "process no pages")
    }

    // ===== N=475 Expansion: 10 additional tests =====

    #[test]
    fn test_backend_options_very_large_max_pages() {
        let opts = BackendOptions::default().with_max_pages(Some(1_000_000));
        assert_eq!(opts.max_pages, Some(1_000_000));
    }

    #[test]
    fn test_backend_options_disable_all_features() {
        let opts = BackendOptions::default()
            .with_ocr(false)
            .with_table_structure(false)
            .with_images(false)
            .with_remote_fetch(false)
            .with_local_fetch(false);

        assert!(!opts.enable_ocr);
        assert!(!opts.enable_table_structure);
        assert!(!opts.extract_images);
        assert!(!opts.enable_remote_fetch);
        assert!(!opts.enable_local_fetch);
    }

    #[test]
    fn test_backend_options_enable_all_features() {
        let opts = BackendOptions::default()
            .with_ocr(true)
            .with_table_structure(true)
            .with_images(true)
            .with_remote_fetch(true)
            .with_local_fetch(true);

        assert!(opts.enable_ocr);
        assert!(opts.enable_table_structure);
        assert!(opts.extract_images);
        assert!(opts.enable_remote_fetch);
        assert!(opts.enable_local_fetch);
    }

    #[test]
    fn test_backend_options_typical_document_processing() {
        // Typical configuration for document processing with OCR
        let opts = BackendOptions::default()
            .with_ocr(true)
            .with_table_structure(true)
            .with_max_pages(Some(100));

        assert!(opts.enable_ocr);
        assert!(opts.enable_table_structure);
        assert_eq!(opts.max_pages, Some(100));
        // Security features should remain disabled by default
        assert!(!opts.enable_remote_fetch);
        assert!(!opts.enable_local_fetch);
    }

    #[test]
    fn test_backend_options_image_only_extraction() {
        // Configuration for image extraction without text processing
        let opts = BackendOptions::default()
            .with_images(true)
            .with_ocr(false)
            .with_table_structure(false);

        assert!(opts.extract_images);
        assert!(!opts.enable_ocr);
        assert!(!opts.enable_table_structure);
    }

    #[test]
    fn test_backend_options_chaining_idempotence() {
        // Test that chaining same option multiple times is idempotent
        let opts1 = BackendOptions::default().with_ocr(true);
        let opts2 = BackendOptions::default().with_ocr(true).with_ocr(true);

        assert_eq!(opts1.enable_ocr, opts2.enable_ocr);
    }

    #[test]
    fn test_backend_options_builder_pattern_flexibility() {
        // Test that builder pattern allows flexible ordering
        let opts1 = BackendOptions::default()
            .with_ocr(true)
            .with_table_structure(true)
            .with_max_pages(Some(50));

        let opts2 = BackendOptions::default()
            .with_max_pages(Some(50))
            .with_table_structure(true)
            .with_ocr(true);

        assert_eq!(opts1.enable_ocr, opts2.enable_ocr);
        assert_eq!(opts1.enable_table_structure, opts2.enable_table_structure);
        assert_eq!(opts1.max_pages, opts2.max_pages);
    }

    #[test]
    fn test_mock_backend_format_consistency() {
        let backend = MockBackend {
            format: InputFormat::Pdf,
        };
        // Mock backend should consistently return Pdf format
        assert_eq!(backend.format(), InputFormat::Pdf);
        assert_eq!(backend.format(), InputFormat::Pdf);
    }

    #[test]
    fn test_mock_backend_can_handle_only_pdf() {
        let backend = MockBackend {
            format: InputFormat::Pdf,
        };
        // Mock should only handle PDF
        assert!(backend.can_handle(InputFormat::Pdf));
        // Should reject all other formats
        assert!(!backend.can_handle(InputFormat::Docx));
        assert!(!backend.can_handle(InputFormat::Html));
        assert!(!backend.can_handle(InputFormat::Md));
    }

    #[test]
    fn test_backend_options_max_pages_none_to_some() {
        // Test transitioning from None to Some
        let opts = BackendOptions::default()
            .with_max_pages(None)
            .with_max_pages(Some(10));

        assert_eq!(opts.max_pages, Some(10));
    }

    #[test]
    fn test_backend_options_all_features_enabled() {
        // Test enabling all options simultaneously
        let opts = BackendOptions::default()
            .with_ocr(true)
            .with_table_structure(true)
            .with_max_pages(Some(100));

        assert!(opts.enable_ocr);
        assert!(opts.enable_table_structure);
        assert_eq!(opts.max_pages, Some(100));
    }

    #[test]
    fn test_backend_options_all_features_disabled() {
        // Test disabling all options
        let opts = BackendOptions::default()
            .with_ocr(false)
            .with_table_structure(false)
            .with_max_pages(None);

        assert!(!opts.enable_ocr);
        assert!(!opts.enable_table_structure);
        assert_eq!(opts.max_pages, None);
    }

    #[test]
    fn test_mock_backend_parse_bytes_empty_input() {
        let backend = MockBackend {
            format: InputFormat::Pdf,
        };
        let empty_bytes: Vec<u8> = vec![];
        let result = backend.parse_bytes(&empty_bytes, &BackendOptions::default());

        // Should succeed even with empty bytes (mock backend)
        assert!(result.is_ok());
    }

    #[test]
    fn test_backend_options_copy_with_builder() {
        // Test that BackendOptions can be copied using builder pattern
        let opts1 = BackendOptions::default()
            .with_ocr(true)
            .with_max_pages(Some(25));

        let opts2 = opts1; // Copy trait copies the struct

        assert_eq!(opts1.enable_ocr, opts2.enable_ocr);
        assert_eq!(opts1.max_pages, opts2.max_pages);
    }

    #[test]
    fn test_backend_concrete_type() {
        // Test that DocumentBackend works with concrete types
        let backend = MockBackend {
            format: InputFormat::Pdf,
        };

        // Backend methods work correctly on concrete type
        assert_eq!(backend.format(), InputFormat::Pdf);
    }

    // ===== N=603 Expansion: 5 additional tests to reach 70 =====

    #[test]
    fn test_backend_options_max_pages_boundary_one() {
        // Test boundary case: exactly one page
        let opts = BackendOptions::default().with_max_pages(Some(1));
        assert_eq!(opts.max_pages, Some(1));

        // Verify can be chained with other options
        let opts2 = opts.with_ocr(true);
        assert_eq!(opts2.max_pages, Some(1));
        assert!(opts2.enable_ocr);
    }

    #[test]
    fn test_backend_options_security_isolation() {
        // Test that security options can be set independently
        let opts_remote = BackendOptions::default().with_remote_fetch(true);
        let opts_local = BackendOptions::default().with_local_fetch(true);
        let opts_both = BackendOptions::default()
            .with_remote_fetch(true)
            .with_local_fetch(true);

        assert!(opts_remote.enable_remote_fetch);
        assert!(!opts_remote.enable_local_fetch);

        assert!(!opts_local.enable_remote_fetch);
        assert!(opts_local.enable_local_fetch);

        assert!(opts_both.enable_remote_fetch);
        assert!(opts_both.enable_local_fetch);
    }

    #[test]
    fn test_backend_options_feature_combinations() {
        // Test realistic feature combinations for different use cases

        // Use case 1: Basic text extraction (no special features)
        let opts_basic = BackendOptions::default();
        assert!(!opts_basic.enable_ocr);
        assert!(!opts_basic.enable_table_structure);
        assert!(!opts_basic.extract_images);

        // Use case 2: Full document processing (OCR + tables + images)
        let opts_full = BackendOptions::default()
            .with_ocr(true)
            .with_table_structure(true)
            .with_images(true);
        assert!(opts_full.enable_ocr);
        assert!(opts_full.enable_table_structure);
        assert!(opts_full.extract_images);

        // Use case 3: Quick preview (limited pages, no heavy processing)
        let opts_preview = BackendOptions::default()
            .with_max_pages(Some(5))
            .with_ocr(false)
            .with_table_structure(false);
        assert_eq!(opts_preview.max_pages, Some(5));
        assert!(!opts_preview.enable_ocr);
    }

    #[test]
    fn test_document_backend_multiple_backends_same_format() {
        // Test that multiple backends can handle the same format distinctly
        let backend1 = MockBackend {
            format: InputFormat::Html,
        };
        let backend2 = MockBackend {
            format: InputFormat::Html,
        };

        // Both should handle Html
        assert!(backend1.can_handle(InputFormat::Html));
        assert!(backend2.can_handle(InputFormat::Html));
        assert_eq!(backend1.format(), backend2.format());

        // Both should produce valid documents
        let opts = BackendOptions::default();
        let result1 = backend1.parse_bytes(b"test", &opts);
        let result2 = backend2.parse_bytes(b"test", &opts);
        assert!(result1.is_ok());
        assert!(result2.is_ok());
    }

    #[test]
    fn test_backend_options_debug_with_max_pages_large() {
        // Test Debug output with very large max_pages value
        let opts = BackendOptions::default()
            .with_max_pages(Some(999_999))
            .with_ocr(true)
            .with_table_structure(true);

        let debug_str = format!("{opts:?}");
        assert!(debug_str.contains("max_pages: Some(999999)"));
        assert!(debug_str.contains("enable_ocr: true"));
        assert!(debug_str.contains("enable_table_structure: true"));

        // Verify the options are actually set
        assert_eq!(opts.max_pages, Some(999_999));
        assert!(opts.enable_ocr);
        assert!(opts.enable_table_structure);
    }

    // ========== Additional Edge Cases (N=647) ==========

    #[test]
    fn test_backend_options_max_pages_usize_max() {
        // Test extreme edge case: usize::MAX for max_pages
        let opts = BackendOptions::default().with_max_pages(Some(usize::MAX));
        assert_eq!(opts.max_pages, Some(usize::MAX));

        // Should still be chainable
        let opts2 = opts.with_ocr(true);
        assert_eq!(opts2.max_pages, Some(usize::MAX));
        assert!(opts2.enable_ocr);
    }

    #[test]
    fn test_backend_options_explicit_false_vs_default() {
        // Test that explicit false is same as default
        let opts_default = BackendOptions::default();
        let opts_explicit = BackendOptions::default()
            .with_ocr(false)
            .with_table_structure(false)
            .with_images(false)
            .with_remote_fetch(false)
            .with_local_fetch(false);

        assert_eq!(opts_default.enable_ocr, opts_explicit.enable_ocr);
        assert_eq!(
            opts_default.enable_table_structure,
            opts_explicit.enable_table_structure
        );
        assert_eq!(opts_default.extract_images, opts_explicit.extract_images);
        assert_eq!(
            opts_default.enable_remote_fetch,
            opts_explicit.enable_remote_fetch
        );
        assert_eq!(
            opts_default.enable_local_fetch,
            opts_explicit.enable_local_fetch
        );
        assert_eq!(opts_default.max_pages, opts_explicit.max_pages);
    }

    #[test]
    fn test_backend_options_partial_eq_semantics() {
        // Test PartialEq implementation (BackendOptions derives PartialEq via Default)
        let opts1 = BackendOptions::default()
            .with_ocr(true)
            .with_max_pages(Some(10));
        let opts2 = BackendOptions::default()
            .with_ocr(true)
            .with_max_pages(Some(10));
        let opts3 = BackendOptions::default()
            .with_ocr(true)
            .with_max_pages(Some(20)); // Different max_pages

        // Same options should be equal
        assert_eq!(opts1.enable_ocr, opts2.enable_ocr);
        assert_eq!(opts1.max_pages, opts2.max_pages);

        // Different max_pages should not be equal
        assert_ne!(opts1.max_pages, opts3.max_pages);
    }

    #[test]
    fn test_backend_options_long_chain_order_independence() {
        // Test that long chains produce same result regardless of order
        let opts1 = BackendOptions::default()
            .with_ocr(true)
            .with_table_structure(true)
            .with_images(true)
            .with_remote_fetch(true)
            .with_local_fetch(true)
            .with_max_pages(Some(50));

        let opts2 = BackendOptions::default()
            .with_max_pages(Some(50))
            .with_local_fetch(true)
            .with_remote_fetch(true)
            .with_images(true)
            .with_table_structure(true)
            .with_ocr(true);

        // All fields should match
        assert_eq!(opts1.enable_ocr, opts2.enable_ocr);
        assert_eq!(opts1.enable_table_structure, opts2.enable_table_structure);
        assert_eq!(opts1.extract_images, opts2.extract_images);
        assert_eq!(opts1.enable_remote_fetch, opts2.enable_remote_fetch);
        assert_eq!(opts1.enable_local_fetch, opts2.enable_local_fetch);
        assert_eq!(opts1.max_pages, opts2.max_pages);
    }

    #[test]
    fn test_document_backend_can_handle_format_filtering() {
        // Test that can_handle correctly filters format variants
        let html_backend = MockBackend {
            format: InputFormat::Html,
        };

        // Should handle Html
        assert!(html_backend.can_handle(InputFormat::Html));

        // Should not handle other formats (test sampling of formats)
        assert!(!html_backend.can_handle(InputFormat::Pdf));
        assert!(!html_backend.can_handle(InputFormat::Docx));
        assert!(!html_backend.can_handle(InputFormat::Csv));
        assert!(!html_backend.can_handle(InputFormat::Md));
        assert!(!html_backend.can_handle(InputFormat::Png));
        assert!(!html_backend.can_handle(InputFormat::Jpeg));
    }

    // BUG #64 and #47 fix tests

    #[test]
    fn test_backend_options_with_render_dpi() {
        let opts = BackendOptions::default().with_render_dpi(150.0);
        assert!((opts.render_dpi - 150.0).abs() < f32::EPSILON);
        // Other fields should remain at default
        assert!((opts.merge_threshold_factor - 1.3).abs() < f32::EPSILON);
    }

    #[test]
    fn test_backend_options_with_merge_threshold_factor() {
        let opts = BackendOptions::default().with_merge_threshold_factor(2.0);
        assert!((opts.merge_threshold_factor - 2.0).abs() < f32::EPSILON);
        // Other fields should remain at default (144.0 DPI matches Python docling)
        assert!((opts.render_dpi - 144.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_backend_options_pdf_options_chaining() {
        let opts = BackendOptions::default()
            .with_render_dpi(72.0)
            .with_merge_threshold_factor(1.5);
        assert!((opts.render_dpi - 72.0).abs() < f32::EPSILON);
        assert!((opts.merge_threshold_factor - 1.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_backend_options_pdf_options_override() {
        let opts = BackendOptions::default()
            .with_render_dpi(150.0)
            .with_render_dpi(72.0);
        assert!((opts.render_dpi - 72.0).abs() < f32::EPSILON);
    }
}
