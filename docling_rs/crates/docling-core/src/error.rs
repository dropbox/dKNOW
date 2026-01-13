//! Error types for document conversion operations.
//!
//! This module defines the error types that can occur during document conversion
//! and provides utilities for error handling.

use thiserror::Error;

/// Error types that can occur during document conversion.
///
/// This enum covers all possible error conditions including Python backend errors,
/// IO errors, format detection failures, and parsing errors.
///
/// # Examples
///
/// ## Pattern Matching on Errors
///
/// ```rust,ignore
/// // Note: DocumentConverter is in docling-backend crate
/// use docling_backend::DocumentConverter;
/// use docling_core::DoclingError;
///
/// let converter = DocumentConverter::new()?;
///
/// match converter.convert("document.pdf") {
///     Ok(result) => println!("Success: {} chars", result.document.metadata.num_characters),
///     Err(DoclingError::IoError(e)) => eprintln!("File error: {}", e),
///     Err(DoclingError::FormatError(msg)) => eprintln!("Unsupported format: {}", msg),
///     Err(DoclingError::ConversionError(msg)) => eprintln!("Conversion failed: {}", msg),
///     Err(e) => eprintln!("Other error: {}", e),
/// }
/// # Ok::<(), DoclingError>(())
/// ```
///
/// ## Using the Result Type Alias
///
/// ```rust,ignore
/// // Note: DocumentConverter is in docling-backend crate
/// use docling_backend::DocumentConverter;
/// use docling_core::Result;
///
/// fn convert_and_save(input: &str, output: &str) -> Result<()> {
///     let converter = DocumentConverter::new()?;
///     let result = converter.convert(input)?;
///     std::fs::write(output, result.document.markdown)?;
///     Ok(())
/// }
/// ```
///
/// ## Error Propagation with ?
///
/// ```rust,ignore
/// // Note: DocumentConverter is in docling-backend crate
/// use docling_backend::DocumentConverter;
/// use docling_core::Result;
///
/// fn batch_convert(files: &[&str]) -> Result<Vec<String>> {
///     let converter = DocumentConverter::new()?;
///     let mut results = Vec::new();
///
///     for file in files {
///         let result = converter.convert(file)?;
///         results.push(result.document.markdown);
///     }
///
///     Ok(results)
/// }
/// ```
#[derive(Error, Debug)]
pub enum DoclingError {
    /// Error from the Python docling backend.
    ///
    /// This occurs when the Python conversion engine encounters an error,
    /// such as unsupported PDF features, corrupted files, or missing dependencies.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// // Note: DocumentConverter is in docling-backend crate
    /// use docling_backend::DocumentConverter;
    /// use docling_core::DoclingError;
    ///
    /// let converter = DocumentConverter::new()?;
    ///
    /// match converter.convert("corrupted.pdf") {
    ///     Err(DoclingError::PythonError(msg)) => {
    ///         eprintln!("Python backend error: {}", msg);
    ///     }
    ///     _ => {}
    /// }
    /// # Ok::<(), DoclingError>(())
    /// ```
    #[error("Python error: {0}")]
    PythonError(String),

    /// General conversion error.
    ///
    /// This is a catch-all error for conversion failures that don't fit
    /// other specific categories.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// // Note: DocumentConverter is in docling-backend crate
    /// use docling_backend::DocumentConverter;
    /// use docling_core::DoclingError;
    ///
    /// let converter = DocumentConverter::new()?;
    ///
    /// match converter.convert("invalid.pdf") {
    ///     Err(DoclingError::ConversionError(msg)) => {
    ///         eprintln!("Conversion failed: {}", msg);
    ///     }
    ///     _ => {}
    /// }
    /// # Ok::<(), DoclingError>(())
    /// ```
    #[error("Conversion error: {0}")]
    ConversionError(String),

    /// File I/O error.
    ///
    /// This occurs when reading input files or writing output files fails,
    /// such as file not found, permission denied, or disk full.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// // Note: DocumentConverter is in docling-backend crate
    /// use docling_backend::DocumentConverter;
    /// use docling_core::DoclingError;
    ///
    /// let converter = DocumentConverter::new()?;
    ///
    /// match converter.convert("missing.pdf") {
    ///     Err(DoclingError::IoError(e)) => {
    ///         eprintln!("File error: {}", e);
    ///         if e.kind() == std::io::ErrorKind::NotFound {
    ///             eprintln!("File does not exist");
    ///         }
    ///     }
    ///     _ => {}
    /// }
    /// # Ok::<(), DoclingError>(())
    /// ```
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// JSON serialization/deserialization error.
    ///
    /// This occurs when parsing JSON output from the Python backend or
    /// serializing structured content fails.
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// YAML serialization/deserialization error.
    ///
    /// This occurs when serializing documents to YAML format fails.
    #[error("YAML error: {0}")]
    YamlError(#[from] serde_yaml::Error),

    /// Format detection or unsupported format error.
    ///
    /// This occurs when the file format cannot be detected from the extension
    /// or when the format is not supported by the current backend.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// // Note: DocumentConverter is in docling-backend crate
    /// use docling_backend::DocumentConverter;
    /// use docling_core::DoclingError;
    ///
    /// let converter = DocumentConverter::new()?;
    ///
    /// match converter.convert("document.xyz") {
    ///     Err(DoclingError::FormatError(msg)) => {
    ///         eprintln!("Unsupported format: {}", msg);
    ///     }
    ///     _ => {}
    /// }
    /// # Ok::<(), DoclingError>(())
    /// ```
    #[error("Format detection error: {0}")]
    FormatError(String),

    /// Backend-specific error.
    ///
    /// This occurs when the Rust backend encounters an error processing
    /// a format that doesn't require the Python backend.
    #[error("Backend error: {0}")]
    BackendError(String),

    /// Parser error from format-specific parsers.
    ///
    /// This occurs when parsing specific formats like IDML, e-books,
    /// or archives fails.
    #[error("Parser error: {0}")]
    ParserError(#[from] anyhow::Error),
}

// pyo3::PyErr conversion removed with python-bridge feature
// Python backend no longer exists - all backends are pure Rust/C++

impl From<docling_adobe::IdmlError> for DoclingError {
    #[inline]
    fn from(err: docling_adobe::IdmlError) -> Self {
        Self::ParserError(anyhow::anyhow!(err.to_string()))
    }
}

/// Type alias for [`Result<T, DoclingError>`].
///
/// This is a convenience type alias for functions that return a [`DoclingError`].
///
/// # Examples
///
/// ```rust,ignore
/// // Note: DocumentConverter is in docling-backend crate
/// use docling_backend::DocumentConverter;
/// use docling_core::Result;
///
/// fn convert_document(path: &str) -> Result<String> {
///     let converter = DocumentConverter::new()?;
///     let result = converter.convert(path)?;
///     Ok(result.document.markdown)
/// }
/// ```
pub type Result<T> = std::result::Result<T, DoclingError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_python_error_display() {
        let error = DoclingError::PythonError("Python module not found".to_string());
        let display = format!("{error}");
        assert_eq!(display, "Python error: Python module not found");
        assert!(display.contains("Python"));
        assert!(display.contains("module not found"));
    }

    #[test]
    fn test_conversion_error_display() {
        let error = DoclingError::ConversionError("Failed to parse document structure".to_string());
        let display = format!("{error}");
        assert_eq!(
            display,
            "Conversion error: Failed to parse document structure"
        );
        assert!(display.contains("Conversion"));
        assert!(display.contains("parse"));
    }

    #[test]
    fn test_io_error_conversion() {
        // Test automatic conversion from std::io::Error
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let docling_err: DoclingError = io_err.into();

        match docling_err {
            DoclingError::IoError(e) => {
                assert_eq!(e.kind(), std::io::ErrorKind::NotFound);
                assert!(e.to_string().contains("file not found"));
            }
            _ => panic!("Expected IoError variant"),
        }
    }

    #[test]
    fn test_json_error_conversion() {
        // Test automatic conversion from serde_json::Error
        let json_str = "{ invalid json }";
        let json_err = serde_json::from_str::<serde_json::Value>(json_str).unwrap_err();
        let docling_err: DoclingError = json_err.into();

        match docling_err {
            DoclingError::JsonError(e) => {
                // Just verify we can convert - error message format varies
                let msg = e.to_string();
                assert!(!msg.is_empty(), "JSON error message should not be empty");
            }
            _ => panic!("Expected JsonError variant"),
        }
    }

    #[test]
    fn test_format_error_display() {
        let error = DoclingError::FormatError("Unknown file extension .xyz".to_string());
        let display = format!("{error}");
        assert_eq!(
            display,
            "Format detection error: Unknown file extension .xyz"
        );
        assert!(display.contains("Format"));
        assert!(display.contains(".xyz"));
    }

    #[test]
    fn test_backend_error_display() {
        let error = DoclingError::BackendError("IDML parsing failed".to_string());
        let display = format!("{error}");
        assert_eq!(display, "Backend error: IDML parsing failed");
        assert!(display.contains("Backend"));
        assert!(display.contains("IDML"));
    }

    #[test]
    fn test_parser_error_from_anyhow() {
        // Test conversion from anyhow::Error
        let anyhow_err = anyhow::anyhow!("Custom parser failure");
        let docling_err: DoclingError = anyhow_err.into();

        match docling_err {
            DoclingError::ParserError(e) => {
                assert!(e.to_string().contains("Custom parser failure"));
            }
            _ => panic!("Expected ParserError variant"),
        }
    }

    #[test]
    fn test_error_debug_format() {
        let error = DoclingError::ConversionError("test error".to_string());
        let debug = format!("{error:?}");
        assert!(debug.contains("ConversionError"));
        assert!(debug.contains("test error"));
    }

    #[test]
    fn test_result_type_alias() {
        // Test that Result<T> type alias works correctly
        fn returns_ok() -> Result<String> {
            Ok("success".to_string())
        }

        fn returns_err() -> Result<String> {
            Err(DoclingError::ConversionError("failure".to_string()))
        }

        assert_eq!(returns_ok().unwrap(), "success");
        assert!(returns_err().is_err());

        match returns_err() {
            Err(DoclingError::ConversionError(msg)) => assert_eq!(msg, "failure"),
            _ => panic!("Expected ConversionError"),
        }
    }

    #[test]
    fn test_error_propagation_with_question_mark() {
        // Test that errors propagate correctly with ? operator
        fn inner_function() -> Result<String> {
            Err(DoclingError::FormatError("unsupported".to_string()))
        }

        fn outer_function() -> Result<String> {
            let _result = inner_function()?;
            Ok("should not reach".to_string())
        }

        match outer_function() {
            Err(DoclingError::FormatError(msg)) => assert_eq!(msg, "unsupported"),
            _ => panic!("Expected FormatError to propagate"),
        }
    }

    #[test]
    fn test_multiple_error_variants() {
        // Test that we can match on different error variants
        let errors: Vec<DoclingError> = vec![
            DoclingError::PythonError("py err".to_string()),
            DoclingError::ConversionError("conv err".to_string()),
            DoclingError::FormatError("fmt err".to_string()),
            DoclingError::BackendError("backend err".to_string()),
        ];

        for error in errors {
            match error {
                DoclingError::PythonError(msg) => assert!(msg.contains("py")),
                DoclingError::ConversionError(msg) => assert!(msg.contains("conv")),
                DoclingError::FormatError(msg) => assert!(msg.contains("fmt")),
                DoclingError::BackendError(msg) => assert!(msg.contains("backend")),
                _ => {}
            }
        }
    }

    #[test]
    fn test_error_size() {
        // Verify error size is reasonable (errors should be small to avoid stack issues)
        use std::mem::size_of;
        let size = size_of::<DoclingError>();

        // DoclingError should be reasonably sized (typically 24-48 bytes)
        // This is a sanity check - if this fails, error variants may need boxing.
        assert!(
            size < 256,
            "DoclingError size is {size} bytes, consider boxing large variants"
        );
    }
}
