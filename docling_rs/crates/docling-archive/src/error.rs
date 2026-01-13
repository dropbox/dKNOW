//! Error types for archive operations

use thiserror::Error;

/// Errors that can occur during archive operations
#[derive(Error, Debug)]
pub enum ArchiveError {
    /// IO error during archive operations
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Invalid ZIP archive format
    #[error("Invalid ZIP archive: {0}")]
    InvalidZip(#[from] zip::result::ZipError),

    /// Archive is password-protected
    #[error("Archive is password-protected")]
    PasswordProtected,

    /// Archive exceeds size limits
    #[error("Archive is too large (max {max} bytes)")]
    TooLarge {
        /// Maximum allowed size in bytes
        max: usize,
    },

    /// Archive nesting exceeds depth limit
    #[error("Archive nesting too deep (max depth {max})")]
    TooDeep {
        /// Maximum allowed nesting depth
        max: usize,
    },

    /// Unsupported compression method
    #[error("Unsupported compression method: {0}")]
    UnsupportedCompression(String),

    /// File within archive exceeds size limit
    #[error("File '{name}' is too large ({size} bytes, max {max} bytes)")]
    FileTooLarge {
        /// Name of the file that exceeded the limit
        name: String,
        /// Actual file size in bytes
        size: u64,
        /// Maximum allowed file size in bytes
        max: u64,
    },

    /// Generic error for other cases
    #[error("Archive error: {0}")]
    Other(String),
}
