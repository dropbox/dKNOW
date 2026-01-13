//! Plugin wrapper for smart thumbnail selection module

use crate::{ThumbnailConfig, ThumbnailError, ThumbnailSelector};
use async_trait::async_trait;
use std::path::Path;
use std::time::Instant;
use tracing::info;
use video_audio_common::Keyframe;
use video_extract_core::plugin::PluginData;
use video_extract_core::{
    Context, Operation, Plugin, PluginConfig, PluginError, PluginRequest, PluginResponse,
};

/// Smart thumbnail selection plugin implementation
pub struct SmartThumbnailPlugin {
    config: PluginConfig,
}

impl SmartThumbnailPlugin {
    /// Create a new smart thumbnail plugin
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
impl Plugin for SmartThumbnailPlugin {
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
        let (min_quality, preferred_resolution) = match &request.operation {
            Operation::SmartThumbnail {
                min_quality,
                preferred_resolution,
            } => (*min_quality, preferred_resolution.clone()),
            _ => {
                return Err(PluginError::InvalidInput(
                    "Expected SmartThumbnail operation".to_string(),
                ))
            }
        };

        if ctx.verbose {
            info!(
                "Selecting smart thumbnail: min_quality={:.2}, resolution={}",
                min_quality.unwrap_or(0.5),
                preferred_resolution.as_deref().unwrap_or("800x600")
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

        if keyframes.is_empty() {
            return Err(PluginError::ExecutionFailed(
                "No keyframes provided".to_string(),
            ));
        }

        // Configure selector
        let mut config = ThumbnailConfig::default();
        if let Some(resolution) = preferred_resolution {
            config.preferred_resolution = resolution;
        }

        let selector = ThumbnailSelector::new(config);

        // Select best thumbnail
        let result = selector.select_best(&keyframes).map_err(|e| match e {
            ThumbnailError::NoKeyframes => {
                PluginError::ExecutionFailed("No keyframes to analyze".to_string())
            }
            ThumbnailError::ImageLoad(msg) => {
                PluginError::ExecutionFailed(format!("Image loading failed: {}", msg))
            }
            other => PluginError::ExecutionFailed(other.to_string()),
        })?;

        // Check minimum quality threshold
        if let Some(min_q) = min_quality {
            if result.quality_score < min_q as f64 {
                return Err(PluginError::ExecutionFailed(format!(
                    "Best thumbnail quality {:.2} below threshold {:.2}",
                    result.quality_score, min_q
                )));
            }
        }

        let duration = start.elapsed();

        if ctx.verbose {
            info!(
                "Smart thumbnail selected in {:?}: frame {} at {:.2}s (score: {:.3})",
                duration,
                result.keyframe.frame_number,
                result.keyframe.timestamp,
                result.quality_score
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thumbnail_config() {
        let config = ThumbnailConfig::default();
        assert_eq!(config.preferred_resolution, "800x600");
        assert!((config.sharpness_weight - 0.30).abs() < 0.01);
    }
}
