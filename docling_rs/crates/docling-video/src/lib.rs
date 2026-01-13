//! Video subtitle extraction for docling
//!
//! This crate provides functionality to extract subtitles from video files
//! (MP4, MKV, MOV, AVI) using `FFmpeg` and parse them using the `subparse` crate.
//!
//! ## Features
//!
//! - **Subtitle extraction**: Extract embedded subtitle tracks from video containers
//! - **Multiple formats**: MP4, MKV, MOV, AVI
//! - **Multiple subtitle formats**: SRT, VTT, ASS, SSA
//! - **Audio transcription** (optional): Extract audio and transcribe using Whisper
//!
//! ## System Requirements
//!
//! - **`FFmpeg`**: Must be installed and available in `PATH`
//!   - macOS: `brew install ffmpeg`
//!   - Ubuntu/Debian: `apt install ffmpeg`
//!   - Windows: Download from <https://ffmpeg.org/download.html>
//!
//! ## Example
//!
//! ```rust,no_run
//! use docling_video::{process_video, VideoProcessingOptions};
//!
//! let result = process_video(
//!     "video.mp4",
//!     VideoProcessingOptions {
//!         extract_subtitles: true,
//!         transcribe_audio: false,
//!         ..Default::default()
//!     }
//! )?;
//!
//! println!("{}", result);
//! # Ok::<(), docling_video::VideoError>(())
//! ```

/// Error types for video processing operations
pub mod error;
/// `FFmpeg` integration for subtitle extraction and audio track handling
pub mod ffmpeg;
/// Subtitle file parsing and formatting utilities
pub mod subtitle;

#[cfg(feature = "transcription")]
pub mod transcription;

pub use error::{Result, VideoError};
pub use ffmpeg::{check_ffmpeg_available, detect_subtitle_tracks, SubtitleTrackInfo};
pub use subtitle::{parse_subtitle_file, SubtitleEntry, SubtitleFile, SubtitleFormat};

use std::fmt::Write;
use std::path::Path;
use tempfile::TempDir;

/// Options for processing video files
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct VideoProcessingOptions {
    /// Extract all subtitle tracks
    pub extract_subtitles: bool,
    /// Transcribe audio track (requires transcription feature)
    pub transcribe_audio: bool,
    /// Preferred subtitle format for extraction (default: srt)
    pub subtitle_format: SubtitleFormat,
    /// Only extract default subtitle track (if false, extracts all tracks)
    pub default_track_only: bool,
}

impl Default for VideoProcessingOptions {
    #[inline]
    fn default() -> Self {
        Self {
            extract_subtitles: true,
            transcribe_audio: false,
            subtitle_format: SubtitleFormat::Srt,
            default_track_only: false,
        }
    }
}

/// Process a video file and extract subtitles and/or audio transcription
///
/// Returns a markdown-formatted string with extracted content.
///
/// # Arguments
///
/// * `video_path` - Path to the video file
/// * `options` - Processing options
///
/// # Errors
///
/// Returns an error if:
/// - `FFmpeg` is not available
/// - Video file cannot be read
/// - No subtitle tracks found (when `extract_subtitles` is true)
/// - Transcription fails (when `transcribe_audio` is true)
#[must_use = "this function returns extracted video content that should be processed"]
pub fn process_video<P: AsRef<Path>>(
    video_path: P,
    options: &VideoProcessingOptions,
) -> Result<String> {
    let video_path = video_path.as_ref();

    // Check FFmpeg availability
    check_ffmpeg_available()?;

    let mut output = String::new();

    // Add video header
    let filename = video_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");
    let _ = writeln!(output, "# Video: {filename}\n");

    // Extract subtitles if requested
    if options.extract_subtitles {
        match extract_all_subtitles(video_path, options) {
            Ok(subtitles_md) => {
                output.push_str(&subtitles_md);
            }
            Err(VideoError::NoSubtitleTracks) => {
                output.push_str("## Subtitles\n\n*No subtitle tracks found*\n\n");
            }
            Err(e) => return Err(e),
        }
    }

    // Transcribe audio if requested (requires transcription feature)
    #[cfg(feature = "transcription")]
    if options.transcribe_audio {
        let transcription_md = transcription::transcribe_video_audio(video_path)?;
        output.push_str(&transcription_md);
    }

    #[cfg(not(feature = "transcription"))]
    if options.transcribe_audio {
        output.push_str("## Audio Transcription\n\n");
        output.push_str("*Transcription not available: Enable the 'transcription' feature*\n\n");
    }

    Ok(output)
}

/// Extract all subtitle tracks from a video file
fn extract_all_subtitles(video_path: &Path, options: &VideoProcessingOptions) -> Result<String> {
    let mut output = String::new();

    // Detect subtitle tracks
    let tracks = detect_subtitle_tracks(video_path)?;

    // Filter tracks if only default track is requested
    let tracks_to_extract: Vec<_> = if options.default_track_only {
        tracks.into_iter().filter(|t| t.is_default).collect()
    } else {
        tracks
    };

    if tracks_to_extract.is_empty() {
        return Err(VideoError::NoSubtitleTracks);
    }

    // Create temporary directory for extracted subtitle files
    let temp_dir = TempDir::new().map_err(|e| VideoError::TempFile(e.to_string()))?;

    // Extract and parse each subtitle track
    for track in tracks_to_extract {
        let subtitle_filename = format!(
            "subtitle_{}.{}",
            track.subtitle_index,
            options.subtitle_format.extension()
        );
        let subtitle_path = temp_dir.path().join(&subtitle_filename);

        // Extract subtitle track
        ffmpeg::extract_subtitle_track(
            video_path,
            track.subtitle_index,
            &subtitle_path,
            options.subtitle_format.extension(),
        )?;

        // Parse subtitle file
        let subtitle_file = parse_subtitle_file(&subtitle_path)?;

        // Format as markdown
        output.push_str(&format_subtitle_track(&track, &subtitle_file));
    }

    Ok(output)
}

/// Format a subtitle track as markdown
fn format_subtitle_track(track: &SubtitleTrackInfo, subtitle_file: &SubtitleFile) -> String {
    let mut output = String::new();

    // Track header
    output.push_str("## Subtitles");
    if let Some(title) = &track.title {
        let _ = write!(output, " - {title}");
    }
    if let Some(lang) = &track.language {
        let _ = write!(output, " ({lang})");
    }
    output.push_str("\n\n");

    // Track metadata
    let _ = writeln!(output, "- **Track:** {}", track.subtitle_index);
    let _ = writeln!(output, "- **Codec:** {}", track.codec);
    if let Some(title) = &track.title {
        let _ = writeln!(output, "- **Title:** {title}");
    }
    if track.is_default {
        output.push_str("- **Default:** Yes\n");
    }
    output.push('\n');

    // Subtitle entries
    for entry in &subtitle_file.entries {
        let start_secs = entry.start_time.as_secs_f64();
        let end_secs = entry.end_time.as_secs_f64();

        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "minutes from seconds always fits in u32"
        )]
        let _ = writeln!(
            output,
            "**[{:02}:{:05.2} - {:02}:{:05.2}]** {}\n",
            (start_secs / 60.0) as u32,
            start_secs % 60.0,
            (end_secs / 60.0) as u32,
            end_secs % 60.0,
            entry.text.trim()
        );
    }

    output
}

/// Process MP4 video file
///
/// Shorthand for `process_video()` with default options.
///
/// # Errors
///
/// See [`process_video`] for error conditions.
#[must_use = "this function returns extracted video content that should be processed"]
pub fn process_mp4<P: AsRef<Path>>(video_path: P) -> Result<String> {
    process_video(video_path, &VideoProcessingOptions::default())
}

/// Process MKV video file
///
/// Shorthand for `process_video()` with default options.
///
/// # Errors
///
/// See [`process_video`] for error conditions.
#[must_use = "this function returns extracted video content that should be processed"]
pub fn process_mkv<P: AsRef<Path>>(video_path: P) -> Result<String> {
    process_video(video_path, &VideoProcessingOptions::default())
}

/// Process MOV video file
///
/// Shorthand for `process_video()` with default options.
///
/// # Errors
///
/// See [`process_video`] for error conditions.
#[must_use = "this function returns extracted video content that should be processed"]
pub fn process_mov<P: AsRef<Path>>(video_path: P) -> Result<String> {
    process_video(video_path, &VideoProcessingOptions::default())
}

/// Process AVI video file
///
/// Shorthand for `process_video()` with default options.
///
/// # Errors
///
/// See [`process_video`] for error conditions.
#[must_use = "this function returns extracted video content that should be processed"]
pub fn process_avi<P: AsRef<Path>>(video_path: P) -> Result<String> {
    process_video(video_path, &VideoProcessingOptions::default())
}

/// Process standalone SRT subtitle file
///
/// Parses an SRT subtitle file and converts it to markdown format.
///
/// # Arguments
///
/// * `srt_path` - Path to the SRT file
///
/// # Returns
///
/// Markdown-formatted string with subtitle content
///
/// # Errors
///
/// Returns an error if:
/// - The file cannot be read (I/O error)
/// - The subtitle format is invalid or cannot be parsed
#[must_use = "this function returns parsed subtitle content that should be processed"]
pub fn process_srt<P: AsRef<Path>>(srt_path: P) -> Result<String> {
    let srt_path = srt_path.as_ref();

    // Parse the SRT file
    let subtitle_file = parse_subtitle_file(srt_path)?;

    // Format as markdown
    let mut output = String::new();

    // Add header with filename
    let filename = srt_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown.srt");
    let _ = writeln!(output, "# Subtitles: {filename}\n");

    // Add subtitle content
    output.push_str(&subtitle::format_as_markdown(&subtitle_file));

    Ok(output)
}

/// Process standalone `WebVTT` subtitle file
///
/// Parses a `WebVTT` subtitle file and converts it to markdown format.
///
/// # Arguments
///
/// * `vtt_path` - Path to the `WebVTT` file
///
/// # Returns
///
/// Markdown-formatted string with subtitle content
///
/// # Errors
///
/// Returns an error if:
/// - The file cannot be read (I/O error)
/// - The subtitle format is invalid or cannot be parsed
#[must_use = "this function returns parsed subtitle content that should be processed"]
pub fn process_webvtt<P: AsRef<Path>>(vtt_path: P) -> Result<String> {
    let vtt_path = vtt_path.as_ref();

    // Parse the WebVTT file (uses same parser as SRT for now)
    let subtitle_file = parse_subtitle_file(vtt_path)?;

    // Format as markdown
    let mut output = String::new();

    // Add header with filename
    let filename = vtt_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown.vtt");
    let _ = writeln!(output, "# Subtitles: {filename}\n");

    // Add subtitle content
    output.push_str(&subtitle::format_as_markdown(&subtitle_file));

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ffmpeg_available() {
        // This test will fail if FFmpeg is not installed
        let result = check_ffmpeg_available();
        match result {
            Ok(version) => {
                println!("FFmpeg version: {version}");
                assert!(version.contains("ffmpeg"));
            }
            Err(e) => {
                eprintln!("Warning: FFmpeg not available: {e}");
                eprintln!("This is expected if FFmpeg is not installed on the system");
            }
        }
    }

    #[test]
    fn test_process_srt() {
        // Test SRT processing with a real test file
        let test_file = "../../test-corpus/subtitles/srt/simple_dialogue.srt";
        let result = process_srt(test_file);

        match result {
            Ok(markdown) => {
                println!("SRT processing succeeded!");
                println!("Markdown output ({} chars):", markdown.len());
                println!("{markdown}");

                // Basic validation
                assert!(markdown.contains("# Subtitles"));
                assert!(markdown.contains("simple_dialogue.srt"));
                assert!(markdown.contains("Hello, how are you"));
                assert!(markdown.len() > 100, "Output too short");
            }
            Err(e) => {
                panic!("SRT processing failed: {e}");
            }
        }
    }
}
