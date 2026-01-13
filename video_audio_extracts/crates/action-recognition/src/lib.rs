//! Action recognition module using motion-based heuristics
//!
//! This module provides activity-level classification for videos by analyzing
//! keyframe sequences and temporal patterns.
//!
//! # Features
//! - Motion intensity classification (static, low, moderate, high motion)
//! - Scene change detection
//! - Temporal pattern analysis
//! - No ML model required (heuristic-based)
//!
//! # Activity Categories
//! - **Static**: Minimal motion (e.g., still camera, fixed scene)
//! - **LowMotion**: Slow movement (e.g., talking head, slow pan)
//! - **ModerateMotion**: Regular activity (e.g., walking, general movement)
//! - **HighMotion**: Intense activity (e.g., sports, action scenes)
//! - **RapidCuts**: Fast editing (e.g., montages, music videos)
//!
//! # Example
//! ```no_run
//! use video_audio_action_recognition::{ActionRecognizer, ActionRecognitionConfig};
//! use video_audio_common::Keyframe;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let config = ActionRecognitionConfig::default();
//! let recognizer = ActionRecognizer::new(config);
//!
//! let keyframes: Vec<Keyframe> = vec![]; // From keyframe extraction
//! let results = recognizer.analyze(&keyframes)?;
//!
//! for segment in results.segments {
//!     println!("{}: {:?} ({:.0}% confidence)",
//!         segment.start_time, segment.activity, segment.confidence * 100.0);
//! }
//! # Ok(())
//! # }
//! ```

pub mod plugin;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, info};
use video_audio_common::{Keyframe, ProcessingError};

/// Action recognition errors
#[derive(Debug, Error)]
pub enum ActionRecognitionError {
    #[error("Insufficient keyframes: need at least {min}, got {actual}")]
    InsufficientKeyframes { min: usize, actual: usize },
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    #[error("Image processing error: {0}")]
    ImageError(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl From<ActionRecognitionError> for ProcessingError {
    fn from(err: ActionRecognitionError) -> Self {
        ProcessingError::Other(err.to_string())
    }
}

/// Activity classification categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum ActivityType {
    /// Minimal motion (< 5% change between frames)
    Static,
    /// Slow movement (5-15% change)
    LowMotion,
    /// Regular activity (15-35% change)
    ModerateMotion,
    /// Intense activity (35-60% change)
    HighMotion,
    /// Fast editing (> 3 scene changes per minute)
    RapidCuts,
}

impl std::fmt::Display for ActivityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActivityType::Static => write!(f, "static"),
            ActivityType::LowMotion => write!(f, "low_motion"),
            ActivityType::ModerateMotion => write!(f, "moderate_motion"),
            ActivityType::HighMotion => write!(f, "high_motion"),
            ActivityType::RapidCuts => write!(f, "rapid_cuts"),
        }
    }
}

/// Configuration for action recognition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionRecognitionConfig {
    /// Minimum segment duration in seconds
    pub min_segment_duration: f64,
    /// Confidence threshold for classifications (0.0-1.0)
    pub confidence_threshold: f32,
    /// Scene change threshold (0.0-1.0, higher = more sensitive)
    pub scene_change_threshold: f32,
}

impl Default for ActionRecognitionConfig {
    fn default() -> Self {
        Self {
            min_segment_duration: 2.0,
            confidence_threshold: 0.5,
            scene_change_threshold: 0.4,
        }
    }
}

/// Action recognition segment result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionSegment {
    pub start_time: f64,
    pub end_time: f64,
    pub activity: ActivityType,
    pub confidence: f32,
    pub motion_score: f32,
    pub scene_changes: usize,
}

/// Complete action recognition result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionRecognitionResult {
    pub segments: Vec<ActionSegment>,
    pub overall_activity: ActivityType,
    pub overall_confidence: f32,
    pub total_scene_changes: usize,
}

/// Action recognizer
pub struct ActionRecognizer {
    config: ActionRecognitionConfig,
}

impl ActionRecognizer {
    /// Create a new action recognizer
    #[must_use]
    pub fn new(config: ActionRecognitionConfig) -> Self {
        Self { config }
    }

    /// Analyze keyframes to recognize activities
    pub fn analyze(
        &self,
        keyframes: &[Keyframe],
    ) -> Result<ActionRecognitionResult, ActionRecognitionError> {
        if keyframes.len() < 2 {
            return Err(ActionRecognitionError::InsufficientKeyframes {
                min: 2,
                actual: keyframes.len(),
            });
        }

        info!(
            "Analyzing {} keyframes for action recognition",
            keyframes.len()
        );

        // Compute motion scores between consecutive keyframes
        let motion_scores = self.compute_motion_scores(keyframes)?;

        // Detect scene changes
        let scene_changes = self.detect_scene_changes(keyframes, &motion_scores);

        // Segment video based on motion patterns and scene changes
        let segments = self.create_segments(keyframes, &motion_scores, &scene_changes);

        // Compute overall activity
        let (overall_activity, overall_confidence) = self.compute_overall_activity(&segments);

        debug!(
            "Detected {} scene changes, {} activity segments",
            scene_changes.len(),
            segments.len()
        );

        Ok(ActionRecognitionResult {
            segments,
            overall_activity,
            overall_confidence,
            total_scene_changes: scene_changes.len(),
        })
    }

    /// Compute motion scores between consecutive keyframes
    /// Returns normalized motion score (0.0-1.0) for each transition
    fn compute_motion_scores(
        &self,
        keyframes: &[Keyframe],
    ) -> Result<Vec<f32>, ActionRecognitionError> {
        let mut scores = Vec::with_capacity(keyframes.len() - 1);

        for i in 0..keyframes.len() - 1 {
            let kf1 = &keyframes[i];
            let kf2 = &keyframes[i + 1];

            // Compute motion score using multiple heuristics
            let motion_score = self.compute_motion_between_frames(kf1, kf2)?;
            scores.push(motion_score);
        }

        Ok(scores)
    }

    /// Compute motion score between two keyframes
    fn compute_motion_between_frames(
        &self,
        kf1: &Keyframe,
        kf2: &Keyframe,
    ) -> Result<f32, ActionRecognitionError> {
        // Method 1: Temporal distance (longer gaps = likely more motion)
        let time_diff = (kf2.timestamp - kf1.timestamp).abs() as f32;
        let temporal_score = (time_diff / 5.0).min(1.0); // Normalize to 5 seconds

        // Method 2: Sharpness change (camera motion or subject motion affects sharpness)
        let sharpness_change =
            ((kf2.sharpness - kf1.sharpness).abs() / kf1.sharpness.max(0.1)) as f32;
        let sharpness_score = sharpness_change.min(1.0);

        // Method 3: Hash difference (perceptual hash indicates visual change)
        let hash_diff = (kf1.hash ^ kf2.hash).count_ones() as f32 / 64.0; // Hamming distance
        let hash_score = hash_diff;

        // Method 4: Try to load images and compute pixel difference (if available)
        let pixel_score = self
            .compute_pixel_difference(kf1, kf2)
            .unwrap_or(hash_score);

        // Weighted combination
        let motion_score =
            temporal_score * 0.15 + sharpness_score * 0.15 + hash_score * 0.20 + pixel_score * 0.50;

        Ok(motion_score.clamp(0.0, 1.0))
    }

    /// Compute pixel-level difference between two keyframes (if images available)
    fn compute_pixel_difference(&self, kf1: &Keyframe, kf2: &Keyframe) -> Option<f32> {
        // Get first available thumbnail path for both keyframes
        let path1 = kf1.thumbnail_paths.values().next()?;
        let path2 = kf2.thumbnail_paths.values().next()?;

        // Load images
        let img1 = image::open(path1).ok()?;
        let img2 = image::open(path2).ok()?;

        // Convert to RGB
        let rgb1 = img1.to_rgb8();
        let rgb2 = img2.to_rgb8();

        // Ensure same dimensions
        if rgb1.dimensions() != rgb2.dimensions() {
            return None;
        }

        // Compute mean absolute difference (sample every 4th pixel for speed)
        let mut total_diff = 0u64;
        let mut pixel_count = 0;

        for (p1, p2) in rgb1.pixels().step_by(4).zip(rgb2.pixels().step_by(4)) {
            for (c1, c2) in p1.0.iter().zip(p2.0.iter()) {
                total_diff += (*c1 as i32 - *c2 as i32).unsigned_abs() as u64;
                pixel_count += 1;
            }
        }

        if pixel_count == 0 {
            return None;
        }

        // Normalize to 0.0-1.0 (255 is max per-channel difference)
        let mean_diff = (total_diff as f32) / (pixel_count as f32) / 255.0;
        Some(mean_diff.clamp(0.0, 1.0))
    }

    /// Detect scene changes based on motion scores
    fn detect_scene_changes(&self, _keyframes: &[Keyframe], motion_scores: &[f32]) -> Vec<usize> {
        // Estimate capacity: typically 10-20% of frames have scene changes
        let mut scene_changes = Vec::with_capacity(motion_scores.len() / 10);

        for (i, &score) in motion_scores.iter().enumerate() {
            if score > self.config.scene_change_threshold {
                scene_changes.push(i + 1); // Scene change after keyframe i
                debug!(
                    "Scene change detected at keyframe {} (score: {:.2})",
                    i + 1,
                    score
                );
            }
        }

        scene_changes
    }

    /// Create activity segments based on motion patterns
    fn create_segments(
        &self,
        keyframes: &[Keyframe],
        motion_scores: &[f32],
        scene_changes: &[usize],
    ) -> Vec<ActionSegment> {
        // Segments = scene_changes + 1 (for final segment)
        let mut segments = Vec::with_capacity(scene_changes.len() + 1);
        let mut segment_start_idx = 0;

        // Create segments at scene changes
        for &change_idx in scene_changes {
            if change_idx > segment_start_idx {
                if let Some(segment) =
                    self.create_segment(keyframes, motion_scores, segment_start_idx, change_idx)
                {
                    segments.push(segment);
                }
                segment_start_idx = change_idx;
            }
        }

        // Final segment
        if segment_start_idx < keyframes.len() {
            if let Some(segment) =
                self.create_segment(keyframes, motion_scores, segment_start_idx, keyframes.len())
            {
                segments.push(segment);
            }
        }

        // If no segments created (no scene changes), create one segment for entire video
        if segments.is_empty() && !keyframes.is_empty() {
            if let Some(segment) = self.create_segment(keyframes, motion_scores, 0, keyframes.len())
            {
                segments.push(segment);
            }
        }

        segments
    }

    /// Create a single activity segment
    fn create_segment(
        &self,
        keyframes: &[Keyframe],
        motion_scores: &[f32],
        start_idx: usize,
        end_idx: usize,
    ) -> Option<ActionSegment> {
        if start_idx >= end_idx || start_idx >= keyframes.len() {
            return None;
        }

        let start_time = keyframes[start_idx].timestamp;
        let end_time = keyframes[end_idx.min(keyframes.len() - 1)].timestamp;

        // Check minimum duration
        if end_time - start_time < self.config.min_segment_duration {
            return None;
        }

        // Compute average motion score for this segment
        let segment_scores: Vec<f32> =
            motion_scores[start_idx..end_idx.min(motion_scores.len())].to_vec();

        if segment_scores.is_empty() {
            return None;
        }

        let mean_motion: f32 = segment_scores.iter().sum::<f32>() / segment_scores.len() as f32;

        // Count scene changes within segment
        let scene_changes = segment_scores
            .iter()
            .filter(|&&s| s > self.config.scene_change_threshold)
            .count();

        // Classify activity based on motion score and scene changes
        let (activity, confidence) =
            self.classify_activity(mean_motion, scene_changes, end_time - start_time);

        Some(ActionSegment {
            start_time,
            end_time,
            activity,
            confidence,
            motion_score: mean_motion,
            scene_changes,
        })
    }

    /// Classify activity type based on motion score and scene changes
    fn classify_activity(
        &self,
        motion_score: f32,
        scene_changes: usize,
        duration: f64,
    ) -> (ActivityType, f32) {
        // Check for rapid cuts (scene changes per minute)
        let scene_change_rate = (scene_changes as f64) / (duration / 60.0);
        if scene_change_rate > 3.0 {
            return (ActivityType::RapidCuts, 0.8);
        }

        // Classify based on motion score
        let (activity, base_confidence) = if motion_score < 0.05 {
            (ActivityType::Static, 0.9)
        } else if motion_score < 0.15 {
            (ActivityType::LowMotion, 0.8)
        } else if motion_score < 0.35 {
            (ActivityType::ModerateMotion, 0.75)
        } else {
            (ActivityType::HighMotion, 0.7)
        };

        // Adjust confidence based on motion score consistency
        // (higher confidence if motion score is clearly in one category)
        let confidence = base_confidence * (1.0 - (motion_score % 0.2) / 0.2 * 0.2);

        (activity, confidence.max(0.5))
    }

    /// Compute overall activity for entire video
    fn compute_overall_activity(&self, segments: &[ActionSegment]) -> (ActivityType, f32) {
        if segments.is_empty() {
            return (ActivityType::Static, 0.5);
        }

        // Weight by duration
        let total_duration: f64 = segments.iter().map(|s| s.end_time - s.start_time).sum();

        // Pre-allocate with capacity 5 (max ActivityType enum variants: Static, LowMotion, ModerateMotion, HighMotion, RapidCuts)
        let mut weighted_scores = std::collections::HashMap::with_capacity(5);
        let mut weighted_confidence = 0.0;

        for segment in segments {
            let duration = segment.end_time - segment.start_time;
            let weight = duration / total_duration;

            *weighted_scores.entry(segment.activity).or_insert(0.0) += weight;
            weighted_confidence += segment.confidence * weight as f32;
        }

        // Find activity type with highest weighted score
        let overall_activity = weighted_scores
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(activity, _)| *activity)
            .unwrap_or(ActivityType::Static);

        (overall_activity, weighted_confidence)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_keyframe(timestamp: f64, hash: u64, sharpness: f64) -> Keyframe {
        Keyframe {
            timestamp,
            frame_number: (timestamp * 30.0) as u64,
            hash,
            sharpness,
            thumbnail_paths: HashMap::new(),
        }
    }

    #[test]
    fn test_recognizer_creation() {
        let config = ActionRecognitionConfig::default();
        let _recognizer = ActionRecognizer::new(config);
    }

    #[test]
    fn test_insufficient_keyframes() {
        let config = ActionRecognitionConfig::default();
        let recognizer = ActionRecognizer::new(config);

        let keyframes = vec![create_test_keyframe(0.0, 0x1234, 100.0)];
        let result = recognizer.analyze(&keyframes);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ActionRecognitionError::InsufficientKeyframes { .. }
        ));
    }

    #[test]
    fn test_static_scene() {
        let config = ActionRecognitionConfig::default();
        let recognizer = ActionRecognizer::new(config);

        // Create keyframes with minimal changes (similar hashes, sharpness)
        let keyframes = vec![
            create_test_keyframe(0.0, 0x1234567812345678, 100.0),
            create_test_keyframe(1.0, 0x1234567812345679, 100.1),
            create_test_keyframe(2.0, 0x123456781234567A, 100.2),
            create_test_keyframe(3.0, 0x123456781234567B, 100.1),
        ];

        let result = recognizer.analyze(&keyframes).unwrap();

        assert!(!result.segments.is_empty());
        // Should classify as static or low motion
        assert!(matches!(
            result.overall_activity,
            ActivityType::Static | ActivityType::LowMotion
        ));
    }

    #[test]
    fn test_high_motion_scene() {
        let config = ActionRecognitionConfig::default();
        let recognizer = ActionRecognizer::new(config);

        // Create keyframes with large changes (very different hashes)
        let keyframes = vec![
            create_test_keyframe(0.0, 0x0000000000000000, 100.0),
            create_test_keyframe(1.0, 0xFFFFFFFFFFFFFFFF, 50.0),
            create_test_keyframe(2.0, 0x0000000000000000, 150.0),
            create_test_keyframe(3.0, 0xFFFFFFFFFFFFFFFF, 75.0),
        ];

        let result = recognizer.analyze(&keyframes).unwrap();

        assert!(!result.segments.is_empty());
        // Should classify as high motion or rapid cuts
        assert!(matches!(
            result.overall_activity,
            ActivityType::HighMotion | ActivityType::RapidCuts | ActivityType::ModerateMotion
        ));
    }

    #[test]
    fn test_scene_change_detection() {
        let config = ActionRecognitionConfig {
            scene_change_threshold: 0.3,
            ..Default::default()
        };
        let recognizer = ActionRecognizer::new(config);

        // Create keyframes with clear scene change (very different hashes at t=2.0)
        let keyframes = vec![
            create_test_keyframe(0.0, 0x1111111111111111, 100.0),
            create_test_keyframe(1.0, 0x1111111111111112, 100.0),
            create_test_keyframe(2.0, 0xFFFFFFFFFFFFFFFF, 100.0), // Scene change
            create_test_keyframe(3.0, 0xFFFFFFFFFFFFFFFE, 100.0),
        ];

        let result = recognizer.analyze(&keyframes).unwrap();

        assert!(result.total_scene_changes > 0);
    }

    #[test]
    fn test_segment_creation() {
        let config = ActionRecognitionConfig::default();
        let recognizer = ActionRecognizer::new(config);

        // Create multiple distinct segments
        let keyframes = vec![
            // Static segment
            create_test_keyframe(0.0, 0x1111111111111111, 100.0),
            create_test_keyframe(1.0, 0x1111111111111111, 100.0),
            create_test_keyframe(2.0, 0x1111111111111111, 100.0),
            // Scene change
            create_test_keyframe(3.0, 0xFFFFFFFFFFFFFFFF, 100.0),
            // Dynamic segment
            create_test_keyframe(4.0, 0x0000000000000000, 50.0),
            create_test_keyframe(5.0, 0xFFFFFFFFFFFFFFFF, 150.0),
        ];

        let result = recognizer.analyze(&keyframes).unwrap();

        // Should create at least one segment
        assert!(!result.segments.is_empty());
        // Each segment should have valid timestamps
        for segment in &result.segments {
            assert!(segment.end_time > segment.start_time);
            assert!(segment.confidence >= 0.0 && segment.confidence <= 1.0);
        }
    }
}
