# Complete Remaining Work - Then Profile and Optimize

**Date**: 2025-10-30
**Authority**: USER directive
**Order**: Medium priority → Low priority → Profiling/Optimization → Git hook

---

## EXECUTION PLAN

### Phase 1: Medium Priority (N=59-60, ~2-3 hours)

#### N=59: Full Test Suite Validation
```bash
# Run complete test suite to verify 100% pass rate
cargo test --release --test standard_test_suite -- --ignored --test-threads=1

# Expected: 98/98 tests passing (100%)
# If failures: Investigate and fix
# Document results in test_results/
```

**Success criteria:**
- 98/98 tests passing
- AAC test passes (<2.0s)
- No performance regressions
- Test results captured in test_results/

---

### Phase 2: Low Priority (N=60-62, ~4-6 hours)

#### N=60: Cleanup (N mod 5)
- Archive obsolete docs
- Clean up test directories
- Update README.md

#### N=61: Audio Extraction Plugin C FFI
**Eliminate remaining 4 process spawns:**

1. **audio-extractor/src/lib.rs:116** - ffprobe (check audio stream)
```rust
// CURRENT: Command::new("ffprobe") to check if audio stream exists
// REPLACE: Use avformat_find_stream_info() C FFI (already have it)

pub fn has_audio_stream_cffi(video_path: &Path) -> Result<bool> {
    unsafe {
        let format_ctx = FormatContext::open(video_path)?;
        let stream_idx = av_find_best_stream(
            format_ctx.ptr,
            AVMEDIA_TYPE_AUDIO,  // Looking for audio
            -1, -1, ptr::null_mut(), 0
        );
        Ok(stream_idx >= 0)
    }
}
```

2. **audio-extractor/src/lib.rs:140** - ffmpeg (extract audio)
```rust
// CURRENT: Command::new("ffmpeg") to extract audio + resample
// REPLACE: Use libavcodec audio decode + libswresample

pub fn extract_audio_cffi(
    input: &Path,
    output: &Path,
    sample_rate: u32,
    channels: u8,
) -> Result<()> {
    unsafe {
        // 1. Open format context
        let format_ctx = FormatContext::open(input)?;

        // 2. Find audio stream
        let (stream_idx, decoder) = format_ctx.find_audio_stream()?;

        // 3. Decode audio packets
        while let Some(audio_frame) = decode_next_audio_frame()? {
            // 4. Resample if needed (libswresample)
            let resampled = resample(audio_frame, sample_rate, channels)?;

            // 5. Write to WAV file
            write_wav_samples(output, resampled)?;
        }
    }
}
```

3. **keyframe-extractor/src/lib.rs:131** - Already has C FFI mode!
```rust
// FIX: Change default from spawn to C FFI (1 line)
pub use_ffmpeg_cli: bool,  // true → false
```

4. **scene-detector/src/lib.rs:195** - DEFER (complex libavfilter API)

**Estimated**: 3-4 hours for audio C FFI

#### N=62: More Test Media (Optional)
- Find 20-30 additional files from user's system
- Download sample datasets
- Generate more synthetic files
- **Estimated**: 2-3 hours

---

### Phase 3: Profiling and Optimization (N=63-67, ~6-10 hours)

#### N=63: Profile Current Performance

**Tools:**
```bash
# CPU profiling
cargo flamegraph --release --test smoke_test -- --ignored

# Memory profiling
cargo instruments --release -t Allocations --test smoke_test -- --ignored

# Detailed timing
cargo build --release && hyperfine --warmup 2 --runs 10 \
  './target/release/video-extract fast --op keyframes video.mp4'
```

**Profile all operations:**
1. Keyframes (fast mode vs debug mode)
2. Object detection (batch vs single)
3. Transcription (various file sizes)
4. Audio extraction
5. Bulk mode (various batch sizes)
6. Parallel pipeline

**Output**: Performance profile report with bottleneck identification

#### N=64: Optimization Opportunities

**Based on profiling, prioritize:**

**1. Startup overhead** (if >20ms found):
- Replace Clap with minimal parsing
- Lazy load plugins
- Strip binary

**2. JPEG encoding** (if bottleneck):
- Try libjpeg-turbo-sys crate
- Direct C API for zero-copy
- SIMD optimization

**3. Audio resampling** (if bottleneck):
- Optimize libswresample usage
- Consider SIMD audio processing

**4. ONNX inference** (if bottleneck):
- Tune thread pool settings
- Try TensorRT (NVIDIA) or CoreML optimizations
- Batch size tuning

**5. Memory allocations** (if excessive):
- Pool allocators
- Pre-allocated buffers
- Reduce copies

#### N=65-67: Implement Top Optimizations
- Choose 2-3 highest impact optimizations
- Implement, benchmark, validate
- Document performance improvements

**Expected gains**: 10-30% per optimization

---

### Phase 4: Git Commit Hook (N=68, ~30 minutes)

#### Configure Pre-Commit Hook

**File**: `.git/hooks/pre-commit`

```bash
#!/bin/bash
# Pre-commit hook: Run smoke tests + check code quality

set -e

echo "🔍 Pre-commit validation..."
echo ""

# 1. Run smoke tests
echo "🧪 Running smoke tests (6 tests, ~3s)..."
cargo test --release --test smoke_test -- --ignored --test-threads=1 --quiet

if [ $? -ne 0 ]; then
    echo "❌ Smoke tests failed. Fix tests or use --no-verify"
    exit 1
fi
echo "✅ Smoke tests passed (6/6)"
echo ""

# 2. Check clippy
echo "🔍 Running clippy..."
cargo clippy --release -- -D warnings 2>&1 | grep -E "warning|error" && {
    echo "❌ Clippy warnings found. Fix or use --no-verify"
    exit 1
}
echo "✅ No clippy warnings"
echo ""

# 3. Check formatting
echo "📝 Checking formatting..."
cargo fmt -- --check || {
    echo "❌ Code not formatted. Run: cargo fmt"
    exit 1
}
echo "✅ Code formatted"
echo ""

echo "✅ All pre-commit checks passed"
exit 0
```

**Make executable:**
```bash
chmod +x .git/hooks/pre-commit
```

**Test:**
```bash
# Try committing - should run smoke tests
git commit -m "test: verify hook works"

# Skip if needed
git commit --no-verify -m "wip: incomplete"
```

---

## EXECUTION TIMELINE

| Phase | Tasks | Time | Result |
|-------|-------|------|--------|
| **Phase 1** | Full test suite, AAC validation | 2-3h | 100% tests passing |
| **Phase 2** | Audio C FFI, more media | 4-6h | Zero spawning, expanded corpus |
| **Phase 3** | Profile + optimize | 6-10h | 10-30% faster |
| **Phase 4** | Git hook | 30min | Auto validation |
| **TOTAL** | | **13-20h** | **Superior system complete** |

---

## SUCCESS CRITERIA

**After all phases:**
- ✅ 98/98 tests passing (100%)
- ✅ Zero process spawning (all plugins use C FFI)
- ✅ 1,850+ test files (diverse, comprehensive)
- ✅ Profiled and optimized (10-30% faster)
- ✅ Git hook prevents regressions (smoke tests on commit)
- ✅ Test tracking captures all results (CSV + metadata)
- ✅ Production ready with validated performance

---

## WORKER N=59 INSTRUCTIONS

**Start with Phase 1:**
1. Run full test suite (98 tests)
2. Verify 98/98 passing
3. If failures: Fix immediately
4. Capture results in test_results/
5. Commit validation results

**Then proceed to Phase 2-4 sequentially.**

**Estimated total**: 13-20 hours (11-17 commits) for complete system

**This creates the superior test system with profiling and optimization.**
