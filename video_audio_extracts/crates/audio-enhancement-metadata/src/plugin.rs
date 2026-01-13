//! Plugin wrapper for audio enhancement metadata module

use crate::{AudioEnhancementAnalyzer, EnhancementConfig, EnhancementError};
use async_trait::async_trait;
use std::path::Path;
use std::time::Instant;
use tracing::info;
use video_extract_core::plugin::PluginData;
use video_extract_core::{
    Context, Operation, Plugin, PluginConfig, PluginError, PluginRequest, PluginResponse,
};

/// Audio enhancement metadata plugin implementation
pub struct AudioEnhancementMetadataPlugin {
    config: PluginConfig,
    analyzer: AudioEnhancementAnalyzer,
}

impl AudioEnhancementMetadataPlugin {
    /// Create a new audio enhancement metadata plugin
    pub fn new(config: PluginConfig) -> Self {
        let analyzer = AudioEnhancementAnalyzer::new(EnhancementConfig::default());
        Self { config, analyzer }
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
impl Plugin for AudioEnhancementMetadataPlugin {
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
        if !matches!(request.operation, Operation::AudioEnhancementMetadata {}) {
            return Err(PluginError::InvalidInput(
                "Expected AudioEnhancementMetadata operation".to_string(),
            ));
        }

        if ctx.verbose {
            info!("Analyzing audio for enhancement recommendations");
        }

        // Get input audio file path
        let audio_path = match &request.input {
            PluginData::FilePath(path) => path,
            _ => {
                return Err(PluginError::InvalidInput(
                    "Expected file path input".to_string(),
                ))
            }
        };

        // Load audio file (WAV format)
        let (audio_data, sample_rate) = Self::load_audio(audio_path)?;

        // Analyze audio
        let metadata = self
            .analyzer
            .analyze(&audio_data, sample_rate)
            .map_err(|e| match e {
                EnhancementError::InvalidAudio(msg) => PluginError::InvalidInput(msg),
                EnhancementError::AnalysisFailed(msg) => PluginError::ExecutionFailed(msg),
            })?;

        let duration = start.elapsed();

        if ctx.verbose {
            info!(
                "Audio analysis complete in {:?}: SNR={:.2}dB, DR={:.2}dB",
                duration, metadata.snr_db, metadata.dynamic_range_db
            );
            info!("Recommendations: {:?}", metadata.recommendations);
        }

        // Serialize metadata to JSON
        let json = serde_json::to_value(&metadata).map_err(PluginError::Serialization)?;

        Ok(PluginResponse {
            output: PluginData::Json(json),
            duration,
            warnings: vec![],
        })
    }
}

impl AudioEnhancementMetadataPlugin {
    /// Load audio from WAV file (float32)
    ///
    /// Returns (samples, sample_rate)
    fn load_audio(path: &Path) -> Result<(Vec<f32>, u32), PluginError> {
        // Read WAV file manually (simple PCM parser)
        let wav_data = std::fs::read(path)?;

        // Basic WAV header parsing (44 bytes minimum)
        if wav_data.len() < 44 {
            return Err(PluginError::ExecutionFailed(
                "Invalid WAV file: too short".to_string(),
            ));
        }

        // Check RIFF header
        if &wav_data[0..4] != b"RIFF" {
            return Err(PluginError::ExecutionFailed(
                "Invalid WAV file: missing RIFF header".to_string(),
            ));
        }

        // Check WAVE format
        if &wav_data[8..12] != b"WAVE" {
            return Err(PluginError::ExecutionFailed(
                "Invalid WAV file: missing WAVE format".to_string(),
            ));
        }

        // Parse fmt chunk to get sample rate
        let mut offset = 12;
        let mut sample_rate = 0;
        let mut bits_per_sample = 0;
        let mut num_channels = 0;
        let mut data_offset = 0;
        let mut data_size = 0;

        while offset < wav_data.len() - 8 {
            let chunk_id = &wav_data[offset..offset + 4];
            let chunk_size = u32::from_le_bytes([
                wav_data[offset + 4],
                wav_data[offset + 5],
                wav_data[offset + 6],
                wav_data[offset + 7],
            ]) as usize;

            if chunk_id == b"fmt " && chunk_size >= 16 {
                sample_rate = u32::from_le_bytes([
                    wav_data[offset + 12],
                    wav_data[offset + 13],
                    wav_data[offset + 14],
                    wav_data[offset + 15],
                ]);
                num_channels = u16::from_le_bytes([wav_data[offset + 10], wav_data[offset + 11]]);
                bits_per_sample =
                    u16::from_le_bytes([wav_data[offset + 22], wav_data[offset + 23]]);
            } else if chunk_id == b"data" {
                data_offset = offset + 8;
                data_size = chunk_size;
            }

            offset += 8 + chunk_size;
        }

        if data_offset == 0 {
            return Err(PluginError::ExecutionFailed(
                "Invalid WAV file: no data chunk found".to_string(),
            ));
        }

        if sample_rate == 0 {
            return Err(PluginError::ExecutionFailed(
                "Invalid WAV file: no fmt chunk found".to_string(),
            ));
        }

        // Convert PCM to float32 normalized to [-1, 1]
        let pcm_data = &wav_data[data_offset..data_offset + data_size];
        let bytes_per_sample = (bits_per_sample / 8) as usize;
        let num_samples = pcm_data.len() / bytes_per_sample;
        let mut audio = Vec::with_capacity(num_samples);

        match bits_per_sample {
            16 => {
                for i in 0..num_samples {
                    let sample_i16 = i16::from_le_bytes([pcm_data[i * 2], pcm_data[i * 2 + 1]]);
                    let sample_f32 = (sample_i16 as f32) / 32768.0;
                    audio.push(sample_f32);
                }
            }
            32 => {
                // Assume 32-bit float
                for i in 0..num_samples {
                    let sample_f32 = f32::from_le_bytes([
                        pcm_data[i * 4],
                        pcm_data[i * 4 + 1],
                        pcm_data[i * 4 + 2],
                        pcm_data[i * 4 + 3],
                    ]);
                    audio.push(sample_f32);
                }
            }
            _ => {
                return Err(PluginError::ExecutionFailed(format!(
                    "Unsupported bits per sample: {}",
                    bits_per_sample
                )))
            }
        }

        // If stereo, convert to mono by averaging channels
        if num_channels == 2 {
            let mono: Vec<f32> = audio
                .chunks(2)
                .map(|chunk| (chunk[0] + chunk[1]) / 2.0)
                .collect();
            Ok((mono, sample_rate))
        } else {
            Ok((audio, sample_rate))
        }
    }
}

impl From<EnhancementError> for PluginError {
    fn from(err: EnhancementError) -> Self {
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
            name: "audio_enhancement_metadata".to_string(),
            description: "Test audio enhancement metadata plugin".to_string(),
            inputs: vec!["wav".to_string(), "Audio".to_string()],
            outputs: vec!["AudioEnhancementMetadata".to_string()],
            config: RuntimeConfig {
                max_file_size_mb: 500,
                requires_gpu: false,
                experimental: false,
            },
            performance: PerformanceConfig {
                avg_processing_time_per_gb: "5s".to_string(),
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
        let plugin = AudioEnhancementMetadataPlugin::new(config);

        assert_eq!(plugin.name(), "audio_enhancement_metadata");
        assert!(plugin.supports_input("wav"));
        assert!(plugin.supports_input("Audio"));
        assert!(plugin.produces_output("AudioEnhancementMetadata"));
    }
}
