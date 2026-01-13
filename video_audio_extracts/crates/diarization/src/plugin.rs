//! Plugin wrapper for diarization module

use crate::{diarize_audio_with_session, DiarizationConfig};
use async_trait::async_trait;
use once_cell::sync::OnceCell;
use ort::session::Session;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tracing::{debug, info};
use video_extract_core::onnx_utils::create_optimized_session;
use video_extract_core::plugin::PluginData;
use video_extract_core::{
    Context, Operation, Plugin, PluginConfig, PluginError, PluginRequest, PluginResponse,
};

/// Diarization plugin implementation with ONNX model caching
pub struct DiarizationPlugin {
    config: PluginConfig,
    model_dir: PathBuf,
    /// Cached ONNX session (key = full model path)
    /// Diarization has 1 model: WeSpeaker speaker embedding model
    cached_sessions: Arc<HashMap<String, Arc<OnceCell<Mutex<Session>>>>>,
}

impl DiarizationPlugin {
    /// Create a new diarization plugin with model caching
    pub fn new(config: PluginConfig, model_dir: impl AsRef<Path>) -> Self {
        let model_dir = model_dir.as_ref().to_path_buf();

        // Pre-allocate cached session for WeSpeaker model (1 model)
        let mut sessions_map = HashMap::with_capacity(1);
        let model_path = model_dir.join("speaker_embedding.onnx");

        sessions_map.insert(
            model_path.to_string_lossy().to_string(),
            Arc::new(OnceCell::new()),
        );

        Self {
            config,
            model_dir,
            cached_sessions: Arc::new(sessions_map),
        }
    }

    /// Load plugin from YAML configuration
    pub fn from_yaml(yaml_path: impl AsRef<Path>) -> Result<Self, PluginError> {
        let contents = std::fs::read_to_string(yaml_path.as_ref())?;
        let config: PluginConfig = serde_yaml::from_str(&contents)
            .map_err(|e| PluginError::ExecutionFailed(format!("Failed to parse YAML: {}", e)))?;

        // Default model directory
        let model_dir = PathBuf::from("models/diarization");

        Ok(Self::new(config, model_dir))
    }

    /// Get or load ONNX session for the speaker embedding model
    fn get_or_load_session(
        &self,
        model_path: &Path,
    ) -> Result<Arc<OnceCell<Mutex<Session>>>, PluginError> {
        let model_path_str = model_path.to_string_lossy().to_string();

        let cell_arc = self.cached_sessions.get(&model_path_str).ok_or_else(|| {
            PluginError::ExecutionFailed(format!(
                "Model path not pre-allocated in cache: {}",
                model_path_str
            ))
        })?;

        // Initialize session if not already loaded
        cell_arc.get_or_try_init(|| {
            info!(
                "Loading ONNX model from: {} with optimizations",
                model_path_str
            );
            create_optimized_session(model_path)
                .map(Mutex::new)
                .map_err(|e| PluginError::ExecutionFailed(e.to_string()))
        })?;

        Ok(Arc::clone(cell_arc))
    }
}

#[async_trait]
impl Plugin for DiarizationPlugin {
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
        let num_speakers = match &request.operation {
            Operation::Diarization { num_speakers } => num_speakers,
            _ => {
                return Err(PluginError::InvalidInput(
                    "Expected Diarization operation".to_string(),
                ))
            }
        };

        if ctx.verbose {
            info!(
                "Performing speaker diarization (num_speakers: {:?})",
                num_speakers
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

        // Build diarization configuration
        let model_path = self.model_dir.join("speaker_embedding.onnx");

        // Get or load cached session
        let session_cell = self.get_or_load_session(&model_path)?;

        let diarization_config = DiarizationConfig {
            min_speakers: num_speakers.map(|n| n as u8),
            max_speakers: num_speakers.map(|n| n as u8),
            embedding_model_path: model_path.to_string_lossy().to_string(),
            vad_aggressiveness: 3,
            min_segment_duration: 0.3,
        };

        if ctx.verbose {
            debug!("Diarization config: {:?}", diarization_config);
        }

        // Perform diarization with cached session
        let diarization_result = tokio::task::spawn_blocking({
            let input_path = input_path.clone();
            move || {
                let session_mutex = session_cell
                    .get()
                    .ok_or_else(|| anyhow::anyhow!("Session not initialized"))?;
                let mut session = session_mutex
                    .lock()
                    .map_err(|e| anyhow::anyhow!("Failed to lock session: {}", e))?;

                diarize_audio_with_session(&input_path, &diarization_config, &mut session)
            }
        })
        .await
        .map_err(|e| PluginError::ExecutionFailed(format!("Task join error: {}", e)))?
        .map_err(|e| PluginError::ExecutionFailed(format!("Diarization failed: {}", e)))?;

        let duration = start.elapsed();
        if ctx.verbose {
            info!(
                "Diarization complete: {} speakers, {} segments in {:.2}s",
                diarization_result.speakers.len(),
                diarization_result.timeline.len(),
                duration.as_secs_f64()
            );
        }

        // Serialize result to JSON
        let json_value = serde_json::to_value(&diarization_result).map_err(|e| {
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
    use std::time::SystemTime;
    use video_extract_core::operation::Operation;
    use video_extract_core::plugin::{CacheConfig, PerformanceConfig, RuntimeConfig};
    use video_extract_core::{Context, ExecutionMode, PluginRequest};

    fn create_test_config() -> PluginConfig {
        PluginConfig {
            name: "diarization".to_string(),
            description: "Speaker diarization plugin".to_string(),
            inputs: vec!["Audio".to_string()],
            outputs: vec!["Diarization".to_string()],
            config: RuntimeConfig {
                max_file_size_mb: 10000,
                requires_gpu: false,
                experimental: false,
            },
            performance: PerformanceConfig {
                avg_processing_time_per_gb: "60s".to_string(),
                memory_per_file_mb: 512,
                supports_streaming: false,
            },
            cache: CacheConfig {
                enabled: true,
                version: 1,
                invalidate_before: SystemTime::UNIX_EPOCH,
            },
        }
    }

    fn create_test_plugin() -> DiarizationPlugin {
        DiarizationPlugin::new(create_test_config(), "models/diarization")
    }

    #[test]
    fn test_plugin_metadata() {
        let plugin = create_test_plugin();
        assert_eq!(plugin.name(), "diarization");
        assert!(plugin.supports_input("Audio"));
        assert!(plugin.produces_output("Diarization"));
        assert!(!plugin.supports_input("Video"));
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
            input: PluginData::FilePath(PathBuf::from("test.wav")),
        };

        let runtime = tokio::runtime::Runtime::new().unwrap();
        let result = runtime.block_on(plugin.execute(&ctx, &request));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Expected Diarization operation"));
    }

    #[test]
    fn test_unsupported_input_type() {
        let plugin = create_test_plugin();
        let ctx = Context::new(ExecutionMode::Debug);

        // Test bytes input
        let request = PluginRequest {
            operation: Operation::Diarization { num_speakers: None },
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
