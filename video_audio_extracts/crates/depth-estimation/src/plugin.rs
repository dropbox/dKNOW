//! Plugin wrapper for depth estimation module

use crate::{DepthConfig, DepthError, DepthMap};
use async_trait::async_trait;
use ndarray::ShapeBuilder;
use once_cell::sync::OnceCell;
use ort::session::Session;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tracing::{debug, info};
use video_extract_core::image_io::load_image;
use video_extract_core::onnx_utils::create_optimized_session;
use video_extract_core::plugin::PluginData;
use video_extract_core::{
    Context, Operation, Plugin, PluginConfig, PluginError, PluginRequest, PluginResponse,
};

/// Depth estimation plugin implementation with model caching
pub struct DepthEstimationPlugin {
    config: PluginConfig,
    model_dir: PathBuf,
    /// Cached ONNX Session - loaded once and reused across all executions
    cached_session: Arc<OnceCell<Mutex<Session>>>,
}

impl DepthEstimationPlugin {
    /// Create a new depth estimation plugin with model caching
    pub fn new(config: PluginConfig, model_dir: impl AsRef<Path>) -> Self {
        Self {
            config,
            model_dir: model_dir.as_ref().to_path_buf(),
            cached_session: Arc::new(OnceCell::new()),
        }
    }

    /// Get or load the ONNX Session (cached after first load)
    fn get_or_load_session(&self, model_path: &Path) -> Result<&Mutex<Session>, PluginError> {
        self.cached_session.get_or_try_init(|| {
            info!(
                "Loading depth estimation model from {} (first time only)",
                model_path.display()
            );

            let session = create_optimized_session(model_path)
                .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;

            info!("Depth model loaded successfully and cached for reuse");
            Ok(Mutex::new(session))
        })
    }

    /// Load plugin from YAML configuration
    pub fn from_yaml(yaml_path: impl AsRef<Path>) -> Result<Self, PluginError> {
        let contents = std::fs::read_to_string(yaml_path.as_ref())?;
        let config: PluginConfig = serde_yaml::from_str(&contents)
            .map_err(|e| PluginError::ExecutionFailed(format!("Failed to parse YAML: {}", e)))?;

        // Default model directory
        let model_dir = PathBuf::from("models/depth-estimation");

        Ok(Self::new(config, model_dir))
    }

    /// Estimate depth with borrowed session
    fn estimate_with_session(
        session_mutex: &Mutex<Session>,
        img: &image::RgbImage,
        config: &DepthConfig,
        resize_to_original: bool,
    ) -> Result<DepthMap, DepthError> {
        use image::Luma;
        use ndarray::Array;
        use ort::value::Value;

        let original_size = img.dimensions();
        let img_size = config.input_size;

        // Preprocess image (same logic as DepthEstimator::preprocess_image)
        let (width, height) = img.dimensions();

        // Resize to input size (square)
        let resized = if width != img_size || height != img_size {
            image::imageops::resize(
                img,
                img_size,
                img_size,
                image::imageops::FilterType::Triangle,
            )
        } else {
            img.clone()
        };

        // Convert to CHW format with ImageNet normalization
        let mean = [0.485, 0.456, 0.406];
        let std = [0.229, 0.224, 0.225];

        let mut array = Array::zeros((1, 3, img_size as usize, img_size as usize).f());

        for (y, row) in resized.enumerate_rows() {
            for (x, _, pixel) in row {
                let r = pixel[0] as f32 / 255.0;
                let g = pixel[1] as f32 / 255.0;
                let b = pixel[2] as f32 / 255.0;

                array[[0, 0, y as usize, x as usize]] = (r - mean[0]) / std[0];
                array[[0, 1, y as usize, x as usize]] = (g - mean[1]) / std[1];
                array[[0, 2, y as usize, x as usize]] = (b - mean[2]) / std[2];
            }
        }

        // Run inference
        let mut session = session_mutex
            .lock()
            .map_err(|e| DepthError::ImageError(format!("Failed to lock session mutex: {}", e)))?;

        let input_value = Value::from_array(array).map_err(DepthError::OrtError)?;

        // Extract output name BEFORE running inference to avoid borrow conflicts
        let output_name = session.outputs[0].name.clone();

        let outputs = session
            .run(ort::inputs![input_value])
            .map_err(DepthError::OrtError)?;

        // Extract output tensor
        let depth_tensor = outputs[output_name.as_str()]
            .try_extract_tensor::<f32>()
            .map_err(DepthError::OrtError)?;

        // Get shape and data
        let (shape, depth_data) = depth_tensor;

        // Validate output shape: [1, H, W] or [1, 1, H, W]
        let (height_out, width_out) = match shape.len() {
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

        // Create grayscale image from depth map
        let mut depth_image = image::GrayImage::new(width_out as u32, height_out as u32);

        if config.normalize_output {
            // Normalize to 0-255 range
            let range = max_depth - min_depth;
            if range > 1e-6 {
                for (i, &depth) in depth_values.iter().enumerate() {
                    let normalized = ((depth - min_depth) / range * 255.0) as u8;
                    let y = i / width_out;
                    let x = i % width_out;
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
                let y = i / width_out;
                let x = i % width_out;
                depth_image.put_pixel(x as u32, y as u32, Luma([value]));
            }
        }

        // Optionally resize to original resolution
        if resize_to_original && (width_out as u32, height_out as u32) != original_size {
            depth_image = image::imageops::resize(
                &depth_image,
                original_size.0,
                original_size.1,
                image::imageops::FilterType::CatmullRom,
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
}

#[async_trait]
impl Plugin for DepthEstimationPlugin {
    fn name(&self) -> &str {
        &self.config.name
    }

    fn config(&self) -> &PluginConfig {
        &self.config
    }

    fn supports_input(&self, input_type: &str) -> bool {
        self.config.inputs.iter().any(|s| s == input_type)
    }

    fn produces_output(&self, output_type: &str) -> bool {
        self.config.outputs.iter().any(|s| s == output_type)
    }

    async fn execute(
        &self,
        ctx: &Context,
        request: &PluginRequest,
    ) -> Result<PluginResponse, PluginError> {
        let start = Instant::now();

        // Extract operation parameters
        let (input_size, normalize, resize_to_original) = match &request.operation {
            Operation::DepthEstimation {
                input_size,
                normalize,
                resize_to_original,
            } => (*input_size, *normalize, *resize_to_original),
            _ => {
                return Err(PluginError::InvalidInput(
                    "Expected DepthEstimation operation".to_string(),
                ))
            }
        };

        if ctx.verbose {
            info!(
                "Estimating depth (input_size: {}, normalize: {}, resize: {})",
                input_size, normalize, resize_to_original
            );
        }

        // Configure estimator
        let estimator_config = DepthConfig {
            input_size,
            normalize_output: normalize,
            resize_to_original,
        };

        // Determine model file based on input size
        let model_filename = if input_size == 256 {
            "midas_v3_small.onnx"
        } else if input_size == 384 {
            "dpt_hybrid.onnx"
        } else {
            "midas_v3.onnx" // fallback
        };

        let model_path = self.model_dir.join(model_filename);

        // Get or load cached ONNX session
        let session_mutex = self.get_or_load_session(&model_path)?;

        // Process input based on type
        let depth_map = match &request.input {
            PluginData::FilePath(path) => {
                debug!("Running depth estimation on: {}", path.display());

                // Load image with optimized I/O
                let img = load_image(path).map_err(|e| {
                    PluginError::ExecutionFailed(format!("Failed to load image: {}", e))
                })?;

                // Perform depth estimation with cached session
                Self::estimate_with_session(
                    session_mutex,
                    &img,
                    &estimator_config,
                    resize_to_original,
                )
                .map_err(|e| {
                    PluginError::ExecutionFailed(format!("Depth estimation failed: {}", e))
                })?
            }
            PluginData::Bytes(_) => {
                return Err(PluginError::UnsupportedFormat(
                    "Bytes input not yet supported, use file path or Keyframes JSON".to_string(),
                ));
            }
            PluginData::Json(keyframes_json) => {
                // Parse Keyframes JSON
                let keyframes: Vec<video_audio_common::Keyframe> =
                    serde_json::from_value(keyframes_json.clone()).map_err(|e| {
                        PluginError::InvalidInput(format!("Failed to parse Keyframes JSON: {}", e))
                    })?;

                if keyframes.is_empty() {
                    return Err(PluginError::InvalidInput(
                        "No keyframes to process".to_string(),
                    ));
                }

                debug!("Running depth estimation on {} keyframes", keyframes.len());

                // Process keyframes (sample every 2 frames for efficiency)
                let sample_rate = 2;

                // Pre-allocate depth_maps Vec with estimated capacity (sampled frames)
                let mut depth_maps = Vec::with_capacity(keyframes.len().div_ceil(sample_rate));

                for (idx, keyframe) in keyframes.iter().enumerate().step_by(sample_rate) {
                    let image_path = keyframe.thumbnail_paths.values().next().ok_or_else(|| {
                        PluginError::InvalidInput(format!(
                            "Keyframe {} has no thumbnail paths",
                            idx
                        ))
                    })?;

                    debug!(
                        "Processing keyframe {} at t={:.2}s from {}",
                        idx,
                        keyframe.timestamp,
                        image_path.display()
                    );

                    // Load image
                    let img = load_image(image_path).map_err(|e| {
                        PluginError::ExecutionFailed(format!(
                            "Failed to load keyframe {} image: {}",
                            idx, e
                        ))
                    })?;

                    // Perform depth estimation
                    let frame_depth = Self::estimate_with_session(
                        session_mutex,
                        &img,
                        &estimator_config,
                        resize_to_original,
                    )
                    .map_err(|e| {
                        PluginError::ExecutionFailed(format!(
                            "Depth estimation failed on keyframe {}: {}",
                            idx, e
                        ))
                    })?;

                    depth_maps.push(frame_depth);
                }

                debug!(
                    "Depth estimation complete across {} keyframes",
                    keyframes.len()
                );

                // Return array of depth maps (without image data for JSON)
                let depth_info: Vec<_> = depth_maps
                    .iter()
                    .map(|dm| {
                        serde_json::json!({
                            "width": dm.width,
                            "height": dm.height,
                            "min_depth": dm.min_depth,
                            "max_depth": dm.max_depth,
                            "mean_depth": dm.mean_depth,
                        })
                    })
                    .collect();

                let json = serde_json::to_value(&depth_info).map_err(PluginError::Serialization)?;

                let duration = start.elapsed();

                return Ok(PluginResponse {
                    output: PluginData::Json(json),
                    duration,
                    warnings: vec![],
                });
            }
            _ => {
                return Err(PluginError::InvalidInput(
                    "Expected file path, bytes, or keyframes JSON".to_string(),
                ))
            }
        };

        let duration = start.elapsed();

        if ctx.verbose {
            info!(
                "Depth estimation complete in {:?}: {}x{} (depth range: {:.3}-{:.3})",
                duration,
                depth_map.width,
                depth_map.height,
                depth_map.min_depth,
                depth_map.max_depth
            );
        }

        // Serialize depth map info to JSON (without raw image data)
        let json = serde_json::json!({
            "width": depth_map.width,
            "height": depth_map.height,
            "min_depth": depth_map.min_depth,
            "max_depth": depth_map.max_depth,
            "mean_depth": depth_map.mean_depth,
        });

        Ok(PluginResponse {
            output: PluginData::Json(json),
            duration,
            warnings: vec![],
        })
    }
}

impl From<DepthError> for PluginError {
    fn from(err: DepthError) -> Self {
        PluginError::ExecutionFailed(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;
    use video_extract_core::plugin::{CacheConfig, PerformanceConfig, RuntimeConfig};

    fn create_test_config() -> PluginConfig {
        PluginConfig {
            name: "depth_estimation".to_string(),
            description: "Test depth estimation plugin".to_string(),
            inputs: vec![
                "jpg".to_string(),
                "png".to_string(),
                "Keyframes".to_string(),
            ],
            outputs: vec!["DepthMap".to_string()],
            config: RuntimeConfig {
                max_file_size_mb: 100,
                requires_gpu: false,
                experimental: false,
            },
            performance: PerformanceConfig {
                avg_processing_time_per_gb: "30s".to_string(),
                memory_per_file_mb: 256,
                supports_streaming: false,
            },
            cache: CacheConfig {
                enabled: true,
                version: 1,
                invalidate_before: SystemTime::UNIX_EPOCH,
            },
        }
    }

    #[test]
    fn test_plugin_creation() {
        let config = create_test_config();
        let plugin = DepthEstimationPlugin::new(config, "models/depth-estimation");

        assert_eq!(plugin.name(), "depth_estimation");
        assert!(plugin.supports_input("jpg"));
        assert!(plugin.supports_input("png"));
        assert!(plugin.supports_input("Keyframes"));
        assert!(plugin.produces_output("DepthMap"));
    }
}
