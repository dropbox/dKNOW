//! Digital signature support for PDF documents.
//!
//! This module provides access to digital signatures embedded in PDF documents.
//! Signatures can be used to verify document integrity and authenticity.
//!
//! # Example
//!
//! ```no_run
//! use pdfium_render_fast::Pdfium;
//!
//! let pdfium = Pdfium::new()?;
//! let doc = pdfium.load_pdf_from_file("signed_document.pdf", None)?;
//!
//! // Check if document has signatures
//! if doc.has_signatures() {
//!     println!("Document has {} signature(s)", doc.signature_count());
//!
//!     for sig in doc.signatures() {
//!         println!("Signature reason: {:?}", sig.reason());
//!         println!("Signature time: {:?}", sig.time());
//!         println!("Sub-filter: {:?}", sig.sub_filter());
//!         println!("Permission: {:?}", sig.doc_mdp_permission());
//!     }
//! }
//! # Ok::<(), pdfium_render_fast::PdfError>(())
//! ```

use crate::error::{PdfError, Result};
use pdfium_sys::{
    FPDFSignatureObj_GetByteRange, FPDFSignatureObj_GetContents,
    FPDFSignatureObj_GetDocMDPPermission, FPDFSignatureObj_GetReason,
    FPDFSignatureObj_GetSubFilter, FPDFSignatureObj_GetTime, FPDF_GetSignatureCount,
    FPDF_GetSignatureObject, FPDF_SIGNATURE,
};
use std::ffi::c_void;

/// DocMDP (Modification Detection and Prevention) permission level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocMDPPermission {
    /// No changes to the document are permitted.
    NoChanges,
    /// Only filling in forms, instantiating page templates, and signing are permitted.
    FillFormsOnly,
    /// Filling in forms, instantiating page templates, signing, and annotation
    /// creation/deletion/modification are permitted.
    FillFormsAndAnnotations,
    /// Unknown or undefined permission level.
    Unknown(u32),
}

impl From<u32> for DocMDPPermission {
    fn from(value: u32) -> Self {
        match value {
            1 => DocMDPPermission::NoChanges,
            2 => DocMDPPermission::FillFormsOnly,
            3 => DocMDPPermission::FillFormsAndAnnotations,
            _ => DocMDPPermission::Unknown(value),
        }
    }
}

/// Represents a digital signature in a PDF document.
#[derive(Debug)]
pub struct PdfSignature {
    handle: FPDF_SIGNATURE,
}

impl PdfSignature {
    pub(crate) fn new(handle: FPDF_SIGNATURE) -> Self {
        Self { handle }
    }

    /// Returns the raw signature contents (typically DER-encoded PKCS#1 or PKCS#7).
    ///
    /// For verification, this data would need to be parsed using a crypto library.
    pub fn contents(&self) -> Result<Option<Vec<u8>>> {
        // First call to get buffer size
        let required_len =
            unsafe { FPDFSignatureObj_GetContents(self.handle, std::ptr::null_mut(), 0) };

        if required_len == 0 {
            return Ok(None);
        }

        let mut buffer: Vec<u8> = vec![0; required_len as usize];

        let result_len = unsafe {
            FPDFSignatureObj_GetContents(
                self.handle,
                buffer.as_mut_ptr() as *mut c_void,
                required_len,
            )
        };

        if result_len == 0 {
            return Ok(None);
        }

        buffer.truncate(result_len as usize);
        Ok(Some(buffer))
    }

    /// Returns the byte ranges covered by this signature.
    ///
    /// The byte range is an array of pairs (offset, length) describing which
    /// bytes of the file are covered by the signature's digest.
    pub fn byte_range(&self) -> Result<Option<Vec<(i32, i32)>>> {
        // First call to get count
        let count = unsafe { FPDFSignatureObj_GetByteRange(self.handle, std::ptr::null_mut(), 0) };

        if count == 0 {
            return Ok(None);
        }

        let mut buffer: Vec<i32> = vec![0; count as usize];

        let result_count =
            unsafe { FPDFSignatureObj_GetByteRange(self.handle, buffer.as_mut_ptr(), count) };

        if result_count == 0 || result_count != count {
            return Ok(None);
        }

        // Convert flat array to pairs
        let pairs: Vec<(i32, i32)> = buffer
            .chunks_exact(2)
            .map(|chunk| (chunk[0], chunk[1]))
            .collect();

        Ok(Some(pairs))
    }

    /// Returns the signature's sub-filter (encoding type).
    ///
    /// Common values include:
    /// - "adbe.pkcs7.detached"
    /// - "adbe.pkcs7.sha1"
    /// - "ETSI.CAdES.detached"
    pub fn sub_filter(&self) -> Result<Option<String>> {
        // First call to get buffer size
        let required_len =
            unsafe { FPDFSignatureObj_GetSubFilter(self.handle, std::ptr::null_mut(), 0) };

        if required_len == 0 {
            return Ok(None);
        }

        let mut buffer: Vec<u8> = vec![0; required_len as usize];

        let result_len = unsafe {
            FPDFSignatureObj_GetSubFilter(self.handle, buffer.as_mut_ptr() as *mut i8, required_len)
        };

        if result_len == 0 {
            return Ok(None);
        }

        // ASCII string with null terminator
        let s = String::from_utf8_lossy(&buffer[..result_len.saturating_sub(1) as usize]);
        if s.is_empty() {
            Ok(None)
        } else {
            Ok(Some(s.into_owned()))
        }
    }

    /// Returns the reason/comment for the signature.
    ///
    /// This is the signer's stated reason for signing the document.
    pub fn reason(&self) -> Result<Option<String>> {
        // First call to get buffer size
        let required_len =
            unsafe { FPDFSignatureObj_GetReason(self.handle, std::ptr::null_mut(), 0) };

        if required_len == 0 {
            return Ok(None);
        }

        let mut buffer: Vec<u8> = vec![0; required_len as usize];

        let result_len = unsafe {
            FPDFSignatureObj_GetReason(
                self.handle,
                buffer.as_mut_ptr() as *mut c_void,
                required_len,
            )
        };

        if result_len == 0 {
            return Ok(None);
        }

        // UTF-16LE string
        let u16_len = (result_len as usize) / 2;
        if u16_len <= 1 {
            return Ok(None);
        }

        let u16_slice: Vec<u16> = buffer[..(u16_len - 1) * 2]
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();

        match String::from_utf16(&u16_slice) {
            Ok(s) if s.is_empty() => Ok(None),
            Ok(s) => Ok(Some(s)),
            Err(_) => Err(PdfError::InvalidData {
                reason: "Invalid UTF-16 in signature reason".into(),
            }),
        }
    }

    /// Returns the signing time.
    ///
    /// Format: D:YYYYMMDDHHMMSS+XX'YY' (PDF date format)
    ///
    /// Note: This value should only be used when the signing time is not
    /// available in the PKCS#7 binary signature itself.
    pub fn time(&self) -> Result<Option<String>> {
        // First call to get buffer size
        let required_len =
            unsafe { FPDFSignatureObj_GetTime(self.handle, std::ptr::null_mut(), 0) };

        if required_len == 0 {
            return Ok(None);
        }

        let mut buffer: Vec<u8> = vec![0; required_len as usize];

        let result_len = unsafe {
            FPDFSignatureObj_GetTime(self.handle, buffer.as_mut_ptr() as *mut i8, required_len)
        };

        if result_len == 0 {
            return Ok(None);
        }

        // ASCII string with null terminator
        let s = String::from_utf8_lossy(&buffer[..result_len.saturating_sub(1) as usize]);
        if s.is_empty() {
            Ok(None)
        } else {
            Ok(Some(s.into_owned()))
        }
    }

    /// Returns the DocMDP permission level for this signature.
    ///
    /// DocMDP (Modification Detection and Prevention) specifies what changes
    /// are allowed after signing.
    ///
    /// Returns `None` if the signature doesn't specify DocMDP permissions.
    pub fn doc_mdp_permission(&self) -> Option<DocMDPPermission> {
        let perm = unsafe { FPDFSignatureObj_GetDocMDPPermission(self.handle) };

        if perm == 0 {
            None
        } else {
            Some(DocMDPPermission::from(perm))
        }
    }
}

/// Collection of signatures in a PDF document.
pub struct PdfSignatures {
    doc_handle: pdfium_sys::FPDF_DOCUMENT,
    count: i32,
}

impl PdfSignatures {
    pub(crate) fn new(doc_handle: pdfium_sys::FPDF_DOCUMENT) -> Self {
        let count = unsafe { FPDF_GetSignatureCount(doc_handle) };
        Self {
            doc_handle,
            count: count.max(0),
        }
    }

    /// Returns the number of signatures in the document.
    pub fn len(&self) -> usize {
        self.count as usize
    }

    /// Returns true if there are no signatures.
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Returns the signature at the given index, if it exists.
    pub fn get(&self, index: usize) -> Option<PdfSignature> {
        if index >= self.count as usize {
            return None;
        }

        let handle = unsafe { FPDF_GetSignatureObject(self.doc_handle, index as i32) };

        if handle.is_null() {
            None
        } else {
            Some(PdfSignature::new(handle))
        }
    }
}

impl IntoIterator for PdfSignatures {
    type Item = PdfSignature;
    type IntoIter = PdfSignaturesIntoIter;

    fn into_iter(self) -> Self::IntoIter {
        PdfSignaturesIntoIter {
            signatures: self,
            index: 0,
        }
    }
}

impl<'a> IntoIterator for &'a PdfSignatures {
    type Item = PdfSignature;
    type IntoIter = PdfSignaturesIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        PdfSignaturesIter {
            signatures: self,
            index: 0,
        }
    }
}

/// Owning iterator over signatures in a document.
pub struct PdfSignaturesIntoIter {
    signatures: PdfSignatures,
    index: usize,
}

impl Iterator for PdfSignaturesIntoIter {
    type Item = PdfSignature;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.signatures.count as usize {
            return None;
        }

        let sig = self.signatures.get(self.index);
        self.index += 1;
        sig
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.signatures.count as usize - self.index;
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for PdfSignaturesIntoIter {}

/// Iterator over signatures in a document.
pub struct PdfSignaturesIter<'a> {
    signatures: &'a PdfSignatures,
    index: usize,
}

impl Iterator for PdfSignaturesIter<'_> {
    type Item = PdfSignature;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.signatures.count as usize {
            return None;
        }

        let sig = self.signatures.get(self.index);
        self.index += 1;
        sig
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.signatures.count as usize - self.index;
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for PdfSignaturesIter<'_> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_doc_mdp_permission_from() {
        assert_eq!(DocMDPPermission::from(1), DocMDPPermission::NoChanges);
        assert_eq!(DocMDPPermission::from(2), DocMDPPermission::FillFormsOnly);
        assert_eq!(
            DocMDPPermission::from(3),
            DocMDPPermission::FillFormsAndAnnotations
        );
        assert_eq!(DocMDPPermission::from(0), DocMDPPermission::Unknown(0));
        assert_eq!(DocMDPPermission::from(99), DocMDPPermission::Unknown(99));
    }
}
