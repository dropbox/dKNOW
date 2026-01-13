//! Error types for email parsing

use std::io;
use thiserror::Error;

/// Result type for email parsing operations
pub type Result<T> = std::result::Result<T, EmailError>;

/// Email parsing errors
#[derive(Debug, Error)]
pub enum EmailError {
    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// Email parse error
    #[error("Failed to parse email: {0}")]
    ParseError(String),

    /// MBOX parse error
    #[error("Failed to parse MBOX: {0}")]
    MboxError(String),

    /// `VCard` parse error
    #[error("Failed to parse vCard: {0}")]
    VCardError(String),

    /// Invalid format
    #[error("Invalid format: {0}")]
    InvalidFormat(String),

    /// Unsupported feature
    #[error("Unsupported feature: {0}")]
    Unsupported(String),
}
