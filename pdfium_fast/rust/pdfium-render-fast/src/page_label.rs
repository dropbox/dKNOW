//! Page label support for PDF documents.
//!
//! Page labels allow PDFs to use custom numbering schemes like
//! "i, ii, iii" for front matter or "A-1, A-2" for appendices.
//!
//! # Example
//!
//! ```no_run
//! use pdfium_render_fast::Pdfium;
//!
//! let pdfium = Pdfium::new()?;
//! let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
//!
//! // Get page label for page 0 (first page)
//! if let Some(label) = doc.page_label(0)? {
//!     println!("Page 1 label: {}", label);
//! }
//!
//! // Get all page labels
//! for i in 0..doc.page_count() {
//!     let label = doc.page_label(i)?.unwrap_or_else(|| format!("{}", i + 1));
//!     println!("Page {} label: {}", i + 1, label);
//! }
//! # Ok::<(), pdfium_render_fast::PdfError>(())
//! ```

use crate::error::{PdfError, Result};
use pdfium_sys::FPDF_GetPageLabel;
use std::ffi::c_void;

/// Get the page label for a specific page in a document.
///
/// # Arguments
///
/// * `doc_handle` - The raw PDFium document handle
/// * `page_index` - Zero-based page index
///
/// # Returns
///
/// * `Ok(Some(label))` - The page label as a String
/// * `Ok(None)` - The page has no custom label
/// * `Err(PdfError)` - An error occurred
pub(crate) fn get_page_label(
    doc_handle: pdfium_sys::FPDF_DOCUMENT,
    page_index: i32,
) -> Result<Option<String>> {
    // First call to get buffer size
    let required_len =
        unsafe { FPDF_GetPageLabel(doc_handle, page_index, std::ptr::null_mut(), 0) };

    if required_len == 0 {
        // No label for this page
        return Ok(None);
    }

    // Allocate buffer (UTF-16LE, so 2 bytes per char)
    let mut buffer: Vec<u8> = vec![0; required_len as usize];

    let result_len = unsafe {
        FPDF_GetPageLabel(
            doc_handle,
            page_index,
            buffer.as_mut_ptr() as *mut c_void,
            required_len,
        )
    };

    if result_len == 0 || result_len > required_len {
        return Ok(None);
    }

    // Convert UTF-16LE to String
    // Buffer contains UTF-16LE with trailing null (2 bytes)
    let u16_len = (result_len as usize) / 2;
    if u16_len <= 1 {
        // Only null terminator or empty
        return Ok(None);
    }

    // Convert bytes to u16 slice (excluding null terminator)
    let u16_slice: Vec<u16> = buffer[..(u16_len - 1) * 2]
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .collect();

    match String::from_utf16(&u16_slice) {
        Ok(s) if s.is_empty() => Ok(None),
        Ok(s) => Ok(Some(s)),
        Err(_) => Err(PdfError::InvalidData {
            reason: "Invalid UTF-16 in page label".into(),
        }),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_page_label_encoding() {
        // Test UTF-16LE conversion
        let test_bytes: Vec<u8> = vec![
            0x41, 0x00, // 'A'
            0x2D, 0x00, // '-'
            0x31, 0x00, // '1'
            0x00, 0x00, // null terminator
        ];

        let u16_slice: Vec<u16> = test_bytes[..6]
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();

        let result = String::from_utf16(&u16_slice).unwrap();
        assert_eq!(result, "A-1");
    }

    #[test]
    fn test_roman_numeral_encoding() {
        // Test Roman numerals in UTF-16LE
        let test_bytes: Vec<u8> = vec![
            0x69, 0x00, // 'i'
            0x69, 0x00, // 'i'
            0x69, 0x00, // 'i'
            0x00, 0x00, // null terminator
        ];

        let u16_slice: Vec<u16> = test_bytes[..6]
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();

        let result = String::from_utf16(&u16_slice).unwrap();
        assert_eq!(result, "iii");
    }
}
