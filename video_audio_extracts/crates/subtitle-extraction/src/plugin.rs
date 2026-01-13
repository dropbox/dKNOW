//! Plugin wrapper for subtitle extraction module

use crate::{extract_subtitles, SubtitleConfig, SubtitleError};
use async_trait::async_trait;
use std::path::Path;
use std::time::Instant;
use tracing::info;
use video_extract_core::plugin::PluginData;
use video_extract_core::{
    Context, Operation, Plugin, PluginConfig, PluginError, PluginRequest, PluginResponse,
};

/// Subtitle extraction plugin implementation
pub struct SubtitleExtractionPlugin {
    config: PluginConfig,
}

impl SubtitleExtractionPlugin {
    /// Create a new subtitle extraction plugin
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
impl Plugin for SubtitleExtractionPlugin {
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
        let (track_index, language) = match &request.operation {
            Operation::SubtitleExtraction {
                track_index,
                language,
            } => (*track_index, language.clone()),
            _ => {
                return Err(PluginError::InvalidInput(
                    "Expected SubtitleExtraction operation".to_string(),
                ))
            }
        };

        if ctx.verbose {
            info!(
                "Extracting subtitles: track={:?}, language={:?}",
                track_index, language
            );
        }

        // Get input file path
        let video_path = match &request.input {
            PluginData::FilePath(path) => path,
            _ => {
                return Err(PluginError::InvalidInput(
                    "Expected file path input".to_string(),
                ))
            }
        };

        // Configure subtitle extractor
        let config = SubtitleConfig {
            track_index,
            language,
            include_formatting: false,
        };

        // Extract subtitles
        let subtitles = extract_subtitles(video_path, config).map_err(|e| match e {
            SubtitleError::NoSubtitles => {
                PluginError::ExecutionFailed("No subtitle streams found in video".to_string())
            }
            other => PluginError::ExecutionFailed(other.to_string()),
        })?;

        let duration = start.elapsed();

        if ctx.verbose {
            info!(
                "Subtitle extraction complete in {:?}: {} entries from {} track(s)",
                duration,
                subtitles.total_entries,
                subtitles.tracks.len()
            );
        }

        // Serialize subtitles to JSON
        let json = serde_json::to_value(&subtitles).map_err(PluginError::Serialization)?;

        Ok(PluginResponse {
            output: PluginData::Json(json),
            duration,
            warnings: vec![],
        })
    }
}

impl From<SubtitleError> for PluginError {
    fn from(err: SubtitleError) -> Self {
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
            name: "subtitle_extraction".to_string(),
            description: "Test subtitle extraction plugin".to_string(),
            inputs: vec!["mp4".to_string(), "mkv".to_string(), "mov".to_string()],
            outputs: vec!["Subtitles".to_string()],
            config: RuntimeConfig {
                max_file_size_mb: 1000,
                requires_gpu: false,
                experimental: false,
            },
            performance: PerformanceConfig {
                avg_processing_time_per_gb: "10s".to_string(),
                memory_per_file_mb: 50,
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
        let plugin = SubtitleExtractionPlugin::new(config);

        assert_eq!(plugin.name(), "subtitle_extraction");
        assert!(plugin.supports_input("mp4"));
        assert!(plugin.supports_input("mkv"));
        assert!(plugin.produces_output("Subtitles"));
    }
}
