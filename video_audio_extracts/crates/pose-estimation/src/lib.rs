//! Pose estimation module using YOLOv8-Pose via ONNX Runtime
//!
//! This module provides human pose estimation capabilities using YOLOv8-Pose models
//! exported to ONNX format. It detects people and their 17 COCO keypoints (nose, eyes,
//! ears, shoulders, elbows, wrists, hips, knees, ankles).
//!
//! # Features
//! - Multiple YOLOv8-Pose model sizes (Nano to XLarge)
//! - 17 COCO keypoints per person
//! - Configurable confidence thresholds for detection and keypoints
//! - Non-maximum suppression (NMS) for duplicate removal
//! - Hardware acceleration via ONNX Runtime (CUDA, TensorRT, CoreML)
//!
//! # Example
//! ```no_run
//! use video_audio_pose_estimation::{PoseEstimator, PoseEstimationConfig, YOLOPoseModel};
//! use image::open;
//!
//! # fn main() -> anyhow::Result<()> {
//! let config = PoseEstimationConfig::default();
//! let mut estimator = PoseEstimator::new("models/pose-estimation/yolov8n-pose.onnx", config)?;
//!
//! let img = open("image.jpg")?.to_rgb8();
//! let detections = estimator.estimate(&img)?;
//!
//! for detection in detections {
//!     println!("Person detected with {} keypoints (confidence: {:.2}%)",
//!              detection.keypoints.len(), detection.confidence * 100.0);
//! }
//! # Ok(())
//! # }
//! ```

pub mod plugin;

use image::RgbImage;
use ndarray::Array;
use ort::{
    session::{Session, SessionOutputs},
    value::TensorRef,
};
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;
use tracing::{debug, info};
use video_audio_common::ProcessingError;

/// YOLOv8-Pose model size variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum YOLOPoseModel {
    /// YOLOv8-Pose Nano - 13MB model, fastest inference (FP32)
    Nano,
    /// YOLOv8-Pose Nano INT8 - 3.6MB model, quantized INT8 (20-50% faster)
    NanoInt8,
    /// YOLOv8-Pose Small - 22MB model, balanced speed/accuracy
    Small,
    /// YOLOv8-Pose Medium - 52MB model, good accuracy
    Medium,
    /// YOLOv8-Pose Large - 87MB model, high accuracy
    Large,
    /// YOLOv8-Pose XLarge - 136MB model, highest accuracy
    XLarge,
}

impl YOLOPoseModel {
    /// Get the typical model filename for this size
    #[must_use]
    pub fn filename(&self) -> &'static str {
        match self {
            YOLOPoseModel::Nano => "yolov8n-pose.onnx",
            YOLOPoseModel::NanoInt8 => "yolov8n-pose-int8.onnx",
            YOLOPoseModel::Small => "yolov8s-pose.onnx",
            YOLOPoseModel::Medium => "yolov8m-pose.onnx",
            YOLOPoseModel::Large => "yolov8l-pose.onnx",
            YOLOPoseModel::XLarge => "yolov8x-pose.onnx",
        }
    }

    /// Get approximate model size in bytes
    #[must_use]
    pub fn size_bytes(&self) -> usize {
        match self {
            YOLOPoseModel::Nano => 13_000_000,
            YOLOPoseModel::NanoInt8 => 3_600_000,
            YOLOPoseModel::Small => 26_000_000,
            YOLOPoseModel::Medium => 55_000_000,
            YOLOPoseModel::Large => 90_000_000,
            YOLOPoseModel::XLarge => 140_000_000,
        }
    }
}

/// Configuration for pose estimation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoseEstimationConfig {
    /// Minimum confidence threshold for person detection (0.0-1.0)
    pub confidence_threshold: f32,
    /// Minimum confidence threshold for keypoint visibility (0.0-1.0)
    pub keypoint_threshold: f32,
    /// IoU threshold for non-maximum suppression (0.0-1.0)
    pub iou_threshold: f32,
    /// Maximum number of detections to return per image
    pub max_detections: usize,
    /// Input image size (YOLOv8-Pose default is 640x640)
    pub input_size: u32,
}

impl Default for PoseEstimationConfig {
    fn default() -> Self {
        Self {
            confidence_threshold: 0.25,
            keypoint_threshold: 0.5,
            iou_threshold: 0.45,
            max_detections: 100,
            input_size: 640,
        }
    }
}

impl PoseEstimationConfig {
    /// Create a fast pose estimation config (higher thresholds, fewer detections)
    #[must_use]
    pub fn fast() -> Self {
        Self {
            confidence_threshold: 0.5,
            keypoint_threshold: 0.6,
            iou_threshold: 0.5,
            max_detections: 50,
            input_size: 640,
        }
    }

    /// Create an accurate pose estimation config (lower thresholds, more detections)
    #[must_use]
    pub fn accurate() -> Self {
        Self {
            confidence_threshold: 0.15,
            keypoint_threshold: 0.3,
            iou_threshold: 0.4,
            max_detections: 200,
            input_size: 640,
        }
    }
}

/// COCO keypoint names (17 keypoints)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeypointName {
    Nose,
    LeftEye,
    RightEye,
    LeftEar,
    RightEar,
    LeftShoulder,
    RightShoulder,
    LeftElbow,
    RightElbow,
    LeftWrist,
    RightWrist,
    LeftHip,
    RightHip,
    LeftKnee,
    RightKnee,
    LeftAnkle,
    RightAnkle,
}

impl KeypointName {
    /// Get keypoint name from index (0-16)
    #[must_use]
    pub fn from_index(index: usize) -> Option<Self> {
        match index {
            0 => Some(KeypointName::Nose),
            1 => Some(KeypointName::LeftEye),
            2 => Some(KeypointName::RightEye),
            3 => Some(KeypointName::LeftEar),
            4 => Some(KeypointName::RightEar),
            5 => Some(KeypointName::LeftShoulder),
            6 => Some(KeypointName::RightShoulder),
            7 => Some(KeypointName::LeftElbow),
            8 => Some(KeypointName::RightElbow),
            9 => Some(KeypointName::LeftWrist),
            10 => Some(KeypointName::RightWrist),
            11 => Some(KeypointName::LeftHip),
            12 => Some(KeypointName::RightHip),
            13 => Some(KeypointName::LeftKnee),
            14 => Some(KeypointName::RightKnee),
            15 => Some(KeypointName::LeftAnkle),
            16 => Some(KeypointName::RightAnkle),
            _ => None,
        }
    }

    /// Get human-readable name
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            KeypointName::Nose => "nose",
            KeypointName::LeftEye => "left_eye",
            KeypointName::RightEye => "right_eye",
            KeypointName::LeftEar => "left_ear",
            KeypointName::RightEar => "right_ear",
            KeypointName::LeftShoulder => "left_shoulder",
            KeypointName::RightShoulder => "right_shoulder",
            KeypointName::LeftElbow => "left_elbow",
            KeypointName::RightElbow => "right_elbow",
            KeypointName::LeftWrist => "left_wrist",
            KeypointName::RightWrist => "right_wrist",
            KeypointName::LeftHip => "left_hip",
            KeypointName::RightHip => "right_hip",
            KeypointName::LeftKnee => "left_knee",
            KeypointName::RightKnee => "right_knee",
            KeypointName::LeftAnkle => "left_ankle",
            KeypointName::RightAnkle => "right_ankle",
        }
    }
}

/// Single keypoint with coordinates and confidence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keypoint {
    /// Keypoint name
    pub name: KeypointName,
    /// X coordinate (normalized 0-1)
    pub x: f32,
    /// Y coordinate (normalized 0-1)
    pub y: f32,
    /// Visibility confidence (0-1)
    pub confidence: f32,
}

impl Keypoint {
    /// Create a new keypoint
    #[must_use]
    pub fn new(name: KeypointName, x: f32, y: f32, confidence: f32) -> Self {
        Self {
            name,
            x,
            y,
            confidence,
        }
    }
}

/// Bounding box with normalized coordinates (0-1)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundingBox {
    /// X coordinate of top-left corner (normalized 0-1)
    pub x: f32,
    /// Y coordinate of top-left corner (normalized 0-1)
    pub y: f32,
    /// Width of box (normalized 0-1)
    pub width: f32,
    /// Height of box (normalized 0-1)
    pub height: f32,
}

impl BoundingBox {
    /// Create a new bounding box
    #[must_use]
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Get center coordinates
    #[must_use]
    pub fn center(&self) -> (f32, f32) {
        (self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    /// Get area of bounding box
    #[must_use]
    #[inline]
    pub fn area(&self) -> f32 {
        self.width * self.height
    }

    /// Calculate Intersection over Union (IoU) with another box
    #[must_use]
    #[inline]
    pub fn iou(&self, other: &BoundingBox) -> f32 {
        let x1 = self.x.max(other.x);
        let y1 = self.y.max(other.y);
        let x2 = (self.x + self.width).min(other.x + other.width);
        let y2 = (self.y + self.height).min(other.y + other.height);

        let intersection_width = (x2 - x1).max(0.0);
        let intersection_height = (y2 - y1).max(0.0);
        let intersection_area = intersection_width * intersection_height;

        let union_area = self.area() + other.area() - intersection_area;

        if union_area > 0.0 {
            intersection_area / union_area
        } else {
            0.0
        }
    }
}

/// Pose detection result (person with keypoints)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoseDetection {
    /// Person bounding box
    pub bbox: BoundingBox,
    /// Detection confidence score (0-1)
    pub confidence: f32,
    /// 17 COCO keypoints (may be < 17 if some keypoints filtered by threshold)
    pub keypoints: Vec<Keypoint>,
}

/// Pose estimator using YOLOv8-Pose ONNX model
pub struct PoseEstimator {
    session: Session,
    config: PoseEstimationConfig,
}

impl PoseEstimator {
    /// Create a new pose estimator with the given ONNX model path
    pub fn new<P: AsRef<Path>>(
        model_path: P,
        config: PoseEstimationConfig,
    ) -> Result<Self, PoseEstimationError> {
        info!("Loading YOLOv8-Pose model from {:?}", model_path.as_ref());

        let session = Session::builder()
            .map_err(|e| PoseEstimationError::ModelLoad(e.to_string()))?
            .commit_from_file(model_path)
            .map_err(|e| PoseEstimationError::ModelLoad(e.to_string()))?;

        info!("YOLOv8-Pose model loaded successfully");

        Ok(Self { session, config })
    }

    /// Estimate poses using a pre-loaded ONNX session (for model caching)
    ///
    /// # Arguments
    /// * `session` - Pre-loaded ONNX Session (mutable reference needed for run())
    /// * `image` - RGB image to process
    /// * `config` - Pose estimation configuration
    pub fn estimate_with_session(
        session: &mut Session,
        image: &RgbImage,
        config: &PoseEstimationConfig,
    ) -> Result<Vec<PoseDetection>, PoseEstimationError> {
        debug!(
            "Running pose estimation on {}x{} image",
            image.width(),
            image.height()
        );

        // Preprocess image to YOLOv8-Pose input format
        let input_array = Self::preprocess_image_static(image, config)?;

        // Run inference
        let outputs = Self::run_inference_static(session, &input_array)?;

        // Post-process outputs to pose detections
        let detections = Self::postprocess_outputs_static(outputs, config)?;

        info!("Detected {} people with poses", detections.len());

        Ok(detections)
    }

    /// Estimate poses in a single image
    pub fn estimate(
        &mut self,
        image: &RgbImage,
    ) -> Result<Vec<PoseDetection>, PoseEstimationError> {
        Self::estimate_with_session(&mut self.session, image, &self.config)
    }

    /// Estimate poses in multiple images (batch processing)
    pub fn estimate_batch(
        &mut self,
        images: &[RgbImage],
    ) -> Result<Vec<Vec<PoseDetection>>, PoseEstimationError> {
        let mut results = Vec::with_capacity(images.len());
        for img in images {
            results.push(self.estimate(img)?);
        }
        Ok(results)
    }

    /// Preprocess image to YOLOv8-Pose input format (1, 3, H, W) with normalization (static version)
    fn preprocess_image_static(
        image: &RgbImage,
        config: &PoseEstimationConfig,
    ) -> Result<Array<f32, ndarray::Dim<[usize; 4]>>, PoseEstimationError> {
        let input_size = config.input_size;

        // Resize image to input size (letterbox)
        let resized = image::imageops::resize(
            image,
            input_size,
            input_size,
            image::imageops::FilterType::Triangle,
        );

        // Convert to CHW format and normalize to [0, 1]
        let mut input_array = Array::zeros((1, 3, input_size as usize, input_size as usize));

        for y in 0..input_size as usize {
            for x in 0..input_size as usize {
                let pixel = resized.get_pixel(x as u32, y as u32);
                input_array[[0, 0, y, x]] = f32::from(pixel[0]) / 255.0;
                input_array[[0, 1, y, x]] = f32::from(pixel[1]) / 255.0;
                input_array[[0, 2, y, x]] = f32::from(pixel[2]) / 255.0;
            }
        }

        Ok(input_array)
    }

    /// Run ONNX inference (static version for model caching)
    fn run_inference_static<'a>(
        session: &'a mut Session,
        input: &Array<f32, ndarray::Dim<[usize; 4]>>,
    ) -> Result<SessionOutputs<'a>, PoseEstimationError> {
        // Zero-copy tensor: use view instead of clone
        // OLD: Value::from_array(input.clone()) - copies entire tensor
        // NEW: TensorRef::from_array_view(input.view()) - zero-copy borrow
        let input_tensor = TensorRef::from_array_view(input.view())
            .map_err(|e| PoseEstimationError::Inference(e.to_string()))?;

        let outputs = session
            .run(ort::inputs![input_tensor])
            .map_err(|e| PoseEstimationError::Inference(e.to_string()))?;

        Ok(outputs)
    }

    /// Post-process ONNX outputs to pose detections
    fn postprocess_outputs_static(
        outputs: SessionOutputs,
        config: &PoseEstimationConfig,
    ) -> Result<Vec<PoseDetection>, PoseEstimationError> {
        // YOLOv8-Pose output shape: (1, 56, 8400) or (batch, features, anchors)
        // Features: 4 box coords (xywh) + 1 objectness + 51 keypoint data (17 keypoints * 3: x, y, visibility)

        let output = &outputs[0];

        // Extract tensor data - returns (shape, data slice)
        let (shape, data) = output.try_extract_tensor::<f32>().map_err(|e| {
            PoseEstimationError::Inference(format!("Failed to extract tensor: {e}"))
        })?;

        debug!("ONNX output shape: {:?}", shape);

        // Convert shape to dimensions
        let dims = shape.as_ref();
        if dims.len() != 3 {
            return Err(PoseEstimationError::Inference(format!(
                "Expected 3D output tensor, got {}D",
                dims.len()
            )));
        }

        let _batch_size = dims[0] as usize;
        let num_features = dims[1] as usize; // Should be 56 (4 + 1 + 51)
        let num_anchors = dims[2] as usize; // 8400

        if num_features != 56 {
            return Err(PoseEstimationError::Inference(format!(
                "Expected 56 features, got {num_features}"
            )));
        }

        // Pre-allocate for ~10% of anchors passing confidence threshold (8400 anchors → ~840 detections)
        let mut raw_detections = Vec::with_capacity(num_anchors / 10);

        // Process each anchor
        // Data layout: [batch, features, anchors] = [1, 56, 8400]
        for anchor_idx in 0..num_anchors {
            // Helper function to get feature value for this anchor
            let get_feature = |feature_idx: usize| data[feature_idx * num_anchors + anchor_idx];

            // Extract box coordinates (center format) - features 0-3
            let x_center = get_feature(0);
            let y_center = get_feature(1);
            let width = get_feature(2);
            let height = get_feature(3);

            // Extract objectness (confidence) - feature 4
            let confidence = get_feature(4);

            // Filter by confidence threshold
            if confidence < config.confidence_threshold {
                continue;
            }

            // Extract 17 keypoints (features 5-55)
            // Each keypoint has 3 values: x, y, visibility
            let mut keypoints = Vec::with_capacity(17);
            for kp_idx in 0..17 {
                let base_feature = 5 + (kp_idx * 3);
                let kp_x = get_feature(base_feature);
                let kp_y = get_feature(base_feature + 1);
                let kp_conf = get_feature(base_feature + 2);

                // Filter keypoints by visibility threshold
                if kp_conf >= config.keypoint_threshold {
                    if let Some(name) = KeypointName::from_index(kp_idx) {
                        // Normalize keypoint coordinates and clamp to [0, 1]
                        // (model may output values slightly outside range due to floating point)
                        let x_norm = (kp_x / config.input_size as f32).clamp(0.0, 1.0);
                        let y_norm = (kp_y / config.input_size as f32).clamp(0.0, 1.0);

                        keypoints.push(Keypoint::new(name, x_norm, y_norm, kp_conf));
                    }
                }
            }

            // Convert from center format to corner format and normalize
            // Clamp to [0, 1] to handle floating point precision issues
            let x = ((x_center - width / 2.0) / config.input_size as f32).clamp(0.0, 1.0);
            let y = ((y_center - height / 2.0) / config.input_size as f32).clamp(0.0, 1.0);
            let w = (width / config.input_size as f32).clamp(0.0, 1.0);
            let h = (height / config.input_size as f32).clamp(0.0, 1.0);

            let bbox = BoundingBox::new(x, y, w, h);

            raw_detections.push(PoseDetection {
                bbox,
                confidence,
                keypoints,
            });
        }

        debug!("Raw pose detections before NMS: {}", raw_detections.len());

        // Apply non-maximum suppression
        let detections = Self::apply_nms_static(raw_detections, config);

        // Limit to max detections
        let detections: Vec<_> = detections.into_iter().take(config.max_detections).collect();

        Ok(detections)
    }

    /// Apply non-maximum suppression to remove duplicate detections
    fn apply_nms_static(
        mut detections: Vec<PoseDetection>,
        config: &PoseEstimationConfig,
    ) -> Vec<PoseDetection> {
        // Sort by confidence (highest first)
        detections.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Pre-allocate for upper bound (all detections pass NMS, typical: 50-80%)
        let mut keep = Vec::with_capacity(detections.len());

        while !detections.is_empty() {
            // Use swap_remove(0) for O(1) removal instead of O(n) remove(0)
            // Order doesn't matter after we take the best detection
            let current = detections.swap_remove(0);

            // Remove all detections with IoU > threshold (borrow current)
            detections.retain(|det| det.bbox.iou(&current.bbox) < config.iou_threshold);

            // Move current into keep (no clone needed - swap_remove(0) returns owned value)
            keep.push(current);
        }

        debug!("Pose detections after NMS: {}", keep.len());
        keep
    }
}

/// Error types for pose estimation
#[derive(Debug, Error)]
pub enum PoseEstimationError {
    #[error("Failed to load model: {0}")]
    ModelLoad(String),
    #[error("Inference error: {0}")]
    Inference(String),
    #[error("Image processing error: {0}")]
    ImageProcessing(String),
}

// Convert to ProcessingError for plugin integration
impl From<PoseEstimationError> for ProcessingError {
    fn from(error: PoseEstimationError) -> Self {
        ProcessingError::Other(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypoint_name_from_index() {
        assert_eq!(KeypointName::from_index(0), Some(KeypointName::Nose));
        assert_eq!(
            KeypointName::from_index(5),
            Some(KeypointName::LeftShoulder)
        );
        assert_eq!(KeypointName::from_index(16), Some(KeypointName::RightAnkle));
        assert_eq!(KeypointName::from_index(17), None);
    }

    #[test]
    fn test_bounding_box_iou() {
        let box1 = BoundingBox::new(0.0, 0.0, 0.5, 0.5);
        let box2 = BoundingBox::new(0.25, 0.25, 0.5, 0.5);

        let iou = box1.iou(&box2);
        assert!(iou > 0.0 && iou < 1.0);
    }

    #[test]
    fn test_pose_config_defaults() {
        let config = PoseEstimationConfig::default();
        assert_eq!(config.confidence_threshold, 0.25);
        assert_eq!(config.keypoint_threshold, 0.5);
        assert_eq!(config.input_size, 640);
    }
}
