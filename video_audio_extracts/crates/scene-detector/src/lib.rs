//! Scene detection module using `FFmpeg`'s scdet filter
//!
//! This module provides scene boundary detection using `FFmpeg`'s built-in scdet filter,
//! which analyzes frame-to-frame differences to identify scene changes.
//!
//! # Features
//! - Fast scene boundary detection using `FFmpeg` scdet filter
//! - Configurable threshold for scene change sensitivity
//! - Returns scene boundaries with timestamps and confidence scores
//! - No external ML models required (uses classical algorithm)
//!
//! # Example
//! ```no_run
//! use video_audio_scene::{detect_scenes, SceneDetectorConfig};
//! use std::path::Path;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let config = SceneDetectorConfig::default();
//! let scenes = detect_scenes(Path::new("video.mp4"), &config)?;
//!
//! for scene in scenes.boundaries {
//!     println!("Scene change at {:.2}s (score: {:.2})",
//!              scene.timestamp, scene.score);
//! }
//! # Ok(())
//! # }
//! ```

pub mod plugin;

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;
use thiserror::Error;
use tracing::{debug, info, warn};
use video_audio_common::ProcessingError;

/// Errors specific to scene detection
#[derive(Error, Debug)]
pub enum SceneDetectionError {
    #[error("FFmpeg execution failed: {0}")]
    FfmpegError(String),

    #[error("Failed to parse scdet output: {0}")]
    ParseError(String),

    #[error("Video file not found: {0}")]
    FileNotFound(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

impl From<SceneDetectionError> for ProcessingError {
    fn from(err: SceneDetectionError) -> Self {
        ProcessingError::Other(err.to_string())
    }
}

/// Configuration for scene detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneDetectorConfig {
    /// Scene change detection threshold (0.0-100.0)
    /// Lower values = more sensitive (more scene changes detected)
    /// Higher values = less sensitive (fewer scene changes detected)
    /// Default: 10.0 (`FFmpeg` default)
    /// Recommended range: 5.0-30.0
    pub threshold: f64,

    /// Minimum time between scene changes in seconds
    /// Prevents rapid scene change detections
    /// Default: 0.0 (no minimum)
    pub min_scene_duration: f64,

    /// Whether to use keyframe-only processing for faster detection
    /// When true, only keyframes (I-frames) are processed, providing 10-30x speedup
    /// but with slightly lower accuracy for scenes between keyframes.
    /// Default: false (process all frames for maximum accuracy)
    pub keyframes_only: bool,
}

impl Default for SceneDetectorConfig {
    fn default() -> Self {
        Self {
            threshold: 10.0,
            min_scene_duration: 0.0,
            keyframes_only: false,
        }
    }
}

/// A detected scene boundary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneBoundary {
    /// Timestamp of the scene change in seconds
    pub timestamp: f64,

    /// Scene change score (higher = more confident)
    /// `FFmpeg` scdet scores typically range from 0.0 to ~10.0 or higher
    /// Scores above the threshold indicate a scene change
    pub score: f64,
}

/// A scene segment (time range between boundaries)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    /// Start time of scene in seconds
    pub start_time: f64,
    /// End time of scene in seconds
    pub end_time: f64,
    /// Start frame number
    pub start_frame: u64,
    /// End frame number
    pub end_frame: u64,
    /// Number of frames in scene
    pub frame_count: u64,
    /// Scene change score at the boundary (0.0 for first scene)
    pub score: f32,
}

/// Result of scene detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneDetectionResult {
    /// List of detected scene boundaries
    pub boundaries: Vec<SceneBoundary>,

    /// List of scenes (time ranges between boundaries)
    pub scenes: Vec<Scene>,

    /// Number of detected scenes
    /// This is `boundaries.len()` + 1 (includes the initial scene before first boundary)
    pub num_scenes: usize,

    /// Configuration used for detection
    pub config: SceneDetectorConfig,
}

/// Detect scene boundaries in a video file using `FFmpeg`'s scdet filter
///
/// # Arguments
/// * `video_path` - Path to the video file
/// * `config` - Configuration for scene detection
///
/// # Returns
/// Scene detection result with timestamps and scores
///
/// # Example
/// ```no_run
/// use video_audio_scene::{detect_scenes, SceneDetectorConfig};
/// use std::path::Path;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let config = SceneDetectorConfig {
///     threshold: 15.0,
///     min_scene_duration: 1.0,
///     keyframes_only: true, // Use keyframes_only for 10-30x speedup
/// };
/// let result = detect_scenes(Path::new("video.mp4"), &config)?;
/// println!("Detected {} scenes", result.num_scenes);
/// # Ok(())
/// # }
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The video file does not exist
/// - `FFmpeg` execution fails
/// - `FFmpeg` output cannot be parsed
#[allow(
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
pub fn detect_scenes(
    video_path: &Path,
    config: &SceneDetectorConfig,
) -> Result<SceneDetectionResult, SceneDetectionError> {
    if !video_path.exists() {
        return Err(SceneDetectionError::FileNotFound(
            video_path.display().to_string(),
        ));
    }

    info!(
        "Running scene detection on {} with threshold {} (keyframes_only: {})",
        video_path.display(),
        config.threshold,
        config.keyframes_only
    );

    // Run FFmpeg with scdet filter
    // The scdet filter outputs scene change scores to stderr
    // Format: [scdet @ 0x...] lavfi.scd.score: X.XXX, lavfi.scd.time: Y.YYY
    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-i").arg(video_path);

    // If keyframes_only is enabled, skip non-keyframes for 10-30x speedup
    if config.keyframes_only {
        cmd.arg("-skip_frame").arg("nokey");
    }

    let output = cmd
        .arg("-vf")
        .arg(format!("scdet=t={}:s=1", config.threshold / 100.0))
        .arg("-an") // Disable audio
        .arg("-f")
        .arg("null")
        .arg("-")
        .output()
        .map_err(|e| SceneDetectionError::FfmpegError(format!("Failed to execute ffmpeg: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // FFmpeg always exits with 0 even for null output, but check anyway
        debug!("FFmpeg stderr: {}", stderr);
    }

    // Parse stderr output to extract duration and scene change timestamps/scores
    let stderr = String::from_utf8_lossy(&output.stderr);
    let mut boundaries: Vec<SceneBoundary> = Vec::with_capacity(100);
    let mut video_duration: Option<f64> = None;

    for line in stderr.lines() {
        // Parse duration line: Duration: HH:MM:SS.MS, start: ...
        if line.contains("Duration:") && video_duration.is_none() {
            if let Some(duration) = parse_duration_line(line) {
                video_duration = Some(duration);
                debug!("Parsed video duration: {:.2}s", duration);
            }
        }

        if line.contains("lavfi.scd.score") && line.contains("lavfi.scd.time") {
            // Parse line format: [scdet @ 0x...] lavfi.scd.score: 1.234, lavfi.scd.time: 5.678
            if let Some((score_str, time_str)) = parse_scdet_line(line) {
                match (score_str.parse::<f64>(), time_str.parse::<f64>()) {
                    (Ok(score), Ok(timestamp)) => {
                        // Only include scenes above threshold
                        if score >= config.threshold {
                            // Check minimum scene duration
                            if config.min_scene_duration > 0.0 {
                                if let Some(last) = boundaries.last() {
                                    if timestamp - last.timestamp < config.min_scene_duration {
                                        debug!(
                                            "Skipping scene at {:.2}s (too close to previous at {:.2}s)",
                                            timestamp, last.timestamp
                                        );
                                        continue;
                                    }
                                }
                            }

                            boundaries.push(SceneBoundary { timestamp, score });
                        }
                    }
                    _ => {
                        warn!(
                            "Failed to parse score or timestamp: {} | {}",
                            score_str, time_str
                        );
                    }
                }
            }
        }
    }

    let num_scenes = boundaries.len() + 1; // Include initial scene before first boundary

    // Build scenes from boundaries
    // Assume 30 FPS for frame calculations (we don't have exact video metadata here)
    let fps = 30.0;
    let mut scenes = Vec::with_capacity(num_scenes);

    if boundaries.is_empty() {
        // Special case: no scene boundaries detected
        // Create a single scene covering the entire video
        let end_time = video_duration.unwrap_or(0.0);
        scenes.push(Scene {
            start_time: 0.0,
            end_time,
            start_frame: 0,
            end_frame: (end_time * fps) as u64,
            frame_count: (end_time * fps) as u64,
            score: 0.0, // No scene change detected
        });
    } else {
        // First scene: from 0 to first boundary
        if let Some(first_boundary) = boundaries.first() {
            scenes.push(Scene {
                start_time: 0.0,
                end_time: first_boundary.timestamp,
                start_frame: 0,
                end_frame: (first_boundary.timestamp * fps) as u64,
                frame_count: (first_boundary.timestamp * fps) as u64,
                score: 0.0, // No boundary before first scene
            });
        }

        // Middle scenes: between boundaries
        for i in 0..(boundaries.len().saturating_sub(1)) {
            let start = boundaries[i].timestamp;
            let end = boundaries[i + 1].timestamp;
            scenes.push(Scene {
                start_time: start,
                end_time: end,
                start_frame: (start * fps) as u64,
                end_frame: (end * fps) as u64,
                frame_count: ((end - start) * fps) as u64,
                score: boundaries[i].score as f32,
            });
        }

        // Last scene: from last boundary to end of video
        if let Some(last_boundary) = boundaries.last() {
            // Use actual video duration if available, otherwise use last boundary + 1.0s as fallback
            let end_time = video_duration.unwrap_or(last_boundary.timestamp + 1.0);
            let duration_from_boundary = end_time - last_boundary.timestamp;

            scenes.push(Scene {
                start_time: last_boundary.timestamp,
                end_time,
                start_frame: (last_boundary.timestamp * fps) as u64,
                end_frame: (end_time * fps) as u64,
                frame_count: (duration_from_boundary * fps) as u64,
                score: last_boundary.score as f32,
            });
        }
    }

    info!(
        "Detected {} scene boundaries ({} total scenes)",
        boundaries.len(),
        num_scenes
    );

    Ok(SceneDetectionResult {
        boundaries,
        scenes,
        num_scenes,
        config: config.clone(),
    })
}

/// Parse duration from `FFmpeg` output line
/// Format: Duration: HH:MM:SS.MS, start: ...
/// Returns duration in seconds if parsing succeeds
fn parse_duration_line(line: &str) -> Option<f64> {
    // Find "Duration: " and extract the timestamp after it
    let duration_start = line.find("Duration: ")?;
    let duration_str_start = duration_start + "Duration: ".len();

    // Extract HH:MM:SS.MS format (stop at comma)
    let duration_end = line[duration_str_start..].find(',')?;
    let duration_str = &line[duration_str_start..duration_str_start + duration_end];

    // Parse HH:MM:SS.MS format
    let parts: Vec<&str> = duration_str.split(':').collect();
    if parts.len() != 3 {
        return None;
    }

    let hours: f64 = parts[0].parse().ok()?;
    let minutes: f64 = parts[1].parse().ok()?;
    let seconds: f64 = parts[2].parse().ok()?;

    Some(hours * 3600.0 + minutes * 60.0 + seconds)
}

/// Parse a line from `FFmpeg` scdet output
/// Format: [scdet @ 0x...] lavfi.scd.score: 1.234, lavfi.scd.time: 5.678
/// Returns (`score_str`, `time_str`) if parsing succeeds
fn parse_scdet_line(line: &str) -> Option<(String, String)> {
    // Find "lavfi.scd.score: " and extract the number after it
    let score_start = line.find("lavfi.scd.score: ")?;
    let score_str_start = score_start + "lavfi.scd.score: ".len();
    let score_end = line[score_str_start..].find(',')?;
    let score_str = &line[score_str_start..score_str_start + score_end];

    // Find "lavfi.scd.time: " and extract the number after it
    let time_start = line.find("lavfi.scd.time: ")?;
    let time_str_start = time_start + "lavfi.scd.time: ".len();
    // Time is at the end of the line or before whitespace
    let time_str = line[time_str_start..].split_whitespace().next()?;

    Some((score_str.to_string(), time_str.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_scdet_line() {
        let line = "[scdet @ 0x600003a3bc00] lavfi.scd.score: 4.793, lavfi.scd.time: 7.433333";
        let (score, time) = parse_scdet_line(line).unwrap();
        assert_eq!(score, "4.793");
        assert_eq!(time, "7.433333");
    }

    #[test]
    fn test_parse_scdet_line_with_trailing_text() {
        let line =
            "[scdet @ 0x600003a3bc00] lavfi.scd.score: 1.094, lavfi.scd.time: 8.883333 frame= 123";
        let (score, time) = parse_scdet_line(line).unwrap();
        assert_eq!(score, "1.094");
        assert_eq!(time, "8.883333");
    }

    #[test]
    fn test_parse_duration_line() {
        // Test typical FFmpeg duration line
        let line = "  Duration: 00:00:09.99, start: 0.000000, bitrate: 258 kb/s";
        let duration = parse_duration_line(line).unwrap();
        assert!((duration - 9.99).abs() < 0.01);

        // Test longer duration
        let line = "  Duration: 01:23:45.67, start: 0.000000, bitrate: 1000 kb/s";
        let duration = parse_duration_line(line).unwrap();
        assert!((duration - (3600.0 + 23.0 * 60.0 + 45.67)).abs() < 0.01);

        // Test short duration
        let line = "Duration: 00:00:01.50, start: 0.000000, bitrate: 100 kb/s";
        let duration = parse_duration_line(line).unwrap();
        assert!((duration - 1.5).abs() < 0.01);
    }

    #[test]
    fn test_default_config() {
        let config = SceneDetectorConfig::default();
        assert_eq!(config.threshold, 10.0);
        assert_eq!(config.min_scene_duration, 0.0);
    }
}
