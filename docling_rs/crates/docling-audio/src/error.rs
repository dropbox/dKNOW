//! Error types for audio processing

use std::path::PathBuf;

/// Result type for audio operations
pub type Result<T> = std::result::Result<T, AudioError>;

/// Errors that can occur during audio processing
#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    /// File I/O error
    #[error("Failed to read audio file {path}: {source}")]
    Io {
        /// Path to the audio file
        path: PathBuf,
        /// Underlying I/O error
        source: std::io::Error,
    },

    /// Invalid audio format or corrupted file
    #[error("Invalid audio format in {path}: {message}")]
    InvalidFormat {
        /// Path to the audio file
        path: PathBuf,
        /// Description of the format error
        message: String,
    },

    /// Unsupported audio format
    #[error("Unsupported audio format: {format}")]
    UnsupportedFormat {
        /// Format name that is not supported
        format: String,
    },

    /// Transcription feature not enabled
    #[error("Transcription feature not enabled. Compile with --features transcription")]
    TranscriptionDisabled,

    /// Transcription model not found
    #[error("Transcription model not found at {path}")]
    ModelNotFound {
        /// Path where the model was expected
        path: PathBuf,
    },

    /// Transcription failed
    #[error("Transcription failed: {message}")]
    TranscriptionFailed {
        /// Description of the transcription failure
        message: String,
    },

    /// Audio resampling failed
    #[error("Failed to resample audio: {message}")]
    ResamplingFailed {
        /// Description of the resampling failure
        message: String,
    },

    /// Other error
    #[error("{0}")]
    Other(String),
}

impl AudioError {
    /// Create an I/O error
    #[inline]
    #[must_use = "creates an I/O error that should be returned or handled"]
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }

    /// Create an invalid format error
    #[inline]
    #[must_use = "creates an invalid format error that should be returned or handled"]
    pub fn invalid_format(path: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        Self::InvalidFormat {
            path: path.into(),
            message: message.into(),
        }
    }

    /// Create an unsupported format error
    #[inline]
    #[must_use = "creates an unsupported format error that should be returned or handled"]
    pub fn unsupported_format(format: impl Into<String>) -> Self {
        Self::UnsupportedFormat {
            format: format.into(),
        }
    }

    /// Create a model not found error
    #[inline]
    #[must_use = "creates a model not found error that should be returned or handled"]
    pub fn model_not_found(path: impl Into<PathBuf>) -> Self {
        Self::ModelNotFound { path: path.into() }
    }

    /// Create a transcription failed error
    #[inline]
    #[must_use = "creates a transcription failed error that should be returned or handled"]
    pub fn transcription_failed(message: impl Into<String>) -> Self {
        Self::TranscriptionFailed {
            message: message.into(),
        }
    }

    /// Create a resampling failed error
    #[inline]
    #[must_use = "creates a resampling failed error that should be returned or handled"]
    pub fn resampling_failed(message: impl Into<String>) -> Self {
        Self::ResamplingFailed {
            message: message.into(),
        }
    }
}
