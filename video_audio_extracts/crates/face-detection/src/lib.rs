//! Face detection module using `RetinaFace` via ONNX Runtime
//!
//! This module provides face detection capabilities using `RetinaFace` models
//! exported to ONNX format. It detects faces and extracts 5-point facial landmarks.
//!
//! # Features
//! - Multiple `RetinaFace` model sizes (`MobileNet`, `ResNet50`)
//! - 5-point facial landmarks (eyes, nose, mouth corners)
//! - Configurable confidence and NMS thresholds
//! - Hardware acceleration via ONNX Runtime (CUDA, `TensorRT`, `CoreML`)
//!
//! # Example
//! ```no_run
//! use video_audio_face_detection::{FaceDetector, FaceDetectionConfig};
//! use image::open;
//!
//! # fn main() -> anyhow::Result<()> {
//! let config = FaceDetectionConfig::default();
//! let mut detector = FaceDetector::new("retinaface_mnet025.onnx", config)?;
//!
//! let img = open("image.jpg")?.to_rgb8();
//! let faces = detector.detect(&img)?;
//!
//! for face in faces {
//!     println!("Face: {:.2}% at ({:.0}, {:.0})",
//!              face.confidence * 100.0,
//!              face.bbox.x1, face.bbox.y1);
//! }
//! # Ok(())
//! # }
//! ```

pub mod anchors;
pub mod plugin;

use image::RgbImage;
use ndarray::Array4;
use ort::{
    session::{Session, SessionOutputs},
    value::TensorRef,
};
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;
use tracing::{debug, info};
use video_audio_common::ProcessingError;

/// `RetinaFace` model variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RetinaFaceModel {
    /// `MobileNet` 0.25 backbone - 1.7MB model, fastest inference
    MobileNet025,
    /// `MobileNet` 0.5 backbone - 3.2MB model, balanced
    MobileNet050,
    /// `ResNet50` backbone - 105MB model, highest accuracy
    ResNet50,
}

impl RetinaFaceModel {
    /// Get the typical model filename for this variant
    #[must_use]
    pub fn filename(&self) -> &'static str {
        match self {
            RetinaFaceModel::MobileNet025 => "retinaface_mnet025.onnx",
            RetinaFaceModel::MobileNet050 => "retinaface_mnet050.onnx",
            RetinaFaceModel::ResNet50 => "retinaface_resnet50.onnx",
        }
    }

    /// Get approximate model size in bytes
    #[must_use]
    pub fn size_bytes(&self) -> usize {
        match self {
            RetinaFaceModel::MobileNet025 => 1_700_000,
            RetinaFaceModel::MobileNet050 => 3_200_000,
            RetinaFaceModel::ResNet50 => 105_000_000,
        }
    }
}

/// Configuration for face detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceDetectionConfig {
    /// Minimum confidence threshold for face detections (0.0-1.0)
    pub confidence_threshold: f32,
    /// `IoU` threshold for non-maximum suppression (0.0-1.0)
    pub nms_threshold: f32,
    /// Whether to detect 5-point facial landmarks
    pub detect_landmarks: bool,
    /// Input image size (`RetinaFace` default varies by model)
    pub input_size: (u32, u32),
    /// Minimum box size as fraction of image (e.g., 0.02 = 2%)
    pub min_box_size: f32,
    /// Reject detections within this margin of edges (e.g., 0.05 = 5%)
    pub edge_margin: f32,
}

impl Default for FaceDetectionConfig {
    fn default() -> Self {
        Self {
            // N=184: Raised threshold from 0.35 to 0.50 to reduce false positives
            confidence_threshold: 0.50, // UltraFace threshold (adjusted for softmax output)
            // N=188: Lowered NMS threshold from 0.4 to 0.25 to better suppress duplicate detections
            // With anchor decoding, we get multiple overlapping detections per face that need stricter suppression
            // Initial attempt with 0.3 was not strict enough for some images (e.g., biden.jpg)
            nms_threshold: 0.25,
            detect_landmarks: false, // UltraFace model doesn't output landmarks
            input_size: (320, 240),  // UltraFace RFB-320 model size
            min_box_size: 0.01,      // Reject boxes smaller than 1% of image dimensions
            edge_margin: 0.01,       // Reject faces very close to edges (1% margin)
        }
    }
}

/// Bounding box for face detection
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoundingBox {
    /// Left x coordinate (normalized 0.0-1.0)
    pub x1: f32,
    /// Top y coordinate (normalized 0.0-1.0)
    pub y1: f32,
    /// Right x coordinate (normalized 0.0-1.0)
    pub x2: f32,
    /// Bottom y coordinate (normalized 0.0-1.0)
    pub y2: f32,
}

impl BoundingBox {
    /// Calculate box width
    #[must_use]
    #[inline]
    pub fn width(&self) -> f32 {
        self.x2 - self.x1
    }

    /// Calculate box height
    #[must_use]
    #[inline]
    pub fn height(&self) -> f32 {
        self.y2 - self.y1
    }

    /// Calculate box area
    #[must_use]
    #[inline]
    pub fn area(&self) -> f32 {
        self.width() * self.height()
    }

    /// Calculate `IoU` (Intersection over Union) with another box
    #[must_use]
    #[inline]
    pub fn iou(&self, other: &BoundingBox) -> f32 {
        let x1 = self.x1.max(other.x1);
        let y1 = self.y1.max(other.y1);
        let x2 = self.x2.min(other.x2);
        let y2 = self.y2.min(other.y2);

        if x2 < x1 || y2 < y1 {
            return 0.0;
        }

        let intersection = (x2 - x1) * (y2 - y1);
        let union = self.area() + other.area() - intersection;

        intersection / union
    }
}

/// 5-point facial landmarks
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FacialLandmarks {
    /// Left eye center (normalized 0.0-1.0)
    pub left_eye: (f32, f32),
    /// Right eye center (normalized 0.0-1.0)
    pub right_eye: (f32, f32),
    /// Nose tip (normalized 0.0-1.0)
    pub nose: (f32, f32),
    /// Left mouth corner (normalized 0.0-1.0)
    pub left_mouth: (f32, f32),
    /// Right mouth corner (normalized 0.0-1.0)
    pub right_mouth: (f32, f32),
}

/// Detected face with bounding box and optional landmarks
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Face {
    /// Detection confidence score (0.0-1.0)
    pub confidence: f32,
    /// Face bounding box (normalized coordinates)
    pub bbox: BoundingBox,
    /// 5-point facial landmarks (if requested in config)
    pub landmarks: Option<FacialLandmarks>,
}

/// Errors that can occur during face detection
#[derive(Error, Debug)]
pub enum FaceDetectionError {
    #[error("Failed to load ONNX model: {0}")]
    ModelLoadError(String),

    #[error("Failed to run inference: {0}")]
    InferenceError(String),

    #[error("Invalid image dimensions: {0}")]
    InvalidImageDimensions(String),

    #[error("Preprocessing failed: {0}")]
    PreprocessingError(String),

    #[error("Postprocessing failed: {0}")]
    PostprocessingError(String),

    #[error("Processing error: {0}")]
    ProcessingError(#[from] ProcessingError),
}

/// Face detector using `RetinaFace` via ONNX Runtime
pub struct FaceDetector {
    session: Session,
    config: FaceDetectionConfig,
    input_width: u32,
    input_height: u32,
    priors: Vec<anchors::PriorBox>, // Prior/anchor boxes for decoding
}

impl FaceDetector {
    /// Create a new face detector from an ONNX model file
    ///
    /// # Arguments
    /// * `model_path` - Path to the `RetinaFace` ONNX model file
    /// * `config` - Face detection configuration
    ///
    /// # Returns
    /// * `Result<Self>` - Face detector instance or error
    pub fn new<P: AsRef<Path>>(
        model_path: P,
        config: FaceDetectionConfig,
    ) -> Result<Self, FaceDetectionError> {
        let model_path = model_path.as_ref();

        info!("Loading RetinaFace model from {:?}", model_path);

        let session = Session::builder()
            .map_err(|e| FaceDetectionError::ModelLoadError(e.to_string()))?
            .commit_from_file(model_path)
            .map_err(|e| FaceDetectionError::ModelLoadError(e.to_string()))?;

        let (input_width, input_height) = config.input_size;

        // Generate prior boxes for UltraFace-320 model
        // TODO: Support other input sizes if needed
        let priors = if input_width == 320 && input_height == 240 {
            anchors::generate_ultraface_320_priors()
        } else {
            // For now, only 320x240 is supported
            // If other sizes are needed, extend anchors.rs with additional generators
            return Err(FaceDetectionError::ModelLoadError(format!(
                "Unsupported input size: {}x{}. Only 320x240 is currently supported.",
                input_width, input_height
            )));
        };

        info!(
            "RetinaFace model loaded successfully (input size: {}x{}, priors: {})",
            input_width,
            input_height,
            priors.len()
        );

        Ok(Self {
            session,
            config,
            input_width,
            input_height,
            priors,
        })
    }

    /// Detect faces using a pre-loaded ONNX session (for model caching)
    ///
    /// # Arguments
    /// * `session` - Pre-loaded ONNX Session (mutable reference needed for run())
    /// * `image` - RGB image to process
    /// * `config` - Face detection configuration
    /// * `input_width` - Model input width
    /// * `input_height` - Model input height
    pub fn detect_with_session(
        session: &mut Session,
        image: &RgbImage,
        config: &FaceDetectionConfig,
        input_width: u32,
        input_height: u32,
    ) -> Result<Vec<Face>, FaceDetectionError> {
        let (orig_width, orig_height) = image.dimensions();

        debug!("Detecting faces in {}x{} image", orig_width, orig_height);

        // Generate priors for the input size
        // TODO: Cache priors to avoid regeneration on each call
        let priors = if input_width == 320 && input_height == 240 {
            anchors::generate_ultraface_320_priors()
        } else {
            return Err(FaceDetectionError::PostprocessingError(format!(
                "Unsupported input size: {}x{}. Only 320x240 is currently supported.",
                input_width, input_height
            )));
        };

        // Preprocess image
        let input_array = Self::preprocess_image_static(image, input_width, input_height)?;

        // Run inference
        let outputs = Self::run_inference_static(session, &input_array)?;

        // Postprocess outputs
        let mut faces = Self::postprocess_outputs_static(outputs, config, orig_width, orig_height, &priors)?;

        // Apply NMS to remove duplicate detections
        faces = Self::non_maximum_suppression_static(faces, config);

        debug!("Detected {} faces after NMS", faces.len());

        Ok(faces)
    }

    /// Detect faces in an RGB image
    ///
    /// # Arguments
    /// * `image` - RGB image to process
    ///
    /// # Returns
    /// * `Result<Vec<Face>>` - Detected faces or error
    pub fn detect(&mut self, image: &RgbImage) -> Result<Vec<Face>, FaceDetectionError> {
        Self::detect_with_session(
            &mut self.session,
            image,
            &self.config,
            self.input_width,
            self.input_height,
        )
    }

    /// Run ONNX inference (static version for model caching)
    fn run_inference_static<'a>(
        session: &'a mut Session,
        input: &Array4<f32>,
    ) -> Result<SessionOutputs<'a>, FaceDetectionError> {
        let input_tensor = TensorRef::from_array_view(input.view())
            .map_err(|e| FaceDetectionError::InferenceError(e.to_string()))?;

        let outputs = session
            .run(ort::inputs![input_tensor])
            .map_err(|e| FaceDetectionError::InferenceError(e.to_string()))?;

        Ok(outputs)
    }

    /// Run ONNX inference on preprocessed input
    #[allow(dead_code)]
    fn run_inference(
        &mut self,
        input: &Array4<f32>,
    ) -> Result<SessionOutputs<'_>, FaceDetectionError> {
        Self::run_inference_static(&mut self.session, input)
    }

    /// Preprocess image for `RetinaFace` inference (static version for model caching)
    fn preprocess_image_static(
        image: &RgbImage,
        input_width: u32,
        input_height: u32,
    ) -> Result<Array4<f32>, FaceDetectionError> {
        // Resize image to model input size
        let resized = image::imageops::resize(
            image,
            input_width,
            input_height,
            image::imageops::FilterType::Triangle,
        );

        // Convert to CHW format with normalization
        // UltraFace expects RGB format with mean=127 and scale=128
        // Formula: (pixel - 127) / 128
        let mut input = Array4::<f32>::zeros((1, 3, input_height as usize, input_width as usize));

        for y in 0..input_height as usize {
            for x in 0..input_width as usize {
                let pixel = resized.get_pixel(x as u32, y as u32);
                // UltraFace normalization: (pixel - 127) / 128
                input[[0, 0, y, x]] = (f32::from(pixel[0]) - 127.0) / 128.0; // R
                input[[0, 1, y, x]] = (f32::from(pixel[1]) - 127.0) / 128.0; // G
                input[[0, 2, y, x]] = (f32::from(pixel[2]) - 127.0) / 128.0; // B
            }
        }

        Ok(input)
    }

    /// Preprocess image for `RetinaFace` inference
    #[allow(dead_code)]
    fn preprocess_image(&self, image: &RgbImage) -> Result<Array4<f32>, FaceDetectionError> {
        Self::preprocess_image_static(image, self.input_width, self.input_height)
    }

    /// Postprocess UltraFace/RetinaFace outputs to extract faces
    ///
    /// Model outputs:
    /// - scores: [1, N, 2] - N anchors with [`background_score`, `face_score`]
    /// - boxes: [1, N, 4] - N bounding boxes [x1, y1, x2, y2] normalized
    ///
    /// Note: This implementation works with `UltraFace` RFB-320 model
    fn postprocess_outputs_static(
        outputs: SessionOutputs,
        config: &FaceDetectionConfig,
        _orig_width: u32,
        _orig_height: u32,
        priors: &[anchors::PriorBox],
    ) -> Result<Vec<Face>, FaceDetectionError> {
        // Extract scores tensor [1, N, 2]
        // UltraFace RFB-320 outputs this as "confidences" instead of "scores"
        let scores_value = outputs
            .get("confidences")
            .or_else(|| outputs.get("scores"))
            .ok_or_else(|| {
                FaceDetectionError::PostprocessingError(
                    "confidences/scores output not found".into(),
                )
            })?;

        let (scores_shape, scores_data) =
            scores_value.try_extract_tensor::<f32>().map_err(|e| {
                FaceDetectionError::PostprocessingError(format!("Failed to extract scores: {e}"))
            })?;

        // Extract boxes tensor [1, N, 4]
        let boxes_value = outputs.get("boxes").ok_or_else(|| {
            FaceDetectionError::PostprocessingError("boxes output not found".into())
        })?;

        let (boxes_shape, boxes_data) = boxes_value.try_extract_tensor::<f32>().map_err(|e| {
            FaceDetectionError::PostprocessingError(format!("Failed to extract boxes: {e}"))
        })?;

        // Validate shapes
        if scores_shape.len() != 3 || boxes_shape.len() != 3 {
            return Err(FaceDetectionError::PostprocessingError(format!(
                "Invalid output shapes: scores={scores_shape:?}, boxes={boxes_shape:?}"
            )));
        }

        // UltraFace outputs confidences as [1, num_detections, num_classes]
        // where num_classes = 2 (background, face)
        let num_boxes = scores_shape[1] as usize;
        let _num_classes = scores_shape[2] as usize;

        if boxes_shape[1] as usize != num_boxes {
            return Err(FaceDetectionError::PostprocessingError(format!(
                "Mismatch between scores and boxes: scores[1]={} vs boxes[1]={}",
                num_boxes, boxes_shape[1]
            )));
        }

        // Verify prior count matches model output
        if priors.len() != num_boxes {
            return Err(FaceDetectionError::PostprocessingError(format!(
                "Prior count mismatch: expected {} priors, model outputs {} boxes",
                priors.len(),
                num_boxes
            )));
        }

        // Decode box regression outputs using prior boxes
        // UltraFace variance parameters (from reference implementation)
        const CENTER_VARIANCE: f32 = 0.1;
        const SIZE_VARIANCE: f32 = 0.2;

        debug!(
            "Boxes tensor shape: {:?}, num elements: {}, priors: {}",
            boxes_shape,
            boxes_data.len(),
            priors.len()
        );

        // boxes_data is [1, N, 4] - already flattened to N*4 elements
        // boxes_data is already &[f32], no need for as_slice()
        let decoded_boxes = anchors::decode_boxes(
            &boxes_data,
            priors,
            CENTER_VARIANCE,
            SIZE_VARIANCE,
        );

        // Extract faces above confidence threshold
        // Pre-allocate: estimate half of boxes will pass threshold
        let mut faces = Vec::with_capacity(num_boxes / 2);

        for i in 0..num_boxes {
            // Get face confidence from class 1 (face class)
            // Data layout: [box0_bg, box0_face, box1_bg, box1_face, ...]
            // So face confidence is at index i * 2 + 1
            //
            // UltraFace outputs raw logits, not probabilities.
            // Apply softmax: p_face = exp(face_logit) / (exp(bg_logit) + exp(face_logit))
            let bg_logit = scores_data[i * 2];
            let face_logit = scores_data[i * 2 + 1];
            let exp_bg = bg_logit.exp();
            let exp_face = face_logit.exp();
            let face_conf = exp_face / (exp_bg + exp_face);

            if face_conf >= config.confidence_threshold {
                // Extract decoded box coordinates [x1, y1, x2, y2]
                let bbox = decoded_boxes[i];
                let x1 = bbox[0];
                let y1 = bbox[1];
                let x2 = bbox[2];
                let y2 = bbox[3];

                // Clamp to valid range [0, 1]
                let x1 = x1.clamp(0.0, 1.0);
                let y1 = y1.clamp(0.0, 1.0);
                let x2 = x2.clamp(0.0, 1.0);
                let y2 = y2.clamp(0.0, 1.0);

                // Skip invalid boxes
                if x2 <= x1 || y2 <= y1 {
                    continue;
                }

                faces.push(Face {
                    confidence: face_conf,
                    bbox: BoundingBox { x1, y1, x2, y2 },
                    landmarks: None, // UltraFace model doesn't output landmarks
                });
            }
        }

        debug!(
            "Found {} faces before filtering (threshold: {})",
            faces.len(),
            config.confidence_threshold
        );

        // Apply minimum box size filter
        faces.retain(|face| {
            let width = face.bbox.width();
            let height = face.bbox.height();
            width >= config.min_box_size && height >= config.min_box_size
        });

        debug!(
            "Retained {} faces after min box size filter (min_size: {})",
            faces.len(),
            config.min_box_size
        );

        // Apply edge margin filter
        faces.retain(|face| {
            face.bbox.x1 > config.edge_margin
                && face.bbox.y1 > config.edge_margin
                && face.bbox.x2 < (1.0 - config.edge_margin)
                && face.bbox.y2 < (1.0 - config.edge_margin)
        });

        debug!(
            "Retained {} faces after edge filter (edge_margin: {})",
            faces.len(),
            config.edge_margin
        );

        Ok(faces)
    }

    /// Apply non-maximum suppression to remove overlapping detections (static version)
    fn non_maximum_suppression_static(
        mut faces: Vec<Face>,
        config: &FaceDetectionConfig,
    ) -> Vec<Face> {
        if faces.is_empty() {
            return faces;
        }

        debug!("NMS starting with {} faces (threshold: {})", faces.len(), config.nms_threshold);

        // Sort by confidence descending
        faces.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Pre-allocate: NMS typically keeps 20-50% of faces
        let mut keep = Vec::with_capacity(faces.len() / 3);
        let mut suppressed = vec![false; faces.len()];

        for i in 0..faces.len() {
            if suppressed[i] {
                continue;
            }

            keep.push(faces[i].clone());

            for j in (i + 1)..faces.len() {
                if suppressed[j] {
                    continue;
                }

                let iou = faces[i].bbox.iou(&faces[j].bbox);
                if iou > config.nms_threshold {
                    suppressed[j] = true;
                }
            }
        }

        debug!("NMS complete - kept {} faces", keep.len());
        keep
    }

    /// Apply non-maximum suppression to remove overlapping detections
    #[allow(dead_code)]
    fn non_maximum_suppression(&self, faces: Vec<Face>) -> Vec<Face> {
        Self::non_maximum_suppression_static(faces, &self.config)
    }
}

/// Convenience function to detect faces in multiple images
///
/// # Arguments
/// * `images` - Slice of RGB images to process
/// * `config` - Face detection configuration
/// * `model_path` - Path to `RetinaFace` ONNX model
///
/// # Returns
/// * `Result<Vec<Vec<Face>>>` - Detected faces per image or error
pub fn detect_faces<P: AsRef<Path>>(
    images: &[RgbImage],
    config: FaceDetectionConfig,
    model_path: P,
) -> Result<Vec<Vec<Face>>, FaceDetectionError> {
    let mut detector = FaceDetector::new(model_path, config)?;

    let mut results = Vec::with_capacity(images.len());
    for img in images {
        results.push(detector.detect(img)?);
    }
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bounding_box_calculations() {
        let bbox = BoundingBox {
            x1: 0.2,
            y1: 0.3,
            x2: 0.6,
            y2: 0.7,
        };

        assert!((bbox.width() - 0.4).abs() < 0.001);
        assert!((bbox.height() - 0.4).abs() < 0.001);
        assert!((bbox.area() - 0.16).abs() < 0.001);
    }

    #[test]
    fn test_bounding_box_iou() {
        let bbox1 = BoundingBox {
            x1: 0.0,
            y1: 0.0,
            x2: 0.5,
            y2: 0.5,
        };

        let bbox2 = BoundingBox {
            x1: 0.25,
            y1: 0.25,
            x2: 0.75,
            y2: 0.75,
        };

        let iou = bbox1.iou(&bbox2);
        // Intersection: 0.25 * 0.25 = 0.0625
        // Union: 0.25 + 0.25 - 0.0625 = 0.4375
        // IoU: 0.0625 / 0.4375 ≈ 0.1428
        assert!((iou - 0.1428).abs() < 0.001);
    }

    #[test]
    fn test_config_defaults() {
        let config = FaceDetectionConfig::default();
        assert_eq!(config.confidence_threshold, 0.85);
        assert_eq!(config.nms_threshold, 0.4);
        assert!(!config.detect_landmarks); // UltraFace doesn't output landmarks
        assert_eq!(config.input_size, (320, 240)); // UltraFace RFB-320
        assert_eq!(config.min_box_size, 0.03);
        assert_eq!(config.edge_margin, 0.10);
    }

    #[test]
    fn test_model_filenames() {
        assert_eq!(
            RetinaFaceModel::MobileNet025.filename(),
            "retinaface_mnet025.onnx"
        );
        assert_eq!(
            RetinaFaceModel::MobileNet050.filename(),
            "retinaface_mnet050.onnx"
        );
        assert_eq!(
            RetinaFaceModel::ResNet50.filename(),
            "retinaface_resnet50.onnx"
        );
    }

    #[test]
    #[ignore] // Run manually with: cargo test --package video-audio-face-detection inspect_model_outputs -- --ignored --nocapture
    fn inspect_model_outputs() {
        use image::RgbImage;

        let model_path = std::env::var("CARGO_MANIFEST_DIR")
            .map(|dir| format!("{dir}/../../models/face-detection/retinaface_mnet025.onnx"))
            .unwrap_or_else(|_| "models/face-detection/retinaface_mnet025.onnx".to_string());

        if !std::path::Path::new(&model_path).exists() {
            println!("Model not found at {model_path}, skipping");
            return;
        }

        let config = FaceDetectionConfig {
            input_size: (320, 240), // UltraFace model expects 320x240
            ..Default::default()
        };
        let mut detector = FaceDetector::new(&model_path, config).expect("Failed to load model");

        println!("\n=== RetinaFace Model Inspection ===");
        println!("Model path: {model_path}");

        // Print input metadata
        println!("\nInputs:");
        for (i, input) in detector.session.inputs.iter().enumerate() {
            println!("  [{}] name: {}", i, input.name);
            println!("      input_type: {:?}", input.input_type);
        }

        // Print output metadata
        println!("\nOutputs:");
        for (i, output) in detector.session.outputs.iter().enumerate() {
            println!("  [{}] name: {}", i, output.name);
            println!("      output_type: {:?}", output.output_type);
        }

        // Run dummy inference with blank 320x240 image (model's expected size)
        println!("\n=== Running Dummy Inference ===");
        let blank_img = RgbImage::new(320, 240);
        let input_array = detector
            .preprocess_image(&blank_img)
            .expect("Preprocess failed");

        println!("Input shape: {:?}", input_array.shape());

        let outputs = detector
            .run_inference(&input_array)
            .expect("Inference failed");

        println!("\nOutput tensors:");
        for (i, (name, tensor)) in outputs.iter().enumerate() {
            println!("  [{i}] name: {name}");

            // Try to extract tensor and print shape
            if let Ok((shape, data)) = tensor.try_extract_tensor::<f32>() {
                println!("      shape: {shape:?}");
                println!("      dtype: f32");

                // Print first few values if tensor is small enough
                let total_elements: usize = shape.iter().map(|&x| x as usize).product();
                if total_elements <= 20 {
                    println!("      values: {data:?}");
                } else {
                    println!("      first 10 values: {:?}", &data[..10.min(data.len())]);
                    println!(
                        "      last 10 values: {:?}",
                        &data[data.len().saturating_sub(10)..]
                    );
                }
            }
        }

        println!("\n=== Inspection Complete ===");
    }

    #[test]
    #[ignore] // Run manually: cargo test --package video-audio-face-detection test_detection_with_real_image -- --ignored --nocapture
    fn test_detection_with_real_image() {
        let model_path = std::env::var("CARGO_MANIFEST_DIR")
            .map(|dir| format!("{dir}/../../models/face-detection/retinaface_mnet025.onnx"))
            .unwrap_or_else(|_| "models/face-detection/retinaface_mnet025.onnx".to_string());

        if !std::path::Path::new(&model_path).exists() {
            println!("Model not found at {model_path}, skipping");
            return;
        }

        println!("\n=== Testing Face Detection with Real Image ===");

        let config = FaceDetectionConfig::default();
        let mut detector = FaceDetector::new(&model_path, config).expect("Failed to load model");

        // Test with actual Kinetics-600 video frame if available
        let test_image_path = "/tmp/test_face_frame.jpg";

        if std::path::Path::new(test_image_path).exists() {
            println!("Loading test image from {test_image_path}");
            let img = image::open(test_image_path)
                .expect("Failed to load test image")
                .to_rgb8();

            println!("Image size: {}x{}", img.width(), img.height());
            let faces = detector.detect(&img).expect("Detection failed");

            println!("Detected {} faces", faces.len());
            for (i, face) in faces.iter().enumerate().take(10) {
                println!(
                    "  Face {}: confidence={:.3}, bbox=({:.3}, {:.3}, {:.3}, {:.3})",
                    i, face.confidence, face.bbox.x1, face.bbox.y1, face.bbox.x2, face.bbox.y2
                );
            }

            // Expect at least 1 face in "talking on cell phone" video
            assert!(
                !faces.is_empty(),
                "Expected at least 1 face in talking video"
            );

            // Check that at least one face has high confidence
            let max_conf = faces.iter().map(|f| f.confidence).fold(0.0f32, f32::max);
            assert!(
                max_conf >= 0.7,
                "Expected at least one face with confidence >= 0.7"
            );

            println!("✓ Face detection works on real image!");
        } else {
            println!("Test image not found at {test_image_path}");
            println!("To create test image, run:");
            println!(
                "  ffmpeg -i \"<kinetics_video>\" -vf \"select=eq(n\\,30)\" -vframes 1 {test_image_path} -y"
            );
            println!("Skipping real image test");
        }
    }
}
