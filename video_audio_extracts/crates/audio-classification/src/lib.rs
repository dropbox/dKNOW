//! Audio classification module using YAMNet via ONNX Runtime
//!
//! This module provides audio event classification using the YAMNet model,
//! which can recognize 521 different audio event classes from the AudioSet ontology.
//!
//! # Features
//! - 521 audio event classes (speech, music, environmental sounds, etc.)
//! - Temporal segmentation (classify audio segments over time)
//! - Confidence thresholds for filtering low-confidence predictions
//! - Top-k results per segment
//! - Hardware acceleration via ONNX Runtime (CoreML on macOS, CUDA on Linux)
//!
//! # Model Details
//! - Model: YAMNet (MobileNetV1 architecture)
//! - Input: 16kHz mono audio, 3-second segments (48,000 samples)
//! - Output: 521 class probabilities per segment
//! - Size: ~15MB ONNX model
//!
//! # Example
//! ```no_run
//! use video_audio_classification::{AudioClassifier, AudioClassificationConfig};
//!
//! # fn main() -> anyhow::Result<()> {
//! let config = AudioClassificationConfig::default();
//! let mut classifier = AudioClassifier::new("models/audio-classification/yamnet.onnx", config)?;
//!
//! let audio_samples = vec![0.0f32; 48000];  // 3 seconds at 16kHz
//! let results = classifier.classify(&audio_samples)?;
//!
//! for result in results {
//!     println!("{}: {:.2}%", result.class_name, result.confidence * 100.0);
//! }
//! # Ok(())
//! # }
//! ```

pub mod plugin;

use ndarray::Array1;
use ort::{session::Session, value::TensorRef};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use thiserror::Error;
use tracing::{debug, info};
use video_audio_common::ProcessingError;

/// Audio classification errors
#[derive(Debug, Error)]
pub enum AudioClassificationError {
    #[error("Model loading failed: {0}")]
    ModelLoad(String),
    #[error("Inference failed: {0}")]
    Inference(String),
    #[error("Invalid audio length: expected multiple of {expected}, got {actual}")]
    InvalidAudioLength { expected: usize, actual: usize },
    #[error("Class map loading failed: {0}")]
    ClassMapLoad(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl From<AudioClassificationError> for ProcessingError {
    fn from(err: AudioClassificationError) -> Self {
        ProcessingError::Other(err.to_string())
    }
}

/// Configuration for audio classification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioClassificationConfig {
    /// Minimum confidence threshold for classifications (0.0-1.0)
    pub confidence_threshold: f32,
    /// Number of top results to return per segment
    pub top_k: usize,
    /// Segment duration in seconds (YAMNet uses 3-second segments)
    pub segment_duration: f32,
}

impl Default for AudioClassificationConfig {
    fn default() -> Self {
        Self {
            confidence_threshold: 0.3,
            top_k: 5,
            segment_duration: 3.0,
        }
    }
}

impl AudioClassificationConfig {
    /// Create a config for high-confidence results only
    #[must_use]
    pub fn high_confidence() -> Self {
        Self {
            confidence_threshold: 0.5,
            top_k: 3,
            segment_duration: 3.0,
        }
    }

    /// Create a config for comprehensive results (low threshold, more results)
    #[must_use]
    pub fn comprehensive() -> Self {
        Self {
            confidence_threshold: 0.1,
            top_k: 10,
            segment_duration: 3.0,
        }
    }
}

/// Audio classification result for a single class
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationResult {
    /// Class ID (0-520)
    pub class_id: usize,
    /// Human-readable class name
    pub class_name: String,
    /// Confidence score (0-1)
    pub confidence: f32,
}

/// Audio classification result for a time segment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentClassification {
    /// Start time in seconds
    pub start_time: f32,
    /// End time in seconds
    pub end_time: f32,
    /// Top-k classification results for this segment
    pub results: Vec<ClassificationResult>,
}

/// Audio classifier using YAMNet ONNX model
pub struct AudioClassifier {
    session: Session,
    config: AudioClassificationConfig,
    class_map: HashMap<usize, String>,
}

impl AudioClassifier {
    /// Create a new audio classifier with the given ONNX model path
    ///
    /// # Arguments
    /// * `model_path` - Path to YAMNet ONNX model file
    /// * `config` - Classification configuration
    ///
    /// # Errors
    /// Returns error if model loading fails or class map cannot be loaded
    pub fn new<P: AsRef<Path>>(
        model_path: P,
        config: AudioClassificationConfig,
    ) -> Result<Self, AudioClassificationError> {
        let model_path_ref = model_path.as_ref();
        info!("Loading YAMNet model from {:?}", model_path_ref);

        let session = Session::builder()
            .map_err(|e| AudioClassificationError::ModelLoad(e.to_string()))?
            .commit_from_file(model_path_ref)
            .map_err(|e| AudioClassificationError::ModelLoad(e.to_string()))?;

        info!("YAMNet model loaded successfully");

        // Load class map from same directory as model
        let class_map_path = model_path_ref
            .parent()
            .unwrap()
            .join("yamnet_class_map.csv");

        let class_map = Self::load_class_map(&class_map_path)?;
        info!("Loaded {} audio event classes", class_map.len());

        Ok(Self {
            session,
            config,
            class_map,
        })
    }

    /// Load YAMNet class map from CSV file
    ///
    /// Format: "index class_name"
    fn load_class_map<P: AsRef<Path>>(
        path: P,
    ) -> Result<HashMap<usize, String>, AudioClassificationError> {
        let content = fs::read_to_string(path)
            .map_err(|e| AudioClassificationError::ClassMapLoad(e.to_string()))?;

        let line_count = content.lines().count();
        let mut map = HashMap::with_capacity(line_count);
        for line in content.lines() {
            let parts: Vec<&str> = line.splitn(2, ' ').collect();
            if parts.len() == 2 {
                if let Ok(idx) = parts[0].parse::<usize>() {
                    map.insert(idx, parts[1].to_string());
                }
            }
        }

        if map.is_empty() {
            return Err(AudioClassificationError::ClassMapLoad(
                "Empty class map".to_string(),
            ));
        }

        Ok(map)
    }

    /// Classify audio samples
    ///
    /// # Arguments
    /// * `audio` - Audio samples (16kHz mono, float32, normalized to [-1, 1])
    ///
    /// # Returns
    /// Vector of segment classifications, one per 3-second segment
    ///
    /// # Errors
    /// Returns error if audio length is invalid or inference fails
    pub fn classify(
        &mut self,
        audio: &[f32],
    ) -> Result<Vec<SegmentClassification>, AudioClassificationError> {
        const SEGMENT_SAMPLES: usize = 48000; // 3 seconds at 16kHz
        const SAMPLE_RATE: f32 = 16000.0;

        // Handle short audio by padding with zeros to minimum segment length
        let audio_data: std::borrow::Cow<[f32]> = if audio.len() < SEGMENT_SAMPLES {
            let mut padded = Vec::with_capacity(SEGMENT_SAMPLES);
            padded.extend_from_slice(audio);
            padded.resize(SEGMENT_SAMPLES, 0.0);
            std::borrow::Cow::Owned(padded)
        } else {
            std::borrow::Cow::Borrowed(audio)
        };

        // Split audio into 3-second segments
        let num_segments = audio_data.len() / SEGMENT_SAMPLES;
        let mut results = Vec::with_capacity(num_segments);

        for segment_idx in 0..num_segments {
            let start_sample = segment_idx * SEGMENT_SAMPLES;
            let end_sample = start_sample + SEGMENT_SAMPLES;
            let segment_audio = &audio_data[start_sample..end_sample];

            let start_time = (start_sample as f32) / SAMPLE_RATE;
            let end_time = (end_sample as f32) / SAMPLE_RATE;

            // Run inference on this segment
            let classifications = self.classify_segment(segment_audio)?;

            if !classifications.is_empty() {
                results.push(SegmentClassification {
                    start_time,
                    end_time,
                    results: classifications,
                });
            }
        }

        Ok(results)
    }

    /// Classify a single 3-second audio segment
    fn classify_segment(
        &mut self,
        audio: &[f32],
    ) -> Result<Vec<ClassificationResult>, AudioClassificationError> {
        debug!("Running YAMNet inference on {} samples", audio.len());

        // Prepare input tensor: [48000] (1D, not 2D)
        // YAMNet expects flat audio input, not batched
        let input_array = Array1::from_shape_vec(48000, audio.to_vec())
            .map_err(|e| AudioClassificationError::Inference(e.to_string()))?;

        // Convert to ONNX TensorRef
        let input_tensor = TensorRef::from_array_view(input_array.view())
            .map_err(|e| AudioClassificationError::Inference(e.to_string()))?;

        // Run inference
        let outputs = self
            .session
            .run(ort::inputs![input_tensor])
            .map_err(|e| AudioClassificationError::Inference(e.to_string()))?;

        // Extract output tensor
        let output_tensor = &outputs[0];

        // Get scores (shape: [num_frames, 521])
        // YAMNet outputs multiple predictions per 3-second segment (one per frame)
        let (shape, data) = output_tensor
            .try_extract_tensor::<f32>()
            .map_err(|e| AudioClassificationError::Inference(e.to_string()))?;

        // YAMNet output shape is [num_frames, 521]
        // We need to average across frames to get a single prediction per segment
        let num_classes = 521;
        let total_elements = data.len();
        let num_frames = total_elements / num_classes;

        debug!(
            "YAMNet output shape: {:?}, total_elements: {}, num_frames: {}, num_classes: {}",
            shape, total_elements, num_frames, num_classes
        );

        if total_elements % num_classes != 0 {
            return Err(AudioClassificationError::Inference(format!(
                "Invalid output shape: {} elements is not a multiple of {} classes",
                total_elements, num_classes
            )));
        }

        // Average scores across all frames for this segment
        let mut scores = vec![0.0f32; num_classes];
        for frame_idx in 0..num_frames {
            let frame_start = frame_idx * num_classes;
            let frame_end = frame_start + num_classes;
            let frame_scores = &data[frame_start..frame_end];
            for (class_idx, &score) in frame_scores.iter().enumerate() {
                scores[class_idx] += score;
            }
        }
        // Divide by num_frames to get average
        for score in &mut scores {
            *score /= num_frames as f32;
        }

        // Find top-k results above confidence threshold
        let mut indexed_scores: Vec<(usize, f32)> = Vec::with_capacity(num_classes);
        indexed_scores.extend(scores.iter().copied().enumerate());

        // Sort by score descending
        indexed_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Take top-k results above threshold
        let mut results = Vec::with_capacity(self.config.top_k);
        for (class_id, confidence) in indexed_scores.iter().take(self.config.top_k) {
            if *confidence >= self.config.confidence_threshold {
                let class_name = self
                    .class_map
                    .get(class_id)
                    .cloned()
                    .unwrap_or_else(|| format!("Class {}", class_id));

                results.push(ClassificationResult {
                    class_id: *class_id,
                    class_name,
                    confidence: *confidence,
                });
            }
        }

        debug!("Found {} classifications above threshold", results.len());

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_classification_config() {
        let config = AudioClassificationConfig::default();
        assert_eq!(config.confidence_threshold, 0.3);
        assert_eq!(config.top_k, 5);

        let high_conf = AudioClassificationConfig::high_confidence();
        assert_eq!(high_conf.confidence_threshold, 0.5);
        assert_eq!(high_conf.top_k, 3);
    }

    #[test]
    fn test_segment_calculation() {
        // 96000 samples = 6 seconds at 16kHz = 2 segments
        let audio_len = 96000;
        let segment_samples = 48000;
        let num_segments = audio_len / segment_samples;
        assert_eq!(num_segments, 2);
    }
}
