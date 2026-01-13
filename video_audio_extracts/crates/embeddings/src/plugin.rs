//! Plugin wrapper for embeddings module

use crate::{
    AudioEmbeddingConfig, AudioEmbeddings, CLIPModel, TextEmbeddingConfig, TextEmbeddings,
    VisionEmbeddingConfig, VisionEmbeddings,
};
use async_trait::async_trait;
use once_cell::sync::OnceCell;
use ort::session::Session;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokenizers::Tokenizer;
use tracing::{debug, info};
use video_extract_core::image_io::load_image;
use video_extract_core::onnx_utils::create_cpu_only_session;
use video_extract_core::operation::{AudioModel, TextModel, VisionModel};
use video_extract_core::plugin::PluginData;
use video_extract_core::{
    Context, Operation, Plugin, PluginConfig, PluginError, PluginRequest, PluginResponse,
};

/// Embeddings plugin implementation (supports vision, text, and audio embeddings)
pub struct EmbeddingsPlugin {
    config: PluginConfig,
    model_dir: PathBuf,
    /// Cached ONNX sessions (one per model path)
    /// HashMap key is model path, value is lazy-loaded session wrapped in Arc for cloning
    cached_sessions: Arc<HashMap<String, Arc<OnceCell<Mutex<Session>>>>>,
    /// Cached tokenizer for text embeddings (shared across text models)
    cached_tokenizer: Arc<OnceCell<Tokenizer>>,
}

impl EmbeddingsPlugin {
    /// Create a new embeddings plugin
    pub fn new(config: PluginConfig, model_dir: impl AsRef<Path>) -> Self {
        let model_dir = model_dir.as_ref().to_path_buf();

        // Pre-allocate cache entries for all supported models (2 vision + 2 text + 1 audio = 5 models)
        let mut cache_map = HashMap::with_capacity(5);

        // Vision models
        cache_map.insert(
            model_dir
                .join("clip_vit_b32.onnx")
                .to_string_lossy()
                .to_string(),
            Arc::new(OnceCell::new()),
        );
        cache_map.insert(
            model_dir
                .join("clip_vit_l14.onnx")
                .to_string_lossy()
                .to_string(),
            Arc::new(OnceCell::new()),
        );

        // Text models
        cache_map.insert(
            model_dir
                .join("all_minilm_l6_v2.onnx")
                .to_string_lossy()
                .to_string(),
            Arc::new(OnceCell::new()),
        );
        cache_map.insert(
            model_dir
                .join("all_mpnet_base_v2.onnx")
                .to_string_lossy()
                .to_string(),
            Arc::new(OnceCell::new()),
        );

        // Audio models
        cache_map.insert(
            model_dir.join("clap.onnx").to_string_lossy().to_string(),
            Arc::new(OnceCell::new()),
        );

        Self {
            config,
            model_dir,
            cached_sessions: Arc::new(cache_map),
            cached_tokenizer: Arc::new(OnceCell::new()),
        }
    }

    /// Load plugin from YAML configuration
    pub fn from_yaml(yaml_path: impl AsRef<Path>) -> Result<Self, PluginError> {
        let contents = std::fs::read_to_string(yaml_path.as_ref())?;
        let config: PluginConfig = serde_yaml::from_str(&contents)
            .map_err(|e| PluginError::ExecutionFailed(format!("Failed to parse YAML: {}", e)))?;

        // Default model directory
        let model_dir = PathBuf::from("models/embeddings");

        Ok(Self::new(config, model_dir))
    }

    /// Convert core vision model enum to CLIP model enum
    fn convert_vision_model(model: &VisionModel) -> CLIPModel {
        match model {
            VisionModel::ClipVitB32 => CLIPModel::VitB32,
            VisionModel::ClipVitL14 => CLIPModel::VitL14,
        }
    }

    /// Get model path for vision model
    fn vision_model_path(&self, model: &VisionModel) -> PathBuf {
        let filename = match model {
            VisionModel::ClipVitB32 => "clip_vit_b32.onnx",
            VisionModel::ClipVitL14 => "clip_vit_l14.onnx",
        };
        self.model_dir.join(filename)
    }

    /// Get model path for text model
    fn text_model_path(&self, model: &TextModel) -> PathBuf {
        let filename = match model {
            TextModel::AllMiniLmL6V2 => "all_minilm_l6_v2.onnx",
            TextModel::AllMpnetBaseV2 => "all_mpnet_base_v2.onnx",
        };
        self.model_dir.join(filename)
    }

    /// Get model path for audio model
    fn audio_model_path(&self, model: &AudioModel) -> PathBuf {
        let filename = match model {
            AudioModel::ClapHtsatFused => "clap.onnx",
        };
        self.model_dir.join(filename)
    }

    /// Get or load audio ONNX session from cache (CPU-only, no CoreML)
    ///
    /// Audio models (CLAP) are incompatible with CoreML execution provider.
    /// Uses CPU-only execution to avoid CoreML inference errors. (N=45 fix)
    fn get_or_load_audio_session(
        &self,
        model_path: &Path,
    ) -> Result<Arc<OnceCell<Mutex<Session>>>, PluginError> {
        let model_path_str = model_path.to_string_lossy().to_string();

        let cell_arc = self.cached_sessions.get(&model_path_str).ok_or_else(|| {
            PluginError::ExecutionFailed(format!("Model path not in cache: {}", model_path_str))
        })?;

        // Eagerly initialize the session if not already loaded (CPU-only)
        cell_arc.get_or_try_init(|| {
            info!(
                "Loading ONNX model from {} with CPU-only execution (CoreML incompatible)",
                model_path_str
            );
            create_cpu_only_session(model_path)
                .map(Mutex::new)
                .map_err(|e| PluginError::ExecutionFailed(e.to_string()))
        })?;

        Ok(Arc::clone(cell_arc))
    }

    /// Get or load tokenizer from cache
    ///
    /// Loads the tokenizer on first call, returns cached tokenizer on subsequent calls.
    fn get_or_load_tokenizer(&self) -> Result<Arc<OnceCell<Tokenizer>>, PluginError> {
        // Load tokenizer if not already loaded
        self.cached_tokenizer.get_or_try_init(|| {
            let tokenizer_path = self.model_dir.join("tokenizer_minilm/tokenizer.json");
            info!(
                "Loading tokenizer from {} (first time only)",
                tokenizer_path.display()
            );
            Tokenizer::from_file(&tokenizer_path).map_err(|e| {
                PluginError::ExecutionFailed(format!("Failed to load tokenizer: {}", e))
            })
        })?;

        Ok(Arc::clone(&self.cached_tokenizer))
    }

    /// Execute vision embeddings
    async fn execute_vision_embeddings(
        &self,
        ctx: &Context,
        model: &VisionModel,
        input_path: &Path,
    ) -> Result<PluginResponse, PluginError> {
        if ctx.verbose {
            info!("Extracting vision embeddings with model {:?}", model);
        }

        let model_path = self.vision_model_path(model);
        let clip_model = Self::convert_vision_model(model);

        // Create configuration
        let config = VisionEmbeddingConfig {
            model: clip_model,
            model_path: model_path.to_string_lossy().to_string(),
            normalize: true,
            image_size: 224,
        };

        // Load image with optimized I/O (mozjpeg for JPEG, 3-5x faster)
        let rgb_img = load_image(input_path)
            .map_err(|e| PluginError::ExecutionFailed(format!("Failed to load image: {}", e)))?;
        let img = image::DynamicImage::ImageRgb8(rgb_img);

        // Get or load cached session (returns Arc which can be moved into async task)
        // CLIP model is incompatible with CoreML, use CPU-only execution (similar to CLAP/YOLOv8)
        let session_cell_arc = self.get_or_load_audio_session(&model_path)?;

        // Run inference in blocking task (CPU-bound)
        let embeddings = tokio::task::spawn_blocking(move || {
            // Get session from OnceCell (guaranteed to be initialized)
            let session_mutex = session_cell_arc.get().ok_or_else(|| {
                PluginError::ExecutionFailed("Session not initialized".to_string())
            })?;

            let mut session = session_mutex.lock().map_err(|e| {
                PluginError::ExecutionFailed(format!("Failed to lock session mutex: {}", e))
            })?;

            VisionEmbeddings::extract_embeddings_with_session(&mut session, &config, &[img])
                .map_err(|e| {
                    PluginError::ExecutionFailed(format!("Failed to extract embeddings: {}", e))
                })
        })
        .await
        .map_err(|e| PluginError::ExecutionFailed(format!("Task join error: {}", e)))??;

        // Serialize result as JSON
        let result_value = serde_json::to_value(&embeddings).map_err(|e| {
            PluginError::ExecutionFailed(format!("Failed to serialize result: {}", e))
        })?;

        debug!("Extracted {} vision embeddings", embeddings.len());

        Ok(PluginResponse {
            output: PluginData::Json(result_value),
            duration: std::time::Duration::default(),
            warnings: Vec::new(),
        })
    }

    /// Execute text embeddings from a text file
    async fn execute_text_embeddings_from_file(
        &self,
        ctx: &Context,
        model: &TextModel,
        input_path: &Path,
    ) -> Result<PluginResponse, PluginError> {
        if ctx.verbose {
            info!(
                "Extracting text embeddings from file with model {:?}",
                model
            );
        }

        // Read text from file
        let text = tokio::fs::read_to_string(input_path).await.map_err(|e| {
            PluginError::ExecutionFailed(format!("Failed to read text file: {}", e))
        })?;

        self.execute_text_embeddings_from_text(ctx, model, vec![text])
            .await
    }

    /// Execute text embeddings from Transcription JSON
    async fn execute_text_embeddings_from_transcription_json(
        &self,
        ctx: &Context,
        model: &TextModel,
        json: &serde_json::Value,
    ) -> Result<PluginResponse, PluginError> {
        if ctx.verbose {
            info!(
                "Extracting text embeddings from Transcription JSON with model {:?}",
                model
            );
        }

        // Extract text from transcription JSON (full text field)
        let text = json
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                PluginError::InvalidInput("Transcription JSON must have a 'text' field".to_string())
            })?
            .to_string();

        self.execute_text_embeddings_from_text(ctx, model, vec![text])
            .await
    }

    /// Execute text embeddings from text strings (core implementation)
    async fn execute_text_embeddings_from_text(
        &self,
        ctx: &Context,
        model: &TextModel,
        texts: Vec<String>,
    ) -> Result<PluginResponse, PluginError> {
        if ctx.verbose {
            info!(
                "Extracting text embeddings with model {:?} for {} text(s)",
                model,
                texts.len()
            );
        }

        let model_path = self.text_model_path(model);

        // Create configuration
        let config = TextEmbeddingConfig {
            model: match model {
                TextModel::AllMiniLmL6V2 => "all-MiniLM-L6-v2".to_string(),
                TextModel::AllMpnetBaseV2 => "all-mpnet-base-v2".to_string(),
            },
            model_path: model_path.to_string_lossy().to_string(),
            normalize: true,
            max_length: 256,
        };

        // Get or load cached session and tokenizer
        // Text embedding models (MiniLM) are incompatible with CoreML, use CPU-only execution (similar to CLIP/CLAP/YOLOv8)
        let session_cell_arc = self.get_or_load_audio_session(&model_path)?;
        let tokenizer_cell_arc = self.get_or_load_tokenizer()?;

        // Run inference in blocking task (CPU-bound)
        let embeddings = tokio::task::spawn_blocking(move || {
            // Get session from OnceCell (guaranteed to be initialized)
            let session_mutex = session_cell_arc.get().ok_or_else(|| {
                PluginError::ExecutionFailed("Session not initialized".to_string())
            })?;
            let mut session = session_mutex.lock().map_err(|e| {
                PluginError::ExecutionFailed(format!("Failed to lock session mutex: {}", e))
            })?;

            // Get tokenizer from OnceCell (guaranteed to be initialized)
            let tokenizer = tokenizer_cell_arc.get().ok_or_else(|| {
                PluginError::ExecutionFailed("Tokenizer not initialized".to_string())
            })?;

            TextEmbeddings::extract_embeddings_with_session(
                &mut session,
                tokenizer,
                &config,
                &texts,
            )
            .map_err(|e| {
                PluginError::ExecutionFailed(format!("Failed to extract embeddings: {}", e))
            })
        })
        .await
        .map_err(|e| PluginError::ExecutionFailed(format!("Task join error: {}", e)))??;

        // Serialize result as JSON
        let result_value = serde_json::to_value(&embeddings).map_err(|e| {
            PluginError::ExecutionFailed(format!("Failed to serialize result: {}", e))
        })?;

        debug!("Extracted {} text embeddings", embeddings.len());

        Ok(PluginResponse {
            output: PluginData::Json(result_value),
            duration: std::time::Duration::default(),
            warnings: Vec::new(),
        })
    }

    /// Execute audio embeddings
    async fn execute_audio_embeddings(
        &self,
        ctx: &Context,
        model: &AudioModel,
        input_path: &Path,
    ) -> Result<PluginResponse, PluginError> {
        if ctx.verbose {
            info!("Extracting audio embeddings with model {:?}", model);
        }

        let model_path = self.audio_model_path(model);

        // Create configuration
        let config = AudioEmbeddingConfig {
            model: match model {
                AudioModel::ClapHtsatFused => "laion/clap-htsat-fused".to_string(),
            },
            model_path: model_path.to_string_lossy().to_string(),
            normalize: true,
            sample_rate: 48000,
        };

        // Convert to WAV if necessary (supports mp3, flac, m4a)
        let wav_path: PathBuf;
        let temp_file: Option<tempfile::TempPath>;

        let extension = input_path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase());

        let audio_file = match extension.as_deref() {
            Some("wav") => {
                // Already WAV format
                temp_file = None;
                input_path.to_path_buf()
            }
            Some("mp3") | Some("flac") | Some("m4a") => {
                // Convert to WAV using audio-extractor
                if ctx.verbose {
                    info!("Converting {} to WAV format", input_path.display());
                }

                let temp = tempfile::NamedTempFile::new().map_err(|e| {
                    PluginError::ExecutionFailed(format!("Failed to create temp file: {}", e))
                })?;
                let temp_path = temp.path().to_path_buf();

                // Use audio-extractor to convert to PCM WAV (48kHz, mono)
                let audio_config = video_audio_extractor::AudioConfig {
                    sample_rate: 48000,
                    channels: 1,
                    format: video_audio_extractor::AudioFormat::PCM,
                    normalize: false,
                };

                wav_path =
                    video_audio_extractor::extract_audio(input_path, &temp_path, &audio_config)
                        .map_err(|e| {
                            PluginError::ExecutionFailed(format!(
                                "Failed to convert audio to WAV: {}",
                                e
                            ))
                        })?;

                temp_file = Some(temp.into_temp_path());
                wav_path
            }
            _ => {
                return Err(PluginError::InvalidInput(format!(
                    "Unsupported audio format: {:?}. Expected wav, mp3, flac, or m4a",
                    extension
                )));
            }
        };

        // Read audio file (WAV format)
        let reader = hound::WavReader::open(&audio_file)
            .map_err(|e| PluginError::ExecutionFailed(format!("Failed to open WAV file: {}", e)))?;

        let spec = reader.spec();
        let samples: Vec<f32> = if spec.sample_format == hound::SampleFormat::Float {
            // Float format (32-bit float)
            reader
                .into_samples::<f32>()
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| {
                    PluginError::ExecutionFailed(format!("Failed to read audio samples: {}", e))
                })?
        } else if spec.bits_per_sample <= 16 {
            // 8-bit or 16-bit integer format
            reader
                .into_samples::<i16>()
                .map(|s| s.map(|sample| sample as f32 / 32768.0))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| {
                    PluginError::ExecutionFailed(format!("Failed to read audio samples: {}", e))
                })?
        } else {
            // 24-bit or 32-bit integer format
            reader
                .into_samples::<i32>()
                .map(|s| s.map(|sample| sample as f32 / 2147483648.0))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| {
                    PluginError::ExecutionFailed(format!("Failed to read audio samples: {}", e))
                })?
        };

        // Clean up temp file if it exists
        drop(temp_file);

        // Get or load cached session (use CPU-only for audio models - CoreML incompatible with CLAP)
        let session_cell_arc = self.get_or_load_audio_session(&model_path)?;

        // Run inference in blocking task (CPU-bound)
        let embeddings = tokio::task::spawn_blocking(move || {
            // Get session from OnceCell (guaranteed to be initialized)
            let session_mutex = session_cell_arc.get().ok_or_else(|| {
                PluginError::ExecutionFailed("Session not initialized".to_string())
            })?;
            let mut session = session_mutex.lock().map_err(|e| {
                PluginError::ExecutionFailed(format!("Failed to lock session mutex: {}", e))
            })?;

            AudioEmbeddings::extract_embeddings_with_session(&mut session, &config, &[samples])
                .map_err(|e| {
                    PluginError::ExecutionFailed(format!("Failed to extract embeddings: {}", e))
                })
        })
        .await
        .map_err(|e| PluginError::ExecutionFailed(format!("Task join error: {}", e)))??;

        // Serialize result as JSON
        let result_value = serde_json::to_value(&embeddings).map_err(|e| {
            PluginError::ExecutionFailed(format!("Failed to serialize result: {}", e))
        })?;

        debug!("Extracted {} audio embeddings", embeddings.len());

        Ok(PluginResponse {
            output: PluginData::Json(result_value),
            duration: std::time::Duration::default(),
            warnings: Vec::new(),
        })
    }
}

#[async_trait]
impl Plugin for EmbeddingsPlugin {
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

        // Dispatch based on operation type
        let response = match &request.operation {
            Operation::VisionEmbeddings { model } => {
                // Handle Keyframes JSON input (same pattern as object-detection, face-detection, OCR)
                if let PluginData::Json(ref json_value) = &request.input {
                    #[derive(serde::Deserialize)]
                    struct Keyframe {
                        thumbnail_paths: HashMap<String, PathBuf>,
                    }

                    // Parse keyframes from JSON
                    let keyframes: Vec<Keyframe> = serde_json::from_value(json_value.clone())
                        .map_err(|e| {
                            PluginError::InvalidInput(format!(
                                "Failed to parse Keyframes JSON: {}",
                                e
                            ))
                        })?;

                    if keyframes.is_empty() {
                        return Err(PluginError::InvalidInput(
                            "Keyframes JSON is empty".to_string(),
                        ));
                    }

                    // Extract image paths from keyframes
                    let mut image_paths = Vec::with_capacity(keyframes.len());
                    for kf in &keyframes {
                        if let Some(path) = kf.thumbnail_paths.values().next() {
                            image_paths.push(path.clone());
                        }
                    }

                    if image_paths.is_empty() {
                        return Err(PluginError::InvalidInput(
                            "No thumbnail paths found in Keyframes JSON".to_string(),
                        ));
                    }

                    // Process each keyframe image and aggregate results
                    // Pre-allocate all_embeddings Vec with image_paths.len() capacity
                    let mut all_embeddings: Vec<Vec<f32>> = Vec::with_capacity(image_paths.len());
                    for (idx, img_path) in image_paths.iter().enumerate() {
                        let response = self.execute_vision_embeddings(ctx, model, img_path).await?;

                        if let PluginData::Json(json) = response.output {
                            // Extract embeddings array from JSON response
                            // Response is Vec<Vec<f32>> serialized as [[emb1], [emb2], ...]
                            let embeddings: Vec<Vec<f32>> =
                                serde_json::from_value(json).map_err(|e| {
                                    PluginError::ExecutionFailed(format!(
                                        "Keyframe {}: Failed to parse embeddings: {}",
                                        idx, e
                                    ))
                                })?;
                            all_embeddings.extend(embeddings);
                        } else {
                            return Err(PluginError::ExecutionFailed(format!(
                                "Keyframe {}: Expected JSON output from vision embeddings",
                                idx
                            )));
                        }
                    }

                    // Aggregate all embeddings into single response
                    PluginResponse {
                        output: PluginData::Json(serde_json::json!({
                            "embeddings": all_embeddings,
                            "count": all_embeddings.len(),
                        })),
                        duration: start.elapsed(),
                        warnings: Vec::new(),
                    }
                } else {
                    // Handle single file input
                    let input_path = match &request.input {
                        PluginData::FilePath(path) => path.clone(),
                        PluginData::Bytes(_) => {
                            return Err(PluginError::InvalidInput(
                                "Bytes input not yet supported for embeddings".to_string(),
                            ))
                        }
                        PluginData::Json(_) => {
                            return Err(PluginError::InvalidInput(
                                "JSON input only supported for Keyframes in VisionEmbeddings"
                                    .to_string(),
                            ))
                        }
                        PluginData::Multiple(_) => {
                            return Err(PluginError::InvalidInput(
                                "Multiple input not supported for embeddings".to_string(),
                            ))
                        }
                    };

                    self.execute_vision_embeddings(ctx, model, &input_path)
                        .await?
                }
            }
            Operation::TextEmbeddings { model } => {
                // Text embeddings - supports single file input or Transcription JSON input
                match &request.input {
                    PluginData::FilePath(path) => {
                        // Single text file input
                        self.execute_text_embeddings_from_file(ctx, model, path)
                            .await?
                    }
                    PluginData::Json(json) => {
                        // Transcription JSON input - extract text from segments
                        self.execute_text_embeddings_from_transcription_json(ctx, model, json)
                            .await?
                    }
                    _ => {
                        return Err(PluginError::InvalidInput(
                            "TextEmbeddings only supports FilePath or Transcription JSON input"
                                .to_string(),
                        ))
                    }
                }
            }
            Operation::AudioEmbeddings { model } => {
                // Audio embeddings - single file input only
                let input_path = match &request.input {
                    PluginData::FilePath(path) => path.clone(),
                    _ => {
                        return Err(PluginError::InvalidInput(
                            "AudioEmbeddings only supports FilePath input".to_string(),
                        ))
                    }
                };
                self.execute_audio_embeddings(ctx, model, &input_path)
                    .await?
            }
            _ => {
                return Err(PluginError::InvalidInput(format!(
                "Expected VisionEmbeddings, TextEmbeddings, or AudioEmbeddings operation, got: {}",
                request.operation.output_type_name()
            )))
            }
        };

        if ctx.verbose {
            info!("Embeddings extraction completed in {:?}", start.elapsed());
        }

        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;
    use video_extract_core::plugin::{CacheConfig, PerformanceConfig, RuntimeConfig};

    fn test_plugin_config() -> PluginConfig {
        PluginConfig {
            name: "embeddings".to_string(),
            description: "Semantic embeddings extraction (CLIP vision, Sentence-Transformers text, CLAP audio)".to_string(),
            inputs: vec![
                "image/jpeg".to_string(),
                "image/png".to_string(),
                "text/plain".to_string(),
                "audio/wav".to_string(),
            ],
            outputs: vec![
                "application/json".to_string(),
            ],
            config: RuntimeConfig {
                max_file_size_mb: 100,
                requires_gpu: false,
                experimental: false,
            },
            performance: PerformanceConfig {
                avg_processing_time_per_gb: "30s".to_string(),
                memory_per_file_mb: 500,
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
        let config = test_plugin_config();
        let plugin = EmbeddingsPlugin::new(config, "models/embeddings");
        assert_eq!(plugin.name(), "embeddings");
    }

    #[test]
    fn test_supports_input() {
        let config = test_plugin_config();
        let plugin = EmbeddingsPlugin::new(config, "models/embeddings");
        assert!(plugin.supports_input("image/jpeg"));
        assert!(plugin.supports_input("text/plain"));
        assert!(plugin.supports_input("audio/wav"));
        assert!(!plugin.supports_input("video/mp4"));
    }

    #[test]
    fn test_produces_output() {
        let config = test_plugin_config();
        let plugin = EmbeddingsPlugin::new(config, "models/embeddings");
        assert!(plugin.produces_output("application/json"));
        assert!(!plugin.produces_output("image/jpeg"));
    }

    #[test]
    fn test_model_path_conversion() {
        let config = test_plugin_config();
        let plugin = EmbeddingsPlugin::new(config, "models/embeddings");

        let path = plugin.vision_model_path(&VisionModel::ClipVitB32);
        assert!(path.to_string_lossy().contains("clip_vit_b32.onnx"));

        let path = plugin.text_model_path(&TextModel::AllMiniLmL6V2);
        assert!(path.to_string_lossy().contains("all_minilm_l6_v2.onnx"));

        let path = plugin.audio_model_path(&AudioModel::ClapHtsatFused);
        assert!(path.to_string_lossy().contains("clap.onnx"));
    }
}
