# Video Audio Object Detection

YOLOv8-based object detection for video frames using ONNX Runtime.

## Features

- Multiple YOLOv8 model sizes (Nano to XLarge)
- 80 COCO object classes
- Hardware acceleration via ONNX Runtime (CUDA, TensorRT, CoreML)
- Configurable confidence and IoU thresholds
- Non-maximum suppression for duplicate removal
- Class filtering for targeted detection

## Model Setup

### Download Pre-trained ONNX Models

YOLOv8 models need to be downloaded separately. Use the Ultralytics Python package:

```bash
pip install ultralytics
python3 -c "from ultralytics import YOLO; YOLO('yolov8n.pt').export(format='onnx')"
```

This exports `yolov8n.onnx` to the current directory.

Available models:
- `yolov8n.onnx` - Nano (6MB, fastest)
- `yolov8s.onnx` - Small (22MB)
- `yolov8m.onnx` - Medium (52MB)
- `yolov8l.onnx` - Large (87MB)
- `yolov8x.onnx` - XLarge (136MB, most accurate)

### Direct Download (Alternative)

If you have pre-exported ONNX models, place them in a `models/` directory:

```bash
mkdir -p models
# Copy your yolov8n.onnx file here
```

## Usage

```rust
use video_audio_object_detection::{ObjectDetector, ObjectDetectionConfig};
use image::open;

// Load model
let config = ObjectDetectionConfig::default();
let mut detector = ObjectDetector::new("models/yolov8n.onnx", config)?;

// Detect objects in an image
let img = open("image.jpg")?.to_rgb8();
let detections = detector.detect(&img)?;

for detection in detections {
    println!("{}: {:.2}% at ({:.2}, {:.2})",
        detection.class_name,
        detection.confidence * 100.0,
        detection.bbox.x,
        detection.bbox.y
    );
}
```

## Configuration Presets

```rust
// Fast detection (higher thresholds, fewer detections)
let config = ObjectDetectionConfig::fast();

// Accurate detection (lower thresholds, more detections)
let config = ObjectDetectionConfig::accurate();

// Person detection only
let config = ObjectDetectionConfig::person_only();

// Custom config
let config = ObjectDetectionConfig {
    confidence_threshold: 0.3,
    iou_threshold: 0.45,
    classes: Some(vec![0, 1, 2]), // person, bicycle, car only
    max_detections: 100,
    input_size: 640,
};
```

## COCO Classes

The model detects 80 COCO object classes:

- Person, bicycle, car, motorcycle, airplane, bus, train, truck, boat
- Traffic light, fire hydrant, stop sign, parking meter, bench
- Bird, cat, dog, horse, sheep, cow, elephant, bear, zebra, giraffe
- Backpack, umbrella, handbag, tie, suitcase, frisbee, skis, snowboard
- Sports ball, kite, baseball bat, baseball glove, skateboard, surfboard
- Tennis racket, bottle, wine glass, cup, fork, knife, spoon, bowl
- Banana, apple, sandwich, orange, broccoli, carrot, hot dog, pizza
- Donut, cake, chair, couch, potted plant, bed, dining table, toilet
- TV, laptop, mouse, remote, keyboard, cell phone, microwave, oven
- Toaster, sink, refrigerator, book, clock, vase, scissors, teddy bear
- Hair drier, toothbrush

## Testing

Unit tests:

```bash
cargo test --package video-audio-object-detection --lib
```

Integration tests (requires ONNX model):

```bash
# Ensure model exists at models/yolov8n.onnx first
cargo test --package video-audio-object-detection --test integration_test -- --ignored
```

## Hardware Acceleration

ONNX Runtime supports multiple execution providers:

- **CUDA**: NVIDIA GPUs (enable with `cuda` feature in workspace Cargo.toml)
- **TensorRT**: Optimized NVIDIA inference
- **CoreML**: Apple Silicon acceleration
- **DirectML**: Windows GPU acceleration

The workspace is configured with CUDA support. To use CoreML on macOS, modify the workspace `ort` dependency.

## Performance

Model inference times (approximate, depends on hardware):

- YOLOv8 Nano: ~10-20ms per frame (GPU), ~50-100ms (CPU)
- YOLOv8 Small: ~20-30ms per frame (GPU), ~100-200ms (CPU)
- YOLOv8 Medium: ~30-50ms per frame (GPU), ~200-400ms (CPU)

## API Reference

See [lib.rs](src/lib.rs) for detailed API documentation.

## Status

✅ Core API implemented
✅ Unit tests pass (9/9)
⚠️ Integration tests require ONNX model download
⏳ Hardware acceleration testing pending
