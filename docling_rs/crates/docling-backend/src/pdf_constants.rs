//! Shared constants for PDF processing.
//!
//! These constants are used by both `pdf.rs` (pdfium-render) and `pdfium_adapter.rs`
//! (pdfium-fast) backends to ensure consistent PDF processing behavior.

/// PDF points per inch - standard PostScript/PDF unit conversion factor.
///
/// PDF dimensions are specified in "points" where 1 inch = 72 points.
/// This constant is used to convert between PDF points and pixel dimensions
/// at a given DPI (e.g., `pixels = points * dpi / PDF_POINTS_PER_INCH`).
pub const PDF_POINTS_PER_INCH: f32 = 72.0;

/// Minimum merge threshold (in points) for text cell grouping.
///
/// Prevents zero thresholds when dealing with zero-height cells (horizontal rules, etc.)
/// Used in both `pdf.rs` and `pdf_fast.rs` for consistent text cell merging behavior.
pub const PDF_MIN_MERGE_THRESHOLD: f32 = 2.0;

/// Vertical alignment threshold factor for grouping text cells into rows.
///
/// Cells are considered on the same row if their vertical positions differ by
/// less than `row_height * PDF_VERTICAL_THRESHOLD_FACTOR`.
pub const PDF_VERTICAL_THRESHOLD_FACTOR: f32 = 0.5;

/// Height threshold (in points) below which a cell is considered "small".
///
/// Small cells (like superscripts/subscripts or ORCID icons) get special treatment
/// during row grouping - they can join a row if they're within the row's span
/// even if strict alignment fails.
pub const PDF_SMALL_CELL_HEIGHT_THRESHOLD: f32 = 1.0;

/// Tolerance (in points) for determining if a small cell is within a row's span.
///
/// A small cell is considered within the row span if:
/// `cell_top >= row_top - PDF_ROW_SPAN_TOLERANCE && cell_bottom <= row_bottom + PDF_ROW_SPAN_TOLERANCE`
pub const PDF_ROW_SPAN_TOLERANCE: f32 = 2.0;
