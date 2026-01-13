//! Plugin wrapper for emotion detection module

use crate::{EmotionDetector, EmotionDetectorConfig};
use async_trait::async_trait;
use once_cell::sync::OnceCell;
use ort::session::Session;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tracing::{debug, info, warn};
use video_audio_face_detection::{FaceDetectionConfig, FaceDetector};
use video_extract_core::image_io::load_image;
use video_extract_core::onnx_utils::create_optimized_session;
use video_extract_core::plugin::PluginData;
use video_extract_core::{
    Context, Operation, Plugin, PluginConfig, PluginError, PluginRequest, PluginResponse,
};

/// Emotion detection plugin implementation with model caching
pub struct EmotionDetectionPlugin {
    config: PluginConfig,
    model_dir: PathBuf,
    /// Cached ONNX Session - loaded once and reused across all executions
    cached_session: Arc<OnceCell<Mutex<Session>>>,
}

impl EmotionDetectionPlugin {
    /// Create a new emotion detection plugin with model caching
    pub fn new(config: PluginConfig, model_dir: impl AsRef<Path>) -> Self {
        Self {
            config,
            model_dir: model_dir.as_ref().to_path_buf(),
            cached_session: Arc::new(OnceCell::new()),
        }
    }

    /// Get or load the ONNX Session (cached after first load)
    fn get_or_load_session(&self, model_path: &Path) -> Result<&Mutex<Session>, PluginError> {
        self.cached_session.get_or_try_init(|| {
            info!(
                "Loading emotion detection model from {} with optimizations (first time only)",
                model_path.display()
            );

            let session = create_optimized_session(model_path)
                .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;

            info!("Emotion detection model loaded successfully and cached for reuse");
            Ok(Mutex::new(session))
        })
    }

    /// Load plugin from YAML configuration
    pub fn from_yaml(yaml_path: impl AsRef<Path>) -> Result<Self, PluginError> {
        let contents = std::fs::read_to_string(yaml_path.as_ref())?;
        let config: PluginConfig = serde_yaml::from_str(&contents)
            .map_err(|e| PluginError::ExecutionFailed(format!("Failed to parse YAML: {}", e)))?;

        // Default model directory
        let model_dir = PathBuf::from("models/emotion-detection");

        Ok(Self::new(config, model_dir))
    }

    /// Check if the image contains faces using RetinaFace detector
    /// Returns the number of faces detected
    fn check_for_faces(&self, img: &image::RgbImage) -> Result<usize, PluginError> {
        // Use fast MobileNet model with default config for quick face check
        let face_model_path = PathBuf::from("models/face-detection/retinaface_mnet025.onnx");

        if !face_model_path.exists() {
            warn!(
                "Face detection model not found at {}. Skipping face validation.",
                face_model_path.display()
            );
            // If face detection model doesn't exist, allow emotion detection to proceed
            // (backwards compatibility - don't break existing workflows)
            return Ok(1);
        }

        let face_config = FaceDetectionConfig {
            confidence_threshold: 0.5, // Lower threshold to be more permissive
            ..Default::default()
        };

        let mut face_detector = FaceDetector::new(&face_model_path, face_config)
            .map_err(|e| PluginError::ExecutionFailed(format!("Failed to create face detector: {}", e)))?;

        let faces = face_detector
            .detect(img)
            .map_err(|e| PluginError::ExecutionFailed(format!("Face detection failed: {}", e)))?;

        Ok(faces.len())
    }
}

#[async_trait]
impl Plugin for EmotionDetectionPlugin {
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
        self.config
            .outputs
            .iter()
            .any(|o| o.eq_ignore_ascii_case(output_type))
    }

    async fn execute(
        &self,
        ctx: &Context,
        request: &PluginRequest,
    ) -> Result<PluginResponse, PluginError> {
        let start = Instant::now();

        // Extract operation parameters
        let include_probabilities = match &request.operation {
            Operation::EmotionDetection {
                include_probabilities,
            } => *include_probabilities,
            _ => {
                return Err(PluginError::InvalidInput(
                    "Expected EmotionDetection operation".to_string(),
                ))
            }
        };

        if ctx.verbose {
            info!(
                "Detecting emotions (include_probabilities: {})",
                include_probabilities
            );
        }

        // Try FER+ model first (better quality), fallback to ResNet18
        let ferplus_path = self.model_dir.join("emotion-ferplus-8.onnx");
        let resnet_path = self.model_dir.join("emotion_resnet18.onnx");

        let model_path = if ferplus_path.exists() {
            info!("Using FER+ emotion model (emotion-ferplus-8.onnx)");
            ferplus_path
        } else if resnet_path.exists() {
            info!("Using ResNet18 emotion model (emotion_resnet18.onnx)");
            resnet_path
        } else {
            return Err(PluginError::ExecutionFailed(format!(
                "No emotion model found in {}. Expected emotion-ferplus-8.onnx or emotion_resnet18.onnx",
                self.model_dir.display()
            )));
        };

        // Get or load cached session
        let _session_mutex = self.get_or_load_session(&model_path)?;

        // Configure detector
        let detector_config = EmotionDetectorConfig {
            include_probabilities,
        };

        // Process input based on type
        let result_json = match &request.input {
            PluginData::FilePath(path) => {
                debug!("Running emotion detection on: {}", path.display());

                // Load image
                let img = load_image(path).map_err(|e| {
                    PluginError::ExecutionFailed(format!("Failed to load image: {}", e))
                })?;

                // Validate that image contains faces before running emotion detection
                let face_count = self.check_for_faces(&img)?;
                if face_count == 0 {
                    warn!(
                        "No faces detected in image {}. Emotion detection requires faces.",
                        path.display()
                    );
                    return Err(PluginError::InvalidInput(
                        "No faces detected in image. Emotion detection requires at least one face.".to_string(),
                    ));
                }

                debug!("Detected {} face(s), proceeding with emotion detection", face_count);

                // Create detector
                let detector = EmotionDetector::new(&model_path, detector_config).map_err(|e| {
                    PluginError::ExecutionFailed(format!("Failed to create detector: {}", e))
                })?;

                // Detect emotion
                let result = detector
                    .detect(&image::DynamicImage::ImageRgb8(img))
                    .map_err(|e| {
                        PluginError::ExecutionFailed(format!("Emotion detection failed: {}", e))
                    })?;

                serde_json::json!({
                    "emotion": result.emotion.as_str(),
                    "confidence": result.confidence,
                    "probabilities": result.probabilities,
                })
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
                    return Err(PluginError::InvalidInput(
                        "No keyframes to process".to_string(),
                    ));
                }

                debug!("Running emotion detection on {} keyframes", keyframes.len());

                let mut all_results = Vec::with_capacity(keyframes.len());

                // Process each keyframe
                for (idx, keyframe) in keyframes.iter().enumerate() {
                    // Get image path from keyframe
                    let image_path = keyframe.thumbnail_paths.values().next().ok_or_else(|| {
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

                    // Validate that keyframe contains faces before running emotion detection
                    let face_count = self.check_for_faces(&img)?;
                    if face_count == 0 {
                        debug!(
                            "No faces detected in keyframe {} at t={:.2}s, skipping emotion detection",
                            idx, keyframe.timestamp
                        );
                        // Skip this keyframe (don't add to results) if no faces found
                        continue;
                    }

                    debug!(
                        "Detected {} face(s) in keyframe {}, proceeding with emotion detection",
                        face_count, idx
                    );

                    // Create detector
                    let detector = EmotionDetector::new(&model_path, detector_config.clone())
                        .map_err(|e| {
                            PluginError::ExecutionFailed(format!(
                                "Failed to create detector: {}",
                                e
                            ))
                        })?;

                    // Detect emotion
                    let result = detector
                        .detect(&image::DynamicImage::ImageRgb8(img))
                        .map_err(|e| {
                            PluginError::ExecutionFailed(format!(
                                "Emotion detection failed for keyframe {}: {}",
                                idx, e
                            ))
                        })?;

                    all_results.push(serde_json::json!({
                        "timestamp": keyframe.timestamp,
                        "emotion": result.emotion.as_str(),
                        "confidence": result.confidence,
                        "probabilities": result.probabilities,
                    }));
                }

                serde_json::json!({
                    "emotions": all_results
                })
            }
            PluginData::Multiple(_) => {
                return Err(PluginError::UnsupportedFormat(
                    "Multiple input not yet supported for emotion detection".to_string(),
                ));
            }
        };

        let elapsed = start.elapsed();
        debug!("Emotion detection completed in {:?}", elapsed);

        Ok(PluginResponse {
            output: PluginData::Json(result_json),
            duration: elapsed,
            warnings: Vec::new(),
        })
    }
}
