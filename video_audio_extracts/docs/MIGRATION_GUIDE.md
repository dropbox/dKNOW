# Migration Guide: Getting Started with v1.0.0

**Target Audience:** New users adopting video-audio-extracts for the first time

**Status:** Production-ready (v1.0.0)

**Note:** This is the first production release. This guide is for new users getting started, not migrating from a previous version.

---

## Overview

The video-audio-extracts library is a high-performance media processing system designed for AI search and agent workflows. This guide will help you:
1. Set up your environment
2. Build and test the system
3. Understand the available operations
4. Run your first extraction
5. Integrate into your application

---

## Prerequisites

### System Requirements

**Minimum:**
- OS: macOS 12+, Linux (Ubuntu 22.04+), Windows 10+
- CPU: 4 cores (8+ recommended for bulk processing)
- RAM: 8 GB (16 GB+ recommended)
- Disk: 10 GB free space (for models and temporary files)

**Recommended:**
- OS: macOS 13+ or Ubuntu 22.04 LTS
- CPU: 8+ cores (Apple Silicon M1/M2 or Intel Xeon)
- RAM: 32 GB+
- GPU: Optional (CoreML on macOS, CUDA on NVIDIA GPUs)

### Software Dependencies

**All platforms:**
- Rust 1.70+ (https://rustup.rs)
- FFmpeg 5+ (with libav* development libraries)
- pkg-config

**macOS:**
- Homebrew (https://brew.sh)

**Linux:**
- GCC/Clang compiler toolchain
- FFTW3 development libraries

**Windows:**
- MSVC or MinGW compiler
- vcpkg for dependency management

---

## Step 1: Environment Setup

### macOS Setup

```bash
# Install Homebrew (if not already installed)
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Install system dependencies
brew install ffmpeg pkg-config fftw

# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Verify installations
ffmpeg -version
cargo --version
pkg-config --version
```

### Linux Setup (Ubuntu/Debian)

```bash
# Update package list
sudo apt-get update

# Install build essentials
sudo apt-get install -y build-essential pkg-config clang llvm

# Install FFmpeg and development libraries
sudo apt-get install -y \
  ffmpeg \
  libavcodec-dev \
  libavformat-dev \
  libavutil-dev \
  libavfilter-dev \
  libswscale-dev \
  libswresample-dev \
  libfftw3-dev

# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Verify installations
ffmpeg -version
cargo --version
pkg-config --version
```

### Windows Setup

```powershell
# Install Rust toolchain
winget install rustup

# Install LLVM
winget install LLVM.LLVM

# Install vcpkg (for FFmpeg and FFTW)
git clone https://github.com/microsoft/vcpkg.git
cd vcpkg
.\bootstrap-vcpkg.bat

# Install dependencies
.\vcpkg install ffmpeg:x64-windows fftw3:x64-windows

# Integrate vcpkg with Visual Studio
.\vcpkg integrate install

# Set environment variables (adjust paths as needed)
$env:FFMPEG_DIR = "C:\vcpkg\installed\x64-windows"
$env:PKG_CONFIG_PATH = "C:\vcpkg\installed\x64-windows\lib\pkgconfig"
```

---

## Step 2: Clone and Build

```bash
# Clone repository
git clone https://github.com/ayates_dbx/video_audio_extracts.git
cd video_audio_extracts

# Build release binary (optimized for performance)
cargo build --release

# Binary location: ./target/release/video-extract
# Size: ~32 MB
# Build time: 5-15 minutes (first build)
```

**Build options:**
```bash
# Debug build (faster compilation, slower runtime)
cargo build

# Release build with all optimizations (recommended)
cargo build --release

# Check for linting issues
cargo clippy --all-features --all-targets
```

---

## Step 3: Verify Installation

### Run Smoke Tests

```bash
# Run comprehensive smoke tests (403 tests, ~4 minutes)
VIDEO_EXTRACT_THREADS=4 cargo test --release --test smoke_test_comprehensive -- --ignored --test-threads=1

# Expected result: 291/294 passing (99.0% pass rate, 3 known test file issues)
# Runtime: ~180-200 seconds
```

**Note on VIDEO_EXTRACT_THREADS:**
- This environment variable limits thread pool sizes to prevent system overload during testing
- Only use during testing
- **Do not set in production** - auto-detect provides maximum performance
- See docs/reference/TEST_THREAD_LIMITING.md for details

### Run Standard Test Suite

```bash
# Run all tests (769 tests, ~15 minutes)
VIDEO_EXTRACT_THREADS=4 cargo test --release --all -- --ignored --test-threads=1

# Expected result: 647/647 passing (100% pass rate)
# All tests passing as of N=143
```

### Verify Binary

```bash
# List all available plugins
./target/release/video-extract plugins

# Expected: 32 plugins listed (27 active, 5 awaiting models)

# Run a simple test
./target/release/video-extract performance --op metadata test_edge_cases/video_tiny_64x64_resolution__scaling_test.mp4

# Expected: JSON output with video metadata
```

---

## Step 4: Understanding Available Operations

### Plugin Categories

**Core Extraction (3 plugins):**
- `audio-extraction` - Extract audio from video/audio files
- `keyframes` - Extract I-frames with deduplication
- `metadata-extraction` - Extract file metadata (format, duration, codec, etc.)

**Speech & Audio (8 plugins):**
- `transcription` - Speech-to-text (Whisper, 99 languages)
- `diarization` - Speaker diarization
- `audio-classification` - Classify 521 audio event types (YAMNet)
- `voice-activity-detection` - Detect speech segments (WebRTC VAD)
- `acoustic-scene-classification` - Classify acoustic environment
- `audio-enhancement-metadata` - Audio quality analysis
- `profanity-detection` - Detect profane language
- `music-source-separation` - Separate music stems (requires user model)

**Vision Analysis (8 plugins):**
- `scene-detection` - Scene boundary detection
- `object-detection` - Detect 80 object classes (YOLOv8)
- `face-detection` - Detect faces with landmarks (RetinaFace)
- `ocr` - Extract text from images (PaddleOCR)
- `action-recognition` - Recognize 400 action categories
- `pose-estimation` - Estimate human pose (17 keypoints)
- `motion-tracking` - Multi-object tracking (ByteTrack)
- `depth-estimation` - Monocular depth estimation (requires user model)

**Intelligence & Content (8 plugins):**
- `smart-thumbnail` - Select best frame for thumbnail
- `subtitle-extraction` - Extract embedded subtitles
- `shot-classification` - Classify camera shot types
- `emotion-detection` - Detect 7 emotions from faces
- `image-quality-assessment` - Assess image quality (NIMA)
- `content-moderation` - NSFW detection
- `logo-detection` - Brand logo detection (requires user model)
- `caption-generation` - Generate image captions (requires user model)

**Semantic Embeddings (3 plugins):**
- `vision-embeddings` - Image embeddings (CLIP)
- `text-embeddings` - Text embeddings (Sentence-Transformers)
- `audio-embeddings` - Audio embeddings (CLAP)

**Utility (2 plugins):**
- `format-conversion` - Convert media formats
- `duplicate-detection` - Perceptual hashing for duplicates

**Full details:**
```bash
./target/release/video-extract plugins
```

---

## Step 5: Your First Extraction

### Example 1: Extract Keyframes

```bash
# Extract keyframes from a video
./target/release/video-extract performance --op keyframes video.mp4

# Output: keyframes_<timestamp>.json
# Contains: List of keyframe timestamps and file paths
```

### Example 2: Transcribe Audio

```bash
# Transcribe speech to text
./target/release/video-extract performance --op transcription audio.wav

# Output: transcription_<timestamp>.json
# Contains: Transcribed text with timestamps and confidence scores
```

### Example 3: Detect Objects in Video

```bash
# Extract keyframes and detect objects (zero-copy pipeline)
./target/release/video-extract fast --op keyframes+detect video.mp4

# Output: object_detection_<timestamp>.json
# Contains: Detected objects with bounding boxes and confidence scores
# Performance: 2.26x faster than separate operations
```

### Example 4: Complex Pipeline

```bash
# Sequential pipeline: Extract audio, then transcribe, then classify
./target/release/video-extract performance --op "audio-extraction;transcription;audio-classification" video.mp4

# Parallel pipeline: Extract audio and keyframes simultaneously
./target/release/video-extract performance --op "[audio-extraction,keyframes]" video.mp4

# Mixed pipeline: Extract keyframes, then detect objects and faces in parallel
./target/release/video-extract performance --op "keyframes;[object-detection,face-detection]" video.mp4
```

### Example 5: Bulk Processing

```bash
# Process multiple files in parallel (8 workers)
./target/release/video-extract bulk --op keyframes *.mp4 --max-concurrent 8

# Output: One JSON file per input file
# Performance: 2.1x speedup vs sequential processing
```

---

## Step 6: Choosing Execution Modes

### Debug Mode

**Use when:**
- Developing or debugging
- Need verbose logging
- Want intermediate file outputs
- Need to inspect pipeline stages

**Command:**
```bash
video-extract debug --op <operation> <input_file>
```

**Characteristics:**
- Saves intermediate outputs to `debug_output/<timestamp>/`
- Verbose logging to stdout
- Slowest mode (~30-50% overhead vs performance mode)

### Performance Mode

**Use when:**
- Production workloads
- Single file processing
- Need maximum speed
- Want streaming JSON output

**Command:**
```bash
video-extract performance --op <operation> <input_file>
```

**Characteristics:**
- Streaming JSON output to stdout
- No intermediate files
- Fastest single-file mode
- Near-zero overhead (<5ms)

### Fast Mode

**Use when:**
- Need absolute maximum speed
- Using zero-copy pipelines (keyframes+detect)
- Single operation or fused operations

**Command:**
```bash
video-extract fast --op <operation> <input_file>
```

**Characteristics:**
- Zero-copy memory pipeline (keyframes+detect)
- No disk I/O for intermediate data
- 2.26x faster than performance mode (for keyframes+detect)
- Only supports specific operation combinations

### Bulk Mode

**Use when:**
- Processing 5+ files
- Have multi-core system (4+ cores)
- Want parallel processing
- Can accept startup overhead

**Command:**
```bash
video-extract bulk --op <operation> <file1> <file2> ... [--max-concurrent N]
```

**Characteristics:**
- File-level parallelism
- 2.1x speedup with 8 workers
- Best for 10+ files
- Optimal concurrency: 4-8 workers

**Recommendation matrix:**
```
Files   | Complexity | Recommended Mode
--------|------------|------------------
1       | Simple     | fast
1       | Complex    | performance
2-4     | Any        | performance
5-20    | Any        | bulk (4 workers)
20+     | Any        | bulk (8 workers)
```

---

## Step 7: Integration Patterns

### Command-Line Integration

```bash
#!/bin/bash
# Process all videos in a directory

for video in videos/*.mp4; do
  echo "Processing $video..."
  ./target/release/video-extract performance \
    --op "keyframes;object-detection" \
    "$video" > "${video%.mp4}_results.json"
done
```

### Rust Library Integration

```rust
use video_extract_core::{ExecutorType, create_executor};
use video_extract_core::common::types::OutputSpec;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create executor
    let executor = create_executor(ExecutorType::Performance)?;

    // Define pipeline
    let spec = OutputSpec::parse("[keyframes,audio-extraction]")?;

    // Execute
    let result = executor.execute("video.mp4", &spec).await?;

    println!("Results: {:?}", result);
    Ok(())
}
```

### Python Integration (via subprocess)

```python
import subprocess
import json

def extract_keyframes(video_path):
    """Extract keyframes using video-extract."""
    cmd = [
        "./target/release/video-extract",
        "performance",
        "--op", "keyframes",
        video_path
    ]

    result = subprocess.run(cmd, capture_output=True, text=True)

    if result.returncode != 0:
        raise RuntimeError(f"Extraction failed: {result.stderr}")

    return json.loads(result.stdout)

# Usage
keyframes = extract_keyframes("video.mp4")
print(f"Extracted {len(keyframes)} keyframes")
```

---

## Step 8: Configuration and Tuning

### Thread Pool Configuration

**Testing only:**
```bash
# Limit thread pool sizes (prevents system overload during tests)
export VIDEO_EXTRACT_THREADS=4
cargo test --release
```

**Production:**
```bash
# DO NOT set VIDEO_EXTRACT_THREADS in production
# Auto-detect provides maximum performance
unset VIDEO_EXTRACT_THREADS
./target/release/video-extract performance --op keyframes video.mp4
```

### Model Configuration

**Default models** (automatically downloaded):
- Whisper base model (Whisper.cpp)
- YOLOv8n object detection (12MB)
- RetinaFace face detection
- PaddleOCR (DBNet + CRNN)
- WeSpeaker diarization
- CLIP vision embeddings (577MB)
- Sentence-Transformers text embeddings
- CLAP audio embeddings
- YAMNet audio classification
- X3D action recognition

**User-provided models** (6 plugins require custom models):
- music-source-separation (Demucs/Spleeter ONNX)
- depth-estimation (MiDaS/DPT ONNX)
- logo-detection (custom YOLOv8 ONNX)
- caption-generation (BLIP/BLIP-2/ViT-GPT2/LLaVA ONNX)

See docs/MODELS.md for model setup instructions.

### Performance Tuning

**For latency-sensitive applications:**
- Use `performance` or `fast` mode
- Avoid `metadata-extraction` (highest latency: 86ms)
- Prefer fast operations: scene-detection (53ms), format-conversion (54ms)

**For throughput-sensitive applications:**
- Use `bulk` mode with 4-8 workers
- Process batches of 10+ files for best amortization
- Use cache-friendly operations (transcription, keyframes)

**For memory-constrained environments:**
- Limit bulk mode concurrency (--max-concurrent 2)
- Use lightweight operations first
- Single-operation memory: ~14-15 MB
- Bulk mode memory: ~14 MB × workers

See docs/PERFORMANCE_BENCHMARKS.md for detailed performance data.

---

## Step 9: Troubleshooting

### Build Issues

**FFmpeg not found:**
```bash
# macOS
brew install ffmpeg pkg-config

# Linux
sudo apt-get install libavcodec-dev libavformat-dev libavutil-dev

# Verify
pkg-config --modversion libavcodec
```

**Linker errors:**
```bash
# Ensure pkg-config is in PATH
which pkg-config

# Check FFmpeg libraries are visible
pkg-config --libs libavcodec libavformat libavutil
```

**Rust version too old:**
```bash
# Update Rust toolchain
rustup update stable
rustup default stable
```

### Runtime Issues

**Binary not found:**
```bash
# Build release binary first
cargo build --release

# Binary location
ls -lh ./target/release/video-extract
```

**Models not found:**
```bash
# Models are downloaded on first use
# Check models directory
ls -lh models/

# If missing, re-run with debug logging
RUST_LOG=debug ./target/release/video-extract performance --op transcription audio.wav
```

**Tests failing:**
```bash
# Ensure test files are present
ls -lh test_edge_cases/

# Run with thread limiting
VIDEO_EXTRACT_THREADS=4 cargo test --release --test smoke_test_comprehensive -- --ignored --test-threads=1

# Check for specific failures
cargo test --release -- --nocapture 2>&1 | grep FAILED
```

**Permission denied:**
```bash
# Ensure binary is executable
chmod +x ./target/release/video-extract

# Check file permissions
ls -l ./target/release/video-extract
```

### Performance Issues

**Slow processing:**
- Verify using `--release` build (debug builds are 10x+ slower)
- Check thread pool size (don't set VIDEO_EXTRACT_THREADS in production)
- Use appropriate execution mode (fast/performance/bulk)
- Check hardware acceleration is enabled (CoreML on macOS)

**High memory usage:**
- Limit bulk mode concurrency (--max-concurrent 4)
- Use streaming output (performance mode)
- Avoid debug mode in production

**Getting help:**
- Check docs/README.md for documentation index
- Review docs/PERFORMANCE_BENCHMARKS.md for performance data
- Check GitHub Issues for known problems
- Contact repository maintainers

---

## Step 10: Next Steps

**Production deployment:**
1. Run full test suite on target platform
2. Benchmark critical operations for your workload
3. Configure appropriate execution modes
4. Set up monitoring and logging
5. Integrate with your application

**Advanced features:**
1. Custom model integration (see docs/MODELS.md)
2. Storage integration (S3/MinIO, Qdrant, PostgreSQL)
3. Cross-modal fusion for unified timelines
4. Custom plugin development

**Stay updated:**
- Watch repository for releases
- Review RELEASE_NOTES_v1.0.0.md for latest features
- Check docs/ for new documentation

---

## Summary

**You've learned:**
- ✅ How to set up your environment
- ✅ How to build and test the system
- ✅ What operations are available
- ✅ How to run basic extractions
- ✅ How to choose execution modes
- ✅ How to integrate into your application
- ✅ How to troubleshoot common issues

**You're ready to:**
- Extract keyframes, transcribe audio, detect objects
- Process batches of files with bulk mode
- Build complex pipelines with parallel execution
- Integrate into your production workflows

**Key takeaways:**
- 32 plugins covering media processing needs
- 3 execution modes (debug, performance, bulk)
- 100% test pass rate (647/647 tests, production-ready)
- Sub-100ms latency for most operations
- 2.1x bulk mode scaling
- 44 formats supported (12 video, 11 audio, 19 image including RAW, 2 document)

**Welcome to video-audio-extracts v1.0.0!**

---

**End of Migration Guide**
