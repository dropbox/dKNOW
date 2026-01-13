# Changelog

## [0.1.0] - 2025-11-03

### Alpha Release

High-performance media processing system for AI search and agent workflows. This alpha release represents 437 development iterations (build-video-audio-extracts branch N=0-436) with comprehensive format and plugin support.

### Key Features

**Format Support (39 formats)**:
- Video (12): MP4, MOV, AVI, MKV, WEBM, ASF, VOB, RM, DV, MXF, GXF, F4V
- Audio (11): WAV, MP3, FLAC, M4A, AAC, WMA, DTS, AC3, ALAC, TTA, AMR
- Image (14): JPG, PNG, WEBP, BMP, ICO, HEIC, HEIF, AVIF, SVG, RAW (NEF, CR2, ARW, RAF, DNG)
- Other (2): PDF, HLS

**Processing Plugins (32 total, 27 active)**:
- Video Analysis: Keyframe extraction, scene detection, shot classification, object detection, face detection, NSFW detection, OCR
- Audio Processing: Transcription (Whisper), voice activity detection, acoustic scene classification, audio embeddings
- ML Features: Vision embeddings (CLIP), text embeddings (MiniLM), smart thumbnails
- Utilities: Format conversion, audio extraction, subtitle extraction, metadata extraction

**Performance**:
- Multi-threaded processing (Rayon thread pool)
- CoreML GPU acceleration for ML inference (1.35x speedup)
- Optimized FFmpeg integration (C FFI, minimal process spawns)
- Zero-copy ONNX Runtime integration
- Aggressive LTO compilation optimizations
- 40-70% throughput gains from optimization work

**Test Coverage**:
- 485 automated Rust tests (363 comprehensive smoke + 116 standard + 6 legacy smoke)
- 100% test pass rate (485/485 all tests: 363/363 comprehensive smoke, 116/116 standard, 6/6 legacy)
- 3,526 test files across 39 formats (working tree, >10MB files removed from git N=432)
- 282 verified format×plugin combinations
- Comprehensive edge case testing (30 edge case files)

**API Support**:
1. Single file API: Minimize latency for individual file processing
2. Bulk file API: Maximize throughput for batch processing
3. Debug API: Detailed processing information and diagnostics

### Technical Highlights

**Architecture**:
- Rust primary language (performance, safety)
- C++ for FFmpeg and OpenCV bindings
- ONNX Runtime for cross-platform ML inference
- Pre-exported ONNX models (no Python runtime dependency)

**Dependencies**:
- FFmpeg (libavcodec, libavformat, libswresample) for media processing
- OpenCV for computer vision operations
- ONNX Runtime with CoreML provider for macOS GPU acceleration
- Whisper.cpp for speech recognition
- Various ONNX models for ML tasks

### Known Limitations

**Alpha Release Constraints**:
- macOS only (Darwin target)
- Requires FFmpeg, OpenCV, ONNX Runtime installed
- Some plugins marked experimental
- Documentation focused on internal development

### Development Stats

- 30 AI worker iterations (main branch, N=0-30 completed at time of N=31 CHANGELOG update)
- 3,526 test files (working tree, >10MB files removed from git N=432)
- 38/39 formats at 5+ test files (97.4% coverage)
- Clean codebase (0 clippy warnings)
- Comprehensive documentation (2,500+ line technical spec)

### Next Steps

Post-alpha development priorities:
- Cross-platform support (Linux, Windows)
- Performance benchmarking and optimization
- Plugin stability improvements
- User documentation expansion
- Installation automation

---

**Copyright**: Andrew Yates 2025
**Target**: Dropbox internal use only
