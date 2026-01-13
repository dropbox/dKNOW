//! Plugin wrapper for metadata extraction module

use crate::{extract_metadata, MetadataConfig};
use async_trait::async_trait;
use std::path::Path;
use std::time::Instant;
use tracing::{debug, info};
use video_extract_core::plugin::PluginData;
use video_extract_core::{
    Context, Operation, Plugin, PluginConfig, PluginError, PluginRequest, PluginResponse,
};

/// Metadata extraction plugin implementation
pub struct MetadataExtractionPlugin {
    config: PluginConfig,
}

impl MetadataExtractionPlugin {
    /// Create a new metadata extraction plugin
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
impl Plugin for MetadataExtractionPlugin {
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
        let include_streams = match &request.operation {
            Operation::Metadata { include_streams } => *include_streams,
            _ => {
                return Err(PluginError::InvalidInput(
                    "Expected Metadata operation".to_string(),
                ))
            }
        };

        if ctx.verbose {
            info!("Extracting metadata (include_streams: {})", include_streams);
        }

        // Get input file path
        let input_path = match &request.input {
            PluginData::FilePath(path) => path.clone(),
            PluginData::Bytes(_) => {
                return Err(PluginError::UnsupportedFormat(
                    "Bytes input not yet supported for metadata extraction".to_string(),
                ))
            }
            _ => {
                return Err(PluginError::InvalidInput(
                    "Expected file path or bytes".to_string(),
                ))
            }
        };

        debug!("Extracting metadata from: {}", input_path.display());

        // Build metadata config
        let metadata_config = MetadataConfig { include_streams };

        // Extract metadata using ffprobe
        let metadata = extract_metadata(&input_path, &metadata_config).map_err(|e| {
            PluginError::ExecutionFailed(format!("Metadata extraction failed: {}", e))
        })?;

        let duration = start.elapsed();

        if ctx.verbose {
            info!("Metadata extraction complete in {:?}", duration);
        }

        // Serialize metadata to JSON Value
        let metadata_value = serde_json::to_value(&metadata).map_err(|e| {
            PluginError::ExecutionFailed(format!("Failed to serialize metadata: {}", e))
        })?;

        Ok(PluginResponse {
            output: PluginData::Json(metadata_value),
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
            name: "metadata-extraction".to_string(),
            description: "Extract media file metadata".to_string(),
            inputs: vec!["MediaFile".to_string()],
            outputs: vec!["Metadata".to_string()],
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
                version: 2,
                invalidate_before: SystemTime::UNIX_EPOCH,
            },
        }
    }

    #[test]
    fn test_plugin_creation() {
        let config = create_test_config();
        let plugin = MetadataExtractionPlugin::new(config);
        assert_eq!(plugin.name(), "metadata-extraction");
    }

    #[test]
    fn test_supports_input() {
        let config = create_test_config();
        let plugin = MetadataExtractionPlugin::new(config);
        assert!(plugin.supports_input("MediaFile"));
        assert!(!plugin.supports_input("Audio"));
    }

    #[test]
    fn test_produces_output() {
        let config = create_test_config();
        let plugin = MetadataExtractionPlugin::new(config);
        assert!(plugin.produces_output("Metadata"));
        assert!(!plugin.produces_output("Audio"));
    }
}
