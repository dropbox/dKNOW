//! Audio format backend for docling-core
//!
//! Processes WAV and MP3 audio files into markdown documents.
//! Supports two modes:
//! - Metadata-only: Extract audio file information (duration, sample rate, channels)
//! - Transcription: Convert speech to text using Whisper (requires `transcription` feature)

use std::fmt::Write;
use std::path::Path;

use crate::error::{DoclingError, Result};

/// Number of seconds in one minute (for time formatting).
#[cfg(feature = "transcription")]
const SECONDS_PER_MINUTE: f64 = 60.0;

/// Process a WAV audio file into markdown
///
/// # Behavior
///
/// - Without `transcription` feature: Returns audio metadata (duration, sample rate, channels)
/// - With `transcription` feature: Returns full transcription + metadata
///
/// # Arguments
///
/// * `path` - Path to the WAV file
///
/// # Returns
///
/// Returns markdown document with audio information and optional transcription.
///
/// # Errors
///
/// Returns an error if the file cannot be read or if WAV parsing fails.
///
/// # Examples
///
/// ```no_run
/// use docling_core::audio::process_wav;
///
/// let markdown = process_wav("meeting.wav")?;
/// println!("{}", markdown);
/// # Ok::<(), docling_core::error::DoclingError>(())
/// ```
#[must_use = "this function returns the extracted markdown content"]
pub fn process_wav<P: AsRef<Path>>(path: P) -> Result<String> {
    let path = path.as_ref();

    // Parse WAV file to get metadata
    let wav_info = docling_audio::parse_wav(path)
        .map_err(|e| DoclingError::ConversionError(format!("Failed to parse WAV: {e}")))?;

    // Start building markdown output
    let mut markdown = String::new();

    // Add title
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("audio.wav");
    let _ = writeln!(markdown, "# Audio: {filename}\n");

    // Add metadata
    markdown.push_str("## Audio Information\n\n");
    markdown.push_str("- **Format:** WAV\n");
    let _ = writeln!(
        markdown,
        "- **Duration:** {:.2} seconds",
        wav_info.duration_secs
    );
    let _ = writeln!(markdown, "- **Sample Rate:** {} Hz", wav_info.sample_rate);
    let _ = writeln!(markdown, "- **Channels:** {}", wav_info.channels);
    let _ = writeln!(markdown, "- **Bit Depth:** {} bits", wav_info.bit_depth);
    markdown.push('\n');

    // Try transcription if feature is enabled
    #[cfg(feature = "transcription")]
    {
        match docling_audio::transcribe_audio(path, None) {
            Ok(transcript) => {
                markdown.push_str("## Transcription\n\n");
                markdown.push_str(&transcript.text);
                markdown.push_str("\n\n");

                if !transcript.segments.is_empty() {
                    markdown.push_str("### Segments\n\n");
                    for (i, segment) in transcript.segments.iter().enumerate() {
                        let start_min = (segment.start / SECONDS_PER_MINUTE).floor();
                        let start_sec = segment.start % SECONDS_PER_MINUTE;
                        let end_min = (segment.end / SECONDS_PER_MINUTE).floor();
                        let end_sec = segment.end % SECONDS_PER_MINUTE;

                        let _ = writeln!(
                            markdown,
                            "{}. **[{:02.0}:{:05.2} - {:02.0}:{:05.2}]** {}",
                            i + 1,
                            start_min,
                            start_sec,
                            end_min,
                            end_sec,
                            segment.text
                        );
                    }
                    markdown.push('\n');
                }
            }
            Err(e) => {
                // Transcription failed, but metadata is still valid
                let _ = writeln!(markdown, "*Transcription not available: {e}*\n");
            }
        }
    }

    #[cfg(not(feature = "transcription"))]
    {
        markdown.push_str(
            "*Transcription not available. Enable the `transcription` feature to transcribe audio.*\n\n",
        );
    }

    Ok(markdown)
}

/// Process an MP3 audio file into markdown
///
/// # Behavior
///
/// - Without `transcription` feature: Returns audio metadata (duration, sample rate, channels)
/// - With `transcription` feature: Returns full transcription + metadata
///
/// # Arguments
///
/// * `path` - Path to the MP3 file
///
/// # Returns
///
/// Returns markdown document with audio information and optional transcription.
///
/// # Errors
///
/// Returns an error if the file cannot be read or if MP3 parsing fails.
///
/// # Examples
///
/// ```no_run
/// use docling_core::audio::process_mp3;
///
/// let markdown = process_mp3("podcast.mp3")?;
/// println!("{}", markdown);
/// # Ok::<(), docling_core::error::DoclingError>(())
/// ```
#[must_use = "this function returns the extracted markdown content"]
pub fn process_mp3<P: AsRef<Path>>(path: P) -> Result<String> {
    let path = path.as_ref();

    // Parse MP3 file to get metadata
    let mp3_info = docling_audio::parse_mp3(path)
        .map_err(|e| DoclingError::ConversionError(format!("Failed to parse MP3: {e}")))?;

    // Start building markdown output
    let mut markdown = String::new();

    // Add title
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("audio.mp3");
    let _ = writeln!(markdown, "# Audio: {filename}\n");

    // Add metadata
    markdown.push_str("## Audio Information\n\n");
    markdown.push_str("- **Format:** MP3\n");
    let _ = writeln!(
        markdown,
        "- **Duration:** {:.2} seconds",
        mp3_info.duration_secs
    );
    let _ = writeln!(markdown, "- **Sample Rate:** {} Hz", mp3_info.sample_rate);
    let _ = writeln!(markdown, "- **Channels:** {}", mp3_info.channels);
    markdown.push('\n');

    // Try transcription if feature is enabled
    #[cfg(feature = "transcription")]
    {
        match docling_audio::transcribe_audio(path, None) {
            Ok(transcript) => {
                markdown.push_str("## Transcription\n\n");
                markdown.push_str(&transcript.text);
                markdown.push_str("\n\n");

                if !transcript.segments.is_empty() {
                    markdown.push_str("### Segments\n\n");
                    for (i, segment) in transcript.segments.iter().enumerate() {
                        let start_min = (segment.start / SECONDS_PER_MINUTE).floor();
                        let start_sec = segment.start % SECONDS_PER_MINUTE;
                        let end_min = (segment.end / SECONDS_PER_MINUTE).floor();
                        let end_sec = segment.end % SECONDS_PER_MINUTE;

                        let _ = writeln!(
                            markdown,
                            "{}. **[{:02.0}:{:05.2} - {:02.0}:{:05.2}]** {}",
                            i + 1,
                            start_min,
                            start_sec,
                            end_min,
                            end_sec,
                            segment.text
                        );
                    }
                    markdown.push('\n');
                }
            }
            Err(e) => {
                // Transcription failed, but metadata is still valid
                let _ = writeln!(markdown, "*Transcription not available: {e}*\n");
            }
        }
    }

    #[cfg(not(feature = "transcription"))]
    {
        markdown.push_str(
            "*Transcription not available. Enable the `transcription` feature to transcribe audio.*\n\n",
        );
    }

    Ok(markdown)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "Requires test audio files"]
    fn test_process_wav() {
        // This test requires a real WAV file
        // Run with: cargo test --features transcription -- --ignored
        let result = process_wav("test-corpus/audio/wav/sample.wav");
        assert!(result.is_ok());
    }

    #[test]
    #[ignore = "Requires test audio files"]
    fn test_process_mp3() {
        // This test requires a real MP3 file
        // Run with: cargo test --features transcription -- --ignored
        let result = process_mp3("test-corpus/audio/mp3/sample.mp3");
        assert!(result.is_ok());
    }

    #[test]
    fn test_process_wav_nonexistent_file() {
        // Test error handling for missing WAV file
        let result = process_wav("/nonexistent/path/to/audio.wav");
        assert!(result.is_err());
    }

    #[test]
    fn test_process_mp3_nonexistent_file() {
        // Test error handling for missing MP3 file
        let result = process_mp3("/nonexistent/path/to/audio.mp3");
        assert!(result.is_err());
    }

    #[test]
    fn test_process_wav_empty_path() {
        // Test empty path
        let result = process_wav("");
        assert!(result.is_err());
    }

    #[test]
    fn test_process_mp3_empty_path() {
        // Test empty path
        let result = process_mp3("");
        assert!(result.is_err());
    }

    #[test]
    fn test_process_wav_path_types() {
        // Test that process_wav accepts different path types
        let path_str = "/nonexistent.wav";
        let result1 = process_wav(path_str);
        assert!(result1.is_err());

        let path_buf = std::path::PathBuf::from("/nonexistent.wav");
        let result2 = process_wav(&path_buf);
        assert!(result2.is_err());

        let path = std::path::Path::new("/nonexistent.wav");
        let result3 = process_wav(path);
        assert!(result3.is_err());
    }

    #[test]
    fn test_process_mp3_path_types() {
        // Test that process_mp3 accepts different path types
        let path_str = "/nonexistent.mp3";
        let result1 = process_mp3(path_str);
        assert!(result1.is_err());

        let path_buf = std::path::PathBuf::from("/nonexistent.mp3");
        let result2 = process_mp3(&path_buf);
        assert!(result2.is_err());

        let path = std::path::Path::new("/nonexistent.mp3");
        let result3 = process_mp3(path);
        assert!(result3.is_err());
    }

    #[test]
    fn test_process_wav_with_spaces_in_path() {
        // Test file path with spaces
        let result = process_wav("/nonexistent/path with spaces/audio.wav");
        assert!(result.is_err());
    }

    #[test]
    fn test_process_mp3_with_spaces_in_path() {
        // Test file path with spaces
        let result = process_mp3("/nonexistent/path with spaces/audio.mp3");
        assert!(result.is_err());
    }

    #[test]
    fn test_process_wav_with_special_characters() {
        // Test file path with special characters
        let result = process_wav("/nonexistent/audio-file_v1.0.wav");
        assert!(result.is_err());
    }

    #[test]
    fn test_process_mp3_with_special_characters() {
        // Test file path with special characters
        let result = process_mp3("/nonexistent/audio-file_v1.0.mp3");
        assert!(result.is_err());
    }

    #[test]
    fn test_process_wav_relative_path() {
        // Test relative path
        let result = process_wav("../nonexistent/audio.wav");
        assert!(result.is_err());
    }

    #[test]
    fn test_process_mp3_relative_path() {
        // Test relative path
        let result = process_mp3("../nonexistent/audio.mp3");
        assert!(result.is_err());
    }
}
