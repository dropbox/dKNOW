//! Main PDFium entry point

use crate::document::PdfDocument;
use crate::error::{PdfError, Result};
use pdfium_sys::*;
use std::ffi::CString;
use std::path::Path;
use std::sync::{Arc, Once};

/// Global initialization - ensures FPDF_InitLibrary is called exactly once
/// and all threads wait until it completes.
static INIT: Once = Once::new();

/// Main entry point for PDFium operations.
///
/// Create a single `Pdfium` instance and use it to open PDF documents.
/// The library is automatically initialized on first use.
///
/// # Example
///
/// ```no_run
/// use pdfium_render_fast::Pdfium;
///
/// let pdfium = Pdfium::new()?;
/// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
/// # Ok::<(), pdfium_render_fast::PdfError>(())
/// ```
#[derive(Clone)]
pub struct Pdfium {
    _inner: Arc<PdfiumInner>,
}

struct PdfiumInner;

impl Pdfium {
    /// Create a new Pdfium instance, initializing the library if needed.
    ///
    /// # Returns
    ///
    /// A new `Pdfium` instance, or an error if initialization fails.
    pub fn new() -> Result<Self> {
        // Initialize the library exactly once. All threads wait until complete.
        INIT.call_once(|| unsafe {
            FPDF_InitLibrary();
        });
        Ok(Self {
            _inner: Arc::new(PdfiumInner),
        })
    }

    /// Load a PDF document from a file path.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the PDF file
    /// * `password` - Optional password for encrypted PDFs
    ///
    /// # Returns
    ///
    /// A `PdfDocument` on success, or an error if the file cannot be opened.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    ///
    /// // Open unencrypted PDF
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    ///
    /// // Open encrypted PDF
    /// let doc = pdfium.load_pdf_from_file("encrypted.pdf", Some("password"))?;
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn load_pdf_from_file<P: AsRef<Path>>(
        &self,
        path: P,
        password: Option<&str>,
    ) -> Result<PdfDocument> {
        let path = path.as_ref();

        // Check if file exists
        if !path.exists() {
            return Err(PdfError::FileNotFound(path.display().to_string()));
        }

        // Convert path to C string
        let c_path =
            CString::new(path.to_string_lossy().as_bytes()).map_err(|_| PdfError::OpenFailed {
                reason: "Invalid path encoding".to_string(),
            })?;

        // Convert password to C string if provided
        let c_password = password.and_then(|p| CString::new(p).ok());

        let password_ptr = c_password
            .as_ref()
            .map(|p| p.as_ptr())
            .unwrap_or(std::ptr::null());

        // Open the document
        let doc = unsafe { FPDF_LoadDocument(c_path.as_ptr(), password_ptr) };

        if doc.is_null() {
            let error = unsafe { FPDF_GetLastError() };
            return Err(match error as u32 {
                FPDF_ERR_PASSWORD => PdfError::InvalidPassword,
                FPDF_ERR_FILE => PdfError::FileNotFound(path.display().to_string()),
                _ => PdfError::OpenFailed {
                    reason: format!("PDFium error code: {}", error),
                },
            });
        }

        Ok(PdfDocument::from_raw(doc))
    }

    /// Load a PDF document from bytes in memory.
    ///
    /// # Arguments
    ///
    /// * `data` - Byte slice containing the PDF data
    /// * `password` - Optional password for encrypted PDFs
    ///
    /// # Returns
    ///
    /// A `PdfDocument` on success, or an error if the data cannot be parsed.
    ///
    /// # Safety
    ///
    /// The data must remain valid for the lifetime of the returned document.
    /// Use `load_pdf_from_bytes_owned` if you want the document to own the data.
    pub fn load_pdf_from_bytes(&self, data: &[u8], password: Option<&str>) -> Result<PdfDocument> {
        // Convert password to C string if provided
        let c_password = password.and_then(|p| CString::new(p).ok());

        let password_ptr = c_password
            .as_ref()
            .map(|p| p.as_ptr())
            .unwrap_or(std::ptr::null());

        // Open from memory
        let doc = unsafe {
            FPDF_LoadMemDocument(
                data.as_ptr() as *const std::ffi::c_void,
                data.len() as i32,
                password_ptr,
            )
        };

        if doc.is_null() {
            let error = unsafe { FPDF_GetLastError() };
            return Err(match error as u32 {
                FPDF_ERR_PASSWORD => PdfError::InvalidPassword,
                _ => PdfError::OpenFailed {
                    reason: format!("PDFium error code: {}", error),
                },
            });
        }

        Ok(PdfDocument::from_raw(doc))
    }

    /// Load a PDF document from owned bytes.
    ///
    /// The document takes ownership of the data, ensuring it remains valid
    /// for the document's lifetime.
    ///
    /// # Arguments
    ///
    /// * `data` - Vec containing the PDF data
    /// * `password` - Optional password for encrypted PDFs
    pub fn load_pdf_from_bytes_owned(
        &self,
        data: Vec<u8>,
        password: Option<&str>,
    ) -> Result<PdfDocument> {
        // Convert password to C string if provided
        let c_password = password.and_then(|p| CString::new(p).ok());

        let password_ptr = c_password
            .as_ref()
            .map(|p| p.as_ptr())
            .unwrap_or(std::ptr::null());

        // Open from memory
        let doc = unsafe {
            FPDF_LoadMemDocument(
                data.as_ptr() as *const std::ffi::c_void,
                data.len() as i32,
                password_ptr,
            )
        };

        if doc.is_null() {
            let error = unsafe { FPDF_GetLastError() };
            return Err(match error as u32 {
                FPDF_ERR_PASSWORD => PdfError::InvalidPassword,
                _ => PdfError::OpenFailed {
                    reason: format!("PDFium error code: {}", error),
                },
            });
        }

        Ok(PdfDocument::from_raw_with_data(doc, data))
    }

    /// Create a new empty PDF document.
    ///
    /// The returned document has no pages. Use `add_page()` or `import_pages()`
    /// to add content.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    ///
    /// // Create new document
    /// let doc = pdfium.create_new_document()?;
    ///
    /// // Import pages from another document
    /// let source = pdfium.load_pdf_from_file("source.pdf", None)?;
    /// doc.import_pages(&source, None, 0)?;
    ///
    /// // Save the new document
    /// doc.save_to_file("new_document.pdf", None)?;
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn create_new_document(&self) -> Result<PdfDocument> {
        let doc = unsafe { FPDF_CreateNewDocument() };

        if doc.is_null() {
            return Err(PdfError::OpenFailed {
                reason: "Failed to create new document".to_string(),
            });
        }

        Ok(PdfDocument::from_raw(doc))
    }

    /// Merge multiple PDF documents into a single document.
    ///
    /// Creates a new document containing all pages from the input documents
    /// in the order provided. Optionally copies viewer preferences from the
    /// first document.
    ///
    /// # Arguments
    ///
    /// * `documents` - Iterator of documents to merge (must yield at least one document)
    /// * `copy_viewer_prefs` - If true, copies viewer preferences from the first document
    ///
    /// # Returns
    ///
    /// A new [`PdfDocument`] containing all pages from the input documents.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The documents iterator is empty
    /// - Failed to create new document
    /// - Failed to import pages from any source document
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    ///
    /// // Load documents to merge
    /// let doc1 = pdfium.load_pdf_from_file("part1.pdf", None)?;
    /// let doc2 = pdfium.load_pdf_from_file("part2.pdf", None)?;
    /// let doc3 = pdfium.load_pdf_from_file("part3.pdf", None)?;
    ///
    /// // Merge all documents
    /// let merged = pdfium.merge_documents([&doc1, &doc2, &doc3], true)?;
    ///
    /// // Save the merged document
    /// merged.save_to_file("merged.pdf", None)?;
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    ///
    /// # Example with Vec
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    ///
    /// // Load documents dynamically
    /// let files = vec!["doc1.pdf", "doc2.pdf", "doc3.pdf"];
    /// let docs: Vec<_> = files.iter()
    ///     .filter_map(|f| pdfium.load_pdf_from_file(f, None).ok())
    ///     .collect();
    ///
    /// // Merge using references
    /// let merged = pdfium.merge_documents(docs.iter(), false)?;
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn merge_documents<I, D>(
        &self,
        documents: I,
        copy_viewer_prefs: bool,
    ) -> Result<PdfDocument>
    where
        I: IntoIterator<Item = D>,
        D: std::borrow::Borrow<PdfDocument>,
    {
        let merged = self.create_new_document()?;
        let mut first = true;
        let mut page_count = 0usize;
        let mut doc_count = 0usize;

        for doc_ref in documents {
            let doc = doc_ref.borrow();
            doc_count += 1;

            // Copy viewer preferences from first document
            if first && copy_viewer_prefs {
                merged.copy_viewer_preferences(doc);
                first = false;
            }

            // Import all pages from this document
            let success = merged.import_pages(doc, None, page_count)?;
            if !success {
                return Err(PdfError::InvalidInput {
                    message: format!(
                        "Failed to import pages from document {} (had {} pages)",
                        doc_count,
                        doc.page_count()
                    ),
                });
            }

            page_count += doc.page_count();
        }

        if doc_count == 0 {
            return Err(PdfError::InvalidInput {
                message: "Cannot merge empty document collection".to_string(),
            });
        }

        Ok(merged)
    }

    /// Merge PDF files from paths into a single document.
    ///
    /// Convenience method that loads and merges PDF files in one operation.
    /// This is equivalent to loading each file individually and calling
    /// [`merge_documents`](Self::merge_documents).
    ///
    /// # Arguments
    ///
    /// * `paths` - Iterator of file paths to PDF documents
    /// * `copy_viewer_prefs` - If true, copies viewer preferences from the first document
    ///
    /// # Returns
    ///
    /// A new [`PdfDocument`] containing all pages from the input files.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The paths iterator is empty
    /// - Any file fails to load
    /// - Failed to create merged document
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    ///
    /// // Merge files directly from paths
    /// let merged = pdfium.merge_files(
    ///     ["chapter1.pdf", "chapter2.pdf", "chapter3.pdf"],
    ///     true
    /// )?;
    ///
    /// merged.save_to_file("book.pdf", None)?;
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn merge_files<I, P>(&self, paths: I, copy_viewer_prefs: bool) -> Result<PdfDocument>
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        let docs: Vec<PdfDocument> = paths
            .into_iter()
            .map(|p| self.load_pdf_from_file(p, None))
            .collect::<Result<Vec<_>>>()?;

        if docs.is_empty() {
            return Err(PdfError::InvalidInput {
                message: "Cannot merge empty file list".to_string(),
            });
        }

        self.merge_documents(&docs, copy_viewer_prefs)
    }
}

impl Default for Pdfium {
    fn default() -> Self {
        Self::new().expect("Failed to initialize PDFium")
    }
}

/// Types of unsupported features that PDFium can encounter.
///
/// When PDFium encounters a feature it doesn't support, it can notify
/// your application via a callback set with [`set_unsupported_feature_handler`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnsupportedFeature {
    /// XFA form (XML Forms Architecture) - dynamic forms not supported.
    XfaForm,
    /// Portable Collection (PDF Package/Portfolio).
    PortableCollection,
    /// Document-level attachment not fully supported.
    Attachment,
    /// Document security feature not supported.
    Security,
    /// Shared review workflow.
    SharedReview,
    /// Shared form via Acrobat.com.
    SharedFormAcrobat,
    /// Shared form via filesystem.
    SharedFormFilesystem,
    /// Shared form via email.
    SharedFormEmail,
    /// 3D annotation.
    Annot3D,
    /// Movie annotation.
    AnnotMovie,
    /// Sound annotation.
    AnnotSound,
    /// Screen media annotation.
    AnnotScreenMedia,
    /// Screen rich media annotation.
    AnnotScreenRichMedia,
    /// Attachment annotation.
    AnnotAttachment,
    /// Signature annotation.
    AnnotSignature,
    /// Unknown or unrecognized unsupported feature.
    Unknown(i32),
}

impl UnsupportedFeature {
    fn from_raw(value: i32) -> Self {
        match value as u32 {
            FPDF_UNSP_DOC_XFAFORM => UnsupportedFeature::XfaForm,
            FPDF_UNSP_DOC_PORTABLECOLLECTION => UnsupportedFeature::PortableCollection,
            FPDF_UNSP_DOC_ATTACHMENT => UnsupportedFeature::Attachment,
            FPDF_UNSP_DOC_SECURITY => UnsupportedFeature::Security,
            FPDF_UNSP_DOC_SHAREDREVIEW => UnsupportedFeature::SharedReview,
            FPDF_UNSP_DOC_SHAREDFORM_ACROBAT => UnsupportedFeature::SharedFormAcrobat,
            FPDF_UNSP_DOC_SHAREDFORM_FILESYSTEM => UnsupportedFeature::SharedFormFilesystem,
            FPDF_UNSP_DOC_SHAREDFORM_EMAIL => UnsupportedFeature::SharedFormEmail,
            FPDF_UNSP_ANNOT_3DANNOT => UnsupportedFeature::Annot3D,
            FPDF_UNSP_ANNOT_MOVIE => UnsupportedFeature::AnnotMovie,
            FPDF_UNSP_ANNOT_SOUND => UnsupportedFeature::AnnotSound,
            FPDF_UNSP_ANNOT_SCREEN_MEDIA => UnsupportedFeature::AnnotScreenMedia,
            FPDF_UNSP_ANNOT_SCREEN_RICHMEDIA => UnsupportedFeature::AnnotScreenRichMedia,
            FPDF_UNSP_ANNOT_ATTACHMENT => UnsupportedFeature::AnnotAttachment,
            FPDF_UNSP_ANNOT_SIG => UnsupportedFeature::AnnotSignature,
            _ => UnsupportedFeature::Unknown(value),
        }
    }

    /// Check if this is a document-level unsupported feature.
    pub fn is_document_level(&self) -> bool {
        matches!(
            self,
            UnsupportedFeature::XfaForm
                | UnsupportedFeature::PortableCollection
                | UnsupportedFeature::Attachment
                | UnsupportedFeature::Security
                | UnsupportedFeature::SharedReview
                | UnsupportedFeature::SharedFormAcrobat
                | UnsupportedFeature::SharedFormFilesystem
                | UnsupportedFeature::SharedFormEmail
        )
    }

    /// Check if this is an annotation-level unsupported feature.
    pub fn is_annotation_level(&self) -> bool {
        matches!(
            self,
            UnsupportedFeature::Annot3D
                | UnsupportedFeature::AnnotMovie
                | UnsupportedFeature::AnnotSound
                | UnsupportedFeature::AnnotScreenMedia
                | UnsupportedFeature::AnnotScreenRichMedia
                | UnsupportedFeature::AnnotAttachment
                | UnsupportedFeature::AnnotSignature
        )
    }

    /// Get a human-readable description of the unsupported feature.
    pub fn description(&self) -> &'static str {
        match self {
            UnsupportedFeature::XfaForm => "XFA forms (dynamic XML forms)",
            UnsupportedFeature::PortableCollection => "Portable Collection (PDF Package)",
            UnsupportedFeature::Attachment => "Document attachment",
            UnsupportedFeature::Security => "Security feature",
            UnsupportedFeature::SharedReview => "Shared review workflow",
            UnsupportedFeature::SharedFormAcrobat => "Shared form via Acrobat.com",
            UnsupportedFeature::SharedFormFilesystem => "Shared form via filesystem",
            UnsupportedFeature::SharedFormEmail => "Shared form via email",
            UnsupportedFeature::Annot3D => "3D annotation",
            UnsupportedFeature::AnnotMovie => "Movie annotation",
            UnsupportedFeature::AnnotSound => "Sound annotation",
            UnsupportedFeature::AnnotScreenMedia => "Screen media annotation",
            UnsupportedFeature::AnnotScreenRichMedia => "Screen rich media annotation",
            UnsupportedFeature::AnnotAttachment => "Attachment annotation",
            UnsupportedFeature::AnnotSignature => "Signature annotation",
            UnsupportedFeature::Unknown(_) => "Unknown feature",
        }
    }
}

/// Type alias for the unsupported feature handler callback.
type UnsupportedHandlerFn = Box<dyn Fn(UnsupportedFeature) + Send + Sync>;

/// Global callback storage for unsupported feature handler.
///
/// This uses a mutex to safely store a boxed callback function.
/// The callback is invoked when PDFium encounters an unsupported feature.
static UNSUPPORTED_HANDLER: std::sync::Mutex<Option<UnsupportedHandlerFn>> =
    std::sync::Mutex::new(None);

/// Static UNSUPPORT_INFO structure that PDFium uses for callbacks.
/// Must be static because PDFium stores a pointer to it.
static mut UNSUPPORT_INFO_STRUCT: _UNSUPPORT_INFO = _UNSUPPORT_INFO {
    version: 1,
    FSDK_UnSupport_Handler: Some(unsupported_handler_callback),
};

/// C callback function that PDFium calls when it encounters an unsupported feature.
unsafe extern "C" fn unsupported_handler_callback(_p_this: *mut _UNSUPPORT_INFO, n_type: i32) {
    if let Ok(guard) = UNSUPPORTED_HANDLER.lock() {
        if let Some(ref callback) = *guard {
            let feature = UnsupportedFeature::from_raw(n_type);
            callback(feature);
        }
    }
}

/// Set a global handler for unsupported PDF features.
///
/// When PDFium encounters a feature it doesn't support (like XFA forms,
/// 3D annotations, or movie annotations), it will call the provided callback.
///
/// This can be useful for:
/// - Logging which unsupported features are encountered
/// - Showing warnings to users
/// - Tracking feature usage in your PDF corpus
///
/// # Arguments
///
/// * `handler` - A callback function that receives the [`UnsupportedFeature`] type.
///   Pass `None` to disable the handler.
///
/// # Returns
///
/// `true` if the handler was successfully set, `false` otherwise.
///
/// # Example
///
/// ```no_run
/// use pdfium_render_fast::{Pdfium, UnsupportedFeature, set_unsupported_feature_handler};
///
/// // Set up a handler to log unsupported features
/// set_unsupported_feature_handler(Some(|feature: UnsupportedFeature| {
///     eprintln!("Warning: Unsupported feature: {}", feature.description());
/// }));
///
/// let pdfium = Pdfium::new()?;
/// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
/// // If the document has unsupported features, your handler will be called
///
/// // Disable the handler when done
/// set_unsupported_feature_handler(None::<fn(UnsupportedFeature)>);
/// # Ok::<(), pdfium_render_fast::PdfError>(())
/// ```
///
/// # Thread Safety
///
/// The handler is global and shared across all threads. Only one handler
/// can be active at a time. Setting a new handler replaces the previous one.
pub fn set_unsupported_feature_handler<F>(handler: Option<F>) -> bool
where
    F: Fn(UnsupportedFeature) + Send + Sync + 'static,
{
    // Update the handler in our storage
    if let Ok(mut guard) = UNSUPPORTED_HANDLER.lock() {
        *guard = handler.map(|f| Box::new(f) as Box<dyn Fn(UnsupportedFeature) + Send + Sync>);
    } else {
        return false;
    }

    // Register or re-register with PDFium
    unsafe {
        let result = FSDK_SetUnSpObjProcessHandler(&raw mut UNSUPPORT_INFO_STRUCT);
        result != 0
    }
}

/// Clear the unsupported feature handler.
///
/// This is equivalent to calling `set_unsupported_feature_handler(None::<fn(UnsupportedFeature)>)`.
pub fn clear_unsupported_feature_handler() {
    if let Ok(mut guard) = UNSUPPORTED_HANDLER.lock() {
        *guard = None;
    }
}

// ============================================================================
// Data Availability API (fpdf_dataavail.h)
// ============================================================================

/// Result of checking PDF linearization status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinearizationStatus {
    /// The PDF is linearized (web-optimized for streaming).
    Linearized,
    /// The PDF is not linearized.
    NotLinearized,
    /// Linearization status cannot be determined (insufficient data).
    Unknown,
}

impl LinearizationStatus {
    fn from_raw(value: i32) -> Self {
        match value {
            x if x == pdfium_sys::PDF_LINEARIZED as i32 => LinearizationStatus::Linearized,
            x if x == pdfium_sys::PDF_NOT_LINEARIZED as i32 => LinearizationStatus::NotLinearized,
            _ => LinearizationStatus::Unknown,
        }
    }
}

/// Result of checking data availability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataAvailability {
    /// Data is available and ready.
    Available,
    /// Data is not yet available (more data needed).
    NotAvailable,
    /// An error occurred checking availability.
    Error,
}

impl DataAvailability {
    /// Convert from raw PDFium value.
    #[allow(dead_code)] // Reserved for future streaming API support
    pub fn from_raw(value: i32) -> Self {
        match value {
            x if x == pdfium_sys::PDF_DATA_AVAIL as i32 => DataAvailability::Available,
            x if x == pdfium_sys::PDF_DATA_NOTAVAIL as i32 => DataAvailability::NotAvailable,
            _ => DataAvailability::Error,
        }
    }
}

/// Result of checking form data availability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormAvailability {
    /// Form data is available and ready.
    Available,
    /// Form data is not yet available.
    NotAvailable,
    /// No form data exists in the document.
    NotExist,
    /// An error occurred.
    Error,
}

impl FormAvailability {
    /// Convert from raw PDFium value.
    #[allow(dead_code)] // Reserved for future streaming API support
    pub fn from_raw(value: i32) -> Self {
        match value {
            x if x == pdfium_sys::PDF_FORM_AVAIL as i32 => FormAvailability::Available,
            x if x == pdfium_sys::PDF_FORM_NOTAVAIL as i32 => FormAvailability::NotAvailable,
            x if x == pdfium_sys::PDF_FORM_NOTEXIST as i32 => FormAvailability::NotExist,
            _ => FormAvailability::Error,
        }
    }
}

/// A simple file availability checker that reports all data as available.
///
/// This is useful for checking linearization status of fully-loaded documents.
struct FullyAvailableFile {
    file_avail: pdfium_sys::_FX_FILEAVAIL,
    file_access: pdfium_sys::FPDF_FILEACCESS,
    data: Vec<u8>,
}

/// Callback for IsDataAvail - always returns true (all data available).
unsafe extern "C" fn is_data_avail_callback(
    _p_this: *mut pdfium_sys::_FX_FILEAVAIL,
    _offset: usize,
    _size: usize,
) -> pdfium_sys::FPDF_BOOL {
    1 // true - all data is available
}

/// Callback for reading file data.
unsafe extern "C" fn get_block_callback(
    param: *mut std::ffi::c_void,
    position: std::os::raw::c_ulong,
    p_buf: *mut std::os::raw::c_uchar,
    size: std::os::raw::c_ulong,
) -> std::os::raw::c_int {
    let file = &*(param as *const FullyAvailableFile);
    let pos = position as usize;
    let sz = size as usize;

    if pos + sz > file.data.len() {
        return 0; // Error: out of bounds
    }

    std::ptr::copy_nonoverlapping(file.data.as_ptr().add(pos), p_buf, sz);
    1 // Success
}

impl FullyAvailableFile {
    fn new(data: Vec<u8>) -> Box<Self> {
        let len = data.len();
        let mut file = Box::new(FullyAvailableFile {
            file_avail: pdfium_sys::_FX_FILEAVAIL {
                version: 1,
                IsDataAvail: Some(is_data_avail_callback),
            },
            file_access: pdfium_sys::FPDF_FILEACCESS {
                m_FileLen: len as std::os::raw::c_ulong,
                m_GetBlock: Some(get_block_callback),
                m_Param: std::ptr::null_mut(),
            },
            data,
        });

        // Set the param to point to ourselves
        file.file_access.m_Param = file.as_ref() as *const _ as *mut std::ffi::c_void;

        file
    }
}

/// Check if a PDF document (loaded from bytes) is linearized.
///
/// Linearized PDFs are optimized for web viewing, allowing the first page
/// to be displayed before the entire file is downloaded.
///
/// # Arguments
///
/// * `data` - The PDF file data
///
/// # Returns
///
/// The linearization status of the PDF.
///
/// # Example
///
/// ```no_run
/// use pdfium_render_fast::{check_linearization, LinearizationStatus};
///
/// let pdf_data = std::fs::read("document.pdf")?;
/// match check_linearization(&pdf_data) {
///     LinearizationStatus::Linearized => println!("PDF is web-optimized"),
///     LinearizationStatus::NotLinearized => println!("PDF is not linearized"),
///     LinearizationStatus::Unknown => println!("Could not determine"),
/// }
/// # Ok::<(), std::io::Error>(())
/// ```
pub fn check_linearization(data: &[u8]) -> LinearizationStatus {
    // Need at least 1KB to determine linearization
    if data.len() < 1024 {
        return LinearizationStatus::Unknown;
    }

    // Create a fully available file wrapper
    let mut file = FullyAvailableFile::new(data.to_vec());

    // Create availability provider
    let avail = unsafe {
        pdfium_sys::FPDFAvail_Create(
            &mut file.file_avail as *mut _,
            &mut file.file_access as *mut _,
        )
    };

    if avail.is_null() {
        return LinearizationStatus::Unknown;
    }

    // Check linearization
    let result = unsafe { pdfium_sys::FPDFAvail_IsLinearized(avail) };

    // Cleanup
    unsafe {
        pdfium_sys::FPDFAvail_Destroy(avail);
    }

    LinearizationStatus::from_raw(result)
}

/// Get the first available page number for a linearized PDF.
///
/// For most linearized PDFs, this returns 0 (the first page).
/// For non-linearized PDFs, this always returns 0.
///
/// # Arguments
///
/// * `doc` - A reference to the PDF document
///
/// # Returns
///
/// The zero-based index of the first available page.
pub fn get_first_available_page(doc: &PdfDocument) -> i32 {
    unsafe { pdfium_sys::FPDFAvail_GetFirstPageNum(doc.handle()) }
}

// ============================================================================
// System Font Info API (fpdf_sysfontinfo.h)
// ============================================================================

/// Font character set identifiers.
///
/// These identify the character encoding/language support of fonts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum FontCharset {
    /// ANSI (Western European)
    Ansi = pdfium_sys::FXFONT_ANSI_CHARSET,
    /// Default system charset
    Default = pdfium_sys::FXFONT_DEFAULT_CHARSET,
    /// Symbol charset (special characters)
    Symbol = pdfium_sys::FXFONT_SYMBOL_CHARSET,
    /// Japanese (Shift-JIS)
    ShiftJis = pdfium_sys::FXFONT_SHIFTJIS_CHARSET,
    /// Korean (Hangeul)
    Hangeul = pdfium_sys::FXFONT_HANGEUL_CHARSET,
    /// Simplified Chinese (GB2312)
    Gb2312 = pdfium_sys::FXFONT_GB2312_CHARSET,
    /// Traditional Chinese (Big5)
    ChineseBig5 = pdfium_sys::FXFONT_CHINESEBIG5_CHARSET,
    /// Greek
    Greek = pdfium_sys::FXFONT_GREEK_CHARSET,
    /// Vietnamese
    Vietnamese = pdfium_sys::FXFONT_VIETNAMESE_CHARSET,
    /// Hebrew
    Hebrew = pdfium_sys::FXFONT_HEBREW_CHARSET,
    /// Arabic
    Arabic = pdfium_sys::FXFONT_ARABIC_CHARSET,
    /// Cyrillic (Russian, etc.)
    Cyrillic = pdfium_sys::FXFONT_CYRILLIC_CHARSET,
    /// Thai
    Thai = pdfium_sys::FXFONT_THAI_CHARSET,
    /// Eastern European
    EasternEuropean = pdfium_sys::FXFONT_EASTERNEUROPEAN_CHARSET,
}

impl FontCharset {
    /// Convert from raw charset value.
    pub fn from_raw(value: i32) -> Option<Self> {
        match value as u32 {
            pdfium_sys::FXFONT_ANSI_CHARSET => Some(FontCharset::Ansi),
            pdfium_sys::FXFONT_DEFAULT_CHARSET => Some(FontCharset::Default),
            pdfium_sys::FXFONT_SYMBOL_CHARSET => Some(FontCharset::Symbol),
            pdfium_sys::FXFONT_SHIFTJIS_CHARSET => Some(FontCharset::ShiftJis),
            pdfium_sys::FXFONT_HANGEUL_CHARSET => Some(FontCharset::Hangeul),
            pdfium_sys::FXFONT_GB2312_CHARSET => Some(FontCharset::Gb2312),
            pdfium_sys::FXFONT_CHINESEBIG5_CHARSET => Some(FontCharset::ChineseBig5),
            pdfium_sys::FXFONT_GREEK_CHARSET => Some(FontCharset::Greek),
            pdfium_sys::FXFONT_VIETNAMESE_CHARSET => Some(FontCharset::Vietnamese),
            pdfium_sys::FXFONT_HEBREW_CHARSET => Some(FontCharset::Hebrew),
            pdfium_sys::FXFONT_ARABIC_CHARSET => Some(FontCharset::Arabic),
            pdfium_sys::FXFONT_CYRILLIC_CHARSET => Some(FontCharset::Cyrillic),
            pdfium_sys::FXFONT_THAI_CHARSET => Some(FontCharset::Thai),
            pdfium_sys::FXFONT_EASTERNEUROPEAN_CHARSET => Some(FontCharset::EasternEuropean),
            _ => None,
        }
    }

    /// Get a human-readable name for this charset.
    pub fn name(&self) -> &'static str {
        match self {
            FontCharset::Ansi => "ANSI (Western European)",
            FontCharset::Default => "Default",
            FontCharset::Symbol => "Symbol",
            FontCharset::ShiftJis => "Japanese (Shift-JIS)",
            FontCharset::Hangeul => "Korean (Hangeul)",
            FontCharset::Gb2312 => "Simplified Chinese (GB2312)",
            FontCharset::ChineseBig5 => "Traditional Chinese (Big5)",
            FontCharset::Greek => "Greek",
            FontCharset::Vietnamese => "Vietnamese",
            FontCharset::Hebrew => "Hebrew",
            FontCharset::Arabic => "Arabic",
            FontCharset::Cyrillic => "Cyrillic",
            FontCharset::Thai => "Thai",
            FontCharset::EasternEuropean => "Eastern European",
        }
    }
}

/// A mapping from character set to TrueType font name.
#[derive(Debug, Clone)]
pub struct CharsetFontMapping {
    /// The character set identifier.
    pub charset: FontCharset,
    /// The raw charset value (for unknown charsets).
    pub charset_raw: i32,
    /// The font name (e.g., "Arial", "SimSun").
    pub font_name: String,
}

/// Get the number of entries in the default charset-to-font map.
///
/// # Returns
///
/// The number of charset-to-font mappings.
pub fn get_default_ttf_map_count() -> usize {
    unsafe { pdfium_sys::FPDF_GetDefaultTTFMapCount() }
}

/// Get a specific entry from the default charset-to-font map.
///
/// # Arguments
///
/// * `index` - The index of the entry (0-based)
///
/// # Returns
///
/// The charset-to-font mapping at the given index, or `None` if out of bounds.
///
/// # Example
///
/// ```no_run
/// use pdfium_render_fast::get_default_ttf_map_entry;
///
/// if let Some(mapping) = get_default_ttf_map_entry(0) {
///     println!("Charset {:?} uses font: {}", mapping.charset, mapping.font_name);
/// }
/// ```
pub fn get_default_ttf_map_entry(index: usize) -> Option<CharsetFontMapping> {
    let entry = unsafe { pdfium_sys::FPDF_GetDefaultTTFMapEntry(index) };

    if entry.is_null() {
        return None;
    }

    let raw_entry = unsafe { &*entry };

    // Check for sentinel (charset == -1)
    if raw_entry.charset == -1 {
        return None;
    }

    let font_name = if raw_entry.fontname.is_null() {
        String::new()
    } else {
        unsafe {
            std::ffi::CStr::from_ptr(raw_entry.fontname)
                .to_string_lossy()
                .into_owned()
        }
    };

    Some(CharsetFontMapping {
        charset: FontCharset::from_raw(raw_entry.charset).unwrap_or(FontCharset::Default),
        charset_raw: raw_entry.charset,
        font_name,
    })
}

/// Get all entries from the default charset-to-font map.
///
/// # Returns
///
/// A vector of all charset-to-font mappings.
///
/// # Example
///
/// ```no_run
/// use pdfium_render_fast::get_default_ttf_map;
///
/// for mapping in get_default_ttf_map() {
///     println!("{}: {}", mapping.charset.name(), mapping.font_name);
/// }
/// ```
pub fn get_default_ttf_map() -> Vec<CharsetFontMapping> {
    let count = get_default_ttf_map_count();
    let mut mappings = Vec::with_capacity(count);

    for i in 0..count {
        if let Some(mapping) = get_default_ttf_map_entry(i) {
            mappings.push(mapping);
        }
    }

    mappings
}
