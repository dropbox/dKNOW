//! SVG error types

use std::io;
use thiserror::Error;

/// SVG parsing errors
#[derive(Error, Debug)]
pub enum SvgError {
    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// XML parsing error
    #[error("XML parse error: {0}")]
    XmlError(String),

    /// Invalid SVG structure
    #[error("Invalid SVG structure: {0}")]
    InvalidStructure(String),
}

/// Result type for SVG operations
pub type Result<T> = std::result::Result<T, SvgError>;
