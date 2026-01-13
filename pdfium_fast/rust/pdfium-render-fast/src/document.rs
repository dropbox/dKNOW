//! PDF document representation

use crate::attachment::PdfAttachments;
use crate::bookmark::{flatten_bookmarks, FlatBookmark, PdfBookmarkIter};
use crate::error::{PdfError, Result};
use crate::form::PdfFormType;
use crate::javascript::PdfJavaScriptActions;
use crate::page::PdfPage;
use crate::pdfium::Pdfium;
use crate::render::{PdfRenderConfig, PixelFormat, RenderedPage};
use crate::signature::PdfSignatures;
use pdfium_sys::*;
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Flags for controlling how a document is saved.
///
/// These flags affect the format and content of the saved PDF file.
#[derive(Debug, Clone, Copy, Default)]
pub struct SaveFlags {
    /// Save incrementally (append changes instead of rewriting).
    /// More efficient for small changes but may result in larger files.
    pub incremental: bool,

    /// Remove security/encryption from the saved document.
    /// Use with caution - only if you have rights to do so.
    pub remove_security: bool,
}

impl SaveFlags {
    /// Create new save flags with defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable incremental save mode.
    ///
    /// Incremental saves append changes to the end of the file rather than
    /// rewriting the entire document. This is faster for small changes but
    /// may result in larger files over time.
    pub fn incremental(mut self) -> Self {
        self.incremental = true;
        self
    }

    /// Enable security removal.
    ///
    /// This will remove any encryption or password protection from the saved
    /// document. Only use this if you have the right to redistribute the
    /// document without security.
    pub fn remove_security(mut self) -> Self {
        self.remove_security = true;
        self
    }

    /// Convert to PDFium save flags.
    fn to_raw(self) -> u64 {
        if self.remove_security {
            FPDF_REMOVE_SECURITY as u64
        } else if self.incremental {
            FPDF_INCREMENTAL as u64
        } else {
            FPDF_NO_INCREMENTAL as u64
        }
    }
}

/// Result of flattening a page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlattenResult {
    /// Flattening succeeded - annotations/forms converted to static content.
    Success,
    /// Nothing to flatten - page had no annotations or form fields.
    NothingToDo,
    /// Flattening failed.
    Fail,
}

impl FlattenResult {
    /// Convert from raw PDFium value.
    pub fn from_raw(value: i32) -> Self {
        match value as u32 {
            FLATTEN_SUCCESS => FlattenResult::Success,
            FLATTEN_NOTHINGTODO => FlattenResult::NothingToDo,
            _ => FlattenResult::Fail,
        }
    }
}

/// Mode for flattening annotations.
#[derive(Debug, Clone, Copy, Default)]
pub enum FlattenMode {
    /// Flatten for normal display (all annotations visible).
    #[default]
    Display,
    /// Flatten for print (only print-visible annotations).
    Print,
}

impl FlattenMode {
    /// Convert to raw PDFium value.
    pub fn to_raw(&self) -> i32 {
        match self {
            FlattenMode::Display => FLAT_NORMALDISPLAY as i32,
            FlattenMode::Print => FLAT_PRINT as i32,
        }
    }
}

/// The initial page mode to use when opening the document.
///
/// This specifies how the document should be displayed when first opened.
/// The page mode is set by the document creator and stored in the document catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PageMode {
    /// Document outline and thumbnails are hidden.
    #[default]
    UseNone,
    /// Document outline (bookmarks) is visible.
    UseOutlines,
    /// Thumbnail images are visible.
    UseThumbs,
    /// Full-screen mode, no menu bar, window controls, or other decorations visible.
    FullScreen,
    /// Optional content group panel is visible.
    UseOC,
    /// Attachments panel is visible.
    UseAttachments,
    /// Unknown page mode.
    Unknown(i32),
}

impl PageMode {
    /// Convert from raw PDFium value.
    pub fn from_raw(value: i32) -> Self {
        match value {
            -1 => PageMode::Unknown(-1), // PAGEMODE_UNKNOWN
            0 => PageMode::UseNone,
            1 => PageMode::UseOutlines,
            2 => PageMode::UseThumbs,
            3 => PageMode::FullScreen,
            4 => PageMode::UseOC,
            5 => PageMode::UseAttachments,
            other => PageMode::Unknown(other),
        }
    }

    /// Check if this is a "normal" display mode (not full-screen or special panel).
    pub fn is_normal(&self) -> bool {
        matches!(
            self,
            PageMode::UseNone | PageMode::UseOutlines | PageMode::UseThumbs
        )
    }

    /// Check if this is full-screen mode.
    pub fn is_fullscreen(&self) -> bool {
        matches!(self, PageMode::FullScreen)
    }
}

/// Duplex mode for printing as specified in viewer preferences.
///
/// This specifies how pages should be oriented when printing double-sided.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DuplexType {
    /// No preference specified.
    #[default]
    Undefined,
    /// Single-sided printing.
    Simplex,
    /// Double-sided, flip on short edge.
    FlipShortEdge,
    /// Double-sided, flip on long edge.
    FlipLongEdge,
}

impl DuplexType {
    /// Convert from raw PDFium value.
    pub fn from_raw(value: FPDF_DUPLEXTYPE) -> Self {
        match value {
            0 => DuplexType::Undefined,     // DuplexUndefined
            1 => DuplexType::Simplex,       // Simplex
            2 => DuplexType::FlipShortEdge, // DuplexFlipShortEdge
            3 => DuplexType::FlipLongEdge,  // DuplexFlipLongEdge
            _ => DuplexType::Undefined,
        }
    }

    /// Check if this is a duplex (double-sided) mode.
    pub fn is_duplex(&self) -> bool {
        matches!(self, DuplexType::FlipShortEdge | DuplexType::FlipLongEdge)
    }
}

/// A PDF document.
///
/// This struct provides access to pages, metadata, and document-level operations.
/// Thread-safe: The document can be shared across threads.
///
/// # Example
///
/// ```no_run
/// use pdfium_render_fast::Pdfium;
///
/// let pdfium = Pdfium::new()?;
/// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
///
/// println!("Pages: {}", doc.page_count());
///
/// for page in doc.pages() {
///     // Process each page
/// }
/// # Ok::<(), pdfium_render_fast::PdfError>(())
/// ```
pub struct PdfDocument {
    inner: Arc<PdfDocumentInner>,
}

pub(crate) struct PdfDocumentInner {
    pub(crate) handle: FPDF_DOCUMENT,
    /// Form handle for form-enabled rendering
    pub(crate) form_handle: FPDF_FORMHANDLE,
    /// Form fill info - MUST be kept alive for form_handle lifetime
    /// PDFium keeps a reference to this struct, so we Box it to ensure stable address
    _form_fill_info: Option<Box<FPDF_FORMFILLINFO>>,
    /// Owned data for memory-loaded documents
    _data: Option<Vec<u8>>,
}

// SAFETY: PDFium document handles are thread-safe when accessed with proper synchronization
unsafe impl Send for PdfDocumentInner {}
unsafe impl Sync for PdfDocumentInner {}

impl PdfDocument {
    /// Create a new PdfDocument from a raw PDFium document handle.
    pub(crate) fn from_raw(handle: FPDF_DOCUMENT) -> Self {
        let (form_handle, form_fill_info) = Self::init_form_handle(handle);
        Self {
            inner: Arc::new(PdfDocumentInner {
                handle,
                form_handle,
                _form_fill_info: form_fill_info,
                _data: None,
            }),
        }
    }

    /// Create a new PdfDocument from a raw handle with owned data.
    pub(crate) fn from_raw_with_data(handle: FPDF_DOCUMENT, data: Vec<u8>) -> Self {
        let (form_handle, form_fill_info) = Self::init_form_handle(handle);
        Self {
            inner: Arc::new(PdfDocumentInner {
                handle,
                form_handle,
                _form_fill_info: form_fill_info,
                _data: Some(data),
            }),
        }
    }

    /// Initialize the form handle for form-enabled rendering.
    /// Returns the form handle and the boxed form fill info struct that must be kept alive.
    fn init_form_handle(doc: FPDF_DOCUMENT) -> (FPDF_FORMHANDLE, Option<Box<FPDF_FORMFILLINFO>>) {
        unsafe {
            let mut form_fill_info = Box::new(std::mem::zeroed::<FPDF_FORMFILLINFO>());
            form_fill_info.version = 2;
            let form_handle = FPDFDOC_InitFormFillEnvironment(doc, form_fill_info.as_mut());
            if form_handle.is_null() {
                (std::ptr::null_mut(), None)
            } else {
                (form_handle, Some(form_fill_info))
            }
        }
    }

    /// Get the raw document handle.
    ///
    /// # Safety
    ///
    /// The handle must not be used after the document is dropped.
    pub fn handle(&self) -> FPDF_DOCUMENT {
        self.inner.handle
    }

    /// Get the number of pages in the document.
    pub fn page_count(&self) -> usize {
        unsafe { FPDF_GetPageCount(self.inner.handle) as usize }
    }

    /// Get a page by index (0-based).
    ///
    /// # Arguments
    ///
    /// * `index` - Page index (0-based)
    ///
    /// # Returns
    ///
    /// The page at the given index, or an error if the index is out of bounds.
    pub fn page(&self, index: usize) -> Result<PdfPage> {
        let count = self.page_count();
        if index >= count {
            return Err(PdfError::PageIndexOutOfBounds { index, count });
        }

        let page = unsafe { FPDF_LoadPage(self.inner.handle, index as i32) };
        if page.is_null() {
            return Err(PdfError::PageLoadFailed { index });
        }

        // Initialize form for this page
        if !self.inner.form_handle.is_null() {
            unsafe {
                FORM_OnAfterLoadPage(page, self.inner.form_handle);
                FORM_DoPageAAction(page, self.inner.form_handle, FPDFPAGE_AACTION_OPEN as i32);
            }
        }

        Ok(PdfPage::new(page, self.inner.clone(), index))
    }

    /// Get an iterator over all pages in the document.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    ///
    /// for page in doc.pages() {
    ///     println!("Page size: {:?}", page.size());
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn pages(&self) -> PdfPages<'_> {
        PdfPages {
            doc: self,
            index: 0,
            count: self.page_count(),
        }
    }

    /// Check if the document is tagged (has structure tree).
    ///
    /// Tagged PDFs contain semantic structure that can be used to skip
    /// ML layout detection.
    pub fn is_tagged(&self) -> bool {
        unsafe { FPDFCatalog_IsTagged(self.inner.handle) != 0 }
    }

    /// Set the language of the document.
    ///
    /// The language should be an RFC 3066 language tag (e.g., "en-US", "ja", "zh-CN").
    /// This is important for accessibility as it tells screen readers which
    /// language to use for pronunciation.
    ///
    /// # Arguments
    ///
    /// * `language` - Language tag (e.g., "en", "en-US", "ja", "zh-CN")
    ///
    /// # Returns
    ///
    /// * `true` if the language was set successfully
    /// * `false` if the operation failed
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    ///
    /// // Set document language for accessibility
    /// doc.set_language("en-US");
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn set_language(&self, language: &str) -> bool {
        let c_language = std::ffi::CString::new(language).ok();
        match c_language {
            Some(lang) => unsafe { FPDFCatalog_SetLanguage(self.inner.handle, lang.as_ptr()) != 0 },
            None => false,
        }
    }

    /// Render all pages in parallel using multiple threads.
    ///
    /// This method uses PDFium's optimized parallel rendering API for
    /// significant speedup on multi-page documents (up to 72x faster).
    ///
    /// # Arguments
    ///
    /// * `config` - Render configuration (DPI, pixel format)
    ///
    /// # Returns
    ///
    /// A vector of `RenderedPage` structs, one per page, ordered by page index.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::{Pdfium, PdfRenderConfig};
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    ///
    /// // Render all pages at 150 DPI with 8 threads
    /// let pages = doc.render_pages_parallel(&PdfRenderConfig::new().set_target_dpi(150.0))?;
    ///
    /// for page in &pages {
    ///     page.save_as_png(&format!("page_{}.png", page.page_index))?;
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn render_pages_parallel(&self, config: &PdfRenderConfig) -> Result<Vec<RenderedPage>> {
        self.render_pages_parallel_range(0, self.page_count(), config, None)
    }

    /// Render all pages in parallel with explicit thread count.
    ///
    /// # Arguments
    ///
    /// * `config` - Render configuration
    /// * `threads` - Number of render threads (0 = auto-detect)
    pub fn render_pages_parallel_threaded(
        &self,
        config: &PdfRenderConfig,
        threads: i32,
    ) -> Result<Vec<RenderedPage>> {
        self.render_pages_parallel_range(0, self.page_count(), config, Some(threads))
    }

    /// Render a range of pages in parallel.
    ///
    /// # Arguments
    ///
    /// * `start` - First page index (0-based)
    /// * `count` - Number of pages to render
    /// * `config` - Render configuration
    /// * `threads` - Optional thread count (None = optimal, Some(0) = auto)
    pub fn render_pages_parallel_range(
        &self,
        start: usize,
        count: usize,
        config: &PdfRenderConfig,
        threads: Option<i32>,
    ) -> Result<Vec<RenderedPage>> {
        if count == 0 {
            return Ok(Vec::new());
        }

        let page_count = self.page_count();
        if start >= page_count {
            return Err(PdfError::PageIndexOutOfBounds {
                index: start,
                count: page_count,
            });
        }

        let end = (start + count).min(page_count);
        let actual_count = end - start;

        // Determine thread count
        let thread_count = match threads {
            Some(t) => t,
            None => unsafe { FPDF_GetOptimalWorkerCountForDocument(self.inner.handle) },
        };

        // Set up parallel rendering options
        let mut options: FPDF_PARALLEL_OPTIONS = unsafe { std::mem::zeroed() };
        options.worker_count = thread_count;
        options.form_handle = self.inner.form_handle as *mut std::ffi::c_void;
        options.dpi = config.dpi();
        options.output_format = match config.pixel_format() {
            PixelFormat::Bgra => FPDF_PARALLEL_FORMAT_BGRx as i32,
            PixelFormat::Bgr => FPDF_PARALLEL_FORMAT_BGR as i32,
            PixelFormat::Gray => FPDF_PARALLEL_FORMAT_GRAY as i32,
        };

        // Create shared state for collecting results
        let results: Arc<Mutex<Vec<Option<RenderedPage>>>> =
            Arc::new(Mutex::new(vec![None; actual_count]));
        let format = config.pixel_format();

        // Create callback context
        struct CallbackContext {
            results: Arc<Mutex<Vec<Option<RenderedPage>>>>,
            start_page: usize,
            format: PixelFormat,
        }

        let context = Box::new(CallbackContext {
            results: results.clone(),
            start_page: start,
            format,
        });
        let context_ptr = Box::into_raw(context);

        // V2 callback - receives raw buffer (more efficient, no bitmap allocation)
        extern "C" fn callback_v2(
            page_index: i32,
            buffer: *const std::ffi::c_void,
            width: i32,
            height: i32,
            stride: i32,
            user_data: *mut std::ffi::c_void,
            success: i32,
        ) {
            let ctx = unsafe { &*(user_data as *const CallbackContext) };

            if success != 0 && !buffer.is_null() {
                let data_size = (height * stride) as usize;
                let data =
                    unsafe { std::slice::from_raw_parts(buffer as *const u8, data_size).to_vec() };

                let page = RenderedPage::new(
                    page_index as usize,
                    width as u32,
                    height as u32,
                    stride as u32,
                    data,
                    ctx.format,
                );

                let slot = page_index as usize - ctx.start_page;
                if let Ok(mut results) = ctx.results.lock() {
                    if slot < results.len() {
                        results[slot] = Some(page);
                    }
                }
            }
        }

        // Call parallel rendering
        let success = unsafe {
            FPDF_RenderPagesParallelV2(
                self.inner.handle,
                start as i32,
                actual_count as i32,
                0, // width=0, use DPI-based calculation
                0, // height=0, use DPI-based calculation
                0, // rotation
                (FPDF_ANNOT | FPDF_PRINTING) as i32,
                &mut options,
                Some(callback_v2),
                context_ptr as *mut std::ffi::c_void,
            )
        };

        // Clean up context
        unsafe {
            drop(Box::from_raw(context_ptr));
        }

        if success == 0 {
            return Err(PdfError::RenderFailed {
                reason: "FPDF_RenderPagesParallelV2 failed".to_string(),
            });
        }

        // Collect results
        let results = Arc::try_unwrap(results)
            .map_err(|_| PdfError::RenderFailed {
                reason: "Failed to unwrap results".to_string(),
            })?
            .into_inner()
            .map_err(|_| PdfError::RenderFailed {
                reason: "Mutex poisoned".to_string(),
            })?;

        // Convert Option<RenderedPage> to RenderedPage, filtering out failures
        let pages: Vec<RenderedPage> = results.into_iter().flatten().collect();

        if pages.len() != actual_count {
            return Err(PdfError::RenderFailed {
                reason: format!(
                    "Only {} of {} pages rendered successfully",
                    pages.len(),
                    actual_count
                ),
            });
        }

        Ok(pages)
    }

    /// Get the optimal thread count for parallel rendering of this document.
    ///
    /// Returns a thread count optimized for the document's page count and
    /// the system's CPU cores.
    pub fn optimal_thread_count(&self) -> i32 {
        unsafe { FPDF_GetOptimalWorkerCountForDocument(self.inner.handle) }
    }

    /// Get document metadata.
    ///
    /// # Arguments
    ///
    /// * `tag` - Metadata tag (e.g., "Title", "Author", "Subject", "Keywords",
    ///   "Creator", "Producer", "CreationDate", "ModDate")
    ///
    /// # Returns
    ///
    /// The metadata value, or None if not present.
    pub fn metadata(&self, tag: &str) -> Option<String> {
        use std::ffi::CString;

        let c_tag = CString::new(tag).ok()?;

        // First, get the required buffer size
        let size =
            unsafe { FPDF_GetMetaText(self.inner.handle, c_tag.as_ptr(), std::ptr::null_mut(), 0) };

        if size <= 2 {
            // Empty or just null terminators
            return None;
        }

        // Allocate buffer and get the text (UTF-16LE)
        let mut buffer: Vec<u8> = vec![0; size as usize];
        unsafe {
            FPDF_GetMetaText(
                self.inner.handle,
                c_tag.as_ptr(),
                buffer.as_mut_ptr() as *mut std::ffi::c_void,
                size,
            );
        }

        // Convert from UTF-16LE to String
        // Remove trailing null characters
        while buffer.len() >= 2 && buffer[buffer.len() - 1] == 0 && buffer[buffer.len() - 2] == 0 {
            buffer.pop();
            buffer.pop();
        }

        // Convert UTF-16LE bytes to u16 code units
        let utf16: Vec<u16> = buffer
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();

        String::from_utf16(&utf16).ok()
    }

    /// Get the document title.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    ///
    /// if let Some(title) = doc.title() {
    ///     println!("Title: {}", title);
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn title(&self) -> Option<String> {
        self.metadata("Title")
    }

    /// Get the document author.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    ///
    /// if let Some(author) = doc.author() {
    ///     println!("Author: {}", author);
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn author(&self) -> Option<String> {
        self.metadata("Author")
    }

    /// Get the document subject.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    ///
    /// if let Some(subject) = doc.subject() {
    ///     println!("Subject: {}", subject);
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn subject(&self) -> Option<String> {
        self.metadata("Subject")
    }

    /// Get the document keywords.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    ///
    /// if let Some(keywords) = doc.keywords() {
    ///     println!("Keywords: {}", keywords);
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn keywords(&self) -> Option<String> {
        self.metadata("Keywords")
    }

    /// Get the application that created the original document.
    ///
    /// This is the name of the application used to create the original content
    /// before it was converted to PDF.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    ///
    /// if let Some(creator) = doc.creator() {
    ///     println!("Created with: {}", creator);
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn creator(&self) -> Option<String> {
        self.metadata("Creator")
    }

    /// Get the application that produced the PDF.
    ///
    /// This is the name of the PDF library or tool that created the PDF file.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    ///
    /// if let Some(producer) = doc.producer() {
    ///     println!("PDF Producer: {}", producer);
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn producer(&self) -> Option<String> {
        self.metadata("Producer")
    }

    /// Get the document creation date.
    ///
    /// Returns the date as a string in PDF date format (D:YYYYMMDDHHmmSSZ).
    /// Use a date parsing library to convert to a structured date.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    ///
    /// if let Some(created) = doc.creation_date() {
    ///     println!("Created: {}", created);
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn creation_date(&self) -> Option<String> {
        self.metadata("CreationDate")
    }

    /// Get the document modification date.
    ///
    /// Returns the date as a string in PDF date format (D:YYYYMMDDHHmmSSZ).
    /// Use a date parsing library to convert to a structured date.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    ///
    /// if let Some(modified) = doc.modification_date() {
    ///     println!("Modified: {}", modified);
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn modification_date(&self) -> Option<String> {
        self.metadata("ModDate")
    }

    /// Get an iterator over the root-level bookmarks (outline items) in the document.
    ///
    /// Bookmarks form a tree structure. Use `PdfBookmark::children()` to traverse
    /// child bookmarks.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    ///
    /// for bookmark in doc.bookmarks() {
    ///     println!("Bookmark: {} -> page {:?}",
    ///         bookmark.title().unwrap_or_default(),
    ///         bookmark.dest_page_index()
    ///     );
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn bookmarks(&self) -> PdfBookmarkIter {
        PdfBookmarkIter::new(self.inner.handle)
    }

    /// Check if the document has any bookmarks.
    pub fn has_bookmarks(&self) -> bool {
        let first = unsafe { FPDFBookmark_GetFirstChild(self.inner.handle, std::ptr::null_mut()) };
        !first.is_null()
    }

    /// Get all bookmarks as a flat list with depth information.
    ///
    /// This is useful for building a table of contents UI.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    ///
    /// for bookmark in doc.bookmarks_flat() {
    ///     let indent = "  ".repeat(bookmark.depth);
    ///     println!("{}{} -> page {:?}", indent, bookmark.title, bookmark.page_index);
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn bookmarks_flat(&self) -> Vec<FlatBookmark> {
        flatten_bookmarks(self.inner.handle)
    }

    /// Get the PDF file version.
    ///
    /// Returns the PDF version as an integer (e.g., 14 for PDF 1.4, 17 for PDF 1.7).
    pub fn file_version(&self) -> Option<i32> {
        let mut version: i32 = 0;
        let success = unsafe { FPDF_GetFileVersion(self.inner.handle, &mut version) };
        if success != 0 {
            Some(version)
        } else {
            None
        }
    }

    /// Get document permissions (security flags).
    ///
    /// Returns raw permission flags from the PDF. See PDF spec for flag meanings.
    pub fn permissions(&self) -> u32 {
        unsafe { FPDF_GetDocPermissions(self.inner.handle) as u32 }
    }

    /// Get the form type of this document.
    ///
    /// Returns whether the document uses AcroForms, XFA forms, or neither.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::{Pdfium, PdfFormType};
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("form.pdf", None)?;
    ///
    /// match doc.form_type() {
    ///     PdfFormType::AcroForm => println!("Standard PDF form"),
    ///     PdfFormType::XfaFull | PdfFormType::XfaForeground => println!("XFA form"),
    ///     PdfFormType::None => println!("No form"),
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn form_type(&self) -> PdfFormType {
        let raw = unsafe { FPDF_GetFormType(self.inner.handle) };
        PdfFormType::from_raw(raw)
    }

    /// Check if this document contains any form fields.
    pub fn has_forms(&self) -> bool {
        !matches!(self.form_type(), PdfFormType::None)
    }

    /// Get the page mode of this document.
    ///
    /// The page mode specifies how the document should be displayed when
    /// first opened by a PDF viewer.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::{Pdfium, PageMode};
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    ///
    /// match doc.page_mode() {
    ///     PageMode::UseOutlines => println!("Shows bookmarks panel"),
    ///     PageMode::UseThumbs => println!("Shows thumbnails panel"),
    ///     PageMode::FullScreen => println!("Opens in full screen"),
    ///     PageMode::UseNone => println!("Normal display"),
    ///     _ => println!("Other mode"),
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn page_mode(&self) -> PageMode {
        let raw = unsafe { FPDFDoc_GetPageMode(self.inner.handle) };
        PageMode::from_raw(raw)
    }

    // ========================================
    // Viewer Reference (Print Preferences) API
    // ========================================

    /// Get whether print scaling should be applied when printing.
    ///
    /// Returns `true` if the document should be scaled to fit the paper when printing,
    /// `false` if it should be printed at actual size.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    ///
    /// if doc.print_scaling() {
    ///     println!("Document will be scaled to fit paper");
    /// } else {
    ///     println!("Document will be printed at actual size");
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn print_scaling(&self) -> bool {
        unsafe { FPDF_VIEWERREF_GetPrintScaling(self.inner.handle) != 0 }
    }

    /// Get the number of copies to print as specified in viewer preferences.
    ///
    /// Returns the number of copies that should be printed by default,
    /// or 1 if not specified.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    ///
    /// println!("Default print copies: {}", doc.num_copies());
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn num_copies(&self) -> i32 {
        unsafe { FPDF_VIEWERREF_GetNumCopies(self.inner.handle) }
    }

    /// Get the duplex mode for printing as specified in viewer preferences.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::{Pdfium, DuplexType};
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    ///
    /// match doc.duplex_mode() {
    ///     DuplexType::Undefined => println!("No duplex preference"),
    ///     DuplexType::Simplex => println!("Single-sided printing"),
    ///     DuplexType::FlipShortEdge => println!("Duplex: flip on short edge"),
    ///     DuplexType::FlipLongEdge => println!("Duplex: flip on long edge"),
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn duplex_mode(&self) -> DuplexType {
        let raw = unsafe { FPDF_VIEWERREF_GetDuplex(self.inner.handle) };
        DuplexType::from_raw(raw)
    }

    /// Get the print page range as specified in viewer preferences.
    ///
    /// Returns a vector of (start, end) page indices (0-based) representing
    /// the ranges of pages to print. An empty vector means print all pages.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    ///
    /// let ranges = doc.print_page_ranges();
    /// if ranges.is_empty() {
    ///     println!("Print all pages");
    /// } else {
    ///     for (start, end) in &ranges {
    ///         println!("Print pages {} to {}", start + 1, end + 1);
    ///     }
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn print_page_ranges(&self) -> Vec<(i32, i32)> {
        let page_range = unsafe { FPDF_VIEWERREF_GetPrintPageRange(self.inner.handle) };
        if page_range.is_null() {
            return Vec::new();
        }

        let count = unsafe { FPDF_VIEWERREF_GetPrintPageRangeCount(page_range) };
        let mut ranges = Vec::new();

        // Page ranges are stored as pairs of (start, end) in consecutive elements
        let mut i = 0;
        while i + 1 < count {
            let start = unsafe { FPDF_VIEWERREF_GetPrintPageRangeElement(page_range, i) };
            let end = unsafe { FPDF_VIEWERREF_GetPrintPageRangeElement(page_range, i + 1) };
            ranges.push((start, end));
            i += 2;
        }

        ranges
    }

    /// Get a viewer preference value by name.
    ///
    /// This allows access to arbitrary viewer preferences stored in the document.
    /// Common preference names include:
    /// - "HideToolbar" - Hide the toolbar
    /// - "HideMenubar" - Hide the menu bar
    /// - "HideWindowUI" - Hide window controls
    /// - "FitWindow" - Fit window to page
    /// - "CenterWindow" - Center the window
    /// - "DisplayDocTitle" - Display document title in title bar
    /// - "Direction" - Reading direction ("L2R" or "R2L")
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    ///
    /// if let Some(direction) = doc.viewer_preference("Direction") {
    ///     println!("Reading direction: {}", direction);
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn viewer_preference(&self, key: &str) -> Option<String> {
        use std::ffi::CString;

        let c_key = CString::new(key).ok()?;

        // First call to get required buffer size
        let len = unsafe {
            FPDF_VIEWERREF_GetName(self.inner.handle, c_key.as_ptr(), std::ptr::null_mut(), 0)
        };

        if len == 0 {
            return None;
        }

        let mut buffer = vec![0u8; len as usize];
        let actual_len = unsafe {
            FPDF_VIEWERREF_GetName(
                self.inner.handle,
                c_key.as_ptr(),
                buffer.as_mut_ptr() as *mut i8,
                len,
            )
        };

        if actual_len == 0 {
            return None;
        }

        // Remove null terminator if present
        if let Some(&0) = buffer.last() {
            buffer.pop();
        }

        String::from_utf8(buffer).ok()
    }

    // ========================================
    // Named Destinations API
    // ========================================

    /// Get the count of named destinations in this document.
    ///
    /// Named destinations allow links to reference specific locations
    /// within a PDF by name rather than page number.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    ///
    /// println!("Document has {} named destinations", doc.named_dest_count());
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn named_dest_count(&self) -> usize {
        let count = unsafe { FPDF_CountNamedDests(self.inner.handle) };
        count as usize
    }

    /// Check if this document has any named destinations.
    pub fn has_named_dests(&self) -> bool {
        self.named_dest_count() > 0
    }

    /// Get a named destination by its name.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the destination to find
    ///
    /// # Returns
    ///
    /// * `Some(PdfDestination)` if found
    /// * `None` if no destination with that name exists
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    ///
    /// if let Some(dest) = doc.named_dest_by_name("chapter1") {
    ///     if let Some(page) = dest.page_index() {
    ///         println!("Chapter 1 is on page {}", page + 1);
    ///     }
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn named_dest_by_name(&self, name: &str) -> Option<crate::destination::PdfDestination> {
        let c_name = std::ffi::CString::new(name).ok()?;
        let dest = unsafe { FPDF_GetNamedDestByName(self.inner.handle, c_name.as_ptr()) };
        crate::destination::PdfDestination::new(dest, self.inner.handle)
    }

    /// Get a named destination by index.
    ///
    /// Returns both the destination and its name.
    ///
    /// # Arguments
    ///
    /// * `index` - Zero-based index of the destination (0 to named_dest_count() - 1)
    ///
    /// # Returns
    ///
    /// * `Some((name, destination))` if the index is valid
    /// * `None` if the index is out of range
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    ///
    /// for i in 0..doc.named_dest_count() {
    ///     if let Some((name, dest)) = doc.named_dest(i) {
    ///         if let Some(page) = dest.page_index() {
    ///             println!("{} -> page {}", name, page + 1);
    ///         }
    ///     }
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn named_dest(&self, index: usize) -> Option<(String, crate::destination::PdfDestination)> {
        // First call to get required buffer size
        let mut buflen: std::ffi::c_long = 0;
        let dest = unsafe {
            FPDF_GetNamedDest(
                self.inner.handle,
                index as i32,
                std::ptr::null_mut(),
                &mut buflen,
            )
        };

        if dest.is_null() || buflen <= 0 {
            return None;
        }

        // Allocate buffer and get name
        // buflen is in bytes, represents wchar_t buffer size
        let num_chars = buflen as usize / std::mem::size_of::<u16>();
        let mut buffer: Vec<u16> = vec![0; num_chars];

        let dest = unsafe {
            FPDF_GetNamedDest(
                self.inner.handle,
                index as i32,
                buffer.as_mut_ptr() as *mut std::ffi::c_void,
                &mut buflen,
            )
        };

        if dest.is_null() || buflen < 0 {
            return None;
        }

        // Convert UTF-16 to String (trim null terminator)
        let name = String::from_utf16_lossy(&buffer)
            .trim_end_matches('\0')
            .to_string();

        crate::destination::PdfDestination::new(dest, self.inner.handle).map(|d| (name, d))
    }

    /// Iterate over all named destinations in this document.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    ///
    /// for (name, dest) in doc.named_dests() {
    ///     if let Some(page) = dest.page_index() {
    ///         println!("{} -> page {}", name, page + 1);
    ///     }
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn named_dests(&self) -> PdfNamedDestsIter<'_> {
        PdfNamedDestsIter {
            doc: self,
            index: 0,
            count: self.named_dest_count(),
        }
    }

    /// Get access to the document's attachments (embedded files).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    ///
    /// let attachments = doc.attachments();
    /// println!("Document has {} attachments", attachments.count());
    ///
    /// for attachment in attachments.iter() {
    ///     if let Some(name) = attachment.name() {
    ///         println!("Attachment: {}", name);
    ///     }
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn attachments(&self) -> PdfAttachments {
        PdfAttachments::new(self.inner.handle)
    }

    /// Check if this document has any attachments.
    pub fn has_attachments(&self) -> bool {
        unsafe { FPDFDoc_GetAttachmentCount(self.inner.handle) > 0 }
    }

    /// Get the number of attachments in this document.
    pub fn attachment_count(&self) -> usize {
        unsafe {
            let count = FPDFDoc_GetAttachmentCount(self.inner.handle);
            if count > 0 {
                count as usize
            } else {
                0
            }
        }
    }

    /// Get the page label for a specific page.
    ///
    /// Page labels allow PDFs to use custom numbering schemes like
    /// "i, ii, iii" for front matter or "A-1, A-2" for appendices.
    ///
    /// # Arguments
    ///
    /// * `page_index` - Zero-based page index
    ///
    /// # Returns
    ///
    /// * `Ok(Some(label))` - The custom page label
    /// * `Ok(None)` - The page uses default numbering
    /// * `Err(PdfError)` - An error occurred
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    ///
    /// // Get label for first page
    /// if let Some(label) = doc.page_label(0)? {
    ///     println!("Page 1 is labeled: {}", label);
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn page_label(&self, page_index: usize) -> Result<Option<String>> {
        crate::page_label::get_page_label(self.inner.handle, page_index as i32)
    }

    /// Get all page labels for the document.
    ///
    /// Returns a vector of optional labels, one for each page.
    /// Pages without custom labels will have `None`.
    pub fn page_labels(&self) -> Result<Vec<Option<String>>> {
        let count = self.page_count();
        let mut labels = Vec::with_capacity(count);
        for i in 0..count {
            labels.push(self.page_label(i)?);
        }
        Ok(labels)
    }

    /// Get access to the document's digital signatures.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("signed.pdf", None)?;
    ///
    /// for sig in doc.signatures() {
    ///     println!("Reason: {:?}", sig.reason());
    ///     println!("Time: {:?}", sig.time());
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn signatures(&self) -> PdfSignatures {
        PdfSignatures::new(self.inner.handle)
    }

    /// Check if this document has any digital signatures.
    pub fn has_signatures(&self) -> bool {
        self.signature_count() > 0
    }

    /// Get the number of digital signatures in this document.
    pub fn signature_count(&self) -> usize {
        self.signatures().len()
    }

    // ========================================
    // JavaScript Actions API
    // ========================================

    /// Get access to document-level JavaScript actions.
    ///
    /// JavaScript in PDFs can be a security concern. This API allows you to:
    /// - Detect presence of JavaScript
    /// - Inspect script contents
    /// - Scan for suspicious patterns
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    ///
    /// // Security scan
    /// if doc.has_javascript() {
    ///     let js = doc.javascript_actions();
    ///     if js.has_any_suspicious() {
    ///         println!("WARNING: Document contains suspicious JavaScript!");
    ///     }
    ///     for action in js.iter() {
    ///         println!("Script: {}", action.name().unwrap_or_default());
    ///     }
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn javascript_actions(&self) -> PdfJavaScriptActions {
        PdfJavaScriptActions::new(self.inner.handle)
    }

    /// Check if this document contains any JavaScript actions.
    ///
    /// This is a quick check for security scanning - documents with JavaScript
    /// may pose security risks and should be handled carefully.
    pub fn has_javascript(&self) -> bool {
        self.javascript_count() > 0
    }

    /// Get the number of JavaScript actions in this document.
    pub fn javascript_count(&self) -> usize {
        self.javascript_actions().count()
    }

    /// Check if any JavaScript in this document has suspicious patterns.
    ///
    /// This is a convenience method for quick security scanning.
    /// Returns true if ANY script contains patterns associated with
    /// malicious PDFs (network access, file operations, external URLs).
    ///
    /// NOTE: This is a heuristic check. For security-critical applications,
    /// use a dedicated JavaScript analyzer.
    pub fn has_suspicious_javascript(&self) -> bool {
        self.javascript_actions().has_any_suspicious()
    }

    // ========================================
    // Document Save API
    // ========================================

    /// Save the document to a file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to save the PDF file
    /// * `flags` - Optional save flags (default: non-incremental)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::{Pdfium, SaveFlags};
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("input.pdf", None)?;
    ///
    /// // Save with default options
    /// doc.save_to_file("output.pdf", None)?;
    ///
    /// // Save incrementally
    /// doc.save_to_file("output.pdf", Some(SaveFlags::new().incremental()))?;
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P, flags: Option<SaveFlags>) -> Result<()> {
        let bytes = self.save_to_bytes(flags)?;
        let mut file = std::fs::File::create(path.as_ref()).map_err(|e| PdfError::IoError {
            message: format!("Failed to create output file: {}", e),
        })?;
        file.write_all(&bytes).map_err(|e| PdfError::IoError {
            message: format!("Failed to write PDF data: {}", e),
        })?;
        Ok(())
    }

    /// Save the document to a byte vector.
    ///
    /// # Arguments
    ///
    /// * `flags` - Optional save flags (default: non-incremental)
    ///
    /// # Returns
    ///
    /// A byte vector containing the PDF document data.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    ///
    /// // Get PDF as bytes
    /// let pdf_bytes = doc.save_to_bytes(None)?;
    /// println!("PDF size: {} bytes", pdf_bytes.len());
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn save_to_bytes(&self, flags: Option<SaveFlags>) -> Result<Vec<u8>> {
        let flags = flags.unwrap_or_default();
        self.save_internal(flags.to_raw(), None)
    }

    /// Save the document with a specific PDF version.
    ///
    /// # Arguments
    ///
    /// * `version` - PDF version number (e.g., 14 for PDF 1.4, 17 for PDF 1.7, 20 for PDF 2.0)
    /// * `flags` - Optional save flags
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    ///
    /// // Save as PDF 1.7
    /// let pdf_bytes = doc.save_with_version(17, None)?;
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn save_with_version(&self, version: i32, flags: Option<SaveFlags>) -> Result<Vec<u8>> {
        let flags = flags.unwrap_or_default();
        self.save_internal(flags.to_raw(), Some(version))
    }

    /// Internal save implementation using PDFium's FPDF_SaveAsCopy/FPDF_SaveWithVersion.
    fn save_internal(&self, flags: u64, version: Option<i32>) -> Result<Vec<u8>> {
        // Collect PDF bytes using a custom file writer
        // Use a boxed buffer to ensure stable address
        let buffer = Box::new(std::cell::RefCell::new(Vec::<u8>::new()));
        let buffer_ptr = buffer.as_ref() as *const std::cell::RefCell<Vec<u8>>;

        // Extended file writer struct that includes buffer pointer after FPDF_FILEWRITE_ fields
        // This allows us to access our data from the callback
        #[repr(C)]
        struct ExtendedFileWriter {
            // Must match FPDF_FILEWRITE_ layout
            version: i32,
            write_block: Option<
                extern "C" fn(
                    *mut FPDF_FILEWRITE_,
                    *const std::ffi::c_void,
                    std::os::raw::c_ulong,
                ) -> i32,
            >,
            // Extended field - our buffer pointer
            buffer_ptr: *const std::cell::RefCell<Vec<u8>>,
        }

        // C callback function
        extern "C" fn write_block(
            pthis: *mut FPDF_FILEWRITE_,
            data: *const std::ffi::c_void,
            size: std::os::raw::c_ulong,
        ) -> i32 {
            unsafe {
                // Cast to our extended struct to access buffer_ptr
                let writer = &*(pthis as *const ExtendedFileWriter);
                let slice = std::slice::from_raw_parts(data as *const u8, size as usize);
                (*writer.buffer_ptr).borrow_mut().extend_from_slice(slice);
                1 // Success
            }
        }

        let mut file_write = ExtendedFileWriter {
            version: 1,
            write_block: Some(write_block),
            buffer_ptr,
        };

        // Save the document
        let success = unsafe {
            match version {
                Some(v) => FPDF_SaveWithVersion(
                    self.inner.handle,
                    &mut file_write as *mut ExtendedFileWriter as *mut FPDF_FILEWRITE_,
                    flags,
                    v,
                ),
                None => FPDF_SaveAsCopy(
                    self.inner.handle,
                    &mut file_write as *mut ExtendedFileWriter as *mut FPDF_FILEWRITE_,
                    flags,
                ),
            }
        };

        if success == 0 {
            return Err(PdfError::SaveFailed {
                reason: "FPDF_SaveAsCopy returned false".to_string(),
            });
        }

        // Extract the buffer
        let result = buffer.into_inner();
        Ok(result)
    }

    // ========================================
    // Page Import/Delete API
    // ========================================

    /// Delete a page from the document.
    ///
    /// # Arguments
    ///
    /// * `page_index` - Zero-based page index to delete
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    ///
    /// // Delete the first page
    /// doc.delete_page(0);
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn delete_page(&self, page_index: usize) {
        unsafe {
            FPDFPage_Delete(self.inner.handle, page_index as i32);
        }
    }

    /// Import pages from another document.
    ///
    /// # Arguments
    ///
    /// * `source` - Source document to import from
    /// * `page_range` - Page range string (e.g., "1,3,5-7" or None for all pages)
    /// * `insert_index` - Where to insert the pages (0-based)
    ///
    /// # Returns
    ///
    /// * `true` if import succeeded
    /// * `false` if import failed
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("target.pdf", None)?;
    /// let source = pdfium.load_pdf_from_file("source.pdf", None)?;
    ///
    /// // Import pages 1-3 from source at the end of target
    /// let page_count = doc.page_count();
    /// doc.import_pages(&source, Some("1-3"), page_count)?;
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn import_pages(
        &self,
        source: &PdfDocument,
        page_range: Option<&str>,
        insert_index: usize,
    ) -> Result<bool> {
        // Need to keep the CString alive for the duration of the FFI call
        let c_range = page_range
            .map(|r| {
                std::ffi::CString::new(r).map_err(|_| PdfError::InvalidInput {
                    message: "Invalid page range string".to_string(),
                })
            })
            .transpose()?;

        let range_ptr = c_range
            .as_ref()
            .map(|c| c.as_ptr())
            .unwrap_or(std::ptr::null());

        let success = unsafe {
            FPDF_ImportPages(
                self.inner.handle,
                source.inner.handle,
                range_ptr,
                insert_index as i32,
            )
        };

        Ok(success != 0)
    }

    /// Copy viewer preferences from another document.
    ///
    /// Viewer preferences include settings like page layout, page mode,
    /// and various display flags.
    ///
    /// # Arguments
    ///
    /// * `source` - Source document to copy preferences from
    ///
    /// # Returns
    ///
    /// * `true` if copy succeeded
    /// * `false` if copy failed
    pub fn copy_viewer_preferences(&self, source: &PdfDocument) -> bool {
        unsafe { FPDF_CopyViewerPreferences(self.inner.handle, source.inner.handle) != 0 }
    }

    // ========================================
    // Document Splitting API
    // ========================================

    /// Extract specific pages from this document into a new document.
    ///
    /// This creates a new document containing only the specified pages.
    /// The page range uses the same syntax as `import_pages`:
    /// - Single page: `"1"` (1-indexed)
    /// - Range: `"1-5"`
    /// - Multiple ranges: `"1-3,5,7-10"`
    /// - Reverse order: `"10-1"`
    ///
    /// # Arguments
    ///
    /// * `page_range` - Page range specification (1-indexed)
    ///
    /// # Returns
    ///
    /// A new [`PdfDocument`] containing only the extracted pages.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("large.pdf", None)?;
    ///
    /// // Extract pages 1-10 into a new document
    /// let first_ten = doc.extract_pages("1-10")?;
    /// first_ten.save_to_file("pages_1_to_10.pdf", None)?;
    ///
    /// // Extract specific pages
    /// let selected = doc.extract_pages("1,3,5,7-9")?;
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn extract_pages(&self, page_range: &str) -> Result<PdfDocument> {
        let pdfium = Pdfium::new()?;
        let new_doc = pdfium.create_new_document()?;
        new_doc.import_pages(self, Some(page_range), 0)?;
        Ok(new_doc)
    }

    /// Extract a single page from this document into a new document.
    ///
    /// # Arguments
    ///
    /// * `page_index` - Index of the page to extract (0-based)
    ///
    /// # Returns
    ///
    /// A new [`PdfDocument`] containing only the extracted page.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    ///
    /// // Extract the first page (index 0)
    /// let first_page = doc.extract_page(0)?;
    /// first_page.save_to_file("first_page.pdf", None)?;
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn extract_page(&self, page_index: usize) -> Result<PdfDocument> {
        if page_index >= self.page_count() {
            return Err(PdfError::InvalidInput {
                message: format!(
                    "Page index {} out of range (document has {} pages)",
                    page_index,
                    self.page_count()
                ),
            });
        }
        // Page range is 1-indexed
        self.extract_pages(&format!("{}", page_index + 1))
    }

    /// Split this document at a specific page index.
    ///
    /// Returns two new documents:
    /// - First document: pages 0 to `split_at - 1`
    /// - Second document: pages `split_at` to end
    ///
    /// # Arguments
    ///
    /// * `split_at` - Page index at which to split (0-based). The page at this
    ///   index will be the first page of the second document.
    ///
    /// # Returns
    ///
    /// A tuple of two [`PdfDocument`]s: (first_part, second_part).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    ///
    /// // Split at page 5 (0-indexed)
    /// // First doc gets pages 0-4, second doc gets pages 5-end
    /// let (first_half, second_half) = doc.split_at(5)?;
    /// first_half.save_to_file("first_part.pdf", None)?;
    /// second_half.save_to_file("second_part.pdf", None)?;
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn split_at(&self, split_at: usize) -> Result<(PdfDocument, PdfDocument)> {
        let total_pages = self.page_count();

        if split_at == 0 {
            return Err(PdfError::InvalidInput {
                message: "Cannot split at index 0 (first part would be empty)".to_string(),
            });
        }

        if split_at >= total_pages {
            return Err(PdfError::InvalidInput {
                message: format!(
                    "Split index {} out of range (document has {} pages)",
                    split_at, total_pages
                ),
            });
        }

        // Page ranges are 1-indexed
        let first_range = format!("1-{}", split_at);
        let second_range = format!("{}-{}", split_at + 1, total_pages);

        let first_doc = self.extract_pages(&first_range)?;
        let second_doc = self.extract_pages(&second_range)?;

        Ok((first_doc, second_doc))
    }

    /// Split this document into chunks of a specified size.
    ///
    /// The last chunk may have fewer pages if the total page count
    /// is not evenly divisible by the chunk size.
    ///
    /// # Arguments
    ///
    /// * `chunk_size` - Maximum number of pages per chunk
    ///
    /// # Returns
    ///
    /// A vector of [`PdfDocument`]s, each containing up to `chunk_size` pages.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("large.pdf", None)?;
    ///
    /// // Split into 10-page chunks
    /// let chunks = doc.split_into_chunks(10)?;
    ///
    /// for (i, chunk) in chunks.iter().enumerate() {
    ///     chunk.save_to_file(&format!("chunk_{}.pdf", i + 1), None)?;
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn split_into_chunks(&self, chunk_size: usize) -> Result<Vec<PdfDocument>> {
        if chunk_size == 0 {
            return Err(PdfError::InvalidInput {
                message: "Chunk size must be greater than 0".to_string(),
            });
        }

        let total_pages = self.page_count();
        if total_pages == 0 {
            return Ok(vec![]);
        }

        let mut chunks = Vec::new();
        let mut start_page = 1; // 1-indexed for page ranges

        while start_page <= total_pages {
            let end_page = (start_page + chunk_size - 1).min(total_pages);
            let range = format!("{}-{}", start_page, end_page);
            chunks.push(self.extract_pages(&range)?);
            start_page = end_page + 1;
        }

        Ok(chunks)
    }

    /// Split this document at every N pages.
    ///
    /// Alias for `split_into_chunks`. Useful for splitting large documents
    /// into smaller, manageable files.
    ///
    /// # Arguments
    ///
    /// * `pages_per_split` - Number of pages per output document
    ///
    /// # Returns
    ///
    /// A vector of [`PdfDocument`]s.
    pub fn split_every(&self, pages_per_split: usize) -> Result<Vec<PdfDocument>> {
        self.split_into_chunks(pages_per_split)
    }

    /// Extract even-numbered pages (2, 4, 6, ...) into a new document.
    ///
    /// # Returns
    ///
    /// A new [`PdfDocument`] containing only even-numbered pages.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    ///
    /// let even_pages = doc.extract_even_pages()?;
    /// even_pages.save_to_file("even_pages.pdf", None)?;
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn extract_even_pages(&self) -> Result<PdfDocument> {
        let total_pages = self.page_count();
        if total_pages == 0 {
            return Err(PdfError::InvalidInput {
                message: "Document has no pages".to_string(),
            });
        }

        let even_pages: Vec<String> = (2..=total_pages)
            .step_by(2)
            .map(|p| p.to_string())
            .collect();

        if even_pages.is_empty() {
            return Err(PdfError::InvalidInput {
                message: "Document has only one page (no even pages)".to_string(),
            });
        }

        self.extract_pages(&even_pages.join(","))
    }

    /// Extract odd-numbered pages (1, 3, 5, ...) into a new document.
    ///
    /// # Returns
    ///
    /// A new [`PdfDocument`] containing only odd-numbered pages.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    ///
    /// let odd_pages = doc.extract_odd_pages()?;
    /// odd_pages.save_to_file("odd_pages.pdf", None)?;
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn extract_odd_pages(&self) -> Result<PdfDocument> {
        let total_pages = self.page_count();
        if total_pages == 0 {
            return Err(PdfError::InvalidInput {
                message: "Document has no pages".to_string(),
            });
        }

        let odd_pages: Vec<String> = (1..=total_pages)
            .step_by(2)
            .map(|p| p.to_string())
            .collect();

        self.extract_pages(&odd_pages.join(","))
    }

    /// Create a new document with pages in reverse order.
    ///
    /// Unlike [`reverse_pages`](Self::reverse_pages), this creates a new document
    /// rather than modifying the original in place.
    ///
    /// # Returns
    ///
    /// A new [`PdfDocument`] with reversed page order.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    ///
    /// let reversed = doc.to_reversed()?;
    /// reversed.save_to_file("reversed.pdf", None)?;
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn to_reversed(&self) -> Result<PdfDocument> {
        let total_pages = self.page_count();
        if total_pages == 0 {
            return Err(PdfError::InvalidInput {
                message: "Document has no pages".to_string(),
            });
        }

        // Build explicit reverse page list (PDFium doesn't support reverse ranges)
        let pages: Vec<String> = (1..=total_pages).rev().map(|p| p.to_string()).collect();
        self.extract_pages(&pages.join(","))
    }

    // ========================================
    // N-up Page Layout API
    // ========================================

    /// Create an N-up layout document from the current document.
    ///
    /// N-up layouts place multiple pages from the source document onto single output pages.
    /// For example, a 2-up layout places 2 source pages side by side on each output page,
    /// and a 4-up layout places 4 source pages in a 2x2 grid.
    ///
    /// This is useful for:
    /// - Creating booklets or pamphlets
    /// - Printing multiple slides per page
    /// - Reducing paper usage when printing
    /// - Creating thumbnail sheets
    ///
    /// # Arguments
    ///
    /// * `output_width` - Width of each output page in points (1 point = 1/72 inch)
    /// * `output_height` - Height of each output page in points
    /// * `num_pages_x` - Number of source pages to place horizontally
    /// * `num_pages_y` - Number of source pages to place vertically
    ///
    /// # Returns
    ///
    /// A new PdfDocument containing the N-up layout, or an error if creation failed.
    ///
    /// # Pages per output page
    ///
    /// `num_pages_x * num_pages_y` = pages per output page
    ///
    /// Common layouts:
    /// - 2-up: `num_pages_x=2, num_pages_y=1` (2 pages side by side)
    /// - 4-up: `num_pages_x=2, num_pages_y=2` (2x2 grid)
    /// - 6-up: `num_pages_x=3, num_pages_y=2` (3x2 grid)
    /// - 9-up: `num_pages_x=3, num_pages_y=3` (3x3 grid)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("slides.pdf", None)?;
    ///
    /// // Create 4-up layout (2x2) on US Letter landscape
    /// let nup_doc = doc.import_pages_n_up(792.0, 612.0, 2, 2)?;
    ///
    /// // Save the N-up document
    /// nup_doc.save_to_file("slides_4up.pdf", None)?;
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn import_pages_n_up(
        &self,
        output_width: f32,
        output_height: f32,
        num_pages_x: usize,
        num_pages_y: usize,
    ) -> Result<PdfDocument> {
        if num_pages_x == 0 || num_pages_y == 0 {
            return Err(PdfError::InvalidInput {
                message: "Number of pages per axis must be at least 1".to_string(),
            });
        }

        let result = unsafe {
            FPDF_ImportNPagesToOne(
                self.inner.handle,
                output_width,
                output_height,
                num_pages_x,
                num_pages_y,
            )
        };

        if result.is_null() {
            return Err(PdfError::InvalidInput {
                message: "Failed to create N-up layout document".to_string(),
            });
        }

        // Use from_raw to properly initialize form handle and other fields
        Ok(PdfDocument::from_raw(result))
    }

    /// Create a 2-up layout (2 pages side by side) on US Letter landscape.
    ///
    /// Convenience method for common 2-up printing layout.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    /// let nup = doc.import_pages_2up_letter()?;
    /// nup.save_to_file("document_2up.pdf", None)?;
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn import_pages_2up_letter(&self) -> Result<PdfDocument> {
        // US Letter landscape: 11 x 8.5 inches = 792 x 612 points
        self.import_pages_n_up(792.0, 612.0, 2, 1)
    }

    /// Create a 4-up layout (2x2 grid) on US Letter landscape.
    ///
    /// Convenience method for common 4-up printing layout.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("slides.pdf", None)?;
    /// let nup = doc.import_pages_4up_letter()?;
    /// nup.save_to_file("slides_4up.pdf", None)?;
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn import_pages_4up_letter(&self) -> Result<PdfDocument> {
        // US Letter landscape: 11 x 8.5 inches = 792 x 612 points
        self.import_pages_n_up(792.0, 612.0, 2, 2)
    }

    /// Create a 2-up layout (2 pages side by side) on A4 landscape.
    ///
    /// Convenience method for common 2-up printing layout (A4 paper).
    pub fn import_pages_2up_a4(&self) -> Result<PdfDocument> {
        // A4 landscape: 297 x 210 mm = 841.89 x 595.28 points
        self.import_pages_n_up(841.89, 595.28, 2, 1)
    }

    /// Create a 4-up layout (2x2 grid) on A4 landscape.
    ///
    /// Convenience method for common 4-up printing layout (A4 paper).
    pub fn import_pages_4up_a4(&self) -> Result<PdfDocument> {
        // A4 landscape: 297 x 210 mm = 841.89 x 595.28 points
        self.import_pages_n_up(841.89, 595.28, 2, 2)
    }

    // ========================================
    // XObject API
    // ========================================

    /// Create an XObject (form template) from a page in another document.
    ///
    /// XObjects are reusable page content templates. You can create an XObject
    /// from any page of a source document and then use it multiple times in
    /// this document.
    ///
    /// This is useful for:
    /// - Creating page templates (headers, footers, watermarks)
    /// - Stamping content onto multiple pages
    /// - Building N-up page layouts
    ///
    /// # Arguments
    ///
    /// * `source` - Source document containing the page to use as template
    /// * `src_page_index` - Index of the page in the source document (0-based)
    ///
    /// # Returns
    ///
    /// A [`PdfXObject`] handle that can be used to create form page objects.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    ///
    /// // Create a new document
    /// let doc = pdfium.create_new_document()?;
    ///
    /// // Load a template document
    /// let template = pdfium.load_pdf_from_file("template.pdf", None)?;
    ///
    /// // Create an XObject from the first page of the template
    /// let xobject = doc.create_xobject_from_page(&template, 0)?;
    ///
    /// // Create a new page and add the template to it
    /// let mut page = doc.new_page(0, 612.0, 792.0)?;
    /// let form_obj = xobject.to_page_object()?;
    /// page.insert_object(form_obj)?;
    ///
    /// doc.save_to_file("output.pdf", None)?;
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn create_xobject_from_page(
        &self,
        source: &PdfDocument,
        src_page_index: usize,
    ) -> Result<PdfXObject> {
        let handle = unsafe {
            pdfium_sys::FPDF_NewXObjectFromPage(
                self.inner.handle,
                source.inner.handle,
                src_page_index as i32,
            )
        };

        if handle.is_null() {
            return Err(PdfError::PageLoadFailed {
                index: src_page_index,
            });
        }

        Ok(PdfXObject { handle })
    }

    // ========================================
    // Page Creation API
    // ========================================

    /// Create a new blank page in the document.
    ///
    /// # Arguments
    ///
    /// * `page_index` - Where to insert the new page (0-based). If larger than
    ///   the current page count, the page is appended.
    /// * `width` - Page width in points (1 point = 1/72 inch)
    /// * `height` - Page height in points
    ///
    /// # Returns
    ///
    /// The newly created page, or an error if creation failed.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.create_new_document()?;
    ///
    /// // Create a US Letter size page (8.5 x 11 inches)
    /// let page = doc.new_page(0, 612.0, 792.0)?;
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn new_page(&self, page_index: usize, width: f64, height: f64) -> Result<PdfPage> {
        let page = unsafe { FPDFPage_New(self.inner.handle, page_index as i32, width, height) };

        if page.is_null() {
            return Err(PdfError::PageLoadFailed { index: page_index });
        }

        // Initialize form for this page
        if !self.inner.form_handle.is_null() {
            unsafe {
                FORM_OnAfterLoadPage(page, self.inner.form_handle);
                FORM_DoPageAAction(page, self.inner.form_handle, FPDFPAGE_AACTION_OPEN as i32);
            }
        }

        Ok(PdfPage::new(page, self.inner.clone(), page_index))
    }

    /// Create a new US Letter size page (8.5 x 11 inches).
    ///
    /// Convenience method for creating a standard US Letter page.
    pub fn new_page_letter(&self, page_index: usize) -> Result<PdfPage> {
        self.new_page(page_index, 612.0, 792.0) // 8.5 x 11 inches
    }

    /// Create a new A4 size page (210 x 297 mm).
    ///
    /// Convenience method for creating a standard A4 page.
    pub fn new_page_a4(&self, page_index: usize) -> Result<PdfPage> {
        self.new_page(page_index, 595.276, 841.890) // A4 in points
    }

    // ========================================
    // Page Reorder API
    // ========================================

    /// Move pages to a new position in the document.
    ///
    /// This is an experimental API that allows reordering pages within a document.
    ///
    /// # Arguments
    ///
    /// * `page_indices` - Indices of pages to move (no duplicates allowed)
    /// * `dest_index` - Destination index where pages will be moved
    ///
    /// # Returns
    ///
    /// * `Ok(true)` if pages were moved successfully
    /// * `Ok(false)` if the operation failed (invalid indices, duplicates, etc.)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    ///
    /// // Move pages 3 and 2 (in that order) to position 1
    /// // [A, B, C, D] with indices [0, 1, 2, 3]
    /// // Moving [3, 2] to position 1 results in: [A, D, C, B]
    /// doc.move_pages(&[3, 2], 1)?;
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn move_pages(&self, page_indices: &[i32], dest_index: usize) -> Result<bool> {
        if page_indices.is_empty() {
            return Ok(true);
        }

        let success = unsafe {
            FPDF_MovePages(
                self.inner.handle,
                page_indices.as_ptr(),
                page_indices.len() as std::os::raw::c_ulong,
                dest_index as i32,
            )
        };

        Ok(success != 0)
    }

    /// Reverse the order of all pages in the document.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    ///
    /// // Reverse all pages: [1, 2, 3, 4] -> [4, 3, 2, 1]
    /// doc.reverse_pages()?;
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn reverse_pages(&self) -> Result<bool> {
        let count = self.page_count();
        if count <= 1 {
            return Ok(true);
        }

        // Move pages in reverse order to position 0
        let indices: Vec<i32> = (0..count as i32).rev().collect();
        self.move_pages(&indices, 0)
    }

    // ========================================
    // File Identifier API
    // ========================================

    /// Get the file identifier from the document.
    ///
    /// PDF documents can have two file identifiers:
    /// - **Permanent**: Assigned when the document is first created, never changes
    /// - **Changing**: Updated whenever the document is modified
    ///
    /// # Arguments
    ///
    /// * `id_type` - Which identifier to retrieve
    ///
    /// # Returns
    ///
    /// The file identifier as a byte string, or None if not present.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::{Pdfium, FileIdentifierType};
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    ///
    /// if let Some(id) = doc.file_identifier(FileIdentifierType::Permanent) {
    ///     println!("Permanent ID: {:?}", id);
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn file_identifier(&self, id_type: FileIdentifierType) -> Option<Vec<u8>> {
        // Get required buffer size
        let size = unsafe {
            FPDF_GetFileIdentifier(self.inner.handle, id_type.to_raw(), std::ptr::null_mut(), 0)
        };

        if size == 0 {
            return None;
        }

        // Allocate buffer and get the identifier
        let mut buffer: Vec<u8> = vec![0; size as usize];
        let actual_size = unsafe {
            FPDF_GetFileIdentifier(
                self.inner.handle,
                id_type.to_raw(),
                buffer.as_mut_ptr() as *mut std::ffi::c_void,
                size,
            )
        };

        if actual_size == 0 {
            return None;
        }

        // Remove trailing null terminator if present
        if !buffer.is_empty() && buffer[buffer.len() - 1] == 0 {
            buffer.pop();
        }

        Some(buffer)
    }

    /// Get the permanent file identifier as a hex string.
    ///
    /// This is a convenience method that converts the permanent ID to a hex string.
    pub fn permanent_id(&self) -> Option<String> {
        self.file_identifier(FileIdentifierType::Permanent)
            .map(|bytes| bytes.iter().map(|b| format!("{:02X}", b)).collect())
    }

    /// Get the changing file identifier as a hex string.
    ///
    /// This is a convenience method that converts the changing ID to a hex string.
    pub fn changing_id(&self) -> Option<String> {
        self.file_identifier(FileIdentifierType::Changing)
            .map(|bytes| bytes.iter().map(|b| format!("{:02X}", b)).collect())
    }

    // ========================================
    // Font Loading API
    // ========================================

    /// Load a standard PDF font.
    ///
    /// Standard fonts (the "Base 14") are guaranteed to be available in all
    /// PDF viewers and don't need to be embedded in the PDF.
    ///
    /// # Arguments
    ///
    /// * `font` - The standard font to load
    ///
    /// # Returns
    ///
    /// A loaded font that can be used to create text objects.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::{Pdfium, StandardFont};
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.create_new_document()?;
    ///
    /// // Load Helvetica font
    /// let font = doc.load_standard_font(StandardFont::Helvetica)?;
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn load_standard_font(&self, font: StandardFont) -> Result<PdfLoadedFont> {
        let font_name =
            std::ffi::CString::new(font.name()).map_err(|_| PdfError::InvalidInput {
                message: "Invalid font name".to_string(),
            })?;

        let handle = unsafe { FPDFText_LoadStandardFont(self.inner.handle, font_name.as_ptr()) };

        if handle.is_null() {
            return Err(PdfError::InvalidInput {
                message: format!("Failed to load font: {}", font.name()),
            });
        }

        Ok(PdfLoadedFont { handle })
    }

    /// Load a font by name.
    ///
    /// This can load standard fonts by their PDF name (e.g., "Helvetica-Bold").
    ///
    /// # Arguments
    ///
    /// * `name` - The font name (e.g., "Helvetica", "Times-Roman", "Courier-Bold")
    ///
    /// # Returns
    ///
    /// A loaded font that can be used to create text objects.
    pub fn load_font_by_name(&self, name: &str) -> Result<PdfLoadedFont> {
        let font_name = std::ffi::CString::new(name).map_err(|_| PdfError::InvalidInput {
            message: "Invalid font name".to_string(),
        })?;

        let handle = unsafe { FPDFText_LoadStandardFont(self.inner.handle, font_name.as_ptr()) };

        if handle.is_null() {
            return Err(PdfError::InvalidInput {
                message: format!("Failed to load font: {}", name),
            });
        }

        Ok(PdfLoadedFont { handle })
    }

    // ========================================
    // Text Object Creation API
    // ========================================

    /// Create a text object with the specified font and size.
    ///
    /// The text object is not added to any page until you call `page.insert_object()`.
    /// Use `text_object.set_text()` to set the actual text content.
    ///
    /// # Arguments
    ///
    /// * `font` - A loaded font (from `load_standard_font` or `load_font_by_name`)
    /// * `font_size` - Font size in points
    ///
    /// # Returns
    ///
    /// A new text object that can be positioned and added to a page.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::{Pdfium, StandardFont};
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.create_new_document()?;
    ///
    /// // Load a font and create a text object
    /// let font = doc.load_standard_font(StandardFont::Helvetica)?;
    /// let mut text_obj = doc.create_text_object(&font, 12.0)?;
    ///
    /// // Set the text content
    /// text_obj.set_text("Hello, World!")?;
    ///
    /// // Position the text
    /// text_obj.transform(1.0, 0.0, 0.0, 1.0, 72.0, 720.0);
    ///
    /// // Create a page and add the text object
    /// let mut page = doc.new_page(0, 612.0, 792.0)?;
    /// page.insert_object(text_obj)?;
    /// page.generate_content()?;
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn create_text_object(
        &self,
        font: &PdfLoadedFont,
        font_size: f32,
    ) -> Result<PdfNewPageObject> {
        let handle =
            unsafe { FPDFPageObj_CreateTextObj(self.inner.handle, font.handle(), font_size) };

        if handle.is_null() {
            return Err(PdfError::InvalidInput {
                message: "Failed to create text object".to_string(),
            });
        }

        Ok(PdfNewPageObject { handle })
    }

    // ========================================
    // Path/Shape Creation API
    // ========================================

    /// Create a new path object starting at the specified point.
    ///
    /// A path object can contain lines, curves, and fills. After creation,
    /// use path methods to build the shape, then add it to a page.
    ///
    /// # Arguments
    ///
    /// * `x` - Starting X coordinate in points
    /// * `y` - Starting Y coordinate in points
    ///
    /// # Returns
    ///
    /// A new path object ready for drawing operations.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.create_new_document()?;
    ///
    /// // Create a triangle path
    /// let mut path = doc.create_path_object(100.0, 100.0)?;
    /// path.line_to(200.0, 100.0)?;
    /// path.line_to(150.0, 200.0)?;
    /// path.close()?;
    /// path.set_stroke_color(0, 0, 0, 255)?;
    /// path.set_draw_mode(true, true)?; // Fill and stroke
    ///
    /// let mut page = doc.new_page(0, 612.0, 792.0)?;
    /// page.insert_object(path)?;
    /// page.generate_content()?;
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn create_path_object(&self, x: f32, y: f32) -> Result<PdfNewPageObject> {
        let handle = unsafe { FPDFPageObj_CreateNewPath(x, y) };

        if handle.is_null() {
            return Err(PdfError::InvalidInput {
                message: "Failed to create path object".to_string(),
            });
        }

        Ok(PdfNewPageObject { handle })
    }

    /// Create a rectangle path object.
    ///
    /// This creates a complete rectangular path ready for filling or stroking.
    ///
    /// # Arguments
    ///
    /// * `x` - Left edge X coordinate in points
    /// * `y` - Bottom edge Y coordinate in points
    /// * `width` - Rectangle width in points
    /// * `height` - Rectangle height in points
    ///
    /// # Returns
    ///
    /// A new rectangle path object.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.create_new_document()?;
    ///
    /// // Create a blue filled rectangle
    /// let mut rect = doc.create_rect_object(100.0, 100.0, 200.0, 100.0)?;
    /// rect.set_fill_color(0, 0, 255, 255)?;
    /// rect.set_draw_mode(true, false)?; // Fill only
    ///
    /// let mut page = doc.new_page(0, 612.0, 792.0)?;
    /// page.insert_object(rect)?;
    /// page.generate_content()?;
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn create_rect_object(
        &self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) -> Result<PdfNewPageObject> {
        let handle = unsafe { FPDFPageObj_CreateNewRect(x, y, width, height) };

        if handle.is_null() {
            return Err(PdfError::InvalidInput {
                message: "Failed to create rectangle object".to_string(),
            });
        }

        Ok(PdfNewPageObject { handle })
    }

    // ========================================
    // Image Object Creation API
    // ========================================

    /// Create a new image object.
    ///
    /// The image object must be populated with image data using methods like
    /// `set_image_bitmap()` before adding to a page.
    ///
    /// # Returns
    ///
    /// A new image object ready to receive image data.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.create_new_document()?;
    ///
    /// // Create an image object
    /// let mut img = doc.create_image_object()?;
    ///
    /// // Set image data from a bitmap...
    /// // Position and scale the image using transform matrix
    /// img.set_image_matrix(100.0, 0.0, 0.0, 100.0, 72.0, 600.0)?;
    ///
    /// let mut page = doc.new_page(0, 612.0, 792.0)?;
    /// page.insert_object(img)?;
    /// page.generate_content()?;
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn create_image_object(&self) -> Result<PdfNewPageObject> {
        let handle = unsafe { FPDFPageObj_NewImageObj(self.inner.handle) };

        if handle.is_null() {
            return Err(PdfError::InvalidInput {
                message: "Failed to create image object".to_string(),
            });
        }

        Ok(PdfNewPageObject { handle })
    }

    // ========================================================================
    // Repeated Content Detection
    // ========================================================================

    /// Find content regions that appear repeatedly across multiple pages.
    ///
    /// This method detects headers, footers, watermarks, logos, and other
    /// content that appears in similar positions on multiple pages. It uses
    /// content hashing to identify matching regions.
    ///
    /// # Algorithm
    ///
    /// 1. Divide each page into a grid of regions (header, footer, margins)
    /// 2. Extract text content from each region
    /// 3. Hash the content for comparison
    /// 4. Group regions with matching hashes across pages
    /// 5. Return regions that appear on at least 2 pages
    ///
    /// # Arguments
    ///
    /// * `tolerance` - Position tolerance in points for matching regions.
    ///   Larger values allow more variation in position.
    ///
    /// # Returns
    ///
    /// A vector of `RepeatedRegion` structs describing content that appears
    /// on multiple pages.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    ///
    /// // Find repeated content with 5pt tolerance
    /// let repeated = doc.find_repeated_regions(5.0);
    ///
    /// for region in &repeated {
    ///     println!("Found repeated content on {} pages", region.occurrence_count);
    ///     if region.is_header(792.0) {
    ///         println!("  - This is likely a header");
    ///     }
    ///     if region.is_footer() {
    ///         println!("  - This is likely a footer");
    ///     }
    /// }
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn find_repeated_regions(&self, tolerance: f32) -> Vec<RepeatedRegion> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let page_count = self.page_count();
        if page_count < 2 {
            return vec![];
        }

        // Structure to track content in regions
        #[derive(Clone)]
        struct RegionData {
            bounds: (f32, f32, f32, f32),
            content_hash: u64,
            page_index: usize,
        }

        let mut all_regions: Vec<RegionData> = Vec::new();

        // Process each page
        for page_idx in 0..page_count {
            let page = match self.page(page_idx) {
                Ok(p) => p,
                Err(_) => continue,
            };

            let (width, height) = page.size();
            let width = width as f32;
            let height = height as f32;

            // Define standard regions to check for repeated content
            // Header region: top 12% of page
            // Footer region: bottom 12% of page
            // Left margin: left 8% of page
            // Right margin: right 8% of page
            let regions: [(f32, f32, f32, f32); 4] = [
                (0.0, height * 0.88, width, height),                 // Header
                (0.0, 0.0, width, height * 0.12),                    // Footer
                (0.0, height * 0.12, width * 0.08, height * 0.88),   // Left margin
                (width * 0.92, height * 0.12, width, height * 0.88), // Right margin
            ];

            // Get text from page
            let text = match page.text() {
                Ok(t) => t,
                Err(_) => continue,
            };

            // Extract content from each region
            for region_bounds in &regions {
                let mut region_text = String::new();

                // Collect text within the region bounds
                for ch in text.chars() {
                    let cx = ((ch.left + ch.right) / 2.0) as f32;
                    let cy = ((ch.bottom + ch.top) / 2.0) as f32;

                    // Check if character center is within region
                    if cx >= region_bounds.0 - tolerance
                        && cx <= region_bounds.2 + tolerance
                        && cy >= region_bounds.1 - tolerance
                        && cy <= region_bounds.3 + tolerance
                    {
                        region_text.push(ch.unicode);
                    }
                }

                // Skip empty regions
                let trimmed = region_text.trim();
                if trimmed.is_empty() {
                    continue;
                }

                // Hash the content
                let mut hasher = DefaultHasher::new();
                trimmed.hash(&mut hasher);
                let content_hash = hasher.finish();

                all_regions.push(RegionData {
                    bounds: *region_bounds,
                    content_hash,
                    page_index: page_idx,
                });
            }
        }

        // Group regions by content hash and similar bounds
        use std::collections::HashMap;
        let mut hash_groups: HashMap<u64, Vec<RegionData>> = HashMap::new();

        for region in all_regions {
            hash_groups
                .entry(region.content_hash)
                .or_default()
                .push(region);
        }

        // Convert groups to RepeatedRegion results
        let mut results: Vec<RepeatedRegion> = Vec::new();

        for (content_hash, regions) in hash_groups {
            // Need at least 2 occurrences to be "repeated"
            if regions.len() < 2 {
                continue;
            }

            // Use the first region's bounds as representative
            let representative_bounds = regions[0].bounds;

            // Collect unique page indices
            let mut page_indices: Vec<usize> = regions.iter().map(|r| r.page_index).collect();
            page_indices.sort_unstable();
            page_indices.dedup();

            // Must appear on at least 2 distinct pages
            if page_indices.len() < 2 {
                continue;
            }

            results.push(RepeatedRegion::new(
                representative_bounds,
                page_indices,
                content_hash,
            ));
        }

        // Sort by occurrence count (most repeated first)
        results.sort_by(|a, b| b.occurrence_count.cmp(&a.occurrence_count));

        results
    }

    /// Find repeated regions with custom region definitions.
    ///
    /// This is an advanced version of `find_repeated_regions` that allows
    /// specifying custom regions to check for repeated content.
    ///
    /// # Arguments
    ///
    /// * `regions` - List of region bounds (left, bottom, right, top) as fractions
    ///   of page size (0.0 to 1.0). Each region is checked for repeated content.
    /// * `tolerance` - Position tolerance in points
    /// * `min_occurrences` - Minimum number of pages a region must appear on
    ///
    /// # Returns
    ///
    /// A vector of `RepeatedRegion` structs.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
    ///
    /// // Check only header and footer regions
    /// let regions = vec![
    ///     (0.0, 0.90, 1.0, 1.0),  // Top 10% (header)
    ///     (0.0, 0.0, 1.0, 0.10),  // Bottom 10% (footer)
    /// ];
    ///
    /// let repeated = doc.find_repeated_regions_custom(&regions, 5.0, 3);
    /// println!("Found {} repeated regions", repeated.len());
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn find_repeated_regions_custom(
        &self,
        region_fractions: &[(f32, f32, f32, f32)],
        tolerance: f32,
        min_occurrences: usize,
    ) -> Vec<RepeatedRegion> {
        use std::collections::hash_map::DefaultHasher;
        use std::collections::HashMap;
        use std::hash::{Hash, Hasher};

        let page_count = self.page_count();
        if page_count < min_occurrences {
            return vec![];
        }

        #[derive(Clone)]
        struct RegionData {
            bounds: (f32, f32, f32, f32),
            content_hash: u64,
            page_index: usize,
        }

        let mut all_regions: Vec<RegionData> = Vec::new();

        for page_idx in 0..page_count {
            let page = match self.page(page_idx) {
                Ok(p) => p,
                Err(_) => continue,
            };

            let (width, height) = page.size();
            let width = width as f32;
            let height = height as f32;
            let text = match page.text() {
                Ok(t) => t,
                Err(_) => continue,
            };

            // Convert fraction-based regions to actual coordinates
            for frac in region_fractions {
                let region_bounds = (
                    frac.0 * width,
                    frac.1 * height,
                    frac.2 * width,
                    frac.3 * height,
                );

                let mut region_text = String::new();

                for ch in text.chars() {
                    let cx = ((ch.left + ch.right) / 2.0) as f32;
                    let cy = ((ch.bottom + ch.top) / 2.0) as f32;

                    if cx >= region_bounds.0 - tolerance
                        && cx <= region_bounds.2 + tolerance
                        && cy >= region_bounds.1 - tolerance
                        && cy <= region_bounds.3 + tolerance
                    {
                        region_text.push(ch.unicode);
                    }
                }

                let trimmed = region_text.trim();
                if trimmed.is_empty() {
                    continue;
                }

                let mut hasher = DefaultHasher::new();
                trimmed.hash(&mut hasher);
                let content_hash = hasher.finish();

                all_regions.push(RegionData {
                    bounds: region_bounds,
                    content_hash,
                    page_index: page_idx,
                });
            }
        }

        let mut hash_groups: HashMap<u64, Vec<RegionData>> = HashMap::new();
        for region in all_regions {
            hash_groups
                .entry(region.content_hash)
                .or_default()
                .push(region);
        }

        let mut results: Vec<RepeatedRegion> = Vec::new();

        for (content_hash, regions) in hash_groups {
            if regions.len() < min_occurrences {
                continue;
            }

            let representative_bounds = regions[0].bounds;
            let mut page_indices: Vec<usize> = regions.iter().map(|r| r.page_index).collect();
            page_indices.sort_unstable();
            page_indices.dedup();

            if page_indices.len() < min_occurrences {
                continue;
            }

            results.push(RepeatedRegion::new(
                representative_bounds,
                page_indices,
                content_hash,
            ));
        }

        results.sort_by(|a, b| b.occurrence_count.cmp(&a.occurrence_count));
        results
    }

    /// Count the number of repeated content regions in the document.
    ///
    /// This is a convenience method that returns just the count without
    /// computing full region details.
    ///
    /// # Arguments
    ///
    /// * `tolerance` - Position tolerance in points
    ///
    /// # Returns
    ///
    /// The number of distinct repeated content regions found.
    pub fn repeated_region_count(&self, tolerance: f32) -> usize {
        self.find_repeated_regions(tolerance).len()
    }

    /// Check if the document has any repeated content regions.
    ///
    /// A quick check that returns true if headers, footers, or other
    /// repeated content is detected.
    ///
    /// # Arguments
    ///
    /// * `tolerance` - Position tolerance in points
    pub fn has_repeated_content(&self, tolerance: f32) -> bool {
        self.repeated_region_count(tolerance) > 0
    }
}

impl Drop for PdfDocumentInner {
    fn drop(&mut self) {
        unsafe {
            if !self.form_handle.is_null() {
                FPDFDOC_ExitFormFillEnvironment(self.form_handle);
            }
            FPDF_CloseDocument(self.handle);
        }
    }
}

/// Iterator over pages in a document.
pub struct PdfPages<'a> {
    doc: &'a PdfDocument,
    index: usize,
    count: usize,
}

impl<'a> Iterator for PdfPages<'a> {
    type Item = PdfPage;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.count {
            return None;
        }
        let page = self.doc.page(self.index).ok()?;
        self.index += 1;
        Some(page)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.count - self.index;
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for PdfPages<'a> {}

/// Iterator over named destinations in a document.
pub struct PdfNamedDestsIter<'a> {
    doc: &'a PdfDocument,
    index: usize,
    count: usize,
}

impl<'a> Iterator for PdfNamedDestsIter<'a> {
    type Item = (String, crate::destination::PdfDestination);

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.count {
            return None;
        }
        let result = self.doc.named_dest(self.index)?;
        self.index += 1;
        Some(result)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.count - self.index;
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for PdfNamedDestsIter<'a> {}

/// Standard PDF fonts (the "Base 14" fonts).
///
/// These fonts are guaranteed to be available in all PDF viewers.
/// They don't need to be embedded in the PDF.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StandardFont {
    /// Courier (fixed-width)
    Courier,
    /// Courier Bold
    CourierBold,
    /// Courier Oblique (italic)
    CourierOblique,
    /// Courier Bold Oblique
    CourierBoldOblique,
    /// Helvetica (sans-serif)
    Helvetica,
    /// Helvetica Bold
    HelveticaBold,
    /// Helvetica Oblique (italic)
    HelveticaOblique,
    /// Helvetica Bold Oblique
    HelveticaBoldOblique,
    /// Times Roman (serif)
    TimesRoman,
    /// Times Bold
    TimesBold,
    /// Times Italic
    TimesItalic,
    /// Times Bold Italic
    TimesBoldItalic,
    /// Symbol (mathematical symbols)
    Symbol,
    /// Zapf Dingbats (decorative symbols)
    ZapfDingbats,
}

impl StandardFont {
    /// Get the font name as a string.
    pub fn name(&self) -> &'static str {
        match self {
            StandardFont::Courier => "Courier",
            StandardFont::CourierBold => "Courier-Bold",
            StandardFont::CourierOblique => "Courier-Oblique",
            StandardFont::CourierBoldOblique => "Courier-BoldOblique",
            StandardFont::Helvetica => "Helvetica",
            StandardFont::HelveticaBold => "Helvetica-Bold",
            StandardFont::HelveticaOblique => "Helvetica-Oblique",
            StandardFont::HelveticaBoldOblique => "Helvetica-BoldOblique",
            StandardFont::TimesRoman => "Times-Roman",
            StandardFont::TimesBold => "Times-Bold",
            StandardFont::TimesItalic => "Times-Italic",
            StandardFont::TimesBoldItalic => "Times-BoldItalic",
            StandardFont::Symbol => "Symbol",
            StandardFont::ZapfDingbats => "ZapfDingbats",
        }
    }

    /// Check if this is a fixed-width font.
    pub fn is_fixed_width(&self) -> bool {
        matches!(
            self,
            StandardFont::Courier
                | StandardFont::CourierBold
                | StandardFont::CourierOblique
                | StandardFont::CourierBoldOblique
        )
    }

    /// Check if this is a serif font.
    pub fn is_serif(&self) -> bool {
        matches!(
            self,
            StandardFont::TimesRoman
                | StandardFont::TimesBold
                | StandardFont::TimesItalic
                | StandardFont::TimesBoldItalic
        )
    }

    /// Check if this is a sans-serif font.
    pub fn is_sans_serif(&self) -> bool {
        matches!(
            self,
            StandardFont::Helvetica
                | StandardFont::HelveticaBold
                | StandardFont::HelveticaOblique
                | StandardFont::HelveticaBoldOblique
        )
    }
}

/// A loaded PDF font that can be used to create text objects.
///
/// This wraps an FPDF_FONT handle and ensures proper cleanup.
pub struct PdfLoadedFont {
    handle: FPDF_FONT,
}

impl PdfLoadedFont {
    /// Get the raw font handle.
    pub fn handle(&self) -> FPDF_FONT {
        self.handle
    }
}

impl Drop for PdfLoadedFont {
    fn drop(&mut self) {
        unsafe {
            FPDFFont_Close(self.handle);
        }
    }
}

/// A newly created page object that can be added to a page.
///
/// This struct owns the page object handle until it's inserted into a page.
/// After insertion, the page takes ownership and this struct should not be used.
pub struct PdfNewPageObject {
    handle: FPDF_PAGEOBJECT,
}

impl PdfNewPageObject {
    /// Get the raw page object handle.
    pub fn handle(&self) -> FPDF_PAGEOBJECT {
        self.handle
    }

    /// Consume this object and return the raw handle.
    ///
    /// Used internally when inserting into a page.
    pub(crate) fn into_handle(self) -> FPDF_PAGEOBJECT {
        let handle = self.handle;
        std::mem::forget(self); // Don't run Drop
        handle
    }

    // ========================================
    // Text Object Methods
    // ========================================

    /// Set the text content of a text object.
    ///
    /// # Arguments
    ///
    /// * `text` - The text to display
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or an error if setting text fails.
    pub fn set_text(&mut self, text: &str) -> crate::error::Result<()> {
        // Convert to UTF-16 with null terminator
        let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();

        let success = unsafe { FPDFText_SetText(self.handle, wide.as_ptr()) };

        if success == 0 {
            return Err(crate::error::PdfError::InvalidInput {
                message: "Failed to set text content".to_string(),
            });
        }

        Ok(())
    }

    // ========================================
    // Path Object Methods
    // ========================================

    /// Move to a new point without drawing (starts a new subpath).
    ///
    /// # Arguments
    ///
    /// * `x` - X coordinate in points
    /// * `y` - Y coordinate in points
    pub fn move_to(&mut self, x: f32, y: f32) -> crate::error::Result<()> {
        let success = unsafe { FPDFPath_MoveTo(self.handle, x, y) };
        if success == 0 {
            return Err(crate::error::PdfError::InvalidInput {
                message: "Failed to move path".to_string(),
            });
        }
        Ok(())
    }

    /// Draw a straight line from the current point to a new point.
    ///
    /// # Arguments
    ///
    /// * `x` - End point X coordinate
    /// * `y` - End point Y coordinate
    pub fn line_to(&mut self, x: f32, y: f32) -> crate::error::Result<()> {
        let success = unsafe { FPDFPath_LineTo(self.handle, x, y) };
        if success == 0 {
            return Err(crate::error::PdfError::InvalidInput {
                message: "Failed to add line to path".to_string(),
            });
        }
        Ok(())
    }

    /// Draw a cubic Bezier curve from the current point.
    ///
    /// # Arguments
    ///
    /// * `x1`, `y1` - First control point
    /// * `x2`, `y2` - Second control point
    /// * `x3`, `y3` - End point
    pub fn bezier_to(
        &mut self,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        x3: f32,
        y3: f32,
    ) -> crate::error::Result<()> {
        let success = unsafe { FPDFPath_BezierTo(self.handle, x1, y1, x2, y2, x3, y3) };
        if success == 0 {
            return Err(crate::error::PdfError::InvalidInput {
                message: "Failed to add bezier to path".to_string(),
            });
        }
        Ok(())
    }

    /// Close the current subpath (draw line back to start).
    pub fn close(&mut self) -> crate::error::Result<()> {
        let success = unsafe { FPDFPath_Close(self.handle) };
        if success == 0 {
            return Err(crate::error::PdfError::InvalidInput {
                message: "Failed to close path".to_string(),
            });
        }
        Ok(())
    }

    /// Set the draw mode for a path object.
    ///
    /// # Arguments
    ///
    /// * `fill` - Whether to fill the path
    /// * `stroke` - Whether to stroke the path outline
    pub fn set_draw_mode(&mut self, fill: bool, stroke: bool) -> crate::error::Result<()> {
        // Fill mode: 0 = no fill, 1 = alternate (even-odd), 2 = winding
        let fill_mode = if fill { 1 } else { 0 };
        let success = unsafe { FPDFPath_SetDrawMode(self.handle, fill_mode, stroke as i32) };
        if success == 0 {
            return Err(crate::error::PdfError::InvalidInput {
                message: "Failed to set path draw mode".to_string(),
            });
        }
        Ok(())
    }

    // ========================================
    // Common Object Methods
    // ========================================

    /// Set the fill color of the object.
    ///
    /// # Arguments
    ///
    /// * `r` - Red component (0-255)
    /// * `g` - Green component (0-255)
    /// * `b` - Blue component (0-255)
    /// * `a` - Alpha component (0-255, 255 = opaque)
    pub fn set_fill_color(&mut self, r: u8, g: u8, b: u8, a: u8) -> crate::error::Result<()> {
        let success = unsafe {
            FPDFPageObj_SetFillColor(self.handle, r as u32, g as u32, b as u32, a as u32)
        };
        if success == 0 {
            return Err(crate::error::PdfError::InvalidInput {
                message: "Failed to set fill color".to_string(),
            });
        }
        Ok(())
    }

    /// Set the stroke (outline) color of the object.
    ///
    /// # Arguments
    ///
    /// * `r` - Red component (0-255)
    /// * `g` - Green component (0-255)
    /// * `b` - Blue component (0-255)
    /// * `a` - Alpha component (0-255, 255 = opaque)
    pub fn set_stroke_color(&mut self, r: u8, g: u8, b: u8, a: u8) -> crate::error::Result<()> {
        let success = unsafe {
            FPDFPageObj_SetStrokeColor(self.handle, r as u32, g as u32, b as u32, a as u32)
        };
        if success == 0 {
            return Err(crate::error::PdfError::InvalidInput {
                message: "Failed to set stroke color".to_string(),
            });
        }
        Ok(())
    }

    /// Set the stroke width of the object.
    ///
    /// # Arguments
    ///
    /// * `width` - Stroke width in points
    pub fn set_stroke_width(&mut self, width: f32) -> crate::error::Result<()> {
        let success = unsafe { FPDFPageObj_SetStrokeWidth(self.handle, width) };
        if success == 0 {
            return Err(crate::error::PdfError::InvalidInput {
                message: "Failed to set stroke width".to_string(),
            });
        }
        Ok(())
    }

    /// Apply a transformation matrix to the object.
    ///
    /// The transformation matrix is:
    /// ```text
    /// | a  b  0 |
    /// | c  d  0 |
    /// | e  f  1 |
    /// ```
    ///
    /// Common transformations:
    /// - Translation: `transform(1, 0, 0, 1, tx, ty)`
    /// - Scale: `transform(sx, 0, 0, sy, 0, 0)`
    /// - Rotation: `transform(cos, sin, -sin, cos, 0, 0)`
    pub fn transform(&mut self, a: f32, b: f32, c: f32, d: f32, e: f32, f: f32) {
        unsafe {
            FPDFPageObj_Transform(
                self.handle,
                a as f64,
                b as f64,
                c as f64,
                d as f64,
                e as f64,
                f as f64,
            );
        }
    }

    /// Set the transformation matrix of the object.
    ///
    /// Unlike `transform()` which multiplies with the existing matrix,
    /// this replaces the entire transformation matrix.
    pub fn set_matrix(
        &mut self,
        a: f32,
        b: f32,
        c: f32,
        d: f32,
        e: f32,
        f: f32,
    ) -> crate::error::Result<()> {
        let matrix = FS_MATRIX { a, b, c, d, e, f };
        let success = unsafe { FPDFPageObj_SetMatrix(self.handle, &matrix) };
        if success == 0 {
            return Err(crate::error::PdfError::InvalidInput {
                message: "Failed to set transformation matrix".to_string(),
            });
        }
        Ok(())
    }

    // ========================================
    // Image Object Methods
    // ========================================

    /// Set the transformation matrix of an image object.
    ///
    /// This is specifically for image objects. The matrix controls position and scale:
    ///
    /// ```text
    /// | a  b |  scale/rotate  | e |  translate x
    /// | c  d |                | f |  translate y
    /// ```
    ///
    /// For a simple positioned and scaled image:
    /// - `a` = display width in points
    /// - `d` = display height in points
    /// - `e` = x position in points
    /// - `f` = y position in points
    /// - `b`, `c` = 0 for no rotation/skew
    ///
    /// # Arguments
    ///
    /// * `a`, `b`, `c`, `d` - Transformation components (scale/rotate)
    /// * `e`, `f` - Translation (position in points)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.create_new_document()?;
    ///
    /// let mut img = doc.create_image_object()?;
    /// // Display as 200x150 points at position (72, 600)
    /// img.set_image_matrix(200.0, 0.0, 0.0, 150.0, 72.0, 600.0)?;
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn set_image_matrix(
        &mut self,
        a: f64,
        b: f64,
        c: f64,
        d: f64,
        e: f64,
        f: f64,
    ) -> crate::error::Result<()> {
        let success = unsafe { FPDFImageObj_SetMatrix(self.handle, a, b, c, d, e, f) };
        if success == 0 {
            return Err(crate::error::PdfError::InvalidInput {
                message: "Failed to set image transformation matrix".to_string(),
            });
        }
        Ok(())
    }

    /// Set the bitmap data for an image object.
    ///
    /// This takes ownership of the bitmap pixels and embeds them in the PDF.
    ///
    /// # Arguments
    ///
    /// * `bitmap` - A bitmap handle from PDFium (FPDF_BITMAP)
    ///
    /// # Safety
    ///
    /// The bitmap must be a valid PDFium bitmap handle.
    ///
    /// # Returns
    ///
    /// Ok(()) on success, or an error if the bitmap could not be set.
    pub unsafe fn set_image_bitmap(&mut self, bitmap: FPDF_BITMAP) -> crate::error::Result<()> {
        let success = FPDFImageObj_SetBitmap(std::ptr::null_mut(), 0, self.handle, bitmap);
        if success == 0 {
            return Err(crate::error::PdfError::InvalidInput {
                message: "Failed to set image bitmap".to_string(),
            });
        }
        Ok(())
    }

    // ========================================
    // Line Style Methods
    // ========================================

    /// Set the line cap style for path objects.
    ///
    /// The line cap determines how the ends of lines are drawn.
    ///
    /// # Arguments
    ///
    /// * `cap` - The line cap style
    pub fn set_line_cap(&mut self, cap: LineCap) -> crate::error::Result<()> {
        let success = unsafe { FPDFPageObj_SetLineCap(self.handle, cap.to_raw()) };
        if success == 0 {
            return Err(crate::error::PdfError::InvalidInput {
                message: "Failed to set line cap".to_string(),
            });
        }
        Ok(())
    }

    /// Get the line cap style of the object.
    pub fn get_line_cap(&self) -> Option<LineCap> {
        let cap = unsafe { FPDFPageObj_GetLineCap(self.handle) };
        if cap < 0 {
            None
        } else {
            Some(LineCap::from_raw(cap))
        }
    }

    /// Set the line join style for path objects.
    ///
    /// The line join determines how corners are drawn where lines meet.
    ///
    /// # Arguments
    ///
    /// * `join` - The line join style
    pub fn set_line_join(&mut self, join: LineJoin) -> crate::error::Result<()> {
        let success = unsafe { FPDFPageObj_SetLineJoin(self.handle, join.to_raw()) };
        if success == 0 {
            return Err(crate::error::PdfError::InvalidInput {
                message: "Failed to set line join".to_string(),
            });
        }
        Ok(())
    }

    /// Get the line join style of the object.
    pub fn get_line_join(&self) -> Option<LineJoin> {
        let join = unsafe { FPDFPageObj_GetLineJoin(self.handle) };
        if join < 0 {
            None
        } else {
            Some(LineJoin::from_raw(join))
        }
    }

    /// Set a dash pattern for stroking.
    ///
    /// A dash pattern alternates between drawn and undrawn segments.
    ///
    /// # Arguments
    ///
    /// * `pattern` - Array of dash/gap lengths in points
    /// * `phase` - Starting offset into the pattern
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.create_new_document()?;
    ///
    /// let mut path = doc.create_path_object(100.0, 100.0)?;
    /// path.line_to(300.0, 100.0)?;
    /// path.set_stroke_color(0, 0, 0, 255)?;
    /// path.set_draw_mode(false, true)?;
    ///
    /// // Dashed line: 10 points on, 5 points off
    /// path.set_dash_pattern(&[10.0, 5.0], 0.0)?;
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn set_dash_pattern(&mut self, pattern: &[f32], phase: f32) -> crate::error::Result<()> {
        let success = unsafe {
            FPDFPageObj_SetDashArray(self.handle, pattern.as_ptr(), pattern.len(), phase)
        };
        if success == 0 {
            return Err(crate::error::PdfError::InvalidInput {
                message: "Failed to set dash pattern".to_string(),
            });
        }
        Ok(())
    }

    // ========================================
    // Blend Mode and Transparency Methods
    // ========================================

    /// Set the blend mode for the object.
    ///
    /// The blend mode determines how the object's colors combine with
    /// underlying content when the object has transparency.
    ///
    /// # Arguments
    ///
    /// * `mode` - The blend mode to use
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::{Pdfium, BlendMode};
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.create_new_document()?;
    ///
    /// let mut rect = doc.create_rect_object(100.0, 100.0, 100.0, 100.0)?;
    /// rect.set_fill_color(255, 0, 0, 128)?;  // Semi-transparent red
    /// rect.set_blend_mode(BlendMode::Multiply);
    /// rect.set_draw_mode(true, false)?;
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn set_blend_mode(&mut self, mode: BlendMode) {
        let mode_str = std::ffi::CString::new(mode.as_str()).unwrap();
        unsafe {
            FPDFPageObj_SetBlendMode(self.handle, mode_str.as_ptr());
        }
    }

    /// Check if this object has any transparency.
    ///
    /// Returns true if the object uses any transparency features
    /// (alpha < 255, blend modes other than Normal, etc.)
    pub fn has_transparency(&self) -> bool {
        unsafe { FPDFPageObj_HasTransparency(self.handle) != 0 }
    }

    /// Get the type of this page object.
    pub fn object_type(&self) -> PageObjectType {
        let obj_type = unsafe { FPDFPageObj_GetType(self.handle) };
        PageObjectType::from_raw(obj_type)
    }

    /// Get the clip path of this page object.
    ///
    /// Returns the clip path if one exists, or None if the object has no clip path.
    pub fn get_clip_path(&self) -> Option<PdfClipPath> {
        let handle = unsafe { FPDFPageObj_GetClipPath(self.handle) };
        if handle.is_null() {
            None
        } else {
            Some(PdfClipPath::from_handle(handle))
        }
    }

    /// Transform the clip path of this page object.
    ///
    /// Applies a transformation matrix to the object's clip path.
    ///
    /// # Arguments
    ///
    /// * `a`, `b`, `c`, `d`, `e`, `f` - The transformation matrix components
    pub fn transform_clip_path(&mut self, a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) {
        unsafe {
            FPDFPageObj_TransformClipPath(self.handle, a, b, c, d, e, f);
        }
    }

    /// Get the ICC profile data from an image object.
    ///
    /// Only applicable to image objects. Returns the embedded ICC profile
    /// if one exists.
    ///
    /// # Arguments
    ///
    /// * `page` - The page this object belongs to (needed for context)
    ///
    /// # Returns
    ///
    /// The ICC profile data and color space, or None if not available.
    pub fn get_icc_profile(&self, page: &crate::page::PdfPage) -> Option<IccProfile> {
        if self.object_type() != PageObjectType::Image {
            return None;
        }

        // First call to get buffer size
        let mut buf_len: usize = 0;
        unsafe {
            FPDFImageObj_GetIccProfileDataDecoded(
                self.handle,
                page.handle(),
                std::ptr::null_mut(),
                0,
                &mut buf_len,
            );
        }

        if buf_len == 0 {
            return None;
        }

        // Allocate buffer and get data
        let mut buffer = vec![0u8; buf_len];
        let mut actual_len: usize = 0;
        let result = unsafe {
            FPDFImageObj_GetIccProfileDataDecoded(
                self.handle,
                page.handle(),
                buffer.as_mut_ptr(),
                buf_len,
                &mut actual_len,
            )
        };

        if result == 0 || actual_len == 0 {
            return None;
        }

        buffer.truncate(actual_len);

        // Get color space from the returned value
        let color_space = IccColorSpace::from_raw(result as u32);

        Some(IccProfile {
            data: buffer,
            color_space,
        })
    }

    /// Get the raw pixel data from an image object.
    ///
    /// Only applicable to image objects. Returns the decoded image data.
    ///
    /// # Returns
    ///
    /// The decoded image data, or None if not available.
    pub fn get_image_data_decoded(&self) -> Option<Vec<u8>> {
        if self.object_type() != PageObjectType::Image {
            return None;
        }

        // First call to get buffer size
        let buf_len =
            unsafe { FPDFImageObj_GetImageDataDecoded(self.handle, std::ptr::null_mut(), 0) };

        if buf_len == 0 {
            return None;
        }

        // Allocate buffer and get data
        let mut buffer = vec![0u8; buf_len as usize];
        let actual_len = unsafe {
            FPDFImageObj_GetImageDataDecoded(self.handle, buffer.as_mut_ptr() as *mut _, buf_len)
        };

        if actual_len == 0 {
            return None;
        }

        buffer.truncate(actual_len as usize);
        Some(buffer)
    }

    /// Get the raw (undecoded) image data from an image object.
    ///
    /// Only applicable to image objects. Returns the raw stream data
    /// before any filters are applied.
    ///
    /// # Returns
    ///
    /// The raw image data, or None if not available.
    pub fn get_image_data_raw(&self) -> Option<Vec<u8>> {
        if self.object_type() != PageObjectType::Image {
            return None;
        }

        // First call to get buffer size
        let buf_len = unsafe { FPDFImageObj_GetImageDataRaw(self.handle, std::ptr::null_mut(), 0) };

        if buf_len == 0 {
            return None;
        }

        // Allocate buffer and get data
        let mut buffer = vec![0u8; buf_len as usize];
        let actual_len = unsafe {
            FPDFImageObj_GetImageDataRaw(self.handle, buffer.as_mut_ptr() as *mut _, buf_len)
        };

        if actual_len == 0 {
            return None;
        }

        buffer.truncate(actual_len as usize);
        Some(buffer)
    }

    /// Get the pixel dimensions of an image object.
    ///
    /// Only applicable to image objects.
    ///
    /// # Returns
    ///
    /// Tuple of (width, height) in pixels, or None if not available.
    pub fn get_image_pixel_size(&self) -> Option<(u32, u32)> {
        if self.object_type() != PageObjectType::Image {
            return None;
        }

        let mut width: u32 = 0;
        let mut height: u32 = 0;
        let result =
            unsafe { FPDFImageObj_GetImagePixelSize(self.handle, &mut width, &mut height) };

        if result != 0 {
            Some((width, height))
        } else {
            None
        }
    }

    /// Get the number of filters applied to an image object.
    ///
    /// Image data in PDFs can have multiple filters (compression methods)
    /// applied. This returns the count of filters.
    ///
    /// # Returns
    ///
    /// The number of filters, or 0 if not applicable.
    pub fn get_image_filter_count(&self) -> i32 {
        if self.object_type() != PageObjectType::Image {
            return 0;
        }
        unsafe { FPDFImageObj_GetImageFilterCount(self.handle) }
    }

    /// Get the name of a specific image filter.
    ///
    /// # Arguments
    ///
    /// * `index` - The filter index (0-based)
    ///
    /// # Returns
    ///
    /// The filter name (e.g., "FlateDecode", "DCTDecode"), or None if not found.
    pub fn get_image_filter(&self, index: i32) -> Option<String> {
        if self.object_type() != PageObjectType::Image {
            return None;
        }

        // First call to get buffer size
        let buf_len =
            unsafe { FPDFImageObj_GetImageFilter(self.handle, index, std::ptr::null_mut(), 0) };

        if buf_len == 0 {
            return None;
        }

        // Allocate buffer and get data
        let mut buffer = vec![0u8; buf_len as usize];
        let actual_len = unsafe {
            FPDFImageObj_GetImageFilter(self.handle, index, buffer.as_mut_ptr() as *mut _, buf_len)
        };

        if actual_len == 0 {
            return None;
        }

        // Remove null terminator if present
        if let Some(pos) = buffer.iter().position(|&b| b == 0) {
            buffer.truncate(pos);
        }

        String::from_utf8(buffer).ok()
    }
}

impl Drop for PdfNewPageObject {
    fn drop(&mut self) {
        // Note: Page objects should be inserted into a page before going out of scope.
        // FPDFPageObj_Destroy can be called on objects not added to pages, but it can
        // crash if internal references (like fonts) have been freed. For safety, we
        // don't auto-destroy - user should insert into page or explicitly handle cleanup.
        // If not inserted, the object will leak (better than crashing).
        // This is intentional - page objects are meant to be added to pages.
    }
}

/// Type of file identifier in a PDF document.
///
/// PDF documents can have two file identifiers:
/// - Permanent: Assigned when the document is first created, never changes
/// - Changing: Changes whenever the document is modified
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileIdentifierType {
    /// Permanent identifier (assigned at creation, never changes).
    Permanent,
    /// Changing identifier (updated on each modification).
    Changing,
}

impl FileIdentifierType {
    fn to_raw(self) -> u32 {
        match self {
            FileIdentifierType::Permanent => FPDF_FILEIDTYPE_FILEIDTYPE_PERMANENT,
            FileIdentifierType::Changing => FPDF_FILEIDTYPE_FILEIDTYPE_CHANGING,
        }
    }
}

/// Line cap style for stroked paths.
///
/// Determines how the ends of lines are drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineCap {
    /// Butt cap - line ends abruptly at the endpoint (default).
    #[default]
    Butt,
    /// Round cap - semicircle extends beyond the endpoint.
    Round,
    /// Projecting square cap - square extends half line width beyond endpoint.
    ProjectingSquare,
}

impl LineCap {
    fn to_raw(self) -> i32 {
        match self {
            LineCap::Butt => FPDF_LINECAP_BUTT as i32,
            LineCap::Round => FPDF_LINECAP_ROUND as i32,
            LineCap::ProjectingSquare => FPDF_LINECAP_PROJECTING_SQUARE as i32,
        }
    }

    fn from_raw(value: i32) -> Self {
        match value as u32 {
            FPDF_LINECAP_ROUND => LineCap::Round,
            FPDF_LINECAP_PROJECTING_SQUARE => LineCap::ProjectingSquare,
            _ => LineCap::Butt,
        }
    }
}

/// Line join style for stroked paths.
///
/// Determines how corners are drawn where lines meet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineJoin {
    /// Miter join - sharp corner (default).
    #[default]
    Miter,
    /// Round join - rounded corner.
    Round,
    /// Bevel join - flattened corner.
    Bevel,
}

impl LineJoin {
    fn to_raw(self) -> i32 {
        match self {
            LineJoin::Miter => FPDF_LINEJOIN_MITER as i32,
            LineJoin::Round => FPDF_LINEJOIN_ROUND as i32,
            LineJoin::Bevel => FPDF_LINEJOIN_BEVEL as i32,
        }
    }

    fn from_raw(value: i32) -> Self {
        match value as u32 {
            FPDF_LINEJOIN_ROUND => LineJoin::Round,
            FPDF_LINEJOIN_BEVEL => LineJoin::Bevel,
            _ => LineJoin::Miter,
        }
    }
}

/// Blend mode for transparency.
///
/// Controls how an object's colors combine with underlying content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BlendMode {
    /// Normal (default) - source replaces destination.
    #[default]
    Normal,
    /// Multiply - darkens the image.
    Multiply,
    /// Screen - lightens the image.
    Screen,
    /// Overlay - combines Multiply and Screen.
    Overlay,
    /// Darken - selects the darker of source and destination.
    Darken,
    /// Lighten - selects the lighter of source and destination.
    Lighten,
    /// ColorDodge - brightens destination based on source.
    ColorDodge,
    /// ColorBurn - darkens destination based on source.
    ColorBurn,
    /// HardLight - similar to Overlay but with source and destination swapped.
    HardLight,
    /// SoftLight - similar to HardLight but softer.
    SoftLight,
    /// Difference - subtracts darker from lighter.
    Difference,
    /// Exclusion - similar to Difference but lower contrast.
    Exclusion,
    /// Hue - uses source hue with destination saturation and luminosity.
    Hue,
    /// Saturation - uses source saturation with destination hue and luminosity.
    Saturation,
    /// Color - uses source hue and saturation with destination luminosity.
    Color,
    /// Luminosity - uses source luminosity with destination hue and saturation.
    Luminosity,
}

impl BlendMode {
    /// Get the PDF blend mode string.
    pub fn as_str(&self) -> &'static str {
        match self {
            BlendMode::Normal => "Normal",
            BlendMode::Multiply => "Multiply",
            BlendMode::Screen => "Screen",
            BlendMode::Overlay => "Overlay",
            BlendMode::Darken => "Darken",
            BlendMode::Lighten => "Lighten",
            BlendMode::ColorDodge => "ColorDodge",
            BlendMode::ColorBurn => "ColorBurn",
            BlendMode::HardLight => "HardLight",
            BlendMode::SoftLight => "SoftLight",
            BlendMode::Difference => "Difference",
            BlendMode::Exclusion => "Exclusion",
            BlendMode::Hue => "Hue",
            BlendMode::Saturation => "Saturation",
            BlendMode::Color => "Color",
            BlendMode::Luminosity => "Luminosity",
        }
    }
}

/// Type of a page object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageObjectType {
    /// Unknown object type.
    Unknown,
    /// Text object.
    Text,
    /// Path object (lines, curves, shapes).
    Path,
    /// Image object.
    Image,
    /// Shading object.
    Shading,
    /// Form XObject.
    Form,
}

impl PageObjectType {
    fn from_raw(value: i32) -> Self {
        match value as u32 {
            FPDF_PAGEOBJ_TEXT => PageObjectType::Text,
            FPDF_PAGEOBJ_PATH => PageObjectType::Path,
            FPDF_PAGEOBJ_IMAGE => PageObjectType::Image,
            FPDF_PAGEOBJ_SHADING => PageObjectType::Shading,
            FPDF_PAGEOBJ_FORM => PageObjectType::Form,
            _ => PageObjectType::Unknown,
        }
    }
}

/// A clip path that defines a clipping region.
///
/// Clip paths restrict the drawing area on a page or page object.
/// Content outside the clip path is not rendered.
///
/// # Example
/// ```no_run
/// use pdfium_render_fast::Pdfium;
///
/// let pdfium = Pdfium::new()?;
/// // Create a rectangular clip path
/// let clip = pdfium_render_fast::PdfClipPath::new_rect(100.0, 100.0, 400.0, 500.0)?;
/// assert_eq!(clip.path_count(), 1);
/// # Ok::<(), pdfium_render_fast::PdfError>(())
/// ```
pub struct PdfClipPath {
    handle: FPDF_CLIPPATH,
    owned: bool,
}

impl PdfClipPath {
    /// Create a rectangular clip path.
    ///
    /// The clip path will contain a single rectangular path with the given
    /// coordinates in PDF page units (typically 1/72 inch).
    ///
    /// # Arguments
    /// * `left` - Left edge x-coordinate
    /// * `bottom` - Bottom edge y-coordinate
    /// * `right` - Right edge x-coordinate
    /// * `top` - Top edge y-coordinate
    ///
    /// # Returns
    /// A new clip path, or an error if creation failed.
    pub fn new_rect(left: f32, bottom: f32, right: f32, top: f32) -> Result<Self> {
        let handle = unsafe { FPDF_CreateClipPath(left, bottom, right, top) };
        if handle.is_null() {
            return Err(PdfError::ClipPathCreationFailed);
        }
        Ok(Self {
            handle,
            owned: true,
        })
    }

    /// Wrap an existing clip path handle (non-owning).
    ///
    /// # Safety
    /// The handle must be valid for the lifetime of this struct.
    pub(crate) fn from_handle(handle: FPDF_CLIPPATH) -> Self {
        Self {
            handle,
            owned: false,
        }
    }

    /// Get the raw handle.
    pub(crate) fn handle(&self) -> FPDF_CLIPPATH {
        self.handle
    }

    /// Get the number of paths in this clip path.
    ///
    /// A clip path can contain multiple sub-paths that define the
    /// clipping region. Returns -1 on error.
    pub fn path_count(&self) -> i32 {
        unsafe { FPDFClipPath_CountPaths(self.handle) }
    }

    /// Get the number of segments in a specific path.
    ///
    /// # Arguments
    /// * `path_index` - Index of the path (0-based)
    ///
    /// # Returns
    /// The number of segments, or -1 on error.
    pub fn segment_count(&self, path_index: i32) -> i32 {
        unsafe { FPDFClipPath_CountPathSegments(self.handle, path_index) }
    }

    /// Get a segment from a path in this clip path.
    ///
    /// # Arguments
    /// * `path_index` - Index of the path (0-based)
    /// * `segment_index` - Index of the segment (0-based)
    ///
    /// # Returns
    /// The segment information, or None if not found.
    pub fn get_segment(&self, path_index: i32, segment_index: i32) -> Option<ClipPathSegment> {
        let segment =
            unsafe { FPDFClipPath_GetPathSegment(self.handle, path_index, segment_index) };
        if segment.is_null() {
            return None;
        }

        let mut x: f32 = 0.0;
        let mut y: f32 = 0.0;
        let has_point = unsafe { FPDFPathSegment_GetPoint(segment, &mut x, &mut y) } != 0;

        let seg_type = unsafe { FPDFPathSegment_GetType(segment) };
        let is_close = unsafe { FPDFPathSegment_GetClose(segment) } != 0;

        Some(ClipPathSegment {
            segment_type: crate::page_object::PathSegmentType::from(seg_type),
            x: if has_point { Some(x) } else { None },
            y: if has_point { Some(y) } else { None },
            is_close,
        })
    }
}

impl Drop for PdfClipPath {
    fn drop(&mut self) {
        if self.owned && !self.handle.is_null() {
            unsafe {
                FPDF_DestroyClipPath(self.handle);
            }
        }
    }
}

/// A segment of a clip path.
#[derive(Debug, Clone)]
pub struct ClipPathSegment {
    /// Type of the segment.
    pub segment_type: crate::page_object::PathSegmentType,
    /// X coordinate (if applicable).
    pub x: Option<f32>,
    /// Y coordinate (if applicable).
    pub y: Option<f32>,
    /// Whether this segment closes the path.
    pub is_close: bool,
}

/// ICC profile color space type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IccColorSpace {
    /// Unknown or unspecified color space.
    Unknown,
    /// Device Gray (1 component).
    DeviceGray,
    /// Device RGB (3 components).
    DeviceRgb,
    /// Device CMYK (4 components).
    DeviceCmyk,
    /// Calibrated Gray.
    CalGray,
    /// Calibrated RGB.
    CalRgb,
    /// L*a*b* color space.
    Lab,
    /// ICC-based color space.
    IccBased,
    /// Separation color space.
    Separation,
    /// DeviceN color space.
    DeviceN,
    /// Indexed color space.
    Indexed,
    /// Pattern color space.
    Pattern,
}

impl IccColorSpace {
    fn from_raw(value: u32) -> Self {
        match value {
            FPDF_COLORSPACE_DEVICEGRAY => IccColorSpace::DeviceGray,
            FPDF_COLORSPACE_DEVICERGB => IccColorSpace::DeviceRgb,
            FPDF_COLORSPACE_DEVICECMYK => IccColorSpace::DeviceCmyk,
            FPDF_COLORSPACE_CALGRAY => IccColorSpace::CalGray,
            FPDF_COLORSPACE_CALRGB => IccColorSpace::CalRgb,
            FPDF_COLORSPACE_LAB => IccColorSpace::Lab,
            FPDF_COLORSPACE_ICCBASED => IccColorSpace::IccBased,
            FPDF_COLORSPACE_SEPARATION => IccColorSpace::Separation,
            FPDF_COLORSPACE_DEVICEN => IccColorSpace::DeviceN,
            FPDF_COLORSPACE_INDEXED => IccColorSpace::Indexed,
            FPDF_COLORSPACE_PATTERN => IccColorSpace::Pattern,
            _ => IccColorSpace::Unknown,
        }
    }
}

/// ICC profile data extracted from an image object.
#[derive(Debug, Clone)]
pub struct IccProfile {
    /// The raw ICC profile data.
    pub data: Vec<u8>,
    /// The color space type.
    pub color_space: IccColorSpace,
}

// ============================================================================
// XObject (Form Template) API
// ============================================================================

/// An XObject (form template) that can be reused as page content.
///
/// XObjects are a PDF mechanism for reusable content. You can create an XObject
/// from a page of another document and then insert it multiple times into
/// pages of your target document. This is useful for:
///
/// - **Page templates**: Headers, footers, watermarks
/// - **N-up layouts**: Placing multiple pages on a single sheet
/// - **Stamping**: Adding logos or annotations to multiple pages
///
/// # Lifecycle
///
/// 1. Create an XObject from a source page using [`PdfDocument::create_xobject_from_page`]
/// 2. Create page objects from it using [`PdfXObject::to_page_object`]
/// 3. Insert those objects into target pages using [`PdfPage::insert_object`]
/// 4. The XObject is automatically freed when dropped
///
/// Note: The XObject remains valid until dropped. Page objects created from it
/// can outlive the XObject (they contain a copy of the content).
///
/// # Example
///
/// ```no_run
/// use pdfium_render_fast::Pdfium;
///
/// let pdfium = Pdfium::new()?;
///
/// // Create a new document
/// let doc = pdfium.create_new_document()?;
///
/// // Load template (e.g., a letterhead)
/// let template = pdfium.load_pdf_from_file("letterhead.pdf", None)?;
///
/// // Create XObject from the letterhead
/// let xobject = doc.create_xobject_from_page(&template, 0)?;
///
/// // Create several pages with the letterhead
/// for i in 0..5 {
///     let mut page = doc.new_page(i, 612.0, 792.0)?;
///     let form_obj = xobject.to_page_object()?;
///     page.insert_object(form_obj)?;
///     // The letterhead is now on this page
/// }
///
/// doc.save_to_file("letters.pdf", None)?;
/// # Ok::<(), pdfium_render_fast::PdfError>(())
/// ```
pub struct PdfXObject {
    handle: pdfium_sys::FPDF_XOBJECT,
}

impl PdfXObject {
    /// Create a new form page object from this XObject.
    ///
    /// The returned page object can be transformed (moved, scaled, rotated)
    /// and then inserted into a page.
    ///
    /// # Returns
    ///
    /// A new [`PdfNewPageObject`] containing the XObject content.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdfium_render_fast::Pdfium;
    ///
    /// let pdfium = Pdfium::new()?;
    /// let doc = pdfium.create_new_document()?;
    /// let template = pdfium.load_pdf_from_file("stamp.pdf", None)?;
    ///
    /// let xobject = doc.create_xobject_from_page(&template, 0)?;
    ///
    /// // Create the page and add the XObject content
    /// let mut page = doc.new_page(0, 612.0, 792.0)?;
    /// let mut form_obj = xobject.to_page_object()?;
    ///
    /// // Transform it (scale to 50%, move to bottom-right corner)
    /// form_obj.set_matrix(0.5, 0.0, 0.0, 0.5, 400.0, 50.0);
    ///
    /// // Insert into the page
    /// page.insert_object(form_obj)?;
    /// # Ok::<(), pdfium_render_fast::PdfError>(())
    /// ```
    pub fn to_page_object(&self) -> Result<PdfNewPageObject> {
        let obj_handle = unsafe { pdfium_sys::FPDF_NewFormObjectFromXObject(self.handle) };

        if obj_handle.is_null() {
            return Err(PdfError::OpenFailed {
                reason: "Failed to create form object from XObject".to_string(),
            });
        }

        Ok(PdfNewPageObject { handle: obj_handle })
    }

    /// Get the raw XObject handle.
    ///
    /// This is mainly useful for advanced operations with the underlying
    /// PDFium API.
    pub fn handle(&self) -> pdfium_sys::FPDF_XOBJECT {
        self.handle
    }
}

impl Drop for PdfXObject {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe {
                pdfium_sys::FPDF_CloseXObject(self.handle);
            }
        }
    }
}

// ============================================================================
// Repeated Content Detection API
// ============================================================================

/// A region of content that appears repeatedly across multiple pages.
///
/// This struct represents content that has been identified as appearing in
/// similar positions on multiple pages, such as headers, footers, logos,
/// or watermarks.
///
/// # Fields
///
/// * `bounds` - The bounding box (left, bottom, right, top) in page coordinates
/// * `page_indices` - List of page indices where this content appears
/// * `content_hash` - Hash of the content for identification
/// * `occurrence_count` - Number of pages where this content appears
///
/// # Example
///
/// ```no_run
/// use pdfium_render_fast::Pdfium;
///
/// let pdfium = Pdfium::new()?;
/// let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
///
/// // Find content that repeats across pages (e.g., headers/footers)
/// let repeated = doc.find_repeated_regions(5.0); // 5pt tolerance
///
/// for region in repeated {
///     println!("Content at ({:.1}, {:.1}) appears on {} pages: {:?}",
///         region.bounds.0, region.bounds.1,
///         region.occurrence_count,
///         region.page_indices);
/// }
/// # Ok::<(), pdfium_render_fast::PdfError>(())
/// ```
#[derive(Debug, Clone)]
pub struct RepeatedRegion {
    /// Bounding box in page coordinates (left, bottom, right, top).
    pub bounds: (f32, f32, f32, f32),
    /// Page indices where this content appears (0-based).
    pub page_indices: Vec<usize>,
    /// Hash of the content for identification.
    pub content_hash: u64,
    /// Number of pages where this content appears.
    pub occurrence_count: usize,
}

impl RepeatedRegion {
    /// Create a new repeated region.
    ///
    /// # Arguments
    /// * `bounds` - The bounding box (left, bottom, right, top)
    /// * `page_indices` - Pages where this content appears
    /// * `content_hash` - Hash identifying the content
    pub fn new(bounds: (f32, f32, f32, f32), page_indices: Vec<usize>, content_hash: u64) -> Self {
        let occurrence_count = page_indices.len();
        Self {
            bounds,
            page_indices,
            content_hash,
            occurrence_count,
        }
    }

    /// Check if this region appears in the header area (top 15% of page).
    ///
    /// # Arguments
    /// * `page_height` - The height of the page
    pub fn is_header(&self, page_height: f32) -> bool {
        self.bounds.3 > page_height * 0.85
    }

    /// Check if this region appears in the footer area (bottom 15% of page).
    pub fn is_footer(&self) -> bool {
        self.bounds.1 < self.bounds.3 * 0.15
    }

    /// Check if this region appears in the margin areas (left or right 10%).
    ///
    /// # Arguments
    /// * `page_width` - The width of the page
    pub fn is_margin(&self, page_width: f32) -> bool {
        self.bounds.0 < page_width * 0.10 || self.bounds.2 > page_width * 0.90
    }

    /// Get the center point of this region.
    pub fn center(&self) -> (f32, f32) {
        (
            (self.bounds.0 + self.bounds.2) / 2.0,
            (self.bounds.1 + self.bounds.3) / 2.0,
        )
    }

    /// Get the width of this region.
    pub fn width(&self) -> f32 {
        self.bounds.2 - self.bounds.0
    }

    /// Get the height of this region.
    pub fn height(&self) -> f32 {
        self.bounds.3 - self.bounds.1
    }

    /// Get the area of this region.
    pub fn area(&self) -> f32 {
        self.width() * self.height()
    }
}
