//! Safe wrapper around the docling-parse C API

use crate::convert::convert_to_segmented_page;
use crate::types::DoclingParseResult;
use crate::{Error, PdfDocument, PdfPage, Result};
use docling_core::types::page::SegmentedPdfPage;
use std::ffi::{CStr, CString};
use std::path::Path;
use std::ptr;

/// A parser for PDF documents using docling-parse
///
/// This wraps the C API and provides a safe Rust interface.
pub struct DoclingParser {
    ptr: *mut docling_parse_sys::DoclingParser,
}

impl DoclingParser {
    /// Create a new parser with the specified log level
    ///
    /// Valid log levels: "debug", "info", "warn", "error", "off"
    #[must_use = "this function returns a parser that should be used"]
    pub fn new(loglevel: &str) -> Result<Self> {
        let loglevel_c = CString::new(loglevel)?;

        let ptr = unsafe { docling_parse_sys::docling_parser_new(loglevel_c.as_ptr()) };

        if ptr.is_null() {
            return Err(Error::OutOfMemory);
        }

        Ok(DoclingParser { ptr })
    }

    /// Load a PDF document
    ///
    /// # Arguments
    ///
    /// * `key` - A unique identifier for this document
    /// * `filename` - Path to the PDF file
    /// * `password` - Optional password for encrypted PDFs
    #[must_use = "this function returns a Result that should be checked for errors"]
    pub fn load_document(
        &mut self,
        key: &str,
        filename: &Path,
        password: Option<&str>,
    ) -> Result<()> {
        let key_c = CString::new(key)?;
        let filename_c = CString::new(filename.to_str().ok_or_else(|| {
            Error::InvalidParameter("Filename contains invalid UTF-8".to_string())
        })?)?;

        let password_c = password.map(CString::new).transpose()?;
        let password_ptr = password_c.as_ref().map_or(ptr::null(), |c| c.as_ptr());

        let result = unsafe {
            docling_parse_sys::docling_parser_load_document(
                self.ptr,
                key_c.as_ptr(),
                filename_c.as_ptr(),
                password_ptr,
            )
        };

        self.check_error(result)?;
        Ok(())
    }

    /// Unload a previously loaded document
    #[must_use = "this function returns a Result that should be checked for errors"]
    pub fn unload_document(&mut self, key: &str) -> Result<()> {
        let key_c = CString::new(key)?;

        let result =
            unsafe { docling_parse_sys::docling_parser_unload_document(self.ptr, key_c.as_ptr()) };

        self.check_error(result)?;
        Ok(())
    }

    /// Check if a document is loaded
    #[must_use = "this function returns document loaded status that should be checked"]
    pub fn is_loaded(&self, key: &str) -> Result<bool> {
        let key_c = CString::new(key)?;

        let result =
            unsafe { docling_parse_sys::docling_parser_is_loaded(self.ptr, key_c.as_ptr()) };

        Ok(result != 0)
    }

    /// Get the number of pages in a loaded document
    #[must_use = "this function returns a page count that should be used"]
    pub fn number_of_pages(&self, key: &str) -> Result<usize> {
        let key_c = CString::new(key)?;

        let result =
            unsafe { docling_parse_sys::docling_parser_number_of_pages(self.ptr, key_c.as_ptr()) };

        if result < 0 {
            return Err(Error::NotLoaded(key.to_string()));
        }

        Ok(result as usize)
    }

    /// Parse a single page and return the JSON string
    ///
    /// # Arguments
    ///
    /// * `key` - The document identifier
    /// * `page_num` - The page number (0-indexed)
    #[must_use = "this function returns a parsed page that should be processed"]
    pub fn parse_page(&self, key: &str, page_num: usize) -> Result<PdfPage> {
        let key_c = CString::new(key)?;

        let mut output = docling_parse_sys::DoclingString {
            data: ptr::null_mut(),
            length: 0,
        };

        let result = unsafe {
            docling_parse_sys::docling_parser_parse_page(
                self.ptr,
                key_c.as_ptr(),
                page_num as i32,
                &mut output,
            )
        };

        self.check_error(result)?;

        // Convert the C string to a Rust string
        let json_str = self.convert_docling_string(&output)?;

        // TEMPORARY: Don't try to parse the JSON yet, just return it
        // Parse JSON into PdfPage
        let page = PdfPage {
            raw_json: Some(json_str),
            ..Default::default()
        };

        Ok(page)
    }

    /// Parse a single page and return a structured SegmentedPdfPage
    ///
    /// This method calls parse_page internally to get the JSON output,
    /// then parses and converts it to a SegmentedPdfPage.
    ///
    /// # Arguments
    ///
    /// * `key` - The document identifier
    /// * `page_num` - The page number (0-indexed)
    #[must_use = "this function returns a parsed page that should be processed"]
    pub fn parse_page_structured(&self, key: &str, page_num: usize) -> Result<SegmentedPdfPage> {
        // Get raw JSON from C API
        let key_c = CString::new(key)?;

        let mut output = docling_parse_sys::DoclingString {
            data: ptr::null_mut(),
            length: 0,
        };

        let result = unsafe {
            docling_parse_sys::docling_parser_parse_page(
                self.ptr,
                key_c.as_ptr(),
                page_num as i32,
                &mut output,
            )
        };

        self.check_error(result)?;

        // Convert the C string to a Rust string
        let json_str = self.convert_docling_string(&output)?;

        // Parse JSON into DoclingParseResult
        let parse_result: DoclingParseResult = serde_json::from_str(&json_str)
            .map_err(|e| Error::JsonParseError(format!("Failed to parse JSON: {}", e)))?;

        // The C API returns all pages in the JSON, find the requested page
        let page_data = parse_result
            .pages
            .iter()
            .find(|p| p.page_number == page_num)
            .ok_or_else(|| {
                Error::ParseFailed(format!("Page {} not found in JSON output", page_num))
            })?;

        // Convert to SegmentedPdfPage
        convert_to_segmented_page(page_data)
            .map_err(|e| Error::ConversionError(format!("Failed to convert page: {}", e)))
    }

    /// Parse all pages and return the JSON string
    #[must_use = "this function returns a parsed document that should be processed"]
    pub fn parse_all_pages(&self, key: &str) -> Result<PdfDocument> {
        let key_c = CString::new(key)?;

        let mut output = docling_parse_sys::DoclingString {
            data: ptr::null_mut(),
            length: 0,
        };

        let result = unsafe {
            docling_parse_sys::docling_parser_parse_all_pages(self.ptr, key_c.as_ptr(), &mut output)
        };

        self.check_error(result)?;

        // Convert the C string to a Rust string
        let json_str = self.convert_docling_string(&output)?;

        // Parse JSON into PdfDocument
        let mut doc: PdfDocument = serde_json::from_str(&json_str)?;
        doc.raw_json = Some(json_str);

        Ok(doc)
    }

    /// Convert a DoclingString to a Rust String and free the C memory
    fn convert_docling_string(&self, s: &docling_parse_sys::DoclingString) -> Result<String> {
        if s.data.is_null() {
            return Ok(String::new());
        }

        // Convert to Rust string
        let c_str = unsafe { CStr::from_ptr(s.data) };
        let rust_str = c_str.to_str()?.to_string();

        // Free the C string
        unsafe {
            let mut s_mut = *s;
            docling_parse_sys::docling_string_free(&mut s_mut);
        }

        Ok(rust_str)
    }

    /// Check if a C error code indicates an error
    #[inline]
    fn check_error(&self, code: docling_parse_sys::DoclingError) -> Result<()> {
        if code == docling_parse_sys::DoclingError_DOCLING_OK {
            Ok(())
        } else {
            Err(Error::from_c_error(code))
        }
    }
}

impl Drop for DoclingParser {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe {
                docling_parse_sys::docling_parser_free(self.ptr);
            }
        }
    }
}

// DoclingParser is thread-safe (the C++ code uses thread-local storage)
unsafe impl Send for DoclingParser {}
unsafe impl Sync for DoclingParser {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "C library is stub implementation (returns NULL), see commit 5edcb25"]
    fn test_parser_creation() {
        let parser = DoclingParser::new("error");
        assert!(parser.is_ok());
    }
}
