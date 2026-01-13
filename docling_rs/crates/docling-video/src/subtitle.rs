use crate::error::{Result, VideoError};
use regex::Regex;
use std::fmt::Write;
use std::path::Path;
use std::sync::LazyLock;
use std::time::Duration;

// =============================================================================
// Pre-compiled regex patterns using std::sync::LazyLock (Rust 1.80+)
// =============================================================================

// -- WebVTT voice and formatting patterns --
static RE_VOICE_TAG: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"<v(?:\.([^>\s]+))?\s+([^>]+)>(.*)").expect("valid voice tag regex")
});
static RE_ITALIC: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<i>([^<]*)</i>").expect("valid italic regex"));
static RE_HTML_TAG: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<[^>]+>").expect("valid html tag regex"));

/// A single subtitle entry with timing and text
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct SubtitleEntry {
    /// Start time of the subtitle
    pub start_time: Duration,
    /// End time of the subtitle
    pub end_time: Duration,
    /// Subtitle text content
    pub text: String,
    /// Line number (for SRT format)
    pub line_number: Option<usize>,
    /// Speaker name (for `WebVTT` voice tags)
    pub speaker: Option<String>,
    /// Speaker classes (for `WebVTT` voice tags)
    pub speaker_classes: Vec<String>,

    // WebVTT positioning and styling metadata (WebVTT spec section 3.4)
    /// Horizontal position as percentage (0-100) or line number
    /// Example: "position:50%" means center horizontally
    pub position: Option<String>,
    /// Text alignment: start, center, end, left, right
    /// Example: "align:center"
    pub align: Option<String>,
    /// Vertical line position (percentage or line number)
    /// Example: "line:0%" means top, "line:100%" means bottom
    pub line: Option<String>,
    /// Cue box width as percentage
    /// Example: "size:50%" means half-width cue box
    pub size: Option<String>,
    /// Vertical text direction: rl (right-to-left) or lr (left-to-right)
    /// Used for Asian languages (Japanese, Chinese, Korean)
    pub vertical: Option<String>,
    /// Region ID reference for spatial layout
    /// Example: "region:top" refers to a REGION block
    pub region: Option<String>,
}

/// Parsed subtitle file with all entries
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct SubtitleFile {
    /// List of subtitle entries
    pub entries: Vec<SubtitleEntry>,
    /// Detected subtitle format
    pub format: SubtitleFormat,
}

/// Supported subtitle formats
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum SubtitleFormat {
    /// `SubRip` format (.srt) - simple text-based format (most common, default)
    #[default]
    Srt,
    /// `WebVTT` format (.vtt) - web standard with styling support
    Vtt,
    /// Advanced `SubStation` Alpha (.ass) - feature-rich styling format
    Ass,
    /// `SubStation` Alpha (.ssa) - predecessor to ASS format
    Ssa,
}

impl std::fmt::Display for SubtitleFormat {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.extension())
    }
}

impl std::str::FromStr for SubtitleFormat {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        // Strip leading dot if present (e.g., ".srt" -> "srt")
        let ext = s.trim().trim_start_matches('.');
        Self::from_extension(ext)
            .ok_or_else(|| format!("Unknown subtitle format '{s}'. Expected: srt, vtt, ass, ssa"))
    }
}

impl SubtitleFormat {
    /// Detect subtitle format from file extension
    #[inline]
    #[must_use = "detects format from file extension"]
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "srt" => Some(Self::Srt),
            "vtt" => Some(Self::Vtt),
            "ass" => Some(Self::Ass),
            "ssa" => Some(Self::Ssa),
            _ => None,
        }
    }

    /// Get file extension for this format
    #[inline]
    #[must_use = "returns file extension for format"]
    pub const fn extension(&self) -> &'static str {
        match self {
            Self::Srt => "srt",
            Self::Vtt => "vtt",
            Self::Ass => "ass",
            Self::Ssa => "ssa",
        }
    }
}

/// Parse `WebVTT` content with voice tag extraction
///
/// Python source: ~/`docling/docling/backend/webvtt_backend.py`
/// Extracts speaker names and classes from `<v.class Speaker>text</v>` tags
fn parse_webvtt_content(content: &str) -> Result<Vec<SubtitleEntry>> {
    let mut entries = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    // Skip WEBVTT header and initial comments/notes
    while i < lines.len() {
        let line = lines[i].trim();
        if line.is_empty() || line.starts_with("WEBVTT") || line.starts_with("NOTE") {
            i += 1;
            continue;
        }
        break;
    }

    // Parse cue blocks
    while i < lines.len() {
        let line = lines[i].trim();

        // Skip empty lines
        if line.is_empty() {
            i += 1;
            continue;
        }

        // Check if this is a timestamp line
        if line.contains("-->") {
            // Parse timestamps and cue settings
            // Format: START --> END [settings]
            // Example: 00:00:00.000 --> 00:00:03.000 line:0% align:start position:50%
            let parts: Vec<&str> = line.split("-->").collect();
            if parts.len() != 2 {
                i += 1;
                continue;
            }

            let start_str = parts[0].split_whitespace().next().unwrap_or("");
            let end_part = parts[1].trim();

            // Split end timestamp from cue settings
            let end_parts: Vec<&str> = end_part.split_whitespace().collect();
            let end_str = end_parts.first().unwrap_or(&"");

            let start_time = parse_vtt_timestamp(start_str)?;
            let end_time = parse_vtt_timestamp(end_str)?;

            // Parse cue settings (everything after end timestamp)
            let cue_settings = (end_parts.len() > 1).then(|| parse_cue_settings(&end_parts[1..]));

            // Collect cue text lines until empty line
            i += 1;
            let mut cue_lines = Vec::new();
            while i < lines.len() {
                let cue_line = lines[i];
                if cue_line.trim().is_empty() {
                    break;
                }
                cue_lines.push(cue_line);
                i += 1;
            }

            // Parse cue text with voice tags and apply cue settings
            let full_text = cue_lines.join("\n");
            parse_cue_text_with_settings(
                &full_text,
                start_time,
                end_time,
                cue_settings,
                &mut entries,
            );
        } else {
            // Skip cue identifier lines or other content
            i += 1;
        }
    }

    Ok(entries)
}

/// Parse `WebVTT` timestamp (HH:MM:SS.mmm or MM:SS.mmm)
fn parse_vtt_timestamp(timestamp: &str) -> Result<Duration> {
    let parts: Vec<&str> = timestamp.split(&[':', '.']).collect();

    let (hours, minutes, seconds, millis) = match parts.len() {
        4 => {
            // HH:MM:SS.mmm
            (
                parts[0].parse::<u64>().unwrap_or(0),
                parts[1].parse::<u64>().unwrap_or(0),
                parts[2].parse::<u64>().unwrap_or(0),
                parts[3].parse::<u64>().unwrap_or(0),
            )
        }
        3 => {
            // MM:SS.mmm
            (
                0,
                parts[0].parse::<u64>().unwrap_or(0),
                parts[1].parse::<u64>().unwrap_or(0),
                parts[2].parse::<u64>().unwrap_or(0),
            )
        }
        _ => {
            return Err(VideoError::SubtitleParsingFailed(format!(
                "Invalid timestamp: {timestamp}"
            )))
        }
    };

    Ok(Duration::from_millis(
        hours * 3600 * 1000 + minutes * 60 * 1000 + seconds * 1000 + millis,
    ))
}

/// `WebVTT` cue settings structure
///
/// Stores positioning and styling metadata from `WebVTT` cue settings
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
struct CueSettings {
    position: Option<String>,
    align: Option<String>,
    line: Option<String>,
    size: Option<String>,
    vertical: Option<String>,
    region: Option<String>,
}

/// Parse `WebVTT` cue settings from settings tokens
///
/// `WebVTT` spec section 3.4: Cue settings
/// Format: key:value pairs separated by whitespace
/// Example: `["line:0%", "align:start", "position:50%"]`
fn parse_cue_settings(settings: &[&str]) -> CueSettings {
    let mut cue_settings = CueSettings::default();

    for setting in settings {
        if let Some((key, value)) = setting.split_once(':') {
            match key {
                "position" => cue_settings.position = Some(value.to_string()),
                "align" => cue_settings.align = Some(value.to_string()),
                "line" => cue_settings.line = Some(value.to_string()),
                "size" => cue_settings.size = Some(value.to_string()),
                "vertical" => cue_settings.vertical = Some(value.to_string()),
                "region" => cue_settings.region = Some(value.to_string()),
                _ => {} // Ignore unknown settings
            }
        }
    }

    cue_settings
}

/// Parse cue text with voice tags, formatting, and apply cue settings
///
/// Enhanced version that accepts optional cue settings
fn parse_cue_text_with_settings(
    text: &str,
    start_time: Duration,
    end_time: Duration,
    cue_settings: Option<CueSettings>,
    entries: &mut Vec<SubtitleEntry>,
) {
    let mut processed_text = text.to_string();
    let mut speaker: Option<String> = None;
    let mut speaker_classes: Vec<String> = Vec::new();
    let mut extracted_text = String::new();

    // Extract voice tags (without requiring closing tag)
    if let Some(caps) = RE_VOICE_TAG.captures(text) {
        if let Some(classes_match) = caps.get(1) {
            speaker_classes = classes_match
                .as_str()
                .split('.')
                .map(std::string::ToString::to_string)
                .collect();
        }
        if let Some(speaker_match) = caps.get(2) {
            speaker = Some(speaker_match.as_str().to_string());
        }
        if let Some(text_match) = caps.get(3) {
            extracted_text = text_match.as_str().to_string();
        }
        processed_text = RE_VOICE_TAG.replace_all(&processed_text, "$3").to_string();
    }

    // If we extracted text from voice tag, use that for further processing
    if !extracted_text.is_empty() {
        processed_text = extracted_text;
    }

    // Handle italic tags (convert to *text*)
    processed_text = RE_ITALIC.replace_all(&processed_text, "*$1*").to_string();

    // Remove any remaining HTML tags
    processed_text = RE_HTML_TAG.replace_all(&processed_text, "").to_string();

    // Create entry with cue settings
    let settings = cue_settings.unwrap_or_default();
    entries.push(SubtitleEntry {
        start_time,
        end_time,
        text: processed_text.trim().to_string(),
        line_number: None,
        speaker,
        speaker_classes,
        position: settings.position,
        align: settings.align,
        line: settings.line,
        size: settings.size,
        vertical: settings.vertical,
        region: settings.region,
    });
}

/// Parse a subtitle file
///
/// Supports SRT and `WebVTT` formats. Uses format-specific parsing.
///
/// # Errors
///
/// Returns an error if:
/// - The file extension is unrecognized (not `.srt` or `.vtt`)
/// - The file cannot be read (I/O error)
/// - The subtitle content is malformed and cannot be parsed
#[must_use = "this function returns a parsed subtitle file that should be processed"]
pub fn parse_subtitle_file<P: AsRef<Path>>(subtitle_path: P) -> Result<SubtitleFile> {
    let subtitle_path = subtitle_path.as_ref();

    // Detect format from extension
    let format = subtitle_path
        .extension()
        .and_then(|ext| ext.to_str())
        .and_then(SubtitleFormat::from_extension)
        .ok_or_else(|| {
            VideoError::UnsupportedSubtitleFormat(subtitle_path.display().to_string())
        })?;

    // Read file content
    let content = std::fs::read_to_string(subtitle_path)
        .map_err(|e| VideoError::SubtitleParsingFailed(e.to_string()))?;

    let entries = if format == SubtitleFormat::Vtt {
        // Use WebVTT-specific parser
        parse_webvtt_content(&content)?
    } else {
        // Parse using srtparse (works for SRT format)
        let parsed_subtitles = srtparse::from_str(&content)
            .map_err(|e| VideoError::SubtitleParsingFailed(format!("SRT parsing failed: {e}")))?;

        // Convert to our format
        parsed_subtitles
            .iter()
            .enumerate()
            .map(|(idx, sub)| {
                // Convert Time to milliseconds
                let start_ms = (sub.start_time.hours * 3600
                    + sub.start_time.minutes * 60
                    + sub.start_time.seconds)
                    * 1000
                    + sub.start_time.milliseconds;
                let end_ms =
                    (sub.end_time.hours * 3600 + sub.end_time.minutes * 60 + sub.end_time.seconds)
                        * 1000
                        + sub.end_time.milliseconds;

                SubtitleEntry {
                    start_time: Duration::from_millis(start_ms),
                    end_time: Duration::from_millis(end_ms),
                    text: sub.text.clone(),
                    line_number: Some(idx + 1),
                    speaker: None,
                    speaker_classes: Vec::new(),
                    // SRT format doesn't have cue settings
                    position: None,
                    align: None,
                    line: None,
                    size: None,
                    vertical: None,
                    region: None,
                }
            })
            .collect()
    };

    Ok(SubtitleFile { entries, format })
}

/// Format subtitle entries as markdown
#[must_use = "formats subtitles as markdown"]
pub fn format_as_markdown(subtitle_file: &SubtitleFile) -> String {
    let mut output = String::new();

    output.push_str("## Subtitles\n\n");

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

/// Format subtitle entries as plain text transcript
#[must_use = "formats subtitles as plain text"]
pub fn format_as_transcript(subtitle_file: &SubtitleFile) -> String {
    subtitle_file
        .entries
        .iter()
        .map(|entry| entry.text.trim())
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Statistics about a parsed subtitle file
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SubtitleStats {
    /// Number of subtitle entries in the file
    pub entry_count: usize,
    /// Total duration from first to last subtitle
    pub total_duration: Duration,
    /// Total character count across all subtitle text
    pub total_text_length: usize,
    /// Average characters per second (reading speed metric)
    pub avg_chars_per_second: f64,
}

impl SubtitleStats {
    /// Calculate statistics from a parsed subtitle file
    #[must_use = "calculates statistics from subtitle file"]
    pub fn from_subtitle_file(subtitle_file: &SubtitleFile) -> Self {
        let entry_count = subtitle_file.entries.len();

        let total_duration = subtitle_file
            .entries
            .last()
            .map(|e| e.end_time)
            .unwrap_or_default();

        let total_text_length: usize = subtitle_file.entries.iter().map(|e| e.text.len()).sum();

        // Precision loss acceptable: text length in subtitles never exceeds f64 mantissa range
        #[allow(clippy::cast_precision_loss)]
        let avg_chars_per_second = if total_duration.as_secs() > 0 {
            total_text_length as f64 / total_duration.as_secs_f64()
        } else {
            0.0
        };

        Self {
            entry_count,
            total_duration,
            total_text_length,
            avg_chars_per_second,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subtitle_format_from_extension() {
        assert_eq!(
            SubtitleFormat::from_extension("srt"),
            Some(SubtitleFormat::Srt)
        );
        assert_eq!(
            SubtitleFormat::from_extension("SRT"),
            Some(SubtitleFormat::Srt)
        );
        assert_eq!(
            SubtitleFormat::from_extension("vtt"),
            Some(SubtitleFormat::Vtt)
        );
        assert_eq!(
            SubtitleFormat::from_extension("ass"),
            Some(SubtitleFormat::Ass)
        );
        assert_eq!(
            SubtitleFormat::from_extension("ssa"),
            Some(SubtitleFormat::Ssa)
        );
        assert_eq!(SubtitleFormat::from_extension("unknown"), None);
    }

    #[test]
    fn test_format_as_transcript() {
        let subtitle_file = SubtitleFile {
            entries: vec![
                SubtitleEntry {
                    start_time: Duration::from_secs(0),
                    end_time: Duration::from_secs(2),
                    text: "Hello world".to_string(),
                    line_number: Some(1),
                    speaker: None,
                    speaker_classes: vec![],
                    position: None,
                    align: None,
                    line: None,
                    size: None,
                    vertical: None,
                    region: None,
                },
                SubtitleEntry {
                    start_time: Duration::from_secs(2),
                    end_time: Duration::from_secs(4),
                    text: "This is a test".to_string(),
                    line_number: Some(2),
                    speaker: None,
                    speaker_classes: vec![],
                    position: None,
                    align: None,
                    line: None,
                    size: None,
                    vertical: None,
                    region: None,
                },
            ],
            format: SubtitleFormat::Srt,
        };

        let transcript = format_as_transcript(&subtitle_file);
        assert_eq!(transcript, "Hello world This is a test");
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn test_subtitle_stats() {
        let subtitle_file = SubtitleFile {
            entries: vec![
                SubtitleEntry {
                    start_time: Duration::from_secs(0),
                    end_time: Duration::from_secs(2),
                    text: "Hello".to_string(), // 5 chars
                    line_number: Some(1),
                    speaker: None,
                    speaker_classes: vec![],
                    position: None,
                    align: None,
                    line: None,
                    size: None,
                    vertical: None,
                    region: None,
                },
                SubtitleEntry {
                    start_time: Duration::from_secs(2),
                    end_time: Duration::from_secs(10),
                    text: "World".to_string(), // 5 chars
                    line_number: Some(2),
                    speaker: None,
                    speaker_classes: vec![],
                    position: None,
                    align: None,
                    line: None,
                    size: None,
                    vertical: None,
                    region: None,
                },
            ],
            format: SubtitleFormat::Srt,
        };

        let stats = SubtitleStats::from_subtitle_file(&subtitle_file);
        assert_eq!(stats.entry_count, 2);
        assert_eq!(stats.total_duration, Duration::from_secs(10));
        assert_eq!(stats.total_text_length, 10);
        assert_eq!(stats.avg_chars_per_second, 1.0);
    }

    #[test]
    fn test_subtitle_format_display() {
        assert_eq!(format!("{}", SubtitleFormat::Srt), "srt");
        assert_eq!(format!("{}", SubtitleFormat::Vtt), "vtt");
        assert_eq!(format!("{}", SubtitleFormat::Ass), "ass");
        assert_eq!(format!("{}", SubtitleFormat::Ssa), "ssa");
    }

    #[test]
    fn test_subtitle_format_from_str() {
        // Exact matches
        assert_eq!(
            "srt".parse::<SubtitleFormat>().unwrap(),
            SubtitleFormat::Srt
        );
        assert_eq!(
            "vtt".parse::<SubtitleFormat>().unwrap(),
            SubtitleFormat::Vtt
        );
        assert_eq!(
            "ass".parse::<SubtitleFormat>().unwrap(),
            SubtitleFormat::Ass
        );
        assert_eq!(
            "ssa".parse::<SubtitleFormat>().unwrap(),
            SubtitleFormat::Ssa
        );

        // Case insensitive
        assert_eq!(
            "SRT".parse::<SubtitleFormat>().unwrap(),
            SubtitleFormat::Srt
        );
        assert_eq!(
            "VTT".parse::<SubtitleFormat>().unwrap(),
            SubtitleFormat::Vtt
        );

        // With leading dot (file extension style)
        assert_eq!(
            ".srt".parse::<SubtitleFormat>().unwrap(),
            SubtitleFormat::Srt
        );
        assert_eq!(
            ".vtt".parse::<SubtitleFormat>().unwrap(),
            SubtitleFormat::Vtt
        );

        // Invalid
        assert!("invalid".parse::<SubtitleFormat>().is_err());
        assert!("mp4".parse::<SubtitleFormat>().is_err());
    }

    #[test]
    fn test_subtitle_format_roundtrip() {
        for format in [
            SubtitleFormat::Srt,
            SubtitleFormat::Vtt,
            SubtitleFormat::Ass,
            SubtitleFormat::Ssa,
        ] {
            let s = format.to_string();
            let parsed: SubtitleFormat = s.parse().unwrap();
            assert_eq!(parsed, format);
        }
    }
}
