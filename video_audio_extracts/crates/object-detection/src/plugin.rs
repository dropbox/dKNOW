//! Plugin wrapper for object detection module

use crate::{ObjectDetectionConfig, ObjectDetectionError, ObjectDetector, YOLOModel};
use async_trait::async_trait;
use once_cell::sync::OnceCell;
use ort::session::Session;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tracing::{debug, info};
use video_extract_core::image_io::load_image;
use video_extract_core::onnx_utils::create_cpu_only_session;
use video_extract_core::operation::ObjectDetectionModel as CoreObjectDetectionModel;
use video_extract_core::plugin::PluginData;
use video_extract_core::{
    Context, Operation, Plugin, PluginConfig, PluginError, PluginRequest, PluginResponse,
};

/// Object detection plugin implementation with model caching
pub struct ObjectDetectionPlugin {
    config: PluginConfig,
    model_dir: PathBuf,
    /// Cached ONNX Session - loaded once per model and reused across all executions
    /// Wrapped in Mutex for interior mutability (Session::run requires &mut self)
    cached_sessions: Arc<OnceCell<Mutex<Session>>>,
}

impl ObjectDetectionPlugin {
    /// Create a new object detection plugin with model caching
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
                "Loading YOLO model from {} with CPU-only execution (first time only)",
                model_path.display()
            );

            // Use CPU-only execution for YOLOv8 models
            // CoreML execution provider is incompatible with YOLOv8 ONNX models
            // (fails during inference with "output_features has no value" error)
            let session = create_cpu_only_session(model_path)
                .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;

            info!("YOLO model loaded successfully with graph optimizations and cached for reuse");
            Ok(Mutex::new(session))
        })
    }

    /// Load plugin from YAML configuration
    pub fn from_yaml(yaml_path: impl AsRef<Path>) -> Result<Self, PluginError> {
        let contents = std::fs::read_to_string(yaml_path.as_ref())?;
        let config: PluginConfig = serde_yaml::from_str(&contents)
            .map_err(|e| PluginError::ExecutionFailed(format!("Failed to parse YAML: {}", e)))?;

        // Default model directory
        let model_dir = PathBuf::from("models/object-detection");

        Ok(Self::new(config, model_dir))
    }

    /// Convert core ObjectDetectionModel enum to crate-specific enum
    fn convert_model(model: &CoreObjectDetectionModel) -> YOLOModel {
        match model {
            CoreObjectDetectionModel::YoloV8n => YOLOModel::Nano,
            CoreObjectDetectionModel::YoloV8s => YOLOModel::Small,
            CoreObjectDetectionModel::YoloV8m => YOLOModel::Medium,
            CoreObjectDetectionModel::YoloV8l => YOLOModel::Large,
            CoreObjectDetectionModel::YoloV8x => YOLOModel::XLarge,
        }
    }

    /// Parse class names to class IDs (COCO classes)
    fn parse_classes(class_names: &Option<Vec<String>>) -> Option<Vec<u8>> {
        class_names.as_ref().map(|names| {
            names
                .iter()
                .filter_map(|name| {
                    crate::COCO_CLASSES
                        .iter()
                        .position(|&class| class == name)
                        .map(|pos| pos as u8)
                })
                .collect()
        })
    }
}

#[async_trait]
impl Plugin for ObjectDetectionPlugin {
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
        let (model, confidence_threshold, classes) = match &request.operation {
            Operation::ObjectDetection {
                model,
                confidence_threshold,
                classes,
            } => (model, *confidence_threshold, classes),
            _ => {
                return Err(PluginError::InvalidInput(
                    "Expected ObjectDetection operation".to_string(),
                ))
            }
        };

        if ctx.verbose {
            info!(
                "Detecting objects with model {:?}, confidence threshold: {:.2}",
                model, confidence_threshold
            );
        }

        // Configure detector
        let yolo_model = Self::convert_model(model);
        let model_path = self.model_dir.join(yolo_model.filename());

        let detector_config = ObjectDetectionConfig {
            confidence_threshold,
            classes: Self::parse_classes(classes),
            ..Default::default()
        };

        // Get or load cached ONNX session
        let session_mutex = self.get_or_load_session(&model_path)?;

        // Process input based on type
        let detections = match &request.input {
            PluginData::FilePath(path) => {
                debug!("Running object detection on: {}", path.display());

                // Load image with optimized I/O (mozjpeg for JPEG, 3-5x faster)
                let img = load_image(path).map_err(|e| {
                    PluginError::ExecutionFailed(format!("Failed to load image: {}", e))
                })?;

                // Perform detection with cached session (lock mutex for duration of inference)
                let mut session = session_mutex.lock().map_err(|e| {
                    PluginError::ExecutionFailed(format!("Failed to lock session mutex: {}", e))
                })?;
                ObjectDetector::detect_with_session(&mut session, &img, &detector_config).map_err(
                    |e| PluginError::ExecutionFailed(format!("Object detection failed: {}", e)),
                )?
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
                    debug!("Running object detection on {} keyframes", keyframes.len());

                    // Lock session once for all keyframes (better performance than lock/unlock per frame)
                    let mut session = session_mutex.lock().map_err(|e| {
                        PluginError::ExecutionFailed(format!("Failed to lock session mutex: {}", e))
                    })?;

                    // Pre-allocate detections Vec with keyframes.len() capacity
                    // Each keyframe produces one ObjectDetectionResult
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

                        // Perform detection
                        let mut frame_detections = ObjectDetector::detect_with_session(
                            &mut session,
                            &img,
                            &detector_config,
                        )
                        .map_err(|e| {
                            PluginError::ExecutionFailed(format!(
                                "Object detection failed on keyframe {}: {}",
                                idx, e
                            ))
                        })?;

                        // Populate frame_idx for each detection (frame_number from keyframe)
                        for detection in &mut frame_detections {
                            detection.frame_idx = Some(keyframe.frame_number as u32);
                        }

                        // Collect detections from this keyframe
                        all_detections.extend(frame_detections);
                    }

                    debug!(
                        "Object detection complete: {} total detections across {} keyframes",
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
                "Object detection complete in {:?}: {} objects detected",
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

impl From<ObjectDetectionError> for PluginError {
    fn from(err: ObjectDetectionError) -> Self {
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
            name: "object_detection".to_string(),
            description: "Test object detection plugin".to_string(),
            inputs: vec![
                "jpg".to_string(),
                "png".to_string(),
                "Keyframes".to_string(),
            ],
            outputs: vec!["ObjectDetection".to_string()],
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
        let plugin = ObjectDetectionPlugin::new(config, "models/yolo");

        assert_eq!(plugin.name(), "object_detection");
        assert!(plugin.supports_input("jpg"));
        assert!(plugin.supports_input("png"));
        assert!(plugin.supports_input("Keyframes"));
        assert!(plugin.produces_output("ObjectDetection"));
    }

    #[test]
    fn test_model_conversion() {
        assert!(matches!(
            ObjectDetectionPlugin::convert_model(&CoreObjectDetectionModel::YoloV8n),
            YOLOModel::Nano
        ));
        assert!(matches!(
            ObjectDetectionPlugin::convert_model(&CoreObjectDetectionModel::YoloV8s),
            YOLOModel::Small
        ));
        assert!(matches!(
            ObjectDetectionPlugin::convert_model(&CoreObjectDetectionModel::YoloV8m),
            YOLOModel::Medium
        ));
    }

    #[test]
    fn test_class_parsing() {
        // Test single class
        let classes = Some(vec!["person".to_string()]);
        let class_ids = ObjectDetectionPlugin::parse_classes(&classes);
        assert_eq!(class_ids, Some(vec![0]));

        // Test multiple classes
        let classes = Some(vec!["person".to_string(), "car".to_string()]);
        let class_ids = ObjectDetectionPlugin::parse_classes(&classes);
        assert_eq!(class_ids, Some(vec![0, 2]));

        // Test unknown class (should be filtered out)
        let classes = Some(vec!["person".to_string(), "unknown_class".to_string()]);
        let class_ids = ObjectDetectionPlugin::parse_classes(&classes);
        assert_eq!(class_ids, Some(vec![0]));

        // Test no classes
        let classes = None;
        let class_ids = ObjectDetectionPlugin::parse_classes(&classes);
        assert_eq!(class_ids, None);
    }
}
