//! Shared utility functions for backend implementations.
//!
//! This module provides common helpers to reduce code duplication across
//! different document backend implementations.

// Clippy pedantic allows:
// - Coordinate calculations use f64 from usize
#![allow(clippy::cast_precision_loss)]

use crate::traits::BackendOptions;
use docling_core::content::{CoordOrigin, ProvenanceItem};
use docling_core::{DoclingError, Document, DocumentMetadata, InputFormat};
use std::fmt::Write;
use std::path::{Path, PathBuf};

/// Number of bytes in a kilobyte (1024 bytes = 1 KB).
///
/// Used for human-readable file size formatting in [`format_file_size`].
pub const BYTES_PER_KB: f64 = 1024.0;

// ========== Image MIME Type Constants ==========
// These constants are used by multiple backends (DOCX, XLSX, PPTX) when
// embedding images. Centralizing them reduces duplication and ensures consistency.

/// MIME type for PNG images.
pub const MIME_IMAGE_PNG: &str = "image/png";

/// MIME type for JPEG images.
pub const MIME_IMAGE_JPEG: &str = "image/jpeg";

/// MIME type for GIF images.
pub const MIME_IMAGE_GIF: &str = "image/gif";

/// MIME type for BMP images.
pub const MIME_IMAGE_BMP: &str = "image/bmp";

/// MIME type for SVG images.
pub const MIME_IMAGE_SVG: &str = "image/svg+xml";

/// Default MIME type for unknown image formats.
pub const MIME_IMAGE_UNKNOWN: &str = "image/unknown";

/// Default MIME type for generic binary data.
pub const MIME_OCTET_STREAM: &str = "application/octet-stream";

/// Detect image MIME type from a file extension.
///
/// Performs case-insensitive matching of common image file extensions to their
/// corresponding MIME types. This function consolidates duplicate MIME type
/// detection logic that existed across multiple backends (DOCX, XLSX, PPTX).
///
/// # Arguments
///
/// * `extension` - The file extension (without leading dot), e.g., "png", "jpg"
/// * `fallback` - MIME type to return if extension is not recognized
///
/// # Returns
///
/// The corresponding MIME type string, or the fallback if not recognized.
///
/// # Supported Extensions
///
/// - `png` → `image/png`
/// - `jpg`, `jpeg` → `image/jpeg`
/// - `gif` → `image/gif`
/// - `bmp` → `image/bmp`
/// - `svg` → `image/svg+xml`
///
/// # Exampless
///
/// ```
/// use docling_backend::utils::{mime_type_from_extension, MIME_IMAGE_PNG, MIME_OCTET_STREAM};
///
/// assert_eq!(mime_type_from_extension("png", MIME_OCTET_STREAM), MIME_IMAGE_PNG);
/// assert_eq!(mime_type_from_extension("PNG", MIME_OCTET_STREAM), MIME_IMAGE_PNG);
/// assert_eq!(mime_type_from_extension("unknown", MIME_OCTET_STREAM), MIME_OCTET_STREAM);
/// ```
#[inline]
#[must_use = "returns the detected MIME type"]
pub const fn mime_type_from_extension<'a>(extension: &str, fallback: &'a str) -> &'a str {
    if extension.eq_ignore_ascii_case("png") {
        MIME_IMAGE_PNG
    } else if extension.eq_ignore_ascii_case("jpg") || extension.eq_ignore_ascii_case("jpeg") {
        MIME_IMAGE_JPEG
    } else if extension.eq_ignore_ascii_case("gif") {
        MIME_IMAGE_GIF
    } else if extension.eq_ignore_ascii_case("bmp") {
        MIME_IMAGE_BMP
    } else if extension.eq_ignore_ascii_case("svg") {
        MIME_IMAGE_SVG
    } else {
        fallback
    }
}

/// Detect image MIME type from a file path.
///
/// Extracts the file extension from the path and determines the MIME type.
/// This is a convenience wrapper around [`mime_type_from_extension`] that
/// handles path parsing.
///
/// # Arguments
///
/// * `path` - The file path (can be relative or absolute)
/// * `fallback` - MIME type to return if extension is not recognized or missing
///
/// # Returns
///
/// The corresponding MIME type string, or the fallback if not recognized.
///
/// # Exampless
///
/// ```
/// use docling_backend::utils::{mime_type_from_path, MIME_IMAGE_JPEG, MIME_OCTET_STREAM};
///
/// assert_eq!(mime_type_from_path("image.jpg", MIME_OCTET_STREAM), MIME_IMAGE_JPEG);
/// assert_eq!(mime_type_from_path("/path/to/Photo.PNG", MIME_OCTET_STREAM), "image/png");
/// assert_eq!(mime_type_from_path("no_extension", MIME_OCTET_STREAM), MIME_OCTET_STREAM);
/// ```
#[inline]
#[must_use = "returns the detected MIME type"]
pub fn mime_type_from_path<'a>(path: &str, fallback: &'a str) -> &'a str {
    std::path::Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map_or(fallback, |ext| mime_type_from_extension(ext, fallback))
}

/// Build a minimal Document with only markdown content and character count.
///
/// This is the most common document construction pattern used across backends
/// when no additional metadata is available.
///
/// # Arguments
///
/// * `markdown` - The markdown content of the document
/// * `format` - The input format of the source document
///
/// # Examples
///
/// ```no_run
/// use docling_backend::utils::build_minimal_document;
/// use docling_core::InputFormat;
///
/// let markdown = "# Hello World\n\nThis is a test.".to_string();
/// let doc = build_minimal_document(markdown, InputFormat::Md);
/// assert_eq!(doc.metadata.num_characters, 32);
/// ```
#[must_use = "creates a minimal document with markdown content"]
pub fn build_minimal_document(markdown: String, format: InputFormat) -> Document {
    let num_characters = markdown.chars().count();
    Document {
        markdown,
        format,
        metadata: DocumentMetadata {
            num_characters,
            ..Default::default()
        },
        content_blocks: None,
        docling_document: None,
    }
}

/// Build a Document with markdown content and optional title metadata.
///
/// Similar to `build_minimal_document` but includes title metadata if available.
///
/// # Arguments
///
/// * `markdown` - The markdown content of the document
/// * `format` - The input format of the source document
/// * `title` - Optional document title
///
/// # Examples
///
/// ```no_run
/// use docling_backend::utils::build_document_with_title;
/// use docling_core::InputFormat;
///
/// let markdown = "# Hello World".to_string();
/// let doc = build_document_with_title(
///     markdown,
///     InputFormat::Docx,
///     Some("My Document".to_string())
/// );
/// assert_eq!(doc.metadata.title, Some("My Document".to_string()));
/// ```
#[must_use = "creates a document with markdown content and optional title"]
pub fn build_document_with_title(
    markdown: String,
    format: InputFormat,
    title: Option<String>,
) -> Document {
    let num_characters = markdown.chars().count();
    Document {
        markdown,
        format,
        metadata: DocumentMetadata {
            num_characters,
            title,
            ..Default::default()
        },
        content_blocks: None,
        docling_document: None,
    }
}

/// Convert a vector to `Option::Some` if non-empty, None if empty.
///
/// Common pattern in backends: `content_blocks: opt_vec(doc_items)`
/// instead of `content_blocks: if doc_items.is_empty() { None } else { Some(doc_items) }`
///
/// # Examples
///
/// ```
/// use docling_backend::utils::opt_vec;
///
/// let empty: Vec<i32> = vec![];
/// assert!(opt_vec(empty).is_none());
///
/// let non_empty = vec![1, 2, 3];
/// assert_eq!(opt_vec(non_empty), Some(vec![1, 2, 3]));
/// ```
#[inline]
#[must_use = "converts empty vectors to None, non-empty to Some"]
pub fn opt_vec<T>(vec: Vec<T>) -> Option<Vec<T>> {
    if vec.is_empty() {
        None
    } else {
        Some(vec)
    }
}

/// Create a standardized error for formats that require a file path.
///
/// Many document formats (ZIP archives, binary formats) cannot be parsed from
/// byte arrays alone and require file path access for extraction or special handling.
///
/// # Arguments
///
/// * `format` - The input format that requires a file path
/// * `reason` - Brief explanation of why file path is needed (e.g., "ZIP archive", "binary format")
///
/// # Examples
///
/// ```no_run
/// use docling_backend::utils::file_path_required_error;
/// use docling_core::InputFormat;
///
/// let error = file_path_required_error(InputFormat::Epub, "ZIP archive");
/// assert!(error.to_string().contains("requires file path"));
/// ```
#[must_use = "creates an error for formats requiring file paths"]
pub fn file_path_required_error(format: InputFormat, reason: &str) -> DoclingError {
    DoclingError::BackendError(format!(
        "{format:?} format requires file path ({reason}), parse_bytes() not supported - use parse_file() instead"
    ))
}

/// Helper for parsing files with special format handling.
///
/// Many backends need to handle certain formats specially (via file path) while
/// delegating other formats to byte-based parsing. This function encapsulates
/// that common pattern.
///
/// # Arguments
///
/// * `path` - Path to the file to parse
/// * `format` - The input format being parsed
/// * `special_formats` - List of formats that require special (file-based) handling
/// * `special_handler` - Function to handle special formats (takes path, returns markdown)
/// * `fallback_handler` - Function to handle normal formats (takes bytes, returns Document)
/// * `options` - Backend parsing options
///
/// # Returns
///
/// A `Document` parsed either via special handling or fallback handler
///
/// # Examples
///
/// ```no_run
/// use docling_backend::utils::parse_file_with_special_handling;
/// use docling_backend::traits::BackendOptions;
/// use docling_core::InputFormat;
/// use std::path::Path;
///
/// let result = parse_file_with_special_handling(
///     Path::new("test.msg"),
///     InputFormat::Msg,
///     &[InputFormat::Msg],
///     |path| Ok("Parsed MSG content".to_string()),
///     |bytes, opts| {
///         // Fallback parsing logic
///         Ok(docling_core::Document::from_markdown(String::new(), InputFormat::Msg))
///     },
///     &BackendOptions::default(),
/// );
/// ```
///
/// # Errors
///
/// Returns an error if file reading fails or if the handler returns an error.
#[must_use = "this function returns a parsed document that should be processed"]
pub fn parse_file_with_special_handling<P, F, G>(
    path: P,
    format: InputFormat,
    special_formats: &[InputFormat],
    special_handler: F,
    fallback_handler: G,
    options: &BackendOptions,
) -> Result<Document, DoclingError>
where
    P: AsRef<Path>,
    F: Fn(&Path) -> Result<String, DoclingError>,
    G: Fn(&[u8], &BackendOptions) -> Result<Document, DoclingError>,
{
    let path_ref = path.as_ref();

    // Check if this format requires special file-based handling
    if special_formats.contains(&format) {
        let markdown = special_handler(path_ref)?;
        return Ok(build_minimal_document(markdown, format));
    }

    // Otherwise, read file and use fallback byte-based handler
    let content = std::fs::read(path_ref).map_err(DoclingError::IoError)?;

    fallback_handler(&content, options)
}

/// Create a backend error with standardized formatting.
///
/// Ensures consistent error message format across all backends.
///
/// # Arguments
///
/// * `operation` - The operation that failed (e.g., "parse", "extract", "convert")
/// * `format_name` - Human-readable format name (e.g., "EPUB", "ZIP", "SRT")
/// * `error` - The underlying error
///
/// # Examples
///
/// ```no_run
/// use docling_backend::utils::backend_error;
/// use std::io;
///
/// let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
/// let error = backend_error("parse", "EPUB", io_err);
/// assert!(error.to_string().contains("Failed to parse EPUB"));
/// ```
#[must_use = "creates a backend error with formatted message"]
pub fn backend_error<E: std::fmt::Display>(
    operation: &str,
    format_name: &str,
    error: E,
) -> DoclingError {
    DoclingError::BackendError(format!("Failed to {operation} {format_name}: {error}"))
}

/// Add a markdown title header to a string buffer.
///
/// Formats: `# {title}\n\n`
///
/// # Arguments
///
/// * `markdown` - The markdown string buffer to append to
/// * `title` - The title text
///
/// # Examples
///
/// ```no_run
/// use docling_backend::utils::add_document_title;
///
/// let mut md = String::new();
/// add_document_title(&mut md, "My Document");
/// assert_eq!(md, "# My Document\n\n");
/// ```
#[inline]
pub fn add_document_title(markdown: &mut String, title: &str) {
    let _ = write!(markdown, "# {title}\n\n");
}

/// Add a YAML-style metadata block to a markdown string buffer.
///
/// Formats metadata as:
/// ```text
/// ---
/// **Key1:** Value1
/// **Key2:** Value2
/// ---
///
/// ```
///
/// # Arguments
///
/// * `markdown` - The markdown string buffer to append to
/// * `metadata` - List of (key, value) pairs to include
///
/// # Examples
///
/// ```no_run
/// use docling_backend::utils::add_metadata_block;
///
/// let mut md = String::new();
/// add_metadata_block(&mut md, &[
///     ("Title", "My Document"),
///     ("Author", "John Doe"),
/// ]);
/// assert!(md.contains("**Title:** My Document"));
/// ```
#[inline]
pub fn add_metadata_block(markdown: &mut String, metadata: &[(&str, &str)]) {
    if metadata.is_empty() {
        return;
    }

    markdown.push_str("---\n");
    for (key, value) in metadata {
        let _ = writeln!(markdown, "**{key}:** {value}");
    }
    markdown.push_str("---\n\n");
}

/// Write data to a temporary file and return its path.
///
/// Creates a temporary file with the specified prefix and extension,
/// writes the data to it, and returns the path. The file will be automatically
/// deleted when the program exits (handled by tempfile crate).
///
/// # Arguments
///
/// * `data` - The binary data to write
/// * `prefix` - Filename prefix (e.g., "document")
/// * `extension` - File extension including dot (e.g., ".pdf")
///
/// # Returns
///
/// Returns the `PathBuf` to the created temporary file.
///
/// # Errors
///
/// Returns `DoclingError::IoError` if file creation or writing fails.
///
/// # Examples
///
/// ```no_run
/// use docling_backend::utils::write_temp_file;
///
/// let data = b"Hello, world!";
/// let temp_path = write_temp_file(data, "test", ".txt")?;
/// // Use temp_path for processing
/// # Ok::<(), docling_core::DoclingError>(())
/// ```
#[must_use = "ignoring the returned path means the temp file cannot be used"]
pub fn write_temp_file(
    data: &[u8],
    prefix: &str,
    extension: &str,
) -> Result<PathBuf, DoclingError> {
    use std::io::Write;

    // Create temp file with prefix and extension
    let mut temp_file = tempfile::Builder::new()
        .prefix(prefix)
        .suffix(extension)
        .tempfile()?;

    // Write data
    temp_file.write_all(data)?;

    // Get path and persist (keep file until program exit)
    let path = temp_file
        .into_temp_path()
        .keep()
        .map_err(|e| std::io::Error::other(e.to_string()))?;

    Ok(path)
}

/// Create a default provenance item for a given page.
///
/// This helper is used by backends that don't have precise bounding box information.
/// Creates a provenance item with a full-page bounding box (0,0 to 1,1).
///
/// This function consolidates duplicate `create_provenance` implementations that
/// existed across multiple backends (archive, email, ebooks, opendocument).
///
/// # Arguments
///
/// * `page_no` - Page number (1-indexed)
/// * `coord_origin` - Coordinate system origin (`TopLeft` or `BottomLeft`)
///
/// # Returns
///
/// A `ProvenanceItem` with default full-page bounding box and no character span
///
/// # Exampless
///
/// ```no_run
/// use docling_backend::utils::create_default_provenance;
/// use docling_core::content::CoordOrigin;
///
/// // Most backends use TopLeft origin
/// let prov = create_default_provenance(1, CoordOrigin::TopLeft);
/// assert_eq!(prov.page_no, 1);
/// assert_eq!(prov.bbox.l, 0.0);
/// assert_eq!(prov.bbox.r, 1.0);
/// ```
#[inline]
#[must_use = "creates a default provenance item for a page"]
pub const fn create_default_provenance(
    page_no: usize,
    coord_origin: CoordOrigin,
) -> ProvenanceItem {
    ProvenanceItem::default_for_page(page_no, coord_origin)
}

/// Create default provenance metadata as a Vec for `DocItem` creation functions.
///
/// This is a convenience wrapper around `create_default_provenance` that returns
/// the provenance wrapped in a Vec, which is the format expected by `DocItem`
/// creation functions. Uses `TopLeft` coordinate origin.
///
/// # Arguments
///
/// * `page_no` - The page number (1-based)
///
/// # Returns
///
/// A Vec containing a single `ProvenanceItem` with default bounding box (0,0 to 1,1)
///
/// # Exampless
///
/// ```
/// use docling_backend::utils::create_provenance;
/// use docling_core::content::CoordOrigin;
///
/// let prov = create_provenance(1);
/// assert_eq!(prov.len(), 1);
/// assert_eq!(prov[0].page_no, 1);
/// assert_eq!(prov[0].bbox.coord_origin, CoordOrigin::TopLeft);
/// ```
#[inline]
#[must_use = "creates default provenance metadata as a Vec"]
pub fn create_provenance(page_no: usize) -> Vec<ProvenanceItem> {
    vec![create_default_provenance(page_no, CoordOrigin::TopLeft)]
}

/// Format a file size in bytes to a human-readable string with KB or MB units.
///
/// Returns a formatted string like "File Size: 42.5 KB\n\n" or "File Size: 3.2 MB\n\n".
/// This helper reduces code duplication across image format backends that include
/// file size information in their markdown output.
///
/// # Arguments
///
/// * `file_size` - The file size in bytes
///
/// # Returns
///
/// A formatted markdown string with the file size
///
/// # Exampless
///
/// ```no_run
/// use docling_backend::utils::format_file_size;
///
/// let formatted = format_file_size(1024);
/// assert_eq!(formatted, "File Size: 1.0 KB\n\n");
///
/// let formatted = format_file_size(1_048_576);
/// assert_eq!(formatted, "File Size: 1.0 MB\n\n");
/// ```
#[inline]
#[must_use = "formats file size in bytes to human-readable string"]
pub fn format_file_size(file_size: usize) -> String {
    let size_kb = file_size as f64 / BYTES_PER_KB;
    if size_kb < BYTES_PER_KB {
        format!("File Size: {size_kb:.1} KB\n\n")
    } else {
        format!("File Size: {:.1} MB\n\n", size_kb / BYTES_PER_KB)
    }
}

/// Create a simple Text `DocItem` with minimal boilerplate.
///
/// This helper reduces code duplication for the common pattern of creating
/// basic Text `DocItems` with default values. Most backends create many Text
/// `DocItems` with the same structure: no parent, no children, body content layer,
/// no formatting, no hyperlink.
///
/// This consolidates a 10-line `DocItem::Text` construction into a single function call.
///
/// # Arguments
///
/// * `text_index` - The index of this text item (used for `self_ref`: "#/texts/{index}")
/// * `text` - The text content (used for both `text` and `orig` fields)
/// * `provenance` - Provenance items (bounding boxes, page numbers)
///
/// # Returns
///
/// A `DocItem::Text` with common default values
///
/// # Exampless
///
/// ```no_run
/// use docling_backend::utils::create_text_item;
/// use docling_core::content::DocItem;
///
/// let item = create_text_item(0, "Hello World".to_string(), vec![]);
/// match item {
///     DocItem::Text { text, .. } => assert_eq!(text, "Hello World"),
///     _ => panic!("Expected Text item"),
/// }
/// ```
#[must_use = "creates a Text DocItem with standard fields"]
pub fn create_text_item(
    text_index: usize,
    text: String,
    provenance: Vec<ProvenanceItem>,
) -> docling_core::content::DocItem {
    use docling_core::content::DocItem;

    DocItem::Text {
        self_ref: format!("#/texts/{text_index}"),
        parent: None,
        children: vec![],
        content_layer: "body".to_string(),
        prov: provenance,
        orig: text.clone(),
        text,
        formatting: None,
        hyperlink: None,
    }
}

/// Create a Text `DocItem` with a hyperlink
///
/// Same as `create_text_item` but with an optional hyperlink URL.
///
/// # Arguments
///
/// * `text_index` - Index for the `self_ref` field
/// * `text` - The text content
/// * `provenance` - Provenance information
/// * `hyperlink` - Optional hyperlink URL
///
/// # Returns
///
/// A `DocItem::Text` with standard fields and hyperlink populated
#[must_use = "creates a Text DocItem with hyperlink"]
pub fn create_text_item_with_hyperlink(
    text_index: usize,
    text: String,
    provenance: Vec<ProvenanceItem>,
    hyperlink: Option<String>,
) -> docling_core::content::DocItem {
    use docling_core::content::DocItem;

    DocItem::Text {
        self_ref: format!("#/texts/{text_index}"),
        parent: None,
        children: vec![],
        content_layer: "body".to_string(),
        prov: provenance,
        orig: text.clone(),
        text,
        formatting: None,
        hyperlink,
    }
}

/// Create a `Code` `DocItem` with standard fields
///
/// This helper eliminates verbose boilerplate for creating code blocks.
/// All `DocItem::Code` instances follow the same structure with minor variations.
///
/// # Arguments
///
/// * `text_index` - Index for the `self_ref` field (e.g., `#/texts/0`)
/// * `text` - The code content
/// * `language` - Optional programming language identifier (e.g., "rust", "python")
/// * `provenance` - Provenance information (page number, bounding box, etc.)
///
/// # Returns
///
/// A `DocItem::Code` with standard fields populated
///
/// # Examples
///
/// ```no_run
/// use docling_backend::utils::{create_code_item, create_default_provenance};
/// use docling_core::content::CoordOrigin;
///
/// let code = create_code_item(
///     0,
///     "fn main() {}".to_string(),
///     Some("rust".to_string()),
///     vec![create_default_provenance(1, CoordOrigin::TopLeft)],
/// );
/// ```
#[must_use = "creates a Code DocItem with standard fields"]
pub fn create_code_item(
    text_index: usize,
    text: String,
    language: Option<String>,
    provenance: Vec<ProvenanceItem>,
) -> docling_core::content::DocItem {
    use docling_core::content::DocItem;

    DocItem::Code {
        self_ref: format!("#/texts/{text_index}"),
        parent: None,
        children: vec![],
        content_layer: "body".to_string(),
        prov: provenance,
        orig: text.clone(),
        text,
        language,
        formatting: None,
        hyperlink: None,
    }
}

/// Create a `SectionHeader` `DocItem` with standard fields
///
/// This helper eliminates verbose boilerplate for creating section headers.
/// All `DocItem::SectionHeader` instances follow the same structure with minor variations.
///
/// # Arguments
///
/// * `text_index` - Index for the `self_ref` field (e.g., `#/texts/0`)
/// * `text` - The heading text content
/// * `level` - Heading level (1-6, where 1 is top-level like `#` in markdown)
/// * `provenance` - Provenance information (page number, bounding box, etc.)
///
/// # Returns
///
/// A `DocItem::SectionHeader` with standard fields populated
///
/// # Examples
///
/// ```no_run
/// use docling_backend::utils::{create_section_header, create_default_provenance};
/// use docling_core::content::CoordOrigin;
///
/// let header = create_section_header(
///     0,
///     "Introduction".to_string(),
///     1,
///     vec![create_default_provenance(1, CoordOrigin::TopLeft)],
/// );
/// ```
#[must_use = "creates a SectionHeader DocItem with standard fields"]
pub fn create_section_header(
    header_index: usize,
    text: String,
    level: usize,
    provenance: Vec<ProvenanceItem>,
) -> docling_core::content::DocItem {
    use docling_core::content::DocItem;

    DocItem::SectionHeader {
        self_ref: format!("#/headers/{header_index}"),
        parent: None,
        children: vec![],
        content_layer: "body".to_string(),
        prov: provenance,
        orig: text.clone(),
        text,
        level,
        formatting: None,
        hyperlink: None,
    }
}

/// Create a `SectionHeader` `DocItem` with a hyperlink
///
/// Same as `create_section_header` but with an optional hyperlink URL.
#[must_use = "creates a SectionHeader DocItem with hyperlink"]
pub fn create_section_header_with_hyperlink(
    header_index: usize,
    text: String,
    level: usize,
    provenance: Vec<ProvenanceItem>,
    hyperlink: Option<String>,
) -> docling_core::content::DocItem {
    use docling_core::content::DocItem;

    DocItem::SectionHeader {
        self_ref: format!("#/headers/{header_index}"),
        parent: None,
        children: vec![],
        content_layer: "body".to_string(),
        prov: provenance,
        orig: text.clone(),
        text,
        level,
        formatting: None,
        hyperlink,
    }
}

/// Create a `ListItem` `DocItem` with standard fields
///
/// This helper eliminates verbose boilerplate for creating list items.
/// All `DocItem::ListItem` instances follow the same structure with minor variations.
///
/// # Arguments
///
/// * `text_index` - Index for the `self_ref` field (e.g., `#/texts/0`)
/// * `text` - The list item text content
/// * `marker` - List marker string (e.g., "- ", "1. ", "* ")
/// * `enumerated` - Whether this is an enumerated (numbered) list
/// * `provenance` - Provenance information (page number, bounding box, etc.)
/// * `parent` - Optional parent reference (for nested lists, use Some(ItemRef))
///
/// # Returns
///
/// A `DocItem::ListItem` with standard fields populated
///
/// # Examples
///
/// ```no_run
/// use docling_backend::utils::{create_list_item, create_default_provenance};
/// use docling_core::content::CoordOrigin;
///
/// // Simple unordered list item
/// let item = create_list_item(
///     0,
///     "First item".to_string(),
///     "- ".to_string(),
///     false,
///     vec![create_default_provenance(1, CoordOrigin::TopLeft)],
///     None,
/// );
///
/// // Ordered list item
/// let item = create_list_item(
///     1,
///     "First step".to_string(),
///     "1. ".to_string(),
///     true,
///     vec![create_default_provenance(1, CoordOrigin::TopLeft)],
///     None,
/// );
/// ```
#[must_use = "creates a ListItem DocItem with standard fields"]
pub fn create_list_item(
    text_index: usize,
    text: String,
    marker: String,
    enumerated: bool,
    provenance: Vec<ProvenanceItem>,
    parent: Option<docling_core::content::ItemRef>,
) -> docling_core::content::DocItem {
    use docling_core::content::DocItem;

    DocItem::ListItem {
        self_ref: format!("#/texts/{text_index}"),
        parent,
        children: vec![],
        content_layer: "body".to_string(),
        prov: provenance,
        orig: text.clone(),
        text,
        marker,
        enumerated,
        formatting: None,
        hyperlink: None,
    }
}

/// Create a `ListItem` `DocItem` with a hyperlink
///
/// Same as `create_list_item` but with an optional hyperlink URL.
#[must_use = "creates a ListItem DocItem with hyperlink"]
pub fn create_list_item_with_hyperlink(
    text_index: usize,
    text: String,
    marker: String,
    enumerated: bool,
    provenance: Vec<ProvenanceItem>,
    parent: Option<docling_core::content::ItemRef>,
    hyperlink: Option<String>,
) -> docling_core::content::DocItem {
    use docling_core::content::DocItem;

    DocItem::ListItem {
        self_ref: format!("#/texts/{text_index}"),
        parent,
        children: vec![],
        content_layer: "body".to_string(),
        prov: provenance,
        orig: text.clone(),
        text,
        marker,
        enumerated,
        formatting: None,
        hyperlink,
    }
}

/// Create a `Code` `DocItem` with standard fields
///
/// # Arguments
///
/// * `code_index` - The index for this code block (used in `self_ref`)
/// * `text` - The code content
/// * `language` - Optional language identifier (e.g., "rust", "python", "vcard")
/// * `provenance` - Provenance information for the code block
///
/// # Returns
///
/// A `DocItem::Code` variant with the given content and metadata
#[must_use = "creates a Code DocItem for code blocks"]
pub fn create_code_block(
    code_index: usize,
    text: String,
    language: Option<String>,
    provenance: Vec<ProvenanceItem>,
) -> docling_core::content::DocItem {
    use docling_core::content::DocItem;

    DocItem::Code {
        self_ref: format!("#/code/{code_index}"),
        parent: None,
        children: vec![],
        content_layer: "body".to_string(),
        prov: provenance,
        orig: text.clone(),
        text,
        language,
        formatting: None,
        hyperlink: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_minimal_document() {
        let markdown = "# Test\n\nContent here.".to_string();
        let doc = build_minimal_document(markdown.clone(), InputFormat::Md);

        assert_eq!(doc.markdown, markdown);
        assert_eq!(doc.format, InputFormat::Md);
        assert_eq!(doc.metadata.num_characters, 21);
        assert_eq!(doc.metadata.title, None);
        assert!(doc.content_blocks.is_none());
    }

    #[test]
    fn test_build_document_with_title() {
        let markdown = "Content".to_string();
        let title = Some("Test Title".to_string());
        let doc = build_document_with_title(markdown.clone(), InputFormat::Docx, title.clone());

        assert_eq!(doc.markdown, markdown);
        assert_eq!(doc.format, InputFormat::Docx);
        assert_eq!(doc.metadata.title, title);
        assert_eq!(doc.metadata.num_characters, 7);
    }

    #[test]
    fn test_file_path_required_error() {
        let error = file_path_required_error(InputFormat::Epub, "ZIP archive");
        let msg = error.to_string();

        assert!(msg.contains("Epub"));
        assert!(msg.contains("ZIP archive"));
        assert!(msg.contains("parse_file()"));
    }

    #[test]
    fn test_backend_error() {
        let error = backend_error("parse", "EPUB", "invalid header");
        let msg = error.to_string();

        assert!(msg.contains("Failed to parse EPUB"));
        assert!(msg.contains("invalid header"));
    }

    #[test]
    fn test_add_document_title() {
        let mut md = String::new();
        add_document_title(&mut md, "Test Document");
        assert_eq!(md, "# Test Document\n\n");
    }

    #[test]
    fn test_add_metadata_block_empty() {
        let mut md = String::new();
        add_metadata_block(&mut md, &[]);
        assert_eq!(md, "");
    }

    #[test]
    fn test_add_metadata_block_with_data() {
        let mut md = String::new();
        add_metadata_block(&mut md, &[("Title", "Doc"), ("Author", "Alice")]);

        assert!(md.starts_with("---\n"));
        assert!(md.contains("**Title:** Doc\n"));
        assert!(md.contains("**Author:** Alice\n"));
        assert!(md.ends_with("---\n\n"));
    }

    #[test]
    fn test_parse_file_with_special_handling_special_format() {
        use std::path::Path;

        let result = parse_file_with_special_handling(
            Path::new("test.txt"),
            InputFormat::Msg,
            &[InputFormat::Msg],
            |_path| Ok("Special handling".to_string()),
            |_bytes, _opts| panic!("Should not call fallback"),
            &BackendOptions::default(),
        );

        assert!(result.is_ok());
        let doc = result.unwrap();
        assert_eq!(doc.markdown, "Special handling");
        assert_eq!(doc.format, InputFormat::Msg);
    }

    #[test]
    fn test_parse_file_with_special_handling_fallback() {
        use std::fs;

        // Create a temporary test file
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_utils_fallback.txt");
        fs::write(&test_file, b"test content").unwrap();

        let result = parse_file_with_special_handling(
            &test_file,
            InputFormat::Md,
            &[InputFormat::Msg], // Md not in special formats
            |_path| panic!("Should not call special handler"),
            |bytes, _opts| {
                assert_eq!(bytes, b"test content");
                Ok(build_minimal_document(
                    "Fallback result".to_string(),
                    InputFormat::Md,
                ))
            },
            &BackendOptions::default(),
        );

        // Cleanup
        let _ = fs::remove_file(&test_file);

        assert!(result.is_ok());
        let doc = result.unwrap();
        assert_eq!(doc.markdown, "Fallback result");
    }

    #[test]
    fn test_create_default_provenance_top_left() {
        let prov = create_default_provenance(1, CoordOrigin::TopLeft);
        assert_eq!(prov.page_no, 1);
        assert_eq!(prov.bbox.l, 0.0);
        assert_eq!(prov.bbox.t, 0.0);
        assert_eq!(prov.bbox.r, 1.0);
        assert_eq!(prov.bbox.b, 1.0);
        assert_eq!(prov.bbox.coord_origin, CoordOrigin::TopLeft);
        assert_eq!(prov.charspan, None);
    }

    #[test]
    fn test_create_default_provenance_bottom_left() {
        let prov = create_default_provenance(2, CoordOrigin::BottomLeft);
        assert_eq!(prov.page_no, 2);
        assert_eq!(prov.bbox.coord_origin, CoordOrigin::BottomLeft);
    }

    #[test]
    fn test_format_file_size_kb() {
        let formatted = format_file_size(1024);
        assert_eq!(formatted, "File Size: 1.0 KB\n\n");

        let formatted = format_file_size(512);
        assert_eq!(formatted, "File Size: 0.5 KB\n\n");
    }

    #[test]
    fn test_format_file_size_mb() {
        let formatted = format_file_size(1_048_576); // 1 MB
        assert_eq!(formatted, "File Size: 1.0 MB\n\n");

        let formatted = format_file_size(2_621_440); // 2.5 MB
        assert_eq!(formatted, "File Size: 2.5 MB\n\n");
    }

    #[test]
    fn test_create_text_item() {
        use docling_core::content::DocItem;

        let item = create_text_item(0, "Hello World".to_string(), vec![]);

        match item {
            DocItem::Text {
                self_ref,
                parent,
                children,
                content_layer,
                prov,
                orig,
                text,
                formatting,
                hyperlink,
            } => {
                assert_eq!(self_ref, "#/texts/0");
                assert_eq!(parent, None);
                assert_eq!(children, vec![]);
                assert_eq!(content_layer, "body");
                assert_eq!(prov, vec![]);
                assert_eq!(orig, "Hello World");
                assert_eq!(text, "Hello World");
                assert_eq!(formatting, None);
                assert_eq!(hyperlink, None);
            }
            _ => panic!("Expected Text item"),
        }
    }

    #[test]
    fn test_create_text_item_with_provenance() {
        use docling_core::content::DocItem;

        let prov = vec![create_default_provenance(1, CoordOrigin::TopLeft)];
        let item = create_text_item(5, "Test content".to_string(), prov.clone());

        match item {
            DocItem::Text {
                self_ref,
                prov: item_prov,
                ..
            } => {
                assert_eq!(self_ref, "#/texts/5");
                assert_eq!(item_prov.len(), 1);
                assert_eq!(item_prov[0].page_no, 1);
            }
            _ => panic!("Expected Text item"),
        }
    }

    // ========== Document Building Edge Cases ==========

    #[test]
    fn test_build_minimal_document_empty() {
        let doc = build_minimal_document(String::new(), InputFormat::Csv);
        assert_eq!(doc.markdown, "");
        assert_eq!(doc.metadata.num_characters, 0);
        assert_eq!(doc.format, InputFormat::Csv);
    }

    #[test]
    fn test_build_document_with_unicode_title() {
        let markdown = "Content".to_string();
        let title = Some("文档标题 📄".to_string());
        let doc = build_document_with_title(markdown, InputFormat::Docx, title.clone());

        assert_eq!(doc.metadata.title, title);
        assert_eq!(doc.metadata.num_characters, 7);
    }

    #[test]
    fn test_build_document_with_very_long_title() {
        let long_title = Some("A".repeat(1000));
        let doc =
            build_document_with_title("Content".to_string(), InputFormat::Md, long_title.clone());

        assert_eq!(doc.metadata.title, long_title);
    }

    // ========== Error Message Tests ==========

    #[test]
    fn test_file_path_required_error_different_formats() {
        let error1 = file_path_required_error(InputFormat::Zip, "archive");
        let error2 = file_path_required_error(InputFormat::Tar, "compressed file");

        assert!(error1.to_string().contains("Zip"));
        assert!(error1.to_string().contains("archive"));
        assert!(error2.to_string().contains("Tar"));
        assert!(error2.to_string().contains("compressed file"));
    }

    #[test]
    fn test_backend_error_complex_types() {
        // Test with different error types
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let error = backend_error("extract", "DOCX", io_error);
        assert!(error.to_string().contains("Failed to extract DOCX"));
        assert!(error.to_string().contains("file missing"));
    }

    // ========== Markdown Formatting Edge Cases ==========

    #[test]
    fn test_add_document_title_empty() {
        let mut md = String::new();
        add_document_title(&mut md, "");
        assert_eq!(md, "# \n\n");
    }

    #[test]
    fn test_add_document_title_special_characters() {
        let mut md = String::new();
        add_document_title(&mut md, "Title with <html> & \"quotes\"");
        assert_eq!(md, "# Title with <html> & \"quotes\"\n\n");
    }

    #[test]
    fn test_add_metadata_block_long_values() {
        let mut md = String::new();
        let long_value = "A".repeat(500);
        add_metadata_block(&mut md, &[("Title", "Short"), ("Description", &long_value)]);

        assert!(md.contains("**Title:** Short\n"));
        assert!(md.contains(&long_value));
        assert!(md.starts_with("---\n"));
        assert!(md.ends_with("---\n\n"));
    }

    // ========== File Size Formatting Edge Cases ==========

    #[test]
    fn test_format_file_size_boundary() {
        // Exactly 1024 KB (1 MB)
        let formatted = format_file_size(1_048_576);
        assert!(formatted.contains("1.0 MB"));
    }

    #[test]
    fn test_format_file_size_very_large() {
        // 5 GB
        let formatted = format_file_size(5_368_709_120);
        assert!(formatted.contains("MB"));
        // 5 GB = 5120 MB
        assert!(formatted.contains("5120.0 MB"));
    }

    // ========== DocItem Helper Tests ==========

    #[test]
    fn test_create_section_header() {
        use docling_core::content::DocItem;

        let prov = vec![create_default_provenance(1, CoordOrigin::TopLeft)];
        let header = create_section_header(0, "Introduction".to_string(), 1, prov.clone());

        match header {
            DocItem::SectionHeader {
                self_ref,
                text,
                level,
                parent,
                children,
                content_layer,
                ..
            } => {
                assert_eq!(self_ref, "#/headers/0");
                assert_eq!(text, "Introduction");
                assert_eq!(level, 1);
                assert_eq!(parent, None);
                assert_eq!(children, vec![]);
                assert_eq!(content_layer, "body");
            }
            _ => panic!("Expected SectionHeader"),
        }
    }

    #[test]
    fn test_create_section_header_level_variations() {
        use docling_core::content::DocItem;

        // Test different heading levels (1-6)
        for level in 1..=6 {
            let header = create_section_header(level - 1, format!("Level {level}"), level, vec![]);

            match header {
                DocItem::SectionHeader { level: l, .. } => {
                    assert_eq!(l, level);
                }
                _ => panic!("Expected SectionHeader"),
            }
        }
    }

    #[test]
    fn test_create_list_item_unordered() {
        use docling_core::content::DocItem;

        let item = create_list_item(
            0,
            "First item".to_string(),
            "- ".to_string(),
            false,
            vec![],
            None,
        );

        match item {
            DocItem::ListItem {
                self_ref,
                text,
                marker,
                enumerated,
                parent,
                ..
            } => {
                assert_eq!(self_ref, "#/texts/0");
                assert_eq!(text, "First item");
                assert_eq!(marker, "- ");
                assert!(!enumerated);
                assert_eq!(parent, None);
            }
            _ => panic!("Expected ListItem"),
        }
    }

    #[test]
    fn test_create_list_item_ordered() {
        use docling_core::content::DocItem;

        let item = create_list_item(
            1,
            "First step".to_string(),
            "1. ".to_string(),
            true,
            vec![],
            None,
        );

        match item {
            DocItem::ListItem {
                marker,
                enumerated,
                text,
                ..
            } => {
                assert_eq!(marker, "1. ");
                assert!(enumerated);
                assert_eq!(text, "First step");
            }
            _ => panic!("Expected ListItem"),
        }
    }

    // ========== Provenance Edge Cases ==========

    #[test]
    fn test_create_default_provenance_page_zero() {
        // Some backends may use 0-indexed pages
        let prov = create_default_provenance(0, CoordOrigin::TopLeft);
        assert_eq!(prov.page_no, 0);
        assert_eq!(prov.bbox.l, 0.0);
        assert_eq!(prov.bbox.r, 1.0);
    }

    #[test]
    fn test_create_default_provenance_large_page() {
        // Test very large page numbers (e.g., 10000-page document)
        let prov = create_default_provenance(10000, CoordOrigin::BottomLeft);
        assert_eq!(prov.page_no, 10000);
        assert_eq!(prov.bbox.coord_origin, CoordOrigin::BottomLeft);
    }

    // ========== Integration/Complex Tests ==========

    #[test]
    fn test_combined_markdown_formatting() {
        // Test combining multiple markdown helpers
        let mut md = String::new();
        add_document_title(&mut md, "My Document");
        add_metadata_block(&mut md, &[("Author", "Alice"), ("Date", "2025-01-13")]);

        assert!(md.starts_with("# My Document\n\n"));
        assert!(md.contains("---\n"));
        assert!(md.contains("**Author:** Alice\n"));
        assert!(md.contains("**Date:** 2025-01-13\n"));
        assert!(md.contains("---\n\n"));
    }

    #[test]
    fn test_parse_file_with_special_handling_read_error() {
        use std::path::Path;

        // Try to read non-existent file (fallback path)
        let result = parse_file_with_special_handling(
            Path::new("/nonexistent/file.txt"),
            InputFormat::Md,
            &[InputFormat::Msg], // Md not special, so will try to read file
            |_path| panic!("Should not call special handler"),
            |_bytes, _opts| panic!("Should not reach fallback handler"),
            &BackendOptions::default(),
        );

        // Should fail with IO error
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("IO error"));
    }

    // ========== write_temp_file() Tests ==========

    #[test]
    fn test_write_temp_file_basic() {
        let data = b"Hello, temporary file!";
        let result = write_temp_file(data, "test", ".txt");

        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.exists());
        assert!(path.to_string_lossy().contains("test"));
        assert!(path.to_string_lossy().ends_with(".txt"));

        // Verify content
        let content = std::fs::read(&path).unwrap();
        assert_eq!(content, data);

        // Cleanup
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_write_temp_file_empty() {
        let data = b"";
        let result = write_temp_file(data, "empty", ".bin");

        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.exists());

        // Verify empty file
        let content = std::fs::read(&path).unwrap();
        assert_eq!(content.len(), 0);

        // Cleanup
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_write_temp_file_large() {
        // Create 1 MB of data
        let data = vec![0xAB; 1_048_576];
        let result = write_temp_file(&data, "large", ".dat");

        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.exists());

        // Verify size
        let metadata = std::fs::metadata(&path).unwrap();
        assert_eq!(metadata.len(), 1_048_576);

        // Cleanup
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_write_temp_file_binary_data() {
        // Binary data with all byte values
        let data: Vec<u8> = (0..=255).collect();
        let result = write_temp_file(&data, "binary", ".bin");

        assert!(result.is_ok());
        let path = result.unwrap();

        // Verify exact binary content
        let content = std::fs::read(&path).unwrap();
        assert_eq!(content, data);

        // Cleanup
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_write_temp_file_unicode_content() {
        let data = "Hello 世界 🌍".as_bytes();
        let result = write_temp_file(data, "unicode", ".txt");

        assert!(result.is_ok());
        let path = result.unwrap();

        // Verify UTF-8 content
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, "Hello 世界 🌍");

        // Cleanup
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_write_temp_file_different_extensions() {
        for (ext, data) in [
            (".pdf", b"PDF data" as &[u8]),
            (".docx", b"DOCX data"),
            (".json", b"{\"key\": \"value\"}"),
            (".xml", b"<root></root>"),
        ] {
            let result = write_temp_file(data, "test", ext);
            assert!(result.is_ok());
            let path = result.unwrap();
            assert!(path.to_string_lossy().ends_with(ext));
            let _ = std::fs::remove_file(path);
        }
    }

    // ========== Unicode and Character Counting Edge Cases ==========

    #[test]
    fn test_build_minimal_document_multibyte_chars() {
        // Test character counting with multibyte UTF-8 characters
        let markdown = "Hello 世界 🌍".to_string(); // "Hello " (6) + "世" (1) + "界" (1) + " " (1) + "🌍" (1) = 10 chars
        let doc = build_minimal_document(markdown.clone(), InputFormat::Md);

        // Should count characters, not bytes
        assert_eq!(doc.metadata.num_characters, 10);
    }

    #[test]
    fn test_build_minimal_document_only_emoji() {
        let markdown = "🚀🌟💡🎉".to_string(); // 4 emojis
        let doc = build_minimal_document(markdown.clone(), InputFormat::Md);

        assert_eq!(doc.metadata.num_characters, 4);
        assert_eq!(doc.markdown, "🚀🌟💡🎉");
    }

    #[test]
    fn test_build_minimal_document_combining_characters() {
        // Combining diacritics (é = e + combining acute)
        let markdown = "Café".to_string(); // May be 4 or 5 chars depending on normalization
        let doc = build_minimal_document(markdown.clone(), InputFormat::Md);

        // Just verify it doesn't panic and counts reasonably
        assert!(doc.metadata.num_characters >= 4);
        assert!(doc.metadata.num_characters <= 5);
    }

    #[test]
    fn test_build_document_with_title_empty_title() {
        let doc = build_document_with_title(
            "Content".to_string(),
            InputFormat::Docx,
            Some("".to_string()),
        );

        assert_eq!(doc.metadata.title, Some("".to_string()));
    }

    #[test]
    fn test_build_document_with_title_none() {
        let doc = build_document_with_title("Content".to_string(), InputFormat::Html, None);

        assert_eq!(doc.metadata.title, None);
    }

    // ========== Metadata Block Edge Cases ==========

    #[test]
    fn test_add_metadata_block_single_entry() {
        let mut md = String::new();
        add_metadata_block(&mut md, &[("Title", "Single Entry")]);

        assert!(md.starts_with("---\n"));
        assert!(md.contains("**Title:** Single Entry\n"));
        assert!(md.ends_with("---\n\n"));
    }

    #[test]
    fn test_add_metadata_block_special_characters_in_values() {
        let mut md = String::new();
        add_metadata_block(
            &mut md,
            &[
                ("Title", "Doc with <html> & \"quotes\""),
                ("Path", "/usr/local/bin"),
            ],
        );

        // Special characters should be preserved as-is
        assert!(md.contains("**Title:** Doc with <html> & \"quotes\"\n"));
        assert!(md.contains("**Path:** /usr/local/bin\n"));
    }

    #[test]
    fn test_add_metadata_block_unicode_values() {
        let mut md = String::new();
        add_metadata_block(&mut md, &[("Author", "张伟"), ("Title", "文档 📄")]);

        assert!(md.contains("**Author:** 张伟\n"));
        assert!(md.contains("**Title:** 文档 📄\n"));
    }

    #[test]
    fn test_add_metadata_block_newlines_in_values() {
        let mut md = String::new();
        add_metadata_block(&mut md, &[("Description", "Line 1\nLine 2\nLine 3")]);

        // Newlines in values should be preserved
        assert!(md.contains("**Description:** Line 1\nLine 2\nLine 3\n"));
    }

    // ========== List Item Edge Cases ==========

    #[test]
    fn test_create_list_item_with_parent() {
        use docling_core::content::{DocItem, ItemRef};

        let parent_ref = ItemRef {
            ref_path: "#/texts/0".to_string(),
        };

        let item = create_list_item(
            1,
            "Nested item".to_string(),
            "  - ".to_string(),
            false,
            vec![],
            Some(parent_ref.clone()),
        );

        match item {
            DocItem::ListItem { parent, marker, .. } => {
                assert_eq!(parent, Some(parent_ref));
                assert_eq!(marker, "  - ");
            }
            _ => panic!("Expected ListItem"),
        }
    }

    #[test]
    fn test_create_list_item_different_markers() {
        use docling_core::content::DocItem;

        for (marker, expected_marker) in [
            ("- ", "- "),
            ("* ", "* "),
            ("+ ", "+ "),
            ("1. ", "1. "),
            ("42. ", "42. "),
        ] {
            let item = create_list_item(
                0,
                "Item".to_string(),
                marker.to_string(),
                false,
                vec![],
                None,
            );

            match item {
                DocItem::ListItem { marker: m, .. } => {
                    assert_eq!(m, expected_marker);
                }
                _ => panic!("Expected ListItem"),
            }
        }
    }

    #[test]
    fn test_create_list_item_long_text() {
        use docling_core::content::DocItem;

        let long_text = "A".repeat(10000);
        let item = create_list_item(0, long_text.clone(), "- ".to_string(), false, vec![], None);

        match item {
            DocItem::ListItem { text, orig, .. } => {
                assert_eq!(text.len(), 10000);
                assert_eq!(orig.len(), 10000);
                assert_eq!(text, long_text);
            }
            _ => panic!("Expected ListItem"),
        }
    }

    // ========== File Size Formatting Edge Cases ==========

    #[test]
    fn test_format_file_size_zero() {
        let formatted = format_file_size(0);
        assert_eq!(formatted, "File Size: 0.0 KB\n\n");
    }

    #[test]
    fn test_format_file_size_one_byte() {
        let formatted = format_file_size(1);
        // 1 byte = 0.0009765625 KB, rounds to 0.0
        assert!(formatted.contains("0.0 KB"));
    }

    #[test]
    fn test_format_file_size_kb_mb_boundary() {
        // Just under 1 MB (1023 KB)
        let formatted = format_file_size(1_047_552); // 1023 KB
        assert!(formatted.contains("1023.0 KB"));

        // Just over 1 MB (1025 KB)
        let formatted = format_file_size(1_049_600); // 1025 KB
        assert!(formatted.contains("1.0 MB"));
    }

    // ========== Error Message Formatting Edge Cases ==========

    #[test]
    fn test_backend_error_empty_strings() {
        let error = backend_error("", "", "test error");
        let msg = error.to_string();

        // Should still format properly even with empty strings
        assert!(msg.contains("Failed to"));
        assert!(msg.contains("test error"));
    }

    #[test]
    fn test_backend_error_unicode() {
        let error = backend_error("解析", "文档", "错误信息");
        let msg = error.to_string();

        assert!(msg.contains("Failed to 解析 文档"));
        assert!(msg.contains("错误信息"));
    }

    // ========== Document Title Edge Cases ==========

    #[test]
    fn test_add_document_title_unicode() {
        let mut md = String::new();
        add_document_title(&mut md, "文档标题 📄");
        assert_eq!(md, "# 文档标题 📄\n\n");
    }

    #[test]
    fn test_add_document_title_newlines() {
        let mut md = String::new();
        add_document_title(&mut md, "Title\nWith\nNewlines");

        // Newlines should be preserved
        assert_eq!(md, "# Title\nWith\nNewlines\n\n");
    }

    #[test]
    fn test_add_document_title_very_long() {
        let mut md = String::new();
        let long_title = "A".repeat(1000);
        add_document_title(&mut md, &long_title);

        assert!(md.starts_with("# A"));
        assert!(md.ends_with("A\n\n"));
        assert_eq!(md.len(), 1000 + 4); // "# " + title + "\n\n"
    }

    // ========== DocItem Helpers - Additional Tests ==========

    #[test]
    fn test_create_text_item_different_indices() {
        use docling_core::content::DocItem;

        for i in [0, 1, 10, 100, 999] {
            let item = create_text_item(i, "Text".to_string(), vec![]);
            match item {
                DocItem::Text { self_ref, .. } => {
                    assert_eq!(self_ref, format!("#/texts/{i}"));
                }
                _ => panic!("Expected Text item"),
            }
        }
    }

    #[test]
    fn test_create_text_item_unicode_text() {
        use docling_core::content::DocItem;

        let item = create_text_item(0, "Hello 世界 🌍".to_string(), vec![]);

        match item {
            DocItem::Text { text, orig, .. } => {
                assert_eq!(text, "Hello 世界 🌍");
                assert_eq!(orig, "Hello 世界 🌍");
            }
            _ => panic!("Expected Text item"),
        }
    }

    #[test]
    fn test_create_section_header_empty_text() {
        use docling_core::content::DocItem;

        let header = create_section_header(0, "".to_string(), 1, vec![]);

        match header {
            DocItem::SectionHeader { text, orig, .. } => {
                assert_eq!(text, "");
                assert_eq!(orig, "");
            }
            _ => panic!("Expected SectionHeader"),
        }
    }

    #[test]
    fn test_create_section_header_large_level() {
        use docling_core::content::DocItem;

        // Test large heading levels (beyond typical 1-6)
        let header = create_section_header(0, "Deep heading".to_string(), 99, vec![]);

        match header {
            DocItem::SectionHeader { level, .. } => {
                assert_eq!(level, 99);
            }
            _ => panic!("Expected SectionHeader"),
        }
    }

    #[test]
    fn test_create_text_item_with_multiple_provenances() {
        use docling_core::content::{DocItem, ProvenanceItem};
        use docling_core::{BoundingBox, CoordOrigin};

        let prov1 = ProvenanceItem {
            page_no: 1,
            bbox: BoundingBox::new(0.0, 0.0, 0.0, 0.0, CoordOrigin::BottomLeft),
            charspan: None,
        };
        let prov2 = ProvenanceItem {
            page_no: 2,
            bbox: BoundingBox::new(0.0, 0.0, 0.0, 0.0, CoordOrigin::BottomLeft),
            charspan: None,
        };
        let prov3 = ProvenanceItem {
            page_no: 3,
            bbox: BoundingBox::new(0.0, 0.0, 0.0, 0.0, CoordOrigin::BottomLeft),
            charspan: None,
        };

        let item = create_text_item(0, "Multi-page text".to_string(), vec![prov1, prov2, prov3]);

        match item {
            DocItem::Text { prov, .. } => {
                assert_eq!(prov.len(), 3);
                assert_eq!(prov[0].page_no, 1);
                assert_eq!(prov[1].page_no, 2);
                assert_eq!(prov[2].page_no, 3);
            }
            _ => panic!("Expected Text item"),
        }
    }

    #[test]
    fn test_create_section_header_with_special_characters() {
        use docling_core::content::DocItem;

        // Test section headers with various special characters
        let special_text = "Section: Test & Verify [Part #1] <Draft>";
        let header = create_section_header(0, special_text.to_string(), 2, vec![]);

        match header {
            DocItem::SectionHeader { text, orig, .. } => {
                assert_eq!(text, special_text);
                assert_eq!(orig, special_text);
                assert!(text.contains('&'));
                assert!(text.contains('['));
                assert!(text.contains('#'));
                assert!(text.contains('<'));
            }
            _ => panic!("Expected SectionHeader"),
        }
    }

    // ===== N=603 Expansion: 5 additional tests to reach 70 =====

    #[test]
    fn test_format_file_size_fractional_kb() {
        // Test fractional KB values (between 0.1 and 1.0 KB)
        let formatted = format_file_size(256); // 0.25 KB
        assert!(formatted.contains("0.2 KB") || formatted.contains("0.3 KB"));

        let formatted = format_file_size(768); // 0.75 KB
        assert!(formatted.contains("0.7 KB") || formatted.contains("0.8 KB"));
    }

    #[test]
    fn test_format_file_size_fractional_mb() {
        // Test fractional MB values (between 1.1 and 9.9 MB)
        let formatted = format_file_size(1_572_864); // 1.5 MB
        assert!(formatted.contains("1.5 MB"));

        let formatted = format_file_size(7_340_032); // 7.0 MB
        assert!(formatted.contains("7.0 MB"));
    }

    #[test]
    fn test_build_document_with_all_metadata() {
        // Test document with comprehensive metadata
        let markdown = "Document content with lots of text here.".to_string();
        let title = Some("Complete Test Document".to_string());
        let doc = build_document_with_title(markdown.clone(), InputFormat::Html, title.clone());

        assert_eq!(doc.markdown, markdown);
        assert_eq!(doc.metadata.title, title);
        assert_eq!(doc.metadata.num_characters, markdown.len());
        assert_eq!(doc.format, InputFormat::Html);
        assert!(doc.content_blocks.is_none()); // Minimal document has no DocItems
    }

    #[test]
    fn test_file_path_required_error_message_format() {
        // Test that error messages have consistent format and helpful information
        let error = file_path_required_error(InputFormat::Tar, "compressed archive");
        let msg = error.to_string();

        // Should contain format name
        assert!(msg.contains("Tar"));
        // Should contain description
        assert!(msg.contains("compressed archive"));
        // Should suggest parse_file
        assert!(msg.contains("parse_file()"));
        // Should explain bytes limitation
        assert!(msg.contains("parse_bytes"));
    }

    #[test]
    fn test_backend_error_with_nested_error() {
        // Test error chaining with nested errors
        let inner_error = std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "Access denied to file",
        );
        let error = backend_error("read", "XLSX", inner_error);
        let msg = error.to_string();

        // Should contain operation
        assert!(msg.contains("Failed to read XLSX"));
        // Should contain nested error details
        assert!(msg.contains("Access denied") || msg.contains("Permission denied"));
    }

    // ========== Additional Edge Cases (N=646) ==========

    #[test]
    fn test_create_list_item_nested_with_parent() {
        // Test nested list items with explicit parent reference
        use docling_core::content::{DocItem, ItemRef};

        let parent_ref = ItemRef::new("#/texts/0");
        let item = create_list_item(
            1,
            "Nested item".to_string(),
            "  - ".to_string(),
            false,
            vec![],
            Some(parent_ref.clone()),
        );

        match item {
            DocItem::ListItem { parent, .. } => {
                assert_eq!(parent, Some(parent_ref));
            }
            _ => panic!("Expected ListItem"),
        }
    }

    #[test]
    fn test_create_default_provenance_coord_origin_variants() {
        // Test both CoordOrigin variants to ensure proper handling
        let origins = vec![CoordOrigin::TopLeft, CoordOrigin::BottomLeft];

        for origin in origins {
            let prov = create_default_provenance(1, origin);
            assert_eq!(prov.bbox.coord_origin, origin);
            assert_eq!(prov.bbox.l, 0.0);
            assert_eq!(prov.bbox.t, 0.0);
            assert_eq!(prov.bbox.r, 1.0);
            assert_eq!(prov.bbox.b, 1.0);
        }
    }

    #[test]
    fn test_add_metadata_block_empty_keys() {
        // Test metadata block with empty key strings (edge case)
        let mut md = String::new();
        add_metadata_block(&mut md, &[("", "Value1"), ("Key2", "")]);

        // Should still create valid markdown (empty keys may be filtered or shown as-is)
        assert!(md.starts_with("---\n"));
        assert!(md.ends_with("---\n\n"));
        // Empty key should be rendered as "**:** Value1"
        assert!(md.contains("**:**"));
    }

    #[test]
    fn test_format_file_size_gigabyte_range() {
        // Test file sizes in gigabyte range (current implementation shows as MB)
        let size_1gb = 1_073_741_824; // 1 GB = 1024 MB
        let formatted = format_file_size(size_1gb);
        assert!(formatted.contains("1024.0 MB"));

        let size_10gb = 10_737_418_240; // 10 GB = 10240 MB
        let formatted = format_file_size(size_10gb);
        assert!(formatted.contains("10240.0 MB"));
    }

    #[test]
    fn test_parse_file_with_special_handling_nonexistent_file() {
        // Test error handling when file doesn't exist
        use std::path::Path;

        let result = parse_file_with_special_handling(
            Path::new("/nonexistent/path/to/file.txt"),
            InputFormat::Md,
            &[], // No special formats
            |_path| panic!("Should not call special handler"),
            |_bytes, _opts| panic!("Should not call fallback"),
            &BackendOptions::default(),
        );

        // Should return an error (file not found)
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("No such file")
                || error_msg.contains("not found")
                || error_msg.contains("cannot find")
        );
    }

    // ========== MIME Type Detection Tests (N=4114) ==========

    #[test]
    fn test_mime_type_from_extension_png() {
        assert_eq!(
            mime_type_from_extension("png", MIME_OCTET_STREAM),
            MIME_IMAGE_PNG
        );
        assert_eq!(
            mime_type_from_extension("PNG", MIME_OCTET_STREAM),
            MIME_IMAGE_PNG
        );
        assert_eq!(
            mime_type_from_extension("Png", MIME_OCTET_STREAM),
            MIME_IMAGE_PNG
        );
    }

    #[test]
    fn test_mime_type_from_extension_jpeg() {
        assert_eq!(
            mime_type_from_extension("jpg", MIME_OCTET_STREAM),
            MIME_IMAGE_JPEG
        );
        assert_eq!(
            mime_type_from_extension("jpeg", MIME_OCTET_STREAM),
            MIME_IMAGE_JPEG
        );
        assert_eq!(
            mime_type_from_extension("JPG", MIME_OCTET_STREAM),
            MIME_IMAGE_JPEG
        );
        assert_eq!(
            mime_type_from_extension("JPEG", MIME_OCTET_STREAM),
            MIME_IMAGE_JPEG
        );
    }

    #[test]
    fn test_mime_type_from_extension_gif() {
        assert_eq!(
            mime_type_from_extension("gif", MIME_OCTET_STREAM),
            MIME_IMAGE_GIF
        );
        assert_eq!(
            mime_type_from_extension("GIF", MIME_OCTET_STREAM),
            MIME_IMAGE_GIF
        );
    }

    #[test]
    fn test_mime_type_from_extension_bmp() {
        assert_eq!(
            mime_type_from_extension("bmp", MIME_OCTET_STREAM),
            MIME_IMAGE_BMP
        );
        assert_eq!(
            mime_type_from_extension("BMP", MIME_OCTET_STREAM),
            MIME_IMAGE_BMP
        );
    }

    #[test]
    fn test_mime_type_from_extension_svg() {
        assert_eq!(
            mime_type_from_extension("svg", MIME_OCTET_STREAM),
            MIME_IMAGE_SVG
        );
        assert_eq!(
            mime_type_from_extension("SVG", MIME_OCTET_STREAM),
            MIME_IMAGE_SVG
        );
    }

    #[test]
    fn test_mime_type_from_extension_unknown() {
        assert_eq!(
            mime_type_from_extension("xyz", MIME_OCTET_STREAM),
            MIME_OCTET_STREAM
        );
        assert_eq!(
            mime_type_from_extension("tiff", MIME_IMAGE_UNKNOWN),
            MIME_IMAGE_UNKNOWN
        );
        assert_eq!(
            mime_type_from_extension("", MIME_OCTET_STREAM),
            MIME_OCTET_STREAM
        );
    }

    #[test]
    fn test_mime_type_from_path_basic() {
        assert_eq!(
            mime_type_from_path("image.png", MIME_OCTET_STREAM),
            MIME_IMAGE_PNG
        );
        assert_eq!(
            mime_type_from_path("photo.jpg", MIME_OCTET_STREAM),
            MIME_IMAGE_JPEG
        );
        assert_eq!(
            mime_type_from_path("/path/to/image.gif", MIME_OCTET_STREAM),
            MIME_IMAGE_GIF
        );
    }

    #[test]
    fn test_mime_type_from_path_case_insensitive() {
        assert_eq!(
            mime_type_from_path("IMAGE.PNG", MIME_OCTET_STREAM),
            MIME_IMAGE_PNG
        );
        assert_eq!(
            mime_type_from_path("Photo.JPG", MIME_OCTET_STREAM),
            MIME_IMAGE_JPEG
        );
    }

    #[test]
    fn test_mime_type_from_path_no_extension() {
        assert_eq!(
            mime_type_from_path("no_extension", MIME_OCTET_STREAM),
            MIME_OCTET_STREAM
        );
        assert_eq!(
            mime_type_from_path("/path/to/file", MIME_IMAGE_UNKNOWN),
            MIME_IMAGE_UNKNOWN
        );
    }

    #[test]
    fn test_mime_type_from_path_hidden_file() {
        // Hidden files (starting with dot) may have no extension
        assert_eq!(
            mime_type_from_path(".hidden", MIME_OCTET_STREAM),
            MIME_OCTET_STREAM
        );
        // But hidden files with extension should work
        assert_eq!(
            mime_type_from_path(".hidden.png", MIME_OCTET_STREAM),
            MIME_IMAGE_PNG
        );
    }

    #[test]
    fn test_mime_type_constants_values() {
        // Verify constant values match expected MIME types
        assert_eq!(MIME_IMAGE_PNG, "image/png");
        assert_eq!(MIME_IMAGE_JPEG, "image/jpeg");
        assert_eq!(MIME_IMAGE_GIF, "image/gif");
        assert_eq!(MIME_IMAGE_BMP, "image/bmp");
        assert_eq!(MIME_IMAGE_SVG, "image/svg+xml");
        assert_eq!(MIME_IMAGE_UNKNOWN, "image/unknown");
        assert_eq!(MIME_OCTET_STREAM, "application/octet-stream");
    }
}
