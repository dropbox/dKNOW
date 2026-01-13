// AngleNet - Text rotation classification model
//
// Reference: AngleNet.cpp, AngleNet.h from RapidOcrOnnx
// Model: ch_ppocr_mobile_v2.0_cls_infer.onnx

// Image dimensions use usize internally but u32 for image crate compatibility.
// Values are always within u32 range for practical image sizes.
#![allow(clippy::cast_possible_truncation)]

use crate::error::Result;
use crate::ocr::types::{Angle, ANGLENET_WIDTH, OCR_MODEL_HEIGHT, OCR_NORMALIZE_DIVISOR};
use image::DynamicImage;
use ndarray::Array4;
use ort::execution_providers::{CPUExecutionProvider, CoreMLExecutionProvider};
use ort::session::{builder::GraphOptimizationLevel, Session};

/// `AngleNet` text rotation classification model
///
/// Classifies text orientation as either 0° or 180°.
///
/// Reference: AngleNet.cpp
pub struct AngleNet {
    session: Session,
}

impl std::fmt::Debug for AngleNet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AngleNet")
            .field("session", &"<Session>")
            .finish()
    }
}

impl AngleNet {
    /// Load `AngleNet` model from ONNX file (CPU backend)
    ///
    /// # Arguments
    /// * `model_path` - Path to `ch_ppocr_mobile_v2.0_cls_infer.onnx`
    #[must_use = "this returns a Result that should be handled"]
    pub fn new(model_path: &str) -> Result<Self> {
        Self::new_with_backend(model_path, false)
    }

    /// Load `AngleNet` model with `CoreML` backend (Apple Neural Engine acceleration)
    ///
    /// On macOS with Apple Silicon, this enables hardware-accelerated inference
    /// via the Apple Neural Engine (ANE), providing 2-3x speedup over CPU.
    ///
    /// # Arguments
    /// * `model_path` - Path to `ch_ppocr_mobile_v2.0_cls_infer.onnx`
    #[must_use = "this returns a Result that should be handled"]
    pub fn new_with_coreml(model_path: &str) -> Result<Self> {
        Self::new_with_backend(model_path, true)
    }

    /// Load `AngleNet` model with specified backend
    ///
    /// # Arguments
    /// * `model_path` - Path to `ch_ppocr_mobile_v2.0_cls_infer.onnx`
    /// * `use_coreml` - If true, use `CoreML` execution provider (macOS ANE acceleration)
    fn new_with_backend(model_path: &str, use_coreml: bool) -> Result<Self> {
        let num_threads = num_cpus::get();

        let session = if use_coreml {
            log::debug!("Creating AngleNet session with CoreML execution provider");
            Session::builder()?
                .with_optimization_level(GraphOptimizationLevel::Level3)?
                .with_intra_threads(num_threads)?
                .with_execution_providers([
                    CoreMLExecutionProvider::default().build(),
                    CPUExecutionProvider::default().build(), // Fallback
                ])?
                .commit_from_file(model_path)?
        } else {
            log::debug!("Creating AngleNet session with CPU execution provider");
            Session::builder()?
                .with_optimization_level(GraphOptimizationLevel::Level3)?
                .with_intra_threads(num_threads)?
                .commit_from_file(model_path)?
        };

        Ok(Self { session })
    }

    /// Classify text rotation for multiple images
    ///
    /// # Pipeline (Reference: AngleNet.cpp:89-108)
    /// 1. Preprocess: Resize to 192x48, normalize (lines 118, 90)
    /// 2. ONNX Inference: Get probabilities [0°, 180°] (lines 99-100)
    /// 3. Postprocess: Argmax to find most likely angle (lines 77-87)
    ///
    /// # Arguments
    /// * `images` - Cropped text region images to classify
    ///
    /// # Returns
    /// Vector of Angle objects (index: 0=0°, 1=180°, score: confidence)
    #[must_use = "angle classification returns results that should be processed"]
    pub fn classify(&mut self, images: &[DynamicImage]) -> Result<Vec<Angle>> {
        let mut angles = Vec::with_capacity(images.len());

        for image in images {
            // 1. Preprocess (resize + normalize)
            let input_tensor = self.preprocess(image)?;

            // 2. ONNX inference
            // Reference: AngleNet.cpp:99-100
            // Convert ndarray to ONNX Value
            let shape = input_tensor.shape().to_vec();
            let (data, _offset) = input_tensor.into_raw_vec_and_offset();
            let input_value = ort::value::Value::from_array((shape.as_slice(), data))?;

            let outputs = self.session.run(ort::inputs!["x" => input_value])?;

            // 3. Postprocess (argmax)
            // Reference: AngleNet.cpp:77-87 (scoreToAngle function)
            let (_output_shape, output_data) = outputs[0].try_extract_tensor::<f32>()?;

            let angle = score_to_angle(output_data);
            angles.push(angle);
        }

        Ok(angles)
    }

    /// Preprocess image for `AngleNet` inference
    ///
    /// # Pipeline (Reference: AngleNet.cpp:118, 90)
    /// 1. Resize to 192x48 (line 118: `cv::resize(partImgs[i], angleImg, cv::Size(dstWidth, dstHeight))`)
    /// 2. Normalize: (pixel - 127.5) / 127.5 (lines 33-34: `meanValues`/`normValues`)
    ///
    /// # Arguments
    /// * `image` - Input image to preprocess
    ///
    /// # Returns
    /// NCHW tensor [1, 3, 48, 192] with normalized pixel values
    // Method signature kept for API consistency with other AngleClassifier methods
    #[allow(clippy::unused_self)]
    #[allow(clippy::unnecessary_wraps)] // Result for API consistency with other preprocess methods
    fn preprocess(&self, image: &DynamicImage) -> Result<Array4<f32>> {
        // Resize to target size
        // Reference: AngleNet.h:35-36 - dstWidth = 192, dstHeight = 48
        let resized = image.resize_exact(
            ANGLENET_WIDTH,
            OCR_MODEL_HEIGHT as u32,
            image::imageops::FilterType::Lanczos3,
        );

        let rgb_image = resized.to_rgb8();

        // Create tensor [1, 3, H, W]
        let mut tensor = Array4::<f32>::zeros((1, 3, OCR_MODEL_HEIGHT, ANGLENET_WIDTH as usize));

        // Normalize: (pixel - mean) / std
        // Reference: AngleNet.h:33-34
        // const float meanValues[3] = {127.5, 127.5, 127.5};
        // const float normValues[3] = {1.0 / 127.5, 1.0 / 127.5, 1.0 / 127.5};
        // NOTE: Different from DbNet! AngleNet uses 127.5, DbNet uses ImageNet mean/std
        for y in 0..OCR_MODEL_HEIGHT {
            for x in 0..ANGLENET_WIDTH {
                let pixel = rgb_image.get_pixel(x, y as u32);
                for c in 0..3 {
                    let value = f32::from(pixel[c]);
                    // Normalize: pixel / 127.5 - 1.0 (maps [0, 255] to [-1, 1])
                    tensor[[0, c, y, x as usize]] = value / OCR_NORMALIZE_DIVISOR - 1.0;
                }
            }
        }

        Ok(tensor)
    }
}

/// Convert output probabilities to Angle classification
///
/// Reference: AngleNet.cpp:77-87
///
/// # Arguments
/// * `output_data` - Output tensor probabilities [p(0°), p(180°)]
///
/// # Returns
/// Angle with index (0 or 1) and confidence score
fn score_to_angle(output_data: &[f32]) -> Angle {
    let mut max_index = 0;
    let mut max_score = 0.0_f32;

    for (i, &score) in output_data.iter().enumerate() {
        if score > max_score {
            max_score = score;
            max_index = i;
        }
    }

    Angle {
        index: max_index,
        score: max_score,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::DynamicImage;

    #[test]
    fn test_anglenet_loading() {
        let model = AngleNet::new("models/rapidocr/ch_ppocr_mobile_v2.0_cls_infer.onnx");
        assert!(
            model.is_ok(),
            "Failed to load AngleNet model: {:?}",
            model.err()
        );
    }

    #[test]
    fn test_anglenet_classify() {
        // Load model
        let mut model = AngleNet::new("models/rapidocr/ch_ppocr_mobile_v2.0_cls_infer.onnx")
            .expect("Failed to load AngleNet model");

        // Create test images (simple solid color images)
        // In real use, these would be cropped text regions from DbNet
        let test_images = vec![
            DynamicImage::new_rgb8(100, 50), // Small test image
            DynamicImage::new_rgb8(200, 30), // Different aspect ratio
        ];

        // Run classification
        let result = model.classify(&test_images);
        assert!(
            result.is_ok(),
            "AngleNet classification failed: {:?}",
            result.err()
        );

        let angles = result.unwrap();
        assert_eq!(angles.len(), 2, "Expected 2 angle results");

        // Verify output structure
        for (i, angle) in angles.iter().enumerate() {
            // Index should be 0 or 1 (0° or 180°)
            assert!(
                angle.index == 0 || angle.index == 1,
                "Angle {} has invalid index: {} (expected 0 or 1)",
                i,
                angle.index
            );

            // Score should be in range [0.0, 1.0]
            assert!(
                angle.score >= 0.0 && angle.score <= 1.0,
                "Angle {} has invalid score: {} (expected 0.0-1.0)",
                i,
                angle.score
            );
        }
    }

    #[test]
    fn test_score_to_angle() {
        // Test case 1: First probability is higher (0° rotation)
        let probs1 = vec![0.9, 0.1];
        let angle1 = score_to_angle(&probs1);
        assert_eq!(
            angle1.index, 0,
            "Expected index 0 for higher first probability"
        );
        assert_eq!(angle1.score, 0.9, "Expected score 0.9");

        // Test case 2: Second probability is higher (180° rotation)
        let probs2 = vec![0.2, 0.8];
        let angle2 = score_to_angle(&probs2);
        assert_eq!(
            angle2.index, 1,
            "Expected index 1 for higher second probability"
        );
        assert_eq!(angle2.score, 0.8, "Expected score 0.8");

        // Test case 3: Equal probabilities (should pick first)
        let probs3 = vec![0.5, 0.5];
        let angle3 = score_to_angle(&probs3);
        assert_eq!(angle3.index, 0, "Expected index 0 for equal probabilities");
        assert_eq!(angle3.score, 0.5, "Expected score 0.5");
    }
}
