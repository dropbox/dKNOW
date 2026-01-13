# MANAGER DIRECTIVE: Implement BOTH Streaming + Bulk Optimization

**Date**: 2025-10-30
**Authority**: User mandate
**Priority**: CRITICAL - Core API requirements

## User Mandate

> "we would want both streaming and bulk! these are likely ideal for different APIs. DO IT."

## Goals Alignment

From CLAUDE.md:
```
Three APIs:
1. single media file: minimize latency to process    ← STREAMING DECODER
2. bulk media files: maximize efficiency per core    ← BULK OPTIMIZATIONS
3. debug: provide details to users about what is happening
```

**Current Status:**
- ✅ Debug API: Working (N=1-9)
- ⚠️ Fast API: Parallel pipeline implemented but needs streaming decoder (N=21, 3-4% speedup)
- ❌ Bulk API: Sequential processing, no optimizations (N=92, identical to debug mode)

## Implementation Plan

### Phase 1: Streaming Decoder for Fast API (N=22-24, ~3-4 commits)

**Goal**: Enable true parallel decode+inference for fast mode

**Architecture Change:**
```rust
// CURRENT (non-streaming)
pub fn decode_iframes_zero_copy(video_path: &Path) -> Result<Vec<RawFrameBuffer>>

// NEW (streaming)
pub fn decode_iframes_streaming(
    video_path: &Path,
    sender: Sender<RawFrameBuffer>
) -> Result<()>
```

**Implementation Steps:**

1. **N=22: Streaming decoder API**
   - File: `crates/video-decoder/src/c_ffi.rs`
   - Add `decode_iframes_streaming()` function
   - Send frames as they're decoded (no Vec collection)
   - Keep existing `decode_iframes_zero_copy()` for backward compatibility

2. **N=23: Integrate streaming decoder with parallel pipeline**
   - File: `crates/video-extract-core/src/parallel_pipeline.rs`
   - Update `decode_thread_worker()` to use streaming API
   - Remove decode thread (FFmpeg decoder thread becomes producer)
   - Simplify architecture: FFmpeg → channel → inference thread

3. **N=24: Benchmark and validate**
   - Test with 20+ keyframe videos
   - Validate 1.5-2x speedup claim
   - If <1.3x speedup, report why and get user decision

**Expected Outcome:** 1.5-2x speedup for fast mode on videos with 20+ keyframes

---

### Phase 2: Bulk API Optimizations (N=25-28, ~4-5 commits)

**Goal**: Maximize throughput for processing multiple files

**Current Bulk Mode Issues:**
- Sequential file processing (no parallelism between files)
- Model reloading for each file (ONNX session not reused)
- No memory pooling (allocates for each file)
- No progress reporting

**Optimizations:**

1. **N=25: Cleanup + Planning** (N mod 5)
   - Archive Phase 1 docs
   - Plan bulk API architecture

2. **N=26: Persistent model sessions**
   - File: `crates/video-extract-core/src/executors/bulk.rs`
   - Load ONNX models once, reuse across all files
   - Expected speedup: 20-30% (eliminates model load overhead)

3. **N=27: Memory pooling**
   - Reuse frame buffers across files
   - Pre-allocate batch inference buffers
   - Expected speedup: 10-15% (reduces allocation overhead)

4. **N=28: Parallel file processing**
   - Process multiple files concurrently (thread pool)
   - Use rayon or tokio for work stealing
   - Expected speedup: 2-3x on multi-core systems

**Expected Outcome:** 3-5x speedup for bulk mode (combined optimizations)

---

## Architecture Decision Points

### Question 1: FFmpeg Thread Safety

**Issue**: Does `libavcodec` support multiple concurrent decoder instances?

**Investigation needed:**
- Check FFmpeg documentation on thread safety
- Test concurrent decode of different files
- If NOT thread-safe: Use mutex or sequential decode with parallel inference

**User instruction:** "If we need to improve the underlying library (e.g., add threading) TELL ME"

→ **TELL USER if FFmpeg doesn't support concurrent decoding**

### Question 2: Bulk Mode Parallelism Strategy

**Options:**

A. **File-level parallelism** (rayon par_iter)
   - Simple: `files.par_iter().map(|f| process(f))`
   - Good for independent file processing
   - Each file gets full CPU resources

B. **Pipeline parallelism** (producer-consumer)
   - Decode thread pool → inference thread pool
   - More complex but better resource utilization
   - Useful if decode is bottleneck

C. **Hybrid** (file-level + within-file pipeline)
   - Parallel files + parallel pipeline per file
   - Maximum throughput but high complexity

**Recommendation:** Start with A (file-level), measure bottlenecks, upgrade to C if needed

### Question 3: Memory Constraints

**Concern**: Processing 100 files in parallel could consume 10-20GB RAM

**Solutions:**
- Limit concurrent file count (e.g., 4-8 files)
- Implement backpressure (bounded queue)
- Monitor memory usage, warn user if >80% RAM

---

## Success Criteria

### Fast API (Streaming Decoder)
- ✅ 1.3x+ speedup on videos with 10+ keyframes
- ✅ 1.5x+ speedup on videos with 20+ keyframes
- ✅ Lower memory usage than current (no Vec collection)
- ✅ Backward compatible with existing fast_path.rs

### Bulk API (Optimizations)
- ✅ 2x+ speedup for processing 10+ files
- ✅ 3x+ speedup for processing 50+ files
- ✅ Model sessions reused (measured: no reload overhead)
- ✅ Memory usage <5GB for 100 files (with backpressure)

---

## Timeline Estimate

**Phase 1 (Streaming)**: 3-4 commits, ~36-48 hours AI time
**Phase 2 (Bulk)**: 4-5 commits, ~48-60 hours AI time
**Total**: 7-9 commits, ~84-108 hours AI time (~1 week AI work)

---

## Worker Instructions

**Immediate Next Steps (N=22):**

1. Read this directive thoroughly
2. Investigate FFmpeg thread safety for concurrent decoding
3. If thread-safe: Proceed with streaming decoder implementation
4. If NOT thread-safe: STOP and report to user for guidance
5. Implement streaming decoder API in video-decoder crate
6. Write tests for streaming decoder
7. Commit as N=22

**Do NOT:**
- Skip FFmpeg thread safety investigation
- Assume streaming decoder will work without testing
- Implement bulk optimizations before streaming decoder is done

**Report to User If:**
- FFmpeg doesn't support concurrent decoding
- Streaming decoder shows <1.3x speedup after implementation
- Memory usage exceeds expected bounds
- Any architectural blocker discovered

---

## Context for Future Workers

This directive supersedes NEXT_PHASE_OPTIONS_N20.md. The user has chosen BOTH:
- **Option A (continued)**: Streaming decoder for fast API
- **Option D**: Bulk API improvements

These are not alternatives - they are complementary optimizations for different APIs.
