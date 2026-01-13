//! Caption generation module using vision-language models via ONNX Runtime
//!
//! This module provides automatic image/video captioning using pre-trained vision-language
//! models like BLIP, BLIP-2, ViT-GPT2, or LLaVA exported to ONNX format.
//!
//! # Features
//! - Image-to-text caption generation
//! - Support for multiple vision-language architectures (BLIP/BLIP-2/ViT-GPT2)
//! - Hardware acceleration via ONNX Runtime (CUDA, CoreML)
//! - Configurable max caption length and generation parameters
//!
//! # Model Architecture
//! Caption generation typically uses an encoder-decoder architecture:
//! - Vision encoder: Processes image → visual embeddings
//! - Language decoder: Generates text from visual embeddings
//!
//! # Supported Models
//! - BLIP (Salesforce): Encoder-decoder with cross-attention
//! - BLIP-2 (Salesforce): Vision encoder + Q-Former + LLM
//! - ViT-GPT2: Vision Transformer encoder + GPT-2 decoder
//! - LLaVA: CLIP vision encoder + LLaMA/Vicuna decoder
//!
//! # Example
//! ```no_run
//! use video_audio_caption_generation::{CaptionGenerator, CaptionConfig};
//! use image::open;
//!
//! # fn main() -> anyhow::Result<()> {
//! let config = CaptionConfig::default();
//! let mut generator = CaptionGenerator::new("models/caption-generation/blip_caption.onnx", config)?;
//!
//! let img = open("image.jpg")?.to_rgb8();
//! let caption = generator.generate_caption(&img)?;
//!
//! println!("Caption: {}", caption.text);
//! # Ok(())
//! # }
//! ```
//!
//! # Note on Model Complexity
//! Caption generation models are LARGE (500MB-7GB) and require significant compute:
//! - BLIP base: ~500MB, moderate quality
//! - BLIP-2 with OPT-2.7B: ~5.5GB, high quality
//! - ViT-GPT2: ~1.3GB, good quality
//! - LLaVA 7B: ~7GB+, excellent quality
//!
//! User must provide ONNX models due to size constraints.

pub mod plugin;
mod generation;

use image::RgbImage;
use ndarray::{Array, ShapeBuilder};
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;
use tracing::{debug, info};
use video_audio_common::ProcessingError;

/// Configuration for caption generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptionConfig {
    /// Input image size (common: 224 for ViT, 384 for BLIP)
    pub input_size: u32,
    /// Maximum caption length in tokens
    pub max_length: usize,
    /// Whether to use beam search (vs greedy decoding)
    pub use_beam_search: bool,
    /// Number of beams for beam search (typically 3-5)
    pub num_beams: usize,
}

impl Default for CaptionConfig {
    fn default() -> Self {
        Self {
            input_size: 384, // BLIP default
            max_length: 50,  // Reasonable caption length
            use_beam_search: true,
            num_beams: 3,
        }
    }
}

/// Caption generation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptionResult {
    /// Generated caption text
    pub text: String,
    /// Confidence score (optional, model-dependent)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f32>,
}

/// Errors that can occur during caption generation
#[derive(Error, Debug)]
pub enum CaptionError {
    #[error("ONNX Runtime error: {0}")]
    OrtError(#[from] ort::Error),

    #[error("Image processing error: {0}")]
    ImageError(String),

    #[error("Invalid model output: {0}")]
    InvalidOutput(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Model not yet implemented: {0}")]
    NotImplemented(String),
}

impl From<CaptionError> for ProcessingError {
    fn from(err: CaptionError) -> Self {
        ProcessingError::Other(err.to_string())
    }
}

/// Caption generator using vision-language models
pub struct CaptionGenerator {
    generator: generation::TextGenerator,
    config: CaptionConfig,
}

impl CaptionGenerator {
    /// Create a new caption generator
    ///
    /// # Arguments
    /// * `model_path` - Path to vision-language ONNX model
    /// * `tokenizer_path` - Path to tokenizer.json file
    /// * `config` - Caption generation configuration
    ///
    /// # Errors
    /// Returns error if model or tokenizer loading fails
    pub fn new<P: AsRef<Path>>(
        model_path: P,
        tokenizer_path: P,
        config: CaptionConfig,
    ) -> Result<Self, CaptionError> {
        let model_path = model_path.as_ref();
        let tokenizer_path = tokenizer_path.as_ref();

        info!(
            "Loading caption generation model from {:?} with tokenizer {:?}",
            model_path, tokenizer_path
        );

        let generator = generation::TextGenerator::new(tokenizer_path, model_path)?;

        debug!(
            "Caption model loaded successfully (input size: {}, max length: {})",
            config.input_size, config.max_length
        );

        Ok(Self { generator, config })
    }

    /// Generate caption for image
    ///
    /// # Arguments
    /// * `image` - RGB image to caption
    ///
    /// # Returns
    /// Caption result with generated text and optional confidence
    ///
    /// # Errors
    /// Returns error if inference fails
    pub fn generate_caption(&mut self, image: &RgbImage) -> Result<CaptionResult, CaptionError> {
        // Preprocess image
        let pixel_values = self.preprocess_image(image)?;

        // Extract 4D array from dynamic array [1, 3, 384, 384]
        let pixel_values_4d = pixel_values
            .into_dimensionality::<ndarray::Ix4>()
            .map_err(|e| {
                CaptionError::ImageError(format!("Failed to convert to 4D array: {}", e))
            })?;

        // Generate caption using greedy or beam search
        let text = if self.config.use_beam_search {
            self.generator.generate_beam_search(
                &pixel_values_4d,
                self.config.max_length,
                self.config.num_beams,
            )?
        } else {
            self.generator
                .generate_greedy(&pixel_values_4d, self.config.max_length)?
        };

        Ok(CaptionResult {
            text,
            confidence: None, // BLIP doesn't provide confidence scores directly
        })
    }

    /// Preprocess image for caption generation model
    fn preprocess_image(
        &self,
        image: &RgbImage,
    ) -> Result<Array<f32, ndarray::IxDyn>, CaptionError> {
        let (width, height) = image.dimensions();

        // Resize to input size
        let resized = if width != self.config.input_size || height != self.config.input_size {
            image::imageops::resize(
                image,
                self.config.input_size,
                self.config.input_size,
                image::imageops::FilterType::Triangle,
            )
        } else {
            image.clone()
        };

        // Convert to CHW format with ImageNet normalization
        // Standard normalization: mean=[0.485, 0.456, 0.406], std=[0.229, 0.224, 0.225]
        let mean = [0.485, 0.456, 0.406];
        let std = [0.229, 0.224, 0.225];

        let img_size = self.config.input_size;
        let mut array = Array::zeros((1, 3, img_size as usize, img_size as usize).f());

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
    fn test_caption_config_default() {
        let config = CaptionConfig::default();
        assert_eq!(config.input_size, 384);
        assert_eq!(config.max_length, 50);
        assert!(config.use_beam_search);
        assert_eq!(config.num_beams, 3);
    }

    #[test]
    fn test_caption_result_serialization() {
        let result = CaptionResult {
            text: "A cat sitting on a couch".to_string(),
            confidence: Some(0.92),
        };

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: CaptionResult = serde_json::from_str(&json).unwrap();

        assert_eq!(result.text, deserialized.text);
        assert_eq!(result.confidence, deserialized.confidence);
    }
}
