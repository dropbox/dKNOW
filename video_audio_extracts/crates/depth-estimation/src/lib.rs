//! Monocular depth estimation module using MiDaS/DPT via ONNX Runtime
//!
//! This module provides single-image depth estimation using MiDaS v3 or DPT
//! (Dense Prediction Transformer) models exported to ONNX format. It predicts
//! relative depth maps from RGB images for 3D reconstruction, AR/VR, and
//! cinematography applications.
//!
//! # Features
//! - Monocular depth estimation (relative depth, not metric)
//! - Support for MiDaS v3.1 models (small 256x256, large 384x384, etc.)
//! - Support for DPT models (hybrid, large, base)
//! - Hardware acceleration via ONNX Runtime (CUDA, TensorRT, CoreML)
//! - Batch processing support for video keyframes
//!
//! # Models
//! - **MiDaS v3.1 Small** (256x256): Fast, suitable for real-time (~15MB)
//! - **DPT Hybrid** (384x384): Balanced quality/speed (~400MB)
//! - **DPT Large** (384x384): Highest quality (~1.3GB)
//!
//! # Example
//! ```no_run
//! use video_audio_depth_estimation::{DepthEstimator, DepthConfig};
//! use image::open;
//!
//! # fn main() -> anyhow::Result<()> {
//! let config = DepthConfig::default();
//! let mut estimator = DepthEstimator::new("models/depth-estimation/midas_v3_small.onnx", config)?;
//!
//! let img = open("image.jpg")?.to_rgb8();
//! let depth_map = estimator.estimate(&img)?;
//!
//! println!("Depth map size: {}x{}", depth_map.width(), depth_map.height());
//! # Ok(())
//! # }
//! ```

pub mod plugin;

use image::{GrayImage, Luma, RgbImage};
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

/// Configuration for depth estimation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepthConfig {
    /// Input image size (model-dependent: 256 for MiDaS small, 384 for DPT)
    pub input_size: u32,
    /// Whether to normalize output depth map to 0-255 range
    pub normalize_output: bool,
    /// Whether to apply bicubic interpolation to match input resolution
    pub resize_to_original: bool,
}

impl Default for DepthConfig {
    fn default() -> Self {
        Self {
            input_size: 384,
            normalize_output: true,
            resize_to_original: false,
        }
    }
}

/// Depth estimation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepthMap {
    /// Width of depth map
    pub width: u32,
    /// Height of depth map
    pub height: u32,
    /// Minimum depth value in map
    pub min_depth: f32,
    /// Maximum depth value in map
    pub max_depth: f32,
    /// Mean depth value
    pub mean_depth: f32,
    /// Depth map as grayscale image (normalized 0-255)
    #[serde(skip)]
    pub image: GrayImage,
}

/// Errors that can occur during depth estimation
#[derive(Error, Debug)]
pub enum DepthError {
    #[error("ONNX Runtime error: {0}")]
    OrtError(#[from] ort::Error),

    #[error("Image processing error: {0}")]
    ImageError(String),

    #[error("Invalid model output shape: expected [1, H, W] or [1, 1, H, W], got {0:?}")]
    InvalidOutputShape(Vec<i64>),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

impl From<DepthError> for ProcessingError {
    fn from(err: DepthError) -> Self {
        ProcessingError::Other(err.to_string())
    }
}

/// Monocular depth estimator using MiDaS/DPT models
pub struct DepthEstimator {
    session: Session,
    config: DepthConfig,
    input_size: u32,
}

impl DepthEstimator {
    /// Create a new depth estimator
    ///
    /// # Arguments
    /// * `model_path` - Path to MiDaS/DPT ONNX model
    /// * `config` - Depth estimation configuration
    ///
    /// # Errors
    /// Returns error if model loading fails
    pub fn new<P: AsRef<Path>>(model_path: P, config: DepthConfig) -> Result<Self, DepthError> {
        let model_path = model_path.as_ref();
        info!("Loading depth estimation model from {:?}", model_path);

        let session = Session::builder()
            .map_err(DepthError::OrtError)?
            .commit_from_file(model_path)
            .map_err(DepthError::OrtError)?;

        debug!(
            "Depth model loaded successfully (input size: {})",
            config.input_size
        );

        Ok(Self {
            session,
            input_size: config.input_size,
            config,
        })
    }

    /// Estimate depth from image
    ///
    /// # Arguments
    /// * `image` - RGB image to process
    ///
    /// # Returns
    /// Depth map with statistics and grayscale visualization
    ///
    /// # Errors
    /// Returns error if inference fails
    pub fn estimate(&mut self, image: &RgbImage) -> Result<DepthMap, DepthError> {
        let original_size = image.dimensions();

        // Preprocess image
        let input = self.preprocess_image(image)?;

        // Run inference
        let input_tensor =
            TensorRef::from_array_view(input.view()).map_err(DepthError::OrtError)?;

        // Extract output name BEFORE running inference to avoid borrow conflicts
        let output_name = self.session.outputs[0].name.clone();

        let outputs: SessionOutputs = self
            .session
            .run(ort::inputs![input_tensor])
            .map_err(DepthError::OrtError)?;

        // Extract output tensor
        let depth_tensor = outputs[output_name.as_str()]
            .try_extract_tensor::<f32>()
            .map_err(DepthError::OrtError)?;

        // Get shape and data
        let (shape, depth_data) = depth_tensor;

        // Validate output shape: [1, H, W] or [1, 1, H, W]
        let (height, width) = match shape.len() {
            3 => (shape[1] as usize, shape[2] as usize),
            4 => (shape[2] as usize, shape[3] as usize),
            _ => return Err(DepthError::InvalidOutputShape(shape.to_vec())),
        };

        // Convert to Vec and compute statistics
        let depth_values: Vec<f32> = depth_data.to_vec();
        let min_depth = depth_values
            .iter()
            .copied()
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0);
        let max_depth = depth_values
            .iter()
            .copied()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(1.0);
        let mean_depth: f32 = depth_values.iter().sum::<f32>() / depth_values.len() as f32;

        debug!(
            "Depth map: min={:.3}, max={:.3}, mean={:.3}",
            min_depth, max_depth, mean_depth
        );

        // Create grayscale image from depth map
        let mut depth_image = GrayImage::new(width as u32, height as u32);

        if self.config.normalize_output {
            // Normalize to 0-255 range
            let range = max_depth - min_depth;
            if range > 1e-6 {
                for (i, &depth) in depth_values.iter().enumerate() {
                    let normalized = ((depth - min_depth) / range * 255.0) as u8;
                    let y = i / width;
                    let x = i % width;
                    depth_image.put_pixel(x as u32, y as u32, Luma([normalized]));
                }
            } else {
                // Constant depth, use mid-gray
                for pixel in depth_image.pixels_mut() {
                    *pixel = Luma([128]);
                }
            }
        } else {
            // Direct mapping (clamp to 0-255)
            for (i, &depth) in depth_values.iter().enumerate() {
                let value = depth.clamp(0.0, 255.0) as u8;
                let y = i / width;
                let x = i % width;
                depth_image.put_pixel(x as u32, y as u32, Luma([value]));
            }
        }

        // Optionally resize to original resolution
        if self.config.resize_to_original && (width as u32, height as u32) != original_size {
            depth_image = image::imageops::resize(
                &depth_image,
                original_size.0,
                original_size.1,
                image::imageops::FilterType::CatmullRom, // Bicubic-like
            );
        }

        let (final_width, final_height) = depth_image.dimensions();

        Ok(DepthMap {
            width: final_width,
            height: final_height,
            min_depth,
            max_depth,
            mean_depth,
            image: depth_image,
        })
    }

    /// Preprocess image for depth estimation
    ///
    /// Uses ImageNet normalization (standard for MiDaS/DPT models)
    fn preprocess_image(&self, image: &RgbImage) -> Result<Array<f32, ndarray::IxDyn>, DepthError> {
        let (width, height) = image.dimensions();

        // Resize to input size (square)
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
