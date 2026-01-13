//! Plugin wrapper for content moderation module

use crate::{CategoryScores, ModerationConfig, ModerationError, ModerationResult};
use async_trait::async_trait;
use ndarray::{Array, ShapeBuilder};
use once_cell::sync::OnceCell;
use ort::session::Session;
use ort::value::Value;
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

/// Content moderation plugin implementation with model caching
pub struct ContentModerationPlugin {
    config: PluginConfig,
    model_dir: PathBuf,
    /// Cached ONNX Session - loaded once and reused across all executions
    /// Wrapped in Mutex for interior mutability (Session::run requires &mut self)
    cached_session: Arc<OnceCell<Mutex<Session>>>,
}

impl ContentModerationPlugin {
    /// Create a new content moderation plugin with model caching
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
                "Loading NSFW detection model from {} with optimizations (first time only)",
                model_path.display()
            );

            let session = create_optimized_session(model_path)
                .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;

            info!("NSFW model loaded successfully with graph optimizations and cached for reuse");
            Ok(Mutex::new(session))
        })
    }

    /// Load plugin from YAML configuration
    pub fn from_yaml(yaml_path: impl AsRef<Path>) -> Result<Self, PluginError> {
        let contents = std::fs::read_to_string(yaml_path.as_ref())?;
        let config: PluginConfig = serde_yaml::from_str(&contents)
            .map_err(|e| PluginError::ExecutionFailed(format!("Failed to parse YAML: {}", e)))?;

        // Default model directory
        let model_dir = PathBuf::from("models/content-moderation");

        Ok(Self::new(config, model_dir))
    }

    /// Classify content with borrowed session
    fn classify_with_session(
        session_mutex: &Mutex<Session>,
        img: &image::RgbImage,
        config: &ModerationConfig,
    ) -> Result<ModerationResult, ModerationError> {
        // Preprocess image inline (same logic as ContentModerator::preprocess_image)
        let img_size = config.input_size;
        let (width, height) = img.dimensions();

        // Resize to input size
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

        // Convert to HWC format (Height-Width-Channels) for OpenNSFW model
        // OpenNSFW expects [batch, height, width, channels] input (not CHW)
        // No normalization - use raw RGB values [0-255]
        let mut array = Array::zeros((1, img_size as usize, img_size as usize, 3).f());

        for (y, row) in resized.enumerate_rows() {
            for (x, _, pixel) in row {
                let r = pixel[0] as f32;
                let g = pixel[1] as f32;
                let b = pixel[2] as f32;

                array[[0, y as usize, x as usize, 0]] = r;
                array[[0, y as usize, x as usize, 1]] = g;
                array[[0, y as usize, x as usize, 2]] = b;
            }
        }

        // Run inference
        let mut session = session_mutex.lock().map_err(|e| {
            ModerationError::ImageError(format!("Failed to lock session mutex: {}", e))
        })?;

        let input_value = Value::from_array(array).map_err(ModerationError::OrtError)?;
        let outputs = session
            .run(ort::inputs![input_value])
            .map_err(ModerationError::OrtError)?;

        // Extract output tensor (try common output names)
        let output_tensor = outputs
            .get("output")
            .or_else(|| outputs.get("outputs"))    // Yahoo OpenNSFW uses "outputs"
            .or_else(|| outputs.get("output0"))
            .or_else(|| outputs.get("predictions"))
            .ok_or_else(|| ModerationError::ImageError("Model output not found".to_string()))?;

        let scores_tensor = output_tensor
            .try_extract_tensor::<f32>()
            .map_err(ModerationError::OrtError)?;

        // Get shape and data
        let (shape, scores_data) = scores_tensor;

        // Validate output shape (should be [1, 2] for Yahoo OpenNSFW: [SFW, NSFW])
        if shape.len() != 2 || shape[1] != 2 {
            return Err(ModerationError::InvalidOutputShape(shape.to_vec()));
        }

        // Extract category scores (SFW probability, NSFW probability)
        let scores: Vec<f32> = scores_data.to_vec();
        let sfw_score = scores[0];
        let nsfw_score = scores[1];

        let is_safe = nsfw_score < config.nsfw_threshold;

        Ok(ModerationResult {
            nsfw_score,
            is_safe,
            // Map 2-class model to 5-class structure for compatibility
            // OpenNSFW only distinguishes SFW/NSFW, no granular categories
            categories: if config.include_categories {
                Some(CategoryScores {
                    drawings: 0.0,      // Not detected by OpenNSFW
                    hentai: 0.0,        // Not detected by OpenNSFW
                    neutral: sfw_score, // Map SFW to neutral
                    porn: nsfw_score,   // Map NSFW to porn (primary NSFW category)
                    sexy: 0.0,          // Not detected by OpenNSFW
                })
            } else {
                None
            },
        })
    }
}

#[async_trait]
impl Plugin for ContentModerationPlugin {
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
        let (include_categories, nsfw_threshold) = match &request.operation {
            Operation::ContentModeration {
                include_categories,
                nsfw_threshold,
            } => (*include_categories, *nsfw_threshold),
            _ => {
                return Err(PluginError::InvalidInput(
                    "Expected ContentModeration operation".to_string(),
                ))
            }
        };

        if ctx.verbose {
            info!(
                "Running content moderation (threshold: {}, include_categories: {})",
                nsfw_threshold, include_categories
            );
        }

        // Configure moderator
        let moderator_config = ModerationConfig {
            input_size: 224,
            nsfw_threshold,
            include_categories,
        };

        let model_path = self.model_dir.join("nsfw_mobilenet.onnx");

        // Get or load cached ONNX session
        let session_mutex = self.get_or_load_session(&model_path)?;

        // Process input based on type
        let result = match &request.input {
            PluginData::FilePath(path) => {
                debug!("Running content moderation on: {}", path.display());

                // Load image with optimized I/O (mozjpeg for JPEG, 3-5x faster)
                let img = load_image(path).map_err(|e| {
                    PluginError::ExecutionFailed(format!("Failed to load image: {}", e))
                })?;

                // Perform content moderation with cached session
                Self::classify_with_session(session_mutex, &img, &moderator_config).map_err(
                    |e| PluginError::ExecutionFailed(format!("Content moderation failed: {}", e)),
                )?
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

                debug!(
                    "Running content moderation on {} keyframes",
                    keyframes.len()
                );

                // Pre-allocate results Vec with keyframes.len() capacity
                let mut results = Vec::with_capacity(keyframes.len());
                let mut unsafe_count = 0;

                // Process all keyframes (no sampling for content moderation - safety critical)
                for (idx, keyframe) in keyframes.iter().enumerate() {
                    // Find the largest available thumbnail
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

                    // Perform content moderation
                    let frame_result =
                        Self::classify_with_session(session_mutex, &img, &moderator_config)
                            .map_err(|e| {
                                PluginError::ExecutionFailed(format!(
                                    "Content moderation failed on keyframe {}: {}",
                                    idx, e
                                ))
                            })?;

                    if !frame_result.is_safe {
                        unsafe_count += 1;
                    }

                    results.push(frame_result);
                }

                debug!(
                    "Content moderation complete: {} unsafe frames out of {} total",
                    unsafe_count,
                    keyframes.len()
                );

                // Return array of results
                let json = serde_json::to_value(&results).map_err(PluginError::Serialization)?;

                let duration = start.elapsed();

                return Ok(PluginResponse {
                    output: PluginData::Json(json),
                    duration,
                    warnings: if unsafe_count > 0 {
                        vec![format!(
                            "Found {} potentially unsafe frames (NSFW threshold: {})",
                            unsafe_count, nsfw_threshold
                        )]
                    } else {
                        vec![]
                    },
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
                "Content moderation complete in {:?}: NSFW score {:.3}, is_safe={}",
                duration, result.nsfw_score, result.is_safe
            );
        }

        // Serialize result to JSON
        let json = serde_json::to_value(&result).map_err(PluginError::Serialization)?;

        Ok(PluginResponse {
            output: PluginData::Json(json),
            duration,
            warnings: if !result.is_safe {
                vec![format!(
                    "Content flagged as potentially unsafe (NSFW score: {:.3})",
                    result.nsfw_score
                )]
            } else {
                vec![]
            },
        })
    }
}

impl From<ModerationError> for PluginError {
    fn from(err: ModerationError) -> Self {
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
            name: "content_moderation".to_string(),
            description: "Test content moderation plugin".to_string(),
            inputs: vec![
                "jpg".to_string(),
                "png".to_string(),
                "Keyframes".to_string(),
            ],
            outputs: vec!["ContentModeration".to_string()],
            config: RuntimeConfig {
                max_file_size_mb: 100,
                requires_gpu: false,
                experimental: false,
            },
            performance: PerformanceConfig {
                avg_processing_time_per_gb: "30s".to_string(),
                memory_per_file_mb: 128,
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
        let plugin = ContentModerationPlugin::new(config, "models/content-moderation");

        assert_eq!(plugin.name(), "content_moderation");
        assert!(plugin.supports_input("jpg"));
        assert!(plugin.supports_input("png"));
        assert!(plugin.supports_input("Keyframes"));
        assert!(plugin.produces_output("ContentModeration"));
    }
}
