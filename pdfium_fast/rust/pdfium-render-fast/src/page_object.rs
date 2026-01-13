//! Page objects (images, text, paths) from PDF pages.
//!
//! This module provides access to the individual graphical objects that
//! make up a PDF page, including images, text, paths, and shading.
//!
//! # Example
//!
//! ```no_run
//! use pdfium_render_fast::{Pdfium, PdfPageObjectType};
//!
//! let pdfium = Pdfium::new()?;
//! let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
//! let page = doc.page(0)?;
//!
//! // Iterate over all objects on the page
//! let objects = page.objects();
//! println!("Page has {} objects", objects.count());
//!
//! for obj in objects.iter() {
//!     match obj.object_type() {
//!         PdfPageObjectType::Image => println!("Image at {:?}", obj.bounds()),
//!         PdfPageObjectType::Text => println!("Text at {:?}", obj.bounds()),
//!         PdfPageObjectType::Path => println!("Path at {:?}", obj.bounds()),
//!         _ => {}
//!     }
//! }
//! # Ok::<(), pdfium_render_fast::PdfError>(())
//! ```

use crate::render::PdfBitmap;
use crate::text::PdfPageText;
use pdfium_sys::{
    FPDFFont_GetAscent, FPDFFont_GetBaseFontName, FPDFFont_GetDescent, FPDFFont_GetFamilyName,
    FPDFFont_GetFlags, FPDFFont_GetGlyphWidth, FPDFFont_GetIsEmbedded, FPDFFont_GetItalicAngle,
    FPDFFont_GetWeight, FPDFImageObj_GetBitmap, FPDFImageObj_GetImageDataDecoded,
    FPDFImageObj_GetImageDataRaw, FPDFImageObj_GetImageFilter, FPDFImageObj_GetImageFilterCount,
    FPDFImageObj_GetImageMetadata, FPDFImageObj_GetImagePixelSize, FPDFImageObj_GetRenderedBitmap,
    FPDFPageObjMark_GetName, FPDFPageObjMark_GetParamStringValue, FPDFPageObj_CountMarks,
    FPDFPageObj_GetBounds, FPDFPageObj_GetFillColor, FPDFPageObj_GetMark, FPDFPageObj_GetMatrix,
    FPDFPageObj_GetStrokeColor, FPDFPageObj_GetStrokeWidth, FPDFPageObj_GetType,
    FPDFPageObj_HasTransparency, FPDFPage_CountObjects, FPDFPage_GetObject,
    FPDFPathSegment_GetClose, FPDFPathSegment_GetPoint, FPDFPathSegment_GetType,
    FPDFPath_CountSegments, FPDFPath_GetDrawMode, FPDFPath_GetPathSegment, FPDFTextObj_GetFont,
    FPDFTextObj_GetFontSize, FPDFTextObj_GetText, FPDFTextObj_GetTextRenderMode, FPDF_DOCUMENT,
    FPDF_FONT, FPDF_IMAGEOBJ_METADATA, FPDF_PAGE, FPDF_PAGEOBJECT, FPDF_PATHSEGMENT, FS_MATRIX,
};

/// Type of PDF artifact (decorative content not part of document structure).
///
/// Artifacts are page elements that are not part of the document's logical content,
/// such as headers, footers, page numbers, watermarks, and decorative elements.
/// They are typically excluded from accessibility processing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArtifactType {
    /// Background artifact (decorative backgrounds)
    Background,
    /// Footer artifact (page footer content)
    Footer,
    /// Header artifact (page header content)
    Header,
    /// Layout artifact (decorative lines, borders)
    Layout,
    /// Page artifact (page numbers)
    Page,
    /// Pagination artifact (running headers/footers)
    Pagination,
    /// Watermark artifact
    Watermark,
    /// Other or unspecified artifact type
    Other,
}

impl ArtifactType {
    /// Parse artifact type from string.
    fn from_type_string(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "background" => ArtifactType::Background,
            "footer" => ArtifactType::Footer,
            "header" => ArtifactType::Header,
            "layout" => ArtifactType::Layout,
            "page" => ArtifactType::Page,
            "pagination" => ArtifactType::Pagination,
            "watermark" => ArtifactType::Watermark,
            _ => ArtifactType::Other,
        }
    }
}

/// Type of a page object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PdfPageObjectType {
    /// Unknown object type
    Unknown,
    /// Text object (characters rendered with a font)
    Text,
    /// Path object (lines, curves, shapes)
    Path,
    /// Image object (raster graphics)
    Image,
    /// Shading object (gradient fills)
    Shading,
    /// Form XObject (reusable content)
    Form,
}

impl From<i32> for PdfPageObjectType {
    fn from(value: i32) -> Self {
        match value {
            1 => PdfPageObjectType::Text,
            2 => PdfPageObjectType::Path,
            3 => PdfPageObjectType::Image,
            4 => PdfPageObjectType::Shading,
            5 => PdfPageObjectType::Form,
            _ => PdfPageObjectType::Unknown,
        }
    }
}

/// RGBA color with 8-bit components.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjectColor {
    /// Red component (0-255)
    pub r: u8,
    /// Green component (0-255)
    pub g: u8,
    /// Blue component (0-255)
    pub b: u8,
    /// Alpha component (0-255, 255 = opaque)
    pub a: u8,
}

impl ObjectColor {
    /// Create a new color from RGBA components.
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Create an opaque color from RGB components.
    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// Convert to a hex string (e.g., "#FF5733").
    pub fn to_hex(&self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }

    /// Convert to a hex string with alpha (e.g., "#FF5733CC").
    pub fn to_hex_with_alpha(&self) -> String {
        format!("#{:02X}{:02X}{:02X}{:02X}", self.r, self.g, self.b, self.a)
    }

    /// Check if the color is fully transparent.
    pub fn is_transparent(&self) -> bool {
        self.a == 0
    }

    /// Check if the color is fully opaque.
    pub fn is_opaque(&self) -> bool {
        self.a == 255
    }
}

/// Text rendering mode for text objects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextRenderMode {
    /// Fill text (default)
    Fill,
    /// Stroke text outline
    Stroke,
    /// Fill then stroke
    FillStroke,
    /// Invisible text (for selection/search)
    Invisible,
    /// Fill text and add to clipping path
    FillClip,
    /// Stroke text and add to clipping path
    StrokeClip,
    /// Fill, stroke, and clip
    FillStrokeClip,
    /// Add to clipping path only
    Clip,
    /// Unknown render mode
    Unknown,
}

impl From<i32> for TextRenderMode {
    fn from(value: i32) -> Self {
        match value {
            0 => TextRenderMode::Fill,
            1 => TextRenderMode::Stroke,
            2 => TextRenderMode::FillStroke,
            3 => TextRenderMode::Invisible,
            4 => TextRenderMode::FillClip,
            5 => TextRenderMode::StrokeClip,
            6 => TextRenderMode::FillStrokeClip,
            7 => TextRenderMode::Clip,
            _ => TextRenderMode::Unknown,
        }
    }
}

/// 2D transformation matrix.
///
/// The matrix represents the transformation:
/// ```text
/// | a  b  0 |
/// | c  d  0 |
/// | e  f  1 |
/// ```
#[derive(Debug, Clone, Copy)]
pub struct ObjectMatrix {
    /// Scale X
    pub a: f32,
    /// Skew Y
    pub b: f32,
    /// Skew X
    pub c: f32,
    /// Scale Y
    pub d: f32,
    /// Translate X
    pub e: f32,
    /// Translate Y
    pub f: f32,
}

impl ObjectMatrix {
    /// Create an identity matrix.
    pub fn identity() -> Self {
        Self {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: 0.0,
            f: 0.0,
        }
    }

    /// Get the X scale factor.
    pub fn scale_x(&self) -> f32 {
        (self.a * self.a + self.b * self.b).sqrt()
    }

    /// Get the Y scale factor.
    pub fn scale_y(&self) -> f32 {
        (self.c * self.c + self.d * self.d).sqrt()
    }

    /// Get the rotation angle in radians.
    pub fn rotation(&self) -> f32 {
        self.b.atan2(self.a)
    }

    /// Get the translation offset.
    pub fn translation(&self) -> (f32, f32) {
        (self.e, self.f)
    }
}

// ==================== EXTRACTED LINE ====================

/// A line extracted from PDF path objects.
///
/// This represents a simple line segment (straight path between two points)
/// useful for detecting separators, table borders, and decorative elements.
///
/// # Example
///
/// ```no_run
/// use pdfium_render_fast::Pdfium;
///
/// let pdfium = Pdfium::new()?;
/// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
/// let page = doc.page(0)?;
///
/// for line in page.extract_lines() {
///     if line.is_horizontal {
///         println!("Horizontal line at y={:.1}", line.start.1);
///     }
/// }
/// # Ok::<(), pdfium_render_fast::PdfError>(())
/// ```
#[derive(Debug, Clone, Copy)]
pub struct ExtractedLine {
    /// Start point (x, y) in page coordinates
    pub start: (f32, f32),
    /// End point (x, y) in page coordinates
    pub end: (f32, f32),
    /// Line thickness (stroke width) in points
    pub thickness: f32,
    /// Line color as RGBA tuple
    pub color: (u8, u8, u8, u8),
    /// Whether this line is primarily horizontal
    pub is_horizontal: bool,
    /// Whether this line is primarily vertical
    pub is_vertical: bool,
}

impl ExtractedLine {
    /// Create a new extracted line from path properties.
    pub fn new(
        start: (f32, f32),
        end: (f32, f32),
        thickness: f32,
        color: (u8, u8, u8, u8),
    ) -> Self {
        let dx = (end.0 - start.0).abs();
        let dy = (end.1 - start.1).abs();

        // A line is horizontal if dx >> dy (threshold: 5x)
        // A line is vertical if dy >> dx (threshold: 5x)
        let is_horizontal = dy < 1.0 || (dx > 0.0 && dx / dy > 5.0);
        let is_vertical = dx < 1.0 || (dy > 0.0 && dy / dx > 5.0);

        Self {
            start,
            end,
            thickness,
            color,
            is_horizontal,
            is_vertical,
        }
    }

    /// Get the length of the line.
    pub fn length(&self) -> f32 {
        let dx = self.end.0 - self.start.0;
        let dy = self.end.1 - self.start.1;
        (dx * dx + dy * dy).sqrt()
    }

    /// Get the midpoint of the line.
    pub fn midpoint(&self) -> (f32, f32) {
        (
            (self.start.0 + self.end.0) / 2.0,
            (self.start.1 + self.end.1) / 2.0,
        )
    }

    /// Get the angle of the line in radians (0 = horizontal, π/2 = vertical).
    pub fn angle(&self) -> f32 {
        let dx = self.end.0 - self.start.0;
        let dy = self.end.1 - self.start.1;
        dy.atan2(dx)
    }

    /// Get the angle in degrees.
    pub fn angle_degrees(&self) -> f32 {
        self.angle().to_degrees()
    }

    /// Check if the line is diagonal (neither horizontal nor vertical).
    pub fn is_diagonal(&self) -> bool {
        !self.is_horizontal && !self.is_vertical
    }

    /// Get bounding box as (left, bottom, right, top).
    pub fn bounds(&self) -> (f32, f32, f32, f32) {
        let left = self.start.0.min(self.end.0);
        let right = self.start.0.max(self.end.0);
        let bottom = self.start.1.min(self.end.1);
        let top = self.start.1.max(self.end.1);
        (left, bottom, right, top)
    }

    /// Check if the line has a visible color (not fully transparent).
    pub fn is_visible(&self) -> bool {
        self.color.3 > 0
    }

    /// Get the y-coordinate for a horizontal line.
    /// Returns None if the line is not horizontal.
    pub fn y_position(&self) -> Option<f32> {
        if self.is_horizontal {
            Some((self.start.1 + self.end.1) / 2.0)
        } else {
            None
        }
    }

    /// Get the x-coordinate for a vertical line.
    /// Returns None if the line is not vertical.
    pub fn x_position(&self) -> Option<f32> {
        if self.is_vertical {
            Some((self.start.0 + self.end.0) / 2.0)
        } else {
            None
        }
    }
}

// ==================== COLORED REGION ====================

/// A colored region (filled path) extracted from a PDF page.
///
/// Colored regions represent filled shapes that may serve as backgrounds,
/// highlights, or decorative elements. They are useful for detecting:
/// - Page backgrounds
/// - Table cell backgrounds
/// - Highlighted text areas
/// - Decorative boxes
///
/// # Example
///
/// ```no_run
/// use pdfium_render_fast::Pdfium;
///
/// let pdfium = Pdfium::new()?;
/// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
/// let page = doc.page(0)?;
///
/// for region in page.extract_colored_regions() {
///     if let Some(fill) = region.fill_color {
///         println!("Region at {:?} with fill color RGBA({}, {}, {}, {})",
///             region.bounds, fill.0, fill.1, fill.2, fill.3);
///     }
/// }
/// # Ok::<(), pdfium_render_fast::PdfError>(())
/// ```
#[derive(Debug, Clone, Copy)]
pub struct ColoredRegion {
    /// Bounding box as (left, bottom, right, top) in page coordinates
    pub bounds: (f32, f32, f32, f32),
    /// Fill color as RGBA tuple, None if not filled
    pub fill_color: Option<(u8, u8, u8, u8)>,
    /// Stroke color as RGBA tuple, None if not stroked
    pub stroke_color: Option<(u8, u8, u8, u8)>,
    /// Whether this region appears behind text objects in z-order
    pub is_behind_text: bool,
}

impl ColoredRegion {
    /// Create a new colored region.
    pub fn new(
        bounds: (f32, f32, f32, f32),
        fill_color: Option<(u8, u8, u8, u8)>,
        stroke_color: Option<(u8, u8, u8, u8)>,
        is_behind_text: bool,
    ) -> Self {
        Self {
            bounds,
            fill_color,
            stroke_color,
            is_behind_text,
        }
    }

    /// Get the width of the region.
    pub fn width(&self) -> f32 {
        (self.bounds.2 - self.bounds.0).abs()
    }

    /// Get the height of the region.
    pub fn height(&self) -> f32 {
        (self.bounds.3 - self.bounds.1).abs()
    }

    /// Get the area of the region in square points.
    pub fn area(&self) -> f32 {
        self.width() * self.height()
    }

    /// Get the center point of the region.
    pub fn center(&self) -> (f32, f32) {
        (
            (self.bounds.0 + self.bounds.2) / 2.0,
            (self.bounds.1 + self.bounds.3) / 2.0,
        )
    }

    /// Check if this region has a fill color.
    pub fn is_filled(&self) -> bool {
        self.fill_color.is_some()
    }

    /// Check if this region has a stroke color.
    pub fn is_stroked(&self) -> bool {
        self.stroke_color.is_some()
    }

    /// Check if this region has any visible color (fill or stroke).
    pub fn is_visible(&self) -> bool {
        // A region is visible if it has a non-transparent fill or stroke
        if let Some((_, _, _, a)) = self.fill_color {
            if a > 0 {
                return true;
            }
        }
        if let Some((_, _, _, a)) = self.stroke_color {
            if a > 0 {
                return true;
            }
        }
        false
    }

    /// Check if the fill color is white or near-white.
    pub fn is_white_fill(&self) -> bool {
        if let Some((r, g, b, _)) = self.fill_color {
            r >= 250 && g >= 250 && b >= 250
        } else {
            false
        }
    }

    /// Check if the fill color is a light color (high luminance).
    pub fn is_light_fill(&self) -> bool {
        if let Some((r, g, b, _)) = self.fill_color {
            // Simple luminance calculation
            let luminance = 0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32;
            luminance > 180.0
        } else {
            false
        }
    }

    /// Check if the fill color is a dark color (low luminance).
    pub fn is_dark_fill(&self) -> bool {
        if let Some((r, g, b, _)) = self.fill_color {
            let luminance = 0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32;
            luminance < 75.0
        } else {
            false
        }
    }

    /// Check if this region contains a given point.
    pub fn contains_point(&self, x: f32, y: f32) -> bool {
        x >= self.bounds.0 && x <= self.bounds.2 && y >= self.bounds.1 && y <= self.bounds.3
    }

    /// Check if this region overlaps with another region.
    pub fn overlaps(&self, other: &ColoredRegion) -> bool {
        // No overlap if one is completely to the left, right, above, or below the other
        !(self.bounds.2 < other.bounds.0
            || other.bounds.2 < self.bounds.0
            || self.bounds.3 < other.bounds.1
            || other.bounds.3 < self.bounds.1)
    }

    /// Check if this region fully contains another region.
    pub fn contains(&self, other: &ColoredRegion) -> bool {
        self.bounds.0 <= other.bounds.0
            && self.bounds.2 >= other.bounds.2
            && self.bounds.1 <= other.bounds.1
            && self.bounds.3 >= other.bounds.3
    }

    /// Get the aspect ratio (width / height).
    pub fn aspect_ratio(&self) -> f32 {
        let h = self.height();
        if h > 0.0 {
            self.width() / h
        } else {
            0.0
        }
    }

    /// Check if this region appears to be a horizontal stripe (width >> height).
    pub fn is_horizontal_stripe(&self) -> bool {
        self.aspect_ratio() > 5.0
    }

    /// Check if this region appears to be a vertical stripe (height >> width).
    pub fn is_vertical_stripe(&self) -> bool {
        let h = self.height();
        let w = self.width();
        w > 0.0 && h / w > 5.0
    }
}

// ==================== PATH SEGMENT TYPES ====================

/// Type of a path segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PathSegmentType {
    /// Unknown segment type
    Unknown,
    /// Move to a new starting point
    MoveTo,
    /// Draw a straight line to a point
    LineTo,
    /// Draw a Bezier curve to a point
    BezierTo,
}

impl From<i32> for PathSegmentType {
    fn from(value: i32) -> Self {
        match value {
            0 => PathSegmentType::LineTo,
            1 => PathSegmentType::BezierTo,
            2 => PathSegmentType::MoveTo,
            _ => PathSegmentType::Unknown,
        }
    }
}

/// A path segment (line, curve, or move command).
pub struct PathSegment {
    handle: FPDF_PATHSEGMENT,
}

impl PathSegment {
    fn new(handle: FPDF_PATHSEGMENT) -> Option<Self> {
        if handle.is_null() {
            None
        } else {
            Some(Self { handle })
        }
    }

    /// Get the point coordinates of this segment.
    pub fn point(&self) -> Option<(f32, f32)> {
        let mut x = 0.0f32;
        let mut y = 0.0f32;
        let ok = unsafe { FPDFPathSegment_GetPoint(self.handle, &mut x, &mut y) };
        if ok != 0 {
            Some((x, y))
        } else {
            None
        }
    }

    /// Get the type of this segment.
    pub fn segment_type(&self) -> PathSegmentType {
        let seg_type = unsafe { FPDFPathSegment_GetType(self.handle) };
        PathSegmentType::from(seg_type)
    }

    /// Check if this segment closes the current subpath.
    pub fn closes_path(&self) -> bool {
        unsafe { FPDFPathSegment_GetClose(self.handle) != 0 }
    }
}

/// Path drawing mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PathDrawMode {
    /// Fill mode: None, Alternate (even-odd), or Winding (non-zero)
    pub fill_mode: PathFillMode,
    /// Whether to stroke the path
    pub stroke: bool,
}

/// Path fill mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PathFillMode {
    /// No fill
    None,
    /// Alternate fill rule (even-odd)
    Alternate,
    /// Winding fill rule (non-zero)
    Winding,
}

// ==================== IMAGE METADATA ====================

/// Colorspace of an image.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ImageColorspace {
    /// Unknown colorspace
    Unknown,
    /// Device-dependent gray
    DeviceGray,
    /// Device-dependent RGB
    DeviceRGB,
    /// Device-dependent CMYK
    DeviceCMYK,
    /// Calibrated gray
    CalGray,
    /// Calibrated RGB
    CalRGB,
    /// Lab colorspace
    Lab,
    /// ICC-based colorspace
    ICCBased,
    /// Separation colorspace
    Separation,
    /// DeviceN colorspace
    DeviceN,
    /// Indexed colorspace
    Indexed,
    /// Pattern colorspace
    Pattern,
}

impl From<i32> for ImageColorspace {
    fn from(value: i32) -> Self {
        match value {
            1 => ImageColorspace::DeviceGray,
            2 => ImageColorspace::DeviceRGB,
            3 => ImageColorspace::DeviceCMYK,
            4 => ImageColorspace::CalGray,
            5 => ImageColorspace::CalRGB,
            6 => ImageColorspace::Lab,
            7 => ImageColorspace::ICCBased,
            8 => ImageColorspace::Separation,
            9 => ImageColorspace::DeviceN,
            10 => ImageColorspace::Indexed,
            11 => ImageColorspace::Pattern,
            _ => ImageColorspace::Unknown,
        }
    }
}

/// Metadata about an image object.
#[derive(Debug, Clone, Copy)]
pub struct ImageMetadata {
    /// Image width in pixels
    pub width: u32,
    /// Image height in pixels
    pub height: u32,
    /// Horizontal DPI
    pub horizontal_dpi: f32,
    /// Vertical DPI
    pub vertical_dpi: f32,
    /// Bits per pixel
    pub bits_per_pixel: u32,
    /// Image colorspace
    pub colorspace: ImageColorspace,
    /// Marked content ID (-1 if none)
    pub marked_content_id: i32,
}

// ==================== IMAGE TECHNICAL METADATA ====================

/// PDF image compression filter.
///
/// Filters specify how image data is encoded in the PDF. Multiple filters
/// can be applied in sequence (e.g., DCTDecode + FlateDecode).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ImageFilter {
    /// ASCII hexadecimal encoding
    ASCIIHexDecode,
    /// ASCII base-85 encoding
    ASCII85Decode,
    /// LZW compression
    LZWDecode,
    /// Zlib/deflate compression (most common)
    FlateDecode,
    /// Run-length encoding
    RunLengthDecode,
    /// CCITT Group 3/4 fax encoding (for bilevel images)
    CCITTFaxDecode,
    /// JBIG2 compression (for bilevel images)
    JBIG2Decode,
    /// DCT (JPEG) compression
    DCTDecode,
    /// JPEG2000 compression
    JPXDecode,
    /// Crypt filter (encryption)
    Crypt,
    /// Unknown or custom filter
    Unknown(String),
}

impl ImageFilter {
    /// Parse filter from its PDF name.
    pub fn from_name(name: &str) -> Self {
        match name {
            "ASCIIHexDecode" | "AHx" => ImageFilter::ASCIIHexDecode,
            "ASCII85Decode" | "A85" => ImageFilter::ASCII85Decode,
            "LZWDecode" | "LZW" => ImageFilter::LZWDecode,
            "FlateDecode" | "Fl" => ImageFilter::FlateDecode,
            "RunLengthDecode" | "RL" => ImageFilter::RunLengthDecode,
            "CCITTFaxDecode" | "CCF" => ImageFilter::CCITTFaxDecode,
            "JBIG2Decode" => ImageFilter::JBIG2Decode,
            "DCTDecode" | "DCT" => ImageFilter::DCTDecode,
            "JPXDecode" => ImageFilter::JPXDecode,
            "Crypt" => ImageFilter::Crypt,
            _ => ImageFilter::Unknown(name.to_string()),
        }
    }

    /// Get the PDF name of this filter.
    pub fn name(&self) -> &str {
        match self {
            ImageFilter::ASCIIHexDecode => "ASCIIHexDecode",
            ImageFilter::ASCII85Decode => "ASCII85Decode",
            ImageFilter::LZWDecode => "LZWDecode",
            ImageFilter::FlateDecode => "FlateDecode",
            ImageFilter::RunLengthDecode => "RunLengthDecode",
            ImageFilter::CCITTFaxDecode => "CCITTFaxDecode",
            ImageFilter::JBIG2Decode => "JBIG2Decode",
            ImageFilter::DCTDecode => "DCTDecode",
            ImageFilter::JPXDecode => "JPXDecode",
            ImageFilter::Crypt => "Crypt",
            ImageFilter::Unknown(name) => name,
        }
    }

    /// Check if this is a lossy compression filter.
    pub fn is_lossy(&self) -> bool {
        matches!(self, ImageFilter::DCTDecode | ImageFilter::JPXDecode)
    }

    /// Check if this is specifically for bilevel (black/white) images.
    pub fn is_bilevel(&self) -> bool {
        matches!(self, ImageFilter::CCITTFaxDecode | ImageFilter::JBIG2Decode)
    }
}

/// Technical metadata about an image object.
///
/// This provides detailed information about an image's encoding and properties
/// for use in document analysis and classification.
#[derive(Debug, Clone)]
pub struct ImageTechMeta {
    /// Image width in pixels
    pub width_px: u32,
    /// Image height in pixels
    pub height_px: u32,
    /// DPI as (horizontal, vertical)
    pub dpi: (f32, f32),
    /// Bits per color component (typically 1, 2, 4, or 8)
    pub bits_per_component: u8,
    /// Image color space
    pub color_space: ImageColorspace,
    /// Compression filters applied to the image data
    pub filters: Vec<ImageFilter>,
    /// Whether the image has an explicit image mask
    pub has_mask: bool,
    /// Whether the image has a soft (alpha) mask
    pub has_soft_mask: bool,
}

impl ImageTechMeta {
    /// Check if this is a JPEG-compressed image.
    pub fn is_jpeg(&self) -> bool {
        self.filters.contains(&ImageFilter::DCTDecode)
    }

    /// Check if this is a JPEG2000-compressed image.
    pub fn is_jpeg2000(&self) -> bool {
        self.filters.contains(&ImageFilter::JPXDecode)
    }

    /// Check if this is a bilevel (black/white) image.
    pub fn is_bilevel(&self) -> bool {
        self.bits_per_component == 1
    }

    /// Check if this is a grayscale image.
    pub fn is_grayscale(&self) -> bool {
        self.color_space == ImageColorspace::DeviceGray
            || self.color_space == ImageColorspace::CalGray
    }

    /// Check if this is a color image.
    pub fn is_color(&self) -> bool {
        matches!(
            self.color_space,
            ImageColorspace::DeviceRGB
                | ImageColorspace::DeviceCMYK
                | ImageColorspace::CalRGB
                | ImageColorspace::Lab
        )
    }

    /// Get the number of color components based on color space.
    pub fn components(&self) -> u8 {
        match self.color_space {
            ImageColorspace::DeviceGray | ImageColorspace::CalGray => 1,
            ImageColorspace::DeviceRGB | ImageColorspace::CalRGB | ImageColorspace::Lab => 3,
            ImageColorspace::DeviceCMYK => 4,
            ImageColorspace::Indexed => 1, // Palette index
            _ => 0,                        // Unknown
        }
    }

    /// Calculate total bits per pixel.
    pub fn bits_per_pixel(&self) -> u32 {
        self.bits_per_component as u32 * self.components() as u32
    }

    /// Estimate uncompressed image size in bytes.
    pub fn uncompressed_size(&self) -> usize {
        let bits =
            self.width_px as usize * self.height_px as usize * self.bits_per_pixel() as usize;
        bits.div_ceil(8)
    }

    /// Check if the image has any mask (explicit or soft).
    pub fn has_any_mask(&self) -> bool {
        self.has_mask || self.has_soft_mask
    }
}

// ==================== FONT INFORMATION ====================

/// PDF font flags (bit flags from PDF spec).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FontFlags(pub i32);

impl FontFlags {
    /// Fixed-width font
    pub fn is_fixed_pitch(&self) -> bool {
        self.0 & (1 << 0) != 0
    }

    /// Serif font
    pub fn is_serif(&self) -> bool {
        self.0 & (1 << 1) != 0
    }

    /// Symbolic font (non-standard encoding)
    pub fn is_symbolic(&self) -> bool {
        self.0 & (1 << 2) != 0
    }

    /// Script/cursive font
    pub fn is_script(&self) -> bool {
        self.0 & (1 << 3) != 0
    }

    /// Non-symbolic font (standard encoding)
    pub fn is_nonsymbolic(&self) -> bool {
        self.0 & (1 << 5) != 0
    }

    /// Italic font
    pub fn is_italic(&self) -> bool {
        self.0 & (1 << 6) != 0
    }

    /// All-caps font
    pub fn is_all_cap(&self) -> bool {
        self.0 & (1 << 16) != 0
    }

    /// Small-caps font
    pub fn is_small_cap(&self) -> bool {
        self.0 & (1 << 17) != 0
    }

    /// Force bold rendering
    pub fn is_force_bold(&self) -> bool {
        self.0 & (1 << 18) != 0
    }
}

/// A PDF font with metadata.
pub struct PdfFont {
    handle: FPDF_FONT,
}

impl PdfFont {
    /// Create a PdfFont from a raw handle.
    ///
    /// Returns None if the handle is null.
    pub fn from_handle(handle: FPDF_FONT) -> Option<Self> {
        if handle.is_null() {
            None
        } else {
            Some(Self { handle })
        }
    }

    /// Get the raw font handle.
    pub fn handle(&self) -> FPDF_FONT {
        self.handle
    }

    /// Get the base font name (e.g., "Helvetica", "Times-Roman").
    pub fn base_name(&self) -> Option<String> {
        // First call to get buffer size
        let len = unsafe { FPDFFont_GetBaseFontName(self.handle, std::ptr::null_mut(), 0) };
        if len == 0 {
            return None;
        }

        let mut buffer = vec![0u8; len];
        let actual_len =
            unsafe { FPDFFont_GetBaseFontName(self.handle, buffer.as_mut_ptr() as *mut i8, len) };

        if actual_len == 0 {
            return None;
        }

        // Remove trailing null
        let str_len = if buffer.last() == Some(&0) {
            buffer.len() - 1
        } else {
            buffer.len()
        };

        String::from_utf8(buffer[..str_len].to_vec()).ok()
    }

    /// Get the font family name (e.g., "Helvetica", "Times").
    pub fn family_name(&self) -> Option<String> {
        // First call to get buffer size
        let len = unsafe { FPDFFont_GetFamilyName(self.handle, std::ptr::null_mut(), 0) };
        if len == 0 {
            return None;
        }

        let mut buffer = vec![0u8; len];
        let actual_len =
            unsafe { FPDFFont_GetFamilyName(self.handle, buffer.as_mut_ptr() as *mut i8, len) };

        if actual_len == 0 {
            return None;
        }

        // Remove trailing null
        let str_len = if buffer.last() == Some(&0) {
            buffer.len() - 1
        } else {
            buffer.len()
        };

        String::from_utf8(buffer[..str_len].to_vec()).ok()
    }

    /// Check if the font is embedded in the PDF.
    pub fn is_embedded(&self) -> bool {
        unsafe { FPDFFont_GetIsEmbedded(self.handle) != 0 }
    }

    /// Get the font flags.
    pub fn flags(&self) -> FontFlags {
        let flags = unsafe { FPDFFont_GetFlags(self.handle) };
        FontFlags(flags)
    }

    /// Get the font weight (100-900, where 400 is normal and 700 is bold).
    ///
    /// Returns -1 if unknown.
    pub fn weight(&self) -> i32 {
        unsafe { FPDFFont_GetWeight(self.handle) }
    }

    /// Get the italic angle in degrees (negative = right-leaning).
    pub fn italic_angle(&self) -> Option<i32> {
        let mut angle = 0i32;
        let ok = unsafe { FPDFFont_GetItalicAngle(self.handle, &mut angle) };
        if ok != 0 {
            Some(angle)
        } else {
            None
        }
    }

    /// Get the font ascent (height above baseline) at a given font size.
    pub fn ascent(&self, font_size: f32) -> Option<f32> {
        let mut ascent = 0.0f32;
        let ok = unsafe { FPDFFont_GetAscent(self.handle, font_size, &mut ascent) };
        if ok != 0 {
            Some(ascent)
        } else {
            None
        }
    }

    /// Get the font descent (depth below baseline) at a given font size.
    ///
    /// Note: Descent is typically negative.
    pub fn descent(&self, font_size: f32) -> Option<f32> {
        let mut descent = 0.0f32;
        let ok = unsafe { FPDFFont_GetDescent(self.handle, font_size, &mut descent) };
        if ok != 0 {
            Some(descent)
        } else {
            None
        }
    }

    /// Get the width of a glyph at a given font size.
    ///
    /// `glyph` is the Unicode code point.
    pub fn glyph_width(&self, glyph: u32, font_size: f32) -> Option<f32> {
        let mut width = 0.0f32;
        let ok = unsafe { FPDFFont_GetGlyphWidth(self.handle, glyph, font_size, &mut width) };
        if ok != 0 {
            Some(width)
        } else {
            None
        }
    }
}

/// Bounding box of a page object.
#[derive(Debug, Clone, Copy)]
pub struct ObjectBounds {
    /// Left coordinate
    pub left: f32,
    /// Bottom coordinate
    pub bottom: f32,
    /// Right coordinate
    pub right: f32,
    /// Top coordinate
    pub top: f32,
}

impl ObjectBounds {
    /// Width of the bounding box.
    pub fn width(&self) -> f32 {
        self.right - self.left
    }

    /// Height of the bounding box.
    pub fn height(&self) -> f32 {
        self.top - self.bottom
    }
}

/// A page object (image, text, path, etc.).
///
/// Page objects are the graphical elements that make up a PDF page.
/// Each object has a type, bounding box, and type-specific properties.
pub struct PdfPageObject {
    handle: FPDF_PAGEOBJECT,
    #[allow(dead_code)]
    doc_handle: FPDF_DOCUMENT,
    #[allow(dead_code)]
    page_handle: FPDF_PAGE,
    index: usize,
}

impl PdfPageObject {
    pub(crate) fn new(
        handle: FPDF_PAGEOBJECT,
        doc_handle: FPDF_DOCUMENT,
        page_handle: FPDF_PAGE,
        index: usize,
    ) -> Self {
        Self {
            handle,
            doc_handle,
            page_handle,
            index,
        }
    }

    /// Get the raw page object handle.
    pub fn handle(&self) -> FPDF_PAGEOBJECT {
        self.handle
    }

    /// Get the index of this object on the page.
    pub fn index(&self) -> usize {
        self.index
    }

    /// Get the type of this object.
    pub fn object_type(&self) -> PdfPageObjectType {
        let obj_type = unsafe { FPDFPageObj_GetType(self.handle) };
        PdfPageObjectType::from(obj_type)
    }

    /// Get the bounding box of this object.
    ///
    /// Returns `None` if the bounds cannot be determined.
    pub fn bounds(&self) -> Option<ObjectBounds> {
        let mut left = 0.0f32;
        let mut bottom = 0.0f32;
        let mut right = 0.0f32;
        let mut top = 0.0f32;

        let ok = unsafe {
            FPDFPageObj_GetBounds(self.handle, &mut left, &mut bottom, &mut right, &mut top)
        };

        if ok != 0 {
            Some(ObjectBounds {
                left,
                bottom,
                right,
                top,
            })
        } else {
            None
        }
    }

    /// Check if this is a text object.
    pub fn is_text(&self) -> bool {
        self.object_type() == PdfPageObjectType::Text
    }

    /// Check if this is an image object.
    pub fn is_image(&self) -> bool {
        self.object_type() == PdfPageObjectType::Image
    }

    /// Check if this is a path object.
    pub fn is_path(&self) -> bool {
        self.object_type() == PdfPageObjectType::Path
    }

    /// Get the font size of a text object.
    ///
    /// Returns `None` if this is not a text object.
    pub fn text_font_size(&self) -> Option<f32> {
        if self.object_type() != PdfPageObjectType::Text {
            return None;
        }

        let mut size = 0.0f32;
        let ok = unsafe { FPDFTextObj_GetFontSize(self.handle, &mut size) };

        if ok != 0 {
            Some(size)
        } else {
            None
        }
    }

    /// Get the pixel dimensions of an image object.
    ///
    /// Returns `(width, height)` or `None` if this is not an image object.
    pub fn image_size(&self) -> Option<(u32, u32)> {
        if self.object_type() != PdfPageObjectType::Image {
            return None;
        }

        let mut width: u32 = 0;
        let mut height: u32 = 0;
        let ok = unsafe { FPDFImageObj_GetImagePixelSize(self.handle, &mut width, &mut height) };

        if ok != 0 {
            Some((width, height))
        } else {
            None
        }
    }

    // ==================== IMAGE EXTRACTION ====================

    /// Get the raw bitmap from an image object.
    ///
    /// Returns the bitmap data as stored in the PDF (before any transformations).
    /// This may return a bitmap even if the image is stored in a compressed format.
    ///
    /// Returns `None` if this is not an image object or the bitmap cannot be retrieved.
    pub fn image_bitmap(&self) -> Option<PdfBitmap> {
        if self.object_type() != PdfPageObjectType::Image {
            return None;
        }

        let handle = unsafe { FPDFImageObj_GetBitmap(self.handle) };
        PdfBitmap::from_handle(handle)
    }

    /// Get the rendered bitmap from an image object.
    ///
    /// Unlike `image_bitmap`, this renders the image as it would appear on the page,
    /// applying any transformations, colorspace conversions, and masks.
    ///
    /// Returns `None` if this is not an image object or rendering fails.
    pub fn image_rendered_bitmap(&self) -> Option<PdfBitmap> {
        if self.object_type() != PdfPageObjectType::Image {
            return None;
        }

        let handle = unsafe {
            FPDFImageObj_GetRenderedBitmap(self.doc_handle, self.page_handle, self.handle)
        };
        PdfBitmap::from_handle(handle)
    }

    /// Get the decoded (decompressed) image data.
    ///
    /// Returns the raw pixel data after decoding but before any colorspace conversion.
    /// Returns `None` if this is not an image object or the data cannot be retrieved.
    pub fn image_data_decoded(&self) -> Option<Vec<u8>> {
        if self.object_type() != PdfPageObjectType::Image {
            return None;
        }

        // First call to get buffer size
        let size =
            unsafe { FPDFImageObj_GetImageDataDecoded(self.handle, std::ptr::null_mut(), 0) };

        if size == 0 {
            return None;
        }

        let mut buffer = vec![0u8; size as usize];
        let actual_size = unsafe {
            FPDFImageObj_GetImageDataDecoded(self.handle, buffer.as_mut_ptr() as *mut _, size)
        };

        if actual_size > 0 {
            buffer.truncate(actual_size as usize);
            Some(buffer)
        } else {
            None
        }
    }

    /// Get the raw (compressed) image data.
    ///
    /// Returns the raw image data as stored in the PDF (e.g., JPEG data).
    /// Returns `None` if this is not an image object or the data cannot be retrieved.
    pub fn image_data_raw(&self) -> Option<Vec<u8>> {
        if self.object_type() != PdfPageObjectType::Image {
            return None;
        }

        // First call to get buffer size
        let size = unsafe { FPDFImageObj_GetImageDataRaw(self.handle, std::ptr::null_mut(), 0) };

        if size == 0 {
            return None;
        }

        let mut buffer = vec![0u8; size as usize];
        let actual_size = unsafe {
            FPDFImageObj_GetImageDataRaw(self.handle, buffer.as_mut_ptr() as *mut _, size)
        };

        if actual_size > 0 {
            buffer.truncate(actual_size as usize);
            Some(buffer)
        } else {
            None
        }
    }

    /// Get the number of image filters applied to this image.
    ///
    /// Returns `None` if this is not an image object.
    pub fn image_filter_count(&self) -> Option<usize> {
        if self.object_type() != PdfPageObjectType::Image {
            return None;
        }

        let count = unsafe { FPDFImageObj_GetImageFilterCount(self.handle) };
        if count >= 0 {
            Some(count as usize)
        } else {
            None
        }
    }

    // ==================== COLORS ====================

    /// Get the fill color of this object.
    ///
    /// Returns `None` if the color cannot be retrieved.
    pub fn fill_color(&self) -> Option<ObjectColor> {
        let mut r: u32 = 0;
        let mut g: u32 = 0;
        let mut b: u32 = 0;
        let mut a: u32 = 0;

        let ok = unsafe { FPDFPageObj_GetFillColor(self.handle, &mut r, &mut g, &mut b, &mut a) };

        if ok != 0 {
            Some(ObjectColor::new(r as u8, g as u8, b as u8, a as u8))
        } else {
            None
        }
    }

    /// Get the stroke (outline) color of this object.
    ///
    /// Returns `None` if the color cannot be retrieved.
    pub fn stroke_color(&self) -> Option<ObjectColor> {
        let mut r: u32 = 0;
        let mut g: u32 = 0;
        let mut b: u32 = 0;
        let mut a: u32 = 0;

        let ok = unsafe { FPDFPageObj_GetStrokeColor(self.handle, &mut r, &mut g, &mut b, &mut a) };

        if ok != 0 {
            Some(ObjectColor::new(r as u8, g as u8, b as u8, a as u8))
        } else {
            None
        }
    }

    /// Get the stroke width of this object.
    ///
    /// Returns `None` if the width cannot be retrieved.
    pub fn stroke_width(&self) -> Option<f32> {
        let mut width = 0.0f32;
        let ok = unsafe { FPDFPageObj_GetStrokeWidth(self.handle, &mut width) };

        if ok != 0 {
            Some(width)
        } else {
            None
        }
    }

    // ==================== TRANSPARENCY & TRANSFORM ====================

    /// Check if this object has transparency.
    pub fn has_transparency(&self) -> bool {
        unsafe { FPDFPageObj_HasTransparency(self.handle) != 0 }
    }

    /// Get the fill opacity of this object as a value from 0.0 (transparent) to 1.0 (opaque).
    ///
    /// This is a convenience method that extracts the alpha channel from the fill color
    /// and normalizes it to a 0.0-1.0 range.
    ///
    /// Returns `None` if the fill color cannot be retrieved.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// for obj in page.objects().iter() {
    ///     if let Some(opacity) = obj.fill_opacity() {
    ///         if opacity < 1.0 {
    ///             println!("Object {} has transparent fill: {:.0}%", obj.index(), opacity * 100.0);
    ///         }
    ///     }
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn fill_opacity(&self) -> Option<f32> {
        self.fill_color().map(|c| c.a as f32 / 255.0)
    }

    /// Get the stroke opacity of this object as a value from 0.0 (transparent) to 1.0 (opaque).
    ///
    /// This is a convenience method that extracts the alpha channel from the stroke color
    /// and normalizes it to a 0.0-1.0 range.
    ///
    /// Returns `None` if the stroke color cannot be retrieved.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// for obj in page.objects().iter() {
    ///     if let Some(opacity) = obj.stroke_opacity() {
    ///         if opacity < 1.0 {
    ///             println!("Object {} has transparent stroke: {:.0}%", obj.index(), opacity * 100.0);
    ///         }
    ///     }
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn stroke_opacity(&self) -> Option<f32> {
        self.stroke_color().map(|c| c.a as f32 / 255.0)
    }

    /// Check if this object has a semi-transparent fill (opacity < 1.0).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// let transparent_fills = page.objects().iter()
    ///     .filter(|obj| obj.is_semi_transparent_fill())
    ///     .count();
    /// println!("Found {} objects with semi-transparent fills", transparent_fills);
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn is_semi_transparent_fill(&self) -> bool {
        self.fill_opacity().map(|o| o < 1.0).unwrap_or(false)
    }

    /// Check if this object has a semi-transparent stroke (opacity < 1.0).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// let transparent_strokes = page.objects().iter()
    ///     .filter(|obj| obj.is_semi_transparent_stroke())
    ///     .count();
    /// println!("Found {} objects with semi-transparent strokes", transparent_strokes);
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn is_semi_transparent_stroke(&self) -> bool {
        self.stroke_opacity().map(|o| o < 1.0).unwrap_or(false)
    }

    /// Get the transformation matrix of this object.
    ///
    /// Returns `None` if the matrix cannot be retrieved.
    pub fn matrix(&self) -> Option<ObjectMatrix> {
        let mut matrix = FS_MATRIX {
            a: 0.0,
            b: 0.0,
            c: 0.0,
            d: 0.0,
            e: 0.0,
            f: 0.0,
        };

        let ok = unsafe { FPDFPageObj_GetMatrix(self.handle, &mut matrix) };

        if ok != 0 {
            Some(ObjectMatrix {
                a: matrix.a,
                b: matrix.b,
                c: matrix.c,
                d: matrix.d,
                e: matrix.e,
                f: matrix.f,
            })
        } else {
            None
        }
    }

    // ==================== TEXT OBJECT PROPERTIES ====================

    /// Get the text content of a text object.
    ///
    /// Requires a text page reference from the page's `text()` method.
    /// Returns `None` if this is not a text object or extraction fails.
    pub fn text_content(&self, text_page: &PdfPageText) -> Option<String> {
        if self.object_type() != PdfPageObjectType::Text {
            return None;
        }

        let text_page_handle = text_page.handle();

        // First call to get buffer size (in UTF-16 code units)
        let len =
            unsafe { FPDFTextObj_GetText(self.handle, text_page_handle, std::ptr::null_mut(), 0) };

        if len == 0 {
            return None;
        }

        let mut buffer = vec![0u16; len as usize];
        let actual_len =
            unsafe { FPDFTextObj_GetText(self.handle, text_page_handle, buffer.as_mut_ptr(), len) };

        if actual_len == 0 {
            return None;
        }

        // Remove trailing null
        let text_len = if buffer.last() == Some(&0) {
            buffer.len() - 1
        } else {
            buffer.len()
        };

        String::from_utf16(&buffer[..text_len]).ok()
    }

    /// Get the text rendering mode of a text object.
    ///
    /// Returns `None` if this is not a text object.
    pub fn text_render_mode(&self) -> Option<TextRenderMode> {
        if self.object_type() != PdfPageObjectType::Text {
            return None;
        }

        let mode = unsafe { FPDFTextObj_GetTextRenderMode(self.handle) };
        Some(TextRenderMode::from(mode))
    }

    /// Check if this text object uses invisible text rendering.
    ///
    /// Invisible text is commonly used in scanned PDFs where an OCR layer
    /// is placed over the scanned image to enable text selection and search.
    /// The text is not visually rendered but can be selected and searched.
    ///
    /// Returns `false` if this is not a text object.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("scanned.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// let invisible_text_objects: Vec<_> = page.objects()
    ///     .text_objects()
    ///     .into_iter()
    ///     .filter(|t| t.is_invisible_text())
    ///     .collect();
    /// println!("Found {} invisible text objects", invisible_text_objects.len());
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn is_invisible_text(&self) -> bool {
        self.text_render_mode()
            .map(|mode| mode == TextRenderMode::Invisible)
            .unwrap_or(false)
    }

    /// Get the font handle from a text object.
    ///
    /// Returns `None` if this is not a text object or the font cannot be retrieved.
    /// Note: The returned pointer is managed by PDFium and should not be freed.
    pub fn text_font_handle(&self) -> Option<*mut std::ffi::c_void> {
        if self.object_type() != PdfPageObjectType::Text {
            return None;
        }

        let font = unsafe { FPDFTextObj_GetFont(self.handle) };
        if font.is_null() {
            None
        } else {
            Some(font as *mut std::ffi::c_void)
        }
    }

    /// Get the font from a text object.
    ///
    /// Returns `None` if this is not a text object or the font cannot be retrieved.
    pub fn text_font(&self) -> Option<PdfFont> {
        if self.object_type() != PdfPageObjectType::Text {
            return None;
        }

        let font = unsafe { FPDFTextObj_GetFont(self.handle) };
        PdfFont::from_handle(font)
    }

    // ==================== PATH OBJECT PROPERTIES ====================

    /// Get the number of segments in a path object.
    ///
    /// Returns `None` if this is not a path object.
    pub fn path_segment_count(&self) -> Option<usize> {
        if self.object_type() != PdfPageObjectType::Path {
            return None;
        }

        let count = unsafe { FPDFPath_CountSegments(self.handle) };
        if count >= 0 {
            Some(count as usize)
        } else {
            None
        }
    }

    /// Get a path segment by index.
    ///
    /// Returns `None` if this is not a path object or the index is out of bounds.
    pub fn path_segment(&self, index: usize) -> Option<PathSegment> {
        if self.object_type() != PdfPageObjectType::Path {
            return None;
        }

        let handle = unsafe { FPDFPath_GetPathSegment(self.handle, index as i32) };
        PathSegment::new(handle)
    }

    /// Get all path segments as a vector.
    ///
    /// Returns `None` if this is not a path object.
    pub fn path_segments(&self) -> Option<Vec<PathSegment>> {
        let count = self.path_segment_count()?;
        let mut segments = Vec::with_capacity(count);
        for i in 0..count {
            if let Some(seg) = self.path_segment(i) {
                segments.push(seg);
            }
        }
        Some(segments)
    }

    /// Get the draw mode of a path object.
    ///
    /// Returns `None` if this is not a path object or the mode cannot be retrieved.
    pub fn path_draw_mode(&self) -> Option<PathDrawMode> {
        if self.object_type() != PdfPageObjectType::Path {
            return None;
        }

        let mut fill_mode: i32 = 0;
        let mut stroke: i32 = 0;
        let ok = unsafe { FPDFPath_GetDrawMode(self.handle, &mut fill_mode, &mut stroke) };

        if ok != 0 {
            let fill = match fill_mode {
                0 => PathFillMode::None,
                1 => PathFillMode::Alternate,
                2 => PathFillMode::Winding,
                _ => PathFillMode::None,
            };
            Some(PathDrawMode {
                fill_mode: fill,
                stroke: stroke != 0,
            })
        } else {
            None
        }
    }

    // ==================== LINE EXTRACTION ====================

    /// Check if this path object is a simple line (MoveTo + LineTo).
    ///
    /// A simple line is a path with exactly 2 segments: MoveTo followed by LineTo.
    /// This is useful for identifying separators, table borders, and decorative elements.
    ///
    /// Returns `None` if this is not a path object.
    pub fn is_simple_line(&self) -> Option<bool> {
        if self.object_type() != PdfPageObjectType::Path {
            return None;
        }

        let segments = self.path_segments()?;
        if segments.len() != 2 {
            return Some(false);
        }

        let first_type = segments[0].segment_type();
        let second_type = segments[1].segment_type();

        Some(first_type == PathSegmentType::MoveTo && second_type == PathSegmentType::LineTo)
    }

    /// Extract a line from this path object if it is a simple line.
    ///
    /// Returns an `ExtractedLine` if this path represents a simple line segment
    /// (MoveTo + LineTo), or `None` if it's a more complex path or not a path at all.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::{Pdfium, PdfPageObjectType};
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// for obj in page.objects().paths() {
    ///     if let Some(line) = obj.extract_line() {
    ///         println!("Line: ({:.1}, {:.1}) -> ({:.1}, {:.1})",
    ///             line.start.0, line.start.1, line.end.0, line.end.1);
    ///     }
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn extract_line(&self) -> Option<ExtractedLine> {
        // Check if it's a simple line
        if !self.is_simple_line().unwrap_or(false) {
            return None;
        }

        // Check if it's stroked (lines need to be stroked to be visible)
        let draw_mode = self.path_draw_mode()?;
        if !draw_mode.stroke {
            return None;
        }

        let segments = self.path_segments()?;
        let start = segments[0].point()?;
        let end = segments[1].point()?;

        // Get stroke properties
        let thickness = self.stroke_width().unwrap_or(1.0);
        let color = self
            .stroke_color()
            .map_or((0, 0, 0, 255), |c| (c.r, c.g, c.b, c.a));

        Some(ExtractedLine::new(start, end, thickness, color))
    }

    /// Extract all lines from this path object.
    ///
    /// Unlike `extract_line()`, this method can extract multiple line segments
    /// from complex paths (e.g., polylines, rectangles drawn as paths).
    ///
    /// Returns `None` if this is not a path object.
    pub fn extract_lines(&self) -> Option<Vec<ExtractedLine>> {
        if self.object_type() != PdfPageObjectType::Path {
            return None;
        }

        // Check if stroked
        let draw_mode = self.path_draw_mode()?;
        if !draw_mode.stroke {
            return Some(Vec::new());
        }

        let segments = self.path_segments()?;
        if segments.is_empty() {
            return Some(Vec::new());
        }

        let thickness = self.stroke_width().unwrap_or(1.0);
        let color = self
            .stroke_color()
            .map_or((0, 0, 0, 255), |c| (c.r, c.g, c.b, c.a));

        let mut lines = Vec::new();
        let mut current_pos: Option<(f32, f32)> = None;
        let mut subpath_start: Option<(f32, f32)> = None;

        for seg in &segments {
            match seg.segment_type() {
                PathSegmentType::MoveTo => {
                    if let Some(pt) = seg.point() {
                        current_pos = Some(pt);
                        subpath_start = Some(pt);
                    }
                }
                PathSegmentType::LineTo => {
                    if let (Some(start), Some(end)) = (current_pos, seg.point()) {
                        lines.push(ExtractedLine::new(start, end, thickness, color));
                        current_pos = Some(end);
                    }
                }
                PathSegmentType::BezierTo => {
                    // Skip Bezier curves for line extraction
                    // but update current position
                    if let Some(pt) = seg.point() {
                        current_pos = Some(pt);
                    }
                }
                PathSegmentType::Unknown => {}
            }

            // Handle path closing
            if seg.closes_path() {
                if let (Some(start), Some(end)) = (current_pos, subpath_start) {
                    if start != end {
                        lines.push(ExtractedLine::new(start, end, thickness, color));
                    }
                }
            }
        }

        Some(lines)
    }

    // ==================== COLORED REGION EXTRACTION ====================

    /// Extract a colored region from this path object if it has a fill.
    ///
    /// Colored regions are filled paths that can represent backgrounds,
    /// table cell fills, or decorative boxes. The `is_behind_text` field
    /// indicates z-order relative to text (requires object index context).
    ///
    /// Returns `None` if this is not a path object or has no fill.
    ///
    /// # Arguments
    ///
    /// * `is_behind_text` - Whether this region appears before text in z-order
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::{Pdfium, PdfPageObjectType};
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// for obj in page.objects().paths() {
    ///     if let Some(region) = obj.extract_colored_region(false) {
    ///         println!("Colored region: {:?}", region.bounds);
    ///     }
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn extract_colored_region(&self, is_behind_text: bool) -> Option<ColoredRegion> {
        if self.object_type() != PdfPageObjectType::Path {
            return None;
        }

        // Get draw mode to check if filled
        let draw_mode = self.path_draw_mode()?;

        // A colored region must have a fill to be meaningful
        if draw_mode.fill_mode == PathFillMode::None {
            return None;
        }

        // Get bounds
        let obj_bounds = self.bounds()?;
        let bounds = (
            obj_bounds.left,
            obj_bounds.bottom,
            obj_bounds.right,
            obj_bounds.top,
        );

        // Get fill color
        let fill_color = self.fill_color().map(|c| (c.r, c.g, c.b, c.a));

        // Get stroke color if stroked
        let stroke_color = if draw_mode.stroke {
            self.stroke_color().map(|c| (c.r, c.g, c.b, c.a))
        } else {
            None
        };

        Some(ColoredRegion::new(
            bounds,
            fill_color,
            stroke_color,
            is_behind_text,
        ))
    }

    /// Check if this path object is a filled rectangle.
    ///
    /// A filled rectangle is a path with fill that has 4 line segments
    /// forming a closed rectangular shape. This is common for backgrounds
    /// and table cells.
    ///
    /// Returns `None` if this is not a path object.
    pub fn is_filled_rectangle(&self) -> Option<bool> {
        if self.object_type() != PdfPageObjectType::Path {
            return None;
        }

        let draw_mode = self.path_draw_mode()?;
        if draw_mode.fill_mode == PathFillMode::None {
            return Some(false);
        }

        let segments = self.path_segments()?;
        // A rectangle has MoveTo + 4 LineTo (or 3 LineTo + close)
        // Check for 4-5 segments that form a closed path
        if segments.len() < 4 || segments.len() > 5 {
            return Some(false);
        }

        // Check first is MoveTo
        if segments[0].segment_type() != PathSegmentType::MoveTo {
            return Some(false);
        }

        // Check remaining are LineTo
        let line_segments: Vec<_> = segments[1..]
            .iter()
            .filter(|s| s.segment_type() == PathSegmentType::LineTo)
            .collect();

        // Need at least 3 line segments (4th is implicit close or explicit)
        if line_segments.len() < 3 {
            return Some(false);
        }

        // Check if path is closed
        let is_closed = segments.iter().any(|s| s.closes_path());
        if !is_closed && line_segments.len() < 4 {
            return Some(false);
        }

        Some(true)
    }

    // ==================== IMAGE METADATA ====================

    /// Get detailed metadata about an image object.
    ///
    /// Returns `None` if this is not an image object or metadata cannot be retrieved.
    pub fn image_metadata(&self) -> Option<ImageMetadata> {
        if self.object_type() != PdfPageObjectType::Image {
            return None;
        }

        let mut metadata = FPDF_IMAGEOBJ_METADATA {
            width: 0,
            height: 0,
            horizontal_dpi: 0.0,
            vertical_dpi: 0.0,
            bits_per_pixel: 0,
            colorspace: 0,
            marked_content_id: -1,
        };

        let ok =
            unsafe { FPDFImageObj_GetImageMetadata(self.handle, self.page_handle, &mut metadata) };

        if ok != 0 {
            Some(ImageMetadata {
                width: metadata.width,
                height: metadata.height,
                horizontal_dpi: metadata.horizontal_dpi,
                vertical_dpi: metadata.vertical_dpi,
                bits_per_pixel: metadata.bits_per_pixel,
                colorspace: ImageColorspace::from(metadata.colorspace),
                marked_content_id: metadata.marked_content_id,
            })
        } else {
            None
        }
    }

    /// Get the compression filters applied to an image object.
    ///
    /// Returns a list of filter names in the order they should be applied.
    /// Returns `None` if this is not an image object.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::{Pdfium, ImageFilter};
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("scanned.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// for obj in page.objects().images() {
    ///     if let Some(filters) = obj.image_filters() {
    ///         for filter in &filters {
    ///             if filter.is_lossy() {
    ///                 println!("Image uses lossy compression: {}", filter.name());
    ///             }
    ///         }
    ///     }
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn image_filters(&self) -> Option<Vec<ImageFilter>> {
        if self.object_type() != PdfPageObjectType::Image {
            return None;
        }

        let count = unsafe { FPDFImageObj_GetImageFilterCount(self.handle) };
        if count < 0 {
            return None;
        }

        let mut filters = Vec::with_capacity(count as usize);
        for i in 0..count {
            // First call to get buffer size
            let len =
                unsafe { FPDFImageObj_GetImageFilter(self.handle, i, std::ptr::null_mut(), 0) };

            if len == 0 {
                continue;
            }

            let mut buffer = vec![0u8; len as usize];
            let actual_len = unsafe {
                FPDFImageObj_GetImageFilter(self.handle, i, buffer.as_mut_ptr() as *mut _, len)
            };

            if actual_len > 0 {
                // Remove trailing null
                let str_len = if buffer.last() == Some(&0) {
                    buffer.len() - 1
                } else {
                    buffer.len()
                };

                if let Ok(name) = std::str::from_utf8(&buffer[..str_len]) {
                    filters.push(ImageFilter::from_name(name));
                }
            }
        }

        Some(filters)
    }

    /// Get comprehensive technical metadata about an image object.
    ///
    /// This method provides detailed information about an image's encoding,
    /// dimensions, color space, and compression filters. It's designed for
    /// document analysis and classification workflows.
    ///
    /// Returns `None` if this is not an image object or metadata cannot be retrieved.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// for obj in page.objects().images() {
    ///     if let Some(meta) = obj.image_tech_metadata() {
    ///         println!("Image: {}x{} @ {:?} DPI", meta.width_px, meta.height_px, meta.dpi);
    ///         println!("  Color space: {:?}", meta.color_space);
    ///         println!("  Bits/component: {}", meta.bits_per_component);
    ///         println!("  Filters: {:?}", meta.filters.iter().map(|f| f.name()).collect::<Vec<_>>());
    ///         if meta.is_jpeg() {
    ///             println!("  This is a JPEG image");
    ///         }
    ///     }
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn image_tech_metadata(&self) -> Option<ImageTechMeta> {
        if self.object_type() != PdfPageObjectType::Image {
            return None;
        }

        // Get basic metadata
        let basic_meta = self.image_metadata()?;

        // Get filters
        let filters = self.image_filters().unwrap_or_default();

        // Calculate bits per component from bits per pixel and colorspace
        let components = match basic_meta.colorspace {
            ImageColorspace::DeviceGray | ImageColorspace::CalGray => 1,
            ImageColorspace::DeviceRGB | ImageColorspace::CalRGB | ImageColorspace::Lab => 3,
            ImageColorspace::DeviceCMYK => 4,
            ImageColorspace::Indexed => 1, // Palette index
            _ => 1,                        // Default assumption
        };

        let bits_per_component = if components > 0 && basic_meta.bits_per_pixel > 0 {
            (basic_meta.bits_per_pixel / components) as u8
        } else {
            8 // Default assumption
        };

        // Detect masks by checking if rendered bitmap differs from raw bitmap
        // This is a heuristic - true mask detection would require deeper PDF parsing
        let has_mask = self.has_transparency();
        let has_soft_mask = has_mask; // Soft mask also triggers transparency

        Some(ImageTechMeta {
            width_px: basic_meta.width,
            height_px: basic_meta.height,
            dpi: (basic_meta.horizontal_dpi, basic_meta.vertical_dpi),
            bits_per_component,
            color_space: basic_meta.colorspace,
            filters,
            has_mask,
            has_soft_mask,
        })
    }

    // ========================================================================
    // Artifact Detection API
    // ========================================================================

    /// Check if this object is marked as an artifact.
    ///
    /// Artifacts are decorative elements not part of the document's logical
    /// content, such as headers, footers, page numbers, watermarks, and
    /// decorative borders. They are typically excluded from accessibility
    /// processing and text extraction.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// for obj in page.objects().iter() {
    ///     if obj.is_artifact() {
    ///         println!("Artifact found at {:?}", obj.bounds());
    ///     }
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn is_artifact(&self) -> bool {
        // Check all content marks for "Artifact"
        let mark_count = unsafe { FPDFPageObj_CountMarks(self.handle) };
        if mark_count <= 0 {
            return false;
        }

        for i in 0..mark_count as u32 {
            let mark = unsafe { FPDFPageObj_GetMark(self.handle, i as _) };
            if mark.is_null() {
                continue;
            }

            // Get the mark name
            let mut buflen: u64 = 0;
            unsafe { FPDFPageObjMark_GetName(mark, std::ptr::null_mut(), 0, &mut buflen) };

            if buflen == 0 {
                continue;
            }

            let mut buffer: Vec<u16> = vec![0; (buflen / 2) as usize + 1];
            let ok = unsafe {
                FPDFPageObjMark_GetName(mark, buffer.as_mut_ptr() as *mut _, buflen, &mut buflen)
            };

            if ok != 0 {
                // Convert UTF-16 to String
                let name = String::from_utf16_lossy(&buffer[..buffer.len() - 1]);
                let name = name.trim_end_matches('\0');
                if name == "Artifact" {
                    return true;
                }
            }
        }

        false
    }

    /// Get the artifact type if this object is an artifact.
    ///
    /// Returns the artifact type (Header, Footer, Watermark, etc.) if this
    /// object is marked as an artifact with a Type property. Returns None
    /// if the object is not an artifact or has no type specified.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::{Pdfium, ArtifactType};
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let page = doc.page(0)?;
    ///
    /// for obj in page.objects().iter() {
    ///     if let Some(artifact_type) = obj.artifact_type() {
    ///         println!("Found artifact: {:?}", artifact_type);
    ///     }
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn artifact_type(&self) -> Option<ArtifactType> {
        let mark_count = unsafe { FPDFPageObj_CountMarks(self.handle) };
        if mark_count <= 0 {
            return None;
        }

        for i in 0..mark_count as u32 {
            let mark = unsafe { FPDFPageObj_GetMark(self.handle, i as _) };
            if mark.is_null() {
                continue;
            }

            // Get the mark name
            let mut buflen: u64 = 0;
            unsafe { FPDFPageObjMark_GetName(mark, std::ptr::null_mut(), 0, &mut buflen) };

            if buflen == 0 {
                continue;
            }

            let mut buffer: Vec<u16> = vec![0; (buflen / 2) as usize + 1];
            let ok = unsafe {
                FPDFPageObjMark_GetName(mark, buffer.as_mut_ptr() as *mut _, buflen, &mut buflen)
            };

            if ok != 0 {
                let name = String::from_utf16_lossy(&buffer[..buffer.len() - 1]);
                let name = name.trim_end_matches('\0');

                if name == "Artifact" {
                    // Try to get the Type parameter
                    let type_key = b"Type\0";
                    let mut type_buflen: u64 = 0;
                    unsafe {
                        FPDFPageObjMark_GetParamStringValue(
                            mark,
                            type_key.as_ptr() as *const _,
                            std::ptr::null_mut(),
                            0,
                            &mut type_buflen,
                        )
                    };

                    if type_buflen > 0 {
                        let mut type_buffer: Vec<u16> = vec![0; (type_buflen / 2) as usize + 1];
                        let type_ok = unsafe {
                            FPDFPageObjMark_GetParamStringValue(
                                mark,
                                type_key.as_ptr() as *const _,
                                type_buffer.as_mut_ptr() as *mut _,
                                type_buflen,
                                &mut type_buflen,
                            )
                        };

                        if type_ok != 0 {
                            let type_str =
                                String::from_utf16_lossy(&type_buffer[..type_buffer.len() - 1]);
                            let type_str = type_str.trim_end_matches('\0');
                            return Some(ArtifactType::from_type_string(type_str));
                        }
                    }

                    // Artifact without Type property
                    return Some(ArtifactType::Other);
                }
            }
        }

        None
    }

    /// Get the number of content marks on this object.
    ///
    /// Content marks are used to tag page objects with metadata like
    /// artifact status, structure element associations, etc.
    pub fn mark_count(&self) -> usize {
        let count = unsafe { FPDFPageObj_CountMarks(self.handle) };
        if count > 0 {
            count as usize
        } else {
            0
        }
    }

    /// Get the names of all content marks on this object.
    ///
    /// Returns a list of mark names (e.g., "Artifact", "P", "Span").
    pub fn mark_names(&self) -> Vec<String> {
        let mark_count = unsafe { FPDFPageObj_CountMarks(self.handle) };
        if mark_count <= 0 {
            return Vec::new();
        }

        let mut names = Vec::new();
        for i in 0..mark_count as u32 {
            let mark = unsafe { FPDFPageObj_GetMark(self.handle, i as _) };
            if mark.is_null() {
                continue;
            }

            let mut buflen: u64 = 0;
            unsafe { FPDFPageObjMark_GetName(mark, std::ptr::null_mut(), 0, &mut buflen) };

            if buflen == 0 {
                continue;
            }

            let mut buffer: Vec<u16> = vec![0; (buflen / 2) as usize + 1];
            let ok = unsafe {
                FPDFPageObjMark_GetName(mark, buffer.as_mut_ptr() as *mut _, buflen, &mut buflen)
            };

            if ok != 0 {
                let name = String::from_utf16_lossy(&buffer[..buffer.len() - 1]);
                let name = name.trim_end_matches('\0');
                if !name.is_empty() {
                    names.push(name.to_string());
                }
            }
        }

        names
    }
}

/// Collection of page objects for iteration.
pub struct PdfPageObjects {
    page_handle: FPDF_PAGE,
    doc_handle: FPDF_DOCUMENT,
    count: usize,
}

impl PdfPageObjects {
    pub(crate) fn new(page_handle: FPDF_PAGE, doc_handle: FPDF_DOCUMENT) -> Self {
        let count = unsafe { FPDFPage_CountObjects(page_handle) };
        Self {
            page_handle,
            doc_handle,
            count: if count > 0 { count as usize } else { 0 },
        }
    }

    /// Get the number of objects on the page.
    pub fn count(&self) -> usize {
        self.count
    }

    /// Check if the page has no objects.
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Get an object by index.
    ///
    /// Returns `None` if the index is out of bounds.
    pub fn get(&self, index: usize) -> Option<PdfPageObject> {
        if index >= self.count {
            return None;
        }

        let handle = unsafe { FPDFPage_GetObject(self.page_handle, index as i32) };
        if handle.is_null() {
            return None;
        }

        Some(PdfPageObject::new(
            handle,
            self.doc_handle,
            self.page_handle,
            index,
        ))
    }

    /// Get an iterator over all objects.
    pub fn iter(&self) -> PdfPageObjectsIter<'_> {
        PdfPageObjectsIter {
            objects: self,
            index: 0,
        }
    }

    /// Count objects of a specific type.
    pub fn count_of_type(&self, obj_type: PdfPageObjectType) -> usize {
        self.iter()
            .filter(|obj| obj.object_type() == obj_type)
            .count()
    }

    /// Get all image objects.
    pub fn images(&self) -> Vec<PdfPageObject> {
        self.iter()
            .filter(|obj| obj.object_type() == PdfPageObjectType::Image)
            .collect()
    }

    /// Get all text objects.
    pub fn text_objects(&self) -> Vec<PdfPageObject> {
        self.iter()
            .filter(|obj| obj.object_type() == PdfPageObjectType::Text)
            .collect()
    }

    /// Get all path objects.
    pub fn paths(&self) -> Vec<PdfPageObject> {
        self.iter()
            .filter(|obj| obj.object_type() == PdfPageObjectType::Path)
            .collect()
    }
}

/// Iterator over page objects.
pub struct PdfPageObjectsIter<'a> {
    objects: &'a PdfPageObjects,
    index: usize,
}

impl<'a> Iterator for PdfPageObjectsIter<'a> {
    type Item = PdfPageObject;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.objects.count {
            return None;
        }
        let obj = self.objects.get(self.index)?;
        self.index += 1;
        Some(obj)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.objects.count - self.index;
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for PdfPageObjectsIter<'a> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_object_type_from() {
        assert_eq!(PdfPageObjectType::from(0), PdfPageObjectType::Unknown);
        assert_eq!(PdfPageObjectType::from(1), PdfPageObjectType::Text);
        assert_eq!(PdfPageObjectType::from(2), PdfPageObjectType::Path);
        assert_eq!(PdfPageObjectType::from(3), PdfPageObjectType::Image);
        assert_eq!(PdfPageObjectType::from(4), PdfPageObjectType::Shading);
        assert_eq!(PdfPageObjectType::from(5), PdfPageObjectType::Form);
        assert_eq!(PdfPageObjectType::from(99), PdfPageObjectType::Unknown);
    }

    #[test]
    fn test_object_bounds() {
        let bounds = ObjectBounds {
            left: 10.0,
            bottom: 20.0,
            right: 110.0,
            top: 120.0,
        };
        assert_eq!(bounds.width(), 100.0);
        assert_eq!(bounds.height(), 100.0);
    }

    #[test]
    fn test_object_color() {
        let color = ObjectColor::new(255, 128, 64, 200);
        assert_eq!(color.r, 255);
        assert_eq!(color.g, 128);
        assert_eq!(color.b, 64);
        assert_eq!(color.a, 200);
        assert!(!color.is_opaque());
        assert!(!color.is_transparent());

        let opaque = ObjectColor::rgb(100, 150, 200);
        assert!(opaque.is_opaque());
        assert_eq!(opaque.a, 255);

        let transparent = ObjectColor::new(0, 0, 0, 0);
        assert!(transparent.is_transparent());

        assert_eq!(ObjectColor::rgb(255, 87, 51).to_hex(), "#FF5733");
        assert_eq!(
            ObjectColor::new(255, 87, 51, 204).to_hex_with_alpha(),
            "#FF5733CC"
        );
    }

    #[test]
    fn test_text_render_mode() {
        assert_eq!(TextRenderMode::from(0), TextRenderMode::Fill);
        assert_eq!(TextRenderMode::from(1), TextRenderMode::Stroke);
        assert_eq!(TextRenderMode::from(2), TextRenderMode::FillStroke);
        assert_eq!(TextRenderMode::from(3), TextRenderMode::Invisible);
        assert_eq!(TextRenderMode::from(4), TextRenderMode::FillClip);
        assert_eq!(TextRenderMode::from(5), TextRenderMode::StrokeClip);
        assert_eq!(TextRenderMode::from(6), TextRenderMode::FillStrokeClip);
        assert_eq!(TextRenderMode::from(7), TextRenderMode::Clip);
        assert_eq!(TextRenderMode::from(99), TextRenderMode::Unknown);
    }

    #[test]
    fn test_object_matrix() {
        let identity = ObjectMatrix::identity();
        assert_eq!(identity.a, 1.0);
        assert_eq!(identity.b, 0.0);
        assert_eq!(identity.c, 0.0);
        assert_eq!(identity.d, 1.0);
        assert_eq!(identity.e, 0.0);
        assert_eq!(identity.f, 0.0);
        assert!((identity.scale_x() - 1.0).abs() < 0.001);
        assert!((identity.scale_y() - 1.0).abs() < 0.001);
        assert!((identity.rotation()).abs() < 0.001);
        assert_eq!(identity.translation(), (0.0, 0.0));

        let scaled = ObjectMatrix {
            a: 2.0,
            b: 0.0,
            c: 0.0,
            d: 3.0,
            e: 100.0,
            f: 200.0,
        };
        assert!((scaled.scale_x() - 2.0).abs() < 0.001);
        assert!((scaled.scale_y() - 3.0).abs() < 0.001);
        assert_eq!(scaled.translation(), (100.0, 200.0));
    }

    #[test]
    fn test_path_segment_type() {
        assert_eq!(PathSegmentType::from(0), PathSegmentType::LineTo);
        assert_eq!(PathSegmentType::from(1), PathSegmentType::BezierTo);
        assert_eq!(PathSegmentType::from(2), PathSegmentType::MoveTo);
        assert_eq!(PathSegmentType::from(99), PathSegmentType::Unknown);
    }

    #[test]
    fn test_path_fill_mode() {
        let mode = PathDrawMode {
            fill_mode: PathFillMode::Alternate,
            stroke: true,
        };
        assert_eq!(mode.fill_mode, PathFillMode::Alternate);
        assert!(mode.stroke);
    }

    #[test]
    fn test_image_colorspace() {
        assert_eq!(ImageColorspace::from(0), ImageColorspace::Unknown);
        assert_eq!(ImageColorspace::from(1), ImageColorspace::DeviceGray);
        assert_eq!(ImageColorspace::from(2), ImageColorspace::DeviceRGB);
        assert_eq!(ImageColorspace::from(3), ImageColorspace::DeviceCMYK);
        assert_eq!(ImageColorspace::from(4), ImageColorspace::CalGray);
        assert_eq!(ImageColorspace::from(5), ImageColorspace::CalRGB);
        assert_eq!(ImageColorspace::from(6), ImageColorspace::Lab);
        assert_eq!(ImageColorspace::from(7), ImageColorspace::ICCBased);
        assert_eq!(ImageColorspace::from(8), ImageColorspace::Separation);
        assert_eq!(ImageColorspace::from(9), ImageColorspace::DeviceN);
        assert_eq!(ImageColorspace::from(10), ImageColorspace::Indexed);
        assert_eq!(ImageColorspace::from(11), ImageColorspace::Pattern);
        assert_eq!(ImageColorspace::from(99), ImageColorspace::Unknown);
    }

    #[test]
    fn test_image_metadata() {
        let meta = ImageMetadata {
            width: 1920,
            height: 1080,
            horizontal_dpi: 96.0,
            vertical_dpi: 96.0,
            bits_per_pixel: 24,
            colorspace: ImageColorspace::DeviceRGB,
            marked_content_id: -1,
        };
        assert_eq!(meta.width, 1920);
        assert_eq!(meta.height, 1080);
        assert_eq!(meta.colorspace, ImageColorspace::DeviceRGB);
    }

    #[test]
    fn test_font_flags() {
        // fixed (bit 0) + serif (bit 1) + symbolic (bit 2) + italic (bit 6) + all_cap (bit 16)
        // = 1 + 2 + 4 + 64 + 65536 = 65607 = 0x10047
        let flags = FontFlags(0x10047);
        assert!(flags.is_fixed_pitch());
        assert!(flags.is_serif());
        assert!(flags.is_symbolic());
        assert!(!flags.is_script());
        assert!(!flags.is_nonsymbolic());
        assert!(flags.is_italic());
        assert!(flags.is_all_cap());
        assert!(!flags.is_small_cap());
        assert!(!flags.is_force_bold());

        let no_flags = FontFlags(0);
        assert!(!no_flags.is_fixed_pitch());
        assert!(!no_flags.is_serif());
    }

    #[test]
    fn test_image_filter_from_name() {
        assert_eq!(ImageFilter::from_name("DCTDecode"), ImageFilter::DCTDecode);
        assert_eq!(ImageFilter::from_name("DCT"), ImageFilter::DCTDecode);
        assert_eq!(
            ImageFilter::from_name("FlateDecode"),
            ImageFilter::FlateDecode
        );
        assert_eq!(ImageFilter::from_name("Fl"), ImageFilter::FlateDecode);
        assert_eq!(ImageFilter::from_name("JPXDecode"), ImageFilter::JPXDecode);
        assert_eq!(
            ImageFilter::from_name("JBIG2Decode"),
            ImageFilter::JBIG2Decode
        );
        assert_eq!(
            ImageFilter::from_name("CCITTFaxDecode"),
            ImageFilter::CCITTFaxDecode
        );
        assert_eq!(ImageFilter::from_name("CCF"), ImageFilter::CCITTFaxDecode);
        assert_eq!(ImageFilter::from_name("LZWDecode"), ImageFilter::LZWDecode);
        assert_eq!(ImageFilter::from_name("LZW"), ImageFilter::LZWDecode);
        assert_eq!(
            ImageFilter::from_name("RunLengthDecode"),
            ImageFilter::RunLengthDecode
        );
        assert_eq!(ImageFilter::from_name("RL"), ImageFilter::RunLengthDecode);
        assert_eq!(
            ImageFilter::from_name("ASCIIHexDecode"),
            ImageFilter::ASCIIHexDecode
        );
        assert_eq!(ImageFilter::from_name("AHx"), ImageFilter::ASCIIHexDecode);
        assert_eq!(
            ImageFilter::from_name("ASCII85Decode"),
            ImageFilter::ASCII85Decode
        );
        assert_eq!(ImageFilter::from_name("A85"), ImageFilter::ASCII85Decode);
        assert_eq!(ImageFilter::from_name("Crypt"), ImageFilter::Crypt);
        assert_eq!(
            ImageFilter::from_name("CustomFilter"),
            ImageFilter::Unknown("CustomFilter".to_string())
        );
    }

    #[test]
    fn test_image_filter_properties() {
        assert!(ImageFilter::DCTDecode.is_lossy());
        assert!(ImageFilter::JPXDecode.is_lossy());
        assert!(!ImageFilter::FlateDecode.is_lossy());
        assert!(!ImageFilter::LZWDecode.is_lossy());

        assert!(ImageFilter::CCITTFaxDecode.is_bilevel());
        assert!(ImageFilter::JBIG2Decode.is_bilevel());
        assert!(!ImageFilter::DCTDecode.is_bilevel());
        assert!(!ImageFilter::FlateDecode.is_bilevel());

        assert_eq!(ImageFilter::DCTDecode.name(), "DCTDecode");
        assert_eq!(ImageFilter::FlateDecode.name(), "FlateDecode");
        assert_eq!(ImageFilter::Unknown("Custom".to_string()).name(), "Custom");
    }

    #[test]
    fn test_image_tech_meta() {
        let meta = ImageTechMeta {
            width_px: 1920,
            height_px: 1080,
            dpi: (300.0, 300.0),
            bits_per_component: 8,
            color_space: ImageColorspace::DeviceRGB,
            filters: vec![ImageFilter::DCTDecode],
            has_mask: false,
            has_soft_mask: false,
        };

        assert_eq!(meta.width_px, 1920);
        assert_eq!(meta.height_px, 1080);
        assert_eq!(meta.dpi, (300.0, 300.0));
        assert_eq!(meta.bits_per_component, 8);
        assert_eq!(meta.color_space, ImageColorspace::DeviceRGB);
        assert!(meta.is_jpeg());
        assert!(!meta.is_jpeg2000());
        assert!(!meta.is_bilevel());
        assert!(!meta.is_grayscale());
        assert!(meta.is_color());
        assert_eq!(meta.components(), 3);
        assert_eq!(meta.bits_per_pixel(), 24);
        assert!(!meta.has_any_mask());

        // Test grayscale image
        let gray_meta = ImageTechMeta {
            width_px: 100,
            height_px: 100,
            dpi: (72.0, 72.0),
            bits_per_component: 8,
            color_space: ImageColorspace::DeviceGray,
            filters: vec![ImageFilter::FlateDecode],
            has_mask: false,
            has_soft_mask: false,
        };
        assert!(gray_meta.is_grayscale());
        assert!(!gray_meta.is_color());
        assert_eq!(gray_meta.components(), 1);
        assert_eq!(gray_meta.bits_per_pixel(), 8);

        // Test bilevel image
        let bilevel_meta = ImageTechMeta {
            width_px: 2550,
            height_px: 3300,
            dpi: (300.0, 300.0),
            bits_per_component: 1,
            color_space: ImageColorspace::DeviceGray,
            filters: vec![ImageFilter::CCITTFaxDecode],
            has_mask: false,
            has_soft_mask: false,
        };
        assert!(bilevel_meta.is_bilevel());
        assert_eq!(bilevel_meta.bits_per_pixel(), 1);

        // Test CMYK image
        let cmyk_meta = ImageTechMeta {
            width_px: 800,
            height_px: 600,
            dpi: (300.0, 300.0),
            bits_per_component: 8,
            color_space: ImageColorspace::DeviceCMYK,
            filters: vec![ImageFilter::DCTDecode],
            has_mask: true,
            has_soft_mask: true,
        };
        assert!(cmyk_meta.is_color());
        assert_eq!(cmyk_meta.components(), 4);
        assert_eq!(cmyk_meta.bits_per_pixel(), 32);
        assert!(cmyk_meta.has_any_mask());
    }

    #[test]
    fn test_image_tech_meta_uncompressed_size() {
        // 100x100 RGB 8-bit image = 100 * 100 * 24 bits = 240000 bits = 30000 bytes
        let rgb_meta = ImageTechMeta {
            width_px: 100,
            height_px: 100,
            dpi: (72.0, 72.0),
            bits_per_component: 8,
            color_space: ImageColorspace::DeviceRGB,
            filters: vec![],
            has_mask: false,
            has_soft_mask: false,
        };
        assert_eq!(rgb_meta.uncompressed_size(), 30000);

        // 100x100 bilevel image = 100 * 100 * 1 bit = 10000 bits = 1250 bytes
        let bilevel_meta = ImageTechMeta {
            width_px: 100,
            height_px: 100,
            dpi: (72.0, 72.0),
            bits_per_component: 1,
            color_space: ImageColorspace::DeviceGray,
            filters: vec![],
            has_mask: false,
            has_soft_mask: false,
        };
        assert_eq!(bilevel_meta.uncompressed_size(), 1250);
    }

    #[test]
    fn test_extracted_line_horizontal() {
        // Horizontal line: 100pt long, 0pt vertical
        let line = ExtractedLine::new((0.0, 100.0), (100.0, 100.0), 1.0, (0, 0, 0, 255));
        assert!(line.is_horizontal);
        assert!(!line.is_vertical);
        assert!(!line.is_diagonal());
        assert!((line.length() - 100.0).abs() < 0.001);
        assert_eq!(line.midpoint(), (50.0, 100.0));
        assert!(line.angle().abs() < 0.01);
        assert!(line.y_position().is_some());
        assert!(line.x_position().is_none());
    }

    #[test]
    fn test_extracted_line_vertical() {
        // Vertical line: 0pt horizontal, 200pt long
        let line = ExtractedLine::new((50.0, 0.0), (50.0, 200.0), 2.0, (255, 0, 0, 255));
        assert!(!line.is_horizontal);
        assert!(line.is_vertical);
        assert!(!line.is_diagonal());
        assert!((line.length() - 200.0).abs() < 0.001);
        assert_eq!(line.midpoint(), (50.0, 100.0));
        assert!(line.x_position().is_some());
        assert!(line.y_position().is_none());
    }

    #[test]
    fn test_extracted_line_diagonal() {
        // Diagonal line: 100pt horizontal, 100pt vertical (45 degrees)
        let line = ExtractedLine::new((0.0, 0.0), (100.0, 100.0), 1.0, (0, 0, 0, 255));
        assert!(!line.is_horizontal);
        assert!(!line.is_vertical);
        assert!(line.is_diagonal());
        // Length should be ~141.4 (sqrt(2) * 100)
        let expected_length = (2.0f32).sqrt() * 100.0;
        assert!((line.length() - expected_length).abs() < 0.1);
        // Angle should be 45 degrees
        let angle_deg = line.angle_degrees();
        assert!((angle_deg - 45.0).abs() < 0.1);
    }

    #[test]
    fn test_extracted_line_bounds() {
        let line = ExtractedLine::new((100.0, 50.0), (200.0, 150.0), 1.0, (0, 0, 0, 255));
        let bounds = line.bounds();
        assert_eq!(bounds.0, 100.0); // left
        assert_eq!(bounds.1, 50.0); // bottom
        assert_eq!(bounds.2, 200.0); // right
        assert_eq!(bounds.3, 150.0); // top
    }

    #[test]
    fn test_extracted_line_visibility() {
        let visible = ExtractedLine::new((0.0, 0.0), (100.0, 0.0), 1.0, (0, 0, 0, 255));
        assert!(visible.is_visible());

        let transparent = ExtractedLine::new((0.0, 0.0), (100.0, 0.0), 1.0, (0, 0, 0, 0));
        assert!(!transparent.is_visible());
    }

    #[test]
    fn test_extracted_line_near_horizontal() {
        // Line with small vertical component (should still be considered horizontal)
        // dx = 100, dy = 10 -> ratio = 10:1 > 5:1 threshold
        let line = ExtractedLine::new((0.0, 0.0), (100.0, 10.0), 1.0, (0, 0, 0, 255));
        assert!(line.is_horizontal);
        assert!(!line.is_vertical);
    }

    #[test]
    fn test_extracted_line_near_vertical() {
        // Line with small horizontal component (should still be considered vertical)
        // dx = 10, dy = 100 -> ratio = 10:1 > 5:1 threshold
        let line = ExtractedLine::new((0.0, 0.0), (10.0, 100.0), 1.0, (0, 0, 0, 255));
        assert!(!line.is_horizontal);
        assert!(line.is_vertical);
    }

    #[test]
    fn test_extracted_line_color() {
        let line = ExtractedLine::new((0.0, 0.0), (100.0, 0.0), 2.5, (128, 64, 32, 200));
        assert_eq!(line.color, (128, 64, 32, 200));
        assert_eq!(line.thickness, 2.5);
    }

    // ==================== COLORED REGION TESTS ====================

    #[test]
    fn test_colored_region_basic() {
        let region = ColoredRegion::new(
            (10.0, 20.0, 110.0, 120.0),
            Some((255, 0, 0, 255)),
            Some((0, 0, 0, 255)),
            true,
        );

        assert_eq!(region.bounds, (10.0, 20.0, 110.0, 120.0));
        assert_eq!(region.fill_color, Some((255, 0, 0, 255)));
        assert_eq!(region.stroke_color, Some((0, 0, 0, 255)));
        assert!(region.is_behind_text);
    }

    #[test]
    fn test_colored_region_dimensions() {
        let region = ColoredRegion::new((0.0, 0.0, 100.0, 50.0), Some((0, 0, 0, 255)), None, false);

        assert_eq!(region.width(), 100.0);
        assert_eq!(region.height(), 50.0);
        assert_eq!(region.area(), 5000.0);
        assert_eq!(region.center(), (50.0, 25.0));
        assert_eq!(region.aspect_ratio(), 2.0);
    }

    #[test]
    fn test_colored_region_visibility() {
        // Visible filled region
        let filled = ColoredRegion::new(
            (0.0, 0.0, 100.0, 100.0),
            Some((255, 0, 0, 255)),
            None,
            false,
        );
        assert!(filled.is_visible());
        assert!(filled.is_filled());
        assert!(!filled.is_stroked());

        // Visible stroked region
        let stroked =
            ColoredRegion::new((0.0, 0.0, 100.0, 100.0), None, Some((0, 0, 0, 255)), false);
        assert!(stroked.is_visible());
        assert!(!stroked.is_filled());
        assert!(stroked.is_stroked());

        // Transparent fill (invisible)
        let transparent = ColoredRegion::new(
            (0.0, 0.0, 100.0, 100.0),
            Some((255, 0, 0, 0)), // alpha = 0
            None,
            false,
        );
        assert!(!transparent.is_visible());

        // No fill or stroke (invisible)
        let empty = ColoredRegion::new((0.0, 0.0, 100.0, 100.0), None, None, false);
        assert!(!empty.is_visible());
    }

    #[test]
    fn test_colored_region_colors() {
        // White fill
        let white = ColoredRegion::new(
            (0.0, 0.0, 100.0, 100.0),
            Some((255, 255, 255, 255)),
            None,
            false,
        );
        assert!(white.is_white_fill());
        assert!(white.is_light_fill());
        assert!(!white.is_dark_fill());

        // Near-white fill
        let near_white = ColoredRegion::new(
            (0.0, 0.0, 100.0, 100.0),
            Some((250, 252, 251, 255)),
            None,
            false,
        );
        assert!(near_white.is_white_fill());

        // Light fill (yellow)
        let light = ColoredRegion::new(
            (0.0, 0.0, 100.0, 100.0),
            Some((255, 255, 200, 255)),
            None,
            false,
        );
        assert!(!light.is_white_fill());
        assert!(light.is_light_fill());
        assert!(!light.is_dark_fill());

        // Dark fill (black)
        let dark = ColoredRegion::new(
            (0.0, 0.0, 100.0, 100.0),
            Some((10, 10, 10, 255)),
            None,
            false,
        );
        assert!(!dark.is_white_fill());
        assert!(!dark.is_light_fill());
        assert!(dark.is_dark_fill());
    }

    #[test]
    fn test_colored_region_contains_point() {
        let region = ColoredRegion::new(
            (10.0, 20.0, 110.0, 120.0),
            Some((0, 0, 0, 255)),
            None,
            false,
        );

        // Inside
        assert!(region.contains_point(50.0, 70.0));
        assert!(region.contains_point(10.0, 20.0)); // corner
        assert!(region.contains_point(110.0, 120.0)); // opposite corner

        // Outside
        assert!(!region.contains_point(0.0, 0.0));
        assert!(!region.contains_point(150.0, 70.0));
        assert!(!region.contains_point(50.0, 150.0));
    }

    #[test]
    fn test_colored_region_overlap() {
        let region1 =
            ColoredRegion::new((0.0, 0.0, 100.0, 100.0), Some((0, 0, 0, 255)), None, false);

        // Overlapping region
        let overlap = ColoredRegion::new(
            (50.0, 50.0, 150.0, 150.0),
            Some((0, 0, 0, 255)),
            None,
            false,
        );
        assert!(region1.overlaps(&overlap));
        assert!(overlap.overlaps(&region1));

        // Non-overlapping (to the right)
        let no_overlap_right = ColoredRegion::new(
            (150.0, 0.0, 250.0, 100.0),
            Some((0, 0, 0, 255)),
            None,
            false,
        );
        assert!(!region1.overlaps(&no_overlap_right));

        // Non-overlapping (above)
        let no_overlap_above = ColoredRegion::new(
            (0.0, 150.0, 100.0, 250.0),
            Some((0, 0, 0, 255)),
            None,
            false,
        );
        assert!(!region1.overlaps(&no_overlap_above));
    }

    #[test]
    fn test_colored_region_contains_other() {
        let outer = ColoredRegion::new((0.0, 0.0, 100.0, 100.0), Some((0, 0, 0, 255)), None, false);

        // Contained region
        let inner = ColoredRegion::new((10.0, 10.0, 90.0, 90.0), Some((0, 0, 0, 255)), None, false);
        assert!(outer.contains(&inner));
        assert!(!inner.contains(&outer));

        // Not contained (extends beyond)
        let extends =
            ColoredRegion::new((10.0, 10.0, 110.0, 90.0), Some((0, 0, 0, 255)), None, false);
        assert!(!outer.contains(&extends));
    }

    #[test]
    fn test_colored_region_stripes() {
        // Horizontal stripe (wide and short)
        let h_stripe = ColoredRegion::new(
            (0.0, 0.0, 600.0, 50.0), // 600:50 = 12:1 > 5:1
            Some((0, 0, 0, 255)),
            None,
            false,
        );
        assert!(h_stripe.is_horizontal_stripe());
        assert!(!h_stripe.is_vertical_stripe());

        // Vertical stripe (tall and narrow)
        let v_stripe = ColoredRegion::new(
            (0.0, 0.0, 50.0, 600.0), // 600:50 = 12:1 > 5:1
            Some((0, 0, 0, 255)),
            None,
            false,
        );
        assert!(!v_stripe.is_horizontal_stripe());
        assert!(v_stripe.is_vertical_stripe());

        // Square (neither stripe)
        let square =
            ColoredRegion::new((0.0, 0.0, 100.0, 100.0), Some((0, 0, 0, 255)), None, false);
        assert!(!square.is_horizontal_stripe());
        assert!(!square.is_vertical_stripe());
    }

    #[test]
    fn test_colored_region_no_fill() {
        // Region with no fill colors (edge case)
        let no_fill = ColoredRegion::new((0.0, 0.0, 100.0, 100.0), None, None, false);
        assert!(!no_fill.is_white_fill());
        assert!(!no_fill.is_light_fill());
        assert!(!no_fill.is_dark_fill());
    }
}
