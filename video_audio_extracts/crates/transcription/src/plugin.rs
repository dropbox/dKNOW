//! Plugin wrapper for transcription module

use crate::{Transcriber, TranscriptionConfig, TranscriptionError};
use async_trait::async_trait;
use once_cell::sync::OnceCell;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tracing::{debug, info};
use video_extract_core::operation::WhisperModel as CoreWhisperModel;
use video_extract_core::plugin::PluginData;
use video_extract_core::{
    Context, Operation, Plugin, PluginConfig, PluginError, PluginRequest, PluginResponse,
};
use whisper_rs::WhisperContext;

/// Transcription plugin implementation with model caching
///
/// # Thread Safety and Serialization
///
/// WhisperContext is not thread-safe. We protect it with a Mutex, which serializes
/// transcription operations across concurrent tasks. This means only ONE transcription
/// can run at a time, even with multiple files in bulk mode.
///
/// **Trade-offs:**
/// - ✅ PRO: Model loaded once (147MB) and shared across all files (eliminates per-file load overhead)
/// - ✅ PRO: Thread-safe, no deadlocks or hangs
/// - ✅ PRO: Memory efficient (single model instance)
/// - ⚠️  CON: Serialized transcription (no parallel inference across files)
///
/// **Architecture:** Arc<OnceCell<Mutex<WhisperContext>>>
/// - Arc: Shared ownership across concurrent tasks
/// - OnceCell: Model loaded exactly once on first use
/// - Mutex: Serializes access to WhisperContext (including create_state() and inference)
///
/// **Future optimization:** If whisper-rs supports thread-safe state creation, we could
/// reduce Mutex scope to only protect create_state(), allowing parallel inference with
/// independent WhisperState objects. Current implementation prioritizes correctness.
pub struct TranscriptionPlugin {
    config: PluginConfig,
    model_path: PathBuf,
    /// Cached WhisperContext - loaded once and protected by Mutex for thread-safe access
    cached_context: Arc<OnceCell<Mutex<WhisperContext>>>,
}

impl TranscriptionPlugin {
    /// Create a new transcription plugin
    pub fn new(config: PluginConfig, model_path: impl AsRef<Path>) -> Self {
        Self {
            config,
            model_path: model_path.as_ref().to_path_buf(),
            cached_context: Arc::new(OnceCell::new()),
        }
    }

    /// Get or load the WhisperContext (cached after first load)
    ///
    /// Returns a reference to the Mutex-protected WhisperContext. Callers must acquire
    /// the Mutex lock before calling WhisperContext methods to ensure thread safety.
    fn get_or_load_context(&self) -> Result<&Mutex<WhisperContext>, PluginError> {
        self.cached_context.get_or_try_init(|| {
            info!(
                "Loading Whisper model from {} (first time only)",
                self.model_path.display()
            );

            if !self.model_path.exists() {
                return Err(PluginError::ExecutionFailed(format!(
                    "Model file not found: {}",
                    self.model_path.display()
                )));
            }

            let ctx_params = whisper_rs::WhisperContextParameters::default();
            let context = WhisperContext::new_with_params(
                self.model_path.to_str().ok_or_else(|| {
                    PluginError::ExecutionFailed("Invalid path encoding".to_string())
                })?,
                ctx_params,
            )
            .map_err(|e| {
                PluginError::ExecutionFailed(format!("Failed to load Whisper model: {}", e))
            })?;

            info!("Whisper model loaded successfully and cached for reuse");
            Ok(Mutex::new(context))
        })
    }

    /// Load plugin from YAML configuration
    pub fn from_yaml(yaml_path: impl AsRef<Path>) -> Result<Self, PluginError> {
        let contents = std::fs::read_to_string(yaml_path.as_ref())?;
        let config: PluginConfig = serde_yaml::from_str(&contents)
            .map_err(|e| PluginError::ExecutionFailed(format!("Failed to parse YAML: {}", e)))?;

        // Default model path
        let model_path = PathBuf::from("models/whisper/ggml-large-v3.bin");

        Ok(Self::new(config, model_path))
    }

    /// Convert core Whisper model enum to crate-specific enum
    fn convert_model(model: &CoreWhisperModel) -> crate::WhisperModel {
        match model {
            CoreWhisperModel::Tiny => crate::WhisperModel::Tiny,
            CoreWhisperModel::Base => crate::WhisperModel::Base,
            CoreWhisperModel::Small => crate::WhisperModel::Small,
            CoreWhisperModel::Medium => crate::WhisperModel::Medium,
            CoreWhisperModel::Large => crate::WhisperModel::LargeV3,
        }
    }
}

#[async_trait]
impl Plugin for TranscriptionPlugin {
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
        let task_id = std::thread::current().id();

        tracing::debug!(
            "[TASK {:?}] Transcription plugin execute() started",
            task_id
        );

        // Extract operation parameters
        let (language, model) = match &request.operation {
            Operation::Transcription { language, model } => (language.clone(), model),
            _ => {
                return Err(PluginError::InvalidInput(
                    "Expected Transcription operation".to_string(),
                ))
            }
        };

        if ctx.verbose {
            info!(
                "Transcribing with model {:?}, language: {:?}",
                model, language
            );
        }

        // Get input file path
        let input_path = match &request.input {
            PluginData::FilePath(path) => path.clone(),
            PluginData::Bytes(_) => {
                // For bytes input, we'd need to write to a temp file
                // For now, return an error
                return Err(PluginError::UnsupportedFormat(
                    "Bytes input not yet supported, use file path".to_string(),
                ));
            }
            _ => {
                return Err(PluginError::InvalidInput(
                    "Expected file path or bytes".to_string(),
                ))
            }
        };

        debug!(
            "[TASK {:?}] Transcribing audio file: {}",
            task_id,
            input_path.display()
        );

        // Configure transcription
        let whisper_model = Self::convert_model(model);
        let mut transcription_config = TranscriptionConfig::balanced();
        transcription_config.model_size = whisper_model;
        if let Some(lang) = language {
            transcription_config.language = Some(lang);
        }

        // Get or load cached model context (this is the Mutex-protected context)
        tracing::debug!("[TASK {:?}] Getting or loading WhisperContext...", task_id);
        let _context_ref = self.get_or_load_context()?; // Ensure model is loaded
        tracing::debug!(
            "[TASK {:?}] WhisperContext cached reference acquired",
            task_id
        );

        // Clone the Arc<OnceCell<Mutex<WhisperContext>>> for the blocking task
        let cached_context = self.cached_context.clone();
        let input_path_clone = input_path.clone();
        let transcription_config_clone = transcription_config.clone();

        tracing::debug!(
            "[TASK {:?}] Spawning blocking task for transcription...",
            task_id
        );

        let transcript = tokio::task::spawn_blocking(move || {
            tracing::debug!("[BLOCKING TASK] Getting WhisperContext Mutex from OnceCell...");

            // Get the Mutex from the OnceCell (we know it's initialized because we called get_or_load_context above)
            let context_mutex = cached_context
                .get()
                .expect("WhisperContext should be initialized");

            tracing::debug!("[BLOCKING TASK] Acquiring WhisperContext Mutex lock...");

            // Acquire Mutex lock (blocks until available)
            let context = context_mutex.lock().map_err(|e| {
                TranscriptionError::ContextError(format!("Failed to lock WhisperContext: {}", e))
            })?;

            tracing::debug!(
                "[BLOCKING TASK] Mutex lock acquired, calling transcribe_with_context..."
            );

            // Perform transcription with the locked context
            let result = Transcriber::transcribe_with_context(
                &context,
                &input_path_clone,
                &transcription_config_clone,
            );

            tracing::debug!("[BLOCKING TASK] Transcription completed, releasing Mutex lock");

            // Mutex lock is automatically released when 'context' goes out of scope
            result
        })
        .await
        .map_err(|e| PluginError::ExecutionFailed(format!("Blocking task panicked: {}", e)))?
        .map_err(|e| PluginError::ExecutionFailed(format!("Transcription failed: {}", e)))?;

        tracing::debug!("[TASK {:?}] Transcription completed", task_id);

        let duration = start.elapsed();

        if ctx.verbose {
            info!(
                "Transcription complete in {:?}: {} segments, {:.1}s audio",
                duration,
                transcript.segments.len(),
                transcript.duration()
            );
        }

        // Serialize transcript to JSON
        let json = serde_json::to_value(&transcript).map_err(PluginError::Serialization)?;

        Ok(PluginResponse {
            output: PluginData::Json(json),
            duration,
            warnings: vec![],
        })
    }
}

impl From<TranscriptionError> for PluginError {
    fn from(err: TranscriptionError) -> Self {
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
            name: "transcription".to_string(),
            description: "Test transcription plugin".to_string(),
            inputs: vec!["wav".to_string(), "mp3".to_string()],
            outputs: vec!["Transcription".to_string()],
            config: RuntimeConfig {
                max_file_size_mb: 512,
                requires_gpu: false,
                experimental: false,
            },
            performance: PerformanceConfig {
                avg_processing_time_per_gb: "120s".to_string(),
                memory_per_file_mb: 1024,
                supports_streaming: true,
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
        let plugin = TranscriptionPlugin::new(config, "models/whisper/ggml-base.bin");

        assert_eq!(plugin.name(), "transcription");
        assert!(plugin.supports_input("wav"));
        assert!(plugin.supports_input("mp3"));
        assert!(plugin.produces_output("Transcription"));
        // Verify cached_context is initially empty
        assert!(plugin.cached_context.get().is_none());
    }

    #[test]
    fn test_model_conversion() {
        assert!(matches!(
            TranscriptionPlugin::convert_model(&CoreWhisperModel::Base),
            crate::WhisperModel::Base
        ));
        assert!(matches!(
            TranscriptionPlugin::convert_model(&CoreWhisperModel::Small),
            crate::WhisperModel::Small
        ));
    }
}
