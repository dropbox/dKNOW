//! Plugin wrapper for action recognition module

use crate::{ActionRecognitionConfig, ActionRecognizer};
use async_trait::async_trait;
use std::path::Path;
use std::time::Instant;
use tracing::info;
use video_audio_common::Keyframe;
use video_extract_core::plugin::PluginData;
use video_extract_core::{
    Context, Operation, Plugin, PluginConfig, PluginError, PluginRequest, PluginResponse,
};

/// Action recognition plugin implementation
pub struct ActionRecognitionPlugin {
    config: PluginConfig,
}

impl ActionRecognitionPlugin {
    /// Create a new action recognition plugin
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
impl Plugin for ActionRecognitionPlugin {
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
        let (min_segment_duration, confidence_threshold, scene_change_threshold) =
            match &request.operation {
                Operation::ActionRecognition {
                    min_segment_duration,
                    confidence_threshold,
                    scene_change_threshold,
                } => (
                    *min_segment_duration,
                    *confidence_threshold,
                    *scene_change_threshold,
                ),
                _ => {
                    return Err(PluginError::InvalidInput(
                        "Expected ActionRecognition operation".to_string(),
                    ))
                }
            };

        if ctx.verbose {
            info!(
                "Recognizing actions: min_segment={}s, confidence={:.2}, scene_threshold={:.2}",
                min_segment_duration.unwrap_or(2.0),
                confidence_threshold.unwrap_or(0.5),
                scene_change_threshold.unwrap_or(0.4),
            );
        }

        // Get input keyframes from JSON
        let keyframes: Vec<Keyframe> = match &request.input {
            PluginData::Json(json) => serde_json::from_value(json.clone()).map_err(|e| {
                PluginError::InvalidInput(format!("Failed to parse keyframes: {}", e))
            })?,
            _ => {
                return Err(PluginError::InvalidInput(
                    "Expected JSON keyframes input".to_string(),
                ))
            }
        };

        if keyframes.len() < 2 {
            return Err(PluginError::ExecutionFailed(format!(
                "Insufficient keyframes: need at least 2, got {}",
                keyframes.len()
            )));
        }

        // Configure recognizer
        let mut config = ActionRecognitionConfig::default();
        if let Some(min_segment) = min_segment_duration {
            config.min_segment_duration = min_segment;
        }
        if let Some(confidence) = confidence_threshold {
            config.confidence_threshold = confidence;
        }
        if let Some(scene_threshold) = scene_change_threshold {
            config.scene_change_threshold = scene_threshold;
        }

        // Create recognizer and analyze
        let recognizer = ActionRecognizer::new(config);
        let result = recognizer
            .analyze(&keyframes)
            .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;

        let elapsed = start.elapsed();
        let output = PluginData::Json(serde_json::to_value(&result).map_err(|e| {
            PluginError::ExecutionFailed(format!("Failed to serialize result: {}", e))
        })?);

        info!(
            "Action recognition complete: {} segments, overall={} ({:.0}% confidence) in {:.2}ms",
            result.segments.len(),
            result.overall_activity,
            result.overall_confidence * 100.0,
            elapsed.as_secs_f64() * 1000.0
        );

        Ok(PluginResponse {
            output,
            duration: elapsed,
            warnings: vec![],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use video_extract_core::plugin::{CacheConfig, PerformanceConfig, RuntimeConfig};

    fn create_test_config() -> PluginConfig {
        PluginConfig {
            name: "action-recognition".to_string(),
            description: "Action recognition plugin".to_string(),
            inputs: vec!["Keyframes".to_string()],
            outputs: vec!["ActionRecognition".to_string()],
            config: RuntimeConfig {
                max_file_size_mb: 1000,
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
                invalidate_before: std::time::SystemTime::UNIX_EPOCH,
            },
        }
    }

    #[test]
    fn test_plugin_creation() {
        let config = create_test_config();
        let plugin = ActionRecognitionPlugin::new(config);
        assert_eq!(plugin.name(), "action-recognition");
    }

    #[test]
    fn test_plugin_input_output_support() {
        let config = create_test_config();
        let plugin = ActionRecognitionPlugin::new(config);
        assert!(plugin.supports_input("Keyframes"));
        assert!(!plugin.supports_input("Audio"));
        assert!(plugin.produces_output("ActionRecognition"));
        assert!(!plugin.produces_output("Keyframes"));
    }
}
