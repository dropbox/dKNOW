//! Error types for XPS parsing

use std::io;
use thiserror::Error;

/// XPS parsing errors
#[derive(Debug, Error)]
pub enum XpsError {
    /// I/O error (file not found, permission denied, etc.)
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// ZIP archive error
    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),

    /// XML parsing error
    #[error("XML parsing error: {0}")]
    Xml(#[from] quick_xml::Error),

    /// Invalid XPS structure (missing required files, invalid format)
    #[error("Invalid XPS structure: {0}")]
    InvalidStructure(String),

    /// Missing required file in XPS archive
    #[error("Missing required file: {0}")]
    MissingFile(String),

    /// Unsupported XPS feature
    #[error("Unsupported feature: {0}")]
    Unsupported(String),

    /// Generic parsing error
    #[error("Parse error: {0}")]
    Parse(String),
}

/// Result type for XPS operations
pub type Result<T> = std::result::Result<T, XpsError>;
