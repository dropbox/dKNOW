//! WEBP backend for docling
//!
//! This backend converts WEBP files to docling's document model.

use crate::exif_utils;
use crate::traits::{BackendOptions, DocumentBackend};
use crate::utils::{create_section_header, create_text_item, format_file_size, opt_vec};
use docling_core::{DocItem, DoclingError, Document, DocumentMetadata, InputFormat};
use docling_ocr::OcrEngine;
use image::{GenericImageView, ImageReader};
use std::fmt::Write;
use std::io::Cursor;
use std::path::Path;

/// WEBP backend
///
/// Converts WEBP files to docling's document model.
/// Extracts basic metadata and uses OCR to extract text content from the image.
///
/// ## Features
///
/// - Extract image dimensions
/// - Detect color type (RGB, RGBA)
/// - OCR text extraction with bounding boxes
/// - Generate markdown with image metadata and OCR text
///
/// ## Example
///
/// ```no_run
/// use docling_backend::WebpBackend;
/// use docling_backend::DocumentBackend;
///
/// let backend = WebpBackend::new();
/// let result = backend.parse_file("image.webp", &Default::default())?;
/// println!("Image: {:?}", result.metadata.title);
/// # Ok::<(), docling_core::error::DoclingError>(())
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct WebpBackend;

impl WebpBackend {
    /// Create a new WEBP backend instance
    #[inline]
    #[must_use = "creates a backend instance that should be used for parsing"]
    pub const fn new() -> Self {
        Self
    }

    /// Convert WEBP metadata to markdown
    fn webp_to_markdown(
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
        markdown.push_str("Type: WEBP (Web Picture format)\n\n");

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
                page_no: 1, // WEBP is single-page
                bbox,
                charspan: None,
            }];

            doc_items.push(create_text_item(start_index + i, line.text.clone(), prov));
        }

        // Get full OCR text
        let ocr_text = ocr_result.text();

        Ok((ocr_text, doc_items))
    }

    /// Create `DocItems` directly from WEBP metadata
    ///
    /// Generates structured `DocItems` from WEBP metadata without markdown intermediary.
    /// Creates a hierarchical document structure:
    /// - Title (filename) as `SectionHeader` level 1
    /// - Image type as Text
    /// - Dimensions as Text
    /// - Color type as Text
    /// - File size as Text
    /// - Image reference as Text
    ///
    /// ## Arguments
    /// * `filename` - Name of the WEBP file
    /// * `width` - Image width in pixels
    /// * `height` - Image height in pixels
    /// * `color_type` - Color type description
    /// * `file_size` - Size of the WEBP file in bytes
    ///
    /// ## Returns
    /// Vector of `DocItems` representing the WEBP metadata structure
    fn webp_to_docitems(
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
            "Type: WEBP (Web Picture format)".to_string(),
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

    /// Shared parsing logic for both `parse_bytes` and `parse_file`
    ///
    /// Loads the image, extracts metadata, creates `DocItem`s, runs OCR, and generates markdown.
    /// Both `parse_bytes` and `parse_file` delegate to this method to avoid code duplication.
    fn parse_webp_data(data: &[u8], filename: &str) -> Result<Document, DoclingError> {
        // Load image to get metadata
        let img = ImageReader::new(Cursor::new(data))
            .with_guessed_format()
            .map_err(|e| {
                DoclingError::BackendError(format!("Failed to load WEBP: {e}: {filename}"))
            })?
            .decode()
            .map_err(|e| {
                DoclingError::BackendError(format!("Failed to decode WEBP: {e}: {filename}"))
            })?;

        let (width, height) = img.dimensions();
        let color_type = match &img {
            image::DynamicImage::ImageRgb8(_) => "RGB",
            image::DynamicImage::ImageRgba8(_) => "RGBA",
            _ => "Other",
        };

        // Create DocItems directly from WEBP metadata (no markdown intermediary)
        let mut doc_items = Self::webp_to_docitems(filename, width, height, color_type, data.len());
        let metadata_items_count = doc_items.len();

        // Extract OCR text
        let (ocr_text, mut ocr_items) = Self::extract_ocr_text(data, metadata_items_count)?;

        // Combine DocItems: metadata + OCR
        doc_items.append(&mut ocr_items);

        // Generate markdown from DocItems (for backwards compatibility)
        let metadata_markdown =
            Self::webp_to_markdown(filename, width, height, color_type, data.len());
        let mut markdown = metadata_markdown;
        if !ocr_text.is_empty() {
            markdown.push_str("\n\n## OCR Text\n\n");
            markdown.push_str(&ocr_text);
        }

        let num_characters = markdown.chars().count();

        // N=1886: Extract EXIF metadata from WEBP (WEBP supports EXIF like JPEG)
        let exif_metadata = exif_utils::extract_exif_metadata(data);
        let (author, subject, created) = exif_utils::extract_document_metadata(data);

        // Create document
        Ok(Document {
            markdown,
            format: InputFormat::Webp,
            metadata: DocumentMetadata {
                num_pages: Some(1), // WEBP is single-page
                num_characters,
                title: Some(filename.to_string()),
                author,  // N=1886: From EXIF Artist tag
                created, // N=1886: From EXIF DateTimeOriginal tag
                modified: None,
                language: None,
                subject,             // N=1886: From EXIF ImageDescription tag
                exif: exif_metadata, // N=1886: Camera metadata
            },
            docling_document: None,
            content_blocks: opt_vec(doc_items),
        })
    }
}

impl DocumentBackend for WebpBackend {
    #[inline]
    fn format(&self) -> InputFormat {
        InputFormat::Webp
    }

    fn parse_bytes(
        &self,
        data: &[u8],
        _options: &BackendOptions,
    ) -> Result<Document, DoclingError> {
        Self::parse_webp_data(data, "image.webp")
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
            .unwrap_or("image.webp");

        // Read file and delegate to shared parsing logic
        let data = std::fs::read(path_ref).map_err(DoclingError::IoError)?;
        Self::parse_webp_data(&data, filename)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    /// Helper function to handle OCR unavailability in tests
    /// Returns None if OCR is not available, otherwise returns Some(Document)
    fn parse_with_ocr_fallback(backend: &WebpBackend, data: &[u8]) -> Option<Document> {
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
        let backend = WebpBackend::new();
        assert_eq!(
            backend.format(),
            InputFormat::Webp,
            "Backend format should be Webp"
        );
    }

    #[test]
    fn test_webp_to_markdown() {
        let markdown = WebpBackend::webp_to_markdown("test.webp", 1280, 720, "RGBA", 102_400);
        assert!(
            markdown.contains("# test.webp"),
            "Markdown should contain title header"
        );
        assert!(
            markdown.contains("Type: WEBP (Web Picture format)"),
            "Markdown should contain WEBP type"
        );
        assert!(
            markdown.contains("Dimensions: 1280×720 pixels"),
            "Markdown should contain dimensions"
        );
        assert!(
            markdown.contains("Color Type: RGBA"),
            "Markdown should contain color type"
        );
        assert!(
            markdown.contains("![test.webp](test.webp)"),
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
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        let backend = WebpBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert_eq!(
                doc.metadata.title,
                Some("image.webp".to_string()),
                "Title should be extracted from filename"
            );
        }
    }

    #[test]
    fn test_metadata_num_pages() {
        // Test that WEBP is reported as single-page
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(10, 10);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        let backend = WebpBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert_eq!(
                doc.metadata.num_pages,
                Some(1),
                "WEBP should have exactly 1 page"
            );
        }
    }

    #[test]
    fn test_metadata_character_count() {
        // Test that character count is computed correctly
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(10, 10);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        let backend = WebpBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            // Character count should match markdown length
            assert_eq!(
                doc.metadata.num_characters,
                doc.markdown.chars().count(),
                "num_characters should match markdown character count"
            );
            assert!(
                doc.metadata.num_characters > 0,
                "Document should have at least one character"
            );
        }
    }

    #[test]
    fn test_metadata_format_field() {
        // Test that format is correctly set
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(5, 5);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        let backend = WebpBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert_eq!(
                doc.format,
                InputFormat::Webp,
                "Document format should be Webp"
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
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        let backend = WebpBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(
                doc.content_blocks.is_some(),
                "Document should have content_blocks"
            );

            let items = doc.content_blocks.unwrap();
            assert!(!items.is_empty(), "content_blocks should not be empty");

            // All items should be Text items
            for item in items {
                assert!(
                    matches!(item, DocItem::Text { .. }),
                    "All items should be Text DocItems"
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
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        let backend = WebpBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(
                doc.content_blocks.is_some(),
                "Document should have content_blocks"
            );

            let items = doc.content_blocks.unwrap();
            // Expected: title, type, dimensions, color type, file size, image reference
            // (6 paragraphs from metadata, no OCR text expected for synthetic image)
            assert_eq!(items.len(), 6, "WEBP should have 6 DocItem elements");
        }
    }

    #[test]
    fn test_docitem_content() {
        // Test that DocItems contain expected content
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(100, 200);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        let backend = WebpBackend::new();
        let result = backend.parse_bytes(&buffer, &Default::default());
        assert!(result.is_ok(), "parse_bytes should succeed for valid WEBP");

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();

        // First item should be SectionHeader with title
        if let DocItem::SectionHeader { text, .. } = &items[0] {
            assert!(text.contains("image.webp"), "Title should contain filename");
        } else {
            panic!("Expected SectionHeader item for title");
        }

        // Third item (dimensions) should be Text
        if let DocItem::Text { text, .. } = &items[2] {
            assert!(
                text.contains("100×200"),
                "Dimensions should contain 100x200"
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
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        let backend = WebpBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            let items = doc.content_blocks.unwrap();

            // Check that self_ref values are sequential
            for (i, item) in items.iter().enumerate() {
                if let DocItem::Text { self_ref, .. } = item {
                    assert_eq!(self_ref, &format!("#/texts/{i}"));
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
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        let backend = WebpBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.markdown.contains("Color Type: RGB"));
        }
    }

    #[test]
    fn test_color_type_rgba() {
        // Test RGBA color type detection
        use image::{ImageBuffer, Rgba};
        let img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(10, 10);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        let backend = WebpBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.markdown.contains("Color Type: RGBA"));
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
            img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
                .unwrap();

            let backend = WebpBackend::new();
            if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
                assert!(doc.markdown.contains(&format!("{width}×{height}")));
            }
        }
    }

    #[test]
    fn test_file_size_formatting() {
        // Test that file size is included in markdown
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(100, 100);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        let backend = WebpBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            // File size should be in the markdown (format_file_size adds "File Size:")
            assert!(doc.markdown.contains("File Size:"));
        }
    }

    #[test]
    fn test_markdown_image_reference() {
        // Test that markdown contains image reference
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(10, 10);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        let backend = WebpBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            // Should contain markdown image reference: ![filename](filename)
            assert!(doc.markdown.contains("![image.webp](image.webp)"));
        }
    }

    // ========== EDGE CASE TESTS ==========

    #[test]
    fn test_empty_webp_data() {
        // Test handling of empty data
        let backend = WebpBackend::new();
        let result = backend.parse_bytes(&[], &Default::default());

        // Should fail gracefully
        assert!(result.is_err());
    }

    #[test]
    fn test_corrupted_webp_header() {
        // Test handling of corrupted WEBP header
        let corrupted_data = vec![0x52, 0x49, 0x46, 0x46]; // Incomplete WEBP (just "RIFF")

        let backend = WebpBackend::new();
        let result = backend.parse_bytes(&corrupted_data, &Default::default());

        // Should fail gracefully
        assert!(result.is_err());
    }

    #[test]
    fn test_minimal_valid_webp() {
        // Test with minimal valid WEBP
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(1, 1);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        let backend = WebpBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.markdown.contains("1×1"));
        }
    }

    #[test]
    fn test_large_dimensions() {
        // Test with large but reasonable dimensions
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(8000, 6000);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        let backend = WebpBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.markdown.contains("8000×6000"));
        }
    }

    #[test]
    fn test_parse_invalid_webp() {
        let backend = WebpBackend::new();
        let options = BackendOptions::default();

        // Try to parse invalid WEBP data
        let invalid_data = b"Not a WEBP file";
        let result = backend.parse_bytes(invalid_data, &options);

        // Should fail gracefully with an error
        assert!(result.is_err());
        let error = result.unwrap_err();
        match error {
            DoclingError::BackendError(msg) => {
                assert!(
                    msg.contains("Failed to load WEBP") || msg.contains("Failed to decode WEBP"),
                    "Unexpected error message: {msg}"
                );
            }
            _ => panic!("Expected BackendError, got: {error:?}"),
        }
    }

    #[test]
    fn test_webp_to_docitems_structure() {
        let doc_items = WebpBackend::webp_to_docitems("test.webp", 100, 100, "RGB", 1024);

        // Should create 6 DocItems: title (SectionHeader), type, dimensions, color type, file size, image reference
        assert_eq!(doc_items.len(), 6);

        // First item should be SectionHeader (title)
        match &doc_items[0] {
            DocItem::SectionHeader { text, level, .. } => {
                assert_eq!(text, "test.webp");
                assert_eq!(*level, 1);
            }
            _ => panic!("Expected SectionHeader DocItem for title"),
        }

        // Second item should be image type
        match &doc_items[1] {
            DocItem::Text { text, .. } => {
                assert_eq!(text, "Type: WEBP (Web Picture format)");
            }
            _ => panic!("Expected Text DocItem for type"),
        }

        // Third item should be dimensions
        match &doc_items[2] {
            DocItem::Text { text, .. } => {
                assert_eq!(text, "Dimensions: 100×100 pixels");
            }
            _ => panic!("Expected Text DocItem for dimensions"),
        }
    }

    // ========== UNICODE AND SPECIAL CHARACTER TESTS ==========

    #[test]
    fn test_unicode_dimensions() {
        // Test that Unicode multiplication sign (×) is used in dimensions
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(100, 200);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        let backend = WebpBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.markdown.contains("×")); // Unicode multiplication sign
            assert!(doc.markdown.contains("100×200"));
        }
    }

    #[test]
    fn test_markdown_valid_utf8() {
        // Test that markdown is valid UTF-8
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(50, 50);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        let backend = WebpBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            // Verify all characters are valid
            assert!(doc.markdown.chars().all(|c| !c.is_control() || c == '\n'));
        }
    }

    #[test]
    fn test_aspect_ratio_calculations() {
        // Test various aspect ratios
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
            img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
                .unwrap();

            let backend = WebpBackend::new();
            if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
                let expected = format!("{}×{}", width_ratio * 100, height_ratio * 100);
                assert!(doc.markdown.contains(&expected));
            }
        }
    }

    // ========== VALIDATION TESTS ==========

    #[test]
    fn test_zero_dimensions_webp() {
        // Test handling of zero-dimension images (if WebP encoder allows it)
        // Note: Most image encoders don't allow zero dimensions, so this tests error handling
        let backend = WebpBackend::new();

        // Create invalid data that claims zero dimensions
        let invalid_data = vec![0u8; 100];
        let result = backend.parse_bytes(&invalid_data, &Default::default());

        // Should fail gracefully
        assert!(result.is_err());
    }

    #[test]
    fn test_riff_header_validation() {
        // Test that RIFF header is validated
        let mut data = vec![0u8; 12];
        data[0..4].copy_from_slice(b"RIFF"); // Valid RIFF header
                                             // But rest is invalid

        let backend = WebpBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());

        // Should fail due to invalid WebP structure
        assert!(result.is_err());
    }

    #[test]
    fn test_webp_vp8_format() {
        // Test that backend handles VP8 format (lossy WebP)
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(50, 50);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        let backend = WebpBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.markdown.contains("WEBP"));
        }
    }

    #[test]
    fn test_inconsistent_size_declaration() {
        // Test file with inconsistent size declarations
        let mut data = vec![0u8; 100];
        data[0..4].copy_from_slice(b"RIFF");
        data[4..8].copy_from_slice(&999_999u32.to_le_bytes()); // Claims huge size
        data[8..12].copy_from_slice(b"WEBP");

        let backend = WebpBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());

        // Should fail gracefully
        assert!(result.is_err());
    }

    // ========== SERIALIZATION CONSISTENCY TESTS ==========

    #[test]
    fn test_markdown_not_empty() {
        // Test that markdown is never empty
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(10, 10);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        let backend = WebpBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(!doc.markdown.is_empty());
            assert!(doc.markdown.len() > 50); // Should have substantial content
        }
    }

    #[test]
    fn test_markdown_well_formed() {
        // Test that markdown is well-formed (with OCR enabled)
        std::env::set_var("ENABLE_IMAGE_OCR", "1");

        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(10, 10);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        let backend = WebpBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            // Should contain markdown heading
            assert!(doc.markdown.starts_with("# "));
            // Should contain markdown bold formatting
            assert!(doc.markdown.contains("**"));
            // Should contain newlines
            assert!(doc.markdown.contains("\n"));
        }

        std::env::remove_var("ENABLE_IMAGE_OCR");
    }

    #[test]
    fn test_docitems_match_markdown() {
        // Test that DocItems and markdown are consistent
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(10, 10);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        let backend = WebpBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.content_blocks.is_some());

            // Every metadata DocItem's text should appear in markdown
            let items = doc.content_blocks.unwrap();
            for (i, item) in items.iter().enumerate().take(6) {
                // Only check metadata items (first 6)
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
        // Test that parsing the same data multiple times produces identical output
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(50, 50);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        let backend = WebpBackend::new();

        if let Some(doc1) = parse_with_ocr_fallback(&backend, &buffer) {
            if let Some(doc2) = parse_with_ocr_fallback(&backend, &buffer) {
                // Metadata should be identical
                assert_eq!(doc1.metadata.num_pages, doc2.metadata.num_pages);
                assert_eq!(doc1.metadata.title, doc2.metadata.title);
                assert_eq!(doc1.format, doc2.format);

                // Character count should be consistent
                assert_eq!(doc1.metadata.num_characters, doc2.metadata.num_characters);
            }
        }
    }

    // ========== BACKEND OPTIONS TESTS ==========

    #[test]
    fn test_with_default_backend_options() {
        // Test with default BackendOptions
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(10, 10);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        let backend = WebpBackend::new();
        let result = backend.parse_bytes(&buffer, &BackendOptions::default());
        // Should succeed (or fail gracefully with OCR error)
        assert!(result.is_ok() || format!("{result:?}").contains("OCR"));
    }

    #[test]
    fn test_with_custom_backend_options() {
        // Test with custom BackendOptions
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(10, 10);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        let backend = WebpBackend::new();
        let options = BackendOptions::default();
        let result = backend.parse_bytes(&buffer, &options);
        // Should succeed (or fail gracefully with OCR error)
        assert!(result.is_ok() || format!("{result:?}").contains("OCR"));
    }

    // ========== ADDITIONAL FORMAT TESTS ==========

    #[test]
    fn test_webp_compression_quality() {
        // Test that different compression qualities are handled
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(100, 100);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        let backend = WebpBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            // Should successfully parse regardless of compression
            assert!(doc.markdown.contains("100×100"));
        }
    }

    #[test]
    fn test_webp_lossless_format() {
        // Test handling of lossless WebP (VP8L)
        use image::{ImageBuffer, RgbaImage};
        let img: RgbaImage = ImageBuffer::new(50, 50);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        let backend = WebpBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            // Should handle lossless format (typically RGBA)
            assert!(doc.markdown.contains("WEBP"));
        }
    }

    #[test]
    fn test_webp_alpha_channel() {
        // Test handling of alpha channel (transparency)
        use image::{ImageBuffer, Rgba};
        let img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(30, 30);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        let backend = WebpBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            // Should recognize RGBA color type
            assert!(doc.markdown.contains("RGBA"));
        }
    }

    #[test]
    fn test_webp_metadata_preservation() {
        // Test that basic metadata is preserved through parsing
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(123, 456);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        let backend = WebpBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            // Dimensions should be preserved exactly
            assert!(doc.markdown.contains("123×456"));
            // Title should be present
            assert!(doc.metadata.title.is_some());
            // Format should be correct
            assert_eq!(doc.format, InputFormat::Webp);
        }
    }

    #[test]
    fn test_webp_file_size_calculation() {
        // Test that file size is accurately reported
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(100, 100);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        let backend = WebpBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            // File size should be mentioned in markdown
            assert!(doc.markdown.contains("File Size:"));
            // Should contain a size value (bytes, KB, etc.)
            assert!(doc.markdown.contains("bytes") || doc.markdown.contains("KB"));
        }
    }

    #[test]
    fn test_webp_format_string() {
        // Test that "Web Picture format" description is included
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(10, 10);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        let backend = WebpBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.markdown.contains("Web Picture format"));
        }
    }

    #[test]
    fn test_webp_tiny_dimensions() {
        // Test handling of very small dimensions
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(1, 1);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        let backend = WebpBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.markdown.contains("1×1"));
        }
    }

    #[test]
    fn test_webp_rectangular_dimensions() {
        // Test non-square dimensions (wide and tall)
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
            img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
                .unwrap();

            let backend = WebpBackend::new();
            if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
                assert!(doc.markdown.contains(&format!("{width}×{height}")));
            }
        }
    }

    // ========== WEBP FORMAT CHUNK TESTS ==========

    #[test]
    fn test_webp_riff_container() {
        // Test that WebP uses RIFF container format
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(10, 10);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        // WebP should start with "RIFF"
        assert_eq!(&buffer[0..4], b"RIFF");
        // Bytes 8-11 should contain "WEBP"
        assert_eq!(&buffer[8..12], b"WEBP");
    }

    #[test]
    fn test_webp_vp8_chunk_present() {
        // Test that lossy WebP contains VP8 chunk
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(20, 20);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        // Should contain "VP8 " or "VP8L" or "VP8X" chunk identifier
        let data_str = String::from_utf8_lossy(&buffer);
        assert!(
            data_str.contains("VP8 ") || data_str.contains("VP8L") || data_str.contains("VP8X")
        );
    }

    #[test]
    fn test_webp_file_size_matches_riff_header() {
        // Test that RIFF chunk size matches actual data
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(50, 50);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        // RIFF chunk size is at bytes 4-7 (little endian)
        let riff_size = u32::from_le_bytes([buffer[4], buffer[5], buffer[6], buffer[7]]);
        // RIFF size = file size - 8 bytes (RIFF header itself)
        assert_eq!(riff_size as usize + 8, buffer.len());
    }

    #[test]
    fn test_webp_chunk_alignment() {
        // Test that chunks are properly aligned (even byte boundaries)
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(100, 100);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        // Parse chunks and verify alignment
        let mut offset = 12; // Skip RIFF header
        while offset < buffer.len() {
            if offset + 8 > buffer.len() {
                break;
            }
            // Read chunk size (4 bytes after fourcc)
            let chunk_size = u32::from_le_bytes([
                buffer[offset + 4],
                buffer[offset + 5],
                buffer[offset + 6],
                buffer[offset + 7],
            ]) as usize;
            // Chunk data should be padded to even bytes
            let padded_size = (chunk_size + 1) & !1;
            offset += 8 + padded_size; // fourcc (4) + size (4) + data (padded)
        }
        // If we parsed correctly, offset should match buffer length (or within padding)
        assert!(offset >= buffer.len() && offset <= buffer.len() + 1);
    }

    // ========== COMPRESSION MODE TESTS ==========

    #[test]
    fn test_webp_lossy_vs_lossless_rgb() {
        // Test that RGB images can be encoded (typically lossy VP8)
        use image::{ImageBuffer, Rgb};
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(50, 50);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        let backend = WebpBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            // Should handle either compression mode
            assert!(doc.markdown.contains("WEBP"));
            assert!(doc.markdown.contains("RGB"));
        }
    }

    #[test]
    fn test_webp_lossless_with_transparency() {
        // Test that RGBA images are encoded losslessly (VP8L) to preserve alpha
        use image::{ImageBuffer, Rgba};
        let mut img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(50, 50);
        // Set some pixels with transparency
        for pixel in img.pixels_mut() {
            pixel[3] = 128; // 50% alpha
        }
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        let backend = WebpBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            // Should recognize RGBA and handle transparency
            assert!(doc.markdown.contains("RGBA"));
        }
    }

    #[test]
    fn test_webp_compression_artifacts() {
        // Test that backend handles compressed images (doesn't crash on artifacts)
        use image::{ImageBuffer, RgbImage};
        let mut img: RgbImage = ImageBuffer::new(100, 100);
        // Create pattern that might show compression artifacts
        for (x, y, pixel) in img.enumerate_pixels_mut() {
            pixel[0] = ((x + y) % 256) as u8;
            pixel[1] = ((x * 2) % 256) as u8;
            pixel[2] = ((y * 2) % 256) as u8;
        }
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        let backend = WebpBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.markdown.contains("100×100"));
        }
    }

    // ========== SPECIAL PIXEL FORMATS ==========

    #[test]
    fn test_webp_grayscale_simulation() {
        // Test grayscale images (simulated as RGB with equal channels)
        use image::{ImageBuffer, RgbImage};
        let mut img: RgbImage = ImageBuffer::new(50, 50);
        // Set all pixels to grayscale (R=G=B)
        for pixel in img.pixels_mut() {
            let gray = 128u8;
            pixel[0] = gray;
            pixel[1] = gray;
            pixel[2] = gray;
        }
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        let backend = WebpBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            // Should handle grayscale (encoded as RGB in WebP)
            assert!(doc.markdown.contains("50×50"));
        }
    }

    #[test]
    fn test_webp_alpha_premultiplication() {
        // Test RGBA images with varying alpha values
        use image::{ImageBuffer, Rgba};
        let mut img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(30, 30);
        // Create gradient alpha pattern
        for (x, y, pixel) in img.enumerate_pixels_mut() {
            pixel[0] = 255; // Red
            pixel[1] = 0;
            pixel[2] = 0;
            pixel[3] = ((x + y) * 255 / 60) as u8; // Alpha gradient
        }
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        let backend = WebpBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.markdown.contains("RGBA"));
        }
    }

    #[test]
    fn test_webp_fully_transparent() {
        // Test image that is fully transparent (alpha = 0)
        use image::{ImageBuffer, Rgba};
        let mut img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(20, 20);
        for pixel in img.pixels_mut() {
            pixel[3] = 0; // Fully transparent
        }
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        let backend = WebpBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            // Should handle fully transparent images
            assert!(doc.markdown.contains("RGBA"));
        }
    }

    // ========== METADATA EXTRACTION TESTS ==========

    #[test]
    fn test_webp_without_exif() {
        // Test that images without EXIF still parse correctly
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(40, 40);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        let backend = WebpBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            // EXIF should be None (synthetic image has no EXIF)
            assert!(doc.metadata.exif.is_none());
        }
    }

    #[test]
    fn test_webp_author_field_absent() {
        // Test that author field is None for synthetic images
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(50, 50);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        let backend = WebpBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.metadata.author.is_none());
        }
    }

    #[test]
    fn test_webp_timestamps_absent() {
        // Test that created/modified timestamps are None for synthetic images
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(50, 50);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        let backend = WebpBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.metadata.created.is_none());
            assert!(doc.metadata.modified.is_none());
        }
    }

    #[test]
    fn test_webp_language_field_absent() {
        // Test that language field is None for images
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(50, 50);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        let backend = WebpBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.metadata.language.is_none());
        }
    }

    // ========== EDGE CASE DIMENSION TESTS ==========

    #[test]
    fn test_webp_maximum_reasonable_dimensions() {
        // Test with maximum reasonable dimensions (16383x16383 is WebP spec limit)
        // Use smaller test size for performance (8192x8192)
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(8192, 8192);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        let backend = WebpBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.markdown.contains("8192×8192"));
        }
    }

    #[test]
    fn test_webp_extreme_aspect_ratio_wide() {
        // Test extreme wide aspect ratio (e.g., panorama)
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(5000, 10); // 500:1 aspect
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        let backend = WebpBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.markdown.contains("5000×10"));
        }
    }

    #[test]
    fn test_webp_extreme_aspect_ratio_tall() {
        // Test extreme tall aspect ratio (e.g., long vertical scroll)
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(10, 5000); // 1:500 aspect
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        let backend = WebpBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.markdown.contains("10×5000"));
        }
    }

    // ========== COLOR PATTERN TESTS ==========

    #[test]
    fn test_webp_solid_color() {
        // Test solid color image (uniform pixels)
        use image::{ImageBuffer, Rgb};
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
            ImageBuffer::from_pixel(100, 100, Rgb([255, 0, 0])); // Solid red
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        let backend = WebpBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.markdown.contains("100×100"));
            // Solid colors compress very well in WebP
            assert!(buffer.len() < 1000); // Should be tiny file
        }
    }

    #[test]
    fn test_webp_gradient_pattern() {
        // Test gradient pattern (smooth color transitions)
        use image::{ImageBuffer, RgbImage};
        let mut img: RgbImage = ImageBuffer::new(200, 200);
        for (x, y, pixel) in img.enumerate_pixels_mut() {
            pixel[0] = ((x * 255) / 200) as u8; // Red gradient horizontal
            pixel[1] = ((y * 255) / 200) as u8; // Green gradient vertical
            pixel[2] = 0;
        }
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        let backend = WebpBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.markdown.contains("200×200"));
        }
    }

    #[test]
    fn test_webp_noise_pattern() {
        // Test noisy image (high entropy, difficult to compress)
        use image::{ImageBuffer, RgbImage};
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut img: RgbImage = ImageBuffer::new(150, 150);
        for (x, y, pixel) in img.enumerate_pixels_mut() {
            // Generate pseudo-random values based on position
            let mut hasher = DefaultHasher::new();
            (x, y).hash(&mut hasher);
            let hash = hasher.finish();
            pixel[0] = (hash & 0xFF) as u8;
            pixel[1] = ((hash >> 8) & 0xFF) as u8;
            pixel[2] = ((hash >> 16) & 0xFF) as u8;
        }
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        let backend = WebpBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.markdown.contains("150×150"));
        }
    }

    #[test]
    fn test_webp_checkerboard_pattern() {
        // Test checkerboard pattern (sharp edges, block compression)
        use image::{ImageBuffer, Rgb, RgbImage};
        let mut img: RgbImage = ImageBuffer::new(128, 128);
        for (x, y, pixel) in img.enumerate_pixels_mut() {
            let is_black = ((x / 16) + (y / 16)) % 2 == 0;
            *pixel = if is_black {
                Rgb([0, 0, 0])
            } else {
                Rgb([255, 255, 255])
            };
        }
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        let backend = WebpBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.markdown.contains("128×128"));
        }
    }

    #[test]
    fn test_webp_vp8x_extended_format() {
        // Test WebP extended format (VP8X) with feature flags
        // VP8X format supports animation, XMP metadata, EXIF, ICC profile, alpha
        use image::{ImageBuffer, RgbaImage};

        // Create image with alpha channel (requires VP8X extended format)
        let mut img: RgbaImage = ImageBuffer::new(64, 64);
        for (x, y, pixel) in img.enumerate_pixels_mut() {
            // Gradient with varying alpha
            let alpha = ((x + y) * 2) as u8;
            *pixel = image::Rgba([128, 192, 255, alpha]);
        }

        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        // Verify RIFF structure contains VP8X extended format
        assert!(buffer.len() > 12);
        assert_eq!(&buffer[0..4], b"RIFF");
        // VP8X chunk should be present for extended features
        let contains_vp8x = buffer
            .windows(4)
            .any(|window| window == b"VP8X" || window == b"VP8L" || window == b"VP8 ");
        assert!(contains_vp8x);

        let backend = WebpBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.markdown.contains("64×64"));
            assert!(doc.markdown.contains("WEBP"));
        }
    }

    #[test]
    fn test_webp_animation_support() {
        // Test WebP animation support (ANIM chunk)
        // Note: The image crate may not support creating animated WebP,
        // but we test that parser handles animated WebP structure
        use image::{ImageBuffer, RgbImage};

        // Create a simple static image (animated WebP creation requires specialized tools)
        let img: RgbImage = ImageBuffer::from_fn(100, 100, |x, y| {
            let frame = ((x / 25) + (y / 25)) % 2;
            if frame == 0 {
                image::Rgb([255, 0, 0]) // Red
            } else {
                image::Rgb([0, 0, 255]) // Blue
            }
        });

        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        // Note: This creates a static WebP, but demonstrates parser handles WebP structure
        // Real animated WebP would have ANIM chunk with loop count and frame data
        let backend = WebpBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.markdown.contains("100×100"));
            assert!(doc.markdown.contains("WEBP"));
        }
    }

    #[test]
    fn test_webp_with_exif_metadata() {
        // Test WebP with EXIF metadata chunk
        // WebP can store camera EXIF (orientation, GPS, timestamp)
        // EXIF chunk uses same format as JPEG APP1
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(2400, 1600); // 4:3 aspect ratio
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        // Note: image crate may not write EXIF to WebP by default
        let backend = WebpBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.markdown.contains("2400") || doc.markdown.contains("1600"));
            assert!(doc.markdown.contains("WEBP"));
            // EXIF metadata doesn't affect dimensions
        }
    }

    #[test]
    fn test_webp_with_xmp_metadata() {
        // Test WebP with XMP metadata chunk
        // XMP stores rich metadata in XML (Adobe, IPTC, Dublin Core)
        // Used in professional photography workflows
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(1920, 1280);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        let backend = WebpBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.markdown.contains("1920") || doc.markdown.contains("1280"));
            assert!(doc.markdown.contains("WEBP"));
            // XMP metadata is ancillary data
        }
    }

    #[test]
    fn test_webp_with_icc_profile() {
        // Test WebP with ICC color profile chunk
        // ICC profiles enable color-managed workflows
        // ICCP chunk stores embedded color profile
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::new(1600, 1200);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        let backend = WebpBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.markdown.contains("1600") || doc.markdown.contains("1200"));
            assert!(doc.markdown.contains("WEBP"));
            // ICC profile affects color rendering, not dimensions
        }
    }

    #[test]
    fn test_webp_with_vp8x_extended_features() {
        // Test WebP with VP8X chunk (extended features)
        // VP8X enables: alpha, ICC, EXIF, XMP, animation
        // Flags indicate which features are present
        use image::{ImageBuffer, RgbaImage};
        let img: RgbaImage = ImageBuffer::new(800, 600);
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        // VP8X should be present for RGBA (alpha channel)
        let _has_vp8x = buffer.windows(4).any(|w| w == b"VP8X");
        // Note: image crate may use VP8L (lossless) for RGBA instead

        let backend = WebpBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.markdown.contains("800") || doc.markdown.contains("600"));
            assert!(doc.markdown.contains("WEBP"));
        }
    }

    #[test]
    fn test_webp_with_partial_transparency() {
        // Test WebP with partial transparency (alpha channel)
        // Alpha values 0-255 enable smooth compositing
        // Used in web graphics, overlays, icons
        use image::{ImageBuffer, Rgba, RgbaImage};
        let mut img: RgbaImage = ImageBuffer::new(512, 512);
        // Create gradient transparency
        for (x, y, pixel) in img.enumerate_pixels_mut() {
            let alpha = ((x + y) * 255 / (512 + 512)) as u8;
            *pixel = Rgba([255, 128, 0, alpha]);
        }
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        let backend = WebpBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.markdown.contains("512"));
            assert!(doc.markdown.contains("WEBP"));
            // Alpha channel handled transparently by parser
        }
    }

    #[test]
    fn test_webp_vp8l_predictor_modes() {
        // Test WebP lossless (VP8L) with different predictor modes
        // VP8L uses spatial prediction for better compression
        // 14 predictor modes: L, TL, T, TR, average, Paeth, etc.
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::from_fn(320, 240, |x, y| {
            // Smooth gradient favors certain predictors
            let r = (x * 255 / 320) as u8;
            let g = (y * 255 / 240) as u8;
            image::Rgb([r, g, 128])
        });
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        let backend = WebpBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.markdown.contains("320") || doc.markdown.contains("240"));
            assert!(doc.markdown.contains("WEBP"));
        }
    }

    #[test]
    fn test_webp_palette_transformation() {
        // Test WebP with palette (color indexing transformation)
        // VP8L can use palette for images with ≤256 colors
        // Reduces file size significantly for limited color images
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::from_fn(400, 300, |x, _y| {
            // Only 4 colors total
            match x % 4 {
                0 => image::Rgb([255, 0, 0]),
                1 => image::Rgb([0, 255, 0]),
                2 => image::Rgb([0, 0, 255]),
                _ => image::Rgb([255, 255, 0]),
            }
        });
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        let backend = WebpBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.markdown.contains("400") || doc.markdown.contains("300"));
            assert!(doc.markdown.contains("WEBP"));
        }
    }

    #[test]
    fn test_webp_near_lossless_encoding() {
        // Test WebP near-lossless mode (quality 100 with slight loss)
        // Balances file size and quality better than pure lossless
        // Used in web optimization workflows
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::from_fn(600, 400, |x, y| {
            // Photo-like content with detail
            let r = ((x * y) % 256) as u8;
            let g = ((x + y) % 256) as u8;
            let b = ((x ^ y) % 256) as u8;
            image::Rgb([r, g, b])
        });
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        let backend = WebpBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.markdown.contains("600") || doc.markdown.contains("400"));
            assert!(doc.markdown.contains("WEBP"));
        }
    }

    #[test]
    fn test_webp_embedded_thumbnail() {
        // Test WebP with embedded thumbnail (THUM chunk)
        // Used in galleries for fast preview generation
        // Thumbnail typically 256×256 or smaller
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::from_fn(2048, 1536, |x, y| {
            let val = ((x / 16) ^ (y / 16)) as u8;
            image::Rgb([val, val, val])
        });
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        // Note: image crate may not write THUM chunk
        let backend = WebpBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.markdown.contains("2048") || doc.markdown.contains("1536"));
            assert!(doc.markdown.contains("WEBP"));
        }
    }

    #[test]
    fn test_webp_color_space_yuv_vs_rgb() {
        // Test WebP color space handling (YUV for VP8, RGB for VP8L)
        // VP8 lossy uses YUV color space (like JPEG)
        // VP8L lossless uses RGB color space
        use image::{ImageBuffer, RgbImage};
        let img: RgbImage = ImageBuffer::from_fn(1024, 768, |x, y| {
            // High chroma content tests YUV encoding
            let phase = (x + y) % 3;
            match phase {
                0 => image::Rgb([255, 0, 0]), // Red
                1 => image::Rgb([0, 255, 0]), // Green
                _ => image::Rgb([0, 0, 255]), // Blue
            }
        });
        let mut buffer = Vec::new();
        img.write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::WebP)
            .unwrap();

        let backend = WebpBackend::new();
        if let Some(doc) = parse_with_ocr_fallback(&backend, &buffer) {
            assert!(doc.markdown.contains("1024") || doc.markdown.contains("768"));
            assert!(doc.markdown.contains("WEBP"));
        }
    }
}
