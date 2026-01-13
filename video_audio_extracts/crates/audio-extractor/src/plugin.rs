//! Plugin wrapper for audio extraction module

use crate::{extract_audio, AudioConfig, AudioFormat};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tracing::{debug, info};
use video_extract_core::plugin::PluginData;
use video_extract_core::{
    Context, Operation, Plugin, PluginConfig, PluginError, PluginRequest, PluginResponse,
};

/// Audio extraction plugin implementation
pub struct AudioExtractionPlugin {
    config: PluginConfig,
    temp_dir: PathBuf,
}

impl AudioExtractionPlugin {
    /// Create a new audio extraction plugin
    pub fn new(config: PluginConfig, temp_dir: impl AsRef<Path>) -> Self {
        Self {
            config,
            temp_dir: temp_dir.as_ref().to_path_buf(),
        }
    }

    /// Load plugin from YAML configuration
    pub fn from_yaml(yaml_path: impl AsRef<Path>) -> Result<Self, PluginError> {
        let contents = std::fs::read_to_string(yaml_path.as_ref())?;
        let config: PluginConfig = serde_yaml::from_str(&contents)
            .map_err(|e| PluginError::ExecutionFailed(format!("Failed to parse YAML: {}", e)))?;

        // Default temp directory
        let temp_dir = PathBuf::from("/tmp/video-extract/audio");
        std::fs::create_dir_all(&temp_dir)?;

        Ok(Self::new(config, temp_dir))
    }

    /// Convert Operation::Audio parameters to AudioConfig
    fn build_audio_config(sample_rate: u32, channels: u8) -> AudioConfig {
        AudioConfig {
            sample_rate,
            channels,
            format: AudioFormat::PCM, // Always PCM for ML pipelines
            normalize: false, // Use C FFI fast path (normalization not required for modern ML models)
        }
    }

    /// Generate output filename for extracted audio
    fn output_filename(&self, input_path: &Path) -> PathBuf {
        let stem = input_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("audio");
        self.temp_dir.join(format!("{}.wav", stem))
    }
}

#[async_trait]
impl Plugin for AudioExtractionPlugin {
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
        let (sample_rate, channels) = match &request.operation {
            Operation::Audio {
                sample_rate,
                channels,
            } => (*sample_rate, *channels),
            _ => {
                return Err(PluginError::InvalidInput(
                    "Expected Audio operation".to_string(),
                ))
            }
        };

        if ctx.verbose {
            info!("Extracting audio: {}Hz, {} channels", sample_rate, channels);
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

        debug!("Extracting audio from: {}", input_path.display());

        // Build audio config
        let audio_config = Self::build_audio_config(sample_rate, channels);

        // Determine output path
        let output_path = self.output_filename(&input_path);

        // Extract audio using FFmpeg
        let output_file = extract_audio(&input_path, &output_path, &audio_config)
            .map_err(|e| PluginError::ExecutionFailed(format!("Audio extraction failed: {}", e)))?;

        let duration = start.elapsed();

        if ctx.verbose {
            info!(
                "Audio extraction complete in {:?}: {}",
                duration,
                output_file.display()
            );
        }

        Ok(PluginResponse {
            output: PluginData::FilePath(output_file),
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
            name: "audio_extraction".to_string(),
            description: "Test audio extraction plugin".to_string(),
            inputs: vec!["mp4".to_string(), "mov".to_string()],
            outputs: vec!["Audio".to_string()],
            config: RuntimeConfig {
                max_file_size_mb: 10000,
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
        let plugin = AudioExtractionPlugin::new(config, "/tmp");

        assert_eq!(plugin.name(), "audio_extraction");
        assert!(plugin.supports_input("mp4"));
        assert!(plugin.supports_input("mov"));
        assert!(plugin.produces_output("Audio"));
    }

    #[test]
    fn test_audio_config_building() {
        let config = AudioExtractionPlugin::build_audio_config(16000, 1);
        assert_eq!(config.sample_rate, 16000);
        assert_eq!(config.channels, 1);
        assert_eq!(config.format, AudioFormat::PCM);
        assert!(config.normalize);
    }
}
