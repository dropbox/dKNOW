//! # DocLayout-YOLO Layout Detector
//!
//! YOLO-based layout detection using DocLayout-YOLO-DocLayNet model.
//!
//! ## Performance (N=3491 Benchmarks)
//!
//! **WARNING: YOLO is 2.5x SLOWER than RT-DETR on CPU!**
//!
//! | Backend | YOLO Time | RT-DETR Time | Comparison |
//! |---------|-----------|--------------|------------|
//! | CPU     | ~590ms    | ~240ms       | YOLO 2.5x slower |
//! | GPU     | ~10ms     | ~200ms       | YOLO 20x faster |
//!
//! - YOLO uses 1120×1120 input (3.06x more pixels than RT-DETR's 640×640)
//! - Published ~10ms speeds require GPU acceleration (CUDA/CoreML/Metal)
//! - For CPU-only deployment, use RT-DETR instead
//! - 11 `DocLayNet` classes (compatible with our core schema)
//!
//! ## Usage
//!
//! ```ignore
//! use docling_pdf_ml::models::layout_predictor::doclayout_yolo::DocLayoutYolo;
//!
//! let model = DocLayoutYolo::load(Path::new("models/doclayout_yolo_doclaynet.onnx"))?;
//! let clusters = model.infer(&image)?;
//! ```

// Image dimensions and coordinates are converted between usize/i64 (array indexing)
// and f32/i32 (normalized model I/O). Precision loss is acceptable for dimensions < 10000.
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]

use crate::baseline::{BBox, LayoutCluster};
use ndarray::{Array3, Array4};
use ort::execution_providers::CPUExecutionProvider;
use ort::session::Session;
use std::path::Path;
use std::time::Instant;

/// `DocLayNet` 11 class labels (standard document layout classes)
const DOCLAYNET_CLASSES: &[&str] = &[
    "Caption",        // 0
    "Footnote",       // 1
    "Formula",        // 2
    "List-item",      // 3
    "Page-footer",    // 4 - Note: lowercase in DocLayNet
    "Page-header",    // 5
    "Picture",        // 6
    "Section-header", // 7
    "Table",          // 8
    "Text",           // 9
    "Title",          // 10
];

/// Result type for inference with timing information.
/// Contains (clusters, `preprocess_ms`, `inference_ms`, `postprocess_ms`).
pub type InferenceTimingResult =
    Result<(Vec<LayoutCluster>, f64, f64, f64), Box<dyn std::error::Error>>;

/// DocLayout-YOLO model for fast layout detection
pub struct DocLayoutYolo {
    session: Session,
    /// Input resolution (default 1120x1120 for `DocLayNet` model)
    input_size: u32,
    /// Confidence threshold for detection filtering
    confidence_threshold: f32,
}

impl std::fmt::Debug for DocLayoutYolo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DocLayoutYolo")
            .field("session", &"<Session>")
            .field("input_size", &self.input_size)
            .field("confidence_threshold", &self.confidence_threshold)
            .finish()
    }
}

impl DocLayoutYolo {
    /// Load DocLayout-YOLO ONNX model
    ///
    /// # Arguments
    ///
    /// * `model_path` - Path to ONNX model file
    ///
    /// # Returns
    ///
    /// Loaded model ready for inference
    #[must_use = "returns loaded model that should be used"]
    pub fn load(model_path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        Self::load_with_config(model_path, 1120, 0.3)
    }

    /// Load DocLayout-YOLO with custom configuration
    ///
    /// # Arguments
    ///
    /// * `model_path` - Path to ONNX model file
    /// * `input_size` - Input resolution (1120 for `DocLayNet`, 1024 for `DocStructBench`)
    /// * `confidence_threshold` - Detection confidence threshold (default 0.3)
    #[must_use = "returns loaded model that should be used"]
    pub fn load_with_config(
        model_path: &Path,
        input_size: u32,
        confidence_threshold: f32,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let session = Session::builder()?
            .with_execution_providers([CPUExecutionProvider::default().build()])?
            .commit_from_file(model_path)?;

        Ok(Self {
            session,
            input_size,
            confidence_threshold,
        })
    }

    /// Run inference on image
    ///
    /// # Arguments
    ///
    /// * `image` - Input image (Array3<u8>, HWC format, [0-255])
    ///
    /// # Returns
    ///
    /// * `Vec<LayoutCluster>` - Detected layout elements with bounding boxes
    #[allow(clippy::similar_names)] // orig_h_px vs orig_w_px are height/width - intentionally similar
    pub fn infer(
        &mut self,
        image: &Array3<u8>,
    ) -> Result<Vec<LayoutCluster>, Box<dyn std::error::Error>> {
        // Get original image dimensions for scaling boxes back
        let (orig_h_px, orig_w_px, _) = image.dim();
        let orig_h = orig_h_px as f32;
        let orig_w = orig_w_px as f32;

        // 1. Preprocess image to model input size
        let preprocessed = self.preprocess(image)?;

        // 2. Convert to ort::Value
        let shape = preprocessed.shape().to_vec();
        let (data, _offset) = preprocessed.into_raw_vec_and_offset();
        let input_value = ort::value::Value::from_array((shape.as_slice(), data))?;

        // 3. Run ONNX inference
        let outputs = self.session.run(ort::inputs!["images" => input_value])?;

        // 4. Extract output - YOLO format: [1, 300, 6]
        // Each detection: [x1, y1, x2, y2, confidence, class_id]
        let output_name = outputs.keys().next().ok_or("No output from model")?;
        let (output_shape, output_data) = outputs[output_name].try_extract_tensor::<f32>()?;
        let output_data_vec = output_data.to_vec();
        let output_shape_vec: Vec<usize> = output_shape.iter().map(|&d| d as usize).collect();

        // Drop outputs to release borrow on session before calling post_process
        drop(outputs);

        // 5. Post-process to LayoutCluster
        let clusters = Self::post_process_yolo(
            &output_shape_vec,
            &output_data_vec,
            orig_w,
            orig_h,
            self.input_size,
            self.confidence_threshold,
        )?;

        Ok(clusters)
    }

    /// Run inference with timing (for profiling)
    ///
    /// Returns (clusters, `preprocessing_ms`, `inference_ms`, `postprocess_ms`)
    #[allow(dead_code, reason = "profiling method for performance benchmarking")]
    #[allow(clippy::similar_names)] // orig_h_px vs orig_w_px are height/width - intentionally similar
    pub fn infer_with_timing(&mut self, image: &Array3<u8>) -> InferenceTimingResult {
        let (orig_h_px, orig_w_px, _) = image.dim();
        let orig_h = orig_h_px as f32;
        let orig_w = orig_w_px as f32;

        // 1. Preprocess
        let t0 = Instant::now();
        let preprocessed = self.preprocess(image)?;
        let preprocess_ms = t0.elapsed().as_secs_f64() * 1000.0;

        // 2. Convert to ort::Value
        let shape = preprocessed.shape().to_vec();
        let (data, _offset) = preprocessed.into_raw_vec_and_offset();
        let input_value = ort::value::Value::from_array((shape.as_slice(), data))?;

        // 3. Run ONNX inference
        let t1 = Instant::now();
        let outputs = self.session.run(ort::inputs!["images" => input_value])?;
        let inference_ms = t1.elapsed().as_secs_f64() * 1000.0;

        // 4. Extract output
        let t2 = Instant::now();
        let output_name = outputs.keys().next().ok_or("No output from model")?;
        let (output_shape, output_data) = outputs[output_name].try_extract_tensor::<f32>()?;
        let output_data_vec = output_data.to_vec();
        let output_shape_vec: Vec<usize> = output_shape.iter().map(|&d| d as usize).collect();
        drop(outputs);

        // 5. Post-process
        let clusters = Self::post_process_yolo(
            &output_shape_vec,
            &output_data_vec,
            orig_w,
            orig_h,
            self.input_size,
            self.confidence_threshold,
        )?;
        let postprocess_ms = t2.elapsed().as_secs_f64() * 1000.0;

        Ok((clusters, preprocess_ms, inference_ms, postprocess_ms))
    }

    /// Preprocess image for YOLO inference
    ///
    /// Resizes to `input_size` x `input_size` and normalizes to [0, 1]
    /// Optimized implementation using raw byte operations for speed.
    fn preprocess(&self, image: &Array3<u8>) -> Result<Array4<f32>, Box<dyn std::error::Error>> {
        let (orig_h, orig_w, _channels) = image.dim();
        let target_size = self.input_size as usize;

        // Letterbox resize (maintain aspect ratio with padding)
        let scale = (target_size as f32 / orig_h as f32).min(target_size as f32 / orig_w as f32);
        let new_w = (orig_w as f32 * scale).round() as usize;
        let new_h = (orig_h as f32 * scale).round() as usize;

        // Calculate padding offsets (center the image)
        let pad_y = (target_size - new_h) / 2;
        let pad_x = (target_size - new_w) / 2;

        // Create output directly in NCHW format with gray (0.5) background
        // Pre-allocate with normalized gray value to avoid filling later
        let mut output = Array4::<f32>::from_elem((1, 3, target_size, target_size), 0.5);
        let out_slice = output.as_slice_mut().ok_or("Output not contiguous")?;
        let img_slice = image.as_slice().ok_or("Image not contiguous")?;

        // Calculate scale factors for nearest-neighbor resize
        let scale_h = orig_h as f32 / new_h as f32;
        let scale_w = orig_w as f32 / new_w as f32;

        let channel_size = target_size * target_size;

        // Optimized loop: process rows to improve cache locality
        // Pre-compute source Y indices
        for dst_y in 0..new_h {
            let src_y = (((dst_y as f32 + 0.5) * scale_h) as usize).min(orig_h - 1);
            let dst_row = (pad_y + dst_y) * target_size;
            let src_row = src_y * orig_w * 3;

            for dst_x in 0..new_w {
                let src_x = (((dst_x as f32 + 0.5) * scale_w) as usize).min(orig_w - 1);
                let dst_col = pad_x + dst_x;
                let src_idx = src_row + src_x * 3;

                // Read RGB values from source (HWC format)
                let r = f32::from(img_slice[src_idx]) / 255.0;
                let g = f32::from(img_slice[src_idx + 1]) / 255.0;
                let b = f32::from(img_slice[src_idx + 2]) / 255.0;

                // Write to destination (NCHW format)
                let dst_base = dst_row + dst_col;
                out_slice[dst_base] = r;
                out_slice[channel_size + dst_base] = g;
                out_slice[2 * channel_size + dst_base] = b;
            }
        }

        Ok(output)
    }

    /// Post-process YOLO output to [`LayoutCluster`]s
    ///
    /// YOLO output format: `[1, num_detections, 6]`
    /// Each detection: `[x1, y1, x2, y2, confidence, class_id]`
    /// Coordinates are in model input space (0 to `input_size`)
    #[allow(clippy::unnecessary_wraps)] // Result for API consistency with other post-process methods
    fn post_process_yolo(
        output_shape: &[usize],
        output_data: &[f32],
        orig_w: f32,
        orig_h: f32,
        model_input_size: u32,
        confidence_threshold: f32,
    ) -> Result<Vec<LayoutCluster>, Box<dyn std::error::Error>> {
        let num_detections = output_shape[1];
        let mut clusters = Vec::new();

        // Calculate letterbox parameters for coordinate conversion
        let input_size = model_input_size as f32;
        let scale = (input_size / orig_h).min(input_size / orig_w);
        let new_h = (orig_h * scale).round();
        let new_w = (orig_w * scale).round();
        let pad_h = (input_size - new_h) / 2.0;
        let pad_w = (input_size - new_w) / 2.0;

        for i in 0..num_detections {
            let base_idx = i * 6;
            let x1 = output_data[base_idx];
            let y1 = output_data[base_idx + 1];
            let x2 = output_data[base_idx + 2];
            let y2 = output_data[base_idx + 3];
            let confidence = output_data[base_idx + 4];
            let class_id = output_data[base_idx + 5] as usize;

            // Filter by confidence
            if confidence < confidence_threshold {
                continue;
            }

            // Convert from letterboxed coordinates to original image coordinates
            let l = f64::from((x1 - pad_w) / scale);
            let t = f64::from((y1 - pad_h) / scale);
            let r = f64::from((x2 - pad_w) / scale);
            let b = f64::from((y2 - pad_h) / scale);

            // Clamp to image bounds
            let l = l.max(0.0).min(f64::from(orig_w));
            let t = t.max(0.0).min(f64::from(orig_h));
            let r = r.max(0.0).min(f64::from(orig_w));
            let b = b.max(0.0).min(f64::from(orig_h));

            // Get label
            let label = if class_id < DOCLAYNET_CLASSES.len() {
                DOCLAYNET_CLASSES[class_id].to_string()
            } else {
                format!("unknown_{class_id}")
            };

            clusters.push(LayoutCluster {
                id: clusters.len() as i32,
                label,
                confidence: f64::from(confidence),
                bbox: BBox { l, t, r, b },
            });
        }

        // Sort by confidence (descending)
        clusters.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());

        Ok(clusters)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_doclayout_yolo_load() {
        let model_paths = [
            "models/doclayout_yolo_doclaynet.onnx",
            "crates/docling-pdf-ml/models/doclayout_yolo_doclaynet.onnx",
        ];

        for path in &model_paths {
            let p = Path::new(path);
            if p.exists() {
                let model = DocLayoutYolo::load(p);
                assert!(
                    model.is_ok(),
                    "Failed to load DocLayout-YOLO: {:?}",
                    model.err()
                );
                eprintln!("Successfully loaded DocLayout-YOLO from: {path}");
                return;
            }
        }

        eprintln!("DocLayout-YOLO model not found, skipping test");
    }

    #[test]
    fn test_doclayout_yolo_inference() {
        let model_paths = [
            "models/doclayout_yolo_doclaynet.onnx",
            "crates/docling-pdf-ml/models/doclayout_yolo_doclaynet.onnx",
        ];

        let mut model_opt = None;
        for path in &model_paths {
            let p = Path::new(path);
            if p.exists() {
                if let Ok(m) = DocLayoutYolo::load(p) {
                    model_opt = Some(m);
                    break;
                }
            }
        }

        let mut model = match model_opt {
            Some(m) => m,
            None => {
                eprintln!("DocLayout-YOLO model not found, skipping inference test");
                return;
            }
        };

        // Create synthetic test image
        let image = Array3::<u8>::zeros((792, 612, 3));
        let result = model.infer(&image);
        assert!(result.is_ok(), "Inference failed: {:?}", result.err());

        let clusters = result.unwrap();
        eprintln!("Detected {} clusters on synthetic image", clusters.len());
    }
}
