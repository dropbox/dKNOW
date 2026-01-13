# ML Models - Setup and Configuration

This document describes the machine learning models used in the video-extract system, their setup, and how to re-export models with custom configurations.

## Overview

The system uses ONNX Runtime for all neural network inference, enabling:
- **Hardware acceleration**: CoreML (macOS), CUDA (NVIDIA GPUs)
- **Cross-platform compatibility**: Linux, macOS, Windows
- **Python-free runtime**: 100% native Rust/C++ implementation

All models are stored in `models/` directory (gitignored, users must download locally).

## Model Inventory

### Core Models (Required)

1. **YOLOv8n** - Object Detection (12MB)
   - Path: `models/object-detection/yolov8n.onnx`
   - Input: RGB images (batch, 3, 640, 640)
   - Output: 80 COCO object classes
   - Batch inference: Enabled (dynamic batch size)
   - Source: Ultralytics YOLOv8

2. **Whisper Base** - Speech Transcription (145MB)
   - Path: `models/whisper-base/ggml-base.bin`
   - Format: GGML (whisper.cpp)
   - Languages: 99 languages supported
   - Source: OpenAI Whisper

3. **RetinaFace** - Face Detection
   - Path: `models/face-detection/retinaface.onnx`
   - Input: RGB images (1, 3, 640, 640)
   - Output: Face bounding boxes + 5-point landmarks
   - Source: biubug6/Pytorch_Retinaface

4. **PaddleOCR** - Text Extraction
   - Detection: `models/ocr/ch_PP-OCRv3_det_infer.onnx`
   - Recognition: `models/ocr/ch_PP-OCRv3_rec_infer.onnx`
   - Languages: Chinese + English
   - Source: PaddlePaddle/PaddleOCR

5. **WeSpeaker** - Speaker Diarization
   - Path: `models/diarization/wespeaker_en_voxceleb_CAM++.onnx`
   - Input: Audio embeddings
   - Output: Speaker identity vectors
   - Source: wenet-e2e/wespeaker

6. **CLIP ViT-B/32** - Vision Embeddings (577MB)
   - Path: `models/embeddings/clip-vit-base-patch32.onnx`
   - Input: RGB images (1, 3, 224, 224)
   - Output: 512-dimensional embeddings
   - Source: OpenAI CLIP

7. **Sentence-Transformers** - Text Embeddings
   - Path: `models/embeddings/all-MiniLM-L6-v2.onnx`
   - Input: Tokenized text
   - Output: 384-dimensional embeddings
   - Source: sentence-transformers/all-MiniLM-L6-v2

8. **CLAP** - Audio Embeddings
   - Path: `models/embeddings/clap-htsat-unfused.onnx`
   - Input: Audio spectrograms
   - Output: 512-dimensional embeddings
   - Source: LAION-AI/CLAP

9. **YAMNet** - Audio Classification
   - Path: `models/audio-classification/yamnet.onnx`
   - Input: Audio waveform
   - Output: 521 audio event classes
   - Source: Google/yamnet

10. **X3D** - Action Recognition
    - Path: `models/action-recognition/x3d_m.onnx`
    - Input: Video clips (1, 3, 16, 224, 224)
    - Output: 400 Kinetics-400 action classes
    - Source: Facebook Research X3D

### Optional Models (User-Provided)

11. **YOLOv8-Pose** - Human Pose Estimation
    - Path: `models/pose-estimation/yolov8n-pose.onnx`
    - Output: 17 COCO keypoints per person
    - Setup: User must export from Ultralytics

12. **Depth Estimation** - Monocular Depth
    - Path: `models/depth-estimation/{midas,dpt}_*.onnx`
    - Examples: MiDaS, DPT-Large
    - Setup: User must export from Intel isl-org/MiDaS

13. **Caption Generation** - Image-to-Text
    - Path: `models/caption-generation/{blip,llava}_*.onnx`
    - Examples: BLIP, BLIP-2, LLaVA, ViT-GPT2
    - Setup: User must export from HuggingFace

14. **Logo Detection** - Brand Logos
    - Path: `models/logo-detection/yolov8_logo.onnx`
    - Output: Custom logo classes
    - Setup: User must train custom YOLOv8 model

15. **Music Source Separation** - Audio Stems
    - Path: `models/music-separation/{demucs,spleeter}_*.onnx`
    - Output: Vocals, drums, bass, other
    - Setup: User must export Demucs/Spleeter

## Model Setup

### Automatic Download (CI/CD)

The GitHub Actions CI/CD workflow automatically downloads all required models:

```yaml
# .github/workflows/ci.yml
- name: Download ML models
  run: |
    mkdir -p models/object-detection
    wget https://github.com/ultralytics/assets/releases/download/v0.0.0/yolov8n.onnx \
      -O models/object-detection/yolov8n.onnx
    # ... (other models)
```

### Manual Download

For local development, download models manually:

```bash
# Create directories
mkdir -p models/{object-detection,whisper-base,face-detection,ocr,diarization,embeddings,audio-classification,action-recognition}

# YOLOv8n (object detection)
wget https://github.com/ultralytics/assets/releases/download/v0.0.0/yolov8n.onnx \
  -O models/object-detection/yolov8n.onnx

# Whisper base (transcription)
wget https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin \
  -O models/whisper-base/ggml-base.bin

# RetinaFace (face detection)
wget https://github.com/onnx/models/raw/main/vision/body_analysis/retinaface/retinaface-resnet50.onnx \
  -O models/face-detection/retinaface.onnx

# ... (other models, see CI/CD workflow for complete list)
```

## Re-Exporting Models

### YOLOv8 with Dynamic Batch Size

**Problem**: Default YOLOv8 export uses fixed `batch_size=1`, preventing batch inference.

**Solution**: Re-export with `dynamic=True` flag.

```python
# yolo_export.py
from ultralytics import YOLO

# Load pre-trained model
model = YOLO('yolov8n.pt')

# Export with dynamic batch size
model.export(
    format='onnx',
    imgsz=640,           # Input size (640x640)
    dynamic=True,        # Enable dynamic batch size
    simplify=True,       # ONNX simplification
    opset=13,            # ONNX opset version
)

print("Model exported: yolov8n.onnx")
```

**Verify dynamic batch size**:

```python
import onnx

# Load model
model = onnx.load('yolov8n.onnx')

# Check input shape
input_shape = model.graph.input[0].type.tensor_type.shape
print(f"Input shape: {[dim.dim_value or dim.dim_param for dim in input_shape.dim]}")
# Expected: ['batch', 3, 'height', 'width']
# batch, height, width are dynamic (not fixed integers)
```

**Usage in video-extract**:

```bash
# Fast path uses BATCH_SIZE=8 for batch inference
video-extract fast --op keyframes+detect video.mp4

# Parallel pipeline uses BATCH_SIZE=8 (configurable)
video-extract fast --op keyframes+detect --parallel video.mp4
```

**Performance**:
- Fixed batch_size=1: 1 frame per ONNX call
- Dynamic batch_size: 8 frames per ONNX call (1.5-2x speedup for videos with ≥16 keyframes)

### YOLOv8-Pose with Dynamic Batch Size

```python
from ultralytics import YOLO

model = YOLO('yolov8n-pose.pt')
model.export(
    format='onnx',
    imgsz=640,
    dynamic=True,
    simplify=True,
)
```

### Custom Logo Detection Model

```python
from ultralytics import YOLO

# Train custom model on logo dataset
model = YOLO('yolov8n.pt')
model.train(
    data='logos.yaml',  # Dataset config
    epochs=100,
    imgsz=640,
)

# Export trained model
model.export(
    format='onnx',
    imgsz=640,
    dynamic=True,
    simplify=True,
)
```

### HuggingFace Models (CLIP, Sentence-Transformers, CLAP)

```python
from optimum.onnxruntime import ORTModelForFeatureExtraction
from transformers import AutoTokenizer

# Export CLIP text encoder
model = ORTModelForFeatureExtraction.from_pretrained(
    "openai/clip-vit-base-patch32",
    export=True,
)
model.save_pretrained("models/embeddings/clip-vit-base-patch32")

# Export Sentence-Transformers
model = ORTModelForFeatureExtraction.from_pretrained(
    "sentence-transformers/all-MiniLM-L6-v2",
    export=True,
)
model.save_pretrained("models/embeddings/all-MiniLM-L6-v2")
```

## Model Configuration

### ONNX Runtime Settings

The system uses aggressive ONNX Runtime optimizations (see N=100 in git history):

```rust
// crates/object-detection/src/lib.rs
let session = SessionBuilder::new(&environment)?
    .with_optimization_level(GraphOptimizationLevel::Level3)?  // Max optimization
    .with_intra_threads(num_cpus::get() as i16)?              // Parallel execution
    .with_model_from_file("models/object-detection/yolov8n.onnx")?;
```

**Optimizations applied**:
- Graph optimization level 3 (all fusions, transformations)
- Intra-op parallelism (multi-threaded operators)
- Memory pattern optimization
- Constant folding, operator fusion

**Performance impact**: +15-25% inference speedup (N=100)

### Hardware Acceleration

#### macOS (CoreML)

```rust
// Enable CoreML GPU acceleration (N=173)
let session = SessionBuilder::new(&environment)?
    .with_optimization_level(GraphOptimizationLevel::Level3)?
    .with_execution_providers([
        ExecutionProvider::CoreML(Default::default()),  // GPU acceleration
        ExecutionProvider::CPU(Default::default()),     // CPU fallback
    ])?
    .with_model_from_file(model_path)?;
```

**Performance**: 1.35x speedup (26% faster test suite, N=173)

#### Linux/Windows (CUDA)

```rust
let session = SessionBuilder::new(&environment)?
    .with_optimization_level(GraphOptimizationLevel::Level3)?
    .with_execution_providers([
        ExecutionProvider::CUDA(Default::default()),  // NVIDIA GPU
        ExecutionProvider::CPU(Default::default()),   // CPU fallback
    ])?
    .with_model_from_file(model_path)?;
```

**Note**: CUDA support requires `onnxruntime` crate built with CUDA feature.

## Batch Inference

### Configuration

```rust
// crates/video-extract-core/src/fast_path.rs
const BATCH_SIZE: usize = 8;  // Process 8 frames per ONNX call

// crates/video-extract-core/src/parallel_pipeline.rs
pub struct ParallelConfig {
    pub batch_size: usize,  // Default: 8
    // ...
}
```

### Trade-offs

**Benefits**:
- Reduced ONNX Runtime overhead (1 call vs 8 calls)
- Better GPU utilization (parallel frame processing)
- 1.5-2x speedup for videos with ≥16 keyframes

**Costs**:
- ~20% per-frame overhead for batch padding/processing
- Requires dynamic batch size model export
- Limited benefit for videos with <8 keyframes

**Optimal use cases**:
- Videos with 16+ keyframes: 1.5-2x speedup
- Videos with 8-16 keyframes: 1.2-1.5x speedup
- Videos with <8 keyframes: Minimal benefit (<10%)

### Validation

```bash
# Test batch inference
cargo test --release test_batch_inference -- --ignored

# Benchmark batch inference
TEST_VIDEO="test_files_wikimedia/mkv/emotion-detection/01_h264_from_mp4.mkv"
for i in {1..5}; do
  time target/release/video-extract fast --op keyframes+detect "$TEST_VIDEO"
done
```

Expected: 0.56-0.62s for 4.6MB video (N=26 validated)

## Model Troubleshooting

### "Got invalid dimensions for input" Error

**Symptom**: ONNX Runtime error: `Got invalid dimensions for input: images - Got: 8 Expected: 1`

**Cause**: Model exported with fixed `batch_size=1`, not dynamic batch size.

**Solution**: Re-export model with `dynamic=True` flag (see above).

### CoreML Incompatibility (INT8 Quantization)

**Symptom**: Models fail to load on macOS after INT8 quantization.

**Cause**: CoreML execution provider requires FP32 models (N=153).

**Solution**: Do NOT quantize models on macOS. Use FP32 models with CoreML acceleration.

### Model Not Found Error

**Symptom**: `Failed to load model: No such file or directory`

**Cause**: Model file not downloaded or in wrong location.

**Solution**:
1. Check `models/` directory exists: `ls -la models/`
2. Download missing model (see Manual Download section)
3. Verify file path matches code expectations

### Performance Regression

**Symptom**: Inference slower after model update.

**Cause**: Model exported without simplification or optimization.

**Solution**:
1. Re-export with `simplify=True`
2. Verify graph optimization level 3 in code
3. Benchmark before/after: `hyperfine "video-extract fast --op keyframes+detect video.mp4"`

## CI/CD Integration

### GitHub Actions Workflow

```yaml
# .github/workflows/ci.yml
- name: Download ML models
  run: |
    mkdir -p models/object-detection
    # YOLOv8n (IMPORTANT: Use dynamic batch size model)
    wget https://github.com/ultralytics/assets/releases/download/v0.0.0/yolov8n.onnx \
      -O models/object-detection/yolov8n.onnx
```

**Note**: GitHub releases may contain fixed batch_size=1 models. For production, host custom-exported dynamic models.

### Model Caching

```yaml
- name: Cache ML models
  uses: actions/cache@v3
  with:
    path: models/
    key: ml-models-${{ hashFiles('MODELS.md') }}
    restore-keys: |
      ml-models-
```

## Future Work

### Model Quantization (INT8)

**Status**: Not viable on macOS (CoreML incompatibility, N=153)

**Linux/Windows**: INT8 quantization could provide:
- 4x model size reduction (12MB → 3MB for YOLOv8n)
- 2-3x inference speedup (CPU)
- Minimal accuracy loss (<1% mAP)

**Implementation**: Use `onnxruntime-tools` for quantization:

```python
from onnxruntime.quantization import quantize_dynamic

quantize_dynamic(
    model_input='yolov8n.onnx',
    model_output='yolov8n_int8.onnx',
    weight_type=QuantType.QInt8,
)
```

### Model Pruning

**Potential**: 30-50% speedup for vision models by removing low-importance weights.

**Tools**: PyTorch pruning, TensorFlow Model Optimization Toolkit.

### Custom Model Training

**Use cases**:
- Domain-specific object detection (medical, industrial)
- Custom logo/brand detection
- Fine-tuned action recognition for specific activities

**Process**:
1. Collect labeled dataset
2. Train YOLOv8/X3D on custom data
3. Export to ONNX with `dynamic=True`
4. Integrate into video-extract

## References

- **ONNX Runtime**: https://onnxruntime.ai/
- **Ultralytics YOLOv8**: https://github.com/ultralytics/ultralytics
- **Whisper.cpp**: https://github.com/ggerganov/whisper.cpp
- **PaddleOCR**: https://github.com/PaddlePaddle/PaddleOCR
- **CLIP**: https://github.com/openai/CLIP
- **Sentence-Transformers**: https://www.sbert.net/
- **CLAP**: https://github.com/LAION-AI/CLAP
- **YAMNet**: https://github.com/tensorflow/models/tree/master/research/audioset/yamnet
- **X3D**: https://github.com/facebookresearch/pytorchvideo

## Version History

- **N=27**: Created MODELS.md with comprehensive model documentation
- **N=26**: Enabled YOLOv8 batch inference (BATCH_SIZE 1→8, dynamic model export)
- **N=23**: Fixed batch inference bug (reduced BATCH_SIZE to 1 due to fixed model)
- **N=173**: Enabled CoreML GPU acceleration (1.35x speedup)
- **N=153**: Investigated INT8 quantization (not viable on macOS)
- **N=100**: ONNX Runtime graph optimization (+15-25% speedup)
