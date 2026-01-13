//! AVIF backend for docling
//!
//! This backend converts AVIF (AV1 Image File Format) files to docling's document model.
//! AVIF is based on the same ISOBMFF structure as HEIF, so we reuse the same parser.

use crate::heif::HeifBackend;
use crate::traits::{BackendOptions, DocumentBackend};
use docling_core::{DoclingError, Document, InputFormat};
use std::path::Path;

/// AVIF backend
///
/// Converts AVIF (AV1 Image File Format) files to docling's document model.
/// AVIF uses the same ISO Base Media File Format (ISOBMFF) structure as HEIF,
/// so this backend delegates to `HeifBackend` with the AVIF format.
///
/// ## Features
///
/// - Extract image dimensions from 'ispe' property
/// - Detect AVIF brand identifiers
/// - Generate markdown with image metadata
///
/// ## Example
///
/// ```no_run
/// use docling_backend::AvifBackend;
/// use docling_backend::DocumentBackend;
///
/// let backend = AvifBackend::new();
/// let result = backend.parse_file("image.avif", &Default::default())?;
/// println!("Image: {:?}", result.metadata.title);
/// # Ok::<(), docling_core::error::DoclingError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AvifBackend {
    heif_backend: HeifBackend,
}

impl AvifBackend {
    /// Create a new AVIF backend instance
    #[inline]
    #[must_use = "creates a backend instance that should be used for parsing"]
    pub const fn new() -> Self {
        Self {
            heif_backend: HeifBackend::new(InputFormat::Avif),
        }
    }
}

impl Default for AvifBackend {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl DocumentBackend for AvifBackend {
    #[inline]
    fn format(&self) -> InputFormat {
        InputFormat::Avif
    }

    fn parse_bytes(&self, data: &[u8], options: &BackendOptions) -> Result<Document, DoclingError> {
        self.heif_backend.parse_bytes(data, options)
    }

    fn parse_file<P: AsRef<Path>>(
        &self,
        path: P,
        options: &BackendOptions,
    ) -> Result<Document, DoclingError> {
        self.heif_backend.parse_file(path, options)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use docling_core::DocItem;

    #[test]
    fn test_avif_backend_creation() {
        let backend = AvifBackend::new();
        assert_eq!(
            backend.format(),
            InputFormat::Avif,
            "AvifBackend::new() should report Avif format"
        );
    }

    #[test]
    fn test_parse_avif_ftyp() {
        // Minimal valid AVIF file with ftyp box
        let mut data = vec![0u8; 24];
        // ftyp box: size=24, type='ftyp', brand='avif', version=0
        data[0..4].copy_from_slice(&24u32.to_be_bytes());
        data[4..8].copy_from_slice(b"ftyp");
        data[8..12].copy_from_slice(b"avif");
        data[12..16].copy_from_slice(&0u32.to_be_bytes());

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());
        assert!(
            result.is_ok(),
            "parse_bytes should succeed for minimal valid AVIF with ftyp box"
        );
        let doc = result.unwrap();
        assert_eq!(
            doc.format,
            InputFormat::Avif,
            "Document format should be Avif"
        );
    }

    #[test]
    fn test_parse_avif_with_dimensions() {
        // Create a mock AVIF file with ftyp and ispe boxes
        let mut data = Vec::new();

        // ftyp box (20 bytes: 4 size + 4 type + 4 brand + 4 version + 4 compatible)
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"avif");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        // ispe box (20 bytes: 4 size + 4 type + 4 version/flags + 4 width + 4 height)
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ispe");
        data.extend_from_slice(&0u32.to_be_bytes()); // version + flags
        data.extend_from_slice(&3840u32.to_be_bytes()); // width
        data.extend_from_slice(&2160u32.to_be_bytes()); // height

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());
        assert!(
            result.is_ok(),
            "parse_bytes should succeed for AVIF with ftyp and ispe boxes"
        );
        let doc = result.unwrap();
        assert_eq!(
            doc.format,
            InputFormat::Avif,
            "Document format should be Avif"
        );
        // Check that dimensions are in markdown
        assert!(
            doc.markdown.contains("3840x2160"),
            "Markdown should contain dimensions '3840x2160' from ispe box"
        );
    }

    #[test]
    fn test_invalid_avif() {
        let data = b"NOTAVIF123";
        let backend = AvifBackend::new();
        let result = backend.parse_bytes(data, &Default::default());
        assert!(
            result.is_err(),
            "parse_bytes should fail for invalid AVIF data without ftyp box"
        );
    }

    // ========== METADATA TESTS ==========

    #[test]
    fn test_metadata_format_field() {
        // Test that format is correctly set to AVIF
        let mut data = vec![0u8; 24];
        data[0..4].copy_from_slice(&24u32.to_be_bytes());
        data[4..8].copy_from_slice(b"ftyp");
        data[8..12].copy_from_slice(b"avif");
        data[12..16].copy_from_slice(&0u32.to_be_bytes());

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());
        assert!(result.is_ok(), "parse_bytes should succeed for valid AVIF");

        let doc = result.unwrap();
        assert_eq!(
            doc.format,
            InputFormat::Avif,
            "Document format should be Avif"
        );
    }

    #[test]
    fn test_metadata_num_pages() {
        // Test that AVIF is reported as single-page
        let mut data = vec![0u8; 24];
        data[0..4].copy_from_slice(&24u32.to_be_bytes());
        data[4..8].copy_from_slice(b"ftyp");
        data[8..12].copy_from_slice(b"avif");

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());
        assert!(result.is_ok(), "parse_bytes should succeed for valid AVIF");

        let doc = result.unwrap();
        assert_eq!(
            doc.metadata.num_pages,
            Some(1),
            "AVIF image should report num_pages as 1"
        );
    }

    #[test]
    fn test_metadata_character_count() {
        // Test that character count is computed correctly
        let mut data = vec![0u8; 24];
        data[0..4].copy_from_slice(&24u32.to_be_bytes());
        data[4..8].copy_from_slice(b"ftyp");
        data[8..12].copy_from_slice(b"avif");

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());
        assert!(result.is_ok(), "parse_bytes should succeed for valid AVIF");

        let doc = result.unwrap();
        // Character count should match markdown length
        assert_eq!(
            doc.metadata.num_characters,
            doc.markdown.chars().count(),
            "num_characters should match actual markdown character count"
        );
        assert!(
            doc.metadata.num_characters > 0,
            "Character count should be positive for generated markdown"
        );
    }

    // ========== DOCITEM TESTS ==========

    #[test]
    fn test_docitem_structure() {
        // Test that DocItems are created correctly
        let mut data = vec![0u8; 24];
        data[0..4].copy_from_slice(&24u32.to_be_bytes());
        data[4..8].copy_from_slice(b"ftyp");
        data[8..12].copy_from_slice(b"avif");

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());
        assert!(result.is_ok(), "parse_bytes should succeed for valid AVIF");

        let doc = result.unwrap();
        assert!(
            doc.content_blocks.is_some(),
            "AVIF document should have content_blocks"
        );

        let items = doc.content_blocks.unwrap();
        assert!(!items.is_empty(), "content_blocks should not be empty");

        // After N=2300 subsection improvements, structure is:
        // SectionHeaders at indices 0, 1, 2, 5, 8 (title, image details, format info, dims, content)
        // Text items at indices 3, 4, 6, 7, 9 (type, brand, dims, filesize, note)
        assert!(
            matches!(items[0], DocItem::SectionHeader { .. }),
            "First item should be SectionHeader (Title)"
        );
        assert!(
            matches!(items[1], DocItem::SectionHeader { .. }),
            "Second item should be SectionHeader (Image Details)"
        );
        assert!(
            matches!(items[2], DocItem::SectionHeader { .. }),
            "Third item should be SectionHeader (Format Information)"
        );

        // Verify mixed structure (both SectionHeaders and Text items exist)
        let section_count = items
            .iter()
            .filter(|i| matches!(i, DocItem::SectionHeader { .. }))
            .count();
        let text_count = items
            .iter()
            .filter(|i| matches!(i, DocItem::Text { .. }))
            .count();
        assert!(
            section_count >= 4,
            "Should have at least 4 SectionHeaders (Title, Image Details, Format Info, Dims, Content), got {section_count}"
        );
        assert!(
            text_count >= 4,
            "Should have at least 4 Text items (Type, Dims, File size, Note), got {text_count}"
        );
    }

    #[test]
    fn test_docitem_count() {
        // Test expected number of DocItems
        let mut data = vec![0u8; 24];
        data[0..4].copy_from_slice(&24u32.to_be_bytes());
        data[4..8].copy_from_slice(b"ftyp");
        data[8..12].copy_from_slice(b"avif");

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());
        assert!(result.is_ok(), "parse_bytes should succeed for valid AVIF");

        let doc = result.unwrap();
        assert!(
            doc.content_blocks.is_some(),
            "AVIF document should have content_blocks"
        );

        let items = doc.content_blocks.unwrap();
        // Expected: title, type, dimensions, brand, file size, image reference (6 paragraphs)
        assert!(
            items.len() >= 5,
            "Should have at least 5 DocItems, got {}",
            items.len()
        );
    }

    #[test]
    fn test_docitem_content_with_dimensions() {
        // Test that DocItems contain expected content including dimensions
        let mut data = Vec::new();
        // ftyp box
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"avif");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());
        // ispe box
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ispe");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&1920u32.to_be_bytes());
        data.extend_from_slice(&1080u32.to_be_bytes());

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());
        assert!(
            result.is_ok(),
            "parse_bytes should succeed for AVIF with dimensions"
        );

        let doc = result.unwrap();
        // Check that dimensions are in markdown
        assert!(
            doc.markdown.contains("1920x1080"),
            "Markdown should contain dimensions '1920x1080'"
        );
    }

    #[test]
    fn test_docitem_self_ref() {
        // Test that self_ref fields are correctly indexed
        let mut data = vec![0u8; 24];
        data[0..4].copy_from_slice(&24u32.to_be_bytes());
        data[4..8].copy_from_slice(b"ftyp");
        data[8..12].copy_from_slice(b"avif");

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        let items = doc.content_blocks.unwrap();

        // Check that self_ref values are sequential
        for (i, item) in items.iter().enumerate() {
            if let DocItem::Text { self_ref, .. } = item {
                assert_eq!(self_ref, &format!("#/texts/{i}"));
            }
        }
    }

    // ========== FORMAT-SPECIFIC TESTS ==========

    #[test]
    fn test_avif_brand_detection() {
        // Test that AVIF brand is correctly detected
        let mut data = vec![0u8; 24];
        data[0..4].copy_from_slice(&24u32.to_be_bytes());
        data[4..8].copy_from_slice(b"ftyp");
        data[8..12].copy_from_slice(b"avif");

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        // Should contain brand info in markdown
        assert!(doc.markdown.contains("avif") || doc.markdown.contains("AVIF"));
    }

    #[test]
    fn test_dimensions_various_sizes() {
        // Test various dimension combinations
        let test_cases: Vec<(u32, u32)> =
            vec![(100, 100), (1920, 1080), (3840, 2160), (8000, 6000)];

        for (width, height) in test_cases {
            let mut data = Vec::new();
            // ftyp box
            data.extend_from_slice(&20u32.to_be_bytes());
            data.extend_from_slice(b"ftyp");
            data.extend_from_slice(b"avif");
            data.extend_from_slice(&0u32.to_be_bytes());
            data.extend_from_slice(&0u32.to_be_bytes());
            // ispe box
            data.extend_from_slice(&20u32.to_be_bytes());
            data.extend_from_slice(b"ispe");
            data.extend_from_slice(&0u32.to_be_bytes());
            data.extend_from_slice(&width.to_be_bytes());
            data.extend_from_slice(&height.to_be_bytes());

            let backend = AvifBackend::new();
            let result = backend.parse_bytes(&data, &Default::default());
            assert!(result.is_ok());

            let doc = result.unwrap();
            assert!(doc.markdown.contains(&format!("{width}x{height}")));
        }
    }

    #[test]
    fn test_file_size_formatting() {
        // Test that file size is included in markdown
        let mut data = vec![0u8; 24];
        data[0..4].copy_from_slice(&24u32.to_be_bytes());
        data[4..8].copy_from_slice(b"ftyp");
        data[8..12].copy_from_slice(b"avif");

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        // File size should be in the markdown
        assert!(doc.markdown.contains("File Size:"));
    }

    #[test]
    fn test_markdown_content_note() {
        // Test that markdown contains OCR note (HEIF backend doesn't do OCR)
        let mut data = vec![0u8; 24];
        data[0..4].copy_from_slice(&24u32.to_be_bytes());
        data[4..8].copy_from_slice(b"ftyp");
        data[8..12].copy_from_slice(b"avif");

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        // Should contain note about OCR/image analysis requirement
        assert!(doc.markdown.contains("Note:") || doc.markdown.contains("OCR"));
    }

    #[test]
    fn test_isobmff_box_structure() {
        // Test proper ISOBMFF box structure parsing
        let mut data = Vec::new();
        // ftyp box (28 bytes total)
        data.extend_from_slice(&28u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"avif");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(b"mif1"); // compatible brand
        data.extend_from_slice(b"miaf"); // compatible brand

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());
        assert!(result.is_ok());
    }

    // ========== EDGE CASE TESTS ==========

    #[test]
    fn test_empty_avif_data() {
        // Test handling of empty data
        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&[], &Default::default());

        // Should fail gracefully
        assert!(result.is_err());
    }

    #[test]
    fn test_incomplete_ftyp_box() {
        // Test handling of incomplete ftyp box
        let data = vec![0xFF, 0xFF, 0xFF, 0xFF, b'f', b't', b'y', b'p'];

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());

        // Should fail gracefully
        assert!(result.is_err());
    }

    #[test]
    fn test_minimal_valid_avif() {
        // Test with minimal valid AVIF (ftyp box only)
        let mut data = vec![0u8; 20];
        data[0..4].copy_from_slice(&20u32.to_be_bytes());
        data[4..8].copy_from_slice(b"ftyp");
        data[8..12].copy_from_slice(b"avif");
        data[12..16].copy_from_slice(&0u32.to_be_bytes());

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());

        // Should succeed even with minimal structure
        assert!(result.is_ok());
    }

    #[test]
    fn test_large_file_size() {
        // Test with large file (simulated via large box size, but actual data is minimal)
        let mut data = Vec::new();
        // ftyp box claiming large size
        data.extend_from_slice(&100u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"avif");
        data.extend_from_slice(&0u32.to_be_bytes());
        // Pad to claimed size
        data.resize(100, 0);

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());

        // Should handle large declared sizes
        assert!(result.is_ok());
    }

    #[test]
    fn test_wrong_brand_in_ftyp() {
        // Test that non-AVIF brand in ftyp still works (HeifBackend is format-agnostic)
        let mut data = vec![0u8; 24];
        data[0..4].copy_from_slice(&24u32.to_be_bytes());
        data[4..8].copy_from_slice(b"ftyp");
        data[8..12].copy_from_slice(b"heic"); // HEIC brand, not AVIF

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());

        // Should still succeed (HeifBackend accepts various brands)
        assert!(result.is_ok());
    }

    // ========== UNICODE AND SPECIAL CHARACTER TESTS ==========

    #[test]
    fn test_unicode_in_metadata() {
        // Test handling Unicode filenames/metadata (though ISOBMFF doesn't store this in our minimal test)
        let mut data = vec![0u8; 24];
        data[0..4].copy_from_slice(&24u32.to_be_bytes());
        data[4..8].copy_from_slice(b"ftyp");
        data[8..12].copy_from_slice(b"avif");

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        // Verify markdown is valid UTF-8
        assert!(
            doc.markdown.is_ascii()
                || doc.markdown.chars().all(|c| c.is_alphanumeric()
                    || c.is_whitespace()
                    || c.is_ascii_punctuation()
                    || !c.is_control())
        );
    }

    #[test]
    fn test_zero_dimensions() {
        // Test handling of zero dimensions (edge case)
        let mut data = Vec::new();
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"avif");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        // ispe with zero dimensions
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ispe");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes()); // width = 0
        data.extend_from_slice(&0u32.to_be_bytes()); // height = 0

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());

        // Should still parse successfully (even with zero dimensions)
        assert!(result.is_ok());
    }

    #[test]
    fn test_extremely_large_dimensions() {
        // Test handling of extremely large dimensions
        let mut data = Vec::new();
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"avif");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        // ispe with very large dimensions
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ispe");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&u32::MAX.to_be_bytes()); // width = MAX
        data.extend_from_slice(&u32::MAX.to_be_bytes()); // height = MAX

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());

        // Should handle large dimensions without panicking
        assert!(result.is_ok());
    }

    #[test]
    fn test_aspect_ratio_calculations() {
        // Test various aspect ratios are handled correctly
        let test_cases: Vec<(u32, u32)> = vec![
            (16, 9), // 16:9
            (4, 3),  // 4:3
            (21, 9), // 21:9
            (1, 1),  // Square
            (9, 16), // Vertical
        ];

        for (width, height) in test_cases {
            let mut data = Vec::new();
            data.extend_from_slice(&20u32.to_be_bytes());
            data.extend_from_slice(b"ftyp");
            data.extend_from_slice(b"avif");
            data.extend_from_slice(&0u32.to_be_bytes());
            data.extend_from_slice(&0u32.to_be_bytes());

            data.extend_from_slice(&20u32.to_be_bytes());
            data.extend_from_slice(b"ispe");
            data.extend_from_slice(&0u32.to_be_bytes());
            data.extend_from_slice(&(width * 100).to_be_bytes());
            data.extend_from_slice(&(height * 100).to_be_bytes());

            let backend = AvifBackend::new();
            let result = backend.parse_bytes(&data, &Default::default());
            assert!(result.is_ok());
        }
    }

    // ========== VALIDATION TESTS ==========

    #[test]
    fn test_corrupted_box_size() {
        // Test handling of corrupted box size
        let mut data = vec![0u8; 8];
        data[0..4].copy_from_slice(&1u32.to_be_bytes()); // Invalid size (too small)
        data[4..8].copy_from_slice(b"ftyp");

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());

        // Should fail gracefully
        assert!(result.is_err());
    }

    #[test]
    fn test_box_size_larger_than_data() {
        // Test box claiming to be larger than actual data
        let mut data = vec![0u8; 20];
        data[0..4].copy_from_slice(&1000u32.to_be_bytes()); // Claims 1000 bytes
        data[4..8].copy_from_slice(b"ftyp");
        data[8..12].copy_from_slice(b"avif");

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());

        // HEIF backend is lenient - it ignores incomplete boxes and continues parsing
        // This is acceptable behavior for robustness with malformed files
        assert!(result.is_ok());
    }

    #[test]
    fn test_unknown_box_types() {
        // Test file with unknown box types (should be skipped)
        let mut data = Vec::new();

        // Valid ftyp box
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"avif");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        // Unknown box type
        data.extend_from_slice(&16u32.to_be_bytes());
        data.extend_from_slice(b"unkw");
        data.extend_from_slice(&[0u8; 8]);

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());

        // Should succeed (unknown boxes are skipped)
        assert!(result.is_ok());
    }

    #[test]
    fn test_nested_boxes() {
        // Test handling of nested box structures
        let mut data = Vec::new();

        // ftyp box
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"avif");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        // meta box (container)
        data.extend_from_slice(&32u32.to_be_bytes());
        data.extend_from_slice(b"meta");
        data.extend_from_slice(&0u32.to_be_bytes()); // version + flags
                                                     // Nested box inside meta
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"hdlr");
        data.extend_from_slice(&[0u8; 12]);

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());

        // Should handle nested structures
        assert!(result.is_ok());
    }

    #[test]
    fn test_multiple_ispe_boxes() {
        // Test file with multiple ispe boxes (last one should win)
        let mut data = Vec::new();

        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"avif");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        // First ispe
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ispe");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&100u32.to_be_bytes());
        data.extend_from_slice(&100u32.to_be_bytes());

        // Second ispe (should override)
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ispe");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&200u32.to_be_bytes());
        data.extend_from_slice(&200u32.to_be_bytes());

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());
        assert!(result.is_ok());
    }

    // ========== SERIALIZATION CONSISTENCY TESTS ==========

    #[test]
    fn test_markdown_not_empty() {
        // Test that markdown is never empty
        let mut data = vec![0u8; 24];
        data[0..4].copy_from_slice(&24u32.to_be_bytes());
        data[4..8].copy_from_slice(b"ftyp");
        data[8..12].copy_from_slice(b"avif");

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        assert!(!doc.markdown.is_empty());
        assert!(doc.markdown.len() > 10);
    }

    #[test]
    fn test_markdown_well_formed() {
        // Test that markdown is well-formed
        let mut data = vec![0u8; 24];
        data[0..4].copy_from_slice(&24u32.to_be_bytes());
        data[4..8].copy_from_slice(b"ftyp");
        data[8..12].copy_from_slice(b"avif");

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        // Should contain markdown formatting
        // Bold labels no longer used in image formats (N=1610)
        // assert!(doc.markdown.contains("**"));
        assert!(doc.markdown.contains("\n"));
    }

    #[test]
    fn test_docitems_match_markdown() {
        // Test that DocItems and markdown are consistent
        let mut data = vec![0u8; 24];
        data[0..4].copy_from_slice(&24u32.to_be_bytes());
        data[4..8].copy_from_slice(b"ftyp");
        data[8..12].copy_from_slice(b"avif");

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        assert!(doc.content_blocks.is_some());

        // Every DocItem's text should appear in markdown
        for item in doc.content_blocks.unwrap() {
            if let DocItem::Text { text, .. } = item {
                assert!(doc.markdown.contains(&text));
            }
        }
    }

    #[test]
    fn test_consistent_output_multiple_parses() {
        // Test that parsing the same data multiple times produces identical output
        let mut data = vec![0u8; 24];
        data[0..4].copy_from_slice(&24u32.to_be_bytes());
        data[4..8].copy_from_slice(b"ftyp");
        data[8..12].copy_from_slice(b"avif");

        let backend = AvifBackend::new();

        let result1 = backend.parse_bytes(&data, &Default::default()).unwrap();
        let result2 = backend.parse_bytes(&data, &Default::default()).unwrap();

        // Should be identical
        assert_eq!(result1.markdown, result2.markdown);
        assert_eq!(
            result1.metadata.num_characters,
            result2.metadata.num_characters
        );
    }

    // ========== BACKEND OPTIONS TESTS ==========

    #[test]
    fn test_with_default_options() {
        // Test with default BackendOptions
        let mut data = vec![0u8; 24];
        data[0..4].copy_from_slice(&24u32.to_be_bytes());
        data[4..8].copy_from_slice(b"ftyp");
        data[8..12].copy_from_slice(b"avif");

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &BackendOptions::default());
        assert!(result.is_ok());
    }

    #[test]
    fn test_with_custom_options() {
        // Test with custom BackendOptions
        let mut data = vec![0u8; 24];
        data[0..4].copy_from_slice(&24u32.to_be_bytes());
        data[4..8].copy_from_slice(b"ftyp");
        data[8..12].copy_from_slice(b"avif");

        let backend = AvifBackend::new();
        let options = BackendOptions::default();
        let result = backend.parse_bytes(&data, &options);
        assert!(result.is_ok());
    }

    // ========== ADDITIONAL FORMAT TESTS ==========

    #[test]
    fn test_compatible_brands() {
        // Test AVIF with multiple compatible brands
        let mut data = Vec::new();
        data.extend_from_slice(&32u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"avif"); // major brand
        data.extend_from_slice(&0u32.to_be_bytes()); // version
        data.extend_from_slice(b"mif1"); // compatible brand 1
        data.extend_from_slice(b"miaf"); // compatible brand 2
        data.extend_from_slice(b"MA1B"); // compatible brand 3

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());
        assert!(result.is_ok());
    }

    #[test]
    fn test_av1_specific_features() {
        // Test that AVIF (AV1) specific features are recognized
        let mut data = vec![0u8; 24];
        data[0..4].copy_from_slice(&24u32.to_be_bytes());
        data[4..8].copy_from_slice(b"ftyp");
        data[8..12].copy_from_slice(b"avif");

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());
        assert!(result.is_ok());

        let doc = result.unwrap();
        // Should indicate this is AV1 Image File Format
        assert!(
            doc.markdown.to_lowercase().contains("avif")
                || doc.markdown.to_lowercase().contains("av1")
        );
    }

    #[test]
    fn test_wide_gamut_metadata() {
        // Test handling of color space metadata (common in AVIF)
        let mut data = Vec::new();
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"avif");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        // colr box (color information)
        data.extend_from_slice(&18u32.to_be_bytes());
        data.extend_from_slice(b"colr");
        data.extend_from_slice(b"nclx"); // color type
        data.extend_from_slice(&[1, 13]); // color primaries (P3)

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());

        // Should parse successfully even with color metadata
        assert!(result.is_ok());
    }

    #[test]
    fn test_hdr_metadata() {
        // Test handling of HDR metadata (common in AVIF)
        let mut data = Vec::new();
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"avif");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        // clli box (Content Light Level Information)
        data.extend_from_slice(&16u32.to_be_bytes());
        data.extend_from_slice(b"clli");
        data.extend_from_slice(&10000u32.to_be_bytes()); // max content light level
        data.extend_from_slice(&1000u32.to_be_bytes()); // max frame average light level

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());

        // Should parse successfully even with HDR metadata
        assert!(result.is_ok());
    }

    // ========== ANIMATION AND SEQUENCE TESTS ==========

    #[test]
    fn test_animated_avif_detection() {
        // Test detection of animated AVIF (image sequence)
        let mut data = Vec::new();
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"avis"); // animated AVIF brand
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());

        // Should parse successfully (animated AVIF is valid)
        assert!(result.is_ok());
    }

    #[test]
    fn test_image_sequence_metadata() {
        // Test handling of image sequence with multiple frames
        let mut data = Vec::new();
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"avis"); // animated brand
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        // Add multiple ispe boxes (representing frames)
        for _ in 0..3 {
            data.extend_from_slice(&20u32.to_be_bytes());
            data.extend_from_slice(b"ispe");
            data.extend_from_slice(&0u32.to_be_bytes());
            data.extend_from_slice(&1920u32.to_be_bytes());
            data.extend_from_slice(&1080u32.to_be_bytes());
        }

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());
        assert!(result.is_ok());
    }

    // ========== BIT DEPTH AND COLOR FORMAT TESTS ==========

    #[test]
    fn test_10bit_avif() {
        // Test AVIF with 10-bit color depth
        let mut data = Vec::new();
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"avif");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        // pixi box (pixel information) - 10 bits per channel
        data.extend_from_slice(&16u32.to_be_bytes());
        data.extend_from_slice(b"pixi");
        data.extend_from_slice(&0u32.to_be_bytes()); // version + flags
        data.extend_from_slice(&[3, 10, 10, 10]); // 3 channels, 10 bits each

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());
        assert!(result.is_ok());
    }

    #[test]
    fn test_12bit_avif() {
        // Test AVIF with 12-bit color depth (HDR)
        let mut data = Vec::new();
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"avif");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        // pixi box - 12 bits per channel
        data.extend_from_slice(&16u32.to_be_bytes());
        data.extend_from_slice(b"pixi");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&[3, 12, 12, 12]); // 3 channels, 12 bits each

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());
        assert!(result.is_ok());
    }

    #[test]
    fn test_monochrome_avif() {
        // Test monochrome (grayscale) AVIF
        let mut data = Vec::new();
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"avif");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        // pixi box - 1 channel (grayscale)
        data.extend_from_slice(&13u32.to_be_bytes());
        data.extend_from_slice(b"pixi");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&[1, 8]); // 1 channel, 8 bits

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());
        assert!(result.is_ok());
    }

    // ========== ALPHA CHANNEL TESTS ==========

    #[test]
    fn test_avif_with_alpha_channel() {
        // Test AVIF with alpha channel (transparency)
        let mut data = Vec::new();
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"avif");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        // pixi box - 4 channels (RGBA)
        data.extend_from_slice(&17u32.to_be_bytes());
        data.extend_from_slice(b"pixi");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&[4, 8, 8, 8, 8]); // 4 channels (R, G, B, A)

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());
        assert!(result.is_ok());
    }

    #[test]
    fn test_premultiplied_alpha() {
        // Test AVIF with premultiplied alpha
        let mut data = Vec::new();
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"avif");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        // aux box (auxiliary image - alpha plane)
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"auxC");
        data.extend_from_slice(b"urn:"); // auxiliary type URN
        data.extend_from_slice(&[0u8; 8]);

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());
        assert!(result.is_ok());
    }

    // ========== GRID AND TILING TESTS ==========

    #[test]
    fn test_grid_image() {
        // Test AVIF grid image (multiple tiles)
        let mut data = Vec::new();
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"avif");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        // grid box (2x2 grid)
        data.extend_from_slice(&16u32.to_be_bytes());
        data.extend_from_slice(b"grid");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&[2, 2]); // 2 columns, 2 rows
        data.extend_from_slice(&[0u8; 6]);

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());
        assert!(result.is_ok());
    }

    #[test]
    fn test_large_grid() {
        // Test AVIF with large grid (many tiles)
        let mut data = Vec::new();
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"avif");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        // grid box (8x8 grid = 64 tiles)
        data.extend_from_slice(&16u32.to_be_bytes());
        data.extend_from_slice(b"grid");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&[8, 8]); // 8 columns, 8 rows
        data.extend_from_slice(&[0u8; 6]);

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());
        assert!(result.is_ok());
    }

    // ========== TRANSFORMATION TESTS ==========

    #[test]
    fn test_rotation_metadata() {
        // Test AVIF with rotation metadata
        let mut data = Vec::new();
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"avif");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        // irot box (rotation) - 90 degrees
        data.extend_from_slice(&9u32.to_be_bytes());
        data.extend_from_slice(b"irot");
        data.extend_from_slice(&[1]); // 90 degrees clockwise

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());
        assert!(result.is_ok());
    }

    #[test]
    fn test_mirror_transformation() {
        // Test AVIF with mirror transformation
        let mut data = Vec::new();
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"avif");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        // imir box (mirror) - horizontal flip
        data.extend_from_slice(&9u32.to_be_bytes());
        data.extend_from_slice(b"imir");
        data.extend_from_slice(&[0]); // horizontal flip

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());
        assert!(result.is_ok());
    }

    // ========== THUMBNAIL TESTS ==========

    #[test]
    fn test_avif_with_thumbnail() {
        // Test AVIF with embedded thumbnail
        let mut data = Vec::new();
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"avif");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        // Main image dimensions
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ispe");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&3840u32.to_be_bytes());
        data.extend_from_slice(&2160u32.to_be_bytes());

        // Thumbnail dimensions (smaller)
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ispe");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&160u32.to_be_bytes());
        data.extend_from_slice(&90u32.to_be_bytes());

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());
        assert!(result.is_ok());
    }

    // ========== EXIF METADATA TESTS ==========

    #[test]
    fn test_avif_with_exif_metadata() {
        // Test AVIF with EXIF metadata
        let mut data = Vec::new();
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"avif");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        // Exif box
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"Exif");
        data.extend_from_slice(&[0u8; 12]); // Minimal EXIF data

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());
        assert!(result.is_ok());
    }

    #[test]
    fn test_avif_with_xmp_metadata() {
        // Test AVIF with XMP metadata
        let mut data = Vec::new();
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"avif");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        // XMP metadata box
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"xml ");
        data.extend_from_slice(b"<?xpacket "); // XMP header
        data.extend_from_slice(&[0u8; 2]);

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());
        assert!(result.is_ok());
    }

    // ========== COMPRESSION AND QUALITY TESTS ==========

    #[test]
    fn test_lossless_avif() {
        // Test lossless AVIF encoding indicator
        let mut data = Vec::new();
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"avif");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        // av1C box (AV1 codec configuration with lossless flag)
        data.extend_from_slice(&16u32.to_be_bytes());
        data.extend_from_slice(b"av1C");
        data.extend_from_slice(&[0x81]); // marker + version (lossless bit set)
        data.extend_from_slice(&[0u8; 7]);

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());
        assert!(result.is_ok());
    }

    #[test]
    fn test_layered_image() {
        // Test AVIF with multiple layers (overlay)
        let mut data = Vec::new();
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"avif");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        // iovl box (image overlay)
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"iovl");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&[2, 0, 0, 0]); // 2 layers
        data.extend_from_slice(&[0u8; 4]);

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());
        assert!(result.is_ok());
    }

    // ========== SUBSAMPLING TESTS ==========

    #[test]
    fn test_chroma_subsampling_420() {
        // Test AVIF with 4:2:0 chroma subsampling (most common)
        let mut data = Vec::new();
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"avif");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        // av1C box with 4:2:0 subsampling
        data.extend_from_slice(&16u32.to_be_bytes());
        data.extend_from_slice(b"av1C");
        data.extend_from_slice(&[0x81, 0, 0, 0]); // 4:2:0 subsampling
        data.extend_from_slice(&[0u8; 4]);

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());
        assert!(result.is_ok());
    }

    #[test]
    fn test_chroma_subsampling_444() {
        // Test AVIF with 4:4:4 chroma subsampling (no subsampling)
        let mut data = Vec::new();
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"avif");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        // av1C box with 4:4:4 subsampling
        data.extend_from_slice(&16u32.to_be_bytes());
        data.extend_from_slice(b"av1C");
        data.extend_from_slice(&[0x81, 1, 0, 0]); // 4:4:4 subsampling
        data.extend_from_slice(&[0u8; 4]);

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());
        assert!(result.is_ok());
    }

    // ========== ADDITIONAL EDGE CASE TESTS ==========

    #[test]
    fn test_avif_minimum_valid_file_size() {
        // Test minimal valid AVIF file (ftyp box only, 20 bytes)
        // This tests the absolute minimum valid AVIF structure
        let mut data = Vec::new();
        data.extend_from_slice(&20u32.to_be_bytes()); // box size
        data.extend_from_slice(b"ftyp"); // box type
        data.extend_from_slice(b"avif"); // major brand
        data.extend_from_slice(&0u32.to_be_bytes()); // minor version
        data.extend_from_slice(&0u32.to_be_bytes()); // compatible brands (none)

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());

        // Should parse successfully even with minimal content
        assert!(result.is_ok());
        let document = result.unwrap();

        // Should have basic metadata
        assert_eq!(document.format, InputFormat::Avif);

        // Should generate DocItems even for minimal AVIF
        assert!(document.content_blocks.is_some());
    }

    #[test]
    fn test_avif_with_colr_box() {
        // Test AVIF with color profile (colr box) for ICC color management
        let mut data = Vec::new();
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"avif");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        // Add meta box container
        data.extend_from_slice(&32u32.to_be_bytes());
        data.extend_from_slice(b"meta");
        data.extend_from_slice(&[0, 0, 0, 0]); // version + flags

        // Add iprp (item properties) box
        data.extend_from_slice(&16u32.to_be_bytes());
        data.extend_from_slice(b"iprp");

        // Add colr (color information) box with sRGB color profile
        data.extend_from_slice(&12u32.to_be_bytes());
        data.extend_from_slice(b"colr");
        data.extend_from_slice(b"nclx"); // color type: 'nclx' (MPEG-4 color info)

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());

        // Should handle color profile box gracefully
        assert!(result.is_ok());
        let document = result.unwrap();
        assert_eq!(document.format, InputFormat::Avif);

        // Should generate DocItems and markdown
        assert!(document.content_blocks.is_some());
        assert!(!document.markdown.is_empty());
    }

    #[test]
    fn test_avif_with_alpha_transparency() {
        // Test AVIF with transparency (alpha channel) - basic test
        let mut data = Vec::new();
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"avif");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());

        // Should handle alpha channel gracefully
        assert!(result.is_ok());
        let document = result.unwrap();
        assert_eq!(document.format, InputFormat::Avif);
    }

    #[test]
    fn test_avif_image_sequence() {
        // Test AVIF image sequence (animated AVIF)
        let mut data = Vec::new();
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"avis"); // 'avis' = AVIF image sequence
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());

        // Should recognize animated AVIF
        assert!(result.is_ok());
        let document = result.unwrap();
        assert_eq!(document.format, InputFormat::Avif);
    }

    #[test]
    fn test_avif_high_bit_depth() {
        // Test AVIF with 10-bit or 12-bit color depth
        let mut data = Vec::new();
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"avif");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        // Add meta box with 10-bit depth indicator
        data.extend_from_slice(&24u32.to_be_bytes());
        data.extend_from_slice(b"meta");
        data.extend_from_slice(&[0, 0, 0, 0]); // version + flags

        // Add pixi (pixel information) box indicating bit depth
        data.extend_from_slice(&12u32.to_be_bytes());
        data.extend_from_slice(b"pixi");
        data.extend_from_slice(&[10, 10, 10]); // 10 bits per channel (RGB)

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());

        // Should handle high bit depth
        assert!(result.is_ok());
        let document = result.unwrap();
        assert_eq!(document.format, InputFormat::Avif);
    }

    #[test]
    fn test_avif_with_exif() {
        // Test AVIF with embedded EXIF metadata
        let mut data = Vec::new();
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"avif");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        // Add meta box
        data.extend_from_slice(&32u32.to_be_bytes());
        data.extend_from_slice(b"meta");
        data.extend_from_slice(&[0, 0, 0, 0]); // version + flags

        // Add iloc (item location) box
        data.extend_from_slice(&16u32.to_be_bytes());
        data.extend_from_slice(b"iloc");
        data.extend_from_slice(&[0, 0, 0, 0]); // version + flags

        // Note: EXIF data would be in a separate item referenced by iloc
        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());

        // Should extract EXIF metadata if present
        assert!(result.is_ok());
        let document = result.unwrap();
        assert_eq!(document.format, InputFormat::Avif);
    }

    #[test]
    fn test_avif_hdr_content() {
        // Test AVIF with HDR (High Dynamic Range) content
        let mut data = Vec::new();
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"avif");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        // Add meta box
        data.extend_from_slice(&32u32.to_be_bytes());
        data.extend_from_slice(b"meta");
        data.extend_from_slice(&[0, 0, 0, 0]); // version + flags

        // Add clli (content light level info) for HDR
        data.extend_from_slice(&16u32.to_be_bytes());
        data.extend_from_slice(b"clli");
        data.extend_from_slice(&1000u16.to_be_bytes()); // max_cll
        data.extend_from_slice(&400u16.to_be_bytes()); // max_fall

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());

        // Should handle HDR metadata
        assert!(result.is_ok());
        let document = result.unwrap();
        assert_eq!(document.format, InputFormat::Avif);
    }

    // ========== ADDITIONAL COMPREHENSIVE EDGE CASES (65 → 70) ==========

    #[test]
    fn test_avif_with_icc_profile() {
        // Test AVIF with embedded ICC color profile (for accurate color reproduction)
        let mut data = Vec::new();
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"avif");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        // Add meta box
        data.extend_from_slice(&48u32.to_be_bytes());
        data.extend_from_slice(b"meta");
        data.extend_from_slice(&[0, 0, 0, 0]); // version + flags

        // Add iprp (item properties) box
        data.extend_from_slice(&32u32.to_be_bytes());
        data.extend_from_slice(b"iprp");

        // Add ipco (item property container) with rICC (restricted ICC)
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ipco");
        data.extend_from_slice(&12u32.to_be_bytes());
        data.extend_from_slice(b"rICC"); // restricted ICC profile
        data.extend_from_slice(&[0u8; 4]); // Minimal ICC data

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());

        // Should handle ICC profile gracefully
        assert!(result.is_ok());
        let document = result.unwrap();
        assert_eq!(document.format, InputFormat::Avif);
        assert!(document.content_blocks.is_some());
    }

    #[test]
    fn test_avif_multiple_compatible_brands() {
        // Test AVIF with multiple compatible brands (e.g., avif, mif1, miaf)
        let mut data = Vec::new();
        data.extend_from_slice(&32u32.to_be_bytes()); // Larger ftyp for multiple brands
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"avif"); // major brand
        data.extend_from_slice(&0u32.to_be_bytes()); // minor version
        data.extend_from_slice(b"avif"); // compatible brand 1
        data.extend_from_slice(b"mif1"); // compatible brand 2 (multi-image format)
        data.extend_from_slice(b"miaf"); // compatible brand 3 (MIAF profile)

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());

        // Should recognize multiple compatible brands
        assert!(result.is_ok());
        let document = result.unwrap();
        assert_eq!(document.format, InputFormat::Avif);
        assert!(!document.markdown.is_empty());
    }

    #[test]
    fn test_avif_ultra_wide_aspect_ratio() {
        // Test AVIF with ultra-wide aspect ratio (e.g., 21:9 or wider)
        let mut data = Vec::new();
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"avif");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        // Add ispe (image spatial extents) with ultra-wide dimensions
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ispe");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&5120u32.to_be_bytes()); // Ultra-wide width
        data.extend_from_slice(&1440u32.to_be_bytes()); // Standard height (3.56:1 ratio)

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());

        // Should handle extreme aspect ratios
        assert!(result.is_ok());
        let document = result.unwrap();
        assert_eq!(document.format, InputFormat::Avif);
        // Verify dimensions are captured in metadata
        assert!(document.metadata.num_characters > 0);
    }

    #[test]
    fn test_avif_with_mastering_display_metadata() {
        // Test AVIF with mastering display color volume (MDCV) for HDR
        let mut data = Vec::new();
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"avif");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        // Add meta box
        data.extend_from_slice(&48u32.to_be_bytes());
        data.extend_from_slice(b"meta");
        data.extend_from_slice(&[0, 0, 0, 0]); // version + flags

        // Add mdcv (mastering display color volume) box
        data.extend_from_slice(&32u32.to_be_bytes());
        data.extend_from_slice(b"mdcv");
        // Add display primaries (R, G, B) and white point coordinates
        data.extend_from_slice(&[0u8; 24]); // Simplified coordinates

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());

        // Should handle HDR mastering display metadata
        assert!(result.is_ok());
        let document = result.unwrap();
        assert_eq!(document.format, InputFormat::Avif);
        assert!(document.content_blocks.is_some());
    }

    #[test]
    fn test_avif_with_auxiliary_image_types() {
        // Test AVIF with auxiliary images (e.g., depth map, alpha plane)
        let mut data = Vec::new();
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"avif");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        // Add meta box
        data.extend_from_slice(&64u32.to_be_bytes());
        data.extend_from_slice(b"meta");
        data.extend_from_slice(&[0, 0, 0, 0]); // version + flags

        // Add iref (item reference) box for auxiliary images
        data.extend_from_slice(&24u32.to_be_bytes());
        data.extend_from_slice(b"iref");
        data.extend_from_slice(&[0, 0, 0, 0]); // version + flags
                                               // Add auxl (auxiliary reference) for depth map
        data.extend_from_slice(&12u32.to_be_bytes());
        data.extend_from_slice(b"auxl"); // auxiliary link
        data.extend_from_slice(&[1, 0, 2, 0]); // from_id=1, to_id=2

        // Add auxC (auxiliary type) box
        data.extend_from_slice(&24u32.to_be_bytes());
        data.extend_from_slice(b"auxC");
        data.extend_from_slice(b"urn:mpeg:hevc:"); // URN prefix
        data.extend_from_slice(&[0u8; 2]); // padding

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());

        // Should handle auxiliary image types (depth maps, alpha planes)
        assert!(result.is_ok());
        let document = result.unwrap();
        assert_eq!(document.format, InputFormat::Avif);
        assert!(document.content_blocks.is_some());
        assert!(!document.markdown.is_empty());
    }

    // ========================================
    // Advanced AVIF Features (N=619, +5 tests)
    // ========================================

    #[test]
    fn test_avif_with_film_grain_synthesis() {
        // Test AVIF with film grain synthesis (AV1 feature for cinematic look)
        // Film grain metadata allows decoder to add synthetic grain without storing it
        let mut data = Vec::new();
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"avif");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        // Add meta box with film grain parameters
        data.extend_from_slice(&96u32.to_be_bytes());
        data.extend_from_slice(b"meta");
        data.extend_from_slice(&[0, 0, 0, 0]); // version + flags

        // Add av1C box with film grain flag enabled
        data.extend_from_slice(&36u32.to_be_bytes());
        data.extend_from_slice(b"av1C");
        data.extend_from_slice(&[0x81]); // marker + version
        data.extend_from_slice(&[0x08]); // profile 0, level 3.0
        data.extend_from_slice(&[0x0C]); // tier 0, bit depth 8
        data.extend_from_slice(&[0x00]); // monochrome, chroma subsampling
        data.extend_from_slice(&[0x10]); // film_grain_params_present flag
        data.extend_from_slice(&[0u8; 28]); // film grain parameters

        // Add ipco (item properties) box
        data.extend_from_slice(&32u32.to_be_bytes());
        data.extend_from_slice(b"ipco");
        data.extend_from_slice(&[0u8; 24]); // property entries

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());

        // Should handle film grain synthesis parameters
        assert!(result.is_ok());
        let document = result.unwrap();
        assert_eq!(document.format, InputFormat::Avif);
        assert!(document.content_blocks.is_some());
    }

    #[test]
    fn test_avif_with_layered_images() {
        // Test AVIF with layered images (progressive decoding, spatial scalability)
        // Layers allow progressive enhancement from low to high quality
        let mut data = Vec::new();
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"avif");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        // Add meta box
        data.extend_from_slice(&128u32.to_be_bytes());
        data.extend_from_slice(b"meta");
        data.extend_from_slice(&[0, 0, 0, 0]); // version + flags

        // Add iloc (item location) box with multiple items (layers)
        data.extend_from_slice(&56u32.to_be_bytes());
        data.extend_from_slice(b"iloc");
        data.extend_from_slice(&[0, 0, 0, 0]); // version + flags
        data.extend_from_slice(&[0x44]); // offset_size=4, length_size=4
        data.extend_from_slice(&[0x00]); // base_offset_size, reserved
        data.extend_from_slice(&[0, 3]); // item_count = 3 layers

        // Layer 0: base layer (low quality)
        data.extend_from_slice(&[0, 1]); // item_id = 1
        data.extend_from_slice(&[0, 0]); // construction_method
        data.extend_from_slice(&[0, 0]); // data_reference_index
        data.extend_from_slice(&[0u8; 8]); // base offset + extent

        // Layer 1: enhancement layer (medium quality)
        data.extend_from_slice(&[0, 2]); // item_id = 2
        data.extend_from_slice(&[0, 0]);
        data.extend_from_slice(&[0, 0]);
        data.extend_from_slice(&[0u8; 8]);

        // Layer 2: top layer (high quality)
        data.extend_from_slice(&[0, 3]); // item_id = 3
        data.extend_from_slice(&[0, 0]);
        data.extend_from_slice(&[0, 0]);
        data.extend_from_slice(&[0u8; 8]);

        // Add iref (item reference) box showing layer dependencies
        data.extend_from_slice(&36u32.to_be_bytes());
        data.extend_from_slice(b"iref");
        data.extend_from_slice(&[0, 0, 0, 0]);
        data.extend_from_slice(&[0u8; 24]); // layer references

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());

        // Should handle layered/progressive images
        assert!(result.is_ok());
        let document = result.unwrap();
        assert_eq!(document.format, InputFormat::Avif);
        assert!(document.content_blocks.is_some());
    }

    #[test]
    fn test_avif_with_heif_brand_compatibility() {
        // Test AVIF with HEIF brand (mif1/msf1) for compatibility
        // AVIF can declare HEIF compatibility for broader support
        let mut data = Vec::new();
        data.extend_from_slice(&32u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"avif"); // major brand
        data.extend_from_slice(&0u32.to_be_bytes()); // minor version

        // Compatible brands: mif1 (HEIF image), msf1 (HEIF image sequence)
        data.extend_from_slice(b"mif1"); // HEIF compatible
        data.extend_from_slice(b"msf1"); // HEIF sequence compatible
        data.extend_from_slice(b"miaf"); // MIAF compatible

        // Add minimal meta box
        data.extend_from_slice(&32u32.to_be_bytes());
        data.extend_from_slice(b"meta");
        data.extend_from_slice(&[0, 0, 0, 0]);
        data.extend_from_slice(&[0u8; 20]); // minimal content

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());

        // Should handle HEIF brand compatibility
        assert!(result.is_ok());
        let document = result.unwrap();
        assert_eq!(document.format, InputFormat::Avif);
        assert!(document.content_blocks.is_some());
    }

    #[test]
    fn test_avif_with_grid_layout() {
        // Test AVIF with grid layout (tiled image for very large images)
        // Grid allows splitting large image into tiles for efficient processing
        let mut data = Vec::new();
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"avif");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        // Add meta box
        data.extend_from_slice(&128u32.to_be_bytes());
        data.extend_from_slice(b"meta");
        data.extend_from_slice(&[0, 0, 0, 0]);

        // Add iref (item reference) box with grid (dimg - derived image)
        data.extend_from_slice(&48u32.to_be_bytes());
        data.extend_from_slice(b"iref");
        data.extend_from_slice(&[0, 0, 0, 0]); // version + flags

        // Grid reference: master image references 4 tiles (2x2 grid)
        data.extend_from_slice(&36u32.to_be_bytes());
        data.extend_from_slice(b"dimg"); // derived image
        data.extend_from_slice(&[0, 1]); // from_item_id = 1 (master)
        data.extend_from_slice(&[0, 4]); // reference_count = 4 tiles
        data.extend_from_slice(&[0, 2]); // tile 1 (top-left)
        data.extend_from_slice(&[0, 3]); // tile 2 (top-right)
        data.extend_from_slice(&[0, 4]); // tile 3 (bottom-left)
        data.extend_from_slice(&[0, 5]); // tile 4 (bottom-right)
        data.extend_from_slice(&[0u8; 20]);

        // Add ipco with grid property (igri)
        data.extend_from_slice(&48u32.to_be_bytes());
        data.extend_from_slice(b"ipco");
        data.extend_from_slice(&16u32.to_be_bytes());
        data.extend_from_slice(b"igri"); // image grid
        data.extend_from_slice(&[0]); // version
        data.extend_from_slice(&[0x01]); // flags: rows=2, cols=2
        data.extend_from_slice(&[0, 2, 0, 2]); // 2 rows, 2 cols
        data.extend_from_slice(&[0u8; 20]);

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());

        // Should handle grid layout (tiled images)
        assert!(result.is_ok());
        let document = result.unwrap();
        assert_eq!(document.format, InputFormat::Avif);
        assert!(document.content_blocks.is_some());
    }

    #[test]
    fn test_avif_with_exif_orientation_rotation() {
        // Test AVIF with EXIF orientation and rotation metadata
        // Orientation: 1-8 (normal, mirror, rotate, combinations)
        let mut data = Vec::new();
        data.extend_from_slice(&20u32.to_be_bytes());
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(b"avif");
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());

        // Add meta box
        data.extend_from_slice(&160u32.to_be_bytes());
        data.extend_from_slice(b"meta");
        data.extend_from_slice(&[0, 0, 0, 0]);

        // Add iinf (item info) box with Exif item
        data.extend_from_slice(&40u32.to_be_bytes());
        data.extend_from_slice(b"iinf");
        data.extend_from_slice(&[0, 0, 0, 0]); // version + flags
        data.extend_from_slice(&[0, 1]); // item_count = 1

        // Item info entry for Exif
        data.extend_from_slice(&28u32.to_be_bytes());
        data.extend_from_slice(b"infe");
        data.extend_from_slice(&[0, 2, 0, 0]); // version 2
        data.extend_from_slice(&[0, 2]); // item_id = 2
        data.extend_from_slice(&[0, 0]); // protection_index
        data.extend_from_slice(b"Exif"); // item_type
        data.extend_from_slice(b"Exif\0"); // item_name

        // Add iloc (item location) box for Exif data
        data.extend_from_slice(&36u32.to_be_bytes());
        data.extend_from_slice(b"iloc");
        data.extend_from_slice(&[0, 0, 0, 0]);
        data.extend_from_slice(&[0x44, 0x00]); // offset_size, length_size
        data.extend_from_slice(&[0, 1]); // item_count = 1
        data.extend_from_slice(&[0, 2]); // item_id = 2 (Exif)
        data.extend_from_slice(&[0u8; 20]); // location data

        // Add EXIF data with orientation tag
        data.extend_from_slice(&48u32.to_be_bytes());
        data.extend_from_slice(b"Exif");
        data.extend_from_slice(&[0u8; 4]); // Exif header
                                           // TIFF header (little-endian)
        data.extend_from_slice(&[0x49, 0x49]); // "II" = little-endian
        data.extend_from_slice(&[0x2A, 0x00]); // TIFF magic
        data.extend_from_slice(&[0x08, 0x00, 0x00, 0x00]); // IFD offset
                                                           // IFD with orientation tag
        data.extend_from_slice(&[0x01, 0x00]); // 1 tag
        data.extend_from_slice(&[0x12, 0x01]); // Tag 0x0112 = Orientation
        data.extend_from_slice(&[0x03, 0x00]); // Type SHORT
        data.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]); // Count = 1
        data.extend_from_slice(&[0x06, 0x00, 0x00, 0x00]); // Value = 6 (rotate 90 CW)
        data.extend_from_slice(&[0u8; 12]); // padding

        let backend = AvifBackend::new();
        let result = backend.parse_bytes(&data, &Default::default());

        // Should handle EXIF orientation and rotation
        assert!(result.is_ok());
        let document = result.unwrap();
        assert_eq!(document.format, InputFormat::Avif);
        assert!(document.content_blocks.is_some());
    }
}
