//! PDF destination support
//!
//! Provides navigation destination information (view type, location, zoom).
//!
//! Destinations are used by bookmarks, links, and actions to specify
//! where in a document to navigate and how to display the target page.

use pdfium_sys::*;

/// A destination in a PDF document.
///
/// Destinations specify a page, view type, and optional location/zoom.
pub struct PdfDestination {
    handle: FPDF_DEST,
    doc_handle: FPDF_DOCUMENT,
}

impl PdfDestination {
    /// Create a new destination from handles.
    pub(crate) fn new(handle: FPDF_DEST, doc_handle: FPDF_DOCUMENT) -> Option<Self> {
        if handle.is_null() {
            None
        } else {
            Some(Self { handle, doc_handle })
        }
    }

    /// Get the raw handle.
    pub fn handle(&self) -> FPDF_DEST {
        self.handle
    }

    /// Get the destination page index (0-based).
    ///
    /// Returns None if the destination is invalid.
    pub fn page_index(&self) -> Option<usize> {
        let index = unsafe { FPDFDest_GetDestPageIndex(self.doc_handle, self.handle) };
        if index < 0 {
            None
        } else {
            Some(index as usize)
        }
    }

    /// Get the destination view type.
    ///
    /// Returns the view type and the number of explicit parameters.
    pub fn view(&self) -> Option<(DestViewType, usize)> {
        let mut num_params: u64 = 0;
        let mut params = [0.0f32; 4];

        let view_type =
            unsafe { FPDFDest_GetView(self.handle, &mut num_params, params.as_mut_ptr()) };

        let dest_view = DestViewType::from(view_type);
        Some((dest_view, num_params as usize))
    }

    /// Get the full location in page (X, Y, zoom).
    ///
    /// Returns (has_x, has_y, has_zoom, x, y, zoom).
    /// When `has_*` is false, the corresponding coordinate is not specified
    /// and the current view position/zoom should be maintained.
    pub fn location_in_page(&self) -> DestLocation {
        let mut has_x = 0i32;
        let mut has_y = 0i32;
        let mut has_zoom = 0i32;
        let mut x = 0.0f32;
        let mut y = 0.0f32;
        let mut zoom = 0.0f32;

        let ok = unsafe {
            FPDFDest_GetLocationInPage(
                self.handle,
                &mut has_x,
                &mut has_y,
                &mut has_zoom,
                &mut x,
                &mut y,
                &mut zoom,
            )
        };

        if ok != 0 {
            DestLocation {
                x: if has_x != 0 { Some(x) } else { None },
                y: if has_y != 0 { Some(y) } else { None },
                zoom: if has_zoom != 0 { Some(zoom) } else { None },
            }
        } else {
            DestLocation {
                x: None,
                y: None,
                zoom: None,
            }
        }
    }
}

/// Destination view type (how to display the target page).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DestViewType {
    /// Unknown/invalid view type
    Unknown,
    /// XYZ: Position at (left, top) with zoom factor
    XYZ,
    /// Fit: Fit entire page in window
    Fit,
    /// FitH: Fit page width, position at top
    FitH,
    /// FitV: Fit page height, position at left
    FitV,
    /// FitR: Fit rectangle (left, bottom, right, top)
    FitR,
    /// FitB: Fit bounding box in window
    FitB,
    /// FitBH: Fit bounding box width, position at top
    FitBH,
    /// FitBV: Fit bounding box height, position at left
    FitBV,
}

impl From<u64> for DestViewType {
    fn from(value: u64) -> Self {
        match value {
            1 => DestViewType::XYZ,
            2 => DestViewType::Fit,
            3 => DestViewType::FitH,
            4 => DestViewType::FitV,
            5 => DestViewType::FitR,
            6 => DestViewType::FitB,
            7 => DestViewType::FitBH,
            8 => DestViewType::FitBV,
            _ => DestViewType::Unknown,
        }
    }
}

/// Location in page with optional X, Y, and zoom values.
#[derive(Debug, Clone, Copy)]
pub struct DestLocation {
    /// X position in page coordinates (None = keep current)
    pub x: Option<f32>,
    /// Y position in page coordinates (None = keep current)
    pub y: Option<f32>,
    /// Zoom factor (None = keep current, 0 = inherit)
    pub zoom: Option<f32>,
}

impl DestLocation {
    /// Check if any location is specified.
    pub fn is_specified(&self) -> bool {
        self.x.is_some() || self.y.is_some() || self.zoom.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dest_view_type() {
        assert_eq!(DestViewType::from(0), DestViewType::Unknown);
        assert_eq!(DestViewType::from(1), DestViewType::XYZ);
        assert_eq!(DestViewType::from(2), DestViewType::Fit);
        assert_eq!(DestViewType::from(3), DestViewType::FitH);
        assert_eq!(DestViewType::from(4), DestViewType::FitV);
        assert_eq!(DestViewType::from(5), DestViewType::FitR);
        assert_eq!(DestViewType::from(6), DestViewType::FitB);
        assert_eq!(DestViewType::from(7), DestViewType::FitBH);
        assert_eq!(DestViewType::from(8), DestViewType::FitBV);
        assert_eq!(DestViewType::from(99), DestViewType::Unknown);
    }

    #[test]
    fn test_dest_location() {
        let loc = DestLocation {
            x: Some(100.0),
            y: Some(200.0),
            zoom: Some(1.5),
        };
        assert!(loc.is_specified());
        assert_eq!(loc.x, Some(100.0));
        assert_eq!(loc.y, Some(200.0));
        assert_eq!(loc.zoom, Some(1.5));

        let empty = DestLocation {
            x: None,
            y: None,
            zoom: None,
        };
        assert!(!empty.is_specified());
    }
}
