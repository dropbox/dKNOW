use std::io;
use thiserror::Error;

/// Errors that can occur during video processing operations
#[derive(Error, Debug)]
pub enum VideoError {
    /// `FFmpeg` is not installed or not found in the system `PATH`
    #[error("FFmpeg not found in PATH. Please install FFmpeg: <https://ffmpeg.org/download.html>")]
    FfmpegNotFound,

    /// `FFmpeg` command execution failed with the given error message
    #[error("FFmpeg command failed: {0}")]
    FfmpegCommandFailed(String),

    /// No subtitle tracks were found in the video file
    #[error("No subtitle tracks found in video file")]
    NoSubtitleTracks,

    /// No audio track was found in the video file
    #[error("No audio track found in video file")]
    NoAudioTrack,

    /// Failed to parse subtitle track metadata from `FFmpeg` output
    #[error("Failed to parse subtitle track information: {0}")]
    SubtitleTrackParsingFailed(String),

    /// Failed to parse subtitle file content (SRT, VTT, etc.)
    #[error("Failed to parse subtitle file: {0}")]
    SubtitleParsingFailed(String),

    /// The subtitle format is not supported
    #[error("Unsupported subtitle format: {0}")]
    UnsupportedSubtitleFormat(String),

    /// Audio transcription operation failed
    #[error("Audio transcription failed: {0}")]
    TranscriptionFailed(String),

    /// I/O error occurred during file operations
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    /// Invalid UTF-8 encountered in output
    #[error("UTF-8 error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),

    /// Failed to create or access a temporary file
    #[error("Temporary file error: {0}")]
    TempFile(String),

    /// The specified video file was not found
    #[error("Video file not found: {0}")]
    FileNotFound(String),
}

/// A specialized Result type for video processing operations
pub type Result<T> = std::result::Result<T, VideoError>;
