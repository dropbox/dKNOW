//! Error types for pdfium-render-fast

use thiserror::Error;

/// Result type for pdfium-render-fast operations
pub type Result<T> = std::result::Result<T, PdfError>;

/// Error types for PDF operations
#[derive(Error, Debug)]
pub enum PdfError {
    /// Failed to initialize PDFium library
    #[error("Failed to initialize PDFium library")]
    InitializationFailed,

    /// File not found
    #[error("File not found: {0}")]
    FileNotFound(String),

    /// Failed to open PDF document
    #[error("Failed to open PDF document: {reason}")]
    OpenFailed { reason: String },

    /// Invalid password for encrypted PDF
    #[error("Invalid password for encrypted PDF")]
    InvalidPassword,

    /// Page index out of bounds
    #[error("Page index {index} out of bounds (document has {count} pages)")]
    PageIndexOutOfBounds { index: usize, count: usize },

    /// Failed to load page
    #[error("Failed to load page {index}")]
    PageLoadFailed { index: usize },

    /// Failed to render page
    #[error("Failed to render page: {reason}")]
    RenderFailed { reason: String },

    /// Failed to create bitmap
    #[error("Failed to create bitmap: {reason}")]
    BitmapCreationFailed { reason: String },

    /// Failed to extract text
    #[error("Failed to extract text: {reason}")]
    TextExtractionFailed { reason: String },

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// PNG encoding error
    #[error("PNG encoding error: {0}")]
    PngEncoding(String),

    /// JPEG encoding error
    #[error("JPEG encoding error: {0}")]
    JpegEncoding(String),

    /// Invalid parameter
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    /// Failed to extract links
    #[error("Failed to extract links: {reason}")]
    LinkExtractionFailed { reason: String },

    /// Invalid data (e.g., malformed UTF-16)
    #[error("Invalid data: {reason}")]
    InvalidData { reason: String },

    /// Search operation failed
    #[error("Search error: {0}")]
    SearchError(String),

    /// Failed to save document
    #[error("Failed to save document: {reason}")]
    SaveFailed { reason: String },

    /// IO operation error (distinct from std::io::Error for custom messages)
    #[error("IO error: {message}")]
    IoError { message: String },

    /// Invalid input provided
    #[error("Invalid input: {message}")]
    InvalidInput { message: String },

    /// Flatten operation failed
    #[error("Flatten failed: {reason}")]
    FlattenFailed { reason: String },

    /// Clip path creation failed
    #[error("Failed to create clip path")]
    ClipPathCreationFailed,

    /// Image loading failed
    #[error("Failed to load image: {reason}")]
    ImageLoadFailed { reason: String },

    /// Object creation failed
    #[error("Failed to create object: {reason}")]
    ObjectCreationFailed { reason: String },
}
