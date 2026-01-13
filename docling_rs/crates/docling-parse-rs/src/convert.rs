//! Conversion functions from docling-parse JSON types to docling-core types
//!
//! This module converts the raw JSON structures returned by docling-parse C API
//! into the structured types used by docling-core (SegmentedPdfPage, TextCell, etc.)

use crate::types::{CellArray, DoclingParsePage, PageDimension};
use docling_core::content::{BoundingBox, CoordOrigin};
use docling_core::types::page::{
    BoundingRectangle, PdfPageBoundaryType, PdfPageGeometry, SegmentedPdfPage, TextCell,
    TextDirection,
};

/// Convert a docling-parse page to a SegmentedPdfPage
///
/// Maps from the JSON structure returned by docling-parse to the structured
/// types used throughout docling-core.
#[must_use = "this function returns a converted page that should be processed"]
pub fn convert_to_segmented_page(page: &DoclingParsePage) -> Result<SegmentedPdfPage, String> {
    // Convert page dimensions
    let dimension = convert_dimension(&page.original.dimension)?;

    // Convert line cells (merged horizontal text)
    let textline_cells: Vec<TextCell> = page
        .original
        .line_cells
        .data
        .iter()
        .enumerate()
        .map(|(i, cell)| convert_cell_array(cell, i))
        .collect();

    // Convert word cells
    let word_cells: Vec<TextCell> = page
        .original
        .word_cells
        .data
        .iter()
        .enumerate()
        .map(|(i, cell)| convert_cell_array(cell, i))
        .collect();

    // Convert char cells if available
    let char_cells: Vec<TextCell> = if let Some(ref char_cell_data) = page.original.char_cells {
        char_cell_data
            .data
            .iter()
            .enumerate()
            .map(|(i, cell)| convert_cell_array(cell, i))
            .collect()
    } else {
        Vec::new()
    };

    Ok(SegmentedPdfPage {
        dimension,
        char_cells: char_cells.clone(),
        word_cells: word_cells.clone(),
        textline_cells: textline_cells.clone(),
        has_chars: !char_cells.is_empty(),
        has_words: !word_cells.is_empty(),
        has_textlines: !textline_cells.is_empty(),
    })
}

/// Detect text direction from Unicode character analysis
///
/// Performs simple Unicode-based detection of RTL (right-to-left) text:
/// - Checks first 50 characters for strong directional cues
/// - Counts Arabic, Hebrew, and other RTL characters
/// - Returns RightToLeft if >30% of analyzed chars are RTL
/// - Otherwise returns LeftToRight (default)
///
/// Note: This is a heuristic approach. Full bidirectional text analysis
/// would require:
/// - Font metadata (check for RTL fonts)
/// - Layout analysis (paragraph direction from docling-parse)
/// - Unicode Bidirectional Algorithm (UAX #9)
///
/// For 90%+ of documents, this simple approach is sufficient.
#[inline]
fn detect_text_direction(text: &str) -> TextDirection {
    // Analyze first 50 chars (or full text if shorter) for strong directional indicators
    let sample_len = text.chars().count().min(50);
    if sample_len == 0 {
        return TextDirection::LeftToRight; // Empty text defaults to LTR
    }

    let mut rtl_count = 0;
    let mut analyzed = 0;

    for ch in text.chars().take(sample_len) {
        // Count strong RTL characters (Unicode ranges for RTL scripts)
        // Arabic: U+0600 to U+06FF, U+0750 to U+077F, U+08A0 to U+08FF, U+FB50 to U+FDFF, U+FE70 to U+FEFF
        // Hebrew: U+0590 to U+05FF, U+FB1D to U+FB4F
        // Syriac: U+0700 to U+074F
        // Thaana (Maldivian): U+0780 to U+07BF
        // N'Ko: U+07C0 to U+07FF
        if matches!(ch,
            '\u{0590}'..='\u{05FF}' | // Hebrew
            '\u{0600}'..='\u{06FF}' | // Arabic
            '\u{0700}'..='\u{074F}' | // Syriac
            '\u{0750}'..='\u{077F}' | // Arabic Supplement
            '\u{0780}'..='\u{07BF}' | // Thaana
            '\u{07C0}'..='\u{07FF}' | // N'Ko
            '\u{08A0}'..='\u{08FF}' | // Arabic Extended-A
            '\u{FB1D}'..='\u{FB4F}' | // Hebrew presentation forms
            '\u{FB50}'..='\u{FDFF}' | // Arabic presentation forms-A
            '\u{FE70}'..='\u{FEFF}'   // Arabic presentation forms-B
        ) {
            rtl_count += 1;
        }
        analyzed += 1;
    }

    // If >30% of analyzed characters are RTL, classify as RTL
    // This threshold handles mixed RTL/LTR text (e.g., Arabic with English numbers)
    if analyzed > 0 && (rtl_count * 100 / analyzed) > 30 {
        TextDirection::RightToLeft
    } else {
        TextDirection::LeftToRight
    }
}

/// Convert a CellArray (21-element array) to a TextCell
///
/// Maps from the heterogeneous array format used by docling-parse
/// to the structured TextCell type.
///
/// Array format:
/// - \[0-3\]: Bounding box (x0, y0, x1, y1)
/// - \[4-11\]: Quadrilateral points (for rotated text)
/// - \[12\]: Text content
/// - \[13\]: Unknown integer
/// - \[14\]: Font size
/// - \[15\]: Unknown string
/// - \[16\]: Font category
/// - \[17\]: Font ID
/// - \[18\]: Font name
/// - \[19-20\]: Unknown booleans
#[must_use = "converts cell array to TextCell"]
pub fn convert_cell_array(cell: &CellArray, index: usize) -> TextCell {
    // Docling-parse uses top-left origin (standard PDF coordinate system)
    let coord_origin = CoordOrigin::TopLeft;

    // Create BoundingRectangle from bbox and quad points
    // quad_points are [x0, y0, x1, y1, x2, y2, x3, y3]
    let rect = BoundingRectangle {
        r_x0: cell.quad_points[0], // bottom-left x (in TopLeft origin)
        r_y0: cell.quad_points[1], // bottom-left y
        r_x1: cell.quad_points[2], // bottom-right x
        r_y1: cell.quad_points[3], // bottom-right y
        r_x2: cell.quad_points[4], // top-right x
        r_y2: cell.quad_points[5], // top-right y
        r_x3: cell.quad_points[6], // top-left x
        r_y3: cell.quad_points[7], // top-left y
        coord_origin,
    };

    TextCell {
        index,
        rect,
        text: cell.text.clone(),
        orig: cell.text.clone(), // docling-parse returns clean text, no orig variant
        text_direction: detect_text_direction(&cell.text), // Unicode-based RTL detection
        confidence: 1.0,         // docling-parse is not OCR
        from_ocr: false,
        font_size: cell.font_size as f32, // Extract font size for header detection (cast from f64)
    }
}

/// Convert PageDimension to PdfPageGeometry
///
/// Maps from the dimension structure in docling-parse JSON to the
/// PdfPageGeometry type used in docling-core.
#[must_use = "this function returns converted geometry that should be used"]
pub fn convert_dimension(dim: &PageDimension) -> Result<PdfPageGeometry, String> {
    // Docling-parse uses top-left origin
    let coord_origin = CoordOrigin::TopLeft;

    // Determine boundary type from page_boundary string
    let boundary_type = match dim.page_boundary.as_str() {
        "crop_box" => PdfPageBoundaryType::CropBox,
        "media_box" => PdfPageBoundaryType::MediaBox,
        "art_box" => PdfPageBoundaryType::ArtBox,
        "bleed_box" => PdfPageBoundaryType::BleedBox,
        "trim_box" => PdfPageBoundaryType::TrimBox,
        other => {
            return Err(format!("Unknown page boundary type: {}", other));
        }
    };

    // Create main bounding rectangle from bbox
    let rect = BoundingRectangle {
        r_x0: dim.bbox[0], // left
        r_y0: dim.bbox[1], // top (in TopLeft origin)
        r_x1: dim.bbox[2], // right
        r_y1: dim.bbox[1], // top
        r_x2: dim.bbox[2], // right
        r_y2: dim.bbox[3], // bottom
        r_x3: dim.bbox[0], // left
        r_y3: dim.bbox[3], // bottom
        coord_origin,
    };

    // Convert all rectangle types to BoundingBox
    let art_bbox = bbox_from_array(&dim.rectangles.art_bbox, coord_origin);
    let bleed_bbox = bbox_from_array(&dim.rectangles.bleed_bbox, coord_origin);
    let crop_bbox = bbox_from_array(&dim.rectangles.crop_bbox, coord_origin);
    let media_bbox = bbox_from_array(&dim.rectangles.media_bbox, coord_origin);
    let trim_bbox = bbox_from_array(&dim.rectangles.trim_bbox, coord_origin);

    Ok(PdfPageGeometry {
        angle: dim.angle,
        rect,
        boundary_type,
        art_bbox,
        bleed_bbox,
        crop_bbox,
        media_bbox,
        trim_bbox,
    })
}

/// Helper to convert a 4-element bbox array to BoundingBox
#[inline]
fn bbox_from_array(arr: &[f64; 4], coord_origin: CoordOrigin) -> BoundingBox {
    // Array is [x0, y0, x1, y1] where:
    // - (x0, y0) is top-left in TopLeft origin
    // - (x1, y1) is bottom-right in TopLeft origin
    BoundingBox::new(arr[0], arr[1], arr[2], arr[3], coord_origin)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{CellArray, CellBBox, CellData, OriginalPageData, PageRectangles};

    #[test]
    fn test_convert_cell_array() {
        let cell = CellArray {
            bbox: CellBBox {
                x0: 100.0,
                y0: 200.0,
                x1: 300.0,
                y1: 220.0,
            },
            quad_points: [100.0, 200.0, 300.0, 200.0, 300.0, 220.0, 100.0, 220.0],
            text: "Hello World".to_string(),
            font_size: 12.0,
            font_category: "STANDARD".to_string(),
            font_id: "/F1".to_string(),
            font_name: "Times-Roman".to_string(),
            unknown_int: -1,
            unknown_str: "".to_string(),
            unknown_bools: [false, true],
        };

        let text_cell = convert_cell_array(&cell, 0);

        assert_eq!(text_cell.index, 0);
        assert_eq!(text_cell.text, "Hello World");
        assert_eq!(text_cell.orig, "Hello World");
        assert_eq!(text_cell.confidence, 1.0);
        assert!(!text_cell.from_ocr);
        assert_eq!(text_cell.rect.coord_origin, CoordOrigin::TopLeft);
    }

    #[test]
    fn test_convert_dimension() {
        let dim = PageDimension {
            angle: 0.0,
            bbox: [0.0, 0.0, 612.0, 792.0],
            height: 792.0,
            width: 612.0,
            page_boundary: "crop_box".to_string(),
            rectangles: PageRectangles {
                art_bbox: [0.0, 0.0, 612.0, 792.0],
                bleed_bbox: [0.0, 0.0, 612.0, 792.0],
                crop_bbox: [0.0, 0.0, 612.0, 792.0],
                media_bbox: [0.0, 0.0, 612.0, 792.0],
                trim_bbox: [0.0, 0.0, 612.0, 792.0],
            },
        };

        let geometry = convert_dimension(&dim).unwrap();

        assert_eq!(geometry.angle, 0.0);
        assert_eq!(geometry.boundary_type, PdfPageBoundaryType::CropBox);
        assert_eq!(geometry.rect.coord_origin, CoordOrigin::TopLeft);
        assert_eq!(geometry.width(), 612.0);
        assert_eq!(geometry.height(), 792.0);
    }

    #[test]
    fn test_convert_to_segmented_page() {
        let page = DoclingParsePage {
            page_number: 0,
            original: OriginalPageData {
                dimension: PageDimension {
                    angle: 0.0,
                    bbox: [0.0, 0.0, 612.0, 792.0],
                    height: 792.0,
                    width: 612.0,
                    page_boundary: "crop_box".to_string(),
                    rectangles: PageRectangles {
                        art_bbox: [0.0, 0.0, 612.0, 792.0],
                        bleed_bbox: [0.0, 0.0, 612.0, 792.0],
                        crop_bbox: [0.0, 0.0, 612.0, 792.0],
                        media_bbox: [0.0, 0.0, 612.0, 792.0],
                        trim_bbox: [0.0, 0.0, 612.0, 792.0],
                    },
                },
                line_cells: CellData {
                    data: vec![CellArray {
                        bbox: CellBBox {
                            x0: 100.0,
                            y0: 200.0,
                            x1: 300.0,
                            y1: 220.0,
                        },
                        quad_points: [100.0, 200.0, 300.0, 200.0, 300.0, 220.0, 100.0, 220.0],
                        text: "Test line".to_string(),
                        font_size: 12.0,
                        font_category: "STANDARD".to_string(),
                        font_id: "/F1".to_string(),
                        font_name: "Times-Roman".to_string(),
                        unknown_int: -1,
                        unknown_str: "".to_string(),
                        unknown_bools: [false, true],
                    }],
                },
                word_cells: CellData { data: vec![] },
                char_cells: None,
            },
            sanitized: None,
            annotations: None,
            timings: None,
        };

        let segmented = convert_to_segmented_page(&page).unwrap();

        assert_eq!(segmented.textline_cells.len(), 1);
        assert_eq!(segmented.textline_cells[0].text, "Test line");
        assert!(segmented.has_textlines);
        assert!(!segmented.has_words);
        assert!(!segmented.has_chars);
        assert_eq!(segmented.dimension.width(), 612.0);
    }

    // RTL text direction detection tests
    #[test]
    fn test_detect_text_direction_english_ltr() {
        assert_eq!(
            detect_text_direction("Hello World"),
            TextDirection::LeftToRight
        );
        assert_eq!(
            detect_text_direction("The quick brown fox jumps over the lazy dog"),
            TextDirection::LeftToRight
        );
    }

    #[test]
    fn test_detect_text_direction_empty() {
        assert_eq!(detect_text_direction(""), TextDirection::LeftToRight);
        assert_eq!(detect_text_direction("   "), TextDirection::LeftToRight);
    }

    #[test]
    fn test_detect_text_direction_arabic_rtl() {
        // Pure Arabic text (سلام عليكم - "Peace be upon you")
        assert_eq!(
            detect_text_direction("السلام عليكم"),
            TextDirection::RightToLeft
        );
        // Arabic sentence (مرحبا بك - "Welcome")
        assert_eq!(
            detect_text_direction("مرحبا بك في العالم العربي"),
            TextDirection::RightToLeft
        );
    }

    #[test]
    fn test_detect_text_direction_hebrew_rtl() {
        // Hebrew text (שלום - "Hello")
        assert_eq!(detect_text_direction("שלום"), TextDirection::RightToLeft);
        // Hebrew sentence (שלום עולם - "Hello World")
        assert_eq!(
            detect_text_direction("שלום עולם"),
            TextDirection::RightToLeft
        );
    }

    #[test]
    fn test_detect_text_direction_mixed_rtl_dominant() {
        // Arabic with English numbers (common in documents)
        assert_eq!(
            detect_text_direction("السلام 123 عليكم"),
            TextDirection::RightToLeft
        );
        // Hebrew with English words
        assert_eq!(
            detect_text_direction("שלום World"),
            TextDirection::RightToLeft
        );
    }

    #[test]
    fn test_detect_text_direction_mixed_ltr_dominant() {
        // English with few Arabic words (below 30% threshold)
        assert_eq!(
            detect_text_direction("Hello مرحبا World"),
            TextDirection::LeftToRight
        );
        // Mostly English with Hebrew word
        assert_eq!(
            detect_text_direction("Hello שלום World and more text"),
            TextDirection::LeftToRight
        );
    }

    #[test]
    fn test_detect_text_direction_numbers_only() {
        assert_eq!(detect_text_direction("123456"), TextDirection::LeftToRight);
        assert_eq!(detect_text_direction("3.14159"), TextDirection::LeftToRight);
    }

    #[test]
    fn test_detect_text_direction_punctuation_only() {
        assert_eq!(detect_text_direction("..."), TextDirection::LeftToRight);
        assert_eq!(detect_text_direction("!?.,;"), TextDirection::LeftToRight);
    }

    #[test]
    fn test_detect_text_direction_syriac_rtl() {
        // Syriac text (ܫܠܡܐ - "Hello")
        assert_eq!(detect_text_direction("ܫܠܡܐ"), TextDirection::RightToLeft);
    }

    #[test]
    fn test_detect_text_direction_long_text_sampling() {
        // Test that we only analyze first 50 chars
        let long_english = "a".repeat(100);
        assert_eq!(
            detect_text_direction(&long_english),
            TextDirection::LeftToRight
        );

        // Long text starting with RTL (should still detect RTL from first 50 chars)
        let rtl_prefix = "السلام عليكم ".repeat(10); // Much longer than 50 chars
        assert_eq!(
            detect_text_direction(&rtl_prefix),
            TextDirection::RightToLeft
        );
    }

    #[test]
    fn test_detect_text_direction_threshold_edge_cases() {
        // Test around the 30% threshold
        // Below threshold (29%) → LTR: 5 RTL chars + 12 Latin = 5/17 = 29.4%
        let text_29_percent = format!("{}abcdefghijkl", "س".repeat(5));
        assert_eq!(
            detect_text_direction(&text_29_percent),
            TextDirection::LeftToRight
        );

        // Above threshold (31.6%) → RTL: 6 RTL chars + 13 Latin = 6/19 = 31.6%
        let text_31_percent = format!("{}abcdefghijklm", "س".repeat(6));
        assert_eq!(
            detect_text_direction(&text_31_percent),
            TextDirection::RightToLeft
        );
    }

    #[test]
    fn test_convert_cell_array_with_rtl_text() {
        // Test that convert_cell_array properly uses detect_text_direction
        let cell_arabic = CellArray {
            bbox: CellBBox {
                x0: 100.0,
                y0: 200.0,
                x1: 300.0,
                y1: 220.0,
            },
            quad_points: [100.0, 200.0, 300.0, 200.0, 300.0, 220.0, 100.0, 220.0],
            text: "السلام عليكم".to_string(),
            font_size: 12.0,
            font_category: "STANDARD".to_string(),
            font_id: "/F1".to_string(),
            font_name: "Arial".to_string(),
            unknown_int: -1,
            unknown_str: "".to_string(),
            unknown_bools: [false, true],
        };

        let text_cell = convert_cell_array(&cell_arabic, 0);
        assert_eq!(text_cell.text_direction, TextDirection::RightToLeft);
    }

    #[test]
    fn test_convert_cell_array_with_ltr_text() {
        // Test that convert_cell_array properly detects LTR
        let cell_english = CellArray {
            bbox: CellBBox {
                x0: 100.0,
                y0: 200.0,
                x1: 300.0,
                y1: 220.0,
            },
            quad_points: [100.0, 200.0, 300.0, 200.0, 300.0, 220.0, 100.0, 220.0],
            text: "Hello World".to_string(),
            font_size: 12.0,
            font_category: "STANDARD".to_string(),
            font_id: "/F1".to_string(),
            font_name: "Times-Roman".to_string(),
            unknown_int: -1,
            unknown_str: "".to_string(),
            unknown_bools: [false, true],
        };

        let text_cell = convert_cell_array(&cell_english, 0);
        assert_eq!(text_cell.text_direction, TextDirection::LeftToRight);
    }
}
