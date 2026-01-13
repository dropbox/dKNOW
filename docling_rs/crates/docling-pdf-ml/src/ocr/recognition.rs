// CrnnNet - Text recognition model with CTC decoding
//
// Reference: CrnnNet.cpp, CrnnNet.h from RapidOcrOnnx
// Model: ch_PP-OCRv4_rec_infer.onnx
// Dictionary: ppocr_keys_v1.txt (6622 characters)

// Intentional ML conversions: tensor indices, image dimensions
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]

use crate::error::Result;
#[cfg(test)]
use crate::ocr::types::OCR_NORMALIZE_DIVISOR;
use crate::ocr::types::{TextLine, OCR_MODEL_HEIGHT};
use image::{DynamicImage, GenericImageView};
use ort::execution_providers::{CPUExecutionProvider, CoreMLExecutionProvider};
use ort::session::{builder::GraphOptimizationLevel, Session};
use ort::value::Value;
use std::fs;
use std::time::{Duration, Instant};

/// Internal profiling data for `CrnnNet` recognition
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CrnnNetProfiling {
    /// Time spent on preprocessing (image to tensor conversion, resize, normalize, pad)
    pub preprocessing_duration: Duration,
    /// Time spent on ONNX inference (CNN + LSTM forward pass)
    pub inference_duration: Duration,
    /// Time spent on CTC decoding (logits to text string)
    pub decoding_duration: Duration,
}

impl CrnnNetProfiling {
    #[inline]
    #[must_use = "returns the total duration sum"]
    pub fn total(&self) -> Duration {
        self.preprocessing_duration + self.inference_duration + self.decoding_duration
    }
}

/// `CrnnNet` text recognition model
///
/// Recognizes text from cropped text regions using CTC (Connectionist Temporal Classification).
///
/// Reference: CrnnNet.cpp
pub struct CrnnNet {
    session: Session,
    keys: Vec<String>,
}

impl std::fmt::Debug for CrnnNet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CrnnNet")
            .field("session", &"<Session>")
            .field("keys_count", &self.keys.len())
            .finish()
    }
}

impl CrnnNet {
    /// Load `CrnnNet` model and character dictionary (CPU backend)
    ///
    /// # Arguments
    /// * `model_path` - Path to `ch_PP-OCRv4_rec_infer.onnx`
    /// * `keys_path` - Path to `ppocr_keys_v1.txt` (character dictionary)
    ///
    /// # Character Dictionary
    /// Contains 6622 characters (Chinese + English + symbols).
    /// Special tokens:
    /// - Index 0: Blank token (CTC)
    /// - Last index: Space token
    #[must_use = "this returns a Result that should be handled"]
    pub fn new(model_path: &str, keys_path: &str) -> Result<Self> {
        Self::new_with_backend(model_path, keys_path, false)
    }

    /// Load `CrnnNet` model with `CoreML` backend (Apple Neural Engine acceleration)
    ///
    /// On macOS with Apple Silicon, this enables hardware-accelerated inference
    /// via the Apple Neural Engine (ANE), providing 2-3x speedup over CPU.
    ///
    /// # Arguments
    /// * `model_path` - Path to `ch_PP-OCRv4_rec_infer.onnx`
    /// * `keys_path` - Path to `ppocr_keys_v1.txt` (character dictionary)
    #[must_use = "this returns a Result that should be handled"]
    pub fn new_with_coreml(model_path: &str, keys_path: &str) -> Result<Self> {
        Self::new_with_backend(model_path, keys_path, true)
    }

    /// Load `CrnnNet` model with specified backend
    ///
    /// # Arguments
    /// * `model_path` - Path to `ch_PP-OCRv4_rec_infer.onnx`
    /// * `keys_path` - Path to `ppocr_keys_v1.txt` (character dictionary)
    /// * `use_coreml` - If true, use `CoreML` execution provider (macOS ANE acceleration)
    fn new_with_backend(model_path: &str, keys_path: &str, use_coreml: bool) -> Result<Self> {
        // Enable Level3 optimizations: includes all optimizations up to Level3
        // - Level1: Constant folding, redundant node elimination, node fusion
        // - Level2: Complex node fusions (GEMM, Matmul, Conv, BERT optimizations)
        // - Level3: Memory layout optimizations (NCHWc for spatial locality)
        //
        // Thread configuration:
        // - intra_threads: Parallelism within operators (CPU ops can use multiple threads)
        // - Using num_cpus() to maximize CPU utilization for LSTM processing
        let num_threads = num_cpus::get();

        let session = if use_coreml {
            log::debug!("Creating CrnnNet session with CoreML execution provider");
            Session::builder()?
                .with_optimization_level(GraphOptimizationLevel::Level3)?
                .with_intra_threads(num_threads)?
                .with_execution_providers([
                    CoreMLExecutionProvider::default().build(),
                    CPUExecutionProvider::default().build(), // Fallback
                ])?
                .commit_from_file(model_path)?
        } else {
            log::debug!("Creating CrnnNet session with CPU execution provider");
            Session::builder()?
                .with_optimization_level(GraphOptimizationLevel::Level3)?
                .with_intra_threads(num_threads)?
                .commit_from_file(model_path)?
        };

        // Load character dictionary
        let keys_content = fs::read_to_string(keys_path)?;

        let mut keys: Vec<String> = keys_content.lines().map(ToString::to_string).collect();

        // Insert special tokens
        // Reference: CrnnNet.cpp:75-78
        keys.insert(0, "#".to_string()); // Blank token for CTC
        keys.push(" ".to_string()); // Space token

        Ok(Self { session, keys })
    }

    /// Recognize text from multiple cropped images using batch processing
    ///
    /// # Pipeline (Reference: CrnnNet.cpp)
    /// 1. Preprocess: Resize to standard height + Normalize (getTextLine)
    /// 2. ONNX Inference: Get character probabilities `[batch, timesteps, vocab_size]`
    /// 3. CTC Decode: Argmax + skip blanks/repeats (scoreToTextLine, lines 98-127)
    ///
    /// # Arguments
    /// * `images` - Cropped text region images (already rotated if needed)
    ///
    /// # Returns
    /// Vector of `TextLine` objects with recognized text and per-character confidence scores
    ///
    /// # Batch Processing
    /// Processes all images in a single ONNX inference call for significant speedup.
    /// Expected: 6-15x faster than sequential processing.
    #[must_use = "text recognition returns results that should be processed"]
    pub fn recognize(&mut self, images: &[DynamicImage]) -> Result<Vec<TextLine>> {
        if images.is_empty() {
            return Ok(Vec::new());
        }

        // Use batch processing for multiple images
        if images.len() > 1 {
            self.recognize_batch(images, false)
                .map(|(text_lines, _)| text_lines)
        } else {
            // Single image: use sequential path (no batching overhead)
            let (text_line, _) = self.recognize_single_with_profiling(&images[0], false)?;
            Ok(vec![text_line])
        }
    }

    /// Recognize text from multiple cropped images with profiling
    ///
    /// # Arguments
    /// * `images` - Cropped text region images (already rotated if needed)
    /// * `enable_profiling` - If true, collect timing information for each stage
    ///
    /// # Returns
    /// Vector of `TextLine` objects and optional profiling data (aggregated across all images)
    ///
    /// # Batch Processing
    /// Uses batch inference when `images.len()` > 1 for significant speedup.
    pub fn recognize_with_profiling(
        &mut self,
        images: &[DynamicImage],
        enable_profiling: bool,
    ) -> Result<(Vec<TextLine>, Option<CrnnNetProfiling>)> {
        if images.is_empty() {
            return Ok((Vec::new(), None));
        }

        // Use batch processing for multiple images
        if images.len() > 1 {
            self.recognize_batch(images, enable_profiling)
        } else {
            // Single image: use sequential path
            let (text_line, profiling) =
                self.recognize_single_with_profiling(&images[0], enable_profiling)?;
            Ok((vec![text_line], profiling))
        }
    }

    /// Recognize text from multiple images using batch processing
    ///
    /// # Batch Processing Strategy
    /// 1. Preprocess all images individually (each gets resized to height=48, proportional width)
    /// 2. Find max width across all preprocessed images
    /// 3. Pad all images to max width (zero-padding on right)
    /// 4. Stack into single batch tensor `[N, 3, 48, max_width]`
    /// 5. Single ONNX inference call (amortizes overhead)
    /// 6. Split output tensor `[N, timesteps, vocab_size]` back to individual results
    /// 7. CTC decode each result separately
    ///
    /// Expected speedup: 6-15x for batch size 19 (from N=141 profiling)
    fn recognize_batch(
        &mut self,
        images: &[DynamicImage],
        enable_profiling: bool,
    ) -> Result<(Vec<TextLine>, Option<CrnnNetProfiling>)> {
        use crate::preprocessing::rapidocr::rapidocr_rec_preprocess;

        let preprocessing_start = enable_profiling.then(Instant::now);

        // Step 1: Preprocess all images individually
        // Each image gets resized to height=48 with proportional width, normalized, converted to CHW
        let mut preprocessed_images = Vec::with_capacity(images.len());
        let mut max_width = 0;

        for image in images {
            // Convert DynamicImage to Array3<u8>
            let rgb_image = image.to_rgb8();
            let (width, height) = (rgb_image.width() as usize, rgb_image.height() as usize);

            let mut image_array = ndarray::Array3::<u8>::zeros((height, width, 3));
            for y in 0..height {
                for x in 0..width {
                    let pixel = rgb_image.get_pixel(x as u32, y as u32);
                    image_array[[y, x, 0]] = pixel[0];
                    image_array[[y, x, 1]] = pixel[1];
                    image_array[[y, x, 2]] = pixel[2];
                }
            }

            // Preprocess: Returns (3, 48, width) where width <= 320 after padding
            let normalized = rapidocr_rec_preprocess(&image_array);
            let actual_width = normalized.shape()[2];
            max_width = max_width.max(actual_width);
            preprocessed_images.push(normalized);
        }

        // Step 2: Create batch tensor [N, 3, OCR_MODEL_HEIGHT, max_width]
        // Pad all images to max_width (they're already padded to 320, but we need consistent width)
        let batch_size = images.len();
        let mut batch_tensor =
            ndarray::Array4::<f32>::zeros((batch_size, 3, OCR_MODEL_HEIGHT, max_width));

        for (i, preprocessed) in preprocessed_images.iter().enumerate() {
            let width = preprocessed.shape()[2];
            // Copy preprocessed image into batch tensor
            // preprocessed is (3, OCR_MODEL_HEIGHT, width), we copy into batch_tensor[i, :, :, :width]
            for c in 0..3 {
                for h in 0..OCR_MODEL_HEIGHT {
                    for w in 0..width {
                        batch_tensor[[i, c, h, w]] = preprocessed[[c, h, w]];
                    }
                }
            }
            // Right portion already zero-padded (if width < max_width)
        }

        let preprocessing_duration = preprocessing_start.map_or(Duration::ZERO, |s| s.elapsed());

        let inference_start = enable_profiling.then(Instant::now);

        // Step 3: ONNX Inference (single call for entire batch)
        let (timesteps, vocab_size, output_data) = {
            let shape = batch_tensor.shape().to_vec();
            let data = batch_tensor
                .as_standard_layout()
                .as_slice()
                .unwrap()
                .to_vec();
            let input_value = Value::from_array((shape.as_slice(), data))?;

            let outputs = self.session.run(ort::inputs!["x" => input_value])?;
            let output_tensor = outputs[0].try_extract_tensor::<f32>()?;
            let output_shape = output_tensor.0;
            let output_data_slice = output_tensor.1;

            let timesteps = output_shape[1] as usize;
            let vocab_size = output_shape[2] as usize;

            // Copy data before dropping outputs
            let output_data = output_data_slice.to_vec();

            (timesteps, vocab_size, output_data)
        };

        let inference_duration = inference_start.map_or(Duration::ZERO, |s| s.elapsed());

        let decoding_start = enable_profiling.then(Instant::now);

        // Step 4: Split output tensor and CTC decode each result
        // Output shape: [batch_size, timesteps, vocab_size]
        // Each image's output is at output_data[i * timesteps * vocab_size .. (i+1) * timesteps * vocab_size]
        let mut text_lines = Vec::with_capacity(batch_size);
        let stride = timesteps * vocab_size;

        for i in 0..batch_size {
            let start_idx = i * stride;
            let end_idx = (i + 1) * stride;
            let image_output = &output_data[start_idx..end_idx];
            let text_line = self.decode_ctc(image_output, timesteps, vocab_size);
            text_lines.push(text_line);
        }

        let decoding_duration = decoding_start.map_or(Duration::ZERO, |s| s.elapsed());

        let profiling = enable_profiling.then_some(CrnnNetProfiling {
            preprocessing_duration,
            inference_duration,
            decoding_duration,
        });

        Ok((text_lines, profiling))
    }

    /// Recognize text from a single image with optional profiling
    ///
    /// Reference: CrnnNet.cpp:125-148 (getTextLine)
    /// Uses validated preprocessing from src/preprocessing/rapidocr.rs
    fn recognize_single_with_profiling(
        &mut self,
        image: &DynamicImage,
        enable_profiling: bool,
    ) -> Result<(TextLine, Option<CrnnNetProfiling>)> {
        use crate::preprocessing::rapidocr::rapidocr_rec_preprocess;

        let preprocessing_start = enable_profiling.then(Instant::now);

        // Step 1: Convert DynamicImage to Array3<u8> for preprocessing
        let rgb_image = image.to_rgb8();
        let (width, height) = (rgb_image.width() as usize, rgb_image.height() as usize);

        let mut image_array = ndarray::Array3::<u8>::zeros((height, width, 3));
        for y in 0..height {
            for x in 0..width {
                let pixel = rgb_image.get_pixel(x as u32, y as u32);
                image_array[[y, x, 0]] = pixel[0];
                image_array[[y, x, 1]] = pixel[1];
                image_array[[y, x, 2]] = pixel[2];
            }
        }

        // Step 2: Use validated preprocessing (passes Phase 2 tests with < 0.02 diff)
        // This function:
        // - Resizes to height 48, width proportional (max 320)
        // - Uses OpenCV INTER_LINEAR (exact match to Python cv2.resize)
        // - Normalizes: (pixel / 255.0 - 0.5) / 0.5
        // - Converts HWC → CHW
        // - Zero-pads width to 320
        let normalized = rapidocr_rec_preprocess(&image_array); // Returns (3, 48, 320)

        // Step 3: Prepare for ONNX Inference
        // Add batch dimension: (3, 48, 320) → (1, 3, 48, 320)
        let normalized_4d = normalized.insert_axis(ndarray::Axis(0));

        let preprocessing_duration = preprocessing_start.map_or(Duration::ZERO, |s| s.elapsed());

        let inference_start = enable_profiling.then(Instant::now);

        // Step 4: ONNX Inference
        // Reference: CrnnNet.cpp:139-140
        // Convert ndarray to (shape, data) tuple for ONNX Runtime
        let shape = normalized_4d.shape().to_vec();
        let data = normalized_4d
            .as_standard_layout()
            .as_slice()
            .unwrap()
            .to_vec();
        let input_value = Value::from_array((shape.as_slice(), data))?;

        // Step 5: Extract output tensor and copy data
        // Reference: CrnnNet.cpp:142-146
        // Output shape: [1, timesteps, vocab_size]
        let (timesteps, vocab_size, output_data) = {
            let outputs = self.session.run(ort::inputs!["x" => input_value])?;
            let output_tensor = outputs[0].try_extract_tensor::<f32>()?;
            let output_shape = output_tensor.0;
            let output_data_slice = output_tensor.1;

            let timesteps = output_shape[1] as usize;
            let vocab_size = output_shape[2] as usize;

            // Copy data before dropping outputs (to avoid borrow checker issues)
            let output_data = output_data_slice.to_vec();

            (timesteps, vocab_size, output_data)
        };

        let inference_duration = inference_start.map_or(Duration::ZERO, |s| s.elapsed());

        let decoding_start = enable_profiling.then(Instant::now);

        // Step 6: CTC Decoding
        // Reference: CrnnNet.cpp:147 (scoreToTextLine)
        let text_line = self.decode_ctc(&output_data, timesteps, vocab_size);

        let decoding_duration = decoding_start.map_or(Duration::ZERO, |s| s.elapsed());

        let profiling = enable_profiling.then_some(CrnnNetProfiling {
            preprocessing_duration,
            inference_duration,
            decoding_duration,
        });

        Ok((text_line, profiling))
    }

    /// Add zero-padding to match Python rapidocr behavior
    ///
    /// Python code (ch_ppocr_rec/main.py:167-169):
    /// ```python
    /// padding_im = np.zeros((img_channel, img_height, img_width), dtype=np.float32)
    /// padding_im[:, :, 0:resized_w] = resized_image
    /// ```
    ///
    /// Creates a zero-padded tensor `[1, 3, 48, max_width]` and copies the resized
    /// image into the left portion. This ensures consistent tensor shapes for ONNX.
    // Method signature kept for API consistency with other TextRecognizer methods
    #[allow(dead_code, reason = "Prepared for future ONNX OCR batch processing")]
    #[allow(clippy::unused_self)]
    fn add_zero_padding(
        &self,
        normalized: ndarray::Array4<f32>,
        actual_width: u32,
        max_width: u32,
    ) -> ndarray::Array4<f32> {
        use ndarray::Array4;

        let (batch, channels, height, current_width) = normalized.dim();

        // If already at max width or wider, no padding needed
        if current_width >= max_width as usize {
            return normalized;
        }

        // Create zero-padded tensor
        let mut padded = Array4::<f32>::zeros((batch, channels, height, max_width as usize));

        // Copy resized image into left portion
        // Python: padding_im[:, :, 0:resized_w] = resized_image
        // Use current_width from normalized array, not actual_width parameter
        for b in 0..batch {
            for c in 0..channels {
                for h in 0..height {
                    for w in 0..current_width {
                        padded[[b, c, h, w]] = normalized[[b, c, h, w]];
                    }
                }
            }
        }

        padded
    }

    /// Normalize image for `CrnnNet`
    ///
    /// Reference: OcrUtils.cpp:substractMeanNormalize
    /// Formula: (pixel * norm - mean * norm)
    ///
    /// This differs from `DbNet` normalization!
    /// - `CrnnNet`: mean=127.5, norm=1/127.5 → (pixel - 127.5) / 127.5
    /// - `DbNet`: `ImageNet` mean/std
    // Method signature kept for API consistency with other TextRecognizer methods
    #[allow(
        dead_code,
        reason = "Alternative normalization method for CrnnNet compatibility"
    )]
    #[allow(clippy::unused_self)]
    fn normalize_crnn(&self, image: &DynamicImage, mean: f32, norm: f32) -> ndarray::Array4<f32> {
        use ndarray::Array4;

        let (width, height) = image.dimensions();
        let rgb_image = image.to_rgb8();

        // Create tensor [1, 3, H, W] in NCHW format
        let mut tensor = Array4::<f32>::zeros((1, 3, height as usize, width as usize));

        // Reference: OcrUtils.cpp:substractMeanNormalize
        // data = pixel * normVals[ch] - meanVals[ch] * normVals[ch]
        for y in 0..height {
            for x in 0..width {
                let pixel = rgb_image.get_pixel(x, y);
                for c in 0..3 {
                    let data = norm.mul_add(f32::from(pixel[c]), -(mean * norm));
                    tensor[[0, c, y as usize, x as usize]] = data;
                }
            }
        }

        tensor
    }

    /// CTC Decoding: Convert probabilities to text
    ///
    /// Reference: CrnnNet.cpp:98-122 (scoreToTextLine)
    ///
    /// Algorithm:
    /// 1. For each timestep, find argmax (character with highest probability)
    /// 2. Skip blank tokens (index 0)
    /// 3. Skip repeated characters (same as previous timestep)
    /// 4. Accumulate character scores for confidence
    fn decode_ctc(&self, output_data: &[f32], timesteps: usize, vocab_size: usize) -> TextLine {
        let mut text = String::new();
        let mut char_scores = Vec::new();
        let mut last_index = 0;

        // Reference: CrnnNet.cpp:107-121
        for t in 0..timesteps {
            let start = t * vocab_size;
            let end = (t + 1) * vocab_size;

            // Find argmax for this timestep
            let (max_index, max_value) = output_data[start..end]
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.total_cmp(b))
                .unwrap();

            // CTC decoding rules:
            // 1. Skip blank token (index 0)
            // 2. Skip if same as previous character (repeated)
            // 3. Must be valid character index
            // Reference: CrnnNet.cpp:116
            if max_index > 0 && max_index < self.keys.len() && !(t > 0 && max_index == last_index) {
                char_scores.push(*max_value);
                text.push_str(&self.keys[max_index]);
            }

            last_index = max_index;
        }

        TextLine { text, char_scores }
    }

    /// Get number of characters in dictionary (including special tokens)
    #[inline]
    #[must_use = "vocabulary size is computed but not used"]
    pub fn vocab_size(&self) -> usize {
        self.keys.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage};
    use ort::session::Session;

    #[test]
    fn test_crnnnet_loading() {
        let model = CrnnNet::new(
            "models/rapidocr/ch_PP-OCRv4_rec_infer.onnx",
            "models/rapidocr/ppocr_keys_v1.txt",
        );
        assert!(
            model.is_ok(),
            "Failed to load CrnnNet model: {:?}",
            model.err()
        );

        // Verify character dictionary loaded correctly (6622 chars + 2 special tokens)
        let model = model.unwrap();
        assert!(
            model.vocab_size() >= 6600,
            "Expected at least 6600 characters, got {}",
            model.vocab_size()
        );
        assert!(
            model.vocab_size() <= 6700,
            "Expected at most 6700 characters, got {}",
            model.vocab_size()
        );
    }

    #[test]
    fn test_ctc_decoding() {
        // Create a simple CrnnNet with minimal dictionary for testing
        // In real usage, this would load from file
        let session = Session::builder()
            .unwrap()
            .commit_from_file("models/rapidocr/ch_PP-OCRv4_rec_infer.onnx")
            .unwrap();

        let keys = vec![
            "#".to_string(), // 0: blank
            "a".to_string(), // 1
            "b".to_string(), // 2
            "c".to_string(), // 3
            " ".to_string(), // 4: space
        ];

        let crnn = CrnnNet { session, keys };

        // Simulate CTC output probabilities
        // 5 timesteps, 5 vocab items
        // Sequence: blank, a, a, b, blank → should decode to "ab"
        #[rustfmt::skip]
        let output_data = vec![
            // t=0: blank (index 0 has highest prob)
            0.9, 0.02, 0.03, 0.03, 0.02,
            // t=1: a (index 1 has highest prob)
            0.1, 0.8, 0.05, 0.03, 0.02,
            // t=2: a again (repeated, should be skipped)
            0.1, 0.7, 0.1, 0.05, 0.05,
            // t=3: b (index 2 has highest prob)
            0.1, 0.1, 0.7, 0.05, 0.05,
            // t=4: blank
            0.8, 0.05, 0.05, 0.05, 0.05,
        ];

        let text_line = crnn.decode_ctc(&output_data, 5, 5);

        assert_eq!(text_line.text, "ab");
        assert_eq!(text_line.char_scores.len(), 2);
        assert!((text_line.char_scores[0] - 0.8).abs() < 0.01);
        assert!((text_line.char_scores[1] - 0.7).abs() < 0.01);
    }

    #[test]
    fn test_normalize_crnn() {
        let model = CrnnNet::new(
            "models/rapidocr/ch_PP-OCRv4_rec_infer.onnx",
            "models/rapidocr/ppocr_keys_v1.txt",
        )
        .unwrap();

        // Create a simple test image
        let mut img = RgbImage::new(100, OCR_MODEL_HEIGHT as u32);
        for y in 0..OCR_MODEL_HEIGHT {
            for x in 0..100 {
                img.put_pixel(x as u32, y as u32, Rgb([128, 128, 128]));
            }
        }

        let dynamic_img = DynamicImage::ImageRgb8(img);
        let tensor = model.normalize_crnn(
            &dynamic_img,
            OCR_NORMALIZE_DIVISOR,
            1.0 / OCR_NORMALIZE_DIVISOR,
        );

        // Check shape
        assert_eq!(tensor.shape(), &[1, 3, OCR_MODEL_HEIGHT, 100]);

        // Check normalization: (128 - 127.5) / 127.5 ≈ 0.0039
        let expected = 128.0 * (1.0 / OCR_NORMALIZE_DIVISOR)
            - OCR_NORMALIZE_DIVISOR * (1.0 / OCR_NORMALIZE_DIVISOR);
        for c in 0..3 {
            for y in 0..OCR_MODEL_HEIGHT {
                for x in 0..100 {
                    let val = tensor[[0, c, y, x]];
                    assert!(
                        (val - expected).abs() < 0.01,
                        "Expected ~{expected}, got {val}"
                    );
                }
            }
        }
    }

    #[test]
    fn test_crnnnet_recognize() {
        let mut model = CrnnNet::new(
            "models/rapidocr/ch_PP-OCRv4_rec_infer.onnx",
            "models/rapidocr/ppocr_keys_v1.txt",
        )
        .unwrap();

        // Create a simple test image (white text on black background)
        // This won't produce meaningful text, but tests the pipeline
        let mut img = RgbImage::new(200, 50);
        for y in 0..50 {
            for x in 0..200 {
                // Black background
                img.put_pixel(x, y, Rgb([0, 0, 0]));
            }
        }

        // Add some white "text" regions
        for y in 10..40 {
            for x in 20..80 {
                img.put_pixel(x, y, Rgb([255, 255, 255]));
            }
        }

        let dynamic_img = DynamicImage::ImageRgb8(img);
        let images = vec![dynamic_img];

        let result = model.recognize(&images);
        assert!(result.is_ok(), "Recognition failed: {:?}", result.err());

        let text_lines = result.unwrap();
        assert_eq!(text_lines.len(), 1);

        // The text content is unpredictable for synthetic images,
        // but we can verify the structure
        let text_line = &text_lines[0];
        // Text should have some output (even if nonsense)
        // Char scores should match text length (each visible character has a score)
        assert_eq!(text_line.text.chars().count(), text_line.char_scores.len());

        // All scores should be in [0, 1] range
        for score in &text_line.char_scores {
            assert!(
                *score >= 0.0 && *score <= 1.0,
                "Score out of range: {score}"
            );
        }
    }
}
