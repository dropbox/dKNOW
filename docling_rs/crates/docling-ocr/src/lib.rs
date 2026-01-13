//! Optical Character Recognition (OCR) support for docling-rs
//!
//! This crate provides OCR functionality using ONNX Runtime with `PaddleOCR` models.
//!
//! # Architecture
//!
//! The OCR pipeline consists of two stages:
//! 1. **Text Detection**: Finds bounding boxes of text regions in the image
//! 2. **Text Recognition**: Recognizes text within each detected region
//!
//! # Platform Support
//!
//! - **Linux**: ONNX Runtime with CPU or CUDA (if available)
//! - **macOS**: ONNX Runtime with `CoreML` execution provider (GPU acceleration)
//! - **Windows**: ONNX Runtime with CPU or `DirectML`
//!
//! # Models
//!
//! Uses `PaddleOCR` PP-OCRv4 models in ONNX format:
//! - Detection model: `det_mbnetv3.onnx` (~3-5 MB)
//! - Recognition model: `rec_crnn.onnx` (~5-10 MB)

use anyhow::Result;
use image::{DynamicImage, GenericImageView, GrayImage, Luma};
use imageproc::contours::find_contours;
use imageproc::morphology::dilate;
use ndarray::{Array3, Array4, ArrayView};
use ort::{
    inputs,
    session::{builder::GraphOptimizationLevel, Session},
    value::TensorRef,
};
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

// ============================================================================
// Image Normalization Constants
// ============================================================================

/// Maximum pixel value for 8-bit images (0-255 range)
const PIXEL_MAX_VALUE_F32: f32 = 255.0;

/// Normalization center for recognition model (maps pixel values to centered range)
/// Used in: (pixel / 255.0 - 0.5) / 0.5 to normalize to [-1, 1] range
const RECOGNITION_NORMALIZE_CENTER: f32 = 0.5;

/// Normalization scale for recognition model
/// Used in: (pixel / 255.0 - 0.5) / 0.5 to normalize to [-1, 1] range
const RECOGNITION_NORMALIZE_SCALE: f32 = 0.5;

/// OCR-specific errors
#[derive(Error, Debug)]
pub enum OcrError {
    /// Failed to load the OCR model from disk or memory
    #[error("Failed to load OCR model: {0}")]
    ModelLoadError(String),

    /// Error during OCR inference (forward pass)
    #[error("Failed to run OCR inference: {0}")]
    InferenceError(String),

    /// Image preprocessing failed (resizing, normalization, etc.)
    #[error("Image preprocessing failed: {0}")]
    PreprocessingError(String),

    /// Failed to decode model output to text
    #[error("Failed to decode OCR output: {0}")]
    DecodingError(String),

    /// Image dimensions are invalid (too small, too large, or zero)
    #[error("Invalid image dimensions: {0}x{1}")]
    InvalidDimensions(u32, u32),

    /// No text regions detected in the image
    #[error("No text detected in image")]
    NoTextDetected,
}

/// Bounding box for detected text region
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BoundingBox {
    /// Left coordinate (x)
    pub x: f32,
    /// Top coordinate (y)
    pub y: f32,
    /// Width
    pub width: f32,
    /// Height
    pub height: f32,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
}

impl BoundingBox {
    /// Create a new bounding box
    #[inline]
    #[must_use = "bounding box is created but not used"]
    pub const fn new(x: f32, y: f32, width: f32, height: f32, confidence: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
            confidence,
        }
    }

    /// Get the right edge coordinate
    #[inline]
    #[must_use = "right coordinate is computed but not used"]
    pub const fn right(&self) -> f32 {
        self.x + self.width
    }

    /// Get the bottom edge coordinate
    #[inline]
    #[must_use = "bottom coordinate is computed but not used"]
    pub const fn bottom(&self) -> f32 {
        self.y + self.height
    }

    /// Get the center point
    #[inline]
    #[must_use = "center coordinates are computed but not used"]
    pub const fn center(&self) -> (f32, f32) {
        (self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    /// Calculate area
    #[inline]
    #[must_use = "area is computed but not used"]
    pub const fn area(&self) -> f32 {
        self.width * self.height
    }
}

/// A single line of recognized text with its bounding box
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextLine {
    /// The recognized text content
    pub text: String,
    /// Bounding box of the text region
    pub bbox: BoundingBox,
    /// Recognition confidence score (0.0 to 1.0)
    pub confidence: f32,
}

impl TextLine {
    /// Create a new text line
    #[inline]
    #[must_use = "text line is created but not used"]
    pub const fn new(text: String, bbox: BoundingBox, confidence: f32) -> Self {
        Self {
            text,
            bbox,
            confidence,
        }
    }
}

/// Result of OCR processing on an image
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OcrResult {
    /// Detected text lines, sorted in reading order
    pub lines: Vec<TextLine>,
    /// Average confidence score across all lines
    pub avg_confidence: f32,
    /// Image dimensions (width, height)
    pub image_size: (u32, u32),
}

impl OcrResult {
    /// Create a new OCR result
    #[inline]
    #[must_use = "OCR result is created but not used"]
    pub fn new(lines: Vec<TextLine>, image_size: (u32, u32)) -> Self {
        // Precision loss acceptable: line count is small (OCR lines per page), well within f32 range
        #[allow(clippy::cast_precision_loss)]
        let avg_confidence = if lines.is_empty() {
            0.0
        } else {
            lines.iter().map(|l| l.confidence).sum::<f32>() / lines.len() as f32
        };

        Self {
            lines,
            avg_confidence,
            image_size,
        }
    }

    /// Get all text concatenated with newlines
    #[inline]
    #[must_use = "concatenated text is returned but not used"]
    pub fn text(&self) -> String {
        self.lines
            .iter()
            .map(|l| l.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Check if any text was detected
    #[inline]
    #[must_use = "emptiness check result is returned but not used"]
    pub const fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Get the number of detected text lines
    #[inline]
    #[must_use = "line count is returned but not used"]
    pub const fn len(&self) -> usize {
        self.lines.len()
    }
}

/// Main OCR engine using ONNX Runtime with `PaddleOCR` models
pub struct OcrEngine {
    /// Detection model session
    det_session: Session,
    /// Recognition model session
    rec_session: Option<Session>,
    /// Character dictionary for CTC decoding
    characters: Vec<String>,
    /// Detection preprocessing configuration
    det_config: DetectionConfig,
    /// Recognition preprocessing configuration
    rec_config: RecognitionConfig,
}

/// Configuration for text detection preprocessing
#[derive(Debug, Clone, PartialEq)]
struct DetectionConfig {
    /// Limit type: "min" or "max"
    limit_type: String,
    /// Limit side length
    limit_side_len: u32,
    /// Normalization mean values [R, G, B]
    mean: [f32; 3],
    /// Normalization std values [R, G, B]
    std: [f32; 3],
    /// Threshold for binary mask (default: 0.3)
    thresh: f32,
    /// Box confidence threshold (default: 0.5)
    box_thresh: f32,
    /// Unclip ratio for expanding text regions (default: 1.6)
    unclip_ratio: f32,
    /// Maximum number of candidates
    max_candidates: usize,
    /// Use dilation for mask
    use_dilation: bool,
}

/// Configuration for text recognition preprocessing
#[derive(Debug, Clone, PartialEq)]
struct RecognitionConfig {
    /// Image shape for recognition: [channels, height, width]
    /// Default: [3, 48, 320] (height is fixed, width is dynamic based on aspect ratio)
    rec_img_shape: [usize; 3],
    /// Batch size for recognition (number of regions to process at once)
    rec_batch_num: usize,
}

impl Default for DetectionConfig {
    #[inline]
    fn default() -> Self {
        Self {
            limit_type: "max".to_string(),
            limit_side_len: 960,
            mean: [0.485, 0.456, 0.406],
            std: [0.229, 0.224, 0.225],
            thresh: 0.3,
            box_thresh: 0.5,
            unclip_ratio: 1.6,
            max_candidates: 1000,
            use_dilation: true,
        }
    }
}

impl Default for RecognitionConfig {
    #[inline]
    fn default() -> Self {
        Self {
            // PP-OCRv4 default: height=48, max_width=320
            rec_img_shape: [3, 48, 320],
            rec_batch_num: 6,
        }
    }
}

impl OcrEngine {
    /// Create a new OCR engine with default embedded models
    ///
    /// Loads models from the assets directory:
    /// - `assets/det_model.onnx` - Text detection
    /// - `assets/rec_model.onnx` - Text recognition
    /// - `assets/ppocr_keys_v1.txt` - Character dictionary
    ///
    /// # Errors
    ///
    /// Returns an error if the assets directory cannot be found or if models fail to load.
    #[must_use = "this returns a Result that should be handled"]
    pub fn new() -> Result<Self> {
        // Try multiple locations for assets directory
        let assets_dir = Self::find_assets_dir()?;
        let det_path = assets_dir.join("det_model.onnx");
        let rec_path = assets_dir.join("rec_model.onnx");
        let dict_path = assets_dir.join("ppocr_keys_v1.txt");

        if !det_path.exists() {
            return Err(anyhow::anyhow!(
                "Detection model not found at {}. Please download models. See assets/README.md",
                det_path.display()
            ));
        }

        if !rec_path.exists() {
            return Err(anyhow::anyhow!(
                "Recognition model not found at {}. Please download models. See assets/README.md",
                rec_path.display()
            ));
        }

        if !dict_path.exists() {
            return Err(anyhow::anyhow!(
                "Character dictionary not found at {}. Please download models. See assets/README.md",
                dict_path.display()
            ));
        }

        Self::with_models(&det_path, Some(&rec_path), Some(&dict_path))
    }

    /// Find the assets directory by checking multiple possible locations
    fn find_assets_dir() -> Result<std::path::PathBuf> {
        // 1. Check environment variable override
        if let Ok(assets_env) = std::env::var("DOCLING_OCR_ASSETS") {
            let path = Path::new(&assets_env);
            if path.exists() && path.is_dir() {
                return Ok(path.to_path_buf());
            }
        }

        // 2. Check current manifest dir (works when building docling-ocr directly)
        if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
            let assets_dir = Path::new(&manifest_dir).join("assets");
            if assets_dir.exists() && assets_dir.is_dir() {
                return Ok(assets_dir);
            }

            // 3. Check if we're in a workspace member, try workspace root
            // CARGO_MANIFEST_DIR might be crates/docling-core when running integration tests
            let parent = Path::new(&manifest_dir).parent();
            if let Some(parent_dir) = parent {
                let workspace_assets = parent_dir.join("docling-ocr").join("assets");
                if workspace_assets.exists() && workspace_assets.is_dir() {
                    return Ok(workspace_assets);
                }

                // Try one level up (if in crates/foo, go to crates/docling-ocr/assets)
                if let Some(grandparent) = parent_dir.parent() {
                    let workspace_assets = grandparent
                        .join("crates")
                        .join("docling-ocr")
                        .join("assets");
                    if workspace_assets.exists() && workspace_assets.is_dir() {
                        return Ok(workspace_assets);
                    }
                }
            }
        }

        // 4. Check relative to current directory (fallback)
        let current_dir_assets = Path::new("crates/docling-ocr/assets");
        if current_dir_assets.exists() && current_dir_assets.is_dir() {
            return Ok(current_dir_assets.to_path_buf());
        }

        Err(anyhow::anyhow!(
            "Could not find OCR assets directory. Tried:\n\
             - DOCLING_OCR_ASSETS environment variable\n\
             - $CARGO_MANIFEST_DIR/assets\n\
             - workspace: ../docling-ocr/assets\n\
             - workspace: ../../crates/docling-ocr/assets\n\
             - relative: crates/docling-ocr/assets\n\
             Please set DOCLING_OCR_ASSETS or ensure models are in crates/docling-ocr/assets/"
        ))
    }

    /// Create a new OCR engine with custom model paths
    ///
    /// # Arguments
    /// * `detection_model` - Path to the detection ONNX model
    /// * `recognition_model` - Optional path to the recognition ONNX model
    /// * `character_dict` - Optional path to character dictionary file
    ///
    /// # Errors
    ///
    /// Returns an error if models fail to load or if the dictionary cannot be read.
    pub fn with_models(
        detection_model: &Path,
        recognition_model: Option<&Path>,
        character_dict: Option<&Path>,
    ) -> Result<Self> {
        // Load detection model (ort 2.0 API)
        let det_session = Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level1)?
            .with_intra_threads(4)?
            .commit_from_file(detection_model)
            .map_err(|e| anyhow::anyhow!("Failed to load detection model: {e}"))?;

        // Load recognition model (optional)
        let rec_session = if let Some(rec_path) = recognition_model {
            Some(
                Session::builder()?
                    .with_optimization_level(GraphOptimizationLevel::Level1)?
                    .with_intra_threads(4)?
                    .commit_from_file(rec_path)
                    .map_err(|e| anyhow::anyhow!("Failed to load recognition model: {e}"))?,
            )
        } else {
            None
        };

        // Load character dictionary
        // Ported from: rapidocr/ch_ppocr_rec/utils.py:CTCLabelDecode.get_character
        // Lines 41-63
        let characters = if let Some(dict_path) = character_dict {
            Self::load_character_dict(dict_path)?
        } else {
            Vec::new()
        };

        Ok(Self {
            det_session,
            rec_session,
            characters,
            det_config: DetectionConfig::default(),
            rec_config: RecognitionConfig::default(),
        })
    }

    /// Load character dictionary from file
    ///
    /// Ported from: `rapidocr/ch_ppocr_rec/utils.py:CTCLabelDecode.read_character_file`
    /// Lines 66-73, and `insert_special_char` lines 76-80
    fn load_character_dict(path: &Path) -> Result<Vec<String>> {
        use std::io::{BufRead, BufReader};

        let file = std::fs::File::open(path)
            .map_err(|e| anyhow::anyhow!("Failed to open character dictionary: {e}"))?;
        let reader = BufReader::new(file);

        let mut characters = Vec::new();

        // Insert "blank" token at position 0 (CTC blank token)
        // Python: utils.py line 62
        characters.push("blank".to_string());

        // Read character list
        // Python: utils.py lines 66-72
        for line in reader.lines() {
            let line = line?;
            let char = line.trim().to_string();
            if !char.is_empty() {
                characters.push(char);
            }
        }

        // Insert " " (space) token at end
        // Python: utils.py line 60
        characters.push(" ".to_string());

        Ok(characters)
    }

    /// Preprocess image for detection model
    ///
    /// Ported from: `rapidocr/ch_ppocr_det/utils.py:DetPreProcess`
    /// Lines 43-110
    // Precision loss acceptable: image dimensions are u32 (max ~4 billion pixels),
    // f32 mantissa (23 bits) handles values up to ~16 million exactly, and larger
    // values lose only sub-pixel precision which is irrelevant for image processing
    // Truncation is safe: image dimensions after resize fit in u32
    #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
    fn preprocess_detection(&self, image: &DynamicImage) -> Result<Array4<f32>> {
        let (width, height) = image.dimensions();
        let max_wh = width.max(height);

        // Determine limit_side_len based on image size
        // Python: main.py lines 68-77
        let limit_side_len = if self.det_config.limit_type == "min" {
            self.det_config.limit_side_len
        } else if max_wh < 960 {
            960
        } else if max_wh < 1500 {
            1500
        } else {
            2000
        };

        // Resize image to multiple of 32
        // Python: utils.py lines 76-110
        let (h, w) = (height as f32, width as f32);
        let ratio = if self.det_config.limit_type == "max" {
            if h.max(w) > limit_side_len as f32 {
                limit_side_len as f32 / h.max(w)
            } else {
                1.0
            }
        } else if h.min(w) < limit_side_len as f32 {
            limit_side_len as f32 / h.min(w)
        } else {
            1.0
        };

        // Sign loss is safe: h, w, ratio are always positive, so result is always positive
        #[allow(clippy::cast_sign_loss)]
        let resize_h = ((h * ratio).round() / 32.0).round() as u32 * 32;
        #[allow(clippy::cast_sign_loss)]
        let resize_w = ((w * ratio).round() / 32.0).round() as u32 * 32;

        if resize_w == 0 || resize_h == 0 {
            return Err(anyhow::anyhow!(
                "Invalid resize dimensions: {resize_w}x{resize_h}"
            ));
        }

        // Resize image
        let resized =
            image.resize_exact(resize_w, resize_h, image::imageops::FilterType::CatmullRom);

        // Convert to RGB
        let rgb_image = resized.to_rgb8();

        // Normalize: (pixel * scale - mean) / std
        // Python: utils.py lines 70-71
        let scale = 1.0 / PIXEL_MAX_VALUE_F32;
        let mean = &self.det_config.mean;
        let std = &self.det_config.std;

        let mut array = Array3::<f32>::zeros((3, resize_h as usize, resize_w as usize));

        for y in 0..resize_h {
            for x in 0..resize_w {
                let pixel = rgb_image.get_pixel(x, y);
                for c in 0..3 {
                    let normalized = (f32::from(pixel[c]) * scale - mean[c]) / std[c];
                    array[[c, y as usize, x as usize]] = normalized;
                }
            }
        }

        // Add batch dimension: (1, C, H, W)
        // Python: utils.py line 67
        let array_4d = array.insert_axis(ndarray::Axis(0));

        Ok(array_4d)
    }

    /// Preprocess image region for recognition model
    ///
    /// Ported from: `rapidocr/ch_ppocr_rec/main.py:resize_norm_img`
    /// Lines 148-169
    // Precision loss acceptable: image dimensions and config values are small integers,
    // well within f32's exact representation range (up to ~16 million)
    // Truncation safe: resized image dimensions fit in u32/usize
    #[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
    fn preprocess_recognition(
        &self,
        img: &DynamicImage,
        max_wh_ratio: f32,
    ) -> ndarray::Array3<f32> {
        let [img_channel, img_height, _img_width] = self.rec_config.rec_img_shape;

        // Calculate dynamic width based on aspect ratio
        // Python: main.py line 152
        // Sign loss safe: img_height and max_wh_ratio are always positive
        #[allow(clippy::cast_sign_loss)]
        let img_width = (img_height as f32 * max_wh_ratio) as usize;

        let (w, h) = img.dimensions();
        let ratio = w as f32 / h as f32;

        // Calculate resize width
        // Python: main.py lines 155-159
        // Sign loss safe: image dimensions and ratios are always positive
        #[allow(clippy::cast_sign_loss)]
        let resized_w = if (img_height as f32 * ratio).ceil() as usize > img_width {
            img_width
        } else {
            #[allow(clippy::cast_sign_loss)]
            {
                (img_height as f32 * ratio).ceil() as usize
            }
        };

        // Resize image to fixed height, dynamic width
        // Python: main.py line 161
        let resized_image = img.resize_exact(
            resized_w as u32,
            img_height as u32,
            image::imageops::FilterType::CatmullRom,
        );

        // Convert to RGB
        let rgb_image = resized_image.to_rgb8();

        // Normalize: pixel / 255, then (pixel - 0.5) / 0.5
        // Python: main.py lines 163-165
        let mut array = ndarray::Array3::<f32>::zeros((img_channel, img_height, resized_w));

        for y in 0..img_height {
            for x in 0..resized_w {
                let pixel = rgb_image.get_pixel(x as u32, y as u32);
                for c in 0..3 {
                    let normalized = (f32::from(pixel[c]) / PIXEL_MAX_VALUE_F32
                        - RECOGNITION_NORMALIZE_CENTER)
                        / RECOGNITION_NORMALIZE_SCALE;
                    array[[c, y, x]] = normalized;
                }
            }
        }

        // Pad to max width if needed
        // Python: main.py lines 167-168
        if resized_w < img_width {
            let mut padded = ndarray::Array3::<f32>::zeros((img_channel, img_height, img_width));
            padded
                .slice_mut(ndarray::s![.., .., ..resized_w])
                .assign(&array);
            return padded;
        }

        array
    }

    /// Run OCR on an image (full pipeline: detection + recognition)
    ///
    /// # Errors
    ///
    /// Returns an error if detection or recognition fails.
    #[must_use = "this function returns OCR results that should be processed"]
    pub fn recognize(&mut self, image: &DynamicImage) -> Result<OcrResult> {
        // Step 1: Detect text regions
        let boxes = self.detect(image)?;

        if boxes.is_empty() {
            return Ok(OcrResult::new(vec![], image.dimensions()));
        }

        // Step 2: Recognize text in each region
        let lines = self.recognize_regions(image, &boxes)?;

        Ok(OcrResult::new(lines, image.dimensions()))
    }

    /// Detect text regions in an image (detection stage only)
    ///
    /// # Errors
    ///
    /// Returns an error if preprocessing or inference fails.
    // Precision loss acceptable: image coordinates are small integers, well within f32 range
    #[allow(clippy::cast_precision_loss)]
    #[must_use = "this function returns detected bounding boxes that should be processed"]
    pub fn detect(&mut self, image: &DynamicImage) -> Result<Vec<BoundingBox>> {
        let ori_shape = image.dimensions();

        // Preprocess image
        let input_tensor = self.preprocess_detection(image)?;

        // Run detection model inference (ort 2.0 API)
        let input_ref: TensorRef<f32> = TensorRef::from_array_view(&input_tensor)?;
        let output_array = {
            let outputs = self
                .det_session
                .run(inputs![input_ref])
                .map_err(|e| anyhow::anyhow!("Detection inference failed: {e}"))?;

            // Extract output tensor (shape: [1, 1, H, W])
            let output_tensor = outputs[0].try_extract_array::<f32>()?;
            output_tensor
                .into_owned()
                .into_dimensionality::<ndarray::Ix4>()
                .map_err(|e| anyhow::anyhow!("Failed to reshape output: {e}"))?
        };

        // Postprocess: threshold, find contours, extract boxes
        let boxes = self.postprocess_detection(output_array.view(), ori_shape);

        Ok(boxes)
    }

    /// Postprocess detection model output
    ///
    /// Ported from: `rapidocr/ch_ppocr_det/utils.py:DBPostProcess`
    /// Lines 117-319
    ///
    /// Uses imageproc for contour detection instead of `OpenCV`
    // Truncation safe: image dimensions from predictions fit in u32
    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_precision_loss)] // Image coords are small integers, f32 precision is fine
    #[allow(clippy::too_many_lines)] // Complex OCR postprocessing - keeping together for clarity
    fn postprocess_detection(
        &self,
        pred: ArrayView<f32, ndarray::Ix4>,
        ori_shape: (u32, u32),
    ) -> Vec<BoundingBox> {
        let (src_w, src_h) = ori_shape;

        // Extract the prediction map (remove batch and channel dims)
        // Python: utils.py lines 143-144
        let pred_3d = pred.index_axis(ndarray::Axis(0), 0);
        let pred_2d = pred_3d.index_axis(ndarray::Axis(0), 0);

        // Apply threshold to create binary mask
        // Python: utils.py line 145
        let shape = pred_2d.shape();
        let (height, width) = (shape[0], shape[1]);

        let mut mask = GrayImage::new(width as u32, height as u32);
        for y in 0..height {
            for x in 0..width {
                let val = if pred_2d[[y, x]] > self.det_config.thresh {
                    255u8
                } else {
                    0u8
                };
                mask.put_pixel(x as u32, y as u32, Luma([val]));
            }
        }

        // Apply dilation if enabled
        // Python: utils.py lines 148-150
        if self.det_config.use_dilation {
            // Dilate with 1 pixel radius (equivalent to 2x2 kernel)
            mask = dilate(&mask, imageproc::distance_transform::Norm::L1, 1);
        }

        // Find contours
        // Python: utils.py lines 166-172
        // imageproc finds foreground (non-zero) pixels as contours
        let contours = find_contours::<u32>(&mask);

        let num_contours = contours.len().min(self.det_config.max_candidates);

        let mut boxes = Vec::new();

        // Process each contour
        // Python: utils.py lines 176-201
        for contour in contours.iter().take(num_contours) {
            // Skip empty contours
            if contour.points.is_empty() {
                continue;
            }

            // Get bounding rectangle and points
            // For now, use axis-aligned bounding box (simplified vs OpenCV's minAreaRect)
            let points: Vec<[f32; 2]> = contour
                .points
                .iter()
                .map(|p| [p.x as f32, p.y as f32])
                .collect();

            // Calculate axis-aligned bounding box
            let min_x = points.iter().map(|p| p[0]).fold(f32::INFINITY, f32::min);
            let max_x = points
                .iter()
                .map(|p| p[0])
                .fold(f32::NEG_INFINITY, f32::max);
            let min_y = points.iter().map(|p| p[1]).fold(f32::INFINITY, f32::min);
            let max_y = points
                .iter()
                .map(|p| p[1])
                .fold(f32::NEG_INFINITY, f32::max);

            let bbox_width = max_x - min_x;
            let bbox_height = max_y - min_y;
            let min_side = bbox_width.min(bbox_height);

            // Skip if too small
            // Python: utils.py lines 180-181
            if min_side < 3.0 {
                continue;
            }

            // Create 4-point box (corners of axis-aligned rect)
            let box_points = vec![
                [min_x, min_y], // top-left
                [max_x, min_y], // top-right
                [max_x, max_y], // bottom-right
                [min_x, max_y], // bottom-left
            ];

            // Calculate confidence score (fast mode)
            // Python: utils.py lines 183-189, 229-241
            let score = Self::box_score_fast(&pred_2d, &box_points);

            if score < self.det_config.box_thresh {
                continue;
            }

            // Unclip the box (expand it)
            // Python: utils.py lines 191-193, 262-269
            let expanded_points = self.unclip_box(&box_points);

            // Get bounding box of expanded polygon
            let exp_min_x = expanded_points
                .iter()
                .map(|p| p[0])
                .fold(f32::INFINITY, f32::min);
            let exp_max_x = expanded_points
                .iter()
                .map(|p| p[0])
                .fold(f32::NEG_INFINITY, f32::max);
            let exp_min_y = expanded_points
                .iter()
                .map(|p| p[1])
                .fold(f32::INFINITY, f32::min);
            let exp_max_y = expanded_points
                .iter()
                .map(|p| p[1])
                .fold(f32::NEG_INFINITY, f32::max);

            let exp_width = exp_max_x - exp_min_x;
            let exp_height = exp_max_y - exp_min_y;
            let exp_min_side = exp_width.min(exp_height);

            // Skip if still too small after expansion
            // Python: utils.py lines 193-194
            if exp_min_side < 5.0 {
                continue;
            }

            // Scale back to original image size
            // Python: utils.py lines 196-199
            let final_min_x =
                (exp_min_x / width as f32 * src_w as f32).clamp(0.0, (src_w - 1) as f32);
            let final_max_x =
                (exp_max_x / width as f32 * src_w as f32).clamp(0.0, (src_w - 1) as f32);
            let final_min_y =
                (exp_min_y / height as f32 * src_h as f32).clamp(0.0, (src_h - 1) as f32);
            let final_max_y =
                (exp_max_y / height as f32 * src_h as f32).clamp(0.0, (src_h - 1) as f32);

            let final_width = final_max_x - final_min_x;
            let final_height = final_max_y - final_min_y;

            // Filter by minimum size
            // Python: utils.py lines 279-282
            if final_width <= 3.0 || final_height <= 3.0 {
                continue;
            }

            boxes.push(BoundingBox::new(
                final_min_x,
                final_min_y,
                final_width,
                final_height,
                score,
            ));
        }

        // Sort boxes in reading order (top-to-bottom, left-to-right)
        // Python: main.py lines 80-103
        boxes = Self::sort_boxes(boxes);

        boxes
    }

    /// Calculate box confidence score (fast mode)
    ///
    /// Ported from: `rapidocr/ch_ppocr_det/utils.py:box_score_fast`
    /// Lines 229-241
    ///
    /// Simplified version: uses bounding box mean instead of polygon fill
    // Precision loss acceptable: image/bitmap indices are small integers
    // Sign loss safe: image coordinates (points) are always non-negative
    // Truncation safe: coordinates clamped to valid image bounds
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation
    )]
    fn box_score_fast(bitmap: &ArrayView<f32, ndarray::Ix2>, points: &[[f32; 2]]) -> f32 {
        let (height, width) = (bitmap.shape()[0], bitmap.shape()[1]);

        let xmin = points
            .iter()
            .map(|p| p[0])
            .fold(f32::INFINITY, f32::min)
            .floor() as usize;
        let xmax = points
            .iter()
            .map(|p| p[0])
            .fold(f32::NEG_INFINITY, f32::max)
            .ceil() as usize;
        let ymin = points
            .iter()
            .map(|p| p[1])
            .fold(f32::INFINITY, f32::min)
            .floor() as usize;
        let ymax = points
            .iter()
            .map(|p| p[1])
            .fold(f32::NEG_INFINITY, f32::max)
            .ceil() as usize;

        let xmin = xmin.clamp(0, width - 1);
        let xmax = xmax.clamp(0, width - 1);
        let ymin = ymin.clamp(0, height - 1);
        let ymax = ymax.clamp(0, height - 1);

        // Calculate mean score in bounding box region
        // Note: Python uses polygon fill, but for axis-aligned boxes this is equivalent
        let mut sum = 0.0f32;
        let mut count = 0;

        for y in ymin..=ymax {
            for x in xmin..=xmax {
                sum += bitmap[[y, x]];
                count += 1;
            }
        }

        if count > 0 {
            sum / count as f32
        } else {
            0.0
        }
    }

    /// Unclip (expand) a box using offset
    ///
    /// Ported from: `rapidocr/ch_ppocr_det/utils.py:unclip`
    /// Lines 262-269
    // Precision loss acceptable: polygon coordinates come from image dimensions (small integers)
    #[allow(clippy::cast_precision_loss)]
    fn unclip_box(&self, points: &[[f32; 2]]) -> Vec<[f32; 2]> {
        // Calculate polygon area and perimeter using Shoelace formula
        let mut area = 0.0f32;
        let mut perimeter = 0.0f32;
        let n = points.len();

        for i in 0..n {
            let j = (i + 1) % n;
            area += points[i][0] * points[j][1];
            area -= points[j][0] * points[i][1];

            let dx = points[j][0] - points[i][0];
            let dy = points[j][1] - points[i][1];
            perimeter += dx.hypot(dy);
        }
        area = area.abs() / 2.0;

        // Calculate offset distance
        let distance = area * self.det_config.unclip_ratio / perimeter;

        // Use clipper to offset polygon
        // For simplicity, approximate with a simple expansion
        // In Python this uses pyclipper library
        let center_x = points.iter().map(|p| p[0]).sum::<f32>() / n as f32;
        let center_y = points.iter().map(|p| p[1]).sum::<f32>() / n as f32;

        let expanded: Vec<[f32; 2]> = points
            .iter()
            .map(|p| {
                let dx = p[0] - center_x;
                let dy = p[1] - center_y;
                let len = dx.hypot(dy);
                if len > 0.0 {
                    let scale = (len + distance) / len;
                    [dx.mul_add(scale, center_x), dy.mul_add(scale, center_y)]
                } else {
                    *p
                }
            })
            .collect();

        expanded
    }

    /// Sort boxes in reading order (top-to-bottom, left-to-right)
    ///
    /// Ported from: `rapidocr/ch_ppocr_det/main.py:sorted_boxes`
    /// Lines 80-103
    fn sort_boxes(mut boxes: Vec<BoundingBox>) -> Vec<BoundingBox> {
        // Handle empty or single box case
        if boxes.len() <= 1 {
            return boxes;
        }

        // Sort by y, then x
        boxes.sort_by(|a, b| a.y.total_cmp(&b.y).then(a.x.total_cmp(&b.x)));

        // Bubble sort with 10px vertical tolerance
        let n = boxes.len();
        for i in 0..(n - 1) {
            for j in (0..=i).rev() {
                if j + 1 < n
                    && (boxes[j + 1].y - boxes[j].y).abs() < 10.0
                    && boxes[j + 1].x < boxes[j].x
                {
                    boxes.swap(j, j + 1);
                } else {
                    break;
                }
            }
        }

        boxes
    }

    /// CTC greedy decoder with word segmentation
    ///
    /// Ported from: `rapidocr/ch_ppocr_rec/utils.py:CTCLabelDecode.decode`
    /// and `get_word_info` for word segmentation
    /// Lines 82-127, 145-220
    ///
    /// Word segmentation is based on column positions in the CTC output:
    /// - When gap between character positions > threshold, insert space
    /// - This handles English word boundaries detected from image spacing
    // Precision loss acceptable: CTC output shape and indices are small integers
    #[allow(clippy::cast_precision_loss)]
    fn ctc_decode(&self, preds: ndarray::ArrayView2<f32>) -> (String, f32) {
        // Word boundary threshold
        // Python RapidOCR uses threshold of 5, but our CTC output has different
        // density. Empirically, gaps between characters within a word are 5-7,
        // while word boundaries have gaps of 9-12+.
        // Using 8 as threshold to capture word boundaries without splitting words.
        const WORD_GAP_THRESHOLD: usize = 8;

        // Get predicted indices (argmax along character dimension)
        // Python: utils.py line 25
        let pred_indices: Vec<usize> = (0..preds.shape()[0])
            .map(|t| {
                let row = preds.row(t);
                row.iter()
                    .enumerate()
                    .max_by(|(_, a), (_, b)| a.total_cmp(b))
                    .map_or(0, |(idx, _)| idx)
            })
            .collect();

        // Get probabilities
        // Python: utils.py line 26
        let pred_probs: Vec<f32> = (0..preds.shape()[0])
            .map(|t| {
                let row = preds.row(t);
                *row.iter().max_by(|a, b| a.total_cmp(b)).unwrap_or(&0.0)
            })
            .collect();

        // Remove duplicates (CTC collapse)
        // Python: utils.py lines 97-99
        let mut selection = vec![true; pred_indices.len()];
        for i in 1..pred_indices.len() {
            if pred_indices[i] == pred_indices[i - 1] {
                selection[i] = false;
            }
        }

        // Remove blank tokens (index 0)
        // Python: utils.py lines 101-102
        let ignored_token = 0; // CTC blank token
        for i in 0..pred_indices.len() {
            if pred_indices[i] == ignored_token {
                selection[i] = false;
            }
        }

        // Collect selected characters with their column positions
        // Python: utils.py get_word_info method
        let mut char_list: Vec<&str> = Vec::new();
        let mut col_positions: Vec<usize> = Vec::new();
        let mut conf_list = Vec::new();

        for (i, &selected) in selection.iter().enumerate() {
            if selected && pred_indices[i] < self.characters.len() {
                char_list.push(self.characters[pred_indices[i]].as_str());
                col_positions.push(i); // Track column position
                conf_list.push(pred_probs[i]);
            }
        }

        // Word segmentation based on column gaps
        // Ported from Python: utils.py get_word_info
        //
        // The CTC decoder outputs one prediction per column. Characters occupy
        // multiple columns (typically 2-3). Word boundaries are detected when
        // the gap between character positions exceeds a threshold (5 in Python).
        //
        // This threshold corresponds to physical spacing in the image - words
        // are separated by more whitespace than characters within a word.
        let mut text = String::new();

        if col_positions.len() > 1 {
            // Calculate gaps between consecutive characters (col_width in Python)
            let gaps: Vec<usize> = col_positions
                .windows(2)
                .map(|w| w[1].saturating_sub(w[0]))
                .collect();

            for (idx, &ch) in char_list.iter().enumerate() {
                if idx > 0 && gaps[idx - 1] > WORD_GAP_THRESHOLD {
                    text.push(' ');
                }
                text.push_str(ch);
            }
        } else {
            // Single character - just return it
            for ch in char_list {
                text.push_str(ch);
            }
        }

        // Calculate mean confidence
        // Python: utils.py line 118
        let confidence = if conf_list.is_empty() {
            0.0
        } else {
            conf_list.iter().sum::<f32>() / conf_list.len() as f32
        };

        (text, confidence)
    }

    /// Recognize text in specific regions (recognition stage only)
    ///
    /// Ported from: `rapidocr/ch_ppocr_rec/main.py`:__call__
    /// Lines 85-146
    ///
    /// # Errors
    ///
    /// Returns an error if the recognition model is not loaded or inference fails.
    ///
    /// # Panics
    ///
    /// This function uses `.unwrap()` on `rec_session` internally, but it is
    /// guarded by an early return that checks `rec_session.is_none()` first.
    /// The panic is unreachable in normal execution.
    #[allow(clippy::cast_precision_loss)] // Image dimensions are small, f32 precision is fine
    pub fn recognize_regions(
        &mut self,
        image: &DynamicImage,
        regions: &[BoundingBox],
    ) -> Result<Vec<TextLine>> {
        // Check if recognition model is loaded
        if self.rec_session.is_none() {
            return Err(anyhow::anyhow!("Recognition model not loaded"));
        }

        if regions.is_empty() {
            return Ok(Vec::new());
        }

        // Crop image regions
        // Python: main.py line 88
        // Sign loss safe: max(0.0)/max(1.0) ensures non-negative values
        // Truncation safe: bounding box coordinates fit in u32
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        let img_list: Vec<DynamicImage> = regions
            .iter()
            .map(|bbox| {
                let x = bbox.x.max(0.0) as u32;
                let y = bbox.y.max(0.0) as u32;
                let w = bbox.width.max(1.0) as u32;
                let h = bbox.height.max(1.0) as u32;
                image.crop_imm(x, y, w, h)
            })
            .collect();

        // Calculate width/height ratios for sorting
        // Python: main.py lines 91-94
        let width_list: Vec<f32> = img_list
            .iter()
            .map(|img| {
                let (w, h) = img.dimensions();
                w as f32 / h as f32
            })
            .collect();

        // Sort by aspect ratio (helps batching efficiency)
        let mut indices: Vec<usize> = (0..img_list.len()).collect();
        indices.sort_by(|&a, &b| width_list[a].total_cmp(&width_list[b]));

        // Process in batches
        // Python: main.py lines 96-133
        let img_num = img_list.len();
        let mut rec_res = vec![(String::new(), 0.0); img_num];
        let batch_num = self.rec_config.rec_batch_num;

        for beg_img_no in (0..img_num).step_by(batch_num) {
            let end_img_no = (beg_img_no + batch_num).min(img_num);

            // Calculate max aspect ratio in this batch
            // Python: main.py lines 105-112
            let [_img_c, _img_h, _img_w] = self.rec_config.rec_img_shape;
            let max_wh_ratio = width_list[indices[beg_img_no..end_img_no]
                .iter()
                .copied()
                .max_by(|&a, &b| width_list[a].total_cmp(&width_list[b]))
                .unwrap_or(beg_img_no)];

            // Preprocess batch
            // Python: main.py lines 114-118
            let mut norm_img_batch = Vec::new();
            for ino in beg_img_no..end_img_no {
                let norm_img = self.preprocess_recognition(&img_list[indices[ino]], max_wh_ratio);
                norm_img_batch.push(norm_img);
            }

            // Stack into batch array (shape: [batch, C, H, W])
            let batch_size = norm_img_batch.len();
            let [img_c, img_h, _img_w] = self.rec_config.rec_img_shape;
            // Sign loss safe: img_h and max_wh_ratio are always positive
            // Truncation safe: dynamic width is bounded by max_wh_ratio and img_h
            #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
            let dynamic_img_w = (img_h as f32 * max_wh_ratio) as usize;

            let mut batch_array =
                ndarray::Array4::<f32>::zeros((batch_size, img_c, img_h, dynamic_img_w));
            for (i, img) in norm_img_batch.iter().enumerate() {
                batch_array
                    .slice_mut(ndarray::s![i, .., .., ..])
                    .assign(img);
            }

            // Run inference (ort 2.0 API) and extract outputs
            // Python: main.py line 120
            let batch_ref: TensorRef<f32> = TensorRef::from_array_view(&batch_array)?;
            let output_array = {
                // SAFETY: rec_session is guaranteed to be Some here because we check
                // rec_session.is_none() at lines 1047-1048 and return early if true.
                let rec_session = self.rec_session.as_mut().unwrap();
                let outputs = rec_session
                    .run(inputs![batch_ref])
                    .map_err(|e| anyhow::anyhow!("Recognition inference failed: {e}"))?;

                // Extract output tensor (shape: [batch, time_steps, num_classes])
                let output_tensor = outputs[0].try_extract_array::<f32>()?;
                output_tensor
                    .into_owned()
                    .into_dimensionality::<ndarray::Ix3>()
                    .map_err(|e| anyhow::anyhow!("Failed to reshape recognition output: {e}"))?
            };

            // Decode predictions for each image in batch
            // Python: main.py lines 121-133
            for (rno, idx) in indices[beg_img_no..end_img_no].iter().enumerate() {
                let pred = output_array.index_axis(ndarray::Axis(0), rno);
                let (text, confidence) = self.ctc_decode(pred);
                rec_res[*idx] = (text, confidence);
            }
        }

        // Create TextLine results
        let lines: Vec<TextLine> = rec_res
            .into_iter()
            .zip(regions.iter())
            .map(|((text, confidence), bbox)| TextLine::new(text, *bbox, confidence))
            .collect();

        Ok(lines)
    }
}

// Note: No Default implementation since model loading requires I/O

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::float_cmp)]
    fn test_bounding_box_creation() {
        let bbox = BoundingBox::new(10.0, 20.0, 100.0, 50.0, 0.95);
        assert_eq!(bbox.x, 10.0);
        assert_eq!(bbox.y, 20.0);
        assert_eq!(bbox.width, 100.0);
        assert_eq!(bbox.height, 50.0);
        assert_eq!(bbox.confidence, 0.95);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn test_bounding_box_calculations() {
        let bbox = BoundingBox::new(10.0, 20.0, 100.0, 50.0, 0.95);
        assert_eq!(bbox.right(), 110.0);
        assert_eq!(bbox.bottom(), 70.0);
        assert_eq!(bbox.center(), (60.0, 45.0));
        assert_eq!(bbox.area(), 5000.0);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn test_text_line_creation() {
        let bbox = BoundingBox::new(0.0, 0.0, 100.0, 20.0, 0.9);
        let line = TextLine::new("Hello World".to_string(), bbox, 0.92);
        assert_eq!(line.text, "Hello World");
        assert_eq!(line.confidence, 0.92);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn test_ocr_result_empty() {
        let result = OcrResult::new(vec![], (800, 600));
        assert!(result.is_empty());
        assert_eq!(result.len(), 0);
        assert_eq!(result.text(), "");
        assert_eq!(result.avg_confidence, 0.0);
    }

    #[test]
    fn test_ocr_result_with_lines() {
        let bbox1 = BoundingBox::new(0.0, 0.0, 100.0, 20.0, 0.9);
        let bbox2 = BoundingBox::new(0.0, 25.0, 100.0, 20.0, 0.95);

        let lines = vec![
            TextLine::new("First line".to_string(), bbox1, 0.9),
            TextLine::new("Second line".to_string(), bbox2, 0.95),
        ];

        let result = OcrResult::new(lines, (800, 600));
        assert!(!result.is_empty());
        assert_eq!(result.len(), 2);
        assert_eq!(result.text(), "First line\nSecond line");
        assert!((result.avg_confidence - 0.925).abs() < 0.001);
    }

    #[test]
    fn test_ocr_engine_requires_models() {
        // Engine creation requires detection model to exist
        let result = OcrEngine::new();
        // Will fail if models not downloaded (expected in CI)
        if let Err(e) = result {
            assert!(
                e.to_string().contains("not found") || e.to_string().contains("Failed to load"),
                "Unexpected error: {e}"
            );
        }
    }

    #[test]
    fn test_full_ocr_pipeline() {
        use image::RgbImage;

        // Create a simple test image with text (white background, black text)
        let mut img = RgbImage::new(200, 50);
        for pixel in img.pixels_mut() {
            *pixel = image::Rgb([255, 255, 255]); // White background
        }

        // Load OCR engine
        let mut engine = match OcrEngine::new() {
            Ok(e) => e,
            Err(e) => {
                eprintln!("Skipping test: {e}");
                return;
            }
        };

        // Verify character dictionary loaded
        assert!(
            !engine.characters.is_empty(),
            "Character dictionary should be loaded"
        );
        assert_eq!(
            engine.characters[0], "blank",
            "First character should be CTC blank token"
        );
        assert!(
            engine.characters.len() > 6000,
            "Should have ~6622 characters"
        );

        // Test detection on a simple image (may not detect anything without real text)
        let dynamic_img = DynamicImage::ImageRgb8(img);
        let result = engine.detect(&dynamic_img);
        assert!(result.is_ok(), "Detection should not fail");

        // Full pipeline test (may return empty for simple image)
        let result = engine.recognize(&dynamic_img);
        assert!(result.is_ok(), "Recognition should not fail");
        let ocr_result = result.unwrap();
        assert_eq!(ocr_result.image_size, (200, 50));
    }

    #[test]
    fn test_character_dictionary_structure() {
        let engine = match OcrEngine::new() {
            Ok(e) => e,
            Err(e) => {
                eprintln!("Skipping test: {e}");
                return;
            }
        };

        // Verify dictionary structure
        assert_eq!(engine.characters[0], "blank", "Index 0 should be CTC blank");
        assert_eq!(
            engine.characters.last().unwrap(),
            " ",
            "Last character should be space"
        );

        // Verify some common characters exist
        assert!(engine.characters.contains(&"a".to_string()));
        assert!(engine.characters.contains(&"A".to_string()));
        assert!(engine.characters.contains(&"0".to_string()));
        assert!(engine.characters.contains(&"1".to_string()));
    }
}
