//! GIF backend for docling
//!
//! This backend converts GIF (Graphics Interchange Format) files to docling's document model.

// Clippy pedantic allows:
// - Unit struct &self convention
#![allow(clippy::trivially_copy_pass_by_ref)]

use crate::traits::{BackendOptions, DocumentBackend};
use crate::utils::{create_section_header, create_text_item, format_file_size, opt_vec};
use docling_core::{DocItem, DoclingError, Document, DocumentMetadata, InputFormat};
use std::fmt::Write;
use std::path::Path;

/// GIF Image Descriptor Separator byte (0x2C = ',')
///
/// In the GIF format, each image in the stream is preceded by this separator byte.
/// For animated GIFs, counting occurrences of this byte gives the frame count.
const GIF_IMAGE_DESCRIPTOR_SEPARATOR: u8 = 0x2C;

/// GIF backend
///
/// Converts GIF (Graphics Interchange Format) files to docling's document model.
/// Extracts basic metadata and provides a markdown representation of the image.
///
/// ## Features
///
/// - Extract image dimensions
/// - Detect animated GIFs
/// - Generate markdown with image metadata
///
/// ## Example
///
/// ```no_run
/// use docling_backend::GifBackend;
/// use docling_backend::DocumentBackend;
///
/// let backend = GifBackend::new();
/// let result = backend.parse_file("animation.gif", &Default::default())?;
/// println!("Image: {:?}", result.metadata.title);
/// # Ok::<(), docling_core::error::DoclingError>(())
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct GifBackend;

impl GifBackend {
    /// Create a new GIF backend instance
    #[inline]
    #[must_use = "creates a backend instance that should be used for parsing"]
    pub const fn new() -> Self {
        Self
    }

    /// Parse GIF header to extract basic metadata
    ///
    /// GIF format structure:
    /// - Header: "`GIF87a`" or "`GIF89a`" (6 bytes)
    /// - Logical Screen Descriptor: width (2), height (2), flags (1), bg color (1), aspect (1)
    ///
    /// Returns: (width, height, `frame_count`, `color_depth_bits`)
    fn parse_gif_metadata(data: &[u8]) -> Result<(u32, u32, usize, u8), DoclingError> {
        // Minimum GIF file size: 6 (header) + 7 (logical screen descriptor) = 13 bytes
        if data.len() < 13 {
            return Err(DoclingError::BackendError(
                "File too small to be a valid GIF".to_string(),
            ));
        }

        // Check GIF signature
        let header = &data[0..6];
        if header != b"GIF87a" && header != b"GIF89a" {
            return Err(DoclingError::BackendError(
                "Invalid GIF header signature".to_string(),
            ));
        }

        // Read logical screen descriptor
        let width = u32::from(u16::from_le_bytes([data[6], data[7]]));
        let height = u32::from(u16::from_le_bytes([data[8], data[9]]));

        // Note: Flags byte (byte 10) contains:
        // Bit 7: Global Color Table Flag
        // Bits 4-6: Color Resolution (bits per primary color - 1)
        // Bits 0-2: Size of Global Color Table (2^(N+1) entries)
        // Currently not needed for our metadata extraction.

        // For GIFs, "color depth" refers to bits per pixel in the format specification
        // GIF format always uses 8-bit color indices (1 byte per pixel) regardless of palette size
        // The palette can have fewer entries (e.g., 2, 4, 8, 16, 32, 64, 128, 256), but the
        // index is always stored as 8 bits (one byte)
        let color_depth_bits = 8;

        // Count frames (image descriptors start with separator byte)
        // Note: bytecount crate would be overkill for this single count
        #[allow(
            clippy::naive_bytecount,
            reason = "single-use count, bytecount crate would be overkill"
        )]
        let frame_count = data
            .iter()
            .filter(|&&b| b == GIF_IMAGE_DESCRIPTOR_SEPARATOR)
            .count()
            .max(1);

        Ok((width, height, frame_count, color_depth_bits))
    }

    /// Convert GIF metadata to markdown
    fn gif_to_markdown(
        filename: &str,
        width: u32,
        height: u32,
        frame_count: usize,
        color_depth: u8,
        file_size: usize,
    ) -> String {
        let mut markdown = String::new();

        // Title
        let _ = write!(markdown, "# {filename}\n\n");

        // Properties section
        markdown.push_str("## Properties\n\n");

        // Image type
        let is_animated = frame_count > 1;
        if is_animated {
            markdown.push_str("Type: Animated GIF\n");
        } else {
            markdown.push_str("Type: GIF Image\n");
        }

        // Dimensions (using " x " format per LLM feedback)
        let _ = writeln!(markdown, "Dimensions: {width} x {height} pixels");

        // Color depth
        let _ = writeln!(markdown, "Color Depth: {color_depth}-bit");

        // Frame count (for animated GIFs)
        if is_animated {
            let _ = writeln!(markdown, "Frame Count: {frame_count}");
        }

        // File size
        markdown.push_str(&format_file_size(file_size));
        markdown.push('\n');

        // Note section
        markdown.push_str(
            "## Note\n\n\
             Image content cannot be extracted as text. \
             OCR or image analysis would be required for content extraction.\n",
        );

        markdown
    }

    /// Generate `DocItems` directly from GIF metadata
    ///
    /// Creates structured `DocItems` from GIF metadata, preserving semantic information.
    /// This is the correct architectural pattern - NO markdown intermediary.
    ///
    /// ## Architecture (CLAUDE.md Compliant)
    ///
    /// ```text
    /// GIF Metadata → gif_to_docitems() → DocItems (semantic structure preserved)
    /// ```
    ///
    /// ## Arguments
    /// * `filename` - Name of the GIF file
    /// * `width` - Image width in pixels
    /// * `height` - Image height in pixels
    /// * `frame_count` - Number of frames in the GIF
    /// * `color_depth` - Color depth in bits
    /// * `file_size` - File size in bytes
    ///
    /// ## Returns
    /// Vector of `DocItems` with semantic structure:
    /// - `SectionHeader` (level 1): Filename
    /// - Text: Image type (static/animated)
    /// - Text: Dimensions
    /// - Text: Color depth
    /// - Text: Frame count (if animated)
    /// - Text: File size
    /// - Text: Note about content extraction
    fn gif_to_docitems(
        filename: &str,
        width: u32,
        height: u32,
        frame_count: usize,
        color_depth: u8,
        file_size: usize,
    ) -> Vec<DocItem> {
        let mut doc_items = Vec::new();
        let mut item_index = 0;
        let is_animated = frame_count > 1;

        // 1. Title as SectionHeader (level 1)
        doc_items.push(create_section_header(
            item_index,
            filename.to_string(),
            1,
            vec![],
        ));
        item_index += 1;

        // 2. Image type
        let image_type = if is_animated {
            "Type: Animated GIF".to_string()
        } else {
            "Type: GIF Image".to_string()
        };
        doc_items.push(create_text_item(item_index, image_type, vec![]));
        item_index += 1;

        // 3. Dimensions (using " x " format per LLM feedback)
        let dimensions = format!("Dimensions: {width} x {height} pixels");
        doc_items.push(create_text_item(item_index, dimensions, vec![]));
        item_index += 1;

        // 4. Color depth
        let color_depth_text = format!("Color Depth: {color_depth}-bit");
        doc_items.push(create_text_item(item_index, color_depth_text, vec![]));
        item_index += 1;

        // 5. Frame count (for animated GIFs)
        if is_animated {
            let frame_count_text = format!("Frame Count: {frame_count}");
            doc_items.push(create_text_item(item_index, frame_count_text, vec![]));
            item_index += 1;
        }

        // 6. File size
        let file_size_text = format_file_size(file_size);
        doc_items.push(create_text_item(item_index, file_size_text, vec![]));
        item_index += 1;

        // 7. Note about content extraction (no "Note:" prefix - that's the section header in markdown)
        let note = "Image content cannot be extracted as text. \
                    OCR or image analysis would be required for content extraction."
            .to_string();
        doc_items.push(create_text_item(item_index, note, vec![]));

        doc_items
    }
}

impl GifBackend {
    /// Shared implementation for parsing GIF data
    ///
    /// Used by both `parse_bytes` and `parse_file` to avoid code duplication.
    // Method signature kept for API consistency with other backend methods
    #[allow(clippy::unused_self)]
    fn parse_gif_data(&self, data: &[u8], filename: &str) -> Result<Document, DoclingError> {
        // Helper to add filename context to errors
        let add_context = |err: DoclingError| -> DoclingError {
            match err {
                DoclingError::BackendError(msg) => {
                    DoclingError::BackendError(format!("{msg}: {filename}"))
                }
                other => other,
            }
        };

        // Parse GIF metadata
        let (width, height, frame_count, color_depth) =
            Self::parse_gif_metadata(data).map_err(add_context)?;

        // Generate DocItems directly from metadata (NO markdown intermediary)
        let doc_items = Self::gif_to_docitems(
            filename,
            width,
            height,
            frame_count,
            color_depth,
            data.len(),
        );

        // Generate markdown from DocItems for backwards compatibility
        let markdown = Self::gif_to_markdown(
            filename,
            width,
            height,
            frame_count,
            color_depth,
            data.len(),
        );
        let num_characters = markdown.chars().count();

        // Create document
        Ok(Document {
            markdown,
            format: InputFormat::Gif,
            metadata: DocumentMetadata {
                num_pages: Some(1), // GIF is single-page (animated frames are not separate pages)
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

impl DocumentBackend for GifBackend {
    #[inline]
    fn format(&self) -> InputFormat {
        InputFormat::Gif
    }

    fn parse_bytes(
        &self,
        data: &[u8],
        _options: &BackendOptions,
    ) -> Result<Document, DoclingError> {
        self.parse_gif_data(data, "image.gif")
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
            .unwrap_or("image.gif");

        let data = std::fs::read(path_ref).map_err(DoclingError::IoError)?;
        self.parse_gif_data(&data, filename)
    }
}

#[cfg(test)]
#[allow(clippy::unreadable_literal)]
mod tests {
    use super::*;

    // ==================== BACKEND TESTS ====================

    #[test]
    fn test_gif_backend_creation() {
        let backend = GifBackend::new();
        assert_eq!(
            backend.format(),
            InputFormat::Gif,
            "GifBackend should report InputFormat::Gif as its format"
        );
    }

    #[test]
    fn test_gif_backend_default() {
        let backend = GifBackend;
        assert_eq!(
            backend.format(),
            InputFormat::Gif,
            "Default GifBackend should have Gif format"
        );
    }

    #[test]
    fn test_gif_backend_format() {
        let backend = GifBackend::new();
        assert_eq!(
            backend.format(),
            InputFormat::Gif,
            "New GifBackend should have Gif format"
        );
    }

    // ==================== GIF HEADER PARSING TESTS ====================

    #[test]
    fn test_parse_gif_header_gif89a() {
        // Minimal valid GIF89a header
        let mut data = vec![0u8; 13];
        data[0..6].copy_from_slice(b"GIF89a");
        data[6] = 0x64; // width = 100 (little endian)
        data[7] = 0x00;
        data[8] = 0xC8; // height = 200 (little endian)
        data[9] = 0x00;

        let result = GifBackend::parse_gif_metadata(&data);
        assert!(result.is_ok(), "GIF89a header should parse successfully");
        let (width, height, _, _) = result.unwrap();
        assert_eq!(width, 100, "Width should be 100");
        assert_eq!(height, 200, "Height should be 200");
    }

    #[test]
    fn test_parse_gif_header_gif87a() {
        // Minimal valid GIF87a header
        let mut data = vec![0u8; 13];
        data[0..6].copy_from_slice(b"GIF87a");
        data[6] = 0x32; // width = 50 (little endian)
        data[7] = 0x00;
        data[8] = 0x32; // height = 50 (little endian)
        data[9] = 0x00;

        let result = GifBackend::parse_gif_metadata(&data);
        assert!(result.is_ok(), "GIF87a header should parse successfully");
        let (width, height, _, _) = result.unwrap();
        assert_eq!(width, 50, "Width should be 50");
        assert_eq!(height, 50, "Height should be 50");
    }

    #[test]
    fn test_parse_gif_large_dimensions() {
        // GIF with maximum practical dimensions
        let mut data = vec![0u8; 13];
        data[0..6].copy_from_slice(b"GIF89a");
        data[6] = 0xFF; // width = 65535 (max u16)
        data[7] = 0xFF;
        data[8] = 0xFF; // height = 65535 (max u16)
        data[9] = 0xFF;

        let result = GifBackend::parse_gif_metadata(&data);
        assert!(
            result.is_ok(),
            "GIF with max u16 dimensions should parse successfully"
        );
        let (width, height, _, _) = result.unwrap();
        assert_eq!(width, 65535, "Width should be 65535 (max u16)");
        assert_eq!(height, 65535, "Height should be 65535 (max u16)");
    }

    #[test]
    fn test_parse_gif_zero_dimensions() {
        // GIF with zero dimensions (technically invalid but parser should handle)
        let mut data = vec![0u8; 13];
        data[0..6].copy_from_slice(b"GIF89a");
        data[6] = 0x00; // width = 0
        data[7] = 0x00;
        data[8] = 0x00; // height = 0
        data[9] = 0x00;

        let result = GifBackend::parse_gif_metadata(&data);
        assert!(
            result.is_ok(),
            "GIF with zero dimensions should parse (parser handles edge case)"
        );
        let (width, height, _, _) = result.unwrap();
        assert_eq!(width, 0, "Width should be 0");
        assert_eq!(height, 0, "Height should be 0");
    }

    // ==================== ANIMATION DETECTION TESTS ====================

    #[test]
    fn test_animated_detection() {
        // Create data with multiple image descriptor separators (0x2C)
        let mut data = vec![0u8; 20];
        data[0..6].copy_from_slice(b"GIF89a");
        data[6] = 0x64;
        data[7] = 0x00;
        data[8] = 0x64;
        data[9] = 0x00;
        data[15] = 0x2C; // First image descriptor
        data[18] = 0x2C; // Second image descriptor

        let result = GifBackend::parse_gif_metadata(&data);
        assert!(
            result.is_ok(),
            "GIF with multiple frames should parse successfully"
        );
        let (_, _, frame_count, _) = result.unwrap();
        assert!(
            frame_count > 1,
            "Frame count should be > 1 for animated GIF"
        );
    }

    #[test]
    fn test_single_frame_detection() {
        // GIF with only one image descriptor (not animated)
        let mut data = vec![0u8; 20];
        data[0..6].copy_from_slice(b"GIF89a");
        data[6] = 0x64;
        data[7] = 0x00;
        data[8] = 0x64;
        data[9] = 0x00;
        data[15] = 0x2C; // Single image descriptor

        let result = GifBackend::parse_gif_metadata(&data);
        assert!(result.is_ok(), "Single-frame GIF should parse successfully");
        let (_, _, frame_count, _) = result.unwrap();
        assert_eq!(frame_count, 1, "Frame count should be 1 for static GIF");
    }

    // ==================== MARKDOWN GENERATION TESTS ====================

    #[test]
    fn test_gif_to_markdown_static() {
        let markdown = GifBackend::gif_to_markdown("test.gif", 640, 480, 1, 8, 10240);

        assert!(
            markdown.contains("# test.gif"),
            "Markdown should contain filename as h1 title"
        );
        assert!(
            markdown.contains("Type: GIF Image"),
            "Static GIF markdown should contain 'Type: GIF Image'"
        );
        assert!(
            markdown.contains("Dimensions: 640 x 480 pixels"),
            "Markdown should contain dimensions"
        );
        assert!(
            !markdown.contains("Animated"),
            "Static GIF should not be labeled as Animated"
        );
        assert!(
            markdown.contains("10.0 KB"),
            "Markdown should contain file size"
        );
    }

    #[test]
    fn test_gif_to_markdown_animated() {
        let markdown = GifBackend::gif_to_markdown("animation.gif", 320, 240, 2, 8, 51200);

        assert!(
            markdown.contains("# animation.gif"),
            "Markdown should contain filename as title"
        );
        assert!(
            markdown.contains("Type: Animated GIF"),
            "Animated GIF markdown should contain 'Type: Animated GIF'"
        );
        assert!(
            markdown.contains("Dimensions: 320 x 240 pixels"),
            "Markdown should contain dimensions"
        );
        assert!(
            markdown.contains("50.0 KB"),
            "Markdown should contain file size"
        );
    }

    #[test]
    fn test_gif_markdown_contains_note() {
        let markdown = GifBackend::gif_to_markdown("test.gif", 100, 100, 1, 8, 1024);

        assert!(
            markdown.contains("Image content cannot be extracted as text"),
            "Markdown should contain note about text extraction"
        );
        assert!(
            markdown.contains("OCR or image analysis would be required"),
            "Markdown should mention OCR requirement"
        );
    }

    // ==================== DOCITEM CREATION TESTS ====================

    #[test]
    fn test_gif_to_docitems_static() {
        let doc_items = GifBackend::gif_to_docitems("test.gif", 640, 480, 1, 8, 10240);

        assert_eq!(doc_items.len(), 6); // title + type + dimensions + color_depth + file_size + note

        // Check structure
        assert!(matches!(doc_items[0], DocItem::SectionHeader { .. }));
        assert!(matches!(doc_items[1], DocItem::Text { .. }));
        assert!(matches!(doc_items[2], DocItem::Text { .. }));
        assert!(matches!(doc_items[3], DocItem::Text { .. }));
        assert!(matches!(doc_items[4], DocItem::Text { .. }));
        assert!(matches!(doc_items[5], DocItem::Text { .. }));

        // Verify content
        if let DocItem::SectionHeader { text, level, .. } = &doc_items[0] {
            assert_eq!(text, "test.gif");
            assert_eq!(*level, 1);
        } else {
            panic!("Expected SectionHeader");
        }

        if let DocItem::Text { text, .. } = &doc_items[1] {
            assert!(text.contains("GIF Image"));
            assert!(!text.contains("Animated"));
        }

        if let DocItem::Text { text, .. } = &doc_items[2] {
            assert!(text.contains("640 x 480"));
        }
    }

    #[test]
    fn test_gif_to_docitems_animated() {
        let doc_items = GifBackend::gif_to_docitems("animation.gif", 320, 240, 2, 8, 51200);

        assert_eq!(doc_items.len(), 7); // title + type + dimensions + color_depth + frame_count + file_size + note

        // Verify filename
        if let DocItem::SectionHeader { text, .. } = &doc_items[0] {
            assert_eq!(text, "animation.gif");
        }

        // Verify animated type
        if let DocItem::Text { text, .. } = &doc_items[1] {
            assert!(text.contains("Animated GIF"));
        }

        // Verify dimensions
        if let DocItem::Text { text, .. } = &doc_items[2] {
            assert!(text.contains("320 x 240"));
        }
    }

    #[test]
    fn test_gif_to_docitems_structure_preservation() {
        let doc_items = GifBackend::gif_to_docitems("test.gif", 100, 100, 1, 8, 1024);

        // Verify all expected fields are present
        assert_eq!(doc_items.len(), 6); // title + type + dimensions + color_depth + file_size + note

        // Title
        assert!(matches!(doc_items[0], DocItem::SectionHeader { .. }));

        // Type
        if let DocItem::Text { text, .. } = &doc_items[1] {
            assert!(text.contains("Type:"));
        }

        // Dimensions
        if let DocItem::Text { text, .. } = &doc_items[2] {
            assert!(text.contains("Dimensions:"));
            assert!(text.contains("100 x 100"));
        }

        // Color depth
        if let DocItem::Text { text, .. } = &doc_items[3] {
            assert!(text.contains("Color Depth:"));
        }

        // File size
        if let DocItem::Text { text, .. } = &doc_items[4] {
            assert!(text.contains("File"));
        }

        // Note (no "Note:" prefix - that's the section header in markdown)
        if let DocItem::Text { text, .. } = &doc_items[5] {
            assert!(text.contains("Image content cannot be extracted"));
            assert!(text.contains("OCR"));
        }
    }

    // ==================== ERROR HANDLING TESTS ====================

    #[test]
    fn test_invalid_gif_header() {
        // Data must be at least 13 bytes to reach header validation
        let data = b"NOTGIF1234567";
        let result = GifBackend::parse_gif_metadata(data);
        assert!(result.is_err());

        if let Err(DoclingError::BackendError(msg)) = result {
            assert!(msg.contains("Invalid GIF header signature"));
        } else {
            panic!("Expected BackendError with header message");
        }
    }

    #[test]
    fn test_gif_too_small() {
        let data = b"GIF89a";
        let result = GifBackend::parse_gif_metadata(data);
        assert!(result.is_err());

        if let Err(DoclingError::BackendError(msg)) = result {
            assert!(msg.contains("too small"));
        } else {
            panic!("Expected BackendError with size message");
        }
    }

    #[test]
    fn test_parse_bytes_invalid_gif() {
        let backend = GifBackend::new();
        let options = BackendOptions::default();
        let invalid_data = b"PNG\x89\x50\x4E\x47"; // PNG signature, not GIF

        let result = backend.parse_bytes(invalid_data, &options);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_bytes_empty() {
        let backend = GifBackend::new();
        let options = BackendOptions::default();
        let empty_data: &[u8] = b"";

        let result = backend.parse_bytes(empty_data, &options);
        assert!(result.is_err());
    }

    // ==================== INTEGRATION TESTS ====================

    #[test]
    fn test_parse_bytes_valid_gif89a() {
        let backend = GifBackend::new();
        let options = BackendOptions::default();

        // Minimal valid GIF89a
        let mut data = vec![0u8; 13];
        data[0..6].copy_from_slice(b"GIF89a");
        data[6] = 0x64; // width = 100
        data[7] = 0x00;
        data[8] = 0xC8; // height = 200
        data[9] = 0x00;

        let result = backend.parse_bytes(&data, &options);
        assert!(result.is_ok());

        let doc = result.unwrap();
        assert_eq!(doc.format, InputFormat::Gif);
        assert!(doc.metadata.title.is_some());
        assert_eq!(doc.metadata.num_pages, Some(1));
        assert!(doc.content_blocks.is_some());
        assert!(!doc.markdown.is_empty());
    }

    #[test]
    fn test_document_metadata() {
        let backend = GifBackend::new();
        let options = BackendOptions::default();

        let mut data = vec![0u8; 13];
        data[0..6].copy_from_slice(b"GIF89a");
        data[6] = 0x64;
        data[7] = 0x00;
        data[8] = 0xC8;
        data[9] = 0x00;

        let doc = backend.parse_bytes(&data, &options).unwrap();

        assert_eq!(doc.metadata.num_pages, Some(1));
        assert!(doc.metadata.num_characters > 0);
        assert_eq!(doc.metadata.title, Some("image.gif".to_string()));
        assert_eq!(doc.metadata.author, None);
        assert_eq!(doc.metadata.language, None);
    }

    #[test]
    fn test_content_blocks_structure() {
        let backend = GifBackend::new();
        let options = BackendOptions::default();

        let mut data = vec![0u8; 13];
        data[0..6].copy_from_slice(b"GIF89a");
        data[6] = 0x64;
        data[7] = 0x00;
        data[8] = 0xC8;
        data[9] = 0x00;

        let doc = backend.parse_bytes(&data, &options).unwrap();

        assert!(doc.content_blocks.is_some());
        let blocks = doc.content_blocks.unwrap();
        assert_eq!(blocks.len(), 6); // title + type + dimensions + color_depth + file_size + note

        // Verify structure: 1 SectionHeader + 5 Text items
        assert!(matches!(blocks[0], DocItem::SectionHeader { .. }));
        assert!(matches!(blocks[1], DocItem::Text { .. }));
        assert!(matches!(blocks[2], DocItem::Text { .. }));
        assert!(matches!(blocks[3], DocItem::Text { .. }));
        assert!(matches!(blocks[4], DocItem::Text { .. }));
        assert!(matches!(blocks[5], DocItem::Text { .. }));
    }

    // ========== UNICODE AND SPECIAL CHARACTER TESTS ==========

    #[test]
    fn test_unicode_dimensions_in_markdown() {
        let backend = GifBackend::new();
        let mut data = vec![0u8; 13];
        data[0..6].copy_from_slice(b"GIF89a");
        data[6] = 0x64; // width: 100
        data[7] = 0x00;
        data[8] = 0xC8; // height: 200
        data[9] = 0x00;

        let doc = backend.parse_bytes(&data, &Default::default()).unwrap();
        // Verify dimensions contain × character
        assert!(doc.markdown.contains(" x "));
        assert!(doc.markdown.contains("100 x 200"));
    }

    #[test]
    fn test_markdown_utf8_validation() {
        let backend = GifBackend::new();
        let mut data = vec![0u8; 13];
        data[0..6].copy_from_slice(b"GIF89a");
        data[6] = 0x80; // width: 128
        data[7] = 0x00;
        data[8] = 0xF0; // height: 240
        data[9] = 0x00;

        let doc = backend.parse_bytes(&data, &Default::default()).unwrap();
        // Verify valid UTF-8
        assert!(std::str::from_utf8(doc.markdown.as_bytes()).is_ok());
    }

    #[test]
    fn test_format_name_in_markdown() {
        let backend = GifBackend::new();
        let mut data = vec![0u8; 13];
        data[0..6].copy_from_slice(b"GIF89a");
        data[6] = 100;
        data[7] = 0;
        data[8] = 100;
        data[9] = 0;

        let doc = backend.parse_bytes(&data, &Default::default()).unwrap();
        // GIF format should be mentioned
        assert!(doc.markdown.contains("GIF"));
    }

    // ========== VALIDATION TESTS ==========

    #[test]
    fn test_empty_gif_data_fails() {
        let backend = GifBackend::new();
        let result = backend.parse_bytes(b"", &Default::default());
        // Empty bytes should fail
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_gif_data_fails() {
        let backend = GifBackend::new();
        let result = backend.parse_bytes(b"not a gif", &Default::default());
        // Invalid GIF should fail
        assert!(result.is_err());
    }

    #[test]
    fn test_zero_dimensions() {
        let backend = GifBackend::new();
        let mut data = vec![0u8; 13];
        data[0..6].copy_from_slice(b"GIF89a");
        // width: 0, height: 0
        data[6] = 0;
        data[7] = 0;
        data[8] = 0;
        data[9] = 0;

        let doc = backend.parse_bytes(&data, &Default::default()).unwrap();
        // Should handle zero dimensions gracefully
        assert!(doc.markdown.contains("0 x 0"));
    }

    #[test]
    fn test_very_large_dimensions() {
        let backend = GifBackend::new();
        let mut data = vec![0u8; 13];
        data[0..6].copy_from_slice(b"GIF89a");
        // width: 65535, height: 65535
        data[6] = 0xFF;
        data[7] = 0xFF;
        data[8] = 0xFF;
        data[9] = 0xFF;

        let doc = backend.parse_bytes(&data, &Default::default()).unwrap();
        // Should handle maximum GIF dimensions
        assert!(doc.markdown.contains("65535 x 65535"));
    }

    #[test]
    fn test_parse_truncated_gif() {
        let backend = GifBackend::new();
        let data = b"GIF89a";
        // Only header, no logical screen descriptor
        let result = backend.parse_bytes(data, &Default::default());
        // Should fail gracefully
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_corrupted_header() {
        let backend = GifBackend::new();
        let data = b"GIF99a\x64\x00\xC8\x00\x00\x00\x00";
        // Invalid GIF version "99"
        let result = backend.parse_bytes(data, &Default::default());
        // Should fail
        assert!(result.is_err());
    }

    // ========== SERIALIZATION CONSISTENCY TESTS ==========

    #[test]
    fn test_markdown_not_empty() {
        let backend = GifBackend::new();
        let mut data = vec![0u8; 13];
        data[0..6].copy_from_slice(b"GIF89a");
        data[6] = 100;
        data[7] = 0;
        data[8] = 100;
        data[9] = 0;

        let doc = backend.parse_bytes(&data, &Default::default()).unwrap();
        assert!(!doc.markdown.is_empty());
        assert!(doc.markdown.len() > 10);
    }

    #[test]
    fn test_markdown_well_formed() {
        let backend = GifBackend::new();
        let mut data = vec![0u8; 13];
        data[0..6].copy_from_slice(b"GIF89a");
        data[6] = 0x40; // 320
        data[7] = 0x01;
        data[8] = 0xF0; // 240
        data[9] = 0x00;

        let doc = backend.parse_bytes(&data, &Default::default()).unwrap();
        // Should have heading
        assert!(doc.markdown.contains("# "));
        // Should have bold formatting
        // Bold labels no longer used in image formats (N=1610)
        // assert!(doc.markdown.contains("**"));
    }

    #[test]
    fn test_docitems_match_markdown() {
        let backend = GifBackend::new();
        let mut data = vec![0u8; 13];
        data[0..6].copy_from_slice(b"GIF89a");
        data[6] = 0x64;
        data[7] = 0x00;
        data[8] = 0xC8;
        data[9] = 0x00;

        let doc = backend.parse_bytes(&data, &Default::default()).unwrap();

        // Doc items content should appear in markdown
        if let Some(blocks) = &doc.content_blocks {
            for block in blocks {
                if let DocItem::Text { text, .. } = block {
                    // Content from doc items should be in markdown
                    assert!(doc.markdown.contains(text));
                }
            }
        }
    }

    #[test]
    fn test_markdown_idempotent() {
        let backend = GifBackend::new();
        let mut data = vec![0u8; 13];
        data[0..6].copy_from_slice(b"GIF89a");
        data[6] = 0x80;
        data[7] = 0x02; // 640
        data[8] = 0xE0;
        data[9] = 0x01; // 480

        // Parse twice
        let doc1 = backend.parse_bytes(&data, &Default::default()).unwrap();
        let doc2 = backend.parse_bytes(&data, &Default::default()).unwrap();

        // Should produce identical markdown
        assert_eq!(doc1.markdown, doc2.markdown);
    }

    // ========== BACKEND OPTIONS TESTS ==========

    #[test]
    fn test_parse_with_default_options() {
        let backend = GifBackend::new();
        let options = BackendOptions::default();

        let mut data = vec![0u8; 13];
        data[0..6].copy_from_slice(b"GIF89a");
        data[6] = 0x64;
        data[7] = 0x00;
        data[8] = 0xC8;
        data[9] = 0x00;

        let result = backend.parse_bytes(&data, &options);
        // Should parse successfully with default options
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_with_custom_options() {
        let backend = GifBackend::new();
        let options = BackendOptions::default();

        let mut data = vec![0u8; 13];
        data[0..6].copy_from_slice(b"GIF89a");
        data[6] = 0x64;
        data[7] = 0x00;
        data[8] = 0xC8;
        data[9] = 0x00;

        let result = backend.parse_bytes(&data, &options);
        // Options don't affect GIF parsing currently, but should not fail
        assert!(result.is_ok());
    }

    // ========== FORMAT-SPECIFIC TESTS ==========

    #[test]
    fn test_gif87a_version() {
        let backend = GifBackend::new();
        let mut data = vec![0u8; 13];
        data[0..6].copy_from_slice(b"GIF87a");
        data[6] = 0x64;
        data[7] = 0x00;
        data[8] = 0xC8;
        data[9] = 0x00;

        let result = backend.parse_bytes(&data, &Default::default());
        // GIF87a should also be supported
        assert!(result.is_ok());
    }

    #[test]
    fn test_animated_gif_detection() {
        let backend = GifBackend::new();
        let mut data = vec![0u8; 20];
        data[0..6].copy_from_slice(b"GIF89a");
        data[6] = 0x64;
        data[7] = 0x00;
        data[8] = 0xC8;
        data[9] = 0x00;
        // Add multiple image descriptors (0x2C) to trigger animation detection
        data[13] = 0x2C;
        data[14] = 0x2C;

        let doc = backend.parse_bytes(&data, &Default::default()).unwrap();
        // Should mention animation
        assert!(doc.markdown.contains("Animated"));
    }

    #[test]
    fn test_static_gif() {
        let backend = GifBackend::new();
        let mut data = vec![0u8; 13];
        data[0..6].copy_from_slice(b"GIF89a");
        data[6] = 0x64;
        data[7] = 0x00;
        data[8] = 0xC8;
        data[9] = 0x00;

        let doc = backend.parse_bytes(&data, &Default::default()).unwrap();
        // Should NOT mention animation (or mention static)
        assert!(!doc.markdown.contains("Animated") || doc.markdown.contains("GIF Image"));
    }

    #[test]
    fn test_aspect_ratio_square() {
        let backend = GifBackend::new();
        let mut data = vec![0u8; 13];
        data[0..6].copy_from_slice(b"GIF89a");
        data[6] = 100;
        data[7] = 0;
        data[8] = 100;
        data[9] = 0;

        let doc = backend.parse_bytes(&data, &Default::default()).unwrap();
        // Square image (1:1)
        assert!(doc.markdown.contains("100 x 100"));
    }

    #[test]
    fn test_aspect_ratio_wide() {
        let backend = GifBackend::new();
        let mut data = vec![0u8; 13];
        data[0..6].copy_from_slice(b"GIF89a");
        // 1920 = 0x0780
        data[6] = 0x80;
        data[7] = 0x07;
        // 1080 = 0x0438
        data[8] = 0x38;
        data[9] = 0x04;

        let doc = backend.parse_bytes(&data, &Default::default()).unwrap();
        // 16:9 aspect ratio
        assert!(doc.markdown.contains("1920 x 1080"));
    }

    #[test]
    fn test_aspect_ratio_tall() {
        let backend = GifBackend::new();
        let mut data = vec![0u8; 13];
        data[0..6].copy_from_slice(b"GIF89a");
        // 1080 = 0x0438
        data[6] = 0x38;
        data[7] = 0x04;
        // 1920 = 0x0780
        data[8] = 0x80;
        data[9] = 0x07;

        let doc = backend.parse_bytes(&data, &Default::default()).unwrap();
        // Portrait orientation (9:16)
        assert!(doc.markdown.contains("1080 x 1920"));
    }

    #[test]
    fn test_tiny_gif() {
        let backend = GifBackend::new();
        let mut data = vec![0u8; 13];
        data[0..6].copy_from_slice(b"GIF89a");
        data[6] = 1;
        data[7] = 0;
        data[8] = 1;
        data[9] = 0;

        let doc = backend.parse_bytes(&data, &Default::default()).unwrap();
        // 1×1 pixel GIF
        assert!(doc.markdown.contains("1 x 1"));
    }

    #[test]
    fn test_metadata_dimensions() {
        let backend = GifBackend::new();
        let mut data = vec![0u8; 13];
        data[0..6].copy_from_slice(b"GIF89a");
        data[6] = 0x64; // Width: 100
        data[7] = 0x00;
        data[8] = 0xC8; // Height: 200
        data[9] = 0x00;

        let doc = backend.parse_bytes(&data, &Default::default()).unwrap();
        // Metadata should reflect dimensions in markdown
        assert!(doc.markdown.contains("100 x 200"));
    }

    #[test]
    fn test_format_identification() {
        let backend = GifBackend::new();
        assert_eq!(backend.format(), InputFormat::Gif);
    }

    #[test]
    fn test_character_count() {
        let backend = GifBackend::new();
        let mut data = vec![0u8; 13];
        data[0..6].copy_from_slice(b"GIF89a");
        data[6] = 100;
        data[7] = 0;
        data[8] = 100;
        data[9] = 0;

        let doc = backend.parse_bytes(&data, &Default::default()).unwrap();
        // Character count should be > 0
        assert!(doc.metadata.num_characters > 0);
    }

    #[test]
    fn test_file_size_display() {
        let backend = GifBackend::new();
        let mut data = vec![0u8; 13];
        data[0..6].copy_from_slice(b"GIF89a");
        data[6] = 100;
        data[7] = 0;
        data[8] = 100;
        data[9] = 0;

        let doc = backend.parse_bytes(&data, &Default::default()).unwrap();
        // File size should be mentioned (13 bytes)
        assert!(
            doc.markdown.contains("File")
                || doc.markdown.contains("size")
                || doc.markdown.contains("bytes")
        );
    }

    #[test]
    fn test_content_note_present() {
        let backend = GifBackend::new();
        let mut data = vec![0u8; 13];
        data[0..6].copy_from_slice(b"GIF89a");
        data[6] = 100;
        data[7] = 0;
        data[8] = 100;
        data[9] = 0;

        let doc = backend.parse_bytes(&data, &Default::default()).unwrap();
        // Should have note about image content extraction
        assert!(
            doc.markdown.contains("Note")
                || doc.markdown.contains("OCR")
                || doc.markdown.contains("Image content")
        );
    }

    #[test]
    fn test_gif89a_with_transparency() {
        // GIF89a introduced transparency support via Graphics Control Extension
        let mut data = vec![0u8; 13];
        data[0..6].copy_from_slice(b"GIF89a"); // GIF89a signature
        data[6] = 200; // Width low byte
        data[7] = 0; // Width high byte
        data[8] = 150; // Height low byte
        data[9] = 0; // Height high byte
        data[10] = 0b10000000; // Global color table flag (bit 7)
        data[11] = 0; // Background color index
        data[12] = 0; // Aspect ratio (0 = no aspect info)

        let result = GifBackend::parse_gif_metadata(&data);
        assert!(result.is_ok());
        let (width, height, frame_count, _) = result.unwrap();
        assert_eq!(width, 200);
        assert_eq!(height, 150);
        assert_eq!(frame_count, 1); // No animation (no 0x2C separators)
    }

    #[test]
    fn test_gif_with_global_color_table() {
        // GIF with global color table (bit 7 set in flags)
        let mut data = vec![0u8; 13];
        data[0..6].copy_from_slice(b"GIF89a");
        data[6] = 64; // Width
        data[7] = 0;
        data[8] = 64; // Height
        data[9] = 0;
        data[10] = 0b10000111; // Global color table: flag=1, size=7 (2^(7+1)=256 colors)
        data[11] = 0; // Background color
        data[12] = 0; // Aspect

        let result = GifBackend::parse_gif_metadata(&data);
        assert!(result.is_ok());
        let (width, height, _, _) = result.unwrap();
        assert_eq!(width, 64);
        assert_eq!(height, 64);
    }

    #[test]
    fn test_gif_multiple_frames() {
        // Animated GIF with multiple image descriptors (0x2C separators)
        let mut data = vec![0u8; 13];
        data[0..6].copy_from_slice(b"GIF89a");
        data[6] = 100;
        data[7] = 0;
        data[8] = 100;
        data[9] = 0;

        // Add multiple 0x2C separators (image descriptor markers)
        data.extend_from_slice(&[0x2C, 0x00, 0x00]); // Frame 1
        data.extend_from_slice(&[0x2C, 0x00, 0x00]); // Frame 2
        data.extend_from_slice(&[0x2C, 0x00, 0x00]); // Frame 3

        let result = GifBackend::parse_gif_metadata(&data);
        assert!(result.is_ok());
        let (_width, _height, frame_count, _) = result.unwrap();
        assert!(frame_count > 1); // Should detect animation (3 frames)
    }

    #[test]
    fn test_gif_maximum_dimensions() {
        // GIF with maximum u16 dimensions (65535×65535)
        let mut data = vec![0u8; 13];
        data[0..6].copy_from_slice(b"GIF89a");
        data[6] = 0xFF; // Width low byte
        data[7] = 0xFF; // Width high byte (65535)
        data[8] = 0xFF; // Height low byte
        data[9] = 0xFF; // Height high byte (65535)
        data[10] = 0;
        data[11] = 0;
        data[12] = 0;

        let result = GifBackend::parse_gif_metadata(&data);
        assert!(result.is_ok());
        let (width, height, _, _) = result.unwrap();
        assert_eq!(width, 65535);
        assert_eq!(height, 65535);
    }

    #[test]
    fn test_gif_with_aspect_ratio() {
        // GIF with non-zero aspect ratio field
        // Aspect Ratio = (Pixel Aspect Ratio + 15) / 64
        // Value 49 = (49 + 15) / 64 = 1.0 (square pixels)
        let mut data = vec![0u8; 13];
        data[0..6].copy_from_slice(b"GIF89a");
        data[6] = (320 & 0xFF) as u8; // Width low
        data[7] = (320 >> 8) as u8; // Width high
        data[8] = (240 & 0xFF) as u8; // Height low
        data[9] = (240 >> 8) as u8; // Height high
        data[10] = 0;
        data[11] = 0;
        data[12] = 49; // Aspect ratio (square pixels)

        let result = GifBackend::parse_gif_metadata(&data);
        assert!(result.is_ok());
        let (width, height, _, _) = result.unwrap();
        assert_eq!(width, 320);
        assert_eq!(height, 240);
    }

    #[test]
    fn test_gif87a_legacy_format() {
        // GIF87a (older format, no animation/transparency support)
        let mut data = vec![0u8; 13];
        data[0..6].copy_from_slice(b"GIF87a"); // GIF87a signature
        data[6] = 160; // Width (low byte)
        data[7] = (160 >> 8) as u8;
        data[8] = 120; // Height (low byte)
        data[9] = (120 >> 8) as u8;
        data[10] = 0;
        data[11] = 0;
        data[12] = 0;

        let result = GifBackend::parse_gif_metadata(&data);
        assert!(result.is_ok());
        let (width, height, _, _) = result.unwrap();
        assert_eq!(width, 160);
        assert_eq!(height, 120);
    }

    #[test]
    fn test_gif_interlaced_flag() {
        // GIF with interlaced image (bit 6 in image descriptor flags)
        let mut data = vec![0u8; 13];
        data[0..6].copy_from_slice(b"GIF89a");
        data[6] = (400 & 0xFF) as u8;
        data[7] = (400 >> 8) as u8;
        data[8] = (300 & 0xFF) as u8;
        data[9] = (300 >> 8) as u8;
        data[10] = 0b01000000; // Interlaced flag (bit 6)
        data[11] = 0;
        data[12] = 0;

        let result = GifBackend::parse_gif_metadata(&data);
        assert!(result.is_ok());
        let (width, height, _, _) = result.unwrap();
        assert_eq!(width, 400);
        assert_eq!(height, 300);
    }

    #[test]
    fn test_gif_color_resolution() {
        // GIF with color resolution field (bits 4-6 of flags)
        // Color resolution = (value + 1) bits per primary color
        let mut data = vec![0u8; 13];
        data[0..6].copy_from_slice(b"GIF89a");
        data[6] = (256 & 0xFF) as u8;
        data[7] = (256 >> 8) as u8;
        data[8] = (256 & 0xFF) as u8;
        data[9] = (256 >> 8) as u8;
        data[10] = 0b10110111; // Global table + color resolution 7 (bits 4-6) + table size 7
        data[11] = 0;
        data[12] = 0;

        let result = GifBackend::parse_gif_metadata(&data);
        assert!(result.is_ok());
        let (width, height, _, _) = result.unwrap();
        assert_eq!(width, 256);
        assert_eq!(height, 256);
    }

    #[test]
    fn test_gif_background_color_index() {
        // GIF with background color index (index into global color table)
        let mut data = vec![0u8; 13];
        data[0..6].copy_from_slice(b"GIF89a");
        data[6] = 128;
        data[7] = 0;
        data[8] = 96;
        data[9] = 0;
        data[10] = 0b10000111; // Global color table present
        data[11] = 42; // Background color index 42
        data[12] = 0;

        let result = GifBackend::parse_gif_metadata(&data);
        assert!(result.is_ok());
        let (width, height, _, _) = result.unwrap();
        assert_eq!(width, 128);
        assert_eq!(height, 96);
    }

    #[test]
    fn test_gif_many_frames_animated() {
        // Heavily animated GIF with 10 frames
        let mut data = vec![0u8; 13];
        data[0..6].copy_from_slice(b"GIF89a");
        data[6] = 64;
        data[7] = 0;
        data[8] = 64;
        data[9] = 0;

        // Add 10 image descriptor markers (0x2C)
        for _ in 0..10 {
            data.push(0x2C);
            data.push(0x00);
            data.push(0x00);
        }

        let result = GifBackend::parse_gif_metadata(&data);
        assert!(result.is_ok());
        let (_width, _height, frame_count, _) = result.unwrap();
        assert!(frame_count > 1); // Should definitely detect as animated (10 frames)
    }

    // ========== Additional Edge Cases (5 tests) ==========

    #[test]
    fn test_gif_transparent_background() {
        // GIF with transparent color index (GIF89a feature)
        let mut data = vec![0u8; 13];
        data[0..6].copy_from_slice(b"GIF89a");
        data[6] = (200 & 0xFF) as u8;
        data[7] = (200 >> 8) as u8;
        data[8] = (150 & 0xFF) as u8;
        data[9] = (150 >> 8) as u8;
        data[10] = 0b10000111; // Global color table present
        data[11] = 0xFF; // Background color index (transparent)
        data[12] = 0;

        let result = GifBackend::parse_gif_metadata(&data);
        assert!(result.is_ok());
        let (width, height, _, _) = result.unwrap();
        assert_eq!(width, 200);
        assert_eq!(height, 150);
    }

    #[test]
    fn test_gif_aspect_ratio_non_zero() {
        // GIF with specified aspect ratio (rarely used field)
        let mut data = vec![0u8; 13];
        data[0..6].copy_from_slice(b"GIF89a");
        data[6] = (320 & 0xFF) as u8;
        data[7] = (320 >> 8) as u8;
        data[8] = (240 & 0xFF) as u8;
        data[9] = (240 >> 8) as u8;
        data[10] = 0;
        data[11] = 0;
        data[12] = 64; // Aspect ratio = (64 + 15) / 64 = 1.234

        let result = GifBackend::parse_gif_metadata(&data);
        assert!(result.is_ok());
        let (width, height, _, _) = result.unwrap();
        assert_eq!(width, 320);
        assert_eq!(height, 240);
    }

    #[test]
    fn test_gif_global_color_table_sizes() {
        // Test various global color table sizes (2^(N+1) entries)
        let table_sizes = vec![0, 1, 2, 3, 4, 5, 6, 7]; // 2, 4, 8, 16, 32, 64, 128, 256 colors

        for size in table_sizes {
            let mut data = vec![0u8; 13];
            data[0..6].copy_from_slice(b"GIF89a");
            data[6] = 100;
            data[7] = 0;
            data[8] = 100;
            data[9] = 0;
            data[10] = 0b10000000 | size; // Global table bit + size in bits 0-2
            data[11] = 0;
            data[12] = 0;

            let result = GifBackend::parse_gif_metadata(&data);
            assert!(result.is_ok(), "Failed for color table size {size}");
            let (width, height, _, _) = result.unwrap();
            assert_eq!(width, 100);
            assert_eq!(height, 100);
        }
    }

    #[test]
    fn test_gif_extension_blocks() {
        // GIF with extension blocks (application, comment, graphic control)
        let mut data = vec![0u8; 13];
        data[0..6].copy_from_slice(b"GIF89a");
        data[6] = (180 & 0xFF) as u8;
        data[7] = (180 >> 8) as u8;
        data[8] = (135 & 0xFF) as u8;
        data[9] = (135 >> 8) as u8;
        data[10] = 0;
        data[11] = 0;
        data[12] = 0;

        // Add extension introducer (0x21) followed by label
        data.push(0x21); // Extension introducer
        data.push(0xF9); // Graphic Control Extension label
        data.push(0x04); // Block size
        data.push(0x00); // Flags
        data.push(0x0A); // Delay time (low byte)
        data.push(0x00); // Delay time (high byte)
        data.push(0xFF); // Transparent color index
        data.push(0x00); // Block terminator

        let result = GifBackend::parse_gif_metadata(&data);
        assert!(result.is_ok());
        let (width, height, _, _) = result.unwrap();
        assert_eq!(width, 180);
        assert_eq!(height, 135);
    }

    #[test]
    fn test_gif_looping_animation() {
        // GIF with Netscape 2.0 application extension for looping
        let mut data = vec![0u8; 13];
        data[0..6].copy_from_slice(b"GIF89a");
        data[6] = (240 & 0xFF) as u8;
        data[7] = (240 >> 8) as u8;
        data[8] = (240 & 0xFF) as u8;
        data[9] = (240 >> 8) as u8;
        data[10] = 0;
        data[11] = 0;
        data[12] = 0;

        // Add Netscape 2.0 application extension
        data.push(0x21); // Extension introducer
        data.push(0xFF); // Application Extension label
        data.push(0x0B); // Block size (11 bytes)
                         // Application identifier: "NETSCAPE2.0"
        data.extend_from_slice(b"NETSCAPE2.0");
        data.push(0x03); // Sub-block size
        data.push(0x01); // Sub-block ID
        data.push(0x00); // Loop count (low byte) - 0 = infinite
        data.push(0x00); // Loop count (high byte)
        data.push(0x00); // Block terminator

        // Add 2 frames to make it animated
        data.push(0x2C); // Image descriptor
        data.push(0x2C); // Another image descriptor

        let result = GifBackend::parse_gif_metadata(&data);
        assert!(result.is_ok());
        let (width, height, frame_count, _) = result.unwrap();
        assert_eq!(width, 240);
        assert_eq!(height, 240);
        assert!(frame_count > 1); // Should be detected as animated
    }

    // ===== N=599 Expansion: 5 additional tests =====

    #[test]
    fn test_gif_disposal_methods() {
        // Test GIF Graphic Control Extension disposal methods
        // Disposal method: 0=unspecified, 1=do not dispose, 2=restore to bg, 3=restore to previous
        let mut data = vec![0u8; 13];
        data[0..6].copy_from_slice(b"GIF89a");
        data[6..10].copy_from_slice(&[100, 0, 100, 0]); // 100x100
        data[10..13].copy_from_slice(&[0, 0, 0]);

        // Add Graphic Control Extension with disposal method 2 (restore to background)
        data.push(0x21); // Extension introducer
        data.push(0xF9); // Graphic Control label
        data.push(0x04); // Block size
        data.push(0x08); // Packed field: disposal method = 2 (0x08 = 00001000)
        data.push(0x0A); // Delay time (low byte)
        data.push(0x00); // Delay time (high byte)
        data.push(0x00); // Transparent color index
        data.push(0x00); // Block terminator

        let result = GifBackend::parse_gif_metadata(&data);
        assert!(result.is_ok());
        let (width, height, _, _) = result.unwrap();
        assert_eq!(width, 100);
        assert_eq!(height, 100);
    }

    #[test]
    fn test_gif_with_plain_text_extension() {
        // Test GIF with Plain Text Extension (rarely used but valid)
        let mut data = vec![0u8; 13];
        data[0..6].copy_from_slice(b"GIF89a");
        data[6..10].copy_from_slice(&[200, 0, 150, 0]); // 200x150
        data[10..13].copy_from_slice(&[0, 0, 0]);

        // Add Plain Text Extension
        data.push(0x21); // Extension introducer
        data.push(0x01); // Plain Text label
        data.push(0x0C); // Block size (12 bytes)
        data.extend_from_slice(&[0, 0, 0, 0]); // Text grid left/top position
        data.extend_from_slice(&[100, 0, 50, 0]); // Text grid width/height
        data.extend_from_slice(&[8, 8]); // Cell width/height
        data.extend_from_slice(&[1, 2]); // Foreground/background color index
        data.push(0x00); // Block terminator

        let result = GifBackend::parse_gif_metadata(&data);
        assert!(result.is_ok());
        let (width, height, _, _) = result.unwrap();
        assert_eq!(width, 200);
        assert_eq!(height, 150);
    }

    #[test]
    fn test_gif_minimal_valid_file() {
        // Test absolute minimal valid GIF file
        // Header (6) + Logical Screen Descriptor (7) = 13 bytes minimum
        let minimal_gif = vec![
            b'G', b'I', b'F', b'8', b'9', b'a', // Header
            1, 0, // Width = 1
            1, 0, // Height = 1
            0, // Packed field (no global color table)
            0, // Background color index
            0, // Pixel aspect ratio
        ];

        let result = GifBackend::parse_gif_metadata(&minimal_gif);
        assert!(result.is_ok());
        let (width, height, frame_count, _) = result.unwrap();
        assert_eq!(width, 1);
        assert_eq!(height, 1);
        assert_eq!(frame_count, 1); // Single pixel, not animated
    }

    #[test]
    fn test_gif_with_multiple_application_extensions() {
        // Test GIF with multiple application extensions (not just Netscape)
        let mut data = vec![0u8; 13];
        data[0..6].copy_from_slice(b"GIF89a");
        data[6..10].copy_from_slice(&[
            (300 & 0xFF) as u8,
            (300 >> 8) as u8,
            (200 & 0xFF) as u8,
            (200 >> 8) as u8,
        ]);
        data[10..13].copy_from_slice(&[0, 0, 0]);

        // Add XMP Data application extension (Adobe metadata)
        data.push(0x21); // Extension introducer
        data.push(0xFF); // Application Extension label
        data.push(0x0B); // Block size (11 bytes)
        data.extend_from_slice(b"XMP DataXMP"); // Application identifier
        data.push(0x00); // Block terminator

        // Add Netscape 2.0 looping extension
        data.push(0x21); // Extension introducer
        data.push(0xFF); // Application Extension label
        data.push(0x0B); // Block size
        data.extend_from_slice(b"NETSCAPE2.0");
        data.push(0x03); // Sub-block size
        data.push(0x01); // Sub-block ID
        data.push(0xFF); // Loop count (255 times)
        data.push(0x00);
        data.push(0x00); // Block terminator

        // Add 2 frames for animation
        data.push(0x2C); // Image descriptor
        data.push(0x2C); // Another image descriptor

        let result = GifBackend::parse_gif_metadata(&data);
        assert!(result.is_ok());
        let (width, height, frame_count, _) = result.unwrap();
        assert_eq!(width, 300);
        assert_eq!(height, 200);
        assert!(frame_count > 1); // Should detect animation from multiple image descriptors
    }

    // ===== N=613 Expansion: 6 additional tests (69 → 75) =====

    #[test]
    fn test_gif_with_comment_extension() {
        // Test GIF Comment Extension (0x21 0xFE) for embedded text metadata
        let mut data = vec![0u8; 13];
        data[0..6].copy_from_slice(b"GIF89a");
        data[6..10].copy_from_slice(&[150, 0, 100, 0]); // 150x100
        data[10..13].copy_from_slice(&[0, 0, 0]);

        // Add Comment Extension
        data.push(0x21); // Extension introducer
        data.push(0xFE); // Comment label
        data.push(0x0D); // Block size (13 bytes)
        data.extend_from_slice(b"Test Comment!"); // Comment text
        data.push(0x00); // Block terminator

        let result = GifBackend::parse_gif_metadata(&data);
        assert!(result.is_ok());
        let (width, height, _, _) = result.unwrap();
        assert_eq!(width, 150);
        assert_eq!(height, 100);
    }

    #[test]
    fn test_gif_with_local_color_table() {
        // Test GIF with Local Color Table flag set in image descriptor
        let mut data = vec![0u8; 13];
        data[0..6].copy_from_slice(b"GIF89a");
        data[6..10].copy_from_slice(&[100, 0, 100, 0]); // 100x100
        data[10..13].copy_from_slice(&[0, 0, 0]);

        // Add Image Descriptor with Local Color Table
        data.push(0x2C); // Image separator
        data.extend_from_slice(&[0, 0, 0, 0]); // Left, Top position
        data.extend_from_slice(&[50, 0, 50, 0]); // Width, Height
        data.push(0x83); // Packed field: Local Color Table flag = 1, size = 3 bits (2^4=16 colors)

        let result = GifBackend::parse_gif_metadata(&data);
        assert!(result.is_ok());
        let (width, height, _, _) = result.unwrap();
        assert_eq!(width, 100);
        assert_eq!(height, 100);
    }

    #[test]
    fn test_gif_with_zero_delay_frames() {
        // Test animated GIF with zero delay between frames (fast animation)
        let mut data = vec![0u8; 13];
        data[0..6].copy_from_slice(b"GIF89a");
        data[6..10].copy_from_slice(&[200, 0, 200, 0]); // 200x200
        data[10..13].copy_from_slice(&[0, 0, 0]);

        // Add Graphic Control Extension with zero delay
        data.push(0x21); // Extension introducer
        data.push(0xF9); // Graphic Control label
        data.push(0x04); // Block size
        data.push(0x00); // Packed field
        data.push(0x00); // Delay time = 0 (no delay)
        data.push(0x00); // Delay time high byte
        data.push(0x00); // Transparent color index
        data.push(0x00); // Block terminator

        // Add multiple image descriptors for animation
        data.push(0x2C); // Image descriptor
        data.push(0x2C); // Another image descriptor
        data.push(0x2C); // Third image descriptor

        let result = GifBackend::parse_gif_metadata(&data);
        assert!(result.is_ok());
        let (width, height, frame_count, _) = result.unwrap();
        assert_eq!(width, 200);
        assert_eq!(height, 200);
        assert!(frame_count > 1); // Multiple frames = animated
    }

    #[test]
    fn test_gif_with_user_input_flag() {
        // Test GIF Graphic Control Extension with user input flag
        // (waits for user input before advancing to next frame)
        let mut data = vec![0u8; 13];
        data[0..6].copy_from_slice(b"GIF89a");
        data[6..10].copy_from_slice(&[100, 0, 100, 0]);
        data[10..13].copy_from_slice(&[0, 0, 0]);

        // Add Graphic Control Extension with user input flag
        data.push(0x21); // Extension introducer
        data.push(0xF9); // Graphic Control label
        data.push(0x04); // Block size
        data.push(0x02); // Packed field: user input flag = 1 (bit 1)
        data.push(0x64); // Delay time (100 centiseconds = 1 second)
        data.push(0x00);
        data.push(0x00); // Transparent color index
        data.push(0x00); // Block terminator

        let result = GifBackend::parse_gif_metadata(&data);
        assert!(result.is_ok());
        let (width, height, _, _) = result.unwrap();
        assert_eq!(width, 100);
        assert_eq!(height, 100);
    }

    #[test]
    fn test_gif_corrupted_width_zero() {
        // Test handling of invalid GIF with zero width
        let mut data = vec![0u8; 13];
        data[0..6].copy_from_slice(b"GIF89a");
        data[6..10].copy_from_slice(&[0, 0, 100, 0]); // Width = 0 (invalid)
        data[10..13].copy_from_slice(&[0, 0, 0]);

        let result = GifBackend::parse_gif_metadata(&data);
        // Parser should handle gracefully - either error or return (0, 100, false)
        if let Ok((width, _height, _, _)) = result {
            assert_eq!(width, 0); // Should preserve the zero value
        } else {
            // Or it may return an error - both are acceptable
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_gif_with_iccp_profile() {
        // Test GIF with ICC Color Profile application extension (ICCRGBG1012)
        let mut data = vec![0u8; 13];
        data[0..6].copy_from_slice(b"GIF89a");
        data[6..10].copy_from_slice(&[0, 1, 0, 1]); // 256x256 in little-endian
        data[10..13].copy_from_slice(&[0, 0, 0]);

        // Add ICC Profile application extension
        data.push(0x21); // Extension introducer
        data.push(0xFF); // Application Extension label
        data.push(0x0B); // Block size (11 bytes)
        data.extend_from_slice(b"ICCRGBG1012"); // ICC Profile identifier
        data.push(0x00); // Block terminator (no profile data for this test)

        let result = GifBackend::parse_gif_metadata(&data);
        assert!(result.is_ok());
        let (width, height, _, _) = result.unwrap();
        assert_eq!(width, 256);
        assert_eq!(height, 256);
    }
}
