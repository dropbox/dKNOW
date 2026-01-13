//! Subtitle extraction module using FFmpeg CLI
//!
//! This module extracts embedded subtitles from video files in various formats
//! (SRT, ASS, VTT, mov_text, etc.) and outputs them in a unified JSON format.
//!
//! # Features
//! - Extracts embedded subtitle streams from MP4, MKV, MOV, AVI, WEBM
//! - Supports multiple subtitle formats (SRT, ASS/SSA, WebVTT, mov_text, etc.)
//! - Preserves timing information (start/end timestamps)
//! - Handles multiple subtitle tracks with language metadata
//! - Uses FFmpeg CLI for maximum compatibility
//!
//! # Example
//! ```no_run
//! use video_audio_subtitle::{extract_subtitles, SubtitleConfig};
//! use std::path::Path;
//!
//! # fn main() -> anyhow::Result<()> {
//! let video_path = Path::new("video_with_subs.mp4");
//! let config = SubtitleConfig::default();
//! let subtitles = extract_subtitles(video_path, config)?;
//!
//! for subtitle in subtitles {
//!     println!("[{:.2}s -> {:.2}s] {}",
//!              subtitle.start_time, subtitle.end_time, subtitle.text);
//! }
//! # Ok(())
//! # }
//! ```

pub mod plugin;

use ffmpeg_next as ffmpeg;
use ffmpeg_next::media::Type;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;
use thiserror::Error;
use tracing::{debug, info, warn};
use video_audio_common::ProcessingError;

/// Subtitle extraction errors
#[derive(Debug, Error)]
pub enum SubtitleError {
    #[error("FFmpeg error: {0}")]
    FFmpeg(#[from] ffmpeg::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("No subtitle streams found")]
    NoSubtitles,

    #[error("Invalid subtitle track: {0}")]
    InvalidTrack(usize),

    #[error("Subtitle decoding failed: {0}")]
    DecodingFailed(String),

    #[error("UTF-8 decoding error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),
}

/// Configuration for subtitle extraction
#[derive(Debug, Clone, Default)]
pub struct SubtitleConfig {
    /// Extract only specific subtitle track (0-indexed), None = all tracks
    pub track_index: Option<usize>,

    /// Filter by language code (e.g., "eng", "spa"), None = all languages
    pub language: Option<String>,

    /// Include subtitle formatting (ASS/SSA only), default: false (plain text)
    pub include_formatting: bool,
}

/// A single subtitle entry with timing and text
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleEntry {
    /// Start time in seconds
    pub start_time: f64,

    /// End time in seconds
    pub end_time: f64,

    /// Subtitle text content (plain text or formatted)
    pub text: String,

    /// Track index this subtitle belongs to
    pub track_index: usize,
}

/// Subtitle track metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleTrack {
    /// Track index (0-indexed)
    pub index: usize,

    /// Codec name (e.g., "subrip", "ass", "mov_text")
    pub codec: String,

    /// Language code (e.g., "eng", "spa")
    pub language: Option<String>,

    /// Whether this is the default track
    pub is_default: bool,

    /// All subtitle entries for this track
    pub entries: Vec<SubtitleEntry>,
}

/// Complete subtitle extraction result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subtitles {
    /// All subtitle tracks found
    pub tracks: Vec<SubtitleTrack>,

    /// Total number of subtitle entries across all tracks
    pub total_entries: usize,
}

/// Extract subtitles from a video file
pub fn extract_subtitles(
    video_path: &Path,
    config: SubtitleConfig,
) -> Result<Subtitles, SubtitleError> {
    info!("Extracting subtitles from {}", video_path.display());

    // Initialize FFmpeg (idempotent)
    ffmpeg::init().map_err(SubtitleError::FFmpeg)?;

    // Open input file to detect subtitle streams
    let input = ffmpeg::format::input(video_path)?;

    // Find all subtitle streams
    let subtitle_streams: Vec<_> = input
        .streams()
        .filter(|stream| stream.parameters().medium() == Type::Subtitle)
        .collect();

    if subtitle_streams.is_empty() {
        return Err(SubtitleError::NoSubtitles);
    }

    info!("Found {} subtitle stream(s)", subtitle_streams.len());

    let mut tracks = Vec::with_capacity(subtitle_streams.len());

    for (track_idx, stream) in subtitle_streams.iter().enumerate() {
        // Apply filters
        if let Some(wanted_idx) = config.track_index {
            if track_idx != wanted_idx {
                continue;
            }
        }

        let language = stream.metadata().get("language").map(|s| s.to_string());

        if let Some(ref wanted_lang) = config.language {
            if language.as_ref() != Some(wanted_lang) {
                continue;
            }
        }

        let codec_name = stream.parameters().id().name();
        let stream_index = stream.index();

        // Check if this is the default track
        let is_default = stream
            .disposition()
            .contains(ffmpeg::format::stream::Disposition::DEFAULT);

        debug!(
            "Processing subtitle track {}: codec={}, language={:?}, stream_index={}, default={}",
            track_idx, codec_name, language, stream_index, is_default
        );

        // Extract subtitle entries from this track using FFmpeg CLI
        let entries = extract_track_subtitles_cli(video_path, stream_index, track_idx)?;

        if !entries.is_empty() {
            tracks.push(SubtitleTrack {
                index: track_idx,
                codec: codec_name.to_string(),
                language,
                is_default,
                entries,
            });
        }
    }

    if tracks.is_empty() {
        warn!("No subtitle tracks matched the filter criteria or contained valid entries");
        return Err(SubtitleError::NoSubtitles);
    }

    let total_entries = tracks.iter().map(|t| t.entries.len()).sum();

    info!(
        "Extracted {} subtitle entries from {} track(s)",
        total_entries,
        tracks.len()
    );

    Ok(Subtitles {
        tracks,
        total_entries,
    })
}

/// Extract subtitle entries from a specific track using FFmpeg CLI
fn extract_track_subtitles_cli(
    video_path: &Path,
    stream_index: usize,
    track_index: usize,
) -> Result<Vec<SubtitleEntry>, SubtitleError> {
    // Create temporary file for SRT output
    let temp_dir = std::env::temp_dir();
    let srt_path = temp_dir.join(format!("subtitles_{}.srt", stream_index));

    // Extract subtitle stream to SRT format using FFmpeg
    let output = Command::new("ffmpeg")
        .arg("-i")
        .arg(video_path)
        .arg("-map")
        .arg(format!("0:{}", stream_index))
        .arg("-c:s")
        .arg("srt") // Convert to SRT format
        .arg("-y") // Overwrite output file
        .arg(&srt_path)
        .output()
        .map_err(SubtitleError::Io)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!(
            "FFmpeg subtitle extraction failed for stream {}: {}",
            stream_index, stderr
        );
        return Ok(Vec::new()); // Return empty rather than error (might be bitmap subs)
    }

    // Parse SRT file
    let entries = parse_srt_file(&srt_path, track_index)?;

    // Clean up temporary file
    std::fs::remove_file(&srt_path).ok(); // Ignore cleanup errors

    debug!(
        "Extracted {} subtitle entries from track {}",
        entries.len(),
        track_index
    );

    Ok(entries)
}

/// Parse an SRT subtitle file
fn parse_srt_file(
    srt_path: &Path,
    track_index: usize,
) -> Result<Vec<SubtitleEntry>, SubtitleError> {
    let content = std::fs::read_to_string(srt_path)?;

    // SRT format:
    // 1
    // 00:00:00,000 --> 00:00:02,500
    // Subtitle text
    //
    // 2
    // ...

    let blocks: Vec<&str> = content.split("\n\n").collect();
    let mut entries = Vec::with_capacity(blocks.len());

    for block in blocks {
        let lines: Vec<&str> = block.trim().lines().collect();

        if lines.len() < 3 {
            continue; // Invalid block
        }

        // Parse timestamp line (line 1, index 0 is sequence number)
        if let Some(timestamps) = lines.get(1) {
            if let Some((start_str, end_str)) = timestamps.split_once(" --> ") {
                let start_time = parse_srt_timestamp(start_str.trim());
                let end_time = parse_srt_timestamp(end_str.trim());

                // Collect text lines (line 2+)
                let text = lines[2..].join("\n");

                entries.push(SubtitleEntry {
                    start_time,
                    end_time,
                    text,
                    track_index,
                });
            }
        }
    }

    Ok(entries)
}

/// Parse SRT timestamp (HH:MM:SS,mmm) to seconds
fn parse_srt_timestamp(timestamp: &str) -> f64 {
    // Format: HH:MM:SS,mmm
    let parts: Vec<&str> = timestamp.split(&[':', ',']).collect();

    if parts.len() != 4 {
        return 0.0;
    }

    let hours: f64 = parts[0].parse().unwrap_or(0.0);
    let minutes: f64 = parts[1].parse().unwrap_or(0.0);
    let seconds: f64 = parts[2].parse().unwrap_or(0.0);
    let millis: f64 = parts[3].parse().unwrap_or(0.0);

    hours * 3600.0 + minutes * 60.0 + seconds + (millis / 1000.0)
}

impl From<SubtitleError> for ProcessingError {
    fn from(err: SubtitleError) -> Self {
        ProcessingError::Other(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subtitle_config_default() {
        let config = SubtitleConfig::default();
        assert_eq!(config.track_index, None);
        assert_eq!(config.language, None);
        assert!(!config.include_formatting);
    }

    #[test]
    fn test_parse_srt_timestamp() {
        assert_eq!(parse_srt_timestamp("00:00:00,000"), 0.0);
        assert_eq!(parse_srt_timestamp("00:00:02,500"), 2.5);
        assert_eq!(parse_srt_timestamp("00:01:30,250"), 90.25);
        assert_eq!(parse_srt_timestamp("01:00:00,000"), 3600.0);
    }
}
