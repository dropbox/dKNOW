//! Error types for `OpenDocument` format parsing

use std::io;
use thiserror::Error;

/// Errors that can occur when parsing `OpenDocument` formats
#[derive(Error, Debug)]
pub enum OdfError {
    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// ZIP archive error
    #[error("ZIP archive error: {0}")]
    Zip(#[from] zip::result::ZipError),

    /// XML parsing error
    #[error("XML parsing error: {0}")]
    Xml(#[from] quick_xml::Error),

    /// Missing required file in archive
    #[error("Missing required file: {0}")]
    MissingFile(String),

    /// Invalid document structure
    #[error("Invalid document structure: {0}")]
    InvalidStructure(String),

    /// Unsupported feature
    #[error("Unsupported feature: {0}")]
    Unsupported(String),

    /// calamine error (for ODS parsing)
    #[error("Spreadsheet parsing error: {0}")]
    Calamine(String),

    /// UTF-8 conversion error
    #[error("UTF-8 conversion error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),

    /// Other error
    #[error("{0}")]
    Other(String),
}

/// Result type for `OpenDocument` operations
pub type Result<T> = std::result::Result<T, OdfError>;

impl From<calamine::Error> for OdfError {
    #[inline]
    fn from(err: calamine::Error) -> Self {
        Self::Calamine(err.to_string())
    }
}

impl From<calamine::OdsError> for OdfError {
    #[inline]
    fn from(err: calamine::OdsError) -> Self {
        Self::Calamine(err.to_string())
    }
}

impl From<calamine::XlsxError> for OdfError {
    #[inline]
    fn from(err: calamine::XlsxError) -> Self {
        Self::Calamine(err.to_string())
    }
}
