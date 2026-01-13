/// Common types and utilities for video/audio processing
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;

/// Processing errors
#[derive(Debug, Error)]
pub enum ProcessingError {
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error("File too large: {size} bytes (max: {max})")]
    FileTooLarge { size: u64, max: u64 },

    #[error("Duration exceeds limit: {duration}s (max: {max}s)")]
    DurationTooLong { duration: f64, max: f64 },

    #[error("No audio stream found")]
    NoAudioStream,

    #[error("No video stream found")]
    NoVideoStream,

    #[error("Corrupted file: {0}")]
    CorruptedFile(String),

    #[error("GPU out of memory")]
    GPUOutOfMemory,

    #[error("Processing timeout after {0}s")]
    Timeout(u64),

    #[error("FFmpeg error: {0}")]
    FFmpegError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Image processing error: {0}")]
    ImageError(String),

    #[error("Other error: {0}")]
    Other(String),
}

impl From<image::ImageError> for ProcessingError {
    fn from(err: image::ImageError) -> Self {
        ProcessingError::ImageError(err.to_string())
    }
}

/// Result type for processing operations
pub type Result<T> = std::result::Result<T, ProcessingError>;

/// Stream type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StreamType {
    Video,
    Audio,
    Subtitle,
}

/// Information about a media stream
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamInfo {
    pub stream_type: StreamType,
    pub codec: String,
    pub bitrate: u64,

    // Video-specific fields
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fps: Option<f64>,

    // Audio-specific fields
    pub sample_rate: Option<u32>,
    pub channels: Option<u8>,
}

/// Complete media file information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaInfo {
    pub format: String,
    pub duration: f64,
    pub streams: Vec<StreamInfo>,
    pub metadata: HashMap<String, String>,
}

impl MediaInfo {
    /// Find the first video stream
    #[must_use]
    pub fn video_stream(&self) -> Option<&StreamInfo> {
        self.streams
            .iter()
            .find(|s| s.stream_type == StreamType::Video)
    }

    /// Find the first audio stream
    #[must_use]
    pub fn audio_stream(&self) -> Option<&StreamInfo> {
        self.streams
            .iter()
            .find(|s| s.stream_type == StreamType::Audio)
    }

    /// Check if the file has video
    #[must_use]
    pub fn has_video(&self) -> bool {
        self.video_stream().is_some()
    }

    /// Check if the file has audio
    #[must_use]
    pub fn has_audio(&self) -> bool {
        self.audio_stream().is_some()
    }
}

/// Keyframe information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keyframe {
    pub timestamp: f64,
    pub frame_number: u64,
    pub hash: u64,
    pub sharpness: f64,
    pub thumbnail_paths: HashMap<String, PathBuf>,
}

/// Scene boundary information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    pub start_time: f64,
    pub end_time: f64,
    pub start_frame: u64,
    pub end_frame: u64,
    pub confidence: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_media_info_accessors() {
        let info = MediaInfo {
            format: "mp4".to_string(),
            duration: 10.0,
            streams: vec![
                StreamInfo {
                    stream_type: StreamType::Video,
                    codec: "h264".to_string(),
                    bitrate: 5_000_000,
                    width: Some(1920),
                    height: Some(1080),
                    fps: Some(30.0),
                    sample_rate: None,
                    channels: None,
                },
                StreamInfo {
                    stream_type: StreamType::Audio,
                    codec: "aac".to_string(),
                    bitrate: 128_000,
                    width: None,
                    height: None,
                    fps: None,
                    sample_rate: Some(48000),
                    channels: Some(2),
                },
            ],
            metadata: HashMap::new(),
        };

        assert!(info.has_video());
        assert!(info.has_audio());
        assert_eq!(info.video_stream().unwrap().codec, "h264");
        assert_eq!(info.audio_stream().unwrap().codec, "aac");
    }
}
