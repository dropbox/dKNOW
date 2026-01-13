# MANDATE: FFmpeg CLI Delegation - Match FFmpeg Speed for Simple Operations

**Date**: 2025-10-30
**Authority**: USER MANDATE via MANAGER
**Status**: ⚠️ CANNOT BE ACHIEVED WITH CURRENT ARCHITECTURE (N=35, N=36)

---

## USER REQUIREMENT

> "FFmpeg CLI comparison: make it so that our solution is always at least as fast as the simple CLI solution"

**Current Reality (N=36):** We are 1.31-1.84x SLOWER than FFmpeg CLI for simple operations

**Architecture Decision Required:** See PERFORMANCE_GAP_ANALYSIS_N36_2025-10-30.md

---

## INVESTIGATION RESULTS (N=35, N=36)

### N=35: .status() Optimization
**Change**: Replaced `Command::new("ffmpeg").output()` with `.status()`
**Expected**: Eliminate stdout/stderr capture overhead (~15ms)
**Result**: Overhead persists (~50ms)
**Conclusion**: Capture overhead was NOT the root cause

### N=36: Root Cause Analysis
**Measured overhead breakdown** (see PERFORMANCE_GAP_ANALYSIS_N36_2025-10-30.md):
- Binary loading + initialization: ~25-30ms (47-56%)
- Clap argument parsing: ~10-15ms (19-28%)
- File validation + directory creation: ~10-15ms (19-28%)
- Post-processing (frame counting): ~10-20ms (19-37%)
- Other (printing, cleanup): ~1-3ms (2-6%)

**Total unavoidable overhead**: 50-54ms with current architecture

### Current Performance Gap (N=36 Benchmarks)

**Keyframes extraction:**
```
FFmpeg CLI:     174.7ms ± 11.5ms (baseline)
Our fast mode:  228.3ms ± 22.1ms
Gap:            53.6ms (1.31x slower, 30.7% overhead)
```

**Audio extraction:**
```
FFmpeg CLI:     58.6ms ± 4.0ms (baseline)
Our fast mode:  107.6ms ± 4.2ms
Gap:            49.0ms (1.84x slower, 83.6% overhead)
```

**Conclusion**: User mandate cannot be met with single-shot binary execution model

---

## THE FIX: Direct FFmpeg CLI Delegation

### Current Code Analysis

**File**: `crates/video-extract-cli/src/commands/fast.rs`
**Line 122**: `fn extract_keyframes_direct()`

**Current implementation:**
```rust
fn extract_keyframes_direct(&self) -> Result<()> {
    let output_pattern = self.output_dir.join("frame_%08d.jpg");

    let output = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel", "panic",
            "-i", self.input.to_str().unwrap(),
            "-vf", "select='eq(pict_type\\,I)'",
            "-vsync", "vfr",
            "-q:v", "2",
            output_pattern.to_str().unwrap(),
        ])
        .output()  // ← WRONG: Uses .output() which captures stdout/stderr
        .context("Failed to execute ffmpeg")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("FFmpeg failed: {}", stderr);
    }

    Ok(())
}
```

**Problem:** `.output()` captures stdout/stderr into memory, adds overhead.

**Fix:** Use `.status()` or `.spawn().wait()` for zero capture overhead.

---

## IMPLEMENTATION

### Change 1: Use .status() Instead of .output()

**File**: `crates/video-extract-cli/src/commands/fast.rs`
**Line 122**: `extract_keyframes_direct()`

**Replace:**
```rust
fn extract_keyframes_direct(&self) -> Result<()> {
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
        .status()  // ← FIXED: No capture overhead
        .context("Failed to execute ffmpeg")?;

    if !status.success() {
        anyhow::bail!("FFmpeg keyframe extraction failed with exit code: {:?}", status.code());
    }

    println!("Extracted keyframes to {}", self.output_dir.display());
    Ok(())
}
```

### Change 2: Same Fix for extract_audio_direct()

**File**: `crates/video-extract-cli/src/commands/fast.rs`
**Line 149**: `extract_audio_direct()`

**Apply same pattern:**
```rust
fn extract_audio_direct(&self) -> Result<PathBuf> {
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
        .status()  // ← FIXED: No capture overhead
        .context("Failed to execute ffmpeg")?;

    if !status.success() {
        anyhow::bail!("FFmpeg audio extraction failed with exit code: {:?}", status.code());
    }

    println!("Extracted audio to {}", output_path.display());
    Ok(output_path)
}
```

---

## EXPECTED RESULT

**After fix:**
```
FFmpeg CLI:     0.179s (baseline)
Our fast mode:  0.185s (1.03x, within 3%)
Overhead:       6ms (acceptable)
```

**Acceptable range:** 0.175s - 0.190s (within 5% of FFmpeg CLI)

---

## VALIDATION REQUIREMENTS

### Step 1: Benchmark Simple Keyframes

```bash
# FFmpeg CLI baseline
hyperfine --warmup 1 --runs 5 \
  'ffmpeg -hide_banner -loglevel panic -i test_edge_cases/video_4k_ultra_hd_3840x2160__stress_test.mp4 -vf "select='"'"'eq(pict_type\\,I)'"'"'" -vsync vfr /tmp/ffmpeg_%d.jpg'

# Our fast mode
hyperfine --warmup 1 --runs 5 \
  './target/release/video-extract fast --op keyframes test_edge_cases/video_4k_ultra_hd_3840x2160__stress_test.mp4'
```

**Success criteria:** Our time ≤ 1.05x FFmpeg CLI time (within 5%)

### Step 2: Benchmark Simple Audio

```bash
# FFmpeg CLI baseline
hyperfine --warmup 1 --runs 5 \
  'ffmpeg -hide_banner -loglevel panic -i test.mp4 -ar 16000 -ac 1 /tmp/audio.wav'

# Our fast mode
hyperfine --warmup 1 --runs 5 \
  './target/release/video-extract fast --op audio test.mp4'
```

**Success criteria:** Our time ≤ 1.05x FFmpeg CLI time (within 5%)

### Step 3: Verify Complex Ops Still Work

```bash
# Should NOT delegate (uses our pipeline)
./target/release/video-extract fast --op keyframes+detect test.mp4
./target/release/video-extract fast --op transcription test.mp4
```

**Success criteria:** Both complete successfully, no regression

---

## COMMIT MESSAGE FORMAT

```
# 34: FFmpeg CLI Delegation - Match FFmpeg Speed for Simple Operations

Implemented Option D Phase 1: Direct FFmpeg CLI calls for simple operations.

Benchmark Results:
- Keyframes: XXXs (FFmpeg: XXXs, gap: X.XXx)
- Audio: XXXs (FFmpeg: XXXs, gap: X.XXx)

[Include actual measurements with hyperfine]

Changes:
- fast.rs:122: extract_keyframes_direct() uses .status() not .output()
- fast.rs:149: extract_audio_direct() uses .status() not .output()

Result: Simple operations now within 5% of FFmpeg CLI speed.
```

---

## THIS IS NON-NEGOTIABLE

**User mandate:** "always at least as fast as simple CLI solution"

**Current status:** 1.30x slower (VIOLATES MANDATE)

**Fix effort:** 1-2 hours (2 functions, ~20 lines changed)

**Worker N=34: Implement this immediately. Do NOT proceed with other work until mandate is met.**

---

## IF STILL >1.05x SLOWER AFTER FIX

**Report to user immediately:**
```
After implementing .status() optimization:
- FFmpeg CLI: XXXs
- Our fast mode: XXXs
- Gap: X.XXx (YY% overhead)

Remaining overhead breakdown:
- Binary loading: ~25ms
- Clap parsing: ~15ms
- File validation: ~10ms
- Total: ~50ms unavoidable

Cannot achieve parity without:
A. Daemon mode (keep process running)
B. Shell wrapper (bypass Rust entirely)
C. Accept 50ms overhead (~5% for >1s operations)

Requesting user decision on next steps.
```

---

## ARCHITECTURE OPTIONS TO ACHIEVE MANDATE

### Option A: Daemon Mode (RECOMMENDED)
- Keep process running, accept commands via socket
- Eliminates binary loading (25ms), Clap parsing (15ms), validation (10ms)
- **Expected result**: 5-10ms overhead (ACHIEVES MANDATE)
- **Effort**: 2-3 AI commits
- See PERFORMANCE_GAP_ANALYSIS_N36_2025-10-30.md for detailed design

### Option B: Shell Wrapper
- Bypass Rust entirely for simple operations
- **Expected result**: 0-5ms overhead (ACHIEVES MANDATE)
- **Effort**: 0.5 AI commits
- **Trade-off**: Loses type safety, harder to maintain

### Option C: Stripped Binary
- Reduce binary size, optimize dependencies
- **Expected result**: 35-40ms overhead (DOES NOT ACHIEVE MANDATE)
- **Effort**: 1-2 AI commits
- **Conclusion**: Insufficient alone

### Option D: Accept Overhead
- Document as known limitation
- Focus on complex operations where overhead is <5%
- **Expected result**: No change (50ms overhead)
- **Effort**: 0 AI commits (update docs only)

---

## USER DECISION REQUIRED

**Question**: Which option should we pursue?

**Recommendation**: Option A (Daemon Mode) - Achieves mandate while maintaining Rust benefits

**Alternative**: Option D (Accept Overhead) - Zero cost, focus on high-value features

**See**: reports/build-video-audio-extracts/PERFORMANCE_GAP_ANALYSIS_N36_2025-10-30.md for complete analysis

---

## SUMMARY

**Current (N=36)**: 1.31-1.84x slower than FFmpeg CLI for simple ops
**Root cause**: Binary startup + Clap parsing + validation + post-processing = 50ms unavoidable overhead
**N=35 attempt**: .output() → .status() (did not eliminate overhead as expected)
**Options**: Daemon mode (achieves mandate) OR accept overhead (document limitation)
**Status**: Awaiting user decision on architecture direction
