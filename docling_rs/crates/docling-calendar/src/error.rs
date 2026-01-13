//! Error types for calendar parsing

use std::path::{Path, PathBuf};
use thiserror::Error;

/// Result type alias for calendar operations
pub type Result<T> = std::result::Result<T, CalendarError>;

/// Error type for calendar parsing operations
#[derive(Error, Debug)]
pub enum CalendarError {
    /// Failed to read calendar file from disk
    #[error("Failed to read calendar file {path}: {source}")]
    ReadError {
        /// Path to the file that failed to read
        path: PathBuf,
        /// Underlying I/O error
        source: std::io::Error,
    },

    /// Calendar file has invalid format or structure
    #[error("Invalid calendar format in {path}: {message}")]
    InvalidFormat {
        /// Path to the file with invalid format
        path: PathBuf,
        /// Description of the format error
        message: String,
    },

    /// General parsing error
    #[error("Failed to parse calendar: {0}")]
    ParseError(String),

    /// Calendar version is not supported
    #[error("Unsupported calendar version: {0}")]
    UnsupportedVersion(String),
}

impl CalendarError {
    /// Create a read error
    #[inline]
    #[must_use = "returns CalendarError for file read failures"]
    pub fn read_error<P: AsRef<Path>>(path: P, source: std::io::Error) -> Self {
        Self::ReadError {
            path: path.as_ref().to_path_buf(),
            source,
        }
    }

    /// Create an invalid format error
    #[inline]
    #[must_use = "returns CalendarError for invalid format"]
    pub fn invalid_format<P: AsRef<Path>>(path: P, message: impl Into<String>) -> Self {
        Self::InvalidFormat {
            path: path.as_ref().to_path_buf(),
            message: message.into(),
        }
    }
}
