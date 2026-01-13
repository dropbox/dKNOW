# Test Framework Guide - Complete Reference

## Overview

The video-audio-extracts project uses a **standard Rust test framework** with integration tests that validate the CLI binary end-to-end. No ad-hoc shell scripts or Python test harnesses.

**Total Tests**: 148 automated tests
- 6 legacy smoke tests (tests/smoke_test.rs)
- 43 comprehensive smoke tests (tests/smoke_test_comprehensive.rs)
- 99 integration tests (tests/standard_test_suite.rs)

## Test Framework Architecture

### 1. Test Files

#### tests/smoke_test_comprehensive.rs (PRIMARY)
**Purpose**: Fast pre-commit validation
**Test Count**: 43 tests
**Target Time**: <120 seconds with thread limiting
**Used By**: Pre-commit hook

**Coverage**:
- **Formats** (16 tests): MP4, MOV, MKV, WEBM, M4A, WAV, MP3, FLAC, AAC, WEBP, BMP, FLV, 3GP, WMV, OGV, M4V, OGG, OPUS
- **Core Operations** (6 tests): Keyframes, audio, transcription, object detection, face detection, OCR
- **Tier 1 Plugins** (5 tests): Motion tracking, action recognition, smart thumbnail, subtitle extraction, audio classification
- **Tier 2 Plugins** (6 tests): Pose estimation, emotion detection, image quality, audio enhancement, shot classification, scene detection
- **Edge Cases** (10 tests): 4K, HEVC, corrupted file, VFR, high FPS, tiny resolution, no audio, single frame, silence, low bitrate

**Run Command**:
```bash
VIDEO_EXTRACT_THREADS=4 cargo test --release --test smoke_test_comprehensive -- --ignored --test-threads=1
```

#### tests/smoke_test.rs (LEGACY)
**Purpose**: Original smoke tests (deprecated in favor of comprehensive)
**Test Count**: 6 tests
**Target Time**: <1 minute

**Tests**:
1. `smoke_video_keyframes_detection` - Most common operation
2. `smoke_audio_transcription` - Audio pipeline
3. `smoke_4k_resolution` - High resolution handling
4. `smoke_corrupted_file` - Error handling
5. `smoke_audio_extraction` - Fast baseline
6. `smoke_hevc_codec` - Modern codec support

**Run Command**:
```bash
VIDEO_EXTRACT_THREADS=4 cargo test --release --test smoke_test -- --ignored --test-threads=1
```

#### tests/standard_test_suite.rs (COMPREHENSIVE)
**Purpose**: Full integration test suite
**Test Count**: 99 tests
**Run Time**: ~130 minutes (full suite)

**Test Suites**:

**Suite 1: Format Validation (19 tests)**
- Validates all supported media formats
- Tests: MP4, MOV, MKV, WEBM, M4A, WAV, MP3, FLAC, AAC, WEBP, BMP, FLV, 3GP, WMV, OGV, M4V, OGG, OPUS, AVI (expects error)

**Suite 2: Performance Validation (1 test)**
- Cache validation (keyframes → object-detection)
- Ensures intermediate results are reused

**Suite 3: Edge Case Validation (7 tests)**
- No audio stream, silent audio, HEVC codec, 4K resolution, corrupted file, single frame, low bitrate

**Suite 4: Stress Testing (2 tests)**
- 1.3GB video (long duration)
- 980MB video (high resolution)

**Suite 5: Video Codec Characteristics (4 tests)**
- H.264, H.265/HEVC, VP9 (WebM), MSMPEG4v3 (AVI)

**Suite 6: Video Resolution Characteristics (5 tests)**
- 64x64 (tiny), 1080p, 4K UHD, unusual aspect ratio, low-res

**Suite 7: Video Size Characteristics (6 tests)**
- <100KB, 11MB, 38MB, 349MB, 980MB, 1.3GB

**Suite 8: Audio Characteristics (8 tests)**
- Codec tests: AAC, MP3, FLAC, WAV
- Size tests: 146KB, 1.1MB, 13MB, 56MB

**Suite 9: Duration Characteristics (6 tests)**
- 1 second, 2 seconds, 3 seconds, 10 seconds, ~5 minutes, ~60 minutes

**Suite 10: Negative Tests (12 tests)**
- Wrong operations on wrong file types (e.g., keyframes on audio-only)
- Validates graceful error handling

**Suite 11: Property-Based Testing (5 tests)**
- All MP4 files support keyframes
- All audio files support transcription
- All video files support audio extraction
- Corrupted files always fail gracefully
- Wrong operations always fail with clear errors

**Suite 12: Random Sampling Tests (10 tests)**
- Random samples from different datasets
- Validates robustness across diverse inputs

**Suite 13: Multi-Operation Pipelines (10 tests)**
- Sequential pipelines (audio→transcription)
- Parallel pipelines ([audio,keyframes])
- Complex chains (keyframes→[object-detection,face-detection,ocr])

**Suite 14: Additional Coverage (10 tests)**
- Vision embeddings, text embeddings, audio embeddings
- Scene detection (long videos, action videos)
- Speaker diarization, OCR on text-heavy videos
- Face detection on multi-face videos

**Suite 15: Tier 2 Plugin Tests (4 tests)**
- Pose estimation, emotion detection, image quality assessment, audio enhancement metadata

**Suite 16: Tier 1 Plugin Tests (5 tests)**
- Motion tracking, action recognition, smart thumbnail, subtitle extraction, audio classification

**Run Commands**:
```bash
# Full suite
VIDEO_EXTRACT_THREADS=4 cargo test --release --test standard_test_suite -- --ignored --test-threads=1

# Specific suite
VIDEO_EXTRACT_THREADS=4 cargo test --release --test standard_test_suite -- --ignored characteristic_audio

# Single test
VIDEO_EXTRACT_THREADS=4 cargo test --release --test standard_test_suite format_mp4_quick_pipeline -- --ignored
```

#### tests/test_result_tracker.rs
**Purpose**: Utility module for test result tracking
**Features**:
- Tracks test results (pass/fail, duration, file size)
- Generates CSV reports for analysis
- Creates timestamped output directories
- Tracks system metadata (hostname, commit hash)

## Test Infrastructure

### Test Media Files

**Location**: Multiple directories
**Total Files**: 1,837 test files

**Categories**:
1. **test_edge_cases/** (13 files)
   - Purpose-built edge case files
   - 4K video, HEVC, VFR, high FPS, corrupted, no audio, etc.

2. **~/Desktop/stuff/stuff/**
   - Real-world video files (screen recordings, Zoom meetings)
   - Sizes: 34MB - 1.3GB
   - Formats: MP4, MOV

3. **~/Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/**
   - Kinetics-600 dataset samples
   - Action recognition datasets
   - Low-res super resolution datasets

4. **Test files from user directories**
   - Audio files (WAV, FLAC, MP3, M4A, AAC)
   - Image files (WEBP, BMP)

**Documentation**: See COMPLETE_TEST_FILE_INVENTORY.md for full catalog

### Thread Limiting (CRITICAL)

**Problem**: Each test spawns a video-extract binary that creates:
- Rayon thread pool: 16 threads (all CPU cores)
- ONNX Runtime thread pool: 16 threads (all physical cores)
- FFmpeg threads: variable

**Result**: 32-48+ threads per test, overwhelming system on high-core-count machines

**Solution**: Set `VIDEO_EXTRACT_THREADS=4` before running tests

**Example**:
```bash
# Correct (prevents system overload)
VIDEO_EXTRACT_THREADS=4 cargo test --release --test smoke_test -- --ignored --test-threads=1

# Wrong (may crash system)
cargo test --release --test smoke_test -- --ignored --test-threads=1
```

**See**: TEST_THREAD_LIMITING.md for complete documentation

### Test Execution Pattern

All tests use this pattern:

```rust
#[test]
#[ignore]  // Run only with --ignored flag
fn test_name() {
    let file = PathBuf::from("test_file.mp4");

    // Execute via CLI binary (integration test)
    let result = run_video_extract("operation", &file);

    // Track results
    record_test_result("test_name", "suite", "operation", Some(&file), &result);

    // Assert success
    assert!(result.passed, "Test failed: {:?}", result.error);

    // Assert performance
    assert!(result.duration_secs < 10.0, "Too slow: {:.2}s", result.duration_secs);
}
```

**Key characteristics**:
1. Tests are marked `#[ignore]` - run only with `--ignored` flag
2. Tests execute the actual CLI binary (realistic usage)
3. Results are tracked for analysis
4. Performance assertions catch regressions

## Pre-Commit Hook

**Location**: `.git/hooks/pre-commit` (not tracked in git)

**What it runs**:
1. Comprehensive smoke tests (43 tests, ~40-60s)
2. Cargo fmt --check (Rust formatting)
3. Cargo clippy (Rust linting)
4. Black + flake8 (if Python files changed)
5. Clang-format (if C++ files changed)

**Command used**:
```bash
VIDEO_EXTRACT_THREADS=4 cargo test --release --test smoke_test_comprehensive -- --ignored --test-threads=1 --quiet
```

**Performance**: ~40-60 seconds with thread limiting

**Bypass** (not recommended):
```bash
git commit --no-verify -m "message"
```

**See**: GIT_HOOK_CONFIG.md for configuration details

## Test Result Tracking

### Output Structure

```
test_results/
├── latest/                          # Symlink to most recent run
├── 2025-10-31_14-30-45_smoke/      # Timestamped run
│   ├── summary.txt                  # Human-readable summary
│   ├── test_results.csv             # Detailed results
│   └── metadata.json                # Run metadata
└── 2025-10-31_15-45-23_standard/
    ├── summary.txt
    ├── test_results.csv
    └── metadata.json
```

### CSV Format

```csv
test_name,suite,status,duration_secs,error_message,file_path,operation,file_size_bytes
smoke_video_keyframes_detection,smoke_tests,passed,2.18,,test_edge_cases/video_variable_framerate_vfr__timing_test.mp4,keyframes,12345678
```

**Columns**:
- `test_name`: Rust test function name
- `suite`: Test suite category
- `status`: passed/failed
- `duration_secs`: Execution time
- `error_message`: Error details (if failed)
- `file_path`: Input file path
- `operation`: Operations performed
- `file_size_bytes`: Input file size

## Test Philosophy

### 1. No Ad-Hoc Scripts
**Rule**: Use standard Rust test framework only
**Rationale**: Consistent, maintainable, CI-friendly

**Don't**:
```bash
# Don't create shell scripts
./test_something.sh
```

**Do**:
```bash
# Use Rust tests
cargo test --test standard_test_suite
```

### 2. Integration Over Unit Tests
**Rule**: Test via CLI binary (end-to-end)
**Rationale**: Tests realistic usage, catches integration issues

**Example**:
```rust
// Integration test (correct)
let output = Command::new("./target/release/video-extract")
    .args(["debug", "--ops", "keyframes", "video.mp4"])
    .output()
    .expect("Failed to execute");

assert!(output.status.success());
```

### 3. Test Real Files
**Rule**: Use diverse real-world media files
**Rationale**: Catches format-specific issues

**Coverage**:
- Multiple formats (MP4, MOV, MKV, etc.)
- Multiple codecs (H.264, HEVC, VP9, etc.)
- Multiple resolutions (64x64 to 4K)
- Multiple durations (1s to 60min)

### 4. Track Results
**Rule**: Record all test results to CSV
**Rationale**: Historical analysis, performance regression detection

### 5. Thread Limiting
**Rule**: Always use VIDEO_EXTRACT_THREADS=4 for tests
**Rationale**: Prevents system overload on high-core-count machines

## Running Tests

### Quick Validation (Pre-Commit)

```bash
# Comprehensive smoke tests (~40-60s)
VIDEO_EXTRACT_THREADS=4 cargo test --release --test smoke_test_comprehensive -- --ignored --test-threads=1
```

### Full Validation

```bash
# All 148 tests (~130 minutes)
VIDEO_EXTRACT_THREADS=4 cargo test --release --test standard_test_suite -- --ignored --test-threads=1
VIDEO_EXTRACT_THREADS=4 cargo test --release --test smoke_test_comprehensive -- --ignored --test-threads=1
```

### Specific Test Suites

```bash
# Format validation only
VIDEO_EXTRACT_THREADS=4 cargo test --release --test standard_test_suite -- --ignored format_

# Audio characteristics only
VIDEO_EXTRACT_THREADS=4 cargo test --release --test standard_test_suite -- --ignored characteristic_audio

# Edge cases only
VIDEO_EXTRACT_THREADS=4 cargo test --release --test standard_test_suite -- --ignored edge_case_
```

### Single Test

```bash
# Run one specific test
VIDEO_EXTRACT_THREADS=4 cargo test --release --test smoke_test smoke_video_keyframes_detection -- --ignored --nocapture
```

## Test Development Guidelines

### Adding New Tests

1. **Choose appropriate suite**:
   - Smoke test: Critical path, fast validation
   - Standard suite: Comprehensive coverage

2. **Follow naming convention**:
   - `{category}_{specific_test}` (e.g., `format_mp4_quick_pipeline`)

3. **Use existing helper functions**:
   - `run_video_extract()` - Execute CLI
   - `record_test_result()` - Track results

4. **Add performance assertions**:
   ```rust
   assert!(result.duration_secs < 10.0, "Too slow: {:.2}s", result.duration_secs);
   ```

5. **Handle missing files gracefully**:
   ```rust
   if !file.exists() {
       eprintln!("⚠️  Test file not found: {}", file.display());
       return;  // Don't panic, just skip
   }
   ```

### Test Maintenance

1. **Update thread limiting**: Ensure all test commands use VIDEO_EXTRACT_THREADS=4
2. **Track test times**: Monitor for performance regressions
3. **Keep test files organized**: Document in COMPLETE_TEST_FILE_INVENTORY.md
4. **Update pre-commit hook**: When adding critical smoke tests

## Troubleshooting

### Tests Hang or Crash System

**Problem**: Thread oversubscription
**Solution**: Use VIDEO_EXTRACT_THREADS=4

### Tests Fail Randomly

**Problem**: Files not available (Dropbox sync)
**Solution**: Tests handle missing files gracefully

### Tests Too Slow

**Problem**: Not using release build or thread limiting
**Solution**:
- Use `--release` flag
- Set VIDEO_EXTRACT_THREADS=4
- Run specific suites instead of full suite

### Pre-Commit Hook Fails

**Problem**: Tests fail in hook but pass manually
**Solution**:
- Check `.git/hooks/pre-commit` has VIDEO_EXTRACT_THREADS=4
- Run hook command manually to debug
- Check binary is up to date: `cargo build --release`

## See Also

- **COMPREHENSIVE_AUDIT_N102.md** - Complete audit findings
- **RUN_STANDARD_TESTS.md** - Test execution guide
- **TEST_THREAD_LIMITING.md** - Thread limiting documentation
- **GIT_HOOK_CONFIG.md** - Pre-commit hook configuration
- **COMPLETE_TEST_FILE_INVENTORY.md** - Test media catalog
