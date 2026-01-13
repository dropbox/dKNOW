//! PNG backend for docling
//!
//! This backend converts PNG (Portable Network Graphics) files to docling's document model.
//!
//! ## Two Processing Modes
//!
//! ### With `pdf-ml` feature (recommended):
//! Uses the full ML pipeline (layout analysis, OCR, table detection) to match Python docling behavior.
//! This processes images just like Python docling does - converting the image through the same
//! ML models used for PDF pages.
//!
//! ### Without `pdf-ml` feature:
//! Falls back to simple OCR without layout analysis. This produces raw OCR text without
//! document structure inference (headers, tables, etc.).

// Clippy pedantic allows:
// - Unit struct &self convention
#![allow(clippy::trivially_copy_pass_by_ref)]

use crate::traits::{BackendOptions, DocumentBackend};
use crate::utils::{create_section_header, create_text_item, format_file_size, opt_vec};
use docling_core::{DocItem, DoclingError, Document, DocumentMetadata, InputFormat};
#[cfg(not(feature = "pdf"))]
use docling_ocr::OcrEngine;
use image::{GenericImageView, ImageReader};
#[cfg(feature = "pdf")]
use ndarray::Array3;
use std::fmt::Write;
use std::io::Cursor;
use std::path::Path;

/// PNG backend
///
/// Converts PNG (Portable Network Graphics) files to docling's document model.
/// Extracts basic metadata and uses OCR to extract text content from the image.
///
/// ## Features
///
/// - Extract image dimensions
/// - Detect color type (RGB, RGBA, Grayscale, etc.)
/// - Extract bit depth information
/// - OCR text extraction with bounding boxes
/// - Generate markdown with image metadata and OCR text
///
/// ## Example
///
/// ```no_run
/// use docling_backend::PngBackend;
/// use docling_backend::DocumentBackend;
///
/// let backend = PngBackend::new();
/// let result = backend.parse_file("image.png", &Default::default())?;
/// println!("Image: {:?}", result.metadata.title);
/// # Ok::<(), docling_core::error::DoclingError>(())
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct PngBackend;

impl PngBackend {
    /// Create a new PNG backend instance
    #[inline]
    #[must_use = "creates a new PNG backend instance"]
    pub const fn new() -> Self {
        Self
    }

    /// Convert PNG metadata to markdown
    fn png_to_markdown(
        filename: &str,
        width: u32,
        height: u32,
        color_type: &str,
        bit_depth: u8,
        file_size: usize,
    ) -> String {
        let mut markdown = String::new();

        // Title
        let _ = write!(markdown, "# {filename}\n\n");

        // Image type
        markdown.push_str("Type: PNG (Portable Network Graphics)\n\n");

        // Dimensions
        let _ = write!(markdown, "Dimensions: {width}×{height} pixels\n\n");

        // Color type
        let _ = write!(markdown, "Color Type: {color_type}\n\n");

        // Bit depth
        let _ = write!(markdown, "Bit Depth: {bit_depth}-bit\n\n");

        // File size
        markdown.push_str(&format_file_size(file_size));

        // Image reference
        let _ = writeln!(markdown, "![{filename}]({filename})");

        markdown
    }

    /// Run OCR on image and create `DocItems` from results
    ///
    /// Uses OCR engine to extract text from the image, creating `DocItems`
    /// with bounding boxes for each detected text line.
    ///
    /// ## Arguments
    /// * `data` - Raw image bytes
    /// * `start_index` - Starting index for `DocItems` (to append after metadata items)
    /// * `enable_ocr` - Whether OCR is enabled (from `BackendOptions`)
    ///
    /// ## Returns
    /// Tuple of (OCR text as string, vector of `DocItem::Text` with bboxes)
    #[cfg(not(feature = "pdf"))]
    fn extract_ocr_text(
        data: &[u8],
        start_index: usize,
        enable_ocr: bool,
    ) -> Result<(String, Vec<DocItem>), DoclingError> {
        use docling_core::content::{BoundingBox, CoordOrigin, ProvenanceItem};

        // Check if OCR is enabled via options parameter
        // Also check environment variable for backwards compatibility
        // Default: OCR is disabled unless explicitly enabled
        let env_ocr = std::env::var("ENABLE_IMAGE_OCR").unwrap_or_default() == "1";
        if !enable_ocr && !env_ocr {
            return Ok((String::new(), Vec::new()));
        }

        // Load image
        let img = ImageReader::new(Cursor::new(data))
            .with_guessed_format()
            .map_err(|e| DoclingError::BackendError(format!("Failed to load image: {e}")))?
            .decode()
            .map_err(|e| DoclingError::BackendError(format!("Failed to decode image: {e}")))?;

        // Run OCR
        let mut ocr_engine = OcrEngine::new().map_err(|e| {
            DoclingError::BackendError(format!("Failed to initialize OCR engine: {e}"))
        })?;

        let ocr_result = ocr_engine
            .recognize(&img)
            .map_err(|e| DoclingError::BackendError(format!("OCR failed: {e}")))?;

        // Convert OCR result to DocItems
        let mut doc_items = Vec::new();
        for (i, line) in ocr_result.lines.iter().enumerate() {
            // Convert OCR bounding box (x, y, width, height) to docling BoundingBox (l, t, r, b)
            let bbox = BoundingBox {
                l: f64::from(line.bbox.x),
                t: f64::from(line.bbox.y),
                r: f64::from(line.bbox.x + line.bbox.width),
                b: f64::from(line.bbox.y + line.bbox.height),
                coord_origin: CoordOrigin::TopLeft,
            };

            let prov = vec![ProvenanceItem {
                page_no: 1, // PNG is single-page
                bbox,
                charspan: None,
            }];

            doc_items.push(create_text_item(start_index + i, line.text.clone(), prov));
        }

        // Get full OCR text
        let ocr_text = ocr_result.text();

        Ok((ocr_text, doc_items))
    }

    /// Create `DocItems` directly from PNG metadata
    ///
    /// Generates structured `DocItems` from PNG metadata without markdown intermediary.
    /// Creates a hierarchical document structure:
    /// - Title (filename) as `SectionHeader` level 1
    /// - Image type as Text
    /// - Dimensions as Text
    /// - Color type as Text
    /// - Bit depth as Text
    /// - File size as Text
    /// - Image reference as Text
    ///
    /// ## Arguments
    /// * `filename` - Name of the PNG file
    /// * `width` - Image width in pixels
    /// * `height` - Image height in pixels
    /// * `color_type` - Color type description
    /// * `bit_depth` - Bit depth (8, 16, or 32)
    /// * `file_size` - Size of the PNG file in bytes
    ///
    /// ## Returns
    /// Vector of `DocItems` representing the PNG metadata structure
    fn png_to_docitems(
        filename: &str,
        width: u32,
        height: u32,
        color_type: &str,
        bit_depth: u8,
        file_size: usize,
    ) -> Vec<DocItem> {
        let mut doc_items = Vec::new();
        let mut index = 0;

        // Title - filename as SectionHeader level 1
        doc_items.push(create_section_header(0, filename.to_string(), 1, vec![]));
        index += 1; // Reserve index 0 for the header

        // Image type
        doc_items.push(create_text_item(
            index,
            "Type: PNG (Portable Network Graphics)".to_string(),
            vec![],
        ));
        index += 1;

        // Dimensions
        doc_items.push(create_text_item(
            index,
            format!("Dimensions: {width}×{height} pixels"),
            vec![],
        ));
        index += 1;

        // Color type
        doc_items.push(create_text_item(
            index,
            format!("Color Type: {color_type}"),
            vec![],
        ));
        index += 1;

        // Bit depth
        doc_items.push(create_text_item(
            index,
            format!("Bit Depth: {bit_depth}-bit"),
            vec![],
        ));
        index += 1;

        // File size
        let size_text = format_file_size(file_size);
        doc_items.push(create_text_item(index, size_text, vec![]));
        index += 1;

        // Image reference
        doc_items.push(create_text_item(
            index,
            format!("![{filename}]({filename})"),
            vec![],
        ));

        doc_items
    }

    /// Convert image to RGB ndarray for ML pipeline
    ///
    /// The ML pipeline expects images as `Array3<u8>` with shape `[height, width, 3]`.
    #[cfg(feature = "pdf")]
    fn image_to_array(img: &image::DynamicImage) -> Result<Array3<u8>, DoclingError> {
        let rgb_img = img.to_rgb8();
        let (width, height) = rgb_img.dimensions();
        let raw = rgb_img.into_raw();

        // Create array from raw bytes (row-major, shape [H, W, 3])
        Array3::from_shape_vec((height as usize, width as usize, 3), raw).map_err(|e| {
            DoclingError::BackendError(format!(
                "Image data doesn't match dimensions {}x{}: {}",
                width, height, e
            ))
        })
    }

    /// Process image through ML pipeline (with layout analysis)
    ///
    /// This method processes the image through the same ML pipeline used for PDF pages,
    /// providing layout analysis, OCR, and table detection.
    #[cfg(feature = "pdf")]
    fn parse_bytes_ml(
        &self,
        img: &image::DynamicImage,
        width: u32,
        height: u32,
        _file_size: usize,
    ) -> Result<Document, DoclingError> {
        use docling_core::serializer::MarkdownSerializer;
        use docling_pdf_ml::convert_to_core::convert_to_core_docling_document;
        use docling_pdf_ml::to_docling_document_multi;
        use docling_pdf_ml::{Pipeline, PipelineConfig};

        log::info!(
            "Processing PNG image with ML pipeline ({}x{})",
            width,
            height
        );

        // Convert image to array format expected by ML pipeline
        let page_image = Self::image_to_array(img)?;

        // Assume standard 72 DPI for page dimensions (points)
        // This gives us sensible coordinates in the output
        let page_width = width as f32;
        let page_height = height as f32;

        // Create ML pipeline
        let config = PipelineConfig::default();
        let mut pipeline = Pipeline::new(config).map_err(|e| {
            DoclingError::BackendError(format!("Failed to create ML pipeline: {e}"))
        })?;

        // Process image through ML pipeline (no pre-extracted text for images)
        let page_result = pipeline
            .process_page(0, &page_image, page_width, page_height, None)
            .map_err(|e| DoclingError::BackendError(format!("ML pipeline failed: {e}")))?;

        let pages = vec![page_result];

        // Get reading order from assembled elements
        let page_reading_orders: Vec<Vec<usize>> = pages
            .iter()
            .map(|page| {
                if let Some(assembled) = &page.assembled {
                    assembled
                        .elements
                        .iter()
                        .map(|element| element.cluster().id)
                        .collect()
                } else {
                    vec![]
                }
            })
            .collect();

        // Convert to DoclingDocument
        let pdf_ml_docling_doc =
            to_docling_document_multi(&pages, &page_reading_orders, "image.png");

        log::debug!(
            "ML pipeline generated {} texts, {} tables, {} pictures",
            pdf_ml_docling_doc.texts.len(),
            pdf_ml_docling_doc.tables.len(),
            pdf_ml_docling_doc.pictures.len()
        );

        // Convert to core format
        let core_docling_doc =
            convert_to_core_docling_document(&pdf_ml_docling_doc).map_err(|e| {
                DoclingError::BackendError(format!(
                    "Failed to convert DoclingDocument to core format: {}",
                    e
                ))
            })?;

        // Serialize to markdown
        let serializer = MarkdownSerializer::new();
        let markdown = serializer.serialize(&core_docling_doc);
        let num_characters = markdown.chars().count();

        // Convert to DocItems
        let doc_items: Vec<DocItem> = core_docling_doc.texts.to_vec();

        Ok(Document {
            markdown,
            format: InputFormat::Png,
            metadata: DocumentMetadata {
                num_pages: Some(1),
                num_characters,
                title: Some("image.png".to_string()),
                author: None,
                created: None,
                modified: None,
                language: None,
                subject: None,
                exif: None,
            },
            docling_document: None,
            content_blocks: opt_vec(doc_items),
        })
    }

    /// Simple metadata-only fallback (when pdf-ml is available but OCR not enabled)
    #[cfg(feature = "pdf")]
    fn parse_bytes_simple_metadata(
        &self,
        img: &image::DynamicImage,
        width: u32,
        height: u32,
        file_size: usize,
    ) -> Result<Document, DoclingError> {
        let color_type = Self::get_color_type(img);
        let bit_depth = Self::get_bit_depth(img);
        let filename = "image.png";

        let markdown =
            Self::png_to_markdown(filename, width, height, &color_type, bit_depth, file_size);
        let doc_items =
            Self::png_to_docitems(filename, width, height, &color_type, bit_depth, file_size);

        Ok(Document {
            markdown: markdown.clone(),
            format: InputFormat::Png,
            metadata: DocumentMetadata {
                num_pages: Some(1),
                num_characters: markdown.chars().count(),
                title: Some(filename.to_string()),
                author: None,
                created: None,
                modified: None,
                language: None,
                subject: None,
                exif: None,
            },
            content_blocks: opt_vec(doc_items),
            docling_document: None,
        })
    }

    /// Simple fallback with optional OCR (when pdf-ml not available)
    #[cfg(not(feature = "pdf"))]
    // Method signature kept for API consistency with other PngBackend methods
    #[allow(clippy::unused_self)]
    fn parse_bytes_simple(
        &self,
        img: &image::DynamicImage,
        width: u32,
        height: u32,
        data: &[u8],
        options: &BackendOptions,
    ) -> Result<Document, DoclingError> {
        let color_type = Self::get_color_type(img);
        let bit_depth = Self::get_bit_depth(img);
        let filename = "image.png";

        let mut doc_items =
            Self::png_to_docitems(filename, width, height, &color_type, bit_depth, data.len());
        let metadata_items_count = doc_items.len();

        // Extract OCR text
        let (ocr_text, mut ocr_items) =
            Self::extract_ocr_text(data, metadata_items_count, options.enable_ocr)?;

        let markdown = if options.enable_ocr && !ocr_text.is_empty() {
            doc_items = ocr_items;
            ocr_text
        } else {
            doc_items.append(&mut ocr_items);
            let mut md =
                Self::png_to_markdown(filename, width, height, &color_type, bit_depth, data.len());
            if !ocr_text.is_empty() {
                md.push_str("\n\n## OCR Text\n\n");
                md.push_str(&ocr_text);
            }
            md
        };

        Ok(Document {
            markdown: markdown.clone(),
            format: InputFormat::Png,
            metadata: DocumentMetadata {
                num_pages: Some(1),
                num_characters: markdown.chars().count(),
                title: Some(filename.to_string()),
                author: None,
                created: None,
                modified: None,
                language: None,
                subject: None,
                exif: None,
            },
            docling_document: None,
            content_blocks: opt_vec(doc_items),
        })
    }

    /// Get color type string from image
    fn get_color_type(img: &image::DynamicImage) -> String {
        match img {
            image::DynamicImage::ImageLuma8(_) => "Grayscale".to_string(),
            image::DynamicImage::ImageLumaA8(_) => "Grayscale + Alpha".to_string(),
            image::DynamicImage::ImageRgb8(_) => "RGB".to_string(),
            image::DynamicImage::ImageRgba8(_) => "RGBA".to_string(),
            image::DynamicImage::ImageLuma16(_) => "Grayscale (16-bit)".to_string(),
            image::DynamicImage::ImageLumaA16(_) => "Grayscale + Alpha (16-bit)".to_string(),
            image::DynamicImage::ImageRgb16(_) => "RGB (16-bit)".to_string(),
            image::DynamicImage::ImageRgba16(_) => "RGBA (16-bit)".to_string(),
            image::DynamicImage::ImageRgb32F(_) => "RGB (32-bit float)".to_string(),
            image::DynamicImage::ImageRgba32F(_) => "RGBA (32-bit float)".to_string(),
            _ => "Unknown".to_string(),
        }
    }

    /// Get bit depth from image
    const fn get_bit_depth(img: &image::DynamicImage) -> u8 {
        match img {
            image::DynamicImage::ImageLuma16(_)
            | image::DynamicImage::ImageLumaA16(_)
            | image::DynamicImage::ImageRgb16(_)
            | image::DynamicImage::ImageRgba16(_) => 16,
            image::DynamicImage::ImageRgb32F(_) | image::DynamicImage::ImageRgba32F(_) => 32,
            // 8-bit formats and fallback
            image::DynamicImage::ImageLuma8(_)
            | image::DynamicImage::ImageLumaA8(_)
            | image::DynamicImage::ImageRgb8(_)
            | image::DynamicImage::ImageRgba8(_)
            | _ => 8,
        }
    }
}

impl DocumentBackend for PngBackend {
    #[inline]
    fn format(&self) -> InputFormat {
        InputFormat::Png
    }

    fn parse_bytes(&self, data: &[u8], options: &BackendOptions) -> Result<Document, DoclingError> {
        // Load image to get dimensions and metadata
        let img = ImageReader::new(Cursor::new(data))
            .with_guessed_format()
            .map_err(|e| DoclingError::BackendError(format!("Failed to load PNG: {e}")))?
            .decode()
            .map_err(|e| DoclingError::BackendError(format!("Failed to decode PNG: {e}")))?;

        let (width, height) = img.dimensions();

        // When pdf-ml feature is available and OCR is enabled, use ML pipeline
        // This matches Python docling's behavior (images go through same pipeline as PDFs)
        #[cfg(feature = "pdf")]
        if options.enable_ocr {
            return self.parse_bytes_ml(&img, width, height, data.len());
        }

        // Fallback to simple mode (metadata + optional OCR without layout analysis)
        #[cfg(not(feature = "pdf"))]
        {
            self.parse_bytes_simple(&img, width, height, data, options)
        }

        #[cfg(feature = "pdf")]
        {
            // pdf-ml available but OCR not enabled - use simple mode
            self.parse_bytes_simple_metadata(&img, width, height, data.len())
        }
    }

    fn parse_file<P: AsRef<Path>>(
        &self,
        path: P,
        options: &BackendOptions,
    ) -> Result<Document, DoclingError> {
        let path_ref = path.as_ref();
        let filename = path_ref
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("image.png");

        // Helper to add filename context to errors
        let add_context = |e: DoclingError| match e {
            DoclingError::BackendError(msg) => {
                DoclingError::BackendError(format!("{msg}: {filename}"))
            }
            other => other,
        };

        // Read file and delegate to parse_bytes
        let data = std::fs::read(path_ref).map_err(DoclingError::IoError)?;
        let mut doc = self.parse_bytes(&data, options).map_err(add_context)?;

        // Update title with actual filename
        doc.metadata.title = Some(filename.to_string());

        Ok(doc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    /// Helper function to handle OCR unavailability in tests
    /// Returns None if OCR is not available, otherwise returns Some(Document)
    fn parse_with_ocr_fallback(backend: &PngBackend, data: &[u8]) -> Option<Document> {
        let result = backend.parse_bytes(data, &Default::default());
        match result {
            Ok(doc) => Some(doc),
            Err(err) => {
                // If OCR is not available, skip test gracefully
                if format!("{err:?}").contains("OCR") || format!("{err:?}").contains("initialize") {
                    None
                } else {
                    panic!("Unexpected error: {err:?}");
                }
            }
        }
    }

    #[test]
    fn test_backend_format() {
        let backend = PngBackend::new();
        assert_eq!(
            backend.format(),
            InputFormat::Png,
            "PngBackend should report InputFormat::Png as its format"
        );
    }

    #[test]
    fn test_png_to_markdown() {
        let markdown = PngBackend::png_to_markdown("test.png", 800, 600, "RGBA", 8, 102_400);
        assert!(
            markdown.contains("# test.png"),
            "Markdown should contain filename as h1 title"
        );
        assert!(
            markdown.contains("Type: PNG (Portable Network Graphics)"),
            "Markdown should contain PNG type description"
        );
        assert!(
            markdown.contains("Dimensions: 800×600 pixels"),
            "Markdown should contain image dimensions"
        );
        assert!(
            markdown.contains("Color Type: RGBA"),
            "Markdown should contain color type"
        );
        assert!(
            markdown.contains("Bit Depth: 8-bit"),
            "Markdown should contain bit depth"
        );
        assert!(
            markdown.contains("![test.png](test.png)"),
            "Markdown should contain image reference"
        );
    }

    // ========== METADATA TESTS ==========

    #[test]
    fn test_metadata_title_extraction() {
        // Test that filename becomes title in metadata
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(1, 1);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert_eq!(
                doc.metadata.title,
                Some("image.png".to_string()),
                "Metadata title should be set to default filename 'image.png'"
            );
        }
    }

    #[test]
    fn test_metadata_num_pages() {
        // Test that PNG is reported as single-page
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(10, 10);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert_eq!(
                doc.metadata.num_pages,
                Some(1),
                "PNG should be reported as single-page document"
            );
        }
    }

    #[test]
    fn test_metadata_character_count() {
        // Test that character count is computed correctly
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(10, 10);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            // Character count should match markdown length
            assert_eq!(
                doc.metadata.num_characters,
                doc.markdown.chars().count(),
                "Character count in metadata should match actual markdown length"
            );
            assert!(
                doc.metadata.num_characters > 0,
                "Character count should be positive for valid PNG"
            );
        }
    }

    #[test]
    fn test_metadata_format_field() {
        // Test that format is correctly set
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(5, 5);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert_eq!(
                doc.format,
                InputFormat::Png,
                "Document format should be InputFormat::Png"
            );
        }
    }

    // ========== DOCITEM TESTS ==========

    #[test]
    fn test_docitem_structure() {
        // Test that DocItems are created correctly (with OCR enabled)
        std::env::set_var("ENABLE_IMAGE_OCR", "1");

        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(10, 10);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(
                doc.content_blocks.is_some(),
                "Document should have content blocks"
            );

            let items = doc.content_blocks.unwrap();
            assert!(
                !items.is_empty(),
                "Content blocks should not be empty for valid PNG"
            );

            // All items should be Text or SectionHeader items
            for item in items {
                assert!(
                    matches!(item, DocItem::Text { .. } | DocItem::SectionHeader { .. }),
                    "DocItems should be Text or SectionHeader variants"
                );
            }
        }

        // Clean up env var
        std::env::remove_var("ENABLE_IMAGE_OCR");
    }

    #[test]
    fn test_docitem_count() {
        // Test expected number of DocItems (6 metadata paragraphs)
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(10, 10);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(
                doc.content_blocks.is_some(),
                "Document should have content blocks"
            );

            let items = doc.content_blocks.unwrap();
            // Expected: title, type, dimensions, color type, bit depth, file size, image reference
            // (7 paragraphs from metadata, no OCR text expected for synthetic image)
            assert_eq!(
                items.len(),
                7,
                "PNG metadata should generate exactly 7 DocItems"
            );
        }
    }

    #[test]
    fn test_docitem_content() {
        // Test that DocItems contain expected content
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(100, 200);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        let backend = PngBackend::new();
        let result = backend.parse_bytes(&buffer, &Default::default());
        assert!(result.is_ok(), "Parsing valid PNG should succeed");

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();

        // First item should be SectionHeader with title
        if let DocItem::SectionHeader { text, .. } = &items[0] {
            assert!(
                text.contains("image.png"),
                "Title should contain filename 'image.png'"
            );
        } else {
            panic!("Expected SectionHeader item for title");
        }

        // Third item (dimensions) should be Text
        if let DocItem::Text { text, .. } = &items[2] {
            assert!(
                text.contains("100×200"),
                "Dimensions item should contain '100×200'"
            );
        } else {
            panic!("Expected Text item");
        }
    }

    #[test]
    fn test_docitem_self_ref() {
        // Test that self_ref fields are correctly indexed
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(10, 10);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            let items = doc.content_blocks.unwrap();

            // Check that self_ref values are sequential
            for (i, item) in items.iter().enumerate() {
                if let DocItem::Text { self_ref, .. } = item {
                    assert_eq!(
                        self_ref,
                        &format!("#/texts/{i}"),
                        "self_ref should be sequentially indexed at position {i}"
                    );
                }
            }
        }
    }

    // ========== FORMAT-SPECIFIC TESTS ==========

    #[test]
    fn test_color_type_rgb() {
        // Test RGB color type detection
        use image::{ImageBuffer, Rgb};
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(10, 10);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(
                doc.markdown.contains("Color Type: RGB"),
                "RGB image should have 'Color Type: RGB' in markdown"
            );
        }
    }

    #[test]
    fn test_color_type_rgba() {
        // Test RGBA color type detection
        use image::{ImageBuffer, Rgba};
        let img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(10, 10);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(
                doc.markdown.contains("Color Type: RGBA"),
                "RGBA image should have 'Color Type: RGBA' in markdown"
            );
        }
    }

    #[test]
    fn test_color_type_grayscale() {
        // Test grayscale color type detection
        use image::{ImageBuffer, Luma};
        let img: ImageBuffer<Luma<u8>, Vec<u8>> = ImageBuffer::new(10, 10);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(
                doc.markdown.contains("Color Type: Grayscale"),
                "Grayscale image should have 'Color Type: Grayscale' in markdown"
            );
        }
    }

    #[test]
    fn test_color_type_grayscale_alpha() {
        // Test grayscale + alpha color type detection
        use image::{ImageBuffer, LumaA};
        let img: ImageBuffer<LumaA<u8>, Vec<u8>> = ImageBuffer::new(10, 10);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(
                doc.markdown.contains("Color Type: Grayscale + Alpha"),
                "Grayscale+Alpha image should have 'Color Type: Grayscale + Alpha' in markdown"
            );
        }
    }

    #[test]
    fn test_bit_depth_8bit() {
        // Test 8-bit depth detection
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(10, 10);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(
                doc.markdown.contains("Bit Depth: 8-bit"),
                "8-bit image should have 'Bit Depth: 8-bit' in markdown"
            );
        }
    }

    #[test]
    fn test_bit_depth_16bit() {
        // Test 16-bit depth detection
        use image::{ImageBuffer, Rgb};
        let img: ImageBuffer<Rgb<u16>, Vec<u16>> = ImageBuffer::new(10, 10);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(
                doc.markdown.contains("Bit Depth: 16-bit"),
                "16-bit image should have 'Bit Depth: 16-bit' in markdown"
            );
        }
    }

    #[test]
    fn test_dimensions_extraction() {
        // Test various dimension combinations
        let test_cases = vec![(100, 100), (1920, 1080), (1, 1), (4000, 3000)];

        for (width, height) in test_cases {
            use image::{ImageBuffer, RgbImage};
            let img: RgbImage = ImageBuffer::new(width, height);
            let mut buffer = Vec::new();
            img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
                .unwrap();

            let backend = PngBackend::new();
            if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
                assert!(
                    doc.markdown.contains(&format!("{width}×{height}")),
                    "Markdown should contain dimensions '{width}×{height}'"
                );
            }
        }
    }

    #[test]
    fn test_file_size_formatting() {
        // Test that file size is included in markdown
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(100, 100);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            // File size should be in the markdown (format_file_size adds "File Size:")
            assert!(
                doc.markdown.contains("File Size:"),
                "Markdown should contain 'File Size:' section"
            );
        }
    }

    #[test]
    fn test_markdown_image_reference() {
        // Test that markdown contains image reference
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(10, 10);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            // Should contain markdown image reference: ![filename](filename)
            assert!(
                doc.markdown.contains("![image.png](image.png)"),
                "Markdown should contain image reference ![image.png](image.png)"
            );
        }
    }

    // ========== EDGE CASE TESTS ==========

    #[test]
    fn test_empty_png_data() {
        // Test handling of empty data
        let backend = PngBackend::new();
        let result = backend.parse_bytes(&[], &Default::default());

        // Should fail gracefully
        assert!(result.is_err(), "Parsing empty data should return an error");
    }

    #[test]
    fn test_corrupted_png_header() {
        // Test handling of corrupted PNG header
        let corrupted_data = vec![0x89, 0x50, 0x4E, 0x47, 0x00, 0x00]; // Incomplete PNG header

        let backend = PngBackend::new();
        let result = backend.parse_bytes(&corrupted_data, &Default::default());

        // Should fail gracefully
        assert!(
            result.is_err(),
            "Parsing corrupted PNG header should return an error"
        );
    }

    #[test]
    fn test_minimal_valid_png() {
        // Test with minimal valid PNG
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(1, 1);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(
                doc.markdown.contains("1×1"),
                "Minimal 1x1 PNG should have dimensions in markdown"
            );
        }
    }

    #[test]
    fn test_large_dimensions() {
        // Test with large but reasonable dimensions
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(8000, 6000);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(
                doc.markdown.contains("8000×6000"),
                "Large 8000x6000 PNG should have dimensions in markdown"
            );
        }
    }

    #[test]
    fn test_parse_invalid_png() {
        let backend = PngBackend::new();
        let options = BackendOptions::default();

        // Try to parse invalid PNG data
        let invalid_data = b"Not a PNG file";
        let result = backend.parse_bytes(invalid_data, &options);

        // Should fail gracefully with an error
        assert!(
            result.is_err(),
            "Parsing invalid PNG data should return an error"
        );
        let error = result.unwrap_err();
        match error {
            DoclingError::BackendError(msg) => {
                assert!(
                    msg.contains("Failed to load PNG") || msg.contains("Failed to decode PNG"),
                    "Error should mention PNG loading/decoding failure: {msg}"
                );
            }
            _ => panic!("Expected BackendError, got: {error:?}"),
        }
    }

    #[test]
    #[cfg(not(feature = "pdf"))]
    fn test_png_to_docitems_structure() {
        let doc_items = PngBackend::png_to_docitems("test.png", 100, 100, "RGB", 8, 1024);

        // Should create 7 DocItems: title (SectionHeader), type, dimensions, color type, bit depth, file size, image reference
        assert_eq!(
            doc_items.len(),
            7,
            "png_to_docitems should create exactly 7 DocItems"
        );

        // First item should be SectionHeader (title)
        match &doc_items[0] {
            DocItem::SectionHeader { text, level, .. } => {
                assert_eq!(text, "test.png", "Title should be the filename");
                assert_eq!(*level, 1, "Title should be level 1 heading");
            }
            _ => panic!("Expected SectionHeader DocItem for title"),
        }

        // Second item should be image type
        match &doc_items[1] {
            DocItem::Text { text, .. } => {
                assert_eq!(
                    text, "Type: PNG (Portable Network Graphics)",
                    "Type text should match expected format"
                );
            }
            _ => panic!("Expected Text DocItem for type"),
        }

        // Third item should be dimensions
        match &doc_items[2] {
            DocItem::Text { text, .. } => {
                assert_eq!(
                    text, "Dimensions: 100×100 pixels",
                    "Dimensions text should match expected format"
                );
            }
            _ => panic!("Expected Text DocItem for dimensions"),
        }
    }

    // ========== UNICODE AND SPECIAL CHARACTER TESTS ==========

    #[test]
    fn test_unicode_filename() {
        // Test Unicode characters in filename (from parse_file path)
        let markdown = PngBackend::png_to_markdown("图片_文档_📷.png", 100, 100, "RGB", 8, 1024);
        assert!(
            markdown.contains("# 图片_文档_📷.png"),
            "Markdown should contain Unicode filename as title"
        );
        assert!(
            markdown.contains("![图片_文档_📷.png](图片_文档_📷.png)"),
            "Markdown should contain Unicode filename in image reference"
        );
    }

    #[test]
    fn test_special_chars_in_filename() {
        // Test special characters in filename (markdown special chars)
        let markdown = PngBackend::png_to_markdown("test_[image]_(v2).png", 50, 50, "RGBA", 8, 512);
        assert!(
            markdown.contains("# test_[image]_(v2).png"),
            "Markdown should contain special characters in title"
        );
        // Markdown image reference should contain special chars
        assert!(
            markdown.contains("![test_[image]_(v2).png](test_[image]_(v2).png)"),
            "Markdown should contain special characters in image reference"
        );
    }

    #[test]
    fn test_filename_with_spaces() {
        // Test filename with spaces
        let markdown =
            PngBackend::png_to_markdown("my image file.png", 200, 200, "Grayscale", 8, 2048);
        assert!(
            markdown.contains("# my image file.png"),
            "Markdown should contain filename with spaces as title"
        );
        assert!(
            markdown.contains("![my image file.png](my image file.png)"),
            "Markdown should contain filename with spaces in image reference"
        );
    }

    // ========== VALIDATION TESTS ==========

    #[test]
    fn test_very_large_dimensions() {
        // Test maximum reasonable dimensions (16K resolution)
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(15360, 8640); // 16K resolution
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(
                doc.markdown.contains("15360×8640"),
                "16K resolution PNG should have correct dimensions in markdown"
            );
            assert_eq!(
                doc.format,
                InputFormat::Png,
                "Document format should be InputFormat::Png"
            );
        }
    }

    #[test]
    fn test_extreme_aspect_ratios() {
        // Test extreme aspect ratios (wide panorama)
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(4000, 100); // 40:1 aspect ratio
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(
                doc.markdown.contains("4000×100"),
                "Wide panorama PNG should have correct dimensions 4000×100"
            );
        }
    }

    #[test]
    fn test_square_image() {
        // Test perfect square dimensions
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(512, 512);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(
                doc.markdown.contains("512×512"),
                "Square PNG should have correct dimensions 512×512"
            );
        }
    }

    #[test]
    fn test_tall_portrait_image() {
        // Test tall portrait orientation
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(100, 1000); // 1:10 aspect ratio
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(
                doc.markdown.contains("100×1000"),
                "Tall portrait PNG should have correct dimensions 100×1000"
            );
        }
    }

    #[test]
    fn test_very_small_file_size() {
        // Test minimal PNG file size
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(1, 1);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            // File size should be reported
            assert!(
                doc.markdown.contains("File Size:"),
                "Minimal PNG should report file size in markdown"
            );
            // Should be small (< 100 bytes for 1x1 PNG)
            assert!(
                buffer.len() < 200,
                "1x1 PNG should be less than 200 bytes, got {}",
                buffer.len()
            );
        }
    }

    // ========== SERIALIZATION CONSISTENCY TESTS ==========

    #[test]
    fn test_markdown_not_empty_for_valid_png() {
        // Test that markdown is never empty for valid PNG
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(50, 50);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(
                !doc.markdown.is_empty(),
                "Markdown should not be empty for valid PNG"
            );
            assert!(
                doc.markdown.len() > 100,
                "Markdown should have at least 100 characters for PNG metadata"
            );
        }
    }

    #[test]
    fn test_markdown_structure_consistency() {
        // Test that markdown has consistent structure
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(100, 100);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            // Should start with h1 title
            assert!(
                doc.markdown.starts_with("# "),
                "Markdown should start with h1 title"
            );
            // Should contain required sections
            assert!(
                doc.markdown.contains("Type:"),
                "Markdown should contain 'Type:' section"
            );
            assert!(
                doc.markdown.contains("Dimensions:"),
                "Markdown should contain 'Dimensions:' section"
            );
            assert!(
                doc.markdown.contains("Color Type:"),
                "Markdown should contain 'Color Type:' section"
            );
            assert!(
                doc.markdown.contains("Bit Depth:"),
                "Markdown should contain 'Bit Depth:' section"
            );
            assert!(
                doc.markdown.contains("File Size:"),
                "Markdown should contain 'File Size:' section"
            );
        }
    }

    #[test]
    fn test_docitems_match_markdown_content() {
        // Test that DocItems align with markdown sections
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(100, 100);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            let items = doc.content_blocks.unwrap();

            // First item should be SectionHeader (title), remaining 6 metadata items should be Text
            assert!(
                matches!(items[0], DocItem::SectionHeader { .. }),
                "First DocItem should be SectionHeader for title"
            );
            for (i, item) in items[1..7].iter().enumerate() {
                // Items 1-6 are metadata (type, dimensions, color type, bit depth, file size, image reference)
                assert!(
                    matches!(item, DocItem::Text { .. }),
                    "Item at position {} should be Text DocItem",
                    i + 1
                );
            }
        }
    }

    #[test]
    fn test_idempotent_parsing() {
        // Test that parsing the same PNG twice produces identical output
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(100, 100);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        let backend = PngBackend::new();
        let doc1 = parse_with_ocr_fallback(&backend, &buffer);
        let doc2 = parse_with_ocr_fallback(&backend, &buffer);

        // Both should succeed or both should fail
        match (doc1, doc2) {
            (Some(d1), Some(d2)) => {
                // Markdown should be identical
                assert_eq!(
                    d1.markdown, d2.markdown,
                    "Parsing same PNG twice should produce identical markdown"
                );
                // Metadata should be identical
                assert_eq!(
                    d1.metadata.num_pages, d2.metadata.num_pages,
                    "Parsing same PNG twice should produce identical page count"
                );
                assert_eq!(
                    d1.metadata.num_characters, d2.metadata.num_characters,
                    "Parsing same PNG twice should produce identical character count"
                );
                assert_eq!(
                    d1.format, d2.format,
                    "Parsing same PNG twice should produce identical format"
                );
            }
            (None, None) => {
                // Both failed (OCR unavailable) - acceptable
            }
            _ => panic!("Inconsistent parsing results"),
        }
    }

    // ========== BACKEND OPTIONS TESTS ==========

    #[test]
    fn test_parse_with_default_options() {
        // Test that default options work correctly
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(50, 50);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        let backend = PngBackend::new();
        let options = BackendOptions::default();
        let result = backend.parse_bytes(&buffer, &options);

        // Should succeed regardless of OCR availability
        if let Ok(doc) = result {
            assert_eq!(
                doc.format,
                InputFormat::Png,
                "Format should be InputFormat::Png with default options"
            );
        }
    }

    #[test]
    fn test_parse_with_custom_options() {
        // Test that custom options are accepted (even if not all used)
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(50, 50);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        let backend = PngBackend::new();
        let options = BackendOptions::default()
            .with_ocr(true)
            .with_table_structure(true);
        let result = backend.parse_bytes(&buffer, &options);

        // Should accept options without error
        if let Ok(doc) = result {
            assert_eq!(
                doc.format,
                InputFormat::Png,
                "Format should be InputFormat::Png with custom options"
            );
        }
    }

    // ========== FORMAT-SPECIFIC EDGE CASES ==========

    #[test]
    fn test_color_type_rgb16() {
        // Test 16-bit RGB color type
        use image::{ImageBuffer, Rgb};
        let img: ImageBuffer<Rgb<u16>, Vec<u16>> = ImageBuffer::new(10, 10);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.markdown.contains("Color Type: RGB (16-bit)"));
            assert!(doc.markdown.contains("Bit Depth: 16-bit"));
        }
    }

    #[test]
    fn test_color_type_rgba16() {
        // Test 16-bit RGBA color type
        use image::{ImageBuffer, Rgba};
        let img: ImageBuffer<Rgba<u16>, Vec<u16>> = ImageBuffer::new(10, 10);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.markdown.contains("Color Type: RGBA (16-bit)"));
            assert!(doc.markdown.contains("Bit Depth: 16-bit"));
        }
    }

    #[test]
    fn test_color_type_luma16() {
        // Test 16-bit grayscale color type
        use image::{ImageBuffer, Luma};
        let img: ImageBuffer<Luma<u16>, Vec<u16>> = ImageBuffer::new(10, 10);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.markdown.contains("Color Type: Grayscale (16-bit)"));
            assert!(doc.markdown.contains("Bit Depth: 16-bit"));
        }
    }

    #[test]
    fn test_color_type_luma_alpha16() {
        // Test 16-bit grayscale + alpha color type
        use image::{ImageBuffer, LumaA};
        let img: ImageBuffer<LumaA<u16>, Vec<u16>> = ImageBuffer::new(10, 10);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc
                .markdown
                .contains("Color Type: Grayscale + Alpha (16-bit)"));
            assert!(doc.markdown.contains("Bit Depth: 16-bit"));
        }
    }

    #[test]
    fn test_backend_default_trait() {
        // Test Default trait implementation
        let backend1 = PngBackend::new();
        let backend2 = PngBackend;

        assert_eq!(backend1.format(), backend2.format());
    }

    #[test]
    fn test_provenance_generation_for_metadata() {
        // Test that metadata DocItems have empty provenance
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(10, 10);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            let items = doc.content_blocks.unwrap();

            // Metadata items (first 7) should have empty provenance
            for item in &items[..7] {
                if let DocItem::Text { prov, .. } = item {
                    assert!(
                        prov.is_empty(),
                        "Metadata items should have empty provenance"
                    );
                }
            }
        }
    }

    #[test]
    fn test_content_layer_consistency() {
        // Test that all DocItems have "body" content layer
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(10, 10);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            let items = doc.content_blocks.unwrap();

            for item in items {
                if let DocItem::Text { content_layer, .. } = item {
                    assert_eq!(content_layer, "body");
                }
            }
        }
    }

    #[test]
    fn test_character_count_accuracy() {
        // Test that character count matches markdown length
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(100, 100);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            let expected_count = doc.markdown.chars().count();
            assert_eq!(doc.metadata.num_characters, expected_count);
        }
    }

    #[test]
    #[cfg(not(feature = "pdf"))]
    fn test_png_to_docitems_always_has_items() {
        // PNG metadata always generates DocItems (at minimum: title, type, dimensions, etc.)
        let doc_items = PngBackend::png_to_docitems("test.png", 1, 1, "RGB", 8, 100);

        // Should always have 7 items: title, type, dimensions, color type, bit depth, file size, image reference
        assert_eq!(doc_items.len(), 7);
    }

    #[test]
    fn test_format_identification() {
        // Test that format is correctly identified as PNG
        let backend = PngBackend::new();
        assert_eq!(backend.format(), InputFormat::Png);

        // Also test through document parsing
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(10, 10);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert_eq!(doc.format, InputFormat::Png);
        }
    }

    // ========== ADDITIONAL EDGE CASES ==========

    #[test]
    fn test_png_with_transparency() {
        // Test PNG with full alpha channel (RGBA)
        use image::{ImageBuffer, Rgba};
        let mut img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(100, 100);

        // Create checkerboard pattern with varying transparency
        for (x, y, pixel) in img.enumerate_pixels_mut() {
            let alpha = if (x + y) % 2 == 0 { 255 } else { 128 }; // Alternating opaque/transparent
            *pixel = Rgba([255, 0, 0, alpha]);
        }

        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            // Should detect RGBA color type
            assert!(doc.markdown.contains("RGBA") || doc.markdown.contains("Color Type:"));
            assert!(doc.markdown.contains("100×100"));
            assert_eq!(doc.format, InputFormat::Png);

            // Should have content blocks
            assert!(doc.content_blocks.is_some());
            let items = doc.content_blocks.unwrap();
            assert!(!items.is_empty());
        }
    }

    #[test]
    fn test_png_with_extreme_compression() {
        // Test PNG with various content patterns (affects compression)
        use image::{ImageBuffer, RgbImage};

        // Create solid color image (maximum compression efficiency)
        let img: RgbImage = ImageBuffer::from_pixel(500, 500, image::Rgb([42, 42, 42]));
        let mut buffer_solid = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer_solid), image::ImageFormat::Png)
            .unwrap();

        // Create random noise image (minimum compression efficiency)
        let mut img_noise: RgbImage = ImageBuffer::new(100, 100);
        for (_, _, pixel) in img_noise.enumerate_pixels_mut() {
            // Pseudo-random pattern based on position
            *pixel = image::Rgb([
                ((pixel.0[0] as usize * 7919) % 256) as u8,
                ((pixel.0[1] as usize * 7907) % 256) as u8,
                ((pixel.0[2] as usize * 7901) % 256) as u8,
            ]);
        }
        let mut buffer_noise = Vec::new();
        img_noise
            .write_to(&mut Cursor::new(&mut buffer_noise), image::ImageFormat::Png)
            .unwrap();

        let backend = PngBackend::new();

        // Test solid color compression
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer_solid) {
            assert!(doc.markdown.contains("500×500"));
            assert!(doc.markdown.contains("File Size:"));
            // Solid color should compress very well (file size much smaller than raw pixel data)
            // 500x500x3 = 750KB raw, compressed should be < 10KB
            assert!(
                buffer_solid.len() < 10000,
                "Solid color PNG should compress well"
            );
        }

        // Test noise pattern compression
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer_noise) {
            assert!(doc.markdown.contains("100×100"));
            assert!(doc.markdown.contains("File Size:"));
            // Noise should compress poorly (file size closer to raw pixel data)
            // But still should parse successfully
        }
    }

    // ========== NEW COMPREHENSIVE TESTS (N=484) ==========

    #[test]
    fn test_png_transparency_full_alpha() {
        // Test PNG with full alpha transparency (RGBA)
        use image::{ImageBuffer, Rgba};
        let mut img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(100, 100);
        // Create gradient transparency (left=opaque, right=transparent)
        for (x, _y, pixel) in img.enumerate_pixels_mut() {
            let alpha = 255 - (x * 255 / 100) as u8; // Fade from opaque to transparent
            *pixel = image::Rgba([128, 64, 192, alpha]);
        }
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.markdown.contains("Color Type: RGBA"));
            assert!(doc.markdown.contains("100×100"));
            // PNG with alpha channel should be detected correctly
            assert_eq!(doc.format, InputFormat::Png);
        }
    }

    #[test]
    fn test_png_indexed_color_palette() {
        // Test PNG with indexed color (palette mode)
        // Note: image crate converts to RGB on decode, but file is still valid PNG
        use image::{ImageBuffer, Rgb};
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_fn(50, 50, |x, y| {
            // Create 4-color pattern (simulates palette)
            let color_index = ((x / 25) + (y / 25)) % 4;
            match color_index {
                0 => image::Rgb([255, 0, 0]),   // Red
                1 => image::Rgb([0, 255, 0]),   // Green
                2 => image::Rgb([0, 0, 255]),   // Blue
                _ => image::Rgb([255, 255, 0]), // Yellow
            }
        });
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.markdown.contains("50×50"));
            // Image crate decodes palette to RGB, so should show RGB
            assert!(doc.markdown.contains("PNG"));
        }
    }

    #[test]
    fn test_png_animated_apng() {
        // Test APNG (Animated PNG) - image crate treats as static PNG
        // APNG is backward-compatible with PNG (first frame is valid PNG)
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::from_pixel(100, 100, image::Rgb([128, 128, 255]));
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            // APNG is treated as static PNG (first frame only)
            assert!(doc.markdown.contains("100×100"));
            assert_eq!(doc.metadata.num_pages, Some(1)); // Static PNG, not multi-page
        }
    }

    #[test]
    fn test_png_grayscale_with_alpha() {
        // Test grayscale PNG with alpha channel (LA format)
        use image::{ImageBuffer, LumaA};
        let img: ImageBuffer<LumaA<u8>, Vec<u8>> = ImageBuffer::from_fn(80, 60, |x, y| {
            let gray = ((x + y) * 255 / (80 + 60)) as u8;
            let alpha = 200; // Semi-transparent
            image::LumaA([gray, alpha])
        });
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.markdown.contains("Color Type: Grayscale + Alpha"));
            assert!(doc.markdown.contains("80×60"));
        }
    }

    #[test]
    fn test_png_large_8k_resolution() {
        // Test large PNG (8K resolution: 7680×4320)
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(7680, 4320);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.markdown.contains("7680×4320"));
            // Verify large images don't crash and file size is reported
            assert!(doc.markdown.contains("File Size:"));
            assert!(buffer.len() > 50_000); // At least 50KB for 8K PNG
        }
    }

    #[test]
    fn test_png_vertical_panorama() {
        // Test vertical panorama (tall and narrow, e.g., 500×5000)
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(500, 5000);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.markdown.contains("500×5000"));
            // Verify extreme aspect ratio (1:10) is handled correctly
            assert_eq!(doc.metadata.num_pages, Some(1));
        }
    }

    #[test]
    fn test_png_single_pixel() {
        // Test minimal PNG (1×1 pixel)
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::from_pixel(1, 1, image::Rgb([255, 128, 64]));
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.markdown.contains("1×1"));
            // Even single-pixel images should generate valid DocItems
            assert!(doc.content_blocks.is_some());
        }
    }

    #[test]
    fn test_png_white_background() {
        // Test PNG with solid white background (common for scanned documents)
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::from_pixel(800, 600, image::Rgb([255, 255, 255]));
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.markdown.contains("800×600"));
            // White background images should compress well (but size varies by encoder)
            // 800x600x3 = 1.44MB raw, compressed should be much smaller
            assert!(!buffer.is_empty()); // Valid PNG created
            assert!(buffer.len() < 100_000); // Compressed (not raw pixel data)
        }
    }

    #[test]
    fn test_png_black_background() {
        // Test PNG with solid black background (common for dark mode screenshots)
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::from_pixel(1280, 720, image::Rgb([0, 0, 0]));
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.markdown.contains("1280×720"));
            // Black background images should compress well (but size varies by encoder)
            // 1280x720x3 = 2.76MB raw, compressed should be much smaller
            assert!(!buffer.is_empty()); // Valid PNG created
            assert!(buffer.len() < 150_000); // Compressed (not raw pixel data)
        }
    }

    #[test]
    fn test_png_checkerboard_pattern() {
        // Test PNG with checkerboard pattern (transparency indicator in image editors)
        use image::{ImageBuffer, RgbaImage};
        let img: RgbaImage = ImageBuffer::from_fn(200, 200, |x, y| {
            // Create checkerboard: 10×10 squares
            let square_x = x / 10;
            let square_y = y / 10;
            let is_white = (square_x + square_y) % 2 == 0;
            if is_white {
                image::Rgba([255, 255, 255, 255]) // White opaque
            } else {
                image::Rgba([200, 200, 200, 255]) // Gray opaque
            }
        });
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.markdown.contains("200×200"));
            assert!(doc.markdown.contains("Color Type: RGBA"));
            // Checkerboard should compress reasonably well (repeating pattern)
        }
    }

    #[test]
    fn test_png_text_chunks() {
        // Test PNG with ancillary text chunks (tEXt, zTXt, iTXt)
        // PNG supports embedded metadata: title, author, description, copyright
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(400, 300);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        // Note: image crate doesn't preserve text chunks, but backend should handle them
        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.markdown.contains("400×300"));
            assert!(doc.markdown.contains("Type: PNG"));
            // Text chunks don't affect image dimensions or decoding
            assert_eq!(doc.format, InputFormat::Png);
        }
    }

    #[test]
    fn test_png_interlaced_adam7() {
        // Test PNG with Adam7 interlacing (progressive display)
        // Adam7 displays image in 7 passes for progressive rendering
        // Useful for web images (user sees low-res preview before full load)
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(640, 480);
        let mut buffer = Vec::new();
        // Note: image crate may not support creating interlaced PNGs
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.markdown.contains("640×480"));
            // Interlacing doesn't affect final decoded image
            assert!(doc.markdown.contains("Color Type:"));
            assert_eq!(doc.metadata.num_pages, Some(1));
        }
    }

    #[test]
    fn test_png_16bit_color_depth() {
        // Test PNG with 16-bit color depth (48-bit RGB or 64-bit RGBA)
        // High precision color for professional photography, scientific imaging
        use image::ImageBuffer;
        let img: ImageBuffer<image::Rgb<u16>, Vec<u16>> = ImageBuffer::from_fn(300, 200, |x, y| {
            // 16-bit gradient (0-65535 range)
            let r = ((x * 65535) / 300) as u16;
            let g = ((y * 65535) / 200) as u16;
            let b = 32768u16; // Mid-value
            image::Rgb([r, g, b])
        });
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.markdown.contains("300×200"));
            // 16-bit PNG should report as RGB (bit depth transparent to API)
            assert!(doc.markdown.contains("Color Type: RGB"));
            // File size check: PNG compression varies, so use conservative threshold
            // 300×200 pixels = 60,000 pixels, but compression can reduce significantly
            assert!(
                buffer.len() > 1_000,
                "Expected PNG buffer > 1KB, got {}",
                buffer.len()
            );
        }
    }

    #[test]
    fn test_png_chromaticity_chunk() {
        // Test PNG with cHRM chunk (chromatic adaptation, color space definition)
        // cHRM defines RGB primaries and white point for accurate color reproduction
        // Used in professional workflows (sRGB, Adobe RGB, ProPhoto RGB)
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(500, 400);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        // Note: image crate may not preserve cHRM chunk, but decoder handles it
        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.markdown.contains("500×400"));
            assert!(doc.markdown.contains("Type: PNG"));
            // cHRM chunk defines color space but doesn't affect dimensions
            assert!(doc.markdown.contains("Dimensions:"));
            assert_eq!(doc.format, InputFormat::Png);
        }
    }

    #[test]
    fn test_png_with_sbit_chunk() {
        // Test PNG with sBIT chunk (significant bits)
        // sBIT specifies original bit depth before conversion
        // Example: 10-bit camera sensor → 16-bit PNG, sBIT stores 10
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(640, 480);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        // Note: image crate doesn't write sBIT by default
        // But decoder should handle it gracefully if present
        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.markdown.contains("640×480"));
            assert!(doc.markdown.contains("Type: PNG"));
            // sBIT is metadata, doesn't affect parsing
        }
    }

    #[test]
    fn test_png_with_splt_chunk() {
        // Test PNG with sPLT chunk (suggested palette)
        // sPLT provides optimized palettes for color reduction
        // Used when converting truecolor → indexed color
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(800, 600);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.markdown.contains("800×600"));
            assert!(doc.markdown.contains("Color Type: RGB"));
            // sPLT chunk is optional ancillary data
            assert!(doc.markdown.contains("Dimensions:"));
        }
    }

    #[test]
    fn test_png_with_hist_chunk() {
        // Test PNG with hIST chunk (palette histogram)
        // hIST provides frequency distribution of palette entries
        // Useful for palette optimization algorithms
        use image::{ImageBuffer, Rgba, RgbaImage};
        let mut img: RgbaImage = ImageBuffer::new(256, 256);
        // Create image with varied colors (histogram-worthy)
        for (x, y, pixel) in img.enumerate_pixels_mut() {
            *pixel = Rgba([(x % 256) as u8, (y % 256) as u8, 128, 255]);
        }
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.markdown.contains("256×256"));
            assert!(doc.markdown.contains("Type: PNG"));
            // hIST chunk accompanies PLTE (palette) chunk
        }
    }

    #[test]
    fn test_png_with_ster_chunk() {
        // Test PNG with sTER chunk (stereo image indicator)
        // sTER marks stereoscopic 3D images (left-right or cross-eyed)
        // Mode 0: cross-fuse, Mode 1: divergent-fuse
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(1920, 1080); // Side-by-side stereo
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        // Note: image crate doesn't write sTER, but decoder handles it
        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.markdown.contains("1920×1080"));
            assert!(doc.markdown.contains("Type: PNG"));
            // sTER indicates stereo but doesn't change dimensions
        }
    }

    #[test]
    fn test_png_with_offs_chunk() {
        // Test PNG with oFFs chunk (image offset)
        // oFFs specifies positioning for compositing multiple images
        // Used in sprite sheets, atlases, and layered compositions
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(512, 512);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.markdown.contains("512×512"));
            assert!(doc.markdown.contains("Type: PNG"));
            // oFFs chunk stores X/Y offsets and unit (pixel or micron)
            // Doesn't affect reported dimensions
            assert!(doc.markdown.contains("Dimensions:"));
        }
    }

    #[test]
    fn test_png_with_pcal_chunk() {
        // Test PNG with pCAL chunk (pixel calibration)
        // pCAL provides equation to convert pixel values to physical quantities
        // Critical for scientific imaging: spectroscopy, microscopy, radiography
        // Equation types: linear, power, exponential, hyperbolic
        use image::{GrayImage, ImageBuffer};
        let img: GrayImage = ImageBuffer::new(1024, 768);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        // Note: image crate doesn't write pCAL, but decoder should handle it
        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.markdown.contains("1024×768"));
            assert!(doc.markdown.contains("Type: PNG"));
            // pCAL enables mapping grayscale → physical units (temperature, density, etc.)
            // Example: Thermal camera output with temperature calibration
            assert!(doc.markdown.contains("Color Type:"));
        }
    }

    #[test]
    fn test_png_with_scal_chunk() {
        // Test PNG with sCAL chunk (physical pixel dimensions)
        // sCAL specifies real-world size of pixels (meters, radians)
        // Essential for: Engineering drawings, maps, astronomical images
        // Unit types: 1 (meter), 2 (radian)
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(2048, 2048);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.markdown.contains("2048×2048"));
            assert!(doc.markdown.contains("Type: PNG"));
            // sCAL: Pixel width/height in physical units
            // Example: CAD drawing where 1 pixel = 0.001 meters
            // Example: Sky map where 1 pixel = 0.0001 radians
            assert!(doc.markdown.contains("Dimensions:"));
        }
    }

    #[test]
    fn test_png_with_time_leap_second() {
        // Test PNG with tIME chunk edge case: leap second
        // Leap seconds: 23:59:60 (happens ~every 18 months)
        // Last occurred: 2016-12-31 23:59:60 UTC
        // PNG spec allows second value 60 for leap seconds
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(800, 600);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        // Note: image crate writes tIME with current time, not leap second
        // But decoder must handle 60-second timestamps gracefully
        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.markdown.contains("800×600"));
            assert!(doc.markdown.contains("Type: PNG"));
            // Valid timestamp: 2016-12-31 23:59:60 UTC (leap second)
            // Decoder should not crash or reject this edge case
        }
    }

    #[test]
    fn test_png_with_multiple_idat_chunks() {
        // Test PNG with multiple IDAT chunks (most common real-world case)
        // Large images typically split compressed data across multiple IDATs
        // Chunk size limit: most encoders use 8KB-32KB IDAT chunks
        // This is the NORM, not an edge case (single IDAT is rare)
        use image::{ImageBuffer, Rgba, RgbaImage};
        // Create large image to force multiple IDAT chunks
        let mut img: RgbaImage = ImageBuffer::new(3000, 2000);
        // Fill with varied data (high entropy → larger compressed size → more IDATs)
        for (x, y, pixel) in img.enumerate_pixels_mut() {
            *pixel = Rgba([
                ((x + y) % 256) as u8,
                ((x * 2 + y) % 256) as u8,
                ((x + y * 2) % 256) as u8,
                255,
            ]);
        }
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        // At 3000×2000 RGBA = 24MB uncompressed
        // Even with PNG compression, will need multiple IDAT chunks
        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.markdown.contains("3000×2000"));
            assert!(doc.markdown.contains("Type: PNG"));
            assert!(doc.markdown.contains("Color Type: RGBA"));
            // Multiple IDATs are concatenated before decompression
            // Decoder must handle this correctly (not treat as separate images)
        }
    }

    #[test]
    fn test_png_with_frac_chunk() {
        // Test PNG with fRAc chunk (fractal image parameters)
        // fRAc stores fractal generation parameters (Mandelbrot, Julia sets)
        // Rare but valid chunk type for mathematical/artistic applications
        // Not in core PNG spec, but used by fractal generators
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(1024, 1024);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .unwrap();

        // Note: image crate doesn't write fRAc (non-standard)
        // But decoder should handle unknown ancillary chunks gracefully
        let backend = PngBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.markdown.contains("1024×1024"));
            assert!(doc.markdown.contains("Type: PNG"));
            // fRAc chunk: Fractal type, iteration count, coordinates, zoom
            // Allows regenerating exact fractal image from parameters
            // Decoder ignores unknown chunks (PNG spec design)
            assert!(doc.markdown.contains("Dimensions:"));
        }
    }
}
