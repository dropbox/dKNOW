# Test Suite Structure - Complete Classification

**Date**: 2025-10-30
**Total Tests**: 98 tests across 14 suites
**Test File**: tests/standard_test_suite.rs (2,656 lines)

---

## Is There A Smoke Test?

**YES** - Dedicated smoke test suite implemented (N=56)

**File**: tests/smoke_test.rs
**Tests**: 6 tests
**Runtime**: ~4.4 seconds (well under 1 minute target)
**Command**: `cargo test --release --test smoke_test -- --ignored --test-threads=1`

**Coverage**:
- Video keyframes + object detection (most common operation) - 1.96s
- Audio transcription (common pipeline) - 0.85s
- 4K resolution (edge case) - 0.50s
- HEVC/H.265 codec (modern codec) - 0.51s
- Corrupted file handling (error handling) - 0.16s
- Fast audio extraction (baseline performance) - 0.38s

**Use case**: Quick validation before commit/push to catch regressions fast

---

## Complete Test Classification Table

| Suite | Test Count | Duration Est. | Purpose | Example Tests |
|-------|------------|---------------|---------|---------------|
| **1. Format Validation** | 12 | ~5 min | Verify all major file formats work | MP4, MOV, MKV, AVI, WEBM, MP3, WAV, FLAC, AAC, M4A, WEBP, BMP |
| **2. Performance Validation** | 1 | ~15 sec | Catch performance regressions | Cache performance check |
| **3. Edge Cases** | 7 | ~15 sec | Critical boundary conditions | 4K video, corrupted file, no audio, single frame, HEVC codec, low bitrate, silent audio |
| **4. Stress Testing** | 2 | ~2-3 min | Large file handling | 980MB video, 1.3GB video |
| **5. Video Codec Characteristics** | 4 | ~1 min | Codec compatibility | H.264, H.265/HEVC, MSMPEG4v3, VP9 |
| **6. Video Resolution Characteristics** | 5 | ~2 min | Resolution handling | 1080p, 4K UHD, low-res, tiny 64x64, unusual aspect ratio |
| **7. Video Size Characteristics** | 6 | ~3 min | File size variations | Tiny (<1MB), small (1-10MB), medium (10-100MB), large (100MB-1GB), very large (>1GB) |
| **8. Audio Characteristics** | 8 | ~2 min | Audio format variations | MP3 codec, AAC codec, FLAC codec, 8kHz sample rate, 96kHz sample rate, 24-bit depth, stereo, 5.1 surround |
| **9. Duration Characteristics** | 6 | ~90 min | Video length variations | Very short (<5s), short (5-30s), medium (30-300s), long (5-30min), very long (>60min) |
| **10. Negative Tests** | 12 | ~30 sec | Error handling | Missing file, corrupted file, no audio stream, no video stream, unsupported format, invalid path |
| **11. Property-Based** | 5 | ~1 min | Invariant validation | All MP4s support keyframes, all videos support audio, all operations produce output |
| **12. Random Sampling** | 10 | ~5 min | Diverse file coverage | Random MP4s, random AVIs, random MP3s, random WEBMs, mixed formats |
| **13. Multi-Operation Pipelines** | 10 | ~8 min | Pipeline correctness | audio→transcription, keyframes→detection, audio→diarization, keyframes→embeddings |
| **14. Additional Coverage** | 10 | ~5 min | Expanded plugin coverage | Face detection, OCR, diarization, scene detection, all 3 embedding types |
| **TOTAL** | **98** | **~130 min** | **Comprehensive validation** | |

---

## Test Pyramid

```
              ┌─────────────┐
              │  Stress (2) │ ← Slowest, most comprehensive
              ├─────────────┤
              │ Duration (6)│
              ├─────────────┤
          ┌───┴──────────────┴───┐
          │   Pipelines (10)      │
          │   Additional (10)     │
          ├────────────────────────┤
       ┌──┴────────────────────────┴──┐
       │  Characteristics (29)         │
       │  Random Sampling (10)         │
       ├────────────────────────────────┤
    ┌──┴────────────────────────────────┴──┐
    │  Format Validation (12)               │ ← Medium speed
    │  Property Tests (5)                   │
    ├───────────────────────────────────────┤
 ┌──┴───────────────────────────────────────┴──┐
 │  Edge Cases (7)                              │ ← Fastest, critical
 │  Negative Tests (12)                         │
 │  Performance Validation (1)                  │
 └──────────────────────────────────────────────┘
```

---

## Test Types by Purpose

### Functional Tests (85 tests)
**Purpose**: Verify operations work correctly
- Format validation: 12
- Codec characteristics: 4
- Resolution characteristics: 5
- Size characteristics: 6
- Audio characteristics: 8
- Duration characteristics: 6
- Pipelines: 10
- Additional coverage: 10
- Random sampling: 10
- Property-based: 5
- Edge cases: 7
- Stress tests: 2

### Negative Tests (12 tests)
**Purpose**: Verify error handling
- Missing files
- Corrupted files
- No audio/video streams
- Unsupported formats
- Invalid paths

### Performance Tests (1 test)
**Purpose**: Catch performance regressions
- Cache performance validation
- (Note: Some other tests have performance thresholds)

---

## Quick Test Subsets

### Smoke Test (IMPLEMENTED N=56)
**Duration**: ~4.4 seconds
**Tests**: 6 critical tests
**Purpose**: Fast validation before commit
**File**: tests/smoke_test.rs

**Includes:**
```
1. smoke_video_keyframes_detection (keyframes+detection) - 1.96s
2. smoke_audio_transcription (audio/transcription) - 0.85s
3. smoke_4k_resolution (large resolution) - 0.50s
4. smoke_hevc_codec (modern codec) - 0.51s
5. smoke_corrupted_file (error handling) - 0.16s
6. smoke_audio_extraction (baseline performance) - 0.38s
```

**Run**: `cargo test --release --test smoke_test -- --ignored --test-threads=1`

### Fast Suite (Current: Edge Cases + Negative)
**Duration**: ~1 minute
**Tests**: 19 tests (edge_case_* + negative_*)
**Purpose**: Quick validation

```bash
cargo test --release --test standard_test_suite -- --ignored edge_case negative
```

### Medium Suite (Formats + Characteristics)
**Duration**: ~10 minutes
**Tests**: 49 tests
**Purpose**: Thorough format/codec validation

```bash
cargo test --release --test standard_test_suite -- --ignored format characteristic
```

### Full Suite (All)
**Duration**: ~130 minutes (2+ hours)
**Tests**: 98 tests
**Purpose**: Complete validation before release

```bash
cargo test --release --test standard_test_suite -- --ignored --test-threads=1
```

---

## Test Suite Quality Assessment

### Coverage ✅
- ✅ All major formats (12 formats)
- ✅ Multiple codecs (H.264, H.265, VP9, MSMPEG4v3)
- ✅ Resolution range (64x64 to 4K)
- ✅ File sizes (146KB to 1.3GB)
- ✅ Error conditions (12 negative tests)
- ✅ Pipeline compositions (10 tests)

### Performance ⚠️
- ✅ Timing measured for all tests
- ⚠️ Only 1 explicit performance test
- ⚠️ ~20% of tests have thresholds
- Recommendation: Add more performance gates

### Correctness ❌
- ❌ No output content validation
- ❌ No golden output comparison
- ❌ No accuracy metrics
- Recommendation: Add snapshot testing (your directive)

---

## Summary for User

**Q1: "is there a smoke test?"**

**A: YES** (implemented N=56) - tests/smoke_test.rs (6 tests, ~4.4 seconds)

**Run**: `cargo test --release --test smoke_test -- --ignored --test-threads=1`

**Q2: "table of different classes of tests"**

**A: 14 test suites organized by purpose:**

| Category | Count | Duration | What It Tests |
|----------|-------|----------|---------------|
| **Format** | 12 | 5 min | File format compatibility |
| **Performance** | 1 | 15 sec | Speed regressions |
| **Edge Cases** | 7 | 15 sec | Boundary conditions |
| **Stress** | 2 | 150 sec | Large files (1GB+) |
| **Codec** | 4 | 1 min | Video codec support |
| **Resolution** | 5 | 2 min | Resolution range |
| **Size** | 6 | 3 min | File size handling |
| **Audio** | 8 | 2 min | Audio format variations |
| **Duration** | 6 | 90 min | Video length range |
| **Negative** | 12 | 30 sec | Error handling |
| **Property** | 5 | 1 min | Invariant checks |
| **Sampling** | 10 | 5 min | Diverse coverage |
| **Pipelines** | 10 | 8 min | Multi-op correctness |
| **Additional** | 10 | 5 min | Plugin coverage |

**Total**: 98 tests, well-organized, comprehensive coverage ✅
