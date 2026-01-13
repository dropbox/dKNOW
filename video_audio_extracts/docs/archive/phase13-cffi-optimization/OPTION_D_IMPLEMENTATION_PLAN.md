# Option D Implementation Plan - Absolute Fastest for Everything

**Date**: 2025-10-30
**Decision**: User selected Option D (A+B combined)
**Goal**: Absolute fastest for ALL operations

---

## Overview

Option D combines:
- **Part A**: FFmpeg CLI delegation (simple ops match FFmpeg speed)
- **Part B**: Streaming decoder (ML ops 1.5x faster)
- **Part C**: Bulk optimizations (multi-file 3-5x faster)

**Timeline**: N=22-28 (7 commits, 16-24 hours AI time)

---

## Phase 1: FFmpeg CLI Delegation (N=22)

### Current Status
Worker has already implemented N=22 (streaming decoder). Need to ADD FFmpeg delegation.

### Implementation

**File**: `crates/video-extract-cli/src/commands/fast.rs`

**Add routing logic**:
```rust
impl FastCommand {
    fn execute_sync(self) -> Result<()> {
        let start = Instant::now();

        // Verify input exists
        if !self.input.exists() {
            antml:parameter::bail!("Input file does not exist: {}", self.input.display());
        }

        // ROUTER: Choose fastest implementation
        let result = match self.op.as_str() {
            // Simple operations: Delegate to FFmpeg CLI (absolute fastest)
            "keyframes" => self.execute_ffmpeg_keyframes(),
            "audio" => self.execute_ffmpeg_audio(),

            // Complex operations: Use our pipeline (we're fastest)
            "keyframes+detect" => self.extract_and_detect_zero_copy(),
            "transcription" => self.execute_transcription(),

            _ => anyhow::bail!("Unsupported operation: {}", self.op),
        };

        let total_elapsed = start.elapsed();
        println!("✓ Completed in {:.3}s", total_elapsed.as_secs_f64());

        result
    }

    /// Direct FFmpeg CLI call for keyframes (ABSOLUTE FASTEST - 0ms overhead)
    fn execute_ffmpeg_keyframes(&self) -> Result<()> {
        let output_pattern = self.output_dir.join("frame_%08d.jpg");

        let status = Command::new("ffmpeg")
            .args([
                "-hide_banner",
                "-loglevel", "panic",
                "-i", self.input.to_str().unwrap(),
                "-vf", "select='eq(pict_type\\,I)'",
                "-vsync", "vfr",
                "-q:v", "2",
                output_pattern.to_str().unwrap(),
            ])
            .status()
            .context("Failed to execute ffmpeg")?;

        if !status.success() {
            anyhow::bail!("FFmpeg keyframe extraction failed");
        }

        Ok(())
    }

    /// Direct FFmpeg CLI call for audio (ABSOLUTE FASTEST - 0ms overhead)
    fn execute_ffmpeg_audio(&self) -> Result<()> {
        let output_path = self.output_dir.join("audio.wav");

        let status = Command::new("ffmpeg")
            .args([
                "-hide_banner",
                "-loglevel", "panic",
                "-i", self.input.to_str().unwrap(),
                "-ar", &self.sample_rate.to_string(),
                "-ac", "1",
                "-y",
                output_path.to_str().unwrap(),
            ])
            .status()
            .context("Failed to execute ffmpeg")?;

        if !status.success() {
            anyhow::bail!("FFmpeg audio extraction failed");
        }

        Ok(())
    }
}
```

### Benchmark Validation

**Test**:
```bash
# Before (N=21)
time ./video-extract fast --op keyframes video.mp4
# Expected: 0.194s

# After (N=22 with delegation)
time ./video-extract fast --op keyframes video.mp4
# Expected: 0.149s (matches FFmpeg CLI exactly)
```

**Success criteria**:
- ✅ Time ≤ 0.155s (within 5% of FFmpeg CLI 0.149s)
- ✅ Output identical to FFmpeg CLI
- ✅ Complex ops still work (keyframes+detect)

---

## Phase 2: Streaming Decoder Integration (N=23-24)

### Current Status
Worker has implemented streaming decoder API at N=22 (already committed).

### Implementation

**N=23**: Integrate streaming decoder with parallel pipeline

**File**: `crates/video-extract-core/src/parallel_pipeline.rs`

**Changes**:
```rust
// OLD (N=21): Non-streaming
let frames = decode_iframes_zero_copy(video_path)?;
for frame in frames {
    sender.send(FrameMessage::Frame(frame))?;
}

// NEW (N=23): Streaming
decode_iframes_streaming(video_path, |frame| {
    sender.send(FrameMessage::Frame(frame))?;
})?;
```

**Benefits**:
- True streaming parallelism (decode + inference overlap)
- Lower memory (no Vec collection)
- Expected 1.5-2x speedup

### Benchmark Validation

**Test**:
```bash
# Before (N=21)
time ./video-extract fast --op keyframes+detect --parallel video.mp4
# Measured: 0.61s

# After (N=23 with streaming)
time ./video-extract fast --op keyframes+detect --parallel video.mp4
# Expected: 0.40s (1.5x faster)
```

**Success criteria**:
- ✅ Time ≤ 0.45s (at least 1.3x faster than 0.61s)
- ✅ Detections identical to sequential pipeline
- ✅ Memory usage lower or same

**N=24**: Benchmark with longer videos (20+ keyframes) to validate 1.5-2x claim

---

## Phase 3: Bulk Optimizations (N=25-28)

### N=25: Cleanup (N mod 5)
- Archive N=22-24 documentation
- Plan bulk optimizations

### N=26: Persistent ONNX Sessions

**Goal**: Eliminate model reload overhead

**Implementation**:
```rust
// crates/video-extract-core/src/executors/bulk.rs

use std::sync::Arc;
use ort::Session;

struct BulkExecutor {
    // Shared sessions (loaded once, reused across all files)
    yolo_session: Arc<Session>,
    clip_session: Arc<Session>,
    whisper_session: Arc<Session>,
    // ... other models
}

impl BulkExecutor {
    fn new() -> Result<Self> {
        // Load models ONCE
        let yolo = Arc::new(load_yolo_session()?);
        let clip = Arc::new(load_clip_session()?);
        // ...
        Ok(Self { yolo_session: yolo, clip_session: clip, ... })
    }

    fn process_files(&self, files: &[PathBuf]) -> Result<Vec<Output>> {
        files.par_iter().map(|file| {
            // Each worker clones Arc (cheap pointer copy)
            let yolo = Arc::clone(&self.yolo_session);
            // Process file with shared session
            self.process_file(file, yolo)
        }).collect()
    }
}
```

**Expected speedup**: 20-30% (eliminates model load overhead)

### N=27: File-Level Parallelism

**Goal**: Process multiple files concurrently

**Implementation**:
```rust
use rayon::prelude::*;

impl BulkExecutor {
    fn process_files_parallel(&self, files: &[PathBuf]) -> Result<Vec<Output>> {
        // Parallel iterator (rayon work stealing)
        files.par_iter()
            .map(|file| {
                // Each file processed by separate worker
                // FFmpeg decoder per worker (separate contexts)
                // ONNX session shared via Arc
                self.process_single_file(file)
            })
            .collect()
    }
}
```

**Threading strategy**:
- FFmpeg: Separate decoder context per worker (thread-safe per FFMPEG_ONNX_THREADING_GUIDE.md)
- ONNX: Shared session via Arc<Session> (concurrent Run() supported)
- Mutex: Only for FFmpeg init (avcodec_open2, avformat_open_input)

**Expected speedup**: 2-3x on quad-core (near-linear scaling)

### N=28: Memory Management + Validation

**Implementation**:
1. Bounded parallelism (limit to 4-8 concurrent files)
2. Memory monitoring (warn if >80% RAM)
3. Progress reporting (processed X/Y files)
4. Full test suite validation

**Expected speedup**: 3-5x combined (session sharing + parallelism)

---

## Timeline Summary

| Phase | Commits | Time | Speedup | Cumulative |
|-------|---------|------|---------|------------|
| **Phase 1: FFmpeg Delegation** | N=22 | 2-3h | Simple: 1.3x | 1.3x (simple) |
| **Phase 2: Streaming Decoder** | N=23-24 | 10-15h | ML: 1.5x | 2x (ML) |
| **Phase 3: Bulk Optimizations** | N=25-28 | 8-12h | Bulk: 3-5x | 3-5x (bulk) |
| **TOTAL** | N=22-28 | 20-30h | All: 1.3-5x | **Fastest for all** |

---

## Success Criteria (Must Meet All)

### Phase 1 (N=22)
- ✅ Simple keyframes: ≤ 0.155s (within 5% of FFmpeg CLI 0.149s)
- ✅ Simple audio: ≤ 0.09s (within 10% of FFmpeg CLI 0.08s)
- ✅ Complex ops still work (no regression)

### Phase 2 (N=23-24)
- ✅ ML pipeline: ≤ 0.45s (at least 1.3x faster than 0.61s)
- ✅ Detections correct (no accuracy loss)
- ✅ Memory usage lower or same

### Phase 3 (N=25-28)
- ✅ Bulk processing: ≥ 5 files/sec (at least 3x faster than 1-2 f/s)
- ✅ Memory usage: < 5GB for 100 files
- ✅ All tests passing (90/98 minimum)

### Overall
- ✅ **Absolute fastest for simple ops** (matches FFmpeg CLI)
- ✅ **Absolute fastest for ML ops** (1.5x faster than current)
- ✅ **Absolute fastest for bulk** (3-5x faster than current)

**Final state**: No solution faster for any operation type.

---

## Worker Instructions for N=22

### Current State
You just committed N=22 (streaming decoder). Good work.

### Next Task
**ADD FFmpeg CLI delegation** to N=22 work (or create N=23 for delegation).

### Implementation Steps

1. **Read this plan** thoroughly
2. **Modify fast.rs** to add routing logic (see code above)
3. **Add two functions**:
   - `execute_ffmpeg_keyframes()` - Direct FFmpeg call
   - `execute_ffmpeg_audio()` - Direct FFmpeg call
4. **Benchmark** both operations to verify 0ms overhead
5. **Commit** with honest measurements

### Testing
```bash
# Test simple ops (should match FFmpeg)
time ./video-extract fast --op keyframes test.mp4
# Expected: ~0.15s

time ./video-extract fast --op audio test.mp4
# Expected: ~0.08s

# Test complex ops (should still work)
time ./video-extract fast --op keyframes+detect test.mp4
# Expected: 0.40s with streaming (or 0.61s if streaming not integrated yet)
```

### Success Criteria
- Simple ops match FFmpeg CLI (±5%)
- Complex ops still functional (no regression)
- Clean build, 0 warnings

**Proceed with implementation.**
