# Test Expansion Plan - Before Threading Optimizations
**Date**: 2025-10-31 (N=145, pre-N=146)
**Purpose**: Add comprehensive tests BEFORE attempting complex performance optimizations
**Rationale**: Avoid N=128 false optimization claims - must have baseline + regression detection

---

## Current Test Infrastructure ✅ EXCELLENT

**Test Result Tracking** (tests/test_result_tracker.rs):
- ✅ Automatic timing capture (duration_secs)
- ✅ System metadata (git hash, CPU, memory, hostname)
- ✅ CSV export (test_results/latest/test_results.csv)
- ✅ Performance summaries (fastest/slowest tests)
- ✅ Failed test reports with error messages
- ✅ File size tracking

**Current Coverage** (159 tests total):
- 45 smoke tests (comprehensive, <60s target)
- 116 standard integration tests
- Recent run: 77 tests, 79.2% pass rate, 21.9 min runtime
- Fastest test: 0.14s (corrupted file)
- Slowest test: 192.13s (multi-speaker diarization)
- Average: 17.08s per test

---

## Test Gaps Analysis

### Gap 1: No Baseline Performance Benchmarks ❌

**Problem**: We have timing data but no "expected baseline" to compare against
- Tests pass/fail on correctness, not performance
- No regression detection if performance degrades
- Can't tell if optimization actually improved anything

**Example**: N=128 claimed 16.1% speedup but code never compiled
- If we had baseline benchmarks, we'd catch this immediately
- Tests would fail if performance regressed >5%

**Needed**:
- Baseline performance file (baseline_performance.json)
- Performance regression tests (fail if >10% slower than baseline)
- Performance improvement validation (verify claimed speedups)

---

### Gap 2: No Memory Profiling in Tests ❌

**Problem**: Tests don't track peak memory usage
- Can't detect memory leaks
- Can't validate memory optimizations
- No baseline for "expected memory"

**Current Memory Analysis** (ad-hoc, not automated):
- N=142: Manual memory investigation (long videos)
- N=184: Extended profiling with manual RSS tracking
- Not integrated into test suite

**Needed**:
- Peak RSS tracking per test
- Memory regression detection
- Memory growth tests (run same operation 10x, check for leaks)

---

### Gap 3: No Throughput Tests ❌

**Problem**: Tests measure wall-clock time but not throughput metrics
- No files/sec measurements
- No MB/sec measurements
- No operations/sec measurements

**Needed**:
- Bulk mode throughput tests (files/sec)
- Data throughput tests (MB/sec for video decoding, audio transcription)
- Inference throughput tests (frames/sec for object detection)

---

### Gap 4: No Performance Variability Tests ❌

**Problem**: Single test run can be misleading
- No standard deviation measurements
- No outlier detection
- Can't distinguish real improvements from noise

**Example**: 6% speedup might be:
- Real: 10.0s → 9.4s consistently (stddev: 0.05s)
- Noise: 10.0s ± 0.6s → 9.4s ± 0.7s (overlapping ranges)

**Needed**:
- Run each benchmark 10x, report median + stddev
- Statistical significance testing (t-test for claimed improvements)
- Warmup runs (first run often slower due to cold cache)

---

### Gap 5: No Stress Tests with File Characteristics ❌

**Problem**: Tests use diverse files but don't systematically test characteristics
- No "large file" suite (1GB+)
- No "many keyframes" suite (1000+ keyframes)
- No "high resolution" suite (4K, 8K)
- No "long duration" suite (60+ minutes)

**Current**: We have large files but no systematic testing

**Needed**:
- File size categories: <10MB, 10-100MB, 100MB-1GB, >1GB
- Keyframe density: Low (<30/min), Medium (30-120/min), High (>120/min)
- Resolution: SD (640x480), HD (1920x1080), 4K (3840x2160), 8K (7680x4320)
- Duration: Short (<1min), Medium (1-10min), Long (10-60min), Very Long (>60min)

---

### Gap 6: No Concurrent Execution Tests ❌

**Problem**: No tests for thread safety or concurrent workloads
- Bulk mode tested but not stress-tested
- No race condition detection
- No deadlock detection

**Needed**:
- Concurrent keyframes extraction (N files simultaneously)
- Concurrent inference (N ONNX sessions in parallel)
- Concurrent transcription (whisper-rs thread safety)
- Detect hangs/deadlocks (timeout after 5 minutes)

---

### Gap 7: No Pre/Post Optimization Comparison Framework ❌

**Problem**: No structured way to validate optimizations
- Claimed improvements not verified
- Regressions not detected
- Trade-offs not measured (e.g., speed vs memory)

**Needed**:
- Optimization validation framework
- Before/after snapshots
- Automated comparison reports
- Fail CI if optimization breaks tests or regresses performance

---

## Proposed Test Additions (50+ new tests)

### Suite 17: Baseline Performance Benchmarks (10 tests)

**Purpose**: Establish baseline for all 21 plugins

```rust
#[test]
#[ignore]
fn baseline_keyframes_small_file() {
    let baseline = BaselinePerformance::load("baselines/keyframes_small.json");
    let result = run_video_extract("keyframes", "test_edge_cases/video_tiny_64x64.mp4");

    assert!(result.passed);
    assert_within_tolerance(result.duration_secs, baseline.median, baseline.tolerance);
    baseline.update_if_better(result.duration_secs);
}

#[test]
#[ignore]
fn baseline_object_detection_1080p() {
    // Measure YOLOv8 inference on 1920x1080 video
    // Baseline: 2.5s (±0.2s) for 30 frames
}

#[test]
#[ignore]
fn baseline_transcription_whisper_base() {
    // Measure Whisper base on 1-minute audio
    // Baseline: 6.5x real-time (9.2s for 60s audio)
}

// Similar tests for all 21 plugins
```

**Expected baselines** (from existing data):
- Keyframes (small): 0.17s (64x64 video)
- Keyframes (HD): 1.3s (1920x1080 video)
- Object detection: 2.2s (YOLO on 30 frames)
- Transcription: 1.1s (10s audio), 7.9s (1min audio)
- Audio embeddings: 11.7s (56MB WAV)
- Face detection: 1.2s (30 frames)
- OCR: 0.8s (text-heavy image)

---

### Suite 18: Memory Profiling Tests (8 tests)

**Purpose**: Track peak RSS and detect memory leaks

```rust
use sysinfo::{System, ProcessRefreshKind};

#[test]
#[ignore]
fn memory_keyframes_long_video() {
    let mut sys = System::new_all();
    let result = run_video_extract_with_memory_tracking(
        "keyframes",
        "~/Desktop/stuff/stuff/mission control video demo 720.mov", // 7.6 min, 827 frames
        &mut sys
    );

    assert!(result.passed);
    assert!(result.peak_rss_mb < 2000); // Expect ~1.8 GB
    assert!(result.peak_rss_mb > 1500); // Sanity check

    // Memory formula: RSS = (num_frames × width × height × 1.5) + 257 MB
    let expected_rss = (827 * 1280 * 828 * 1.5 / 1_000_000.0) + 257.0;
    assert_within_tolerance(result.peak_rss_mb, expected_rss, 20.0); // ±20%
}

#[test]
#[ignore]
fn memory_leak_detection_repeated_inference() {
    let mut sys = System::new_all();
    let mut peak_rss_values = Vec::new();

    // Run same operation 10 times
    for i in 0..10 {
        let result = run_video_extract("object-detection", "test_edge_cases/video_hevc_h265.mp4");
        peak_rss_values.push(result.peak_rss_mb);
    }

    // Check for memory growth (leak detection)
    let first_3_avg = peak_rss_values[0..3].iter().sum::<f64>() / 3.0;
    let last_3_avg = peak_rss_values[7..10].iter().sum::<f64>() / 3.0;
    let growth_pct = (last_3_avg - first_3_avg) / first_3_avg * 100.0;

    assert!(
        growth_pct < 5.0,
        "Memory leak detected: {}% growth from {} MB to {} MB",
        growth_pct, first_3_avg, last_3_avg
    );
}
```

**Tests needed**:
1. Long video memory (7.6 min, 56 min videos)
2. Repeated inference (10x same operation, check for leaks)
3. Concurrent operations (N parallel, check total RSS)
4. Large batch memory (100 small files in bulk mode)
5. ML model memory (each plugin's peak RSS)
6. FFmpeg memory (video decoding RSS)
7. Audio embeddings memory (mel-spectrogram RSS)
8. Memory release (check RSS drops after operation)

---

### Suite 19: Throughput Benchmarks (8 tests)

**Purpose**: Measure files/sec, MB/sec, frames/sec

```rust
#[test]
#[ignore]
fn throughput_bulk_keyframes() {
    let files: Vec<&str> = vec![/* 20 small MP4 files */];
    let start = Instant::now();

    let result = run_bulk_video_extract("keyframes", &files, 8 /* workers */);
    let duration = start.elapsed().as_secs_f64();

    let throughput_files_sec = files.len() as f64 / duration;
    assert!(result.passed);
    assert!(
        throughput_files_sec > 5.0,
        "Bulk throughput too low: {} files/sec (expected >5.0)",
        throughput_files_sec
    );
}

#[test]
#[ignore]
fn throughput_video_decode_mbps() {
    let file = "~/Desktop/stuff/stuff/mission control video demo 720.mov"; // 277 MB
    let file_size_mb = std::fs::metadata(file).unwrap().len() as f64 / 1_000_000.0;

    let result = run_video_extract("keyframes", file);
    let throughput_mbps = file_size_mb / result.duration_secs;

    assert!(
        throughput_mbps > 4.0,
        "Video decode throughput: {} MB/s (expected >4.0)",
        throughput_mbps
    );
}

#[test]
#[ignore]
fn throughput_inference_frames_per_sec() {
    // Measure YOLOv8 inference throughput
    let result = run_video_extract("object-detection", "test_edge_cases/video_hevc_h265.mp4");
    let num_frames = 30; // Known from video
    let fps = num_frames as f64 / result.duration_secs;

    assert!(
        fps > 10.0,
        "Inference throughput: {} frames/sec (expected >10.0)",
        fps
    );
}
```

**Throughput baselines** (from existing data):
- Bulk keyframes: 5.79 files/sec (N=38)
- Bulk transcription: 2.36 files/sec (N=38)
- Video decode: 5.01 MB/s (N=122)
- Transcription: 7.56 MB/s (6.58x real-time, N=122)
- Scene detection: 2.2 GB/s (keyframe-only, N=111)

---

### Suite 20: Performance Variability Tests (6 tests)

**Purpose**: Run benchmarks 10x, measure consistency

```rust
#[test]
#[ignore]
fn variability_keyframes_consistency() {
    let mut durations = Vec::new();

    // Warmup run (discard)
    run_video_extract("keyframes", "test_edge_cases/video_hevc_h265.mp4");

    // 10 measurement runs
    for _ in 0..10 {
        let result = run_video_extract("keyframes", "test_edge_cases/video_hevc_h265.mp4");
        durations.push(result.duration_secs);
    }

    let median = median(&durations);
    let stddev = stddev(&durations);
    let coeff_of_variation = stddev / median * 100.0;

    println!("Keyframes consistency: median={:.3}s, stddev={:.3}s, CV={:.1}%",
             median, stddev, coeff_of_variation);

    assert!(
        coeff_of_variation < 10.0,
        "High variability: {}% (stddev={}, median={})",
        coeff_of_variation, stddev, median
    );
}

fn median(values: &[f64]) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    sorted[sorted.len() / 2]
}

fn stddev(values: &[f64]) -> f64 {
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / values.len() as f64;
    variance.sqrt()
}
```

**Tests needed**:
1. Keyframes consistency (10 runs)
2. Object detection consistency (10 runs)
3. Transcription consistency (10 runs)
4. Bulk mode consistency (10 runs, 10 files each)
5. Fast mode consistency (10 runs)
6. Cold vs warm cache (first run vs subsequent runs)

---

### Suite 21: Stress Tests - File Characteristics (12 tests)

**Purpose**: Systematically test file size/duration/resolution extremes

```rust
// File size categories
#[test]
#[ignore]
fn stress_tiny_file_under_100kb() {
    // Test overhead for tiny files
    // Expected: Overhead dominates (binary load, FFmpeg init)
}

#[test]
#[ignore]
fn stress_small_file_1_10mb() {
    // test_edge_cases files (~3MB)
}

#[test]
#[ignore]
fn stress_medium_file_10_100mb() {
    // benchmark_n103 files (34-38MB)
}

#[test]
#[ignore]
fn stress_large_file_100mb_1gb() {
    // mission control demo (277MB)
    // Investor update (349MB)
    // GMT recordings (980MB)
}

#[test]
#[ignore]
fn stress_very_large_file_over_1gb() {
    // GMT20250520 recording (1.3GB, 86min)
    assert_timeout(300.0); // 5 minute timeout
}

// Resolution categories
#[test]
#[ignore]
fn stress_tiny_resolution_64x64() {
    test_format("test_edge_cases/video_tiny_64x64.mp4", "object-detection");
}

#[test]
#[ignore]
fn stress_4k_resolution_3840x2160() {
    test_format("test_edge_cases/video_4k_ultra_hd.mp4", "object-detection");
}

#[test]
#[ignore]
fn stress_8k_resolution_7680x4320() {
    // TODO: Generate 8K test file
    // Expected: 16x more pixels than 1080p, ~16x memory
}

// Duration categories
#[test]
#[ignore]
fn stress_very_short_under_1sec() {
    // Test minimum viable duration
}

#[test]
#[ignore]
fn stress_medium_duration_10min() {
    // mission control demo (7.6 min)
}

#[test]
#[ignore]
fn stress_long_duration_60min() {
    // GMT braintrust (56 min)
}

#[test]
#[ignore]
fn stress_very_long_duration_over_60min() {
    // GMT recording (86 min)
    // Test: Memory scaling, progress reporting, timeout handling
}
```

---

### Suite 22: Concurrent Execution Tests (6 tests)

**Purpose**: Test thread safety and concurrent performance

```rust
use std::thread;
use std::sync::Arc;

#[test]
#[ignore]
fn concurrent_keyframes_4_files_parallel() {
    let files = vec![
        "test_edge_cases/video_hevc_h265.mp4",
        "test_edge_cases/video_4k_ultra_hd.mp4",
        "test_edge_cases/video_tiny_64x64.mp4",
        "test_edge_cases/video_single_frame.mp4",
    ];

    let handles: Vec<_> = files.into_iter().map(|file| {
        thread::spawn(move || {
            run_video_extract("keyframes", file)
        })
    }).collect();

    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    // All should pass
    assert!(results.iter().all(|r| r.passed));

    // No hangs (completed in reasonable time)
    assert!(results.iter().all(|r| r.duration_secs < 30.0));
}

#[test]
#[ignore]
fn concurrent_transcription_whisper_thread_safety() {
    // Test whisper-rs thread safety (known issue from UPSTREAM_IMPROVEMENTS.md)
    // With current Mutex wrapper, this should NOT hang

    let files = vec![
        "test_edge_cases/audio_lowquality_16kbps.mp3",
        "/Users/ayates/docling/tests/data/audio/sample_10s_audio-aac.aac",
        "test_edge_cases/audio_complete_silence_3sec.wav",
    ];

    let handles: Vec<_> = files.into_iter().map(|file| {
        thread::spawn(move || {
            run_video_extract("transcription", file)
        })
    }).collect();

    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    assert!(results.iter().all(|r| r.passed), "Transcription thread safety issue detected");
}

#[test]
#[ignore]
fn concurrent_onnx_inference_session_pool() {
    // Test ONNX Runtime session pool (8 concurrent sessions)
    let file = "test_edge_cases/video_hevc_h265.mp4";

    let handles: Vec<_> = (0..8).map(|_| {
        let f = file.to_string();
        thread::spawn(move || {
            run_video_extract("object-detection", &f)
        })
    }).collect();

    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert!(results.iter().all(|r| r.passed));
}
```

---

### Suite 23: Optimization Validation Framework (4 tests)

**Purpose**: Before/after comparison for claimed optimizations

```rust
#[test]
#[ignore]
fn validate_optimization_claimed_speedup() {
    // This test would be run AFTER implementing an optimization
    // It compares current performance against pre-optimization baseline

    let baseline_file = "baselines/pre_optimization_n146_mozjpeg.json";
    let baseline = BaselinePerformance::load(baseline_file);

    let result = run_video_extract("keyframes", "test_edge_cases/video_hevc_h265.mp4");

    let speedup = baseline.median / result.duration_secs;
    let claimed_speedup = 2.5; // Example: mozjpeg claims 2-3x

    assert!(
        speedup >= claimed_speedup * 0.8,
        "Optimization underperformed: {:.2}x speedup (claimed {}x)",
        speedup, claimed_speedup
    );

    assert!(
        speedup <= claimed_speedup * 1.5,
        "Optimization overperformed (measurement error?): {:.2}x speedup (claimed {}x)",
        speedup, claimed_speedup
    );
}

#[test]
#[ignore]
fn validate_no_regression_after_optimization() {
    // Run ALL baseline tests, ensure none regressed >5%
    let baseline_suite = BaselinePerformance::load_suite("baselines/all_plugins.json");
    let current_results = run_all_baseline_tests();

    for (test_name, current_result) in current_results {
        let baseline = baseline_suite.get(&test_name).unwrap();
        let regression_pct = (current_result.duration_secs - baseline.median) / baseline.median * 100.0;

        assert!(
            regression_pct < 10.0,
            "REGRESSION DETECTED: {} is {:.1}% slower ({:.3}s vs baseline {:.3}s)",
            test_name, regression_pct, current_result.duration_secs, baseline.median
        );
    }
}
```

---

## Implementation Plan

### Phase 1: Foundation (N=146-148, ~3 commits)

1. **N=146: Baseline performance framework**
   - Create `BaselinePerformance` struct
   - Add `baselines/` directory with JSON files
   - Implement load/save/compare utilities
   - Document baseline recording process

2. **N=147: Memory tracking infrastructure**
   - Add `sysinfo` dependency
   - Implement `run_video_extract_with_memory_tracking()`
   - Add peak RSS to TestResultRow
   - Update test_result_tracker.rs

3. **N=148: Statistical utilities**
   - Implement median/stddev/coefficient_of_variation
   - Add `assert_within_tolerance()` helper
   - Add `assert_statistically_significant()` (t-test)

### Phase 2: Baseline Tests (N=149-151, ~3 commits)

4. **N=149: Suite 17 - Baseline benchmarks (part 1)**
   - Add 5 baseline tests (keyframes, object-detection, transcription, audio-embeddings, face-detection)
   - Run 10x each, record median + stddev
   - Save to baselines/

5. **N=150: Cleanup cycle (N mod 5)**
   - Standard cleanup
   - Verify 0 clippy warnings
   - Update documentation

6. **N=151: Suite 17 - Baseline benchmarks (part 2)**
   - Add remaining 16 baseline tests (all plugins)
   - Complete baseline suite
   - Document baseline values in README

### Phase 3: Memory & Throughput (N=152-154, ~3 commits)

7. **N=152: Suite 18 - Memory profiling**
   - Add 8 memory tests
   - Validate memory leak detection works
   - Document expected RSS values

8. **N=153: Suite 19 - Throughput benchmarks**
   - Add 8 throughput tests
   - Measure files/sec, MB/sec, frames/sec
   - Compare against known baselines

9. **N=154: Suite 20 - Performance variability**
   - Add 6 consistency tests
   - Run each benchmark 10x
   - Measure coefficient of variation

### Phase 4: Stress & Concurrency (N=155-156, ~2 commits)

10. **N=155: Suite 21 - Stress tests**
    - Add 12 file characteristic tests
    - Test size/resolution/duration extremes
    - Generate 8K test file if needed

11. **N=156: Suite 22 - Concurrent execution**
    - Add 6 concurrent tests
    - Validate thread safety
    - Test race conditions

### Phase 5: Optimization Framework (N=157-158, ~2 commits)

12. **N=157: Suite 23 - Optimization validation**
    - Add 4 optimization validation tests
    - Implement regression detection
    - Document usage for future optimizations

13. **N=158: Integration & documentation**
    - Run full test suite (159 + 54 new = 213 tests)
    - Verify all pass
    - Update README with test counts
    - Document baseline recording process

---

## Expected Results

**After implementation**:
- ✅ 213 total tests (159 existing + 54 new)
- ✅ Baseline benchmarks for all 21 plugins
- ✅ Memory profiling for all major operations
- ✅ Throughput metrics (files/sec, MB/sec, frames/sec)
- ✅ Performance regression detection (<10% tolerance)
- ✅ Optimization validation framework
- ✅ Statistical significance testing (t-test)

**Benefits**:
1. **Prevent false optimization claims** (like N=128)
2. **Detect regressions immediately** (CI fails if >10% slower)
3. **Validate real improvements** (statistical significance)
4. **Memory leak detection** (automated, not manual)
5. **Comprehensive performance tracking** (not just correctness)

**Estimated effort**: 13 AI commits (N=146-158)
**Timeline**: ~2 weeks (at 1 commit/day pace)

**Before starting LOCAL_PERFORMANCE_IMPROVEMENTS.md optimizations**, we should complete this test suite to have:
- Baseline measurements
- Regression detection
- Optimization validation

---

## Link to Optimization Roadmap

**Workflow**:
1. ✅ **N=146-158: Add comprehensive tests** (THIS DOCUMENT)
2. ⏳ **N=159+: Begin optimizations** (LOCAL_PERFORMANCE_IMPROVEMENTS.md)
3. ✅ **Validate each optimization** (Suite 23)
4. ✅ **Detect regressions** (CI fails if tests regress)
5. ✅ **Measure real gains** (not estimates)

**Example optimization workflow**:
```bash
# N=159: Before mozjpeg optimization
cargo test --release baseline_keyframes_small -- --ignored
# Result: 1.30s (±0.05s) median, saved to baselines/pre_n159_mozjpeg.json

# N=159: Implement mozjpeg
# ... code changes ...

# N=159: After mozjpeg optimization
cargo test --release baseline_keyframes_small -- --ignored
# Result: 0.52s (±0.03s) median

# N=159: Validate optimization
cargo test --release validate_optimization_claimed_speedup -- --ignored
# ✅ Speedup: 2.50x (claimed: 2-3x) - VALIDATED

# N=159: Check for regressions
cargo test --release validate_no_regression_after_optimization -- --ignored
# ✅ All tests within 10% of baseline - NO REGRESSIONS
```

---

## Next Steps for Worker AI

**N=146**: Start with Phase 1 - Baseline performance framework
- Read this document completely
- Implement BaselinePerformance struct
- Create baselines/ directory structure
- Test with one plugin (keyframes)

**Do NOT**:
- Start optimizations before completing test suite
- Skip baseline recording
- Claim improvements without validation
- Break existing 159 tests
