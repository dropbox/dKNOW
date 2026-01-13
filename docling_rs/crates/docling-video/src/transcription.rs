//! Video audio transcription using Whisper
//!
//! This module provides functionality to extract audio from video files
//! and transcribe them using the Whisper speech recognition model.
//!
//! This module is only available when the `transcription` feature is enabled.

use crate::error::{Result, VideoError};
use crate::ffmpeg;
use std::path::Path;
use tempfile::NamedTempFile;

/// Transcribe audio from a video file
///
/// Extracts the audio track from the video file, converts it to WAV format
/// (16kHz mono), and transcribes it using the Whisper model via docling-audio.
///
/// # Arguments
///
/// * `video_path` - Path to the video file
///
/// # Returns
///
/// Markdown-formatted transcription with timestamped segments
///
/// # Errors
///
/// Returns an error if:
/// - Video file has no audio track
/// - Audio extraction fails
/// - Transcription fails
#[must_use = "this function returns transcription results that should be processed"]
pub fn transcribe_video_audio<P: AsRef<Path>>(video_path: P) -> Result<String> {
    let video_path = video_path.as_ref();

    // Check if video has an audio track
    if !ffmpeg::has_audio_track(video_path)? {
        return Err(VideoError::NoAudioTrack);
    }

    // Create temporary file for extracted audio
    let temp_audio = NamedTempFile::new()
        .map_err(|e| VideoError::TempFile(e.to_string()))?
        .into_temp_path();

    // Extract audio to WAV (16kHz mono, 16-bit PCM - required by Whisper)
    ffmpeg::extract_audio_track(video_path, &temp_audio)?;

    // Transcribe audio using docling-audio
    let transcription_result = docling_audio::transcribe_audio(&temp_audio, None)
        .map_err(|e| VideoError::TranscriptionFailed(e.to_string()))?;

    // Format as markdown
    let mut output = String::new();
    output.push_str("## Audio Transcription\n\n");

    // Full transcript
    output.push_str(&transcription_result.text);
    output.push_str("\n\n");

    // Timestamped segments
    if !transcription_result.segments.is_empty() {
        output.push_str("### Segments\n\n");
        for (i, segment) in transcription_result.segments.iter().enumerate() {
            use std::fmt::Write;
            // Cast f64 to u32 is intentional for extracting minutes from timestamp
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let _ = writeln!(
                output,
                "{}. **[{:02}:{:05.2} - {:02}:{:05.2}]** {}",
                i + 1,
                (segment.start / 60.0) as u32,
                segment.start % 60.0,
                (segment.end / 60.0) as u32,
                segment.end % 60.0,
                segment.text.trim()
            );
        }
        output.push('\n');
    }

    Ok(output)
}
