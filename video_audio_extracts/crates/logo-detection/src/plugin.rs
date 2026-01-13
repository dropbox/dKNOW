//! Plugin wrapper for logo detection module (CLIP-based)

use crate::lib_clip::{ClipLogoConfig, ClipLogoDetector, LogoDatabase};
use async_trait::async_trait;
use once_cell::sync::OnceCell;
use ort::session::Session;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tracing::{debug, info};
use video_audio_embeddings::{VisionEmbeddingConfig, CLIPModel};
use video_extract_core::image_io::load_image;
use video_extract_core::plugin::PluginData;
use video_extract_core::{
    Context, Operation, Plugin, PluginConfig, PluginError, PluginRequest, PluginResponse,
};

/// Logo detection plugin implementation with CLIP model caching
pub struct LogoDetectionPlugin {
    config: PluginConfig,
    model_dir: PathBuf,
    /// Cached CLIP ONNX Session - loaded once per model and reused across all executions
    /// Wrapped in Mutex for interior mutability (Session::run requires &mut self)
    cached_clip_session: Arc<OnceCell<Mutex<Session>>>,
    /// Cached logo database (loaded once and reused)
    cached_logo_database: Arc<OnceCell<LogoDatabase>>,
    /// CLIP vision config
    cached_vision_config: Arc<OnceCell<VisionEmbeddingConfig>>,
}

impl LogoDetectionPlugin {
    /// Create a new logo detection plugin with CLIP model caching
    pub fn new(config: PluginConfig, model_dir: impl AsRef<Path>) -> Self {
        Self {
            config,
            model_dir: model_dir.as_ref().to_path_buf(),
            cached_clip_session: Arc::new(OnceCell::new()),
            cached_logo_database: Arc::new(OnceCell::new()),
            cached_vision_config: Arc::new(OnceCell::new()),
        }
    }

    /// Get or load the CLIP ONNX Session (cached after first load)
    fn get_or_load_clip_session(&self, model_path: &Path) -> Result<&Mutex<Session>, PluginError> {
        self.cached_clip_session.get_or_try_init(|| {
            info!(
                "Loading CLIP vision model from {} for logo detection (CPU-only to avoid CoreML region processing issues)",
                model_path.display()
            );

            // Use CPU-only execution provider for logo detection to avoid CoreML batch inference
            // issues with small extracted regions. See reports/main/N236_LOGO_DETECTION_COREML_ISSUE.md
            let session = video_extract_core::onnx_utils::create_cpu_only_session(model_path)
                .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;

            info!(
                "CLIP vision model loaded successfully (CPU-only) and cached for reuse"
            );
            Ok(Mutex::new(session))
        })
    }

    /// Get or load the logo database (cached after first load)
    fn get_or_load_logo_database(&self, database_path: &Path) -> Result<&LogoDatabase, PluginError> {
        self.cached_logo_database.get_or_try_init(|| {
            info!(
                "Loading logo database from {} (first time only)",
                database_path.display()
            );

            let database_json = std::fs::read_to_string(database_path).map_err(|e| {
                PluginError::ExecutionFailed(format!(
                    "Failed to read logo database file {:?}: {}",
                    database_path, e
                ))
            })?;

            let logo_database: LogoDatabase = serde_json::from_str(&database_json)
                .map_err(|e| PluginError::ExecutionFailed(format!("Failed to parse logo database JSON: {}", e)))?;

            if logo_database.logos.is_empty() {
                return Err(PluginError::ExecutionFailed(
                    "Logo database is empty".to_string(),
                ));
            }

            info!("Loaded {} logos from database (model: {}, dim: {})",
                logo_database.logos.len(),
                logo_database.model,
                logo_database.embedding_dim);
            Ok(logo_database)
        })
    }

    /// Get or create the CLIP vision config (cached after first creation)
    fn get_or_create_vision_config(&self, model_path: &Path) -> Result<&VisionEmbeddingConfig, PluginError> {
        self.cached_vision_config.get_or_try_init(|| {
            Ok(VisionEmbeddingConfig {
                model: CLIPModel::VitB32,
                model_path: model_path.to_string_lossy().to_string(),
                normalize: true,
                image_size: 224,
            })
        })
    }

    /// Load plugin from YAML configuration
    pub fn from_yaml(yaml_path: impl AsRef<Path>) -> Result<Self, PluginError> {
        let contents = std::fs::read_to_string(yaml_path.as_ref())?;
        let config: PluginConfig = serde_yaml::from_str(&contents)
            .map_err(|e| PluginError::ExecutionFailed(format!("Failed to parse YAML: {}", e)))?;

        // Default model directory
        let model_dir = PathBuf::from("models/logo-detection");

        Ok(Self::new(config, model_dir))
    }
}

#[async_trait]
impl Plugin for LogoDetectionPlugin {
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
        let (confidence_threshold, logo_classes) = match &request.operation {
            Operation::LogoDetection {
                confidence_threshold,
                logo_classes,
            } => (*confidence_threshold, logo_classes),
            _ => {
                return Err(PluginError::InvalidInput(
                    "Expected LogoDetection operation".to_string(),
                ))
            }
        };

        if ctx.verbose {
            info!(
                "Detecting logos with CLIP similarity threshold: {:.2}",
                confidence_threshold
            );
        }

        // Get CLIP model path (use embeddings model)
        let clip_model_path = PathBuf::from("models/embeddings/clip_vit_b32.onnx");

        // Get logo database path
        let database_path = self.model_dir.join("clip_database/logo_database.json");

        // Check if CLIP model and database exist
        if !clip_model_path.exists() {
            return Err(PluginError::ExecutionFailed(format!(
                "CLIP vision model not found at {:?}. Logo detection requires CLIP model for similarity search.",
                clip_model_path
            )));
        }

        if !database_path.exists() {
            return Err(PluginError::ExecutionFailed(format!(
                "Logo database not found at {:?}. Run tools/build_logo_database to create it.",
                database_path
            )));
        }

        // Get or load cached resources
        let clip_session_mutex = self.get_or_load_clip_session(&clip_model_path)?;
        let logo_database = self.get_or_load_logo_database(&database_path)?;
        let vision_config = self.get_or_create_vision_config(&clip_model_path)?;

        // Configure CLIP detector
        let clip_config = ClipLogoConfig {
            similarity_threshold: confidence_threshold,
            max_detections: 50,
            grid_size: 4,
            overlap_ratio: 0.25,
        };

        // Process input based on type
        let detections = match &request.input {
            PluginData::FilePath(path) => {
                debug!("Running CLIP logo detection on: {}", path.display());

                // Load image with optimized I/O (mozjpeg for JPEG, 3-5x faster)
                let img = load_image(path).map_err(|e| {
                    PluginError::ExecutionFailed(format!("Failed to load image: {}", e))
                })?;

                // Perform CLIP detection with cached session (lock mutex for duration of inference)
                let mut clip_session = clip_session_mutex.lock().map_err(|e| {
                    PluginError::ExecutionFailed(format!("Failed to lock CLIP session mutex: {}", e))
                })?;

                // Filter by logo_classes if specified
                let filtered_detections = ClipLogoDetector::detect_with_session(
                    &mut clip_session,
                    &img,
                    &clip_config,
                    vision_config,
                    logo_database,
                )
                .map_err(|e| {
                    PluginError::ExecutionFailed(format!("CLIP logo detection failed: {}", e))
                })?;

                // Filter by logo_classes if specified (brand name filtering)
                if let Some(ref classes) = logo_classes {
                    filtered_detections.into_iter()
                        .filter(|det| classes.iter().any(|c| c.eq_ignore_ascii_case(&det.brand)))
                        .collect()
                } else {
                    filtered_detections
                }
            }
            PluginData::Bytes(_) => {
                return Err(PluginError::UnsupportedFormat(
                    "Bytes input not yet supported, use file path or Keyframes JSON".to_string(),
                ));
            }
            PluginData::Json(keyframes_json) => {
                // Parse Keyframes JSON
                let keyframes: Vec<video_audio_common::Keyframe> =
                    serde_json::from_value(keyframes_json.clone()).map_err(|e| {
                        PluginError::InvalidInput(format!("Failed to parse Keyframes JSON: {}", e))
                    })?;

                if keyframes.is_empty() {
                    info!("No keyframes to process");
                    Vec::new()
                } else {
                    debug!("Running CLIP logo detection on {} keyframes", keyframes.len());

                    // Lock CLIP session once for all keyframes (better performance than lock/unlock per frame)
                    let mut clip_session = clip_session_mutex.lock().map_err(|e| {
                        PluginError::ExecutionFailed(format!("Failed to lock CLIP session mutex: {}", e))
                    })?;

                    // Pre-allocate detections Vec with estimated capacity
                    let mut all_detections = Vec::new();

                    // Process each keyframe
                    for (idx, keyframe) in keyframes.iter().enumerate() {
                        // Find the largest available thumbnail (prefer higher resolution)
                        let image_path =
                            keyframe.thumbnail_paths.values().next().ok_or_else(|| {
                                PluginError::InvalidInput(format!(
                                    "Keyframe {} has no thumbnail paths",
                                    idx
                                ))
                            })?;

                        debug!(
                            "Processing keyframe {} at t={:.2}s from {}",
                            idx,
                            keyframe.timestamp,
                            image_path.display()
                        );

                        // Load image
                        let img = load_image(image_path).map_err(|e| {
                            PluginError::ExecutionFailed(format!(
                                "Failed to load keyframe {} image: {}",
                                idx, e
                            ))
                        })?;

                        // Perform CLIP detection
                        let frame_detections = ClipLogoDetector::detect_with_session(
                            &mut clip_session,
                            &img,
                            &clip_config,
                            vision_config,
                            logo_database,
                        )
                        .map_err(|e| {
                            PluginError::ExecutionFailed(format!(
                                "CLIP logo detection failed on keyframe {}: {}",
                                idx, e
                            ))
                        })?;

                        // Filter by logo_classes if specified
                        let filtered = if let Some(ref classes) = logo_classes {
                            frame_detections.into_iter()
                                .filter(|det| classes.iter().any(|c| c.eq_ignore_ascii_case(&det.brand)))
                                .collect::<Vec<_>>()
                        } else {
                            frame_detections
                        };

                        // Collect detections from this keyframe
                        all_detections.extend(filtered);
                    }

                    debug!(
                        "CLIP logo detection complete: {} total detections across {} keyframes",
                        all_detections.len(),
                        keyframes.len()
                    );

                    all_detections
                }
            }
            _ => {
                return Err(PluginError::InvalidInput(
                    "Expected file path, bytes, or keyframes JSON".to_string(),
                ))
            }
        };

        let duration = start.elapsed();

        if ctx.verbose {
            info!(
                "Logo detection complete in {:?}: {} logos detected",
                duration,
                detections.len()
            );
        }

        // Serialize detections to JSON
        let json = serde_json::to_value(&detections).map_err(PluginError::Serialization)?;

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
    use std::time::SystemTime;
    use video_extract_core::plugin::{CacheConfig, PerformanceConfig, RuntimeConfig};

    fn create_test_config() -> PluginConfig {
        PluginConfig {
            name: "logo_detection".to_string(),
            description: "Test CLIP logo detection plugin".to_string(),
            inputs: vec![
                "jpg".to_string(),
                "png".to_string(),
                "Keyframes".to_string(),
            ],
            outputs: vec!["LogoDetection".to_string()],
            config: RuntimeConfig {
                max_file_size_mb: 100,
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

    #[test]
    fn test_plugin_creation() {
        let config = create_test_config();
        let plugin = LogoDetectionPlugin::new(config, "models/logo-detection");

        assert_eq!(plugin.name(), "logo_detection");
        assert!(plugin.supports_input("jpg"));
        assert!(plugin.supports_input("png"));
        assert!(plugin.supports_input("Keyframes"));
        assert!(plugin.produces_output("LogoDetection"));
    }
}
