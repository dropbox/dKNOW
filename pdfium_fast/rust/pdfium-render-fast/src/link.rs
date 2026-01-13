//! PDF link extraction support
//!
//! Provides extraction of web links from PDF pages.

use crate::error::{PdfError, Result};
use pdfium_sys::*;

/// A web link found in a PDF page.
#[derive(Debug, Clone)]
pub struct PdfLink {
    /// The URL of the link.
    pub url: String,
    /// The bounding rectangles of the link (may span multiple lines).
    pub rects: Vec<LinkRect>,
}

/// A bounding rectangle for a link.
#[derive(Debug, Clone, Copy)]
pub struct LinkRect {
    /// Left edge in page coordinates.
    pub left: f64,
    /// Top edge in page coordinates.
    pub top: f64,
    /// Right edge in page coordinates.
    pub right: f64,
    /// Bottom edge in page coordinates.
    pub bottom: f64,
}

impl LinkRect {
    /// Width of the rectangle.
    pub fn width(&self) -> f64 {
        self.right - self.left
    }

    /// Height of the rectangle.
    pub fn height(&self) -> f64 {
        self.top - self.bottom
    }
}

/// Container for web links extracted from a page.
pub struct PdfPageLinks {
    handle: FPDF_PAGELINK,
}

impl PdfPageLinks {
    /// Create from a text page handle.
    pub(crate) fn from_text_page(text_handle: FPDF_TEXTPAGE) -> Result<Self> {
        let handle = unsafe { FPDFLink_LoadWebLinks(text_handle) };
        if handle.is_null() {
            return Err(PdfError::LinkExtractionFailed {
                reason: "Failed to load web links".to_string(),
            });
        }
        Ok(Self { handle })
    }

    /// Get the number of web links on the page.
    pub fn count(&self) -> usize {
        let count = unsafe { FPDFLink_CountWebLinks(self.handle) };
        if count < 0 {
            0
        } else {
            count as usize
        }
    }

    /// Get a specific link by index.
    pub fn get(&self, index: usize) -> Option<PdfLink> {
        if index >= self.count() {
            return None;
        }
        let index = index as i32;

        // Get URL
        let url = self.get_url(index)?;

        // Get rectangles
        let rect_count = unsafe { FPDFLink_CountRects(self.handle, index) };
        let mut rects = Vec::new();

        for i in 0..rect_count {
            let mut left = 0.0f64;
            let mut top = 0.0f64;
            let mut right = 0.0f64;
            let mut bottom = 0.0f64;

            let success = unsafe {
                FPDFLink_GetRect(
                    self.handle,
                    index,
                    i,
                    &mut left,
                    &mut top,
                    &mut right,
                    &mut bottom,
                )
            };

            if success != 0 {
                rects.push(LinkRect {
                    left,
                    top,
                    right,
                    bottom,
                });
            }
        }

        Some(PdfLink { url, rects })
    }

    fn get_url(&self, index: i32) -> Option<String> {
        unsafe {
            // First call to get buffer size
            let size = FPDFLink_GetURL(self.handle, index, std::ptr::null_mut(), 0);
            if size == 0 {
                return None;
            }

            // Allocate buffer (size includes null terminator, in unsigned shorts)
            let mut buffer: Vec<u16> = vec![0; size as usize];
            FPDFLink_GetURL(self.handle, index, buffer.as_mut_ptr(), size);

            // Remove trailing null
            while buffer.last() == Some(&0) {
                buffer.pop();
            }

            String::from_utf16(&buffer).ok()
        }
    }

    /// Get all links on the page.
    pub fn all(&self) -> Vec<PdfLink> {
        (0..self.count()).filter_map(|i| self.get(i)).collect()
    }

    /// Iterate over all links.
    pub fn iter(&self) -> PdfLinkIter<'_> {
        PdfLinkIter {
            links: self,
            index: 0,
        }
    }
}

impl Drop for PdfPageLinks {
    fn drop(&mut self) {
        unsafe {
            FPDFLink_CloseWebLinks(self.handle);
        }
    }
}

/// Iterator over page links.
pub struct PdfLinkIter<'a> {
    links: &'a PdfPageLinks,
    index: usize,
}

impl<'a> Iterator for PdfLinkIter<'a> {
    type Item = PdfLink;

    fn next(&mut self) -> Option<Self::Item> {
        let link = self.links.get(self.index)?;
        self.index += 1;
        Some(link)
    }
}

impl<'a> IntoIterator for &'a PdfPageLinks {
    type Item = PdfLink;
    type IntoIter = PdfLinkIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_link_rect() {
        let rect = LinkRect {
            left: 10.0,
            top: 100.0,
            right: 200.0,
            bottom: 80.0,
        };
        assert_eq!(rect.width(), 190.0);
        assert_eq!(rect.height(), 20.0);
    }
}
