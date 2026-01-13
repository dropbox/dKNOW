//! Logo detection module with two approaches
//!
//! This module provides brand logo detection capabilities:
//! 1. **YOLOv8 approach**: Custom trained models on logo datasets (requires model training)
//! 2. **CLIP approach** (recommended): Zero-shot detection via similarity search (no training required)
//!
//! # Features
//! - **CLIP-based detection** (lib_clip): Zero-shot logo detection using pre-trained CLIP embeddings
//! - **YOLOv8 detection**: Custom YOLOv8 models trained on logo datasets
//! - Configurable confidence thresholds
//! - Non-maximum suppression (NMS) for duplicate removal
//! - Hardware acceleration via ONNX Runtime (CUDA, TensorRT, CoreML)
//!
//! # Example (CLIP - Recommended)
//! ```no_run
//! use video_audio_logo_detection::{ClipLogoDetector, ClipLogoConfig};
//! use image::open;
//!
//! # fn main() -> anyhow::Result<()> {
//! let config = ClipLogoConfig::default();
//! let mut detector = ClipLogoDetector::new(
//!     "models/embeddings/clip_vit_b32.onnx",
//!     "models/logo-detection/clip_database/logo_database.json",
//!     config
//! )?;
//!
//! let img = open("image.jpg")?.to_rgb8();
//! let detections = detector.detect(&img)?;
//!
//! for detection in detections {
//!     println!("{}: {:.2}%", detection.brand, detection.confidence * 100.0);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Example (YOLOv8)
//! ```no_run
//! use video_audio_logo_detection::{LogoDetector, LogoDetectionConfig};
//! use image::open;
//!
//! # fn main() -> anyhow::Result<()> {
//! let config = LogoDetectionConfig::default();
//! let mut detector = LogoDetector::new(
//!     "yolov8_logo.onnx",
//!     "logos.txt",
//!     config
//! )?;
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

pub mod lib_clip;
pub mod plugin;

use image::RgbImage;
use ndarray::Array;
use ort::{
    session::{Session, SessionOutputs},
    value::TensorRef,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use thiserror::Error;
use tracing::{debug, info};
use video_audio_common::ProcessingError;

/// Configuration for logo detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogoDetectionConfig {
    /// Minimum confidence threshold for detections (0.0-1.0)
    pub confidence_threshold: f32,
    /// IoU threshold for non-maximum suppression (0.0-1.0)
    pub iou_threshold: f32,
    /// Filter detections to specific logo class IDs (None = all classes)
    pub classes: Option<Vec<u32>>,
    /// Maximum number of detections to return per image
    pub max_detections: usize,
    /// Input image size (YOLOv8 default is 640x640)
    pub input_size: u32,
}

impl Default for LogoDetectionConfig {
    fn default() -> Self {
        Self {
            confidence_threshold: 0.50, // N=241: Increased from 0.35 to reduce false positives
            iou_threshold: 0.45,
            classes: None,
            max_detections: 100, // Fewer logos per image typically
            input_size: 640,
        }
    }
}

impl LogoDetectionConfig {
    /// Create a fast detection config (higher thresholds, fewer detections)
    #[must_use]
    pub fn fast() -> Self {
        Self {
            confidence_threshold: 0.55,
            iou_threshold: 0.5,
            classes: None,
            max_detections: 50,
            input_size: 640,
        }
    }

    /// Create an accurate detection config (lower thresholds, more detections)
    #[must_use]
    pub fn accurate() -> Self {
        Self {
            confidence_threshold: 0.20,
            iou_threshold: 0.4,
            classes: None,
            max_detections: 200,
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

/// Logo detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Detection {
    /// Logo class ID (0 to num_classes-1)
    pub class_id: u32,
    /// Human-readable logo/brand name
    pub class_name: String,
    /// Confidence score (0-1)
    pub confidence: f32,
    /// Bounding box with normalized coordinates
    pub bbox: BoundingBox,
}

/// Logo detector using YOLOv8 ONNX model
pub struct LogoDetector {
    session: Session,
    config: LogoDetectionConfig,
    /// Logo class names loaded from file (index = class_id)
    class_names: Vec<String>,
    /// Number of classes in the model
    num_classes: usize,
}

impl LogoDetector {
    /// Create a new logo detector with the given ONNX model path and class names file
    ///
    /// # Arguments
    /// * `model_path` - Path to YOLOv8 ONNX model trained on logo dataset
    /// * `classes_path` - Path to text file with logo class names (one per line)
    /// * `config` - Detection configuration
    pub fn new<P: AsRef<Path>>(
        model_path: P,
        classes_path: P,
        config: LogoDetectionConfig,
    ) -> Result<Self, LogoDetectionError> {
        info!("Loading YOLOv8 logo model from {:?}", model_path.as_ref());

        let session = Session::builder()
            .map_err(|e| LogoDetectionError::ModelLoad(e.to_string()))?
            .commit_from_file(model_path)
            .map_err(|e| LogoDetectionError::ModelLoad(e.to_string()))?;

        info!("YOLOv8 logo model loaded successfully");

        // Load class names from file
        let class_names = Self::load_class_names(classes_path)?;
        let num_classes = class_names.len();

        info!("Loaded {} logo classes", num_classes);

        Ok(Self {
            session,
            config,
            class_names,
            num_classes,
        })
    }

    /// Load class names from text file (one class name per line)
    fn load_class_names<P: AsRef<Path>>(path: P) -> Result<Vec<String>, LogoDetectionError> {
        let contents = fs::read_to_string(path.as_ref()).map_err(|e| {
            LogoDetectionError::ClassNamesLoad(format!(
                "Failed to read class names file {:?}: {}",
                path.as_ref(),
                e
            ))
        })?;

        let names: Vec<String> = contents
            .lines()
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty())
            .collect();

        if names.is_empty() {
            return Err(LogoDetectionError::ClassNamesLoad(
                "Class names file is empty".to_string(),
            ));
        }

        Ok(names)
    }

    /// Detect logos using a pre-loaded ONNX session (for model caching)
    ///
    /// # Arguments
    /// * `session` - Pre-loaded ONNX Session (mutable reference needed for run())
    /// * `image` - RGB image to process
    /// * `config` - Detection configuration
    /// * `class_names` - Logo class names (index = class_id)
    /// * `num_classes` - Number of logo classes in the model
    pub fn detect_with_session(
        session: &mut Session,
        image: &RgbImage,
        config: &LogoDetectionConfig,
        class_names: &[String],
        num_classes: usize,
    ) -> Result<Vec<Detection>, LogoDetectionError> {
        debug!(
            "Running logo detection on {}x{} image",
            image.width(),
            image.height()
        );

        // Preprocess image to YOLOv8 input format
        let input_array = Self::preprocess_image_static(image, config)?;

        // Run inference
        let outputs = Self::run_inference_static(session, &input_array)?;

        // Post-process outputs to detections
        let detections =
            Self::postprocess_outputs_static(outputs, config, class_names, num_classes)?;

        info!("Detected {} logos", detections.len());

        Ok(detections)
    }

    /// Detect logos in a single image
    pub fn detect(&mut self, image: &RgbImage) -> Result<Vec<Detection>, LogoDetectionError> {
        Self::detect_with_session(
            &mut self.session,
            image,
            &self.config,
            &self.class_names,
            self.num_classes,
        )
    }

    /// Detect logos in multiple images (batch processing)
    pub fn detect_batch(
        &mut self,
        images: &[RgbImage],
    ) -> Result<Vec<Vec<Detection>>, LogoDetectionError> {
        let mut results = Vec::with_capacity(images.len());
        for img in images {
            results.push(self.detect(img)?);
        }
        Ok(results)
    }

    /// Preprocess image to YOLOv8 input format (1, 3, H, W) with normalization (static version)
    fn preprocess_image_static(
        image: &RgbImage,
        config: &LogoDetectionConfig,
    ) -> Result<Array<f32, ndarray::Dim<[usize; 4]>>, LogoDetectionError> {
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
    ) -> Result<SessionOutputs<'a>, LogoDetectionError> {
        // Zero-copy tensor: use view instead of clone
        let input_tensor = TensorRef::from_array_view(input.view())
            .map_err(|e| LogoDetectionError::Inference(e.to_string()))?;

        let outputs = session
            .run(ort::inputs![input_tensor])
            .map_err(|e| LogoDetectionError::Inference(e.to_string()))?;

        Ok(outputs)
    }

    /// Post-process ONNX outputs to detections
    fn postprocess_outputs_static(
        outputs: SessionOutputs,
        config: &LogoDetectionConfig,
        class_names: &[String],
        num_classes: usize,
    ) -> Result<Vec<Detection>, LogoDetectionError> {
        // YOLOv8 output shape: (1, 4+num_classes, 8400) or (batch, 4+classes, anchors)
        // First 4 values are [x_center, y_center, width, height]
        // Remaining num_classes values are class probabilities

        let output = &outputs[0];

        // Extract tensor data - returns (shape, data slice)
        let (shape, data) = output
            .try_extract_tensor::<f32>()
            .map_err(|e| LogoDetectionError::Inference(format!("Failed to extract tensor: {e}")))?;

        debug!("ONNX output shape: {:?}", shape);

        // Convert shape to dimensions
        let dims = shape.as_ref();
        if dims.len() != 3 {
            return Err(LogoDetectionError::Inference(format!(
                "Expected 3D output tensor, got {}D",
                dims.len()
            )));
        }

        let _batch_size = dims[0] as usize;
        let num_features = dims[1] as usize; // 4 box coords + num_classes
        let num_anchors = dims[2] as usize; // typically 8400

        // Validate feature count matches model
        let expected_features = 4 + num_classes;
        if num_features != expected_features {
            return Err(LogoDetectionError::Inference(format!(
                "Expected {} features (4 box coords + {} classes), got {}",
                expected_features, num_classes, num_features
            )));
        }

        // Pre-allocate for ~10% of anchors passing confidence threshold (8400 anchors → ~840 detections)
        let mut raw_detections = Vec::with_capacity(num_anchors / 10);

        // Process each anchor
        // Data layout: [batch, features, anchors] = [1, 4+num_classes, 8400]
        for anchor_idx in 0..num_anchors {
            // Helper function to get feature value for this anchor
            let get_feature = |feature_idx: usize| data[feature_idx * num_anchors + anchor_idx];

            // Extract box coordinates (center format) - features 0-3
            let x_center = get_feature(0);
            let y_center = get_feature(1);
            let width = get_feature(2);
            let height = get_feature(3);

            // Find class with highest probability - features 4..(4+num_classes)
            let mut max_prob = 0.0f32;
            let mut max_class_id = 0usize;

            for class_id in 0..num_classes {
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
                if !classes.contains(&(max_class_id as u32)) {
                    continue;
                }
            }

            // Convert from center format to corner format and normalize
            let x = (x_center - width / 2.0) / config.input_size as f32;
            let y = (y_center - height / 2.0) / config.input_size as f32;
            let w = width / config.input_size as f32;
            let h = height / config.input_size as f32;

            let bbox = BoundingBox::new(x, y, w, h);

            // Get class name, fallback to "unknown_logo" if out of bounds
            let class_name = class_names
                .get(max_class_id)
                .cloned()
                .unwrap_or_else(|| format!("unknown_logo_{}", max_class_id));

            raw_detections.push(Detection {
                class_id: max_class_id as u32,
                class_name,
                confidence,
                bbox,
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
        config: &LogoDetectionConfig,
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

    /// Get class name from class ID
    #[must_use]
    pub fn get_class_name(&self, class_id: u32) -> Option<&str> {
        self.class_names.get(class_id as usize).map(|s| s.as_str())
    }

    /// Get number of classes
    #[must_use]
    pub fn num_classes(&self) -> usize {
        self.num_classes
    }
}

/// Error types for logo detection
#[derive(Debug, Error)]
pub enum LogoDetectionError {
    #[error("Failed to load model: {0}")]
    ModelLoad(String),

    #[error("Failed to load class names: {0}")]
    ClassNamesLoad(String),

    #[error("Inference error: {0}")]
    Inference(String),

    #[error("Image processing error: {0}")]
    ImageProcessing(String),

    #[error("ONNX Runtime error: {0}")]
    OnnxRuntime(#[from] ort::Error),
}

impl From<LogoDetectionError> for ProcessingError {
    fn from(err: LogoDetectionError) -> Self {
        ProcessingError::Other(err.to_string())
    }
}

// Re-export CLIP-based types for convenience
pub use lib_clip::{
    BoundingBox as ClipBoundingBox, ClipLogoConfig, ClipLogoDetection, ClipLogoDetector,
};
