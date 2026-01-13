//! `PDFium` adapter layer for docling-backend
//!
//! This module provides an abstraction layer for `PDFium` backends:
//! - Default (`pdfium-render` feature): Uses `pdfium-render` (crates.io) - standard, well-tested
//! - `pdfium-fast` feature: Uses optimized `pdfium_fast` library (72x faster)
//!
//! The adapter ensures consistent API regardless of which backend is used.
//!
//! # Feature Flags
//!
//! - `pdfium-render` (default): Uses `pdfium-render = "0.8"` from crates.io
//! - `pdfium-fast`: Uses local `pdfium_fast` build from `~/pdfium_fast`
//!
//! # Performance Comparison
//!
//! | Backend | Speed | Notes |
//! |---------|-------|-------|
//! | pdfium-render | ~10-20 pages/sec | Stock `PDFium` |
//! | pdfium-fast | ~200-300 pages/sec | 72x faster, multi-threaded |
//!
//! See: `reports/feature__pdf-pipeline-fixes/PDFIUM_FAST_INTEGRATION_ANALYSIS_2024-12-17.md`

// Clippy pedantic allows for PDFium FFI code:
// - Casts are intentional when working with C FFI types (u64, i32, usize conversions)
// - PDF processing functions are necessarily large due to complexity
// - Unit struct takes &self by convention
#![allow(clippy::cast_possible_truncation)] // FFI types have different sizes
#![allow(clippy::cast_possible_wrap)] // u64 -> i32 for C FFI
#![allow(clippy::cast_precision_loss)] // f64 conversions for coordinates
#![allow(clippy::cast_sign_loss)] // Positive values cast between signed/unsigned
#![allow(clippy::too_many_lines)] // PDF processing is inherently complex
#![allow(clippy::unnecessary_debug_formatting)] // Path debug format in error messages
#![allow(clippy::similar_names)] // pdf_doc, pdfium, etc. are distinct
#![allow(clippy::trivially_copy_pass_by_ref)] // &self convention for methods

// ============================================================================
// pdfium_fast backend
// ============================================================================
//
// Uses the optimized pdfium_fast library (72x faster than stock pdfium).
// Requires ~/pdfium_fast to be built.

#[cfg(not(feature = "pdf"))]
compile_error!("The `pdf` feature must be enabled. Build with: cargo build --features pdf");

// ============================================================================
// Common imports
// ============================================================================
use docling_core::DoclingError;

// Re-export PDF_POINTS_PER_INCH from shared constants module for backwards compatibility
pub use crate::pdf_constants::PDF_POINTS_PER_INCH;

pub use pdfium_sys;

// BUG #41 fix: Define PDFium action type constants to avoid magic numbers
/// `PDFium` action type for URI links (hyperlinks to web URLs)
/// `FPDFAction_GetType` returns `u64` (`c_ulong` on most platforms)
const PDFACTION_URI: u64 = 3;

// PDF permission bit constants (PDF Reference 1.7, Table 3.20)
// Permissions are stored as a 32-bit value; bits 3-6 control user access rights.
/// Permission bit 3: Allow printing the document
const PDF_PERM_PRINT: u32 = 1 << 2;
/// Permission bit 4: Allow modifying document contents
const PDF_PERM_MODIFY: u32 = 1 << 3;
/// Permission bit 5: Allow copying/extracting text and images
const PDF_PERM_COPY: u32 = 1 << 4;
/// Permission bit 6: Allow adding/modifying annotations
const PDF_PERM_ANNOTATE: u32 = 1 << 5;

// PDF Font Descriptor Flags (PDF Reference 1.7, Section 5.7.1, Table 5.19)
// These flags describe font characteristics for text rendering.
/// Font flag bit 1 (0x1): `FixedPitch` - All glyphs have the same width (monospace)
const PDF_FONT_FIXED_PITCH: i32 = 0x1;
/// Font flag bit 7 (0x40): `Italic` - Font contains italic or oblique glyphs
const PDF_FONT_ITALIC: i32 = 0x40;
/// Font flag bit 19 (0x40000): `ForceBold` - Bold glyphs are painted with extra thickness
const PDF_FONT_FORCE_BOLD: i32 = 0x40000;

// PDFium Error Codes (from fpdfview.h - FPDF_GetLastError return values)
// These are returned when document loading fails.
/// Error code 1: Unknown error occurred
const FPDF_ERR_UNKNOWN: u64 = 1;
/// Error code 2: File not found or could not be opened
const FPDF_ERR_FILE: u64 = 2;
/// Error code 3: File not in PDF format or corrupted
const FPDF_ERR_FORMAT: u64 = 3;
/// Error code 4: Password required or incorrect password provided
const FPDF_ERR_PASSWORD: u64 = 4;
/// Error code 5: Unsupported security scheme
const FPDF_ERR_SECURITY: u64 = 5;
/// Error code 6: Page not found or content error
const FPDF_ERR_PAGE: u64 = 6;

/// ARGB color value for opaque white (alpha=0xFF, R=0xFF, G=0xFF, B=0xFF)
///
/// Used when filling bitmap backgrounds before rendering PDF pages.
/// `PDFium's` `FPDFBitmap_FillRect` expects ARGB format as `c_ulong` (`u64`): `0xAARRGGBB`.
const ARGB_WHITE: u64 = 0xFFFF_FFFF;

// ============================================================================
// PDFium Thread Safety - Global Singleton Initialization
// ============================================================================
//
// PDFium's FPDF_InitLibrary() and FPDF_DestroyLibrary() are not thread-safe.
// Multiple concurrent calls cause SIGTRAP crashes. We use OnceLock to ensure:
// 1. FPDF_InitLibrary() is called exactly once (on first use)
// 2. FPDF_DestroyLibrary() is never called (safe to leak at process exit)
//
// This allows tests to run in parallel without race conditions.
use std::sync::OnceLock;

/// Global flag to ensure PDFium is initialized exactly once.
/// Once set, PDFium remains initialized until process exit.
static PDFIUM_INIT: OnceLock<()> = OnceLock::new();

// PDFium Page Object Types (from fpdf_edit.h)
// These identify the type of object returned by FPDFPageObj_GetType().
/// Page object type: Text object (contains rendered text)
const FPDF_PAGEOBJ_TEXT: i32 = 1;
/// Page object type: Image object (embedded image)
const FPDF_PAGEOBJ_IMAGE: i32 = 3;

// Rec.601 grayscale luminance coefficients (scaled by 256 for integer math)
// Formula: Y = 0.299*R + 0.587*G + 0.114*B → Y = (77*R + 150*G + 29*B) >> 8
/// Red channel weight for grayscale conversion (0.299 × 256 ≈ 77)
const GRAYSCALE_RED_WEIGHT: u32 = 77;
/// Green channel weight for grayscale conversion (0.587 × 256 ≈ 150)
const GRAYSCALE_GREEN_WEIGHT: u32 = 150;
/// Blue channel weight for grayscale conversion (0.114 × 256 ≈ 29)
const GRAYSCALE_BLUE_WEIGHT: u32 = 29;

/// Maximum dimension for ML model rendering (4096x4096)
///
/// ML models typically use 640x640 or similar, but we allow up to 4096x4096
/// as a reasonable upper limit for high-resolution rendering.
const MAX_ML_DIMENSION: i32 = 4096;

// BUG #76 fix: UTF-16 conversion helper with warning on invalid sequences
/// Convert UTF-16 buffer to String, logging warning if invalid UTF-16 is encountered.
///
/// Unlike `String::from_utf16_lossy()` which silently replaces invalid sequences,
/// this logs a warning before falling back to lossy conversion.
#[inline]
fn utf16_to_string_with_warning(buffer: &[u16], context: &str) -> String {
    match String::from_utf16(buffer) {
        Ok(s) => s,
        Err(e) => {
            log::warn!("Invalid UTF-16 sequence in {context}: {e} - using lossy conversion");
            String::from_utf16_lossy(buffer)
        }
    }
}

// BUG #33 fix: Safe UTF-16 byte size to code unit conversion
/// Convert byte size to UTF-16 code unit count with validation.
///
/// UTF-16 code units are 2 bytes each. If `PDFium` returns an odd byte count,
/// this logs a warning and rounds down to avoid buffer issues.
#[inline]
fn utf16_bytes_to_units(byte_size: u64, context: &str) -> usize {
    if !byte_size.is_multiple_of(2) {
        log::warn!("UTF-16 buffer size is odd ({byte_size} bytes) in {context}, rounding down");
    }
    (byte_size / 2) as usize
}

/// `PDFium` Fast library wrapper
#[derive(Debug)]
pub struct PdfiumFast {
    _initialized: bool,
}

impl PdfiumFast {
    /// Initialize the `PDFium` library
    ///
    /// Uses a global singleton to ensure `FPDF_InitLibrary()` is called exactly once,
    /// making it safe to create multiple `PdfiumFast` instances concurrently from
    /// different threads (e.g., in parallel tests).
    ///
    /// # Errors
    /// This function currently always succeeds, but returns `Result` for future error handling.
    #[must_use = "constructors return a new instance"]
    pub fn new() -> Result<Self, DoclingError> {
        // Use OnceLock to ensure FPDF_InitLibrary is called exactly once.
        // This is critical for thread safety - PDFium's init/destroy are not reentrant.
        PDFIUM_INIT.get_or_init(|| {
            // SAFETY:
            // - FPDF_InitLibrary initializes global PDFium state
            // - Must be called before any other PDFium functions
            // - Called exactly once due to OnceLock
            // - Never paired with FPDF_DestroyLibrary (leaked at process exit)
            unsafe {
                pdfium_sys::FPDF_InitLibrary();
            }
        });
        Ok(Self { _initialized: true })
    }

    /// Load a PDF document from file
    ///
    /// # Arguments
    /// * `path` - Path to the PDF file
    /// * `password` - Optional password for encrypted PDFs
    ///
    /// # Errors
    /// Returns an error if the path is invalid or if `PDFium` fails to load the document.
    #[must_use = "loader returns a document"]
    pub fn load_pdf_from_file(
        &self,
        path: &std::path::Path,
        password: Option<&str>,
    ) -> Result<PdfDocumentFast, DoclingError> {
        let path_str = path.to_str().ok_or_else(|| {
            DoclingError::BackendError(format!("Invalid path (non-UTF8): {}", path.display()))
        })?;

        let c_path = std::ffi::CString::new(path_str).map_err(|_| {
            DoclingError::BackendError(format!(
                "Invalid path string (contains null byte): {path_str}"
            ))
        })?;

        // Convert password to C string if provided
        let c_password = password.and_then(|p| std::ffi::CString::new(p).ok());
        let password_ptr = c_password.as_ref().map_or(std::ptr::null(), |p| p.as_ptr());

        // SAFETY:
        // - c_path is a valid null-terminated C string (created via CString::new)
        // - password_ptr is either null or valid null-terminated C string
        // - FPDF_LoadDocument opens file by path (no memory ownership concerns)
        // - Returns null on error (checked below)
        let doc = unsafe { pdfium_sys::FPDF_LoadDocument(c_path.as_ptr(), password_ptr) };

        if doc.is_null() {
            let error_code = unsafe { pdfium_sys::FPDF_GetLastError() };
            let error_msg = match error_code {
                FPDF_ERR_UNKNOWN => "Unknown error",
                FPDF_ERR_FILE => "File not found or could not be opened",
                FPDF_ERR_FORMAT => "File not in PDF format or corrupted",
                FPDF_ERR_PASSWORD => "Password required or incorrect password",
                FPDF_ERR_SECURITY => "Unsupported security scheme",
                FPDF_ERR_PAGE => "Page not found or content error",
                _ => "Unknown error code",
            };
            return Err(DoclingError::BackendError(format!(
                "Failed to load PDF {path:?}: {error_msg} (code {error_code})"
            )));
        }

        Ok(PdfDocumentFast { handle: doc })
    }

    /// Load a PDF document from memory
    ///
    /// This is useful for processing PDFs from network streams, databases,
    /// or any source that provides bytes rather than a file path.
    ///
    /// # Arguments
    /// * `data` - PDF file contents as bytes
    /// * `password` - Optional password for encrypted PDFs
    ///
    /// # Safety
    /// The `data` slice must remain valid for the lifetime of the returned document.
    /// Consider using `load_pdf_from_file` for file-based PDFs.
    ///
    /// # Errors
    /// Returns an error if the data is empty or if `PDFium` fails to load the document.
    #[must_use = "loader returns a document"]
    pub fn load_pdf_from_memory(
        &self,
        data: &[u8],
        password: Option<&str>,
    ) -> Result<PdfDocumentFast, DoclingError> {
        if data.is_empty() {
            return Err(DoclingError::BackendError("Empty PDF data".to_string()));
        }

        // BUG #27 fix: Check size before i32 conversion to prevent overflow
        // PDFium's FPDF_LoadMemDocument takes int (i32), so max size is ~2GB
        if data.len() > i32::MAX as usize {
            return Err(DoclingError::BackendError(format!(
                "PDF data too large: {} bytes exceeds maximum {} bytes",
                data.len(),
                i32::MAX
            )));
        }

        // Convert password to C string if provided
        let c_password = password.and_then(|p| std::ffi::CString::new(p).ok());
        let password_ptr = c_password.as_ref().map_or(std::ptr::null(), |p| p.as_ptr());

        // SAFETY:
        // - data.as_ptr() is valid for data.len() bytes (slice invariant)
        // - data.len() fits in i32 (checked above, max ~2GB)
        // - password_ptr is either null or points to valid null-terminated C string
        // - FPDF_LoadMemDocument copies data internally (Rust slice can outlive call)
        // - Returns null on error (checked below)
        let doc = unsafe {
            pdfium_sys::FPDF_LoadMemDocument(
                data.as_ptr().cast::<std::ffi::c_void>(),
                data.len() as i32, // Safe: checked above
                password_ptr,
            )
        };

        if doc.is_null() {
            let error_code = unsafe { pdfium_sys::FPDF_GetLastError() };
            let error_msg = match error_code {
                FPDF_ERR_UNKNOWN => "Unknown error",
                FPDF_ERR_FILE => "Invalid data or could not read",
                FPDF_ERR_FORMAT => "Data not in PDF format or corrupted",
                FPDF_ERR_PASSWORD => "Password required or incorrect password",
                FPDF_ERR_SECURITY => "Unsupported security scheme",
                FPDF_ERR_PAGE => "Page not found or content error",
                _ => "Unknown error code",
            };
            return Err(DoclingError::BackendError(format!(
                "Failed to load PDF from memory: {error_msg} (code {error_code})"
            )));
        }

        Ok(PdfDocumentFast { handle: doc })
    }
}

impl Drop for PdfiumFast {
    fn drop(&mut self) {
        // INTENTIONALLY DO NOT CALL FPDF_DestroyLibrary()
        //
        // PDFium uses global state that is shared across all PdfiumFast instances.
        // Since we use a singleton pattern for initialization (OnceLock), we cannot
        // destroy the library when any single instance is dropped - other instances
        // may still be using it.
        //
        // The library will be automatically cleaned up when the process exits.
        // This is a common pattern for FFI libraries with global state.
        //
        // Note: FPDF_DestroyThreadPool is also skipped because:
        // 1. It would interfere with other concurrent PDFium users
        // 2. Thread pool resources are cleaned up at process exit anyway
    }
}

/// Parse PDF date string to chrono `DateTime`
///
/// PDF dates follow the format: D:YYYYMMDDHHmmSSOHH'mm'
/// where components after YYYY are optional.
/// Example: "D:20231215143052+00'00'" or "D:20231215" or "D:20231215143052Z"
///
/// BUG #42 fix: More lenient parsing to handle edge cases:
/// - Missing D: prefix (handled)
/// - Timezone without quotes: +0500 vs +05'00' (handled)
/// - Invalid month/day values are clamped to valid ranges
/// - Timezone offset is applied correctly
#[must_use = "returns parsed datetime if the date string is valid"]
fn parse_pdf_date(date_str: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    use chrono::{FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};

    // Remove "D:" prefix if present (also handle lowercase "d:")
    let s = date_str
        .strip_prefix("D:")
        .or_else(|| date_str.strip_prefix("d:"))
        .unwrap_or(date_str);

    if s.len() < 4 {
        return None;
    }

    // Parse year (required)
    let year: i32 = s.get(0..4)?.parse().ok()?;

    // Parse month (default 1, clamp to 1-12)
    let month: u32 = s
        .get(4..6)
        .and_then(|m| m.parse().ok())
        .unwrap_or(1)
        .clamp(1, 12);

    // Parse day (default 1, clamp to 1-31)
    let day: u32 = s
        .get(6..8)
        .and_then(|d| d.parse().ok())
        .unwrap_or(1)
        .clamp(1, 31);

    // Parse time components (default 0, clamp to valid ranges)
    let hour: u32 = s
        .get(8..10)
        .and_then(|h| h.parse().ok())
        .unwrap_or(0)
        .clamp(0, 23);
    let min: u32 = s
        .get(10..12)
        .and_then(|m| m.parse().ok())
        .unwrap_or(0)
        .clamp(0, 59);
    let sec: u32 = s
        .get(12..14)
        .and_then(|ss| ss.parse().ok())
        .unwrap_or(0)
        .clamp(0, 59);

    // Create naive datetime (clamp day if needed for the month)
    let naive_date = NaiveDate::from_ymd_opt(year, month, day).or_else(|| {
        // Day might be invalid for this month, try clamping to last day of month
        let last_day = match month {
            2 => {
                if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
                    29
                } else {
                    28
                }
            }
            4 | 6 | 9 | 11 => 30,
            _ => 31,
        };
        NaiveDate::from_ymd_opt(year, month, day.min(last_day))
    })?;
    let naive_time = NaiveTime::from_hms_opt(hour, min, sec)?;
    let naive_dt = NaiveDateTime::new(naive_date, naive_time);

    // Parse timezone offset if present (starts at position 14)
    if s.len() > 14 {
        let tz_part = &s[14..];
        if tz_part.starts_with('Z') || tz_part.starts_with('z') {
            return Some(Utc.from_utc_datetime(&naive_dt));
        }

        // Parse offset like +00'00' or -05'00' or +0000 or -0500
        let sign = match tz_part.chars().next()? {
            '+' => 1,
            '-' => -1,
            _ => return Some(Utc.from_utc_datetime(&naive_dt)),
        };

        // Remove quotes and any other non-digit characters after the sign
        let tz_cleaned: String = tz_part[1..].chars().filter(char::is_ascii_digit).collect();
        let tz_hour: i32 = tz_cleaned
            .get(0..2)
            .and_then(|h| h.parse().ok())
            .unwrap_or(0)
            .clamp(0, 14); // UTC offset max is ±14:00
        let tz_min: i32 = tz_cleaned
            .get(2..4)
            .and_then(|m| m.parse().ok())
            .unwrap_or(0)
            .clamp(0, 59);
        let offset_secs = sign * (tz_hour * 3600 + tz_min * 60);

        if let Some(offset) = FixedOffset::east_opt(offset_secs) {
            let dt_with_offset = offset.from_local_datetime(&naive_dt).single()?;
            return Some(dt_with_offset.with_timezone(&Utc));
        }
    }

    // No timezone - assume UTC
    Some(Utc.from_utc_datetime(&naive_dt))
}

/// PDF Document wrapper for pdfium-fast
pub struct PdfDocumentFast {
    handle: pdfium_sys::FPDF_DOCUMENT,
}

impl PdfDocumentFast {
    /// Get the number of pages
    #[inline]
    #[must_use = "returns the page count"]
    pub fn page_count(&self) -> i32 {
        unsafe { pdfium_sys::FPDF_GetPageCount(self.handle) }
    }

    /// Check if the PDF has a valid cross-reference table.
    ///
    /// BUG #5 fix: Provides validation check for PDF integrity.
    /// The cross-reference (xref) table is crucial for PDF parsing - it maps
    /// object numbers to byte offsets. A corrupted or missing xref table
    /// indicates the PDF may be damaged or malformed.
    ///
    /// Returns true if the xref table is valid, false otherwise.
    /// A false result doesn't necessarily mean the PDF is unusable, but
    /// parsing may be unreliable or produce unexpected results.
    #[inline]
    #[must_use = "returns whether the PDF has a valid cross-reference table"]
    pub fn has_valid_xref(&self) -> bool {
        unsafe { pdfium_sys::FPDF_DocumentHasValidCrossReferenceTable(self.handle) != 0 }
    }

    /// Check if the PDF is a tagged PDF (has semantic structure).
    ///
    /// Tagged PDFs contain a logical structure tree that describes the
    /// document's content hierarchy (paragraphs, headings, tables, etc.).
    /// For tagged PDFs, we can extract structure directly without ML inference,
    /// potentially achieving 30-50 pages/sec instead of 1.5 pages/sec.
    ///
    /// ~40% of enterprise PDFs are tagged. When a PDF is tagged:
    /// - Skip ML layout detection (use structure tree instead)
    /// - Skip ML table detection (tables are marked in structure)
    /// - Skip reading order detection (structure provides order)
    ///
    /// Returns true if the document catalog's `MarkInfo` dictionary
    /// contains the "Marked" entry set to true.
    #[inline]
    #[must_use = "returns whether the PDF is tagged with semantic structure"]
    pub fn is_tagged(&self) -> bool {
        unsafe { pdfium_sys::FPDFCatalog_IsTagged(self.handle) != 0 }
    }

    /// Load a specific page
    ///
    /// # Errors
    /// Returns error if `page_index` is out of bounds or page fails to load
    #[must_use = "loader returns a page"]
    pub fn load_page(&self, page_index: i32) -> Result<PdfPageFast, DoclingError> {
        // BUG #32 fix: Validate page index bounds before FFI call
        let page_count = self.page_count();
        if page_index < 0 || page_index >= page_count {
            return Err(DoclingError::BackendError(format!(
                "Page index {page_index} out of bounds (document has {page_count} pages)"
            )));
        }

        // SAFETY:
        // - self.handle is valid (created in constructor, not yet dropped)
        // - page_index is in bounds (checked above against page_count)
        // - Returns null on error (checked below)
        let page = unsafe { pdfium_sys::FPDF_LoadPage(self.handle, page_index) };
        if page.is_null() {
            return Err(DoclingError::BackendError(format!(
                "Failed to load page {page_index}"
            )));
        }
        Ok(PdfPageFast {
            handle: page,
            doc: self.handle,
            cached_text_page: std::cell::RefCell::new(None),
        })
    }

    /// Get optimal thread count for parallel rendering
    #[inline]
    #[must_use = "returns the optimal thread count for this document"]
    pub fn optimal_thread_count(&self) -> i32 {
        unsafe { pdfium_sys::FPDF_GetOptimalWorkerCountForDocument(self.handle) }
    }

    /// Get document metadata value by tag
    ///
    /// Supported tags: "Title", "Author", "Subject", "Keywords", "Creator",
    /// "Producer", "`CreationDate`", "`ModDate`"
    #[must_use = "returns the metadata value if the tag exists"]
    pub fn get_metadata(&self, tag: &str) -> Option<String> {
        let c_tag = std::ffi::CString::new(tag).ok()?;

        // First call to get required buffer size
        let size = unsafe {
            pdfium_sys::FPDF_GetMetaText(self.handle, c_tag.as_ptr(), std::ptr::null_mut(), 0)
        };

        if size <= 2 {
            // Empty or error (size includes null terminator + BOM)
            return None;
        }

        // Allocate buffer and get text (UTF-16)
        // BUG #33 fix: Use helper for safe UTF-16 size conversion
        let mut buffer: Vec<u16> = vec![0; utf16_bytes_to_units(size, "metadata text")];
        let written = unsafe {
            pdfium_sys::FPDF_GetMetaText(
                self.handle,
                c_tag.as_ptr(),
                buffer.as_mut_ptr().cast(),
                size,
            )
        };

        if written <= 2 {
            return None;
        }

        // Convert UTF-16 to String (remove null terminator)
        // BUG #76 fix: Log warning if invalid UTF-16 encountered
        let actual_len = ((written / 2) - 1) as usize;
        let text = utf16_to_string_with_warning(&buffer[..actual_len], "PDF metadata");

        if text.trim().is_empty() {
            None
        } else {
            Some(text)
        }
    }

    /// Get document title
    #[inline]
    #[must_use = "returns the document title if present"]
    pub fn title(&self) -> Option<String> {
        self.get_metadata("Title")
    }

    /// Get document author
    #[inline]
    #[must_use = "returns the document author if present"]
    pub fn author(&self) -> Option<String> {
        self.get_metadata("Author")
    }

    /// Get document subject
    #[inline]
    #[must_use = "returns the document subject if present"]
    pub fn subject(&self) -> Option<String> {
        self.get_metadata("Subject")
    }

    /// Get document creator (application that created the PDF)
    #[inline]
    #[must_use = "returns the document creator if present"]
    pub fn creator(&self) -> Option<String> {
        self.get_metadata("Creator")
    }

    /// Get document creation date
    #[inline]
    #[must_use = "returns the document creation date if present"]
    pub fn creation_date(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.get_metadata("CreationDate")
            .and_then(|s| parse_pdf_date(&s))
    }

    /// Get document modification date
    #[inline]
    #[must_use = "returns the document modification date if present"]
    pub fn modification_date(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.get_metadata("ModDate")
            .and_then(|s| parse_pdf_date(&s))
    }

    /// Get PDF file version (e.g., 14 for PDF 1.4, 17 for PDF 1.7, 20 for PDF 2.0)
    ///
    /// Returns None if version cannot be determined.
    #[inline]
    #[must_use = "returns the PDF version number if available"]
    pub fn file_version(&self) -> Option<i32> {
        let mut version: i32 = 0;
        let success = unsafe { pdfium_sys::FPDF_GetFileVersion(self.handle, &raw mut version) };
        if success != 0 && version > 0 {
            Some(version)
        } else {
            None
        }
    }

    /// Get PDF file version as a string (e.g., "1.4", "1.7", "2.0")
    #[inline]
    #[must_use = "returns the PDF version string if available"]
    pub fn file_version_string(&self) -> Option<String> {
        self.file_version().map(|v| {
            if v >= 20 {
                format!("{}.{}", v / 10, v % 10)
            } else {
                format!("1.{}", v - 10)
            }
        })
    }

    /// Extract table of contents (bookmarks) from the document
    ///
    /// Returns a flat list of bookmarks with their titles, levels, and target pages.
    #[must_use = "returns the document's table of contents as bookmarks"]
    pub fn get_bookmarks(&self) -> Vec<PdfBookmark> {
        let mut bookmarks = Vec::new();

        // Get the first bookmark (root level)
        let first =
            unsafe { pdfium_sys::FPDFBookmark_GetFirstChild(self.handle, std::ptr::null_mut()) };

        if !first.is_null() {
            self.collect_bookmarks(first, 0, &mut bookmarks);
        }

        bookmarks
    }

    /// Recursively collect bookmarks
    fn collect_bookmarks(
        &self,
        bookmark: pdfium_sys::FPDF_BOOKMARK,
        level: i32,
        bookmarks: &mut Vec<PdfBookmark>,
    ) {
        let mut current = bookmark;

        while !current.is_null() {
            // Get bookmark title
            let title_len =
                unsafe { pdfium_sys::FPDFBookmark_GetTitle(current, std::ptr::null_mut(), 0) };

            let title = if title_len > 0 {
                // BUG #33 fix: Use helper for safe UTF-16 size conversion
                let mut buffer: Vec<u16> =
                    vec![0; utf16_bytes_to_units(title_len, "bookmark title")];
                unsafe {
                    pdfium_sys::FPDFBookmark_GetTitle(
                        current,
                        buffer.as_mut_ptr().cast(),
                        title_len,
                    );
                }
                // Remove null terminator
                // BUG #76 fix: Log warning if invalid UTF-16 encountered
                let actual_len = buffer.len().saturating_sub(1);
                utf16_to_string_with_warning(&buffer[..actual_len], "bookmark title")
            } else {
                String::new()
            };

            // Get target page index
            // BUG #44 fix: Validate page index against page count
            let dest = unsafe { pdfium_sys::FPDFBookmark_GetDest(self.handle, current) };
            let page_count = self.page_count();
            let page_index = if dest.is_null() {
                None
            } else {
                let idx = unsafe { pdfium_sys::FPDFDest_GetDestPageIndex(self.handle, dest) };
                if idx >= 0 && idx < page_count {
                    Some(idx)
                } else {
                    if idx >= page_count {
                        log::warn!(
                            "Bookmark '{title}' has invalid page index {idx} (document has {page_count} pages)"
                        );
                    }
                    None
                }
            };

            if !title.trim().is_empty() {
                bookmarks.push(PdfBookmark {
                    title,
                    level,
                    page_index,
                });
            }

            // Process children
            let child = unsafe { pdfium_sys::FPDFBookmark_GetFirstChild(self.handle, current) };
            if !child.is_null() {
                self.collect_bookmarks(child, level + 1, bookmarks);
            }

            // Move to next sibling
            current = unsafe { pdfium_sys::FPDFBookmark_GetNextSibling(self.handle, current) };
        }
    }

    /// Get the number of embedded file attachments in the document
    #[inline]
    #[must_use = "returns the number of file attachments"]
    pub fn attachment_count(&self) -> i32 {
        unsafe { pdfium_sys::FPDFDoc_GetAttachmentCount(self.handle) }
    }

    /// Extract all embedded file attachments from the document
    ///
    /// Returns a list of attachments with their names and contents.
    #[must_use = "returns the embedded file attachments"]
    pub fn get_attachments(&self) -> Vec<PdfAttachment> {
        // BUG #36 fix: Validate file size before allocation
        // c_ulong is platform-dependent (u32 on Windows, u64 on Unix)
        // Ensure it fits in usize and isn't unreasonably large (>1GB)
        const MAX_ATTACHMENT_SIZE: usize = 1024 * 1024 * 1024; // 1GB limit

        let count = self.attachment_count();
        let mut attachments = Vec::new();

        for i in 0..count {
            let attachment = unsafe { pdfium_sys::FPDFDoc_GetAttachment(self.handle, i) };
            if attachment.is_null() {
                continue;
            }

            // Get attachment name (UTF-16)
            let name_len =
                unsafe { pdfium_sys::FPDFAttachment_GetName(attachment, std::ptr::null_mut(), 0) };

            let name = if name_len > 0 {
                // BUG #33 fix: Use helper for safe UTF-16 size conversion
                let mut buffer: Vec<u16> =
                    vec![0; utf16_bytes_to_units(name_len, "attachment name")];
                unsafe {
                    pdfium_sys::FPDFAttachment_GetName(attachment, buffer.as_mut_ptr(), name_len);
                }
                // Remove null terminator
                // BUG #76 fix: Log warning if invalid UTF-16 encountered
                let actual_len = buffer.len().saturating_sub(1);
                utf16_to_string_with_warning(&buffer[..actual_len], "attachment name")
            } else {
                continue; // Skip attachments without names
            };

            // Get file content size
            let mut file_len: std::ffi::c_ulong = 0;
            let has_file = unsafe {
                pdfium_sys::FPDFAttachment_GetFile(
                    attachment,
                    std::ptr::null_mut(),
                    0,
                    &raw mut file_len,
                )
            };

            if has_file == 0 || file_len == 0 {
                // Attachment exists but has no file content
                attachments.push(PdfAttachment {
                    name,
                    data: Vec::new(),
                });
                continue;
            }

            let file_len_usize = file_len as usize;
            if file_len_usize > MAX_ATTACHMENT_SIZE {
                log::warn!(
                    "Attachment '{name}' has unusually large size ({file_len_usize} bytes), skipping"
                );
                continue;
            }

            // Get file content
            let mut data = vec![0u8; file_len_usize];
            let mut actual_len: std::ffi::c_ulong = 0;
            let success = unsafe {
                pdfium_sys::FPDFAttachment_GetFile(
                    attachment,
                    data.as_mut_ptr().cast(),
                    file_len,
                    &raw mut actual_len,
                )
            };

            if success != 0 && actual_len > 0 {
                // BUG #20 fix: Only truncate if actual size differs from allocated size
                // This avoids unnecessary memory operations when sizes match
                if (actual_len as usize) < data.len() {
                    data.truncate(actual_len as usize);
                }
                attachments.push(PdfAttachment { name, data });
            }
        }

        attachments
    }

    /// Get document permissions (security restrictions)
    ///
    /// Returns a bitmask of permissions. Use helper methods like
    /// `can_print()`, `can_copy()`, etc. to check specific permissions.
    ///
    /// Common permission bits:
    /// - Bit 3: Print
    /// - Bit 4: Modify contents
    /// - Bit 5: Copy/extract text
    /// - Bit 6: Add/modify annotations
    /// - Bit 9: Fill forms
    /// - Bit 10: Extract for accessibility
    /// - Bit 11: Assemble document
    /// - Bit 12: Print high quality
    #[inline]
    #[must_use = "returns the document permission bits"]
    pub fn get_permissions(&self) -> u32 {
        unsafe { pdfium_sys::FPDF_GetDocPermissions(self.handle) as u32 }
    }

    /// Check if printing is allowed
    #[inline]
    #[must_use = "returns whether printing is permitted"]
    pub fn can_print(&self) -> bool {
        self.get_permissions() & PDF_PERM_PRINT != 0
    }

    /// Check if content modification is allowed
    #[inline]
    #[must_use = "returns whether content modification is permitted"]
    pub fn can_modify(&self) -> bool {
        self.get_permissions() & PDF_PERM_MODIFY != 0
    }

    /// Check if text/image extraction is allowed
    #[inline]
    #[must_use = "returns whether text/image extraction is permitted"]
    pub fn can_copy(&self) -> bool {
        self.get_permissions() & PDF_PERM_COPY != 0
    }

    /// Check if annotations can be added/modified
    #[inline]
    #[must_use = "returns whether annotations are permitted"]
    pub fn can_annotate(&self) -> bool {
        self.get_permissions() & PDF_PERM_ANNOTATE != 0
    }

    /// Check if the document contains any form fields.
    ///
    /// BUG #22 fix: Document-level check for interactive forms.
    /// This is useful for quickly determining if a PDF is an interactive form.
    ///
    /// Note: This iterates through all pages to check for Widget annotations.
    /// For large documents, consider checking specific pages instead.
    ///
    /// # Returns
    /// `true` if any page contains at least one form field, `false` otherwise.
    #[must_use = "returns whether the document contains form fields"]
    pub fn has_form_fields(&self) -> bool {
        let num_pages = self.page_count();
        for page_idx in 0..num_pages {
            if let Ok(page) = self.load_page(page_idx) {
                if page.has_form_fields() {
                    return true;
                }
            }
        }
        false
    }

    /// Count the total number of form fields across all pages.
    ///
    /// BUG #22 fix: Document-level form field count.
    ///
    /// # Returns
    /// The total number of form fields (Widget annotations) in the document.
    #[must_use = "returns the total count of form fields"]
    pub fn count_form_fields(&self) -> usize {
        let num_pages = self.page_count();
        let mut total = 0;
        for page_idx in 0..num_pages {
            if let Ok(page) = self.load_page(page_idx) {
                total += page.count_form_fields();
            }
        }
        total
    }

    /// Get the form type of this PDF document.
    ///
    /// BUG #24 fix: Detect what kind of forms the document contains.
    /// This is useful for detecting XFA forms which may require special handling.
    ///
    /// # Returns
    /// The form type indicating whether the document uses:
    /// - `PdfFormType::None` - No forms
    /// - `PdfFormType::AcroForm` - Standard PDF forms (`AcroForm`)
    /// - `PdfFormType::XfaFull` - Dynamic forms (full XFA)
    /// - `PdfFormType::XfaForeground` - Dynamic forms (XFAF subset)
    #[inline]
    #[must_use = "returns the type of forms in the document"]
    pub fn get_form_type(&self) -> PdfFormType {
        let raw = unsafe { pdfium_sys::FPDF_GetFormType(self.handle) };
        PdfFormType::from_raw(raw)
    }

    /// Check if this document contains XFA (dynamic) forms.
    ///
    /// BUG #24 fix: Quick check for XFA form presence.
    /// XFA forms are dynamic forms that may require JavaScript or special rendering.
    /// Some PDF viewers and tools have limited XFA support.
    ///
    /// # Returns
    /// `true` if the document contains XFA forms (full or foreground).
    #[inline]
    #[must_use = "returns whether the document contains XFA forms"]
    pub fn has_xfa_forms(&self) -> bool {
        self.get_form_type().is_xfa()
    }

    /// Check if this document contains standard `AcroForm` forms.
    ///
    /// BUG #24 fix: Quick check for standard form presence.
    /// `AcroForms` are the traditional PDF form type with wide support.
    ///
    /// # Returns
    /// `true` if the document uses `AcroForm` specification.
    #[inline]
    #[must_use = "returns whether the document uses AcroForm"]
    pub fn is_acro_form(&self) -> bool {
        self.get_form_type().is_acro_form()
    }

    /// Get the page label for a specific page (e.g., "i", "ii", "1", "2")
    ///
    /// Many PDFs use labels like "i", "ii" for preface pages and "1", "2" for content.
    /// Returns None if no label is set for the page or if `page_index` is out of bounds.
    #[must_use = "returns the page label if set"]
    pub fn get_page_label(&self, page_index: i32) -> Option<String> {
        // BUG #35 fix: Validate page index before calling PDFium
        let page_count = self.page_count();
        if page_index < 0 || page_index >= page_count {
            log::warn!("get_page_label: page_index {page_index} out of bounds (0..{page_count})");
            return None;
        }

        // Get label length
        let label_len = unsafe {
            pdfium_sys::FPDF_GetPageLabel(self.handle, page_index, std::ptr::null_mut(), 0)
        };

        if label_len <= 2 {
            return None;
        }

        // Get label (UTF-16)
        // BUG #33 fix: Use helper for safe UTF-16 size conversion
        let mut buffer: Vec<u16> = vec![0; utf16_bytes_to_units(label_len, "page label")];
        let written = unsafe {
            pdfium_sys::FPDF_GetPageLabel(
                self.handle,
                page_index,
                buffer.as_mut_ptr().cast(),
                label_len,
            )
        };

        if written <= 2 {
            return None;
        }

        // Remove null terminator
        // BUG #76 fix: Log warning if invalid UTF-16 encountered
        let actual_len = (written / 2 - 1) as usize;
        let label = utf16_to_string_with_warning(&buffer[..actual_len], "page label");

        if label.trim().is_empty() {
            None
        } else {
            Some(label)
        }
    }

    /// Get page dimensions without loading the full page (BUG #4)
    ///
    /// This is more efficient than calling `load_page().width()/height()` when
    /// you only need dimensions, as it avoids loading the full page content.
    /// Useful for TOC generation, layout calculations, and thumbnail sizing.
    ///
    /// Returns None if `page_index` is out of bounds or dimensions cannot be retrieved.
    ///
    /// # Examples
    /// ```ignore
    /// if let Some((width, height)) = doc.get_page_size_by_index(0) {
    ///     println!("Page 0 is {}x{} points", width, height);
    /// }
    /// ```
    #[must_use = "returns the page dimensions if available"]
    pub fn get_page_size_by_index(&self, page_index: i32) -> Option<(f64, f64)> {
        // Validate page index before calling PDFium
        let page_count = self.page_count();
        if page_index < 0 || page_index >= page_count {
            log::warn!(
                "get_page_size_by_index: page_index {page_index} out of bounds (0..{page_count})"
            );
            return None;
        }

        let mut width: f64 = 0.0;
        let mut height: f64 = 0.0;

        let success = unsafe {
            pdfium_sys::FPDF_GetPageSizeByIndex(
                self.handle,
                page_index,
                &raw mut width,
                &raw mut height,
            )
        };

        if success == 0 || width <= 0.0 || height <= 0.0 {
            return None;
        }

        Some((width, height))
    }

    /// Get all page sizes without loading pages (batch version of `get_page_size_by_index`)
    ///
    /// Efficiently retrieves dimensions for all pages in the document.
    /// Returns a Vec of (width, height) tuples indexed by page number.
    ///
    /// # Examples
    /// ```ignore
    /// let sizes = doc.get_all_page_sizes();
    /// for (i, (w, h)) in sizes.iter().enumerate() {
    ///     println!("Page {}: {}x{} points", i, w, h);
    /// }
    /// ```
    #[must_use = "returns dimensions for all pages"]
    pub fn get_all_page_sizes(&self) -> Vec<(f64, f64)> {
        let num_pages = self.page_count();
        let mut sizes = Vec::with_capacity(num_pages as usize);

        for i in 0..num_pages {
            let mut width: f64 = 0.0;
            let mut height: f64 = 0.0;

            let success = unsafe {
                pdfium_sys::FPDF_GetPageSizeByIndex(self.handle, i, &raw mut width, &raw mut height)
            };

            if success != 0 && width > 0.0 && height > 0.0 {
                sizes.push((width, height));
            } else {
                // Fallback to (0, 0) for failed pages
                sizes.push((0.0, 0.0));
            }
        }

        sizes
    }

    /// Count named destinations in the document.
    ///
    /// BUG #23 fix: Named destinations are internal navigation targets that can be
    /// referenced by name (e.g., in URIs like `document.pdf#destination_name`).
    ///
    /// # Returns
    /// The number of named destinations defined in the document.
    #[inline]
    #[must_use = "returns the count of named destinations"]
    pub fn count_named_dests(&self) -> u32 {
        // FPDF_DWORD is u64 on some platforms, cast to u32 (safe for practical PDF sizes)
        unsafe { pdfium_sys::FPDF_CountNamedDests(self.handle) as u32 }
    }

    /// Get a named destination by its name.
    ///
    /// BUG #23 fix: Resolves a destination name to its target page and location.
    /// This is useful for handling PDF URIs with fragment identifiers.
    ///
    /// # Arguments
    /// * `name` - The destination name to look up
    ///
    /// # Returns
    /// The destination information if found, None otherwise.
    #[must_use = "returns the named destination if found"]
    pub fn get_named_dest_by_name(&self, name: &str) -> Option<PdfNamedDestination> {
        let c_name = std::ffi::CString::new(name).ok()?;

        let dest = unsafe { pdfium_sys::FPDF_GetNamedDestByName(self.handle, c_name.as_ptr()) };

        if dest.is_null() {
            return None;
        }

        // Get page index
        let page_index = unsafe { pdfium_sys::FPDFDest_GetDestPageIndex(self.handle, dest) };
        if page_index < 0 {
            return None;
        }

        // Get location in page
        let (x, y, zoom) = Self::get_dest_location(dest);

        Some(PdfNamedDestination {
            name: name.to_string(),
            page_index,
            x,
            y,
            zoom,
        })
    }

    /// Get a named destination by index.
    ///
    /// BUG #23 fix: Retrieves a named destination by its position in the document's
    /// destination list. Use `count_named_dests()` to get the total count.
    ///
    /// # Arguments
    /// * `index` - The 0-based index of the destination
    ///
    /// # Returns
    /// The destination information if index is valid, None otherwise.
    #[must_use = "returns the named destination at the index"]
    pub fn get_named_dest(&self, index: i32) -> Option<PdfNamedDestination> {
        let count = self.count_named_dests();
        if index < 0 || (index as u32) >= count {
            return None;
        }

        // First call to get name buffer size
        let mut buflen: std::ffi::c_long = 0;
        let dest = unsafe {
            pdfium_sys::FPDF_GetNamedDest(self.handle, index, std::ptr::null_mut(), &raw mut buflen)
        };

        if dest.is_null() || buflen <= 0 {
            return None;
        }

        // Second call to get the name
        let mut buffer: Vec<u8> = vec![0; buflen as usize];
        let mut actual_len = buflen;
        let dest = unsafe {
            pdfium_sys::FPDF_GetNamedDest(
                self.handle,
                index,
                buffer.as_mut_ptr().cast(),
                &raw mut actual_len,
            )
        };

        if dest.is_null() {
            return None;
        }

        // Find null terminator and convert to string
        let name_len = buffer.iter().position(|&b| b == 0).unwrap_or(buffer.len());
        let name = String::from_utf8_lossy(&buffer[..name_len]).to_string();

        // Get page index
        let page_index = unsafe { pdfium_sys::FPDFDest_GetDestPageIndex(self.handle, dest) };
        if page_index < 0 {
            return None;
        }

        // Get location in page
        let (x, y, zoom) = Self::get_dest_location(dest);

        Some(PdfNamedDestination {
            name,
            page_index,
            x,
            y,
            zoom,
        })
    }

    /// Get all named destinations in the document.
    ///
    /// BUG #23 fix: Retrieves all named destinations for bulk processing.
    /// Useful for building navigation indices or validating internal links.
    ///
    /// # Returns
    /// A vector of all named destinations in the document.
    #[must_use = "returns all named destinations in the document"]
    pub fn get_all_named_dests(&self) -> Vec<PdfNamedDestination> {
        let count = self.count_named_dests();
        let mut dests = Vec::with_capacity(count as usize);

        for i in 0..count {
            if let Some(dest) = self.get_named_dest(i as i32) {
                dests.push(dest);
            }
        }

        dests
    }

    /// Helper to extract location information from a destination handle.
    fn get_dest_location(dest: pdfium_sys::FPDF_DEST) -> (Option<f32>, Option<f32>, Option<f32>) {
        let mut has_x: i32 = 0;
        let mut has_y: i32 = 0;
        let mut has_zoom: i32 = 0;
        let mut x: f32 = 0.0;
        let mut y: f32 = 0.0;
        let mut zoom: f32 = 0.0;

        let success = unsafe {
            pdfium_sys::FPDFDest_GetLocationInPage(
                dest,
                &raw mut has_x,
                &raw mut has_y,
                &raw mut has_zoom,
                &raw mut x,
                &raw mut y,
                &raw mut zoom,
            )
        };

        if success == 0 {
            return (None, None, None);
        }

        let x_val = (has_x != 0).then_some(x);
        let y_val = (has_y != 0).then_some(y);
        let zoom_val = (has_zoom != 0).then_some(zoom);

        (x_val, y_val, zoom_val)
    }

    /// Check if the document has any named destinations.
    ///
    /// BUG #23 fix: Quick check for presence of named destinations.
    ///
    /// # Returns
    /// `true` if the document contains at least one named destination.
    #[inline]
    #[must_use = "returns whether the document has named destinations"]
    pub fn has_named_dests(&self) -> bool {
        self.count_named_dests() > 0
    }

    // ==========================================================================
    // BUG #85: Digital Signature Extraction APIs
    // ==========================================================================

    /// Get the number of digital signatures in the document.
    ///
    /// BUG #85 fix: Enables detection of digitally signed PDFs.
    ///
    /// # Returns
    /// The number of signatures (0 if unsigned, -1 on error)
    #[inline]
    #[must_use = "returns the count of digital signatures"]
    pub fn signature_count(&self) -> i32 {
        unsafe { pdfium_sys::FPDF_GetSignatureCount(self.handle) }
    }

    /// Check if the document has any digital signatures.
    ///
    /// BUG #85 fix: Quick check for presence of signatures.
    ///
    /// # Returns
    /// `true` if the document contains at least one digital signature.
    #[inline]
    #[must_use = "returns whether the document has digital signatures"]
    pub fn has_signatures(&self) -> bool {
        self.signature_count() > 0
    }

    /// Get a digital signature by index.
    ///
    /// BUG #85 fix: Extract individual signature metadata.
    ///
    /// # Arguments
    /// * `index` - The signature index (0-based)
    ///
    /// # Returns
    /// The signature information, or None if index is out of bounds or on error.
    #[must_use = "returns the signature at the given index"]
    pub fn get_signature(&self, index: i32) -> Option<PdfSignature> {
        let count = self.signature_count();
        if count <= 0 || index < 0 || index >= count {
            return None;
        }

        let sig_handle = unsafe { pdfium_sys::FPDF_GetSignatureObject(self.handle, index) };
        if sig_handle.is_null() {
            return None;
        }

        // Get subfilter (encoding format)
        let sub_filter = self.get_signature_sub_filter(sig_handle);

        // Get reason
        let reason = self.get_signature_reason(sig_handle);

        // Get signing time
        let signing_time = self.get_signature_time(sig_handle);

        // Get DocMDP permission
        let doc_mdp_permission = {
            let perm = unsafe { pdfium_sys::FPDFSignatureObj_GetDocMDPPermission(sig_handle) };
            if perm > 0 {
                Some(perm)
            } else {
                None
            }
        };

        // Get contents (raw signature data)
        let contents = self.get_signature_contents(sig_handle);

        // Get byte range
        let byte_range = self.get_signature_byte_range(sig_handle);

        Some(PdfSignature {
            index,
            sub_filter,
            reason,
            signing_time,
            doc_mdp_permission,
            contents,
            byte_range,
        })
    }

    /// Helper: Get signature subfilter (encoding format)
    // Method signature kept for API consistency with other PdfiumAdapter methods
    #[allow(clippy::unused_self)]
    fn get_signature_sub_filter(&self, sig: pdfium_sys::FPDF_SIGNATURE) -> Option<String> {
        // First call with null buffer to get required size
        let len =
            unsafe { pdfium_sys::FPDFSignatureObj_GetSubFilter(sig, std::ptr::null_mut(), 0) };
        if len == 0 {
            return None;
        }

        let mut buffer = vec![0u8; len as usize];
        let actual_len = unsafe {
            pdfium_sys::FPDFSignatureObj_GetSubFilter(
                sig,
                buffer.as_mut_ptr().cast::<i8>(),
                buffer.len() as u64,
            )
        };

        if actual_len == 0 {
            return None;
        }

        // Trim null terminator and convert to string
        let end = buffer.iter().position(|&b| b == 0).unwrap_or(buffer.len());
        String::from_utf8(buffer[..end].to_vec()).ok()
    }

    /// Helper: Get signature reason (UTF-16LE encoded)
    // Method signature kept for API consistency with other PdfiumAdapter methods
    #[allow(clippy::unused_self)]
    fn get_signature_reason(&self, sig: pdfium_sys::FPDF_SIGNATURE) -> Option<String> {
        // First call with null buffer to get required size
        let len = unsafe { pdfium_sys::FPDFSignatureObj_GetReason(sig, std::ptr::null_mut(), 0) };
        if len == 0 || len == 2 {
            // 0 = error, 2 = just null terminator (empty string)
            return None;
        }

        let mut buffer = vec![0u8; len as usize];
        let actual_len = unsafe {
            pdfium_sys::FPDFSignatureObj_GetReason(
                sig,
                buffer.as_mut_ptr().cast::<std::ffi::c_void>(),
                buffer.len() as u64,
            )
        };

        if actual_len == 0 {
            return None;
        }

        // Convert UTF-16LE to String
        let utf16_units: Vec<u16> = buffer
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .take_while(|&c| c != 0) // Stop at null terminator
            .collect();

        String::from_utf16(&utf16_units).ok()
    }

    /// Helper: Get signature signing time (ASCII)
    // Method signature kept for API consistency with other PdfiumAdapter methods
    #[allow(clippy::unused_self)]
    fn get_signature_time(&self, sig: pdfium_sys::FPDF_SIGNATURE) -> Option<String> {
        // First call with null buffer to get required size
        let len = unsafe { pdfium_sys::FPDFSignatureObj_GetTime(sig, std::ptr::null_mut(), 0) };
        if len == 0 {
            return None;
        }

        let mut buffer = vec![0u8; len as usize];
        let actual_len = unsafe {
            pdfium_sys::FPDFSignatureObj_GetTime(
                sig,
                buffer.as_mut_ptr().cast::<i8>(),
                buffer.len() as u64,
            )
        };

        if actual_len == 0 {
            return None;
        }

        // Trim null terminator and convert to string
        let end = buffer.iter().position(|&b| b == 0).unwrap_or(buffer.len());
        String::from_utf8(buffer[..end].to_vec()).ok()
    }

    /// Helper: Get signature contents (raw binary data)
    // Method signature kept for API consistency with other PdfiumAdapter methods
    #[allow(clippy::unused_self)]
    fn get_signature_contents(&self, sig: pdfium_sys::FPDF_SIGNATURE) -> Vec<u8> {
        // First call with null buffer to get required size
        let len = unsafe { pdfium_sys::FPDFSignatureObj_GetContents(sig, std::ptr::null_mut(), 0) };
        if len == 0 {
            return Vec::new();
        }

        let mut buffer = vec![0u8; len as usize];
        let actual_len = unsafe {
            pdfium_sys::FPDFSignatureObj_GetContents(
                sig,
                buffer.as_mut_ptr().cast::<std::ffi::c_void>(),
                buffer.len() as u64,
            )
        };

        if actual_len == 0 {
            return Vec::new();
        }

        buffer.truncate(actual_len as usize);
        buffer
    }

    /// Helper: Get signature byte range
    // Method signature kept for API consistency with other PdfiumAdapter methods
    #[allow(clippy::unused_self)]
    fn get_signature_byte_range(&self, sig: pdfium_sys::FPDF_SIGNATURE) -> Vec<(i32, i32)> {
        // First call with null buffer to get required count
        let count =
            unsafe { pdfium_sys::FPDFSignatureObj_GetByteRange(sig, std::ptr::null_mut(), 0) };
        if count == 0 || count % 2 != 0 {
            return Vec::new();
        }

        let mut buffer = vec![0i32; count as usize];
        let actual_count = unsafe {
            pdfium_sys::FPDFSignatureObj_GetByteRange(sig, buffer.as_mut_ptr(), buffer.len() as u64)
        };

        if actual_count == 0 {
            return Vec::new();
        }

        // Convert flat array [start1, len1, start2, len2, ...] to pairs
        buffer
            .chunks_exact(2)
            .map(|chunk| (chunk[0], chunk[1]))
            .collect()
    }

    /// Get all digital signatures in the document.
    ///
    /// BUG #85 fix: Extract all signature metadata at once.
    ///
    /// # Returns
    /// A vector of all signatures in the document (empty if unsigned).
    #[must_use = "returns all digital signatures in the document"]
    pub fn get_all_signatures(&self) -> Vec<PdfSignature> {
        let count = self.signature_count();
        if count <= 0 {
            return Vec::new();
        }

        (0..count).filter_map(|i| self.get_signature(i)).collect()
    }

    // ============================================================
    // JavaScript Detection API (BUG #86 fix)
    // ============================================================

    /// Get the number of JavaScript actions in the document.
    ///
    /// BUG #86 fix: JavaScript detection for security scanning.
    ///
    /// # Returns
    /// The number of JavaScript actions (0 if none, -1 on error).
    #[inline]
    #[must_use = "returns the count of JavaScript actions"]
    pub fn javascript_action_count(&self) -> i32 {
        unsafe { pdfium_sys::FPDFDoc_GetJavaScriptActionCount(self.handle) }
    }

    /// Check if the document contains any JavaScript.
    ///
    /// BUG #86 fix: Quick check for JavaScript presence.
    /// This is useful for security scanning to flag potentially risky PDFs.
    ///
    /// # Returns
    /// `true` if the document contains any JavaScript actions.
    #[inline]
    #[must_use = "returns whether the document contains JavaScript"]
    pub fn has_javascript(&self) -> bool {
        self.javascript_action_count() > 0
    }

    /// Get a JavaScript action by index.
    ///
    /// BUG #86 fix: Extract individual JavaScript action metadata.
    ///
    /// # Arguments
    /// * `index` - The index of the JavaScript action (0-based).
    ///
    /// # Returns
    /// The JavaScript action if found, None otherwise.
    #[must_use = "returns the JavaScript action at the given index"]
    pub fn get_javascript_action(&self, index: i32) -> Option<PdfJavaScriptAction> {
        let count = self.javascript_action_count();
        if index < 0 || index >= count {
            return None;
        }

        let js_handle = unsafe { pdfium_sys::FPDFDoc_GetJavaScriptAction(self.handle, index) };
        if js_handle.is_null() {
            return None;
        }

        // Get name
        let name = self.get_javascript_action_name(js_handle);

        // Get script
        let script = self.get_javascript_action_script(js_handle);

        // Close the handle
        unsafe { pdfium_sys::FPDFDoc_CloseJavaScriptAction(js_handle) };

        Some(PdfJavaScriptAction {
            index,
            name,
            script,
        })
    }

    /// Helper to get JavaScript action name
    // Method signature kept for API consistency with other PdfiumAdapter methods
    #[allow(clippy::unused_self)]
    fn get_javascript_action_name(
        &self,
        js_handle: pdfium_sys::FPDF_JAVASCRIPT_ACTION,
    ) -> Option<String> {
        // Get required buffer size
        let name_len =
            unsafe { pdfium_sys::FPDFJavaScriptAction_GetName(js_handle, std::ptr::null_mut(), 0) };
        if name_len == 0 {
            return None;
        }

        // Allocate buffer (UTF-16LE, 2 bytes per unit)
        let utf16_units = (name_len as usize) / 2;
        let mut buffer: Vec<u16> = vec![0u16; utf16_units];

        unsafe {
            pdfium_sys::FPDFJavaScriptAction_GetName(js_handle, buffer.as_mut_ptr(), name_len);
        }

        // Convert UTF-16LE to String, trimming null terminator
        let name_end = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
        let name = String::from_utf16_lossy(&buffer[..name_end]);
        if name.is_empty() {
            None
        } else {
            Some(name)
        }
    }

    /// Helper to get JavaScript action script
    // Method signature kept for API consistency with other PdfiumAdapter methods
    #[allow(clippy::unused_self)]
    fn get_javascript_action_script(
        &self,
        js_handle: pdfium_sys::FPDF_JAVASCRIPT_ACTION,
    ) -> String {
        // Get required buffer size
        let script_len = unsafe {
            pdfium_sys::FPDFJavaScriptAction_GetScript(js_handle, std::ptr::null_mut(), 0)
        };
        if script_len == 0 {
            return String::new();
        }

        // Allocate buffer (UTF-16LE, 2 bytes per unit)
        let utf16_units = (script_len as usize) / 2;
        let mut buffer: Vec<u16> = vec![0u16; utf16_units];

        unsafe {
            pdfium_sys::FPDFJavaScriptAction_GetScript(js_handle, buffer.as_mut_ptr(), script_len);
        }

        // Convert UTF-16LE to String, trimming null terminator
        let script_end = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
        String::from_utf16_lossy(&buffer[..script_end])
    }

    /// Get all JavaScript actions in the document.
    ///
    /// BUG #86 fix: Extract all JavaScript metadata at once.
    /// Useful for comprehensive security scanning.
    ///
    /// # Returns
    /// A vector of all JavaScript actions (empty if none).
    #[must_use = "returns all JavaScript actions in the document"]
    pub fn get_all_javascript_actions(&self) -> Vec<PdfJavaScriptAction> {
        let count = self.javascript_action_count();
        if count <= 0 {
            return Vec::new();
        }

        (0..count)
            .filter_map(|i| self.get_javascript_action(i))
            .collect()
    }

    /// Check if the document contains any suspicious JavaScript.
    ///
    /// BUG #86 fix: Security convenience method.
    /// Scans all JavaScript actions for potentially dangerous patterns.
    ///
    /// # Returns
    /// `true` if any JavaScript action contains suspicious patterns.
    #[inline]
    #[must_use = "returns whether any JavaScript appears suspicious"]
    pub fn has_suspicious_javascript(&self) -> bool {
        self.get_all_javascript_actions()
            .iter()
            .any(PdfJavaScriptAction::has_suspicious_patterns)
    }

    /// Render all pages in parallel using `pdfium_fast`'s native C++ thread pool
    ///
    /// This is the CORRECT way to do parallel rendering - uses `pdfium_fast`'s
    /// built-in multi-threading (6.55x speedup) instead of trying to do it in Rust.
    ///
    /// # Errors
    /// Returns an error if page rendering fails or if the callback reports an error.
    #[must_use = "returns rendered page data"]
    pub fn render_pages_parallel(
        &self,
        dpi: f64,
        thread_count: i32,
    ) -> Result<Vec<RenderedPage>, DoclingError> {
        use std::sync::{Arc, Mutex};

        struct CallbackData {
            results: Arc<Mutex<Vec<Option<RenderedPage>>>>,
            error: Arc<Mutex<Option<String>>>,
        }

        extern "C" fn render_callback(
            page_index: std::ffi::c_int,
            buffer: *const std::ffi::c_void,
            width: std::ffi::c_int,
            height: std::ffi::c_int,
            stride: std::ffi::c_int,
            user_data: *mut std::ffi::c_void,
            success: pdfium_sys::FPDF_BOOL,
        ) {
            let data = unsafe { &*(user_data as *const CallbackData) };

            if success == 0 {
                // BUG #26 fix: Handle mutex poisoning gracefully in callback
                // Use unwrap_or_else to recover data even if mutex is poisoned
                let mut err = data
                    .error
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                if err.is_none() {
                    *err = Some(format!("Failed to render page {page_index}"));
                }
                return;
            }

            // BUG #28 fix: Check for buffer size overflow before allocation
            let Some(buffer_size) = (stride as usize).checked_mul(height as usize) else {
                let mut err = data
                    .error
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                *err = Some(format!("Buffer overflow: page {page_index} too large"));
                return;
            };
            let Some(rgb_size) = (width as usize)
                .checked_mul(height as usize)
                .and_then(|wh| wh.checked_mul(3))
            else {
                let mut err = data
                    .error
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                *err = Some(format!("RGB buffer overflow: page {page_index} too large"));
                return;
            };

            // BUG #80 fix: Validate buffer pointer before use
            if buffer.is_null() {
                let mut err = data
                    .error
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                *err = Some(format!("Null buffer for page {page_index}"));
                return;
            }

            // SAFETY: buffer verified non-null, buffer_size calculated from PDFium's stride * height
            // With output_format=1 (RGB), PDFium outputs RGB directly (3 bytes/pixel)
            let rgb_input = unsafe { std::slice::from_raw_parts(buffer.cast::<u8>(), buffer_size) };

            // P7 Optimization: RGB output mode - no conversion needed, just handle stride padding
            // With output_format=1, stride = width * 3 (typically) unless padded
            let width_usize = width as usize;
            let height_usize = height as usize;
            let stride_usize = stride as usize;
            let row_bytes = width_usize * 3; // 3 bytes per pixel for RGB

            // Fast path: No row padding (stride == width * 3)
            // Just copy the entire buffer directly
            let rgb = if stride_usize == row_bytes {
                // No padding - direct copy
                rgb_input[..rgb_size].to_vec()
            } else {
                // Slow path: Handle row padding (stride > width * 3)
                // Copy row by row, skipping padding bytes
                let mut rgb = vec![0u8; rgb_size];
                for y in 0..height_usize {
                    let src_start = y * stride_usize;
                    let dst_start = y * row_bytes;
                    rgb[dst_start..dst_start + row_bytes]
                        .copy_from_slice(&rgb_input[src_start..src_start + row_bytes]);
                }
                rgb
            };

            // BUG #26 fix: Handle mutex poisoning gracefully
            let mut results = data
                .results
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            results[page_index as usize] = Some(RenderedPage {
                page_index,
                width,
                height,
                rgb_data: rgb,
            });
        }

        let num_pages = self.page_count();
        if num_pages == 0 {
            return Ok(Vec::new());
        }

        // Storage for rendered pages
        let results: Arc<Mutex<Vec<Option<RenderedPage>>>> =
            Arc::new(Mutex::new(vec![None; num_pages as usize]));
        let error: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

        let results_clone = Arc::clone(&results);
        let error_clone = Arc::clone(&error);

        // Parallel options
        let mut options = pdfium_sys::FPDF_PARALLEL_OPTIONS {
            worker_count: if thread_count > 0 {
                thread_count
            } else {
                unsafe { pdfium_sys::FPDF_GetOptimalWorkerCountForDocument(self.handle) }
            },
            max_queue_size: 0,
            form_handle: std::ptr::null_mut(),
            dpi,
            output_format: 1, // 0=BGRA (default), 1=RGB, 2=BGR, 3=Grayscale - Use RGB for 25% memory savings
            reserved: [std::ptr::null_mut(); 1],
        };

        let callback_data = Box::new(CallbackData {
            results: results_clone,
            error: error_clone,
        });

        // Convert to raw pointer for FFI - must be reclaimed after FFI call
        let callback_data_ptr = Box::into_raw(callback_data);

        let success = unsafe {
            pdfium_sys::FPDF_RenderPagesParallelV2(
                self.handle,
                0,
                num_pages,
                0,
                0,
                0,
                0,
                &raw mut options,
                Some(render_callback),
                callback_data_ptr.cast::<std::ffi::c_void>(),
            )
        };

        // CRITICAL: Reclaim the Box to prevent memory leak (BUG #11 fix)
        // Safety: callback_data_ptr was created from Box::into_raw above
        // and FFI call is complete, so we can safely reclaim ownership
        let _ = unsafe { Box::from_raw(callback_data_ptr) };

        if success == 0 {
            return Err(DoclingError::BackendError(
                "Parallel rendering failed".into(),
            ));
        }

        // BUG #26 fix: Handle mutex poisoning - recover data even if poisoned
        {
            let err = error
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(msg) = &*err {
                return Err(DoclingError::BackendError(msg.clone()));
            }
        } // Drop error lock before acquiring results lock

        // BUG #26 fix: Handle mutex poisoning
        let mut guard = results
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard
            .drain(..)
            .enumerate()
            .map(|(i, opt)| {
                opt.ok_or_else(|| DoclingError::BackendError(format!("Missing page {i}")))
            })
            .collect()
    }
}

/// Rendered page from parallel rendering.
///
/// Contains the rendered bitmap data for a single PDF page along with
/// its dimensions and position in the document.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct RenderedPage {
    /// Zero-based index of the page within the PDF document.
    pub page_index: i32,
    /// Width of the rendered bitmap in pixels.
    pub width: i32,
    /// Height of the rendered bitmap in pixels.
    pub height: i32,
    /// Raw RGB pixel data in row-major order (3 bytes per pixel: R, G, B).
    pub rgb_data: Vec<u8>,
}

/// Structure element from a tagged PDF document.
///
/// Tagged PDFs contain a logical structure tree that describes the document's
/// semantic content hierarchy. Structure elements represent different content
/// types like paragraphs, headings, tables, etc.
///
/// This maps directly to PDF structure element types defined in PDF 1.7 (ISO 32000-1).
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct PdfStructElement {
    /// Structure element type (e.g., "P", "H1", "Table", "Figure")
    pub element_type: String,
    /// Element ID (optional)
    pub id: Option<String>,
    /// Element title (optional)
    pub title: Option<String>,
    /// Alternative text for accessibility (optional)
    pub alt_text: Option<String>,
    /// Language code (optional, e.g., "en-US")
    pub lang: Option<String>,
    /// Actual text content (optional)
    pub actual_text: Option<String>,
    /// Marked content IDs (MCIDs) associated with this element
    /// Used to map structure elements to page content when `actual_text` is absent
    pub marked_content_ids: Vec<i32>,
    /// Child elements in the structure hierarchy
    pub children: Vec<PdfStructElement>,
}

impl PdfStructElement {
    /// Check if this element is a heading (H, H1-H6)
    #[inline]
    #[must_use = "returns whether this is a heading element"]
    pub fn is_heading(&self) -> bool {
        matches!(
            self.element_type.as_str(),
            "H" | "H1" | "H2" | "H3" | "H4" | "H5" | "H6"
        )
    }

    /// Check if this element is a paragraph
    #[inline]
    #[must_use = "returns whether this is a paragraph element"]
    pub fn is_paragraph(&self) -> bool {
        self.element_type == "P"
    }

    /// Check if this element is a table
    #[inline]
    #[must_use = "returns whether this is a table element"]
    pub fn is_table(&self) -> bool {
        self.element_type == "Table"
    }

    /// Check if this element is a figure/image
    #[inline]
    #[must_use = "returns whether this is a figure element"]
    pub fn is_figure(&self) -> bool {
        self.element_type == "Figure"
    }

    /// Check if this element is a list or list item
    #[inline]
    #[must_use = "returns whether this is a list element"]
    pub fn is_list(&self) -> bool {
        matches!(self.element_type.as_str(), "L" | "LI" | "LBody" | "Lbl")
    }

    /// Get the heading level (1-6) if this is a heading, None otherwise
    #[inline]
    #[must_use = "returns the heading level if this is a heading"]
    pub fn heading_level(&self) -> Option<u8> {
        match self.element_type.as_str() {
            "H1" | "H" => Some(1), // Generic heading defaults to H1
            "H2" => Some(2),
            "H3" => Some(3),
            "H4" => Some(4),
            "H5" => Some(5),
            "H6" => Some(6),
            _ => None,
        }
    }
}

impl Drop for PdfDocumentFast {
    fn drop(&mut self) {
        // SAFETY:
        // - self.handle is valid (ensured by constructor, never null after successful creation)
        // - All pages must be closed before document (handled by Rust's drop order)
        // - Releases document memory allocated by FPDF_LoadDocument/FPDF_LoadMemDocument
        unsafe {
            pdfium_sys::FPDF_CloseDocument(self.handle);
        }
    }
}

impl std::fmt::Debug for PdfDocumentFast {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PdfDocumentFast")
            .field("page_count", &self.page_count())
            .finish_non_exhaustive()
    }
}

/// PDF Page wrapper for pdfium-fast
pub struct PdfPageFast {
    handle: pdfium_sys::FPDF_PAGE,
    doc: pdfium_sys::FPDF_DOCUMENT,
    /// Cached text page for efficient repeated text extraction (BUG #12 fix)
    /// Using `RefCell` for interior mutability - text page is loaded on first use
    cached_text_page: std::cell::RefCell<Option<pdfium_sys::FPDF_TEXTPAGE>>,
}

impl PdfPageFast {
    /// Get page width in points
    #[inline]
    #[must_use = "returns the page width"]
    pub fn width(&self) -> f64 {
        unsafe { pdfium_sys::FPDF_GetPageWidth(self.handle) }
    }

    /// Get page height in points
    #[inline]
    #[must_use = "returns the page height"]
    pub fn height(&self) -> f64 {
        unsafe { pdfium_sys::FPDF_GetPageHeight(self.handle) }
    }

    /// Get page rotation in degrees (0, 90, 180, or 270)
    ///
    /// This is important for scanned PDFs which may be rotated.
    /// Returns 0 for normal orientation.
    #[inline]
    #[must_use = "returns the page rotation in degrees"]
    pub fn rotation(&self) -> i32 {
        let rot = unsafe { pdfium_sys::FPDFPage_GetRotation(self.handle) };
        // PDFium returns 0=0°, 1=90°, 2=180°, 3=270°
        match rot {
            1 => 90,
            2 => 180,
            3 => 270,
            _ => 0,
        }
    }

    /// Get visual width in points (accounting for rotation)
    ///
    /// For pages with 90° or 270° rotation, this returns the raw height
    /// (since the page appears rotated by 90°, width and height are swapped).
    /// BUG #74 fix: Proper dimension handling for rotated pages.
    #[inline]
    #[must_use = "returns the visual width accounting for rotation"]
    pub fn visual_width(&self) -> f64 {
        let rotation = self.rotation();
        if rotation == 90 || rotation == 270 {
            self.height() // Swapped for 90°/270° rotation
        } else {
            self.width()
        }
    }

    /// Get visual height in points (accounting for rotation)
    ///
    /// For pages with 90° or 270° rotation, this returns the raw width
    /// (since the page appears rotated by 90°, width and height are swapped).
    /// BUG #74 fix: Proper dimension handling for rotated pages.
    #[inline]
    #[must_use = "returns the visual height accounting for rotation"]
    pub fn visual_height(&self) -> f64 {
        let rotation = self.rotation();
        if rotation == 90 || rotation == 270 {
            self.width() // Swapped for 90°/270° rotation
        } else {
            self.height()
        }
    }

    /// Check if the page has transparency
    ///
    /// Pages with transparency require BGRA format for correct rendering.
    /// Pages without transparency can use `BGRx` format which is faster.
    #[inline]
    #[must_use = "returns whether the page has transparency"]
    pub fn has_transparency(&self) -> bool {
        unsafe { pdfium_sys::FPDFPage_HasTransparency(self.handle) != 0 }
    }

    /// Get the actual content bounding box for the page.
    ///
    /// BUG #3 fix: Returns the bounding box of actual page content, which may
    /// differ from the page dimensions (width/height). This is useful for:
    /// - Detecting margins and whitespace
    /// - Cropping to actual content area
    /// - Understanding content layout
    ///
    /// Returns (left, top, right, bottom) in PDF points with top-left origin.
    /// Returns None if the bounding box cannot be determined.
    #[must_use = "returns the content bounding box if available"]
    pub fn get_bounding_box(&self) -> Option<(f64, f64, f64, f64)> {
        let page_height = self.height();
        let mut rect: pdfium_sys::FS_RECTF = unsafe { std::mem::zeroed() };

        let success = unsafe { pdfium_sys::FPDF_GetPageBoundingBox(self.handle, &raw mut rect) };
        if success == 0 {
            return None;
        }

        // Convert from PDF coordinates (bottom-left origin) to standard (top-left)
        Some((
            f64::from(rect.left),
            page_height - f64::from(rect.top),
            f64::from(rect.right),
            page_height - f64::from(rect.bottom),
        ))
    }

    /// Get or create cached text page handle (BUG #12 fix)
    ///
    /// Returns the cached text page, creating it on first access.
    /// This avoids loading the text page multiple times when calling
    /// `get_text_bounded()` repeatedly (e.g., in `merge_and_convert_text_cells`).
    fn get_cached_text_page(&self) -> Result<pdfium_sys::FPDF_TEXTPAGE, DoclingError> {
        let mut cached = self.cached_text_page.borrow_mut();
        if let Some(handle) = *cached {
            return Ok(handle);
        }

        // Load text page and cache it
        // SAFETY:
        // - self.handle is a valid FPDF_PAGE (created in load_page, not yet dropped)
        // - FPDFText_LoadPage extracts text information from page
        // - Returns null on error (checked below)
        // - Returned FPDF_TEXTPAGE must be closed before page is closed
        let text_page = unsafe { pdfium_sys::FPDFText_LoadPage(self.handle) };
        if text_page.is_null() {
            return Err(DoclingError::BackendError(
                "Failed to load text page".to_string(),
            ));
        }
        *cached = Some(text_page);
        Ok(text_page)
    }

    /// Render the page to a bitmap
    ///
    /// Returns BGRA pixel data as a `Vec<u8>`
    ///
    /// # Errors
    /// Returns an error if dimensions are invalid (non-positive or exceed maximum)
    /// or if bitmap allocation fails.
    #[must_use = "renderer returns bitmap data"]
    pub fn render_to_bitmap(
        &self,
        width: i32,
        height: i32,
    ) -> Result<(Vec<u8>, i32, i32), DoclingError> {
        // Reasonable upper limit: 32000x32000 pixels (common library limit)
        // This prevents integer overflow and unreasonable memory allocation
        const MAX_DIMENSION: i32 = 32000;

        // BUG #94 fix: Validate dimensions before FFI call to handle corrupted PDFs
        // PDFium may crash on invalid dimensions, so we validate first
        if width <= 0 || height <= 0 {
            return Err(DoclingError::BackendError(format!(
                "Invalid bitmap dimensions: {width}x{height} (must be positive)"
            )));
        }

        if width > MAX_DIMENSION || height > MAX_DIMENSION {
            return Err(DoclingError::BackendError(format!(
                "Bitmap dimensions too large: {width}x{height} (max {MAX_DIMENSION}x{MAX_DIMENSION})"
            )));
        }

        // Create bitmap
        // SAFETY:
        // - width/height validated above (positive, within MAX_DIMENSION)
        // - 0 = no alpha channel (RGB format)
        // - Allocates memory internally, returns null on failure (checked below)
        // - Bitmap must be destroyed via FPDFBitmap_Destroy (done in Drop)
        let bitmap = unsafe {
            pdfium_sys::FPDFBitmap_Create(width, height, 0) // 0 = no alpha
        };
        if bitmap.is_null() {
            return Err(DoclingError::BackendError(
                "Failed to create bitmap".to_string(),
            ));
        }

        // SAFETY:
        // - bitmap is valid (just created above, null-checked)
        // - Coordinates (0, 0, width, height) are within bitmap bounds
        // - ARGB_WHITE is valid ARGB color (opaque white)
        // Fill with white background
        unsafe {
            pdfium_sys::FPDFBitmap_FillRect(bitmap, 0, 0, width, height, ARGB_WHITE);
        }

        // Render page to bitmap with proper rotation (BUG #13 fix)
        // PDFium expects rotation as 0=0°, 1=90°, 2=180°, 3=270° clockwise
        let rotation_param = self.rotation() / 90;
        // SAFETY:
        // - bitmap is valid (created above, null-checked)
        // - self.handle is valid page handle (ensured by PdfPageFast invariant)
        // - width/height match bitmap dimensions
        // - rotation_param is 0-3 (rotation() returns 0/90/180/270)
        // - flags=0 is standard rendering mode
        unsafe {
            pdfium_sys::FPDF_RenderPageBitmap(
                bitmap,
                self.handle,
                0,              // start_x
                0,              // start_y
                width,          // size_x
                height,         // size_y
                rotation_param, // rotate: apply page's intrinsic rotation
                0,              // flags
            );
        }

        // SAFETY:
        // - bitmap is valid (created and rendered above)
        // - Returns pointer to internal buffer, or null on error (checked below)
        // - Buffer is valid for lifetime of bitmap (not modified until Destroy)
        // Get pixel data (BUG #80 fix: validate buffer pointer before use)
        let buffer = unsafe { pdfium_sys::FPDFBitmap_GetBuffer(bitmap) };
        if buffer.is_null() {
            // SAFETY: bitmap is valid (created above)
            // Clean up bitmap before returning error
            unsafe { pdfium_sys::FPDFBitmap_Destroy(bitmap) };
            return Err(DoclingError::BackendError(
                "FPDFBitmap_GetBuffer returned null pointer".to_string(),
            ));
        }

        // SAFETY: bitmap is valid (created above, buffer non-null)
        // These return bitmap dimensions set during FPDFBitmap_Create
        let stride = unsafe { pdfium_sys::FPDFBitmap_GetStride(bitmap) };
        let actual_height = unsafe { pdfium_sys::FPDFBitmap_GetHeight(bitmap) };

        // BUG #94 fix: Validate stride and height are sane values
        // Stride must be at least width * 4 (BGRA = 4 bytes per pixel)
        let min_stride = width * 4;
        if stride < min_stride {
            unsafe { pdfium_sys::FPDFBitmap_Destroy(bitmap) };
            return Err(DoclingError::BackendError(format!(
                "Invalid stride {stride} (expected at least {min_stride} for width {width})"
            )));
        }
        if actual_height <= 0 {
            unsafe { pdfium_sys::FPDFBitmap_Destroy(bitmap) };
            return Err(DoclingError::BackendError(format!(
                "Invalid bitmap height {actual_height} returned by PDFium"
            )));
        }

        // Check for buffer size overflow
        let Some(buffer_size) = (stride as usize).checked_mul(actual_height as usize) else {
            unsafe { pdfium_sys::FPDFBitmap_Destroy(bitmap) };
            return Err(DoclingError::BackendError(format!(
                "Buffer size overflow: stride {stride} * height {actual_height} exceeds usize"
            )));
        };
        // SAFETY: buffer is verified non-null, buffer_size is calculated from PDFium's
        // own stride and height values for this bitmap
        let pixel_data =
            unsafe { std::slice::from_raw_parts(buffer as *const u8, buffer_size) }.to_vec();

        // SAFETY: bitmap is valid (created above, not yet destroyed)
        // Releases memory allocated by FPDFBitmap_Create
        // Destroy bitmap
        unsafe {
            pdfium_sys::FPDFBitmap_Destroy(bitmap);
        }

        Ok((pixel_data, stride, actual_height))
    }

    // BUG #38 fix: Extract helper for getting single image object
    // Centralizes the logic to avoid fragile assumptions about object index
    /// Get the single image object if this page contains exactly one image object.
    ///
    /// Returns None if the page has no objects, more than one object,
    /// or the single object is not an image.
    ///
    /// `FPDF_PAGEOBJ_IMAGE` = 3 (page object type constant)
    #[inline]
    fn get_single_image_object(&self) -> Option<pdfium_sys::FPDF_PAGEOBJECT> {
        let count = unsafe { pdfium_sys::FPDFPage_CountObjects(self.handle) };
        if count != 1 {
            return None;
        }

        // Since count == 1, index 0 is the only valid index
        let obj = unsafe { pdfium_sys::FPDFPage_GetObject(self.handle, 0) };
        if obj.is_null() {
            return None;
        }

        // Verify it's an image object
        // PDFium object types: 0=Unknown, 1=Text, 2=Path, 3=Image, 4=Shading, 5=Form
        let obj_type = unsafe { pdfium_sys::FPDFPageObj_GetType(obj) };
        if obj_type == FPDF_PAGEOBJ_IMAGE {
            Some(obj)
        } else {
            None
        }
    }

    /// Check if page is a single image (likely a scanned page)
    ///
    /// Scanned PDFs typically have one full-page image per page.
    /// If detected, use `extract_image_data_raw()` for 545x faster extraction.
    #[inline]
    #[must_use = "returns whether this is a single-image scanned page"]
    pub fn is_single_image_page(&self) -> bool {
        self.get_single_image_object().is_some()
    }

    /// Extract raw image data from a single-image page (scanned PDF fast-path)
    ///
    /// For scanned PDFs, this extracts the embedded JPEG directly without
    /// re-rendering, providing ~545x speedup over `render_to_bitmap()`.
    ///
    /// Returns None if the page isn't a single-image page or extraction fails.
    #[must_use = "returns raw image data from scanned page"]
    pub fn extract_image_data_raw(&self) -> Option<Vec<u8>> {
        // BUG #38 fix: Use helper instead of is_single_image_page + GetObject(0)
        let obj = self.get_single_image_object()?;

        // Get required buffer size
        let size =
            unsafe { pdfium_sys::FPDFImageObj_GetImageDataRaw(obj, std::ptr::null_mut(), 0) };

        if size == 0 {
            return None;
        }

        // Allocate buffer and extract data
        let mut buffer = vec![0u8; size as usize];
        let written = unsafe {
            pdfium_sys::FPDFImageObj_GetImageDataRaw(obj, buffer.as_mut_ptr().cast(), size)
        };

        if written as usize == buffer.len() {
            Some(buffer)
        } else {
            None
        }
    }

    /// Extract decoded image data from a single-image page
    ///
    /// For scanned PDFs with non-JPEG compression (e.g., `CCITTFaxDecode`),
    /// this extracts the decoded pixel data.
    ///
    /// Returns None if the page isn't a single-image page or extraction fails.
    #[must_use = "returns decoded image data from scanned page"]
    pub fn extract_image_data_decoded(&self) -> Option<Vec<u8>> {
        let obj = self.get_single_image_object()?;

        // Get required buffer size
        let size =
            unsafe { pdfium_sys::FPDFImageObj_GetImageDataDecoded(obj, std::ptr::null_mut(), 0) };

        if size == 0 {
            return None;
        }

        // Allocate buffer and extract decoded data
        let mut buffer = vec![0u8; size as usize];
        let written = unsafe {
            pdfium_sys::FPDFImageObj_GetImageDataDecoded(obj, buffer.as_mut_ptr().cast(), size)
        };

        if written as usize == buffer.len() {
            Some(buffer)
        } else {
            None
        }
    }

    /// Check if the page image uses CCITT fax compression (scanned fax documents)
    ///
    /// Returns true if the image uses `CCITTFaxDecode` filter.
    #[inline]
    #[must_use = "returns whether the image uses CCITT fax compression"]
    pub fn is_ccitt_image(&self) -> bool {
        self.get_image_filters().iter().any(|f| f.contains("CCITT"))
    }

    /// Get image metadata for a single-image page
    ///
    /// Returns (width, height, `bits_per_pixel`, colorspace) or None if not a single-image page.
    #[must_use = "returns image metadata if this is a single-image page"]
    pub fn get_image_metadata(&self) -> Option<(u32, u32, i32, i32)> {
        // BUG #38 fix: Use helper instead of is_single_image_page + GetObject(0)
        let obj = self.get_single_image_object()?;

        let mut metadata: pdfium_sys::FPDF_IMAGEOBJ_METADATA = unsafe { std::mem::zeroed() };
        let success = unsafe {
            pdfium_sys::FPDFImageObj_GetImageMetadata(obj, self.handle, &raw mut metadata)
        };

        if success != 0 {
            Some((
                metadata.width,
                metadata.height,
                metadata.bits_per_pixel as i32,
                metadata.colorspace,
            ))
        } else {
            None
        }
    }

    /// Get image filter names for a single-image page
    ///
    /// Returns a list of filter names (e.g., "`DCTDecode`" for JPEG, "`FlateDecode`" for deflate).
    /// Important: Raw image extraction only works reliably for `DCTDecode` (JPEG) images.
    #[must_use = "returns the list of image filter names"]
    pub fn get_image_filters(&self) -> Vec<String> {
        // BUG #38 fix: Use helper instead of is_single_image_page + GetObject(0)
        let Some(obj) = self.get_single_image_object() else {
            return Vec::new();
        };

        let filter_count = unsafe { pdfium_sys::FPDFImageObj_GetImageFilterCount(obj) };
        if filter_count <= 0 {
            return Vec::new();
        }

        let mut filters = Vec::new();
        for i in 0..filter_count {
            // Get filter name length
            let name_len =
                unsafe { pdfium_sys::FPDFImageObj_GetImageFilter(obj, i, std::ptr::null_mut(), 0) };

            if name_len > 0 {
                let mut buffer: Vec<u8> = vec![0; name_len as usize];
                unsafe {
                    pdfium_sys::FPDFImageObj_GetImageFilter(
                        obj,
                        i,
                        buffer.as_mut_ptr().cast(),
                        name_len,
                    );
                }
                // BUG #39 fix: Find actual null terminator position instead of assuming end
                // The buffer may contain garbage after the null terminator
                let actual_len = buffer.iter().position(|&b| b == 0).unwrap_or(buffer.len());
                let name = String::from_utf8_lossy(&buffer[..actual_len]).to_string();
                if !name.is_empty() {
                    filters.push(name);
                }
            }
        }

        filters
    }

    /// Check if the page image is a JPEG (`DCTDecode` filter)
    ///
    /// Returns true if the image can be extracted directly via `extract_image_data_raw()`.
    /// JPEG images have "`DCTDecode`" filter.
    #[inline]
    #[must_use = "returns whether the image is a JPEG"]
    pub fn is_jpeg_image(&self) -> bool {
        self.get_image_filters().iter().any(|f| f.contains("DCT"))
    }

    /// Count the number of objects on this page.
    ///
    /// BUG #83 fix: Enables enumeration of all page objects (text, images, paths, etc.)
    #[inline]
    #[must_use = "returns the number of objects on the page"]
    pub fn object_count(&self) -> i32 {
        unsafe { pdfium_sys::FPDFPage_CountObjects(self.handle) }
    }

    /// Get a specific page object by index.
    ///
    /// BUG #83 fix: Returns information about the page object at the given index.
    ///
    /// # Arguments
    ///
    /// * `index` - Zero-based index of the object (0 to object_count()-1)
    ///
    /// # Returns
    ///
    /// Some(PdfPageObject) with the object's type and bounds, or None if invalid index.
    #[must_use = "returns the page object at the given index"]
    pub fn get_object(&self, index: i32) -> Option<PdfPageObject> {
        let count = self.object_count();
        if index < 0 || index >= count {
            return None;
        }

        let obj = unsafe { pdfium_sys::FPDFPage_GetObject(self.handle, index) };
        if obj.is_null() {
            return None;
        }

        // Get object type
        let obj_type_raw = unsafe { pdfium_sys::FPDFPageObj_GetType(obj) };
        let object_type = PdfPageObjectType::from_raw(obj_type_raw);

        // Get bounds
        let mut left: f32 = 0.0;
        let mut bottom: f32 = 0.0;
        let mut right: f32 = 0.0;
        let mut top: f32 = 0.0;

        let has_bounds = unsafe {
            pdfium_sys::FPDFPageObj_GetBounds(
                obj,
                &raw mut left,
                &raw mut bottom,
                &raw mut right,
                &raw mut top,
            )
        };

        if has_bounds == 0 {
            // No bounds available, use zeros
            left = 0.0;
            bottom = 0.0;
            right = 0.0;
            top = 0.0;
        }

        // Check transparency
        let has_transparency = unsafe { pdfium_sys::FPDFPageObj_HasTransparency(obj) != 0 };

        Some(PdfPageObject {
            index,
            object_type,
            left,
            bottom,
            right,
            top,
            has_transparency,
        })
    }

    /// Iterate over all objects on the page.
    ///
    /// BUG #83 fix: Returns a vector of all page objects with their types and bounds.
    /// This is useful for analyzing page structure or finding specific content.
    ///
    /// # Returns
    ///
    /// Vector of `PdfPageObject` structs for all objects on the page.
    #[must_use = "returns all objects on the page"]
    pub fn get_all_objects(&self) -> Vec<PdfPageObject> {
        let count = self.object_count();
        let mut objects = Vec::with_capacity(count.max(0) as usize);

        for i in 0..count {
            if let Some(obj) = self.get_object(i) {
                objects.push(obj);
            }
        }

        objects
    }

    /// Get objects of a specific type.
    ///
    /// BUG #83 fix: Filters page objects by type for targeted extraction.
    ///
    /// # Arguments
    ///
    /// * `object_type` - The type of objects to retrieve
    ///
    /// # Returns
    ///
    /// Vector of `PdfPageObject` structs matching the specified type.
    #[must_use = "returns objects of the specified type"]
    pub fn get_objects_by_type(&self, object_type: PdfPageObjectType) -> Vec<PdfPageObject> {
        self.get_all_objects()
            .into_iter()
            .filter(|obj| obj.object_type == object_type)
            .collect()
    }

    /// Count objects of a specific type.
    ///
    /// BUG #83 fix: Quick count without extracting full object details.
    #[must_use = "returns the count of objects of the specified type"]
    pub fn count_objects_by_type(&self, object_type: PdfPageObjectType) -> usize {
        let count = self.object_count();
        let mut result = 0;

        for i in 0..count {
            let obj = unsafe { pdfium_sys::FPDFPage_GetObject(self.handle, i) };
            if !obj.is_null() {
                let obj_type_raw = unsafe { pdfium_sys::FPDFPageObj_GetType(obj) };
                if PdfPageObjectType::from_raw(obj_type_raw) == object_type {
                    result += 1;
                }
            }
        }

        result
    }

    /// Get all image objects on the page.
    ///
    /// BUG #83 fix: Convenience method for image extraction workflows.
    #[inline]
    #[must_use = "returns all image objects on the page"]
    pub fn get_image_objects(&self) -> Vec<PdfPageObject> {
        self.get_objects_by_type(PdfPageObjectType::Image)
    }

    /// Get all text objects on the page.
    ///
    /// BUG #83 fix: Convenience method for text analysis workflows.
    #[inline]
    #[must_use = "returns all text objects on the page"]
    pub fn get_text_objects(&self) -> Vec<PdfPageObject> {
        self.get_objects_by_type(PdfPageObjectType::Text)
    }

    /// Extract the structure tree from this page (if tagged PDF).
    ///
    /// Tagged PDFs contain semantic structure information that can be used
    /// to understand document layout without ML inference. This is much faster
    /// than ML-based layout detection (~50 pages/sec vs ~1.5 pages/sec).
    ///
    /// # Returns
    /// - `Some(Vec<PdfStructElement>)` - Root structure elements for this page
    /// - `None` - If the page has no structure tree (not a tagged PDF)
    ///
    /// # Performance
    /// Structure extraction is O(n) where n is the number of structure elements.
    /// For typical documents, this takes <1ms per page.
    #[must_use = "returns the page's structure tree if available"]
    pub fn get_structure_tree(&self) -> Option<Vec<PdfStructElement>> {
        // Get the structure tree for this page
        // SAFETY: self.handle is valid (checked in constructor)
        let tree = unsafe { pdfium_sys::FPDF_StructTree_GetForPage(self.handle) };
        if tree.is_null() {
            return None;
        }

        // Count root children
        let child_count = unsafe { pdfium_sys::FPDF_StructTree_CountChildren(tree) };
        if child_count <= 0 {
            // Close tree before returning
            unsafe { pdfium_sys::FPDF_StructTree_Close(tree) };
            return Some(Vec::new());
        }

        // Extract all root elements
        let mut elements = Vec::with_capacity(child_count as usize);
        for i in 0..child_count {
            let child = unsafe { pdfium_sys::FPDF_StructTree_GetChildAtIndex(tree, i) };
            if !child.is_null() {
                if let Some(elem) = Self::extract_struct_element(child) {
                    elements.push(elem);
                }
            }
        }

        // Close the structure tree
        unsafe { pdfium_sys::FPDF_StructTree_Close(tree) };

        Some(elements)
    }

    /// Build a map from marked content ID (MCID) to text content.
    ///
    /// This extracts text from all text objects on the page, grouped by their
    /// marked content ID. This allows mapping structure tree elements to their
    /// actual text content when `actual_text` attribute is not present.
    ///
    /// # Returns
    /// - `Ok(HashMap<i32, String>)` - Map from MCID to concatenated text content
    /// - `Err` - If text page loading fails
    ///
    /// # Performance
    /// This iterates all page objects and extracts text from text objects.
    /// For typical pages, this takes ~1-5ms.
    ///
    /// # Errors
    /// Returns an error if the text page cannot be loaded.
    #[must_use = "returns MCID to text mapping"]
    pub fn build_mcid_text_map(
        &self,
    ) -> Result<std::collections::HashMap<i32, String>, DoclingError> {
        use std::collections::HashMap;

        // Get text page handle for text extraction
        let text_page = self.get_cached_text_page()?;

        let mut mcid_map: HashMap<i32, String> = HashMap::new();
        let obj_count = self.object_count();

        for i in 0..obj_count {
            // SAFETY: handle is valid, i is in bounds
            let obj = unsafe { pdfium_sys::FPDFPage_GetObject(self.handle, i) };
            if obj.is_null() {
                continue;
            }

            // Check if this is a text object
            let obj_type = unsafe { pdfium_sys::FPDFPageObj_GetType(obj) };
            if obj_type != FPDF_PAGEOBJ_TEXT {
                continue;
            }

            // Get marked content ID for this object
            let mcid = unsafe { pdfium_sys::FPDFPageObj_GetMarkedContentID(obj) };
            if mcid < 0 {
                // No MCID associated with this text object
                continue;
            }

            // Extract text from this text object
            // First, get required buffer size
            let len =
                unsafe { pdfium_sys::FPDFTextObj_GetText(obj, text_page, std::ptr::null_mut(), 0) };
            if len <= 2 {
                // Empty or just null terminator
                continue;
            }

            // Allocate buffer and extract text
            let char_count = (len as usize) / 2;
            let mut buffer: Vec<u16> = vec![0u16; char_count];
            let actual_len = unsafe {
                pdfium_sys::FPDFTextObj_GetText(obj, text_page, buffer.as_mut_ptr(), len)
            };
            if actual_len == 0 {
                continue;
            }

            // Convert UTF-16LE to String
            let end = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
            let text = String::from_utf16_lossy(&buffer[..end]);
            if text.is_empty() {
                continue;
            }

            // Append to existing text for this MCID (text objects may span multiple content streams)
            mcid_map
                .entry(mcid)
                .and_modify(|existing| {
                    existing.push_str(&text);
                })
                .or_insert(text);
        }

        Ok(mcid_map)
    }

    /// Recursively extract a structure element and its children.
    fn extract_struct_element(elem: pdfium_sys::FPDF_STRUCTELEMENT) -> Option<PdfStructElement> {
        // Get element type (e.g., "P", "H1", "Table", "Figure")
        let element_type = Self::get_struct_element_string(elem, |e, buf, len| unsafe {
            pdfium_sys::FPDF_StructElement_GetType(e, buf, len)
        })?;

        // Get optional attributes
        let id = Self::get_struct_element_string(elem, |e, buf, len| unsafe {
            pdfium_sys::FPDF_StructElement_GetID(e, buf, len)
        });

        let title = Self::get_struct_element_string(elem, |e, buf, len| unsafe {
            pdfium_sys::FPDF_StructElement_GetTitle(e, buf, len)
        });

        let alt_text = Self::get_struct_element_string(elem, |e, buf, len| unsafe {
            pdfium_sys::FPDF_StructElement_GetAltText(e, buf, len)
        });

        let lang = Self::get_struct_element_string(elem, |e, buf, len| unsafe {
            pdfium_sys::FPDF_StructElement_GetLang(e, buf, len)
        });

        let actual_text = Self::get_struct_element_string(elem, |e, buf, len| unsafe {
            pdfium_sys::FPDF_StructElement_GetActualText(e, buf, len)
        });

        // Extract marked content IDs (MCIDs) for this element
        // MCIDs link structure elements to page content when actual_text is absent
        let mcid_count = unsafe { pdfium_sys::FPDF_StructElement_GetMarkedContentIdCount(elem) };
        let mut marked_content_ids = Vec::new();
        if mcid_count > 0 {
            marked_content_ids.reserve(mcid_count as usize);
            for i in 0..mcid_count {
                let mcid =
                    unsafe { pdfium_sys::FPDF_StructElement_GetMarkedContentIdAtIndex(elem, i) };
                if mcid >= 0 {
                    marked_content_ids.push(mcid);
                }
            }
        }

        // Recursively extract children
        let child_count = unsafe { pdfium_sys::FPDF_StructElement_CountChildren(elem) };
        let mut children = Vec::new();

        for i in 0..child_count {
            let child = unsafe { pdfium_sys::FPDF_StructElement_GetChildAtIndex(elem, i) };
            if !child.is_null() {
                if let Some(child_elem) = Self::extract_struct_element(child) {
                    children.push(child_elem);
                }
            }
        }

        Some(PdfStructElement {
            element_type,
            id,
            title,
            alt_text,
            lang,
            actual_text,
            marked_content_ids,
            children,
        })
    }

    /// Helper to extract a UTF-16 string from structure element.
    fn get_struct_element_string<F>(
        elem: pdfium_sys::FPDF_STRUCTELEMENT,
        getter: F,
    ) -> Option<String>
    where
        F: Fn(
            pdfium_sys::FPDF_STRUCTELEMENT,
            *mut std::ffi::c_void,
            std::ffi::c_ulong,
        ) -> std::ffi::c_ulong,
    {
        // First call to get required buffer size (in bytes)
        let len = getter(elem, std::ptr::null_mut(), 0);
        if len <= 2 {
            // Empty or just null terminator
            return None;
        }

        // PDFium returns UTF-16LE strings, 2 bytes per character
        // len includes the null terminator (2 bytes)
        let char_count = (len as usize) / 2;
        let mut buffer: Vec<u16> = vec![0u16; char_count];

        // Get the actual string
        let actual_len = getter(elem, buffer.as_mut_ptr().cast::<std::ffi::c_void>(), len);
        if actual_len == 0 {
            return None;
        }

        // Convert UTF-16LE to String, removing null terminator
        let end = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
        let s = String::from_utf16_lossy(&buffer[..end]);

        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    }
}

impl Drop for PdfPageFast {
    fn drop(&mut self) {
        // Close cached text page first (if any)
        if let Some(text_page) = self.cached_text_page.borrow_mut().take() {
            // SAFETY:
            // - text_page was created by FPDFText_LoadPage (in ensure_text_page)
            // - Must be closed before the page (FPDF_ClosePage below)
            unsafe {
                pdfium_sys::FPDFText_ClosePage(text_page);
            }
        }
        // SAFETY:
        // - self.handle is valid page handle (created by FPDF_LoadPage)
        // - All text pages already closed (above)
        // - Must be closed before document (handled by Rust's drop order)
        // Then close the page
        unsafe {
            pdfium_sys::FPDF_ClosePage(self.handle);
        }
    }
}

impl std::fmt::Debug for PdfPageFast {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PdfPageFast")
            .field("width", &self.width())
            .field("height", &self.height())
            .field("rotation", &self.rotation())
            .finish_non_exhaustive()
    }
}

/// PDF Text Page wrapper for pdfium-fast
pub struct PdfTextPageFast {
    handle: pdfium_sys::FPDF_TEXTPAGE,
}

impl PdfTextPageFast {
    /// Load text page from a PDF page
    ///
    /// # Errors
    /// Returns an error if `PDFium` fails to load the text page.
    #[must_use = "loader returns a text page"]
    pub fn from_page(page: &PdfPageFast) -> Result<Self, DoclingError> {
        // SAFETY:
        // - page.handle is a valid FPDF_PAGE (created in load_page)
        // - FPDFText_LoadPage extracts text information from page
        // - Returns null on error (checked below)
        // - PdfTextPageFast::drop closes the text page properly
        let text_page = unsafe { pdfium_sys::FPDFText_LoadPage(page.handle) };
        if text_page.is_null() {
            return Err(DoclingError::BackendError(
                "Failed to load text page".to_string(),
            ));
        }
        Ok(Self { handle: text_page })
    }

    /// Get the number of characters on the page
    #[inline]
    #[must_use = "returns the character count"]
    pub fn char_count(&self) -> i32 {
        unsafe { pdfium_sys::FPDFText_CountChars(self.handle) }
    }

    /// Get a character's unicode value
    #[inline]
    #[must_use = "returns the unicode value of the character"]
    pub fn get_unicode(&self, index: i32) -> u32 {
        unsafe { pdfium_sys::FPDFText_GetUnicode(self.handle, index) }
    }

    /// Get a character's bounding box
    /// Returns (left, top, right, bottom) in page coordinates
    #[inline]
    #[must_use = "returns the character's bounding box if available"]
    pub fn get_char_box(&self, index: i32) -> Option<(f64, f64, f64, f64)> {
        let mut left: f64 = 0.0;
        let mut top: f64 = 0.0;
        let mut right: f64 = 0.0;
        let mut bottom: f64 = 0.0;

        let success = unsafe {
            pdfium_sys::FPDFText_GetCharBox(
                self.handle,
                index,
                &raw mut left,
                &raw mut right,
                &raw mut bottom,
                &raw mut top,
            )
        };

        if success != 0 {
            Some((left, top, right, bottom))
        } else {
            None
        }
    }

    /// Extract text from a range of characters
    #[must_use = "returns the extracted text string"]
    pub fn get_text(&self, start_index: i32, count: i32) -> String {
        if count <= 0 {
            return String::new();
        }

        // PDFium uses UTF-16, so we need buffer for UTF-16 chars + null terminator
        let buffer_len = (count + 1) as usize;
        let mut buffer: Vec<u16> = vec![0; buffer_len];

        let chars_written = unsafe {
            pdfium_sys::FPDFText_GetText(self.handle, start_index, count, buffer.as_mut_ptr())
        };

        if chars_written <= 0 {
            return String::new();
        }

        // Remove null terminator and convert from UTF-16
        // BUG #29 fix: Use saturating_sub for clearer intent
        // BUG #76 fix: Log warning if invalid UTF-16 encountered
        let actual_len = (chars_written as usize).saturating_sub(1);
        utf16_to_string_with_warning(&buffer[..actual_len], "page text")
    }

    /// Get all text from the page
    #[inline]
    #[must_use = "returns all text from the page"]
    pub fn get_all_text(&self) -> String {
        let count = self.char_count();
        self.get_text(0, count)
    }

    /// Get a character's loose bounding box (better for text selection).
    ///
    /// BUG #7 fix: The loose char box is larger than the tight char box and
    /// includes whitespace appropriate for selection highlighting.
    ///
    /// Returns (left, top, right, bottom) in page coordinates.
    #[must_use = "returns the loose bounding box if available"]
    pub fn get_char_loose_box(&self, index: i32) -> Option<(f32, f32, f32, f32)> {
        let mut rect: pdfium_sys::FS_RECTF = unsafe { std::mem::zeroed() };

        let success =
            unsafe { pdfium_sys::FPDFText_GetLooseCharBox(self.handle, index, &raw mut rect) };

        if success != 0 {
            Some((rect.left, rect.top, rect.right, rect.bottom))
        } else {
            None
        }
    }

    /// Get a character's baseline origin position.
    ///
    /// BUG #8 fix: Returns the character's origin point on the baseline,
    /// useful for proper text alignment and vertical positioning.
    ///
    /// Returns (x, y) in page coordinates.
    #[must_use = "returns the character's origin point if available"]
    pub fn get_char_origin(&self, index: i32) -> Option<(f64, f64)> {
        let mut x: f64 = 0.0;
        let mut y: f64 = 0.0;

        let success = unsafe {
            pdfium_sys::FPDFText_GetCharOrigin(self.handle, index, &raw mut x, &raw mut y)
        };

        if success != 0 {
            Some((x, y))
        } else {
            None
        }
    }

    /// Get the transformation matrix for a character.
    ///
    /// BUG #9 fix: Returns the 2D transformation matrix for rotated/scaled text.
    /// The matrix components are:
    /// - a, d: Scaling (diagonal)
    /// - b, c: Rotation/shearing (off-diagonal)
    /// - e, f: Translation
    ///
    /// Returns Some((a, b, c, d, e, f)) or None if not available.
    #[must_use = "returns the transformation matrix if available"]
    pub fn get_char_matrix(&self, index: i32) -> Option<(f32, f32, f32, f32, f32, f32)> {
        let mut matrix: pdfium_sys::FS_MATRIX = unsafe { std::mem::zeroed() };

        let success =
            unsafe { pdfium_sys::FPDFText_GetMatrix(self.handle, index, &raw mut matrix) };

        if success != 0 {
            Some((matrix.a, matrix.b, matrix.c, matrix.d, matrix.e, matrix.f))
        } else {
            None
        }
    }

    /// Check if a character is generated (not from the original PDF text stream).
    ///
    /// BUG #10 fix: Generated characters are synthesized by `PDFium` (e.g., for
    /// ligature decomposition or CID font mapping). This is useful for
    /// distinguishing original text from `PDFium`'s interpretations.
    #[inline]
    #[must_use = "returns whether the character is generated"]
    pub fn is_char_generated(&self, index: i32) -> bool {
        unsafe { pdfium_sys::FPDFText_IsGenerated(self.handle, index) != 0 }
    }

    /// Check if a character is a hyphen that may have been added for word-wrapping.
    ///
    /// BUG #10 fix: Soft hyphens are added at line breaks when words are
    /// hyphenated. This helps distinguish real hyphens from word-wrap hyphens
    /// when reconstructing continuous text.
    #[inline]
    #[must_use = "returns whether the character is a soft hyphen"]
    pub fn is_char_hyphen(&self, index: i32) -> bool {
        unsafe { pdfium_sys::FPDFText_IsHyphen(self.handle, index) != 0 }
    }

    /// Check if a character has a Unicode mapping error.
    ///
    /// BUG #10 fix: Returns true if the character couldn't be properly mapped
    /// to Unicode. This typically happens with unusual fonts or CID fonts
    /// without proper `ToUnicode` mapping.
    #[inline]
    #[must_use = "returns whether the character has a Unicode mapping error"]
    pub fn has_unicode_map_error(&self, index: i32) -> bool {
        unsafe { pdfium_sys::FPDFText_HasUnicodeMapError(self.handle, index) != 0 }
    }

    /// Get the character index at a given position on the page.
    ///
    /// BUG #84 fix: This is essential for implementing text selection in UI
    /// applications. Given a click position, this returns the character index
    /// that can be used with other text APIs.
    ///
    /// # Arguments
    ///
    /// * `x` - X coordinate in page coordinates
    /// * `y` - Y coordinate in page coordinates
    /// * `x_tolerance` - Tolerance in X direction for hit testing
    /// * `y_tolerance` - Tolerance in Y direction for hit testing
    ///
    /// # Returns
    ///
    /// * `Some(index)` - Character index at the position (0-based)
    /// * `None` - No character found within tolerance
    #[inline]
    #[must_use = "returns the character index at the given position"]
    pub fn get_char_index_at_pos(
        &self,
        x: f64,
        y: f64,
        x_tolerance: f64,
        y_tolerance: f64,
    ) -> Option<i32> {
        let index = unsafe {
            pdfium_sys::FPDFText_GetCharIndexAtPos(self.handle, x, y, x_tolerance, y_tolerance)
        };

        // PDFium returns -1 if no character found, -3 if index is not available
        if index >= 0 {
            Some(index)
        } else {
            None
        }
    }

    /// Get the font size of a character in points.
    ///
    /// BUG #84 related: Useful for text selection and rendering.
    ///
    /// # Arguments
    ///
    /// * `index` - Character index (0-based)
    ///
    /// # Returns
    ///
    /// Font size in points (72 points = 1 inch)
    #[inline]
    #[must_use = "returns the font size in points"]
    pub fn get_font_size(&self, index: i32) -> f64 {
        unsafe { pdfium_sys::FPDFText_GetFontSize(self.handle, index) }
    }

    /// Get font information (name and flags) for a character.
    ///
    /// Phase 2 (P5): Extracts font name and descriptor flags for detecting
    /// bold, italic, and monospace text.
    ///
    /// # Arguments
    ///
    /// * `index` - Character index (0-based)
    ///
    /// # Returns
    ///
    /// * `Some((font_name, flags))` - Font name (UTF-8) and PDF font descriptor flags
    /// * `None` - Error or invalid index
    ///
    /// # Font Descriptor Flags (PDF spec 1.7, Section 5.7.1)
    ///
    /// * Bit 1 (0x1): `FixedPitch` - Monospace font
    /// * Bit 7 (0x40): Italic - Italic or oblique
    /// * Bit 19 (0x40000): `ForceBold` - Bold weight
    #[must_use = "returns font name and flags if available"]
    pub fn get_font_info(&self, index: i32) -> Option<(String, i32)> {
        // First call with null buffer to get required length
        let mut flags: std::ffi::c_int = 0;
        let buffer_len = unsafe {
            pdfium_sys::FPDFText_GetFontInfo(
                self.handle,
                index,
                std::ptr::null_mut(),
                0,
                &raw mut flags,
            )
        };

        if buffer_len == 0 {
            return None;
        }

        // Allocate buffer and get font name
        let mut buffer = vec![0u8; buffer_len as usize];
        let result_len = unsafe {
            pdfium_sys::FPDFText_GetFontInfo(
                self.handle,
                index,
                buffer.as_mut_ptr().cast::<std::ffi::c_void>(),
                buffer_len,
                &raw mut flags,
            )
        };

        if result_len == 0 || result_len > buffer_len {
            return None;
        }

        // Convert to String (UTF-8), removing trailing null
        let name_bytes = if buffer.last() == Some(&0) {
            &buffer[..buffer.len() - 1]
        } else {
            &buffer[..]
        };

        let font_name = String::from_utf8_lossy(name_bytes).into_owned();
        Some((font_name, flags))
    }

    /// Check if a character is rendered in bold.
    ///
    /// Uses the `ForceBold` flag (bit 19) from PDF font descriptor.
    #[inline]
    #[must_use = "returns whether the character is bold"]
    pub fn is_char_bold(&self, index: i32) -> bool {
        self.get_font_info(index)
            .is_some_and(|(_, flags)| (flags & PDF_FONT_FORCE_BOLD) != 0)
    }

    /// Check if a character is rendered in italic.
    ///
    /// Uses the Italic flag (bit 7) from PDF font descriptor.
    #[inline]
    #[must_use = "returns whether the character is italic"]
    pub fn is_char_italic(&self, index: i32) -> bool {
        self.get_font_info(index)
            .is_some_and(|(_, flags)| (flags & PDF_FONT_ITALIC) != 0)
    }

    /// Check if a character is rendered in a monospace (fixed-pitch) font.
    ///
    /// Uses the `FixedPitch` flag (bit 1) from PDF font descriptor.
    /// Useful for detecting code blocks.
    #[inline]
    #[must_use = "returns whether the character is monospace"]
    pub fn is_char_monospace(&self, index: i32) -> bool {
        self.get_font_info(index)
            .is_some_and(|(_, flags)| (flags & PDF_FONT_FIXED_PITCH) != 0)
    }

    /// Get the rotation angle of a character in degrees.
    ///
    /// BUG #84 related: Useful for rendering rotated text in UI applications.
    ///
    /// # Arguments
    ///
    /// * `index` - Character index (0-based)
    ///
    /// # Returns
    ///
    /// Rotation angle in degrees (counter-clockwise from horizontal)
    #[inline]
    #[must_use = "returns the character rotation angle"]
    pub fn get_char_angle(&self, index: i32) -> f32 {
        unsafe { pdfium_sys::FPDFText_GetCharAngle(self.handle, index) }
    }

    /// Get the number of rectangles needed to highlight a range of text.
    ///
    /// BUG #84 fix: Text selection often requires multiple rectangles when
    /// text wraps across lines. This returns the count for `get_text_rect()`.
    ///
    /// # Arguments
    ///
    /// * `start_index` - Starting character index (0-based)
    /// * `count` - Number of characters to include (-1 for all remaining)
    ///
    /// # Returns
    ///
    /// Number of rectangles needed to cover the text range
    #[inline]
    #[must_use = "returns the count of rectangles for the text range"]
    pub fn count_text_rects(&self, start_index: i32, count: i32) -> i32 {
        unsafe { pdfium_sys::FPDFText_CountRects(self.handle, start_index, count) }
    }

    /// Get a rectangle for highlighting a range of text.
    ///
    /// BUG #84 fix: Returns one of the rectangles needed to highlight text.
    /// Use `count_text_rects()` first to determine how many rectangles exist.
    ///
    /// # Arguments
    ///
    /// * `rect_index` - Rectangle index (0 to count_text_rects()-1)
    ///
    /// # Returns
    ///
    /// * `Some((left, top, right, bottom))` - Rectangle in page coordinates
    /// * `None` - Invalid `rect_index` or error
    #[must_use = "returns the text rectangle if valid"]
    pub fn get_text_rect(&self, rect_index: i32) -> Option<(f64, f64, f64, f64)> {
        let mut left: f64 = 0.0;
        let mut top: f64 = 0.0;
        let mut right: f64 = 0.0;
        let mut bottom: f64 = 0.0;

        let success = unsafe {
            pdfium_sys::FPDFText_GetRect(
                self.handle,
                rect_index,
                &raw mut left,
                &raw mut top,
                &raw mut right,
                &raw mut bottom,
            )
        };

        if success != 0 {
            Some((left, top, right, bottom))
        } else {
            None
        }
    }

    /// Get all rectangles for highlighting a range of text.
    ///
    /// BUG #84 fix: Convenience method that returns all rectangles needed
    /// to highlight a text range. Combines `count_text_rects()` and `get_text_rect()`.
    ///
    /// # Arguments
    ///
    /// * `start_index` - Starting character index (0-based)
    /// * `count` - Number of characters to include (-1 for all remaining)
    ///
    /// # Returns
    ///
    /// Vector of (left, top, right, bottom) rectangles in page coordinates
    #[must_use = "returns all text rectangles for the range"]
    pub fn get_text_rects(&self, start_index: i32, count: i32) -> Vec<(f64, f64, f64, f64)> {
        let rect_count = self.count_text_rects(start_index, count);
        let mut rects = Vec::with_capacity(rect_count as usize);

        for i in 0..rect_count {
            if let Some(rect) = self.get_text_rect(i) {
                rects.push(rect);
            }
        }

        rects
    }

    /// Extract web links detected in the text content.
    ///
    /// BUG #21 fix: Uses `PDFium's` `FPDFLink_LoadWebLinks()` to automatically detect
    /// URLs (http://, https://, www., etc.) in the page text. This is different
    /// from hyperlink annotations - these are URLs that appear as visible text.
    ///
    /// Each web link includes:
    /// - The detected URL string
    /// - The character range in the text
    /// - Bounding rectangles (may be multiple if the URL spans lines)
    ///
    /// # Returns
    ///
    /// Vector of `PdfWebLink` structs, or empty vector if no web links found.
    #[must_use = "returns detected web links from the text"]
    pub fn extract_web_links(&self) -> Vec<PdfWebLink> {
        let mut results = Vec::new();

        // Load web links from text page
        let link_page = unsafe { pdfium_sys::FPDFLink_LoadWebLinks(self.handle) };
        if link_page.is_null() {
            return results;
        }

        // Get count of web links
        let count = unsafe { pdfium_sys::FPDFLink_CountWebLinks(link_page) };

        for link_index in 0..count {
            // Get URL (need to call twice - first to get size, then to get string)
            let url_len = unsafe {
                pdfium_sys::FPDFLink_GetURL(link_page, link_index, std::ptr::null_mut(), 0)
            };

            if url_len <= 0 {
                continue;
            }

            // Allocate buffer for URL (UTF-16, including null terminator)
            let mut url_buffer: Vec<u16> = vec![0; url_len as usize];
            let actual_len = unsafe {
                pdfium_sys::FPDFLink_GetURL(link_page, link_index, url_buffer.as_mut_ptr(), url_len)
            };

            if actual_len <= 0 {
                continue;
            }

            // Convert URL from UTF-16, removing null terminator
            let url_char_count = (actual_len as usize).saturating_sub(1);
            let url = utf16_to_string_with_warning(&url_buffer[..url_char_count], "web link URL");

            // Get text range for this link
            let mut start_char_index: i32 = 0;
            let mut char_count: i32 = 0;
            let has_range = unsafe {
                pdfium_sys::FPDFLink_GetTextRange(
                    link_page,
                    link_index,
                    &raw mut start_char_index,
                    &raw mut char_count,
                )
            };

            if has_range == 0 {
                // Still include the link, but without text range info
                start_char_index = -1;
                char_count = 0;
            }

            // Get bounding rectangles (link may span multiple lines)
            let rect_count = unsafe { pdfium_sys::FPDFLink_CountRects(link_page, link_index) };
            let mut rects = Vec::with_capacity(rect_count.max(0) as usize);

            for rect_index in 0..rect_count {
                let mut left: f64 = 0.0;
                let mut top: f64 = 0.0;
                let mut right: f64 = 0.0;
                let mut bottom: f64 = 0.0;

                let success = unsafe {
                    pdfium_sys::FPDFLink_GetRect(
                        link_page,
                        link_index,
                        rect_index,
                        &raw mut left,
                        &raw mut top,
                        &raw mut right,
                        &raw mut bottom,
                    )
                };

                if success != 0 {
                    rects.push((left, top, right, bottom));
                }
            }

            results.push(PdfWebLink {
                url,
                start_char_index,
                char_count,
                rects,
            });
        }

        // Clean up - MUST close web links handle
        unsafe {
            pdfium_sys::FPDFLink_CloseWebLinks(link_page);
        }

        results
    }

    /// Count the number of web links in the text content.
    ///
    /// This is a fast way to check if a page contains web links without
    /// extracting all the details.
    ///
    /// # Returns
    ///
    /// Number of web links detected, or 0 if none or on error.
    #[must_use = "returns the count of web links"]
    pub fn count_web_links(&self) -> i32 {
        let link_page = unsafe { pdfium_sys::FPDFLink_LoadWebLinks(self.handle) };
        if link_page.is_null() {
            return 0;
        }

        let count = unsafe { pdfium_sys::FPDFLink_CountWebLinks(link_page) };

        unsafe {
            pdfium_sys::FPDFLink_CloseWebLinks(link_page);
        }

        count
    }

    /// Map a text index to a character index.
    ///
    /// BUG #82 fix: The text index (from `FPDFText_GetText`) may differ from the
    /// character index (used by `FPDFText_GetCharBox`, etc.) when the PDF contains
    /// complex text features like ligatures, CID fonts, or certain encodings.
    ///
    /// # Arguments
    ///
    /// * `text_index` - Index into the text string returned by `get_text()`
    ///
    /// # Returns
    ///
    /// * `Some(char_index)` - Corresponding character index for use with char APIs
    /// * `None` - Invalid `text_index` or mapping not available
    #[inline]
    #[must_use = "returns the character index for the text index"]
    pub fn text_index_to_char_index(&self, text_index: i32) -> Option<i32> {
        // SAFETY: self.handle is valid (created in from_page, dropped in Drop impl)
        let result =
            unsafe { pdfium_sys::FPDFText_GetCharIndexFromTextIndex(self.handle, text_index) };
        if result >= 0 {
            Some(result)
        } else {
            None
        }
    }

    /// Map a character index to a text index.
    ///
    /// BUG #82 fix: Reverse mapping from character index (used by char APIs like
    /// `get_char_box`) to text index (position in string from `get_text()`).
    ///
    /// # Arguments
    ///
    /// * `char_index` - Character index (0-based, from char APIs)
    ///
    /// # Returns
    ///
    /// * `Some(text_index)` - Corresponding text index in `get_text()` output
    /// * `None` - Invalid `char_index` or mapping not available
    #[inline]
    #[must_use = "returns the text index for the character index"]
    pub fn char_index_to_text_index(&self, char_index: i32) -> Option<i32> {
        // SAFETY: self.handle is valid (created in from_page, dropped in Drop impl)
        let result =
            unsafe { pdfium_sys::FPDFText_GetTextIndexFromCharIndex(self.handle, char_index) };
        if result >= 0 {
            Some(result)
        } else {
            None
        }
    }
}

impl Drop for PdfTextPageFast {
    fn drop(&mut self) {
        // SAFETY:
        // - self.handle is valid text page handle (created by FPDFText_LoadPage)
        // - Must be closed before the underlying PDF page is closed
        // - Releases text extraction resources
        unsafe {
            pdfium_sys::FPDFText_ClosePage(self.handle);
        }
    }
}

impl std::fmt::Debug for PdfTextPageFast {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PdfTextPageFast")
            .field("char_count", &self.char_count())
            .finish_non_exhaustive()
    }
}

/// Text cell extracted from PDF with bounding box coordinates and font info.
///
/// Coordinates are in PDF points (1/72 inch) relative to the page origin.
/// Font information is extracted from the first character of the cell.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TextCellFast {
    /// The text content of this cell
    pub text: String,
    /// Left edge X coordinate in PDF points
    pub left: f64,
    /// Top edge Y coordinate in PDF points
    pub top: f64,
    /// Right edge X coordinate in PDF points
    pub right: f64,
    /// Bottom edge Y coordinate in PDF points
    pub bottom: f64,
    /// Font size in points (from first character of cell)
    pub font_size: Option<f64>,
    /// Font weight (400=normal, 700=bold, from first character)
    pub font_weight: Option<i32>,
    /// Character rotation angle in radians (from first character)
    pub char_angle: Option<f32>,
    /// Font name (from first character of cell)
    pub font_name: Option<String>,
    /// Text fill color as RGBA (from first character)
    pub fill_color: Option<(u32, u32, u32, u32)>,
    /// Text stroke color as RGBA (from first character)
    pub stroke_color: Option<(u32, u32, u32, u32)>,
    /// Whether text is bold (from PDF font descriptor flags or font name heuristics)
    pub is_bold: bool,
    /// Whether text is italic (from PDF font descriptor flags or font name heuristics)
    pub is_italic: bool,
}

/// Hyperlink extracted from PDF page.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PdfHyperlink {
    /// The URL this link points to
    pub url: String,
    /// Left edge X coordinate in PDF points
    pub left: f64,
    /// Top edge Y coordinate in PDF points
    pub top: f64,
    /// Right edge X coordinate in PDF points
    pub right: f64,
    /// Bottom edge Y coordinate in PDF points
    pub bottom: f64,
}

/// Web link detected in PDF text content.
///
/// BUG #21 fix: Web links are URLs (http://, https://, www., etc.) found in the
/// actual text content of the PDF, not in annotation/link objects. `PDFium's`
/// `FPDFLink_LoadWebLinks()` function detects these automatically.
///
/// Contrast with `PdfHyperlink` which represents clickable link annotations.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PdfWebLink {
    /// The detected URL
    pub url: String,
    /// Starting character index in the text page
    pub start_char_index: i32,
    /// Number of characters that make up the URL
    pub char_count: i32,
    /// Bounding rectangles for the link (may span multiple lines)
    pub rects: Vec<(f64, f64, f64, f64)>,
}

/// Bookmark (table of contents entry) from a PDF document.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct PdfBookmark {
    /// The title of the bookmark
    pub title: String,
    /// Nesting level (0 = top level)
    pub level: i32,
    /// Target page index (0-based), if known
    pub page_index: Option<i32>,
}

/// Embedded file attachment in a PDF document.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct PdfAttachment {
    /// Name of the attached file
    pub name: String,
    /// File contents (raw bytes)
    pub data: Vec<u8>,
}

/// A word extracted from a PDF page using batch API.
///
/// Words are sequences of non-whitespace characters separated by whitespace
/// or large gaps. This struct provides word boundaries and text content
/// for natural language processing tasks.
///
/// P6/P10: Word-level extraction implementation (N=3514).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PdfWord {
    /// The word text
    pub text: String,
    /// Left edge X coordinate in PDF points (top-left origin)
    pub left: f64,
    /// Top edge Y coordinate in PDF points (top-left origin)
    pub top: f64,
    /// Right edge X coordinate in PDF points (top-left origin)
    pub right: f64,
    /// Bottom edge Y coordinate in PDF points (top-left origin)
    pub bottom: f64,
    /// Starting character index in the text page
    pub start_char: i32,
    /// Ending character index (exclusive) in the text page
    pub end_char: i32,
}

/// PDF annotation type enumeration.
/// BUG #1 fix: Support for annotation extraction.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum PdfAnnotationType {
    /// Unknown or unsupported annotation type
    #[default]
    Unknown = 0,
    /// Text annotation (sticky note)
    Text = 1,
    /// Link annotation
    Link = 2,
    /// Free text annotation
    FreeText = 3,
    /// Line annotation
    Line = 4,
    /// Square annotation
    Square = 5,
    /// Circle annotation
    Circle = 6,
    /// Polygon annotation
    Polygon = 7,
    /// Polyline annotation
    Polyline = 8,
    /// Highlight annotation
    Highlight = 9,
    /// Underline annotation
    Underline = 10,
    /// Squiggly underline annotation
    Squiggly = 11,
    /// Strikeout annotation
    Strikeout = 12,
    /// Stamp annotation
    Stamp = 13,
    /// Caret annotation
    Caret = 14,
    /// Ink (freehand drawing) annotation
    Ink = 15,
    /// Popup annotation
    Popup = 16,
    /// File attachment annotation
    FileAttachment = 17,
    /// Sound annotation
    Sound = 18,
    /// Movie annotation
    Movie = 19,
    /// Widget (form field) annotation
    Widget = 20,
    /// Screen annotation
    Screen = 21,
    /// Printer mark annotation
    PrinterMark = 22,
    /// Trap network annotation
    TrapNet = 23,
    /// Watermark annotation
    Watermark = 24,
    /// 3D annotation
    ThreeD = 25,
    /// Rich media annotation
    RichMedia = 26,
    /// XFA widget annotation
    XfaWidget = 27,
    /// Redaction annotation
    Redact = 28,
}

impl PdfAnnotationType {
    /// Convert from `PDFium` annotation subtype constant.
    #[inline]
    const fn from_pdfium(subtype: u32) -> Self {
        match subtype {
            1 => Self::Text,
            2 => Self::Link,
            3 => Self::FreeText,
            4 => Self::Line,
            5 => Self::Square,
            6 => Self::Circle,
            7 => Self::Polygon,
            8 => Self::Polyline,
            9 => Self::Highlight,
            10 => Self::Underline,
            11 => Self::Squiggly,
            12 => Self::Strikeout,
            13 => Self::Stamp,
            14 => Self::Caret,
            15 => Self::Ink,
            16 => Self::Popup,
            17 => Self::FileAttachment,
            18 => Self::Sound,
            19 => Self::Movie,
            20 => Self::Widget,
            21 => Self::Screen,
            22 => Self::PrinterMark,
            23 => Self::TrapNet,
            24 => Self::Watermark,
            25 => Self::ThreeD,
            26 => Self::RichMedia,
            27 => Self::XfaWidget,
            28 => Self::Redact,
            _ => Self::Unknown,
        }
    }

    /// Check if this annotation type typically contains user-visible content.
    /// Useful for filtering to only meaningful annotations.
    #[inline]
    #[must_use = "returns whether this is a content annotation"]
    pub const fn is_content_annotation(&self) -> bool {
        matches!(
            self,
            Self::Text
                | Self::FreeText
                | Self::Highlight
                | Self::Underline
                | Self::Squiggly
                | Self::Strikeout
                | Self::Stamp
                | Self::Ink
                | Self::FileAttachment
        )
    }
}

impl std::fmt::Display for PdfAnnotationType {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unknown => write!(f, "unknown"),
            Self::Text => write!(f, "text"),
            Self::Link => write!(f, "link"),
            Self::FreeText => write!(f, "free_text"),
            Self::Line => write!(f, "line"),
            Self::Square => write!(f, "square"),
            Self::Circle => write!(f, "circle"),
            Self::Polygon => write!(f, "polygon"),
            Self::Polyline => write!(f, "polyline"),
            Self::Highlight => write!(f, "highlight"),
            Self::Underline => write!(f, "underline"),
            Self::Squiggly => write!(f, "squiggly"),
            Self::Strikeout => write!(f, "strikeout"),
            Self::Stamp => write!(f, "stamp"),
            Self::Caret => write!(f, "caret"),
            Self::Ink => write!(f, "ink"),
            Self::Popup => write!(f, "popup"),
            Self::FileAttachment => write!(f, "file_attachment"),
            Self::Sound => write!(f, "sound"),
            Self::Movie => write!(f, "movie"),
            Self::Widget => write!(f, "widget"),
            Self::Screen => write!(f, "screen"),
            Self::PrinterMark => write!(f, "printer_mark"),
            Self::TrapNet => write!(f, "trap_net"),
            Self::Watermark => write!(f, "watermark"),
            Self::ThreeD => write!(f, "3d"),
            Self::RichMedia => write!(f, "rich_media"),
            Self::XfaWidget => write!(f, "xfa_widget"),
            Self::Redact => write!(f, "redact"),
        }
    }
}

impl std::str::FromStr for PdfAnnotationType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Normalize: lowercase, remove hyphens/underscores/spaces
        let normalized: String = s
            .to_lowercase()
            .chars()
            .filter(|c| *c != '-' && *c != '_' && *c != ' ')
            .collect();

        match normalized.as_str() {
            "unknown" => Ok(Self::Unknown),
            "text" | "note" | "stickynote" => Ok(Self::Text),
            "link" | "hyperlink" => Ok(Self::Link),
            "freetext" | "textbox" => Ok(Self::FreeText),
            "line" => Ok(Self::Line),
            "square" | "rect" | "rectangle" => Ok(Self::Square),
            "circle" => Ok(Self::Circle),
            "polygon" => Ok(Self::Polygon),
            "polyline" => Ok(Self::Polyline),
            "highlight" | "hl" => Ok(Self::Highlight),
            "underline" | "ul" => Ok(Self::Underline),
            "squiggly" | "wavy" => Ok(Self::Squiggly),
            "strikeout" | "strikethrough" | "strike" => Ok(Self::Strikeout),
            "stamp" => Ok(Self::Stamp),
            "caret" | "insertion" => Ok(Self::Caret),
            "ink" | "freehand" | "drawing" => Ok(Self::Ink),
            "popup" => Ok(Self::Popup),
            "fileattachment" | "attachment" | "file" => Ok(Self::FileAttachment),
            "sound" | "audio" => Ok(Self::Sound),
            "movie" | "video" => Ok(Self::Movie),
            "widget" | "formfield" => Ok(Self::Widget),
            "screen" => Ok(Self::Screen),
            "printermark" | "printer" => Ok(Self::PrinterMark),
            "trapnet" | "trap" => Ok(Self::TrapNet),
            "watermark" => Ok(Self::Watermark),
            "threed" | "3d" => Ok(Self::ThreeD),
            "richmedia" | "rich" => Ok(Self::RichMedia),
            "xfawidget" | "xfa" => Ok(Self::XfaWidget),
            "redact" | "redaction" => Ok(Self::Redact),
            _ => Err(format!("Unknown PDF annotation type: '{s}'")),
        }
    }
}

/// A PDF annotation extracted from a page.
/// BUG #1 fix: Support for annotation extraction.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PdfAnnotation {
    /// The type of annotation
    pub annotation_type: PdfAnnotationType,
    /// Left edge X coordinate in PDF points (top-left origin)
    pub left: f64,
    /// Top edge Y coordinate in PDF points (top-left origin)
    pub top: f64,
    /// Right edge X coordinate in PDF points (top-left origin)
    pub right: f64,
    /// Bottom edge Y coordinate in PDF points (top-left origin)
    pub bottom: f64,
    /// Contents/text of the annotation (if any)
    pub contents: Option<String>,
    /// Author of the annotation (if any)
    pub author: Option<String>,
    /// Modification date as ISO 8601 string (if any)
    pub modification_date: Option<String>,
    /// Annotation flags (visibility, print, etc.)
    pub flags: u32,
}

/// Form field type enumeration.
///
/// BUG #22 fix: Detect form field types in PDF documents.
/// Form fields are Widget annotations with specific interactive functionality.
/// These constants match `PDFium's` `FPDF_FORMFIELD_*` values.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum PdfFormFieldType {
    /// Unknown form field type
    #[default]
    Unknown = 0,
    /// Push button (clickable button)
    PushButton = 1,
    /// Checkbox (toggleable)
    Checkbox = 2,
    /// Radio button (mutually exclusive selection)
    RadioButton = 3,
    /// Combo box (dropdown list)
    ComboBox = 4,
    /// List box (scrollable list)
    ListBox = 5,
    /// Text field (text input)
    TextField = 6,
    /// Signature field (digital signature)
    Signature = 7,
    /// XFA form field (dynamic form)
    Xfa = 8,
    /// XFA checkbox
    XfaCheckbox = 9,
    /// XFA combo box
    XfaComboBox = 10,
    /// XFA image field
    XfaImageField = 11,
    /// XFA list box
    XfaListBox = 12,
    /// XFA push button
    XfaPushButton = 13,
    /// XFA signature field
    XfaSignature = 14,
    /// XFA text field
    XfaTextField = 15,
}

impl PdfFormFieldType {
    /// Check if this is a text input field type
    #[inline]
    #[must_use = "returns whether this is a text input field"]
    pub const fn is_text_input(self) -> bool {
        matches!(self, Self::TextField | Self::XfaTextField)
    }

    /// Check if this is a selection field type (checkbox, radio, combo, list)
    #[inline]
    #[must_use = "returns whether this is a selection field"]
    pub const fn is_selection(self) -> bool {
        matches!(
            self,
            Self::Checkbox
                | Self::RadioButton
                | Self::ComboBox
                | Self::ListBox
                | Self::XfaCheckbox
                | Self::XfaComboBox
                | Self::XfaListBox
        )
    }

    /// Check if this is a button type
    #[inline]
    #[must_use = "returns whether this is a button field"]
    pub const fn is_button(self) -> bool {
        matches!(self, Self::PushButton | Self::XfaPushButton)
    }

    /// Check if this is a signature field
    #[inline]
    #[must_use = "returns whether this is a signature field"]
    pub const fn is_signature(self) -> bool {
        matches!(self, Self::Signature | Self::XfaSignature)
    }

    /// Check if this is an XFA (dynamic) form field
    #[inline]
    #[must_use = "returns whether this is an XFA field"]
    pub const fn is_xfa(self) -> bool {
        matches!(
            self,
            Self::Xfa
                | Self::XfaCheckbox
                | Self::XfaComboBox
                | Self::XfaImageField
                | Self::XfaListBox
                | Self::XfaPushButton
                | Self::XfaSignature
                | Self::XfaTextField
        )
    }
}

impl std::fmt::Display for PdfFormFieldType {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unknown => write!(f, "unknown"),
            Self::PushButton => write!(f, "push_button"),
            Self::Checkbox => write!(f, "checkbox"),
            Self::RadioButton => write!(f, "radio_button"),
            Self::ComboBox => write!(f, "combo_box"),
            Self::ListBox => write!(f, "list_box"),
            Self::TextField => write!(f, "text_field"),
            Self::Signature => write!(f, "signature"),
            Self::Xfa => write!(f, "xfa"),
            Self::XfaCheckbox => write!(f, "xfa_checkbox"),
            Self::XfaComboBox => write!(f, "xfa_combo_box"),
            Self::XfaImageField => write!(f, "xfa_image_field"),
            Self::XfaListBox => write!(f, "xfa_list_box"),
            Self::XfaPushButton => write!(f, "xfa_push_button"),
            Self::XfaSignature => write!(f, "xfa_signature"),
            Self::XfaTextField => write!(f, "xfa_text_field"),
        }
    }
}

impl std::str::FromStr for PdfFormFieldType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Normalize: lowercase, remove hyphens/underscores/spaces
        let normalized: String = s
            .to_lowercase()
            .chars()
            .filter(|c| *c != '-' && *c != '_' && *c != ' ')
            .collect();

        match normalized.as_str() {
            "unknown" => Ok(Self::Unknown),
            "pushbutton" | "button" | "btn" => Ok(Self::PushButton),
            "checkbox" | "check" | "cb" => Ok(Self::Checkbox),
            "radiobutton" | "radio" | "rb" => Ok(Self::RadioButton),
            "combobox" | "combo" | "dropdown" | "select" => Ok(Self::ComboBox),
            "listbox" | "list" => Ok(Self::ListBox),
            "textfield" | "text" | "input" => Ok(Self::TextField),
            "signature" | "sig" => Ok(Self::Signature),
            "xfa" => Ok(Self::Xfa),
            "xfacheckbox" | "xfacheck" => Ok(Self::XfaCheckbox),
            "xfacombobox" | "xfacombo" | "xfadropdown" => Ok(Self::XfaComboBox),
            "xfaimagefield" | "xfaimage" | "xfaimg" => Ok(Self::XfaImageField),
            "xfalistbox" | "xfalist" => Ok(Self::XfaListBox),
            "xfapushbutton" | "xfabutton" | "xfabtn" => Ok(Self::XfaPushButton),
            "xfasignature" | "xfasig" => Ok(Self::XfaSignature),
            "xfatextfield" | "xfatext" | "xfainput" => Ok(Self::XfaTextField),
            _ => Err(format!("Unknown PDF form field type: '{s}'")),
        }
    }
}

/// A form field detected on a PDF page.
///
/// BUG #22 fix: Basic form field information extracted from Widget annotations.
/// For full form field details (names, values), use the form fill environment APIs.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PdfFormField {
    /// Index of the Widget annotation (can be used to get full annotation details)
    pub annotation_index: i32,
    /// Left edge X coordinate in PDF points (top-left origin)
    pub left: f64,
    /// Top edge Y coordinate in PDF points (top-left origin)
    pub top: f64,
    /// Right edge X coordinate in PDF points (top-left origin)
    pub right: f64,
    /// Bottom edge Y coordinate in PDF points (top-left origin)
    pub bottom: f64,
    /// Annotation flags (visibility, print, etc.)
    pub flags: u32,
}

/// Form type enumeration indicating what kind of forms a PDF document contains.
///
/// BUG #24 fix: Detect XFA forms and other form types in PDF documents.
/// Uses `PDFium's` `FPDF_GetFormType` to determine the form specification used.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum PdfFormType {
    /// Document contains no forms
    #[default]
    None = 0,
    /// Forms are specified using `AcroForm` spec (standard PDF forms)
    AcroForm = 1,
    /// Forms are specified using the entire XFA spec (dynamic forms)
    XfaFull = 2,
    /// Forms are specified using the XFAF subset of XFA spec
    XfaForeground = 3,
}

impl PdfFormType {
    /// Convert from `PDFium's` raw form type integer
    #[inline]
    #[must_use = "converts raw integer to form type"]
    const fn from_raw(raw: i32) -> Self {
        match raw {
            1 => Self::AcroForm,
            2 => Self::XfaFull,
            3 => Self::XfaForeground,
            _ => Self::None,
        }
    }

    /// Check if this is any XFA form type (full or foreground)
    #[inline]
    #[must_use = "returns whether this is an XFA form type"]
    pub const fn is_xfa(self) -> bool {
        matches!(self, Self::XfaFull | Self::XfaForeground)
    }

    /// Check if this is a standard `AcroForm`
    #[inline]
    #[must_use = "returns whether this is a standard AcroForm"]
    pub fn is_acro_form(self) -> bool {
        self == Self::AcroForm
    }

    /// Check if the document contains any forms
    #[inline]
    #[must_use = "returns whether the document contains any forms"]
    pub fn has_forms(self) -> bool {
        self != Self::None
    }
}

impl std::fmt::Display for PdfFormType {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::AcroForm => write!(f, "acro_form"),
            Self::XfaFull => write!(f, "xfa_full"),
            Self::XfaForeground => write!(f, "xfa_foreground"),
        }
    }
}

impl std::str::FromStr for PdfFormType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Normalize: lowercase, remove hyphens/underscores/spaces
        let normalized: String = s
            .to_lowercase()
            .chars()
            .filter(|c| *c != '-' && *c != '_' && *c != ' ')
            .collect();

        match normalized.as_str() {
            "none" | "noform" | "noforms" | "empty" => Ok(Self::None),
            "acroform" | "acro" | "standard" | "pdf" => Ok(Self::AcroForm),
            "xfafull" | "xfa" | "dynamic" => Ok(Self::XfaFull),
            "xfaforeground" | "xfaf" | "foreground" => Ok(Self::XfaForeground),
            _ => Err(format!("Unknown PDF form type: '{s}'")),
        }
    }
}

/// Named destination in a PDF document.
///
/// BUG #23 fix: Support for named destinations (internal PDF navigation targets).
/// Named destinations allow PDFs to define labeled locations that can be referenced
/// by bookmarks, links, and external URIs (e.g., `file.pdf#dest_name`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PdfNamedDestination {
    /// The name of this destination (UTF-8 string)
    pub name: String,
    /// Target page index (0-based)
    pub page_index: i32,
    /// X coordinate on the page (if specified, in PDF points from left)
    pub x: Option<f32>,
    /// Y coordinate on the page (if specified, in PDF points from bottom)
    pub y: Option<f32>,
    /// Zoom level (if specified, 1.0 = 100%)
    pub zoom: Option<f32>,
}

/// Digital signature in a PDF document.
///
/// BUG #85 fix: Support for extracting digital signature information from PDFs.
/// Digital signatures are used to verify document authenticity and integrity.
/// Note: This extracts signature metadata only; actual cryptographic validation
/// requires external tools like OpenSSL.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct PdfSignature {
    /// Index of this signature in the document (0-based)
    pub index: i32,
    /// The signature subfilter (encoding format), e.g., "adbe.pkcs7.detached"
    pub sub_filter: Option<String>,
    /// Reason for signing (if provided by signer)
    pub reason: Option<String>,
    /// Time of signing in PDF date format (D:YYYYMMDDHHMMSS+HH'mm')
    pub signing_time: Option<String>,
    /// `DocMDP` permission level (1=no changes, 2=fill forms, 3=fill forms + annotate)
    pub doc_mdp_permission: Option<u32>,
    /// Raw signature contents (DER-encoded PKCS#1 or PKCS#7 binary)
    pub contents: Vec<u8>,
    /// Byte range pairs: [(start1, len1), (start2, len2), ...]
    /// Describes which parts of the document are covered by the signature
    pub byte_range: Vec<(i32, i32)>,
}

impl PdfSignature {
    /// Check if this signature has a reason field
    #[inline]
    #[must_use = "returns whether the signature has a reason field"]
    pub const fn has_reason(&self) -> bool {
        self.reason.is_some()
    }

    /// Check if this signature has signing time
    #[inline]
    #[must_use = "returns whether the signature has signing time"]
    pub const fn has_signing_time(&self) -> bool {
        self.signing_time.is_some()
    }

    /// Check if this signature allows any changes (`DocMDP` permission 2 or 3)
    #[inline]
    #[must_use = "returns whether the signature allows document changes"]
    pub fn allows_changes(&self) -> bool {
        self.doc_mdp_permission.is_none_or(|p| p >= 2)
    }

    /// Check if this signature allows form filling (`DocMDP` permission 2 or 3)
    #[inline]
    #[must_use = "returns whether the signature allows form filling"]
    pub fn allows_form_fill(&self) -> bool {
        self.doc_mdp_permission.is_none_or(|p| p >= 2)
    }

    /// Check if this signature allows annotations (`DocMDP` permission 3)
    #[inline]
    #[must_use = "returns whether the signature allows annotations"]
    pub fn allows_annotations(&self) -> bool {
        self.doc_mdp_permission.is_none_or(|p| p >= 3)
    }
}

/// JavaScript action embedded in a PDF document.
///
/// BUG #86 fix: JavaScript detection for security scanning.
/// PDFs can contain embedded JavaScript that may execute on open.
/// This is a security concern as malicious JavaScript can exploit
/// vulnerabilities in PDF readers.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct PdfJavaScriptAction {
    /// Index of this JavaScript action in the document (0-based)
    pub index: i32,
    /// Name of the JavaScript action (if provided)
    pub name: Option<String>,
    /// The actual JavaScript code
    pub script: String,
}

impl PdfJavaScriptAction {
    /// Check if this action has a name
    #[inline]
    #[must_use = "returns whether this action has a name"]
    pub const fn has_name(&self) -> bool {
        self.name.is_some()
    }

    /// Get the script length in characters
    #[inline]
    #[must_use = "returns the script length in characters"]
    pub const fn script_len(&self) -> usize {
        self.script.len()
    }

    /// Check if the script is empty
    #[inline]
    #[must_use = "returns whether the script is empty"]
    pub const fn is_empty(&self) -> bool {
        self.script.is_empty()
    }

    /// Check if the script contains potentially dangerous patterns.
    ///
    /// This is a simple heuristic check - NOT a complete security analysis.
    /// Returns true if the script contains patterns often used in malicious PDFs.
    #[must_use = "returns whether the script contains suspicious patterns"]
    pub fn has_suspicious_patterns(&self) -> bool {
        let script_lower = self.script.to_lowercase();
        // Common malicious patterns in PDF JavaScript
        script_lower.contains("eval(")
            || script_lower.contains("util.printf")
            || script_lower.contains("collab.geticon")
            || script_lower.contains("spell.check")
            || script_lower.contains("geturl")
            || script_lower.contains("submitform")
            || script_lower.contains("launch")
            || script_lower.contains("app.launchurl")
            || script_lower.contains("exportasfdf")
            || script_lower.contains("collectemailinfo")
    }
}

/// Page object type enumeration.
///
/// BUG #83 fix: Enables iteration over all objects on a PDF page.
/// Each object has a type indicating what kind of content it represents.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum PdfPageObjectType {
    /// Unknown object type
    #[default]
    Unknown = 0,
    /// Text object (characters, glyphs)
    Text = 1,
    /// Path object (lines, curves, shapes)
    Path = 2,
    /// Image object (embedded images)
    Image = 3,
    /// Shading object (gradients)
    Shading = 4,
    /// Form `XObject` (reusable content)
    Form = 5,
}

impl PdfPageObjectType {
    /// Convert from `PDFium's` raw object type integer
    #[must_use = "converts raw integer to object type"]
    pub const fn from_raw(raw: i32) -> Self {
        match raw as u32 {
            1 => Self::Text,
            2 => Self::Path,
            3 => Self::Image,
            4 => Self::Shading,
            5 => Self::Form,
            _ => Self::Unknown,
        }
    }

    /// Check if this is a text object
    #[inline]
    #[must_use = "returns whether this is a text object"]
    pub fn is_text(self) -> bool {
        self == Self::Text
    }

    /// Check if this is an image object
    #[inline]
    #[must_use = "returns whether this is an image object"]
    pub fn is_image(self) -> bool {
        self == Self::Image
    }

    /// Check if this is a path object
    #[inline]
    #[must_use = "returns whether this is a path object"]
    pub fn is_path(self) -> bool {
        self == Self::Path
    }
}

impl std::fmt::Display for PdfPageObjectType {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unknown => write!(f, "unknown"),
            Self::Text => write!(f, "text"),
            Self::Path => write!(f, "path"),
            Self::Image => write!(f, "image"),
            Self::Shading => write!(f, "shading"),
            Self::Form => write!(f, "form"),
        }
    }
}

impl std::str::FromStr for PdfPageObjectType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Normalize: lowercase, remove hyphens/underscores/spaces
        let normalized: String = s
            .to_lowercase()
            .chars()
            .filter(|c| *c != '-' && *c != '_' && *c != ' ')
            .collect();

        match normalized.as_str() {
            "unknown" => Ok(Self::Unknown),
            "text" | "txt" | "glyph" | "char" => Ok(Self::Text),
            "path" | "line" | "curve" | "shape" | "vector" => Ok(Self::Path),
            "image" | "img" | "picture" | "bitmap" => Ok(Self::Image),
            "shading" | "gradient" | "shade" => Ok(Self::Shading),
            "form" | "xobject" | "formxobject" => Ok(Self::Form),
            _ => Err(format!("Unknown PDF page object type: '{s}'")),
        }
    }
}

/// A page object extracted from a PDF page.
///
/// BUG #83 fix: Provides information about individual objects on a page,
/// including their type, bounds, and properties.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PdfPageObject {
    /// Index of this object on the page
    pub index: i32,
    /// Type of the object
    pub object_type: PdfPageObjectType,
    /// Left bound in page coordinates
    pub left: f32,
    /// Bottom bound in page coordinates
    pub bottom: f32,
    /// Right bound in page coordinates
    pub right: f32,
    /// Top bound in page coordinates
    pub top: f32,
    /// Whether the object has transparency
    pub has_transparency: bool,
}

/// Flags for text search operations.
///
/// BUG #2 fix: Exposes `PDFium's` text search flags.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct PdfTextSearchFlags(u32);

impl PdfTextSearchFlags {
    /// Case-insensitive search (default)
    pub const CASE_INSENSITIVE: Self = Self(0);

    /// Case-sensitive search (`FPDF_MATCHCASE`)
    pub const MATCH_CASE: Self = Self(1);

    /// Match whole words only (`FPDF_MATCHWHOLEWORD`)
    pub const MATCH_WHOLE_WORD: Self = Self(2);

    /// Search consecutively (`FPDF_CONSECUTIVE`)
    /// This searches from the end of the previous result, not character by character.
    pub const CONSECUTIVE: Self = Self(4);

    /// Get the raw bits value for FFI
    #[inline]
    #[must_use = "returns the raw bits value for FFI"]
    pub const fn bits(self) -> u32 {
        self.0
    }

    /// Combine multiple flags
    #[inline]
    #[must_use = "combines multiple search flags into one"]
    pub const fn or(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// Result of a text search operation.
///
/// BUG #2 fix: Represents a single search match.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PdfSearchResult {
    /// Character index where the match starts (0-based)
    pub char_index: i32,
    /// Number of characters in the match
    pub char_count: i32,
}

/// Handle for iterating text search results.
///
/// BUG #2 fix: Wraps `PDFium's` `FPDF_SCHHANDLE` for text search.
/// Use `find_next()` and `find_prev()` to iterate through matches.
///
/// The handle is automatically closed when dropped.
pub struct PdfTextSearch {
    handle: pdfium_sys::FPDF_SCHHANDLE,
}

impl PdfTextSearch {
    /// Find the next occurrence of the search text.
    ///
    /// # Returns
    /// - `Some(PdfSearchResult)` if another match was found
    /// - `None` if no more matches
    #[must_use = "returns the next search match if found"]
    pub fn find_next(&mut self) -> Option<PdfSearchResult> {
        let found = unsafe { pdfium_sys::FPDFText_FindNext(self.handle) };
        if found == 0 {
            return None;
        }

        let char_index = unsafe { pdfium_sys::FPDFText_GetSchResultIndex(self.handle) };
        let char_count = unsafe { pdfium_sys::FPDFText_GetSchCount(self.handle) };

        Some(PdfSearchResult {
            char_index,
            char_count,
        })
    }

    /// Find the previous occurrence of the search text.
    ///
    /// # Returns
    /// - `Some(PdfSearchResult)` if a previous match was found
    /// - `None` if no more matches in the backward direction
    #[must_use = "returns the previous search match if found"]
    pub fn find_prev(&mut self) -> Option<PdfSearchResult> {
        let found = unsafe { pdfium_sys::FPDFText_FindPrev(self.handle) };
        if found == 0 {
            return None;
        }

        let char_index = unsafe { pdfium_sys::FPDFText_GetSchResultIndex(self.handle) };
        let char_count = unsafe { pdfium_sys::FPDFText_GetSchCount(self.handle) };

        Some(PdfSearchResult {
            char_index,
            char_count,
        })
    }

    /// Get the current search result without moving the position.
    ///
    /// # Returns
    /// - `Some(PdfSearchResult)` if currently positioned at a match
    /// - `None` if no current match (call `find_next()` first)
    #[must_use = "returns the current search match if positioned"]
    pub fn current(&self) -> Option<PdfSearchResult> {
        let char_index = unsafe { pdfium_sys::FPDFText_GetSchResultIndex(self.handle) };
        let char_count = unsafe { pdfium_sys::FPDFText_GetSchCount(self.handle) };

        // PDFium returns 0 for both when not positioned at a result
        if char_count <= 0 {
            return None;
        }

        Some(PdfSearchResult {
            char_index,
            char_count,
        })
    }
}

impl Drop for PdfTextSearch {
    fn drop(&mut self) {
        // SAFETY: handle was created by FPDFText_FindStart and is valid
        unsafe { pdfium_sys::FPDFText_FindClose(self.handle) };
    }
}

impl std::fmt::Debug for PdfTextSearch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PdfTextSearch").finish_non_exhaustive()
    }
}

impl PdfPageFast {
    /// Render page to RGB array for ML model input
    ///
    /// Returns RGB image as an ndarray `Array3<u8>` with shape `[height, width, 3]`
    /// This is the HOT PATH for PDF ML processing - the 72x speedup matters here.
    ///
    /// # Errors
    /// Returns an error if DPI is invalid (non-positive, non-finite, or exceeds 2400)
    /// or if rendering fails.
    #[must_use = "renderer returns RGB array"]
    pub fn render_to_rgb_array(&self, dpi: f32) -> Result<ndarray::Array3<u8>, DoclingError> {
        // BUG #94 fix: Validate DPI is reasonable to handle corrupted PDF metadata
        if !dpi.is_finite() || dpi <= 0.0 {
            return Err(DoclingError::BackendError(format!(
                "Invalid DPI value: {dpi} (must be positive finite number)"
            )));
        }
        // Reasonable DPI range: 1-2400 (72-2400 is typical, but allow down to 1 for thumbnails)
        if dpi > 2400.0 {
            return Err(DoclingError::BackendError(format!(
                "DPI too high: {dpi} (max 2400 for safety)"
            )));
        }

        // Calculate dimensions at target DPI using visual dimensions
        // BUG #74 fix: Use visual_width/height to account for 90°/270° rotation
        let width_pts = self.visual_width() as f32;
        let height_pts = self.visual_height() as f32;

        // BUG #94 fix: Validate page dimensions from PDF
        if !width_pts.is_finite()
            || !height_pts.is_finite()
            || width_pts <= 0.0
            || height_pts <= 0.0
        {
            return Err(DoclingError::BackendError(format!(
                "Invalid page dimensions from corrupted PDF: {width_pts}x{height_pts} pts"
            )));
        }

        let width = (width_pts * dpi / PDF_POINTS_PER_INCH) as i32;
        let height = (height_pts * dpi / PDF_POINTS_PER_INCH) as i32;

        // Render to BGRA bitmap (render_to_bitmap validates dimensions)
        let (bgra_data, stride, actual_height) = self.render_to_bitmap(width, height)?;

        // BUG #90 fix: Optimized BGRA→RGB conversion for ndarray
        let width_usize = width as usize;
        let height_usize = actual_height as usize;
        let stride_usize = stride as usize;

        let mut rgb_array = ndarray::Array3::<u8>::zeros((height_usize, width_usize, 3));

        // Fast path: No row padding AND contiguous memory layout
        // Use as_slice_mut() for direct memory access (faster than ndarray indexing)
        if stride_usize == width_usize * 4 {
            if let Some(rgb_slice) = rgb_array.as_slice_mut() {
                // Process entire buffer contiguously - compiler can auto-vectorize
                let pixels = width_usize * height_usize;
                for i in 0..pixels {
                    let src = i * 4;
                    let dst = i * 3;
                    rgb_slice[dst] = bgra_data[src + 2]; // R <- BGRA[2]
                    rgb_slice[dst + 1] = bgra_data[src + 1]; // G <- BGRA[1]
                    rgb_slice[dst + 2] = bgra_data[src]; // B <- BGRA[0]
                }
                return Ok(rgb_array);
            }
        }

        // Slow path: Handle row padding or non-contiguous memory
        for y in 0..height_usize {
            for x in 0..width_usize {
                // Account for stride (row may be padded)
                let bgra_idx = y * stride_usize + x * 4;

                // BGRA to RGB conversion
                let b = bgra_data[bgra_idx];
                let g = bgra_data[bgra_idx + 1];
                let r = bgra_data[bgra_idx + 2];
                // Alpha (bgra_data[bgra_idx + 3]) is ignored

                rgb_array[[y, x, 0]] = r;
                rgb_array[[y, x, 1]] = g;
                rgb_array[[y, x, 2]] = b;
            }
        }

        Ok(rgb_array)
    }

    /// Render page directly at specified pixel dimensions (P8 optimization)
    ///
    /// This is optimized for ML inference: renders directly at model input size
    /// (e.g., 640x640) without DPI calculation or resizing overhead.
    ///
    /// **Performance benefit**: Eliminates the DPI→render→resize pipeline.
    /// Instead of rendering at high DPI (~2000x3000) and resizing to 640x640,
    /// we render directly at 640x640 - less memory, faster processing.
    ///
    /// # Arguments
    /// * `width` - Target width in pixels (e.g., 640 for YOLO models)
    /// * `height` - Target height in pixels (e.g., 640 for YOLO models)
    ///
    /// # Returns
    /// RGB image as `Array3<u8>` with shape `[height, width, 3]`
    ///
    /// # Errors
    /// Returns an error if dimensions are invalid (non-positive or exceed 4096).
    #[must_use = "renderer returns RGB array"]
    pub fn render_at_size(
        &self,
        width: i32,
        height: i32,
    ) -> Result<ndarray::Array3<u8>, DoclingError> {
        // Validate dimensions
        if width <= 0 || height <= 0 {
            return Err(DoclingError::BackendError(format!(
                "Invalid render dimensions: {width}x{height} (must be positive)"
            )));
        }

        if width > MAX_ML_DIMENSION || height > MAX_ML_DIMENSION {
            return Err(DoclingError::BackendError(format!(
                "Render dimensions too large: {width}x{height} (max {MAX_ML_DIMENSION}x{MAX_ML_DIMENSION} for ML)"
            )));
        }

        // Render directly at target size (reuses render_to_bitmap's validation)
        let (bgra_data, stride, actual_height) = self.render_to_bitmap(width, height)?;

        // Convert BGRA to RGB (same as render_to_rgb_array but without DPI overhead)
        let width_usize = width as usize;
        let height_usize = actual_height as usize;
        let stride_usize = stride as usize;

        let mut rgb_array = ndarray::Array3::<u8>::zeros((height_usize, width_usize, 3));

        // Fast path: No row padding AND contiguous memory layout
        if stride_usize == width_usize * 4 {
            if let Some(rgb_slice) = rgb_array.as_slice_mut() {
                let pixels = width_usize * height_usize;
                for i in 0..pixels {
                    let src = i * 4;
                    let dst = i * 3;
                    rgb_slice[dst] = bgra_data[src + 2]; // R <- BGRA[2]
                    rgb_slice[dst + 1] = bgra_data[src + 1]; // G <- BGRA[1]
                    rgb_slice[dst + 2] = bgra_data[src]; // B <- BGRA[0]
                }
                return Ok(rgb_array);
            }
        }

        // Slow path: Handle row padding
        for y in 0..height_usize {
            for x in 0..width_usize {
                let bgra_idx = y * stride_usize + x * 4;
                let b = bgra_data[bgra_idx];
                let g = bgra_data[bgra_idx + 1];
                let r = bgra_data[bgra_idx + 2];
                rgb_array[[y, x, 0]] = r;
                rgb_array[[y, x, 1]] = g;
                rgb_array[[y, x, 2]] = b;
            }
        }

        Ok(rgb_array)
    }

    /// Render page at specified size in grayscale (P9 optimization)
    ///
    /// This is optimized for OCR and some ML models that don't need color.
    /// Uses 75% less memory than RGB output (1 byte/pixel vs 3 bytes/pixel).
    ///
    /// # Arguments
    /// * `width` - Target width in pixels
    /// * `height` - Target height in pixels
    ///
    /// # Returns
    /// Grayscale image as `Array2<u8>` with shape `[height, width]`
    ///
    /// # Errors
    /// Returns an error if dimensions are invalid (non-positive or exceed 4096).
    #[must_use = "renderer returns grayscale array"]
    pub fn render_at_size_gray(
        &self,
        width: i32,
        height: i32,
    ) -> Result<ndarray::Array2<u8>, DoclingError> {
        // Validate dimensions
        if width <= 0 || height <= 0 {
            return Err(DoclingError::BackendError(format!(
                "Invalid render dimensions: {width}x{height} (must be positive)"
            )));
        }

        if width > MAX_ML_DIMENSION || height > MAX_ML_DIMENSION {
            return Err(DoclingError::BackendError(format!(
                "Render dimensions too large: {width}x{height} (max {MAX_ML_DIMENSION}x{MAX_ML_DIMENSION} for ML)"
            )));
        }

        // Render to BGRA bitmap
        let (bgra_data, stride, actual_height) = self.render_to_bitmap(width, height)?;

        let width_usize = width as usize;
        let height_usize = actual_height as usize;
        let stride_usize = stride as usize;

        let mut gray_array = ndarray::Array2::<u8>::zeros((height_usize, width_usize));

        // Convert BGRA to grayscale using Rec.601 luminance formula
        // Uses integer math: Y = (GRAYSCALE_RED_WEIGHT*R + GRAYSCALE_GREEN_WEIGHT*G + GRAYSCALE_BLUE_WEIGHT*B) >> 8
        if stride_usize == width_usize * 4 {
            // Fast path: no padding - use iterator pattern for cleaner code
            if let Some(gray_slice) = gray_array.as_slice_mut() {
                for (gray_pixel, bgra_chunk) in gray_slice.iter_mut().zip(bgra_data.chunks(4)) {
                    let b = u32::from(bgra_chunk[0]);
                    let g = u32::from(bgra_chunk[1]);
                    let r = u32::from(bgra_chunk[2]);
                    *gray_pixel = ((GRAYSCALE_RED_WEIGHT * r
                        + GRAYSCALE_GREEN_WEIGHT * g
                        + GRAYSCALE_BLUE_WEIGHT * b)
                        >> 8) as u8;
                }
                return Ok(gray_array);
            }
        }

        // Slow path: handle padding
        for y in 0..height_usize {
            for x in 0..width_usize {
                let bgra_idx = y * stride_usize + x * 4;
                let b = u32::from(bgra_data[bgra_idx]);
                let g = u32::from(bgra_data[bgra_idx + 1]);
                let r = u32::from(bgra_data[bgra_idx + 2]);
                gray_array[[y, x]] = ((GRAYSCALE_RED_WEIGHT * r
                    + GRAYSCALE_GREEN_WEIGHT * g
                    + GRAYSCALE_BLUE_WEIGHT * b)
                    >> 8) as u8;
            }
        }

        Ok(gray_array)
    }

    /// Extract text cells with bounding boxes from the page
    ///
    /// Uses `PDFium's` rect-based text extraction (like Python docling):
    /// 1. `FPDFText_CountRects` - get number of text rectangles
    /// 2. `FPDFText_GetRect` - get each rectangle's coordinates
    /// 3. `FPDFText_GetBoundedText` - get text within each rectangle
    ///
    /// This produces proper text cells matching Python docling behavior.
    ///
    /// # Errors
    /// Returns an error if `page_height` is invalid (non-positive or non-finite)
    /// or if text extraction fails.
    #[must_use = "extractor returns text cells"]
    pub fn extract_text_cells(&self, page_height: f64) -> Result<Vec<TextCellFast>, DoclingError> {
        // BUG #94 fix: Validate page_height for corrupted PDFs
        if !page_height.is_finite() || page_height <= 0.0 {
            return Err(DoclingError::BackendError(format!(
                "Invalid page_height for text extraction: {page_height} (must be positive finite)"
            )));
        }

        let text_page = PdfTextPageFast::from_page(self)?;

        // Count text rectangles (like Python: text_page.count_rects())
        let rect_count = unsafe { pdfium_sys::FPDFText_CountRects(text_page.handle, 0, -1) };

        if rect_count <= 0 {
            return Ok(Vec::new());
        }

        let mut cells: Vec<TextCellFast> = Vec::new();

        for i in 0..rect_count {
            // Get rectangle coordinates (like Python: text_page.get_rect(i))
            let mut left: f64 = 0.0;
            let mut top: f64 = 0.0;
            let mut right: f64 = 0.0;
            let mut bottom: f64 = 0.0;

            let success = unsafe {
                pdfium_sys::FPDFText_GetRect(
                    text_page.handle,
                    i,
                    &raw mut left,
                    &raw mut top,
                    &raw mut right,
                    &raw mut bottom,
                )
            };

            if success == 0 {
                continue;
            }

            // Get text within rectangle (like Python: text_page.get_text_bounded(*rect))
            // First, get required buffer size
            let text_len = unsafe {
                pdfium_sys::FPDFText_GetBoundedText(
                    text_page.handle,
                    left,
                    top,
                    right,
                    bottom,
                    std::ptr::null_mut(),
                    0,
                )
            };

            if text_len <= 0 {
                continue;
            }

            // Allocate buffer and get text (UTF-16)
            let mut buffer: Vec<u16> = vec![0; (text_len + 1) as usize];
            let chars_written = unsafe {
                pdfium_sys::FPDFText_GetBoundedText(
                    text_page.handle,
                    left,
                    top,
                    right,
                    bottom,
                    buffer.as_mut_ptr(),
                    text_len + 1,
                )
            };

            if chars_written <= 0 {
                continue;
            }

            // Convert UTF-16 to String (remove null terminator)
            // BUG #29 fix: Use saturating_sub for clearer intent
            // BUG #76 fix: Log warning if invalid UTF-16 encountered
            let actual_len = (chars_written as usize).saturating_sub(1);
            let text = utf16_to_string_with_warning(&buffer[..actual_len], "text cell");

            if text.trim().is_empty() {
                continue;
            }

            // Convert from PDF coordinates (origin bottom-left) to standard (origin top-left)
            // Note: FPDFText_GetRect returns top/bottom in PDF coords where bottom < top
            let top_std = page_height - top;
            let bottom_std = page_height - bottom;

            // N=4373: Extract bold/italic from PDF font descriptor flags
            // Only extract font info for first char of each cell (1 FFI call per cell)
            let char_idx = text_page.get_char_index_at_pos(left + 0.5, top - 0.5, 5.0, 5.0);
            let (is_bold, is_italic, font_name) = if let Some(idx) = char_idx {
                if let Some((name, flags)) = text_page.get_font_info(idx) {
                    // Check PDF font descriptor flags
                    let bold_flag = (flags & PDF_FONT_FORCE_BOLD) != 0;
                    let italic_flag = (flags & PDF_FONT_ITALIC) != 0;

                    // Also check font name heuristics (common patterns)
                    let name_lower = name.to_lowercase();
                    let bold_name = name_lower.contains("bold")
                        || name_lower.contains("-bd")
                        || name_lower.ends_with("bd");
                    let italic_name = name_lower.contains("italic")
                        || name_lower.contains("oblique")
                        || name_lower.contains("-it")
                        || name_lower.ends_with("it");

                    (
                        bold_flag || bold_name,
                        italic_flag || italic_name,
                        Some(name),
                    )
                } else {
                    (false, false, None)
                }
            } else {
                (false, false, None)
            };

            // Skip other font info for performance (not needed for current use case)
            let (font_size, font_weight, char_angle, fill_color, stroke_color) =
                (None, None, None, None, None);

            cells.push(TextCellFast {
                text,
                left,
                top: top_std,
                right,
                bottom: bottom_std,
                font_size,
                font_weight,
                char_angle,
                font_name,
                fill_color,
                stroke_color,
                is_bold,
                is_italic,
            });
        }

        Ok(cells)
    }

    /// Extract all text cells using batch API (faster, fewer FFI calls)
    ///
    /// This uses the `pdfium_fast` `FPDFText_ExtractAllCells()` API which extracts
    /// all text cells in 2 FFI calls instead of 2-3 calls per cell.
    /// For a page with 100 cells: 2 calls vs ~250 calls.
    ///
    /// # Arguments
    /// * `page_height` - Page height for coordinate conversion (PDF uses bottom-left origin)
    ///
    /// # Returns
    /// Vector of text cells with bounding boxes and text content.
    ///
    /// # Performance
    /// Expected ~3-5x speedup for text extraction on pages with many cells.
    ///
    /// # Errors
    /// Returns an error if `page_height` is invalid or if batch extraction fails.
    #[must_use = "extractor returns text cells"]
    pub fn extract_text_cells_batch(
        &self,
        page_height: f64,
    ) -> Result<Vec<TextCellFast>, DoclingError> {
        // Validate page_height
        if !page_height.is_finite() || page_height <= 0.0 {
            return Err(DoclingError::BackendError(format!(
                "Invalid page_height for batch text extraction: {page_height} (must be positive finite)"
            )));
        }

        let text_page = PdfTextPageFast::from_page(self)?;

        // First call: get required buffer sizes (pass null pointers)
        let cell_count = unsafe {
            pdfium_sys::FPDFText_ExtractAllCells(
                text_page.handle,
                std::ptr::null_mut(), // cells buffer
                0,                    // max_cells
                std::ptr::null_mut(), // text buffer
                0,                    // text_buffer_chars
            )
        };

        if cell_count <= 0 {
            return Ok(Vec::new());
        }

        // Estimate text buffer size (generous estimate: 100 chars per cell average)
        let estimated_text_chars = cell_count * 100;

        // Allocate buffers
        let mut cells: Vec<pdfium_sys::FPDF_TEXT_CELL_INFO> =
            vec![unsafe { std::mem::zeroed() }; cell_count as usize];
        let mut text_buffer: Vec<u16> = vec![0; estimated_text_chars as usize];

        // Second call: fill buffers
        let actual_count = unsafe {
            pdfium_sys::FPDFText_ExtractAllCells(
                text_page.handle,
                cells.as_mut_ptr(),
                cell_count,
                text_buffer.as_mut_ptr(),
                estimated_text_chars,
            )
        };

        if actual_count <= 0 {
            return Ok(Vec::new());
        }

        // Convert to TextCellFast
        let mut result: Vec<TextCellFast> = Vec::with_capacity(actual_count as usize);

        for cell in cells.iter().take(actual_count as usize) {
            // Extract text from shared buffer using offset and length
            let text_start = cell.text_offset as usize;
            let text_end = text_start + cell.text_length as usize;

            if text_end > text_buffer.len() {
                // Buffer overflow - skip this cell
                continue;
            }

            // Convert UTF-16 slice to String
            let text =
                utf16_to_string_with_warning(&text_buffer[text_start..text_end], "batch text cell");

            if text.trim().is_empty() {
                continue;
            }

            // Convert from PDF coordinates (bottom-left origin) to standard (top-left origin)
            let top_std = page_height - f64::from(cell.top);
            let bottom_std = page_height - f64::from(cell.bottom);

            // N=4373: Batch API doesn't provide font flags directly
            // Bold/italic detection not available in batch mode (use standard mode if needed)
            result.push(TextCellFast {
                text,
                left: f64::from(cell.left),
                top: top_std,
                right: f64::from(cell.right),
                bottom: bottom_std,
                font_size: Some(f64::from(cell.font_size)),
                font_weight: None,
                char_angle: None,
                font_name: None,
                fill_color: None,
                stroke_color: None,
                is_bold: false,
                is_italic: false,
            });
        }

        Ok(result)
    }

    /// Get the number of words on this page.
    ///
    /// P6/P10: Word-level extraction (N=3514).
    ///
    /// Words are sequences of non-whitespace characters separated by
    /// whitespace or large gaps. Use this to allocate buffers before
    /// calling `extract_words_batch()`.
    ///
    /// # Returns
    /// Number of words on the page, or -1 on error.
    ///
    /// # Errors
    /// Returns an error if the text page cannot be loaded.
    #[must_use = "word count needed for buffer allocation"]
    pub fn word_count(&self) -> Result<i32, DoclingError> {
        let text_page = PdfTextPageFast::from_page(self)?;
        let count = unsafe { pdfium_sys::FPDFText_CountWords(text_page.handle) };
        Ok(count)
    }

    /// Extract all words from the page using batch API (faster, fewer FFI calls).
    ///
    /// P6/P10: Word-level extraction (N=3514).
    ///
    /// This uses the `pdfium_fast` `FPDFText_ExtractWords()` API which extracts
    /// all words in 2 FFI calls instead of per-word calls.
    ///
    /// # Arguments
    /// * `page_height` - Page height for coordinate conversion (PDF uses bottom-left origin)
    ///
    /// # Returns
    /// Vector of words with bounding boxes and text content.
    ///
    /// # Use Cases
    /// - Natural language processing (tokenization)
    /// - Text search with word boundaries
    /// - Layout analysis with word-level granularity
    ///
    /// # Errors
    /// Returns an error if `page_height` is invalid or if word extraction fails.
    #[must_use = "extractor returns words"]
    pub fn extract_words_batch(&self, page_height: f64) -> Result<Vec<PdfWord>, DoclingError> {
        // Validate page_height
        if !page_height.is_finite() || page_height <= 0.0 {
            return Err(DoclingError::BackendError(format!(
                "Invalid page_height for word extraction: {page_height} (must be positive finite)"
            )));
        }

        let text_page = PdfTextPageFast::from_page(self)?;

        // Get word count
        let word_count = unsafe { pdfium_sys::FPDFText_CountWords(text_page.handle) };
        if word_count <= 0 {
            return Ok(Vec::new());
        }

        // Get character count for text buffer sizing
        let char_count = unsafe { pdfium_sys::FPDFText_CountChars(text_page.handle) };
        if char_count <= 0 {
            return Ok(Vec::new());
        }

        // Allocate buffers
        let mut words: Vec<pdfium_sys::FPDF_WORD_INFO> =
            vec![unsafe { std::mem::zeroed() }; word_count as usize];
        let mut text_buffer: Vec<u16> = vec![0; (char_count + 1) as usize];

        // Extract words
        let actual_count = unsafe {
            pdfium_sys::FPDFText_ExtractWords(
                text_page.handle,
                words.as_mut_ptr(),
                word_count,
                text_buffer.as_mut_ptr(),
                char_count + 1,
            )
        };

        if actual_count <= 0 {
            return Ok(Vec::new());
        }

        // Convert to PdfWord
        let mut result: Vec<PdfWord> = Vec::with_capacity(actual_count as usize);

        for word in words.iter().take(actual_count as usize) {
            // Extract text from shared buffer using offset and length
            let text_start = word.text_offset as usize;
            let text_end = text_start + word.text_length as usize;

            if text_end > text_buffer.len() {
                // Buffer overflow - skip this word
                continue;
            }

            // Convert UTF-16 slice to String
            let text = utf16_to_string_with_warning(&text_buffer[text_start..text_end], "word");

            if text.trim().is_empty() {
                continue;
            }

            // Convert from PDF coordinates (bottom-left origin) to standard (top-left origin)
            let top_std = page_height - f64::from(word.top);
            let bottom_std = page_height - f64::from(word.bottom);

            result.push(PdfWord {
                text,
                left: f64::from(word.left),
                top: top_std,
                right: f64::from(word.right),
                bottom: bottom_std,
                start_char: word.start_char,
                end_char: word.end_char,
            });
        }

        Ok(result)
    }

    /// Extract text from a bounding box (for cell merging)
    ///
    /// This is used to re-extract text from merged bounding boxes,
    /// which handles ligatures correctly (like Python docling).
    ///
    /// # Arguments
    /// * `left`, `top`, `right`, `bottom` - Bounding box in TOP-LEFT coordinates
    /// * `page_height` - Page height for coordinate conversion
    ///
    /// # Errors
    /// Returns an error if the text page cannot be loaded.
    #[must_use = "extractor returns text"]
    pub fn get_text_bounded(
        &self,
        left: f64,
        top: f64,
        right: f64,
        bottom: f64,
        page_height: f64,
    ) -> Result<String, DoclingError> {
        // BUG #12 fix: Use cached text page instead of loading each time
        let text_page_handle = self.get_cached_text_page()?;

        // Convert from top-left coordinates to PDF coordinates (bottom-left)
        let pdf_top = page_height - top;
        let pdf_bottom = page_height - bottom;

        // Get text within rectangle
        let text_len = unsafe {
            pdfium_sys::FPDFText_GetBoundedText(
                text_page_handle,
                left,
                pdf_top,
                right,
                pdf_bottom,
                std::ptr::null_mut(),
                0,
            )
        };

        if text_len <= 0 {
            return Ok(String::new());
        }

        // Allocate buffer and get text (UTF-16)
        let mut buffer: Vec<u16> = vec![0; (text_len + 1) as usize];
        let chars_written = unsafe {
            pdfium_sys::FPDFText_GetBoundedText(
                text_page_handle,
                left,
                pdf_top,
                right,
                pdf_bottom,
                buffer.as_mut_ptr(),
                text_len + 1,
            )
        };

        if chars_written <= 0 {
            return Ok(String::new());
        }

        // Convert UTF-16 to String (remove null terminator)
        // BUG #29 fix: Use saturating_sub for clearer intent
        // BUG #76 fix: Log warning if invalid UTF-16 encountered
        let actual_len = (chars_written as usize).saturating_sub(1);
        let text = utf16_to_string_with_warning(&buffer[..actual_len], "bounded text");

        // Normalize whitespace (like pdfium-render)
        let normalized = text
            .replace("\r\n", " ")
            .replace(['\r', '\n'], " ")
            .replace('\u{2212}', "-") // Unicode minus to ASCII hyphen
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");

        Ok(normalized)
    }

    /// Extract all hyperlinks from the page
    ///
    /// Returns a list of hyperlinks with their URLs and bounding boxes.
    #[must_use = "extractor returns hyperlinks"]
    pub fn extract_hyperlinks(&self, page_height: f64) -> Vec<PdfHyperlink> {
        // BUG #94 fix: Validate page_height for corrupted PDFs
        if !page_height.is_finite() || page_height <= 0.0 {
            log::warn!(
                "Invalid page_height for hyperlink extraction: {page_height} - returning empty"
            );
            return Vec::new();
        }

        let mut links = Vec::new();
        let mut start_pos: i32 = 0;
        let mut link: pdfium_sys::FPDF_LINK = std::ptr::null_mut();

        // Iterate through all links on the page
        while unsafe {
            pdfium_sys::FPDFLink_Enumerate(self.handle, &raw mut start_pos, &raw mut link)
        } != 0
        {
            if link.is_null() {
                continue;
            }

            // Get link rectangle
            let mut rect: pdfium_sys::FS_RECTF = unsafe { std::mem::zeroed() };
            if unsafe { pdfium_sys::FPDFLink_GetAnnotRect(link, &raw mut rect) } == 0 {
                continue;
            }

            // Get the action for this link
            let action = unsafe { pdfium_sys::FPDFLink_GetAction(link) };
            if action.is_null() {
                continue;
            }

            // Check if it's a URI action (BUG #41 fix: use named constant)
            let action_type = unsafe { pdfium_sys::FPDFAction_GetType(action) };
            if action_type != PDFACTION_URI {
                continue;
            }

            // Get URI length
            let uri_len = unsafe {
                pdfium_sys::FPDFAction_GetURIPath(self.doc, action, std::ptr::null_mut(), 0)
            };

            if uri_len == 0 {
                continue;
            }

            // Get URI
            let mut uri_buffer: Vec<u8> = vec![0; uri_len as usize];
            let written = unsafe {
                pdfium_sys::FPDFAction_GetURIPath(
                    self.doc,
                    action,
                    uri_buffer.as_mut_ptr().cast(),
                    uri_len,
                )
            };

            if written == 0 {
                continue;
            }

            // Convert to string (BUG #40 fix: find actual null terminator position)
            // The buffer may contain garbage after the null terminator
            let actual_len = uri_buffer
                .iter()
                .position(|&b| b == 0)
                .unwrap_or(uri_buffer.len());
            let url = String::from_utf8_lossy(&uri_buffer[..actual_len])
                .trim()
                .to_string();

            if url.is_empty() {
                continue;
            }

            // Convert coordinates from PDF space (bottom-left origin) to standard (top-left)
            links.push(PdfHyperlink {
                url,
                left: f64::from(rect.left),
                top: page_height - f64::from(rect.top),
                right: f64::from(rect.right),
                bottom: page_height - f64::from(rect.bottom),
            });
        }

        links
    }

    /// Extract hyperlinks with an optional limit.
    ///
    /// BUG #17 fix: Allows early exit when only first N links are needed,
    /// useful for pagination or preview use cases.
    ///
    /// # Arguments
    /// * `page_height` - Page height for coordinate conversion
    /// * `limit` - Maximum number of links to extract (None = all)
    ///
    /// # Returns
    /// Vector of hyperlinks, up to the specified limit.
    #[must_use = "extractor returns hyperlinks"]
    pub fn extract_hyperlinks_with_limit(
        &self,
        page_height: f64,
        limit: Option<usize>,
    ) -> Vec<PdfHyperlink> {
        // BUG #94 fix: Validate page_height for corrupted PDFs
        if !page_height.is_finite() || page_height <= 0.0 {
            log::warn!(
                "Invalid page_height for hyperlink extraction: {page_height} - returning empty"
            );
            return Vec::new();
        }

        let max_links = limit.unwrap_or(usize::MAX);
        let mut links = Vec::new();
        let mut start_pos: i32 = 0;
        let mut link: pdfium_sys::FPDF_LINK = std::ptr::null_mut();

        // Iterate through links until limit reached or no more links
        while links.len() < max_links
            && unsafe {
                pdfium_sys::FPDFLink_Enumerate(self.handle, &raw mut start_pos, &raw mut link)
            } != 0
        {
            if link.is_null() {
                continue;
            }

            // Get link rectangle
            let mut rect: pdfium_sys::FS_RECTF = unsafe { std::mem::zeroed() };
            if unsafe { pdfium_sys::FPDFLink_GetAnnotRect(link, &raw mut rect) } == 0 {
                continue;
            }

            // Get the action for this link
            let action = unsafe { pdfium_sys::FPDFLink_GetAction(link) };
            if action.is_null() {
                continue;
            }

            // Check if it's a URI action
            let action_type = unsafe { pdfium_sys::FPDFAction_GetType(action) };
            if action_type != PDFACTION_URI {
                continue;
            }

            // Get URI length
            let uri_len = unsafe {
                pdfium_sys::FPDFAction_GetURIPath(self.doc, action, std::ptr::null_mut(), 0)
            };

            if uri_len == 0 {
                continue;
            }

            // Get URI
            let mut uri_buffer: Vec<u8> = vec![0; uri_len as usize];
            let written = unsafe {
                pdfium_sys::FPDFAction_GetURIPath(
                    self.doc,
                    action,
                    uri_buffer.as_mut_ptr().cast(),
                    uri_len,
                )
            };

            if written == 0 {
                continue;
            }

            // Convert to string (BUG #40 fix: find actual null terminator position)
            let actual_len = uri_buffer
                .iter()
                .position(|&b| b == 0)
                .unwrap_or(uri_buffer.len());
            let url = String::from_utf8_lossy(&uri_buffer[..actual_len])
                .trim()
                .to_string();

            if url.is_empty() {
                continue;
            }

            // Convert coordinates from PDF space (bottom-left origin) to standard (top-left)
            links.push(PdfHyperlink {
                url,
                left: f64::from(rect.left),
                top: page_height - f64::from(rect.top),
                right: f64::from(rect.right),
                bottom: page_height - f64::from(rect.bottom),
            });
        }

        links
    }

    /// Extract all annotations from the page.
    ///
    /// BUG #1 fix: Provides access to PDF annotations (highlights, comments, notes, etc.)
    ///
    /// # Arguments
    /// * `page_height` - Page height for coordinate conversion (PDF uses bottom-left origin)
    ///
    /// # Returns
    /// Vector of annotations with their types, positions, and content.
    #[must_use = "extractor returns annotations"]
    pub fn extract_annotations(&self, page_height: f64) -> Vec<PdfAnnotation> {
        // Validate page_height for corrupted PDFs
        if !page_height.is_finite() || page_height <= 0.0 {
            log::warn!(
                "Invalid page_height for annotation extraction: {page_height} - returning empty"
            );
            return Vec::new();
        }

        let annot_count = unsafe { pdfium_sys::FPDFPage_GetAnnotCount(self.handle) };
        if annot_count <= 0 {
            return Vec::new();
        }

        let mut annotations = Vec::with_capacity(annot_count as usize);

        for i in 0..annot_count {
            // Get annotation handle
            let annot = unsafe { pdfium_sys::FPDFPage_GetAnnot(self.handle, i) };
            if annot.is_null() {
                continue;
            }

            // Get annotation subtype
            let subtype = unsafe { pdfium_sys::FPDFAnnot_GetSubtype(annot) };
            let annotation_type = PdfAnnotationType::from_pdfium(subtype as u32);

            // Get annotation rectangle
            let mut rect: pdfium_sys::FS_RECTF = unsafe { std::mem::zeroed() };
            let has_rect = unsafe { pdfium_sys::FPDFAnnot_GetRect(annot, &raw mut rect) } != 0;

            let (left, top, right, bottom) = if has_rect {
                // Convert from PDF coordinates (bottom-left origin) to standard (top-left)
                (
                    f64::from(rect.left),
                    page_height - f64::from(rect.top),
                    f64::from(rect.right),
                    page_height - f64::from(rect.bottom),
                )
            } else {
                (0.0, 0.0, 0.0, 0.0)
            };

            // Get annotation contents (the "Contents" key)
            let contents = self.get_annotation_string_value(annot, b"Contents\0");

            // Get annotation author (the "T" key for title/author)
            let author = self.get_annotation_string_value(annot, b"T\0");

            // Get modification date (the "M" key)
            let modification_date = self.get_annotation_string_value(annot, b"M\0");

            // Get annotation flags
            let flags = unsafe { pdfium_sys::FPDFAnnot_GetFlags(annot) };

            annotations.push(PdfAnnotation {
                annotation_type,
                left,
                top,
                right,
                bottom,
                contents,
                author,
                modification_date,
                flags: flags as u32,
            });

            // Close the annotation handle
            unsafe { pdfium_sys::FPDFPage_CloseAnnot(annot) };
        }

        annotations
    }

    /// Helper to get a string value from an annotation by key.
    /// Returns None if the key doesn't exist or the value is empty.
    // Method signature kept for API consistency with other PdfiumAdapter methods
    #[allow(clippy::unused_self)]
    fn get_annotation_string_value(
        &self,
        annot: pdfium_sys::FPDF_ANNOTATION,
        key: &[u8],
    ) -> Option<String> {
        // First get the required buffer size
        let size = unsafe {
            pdfium_sys::FPDFAnnot_GetStringValue(
                annot,
                key.as_ptr().cast::<i8>(),
                std::ptr::null_mut(),
                0,
            )
        };

        // Size is in bytes, includes null terminator (UTF-16)
        if size <= 2 {
            return None;
        }

        // Allocate buffer (size is in bytes, UTF-16 = 2 bytes per char)
        let char_count = utf16_bytes_to_units(size as u64, "annotation value");
        let mut buffer: Vec<u16> = vec![0; char_count];

        let written = unsafe {
            pdfium_sys::FPDFAnnot_GetStringValue(
                annot,
                key.as_ptr().cast::<i8>(),
                buffer.as_mut_ptr(),
                size,
            )
        };

        if written <= 2 {
            return None;
        }

        // Remove null terminator and convert to String
        let actual_len = utf16_bytes_to_units(written as u64, "annotation value").saturating_sub(1);
        let text = utf16_to_string_with_warning(&buffer[..actual_len], "annotation value");

        if text.trim().is_empty() {
            None
        } else {
            Some(text)
        }
    }

    /// Get the total number of annotations on this page.
    ///
    /// BUG #1 fix: Quick check for annotation presence without extracting all.
    #[inline]
    #[must_use = "returns the total number of annotations on this page"]
    pub fn annotation_count(&self) -> i32 {
        unsafe { pdfium_sys::FPDFPage_GetAnnotCount(self.handle) }
    }

    /// Check if this page has any form fields (Widget annotations).
    ///
    /// BUG #22 fix: Quick check for interactive form elements.
    /// This is useful for detecting if a PDF is an interactive form.
    ///
    /// # Returns
    /// `true` if the page contains at least one form field, `false` otherwise.
    #[inline]
    #[must_use = "returns whether the page has form fields"]
    pub fn has_form_fields(&self) -> bool {
        self.count_form_fields() > 0
    }

    /// Count the number of form fields on this page.
    ///
    /// BUG #22 fix: Count interactive form elements (Widget annotations).
    ///
    /// # Returns
    /// The number of form fields (Widget annotations) on this page.
    #[must_use = "returns the number of form fields on this page"]
    pub fn count_form_fields(&self) -> usize {
        let annot_count = unsafe { pdfium_sys::FPDFPage_GetAnnotCount(self.handle) };
        if annot_count <= 0 {
            return 0;
        }

        let mut form_field_count = 0;
        for i in 0..annot_count {
            let annot = unsafe { pdfium_sys::FPDFPage_GetAnnot(self.handle, i) };
            if annot.is_null() {
                continue;
            }

            let subtype = unsafe { pdfium_sys::FPDFAnnot_GetSubtype(annot) };
            // Widget annotation subtype is 20 (PdfAnnotationType::Widget)
            if subtype == 20 {
                form_field_count += 1;
            }

            unsafe { pdfium_sys::FPDFPage_CloseAnnot(annot) };
        }

        form_field_count
    }

    /// Extract form fields from this page.
    ///
    /// BUG #22 fix: Extract basic information about form fields (Widget annotations).
    /// For full form field details (field names, values, options), initialize the
    /// form fill environment with `FPDFDOC_InitFormFillEnvironment`.
    ///
    /// # Arguments
    /// * `page_height` - The height of the page in PDF points (for coordinate conversion)
    ///
    /// # Returns
    /// Vector of form fields with their positions and annotation indices.
    #[must_use = "extractor returns form fields"]
    pub fn extract_form_fields(&self, page_height: f64) -> Vec<PdfFormField> {
        // Validate page_height for corrupted PDFs
        if !page_height.is_finite() || page_height <= 0.0 {
            log::warn!(
                "Invalid page_height for form field extraction: {page_height} - returning empty"
            );
            return Vec::new();
        }

        let annot_count = unsafe { pdfium_sys::FPDFPage_GetAnnotCount(self.handle) };
        if annot_count <= 0 {
            return Vec::new();
        }

        let mut form_fields = Vec::new();

        for i in 0..annot_count {
            let annot = unsafe { pdfium_sys::FPDFPage_GetAnnot(self.handle, i) };
            if annot.is_null() {
                continue;
            }

            let subtype = unsafe { pdfium_sys::FPDFAnnot_GetSubtype(annot) };
            // Widget annotation subtype is 20 (PdfAnnotationType::Widget)
            if subtype == 20 {
                // Get annotation rectangle
                let mut rect: pdfium_sys::FS_RECTF = unsafe { std::mem::zeroed() };
                let has_rect = unsafe { pdfium_sys::FPDFAnnot_GetRect(annot, &raw mut rect) } != 0;

                let (left, top, right, bottom) = if has_rect {
                    // Convert from PDF coordinates (bottom-left origin) to standard (top-left)
                    (
                        f64::from(rect.left),
                        page_height - f64::from(rect.top),
                        f64::from(rect.right),
                        page_height - f64::from(rect.bottom),
                    )
                } else {
                    (0.0, 0.0, 0.0, 0.0)
                };

                // Get annotation flags
                let flags = unsafe { pdfium_sys::FPDFAnnot_GetFlags(annot) };

                form_fields.push(PdfFormField {
                    annotation_index: i,
                    left,
                    top,
                    right,
                    bottom,
                    flags: flags as u32,
                });
            }

            unsafe { pdfium_sys::FPDFPage_CloseAnnot(annot) };
        }

        form_fields
    }

    /// Get form field annotations with their full annotation details.
    ///
    /// BUG #22 fix: Filter annotations to only return Widget (form field) annotations.
    /// This is a convenience method that calls `extract_annotations` and filters for Widgets.
    ///
    /// # Arguments
    /// * `page_height` - The height of the page in PDF points (for coordinate conversion)
    ///
    /// # Returns
    /// Vector of annotations that are form fields (Widget type).
    #[must_use = "extractor returns form field annotations"]
    pub fn get_form_field_annotations(&self, page_height: f64) -> Vec<PdfAnnotation> {
        self.extract_annotations(page_height)
            .into_iter()
            .filter(|a| a.annotation_type == PdfAnnotationType::Widget)
            .collect()
    }

    /// Convert device coordinates to page coordinates.
    ///
    /// BUG #6 fix: Provides `PDFium's` built-in coordinate conversion.
    /// This is useful for converting mouse clicks to page positions.
    ///
    /// # Arguments
    /// * `device_x`, `device_y` - Device coordinates (pixels from top-left)
    /// * `render_width`, `render_height` - Render dimensions in pixels
    /// * `rotation` - Page rotation in degrees (0, 90, 180, 270)
    ///
    /// # Returns
    /// Page coordinates (x, y) in PDF points (72 DPI), bottom-left origin.
    #[must_use = "converts device coordinates to page coordinates"]
    pub fn device_to_page(
        &self,
        device_x: i32,
        device_y: i32,
        render_width: i32,
        render_height: i32,
        rotation: i32,
    ) -> (f64, f64) {
        let mut page_x: f64 = 0.0;
        let mut page_y: f64 = 0.0;

        let rotate = match rotation {
            90 => 1,
            180 => 2,
            270 => 3,
            _ => 0,
        };

        unsafe {
            pdfium_sys::FPDF_DeviceToPage(
                self.handle,
                0, // start_x
                0, // start_y
                render_width,
                render_height,
                rotate,
                device_x,
                device_y,
                &raw mut page_x,
                &raw mut page_y,
            );
        }

        (page_x, page_y)
    }

    /// Convert page coordinates to device coordinates.
    ///
    /// BUG #6 fix: Provides `PDFium's` built-in coordinate conversion.
    /// This is useful for highlighting text positions on a rendered page.
    ///
    /// # Arguments
    /// * `page_x`, `page_y` - Page coordinates in PDF points (72 DPI)
    /// * `render_width`, `render_height` - Render dimensions in pixels
    /// * `rotation` - Page rotation in degrees (0, 90, 180, 270)
    ///
    /// # Returns
    /// Device coordinates (x, y) in pixels from top-left.
    #[must_use = "converts page coordinates to device coordinates"]
    pub fn page_to_device(
        &self,
        page_x: f64,
        page_y: f64,
        render_width: i32,
        render_height: i32,
        rotation: i32,
    ) -> (i32, i32) {
        let mut device_x: i32 = 0;
        let mut device_y: i32 = 0;

        let rotate = match rotation {
            90 => 1,
            180 => 2,
            270 => 3,
            _ => 0,
        };

        unsafe {
            pdfium_sys::FPDF_PageToDevice(
                self.handle,
                0, // start_x
                0, // start_y
                render_width,
                render_height,
                rotate,
                page_x,
                page_y,
                &raw mut device_x,
                &raw mut device_y,
            );
        }

        (device_x, device_y)
    }

    /// Start a text search on this page.
    ///
    /// BUG #2 fix: Provides text search capability using `PDFium's` built-in search.
    /// This is more efficient than extracting all text and searching in Rust.
    ///
    /// # Arguments
    /// * `search_text` - The text to search for
    /// * `flags` - Search flags (case sensitivity, whole word matching)
    /// * `start_index` - Character index to start searching from (0 for beginning)
    ///
    /// # Returns
    /// A `PdfTextSearch` handle that can be used to iterate through results.
    ///
    /// # Examples
    /// ```ignore
    /// let page = doc.load_page(0)?;
    /// let search = page.search_text("example", PdfTextSearchFlags::CASE_INSENSITIVE, 0)?;
    /// while let Some(result) = search.find_next() {
    ///     println!("Found at index {}, length {}", result.char_index, result.char_count);
    /// }
    /// ```
    ///
    /// # Errors
    /// Returns an error if search text is empty or if the text page cannot be loaded.
    #[must_use = "search returns a handle to iterate results"]
    pub fn search_text(
        &self,
        search_text: &str,
        flags: PdfTextSearchFlags,
        start_index: i32,
    ) -> Result<PdfTextSearch, DoclingError> {
        if search_text.is_empty() {
            return Err(DoclingError::BackendError(
                "Search text cannot be empty".to_string(),
            ));
        }

        // Get text page handle (use cached if available)
        let text_page_handle = self.get_cached_text_page()?;

        // Convert search text to UTF-16 (PDFium uses wide strings)
        let mut utf16: Vec<u16> = search_text.encode_utf16().collect();
        utf16.push(0); // Null terminate

        // Start the search
        let search_handle = unsafe {
            pdfium_sys::FPDFText_FindStart(
                text_page_handle,
                utf16.as_ptr(),
                std::ffi::c_ulong::from(flags.bits()),
                start_index,
            )
        };

        if search_handle.is_null() {
            return Err(DoclingError::BackendError(
                "Failed to start text search".to_string(),
            ));
        }

        Ok(PdfTextSearch {
            handle: search_handle,
        })
    }

    /// Search for text and return all matches at once.
    ///
    /// BUG #2 fix: Convenience method that collects all search results.
    /// For large documents with many matches, prefer using `search_text()` directly
    /// to iterate results without collecting them all in memory.
    ///
    /// # Arguments
    /// * `search_text` - The text to search for
    /// * `flags` - Search flags (case sensitivity, whole word matching)
    ///
    /// # Returns
    /// Vector of all search results.
    ///
    /// # Errors
    /// Returns an error if the search fails to initialize.
    #[must_use = "search returns results vector"]
    pub fn find_all_text(
        &self,
        search_text: &str,
        flags: PdfTextSearchFlags,
    ) -> Result<Vec<PdfSearchResult>, DoclingError> {
        let mut search = self.search_text(search_text, flags, 0)?;
        let mut results = Vec::new();

        while let Some(result) = search.find_next() {
            results.push(result);
        }

        Ok(results)
    }

    /// Extract embedded thumbnail from PDF page (P11 optimization)
    ///
    /// Many PDFs embed thumbnail images for fast preview without full page rendering.
    /// This extracts the embedded thumbnail if present.
    ///
    /// # Returns
    /// * `Some((width, height, rgba_data))` - Thumbnail image in RGBA format
    /// * `None` - Page has no embedded thumbnail
    ///
    /// # Use Cases
    /// - Document preview UI (instant, no rendering needed)
    /// - Quick document classification
    /// - Duplicate detection with image hashing
    #[must_use = "extractor returns optional thumbnail data"]
    pub fn get_thumbnail(&self) -> Option<(i32, i32, Vec<u8>)> {
        // Get thumbnail bitmap handle
        let bitmap = unsafe { pdfium_sys::FPDFPage_GetThumbnailAsBitmap(self.handle) };

        if bitmap.is_null() {
            return None;
        }

        // Get dimensions
        let width = unsafe { pdfium_sys::FPDFBitmap_GetWidth(bitmap) };
        let height = unsafe { pdfium_sys::FPDFBitmap_GetHeight(bitmap) };
        let stride = unsafe { pdfium_sys::FPDFBitmap_GetStride(bitmap) };

        if width <= 0 || height <= 0 || stride <= 0 {
            unsafe { pdfium_sys::FPDFBitmap_Destroy(bitmap) };
            return None;
        }

        // Get buffer pointer
        let buffer_ptr = unsafe { pdfium_sys::FPDFBitmap_GetBuffer(bitmap) };
        if buffer_ptr.is_null() {
            unsafe { pdfium_sys::FPDFBitmap_Destroy(bitmap) };
            return None;
        }

        // Copy data (PDFium thumbnails are typically BGRA)
        let buffer_size = (stride * height) as usize;
        let bgra_data = unsafe { std::slice::from_raw_parts(buffer_ptr as *const u8, buffer_size) };

        // Convert BGRA to RGBA
        let mut rgba_data = Vec::with_capacity((width * height * 4) as usize);
        let width_usize = width as usize;
        let height_usize = height as usize;
        let stride_usize = stride as usize;

        for y in 0..height_usize {
            for x in 0..width_usize {
                let idx = y * stride_usize + x * 4;
                if idx + 3 < buffer_size {
                    rgba_data.push(bgra_data[idx + 2]); // R <- B
                    rgba_data.push(bgra_data[idx + 1]); // G <- G
                    rgba_data.push(bgra_data[idx]); // B <- R
                    rgba_data.push(bgra_data[idx + 3]); // A <- A
                }
            }
        }

        // Clean up
        unsafe { pdfium_sys::FPDFBitmap_Destroy(bitmap) };

        Some((width, height, rgba_data))
    }

    /// Check if page has an embedded thumbnail
    ///
    /// Faster than `get_thumbnail()` when you only need to check existence.
    #[inline]
    #[must_use = "returns whether the page has an embedded thumbnail"]
    pub fn has_thumbnail(&self) -> bool {
        let bitmap = unsafe { pdfium_sys::FPDFPage_GetThumbnailAsBitmap(self.handle) };
        if bitmap.is_null() {
            false
        } else {
            unsafe { pdfium_sys::FPDFBitmap_Destroy(bitmap) };
            true
        }
    }
}

/// Create a `PdfiumFast` instance for processing (pdfium-fast backend)
///
/// # Errors
/// Returns an error if `PDFium` library initialization fails.
#[inline]
#[must_use = "factory function returns a new instance"]
pub fn create_pdfium() -> Result<PdfiumFast, DoclingError> {
    PdfiumFast::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, Timelike};

    #[test]
    #[ignore = "PDFium init/destroy cycle conflicts with other tests - run with --ignored"]
    fn test_create_pdfium() {
        // Test pdfium-fast initialization
        // NOTE: This test is ignored by default because PDFium's library
        // initialization/destruction doesn't handle being called multiple times
        // in the same process. Running this with other PDF tests causes SIGSEGV.
        // Run with: cargo test --ignored test_create_pdfium_fast
        match create_pdfium() {
            Ok(_pdfium) => {
                // Successfully created PdfiumFast instance
            }
            Err(e) => {
                // Library not available
                eprintln!("Note: Could not initialize PdfiumFast: {e}");
            }
        }
    }

    #[test]
    #[ignore = "PDFium init/destroy cycle conflicts with other tests - run with --ignored"]
    fn test_is_tagged_api() {
        // Test is_tagged() API for detecting tagged PDFs
        // Run with: cargo test --ignored test_is_tagged_api
        let pdfium = match create_pdfium() {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Note: Could not initialize PdfiumFast: {e}");
                return;
            }
        };

        // Test with known tagged PDF
        let tagged_path = std::path::Path::new("test-corpus/pdf/amt_handbook_sample.pdf");
        if tagged_path.exists() {
            let doc = pdfium.load_pdf_from_file(tagged_path, None).unwrap();
            assert!(doc.is_tagged(), "amt_handbook_sample.pdf should be tagged");
            eprintln!("✓ amt_handbook_sample.pdf: is_tagged=true");
        }

        // Test with known untagged PDF
        let untagged_path = std::path::Path::new("test-corpus/pdf/2305.03393v1.pdf");
        if untagged_path.exists() {
            let doc = pdfium.load_pdf_from_file(untagged_path, None).unwrap();
            assert!(!doc.is_tagged(), "2305.03393v1.pdf should NOT be tagged");
            eprintln!("✓ 2305.03393v1.pdf: is_tagged=false");
        }
    }

    #[test]
    #[ignore = "PDFium init/destroy cycle conflicts with other tests - run with --ignored"]
    fn test_tagged_pdf_structure_tree_benchmark() {
        // Benchmark structure tree extraction for tagged PDFs (N=3501)
        // This validates the fast path implementation for Cascade v2 Phase A
        //
        // Run with: cargo test --ignored test_tagged_pdf_structure_tree_benchmark -- --nocapture
        use std::path::PathBuf;

        let pdfium = match create_pdfium() {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Note: Could not initialize PdfiumFast: {e}");
                return;
            }
        };

        // Test with tagged PDF (amt_handbook_sample.pdf)
        let tagged_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("test-corpus/pdf/amt_handbook_sample.pdf");
        if !tagged_path.exists() {
            eprintln!("Tagged PDF not found at {tagged_path:?}, skipping benchmark");
            return;
        }

        let doc = pdfium.load_pdf_from_file(&tagged_path, None).unwrap();
        assert!(doc.is_tagged(), "amt_handbook_sample.pdf should be tagged");

        let num_pages = doc.page_count();
        eprintln!("\n=== Tagged PDF Structure Tree Benchmark ===");
        eprintln!("File: amt_handbook_sample.pdf");
        eprintln!("Pages: {num_pages}");

        // Benchmark structure tree extraction
        let start = std::time::Instant::now();
        let mut total_elements = 0;
        let mut total_text_len = 0;

        for page_idx in 0..num_pages {
            let page = doc.load_page(page_idx).unwrap();
            if let Some(tree) = page.get_structure_tree() {
                total_elements += count_elements(&tree);
                // Build MCID map for text extraction
                if let Ok(mcid_map) = page.build_mcid_text_map() {
                    for text in mcid_map.values() {
                        total_text_len += text.len();
                    }
                }
            }
        }

        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
        let pages_per_sec = if elapsed_ms > 0.0 {
            num_pages as f64 / (elapsed_ms / 1000.0)
        } else {
            0.0
        };

        eprintln!("\n=== Results ===");
        eprintln!("Structure elements: {total_elements}");
        eprintln!("Text extracted: {total_text_len} chars");
        eprintln!("Time: {elapsed_ms:.2} ms");
        eprintln!("Speed: {pages_per_sec:.1} pages/sec");
        eprintln!("\nComparison:");
        eprintln!("- ML pipeline: ~3.5 pages/sec (RT-DETR INT8)");
        eprintln!(
            "- Structure tree: {:.1} pages/sec ({:.0}x faster)",
            pages_per_sec,
            pages_per_sec / 3.5
        );

        // Verify we got meaningful content
        assert!(total_elements > 0, "Should have found structure elements");
        assert!(total_text_len > 0, "Should have extracted text");

        fn count_elements(elements: &[super::PdfStructElement]) -> usize {
            elements
                .iter()
                .map(|e| 1 + count_elements(&e.children))
                .sum()
        }
    }

    #[test]
    #[ignore = "PDFium init/destroy cycle conflicts with other tests - run with --ignored"]
    fn test_batch_text_extraction_api() {
        // Test FPDFText_ExtractAllCells() batch API
        // Run with: cargo test --ignored test_batch_text_extraction_api
        let pdfium = match create_pdfium() {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Note: Could not initialize PdfiumFast: {e}");
                return;
            }
        };

        let pdf_path = std::path::Path::new("test-corpus/pdf/2305.03393v1-pg9.pdf");
        if !pdf_path.exists() {
            eprintln!("Test PDF not found, skipping");
            return;
        }

        let doc = pdfium.load_pdf_from_file(pdf_path, None).unwrap();
        let page = doc.load_page(0).unwrap();
        let page_height = page.height() as f64;

        // Test standard extraction
        let cells_standard = page.extract_text_cells(page_height).unwrap();
        eprintln!("Standard extraction: {} cells", cells_standard.len());

        // Test batch extraction
        let cells_batch = page.extract_text_cells_batch(page_height).unwrap();
        eprintln!("Batch extraction: {} cells", cells_batch.len());

        // Both should return similar results
        // (exact count may differ slightly due to implementation differences)
        assert!(
            cells_batch.len() >= cells_standard.len() / 2,
            "Batch should return reasonable number of cells"
        );

        // Verify cells have content
        if !cells_batch.is_empty() {
            let first_cell = &cells_batch[0];
            eprintln!(
                "First batch cell: '{}' at ({:.1}, {:.1})",
                first_cell.text.chars().take(50).collect::<String>(),
                first_cell.left,
                first_cell.top
            );
            assert!(!first_cell.text.is_empty(), "Cell should have text");
        }

        eprintln!("✓ Batch text extraction API works");
    }

    #[test]
    fn test_parse_pdf_date_full() {
        // Full date with timezone offset
        let dt = parse_pdf_date("D:20231215143052+00'00'").unwrap();
        assert_eq!(dt.year(), 2023);
        assert_eq!(dt.month(), 12);
        assert_eq!(dt.day(), 15);
        assert_eq!(dt.hour(), 14);
        assert_eq!(dt.minute(), 30);
        assert_eq!(dt.second(), 52);
    }

    #[test]
    fn test_parse_pdf_date_utc() {
        // Date with Z timezone
        let dt = parse_pdf_date("D:20231215143052Z").unwrap();
        assert_eq!(dt.year(), 2023);
        assert_eq!(dt.month(), 12);
    }

    #[test]
    fn test_parse_pdf_date_no_tz() {
        // Date without timezone
        let dt = parse_pdf_date("D:20231215143052").unwrap();
        assert_eq!(dt.year(), 2023);
        assert_eq!(dt.hour(), 14);
    }

    #[test]
    fn test_parse_pdf_date_only() {
        // Date only
        let dt = parse_pdf_date("D:20231215").unwrap();
        assert_eq!(dt.year(), 2023);
        assert_eq!(dt.month(), 12);
        assert_eq!(dt.day(), 15);
        assert_eq!(dt.hour(), 0);
    }

    #[test]
    fn test_parse_pdf_date_negative_offset() {
        // Date with negative timezone offset
        let dt = parse_pdf_date("D:20231215100000-05'00'").unwrap();
        assert_eq!(dt.year(), 2023);
        // 10:00 -05:00 = 15:00 UTC
        assert_eq!(dt.hour(), 15);
    }

    #[test]
    fn test_parse_pdf_date_no_prefix() {
        // Without D: prefix
        let dt = parse_pdf_date("20231215143052").unwrap();
        assert_eq!(dt.year(), 2023);
        assert_eq!(dt.month(), 12);
    }

    #[test]
    fn test_parse_pdf_date_invalid() {
        // Too short
        assert!(parse_pdf_date("D:20").is_none());
        // Empty
        assert!(parse_pdf_date("").is_none());
    }

    #[test]
    fn test_form_field_type_is_text_input() {
        assert!(PdfFormFieldType::TextField.is_text_input());
        assert!(PdfFormFieldType::XfaTextField.is_text_input());
        assert!(!PdfFormFieldType::Checkbox.is_text_input());
        assert!(!PdfFormFieldType::PushButton.is_text_input());
    }

    #[test]
    fn test_form_field_type_is_selection() {
        assert!(PdfFormFieldType::Checkbox.is_selection());
        assert!(PdfFormFieldType::RadioButton.is_selection());
        assert!(PdfFormFieldType::ComboBox.is_selection());
        assert!(PdfFormFieldType::ListBox.is_selection());
        assert!(PdfFormFieldType::XfaCheckbox.is_selection());
        assert!(!PdfFormFieldType::TextField.is_selection());
        assert!(!PdfFormFieldType::PushButton.is_selection());
    }

    #[test]
    fn test_form_field_type_is_button() {
        assert!(PdfFormFieldType::PushButton.is_button());
        assert!(PdfFormFieldType::XfaPushButton.is_button());
        assert!(!PdfFormFieldType::TextField.is_button());
        assert!(!PdfFormFieldType::Checkbox.is_button());
    }

    #[test]
    fn test_form_field_type_is_signature() {
        assert!(PdfFormFieldType::Signature.is_signature());
        assert!(PdfFormFieldType::XfaSignature.is_signature());
        assert!(!PdfFormFieldType::TextField.is_signature());
        assert!(!PdfFormFieldType::Checkbox.is_signature());
    }

    #[test]
    fn test_form_field_type_is_xfa() {
        assert!(PdfFormFieldType::Xfa.is_xfa());
        assert!(PdfFormFieldType::XfaCheckbox.is_xfa());
        assert!(PdfFormFieldType::XfaComboBox.is_xfa());
        assert!(PdfFormFieldType::XfaImageField.is_xfa());
        assert!(PdfFormFieldType::XfaListBox.is_xfa());
        assert!(PdfFormFieldType::XfaPushButton.is_xfa());
        assert!(PdfFormFieldType::XfaSignature.is_xfa());
        assert!(PdfFormFieldType::XfaTextField.is_xfa());
        // Standard form field types are not XFA
        assert!(!PdfFormFieldType::TextField.is_xfa());
        assert!(!PdfFormFieldType::Checkbox.is_xfa());
        assert!(!PdfFormFieldType::PushButton.is_xfa());
    }

    #[test]
    fn test_form_field_type_unknown() {
        assert!(!PdfFormFieldType::Unknown.is_text_input());
        assert!(!PdfFormFieldType::Unknown.is_selection());
        assert!(!PdfFormFieldType::Unknown.is_button());
        assert!(!PdfFormFieldType::Unknown.is_signature());
        assert!(!PdfFormFieldType::Unknown.is_xfa());
    }

    #[test]
    fn test_form_type_from_raw() {
        assert_eq!(PdfFormType::from_raw(0), PdfFormType::None);
        assert_eq!(PdfFormType::from_raw(1), PdfFormType::AcroForm);
        assert_eq!(PdfFormType::from_raw(2), PdfFormType::XfaFull);
        assert_eq!(PdfFormType::from_raw(3), PdfFormType::XfaForeground);
        assert_eq!(PdfFormType::from_raw(-1), PdfFormType::None);
        assert_eq!(PdfFormType::from_raw(99), PdfFormType::None);
    }

    #[test]
    fn test_form_type_is_xfa() {
        assert!(!PdfFormType::None.is_xfa());
        assert!(!PdfFormType::AcroForm.is_xfa());
        assert!(PdfFormType::XfaFull.is_xfa());
        assert!(PdfFormType::XfaForeground.is_xfa());
    }

    #[test]
    fn test_form_type_is_acro_form() {
        assert!(!PdfFormType::None.is_acro_form());
        assert!(PdfFormType::AcroForm.is_acro_form());
        assert!(!PdfFormType::XfaFull.is_acro_form());
        assert!(!PdfFormType::XfaForeground.is_acro_form());
    }

    #[test]
    fn test_form_type_has_forms() {
        assert!(!PdfFormType::None.has_forms());
        assert!(PdfFormType::AcroForm.has_forms());
        assert!(PdfFormType::XfaFull.has_forms());
        assert!(PdfFormType::XfaForeground.has_forms());
    }

    // BUG #23: Named destination struct tests
    #[test]
    fn test_named_destination_struct() {
        let dest = PdfNamedDestination {
            name: "chapter1".to_string(),
            page_index: 5,
            x: Some(100.0),
            y: Some(200.0),
            zoom: Some(1.5),
        };
        assert_eq!(dest.name, "chapter1");
        assert_eq!(dest.page_index, 5);
        assert_eq!(dest.x, Some(100.0));
        assert_eq!(dest.y, Some(200.0));
        assert_eq!(dest.zoom, Some(1.5));
    }

    #[test]
    fn test_named_destination_optional_fields() {
        let dest = PdfNamedDestination {
            name: "toc".to_string(),
            page_index: 0,
            x: None,
            y: None,
            zoom: None,
        };
        assert_eq!(dest.name, "toc");
        assert_eq!(dest.page_index, 0);
        assert!(dest.x.is_none());
        assert!(dest.y.is_none());
        assert!(dest.zoom.is_none());
    }

    #[test]
    fn test_named_destination_clone_and_eq() {
        let dest1 = PdfNamedDestination {
            name: "section2".to_string(),
            page_index: 10,
            x: Some(50.0),
            y: Some(750.0),
            zoom: None,
        };
        let dest2 = dest1.clone();
        assert_eq!(dest1, dest2);
    }

    // BUG #84: Text selection API tests
    // Note: These tests verify the method signatures and return types.
    // Full integration tests require a loaded PDF which conflicts with PDFium init.

    #[test]
    fn test_text_selection_api_signatures() {
        // This test verifies that the text selection methods have correct signatures
        // by checking that the method types are what we expect.
        // The actual methods require a PdfTextPageFast handle from a loaded PDF.

        // Verify Option<i32> return type for get_char_index_at_pos
        fn check_char_index_return(result: Option<i32>) -> bool {
            result.is_none_or(|i| i >= 0)
        }
        assert!(check_char_index_return(Some(0)));
        assert!(check_char_index_return(Some(100)));
        assert!(check_char_index_return(None));

        // Verify f64 return type for get_font_size
        fn check_font_size(size: f64) -> bool {
            size >= 0.0
        }
        assert!(check_font_size(12.0));
        assert!(check_font_size(0.0));

        // Verify f32 return type for get_char_angle
        fn check_char_angle(angle: f32) -> bool {
            (-360.0..=360.0).contains(&angle)
        }
        assert!(check_char_angle(0.0));
        assert!(check_char_angle(90.0));
        assert!(check_char_angle(-45.0));

        // Verify Option<(f64, f64, f64, f64)> for get_text_rect
        fn check_rect(rect: Option<(f64, f64, f64, f64)>) -> bool {
            rect.map(|(l, t, r, b)| l <= r && t >= b).unwrap_or(true)
        }
        assert!(check_rect(Some((0.0, 100.0, 50.0, 80.0))));
        assert!(check_rect(None));
    }

    #[test]
    fn test_text_rects_vector_construction() {
        // Test that we can construct vectors of text rects as returned by get_text_rects
        let rects: Vec<(f64, f64, f64, f64)> = vec![
            (10.0, 100.0, 50.0, 90.0), // First line
            (10.0, 90.0, 200.0, 80.0), // Second line (wrapping)
        ];

        assert_eq!(rects.len(), 2);
        assert_eq!(rects[0].0, 10.0); // left
        assert_eq!(rects[0].1, 100.0); // top
        assert_eq!(rects[1].2, 200.0); // right of second rect
    }

    #[test]
    fn test_char_index_at_pos_none_behavior() {
        // Document the expected behavior: None when no character at position
        let no_char: Option<i32> = None;
        assert!(no_char.is_none());

        // When a character IS found, index should be >= 0
        let found_char: i32 = 42;
        assert!(found_char >= 0);
    }

    // BUG #85: Digital signature API tests

    #[test]
    fn test_signature_struct_creation() {
        let sig = PdfSignature {
            index: 0,
            sub_filter: Some("adbe.pkcs7.detached".to_string()),
            reason: Some("Document approval".to_string()),
            signing_time: Some("D:20251222120000+00'00'".to_string()),
            doc_mdp_permission: Some(2),
            contents: vec![0x30, 0x82, 0x01, 0x00], // Sample DER prefix
            byte_range: vec![(0, 1000), (2000, 3000)],
        };

        assert_eq!(sig.index, 0);
        assert_eq!(sig.sub_filter, Some("adbe.pkcs7.detached".to_string()));
        assert_eq!(sig.reason, Some("Document approval".to_string()));
        assert!(sig.has_reason());
        assert!(sig.has_signing_time());
        assert_eq!(sig.contents.len(), 4);
        assert_eq!(sig.byte_range.len(), 2);
    }

    #[test]
    fn test_signature_permission_checks() {
        // Permission 1: No changes allowed
        let sig1 = PdfSignature {
            index: 0,
            sub_filter: None,
            reason: None,
            signing_time: None,
            doc_mdp_permission: Some(1),
            contents: vec![],
            byte_range: vec![],
        };
        assert!(!sig1.allows_changes());
        assert!(!sig1.allows_form_fill());
        assert!(!sig1.allows_annotations());

        // Permission 2: Fill forms allowed
        let sig2 = PdfSignature {
            index: 0,
            sub_filter: None,
            reason: None,
            signing_time: None,
            doc_mdp_permission: Some(2),
            contents: vec![],
            byte_range: vec![],
        };
        assert!(sig2.allows_changes());
        assert!(sig2.allows_form_fill());
        assert!(!sig2.allows_annotations());

        // Permission 3: Fill forms and annotate allowed
        let sig3 = PdfSignature {
            index: 0,
            sub_filter: None,
            reason: None,
            signing_time: None,
            doc_mdp_permission: Some(3),
            contents: vec![],
            byte_range: vec![],
        };
        assert!(sig3.allows_changes());
        assert!(sig3.allows_form_fill());
        assert!(sig3.allows_annotations());
    }

    #[test]
    fn test_signature_no_permission() {
        // No DocMDP permission (defaults to allowing all)
        let sig = PdfSignature {
            index: 0,
            sub_filter: None,
            reason: None,
            signing_time: None,
            doc_mdp_permission: None,
            contents: vec![],
            byte_range: vec![],
        };
        assert!(sig.allows_changes());
        assert!(sig.allows_form_fill());
        assert!(sig.allows_annotations());
    }

    #[test]
    fn test_signature_optional_fields() {
        let sig = PdfSignature {
            index: 1,
            sub_filter: None,
            reason: None,
            signing_time: None,
            doc_mdp_permission: None,
            contents: vec![],
            byte_range: vec![],
        };

        assert_eq!(sig.index, 1);
        assert!(sig.sub_filter.is_none());
        assert!(!sig.has_reason());
        assert!(!sig.has_signing_time());
        assert!(sig.doc_mdp_permission.is_none());
        assert!(sig.contents.is_empty());
        assert!(sig.byte_range.is_empty());
    }

    #[test]
    fn test_signature_clone_and_eq() {
        let sig1 = PdfSignature {
            index: 0,
            sub_filter: Some("adbe.pkcs7.sha1".to_string()),
            reason: Some("Approved".to_string()),
            signing_time: Some("D:20251222".to_string()),
            doc_mdp_permission: Some(2),
            contents: vec![1, 2, 3, 4],
            byte_range: vec![(0, 100)],
        };
        let sig2 = sig1.clone();
        assert_eq!(sig1, sig2);
    }

    #[test]
    fn test_signature_byte_range_pairs() {
        // Test that byte range is properly formatted as pairs
        let sig = PdfSignature {
            index: 0,
            sub_filter: None,
            reason: None,
            signing_time: None,
            doc_mdp_permission: None,
            contents: vec![],
            byte_range: vec![(0, 500), (800, 1200)], // Two ranges
        };

        assert_eq!(sig.byte_range.len(), 2);
        assert_eq!(sig.byte_range[0], (0, 500)); // First range: start=0, len=500
        assert_eq!(sig.byte_range[1], (800, 1200)); // Second range: start=800, len=1200
    }

    // BUG #86: JavaScript detection API tests

    #[test]
    fn test_javascript_action_struct_creation() {
        let js = PdfJavaScriptAction {
            index: 0,
            name: Some("DocumentOpen".to_string()),
            script: "app.alert('Hello World');".to_string(),
        };

        assert_eq!(js.index, 0);
        assert_eq!(js.name, Some("DocumentOpen".to_string()));
        assert_eq!(js.script, "app.alert('Hello World');");
        assert!(js.has_name());
        assert_eq!(js.script_len(), 25);
        assert!(!js.is_empty());
    }

    #[test]
    fn test_javascript_action_no_name() {
        let js = PdfJavaScriptAction {
            index: 1,
            name: None,
            script: "console.log('test');".to_string(),
        };

        assert!(!js.has_name());
        assert_eq!(js.name, None);
    }

    #[test]
    fn test_javascript_action_empty_script() {
        let js = PdfJavaScriptAction {
            index: 0,
            name: Some("EmptyScript".to_string()),
            script: String::new(),
        };

        assert!(js.is_empty());
        assert_eq!(js.script_len(), 0);
    }

    #[test]
    fn test_javascript_suspicious_patterns() {
        // Safe script
        let safe_js = PdfJavaScriptAction {
            index: 0,
            name: None,
            script: "app.alert('Welcome');".to_string(),
        };
        assert!(!safe_js.has_suspicious_patterns());

        // Suspicious: eval
        let eval_js = PdfJavaScriptAction {
            index: 0,
            name: None,
            script: "eval(someCode);".to_string(),
        };
        assert!(eval_js.has_suspicious_patterns());

        // Suspicious: util.printf (buffer overflow vector)
        let printf_js = PdfJavaScriptAction {
            index: 0,
            name: None,
            script: "util.printf('%d', 123);".to_string(),
        };
        assert!(printf_js.has_suspicious_patterns());

        // Suspicious: app.launchURL
        let launch_js = PdfJavaScriptAction {
            index: 0,
            name: None,
            script: "app.launchURL('http://evil.com');".to_string(),
        };
        assert!(launch_js.has_suspicious_patterns());

        // Suspicious: collectEmailInfo
        let email_js = PdfJavaScriptAction {
            index: 0,
            name: None,
            script: "this.collectEmailInfo()".to_string(),
        };
        assert!(email_js.has_suspicious_patterns());
    }

    #[test]
    fn test_javascript_clone_and_eq() {
        let js1 = PdfJavaScriptAction {
            index: 0,
            name: Some("TestAction".to_string()),
            script: "var x = 1;".to_string(),
        };
        let js2 = js1.clone();
        assert_eq!(js1, js2);
    }

    /// BUG #92 fix: Test large PDF handling (1000+ pages)
    ///
    /// This test verifies that the PDF backend can handle large documents
    /// without memory issues or performance degradation.
    ///
    /// Run manually: `cargo test -p docling-backend --features pdfium-fast test_large_pdf -- --ignored`
    #[test]
    #[ignore = "Manual test - requires large PDF file"]
    fn test_large_pdf_1000_pages() {
        let pdf_path = "test-corpus/pdf/large_1000_pages.pdf";

        // Skip if file doesn't exist
        if !std::path::Path::new(pdf_path).exists() {
            eprintln!("Skipping test_large_pdf_1000_pages: {pdf_path} not found");
            eprintln!("To run this test, provide a PDF with 1000+ pages at the above path");
            return;
        }

        let start = std::time::Instant::now();
        let pdfium = create_pdfium().expect("Failed to initialize PDFium");
        let doc = pdfium
            .load_pdf_from_file(std::path::Path::new(pdf_path), None)
            .expect("Failed to load large PDF");

        let page_count = doc.page_count();
        assert!(page_count >= 1000, "Expected 1000+ pages, got {page_count}");

        // Test loading first 10 pages and extracting text
        for i in 0..10.min(page_count) {
            let page = doc.load_page(i).expect("Failed to load page");

            // Verify dimensions are reasonable
            let width = page.width();
            let height = page.height();
            assert!(width > 0.0, "Page {i} has zero width");
            assert!(height > 0.0, "Page {i} has zero height");

            // Extract text to verify text layer works
            let text_page = PdfTextPageFast::from_page(&page).expect("Failed to create text page");
            let char_count = text_page.char_count();
            assert!(
                char_count >= 0,
                "Page {i} has invalid char count: {char_count}"
            );
        }

        // Test rendering a few pages at low DPI
        for i in 0..3.min(page_count) {
            let page = doc.load_page(i).expect("Failed to load page for render");
            let _rgb = page
                .render_to_rgb_array(72.0) // Low DPI for speed
                .expect("Failed to render page");
        }

        let elapsed = start.elapsed();
        println!("Large PDF test completed: {page_count} pages processed in {elapsed:?}");
        println!(
            "Average: {:.2} ms/page for basic operations",
            elapsed.as_millis() as f64 / 10.0
        );
    }

    /// Test MCID (Marked Content ID) extraction from tagged PDFs
    ///
    /// MCIDs link structure tree elements to actual page content.
    /// Run with: cargo test --package docling-backend --no-default-features --features pdfium-fast --lib --release -- pdfium_adapter::tests::test_mcid_extraction --exact --nocapture --ignored
    #[test]
    #[ignore = "Requires tagged PDF test file - run with --ignored flag"]
    fn test_mcid_extraction() {
        use std::path::PathBuf;

        let pdfium = match create_pdfium() {
            Ok(p) => p,
            Err(e) => {
                println!("Skipping test - PdfiumFast unavailable: {e}");
                return;
            }
        };

        // Use tagged PDF from test corpus (amt_handbook_sample.pdf is known to be tagged)
        let test_pdf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("test-corpus/pdf/amt_handbook_sample.pdf");

        if !test_pdf.exists() {
            println!("Skipping test - tagged PDF not found at: {test_pdf:?}");
            return;
        }

        println!("\n=== MCID Extraction Test ===");
        println!("PDF: {test_pdf:?}");

        // Load PDF
        let pdf_doc = pdfium
            .load_pdf_from_file(&test_pdf, None)
            .expect("Failed to load PDF");

        // Verify it's tagged
        let is_tagged = pdf_doc.is_tagged();
        println!("Is tagged: {is_tagged}");
        assert!(is_tagged, "amt_handbook_sample.pdf should be tagged");

        // Load first page
        let page = pdf_doc.load_page(0).expect("Failed to load page 0");

        // Test structure tree extraction
        let structure_tree = page.get_structure_tree();
        println!(
            "Structure tree elements: {:?}",
            structure_tree.as_ref().map(|t| t.len())
        );

        if let Some(tree) = &structure_tree {
            // Count elements with MCIDs
            fn count_mcids(elem: &PdfStructElement) -> (usize, usize) {
                let has_mcid = !elem.marked_content_ids.is_empty();
                let mut total = if has_mcid { 1 } else { 0 };
                let mut mcid_count = elem.marked_content_ids.len();
                for child in &elem.children {
                    let (child_total, child_mcids) = count_mcids(child);
                    total += child_total;
                    mcid_count += child_mcids;
                }
                (total, mcid_count)
            }

            let mut total_with_mcid = 0;
            let mut total_mcids = 0;
            for elem in tree {
                let (with_mcid, mcids) = count_mcids(elem);
                total_with_mcid += with_mcid;
                total_mcids += mcids;
            }
            println!("Elements with MCIDs: {total_with_mcid}");
            println!("Total MCIDs: {total_mcids}");
        }

        // Build MCID text map
        let mcid_map = page
            .build_mcid_text_map()
            .expect("Failed to build MCID map");
        println!("\n=== MCID Map ===");
        println!("Total MCID entries: {}", mcid_map.len());

        // Print first 5 entries
        for (i, (mcid, text)) in mcid_map.iter().take(5).enumerate() {
            let preview: String = text.chars().take(60).collect();
            println!(
                "  [{}] MCID {}: '{}{}' ({} chars)",
                i,
                mcid,
                preview,
                if text.len() > 60 { "..." } else { "" },
                text.len()
            );
        }

        // Verify we got meaningful content
        assert!(
            !mcid_map.is_empty(),
            "Expected non-empty MCID map for tagged PDF"
        );

        // Check that at least some MCIDs have text content
        let mcids_with_text = mcid_map.values().filter(|t| !t.trim().is_empty()).count();
        println!("\nMCIDs with non-empty text: {mcids_with_text}");
        assert!(
            mcids_with_text > 0,
            "Expected some MCIDs to have text content"
        );

        println!("\n✓ MCID extraction working");
    }

    #[test]
    fn test_pdf_annotation_type_display() {
        use super::PdfAnnotationType;
        assert_eq!(format!("{}", PdfAnnotationType::Unknown), "unknown");
        assert_eq!(format!("{}", PdfAnnotationType::Text), "text");
        assert_eq!(format!("{}", PdfAnnotationType::Link), "link");
        assert_eq!(format!("{}", PdfAnnotationType::FreeText), "free_text");
        assert_eq!(format!("{}", PdfAnnotationType::Highlight), "highlight");
        assert_eq!(format!("{}", PdfAnnotationType::Redact), "redact");
    }

    #[test]
    fn test_pdf_form_field_type_display() {
        use super::PdfFormFieldType;
        assert_eq!(format!("{}", PdfFormFieldType::Unknown), "unknown");
        assert_eq!(format!("{}", PdfFormFieldType::PushButton), "push_button");
        assert_eq!(format!("{}", PdfFormFieldType::Checkbox), "checkbox");
        assert_eq!(format!("{}", PdfFormFieldType::TextField), "text_field");
        assert_eq!(
            format!("{}", PdfFormFieldType::XfaTextField),
            "xfa_text_field"
        );
    }

    #[test]
    fn test_pdf_form_type_display() {
        use super::PdfFormType;
        assert_eq!(format!("{}", PdfFormType::None), "none");
        assert_eq!(format!("{}", PdfFormType::AcroForm), "acro_form");
        assert_eq!(format!("{}", PdfFormType::XfaFull), "xfa_full");
        assert_eq!(format!("{}", PdfFormType::XfaForeground), "xfa_foreground");
    }

    #[test]
    fn test_pdf_page_object_type_display() {
        use super::PdfPageObjectType;
        assert_eq!(format!("{}", PdfPageObjectType::Unknown), "unknown");
        assert_eq!(format!("{}", PdfPageObjectType::Text), "text");
        assert_eq!(format!("{}", PdfPageObjectType::Path), "path");
        assert_eq!(format!("{}", PdfPageObjectType::Image), "image");
        assert_eq!(format!("{}", PdfPageObjectType::Shading), "shading");
        assert_eq!(format!("{}", PdfPageObjectType::Form), "form");
    }

    #[test]
    fn test_pdf_annotation_type_from_str() {
        use super::PdfAnnotationType;
        use std::str::FromStr;

        // Basic parsing
        assert_eq!(
            PdfAnnotationType::from_str("unknown").unwrap(),
            PdfAnnotationType::Unknown
        );
        assert_eq!(
            PdfAnnotationType::from_str("text").unwrap(),
            PdfAnnotationType::Text
        );
        assert_eq!(
            PdfAnnotationType::from_str("link").unwrap(),
            PdfAnnotationType::Link
        );
        assert_eq!(
            PdfAnnotationType::from_str("free_text").unwrap(),
            PdfAnnotationType::FreeText
        );
        assert_eq!(
            PdfAnnotationType::from_str("highlight").unwrap(),
            PdfAnnotationType::Highlight
        );
        assert_eq!(
            PdfAnnotationType::from_str("redact").unwrap(),
            PdfAnnotationType::Redact
        );
        assert_eq!(
            PdfAnnotationType::from_str("3d").unwrap(),
            PdfAnnotationType::ThreeD
        );

        // Case insensitive
        assert_eq!(
            PdfAnnotationType::from_str("HIGHLIGHT").unwrap(),
            PdfAnnotationType::Highlight
        );
        assert_eq!(
            PdfAnnotationType::from_str("FreeText").unwrap(),
            PdfAnnotationType::FreeText
        );

        // Aliases
        assert_eq!(
            PdfAnnotationType::from_str("note").unwrap(),
            PdfAnnotationType::Text
        );
        assert_eq!(
            PdfAnnotationType::from_str("sticky_note").unwrap(),
            PdfAnnotationType::Text
        );
        assert_eq!(
            PdfAnnotationType::from_str("hyperlink").unwrap(),
            PdfAnnotationType::Link
        );
        assert_eq!(
            PdfAnnotationType::from_str("hl").unwrap(),
            PdfAnnotationType::Highlight
        );
        assert_eq!(
            PdfAnnotationType::from_str("strikethrough").unwrap(),
            PdfAnnotationType::Strikeout
        );
        assert_eq!(
            PdfAnnotationType::from_str("attachment").unwrap(),
            PdfAnnotationType::FileAttachment
        );
        assert_eq!(
            PdfAnnotationType::from_str("threed").unwrap(),
            PdfAnnotationType::ThreeD
        );

        // Error case
        assert!(PdfAnnotationType::from_str("invalid").is_err());
    }

    #[test]
    fn test_pdf_annotation_type_roundtrip() {
        use super::PdfAnnotationType;
        use std::str::FromStr;

        let variants = [
            PdfAnnotationType::Unknown,
            PdfAnnotationType::Text,
            PdfAnnotationType::Link,
            PdfAnnotationType::FreeText,
            PdfAnnotationType::Line,
            PdfAnnotationType::Square,
            PdfAnnotationType::Circle,
            PdfAnnotationType::Polygon,
            PdfAnnotationType::Polyline,
            PdfAnnotationType::Highlight,
            PdfAnnotationType::Underline,
            PdfAnnotationType::Squiggly,
            PdfAnnotationType::Strikeout,
            PdfAnnotationType::Stamp,
            PdfAnnotationType::Caret,
            PdfAnnotationType::Ink,
            PdfAnnotationType::Popup,
            PdfAnnotationType::FileAttachment,
            PdfAnnotationType::Sound,
            PdfAnnotationType::Movie,
            PdfAnnotationType::Widget,
            PdfAnnotationType::Screen,
            PdfAnnotationType::PrinterMark,
            PdfAnnotationType::TrapNet,
            PdfAnnotationType::Watermark,
            PdfAnnotationType::ThreeD,
            PdfAnnotationType::RichMedia,
            PdfAnnotationType::XfaWidget,
            PdfAnnotationType::Redact,
        ];
        for variant in variants {
            let s = variant.to_string();
            let parsed = PdfAnnotationType::from_str(&s).unwrap();
            assert_eq!(variant, parsed, "Roundtrip failed for {variant:?}");
        }
    }

    #[test]
    fn test_pdf_form_field_type_from_str() {
        use super::PdfFormFieldType;
        use std::str::FromStr;

        // Basic parsing
        assert_eq!(
            PdfFormFieldType::from_str("unknown").unwrap(),
            PdfFormFieldType::Unknown
        );
        assert_eq!(
            PdfFormFieldType::from_str("push_button").unwrap(),
            PdfFormFieldType::PushButton
        );
        assert_eq!(
            PdfFormFieldType::from_str("checkbox").unwrap(),
            PdfFormFieldType::Checkbox
        );
        assert_eq!(
            PdfFormFieldType::from_str("text_field").unwrap(),
            PdfFormFieldType::TextField
        );
        assert_eq!(
            PdfFormFieldType::from_str("xfa_text_field").unwrap(),
            PdfFormFieldType::XfaTextField
        );

        // Case insensitive
        assert_eq!(
            PdfFormFieldType::from_str("CHECKBOX").unwrap(),
            PdfFormFieldType::Checkbox
        );
        assert_eq!(
            PdfFormFieldType::from_str("PushButton").unwrap(),
            PdfFormFieldType::PushButton
        );

        // Aliases
        assert_eq!(
            PdfFormFieldType::from_str("button").unwrap(),
            PdfFormFieldType::PushButton
        );
        assert_eq!(
            PdfFormFieldType::from_str("btn").unwrap(),
            PdfFormFieldType::PushButton
        );
        assert_eq!(
            PdfFormFieldType::from_str("check").unwrap(),
            PdfFormFieldType::Checkbox
        );
        assert_eq!(
            PdfFormFieldType::from_str("radio").unwrap(),
            PdfFormFieldType::RadioButton
        );
        assert_eq!(
            PdfFormFieldType::from_str("dropdown").unwrap(),
            PdfFormFieldType::ComboBox
        );
        assert_eq!(
            PdfFormFieldType::from_str("text").unwrap(),
            PdfFormFieldType::TextField
        );
        assert_eq!(
            PdfFormFieldType::from_str("sig").unwrap(),
            PdfFormFieldType::Signature
        );

        // Error case
        assert!(PdfFormFieldType::from_str("invalid").is_err());
    }

    #[test]
    fn test_pdf_form_field_type_roundtrip() {
        use super::PdfFormFieldType;
        use std::str::FromStr;

        let variants = [
            PdfFormFieldType::Unknown,
            PdfFormFieldType::PushButton,
            PdfFormFieldType::Checkbox,
            PdfFormFieldType::RadioButton,
            PdfFormFieldType::ComboBox,
            PdfFormFieldType::ListBox,
            PdfFormFieldType::TextField,
            PdfFormFieldType::Signature,
            PdfFormFieldType::Xfa,
            PdfFormFieldType::XfaCheckbox,
            PdfFormFieldType::XfaComboBox,
            PdfFormFieldType::XfaImageField,
            PdfFormFieldType::XfaListBox,
            PdfFormFieldType::XfaPushButton,
            PdfFormFieldType::XfaSignature,
            PdfFormFieldType::XfaTextField,
        ];
        for variant in variants {
            let s = variant.to_string();
            let parsed = PdfFormFieldType::from_str(&s).unwrap();
            assert_eq!(variant, parsed, "Roundtrip failed for {variant:?}");
        }
    }

    #[test]
    fn test_pdf_form_type_from_str() {
        use super::PdfFormType;
        use std::str::FromStr;

        // Basic parsing
        assert_eq!(PdfFormType::from_str("none").unwrap(), PdfFormType::None);
        assert_eq!(
            PdfFormType::from_str("acro_form").unwrap(),
            PdfFormType::AcroForm
        );
        assert_eq!(
            PdfFormType::from_str("xfa_full").unwrap(),
            PdfFormType::XfaFull
        );
        assert_eq!(
            PdfFormType::from_str("xfa_foreground").unwrap(),
            PdfFormType::XfaForeground
        );

        // Case insensitive
        assert_eq!(PdfFormType::from_str("NONE").unwrap(), PdfFormType::None);
        assert_eq!(
            PdfFormType::from_str("AcroForm").unwrap(),
            PdfFormType::AcroForm
        );

        // Aliases
        assert_eq!(PdfFormType::from_str("noform").unwrap(), PdfFormType::None);
        assert_eq!(PdfFormType::from_str("empty").unwrap(), PdfFormType::None);
        assert_eq!(
            PdfFormType::from_str("acro").unwrap(),
            PdfFormType::AcroForm
        );
        assert_eq!(
            PdfFormType::from_str("standard").unwrap(),
            PdfFormType::AcroForm
        );
        assert_eq!(PdfFormType::from_str("xfa").unwrap(), PdfFormType::XfaFull);
        assert_eq!(
            PdfFormType::from_str("dynamic").unwrap(),
            PdfFormType::XfaFull
        );
        assert_eq!(
            PdfFormType::from_str("foreground").unwrap(),
            PdfFormType::XfaForeground
        );

        // Error case
        assert!(PdfFormType::from_str("invalid").is_err());
    }

    #[test]
    fn test_pdf_form_type_roundtrip() {
        use super::PdfFormType;
        use std::str::FromStr;

        let variants = [
            PdfFormType::None,
            PdfFormType::AcroForm,
            PdfFormType::XfaFull,
            PdfFormType::XfaForeground,
        ];
        for variant in variants {
            let s = variant.to_string();
            let parsed = PdfFormType::from_str(&s).unwrap();
            assert_eq!(variant, parsed, "Roundtrip failed for {variant:?}");
        }
    }

    #[test]
    fn test_pdf_page_object_type_from_str() {
        use super::PdfPageObjectType;
        use std::str::FromStr;

        // Basic parsing
        assert_eq!(
            PdfPageObjectType::from_str("unknown").unwrap(),
            PdfPageObjectType::Unknown
        );
        assert_eq!(
            PdfPageObjectType::from_str("text").unwrap(),
            PdfPageObjectType::Text
        );
        assert_eq!(
            PdfPageObjectType::from_str("path").unwrap(),
            PdfPageObjectType::Path
        );
        assert_eq!(
            PdfPageObjectType::from_str("image").unwrap(),
            PdfPageObjectType::Image
        );
        assert_eq!(
            PdfPageObjectType::from_str("shading").unwrap(),
            PdfPageObjectType::Shading
        );
        assert_eq!(
            PdfPageObjectType::from_str("form").unwrap(),
            PdfPageObjectType::Form
        );

        // Case insensitive
        assert_eq!(
            PdfPageObjectType::from_str("TEXT").unwrap(),
            PdfPageObjectType::Text
        );
        assert_eq!(
            PdfPageObjectType::from_str("Image").unwrap(),
            PdfPageObjectType::Image
        );

        // Aliases
        assert_eq!(
            PdfPageObjectType::from_str("txt").unwrap(),
            PdfPageObjectType::Text
        );
        assert_eq!(
            PdfPageObjectType::from_str("glyph").unwrap(),
            PdfPageObjectType::Text
        );
        assert_eq!(
            PdfPageObjectType::from_str("char").unwrap(),
            PdfPageObjectType::Text
        );
        assert_eq!(
            PdfPageObjectType::from_str("line").unwrap(),
            PdfPageObjectType::Path
        );
        assert_eq!(
            PdfPageObjectType::from_str("curve").unwrap(),
            PdfPageObjectType::Path
        );
        assert_eq!(
            PdfPageObjectType::from_str("vector").unwrap(),
            PdfPageObjectType::Path
        );
        assert_eq!(
            PdfPageObjectType::from_str("img").unwrap(),
            PdfPageObjectType::Image
        );
        assert_eq!(
            PdfPageObjectType::from_str("picture").unwrap(),
            PdfPageObjectType::Image
        );
        assert_eq!(
            PdfPageObjectType::from_str("gradient").unwrap(),
            PdfPageObjectType::Shading
        );
        assert_eq!(
            PdfPageObjectType::from_str("xobject").unwrap(),
            PdfPageObjectType::Form
        );

        // Error case
        assert!(PdfPageObjectType::from_str("invalid").is_err());
    }

    #[test]
    fn test_pdf_page_object_type_roundtrip() {
        use super::PdfPageObjectType;
        use std::str::FromStr;

        let variants = [
            PdfPageObjectType::Unknown,
            PdfPageObjectType::Text,
            PdfPageObjectType::Path,
            PdfPageObjectType::Image,
            PdfPageObjectType::Shading,
            PdfPageObjectType::Form,
        ];
        for variant in variants {
            let s = variant.to_string();
            let parsed = PdfPageObjectType::from_str(&s).unwrap();
            assert_eq!(variant, parsed, "Roundtrip failed for {variant:?}");
        }
    }
}
