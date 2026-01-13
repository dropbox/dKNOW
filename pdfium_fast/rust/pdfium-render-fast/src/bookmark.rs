//! PDF bookmark (outline) support
//!
//! Provides access to the document's outline/bookmark tree for navigation.

use crate::action::PdfAction;
use crate::destination::PdfDestination;
use crate::error::{PdfError, Result};
use pdfium_sys::*;

/// A bookmark (outline item) in a PDF document.
///
/// Bookmarks form a tree structure for document navigation.
/// Use `PdfDocument::bookmarks()` to get the root iterator.
#[derive(Debug)]
pub struct PdfBookmark {
    handle: FPDF_BOOKMARK,
    doc_handle: FPDF_DOCUMENT,
}

impl PdfBookmark {
    pub(crate) fn new(handle: FPDF_BOOKMARK, doc_handle: FPDF_DOCUMENT) -> Self {
        Self { handle, doc_handle }
    }

    /// Get the bookmark title.
    ///
    /// Returns the display text for this bookmark entry.
    pub fn title(&self) -> Result<String> {
        unsafe {
            // First call to get required buffer size
            let size = FPDFBookmark_GetTitle(self.handle, std::ptr::null_mut(), 0);
            if size == 0 {
                return Ok(String::new());
            }

            // Allocate buffer for UTF-16 string (size is in bytes)
            let mut buffer: Vec<u16> = vec![0; (size as usize) / 2];
            FPDFBookmark_GetTitle(
                self.handle,
                buffer.as_mut_ptr() as *mut std::ffi::c_void,
                size,
            );

            // Remove trailing null and convert to String
            while buffer.last() == Some(&0) {
                buffer.pop();
            }
            String::from_utf16(&buffer).map_err(|_| PdfError::InvalidData {
                reason: "Invalid UTF-16 in bookmark title".to_string(),
            })
        }
    }

    /// Get the destination page index for this bookmark.
    ///
    /// Returns the 0-based page index this bookmark points to,
    /// or None if the bookmark has no destination.
    pub fn dest_page_index(&self) -> Option<usize> {
        self.destination().and_then(|d| d.page_index())
    }

    /// Get the full destination for this bookmark.
    ///
    /// Returns the destination with page index, view type, and location info.
    pub fn destination(&self) -> Option<PdfDestination> {
        let dest = unsafe { FPDFBookmark_GetDest(self.doc_handle, self.handle) };
        PdfDestination::new(dest, self.doc_handle)
    }

    /// Get the action associated with this bookmark.
    ///
    /// Actions can be GoTo (navigate), URI (open link), Launch (open file), etc.
    pub fn action(&self) -> Option<PdfAction> {
        let action = unsafe { FPDFBookmark_GetAction(self.handle) };
        PdfAction::new(action, self.doc_handle)
    }

    /// Get the number of direct children of this bookmark.
    ///
    /// A negative count indicates the bookmark is closed by default.
    /// The absolute value gives the number of children.
    pub fn child_count(&self) -> i32 {
        unsafe { FPDFBookmark_GetCount(self.handle) }
    }

    /// Check if this bookmark has children.
    pub fn has_children(&self) -> bool {
        self.child_count() != 0
    }

    /// Check if this bookmark is initially open (expanded).
    pub fn is_open(&self) -> bool {
        self.child_count() > 0
    }

    /// Get an iterator over the children of this bookmark.
    pub fn children(&self) -> PdfBookmarkChildIter {
        let first_child = unsafe { FPDFBookmark_GetFirstChild(self.doc_handle, self.handle) };
        PdfBookmarkChildIter {
            current: first_child,
            doc_handle: self.doc_handle,
        }
    }

    /// Get the first child bookmark, if any.
    pub fn first_child(&self) -> Option<PdfBookmark> {
        let child = unsafe { FPDFBookmark_GetFirstChild(self.doc_handle, self.handle) };
        if child.is_null() {
            None
        } else {
            Some(PdfBookmark::new(child, self.doc_handle))
        }
    }

    /// Get the next sibling bookmark, if any.
    pub fn next_sibling(&self) -> Option<PdfBookmark> {
        let sibling = unsafe { FPDFBookmark_GetNextSibling(self.doc_handle, self.handle) };
        if sibling.is_null() {
            None
        } else {
            Some(PdfBookmark::new(sibling, self.doc_handle))
        }
    }
}

/// Iterator over bookmark children.
pub struct PdfBookmarkChildIter {
    current: FPDF_BOOKMARK,
    doc_handle: FPDF_DOCUMENT,
}

impl Iterator for PdfBookmarkChildIter {
    type Item = PdfBookmark;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current.is_null() {
            return None;
        }
        let bookmark = PdfBookmark::new(self.current, self.doc_handle);
        self.current = unsafe { FPDFBookmark_GetNextSibling(self.doc_handle, self.current) };
        Some(bookmark)
    }
}

/// Iterator over all bookmarks at the root level.
pub struct PdfBookmarkIter {
    current: FPDF_BOOKMARK,
    doc_handle: FPDF_DOCUMENT,
}

impl PdfBookmarkIter {
    pub(crate) fn new(doc_handle: FPDF_DOCUMENT) -> Self {
        let first = unsafe { FPDFBookmark_GetFirstChild(doc_handle, std::ptr::null_mut()) };
        Self {
            current: first,
            doc_handle,
        }
    }
}

impl Iterator for PdfBookmarkIter {
    type Item = PdfBookmark;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current.is_null() {
            return None;
        }
        let bookmark = PdfBookmark::new(self.current, self.doc_handle);
        self.current = unsafe { FPDFBookmark_GetNextSibling(self.doc_handle, self.current) };
        Some(bookmark)
    }
}

/// A flattened bookmark entry with depth information.
#[derive(Debug, Clone)]
pub struct FlatBookmark {
    /// The bookmark title.
    pub title: String,
    /// The destination page index (0-based), if any.
    pub page_index: Option<usize>,
    /// The depth in the tree (0 = root level).
    pub depth: usize,
}

/// Recursively collect all bookmarks into a flat list with depth info.
pub fn flatten_bookmarks(doc_handle: FPDF_DOCUMENT) -> Vec<FlatBookmark> {
    let mut result = Vec::new();
    flatten_recursive(doc_handle, std::ptr::null_mut(), 0, &mut result);
    result
}

fn flatten_recursive(
    doc_handle: FPDF_DOCUMENT,
    parent: FPDF_BOOKMARK,
    depth: usize,
    result: &mut Vec<FlatBookmark>,
) {
    let first = unsafe { FPDFBookmark_GetFirstChild(doc_handle, parent) };
    let mut current = first;

    while !current.is_null() {
        let bookmark = PdfBookmark::new(current, doc_handle);

        if let Ok(title) = bookmark.title() {
            result.push(FlatBookmark {
                title,
                page_index: bookmark.dest_page_index(),
                depth,
            });
        }

        // Recurse into children
        flatten_recursive(doc_handle, current, depth + 1, result);

        // Move to next sibling
        current = unsafe { FPDFBookmark_GetNextSibling(doc_handle, current) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flat_bookmark_struct() {
        let fb = FlatBookmark {
            title: "Chapter 1".to_string(),
            page_index: Some(5),
            depth: 0,
        };
        assert_eq!(fb.title, "Chapter 1");
        assert_eq!(fb.page_index, Some(5));
        assert_eq!(fb.depth, 0);
    }
}
