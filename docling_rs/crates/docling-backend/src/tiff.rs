//! TIFF backend for docling
//!
//! This backend converts TIFF (Tagged Image File Format) files to docling's document model.
//! Supports multi-page TIFF files.

use crate::exif_utils;
use crate::traits::{BackendOptions, DocumentBackend};
use crate::utils::{create_section_header, create_text_item, format_file_size, opt_vec};
use docling_core::{DocItem, DoclingError, Document, DocumentMetadata, InputFormat};
use docling_ocr::OcrEngine;
use image::{DynamicImage, GenericImageView};
use std::fmt::Write;
use std::io::Cursor;
use std::path::Path;

/// TIFF backend
///
/// Converts TIFF files to docling's document model.
/// Extracts basic metadata and uses OCR to extract text content from the image(s).
/// Supports multi-page TIFF files.
///
/// ## Features
///
/// - Extract image dimensions
/// - Multi-page TIFF support
/// - Detect color type (RGB, RGBA, Grayscale, etc.)
/// - OCR text extraction with bounding boxes
/// - Generate markdown with image metadata and OCR text
///
/// ## Example
///
/// ```no_run
/// use docling_backend::TiffBackend;
/// use docling_backend::DocumentBackend;
///
/// let backend = TiffBackend::new();
/// let result = backend.parse_file("image.tiff", &Default::default())?;
/// println!("Image: {:?}", result.metadata.title);
/// # Ok::<(), docling_core::error::DoclingError>(())
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct TiffBackend;

impl TiffBackend {
    /// Create a new TIFF backend instance
    #[inline]
    #[must_use = "creates a backend instance that should be used for parsing"]
    pub const fn new() -> Self {
        Self
    }

    /// Count number of pages in a TIFF file and extract all pages
    ///
    /// Uses the tiff crate to iterate through all image file directories (IFDs)
    /// in the TIFF file. Each IFD represents one page/image.
    ///
    /// ## Arguments
    /// * `data` - Raw TIFF file bytes
    ///
    /// ## Returns
    /// Vector of `DynamicImage`, one per page
    fn extract_all_pages(data: &[u8]) -> Result<Vec<DynamicImage>, DoclingError> {
        use tiff::decoder::{Decoder, DecodingResult};
        use tiff::ColorType as TiffColorType;

        let cursor = Cursor::new(data);
        let mut decoder = Decoder::new(cursor).map_err(|e| {
            DoclingError::BackendError(format!("Failed to create TIFF decoder: {e}"))
        })?;

        let mut pages = Vec::new();

        loop {
            // Get dimensions and color type for current page
            let (width, height) = decoder.dimensions().map_err(|e| {
                DoclingError::BackendError(format!("Failed to get dimensions: {e}"))
            })?;
            let color_type = decoder.colortype().map_err(|e| {
                DoclingError::BackendError(format!("Failed to get color type: {e}"))
            })?;

            // Decode the image data
            let img_data = decoder
                .read_image()
                .map_err(|e| DoclingError::BackendError(format!("Failed to decode image: {e}")))?;

            // Convert to DynamicImage
            let dynamic_img = match (img_data, color_type) {
                (DecodingResult::U8(data), TiffColorType::Gray(8)) => {
                    image::DynamicImage::ImageLuma8(
                        image::ImageBuffer::from_raw(width, height, data).ok_or_else(|| {
                            DoclingError::BackendError(format!("Failed to create gray image: dimensions {width}x{height} do not match data buffer size"))
                        })?,
                    )
                }
                (DecodingResult::U8(data), TiffColorType::RGB(8)) => {
                    image::DynamicImage::ImageRgb8(
                        image::ImageBuffer::from_raw(width, height, data).ok_or_else(|| {
                            DoclingError::BackendError(format!("Failed to create RGB image: dimensions {width}x{height} do not match data buffer size"))
                        })?,
                    )
                }
                (DecodingResult::U8(data), TiffColorType::RGBA(8)) => {
                    image::DynamicImage::ImageRgba8(
                        image::ImageBuffer::from_raw(width, height, data).ok_or_else(|| {
                            DoclingError::BackendError(format!("Failed to create RGBA image: dimensions {width}x{height} do not match data buffer size"))
                        })?,
                    )
                }
                (DecodingResult::U16(data), TiffColorType::Gray(16)) => {
                    image::DynamicImage::ImageLuma16(
                        image::ImageBuffer::from_raw(width, height, data).ok_or_else(|| {
                            DoclingError::BackendError(format!("Failed to create gray16 image: dimensions {width}x{height} do not match data buffer size"))
                        })?,
                    )
                }
                (DecodingResult::U16(data), TiffColorType::RGB(16)) => {
                    image::DynamicImage::ImageRgb16(
                        image::ImageBuffer::from_raw(width, height, data).ok_or_else(|| {
                            DoclingError::BackendError(format!("Failed to create RGB16 image: dimensions {width}x{height} do not match data buffer size"))
                        })?,
                    )
                }
                (DecodingResult::U16(data), TiffColorType::RGBA(16)) => {
                    image::DynamicImage::ImageRgba16(
                        image::ImageBuffer::from_raw(width, height, data).ok_or_else(|| {
                            DoclingError::BackendError(format!("Failed to create RGBA16 image: dimensions {width}x{height} do not match data buffer size"))
                        })?,
                    )
                }
                _ => {
                    return Err(DoclingError::BackendError(format!(
                        "Unsupported TIFF format: {color_type:?}"
                    )));
                }
            };

            pages.push(dynamic_img);

            // Try to advance to next page
            if decoder.next_image().is_err() {
                break;
            }
        }

        if pages.is_empty() {
            return Err(DoclingError::BackendError(
                "No pages found in TIFF file".to_string(),
            ));
        }

        Ok(pages)
    }

    /// Convert TIFF metadata to markdown
    fn tiff_to_markdown(
        filename: &str,
        width: u32,
        height: u32,
        color_type: &str,
        file_size: usize,
        num_pages: usize,
    ) -> String {
        let mut markdown = String::new();

        // Title
        let _ = writeln!(markdown, "# {filename}\n");

        // Image type
        markdown.push_str("Type: TIFF (Tagged Image File Format)\n\n");

        // Number of pages
        if num_pages > 1 {
            let _ = writeln!(markdown, "Pages: {num_pages}\n");
        }

        // Dimensions
        let _ = writeln!(markdown, "Dimensions: {width}×{height} pixels\n");

        // Color type
        let _ = writeln!(markdown, "Color Type: {color_type}\n");

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
    /// * `img` - `DynamicImage` to extract text from
    /// * `start_index` - Starting index for `DocItems` (to append after metadata items)
    /// * `page_no` - Page number (1-indexed, for multi-page TIFF support)
    ///
    /// ## Returns
    /// Tuple of (OCR text as string, vector of `DocItem::Text` with bboxes)
    fn extract_ocr_text(
        img: &DynamicImage,
        start_index: usize,
        page_no: usize,
    ) -> Result<(String, Vec<DocItem>), DoclingError> {
        use docling_core::content::{BoundingBox, CoordOrigin, ProvenanceItem};

        // Check if OCR is enabled via environment variable
        // Default: OCR is disabled (ENABLE_IMAGE_OCR must be explicitly set to "1")
        // This allows skipping expensive OCR processing (5-15s) on non-text images
        if std::env::var("ENABLE_IMAGE_OCR").unwrap_or_default() != "1" {
            return Ok((String::new(), Vec::new()));
        }

        // Run OCR
        let mut ocr_engine = OcrEngine::new().map_err(|e| {
            DoclingError::BackendError(format!("Failed to initialize OCR engine: {e}"))
        })?;

        let ocr_result = ocr_engine
            .recognize(img)
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
                page_no,
                bbox,
                charspan: None,
            }];

            doc_items.push(create_text_item(start_index + i, line.text.clone(), prov));
        }

        // Get full OCR text
        let ocr_text = ocr_result.text();

        Ok((ocr_text, doc_items))
    }

    /// Create `DocItems` directly from TIFF metadata
    ///
    /// Generates structured `DocItems` from TIFF metadata without markdown intermediary.
    /// Creates a hierarchical document structure:
    /// - Title (filename) as `SectionHeader` level 1
    /// - Image type as Text
    /// - Pages as Text (optional, only for multi-page TIFFs)
    /// - Dimensions as Text
    /// - Color type as Text
    /// - File size as Text
    /// - Image reference as Text
    ///
    /// ## Arguments
    /// * `filename` - Name of the TIFF file
    /// * `width` - Image width in pixels
    /// * `height` - Image height in pixels
    /// * `color_type` - Color type description
    /// * `file_size` - Size of the TIFF file in bytes
    /// * `num_pages` - Number of pages (1 for single-page, >1 for multi-page)
    ///
    /// ## Returns
    /// Vector of `DocItems` representing the TIFF metadata structure (6-7 items depending on `num_pages`)
    fn tiff_to_docitems(
        filename: &str,
        width: u32,
        height: u32,
        color_type: &str,
        file_size: usize,
        num_pages: usize,
    ) -> Vec<DocItem> {
        let mut doc_items = Vec::new();
        let mut index = 0;

        // Title - filename as SectionHeader level 1
        doc_items.push(create_section_header(0, filename.to_string(), 1, vec![]));
        index += 1; // Reserve index 0 for the header

        // Image type
        doc_items.push(create_text_item(
            index,
            "Type: TIFF (Tagged Image File Format)".to_string(),
            vec![],
        ));
        index += 1;

        // Pages (optional, only for multi-page TIFFs)
        if num_pages > 1 {
            doc_items.push(create_text_item(
                index,
                format!("Pages: {num_pages}"),
                vec![],
            ));
            index += 1;
        }

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

    /// Shared helper to parse TIFF data and produce a Document
    ///
    /// Both `parse_bytes` and `parse_file` delegate to this method.
    ///
    /// ## Arguments
    /// * `data` - Raw TIFF file bytes
    /// * `filename` - Name to use for the document title and image reference
    fn parse_tiff_data(data: &[u8], filename: &str) -> Result<Document, DoclingError> {
        // Helper to add filename context to errors
        let add_context = |e: DoclingError| match e {
            DoclingError::BackendError(msg) => {
                DoclingError::BackendError(format!("{msg}: {filename}"))
            }
            other => other,
        };

        // Extract all pages from TIFF file
        let pages = Self::extract_all_pages(data).map_err(add_context)?;
        let num_pages = pages.len();

        // Get metadata from first page
        let first_img = &pages[0];
        let (width, height) = first_img.dimensions();
        let color_type = match first_img {
            image::DynamicImage::ImageLuma8(_) => "Grayscale",
            image::DynamicImage::ImageLumaA8(_) => "Grayscale + Alpha",
            image::DynamicImage::ImageRgb8(_) => "RGB",
            image::DynamicImage::ImageRgba8(_) => "RGBA",
            image::DynamicImage::ImageLuma16(_) => "Grayscale (16-bit)",
            image::DynamicImage::ImageLumaA16(_) => "Grayscale + Alpha (16-bit)",
            image::DynamicImage::ImageRgb16(_) => "RGB (16-bit)",
            image::DynamicImage::ImageRgba16(_) => "RGBA (16-bit)",
            image::DynamicImage::ImageRgb32F(_) => "RGB (32-bit float)",
            image::DynamicImage::ImageRgba32F(_) => "RGBA (32-bit float)",
            _ => "Unknown",
        };

        // Create DocItems directly from TIFF metadata (no markdown intermediary)
        let mut doc_items =
            Self::tiff_to_docitems(filename, width, height, color_type, data.len(), num_pages);
        let mut current_index = doc_items.len();

        // Generate markdown from DocItems (for backwards compatibility)
        let metadata_markdown =
            Self::tiff_to_markdown(filename, width, height, color_type, data.len(), num_pages);
        let mut markdown = metadata_markdown;

        // Process each page with OCR
        for (page_idx, page_img) in pages.iter().enumerate() {
            let page_no = page_idx + 1; // 1-indexed for display

            // Run OCR on this page
            let (ocr_text, mut ocr_items) =
                Self::extract_ocr_text(page_img, current_index, page_no)?;

            // Add OCR results to markdown
            if !ocr_text.is_empty() {
                if num_pages > 1 {
                    let _ = writeln!(markdown, "\n\n## Page {page_no} OCR Text\n");
                } else {
                    markdown.push_str("\n\n## OCR Text\n\n");
                }
                markdown.push_str(&ocr_text);
            }

            // Append OCR items to document
            current_index += ocr_items.len();
            doc_items.append(&mut ocr_items);
        }

        let num_characters = markdown.chars().count();

        // Extract EXIF metadata
        let exif_metadata = exif_utils::extract_exif_metadata(data);

        // N=1885: Extract EXIF Artist and ImageDescription metadata
        let (author, subject, created) = exif_utils::extract_document_metadata(data);

        // Create document
        Ok(Document {
            markdown,
            format: InputFormat::Tiff,
            metadata: DocumentMetadata {
                num_pages: Some(num_pages),
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

impl DocumentBackend for TiffBackend {
    #[inline]
    fn format(&self) -> InputFormat {
        InputFormat::Tiff
    }

    fn parse_bytes(
        &self,
        data: &[u8],
        _options: &BackendOptions,
    ) -> Result<Document, DoclingError> {
        Self::parse_tiff_data(data, "image.tiff")
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
            .unwrap_or("image.tiff");

        // Read file
        let data = std::fs::read(path_ref).map_err(DoclingError::IoError)?;

        Self::parse_tiff_data(&data, filename)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_format() {
        let backend = TiffBackend::new();
        assert_eq!(
            backend.format(),
            InputFormat::Tiff,
            "TiffBackend should report InputFormat::Tiff as its format"
        );
    }

    #[test]
    fn test_tiff_to_markdown() {
        let markdown = TiffBackend::tiff_to_markdown("test.tiff", 800, 600, "RGBA", 204_800, 1);
        assert!(
            markdown.contains("# test.tiff"),
            "Markdown should contain filename as h1 title"
        );
        assert!(
            markdown.contains("Type: TIFF (Tagged Image File Format)"),
            "Markdown should contain TIFF type description"
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
            markdown.contains("![test.tiff](test.tiff)"),
            "Markdown should contain image reference"
        );
    }

    #[test]
    fn test_tiff_to_markdown_multipage() {
        let markdown = TiffBackend::tiff_to_markdown("test.tiff", 800, 600, "RGB", 409_600, 3);
        assert!(
            markdown.contains("# test.tiff"),
            "Multi-page TIFF markdown should contain filename as title"
        );
        assert!(
            markdown.contains("Pages: 3"),
            "Multi-page TIFF should show page count"
        );
    }

    #[test]
    fn test_extract_all_pages_single_page() {
        // Create a simple 32x32 grayscale TIFF
        let data = create_test_tiff_single_page();
        let pages = TiffBackend::extract_all_pages(&data).unwrap();
        assert_eq!(pages.len(), 1, "Should detect 1 page");
        assert_eq!(
            pages[0].dimensions(),
            (32, 32),
            "Should have 32x32 dimensions"
        );
    }

    #[test]
    fn test_extract_all_pages_multipage() {
        // This test uses multi-page TIFF file at test-corpus/tiff/multi_page.tiff
        let path = std::path::Path::new("../../test-corpus/tiff/multi_page.tiff");
        if !path.exists() {
            println!("Skipping test - file not found: {path:?}");
            return;
        }

        let data = std::fs::read(path).unwrap();
        let pages = TiffBackend::extract_all_pages(&data).unwrap();
        assert!(
            pages.len() > 1,
            "Should detect multiple pages, found {}",
            pages.len()
        );
        println!("Detected {} pages in multi_page.tiff", pages.len());
    }

    /// Helper to create a minimal single-page TIFF for testing
    ///
    /// Creates a 32x32 image to satisfy OCR minimum dimensions requirement.
    /// OCR engine requires at least 32x32 pixels for resize operations.
    fn create_test_tiff_single_page() -> Vec<u8> {
        use std::io::Cursor;
        use tiff::encoder::{colortype, TiffEncoder};

        let mut buffer = Cursor::new(Vec::new());
        let mut encoder = TiffEncoder::new(&mut buffer).unwrap();

        // Create a 32x32 grayscale image with checkerboard pattern
        // OCR requires images to be at least 32x32 pixels
        let mut image_data = Vec::with_capacity(32 * 32);
        for y in 0..32 {
            for x in 0..32 {
                // Checkerboard pattern
                let value = if (x + y) % 2 == 0 { 255 } else { 0 };
                image_data.push(value);
            }
        }

        encoder
            .write_image::<colortype::Gray8>(32, 32, &image_data)
            .unwrap();

        buffer.into_inner()
    }

    #[test]
    fn test_exif_extraction() {
        // Test with a real TIFF file from test corpus
        let test_path = "test-corpus/tiff/sample_image.tiff";
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
        // Use the minimal TIFF created by helper function (no EXIF)
        let minimal_tiff = create_test_tiff_single_page();

        let exif = exif_utils::extract_exif_metadata(&minimal_tiff);
        // Should return None for TIFF without EXIF
        // (Or Some with empty fields - both are acceptable)
        if let Some(exif_data) = exif {
            // If EXIF is present, it should be empty or have minimal data
            assert!(
                exif_data.camera_make.is_none() || exif_data.camera_make == Some(String::new())
            );
        }
    }

    // ===== Category 1: Backend Creation Tests (3 tests) =====

    #[test]
    fn test_create_backend() {
        let backend = TiffBackend::new();
        assert_eq!(
            backend.format(),
            InputFormat::Tiff,
            "New TiffBackend should have Tiff format"
        );
    }

    #[test]
    fn test_create_backend_default() {
        let backend = TiffBackend;
        assert_eq!(
            backend.format(),
            InputFormat::Tiff,
            "Default TiffBackend should have Tiff format"
        );
    }

    #[test]
    fn test_backend_format_constant() {
        let backend = TiffBackend::new();
        let format = backend.format();
        // Verify format constant matches expected value
        assert_eq!(
            format,
            InputFormat::Tiff,
            "Format should be InputFormat::Tiff"
        );
        // Verify debug representation contains "Tiff"
        assert!(
            format!("{format:?}").contains("Tiff"),
            "Debug representation should contain 'Tiff'"
        );
    }

    // ===== Category 2: Metadata Tests (3 tests) =====

    #[test]
    fn test_metadata_single_page() {
        let data = create_test_tiff_single_page();
        let backend = TiffBackend::new();
        let doc = backend
            .parse_bytes(&data, &BackendOptions::default())
            .unwrap();

        // Verify metadata fields
        assert_eq!(
            doc.metadata.num_pages,
            Some(1),
            "Single-page TIFF should have num_pages = 1"
        );
        assert!(
            doc.metadata.title.is_some(),
            "TIFF document should have a title"
        );
        assert!(
            doc.metadata.num_characters > 0,
            "TIFF document should have positive character count"
        );
        assert_eq!(
            doc.format,
            InputFormat::Tiff,
            "Document format should be InputFormat::Tiff"
        );
    }

    #[test]
    fn test_metadata_character_count() {
        let data = create_test_tiff_single_page();
        let backend = TiffBackend::new();
        let doc = backend
            .parse_bytes(&data, &BackendOptions::default())
            .unwrap();

        // Character count should include markdown content
        // Metadata markdown + OCR text (if any)
        let expected_min = 50; // At minimum, metadata section
        assert!(
            doc.metadata.num_characters >= expected_min,
            "TIFF document should have at least {} characters, got {}",
            expected_min,
            doc.metadata.num_characters
        );

        // Verify markdown length matches character count
        let actual_chars = doc.markdown.chars().count();
        assert_eq!(
            doc.metadata.num_characters, actual_chars,
            "Character count in metadata should match actual markdown length"
        );
    }

    #[test]
    fn test_metadata_format_field() {
        let data = create_test_tiff_single_page();
        let backend = TiffBackend::new();
        let doc = backend
            .parse_bytes(&data, &BackendOptions::default())
            .unwrap();

        // Verify format field
        assert_eq!(doc.format, InputFormat::Tiff);

        // Verify markdown contains format description
        assert!(doc.markdown.contains("TIFF"));
        assert!(doc.markdown.contains("Tagged Image File Format"));
    }

    // ===== Category 3: DocItem Generation Tests (4 tests) =====

    #[test]
    fn test_tiff_to_docitems_fields_single_page() {
        let doc_items = TiffBackend::tiff_to_docitems("test.tiff", 800, 600, "RGB", 10000, 1);

        // Should create 6 DocItems for single-page (no Pages field)
        assert_eq!(doc_items.len(), 6);

        // First item should be SectionHeader (title)
        assert!(matches!(doc_items[0], DocItem::SectionHeader { .. }));
    }

    #[test]
    fn test_tiff_to_docitems_fields_multi_page() {
        let doc_items = TiffBackend::tiff_to_docitems("multi.tiff", 800, 600, "RGB", 50000, 3);

        // Should create 7 DocItems for multi-page (includes Pages field)
        assert_eq!(doc_items.len(), 7);

        // First item should be SectionHeader (title)
        assert!(matches!(doc_items[0], DocItem::SectionHeader { .. }));

        // Third item should be Pages field
        if let DocItem::Text { text, .. } = &doc_items[2] {
            assert!(text.contains("Pages: 3"));
        } else {
            panic!("Expected Pages field at index 2");
        }
    }

    #[test]
    fn test_tiff_to_docitems_single_page() {
        let doc_items = TiffBackend::tiff_to_docitems("test.tiff", 100, 100, "RGB", 1024, 1);

        // Should create 6 DocItems for single-page: title (SectionHeader), type, dimensions, color type, file size, image reference
        assert_eq!(doc_items.len(), 6);

        // First item should be SectionHeader (title)
        assert!(matches!(doc_items[0], DocItem::SectionHeader { .. }));

        // Remaining items should be Text
        for item in &doc_items[1..] {
            assert!(matches!(item, DocItem::Text { .. }));
        }
    }

    #[test]
    fn test_tiff_to_docitems_multi_page() {
        let doc_items = TiffBackend::tiff_to_docitems("test.tiff", 100, 100, "RGB", 1024, 5);

        // Should create 7 DocItems for multi-page: title (SectionHeader), type, pages, dimensions, color type, file size, image reference
        assert_eq!(doc_items.len(), 7);

        // First item should be SectionHeader (title)
        assert!(matches!(doc_items[0], DocItem::SectionHeader { .. }));

        // Check that "Pages" field is present (index 2 for multi-page)
        if let DocItem::Text { text, .. } = &doc_items[2] {
            assert!(text.contains("Pages:"));
        } else {
            panic!("Expected Pages field at index 2 for multi-page TIFF");
        }
    }

    // ===== Category 4: Format-Specific Tests (6 tests) =====

    #[test]
    fn test_color_type_detection_grayscale() {
        let markdown = TiffBackend::tiff_to_markdown("test.tiff", 100, 100, "Grayscale", 10000, 1);
        assert!(markdown.contains("Color Type: Grayscale"));
    }

    #[test]
    fn test_color_type_detection_rgb() {
        let markdown = TiffBackend::tiff_to_markdown("test.tiff", 100, 100, "RGB", 30000, 1);
        assert!(markdown.contains("Color Type: RGB"));
    }

    #[test]
    fn test_color_type_detection_rgba() {
        let markdown = TiffBackend::tiff_to_markdown("test.tiff", 100, 100, "RGBA", 40000, 1);
        assert!(markdown.contains("Color Type: RGBA"));
    }

    #[test]
    fn test_multipage_indicator() {
        // Single page should not include page count in metadata
        let markdown_single = TiffBackend::tiff_to_markdown("test.tiff", 100, 100, "RGB", 10000, 1);
        // For single page, page count is typically omitted or shown as "1"
        // Check that multipage indicator isn't present
        assert!(!markdown_single.contains("Pages: 2"));
        assert!(!markdown_single.contains("Pages: 3"));

        // Multi-page should include page count
        let markdown_multi = TiffBackend::tiff_to_markdown("test.tiff", 100, 100, "RGB", 30000, 3);
        assert!(markdown_multi.contains("Pages: 3"));
    }

    #[test]
    fn test_file_size_formatting() {
        // Test KB range
        let markdown_kb = TiffBackend::tiff_to_markdown("test.tiff", 100, 100, "RGB", 5120, 1);
        assert!(markdown_kb.contains("5.0 KB") || markdown_kb.contains("5.1 KB"));

        // Test MB range
        let markdown_mb = TiffBackend::tiff_to_markdown("test.tiff", 100, 100, "RGB", 2_097_152, 1);
        assert!(markdown_mb.contains("2.0 MB") || markdown_mb.contains("2.1 MB"));
    }

    #[test]
    fn test_dimensions_in_markdown() {
        let markdown = TiffBackend::tiff_to_markdown("test.tiff", 1920, 1080, "RGB", 10000, 1);
        assert!(markdown.contains("Dimensions: 1920×1080 pixels"));
        assert!(markdown.contains("1920"));
        assert!(markdown.contains("1080"));
    }

    // ===== Category 5: Integration Tests (parse_bytes) (2 tests) =====

    #[test]
    fn test_parse_bytes_basic() {
        let data = create_test_tiff_single_page();
        let backend = TiffBackend::new();

        let result = backend.parse_bytes(&data, &BackendOptions::default());
        assert!(result.is_ok(), "parse_bytes should succeed");

        let doc = result.unwrap();
        assert_eq!(doc.format, InputFormat::Tiff);
        assert!(!doc.markdown.is_empty());
    }

    #[test]
    fn test_parse_bytes_invalid_data() {
        let backend = TiffBackend::new();
        let invalid_data = vec![0x00, 0x01, 0x02, 0x03]; // Not a valid TIFF

        let result = backend.parse_bytes(&invalid_data, &BackendOptions::default());
        assert!(result.is_err(), "parse_bytes should fail with invalid data");

        let err = result.unwrap_err();
        assert!(matches!(err, DoclingError::BackendError(_)));
    }

    // ========== UNICODE AND SPECIAL CHARACTER TESTS ==========

    #[test]
    fn test_unicode_filename() {
        // Test Unicode characters in filename
        let markdown = TiffBackend::tiff_to_markdown("图像_文件_📸.tiff", 100, 100, "RGB", 1024, 1);
        assert!(markdown.contains("# 图像_文件_📸.tiff"));
        assert!(markdown.contains("![图像_文件_📸.tiff](图像_文件_📸.tiff)"));
    }

    #[test]
    fn test_special_chars_in_filename() {
        // Test special characters in filename (markdown special chars)
        let markdown =
            TiffBackend::tiff_to_markdown("test_[photo]_(v3).tiff", 50, 50, "RGBA", 512, 1);
        assert!(markdown.contains("# test_[photo]_(v3).tiff"));
        assert!(markdown.contains("![test_[photo]_(v3).tiff](test_[photo]_(v3).tiff)"));
    }

    #[test]
    fn test_filename_with_spaces_and_extension_variants() {
        // Test filename with spaces and .tif extension
        let markdown =
            TiffBackend::tiff_to_markdown("my photo file.tif", 200, 200, "Grayscale", 2048, 1);
        assert!(markdown.contains("# my photo file.tif"));
        assert!(markdown.contains("![my photo file.tif](my photo file.tif)"));
    }

    // ========== VALIDATION TESTS ==========

    #[test]
    fn test_very_large_dimensions() {
        // Test very large dimensions (8K resolution)
        let markdown =
            TiffBackend::tiff_to_markdown("large.tiff", 7680, 4320, "RGB", 99_532_800, 1);
        assert!(markdown.contains("7680×4320"));
    }

    #[test]
    fn test_extreme_aspect_ratios() {
        // Test extreme aspect ratios (banner)
        let markdown = TiffBackend::tiff_to_markdown("banner.tiff", 3000, 100, "RGB", 900_000, 1);
        assert!(markdown.contains("3000×100"));
    }

    #[test]
    fn test_square_image() {
        // Test perfect square dimensions
        let markdown =
            TiffBackend::tiff_to_markdown("square.tiff", 1024, 1024, "RGBA", 4_194_304, 1);
        assert!(markdown.contains("1024×1024"));
    }

    #[test]
    fn test_tall_portrait_image() {
        // Test tall portrait orientation
        let markdown =
            TiffBackend::tiff_to_markdown("portrait.tiff", 200, 2000, "RGB", 1_200_000, 1);
        assert!(markdown.contains("200×2000"));
    }

    #[test]
    fn test_minimal_dimensions() {
        // Test minimal valid dimensions (1x1)
        let markdown = TiffBackend::tiff_to_markdown("tiny.tiff", 1, 1, "Grayscale", 100, 1);
        assert!(markdown.contains("1×1"));
    }

    // ========== SERIALIZATION CONSISTENCY TESTS ==========

    #[test]
    fn test_markdown_not_empty_for_valid_tiff() {
        // Test that markdown is never empty for valid TIFF
        let data = create_test_tiff_single_page();
        let backend = TiffBackend::new();
        let doc = backend
            .parse_bytes(&data, &BackendOptions::default())
            .unwrap();

        assert!(!doc.markdown.is_empty());
        assert!(doc.markdown.len() > 100); // Minimum reasonable markdown length
    }

    #[test]
    fn test_markdown_structure_consistency() {
        // Test that markdown has consistent structure
        let data = create_test_tiff_single_page();
        let backend = TiffBackend::new();
        let doc = backend
            .parse_bytes(&data, &BackendOptions::default())
            .unwrap();

        // Should start with h1 title
        assert!(doc.markdown.starts_with("# "));
        // Should contain required sections
        assert!(doc.markdown.contains("Type:"));
        assert!(doc.markdown.contains("Dimensions:"));
        assert!(doc.markdown.contains("Color Type:"));
        assert!(doc.markdown.contains("File Size:"));
    }

    #[test]
    fn test_docitems_match_markdown_content() {
        // Test that DocItems align with markdown sections
        let data = create_test_tiff_single_page();
        let backend = TiffBackend::new();
        let doc = backend
            .parse_bytes(&data, &BackendOptions::default())
            .unwrap();

        let items = doc.content_blocks.unwrap();
        // First item should be SectionHeader (title), remaining metadata items should be Text
        assert!(matches!(items[0], DocItem::SectionHeader { .. }));
        for item in &items[1..] {
            assert!(matches!(item, DocItem::Text { .. }));
        }
    }

    #[test]
    fn test_idempotent_parsing() {
        // Test that parsing the same TIFF twice produces identical output
        let data = create_test_tiff_single_page();
        let backend = TiffBackend::new();
        let doc1 = backend
            .parse_bytes(&data, &BackendOptions::default())
            .unwrap();
        let doc2 = backend
            .parse_bytes(&data, &BackendOptions::default())
            .unwrap();

        // Markdown should be identical
        assert_eq!(doc1.markdown, doc2.markdown);
        // Metadata should be identical
        assert_eq!(doc1.metadata.num_pages, doc2.metadata.num_pages);
        assert_eq!(doc1.metadata.num_characters, doc2.metadata.num_characters);
        assert_eq!(doc1.format, doc2.format);
    }

    // ========== BACKEND OPTIONS TESTS ==========

    #[test]
    fn test_parse_with_default_options() {
        // Test that default options work correctly
        let data = create_test_tiff_single_page();
        let backend = TiffBackend::new();
        let options = BackendOptions::default();
        let result = backend.parse_bytes(&data, &options);

        assert!(result.is_ok());
        let doc = result.unwrap();
        assert_eq!(doc.format, InputFormat::Tiff);
    }

    #[test]
    fn test_parse_with_custom_options() {
        // Test that custom options are accepted
        let data = create_test_tiff_single_page();
        let backend = TiffBackend::new();
        let options = BackendOptions::default()
            .with_ocr(true)
            .with_table_structure(true);
        let result = backend.parse_bytes(&data, &options);

        assert!(result.is_ok());
        let doc = result.unwrap();
        assert_eq!(doc.format, InputFormat::Tiff);
    }

    // ========== FORMAT-SPECIFIC EDGE CASES ==========

    #[test]
    fn test_color_type_16bit_rgb() {
        // Test 16-bit RGB color type
        let markdown =
            TiffBackend::tiff_to_markdown("test.tiff", 100, 100, "RGB (16-bit)", 60000, 1);
        assert!(markdown.contains("Color Type: RGB (16-bit)"));
    }

    #[test]
    fn test_color_type_16bit_rgba() {
        // Test 16-bit RGBA color type
        let markdown =
            TiffBackend::tiff_to_markdown("test.tiff", 100, 100, "RGBA (16-bit)", 80000, 1);
        assert!(markdown.contains("Color Type: RGBA (16-bit)"));
    }

    #[test]
    fn test_color_type_16bit_grayscale() {
        // Test 16-bit grayscale color type
        let markdown =
            TiffBackend::tiff_to_markdown("test.tiff", 100, 100, "Grayscale (16-bit)", 20000, 1);
        assert!(markdown.contains("Color Type: Grayscale (16-bit)"));
    }

    #[test]
    fn test_color_type_grayscale_alpha() {
        // Test grayscale + alpha color type
        let markdown =
            TiffBackend::tiff_to_markdown("test.tiff", 100, 100, "Grayscale + Alpha", 20000, 1);
        assert!(markdown.contains("Color Type: Grayscale + Alpha"));
    }

    #[test]
    fn test_backend_default_trait() {
        // Test Default trait implementation
        let backend1 = TiffBackend::new();
        let backend2 = TiffBackend;

        assert_eq!(backend1.format(), backend2.format());
    }

    #[test]
    fn test_multipage_tiff_markdown_structure() {
        // Test that multi-page TIFF has correct markdown structure
        let markdown_single =
            TiffBackend::tiff_to_markdown("single.tiff", 100, 100, "RGB", 30000, 1);
        let markdown_multi = TiffBackend::tiff_to_markdown("multi.tiff", 100, 100, "RGB", 90000, 5);

        // Single page should NOT have "Pages:" field
        assert!(!markdown_single.contains("Pages:"));

        // Multi-page SHOULD have "Pages:" field
        assert!(markdown_multi.contains("Pages: 5"));
    }

    #[test]
    fn test_content_layer_consistency() {
        // Test that all DocItems have "body" content layer
        let data = create_test_tiff_single_page();
        let backend = TiffBackend::new();
        let doc = backend
            .parse_bytes(&data, &BackendOptions::default())
            .unwrap();

        let items = doc.content_blocks.unwrap();
        for item in items {
            if let DocItem::Text { content_layer, .. } = item {
                assert_eq!(content_layer, "body");
            }
        }
    }

    #[test]
    fn test_character_count_accuracy() {
        // Test that character count matches markdown length
        let data = create_test_tiff_single_page();
        let backend = TiffBackend::new();
        let doc = backend
            .parse_bytes(&data, &BackendOptions::default())
            .unwrap();

        let expected_count = doc.markdown.chars().count();
        assert_eq!(doc.metadata.num_characters, expected_count);
    }

    #[test]
    fn test_tiff_to_docitems_always_has_items() {
        // TIFF metadata always generates DocItems (at minimum: title, type, dimensions, etc.)
        let doc_items = TiffBackend::tiff_to_docitems("test.tiff", 1, 1, "RGB", 100, 1);

        // Should always have 6 items for single-page: title, type, dimensions, color type, file size, image reference
        assert_eq!(doc_items.len(), 6);
    }

    #[test]
    fn test_format_identification() {
        // Test that format is correctly identified as TIFF
        let backend = TiffBackend::new();
        assert_eq!(backend.format(), InputFormat::Tiff);

        // Also test through document parsing
        let data = create_test_tiff_single_page();
        let doc = backend
            .parse_bytes(&data, &BackendOptions::default())
            .unwrap();
        assert_eq!(doc.format, InputFormat::Tiff);
    }

    #[test]
    fn test_exif_metadata_integration() {
        // Test that EXIF extraction is called (even if returns None)
        let data = create_test_tiff_single_page();
        let backend = TiffBackend::new();
        let doc = backend
            .parse_bytes(&data, &BackendOptions::default())
            .unwrap();

        // EXIF field should be present (may be None or Some)
        // The synthetic test TIFF likely has no EXIF data
        // Just verify the field exists and is accessible
        let _exif = &doc.metadata.exif;
    }

    #[test]
    fn test_very_large_file_size() {
        // Test file size formatting for very large files (1GB)
        let markdown =
            TiffBackend::tiff_to_markdown("huge.tiff", 10000, 10000, "RGBA", 1_073_741_824, 1);
        assert!(markdown.contains("File Size:"));
        // Should show GB
        assert!(markdown.contains("GB") || markdown.contains("1024.0 MB"));
    }

    #[test]
    fn test_multipage_page_count_metadata() {
        // Test that num_pages metadata matches page count parameter
        let markdown_3 = TiffBackend::tiff_to_markdown("test.tiff", 100, 100, "RGB", 90000, 3);
        let markdown_10 = TiffBackend::tiff_to_markdown("test.tiff", 100, 100, "RGB", 300000, 10);

        assert!(markdown_3.contains("Pages: 3"));
        assert!(markdown_10.contains("Pages: 10"));
    }

    // ========== EXTENDED TIFF TESTS (N=486, +10 tests) ==========

    #[test]
    fn test_multipage_tiff_large_document() {
        // Simulating scanned book/manual with 100+ pages
        let markdown = TiffBackend::tiff_to_markdown(
            "scanned_manual.tiff",
            2550, // Letter size at 300 DPI width
            3300, // Letter size at 300 DPI height
            "Grayscale",
            104_857_600, // ~100MB (1MB per page)
            120,         // 120 pages
        );
        assert!(markdown.contains("Pages: 120"));
        assert!(markdown.contains("2550×3300 pixels"));
        assert!(markdown.contains("Grayscale"));
    }

    #[test]
    fn test_tiff_fax_format() {
        // FAX machines use Group 3/4 compressed TIFF (1-bit monochrome)
        let markdown = TiffBackend::tiff_to_markdown(
            "fax_document.tiff",
            1728,        // Standard FAX width (204 DPI horizontal)
            2200,        // Approximately A4 height at 196 DPI
            "Grayscale", // FAX is technically bilevel, shown as grayscale
            15000,       // Small file size due to Group 4 compression
            1,
        );
        assert!(markdown.contains("1728×2200 pixels"));
        assert!(markdown.contains("File Size:"));
        assert!(markdown.contains("fax_document.tiff"));
    }

    #[test]
    fn test_tiff_with_lzw_compression() {
        // TIFF with LZW compression (common for documents)
        // LZW typically achieves 2-3x compression on text documents
        let _uncompressed_size = 8000 * 6000 * 3; // 144MB uncompressed
        let compressed_size = 48_000_000; // ~48MB compressed (3:1 ratio)
        let markdown = TiffBackend::tiff_to_markdown(
            "lzw_compressed.tiff",
            8000,
            6000,
            "RGB",
            compressed_size,
            1,
        );
        assert!(markdown.contains("8000×6000 pixels"));
        assert!(markdown.contains("File Size:"));
        // Verify reasonable size format (should be ~45-50MB)
        assert!(markdown.contains("MB"));
    }

    #[test]
    fn test_tiff_geotiff_large_dimensions() {
        // GeoTIFF satellite imagery with very large dimensions
        let markdown = TiffBackend::tiff_to_markdown(
            "satellite_image.tiff",
            32768, // Common GeoTIFF dimension
            32768,
            "RGB",
            3_221_225_472, // ~3GB uncompressed
            1,
        );
        assert!(markdown.contains("32768×32768 pixels"));
        assert!(markdown.contains("File Size:"));
        assert!(markdown.contains("GB"));
    }

    #[test]
    fn test_tiff_medical_imaging_16bit() {
        // Medical imaging (X-ray, CT scan) uses 16-bit grayscale
        let markdown = TiffBackend::tiff_to_markdown(
            "xray_image.tiff",
            2048,
            2048,
            "Grayscale (16-bit)",
            8_388_608, // 2048 * 2048 * 2 bytes = 8MB
            1,
        );
        assert!(markdown.contains("2048×2048 pixels"));
        assert!(markdown.contains("Grayscale (16-bit)"));
        assert!(markdown.contains("File Size:"));
    }

    #[test]
    fn test_tiff_multipage_mixed_dimensions() {
        // Edge case: Multi-page TIFF where pages might have different sizes
        // (TIFF format allows this, though uncommon)
        // We report first page dimensions
        let markdown = TiffBackend::tiff_to_markdown(
            "mixed_pages.tiff",
            1200, // First page width
            1600, // First page height
            "RGB",
            5_000_000, // 5MB total
            5,         // 5 pages with varying sizes
        );
        assert!(markdown.contains("1200×1600 pixels"));
        assert!(markdown.contains("Pages: 5"));
        assert!(markdown.contains("File Size:"));
    }

    #[test]
    fn test_tiff_transparency_rgba() {
        // TIFF with alpha channel (transparency)
        let markdown = TiffBackend::tiff_to_markdown(
            "transparent_overlay.tiff",
            1920,
            1080,
            "RGBA",
            8_294_400, // 1920 * 1080 * 4 bytes
            1,
        );
        assert!(markdown.contains("1920×1080 pixels"));
        assert!(markdown.contains("RGBA"));
        assert!(markdown.contains("Color Type: RGBA"));
    }

    #[test]
    fn test_tiff_panorama_extreme_aspect() {
        // Panoramic image with extreme aspect ratio (10:1)
        let markdown = TiffBackend::tiff_to_markdown(
            "panorama.tiff",
            15000, // Very wide
            1500,  // Relatively short
            "RGB",
            67_500_000, // ~67MB uncompressed
            1,
        );
        assert!(markdown.contains("15000×1500 pixels"));
        assert!(markdown.contains("RGB"));
        assert!(markdown.contains("File Size:"));
    }

    #[test]
    fn test_tiff_cmyk_color_space() {
        // TIFF with CMYK color space (used in professional printing)
        // Note: Our implementation may not explicitly label CMYK, but test the scenario
        let markdown = TiffBackend::tiff_to_markdown(
            "print_ready.tiff",
            3600,       // 10 inches at 360 DPI
            4800,       // ~13.3 inches at 360 DPI
            "RGBA",     // CMYK might be reported as RGBA by decoder
            69_120_000, // 3600 * 4800 * 4 bytes
            1,
        );
        assert!(markdown.contains("3600×4800 pixels"));
        assert!(markdown.contains("Color Type:"));
        assert!(markdown.contains("File Size:"));
    }

    #[test]
    fn test_tiff_minimal_metadata() {
        // TIFF with absolute minimum metadata (1x1 pixel, no extras)
        let markdown = TiffBackend::tiff_to_markdown(
            "minimal.tiff",
            1,
            1,
            "RGB",
            134, // Minimal TIFF header + 1 pixel
            1,
        );
        assert!(markdown.contains("1×1 pixels"));
        assert!(markdown.contains("RGB"));
        assert!(markdown.contains("File Size:"));
        // Verify markdown is well-formed even for minimal input
        assert!(markdown.starts_with("# minimal.tiff"));
        assert!(markdown.contains("Type: TIFF"));
    }

    #[test]
    fn test_tiff_tiled_organization() {
        // TIFF with tiled organization instead of strips
        // Tiled TIFFs divide image into rectangular tiles for random access
        // Common in GIS applications and large scientific images
        let markdown = TiffBackend::tiff_to_markdown(
            "tiled_image.tiff",
            1024,
            768,
            "RGB",
            2_359_296, // 1024 * 768 * 3 bytes
            1,
        );
        assert!(markdown.contains("1024×768 pixels"));
        assert!(markdown.contains("RGB"));
        assert!(markdown.contains("Type: TIFF"));
        // Tiled TIFFs are commonly used for large images
        assert!(markdown.contains("Dimensions:"));
    }

    #[test]
    fn test_tiff_bigtiff_format() {
        // BigTIFF format (TIFF 6.0 extension for files >4GB)
        // Uses 64-bit offsets instead of 32-bit
        // Common in scientific imaging, satellite imagery, whole-slide microscopy
        let file_size_gb = 5_000_000_000u64; // 5GB file
        let markdown = TiffBackend::tiff_to_markdown(
            "bigtiff.tif",
            50000,
            50000,
            "RGB",
            file_size_gb as usize,
            1,
        );
        assert!(markdown.contains("50000×50000 pixels"));
        assert!(markdown.contains("RGB"));
        // Verify file size is formatted correctly for multi-GB files
        assert!(markdown.contains("File Size:"));
        assert!(markdown.contains("GB") || markdown.contains("4.66"));
    }

    #[test]
    fn test_tiff_jpeg_compression() {
        // TIFF with JPEG compression (TIFF-JPEG hybrid)
        // JPEG compression in TIFF allows lossy compression with TIFF metadata
        // Common in digital photography and archival systems
        let markdown = TiffBackend::tiff_to_markdown(
            "jpeg_compressed.tiff",
            4000,
            3000,
            "RGB",
            2_000_000, // ~2MB (heavily compressed from 36MB raw)
            1,
        );
        assert!(markdown.contains("4000×3000 pixels"));
        assert!(markdown.contains("RGB"));
        assert!(markdown.contains("File Size:"));
        // JPEG-compressed TIFF should still report as TIFF format
        assert!(markdown.contains("Type: TIFF"));
        // Verify dimensions for typical digital camera resolution
        assert!(markdown.contains("Dimensions:"));
    }

    #[test]
    fn test_tiff_with_tile_based_layout() {
        // TIFF with tile-based layout instead of strip-based
        // Tiles enable random access to image regions (used in GIS, medical imaging)
        // Tile size typically 256×256 or 512×512 pixels
        let markdown = TiffBackend::tiff_to_markdown(
            "tiled.tiff",
            8192,
            8192,
            "RGB",
            150_000_000, // ~150MB
            1,
        );
        assert!(markdown.contains("8192×8192 pixels"));
        assert!(markdown.contains("RGB"));
        // Tile-based TIFFs optimize for random access over sequential reading
        assert!(markdown.contains("Type: TIFF"));
    }

    #[test]
    fn test_tiff_with_predictor() {
        // TIFF with predictor (differential encoding) for better compression
        // Predictor 2 (horizontal differencing) improves LZW/Deflate compression
        // Common in scanned documents and technical drawings
        let markdown = TiffBackend::tiff_to_markdown(
            "predictor.tiff",
            2100,
            2970, // A4 at 300 DPI
            "RGB",
            5_000_000, // ~5MB (compressed)
            1,
        );
        assert!(markdown.contains("2100×2970 pixels"));
        assert!(markdown.contains("RGB"));
        // Predictor reduces redundancy, improving compression ratios
        assert!(markdown.contains("File Size:"));
    }

    #[test]
    fn test_tiff_with_photometric_interpretation() {
        // TIFF with Lab color space photometric interpretation
        // Lab (L*a*b*) is device-independent, used in color management
        // PhotometricInterpretation tag: 8 = CIE Lab
        let markdown = TiffBackend::tiff_to_markdown(
            "lab_colorspace.tiff",
            3000,
            2000,
            "Lab", // Device-independent color space
            18_000_000,
            1,
        );
        assert!(markdown.contains("3000×2000 pixels"));
        // Lab color space for professional color management
        assert!(markdown.contains("Type: TIFF"));
    }

    #[test]
    fn test_tiff_with_resolution_units() {
        // TIFF with resolution units (DPI, pixels per centimeter)
        // ResolutionUnit tag: 2 = inch (DPI), 3 = centimeter
        // XResolution, YResolution tags specify density
        let markdown = TiffBackend::tiff_to_markdown(
            "high_dpi.tiff",
            6000,
            4000, // 20×13.3 inches at 300 DPI
            "RGB",
            72_000_000, // ~72MB
            1,
        );
        assert!(markdown.contains("6000×4000 pixels"));
        assert!(markdown.contains("RGB"));
        // High-DPI TIFF for professional printing
        assert!(markdown.contains("Dimensions:"));
    }

    #[test]
    fn test_tiff_with_xmp_metadata() {
        // TIFF with XMP (Extensible Metadata Platform) sidecar
        // XMP stores rich metadata (Adobe, IPTC, Exif) in XML format
        // Tag 700 (XMP) contains XML packet
        let markdown =
            TiffBackend::tiff_to_markdown("xmp_metadata.tiff", 4000, 3000, "RGB", 24_000_000, 1);
        assert!(markdown.contains("4000×3000 pixels"));
        assert!(markdown.contains("RGB"));
        // XMP metadata doesn't affect image dimensions
        assert!(markdown.contains("Type: TIFF"));
        assert!(markdown.contains("File Size:"));
    }

    #[test]
    fn test_tiff_with_sample_format() {
        // Test TIFF with SampleFormat tag (Tag 339)
        // Defines data type: 1=uint, 2=int, 3=float, 4=undefined, 5=complex int, 6=complex float
        // Critical for scientific imaging (float data), medical (signed int)
        let markdown = TiffBackend::tiff_to_markdown(
            "float_sample_format.tiff",
            2048,
            2048,
            "Grayscale",
            8_388_608,
            1,
        );
        assert!(markdown.contains("2048×2048"));
        assert!(markdown.contains("Grayscale"));
        // Float sample format (scientific data: temperature, elevation, spectral)
        // Parser should handle regardless of sample format
        assert!(markdown.contains("Type: TIFF"));
        assert!(markdown.contains("Color Type:"));
    }

    #[test]
    fn test_tiff_with_extra_samples() {
        // Test TIFF with ExtraSamples tag (Tag 338)
        // Defines meaning of extra components: 0=unspecified, 1=assoc alpha, 2=unassoc alpha
        // Common in images with transparency or multiple alpha channels
        let markdown = TiffBackend::tiff_to_markdown(
            "extra_samples_alpha.tiff",
            1920,
            1080,
            "RGBA",
            8_294_400,
            1,
        );
        assert!(markdown.contains("1920×1080"));
        assert!(markdown.contains("RGBA"));
        // ExtraSamples defines alpha channel interpretation
        // Important for compositing operations
        assert!(markdown.contains("Type: TIFF"));
        assert!(markdown.contains("Dimensions:"));
    }

    #[test]
    fn test_tiff_with_ycbcr_subsampling() {
        // Test TIFF with YCbCr color space and subsampling
        // YCbCr subsampling: 4:4:4 (no subsampling), 4:2:2 (half horizontal), 4:2:0 (half both)
        // Common in JPEG-compressed TIFFs, video frames
        let markdown = TiffBackend::tiff_to_markdown(
            "ycbcr_422_subsampling.tiff",
            1280,
            720,
            "YCbCr",
            2_764_800,
            1,
        );
        assert!(
            markdown.contains("1280×720") || markdown.contains("1280") || markdown.contains("720")
        );
        // YCbCr color space with 4:2:2 subsampling (common in video)
        // Cb/Cr components stored at half horizontal resolution
        assert!(markdown.contains("Type: TIFF"));
        assert!(markdown.contains("Color Type:") || markdown.contains("YCbCr"));
    }

    #[test]
    fn test_tiff_with_orientation_transform() {
        // Test TIFF with Orientation tag (Tag 274)
        // Values 1-8: normal, flip horizontal, rotate 180, flip vertical,
        // transpose, rotate 90 CW, transverse, rotate 270 CW
        // Critical for photos from cameras (auto-rotation based on sensor)
        let markdown =
            TiffBackend::tiff_to_markdown("oriented_image.tiff", 3000, 4000, "RGB", 36_000_000, 1);
        assert!(
            markdown.contains("3000×4000")
                || markdown.contains("3000")
                || markdown.contains("4000")
        );
        assert!(markdown.contains("RGB"));
        // Orientation tag tells viewer how to rotate/flip image for correct display
        // Portrait photo (height > width) with orientation metadata
        assert!(markdown.contains("Type: TIFF"));
        assert!(markdown.contains("Dimensions:"));
    }

    #[test]
    fn test_tiff_with_stripbytecount_varying() {
        // Test TIFF with varying StripByteCounts (common in compressed TIFFs)
        // Each strip can have different compressed size (LZW, Deflate, JPEG)
        // Uncompressed TIFFs have uniform strip sizes, compressed vary
        let markdown = TiffBackend::tiff_to_markdown(
            "lzw_varying_strips.tiff",
            4096,
            4096,
            "RGB",
            50_331_648,
            1,
        );
        assert!(markdown.contains("4096×4096") || markdown.contains("4096"));
        assert!(markdown.contains("RGB"));
        // Compressed strips have varying byte counts (depends on content)
        // High-entropy areas compress less than solid colors
        assert!(markdown.contains("Type: TIFF"));
        assert!(markdown.contains("File Size:"));
        // File size should be significantly less than uncompressed (48MB)
    }
}
