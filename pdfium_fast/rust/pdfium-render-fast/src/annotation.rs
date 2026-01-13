//! PDF annotation support
//!
//! This module provides access to PDF annotations (highlights, notes, links, etc.).
//! Supports both reading and creating/modifying annotations.
//!
//! # Reading Annotations
//!
//! ```no_run
//! use pdfium_render_fast::Pdfium;
//!
//! let pdfium = Pdfium::new()?;
//! let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
//! let page = doc.page(0)?;
//!
//! // Iterate over annotations
//! for annot in page.annotations() {
//!     println!("Annotation type: {:?}", annot.annotation_type());
//!     if let Some(contents) = annot.contents() {
//!         println!("  Contents: {}", contents);
//!     }
//! }
//! # Ok::<(), pdfium_render_fast::PdfError>(())
//! ```
//!
//! # Creating Annotations
//!
//! ```no_run
//! use pdfium_render_fast::{Pdfium, PdfAnnotationType, AnnotRect, AnnotColor};
//!
//! let pdfium = Pdfium::new()?;
//! let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
//! let page = doc.page(0)?;
//!
//! // Create a highlight annotation via annotations collection
//! let mut annotations = page.annotations();
//! if let Some(mut annot) = annotations.create(PdfAnnotationType::Highlight) {
//!     annot.set_rect(&AnnotRect {
//!         left: 100.0,
//!         top: 500.0,
//!         right: 200.0,
//!         bottom: 480.0,
//!     })?;
//!     annot.set_color(&AnnotColor::yellow())?;
//! }
//! # Ok::<(), pdfium_render_fast::PdfError>(())
//! ```

use crate::error::{PdfError, Result};
use pdfium_sys::*;
use std::ffi::CString;

/// Types of PDF annotations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum PdfAnnotationType {
    /// Unknown annotation type
    Unknown = FPDF_ANNOT_UNKNOWN,
    /// Text note annotation (sticky note)
    Text = FPDF_ANNOT_TEXT,
    /// Link annotation
    Link = FPDF_ANNOT_LINK,
    /// Free text annotation (text box)
    FreeText = FPDF_ANNOT_FREETEXT,
    /// Line annotation
    Line = FPDF_ANNOT_LINE,
    /// Square/rectangle annotation
    Square = FPDF_ANNOT_SQUARE,
    /// Circle/ellipse annotation
    Circle = FPDF_ANNOT_CIRCLE,
    /// Polygon annotation
    Polygon = FPDF_ANNOT_POLYGON,
    /// Polyline annotation
    Polyline = FPDF_ANNOT_POLYLINE,
    /// Highlight annotation
    Highlight = FPDF_ANNOT_HIGHLIGHT,
    /// Underline annotation
    Underline = FPDF_ANNOT_UNDERLINE,
    /// Squiggly underline annotation
    Squiggly = FPDF_ANNOT_SQUIGGLY,
    /// Strikeout annotation
    Strikeout = FPDF_ANNOT_STRIKEOUT,
    /// Stamp annotation
    Stamp = FPDF_ANNOT_STAMP,
    /// Caret annotation (insertion point)
    Caret = FPDF_ANNOT_CARET,
    /// Ink (freehand) annotation
    Ink = FPDF_ANNOT_INK,
    /// Popup annotation
    Popup = FPDF_ANNOT_POPUP,
    /// File attachment annotation
    FileAttachment = FPDF_ANNOT_FILEATTACHMENT,
    /// Sound annotation
    Sound = FPDF_ANNOT_SOUND,
    /// Movie annotation
    Movie = FPDF_ANNOT_MOVIE,
    /// Widget annotation (form field)
    Widget = FPDF_ANNOT_WIDGET,
    /// Screen annotation
    Screen = FPDF_ANNOT_SCREEN,
    /// Printer mark annotation
    PrinterMark = FPDF_ANNOT_PRINTERMARK,
    /// Trap network annotation
    TrapNet = FPDF_ANNOT_TRAPNET,
    /// Watermark annotation
    Watermark = FPDF_ANNOT_WATERMARK,
    /// 3D annotation
    ThreeD = FPDF_ANNOT_THREED,
    /// Rich media annotation
    RichMedia = FPDF_ANNOT_RICHMEDIA,
    /// XFA widget annotation
    XfaWidget = FPDF_ANNOT_XFAWIDGET,
    /// Redaction annotation
    Redact = FPDF_ANNOT_REDACT,
}

impl PdfAnnotationType {
    /// Create annotation type from raw PDFium value.
    pub fn from_raw(value: u32) -> Self {
        match value {
            FPDF_ANNOT_TEXT => Self::Text,
            FPDF_ANNOT_LINK => Self::Link,
            FPDF_ANNOT_FREETEXT => Self::FreeText,
            FPDF_ANNOT_LINE => Self::Line,
            FPDF_ANNOT_SQUARE => Self::Square,
            FPDF_ANNOT_CIRCLE => Self::Circle,
            FPDF_ANNOT_POLYGON => Self::Polygon,
            FPDF_ANNOT_POLYLINE => Self::Polyline,
            FPDF_ANNOT_HIGHLIGHT => Self::Highlight,
            FPDF_ANNOT_UNDERLINE => Self::Underline,
            FPDF_ANNOT_SQUIGGLY => Self::Squiggly,
            FPDF_ANNOT_STRIKEOUT => Self::Strikeout,
            FPDF_ANNOT_STAMP => Self::Stamp,
            FPDF_ANNOT_CARET => Self::Caret,
            FPDF_ANNOT_INK => Self::Ink,
            FPDF_ANNOT_POPUP => Self::Popup,
            FPDF_ANNOT_FILEATTACHMENT => Self::FileAttachment,
            FPDF_ANNOT_SOUND => Self::Sound,
            FPDF_ANNOT_MOVIE => Self::Movie,
            FPDF_ANNOT_WIDGET => Self::Widget,
            FPDF_ANNOT_SCREEN => Self::Screen,
            FPDF_ANNOT_PRINTERMARK => Self::PrinterMark,
            FPDF_ANNOT_TRAPNET => Self::TrapNet,
            FPDF_ANNOT_WATERMARK => Self::Watermark,
            FPDF_ANNOT_THREED => Self::ThreeD,
            FPDF_ANNOT_RICHMEDIA => Self::RichMedia,
            FPDF_ANNOT_XFAWIDGET => Self::XfaWidget,
            FPDF_ANNOT_REDACT => Self::Redact,
            _ => Self::Unknown,
        }
    }

    /// Check if this is a markup annotation (highlight, underline, etc.).
    pub fn is_markup(&self) -> bool {
        matches!(
            self,
            Self::Highlight | Self::Underline | Self::Squiggly | Self::Strikeout
        )
    }

    /// Check if this is a shape annotation (line, square, circle, etc.).
    pub fn is_shape(&self) -> bool {
        matches!(
            self,
            Self::Line | Self::Square | Self::Circle | Self::Polygon | Self::Polyline
        )
    }

    /// Convert to raw PDFium value.
    pub fn to_raw(&self) -> FPDF_ANNOTATION_SUBTYPE {
        match self {
            Self::Unknown => FPDF_ANNOT_UNKNOWN as i32,
            Self::Text => FPDF_ANNOT_TEXT as i32,
            Self::Link => FPDF_ANNOT_LINK as i32,
            Self::FreeText => FPDF_ANNOT_FREETEXT as i32,
            Self::Line => FPDF_ANNOT_LINE as i32,
            Self::Square => FPDF_ANNOT_SQUARE as i32,
            Self::Circle => FPDF_ANNOT_CIRCLE as i32,
            Self::Polygon => FPDF_ANNOT_POLYGON as i32,
            Self::Polyline => FPDF_ANNOT_POLYLINE as i32,
            Self::Highlight => FPDF_ANNOT_HIGHLIGHT as i32,
            Self::Underline => FPDF_ANNOT_UNDERLINE as i32,
            Self::Squiggly => FPDF_ANNOT_SQUIGGLY as i32,
            Self::Strikeout => FPDF_ANNOT_STRIKEOUT as i32,
            Self::Stamp => FPDF_ANNOT_STAMP as i32,
            Self::Caret => FPDF_ANNOT_CARET as i32,
            Self::Ink => FPDF_ANNOT_INK as i32,
            Self::Popup => FPDF_ANNOT_POPUP as i32,
            Self::FileAttachment => FPDF_ANNOT_FILEATTACHMENT as i32,
            Self::Sound => FPDF_ANNOT_SOUND as i32,
            Self::Movie => FPDF_ANNOT_MOVIE as i32,
            Self::Widget => FPDF_ANNOT_WIDGET as i32,
            Self::Screen => FPDF_ANNOT_SCREEN as i32,
            Self::PrinterMark => FPDF_ANNOT_PRINTERMARK as i32,
            Self::TrapNet => FPDF_ANNOT_TRAPNET as i32,
            Self::Watermark => FPDF_ANNOT_WATERMARK as i32,
            Self::ThreeD => FPDF_ANNOT_THREED as i32,
            Self::RichMedia => FPDF_ANNOT_RICHMEDIA as i32,
            Self::XfaWidget => FPDF_ANNOT_XFAWIDGET as i32,
            Self::Redact => FPDF_ANNOT_REDACT as i32,
        }
    }

    /// Check if this annotation subtype is supported for creation.
    pub fn is_supported_for_creation(&self) -> bool {
        let raw = self.to_raw();
        unsafe { FPDFAnnot_IsSupportedSubtype(raw) != 0 }
    }
}

/// Rectangle representing annotation bounds (in page coordinates).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnnotRect {
    /// Left edge (x-coordinate)
    pub left: f32,
    /// Top edge (y-coordinate)
    pub top: f32,
    /// Right edge (x-coordinate)
    pub right: f32,
    /// Bottom edge (y-coordinate)
    pub bottom: f32,
}

impl AnnotRect {
    /// Get the width of the rectangle.
    pub fn width(&self) -> f32 {
        self.right - self.left
    }

    /// Get the height of the rectangle.
    pub fn height(&self) -> f32 {
        self.top - self.bottom
    }
}

/// RGBA color for annotations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnnotColor {
    /// Red component (0-255)
    pub r: u8,
    /// Green component (0-255)
    pub g: u8,
    /// Blue component (0-255)
    pub b: u8,
    /// Alpha component (0-255, where 255 is opaque)
    pub a: u8,
}

impl Default for AnnotColor {
    fn default() -> Self {
        Self {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        }
    }
}

impl AnnotColor {
    /// Create a new color.
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Create an opaque color (alpha = 255).
    pub fn opaque(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// Yellow highlight color.
    pub fn yellow() -> Self {
        Self::new(255, 255, 0, 128)
    }

    /// Red color.
    pub fn red() -> Self {
        Self::opaque(255, 0, 0)
    }

    /// Green color.
    pub fn green() -> Self {
        Self::opaque(0, 255, 0)
    }

    /// Blue color.
    pub fn blue() -> Self {
        Self::opaque(0, 0, 255)
    }
}

/// Annotation flags (from PDF Reference table 8.16).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnnotFlags(pub u32);

impl AnnotFlags {
    /// No flags set.
    pub const NONE: AnnotFlags = AnnotFlags(FPDF_ANNOT_FLAG_NONE);
    /// Do not display if no appearance stream.
    pub const INVISIBLE: AnnotFlags = AnnotFlags(FPDF_ANNOT_FLAG_INVISIBLE);
    /// Do not display or print.
    pub const HIDDEN: AnnotFlags = AnnotFlags(FPDF_ANNOT_FLAG_HIDDEN);
    /// Print the annotation.
    pub const PRINT: AnnotFlags = AnnotFlags(FPDF_ANNOT_FLAG_PRINT);
    /// Do not scale when page is zoomed.
    pub const NO_ZOOM: AnnotFlags = AnnotFlags(FPDF_ANNOT_FLAG_NOZOOM);
    /// Do not rotate when page is rotated.
    pub const NO_ROTATE: AnnotFlags = AnnotFlags(FPDF_ANNOT_FLAG_NOROTATE);
    /// Do not display but allow printing.
    pub const NO_VIEW: AnnotFlags = AnnotFlags(FPDF_ANNOT_FLAG_NOVIEW);
    /// Do not allow user interaction.
    pub const READ_ONLY: AnnotFlags = AnnotFlags(FPDF_ANNOT_FLAG_READONLY);
    /// Do not allow deletion or property modification.
    pub const LOCKED: AnnotFlags = AnnotFlags(FPDF_ANNOT_FLAG_LOCKED);
    /// Toggle NO_VIEW flag.
    pub const TOGGLE_NO_VIEW: AnnotFlags = AnnotFlags(FPDF_ANNOT_FLAG_TOGGLENOVIEW);

    /// Check if a flag is set.
    pub fn contains(&self, flag: AnnotFlags) -> bool {
        (self.0 & flag.0) != 0
    }

    /// Add a flag.
    pub fn with(&self, flag: AnnotFlags) -> AnnotFlags {
        AnnotFlags(self.0 | flag.0)
    }

    /// Remove a flag.
    pub fn without(&self, flag: AnnotFlags) -> AnnotFlags {
        AnnotFlags(self.0 & !flag.0)
    }
}

impl Default for AnnotFlags {
    fn default() -> Self {
        Self::NONE
    }
}

/// Annotation border configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnnotBorder {
    /// Horizontal corner radius.
    pub horizontal_radius: f32,
    /// Vertical corner radius.
    pub vertical_radius: f32,
    /// Border width.
    pub width: f32,
}

impl Default for AnnotBorder {
    fn default() -> Self {
        Self {
            horizontal_radius: 0.0,
            vertical_radius: 0.0,
            width: 1.0,
        }
    }
}

/// Appearance mode for annotation appearance streams.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum AppearanceMode {
    /// Normal appearance.
    Normal = FPDF_ANNOT_APPEARANCEMODE_NORMAL as i32,
    /// Rollover (hover) appearance.
    Rollover = FPDF_ANNOT_APPEARANCEMODE_ROLLOVER as i32,
    /// Down (pressed) appearance.
    Down = FPDF_ANNOT_APPEARANCEMODE_DOWN as i32,
}

/// A PDF annotation.
///
/// Annotations are interactive elements like highlights, notes, and links.
pub struct PdfAnnotation {
    handle: FPDF_ANNOTATION,
    #[allow(dead_code)]
    page_handle: FPDF_PAGE,
    index: i32,
}

impl PdfAnnotation {
    pub(crate) fn new(handle: FPDF_ANNOTATION, page_handle: FPDF_PAGE, index: i32) -> Self {
        Self {
            handle,
            page_handle,
            index,
        }
    }

    /// Get the raw annotation handle.
    pub fn handle(&self) -> FPDF_ANNOTATION {
        self.handle
    }

    /// Get the annotation index on this page.
    pub fn index(&self) -> i32 {
        self.index
    }

    /// Get the annotation type (highlight, link, text note, etc.).
    pub fn annotation_type(&self) -> PdfAnnotationType {
        let raw = unsafe { FPDFAnnot_GetSubtype(self.handle) } as u32;
        PdfAnnotationType::from_raw(raw)
    }

    /// Get the bounding rectangle of the annotation.
    pub fn rect(&self) -> Result<AnnotRect> {
        let mut rect = FS_RECTF {
            left: 0.0,
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
        };

        let success = unsafe { FPDFAnnot_GetRect(self.handle, &mut rect) };
        if success == 0 {
            return Err(PdfError::InvalidData {
                reason: "Failed to get annotation rect".to_string(),
            });
        }

        Ok(AnnotRect {
            left: rect.left,
            top: rect.top,
            right: rect.right,
            bottom: rect.bottom,
        })
    }

    /// Get the annotation color.
    ///
    /// Returns the fill color for shape annotations or the main color for others.
    pub fn color(&self) -> Option<AnnotColor> {
        let mut r: u32 = 0;
        let mut g: u32 = 0;
        let mut b: u32 = 0;
        let mut a: u32 = 0;

        // Try to get stroke color first (FPDFANNOT_COLORTYPE_Color = 0)
        let success = unsafe {
            FPDFAnnot_GetColor(
                self.handle,
                0, // FPDFANNOT_COLORTYPE_Color
                &mut r,
                &mut g,
                &mut b,
                &mut a,
            )
        };

        if success != 0 {
            Some(AnnotColor {
                r: r as u8,
                g: g as u8,
                b: b as u8,
                a: a as u8,
            })
        } else {
            None
        }
    }

    /// Check if a key exists in the annotation dictionary.
    pub fn has_key(&self, key: &str) -> bool {
        let key_cstr = std::ffi::CString::new(key).unwrap();
        let result = unsafe { FPDFAnnot_HasKey(self.handle, key_cstr.as_ptr()) };
        result != 0
    }

    /// Get a string value from the annotation dictionary.
    pub fn get_string_value(&self, key: &str) -> Option<String> {
        let key_cstr = std::ffi::CString::new(key).ok()?;

        // First get the length
        let len = unsafe {
            FPDFAnnot_GetStringValue(self.handle, key_cstr.as_ptr(), std::ptr::null_mut(), 0)
        };

        if len == 0 {
            return None;
        }

        // Allocate buffer for UTF-16LE (2 bytes per character)
        let mut buffer: Vec<u16> = vec![0; len as usize / 2 + 1];
        let actual_len = unsafe {
            FPDFAnnot_GetStringValue(self.handle, key_cstr.as_ptr(), buffer.as_mut_ptr(), len)
        };

        if actual_len == 0 {
            return None;
        }

        // Convert UTF-16LE to String
        let chars = actual_len as usize / 2;
        let trimmed: Vec<u16> = buffer[..chars]
            .iter()
            .copied()
            .take_while(|&c| c != 0)
            .collect();

        String::from_utf16(&trimmed).ok()
    }

    /// Get the "Contents" field of the annotation (comment text).
    pub fn contents(&self) -> Option<String> {
        self.get_string_value("Contents")
    }

    /// Get the "T" (title/author) field of the annotation.
    pub fn author(&self) -> Option<String> {
        self.get_string_value("T")
    }

    /// Get the modification date of the annotation.
    pub fn modification_date(&self) -> Option<String> {
        self.get_string_value("M")
    }

    /// Get the number of attachment points (for markup annotations).
    pub fn attachment_point_count(&self) -> usize {
        unsafe { FPDFAnnot_CountAttachmentPoints(self.handle) }
    }

    /// Check if this annotation has attachment points.
    pub fn has_attachment_points(&self) -> bool {
        unsafe { FPDFAnnot_HasAttachmentPoints(self.handle) != 0 }
    }

    /// Get an attachment point quadrilateral by index.
    ///
    /// Returns four points (x1,y1), (x2,y2), (x3,y3), (x4,y4) representing the quad.
    pub fn get_attachment_point(&self, index: usize) -> Option<[(f32, f32); 4]> {
        let mut quad = FS_QUADPOINTSF {
            x1: 0.0,
            y1: 0.0,
            x2: 0.0,
            y2: 0.0,
            x3: 0.0,
            y3: 0.0,
            x4: 0.0,
            y4: 0.0,
        };

        let success = unsafe { FPDFAnnot_GetAttachmentPoints(self.handle, index, &mut quad) };

        if success != 0 {
            Some([
                (quad.x1, quad.y1),
                (quad.x2, quad.y2),
                (quad.x3, quad.y3),
                (quad.x4, quad.y4),
            ])
        } else {
            None
        }
    }

    // ========== Modification Methods ==========

    /// Set the bounding rectangle of the annotation.
    pub fn set_rect(&mut self, rect: &AnnotRect) -> Result<()> {
        let fs_rect = FS_RECTF {
            left: rect.left,
            top: rect.top,
            right: rect.right,
            bottom: rect.bottom,
        };

        let success = unsafe { FPDFAnnot_SetRect(self.handle, &fs_rect) };
        if success == 0 {
            return Err(PdfError::InvalidData {
                reason: "Failed to set annotation rect".to_string(),
            });
        }
        Ok(())
    }

    /// Set the annotation color.
    ///
    /// This sets the stroke/border color for shape annotations or the main color for others.
    pub fn set_color(&mut self, color: &AnnotColor) -> Result<()> {
        // FPDFANNOT_COLORTYPE_Color = 0
        let success = unsafe {
            FPDFAnnot_SetColor(
                self.handle,
                0,
                color.r as u32,
                color.g as u32,
                color.b as u32,
                color.a as u32,
            )
        };
        if success == 0 {
            return Err(PdfError::InvalidData {
                reason: "Failed to set annotation color".to_string(),
            });
        }
        Ok(())
    }

    /// Set the interior (fill) color of the annotation.
    pub fn set_interior_color(&mut self, color: &AnnotColor) -> Result<()> {
        // FPDFANNOT_COLORTYPE_InteriorColor = 1
        let success = unsafe {
            FPDFAnnot_SetColor(
                self.handle,
                1,
                color.r as u32,
                color.g as u32,
                color.b as u32,
                color.a as u32,
            )
        };
        if success == 0 {
            return Err(PdfError::InvalidData {
                reason: "Failed to set annotation interior color".to_string(),
            });
        }
        Ok(())
    }

    /// Set the border characteristics.
    pub fn set_border(&mut self, border: &AnnotBorder) -> Result<()> {
        let success = unsafe {
            FPDFAnnot_SetBorder(
                self.handle,
                border.horizontal_radius,
                border.vertical_radius,
                border.width,
            )
        };
        if success == 0 {
            return Err(PdfError::InvalidData {
                reason: "Failed to set annotation border".to_string(),
            });
        }
        Ok(())
    }

    /// Get the border characteristics.
    pub fn border(&self) -> Option<AnnotBorder> {
        let mut h_radius: f32 = 0.0;
        let mut v_radius: f32 = 0.0;
        let mut width: f32 = 0.0;

        let success =
            unsafe { FPDFAnnot_GetBorder(self.handle, &mut h_radius, &mut v_radius, &mut width) };

        if success != 0 {
            Some(AnnotBorder {
                horizontal_radius: h_radius,
                vertical_radius: v_radius,
                width,
            })
        } else {
            None
        }
    }

    /// Set a string value in the annotation dictionary.
    pub fn set_string_value(&mut self, key: &str, value: &str) -> Result<()> {
        let key_cstr = CString::new(key).map_err(|_| PdfError::InvalidData {
            reason: "Invalid key string".to_string(),
        })?;

        // Convert value to UTF-16LE
        let utf16: Vec<u16> = value.encode_utf16().chain(std::iter::once(0)).collect();

        let success =
            unsafe { FPDFAnnot_SetStringValue(self.handle, key_cstr.as_ptr(), utf16.as_ptr()) };

        if success == 0 {
            return Err(PdfError::InvalidData {
                reason: format!("Failed to set annotation string value for key '{}'", key),
            });
        }
        Ok(())
    }

    /// Set the annotation flags.
    pub fn set_flags(&mut self, flags: AnnotFlags) -> Result<()> {
        let success = unsafe { FPDFAnnot_SetFlags(self.handle, flags.0 as i32) };
        if success == 0 {
            return Err(PdfError::InvalidData {
                reason: "Failed to set annotation flags".to_string(),
            });
        }
        Ok(())
    }

    /// Get the annotation flags.
    pub fn flags(&self) -> AnnotFlags {
        let raw = unsafe { FPDFAnnot_GetFlags(self.handle) };
        AnnotFlags(raw as u32)
    }

    /// Set a URI action for link annotations.
    pub fn set_uri(&mut self, uri: &str) -> Result<()> {
        let uri_cstr = CString::new(uri).map_err(|_| PdfError::InvalidData {
            reason: "Invalid URI string".to_string(),
        })?;

        let success = unsafe { FPDFAnnot_SetURI(self.handle, uri_cstr.as_ptr()) };
        if success == 0 {
            return Err(PdfError::InvalidData {
                reason: "Failed to set annotation URI".to_string(),
            });
        }
        Ok(())
    }

    /// Set the "Contents" field (comment text) of the annotation.
    pub fn set_contents(&mut self, contents: &str) -> Result<()> {
        self.set_string_value("Contents", contents)
    }

    /// Set the "T" (title/author) field of the annotation.
    pub fn set_author(&mut self, author: &str) -> Result<()> {
        self.set_string_value("T", author)
    }

    /// Set the appearance string for a specific mode.
    pub fn set_appearance(&mut self, mode: AppearanceMode, value: Option<&str>) -> Result<()> {
        let utf16: Option<Vec<u16>> =
            value.map(|v| v.encode_utf16().chain(std::iter::once(0)).collect());

        let ptr = utf16
            .as_ref()
            .map(|v| v.as_ptr())
            .unwrap_or(std::ptr::null());

        let success = unsafe { FPDFAnnot_SetAP(self.handle, mode as i32, ptr) };
        if success == 0 {
            return Err(PdfError::InvalidData {
                reason: "Failed to set annotation appearance".to_string(),
            });
        }
        Ok(())
    }

    /// Add an ink stroke to ink annotations.
    ///
    /// Returns the index of the new stroke, or an error if failed.
    pub fn add_ink_stroke(&mut self, points: &[(f32, f32)]) -> Result<i32> {
        let fs_points: Vec<FS_POINTF> = points
            .iter()
            .map(|(x, y)| FS_POINTF { x: *x, y: *y })
            .collect();

        let index =
            unsafe { FPDFAnnot_AddInkStroke(self.handle, fs_points.as_ptr(), fs_points.len()) };

        if index < 0 {
            return Err(PdfError::InvalidData {
                reason: "Failed to add ink stroke".to_string(),
            });
        }
        Ok(index)
    }

    /// Remove all ink strokes from ink annotations.
    pub fn remove_ink_list(&mut self) -> Result<()> {
        let success = unsafe { FPDFAnnot_RemoveInkList(self.handle) };
        if success == 0 {
            return Err(PdfError::InvalidData {
                reason: "Failed to remove ink list".to_string(),
            });
        }
        Ok(())
    }

    /// Append attachment points (quadpoints) for markup annotations.
    pub fn append_attachment_points(&mut self, quad: [(f32, f32); 4]) -> Result<()> {
        let quad_points = FS_QUADPOINTSF {
            x1: quad[0].0,
            y1: quad[0].1,
            x2: quad[1].0,
            y2: quad[1].1,
            x3: quad[2].0,
            y3: quad[2].1,
            x4: quad[3].0,
            y4: quad[3].1,
        };

        let success = unsafe { FPDFAnnot_AppendAttachmentPoints(self.handle, &quad_points) };
        if success == 0 {
            return Err(PdfError::InvalidData {
                reason: "Failed to append attachment points".to_string(),
            });
        }
        Ok(())
    }

    /// Set attachment points at a specific index.
    pub fn set_attachment_points(&mut self, index: usize, quad: [(f32, f32); 4]) -> Result<()> {
        let quad_points = FS_QUADPOINTSF {
            x1: quad[0].0,
            y1: quad[0].1,
            x2: quad[1].0,
            y2: quad[1].1,
            x3: quad[2].0,
            y3: quad[2].1,
            x4: quad[3].0,
            y4: quad[3].1,
        };

        let success = unsafe { FPDFAnnot_SetAttachmentPoints(self.handle, index, &quad_points) };
        if success == 0 {
            return Err(PdfError::InvalidData {
                reason: "Failed to set attachment points".to_string(),
            });
        }
        Ok(())
    }

    /// Get the number of objects in the annotation.
    pub fn object_count(&self) -> i32 {
        unsafe { FPDFAnnot_GetObjectCount(self.handle) }
    }
}

impl Drop for PdfAnnotation {
    fn drop(&mut self) {
        unsafe {
            FPDFPage_CloseAnnot(self.handle);
        }
    }
}

/// Iterator over annotations on a page.
pub struct PdfPageAnnotations {
    page_handle: FPDF_PAGE,
    count: i32,
    current: i32,
}

impl PdfPageAnnotations {
    pub(crate) fn new(page_handle: FPDF_PAGE) -> Self {
        let count = unsafe { FPDFPage_GetAnnotCount(page_handle) };
        Self {
            page_handle,
            count,
            current: 0,
        }
    }

    /// Get the total number of annotations.
    pub fn count(&self) -> i32 {
        self.count
    }

    /// Get an annotation by index.
    pub fn get(&self, index: i32) -> Option<PdfAnnotation> {
        if index < 0 || index >= self.count {
            return None;
        }

        let handle = unsafe { FPDFPage_GetAnnot(self.page_handle, index) };
        if handle.is_null() {
            return None;
        }

        Some(PdfAnnotation::new(handle, self.page_handle, index))
    }

    /// Create a new annotation of the specified type.
    ///
    /// Returns the annotation if creation was successful.
    pub fn create(&mut self, annot_type: PdfAnnotationType) -> Option<PdfAnnotation> {
        let handle = unsafe { FPDFPage_CreateAnnot(self.page_handle, annot_type.to_raw()) };
        if handle.is_null() {
            return None;
        }

        // Update count and return the new annotation
        self.count = unsafe { FPDFPage_GetAnnotCount(self.page_handle) };
        Some(PdfAnnotation::new(handle, self.page_handle, self.count - 1))
    }

    /// Remove an annotation by index.
    ///
    /// Returns true if removal was successful.
    pub fn remove(&mut self, index: i32) -> bool {
        if index < 0 || index >= self.count {
            return false;
        }

        let success = unsafe { FPDFPage_RemoveAnnot(self.page_handle, index) };
        if success != 0 {
            self.count = unsafe { FPDFPage_GetAnnotCount(self.page_handle) };
            true
        } else {
            false
        }
    }

    /// Get the raw page handle.
    pub fn page_handle(&self) -> FPDF_PAGE {
        self.page_handle
    }
}

impl Iterator for PdfPageAnnotations {
    type Item = PdfAnnotation;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.count {
            return None;
        }

        let result = self.get(self.current);
        self.current += 1;
        result
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = (self.count - self.current) as usize;
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for PdfPageAnnotations {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_annotation_type_from_raw() {
        assert_eq!(
            PdfAnnotationType::from_raw(FPDF_ANNOT_HIGHLIGHT),
            PdfAnnotationType::Highlight
        );
        assert_eq!(
            PdfAnnotationType::from_raw(FPDF_ANNOT_TEXT),
            PdfAnnotationType::Text
        );
        assert_eq!(
            PdfAnnotationType::from_raw(9999),
            PdfAnnotationType::Unknown
        );
    }

    #[test]
    fn test_annotation_type_to_raw() {
        assert_eq!(
            PdfAnnotationType::Highlight.to_raw(),
            FPDF_ANNOT_HIGHLIGHT as i32
        );
        assert_eq!(PdfAnnotationType::Text.to_raw(), FPDF_ANNOT_TEXT as i32);
        assert_eq!(PdfAnnotationType::Link.to_raw(), FPDF_ANNOT_LINK as i32);
        assert_eq!(PdfAnnotationType::Square.to_raw(), FPDF_ANNOT_SQUARE as i32);
    }

    #[test]
    fn test_annotation_type_categories() {
        assert!(PdfAnnotationType::Highlight.is_markup());
        assert!(PdfAnnotationType::Underline.is_markup());
        assert!(!PdfAnnotationType::Link.is_markup());

        assert!(PdfAnnotationType::Line.is_shape());
        assert!(PdfAnnotationType::Circle.is_shape());
        assert!(!PdfAnnotationType::Highlight.is_shape());
    }

    #[test]
    fn test_annot_rect() {
        let rect = AnnotRect {
            left: 10.0,
            top: 100.0,
            right: 50.0,
            bottom: 80.0,
        };
        assert_eq!(rect.width(), 40.0);
        assert_eq!(rect.height(), 20.0);
    }

    #[test]
    fn test_annot_color_constructors() {
        let c1 = AnnotColor::new(100, 150, 200, 128);
        assert_eq!(c1.r, 100);
        assert_eq!(c1.g, 150);
        assert_eq!(c1.b, 200);
        assert_eq!(c1.a, 128);

        let c2 = AnnotColor::opaque(50, 75, 100);
        assert_eq!(c2.a, 255);

        let yellow = AnnotColor::yellow();
        assert_eq!(yellow.r, 255);
        assert_eq!(yellow.g, 255);
        assert_eq!(yellow.b, 0);
    }

    #[test]
    fn test_annot_flags() {
        let flags = AnnotFlags::PRINT.with(AnnotFlags::LOCKED);
        assert!(flags.contains(AnnotFlags::PRINT));
        assert!(flags.contains(AnnotFlags::LOCKED));
        assert!(!flags.contains(AnnotFlags::HIDDEN));

        let reduced = flags.without(AnnotFlags::PRINT);
        assert!(!reduced.contains(AnnotFlags::PRINT));
        assert!(reduced.contains(AnnotFlags::LOCKED));
    }

    #[test]
    fn test_annot_border_default() {
        let border = AnnotBorder::default();
        assert_eq!(border.horizontal_radius, 0.0);
        assert_eq!(border.vertical_radius, 0.0);
        assert_eq!(border.width, 1.0);
    }

    #[test]
    fn test_appearance_mode() {
        assert_eq!(AppearanceMode::Normal as i32, 0);
        assert_eq!(AppearanceMode::Rollover as i32, 1);
        assert_eq!(AppearanceMode::Down as i32, 2);
    }
}
