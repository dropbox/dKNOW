//! Page-level types for PDF document processing
//!
//! This module contains types for representing page geometry, text cells,
//! and segmented pages. These types match Python's `docling_core.types.doc.page`
//!
//! Python source: `docling_core/types/doc/page.py`
//! Lines referenced: Full file (`BoundingRectangle`, `TextCell`, `PdfPageGeometry`, `SegmentedPdfPage`)

use crate::content::{BoundingBox, CoordOrigin};
use serde::{Deserialize, Serialize};

// ============================================================================
// Enums
// ============================================================================

/// PDF page boundary types
/// Python: `PdfPageBoundaryType` (line 51)
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PdfPageBoundaryType {
    /// Art box - extent of meaningful content
    ArtBox,
    /// Bleed box - extent including bleed for printing
    BleedBox,
    /// Crop box - region to display/print
    CropBox,
    /// Media box - physical page size
    #[default]
    MediaBox,
    /// Trim box - intended finished page size
    TrimBox,
}

impl std::fmt::Display for PdfPageBoundaryType {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::ArtBox => "art_box",
            Self::BleedBox => "bleed_box",
            Self::CropBox => "crop_box",
            Self::MediaBox => "media_box",
            Self::TrimBox => "trim_box",
        };
        write!(f, "{s}")
    }
}

impl std::str::FromStr for PdfPageBoundaryType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "art_box" | "artbox" | "art-box" => Ok(Self::ArtBox),
            "bleed_box" | "bleedbox" | "bleed-box" => Ok(Self::BleedBox),
            "crop_box" | "cropbox" | "crop-box" => Ok(Self::CropBox),
            "media_box" | "mediabox" | "media-box" => Ok(Self::MediaBox),
            "trim_box" | "trimbox" | "trim-box" => Ok(Self::TrimBox),
            _ => Err(format!("unknown PDF page boundary type: '{s}'")),
        }
    }
}

/// Text direction for text cells
/// Python: `TextDirection` (line 248)
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextDirection {
    /// Left-to-right text (English, most European languages)
    #[default]
    LeftToRight,
    /// Right-to-left text (Arabic, Hebrew)
    RightToLeft,
    /// Direction not specified
    Unspecified,
}

impl std::fmt::Display for TextDirection {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::LeftToRight => "left_to_right",
            Self::RightToLeft => "right_to_left",
            Self::Unspecified => "unspecified",
        };
        write!(f, "{s}")
    }
}

impl std::str::FromStr for TextDirection {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "left_to_right" | "lefttoright" | "left-to-right" | "ltr" => Ok(Self::LeftToRight),
            "right_to_left" | "righttoleft" | "right-to-left" | "rtl" => Ok(Self::RightToLeft),
            "unspecified" | "unknown" | "" => Ok(Self::Unspecified),
            _ => Err(format!("unknown text direction: '{s}'")),
        }
    }
}

// ============================================================================
// BoundingRectangle - 4-corner bounding box
// ============================================================================

/// Model representing a rectangular boundary with four corner points
/// Python: `BoundingRectangle` (line 94)
///
/// Coordinate system:
/// - `r_x0`, `r_y0`: bottom-left corner (or top-left if `coord_origin=TOPLEFT`)
/// - `r_x1`, `r_y1`: bottom-right corner (or top-right if `coord_origin=TOPLEFT`)
/// - `r_x2`, `r_y2`: top-right corner (or bottom-right if `coord_origin=TOPLEFT`)
/// - `r_x3`, `r_y3`: top-left corner (or bottom-left if `coord_origin=TOPLEFT`)
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct BoundingRectangle {
    /// X coordinate of corner 0
    pub r_x0: f64,
    /// Y coordinate of corner 0
    pub r_y0: f64,
    /// X coordinate of corner 1
    pub r_x1: f64,
    /// Y coordinate of corner 1
    pub r_y1: f64,
    /// X coordinate of corner 2
    pub r_x2: f64,
    /// Y coordinate of corner 2
    pub r_y2: f64,
    /// X coordinate of corner 3
    pub r_x3: f64,
    /// Y coordinate of corner 3
    pub r_y3: f64,
    /// Coordinate origin (bottom-left or top-left)
    pub coord_origin: CoordOrigin,
}

impl BoundingRectangle {
    /// Calculate the width of the rectangle
    /// Python: width property (line 111)
    #[inline]
    #[must_use = "rectangle width is computed but not used"]
    pub fn width(&self) -> f64 {
        (self.r_x1 - self.r_x0).hypot(self.r_y1 - self.r_y0)
    }

    /// Calculate the height of the rectangle
    /// Python: height property (line 116)
    #[inline]
    #[must_use = "rectangle height is computed but not used"]
    pub fn height(&self) -> f64 {
        (self.r_x3 - self.r_x0).hypot(self.r_y3 - self.r_y0)
    }

    /// Convert to a `BoundingBox` representation
    /// Python: `to_bounding_box()` (line 144)
    #[inline]
    #[must_use = "bounding box is created but not used"]
    pub fn to_bounding_box(&self) -> BoundingBox {
        let (top, bottom) = if self.coord_origin == CoordOrigin::BottomLeft {
            let top = self.r_y0.max(self.r_y1).max(self.r_y2).max(self.r_y3);
            let bottom = self.r_y0.min(self.r_y1).min(self.r_y2).min(self.r_y3);
            (top, bottom)
        } else {
            let top = self.r_y0.min(self.r_y1).min(self.r_y2).min(self.r_y3);
            let bottom = self.r_y0.max(self.r_y1).max(self.r_y2).max(self.r_y3);
            (top, bottom)
        };

        let left = self.r_x0.min(self.r_x1).min(self.r_x2).min(self.r_x3);
        let right = self.r_x0.max(self.r_x1).max(self.r_x2).max(self.r_x3);

        BoundingBox::new(left, top, right, bottom, self.coord_origin)
    }

    /// Convert a `BoundingBox` into a `BoundingRectangle`
    /// Python: `from_bounding_box()` (line 162)
    #[inline]
    #[must_use = "bounding rectangle is created but not used"]
    pub const fn from_bounding_box(bbox: &BoundingBox) -> Self {
        Self {
            r_x0: bbox.l,
            r_y0: bbox.b,
            r_x1: bbox.r,
            r_y1: bbox.b,
            r_x2: bbox.r,
            r_y2: bbox.t,
            r_x3: bbox.l,
            r_y3: bbox.t,
            coord_origin: bbox.coord_origin,
        }
    }

    /// Convert coordinates to use bottom-left origin
    /// Python: `to_bottom_left_origin()` (line 178)
    ///
    /// Args:
    ///     `page_height`: The height of the page
    ///
    /// Returns:
    ///     `BoundingRectangle` with bottom-left origin
    #[inline]
    #[must_use = "converted rectangle is returned but not used"]
    pub fn to_bottom_left_origin(&self, page_height: f64) -> Self {
        if self.coord_origin == CoordOrigin::BottomLeft {
            return *self;
        }

        // coord_origin == CoordOrigin::TopLeft
        Self {
            r_x0: self.r_x0,
            r_x1: self.r_x1,
            r_x2: self.r_x2,
            r_x3: self.r_x3,
            r_y0: page_height - self.r_y0,
            r_y1: page_height - self.r_y1,
            r_y2: page_height - self.r_y2,
            r_y3: page_height - self.r_y3,
            coord_origin: CoordOrigin::BottomLeft,
        }
    }

    /// Convert coordinates to use top-left origin
    /// Python: `to_top_left_origin()` (line 199)
    ///
    /// Args:
    ///     `page_height`: The height of the page
    ///
    /// Returns:
    ///     `BoundingRectangle` with top-left origin
    #[inline]
    #[must_use = "converted rectangle is returned but not used"]
    pub fn to_top_left_origin(&self, page_height: f64) -> Self {
        if self.coord_origin == CoordOrigin::TopLeft {
            return *self;
        }

        // coord_origin == CoordOrigin::BottomLeft
        Self {
            r_x0: self.r_x0,
            r_x1: self.r_x1,
            r_x2: self.r_x2,
            r_x3: self.r_x3,
            r_y0: page_height - self.r_y0,
            r_y1: page_height - self.r_y1,
            r_y2: page_height - self.r_y2,
            r_y3: page_height - self.r_y3,
            coord_origin: CoordOrigin::TopLeft,
        }
    }
}

// ============================================================================
// TextCell - Text with positioning
// ============================================================================

/// Model representing a text cell with positioning and content information
/// Python: `TextCell` (line 261)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextCell {
    /// Ordering index (from `OrderedElement`)
    pub index: usize,

    /// Bounding rectangle
    pub rect: BoundingRectangle,

    /// Extracted text
    pub text: String,

    /// Original text (before corrections)
    pub orig: String,

    /// Text direction
    #[serde(default = "default_text_direction")]
    pub text_direction: TextDirection,

    /// OCR confidence (1.0 for non-OCR)
    #[serde(default = "default_confidence")]
    pub confidence: f32,

    /// Whether from OCR or native PDF text
    pub from_ocr: bool,

    /// Font size (for header detection)
    #[serde(default = "default_font_size")]
    pub font_size: f32,
}

#[inline]
const fn default_text_direction() -> TextDirection {
    TextDirection::LeftToRight
}

#[inline]
const fn default_confidence() -> f32 {
    1.0
}

#[inline]
const fn default_font_size() -> f32 {
    12.0 // Default font size when not specified
}

impl TextCell {
    /// Convert the cell rectangle to a `BoundingBox`
    #[inline]
    #[must_use = "bounding box is created but not used"]
    pub fn to_bounding_box(&self) -> BoundingBox {
        self.rect.to_bounding_box()
    }

    /// Convert the cell's coordinates to use bottom-left origin
    #[inline]
    pub fn to_bottom_left_origin(&mut self, page_height: f64) {
        self.rect = self.rect.to_bottom_left_origin(page_height);
    }

    /// Convert the cell's coordinates to use top-left origin
    #[inline]
    pub fn to_top_left_origin(&mut self, page_height: f64) {
        self.rect = self.rect.to_top_left_origin(page_height);
    }
}

// ============================================================================
// PageGeometry - Page dimensions
// ============================================================================

/// Model representing dimensions of a page
/// Python: `PageGeometry` (line 502)
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct PageGeometry {
    /// Page rotation angle in degrees
    pub angle: f64,
    /// Page bounding rectangle
    pub rect: BoundingRectangle,
}

impl PageGeometry {
    /// Get the width of the page
    #[inline]
    #[must_use = "page width is computed but not used"]
    pub fn width(&self) -> f64 {
        self.rect.width()
    }

    /// Get the height of the page
    #[inline]
    #[must_use = "page height is computed but not used"]
    pub fn height(&self) -> f64 {
        self.rect.height()
    }
}

// ============================================================================
// PdfPageGeometry - PDF-specific page dimensions
// ============================================================================

/// Extended dimensions model specific to PDF pages with boundary types
/// Python: `PdfPageGeometry` (line 524)
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct PdfPageGeometry {
    /// Page rotation angle in degrees
    pub angle: f64,
    /// Page bounding rectangle
    pub rect: BoundingRectangle,
    /// Boundary type (media box, crop box, etc.)
    pub boundary_type: PdfPageBoundaryType,

    /// Art box
    pub art_bbox: BoundingBox,

    /// Bleed box
    pub bleed_bbox: BoundingBox,

    /// Crop box
    pub crop_bbox: BoundingBox,

    /// Media box
    pub media_bbox: BoundingBox,

    /// Trim box
    pub trim_bbox: BoundingBox,
}

impl PdfPageGeometry {
    /// Get the width of the PDF page based on crop box
    /// Python: width property (line 537)
    #[inline]
    #[must_use = "page width is computed but not used"]
    pub const fn width(&self) -> f64 {
        self.crop_bbox.width()
    }

    /// Get the height of the PDF page based on crop box
    /// Python: height property (line 543)
    #[inline]
    #[must_use = "page height is computed but not used"]
    pub const fn height(&self) -> f64 {
        self.crop_bbox.height()
    }
}

// ============================================================================
// SegmentedPdfPage - Segmented page with text cells
// ============================================================================

/// Extended segmented page model specific to PDF documents
/// Python: `SegmentedPdfPage` (line 718)
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SegmentedPdfPage {
    /// Page dimensions
    pub dimension: PdfPageGeometry,

    /// Text cells at character level
    #[serde(default)]
    pub char_cells: Vec<TextCell>,

    /// Text cells at word level
    #[serde(default)]
    pub word_cells: Vec<TextCell>,

    /// Text cells at line level
    #[serde(default)]
    pub textline_cells: Vec<TextCell>,

    /// Whether `char_cells` are populated
    #[serde(default)]
    pub has_chars: bool,

    /// Whether `word_cells` are populated
    #[serde(default)]
    pub has_words: bool,

    /// Whether `textline_cells` are populated
    #[serde(default)]
    pub has_textlines: bool,
}

impl SegmentedPdfPage {
    /// Create a new `SegmentedPdfPage`
    #[inline]
    #[must_use = "segmented page is created but not used"]
    pub const fn new(
        dimension: PdfPageGeometry,
        textline_cells: Vec<TextCell>,
        char_cells: Vec<TextCell>,
        word_cells: Vec<TextCell>,
    ) -> Self {
        let has_textlines = !textline_cells.is_empty();
        let has_chars = !char_cells.is_empty();
        let has_words = !word_cells.is_empty();

        Self {
            dimension,
            char_cells,
            word_cells,
            textline_cells,
            has_chars,
            has_words,
            has_textlines,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bounding_rectangle_from_bounding_box() {
        let bbox = BoundingBox::new(10.0, 20.0, 100.0, 80.0, CoordOrigin::TopLeft);
        let rect = BoundingRectangle::from_bounding_box(&bbox);

        assert_eq!(rect.r_x0, 10.0); // left
        assert_eq!(rect.r_y0, 80.0); // bottom
        assert_eq!(rect.r_x1, 100.0); // right
        assert_eq!(rect.r_y1, 80.0); // bottom
        assert_eq!(rect.r_x2, 100.0); // right
        assert_eq!(rect.r_y2, 20.0); // top
        assert_eq!(rect.r_x3, 10.0); // left
        assert_eq!(rect.r_y3, 20.0); // top

        // Convert back and check
        let bbox2 = rect.to_bounding_box();
        assert_eq!(bbox2.l, 10.0);
        assert_eq!(bbox2.t, 20.0);
        assert_eq!(bbox2.r, 100.0);
        assert_eq!(bbox2.b, 80.0);
    }

    #[test]
    fn test_coordinate_conversion_bottom_left_to_top_left() {
        // Create a rectangle at bottom-left origin
        let bbox = BoundingBox::new(10.0, 100.0, 50.0, 80.0, CoordOrigin::BottomLeft);
        let rect = BoundingRectangle::from_bounding_box(&bbox);

        // Convert to top-left origin
        let page_height = 200.0;
        let rect_top = rect.to_top_left_origin(page_height);

        // Y coordinates should be flipped: y' = page_height - y
        assert_eq!(rect_top.r_y0, page_height - rect.r_y0); // bottom becomes top
        assert_eq!(rect_top.r_y2, page_height - rect.r_y2); // top becomes bottom
        assert_eq!(rect_top.coord_origin, CoordOrigin::TopLeft);

        // X coordinates should remain unchanged
        assert_eq!(rect_top.r_x0, rect.r_x0);
        assert_eq!(rect_top.r_x1, rect.r_x1);
    }

    #[test]
    fn test_coordinate_conversion_top_left_to_bottom_left() {
        // Create a rectangle at top-left origin
        let bbox = BoundingBox::new(10.0, 20.0, 50.0, 80.0, CoordOrigin::TopLeft);
        let rect = BoundingRectangle::from_bounding_box(&bbox);

        // Convert to bottom-left origin
        let page_height = 200.0;
        let rect_bottom = rect.to_bottom_left_origin(page_height);

        // Y coordinates should be flipped: y' = page_height - y
        assert_eq!(rect_bottom.r_y0, page_height - rect.r_y0);
        assert_eq!(rect_bottom.r_y2, page_height - rect.r_y2);
        assert_eq!(rect_bottom.coord_origin, CoordOrigin::BottomLeft);

        // X coordinates should remain unchanged
        assert_eq!(rect_bottom.r_x0, rect.r_x0);
        assert_eq!(rect_bottom.r_x1, rect.r_x1);
    }

    #[test]
    fn test_coordinate_conversion_round_trip() {
        let bbox = BoundingBox::new(10.0, 20.0, 100.0, 80.0, CoordOrigin::BottomLeft);
        let rect = BoundingRectangle::from_bounding_box(&bbox);
        let page_height = 200.0;

        // Convert to top-left and back to bottom-left
        let rect2 = rect
            .to_top_left_origin(page_height)
            .to_bottom_left_origin(page_height);

        // Should match original
        assert_eq!(rect.r_x0, rect2.r_x0);
        assert_eq!(rect.r_y0, rect2.r_y0);
        assert_eq!(rect.r_x1, rect2.r_x1);
        assert_eq!(rect.r_y1, rect2.r_y1);
        assert_eq!(rect.r_x2, rect2.r_x2);
        assert_eq!(rect.r_y2, rect2.r_y2);
        assert_eq!(rect.r_x3, rect2.r_x3);
        assert_eq!(rect.r_y3, rect2.r_y3);
        assert_eq!(rect.coord_origin, rect2.coord_origin);
    }

    #[test]
    fn test_text_cell_creation() {
        let bbox = BoundingBox::new(10.0, 20.0, 100.0, 80.0, CoordOrigin::TopLeft);
        let rect = BoundingRectangle::from_bounding_box(&bbox);

        let cell = TextCell {
            index: 0,
            rect,
            text: "Hello World".to_string(),
            orig: "Hello World".to_string(),
            text_direction: TextDirection::LeftToRight,
            confidence: 1.0,
            from_ocr: false,
            font_size: 12.0,
        };

        assert_eq!(cell.text, "Hello World");
        assert_eq!(cell.confidence, 1.0);
        assert!(!cell.from_ocr);

        let bbox2 = cell.to_bounding_box();
        assert_eq!(bbox2.l, 10.0);
        assert_eq!(bbox2.t, 20.0);
    }

    #[test]
    fn test_pdf_page_boundary_type_display() {
        assert_eq!(PdfPageBoundaryType::ArtBox.to_string(), "art_box");
        assert_eq!(PdfPageBoundaryType::BleedBox.to_string(), "bleed_box");
        assert_eq!(PdfPageBoundaryType::CropBox.to_string(), "crop_box");
        assert_eq!(PdfPageBoundaryType::MediaBox.to_string(), "media_box");
        assert_eq!(PdfPageBoundaryType::TrimBox.to_string(), "trim_box");
    }

    #[test]
    fn test_text_direction_display() {
        assert_eq!(TextDirection::LeftToRight.to_string(), "left_to_right");
        assert_eq!(TextDirection::RightToLeft.to_string(), "right_to_left");
        assert_eq!(TextDirection::Unspecified.to_string(), "unspecified");
    }

    #[test]
    fn test_pdf_page_boundary_type_from_str() {
        use std::str::FromStr;

        // Standard formats
        assert_eq!(
            PdfPageBoundaryType::from_str("art_box").unwrap(),
            PdfPageBoundaryType::ArtBox
        );
        assert_eq!(
            PdfPageBoundaryType::from_str("bleed_box").unwrap(),
            PdfPageBoundaryType::BleedBox
        );
        assert_eq!(
            PdfPageBoundaryType::from_str("crop_box").unwrap(),
            PdfPageBoundaryType::CropBox
        );
        assert_eq!(
            PdfPageBoundaryType::from_str("media_box").unwrap(),
            PdfPageBoundaryType::MediaBox
        );
        assert_eq!(
            PdfPageBoundaryType::from_str("trim_box").unwrap(),
            PdfPageBoundaryType::TrimBox
        );

        // Alternative formats
        assert_eq!(
            PdfPageBoundaryType::from_str("artbox").unwrap(),
            PdfPageBoundaryType::ArtBox
        );
        assert_eq!(
            PdfPageBoundaryType::from_str("media-box").unwrap(),
            PdfPageBoundaryType::MediaBox
        );

        // Case insensitive
        assert_eq!(
            PdfPageBoundaryType::from_str("MEDIA_BOX").unwrap(),
            PdfPageBoundaryType::MediaBox
        );

        // Invalid
        assert!(PdfPageBoundaryType::from_str("invalid").is_err());
    }

    #[test]
    fn test_pdf_page_boundary_type_roundtrip() {
        use std::str::FromStr;

        for boundary in [
            PdfPageBoundaryType::ArtBox,
            PdfPageBoundaryType::BleedBox,
            PdfPageBoundaryType::CropBox,
            PdfPageBoundaryType::MediaBox,
            PdfPageBoundaryType::TrimBox,
        ] {
            let s = boundary.to_string();
            let parsed = PdfPageBoundaryType::from_str(&s).unwrap();
            assert_eq!(boundary, parsed);
        }
    }

    #[test]
    fn test_text_direction_from_str() {
        use std::str::FromStr;

        // Standard formats
        assert_eq!(
            TextDirection::from_str("left_to_right").unwrap(),
            TextDirection::LeftToRight
        );
        assert_eq!(
            TextDirection::from_str("right_to_left").unwrap(),
            TextDirection::RightToLeft
        );
        assert_eq!(
            TextDirection::from_str("unspecified").unwrap(),
            TextDirection::Unspecified
        );

        // Abbreviations
        assert_eq!(
            TextDirection::from_str("ltr").unwrap(),
            TextDirection::LeftToRight
        );
        assert_eq!(
            TextDirection::from_str("rtl").unwrap(),
            TextDirection::RightToLeft
        );

        // Alternative formats
        assert_eq!(
            TextDirection::from_str("lefttoright").unwrap(),
            TextDirection::LeftToRight
        );
        assert_eq!(
            TextDirection::from_str("left-to-right").unwrap(),
            TextDirection::LeftToRight
        );

        // Empty string defaults to Unspecified
        assert_eq!(
            TextDirection::from_str("").unwrap(),
            TextDirection::Unspecified
        );

        // Invalid
        assert!(TextDirection::from_str("invalid").is_err());
    }

    #[test]
    fn test_text_direction_roundtrip() {
        use std::str::FromStr;

        for direction in [
            TextDirection::LeftToRight,
            TextDirection::RightToLeft,
            TextDirection::Unspecified,
        ] {
            let s = direction.to_string();
            let parsed = TextDirection::from_str(&s).unwrap();
            assert_eq!(direction, parsed);
        }
    }

    #[test]
    fn test_segmented_pdf_page_default() {
        let page = SegmentedPdfPage::default();

        // Default should have empty cell vectors
        assert!(page.char_cells.is_empty());
        assert!(page.word_cells.is_empty());
        assert!(page.textline_cells.is_empty());

        // Default should have false flags
        assert!(!page.has_chars);
        assert!(!page.has_words);
        assert!(!page.has_textlines);

        // Default dimension should match PdfPageGeometry default
        assert_eq!(page.dimension, PdfPageGeometry::default());
    }
}
