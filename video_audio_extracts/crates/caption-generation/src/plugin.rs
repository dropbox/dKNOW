//! Plugin wrapper for caption generation module

use crate::{CaptionConfig, CaptionError, CaptionGenerator, CaptionResult};
use async_trait::async_trait;
use once_cell::sync::OnceCell;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tracing::{debug, info};
use video_extract_core::image_io::load_image;
use video_extract_core::plugin::PluginData;
use video_extract_core::{
    Context, Operation, Plugin, PluginConfig, PluginError, PluginRequest, PluginResponse,
};

/// Caption generation plugin implementation with model caching
pub struct CaptionGenerationPlugin {
    config: PluginConfig,
    model_dir: PathBuf,
    /// Cached CaptionGenerator - loaded once and reused across all executions
    cached_generator: Arc<OnceCell<Mutex<CaptionGenerator>>>,
}

impl CaptionGenerationPlugin {
    /// Create a new caption generation plugin with model caching
    pub fn new(config: PluginConfig, model_dir: impl AsRef<Path>) -> Self {
        Self {
            config,
            model_dir: model_dir.as_ref().to_path_buf(),
            cached_generator: Arc::new(OnceCell::new()),
        }
    }

    /// Get or load the CaptionGenerator (cached after first load)
    fn get_or_load_generator(
        &self,
        caption_config: &CaptionConfig,
    ) -> Result<&Mutex<CaptionGenerator>, PluginError> {
        self.cached_generator.get_or_try_init(|| {
            let model_path = self.model_dir.join("blip.onnx");
            let tokenizer_path = self.model_dir.join("tokenizer.json");

            info!(
                "Loading caption generation model from {} with tokenizer {} (first time only)",
                model_path.display(),
                tokenizer_path.display()
            );

            // Check if files exist
            if !model_path.exists() {
                return Err(PluginError::ExecutionFailed(format!(
                    "Caption generation model not found at {:?}",
                    model_path
                )));
            }

            if !tokenizer_path.exists() {
                return Err(PluginError::ExecutionFailed(format!(
                    "Tokenizer not found at {:?}",
                    tokenizer_path
                )));
            }

            let generator = CaptionGenerator::new(model_path, tokenizer_path, caption_config.clone())
                .map_err(|e| PluginError::ExecutionFailed(format!("Failed to load caption generator: {}", e)))?;

            info!("Caption generator loaded successfully and cached for reuse");
            Ok(Mutex::new(generator))
        })
    }

    /// Load plugin from YAML configuration
    pub fn from_yaml(yaml_path: impl AsRef<Path>) -> Result<Self, PluginError> {
        let contents = std::fs::read_to_string(yaml_path.as_ref())?;
        let config: PluginConfig = serde_yaml::from_str(&contents)
            .map_err(|e| PluginError::ExecutionFailed(format!("Failed to parse YAML: {}", e)))?;

        // Default model directory
        let model_dir = PathBuf::from("models/caption-generation");

        Ok(Self::new(config, model_dir))
    }

    /// Generate caption with cached generator
    fn generate_caption_with_generator(
        generator_mutex: &Mutex<CaptionGenerator>,
        img: &image::RgbImage,
    ) -> Result<CaptionResult, CaptionError> {
        let mut generator = generator_mutex
            .lock()
            .map_err(|e| CaptionError::InvalidOutput(format!("Failed to lock generator: {}", e)))?;

        generator.generate_caption(img)
    }
}

#[async_trait]
impl Plugin for CaptionGenerationPlugin {
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
        let (max_length, use_beam_search, num_beams) = match &request.operation {
            Operation::CaptionGeneration {
                max_length,
                use_beam_search,
                num_beams,
            } => (*max_length, *use_beam_search, *num_beams),
            _ => {
                return Err(PluginError::InvalidInput(
                    "Expected CaptionGeneration operation".to_string(),
                ))
            }
        };

        if ctx.verbose {
            info!(
                "Running caption generation (max_length: {}, beam_search: {}, beams: {})",
                max_length, use_beam_search, num_beams
            );
        }

        // Configure caption generator
        let caption_config = CaptionConfig {
            input_size: 384, // BLIP default
            max_length,
            use_beam_search,
            num_beams,
        };

        // Get or load cached generator
        let generator_mutex = self.get_or_load_generator(&caption_config)?;

        // Process input based on type
        let result = match &request.input {
            PluginData::FilePath(path) => {
                debug!("Running caption generation on: {}", path.display());

                // Load image with optimized I/O (mozjpeg for JPEG, 3-5x faster)
                let img = load_image(path).map_err(|e| {
                    PluginError::ExecutionFailed(format!("Failed to load image: {}", e))
                })?;

                // Generate caption with cached generator
                Self::generate_caption_with_generator(generator_mutex, &img).map_err(
                    |e| PluginError::ExecutionFailed(format!("Caption generation failed: {}", e)),
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
                    "Running caption generation on {} keyframes",
                    keyframes.len()
                );

                // Pre-allocate results Vec with keyframes.len() capacity
                let mut results = Vec::with_capacity(keyframes.len());

                // Process all keyframes
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

                    // Generate caption
                    let frame_result =
                        Self::generate_caption_with_generator(generator_mutex, &img)
                            .map_err(|e| {
                            PluginError::ExecutionFailed(format!(
                                "Caption generation failed on keyframe {}: {}",
                                idx, e
                            ))
                        })?;

                    results.push(frame_result);
                }

                debug!(
                    "Caption generation complete: {} captions generated",
                    keyframes.len()
                );

                // Return array of results
                let json = serde_json::to_value(&results).map_err(PluginError::Serialization)?;

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
                "Caption generation complete in {:?}: '{}'",
                duration, result.text
            );
        }

        // Serialize result to JSON
        let json = serde_json::to_value(&result).map_err(PluginError::Serialization)?;

        Ok(PluginResponse {
            output: PluginData::Json(json),
            duration,
            warnings: vec![],
        })
    }
}

impl From<CaptionError> for PluginError {
    fn from(err: CaptionError) -> Self {
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
            name: "caption_generation".to_string(),
            description: "Test caption generation plugin".to_string(),
            inputs: vec![
                "jpg".to_string(),
                "png".to_string(),
                "Keyframes".to_string(),
            ],
            outputs: vec!["CaptionGeneration".to_string()],
            config: RuntimeConfig {
                max_file_size_mb: 100,
                requires_gpu: false,
                experimental: true,
            },
            performance: PerformanceConfig {
                avg_processing_time_per_gb: "60s".to_string(),
                memory_per_file_mb: 512,
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
        let plugin = CaptionGenerationPlugin::new(config, "models/caption-generation");

        assert_eq!(plugin.name(), "caption_generation");
        assert!(plugin.supports_input("jpg"));
        assert!(plugin.supports_input("png"));
        assert!(plugin.supports_input("Keyframes"));
        assert!(plugin.produces_output("CaptionGeneration"));
    }
}
