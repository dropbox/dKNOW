//! # pdfium-render-fast
//!
//! Fast, thread-safe PDF rendering and text extraction using optimized PDFium.
//!
//! This crate provides a high-level, pdfium-render-compatible API with significant
//! performance improvements:
//!
//! - **72x faster rendering** via parallel processing
//! - **545x speedup** for scanned PDFs via JPEG fast path
//! - **Batch text extraction** to reduce FFI overhead
//! - **Thread-safe** document and page access
//!
//! ## Quick Start
//!
//! ```no_run
//! use pdfium_render_fast::{Pdfium, PdfRenderConfig};
//!
//! // Initialize PDFium
//! let pdfium = Pdfium::new()?;
//!
//! // Open a document
//! let doc = pdfium.load_pdf_from_file("document.pdf", None)?;
//!
//! // Extract text from all pages
//! for page in doc.pages() {
//!     let text = page.text()?;
//!     println!("{}", text.all());
//! }
//!
//! // Render pages as images
//! for (i, page) in doc.pages().enumerate() {
//!     let config = PdfRenderConfig::new()
//!         .set_target_dpi(300.0);
//!     let bitmap = page.render_with_config(&config)?;
//!     bitmap.save_as_png(&format!("page_{}.png", i))?;
//! }
//! # Ok::<(), pdfium_render_fast::PdfError>(())
//! ```

mod action;
mod annotation;
mod attachment;
mod bookmark;
mod destination;
mod docling;
mod document;
mod error;
mod form;
mod javascript;
mod link;
mod page;
mod page_label;
mod page_object;
mod pdfium;
mod render;
mod search;
mod signature;
mod structure;
mod text;

pub use action::{ActionDetails, ActionType, PdfAction};
pub use annotation::{
    AnnotBorder, AnnotColor, AnnotFlags, AnnotRect, AppearanceMode, PdfAnnotation,
    PdfAnnotationType, PdfPageAnnotations,
};
pub use attachment::{
    AttachmentMetadata, AttachmentValueType, PdfAttachment, PdfAttachments, PdfAttachmentsIter,
};
pub use bookmark::{
    flatten_bookmarks, FlatBookmark, PdfBookmark, PdfBookmarkChildIter, PdfBookmarkIter,
};
pub use destination::{DestLocation, DestViewType, PdfDestination};
pub use document::{
    BlendMode, ClipPathSegment, DuplexType, FileIdentifierType, FlattenMode, FlattenResult,
    IccColorSpace, IccProfile, LineCap, LineJoin, PageMode, PageObjectType, PdfClipPath,
    PdfDocument, PdfLoadedFont, PdfNamedDestsIter, PdfNewPageObject, PdfXObject, RepeatedRegion,
    SaveFlags, StandardFont,
};
pub use error::{PdfError, Result};
pub use form::{
    FormError, FormFieldFlags, FormFieldOption, FormResult, PdfFormField, PdfFormFieldEditor,
    PdfFormFieldType, PdfFormType, PdfPageFormFieldEditors, PdfPageFormFields,
};
pub use javascript::{
    PdfJavaScriptAction, PdfJavaScriptActions, PdfJavaScriptActionsIntoIter,
    PdfJavaScriptActionsIter,
};
pub use link::{LinkRect, PdfLink, PdfPageLinks};
pub use page::{
    is_hiragana, is_japanese_char, is_kanji, is_katakana, is_known_math_font, AlignedColumn,
    AlignmentType, AlternatingPattern, BracketType, BracketedReference, CenteredBlock,
    ColumnGutter, ColumnLayout, DensityCell, DensityMap, EmphasisMark, EmphasisMarkType,
    FontUsageInfo, GapMatrix, GapOrientation, GridAnalysis, GridIntersection, IndentationAnalysis,
    IndentedLine, JPunctType, JapaneseCharAnalysis, JapanesePunctuation, ListMarker,
    ListMarkerType, MathCharAnalysis, NumericRegion, PdfClipRect, PdfMatrix, PdfPage, PdfPageBox,
    PdfPageRotation, ReferencePosition, RubyAnnotation, ScannedPageContent, ScriptChar,
    ScriptCluster, ScriptPosition, TextBlockMetrics, TextCluster, TextDecoration,
    TextDecorationType, WhitespaceGap, WritingDirection, WritingDirectionInfo,
};
pub use page_object::{
    ArtifactType, ColoredRegion, ExtractedLine, FontFlags, ImageColorspace, ImageFilter,
    ImageMetadata, ImageTechMeta, ObjectBounds, ObjectColor, ObjectMatrix, PathDrawMode,
    PathFillMode, PathSegment, PathSegmentType, PdfFont, PdfPageObject, PdfPageObjectType,
    PdfPageObjects, PdfPageObjectsIter, TextRenderMode,
};
pub use pdfium::{
    // Data Availability API
    check_linearization,
    // Core
    clear_unsupported_feature_handler,
    // Font System API
    get_default_ttf_map,
    get_default_ttf_map_count,
    get_default_ttf_map_entry,
    get_first_available_page,
    set_unsupported_feature_handler,
    CharsetFontMapping,
    DataAvailability,
    FontCharset,
    FormAvailability,
    LinearizationStatus,
    Pdfium,
    UnsupportedFeature,
};
pub use render::{
    PdfBitmap, PdfRenderConfig, PixelFormat, ProgressiveRender, RenderStatus, RenderedPage,
};
pub use search::{PdfSearchOptions, PdfTextSearch};
pub use signature::{
    DocMDPPermission, PdfSignature, PdfSignatures, PdfSignaturesIntoIter, PdfSignaturesIter,
};
pub use structure::{
    PdfStructAttribute,
    PdfStructAttributeValue,
    PdfStructAttributeValueType,
    PdfStructElement,
    PdfStructElementChildIter,
    PdfStructTree,
    PdfStructTreeChildIter,
    // Tagged table extraction for docling
    TaggedTable,
    TaggedTableCell,
    TaggedTableRow,
};
pub use text::{font_flags, PdfChar, PdfPageText, PdfTextCell, PdfWord};

// Docling integration features
pub use docling::{
    // Font analysis
    analyze_font_clusters,
    // Layout detection
    detect_layout_regions,
    // Reading order
    extract_reading_order,
    // Document classification
    DoclingClassification,
    DocumentType,
    FontCluster,
    FontSemanticRole,
    // Image hints
    ImageContentHint,
    LayoutRegion,
    LayoutRegionType,
    ReadingOrderSegment,
};

/// Re-export of parallel rendering constants
pub use pdfium_sys::{FPDF_GetOptimalWorkerCount, FPDF_GetOptimalWorkerCountForDocument};
