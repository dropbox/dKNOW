//! Plugin wrapper for pose estimation module

use crate::{PoseEstimationConfig, PoseEstimationError, PoseEstimator, YOLOPoseModel};
use async_trait::async_trait;
use once_cell::sync::OnceCell;
use ort::session::Session;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tracing::{debug, info};
use video_extract_core::image_io::load_image;
use video_extract_core::onnx_utils::create_optimized_session;
use video_extract_core::operation::PoseEstimationModel as CorePoseEstimationModel;
use video_extract_core::plugin::PluginData;
use video_extract_core::{
    Context, Operation, Plugin, PluginConfig, PluginError, PluginRequest, PluginResponse,
};

/// Pose estimation plugin implementation with model caching
pub struct PoseEstimationPlugin {
    config: PluginConfig,
    model_dir: PathBuf,
    /// Cached ONNX Session - loaded once per model and reused across all executions
    /// Wrapped in Mutex for interior mutability (Session::run requires &mut self)
    cached_sessions: Arc<OnceCell<Mutex<Session>>>,
}

impl PoseEstimationPlugin {
    /// Create a new pose estimation plugin with model caching
    pub fn new(config: PluginConfig, model_dir: impl AsRef<Path>) -> Self {
        Self {
            config,
            model_dir: model_dir.as_ref().to_path_buf(),
            cached_sessions: Arc::new(OnceCell::new()),
        }
    }

    /// Get or load the ONNX Session (cached after first load)
    fn get_or_load_session(&self, model_path: &Path) -> Result<&Mutex<Session>, PluginError> {
        self.cached_sessions.get_or_try_init(|| {
            info!(
                "Loading YOLOv8-Pose model from {} with optimizations (first time only)",
                model_path.display()
            );

            let session = create_optimized_session(model_path)
                .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;

            info!("YOLOv8-Pose model loaded successfully with graph optimizations and cached for reuse");
            Ok(Mutex::new(session))
        })
    }

    /// Load plugin from YAML configuration
    pub fn from_yaml(yaml_path: impl AsRef<Path>) -> Result<Self, PluginError> {
        let contents = std::fs::read_to_string(yaml_path.as_ref())?;
        let config: PluginConfig = serde_yaml::from_str(&contents)
            .map_err(|e| PluginError::ExecutionFailed(format!("Failed to parse YAML: {}", e)))?;

        // Default model directory
        let model_dir = PathBuf::from("models/pose-estimation");

        Ok(Self::new(config, model_dir))
    }

    /// Convert core PoseEstimationModel enum to crate-specific enum
    fn convert_model(model: &CorePoseEstimationModel) -> YOLOPoseModel {
        match model {
            CorePoseEstimationModel::YoloV8nPose => YOLOPoseModel::Nano,
            CorePoseEstimationModel::YoloV8nPoseInt8 => YOLOPoseModel::NanoInt8,
            CorePoseEstimationModel::YoloV8sPose => YOLOPoseModel::Small,
            CorePoseEstimationModel::YoloV8mPose => YOLOPoseModel::Medium,
            CorePoseEstimationModel::YoloV8lPose => YOLOPoseModel::Large,
            CorePoseEstimationModel::YoloV8xPose => YOLOPoseModel::XLarge,
        }
    }
}

#[async_trait]
impl Plugin for PoseEstimationPlugin {
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
        let (model, confidence_threshold, keypoint_threshold) = match &request.operation {
            Operation::PoseEstimation {
                model,
                confidence_threshold,
                keypoint_threshold,
            } => (model, *confidence_threshold, *keypoint_threshold),
            _ => {
                return Err(PluginError::InvalidInput(
                    "Expected PoseEstimation operation".to_string(),
                ))
            }
        };

        if ctx.verbose {
            info!(
                "Estimating poses with model {:?}, confidence threshold: {:.2}, keypoint threshold: {:.2}",
                model, confidence_threshold, keypoint_threshold
            );
        }

        // Configure estimator
        let yolo_model = Self::convert_model(model);
        let model_path = self.model_dir.join(yolo_model.filename());

        let estimator_config = PoseEstimationConfig {
            confidence_threshold,
            keypoint_threshold,
            ..Default::default()
        };

        // Get or load cached ONNX session
        let session_mutex = self.get_or_load_session(&model_path)?;

        // Process input based on type
        let detections = match &request.input {
            PluginData::FilePath(path) => {
                debug!("Running pose estimation on: {}", path.display());

                // Load image with optimized I/O (mozjpeg for JPEG, 3-5x faster)
                let img = load_image(path).map_err(|e| {
                    PluginError::ExecutionFailed(format!("Failed to load image: {}", e))
                })?;

                // Perform pose estimation with cached session (lock mutex for duration of inference)
                let mut session = session_mutex.lock().map_err(|e| {
                    PluginError::ExecutionFailed(format!("Failed to lock session mutex: {}", e))
                })?;
                PoseEstimator::estimate_with_session(&mut session, &img, &estimator_config)
                    .map_err(|e| {
                        PluginError::ExecutionFailed(format!("Pose estimation failed: {}", e))
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
                    debug!("Running pose estimation on {} keyframes", keyframes.len());

                    // Lock session once for all keyframes (better performance than lock/unlock per frame)
                    let mut session = session_mutex.lock().map_err(|e| {
                        PluginError::ExecutionFailed(format!("Failed to lock session mutex: {}", e))
                    })?;

                    // Pre-allocate detections Vec with keyframes.len() capacity
                    // Each keyframe produces one PoseEstimationResult
                    let mut all_detections = Vec::with_capacity(keyframes.len());

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

                        // Perform pose estimation
                        let frame_detections = PoseEstimator::estimate_with_session(
                            &mut session,
                            &img,
                            &estimator_config,
                        )
                        .map_err(|e| {
                            PluginError::ExecutionFailed(format!(
                                "Pose estimation failed on keyframe {}: {}",
                                idx, e
                            ))
                        })?;

                        // Collect detections from this keyframe
                        all_detections.extend(frame_detections);
                    }

                    debug!(
                        "Pose estimation complete: {} total detections across {} keyframes",
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
                "Pose estimation complete in {:?}: {} people detected",
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

impl From<PoseEstimationError> for PluginError {
    fn from(err: PoseEstimationError) -> Self {
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
            name: "pose_estimation".to_string(),
            description: "Test pose estimation plugin".to_string(),
            inputs: vec![
                "jpg".to_string(),
                "png".to_string(),
                "Keyframes".to_string(),
            ],
            outputs: vec!["PoseEstimation".to_string()],
            config: RuntimeConfig {
                max_file_size_mb: 100,
                requires_gpu: false,
                experimental: false,
            },
            performance: PerformanceConfig {
                avg_processing_time_per_gb: "30s".to_string(),
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
        let plugin = PoseEstimationPlugin::new(config, "models/pose-estimation");

        assert_eq!(plugin.name(), "pose_estimation");
        assert!(plugin.supports_input("jpg"));
        assert!(plugin.supports_input("png"));
        assert!(plugin.supports_input("Keyframes"));
        assert!(plugin.produces_output("PoseEstimation"));
    }

    #[test]
    fn test_model_conversion() {
        assert!(matches!(
            PoseEstimationPlugin::convert_model(&CorePoseEstimationModel::YoloV8nPose),
            YOLOPoseModel::Nano
        ));
        assert!(matches!(
            PoseEstimationPlugin::convert_model(&CorePoseEstimationModel::YoloV8sPose),
            YOLOPoseModel::Small
        ));
        assert!(matches!(
            PoseEstimationPlugin::convert_model(&CorePoseEstimationModel::YoloV8mPose),
            YOLOPoseModel::Medium
        ));
    }
}
