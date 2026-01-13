//! PDF attachment support
//!
//! Provides access to embedded files (attachments) in PDF documents.
//!
//! PDF documents can contain embedded files as attachments in the
//! document's attachment collection.

use crate::error::{PdfError, Result};
use pdfium_sys::*;

/// An attachment (embedded file) in a PDF document.
pub struct PdfAttachment {
    handle: FPDF_ATTACHMENT,
    #[allow(dead_code)]
    doc_handle: FPDF_DOCUMENT,
}

impl PdfAttachment {
    /// Create a new attachment from handles.
    pub(crate) fn new(handle: FPDF_ATTACHMENT, doc_handle: FPDF_DOCUMENT) -> Option<Self> {
        if handle.is_null() {
            None
        } else {
            Some(Self { handle, doc_handle })
        }
    }

    /// Get the raw handle.
    pub fn handle(&self) -> FPDF_ATTACHMENT {
        self.handle
    }

    /// Get the attachment name (filename).
    pub fn name(&self) -> Option<String> {
        unsafe {
            // First call to get buffer size (in bytes, UTF-16)
            let size = FPDFAttachment_GetName(self.handle, std::ptr::null_mut(), 0);
            if size == 0 {
                return None;
            }

            // Allocate buffer (size is in bytes, UTF-16 = 2 bytes per char)
            let mut buffer: Vec<u16> = vec![0; (size as usize) / 2];
            let actual_size = FPDFAttachment_GetName(self.handle, buffer.as_mut_ptr(), size);

            if actual_size == 0 {
                return None;
            }

            // Remove trailing nulls
            while buffer.last() == Some(&0) {
                buffer.pop();
            }

            String::from_utf16(&buffer).ok()
        }
    }

    /// Check if the attachment has a specific key in its dictionary.
    pub fn has_key(&self, key: &str) -> bool {
        let c_key = std::ffi::CString::new(key).ok();
        match c_key {
            Some(k) => unsafe { FPDFAttachment_HasKey(self.handle, k.as_ptr()) != 0 },
            None => false,
        }
    }

    /// Get the value type for a key.
    pub fn value_type(&self, key: &str) -> AttachmentValueType {
        let c_key = std::ffi::CString::new(key).ok();
        match c_key {
            Some(k) => {
                let val_type = unsafe { FPDFAttachment_GetValueType(self.handle, k.as_ptr()) };
                AttachmentValueType::from(val_type)
            }
            None => AttachmentValueType::Unknown,
        }
    }

    /// Get a string value for a key.
    pub fn get_string_value(&self, key: &str) -> Option<String> {
        let c_key = std::ffi::CString::new(key).ok()?;

        unsafe {
            // First call to get buffer size
            let size =
                FPDFAttachment_GetStringValue(self.handle, c_key.as_ptr(), std::ptr::null_mut(), 0);
            if size == 0 {
                return None;
            }

            // Allocate buffer (UTF-16)
            let mut buffer: Vec<u16> = vec![0; (size as usize) / 2];
            let actual_size = FPDFAttachment_GetStringValue(
                self.handle,
                c_key.as_ptr(),
                buffer.as_mut_ptr(),
                size,
            );

            if actual_size == 0 {
                return None;
            }

            // Remove trailing nulls
            while buffer.last() == Some(&0) {
                buffer.pop();
            }

            String::from_utf16(&buffer).ok()
        }
    }

    /// Get the subtype (MIME type) of the attachment.
    ///
    /// Returns the MIME type string (e.g., "application/pdf", "image/png").
    pub fn subtype(&self) -> Option<String> {
        unsafe {
            // First call to get buffer size (in bytes, UTF-16)
            let size = FPDFAttachment_GetSubtype(self.handle, std::ptr::null_mut(), 0);
            if size == 0 {
                return None;
            }

            // Allocate buffer (UTF-16)
            let mut buffer: Vec<u16> = vec![0; (size as usize) / 2];
            let actual_size = FPDFAttachment_GetSubtype(self.handle, buffer.as_mut_ptr(), size);

            if actual_size == 0 {
                return None;
            }

            // Remove trailing nulls
            while buffer.last() == Some(&0) {
                buffer.pop();
            }

            String::from_utf16(&buffer).ok()
        }
    }

    /// Get the file contents of the attachment.
    ///
    /// Returns the raw file data.
    pub fn get_file(&self) -> Result<Vec<u8>> {
        unsafe {
            // First call to get buffer size
            let mut out_buflen: u64 = 0;
            let ok = FPDFAttachment_GetFile(self.handle, std::ptr::null_mut(), 0, &mut out_buflen);

            if ok == 0 || out_buflen == 0 {
                return Err(PdfError::InvalidData {
                    reason: "Failed to get attachment file size".to_string(),
                });
            }

            // Allocate buffer
            let mut buffer = vec![0u8; out_buflen as usize];
            let ok = FPDFAttachment_GetFile(
                self.handle,
                buffer.as_mut_ptr() as *mut std::ffi::c_void,
                out_buflen,
                &mut out_buflen,
            );

            if ok == 0 {
                return Err(PdfError::InvalidData {
                    reason: "Failed to get attachment file contents".to_string(),
                });
            }

            buffer.truncate(out_buflen as usize);
            Ok(buffer)
        }
    }

    /// Get common metadata values.
    pub fn metadata(&self) -> AttachmentMetadata {
        AttachmentMetadata {
            name: self.name(),
            subtype: self.subtype(),
            creation_date: self.get_string_value("CreationDate"),
            mod_date: self.get_string_value("ModDate"),
            checksum: self.get_string_value("CheckSum"),
            size: self.get_string_value("Size"),
        }
    }
}

/// Value type for attachment dictionary keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AttachmentValueType {
    /// Unknown or unsupported type
    Unknown,
    /// Boolean value
    Boolean,
    /// Number value
    Number,
    /// String value
    String,
    /// Name value
    Name,
    /// Array value
    Array,
    /// Dictionary value
    Dictionary,
    /// Stream value
    Stream,
    /// Reference value
    Reference,
}

impl From<i32> for AttachmentValueType {
    fn from(value: i32) -> Self {
        match value {
            0 => AttachmentValueType::Unknown,
            1 => AttachmentValueType::Boolean,
            2 => AttachmentValueType::Number,
            3 => AttachmentValueType::String,
            4 => AttachmentValueType::Name,
            5 => AttachmentValueType::Array,
            6 => AttachmentValueType::Dictionary,
            7 => AttachmentValueType::Stream,
            8 => AttachmentValueType::Reference,
            _ => AttachmentValueType::Unknown,
        }
    }
}

/// Common attachment metadata.
#[derive(Debug, Clone)]
pub struct AttachmentMetadata {
    /// Filename
    pub name: Option<String>,
    /// MIME type
    pub subtype: Option<String>,
    /// Creation date (PDF date format)
    pub creation_date: Option<String>,
    /// Modification date (PDF date format)
    pub mod_date: Option<String>,
    /// Checksum (if available)
    pub checksum: Option<String>,
    /// Size as string (if available)
    pub size: Option<String>,
}

/// Collection of attachments in a document.
pub struct PdfAttachments {
    doc_handle: FPDF_DOCUMENT,
    count: usize,
}

impl PdfAttachments {
    /// Create from a document handle.
    pub(crate) fn new(doc_handle: FPDF_DOCUMENT) -> Self {
        let count = unsafe { FPDFDoc_GetAttachmentCount(doc_handle) };
        Self {
            doc_handle,
            count: if count > 0 { count as usize } else { 0 },
        }
    }

    /// Get the number of attachments.
    pub fn count(&self) -> usize {
        self.count
    }

    /// Check if there are no attachments.
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Get an attachment by index.
    pub fn get(&self, index: usize) -> Option<PdfAttachment> {
        if index >= self.count {
            return None;
        }

        let handle = unsafe { FPDFDoc_GetAttachment(self.doc_handle, index as i32) };
        PdfAttachment::new(handle, self.doc_handle)
    }

    /// Iterate over all attachments.
    pub fn iter(&self) -> PdfAttachmentsIter<'_> {
        PdfAttachmentsIter {
            attachments: self,
            index: 0,
        }
    }

    /// Get all attachment metadata.
    pub fn all_metadata(&self) -> Vec<AttachmentMetadata> {
        self.iter().map(|a| a.metadata()).collect()
    }
}

/// Iterator over attachments.
pub struct PdfAttachmentsIter<'a> {
    attachments: &'a PdfAttachments,
    index: usize,
}

impl<'a> Iterator for PdfAttachmentsIter<'a> {
    type Item = PdfAttachment;

    fn next(&mut self) -> Option<Self::Item> {
        let attachment = self.attachments.get(self.index)?;
        self.index += 1;
        Some(attachment)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.attachments.count.saturating_sub(self.index);
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for PdfAttachmentsIter<'a> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attachment_value_type() {
        assert_eq!(AttachmentValueType::from(0), AttachmentValueType::Unknown);
        assert_eq!(AttachmentValueType::from(1), AttachmentValueType::Boolean);
        assert_eq!(AttachmentValueType::from(2), AttachmentValueType::Number);
        assert_eq!(AttachmentValueType::from(3), AttachmentValueType::String);
        assert_eq!(AttachmentValueType::from(4), AttachmentValueType::Name);
        assert_eq!(AttachmentValueType::from(5), AttachmentValueType::Array);
        assert_eq!(
            AttachmentValueType::from(6),
            AttachmentValueType::Dictionary
        );
        assert_eq!(AttachmentValueType::from(7), AttachmentValueType::Stream);
        assert_eq!(AttachmentValueType::from(8), AttachmentValueType::Reference);
        assert_eq!(AttachmentValueType::from(99), AttachmentValueType::Unknown);
    }

    #[test]
    fn test_attachment_metadata() {
        let meta = AttachmentMetadata {
            name: Some("test.pdf".to_string()),
            subtype: Some("application/pdf".to_string()),
            creation_date: Some("D:20250101120000".to_string()),
            mod_date: None,
            checksum: None,
            size: Some("12345".to_string()),
        };
        assert_eq!(meta.name, Some("test.pdf".to_string()));
        assert_eq!(meta.subtype, Some("application/pdf".to_string()));
    }
}
