//! Error types for Jupyter notebook parsing

use thiserror::Error;

/// Error type for notebook parsing operations
#[derive(Error, Debug)]
pub enum NotebookError {
    /// I/O error when reading notebook file
    #[error("Failed to read notebook file: {0}")]
    IoError(#[from] std::io::Error),

    /// JSON parsing error
    #[error("Failed to parse notebook JSON: {0}")]
    JsonError(#[from] serde_json::Error),

    /// Invalid notebook structure or format
    #[error("Invalid notebook format: {0}")]
    InvalidFormat(String),

    /// Notebook version not supported
    #[error("Unsupported notebook version: {major}.{minor}")]
    UnsupportedVersion {
        /// Major version number
        major: u32,
        /// Minor version number
        minor: u32,
    },

    /// General parsing error
    #[error("Notebook parsing error: {0}")]
    ParseError(String),
}

/// Result type alias for notebook operations
pub type Result<T> = std::result::Result<T, NotebookError>;
