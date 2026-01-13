//! Error types for GPS format parsing

use std::io;
use thiserror::Error;

/// Errors that can occur during GPS format parsing
#[derive(Debug, Error)]
pub enum GpsError {
    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    /// GPX parsing error
    #[error("GPX parsing error: {0}")]
    GpxParse(String),

    /// KML parsing error
    #[error("KML parsing error: {0}")]
    KmlParse(String),

    /// Invalid GPX format
    #[error("Invalid GPX format: {0}")]
    InvalidFormat(String),

    /// Unsupported GPS format
    #[error("Unsupported GPS format: {0}")]
    UnsupportedFormat(String),
}

/// Result type for GPS operations
pub type Result<T> = std::result::Result<T, GpsError>;
