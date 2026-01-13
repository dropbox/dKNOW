//! Plugin wrapper for acoustic scene classification module

use crate::{AcousticSceneClassifier, AcousticSceneConfig};
use async_trait::async_trait;
use std::path::Path;
use std::time::Instant;
use tracing::{debug, info};
use video_extract_core::plugin::PluginData;
use video_extract_core::{
    Context, Operation, Plugin, PluginConfig, PluginError, PluginRequest, PluginResponse,
};

/// Acoustic Scene Classification plugin implementation
pub struct AcousticSceneClassificationPlugin {
    config: PluginConfig,
}

impl AcousticSceneClassificationPlugin {
    /// Create a new acoustic scene classification plugin
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
impl Plugin for AcousticSceneClassificationPlugin {
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
        let confidence_threshold = match &request.operation {
            Operation::AcousticSceneClassification {
                confidence_threshold,
            } => confidence_threshold.unwrap_or(0.2), // Default: 0.2 (AcousticSceneConfig::default)
            _ => {
                return Err(PluginError::InvalidInput(
                    "Expected AcousticSceneClassification operation".to_string(),
                ))
            }
        };

        if ctx.verbose {
            info!(
                "Performing acoustic scene classification (confidence threshold: {:.2})",
                confidence_threshold
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
        let (samples, _sample_rate) = video_audio_decoder::c_ffi::load_audio_samples_f32(
            &input_path,
            16000, // 16kHz for acoustic scene classification
            1,     // Mono
        )
        .map_err(|e| PluginError::ExecutionFailed(format!("Failed to load audio: {}", e)))?;

        if ctx.verbose {
            debug!("Processing {} audio samples at 16kHz", samples.len());
        }

        // Build scene classification configuration
        let scene_config = AcousticSceneConfig {
            confidence_threshold,
            segment_duration: 3.0,
        };

        // Get model path from environment or use default
        let model_path = std::env::var("YAMNET_MODEL_PATH")
            .unwrap_or_else(|_| "models/audio-classification/yamnet.onnx".to_string());

        // Create classifier and run scene detection
        let mut classifier = AcousticSceneClassifier::new(&model_path, scene_config)
            .map_err(|e| PluginError::ExecutionFailed(format!("Classifier init failed: {}", e)))?;

        let scenes = classifier.classify_scenes(&samples).map_err(|e| {
            PluginError::ExecutionFailed(format!("Scene classification failed: {}", e))
        })?;

        let elapsed = start.elapsed();

        if ctx.verbose {
            info!(
                "Acoustic scene classification completed in {:.3}s: {} scenes detected",
                elapsed.as_secs_f64(),
                scenes.len()
            );
        }

        // Serialize result to JSON
        let json = serde_json::to_value(&scenes).map_err(|e| {
            PluginError::ExecutionFailed(format!("Failed to serialize result: {}", e))
        })?;

        Ok(PluginResponse {
            output: PluginData::Json(json),
            duration: elapsed,
            warnings: vec![],
        })
    }
}
