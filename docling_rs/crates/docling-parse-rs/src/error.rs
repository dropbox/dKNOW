//! Error types for docling-parse operations

use std::ffi::NulError;
use thiserror::Error;

/// Result type for docling-parse operations
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur when using docling-parse
#[derive(Debug, Error)]
pub enum Error {
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Failed to load document: {0}")]
    LoadFailed(String),

    #[error("Failed to parse document: {0}")]
    ParseFailed(String),

    #[error("Document not loaded: {0}")]
    NotLoaded(String),

    #[error("Out of memory")]
    OutOfMemory,

    #[error("Unknown error code: {0}")]
    UnknownError(i32),

    #[error("Null pointer in C string")]
    NullPointer,

    #[error("Invalid UTF-8 in string: {0}")]
    InvalidUtf8(#[from] std::str::Utf8Error),

    #[error("Failed to create C string: {0}")]
    NulByteInString(#[from] NulError),

    #[error("Failed to parse JSON: {0}")]
    JsonParseFailed(#[from] serde_json::Error),

    #[error("JSON parse error: {0}")]
    JsonParseError(String),

    #[error("Conversion error: {0}")]
    ConversionError(String),
}

impl Error {
    /// Convert a DoclingError code from C to a Rust error
    #[inline]
    pub(crate) fn from_c_error(code: docling_parse_sys::DoclingError) -> Self {
        use docling_parse_sys::*;
        #[allow(non_upper_case_globals, reason = "C FFI constants use underscore_case naming")]
        match code {
            DoclingError_DOCLING_OK => panic!("Cannot create error from DOCLING_OK"),
            DoclingError_DOCLING_ERROR_INVALID_PARAM => Error::InvalidParameter("".to_string()),
            DoclingError_DOCLING_ERROR_FILE_NOT_FOUND => Error::FileNotFound("".to_string()),
            DoclingError_DOCLING_ERROR_LOAD_FAILED => Error::LoadFailed("".to_string()),
            DoclingError_DOCLING_ERROR_PARSE_FAILED => Error::ParseFailed("".to_string()),
            DoclingError_DOCLING_ERROR_NOT_LOADED => Error::NotLoaded("".to_string()),
            DoclingError_DOCLING_ERROR_OUT_OF_MEMORY => Error::OutOfMemory,
            _ => Error::UnknownError(code as i32),
        }
    }
}
