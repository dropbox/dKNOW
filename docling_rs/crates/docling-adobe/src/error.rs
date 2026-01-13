//! Error types for docling-adobe
use thiserror::Error;

/// A specialized Result type for IDML operations
pub type Result<T> = std::result::Result<T, IdmlError>;

/// Errors that can occur during IDML parsing and processing
#[derive(Error, Debug, Clone, PartialEq, Eq, Hash)]
pub enum IdmlError {
    /// I/O error (file not found, read error, etc.)
    #[error("IO error: {0}")]
    IoError(String),

    /// XML or content parsing error
    #[error("Parse error: {0}")]
    ParseError(String),

    /// Invalid IDML package structure
    #[error("Invalid IDML structure: {0}")]
    InvalidStructure(String),
}
