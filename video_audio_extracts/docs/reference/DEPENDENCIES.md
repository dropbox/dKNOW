# Dependencies - Complete Reference

Complete audit of all dependencies across 24 crates in the workspace.

## Workspace-Level Dependencies (Shared)

### Core Infrastructure

| Dependency | Version | Purpose | Features |
|------------|---------|---------|----------|
| `ffmpeg-next` | 8.0 | FFmpeg bindings for video/audio decode | All codecs, formats |
| `tokio` | 1.35 | Async runtime | full |
| `ort` | 2.0.0-rc.10 | ONNX Runtime (ML inference) | cuda, coreml |

### Image Processing

| Dependency | Version | Purpose | Features |
|------------|---------|---------|----------|
| `image` | 0.25 | Image I/O and manipulation | jpeg, png |
| `imageproc` | 0.25 | Image processing algorithms | - |
| `ndarray` | 0.16 | N-dimensional arrays (ML tensors) | - |

### Error Handling

| Dependency | Version | Purpose |
|------------|---------|---------|
| `thiserror` | 2.0 | Error type derivation |
| `anyhow` | 1.0 | Flexible error handling |

### Serialization

| Dependency | Version | Purpose |
|------------|---------|---------|
| `serde` | 1.0 | Serialization framework |
| `serde_json` | 1.0 | JSON support |

### Storage (api-server only)

| Dependency | Version | Purpose |
|------------|---------|---------|
| `aws-sdk-s3` | 1.63 | S3 object storage |
| `qdrant-client` | 1.13 | Vector database |
| `tokio-postgres` | 0.7 | PostgreSQL client |

### Logging

| Dependency | Version | Purpose | Features |
|------------|---------|---------|----------|
| `tracing` | 0.1 | Structured logging | - |
| `tracing-subscriber` | 0.3 | Log subscriber | env-filter, fmt |

### Testing

| Dependency | Version | Purpose | Features |
|------------|---------|---------|----------|
| `criterion` | 0.5 | Benchmarking framework | html_reports |

## CLI Crate (video-extract-cli)

### Core Dependencies

| Dependency | Version | Purpose |
|------------|---------|---------|
| `video-extract-core` | 0.1.0 | Core library (workspace crate) |
| `video-audio-decoder` | 0.1.0 | Video/audio decoding (workspace crate) |

### Plugins (All Workspace Crates)

**Core Plugins:**
- `transcription` - Whisper.cpp transcription
- `video-audio-extractor` - Audio extraction
- `video-audio-keyframe` - Keyframe extraction

**ONNX Plugins:**
- `video-audio-object-detection` - YOLOv8 object detection
- `video-audio-face-detection` - RetinaFace detection
- `video-audio-ocr` - PaddleOCR text recognition
- `video-audio-embeddings` - CLIP/CLAP/MiniLM embeddings
- `video-audio-diarization` - Speaker diarization
- `video-audio-scene` - Scene detection

**Tier 1 Plugins:**
- `video-audio-subtitle` - Subtitle extraction (SRT/ASS/VTT)
- `video-audio-classification` - Audio classification (YAMNet, 521 classes)
- `video-audio-smart-thumbnail` - Intelligent thumbnail selection
- `video-audio-action-recognition` - Action recognition (Kinetics-600)
- `video-audio-motion-tracking` - Object motion tracking

**Tier 2 Plugins:**
- `video-audio-pose-estimation` - Human pose estimation (YOLOv8-pose)
- `video-audio-image-quality` - Image quality assessment (NIMA)
- `video-audio-emotion-detection` - Facial emotion detection (ResNet18)
- `audio-enhancement-metadata` - Audio quality metadata
- `shot-classification` - Shot type classification

### Additional Dependencies

| Dependency | Version | Purpose | Features |
|------------|---------|---------|----------|
| `clap` | 4.5 | CLI argument parsing | derive, cargo |
| `rayon` | 1.10 | Data parallelism | - |

## Core Library (video-extract-core)

### Core Dependencies

| Dependency | Version | Purpose |
|------------|---------|---------|
| `async-trait` | 0.1 | Async trait support |
| `crossbeam-channel` | 0.5 | Concurrent channels |
| `serde_yaml` | 0.9 | YAML support (plugin configs) |
| `blake3` | 1.5 | Cryptographic hashing (cache keys) |
| `num_cpus` | 1.16 | CPU core detection |
| `mozjpeg` | 0.10 | Optimized JPEG encoding |

### Internal Dependencies

| Dependency | Purpose |
|------------|---------|
| `video-audio-common` | Shared types and utilities |
| `video-audio-decoder` | FFmpeg video/audio decoding |

## Plugin-Specific Dependencies

### Transcription (whisper-rs)

| Dependency | Version | Purpose | Features |
|------------|---------|---------|----------|
| `whisper-rs` | 0.15 | Whisper.cpp bindings | metal (macOS acceleration) |

**Model**: ggml-base.bin (147MB, 99 languages)

### Audio Classification (YAMNet)

| Dependency | Version | Purpose |
|------------|---------|---------|
| `ndarray` | 0.16 | Audio tensor operations |
| `serde_yaml` | 0.9 | Class map loading |

**Model**: yamnet.onnx (16MB, 521 audio event classes)

### Embeddings (CLIP/CLAP/MiniLM)

| Dependency | Version | Purpose | Features |
|------------|---------|---------|----------|
| `tokenizers` | 0.22 | Text tokenization | - |
| `once_cell` | 1.20 | Model caching | - |
| `fftw` | 0.8 | FFT library | system (use system FFTW) |
| `hound` | 3.5 | WAV file I/O | - |

**Models**:
- CLIP: clip_vit_b32.onnx (605MB)
- CLAP: clap.onnx (114MB)
- MiniLM: all_minilm_l6_v2.onnx (90MB)

### Audio Enhancement Metadata

| Dependency | Version | Purpose |
|------------|---------|---------|
| `rustfft` | 6.2 | Pure Rust FFT implementation |

**Features**:
- Spectral analysis (frequency content, spectral centroid)
- Dynamic range analysis
- Noise floor estimation

### Diarization (Speaker Separation)

| Dependency | Version | Purpose |
|------------|---------|---------|
| `ndarray` | 0.16 | Embedding operations |

**Model**: speaker_embedding.onnx (26MB, WeSpeaker CAMPPlus)

### Object Detection (YOLOv8)

| Dependency | Version | Purpose |
|------------|---------|---------|
| `image` | 0.25 | Image preprocessing |
| `ndarray` | 0.16 | Tensor operations |

**Model**: yolov8n.onnx (12MB, 80 COCO classes)

### Face Detection (RetinaFace)

| Dependency | Version | Purpose |
|------------|---------|---------|
| `image` | 0.25 | Image preprocessing |
| `ndarray` | 0.16 | Tensor operations |

**Model**: retinaface_mnet025.onnx (1.2MB)

### OCR (PaddleOCR)

| Dependency | Version | Purpose |
|------------|---------|---------|
| `image` | 0.25 | Image preprocessing |
| `ndarray` | 0.16 | Tensor operations |

**Models**:
- Detection: ch_PP-OCRv4_det.onnx (2.4MB)
- Recognition: ch_PP-OCRv4_rec.onnx (10.6MB)
- Dictionary: ppocr_keys_v1.txt (26KB, 6623 characters)

### Pose Estimation (YOLOv8-Pose)

| Dependency | Version | Purpose |
|------------|---------|---------|
| `image` | 0.25 | Image preprocessing |
| `ndarray` | 0.16 | Tensor operations |

**Model**: yolov8n-pose.onnx (13MB, 17 keypoints)

### Image Quality Assessment (NIMA)

| Dependency | Version | Purpose |
|------------|---------|---------|
| `image` | 0.25 | Image preprocessing |
| `ndarray` | 0.16 | Tensor operations |

**Model**: nima_mobilenetv2.onnx (8.8MB)

### Emotion Detection (ResNet18)

| Dependency | Version | Purpose |
|------------|---------|---------|
| `image` | 0.25 | Image preprocessing |
| `ndarray` | 0.16 | Tensor operations |
| `base64` | 0.22 | Base64 encoding |

**Model**: emotion_resnet18.onnx (44MB, 7 emotions)

### Subtitle Extraction

| Dependency | Version | Purpose |
|------------|---------|---------|
| `ffmpeg-next` | 8.0 | Extract subtitle streams |

**Formats**: SRT, ASS, VTT

### Scene Detection

| Dependency | Version | Purpose |
|------------|---------|---------|
| `video-audio-decoder` | 0.1.0 | Keyframe access |

**Algorithm**: Keyframe-only optimization (45-100x speedup)

## Web API (api-server)

### Web Framework

| Dependency | Version | Purpose | Features |
|------------|---------|---------|----------|
| `axum` | 0.8 | Web framework | - |
| `tower` | 0.5 | Middleware | - |
| `tower-http` | 0.6 | HTTP middleware | cors, trace |

## Dev Dependencies (Testing)

| Dependency | Version | Purpose |
|------------|---------|---------|
| `tokio-test` | 0.4 | Tokio testing utilities |
| `tempfile` | 3.8 | Temporary files |
| `shellexpand` | 3.1 | Shell expansion (~, $HOME) |
| `sysinfo` | 0.30 | System information |
| `chrono` | 0.4 | Date/time handling |
| `csv` | 1.3 | CSV generation |
| `sha2` | 0.10 | SHA-256 hashing |
| `hostname` | 0.4 | Hostname detection |

## Build Profile Settings

### Release Profile

```toml
[profile.release]
opt-level = 3           # Maximum optimization
lto = "fat"             # Link-time optimization
codegen-units = 1       # Single codegen unit for better optimization
strip = true            # Strip debug symbols
```

### Bench Profile

```toml
[profile.bench]
inherits = "release"    # Same as release
```

### Flamegraph Profile

```toml
[profile.flamegraph]
inherits = "release"
debug = true            # Keep debug info for profiling
strip = false           # Don't strip symbols
```

## Dependency Summary by Category

| Category | Count | Total Size (Models) |
|----------|-------|---------------------|
| ML Inference (ONNX Runtime) | 1 | - |
| ML Models | 16 | ~1.2 GB |
| Video/Audio Processing | 3 | - |
| Async Runtime | 2 | - |
| Serialization | 4 | - |
| Error Handling | 2 | - |
| Storage | 3 | - |
| Logging | 2 | - |
| CLI | 2 | - |
| Testing | 8 | - |
| Math/DSP | 4 | - |
| Web | 3 | - |
| Internal Workspace Crates | 24 | - |

**Total Unique External Dependencies**: ~35
**Total Workspace Crates**: 24

## Model Storage

All ONNX models are stored in `models/` directory:

```
models/
├── audio-classification/    # YAMNet (16MB)
├── diarization/            # WeSpeaker (26MB)
├── embeddings/             # CLIP (605MB), CLAP (114MB), MiniLM (90MB)
├── emotion-detection/      # ResNet18 (44MB)
├── face-detection/         # RetinaFace (1.2MB)
├── image-quality/          # NIMA (8.8MB)
├── object-detection/       # YOLOv8 (12MB)
├── ocr/                    # PaddleOCR (13MB)
├── pose-estimation/        # YOLOv8-Pose (13MB)
└── whisper/                # Whisper base (147MB)
```

**Total Model Size**: ~1.2 GB

## Security

**Dependency Auditing**:
```bash
cargo audit
```

Run regularly to check for known vulnerabilities.

**FFTW Note**: Using `system` feature to avoid vulnerable build dependencies in bundled FFTW.

## Python Dependencies

**Status**: ZERO Python dependencies

Python is completely eliminated from the project. All ML inference uses ONNX Runtime with pre-exported models.

**Python scripts remaining**:
- `models/*/export_*.py` - One-time model export to ONNX format
- `scripts/export_models/*.py` - One-time model export utilities

These are only needed once to convert PyTorch models to ONNX format.

## C++ Dependencies

**FFmpeg** (via ffmpeg-next):
- Required for video/audio decoding
- System dependency (install via Homebrew/apt)

**Whisper.cpp** (via whisper-rs):
- Required for transcription
- Bundled with whisper-rs crate
- Uses Metal acceleration on macOS

## Platform-Specific Features

### macOS
- **Metal**: GPU acceleration for Whisper (whisper-rs)
- **CoreML**: GPU acceleration for ONNX models (ort coreml feature)
- **VideoToolbox**: Intentionally NOT used (5-10x slower than software decode)

### Linux
- **CUDA**: GPU acceleration for ONNX models (ort cuda feature)
- **System FFTW**: Used via fftw crate

### Cross-Platform
- **CPU fallback**: All features work on CPU-only systems
- **Multi-threading**: Rayon for data parallelism, ONNX Runtime for model parallelism

## Installation Requirements

### System Dependencies

**macOS**:
```bash
brew install ffmpeg
```

**Ubuntu/Debian**:
```bash
sudo apt-get install -y \
    libavcodec-dev \
    libavformat-dev \
    libavutil-dev \
    libavfilter-dev \
    libavdevice-dev \
    libswscale-dev \
    libswresample-dev \
    pkg-config \
    clang
```

### Building

```bash
# Clone repository
git clone https://github.com/dropbox/dKNOW/video_audio_extracts
cd video_audio_extracts

# Build release binary
cargo build --release

# Binary location
./target/release/video-extract
```

## Version Policy

- **Rust Edition**: 2021
- **MSRV**: Not specified (latest stable recommended)
- **Dependency Updates**: Workspace-level version pinning for consistency

## See Also

- **COMPREHENSIVE_AUDIT_N102.md** - Complete audit report
- **Cargo.toml** - Workspace configuration
- **crates/*/Cargo.toml** - Individual crate dependencies
