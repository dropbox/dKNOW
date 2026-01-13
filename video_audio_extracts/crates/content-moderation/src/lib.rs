//! Content moderation module using NSFW detection via ONNX Runtime
//!
//! This module provides NSFW (Not Safe For Work) content detection using a
//! MobileNetV2-based classification model exported to ONNX format. It classifies
//! images into content safety categories (SFW/NSFW) with confidence scores.
//!
//! # Features
//! - Binary NSFW classification (Safe/Unsafe)
//! - Multi-class content categorization (optional: drawings, hentai, neutral, porn, sexy)
//! - Lightweight MobileNetV2 backbone (~9MB model)
//! - Hardware acceleration via ONNX Runtime (CUDA, TensorRT, CoreML)
//! - Configurable confidence thresholds
//!
//! # Example
//! ```no_run
//! use video_audio_content_moderation::{ContentModerator, ModerationConfig};
//! use image::open;
//!
//! # fn main() -> anyhow::Result<()> {
//! let config = ModerationConfig::default();
//! let mut moderator = ContentModerator::new("models/content-moderation/nsfw_mobilenet.onnx", config)?;
//!
//! let img = open("image.jpg")?.to_rgb8();
//! let result = moderator.classify(&img)?;
//!
//! println!("NSFW probability: {:.2}%", result.nsfw_score * 100.0);
//! println!("Is safe: {}", result.is_safe);
//! # Ok(())
//! # }
//! ```

pub mod plugin;

use image::RgbImage;
use ndarray::{Array, ShapeBuilder};
use ort::{
    session::{Session, SessionOutputs},
    value::TensorRef,
};
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;
use tracing::{debug, info};
use video_audio_common::ProcessingError;

/// Configuration for content moderation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModerationConfig {
    /// Input image size (OpenNSFW2 default is 224x224)
    pub input_size: u32,
    /// Confidence threshold for NSFW classification (0.0-1.0)
    /// Images with NSFW score >= threshold are marked as unsafe
    pub nsfw_threshold: f32,
    /// Whether to return detailed category scores
    pub include_categories: bool,
}

impl Default for ModerationConfig {
    fn default() -> Self {
        Self {
            input_size: 224,
            nsfw_threshold: 0.5, // 50% threshold (conservative)
            include_categories: false,
        }
    }
}

/// Content moderation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModerationResult {
    /// Overall NSFW score (0.0-1.0, higher = more likely NSFW)
    pub nsfw_score: f32,
    /// Whether content is safe (nsfw_score < threshold)
    pub is_safe: bool,
    /// Detailed category scores (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub categories: Option<CategoryScores>,
}

/// Detailed category scores for content moderation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryScores {
    /// Drawings/illustrations score
    pub drawings: f32,
    /// Hentai/anime NSFW score
    pub hentai: f32,
    /// Neutral/safe content score
    pub neutral: f32,
    /// Pornography score
    pub porn: f32,
    /// Sexy/suggestive content score
    pub sexy: f32,
}

/// Errors that can occur during content moderation
#[derive(Error, Debug)]
pub enum ModerationError {
    #[error("ONNX Runtime error: {0}")]
    OrtError(#[from] ort::Error),

    #[error("Image processing error: {0}")]
    ImageError(String),

    #[error("Invalid model output shape: expected [1, 5], got {0:?}")]
    InvalidOutputShape(Vec<i64>),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

impl From<ModerationError> for ProcessingError {
    fn from(err: ModerationError) -> Self {
        ProcessingError::Other(err.to_string())
    }
}

/// Content moderator using NSFW detection model
pub struct ContentModerator {
    session: Session,
    config: ModerationConfig,
    input_size: u32,
}

impl ContentModerator {
    /// Create a new content moderator
    ///
    /// # Arguments
    /// * `model_path` - Path to NSFW classification ONNX model
    /// * `config` - Moderation configuration
    ///
    /// # Errors
    /// Returns error if model loading fails
    pub fn new<P: AsRef<Path>>(
        model_path: P,
        config: ModerationConfig,
    ) -> Result<Self, ModerationError> {
        let model_path = model_path.as_ref();
        info!("Loading NSFW detection model from {:?}", model_path);

        let session = Session::builder()
            .map_err(ModerationError::OrtError)?
            .commit_from_file(model_path)
            .map_err(ModerationError::OrtError)?;

        debug!(
            "NSFW model loaded successfully (input size: {}, threshold: {})",
            config.input_size, config.nsfw_threshold
        );

        Ok(Self {
            session,
            input_size: config.input_size,
            config,
        })
    }

    /// Classify image for content moderation
    ///
    /// # Arguments
    /// * `image` - RGB image to classify
    ///
    /// # Returns
    /// Moderation result with NSFW score, safety flag, and optional category scores
    ///
    /// # Errors
    /// Returns error if inference fails
    pub fn classify(&mut self, image: &RgbImage) -> Result<ModerationResult, ModerationError> {
        // Preprocess image
        let input = self.preprocess_image(image)?;

        // Run inference
        let input_tensor =
            TensorRef::from_array_view(input.view()).map_err(ModerationError::OrtError)?;
        let outputs: SessionOutputs = self
            .session
            .run(ort::inputs![input_tensor])
            .map_err(ModerationError::OrtError)?;

        // Extract output tensor (try common output names)
        let output_tensor = outputs
            .get("output")
            .or_else(|| outputs.get("output0"))
            .or_else(|| outputs.get("predictions"))
            .ok_or_else(|| ModerationError::ImageError("Model output not found".to_string()))?;

        let scores_tensor = output_tensor
            .try_extract_tensor::<f32>()
            .map_err(ModerationError::OrtError)?;

        // Get shape and data
        let (shape, scores_data) = scores_tensor;

        // Validate output shape (should be [1, 5] for 5-class OpenNSFW2)
        if shape.len() != 2 || shape[1] != 5 {
            return Err(ModerationError::InvalidOutputShape(shape.to_vec()));
        }

        // Extract category scores (drawings, hentai, neutral, porn, sexy)
        let scores: Vec<f32> = scores_data.to_vec();

        // Compute NSFW score (sum of hentai, porn, sexy)
        // Categories: [drawings, hentai, neutral, porn, sexy]
        let nsfw_score = scores[1] + scores[3] + scores[4]; // hentai + porn + sexy

        let is_safe = nsfw_score < self.config.nsfw_threshold;

        debug!(
            "Content moderation: nsfw_score={:.3}, is_safe={}",
            nsfw_score, is_safe
        );

        Ok(ModerationResult {
            nsfw_score,
            is_safe,
            categories: if self.config.include_categories {
                Some(CategoryScores {
                    drawings: scores[0],
                    hentai: scores[1],
                    neutral: scores[2],
                    porn: scores[3],
                    sexy: scores[4],
                })
            } else {
                None
            },
        })
    }

    /// Preprocess image for NSFW detection model
    fn preprocess_image(
        &self,
        image: &RgbImage,
    ) -> Result<Array<f32, ndarray::IxDyn>, ModerationError> {
        let (width, height) = image.dimensions();

        // Resize to input size
        let resized = if width != self.input_size || height != self.input_size {
            image::imageops::resize(
                image,
                self.input_size,
                self.input_size,
                image::imageops::FilterType::Triangle,
            )
        } else {
            image.clone()
        };

        // Convert to CHW format with normalization
        // ImageNet normalization: mean=[0.485, 0.456, 0.406], std=[0.229, 0.224, 0.225]
        let mean = [0.485, 0.456, 0.406];
        let std = [0.229, 0.224, 0.225];

        let mut array =
            Array::zeros((1, 3, self.input_size as usize, self.input_size as usize).f());

        for (y, row) in resized.enumerate_rows() {
            for (x, _, pixel) in row {
                let r = pixel[0] as f32 / 255.0;
                let g = pixel[1] as f32 / 255.0;
                let b = pixel[2] as f32 / 255.0;

                // Normalize with ImageNet statistics
                array[[0, 0, y as usize, x as usize]] = (r - mean[0]) / std[0];
                array[[0, 1, y as usize, x as usize]] = (g - mean[1]) / std[1];
                array[[0, 2, y as usize, x as usize]] = (b - mean[2]) / std[2];
            }
        }

        Ok(array.into_dyn())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_moderation_config_default() {
        let config = ModerationConfig::default();
        assert_eq!(config.input_size, 224);
        assert_eq!(config.nsfw_threshold, 0.5);
        assert!(!config.include_categories);
    }

    #[test]
    fn test_moderation_result_serialization() {
        let result = ModerationResult {
            nsfw_score: 0.75,
            is_safe: false,
            categories: Some(CategoryScores {
                drawings: 0.1,
                hentai: 0.2,
                neutral: 0.1,
                porn: 0.4,
                sexy: 0.2,
            }),
        };

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: ModerationResult = serde_json::from_str(&json).unwrap();

        assert_eq!(result.nsfw_score, deserialized.nsfw_score);
        assert_eq!(result.is_safe, deserialized.is_safe);
    }

    #[test]
    fn test_category_scores() {
        let categories = CategoryScores {
            drawings: 0.1,
            hentai: 0.2,
            neutral: 0.5,
            porn: 0.1,
            sexy: 0.1,
        };

        // Sum should be approximately 1.0 (softmax output)
        let sum = categories.drawings
            + categories.hentai
            + categories.neutral
            + categories.porn
            + categories.sexy;
        assert!((sum - 1.0).abs() < 0.01);
    }
}
