//! Format conversion module using FFmpeg
//!
//! This module provides transcoding of media files to different codecs and containers:
//! - Video codec conversion (H.264, H.265/HEVC, VP9, AV1)
//! - Audio codec conversion (AAC, MP3, Opus, FLAC)
//! - Container format conversion (MP4, MKV, WebM)
//! - Resolution/bitrate adjustments for streaming optimization
//!
//! # Example
//! ```no_run
//! use format_conversion::{convert_format, ConversionConfig, VideoCodec, AudioCodec, Container};
//! use std::path::Path;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let config = ConversionConfig {
//!     video_codec: Some(VideoCodec::H264),
//!     audio_codec: Some(AudioCodec::Aac),
//!     container: Container::Mp4,
//!     video_bitrate: Some("2M".to_string()),
//!     audio_bitrate: Some("128k".to_string()),
//!     width: None,
//!     height: None,
//!     crf: Some(23), // Quality factor for H.264 (lower = better quality)
//! };
//!
//! convert_format(
//!     Path::new("input.mkv"),
//!     Path::new("output.mp4"),
//!     &config
//! )?;
//! # Ok(())
//! # }
//! ```

pub mod plugin;

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;
use thiserror::Error;
use tracing::debug;
use video_audio_common::ProcessingError;

/// Errors specific to format conversion
#[derive(Error, Debug)]
pub enum ConversionError {
    #[error("ffmpeg execution failed: {0}")]
    FfmpegError(String),

    #[error("Unsupported codec or format: {0}")]
    UnsupportedFormat(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

impl From<ConversionError> for ProcessingError {
    fn from(err: ConversionError) -> Self {
        ProcessingError::Other(err.to_string())
    }
}

/// Supported video codecs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VideoCodec {
    /// H.264/AVC - widely compatible, good compression
    H264,
    /// H.265/HEVC - better compression than H.264, less compatible
    H265,
    /// VP9 - open codec, used in WebM
    Vp9,
    /// AV1 - latest open codec, best compression, slow encoding
    Av1,
    /// Copy video stream without re-encoding
    Copy,
}

impl VideoCodec {
    fn to_ffmpeg_str(self) -> &'static str {
        match self {
            VideoCodec::H264 => "libx264",
            VideoCodec::H265 => "libx265",
            VideoCodec::Vp9 => "libvpx-vp9",
            VideoCodec::Av1 => "libaom-av1",
            VideoCodec::Copy => "copy",
        }
    }
}

/// Supported audio codecs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioCodec {
    /// AAC - widely compatible
    Aac,
    /// MP3 - universal compatibility
    Mp3,
    /// Opus - best quality per bitrate, used in WebM
    Opus,
    /// FLAC - lossless compression
    Flac,
    /// Copy audio stream without re-encoding
    Copy,
}

impl AudioCodec {
    fn to_ffmpeg_str(self) -> &'static str {
        match self {
            AudioCodec::Aac => "aac",
            AudioCodec::Mp3 => "libmp3lame",
            AudioCodec::Opus => "libopus",
            AudioCodec::Flac => "flac",
            AudioCodec::Copy => "copy",
        }
    }
}

/// Supported container formats
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Container {
    /// MP4 - universal compatibility (H.264/AAC recommended)
    Mp4,
    /// MKV - supports any codec, large ecosystem
    Mkv,
    /// WebM - open format (VP9/Opus recommended)
    Webm,
    /// MOV - QuickTime format
    Mov,
}

impl Container {
    fn extension(self) -> &'static str {
        match self {
            Container::Mp4 => "mp4",
            Container::Mkv => "mkv",
            Container::Webm => "webm",
            Container::Mov => "mov",
        }
    }

    /// Get FFmpeg format name (may differ from file extension)
    fn to_ffmpeg_format(self) -> &'static str {
        match self {
            Container::Mp4 => "mp4",
            Container::Mkv => "matroska", // FFmpeg uses "matroska" not "mkv"
            Container::Webm => "webm",
            Container::Mov => "mov",
        }
    }
}

/// Preset conversion profiles for common use cases
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Preset {
    /// Web streaming - H.264 MP4, balanced quality/size (CRF 28, 1080p max)
    Web,
    /// Mobile streaming - H.264 MP4, smaller file size (CRF 32, 720p max)
    Mobile,
    /// Archive - H.265 MP4, best compression for storage (CRF 20)
    Archive,
    /// Universal compatibility - H.264 MP4, near-lossless (CRF 18)
    Compatible,
    /// Open web format - VP9 WebM for modern browsers (CRF 30)
    WebOpen,
    /// Low bandwidth - H.264 MP4, aggressive compression (CRF 35, 480p max)
    LowBandwidth,
    /// Audio only - AAC in MP4 container, 128kbps
    AudioOnly,
    /// Container copy - remux without re-encoding (fast, lossless)
    Copy,
}

impl Preset {
    /// Convert preset to ConversionConfig
    pub fn to_config(self) -> ConversionConfig {
        match self {
            Preset::Web => ConversionConfig {
                video_codec: Some(VideoCodec::H264),
                audio_codec: Some(AudioCodec::Aac),
                container: Container::Mp4,
                video_bitrate: None,
                audio_bitrate: Some("128k".to_string()),
                width: None,
                height: Some(1080), // Max 1080p height
                crf: Some(28),
            },
            Preset::Mobile => ConversionConfig {
                video_codec: Some(VideoCodec::H264),
                audio_codec: Some(AudioCodec::Aac),
                container: Container::Mp4,
                video_bitrate: None,
                audio_bitrate: Some("96k".to_string()),
                width: None,
                height: Some(720), // Max 720p height
                crf: Some(32),
            },
            Preset::Archive => ConversionConfig {
                video_codec: Some(VideoCodec::H265),
                audio_codec: Some(AudioCodec::Aac),
                container: Container::Mp4,
                video_bitrate: None,
                audio_bitrate: Some("192k".to_string()),
                width: None,
                height: None,
                crf: Some(20), // High quality
            },
            Preset::Compatible => ConversionConfig {
                video_codec: Some(VideoCodec::H264),
                audio_codec: Some(AudioCodec::Aac),
                container: Container::Mp4,
                video_bitrate: None,
                audio_bitrate: Some("192k".to_string()),
                width: None,
                height: None,
                crf: Some(18), // Near-lossless
            },
            Preset::WebOpen => ConversionConfig {
                video_codec: Some(VideoCodec::Vp9),
                audio_codec: Some(AudioCodec::Opus),
                container: Container::Webm,
                video_bitrate: None,
                audio_bitrate: Some("96k".to_string()),
                width: None,
                height: None,
                crf: Some(30),
            },
            Preset::LowBandwidth => ConversionConfig {
                video_codec: Some(VideoCodec::H264),
                audio_codec: Some(AudioCodec::Aac),
                container: Container::Mp4,
                video_bitrate: None,
                audio_bitrate: Some("64k".to_string()),
                width: None,
                height: Some(480), // Max 480p height
                crf: Some(35),
            },
            Preset::AudioOnly => ConversionConfig {
                video_codec: None, // No video
                audio_codec: Some(AudioCodec::Aac),
                container: Container::Mp4,
                video_bitrate: None,
                audio_bitrate: Some("128k".to_string()),
                width: None,
                height: None,
                crf: None,
            },
            Preset::Copy => ConversionConfig {
                video_codec: Some(VideoCodec::Copy),
                audio_codec: Some(AudioCodec::Copy),
                container: Container::Mp4,
                video_bitrate: None,
                audio_bitrate: None,
                width: None,
                height: None,
                crf: None,
            },
        }
    }

    /// Get human-readable description of preset
    pub fn description(self) -> &'static str {
        match self {
            Preset::Web => "Web streaming - H.264 MP4, balanced quality/size (CRF 28, max 1080p)",
            Preset::Mobile => "Mobile streaming - H.264 MP4, smaller file size (CRF 32, max 720p)",
            Preset::Archive => {
                "Archive - H.265 MP4, best compression for storage (CRF 20, high quality)"
            }
            Preset::Compatible => "Universal compatibility - H.264 MP4, near-lossless (CRF 18)",
            Preset::WebOpen => "Open web format - VP9 WebM for modern browsers (CRF 30)",
            Preset::LowBandwidth => {
                "Low bandwidth - H.264 MP4, aggressive compression (CRF 35, max 480p)"
            }
            Preset::AudioOnly => "Audio only - AAC in MP4 container, 128kbps",
            Preset::Copy => "Container copy - remux without re-encoding (fast, lossless)",
        }
    }
}

/// Configuration for format conversion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionConfig {
    /// Target video codec (None = no video or copy)
    pub video_codec: Option<VideoCodec>,
    /// Target audio codec (None = no audio or copy)
    pub audio_codec: Option<AudioCodec>,
    /// Target container format
    pub container: Container,
    /// Video bitrate (e.g., "2M", "500k") - None for CRF mode
    pub video_bitrate: Option<String>,
    /// Audio bitrate (e.g., "128k", "192k")
    pub audio_bitrate: Option<String>,
    /// Target width (None = keep original)
    pub width: Option<u32>,
    /// Target height (None = keep original)
    pub height: Option<u32>,
    /// Constant Rate Factor for quality (lower = better, 0-51 for H.264, 23 is default)
    pub crf: Option<u32>,
}

impl Default for ConversionConfig {
    fn default() -> Self {
        Self {
            video_codec: Some(VideoCodec::H264),
            audio_codec: Some(AudioCodec::Aac),
            container: Container::Mp4,
            video_bitrate: None,
            audio_bitrate: Some("128k".to_string()),
            width: None,
            height: None,
            crf: Some(23), // H.264 default
        }
    }
}

/// Result of format conversion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionResult {
    /// Path to output file
    pub output_path: String,
    /// Configuration used
    pub config: ConversionConfig,
    /// File size before conversion (bytes)
    pub input_size: u64,
    /// File size after conversion (bytes)
    pub output_size: u64,
    /// Compression ratio (output_size / input_size)
    pub compression_ratio: f64,
}

/// Convert a media file to a different format using FFmpeg
///
/// # Arguments
/// * `input_path` - Path to input media file
/// * `output_path` - Path to write output file
/// * `config` - Conversion configuration
///
/// # Returns
/// Conversion result with output file information
pub fn convert_format(
    input_path: &Path,
    output_path: &Path,
    config: &ConversionConfig,
) -> Result<ConversionResult, ConversionError> {
    if !input_path.exists() {
        return Err(ConversionError::FileNotFound(
            input_path.display().to_string(),
        ));
    }

    debug!(
        "Converting {} to {} (codec: {:?}/{:?}, container: {:?})",
        input_path.display(),
        output_path.display(),
        config.video_codec,
        config.audio_codec,
        config.container
    );

    // Get input file size
    let input_size = std::fs::metadata(input_path)?.len();

    // Build FFmpeg command
    let mut cmd = Command::new("ffmpeg");
    cmd.args(["-y", "-i"]) // -y = overwrite output
        .arg(input_path);

    // Video codec
    if let Some(video_codec) = config.video_codec {
        cmd.args(["-c:v", video_codec.to_ffmpeg_str()]);

        // CRF mode (quality-based) or bitrate mode
        if video_codec != VideoCodec::Copy {
            if let Some(crf) = config.crf {
                cmd.args(["-crf", &crf.to_string()]);
            } else if let Some(bitrate) = &config.video_bitrate {
                cmd.args(["-b:v", bitrate]);
            }

            // Preset for encoding speed/quality tradeoff
            match video_codec {
                VideoCodec::H264 | VideoCodec::H265 => {
                    cmd.args(["-preset", "medium"]);
                }
                VideoCodec::Vp9 => {
                    cmd.args(["-deadline", "good"]);
                }
                _ => {}
            }
        }
    } else {
        cmd.args(["-vn"]); // No video
    }

    // Audio codec
    if let Some(audio_codec) = config.audio_codec {
        cmd.args(["-c:a", audio_codec.to_ffmpeg_str()]);

        if audio_codec != AudioCodec::Copy {
            if let Some(bitrate) = &config.audio_bitrate {
                cmd.args(["-b:a", bitrate]);
            }
        }
    } else {
        cmd.args(["-an"]); // No audio
    }

    // Resolution scaling
    match (config.width, config.height) {
        (Some(width), Some(height)) => {
            // Both width and height specified
            cmd.args(["-vf", &format!("scale={}:{}", width, height)]);
        }
        (Some(width), None) => {
            // Width only - preserve aspect ratio
            cmd.args(["-vf", &format!("scale={}:-2", width)]);
        }
        (None, Some(height)) => {
            // Height only - preserve aspect ratio, downscale only
            cmd.args(["-vf", &format!("scale=-2:'min({},ih)'", height)]);
        }
        (None, None) => {
            // No scaling
        }
    }

    // Output format
    cmd.args(["-f", config.container.to_ffmpeg_format()]);
    cmd.arg(output_path);

    debug!("FFmpeg command: {:?}", cmd);

    // Execute FFmpeg
    let output = cmd
        .output()
        .map_err(|e| ConversionError::FfmpegError(format!("Failed to execute ffmpeg: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ConversionError::FfmpegError(format!(
            "ffmpeg failed: {}",
            stderr
        )));
    }

    // Get output file size
    let output_size = std::fs::metadata(output_path)?.len();
    let compression_ratio = output_size as f64 / input_size as f64;

    debug!(
        "Conversion complete: {} -> {} ({:.1}% size)",
        input_size,
        output_size,
        compression_ratio * 100.0
    );

    Ok(ConversionResult {
        output_path: output_path.display().to_string(),
        config: config.clone(),
        input_size,
        output_size,
        compression_ratio,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_video_codec_to_ffmpeg() {
        assert_eq!(VideoCodec::H264.to_ffmpeg_str(), "libx264");
        assert_eq!(VideoCodec::H265.to_ffmpeg_str(), "libx265");
        assert_eq!(VideoCodec::Vp9.to_ffmpeg_str(), "libvpx-vp9");
        assert_eq!(VideoCodec::Copy.to_ffmpeg_str(), "copy");
    }

    #[test]
    fn test_audio_codec_to_ffmpeg() {
        assert_eq!(AudioCodec::Aac.to_ffmpeg_str(), "aac");
        assert_eq!(AudioCodec::Mp3.to_ffmpeg_str(), "libmp3lame");
        assert_eq!(AudioCodec::Opus.to_ffmpeg_str(), "libopus");
        assert_eq!(AudioCodec::Copy.to_ffmpeg_str(), "copy");
    }

    #[test]
    fn test_container_extension() {
        assert_eq!(Container::Mp4.extension(), "mp4");
        assert_eq!(Container::Mkv.extension(), "mkv");
        assert_eq!(Container::Webm.extension(), "webm");
    }

    #[test]
    fn test_container_to_ffmpeg_format() {
        assert_eq!(Container::Mp4.to_ffmpeg_format(), "mp4");
        assert_eq!(Container::Mkv.to_ffmpeg_format(), "matroska");
        assert_eq!(Container::Webm.to_ffmpeg_format(), "webm");
        assert_eq!(Container::Mov.to_ffmpeg_format(), "mov");
    }

    #[test]
    fn test_config_default() {
        let config = ConversionConfig::default();
        assert_eq!(config.video_codec, Some(VideoCodec::H264));
        assert_eq!(config.audio_codec, Some(AudioCodec::Aac));
        assert_eq!(config.container, Container::Mp4);
        assert_eq!(config.crf, Some(23));
    }

    #[test]
    fn test_preset_web() {
        let config = Preset::Web.to_config();
        assert_eq!(config.video_codec, Some(VideoCodec::H264));
        assert_eq!(config.audio_codec, Some(AudioCodec::Aac));
        assert_eq!(config.container, Container::Mp4);
        assert_eq!(config.crf, Some(28));
        assert_eq!(config.height, Some(1080));
        assert_eq!(config.audio_bitrate, Some("128k".to_string()));
    }

    #[test]
    fn test_preset_mobile() {
        let config = Preset::Mobile.to_config();
        assert_eq!(config.video_codec, Some(VideoCodec::H264));
        assert_eq!(config.crf, Some(32));
        assert_eq!(config.height, Some(720));
        assert_eq!(config.audio_bitrate, Some("96k".to_string()));
    }

    #[test]
    fn test_preset_archive() {
        let config = Preset::Archive.to_config();
        assert_eq!(config.video_codec, Some(VideoCodec::H265));
        assert_eq!(config.crf, Some(20));
        assert_eq!(config.height, None); // No scaling
        assert_eq!(config.audio_bitrate, Some("192k".to_string()));
    }

    #[test]
    fn test_preset_audio_only() {
        let config = Preset::AudioOnly.to_config();
        assert_eq!(config.video_codec, None); // No video
        assert_eq!(config.audio_codec, Some(AudioCodec::Aac));
        assert_eq!(config.container, Container::Mp4);
    }

    #[test]
    fn test_preset_copy() {
        let config = Preset::Copy.to_config();
        assert_eq!(config.video_codec, Some(VideoCodec::Copy));
        assert_eq!(config.audio_codec, Some(AudioCodec::Copy));
        assert_eq!(config.crf, None);
    }

    #[test]
    fn test_preset_web_open() {
        let config = Preset::WebOpen.to_config();
        assert_eq!(config.video_codec, Some(VideoCodec::Vp9));
        assert_eq!(config.audio_codec, Some(AudioCodec::Opus));
        assert_eq!(config.container, Container::Webm);
    }

    #[test]
    fn test_preset_descriptions() {
        // Verify all presets have non-empty descriptions
        assert!(!Preset::Web.description().is_empty());
        assert!(!Preset::Mobile.description().is_empty());
        assert!(!Preset::Archive.description().is_empty());
        assert!(!Preset::Compatible.description().is_empty());
        assert!(!Preset::WebOpen.description().is_empty());
        assert!(!Preset::LowBandwidth.description().is_empty());
        assert!(!Preset::AudioOnly.description().is_empty());
        assert!(!Preset::Copy.description().is_empty());
    }
}
