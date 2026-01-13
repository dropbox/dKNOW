//! Plugin wrapper for music source separation module

use crate::{MusicSourceSeparator, SeparatedStem, SourceSeparationConfig, SourceSeparationError};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Instant;
use tracing::info;
use video_extract_core::plugin::PluginData;
use video_extract_core::{
    Context, Operation, Plugin, PluginConfig, PluginError, PluginRequest, PluginResponse,
};

/// Music source separation plugin implementation
pub struct MusicSourceSeparationPlugin {
    config: PluginConfig,
    separator: Option<MusicSourceSeparator>,
}

impl MusicSourceSeparationPlugin {
    /// Create a new music source separation plugin
    pub fn new(config: PluginConfig) -> Self {
        Self {
            config,
            separator: None,
        }
    }

    /// Load plugin from YAML configuration
    pub fn from_yaml(yaml_path: impl AsRef<Path>) -> Result<Self, PluginError> {
        let contents = std::fs::read_to_string(yaml_path.as_ref())?;
        let config: PluginConfig = serde_yaml::from_str(&contents)
            .map_err(|e| PluginError::ExecutionFailed(format!("Failed to parse YAML: {}", e)))?;

        Ok(Self::new(config))
    }

    /// Initialize the separator (lazy loading)
    fn ensure_separator(
        &mut self,
        model_path: &Path,
        stem_names_path: &Path,
    ) -> Result<(), PluginError> {
        if self.separator.is_none() {
            let config = SourceSeparationConfig::default();
            let separator = MusicSourceSeparator::new(model_path, stem_names_path, config)
                .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;
            self.separator = Some(separator);
        }
        Ok(())
    }

    /// Load audio file and convert to stereo 44.1kHz float32
    ///
    /// This function:
    /// 1. Uses audio-extractor to convert input (MP3, M4A, etc.) to WAV
    /// 2. Reads WAV file using hound
    /// 3. Resamples to 44.1kHz if needed (simple linear interpolation)
    /// 4. Converts to stereo if mono
    /// 5. Returns interleaved f32 samples
    fn load_audio(path: &Path) -> Result<Vec<f32>, PluginError> {
        use video_audio_extractor::{extract_audio, AudioConfig, AudioFormat};

        // Step 1: Extract audio to temporary WAV file (handles MP3, M4A, FLAC, etc.)
        let temp_wav = std::env::temp_dir().join(format!(
            "music_sep_{}_{}.wav",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));

        // Configure for 44.1kHz stereo PCM (Demucs requirement)
        let config = AudioConfig {
            sample_rate: 44100,
            channels: 2,
            format: AudioFormat::PCM,
            normalize: false,
        };

        let wav_path = extract_audio(path, &temp_wav, &config)
            .map_err(|e| PluginError::ExecutionFailed(format!("Audio extraction failed: {}", e)))?;

        // Step 2: Read WAV file using hound
        let mut reader = hound::WavReader::open(&wav_path)
            .map_err(|e| PluginError::ExecutionFailed(format!("Failed to open WAV: {}", e)))?;

        let spec = reader.spec();
        let sample_rate = spec.sample_rate;
        let channels = spec.channels;

        // Step 3: Read all samples and convert to f32
        let samples: Result<Vec<f32>, _> = match spec.sample_format {
            hound::SampleFormat::Float => {
                reader.samples::<f32>().collect()
            }
            hound::SampleFormat::Int => {
                // Convert i16 to f32 (normalized to [-1.0, 1.0])
                reader.samples::<i16>()
                    .map(|s| s.map(|v| v as f32 / 32768.0))
                    .collect()
            }
        };

        let mut samples = samples
            .map_err(|e| PluginError::ExecutionFailed(format!("Failed to read samples: {}", e)))?;

        // Step 4: Convert mono to stereo if needed (duplicate channel)
        if channels == 1 {
            let mut stereo_samples = Vec::with_capacity(samples.len() * 2);
            for sample in samples {
                stereo_samples.push(sample); // Left
                stereo_samples.push(sample); // Right (duplicate)
            }
            samples = stereo_samples;
        } else if channels != 2 {
            return Err(PluginError::ExecutionFailed(format!(
                "Unsupported channel count: {} (expected 1 or 2)",
                channels
            )));
        }

        // Step 5: Resample to 44.1kHz if needed (simple linear interpolation)
        if sample_rate != 44100 {
            let ratio = 44100.0 / sample_rate as f32;
            let new_len = (samples.len() as f32 * ratio) as usize;
            let mut resampled = Vec::with_capacity(new_len);

            for i in 0..new_len {
                let src_pos = i as f32 / ratio;
                let src_idx = src_pos as usize;
                let frac = src_pos - src_idx as f32;

                if src_idx + 1 < samples.len() {
                    // Linear interpolation
                    let sample = samples[src_idx] * (1.0 - frac) + samples[src_idx + 1] * frac;
                    resampled.push(sample);
                } else if src_idx < samples.len() {
                    resampled.push(samples[src_idx]);
                }
            }
            samples = resampled;
        }

        // Clean up temporary file
        let _ = std::fs::remove_file(wav_path);

        Ok(samples)
    }
}

#[async_trait]
impl Plugin for MusicSourceSeparationPlugin {
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
        let stems_filter = match &request.operation {
            Operation::MusicSourceSeparation { stems } => stems.clone(),
            _ => {
                return Err(PluginError::InvalidInput(
                    "Expected MusicSourceSeparation operation".to_string(),
                ))
            }
        };

        if ctx.verbose {
            if let Some(ref stems) = stems_filter {
                info!("Separating music: stems={:?}", stems);
            } else {
                info!("Separating music: all stems");
            }
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

        // Load audio file
        let audio_data = Self::load_audio(audio_path)?;

        // Initialize separator with model path
        let model_path = Path::new("models/music-source-separation/demucs.onnx");
        let stem_names_path = Path::new("models/music-source-separation/stems.txt");

        // Create separator configuration
        let mut config = SourceSeparationConfig::default();
        if let Some(stems) = stems_filter {
            config.stems_filter = stems;
        }

        // Create separator
        let mut separator = MusicSourceSeparator::new(model_path, stem_names_path, config)
            .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;

        // Separate audio into stems
        let separated_stems = separator.separate(&audio_data).map_err(|e| match e {
            SourceSeparationError::InvalidAudioLength { min, actual } => {
                PluginError::ExecutionFailed(format!(
                    "Audio too short: expected at least {} samples, got {}",
                    min, actual
                ))
            }
            other => PluginError::ExecutionFailed(other.to_string()),
        })?;

        let duration = start.elapsed();

        if ctx.verbose {
            info!(
                "Music source separation complete in {:?}: {} stems",
                duration,
                separated_stems.len()
            );
        }

        // Serialize separated stems to JSON
        let json = serde_json::to_value(&separated_stems).map_err(PluginError::Serialization)?;

        Ok(PluginResponse {
            output: PluginData::Json(json),
            duration,
            warnings: vec![],
        })
    }
}

/// Result structure for serialization
#[derive(Debug, Serialize, Deserialize)]
pub struct SeparationResult {
    pub stems: Vec<SeparatedStem>,
}
