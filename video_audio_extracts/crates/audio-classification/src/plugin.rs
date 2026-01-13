//! Plugin wrapper for audio classification module

use crate::{AudioClassificationConfig, AudioClassificationError, AudioClassifier};
use async_trait::async_trait;
use std::path::Path;
use std::time::Instant;
use tracing::info;
use video_extract_core::plugin::PluginData;
use video_extract_core::{
    Context, Operation, Plugin, PluginConfig, PluginError, PluginRequest, PluginResponse,
};

/// Audio classification plugin implementation
pub struct AudioClassificationPlugin {
    config: PluginConfig,
    classifier: Option<AudioClassifier>,
}

impl AudioClassificationPlugin {
    /// Create a new audio classification plugin
    pub fn new(config: PluginConfig) -> Self {
        Self {
            config,
            classifier: None,
        }
    }

    /// Load plugin from YAML configuration
    pub fn from_yaml(yaml_path: impl AsRef<Path>) -> Result<Self, PluginError> {
        let contents = std::fs::read_to_string(yaml_path.as_ref())?;
        let config: PluginConfig = serde_yaml::from_str(&contents)
            .map_err(|e| PluginError::ExecutionFailed(format!("Failed to parse YAML: {}", e)))?;

        Ok(Self::new(config))
    }

    /// Initialize the classifier (lazy loading)
    fn ensure_classifier(&mut self, model_path: &Path) -> Result<(), PluginError> {
        if self.classifier.is_none() {
            let config = AudioClassificationConfig::default();
            let classifier = AudioClassifier::new(model_path, config)
                .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;
            self.classifier = Some(classifier);
        }
        Ok(())
    }
}

#[async_trait]
impl Plugin for AudioClassificationPlugin {
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
        let (confidence_threshold, top_k) = match &request.operation {
            Operation::AudioClassification {
                confidence_threshold,
                top_k,
            } => (*confidence_threshold, *top_k),
            _ => {
                return Err(PluginError::InvalidInput(
                    "Expected AudioClassification operation".to_string(),
                ))
            }
        };

        if ctx.verbose {
            info!(
                "Classifying audio: threshold={:.2}, top_k={}",
                confidence_threshold.unwrap_or(0.3),
                top_k.unwrap_or(5)
            );
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

        // Load audio file (16kHz mono WAV)
        let audio_data = Self::load_audio(audio_path)?;

        // Initialize classifier with model path
        let model_path = Path::new("models/audio-classification/yamnet.onnx");
        let mut plugin = Self {
            config: self.config.clone(),
            classifier: None,
        };
        plugin.ensure_classifier(model_path)?;

        // Configure classifier
        let mut classifier = plugin.classifier.unwrap();
        classifier.config.confidence_threshold = confidence_threshold.unwrap_or(0.3);
        classifier.config.top_k = top_k.unwrap_or(5);

        // Classify audio
        let classifications = classifier.classify(&audio_data).map_err(|e| match e {
            AudioClassificationError::InvalidAudioLength { expected, actual } => {
                PluginError::ExecutionFailed(format!(
                    "Audio too short: expected at least {} samples, got {}",
                    expected, actual
                ))
            }
            other => PluginError::ExecutionFailed(other.to_string()),
        })?;

        let duration = start.elapsed();

        if ctx.verbose {
            info!(
                "Audio classification complete in {:?}: {} segments",
                duration,
                classifications.len()
            );
        }

        // Serialize classifications to JSON
        let json = serde_json::to_value(&classifications).map_err(PluginError::Serialization)?;

        Ok(PluginResponse {
            output: PluginData::Json(json),
            duration,
            warnings: vec![],
        })
    }
}

impl AudioClassificationPlugin {
    /// Load audio from WAV file (16kHz mono, float32)
    fn load_audio(path: &Path) -> Result<Vec<f32>, PluginError> {
        // Read WAV file manually (simple PCM parser)
        let wav_data = std::fs::read(path)?;

        // Basic WAV header parsing (44 bytes)
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

        // Find data chunk (skip non-data chunks)
        let mut offset = 12;
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

            if chunk_id == b"data" {
                data_offset = offset + 8;
                data_size = chunk_size;
                break;
            }

            offset += 8 + chunk_size;
        }

        if data_offset == 0 {
            return Err(PluginError::ExecutionFailed(
                "Invalid WAV file: no data chunk found".to_string(),
            ));
        }

        // Convert PCM16 to float32 normalized to [-1, 1]
        let pcm_data = &wav_data[data_offset..data_offset + data_size];
        let num_samples = pcm_data.len() / 2; // 16-bit = 2 bytes per sample
        let mut audio = Vec::with_capacity(num_samples);

        for i in 0..num_samples {
            let sample_i16 = i16::from_le_bytes([pcm_data[i * 2], pcm_data[i * 2 + 1]]);
            let sample_f32 = (sample_i16 as f32) / 32768.0;
            audio.push(sample_f32);
        }

        Ok(audio)
    }
}

impl From<AudioClassificationError> for PluginError {
    fn from(err: AudioClassificationError) -> Self {
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
            name: "audio_classification".to_string(),
            description: "Test audio classification plugin".to_string(),
            inputs: vec!["wav".to_string()],
            outputs: vec!["AudioClassification".to_string()],
            config: RuntimeConfig {
                max_file_size_mb: 500,
                requires_gpu: false,
                experimental: false,
            },
            performance: PerformanceConfig {
                avg_processing_time_per_gb: "5s".to_string(),
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
        let plugin = AudioClassificationPlugin::new(config);

        assert_eq!(plugin.name(), "audio_classification");
        assert!(plugin.supports_input("wav"));
        assert!(plugin.produces_output("AudioClassification"));
    }
}
