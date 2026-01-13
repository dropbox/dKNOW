//! Metadata extraction module using FFmpeg/ffprobe
//!
//! This module provides extraction of media file metadata including:
//! - Format information (duration, bitrate, size)
//! - Video stream metadata (codec, resolution, frame rate, aspect ratio)
//! - Audio stream metadata (codec, sample rate, channels, bitrate)
//! - Container metadata (EXIF, ID3, etc.)
//! - Creation date, GPS coordinates (if available)
//!
//! # Example
//! ```no_run
//! use video_audio_metadata::{extract_metadata, MetadataConfig};
//! use std::path::Path;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let config = MetadataConfig { include_streams: true };
//! let metadata = extract_metadata(Path::new("video.mp4"), &config)?;
//!
//! println!("Duration: {}s", metadata.format.duration.unwrap_or(0.0));
//! println!("Resolution: {}x{}",
//!     metadata.video_stream.as_ref().map(|v| v.width).unwrap_or(0),
//!     metadata.video_stream.as_ref().map(|v| v.height).unwrap_or(0)
//! );
//! # Ok(())
//! # }
//! ```

pub mod plugin;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use thiserror::Error;
use tracing::debug;
use video_audio_common::ProcessingError;

/// Errors specific to metadata extraction
#[derive(Error, Debug)]
pub enum MetadataError {
    #[error("ffprobe execution failed: {0}")]
    FfprobeError(String),

    #[error("Failed to parse ffprobe output: {0}")]
    ParseError(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

impl From<MetadataError> for ProcessingError {
    fn from(err: MetadataError) -> Self {
        ProcessingError::Other(err.to_string())
    }
}

/// Configuration for metadata extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataConfig {
    /// Include detailed stream information (video/audio codec details)
    pub include_streams: bool,
}

impl Default for MetadataConfig {
    fn default() -> Self {
        Self {
            include_streams: true,
        }
    }
}

/// Format-level metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatMetadata {
    /// File format name (e.g., "mov,mp4,m4a,3gp,3g2,mj2")
    pub format_name: Option<String>,
    /// Long format name (e.g., "QuickTime / MOV")
    pub format_long_name: Option<String>,
    /// Duration in seconds
    pub duration: Option<f64>,
    /// File size in bytes
    pub size: Option<u64>,
    /// Overall bitrate in bits/second
    pub bit_rate: Option<u64>,
    /// Number of streams
    pub nb_streams: usize,
    /// Format-level tags (e.g., creation_time, title, artist)
    pub tags: HashMap<String, String>,
}

/// Video stream metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoStreamMetadata {
    /// Video codec name (e.g., "h264", "vp9")
    pub codec_name: Option<String>,
    /// Codec long name (e.g., "H.264 / AVC / MPEG-4 AVC")
    pub codec_long_name: Option<String>,
    /// Video width in pixels
    pub width: u32,
    /// Video height in pixels
    pub height: u32,
    /// Frame rate (frames per second)
    pub fps: Option<f64>,
    /// Aspect ratio (e.g., "16:9")
    pub aspect_ratio: Option<String>,
    /// Pixel format (e.g., "yuv420p")
    pub pix_fmt: Option<String>,
    /// Bitrate in bits/second
    pub bit_rate: Option<u64>,
}

/// Audio stream metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioStreamMetadata {
    /// Audio codec name (e.g., "aac", "mp3")
    pub codec_name: Option<String>,
    /// Codec long name (e.g., "AAC (Advanced Audio Coding)")
    pub codec_long_name: Option<String>,
    /// Sample rate in Hz
    pub sample_rate: u32,
    /// Number of audio channels
    pub channels: u32,
    /// Channel layout (e.g., "stereo", "5.1")
    pub channel_layout: Option<String>,
    /// Bitrate in bits/second
    pub bit_rate: Option<u64>,
}

/// Complete metadata result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaMetadata {
    /// Format-level metadata
    pub format: FormatMetadata,
    /// Video stream metadata (if video stream exists)
    pub video_stream: Option<VideoStreamMetadata>,
    /// Audio stream metadata (if audio stream exists)
    pub audio_stream: Option<AudioStreamMetadata>,
    /// Configuration used for extraction
    pub config: MetadataConfig,
}

/// Extract metadata from a media file using ffprobe
///
/// # Arguments
/// * `file_path` - Path to the media file
/// * `config` - Configuration for metadata extraction
///
/// # Returns
/// Complete metadata including format and stream information
pub fn extract_metadata(
    file_path: &Path,
    config: &MetadataConfig,
) -> Result<MediaMetadata, MetadataError> {
    if !file_path.exists() {
        return Err(MetadataError::FileNotFound(file_path.display().to_string()));
    }

    debug!("Extracting metadata from: {}", file_path.display());

    // Use ffprobe to extract metadata as JSON
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "quiet",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
        ])
        .arg(file_path)
        .output()
        .map_err(|e| MetadataError::FfprobeError(format!("Failed to execute ffprobe: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(MetadataError::FfprobeError(format!(
            "ffprobe failed: {}",
            stderr
        )));
    }

    let json_output = String::from_utf8_lossy(&output.stdout);
    let ffprobe_result: FfprobeOutput = serde_json::from_str(&json_output)
        .map_err(|e| MetadataError::ParseError(format!("Failed to parse JSON: {}", e)))?;

    // Parse format metadata
    let format = parse_format_metadata(&ffprobe_result.format);

    // Parse stream metadata
    let video_stream = if config.include_streams {
        parse_video_stream(&ffprobe_result.streams)
    } else {
        None
    };

    let audio_stream = if config.include_streams {
        parse_audio_stream(&ffprobe_result.streams)
    } else {
        None
    };

    Ok(MediaMetadata {
        format,
        video_stream,
        audio_stream,
        config: config.clone(),
    })
}

// ────────── Internal ffprobe JSON parsing ──────────

#[derive(Debug, Deserialize)]
struct FfprobeOutput {
    format: FfprobeFormat,
    streams: Vec<FfprobeStream>,
}

#[derive(Debug, Deserialize)]
struct FfprobeFormat {
    format_name: Option<String>,
    format_long_name: Option<String>,
    duration: Option<String>,
    size: Option<String>,
    bit_rate: Option<String>,
    nb_streams: Option<usize>,
    tags: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize)]
struct FfprobeStream {
    codec_type: Option<String>,
    codec_name: Option<String>,
    codec_long_name: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    r_frame_rate: Option<String>,
    display_aspect_ratio: Option<String>,
    pix_fmt: Option<String>,
    sample_rate: Option<String>,
    channels: Option<u32>,
    channel_layout: Option<String>,
    bit_rate: Option<String>,
}

fn parse_format_metadata(format: &FfprobeFormat) -> FormatMetadata {
    FormatMetadata {
        format_name: format.format_name.clone(),
        format_long_name: format.format_long_name.clone(),
        duration: format.duration.as_ref().and_then(|d| d.parse::<f64>().ok()),
        size: format.size.as_ref().and_then(|s| s.parse::<u64>().ok()),
        bit_rate: format.bit_rate.as_ref().and_then(|b| b.parse::<u64>().ok()),
        nb_streams: format.nb_streams.unwrap_or(0),
        tags: format.tags.clone().unwrap_or_default(),
    }
}

fn parse_video_stream(streams: &[FfprobeStream]) -> Option<VideoStreamMetadata> {
    streams
        .iter()
        .find(|s| s.codec_type.as_deref() == Some("video"))
        .map(|stream| {
            let fps = stream.r_frame_rate.as_ref().and_then(|fps_str| {
                // Parse "30000/1001" format
                let parts: Vec<&str> = fps_str.split('/').collect();
                if parts.len() == 2 {
                    let num = parts[0].parse::<f64>().ok()?;
                    let den = parts[1].parse::<f64>().ok()?;
                    Some(num / den)
                } else {
                    fps_str.parse::<f64>().ok()
                }
            });

            VideoStreamMetadata {
                codec_name: stream.codec_name.clone(),
                codec_long_name: stream.codec_long_name.clone(),
                width: stream.width.unwrap_or(0),
                height: stream.height.unwrap_or(0),
                fps,
                aspect_ratio: stream.display_aspect_ratio.clone(),
                pix_fmt: stream.pix_fmt.clone(),
                bit_rate: stream.bit_rate.as_ref().and_then(|b| b.parse::<u64>().ok()),
            }
        })
}

fn parse_audio_stream(streams: &[FfprobeStream]) -> Option<AudioStreamMetadata> {
    streams
        .iter()
        .find(|s| s.codec_type.as_deref() == Some("audio"))
        .map(|stream| AudioStreamMetadata {
            codec_name: stream.codec_name.clone(),
            codec_long_name: stream.codec_long_name.clone(),
            sample_rate: stream
                .sample_rate
                .as_ref()
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(0),
            channels: stream.channels.unwrap_or(0),
            channel_layout: stream.channel_layout.clone(),
            bit_rate: stream.bit_rate.as_ref().and_then(|b| b.parse::<u64>().ok()),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_config_default() {
        let config = MetadataConfig::default();
        assert!(config.include_streams);
    }

    #[test]
    fn test_parse_fps_fraction() {
        // Test that we can parse "30000/1001" (29.97 fps NTSC)
        let fps_str = "30000/1001";
        let parts: Vec<&str> = fps_str.split('/').collect();
        assert_eq!(parts.len(), 2);
        let num = parts[0].parse::<f64>().unwrap();
        let den = parts[1].parse::<f64>().unwrap();
        let fps = num / den;
        assert!((fps - 29.97).abs() < 0.01);
    }
}
