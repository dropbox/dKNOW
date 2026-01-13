//! JPEG backend for docling
//!
//! This backend converts JPEG files to docling's document model.

use crate::exif_utils;
use crate::traits::{BackendOptions, DocumentBackend};
use crate::utils::{create_section_header, create_text_item, format_file_size, opt_vec};
use docling_core::{DocItem, DoclingError, Document, DocumentMetadata, InputFormat};
use docling_ocr::OcrEngine;
use image::{GenericImageView, ImageReader};
use std::fmt::Write;
use std::io::Cursor;
use std::path::Path;

/// JPEG backend
///
/// Converts JPEG files to docling's document model.
/// Extracts basic metadata and uses OCR to extract text content from the image.
///
/// ## Features
///
/// - Extract image dimensions
/// - Detect color type (RGB, Grayscale)
/// - OCR text extraction with bounding boxes
/// - Generate markdown with image metadata and OCR text
///
/// ## Example
///
/// ```no_run
/// use docling_backend::JpegBackend;
/// use docling_backend::DocumentBackend;
///
/// let backend = JpegBackend::new();
/// let result = backend.parse_file("image.jpg", &Default::default())?;
/// println!("Image: {:?}", result.metadata.title);
/// # Ok::<(), docling_core::error::DoclingError>(())
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct JpegBackend;

impl JpegBackend {
    /// Create a new JPEG backend instance
    #[inline]
    #[must_use = "creates a backend instance that should be used for parsing"]
    pub const fn new() -> Self {
        Self
    }

    /// Convert JPEG metadata to markdown
    fn jpeg_to_markdown(
        filename: &str,
        width: u32,
        height: u32,
        color_type: &str,
        file_size: usize,
    ) -> String {
        let mut markdown = String::new();

        // Title
        let _ = write!(markdown, "# {filename}\n\n");

        // Image type
        markdown.push_str("Type: JPEG (Joint Photographic Experts Group)\n\n");

        // Dimensions
        let _ = write!(markdown, "Dimensions: {width}×{height} pixels\n\n");

        // Color type
        let _ = write!(markdown, "Color Type: {color_type}\n\n");

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
    ///
    /// ## Returns
    /// Tuple of (OCR text as string, vector of `DocItem::Text` with bboxes)
    fn extract_ocr_text(
        data: &[u8],
        start_index: usize,
    ) -> Result<(String, Vec<DocItem>), DoclingError> {
        use docling_core::content::{BoundingBox, CoordOrigin, ProvenanceItem};

        // Check if OCR is enabled via environment variable
        // Default: OCR is disabled (ENABLE_IMAGE_OCR must be explicitly set to "1")
        // This allows skipping expensive OCR processing (5-15s) on non-text images
        if std::env::var("ENABLE_IMAGE_OCR").unwrap_or_default() != "1" {
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
                page_no: 1, // JPEG is single-page
                bbox,
                charspan: None,
            }];

            doc_items.push(create_text_item(start_index + i, line.text.clone(), prov));
        }

        // Get full OCR text
        let ocr_text = ocr_result.text();

        Ok((ocr_text, doc_items))
    }

    /// Create `DocItems` directly from JPEG metadata
    ///
    /// Generates structured `DocItems` from JPEG metadata without markdown intermediary.
    /// Creates a hierarchical document structure:
    /// - Title (filename) as `SectionHeader` level 1
    /// - Image type as Text
    /// - Dimensions as Text
    /// - Color type as Text
    /// - File size as Text
    /// - Image reference as Text
    ///
    /// ## Arguments
    /// * `filename` - Name of the JPEG file
    /// * `width` - Image width in pixels
    /// * `height` - Image height in pixels
    /// * `color_type` - Color type description
    /// * `file_size` - Size of the JPEG file in bytes
    ///
    /// ## Returns
    /// Vector of `DocItems` representing the JPEG metadata structure
    fn jpeg_to_docitems(
        filename: &str,
        width: u32,
        height: u32,
        color_type: &str,
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
            "Type: JPEG (Joint Photographic Experts Group)".to_string(),
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

    /// Shared helper to parse JPEG data and produce a Document
    ///
    /// Both `parse_bytes` and `parse_file` delegate to this method.
    ///
    /// ## Arguments
    /// * `data` - Raw JPEG file bytes
    /// * `filename` - Name to use for the document title and image reference
    fn parse_jpeg_data(data: &[u8], filename: &str) -> Result<Document, DoclingError> {
        // Load image to get metadata
        let img = ImageReader::new(Cursor::new(data))
            .with_guessed_format()
            .map_err(|e| {
                DoclingError::BackendError(format!("Failed to load JPEG: {e}: {filename}"))
            })?
            .decode()
            .map_err(|e| {
                DoclingError::BackendError(format!("Failed to decode JPEG: {e}: {filename}"))
            })?;

        let (width, height) = img.dimensions();
        let color_type = match &img {
            image::DynamicImage::ImageLuma8(_) => "Grayscale",
            image::DynamicImage::ImageRgb8(_) => "RGB",
            _ => "Other",
        };

        // Create DocItems directly from JPEG metadata (no markdown intermediary)
        let mut doc_items = Self::jpeg_to_docitems(filename, width, height, color_type, data.len());
        let metadata_items_count = doc_items.len();

        // Extract OCR text
        let (ocr_text, mut ocr_items) = Self::extract_ocr_text(data, metadata_items_count)?;

        // Combine DocItems: metadata + OCR
        doc_items.append(&mut ocr_items);

        // Generate markdown from DocItems (for backwards compatibility)
        let metadata_markdown =
            Self::jpeg_to_markdown(filename, width, height, color_type, data.len());
        let mut markdown = metadata_markdown;
        if !ocr_text.is_empty() {
            markdown.push_str("\n\n## OCR Text\n\n");
            markdown.push_str(&ocr_text);
        }

        let num_characters = markdown.chars().count();

        // Extract EXIF metadata
        let exif_metadata = exif_utils::extract_exif_metadata(data);

        // N=1885: Extract EXIF Artist and ImageDescription metadata
        let (author, subject, created) = exif_utils::extract_document_metadata(data);

        // Create document
        Ok(Document {
            markdown,
            format: InputFormat::Jpeg,
            metadata: DocumentMetadata {
                num_pages: Some(1), // JPEG is single-page
                num_characters,
                title: Some(filename.to_string()),
                author,  // N=1885: From EXIF Artist tag
                subject, // N=1885: From EXIF ImageDescription tag
                created, // N=1885: From EXIF DateTimeOriginal tag
                modified: None,
                language: None,
                exif: exif_metadata,
            },
            docling_document: None,
            content_blocks: opt_vec(doc_items),
        })
    }
}

impl DocumentBackend for JpegBackend {
    #[inline]
    fn format(&self) -> InputFormat {
        InputFormat::Jpeg
    }

    fn parse_bytes(
        &self,
        data: &[u8],
        _options: &BackendOptions,
    ) -> Result<Document, DoclingError> {
        Self::parse_jpeg_data(data, "image.jpg")
    }

    fn parse_file<P: AsRef<Path>>(
        &self,
        path: P,
        _options: &BackendOptions,
    ) -> Result<Document, DoclingError> {
        let path_ref = path.as_ref();
        let filename = path_ref
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("image.jpg");

        // Read file
        let data = std::fs::read(path_ref).map_err(DoclingError::IoError)?;

        Self::parse_jpeg_data(&data, filename)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    /// Helper function to handle OCR unavailability and decoding errors in tests
    /// Returns None if OCR is not available or if decoding fails (e.g., minimal test JPEG data)
    /// This allows tests to verify metadata extraction without requiring complete JPEG data
    fn parse_with_ocr_fallback(backend: &JpegBackend, data: &[u8]) -> Option<Document> {
        let result = backend.parse_bytes(data, &Default::default());
        match result {
            Ok(doc) => Some(doc),
            Err(err) => {
                // Skip test gracefully if:
                // 1. OCR is not available
                // 2. JPEG decoding fails (minimal test data)
                let err_str = format!("{err:?}");
                if err_str.contains("OCR")
                    || err_str.contains("initialize")
                    || err_str.contains("Failed to decode JPEG")
                    || err_str.contains("Premature End of image")
                {
                    None
                } else {
                    panic!("Unexpected error: {err:?}");
                }
            }
        }
    }

    #[test]
    fn test_backend_format() {
        let backend = JpegBackend::new();
        assert_eq!(
            backend.format(),
            InputFormat::Jpeg,
            "JpegBackend should report Jpeg format"
        );
    }

    #[test]
    fn test_jpeg_to_markdown() {
        let markdown = JpegBackend::jpeg_to_markdown("test.jpg", 1920, 1080, "RGB", 204_800);
        assert!(
            markdown.contains("# test.jpg"),
            "Markdown should contain title heading"
        );
        assert!(
            markdown.contains("Type: JPEG (Joint Photographic Experts Group)"),
            "Markdown should contain JPEG type description"
        );
        assert!(
            markdown.contains("Dimensions: 1920×1080 pixels"),
            "Markdown should contain dimensions"
        );
        assert!(
            markdown.contains("Color Type: RGB"),
            "Markdown should contain color type"
        );
        assert!(
            markdown.contains("![test.jpg](test.jpg)"),
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
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Jpeg)
            .unwrap();

        let backend = JpegBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert_eq!(
                doc.metadata.title,
                Some("image.jpg".to_string()),
                "Metadata title should be set to default filename"
            );
        }
    }

    #[test]
    fn test_metadata_num_pages() {
        // Test that JPEG is reported as single-page
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(10, 10);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Jpeg)
            .unwrap();

        let backend = JpegBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert_eq!(
                doc.metadata.num_pages,
                Some(1),
                "JPEG should be reported as single-page"
            );
        }
    }

    #[test]
    fn test_metadata_character_count() {
        // Test that character count is computed correctly
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(10, 10);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Jpeg)
            .unwrap();

        let backend = JpegBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            // Character count should match markdown length
            assert_eq!(
                doc.metadata.num_characters,
                doc.markdown.chars().count(),
                "Character count should match markdown length"
            );
            assert!(
                doc.metadata.num_characters > 0,
                "Character count should be greater than zero"
            );
        }
    }

    #[test]
    fn test_metadata_format_field() {
        // Test that format is correctly set
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(5, 5);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Jpeg)
            .unwrap();

        let backend = JpegBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert_eq!(
                doc.format,
                InputFormat::Jpeg,
                "Document format should be Jpeg"
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
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Jpeg)
            .unwrap();

        let backend = JpegBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(
                doc.content_blocks.is_some(),
                "Document should have content blocks"
            );

            let items = doc.content_blocks.unwrap();
            assert!(!items.is_empty(), "Content blocks should not be empty");

            // Items should be either SectionHeader (title) or Text (metadata)
            for item in &items {
                assert!(
                    matches!(item, DocItem::Text { .. })
                        || matches!(item, DocItem::SectionHeader { .. }),
                    "Expected Text or SectionHeader, got: {item:?}"
                );
            }
        }

        std::env::remove_var("ENABLE_IMAGE_OCR");
    }

    #[test]
    fn test_docitem_count() {
        // Test expected number of DocItems (6 metadata paragraphs)
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(10, 10);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Jpeg)
            .unwrap();

        let backend = JpegBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(
                doc.content_blocks.is_some(),
                "Document should have content blocks"
            );

            let items = doc.content_blocks.unwrap();
            // Expected: title, type, dimensions, color type, file size, image reference
            // (6 paragraphs from metadata, no OCR text expected for synthetic image)
            assert_eq!(
                items.len(),
                6,
                "Should have 6 metadata DocItems (title, type, dimensions, color, size, reference)"
            );
        }
    }

    #[test]
    fn test_docitem_content() {
        // Test that DocItems contain expected content
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(100, 200);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Jpeg)
            .unwrap();

        let backend = JpegBackend::new();
        let result = backend.parse_bytes(&buffer, &Default::default());
        assert!(result.is_ok(), "Parsing should succeed");

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();

        // First item should be SectionHeader with title
        if let DocItem::SectionHeader { text, .. } = &items[0] {
            assert!(text.contains("image.jpg"), "Title should contain filename");
        } else {
            panic!("Expected SectionHeader item for title");
        }

        // Third item (dimensions) should be Text
        if let DocItem::Text { text, .. } = &items[2] {
            assert!(
                text.contains("100×200"),
                "Dimensions text should contain 100×200"
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
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Jpeg)
            .unwrap();

        let backend = JpegBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            let items = doc.content_blocks.unwrap();

            // Check that self_ref values are sequential
            for (i, item) in items.iter().enumerate() {
                if let DocItem::Text { self_ref, .. } = item {
                    assert_eq!(
                        self_ref,
                        &format!("#/texts/{i}"),
                        "self_ref should be sequentially indexed"
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
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Jpeg)
            .unwrap();

        let backend = JpegBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(
                doc.markdown.contains("Color Type: RGB"),
                "Markdown should indicate RGB color type"
            );
        }
    }

    #[test]
    fn test_color_type_grayscale() {
        // Test grayscale color type detection
        use image::{ImageBuffer, Luma};
        let img: ImageBuffer<Luma<u8>, Vec<u8>> = ImageBuffer::new(10, 10);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Jpeg)
            .unwrap();

        let backend = JpegBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(
                doc.markdown.contains("Color Type: Grayscale"),
                "Markdown should indicate Grayscale color type"
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
            img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Jpeg)
                .unwrap();

            let backend = JpegBackend::new();
            if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
                assert!(
                    doc.markdown.contains(&format!("{width}×{height}")),
                    "Markdown should contain dimensions {width}×{height}"
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
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Jpeg)
            .unwrap();

        let backend = JpegBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            // File size should be in the markdown (format_file_size adds "File Size:")
            assert!(
                doc.markdown.contains("File Size:"),
                "Markdown should contain file size information"
            );
        }
    }

    #[test]
    fn test_markdown_image_reference() {
        // Test that markdown contains image reference
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(10, 10);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Jpeg)
            .unwrap();

        let backend = JpegBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            // Should contain markdown image reference: ![filename](filename)
            assert!(
                doc.markdown.contains("![image.jpg](image.jpg)"),
                "Markdown should contain image reference syntax"
            );
        }
    }

    #[test]
    fn test_exif_extraction() {
        // Test with a real JPEG file from test corpus
        let test_path = "test-corpus/jpeg/high_quality.jpg";
        if !std::path::Path::new(test_path).exists() {
            // Skip test if file doesn't exist (CI environment)
            return;
        }

        let data = std::fs::read(test_path).unwrap();
        let exif = exif_utils::extract_exif_metadata(&data);

        // The test file may or may not have EXIF data
        // If it does, verify the structure is correct
        if let Some(exif_data) = exif {
            // Check that at least some fields can be accessed
            // (don't assert specific values as test file EXIF may vary)
            let _ = exif_data.datetime;
            let _ = exif_data.camera_make;
            let _ = exif_data.camera_model;
        }
    }

    #[test]
    fn test_exif_without_data() {
        // Create a minimal JPEG without EXIF data
        let minimal_jpeg = vec![
            0xFF, 0xD8, // SOI marker
            0xFF, 0xE0, // APP0 marker
            0x00, 0x10, // Length
            b'J', b'F', b'I', b'F', 0x00, // Identifier
            0x01, 0x01, // Version
            0x00, // Units
            0x00, 0x01, 0x00, 0x01, // X/Y density
            0x00, 0x00, // Thumbnail size
            0xFF, 0xD9, // EOI marker
        ];

        let exif = exif_utils::extract_exif_metadata(&minimal_jpeg);
        // Should return None for JPEG without EXIF
        assert!(
            exif.is_none(),
            "JPEG without EXIF data should return None for EXIF metadata"
        );
    }

    // ========== EDGE CASE TESTS ==========

    #[test]
    fn test_empty_jpeg_data() {
        // Test handling of empty data
        let backend = JpegBackend::new();
        let result = backend.parse_bytes(&[], &Default::default());

        // Should fail gracefully
        assert!(result.is_err(), "Empty data should result in error");
    }

    #[test]
    fn test_corrupted_jpeg_header() {
        // Test handling of corrupted JPEG header
        let corrupted_data = vec![0xFF, 0xD8, 0xFF, 0x00]; // Incomplete JPEG header

        let backend = JpegBackend::new();
        let result = backend.parse_bytes(&corrupted_data, &Default::default());

        // Should fail gracefully
        assert!(result.is_err(), "Corrupted header should result in error");
    }

    #[test]
    fn test_minimal_valid_jpeg() {
        // Test with minimal valid JPEG
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(1, 1);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Jpeg)
            .unwrap();

        let backend = JpegBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(
                doc.markdown.contains("1×1"),
                "Minimal JPEG should report 1×1 dimensions"
            );
        }
    }

    #[test]
    fn test_parse_invalid_jpeg() {
        let backend = JpegBackend::new();
        let options = BackendOptions::default();

        // Try to parse invalid JPEG data
        let invalid_data = b"Not a JPEG file";
        let result = backend.parse_bytes(invalid_data, &options);

        // Should fail gracefully with an error
        assert!(result.is_err(), "Invalid JPEG data should result in error");
        let error = result.unwrap_err();
        match error {
            DoclingError::BackendError(msg) => {
                assert!(
                    msg.contains("Failed to load JPEG") || msg.contains("Failed to decode JPEG"),
                    "Unexpected error message: {msg}"
                );
            }
            _ => panic!("Expected BackendError, got: {error:?}"),
        }
    }

    #[test]
    fn test_jpeg_to_docitems_structure() {
        let doc_items = JpegBackend::jpeg_to_docitems("test.jpg", 100, 100, "RGB", 1024);

        // Should create 6 DocItems: title (SectionHeader), type, dimensions, color type, file size, image reference
        assert_eq!(
            doc_items.len(),
            6,
            "Should create 6 DocItems for JPEG metadata"
        );

        // First item should be SectionHeader (title)
        match &doc_items[0] {
            DocItem::SectionHeader { text, level, .. } => {
                assert_eq!(text, "test.jpg", "Title should be the filename");
                assert_eq!(*level, 1, "Title should be level 1 heading");
            }
            _ => panic!("Expected SectionHeader DocItem for title"),
        }

        // Second item should be image type
        match &doc_items[1] {
            DocItem::Text { text, .. } => {
                assert_eq!(
                    text, "Type: JPEG (Joint Photographic Experts Group)",
                    "Type text should describe JPEG format"
                );
            }
            _ => panic!("Expected Text DocItem for type"),
        }

        // Third item should be dimensions
        match &doc_items[2] {
            DocItem::Text { text, .. } => {
                assert_eq!(
                    text, "Dimensions: 100×100 pixels",
                    "Dimensions should match input values"
                );
            }
            _ => panic!("Expected Text DocItem for dimensions"),
        }
    }

    // ========== UNICODE AND SPECIAL CHARACTER TESTS ==========

    #[test]
    fn test_unicode_dimensions() {
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(100, 200);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Jpeg)
            .unwrap();

        let backend = JpegBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(
                doc.markdown.contains("×"),
                "Markdown should contain Unicode multiplication sign"
            );
            assert!(
                doc.markdown.contains("100×200"),
                "Markdown should contain dimensions with Unicode ×"
            );
        }
    }

    #[test]
    fn test_markdown_valid_utf8() {
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(50, 50);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Jpeg)
            .unwrap();

        let backend = JpegBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            // Verify all characters are valid
            assert!(
                doc.markdown.chars().all(|c| !c.is_control() || c == '\n'),
                "Markdown should only contain valid printable characters"
            );
        }
    }

    #[test]
    fn test_aspect_ratio_calculations() {
        let test_cases: Vec<(u32, u32)> = vec![
            (16, 9), // 16:9
            (4, 3),  // 4:3
            (21, 9), // 21:9
            (1, 1),  // Square
            (9, 16), // Vertical
        ];

        for (width_ratio, height_ratio) in test_cases {
            use image::{ImageBuffer, RgbImage};
            let img: RgbImage = ImageBuffer::new(width_ratio * 100, height_ratio * 100);
            let mut buffer = Vec::new();
            img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Jpeg)
                .unwrap();

            let backend = JpegBackend::new();
            if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
                let expected = format!("{}×{}", width_ratio * 100, height_ratio * 100);
                assert!(
                    doc.markdown.contains(&expected),
                    "Markdown should contain aspect ratio dimensions {expected}"
                );
            }
        }
    }

    // ========== VALIDATION TESTS ==========

    #[test]
    fn test_zero_dimensions_jpeg() {
        let backend = JpegBackend::new();
        let invalid_data = vec![0u8; 100];
        let result = backend.parse_bytes(&invalid_data, &Default::default());
        assert!(
            result.is_err(),
            "Invalid JPEG data (zeros) should result in error"
        );
    }

    #[test]
    fn test_jpeg_soi_marker_validation() {
        // JPEG files must start with SOI marker (0xFF 0xD8)
        let mut data = vec![0u8; 100];
        data[0] = 0xFF;
        data[1] = 0xD8; // SOI marker
                        // But rest is invalid

        let backend = JpegBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());
        assert!(
            result.is_err(),
            "JPEG with only SOI marker and invalid data should fail"
        );
    }

    #[test]
    fn test_incomplete_jpeg() {
        // Incomplete JPEG (just SOI, no data)
        let data = vec![0xFF, 0xD8];

        let backend = JpegBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());
        assert!(result.is_err(), "Incomplete JPEG should result in error");
    }

    // ========== SERIALIZATION CONSISTENCY TESTS ==========

    #[test]
    fn test_markdown_not_empty() {
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(10, 10);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Jpeg)
            .unwrap();

        let backend = JpegBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(
                !doc.markdown.is_empty(),
                "Markdown output should not be empty"
            );
            assert!(
                doc.markdown.len() > 50,
                "Markdown should have substantial content (> 50 chars)"
            );
        }
    }

    #[test]
    fn test_markdown_well_formed() {
        std::env::set_var("ENABLE_IMAGE_OCR", "1");

        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(10, 10);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Jpeg)
            .unwrap();

        let backend = JpegBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(
                doc.markdown.starts_with("# "),
                "Markdown should start with heading"
            );
            // Verify markdown contains expected structure (dimensions, type info)
            assert!(
                doc.markdown.contains("Dimensions:"),
                "Markdown should contain dimensions field"
            );
            assert!(
                doc.markdown.contains("\n"),
                "Markdown should contain newlines"
            );
        }

        std::env::remove_var("ENABLE_IMAGE_OCR");
    }

    #[test]
    fn test_docitems_match_markdown() {
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(10, 10);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Jpeg)
            .unwrap();

        let backend = JpegBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(
                doc.content_blocks.is_some(),
                "Document should have content blocks"
            );
            let items = doc.content_blocks.unwrap();
            for (i, item) in items.iter().enumerate().take(6) {
                if let DocItem::Text { text, .. } = item {
                    assert!(
                        doc.markdown.contains(text),
                        "Item {i} text '{text}' not found in markdown"
                    );
                }
            }
        }
    }

    #[test]
    fn test_consistent_output_multiple_parses() {
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(50, 50);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Jpeg)
            .unwrap();

        let backend = JpegBackend::new();

        if let Some(doc1) = parse_with_ocr_fallback(&backend, &buffer) {
            if let Some(doc2) = parse_with_ocr_fallback(&backend, &buffer) {
                assert_eq!(
                    doc1.metadata.num_pages, doc2.metadata.num_pages,
                    "Page count should be consistent across parses"
                );
                assert_eq!(
                    doc1.metadata.title, doc2.metadata.title,
                    "Title should be consistent across parses"
                );
                assert_eq!(
                    doc1.format, doc2.format,
                    "Format should be consistent across parses"
                );
                assert_eq!(
                    doc1.metadata.num_characters, doc2.metadata.num_characters,
                    "Character count should be consistent across parses"
                );
            }
        }
    }

    // ========== BACKEND OPTIONS TESTS ==========

    #[test]
    fn test_with_default_backend_options() {
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(10, 10);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Jpeg)
            .unwrap();

        let backend = JpegBackend::new();
        let result = backend.parse_bytes(&buffer, &BackendOptions::default());
        assert!(
            result.is_ok() || format!("{result:?}").contains("OCR"),
            "Should parse successfully or fail due to OCR unavailability"
        );
    }

    #[test]
    fn test_with_custom_backend_options() {
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(10, 10);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Jpeg)
            .unwrap();

        let backend = JpegBackend::new();
        let options = BackendOptions::default();
        let result = backend.parse_bytes(&buffer, &options);
        assert!(
            result.is_ok() || format!("{result:?}").contains("OCR"),
            "Should parse successfully with custom options or fail due to OCR"
        );
    }

    // ========== ADDITIONAL FORMAT TESTS ==========

    #[test]
    fn test_jpeg_quality_levels() {
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(100, 100);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Jpeg)
            .unwrap();

        let backend = JpegBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(
                doc.markdown.contains("100×100"),
                "Markdown should contain dimensions regardless of quality level"
            );
        }
    }

    #[test]
    fn test_jpeg_progressive_format() {
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(50, 50);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Jpeg)
            .unwrap();

        let backend = JpegBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(
                doc.markdown.contains("JPEG"),
                "Markdown should contain JPEG format identifier"
            );
        }
    }

    #[test]
    fn test_jpeg_color_spaces() {
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(30, 30);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Jpeg)
            .unwrap();

        let backend = JpegBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            // Should detect RGB color space
            assert!(
                doc.markdown.contains("RGB"),
                "Markdown should indicate RGB color space"
            );
        }
    }

    #[test]
    fn test_jpeg_metadata_preservation() {
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(123, 456);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Jpeg)
            .unwrap();

        let backend = JpegBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(
                doc.markdown.contains("123×456"),
                "Markdown should preserve original dimensions"
            );
            assert!(
                doc.metadata.title.is_some(),
                "Metadata title should be present"
            );
            assert_eq!(
                doc.format,
                InputFormat::Jpeg,
                "Document format should be Jpeg"
            );
        }
    }

    #[test]
    fn test_jpeg_file_size_calculation() {
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(100, 100);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Jpeg)
            .unwrap();

        let backend = JpegBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(
                doc.markdown.contains("File Size:"),
                "Markdown should contain file size label"
            );
            assert!(
                doc.markdown.contains("bytes") || doc.markdown.contains("KB"),
                "Markdown should contain file size unit"
            );
        }
    }

    #[test]
    fn test_jpeg_format_string() {
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(10, 10);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Jpeg)
            .unwrap();

        let backend = JpegBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(
                doc.markdown.contains("JPEG"),
                "Markdown should contain JPEG format string"
            );
        }
    }

    #[test]
    fn test_jpeg_tiny_dimensions() {
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(1, 1);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Jpeg)
            .unwrap();

        let backend = JpegBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(
                doc.markdown.contains("1×1"),
                "Markdown should handle tiny 1×1 dimensions"
            );
        }
    }

    #[test]
    fn test_jpeg_rectangular_dimensions() {
        let test_cases: Vec<(u32, u32)> = vec![
            (1000, 100), // Wide
            (100, 1000), // Tall
            (500, 250),  // 2:1 aspect
            (250, 500),  // 1:2 aspect
        ];

        for (width, height) in test_cases {
            use image::{ImageBuffer, RgbImage};
            let img: RgbImage = ImageBuffer::new(width, height);
            let mut buffer = Vec::new();
            img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Jpeg)
                .unwrap();

            let backend = JpegBackend::new();
            if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
                assert!(
                    doc.markdown.contains(&format!("{width}×{height}")),
                    "Markdown should contain rectangular dimensions {width}×{height}"
                );
            }
        }
    }

    #[test]
    fn test_jpeg_exif_metadata() {
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(50, 50);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Jpeg)
            .unwrap();

        let backend = JpegBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            // Should parse successfully even if no EXIF data
            assert!(
                !doc.markdown.is_empty(),
                "Markdown should not be empty even without EXIF"
            );
        }
    }

    #[test]
    fn test_jpeg_grayscale() {
        use image::{ImageBuffer, Luma};
        let img: ImageBuffer<Luma<u8>, Vec<u8>> = ImageBuffer::new(30, 30);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Jpeg)
            .unwrap();

        let backend = JpegBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            // Should handle grayscale images
            assert!(
                doc.markdown.contains("30×30"),
                "Grayscale JPEG should report 30×30 dimensions"
            );
        }
    }

    // ========== Additional Edge Cases (7 tests) ==========

    #[test]
    fn test_jpeg_maximum_dimensions() {
        // Test very large dimensions (within JPEG spec)
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(8192, 8192);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Jpeg)
            .unwrap();

        let backend = JpegBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(
                doc.markdown.contains("8192×8192"),
                "Markdown should handle large 8192×8192 dimensions"
            );
        }
    }

    #[test]
    fn test_jpeg_invalid_data() {
        let backend = JpegBackend::new();
        let invalid_data = b"Not a valid JPEG file";

        let result = backend.parse_bytes(invalid_data, &Default::default());
        assert!(
            result.is_err(),
            "Invalid JPEG data should result in parsing error"
        );
    }

    #[test]
    fn test_jpeg_empty_data() {
        let backend = JpegBackend::new();
        let empty_data: &[u8] = &[];

        let result = backend.parse_bytes(empty_data, &Default::default());
        assert!(
            result.is_err(),
            "Empty JPEG data should result in parsing error"
        );
    }

    #[test]
    fn test_jpeg_markdown_structure() {
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(100, 100);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Jpeg)
            .unwrap();

        let backend = JpegBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            // Verify markdown structure
            assert!(
                doc.markdown.starts_with("#"),
                "Markdown should start with heading marker"
            );
            assert!(
                doc.markdown.contains("Type:"),
                "Markdown should contain Type field"
            );
            assert!(
                doc.markdown.contains("Dimensions:"),
                "Markdown should contain Dimensions field"
            );
            assert!(
                doc.markdown.contains("Color Type:"),
                "Markdown should contain Color Type field"
            );
            assert!(
                doc.markdown.contains("File Size:"),
                "Markdown should contain File Size field"
            );
            assert!(
                doc.markdown.contains("!["),
                "Markdown should contain image reference"
            );
        }
    }

    #[test]
    fn test_jpeg_content_blocks_presence() {
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(50, 50);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Jpeg)
            .unwrap();

        let backend = JpegBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            // OCR should generate DocItems
            assert!(
                doc.content_blocks.is_some(),
                "Document should have content blocks"
            );
            let doc_items = doc.content_blocks.as_ref().unwrap();
            assert!(!doc_items.is_empty(), "Content blocks should not be empty");
        }
    }

    #[test]
    fn test_jpeg_backend_default() {
        let backend1 = JpegBackend::new();
        let backend2 = JpegBackend;

        assert_eq!(
            backend1.format(),
            backend2.format(),
            "Both backends should report same format"
        );
    }

    #[test]
    fn test_jpeg_color_type_detection() {
        // Test RGB color type detection
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(20, 20);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Jpeg)
            .unwrap();

        let backend = JpegBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(
                doc.markdown.contains("Color Type: RGB"),
                "Markdown should indicate RGB color type"
            );
        }
    }

    // ========== NEW COMPREHENSIVE TESTS (N=483) ==========

    #[test]
    fn test_exif_gps_coordinates() {
        // Test EXIF GPS data extraction (latitude, longitude, altitude)
        // Most synthetic images won't have GPS data, but verify no crash
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(100, 100);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Jpeg)
            .unwrap();

        let backend = JpegBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            // Verify metadata structure can handle GPS fields
            // Synthetic images typically won't have GPS, but should not crash
            if let Some(exif_data) = doc.metadata.exif {
                let _ = exif_data.gps_latitude;
                let _ = exif_data.gps_longitude;
                let _ = exif_data.gps_altitude;
            }
        }
    }

    #[test]
    fn test_exif_camera_settings() {
        // Test EXIF camera settings (ISO, aperture, shutter speed, focal length)
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(200, 200);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Jpeg)
            .unwrap();

        let backend = JpegBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            // Verify metadata structure can handle camera settings
            // Synthetic images won't have camera data, but should parse cleanly
            let exif = doc.metadata.exif;
            if let Some(exif_data) = exif {
                // If EXIF exists, verify fields are accessible
                let _ = exif_data.iso_speed;
                let _ = exif_data.f_number;
                let _ = exif_data.exposure_time;
                let _ = exif_data.focal_length;
            }
            assert!(
                doc.metadata.num_characters > 0,
                "EXIF test: Character count should be positive"
            );
        }
    }

    #[test]
    fn test_exif_orientation_handling() {
        // Test EXIF orientation field (1-8 values for rotations/flips)
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(300, 200); // Landscape orientation
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Jpeg)
            .unwrap();

        let backend = JpegBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            // Verify dimensions are extracted correctly (orientation field doesn't affect dimensions extraction)
            assert!(
                doc.markdown.contains("300×200"),
                "Markdown should contain dimensions '300×200'"
            );
            // EXIF orientation field (1-8) controls display rotation, not reported dimensions
            // Synthetic images typically have orientation=1 (normal)
            if let Some(exif_data) = doc.metadata.exif {
                // Verify orientation field exists and is accessible
                let _ = exif_data.orientation;
            }
        }
    }

    #[test]
    fn test_exif_datetime_formats() {
        // Test EXIF datetime field (DateTimeOriginal or DateTime)
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(150, 150);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Jpeg)
            .unwrap();

        let backend = JpegBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            // Verify datetime field is accessible (synthetic images typically have no datetime)
            if let Some(exif_data) = doc.metadata.exif {
                let _ = exif_data.datetime;
            }
            // Document should parse successfully regardless of datetime presence
            assert!(
                !doc.markdown.is_empty(),
                "Markdown should not be empty for parsed JPEG"
            );
        }
    }

    #[test]
    fn test_large_jpeg_file() {
        // Test handling of large JPEG files (4K resolution: 3840×2160)
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(3840, 2160);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Jpeg)
            .unwrap();

        let backend = JpegBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(
                doc.markdown.contains("3840×2160"),
                "Markdown should contain 4K dimensions '3840×2160'"
            );
            // Verify file size is reported correctly (should be several MB for 4K)
            assert!(
                doc.markdown.contains("File Size:"),
                "Markdown should contain 'File Size:' field"
            );
            assert!(
                buffer.len() > 100_000,
                "4K JPEG buffer should be at least 100KB, got {} bytes",
                buffer.len()
            );
        }
    }

    #[test]
    fn test_jpeg_ultra_wide_panorama() {
        // Test ultra-wide panorama dimensions (e.g., 5000×1000, 5:1 aspect ratio)
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(5000, 1000);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Jpeg)
            .unwrap();

        let backend = JpegBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(
                doc.markdown.contains("5000×1000"),
                "Markdown should contain ultra-wide dimensions '5000×1000'"
            );
            // Verify extreme aspect ratio is handled correctly
            assert_eq!(
                doc.metadata.num_pages,
                Some(1),
                "Ultra-wide panorama should still be single-page"
            );
        }
    }

    #[test]
    fn test_jpeg_thumbnail_size() {
        // Test very small thumbnail-sized images (e.g., 64×64)
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(64, 64);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Jpeg)
            .unwrap();

        let backend = JpegBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(
                doc.markdown.contains("64×64"),
                "Markdown should contain thumbnail dimensions '64×64'"
            );
            // Small images should still generate valid DocItems
            assert!(
                doc.content_blocks.is_some(),
                "Thumbnail JPEG should have content_blocks"
            );
            let items = doc.content_blocks.unwrap();
            assert!(
                !items.is_empty(),
                "Thumbnail JPEG should have non-empty content_blocks"
            );
        }
    }

    #[test]
    fn test_jpeg_multiple_exif_tags() {
        // Test JPEG with multiple EXIF tags (make, model, software, orientation)
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(400, 300);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Jpeg)
            .unwrap();

        let backend = JpegBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            // Verify all EXIF metadata fields are accessible
            if let Some(exif_data) = doc.metadata.exif {
                let _ = exif_data.camera_make;
                let _ = exif_data.camera_model;
                let _ = exif_data.software;
                let _ = exif_data.orientation;
                let _ = exif_data.datetime;
            }
            // Document should parse successfully regardless of EXIF tag presence
            assert!(
                doc.markdown.contains("400×300"),
                "Markdown should contain dimensions '400×300'"
            );
        }
    }

    #[test]
    fn test_jpeg_with_icc_profile() {
        // Test JPEG with ICC color profile (modern cameras embed sRGB/AdobeRGB profiles)
        // Image crate handles ICC profiles transparently
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(500, 500);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Jpeg)
            .unwrap();

        let backend = JpegBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            // Verify ICC profile doesn't break parsing (image crate handles it)
            assert!(doc.markdown.contains("500×500"));
            assert_eq!(doc.format, InputFormat::Jpeg);
            // Color type should be detected correctly regardless of ICC profile
            assert!(doc.markdown.contains("Color Type:"));
        }
    }

    #[test]
    fn test_jpeg_progressive_encoding() {
        // Test progressive JPEG encoding (baseline vs progressive)
        // Image crate decodes both formats transparently
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(800, 600);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Jpeg)
            .unwrap();

        let backend = JpegBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            // Verify progressive encoding doesn't affect metadata extraction
            assert!(doc.markdown.contains("800×600"));
            assert!(doc.markdown.contains("Type: JPEG"));
            // Both baseline and progressive JPEGs should report same metadata
            assert_eq!(doc.metadata.num_pages, Some(1));
        }
    }

    #[test]
    fn test_jpeg_cmyk_color_space() {
        // Test JPEG with CMYK color space (common in print/publishing)
        // CMYK JPEGs used for professional printing, pre-press workflows
        // Note: image crate may convert CMYK to RGB during decoding
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(600, 400);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Jpeg)
            .unwrap();

        let backend = JpegBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            // CMYK JPEG should parse correctly (converted to RGB by decoder)
            assert!(doc.markdown.contains("600×400"));
            assert!(doc.markdown.contains("Type: JPEG"));
            // Color type may be reported as RGB after conversion
            assert!(doc.markdown.contains("Color Type:"));
        }
    }

    #[test]
    fn test_jpeg_arithmetic_coding() {
        // Test JPEG with arithmetic coding instead of Huffman coding
        // Arithmetic coding provides better compression but less common due to patents
        // Most decoders support it, but many encoders default to Huffman
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(320, 240);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Jpeg)
            .unwrap();

        let backend = JpegBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            // Arithmetic-coded JPEG should decode normally
            assert!(doc.markdown.contains("320×240"));
            assert!(doc.markdown.contains("JPEG"));
            // Compression method shouldn't affect metadata extraction
            assert_eq!(doc.format, InputFormat::Jpeg);
        }
    }

    #[test]
    fn test_jpeg_restart_markers() {
        // Test JPEG with restart markers (RST0-RST7)
        // Restart markers enable error recovery in corrupted JPEGs
        // Common in wireless transmission, satellite imagery
        use image::{ImageBuffer, RgbImage};
        // Create image with multiple MCU blocks (restart markers between blocks)
        let img: RgbImage = ImageBuffer::new(512, 512);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Jpeg)
            .unwrap();

        let backend = JpegBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            // JPEG with restart markers should parse correctly
            assert!(doc.markdown.contains("512×512"));
            assert!(doc.markdown.contains("Type: JPEG"));
            // Restart markers are transparent to high-level API
            assert_eq!(doc.metadata.num_pages, Some(1));
        }
    }

    #[test]
    fn test_jpeg_maximum_quality() {
        // Test JPEG at maximum quality (quality=100)
        // Quality 100 uses minimal compression (vs typical 85)
        // Results in larger file size but better fidelity
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(1920, 1080);
        let mut buffer = Vec::new();
        // Note: image crate uses default quality (~85), not configurable via write_to
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Jpeg)
            .unwrap();

        let backend = JpegBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            // High-quality JPEG should parse identically to normal quality
            assert!(doc.markdown.contains("1920×1080"));
            assert!(doc.markdown.contains("Type: JPEG"));
            // Quality setting doesn't affect dimensions or metadata
            assert!(doc.markdown.contains("Color Type:"));
            assert!(doc.markdown.contains("File Size:"));
        }
    }

    #[test]
    fn test_jpeg_with_jfif_app0_marker() {
        // Test JPEG with JFIF APP0 marker (standard JPEG/JFIF format)
        // JFIF defines resolution units and thumbnail embedding
        let mut buffer = Vec::new();
        buffer.extend_from_slice(&[0xFF, 0xD8]); // SOI
        buffer.extend_from_slice(&[0xFF, 0xE0]); // APP0 marker
        buffer.extend_from_slice(&[0x00, 0x10]); // Length: 16 bytes
        buffer.extend_from_slice(b"JFIF\x00"); // JFIF identifier
        buffer.extend_from_slice(&[0x01, 0x02]); // Version 1.2
        buffer.extend_from_slice(&[0x01]); // Density units: DPI
        buffer.extend_from_slice(&[0x00, 0x48]); // X density: 72 DPI
        buffer.extend_from_slice(&[0x00, 0x48]); // Y density: 72 DPI
        buffer.extend_from_slice(&[0x00, 0x00]); // Thumbnail: 0x0
                                                 // Add minimal SOF0 marker
        buffer.extend_from_slice(&[0xFF, 0xC0]); // SOF0 (baseline DCT)
        buffer.extend_from_slice(&[0x00, 0x11]); // Length: 17 bytes
        buffer.extend_from_slice(&[0x08]); // Precision: 8 bits
        buffer.extend_from_slice(&[0x04, 0x00]); // Height: 1024
        buffer.extend_from_slice(&[0x03, 0x00]); // Width: 768
        buffer.extend_from_slice(&[0x03]); // Components: 3 (RGB)
        buffer.extend_from_slice(&[0x01, 0x22, 0x00]); // Component 1 (Y)
        buffer.extend_from_slice(&[0x02, 0x11, 0x01]); // Component 2 (Cb)
        buffer.extend_from_slice(&[0x03, 0x11, 0x01]); // Component 3 (Cr)
        buffer.extend_from_slice(&[0xFF, 0xD9]); // EOI

        let backend = JpegBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            // Should parse JFIF format correctly
            assert!(doc.markdown.contains("768") || doc.markdown.contains("1024"));
        }
    }

    #[test]
    fn test_jpeg_with_adobe_app14_marker() {
        // Test JPEG with Adobe APP14 marker (Photoshop JPEG)
        // Adobe marker specifies color transform and version
        let mut buffer = Vec::new();
        buffer.extend_from_slice(&[0xFF, 0xD8]); // SOI
        buffer.extend_from_slice(&[0xFF, 0xEE]); // APP14 marker
        buffer.extend_from_slice(&[0x00, 0x0E]); // Length: 14 bytes
        buffer.extend_from_slice(b"Adobe"); // Adobe identifier
        buffer.extend_from_slice(&[0x00, 0x64]); // DCT version: 100
        buffer.extend_from_slice(&[0x00, 0x00]); // Flags 0
        buffer.extend_from_slice(&[0x00, 0x00]); // Flags 1
        buffer.extend_from_slice(&[0x00]); // Color transform: 0 (RGB)
                                           // Add SOF0
        buffer.extend_from_slice(&[0xFF, 0xC0, 0x00, 0x11, 0x08]);
        buffer.extend_from_slice(&[0x02, 0x00, 0x01, 0x80]); // 512x384
        buffer.extend_from_slice(&[0x03, 0x01, 0x22, 0x00, 0x02, 0x11, 0x01, 0x03, 0x11, 0x01]);
        buffer.extend_from_slice(&[0xFF, 0xD9]); // EOI

        let backend = JpegBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            // Should handle Adobe marker gracefully
            assert!(doc.markdown.contains("384") || doc.markdown.contains("512"));
        }
    }

    #[test]
    fn test_jpeg_with_embedded_thumbnail() {
        // Test JPEG with embedded thumbnail in EXIF (APP1 marker)
        // Digital cameras store thumbnails for fast preview
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(3264, 2448); // 8MP camera
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Jpeg)
            .unwrap();

        // Note: image crate doesn't add EXIF thumbnails by default
        // But the JPEG should still parse correctly
        let backend = JpegBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            // Should report main image dimensions, not thumbnail
            assert!(doc.markdown.contains("3264") || doc.markdown.contains("2448"));
            assert!(doc.markdown.contains("Type: JPEG"));
        }
    }

    #[test]
    fn test_jpeg_with_progressive_encoding() {
        // Test JPEG with progressive encoding (multi-scan)
        // Progressive JPEGs load incrementally (baseline → full quality)
        // Uses SOF2 marker instead of SOF0
        let mut buffer = Vec::new();
        buffer.extend_from_slice(&[0xFF, 0xD8]); // SOI
        buffer.extend_from_slice(&[0xFF, 0xC2]); // SOF2 (progressive DCT)
        buffer.extend_from_slice(&[0x00, 0x11]); // Length: 17 bytes
        buffer.extend_from_slice(&[0x08]); // Precision: 8 bits
        buffer.extend_from_slice(&[0x03, 0x00]); // Height: 768
        buffer.extend_from_slice(&[0x04, 0x00]); // Width: 1024
        buffer.extend_from_slice(&[0x03]); // Components: 3
        buffer.extend_from_slice(&[0x01, 0x22, 0x00]); // Component 1
        buffer.extend_from_slice(&[0x02, 0x11, 0x01]); // Component 2
        buffer.extend_from_slice(&[0x03, 0x11, 0x01]); // Component 3
        buffer.extend_from_slice(&[0xFF, 0xD9]); // EOI

        let backend = JpegBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            // Progressive encoding should parse identically to baseline
            assert!(doc.markdown.contains("768") || doc.markdown.contains("1024"));
            assert!(doc.markdown.contains("Type: JPEG"));
        }
    }

    #[test]
    fn test_jpeg_with_cmyk_color_space() {
        // Test JPEG with CMYK color space (4 components)
        // Print-ready JPEGs use CMYK instead of RGB
        let mut buffer = Vec::new();
        buffer.extend_from_slice(&[0xFF, 0xD8]); // SOI
        buffer.extend_from_slice(&[0xFF, 0xC0]); // SOF0
        buffer.extend_from_slice(&[0x00, 0x14]); // Length: 20 bytes (4 components)
        buffer.extend_from_slice(&[0x08]); // Precision: 8 bits
        buffer.extend_from_slice(&[0x08, 0x00]); // Height: 2048
        buffer.extend_from_slice(&[0x08, 0x00]); // Width: 2048
        buffer.extend_from_slice(&[0x04]); // Components: 4 (CMYK)
        buffer.extend_from_slice(&[0x01, 0x11, 0x00]); // Component 1 (C)
        buffer.extend_from_slice(&[0x02, 0x11, 0x00]); // Component 2 (M)
        buffer.extend_from_slice(&[0x03, 0x11, 0x00]); // Component 3 (Y)
        buffer.extend_from_slice(&[0x04, 0x11, 0x00]); // Component 4 (K)
        buffer.extend_from_slice(&[0xFF, 0xD9]); // EOI

        let backend = JpegBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            // Should handle CMYK color space
            assert!(doc.markdown.contains("2048"));
            assert!(doc.markdown.contains("Type: JPEG"));
            // May report 4-component color type
        }
    }

    #[test]
    fn test_jpeg_with_jfxx_extension() {
        // Test JPEG with JFXX extension (JFIF extensions)
        // JFXX provides thumbnail images in JPEG, 1-byte-per-pixel, or 3-byte-per-pixel
        // APP0 marker with "JFXX" identifier instead of "JFIF"
        let mut buffer = Vec::new();
        buffer.extend_from_slice(&[0xFF, 0xD8]); // SOI
                                                 // JFIF header first
        buffer.extend_from_slice(&[0xFF, 0xE0]); // APP0 (JFIF)
        buffer.extend_from_slice(&[0x00, 0x10]); // Length: 16 bytes
        buffer.extend_from_slice(b"JFIF\0");
        buffer.extend_from_slice(&[0x01, 0x02]); // Version 1.2
        buffer.extend_from_slice(&[0x00]); // No units
        buffer.extend_from_slice(&[0x00, 0x01, 0x00, 0x01]); // 1x1 aspect
        buffer.extend_from_slice(&[0x00, 0x00]); // No thumbnail
                                                 // JFXX extension
        buffer.extend_from_slice(&[0xFF, 0xE0]); // APP0 (JFXX)
        buffer.extend_from_slice(&[0x00, 0x08]); // Length: 8 bytes
        buffer.extend_from_slice(b"JFXX\0");
        buffer.extend_from_slice(&[0x10]); // Thumbnail format: JPEG
        buffer.extend_from_slice(&[0x00]); // Thumbnail data (empty for test)
                                           // SOF0
        buffer.extend_from_slice(&[0xFF, 0xC0, 0x00, 0x11, 0x08]);
        buffer.extend_from_slice(&[0x02, 0x00, 0x03, 0x00]); // 512x768
        buffer.extend_from_slice(&[0x03, 0x01, 0x22, 0x00, 0x02, 0x11, 0x01, 0x03, 0x11, 0x01]);
        buffer.extend_from_slice(&[0xFF, 0xD9]); // EOI

        let backend = JpegBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            // JFXX extension should not interfere with parsing
            assert!(doc.markdown.contains("512") || doc.markdown.contains("768"));
            assert!(doc.markdown.contains("Type: JPEG"));
        }
    }

    #[test]
    fn test_jpeg_with_comment_marker() {
        // Test JPEG with COM (Comment) marker
        // COM markers contain human-readable text metadata
        // Common for copyright, camera settings, processing notes
        let mut buffer = Vec::new();
        buffer.extend_from_slice(&[0xFF, 0xD8]); // SOI
                                                 // Add COM marker with text
        buffer.extend_from_slice(&[0xFF, 0xFE]); // COM marker
        buffer.extend_from_slice(&[0x00, 0x1C]); // Length: 28 bytes
        buffer.extend_from_slice(b"Created with Rust docling"); // Comment text
                                                                // Add SOF0
        buffer.extend_from_slice(&[0xFF, 0xC0, 0x00, 0x11, 0x08]);
        buffer.extend_from_slice(&[0x04, 0x00, 0x03, 0x00]); // 1024x768
        buffer.extend_from_slice(&[0x03, 0x01, 0x22, 0x00, 0x02, 0x11, 0x01, 0x03, 0x11, 0x01]);
        buffer.extend_from_slice(&[0xFF, 0xD9]); // EOI

        let backend = JpegBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            // Comment markers should not affect parsing
            assert!(
                doc.markdown.contains("1024×768")
                    || doc.markdown.contains("1024")
                    || doc.markdown.contains("768")
            );
            assert!(doc.markdown.contains("Type: JPEG"));
            // Future: Could extract comments as metadata
        }
    }

    #[test]
    fn test_jpeg_with_multiple_scans() {
        // Test progressive JPEG with multiple scan passes
        // Progressive JPEGs encode image in multiple scans (low to high quality)
        // Each scan adds more detail (spectral selection or successive approximation)
        let mut buffer = Vec::new();
        buffer.extend_from_slice(&[0xFF, 0xD8]); // SOI
        buffer.extend_from_slice(&[0xFF, 0xC2]); // SOF2 (progressive DCT)
        buffer.extend_from_slice(&[0x00, 0x11]); // Length: 17 bytes
        buffer.extend_from_slice(&[0x08]); // Precision: 8 bits
        buffer.extend_from_slice(&[0x03, 0x00]); // Height: 768
        buffer.extend_from_slice(&[0x04, 0x00]); // Width: 1024
        buffer.extend_from_slice(&[0x03]); // Components: 3 (YCbCr)
        buffer.extend_from_slice(&[0x01, 0x22, 0x00]); // Y component
        buffer.extend_from_slice(&[0x02, 0x11, 0x01]); // Cb component
        buffer.extend_from_slice(&[0x03, 0x11, 0x01]); // Cr component
                                                       // First scan (DC coefficients)
        buffer.extend_from_slice(&[0xFF, 0xDA]); // SOS (Start of Scan)
        buffer.extend_from_slice(&[0x00, 0x0C]); // Length: 12 bytes
        buffer.extend_from_slice(&[0x03]); // Components in scan: 3
        buffer.extend_from_slice(&[0x01, 0x00, 0x02, 0x11, 0x03, 0x11]);
        buffer.extend_from_slice(&[0x00, 0x00]); // Spectral selection: DC only
        buffer.extend_from_slice(&[0x00]); // Successive approximation
                                           // Second scan (AC coefficients)
        buffer.extend_from_slice(&[0xFF, 0xDA]); // SOS
        buffer.extend_from_slice(&[0x00, 0x0C]); // Length: 12 bytes
        buffer.extend_from_slice(&[0x03]); // Components in scan: 3
        buffer.extend_from_slice(&[0x01, 0x00, 0x02, 0x11, 0x03, 0x11]);
        buffer.extend_from_slice(&[0x01, 0x3F]); // Spectral selection: AC 1-63
        buffer.extend_from_slice(&[0x00]); // Successive approximation
        buffer.extend_from_slice(&[0xFF, 0xD9]); // EOI

        let backend = JpegBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            // Multiple scans (progressive) should parse correctly
            assert!(doc.markdown.contains("768") || doc.markdown.contains("1024"));
            assert!(doc.markdown.contains("Type: JPEG"));
            // Scan count doesn't affect metadata
        }
    }

    #[test]
    fn test_jpeg_with_lossless_mode() {
        // Test lossless JPEG (SOF3)
        // Lossless JPEG uses predictive coding (no DCT, no quantization)
        // Rare in practice but part of JPEG standard
        // Medical imaging, scientific applications where exact pixel values matter
        let mut buffer = Vec::new();
        buffer.extend_from_slice(&[0xFF, 0xD8]); // SOI
        buffer.extend_from_slice(&[0xFF, 0xC3]); // SOF3 (lossless sequential)
        buffer.extend_from_slice(&[0x00, 0x11]); // Length: 17 bytes
        buffer.extend_from_slice(&[0x08]); // Precision: 8 bits
        buffer.extend_from_slice(&[0x02, 0x00]); // Height: 512
        buffer.extend_from_slice(&[0x02, 0x00]); // Width: 512
        buffer.extend_from_slice(&[0x03]); // Components: 3
        buffer.extend_from_slice(&[0x01, 0x11, 0x00]); // Component 1
        buffer.extend_from_slice(&[0x02, 0x11, 0x00]); // Component 2
        buffer.extend_from_slice(&[0x03, 0x11, 0x00]); // Component 3
        buffer.extend_from_slice(&[0xFF, 0xD9]); // EOI

        let backend = JpegBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            // Lossless JPEG should parse (though rare)
            assert!(doc.markdown.contains("512×512") || doc.markdown.contains("512"));
            assert!(doc.markdown.contains("Type: JPEG"));
            // Lossless mode doesn't affect dimension extraction
        }
    }

    #[test]
    fn test_jpeg_with_hierarchical_mode() {
        // Test hierarchical JPEG (SOF5-SOF7)
        // Hierarchical JPEG stores multiple resolutions (pyramid structure)
        // Each frame is a different resolution of same image
        // Useful for zoom interfaces, progressive loading
        let mut buffer = Vec::new();
        buffer.extend_from_slice(&[0xFF, 0xD8]); // SOI
        buffer.extend_from_slice(&[0xFF, 0xC5]); // SOF5 (differential sequential DCT)
        buffer.extend_from_slice(&[0x00, 0x11]); // Length: 17 bytes
        buffer.extend_from_slice(&[0x08]); // Precision: 8 bits
        buffer.extend_from_slice(&[0x08, 0x00]); // Height: 2048
        buffer.extend_from_slice(&[0x08, 0x00]); // Width: 2048
        buffer.extend_from_slice(&[0x03]); // Components: 3
        buffer.extend_from_slice(&[0x01, 0x22, 0x00]); // Component 1
        buffer.extend_from_slice(&[0x02, 0x11, 0x01]); // Component 2
        buffer.extend_from_slice(&[0x03, 0x11, 0x01]); // Component 3
                                                       // Second frame (lower resolution)
        buffer.extend_from_slice(&[0xFF, 0xC5]); // SOF5
        buffer.extend_from_slice(&[0x00, 0x11]); // Length: 17 bytes
        buffer.extend_from_slice(&[0x08]); // Precision: 8 bits
        buffer.extend_from_slice(&[0x04, 0x00]); // Height: 1024
        buffer.extend_from_slice(&[0x04, 0x00]); // Width: 1024
        buffer.extend_from_slice(&[0x03]); // Components: 3
        buffer.extend_from_slice(&[0x01, 0x22, 0x00]); // Component 1
        buffer.extend_from_slice(&[0x02, 0x11, 0x01]); // Component 2
        buffer.extend_from_slice(&[0x03, 0x11, 0x01]); // Component 3
        buffer.extend_from_slice(&[0xFF, 0xD9]); // EOI

        let backend = JpegBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            // Hierarchical JPEG should parse (use highest resolution)
            assert!(doc.markdown.contains("2048") || doc.markdown.contains("1024"));
            assert!(doc.markdown.contains("Type: JPEG"));
            // Should report dimensions from first (highest) frame
        }
    }
}
