// DbNet - Text detection model
//
// Reference: DbNet.cpp, DbNet.h from RapidOcrOnnx
// Model: ch_PP-OCRv4_det_infer.onnx

// Image dimensions and coordinates use various numeric types.
// Conversions are safe for practical image sizes (< 10000 pixels).
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]

// Common imports (used by both DbNet and DbNetPure)
use crate::error::Result;
use crate::ocr::types::{DetectionParams, TextBox};
#[cfg(feature = "opencv-preprocessing")]
use crate::ocr::types::{OCR_LONG_SIDE_THRESH, OCR_MAX_CANDIDATES};
use image::{DynamicImage, GenericImageView};
use ndarray::Array4;
use ort::execution_providers::{CPUExecutionProvider, CoreMLExecutionProvider};
use ort::session::{builder::GraphOptimizationLevel, Session};
use std::time::{Duration, Instant};

// OpenCV-specific imports (only for DbNet)
#[cfg(feature = "opencv-preprocessing")]
use geo::{Area, Coord, EuclideanLength, LineString, Polygon};
#[cfg(feature = "opencv-preprocessing")]
use opencv::{
    core::{self, DataType, Mat, Point, Point2f, Scalar, Size, Vector, BORDER_CONSTANT},
    imgproc,
    prelude::{MatExprTraitConst, MatTrait, MatTraitConst},
};

// =============================================================================
// OCR Image Normalization Constants
// =============================================================================

/// Scale factor for pixel normalization (1/255).
///
/// Converts 8-bit pixel values [0, 255] to float range [0, 1].
/// Standard preprocessing for PaddleOCR/RapidOCR detection models.
const OCR_PIXEL_SCALE: f32 = 1.0 / 255.0;

/// Mean value for image normalization.
///
/// `PaddleOCR` uses mean=[0.5, 0.5, 0.5] for all channels.
/// Formula: (pixel * scale - mean) / std
const OCR_NORMALIZE_MEAN: f32 = 0.5;

/// Standard deviation for image normalization.
///
/// `PaddleOCR` uses std=[0.5, 0.5, 0.5] for all channels.
/// Final normalized range: [-1, 1]
const OCR_NORMALIZE_STD: f32 = 0.5;

/// Internal profiling data for `DbNet` detection
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DbNetProfiling {
    /// Time spent on preprocessing (resize, normalize, tensor conversion)
    pub preprocessing_duration: Duration,
    /// Time spent on ONNX inference (detection map generation)
    pub inference_duration: Duration,
    /// Time spent on postprocessing (threshold, dilate, contours, unclip, scoring)
    pub postprocessing_duration: Duration,
}

impl DbNetProfiling {
    #[inline]
    #[must_use = "returns the total duration sum"]
    pub fn total(&self) -> Duration {
        self.preprocessing_duration + self.inference_duration + self.postprocessing_duration
    }
}

/// Round using banker's rounding (round half to even)
///
/// Python `numpy` uses this: `np.round(0.5)` = 0, `np.round(1.5)` = 2, `np.round(2.5)` = 2
/// Standard Rust `round()` uses round-half-away-from-zero: 0.5 → 1, 1.5 → 2, 2.5 → 3
#[inline]
#[must_use = "returns the rounded value using banker's rounding"]
pub fn round_half_to_even(value: f32) -> f32 {
    let rounded_down = value.floor();
    let fraction = value - rounded_down;

    if fraction < 0.5 {
        rounded_down
    } else if fraction > 0.5 {
        rounded_down + 1.0
    } else {
        // Exactly 0.5 - round to even
        if (rounded_down as i32) % 2 == 0 {
            rounded_down // Already even
        } else {
            rounded_down + 1.0 // Make even
        }
    }
}

/// Round value to nearest multiple of 32 using Python's "round half to even" behavior
///
/// Python 3 uses banker's rounding: round(0.5) → 0, round(1.5) → 2, round(2.5) → 2
/// This matches Python's `round()` function for ties (x.5 values)
const fn round_to_multiple_of_32_python(value: u32) -> u32 {
    let quotient = value / 32;
    let remainder = value % 32;

    if remainder < 16 {
        // Round down
        quotient * 32
    } else if remainder > 16 {
        // Round up
        (quotient + 1) * 32
    } else {
        // remainder == 16 (exactly x.5), use banker's rounding (round to even)
        if quotient % 2 == 0 {
            quotient * 32 // Round to even (down)
        } else {
            (quotient + 1) * 32 // Round to even (up)
        }
    }
}

/// DbNet text detection model
///
/// Detects text regions in images and returns bounding boxes with confidence scores.
///
/// Reference: DbNet.cpp
#[cfg(feature = "opencv-preprocessing")]
pub struct DbNet {
    session: Session,
}

#[cfg(feature = "opencv-preprocessing")]
impl std::fmt::Debug for DbNet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DbNet")
            .field("session", &"<Session>")
            .finish()
    }
}

#[cfg(feature = "opencv-preprocessing")]
impl DbNet {
    /// Load DbNet model from ONNX file
    ///
    /// # Arguments
    /// * `model_path` - Path to ch_PP-OCRv4_det_infer.onnx
    #[must_use = "this returns a Result that should be handled"]
    pub fn new(model_path: &str) -> Result<Self> {
        // Enable Level3 optimizations + threading for detection model (N=143)
        let num_threads = num_cpus::get();
        let session = Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_intra_threads(num_threads)?
            .commit_from_file(model_path)?;

        Ok(Self { session })
    }

    /// Detect text regions in image
    ///
    /// # Pipeline (Reference: DbNet.cpp:122-171)
    /// 1. Preprocess: Resize + Normalize (lines 123-130)
    /// 2. ONNX Inference: Get detection map (lines 134-141)
    /// 3. Postprocess: Threshold + Dilate + Find Contours + Score + UnClip (lines 143-171)
    ///
    /// # Arguments
    /// * `image` - Input image
    /// * `params` - Detection parameters (thresholds, unclip ratio, etc.)
    ///
    /// # Returns
    /// Vector of detected text boxes with coordinates and confidence scores
    pub fn detect(
        &mut self,
        image: &DynamicImage,
        params: &DetectionParams,
    ) -> Result<Vec<TextBox>> {
        self.detect_with_profiling(image, params, false)
            .map(|(boxes, _)| boxes)
    }

    /// Detect text regions in multiple images using batch processing
    ///
    /// # Batch Processing Strategy
    /// 1. Preprocess all images individually (each gets resized according to params)
    /// 2. Find max dimensions (H, W) across all preprocessed images
    /// 3. Pad all images to max dimensions (zero-padding)
    /// 4. Stack into single batch tensor [N, 3, H, W]
    /// 5. Single ONNX inference call (amortizes overhead)
    /// 6. Split output detection maps [N, 1, H_out, W_out] back to individual results
    /// 7. Postprocess each detection map separately
    ///
    /// Expected speedup: 1.2-1.5x for batch size 5-10 (CPU memory bandwidth limited)
    ///
    /// # Arguments
    /// * `images` - Vector of input images
    /// * `params` - Detection parameters (same for all images)
    ///
    /// # Returns
    /// Vector of text box vectors (one per image)
    pub fn detect_batch(
        &mut self,
        images: &[DynamicImage],
        params: &DetectionParams,
    ) -> Result<Vec<Vec<TextBox>>> {
        if images.is_empty() {
            return Ok(Vec::new());
        }

        // Single image: use sequential path (no batching overhead)
        if images.len() == 1 {
            let boxes = self.detect(&images[0], params)?;
            return Ok(vec![boxes]);
        }

        // Step 1: Preprocess all images individually
        let mut preprocessed_data = Vec::with_capacity(images.len());
        let mut max_height = 0;
        let mut max_width = 0;

        for image in images {
            let (tensor, width, height) = self.preprocess(image, params)?;
            max_height = max_height.max(height as usize);
            max_width = max_width.max(width as usize);
            preprocessed_data.push((tensor, width, height));
        }

        // Step 2: Create batch tensor [N, 3, H_max, W_max]
        // Pad all images to max dimensions
        let batch_size = images.len();
        let mut batch_tensor = Array4::<f32>::zeros((batch_size, 3, max_height, max_width));

        for (i, (tensor, width, height)) in preprocessed_data.iter().enumerate() {
            let h = *height as usize;
            let w = *width as usize;

            // Copy preprocessed tensor into batch (top-left corner, rest is zeros)
            for c in 0..3 {
                for y in 0..h {
                    for x in 0..w {
                        batch_tensor[[i, c, y, x]] = tensor[[0, c, y, x]];
                    }
                }
            }
        }

        // Step 3: Single ONNX inference call
        let shape = batch_tensor.shape().to_vec();
        let (data, _offset) = batch_tensor.into_raw_vec_and_offset();
        let input_value = ort::value::Value::from_array((shape.as_slice(), data))?;

        let outputs = self.session.run(ort::inputs!["x" => input_value])?;

        // Step 4: Split outputs and postprocess each
        let (output_shape, output_data) = outputs[0].try_extract_tensor::<f32>()?;

        // Output shape: [N, 1, H_out, W_out]
        let out_height = output_shape[2] as i32;
        let out_width = output_shape[3] as i32;
        let out_area = (out_height * out_width) as usize;

        let mut all_boxes = Vec::with_capacity(batch_size);

        for (i, (image, (_, preprocessed_width, preprocessed_height))) in
            images.iter().zip(&preprocessed_data).enumerate()
        {
            // Extract detection map for this image
            let detection_map = &output_data[i * out_area..(i + 1) * out_area];

            // Calculate scale factors
            let (orig_width, orig_height) = image.dimensions();
            let scale_x = orig_width as f32 / *preprocessed_width as f32;
            let scale_y = orig_height as f32 / *preprocessed_height as f32;

            // Postprocess
            let text_boxes = Self::postprocess(
                detection_map,
                out_width,
                out_height,
                params,
                scale_x,
                scale_y,
                orig_width,
                orig_height,
            )?;

            all_boxes.push(text_boxes);
        }

        Ok(all_boxes)
    }

    /// Detect text regions in image with profiling
    ///
    /// # Pipeline (Reference: DbNet.cpp:122-171)
    /// 1. Preprocess: Resize + Normalize (lines 123-130)
    /// 2. ONNX Inference: Get detection map (lines 134-141)
    /// 3. Postprocess: Threshold + Dilate + Find Contours + Score + UnClip (lines 143-171)
    ///
    /// # Arguments
    /// * `image` - Input image
    /// * `params` - Detection parameters (thresholds, unclip ratio, etc.)
    /// * `enable_profiling` - If true, collect timing information for each stage
    ///
    /// # Returns
    /// Vector of detected text boxes and optional profiling data
    pub fn detect_with_profiling(
        &mut self,
        image: &DynamicImage,
        params: &DetectionParams,
        enable_profiling: bool,
    ) -> Result<(Vec<TextBox>, Option<DbNetProfiling>)> {
        // Stage 1: Preprocessing
        let preprocessing_start = if enable_profiling {
            Some(Instant::now())
        } else {
            None
        };

        let (input_tensor, preprocessed_width, preprocessed_height) =
            self.preprocess(image, params)?;

        let preprocessing_duration = preprocessing_start.map(|start| start.elapsed());

        // Stage 2: ONNX Inference
        // Reference: DbNet.cpp:134-141
        let inference_start = if enable_profiling {
            Some(Instant::now())
        } else {
            None
        };

        // Convert ndarray to Value
        let shape = input_tensor.shape().to_vec();
        let (data, _offset) = input_tensor.into_raw_vec_and_offset();
        let input_value = ort::value::Value::from_array((shape.as_slice(), data))?;

        let outputs = self.session.run(ort::inputs!["x" => input_value])?;

        let inference_duration = inference_start.map(|start| start.elapsed());

        // Stage 3: Postprocessing
        // Reference: DbNet.cpp:143-171
        let postprocessing_start = if enable_profiling {
            Some(Instant::now())
        } else {
            None
        };

        let (output_shape, output_data) = outputs[0].try_extract_tensor::<f32>()?;

        // Extract detection map dimensions [1, 1, H, W]
        let out_height = output_shape[2] as i32;
        let out_width = output_shape[3] as i32;

        // Calculate scale factors for converting back to original image size
        let (orig_width, orig_height) = image.dimensions();
        let scale_x = orig_width as f32 / preprocessed_width as f32;
        let scale_y = orig_height as f32 / preprocessed_height as f32;

        let text_boxes = Self::postprocess(
            output_data,
            out_width,
            out_height,
            params,
            scale_x,
            scale_y,
            orig_width,
            orig_height,
        )?;

        let postprocessing_duration = postprocessing_start.map(|start| start.elapsed());

        // Build profiling data if requested
        let profiling = if enable_profiling {
            Some(DbNetProfiling {
                preprocessing_duration: preprocessing_duration.unwrap(),
                inference_duration: inference_duration.unwrap(),
                postprocessing_duration: postprocessing_duration.unwrap(),
            })
        } else {
            None
        };

        Ok((text_boxes, profiling))
    }

    /// Preprocess image for DbNet inference
    ///
    /// Two-stage preprocessing that preserves aspect ratio:
    /// 1. Global preprocessing (Python preprocess method)
    /// 2. Detection preprocessing (Python DetPreProcess.resize)
    ///
    /// Reference:
    /// - rapidocr_onnxruntime/main.py:129-140 (preprocess)
    /// - rapidocr_onnxruntime/utils/process_img.py:10-37 (reduce_max_side)
    /// - rapidocr_onnxruntime/utils/process_img.py:40-67 (increase_min_side)
    /// - rapidocr_onnxruntime/ch_ppocr_det/utils.py:45-79 (DetPreProcess.resize)
    ///
    /// # Arguments
    /// * `image` - Input image
    /// * `params` - Detection parameters
    ///
    /// # Returns
    /// (tensor, final_width, final_height) - Tensor in NCHW format and dimensions after preprocessing
    fn preprocess(
        &self,
        image: &DynamicImage,
        params: &DetectionParams,
    ) -> Result<(Array4<f32>, u32, u32)> {
        let (mut width, mut height) = image.dimensions();

        // Stage 1: Global preprocessing (max_side_len, min_side_len)
        // Reference: rapidocr_onnxruntime/main.py:129-140

        // Step 1a: reduce_max_side - Scale down if max side > max_side_len
        let max_value = width.max(height);
        let mut resized_img = image.clone();

        if max_value > params.max_side_len {
            // Calculate ratio to reduce max side to max_side_len
            let ratio = if height > width {
                params.max_side_len as f32 / height as f32
            } else {
                params.max_side_len as f32 / width as f32
            };

            let mut resize_h = (height as f32 * ratio) as u32;
            let mut resize_w = (width as f32 * ratio) as u32;

            // Round to nearest multiple of 32 using Python's banker's rounding
            resize_h = round_to_multiple_of_32_python(resize_h);
            resize_w = round_to_multiple_of_32_python(resize_w);

            resized_img =
                resized_img.resize_exact(resize_w, resize_h, image::imageops::FilterType::Lanczos3);
            width = resize_w;
            height = resize_h;
        }

        // Step 1b: increase_min_side - Scale up if min side < min_side_len
        let min_value = width.min(height);
        if min_value < params.min_side_len {
            // Calculate ratio to increase min side to min_side_len
            let ratio = if height < width {
                params.min_side_len as f32 / height as f32
            } else {
                params.min_side_len as f32 / width as f32
            };

            let mut resize_h = (height as f32 * ratio) as u32;
            let mut resize_w = (width as f32 * ratio) as u32;

            // Round to nearest multiple of 32 using Python's banker's rounding
            resize_h = round_to_multiple_of_32_python(resize_h);
            resize_w = round_to_multiple_of_32_python(resize_w);

            resized_img =
                resized_img.resize_exact(resize_w, resize_h, image::imageops::FilterType::Lanczos3);
            width = resize_w;
            height = resize_h;
        }

        // Stage 2: Detection preprocessing (limit_side_len, limit_type)
        // Reference: rapidocr_onnxruntime/ch_ppocr_det/utils.py:45-79

        use crate::ocr::types::LimitType;
        let ratio = match params.limit_type {
            LimitType::Max => {
                // If max(h, w) > limit_side_len, scale down
                if width.max(height) > params.limit_side_len {
                    if height > width {
                        params.limit_side_len as f32 / height as f32
                    } else {
                        params.limit_side_len as f32 / width as f32
                    }
                } else {
                    1.0
                }
            }
            LimitType::Min => {
                // If min(h, w) < limit_side_len, scale up
                if width.min(height) < params.limit_side_len {
                    if height < width {
                        params.limit_side_len as f32 / height as f32
                    } else {
                        params.limit_side_len as f32 / width as f32
                    }
                } else {
                    1.0
                }
            }
        };

        let mut resize_h = (height as f32 * ratio) as u32;
        let mut resize_w = (width as f32 * ratio) as u32;

        // Round to nearest multiple of 32 using Python's banker's rounding
        resize_h = round_to_multiple_of_32_python(resize_h);
        resize_w = round_to_multiple_of_32_python(resize_w);

        let final_img =
            resized_img.resize_exact(resize_w, resize_h, image::imageops::FilterType::Lanczos3);
        let rgb_image = final_img.to_rgb8();

        // Normalize and convert to tensor
        // Note: Python uses mean=[0.5, 0.5, 0.5], std=[0.5, 0.5, 0.5]
        // Formula: (pixel / 255.0 - mean) / std
        let mut tensor = Array4::<f32>::zeros((1, 3, resize_h as usize, resize_w as usize));

        for y in 0..resize_h {
            for x in 0..resize_w {
                let pixel = rgb_image.get_pixel(x, y);
                for c in 0..3 {
                    let normalized = (pixel[c] as f32 * OCR_PIXEL_SCALE - OCR_NORMALIZE_MEAN)
                        / OCR_NORMALIZE_STD;
                    tensor[[0, c, y as usize, x as usize]] = normalized;
                }
            }
        }

        Ok((tensor, resize_w, resize_h))
    }

    /// Postprocess detection map to extract text boxes
    ///
    /// Reference: DbNet.cpp:143-171, findRsBoxes (lines 62-119)
    ///
    /// # Steps
    /// 1. Threshold detection map to create binary mask
    /// 2. Dilate binary mask
    /// 3. Find contours
    /// 4. For each contour:
    ///    - Get minimum area rotated rectangle
    ///    - Filter by size (long side ≥ 3)
    ///    - Calculate box score (average detection map value)
    ///    - Filter by box_score_thresh
    ///    - UnClip (expand box)
    ///    - Scale back to original image size
    ///
    /// # Arguments
    /// * `detection_map` - Output from ONNX model (f32 values 0.0-1.0)
    /// * `width` - Detection map width
    /// * `height` - Detection map height
    /// * `params` - Detection parameters
    /// * `scale_x` - Scale factor for x coordinates (original_width / target_width)
    /// * `scale_y` - Scale factor for y coordinates (original_height / target_height)
    ///
    /// # Returns
    /// Vector of detected text boxes
    #[allow(
        clippy::too_many_arguments,
        reason = "matches C++ RapidOCR API signature for postprocessing"
    )]
    pub fn postprocess(
        detection_map: &[f32],
        width: i32,
        height: i32,
        params: &DetectionParams,
        scale_x: f32,
        scale_y: f32,
        orig_width: u32,
        orig_height: u32,
    ) -> Result<Vec<TextBox>> {
        // Note: scale_x and scale_y already account for original vs preprocessed dimensions
        // They are calculated in detect() as orig_width / preprocessed_width
        // Reference: DbNet.cpp:143-158
        // Create OpenCV Mats for processing

        // predMat: f32 values (0.0 - 1.0)
        // Create single-channel f32 Mat
        use opencv::core::{CV_32FC1, CV_8UC1};
        let mut pred_mat =
            Mat::new_rows_cols_with_default(height, width, CV_32FC1, Scalar::all(0.0))?;
        // Copy data into Mat
        unsafe {
            let pred_data = pred_mat.data_mut();
            let pred_slice =
                std::slice::from_raw_parts_mut(pred_data as *mut f32, (height * width) as usize);
            pred_slice.copy_from_slice(detection_map);
        }

        // cBufMat: u8 values (0 - 255)
        let cbuf_data: Vec<u8> = detection_map.iter().map(|&v| (v * 255.0) as u8).collect();
        let mut cbuf_mat =
            Mat::new_rows_cols_with_default(height, width, CV_8UC1, Scalar::all(0.0))?;
        unsafe {
            let cbuf_data_ptr = cbuf_mat.data_mut();
            let cbuf_slice =
                std::slice::from_raw_parts_mut(cbuf_data_ptr, (height * width) as usize);
            cbuf_slice.copy_from_slice(&cbuf_data);
        }

        // Reference: DbNet.cpp:159-163
        // Threshold at box_thresh * 255
        let mut threshold_mat = Mat::default();
        let threshold = (params.box_thresh * 255.0) as f64;
        imgproc::threshold(
            &cbuf_mat,
            &mut threshold_mat,
            threshold,
            255.0,
            imgproc::THRESH_BINARY,
        )?;

        // Reference: DbNet.cpp:165-168
        // Dilate with 2x2 kernel
        let mut dilate_mat = Mat::default();
        let dilate_element = imgproc::get_structuring_element(
            imgproc::MORPH_RECT,
            Size::new(2, 2),
            Point::new(-1, -1),
        )?;
        imgproc::dilate(
            &threshold_mat,
            &mut dilate_mat,
            &dilate_element,
            Point::new(-1, -1),
            1,
            BORDER_CONSTANT,
            Scalar::default(),
        )?;

        // Reference: DbNet.cpp:70-71
        // Find contours
        let mut contours = Vector::<Vector<Point>>::new();
        imgproc::find_contours(
            &dilate_mat,
            &mut contours,
            imgproc::RETR_LIST,
            imgproc::CHAIN_APPROX_SIMPLE,
            Point::new(0, 0),
        )?;

        // Reference: DbNet.cpp:73-74, 64-65
        let num_contours = contours.len().min(OCR_MAX_CANDIDATES);

        let mut text_boxes = Vec::new();

        // Reference: DbNet.cpp:77-116
        // Process each contour
        for i in 0..num_contours {
            let contour = contours.get(i)?;

            // Skip tiny contours (≤ 2 points)
            if contour.len() <= 2 {
                continue;
            }

            // Get minimum area rotated rectangle
            let min_area_rect = imgproc::min_area_rect(&contour)?;

            // Get box corners and long side length
            let (min_boxes, long_side) = get_min_boxes(&min_area_rect)?;

            // Filter by size (long side ≥ 3)
            if long_side < OCR_LONG_SIDE_THRESH {
                continue;
            }

            // Calculate box score (average detection map value inside box)
            let box_score = box_score_fast(&min_boxes, &pred_mat)?;
            if box_score < params.box_score_thresh {
                continue;
            }

            // UnClip: Expand box by unclip_ratio
            let clip_rect = unclip(&min_boxes, params.unclip_ratio)?;
            if clip_rect.size.width < 1.001 || clip_rect.size.height < 1.001 {
                continue;
            }

            // Get corners of expanded box
            let (clip_min_boxes, clip_long_side) = get_min_boxes(&clip_rect)?;

            // Filter expanded box by size (long side ≥ 3 + 2)
            if clip_long_side < OCR_LONG_SIDE_THRESH + 2.0 {
                continue;
            }

            // Scale back to original image coordinates
            // Python: box[:, 0] = np.clip(np.round(box[:, 0] / width * dest_width), 0, dest_width)
            // IMPORTANT: numpy uses "round half to even" (banker's rounding), not "round half away from zero"
            let mut scaled_corners = Vec::with_capacity(4);
            for point in &clip_min_boxes {
                let x_scaled = point.x / width as f32 * orig_width as f32;
                let y_scaled = point.y / height as f32 * orig_height as f32;

                // Use banker's rounding (round half to even) to match numpy
                let x = round_half_to_even(x_scaled).clamp(0.0, orig_width as f32);
                let y = round_half_to_even(y_scaled).clamp(0.0, orig_height as f32);
                scaled_corners.push((x, y));
            }

            text_boxes.push(TextBox {
                corners: scaled_corners,
                score: box_score,
            });
        }

        // Reference: DbNet.cpp:117
        // Reverse order (C++ implementation does this)
        text_boxes.reverse();

        Ok(text_boxes)
    }
}

/// Get minimum bounding box corners from rotated rectangle
///
/// Reference: OcrUtils.cpp:187-212 (getMinBoxes)
/// Python: cv2.boxPoints() then custom sort/reorder
///
/// Returns corners in specific order (clockwise from top-left) and max side length
#[cfg(feature = "opencv-preprocessing")]
fn get_min_boxes(box_rect: &core::RotatedRect) -> Result<(Vec<Point2f>, f32)> {
    // Get max side length
    let max_side = box_rect.size.width.max(box_rect.size.height);

    // Use OpenCV's box_points (equivalent to Python's cv2.boxPoints)
    let mut pts_mat = Mat::default();
    imgproc::box_points(*box_rect, &mut pts_mat)?;

    // Reshape to Point2f matrix (test approach from opencv-0.97.1/tests/imgproc.rs)
    let pts_reshaped = pts_mat.reshape_def(Point2f::opencv_channels())?;

    // Extract points from reshaped Mat
    let mut box_points_unsorted: Vec<Point2f> = Vec::with_capacity(4);
    for i in 0..4 {
        let pt: &Point2f = pts_reshaped.at(i)?;
        box_points_unsorted.push(*pt);
    }

    // Sort by x coordinate (Reference: OcrUtils.cpp:190, Python line 174)
    box_points_unsorted.sort_by(|a, b| a.x.total_cmp(&b.x));

    // Determine order (Reference: OcrUtils.cpp:191-210, Python lines 177-192)
    let (index1, index4) = if box_points_unsorted[1].y > box_points_unsorted[0].y {
        (0, 1)
    } else {
        (1, 0)
    };

    let (index2, index3) = if box_points_unsorted[3].y > box_points_unsorted[2].y {
        (2, 3)
    } else {
        (3, 2)
    };

    let min_box = vec![
        box_points_unsorted[index1],
        box_points_unsorted[index2],
        box_points_unsorted[index3],
        box_points_unsorted[index4],
    ];

    Ok((min_box, max_side))
}

/// Calculate box score (average detection map value inside box)
///
/// Reference: OcrUtils.cpp:214-243 (boxScoreFast)
#[cfg(feature = "opencv-preprocessing")]
fn box_score_fast(boxes: &[Point2f], pred_mat: &impl core::MatTraitConst) -> Result<f32> {
    let width = pred_mat.cols();
    let height = pred_mat.rows();

    // Get bounding box
    let xs: Vec<f32> = boxes.iter().map(|p| p.x).collect();
    let ys: Vec<f32> = boxes.iter().map(|p| p.y).collect();

    let min_x = xs.iter().copied().fold(f32::INFINITY, f32::min).floor() as i32;
    let max_x = xs.iter().copied().fold(f32::NEG_INFINITY, f32::max).ceil() as i32;
    let min_y = ys.iter().copied().fold(f32::INFINITY, f32::min).floor() as i32;
    let max_y = ys.iter().copied().fold(f32::NEG_INFINITY, f32::max).ceil() as i32;

    let min_x = min_x.clamp(0, width - 1);
    let max_x = max_x.clamp(0, width - 1);
    let min_y = min_y.clamp(0, height - 1);
    let max_y = max_y.clamp(0, height - 1);

    // Create mask (MatExpr -> Mat conversion)
    let mask_size = Size::new(max_x - min_x + 1, max_y - min_y + 1);
    let mut mask = Mat::zeros_size(mask_size, core::CV_8UC1)?.to_mat()?;

    // Translate box points to mask coordinates
    let mask_points: Vec<Point> = boxes
        .iter()
        .map(|p| Point::new(p.x as i32 - min_x, p.y as i32 - min_y))
        .collect();

    let pts = Vector::<Point>::from_iter(mask_points);
    let contours = Vector::<Vector<Point>>::from_iter(vec![pts]);
    imgproc::fill_poly(
        &mut mask,
        &contours,
        Scalar::new(1.0, 0.0, 0.0, 0.0),
        imgproc::LINE_8,
        0,
        Point::new(0, 0),
    )?;

    // Crop prediction map
    let cropped = Mat::roi(
        pred_mat,
        core::Rect::new(min_x, min_y, max_x - min_x + 1, max_y - min_y + 1),
    )?;

    // Calculate mean with mask
    let mean = core::mean(&cropped, &mask)?;

    Ok(mean[0] as f32)
}

/// Expand box by unclip_ratio (UnClip algorithm)
///
/// Reference: OcrUtils.cpp:262-290 (unClip)
#[cfg(feature = "opencv-preprocessing")]
fn unclip(boxes: &[Point2f], unclip_ratio: f32) -> Result<core::RotatedRect> {
    // Convert to geo::Polygon for ClipperLib
    // IMPORTANT: Python calculates distance from FLOAT polygon, then uses INTEGER polygon for clipping
    // Step 1: Calculate distance from float coords (matches Python Shapely)
    let coords_float: Vec<Coord<f64>> = boxes
        .iter()
        .map(|p| Coord {
            x: p.x as f64,
            y: p.y as f64,
        })
        .collect();

    let poly_float = Polygon::new(LineString::from(coords_float.clone()), vec![]);
    let distance =
        poly_float.unsigned_area() * unclip_ratio as f64 / poly_float.exterior().euclidean_length();

    // Step 2: Truncate coords to integers (matches Python pyclipper)
    // pyclipper uses int64, returns int64 - we must do the same
    let coords_int: Vec<Coord<i64>> = boxes
        .iter()
        .map(|p| Coord {
            x: p.x.trunc() as i64, // Truncate to int64
            y: p.y.trunc() as i64, // Truncate to int64
        })
        .collect();

    let poly_int = Polygon::new(LineString::from(coords_int), vec![]);

    // Offset polygon (expand)
    // Use Polygon<i64>.offset() - returns MultiPolygon<i64> (matches pyclipper)
    // Python pyclipper: ArcTolerance=0.25, MiterLimit=2.0
    use geo_clipper::{ClipperInt, EndType, JoinType};
    let offset_polys_int = poly_int.offset(
        distance,
        JoinType::Round(0.25), // Arc tolerance = 0.25
        EndType::ClosedPolygon,
    );

    // Get first offset polygon (should only be one)
    let first_poly_int = offset_polys_int.0.first();
    if first_poly_int.is_none() {
        // Return original as rotated rect if offset failed
        let pts = Vector::<Point2f>::from_iter(boxes.to_vec());
        return Ok(imgproc::min_area_rect(&pts)?);
    }

    // Convert back to OpenCV points (i64 -> f32)
    // IMPORTANT: geo-clipper includes duplicate closing point, pyclipper doesn't
    let exterior = first_poly_int.unwrap().exterior();
    let all_coords: Vec<_> = exterior.coords().collect();

    // geo LineString.coords() ALWAYS includes closing point (last = first)
    // Python pyclipper returns open paths (no duplicate)
    // ALWAYS remove last coord to match Python behavior
    let num_coords = all_coords.len();
    let offset_points: Vec<Point2f> = if num_coords > 2 {
        // Remove last point (closing duplicate)
        // Use num_coords - 1 to exclude it
        all_coords[0..num_coords - 1]
            .iter()
            .map(|c| Point2f::new(c.x as f32, c.y as f32))
            .collect()
    } else {
        // Edge case: < 3 points, use all
        all_coords
            .iter()
            .map(|c| Point2f::new(c.x as f32, c.y as f32))
            .collect()
    };

    // Get minimum area rect of offset polygon
    if offset_points.is_empty() {
        // Return original if no points
        let pts = Vector::<Point2f>::from_iter(boxes.to_vec());
        return Ok(imgproc::min_area_rect(&pts)?);
    }

    let pts = Vector::<Point2f>::from_iter(offset_points);
    Ok(imgproc::min_area_rect(&pts)?)
}

/// Calculate polygon area and perimeter for UnClip
///
/// Reference: OcrUtils.cpp:245-260 (getContourArea)
#[cfg(feature = "opencv-preprocessing")]
#[allow(
    dead_code,
    reason = "reference implementation from OcrUtils.cpp for future opencv preprocessing"
)]
fn get_contour_area(boxes: &[Point2f], unclip_ratio: f32) -> f32 {
    let size = boxes.len();
    let mut area = 0.0f32;
    let mut dist = 0.0f32;

    for i in 0..size {
        let next = (i + 1) % size;
        // Shoelace formula for area
        area += boxes[i].x * boxes[next].y - boxes[i].y * boxes[next].x;

        // Euclidean distance for perimeter
        let dx = boxes[i].x - boxes[next].x;
        let dy = boxes[i].y - boxes[next].y;
        dist += (dx * dx + dy * dy).sqrt();
    }

    area = (area / 2.0).abs();

    // Return offset distance (area * ratio / perimeter)
    area * unclip_ratio / dist
}

// ============================================================================
// DbNetPure: Pure Rust implementation without OpenCV dependency
// ============================================================================

use crate::ocr::postprocess_pure::postprocess_pure;

/// `DbNet` text detection model (Pure Rust)
///
/// Detects text regions in images and returns bounding boxes with confidence scores.
/// This version uses pure Rust postprocessing without `OpenCV` dependency.
///
/// Reference: DbNet.cpp
pub struct DbNetPure {
    session: Session,
}

impl std::fmt::Debug for DbNetPure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DbNetPure")
            .field("session", &"<Session>")
            .finish()
    }
}

impl DbNetPure {
    /// Load `DbNet` model from ONNX file (CPU backend)
    ///
    /// # Arguments
    /// * `model_path` - Path to ch_PP-OCRv4_det_infer.onnx
    #[must_use = "this returns a Result that should be handled"]
    pub fn new(model_path: &str) -> Result<Self> {
        Self::new_with_backend(model_path, false)
    }

    /// Load `DbNet` model with `CoreML` backend (Apple Neural Engine acceleration)
    ///
    /// On macOS with Apple Silicon, this enables hardware-accelerated inference
    /// via the Apple Neural Engine (ANE), providing 2-3x speedup over CPU.
    ///
    /// # Arguments
    /// * `model_path` - Path to ch_PP-OCRv4_det_infer.onnx
    #[must_use = "this returns a Result that should be handled"]
    pub fn new_with_coreml(model_path: &str) -> Result<Self> {
        Self::new_with_backend(model_path, true)
    }

    /// Load `DbNet` model with specified backend
    ///
    /// # Arguments
    /// * `model_path` - Path to ch_PP-OCRv4_det_infer.onnx
    /// * `use_coreml` - If true, use `CoreML` execution provider (macOS ANE acceleration)
    fn new_with_backend(model_path: &str, use_coreml: bool) -> Result<Self> {
        let num_threads = num_cpus::get();

        let session = if use_coreml {
            log::debug!("Creating DbNetPure session with CoreML execution provider");
            Session::builder()?
                .with_optimization_level(GraphOptimizationLevel::Level3)?
                .with_intra_threads(num_threads)?
                .with_execution_providers([
                    CoreMLExecutionProvider::default().build(),
                    CPUExecutionProvider::default().build(), // Fallback
                ])?
                .commit_from_file(model_path)?
        } else {
            log::debug!("Creating DbNetPure session with CPU execution provider");
            Session::builder()?
                .with_optimization_level(GraphOptimizationLevel::Level3)?
                .with_intra_threads(num_threads)?
                .commit_from_file(model_path)?
        };

        Ok(Self { session })
    }

    /// Detect text regions in image
    pub fn detect(
        &mut self,
        image: &DynamicImage,
        params: &DetectionParams,
    ) -> Result<Vec<TextBox>> {
        self.detect_with_profiling(image, params, false)
            .map(|(boxes, _)| boxes)
    }

    /// Detect text regions in image with profiling
    pub fn detect_with_profiling(
        &mut self,
        image: &DynamicImage,
        params: &DetectionParams,
        enable_profiling: bool,
    ) -> Result<(Vec<TextBox>, Option<DbNetProfiling>)> {
        // Stage 1: Preprocessing
        let preprocessing_start = enable_profiling.then(Instant::now);

        let (input_tensor, preprocessed_width, preprocessed_height) =
            self.preprocess(image, params)?;

        let preprocessing_duration = preprocessing_start.map(|start| start.elapsed());

        // Stage 2: ONNX Inference
        let inference_start = enable_profiling.then(Instant::now);

        let shape = input_tensor.shape().to_vec();
        let (data, _offset) = input_tensor.into_raw_vec_and_offset();
        let input_value = ort::value::Value::from_array((shape.as_slice(), data))?;

        let outputs = self.session.run(ort::inputs!["x" => input_value])?;

        let inference_duration = inference_start.map(|start| start.elapsed());

        // Stage 3: Postprocessing (using pure Rust implementation)
        let postprocessing_start = enable_profiling.then(Instant::now);

        let (output_shape, output_data) = outputs[0].try_extract_tensor::<f32>()?;

        let out_height = output_shape[2] as i32;
        let out_width = output_shape[3] as i32;

        let (orig_width, orig_height) = image.dimensions();
        let scale_x = orig_width as f32 / preprocessed_width as f32;
        let scale_y = orig_height as f32 / preprocessed_height as f32;

        // Use pure Rust postprocessing
        let text_boxes = postprocess_pure(
            output_data,
            out_width,
            out_height,
            params,
            scale_x,
            scale_y,
            orig_width,
            orig_height,
        )?;

        let postprocessing_duration = postprocessing_start.map(|start| start.elapsed());

        let profiling = enable_profiling.then(|| DbNetProfiling {
            preprocessing_duration: preprocessing_duration.unwrap(),
            inference_duration: inference_duration.unwrap(),
            postprocessing_duration: postprocessing_duration.unwrap(),
        });

        Ok((text_boxes, profiling))
    }

    /// Preprocess image for `DbNet` inference (pure Rust)
    // Method signature kept for API consistency with other TextDetector methods
    #[allow(clippy::unused_self)]
    #[allow(clippy::unnecessary_wraps)] // Result for API consistency with other preprocess methods
    fn preprocess(
        &self,
        image: &DynamicImage,
        params: &DetectionParams,
    ) -> Result<(Array4<f32>, u32, u32)> {
        use crate::ocr::types::LimitType;

        let (mut width, mut height) = image.dimensions();

        // Stage 1: Global preprocessing (max_side_len, min_side_len)
        let max_value = width.max(height);
        let mut resized_img = image.clone();

        if max_value > params.max_side_len {
            let ratio = if height > width {
                params.max_side_len as f32 / height as f32
            } else {
                params.max_side_len as f32 / width as f32
            };

            let mut resize_h = (height as f32 * ratio) as u32;
            let mut resize_w = (width as f32 * ratio) as u32;

            resize_h = round_to_multiple_of_32_python(resize_h);
            resize_w = round_to_multiple_of_32_python(resize_w);

            resized_img =
                resized_img.resize_exact(resize_w, resize_h, image::imageops::FilterType::Lanczos3);
            width = resize_w;
            height = resize_h;
        }

        let min_value = width.min(height);
        if min_value < params.min_side_len {
            let ratio = if height < width {
                params.min_side_len as f32 / height as f32
            } else {
                params.min_side_len as f32 / width as f32
            };

            let mut resize_h = (height as f32 * ratio) as u32;
            let mut resize_w = (width as f32 * ratio) as u32;

            resize_h = round_to_multiple_of_32_python(resize_h);
            resize_w = round_to_multiple_of_32_python(resize_w);

            resized_img =
                resized_img.resize_exact(resize_w, resize_h, image::imageops::FilterType::Lanczos3);
            width = resize_w;
            height = resize_h;
        }

        // Stage 2: Detection preprocessing (limit_side_len, limit_type)
        let ratio = match params.limit_type {
            LimitType::Max => {
                if width.max(height) > params.limit_side_len {
                    if height > width {
                        params.limit_side_len as f32 / height as f32
                    } else {
                        params.limit_side_len as f32 / width as f32
                    }
                } else {
                    1.0
                }
            }
            LimitType::Min => {
                if width.min(height) < params.limit_side_len {
                    if height < width {
                        params.limit_side_len as f32 / height as f32
                    } else {
                        params.limit_side_len as f32 / width as f32
                    }
                } else {
                    1.0
                }
            }
        };

        let mut resize_h = (height as f32 * ratio) as u32;
        let mut resize_w = (width as f32 * ratio) as u32;

        resize_h = round_to_multiple_of_32_python(resize_h);
        resize_w = round_to_multiple_of_32_python(resize_w);

        let final_img =
            resized_img.resize_exact(resize_w, resize_h, image::imageops::FilterType::Lanczos3);
        let rgb_image = final_img.to_rgb8();

        // Normalize and convert to tensor
        let mut tensor = Array4::<f32>::zeros((1, 3, resize_h as usize, resize_w as usize));

        for y in 0..resize_h {
            for x in 0..resize_w {
                let pixel = rgb_image.get_pixel(x, y);
                for c in 0..3 {
                    let normalized = (f32::from(pixel[c]) * OCR_PIXEL_SCALE - OCR_NORMALIZE_MEAN)
                        / OCR_NORMALIZE_STD;
                    tensor[[0, c, y as usize, x as usize]] = normalized;
                }
            }
        }

        Ok((tensor, resize_w, resize_h))
    }
}

#[cfg(test)]
mod tests_pure {
    use super::*;

    #[test]
    fn test_dbnet_pure_loading() {
        let model = DbNetPure::new("models/rapidocr/ch_PP-OCRv4_det_infer.onnx");
        assert!(
            model.is_ok(),
            "Failed to load DbNetPure model: {:?}",
            model.err()
        );
    }

    #[test]
    fn test_dbnet_pure_preprocess() {
        let img = DynamicImage::new_rgb8(100, 80);

        let model = DbNetPure::new("models/rapidocr/ch_PP-OCRv4_det_infer.onnx")
            .expect("Failed to load model");

        let params = DetectionParams::default();

        let (tensor, width, height) = model
            .preprocess(&img, &params)
            .expect("Failed to preprocess");

        assert_eq!(width % 32, 0, "Width should be multiple of 32");
        assert_eq!(height % 32, 0, "Height should be multiple of 32");
        assert_eq!(tensor.shape(), &[1, 3, height as usize, width as usize]);
    }

    #[test]
    fn test_dbnet_pure_inference() {
        use image::GenericImage;

        let mut img = DynamicImage::new_rgb8(200, 200);

        // Add a white rectangle (simulating text region)
        for y in 50..150 {
            for x in 50..150 {
                img.put_pixel(x, y, image::Rgba([255, 255, 255, 255]));
            }
        }

        let mut model = DbNetPure::new("models/rapidocr/ch_PP-OCRv4_det_infer.onnx")
            .expect("Failed to load model");

        let params = DetectionParams::default();

        let result = model.detect(&img, &params);
        assert!(result.is_ok(), "Detection failed: {:?}", result.err());

        let boxes = result.unwrap();
        log::debug!("DbNetPure detected {} text boxes", boxes.len());
    }
}

#[cfg(all(test, feature = "opencv-preprocessing"))]
mod tests {
    use super::*;
    use image::GenericImage;
    use log;

    #[test]
    fn test_dbnet_loading() {
        let model = DbNet::new("models/rapidocr/ch_PP-OCRv4_det_infer.onnx");
        assert!(
            model.is_ok(),
            "Failed to load DbNet model: {:?}",
            model.err()
        );
    }

    #[test]
    fn test_dbnet_preprocess() {
        // Create a test image
        let img = image::DynamicImage::new_rgb8(100, 80);

        let model =
            DbNet::new("models/rapidocr/ch_PP-OCRv4_det_infer.onnx").expect("Failed to load model");

        let params = DetectionParams::default();

        // Test preprocessing
        let (tensor, width, height) = model
            .preprocess(&img, &params)
            .expect("Failed to preprocess");

        // Check that dimensions are multiples of 32
        assert_eq!(width % 32, 0, "Width should be multiple of 32");
        assert_eq!(height % 32, 0, "Height should be multiple of 32");

        // Check output shape matches dimensions
        assert_eq!(tensor.shape(), &[1, 3, height as usize, width as usize]);

        // Check values are normalized (should be in reasonable range)
        let max_val = tensor.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
        let min_val = tensor.iter().fold(f32::INFINITY, |a, &b| a.min(b));
        assert!(max_val < 10.0, "Max value too large: {}", max_val);
        assert!(min_val > -10.0, "Min value too small: {}", min_val);
    }

    #[test]
    fn test_dbnet_inference() {
        // Create a test image with some content (white on black)
        let mut img = image::DynamicImage::new_rgb8(200, 200);

        // Add a white rectangle (simulating text region)
        for y in 50..150 {
            for x in 50..150 {
                img.put_pixel(x, y, image::Rgba([255, 255, 255, 255]));
            }
        }

        let mut model =
            DbNet::new("models/rapidocr/ch_PP-OCRv4_det_infer.onnx").expect("Failed to load model");

        let params = DetectionParams::default();

        // Test full detect pipeline
        let result = model.detect(&img, &params);
        assert!(result.is_ok(), "Detection failed: {:?}", result.err());

        let boxes = result.unwrap();
        // Note: Detection may or may not find text depending on model output
        // Just verify it runs without error
        log::debug!("Detected {} text boxes", boxes.len());
    }
}
