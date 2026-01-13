# Next Steps: N=11 ONNX Zero-Copy Implementation

**Status**: Phase 1 Complete (C FFI decoder working ✅)
**Next**: Implement zero-copy ONNX inference in fast_path.rs
**Context**: 11.6% used (116k/1000k), 88.4% available

## What Works Now

1. ✅ **decode_iframes_zero_copy()** - Working C FFI decoder
   - File: crates/video-decoder/src/c_ffi.rs
   - Returns: Vec<RawFrameBuffer> with AVFrame* pointers
   - Test: crates/video-decoder/tests/c_ffi_test.rs (passing)
   - Performance: 1.66s for ultra-HD video (3446x1996)

2. ✅ **Fast-path infrastructure** - Pipeline skeleton
   - File: crates/video-extract-core/src/fast_path.rs
   - API: extract_and_detect_zero_copy()
   - Status: Stub implementation (returns empty Vec)

3. ✅ **Build system** - All dependencies ready
   - ffmpeg-sys-next: C FFI bindings
   - ort: ONNX Runtime (CoreML GPU support)
   - ndarray: Zero-copy array views

## What Needs Implementation

### File: crates/video-extract-core/src/fast_path.rs

**Function**: `detect_objects_from_raw_frame()`
**Input**: `&RawFrameBuffer` (width, height, data_ptr, linesize)
**Output**: `Vec<Detection>` (class_id, class_name, confidence, bbox)

### Step-by-Step Implementation

#### Step 1: Load YOLO Model (One-time)

```rust
use ort::Session;
use std::sync::OnceLock;

static YOLO_SESSION: OnceLock<Session> = OnceLock::new();

fn get_yolo_session() -> &'static Session {
    YOLO_SESSION.get_or_init(|| {
        // Load YOLOv8 model with CoreML acceleration
        let model_path = "models/object-detection/yolov8n.onnx";

        Session::builder()
            .unwrap()
            .with_execution_providers([
                ort::ExecutionProvider::CoreML(Default::default()),
                ort::ExecutionProvider::CPU(Default::default()),
            ])
            .unwrap()
            .with_optimization_level(ort::GraphOptimizationLevel::Level3)
            .unwrap()
            .with_model_from_file(model_path)
            .unwrap()
    })
}
```

#### Step 2: Create ndarray::ArrayView (Zero-Copy)

```rust
use ndarray::{ArrayView3, s};

fn create_zero_copy_view(frame: &RawFrameBuffer) -> ArrayView3<'_, u8> {
    unsafe {
        // Create zero-copy view of RGB24 data
        // Shape: [height, width, 3] (RGB channels)
        ArrayView3::from_shape_ptr(
            (frame.height as usize, frame.width as usize, 3),
            frame.data_ptr as *const u8
        )
    }
}
```

**IMPORTANT**: This assumes RGB24 format with no row padding. If linesize != width * 3, you need to handle stride.

#### Step 3: Preprocess for YOLO (Resize + Normalize)

```rust
use image::{ImageBuffer, Rgb};

fn preprocess_for_yolo(frame_view: ArrayView3<u8>) -> ndarray::Array4<f32> {
    // YOLOv8 expects: [1, 3, 640, 640] (batch=1, channels=3, height=640, width=640)

    // Convert to image crate format
    let height = frame_view.shape()[0];
    let width = frame_view.shape()[1];

    // This DOES allocate memory (unavoidable for resize/normalize)
    let img = ImageBuffer::<Rgb<u8>, _>::from_raw(
        width as u32,
        height as u32,
        frame_view.as_slice().unwrap().to_vec()  // TODO: Avoid this copy if possible
    ).unwrap();

    // Resize to 640x640 (YOLO input size)
    let resized = image::imageops::resize(
        &img,
        640,
        640,
        image::imageops::FilterType::Triangle
    );

    // Convert to ndarray and normalize
    let mut input = ndarray::Array4::<f32>::zeros((1, 3, 640, 640));

    for y in 0..640 {
        for x in 0..640 {
            let pixel = resized.get_pixel(x, y);
            input[[0, 0, y as usize, x as usize]] = pixel[0] as f32 / 255.0; // R
            input[[0, 1, y as usize, x as usize]] = pixel[1] as f32 / 255.0; // G
            input[[0, 2, y as usize, x as usize]] = pixel[2] as f32 / 255.0; // B
        }
    }

    input
}
```

**NOTE**: This step DOES allocate memory for resize/normalize. True zero-copy for preprocessing would require SIMD-optimized resize (future optimization).

#### Step 4: Run ONNX Inference

```rust
fn run_yolo_inference(input: ndarray::Array4<f32>) -> Vec<Detection> {
    let session = get_yolo_session();

    // Create ONNX tensor from ndarray
    let input_tensor = ort::inputs!["images" => input.view()].unwrap();

    // Run inference
    let outputs = session.run(input_tensor).unwrap();

    // Parse YOLOv8 output format
    // Output shape: [1, 84, 8400]
    // 84 = 4 bbox coords + 80 class scores
    let output = outputs["output0"]
        .try_extract_tensor::<f32>()
        .unwrap();

    let output_data = output.view();

    // Post-process: NMS, confidence filtering
    let detections = post_process_yolo(output_data);

    detections
}
```

#### Step 5: Post-process (NMS, Confidence Filtering)

```rust
fn post_process_yolo(output: ndarray::ArrayView3<f32>) -> Vec<Detection> {
    let mut detections = Vec::new();

    // YOLOv8 output: [1, 84, 8400]
    // Each of 8400 boxes has: [x, y, w, h, class0_score, class1_score, ..., class79_score]

    for i in 0..8400 {
        // Extract bbox coordinates
        let x = output[[0, 0, i]];
        let y = output[[0, 1, i]];
        let w = output[[0, 2, i]];
        let h = output[[0, 3, i]];

        // Find best class
        let mut max_score = 0.0f32;
        let mut best_class = 0u8;

        for class_id in 0..80 {
            let score = output[[0, 4 + class_id, i]];
            if score > max_score {
                max_score = score;
                best_class = class_id as u8;
            }
        }

        // Filter by confidence threshold
        if max_score > 0.25 {
            detections.push(Detection {
                class_id: best_class,
                class_name: get_coco_class_name(best_class),
                confidence: max_score,
                bbox: BoundingBox {
                    x: (x - w / 2.0) / 640.0,
                    y: (y - h / 2.0) / 640.0,
                    width: w / 640.0,
                    height: h / 640.0,
                },
            });
        }
    }

    // Apply NMS (Non-Maximum Suppression)
    let filtered = apply_nms(detections, 0.45);

    filtered
}
```

#### Step 6: Helper Functions

```rust
fn get_coco_class_name(class_id: u8) -> String {
    const COCO_CLASSES: &[&str] = &[
        "person", "bicycle", "car", "motorcycle", "airplane",
        "bus", "train", "truck", "boat", "traffic light",
        // ... (80 classes total, copy from crates/object-detection/src/lib.rs)
    ];

    COCO_CLASSES.get(class_id as usize)
        .unwrap_or(&"unknown")
        .to_string()
}

fn apply_nms(detections: Vec<Detection>, iou_threshold: f32) -> Vec<Detection> {
    // TODO: Implement NMS algorithm
    // For now, return all detections (will implement proper NMS in next iteration)
    detections
}
```

### Complete Implementation (All Steps Combined)

**File**: crates/video-extract-core/src/fast_path.rs

Replace the stub `detect_objects_from_raw_frame()` with the above implementation.

### Dependencies Needed

Add to `crates/video-extract-core/Cargo.toml`:

```toml
[dependencies]
# ... existing dependencies ...
image = "0.25"  # For resize/preprocess
ndarray = "0.16"  # For array operations
ort = "2.0"  # ONNX Runtime (should already be present)
video-audio-decoder = { path = "../video-decoder" }  # For RawFrameBuffer
```

## Testing Plan

1. **Unit test**: Test with single RawFrameBuffer
   ```bash
   cargo test --release -p video-extract-core zero_copy_pipeline
   ```

2. **Integration test**: Test full pipeline
   ```rust
   #[test]
   #[ignore]
   fn test_extract_and_detect_zero_copy() {
       let video = Path::new("test_edge_cases/video_variable_framerate_vfr__timing_test.mp4");
       let detections = extract_and_detect_zero_copy(video, 0.25, None).unwrap();
       assert!(!detections.is_empty(), "Should find some objects");
       println!("Found {} detections", detections.len());
   }
   ```

3. **Benchmark**: Compare vs plugin system
   ```bash
   # Zero-copy fast path
   time ./target/release/video-extract fast keyframes+detect video.mp4

   # Plugin system
   time ./target/release/video-extract debug -i video.mp4 --ops keyframes,object-detection
   ```

## Expected Performance

**Current (plugin system)**:
- Keyframes: 0.40s (decode + write JPEG)
- Object-detection: 1.51s (read JPEG + ONNX)
- Total: 1.91s

**With zero-copy**:
- Decode: 0.40s (C FFI, same as before)
- ONNX: 1.46s (no JPEG read, ~0.05s saved)
- Total: 1.86s (**2.6% faster**, 0.05s saved from disk I/O elimination)

**NOTE**: This is less than the 11% originally estimated because:
- N=9 analysis measured disk I/O at 5.2% (0.10s)
- ONNX inference dominates (76.4% of time)
- Zero-copy eliminates disk I/O but not ONNX overhead

**Real speedup will come from**:
- Phase 2: Batch inference (2-3x on ONNX)
- Phase 3: Parallel pipeline (1.5-2x overall)
- Phase 4: SIMD preprocessing (10-15% on resize)

## Known Issues / Limitations

1. **Preprocessing allocates memory**: Resize/normalize requires allocation
   - Future: Use SIMD-optimized resize (save ~20ms)

2. **No batch inference yet**: Processing one frame at a time
   - Future: Batch multiple frames (2-3x faster)

3. **NMS not implemented**: Returns all detections above threshold
   - Future: Implement proper NMS (IoU-based filtering)

4. **Model loading**: Currently loads on first call
   - Could be slow (~500ms first time)
   - Subsequent calls reuse cached session

## Success Criteria

1. ✅ Code compiles without errors
2. ✅ Test passes with real video file
3. ✅ Detections are reasonable (correct classes, confidence)
4. ✅ Performance is ≥2% faster than plugin system
5. ✅ Memory usage is acceptable (<2GB for ultra-HD)

## Next Worker: Start Here

1. Read this file
2. Implement Step 1-6 in crates/video-extract-core/src/fast_path.rs
3. Add dependencies to Cargo.toml
4. Run tests
5. Benchmark
6. Commit as N=11

**Estimated**: 6-8 commits, ~3-4 hours AI time

**Context**: Start with 88.4% available, should finish with >70% available

**Report**: reports/build-video-audio-extracts/onnx_zero_copy_N11_20251030.md
