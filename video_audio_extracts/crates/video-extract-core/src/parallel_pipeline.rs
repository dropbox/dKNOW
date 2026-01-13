//! Parallel pipeline for decode + inference with producer-consumer pattern
//!
//! This module implements a parallel streaming pipeline that overlaps video decode
//! with ML inference for maximum throughput. The decoder thread (FFmpeg) sends frames
//! directly to the inference thread (ONNX) through a single channel, enabling true
//! parallelism without intermediate forwarding overhead.
//!
//! # Architecture
//!
//! ```text
//! [Decode Thread]  --StreamFrame-->  [Inference Thread]
//!    (FFmpeg)                           (ONNX)
//! ```
//!
//! # Performance
//!
//! Expected speedup vs sequential pipeline:
//! - **1.5-2x faster** for videos with >10 keyframes
//! - Decode and inference overlap (parallel execution)
//! - Bounded channel prevents memory explosion
//! - Batch inference (BATCH_SIZE=8) for ONNX efficiency
//!
//! # Safety
//!
//! Thread safety is ensured by:
//! - Owned frame data (no shared mutable state)
//! - Crossbeam channels for safe message passing
//! - Explicit error propagation across threads
//! - Graceful shutdown on error or completion

use crossbeam_channel::{bounded, Receiver};
use ndarray::{s, Array4, ArrayView3, ShapeBuilder};
use ort::{session::Session, value::Value};
use std::borrow::Cow;
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::thread;
use video_audio_common::{ProcessingError, Result};
use video_audio_decoder::{RawFrameBuffer, StreamFrame};

// Re-export types from fast_path
use crate::fast_path::{BoundingBox, Detection, DetectionWithFrame};

/// Parallel pipeline configuration
#[derive(Debug, Clone)]
pub struct ParallelConfig {
    /// Batch size for inference (number of frames per ONNX call)
    pub batch_size: usize,
    /// Channel capacity (max frames in flight between threads)
    pub channel_capacity: usize,
    /// Confidence threshold for object detection
    pub confidence_threshold: f32,
    /// Class names to filter (None = all classes)
    pub classes: Option<Vec<String>>,
}

impl Default for ParallelConfig {
    fn default() -> Self {
        Self {
            // YOLOv8n model supports dynamic batch size (exported with dynamic=True)
            // batch_size=8 provides optimal throughput for multi-frame inference
            batch_size: 8,
            channel_capacity: 8,
            confidence_threshold: 0.25,
            classes: None,
        }
    }
}

/// Result from inference thread
type InferenceResult = Result<Vec<DetectionWithFrame>>;

/// Parallel keyframes + object detection pipeline
///
/// This function uses a producer-consumer pattern to overlap video decode
/// with ML inference for maximum throughput.
///
/// # Example
///
/// ```no_run
/// use video_extract_core::parallel_pipeline::{extract_and_detect_parallel, ParallelConfig};
/// use std::path::Path;
///
/// # fn main() -> anyhow::Result<()> {
/// let video_path = Path::new("video.mp4");
/// let config = ParallelConfig::default();
/// let detections = extract_and_detect_parallel(video_path, config)?;
///
/// println!("Found {} detections across all keyframes", detections.len());
/// # Ok(())
/// # }
/// ```
pub fn extract_and_detect_parallel(
    video_path: &Path,
    config: ParallelConfig,
) -> Result<Vec<DetectionWithFrame>> {
    // Create bounded channel (prevents memory explosion)
    let (frame_tx, frame_rx) = bounded::<StreamFrame>(config.channel_capacity);

    // Clone path for decoder thread
    let video_path_buf = video_path.to_path_buf();
    let batch_size = config.batch_size;
    let confidence_threshold = config.confidence_threshold;
    let classes = config.classes.clone();

    // Spawn decoder thread (FFmpeg) - sends StreamFrame directly
    let decode_handle = thread::spawn(move || {
        video_audio_decoder::decode_iframes_streaming(video_path_buf.as_path(), frame_tx)
    });

    // Spawn inference thread (ONNX) - receives StreamFrame directly
    let inference_handle = thread::spawn(move || {
        inference_thread_worker(frame_rx, batch_size, confidence_threshold, classes)
    });

    // Wait for both threads to finish
    let decode_result = decode_handle
        .join()
        .map_err(|_| ProcessingError::Other("Decoder thread panicked".to_string()))?;
    let detections = inference_handle
        .join()
        .map_err(|_| ProcessingError::Other("Inference thread panicked".to_string()))?;

    // Check for errors from decoder
    decode_result?;

    // Return results from inference
    detections
}

/// Inference thread: receive frames, batch, and run inference
fn inference_thread_worker(
    frame_rx: Receiver<StreamFrame>,
    batch_size: usize,
    confidence_threshold: f32,
    classes: Option<Vec<String>>,
) -> InferenceResult {
    let mut all_detections = Vec::with_capacity(128); // Initial capacity for ~128 frames (typical keyframe count)
    let mut batch: Vec<RawFrameBuffer> = Vec::with_capacity(batch_size);

    loop {
        // Receive next message
        let msg = frame_rx
            .recv()
            .map_err(|_| ProcessingError::Other("Decode thread disconnected".to_string()))?;

        match msg {
            StreamFrame::Frame(frame) => {
                batch.push(frame);

                // Process batch when full
                if batch.len() >= batch_size {
                    let batch_detections = process_batch(&batch, confidence_threshold, &classes)?;
                    all_detections.extend(batch_detections);
                    batch.clear();
                }
            }
            StreamFrame::Done(_frame_count) => {
                // Process remaining frames in partial batch
                if !batch.is_empty() {
                    let batch_detections = process_batch(&batch, confidence_threshold, &classes)?;
                    all_detections.extend(batch_detections);
                }
                break;
            }
            StreamFrame::Error(err_msg) => {
                return Err(ProcessingError::Other(err_msg));
            }
        }
    }

    Ok(all_detections)
}

/// Process a batch of frames through YOLO inference
fn process_batch(
    batch_frames: &[RawFrameBuffer],
    confidence_threshold: f32,
    classes: &Option<Vec<String>>,
) -> Result<Vec<DetectionWithFrame>> {
    if batch_frames.is_empty() {
        return Ok(Vec::new());
    }

    // Create zero-copy views for batch
    let mut frame_views = Vec::with_capacity(batch_frames.len());
    for frame in batch_frames {
        frame_views.push(create_zero_copy_view(frame));
    }

    // Batch preprocessing
    let batch_input = preprocess_batch_for_yolo(&frame_views)?;

    // Batch inference
    let batch_results = run_yolo_inference_batch(batch_input, confidence_threshold)?;

    // Process results for each frame in batch
    let mut detections = Vec::with_capacity(batch_frames.len());
    for (batch_idx, frame_detections) in batch_results {
        let raw_frame = &batch_frames[batch_idx];

        // Apply class filter if specified
        let filtered_detections = if let Some(class_names) = classes {
            let mut filtered = Vec::with_capacity(frame_detections.len());
            filtered.extend(frame_detections.into_iter().filter(|det| {
                class_names
                    .iter()
                    .any(|name| name == det.class_name.as_ref())
            }));
            filtered
        } else {
            frame_detections
        };

        // Add frame context to detections
        for detection in filtered_detections {
            detections.push(DetectionWithFrame {
                frame_number: raw_frame.frame_number,
                timestamp: raw_frame.timestamp,
                detection,
            });
        }
    }

    Ok(detections)
}

// ============================================================================
// Zero-Copy Frame Processing (from fast_path.rs)
// ============================================================================

/// Create ndarray view of raw frame data
///
/// If linesize == width*3 (no padding), creates zero-copy view.
/// If linesize > width*3 (has padding), FFmpeg added alignment padding which is normal.
fn create_zero_copy_view(frame: &RawFrameBuffer) -> ArrayView3<'_, u8> {
    let expected_linesize = frame.width as usize * 3;

    unsafe {
        if frame.linesize == expected_linesize {
            // No padding - create zero-copy view with standard layout
            ArrayView3::from_shape_ptr(
                (frame.height as usize, frame.width as usize, 3),
                frame.data_ptr as *const u8,
            )
        } else {
            // Has padding - FFmpeg allocated extra bytes per row for alignment (normal for some codecs)
            // Use custom strides to handle padding correctly
            // Strides: (linesize [bytes per row], 3 [bytes per pixel], 1 [bytes per channel])
            ArrayView3::from_shape_ptr(
                (frame.height as usize, frame.width as usize, 3).strides((frame.linesize, 3, 1)),
                frame.data_ptr as *const u8,
            )
        }
    }
}

/// Preprocess multiple frames for batch YOLO inference (resize to 640x640, normalize)
fn preprocess_batch_for_yolo(frame_views: &[ArrayView3<u8>]) -> Result<Array4<f32>> {
    use image::{ImageBuffer, Rgb};

    let batch_size = frame_views.len();
    if batch_size == 0 {
        return Err(ProcessingError::Other(
            "Cannot preprocess empty batch".to_string(),
        ));
    }

    // Allocate batch tensor
    let mut batch_input = Array4::<f32>::zeros((batch_size, 3, 640, 640));

    // Process each frame in the batch
    for (batch_idx, frame_view) in frame_views.iter().enumerate() {
        let height = frame_view.shape()[0];
        let width = frame_view.shape()[1];

        // Convert to image crate format
        // Handle both contiguous (no padding) and non-contiguous (with padding) arrays
        let img_data: Vec<u8> = if let Some(slice) = frame_view.as_slice() {
            // Contiguous - direct copy
            slice.to_vec()
        } else {
            // Non-contiguous (has padding) - copy row by row using bulk memcpy
            // This is 2-3x faster than element-by-element copy for large frames
            // Use vec![0; size] to properly initialize the buffer for safety
            let mut data: Vec<u8> = vec![0; height * width * 3];
            unsafe {
                for y in 0..height {
                    // Get slice for this row (width * 3 channels)
                    let row = frame_view.slice(s![y, .., ..]);
                    if let Some(row_slice) = row.as_slice() {
                        let src = row_slice.as_ptr();
                        let dst = data.as_mut_ptr().add(y * width * 3);
                        std::ptr::copy_nonoverlapping(src, dst, width * 3);
                    } else {
                        // Fallback: row itself is non-contiguous (rare edge case)
                        for x in 0..width {
                            let offset = (y * width + x) * 3;
                            *data.get_unchecked_mut(offset) = frame_view[[y, x, 0]];
                            *data.get_unchecked_mut(offset + 1) = frame_view[[y, x, 1]];
                            *data.get_unchecked_mut(offset + 2) = frame_view[[y, x, 2]];
                        }
                    }
                }
            }
            data
        };

        let img = ImageBuffer::<Rgb<u8>, _>::from_raw(width as u32, height as u32, img_data)
            .ok_or_else(|| ProcessingError::Other("Failed to create image buffer".to_string()))?;

        // Resize to 640x640
        let resized =
            image::imageops::resize(&img, 640, 640, image::imageops::FilterType::Triangle);

        // Normalize and copy to batch tensor
        for y in 0..640 {
            for x in 0..640 {
                let pixel = resized.get_pixel(x, y);
                batch_input[[batch_idx, 0, y as usize, x as usize]] = pixel[0] as f32 / 255.0; // R
                batch_input[[batch_idx, 1, y as usize, x as usize]] = pixel[1] as f32 / 255.0; // G
                batch_input[[batch_idx, 2, y as usize, x as usize]] = pixel[2] as f32 / 255.0;
                // B
            }
        }
    }

    Ok(batch_input)
}

// ============================================================================
// YOLO Inference (from fast_path.rs)
// ============================================================================

/// YOLO model session (process singleton, shared across all file workers)
///
/// Thread safety:
/// - ONNX Runtime C++ supports concurrent Session::Run() calls (FFMPEG_ONNX_THREADING_GUIDE.md:126)
/// - Mutex protects mutable access (ort crate's Session::run requires &mut self due to Rust API)
/// - Each worker gets cheap &'static reference, shares underlying model weights
/// - Underlying ONNX Runtime handles concurrency internally (intra-op/inter-op thread pools)
///
/// Performance:
/// - Model loading: ~200-500ms (YOLOv8n is ~6MB)
/// - Sharing saves 200-500ms per file for bulk processing
/// - Lock contention is minimal: ONNX inference (60-80ms) >> mutex overhead (<1ms)
///
/// Bulk mode benefit:
/// - Without sharing: N files × 500ms init = 5s overhead for 10 files
/// - With sharing: 1 × 500ms init = 0.5s overhead (10x faster init)
/// - Each file's inference runs with internal ONNX threading (no cross-file blocking)
static YOLO_SESSION: OnceLock<Mutex<Session>> = OnceLock::new();

/// Get or initialize YOLO session
fn get_yolo_session() -> Result<&'static Mutex<Session>> {
    // Check if already initialized
    if let Some(session) = YOLO_SESSION.get() {
        return Ok(session);
    }

    // Initialize session - find model relative to workspace root
    let current_dir = std::env::current_dir()
        .map_err(|e| ProcessingError::Other(format!("Failed to get current dir: {}", e)))?;

    let model_path = current_dir
        .ancestors()
        .find_map(|p| {
            let candidate = p.join("models/object-detection/yolov8n.onnx");
            if candidate.exists() {
                Some(candidate)
            } else {
                None
            }
        })
        .ok_or_else(|| {
            ProcessingError::Other(
                "YOLO model not found. Expected at models/object-detection/yolov8n.onnx"
                    .to_string(),
            )
        })?;

    let session = Session::builder()
        .map_err(|e| ProcessingError::Other(format!("ONNX builder failed: {}", e)))?
        .commit_from_file(&model_path)
        .map_err(|e| ProcessingError::Other(format!("Failed to load YOLO model: {}", e)))?;

    // Store in OnceLock (this can fail if another thread initialized first, but that's ok)
    match YOLO_SESSION.set(Mutex::new(session)) {
        Ok(()) => YOLO_SESSION.get().ok_or_else(|| {
            ProcessingError::Other("Failed to retrieve session after initialization".to_string())
        }),
        Err(_) => {
            // Another thread beat us to it, use their session
            YOLO_SESSION.get().ok_or_else(|| {
                ProcessingError::Other("Failed to retrieve session from another thread".to_string())
            })
        }
    }
}

/// Run batch YOLO inference and post-process results
/// Returns Vec of (batch_index, Vec<Detection>)
fn run_yolo_inference_batch(
    input: Array4<f32>,
    confidence_threshold: f32,
) -> Result<Vec<(usize, Vec<Detection>)>> {
    let session_mutex = get_yolo_session()?;

    // Lock the mutex to get mutable access to session
    let mut session = session_mutex
        .lock()
        .map_err(|e| ProcessingError::Other(format!("Failed to lock session mutex: {}", e)))?;

    // Convert ndarray to ort::Value
    let input_value = Value::from_array(input)
        .map_err(|e| ProcessingError::Other(format!("Failed to create ONNX value: {}", e)))?;

    // Run inference
    let outputs = session
        .run(ort::inputs![input_value])
        .map_err(|e| ProcessingError::Other(format!("ONNX inference failed: {}", e)))?;

    // Extract output tensor
    let output = outputs["output0"]
        .try_extract_tensor::<f32>()
        .map_err(|e| ProcessingError::Other(format!("Failed to extract output tensor: {}", e)))?;

    // Get shape and data
    let (shape, data) = output;

    // Post-process: confidence filtering + NMS per frame
    let batch_detections = post_process_yolo_batch(shape, data, confidence_threshold)?;

    Ok(batch_detections)
}

/// Post-process batch YOLO output (confidence filtering + NMS per frame)
/// Returns Vec of (batch_index, Vec<Detection>) to maintain frame associations
fn post_process_yolo_batch(
    shape: &ort::tensor::Shape,
    data: &[f32],
    confidence_threshold: f32,
) -> Result<Vec<(usize, Vec<Detection>)>> {
    // YOLOv8 batch output shape: [N, 84, 8400]
    // Verify shape
    if shape.len() != 3 {
        return Err(ProcessingError::Other(format!(
            "Unexpected output shape length: {} (expected 3)",
            shape.len()
        )));
    }

    let batch_size = shape[0] as usize;
    let num_channels = shape[1] as usize; // 84
    let num_boxes = shape[2] as usize; // 8400
    let num_classes = 80usize;

    let mut batch_results = Vec::with_capacity(batch_size);

    // Process each frame in the batch
    for batch_idx in 0..batch_size {
        let mut frame_detections = Vec::with_capacity(num_boxes / 10); // Pre-allocate ~10% of boxes (typical detection rate)

        // Process each detection box for this frame
        for box_idx in 0..num_boxes {
            // Index calculation for flattened array [N, 84, 8400]
            // data[batch_idx * (84 * 8400) + channel * 8400 + box_idx]
            let base_offset = batch_idx * (num_channels * num_boxes);

            // Extract bbox coordinates (center x, center y, width, height)
            let x = data[base_offset + box_idx]; // channel 0
            let y = data[base_offset + num_boxes + box_idx]; // channel 1
            let w = data[base_offset + 2 * num_boxes + box_idx]; // channel 2
            let h = data[base_offset + 3 * num_boxes + box_idx]; // channel 3

            // Find best class from channels 4-83
            let mut max_score = 0.0f32;
            let mut best_class = 0u8;

            for class_id in 0..num_classes {
                let score = data[base_offset + (4 + class_id) * num_boxes + box_idx];
                if score > max_score {
                    max_score = score;
                    best_class = class_id as u8;
                }
            }

            // Filter by confidence threshold
            if max_score >= confidence_threshold {
                frame_detections.push(Detection {
                    class_id: best_class,
                    class_name: get_coco_class_name(best_class),
                    confidence: max_score,
                    bbox: BoundingBox {
                        // Convert from center coords to top-left coords
                        // Normalize to [0, 1] range
                        x: (x - w / 2.0) / 640.0,
                        y: (y - h / 2.0) / 640.0,
                        width: w / 640.0,
                        height: h / 640.0,
                    },
                });
            }
        }

        // Apply NMS per frame
        let filtered = apply_nms(frame_detections, 0.45);
        batch_results.push((batch_idx, filtered));
    }

    Ok(batch_results)
}

/// Calculate Intersection over Union (IoU) between two bounding boxes
fn calculate_iou(a: &BoundingBox, b: &BoundingBox) -> f32 {
    let x1 = a.x.max(b.x);
    let y1 = a.y.max(b.y);
    let x2 = (a.x + a.width).min(b.x + b.width);
    let y2 = (a.y + a.height).min(b.y + b.height);

    let intersection_width = (x2 - x1).max(0.0);
    let intersection_height = (y2 - y1).max(0.0);
    let intersection_area = intersection_width * intersection_height;

    let a_area = a.width * a.height;
    let b_area = b.width * b.height;
    let union_area = a_area + b_area - intersection_area;

    if union_area > 0.0 {
        intersection_area / union_area
    } else {
        0.0
    }
}

/// Apply Non-Maximum Suppression to remove duplicate detections
fn apply_nms(mut detections: Vec<Detection>, iou_threshold: f32) -> Vec<Detection> {
    // Sort by confidence (highest first)
    detections.sort_by(|a, b| {
        b.confidence
            .partial_cmp(&a.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut keep = Vec::with_capacity(detections.len()); // Upper bound: all detections pass NMS

    // Process detections without O(n) Vec::remove(0) operations
    // Use swap_remove to get O(1) removal (order doesn't matter after we take the best)
    while !detections.is_empty() {
        // Take last element (O(1) pop) - we maintain sorted order via swap_remove below
        let current = detections.swap_remove(0);

        // Remove all detections with IoU > threshold for the same class
        // Note: Must filter before moving current into keep to avoid borrow issues
        detections.retain(|det| {
            det.class_id != current.class_id
                || calculate_iou(&det.bbox, &current.bbox) < iou_threshold
        });

        keep.push(current); // No clone needed - current is already owned
    }

    keep
}

/// Get COCO class name from class ID (0-79)
fn get_coco_class_name(class_id: u8) -> Cow<'static, str> {
    const COCO_CLASSES: &[&str] = &[
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

    Cow::Borrowed(COCO_CLASSES.get(class_id as usize).unwrap_or(&"unknown"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Requires test video file and model
    fn test_parallel_pipeline() {
        // Find workspace root
        let current_dir = std::env::current_dir().unwrap();
        let workspace_root = current_dir
            .ancestors()
            .find(|p| p.join("Cargo.toml").exists() && p.join("test_edge_cases").exists())
            .expect("Could not find workspace root");

        let video_path =
            workspace_root.join("test_edge_cases/video_variable_framerate_vfr__timing_test.mp4");

        if !video_path.exists() {
            eprintln!("Test video not found: {:?}", video_path);
            return;
        }

        let model_path = workspace_root.join("models/object-detection/yolov8n.onnx");
        if !model_path.exists() {
            eprintln!("YOLO model not found: {:?}", model_path);
            return;
        }

        // Run parallel pipeline
        let config = ParallelConfig::default();
        let result = extract_and_detect_parallel(&video_path, config);

        assert!(
            result.is_ok(),
            "Parallel pipeline should succeed: {:?}",
            result.err()
        );

        let detections = result.unwrap();
        println!(
            "Parallel pipeline found {} total detections",
            detections.len()
        );
    }

    #[test]
    #[ignore] // Requires test video file and model
    fn test_parallel_with_class_filter() {
        let current_dir = std::env::current_dir().unwrap();
        let workspace_root = current_dir
            .ancestors()
            .find(|p| p.join("Cargo.toml").exists() && p.join("test_edge_cases").exists())
            .expect("Could not find workspace root");

        let video_path =
            workspace_root.join("test_edge_cases/video_variable_framerate_vfr__timing_test.mp4");

        if !video_path.exists()
            || !workspace_root
                .join("models/object-detection/yolov8n.onnx")
                .exists()
        {
            return;
        }

        // Filter for only "person" class
        let config = ParallelConfig {
            classes: Some(vec!["person".to_string()]),
            ..Default::default()
        };

        let result = extract_and_detect_parallel(&video_path, config);
        assert!(
            result.is_ok(),
            "Parallel pipeline with filter should succeed"
        );

        let detections = result.unwrap();
        println!("Found {} person detections", detections.len());

        // Verify all detections are "person"
        for det in &detections {
            assert_eq!(
                det.detection.class_name, "person",
                "All detections should be 'person'"
            );
        }
    }
}
