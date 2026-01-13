//! Error types for the Docling PDF parsing library
//!
//! This module defines the error types that can occur during PDF parsing.
//! All public APIs use the `Result<T>` type alias which wraps `DoclingError`.
//!
//! # Examples
//!
//! ```no_run
//! use docling_pdf_ml::{Pipeline, PipelineConfig, DoclingError};
//! use ndarray::Array3;
//!
//! # fn example() -> docling_pdf_ml::Result<()> {
//! let mut pipeline = Pipeline::new(PipelineConfig::default())?;
//!
//! // Match on specific error types
//! # let image = Array3::<u8>::zeros((792, 612, 3));
//! match pipeline.process_page(0, &image, 612.0, 792.0, None) {
//!     Ok(page) => log::debug!("Success"),
//!     Err(DoclingError::InferenceError { model_name, .. }) => {
//!         log::warn!("Inference failed for {}", model_name);
//!     }
//!     Err(e) => log::warn!("Other error: {}", e),
//! }
//! # Ok(())
//! # }
//! ```

use std::fmt;

/// Errors that can occur during PDF parsing
///
/// This enum represents all possible errors that can occur when using
/// the Docling PDF parsing library. Errors are categorized by the
/// stage of processing where they occurred.
///
/// # Error Categories
///
/// - **Configuration Errors** ([`ConfigError`]): Invalid configuration (user error, fixable)
/// - **Model Errors** ([`ModelLoadError`]): Model loading failed (setup issue, check paths/files)
/// - **Processing Errors** ([`PreprocessingError`], [`InferenceError`], [`AssemblyError`]): Runtime errors during processing
/// - **I/O Errors** ([`IoError`]): File system errors
///
/// [`ConfigError`]: DoclingError::ConfigError
/// [`ModelLoadError`]: DoclingError::ModelLoadError
/// [`PreprocessingError`]: DoclingError::PreprocessingError
/// [`InferenceError`]: DoclingError::InferenceError
/// [`AssemblyError`]: DoclingError::AssemblyError
/// [`IoError`]: DoclingError::IoError
#[derive(Debug)]
pub enum DoclingError {
    /// Model loading failed
    ///
    /// This error occurs when ML models fail to load at pipeline initialization.
    /// Common causes:
    /// - Model files not found at specified paths
    /// - Corrupted model files
    /// - Insufficient memory to load models
    /// - Incompatible model format
    ModelLoadError {
        /// Name of the model that failed to load (e.g., "`LayoutPredictor`", "`TableFormer`")
        model_name: String,
        /// The underlying error that caused the failure
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Image preprocessing failed
    ///
    /// This error occurs when image preprocessing operations fail.
    /// Common causes:
    /// - Invalid image dimensions
    /// - Unsupported image format
    /// - Memory allocation failure during preprocessing
    PreprocessingError {
        /// Description of what went wrong during preprocessing
        reason: String,
    },

    /// ML inference failed
    ///
    /// This error occurs when ML model inference fails during page processing.
    /// Common causes:
    /// - Invalid input dimensions
    /// - GPU out of memory
    /// - Model execution error
    InferenceError {
        /// Name of the model that failed inference (e.g., "`LayoutPredictor`", "`TableFormer`")
        model_name: String,
        /// The underlying error that caused the failure
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Page assembly failed
    ///
    /// This error occurs when converting ML predictions into structured page elements fails.
    /// Common causes:
    /// - Inconsistent cluster IDs
    /// - Invalid bounding boxes
    /// - Missing required data
    AssemblyError {
        /// Description of what went wrong during assembly
        reason: String,
    },

    /// Invalid configuration
    ///
    /// This error occurs when pipeline configuration is invalid.
    /// Common causes:
    /// - Invalid model paths
    /// - Unsupported device configuration
    /// - Conflicting configuration options
    ConfigError {
        /// Description of what is invalid in the configuration
        reason: String,
    },

    /// IO error (file not found, permission denied, etc.)
    ///
    /// This error wraps standard I/O errors that occur when reading/writing files.
    IoError(std::io::Error),
}

impl fmt::Display for DoclingError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ModelLoadError { model_name, source } => {
                write!(f, "Failed to load {model_name} model: {source}")
            }
            Self::PreprocessingError { reason } => {
                write!(f, "Image preprocessing failed: {reason}")
            }
            Self::InferenceError { model_name, source } => {
                write!(f, "Inference failed for {model_name}: {source}")
            }
            Self::AssemblyError { reason } => {
                write!(f, "Page assembly failed: {reason}")
            }
            Self::ConfigError { reason } => {
                write!(f, "Invalid configuration: {reason}")
            }
            Self::IoError(err) => {
                write!(f, "IO error: {err}")
            }
        }
    }
}

impl std::error::Error for DoclingError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ModelLoadError { source, .. } => {
                Some(source.as_ref() as &(dyn std::error::Error + 'static))
            }
            Self::InferenceError { source, .. } => {
                Some(source.as_ref() as &(dyn std::error::Error + 'static))
            }
            Self::IoError(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for DoclingError {
    #[inline]
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err)
    }
}

impl From<ort::Error> for DoclingError {
    #[inline]
    fn from(err: ort::Error) -> Self {
        Self::ModelLoadError {
            model_name: "ONNX".to_string(),
            source: Box::new(err),
        }
    }
}

impl From<anyhow::Error> for DoclingError {
    #[inline]
    fn from(err: anyhow::Error) -> Self {
        Self::PreprocessingError {
            reason: err.to_string(),
        }
    }
}

#[cfg(feature = "opencv-preprocessing")]
impl From<opencv::Error> for DoclingError {
    #[inline]
    fn from(err: opencv::Error) -> Self {
        Self::PreprocessingError {
            reason: format!("OpenCV error: {}", err),
        }
    }
}

impl DoclingError {
    /// Returns true if this error is a configuration error (user-fixable)
    ///
    /// Configuration errors indicate that the user provided invalid configuration.
    /// These can typically be fixed by adjusting the configuration parameters.
    ///
    /// # Examples
    ///
    /// ```
    /// use docling_pdf_ml::DoclingError;
    ///
    /// let err = DoclingError::ConfigError {
    ///     reason: "Invalid device".to_string()
    /// };
    /// assert!(err.is_config_error());
    /// ```
    #[inline]
    #[must_use = "this method returns a boolean, not modifying the error"]
    pub const fn is_config_error(&self) -> bool {
        matches!(self, Self::ConfigError { .. })
    }

    /// Returns true if this error is a model loading error
    ///
    /// Model loading errors typically indicate setup issues like missing model files
    /// or corrupted weights. Check model paths and ensure models are downloaded.
    #[inline]
    #[must_use = "this method returns a boolean, not modifying the error"]
    pub const fn is_model_load_error(&self) -> bool {
        matches!(self, Self::ModelLoadError { .. })
    }

    /// Returns true if this error occurred during inference
    ///
    /// Inference errors may indicate GPU out of memory, invalid input dimensions,
    /// or model execution failures. Consider using CPU device or smaller batch sizes.
    #[inline]
    #[must_use = "this method returns a boolean, not modifying the error"]
    pub const fn is_inference_error(&self) -> bool {
        matches!(self, Self::InferenceError { .. })
    }

    /// Returns true if this is an I/O error
    ///
    /// I/O errors indicate file system issues like missing files or permission problems.
    #[inline]
    #[must_use = "this method returns a boolean, not modifying the error"]
    pub const fn is_io_error(&self) -> bool {
        matches!(self, Self::IoError(_))
    }
}

/// Type alias for Result with `DoclingError`
///
/// This is a convenience type alias that is used throughout the library.
/// It is equivalent to `std::result::Result<T, DoclingError>`.
///
/// # Examples
///
/// ```no_run
/// use docling_pdf_ml::Result;
///
/// fn my_function() -> Result<()> {
///     // Your code here
///     Ok(())
/// }
/// ```
pub type Result<T> = std::result::Result<T, DoclingError>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    /// Helper to create a simple boxed error for testing
    fn make_test_error() -> Box<dyn std::error::Error + Send + Sync> {
        Box::new(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "test error",
        ))
    }

    // ========== Display implementation tests ==========

    #[test]
    fn test_model_load_error_display() {
        let err = DoclingError::ModelLoadError {
            model_name: "LayoutPredictor".to_string(),
            source: make_test_error(),
        };
        let msg = err.to_string();
        assert!(msg.contains("Failed to load LayoutPredictor model"));
        assert!(msg.contains("test error"));
    }

    #[test]
    fn test_preprocessing_error_display() {
        let err = DoclingError::PreprocessingError {
            reason: "Invalid image dimensions".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Image preprocessing failed: Invalid image dimensions"
        );
    }

    #[test]
    fn test_inference_error_display() {
        let err = DoclingError::InferenceError {
            model_name: "TableFormer".to_string(),
            source: make_test_error(),
        };
        let msg = err.to_string();
        assert!(msg.contains("Inference failed for TableFormer"));
        assert!(msg.contains("test error"));
    }

    #[test]
    fn test_assembly_error_display() {
        let err = DoclingError::AssemblyError {
            reason: "Invalid cluster ID".to_string(),
        };
        assert_eq!(err.to_string(), "Page assembly failed: Invalid cluster ID");
    }

    #[test]
    fn test_config_error_display() {
        let err = DoclingError::ConfigError {
            reason: "Invalid device".to_string(),
        };
        assert_eq!(err.to_string(), "Invalid configuration: Invalid device");
    }

    #[test]
    fn test_io_error_display() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err = DoclingError::IoError(io_err);
        assert!(err.to_string().contains("IO error"));
        assert!(err.to_string().contains("file not found"));
    }

    // ========== Error source tests ==========

    #[test]
    fn test_model_load_error_has_source() {
        let err = DoclingError::ModelLoadError {
            model_name: "Test".to_string(),
            source: make_test_error(),
        };
        assert!(err.source().is_some());
    }

    #[test]
    fn test_inference_error_has_source() {
        let err = DoclingError::InferenceError {
            model_name: "Test".to_string(),
            source: make_test_error(),
        };
        assert!(err.source().is_some());
    }

    #[test]
    fn test_io_error_has_source() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "test");
        let err = DoclingError::IoError(io_err);
        assert!(err.source().is_some());
    }

    #[test]
    fn test_preprocessing_error_no_source() {
        let err = DoclingError::PreprocessingError {
            reason: "test".to_string(),
        };
        assert!(err.source().is_none());
    }

    #[test]
    fn test_assembly_error_no_source() {
        let err = DoclingError::AssemblyError {
            reason: "test".to_string(),
        };
        assert!(err.source().is_none());
    }

    #[test]
    fn test_config_error_no_source() {
        let err = DoclingError::ConfigError {
            reason: "test".to_string(),
        };
        assert!(err.source().is_none());
    }

    // ========== Helper method tests ==========

    #[test]
    fn test_is_config_error() {
        let config_err = DoclingError::ConfigError {
            reason: "test".to_string(),
        };
        assert!(config_err.is_config_error());

        let other_err = DoclingError::PreprocessingError {
            reason: "test".to_string(),
        };
        assert!(!other_err.is_config_error());
    }

    #[test]
    fn test_is_model_load_error() {
        let model_err = DoclingError::ModelLoadError {
            model_name: "Test".to_string(),
            source: make_test_error(),
        };
        assert!(model_err.is_model_load_error());

        let other_err = DoclingError::ConfigError {
            reason: "test".to_string(),
        };
        assert!(!other_err.is_model_load_error());
    }

    #[test]
    fn test_is_inference_error() {
        let inference_err = DoclingError::InferenceError {
            model_name: "Test".to_string(),
            source: make_test_error(),
        };
        assert!(inference_err.is_inference_error());

        let other_err = DoclingError::ConfigError {
            reason: "test".to_string(),
        };
        assert!(!other_err.is_inference_error());
    }

    #[test]
    fn test_is_io_error() {
        let io_err =
            DoclingError::IoError(std::io::Error::new(std::io::ErrorKind::NotFound, "test"));
        assert!(io_err.is_io_error());

        let other_err = DoclingError::ConfigError {
            reason: "test".to_string(),
        };
        assert!(!other_err.is_io_error());
    }

    // ========== From trait tests ==========

    #[test]
    fn test_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "not found");
        let docling_err: DoclingError = io_err.into();
        assert!(docling_err.is_io_error());
    }

    #[test]
    fn test_from_anyhow_error() {
        let anyhow_err = anyhow::anyhow!("test anyhow error");
        let docling_err: DoclingError = anyhow_err.into();
        // anyhow errors become PreprocessingError
        assert!(matches!(
            docling_err,
            DoclingError::PreprocessingError { .. }
        ));
        assert!(docling_err.to_string().contains("test anyhow error"));
    }
}
