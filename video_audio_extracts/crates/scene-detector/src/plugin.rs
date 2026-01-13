//! Plugin wrapper for scene detection module

use crate::{detect_scenes, SceneDetectorConfig};
use async_trait::async_trait;
use std::path::Path;
use std::time::Instant;
use tracing::{debug, info};
use video_extract_core::plugin::PluginData;
use video_extract_core::{
    Context, Operation, Plugin, PluginConfig, PluginError, PluginRequest, PluginResponse,
};

/// Scene detection plugin implementation
pub struct SceneDetectionPlugin {
    config: PluginConfig,
}

impl SceneDetectionPlugin {
    /// Create a new scene detection plugin
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
impl Plugin for SceneDetectionPlugin {
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
        let (threshold, keyframes_only) = match &request.operation {
            Operation::SceneDetection {
                threshold,
                keyframes_only,
            } => (*threshold, *keyframes_only),
            _ => {
                return Err(PluginError::InvalidInput(
                    "Expected SceneDetection operation".to_string(),
                ))
            }
        };

        if ctx.verbose {
            info!(
                "Detecting scenes (threshold: {}, keyframes_only: {})",
                threshold, keyframes_only
            );
        }

        // Get input file path
        let input_path = match &request.input {
            PluginData::FilePath(path) => path.clone(),
            PluginData::Bytes(_) => {
                return Err(PluginError::UnsupportedFormat(
                    "Bytes input not yet supported, use file path".to_string(),
                ))
            }
            _ => {
                return Err(PluginError::InvalidInput(
                    "Expected file path or bytes".to_string(),
                ))
            }
        };

        if ctx.verbose {
            debug!("Input video file: {:?}", input_path);
        }

        // Build scene detection configuration
        let scene_config = SceneDetectorConfig {
            threshold: f64::from(threshold),
            min_scene_duration: 0.0,
            keyframes_only,
        };

        if ctx.verbose {
            debug!("Scene detection config: {:?}", scene_config);
        }

        // Perform scene detection
        let scene_result = tokio::task::spawn_blocking({
            let input_path = input_path.clone();
            move || detect_scenes(&input_path, &scene_config)
        })
        .await
        .map_err(|e| PluginError::ExecutionFailed(format!("Task join error: {}", e)))?
        .map_err(|e| PluginError::ExecutionFailed(format!("Scene detection failed: {}", e)))?;

        let duration = start.elapsed();
        if ctx.verbose {
            info!(
                "Scene detection complete: {} scenes, {} boundaries in {:.2}s",
                scene_result.num_scenes,
                scene_result.boundaries.len(),
                duration.as_secs_f64()
            );
        }

        // Serialize result to JSON
        let json_value = serde_json::to_value(&scene_result).map_err(|e| {
            PluginError::ExecutionFailed(format!("JSON serialization failed: {}", e))
        })?;

        Ok(PluginResponse {
            output: PluginData::Json(json_value),
            duration,
            warnings: vec![],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::SystemTime;
    use video_extract_core::operation::Operation;
    use video_extract_core::plugin::{CacheConfig, PerformanceConfig, RuntimeConfig};
    use video_extract_core::{Context, ExecutionMode, PluginRequest};

    fn create_test_config() -> PluginConfig {
        PluginConfig {
            name: "scene_detection".to_string(),
            description: "Scene detection plugin".to_string(),
            inputs: vec!["mp4".to_string(), "mov".to_string(), "avi".to_string()],
            outputs: vec!["SceneDetection".to_string()],
            config: RuntimeConfig {
                max_file_size_mb: 10000,
                requires_gpu: false,
                experimental: false,
            },
            performance: PerformanceConfig {
                avg_processing_time_per_gb: "5s".to_string(),
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

    fn create_test_plugin() -> SceneDetectionPlugin {
        SceneDetectionPlugin::new(create_test_config())
    }

    #[test]
    fn test_plugin_metadata() {
        let plugin = create_test_plugin();
        assert_eq!(plugin.name(), "scene_detection");
        assert!(plugin.supports_input("mp4"));
        assert!(plugin.supports_input("mov"));
        assert!(plugin.produces_output("SceneDetection"));
        assert!(!plugin.supports_input("wav"));
    }

    #[test]
    fn test_unsupported_operation() {
        let plugin = create_test_plugin();
        let ctx = Context::new(ExecutionMode::Debug);
        let request = PluginRequest {
            operation: Operation::Audio {
                sample_rate: 16000,
                channels: 1,
            },
            input: PluginData::FilePath(PathBuf::from("test.mp4")),
        };

        let runtime = tokio::runtime::Runtime::new().unwrap();
        let result = runtime.block_on(plugin.execute(&ctx, &request));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Expected SceneDetection operation"));
    }

    #[test]
    fn test_unsupported_input_type() {
        let plugin = create_test_plugin();
        let ctx = Context::new(ExecutionMode::Debug);

        // Test bytes input
        let request = PluginRequest {
            operation: Operation::SceneDetection {
                threshold: 10.0,
                keyframes_only: true,
            },
            input: PluginData::Bytes(vec![1, 2, 3]),
        };

        let runtime = tokio::runtime::Runtime::new().unwrap();
        let result = runtime.block_on(plugin.execute(&ctx, &request));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not yet supported"));
    }
}
