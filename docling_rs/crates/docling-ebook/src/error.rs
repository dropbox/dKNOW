/// Error types for e-book parsing
use std::io;

/// Result type alias for e-book operations
pub type Result<T> = std::result::Result<T, EbookError>;

/// Errors that can occur during e-book parsing
#[derive(Debug, thiserror::Error)]
pub enum EbookError {
    /// Invalid e-book structure (missing or malformed components)
    #[error("Invalid e-book structure: {0}")]
    InvalidStructure(String),

    /// Required file not found in e-book archive
    #[error("Missing required file: {0}")]
    MissingFile(String),

    /// XML parsing failed
    #[error("Failed to parse XML: {0}")]
    XmlParse(String),

    /// ZIP archive extraction error
    #[error("Failed to extract ZIP: {0}")]
    ZipError(#[from] zip::result::ZipError),

    /// E-book is DRM-protected and cannot be parsed
    #[error("DRM-protected content: {0}")]
    DrmProtected(String),

    /// E-book format version not supported
    #[error("Unsupported format version: {0}")]
    UnsupportedVersion(String),

    /// Invalid or malformed metadata
    #[error("Invalid metadata: {0}")]
    InvalidMetadata(String),

    /// Standard I/O error
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    /// UTF-8 decoding error
    #[error("UTF-8 decode error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),

    /// EPUB-specific parsing error
    #[error("EPUB parsing error: {0}")]
    EpubError(String),

    /// General parsing error
    #[error("Parse error: {0}")]
    ParseError(String),

    /// I/O error with custom message
    #[error("IO error: {0}")]
    IoError(String),
}
