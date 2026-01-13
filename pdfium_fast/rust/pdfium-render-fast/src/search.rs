//! Text search functionality for PDF pages.
//!
//! This module provides text search capabilities using PDFium's search APIs.
//!
//! # Example
//!
//! ```no_run
//! use pdfium_render_fast::{Pdfium, PdfSearchOptions};
//!
//! let pdfium = Pdfium::new()?;
//! let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
//! let page = doc.page(0)?;
//! let text = page.text()?;
//!
//! // Search for text
//! let mut search = text.search("keyword", PdfSearchOptions::default())?;
//!
//! // Find all matches
//! while search.find_next() {
//!     let start = search.match_start();
//!     let count = search.match_count();
//!     println!("Found match at char {} (length {})", start, count);
//! }
//! # Ok::<(), pdfium_render_fast::PdfError>(())
//! ```

use crate::error::{PdfError, Result};
use pdfium_sys::{
    FPDFText_FindClose, FPDFText_FindNext, FPDFText_FindPrev, FPDFText_FindStart,
    FPDFText_GetSchCount, FPDFText_GetSchResultIndex, FPDF_SCHHANDLE, FPDF_TEXTPAGE,
};

/// Search option flags.
#[derive(Debug, Clone, Copy, Default)]
pub struct PdfSearchOptions {
    /// Case-sensitive matching.
    pub match_case: bool,
    /// Match whole words only.
    pub match_whole_word: bool,
    /// Find consecutive matches (no gap between matches).
    pub consecutive: bool,
}

impl PdfSearchOptions {
    /// Create new search options with all flags disabled.
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable case-sensitive matching.
    pub fn case_sensitive(mut self) -> Self {
        self.match_case = true;
        self
    }

    /// Enable whole word matching.
    pub fn whole_word(mut self) -> Self {
        self.match_whole_word = true;
        self
    }

    /// Enable consecutive matching.
    pub fn consecutive(mut self) -> Self {
        self.consecutive = true;
        self
    }

    /// Convert to PDFium flags.
    fn to_flags(self) -> u32 {
        let mut flags = 0u32;
        if self.match_case {
            flags |= 0x0001; // FPDF_MATCHCASE
        }
        if self.match_whole_word {
            flags |= 0x0002; // FPDF_MATCHWHOLEWORD
        }
        if self.consecutive {
            flags |= 0x0004; // FPDF_CONSECUTIVE
        }
        flags
    }
}

/// A text search context for finding text within a PDF page.
///
/// The search handle is automatically closed when dropped.
pub struct PdfTextSearch {
    handle: FPDF_SCHHANDLE,
    current_match_valid: bool,
}

impl PdfTextSearch {
    /// Create a new search context.
    ///
    /// # Arguments
    ///
    /// * `text_page` - The text page handle to search within
    /// * `pattern` - The text pattern to search for
    /// * `options` - Search options (case sensitivity, whole word, etc.)
    /// * `start_index` - Character index to start searching from (-1 for end)
    ///
    /// # Safety
    ///
    /// The text_page handle must be valid for the lifetime of this search.
    pub(crate) fn new(
        text_page: FPDF_TEXTPAGE,
        pattern: &str,
        options: PdfSearchOptions,
        start_index: i32,
    ) -> Result<Self> {
        // Convert pattern to UTF-16LE (null-terminated)
        let utf16: Vec<u16> = pattern.encode_utf16().chain(std::iter::once(0)).collect();

        let handle = unsafe {
            FPDFText_FindStart(
                text_page,
                utf16.as_ptr(),
                options.to_flags() as ::std::os::raw::c_ulong,
                start_index,
            )
        };

        if handle.is_null() {
            return Err(PdfError::SearchError(
                "Failed to start text search".to_string(),
            ));
        }

        Ok(Self {
            handle,
            current_match_valid: false,
        })
    }

    /// Find the next occurrence of the search pattern.
    ///
    /// Returns `true` if a match was found, `false` otherwise.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use pdfium_render_fast::{Pdfium, PdfSearchOptions};
    /// # let pdfium = Pdfium::new()?;
    /// # let doc = pdfium.load_pdf_from_file("test.pdf", None)?;
    /// # let page = doc.page(0)?;
    /// # let text = page.text()?;
    /// let mut search = text.search("hello", PdfSearchOptions::default())?;
    /// while search.find_next() {
    ///     println!("Found at index {}", search.match_start());
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn find_next(&mut self) -> bool {
        let result = unsafe { FPDFText_FindNext(self.handle) };
        self.current_match_valid = result != 0;
        self.current_match_valid
    }

    /// Find the previous occurrence of the search pattern.
    ///
    /// Returns `true` if a match was found, `false` otherwise.
    pub fn find_prev(&mut self) -> bool {
        let result = unsafe { FPDFText_FindPrev(self.handle) };
        self.current_match_valid = result != 0;
        self.current_match_valid
    }

    /// Get the starting character index of the current match.
    ///
    /// Returns the character index, or -1 if no current match.
    pub fn match_start(&self) -> i32 {
        if !self.current_match_valid {
            return -1;
        }
        unsafe { FPDFText_GetSchResultIndex(self.handle) }
    }

    /// Get the number of characters in the current match.
    ///
    /// Returns the match length, or 0 if no current match.
    pub fn match_count(&self) -> i32 {
        if !self.current_match_valid {
            return 0;
        }
        unsafe { FPDFText_GetSchCount(self.handle) }
    }

    /// Check if there is a current valid match.
    pub fn has_match(&self) -> bool {
        self.current_match_valid
    }

    /// Get the current match as a (start_index, length) tuple.
    ///
    /// Returns `None` if no current match.
    pub fn current_match(&self) -> Option<(i32, i32)> {
        if self.current_match_valid {
            Some((self.match_start(), self.match_count()))
        } else {
            None
        }
    }
}

impl Drop for PdfTextSearch {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe {
                FPDFText_FindClose(self.handle);
            }
        }
    }
}

// PdfTextSearch is not Send/Sync because the underlying PDFium handle
// is tied to the text page which is tied to the document.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_options_default() {
        let opts = PdfSearchOptions::default();
        assert!(!opts.match_case);
        assert!(!opts.match_whole_word);
        assert!(!opts.consecutive);
        assert_eq!(opts.to_flags(), 0);
    }

    #[test]
    fn test_search_options_flags() {
        let opts = PdfSearchOptions::new().case_sensitive().whole_word();
        assert!(opts.match_case);
        assert!(opts.match_whole_word);
        assert!(!opts.consecutive);
        assert_eq!(opts.to_flags(), 0x0003);
    }

    #[test]
    fn test_search_options_all_flags() {
        let opts = PdfSearchOptions::new()
            .case_sensitive()
            .whole_word()
            .consecutive();
        assert_eq!(opts.to_flags(), 0x0007);
    }
}
