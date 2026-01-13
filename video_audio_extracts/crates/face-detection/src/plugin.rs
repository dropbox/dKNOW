//! Plugin wrapper for face detection module

use crate::{FaceDetectionConfig, FaceDetectionError, FaceDetector, RetinaFaceModel};
use async_trait::async_trait;
use once_cell::sync::OnceCell;
use ort::session::Session;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tracing::{debug, info};
use video_extract_core::image_io::load_image;
use video_extract_core::onnx_utils::create_optimized_session;
use video_extract_core::plugin::PluginData;
use video_extract_core::{
    Context, Operation, Plugin, PluginConfig, PluginError, PluginRequest, PluginResponse,
};

/// Face detection plugin implementation with model caching
pub struct FaceDetectionPlugin {
    config: PluginConfig,
    model_dir: PathBuf,
    /// Cached ONNX Session - loaded once per model and reused across all executions
    /// Wrapped in Mutex for interior mutability (Session::run requires &mut self)
    cached_sessions: Arc<OnceCell<Mutex<Session>>>,
    /// Cached model input dimensions (width, height)
    input_size: (u32, u32),
}

impl FaceDetectionPlugin {
    /// Create a new face detection plugin with model caching
    pub fn new(config: PluginConfig, model_dir: impl AsRef<Path>) -> Self {
        Self {
            config,
            model_dir: model_dir.as_ref().to_path_buf(),
            cached_sessions: Arc::new(OnceCell::new()),
            input_size: (320, 240), // RetinaFace MobileNet025 input size
        }
    }

    /// Get or load the ONNX Session (cached after first load)
    fn get_or_load_session(&self, model_path: &Path) -> Result<&Mutex<Session>, PluginError> {
        self.cached_sessions.get_or_try_init(|| {
            info!(
                "Loading RetinaFace model from {} with optimizations (first time only)",
                model_path.display()
            );

            let session = create_optimized_session(model_path)
                .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;

            info!("RetinaFace model loaded successfully with graph optimizations and cached for reuse");
            Ok(Mutex::new(session))
        })
    }

    /// Load plugin from YAML configuration
    pub fn from_yaml(yaml_path: impl AsRef<Path>) -> Result<Self, PluginError> {
        let contents = std::fs::read_to_string(yaml_path.as_ref())?;
        let config: PluginConfig = serde_yaml::from_str(&contents)
            .map_err(|e| PluginError::ExecutionFailed(format!("Failed to parse YAML: {}", e)))?;

        // Default model directory (matches actual models/ directory structure)
        let model_dir = PathBuf::from("models/face-detection");

        Ok(Self::new(config, model_dir))
    }
}

#[async_trait]
impl Plugin for FaceDetectionPlugin {
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
        let (min_size, include_landmarks) = match &request.operation {
            Operation::FaceDetection {
                min_size,
                include_landmarks,
            } => (*min_size, *include_landmarks),
            _ => {
                return Err(PluginError::InvalidInput(
                    "Expected FaceDetection operation".to_string(),
                ))
            }
        };

        if ctx.verbose {
            info!(
                "Detecting faces with min_size={}, landmarks={}",
                min_size, include_landmarks
            );
        }

        // Configure detector (using MobileNet025 as default - fastest model)
        let model = RetinaFaceModel::MobileNet025;
        let model_path = self.model_dir.join(model.filename());

        let mut detector_config = FaceDetectionConfig::default();
        detector_config.detect_landmarks = include_landmarks;

        // Get or load cached ONNX session
        let session_mutex = self.get_or_load_session(&model_path)?;

        // Process input based on type
        let faces = match &request.input {
            PluginData::FilePath(path) => {
                debug!("Running face detection on: {}", path.display());

                // Load image with optimized I/O (mozjpeg for JPEG, 3-5x faster)
                let img = load_image(path).map_err(|e| {
                    PluginError::ExecutionFailed(format!("Failed to load image: {}", e))
                })?;

                // Perform detection with cached session (lock mutex for duration of inference)
                let mut session = session_mutex.lock().map_err(|e| {
                    PluginError::ExecutionFailed(format!("Failed to lock session mutex: {}", e))
                })?;
                let (input_width, input_height) = self.input_size;
                FaceDetector::detect_with_session(
                    &mut session,
                    &img,
                    &detector_config,
                    input_width,
                    input_height,
                )
                .map_err(|e| {
                    PluginError::ExecutionFailed(format!("Face detection failed: {}", e))
                })?
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
                    debug!("Running face detection on {} keyframes", keyframes.len());

                    // Lock session once for all keyframes (better performance than lock/unlock per frame)
                    let mut session = session_mutex.lock().map_err(|e| {
                        PluginError::ExecutionFailed(format!("Failed to lock session mutex: {}", e))
                    })?;

                    // Pre-allocate faces Vec with keyframes.len() capacity
                    // Each keyframe produces one FaceDetectionResult
                    let mut all_faces = Vec::with_capacity(keyframes.len());

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

                        // Perform detection
                        let (input_width, input_height) = self.input_size;
                        let frame_faces = FaceDetector::detect_with_session(
                            &mut session,
                            &img,
                            &detector_config,
                            input_width,
                            input_height,
                        )
                        .map_err(|e| {
                            PluginError::ExecutionFailed(format!(
                                "Face detection failed on keyframe {}: {}",
                                idx, e
                            ))
                        })?;

                        debug!("Keyframe {}: {} faces detected", idx, frame_faces.len());

                        // Add all faces from this keyframe to aggregated results
                        all_faces.extend(frame_faces);
                    }

                    all_faces
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
                "Face detection complete in {:?}: {} faces detected",
                duration,
                faces.len()
            );
        }

        // Serialize faces to JSON
        let json = serde_json::to_value(&faces).map_err(PluginError::Serialization)?;

        Ok(PluginResponse {
            output: PluginData::Json(json),
            duration,
            warnings: vec![],
        })
    }
}

impl From<FaceDetectionError> for PluginError {
    fn from(err: FaceDetectionError) -> Self {
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
            name: "face_detection".to_string(),
            description: "Test face detection plugin".to_string(),
            inputs: vec![
                "jpg".to_string(),
                "png".to_string(),
                "Keyframes".to_string(),
            ],
            outputs: vec!["FaceDetection".to_string()],
            config: RuntimeConfig {
                max_file_size_mb: 100,
                requires_gpu: false,
                experimental: false,
            },
            performance: PerformanceConfig {
                avg_processing_time_per_gb: "45s".to_string(),
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
        let plugin = FaceDetectionPlugin::new(config, "models/retinaface");

        assert_eq!(plugin.name(), "face_detection");
        assert!(plugin.supports_input("jpg"));
        assert!(plugin.supports_input("png"));
        assert!(plugin.supports_input("Keyframes"));
        assert!(plugin.produces_output("FaceDetection"));
    }
}
