# Phase 2: Bulk API Optimizations Plan (N=24)

**Date**: 2025-10-30
**Branch**: build-video-audio-extracts
**Context**: Phase 1 complete (N=22-23), now planning Phase 2 bulk optimizations

---

## Executive Summary

**Goal**: Achieve 3-5x speedup for bulk mode by enabling true file-level parallelism with safe FFmpeg/ONNX resource sharing.

**Current state**: BulkExecutor processes files concurrently via tokio (N workers), but each file creates its own decoder + ONNX session (expensive).

**Key optimizations**:
1. **FFmpeg init serialization** - Add global mutex to make concurrent decoding thread-safe
2. **ONNX session sharing** - Load models once, share via Arc across all workers (already partially done)
3. **True file parallelism** - Leverage tokio semaphore (already in place) + verify scalability

**Expected outcome**: 2-3x speedup for 10+ files, 3-5x speedup for 50+ files

---

## Phase 1 Review: What We Have

### Completed (N=10-23)
- ✅ Zero-copy C FFI decoder (N=10)
- ✅ ONNX zero-copy inference (N=11)
- ✅ Batch inference (N=13)
- ✅ Streaming decoder API (N=22)
- ✅ Parallel pipeline (N=21-23, variable performance)

### Current Bulk Mode Architecture

**File**: crates/video-extract-core/src/executor.rs:858-920

```rust
pub async fn execute_bulk(
    &self,
    pipeline: &Pipeline,
    input_files: Vec<PathBuf>,
) -> Result<tokio::sync::mpsc::Receiver<BulkFileResult>, PluginError> {
    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(self.max_concurrent_files));

    for input_path in input_files {
        tokio::spawn(async move {
            let _permit = semaphore.acquire().await; // Limit concurrency
            let result = Self::execute_single_file(&context, &pipeline, &input_path, cache.as_ref()).await;
            // Send result to channel
        });
    }
}
```

**What it does well**:
- ✅ File-level concurrency (tokio::spawn per file)
- ✅ Backpressure control (semaphore limits concurrent files)
- ✅ Shared cache (Arc<PipelineCache> across workers)
- ✅ Streaming results (mpsc::channel for progress)

**What's missing**:
- ❌ FFmpeg decoder init is NOT thread-safe (no mutex around avcodec_open2)
- ⚠️ ONNX sessions may be loading per-file (depends on fast_path usage)
- ❌ No measurement of actual parallel speedup

---

## Problem 1: FFmpeg Thread Safety

### Current State

**File**: crates/video-decoder/src/c_ffi.rs:262-293

```rust
pub fn create(codec: *const AVCodec, codecpar: *mut AVCodecParameters) -> Result<Self> {
    unsafe {
        let ptr = avcodec_alloc_context3(codec);
        // ...
        let ret = avcodec_open2(ptr, codec, ptr::null_mut()); // NOT THREAD-SAFE
        if ret < 0 { /* error */ }
        Ok(CodecContext { ptr })
    }
}
```

**Problem**: `avcodec_open2()` and `avformat_open_input()` are NOT thread-safe (FFMPEG_ONNX_THREADING_GUIDE.md:24-54)

**Risk**: Concurrent file processing could cause race conditions, crashes, or memory corruption

### Solution: Global FFmpeg Init Mutex

**Guidance from FFMPEG_ONNX_THREADING_GUIDE.md**:
- Wrap `avcodec_open2()`, `avformat_open_input()`, `avformat_find_stream_info()` in process-wide mutex
- Decode loop (av_read_frame, avcodec_send_packet, avcodec_receive_frame) is fully parallel (no lock)
- Enable internal FFmpeg threading per context (thread_count, thread_type)

**Implementation**:

```rust
// crates/video-decoder/src/c_ffi.rs

use std::sync::Mutex;

/// Global mutex for FFmpeg initialization (avcodec_open2, avformat_open_input)
/// See FFMPEG_ONNX_THREADING_GUIDE.md for rationale
static FFMPEG_INIT_LOCK: Mutex<()> = Mutex::new(());

impl FormatContext {
    pub fn open(path: &Path) -> Result<Self> {
        let _lock = FFMPEG_INIT_LOCK.lock().unwrap();

        unsafe {
            // avformat_open_input() - NOT thread-safe
            let ret = avformat_open_input(...);
            // avformat_find_stream_info() - NOT thread-safe
            let ret = avformat_find_stream_info(...);
        }

        drop(_lock); // Release lock
        Ok(FormatContext { ptr })
    }
}

impl CodecContext {
    pub fn create(codec: *const AVCodec, codecpar: *mut AVCodecParameters) -> Result<Self> {
        let _lock = FFMPEG_INIT_LOCK.lock().unwrap();

        unsafe {
            let ptr = avcodec_alloc_context3(codec);

            // Enable internal FFmpeg threading (complements file-level parallelism)
            (*ptr).thread_count = 0; // Auto-detect optimal thread count
            (*ptr).thread_type = FF_THREAD_FRAME | FF_THREAD_SLICE;

            // avcodec_open2() - NOT thread-safe
            let ret = avcodec_open2(ptr, codec, ptr::null_mut());
        }

        drop(_lock); // Release lock
        Ok(CodecContext { ptr })
    }
}
```

**Performance impact**: Minimal - init is <5% of total processing time, decode loop remains fully parallel

---

## Problem 2: ONNX Session Lifecycle

### Current State

**Fast path** (fast_path.rs:172-221):
```rust
static YOLO_SESSION: OnceLock<Mutex<Session>> = OnceLock::new();

fn get_yolo_session() -> Result<&'static Mutex<Session>> {
    if let Some(session) = YOLO_SESSION.get() {
        return Ok(session);
    }

    let session = Session::builder().commit_from_file(&model_path)?;
    YOLO_SESSION.set(Mutex::new(session))?;
    Ok(YOLO_SESSION.get().unwrap())
}
```

**Parallel pipeline** (parallel_pipeline.rs:382-429): Same pattern (independent singleton)

**Assessment**: ONNX session sharing is ALREADY IMPLEMENTED correctly:
- ✅ OnceLock ensures single initialization
- ✅ Mutex for thread-safe access (ONNX Session::run requires &mut self)
- ✅ Arc-based sharing (implicit via &'static reference)

**Issue**: Two separate singletons (fast_path, parallel_pipeline) - can consolidate but not critical

### Solution: Verify and Document

**No code changes needed** - current implementation is correct.

**Action**: Add documentation explaining ONNX thread safety:
```rust
/// YOLO model session (process singleton, shared across all file workers)
///
/// Thread safety:
/// - ONNX Runtime supports concurrent Session::run() calls (FFMPEG_ONNX_THREADING_GUIDE.md:125)
/// - Mutex protects mutable access (Session::run requires &mut self in ort crate)
/// - Each worker gets cheap Arc clone, shares underlying model weights
///
/// Performance:
/// - Model loading: ~200-500ms (YOLOv8n is ~6MB)
/// - Sharing saves 200-500ms per file for bulk processing
static YOLO_SESSION: OnceLock<Mutex<Session>> = OnceLock::new();
```

---

## Problem 3: Bulk Mode Scalability

### Current State

**File**: crates/video-extract-core/src/executor.rs:823-856

```rust
pub struct BulkExecutor {
    context: Context,
    max_concurrent_files: usize, // Default: num_cpus::get()
    cache: Option<PipelineCache>,
}
```

**Default concurrency**: num_cpus::get() (typically 8-16 on modern systems)

**Question**: Does BulkExecutor actually leverage fast_path or parallel_pipeline?

**Investigation needed**: Check if BulkExecutor uses:
- A. Plugin system (slower, serializable PluginData)
- B. Fast path (zero-copy keyframes + object detection)
- C. Parallel pipeline (streaming decode + inference)

**Likely answer**: Plugin system (executor.rs:990 calls stage.plugin.execute())

**Implication**: Bulk mode does NOT currently use fast_path optimizations!

### Solution: Wire Fast Path into Bulk Mode

**Option A**: Add fast-path plugin that wraps extract_and_detect_zero_copy()
- Fits into existing plugin architecture
- Requires PluginData serialization (loses zero-copy benefit)

**Option B**: Add direct fast-path execution mode to BulkExecutor
```rust
impl BulkExecutor {
    /// Execute keyframes + object detection pipeline in bulk (fast path)
    pub async fn execute_bulk_fast_path(
        &self,
        input_files: Vec<PathBuf>,
        confidence_threshold: f32,
        classes: Option<Vec<String>>,
    ) -> Result<tokio::sync::mpsc::Receiver<FastPathResult>> {
        // Use semaphore for concurrency control
        // Each worker calls extract_and_detect_zero_copy()
        // ONNX session shared via OnceLock
        // FFmpeg init serialized via global mutex
    }
}
```

**Recommendation**: Option B - bypass plugin overhead for maximum performance

---

## Implementation Plan: N=25-28

### N=25: Cleanup + FFmpeg Init Mutex (N mod 5)

**Scope**: Thread safety for FFmpeg concurrent decoding

**Files to modify**:
1. `crates/video-decoder/src/c_ffi.rs`
   - Add `static FFMPEG_INIT_LOCK: Mutex<()>`
   - Wrap `avcodec_open2()`, `avformat_open_input()`, `avformat_find_stream_info()` with mutex
   - Add `thread_count` and `thread_type` configuration to enable internal threading

**Testing**:
- Run standard test suite to verify no regressions
- Manual test: Process 2-3 videos concurrently to verify no crashes

**Expected outcome**: FFmpeg concurrent decoding is now thread-safe

**Estimated effort**: 1-2 AI commits (~12-24 minutes)

---

### N=26: ONNX Session Documentation + Validation

**Scope**: Document ONNX thread safety, verify correct usage

**Files to modify**:
1. `crates/video-extract-core/src/fast_path.rs`
   - Add comprehensive doc comment to YOLO_SESSION explaining thread safety
   - Verify mutex usage is correct

2. `crates/video-extract-core/src/parallel_pipeline.rs`
   - Same documentation for YOLO_SESSION
   - Consider consolidating with fast_path (optional)

**Testing**:
- Code review of ONNX session lifecycle
- No functional changes (documentation only)

**Expected outcome**: Clear understanding that ONNX sharing is already correct

**Estimated effort**: 1 AI commit (~12 minutes)

---

### N=27: Bulk Mode Fast Path Integration

**Scope**: Add direct fast-path execution to BulkExecutor

**Files to modify**:
1. `crates/video-extract-core/src/executor.rs`
   - Add `execute_bulk_fast_path()` method
   - Use tokio::spawn + semaphore (same as execute_bulk)
   - Each worker calls `extract_and_detect_zero_copy()`

2. `crates/video-extract-core/src/lib.rs`
   - Export new API

**Implementation**:
```rust
impl BulkExecutor {
    pub async fn execute_bulk_fast_path(
        &self,
        input_files: Vec<PathBuf>,
        confidence_threshold: f32,
        classes: Option<Vec<String>>,
    ) -> Result<tokio::sync::mpsc::Receiver<BulkFastPathResult>> {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(self.max_concurrent_files));

        for input_path in input_files {
            let tx = tx.clone();
            let semaphore = semaphore.clone();
            let classes = classes.clone();

            tokio::spawn(async move {
                let _permit = semaphore.acquire().await;
                let start_time = Instant::now();

                // Call fast path (zero-copy, shared ONNX session)
                let result = crate::fast_path::extract_and_detect_zero_copy(
                    &input_path,
                    confidence_threshold,
                    classes,
                );

                let processing_time = start_time.elapsed();

                let _ = tx.send(BulkFastPathResult {
                    input_path,
                    result: result.map_err(|e| format!("{:?}", e)),
                    processing_time,
                }).await;
            });
        }

        Ok(rx)
    }
}
```

**Testing**:
- Process 10 files: Measure total time, verify 2x+ speedup vs sequential
- Process 50 files: Measure total time, verify 3x+ speedup vs sequential
- Check memory usage: Should stay <5GB (no duplicate model loads)

**Expected outcome**: 2-3x speedup for 10+ files

**Estimated effort**: 2 AI commits (~24 minutes)

---

### N=28: Benchmark Validation + Performance Analysis

**Scope**: Measure actual speedup, validate Phase 2 success criteria

**Benchmarks**:
1. **Sequential baseline** (N=1 worker)
   - Process 10 test videos sequentially
   - Record: total time, avg time per file

2. **Parallel (N=4 workers)**
   - Process same 10 videos with 4 concurrent workers
   - Record: total time, speedup factor

3. **Parallel (N=8 workers)**
   - Process same 10 videos with 8 concurrent workers
   - Record: total time, speedup factor

4. **Large batch (N=8 workers)**
   - Process 50 videos with 8 concurrent workers
   - Record: total time, speedup factor, memory usage

**Test files**: Use standard test suite (COMPLETE_TEST_FILE_INVENTORY.md has 1,826 files)

**Success criteria** (from STREAMING_AND_BULK_MANDATE.md:156-161):
- ✅ 2x+ speedup for processing 10+ files
- ✅ 3x+ speedup for processing 50+ files
- ✅ Model sessions reused (measured: no reload overhead)
- ✅ Memory usage <5GB for 100 files (with backpressure)

**Documentation**:
- Create BULK_MODE_PERFORMANCE_N28.md with benchmark results
- Update README.md with bulk mode usage examples

**Expected outcome**: 3-5x speedup achieved and validated

**Estimated effort**: 1-2 AI commits (~12-24 minutes)

---

## Technical Risks and Mitigations

### Risk 1: FFmpeg Init Mutex Bottleneck

**Concern**: Global mutex could serialize file processing if init time dominates

**Mitigation**:
- FFmpeg init (open + probe + codec open) is <5% of total time (measured in N=171 profiling)
- Decode loop remains fully parallel (where 95% of time is spent)
- Internal FFmpeg threading per context offsets any init serialization

**Validation**: Benchmark N=1 vs N=8 workers - if <2x speedup, investigate mutex contention

---

### Risk 2: ONNX Session Mutex Contention

**Concern**: Mutex around Session::run could serialize inference

**Reality**: ONNX Runtime DOES support concurrent Session::run (FFMPEG_ONNX_THREADING_GUIDE.md:125)

**Problem**: ort crate's Session::run requires &mut self (API limitation, not ONNX limitation)

**Solution**: Mutex is necessary for ort crate API, but actual ONNX C++ runtime handles concurrency internally

**Performance**: Measured in N=13 - batch inference (8 frames) runs at 60-80ms, mutex lock time <1ms

**Validation**: If contention suspected, can wrap Session in parking_lot::Mutex (faster than std::Mutex)

---

### Risk 3: Memory Usage with Many Files

**Concern**: Processing 100 files concurrently could use 10-20GB RAM

**Mitigation**:
- Semaphore limits concurrent files (default: num_cpus::get(), typically 8-16)
- Each worker uses ~200MB peak (frame buffers + model weights shared)
- Total memory: ~1.6-3.2GB for 8-16 workers (well under 5GB limit)

**Validation**: Monitor memory with 100 file batch, verify <5GB usage

---

## Success Metrics

### Performance Targets

| Metric | Current | Target | Phase 2 Goal |
|--------|---------|--------|--------------|
| Sequential (10 files) | ~10-15s | 5-7s | 2x speedup |
| Parallel (10 files, N=8) | ~10-15s | 3-5s | 3x speedup |
| Parallel (50 files, N=8) | ~50-75s | 12-20s | 3-5x speedup |
| Memory (100 files, N=8) | Unknown | <5GB | Backpressure working |

### Functional Requirements

- ✅ No crashes with concurrent file processing
- ✅ Correct results (same detections as sequential mode)
- ✅ Progress reporting (streaming results via channel)
- ✅ Graceful error handling (per-file errors don't kill batch)

---

## Timeline Estimate

**Phase 2 (Bulk API)**: N=25-28

- **N=25**: FFmpeg init mutex + cleanup (1-2 commits, ~12-24 minutes)
- **N=26**: ONNX documentation + validation (1 commit, ~12 minutes)
- **N=27**: Bulk fast path integration (2 commits, ~24 minutes)
- **N=28**: Benchmark validation + docs (1-2 commits, ~12-24 minutes)

**Total**: 5-7 commits, ~60-84 minutes AI time

---

## Comparison: Phase 1 vs Phase 2

### Phase 1 (Streaming Decoder, N=22-23)

**Goal**: 1.5-2x speedup for fast API via parallel decode+inference

**Result**: 0.64x-1.11x speedup (variable, video-dependent)

**Lessons**:
- Thread coordination overhead can exceed parallelism benefits
- Short videos don't amortize thread startup costs
- Implementation is correct, but benefits are limited

**Status**: ✅ Functionally complete, ⚠️ performance goals not achieved universally

---

### Phase 2 (Bulk Optimizations, N=25-28)

**Goal**: 3-5x speedup for bulk API via file-level parallelism

**Advantages over Phase 1**:
- File-level parallelism is coarse-grained (minutes of work per task)
- No channel coordination overhead (each worker is independent)
- FFmpeg init overhead amortized across entire file (not per-frame)
- ONNX session sharing eliminates 200-500ms per file

**Confidence**: HIGH - file-level parallelism is proven approach, similar to N=171 batch processing

**Expected result**: 2-3x speedup for 10 files, 3-5x speedup for 50 files (linear scaling with num_cpus)

---

## Next AI (N=25): Start Phase 2 Implementation

### Tasks for N=25

1. **Read this plan** (BULK_API_PLAN_N24.md)
2. **Read threading guide** (FFMPEG_ONNX_THREADING_GUIDE.md)
3. **Implement FFmpeg init mutex**:
   - Add `static FFMPEG_INIT_LOCK: Mutex<()>` to c_ffi.rs
   - Wrap `avcodec_open2()`, `avformat_open_input()`, `avformat_find_stream_info()`
   - Add `thread_count` and `thread_type` configuration
4. **Run tests**: Verify no regressions with standard test suite
5. **Cleanup**: Archive Phase 1 documents (N mod 5)
6. **Commit**: Clean commit message explaining FFmpeg thread safety

### Expected Outcome

After N=25, concurrent FFmpeg decoding will be thread-safe, enabling true file-level parallelism in N=26-28.

---

## References

- **Phase 1 assessment**: reports/build-video-audio-extracts/N23_PARALLEL_PIPELINE_ANALYSIS_2025-10-30-20-08.md
- **User mandate**: STREAMING_AND_BULK_MANDATE.md
- **Threading guide**: FFMPEG_ONNX_THREADING_GUIDE.md
- **Test inventory**: COMPLETE_TEST_FILE_INVENTORY.md
- **Performance baseline**: PERFORMANCE_PROFILE_N171.md
