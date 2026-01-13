// FFI boundary requires many casts. These are safe because:
// - ptr::null() returns correctly typed null pointers
// - bbox coordinates use f32 (sufficient precision for pixel coords)
// - C strings are UTF-8 validated
// - Similar names (state/stage) are intentional FFI parameter patterns
#![allow(
    clippy::cast_possible_truncation,  // bbox f64→f32, len→u32 safe for reasonable sizes
    clippy::cast_possible_wrap,        // len fits in i32 for FFI
    clippy::cast_sign_loss,            // coordinates/indices always non-negative
    clippy::cast_precision_loss,       // usize→f32 ok for display/counts
    clippy::similar_names,             // state/stage pattern is intentional
    clippy::wildcard_imports,          // acceptable for types module
    clippy::unnecessary_wraps,         // FFI consistency requires Option/Result
    clippy::used_underscore_binding,   // FFI parameters may be unused in some paths
    clippy::single_match_else,         // clearer for FFI error handling
    clippy::uninlined_format_args,     // format strings in error messages
    clippy::redundant_closure_for_method_calls, // clearer intent in map() chains
    clippy::unnecessary_debug_formatting, // useful for FFI error messages
    clippy::needless_pass_by_value,    // FFI boundaries often require owned types
    clippy::must_use_candidate,        // FFI functions don't need must_use
    clippy::format_push_string,        // string building in error messages
    clippy::needless_raw_string_hashes,// raw strings in tests
    clippy::redundant_closure,         // explicit closures clearer in some cases
    clippy::doc_markdown,              // documentation references vary
    clippy::if_not_else,               // clearer control flow in FFI
    clippy::ptr_as_ptr,                // FFI pointer conversions
    clippy::borrow_as_ptr,             // FFI test pointer patterns
    clippy::float_cmp,                 // exact test values
    clippy::manual_let_else,           // clearer control flow in tests
)]

//! `DoclingViz` FFI Bridge
//!
//! C FFI bindings for the `DoclingViz` macOS visualization application.
//! Provides access to PDF extraction pipeline stages for step-by-step debugging.
//!
//! # Architecture
//!
//! This crate exposes a C API that Swift can call via FFI. The main components are:
//!
//! - **Pipeline Handle**: Opaque pointer to the Rust extraction pipeline
//! - **Stage Snapshots**: Intermediate results at each pipeline stage
//! - **Bounding Boxes**: Layout detection results with labels and confidence
//! - **Text Cells**: OCR results with character-level positioning
//!
//! # Memory Management
//!
//! All memory allocated by Rust functions must be freed using the corresponding
//! `*_free` functions. Failing to do so will cause memory leaks.
//!
//! # Thread Safety
//!
//! Pipeline handles are NOT thread-safe. Each handle should be used from a single
//! thread only. Create multiple handles for concurrent processing.

use serde::{Deserialize, Serialize};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

pub mod batch_processor;
pub mod pdf_state;
pub mod pipeline_integration;
pub mod types;
pub mod visualization;

use pdf_state::PdfState;

/// US Letter page width in points (8.5 inches × 72 dpi)
const US_LETTER_WIDTH_F32: f32 = 612.0;
/// US Letter page height in points (11 inches × 72 dpi)
const US_LETTER_HEIGHT_F32: f32 = 792.0;

/// Result code for FFI operations
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DlvizResult {
    /// Operation succeeded
    Success = 0,
    /// Invalid argument (null pointer, out of bounds, etc.)
    InvalidArgument = -1,
    /// File not found or cannot be read
    FileNotFound = -2,
    /// PDF parsing error
    ParseError = -3,
    /// ML inference error
    InferenceError = -4,
    /// Out of memory
    OutOfMemory = -5,
    /// Internal error (bug)
    InternalError = -99,
}

impl std::fmt::Display for DlvizResult {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Success => "success",
            Self::InvalidArgument => "invalid argument",
            Self::FileNotFound => "file not found",
            Self::ParseError => "parse error",
            Self::InferenceError => "inference error",
            Self::OutOfMemory => "out of memory",
            Self::InternalError => "internal error",
        };
        write!(f, "{s}")
    }
}

impl std::str::FromStr for DlvizResult {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Normalize: lowercase and remove spaces/hyphens/underscores
        let normalized: String = s
            .to_lowercase()
            .chars()
            .filter(|c| !c.is_whitespace() && *c != '-' && *c != '_')
            .collect();
        match normalized.as_str() {
            "success" | "ok" | "0" => Ok(Self::Success),
            "invalidargument" | "invalid" | "badarg" | "1" => Ok(Self::InvalidArgument),
            "filenotfound" | "notfound" | "missing" | "2" => Ok(Self::FileNotFound),
            "parseerror" | "parse" | "3" => Ok(Self::ParseError),
            "inferenceerror" | "inference" | "ml" | "4" => Ok(Self::InferenceError),
            "outofmemory" | "oom" | "memory" | "5" => Ok(Self::OutOfMemory),
            "internalerror" | "internal" | "bug" | "99" => Ok(Self::InternalError),
            _ => Err(format!(
                "unknown result code: '{s}' (expected: success, invalid argument, file not found, \
                parse error, inference error, out of memory, internal error)"
            )),
        }
    }
}

/// Document item label (matches `DocItemLabel` in docling-pdf-ml)
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DlvizLabel {
    /// Caption (figure/table caption)
    Caption = 0,
    /// Footnote
    Footnote = 1,
    /// Formula/equation
    Formula = 2,
    /// List item
    ListItem = 3,
    /// Page footer
    PageFooter = 4,
    /// Page header
    PageHeader = 5,
    /// Picture/image
    Picture = 6,
    /// Section header
    SectionHeader = 7,
    /// Table
    Table = 8,
    /// Regular text
    Text = 9,
    /// Title
    Title = 10,
    /// Code block
    Code = 11,
    /// Checkbox (selected)
    CheckboxSelected = 12,
    /// Checkbox (unselected)
    CheckboxUnselected = 13,
    /// Document index
    DocumentIndex = 14,
    /// Form element
    Form = 15,
    /// Key-value region
    KeyValueRegion = 16,
}

impl std::fmt::Display for DlvizLabel {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Caption => "caption",
            Self::Footnote => "footnote",
            Self::Formula => "formula",
            Self::ListItem => "list item",
            Self::PageFooter => "page footer",
            Self::PageHeader => "page header",
            Self::Picture => "picture",
            Self::SectionHeader => "section header",
            Self::Table => "table",
            Self::Text => "text",
            Self::Title => "title",
            Self::Code => "code",
            Self::CheckboxSelected => "checkbox (selected)",
            Self::CheckboxUnselected => "checkbox (unselected)",
            Self::DocumentIndex => "document index",
            Self::Form => "form",
            Self::KeyValueRegion => "key-value region",
        };
        write!(f, "{s}")
    }
}

impl std::str::FromStr for DlvizLabel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Normalize: lowercase and remove spaces/hyphens/underscores
        let normalized: String = s
            .to_lowercase()
            .chars()
            .filter(|c| !c.is_whitespace() && *c != '-' && *c != '_')
            .collect();
        match normalized.as_str() {
            "caption" => Ok(Self::Caption),
            "footnote" => Ok(Self::Footnote),
            "formula" | "equation" => Ok(Self::Formula),
            "listitem" | "list" => Ok(Self::ListItem),
            "pagefooter" | "footer" => Ok(Self::PageFooter),
            "pageheader" | "header" => Ok(Self::PageHeader),
            "picture" | "image" | "img" => Ok(Self::Picture),
            "sectionheader" | "section" | "heading" => Ok(Self::SectionHeader),
            "table" => Ok(Self::Table),
            "text" | "paragraph" => Ok(Self::Text),
            "title" => Ok(Self::Title),
            "code" | "codeblock" => Ok(Self::Code),
            "checkboxselected" | "checkbox(selected)" | "checkedbox" => Ok(Self::CheckboxSelected),
            "checkboxunselected" | "checkbox(unselected)" | "uncheckedbox" => Ok(Self::CheckboxUnselected),
            "documentindex" | "index" | "toc" => Ok(Self::DocumentIndex),
            "form" | "formelement" => Ok(Self::Form),
            "keyvalueregion" | "keyvalue" | "kv" => Ok(Self::KeyValueRegion),
            _ => Err(format!(
                "unknown label: '{s}' (expected: caption, footnote, formula, list item, page footer, \
                page header, picture, section header, table, text, title, code, checkbox, \
                document index, form, key-value region)"
            )),
        }
    }
}

/// Bounding box in PDF coordinates (origin at bottom-left)
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct DlvizBBox {
    /// Left edge X coordinate
    pub x: f32,
    /// Bottom edge Y coordinate (PDF coordinates, origin at bottom)
    pub y: f32,
    /// Width
    pub width: f32,
    /// Height
    pub height: f32,
}

/// A detected layout element (bounding box + label + confidence)
#[repr(C)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DlvizElement {
    /// Element unique ID within the page
    pub id: u32,
    /// Bounding box in PDF coordinates
    pub bbox: DlvizBBox,
    /// Element label/class
    pub label: DlvizLabel,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
    /// Reading order index (-1 if not assigned)
    pub reading_order: i32,
}

/// An OCR text cell (character/word with position)
#[repr(C)]
#[derive(Debug, Clone, PartialEq)]
pub struct DlvizTextCell {
    /// Cell unique ID
    pub id: u32,
    /// Bounding box in PDF coordinates
    pub bbox: DlvizBBox,
    /// OCR confidence (0.0 - 1.0)
    pub confidence: f32,
    /// Assigned element ID (-1 if orphan)
    pub element_id: i32,
}

/// Pipeline stage identifier
#[repr(i32)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum DlvizStage {
    /// Raw PDF render (no ML processing)
    #[default]
    RawPdf = 0,
    /// OCR text detection (`DBNet`)
    OcrDetection = 1,
    /// OCR text recognition (CRNN)
    OcrRecognition = 2,
    /// Layout detection (YOLO)
    LayoutDetection = 3,
    /// Cell-to-cluster assignment
    CellAssignment = 4,
    /// Empty cluster removal
    EmptyClusterRemoval = 5,
    /// Orphan cell detection
    OrphanDetection = 6,
    /// `BBox` adjustment iteration 1
    BBoxAdjust1 = 7,
    /// `BBox` adjustment iteration 2
    BBoxAdjust2 = 8,
    /// Final element assembly
    FinalAssembly = 9,
    /// Reading order assignment
    ReadingOrder = 10,
}

impl DlvizStage {
    /// Total number of stages
    pub const COUNT: usize = 11;

    /// Get stage from index
    #[inline]
    #[must_use = "returns stage from numeric index"]
    pub const fn from_index(idx: usize) -> Option<Self> {
        match idx {
            0 => Some(Self::RawPdf),
            1 => Some(Self::OcrDetection),
            2 => Some(Self::OcrRecognition),
            3 => Some(Self::LayoutDetection),
            4 => Some(Self::CellAssignment),
            5 => Some(Self::EmptyClusterRemoval),
            6 => Some(Self::OrphanDetection),
            7 => Some(Self::BBoxAdjust1),
            8 => Some(Self::BBoxAdjust2),
            9 => Some(Self::FinalAssembly),
            10 => Some(Self::ReadingOrder),
            _ => None,
        }
    }
}

impl std::fmt::Display for DlvizStage {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::RawPdf => "raw PDF",
            Self::OcrDetection => "OCR detection",
            Self::OcrRecognition => "OCR recognition",
            Self::LayoutDetection => "layout detection",
            Self::CellAssignment => "cell assignment",
            Self::EmptyClusterRemoval => "empty cluster removal",
            Self::OrphanDetection => "orphan detection",
            Self::BBoxAdjust1 => "bbox adjust 1",
            Self::BBoxAdjust2 => "bbox adjust 2",
            Self::FinalAssembly => "final assembly",
            Self::ReadingOrder => "reading order",
        };
        write!(f, "{s}")
    }
}

impl std::str::FromStr for DlvizStage {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Normalize: lowercase and remove spaces/hyphens/underscores
        let normalized: String = s
            .to_lowercase()
            .chars()
            .filter(|c| !c.is_whitespace() && *c != '-' && *c != '_')
            .collect();
        match normalized.as_str() {
            "rawpdf" | "raw" | "pdf" | "0" => Ok(Self::RawPdf),
            "ocrdetection" | "ocr" | "detection" | "1" => Ok(Self::OcrDetection),
            "ocrrecognition" | "recognition" | "2" => Ok(Self::OcrRecognition),
            "layoutdetection" | "layout" | "3" => Ok(Self::LayoutDetection),
            "cellassignment" | "cell" | "4" => Ok(Self::CellAssignment),
            "emptyclusterremoval" | "emptycluster" | "5" => Ok(Self::EmptyClusterRemoval),
            "orphandetection" | "orphan" | "6" => Ok(Self::OrphanDetection),
            "bboxadjust1" | "adjust1" | "7" => Ok(Self::BBoxAdjust1),
            "bboxadjust2" | "adjust2" | "8" => Ok(Self::BBoxAdjust2),
            "finalassembly" | "final" | "assembly" | "9" => Ok(Self::FinalAssembly),
            "readingorder" | "reading" | "order" | "10" => Ok(Self::ReadingOrder),
            _ => Err(format!(
                "unknown stage: '{s}' (expected: raw pdf, ocr detection, ocr recognition, \
                layout detection, cell assignment, empty cluster removal, orphan detection, \
                bbox adjust 1/2, final assembly, reading order, or stage number 0-10)"
            )),
        }
    }
}

/// Snapshot of pipeline state at a specific stage
#[repr(C)]
pub struct DlvizStageSnapshot {
    /// Stage this snapshot represents
    pub stage: DlvizStage,
    /// Number of elements
    pub element_count: usize,
    /// Elements array (caller must not free)
    pub elements: *const DlvizElement,
    /// Number of text cells
    pub cell_count: usize,
    /// Text cells array (caller must not free)
    pub cells: *const DlvizTextCell,
    /// Processing time for this stage (milliseconds)
    pub processing_time_ms: f64,
}

/// Opaque pipeline handle
pub struct DlvizPipeline {
    /// Loaded PDF state (if any)
    pdf_state: Option<PdfState>,
}

/// Opaque page handle
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct DlvizPage {
    // Internal state
    _private: (),
}

// ============================================================================
// Pipeline Lifecycle
// ============================================================================

/// Create a new pipeline instance
///
/// # Returns
/// Pointer to new pipeline, or null on error
///
/// # Memory
/// Caller must free with `dlviz_pipeline_free`
#[no_mangle]
pub extern "C" fn dlviz_pipeline_new() -> *mut DlvizPipeline {
    Box::into_raw(Box::new(DlvizPipeline { pdf_state: None }))
}

/// Free a pipeline instance
///
/// # Safety
/// - `pipeline` must be a valid pointer from `dlviz_pipeline_new`
/// - `pipeline` must not be used after this call
#[no_mangle]
pub unsafe extern "C" fn dlviz_pipeline_free(pipeline: *mut DlvizPipeline) {
    if !pipeline.is_null() {
        drop(Box::from_raw(pipeline));
    }
}

// ============================================================================
// Feature Detection
// ============================================================================

/// Check if PDF ML pipeline feature is available
///
/// # Returns
/// `true` if the library was built with `--features pdf-ml` and the
/// ML models can be initialized, `false` otherwise.
///
/// # Note
/// Even if this returns `true` at library level, individual pipelines
/// may fail to initialize if model files are missing. Use
/// `dlviz_pipeline_has_ml()` to check a specific pipeline instance.
#[no_mangle]
pub const extern "C" fn dlviz_has_pdf_ml() -> bool {
    cfg!(feature = "pdf-ml")
}

/// Check if a pipeline instance has ML capabilities
///
/// # Arguments
/// - `pipeline`: Pipeline instance
///
/// # Returns
/// `true` if the pipeline has ML models initialized and ready,
/// `false` if ML is not available or models failed to load.
///
/// # Safety
/// - `pipeline` must be a valid pointer from `dlviz_pipeline_new`, or null
#[no_mangle]
pub unsafe extern "C" fn dlviz_pipeline_has_ml(pipeline: *const DlvizPipeline) -> bool {
    if pipeline.is_null() {
        return false;
    }

    let pipeline = &*pipeline;
    pipeline
        .pdf_state
        .as_ref()
        .is_some_and(PdfState::has_ml_pipeline)
}

// ============================================================================
// Document Processing
// ============================================================================

/// Load a PDF document for processing
///
/// # Arguments
/// - `pipeline`: Pipeline instance
/// - `path`: UTF-8 encoded file path (null-terminated)
///
/// # Returns
/// Result code
///
/// # Safety
/// - `pipeline` must be a valid pointer from `dlviz_pipeline_new`
/// - `path` must be a valid null-terminated UTF-8 string
#[no_mangle]
pub unsafe extern "C" fn dlviz_load_pdf(
    pipeline: *mut DlvizPipeline,
    path: *const c_char,
) -> DlvizResult {
    if pipeline.is_null() || path.is_null() {
        return DlvizResult::InvalidArgument;
    }

    let Ok(path_str) = CStr::from_ptr(path).to_str() else {
        return DlvizResult::InvalidArgument;
    };

    let pipeline = &mut *pipeline;

    // Load the PDF using PdfState
    match PdfState::load(path_str) {
        Ok(state) => {
            pipeline.pdf_state = Some(state);
            DlvizResult::Success
        }
        Err(e) => {
            log::error!("Failed to load PDF: {e}");
            if e.contains("not available") {
                // Built without pdf-render feature
                DlvizResult::InternalError
            } else if e.contains("read") || e.contains("not found") {
                DlvizResult::FileNotFound
            } else {
                DlvizResult::ParseError
            }
        }
    }
}

/// Get the number of pages in the loaded document
///
/// # Returns
/// Number of pages, or 0 if no document loaded
///
/// # Safety
/// - `pipeline` must be a valid pointer from `dlviz_pipeline_new`, or null
#[no_mangle]
pub unsafe extern "C" fn dlviz_get_page_count(pipeline: *const DlvizPipeline) -> usize {
    if pipeline.is_null() {
        return 0;
    }

    let pipeline = &*pipeline;
    pipeline.pdf_state.as_ref().map_or(0, PdfState::page_count)
}

/// Get page dimensions
///
/// # Arguments
/// - `pipeline`: Pipeline instance
/// - `page_num`: Page number (0-indexed)
/// - `width`: Output width in points
/// - `height`: Output height in points
///
/// # Returns
/// Result code
///
/// # Safety
/// - `pipeline` must be a valid pointer from `dlviz_pipeline_new`
/// - `width` and `height` must be valid writable pointers
#[no_mangle]
pub unsafe extern "C" fn dlviz_get_page_size(
    pipeline: *const DlvizPipeline,
    page_num: usize,
    width: *mut f32,
    height: *mut f32,
) -> DlvizResult {
    if pipeline.is_null() || width.is_null() || height.is_null() {
        return DlvizResult::InvalidArgument;
    }

    let pipeline = &*pipeline;

    if let Some((w, h)) = pipeline
        .pdf_state
        .as_ref()
        .and_then(|s| s.page_size(page_num))
    {
        *width = w;
        *height = h;
        DlvizResult::Success
    } else {
        // No PDF loaded or page out of range, return default letter size
        *width = US_LETTER_WIDTH_F32;
        *height = US_LETTER_HEIGHT_F32;
        DlvizResult::InvalidArgument
    }
}

/// Render a page to RGBA image buffer
///
/// # Arguments
/// - `pipeline`: Pipeline instance
/// - `page_num`: Page number (0-indexed)
/// - `scale`: Scale factor (1.0 = native resolution)
/// - `width_out`: Output image width
/// - `height_out`: Output image height
/// - `buffer`: Output buffer for RGBA data (4 bytes per pixel)
/// - `buffer_size`: Size of buffer in bytes
///
/// # Returns
/// Result code. If buffer is too small, returns `InvalidArgument` and sets `width_out`/`height_out`
/// to required dimensions.
///
/// # Safety
/// - `pipeline` must be a valid pointer from `dlviz_pipeline_new`
/// - All output pointers must be valid writable locations
/// - `buffer` must have at least `buffer_size` bytes available
#[no_mangle]
pub unsafe extern "C" fn dlviz_render_page(
    pipeline: *const DlvizPipeline,
    page_num: usize,
    scale: f32,
    width_out: *mut u32,
    height_out: *mut u32,
    buffer: *mut u8,
    buffer_size: usize,
) -> DlvizResult {
    if pipeline.is_null() || width_out.is_null() || height_out.is_null() {
        return DlvizResult::InvalidArgument;
    }

    let pipeline = &*pipeline;

    // Attempt to render
    let render_result = pipeline
        .pdf_state
        .as_ref()
        .and_then(|s| s.render_page(page_num, scale));

    if let Some((width, height, rgba_data)) = render_result {
        *width_out = width;
        *height_out = height;

        let required_size = (width * height * 4) as usize;
        if buffer.is_null() || buffer_size < required_size {
            // Buffer too small - caller should allocate more
            return DlvizResult::InvalidArgument;
        }

        // Copy data to output buffer
        std::ptr::copy_nonoverlapping(rgba_data.as_ptr(), buffer, rgba_data.len());
        DlvizResult::Success
    } else {
        *width_out = 0;
        *height_out = 0;
        DlvizResult::ParseError
    }
}

// ============================================================================
// Pipeline Stage Execution
// ============================================================================

/// Run pipeline up to and including the specified stage
///
/// # Arguments
/// - `pipeline`: Pipeline instance
/// - `page_num`: Page number (0-indexed)
/// - `stage`: Target stage
///
/// # Returns
/// Result code
///
/// # Safety
/// - `pipeline` must be a valid pointer from `dlviz_pipeline_new`
#[no_mangle]
pub unsafe extern "C" fn dlviz_run_to_stage(
    pipeline: *mut DlvizPipeline,
    page_num: usize,
    stage: DlvizStage,
) -> DlvizResult {
    if pipeline.is_null() {
        return DlvizResult::InvalidArgument;
    }

    let pipeline = &mut *pipeline;

    // Get PDF state
    let Some(state) = pipeline.pdf_state.as_mut() else {
        log::error!("No PDF loaded");
        return DlvizResult::InvalidArgument;
    };

    // Check if ML pipeline is available
    if !state.has_ml_pipeline() {
        if let Some(err) = state.ml_init_error() {
            log::warn!("ML pipeline not available: {err}");
        }
        // Return success but with no ML data (raw PDF only)
        return DlvizResult::Success;
    }

    // Only run ML if we need it (stage > RawPdf and not already processed)
    if stage != DlvizStage::RawPdf && !state.has_ml_page(page_num) {
        // Run ML pipeline with a reasonable scale (2.0 for good quality)
        const ML_SCALE: f32 = 2.0;
        if let Err(e) = state.run_ml_pipeline(page_num, ML_SCALE) {
            log::error!("ML pipeline error for page {page_num}: {e}");
            return DlvizResult::InferenceError;
        }
    }

    DlvizResult::Success
}

/// Get snapshot of pipeline state at a stage
///
/// # Arguments
/// - `pipeline`: Pipeline instance
/// - `page_num`: Page number (0-indexed)
/// - `stage`: Stage to get snapshot for
/// - `snapshot`: Output snapshot (must be pre-allocated)
///
/// # Returns
/// Result code
///
/// # Note
/// The snapshot contains pointers to internal data that remain valid
/// until the next pipeline operation or free.
///
/// # Safety
/// - `pipeline` must be a valid pointer from `dlviz_pipeline_new`
/// - `snapshot` must be a valid writable pointer to a `DlvizStageSnapshot`
#[no_mangle]
pub unsafe extern "C" fn dlviz_get_stage_snapshot(
    pipeline: *const DlvizPipeline,
    page_num: usize,
    stage: DlvizStage,
    snapshot: *mut DlvizStageSnapshot,
) -> DlvizResult {
    if pipeline.is_null() || snapshot.is_null() {
        return DlvizResult::InvalidArgument;
    }

    let pipeline = &*pipeline;

    // Try to get ML snapshot data
    if let Some(ref state) = pipeline.pdf_state {
        if let Some(data) = state.get_ml_snapshot(page_num, stage) {
            // Return real ML data
            (*snapshot).stage = stage;
            (*snapshot).element_count = data.elements.len();
            (*snapshot).elements = if data.elements.is_empty() {
                ptr::null()
            } else {
                data.elements.as_ptr()
            };
            (*snapshot).cell_count = data.cells.len();
            (*snapshot).cells = if data.cells.is_empty() {
                ptr::null()
            } else {
                data.cells.as_ptr()
            };
            (*snapshot).processing_time_ms = data.processing_time_ms;
            return DlvizResult::Success;
        }
    }

    // No ML data available - return empty snapshot
    (*snapshot).stage = stage;
    (*snapshot).element_count = 0;
    (*snapshot).elements = ptr::null();
    (*snapshot).cell_count = 0;
    (*snapshot).cells = ptr::null();
    (*snapshot).processing_time_ms = 0.0;

    DlvizResult::Success
}

// ============================================================================
// Text Access
// ============================================================================

/// Get text content of an element
///
/// # Arguments
/// - `pipeline`: Pipeline instance
/// - `page_num`: Page number (0-indexed)
/// - `element_id`: Element ID from snapshot
/// - `buffer`: Output buffer for UTF-8 text
/// - `buffer_size`: Size of buffer in bytes
/// - `actual_size`: Output actual size needed (including null terminator)
///
/// # Returns
/// Result code. If buffer is too small, returns Success but `actual_size` > `buffer_size`.
///
/// # Safety
/// - `pipeline` must be a valid pointer from `dlviz_pipeline_new`
/// - `buffer` must be a valid writable buffer of at least `buffer_size` bytes
/// - `actual_size` must be a valid writable pointer
#[no_mangle]
pub unsafe extern "C" fn dlviz_get_element_text(
    pipeline: *const DlvizPipeline,
    page_num: usize,
    element_id: u32,
    buffer: *mut c_char,
    buffer_size: usize,
    actual_size: *mut usize,
) -> DlvizResult {
    if pipeline.is_null() || buffer.is_null() || actual_size.is_null() {
        return DlvizResult::InvalidArgument;
    }

    let pipeline_ref = &*pipeline;

    // Get text from the ML pipeline results
    let text = pipeline_ref
        .pdf_state
        .as_ref()
        .and_then(|pdf_state| pdf_state.get_ml_element_text(page_num, element_id));

    // If no text available, return empty string
    let text_bytes = match text {
        Some(t) => t.as_bytes(),
        None => b"",
    };

    // Set actual size needed (including null terminator)
    *actual_size = text_bytes.len() + 1;

    // Copy text to buffer if it fits
    if buffer_size > 0 {
        let copy_len = std::cmp::min(text_bytes.len(), buffer_size - 1);
        std::ptr::copy_nonoverlapping(text_bytes.as_ptr(), buffer.cast::<u8>(), copy_len);
        // Null-terminate
        *buffer.add(copy_len) = 0;
    }

    DlvizResult::Success
}

/// Get text content of a cell
///
/// # Arguments
/// - `pipeline`: Pipeline instance
/// - `page_num`: Page number (0-indexed)
/// - `cell_id`: Cell ID from snapshot
/// - `buffer`: Output buffer for UTF-8 text
/// - `buffer_size`: Size of buffer in bytes
/// - `actual_size`: Output actual size needed
///
/// # Returns
/// Result code
///
/// # Safety
/// - `pipeline` must be a valid pointer from `dlviz_pipeline_new`
/// - `buffer` must be a valid writable buffer of at least `buffer_size` bytes
/// - `actual_size` must be a valid writable pointer
#[no_mangle]
pub unsafe extern "C" fn dlviz_get_cell_text(
    pipeline: *const DlvizPipeline,
    page_num: usize,
    cell_id: u32,
    buffer: *mut c_char,
    buffer_size: usize,
    actual_size: *mut usize,
) -> DlvizResult {
    if pipeline.is_null() || buffer.is_null() || actual_size.is_null() {
        return DlvizResult::InvalidArgument;
    }

    let _ = (page_num, cell_id); // Suppress unused warnings

    // Placeholder
    *actual_size = 1;
    if buffer_size > 0 {
        *buffer = 0;
    }

    DlvizResult::Success
}

// ============================================================================
// JSON Export
// ============================================================================

/// Export current pipeline state as JSON
///
/// # Arguments
/// - `pipeline`: Pipeline instance
/// - `page_num`: Page number (0-indexed)
///
/// # Returns
/// Pointer to null-terminated JSON string, or null on error.
/// Caller must free with `dlviz_string_free`.
///
/// # Safety
/// - `pipeline` must be a valid pointer from `dlviz_pipeline_new`, or null
#[no_mangle]
pub unsafe extern "C" fn dlviz_export_json(
    pipeline: *const DlvizPipeline,
    page_num: usize,
) -> *mut c_char {
    // Build JSON export structure
    #[derive(Serialize)]
    struct ExportElement {
        id: u32,
        bbox: DlvizBBox,
        label: DlvizLabel,
        confidence: f32,
        reading_order: i32,
        text: Option<String>,
    }

    #[derive(Serialize)]
    struct ExportData {
        page: usize,
        stage: String,
        elements: Vec<ExportElement>,
        element_count: usize,
    }

    if pipeline.is_null() {
        return ptr::null_mut();
    }

    let pipeline_ref = &*pipeline;

    // Get ML snapshot data if available
    if let Some(ref state) = pipeline_ref.pdf_state {
        // Try to get the reading order snapshot (most complete)
        if let Some(data) = state.get_ml_snapshot(page_num, DlvizStage::ReadingOrder) {
            let elements: Vec<ExportElement> = data
                .elements
                .iter()
                .map(|e| {
                    // Get text for this element (convert &str to String)
                    let text = state
                        .get_ml_element_text(page_num, e.id)
                        .map(ToString::to_string);
                    ExportElement {
                        id: e.id,
                        bbox: e.bbox,
                        label: e.label,
                        confidence: e.confidence,
                        reading_order: e.reading_order,
                        text,
                    }
                })
                .collect();

            let export = ExportData {
                page: page_num,
                stage: "reading_order".to_string(),
                element_count: elements.len(),
                elements,
            };

            match serde_json::to_string(&export) {
                Ok(json) => match CString::new(json) {
                    Ok(s) => return s.into_raw(),
                    Err(_) => return ptr::null_mut(),
                },
                Err(_) => return ptr::null_mut(),
            }
        }
    }

    // No data available - return empty structure
    let empty = ExportData {
        page: page_num,
        stage: "none".to_string(),
        elements: vec![],
        element_count: 0,
    };

    serde_json::to_string(&empty).map_or(ptr::null_mut(), |json| {
        CString::new(json).map_or(ptr::null_mut(), |s| s.into_raw())
    })
}

/// Export current pipeline state as pretty-printed JSON
///
/// Same as `dlviz_export_json` but with human-readable formatting.
///
/// # Arguments
/// - `pipeline`: Pipeline instance
/// - `page_num`: Page number (0-indexed)
///
/// # Returns
/// Pointer to null-terminated JSON string with indentation, or null on error.
/// Caller must free with `dlviz_string_free`.
///
/// # Safety
/// - `pipeline` must be a valid pointer from `dlviz_pipeline_new`, or null
#[no_mangle]
pub unsafe extern "C" fn dlviz_export_json_pretty(
    pipeline: *const DlvizPipeline,
    page_num: usize,
) -> *mut c_char {
    // Build JSON export structure
    #[derive(Serialize)]
    struct ExportElement {
        id: u32,
        bbox: DlvizBBox,
        label: DlvizLabel,
        confidence: f32,
        reading_order: i32,
        text: Option<String>,
    }

    #[derive(Serialize)]
    struct ExportData {
        page: usize,
        stage: String,
        elements: Vec<ExportElement>,
        element_count: usize,
    }

    if pipeline.is_null() {
        return ptr::null_mut();
    }

    let pipeline_ref = &*pipeline;

    // Get ML snapshot data if available
    if let Some(ref state) = pipeline_ref.pdf_state {
        // Try to get the reading order snapshot (most complete)
        if let Some(data) = state.get_ml_snapshot(page_num, DlvizStage::ReadingOrder) {
            let elements: Vec<ExportElement> = data
                .elements
                .iter()
                .map(|e| {
                    let text = state
                        .get_ml_element_text(page_num, e.id)
                        .map(ToString::to_string);
                    ExportElement {
                        id: e.id,
                        bbox: e.bbox,
                        label: e.label,
                        confidence: e.confidence,
                        reading_order: e.reading_order,
                        text,
                    }
                })
                .collect();

            let export = ExportData {
                page: page_num,
                stage: "reading_order".to_string(),
                element_count: elements.len(),
                elements,
            };

            match serde_json::to_string_pretty(&export) {
                Ok(json) => match CString::new(json) {
                    Ok(s) => return s.into_raw(),
                    Err(_) => return ptr::null_mut(),
                },
                Err(_) => return ptr::null_mut(),
            }
        }
    }

    // No data available - return empty structure
    let empty = ExportData {
        page: page_num,
        stage: "none".to_string(),
        elements: vec![],
        element_count: 0,
    };

    serde_json::to_string_pretty(&empty).map_or(ptr::null_mut(), |json| {
        CString::new(json).map_or(ptr::null_mut(), |s| s.into_raw())
    })
}

/// Export all processed pages as a single JSON document
///
/// # Arguments
/// - `pipeline`: Pipeline instance
/// - `pretty`: If true, output will be pretty-printed with indentation
///
/// # Returns
/// Pointer to null-terminated JSON string containing all pages, or null on error.
/// Caller must free with `dlviz_string_free`.
///
/// # JSON Structure
/// ```json
/// {
///   "document": "filename.pdf",
///   "page_count": 3,
///   "pages": [
///     { "page": 0, "stage": "reading_order", "elements": [...], "element_count": N },
///     { "page": 1, ... },
///     ...
///   ]
/// }
/// ```
///
/// # Safety
/// - `pipeline` must be a valid pointer from `dlviz_pipeline_new`, or null
#[no_mangle]
pub unsafe extern "C" fn dlviz_export_all_pages_json(
    pipeline: *const DlvizPipeline,
    pretty: bool,
) -> *mut c_char {
    // Build JSON export structures
    #[derive(Serialize)]
    struct ExportElement {
        id: u32,
        bbox: DlvizBBox,
        label: DlvizLabel,
        confidence: f32,
        reading_order: i32,
        text: Option<String>,
    }

    #[derive(Serialize)]
    struct PageData {
        page: usize,
        stage: String,
        elements: Vec<ExportElement>,
        element_count: usize,
    }

    #[derive(Serialize)]
    struct DocumentExport {
        document: Option<String>,
        page_count: usize,
        pages: Vec<PageData>,
    }

    if pipeline.is_null() {
        return ptr::null_mut();
    }

    let pipeline_ref = &*pipeline;

    let mut pages = Vec::new();
    let mut doc_name = None;

    if let Some(ref state) = pipeline_ref.pdf_state {
        let page_count = state.page_count();
        doc_name = state.document_path().map(ToString::to_string);

        for page_num in 0..page_count {
            // Try to get reading order snapshot for each page
            if let Some(data) = state.get_ml_snapshot(page_num, DlvizStage::ReadingOrder) {
                let elements: Vec<ExportElement> = data
                    .elements
                    .iter()
                    .map(|e| {
                        let text = state
                            .get_ml_element_text(page_num, e.id)
                            .map(ToString::to_string);
                        ExportElement {
                            id: e.id,
                            bbox: e.bbox,
                            label: e.label,
                            confidence: e.confidence,
                            reading_order: e.reading_order,
                            text,
                        }
                    })
                    .collect();

                pages.push(PageData {
                    page: page_num,
                    stage: "reading_order".to_string(),
                    element_count: elements.len(),
                    elements,
                });
            } else {
                // Page not processed yet
                pages.push(PageData {
                    page: page_num,
                    stage: "not_processed".to_string(),
                    elements: vec![],
                    element_count: 0,
                });
            }
        }
    }

    let export = DocumentExport {
        document: doc_name,
        page_count: pages.len(),
        pages,
    };

    let json_result = if pretty {
        serde_json::to_string_pretty(&export)
    } else {
        serde_json::to_string(&export)
    };

    json_result.map_or(ptr::null_mut(), |json| {
        CString::new(json).map_or(ptr::null_mut(), |s| s.into_raw())
    })
}

// ============================================================================
// ML Training Data Export (YOLO/COCO formats)
// ============================================================================

/// Export page detection results in YOLO format
///
/// YOLO format: one line per detection with `class_id x_center y_center width height`
/// All coordinates are normalized to 0-1 range.
///
/// # Arguments
/// - `pipeline`: Pipeline instance
/// - `page_num`: Page number (0-indexed)
///
/// # Returns
/// Pointer to null-terminated string in YOLO format, or null on error.
/// Caller must free with `dlviz_string_free`.
///
/// # Safety
/// - `pipeline` must be a valid pointer from `dlviz_pipeline_new`, or null
#[no_mangle]
pub unsafe extern "C" fn dlviz_export_yolo(
    pipeline: *const DlvizPipeline,
    page_num: usize,
) -> *mut c_char {
    if pipeline.is_null() {
        return ptr::null_mut();
    }

    let pipeline_ref = &*pipeline;

    let Some(ref state) = pipeline_ref.pdf_state else {
        return ptr::null_mut();
    };

    let Some((page_width, page_height)) = state.page_size(page_num) else {
        return ptr::null_mut();
    };

    let Some(data) = state.get_ml_snapshot(page_num, DlvizStage::ReadingOrder) else {
        // Return empty string for no data
        return CString::new("").map_or(ptr::null_mut(), |s| s.into_raw());
    };

    let mut output = String::new();
    for elem in &data.elements {
        // Convert to YOLO format
        let yolo = dlviz_bbox_to_yolo(elem.bbox, page_width, page_height);
        // class_id x_center y_center width height
        output.push_str(&format!(
            "{} {:.6} {:.6} {:.6} {:.6}\n",
            elem.label as i32, yolo.x_center, yolo.y_center, yolo.width, yolo.height
        ));
    }

    CString::new(output).map_or(ptr::null_mut(), |s| s.into_raw())
}

/// Export page detection results in COCO annotation format
///
/// Returns a JSON object suitable for inclusion in a COCO dataset's annotations array.
///
/// # Arguments
/// - `pipeline`: Pipeline instance
/// - `page_num`: Page number (0-indexed)
/// - `image_id`: Image ID for the COCO annotation
/// - `annotation_id_start`: Starting annotation ID (increments for each detection)
///
/// # Returns
/// Pointer to null-terminated JSON string, or null on error.
/// Caller must free with `dlviz_string_free`.
///
/// # Safety
/// - `pipeline` must be a valid pointer from `dlviz_pipeline_new`, or null
#[no_mangle]
pub unsafe extern "C" fn dlviz_export_coco_annotations(
    pipeline: *const DlvizPipeline,
    page_num: usize,
    image_id: u32,
    annotation_id_start: u32,
) -> *mut c_char {
    #[derive(Serialize)]
    struct CocoAnnotation {
        id: u32,
        image_id: u32,
        category_id: i32,
        bbox: [f32; 4], // [x, y, width, height] in pixels, top-left origin
        area: f32,
        iscrowd: u8,
        score: f32,
    }

    if pipeline.is_null() {
        return ptr::null_mut();
    }

    let pipeline_ref = &*pipeline;

    let Some(ref state) = pipeline_ref.pdf_state else {
        return ptr::null_mut();
    };

    let Some((_, page_height)) = state.page_size(page_num) else {
        return ptr::null_mut();
    };

    let Some(data) = state.get_ml_snapshot(page_num, DlvizStage::ReadingOrder) else {
        // Return empty array for no data
        return CString::new("[]").map_or(ptr::null_mut(), |s| s.into_raw());
    };

    let mut annotations = Vec::new();
    for (idx, elem) in data.elements.iter().enumerate() {
        let coco_bbox = dlviz_bbox_to_coco(elem.bbox, page_height);
        annotations.push(CocoAnnotation {
            id: annotation_id_start + idx as u32,
            image_id,
            category_id: elem.label as i32,
            bbox: [coco_bbox.x, coco_bbox.y, coco_bbox.width, coco_bbox.height],
            area: coco_bbox.width * coco_bbox.height,
            iscrowd: 0,
            score: elem.confidence,
        });
    }

    serde_json::to_string(&annotations).map_or(ptr::null_mut(), |json| {
        CString::new(json).map_or(ptr::null_mut(), |s| s.into_raw())
    })
}

/// Get COCO categories for DocItem labels
///
/// Returns a JSON array of COCO category objects for all label types.
///
/// # Returns
/// Pointer to null-terminated JSON string, or null on error.
/// Caller must free with `dlviz_string_free`.
#[no_mangle]
pub extern "C" fn dlviz_get_coco_categories() -> *mut c_char {
    #[derive(Serialize)]
    struct CocoCategory {
        id: i32,
        name: &'static str,
        supercategory: &'static str,
    }

    let categories = vec![
        CocoCategory {
            id: 0,
            name: "Caption",
            supercategory: "text",
        },
        CocoCategory {
            id: 1,
            name: "Footnote",
            supercategory: "text",
        },
        CocoCategory {
            id: 2,
            name: "Formula",
            supercategory: "content",
        },
        CocoCategory {
            id: 3,
            name: "ListItem",
            supercategory: "text",
        },
        CocoCategory {
            id: 4,
            name: "PageFooter",
            supercategory: "page_element",
        },
        CocoCategory {
            id: 5,
            name: "PageHeader",
            supercategory: "page_element",
        },
        CocoCategory {
            id: 6,
            name: "Picture",
            supercategory: "content",
        },
        CocoCategory {
            id: 7,
            name: "SectionHeader",
            supercategory: "text",
        },
        CocoCategory {
            id: 8,
            name: "Table",
            supercategory: "content",
        },
        CocoCategory {
            id: 9,
            name: "Text",
            supercategory: "text",
        },
        CocoCategory {
            id: 10,
            name: "Title",
            supercategory: "text",
        },
        CocoCategory {
            id: 11,
            name: "Code",
            supercategory: "content",
        },
        CocoCategory {
            id: 12,
            name: "CheckboxSelected",
            supercategory: "form",
        },
        CocoCategory {
            id: 13,
            name: "CheckboxUnselected",
            supercategory: "form",
        },
        CocoCategory {
            id: 14,
            name: "DocumentIndex",
            supercategory: "text",
        },
        CocoCategory {
            id: 15,
            name: "Form",
            supercategory: "form",
        },
        CocoCategory {
            id: 16,
            name: "KeyValueRegion",
            supercategory: "form",
        },
    ];

    serde_json::to_string(&categories).map_or(ptr::null_mut(), |json| {
        CString::new(json).map_or(ptr::null_mut(), |s| s.into_raw())
    })
}

// ============================================================================
// Visualization Export (AI Visual Testing)
// ============================================================================

/// Render visualization to PNG file with bounding box overlays
///
/// Renders the PDF page with colored bounding boxes for each detected element,
/// plus labels and reading order numbers. Also generates a JSON sidecar file.
///
/// # Arguments
/// - `pipeline`: Pipeline instance with loaded PDF
/// - `page_num`: Page number (0-indexed)
/// - `stage`: Stage to visualize
/// - `scale`: Render scale (2.0 recommended for AI vision)
/// - `output_path`: Path to write PNG file (JSON sidecar written to same name with .json)
///
/// # Returns
/// Result code
///
/// # Safety
/// - `pipeline` must be a valid pointer from `dlviz_pipeline_new`
/// - `output_path` must be a valid null-terminated UTF-8 string
#[no_mangle]
pub unsafe extern "C" fn dlviz_render_visualization(
    pipeline: *mut DlvizPipeline,
    page_num: usize,
    stage: DlvizStage,
    scale: f32,
    output_path: *const c_char,
) -> DlvizResult {
    use std::path::Path;
    use std::time::Instant;

    if pipeline.is_null() || output_path.is_null() {
        return DlvizResult::InvalidArgument;
    }

    let pipeline = &mut *pipeline;

    // Parse output path
    let Ok(output_path_str) = CStr::from_ptr(output_path).to_str() else {
        return DlvizResult::InvalidArgument;
    };
    let output_path = Path::new(output_path_str);

    // Get PDF state
    let Some(state) = pipeline.pdf_state.as_mut() else {
        log::error!("No PDF loaded");
        return DlvizResult::InvalidArgument;
    };

    // Check page bounds
    let page_count = state.page_count();
    if page_num >= page_count {
        log::error!("Page {page_num} out of bounds (total: {page_count})");
        return DlvizResult::InvalidArgument;
    }

    let start_time = Instant::now();

    // Run pipeline to requested stage
    if let Err(e) = state.run_ml_pipeline(page_num, scale) {
        log::error!("ML pipeline error: {e}");
        return DlvizResult::InferenceError;
    }

    // Render page to buffer
    let Some((page_width, page_height, page_buffer)) = state.render_page(page_num, scale) else {
        log::error!("Failed to render page {page_num}");
        return DlvizResult::ParseError;
    };

    // Get page dimensions for coordinate conversion
    let Some(page_size) = state.page_size(page_num) else {
        return DlvizResult::InvalidArgument;
    };

    // Get elements for the requested stage
    let (elements, element_texts_vec): (Vec<DlvizElement>, Vec<Option<String>>) =
        state.get_ml_snapshot(page_num, stage).map_or_else(
            || {
                log::warn!("No snapshot available for stage {stage:?}");
                (vec![], vec![])
            },
            |s| {
                let texts: Vec<Option<String>> = s
                    .elements
                    .iter()
                    .map(|e| {
                        state
                            .get_ml_element_text(page_num, e.id)
                            .map(ToString::to_string)
                    })
                    .collect();
                (s.elements.clone(), texts)
            },
        );

    // Render visualization with overlays
    let options = visualization::RenderOptions::default();
    let img = visualization::render_visualization(
        &page_buffer,
        page_width,
        page_height,
        page_size.1, // page height in points
        scale,
        &elements,
        &options,
    );

    // Save PNG
    if let Err(e) = visualization::save_visualization(&img, output_path) {
        log::error!("Failed to save visualization: {e}");
        return DlvizResult::InternalError;
    }

    // Generate and save JSON sidecar
    let pdf_name = state
        .document_path()
        .and_then(|p| std::path::Path::new(p).file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("unknown.pdf");
    let render_time_ms = start_time.elapsed().as_secs_f64() * 1000.0;
    let sidecar = visualization::generate_sidecar(
        pdf_name,
        page_num,
        page_size,
        stage,
        render_time_ms,
        &elements,
        &element_texts_vec,
    );

    let json_path = output_path.with_extension("json");
    if let Err(e) = visualization::save_sidecar(&sidecar, &json_path) {
        log::error!("Failed to save sidecar: {e}");
        // Not a fatal error, PNG was saved successfully
    }

    DlvizResult::Success
}

/// Render all pages of a PDF to visualization PNGs
///
/// Processes all pages and saves to `output_dir` as:
/// - `{stem}_page_{N:03}_{stage}.png`
/// - `{stem}_page_{N:03}_{stage}.json`
///
/// # Arguments
/// - `pipeline`: Pipeline instance with loaded PDF
/// - `stage`: Stage to visualize
/// - `scale`: Render scale (2.0 recommended)
/// - `output_dir`: Directory to write output files
///
/// # Returns
/// Number of pages processed, or negative error code
///
/// # Safety
/// - `pipeline` must be a valid pointer from `dlviz_pipeline_new`
/// - `output_dir` must be a valid null-terminated UTF-8 string
#[no_mangle]
pub unsafe extern "C" fn dlviz_render_all_pages(
    pipeline: *mut DlvizPipeline,
    stage: DlvizStage,
    scale: f32,
    output_dir: *const c_char,
) -> i32 {
    use std::path::Path;

    if pipeline.is_null() || output_dir.is_null() {
        return DlvizResult::InvalidArgument as i32;
    }

    let pipeline_ref = &mut *pipeline;

    // Parse output dir
    let Ok(output_dir_str) = CStr::from_ptr(output_dir).to_str() else {
        return DlvizResult::InvalidArgument as i32;
    };
    let output_dir = Path::new(output_dir_str);

    // Create output directory if needed
    if let Err(e) = std::fs::create_dir_all(output_dir) {
        log::error!("Failed to create output directory: {e}");
        return DlvizResult::InternalError as i32;
    }

    // Get page count
    let Some(state) = pipeline_ref.pdf_state.as_ref() else {
        return DlvizResult::InvalidArgument as i32;
    };
    let page_count = state.page_count();

    // Get PDF stem for naming
    let pdf_stem = state
        .document_path()
        .and_then(|p| std::path::Path::new(p).file_stem())
        .and_then(|n| n.to_str())
        .unwrap_or("document");

    let stage_name = match stage {
        DlvizStage::RawPdf => "raw",
        DlvizStage::OcrDetection => "ocr_det",
        DlvizStage::OcrRecognition => "ocr_rec",
        DlvizStage::LayoutDetection => "layout",
        DlvizStage::CellAssignment => "cells",
        DlvizStage::EmptyClusterRemoval => "empty",
        DlvizStage::OrphanDetection => "orphan",
        DlvizStage::BBoxAdjust1 => "bbox1",
        DlvizStage::BBoxAdjust2 => "bbox2",
        DlvizStage::FinalAssembly => "final",
        DlvizStage::ReadingOrder => "reading",
    };

    let mut processed = 0i32;

    for page_num in 0..page_count {
        let output_path =
            output_dir.join(format!("{pdf_stem}_page_{page_num:03}_{stage_name}.png"));
        let output_path_str = output_path.to_string_lossy();
        let Ok(output_path_cstr) = CString::new(output_path_str.as_bytes()) else {
            continue;
        };

        let result =
            dlviz_render_visualization(pipeline, page_num, stage, scale, output_path_cstr.as_ptr());

        if result == DlvizResult::Success {
            processed += 1;
        } else {
            log::warn!("Failed to render page {page_num}: {result:?}");
        }
    }

    processed
}

/// Free a string returned by FFI functions
///
/// # Safety
/// - `s` must be a valid pointer from a dlviz_* function
/// - `s` must not be used after this call
#[no_mangle]
pub unsafe extern "C" fn dlviz_string_free(s: *mut c_char) {
    if !s.is_null() {
        drop(CString::from_raw(s));
    }
}

// ============================================================================
// Version Info
// ============================================================================

/// Get library version string
///
/// # Returns
/// Static version string (do not free)
#[no_mangle]
pub extern "C" fn dlviz_version() -> *const c_char {
    static VERSION: &[u8] = b"0.1.0\0";
    VERSION.as_ptr().cast::<c_char>()
}

/// Get number of pipeline stages
#[no_mangle]
pub const extern "C" fn dlviz_stage_count() -> usize {
    DlvizStage::COUNT
}

/// Check if PDF rendering is available
///
/// Returns true if the library was built with pdfium-render support.
#[no_mangle]
pub const extern "C" fn dlviz_has_pdf_render() -> bool {
    cfg!(feature = "pdf-render")
}

/// Get stage name
///
/// # Arguments
/// - `stage`: Stage to get name for
///
/// # Returns
/// Static stage name string (do not free)
#[no_mangle]
pub extern "C" fn dlviz_stage_name(stage: DlvizStage) -> *const c_char {
    static NAMES: [&[u8]; 11] = [
        b"Raw PDF\0",
        b"OCR Detection\0",
        b"OCR Recognition\0",
        b"Layout Detection\0",
        b"Cell Assignment\0",
        b"Empty Cluster Removal\0",
        b"Orphan Detection\0",
        b"BBox Adjust 1\0",
        b"BBox Adjust 2\0",
        b"Final Assembly\0",
        b"Reading Order\0",
    ];

    static UNKNOWN: &[u8] = b"Unknown\0";

    let idx = stage as usize;
    if idx < NAMES.len() {
        NAMES[idx].as_ptr().cast::<c_char>()
    } else {
        UNKNOWN.as_ptr().cast::<c_char>()
    }
}

// ============================================================================
// Label Information
// ============================================================================

/// Get the total number of label types
///
/// # Returns
/// Number of distinct label types (17)
#[no_mangle]
pub const extern "C" fn dlviz_label_count() -> usize {
    17 // DlvizLabel has 17 variants (0-16)
}

/// Get the display name for a label
///
/// # Arguments
/// - `label`: Label to get name for
///
/// # Returns
/// Static label name string (do not free)
#[no_mangle]
pub extern "C" fn dlviz_label_name(label: DlvizLabel) -> *const c_char {
    static NAMES: [&[u8]; 17] = [
        b"Caption\0",
        b"Footnote\0",
        b"Formula\0",
        b"List Item\0",
        b"Page Footer\0",
        b"Page Header\0",
        b"Picture\0",
        b"Section Header\0",
        b"Table\0",
        b"Text\0",
        b"Title\0",
        b"Code\0",
        b"Checkbox (Selected)\0",
        b"Checkbox (Unselected)\0",
        b"Document Index\0",
        b"Form\0",
        b"Key-Value Region\0",
    ];

    static UNKNOWN: &[u8] = b"Unknown\0";

    let idx = label as usize;
    if idx < NAMES.len() {
        NAMES[idx].as_ptr().cast::<c_char>()
    } else {
        UNKNOWN.as_ptr().cast::<c_char>()
    }
}

/// Get the short name for a label (for YOLO/COCO class names)
///
/// # Arguments
/// - `label`: Label to get short name for
///
/// # Returns
/// Static short name string suitable for annotation files (do not free)
#[no_mangle]
pub extern "C" fn dlviz_label_short_name(label: DlvizLabel) -> *const c_char {
    static NAMES: [&[u8]; 17] = [
        b"caption\0",
        b"footnote\0",
        b"formula\0",
        b"list_item\0",
        b"page_footer\0",
        b"page_header\0",
        b"picture\0",
        b"section_header\0",
        b"table\0",
        b"text\0",
        b"title\0",
        b"code\0",
        b"checkbox_selected\0",
        b"checkbox_unselected\0",
        b"document_index\0",
        b"form\0",
        b"key_value_region\0",
    ];

    static UNKNOWN: &[u8] = b"unknown\0";

    let idx = label as usize;
    if idx < NAMES.len() {
        NAMES[idx].as_ptr().cast::<c_char>()
    } else {
        UNKNOWN.as_ptr().cast::<c_char>()
    }
}

/// RGBA color for visualization (C-compatible)
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct DlvizColor {
    /// Red component (0-255)
    pub r: u8,
    /// Green component (0-255)
    pub g: u8,
    /// Blue component (0-255)
    pub b: u8,
    /// Alpha component (0-255, 255 = opaque)
    pub a: u8,
}

/// Get the visualization color for a label
///
/// # Arguments
/// - `label`: Label to get color for
///
/// # Returns
/// RGBA color for the label
#[no_mangle]
pub const extern "C" fn dlviz_label_color(label: DlvizLabel) -> DlvizColor {
    match label {
        DlvizLabel::Caption => DlvizColor {
            r: 255,
            g: 165,
            b: 0,
            a: 255,
        }, // Orange
        DlvizLabel::Footnote => DlvizColor {
            r: 128,
            g: 128,
            b: 128,
            a: 255,
        }, // Gray
        DlvizLabel::Formula => DlvizColor {
            r: 0,
            g: 255,
            b: 255,
            a: 255,
        }, // Cyan
        DlvizLabel::ListItem => DlvizColor {
            r: 144,
            g: 238,
            b: 144,
            a: 255,
        }, // Light green
        DlvizLabel::PageFooter | DlvizLabel::PageHeader => DlvizColor {
            r: 192,
            g: 192,
            b: 192,
            a: 255,
        }, // Silver
        DlvizLabel::Picture => DlvizColor {
            r: 255,
            g: 0,
            b: 255,
            a: 255,
        }, // Magenta
        DlvizLabel::SectionHeader => DlvizColor {
            r: 0,
            g: 0,
            b: 255,
            a: 255,
        }, // Blue
        DlvizLabel::Table => DlvizColor {
            r: 0,
            g: 255,
            b: 0,
            a: 255,
        }, // Green
        DlvizLabel::Text => DlvizColor {
            r: 255,
            g: 255,
            b: 0,
            a: 255,
        }, // Yellow
        DlvizLabel::Title => DlvizColor {
            r: 255,
            g: 0,
            b: 0,
            a: 255,
        }, // Red
        DlvizLabel::Code => DlvizColor {
            r: 128,
            g: 0,
            b: 128,
            a: 255,
        }, // Purple
        DlvizLabel::CheckboxSelected => DlvizColor {
            r: 0,
            g: 128,
            b: 0,
            a: 255,
        }, // Dark green
        DlvizLabel::CheckboxUnselected => DlvizColor {
            r: 128,
            g: 0,
            b: 0,
            a: 255,
        }, // Dark red
        DlvizLabel::DocumentIndex => DlvizColor {
            r: 0,
            g: 128,
            b: 128,
            a: 255,
        }, // Teal
        DlvizLabel::Form => DlvizColor {
            r: 255,
            g: 192,
            b: 203,
            a: 255,
        }, // Pink
        DlvizLabel::KeyValueRegion => DlvizColor {
            r: 255,
            g: 215,
            b: 0,
            a: 255,
        }, // Gold
    }
}

/// Get label from integer index
///
/// # Arguments
/// - `index`: Label index (0-16)
///
/// # Returns
/// The label, or -1 if index is out of range
#[no_mangle]
pub const extern "C" fn dlviz_label_from_index(index: usize) -> i32 {
    if index < 17 {
        index as i32
    } else {
        -1
    }
}

// ============================================================================
// Annotation Format Conversion
// ============================================================================

/// YOLO format bounding box (normalized 0-1 coordinates)
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct DlvizYoloBBox {
    /// Center X coordinate (0.0 - 1.0)
    pub x_center: f32,
    /// Center Y coordinate (0.0 - 1.0)
    pub y_center: f32,
    /// Width (0.0 - 1.0)
    pub width: f32,
    /// Height (0.0 - 1.0)
    pub height: f32,
}

/// COCO format bounding box (absolute pixel coordinates)
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct DlvizCocoBBox {
    /// Top-left X coordinate
    pub x: f32,
    /// Top-left Y coordinate
    pub y: f32,
    /// Width
    pub width: f32,
    /// Height
    pub height: f32,
    /// Area (width * height)
    pub area: f32,
}

/// Pascal VOC format bounding box (absolute pixel coordinates)
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct DlvizVocBBox {
    /// Minimum X coordinate (left edge)
    pub xmin: f32,
    /// Minimum Y coordinate (top edge)
    pub ymin: f32,
    /// Maximum X coordinate (right edge)
    pub xmax: f32,
    /// Maximum Y coordinate (bottom edge)
    pub ymax: f32,
}

/// Convert a PDF bounding box to YOLO format
///
/// YOLO format uses normalized coordinates (0-1) with center point.
/// PDF coordinates have origin at bottom-left, but YOLO uses top-left.
///
/// # Arguments
/// - `bbox`: PDF bounding box (origin at bottom-left)
/// - `page_width`: Page width in points
/// - `page_height`: Page height in points
///
/// # Returns
/// YOLO format bounding box (center point, normalized)
#[no_mangle]
pub extern "C" fn dlviz_bbox_to_yolo(
    bbox: DlvizBBox,
    page_width: f32,
    page_height: f32,
) -> DlvizYoloBBox {
    // PDF origin is bottom-left, YOLO uses top-left
    // Convert Y coordinates: yolo_y = page_height - pdf_y - height
    let yolo_top = page_height - bbox.y - bbox.height;

    DlvizYoloBBox {
        x_center: (bbox.x + bbox.width / 2.0) / page_width,
        y_center: (yolo_top + bbox.height / 2.0) / page_height,
        width: bbox.width / page_width,
        height: bbox.height / page_height,
    }
}

/// Convert a YOLO bounding box back to PDF format
///
/// # Arguments
/// - `yolo`: YOLO format bounding box
/// - `page_width`: Page width in points
/// - `page_height`: Page height in points
///
/// # Returns
/// PDF format bounding box (origin at bottom-left)
#[no_mangle]
pub extern "C" fn dlviz_yolo_to_bbox(
    yolo: DlvizYoloBBox,
    page_width: f32,
    page_height: f32,
) -> DlvizBBox {
    let width = yolo.width * page_width;
    let height = yolo.height * page_height;
    let x = yolo.x_center.mul_add(page_width, -(width / 2.0));
    let yolo_top = yolo.y_center.mul_add(page_height, -(height / 2.0));

    // Convert from YOLO top-left origin to PDF bottom-left origin
    let pdf_y = page_height - yolo_top - height;

    DlvizBBox {
        x,
        y: pdf_y,
        width,
        height,
    }
}

/// Convert a PDF bounding box to COCO format
///
/// COCO format uses absolute coordinates with origin at top-left.
/// Returns [x, y, width, height] where (x, y) is the top-left corner.
///
/// # Arguments
/// - `bbox`: PDF bounding box (origin at bottom-left)
/// - `page_height`: Page height in points (needed for Y coordinate conversion)
///
/// # Returns
/// COCO format bounding box with area pre-calculated
#[no_mangle]
pub extern "C" fn dlviz_bbox_to_coco(bbox: DlvizBBox, page_height: f32) -> DlvizCocoBBox {
    // PDF origin is bottom-left, COCO uses top-left
    let coco_y = page_height - bbox.y - bbox.height;

    DlvizCocoBBox {
        x: bbox.x,
        y: coco_y,
        width: bbox.width,
        height: bbox.height,
        area: bbox.width * bbox.height,
    }
}

/// Convert a COCO bounding box back to PDF format
///
/// # Arguments
/// - `coco`: COCO format bounding box
/// - `page_height`: Page height in points
///
/// # Returns
/// PDF format bounding box (origin at bottom-left)
#[no_mangle]
pub extern "C" fn dlviz_coco_to_bbox(coco: DlvizCocoBBox, page_height: f32) -> DlvizBBox {
    // Convert from COCO top-left origin to PDF bottom-left origin
    let pdf_y = page_height - coco.y - coco.height;

    DlvizBBox {
        x: coco.x,
        y: pdf_y,
        width: coco.width,
        height: coco.height,
    }
}

/// Convert a PDF bounding box to Pascal VOC format
///
/// VOC format uses absolute coordinates (xmin, ymin, xmax, ymax) with origin at top-left.
///
/// # Arguments
/// - `bbox`: PDF bounding box (origin at bottom-left)
/// - `page_height`: Page height in points
///
/// # Returns
/// Pascal VOC format bounding box
#[no_mangle]
pub extern "C" fn dlviz_bbox_to_voc(bbox: DlvizBBox, page_height: f32) -> DlvizVocBBox {
    // PDF origin is bottom-left, VOC uses top-left
    let ymin = page_height - bbox.y - bbox.height;
    let ymax = page_height - bbox.y;

    DlvizVocBBox {
        xmin: bbox.x,
        ymin,
        xmax: bbox.x + bbox.width,
        ymax,
    }
}

/// Convert a Pascal VOC bounding box back to PDF format
///
/// # Arguments
/// - `voc`: Pascal VOC format bounding box
/// - `page_height`: Page height in points
///
/// # Returns
/// PDF format bounding box (origin at bottom-left)
#[no_mangle]
pub extern "C" fn dlviz_voc_to_bbox(voc: DlvizVocBBox, page_height: f32) -> DlvizBBox {
    let width = voc.xmax - voc.xmin;
    let height = voc.ymax - voc.ymin;
    // Convert from VOC top-left origin to PDF bottom-left origin
    let pdf_y = page_height - voc.ymax;

    DlvizBBox {
        x: voc.xmin,
        y: pdf_y,
        width,
        height,
    }
}

/// Calculate the Intersection over Union (`IoU`) of two bounding boxes
///
/// `IoU` is commonly used to evaluate detection accuracy.
///
/// # Arguments
/// - `bbox1`: First bounding box (PDF coordinates)
/// - `bbox2`: Second bounding box (PDF coordinates)
///
/// # Returns
/// `IoU` value between 0.0 and 1.0
#[no_mangle]
pub extern "C" fn dlviz_bbox_iou(bbox1: DlvizBBox, bbox2: DlvizBBox) -> f32 {
    // Calculate intersection
    let x1 = bbox1.x.max(bbox2.x);
    let y1 = bbox1.y.max(bbox2.y);
    let x2 = (bbox1.x + bbox1.width).min(bbox2.x + bbox2.width);
    let y2 = (bbox1.y + bbox1.height).min(bbox2.y + bbox2.height);

    // Check if there's an intersection
    if x2 <= x1 || y2 <= y1 {
        return 0.0;
    }

    let intersection = (x2 - x1) * (y2 - y1);
    let area1 = bbox1.width * bbox1.height;
    let area2 = bbox2.width * bbox2.height;
    let union = area1 + area2 - intersection;

    if union <= 0.0 {
        0.0
    } else {
        intersection / union
    }
}

/// Check if two bounding boxes overlap
///
/// # Arguments
/// - `bbox1`: First bounding box
/// - `bbox2`: Second bounding box
///
/// # Returns
/// True if the boxes overlap, false otherwise
#[no_mangle]
pub extern "C" fn dlviz_bbox_overlaps(bbox1: DlvizBBox, bbox2: DlvizBBox) -> bool {
    let x1_max = bbox1.x + bbox1.width;
    let y1_max = bbox1.y + bbox1.height;
    let x2_max = bbox2.x + bbox2.width;
    let y2_max = bbox2.y + bbox2.height;

    // No overlap if one box is completely to the left/right/above/below the other
    !(x1_max <= bbox2.x || bbox1.x >= x2_max || y1_max <= bbox2.y || bbox1.y >= y2_max)
}

/// Check if one bounding box contains another
///
/// # Arguments
/// - `outer`: Outer bounding box
/// - `inner`: Inner bounding box
///
/// # Returns
/// True if outer completely contains inner
#[no_mangle]
pub extern "C" fn dlviz_bbox_contains(outer: DlvizBBox, inner: DlvizBBox) -> bool {
    outer.x <= inner.x
        && outer.y <= inner.y
        && outer.x + outer.width >= inner.x + inner.width
        && outer.y + outer.height >= inner.y + inner.height
}

// ============================================================================
// Element Statistics and Analysis
// ============================================================================

/// Statistics about a collection of elements
///
/// Contains counts for each label type
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct DlvizElementStats {
    /// Total number of elements
    pub total_count: usize,
    /// Count of Caption elements
    pub caption_count: usize,
    /// Count of Footnote elements
    pub footnote_count: usize,
    /// Count of Formula elements
    pub formula_count: usize,
    /// Count of `ListItem` elements
    pub list_item_count: usize,
    /// Count of `PageFooter` elements
    pub page_footer_count: usize,
    /// Count of `PageHeader` elements
    pub page_header_count: usize,
    /// Count of `Picture` elements
    pub picture_count: usize,
    /// Count of `SectionHeader` elements
    pub section_header_count: usize,
    /// Count of `Table` elements
    pub table_count: usize,
    /// Count of `Text` elements
    pub text_count: usize,
    /// Count of `Title` elements
    pub title_count: usize,
    /// Count of `Code` elements
    pub code_count: usize,
    /// Count of `CheckboxSelected` elements
    pub checkbox_selected_count: usize,
    /// Count of `CheckboxUnselected` elements
    pub checkbox_unselected_count: usize,
    /// Count of `DocumentIndex` elements
    pub document_index_count: usize,
    /// Count of `Form` elements
    pub form_count: usize,
    /// Count of `KeyValueRegion` elements
    pub key_value_region_count: usize,
    /// Average confidence score across all elements
    pub avg_confidence: f32,
    /// Minimum confidence score
    pub min_confidence: f32,
    /// Maximum confidence score
    pub max_confidence: f32,
}

/// Compute statistics about an array of elements
///
/// # Arguments
/// - `elements`: Pointer to array of elements
/// - `count`: Number of elements in the array
/// - `stats`: Output pointer to store computed statistics
///
/// # Returns
/// `DlvizResult::Success` on success, error code on failure
///
/// # Safety
/// - `elements` must be valid for `count` elements or null (if count is 0)
/// - `stats` must be a valid writable pointer
#[no_mangle]
pub unsafe extern "C" fn dlviz_compute_element_stats(
    elements: *const DlvizElement,
    count: usize,
    stats: *mut DlvizElementStats,
) -> DlvizResult {
    if stats.is_null() {
        return DlvizResult::InvalidArgument;
    }

    // Initialize stats with proper defaults for min/max tracking
    let mut result = DlvizElementStats {
        min_confidence: f32::MAX,
        max_confidence: f32::MIN,
        ..Default::default()
    };

    if count == 0 || elements.is_null() {
        result.min_confidence = 0.0;
        result.max_confidence = 0.0;
        *stats = result;
        return DlvizResult::Success;
    }

    result.total_count = count;
    let mut confidence_sum = 0.0f32;

    let elements_slice = std::slice::from_raw_parts(elements, count);
    for elem in elements_slice {
        // Count by label
        match elem.label {
            DlvizLabel::Caption => result.caption_count += 1,
            DlvizLabel::Footnote => result.footnote_count += 1,
            DlvizLabel::Formula => result.formula_count += 1,
            DlvizLabel::ListItem => result.list_item_count += 1,
            DlvizLabel::PageFooter => result.page_footer_count += 1,
            DlvizLabel::PageHeader => result.page_header_count += 1,
            DlvizLabel::Picture => result.picture_count += 1,
            DlvizLabel::SectionHeader => result.section_header_count += 1,
            DlvizLabel::Table => result.table_count += 1,
            DlvizLabel::Text => result.text_count += 1,
            DlvizLabel::Title => result.title_count += 1,
            DlvizLabel::Code => result.code_count += 1,
            DlvizLabel::CheckboxSelected => result.checkbox_selected_count += 1,
            DlvizLabel::CheckboxUnselected => result.checkbox_unselected_count += 1,
            DlvizLabel::DocumentIndex => result.document_index_count += 1,
            DlvizLabel::Form => result.form_count += 1,
            DlvizLabel::KeyValueRegion => result.key_value_region_count += 1,
        }

        // Track confidence
        confidence_sum += elem.confidence;
        if elem.confidence < result.min_confidence {
            result.min_confidence = elem.confidence;
        }
        if elem.confidence > result.max_confidence {
            result.max_confidence = elem.confidence;
        }
    }

    result.avg_confidence = confidence_sum / count as f32;

    *stats = result;
    DlvizResult::Success
}

/// Get count for a specific label from statistics
///
/// # Arguments
/// - `stats`: Pointer to computed statistics
/// - `label`: Label to get count for
///
/// # Returns
/// Count for the specified label, or 0 if stats is null
///
/// # Safety
/// - `stats` must be a valid pointer to a `DlvizElementStats` struct or null
#[no_mangle]
pub const unsafe extern "C" fn dlviz_stats_get_label_count(
    stats: *const DlvizElementStats,
    label: DlvizLabel,
) -> usize {
    if stats.is_null() {
        return 0;
    }

    let stats = &*stats;
    match label {
        DlvizLabel::Caption => stats.caption_count,
        DlvizLabel::Footnote => stats.footnote_count,
        DlvizLabel::Formula => stats.formula_count,
        DlvizLabel::ListItem => stats.list_item_count,
        DlvizLabel::PageFooter => stats.page_footer_count,
        DlvizLabel::PageHeader => stats.page_header_count,
        DlvizLabel::Picture => stats.picture_count,
        DlvizLabel::SectionHeader => stats.section_header_count,
        DlvizLabel::Table => stats.table_count,
        DlvizLabel::Text => stats.text_count,
        DlvizLabel::Title => stats.title_count,
        DlvizLabel::Code => stats.code_count,
        DlvizLabel::CheckboxSelected => stats.checkbox_selected_count,
        DlvizLabel::CheckboxUnselected => stats.checkbox_unselected_count,
        DlvizLabel::DocumentIndex => stats.document_index_count,
        DlvizLabel::Form => stats.form_count,
        DlvizLabel::KeyValueRegion => stats.key_value_region_count,
    }
}

// ============================================================================
// Bbox Transformation Utilities
// ============================================================================

/// Scale a bounding box by a factor
///
/// Useful for converting between different DPI/resolution coordinate systems.
///
/// # Arguments
/// - `bbox`: Original bounding box
/// - `scale`: Scale factor (e.g., 2.0 doubles size, 0.5 halves size)
///
/// # Returns
/// Scaled bounding box
#[no_mangle]
pub extern "C" fn dlviz_bbox_scale(bbox: DlvizBBox, scale: f32) -> DlvizBBox {
    DlvizBBox {
        x: bbox.x * scale,
        y: bbox.y * scale,
        width: bbox.width * scale,
        height: bbox.height * scale,
    }
}

/// Translate a bounding box by an offset
///
/// # Arguments
/// - `bbox`: Original bounding box
/// - `dx`: X offset (positive moves right)
/// - `dy`: Y offset (positive moves up in PDF coordinates)
///
/// # Returns
/// Translated bounding box
#[no_mangle]
pub extern "C" fn dlviz_bbox_translate(bbox: DlvizBBox, dx: f32, dy: f32) -> DlvizBBox {
    DlvizBBox {
        x: bbox.x + dx,
        y: bbox.y + dy,
        width: bbox.width,
        height: bbox.height,
    }
}

/// Compute the union of two bounding boxes
///
/// Returns the smallest bounding box that contains both inputs.
///
/// # Arguments
/// - `bbox1`: First bounding box
/// - `bbox2`: Second bounding box
///
/// # Returns
/// Union bounding box
#[no_mangle]
pub extern "C" fn dlviz_bbox_union(bbox1: DlvizBBox, bbox2: DlvizBBox) -> DlvizBBox {
    let x_min = bbox1.x.min(bbox2.x);
    let y_min = bbox1.y.min(bbox2.y);
    let x_max = (bbox1.x + bbox1.width).max(bbox2.x + bbox2.width);
    let y_max = (bbox1.y + bbox1.height).max(bbox2.y + bbox2.height);

    DlvizBBox {
        x: x_min,
        y: y_min,
        width: x_max - x_min,
        height: y_max - y_min,
    }
}

/// Compute the intersection of two bounding boxes
///
/// Returns the overlapping region, or a zero-sized box if no overlap.
///
/// # Arguments
/// - `bbox1`: First bounding box
/// - `bbox2`: Second bounding box
///
/// # Returns
/// Intersection bounding box (width/height may be 0 if no overlap)
#[no_mangle]
pub extern "C" fn dlviz_bbox_intersection(bbox1: DlvizBBox, bbox2: DlvizBBox) -> DlvizBBox {
    let x_min = bbox1.x.max(bbox2.x);
    let y_min = bbox1.y.max(bbox2.y);
    let x_max = (bbox1.x + bbox1.width).min(bbox2.x + bbox2.width);
    let y_max = (bbox1.y + bbox1.height).min(bbox2.y + bbox2.height);

    let width = (x_max - x_min).max(0.0);
    let height = (y_max - y_min).max(0.0);

    DlvizBBox {
        x: if width > 0.0 { x_min } else { 0.0 },
        y: if height > 0.0 { y_min } else { 0.0 },
        width,
        height,
    }
}

/// Compute the area of a bounding box
///
/// # Arguments
/// - `bbox`: Bounding box
///
/// # Returns
/// Area (width * height)
#[no_mangle]
pub extern "C" fn dlviz_bbox_area(bbox: DlvizBBox) -> f32 {
    bbox.width * bbox.height
}

/// Check if a bounding box is valid (non-negative dimensions)
///
/// # Arguments
/// - `bbox`: Bounding box to validate
///
/// # Returns
/// True if width and height are both non-negative
#[no_mangle]
pub extern "C" fn dlviz_bbox_is_valid(bbox: DlvizBBox) -> bool {
    bbox.width >= 0.0 && bbox.height >= 0.0
}

/// Expand a bounding box by a margin on all sides
///
/// # Arguments
/// - `bbox`: Original bounding box
/// - `margin`: Margin to add on all sides (can be negative to shrink)
///
/// # Returns
/// Expanded bounding box
#[no_mangle]
pub extern "C" fn dlviz_bbox_expand(bbox: DlvizBBox, margin: f32) -> DlvizBBox {
    DlvizBBox {
        x: bbox.x - margin,
        y: bbox.y - margin,
        width: margin.mul_add(2.0, bbox.width).max(0.0),
        height: margin.mul_add(2.0, bbox.height).max(0.0),
    }
}

/// Get the center point of a bounding box
///
/// # Arguments
/// - `bbox`: Bounding box
/// - `cx`: Output X coordinate of center
/// - `cy`: Output Y coordinate of center
///
/// # Safety
/// - `cx` must be a valid writable pointer or null
/// - `cy` must be a valid writable pointer or null
#[no_mangle]
pub unsafe extern "C" fn dlviz_bbox_center(bbox: DlvizBBox, cx: *mut f32, cy: *mut f32) {
    if !cx.is_null() {
        *cx = bbox.x + bbox.width / 2.0;
    }
    if !cy.is_null() {
        *cy = bbox.y + bbox.height / 2.0;
    }
}

// ============================================================================
// Spatial Query Functions
// ============================================================================

/// Check if a point is inside a bounding box
///
/// # Arguments
/// - `bbox`: Bounding box to test
/// - `x`: X coordinate of point
/// - `y`: Y coordinate of point
///
/// # Returns
/// True if point is inside or on the edge of the bbox
#[no_mangle]
pub extern "C" fn dlviz_point_in_bbox(bbox: DlvizBBox, x: f32, y: f32) -> bool {
    x >= bbox.x && x <= bbox.x + bbox.width && y >= bbox.y && y <= bbox.y + bbox.height
}

/// Find the first element containing a point
///
/// Searches through an array of elements and returns the index of the first
/// element whose bounding box contains the given point. Returns -1 if no
/// element contains the point.
///
/// # Arguments
/// - `elements`: Pointer to array of elements
/// - `count`: Number of elements in the array
/// - `x`: X coordinate of point
/// - `y`: Y coordinate of point
///
/// # Returns
/// Index of first element containing the point, or -1 if none found
///
/// # Safety
/// - `elements` must be valid for `count` elements or null (if count is 0)
#[no_mangle]
pub unsafe extern "C" fn dlviz_find_element_at_point(
    elements: *const DlvizElement,
    count: usize,
    x: f32,
    y: f32,
) -> i32 {
    if count == 0 || elements.is_null() {
        return -1;
    }

    let elements_slice = std::slice::from_raw_parts(elements, count);
    for (i, elem) in elements_slice.iter().enumerate() {
        if dlviz_point_in_bbox(elem.bbox, x, y) {
            return i as i32;
        }
    }
    -1
}

/// Find all elements containing a point
///
/// Searches through an array of elements and counts how many elements'
/// bounding boxes contain the given point. Optionally fills an output
/// array with the indices.
///
/// # Arguments
/// - `elements`: Pointer to array of elements
/// - `count`: Number of elements in the array
/// - `x`: X coordinate of point
/// - `y`: Y coordinate of point
/// - `out_indices`: Optional output array to store matching indices (can be null)
/// - `out_capacity`: Capacity of `out_indices` array (ignored if `out_indices` is null)
///
/// # Returns
/// Number of elements containing the point
///
/// # Safety
/// - `elements` must be valid for `count` elements or null (if count is 0)
/// - If `out_indices` is not null, it must be valid for at least `out_capacity` elements
#[no_mangle]
pub unsafe extern "C" fn dlviz_find_all_elements_at_point(
    elements: *const DlvizElement,
    count: usize,
    x: f32,
    y: f32,
    out_indices: *mut usize,
    out_capacity: usize,
) -> usize {
    if count == 0 || elements.is_null() {
        return 0;
    }

    let elements_slice = std::slice::from_raw_parts(elements, count);
    let mut found = 0usize;

    for (i, elem) in elements_slice.iter().enumerate() {
        if dlviz_point_in_bbox(elem.bbox, x, y) {
            // Store index if output array is provided and has capacity
            if !out_indices.is_null() && found < out_capacity {
                *out_indices.add(found) = i;
            }
            found += 1;
        }
    }
    found
}

/// Count elements overlapping with a region
///
/// # Arguments
/// - `elements`: Pointer to array of elements
/// - `count`: Number of elements in the array
/// - `region`: Region to test against
///
/// # Returns
/// Number of elements whose bounding box overlaps the region
///
/// # Safety
/// - `elements` must be valid for `count` elements or null (if count is 0)
#[no_mangle]
pub unsafe extern "C" fn dlviz_count_elements_in_region(
    elements: *const DlvizElement,
    count: usize,
    region: DlvizBBox,
) -> usize {
    if count == 0 || elements.is_null() {
        return 0;
    }

    let elements_slice = std::slice::from_raw_parts(elements, count);
    elements_slice
        .iter()
        .filter(|e| dlviz_bbox_overlaps(e.bbox, region))
        .count()
}

/// Count elements completely contained within a region
///
/// # Arguments
/// - `elements`: Pointer to array of elements
/// - `count`: Number of elements in the array
/// - `region`: Region to test against
///
/// # Returns
/// Number of elements whose bounding box is completely inside the region
///
/// # Safety
/// - `elements` must be valid for `count` elements or null (if count is 0)
#[no_mangle]
pub unsafe extern "C" fn dlviz_count_elements_contained_in_region(
    elements: *const DlvizElement,
    count: usize,
    region: DlvizBBox,
) -> usize {
    if count == 0 || elements.is_null() {
        return 0;
    }

    let elements_slice = std::slice::from_raw_parts(elements, count);
    elements_slice
        .iter()
        .filter(|e| dlviz_bbox_contains(region, e.bbox))
        .count()
}

/// Find element with highest confidence at a point
///
/// When multiple elements overlap at a point, returns the index of the
/// element with the highest confidence score.
///
/// # Arguments
/// - `elements`: Pointer to array of elements
/// - `count`: Number of elements in the array
/// - `x`: X coordinate of point
/// - `y`: Y coordinate of point
///
/// # Returns
/// Index of highest-confidence element at point, or -1 if none found
///
/// # Safety
/// - `elements` must be valid for `count` elements or null (if count is 0)
#[no_mangle]
pub unsafe extern "C" fn dlviz_find_best_element_at_point(
    elements: *const DlvizElement,
    count: usize,
    x: f32,
    y: f32,
) -> i32 {
    if count == 0 || elements.is_null() {
        return -1;
    }

    let elements_slice = std::slice::from_raw_parts(elements, count);
    let mut best_index: i32 = -1;
    let mut best_confidence: f32 = -1.0;

    for (i, elem) in elements_slice.iter().enumerate() {
        if dlviz_point_in_bbox(elem.bbox, x, y) && elem.confidence > best_confidence {
            best_confidence = elem.confidence;
            best_index = i as i32;
        }
    }
    best_index
}

/// Find element with smallest area at a point
///
/// When multiple elements overlap at a point, returns the index of the
/// element with the smallest bounding box area. Useful for selecting
/// the most specific element when clicking.
///
/// # Arguments
/// - `elements`: Pointer to array of elements
/// - `count`: Number of elements in the array
/// - `x`: X coordinate of point
/// - `y`: Y coordinate of point
///
/// # Returns
/// Index of smallest element at point, or -1 if none found
///
/// # Safety
/// - `elements` must be valid for `count` elements or null (if count is 0)
#[no_mangle]
pub unsafe extern "C" fn dlviz_find_smallest_element_at_point(
    elements: *const DlvizElement,
    count: usize,
    x: f32,
    y: f32,
) -> i32 {
    if count == 0 || elements.is_null() {
        return -1;
    }

    let elements_slice = std::slice::from_raw_parts(elements, count);
    let mut best_index: i32 = -1;
    let mut best_area: f32 = f32::MAX;

    for (i, elem) in elements_slice.iter().enumerate() {
        if dlviz_point_in_bbox(elem.bbox, x, y) {
            let area = dlviz_bbox_area(elem.bbox);
            if area < best_area {
                best_area = area;
                best_index = i as i32;
            }
        }
    }
    best_index
}

/// Get the bounding box encompassing all elements
///
/// Computes the smallest bounding box that contains all elements in the array.
///
/// # Arguments
/// - `elements`: Pointer to array of elements
/// - `count`: Number of elements in the array
/// - `out_bbox`: Output pointer to store the result
///
/// # Returns
/// `DlvizResult::Success` if at least one element exists, `InvalidArgument` otherwise
///
/// # Safety
/// - `elements` must be valid for `count` elements or null (if count is 0)
/// - `out_bbox` must be a valid writable pointer
#[no_mangle]
pub unsafe extern "C" fn dlviz_get_elements_bounds(
    elements: *const DlvizElement,
    count: usize,
    out_bbox: *mut DlvizBBox,
) -> DlvizResult {
    if out_bbox.is_null() {
        return DlvizResult::InvalidArgument;
    }

    if count == 0 || elements.is_null() {
        return DlvizResult::InvalidArgument;
    }

    let elements_slice = std::slice::from_raw_parts(elements, count);

    // Start with first element's bbox
    let mut result = elements_slice[0].bbox;

    // Union with remaining elements
    for elem in elements_slice.iter().skip(1) {
        result = dlviz_bbox_union(result, elem.bbox);
    }

    *out_bbox = result;
    DlvizResult::Success
}

// ============================================================================
// Reading Order Traversal Functions
// ============================================================================

/// Find element with a specific reading order index
///
/// # Arguments
/// - `elements`: Pointer to array of elements
/// - `count`: Number of elements in the array
/// - `reading_order`: Reading order value to search for
///
/// # Returns
/// Array index of element with matching reading order, or -1 if not found
///
/// # Safety
/// - `elements` must be valid for `count` elements or null (if count is 0)
#[no_mangle]
pub unsafe extern "C" fn dlviz_get_element_by_reading_order(
    elements: *const DlvizElement,
    count: usize,
    reading_order: i32,
) -> i32 {
    if count == 0 || elements.is_null() {
        return -1;
    }

    let elements_slice = std::slice::from_raw_parts(elements, count);
    for (i, elem) in elements_slice.iter().enumerate() {
        if elem.reading_order == reading_order {
            return i as i32;
        }
    }
    -1
}

/// Get the minimum and maximum reading order values in an element array
///
/// # Arguments
/// - `elements`: Pointer to array of elements
/// - `count`: Number of elements in the array
/// - `out_min`: Output pointer for minimum reading order (can be null)
/// - `out_max`: Output pointer for maximum reading order (can be null)
///
/// # Returns
/// `DlvizResult::Success` if at least one element with valid reading order exists,
/// `InvalidArgument` otherwise
///
/// # Safety
/// - `elements` must be valid for `count` elements or null (if count is 0)
/// - If `out_min`/`out_max` are not null, they must be valid writable pointers
///
/// # Panics
///
/// This function uses `.unwrap()` on `min()` and `max()` iterators, but they are
/// guarded by an early return that checks `valid_orders.is_empty()` first.
/// The panic is unreachable in normal execution.
#[no_mangle]
pub unsafe extern "C" fn dlviz_get_reading_order_range(
    elements: *const DlvizElement,
    count: usize,
    out_min: *mut i32,
    out_max: *mut i32,
) -> DlvizResult {
    if count == 0 || elements.is_null() {
        return DlvizResult::InvalidArgument;
    }

    let elements_slice = std::slice::from_raw_parts(elements, count);

    // Find elements with valid reading order (>= 0)
    let valid_orders: Vec<i32> = elements_slice
        .iter()
        .filter(|e| e.reading_order >= 0)
        .map(|e| e.reading_order)
        .collect();

    if valid_orders.is_empty() {
        return DlvizResult::InvalidArgument;
    }

    let min_order = *valid_orders.iter().min().unwrap();
    let max_order = *valid_orders.iter().max().unwrap();

    if !out_min.is_null() {
        *out_min = min_order;
    }
    if !out_max.is_null() {
        *out_max = max_order;
    }

    DlvizResult::Success
}

/// Find element with the next reading order after a given value
///
/// Finds the element with the smallest reading order that is greater than
/// the given value. Useful for iterating through elements in reading order.
///
/// # Arguments
/// - `elements`: Pointer to array of elements
/// - `count`: Number of elements in the array
/// - `current_order`: Current reading order value
///
/// # Returns
/// Array index of element with next reading order, or -1 if none found
///
/// # Safety
/// - `elements` must be valid for `count` elements or null (if count is 0)
#[no_mangle]
pub unsafe extern "C" fn dlviz_get_next_reading_order_element(
    elements: *const DlvizElement,
    count: usize,
    current_order: i32,
) -> i32 {
    if count == 0 || elements.is_null() {
        return -1;
    }

    let elements_slice = std::slice::from_raw_parts(elements, count);
    let mut best_index: i32 = -1;
    let mut best_order: i32 = i32::MAX;

    for (i, elem) in elements_slice.iter().enumerate() {
        if elem.reading_order > current_order && elem.reading_order < best_order {
            best_order = elem.reading_order;
            best_index = i as i32;
        }
    }
    best_index
}

/// Find element with the previous reading order before a given value
///
/// Finds the element with the largest reading order that is less than
/// the given value. Useful for reverse iteration through reading order.
///
/// # Arguments
/// - `elements`: Pointer to array of elements
/// - `count`: Number of elements in the array
/// - `current_order`: Current reading order value
///
/// # Returns
/// Array index of element with previous reading order, or -1 if none found
///
/// # Safety
/// - `elements` must be valid for `count` elements or null (if count is 0)
#[no_mangle]
pub unsafe extern "C" fn dlviz_get_prev_reading_order_element(
    elements: *const DlvizElement,
    count: usize,
    current_order: i32,
) -> i32 {
    if count == 0 || elements.is_null() {
        return -1;
    }

    let elements_slice = std::slice::from_raw_parts(elements, count);
    let mut best_index: i32 = -1;
    let mut best_order: i32 = i32::MIN;

    for (i, elem) in elements_slice.iter().enumerate() {
        // Only consider valid reading orders (>= 0)
        if elem.reading_order >= 0
            && elem.reading_order < current_order
            && elem.reading_order > best_order
        {
            best_order = elem.reading_order;
            best_index = i as i32;
        }
    }
    best_index
}

/// Find first element in reading order
///
/// Returns the array index of the element with the lowest reading order value.
///
/// # Arguments
/// - `elements`: Pointer to array of elements
/// - `count`: Number of elements in the array
///
/// # Returns
/// Array index of element with lowest reading order, or -1 if no valid elements
///
/// # Safety
/// - `elements` must be valid for `count` elements or null (if count is 0)
#[no_mangle]
pub unsafe extern "C" fn dlviz_get_first_reading_order_element(
    elements: *const DlvizElement,
    count: usize,
) -> i32 {
    if count == 0 || elements.is_null() {
        return -1;
    }

    let elements_slice = std::slice::from_raw_parts(elements, count);
    let mut best_index: i32 = -1;
    let mut best_order: i32 = i32::MAX;

    for (i, elem) in elements_slice.iter().enumerate() {
        if elem.reading_order >= 0 && elem.reading_order < best_order {
            best_order = elem.reading_order;
            best_index = i as i32;
        }
    }
    best_index
}

/// Find last element in reading order
///
/// Returns the array index of the element with the highest reading order value.
///
/// # Arguments
/// - `elements`: Pointer to array of elements
/// - `count`: Number of elements in the array
///
/// # Returns
/// Array index of element with highest reading order, or -1 if no valid elements
///
/// # Safety
/// - `elements` must be valid for `count` elements or null (if count is 0)
#[no_mangle]
pub unsafe extern "C" fn dlviz_get_last_reading_order_element(
    elements: *const DlvizElement,
    count: usize,
) -> i32 {
    if count == 0 || elements.is_null() {
        return -1;
    }

    let elements_slice = std::slice::from_raw_parts(elements, count);
    let mut best_index: i32 = -1;
    let mut best_order: i32 = -1;

    for (i, elem) in elements_slice.iter().enumerate() {
        if elem.reading_order > best_order {
            best_order = elem.reading_order;
            best_index = i as i32;
        }
    }
    best_index
}

/// Count elements with valid reading order
///
/// # Arguments
/// - `elements`: Pointer to array of elements
/// - `count`: Number of elements in the array
///
/// # Returns
/// Number of elements with `reading_order` >= 0
///
/// # Safety
/// - `elements` must be valid for `count` elements or null (if count is 0)
#[no_mangle]
pub unsafe extern "C" fn dlviz_count_elements_with_reading_order(
    elements: *const DlvizElement,
    count: usize,
) -> usize {
    if count == 0 || elements.is_null() {
        return 0;
    }

    let elements_slice = std::slice::from_raw_parts(elements, count);
    elements_slice
        .iter()
        .filter(|e| e.reading_order >= 0)
        .count()
}

// =============================================================================
// Phase 4: Golden Set Builder - Correction Export for AI Validation
// =============================================================================

/// Save corrected elements to a JSON file
///
/// Saves human-corrected detection results for later validation by the AI judge.
/// The output format matches what `ai_visual_judge.py` expects.
///
/// # Arguments
/// - `elements`: Array of corrected elements
/// - `count`: Number of elements
/// - `element_texts`: Array of text strings (parallel to elements, can be null entries)
/// - `pdf_name`: Name of the source PDF
/// - `page_num`: Page number (0-indexed)
/// - `page_width`: Page width in PDF points
/// - `page_height`: Page height in PDF points
/// - `output_path`: Path to write JSON file
///
/// # Returns
/// Result code
///
/// # Safety
/// - `elements` must be valid for `count` elements
/// - `element_texts` must be valid for `count` elements (can be null pointers)
/// - `pdf_name` and `output_path` must be valid null-terminated UTF-8 strings
#[no_mangle]
pub unsafe extern "C" fn dlviz_save_corrected_elements(
    elements: *const DlvizElement,
    count: usize,
    element_texts: *const *const c_char,
    pdf_name: *const c_char,
    page_num: usize,
    page_width: f32,
    page_height: f32,
    output_path: *const c_char,
) -> DlvizResult {
    if elements.is_null() && count > 0 {
        return DlvizResult::InvalidArgument;
    }
    if pdf_name.is_null() || output_path.is_null() {
        return DlvizResult::InvalidArgument;
    }

    let Ok(pdf_name_str) = CStr::from_ptr(pdf_name).to_str() else {
        return DlvizResult::InvalidArgument;
    };
    let Ok(output_path_str) = CStr::from_ptr(output_path).to_str() else {
        return DlvizResult::InvalidArgument;
    };

    let elements_slice = if count > 0 {
        std::slice::from_raw_parts(elements, count)
    } else {
        &[]
    };

    // Build text array
    let texts: Vec<Option<String>> = if element_texts.is_null() {
        vec![None; count]
    } else {
        let text_ptrs = std::slice::from_raw_parts(element_texts, count);
        text_ptrs
            .iter()
            .map(|&ptr| {
                if ptr.is_null() {
                    None
                } else {
                    CStr::from_ptr(ptr).to_str().ok().map(String::from)
                }
            })
            .collect()
    };

    // Generate sidecar JSON using existing helper
    let sidecar = visualization::generate_sidecar(
        pdf_name_str,
        page_num,
        (page_width, page_height),
        DlvizStage::ReadingOrder,
        0.0, // render time not relevant for corrections
        elements_slice,
        &texts,
    );

    // Save to file
    let path = std::path::Path::new(output_path_str);
    if let Err(e) = visualization::save_sidecar(&sidecar, path) {
        log::error!("Failed to save corrected elements: {e}");
        return DlvizResult::InternalError;
    }

    DlvizResult::Success
}

/// Load elements from a JSON file
///
/// Loads previously saved detection results or corrections for editing.
///
/// # Arguments
/// - `input_path`: Path to JSON file
/// - `out_elements`: Pointer to array to fill with loaded elements
/// - `out_count`: Pointer to store number of elements loaded
/// - `max_elements`: Maximum number of elements to load
///
/// # Returns
/// Result code
///
/// # Safety
/// - `input_path` must be a valid null-terminated UTF-8 string
/// - `out_elements` must be valid for `max_elements` writes
/// - `out_count` must be a valid pointer
#[no_mangle]
pub unsafe extern "C" fn dlviz_load_elements_from_json(
    input_path: *const c_char,
    out_elements: *mut DlvizElement,
    out_count: *mut usize,
    max_elements: usize,
) -> DlvizResult {
    if input_path.is_null() || out_elements.is_null() || out_count.is_null() {
        return DlvizResult::InvalidArgument;
    }

    let Ok(input_path_str) = CStr::from_ptr(input_path).to_str() else {
        return DlvizResult::InvalidArgument;
    };

    // Read and parse JSON
    let content = match std::fs::read_to_string(input_path_str) {
        Ok(c) => c,
        Err(e) => {
            log::error!("Failed to read JSON file: {e}");
            return DlvizResult::FileNotFound;
        }
    };

    let sidecar: visualization::VisualizationSidecar = match serde_json::from_str(&content) {
        Ok(s) => s,
        Err(e) => {
            log::error!("Failed to parse JSON: {e}");
            return DlvizResult::ParseError;
        }
    };

    // Convert to FFI elements
    let count = sidecar.elements.len().min(max_elements);
    *out_count = count;

    for (i, elem) in sidecar.elements.iter().take(count).enumerate() {
        let label = match elem.label.as_str() {
            "caption" => DlvizLabel::Caption,
            "footnote" => DlvizLabel::Footnote,
            "formula" => DlvizLabel::Formula,
            "list_item" | "list" => DlvizLabel::ListItem,
            "page_footer" | "footer" => DlvizLabel::PageFooter,
            "page_header" | "header" => DlvizLabel::PageHeader,
            "picture" => DlvizLabel::Picture,
            "section_header" | "section" => DlvizLabel::SectionHeader,
            "table" => DlvizLabel::Table,
            "title" => DlvizLabel::Title,
            "code" => DlvizLabel::Code,
            "checkbox_selected" | "checkbox" => DlvizLabel::CheckboxSelected,
            "checkbox_unselected" => DlvizLabel::CheckboxUnselected,
            "form" => DlvizLabel::Form,
            "key_value_region" | "kv" => DlvizLabel::KeyValueRegion,
            "document_index" | "index" => DlvizLabel::DocumentIndex,
            _ => DlvizLabel::Text, // Default unknown labels to Text
        };

        *out_elements.add(i) = DlvizElement {
            id: elem.id,
            label,
            confidence: elem.confidence,
            bbox: DlvizBBox {
                x: elem.bbox.x,
                y: elem.bbox.y,
                width: elem.bbox.width,
                height: elem.bbox.height,
            },
            reading_order: elem.reading_order,
        };
    }

    DlvizResult::Success
}

// ============================================================================
// Batch Processing FFI Functions
// ============================================================================

use batch_processor::BatchProcessor;
use std::path::PathBuf;

/// Batch processing status for C FFI
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DlvizBatchStats {
    /// Total documents in batch
    pub total_docs: usize,
    /// Completed documents
    pub completed_docs: usize,
    /// Failed documents
    pub failed_docs: usize,
    /// Is processing running
    pub is_running: bool,
    /// Is processing paused
    pub is_paused: bool,
    /// Current playback speed
    pub speed: f64,
}

/// Create a new batch processor
///
/// # Returns
/// Pointer to new batch processor, or null on error
///
/// # Memory
/// Caller must free with `dlviz_batch_free`
#[no_mangle]
pub extern "C" fn dlviz_batch_new() -> *mut BatchProcessor {
    Box::into_raw(Box::new(BatchProcessor::new()))
}

/// Free a batch processor
///
/// # Safety
/// - `batch` must be a valid pointer from `dlviz_batch_new`
/// - `batch` must not be used after this call
#[no_mangle]
pub unsafe extern "C" fn dlviz_batch_free(batch: *mut BatchProcessor) {
    if !batch.is_null() {
        drop(Box::from_raw(batch));
    }
}

/// Start batch processing
///
/// # Arguments
/// - `batch`: Batch processor handle
/// - `input_dir`: Path to directory containing PDF files
/// - `output_dir`: Path to output directory for results
///
/// # Safety
/// - `batch` must be a valid pointer from `dlviz_batch_new`
/// - `input_dir` and `output_dir` must be valid null-terminated C strings
#[no_mangle]
pub unsafe extern "C" fn dlviz_batch_start(
    batch: *mut BatchProcessor,
    input_dir: *const c_char,
    output_dir: *const c_char,
) -> DlvizResult {
    if batch.is_null() || input_dir.is_null() || output_dir.is_null() {
        return DlvizResult::InvalidArgument;
    }

    let batch = &mut *batch;
    let input = match CStr::from_ptr(input_dir).to_str() {
        Ok(s) => PathBuf::from(s),
        Err(_) => return DlvizResult::InvalidArgument,
    };
    let output = match CStr::from_ptr(output_dir).to_str() {
        Ok(s) => PathBuf::from(s),
        Err(_) => return DlvizResult::InvalidArgument,
    };

    batch.start(input, output);
    DlvizResult::Success
}

/// Poll for next batch progress update (non-blocking)
///
/// Returns a JSON string with progress information, or null if no updates available.
///
/// # Arguments
/// - `batch`: Batch processor handle
///
/// # Returns
/// JSON string with `BatchProgress` data, or null if no updates
///
/// # Memory
/// Caller must free returned string with `dlviz_string_free`
///
/// # Safety
/// - `batch` must be a valid pointer from `dlviz_batch_new`
#[no_mangle]
pub unsafe extern "C" fn dlviz_batch_poll(batch: *const BatchProcessor) -> *mut c_char {
    if batch.is_null() {
        return ptr::null_mut();
    }

    let batch = &*batch;
    match batch.poll_progress() {
        Some(progress) => {
            let Ok(json) = serde_json::to_string(&progress) else {
                return ptr::null_mut();
            };
            CString::new(json)
                .map(CString::into_raw)
                .unwrap_or(ptr::null_mut())
        }
        None => ptr::null_mut(),
    }
}

/// Pause batch processing
///
/// # Safety
/// - `batch` must be a valid pointer from `dlviz_batch_new`
#[no_mangle]
pub unsafe extern "C" fn dlviz_batch_pause(batch: *mut BatchProcessor) -> DlvizResult {
    if batch.is_null() {
        return DlvizResult::InvalidArgument;
    }

    let batch = &*batch;
    batch.pause();
    DlvizResult::Success
}

/// Resume batch processing
///
/// # Safety
/// - `batch` must be a valid pointer from `dlviz_batch_new`
#[no_mangle]
pub unsafe extern "C" fn dlviz_batch_resume(batch: *mut BatchProcessor) -> DlvizResult {
    if batch.is_null() {
        return DlvizResult::InvalidArgument;
    }

    let batch = &*batch;
    batch.resume();
    DlvizResult::Success
}

/// Stop batch processing
///
/// # Safety
/// - `batch` must be a valid pointer from `dlviz_batch_new`
#[no_mangle]
pub unsafe extern "C" fn dlviz_batch_stop(batch: *mut BatchProcessor) -> DlvizResult {
    if batch.is_null() {
        return DlvizResult::InvalidArgument;
    }

    let batch = &mut *batch;
    batch.stop();
    DlvizResult::Success
}

/// Set batch processing playback speed
///
/// Speed multiplier for visualization. Values <1.0 slow down, >1.0 speed up.
/// Clamped to range [0.1, 10.0].
///
/// # Arguments
/// - `batch`: Batch processor handle
/// - `speed`: Playback speed multiplier (0.1 to 10.0)
///
/// # Safety
/// - `batch` must be a valid pointer from `dlviz_batch_new`
#[no_mangle]
pub unsafe extern "C" fn dlviz_batch_set_speed(
    batch: *mut BatchProcessor,
    speed: f64,
) -> DlvizResult {
    if batch.is_null() {
        return DlvizResult::InvalidArgument;
    }

    let batch = &*batch;
    batch.set_speed(speed);
    DlvizResult::Success
}

/// Get batch processing statistics
///
/// # Arguments
/// - `batch`: Batch processor handle
///
/// # Returns
/// Current batch processing statistics
///
/// # Safety
/// - `batch` must be a valid pointer from `dlviz_batch_new`, or null
#[no_mangle]
pub unsafe extern "C" fn dlviz_batch_get_stats(batch: *const BatchProcessor) -> DlvizBatchStats {
    if batch.is_null() {
        return DlvizBatchStats {
            total_docs: 0,
            completed_docs: 0,
            failed_docs: 0,
            is_running: false,
            is_paused: false,
            speed: 1.0,
        };
    }

    let batch = &*batch;
    DlvizBatchStats {
        total_docs: batch.total_docs(),
        completed_docs: batch.completed_count(),
        failed_docs: batch.failed_count(),
        is_running: batch.is_running(),
        is_paused: batch.is_paused(),
        speed: batch.get_speed(),
    }
}

/// Check if batch processing is currently running
///
/// # Safety
/// - `batch` must be a valid pointer from `dlviz_batch_new`, or null
#[no_mangle]
pub unsafe extern "C" fn dlviz_batch_is_running(batch: *const BatchProcessor) -> bool {
    if batch.is_null() {
        return false;
    }
    let batch = &*batch;
    batch.is_running()
}

/// Check if batch processing is paused
///
/// # Safety
/// - `batch` must be a valid pointer from `dlviz_batch_new`, or null
#[no_mangle]
pub unsafe extern "C" fn dlviz_batch_is_paused(batch: *const BatchProcessor) -> bool {
    if batch.is_null() {
        return false;
    }
    let batch = &*batch;
    batch.is_paused()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_lifecycle() {
        unsafe {
            let pipeline = dlviz_pipeline_new();
            assert!(!pipeline.is_null());
            dlviz_pipeline_free(pipeline);
        }
    }

    #[test]
    fn test_version() {
        let version = dlviz_version();
        assert!(!version.is_null());
        unsafe {
            let s = CStr::from_ptr(version);
            assert_eq!(s.to_str().unwrap(), "0.1.0");
        }
    }

    #[test]
    fn test_stage_count() {
        assert_eq!(dlviz_stage_count(), 11);
    }

    #[test]
    fn test_stage_names() {
        assert!(!dlviz_stage_name(DlvizStage::RawPdf).is_null());
        assert!(!dlviz_stage_name(DlvizStage::ReadingOrder).is_null());
    }

    #[test]
    fn test_feature_queries() {
        // These should compile and run regardless of features enabled
        let _has_render = dlviz_has_pdf_render();
        let _has_ml = dlviz_has_pdf_ml();
        // In default build, both should be false
        #[cfg(not(feature = "pdf-render"))]
        assert!(!dlviz_has_pdf_render());
        #[cfg(not(feature = "pdf-ml"))]
        assert!(!dlviz_has_pdf_ml());
    }

    #[test]
    fn test_load_nonexistent_pdf() {
        unsafe {
            let pipeline = dlviz_pipeline_new();
            assert!(!pipeline.is_null());

            // Try to load a non-existent file
            let path = std::ffi::CString::new("/nonexistent/path/file.pdf").unwrap();
            let result = dlviz_load_pdf(pipeline, path.as_ptr());

            // Should fail (either FileNotFound or InternalError if no pdf-render)
            assert_ne!(result, DlvizResult::Success);

            // Page count should be 0 for unloaded pipeline
            assert_eq!(dlviz_get_page_count(pipeline), 0);

            dlviz_pipeline_free(pipeline);
        }
    }

    #[test]
    fn test_page_size_without_pdf() {
        unsafe {
            let pipeline = dlviz_pipeline_new();
            assert!(!pipeline.is_null());

            let mut width: f32 = 0.0;
            let mut height: f32 = 0.0;

            // Should return error for pipeline without loaded PDF
            let result = dlviz_get_page_size(pipeline, 0, &mut width, &mut height);
            assert_eq!(result, DlvizResult::InvalidArgument);

            dlviz_pipeline_free(pipeline);
        }
    }

    #[test]
    fn test_render_without_pdf() {
        unsafe {
            let pipeline = dlviz_pipeline_new();
            assert!(!pipeline.is_null());

            let mut width: u32 = 0;
            let mut height: u32 = 0;
            let mut buffer: [u8; 16] = [0; 16];

            // Should return error for pipeline without loaded PDF
            let result = dlviz_render_page(
                pipeline,
                0,
                1.0,
                &mut width,
                &mut height,
                buffer.as_mut_ptr(),
                buffer.len(),
            );
            assert_eq!(result, DlvizResult::ParseError);

            dlviz_pipeline_free(pipeline);
        }
    }

    #[test]
    #[cfg(feature = "pdf-render")]
    fn test_load_real_pdf() {
        unsafe {
            let pipeline = dlviz_pipeline_new();
            assert!(!pipeline.is_null());

            // Try to load a test PDF - use CARGO_MANIFEST_DIR to find workspace root
            let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            let workspace_root = manifest_dir.parent().unwrap().parent().unwrap();
            let test_pdf = workspace_root.join("test-corpus/pdf/code_and_formula.pdf");

            if test_pdf.exists() {
                let path = std::ffi::CString::new(test_pdf.to_str().unwrap()).unwrap();
                let result = dlviz_load_pdf(pipeline, path.as_ptr());

                if result == DlvizResult::Success {
                    // Should have pages
                    let page_count = dlviz_get_page_count(pipeline);
                    assert!(page_count > 0, "Expected at least 1 page");

                    // Should be able to get page size
                    let mut width: f32 = 0.0;
                    let mut height: f32 = 0.0;
                    let size_result = dlviz_get_page_size(pipeline, 0, &mut width, &mut height);
                    assert_eq!(size_result, DlvizResult::Success);
                    assert!(width > 0.0, "Expected positive width");
                    assert!(height > 0.0, "Expected positive height");

                    println!(
                        "Loaded PDF: {} pages, first page: {}x{} points",
                        page_count, width, height
                    );
                } else {
                    // Pdfium library not found - this is OK for CI
                    println!(
                        "Could not load PDF (pdfium library may not be installed): {:?}",
                        result
                    );
                }
            } else {
                println!("Test PDF not found, skipping real PDF test");
            }

            dlviz_pipeline_free(pipeline);
        }
    }

    #[test]
    fn test_label_count() {
        assert_eq!(dlviz_label_count(), 17);
    }

    #[test]
    fn test_label_names() {
        unsafe {
            // Test first and last labels
            let caption_name = dlviz_label_name(DlvizLabel::Caption);
            assert!(!caption_name.is_null());
            assert_eq!(CStr::from_ptr(caption_name).to_str().unwrap(), "Caption");

            let key_value_name = dlviz_label_name(DlvizLabel::KeyValueRegion);
            assert!(!key_value_name.is_null());
            assert_eq!(
                CStr::from_ptr(key_value_name).to_str().unwrap(),
                "Key-Value Region"
            );

            // Test a middle label
            let table_name = dlviz_label_name(DlvizLabel::Table);
            assert!(!table_name.is_null());
            assert_eq!(CStr::from_ptr(table_name).to_str().unwrap(), "Table");
        }
    }

    #[test]
    fn test_label_short_names() {
        unsafe {
            // Test first and last labels
            let caption_name = dlviz_label_short_name(DlvizLabel::Caption);
            assert!(!caption_name.is_null());
            assert_eq!(CStr::from_ptr(caption_name).to_str().unwrap(), "caption");

            let key_value_name = dlviz_label_short_name(DlvizLabel::KeyValueRegion);
            assert!(!key_value_name.is_null());
            assert_eq!(
                CStr::from_ptr(key_value_name).to_str().unwrap(),
                "key_value_region"
            );

            // Test checkbox selected (has underscore in name)
            let checkbox_name = dlviz_label_short_name(DlvizLabel::CheckboxSelected);
            assert!(!checkbox_name.is_null());
            assert_eq!(
                CStr::from_ptr(checkbox_name).to_str().unwrap(),
                "checkbox_selected"
            );
        }
    }

    #[test]
    fn test_label_colors() {
        // Test a few representative colors
        let title_color = dlviz_label_color(DlvizLabel::Title);
        assert_eq!(title_color.r, 255);
        assert_eq!(title_color.g, 0);
        assert_eq!(title_color.b, 0);
        assert_eq!(title_color.a, 255);

        let table_color = dlviz_label_color(DlvizLabel::Table);
        assert_eq!(table_color.r, 0);
        assert_eq!(table_color.g, 255);
        assert_eq!(table_color.b, 0);
        assert_eq!(table_color.a, 255);

        let caption_color = dlviz_label_color(DlvizLabel::Caption);
        assert_eq!(caption_color.r, 255);
        assert_eq!(caption_color.g, 165);
        assert_eq!(caption_color.b, 0);
        assert_eq!(caption_color.a, 255);
    }

    #[test]
    fn test_label_from_index() {
        // Valid indices
        assert_eq!(dlviz_label_from_index(0), 0); // Caption
        assert_eq!(dlviz_label_from_index(8), 8); // Table
        assert_eq!(dlviz_label_from_index(16), 16); // KeyValueRegion

        // Invalid indices
        assert_eq!(dlviz_label_from_index(17), -1);
        assert_eq!(dlviz_label_from_index(100), -1);
    }

    #[test]
    fn test_all_labels_have_names_and_colors() {
        // Ensure all 17 labels have valid names and colors
        for i in 0i32..17 {
            let label = unsafe { std::mem::transmute::<i32, DlvizLabel>(i) };

            // Check name is not null and not "Unknown"
            let name = dlviz_label_name(label);
            assert!(!name.is_null());
            unsafe {
                let name_str = CStr::from_ptr(name).to_str().unwrap();
                assert_ne!(name_str, "Unknown", "Label {i} should have a name");
            }

            // Check short name is not null and not "unknown"
            let short_name = dlviz_label_short_name(label);
            assert!(!short_name.is_null());
            unsafe {
                let short_name_str = CStr::from_ptr(short_name).to_str().unwrap();
                assert_ne!(
                    short_name_str, "unknown",
                    "Label {i} should have a short name"
                );
            }

            // Check color is opaque (alpha = 255)
            let color = dlviz_label_color(label);
            assert_eq!(color.a, 255, "Label {i} should have opaque color");
        }
    }

    // Annotation format conversion tests

    #[test]
    fn test_yolo_conversion_roundtrip() {
        // Test that PDF -> YOLO -> PDF gives back the original bbox
        let pdf_bbox = DlvizBBox {
            x: 100.0,
            y: 200.0,
            width: 50.0,
            height: 30.0,
        };
        let page_width = US_LETTER_WIDTH_F32;
        let page_height = US_LETTER_HEIGHT_F32;

        let yolo = dlviz_bbox_to_yolo(pdf_bbox, page_width, page_height);

        // YOLO coordinates should be normalized (0-1)
        assert!(yolo.x_center >= 0.0 && yolo.x_center <= 1.0);
        assert!(yolo.y_center >= 0.0 && yolo.y_center <= 1.0);
        assert!(yolo.width >= 0.0 && yolo.width <= 1.0);
        assert!(yolo.height >= 0.0 && yolo.height <= 1.0);

        // Convert back
        let recovered = dlviz_yolo_to_bbox(yolo, page_width, page_height);

        // Should match original (with small floating point tolerance)
        assert!((recovered.x - pdf_bbox.x).abs() < 0.01);
        assert!((recovered.y - pdf_bbox.y).abs() < 0.01);
        assert!((recovered.width - pdf_bbox.width).abs() < 0.01);
        assert!((recovered.height - pdf_bbox.height).abs() < 0.01);
    }

    #[test]
    fn test_coco_conversion_roundtrip() {
        let pdf_bbox = DlvizBBox {
            x: 50.0,
            y: 100.0,
            width: 200.0,
            height: 150.0,
        };
        let page_height = US_LETTER_HEIGHT_F32;

        let coco = dlviz_bbox_to_coco(pdf_bbox, page_height);

        // Area should be calculated correctly
        assert!((coco.area - 200.0 * 150.0).abs() < 0.01);

        // Convert back
        let recovered = dlviz_coco_to_bbox(coco, page_height);

        assert!((recovered.x - pdf_bbox.x).abs() < 0.01);
        assert!((recovered.y - pdf_bbox.y).abs() < 0.01);
        assert!((recovered.width - pdf_bbox.width).abs() < 0.01);
        assert!((recovered.height - pdf_bbox.height).abs() < 0.01);
    }

    #[test]
    fn test_voc_conversion_roundtrip() {
        let pdf_bbox = DlvizBBox {
            x: 75.0,
            y: 150.0,
            width: 100.0,
            height: 80.0,
        };
        let page_height = US_LETTER_HEIGHT_F32;

        let voc = dlviz_bbox_to_voc(pdf_bbox, page_height);

        // VOC should have xmax > xmin and ymax > ymin
        assert!(voc.xmax > voc.xmin);
        assert!(voc.ymax > voc.ymin);
        assert!((voc.xmax - voc.xmin - pdf_bbox.width).abs() < 0.01);
        assert!((voc.ymax - voc.ymin - pdf_bbox.height).abs() < 0.01);

        // Convert back
        let recovered = dlviz_voc_to_bbox(voc, page_height);

        assert!((recovered.x - pdf_bbox.x).abs() < 0.01);
        assert!((recovered.y - pdf_bbox.y).abs() < 0.01);
        assert!((recovered.width - pdf_bbox.width).abs() < 0.01);
        assert!((recovered.height - pdf_bbox.height).abs() < 0.01);
    }

    #[test]
    fn test_bbox_iou_identical() {
        let bbox = DlvizBBox {
            x: 100.0,
            y: 100.0,
            width: 50.0,
            height: 50.0,
        };

        // Identical boxes should have IoU = 1.0
        let iou = dlviz_bbox_iou(bbox, bbox);
        assert!((iou - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_bbox_iou_no_overlap() {
        let bbox1 = DlvizBBox {
            x: 0.0,
            y: 0.0,
            width: 50.0,
            height: 50.0,
        };
        let bbox2 = DlvizBBox {
            x: 100.0,
            y: 100.0,
            width: 50.0,
            height: 50.0,
        };

        // Non-overlapping boxes should have IoU = 0.0
        let iou = dlviz_bbox_iou(bbox1, bbox2);
        assert!(iou.abs() < 0.001);
    }

    #[test]
    fn test_bbox_iou_partial() {
        let bbox1 = DlvizBBox {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        };
        let bbox2 = DlvizBBox {
            x: 50.0,
            y: 50.0,
            width: 100.0,
            height: 100.0,
        };

        // Partially overlapping boxes
        // Intersection: 50x50 = 2500
        // Union: 10000 + 10000 - 2500 = 17500
        // IoU = 2500/17500 ≈ 0.143
        let iou = dlviz_bbox_iou(bbox1, bbox2);
        assert!(iou > 0.1 && iou < 0.2);
    }

    #[test]
    fn test_bbox_overlaps() {
        let bbox1 = DlvizBBox {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        };
        let bbox2 = DlvizBBox {
            x: 50.0,
            y: 50.0,
            width: 100.0,
            height: 100.0,
        };
        let bbox3 = DlvizBBox {
            x: 200.0,
            y: 200.0,
            width: 50.0,
            height: 50.0,
        };

        assert!(dlviz_bbox_overlaps(bbox1, bbox2));
        assert!(!dlviz_bbox_overlaps(bbox1, bbox3));
        assert!(!dlviz_bbox_overlaps(bbox2, bbox3));
    }

    #[test]
    fn test_bbox_contains() {
        let outer = DlvizBBox {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        };
        let inner = DlvizBBox {
            x: 25.0,
            y: 25.0,
            width: 50.0,
            height: 50.0,
        };
        let outside = DlvizBBox {
            x: 150.0,
            y: 150.0,
            width: 50.0,
            height: 50.0,
        };
        let partial = DlvizBBox {
            x: 75.0,
            y: 75.0,
            width: 50.0,
            height: 50.0,
        };

        assert!(dlviz_bbox_contains(outer, inner));
        assert!(!dlviz_bbox_contains(outer, outside));
        assert!(!dlviz_bbox_contains(outer, partial)); // Partially outside
        assert!(dlviz_bbox_contains(outer, outer)); // Box contains itself
    }

    #[test]
    fn test_yolo_format_at_corners() {
        // Test box at top-left corner of page
        let top_left = DlvizBBox {
            x: 0.0,
            y: 692.0, // PDF y-coordinate (bottom-left origin)
            width: 100.0,
            height: 100.0,
        };
        let page_width = US_LETTER_WIDTH_F32;
        let page_height = US_LETTER_HEIGHT_F32;

        let yolo = dlviz_bbox_to_yolo(top_left, page_width, page_height);

        // Should be near (0.08, 0.06) center in YOLO (top-left origin)
        assert!(yolo.x_center < 0.2);
        assert!(yolo.y_center < 0.2);
    }

    // Element statistics tests

    #[test]
    fn test_compute_element_stats_empty() {
        unsafe {
            let mut stats = DlvizElementStats::default();
            let result = dlviz_compute_element_stats(ptr::null(), 0, &mut stats);
            assert_eq!(result, DlvizResult::Success);
            assert_eq!(stats.total_count, 0);
            assert_eq!(stats.min_confidence, 0.0);
            assert_eq!(stats.max_confidence, 0.0);
        }
    }

    #[test]
    fn test_compute_element_stats_single() {
        unsafe {
            let elements = [DlvizElement {
                id: 1,
                bbox: DlvizBBox {
                    x: 0.0,
                    y: 0.0,
                    width: 100.0,
                    height: 50.0,
                },
                label: DlvizLabel::Title,
                confidence: 0.95,
                reading_order: 0,
            }];

            let mut stats = DlvizElementStats::default();
            let result = dlviz_compute_element_stats(elements.as_ptr(), 1, &mut stats);

            assert_eq!(result, DlvizResult::Success);
            assert_eq!(stats.total_count, 1);
            assert_eq!(stats.title_count, 1);
            assert_eq!(stats.text_count, 0);
            assert!((stats.avg_confidence - 0.95).abs() < 0.001);
            assert!((stats.min_confidence - 0.95).abs() < 0.001);
            assert!((stats.max_confidence - 0.95).abs() < 0.001);
        }
    }

    #[test]
    fn test_compute_element_stats_multiple() {
        unsafe {
            let elements = [
                DlvizElement {
                    id: 1,
                    bbox: DlvizBBox {
                        x: 0.0,
                        y: 0.0,
                        width: 100.0,
                        height: 50.0,
                    },
                    label: DlvizLabel::Title,
                    confidence: 0.9,
                    reading_order: 0,
                },
                DlvizElement {
                    id: 2,
                    bbox: DlvizBBox {
                        x: 0.0,
                        y: 100.0,
                        width: 200.0,
                        height: 30.0,
                    },
                    label: DlvizLabel::Text,
                    confidence: 0.8,
                    reading_order: 1,
                },
                DlvizElement {
                    id: 3,
                    bbox: DlvizBBox {
                        x: 0.0,
                        y: 200.0,
                        width: 200.0,
                        height: 30.0,
                    },
                    label: DlvizLabel::Text,
                    confidence: 0.7,
                    reading_order: 2,
                },
                DlvizElement {
                    id: 4,
                    bbox: DlvizBBox {
                        x: 0.0,
                        y: 300.0,
                        width: 150.0,
                        height: 100.0,
                    },
                    label: DlvizLabel::Table,
                    confidence: 0.85,
                    reading_order: 3,
                },
            ];

            let mut stats = DlvizElementStats::default();
            let result = dlviz_compute_element_stats(elements.as_ptr(), 4, &mut stats);

            assert_eq!(result, DlvizResult::Success);
            assert_eq!(stats.total_count, 4);
            assert_eq!(stats.title_count, 1);
            assert_eq!(stats.text_count, 2);
            assert_eq!(stats.table_count, 1);
            assert_eq!(stats.picture_count, 0);
            assert!((stats.min_confidence - 0.7).abs() < 0.001);
            assert!((stats.max_confidence - 0.9).abs() < 0.001);
            // avg = (0.9 + 0.8 + 0.7 + 0.85) / 4 = 0.8125
            assert!((stats.avg_confidence - 0.8125).abs() < 0.001);
        }
    }

    #[test]
    fn test_stats_get_label_count() {
        unsafe {
            let stats = DlvizElementStats {
                total_count: 10,
                caption_count: 1,
                text_count: 5,
                table_count: 2,
                title_count: 1,
                picture_count: 1,
                ..Default::default()
            };

            assert_eq!(dlviz_stats_get_label_count(&stats, DlvizLabel::Caption), 1);
            assert_eq!(dlviz_stats_get_label_count(&stats, DlvizLabel::Text), 5);
            assert_eq!(dlviz_stats_get_label_count(&stats, DlvizLabel::Table), 2);
            assert_eq!(dlviz_stats_get_label_count(&stats, DlvizLabel::Formula), 0);
        }
    }

    // Bbox transformation tests

    #[test]
    fn test_bbox_scale() {
        let bbox = DlvizBBox {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 50.0,
        };

        let scaled = dlviz_bbox_scale(bbox, 2.0);
        assert!((scaled.x - 20.0).abs() < 0.001);
        assert!((scaled.y - 40.0).abs() < 0.001);
        assert!((scaled.width - 200.0).abs() < 0.001);
        assert!((scaled.height - 100.0).abs() < 0.001);

        let half = dlviz_bbox_scale(bbox, 0.5);
        assert!((half.x - 5.0).abs() < 0.001);
        assert!((half.y - 10.0).abs() < 0.001);
        assert!((half.width - 50.0).abs() < 0.001);
        assert!((half.height - 25.0).abs() < 0.001);
    }

    #[test]
    fn test_bbox_translate() {
        let bbox = DlvizBBox {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 50.0,
        };

        let translated = dlviz_bbox_translate(bbox, 5.0, -10.0);
        assert!((translated.x - 15.0).abs() < 0.001);
        assert!((translated.y - 10.0).abs() < 0.001);
        assert!((translated.width - 100.0).abs() < 0.001);
        assert!((translated.height - 50.0).abs() < 0.001);
    }

    #[test]
    fn test_bbox_union() {
        let bbox1 = DlvizBBox {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        };
        let bbox2 = DlvizBBox {
            x: 50.0,
            y: 50.0,
            width: 100.0,
            height: 100.0,
        };

        let union = dlviz_bbox_union(bbox1, bbox2);
        assert!((union.x - 0.0).abs() < 0.001);
        assert!((union.y - 0.0).abs() < 0.001);
        assert!((union.width - 150.0).abs() < 0.001);
        assert!((union.height - 150.0).abs() < 0.001);
    }

    #[test]
    fn test_bbox_intersection_overlap() {
        let bbox1 = DlvizBBox {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        };
        let bbox2 = DlvizBBox {
            x: 50.0,
            y: 50.0,
            width: 100.0,
            height: 100.0,
        };

        let inter = dlviz_bbox_intersection(bbox1, bbox2);
        assert!((inter.x - 50.0).abs() < 0.001);
        assert!((inter.y - 50.0).abs() < 0.001);
        assert!((inter.width - 50.0).abs() < 0.001);
        assert!((inter.height - 50.0).abs() < 0.001);
    }

    #[test]
    fn test_bbox_intersection_no_overlap() {
        let bbox1 = DlvizBBox {
            x: 0.0,
            y: 0.0,
            width: 50.0,
            height: 50.0,
        };
        let bbox2 = DlvizBBox {
            x: 100.0,
            y: 100.0,
            width: 50.0,
            height: 50.0,
        };

        let inter = dlviz_bbox_intersection(bbox1, bbox2);
        assert!((inter.width).abs() < 0.001);
        assert!((inter.height).abs() < 0.001);
    }

    #[test]
    fn test_bbox_area() {
        let bbox = DlvizBBox {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 50.0,
        };

        let area = dlviz_bbox_area(bbox);
        assert!((area - 5000.0).abs() < 0.001);
    }

    #[test]
    fn test_bbox_is_valid() {
        let valid = DlvizBBox {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 50.0,
        };
        assert!(dlviz_bbox_is_valid(valid));

        let invalid_width = DlvizBBox {
            x: 10.0,
            y: 20.0,
            width: -10.0,
            height: 50.0,
        };
        assert!(!dlviz_bbox_is_valid(invalid_width));

        let invalid_height = DlvizBBox {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: -5.0,
        };
        assert!(!dlviz_bbox_is_valid(invalid_height));

        let zero_size = DlvizBBox {
            x: 10.0,
            y: 20.0,
            width: 0.0,
            height: 0.0,
        };
        assert!(dlviz_bbox_is_valid(zero_size)); // Zero is valid
    }

    #[test]
    fn test_bbox_expand() {
        let bbox = DlvizBBox {
            x: 50.0,
            y: 50.0,
            width: 100.0,
            height: 100.0,
        };

        let expanded = dlviz_bbox_expand(bbox, 10.0);
        assert!((expanded.x - 40.0).abs() < 0.001);
        assert!((expanded.y - 40.0).abs() < 0.001);
        assert!((expanded.width - 120.0).abs() < 0.001);
        assert!((expanded.height - 120.0).abs() < 0.001);

        let shrunk = dlviz_bbox_expand(bbox, -10.0);
        assert!((shrunk.x - 60.0).abs() < 0.001);
        assert!((shrunk.y - 60.0).abs() < 0.001);
        assert!((shrunk.width - 80.0).abs() < 0.001);
        assert!((shrunk.height - 80.0).abs() < 0.001);
    }

    #[test]
    fn test_bbox_center() {
        let bbox = DlvizBBox {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 50.0,
        };

        unsafe {
            let mut cx: f32 = 0.0;
            let mut cy: f32 = 0.0;
            dlviz_bbox_center(bbox, &mut cx, &mut cy);
            assert!((cx - 50.0).abs() < 0.001);
            assert!((cy - 25.0).abs() < 0.001);
        }
    }

    // Spatial query tests

    #[test]
    fn test_point_in_bbox() {
        let bbox = DlvizBBox {
            x: 100.0,
            y: 100.0,
            width: 50.0,
            height: 30.0,
        };

        // Inside
        assert!(dlviz_point_in_bbox(bbox, 125.0, 115.0));
        // On edge
        assert!(dlviz_point_in_bbox(bbox, 100.0, 100.0));
        assert!(dlviz_point_in_bbox(bbox, 150.0, 130.0));
        // Outside
        assert!(!dlviz_point_in_bbox(bbox, 99.0, 115.0));
        assert!(!dlviz_point_in_bbox(bbox, 151.0, 115.0));
        assert!(!dlviz_point_in_bbox(bbox, 125.0, 99.0));
        assert!(!dlviz_point_in_bbox(bbox, 125.0, 131.0));
    }

    #[test]
    fn test_find_element_at_point() {
        unsafe {
            let elements = [
                DlvizElement {
                    id: 1,
                    bbox: DlvizBBox {
                        x: 0.0,
                        y: 0.0,
                        width: 100.0,
                        height: 100.0,
                    },
                    label: DlvizLabel::Title,
                    confidence: 0.9,
                    reading_order: 0,
                },
                DlvizElement {
                    id: 2,
                    bbox: DlvizBBox {
                        x: 50.0,
                        y: 50.0,
                        width: 100.0,
                        height: 100.0,
                    },
                    label: DlvizLabel::Text,
                    confidence: 0.8,
                    reading_order: 1,
                },
            ];

            // Point in first element only
            assert_eq!(
                dlviz_find_element_at_point(elements.as_ptr(), 2, 25.0, 25.0),
                0
            );
            // Point in second element only
            assert_eq!(
                dlviz_find_element_at_point(elements.as_ptr(), 2, 125.0, 125.0),
                1
            );
            // Point in both - returns first
            assert_eq!(
                dlviz_find_element_at_point(elements.as_ptr(), 2, 75.0, 75.0),
                0
            );
            // Point in neither
            assert_eq!(
                dlviz_find_element_at_point(elements.as_ptr(), 2, 200.0, 200.0),
                -1
            );
            // Empty array
            assert_eq!(dlviz_find_element_at_point(ptr::null(), 0, 50.0, 50.0), -1);
        }
    }

    #[test]
    fn test_find_all_elements_at_point() {
        unsafe {
            let elements = [
                DlvizElement {
                    id: 1,
                    bbox: DlvizBBox {
                        x: 0.0,
                        y: 0.0,
                        width: 100.0,
                        height: 100.0,
                    },
                    label: DlvizLabel::Title,
                    confidence: 0.9,
                    reading_order: 0,
                },
                DlvizElement {
                    id: 2,
                    bbox: DlvizBBox {
                        x: 50.0,
                        y: 50.0,
                        width: 100.0,
                        height: 100.0,
                    },
                    label: DlvizLabel::Text,
                    confidence: 0.8,
                    reading_order: 1,
                },
            ];

            // Point in both elements
            let mut indices: [usize; 4] = [0; 4];
            let count = dlviz_find_all_elements_at_point(
                elements.as_ptr(),
                2,
                75.0,
                75.0,
                indices.as_mut_ptr(),
                4,
            );
            assert_eq!(count, 2);
            assert_eq!(indices[0], 0);
            assert_eq!(indices[1], 1);

            // Point in one element
            let count = dlviz_find_all_elements_at_point(
                elements.as_ptr(),
                2,
                25.0,
                25.0,
                indices.as_mut_ptr(),
                4,
            );
            assert_eq!(count, 1);
            assert_eq!(indices[0], 0);

            // Point in none
            let count = dlviz_find_all_elements_at_point(
                elements.as_ptr(),
                2,
                200.0,
                200.0,
                indices.as_mut_ptr(),
                4,
            );
            assert_eq!(count, 0);

            // Null output array (just count)
            let count = dlviz_find_all_elements_at_point(
                elements.as_ptr(),
                2,
                75.0,
                75.0,
                ptr::null_mut(),
                0,
            );
            assert_eq!(count, 2);
        }
    }

    #[test]
    fn test_count_elements_in_region() {
        unsafe {
            let elements = [
                DlvizElement {
                    id: 1,
                    bbox: DlvizBBox {
                        x: 0.0,
                        y: 0.0,
                        width: 50.0,
                        height: 50.0,
                    },
                    label: DlvizLabel::Title,
                    confidence: 0.9,
                    reading_order: 0,
                },
                DlvizElement {
                    id: 2,
                    bbox: DlvizBBox {
                        x: 100.0,
                        y: 100.0,
                        width: 50.0,
                        height: 50.0,
                    },
                    label: DlvizLabel::Text,
                    confidence: 0.8,
                    reading_order: 1,
                },
            ];

            // Region overlapping first element
            let region = DlvizBBox {
                x: 25.0,
                y: 25.0,
                width: 50.0,
                height: 50.0,
            };
            assert_eq!(
                dlviz_count_elements_in_region(elements.as_ptr(), 2, region),
                1
            );

            // Region overlapping both
            let large_region = DlvizBBox {
                x: 0.0,
                y: 0.0,
                width: 200.0,
                height: 200.0,
            };
            assert_eq!(
                dlviz_count_elements_in_region(elements.as_ptr(), 2, large_region),
                2
            );

            // Region overlapping none
            let empty_region = DlvizBBox {
                x: 200.0,
                y: 200.0,
                width: 50.0,
                height: 50.0,
            };
            assert_eq!(
                dlviz_count_elements_in_region(elements.as_ptr(), 2, empty_region),
                0
            );
        }
    }

    #[test]
    fn test_find_best_element_at_point() {
        unsafe {
            let elements = [
                DlvizElement {
                    id: 1,
                    bbox: DlvizBBox {
                        x: 0.0,
                        y: 0.0,
                        width: 100.0,
                        height: 100.0,
                    },
                    label: DlvizLabel::Title,
                    confidence: 0.7,
                    reading_order: 0,
                },
                DlvizElement {
                    id: 2,
                    bbox: DlvizBBox {
                        x: 50.0,
                        y: 50.0,
                        width: 100.0,
                        height: 100.0,
                    },
                    label: DlvizLabel::Text,
                    confidence: 0.95,
                    reading_order: 1,
                },
            ];

            // Point in both - should return element with highest confidence
            let best = dlviz_find_best_element_at_point(elements.as_ptr(), 2, 75.0, 75.0);
            assert_eq!(best, 1); // Index 1 has confidence 0.95

            // Point in only first
            let best = dlviz_find_best_element_at_point(elements.as_ptr(), 2, 25.0, 25.0);
            assert_eq!(best, 0);
        }
    }

    #[test]
    fn test_find_smallest_element_at_point() {
        unsafe {
            let elements = [
                DlvizElement {
                    id: 1,
                    bbox: DlvizBBox {
                        x: 0.0,
                        y: 0.0,
                        width: 200.0, // Large element
                        height: 200.0,
                    },
                    label: DlvizLabel::Title,
                    confidence: 0.9,
                    reading_order: 0,
                },
                DlvizElement {
                    id: 2,
                    bbox: DlvizBBox {
                        x: 50.0,
                        y: 50.0,
                        width: 50.0, // Small element
                        height: 50.0,
                    },
                    label: DlvizLabel::Text,
                    confidence: 0.8,
                    reading_order: 1,
                },
            ];

            // Point in both - should return smaller element
            let smallest = dlviz_find_smallest_element_at_point(elements.as_ptr(), 2, 75.0, 75.0);
            assert_eq!(smallest, 1); // Index 1 has smaller area

            // Point in only first
            let smallest = dlviz_find_smallest_element_at_point(elements.as_ptr(), 2, 25.0, 25.0);
            assert_eq!(smallest, 0);
        }
    }

    #[test]
    fn test_get_elements_bounds() {
        unsafe {
            let elements = [
                DlvizElement {
                    id: 1,
                    bbox: DlvizBBox {
                        x: 10.0,
                        y: 20.0,
                        width: 30.0,
                        height: 40.0,
                    },
                    label: DlvizLabel::Title,
                    confidence: 0.9,
                    reading_order: 0,
                },
                DlvizElement {
                    id: 2,
                    bbox: DlvizBBox {
                        x: 50.0,
                        y: 60.0,
                        width: 70.0,
                        height: 80.0,
                    },
                    label: DlvizLabel::Text,
                    confidence: 0.8,
                    reading_order: 1,
                },
            ];

            let mut bounds = DlvizBBox::default();
            let result = dlviz_get_elements_bounds(elements.as_ptr(), 2, &mut bounds);
            assert_eq!(result, DlvizResult::Success);

            // Should encompass both elements
            // Min x = 10, max x = 50 + 70 = 120, so width = 110
            // Min y = 20, max y = 60 + 80 = 140, so height = 120
            assert!((bounds.x - 10.0).abs() < 0.001);
            assert!((bounds.y - 20.0).abs() < 0.001);
            assert!((bounds.width - 110.0).abs() < 0.001);
            assert!((bounds.height - 120.0).abs() < 0.001);

            // Empty array should fail
            let result = dlviz_get_elements_bounds(ptr::null(), 0, &mut bounds);
            assert_eq!(result, DlvizResult::InvalidArgument);
        }
    }

    // Reading order traversal tests

    fn create_reading_order_test_elements() -> Vec<DlvizElement> {
        vec![
            DlvizElement {
                id: 1,
                bbox: DlvizBBox::default(),
                label: DlvizLabel::Title,
                confidence: 0.9,
                reading_order: 0,
            },
            DlvizElement {
                id: 2,
                bbox: DlvizBBox::default(),
                label: DlvizLabel::Text,
                confidence: 0.8,
                reading_order: 2, // Note: gap in sequence (no 1)
            },
            DlvizElement {
                id: 3,
                bbox: DlvizBBox::default(),
                label: DlvizLabel::Text,
                confidence: 0.85,
                reading_order: 5,
            },
            DlvizElement {
                id: 4,
                bbox: DlvizBBox::default(),
                label: DlvizLabel::Picture,
                confidence: 0.7,
                reading_order: -1, // Invalid reading order
            },
        ]
    }

    #[test]
    fn test_get_element_by_reading_order() {
        unsafe {
            let elements = create_reading_order_test_elements();

            assert_eq!(
                dlviz_get_element_by_reading_order(elements.as_ptr(), elements.len(), 0),
                0
            );
            assert_eq!(
                dlviz_get_element_by_reading_order(elements.as_ptr(), elements.len(), 2),
                1
            );
            assert_eq!(
                dlviz_get_element_by_reading_order(elements.as_ptr(), elements.len(), 5),
                2
            );
            // Non-existent reading order
            assert_eq!(
                dlviz_get_element_by_reading_order(elements.as_ptr(), elements.len(), 1),
                -1
            );
            assert_eq!(
                dlviz_get_element_by_reading_order(elements.as_ptr(), elements.len(), 10),
                -1
            );
            // Empty array
            assert_eq!(dlviz_get_element_by_reading_order(ptr::null(), 0, 0), -1);
        }
    }

    #[test]
    fn test_get_reading_order_range() {
        unsafe {
            let elements = create_reading_order_test_elements();

            let mut min_order: i32 = 0;
            let mut max_order: i32 = 0;
            let result = dlviz_get_reading_order_range(
                elements.as_ptr(),
                elements.len(),
                &mut min_order,
                &mut max_order,
            );

            assert_eq!(result, DlvizResult::Success);
            assert_eq!(min_order, 0);
            assert_eq!(max_order, 5);

            // Empty array should fail
            let result =
                dlviz_get_reading_order_range(ptr::null(), 0, &mut min_order, &mut max_order);
            assert_eq!(result, DlvizResult::InvalidArgument);
        }
    }

    #[test]
    fn test_get_next_reading_order_element() {
        unsafe {
            let elements = create_reading_order_test_elements();

            // Next after 0 should be 2 (index 1)
            assert_eq!(
                dlviz_get_next_reading_order_element(elements.as_ptr(), elements.len(), 0),
                1
            );
            // Next after 2 should be 5 (index 2)
            assert_eq!(
                dlviz_get_next_reading_order_element(elements.as_ptr(), elements.len(), 2),
                2
            );
            // Next after 5 should be none (-1)
            assert_eq!(
                dlviz_get_next_reading_order_element(elements.as_ptr(), elements.len(), 5),
                -1
            );
            // Next after -1 should be 0 (index 0)
            assert_eq!(
                dlviz_get_next_reading_order_element(elements.as_ptr(), elements.len(), -1),
                0
            );
        }
    }

    #[test]
    fn test_get_prev_reading_order_element() {
        unsafe {
            let elements = create_reading_order_test_elements();

            // Prev before 5 should be 2 (index 1)
            assert_eq!(
                dlviz_get_prev_reading_order_element(elements.as_ptr(), elements.len(), 5),
                1
            );
            // Prev before 2 should be 0 (index 0)
            assert_eq!(
                dlviz_get_prev_reading_order_element(elements.as_ptr(), elements.len(), 2),
                0
            );
            // Prev before 0 should be none (-1)
            assert_eq!(
                dlviz_get_prev_reading_order_element(elements.as_ptr(), elements.len(), 0),
                -1
            );
        }
    }

    #[test]
    fn test_get_first_last_reading_order_element() {
        unsafe {
            let elements = create_reading_order_test_elements();

            // First should be reading_order 0 (index 0)
            assert_eq!(
                dlviz_get_first_reading_order_element(elements.as_ptr(), elements.len()),
                0
            );
            // Last should be reading_order 5 (index 2)
            assert_eq!(
                dlviz_get_last_reading_order_element(elements.as_ptr(), elements.len()),
                2
            );

            // Empty array
            assert_eq!(dlviz_get_first_reading_order_element(ptr::null(), 0), -1);
            assert_eq!(dlviz_get_last_reading_order_element(ptr::null(), 0), -1);
        }
    }

    #[test]
    fn test_count_elements_with_reading_order() {
        unsafe {
            let elements = create_reading_order_test_elements();

            // 3 elements have valid reading order (0, 2, 5), 1 has -1
            assert_eq!(
                dlviz_count_elements_with_reading_order(elements.as_ptr(), elements.len()),
                3
            );

            // Empty array
            assert_eq!(dlviz_count_elements_with_reading_order(ptr::null(), 0), 0);
        }
    }

    #[test]
    fn test_reading_order_iteration() {
        unsafe {
            let elements = create_reading_order_test_elements();

            // Iterate through all elements in reading order
            let mut visited = Vec::new();
            let first = dlviz_get_first_reading_order_element(elements.as_ptr(), elements.len());
            assert!(first >= 0);

            let mut current_order = elements[first as usize].reading_order;
            visited.push(first as usize);

            loop {
                let next = dlviz_get_next_reading_order_element(
                    elements.as_ptr(),
                    elements.len(),
                    current_order,
                );
                if next < 0 {
                    break;
                }
                current_order = elements[next as usize].reading_order;
                visited.push(next as usize);
            }

            // Should have visited 3 elements (indices 0, 1, 2) in reading order
            assert_eq!(visited, vec![0, 1, 2]);
        }
    }

    /// Integration test: Run ML pipeline on a real PDF and verify we get real elements
    ///
    /// This test verifies the critical path:
    /// PDF → ML Pipeline → Stage Snapshots → Real Element Data
    #[test]
    #[cfg(feature = "pdf-ml")]
    fn test_ml_pipeline_integration() {
        unsafe {
            let pipeline = dlviz_pipeline_new();
            assert!(!pipeline.is_null());

            // Verify ML feature is compiled in
            assert!(dlviz_has_pdf_ml(), "pdf-ml feature should be enabled");

            // Find test PDF
            let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            let workspace_root = manifest_dir.parent().unwrap().parent().unwrap();
            let test_pdf = workspace_root.join("test-corpus/pdf/code_and_formula.pdf");

            if !test_pdf.exists() {
                println!("Test PDF not found, skipping ML integration test");
                dlviz_pipeline_free(pipeline);
                return;
            }

            // Load the PDF
            let path = std::ffi::CString::new(test_pdf.to_str().unwrap()).unwrap();
            let result = dlviz_load_pdf(pipeline, path.as_ptr());

            if result != DlvizResult::Success {
                println!(
                    "Could not load PDF (pdfium may not be available): {:?}",
                    result
                );
                dlviz_pipeline_free(pipeline);
                return;
            }

            // Check if ML pipeline initialized
            let has_ml = dlviz_pipeline_has_ml(pipeline);
            if !has_ml {
                println!("ML pipeline not available (models may not be loaded)");
                dlviz_pipeline_free(pipeline);
                return;
            }

            println!("ML pipeline is available, running layout detection...");

            // Run pipeline to LayoutDetection stage on page 0
            let run_result = dlviz_run_to_stage(pipeline, 0, DlvizStage::LayoutDetection);
            assert_eq!(
                run_result,
                DlvizResult::Success,
                "dlviz_run_to_stage should succeed"
            );

            // Get the layout detection snapshot
            let mut snapshot = DlvizStageSnapshot {
                stage: DlvizStage::RawPdf,
                element_count: 0,
                elements: ptr::null(),
                cell_count: 0,
                cells: ptr::null(),
                processing_time_ms: 0.0,
            };
            let snap_result =
                dlviz_get_stage_snapshot(pipeline, 0, DlvizStage::LayoutDetection, &mut snapshot);

            assert_eq!(
                snap_result,
                DlvizResult::Success,
                "dlviz_get_stage_snapshot should succeed"
            );

            // Verify we got real ML data
            println!(
                "ML Pipeline Results:\n  Stage: {:?}\n  Elements: {}\n  Cells: {}\n  Processing time: {:.2}ms",
                snapshot.stage,
                snapshot.element_count,
                snapshot.cell_count,
                snapshot.processing_time_ms
            );

            // Critical assertion: we should have detected elements
            assert!(
                snapshot.element_count > 0,
                "ML pipeline should detect at least one element in code_and_formula.pdf"
            );

            // Processing time should be positive (real inference happened)
            assert!(
                snapshot.processing_time_ms > 0.0,
                "Processing time should be positive (real ML inference)"
            );

            // Verify elements have valid data
            if snapshot.element_count > 0 && !snapshot.elements.is_null() {
                let elements =
                    std::slice::from_raw_parts(snapshot.elements, snapshot.element_count);
                for (i, elem) in elements.iter().enumerate() {
                    println!(
                        "  Element {}: label={:?}, bbox=({:.1},{:.1},{:.1}x{:.1}), conf={:.3}",
                        i,
                        elem.label,
                        elem.bbox.x,
                        elem.bbox.y,
                        elem.bbox.width,
                        elem.bbox.height,
                        elem.confidence
                    );

                    // Verify bounding boxes are valid
                    assert!(
                        elem.bbox.width > 0.0 && elem.bbox.height > 0.0,
                        "Element {} should have positive bbox dimensions",
                        i
                    );

                    // Confidence should be in [0, 1] range
                    assert!(
                        elem.confidence >= 0.0 && elem.confidence <= 1.0,
                        "Element {} confidence should be in [0,1] range",
                        i
                    );
                }
            }

            dlviz_pipeline_free(pipeline);
        }
    }

    #[test]
    #[cfg(all(feature = "pdf-render", feature = "pdf-ml"))]
    fn test_element_text_extraction() {
        unsafe {
            let pipeline = dlviz_pipeline_new();
            assert!(!pipeline.is_null());

            // Find test PDF
            let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            let workspace_root = manifest_dir.parent().unwrap().parent().unwrap();
            let test_pdf = workspace_root.join("test-corpus/pdf/code_and_formula.pdf");

            if !test_pdf.exists() {
                println!("Test PDF not found, skipping text extraction test");
                dlviz_pipeline_free(pipeline);
                return;
            }

            // Load the PDF
            let path = std::ffi::CString::new(test_pdf.to_str().unwrap()).unwrap();
            let result = dlviz_load_pdf(pipeline, path.as_ptr());

            if result != DlvizResult::Success {
                println!("Could not load PDF: {:?}", result);
                dlviz_pipeline_free(pipeline);
                return;
            }

            // Check if ML pipeline is available
            if !dlviz_pipeline_has_ml(pipeline) {
                println!("ML pipeline not available, skipping text extraction test");
                dlviz_pipeline_free(pipeline);
                return;
            }

            // Run pipeline to ReadingOrder stage
            let run_result = dlviz_run_to_stage(pipeline, 0, DlvizStage::ReadingOrder);
            assert_eq!(run_result, DlvizResult::Success);

            // Get snapshot to find element IDs
            let mut snapshot = DlvizStageSnapshot {
                stage: DlvizStage::RawPdf,
                element_count: 0,
                elements: ptr::null(),
                cell_count: 0,
                cells: ptr::null(),
                processing_time_ms: 0.0,
            };
            let snap_result =
                dlviz_get_stage_snapshot(pipeline, 0, DlvizStage::ReadingOrder, &mut snapshot);
            assert_eq!(snap_result, DlvizResult::Success);

            // Test text extraction API
            if snapshot.element_count > 0 && !snapshot.elements.is_null() {
                let elements =
                    std::slice::from_raw_parts(snapshot.elements, snapshot.element_count);

                // Try to get text for first element
                let first_elem = &elements[0];
                let mut buffer = [0i8; 1024];
                let mut actual_size: usize = 0;

                let text_result = dlviz_get_element_text(
                    pipeline,
                    0,
                    first_elem.id,
                    buffer.as_mut_ptr(),
                    buffer.len(),
                    &mut actual_size,
                );

                assert_eq!(
                    text_result,
                    DlvizResult::Success,
                    "dlviz_get_element_text should succeed"
                );

                // actual_size should be at least 1 (for null terminator)
                assert!(
                    actual_size >= 1,
                    "actual_size should include null terminator"
                );

                // Convert buffer to string for debugging
                let text = std::ffi::CStr::from_ptr(buffer.as_ptr())
                    .to_str()
                    .unwrap_or("");
                println!(
                    "Element {} (id={}) text: '{}' (len={})",
                    0,
                    first_elem.id,
                    text,
                    text.len()
                );

                // Note: Text may be empty because we're running without OCR
                // This test verifies the API works, not that text is present
            }

            // Test with invalid element ID - should still return success with empty string
            let mut buffer = [0i8; 256];
            let mut actual_size: usize = 0;
            let result = dlviz_get_element_text(
                pipeline,
                0,
                999999, // Non-existent element
                buffer.as_mut_ptr(),
                buffer.len(),
                &mut actual_size,
            );
            assert_eq!(
                result,
                DlvizResult::Success,
                "Text query for non-existent element should return Success with empty string"
            );
            assert_eq!(
                actual_size, 1,
                "Empty string needs 1 byte for null terminator"
            );

            dlviz_pipeline_free(pipeline);
        }
    }

    #[test]
    #[cfg(all(feature = "pdf-render", feature = "pdf-ml"))]
    fn test_json_export() {
        unsafe {
            let pipeline = dlviz_pipeline_new();
            assert!(!pipeline.is_null());

            // Find test PDF
            let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            let workspace_root = manifest_dir.parent().unwrap().parent().unwrap();
            let test_pdf = workspace_root.join("test-corpus/pdf/code_and_formula.pdf");

            if !test_pdf.exists() {
                println!("Test PDF not found, skipping JSON export test");
                dlviz_pipeline_free(pipeline);
                return;
            }

            // Load the PDF
            let path = std::ffi::CString::new(test_pdf.to_str().unwrap()).unwrap();
            let result = dlviz_load_pdf(pipeline, path.as_ptr());

            if result != DlvizResult::Success {
                println!("Could not load PDF: {:?}", result);
                dlviz_pipeline_free(pipeline);
                return;
            }

            // Check if ML pipeline is available
            if !dlviz_pipeline_has_ml(pipeline) {
                println!("ML pipeline not available, skipping JSON export test");
                dlviz_pipeline_free(pipeline);
                return;
            }

            // Run pipeline to ReadingOrder stage
            let run_result = dlviz_run_to_stage(pipeline, 0, DlvizStage::ReadingOrder);
            assert_eq!(run_result, DlvizResult::Success);

            // Export JSON
            let json_ptr = dlviz_export_json(pipeline, 0);
            assert!(!json_ptr.is_null(), "JSON export should return non-null");

            // Parse and verify JSON
            let json_str = std::ffi::CStr::from_ptr(json_ptr)
                .to_str()
                .expect("Invalid UTF-8 in JSON");

            println!(
                "JSON export (first 500 chars):\n{}",
                &json_str[..json_str.len().min(500)]
            );

            // Verify it's valid JSON
            let parsed: serde_json::Value =
                serde_json::from_str(json_str).expect("Invalid JSON output");

            // Verify structure
            assert!(parsed.is_object(), "JSON should be an object");
            assert!(
                parsed.get("page").is_some(),
                "JSON should have 'page' field"
            );
            assert!(
                parsed.get("stage").is_some(),
                "JSON should have 'stage' field"
            );
            assert!(
                parsed.get("elements").is_some(),
                "JSON should have 'elements' field"
            );
            assert!(
                parsed.get("element_count").is_some(),
                "JSON should have 'element_count' field"
            );

            // Verify element_count matches elements array length
            let elements = parsed.get("elements").unwrap().as_array().unwrap();
            let count = parsed.get("element_count").unwrap().as_u64().unwrap() as usize;
            assert_eq!(
                elements.len(),
                count,
                "element_count should match elements array length"
            );

            // Verify elements have required fields
            if !elements.is_empty() {
                let first = &elements[0];
                assert!(first.get("id").is_some(), "Element should have 'id' field");
                assert!(
                    first.get("bbox").is_some(),
                    "Element should have 'bbox' field"
                );
                assert!(
                    first.get("label").is_some(),
                    "Element should have 'label' field"
                );
                assert!(
                    first.get("confidence").is_some(),
                    "Element should have 'confidence' field"
                );
                assert!(
                    first.get("reading_order").is_some(),
                    "Element should have 'reading_order' field"
                );
            }

            // Free the JSON string
            dlviz_string_free(json_ptr);

            dlviz_pipeline_free(pipeline);
        }
    }

    #[test]
    #[cfg(all(feature = "pdf-render", feature = "pdf-ml"))]
    fn test_json_export_pretty() {
        unsafe {
            let pipeline = dlviz_pipeline_new();
            assert!(!pipeline.is_null());

            // Find test PDF
            let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            let workspace_root = manifest_dir.parent().unwrap().parent().unwrap();
            let test_pdf = workspace_root.join("test-corpus/pdf/code_and_formula.pdf");

            if !test_pdf.exists() {
                println!("Test PDF not found, skipping JSON pretty export test");
                dlviz_pipeline_free(pipeline);
                return;
            }

            // Load the PDF
            let path = std::ffi::CString::new(test_pdf.to_str().unwrap()).unwrap();
            let result = dlviz_load_pdf(pipeline, path.as_ptr());

            if result != DlvizResult::Success {
                println!("Could not load PDF: {:?}", result);
                dlviz_pipeline_free(pipeline);
                return;
            }

            // Check if ML pipeline is available
            if !dlviz_pipeline_has_ml(pipeline) {
                println!("ML pipeline not available, skipping JSON pretty export test");
                dlviz_pipeline_free(pipeline);
                return;
            }

            // Run pipeline to ReadingOrder stage
            let run_result = dlviz_run_to_stage(pipeline, 0, DlvizStage::ReadingOrder);
            assert_eq!(run_result, DlvizResult::Success);

            // Export pretty JSON
            let json_ptr = dlviz_export_json_pretty(pipeline, 0);
            assert!(
                !json_ptr.is_null(),
                "JSON pretty export should return non-null"
            );

            // Parse and verify JSON
            let json_str = std::ffi::CStr::from_ptr(json_ptr)
                .to_str()
                .expect("Invalid UTF-8 in JSON");

            // Pretty print should contain newlines
            assert!(
                json_str.contains('\n'),
                "Pretty JSON should contain newlines"
            );
            assert!(
                json_str.contains("  "),
                "Pretty JSON should contain indentation"
            );

            println!(
                "Pretty JSON export (first 500 chars):\n{}",
                &json_str[..json_str.len().min(500)]
            );

            // Verify it's valid JSON
            let parsed: serde_json::Value =
                serde_json::from_str(json_str).expect("Invalid JSON output");

            // Verify structure
            assert!(parsed.is_object(), "JSON should be an object");
            assert!(
                parsed.get("elements").is_some(),
                "JSON should have 'elements' field"
            );

            // Free the JSON string
            dlviz_string_free(json_ptr);

            dlviz_pipeline_free(pipeline);
        }
    }

    #[test]
    #[cfg(all(feature = "pdf-render", feature = "pdf-ml"))]
    fn test_export_all_pages_json() {
        unsafe {
            let pipeline = dlviz_pipeline_new();
            assert!(!pipeline.is_null());

            // Find test PDF
            let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            let workspace_root = manifest_dir.parent().unwrap().parent().unwrap();
            let test_pdf = workspace_root.join("test-corpus/pdf/code_and_formula.pdf");

            if !test_pdf.exists() {
                println!("Test PDF not found, skipping all-pages export test");
                dlviz_pipeline_free(pipeline);
                return;
            }

            // Load the PDF
            let path = std::ffi::CString::new(test_pdf.to_str().unwrap()).unwrap();
            let result = dlviz_load_pdf(pipeline, path.as_ptr());

            if result != DlvizResult::Success {
                println!("Could not load PDF: {:?}", result);
                dlviz_pipeline_free(pipeline);
                return;
            }

            // Check if ML pipeline is available
            if !dlviz_pipeline_has_ml(pipeline) {
                println!("ML pipeline not available, skipping all-pages export test");
                dlviz_pipeline_free(pipeline);
                return;
            }

            // Run pipeline on page 0
            let run_result = dlviz_run_to_stage(pipeline, 0, DlvizStage::ReadingOrder);
            assert_eq!(run_result, DlvizResult::Success);

            // Export all pages (pretty-printed)
            let json_ptr = dlviz_export_all_pages_json(pipeline, true);
            assert!(
                !json_ptr.is_null(),
                "All-pages export should return non-null"
            );

            // Parse and verify JSON
            let json_str = std::ffi::CStr::from_ptr(json_ptr)
                .to_str()
                .expect("Invalid UTF-8 in JSON");

            println!(
                "All-pages JSON export (first 800 chars):\n{}",
                &json_str[..json_str.len().min(800)]
            );

            // Verify it's valid JSON
            let parsed: serde_json::Value =
                serde_json::from_str(json_str).expect("Invalid JSON output");

            // Verify document-level structure
            assert!(parsed.is_object(), "JSON should be an object");
            assert!(
                parsed.get("document").is_some(),
                "JSON should have 'document' field"
            );
            assert!(
                parsed.get("page_count").is_some(),
                "JSON should have 'page_count' field"
            );
            assert!(
                parsed.get("pages").is_some(),
                "JSON should have 'pages' array"
            );

            // Verify pages array
            let pages = parsed.get("pages").unwrap().as_array().unwrap();
            let page_count = parsed.get("page_count").unwrap().as_u64().unwrap() as usize;
            assert_eq!(
                pages.len(),
                page_count,
                "page_count should match pages array length"
            );

            // Verify first page (which we processed)
            if !pages.is_empty() {
                let first_page = &pages[0];
                assert!(
                    first_page.get("page").is_some(),
                    "Page should have 'page' field"
                );
                assert!(
                    first_page.get("stage").is_some(),
                    "Page should have 'stage' field"
                );
                assert!(
                    first_page.get("elements").is_some(),
                    "Page should have 'elements' field"
                );

                // First page should be processed
                let stage = first_page.get("stage").unwrap().as_str().unwrap();
                assert_eq!(
                    stage, "reading_order",
                    "First page should have reading_order stage"
                );
            }

            // Free the JSON string
            dlviz_string_free(json_ptr);

            dlviz_pipeline_free(pipeline);
        }
    }

    #[test]
    #[cfg(all(feature = "pdf-render", feature = "pdf-ml"))]
    fn test_export_yolo() {
        unsafe {
            let pipeline = dlviz_pipeline_new();
            assert!(!pipeline.is_null());

            // Find test PDF
            let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            let workspace_root = manifest_dir.parent().unwrap().parent().unwrap();
            let test_pdf = workspace_root.join("test-corpus/pdf/code_and_formula.pdf");

            if !test_pdf.exists() {
                println!("Test PDF not found, skipping YOLO export test");
                dlviz_pipeline_free(pipeline);
                return;
            }

            // Load the PDF
            let path = std::ffi::CString::new(test_pdf.to_str().unwrap()).unwrap();
            let result = dlviz_load_pdf(pipeline, path.as_ptr());

            if result != DlvizResult::Success {
                println!("Could not load PDF: {:?}", result);
                dlviz_pipeline_free(pipeline);
                return;
            }

            // Check if ML pipeline is available
            if !dlviz_pipeline_has_ml(pipeline) {
                println!("ML pipeline not available, skipping YOLO export test");
                dlviz_pipeline_free(pipeline);
                return;
            }

            // Run pipeline
            let run_result = dlviz_run_to_stage(pipeline, 0, DlvizStage::ReadingOrder);
            assert_eq!(run_result, DlvizResult::Success);

            // Export YOLO
            let yolo_ptr = dlviz_export_yolo(pipeline, 0);
            assert!(!yolo_ptr.is_null(), "YOLO export should return non-null");

            // Parse and verify YOLO format
            let yolo_str = std::ffi::CStr::from_ptr(yolo_ptr)
                .to_str()
                .expect("Invalid UTF-8 in YOLO");

            println!("YOLO export:\n{}", yolo_str);

            // Verify format: each line should have 5 space-separated values
            for line in yolo_str.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                assert_eq!(
                    parts.len(),
                    5,
                    "YOLO line should have 5 values: class_id x_center y_center width height"
                );

                // Verify class_id is an integer
                parts[0]
                    .parse::<i32>()
                    .expect("class_id should be an integer");

                // Verify coordinates are valid floats
                // Note: values may be outside 0-1 range if detections extend beyond page boundaries
                for i in 1..5 {
                    parts[i]
                        .parse::<f32>()
                        .expect("Coordinate should be a float");
                }
            }

            // Free the string
            dlviz_string_free(yolo_ptr);

            dlviz_pipeline_free(pipeline);
        }
    }

    #[test]
    #[cfg(all(feature = "pdf-render", feature = "pdf-ml"))]
    fn test_export_coco_annotations() {
        unsafe {
            let pipeline = dlviz_pipeline_new();
            assert!(!pipeline.is_null());

            // Find test PDF
            let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            let workspace_root = manifest_dir.parent().unwrap().parent().unwrap();
            let test_pdf = workspace_root.join("test-corpus/pdf/code_and_formula.pdf");

            if !test_pdf.exists() {
                println!("Test PDF not found, skipping COCO export test");
                dlviz_pipeline_free(pipeline);
                return;
            }

            // Load the PDF
            let path = std::ffi::CString::new(test_pdf.to_str().unwrap()).unwrap();
            let result = dlviz_load_pdf(pipeline, path.as_ptr());

            if result != DlvizResult::Success {
                println!("Could not load PDF: {:?}", result);
                dlviz_pipeline_free(pipeline);
                return;
            }

            // Check if ML pipeline is available
            if !dlviz_pipeline_has_ml(pipeline) {
                println!("ML pipeline not available, skipping COCO export test");
                dlviz_pipeline_free(pipeline);
                return;
            }

            // Run pipeline
            let run_result = dlviz_run_to_stage(pipeline, 0, DlvizStage::ReadingOrder);
            assert_eq!(run_result, DlvizResult::Success);

            // Export COCO annotations
            let coco_ptr = dlviz_export_coco_annotations(pipeline, 0, 1, 100);
            assert!(!coco_ptr.is_null(), "COCO export should return non-null");

            // Parse and verify COCO format
            let coco_str = std::ffi::CStr::from_ptr(coco_ptr)
                .to_str()
                .expect("Invalid UTF-8 in COCO");

            println!(
                "COCO annotations (first 500 chars):\n{}",
                &coco_str[..coco_str.len().min(500)]
            );

            // Verify it's valid JSON array
            let parsed: serde_json::Value =
                serde_json::from_str(coco_str).expect("Invalid JSON output");
            assert!(parsed.is_array(), "COCO should be an array");

            // Verify annotation structure
            let annotations = parsed.as_array().unwrap();
            if !annotations.is_empty() {
                let first = &annotations[0];
                assert!(
                    first.get("id").is_some(),
                    "Annotation should have 'id' field"
                );
                assert!(
                    first.get("image_id").is_some(),
                    "Annotation should have 'image_id' field"
                );
                assert!(
                    first.get("category_id").is_some(),
                    "Annotation should have 'category_id' field"
                );
                assert!(
                    first.get("bbox").is_some(),
                    "Annotation should have 'bbox' field"
                );
                assert!(
                    first.get("area").is_some(),
                    "Annotation should have 'area' field"
                );
                assert!(
                    first.get("score").is_some(),
                    "Annotation should have 'score' field"
                );

                // Verify image_id matches what we passed
                assert_eq!(first.get("image_id").unwrap().as_u64().unwrap(), 1);

                // Verify annotation IDs start from 100
                assert!(first.get("id").unwrap().as_u64().unwrap() >= 100);
            }

            // Free the string
            dlviz_string_free(coco_ptr);

            dlviz_pipeline_free(pipeline);
        }
    }

    #[test]
    fn test_coco_categories() {
        unsafe {
            let categories_ptr = dlviz_get_coco_categories();
            assert!(
                !categories_ptr.is_null(),
                "COCO categories should return non-null"
            );

            // Parse and verify
            let categories_str = std::ffi::CStr::from_ptr(categories_ptr)
                .to_str()
                .expect("Invalid UTF-8 in categories");

            let parsed: serde_json::Value =
                serde_json::from_str(categories_str).expect("Invalid JSON output");
            assert!(parsed.is_array(), "Categories should be an array");

            let categories = parsed.as_array().unwrap();
            assert_eq!(
                categories.len(),
                17,
                "Should have 17 categories (one per label)"
            );

            // Verify structure of first category
            let first = &categories[0];
            assert!(first.get("id").is_some(), "Category should have 'id' field");
            assert!(
                first.get("name").is_some(),
                "Category should have 'name' field"
            );
            assert!(
                first.get("supercategory").is_some(),
                "Category should have 'supercategory' field"
            );

            dlviz_string_free(categories_ptr);
        }
    }

    #[test]
    fn test_dlviz_result_display() {
        assert_eq!(format!("{}", DlvizResult::Success), "success");
        assert_eq!(
            format!("{}", DlvizResult::InvalidArgument),
            "invalid argument"
        );
        assert_eq!(format!("{}", DlvizResult::FileNotFound), "file not found");
        assert_eq!(format!("{}", DlvizResult::ParseError), "parse error");
        assert_eq!(
            format!("{}", DlvizResult::InferenceError),
            "inference error"
        );
        assert_eq!(format!("{}", DlvizResult::OutOfMemory), "out of memory");
        assert_eq!(format!("{}", DlvizResult::InternalError), "internal error");
    }

    #[test]
    fn test_dlviz_label_display() {
        assert_eq!(format!("{}", DlvizLabel::Caption), "caption");
        assert_eq!(format!("{}", DlvizLabel::Table), "table");
        assert_eq!(format!("{}", DlvizLabel::Text), "text");
        assert_eq!(format!("{}", DlvizLabel::Title), "title");
        assert_eq!(
            format!("{}", DlvizLabel::KeyValueRegion),
            "key-value region"
        );
    }

    #[test]
    fn test_dlviz_stage_display() {
        assert_eq!(format!("{}", DlvizStage::RawPdf), "raw PDF");
        assert_eq!(format!("{}", DlvizStage::OcrDetection), "OCR detection");
        assert_eq!(
            format!("{}", DlvizStage::LayoutDetection),
            "layout detection"
        );
        assert_eq!(format!("{}", DlvizStage::ReadingOrder), "reading order");
    }

    #[test]
    fn test_dlviz_result_from_str() {
        use std::str::FromStr;

        // Primary names
        assert_eq!(
            DlvizResult::from_str("success").unwrap(),
            DlvizResult::Success
        );
        assert_eq!(
            DlvizResult::from_str("invalid argument").unwrap(),
            DlvizResult::InvalidArgument
        );
        assert_eq!(
            DlvizResult::from_str("file not found").unwrap(),
            DlvizResult::FileNotFound
        );
        assert_eq!(
            DlvizResult::from_str("parse error").unwrap(),
            DlvizResult::ParseError
        );
        assert_eq!(
            DlvizResult::from_str("inference error").unwrap(),
            DlvizResult::InferenceError
        );
        assert_eq!(
            DlvizResult::from_str("out of memory").unwrap(),
            DlvizResult::OutOfMemory
        );
        assert_eq!(
            DlvizResult::from_str("internal error").unwrap(),
            DlvizResult::InternalError
        );

        // Aliases
        assert_eq!(DlvizResult::from_str("ok").unwrap(), DlvizResult::Success);
        assert_eq!(
            DlvizResult::from_str("OOM").unwrap(),
            DlvizResult::OutOfMemory
        );

        // Error case
        assert!(DlvizResult::from_str("unknown").is_err());
    }

    #[test]
    fn test_dlviz_result_roundtrip() {
        use std::str::FromStr;

        for res in [
            DlvizResult::Success,
            DlvizResult::InvalidArgument,
            DlvizResult::FileNotFound,
            DlvizResult::ParseError,
            DlvizResult::InferenceError,
            DlvizResult::OutOfMemory,
            DlvizResult::InternalError,
        ] {
            let s = res.to_string();
            let parsed = DlvizResult::from_str(&s).unwrap();
            assert_eq!(res, parsed, "roundtrip failed for {s}");
        }
    }

    #[test]
    fn test_dlviz_label_from_str() {
        use std::str::FromStr;

        // Primary names (with various formats)
        assert_eq!(
            DlvizLabel::from_str("caption").unwrap(),
            DlvizLabel::Caption
        );
        assert_eq!(
            DlvizLabel::from_str("list item").unwrap(),
            DlvizLabel::ListItem
        );
        assert_eq!(
            DlvizLabel::from_str("list_item").unwrap(),
            DlvizLabel::ListItem
        );
        assert_eq!(
            DlvizLabel::from_str("list-item").unwrap(),
            DlvizLabel::ListItem
        );
        assert_eq!(
            DlvizLabel::from_str("section header").unwrap(),
            DlvizLabel::SectionHeader
        );
        assert_eq!(
            DlvizLabel::from_str("key-value region").unwrap(),
            DlvizLabel::KeyValueRegion
        );

        // Aliases
        assert_eq!(DlvizLabel::from_str("image").unwrap(), DlvizLabel::Picture);
        assert_eq!(
            DlvizLabel::from_str("heading").unwrap(),
            DlvizLabel::SectionHeader
        );
        assert_eq!(
            DlvizLabel::from_str("toc").unwrap(),
            DlvizLabel::DocumentIndex
        );
        assert_eq!(
            DlvizLabel::from_str("kv").unwrap(),
            DlvizLabel::KeyValueRegion
        );

        // Error case
        assert!(DlvizLabel::from_str("unknown_label").is_err());
    }

    #[test]
    fn test_dlviz_label_roundtrip() {
        use std::str::FromStr;

        for label in [
            DlvizLabel::Caption,
            DlvizLabel::Footnote,
            DlvizLabel::Formula,
            DlvizLabel::ListItem,
            DlvizLabel::PageFooter,
            DlvizLabel::PageHeader,
            DlvizLabel::Picture,
            DlvizLabel::SectionHeader,
            DlvizLabel::Table,
            DlvizLabel::Text,
            DlvizLabel::Title,
            DlvizLabel::Code,
            DlvizLabel::CheckboxSelected,
            DlvizLabel::CheckboxUnselected,
            DlvizLabel::DocumentIndex,
            DlvizLabel::Form,
            DlvizLabel::KeyValueRegion,
        ] {
            let s = label.to_string();
            let parsed = DlvizLabel::from_str(&s).unwrap();
            assert_eq!(label, parsed, "roundtrip failed for {s}");
        }
    }

    #[test]
    fn test_dlviz_stage_from_str() {
        use std::str::FromStr;

        // Primary names
        assert_eq!(DlvizStage::from_str("raw PDF").unwrap(), DlvizStage::RawPdf);
        assert_eq!(
            DlvizStage::from_str("ocr detection").unwrap(),
            DlvizStage::OcrDetection
        );
        assert_eq!(
            DlvizStage::from_str("layout detection").unwrap(),
            DlvizStage::LayoutDetection
        );
        assert_eq!(
            DlvizStage::from_str("reading order").unwrap(),
            DlvizStage::ReadingOrder
        );

        // Stage numbers
        assert_eq!(DlvizStage::from_str("0").unwrap(), DlvizStage::RawPdf);
        assert_eq!(
            DlvizStage::from_str("3").unwrap(),
            DlvizStage::LayoutDetection
        );
        assert_eq!(
            DlvizStage::from_str("10").unwrap(),
            DlvizStage::ReadingOrder
        );

        // Aliases
        assert_eq!(
            DlvizStage::from_str("layout").unwrap(),
            DlvizStage::LayoutDetection
        );
        assert_eq!(
            DlvizStage::from_str("ocr").unwrap(),
            DlvizStage::OcrDetection
        );

        // Error case
        assert!(DlvizStage::from_str("unknown_stage").is_err());
        assert!(DlvizStage::from_str("11").is_err()); // Out of range
    }

    #[test]
    fn test_dlviz_stage_roundtrip() {
        use std::str::FromStr;

        for stage in [
            DlvizStage::RawPdf,
            DlvizStage::OcrDetection,
            DlvizStage::OcrRecognition,
            DlvizStage::LayoutDetection,
            DlvizStage::CellAssignment,
            DlvizStage::EmptyClusterRemoval,
            DlvizStage::OrphanDetection,
            DlvizStage::BBoxAdjust1,
            DlvizStage::BBoxAdjust2,
            DlvizStage::FinalAssembly,
            DlvizStage::ReadingOrder,
        ] {
            let s = stage.to_string();
            let parsed = DlvizStage::from_str(&s).unwrap();
            assert_eq!(stage, parsed, "roundtrip failed for {s}");
        }
    }
}
