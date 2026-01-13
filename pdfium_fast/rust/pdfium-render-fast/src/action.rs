//! PDF action support
//!
//! Provides access to PDF actions associated with links and bookmarks.
//!
//! Actions define what happens when a user clicks on a link or bookmark,
//! such as navigating to a page, opening a URI, or executing JavaScript.

use crate::destination::PdfDestination;
use pdfium_sys::*;

/// A PDF action (what happens when a link or bookmark is activated).
pub struct PdfAction {
    handle: FPDF_ACTION,
    doc_handle: FPDF_DOCUMENT,
}

impl PdfAction {
    /// Create a new action from handles.
    pub(crate) fn new(handle: FPDF_ACTION, doc_handle: FPDF_DOCUMENT) -> Option<Self> {
        if handle.is_null() {
            None
        } else {
            Some(Self { handle, doc_handle })
        }
    }

    /// Get the raw handle.
    pub fn handle(&self) -> FPDF_ACTION {
        self.handle
    }

    /// Get the action type.
    pub fn action_type(&self) -> ActionType {
        let action_type = unsafe { FPDFAction_GetType(self.handle) };
        ActionType::from(action_type)
    }

    /// Get the destination for a GoTo action.
    ///
    /// Returns None if this is not a GoTo action or the destination is invalid.
    pub fn destination(&self) -> Option<PdfDestination> {
        let dest = unsafe { FPDFAction_GetDest(self.doc_handle, self.handle) };
        PdfDestination::new(dest, self.doc_handle)
    }

    /// Get the file path for a RemoteGoTo or Launch action.
    ///
    /// Returns None if no file path is associated with this action.
    pub fn file_path(&self) -> Option<String> {
        unsafe {
            // First call to get buffer size
            let size = FPDFAction_GetFilePath(self.handle, std::ptr::null_mut(), 0);
            if size == 0 {
                return None;
            }

            // Allocate buffer
            let mut buffer = vec![0u8; size as usize];
            let actual_size = FPDFAction_GetFilePath(
                self.handle,
                buffer.as_mut_ptr() as *mut std::ffi::c_void,
                size,
            );

            if actual_size == 0 {
                return None;
            }

            // Remove trailing null
            if buffer.last() == Some(&0) {
                buffer.pop();
            }

            String::from_utf8(buffer).ok()
        }
    }

    /// Get the URI path for a URI action.
    ///
    /// Returns None if this is not a URI action or no URI is available.
    pub fn uri_path(&self) -> Option<String> {
        unsafe {
            // First call to get buffer size
            let size = FPDFAction_GetURIPath(self.doc_handle, self.handle, std::ptr::null_mut(), 0);
            if size == 0 {
                return None;
            }

            // Allocate buffer
            let mut buffer = vec![0u8; size as usize];
            let actual_size = FPDFAction_GetURIPath(
                self.doc_handle,
                self.handle,
                buffer.as_mut_ptr() as *mut std::ffi::c_void,
                size,
            );

            if actual_size == 0 {
                return None;
            }

            // Remove trailing null
            if buffer.last() == Some(&0) {
                buffer.pop();
            }

            String::from_utf8(buffer).ok()
        }
    }

    /// Convenience: Get action details as an enum.
    pub fn details(&self) -> ActionDetails {
        match self.action_type() {
            ActionType::GoTo => {
                if let Some(dest) = self.destination() {
                    ActionDetails::GoTo {
                        page_index: dest.page_index(),
                        location: dest.location_in_page(),
                    }
                } else {
                    ActionDetails::Unknown
                }
            }
            ActionType::RemoteGoTo => ActionDetails::RemoteGoTo {
                file_path: self.file_path(),
            },
            ActionType::URI => ActionDetails::URI {
                uri: self.uri_path(),
            },
            ActionType::Launch => ActionDetails::Launch {
                file_path: self.file_path(),
            },
            _ => ActionDetails::Unknown,
        }
    }
}

/// Type of a PDF action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActionType {
    /// Unsupported or unknown action type
    Unsupported,
    /// Go to a destination in the same document
    GoTo,
    /// Go to a destination in another document
    RemoteGoTo,
    /// Open a URI
    URI,
    /// Launch an application or file
    Launch,
}

impl From<u64> for ActionType {
    fn from(value: u64) -> Self {
        match value {
            1 => ActionType::GoTo,
            2 => ActionType::RemoteGoTo,
            3 => ActionType::URI,
            4 => ActionType::Launch,
            _ => ActionType::Unsupported,
        }
    }
}

/// Detailed information about an action.
#[derive(Debug, Clone)]
pub enum ActionDetails {
    /// Unknown or unsupported action
    Unknown,
    /// Navigate to a page in this document
    GoTo {
        /// Target page index (0-based)
        page_index: Option<usize>,
        /// Target location on page
        location: crate::destination::DestLocation,
    },
    /// Navigate to another document
    RemoteGoTo {
        /// Path to the target document
        file_path: Option<String>,
    },
    /// Open a URI
    URI {
        /// The URI to open
        uri: Option<String>,
    },
    /// Launch an application or file
    Launch {
        /// Path to launch
        file_path: Option<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_type() {
        assert_eq!(ActionType::from(0), ActionType::Unsupported);
        assert_eq!(ActionType::from(1), ActionType::GoTo);
        assert_eq!(ActionType::from(2), ActionType::RemoteGoTo);
        assert_eq!(ActionType::from(3), ActionType::URI);
        assert_eq!(ActionType::from(4), ActionType::Launch);
        assert_eq!(ActionType::from(99), ActionType::Unsupported);
    }
}
