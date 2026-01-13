//! Acoustic scene classification module using YAMNet
//!
//! This module provides acoustic scene classification by filtering YAMNet's audio
//! classification results for environmental scene categories. YAMNet includes 5
//! acoustic scene classes (IDs 500-504) that describe the recording environment.
//!
//! # Scene Classes
//! - **Inside, small room** (ID 500): Bedroom, office, bathroom
//! - **Inside, large room or hall** (ID 501): Auditorium, gymnasium, large hall
//! - **Inside, public space** (ID 502): Restaurant, airport, train station, mall
//! - **Outside, urban or manmade** (ID 503): City street, parking lot, construction site
//! - **Outside, rural or natural** (ID 504): Forest, beach, park, countryside
//!
//! # Features
//! - Focused scene classification (5 scene types)
//! - Temporal segmentation (classify scene changes over time)
//! - Confidence thresholds for filtering low-confidence predictions
//! - Uses YAMNet model (same as audio-classification plugin)
//!
//! # Example
//! ```no_run
//! use video_audio_acoustic_scene_classification::{AcousticSceneClassifier, AcousticSceneConfig};
//!
//! # fn main() -> anyhow::Result<()> {
//! let config = AcousticSceneConfig::default();
//! let mut classifier = AcousticSceneClassifier::new("models/audio-classification/yamnet.onnx", config)?;
//!
//! let audio_samples = vec![0.0f32; 48000];  // 3 seconds at 16kHz
//! let scenes = classifier.classify_scenes(&audio_samples)?;
//!
//! for scene in scenes {
//!     println!("{:.1}s-{:.1}s: {} ({:.1}%)",
//!         scene.start_time, scene.end_time, scene.scene_name, scene.confidence * 100.0);
//! }
//! # Ok(())
//! # }
//! ```

pub mod plugin;

use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;
use tracing::{debug, info};
use video_audio_classification::{AudioClassificationConfig, AudioClassifier};
use video_audio_common::ProcessingError;

/// Acoustic scene classification errors
#[derive(Debug, Error)]
pub enum AcousticSceneError {
    #[error("Audio classification failed: {0}")]
    Classification(String),
    #[error("Invalid audio length: {0}")]
    InvalidAudioLength(String),
}

impl From<AcousticSceneError> for ProcessingError {
    fn from(err: AcousticSceneError) -> Self {
        ProcessingError::Other(err.to_string())
    }
}

/// Configuration for acoustic scene classification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcousticSceneConfig {
    /// Minimum confidence threshold for scene detection (0.0-1.0)
    pub confidence_threshold: f32,
    /// Segment duration in seconds (YAMNet uses 3-second segments)
    pub segment_duration: f32,
}

impl Default for AcousticSceneConfig {
    fn default() -> Self {
        Self {
            confidence_threshold: 0.2, // Lower threshold for scenes (they're often subtle)
            segment_duration: 3.0,
        }
    }
}

impl AcousticSceneConfig {
    /// Create a config for high-confidence scene detection
    #[must_use]
    pub fn high_confidence() -> Self {
        Self {
            confidence_threshold: 0.4,
            segment_duration: 3.0,
        }
    }
}

/// Acoustic scene type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SceneType {
    /// Inside, small room (bedroom, office, bathroom)
    InsideSmallRoom,
    /// Inside, large room or hall (auditorium, gymnasium)
    InsideLargeRoom,
    /// Inside, public space (restaurant, airport, mall)
    InsidePublicSpace,
    /// Outside, urban or manmade (city street, parking lot)
    OutsideUrban,
    /// Outside, rural or natural (forest, beach, park)
    OutsideRural,
}

impl SceneType {
    /// Get the YAMNet class ID for this scene type
    #[must_use]
    pub const fn class_id(self) -> usize {
        match self {
            Self::InsideSmallRoom => 500,
            Self::InsideLargeRoom => 501,
            Self::InsidePublicSpace => 502,
            Self::OutsideUrban => 503,
            Self::OutsideRural => 504,
        }
    }

    /// Get human-readable scene name
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::InsideSmallRoom => "Inside, small room",
            Self::InsideLargeRoom => "Inside, large room or hall",
            Self::InsidePublicSpace => "Inside, public space",
            Self::OutsideUrban => "Outside, urban or manmade",
            Self::OutsideRural => "Outside, rural or natural",
        }
    }

    /// Create from YAMNet class ID
    #[must_use]
    pub const fn from_class_id(id: usize) -> Option<Self> {
        match id {
            500 => Some(Self::InsideSmallRoom),
            501 => Some(Self::InsideLargeRoom),
            502 => Some(Self::InsidePublicSpace),
            503 => Some(Self::OutsideUrban),
            504 => Some(Self::OutsideRural),
            _ => None,
        }
    }
}

/// Acoustic scene detection result for a time segment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneDetection {
    /// Start time in seconds
    pub start_time: f32,
    /// End time in seconds
    pub end_time: f32,
    /// Detected scene type
    pub scene_type: SceneType,
    /// Human-readable scene name
    pub scene_name: String,
    /// Confidence score (0-1)
    pub confidence: f32,
}

/// Acoustic scene classifier using YAMNet
pub struct AcousticSceneClassifier {
    audio_classifier: AudioClassifier,
    _config: AcousticSceneConfig,
}

impl AcousticSceneClassifier {
    /// Create a new acoustic scene classifier with the given ONNX model path
    ///
    /// # Arguments
    /// * `model_path` - Path to YAMNet ONNX model file
    /// * `config` - Scene classification configuration
    ///
    /// # Errors
    /// Returns error if model loading fails
    pub fn new<P: AsRef<Path>>(
        model_path: P,
        config: AcousticSceneConfig,
    ) -> Result<Self, AcousticSceneError> {
        info!("Initializing acoustic scene classifier");

        // Create audio classifier with custom config optimized for scene detection
        let audio_config = AudioClassificationConfig {
            confidence_threshold: config.confidence_threshold,
            top_k: 10, // Check top 10 to find scene classes
            segment_duration: config.segment_duration,
        };

        let audio_classifier = AudioClassifier::new(model_path, audio_config)
            .map_err(|e| AcousticSceneError::Classification(e.to_string()))?;

        info!("Acoustic scene classifier ready");

        Ok(Self {
            audio_classifier,
            _config: config,
        })
    }

    /// Classify acoustic scenes in audio samples
    ///
    /// # Arguments
    /// * `audio` - Audio samples (16kHz mono, float32, normalized to [-1, 1])
    ///
    /// # Returns
    /// Vector of scene detections, one per 3-second segment where a scene is detected
    ///
    /// # Errors
    /// Returns error if audio length is invalid or classification fails
    pub fn classify_scenes(
        &mut self,
        audio: &[f32],
    ) -> Result<Vec<SceneDetection>, AcousticSceneError> {
        debug!("Classifying acoustic scenes for {} samples", audio.len());

        // Run full audio classification
        let segment_results = self
            .audio_classifier
            .classify(audio)
            .map_err(|e| AcousticSceneError::Classification(e.to_string()))?;

        // Estimate capacity: typically 1-2 scene detections per segment
        let mut scene_detections = Vec::with_capacity(segment_results.len() * 2);

        // Filter for scene classes (500-504) and create scene detections
        for segment in &segment_results {
            // Look for scene classes in this segment's results
            for result in &segment.results {
                if let Some(scene_type) = SceneType::from_class_id(result.class_id) {
                    // Found a scene class above threshold
                    scene_detections.push(SceneDetection {
                        start_time: segment.start_time,
                        end_time: segment.end_time,
                        scene_type,
                        scene_name: scene_type.name().to_string(),
                        confidence: result.confidence,
                    });

                    debug!(
                        "Detected scene: {} at {:.1}s-{:.1}s ({:.1}%)",
                        scene_type.name(),
                        segment.start_time,
                        segment.end_time,
                        result.confidence * 100.0
                    );

                    // Only report the highest-confidence scene per segment
                    break;
                }
            }
        }

        info!(
            "Found {} scene detections in {} segments",
            scene_detections.len(),
            segment_results.len()
        );

        Ok(scene_detections)
    }

    /// Get the most common scene across all segments
    ///
    /// Returns the scene type with the highest average confidence across all detections
    #[must_use]
    pub fn get_dominant_scene(&self, detections: &[SceneDetection]) -> Option<SceneType> {
        if detections.is_empty() {
            return None;
        }

        // Calculate average confidence for each scene type
        // Estimate: typically 1-3 unique scene types in a segment
        let mut scene_confidences: std::collections::HashMap<SceneType, Vec<f32>> =
            std::collections::HashMap::with_capacity(3);

        for detection in detections {
            scene_confidences
                .entry(detection.scene_type)
                .or_default()
                .push(detection.confidence);
        }

        // Find scene with highest average confidence
        scene_confidences
            .into_iter()
            .max_by(|a, b| {
                let avg_a = a.1.iter().sum::<f32>() / a.1.len() as f32;
                let avg_b = b.1.iter().sum::<f32>() / b.1.len() as f32;
                avg_a
                    .partial_cmp(&avg_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(scene_type, _)| scene_type)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scene_type_mapping() {
        assert_eq!(SceneType::InsideSmallRoom.class_id(), 500);
        assert_eq!(SceneType::InsideLargeRoom.class_id(), 501);
        assert_eq!(SceneType::InsidePublicSpace.class_id(), 502);
        assert_eq!(SceneType::OutsideUrban.class_id(), 503);
        assert_eq!(SceneType::OutsideRural.class_id(), 504);

        assert_eq!(
            SceneType::from_class_id(500),
            Some(SceneType::InsideSmallRoom)
        );
        assert_eq!(SceneType::from_class_id(999), None);
    }

    #[test]
    fn test_scene_names() {
        assert_eq!(SceneType::InsideSmallRoom.name(), "Inside, small room");
        assert_eq!(
            SceneType::InsideLargeRoom.name(),
            "Inside, large room or hall"
        );
        assert_eq!(SceneType::InsidePublicSpace.name(), "Inside, public space");
        assert_eq!(SceneType::OutsideUrban.name(), "Outside, urban or manmade");
        assert_eq!(SceneType::OutsideRural.name(), "Outside, rural or natural");
    }

    #[test]
    fn test_config() {
        let config = AcousticSceneConfig::default();
        assert_eq!(config.confidence_threshold, 0.2);
        assert_eq!(config.segment_duration, 3.0);

        let high_conf = AcousticSceneConfig::high_confidence();
        assert_eq!(high_conf.confidence_threshold, 0.4);
    }
}
