//! Plugin wrapper for voice activity detection module

use crate::{VadConfig, VoiceActivityDetector};
use async_trait::async_trait;
use std::path::Path;
use std::time::Instant;
use tracing::{debug, info};
use video_extract_core::plugin::PluginData;
use video_extract_core::{
    Context, Operation, Plugin, PluginConfig, PluginError, PluginRequest, PluginResponse,
};

/// Voice Activity Detection plugin implementation
pub struct VoiceActivityDetectionPlugin {
    config: PluginConfig,
}

impl VoiceActivityDetectionPlugin {
    /// Create a new VAD plugin
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

    /// Convert VadAggressiveness enum to u8
    fn aggressiveness_to_u8(
        aggressiveness: &video_extract_core::operation::VadAggressiveness,
    ) -> u8 {
        match aggressiveness {
            video_extract_core::operation::VadAggressiveness::Quality => 0,
            video_extract_core::operation::VadAggressiveness::LowBitrate => 1,
            video_extract_core::operation::VadAggressiveness::Aggressive => 2,
            video_extract_core::operation::VadAggressiveness::VeryAggressive => 3,
        }
    }
}

#[async_trait]
impl Plugin for VoiceActivityDetectionPlugin {
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
        let (aggressiveness, min_segment_duration) = match &request.operation {
            Operation::VoiceActivityDetection {
                aggressiveness,
                min_segment_duration,
            } => (aggressiveness, min_segment_duration),
            _ => {
                return Err(PluginError::InvalidInput(
                    "Expected VoiceActivityDetection operation".to_string(),
                ))
            }
        };

        if ctx.verbose {
            info!(
                "Performing voice activity detection (aggressiveness: {:?}, min_segment_duration: {}s)",
                aggressiveness, min_segment_duration
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
            debug!("Input audio file: {:?}", input_path);
        }

        // Load audio samples from file using C FFI (no process spawn)
        let (samples, sample_rate) = video_audio_decoder::c_ffi::load_audio_samples_f32(
            &input_path,
            16000, // 16kHz for VAD
            1,     // Mono
        )
        .map_err(|e| PluginError::ExecutionFailed(format!("Failed to load audio: {}", e)))?;

        if ctx.verbose {
            debug!(
                "Processing {} audio samples at {}Hz",
                samples.len(),
                sample_rate
            );
        }

        // Build VAD configuration
        let vad_config = VadConfig {
            vad_aggressiveness: Self::aggressiveness_to_u8(aggressiveness),
            min_segment_duration: *min_segment_duration,
            frame_duration_ms: 30, // Standard 30ms frames
        };

        // Create VAD detector and run detection
        let detector = VoiceActivityDetector::new(vad_config);
        let result = detector
            .detect(&samples, sample_rate)
            .map_err(|e| PluginError::ExecutionFailed(format!("VAD failed: {}", e)))?;

        let elapsed = start.elapsed();

        if ctx.verbose {
            info!(
                "VAD completed in {:.3}s: {} segments, {:.1}% voice",
                elapsed.as_secs_f64(),
                result.segments.len(),
                result.voice_percentage * 100.0
            );
        }

        // Serialize result to JSON
        let json = serde_json::to_value(&result).map_err(|e| {
            PluginError::ExecutionFailed(format!("Failed to serialize result: {}", e))
        })?;

        Ok(PluginResponse {
            output: PluginData::Json(json),
            duration: elapsed,
            warnings: vec![],
        })
    }
}
