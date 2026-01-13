# Performance Benchmarks

**Date:** 2025-11-09 (N=159, updated from N=158)
**Status:** Phase 5.2 Complete - 25/32 Operations Benchmarked (78% coverage, emotion_detection blocked by face detection bug)
**Hardware:** Apple M2 Max, 64 GB RAM, macOS Darwin 24.6.0
**Binary:** video-extract v1.0.0 (release build)

---

## Executive Summary

Comprehensive performance benchmarks for the video-audio-extracts library covering 25 operations across all categories. Operations demonstrate latencies ranging from 50ms to 450ms on small files, with memory usage from 14 MB to 309 MB depending on operation complexity and pipeline requirements.

**Key Findings:**
- **Low Latency:** Most operations complete in 53-260ms on test files (dominated by startup overhead on small files)
- **Memory Usage:** Peak memory ranges from 14-108 MB depending on operation complexity
- **High Throughput:** Audio operations achieve 7-8 MB/s throughput on larger files
- **Bulk Mode Scaling:** 2.1x speedup with 8 concurrent workers (see Concurrency Scaling)
- **ML Operations:** Diarization (350ms, 108 MB) and pose estimation (260ms, 89.5 MB) have higher resource requirements
- **Lightweight Operations:** Audio enhancement (140ms, 17.9 MB) and shot classification (130ms, 17.3 MB) are efficient

---

## Benchmark Methodology

**Test Configuration:**
- Environment variable: `VIDEO_EXTRACT_THREADS=4` (limits thread pool sizes)
- Execution mode: `performance` mode (optimized for speed, no debug overhead)
- Measurement tools: `/usr/bin/time -l` (macOS) for memory and wall-clock time
- Test files: Small (30-150 KB) test files from `test_edge_cases/` directory

**Note on Test File Sizes:**
The benchmark test files are intentionally small (30-150 KB) to enable fast testing. Latency measurements (53-86ms) are dominated by binary startup overhead (~50ms). For production workloads with larger files (1-100 MB), actual processing time will be proportionally higher and throughput metrics (MB/s) will be more representative.

---

## Operation Performance Matrix

### Core Extraction Operations (3/3 benchmarked)

| Operation | File Type | File Size | Latency | Peak Memory | Throughput | Notes |
|-----------|-----------|-----------|---------|-------------|------------|-------|
| **metadata_extraction** | video_medium | 0.15 MB | 86ms | 14.59 MB | 1.74 MB/s | FFprobe-based extraction |
| **keyframes** | video_medium | 0.15 MB | 59ms | 14.57 MB | 2.55 MB/s | I-frame extraction with dedup |
| **audio_extraction** | video_medium | 0.15 MB | 57ms | 14.67 MB | 2.61 MB/s | FFmpeg audio decode |

**Key Insights:**
- Metadata extraction has highest latency (86ms) due to FFprobe overhead
- Keyframes and audio extraction are highly optimized (57-59ms)
- Memory usage is consistent (~14.6 MB) across all operations

---

### Speech & Audio Operations (7/8 benchmarked)

| Operation | File Type | File Size | Latency | Peak Memory | Throughput | Notes |
|-----------|-----------|-----------|---------|-------------|------------|-------|
| **transcription** | audio_short (1s) | 0.09 MB | 57ms | 14.84 MB | 1.58 MB/s | Whisper.cpp (base model) |
| **voice_activity_detection** | audio_medium | 0.45 MB | 59ms | 14.57 MB | 7.62 MB/s | WebRTC VAD |
| **audio_classification** | audio_medium | 0.45 MB | 57ms | 14.60 MB | 7.85 MB/s | YAMNet (521 classes) |
| **diarization** | audio_medium | 0.46 MB | 350ms | 108.2 MB | 1.34 MB/s | WebRTC VAD + ONNX embeddings + K-means |
| **acoustic_scene_classification** | audio_medium | 0.46 MB | 190ms | 71.3 MB | 2.47 MB/s | YAMNet (indoor/outdoor, room size) |
| **audio_enhancement_metadata** | audio_medium | 0.46 MB | 140ms | 17.9 MB | 3.35 MB/s | Spectral analysis + recommendations |
| **profanity_detection** | audio_short (1s) | 0.09 MB | 450ms (pipeline) | 309 MB | N/A | Pipeline: transcription + profanity detection (<1ms overhead) |

**Key Insights:**
- Audio operations achieve highest throughput (7-8 MB/s) on larger files
- Transcription is slower (1.58 MB/s) due to ML inference complexity
- Voice activity detection and classification are highly efficient
- Diarization has higher latency (350ms) and memory (108 MB) due to speaker embedding extraction
- Acoustic scene classification uses CoreML acceleration (71.3 MB memory)
- Audio enhancement metadata is lightweight (17.9 MB) with pure signal processing
- Profanity detection adds <1ms overhead (measured 17.75µs) when chained with transcription
- All operations maintain reasonable memory footprint

**Not Benchmarked (1/8):**
- audio_embeddings (benchmarked separately, see Embeddings)

---

### Vision Analysis Operations (6/8 benchmarked)

| Operation | File Type | File Size | Latency | Peak Memory | Throughput | Notes |
|-----------|-----------|-----------|---------|-------------|------------|-------|
| **object_detection** | image_medium | 8.3 KB | 56ms | 14.56 MB | N/A | YOLOv8 ONNX (small file) |
| **face_detection** | image_medium | 8.3 KB | 65ms | 14.64 MB | N/A | RetinaFace ONNX |
| **ocr** | image_medium | 8.3 KB | 63ms | 14.59 MB | N/A | PaddleOCR ONNX |
| **scene_detection** | video_medium | 0.15 MB | 53ms | 14.65 MB | 2.80 MB/s | FFmpeg scdet filter |
| **pose_estimation** | image_medium | 8.3 KB | 260ms | 89.5 MB | N/A | YOLOv8-Pose ONNX (17 COCO keypoints) |
| **action_recognition** | video_30s | 7.7 MB | 1130ms (pipeline) | 268 MB | 6.81 MB/s | Motion analysis (pipeline: keyframes;action-recognition, action stage: 32ms) |

**Key Insights:**
- Most vision operations complete in 53-65ms on small files
- Scene detection is fastest (53ms) with FFmpeg's optimized scdet filter
- Pose estimation has higher latency (260ms) due to keypoint regression complexity
- Pose estimation uses more memory (89.5 MB) than other vision operations due to larger model
- Action recognition benchmarked as pipeline (keyframes;action-recognition): 1130ms total, 32ms for action analysis stage
- Action recognition memory (268 MB) dominated by keyframe extraction (29 frames)
- Latency dominated by model loading and startup overhead
- Throughput metrics not applicable for small image files (8.3 KB)

**Production Performance (from plugin metadata):**
- object_detection: 60s per GB, 512 MB memory
- face_detection: 45s per GB, 256 MB memory
- ocr: 90s per GB, 512 MB memory
- scene_detection: 10s per GB, 100 MB memory
- action_recognition: 20s per GB, 100 MB memory

**Not Benchmarked (2/8):**
- depth_estimation (requires user-provided MiDaS/DPT model)
- motion-tracking (requires ObjectDetection input + test media with detectable objects)

---

### Intelligence & Content Operations (4/8 benchmarked)

| Operation | File Type | File Size | Latency | Peak Memory | Throughput | Notes |
|-----------|-----------|-----------|---------|-------------|------------|-------|
| **image_quality_assessment** | image_medium | 8.3 KB | 55ms | 14.64 MB | N/A | NIMA ONNX model |
| **smart_thumbnail** | video_medium | 0.15 MB | 56ms | 14.71 MB | 2.69 MB/s | Quality heuristics |
| **shot_classification** | image_medium | 8.3 KB | 130ms | 17.3 MB | N/A | Camera shot type classification |
| **subtitle_extraction** | video_subtitle | 0.017 MB | 50ms | 24.8 MB | 0.34 MB/s | FFmpeg subtitle decode (mov_text codec) |

**Key Insights:**
- Image quality assessment (55ms) uses simple CNN inference
- Smart thumbnail achieves good throughput (2.69 MB/s)
- Shot classification is lightweight (130ms, 17.3 MB) using heuristics
- Subtitle extraction is very fast (50ms) with FFmpeg subtitle parser
- Memory usage ranges from 14.6-24.8 MB (subtitle extraction uses more memory for text buffers)

**Not Benchmarked (4/8):**
- emotion_detection (blocked by face detection bug - see N=159 investigation)
- content_moderation (requires user-provided model: nsfw_mobilenet.onnx)
- logo_detection (requires user-provided model: yolov8_logo.onnx)
- caption_generation (requires user-provided model: BLIP/BLIP-2/ViT-GPT2)

---

### Embeddings Operations (3/3 benchmarked)

| Operation | File Type | File Size | Latency | Peak Memory | Throughput | Notes |
|-----------|-----------|-----------|---------|-------------|------------|-------|
| **vision_embeddings** | image_medium | 8.3 KB | 66ms | 14.53 MB | N/A | CLIP vision encoder |
| **audio_embeddings** | audio_medium | 0.45 MB | 56ms | 15.15 MB | 8.00 MB/s | CLAP embeddings |
| **text_embeddings** | audio_short (1s) | 0.09 MB | 480ms (pipeline) | 432 MB | N/A | Pipeline: transcription + text-embeddings (~100ms overhead) |

**Key Insights:**
- Vision embeddings slightly slower (66ms) due to CLIP model complexity
- Audio embeddings achieve excellent throughput (8.00 MB/s)
- Text embeddings requires Transcription input (chained operation)
- Text embeddings adds ~100ms overhead on top of transcription (480ms total vs 410ms transcription alone)
- Text embeddings uses all-minilm-l6-v2 model (384-dimensional vectors)
- Memory usage for standalone embeddings is consistent (~14-15 MB), but text_embeddings pipeline requires 432 MB due to Whisper model

**Production Performance (from plugin metadata):**
- vision_embeddings: 30s per GB, 500 MB memory
- audio_embeddings: 60s per GB, 400 MB memory
- text_embeddings: 20s per GB, 300 MB memory

---

### Utility Operations (2/2 benchmarked)

| Operation | File Type | File Size | Latency | Peak Memory | Throughput | Notes |
|-----------|-----------|-----------|---------|-------------|------------|-------|
| **duplicate_detection** | video_small | 0.03 MB | 58ms | 14.51 MB | 0.51 MB/s | Perceptual hashing |
| **format_conversion** | video_small | 0.03 MB | 54ms | 14.84 MB | 0.55 MB/s | FFmpeg transcode |

**Key Insights:**
- Both operations complete in 54-58ms on tiny files
- Throughput is low (0.5 MB/s) due to very small file sizes
- Format conversion will scale with file size (expect ~3-5 MB/s on larger files)

---

## Concurrency and Scaling Performance

### Bulk Mode Throughput Scaling

**Test:** Keyframes extraction with varying file counts
**Configuration:** System default (8 concurrent workers)
**Source:** bulk_benchmark_results.txt (2025-10-30)

| File Count | Total Time | Throughput | Notes |
|------------|------------|------------|-------|
| 1 file | 0.12s | **8.46 files/sec** | Single file overhead |
| 5 files | 0.75s | **6.68 files/sec** | Slight overhead scaling |
| 8 files | 0.94s | **8.53 files/sec** | Optimal throughput |

**Insights:**
- Bulk mode achieves 8.5 files/sec throughput on 8 files
- Slight throughput reduction (6.7 files/sec) on 5 files due to incomplete parallelism
- Single file throughput is high (8.5 files/sec) but includes startup overhead

---

### Concurrency Scaling Efficiency

**Test:** Keyframes extraction with varying concurrency levels
**Files:** 8 test files
**Source:** bulk_benchmark_results.txt

| Concurrency | Time | Throughput | Speedup vs Sequential | Parallel Efficiency |
|-------------|------|------------|----------------------|---------------------|
| 1 (sequential) | 1.93s | 4.13 files/sec | 1.00x | 100% |
| 2 workers | 1.12s | 7.12 files/sec | 1.72x | **86.0%** |
| 4 workers | 1.00s | 8.00 files/sec | 1.93x | 48.2% |
| 8 workers | 0.92s | 8.71 files/sec | **2.10x** | 26.2% |
| 16 workers | 0.97s | 8.27 files/sec | 2.00x | 12.5% |

**Insights:**
- **Best parallel efficiency:** 2 workers (86% efficiency, 1.72x speedup)
- **Best absolute speedup:** 8 workers (2.10x speedup, 8.71 files/sec)
- **Diminishing returns:** Beyond 8 workers, performance plateaus due to overhead
- **Recommendation:** Use 4-8 workers for production workloads (optimal speedup/efficiency trade-off)

**Parallel Efficiency Formula:**
```
Parallel Efficiency = Speedup / Number of Workers
```

---

## Memory Usage Analysis

### Peak Memory Consistency

All benchmarked operations maintain remarkably consistent peak memory usage:

| Operation Category | Average Peak Memory | Range | Notes |
|-------------------|---------------------|-------|-------|
| Core Extraction | 14.61 MB | 14.57-14.67 MB | ±0.10 MB variance |
| Speech & Audio | 14.67 MB | 14.57-14.84 MB | ±0.27 MB variance |
| Vision Analysis | 14.60 MB | 14.56-14.64 MB | ±0.08 MB variance |
| Intelligence & Content | 14.67 MB | 14.64-14.71 MB | ±0.07 MB variance |
| Embeddings | 14.84 MB | 14.53-15.15 MB | ±0.62 MB variance |
| Utility | 14.68 MB | 14.51-14.84 MB | ±0.33 MB variance |

**Overall Average:** 14.65 MB ± 0.25 MB

**Insights:**
- Extremely consistent memory usage across all operations
- Low memory footprint suitable for embedded and constrained environments
- Variance within ±2% (14.4-15.2 MB range) indicates excellent resource management
- Memory usage is dominated by binary base overhead (~14 MB)

**Production Memory Usage (from plugin metadata):**
For larger files and complex operations, memory scales with file size:
- Light operations: 16-128 MB (metadata, VAD, quality assessment)
- Medium operations: 256-512 MB (keyframes, object detection, transcription)
- Heavy operations: 500-1024 MB (vision embeddings, music separation, transcription large model)

---

## Performance by File Type

### Video Files

**Operations Tested:** keyframes, audio_extraction, metadata_extraction, smart_thumbnail, scene_detection

| File Size | Best Throughput | Best Operation | Worst Throughput | Worst Operation |
|-----------|-----------------|----------------|------------------|-----------------|
| Tiny (30 KB) | 0.55 MB/s | format_conversion | 0.51 MB/s | duplicate_detection |
| Medium (150 KB) | 2.80 MB/s | scene_detection | 1.74 MB/s | metadata_extraction |

**Insights:**
- Scene detection achieves best throughput (2.80 MB/s) due to FFmpeg optimization
- Metadata extraction has overhead from FFprobe invocation
- Tiny files (<100 KB) have low throughput due to startup overhead dominance

---

### Audio Files

**Operations Tested:** transcription, voice_activity_detection, audio_classification, audio_embeddings

| File Size | Best Throughput | Best Operation | Worst Throughput | Worst Operation |
|-----------|-----------------|----------------|------------------|-----------------|
| Short (90 KB) | 1.58 MB/s | transcription | 1.58 MB/s | transcription (only test) |
| Medium (450 KB) | 8.00 MB/s | audio_embeddings | 7.62 MB/s | voice_activity_detection |

**Insights:**
- Audio operations achieve excellent throughput (7-8 MB/s) on larger files
- Transcription is slower (1.58 MB/s) due to Whisper model complexity
- File size significantly impacts throughput (8x improvement from 90 KB → 450 KB)

---

### Image Files

**Operations Tested:** object_detection, face_detection, ocr, image_quality_assessment, vision_embeddings

| File Size | Average Latency | Fastest Operation | Slowest Operation |
|-----------|-----------------|-------------------|-------------------|
| Small (8.3 KB) | 60ms | image_quality_assessment (55ms) | vision_embeddings (66ms) |

**Insights:**
- All image operations complete in 55-66ms (consistent performance)
- Throughput metrics not meaningful for 8.3 KB files (startup overhead dominates)
- Vision embeddings slightly slower (66ms) due to CLIP model complexity

---

## Startup Overhead Analysis

**Binary Base Overhead:** ~50-55ms

**Evidence:**
- Fastest operation: scene_detection at 53ms
- Most operations: 55-65ms range
- Only 11ms spread (53-66ms) across 16 diverse operations

**Conclusion:**
On small test files (30-450 KB), binary startup overhead dominates total latency. For production workloads with larger files (1-100 MB), processing time will dominate and throughput metrics will be more representative.

**Estimated Production Latency (1 MB file):**
```
Total Latency ≈ Startup Overhead (50ms) + Processing Time (file_size / throughput)

Example (keyframes on 10 MB video):
  Startup: 50ms
  Processing: 10 MB / 2.55 MB/s = 3,922ms
  Total: 3,972ms (≈4 seconds)
```

---

## Performance Comparison to Baselines

### Existing Benchmark Data

From PERFORMANCE_BENCHMARK_PLAN_N28.md and historical benchmarks:

| Operation | Historical Throughput | Current Benchmark | Change |
|-----------|----------------------|-------------------|--------|
| keyframes | 5.01 MB/s (large files) | 2.55 MB/s (small files) | N/A (different file sizes) |
| transcription | 7.56 MB/s (6.5x real-time) | 1.58 MB/s (1s audio) | N/A (small file, startup overhead) |
| scene_detection | 2200 MB/s (historical) | 2.80 MB/s (small file) | N/A (different measurement methodology) |

**Note:** Direct comparison is not meaningful due to different file sizes and test conditions. Historical benchmarks used larger files (1-100 MB) where processing time dominates, while N=57 benchmarks used small files (30-450 KB) where startup overhead dominates.

---

## Recommendations

### For Production Workloads

1. **Use bulk mode for ≥5 files:** Achieves 2.1x speedup with 8 workers
2. **Optimal concurrency:** 4-8 workers for best speedup/efficiency trade-off
3. **Performance mode:** Always use `performance` command (not `debug`) for production
4. **Thread limiting:** Only set `VIDEO_EXTRACT_THREADS` for testing; production should auto-detect
5. **Large files:** Throughput metrics become representative on files >1 MB

### For Latency-Sensitive Applications

1. **Single-file mode:** Use `performance` command for lowest overhead
2. **Avoid metadata_extraction:** Has highest latency (86ms) due to FFprobe overhead
3. **Prefer fast operations:** scene_detection (53ms), format_conversion (54ms), image_quality_assessment (55ms)

### For Memory-Constrained Environments

1. **All operations safe:** Peak memory usage is consistent at ~14-15 MB for single operations
2. **Bulk mode scaling:** Memory scales linearly with concurrency (expect 14 MB × workers)
3. **Large file operations:** Check plugin metadata for production memory requirements (256-1024 MB)

---

## Performance Comparison Charts

**Status:** ✅ COMPLETE (Phase 5.3, N=66)

Visual performance comparisons are now available:

**📊 Interactive HTML Dashboard:**
- `docs/charts/performance_charts.html` - View all 4 charts in one page

**📈 Individual SVG Charts:**
1. `docs/charts/throughput_comparison.svg` - Operations sorted by MB/s throughput
2. `docs/charts/latency_distribution.svg` - Operations sorted by milliseconds latency
3. `docs/charts/memory_usage.svg` - Memory by category with min/max ranges
4. `docs/charts/concurrency_scaling.svg` - Speedup and efficiency vs worker count

**View Charts:**
```bash
# Open interactive dashboard
open docs/charts/performance_charts.html

# Regenerate charts (Python 3.6+, no external libraries)
python3 scripts/generate_performance_charts.py
```

**Documentation:**
- `docs/CHART_GENERATION_GUIDE.md` - Complete chart generation methodology

---

## Unbenchmarked Operations (7/32)

**Current Status (N=161):** 25/32 operations benchmarked (78% coverage). Remaining 7 operations cannot be benchmarked due to external blockers.

**Note:** Total plugin count corrected from 33 to 32 (N=161). README.md confirms 32 total plugins.

The following operations have not been benchmarked:

**Operations Blocked by Bugs (2):**
- **emotion_detection**: **BLOCKER** - Face detection model/code mismatch prevents RetinaFace from detecting faces even when present in test images (tested N=158 with lena.jpg, biden.jpg). Error: "No faces detected" despite faces being clearly visible. Root cause investigated in N=159 (reports/main/N159_face_detection_extended_analysis.md): UltraFace postprocessing code expects "scores" and "boxes" outputs, but RetinaFace model provides different tensor names. Requires model replacement or postprocessing rewrite.
- **motion-tracking**: Requires ObjectDetection input. Test attempted (N=158) with keyframes;object-detection;motion-tracking pipeline on test_media_generated/test_av1_5s.mp4, but failed due to no detectable objects in synthetic test video. Error: "No detections found in input data". Requires test media with real-world objects (e.g., people, vehicles, animals).

**Operations Missing Required Models (5):**
- **content_moderation**: Missing nsfw_mobilenet.onnx model file.
- **logo_detection**: Missing yolov8_logo.onnx model file.
- **caption_generation**: Missing caption generation model file (e.g., BLIP, BLIP-2, ViT-GPT2).
- **music_source_separation**: Missing Demucs/Spleeter model file.
- **depth_estimation**: Missing MiDaS or DPT model file (midas_v3_small.onnx or dpt_hybrid.onnx). Plugin exists but requires user-provided ONNX model. CLI operation partially exposed (N=161 investigation) but non-functional without model.

**Plugin Metadata Available:** All 7 unbenchmarked operations have documented performance estimates in their plugin metadata (accessible via `video-extract plugins` command).

---

## Future Benchmarking Work

**Phase 5.2: Complete (N=161)** - 25/32 operations benchmarked (78% coverage)

**Status:** Performance benchmarking phase effectively complete. Remaining 7 operations cannot be benchmarked due to external blockers (2 bugs, 5 missing user-provided models). All benchmarkable operations have been measured.

**Attempted N=158-161:** Investigation confirmed all remaining operations are blocked:
- **emotion_detection**: Face detection model/code mismatch (N=159 root cause analysis)
- **motion-tracking**: Requires real-world test media with detectable objects
- **5 user-model operations**: Require user-provided ONNX models (content_moderation, logo_detection, caption_generation, music_source_separation, depth_estimation)

**Phase 5.3: Hardware Configuration Testing (Optional)**
- Benchmark on low-end (4 cores, 8 GB RAM)
- Benchmark on mid-range (8 cores, 16 GB RAM)
- Document scaling characteristics
- **Status:** Requires access to different hardware configurations

---

## Appendix: Benchmark Data

### Raw Benchmark Results

**File:** benchmarks/results/quick_benchmark_20251107_034718.json
**Date:** 2025-11-07 03:47:18 PST
**Hardware:** Apple M2 Max, 64 GB RAM, Darwin 24.6.0
**Operations:** 16 operations benchmarked

See `benchmarks/results/` directory for full JSON data.

### Test Files Used

| Name | Path | Size | Type |
|------|------|------|------|
| video_tiny | test_edge_cases/video_tiny_64x64_resolution__scaling_test.mp4 | 30 KB | Video |
| video_small | test_edge_cases/video_tiny_64x64_resolution__scaling_test.mp4 | 30 KB | Video |
| video_medium | test_edge_cases/video_hevc_h265_modern_codec__compatibility.mp4 | 150 KB | Video |
| video_large | test_edge_cases/video_4k_ultra_hd_3840x2160__stress_test.mp4 | 153 KB | Video |
| video_30s | test_media_generated/test_keyframes_100_30s.mp4 | 7.7 MB | Video (29 keyframes) |
| audio_short | test_edge_cases/audio_very_short_1sec__duration_min.wav | 90 KB | Audio |
| audio_medium | test_edge_cases/audio_mono_single_channel__channel_test.wav | 450 KB | Audio |
| image_medium | test_edge_cases/image_test_dog.jpg | 8.3 KB | Image |
| image_large | test_edge_cases/image_test_mandrill.png | 12 KB | Image |
| video_subtitle | test_edge_cases/video_with_subtitles__subtitle_test.mp4 | 17 KB | Video with subs |

---

## Benchmark Reproducibility

**To reproduce these benchmarks:**

```bash
# Run quick benchmark (16 operations, ~2 seconds)
./benchmarks/benchmark_quick.sh

# Run comprehensive benchmark (all 33 operations, ~10-20 minutes)
./benchmarks/benchmark_all_operations.sh

# Run single operation benchmark with hyperfine (10 runs for percentiles)
./benchmarks/benchmark_operation.sh <operation> <file1> [file2] ...
```

**Requirements:**
- hyperfine (`brew install hyperfine`)
- Release build (`cargo build --release`)
- Test files in `test_edge_cases/` directory

---

**End of Performance Benchmarks**
**Next:** Phase 5.2 - Hardware Configuration Testing (N=58-59)
