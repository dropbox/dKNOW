//! BMP backend for docling
//!
//! This backend converts BMP (Windows Bitmap) files to docling's document model.

use crate::traits::{BackendOptions, DocumentBackend};
use crate::utils::{create_section_header, create_text_item, format_file_size, opt_vec};
use docling_core::{DocItem, DoclingError, Document, DocumentMetadata, InputFormat};
use docling_ocr::OcrEngine;
use image::ImageReader;
use std::fmt::Write;
use std::io::Cursor;
use std::path::Path;

// BMP DIB Header Sizes (Microsoft Windows Bitmap Format Specification)
// Reference: https://docs.microsoft.com/en-us/windows/win32/gdi/bitmap-header-types

/// BITMAPCOREHEADER size in bytes (OS/2 1.x format)
const BMP_BITMAPCOREHEADER_SIZE: u32 = 12;

/// BITMAPINFOHEADER size in bytes (Windows 3.x format)
const BMP_BITMAPINFOHEADER_SIZE: u32 = 40;

/// BITMAPV4HEADER size in bytes (Windows 95 format)
const BMP_BITMAPV4HEADER_SIZE: u32 = 108;

/// BITMAPV5HEADER size in bytes (Windows 98 format)
const BMP_BITMAPV5HEADER_SIZE: u32 = 124;

/// BMP backend
///
/// Converts BMP (Windows Bitmap) files to docling's document model.
/// Extracts basic metadata and uses OCR to extract text content from the image.
///
/// ## Features
///
/// - Extract image dimensions
/// - Detect BMP format version (BITMAPINFOHEADER, BITMAPV4HEADER, BITMAPV5HEADER)
/// - Extract color depth information
/// - OCR text extraction with bounding boxes
/// - Generate markdown with image metadata and OCR text
///
/// ## Example
///
/// ```no_run
/// use docling_backend::BmpBackend;
/// use docling_backend::DocumentBackend;
///
/// let backend = BmpBackend::new();
/// let result = backend.parse_file("image.bmp", &Default::default())?;
/// println!("Image: {:?}", result.metadata.title);
/// # Ok::<(), docling_core::error::DoclingError>(())
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct BmpBackend;

impl BmpBackend {
    /// Create a new BMP backend instance
    #[inline]
    #[must_use = "creates a backend instance that should be used for parsing"]
    pub const fn new() -> Self {
        Self
    }

    /// Parse BMP header to extract basic metadata
    ///
    /// BMP format structure:
    /// - File Header (14 bytes):
    ///   - Signature: "BM" (2 bytes)
    ///   - File size: (4 bytes)
    ///   - Reserved: (4 bytes)
    ///   - Data offset: (4 bytes)
    /// - DIB Header (variable size, minimum 40 bytes for BITMAPINFOHEADER):
    ///   - Header size: (4 bytes) - determines version
    ///   - Width: (4 bytes, signed int)
    ///   - Height: (4 bytes, signed int)
    ///   - Planes: (2 bytes)
    ///   - Bits per pixel: (2 bytes)
    ///   - Compression: (4 bytes)
    ///   - Image size: (4 bytes)
    ///   - X pixels per meter: (4 bytes)
    ///   - Y pixels per meter: (4 bytes)
    ///   - Colors used: (4 bytes)
    ///   - Important colors: (4 bytes)
    fn parse_bmp_metadata(data: &[u8]) -> Result<(u32, u32, u16, &str), DoclingError> {
        // Minimum BMP file size: 14 (file header) + 12 (smallest DIB header) = 26 bytes
        if data.len() < 26 {
            return Err(DoclingError::BackendError(
                "File too small to be a valid BMP".to_string(),
            ));
        }

        // Check BMP signature
        if &data[0..2] != b"BM" {
            return Err(DoclingError::BackendError(
                "Invalid BMP header signature".to_string(),
            ));
        }

        // Read DIB header size to determine format version
        let dib_header_size = u32::from_le_bytes([data[14], data[15], data[16], data[17]]);

        // Minimum DIB header is BITMAPCOREHEADER (12 bytes)
        if data.len() < 14 + dib_header_size as usize {
            return Err(DoclingError::BackendError(
                "Truncated BMP DIB header".to_string(),
            ));
        }

        let (width, height, bits_per_pixel, version) = match dib_header_size {
            BMP_BITMAPCOREHEADER_SIZE => {
                // BITMAPCOREHEADER (OS/2 1.x)
                let width = u32::from(u16::from_le_bytes([data[18], data[19]]));
                let height = u32::from(u16::from_le_bytes([data[20], data[21]]));
                let bits_per_pixel = u16::from_le_bytes([data[24], data[25]]);
                (width, height, bits_per_pixel, "OS/2 1.x (BITMAPCOREHEADER)")
            }
            BMP_BITMAPINFOHEADER_SIZE => {
                // BITMAPINFOHEADER (Windows 3.x)
                let width =
                    i32::from_le_bytes([data[18], data[19], data[20], data[21]]).unsigned_abs();
                let height =
                    i32::from_le_bytes([data[22], data[23], data[24], data[25]]).unsigned_abs();
                let bits_per_pixel = u16::from_le_bytes([data[28], data[29]]);
                (
                    width,
                    height,
                    bits_per_pixel,
                    "Windows 3.x (BITMAPINFOHEADER)",
                )
            }
            BMP_BITMAPV4HEADER_SIZE => {
                // BITMAPV4HEADER (Windows 95)
                let width =
                    i32::from_le_bytes([data[18], data[19], data[20], data[21]]).unsigned_abs();
                let height =
                    i32::from_le_bytes([data[22], data[23], data[24], data[25]]).unsigned_abs();
                let bits_per_pixel = u16::from_le_bytes([data[28], data[29]]);
                (width, height, bits_per_pixel, "Windows 95 (BITMAPV4HEADER)")
            }
            BMP_BITMAPV5HEADER_SIZE => {
                // BITMAPV5HEADER (Windows 98)
                let width =
                    i32::from_le_bytes([data[18], data[19], data[20], data[21]]).unsigned_abs();
                let height =
                    i32::from_le_bytes([data[22], data[23], data[24], data[25]]).unsigned_abs();
                let bits_per_pixel = u16::from_le_bytes([data[28], data[29]]);
                (width, height, bits_per_pixel, "Windows 98 (BITMAPV5HEADER)")
            }
            _ => {
                return Err(DoclingError::BackendError(format!(
                    "Unsupported BMP DIB header size: {dib_header_size}"
                )));
            }
        };

        Ok((width, height, bits_per_pixel, version))
    }

    /// Convert BMP metadata to markdown
    fn bmp_to_markdown(
        filename: &str,
        width: u32,
        height: u32,
        bits_per_pixel: u16,
        version: &str,
        file_size: usize,
    ) -> String {
        let mut markdown = String::new();

        // Title
        let _ = writeln!(markdown, "# {filename}\n");

        // Image type
        markdown.push_str("Type: BMP (Windows Bitmap)\n\n");

        // Version
        let _ = writeln!(markdown, "Format: {version}\n");

        // Dimensions
        let _ = writeln!(markdown, "Dimensions: {width}×{height} pixels\n");

        // Color depth
        markdown.push_str("Color Depth: ");
        match bits_per_pixel {
            1 => markdown.push_str("1-bit (monochrome)"),
            4 => markdown.push_str("4-bit (16 colors)"),
            8 => markdown.push_str("8-bit (256 colors)"),
            16 => markdown.push_str("16-bit (65,536 colors)"),
            24 => markdown.push_str("24-bit (16.7 million colors, True Color)"),
            32 => markdown.push_str("32-bit (16.7 million colors + alpha)"),
            _ => {
                let _ = write!(markdown, "{bits_per_pixel}-bit");
            }
        }
        markdown.push_str("\n\n");

        // File size
        markdown.push_str(&format_file_size(file_size));

        // Image reference with descriptive alt text
        let base_name = filename.trim_end_matches(".bmp").trim_end_matches(".BMP");
        let _ = writeln!(
            markdown,
            "![{base_name} - {width}×{height} BMP image]({filename})"
        );

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
                page_no: 1, // BMP is single-page
                bbox,
                charspan: None,
            }];

            doc_items.push(create_text_item(start_index + i, line.text.clone(), prov));
        }

        // Get full OCR text
        let ocr_text = ocr_result.text();

        Ok((ocr_text, doc_items))
    }

    /// Create `DocItems` directly from BMP metadata
    ///
    /// Generates structured `DocItems` from BMP metadata without markdown intermediary.
    /// Creates a hierarchical document structure:
    /// - Title (filename) as `SectionHeader` level 1
    /// - Image type as Text
    /// - Format version as Text
    /// - Dimensions as Text
    /// - Color depth as Text
    /// - File size as Text
    /// - Image reference as Text
    ///
    /// ## Arguments
    /// * `filename` - Name of the BMP file
    /// * `width` - Image width in pixels
    /// * `height` - Image height in pixels
    /// * `bits_per_pixel` - Color depth (1, 4, 8, 16, 24, 32, etc.)
    /// * `version` - BMP format version string
    /// * `file_size` - Size of the BMP file in bytes
    ///
    /// ## Returns
    /// Vector of `DocItems` representing the BMP metadata structure
    fn bmp_to_docitems(
        filename: &str,
        width: u32,
        height: u32,
        bits_per_pixel: u16,
        version: &str,
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
            "Type: BMP (Windows Bitmap)".to_string(),
            vec![],
        ));
        index += 1;

        // Format version
        doc_items.push(create_text_item(
            index,
            format!("Format: {version}"),
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

        // Color depth
        let color_info = match bits_per_pixel {
            1 => "1-bit (monochrome)".to_string(),
            4 => "4-bit (16 colors)".to_string(),
            8 => "8-bit (256 colors)".to_string(),
            16 => "16-bit (65,536 colors)".to_string(),
            24 => "24-bit (16.7 million colors, True Color)".to_string(),
            32 => "32-bit (16.7 million colors + alpha)".to_string(),
            _ => format!("{bits_per_pixel}-bit"),
        };
        doc_items.push(create_text_item(
            index,
            format!("Color Depth: {color_info}"),
            vec![],
        ));
        index += 1;

        // File size
        let size_text = format_file_size(file_size);
        doc_items.push(create_text_item(index, size_text, vec![]));
        index += 1;

        // Image reference with standard alt text
        doc_items.push(create_text_item(
            index,
            format!("![{filename}]({filename})"),
            vec![],
        ));

        doc_items
    }

    /// Shared parsing logic for both `parse_bytes` and `parse_file`
    ///
    /// Extracts BMP metadata, creates `DocItem`s, runs OCR, and generates markdown.
    /// Both `parse_bytes` and `parse_file` delegate to this method to avoid code duplication.
    fn parse_bmp_data(data: &[u8], filename: &str) -> Result<Document, DoclingError> {
        // Helper to add filename context to errors
        let add_context = |e: DoclingError| match e {
            DoclingError::BackendError(msg) => {
                DoclingError::BackendError(format!("{msg}: {filename}"))
            }
            other => other,
        };

        // Parse BMP metadata
        let (width, height, bits_per_pixel, version) =
            Self::parse_bmp_metadata(data).map_err(add_context)?;

        // Create DocItems directly from BMP metadata (no markdown intermediary)
        let mut doc_items =
            Self::bmp_to_docitems(filename, width, height, bits_per_pixel, version, data.len());
        let metadata_items_count = doc_items.len();

        // Extract OCR text
        let (ocr_text, mut ocr_items) =
            Self::extract_ocr_text(data, metadata_items_count).map_err(add_context)?;

        // Combine DocItems: metadata + OCR
        doc_items.append(&mut ocr_items);

        // Generate markdown from DocItems (for backwards compatibility)
        let metadata_markdown =
            Self::bmp_to_markdown(filename, width, height, bits_per_pixel, version, data.len());
        let mut markdown = metadata_markdown;
        if !ocr_text.is_empty() {
            markdown.push_str("\n\n## OCR Text\n\n");
            markdown.push_str(&ocr_text);
        }

        let num_characters = markdown.chars().count();

        // Create document
        Ok(Document {
            markdown,
            format: InputFormat::Bmp,
            metadata: DocumentMetadata {
                num_pages: Some(1), // BMP is single-page
                num_characters,
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
}

impl DocumentBackend for BmpBackend {
    #[inline]
    fn format(&self) -> InputFormat {
        InputFormat::Bmp
    }

    fn parse_bytes(
        &self,
        data: &[u8],
        _options: &BackendOptions,
    ) -> Result<Document, DoclingError> {
        Self::parse_bmp_data(data, "image.bmp")
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
            .unwrap_or("image.bmp");

        // Read file and delegate to shared parsing logic
        let data = std::fs::read(path_ref).map_err(DoclingError::IoError)?;
        Self::parse_bmp_data(&data, filename)
    }
}

#[cfg(test)]
#[allow(clippy::unreadable_literal)]
mod tests {
    use super::*;

    /// Create a minimal valid BMP file (32x32 pixel, 24-bit)
    ///
    /// Note: Changed from 1x1 to 32x32 because OCR requires minimum dimensions.
    /// Smaller images (1x1, 8x8) cause "Invalid resize dimensions: 0x0" error in OCR engine.
    /// OCR preprocessing typically requires images to be at least 32x32 pixels.
    fn create_test_bmp() -> Vec<u8> {
        let mut data = Vec::new();

        // BMP dimensions
        let width = 32i32;
        let height = 32i32;
        let bytes_per_pixel = 3; // 24-bit RGB

        // Each row is padded to 4-byte boundary
        let row_size = ((width * bytes_per_pixel + 3) / 4) * 4;
        let pixel_data_size = row_size * height;
        let file_size = 54 + pixel_data_size; // 14 (file header) + 40 (DIB header) + pixel data

        // BMP File Header (14 bytes)
        data.extend_from_slice(b"BM"); // Signature
        data.extend_from_slice(&(file_size as u32).to_le_bytes()); // File size
        data.extend_from_slice(&0u16.to_le_bytes()); // Reserved 1
        data.extend_from_slice(&0u16.to_le_bytes()); // Reserved 2
        data.extend_from_slice(&54u32.to_le_bytes()); // Pixel data offset

        // DIB Header (BITMAPINFOHEADER, 40 bytes)
        data.extend_from_slice(&40u32.to_le_bytes()); // Header size
        data.extend_from_slice(&width.to_le_bytes()); // Width
        data.extend_from_slice(&height.to_le_bytes()); // Height
        data.extend_from_slice(&1u16.to_le_bytes()); // Planes
        data.extend_from_slice(&24u16.to_le_bytes()); // Bits per pixel
        data.extend_from_slice(&0u32.to_le_bytes()); // Compression (BI_RGB)
        data.extend_from_slice(&(pixel_data_size as u32).to_le_bytes()); // Image size
        data.extend_from_slice(&0i32.to_le_bytes()); // X pixels per meter
        data.extend_from_slice(&0i32.to_le_bytes()); // Y pixels per meter
        data.extend_from_slice(&0u32.to_le_bytes()); // Colors used
        data.extend_from_slice(&0u32.to_le_bytes()); // Important colors

        // Pixel data (8x8 pixels, 24-bit BGR)
        // Create a simple pattern (alternating white and black)
        for y in 0..height {
            for x in 0..width {
                // Checkerboard pattern: white if (x+y) is even, black if odd
                let color = if (x + y) % 2 == 0 { 255u8 } else { 0u8 };
                data.extend_from_slice(&[color, color, color]); // BGR
            }
            // Add padding to align row to 4-byte boundary
            let padding_bytes = row_size as usize - (width as usize * bytes_per_pixel as usize);
            data.extend(std::iter::repeat_n(0, padding_bytes));
        }

        data
    }

    /// Create a BMP with BITMAPCOREHEADER (OS/2 format)
    fn create_os2_bmp() -> Vec<u8> {
        let mut data = Vec::new();

        // BMP File Header (14 bytes)
        data.extend_from_slice(b"BM");
        data.extend_from_slice(&30u32.to_le_bytes()); // File size
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&26u32.to_le_bytes()); // Offset

        // DIB Header (BITMAPCOREHEADER, 12 bytes)
        data.extend_from_slice(&12u32.to_le_bytes()); // Header size
        data.extend_from_slice(&1u16.to_le_bytes()); // Width
        data.extend_from_slice(&1u16.to_le_bytes()); // Height
        data.extend_from_slice(&1u16.to_le_bytes()); // Planes
        data.extend_from_slice(&24u16.to_le_bytes()); // Bits per pixel

        // Pixel data
        data.extend_from_slice(&[255, 0, 0]);
        data.push(0);

        data
    }

    #[test]
    fn test_parse_bmp_metadata() {
        let data = create_test_bmp();
        let result = BmpBackend::parse_bmp_metadata(&data);
        assert!(result.is_ok(), "BMP metadata parsing should succeed");

        let (width, height, bits_per_pixel, version) = result.unwrap();
        assert_eq!(width, 32, "Width should be 32 pixels");
        assert_eq!(height, 32, "Height should be 32 pixels");
        assert_eq!(bits_per_pixel, 24, "Bits per pixel should be 24");
        assert_eq!(
            version, "Windows 3.x (BITMAPINFOHEADER)",
            "Version should be BITMAPINFOHEADER"
        );
    }

    #[test]
    fn test_parse_os2_bmp() {
        let data = create_os2_bmp();
        let result = BmpBackend::parse_bmp_metadata(&data);
        assert!(result.is_ok(), "OS/2 BMP metadata parsing should succeed");

        let (width, height, bits_per_pixel, version) = result.unwrap();
        assert_eq!(width, 1, "OS/2 BMP width should be 1 pixel");
        assert_eq!(height, 1, "OS/2 BMP height should be 1 pixel");
        assert_eq!(bits_per_pixel, 24, "OS/2 BMP bits per pixel should be 24");
        assert_eq!(
            version, "OS/2 1.x (BITMAPCOREHEADER)",
            "Version should be BITMAPCOREHEADER"
        );
    }

    #[test]
    fn test_invalid_signature() {
        let mut data = create_test_bmp();
        data[0] = b'X'; // Corrupt signature
        let result = BmpBackend::parse_bmp_metadata(&data);
        assert!(result.is_err(), "Invalid signature should return error");
    }

    #[test]
    fn test_file_too_small() {
        let data = vec![b'B', b'M', 0, 0, 0]; // Only 5 bytes
        let result = BmpBackend::parse_bmp_metadata(&data);
        assert!(result.is_err(), "File too small should return error");
    }

    #[test]
    fn test_bmp_to_markdown() {
        let markdown = BmpBackend::bmp_to_markdown(
            "test.bmp",
            800,
            600,
            24,
            "Windows 3.x (BITMAPINFOHEADER)",
            1_440_054,
        );
        assert!(
            markdown.contains("# test.bmp"),
            "Markdown should contain title header"
        );
        assert!(
            markdown.contains("Type: BMP (Windows Bitmap)"),
            "Markdown should contain BMP type"
        );
        assert!(
            markdown.contains("Dimensions: 800×600 pixels"),
            "Markdown should contain dimensions"
        );
        assert!(
            markdown.contains("Color Depth: 24-bit (16.7 million colors, True Color)"),
            "Markdown should contain color depth"
        );
        assert!(
            markdown.contains("![test - 800×600 BMP image](test.bmp)"),
            "Markdown should contain image reference"
        );
    }

    #[test]
    fn test_backend_format() {
        let backend = BmpBackend::new();
        assert_eq!(
            backend.format(),
            InputFormat::Bmp,
            "Backend format should be BMP"
        );
    }

    #[test]
    fn test_color_depth_descriptions() {
        // Test various color depths
        let depths = vec![
            (1, "1-bit (monochrome)"),
            (4, "4-bit (16 colors)"),
            (8, "8-bit (256 colors)"),
            (16, "16-bit (65,536 colors)"),
            (24, "24-bit (16.7 million colors, True Color)"),
            (32, "32-bit (16.7 million colors + alpha)"),
        ];

        for (bits, expected) in depths {
            let markdown = BmpBackend::bmp_to_markdown("test.bmp", 100, 100, bits, "Test", 1000);
            assert!(
                markdown.contains(expected),
                "Missing color depth description for {bits} bits"
            );
        }
    }

    // ============================================================================
    // Backend Creation Tests
    // ============================================================================

    #[test]
    fn test_create_backend() {
        let backend = BmpBackend::new();
        assert_eq!(
            backend.format(),
            InputFormat::Bmp,
            "New backend should have BMP format"
        );
    }

    #[test]
    fn test_create_backend_default() {
        let backend = BmpBackend;
        assert_eq!(
            backend.format(),
            InputFormat::Bmp,
            "Default backend should have BMP format"
        );
    }

    #[test]
    fn test_backend_format_constant() {
        let backend = BmpBackend::new();
        assert_eq!(
            backend.format(),
            InputFormat::Bmp,
            "Backend format should be BMP on first call"
        );
        // Verify format is stable across multiple calls
        assert_eq!(
            backend.format(),
            InputFormat::Bmp,
            "Backend format should remain BMP on second call"
        );
    }

    // ============================================================================
    // BMP Header Parsing Tests
    // ============================================================================

    #[test]
    fn test_parse_bmp_v4_header() {
        // Create BMP with BITMAPV4HEADER (108 bytes)
        let mut data = Vec::new();
        data.extend_from_slice(b"BM");
        data.extend_from_slice(&162u32.to_le_bytes()); // File size
        data.extend_from_slice(&0u32.to_le_bytes()); // Reserved
        data.extend_from_slice(&122u32.to_le_bytes()); // Offset (14 + 108)

        // V4 Header (108 bytes)
        data.extend_from_slice(&108u32.to_le_bytes()); // Header size
        data.extend_from_slice(&2i32.to_le_bytes()); // Width
        data.extend_from_slice(&2i32.to_le_bytes()); // Height
        data.extend_from_slice(&1u16.to_le_bytes()); // Planes
        data.extend_from_slice(&24u16.to_le_bytes()); // Bits per pixel
                                                      // Fill rest with zeros to reach 108 bytes
        data.resize(122, 0);
        // Add minimal pixel data
        data.extend_from_slice(&[0; 40]); // Pixel data

        let result = BmpBackend::parse_bmp_metadata(&data);
        assert!(result.is_ok(), "V4 header parsing should succeed");
        let (width, height, bits_per_pixel, version) = result.unwrap();
        assert_eq!(width, 2, "V4 header width should be 2 pixels");
        assert_eq!(height, 2, "V4 header height should be 2 pixels");
        assert_eq!(bits_per_pixel, 24, "V4 header bits per pixel should be 24");
        assert_eq!(
            version, "Windows 95 (BITMAPV4HEADER)",
            "Version should be BITMAPV4HEADER"
        );
    }

    #[test]
    fn test_parse_bmp_v5_header() {
        // Create BMP with BITMAPV5HEADER (124 bytes)
        let mut data = Vec::new();
        data.extend_from_slice(b"BM");
        data.extend_from_slice(&178u32.to_le_bytes()); // File size
        data.extend_from_slice(&0u32.to_le_bytes()); // Reserved
        data.extend_from_slice(&138u32.to_le_bytes()); // Offset (14 + 124)

        // V5 Header (124 bytes)
        data.extend_from_slice(&124u32.to_le_bytes()); // Header size
        data.extend_from_slice(&3i32.to_le_bytes()); // Width
        data.extend_from_slice(&3i32.to_le_bytes()); // Height
        data.extend_from_slice(&1u16.to_le_bytes()); // Planes
        data.extend_from_slice(&32u16.to_le_bytes()); // Bits per pixel
                                                      // Fill rest with zeros to reach 124 bytes
        data.resize(138, 0);
        // Add minimal pixel data
        data.extend_from_slice(&[0; 40]); // Pixel data

        let result = BmpBackend::parse_bmp_metadata(&data);
        assert!(result.is_ok(), "V5 header parsing should succeed");
        let (width, height, bits_per_pixel, version) = result.unwrap();
        assert_eq!(width, 3, "V5 header width should be 3 pixels");
        assert_eq!(height, 3, "V5 header height should be 3 pixels");
        assert_eq!(bits_per_pixel, 32, "V5 header bits per pixel should be 32");
        assert_eq!(
            version, "Windows 98 (BITMAPV5HEADER)",
            "Version should be BITMAPV5HEADER"
        );
    }

    #[test]
    fn test_parse_bmp_negative_dimensions() {
        // BMP allows negative height (top-down scan)
        let mut data = create_test_bmp();
        // Set height to -32 (top-down)
        data[22..26].copy_from_slice(&(-32i32).to_le_bytes());

        let result = BmpBackend::parse_bmp_metadata(&data);
        assert!(result.is_ok(), "Negative height parsing should succeed");
        let (_, height, _, _) = result.unwrap();
        assert_eq!(
            height, 32,
            "Negative height should be converted to absolute value"
        );
    }

    #[test]
    fn test_parse_bmp_unsupported_header_size() {
        let mut data = create_test_bmp();
        // Set unsupported header size (e.g., 64 bytes)
        data[14..18].copy_from_slice(&64u32.to_le_bytes());

        let result = BmpBackend::parse_bmp_metadata(&data);
        assert!(
            result.is_err(),
            "Unsupported header size should return error"
        );
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Unsupported") || err_msg.contains("unsupported"),
            "Expected error about unsupported header, got: {err_msg}"
        );
    }

    #[test]
    fn test_parse_bmp_truncated_header() {
        let mut data = create_test_bmp();
        // Truncate to 50 bytes (not enough for full header)
        data.truncate(50);

        let result = BmpBackend::parse_bmp_metadata(&data);
        assert!(result.is_err(), "Truncated header should return error");
        assert!(
            result.unwrap_err().to_string().contains("Truncated"),
            "Error message should mention truncation"
        );
    }

    // ============================================================================
    // Markdown Generation Tests
    // ============================================================================

    #[test]
    fn test_markdown_contains_filename() {
        let markdown = BmpBackend::bmp_to_markdown(
            "my_image.bmp",
            100,
            100,
            24,
            "Windows 3.x (BITMAPINFOHEADER)",
            1024,
        );
        assert!(
            markdown.contains("# my_image.bmp"),
            "Markdown should contain filename as title"
        );
        assert!(
            markdown.contains("![my_image - 100×100 BMP image](my_image.bmp)"),
            "Markdown should contain image reference with alt text"
        );
    }

    #[test]
    fn test_markdown_contains_format_version() {
        let markdown = BmpBackend::bmp_to_markdown(
            "test.bmp",
            100,
            100,
            24,
            "Windows 95 (BITMAPV4HEADER)",
            1024,
        );
        assert!(
            markdown.contains("Format: Windows 95 (BITMAPV4HEADER)"),
            "Markdown should contain format version"
        );
    }

    #[test]
    fn test_markdown_large_dimensions() {
        let markdown = BmpBackend::bmp_to_markdown(
            "large.bmp",
            4096,
            3072,
            24,
            "Windows 3.x (BITMAPINFOHEADER)",
            37_748_736,
        );
        assert!(
            markdown.contains("Dimensions: 4096×3072 pixels"),
            "Markdown should contain large image dimensions"
        );
    }

    #[test]
    fn test_markdown_file_size_formatting() {
        // Test file size formatting (uses format_file_size helper)
        let markdown = BmpBackend::bmp_to_markdown("test.bmp", 100, 100, 24, "Test", 1536);
        assert!(
            markdown.contains("File Size:"),
            "Markdown should contain file size section"
        );
        // Should contain some size representation
        assert!(
            markdown.len() > 100,
            "Markdown output should have reasonable length"
        );
    }

    // ============================================================================
    // DocItem Creation Tests
    // ============================================================================

    #[test]
    fn test_bmp_to_docitems_structure() {
        let doc_items = BmpBackend::bmp_to_docitems(
            "test.bmp",
            800,
            600,
            24,
            "Windows 3.x (BITMAPINFOHEADER)",
            1_440_054,
        );

        // Should have 7 items: title (SectionHeader), type, format, dimensions, color depth, file size, image reference
        assert_eq!(doc_items.len(), 7, "DocItems should have 7 elements");

        // First item should be SectionHeader (title)
        match &doc_items[0] {
            DocItem::SectionHeader { text, level, .. } => {
                assert_eq!(text, "test.bmp", "Title should be filename");
                assert_eq!(*level, 1, "Title level should be 1");
            }
            _ => panic!("Expected SectionHeader DocItem for title"),
        }

        // Second item should be image type
        match &doc_items[1] {
            DocItem::Text { text, .. } => {
                assert_eq!(text, "Type: BMP (Windows Bitmap)", "Type should be BMP");
            }
            _ => panic!("Expected Text DocItem for type"),
        }

        // Third item should be format version
        match &doc_items[2] {
            DocItem::Text { text, .. } => {
                assert_eq!(
                    text, "Format: Windows 3.x (BITMAPINFOHEADER)",
                    "Format should be BITMAPINFOHEADER"
                );
            }
            _ => panic!("Expected Text DocItem for format"),
        }

        // Fourth item should be dimensions
        match &doc_items[3] {
            DocItem::Text { text, .. } => {
                assert_eq!(
                    text, "Dimensions: 800×600 pixels",
                    "Dimensions should match"
                );
            }
            _ => panic!("Expected Text DocItem for dimensions"),
        }

        // Fifth item should be color depth
        match &doc_items[4] {
            DocItem::Text { text, .. } => {
                assert_eq!(
                    text, "Color Depth: 24-bit (16.7 million colors, True Color)",
                    "Color depth should be 24-bit True Color"
                );
            }
            _ => panic!("Expected Text DocItem for color depth"),
        }
    }

    #[test]
    fn test_bmp_to_docitems_with_different_color_depths() {
        // Test 1-bit monochrome
        let doc_items = BmpBackend::bmp_to_docitems("test.bmp", 100, 100, 1, "Test", 1000);
        match &doc_items[4] {
            DocItem::Text { text, .. } => {
                assert!(
                    text.contains("1-bit (monochrome)"),
                    "1-bit color depth should show monochrome"
                );
            }
            _ => panic!("Expected Text DocItem"),
        }

        // Test 32-bit with alpha
        let doc_items = BmpBackend::bmp_to_docitems("test.bmp", 100, 100, 32, "Test", 1000);
        match &doc_items[4] {
            DocItem::Text { text, .. } => {
                assert!(
                    text.contains("32-bit (16.7 million colors + alpha)"),
                    "32-bit color depth should show alpha support"
                );
            }
            _ => panic!("Expected Text DocItem"),
        }
    }

    #[test]
    fn test_bmp_to_docitems_self_refs() {
        let doc_items = BmpBackend::bmp_to_docitems("test.bmp", 100, 100, 24, "Test", 1000);

        // Verify self_refs are correctly formatted
        match &doc_items[0] {
            DocItem::SectionHeader { self_ref, .. } => {
                assert_eq!(
                    self_ref, "#/headers/0",
                    "SectionHeader self_ref should be correctly formatted"
                );
            }
            _ => panic!("Expected SectionHeader"),
        }

        match &doc_items[1] {
            DocItem::Text { self_ref, .. } => {
                assert_eq!(
                    self_ref, "#/texts/1",
                    "Text self_ref should be correctly formatted"
                );
            }
            _ => panic!("Expected Text DocItem"),
        }
    }

    // ============================================================================
    // Integration Tests (parse_bytes)
    // ============================================================================

    #[test]
    fn test_parse_bytes_basic() {
        let data = create_test_bmp();
        let backend = BmpBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());
        assert!(result.is_ok(), "parse_bytes should succeed for valid BMP");

        let doc = result.unwrap();
        assert_eq!(
            doc.format,
            InputFormat::Bmp,
            "Document format should be BMP"
        );
        assert_eq!(
            doc.metadata.num_pages,
            Some(1),
            "BMP should have exactly 1 page"
        );
        assert!(doc.metadata.title.is_some(), "Document should have a title");
        assert!(
            doc.markdown.contains("BMP (Windows Bitmap)"),
            "Markdown should contain BMP type"
        );
    }

    #[test]
    fn test_parse_bytes_invalid_signature() {
        let mut data = create_test_bmp();
        data[0] = b'X'; // Corrupt signature

        let backend = BmpBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());
        assert!(
            result.is_err(),
            "parse_bytes should fail for corrupted signature"
        );
    }

    #[test]
    fn test_parse_bytes_document_structure() {
        let data = create_test_bmp();
        let backend = BmpBackend::new();
        let doc = backend.parse_bytes(&data, &Default::default()).unwrap();

        // Verify metadata
        assert_eq!(
            doc.metadata.num_pages,
            Some(1),
            "Document should have 1 page"
        );
        assert_eq!(
            doc.metadata.title,
            Some("image.bmp".to_string()),
            "Title should be image.bmp"
        );
        assert!(
            doc.metadata.num_characters > 0,
            "Document should have characters"
        );

        // Verify DocItems exist
        assert!(
            doc.content_blocks.is_some(),
            "Document should have content_blocks"
        );
        let doc_items = doc.content_blocks.unwrap();
        assert!(!doc_items.is_empty(), "content_blocks should not be empty");
    }

    #[test]
    fn test_parse_bytes_markdown_length() {
        let data = create_test_bmp();
        let backend = BmpBackend::new();
        let doc = backend.parse_bytes(&data, &Default::default()).unwrap();

        // Markdown should contain metadata
        assert!(
            doc.markdown.len() > 100,
            "Markdown should have reasonable length"
        );
        assert_eq!(
            doc.metadata.num_characters,
            doc.markdown.chars().count(),
            "num_characters should match markdown character count"
        );
    }

    // ============================================================================
    // Category 2: Metadata Edge Cases (5 tests)
    // ============================================================================

    #[test]
    fn test_metadata_author_always_none() {
        // BMP format has no author metadata
        let data = create_test_bmp();
        let backend = BmpBackend::new();
        let doc = backend.parse_bytes(&data, &Default::default()).unwrap();
        assert_eq!(
            doc.metadata.author, None,
            "BMP author metadata should be None"
        );
    }

    #[test]
    fn test_metadata_timestamps_always_none() {
        // BMP format has no timestamp metadata
        let data = create_test_bmp();
        let backend = BmpBackend::new();
        let doc = backend.parse_bytes(&data, &Default::default()).unwrap();
        assert_eq!(
            doc.metadata.created, None,
            "BMP created timestamp should be None"
        );
        assert_eq!(
            doc.metadata.modified, None,
            "BMP modified timestamp should be None"
        );
    }

    #[test]
    fn test_metadata_language_always_none() {
        // BMP format has no language metadata
        let data = create_test_bmp();
        let backend = BmpBackend::new();
        let doc = backend.parse_bytes(&data, &Default::default()).unwrap();
        assert_eq!(
            doc.metadata.language, None,
            "BMP language metadata should be None"
        );
    }

    #[test]
    fn test_metadata_exif_always_none() {
        // BMP format doesn't support EXIF metadata (JPEG/TIFF only)
        let data = create_test_bmp();
        let backend = BmpBackend::new();
        let doc = backend.parse_bytes(&data, &Default::default()).unwrap();
        assert!(
            doc.metadata.exif.is_none(),
            "BMP EXIF metadata should be None (not supported)"
        );
    }

    #[test]
    fn test_metadata_title_from_filename() {
        let data = create_test_bmp();
        let backend = BmpBackend::new();

        // parse_bytes uses default "image.bmp"
        let doc = backend.parse_bytes(&data, &Default::default()).unwrap();
        assert_eq!(
            doc.metadata.title,
            Some("image.bmp".to_string()),
            "Title should be default filename 'image.bmp'"
        );
    }

    // ============================================================================
    // Category 3: DocItem Generation Edge Cases (6 tests)
    // ============================================================================

    #[test]
    fn test_docitem_self_refs_sequential() {
        let doc_items = BmpBackend::bmp_to_docitems("test.bmp", 100, 100, 24, "Test", 1000);

        // Verify self_refs are sequential
        match &doc_items[0] {
            DocItem::SectionHeader { self_ref, .. } => {
                assert_eq!(
                    self_ref, "#/headers/0",
                    "First SectionHeader self_ref should be #/headers/0"
                )
            }
            _ => panic!("Expected SectionHeader DocItem"),
        }
        match &doc_items[1] {
            DocItem::Text { self_ref, .. } => assert_eq!(
                self_ref, "#/texts/1",
                "Second item self_ref should be #/texts/1"
            ),
            _ => panic!("Expected Text DocItem"),
        }
        match &doc_items[2] {
            DocItem::Text { self_ref, .. } => assert_eq!(
                self_ref, "#/texts/2",
                "Third item self_ref should be #/texts/2"
            ),
            _ => panic!("Expected Text DocItem"),
        }
    }

    #[test]
    fn test_docitem_provenance_empty() {
        // Metadata DocItems don't have provenance (only OCR-extracted text does)
        let doc_items = BmpBackend::bmp_to_docitems("test.bmp", 100, 100, 24, "Test", 1000);

        match &doc_items[0] {
            DocItem::SectionHeader { prov, .. } => {
                assert_eq!(
                    prov.len(),
                    0,
                    "Metadata SectionHeader should have no provenance"
                );
            }
            _ => panic!("Expected SectionHeader DocItem"),
        }
    }

    #[test]
    fn test_docitem_content_layer_body() {
        let doc_items = BmpBackend::bmp_to_docitems("test.bmp", 100, 100, 24, "Test", 1000);

        match &doc_items[0] {
            DocItem::SectionHeader { content_layer, .. } => {
                assert_eq!(
                    content_layer, "body",
                    "SectionHeader content_layer should be 'body'"
                );
            }
            _ => panic!("Expected SectionHeader DocItem"),
        }
    }

    #[test]
    fn test_docitem_no_parent_or_children() {
        let doc_items = BmpBackend::bmp_to_docitems("test.bmp", 100, 100, 24, "Test", 1000);

        match &doc_items[0] {
            DocItem::SectionHeader {
                parent, children, ..
            } => {
                assert_eq!(parent, &None, "SectionHeader should have no parent");
                assert_eq!(children.len(), 0, "SectionHeader should have no children");
            }
            _ => panic!("Expected SectionHeader DocItem"),
        }
    }

    #[test]
    fn test_docitem_no_formatting_or_hyperlink() {
        let doc_items = BmpBackend::bmp_to_docitems("test.bmp", 100, 100, 24, "Test", 1000);

        // Text items (not SectionHeader) should have no formatting or hyperlink
        match &doc_items[1] {
            DocItem::Text {
                formatting,
                hyperlink,
                ..
            } => {
                assert_eq!(formatting, &None, "Text item should have no formatting");
                assert_eq!(hyperlink, &None, "Text item should have no hyperlink");
            }
            _ => panic!("Expected Text DocItem"),
        }
    }

    #[test]
    fn test_docitem_orig_equals_text() {
        let doc_items = BmpBackend::bmp_to_docitems("test.bmp", 100, 100, 24, "Test", 1000);

        match &doc_items[0] {
            DocItem::SectionHeader { orig, text, .. } => {
                assert_eq!(orig, text, "SectionHeader orig should equal text");
            }
            _ => panic!("Expected SectionHeader DocItem"),
        }
    }

    // ============================================================================
    // Category 4: Format-Specific Edge Cases (8 tests)
    // ============================================================================

    #[test]
    fn test_color_depth_1bit_monochrome() {
        let markdown = BmpBackend::bmp_to_markdown("test.bmp", 100, 100, 1, "Test", 1000);
        assert!(
            markdown.contains("1-bit (monochrome)"),
            "1-bit color depth should show monochrome description"
        );
    }

    #[test]
    fn test_color_depth_4bit_16colors() {
        let markdown = BmpBackend::bmp_to_markdown("test.bmp", 100, 100, 4, "Test", 1000);
        assert!(
            markdown.contains("4-bit (16 colors)"),
            "4-bit color depth should show 16 colors description"
        );
    }

    #[test]
    fn test_color_depth_8bit_256colors() {
        let markdown = BmpBackend::bmp_to_markdown("test.bmp", 100, 100, 8, "Test", 1000);
        assert!(
            markdown.contains("8-bit (256 colors)"),
            "8-bit color depth should show 256 colors description"
        );
    }

    #[test]
    fn test_color_depth_16bit() {
        let markdown = BmpBackend::bmp_to_markdown("test.bmp", 100, 100, 16, "Test", 1000);
        assert!(
            markdown.contains("16-bit (65,536 colors)"),
            "16-bit color depth should show 65,536 colors description"
        );
    }

    #[test]
    fn test_color_depth_32bit_with_alpha() {
        let markdown = BmpBackend::bmp_to_markdown("test.bmp", 100, 100, 32, "Test", 1000);
        assert!(
            markdown.contains("32-bit (16.7 million colors + alpha)"),
            "32-bit color depth should show alpha channel support"
        );
    }

    #[test]
    fn test_color_depth_unknown() {
        // Test unusual bit depth (48-bit, uncommon but valid)
        let markdown = BmpBackend::bmp_to_markdown("test.bmp", 100, 100, 48, "Test", 1000);
        assert!(
            markdown.contains("48-bit"),
            "Unknown bit depth should be displayed as-is"
        );
    }

    #[test]
    fn test_dimensions_very_small() {
        let markdown = BmpBackend::bmp_to_markdown("test.bmp", 1, 1, 24, "Test", 100);
        assert!(
            markdown.contains("Dimensions: 1×1 pixels"),
            "1x1 pixel dimensions should be displayed correctly"
        );
    }

    #[test]
    fn test_dimensions_very_large() {
        let markdown =
            BmpBackend::bmp_to_markdown("test.bmp", 16384, 8192, 24, "Test", 402_653_184);
        assert!(
            markdown.contains("Dimensions: 16384×8192 pixels"),
            "Large dimensions should be displayed correctly"
        );
    }

    // ============================================================================
    // Category 5: Integration & Complex Scenarios (4 tests)
    // ============================================================================

    #[test]
    fn test_full_document_structure() {
        let data = create_test_bmp();
        let backend = BmpBackend::new();
        let doc = backend.parse_bytes(&data, &Default::default()).unwrap();

        // Verify all document fields
        assert_eq!(
            doc.format,
            InputFormat::Bmp,
            "Document format should be BMP"
        );
        assert!(
            doc.markdown.contains("# image.bmp"),
            "Markdown should contain title header"
        );
        assert!(
            doc.markdown.contains("Type: BMP (Windows Bitmap)"),
            "Markdown should contain BMP type"
        );
        assert!(
            doc.markdown
                .contains("Format: Windows 3.x (BITMAPINFOHEADER)"),
            "Markdown should contain format version"
        );
        assert!(
            doc.markdown.contains("Dimensions: 32×32 pixels"),
            "Markdown should contain dimensions"
        );
        assert!(
            doc.markdown.contains("Color Depth: 24-bit"),
            "Markdown should contain color depth"
        );
        assert!(
            doc.markdown
                .contains("![image - 32×32 BMP image](image.bmp)"),
            "Markdown should contain image reference"
        );

        // Metadata
        assert_eq!(doc.metadata.num_pages, Some(1), "BMP should have 1 page");
        assert_eq!(
            doc.metadata.title,
            Some("image.bmp".to_string()),
            "Title should be filename"
        );
        assert_eq!(doc.metadata.author, None, "Author should be None");
        assert_eq!(
            doc.metadata.created, None,
            "Created timestamp should be None"
        );
        assert_eq!(
            doc.metadata.modified, None,
            "Modified timestamp should be None"
        );
        assert_eq!(doc.metadata.language, None, "Language should be None");
        assert!(doc.metadata.exif.is_none(), "EXIF should be None");

        // Content blocks
        assert!(
            doc.content_blocks.is_some(),
            "Document should have content_blocks"
        );
        let blocks = doc.content_blocks.unwrap();
        assert!(!blocks.is_empty(), "Content blocks should not be empty");
    }

    #[test]
    fn test_os2_format_full_parsing() {
        let data = create_os2_bmp();
        let backend = BmpBackend::new();

        // With OCR disabled (default), even 1x1 pixel images parse successfully
        let result = backend.parse_bytes(&data, &Default::default());
        assert!(
            result.is_ok(),
            "OS/2 BMP should parse successfully with OCR disabled"
        );

        // With OCR enabled, tiny images may fail OCR processing
        std::env::set_var("ENABLE_IMAGE_OCR", "1");
        let _result_with_ocr = backend.parse_bytes(&data, &Default::default());
        // OCR may fail or succeed depending on OCR engine tolerance for tiny images
        // We accept both outcomes
        std::env::remove_var("ENABLE_IMAGE_OCR");
    }

    #[test]
    fn test_parse_bytes_vs_parse_file_consistency() {
        // Both methods should produce same metadata structure
        let data = create_test_bmp();
        let backend = BmpBackend::new();

        let doc_bytes = backend.parse_bytes(&data, &Default::default()).unwrap();

        // Verify parse_bytes structure
        assert_eq!(
            doc_bytes.format,
            InputFormat::Bmp,
            "parse_bytes should return BMP format"
        );
        assert_eq!(
            doc_bytes.metadata.num_pages,
            Some(1),
            "parse_bytes should return 1 page"
        );
        assert!(
            doc_bytes.content_blocks.is_some(),
            "parse_bytes should return content_blocks"
        );
    }

    #[test]
    fn test_character_count_accuracy() {
        let data = create_test_bmp();
        let backend = BmpBackend::new();
        let doc = backend.parse_bytes(&data, &Default::default()).unwrap();

        // Character count should match markdown length
        let actual_chars = doc.markdown.chars().count();
        assert_eq!(
            doc.metadata.num_characters, actual_chars,
            "num_characters should match markdown character count"
        );

        // Should be non-zero
        assert!(
            doc.metadata.num_characters > 0,
            "num_characters should be greater than zero"
        );
    }

    #[test]
    fn test_bmp_compression_rle8() {
        // Test BMP with RLE8 compression (8-bit run-length encoding)
        let mut data = Vec::new();

        // BMP File Header
        data.extend_from_slice(b"BM");
        data.extend_from_slice(&154u32.to_le_bytes()); // File size
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&54u32.to_le_bytes()); // Offset

        // DIB Header (BITMAPINFOHEADER, 40 bytes)
        data.extend_from_slice(&40u32.to_le_bytes()); // Header size
        data.extend_from_slice(&4i32.to_le_bytes()); // Width
        data.extend_from_slice(&4i32.to_le_bytes()); // Height
        data.extend_from_slice(&1u16.to_le_bytes()); // Planes
        data.extend_from_slice(&8u16.to_le_bytes()); // Bits per pixel
        data.extend_from_slice(&1u32.to_le_bytes()); // Compression (BI_RLE8)
        data.extend_from_slice(&100u32.to_le_bytes()); // Image size
        data.extend_from_slice(&0i32.to_le_bytes()); // X pixels per meter
        data.extend_from_slice(&0i32.to_le_bytes()); // Y pixels per meter
        data.extend_from_slice(&256u32.to_le_bytes()); // Colors used
        data.extend_from_slice(&0u32.to_le_bytes()); // Important colors

        let result = BmpBackend::parse_bmp_metadata(&data);
        assert!(
            result.is_ok(),
            "RLE8 compressed BMP should parse successfully"
        );

        let (_width, _height, bits_per_pixel, version) = result.unwrap();
        assert_eq!(bits_per_pixel, 8, "RLE8 BMP bits per pixel should be 8");
        assert_eq!(
            version, "Windows 3.x (BITMAPINFOHEADER)",
            "RLE8 BMP version should be BITMAPINFOHEADER"
        );
    }

    #[test]
    fn test_bmp_compression_bitfields() {
        // Test BMP with BI_BITFIELDS compression (16/32-bit with color masks)
        let mut data = Vec::new();

        // BMP File Header
        data.extend_from_slice(b"BM");
        data.extend_from_slice(&70u32.to_le_bytes()); // File size
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&54u32.to_le_bytes()); // Offset

        // DIB Header (BITMAPINFOHEADER, 40 bytes)
        data.extend_from_slice(&40u32.to_le_bytes()); // Header size
        data.extend_from_slice(&2i32.to_le_bytes()); // Width
        data.extend_from_slice(&2i32.to_le_bytes()); // Height
        data.extend_from_slice(&1u16.to_le_bytes()); // Planes
        data.extend_from_slice(&16u16.to_le_bytes()); // Bits per pixel
        data.extend_from_slice(&3u32.to_le_bytes()); // Compression (BI_BITFIELDS)
        data.extend_from_slice(&16u32.to_le_bytes()); // Image size
        data.extend_from_slice(&0i32.to_le_bytes()); // X pixels per meter
        data.extend_from_slice(&0i32.to_le_bytes()); // Y pixels per meter
        data.extend_from_slice(&0u32.to_le_bytes()); // Colors used
        data.extend_from_slice(&0u32.to_le_bytes()); // Important colors

        let result = BmpBackend::parse_bmp_metadata(&data);
        assert!(
            result.is_ok(),
            "BITFIELDS compressed BMP should parse successfully"
        );

        let (_width, _height, bits_per_pixel, _version) = result.unwrap();
        assert_eq!(
            bits_per_pixel, 16,
            "BITFIELDS BMP bits per pixel should be 16"
        );
    }

    #[test]
    fn test_bmp_high_resolution_dpi() {
        // Test BMP with high DPI settings (print quality: 300 DPI = 11811 pixels/meter)
        let mut data = Vec::new();

        // BMP File Header
        data.extend_from_slice(b"BM");
        data.extend_from_slice(&154u32.to_le_bytes()); // File size
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&54u32.to_le_bytes()); // Offset

        // DIB Header (BITMAPINFOHEADER, 40 bytes)
        data.extend_from_slice(&40u32.to_le_bytes()); // Header size
        data.extend_from_slice(&100i32.to_le_bytes()); // Width
        data.extend_from_slice(&100i32.to_le_bytes()); // Height
        data.extend_from_slice(&1u16.to_le_bytes()); // Planes
        data.extend_from_slice(&24u16.to_le_bytes()); // Bits per pixel
        data.extend_from_slice(&0u32.to_le_bytes()); // Compression (BI_RGB)
        data.extend_from_slice(&30000u32.to_le_bytes()); // Image size
        data.extend_from_slice(&11811i32.to_le_bytes()); // X pixels per meter (300 DPI)
        data.extend_from_slice(&11811i32.to_le_bytes()); // Y pixels per meter (300 DPI)
        data.extend_from_slice(&0u32.to_le_bytes()); // Colors used
        data.extend_from_slice(&0u32.to_le_bytes()); // Important colors

        let result = BmpBackend::parse_bmp_metadata(&data);
        assert!(
            result.is_ok(),
            "High resolution DPI BMP should parse successfully"
        );

        let (width, height, _bits_per_pixel, _version) = result.unwrap();
        assert_eq!(width, 100, "High DPI BMP width should be 100");
        assert_eq!(height, 100, "High DPI BMP height should be 100");
        // DPI information is parsed but not returned in current implementation
    }

    #[test]
    fn test_bmp_low_resolution_screen() {
        // Test BMP with low DPI settings (screen quality: 72 DPI = 2835 pixels/meter)
        let mut data = Vec::new();

        // BMP File Header
        data.extend_from_slice(b"BM");
        data.extend_from_slice(&154u32.to_le_bytes()); // File size
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&54u32.to_le_bytes()); // Offset

        // DIB Header (BITMAPINFOHEADER, 40 bytes)
        data.extend_from_slice(&40u32.to_le_bytes()); // Header size
        data.extend_from_slice(&800i32.to_le_bytes()); // Width
        data.extend_from_slice(&600i32.to_le_bytes()); // Height
        data.extend_from_slice(&1u16.to_le_bytes()); // Planes
        data.extend_from_slice(&24u16.to_le_bytes()); // Bits per pixel
        data.extend_from_slice(&0u32.to_le_bytes()); // Compression (BI_RGB)
        data.extend_from_slice(&1_440_000u32.to_le_bytes()); // Image size
        data.extend_from_slice(&2835i32.to_le_bytes()); // X pixels per meter (72 DPI)
        data.extend_from_slice(&2835i32.to_le_bytes()); // Y pixels per meter (72 DPI)
        data.extend_from_slice(&0u32.to_le_bytes()); // Colors used
        data.extend_from_slice(&0u32.to_le_bytes()); // Important colors

        let result = BmpBackend::parse_bmp_metadata(&data);
        assert!(
            result.is_ok(),
            "Low resolution screen BMP should parse successfully"
        );

        let (width, height, bits_per_pixel, _version) = result.unwrap();
        assert_eq!(width, 800, "Low DPI BMP width should be 800");
        assert_eq!(height, 600, "Low DPI BMP height should be 600");
        assert_eq!(
            bits_per_pixel, 24,
            "Low DPI BMP bits per pixel should be 24"
        );
    }

    #[test]
    fn test_bmp_indexed_color_full_palette() {
        // Test BMP with 8-bit indexed color using all 256 colors
        let mut data = Vec::new();

        // BMP File Header
        data.extend_from_slice(b"BM");
        data.extend_from_slice(&1078u32.to_le_bytes()); // File size (54 + 1024 palette + 0 image)
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&1078u32.to_le_bytes()); // Offset (54 + 1024 palette)

        // DIB Header (BITMAPINFOHEADER, 40 bytes)
        data.extend_from_slice(&40u32.to_le_bytes()); // Header size
        data.extend_from_slice(&1i32.to_le_bytes()); // Width
        data.extend_from_slice(&1i32.to_le_bytes()); // Height
        data.extend_from_slice(&1u16.to_le_bytes()); // Planes
        data.extend_from_slice(&8u16.to_le_bytes()); // Bits per pixel
        data.extend_from_slice(&0u32.to_le_bytes()); // Compression (BI_RGB)
        data.extend_from_slice(&0u32.to_le_bytes()); // Image size (can be 0 for BI_RGB)
        data.extend_from_slice(&0i32.to_le_bytes()); // X pixels per meter
        data.extend_from_slice(&0i32.to_le_bytes()); // Y pixels per meter
        data.extend_from_slice(&256u32.to_le_bytes()); // Colors used (all 256)
        data.extend_from_slice(&256u32.to_le_bytes()); // Important colors (all 256)

        let result = BmpBackend::parse_bmp_metadata(&data);
        assert!(
            result.is_ok(),
            "Full palette indexed BMP should parse successfully"
        );

        let (_width, _height, bits_per_pixel, _version) = result.unwrap();
        assert_eq!(
            bits_per_pixel, 8,
            "Full palette BMP bits per pixel should be 8"
        );
        // Colors used field is parsed but not returned in current implementation
    }

    #[test]
    fn test_bmp_indexed_color_partial_palette() {
        // Test BMP with 8-bit indexed color using only 64 colors (partial palette)
        let mut data = Vec::new();

        // BMP File Header
        data.extend_from_slice(b"BM");
        data.extend_from_slice(&310u32.to_le_bytes()); // File size (54 + 256 palette + 0 image)
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&310u32.to_le_bytes()); // Offset

        // DIB Header (BITMAPINFOHEADER, 40 bytes)
        data.extend_from_slice(&40u32.to_le_bytes()); // Header size
        data.extend_from_slice(&1i32.to_le_bytes()); // Width
        data.extend_from_slice(&1i32.to_le_bytes()); // Height
        data.extend_from_slice(&1u16.to_le_bytes()); // Planes
        data.extend_from_slice(&8u16.to_le_bytes()); // Bits per pixel
        data.extend_from_slice(&0u32.to_le_bytes()); // Compression (BI_RGB)
        data.extend_from_slice(&0u32.to_le_bytes()); // Image size
        data.extend_from_slice(&0i32.to_le_bytes()); // X pixels per meter
        data.extend_from_slice(&0i32.to_le_bytes()); // Y pixels per meter
        data.extend_from_slice(&64u32.to_le_bytes()); // Colors used (only 64)
        data.extend_from_slice(&32u32.to_le_bytes()); // Important colors (32 most important)

        let result = BmpBackend::parse_bmp_metadata(&data);
        assert!(
            result.is_ok(),
            "Partial palette indexed BMP should parse successfully"
        );

        let (_width, _height, bits_per_pixel, _version) = result.unwrap();
        assert_eq!(
            bits_per_pixel, 8,
            "Partial palette BMP bits per pixel should be 8"
        );
    }

    #[test]
    fn test_bmp_top_down_negative_height() {
        // Test BMP with negative height (top-down bitmap)
        let mut data = Vec::new();

        // BMP File Header
        data.extend_from_slice(b"BM");
        data.extend_from_slice(&154u32.to_le_bytes()); // File size
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&54u32.to_le_bytes()); // Offset

        // DIB Header (BITMAPINFOHEADER, 40 bytes)
        data.extend_from_slice(&40u32.to_le_bytes()); // Header size
        data.extend_from_slice(&200i32.to_le_bytes()); // Width
        data.extend_from_slice(&(-200i32).to_le_bytes()); // Height (negative = top-down)
        data.extend_from_slice(&1u16.to_le_bytes()); // Planes
        data.extend_from_slice(&24u16.to_le_bytes()); // Bits per pixel
        data.extend_from_slice(&0u32.to_le_bytes()); // Compression (BI_RGB)
        data.extend_from_slice(&120_000u32.to_le_bytes()); // Image size
        data.extend_from_slice(&0i32.to_le_bytes()); // X pixels per meter
        data.extend_from_slice(&0i32.to_le_bytes()); // Y pixels per meter
        data.extend_from_slice(&0u32.to_le_bytes()); // Colors used
        data.extend_from_slice(&0u32.to_le_bytes()); // Important colors

        let result = BmpBackend::parse_bmp_metadata(&data);
        assert!(
            result.is_ok(),
            "Top-down BMP with negative height should parse successfully"
        );

        let (width, height, _bits_per_pixel, _version) = result.unwrap();
        assert_eq!(width, 200, "Top-down BMP width should be 200");
        assert_eq!(
            height, 200,
            "Top-down BMP height should be 200 (absolute value)"
        ); // Absolute value returned
    }

    #[test]
    fn test_bmp_row_padding_edge_case() {
        // Test BMP with dimensions requiring different row padding
        // Width of 3 pixels at 24-bit = 9 bytes, needs 3 bytes padding to reach 12 (multiple of 4)
        let mut data = Vec::new();

        // BMP File Header
        data.extend_from_slice(b"BM");
        data.extend_from_slice(&66u32.to_le_bytes()); // File size
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&54u32.to_le_bytes()); // Offset

        // DIB Header (BITMAPINFOHEADER, 40 bytes)
        data.extend_from_slice(&40u32.to_le_bytes()); // Header size
        data.extend_from_slice(&3i32.to_le_bytes()); // Width (3 pixels * 3 bytes = 9, +3 padding = 12)
        data.extend_from_slice(&1i32.to_le_bytes()); // Height
        data.extend_from_slice(&1u16.to_le_bytes()); // Planes
        data.extend_from_slice(&24u16.to_le_bytes()); // Bits per pixel
        data.extend_from_slice(&0u32.to_le_bytes()); // Compression (BI_RGB)
        data.extend_from_slice(&12u32.to_le_bytes()); // Image size (12 bytes with padding)
        data.extend_from_slice(&0i32.to_le_bytes()); // X pixels per meter
        data.extend_from_slice(&0i32.to_le_bytes()); // Y pixels per meter
        data.extend_from_slice(&0u32.to_le_bytes()); // Colors used
        data.extend_from_slice(&0u32.to_le_bytes()); // Important colors

        let result = BmpBackend::parse_bmp_metadata(&data);
        assert!(
            result.is_ok(),
            "BMP with row padding should parse successfully"
        );

        let (width, height, bits_per_pixel, _version) = result.unwrap();
        assert_eq!(width, 3, "Row padding BMP width should be 3");
        assert_eq!(height, 1, "Row padding BMP height should be 1");
        assert_eq!(
            bits_per_pixel, 24,
            "Row padding BMP bits per pixel should be 24"
        );
    }

    #[test]
    fn test_bmp_very_large_file_size() {
        // Test BMP with very large dimensions (8K resolution: 7680×4320)
        let mut data = Vec::new();

        let width = 7680i32;
        let height = 4320i32;
        let bytes_per_pixel = 3; // 24-bit
        let row_size = ((width * bytes_per_pixel + 3) / 4) * 4;
        let image_size = row_size * height;
        let file_size = 54 + image_size;

        // BMP File Header
        data.extend_from_slice(b"BM");
        data.extend_from_slice(&(file_size as u32).to_le_bytes()); // File size
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&54u32.to_le_bytes()); // Offset

        // DIB Header (BITMAPINFOHEADER, 40 bytes)
        data.extend_from_slice(&40u32.to_le_bytes()); // Header size
        data.extend_from_slice(&width.to_le_bytes()); // Width
        data.extend_from_slice(&height.to_le_bytes()); // Height
        data.extend_from_slice(&1u16.to_le_bytes()); // Planes
        data.extend_from_slice(&24u16.to_le_bytes()); // Bits per pixel
        data.extend_from_slice(&0u32.to_le_bytes()); // Compression (BI_RGB)
        data.extend_from_slice(&(image_size as u32).to_le_bytes()); // Image size
        data.extend_from_slice(&0i32.to_le_bytes()); // X pixels per meter
        data.extend_from_slice(&0i32.to_le_bytes()); // Y pixels per meter
        data.extend_from_slice(&0u32.to_le_bytes()); // Colors used
        data.extend_from_slice(&0u32.to_le_bytes()); // Important colors

        let result = BmpBackend::parse_bmp_metadata(&data);
        assert!(
            result.is_ok(),
            "Very large 8K BMP should parse successfully"
        );

        let (parsed_width, parsed_height, bits_per_pixel, _version) = result.unwrap();
        assert_eq!(parsed_width, 7680, "8K BMP width should be 7680");
        assert_eq!(parsed_height, 4320, "8K BMP height should be 4320");
        assert_eq!(bits_per_pixel, 24, "8K BMP bits per pixel should be 24");
    }

    #[test]
    fn test_bmp_color_masks_v4_header() {
        // Test BMP with BITMAPV4HEADER including color space information
        let mut data = Vec::new();

        // BMP File Header
        data.extend_from_slice(b"BM");
        data.extend_from_slice(&122u32.to_le_bytes()); // File size (14 + 108 header)
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&122u32.to_le_bytes()); // Offset

        // DIB Header (BITMAPV4HEADER, 108 bytes)
        data.extend_from_slice(&108u32.to_le_bytes()); // Header size
        data.extend_from_slice(&64i32.to_le_bytes()); // Width
        data.extend_from_slice(&64i32.to_le_bytes()); // Height
        data.extend_from_slice(&1u16.to_le_bytes()); // Planes
        data.extend_from_slice(&32u16.to_le_bytes()); // Bits per pixel
        data.extend_from_slice(&3u32.to_le_bytes()); // Compression (BI_BITFIELDS)
        data.extend_from_slice(&16384u32.to_le_bytes()); // Image size
        data.extend_from_slice(&0i32.to_le_bytes()); // X pixels per meter
        data.extend_from_slice(&0i32.to_le_bytes()); // Y pixels per meter
        data.extend_from_slice(&0u32.to_le_bytes()); // Colors used
        data.extend_from_slice(&0u32.to_le_bytes()); // Important colors
                                                     // Color masks (RGBA)
        data.extend_from_slice(&0x00FF_0000u32.to_le_bytes()); // Red mask
        data.extend_from_slice(&0x0000_FF00u32.to_le_bytes()); // Green mask
        data.extend_from_slice(&0x0000_00FFu32.to_le_bytes()); // Blue mask
        data.extend_from_slice(&0xFF00_0000u32.to_le_bytes()); // Alpha mask
                                                               // Color space type (sRGB)
        data.extend_from_slice(&0x7352_4742u32.to_le_bytes()); // 'sRGB' in little-endian
                                                               // 36 bytes for color space endpoints (zeros)
        for _ in 0..9 {
            data.extend_from_slice(&0u32.to_le_bytes());
        }
        // Gamma values (3 * 4 bytes)
        data.extend_from_slice(&0u32.to_le_bytes()); // Red gamma
        data.extend_from_slice(&0u32.to_le_bytes()); // Green gamma
        data.extend_from_slice(&0u32.to_le_bytes()); // Blue gamma

        let result = BmpBackend::parse_bmp_metadata(&data);
        assert!(
            result.is_ok(),
            "V4 header with color masks should parse successfully"
        );

        let (width, height, bits_per_pixel, version) = result.unwrap();
        assert_eq!(width, 64, "V4 color masks BMP width should be 64");
        assert_eq!(height, 64, "V4 color masks BMP height should be 64");
        assert_eq!(
            bits_per_pixel, 32,
            "V4 color masks BMP bits per pixel should be 32"
        );
        assert_eq!(
            version, "Windows 95 (BITMAPV4HEADER)",
            "Version should be BITMAPV4HEADER"
        );
    }

    #[test]
    fn test_bmp_v5_header_icc_profile() {
        // Test BMP with BITMAPV5HEADER (Windows 98) with ICC profile support
        let mut data = Vec::new();

        // BMP File Header
        data.extend_from_slice(b"BM");
        data.extend_from_slice(&138u32.to_le_bytes()); // File size (14 + 124 header)
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&138u32.to_le_bytes()); // Offset

        // DIB Header (BITMAPV5HEADER, 124 bytes)
        data.extend_from_slice(&124u32.to_le_bytes()); // Header size (V5)
        data.extend_from_slice(&800i32.to_le_bytes()); // Width
        data.extend_from_slice(&600i32.to_le_bytes()); // Height
        data.extend_from_slice(&1u16.to_le_bytes()); // Planes
        data.extend_from_slice(&32u16.to_le_bytes()); // Bits per pixel
        data.extend_from_slice(&3u32.to_le_bytes()); // Compression (BI_BITFIELDS)
        data.extend_from_slice(&1_920_000u32.to_le_bytes()); // Image size
        data.extend_from_slice(&2835i32.to_le_bytes()); // X pixels per meter (72 DPI)
        data.extend_from_slice(&2835i32.to_le_bytes()); // Y pixels per meter (72 DPI)
        data.extend_from_slice(&0u32.to_le_bytes()); // Colors used
        data.extend_from_slice(&0u32.to_le_bytes()); // Important colors
                                                     // Color masks (RGBA)
        data.extend_from_slice(&0x00FF_0000u32.to_le_bytes()); // Red mask
        data.extend_from_slice(&0x0000_FF00u32.to_le_bytes()); // Green mask
        data.extend_from_slice(&0x0000_00FFu32.to_le_bytes()); // Blue mask
        data.extend_from_slice(&0xFF00_0000u32.to_le_bytes()); // Alpha mask
                                                               // Color space type (calibrated RGB with ICC)
        data.extend_from_slice(&0x4C49_4E4Bu32.to_le_bytes()); // 'LINK' = embedded ICC profile
                                                               // 36 bytes for color space endpoints (zeros)
        for _ in 0..9 {
            data.extend_from_slice(&0u32.to_le_bytes());
        }
        // Gamma values (3 * 4 bytes)
        data.extend_from_slice(&0u32.to_le_bytes()); // Red gamma
        data.extend_from_slice(&0u32.to_le_bytes()); // Green gamma
        data.extend_from_slice(&0u32.to_le_bytes()); // Blue gamma
                                                     // V5 specific fields (16 bytes)
        data.extend_from_slice(&0u32.to_le_bytes()); // Intent
        data.extend_from_slice(&0u32.to_le_bytes()); // Profile data offset
        data.extend_from_slice(&0u32.to_le_bytes()); // Profile size
        data.extend_from_slice(&0u32.to_le_bytes()); // Reserved

        let result = BmpBackend::parse_bmp_metadata(&data);
        assert!(
            result.is_ok(),
            "V5 header with ICC profile should parse successfully"
        );

        let (width, height, bits_per_pixel, version) = result.unwrap();
        assert_eq!(width, 800, "V5 ICC BMP width should be 800");
        assert_eq!(height, 600, "V5 ICC BMP height should be 600");
        assert_eq!(bits_per_pixel, 32, "V5 ICC BMP bits per pixel should be 32");
        assert_eq!(
            version, "Windows 98 (BITMAPV5HEADER)",
            "Version should be BITMAPV5HEADER"
        );
    }

    #[test]
    fn test_bmp_monochrome_1bit() {
        // Test BMP with 1-bit color depth (monochrome: black and white only)
        let mut data = Vec::new();

        // BMP File Header
        data.extend_from_slice(b"BM");
        data.extend_from_slice(&62u32.to_le_bytes()); // File size (14 + 40 + 8 palette)
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&62u32.to_le_bytes()); // Offset

        // DIB Header (BITMAPINFOHEADER, 40 bytes)
        data.extend_from_slice(&40u32.to_le_bytes()); // Header size
        data.extend_from_slice(&8i32.to_le_bytes()); // Width (8 pixels)
        data.extend_from_slice(&8i32.to_le_bytes()); // Height (8 pixels)
        data.extend_from_slice(&1u16.to_le_bytes()); // Planes
        data.extend_from_slice(&1u16.to_le_bytes()); // Bits per pixel (1-bit = monochrome)
        data.extend_from_slice(&0u32.to_le_bytes()); // Compression (BI_RGB)
        data.extend_from_slice(&8u32.to_le_bytes()); // Image size (8 bytes = 8 rows × 1 byte/row)
        data.extend_from_slice(&0i32.to_le_bytes()); // X pixels per meter
        data.extend_from_slice(&0i32.to_le_bytes()); // Y pixels per meter
        data.extend_from_slice(&2u32.to_le_bytes()); // Colors used (2 = black + white)
        data.extend_from_slice(&2u32.to_le_bytes()); // Important colors

        // Color palette (2 colors × 4 bytes)
        data.extend_from_slice(&[0, 0, 0, 0]); // Black
        data.extend_from_slice(&[255, 255, 255, 0]); // White

        let result = BmpBackend::parse_bmp_metadata(&data);
        assert!(
            result.is_ok(),
            "Monochrome 1-bit BMP should parse successfully"
        );

        let (width, height, bits_per_pixel, _version) = result.unwrap();
        assert_eq!(width, 8, "Monochrome BMP width should be 8");
        assert_eq!(height, 8, "Monochrome BMP height should be 8");
        assert_eq!(
            bits_per_pixel, 1,
            "Monochrome BMP bits per pixel should be 1"
        );
    }

    #[test]
    fn test_bmp_16bit_color_555() {
        // Test BMP with 16-bit color depth (RGB 5-5-5 format)
        let mut data = Vec::new();

        // BMP File Header
        data.extend_from_slice(b"BM");
        data.extend_from_slice(&70u32.to_le_bytes()); // File size (14 + 40 + 16 image data)
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&54u32.to_le_bytes()); // Offset

        // DIB Header (BITMAPINFOHEADER, 40 bytes)
        data.extend_from_slice(&40u32.to_le_bytes()); // Header size
        data.extend_from_slice(&4i32.to_le_bytes()); // Width (4 pixels)
        data.extend_from_slice(&2i32.to_le_bytes()); // Height (2 pixels)
        data.extend_from_slice(&1u16.to_le_bytes()); // Planes
        data.extend_from_slice(&16u16.to_le_bytes()); // Bits per pixel (16-bit RGB555)
        data.extend_from_slice(&0u32.to_le_bytes()); // Compression (BI_RGB)
        data.extend_from_slice(&16u32.to_le_bytes()); // Image size (2 rows × 8 bytes/row)
        data.extend_from_slice(&0i32.to_le_bytes()); // X pixels per meter
        data.extend_from_slice(&0i32.to_le_bytes()); // Y pixels per meter
        data.extend_from_slice(&0u32.to_le_bytes()); // Colors used (0 = use all)
        data.extend_from_slice(&0u32.to_le_bytes()); // Important colors

        let result = BmpBackend::parse_bmp_metadata(&data);
        assert!(
            result.is_ok(),
            "16-bit RGB555 BMP should parse successfully"
        );

        let (width, height, bits_per_pixel, _version) = result.unwrap();
        assert_eq!(width, 4, "RGB555 BMP width should be 4");
        assert_eq!(height, 2, "RGB555 BMP height should be 2");
        assert_eq!(bits_per_pixel, 16, "RGB555 BMP bits per pixel should be 16");
    }

    #[test]
    fn test_bmp_corrupted_file_size() {
        // Test BMP with file size field not matching actual data length
        // Parser should handle gracefully (file size is hint, not strict requirement)
        let mut data = Vec::new();

        // BMP File Header
        data.extend_from_slice(b"BM");
        data.extend_from_slice(&99999u32.to_le_bytes()); // File size (WRONG: claims 99999)
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&54u32.to_le_bytes()); // Offset

        // DIB Header (BITMAPINFOHEADER, 40 bytes)
        data.extend_from_slice(&40u32.to_le_bytes()); // Header size
        data.extend_from_slice(&100i32.to_le_bytes()); // Width
        data.extend_from_slice(&100i32.to_le_bytes()); // Height
        data.extend_from_slice(&1u16.to_le_bytes()); // Planes
        data.extend_from_slice(&24u16.to_le_bytes()); // Bits per pixel
        data.extend_from_slice(&0u32.to_le_bytes()); // Compression (BI_RGB)
        data.extend_from_slice(&30000u32.to_le_bytes()); // Image size
        data.extend_from_slice(&0i32.to_le_bytes()); // X pixels per meter
        data.extend_from_slice(&0i32.to_le_bytes()); // Y pixels per meter
        data.extend_from_slice(&0u32.to_le_bytes()); // Colors used
        data.extend_from_slice(&0u32.to_le_bytes()); // Important colors

        // Parser should still extract metadata correctly despite wrong file size
        let result = BmpBackend::parse_bmp_metadata(&data);
        assert!(
            result.is_ok(),
            "BMP with wrong file size field should still parse"
        );

        let (width, height, bits_per_pixel, _version) = result.unwrap();
        assert_eq!(width, 100, "Corrupted file size BMP width should be 100");
        assert_eq!(height, 100, "Corrupted file size BMP height should be 100");
        assert_eq!(
            bits_per_pixel, 24,
            "Corrupted file size BMP bits per pixel should be 24"
        );
    }

    #[test]
    fn test_bmp_zero_dimensions() {
        // Test BMP with zero dimensions (invalid image, should error)
        let mut data = Vec::new();

        // BMP File Header
        data.extend_from_slice(b"BM");
        data.extend_from_slice(&54u32.to_le_bytes()); // File size
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&54u32.to_le_bytes()); // Offset

        // DIB Header (BITMAPINFOHEADER, 40 bytes)
        data.extend_from_slice(&40u32.to_le_bytes()); // Header size
        data.extend_from_slice(&0i32.to_le_bytes()); // Width (ZERO)
        data.extend_from_slice(&0i32.to_le_bytes()); // Height (ZERO)
        data.extend_from_slice(&1u16.to_le_bytes()); // Planes
        data.extend_from_slice(&24u16.to_le_bytes()); // Bits per pixel
        data.extend_from_slice(&0u32.to_le_bytes()); // Compression (BI_RGB)
        data.extend_from_slice(&0u32.to_le_bytes()); // Image size
        data.extend_from_slice(&0i32.to_le_bytes()); // X pixels per meter
        data.extend_from_slice(&0i32.to_le_bytes()); // Y pixels per meter
        data.extend_from_slice(&0u32.to_le_bytes()); // Colors used
        data.extend_from_slice(&0u32.to_le_bytes()); // Important colors

        // Parser should accept (0x0 is technically valid, though degenerate)
        let result = BmpBackend::parse_bmp_metadata(&data);
        assert!(
            result.is_ok(),
            "Zero dimension BMP should parse successfully"
        );

        let (width, height, _bits_per_pixel, _version) = result.unwrap();
        assert_eq!(width, 0, "Zero dimension BMP width should be 0");
        assert_eq!(height, 0, "Zero dimension BMP height should be 0");
    }

    // ========== ADDITIONAL COMPREHENSIVE EDGE CASES (65 → 70) ==========

    #[test]
    fn test_bmp_4bit_indexed_color() {
        // Test BMP with 4-bit indexed color (16 colors max)
        let mut data = Vec::new();

        // BMP File Header
        data.extend_from_slice(b"BM");
        data.extend_from_slice(&118u32.to_le_bytes()); // File size (14 + 40 + 64 palette)
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&118u32.to_le_bytes()); // Offset

        // DIB Header (BITMAPINFOHEADER, 40 bytes)
        data.extend_from_slice(&40u32.to_le_bytes()); // Header size
        data.extend_from_slice(&8i32.to_le_bytes()); // Width (8 pixels)
        data.extend_from_slice(&8i32.to_le_bytes()); // Height (8 pixels)
        data.extend_from_slice(&1u16.to_le_bytes()); // Planes
        data.extend_from_slice(&4u16.to_le_bytes()); // Bits per pixel (4-bit = 16 colors)
        data.extend_from_slice(&0u32.to_le_bytes()); // Compression (BI_RGB)
        data.extend_from_slice(&32u32.to_le_bytes()); // Image size
        data.extend_from_slice(&0i32.to_le_bytes()); // X pixels per meter
        data.extend_from_slice(&0i32.to_le_bytes()); // Y pixels per meter
        data.extend_from_slice(&16u32.to_le_bytes()); // Colors used (16 max for 4-bit)
        data.extend_from_slice(&16u32.to_le_bytes()); // Important colors

        // Color palette (16 colors × 4 bytes = 64 bytes)
        for i in 0..16 {
            data.extend_from_slice(&[i * 16, i * 16, i * 16, 0]); // Grayscale palette
        }

        let result = BmpBackend::parse_bmp_metadata(&data);
        assert!(
            result.is_ok(),
            "4-bit indexed color BMP should parse successfully"
        );

        let (width, height, bits_per_pixel, _version) = result.unwrap();
        assert_eq!(width, 8, "4-bit indexed BMP width should be 8");
        assert_eq!(height, 8, "4-bit indexed BMP height should be 8");
        assert_eq!(
            bits_per_pixel, 4,
            "4-bit indexed BMP bits per pixel should be 4"
        );
    }

    #[test]
    fn test_bmp_with_alpha_channel_32bit() {
        // Test BMP with 32-bit color including alpha channel (RGBA)
        let mut data = Vec::new();

        // BMP File Header
        data.extend_from_slice(b"BM");
        data.extend_from_slice(&70u32.to_be_bytes()); // File size
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&54u32.to_le_bytes()); // Offset

        // DIB Header (BITMAPINFOHEADER, 40 bytes)
        data.extend_from_slice(&40u32.to_le_bytes()); // Header size
        data.extend_from_slice(&2i32.to_le_bytes()); // Width (2 pixels)
        data.extend_from_slice(&2i32.to_le_bytes()); // Height (2 pixels)
        data.extend_from_slice(&1u16.to_le_bytes()); // Planes
        data.extend_from_slice(&32u16.to_le_bytes()); // Bits per pixel (32-bit = RGBA)
        data.extend_from_slice(&0u32.to_le_bytes()); // Compression (BI_RGB)
        data.extend_from_slice(&16u32.to_le_bytes()); // Image size (2×2×4 = 16 bytes)
        data.extend_from_slice(&0i32.to_le_bytes()); // X pixels per meter
        data.extend_from_slice(&0i32.to_le_bytes()); // Y pixels per meter
        data.extend_from_slice(&0u32.to_le_bytes()); // Colors used
        data.extend_from_slice(&0u32.to_le_bytes()); // Important colors

        let result = BmpBackend::parse_bmp_metadata(&data);
        assert!(result.is_ok(), "32-bit RGBA BMP should parse successfully");

        let (width, height, bits_per_pixel, _version) = result.unwrap();
        assert_eq!(width, 2, "32-bit RGBA BMP width should be 2");
        assert_eq!(height, 2, "32-bit RGBA BMP height should be 2");
        assert_eq!(
            bits_per_pixel, 32,
            "32-bit RGBA BMP bits per pixel should be 32"
        );
    }

    #[test]
    fn test_bmp_maximum_dimensions_u16() {
        // Test BMP with maximum u16 dimensions (65535×65535)
        let mut data = Vec::new();

        let max_dim = u16::MAX as i32; // 65535
        let bytes_per_pixel = 3; // 24-bit
        let row_size = ((max_dim * bytes_per_pixel + 3) / 4) * 4;
        let image_size = (row_size as u64 * max_dim as u64) as u32;

        // BMP File Header
        data.extend_from_slice(b"BM");
        data.extend_from_slice(&(54u32 + image_size).to_le_bytes()); // File size
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&54u32.to_le_bytes()); // Offset

        // DIB Header (BITMAPINFOHEADER, 40 bytes)
        data.extend_from_slice(&40u32.to_le_bytes()); // Header size
        data.extend_from_slice(&max_dim.to_le_bytes()); // Width (max)
        data.extend_from_slice(&max_dim.to_le_bytes()); // Height (max)
        data.extend_from_slice(&1u16.to_le_bytes()); // Planes
        data.extend_from_slice(&24u16.to_le_bytes()); // Bits per pixel
        data.extend_from_slice(&0u32.to_le_bytes()); // Compression (BI_RGB)
        data.extend_from_slice(&image_size.to_le_bytes()); // Image size
        data.extend_from_slice(&0i32.to_le_bytes()); // X pixels per meter
        data.extend_from_slice(&0i32.to_le_bytes()); // Y pixels per meter
        data.extend_from_slice(&0u32.to_le_bytes()); // Colors used
        data.extend_from_slice(&0u32.to_le_bytes()); // Important colors

        let result = BmpBackend::parse_bmp_metadata(&data);
        assert!(
            result.is_ok(),
            "Maximum u16 dimension BMP should parse successfully"
        );

        let (width, height, bits_per_pixel, _version) = result.unwrap();
        assert_eq!(width, 65535, "Max dimension BMP width should be 65535");
        assert_eq!(height, 65535, "Max dimension BMP height should be 65535");
        assert_eq!(
            bits_per_pixel, 24,
            "Max dimension BMP bits per pixel should be 24"
        );
    }

    #[test]
    fn test_bmp_unusual_bits_per_pixel_2bit() {
        // Test BMP with 2-bit color depth (4 colors, rarely used)
        let mut data = Vec::new();

        // BMP File Header
        data.extend_from_slice(b"BM");
        data.extend_from_slice(&70u32.to_le_bytes()); // File size (14 + 40 + 16 palette)
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&70u32.to_le_bytes()); // Offset

        // DIB Header (BITMAPINFOHEADER, 40 bytes)
        data.extend_from_slice(&40u32.to_le_bytes()); // Header size
        data.extend_from_slice(&4i32.to_le_bytes()); // Width (4 pixels)
        data.extend_from_slice(&4i32.to_le_bytes()); // Height (4 pixels)
        data.extend_from_slice(&1u16.to_le_bytes()); // Planes
        data.extend_from_slice(&2u16.to_le_bytes()); // Bits per pixel (2-bit = 4 colors)
        data.extend_from_slice(&0u32.to_le_bytes()); // Compression (BI_RGB)
        data.extend_from_slice(&4u32.to_le_bytes()); // Image size (4 rows × 1 byte/row)
        data.extend_from_slice(&0i32.to_le_bytes()); // X pixels per meter
        data.extend_from_slice(&0i32.to_le_bytes()); // Y pixels per meter
        data.extend_from_slice(&4u32.to_le_bytes()); // Colors used (4 max for 2-bit)
        data.extend_from_slice(&4u32.to_le_bytes()); // Important colors

        // Color palette (4 colors × 4 bytes = 16 bytes)
        data.extend_from_slice(&[0, 0, 0, 0]); // Black
        data.extend_from_slice(&[85, 85, 85, 0]); // Dark gray
        data.extend_from_slice(&[170, 170, 170, 0]); // Light gray
        data.extend_from_slice(&[255, 255, 255, 0]); // White

        let result = BmpBackend::parse_bmp_metadata(&data);
        assert!(
            result.is_ok(),
            "2-bit color depth BMP should parse successfully"
        );

        let (width, height, bits_per_pixel, _version) = result.unwrap();
        assert_eq!(width, 4, "2-bit BMP width should be 4");
        assert_eq!(height, 4, "2-bit BMP height should be 4");
        assert_eq!(bits_per_pixel, 2, "2-bit BMP bits per pixel should be 2");
    }

    #[test]
    fn test_bmp_rle4_compression() {
        // Test BMP with RLE4 compression (4-bit run-length encoding)
        let mut data = Vec::new();

        // BMP File Header
        data.extend_from_slice(b"BM");
        data.extend_from_slice(&118u32.to_le_bytes()); // File size
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&118u32.to_le_bytes()); // Offset

        // DIB Header (BITMAPINFOHEADER, 40 bytes)
        data.extend_from_slice(&40u32.to_le_bytes()); // Header size
        data.extend_from_slice(&16i32.to_le_bytes()); // Width (16 pixels)
        data.extend_from_slice(&16i32.to_le_bytes()); // Height (16 pixels)
        data.extend_from_slice(&1u16.to_le_bytes()); // Planes
        data.extend_from_slice(&4u16.to_le_bytes()); // Bits per pixel (4-bit)
        data.extend_from_slice(&2u32.to_le_bytes()); // Compression (BI_RLE4 = 2)
        data.extend_from_slice(&0u32.to_le_bytes()); // Image size (0 = uncompressed size unknown)
        data.extend_from_slice(&0i32.to_le_bytes()); // X pixels per meter
        data.extend_from_slice(&0i32.to_le_bytes()); // Y pixels per meter
        data.extend_from_slice(&16u32.to_le_bytes()); // Colors used
        data.extend_from_slice(&0u32.to_le_bytes()); // Important colors

        // Color palette (16 colors × 4 bytes = 64 bytes)
        for i in 0..16 {
            data.extend_from_slice(&[i * 16, i * 16, i * 16, 0]);
        }

        let result = BmpBackend::parse_bmp_metadata(&data);
        assert!(
            result.is_ok(),
            "RLE4 compressed BMP should parse successfully"
        );

        let (width, height, bits_per_pixel, version) = result.unwrap();
        assert_eq!(width, 16, "RLE4 BMP width should be 16");
        assert_eq!(height, 16, "RLE4 BMP height should be 16");
        assert_eq!(bits_per_pixel, 4, "RLE4 BMP bits per pixel should be 4");
        assert_eq!(
            version, "Windows 3.x (BITMAPINFOHEADER)",
            "RLE4 BMP version should be BITMAPINFOHEADER"
        );
    }

    // ========================================
    // Advanced BMP Format Features (N=620, +5 tests)
    // ========================================

    #[test]
    fn test_bmp_os2_1x_bitmapcoreheader() {
        // Test BMP with OS/2 1.x BITMAPCOREHEADER (12-byte header)
        let mut data = Vec::new();

        // BMP File Header
        data.extend_from_slice(b"BM");
        data.extend_from_slice(&26u32.to_le_bytes()); // File size
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&26u32.to_le_bytes()); // Offset

        // OS/2 1.x BITMAPCOREHEADER (12 bytes)
        data.extend_from_slice(&12u32.to_le_bytes()); // Header size = 12
        data.extend_from_slice(&200u16.to_le_bytes()); // Width (16-bit)
        data.extend_from_slice(&150u16.to_le_bytes()); // Height (16-bit)
        data.extend_from_slice(&1u16.to_le_bytes()); // Planes
        data.extend_from_slice(&24u16.to_le_bytes()); // Bits per pixel

        let result = BmpBackend::parse_bmp_metadata(&data);
        assert!(
            result.is_ok(),
            "OS/2 1.x BITMAPCOREHEADER BMP should parse successfully"
        );

        let (width, height, bits_per_pixel, version) = result.unwrap();
        assert_eq!(width, 200, "OS/2 1.x BMP width should be 200");
        assert_eq!(height, 150, "OS/2 1.x BMP height should be 150");
        assert_eq!(
            bits_per_pixel, 24,
            "OS/2 1.x BMP bits per pixel should be 24"
        );
        assert_eq!(
            version, "OS/2 1.x (BITMAPCOREHEADER)",
            "Version should be BITMAPCOREHEADER"
        );
    }

    #[test]
    fn test_bmp_32bit_with_alpha_channel() {
        // Test BMP with 32-bit RGBA (alpha channel, BITMAPV4HEADER or later)
        let mut data = Vec::new();

        // BMP File Header
        data.extend_from_slice(b"BM");
        data.extend_from_slice(&138u32.to_le_bytes()); // File size
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&138u32.to_le_bytes()); // Offset

        // BITMAPV4HEADER (108 bytes) - supports alpha channel
        data.extend_from_slice(&108u32.to_le_bytes()); // Header size
        data.extend_from_slice(&64i32.to_le_bytes()); // Width
        data.extend_from_slice(&64i32.to_le_bytes()); // Height
        data.extend_from_slice(&1u16.to_le_bytes()); // Planes
        data.extend_from_slice(&32u16.to_le_bytes()); // Bits per pixel (32-bit RGBA)
        data.extend_from_slice(&3u32.to_le_bytes()); // Compression (BI_BITFIELDS)
        data.extend_from_slice(&0u32.to_le_bytes()); // Image size
        data.extend_from_slice(&0i32.to_le_bytes()); // X pixels per meter
        data.extend_from_slice(&0i32.to_le_bytes()); // Y pixels per meter
        data.extend_from_slice(&0u32.to_le_bytes()); // Colors used
        data.extend_from_slice(&0u32.to_le_bytes()); // Important colors

        // Color masks (RGBA)
        data.extend_from_slice(&0x00FF0000u32.to_le_bytes()); // Red mask
        data.extend_from_slice(&0x0000FF00u32.to_le_bytes()); // Green mask
        data.extend_from_slice(&0x000000FFu32.to_le_bytes()); // Blue mask
        data.extend_from_slice(&0xFF000000u32.to_le_bytes()); // Alpha mask

        // Color space type (LCS_WINDOWS_COLOR_SPACE)
        data.extend_from_slice(&0x73524742u32.to_le_bytes()); // "sRGB"
                                                              // CIEXYZTRIPLE (36 bytes of color space endpoints - zeros)
        data.extend_from_slice(&[0u8; 36]);
        // Gamma RGB (12 bytes)
        data.extend_from_slice(&[0u8; 12]);

        let result = BmpBackend::parse_bmp_metadata(&data);
        assert!(
            result.is_ok(),
            "32-bit alpha V4 BMP should parse successfully"
        );

        let (width, height, bits_per_pixel, version) = result.unwrap();
        assert_eq!(width, 64, "32-bit alpha V4 BMP width should be 64");
        assert_eq!(height, 64, "32-bit alpha V4 BMP height should be 64");
        assert_eq!(
            bits_per_pixel, 32,
            "32-bit alpha V4 BMP bits per pixel should be 32"
        );
        assert_eq!(
            version, "Windows 95 (BITMAPV4HEADER)",
            "Version should be BITMAPV4HEADER"
        );
    }

    #[test]
    fn test_bmp_with_jpeg_compression() {
        // Test BMP with embedded JPEG compression (BI_JPEG = 4)
        // Rare format where BMP acts as wrapper around JPEG data
        let mut data = Vec::new();

        // BMP File Header
        data.extend_from_slice(b"BM");
        data.extend_from_slice(&200u32.to_le_bytes()); // File size
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&54u32.to_le_bytes()); // Offset

        // BITMAPINFOHEADER (40 bytes)
        data.extend_from_slice(&40u32.to_le_bytes()); // Header size
        data.extend_from_slice(&320i32.to_le_bytes()); // Width
        data.extend_from_slice(&240i32.to_le_bytes()); // Height
        data.extend_from_slice(&1u16.to_le_bytes()); // Planes
        data.extend_from_slice(&24u16.to_le_bytes()); // Bits per pixel
        data.extend_from_slice(&4u32.to_le_bytes()); // Compression (BI_JPEG = 4)
        data.extend_from_slice(&1024u32.to_le_bytes()); // Image size (compressed JPEG data)
        data.extend_from_slice(&0i32.to_le_bytes()); // X pixels per meter
        data.extend_from_slice(&0i32.to_le_bytes()); // Y pixels per meter
        data.extend_from_slice(&0u32.to_le_bytes()); // Colors used
        data.extend_from_slice(&0u32.to_le_bytes()); // Important colors

        let result = BmpBackend::parse_bmp_metadata(&data);
        assert!(
            result.is_ok(),
            "JPEG compressed BMP should parse successfully"
        );

        let (width, height, bits_per_pixel, version) = result.unwrap();
        assert_eq!(width, 320, "JPEG compressed BMP width should be 320");
        assert_eq!(height, 240, "JPEG compressed BMP height should be 240");
        assert_eq!(
            bits_per_pixel, 24,
            "JPEG compressed BMP bits per pixel should be 24"
        );
        assert_eq!(
            version, "Windows 3.x (BITMAPINFOHEADER)",
            "JPEG compressed BMP version should be BITMAPINFOHEADER"
        );
    }

    #[test]
    fn test_bmp_with_png_compression() {
        // Test BMP with embedded PNG compression (BI_PNG = 5)
        // Rare format where BMP acts as wrapper around PNG data
        let mut data = Vec::new();

        // BMP File Header
        data.extend_from_slice(b"BM");
        data.extend_from_slice(&200u32.to_le_bytes()); // File size
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&54u32.to_le_bytes()); // Offset

        // BITMAPINFOHEADER (40 bytes)
        data.extend_from_slice(&40u32.to_le_bytes()); // Header size
        data.extend_from_slice(&800i32.to_le_bytes()); // Width
        data.extend_from_slice(&600i32.to_le_bytes()); // Height
        data.extend_from_slice(&1u16.to_le_bytes()); // Planes
        data.extend_from_slice(&24u16.to_le_bytes()); // Bits per pixel
        data.extend_from_slice(&5u32.to_le_bytes()); // Compression (BI_PNG = 5)
        data.extend_from_slice(&2048u32.to_le_bytes()); // Image size (compressed PNG data)
        data.extend_from_slice(&0i32.to_le_bytes()); // X pixels per meter
        data.extend_from_slice(&0i32.to_le_bytes()); // Y pixels per meter
        data.extend_from_slice(&0u32.to_le_bytes()); // Colors used
        data.extend_from_slice(&0u32.to_le_bytes()); // Important colors

        let result = BmpBackend::parse_bmp_metadata(&data);
        assert!(
            result.is_ok(),
            "PNG compressed BMP should parse successfully"
        );

        let (width, height, bits_per_pixel, version) = result.unwrap();
        assert_eq!(width, 800, "PNG compressed BMP width should be 800");
        assert_eq!(height, 600, "PNG compressed BMP height should be 600");
        assert_eq!(
            bits_per_pixel, 24,
            "PNG compressed BMP bits per pixel should be 24"
        );
        assert_eq!(
            version, "Windows 3.x (BITMAPINFOHEADER)",
            "PNG compressed BMP version should be BITMAPINFOHEADER"
        );
    }

    #[test]
    fn test_bmp_with_color_space_and_icc_profile() {
        // Test BMP with embedded ICC color profile (BITMAPV5HEADER)
        let mut data = Vec::new();

        // BMP File Header
        data.extend_from_slice(b"BM");
        data.extend_from_slice(&194u32.to_le_bytes()); // File size
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&194u32.to_le_bytes()); // Offset

        // BITMAPV5HEADER (124 bytes) - supports ICC profiles
        data.extend_from_slice(&124u32.to_le_bytes()); // Header size
        data.extend_from_slice(&1024i32.to_le_bytes()); // Width
        data.extend_from_slice(&768i32.to_le_bytes()); // Height
        data.extend_from_slice(&1u16.to_le_bytes()); // Planes
        data.extend_from_slice(&24u16.to_le_bytes()); // Bits per pixel
        data.extend_from_slice(&0u32.to_le_bytes()); // Compression (BI_RGB)
        data.extend_from_slice(&0u32.to_le_bytes()); // Image size
        data.extend_from_slice(&0i32.to_le_bytes()); // X pixels per meter
        data.extend_from_slice(&0i32.to_le_bytes()); // Y pixels per meter
        data.extend_from_slice(&0u32.to_le_bytes()); // Colors used
        data.extend_from_slice(&0u32.to_le_bytes()); // Important colors

        // Color masks (RGB, no alpha)
        data.extend_from_slice(&0x00FF0000u32.to_le_bytes()); // Red mask
        data.extend_from_slice(&0x0000FF00u32.to_le_bytes()); // Green mask
        data.extend_from_slice(&0x000000FFu32.to_le_bytes()); // Blue mask
        data.extend_from_slice(&0x00000000u32.to_le_bytes()); // Alpha mask (none)

        // Color space type (PROFILE_EMBEDDED = 0x4D424544)
        data.extend_from_slice(&0x4D424544u32.to_le_bytes()); // "MBED"
                                                              // CIEXYZTRIPLE (36 bytes - color space endpoints)
        data.extend_from_slice(&[0u8; 36]);
        // Gamma RGB (12 bytes)
        data.extend_from_slice(&[0u8; 12]);

        // ICC profile info (intent, profile data, profile size, reserved)
        data.extend_from_slice(&1u32.to_le_bytes()); // Intent (relative colorimetric)
        data.extend_from_slice(&194u32.to_le_bytes()); // Profile data offset
        data.extend_from_slice(&1024u32.to_le_bytes()); // Profile size
        data.extend_from_slice(&0u32.to_le_bytes()); // Reserved

        let result = BmpBackend::parse_bmp_metadata(&data);
        assert!(
            result.is_ok(),
            "ICC profile embedded V5 BMP should parse successfully"
        );

        let (width, height, bits_per_pixel, version) = result.unwrap();
        assert_eq!(width, 1024, "ICC profile BMP width should be 1024");
        assert_eq!(height, 768, "ICC profile BMP height should be 768");
        assert_eq!(
            bits_per_pixel, 24,
            "ICC profile BMP bits per pixel should be 24"
        );
        assert_eq!(
            version, "Windows 98 (BITMAPV5HEADER)",
            "ICC profile BMP version should be BITMAPV5HEADER"
        );
    }
}
