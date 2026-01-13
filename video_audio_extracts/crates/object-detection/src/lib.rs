//! Object detection module using `YOLOv8` via ONNX Runtime
//!
//! This module provides object detection capabilities using `YOLOv8` models
//! exported to ONNX format. It supports multiple model sizes (Nano to `XLarge`)
//! and can detect 80 COCO object classes.
//!
//! # Features
//! - Multiple `YOLOv8` model sizes (Nano 6MB to `XLarge` 136MB)
//! - 80 COCO object classes (person, car, laptop, etc.)
//! - Configurable confidence and `IoU` thresholds
//! - Class filtering for targeted detection
//! - Non-maximum suppression (NMS) for duplicate removal
//! - Hardware acceleration via ONNX Runtime (CUDA, `TensorRT`, `CoreML`)
//!
//! # Example
//! ```no_run
//! use video_audio_object_detection::{ObjectDetector, ObjectDetectionConfig, YOLOModel};
//! use image::open;
//!
//! # fn main() -> anyhow::Result<()> {
//! let config = ObjectDetectionConfig::default();
//! let mut detector = ObjectDetector::new("yolov8n.onnx", config)?;
//!
//! let img = open("image.jpg")?.to_rgb8();
//! let detections = detector.detect(&img)?;
//!
//! for detection in detections {
//!     println!("{}: {:.2}%", detection.class_name, detection.confidence * 100.0);
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

/// `YOLOv8` model size variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum YOLOModel {
    /// `YOLOv8` Nano - 6MB model, fastest inference
    Nano,
    /// `YOLOv8` Small - 22MB model, balanced speed/accuracy
    Small,
    /// `YOLOv8` Medium - 52MB model, good accuracy
    Medium,
    /// `YOLOv8` Large - 87MB model, high accuracy
    Large,
    /// `YOLOv8` `XLarge` - 136MB model, highest accuracy
    XLarge,
}

impl YOLOModel {
    /// Get the typical model filename for this size
    #[must_use]
    pub fn filename(&self) -> &'static str {
        match self {
            YOLOModel::Nano => "yolov8n.onnx",
            YOLOModel::Small => "yolov8s.onnx",
            YOLOModel::Medium => "yolov8m.onnx",
            YOLOModel::Large => "yolov8l.onnx",
            YOLOModel::XLarge => "yolov8x.onnx",
        }
    }

    /// Get approximate model size in bytes
    #[must_use]
    pub fn size_bytes(&self) -> usize {
        match self {
            YOLOModel::Nano => 6_000_000,
            YOLOModel::Small => 43_000_000,  // Updated to actual ONNX size (was 22MB)
            YOLOModel::Medium => 52_000_000,
            YOLOModel::Large => 87_000_000,
            YOLOModel::XLarge => 136_000_000,
        }
    }
}

/// Configuration for object detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectDetectionConfig {
    /// Minimum confidence threshold for detections (0.0-1.0)
    pub confidence_threshold: f32,
    /// `IoU` threshold for non-maximum suppression (0.0-1.0)
    pub iou_threshold: f32,
    /// Filter detections to specific COCO class IDs (None = all classes)
    pub classes: Option<Vec<u8>>,
    /// Maximum number of detections to return per image
    pub max_detections: usize,
    /// Input image size (`YOLOv8` default is 640x640)
    pub input_size: u32,
}

impl Default for ObjectDetectionConfig {
    fn default() -> Self {
        Self {
            confidence_threshold: 0.25,
            iou_threshold: 0.45,
            classes: None,
            max_detections: 300,
            input_size: 640,
        }
    }
}

impl ObjectDetectionConfig {
    /// Create a fast detection config (higher thresholds, fewer detections)
    #[must_use]
    pub fn fast() -> Self {
        Self {
            confidence_threshold: 0.5,
            iou_threshold: 0.5,
            classes: None,
            max_detections: 100,
            input_size: 640,
        }
    }

    /// Create an accurate detection config (lower thresholds, more detections)
    #[must_use]
    pub fn accurate() -> Self {
        Self {
            confidence_threshold: 0.15,
            iou_threshold: 0.4,
            classes: None,
            max_detections: 500,
            input_size: 640,
        }
    }

    /// Create a config for person detection only
    #[must_use]
    pub fn person_only() -> Self {
        Self {
            confidence_threshold: 0.3,
            iou_threshold: 0.45,
            classes: Some(vec![0]), // COCO class 0 = person
            max_detections: 100,
            input_size: 640,
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

    /// Calculate Intersection over Union (`IoU`) with another box
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

/// Object detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Detection {
    /// COCO class ID (0-79)
    pub class_id: u8,
    /// Human-readable class name
    pub class_name: String,
    /// Confidence score (0-1)
    pub confidence: f32,
    /// Bounding box with normalized coordinates
    pub bbox: BoundingBox,
    /// Frame index (for video processing, None for single images)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame_idx: Option<u32>,
}

/// Object detector using `YOLOv8` ONNX model
pub struct ObjectDetector {
    session: Session,
    config: ObjectDetectionConfig,
}

impl ObjectDetector {
    /// Create a new object detector with the given ONNX model path
    pub fn new<P: AsRef<Path>>(
        model_path: P,
        config: ObjectDetectionConfig,
    ) -> Result<Self, ObjectDetectionError> {
        info!("Loading YOLOv8 model from {:?}", model_path.as_ref());

        let session = Session::builder()
            .map_err(|e| ObjectDetectionError::ModelLoad(e.to_string()))?
            .commit_from_file(model_path)
            .map_err(|e| ObjectDetectionError::ModelLoad(e.to_string()))?;

        info!("YOLOv8 model loaded successfully");

        Ok(Self { session, config })
    }

    /// Detect objects using a pre-loaded ONNX session (for model caching)
    ///
    /// # Arguments
    /// * `session` - Pre-loaded ONNX Session (mutable reference needed for run())
    /// * `image` - RGB image to process
    /// * `config` - Detection configuration
    pub fn detect_with_session(
        session: &mut Session,
        image: &RgbImage,
        config: &ObjectDetectionConfig,
    ) -> Result<Vec<Detection>, ObjectDetectionError> {
        debug!(
            "Running object detection on {}x{} image",
            image.width(),
            image.height()
        );

        // Preprocess image to YOLOv8 input format
        let input_array = Self::preprocess_image_static(image, config)?;

        // Run inference
        let outputs = Self::run_inference_static(session, &input_array)?;

        // Post-process outputs to detections
        let detections = Self::postprocess_outputs_static(outputs, config)?;

        info!("Detected {} objects", detections.len());

        Ok(detections)
    }

    /// Detect objects in a single image
    pub fn detect(&mut self, image: &RgbImage) -> Result<Vec<Detection>, ObjectDetectionError> {
        Self::detect_with_session(&mut self.session, image, &self.config)
    }

    /// Detect objects in multiple images (batch processing)
    pub fn detect_batch(
        &mut self,
        images: &[RgbImage],
    ) -> Result<Vec<Vec<Detection>>, ObjectDetectionError> {
        let mut results = Vec::with_capacity(images.len());
        for img in images {
            results.push(self.detect(img)?);
        }
        Ok(results)
    }

    /// Preprocess image to `YOLOv8` input format (1, 3, H, W) with normalization (static version)
    fn preprocess_image_static(
        image: &RgbImage,
        config: &ObjectDetectionConfig,
    ) -> Result<Array<f32, ndarray::Dim<[usize; 4]>>, ObjectDetectionError> {
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

    /// Preprocess image to `YOLOv8` input format (1, 3, H, W) with normalization
    #[allow(dead_code)]
    fn preprocess_image(
        &self,
        image: &RgbImage,
    ) -> Result<Array<f32, ndarray::Dim<[usize; 4]>>, ObjectDetectionError> {
        Self::preprocess_image_static(image, &self.config)
    }

    /// Run ONNX inference (static version for model caching)
    fn run_inference_static<'a>(
        session: &'a mut Session,
        input: &Array<f32, ndarray::Dim<[usize; 4]>>,
    ) -> Result<SessionOutputs<'a>, ObjectDetectionError> {
        // Zero-copy tensor: use view instead of clone
        let input_tensor = TensorRef::from_array_view(input.view())
            .map_err(|e| ObjectDetectionError::Inference(e.to_string()))?;

        let outputs = session
            .run(ort::inputs![input_tensor])
            .map_err(|e| ObjectDetectionError::Inference(e.to_string()))?;

        Ok(outputs)
    }

    /// Run ONNX inference
    #[allow(dead_code)]
    fn run_inference(
        &mut self,
        input: &Array<f32, ndarray::Dim<[usize; 4]>>,
    ) -> Result<SessionOutputs<'_>, ObjectDetectionError> {
        Self::run_inference_static(&mut self.session, input)
    }

    /// Post-process ONNX outputs to detections
    fn postprocess_outputs_static(
        outputs: SessionOutputs,
        config: &ObjectDetectionConfig,
    ) -> Result<Vec<Detection>, ObjectDetectionError> {
        // YOLOv8 output shape: (1, 84, 8400) or (batch, 4+classes, anchors)
        // First 4 values are [x_center, y_center, width, height]
        // Remaining 80 values are class probabilities

        let output = &outputs[0];

        // Extract tensor data - returns (shape, data slice)
        let (shape, data) = output.try_extract_tensor::<f32>().map_err(|e| {
            ObjectDetectionError::Inference(format!("Failed to extract tensor: {e}"))
        })?;

        debug!("ONNX output shape: {:?}", shape);

        // Convert shape to dimensions
        let dims = shape.as_ref();
        if dims.len() != 3 {
            return Err(ObjectDetectionError::Inference(format!(
                "Expected 3D output tensor, got {}D",
                dims.len()
            )));
        }

        let _batch_size = dims[0] as usize;
        let _num_features = dims[1] as usize; // 84 = 4 box coords + 80 classes
        let num_anchors = dims[2] as usize; // 8400
                                            // Pre-allocate for ~10% of anchors passing confidence threshold (8400 anchors → ~840 detections)
        let mut raw_detections = Vec::with_capacity(num_anchors / 10);

        // Process each anchor
        // Data layout: [batch, features, anchors] = [1, 84, 8400]
        // For each anchor i, features are at indices [i*84..(i+1)*84]
        // But we need to access as data[batch][feature][anchor]
        for anchor_idx in 0..num_anchors {
            // For data layout [batch, features, anchors], anchor i's features are:
            // data[feature * num_anchors + anchor_idx] for each feature

            // Helper function to get feature value for this anchor
            let get_feature = |feature_idx: usize| data[feature_idx * num_anchors + anchor_idx];

            // Extract box coordinates (center format) - features 0-3
            let x_center = get_feature(0);
            let y_center = get_feature(1);
            let width = get_feature(2);
            let height = get_feature(3);

            // Find class with highest probability - features 4-83 (80 classes)
            let mut max_prob = 0.0f32;
            let mut max_class_id = 0usize;

            for class_id in 0..80usize {
                let prob = get_feature(4 + class_id);
                if prob > max_prob {
                    max_prob = prob;
                    max_class_id = class_id;
                }
            }

            let confidence = max_prob;

            // Filter by confidence threshold
            if confidence < config.confidence_threshold {
                continue;
            }

            // Filter by class if specified
            if let Some(ref classes) = config.classes {
                if !classes.contains(&(max_class_id as u8)) {
                    continue;
                }
            }

            // Convert from center format to corner format and normalize
            let x = (x_center - width / 2.0) / config.input_size as f32;
            let y = (y_center - height / 2.0) / config.input_size as f32;
            let w = width / config.input_size as f32;
            let h = height / config.input_size as f32;

            let bbox = BoundingBox::new(x, y, w, h);

            raw_detections.push(Detection {
                class_id: max_class_id as u8,
                class_name: get_coco_class_name(max_class_id as u8).to_string(),
                confidence,
                bbox,
                frame_idx: None,
            });
        }

        debug!("Raw detections before NMS: {}", raw_detections.len());

        // Apply non-maximum suppression
        let detections = Self::apply_nms_static(raw_detections, config);

        // Limit to max detections
        let detections: Vec<_> = detections.into_iter().take(config.max_detections).collect();

        Ok(detections)
    }

    /// Apply non-maximum suppression to remove duplicate detections
    fn apply_nms_static(
        mut detections: Vec<Detection>,
        config: &ObjectDetectionConfig,
    ) -> Vec<Detection> {
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

            // Remove all detections with IoU > threshold for the same class (borrow current)
            detections.retain(|det| {
                det.class_id != current.class_id
                    || det.bbox.iou(&current.bbox) < config.iou_threshold
            });

            // Move current into keep (no clone needed - swap_remove(0) returns owned value)
            keep.push(current);
        }

        debug!("Detections after NMS: {}", keep.len());
        keep
    }
}

/// Get COCO class name from class ID (0-79)
#[must_use]
pub fn get_coco_class_name(class_id: u8) -> &'static str {
    COCO_CLASSES.get(class_id as usize).unwrap_or(&"unknown")
}

/// Error types for object detection
#[derive(Debug, Error)]
pub enum ObjectDetectionError {
    #[error("Failed to load model: {0}")]
    ModelLoad(String),

    #[error("Inference error: {0}")]
    Inference(String),

    #[error("Image processing error: {0}")]
    ImageProcessing(String),

    #[error("ONNX Runtime error: {0}")]
    OnnxRuntime(#[from] ort::Error),
}

impl From<ObjectDetectionError> for ProcessingError {
    fn from(err: ObjectDetectionError) -> Self {
        ProcessingError::Other(err.to_string())
    }
}

/// 80 COCO object classes (in order)
pub const COCO_CLASSES: &[&str] = &[
    "person",
    "bicycle",
    "car",
    "motorcycle",
    "airplane",
    "bus",
    "train",
    "truck",
    "boat",
    "traffic light",
    "fire hydrant",
    "stop sign",
    "parking meter",
    "bench",
    "bird",
    "cat",
    "dog",
    "horse",
    "sheep",
    "cow",
    "elephant",
    "bear",
    "zebra",
    "giraffe",
    "backpack",
    "umbrella",
    "handbag",
    "tie",
    "suitcase",
    "frisbee",
    "skis",
    "snowboard",
    "sports ball",
    "kite",
    "baseball bat",
    "baseball glove",
    "skateboard",
    "surfboard",
    "tennis racket",
    "bottle",
    "wine glass",
    "cup",
    "fork",
    "knife",
    "spoon",
    "bowl",
    "banana",
    "apple",
    "sandwich",
    "orange",
    "broccoli",
    "carrot",
    "hot dog",
    "pizza",
    "donut",
    "cake",
    "chair",
    "couch",
    "potted plant",
    "bed",
    "dining table",
    "toilet",
    "tv",
    "laptop",
    "mouse",
    "remote",
    "keyboard",
    "cell phone",
    "microwave",
    "oven",
    "toaster",
    "sink",
    "refrigerator",
    "book",
    "clock",
    "vase",
    "scissors",
    "teddy bear",
    "hair drier",
    "toothbrush",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_yolo_model_filenames() {
        assert_eq!(YOLOModel::Nano.filename(), "yolov8n.onnx");
        assert_eq!(YOLOModel::Small.filename(), "yolov8s.onnx");
        assert_eq!(YOLOModel::Medium.filename(), "yolov8m.onnx");
        assert_eq!(YOLOModel::Large.filename(), "yolov8l.onnx");
        assert_eq!(YOLOModel::XLarge.filename(), "yolov8x.onnx");
    }

    #[test]
    fn test_yolo_model_sizes() {
        assert_eq!(YOLOModel::Nano.size_bytes(), 6_000_000);
        assert_eq!(YOLOModel::Small.size_bytes(), 43_000_000);
        assert_eq!(YOLOModel::Medium.size_bytes(), 52_000_000);
        assert_eq!(YOLOModel::Large.size_bytes(), 87_000_000);
        assert_eq!(YOLOModel::XLarge.size_bytes(), 136_000_000);
    }

    #[test]
    fn test_config_defaults() {
        let config = ObjectDetectionConfig::default();
        assert_eq!(config.confidence_threshold, 0.25);
        assert_eq!(config.iou_threshold, 0.45);
        assert_eq!(config.max_detections, 300);
        assert_eq!(config.input_size, 640);
        assert!(config.classes.is_none());
    }

    #[test]
    fn test_config_presets() {
        let fast = ObjectDetectionConfig::fast();
        assert_eq!(fast.confidence_threshold, 0.5);
        assert_eq!(fast.max_detections, 100);

        let accurate = ObjectDetectionConfig::accurate();
        assert_eq!(accurate.confidence_threshold, 0.15);
        assert_eq!(accurate.max_detections, 500);

        let person_only = ObjectDetectionConfig::person_only();
        assert_eq!(person_only.classes, Some(vec![0]));
    }

    #[test]
    fn test_bbox_iou() {
        let box1 = BoundingBox::new(0.0, 0.0, 0.5, 0.5);
        let box2 = BoundingBox::new(0.25, 0.25, 0.5, 0.5);

        // Overlapping boxes should have IoU > 0
        let iou = box1.iou(&box2);
        assert!(iou > 0.0 && iou < 1.0);

        // Identical boxes should have IoU = 1.0
        let box3 = BoundingBox::new(0.0, 0.0, 0.5, 0.5);
        let iou_same = box1.iou(&box3);
        assert!((iou_same - 1.0).abs() < 0.001);

        // Non-overlapping boxes should have IoU = 0
        let box4 = BoundingBox::new(0.6, 0.6, 0.3, 0.3);
        let iou_none = box1.iou(&box4);
        assert_eq!(iou_none, 0.0);
    }

    #[test]
    fn test_bbox_area() {
        let bbox = BoundingBox::new(0.0, 0.0, 0.5, 0.4);
        assert_eq!(bbox.area(), 0.2);
    }

    #[test]
    fn test_bbox_center() {
        let bbox = BoundingBox::new(0.1, 0.2, 0.4, 0.6);
        let (cx, cy) = bbox.center();
        assert_eq!(cx, 0.3);
        assert_eq!(cy, 0.5);
    }

    #[test]
    fn test_coco_classes() {
        assert_eq!(COCO_CLASSES.len(), 80);
        assert_eq!(COCO_CLASSES[0], "person");
        assert_eq!(COCO_CLASSES[2], "car");
        assert_eq!(COCO_CLASSES[63], "laptop");
    }

    #[test]
    fn test_get_coco_class_name() {
        assert_eq!(get_coco_class_name(0), "person");
        assert_eq!(get_coco_class_name(2), "car");
        assert_eq!(get_coco_class_name(63), "laptop");
        assert_eq!(get_coco_class_name(79), "toothbrush");
        assert_eq!(get_coco_class_name(200), "unknown");
    }
}
