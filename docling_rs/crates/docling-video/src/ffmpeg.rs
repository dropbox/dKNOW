use crate::error::{Result, VideoError};
use regex::Regex;
use std::path::Path;
use std::process::Command;
use std::sync::LazyLock;

// =============================================================================
// Pre-compiled regex patterns using std::sync::LazyLock (Rust 1.80+)
// =============================================================================

// -- FFmpeg output parsing patterns --
static RE_STREAM: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"Stream #0:(\d+)(?:\((\w+)\))?: Subtitle: (\w+)(?: \((\w+)\))?")
        .expect("valid stream regex")
});
static RE_TITLE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\(title\s*:\s*([^)]+)\)").expect("valid title regex"));

/// Check if `FFmpeg` is available in the system PATH
///
/// # Errors
///
/// Returns `VideoError::FfmpegNotFound` if `FFmpeg` is not installed or not in PATH
#[must_use = "this function returns the FFmpeg version string that should be used or logged"]
pub fn check_ffmpeg_available() -> Result<String> {
    let output = Command::new("ffmpeg")
        .arg("-version")
        .output()
        .map_err(|_| VideoError::FfmpegNotFound)?;

    if !output.status.success() {
        return Err(VideoError::FfmpegNotFound);
    }

    let version_output = String::from_utf8_lossy(&output.stdout);

    // Extract version from first line: "ffmpeg version N-xxxxx-xxxxx"
    let version = version_output
        .lines()
        .next()
        .unwrap_or("unknown")
        .to_string();

    Ok(version)
}

/// Information about a subtitle track in a video file
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct SubtitleTrackInfo {
    /// Stream index (e.g., 0, 1, 2)
    pub stream_index: usize,
    /// Subtitle stream index within subtitle streams (e.g., s:0, s:1)
    pub subtitle_index: usize,
    /// Language code (e.g., "eng", "spa", "fra")
    pub language: Option<String>,
    /// Codec name (e.g., "subrip", "webvtt", "ass")
    pub codec: String,
    /// Whether this is the default subtitle track
    pub is_default: bool,
    /// Track title/name (if available)
    pub title: Option<String>,
}

/// Detect subtitle tracks in a video file
///
/// Uses `ffmpeg -i` to probe the video file and parses the output
/// to identify subtitle streams.
///
/// Example ffmpeg output:
/// ```text
/// Stream #0:2(eng): Subtitle: subrip (default)
/// Stream #0:3(spa): Subtitle: ass
/// ```
///
/// # Errors
///
/// Returns errors if:
/// - Video file does not exist (`VideoError::FileNotFound`)
/// - `FFmpeg` command fails (I/O error)
/// - `FFmpeg` output is not valid UTF-8
/// - Subtitle track parsing fails (`VideoError::SubtitleTrackParsingFailed`)
#[must_use = "this function returns subtitle track information that should be processed"]
pub fn detect_subtitle_tracks<P: AsRef<Path>>(video_path: P) -> Result<Vec<SubtitleTrackInfo>> {
    let video_path = video_path.as_ref();

    if !video_path.exists() {
        return Err(VideoError::FileNotFound(video_path.display().to_string()));
    }

    let output = Command::new("ffmpeg").arg("-i").arg(video_path).output()?;

    // FFmpeg writes stream info to stderr
    let stderr = String::from_utf8(output.stderr)?;

    parse_subtitle_tracks(&stderr)
}

/// Parse subtitle track information from ffmpeg output
///
/// Example lines:
/// - `Stream #0:2(eng): Subtitle: subrip (default)`
/// - `Stream #0:3(spa): Subtitle: ass`
/// - `Stream #0:4: Subtitle: webvtt`
/// - `Stream #0:2(eng): Subtitle: subrip (title : English Subtitles)`
fn parse_subtitle_tracks(ffmpeg_output: &str) -> Result<Vec<SubtitleTrackInfo>> {
    let mut tracks = Vec::new();
    let mut subtitle_index = 0;

    let lines: Vec<&str> = ffmpeg_output.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        if let Some(captures) = RE_STREAM.captures(line) {
            let stream_index = captures
                .get(1)
                .and_then(|m| m.as_str().parse().ok())
                .ok_or_else(|| {
                    VideoError::SubtitleTrackParsingFailed(format!(
                        "Failed to parse stream index from: {line}"
                    ))
                })?;

            let language = captures.get(2).map(|m| m.as_str().to_string());
            let codec = captures
                .get(3)
                .map(|m| m.as_str().to_string())
                .ok_or_else(|| {
                    VideoError::SubtitleTrackParsingFailed(format!(
                        "Failed to parse codec from: {line}"
                    ))
                })?;

            let is_default = captures.get(4).is_some_and(|m| m.as_str() == "default");

            // Try to extract title from the stream line itself
            let mut title = RE_TITLE
                .captures(line)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().trim().to_string());

            // If no title in stream line, check the next line for Metadata section
            if title.is_none() && i + 1 < lines.len() {
                let next_line = lines[i + 1];
                if next_line.trim().starts_with("Metadata:") {
                    // Check following lines for title field
                    let mut j = i + 2;
                    while j < lines.len() && lines[j].starts_with("      ") {
                        let metadata_line = lines[j].trim();
                        if metadata_line.starts_with("title") {
                            if let Some(colon_pos) = metadata_line.find(':') {
                                title = Some(metadata_line[colon_pos + 1..].trim().to_string());
                                break;
                            }
                        }
                        j += 1;
                    }
                }
            }

            tracks.push(SubtitleTrackInfo {
                stream_index,
                subtitle_index,
                language,
                codec,
                is_default,
                title,
            });

            subtitle_index += 1;
        }

        i += 1;
    }

    if tracks.is_empty() {
        return Err(VideoError::NoSubtitleTracks);
    }

    Ok(tracks)
}

/// Extract a subtitle track from a video file
///
/// Extracts the subtitle track at the given index to an output file.
/// The output format is determined by the `output_format` parameter
/// (typically "srt" for maximum compatibility).
///
/// # Arguments
///
/// * `video_path` - Path to the video file
/// * `subtitle_index` - Index of the subtitle stream (0-based, within subtitle streams)
/// * `output_path` - Path to write the extracted subtitle file
/// * `output_format` - Subtitle format to convert to (e.g., "srt", "vtt", "ass")
///
/// # Errors
///
/// Returns errors if:
/// - `FFmpeg` command fails (I/O error)
/// - `FFmpeg` extraction fails (`VideoError::FfmpegCommandFailed`)
pub fn extract_subtitle_track<P: AsRef<Path>>(
    video_path: P,
    subtitle_index: usize,
    output_path: P,
    output_format: &str,
) -> Result<()> {
    let video_path = video_path.as_ref();
    let output_path = output_path.as_ref();

    let status = Command::new("ffmpeg")
        .arg("-y") // Overwrite output file
        .arg("-i")
        .arg(video_path)
        .arg("-map")
        .arg(format!("0:s:{subtitle_index}"))
        .arg("-c:s")
        .arg(output_format)
        .arg(output_path)
        .status()?;

    if !status.success() {
        return Err(VideoError::FfmpegCommandFailed(format!(
            "Failed to extract subtitle track {} from {}",
            subtitle_index,
            video_path.display()
        )));
    }

    Ok(())
}

/// Extract audio track from video file to WAV format (16kHz mono, 16-bit PCM)
///
/// This format is suitable for audio transcription using Whisper or other
/// speech recognition systems.
///
/// # Arguments
///
/// * `video_path` - Path to the video file
/// * `output_path` - Path to write the extracted audio WAV file
///
/// # Errors
///
/// Returns an error if:
/// - `ffmpeg` command fails to execute
/// - Audio extraction fails (e.g., no audio track or codec issue)
pub fn extract_audio_track<P: AsRef<Path>>(video_path: P, output_path: P) -> Result<()> {
    let video_path = video_path.as_ref();
    let output_path = output_path.as_ref();

    let status = Command::new("ffmpeg")
        .arg("-y") // Overwrite output file
        .arg("-i")
        .arg(video_path)
        .arg("-vn") // No video
        .arg("-acodec")
        .arg("pcm_s16le") // 16-bit PCM
        .arg("-ar")
        .arg("16000") // 16kHz sample rate (required by Whisper)
        .arg("-ac")
        .arg("1") // Mono (required by Whisper)
        .arg(output_path)
        .status()?;

    if !status.success() {
        return Err(VideoError::FfmpegCommandFailed(format!(
            "Failed to extract audio track from {}",
            video_path.display()
        )));
    }

    Ok(())
}

/// Check if video file has an audio track
///
/// # Errors
///
/// Returns an error if:
/// - `ffmpeg` command fails to execute
/// - `ffmpeg` output contains invalid UTF-8
#[must_use = "this function returns whether the video has audio that should be checked"]
pub fn has_audio_track<P: AsRef<Path>>(video_path: P) -> Result<bool> {
    let video_path = video_path.as_ref();

    let output = Command::new("ffmpeg").arg("-i").arg(video_path).output()?;

    let stderr = String::from_utf8(output.stderr)?;

    // Look for "Audio:" in stream information
    Ok(stderr.contains("Audio:"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_subtitle_tracks_single() {
        let ffmpeg_output = r"
Input #0, matroska,webm, from 'video.mkv':
  Duration: 00:05:00.00, start: 0.000000, bitrate: 1000 kb/s
    Stream #0:0: Video: h264 (High), yuv420p, 1920x1080
    Stream #0:1: Audio: aac (LC), 48000 Hz, stereo
    Stream #0:2(eng): Subtitle: subrip (default)
";

        let tracks = parse_subtitle_tracks(ffmpeg_output).unwrap();
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].stream_index, 2);
        assert_eq!(tracks[0].subtitle_index, 0);
        assert_eq!(tracks[0].language, Some("eng".to_string()));
        assert_eq!(tracks[0].codec, "subrip");
        assert!(tracks[0].is_default);
        assert_eq!(tracks[0].title, None);
    }

    #[test]
    fn test_parse_subtitle_tracks_multiple() {
        let ffmpeg_output = r"
    Stream #0:2(eng): Subtitle: subrip (default)
    Stream #0:3(spa): Subtitle: ass
    Stream #0:4: Subtitle: webvtt
";

        let tracks = parse_subtitle_tracks(ffmpeg_output).unwrap();
        assert_eq!(tracks.len(), 3);

        assert_eq!(tracks[0].subtitle_index, 0);
        assert_eq!(tracks[0].language, Some("eng".to_string()));
        assert!(tracks[0].is_default);
        assert_eq!(tracks[0].title, None);

        assert_eq!(tracks[1].subtitle_index, 1);
        assert_eq!(tracks[1].language, Some("spa".to_string()));
        assert!(!tracks[1].is_default);
        assert_eq!(tracks[1].title, None);

        assert_eq!(tracks[2].subtitle_index, 2);
        assert_eq!(tracks[2].language, None);
        assert!(!tracks[2].is_default);
        assert_eq!(tracks[2].title, None);
    }

    #[test]
    fn test_parse_subtitle_tracks_none() {
        let ffmpeg_output = r"
    Stream #0:0: Video: h264
    Stream #0:1: Audio: aac
";

        let result = parse_subtitle_tracks(ffmpeg_output);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), VideoError::NoSubtitleTracks));
    }

    #[test]
    fn test_parse_subtitle_tracks_with_title_inline() {
        let ffmpeg_output = r"
    Stream #0:2(eng): Subtitle: subrip (default) (title : English Subtitles)
    Stream #0:3(spa): Subtitle: ass (title: Spanish)
";

        let tracks = parse_subtitle_tracks(ffmpeg_output).unwrap();
        assert_eq!(tracks.len(), 2);

        assert_eq!(tracks[0].title, Some("English Subtitles".to_string()));
        assert_eq!(tracks[0].language, Some("eng".to_string()));
        assert!(tracks[0].is_default);

        assert_eq!(tracks[1].title, Some("Spanish".to_string()));
        assert_eq!(tracks[1].language, Some("spa".to_string()));
        assert!(!tracks[1].is_default);
    }

    #[test]
    fn test_parse_subtitle_tracks_with_title_metadata() {
        let ffmpeg_output = r"
    Stream #0:2(eng): Subtitle: subrip (default)
    Metadata:
      title           : English Subtitles
      language        : eng
    Stream #0:3(spa): Subtitle: ass
    Metadata:
      title           : Spanish Commentary
      language        : spa
";

        let tracks = parse_subtitle_tracks(ffmpeg_output).unwrap();
        assert_eq!(tracks.len(), 2);

        assert_eq!(tracks[0].title, Some("English Subtitles".to_string()));
        assert_eq!(tracks[0].language, Some("eng".to_string()));

        assert_eq!(tracks[1].title, Some("Spanish Commentary".to_string()));
        assert_eq!(tracks[1].language, Some("spa".to_string()));
    }

    #[test]
    fn test_parse_subtitle_tracks_mixed_title_formats() {
        let ffmpeg_output = r"
    Stream #0:2(eng): Subtitle: subrip (default) (title : Inline Title)
    Stream #0:3(spa): Subtitle: ass
    Metadata:
      title           : Metadata Title
    Stream #0:4: Subtitle: webvtt
";

        let tracks = parse_subtitle_tracks(ffmpeg_output).unwrap();
        assert_eq!(tracks.len(), 3);

        // First track has inline title
        assert_eq!(tracks[0].title, Some("Inline Title".to_string()));

        // Second track has metadata title
        assert_eq!(tracks[1].title, Some("Metadata Title".to_string()));

        // Third track has no title
        assert_eq!(tracks[2].title, None);
    }
}
