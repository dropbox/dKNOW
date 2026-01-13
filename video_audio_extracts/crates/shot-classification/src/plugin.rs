//! Plugin wrapper for shot classification module

use crate::{classify_shot, classify_shot_from_image};
use async_trait::async_trait;
use std::path::Path;
use std::time::Instant;
use tracing::info;
use video_extract_core::image_io::load_image;
use video_extract_core::plugin::PluginData;
use video_extract_core::{
    Context, Operation, Plugin, PluginConfig, PluginError, PluginRequest, PluginResponse,
};

/// Shot classification plugin implementation
pub struct ShotClassificationPlugin {
    config: PluginConfig,
}

impl ShotClassificationPlugin {
    /// Create a new shot classification plugin
    pub fn new(config: PluginConfig) -> Self {
        Self { config }
    }

    /// Load plugin from YAML configuration
    pub fn from_yaml(yaml_path: impl AsRef<Path>) -> Result<Self, PluginError> {
        let contents = std::fs::read_to_string(yaml_path.as_ref())?;
        let config: PluginConfig = serde_yaml::from_str(&contents)
            .map_err(|e| PluginError::ExecutionFailed(format!("Failed to parse YAML: {}", e)))?;

        Ok(Self::new(config))
    }
}

#[async_trait]
impl Plugin for ShotClassificationPlugin {
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

        // Verify operation type
        if !matches!(request.operation, Operation::ShotClassification {}) {
            return Err(PluginError::InvalidInput(
                "Expected ShotClassification operation".to_string(),
            ));
        }

        if ctx.verbose {
            info!("Classifying shot type from image");
        }

        // Get input
        let classification = match &request.input {
            PluginData::FilePath(path) => {
                // Load and classify from file
                classify_shot(path).map_err(|e| {
                    PluginError::ExecutionFailed(format!("Classification failed: {}", e))
                })?
            }
            PluginData::Json(keyframes_json) => {
                // Parse Keyframes JSON (array of Keyframe objects)
                let keyframes: Vec<video_audio_common::Keyframe> =
                    serde_json::from_value(keyframes_json.clone()).map_err(|e| {
                        PluginError::InvalidInput(format!("Failed to parse Keyframes JSON: {}", e))
                    })?;

                // Classify all frames
                // Pre-allocate results Vec with keyframes.len() capacity
                let mut results = Vec::with_capacity(keyframes.len());

                for keyframe in keyframes {
                    // Get the first available thumbnail path
                    let path = keyframe.thumbnail_paths.values().next().ok_or_else(|| {
                        PluginError::InvalidInput("No thumbnail path in keyframe".to_string())
                    })?;

                    let timestamp_ms = Some((keyframe.timestamp * 1000.0) as u64);
                    let frame_number = Some(keyframe.frame_number as u32);

                    // Load image
                    let img_buffer = load_image(Path::new(path)).map_err(|e| {
                        PluginError::ExecutionFailed(format!("Failed to load image: {}", e))
                    })?;
                    let img = image::DynamicImage::ImageRgb8(img_buffer);

                    // Classify
                    let classification = classify_shot_from_image(&img, timestamp_ms, frame_number)
                        .map_err(|e| {
                            PluginError::ExecutionFailed(format!("Classification failed: {}", e))
                        })?;

                    results.push(classification);
                }

                let duration = start.elapsed();

                if ctx.verbose {
                    info!("Classified {} frames in {:?}", results.len(), duration);
                }

                // Return batch results
                let json = serde_json::json!({
                    "shots": results,
                    "frame_count": results.len(),
                });

                return Ok(PluginResponse {
                    output: PluginData::Json(json),
                    duration,
                    warnings: vec![],
                });
            }
            _ => {
                return Err(PluginError::InvalidInput(
                    "Expected file path or Keyframes JSON".to_string(),
                ))
            }
        };

        let duration = start.elapsed();

        if ctx.verbose {
            info!(
                "Shot classification complete in {:?}: {} (confidence: {:.2})",
                duration, classification.shot_type, classification.confidence
            );
        }

        // Serialize result to JSON
        let json = serde_json::to_value(&classification).map_err(PluginError::Serialization)?;

        Ok(PluginResponse {
            output: PluginData::Json(json),
            duration,
            warnings: vec![],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;
    use video_extract_core::plugin::{CacheConfig, PerformanceConfig, RuntimeConfig};

    fn create_test_config() -> PluginConfig {
        PluginConfig {
            name: "shot_classification".to_string(),
            description: "Test shot classification plugin".to_string(),
            inputs: vec![
                "Keyframes".to_string(),
                "jpg".to_string(),
                "jpeg".to_string(),
                "png".to_string(),
            ],
            outputs: vec!["ShotClassification".to_string()],
            config: RuntimeConfig {
                max_file_size_mb: 50,
                requires_gpu: false,
                experimental: false,
            },
            performance: PerformanceConfig {
                avg_processing_time_per_gb: "10s".to_string(),
                memory_per_file_mb: 100,
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
        let plugin = ShotClassificationPlugin::new(config);

        assert_eq!(plugin.name(), "shot_classification");
        assert!(plugin.supports_input("Keyframes"));
        assert!(plugin.supports_input("jpg"));
        assert!(plugin.produces_output("ShotClassification"));
    }
}
