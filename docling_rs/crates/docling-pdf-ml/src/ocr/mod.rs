// OCR - Native Rust OCR implementations
//
// RapidOCR: Port of RapidOcrOnnx C++ implementation to Rust
// Reference: https://github.com/RapidAI/RapidOcrOnnx
// Pipeline: Detection (DbNet) → Classification (AngleNet) → Recognition (CrnnNet)
//
// Apple Vision: macOS-only high-quality OCR via Vision framework
// Uses macocr CLI wrapper: https://github.com/riddleling/macocr
// Produces 7x better results than RapidOCR on scanned English documents

// Intentional ML conversions: OCR coordinates, tensor indices
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]

pub mod apple_vision;
pub mod box_points;
pub mod classification;
pub mod detection;
pub mod postprocess_pure;
pub mod recognition;
pub mod types;
pub mod utils;

// Re-export commonly used types
pub use detection::DbNetProfiling;
pub use detection::DbNetPure;
pub use recognition::CrnnNetProfiling;
pub use types::OcrParams;

// Re-export Apple Vision OCR (macOS only)
pub use apple_vision::AppleVisionOcr;

// Re-export RapidOcrPure (always available, pure Rust)
// RapidOcr requires opencv-preprocessing feature

#[cfg(feature = "opencv-preprocessing")]
use crate::error::Result;
#[cfg(feature = "opencv-preprocessing")]
use crate::ocr::types::*;
#[cfg(feature = "opencv-preprocessing")]
use image::DynamicImage;
use std::time::Duration;
#[cfg(feature = "opencv-preprocessing")]
use std::time::Instant;

/// Stage-level profiling for `RapidOCR` pipeline.
///
/// Tracks the time spent in each stage of the OCR processing pipeline,
/// enabling performance analysis and optimization.
///
/// # Stages
///
/// 1. **Detection** (`DbNet`): Locate text regions in the image
/// 2. **Crop**: Extract image patches for each detected text region
/// 3. **Classification** (`AngleNet`): Determine text orientation (0° or 180°)
/// 4. **Rotation**: Rotate images that are upside-down
/// 5. **Recognition** (`CrnnNet`): Extract text from each region via CTC decoding
/// 6. **Assembly**: Combine results into `TextCell` objects with bounding boxes
///
/// # Examples
///
/// ```no_run
/// use docling_pdf_ml::ocr::OcrProfiling;
///
/// let profiling = OcrProfiling::default();
/// println!("Total OCR time: {:?}", profiling.total());
/// profiling.print(); // Prints detailed breakdown
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OcrProfiling {
    /// Time spent in text region detection (`DbNet` model inference + postprocessing).
    /// This stage identifies rectangular regions containing text.
    pub detection_duration: Duration,

    /// Time spent cropping detected text regions from the source image.
    /// Each detected region is extracted as a separate image patch.
    pub crop_duration: Duration,

    /// Time spent classifying text orientation (`AngleNet` model inference).
    /// Determines if text is upright (0°) or upside-down (180°).
    pub classification_duration: Duration,

    /// Time spent rotating upside-down text regions.
    /// Only regions classified as 180° rotated are processed.
    pub rotation_duration: Duration,

    /// Time spent in text recognition (`CrnnNet` model inference + CTC decoding).
    /// This is typically the longest stage for multi-region images.
    pub recognition_duration: Duration,

    /// Time spent assembling final `TextCell` results.
    /// Combines text, bounding boxes, and confidence scores.
    pub assembly_duration: Duration,
}

impl OcrProfiling {
    #[inline]
    #[must_use = "returns the total duration sum"]
    pub fn total(&self) -> Duration {
        self.detection_duration
            + self.crop_duration
            + self.classification_duration
            + self.rotation_duration
            + self.recognition_duration
            + self.assembly_duration
    }

    pub fn print(&self) {
        let total_ms = self.total().as_secs_f64() * 1000.0;
        log::debug!("\n╔═══════════════════════════════════════════════════════════════╗");
        log::debug!("║  RapidOCR Stage-Level Profiling");
        log::debug!("╚═══════════════════════════════════════════════════════════════╝");
        log::debug!("");
        log::debug!("  {:30} {:>12} {:>12}", "Stage", "Time (ms)", "% of Total");
        log::debug!("  {}", "─".repeat(55));

        let stages = [
            ("Detection (DbNet)", self.detection_duration),
            ("Crop text regions", self.crop_duration),
            ("Classification (AngleNet)", self.classification_duration),
            ("Rotation", self.rotation_duration),
            ("Recognition (CrnnNet)", self.recognition_duration),
            ("Assembly", self.assembly_duration),
        ];

        for (name, duration) in &stages {
            let ms = duration.as_secs_f64() * 1000.0;
            let pct = (ms / total_ms) * 100.0;
            log::debug!("  {name:30} {ms:12.2} {pct:11.1}%");
        }

        log::debug!("  {}", "─".repeat(55));
        log::debug!("  {:30} {:12.2} {:>12}", "TOTAL", total_ms, "100.0%");
        log::debug!("");
    }
}

/// Main RapidOCR orchestrator
///
/// Integrates three ONNX models:
/// 1. DbNet: Text region detection
/// 2. AngleNet: Text rotation classification (0° or 180°)
/// 3. CrnnNet: Text recognition with CTC decoding
#[cfg(feature = "opencv-preprocessing")]
pub struct RapidOcr {
    detection: detection::DbNet,
    classification: classification::AngleNet,
    recognition: recognition::CrnnNet,
}

#[cfg(feature = "opencv-preprocessing")]
impl std::fmt::Debug for RapidOcr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RapidOcr")
            .field("detection", &self.detection)
            .field("classification", &self.classification)
            .field("recognition", &self.recognition)
            .finish()
    }
}

#[cfg(feature = "opencv-preprocessing")]
impl RapidOcr {
    /// Create new RapidOcr instance with models from directory
    ///
    /// # Arguments
    /// * `models_dir` - Path to directory containing ONNX models
    ///
    /// # Models Required
    /// - `ch_PP-OCRv4_det_infer.onnx` - Detection model
    /// - `ch_ppocr_mobile_v2.0_cls_infer.onnx` - Classification model
    /// - `ch_PP-OCRv4_rec_infer.onnx` - Recognition model
    /// - `ppocr_keys_v1.txt` - Character dictionary (6622 characters)
    #[must_use = "this returns a Result that should be handled"]
    pub fn new(models_dir: &str) -> Result<Self> {
        let detection =
            detection::DbNet::new(&format!("{}/ch_PP-OCRv4_det_infer.onnx", models_dir))?;

        let classification = classification::AngleNet::new(&format!(
            "{}/ch_ppocr_mobile_v2.0_cls_infer.onnx",
            models_dir
        ))?;

        let recognition = recognition::CrnnNet::new(
            &format!("{}/ch_PP-OCRv4_rec_infer.onnx", models_dir),
            &format!("{}/ppocr_keys_v1.txt", models_dir),
        )?;

        Ok(Self {
            detection,
            classification,
            recognition,
        })
    }

    /// Perform OCR on an image with optional stage-level profiling
    ///
    /// # Pipeline
    /// 1. Detection: Find text regions in image
    /// 2. Extraction: Crop detected text regions
    /// 3. Classification: Detect text rotation (0° or 180°)
    /// 4. Rotation: Rotate images if needed
    /// 5. Recognition: OCR text from rotated regions
    /// 6. Assembly: Combine results into `TextCell` objects
    ///
    /// # Arguments
    /// * `image` - Input image to process
    /// * `params` - OCR parameters (thresholds, etc.)
    ///
    /// # Returns
    /// Tuple of (TextCell vector, optional OcrProfiling)
    pub fn detect_with_profiling(
        &mut self,
        image: &DynamicImage,
        params: &OcrParams,
        enable_profiling: bool,
    ) -> Result<(Vec<TextCell>, Option<OcrProfiling>)> {
        let mut profiling = if enable_profiling {
            Some(OcrProfiling::default())
        } else {
            None
        };

        // Step 1: Detection - Find text boxes
        let start = Instant::now();
        let text_boxes = self.detection.detect(image, &params.detection)?;
        if let Some(ref mut p) = profiling {
            p.detection_duration = start.elapsed();
        }

        if text_boxes.is_empty() {
            return Ok((Vec::new(), profiling));
        }

        // Step 2: Extract part images - Crop detected text regions
        let start = Instant::now();
        let mut part_images: Vec<DynamicImage> = text_boxes
            .iter()
            .map(|text_box| utils::crop_text_box(image, text_box))
            .collect();
        if let Some(ref mut p) = profiling {
            p.crop_duration = start.elapsed();
        }

        // Step 3: Classification - Detect text rotation (0° or 180°)
        let start = Instant::now();
        let angles = self.classification.classify(&part_images)?;
        if let Some(ref mut p) = profiling {
            p.classification_duration = start.elapsed();
        }

        // Step 4: Rotation - Rotate images if needed (angle.index == 1 means 180°)
        let start = Instant::now();
        for (i, angle) in angles.iter().enumerate() {
            if angle.index == 1 {
                part_images[i] = utils::rotate_180(&part_images[i]);
            }
        }
        if let Some(ref mut p) = profiling {
            p.rotation_duration = start.elapsed();
        }

        // Step 5: Recognition - OCR text from rotated regions
        let start = Instant::now();
        let text_lines = self.recognition.recognize(&part_images)?;
        if let Some(ref mut p) = profiling {
            p.recognition_duration = start.elapsed();
        }

        // Step 6: Assembly - Combine TextBox + Angle + TextLine → TextCell
        let start = Instant::now();
        let mut text_cells = Vec::new();
        let mut cell_index = 0;
        for ((text_box, _angle), text_line) in
            text_boxes.iter().zip(angles.iter()).zip(text_lines.iter())
        {
            // Calculate overall confidence (average of character scores)
            let confidence = if text_line.char_scores.is_empty() {
                0.5 // Default if no character scores
            } else {
                text_line.char_scores.iter().sum::<f32>() / text_line.char_scores.len() as f32
            };

            // Step 7: Filter by text_score (Python: filter_result)
            // Reference: main.py:309-319, filter_result
            if confidence < params.text_score {
                continue;
            }

            // Convert TextBox corners to BoundingRectangle
            // Corners are clockwise from top-left: [TL, TR, BR, BL]
            let corners = &text_box.corners;

            let rect = crate::pipeline::data_structures::BoundingRectangle {
                r_x0: corners[0].0,
                r_y0: corners[0].1,
                r_x1: corners[1].0,
                r_y1: corners[1].1,
                r_x2: corners[2].0,
                r_y2: corners[2].1,
                r_x3: corners[3].0,
                r_y3: corners[3].1,
                coord_origin: crate::pipeline::data_structures::CoordOrigin::TopLeft,
            };

            text_cells.push(TextCell {
                index: cell_index,
                text: text_line.text.clone(),
                orig: text_line.text.clone(),
                confidence,
                from_ocr: true,
                rect,
            });
            cell_index += 1;
        }
        if let Some(ref mut p) = profiling {
            p.assembly_duration = start.elapsed();
        }

        Ok((text_cells, profiling))
    }

    /// Perform OCR on an image
    ///
    /// # Pipeline
    /// 1. Detection: Find text regions in image
    /// 2. Extraction: Crop detected text regions
    /// 3. Classification: Detect text rotation (0° or 180°)
    /// 4. Rotation: Rotate images if needed
    /// 5. Recognition: OCR text from rotated regions
    /// 6. Assembly: Combine results into `TextCell` objects
    ///
    /// # Arguments
    /// * `image` - Input image to process
    /// * `params` - OCR parameters (thresholds, etc.)
    ///
    /// # Returns
    /// Vector of `TextCell` objects with text, bounding boxes, and confidence scores
    #[must_use = "OCR detection returns results that should be processed"]
    pub fn detect(&mut self, image: &DynamicImage, params: &OcrParams) -> Result<Vec<TextCell>> {
        // Call detect_with_profiling without profiling enabled
        let (cells, _) = self.detect_with_profiling(image, params, false)?;
        Ok(cells)
    }
}

/// Pure Rust `RapidOCR` orchestrator (no `OpenCV` dependency)
///
/// Uses `DbNetPure` for text detection instead of the `OpenCV`-based `DbNet`.
/// This enables complete OCR pipeline without any `OpenCV`/C++ dependencies.
///
/// Integrates three ONNX models:
/// 1. `DbNetPure`: Text region detection (pure Rust postprocessing)
/// 2. `AngleNet`: Text rotation classification (0° or 180°)
/// 3. `CrnnNet`: Text recognition with CTC decoding
pub struct RapidOcrPure {
    detection: detection::DbNetPure,
    classification: classification::AngleNet,
    recognition: recognition::CrnnNet,
}

impl std::fmt::Debug for RapidOcrPure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RapidOcrPure")
            .field("detection", &self.detection)
            .field("classification", &self.classification)
            .field("recognition", &self.recognition)
            .finish()
    }
}

impl RapidOcrPure {
    /// Create new `RapidOcrPure` instance with models from directory (CPU backend)
    ///
    /// # Arguments
    /// * `models_dir` - Path to directory containing ONNX models
    ///
    /// # Models Required
    /// - `ch_PP-OCRv4_det_infer.onnx` - Detection model
    /// - `ch_ppocr_mobile_v2.0_cls_infer.onnx` - Classification model
    /// - `ch_PP-OCRv4_rec_infer.onnx` - Recognition model
    /// - `ppocr_keys_v1.txt` - Character dictionary (6622 characters)
    #[must_use = "this returns a Result that should be handled"]
    pub fn new(models_dir: &str) -> crate::error::Result<Self> {
        Self::new_with_backend(models_dir, false)
    }

    /// Create new `RapidOcrPure` instance with `CoreML` backend (Apple Neural Engine)
    ///
    /// On macOS with Apple Silicon, this enables hardware-accelerated inference
    /// via the Apple Neural Engine (ANE), providing 2-3x speedup over CPU.
    ///
    /// # Arguments
    /// * `models_dir` - Path to directory containing ONNX models
    ///
    /// # Models Required
    /// - `ch_PP-OCRv4_det_infer.onnx` - Detection model
    /// - `ch_ppocr_mobile_v2.0_cls_infer.onnx` - Classification model
    /// - `ch_PP-OCRv4_rec_infer.onnx` - Recognition model
    /// - `ppocr_keys_v1.txt` - Character dictionary (6622 characters)
    pub fn new_with_coreml(models_dir: &str) -> crate::error::Result<Self> {
        Self::new_with_backend(models_dir, true)
    }

    /// Create new `RapidOcrPure` instance with specified backend
    ///
    /// # Arguments
    /// * `models_dir` - Path to directory containing ONNX models
    /// * `use_coreml` - If true, use `CoreML` execution provider (macOS ANE acceleration)
    fn new_with_backend(models_dir: &str, use_coreml: bool) -> crate::error::Result<Self> {
        let detection = if use_coreml {
            detection::DbNetPure::new_with_coreml(&format!(
                "{models_dir}/ch_PP-OCRv4_det_infer.onnx"
            ))?
        } else {
            detection::DbNetPure::new(&format!("{models_dir}/ch_PP-OCRv4_det_infer.onnx"))?
        };

        let classification = if use_coreml {
            classification::AngleNet::new_with_coreml(&format!(
                "{models_dir}/ch_ppocr_mobile_v2.0_cls_infer.onnx"
            ))?
        } else {
            classification::AngleNet::new(&format!(
                "{models_dir}/ch_ppocr_mobile_v2.0_cls_infer.onnx"
            ))?
        };

        let recognition = if use_coreml {
            recognition::CrnnNet::new_with_coreml(
                &format!("{models_dir}/ch_PP-OCRv4_rec_infer.onnx"),
                &format!("{models_dir}/ppocr_keys_v1.txt"),
            )?
        } else {
            recognition::CrnnNet::new(
                &format!("{models_dir}/ch_PP-OCRv4_rec_infer.onnx"),
                &format!("{models_dir}/ppocr_keys_v1.txt"),
            )?
        };

        Ok(Self {
            detection,
            classification,
            recognition,
        })
    }

    /// Perform OCR on an image with optional stage-level profiling
    ///
    /// # Pipeline
    /// 1. Detection: Find text regions in image (`DbNetPure` - pure Rust)
    /// 2. Extraction: Crop detected text regions
    /// 3. Classification: Detect text rotation (0° or 180°)
    /// 4. Rotation: Rotate images if needed
    /// 5. Recognition: OCR text from rotated regions
    /// 6. Assembly: Combine results into `TextCell` objects
    pub fn detect_with_profiling(
        &mut self,
        image: &image::DynamicImage,
        params: &types::OcrParams,
        enable_profiling: bool,
    ) -> crate::error::Result<(Vec<types::TextCell>, Option<OcrProfiling>)> {
        use std::time::Instant;

        let mut profiling = if enable_profiling {
            Some(OcrProfiling::default())
        } else {
            None
        };

        // Step 1: Detection - Find text boxes (pure Rust)
        let start = Instant::now();
        let text_boxes = self.detection.detect(image, &params.detection)?;
        if let Some(ref mut p) = profiling {
            p.detection_duration = start.elapsed();
        }

        if text_boxes.is_empty() {
            return Ok((Vec::new(), profiling));
        }

        // Step 2: Extract part images - Crop detected text regions
        let start = Instant::now();
        let mut part_images: Vec<image::DynamicImage> = text_boxes
            .iter()
            .map(|text_box| utils::crop_text_box(image, text_box))
            .collect();
        if let Some(ref mut p) = profiling {
            p.crop_duration = start.elapsed();
        }

        // Step 3: Classification - Detect text rotation (0° or 180°)
        let start = Instant::now();
        let angles = self.classification.classify(&part_images)?;
        if let Some(ref mut p) = profiling {
            p.classification_duration = start.elapsed();
        }

        // Step 4: Rotation - Rotate images if needed (angle.index == 1 means 180°)
        let start = Instant::now();
        for (i, angle) in angles.iter().enumerate() {
            if angle.index == 1 {
                part_images[i] = utils::rotate_180(&part_images[i]);
            }
        }
        if let Some(ref mut p) = profiling {
            p.rotation_duration = start.elapsed();
        }

        // Step 5: Recognition - OCR text from rotated regions
        let start = Instant::now();
        let text_lines = self.recognition.recognize(&part_images)?;
        if let Some(ref mut p) = profiling {
            p.recognition_duration = start.elapsed();
        }

        // Step 6: Assembly - Combine TextBox + Angle + TextLine → TextCell
        let start = Instant::now();
        let mut text_cells = Vec::new();
        let mut cell_index = 0;
        for ((text_box, _angle), text_line) in
            text_boxes.iter().zip(angles.iter()).zip(text_lines.iter())
        {
            // Calculate overall confidence (average of character scores)
            let confidence = if text_line.char_scores.is_empty() {
                0.5 // Default if no character scores
            } else {
                text_line.char_scores.iter().sum::<f32>() / text_line.char_scores.len() as f32
            };

            // Filter by text_score
            if confidence < params.text_score {
                continue;
            }

            // Convert TextBox corners to BoundingRectangle
            // Corners are clockwise from top-left: [TL, TR, BR, BL]
            let corners = &text_box.corners;

            let rect = crate::pipeline::data_structures::BoundingRectangle {
                r_x0: corners[0].0,
                r_y0: corners[0].1,
                r_x1: corners[1].0,
                r_y1: corners[1].1,
                r_x2: corners[2].0,
                r_y2: corners[2].1,
                r_x3: corners[3].0,
                r_y3: corners[3].1,
                coord_origin: crate::pipeline::data_structures::CoordOrigin::TopLeft,
            };

            text_cells.push(types::TextCell {
                index: cell_index,
                text: text_line.text.clone(),
                orig: text_line.text.clone(),
                confidence,
                from_ocr: true,
                rect,
            });
            cell_index += 1;
        }
        if let Some(ref mut p) = profiling {
            p.assembly_duration = start.elapsed();
        }

        Ok((text_cells, profiling))
    }

    /// Perform OCR on an image
    ///
    /// # Arguments
    /// * `image` - Input image to process
    /// * `params` - OCR parameters (thresholds, etc.)
    ///
    /// # Returns
    /// Vector of `TextCell` objects with text, bounding boxes, and confidence scores
    pub fn detect(
        &mut self,
        image: &image::DynamicImage,
        params: &types::OcrParams,
    ) -> crate::error::Result<Vec<types::TextCell>> {
        let (cells, _) = self.detect_with_profiling(image, params, false)?;
        Ok(cells)
    }
}

#[cfg(all(test, feature = "opencv-preprocessing"))]
mod tests {
    use super::*;

    #[test]
    fn test_rapidocr_loading() {
        // Test that models can be loaded successfully
        let ocr = RapidOcr::new("models/rapidocr");
        assert!(
            ocr.is_ok(),
            "Failed to load RapidOCR models: {:?}",
            ocr.err()
        );
    }

    #[test]
    fn test_rapidocr_pipeline() {
        // Test full pipeline integration (DbNet → AngleNet → CrnnNet)
        use image::{DynamicImage, RgbImage};
        use log;

        // Create a simple test image (100x50 white background)
        let img = RgbImage::from_pixel(100, 50, image::Rgb([255, 255, 255]));
        let test_image = DynamicImage::ImageRgb8(img);

        // Load RapidOCR models
        let mut ocr = RapidOcr::new("models/rapidocr").expect("Failed to load RapidOCR models");

        // Run pipeline
        let params = OcrParams::default();
        let result = ocr.detect(&test_image, &params);

        // Should not error (even if no text detected)
        assert!(result.is_ok(), "Pipeline failed: {:?}", result.err());

        // For blank image, expect 0 text cells
        let text_cells = result.unwrap();
        log::debug!("Detected {} text cells", text_cells.len());
    }
}

#[cfg(test)]
mod tests_pure {
    use super::*;

    #[test]
    fn test_rapidocr_pure_loading() {
        // Test that models can be loaded successfully (pure Rust version)
        let ocr = RapidOcrPure::new("models/rapidocr");
        assert!(
            ocr.is_ok(),
            "Failed to load RapidOcrPure models: {:?}",
            ocr.err()
        );
    }

    #[test]
    fn test_rapidocr_pure_pipeline() {
        // Test full pipeline integration (DbNetPure → AngleNet → CrnnNet)
        use image::{DynamicImage, RgbImage};

        // Create a simple test image (100x50 white background)
        let img = RgbImage::from_pixel(100, 50, image::Rgb([255, 255, 255]));
        let test_image = DynamicImage::ImageRgb8(img);

        // Load RapidOcrPure models
        let mut ocr =
            RapidOcrPure::new("models/rapidocr").expect("Failed to load RapidOcrPure models");

        // Run pipeline
        let params = types::OcrParams::default();
        let result = ocr.detect(&test_image, &params);

        // Should not error (even if no text detected)
        assert!(result.is_ok(), "Pipeline failed: {:?}", result.err());

        // For blank image, expect 0 text cells
        let text_cells = result.unwrap();
        log::debug!("Detected {} text cells", text_cells.len());
    }
}
