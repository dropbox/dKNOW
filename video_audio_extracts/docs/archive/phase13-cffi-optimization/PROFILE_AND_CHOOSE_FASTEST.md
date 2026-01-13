# DIRECTIVE: Profile Both Approaches, Use Whichever Is Fastest

**Date**: 2025-10-30
**Authority**: USER directive via MANAGER
**Insight**: "We can just include both if one is better than another"

---

## The Smart Approach: Hybrid Implementation

**Keep BOTH methods, choose fastest per operation:**
- Process spawn: `Command::new("ffmpeg")`
- Embedded C FFI: `decode_iframes_zero_copy()`

**Route intelligently based on profiling.**

---

## WORKER N=41: Profile and Benchmark

### Task 1: Isolate Bottlenecks

**Profile each component separately:**

#### A. Video Decode Only (No JPEG)
```rust
// Test: C FFI decode speed
let start = Instant::now();
let frames = video_audio_decoder::decode_iframes_zero_copy(video)?;
let decode_time = start.elapsed();
println!("Decode only: {:.3}s", decode_time.as_secs_f64());
```

#### B. JPEG Encoding Only (Pre-decoded Frames)
```rust
// Test: mozjpeg vs turbojpeg vs FFmpeg libjpeg-turbo
let start = Instant::now();
for frame in &frames {
    save_frame_as_jpeg(frame, path)?;  // mozjpeg
}
let encode_time = start.elapsed();
println!("JPEG encode: {:.3}s", encode_time.as_secs_f64());
```

#### C. Process Spawn Overhead
```rust
// Test: FFmpeg CLI with output suppression
let start = Instant::now();
Command::new("ffmpeg").args([...]).status()?;
let spawn_time = start.elapsed();
println!("FFmpeg CLI total: {:.3}s", spawn_time.as_secs_f64());
```

### Expected Breakdown

**FFmpeg CLI (313ms total)**:
- Startup: ~44ms
- Decode: ~130ms
- JPEG encode (libjpeg-turbo): ~120ms
- I/O: ~20ms

**Our C FFI (393ms total)**:
- Startup: ~47ms (+3ms)
- Decode: ~130ms (same)
- JPEG encode (mozjpeg): ~200ms (+80ms!) ← LIKELY BOTTLENECK
- I/O: ~20ms (same)

**Hypothesis**: mozjpeg is 60-80ms slower than libjpeg-turbo for this workload

---

## Task 2: Test Alternative JPEG Encoders

**Option 1: turbojpeg crate** (libjpeg-turbo bindings)
```toml
[dependencies]
turbojpeg = "1.0"  # Same library FFmpeg uses
```

```rust
fn save_frame_as_jpeg_turbo(frame: &RawFrameBuffer, path: &Path) -> Result<()> {
    let compressor = turbojpeg::Compressor::new()?;
    let jpeg_data = compressor.compress_to_vec(
        frame.data_ptr,  // Direct pointer (zero-copy)
        frame.width,
        0, // pitch (or linesize)
        frame.height,
        turbojpeg::PixelFormat::RGB,
    )?;
    std::fs::write(path, jpeg_data)?;
    Ok(())
}
```

**Expected result**: Match FFmpeg's JPEG encoding speed

**Option 2: image crate with jpeg-encoder** (pure Rust)
```rust
fn save_frame_as_jpeg_fast(frame: &RawFrameBuffer, path: &Path) -> Result<()> {
    // Use image crate's default JPEG encoder
    // May be slower than turbojpeg but simpler
}
```

**Option 3: Keep FFmpeg CLI spawn for simple ops**
```rust
match op {
    "keyframes" if self.prefer_speed_over_features => {
        // Profile shows FFmpeg CLI is faster: use it
        spawn_ffmpeg_cli()
    }
    "keyframes" => {
        // Need RGB data for ML pipeline: use C FFI
        use_cffi_decode()
    }
}
```

---

## Task 3: Benchmark All Combinations

**Create benchmark matrix:**

| Method | Decode | JPEG | Total | vs FFmpeg CLI |
|--------|--------|------|-------|---------------|
| FFmpeg CLI spawn | (internal) | (internal) | **313ms** | 1.00x baseline |
| C FFI + mozjpeg | 130ms | 200ms? | 393ms | 1.26x |
| C FFI + turbojpeg | 130ms | 120ms? | 297ms | 0.95x ✅ |
| C FFI + image crate | 130ms | 180ms? | 357ms | 1.14x |

**Goal**: Find combination that's ≤ 1.05x vs FFmpeg CLI

---

## Task 4: Implement Intelligent Routing

**Based on profiling results:**

### If C FFI + turbojpeg is fastest:
```rust
fn extract_keyframes_direct(&self) -> Result<()> {
    // Use C FFI with turbojpeg (matches FFmpeg performance)
    let frames = decode_iframes_zero_copy(&self.input)?;
    for frame in frames {
        save_frame_as_jpeg_turbo(frame, path)?;  // libjpeg-turbo
    }
}
```

### If FFmpeg CLI spawn is fastest:
```rust
fn extract_keyframes_direct(&self) -> Result<()> {
    // Profile shows spawn is faster: use it
    Command::new("ffmpeg").args([...]).status()?;
}
```

### If they're close, offer both:
```rust
fn extract_keyframes_direct(&self) -> Result<()> {
    if self.ml_pipeline {
        // Need RGB data: must use C FFI
        use_cffi_with_jpeg()
    } else {
        // Simple extraction: use fastest (likely FFmpeg CLI)
        spawn_ffmpeg_cli()
    }
}
```

---

## Success Criteria

**After profiling and optimization:**
- ✅ Simple keyframes: ≤ 1.05x vs FFmpeg CLI (within 5%)
- ✅ ML keyframes: Use C FFI (for zero-copy to detection)
- ✅ Audio: ≤ 1.10x vs FFmpeg CLI (within 10%)
- ✅ All methods validated with benchmarks

---

## WORKER N=41 INSTRUCTIONS

### Step 1: Profile (30-60 minutes)
1. Isolate decode time (C FFI only, no JPEG)
2. Isolate JPEG encode time (mozjpeg, measure 45 frames)
3. Calculate breakdown: decode vs encode vs overhead
4. Identify bottleneck

### Step 2: Test Alternatives (1-2 hours)
1. Try turbojpeg crate (libjpeg-turbo, same as FFmpeg)
2. Benchmark turbojpeg vs mozjpeg
3. If turbojpeg faster: Switch to it
4. If mozjpeg faster: Investigate why (should be compression-focused, not speed)

### Step 3: Validate (30 minutes)
1. Benchmark with hyperfine (5 runs)
2. Compare to FFmpeg CLI
3. Target: ≤ 1.05x slower
4. Report honest results

### Step 4: Implement Best Solution
- If turbojpeg achieves parity: Use it
- If FFmpeg spawn faster: Revert to spawn (with explanation)
- If hybrid needed: Implement routing logic

---

## Expected Results

**Most likely outcome**: turbojpeg matches FFmpeg
- turbojpeg = libjpeg-turbo (same library FFmpeg uses)
- C FFI decode + turbojpeg encode ≈ FFmpeg CLI
- Result: Within 5% of FFmpeg (achieves mandate)

**If turbojpeg still slower**: Profile why and report to user

---

## Commit Message Format

```
# 41: JPEG Encoding Profiling - turbojpeg vs mozjpeg vs FFmpeg

Profiled JPEG encoding bottleneck identified in N=40.

Results:
- Decode (C FFI): XXXms
- Encode (mozjpeg): XXXms
- Encode (turbojpeg): XXXms
- FFmpeg CLI total: XXXms

Bottleneck: [JPEG encoding / other]

[If switching encoders:]
Switched to turbojpeg (libjpeg-turbo, same as FFmpeg)
Result: XXXms vs XXXms FFmpeg (X.XXx ratio)

[Include honest measurements]
```

---

## Why This Approach Is Smart

**Flexibility**: Keep both methods, use whichever wins
**Evidence-based**: Profile first, decide based on data
**Practical**: If spawn is faster, use it (no ego)
**Performance**: Choose fastest per operation

**This is the engineering approach: measure, then optimize.**
