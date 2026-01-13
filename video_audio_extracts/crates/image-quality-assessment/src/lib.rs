//! Image quality assessment module using NIMA (Neural Image Assessment) via ONNX Runtime
//!
//! This module provides aesthetic and technical image quality assessment using a
//! MobileNetV2-based NIMA model exported to ONNX format. It predicts a quality score
//! distribution over 10 classes (1-10) and computes a mean quality score.
//!
//! # Features
//! - Aesthetic quality assessment (1-10 scale)
//! - Technical quality evaluation
//! - Lightweight MobileNetV2 backbone (8.5MB model)
//! - Hardware acceleration via ONNX Runtime (CUDA, TensorRT, CoreML)
//! - Batch processing support
//!
//! # Example
//! ```no_run
//! use video_audio_image_quality::{ImageQualityAssessor, QualityConfig};
//! use image::open;
//!
//! # fn main() -> anyhow::Result<()> {
//! let config = QualityConfig::default();
//! let mut assessor = ImageQualityAssessor::new("models/image-quality/nima_mobilenetv2.onnx", config)?;
//!
//! let img = open("image.jpg")?.to_rgb8();
//! let assessment = assessor.assess(&img)?;
//!
//! println!("Quality score: {:.2}/10 (std: {:.2})",
//!          assessment.mean_score, assessment.std_score);
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

/// Configuration for image quality assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityConfig {
    /// Input image size (NIMA default is 224x224)
    pub input_size: u32,
    /// Whether to return full score distribution (vs just mean/std)
    pub include_distribution: bool,
}

impl Default for QualityConfig {
    fn default() -> Self {
        Self {
            input_size: 224,
            include_distribution: false,
        }
    }
}

/// Quality assessment result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityAssessment {
    /// Mean quality score (1.0-10.0)
    pub mean_score: f32,
    /// Standard deviation of score distribution
    pub std_score: f32,
    /// Full score distribution (optional, 10 classes)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distribution: Option<Vec<f32>>,
}

/// Errors that can occur during image quality assessment
#[derive(Error, Debug)]
pub enum QualityError {
    #[error("ONNX Runtime error: {0}")]
    OrtError(#[from] ort::Error),

    #[error("Image processing error: {0}")]
    ImageError(String),

    #[error("Invalid model output shape: expected [1, 10], got {0:?}")]
    InvalidOutputShape(Vec<i64>),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

impl From<QualityError> for ProcessingError {
    fn from(err: QualityError) -> Self {
        ProcessingError::Other(err.to_string())
    }
}

/// Image quality assessor using NIMA model
pub struct ImageQualityAssessor {
    session: Session,
    config: QualityConfig,
    input_size: u32,
}

impl ImageQualityAssessor {
    /// Create a new image quality assessor
    ///
    /// # Arguments
    /// * `model_path` - Path to NIMA ONNX model
    /// * `config` - Quality assessment configuration
    ///
    /// # Errors
    /// Returns error if model loading fails
    pub fn new<P: AsRef<Path>>(model_path: P, config: QualityConfig) -> Result<Self, QualityError> {
        let model_path = model_path.as_ref();
        info!("Loading NIMA model from {:?}", model_path);

        let session = Session::builder()
            .map_err(QualityError::OrtError)?
            .commit_from_file(model_path)
            .map_err(QualityError::OrtError)?;

        debug!(
            "NIMA model loaded successfully (input size: {})",
            config.input_size
        );

        Ok(Self {
            session,
            input_size: config.input_size,
            config,
        })
    }

    /// Assess image quality
    ///
    /// # Arguments
    /// * `image` - RGB image to assess
    ///
    /// # Returns
    /// Quality assessment with mean score (1-10), standard deviation, and optional distribution
    ///
    /// # Errors
    /// Returns error if inference fails
    pub fn assess(&mut self, image: &RgbImage) -> Result<QualityAssessment, QualityError> {
        // Preprocess image
        let input = self.preprocess_image(image)?;

        // Run inference
        let input_tensor =
            TensorRef::from_array_view(input.view()).map_err(QualityError::OrtError)?;
        let outputs: SessionOutputs = self
            .session
            .run(ort::inputs![input_tensor])
            .map_err(QualityError::OrtError)?;

        // Extract output tensor
        let scores_tensor = outputs["scores"]
            .try_extract_tensor::<f32>()
            .map_err(QualityError::OrtError)?;

        // Get shape and data
        let (shape, scores_data) = scores_tensor;

        // Validate output shape
        if shape.len() != 2 || shape[1] != 10 {
            return Err(QualityError::InvalidOutputShape(shape.to_vec()));
        }

        // Extract probability distribution
        let distribution: Vec<f32> = scores_data.to_vec();

        // Compute mean score (weighted average: sum of i * prob(i) for i=1..10)
        let mean_score: f32 = distribution
            .iter()
            .enumerate()
            .map(|(i, &prob)| (i + 1) as f32 * prob)
            .sum();

        // Compute standard deviation
        let variance: f32 = distribution
            .iter()
            .enumerate()
            .map(|(i, &prob)| {
                let diff = (i + 1) as f32 - mean_score;
                diff * diff * prob
            })
            .sum();
        let std_score = variance.sqrt();

        debug!(
            "Quality assessment: mean={:.2}, std={:.2}",
            mean_score, std_score
        );

        Ok(QualityAssessment {
            mean_score,
            std_score,
            distribution: if self.config.include_distribution {
                Some(distribution)
            } else {
                None
            },
        })
    }

    /// Preprocess image for NIMA model
    fn preprocess_image(
        &self,
        image: &RgbImage,
    ) -> Result<Array<f32, ndarray::IxDyn>, QualityError> {
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
    fn test_quality_config_default() {
        let config = QualityConfig::default();
        assert_eq!(config.input_size, 224);
        assert!(!config.include_distribution);
    }

    #[test]
    fn test_quality_assessment_serialization() {
        let assessment = QualityAssessment {
            mean_score: 7.5,
            std_score: 1.2,
            distribution: Some(vec![0.0; 10]),
        };

        let json = serde_json::to_string(&assessment).unwrap();
        let deserialized: QualityAssessment = serde_json::from_str(&json).unwrap();

        assert_eq!(assessment.mean_score, deserialized.mean_score);
        assert_eq!(assessment.std_score, deserialized.std_score);
    }
}
