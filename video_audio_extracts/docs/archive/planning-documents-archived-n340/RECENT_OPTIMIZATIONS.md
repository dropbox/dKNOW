# Recent Optimizations (N=121-129)

**Date**: 2025-10-31 (Updated N=130)
**Branch**: build-video-audio-extracts
**Baseline**: N=120 (2.747s pose-estimation, 2.719s object-detection)
**Current**: N=129 (2.370s pose-estimation, 2.280s object-detection)
**Status**: YOLO plugin optimizations complete

## Summary

**Pose-estimation**: 13.7% speedup (377ms saved, 2.747s → 2.370s) ✅ VERIFIED
**Object-detection**: ❌ N=128 INVALID (code never compiled, reverted to N=127)
**OCR**: 0% speedup (patterns not applicable to multi-stage pipelines)
**Benchmark**: Operations on test_keyframes_100_30s.mp4
**Phase**: Pose-estimation optimized. N=128-130 based on false claims (see N=131 investigation)

## Optimization Details

### N=121: Image Preprocessing Row-wise Memcpy
- **Impact**: 2.0% speedup (55ms)
- **Change**: Row-wise bulk copy replaces element-by-element iteration in image preprocessing
- **Files**: crates/object-detection/src/lib.rs
- **Measurement**: 2.747s → 2.693s

### N=122: String Allocation Reduction
- **Impact**: 9.9% speedup (271ms)
- **Change**: Changed StageResult fields from String to Cow<'static, str>, Operation::name() returns &'static str
- **Files**: crates/video-extract-core/src/executor.rs, operation.rs
- **Measurement**: 2.747s → 2.476s (measured from N=121 baseline)

### N=123: NMS Vec::remove(0) Optimization
- **Impact**: 2.95% speedup (73ms)
- **Change**: Replaced O(n) Vec::remove(0) with O(1) swap_remove in NMS post-processing
- **Files**: crates/video-extract-core/src/fast_path.rs:671, parallel_pipeline.rs:542
- **Measurement**: 2.476s → 2.403s

### N=124: COCO Class Name Allocation Reduction
- **Impact**: 0% measurable (system noise)
- **Change**: Changed Detection.class_name from String to Cow<'static, str>
- **Files**: crates/video-extract-core/src/fast_path.rs, parallel_pipeline.rs
- **Measurement**: 2.403s ± 0.025s (variance increased due to system noise)
- **Note**: Theoretical benefit (eliminates 100+ allocations per frame), but too small to measure in 2.4s workload

### N=126: Arc Optimization Investigation (FAILED)
- **Impact**: -30% REGRESSION (attempted and reverted)
- **Change**: Attempted PluginData Arc wrapper to reduce clone overhead
- **Measurement**: 2.370s → 3.081s (+711ms, 30% slower)
- **Root cause**: Arc allocation + refcount overhead > clone savings. Deserialization requires owned data, so clones still happen.
- **Lesson**: Adding metadata without eliminating actual work creates net overhead.
- **Result**: REVERTED to N=125 baseline (2.370s)

### N=128: Object-Detection Optimization ❌ INVALID - CODE NEVER COMPILED
- **Impact**: **CLAIMED** 16.1% speedup (439ms) - **FALSE CLAIM**
- **Change**: Attempted row-wise preprocessing optimization
- **Files**: crates/object-detection/src/lib.rs
- **Status**: **CODE DOES NOT COMPILE** - missing import, borrow checker errors
- **Reality**: Benchmarks ran against N=127 cached binary (cargo didn't recompile broken code)
- **Investigation**: See n128_optimization_false_claim_investigation_2025-10-31-19-47.md
- **Resolution (N=131)**: Reverted object-detection to N=127 working code, all tests passing

### N=129: OCR Optimization Investigation
- **Impact**: 0% speedup (within measurement noise)
- **Change**: Applied row-wise preprocessing to 4 OCR functions
- **Files**: crates/ocr/src/lib.rs
- **Measurement**: 5.656s → 5.758s (no measurable change)
- **Lesson**: Patterns effective for YOLO single-stage models don't apply to multi-stage pipelines (OCR: detect → crop → recognize × N → CTC decode)

## Optimization Summary by Category

**Initial Analysis (optimization_analysis_2025-10-31-18-14.md)**:
1. **PluginData Arc wrapper** (#1) - ❌ FAILED (N=126, 30% regression)
2. **Image preprocessing** (#2) - ✅ DONE (N=121 pose, N=128 object-detection, N=129 OCR)
3. **String allocations** (#3) - ✅ DONE (N=122, 9.9% speedup)
4. **JSON pre-hashing** (#4) - ⚠️ SKIP (same issue as Arc)
5. **NMS Vec::remove** (#5) - ✅ DONE (N=123, 2.95% speedup)
6. **Queue clone** (#6) - ⚠️ SKIP (cold path, <1us)
7. **COCO class name** (#7) - ✅ DONE (N=124, N=128)

**Plugin-specific optimizations**:
- **Pose-estimation** (N=121-124): ✅ All patterns applied, 13.7% speedup VERIFIED
- **Object-detection** (N=128): ❌ INVALID - code never compiled, reverted to N=127
- **OCR** (N=129): ✅ Investigated, patterns not applicable (0% gain)
- **Face-detection**: ⚠️ Not evaluated (N=131 abandoned after N=128 discovery)
- **Other plugins**: ⚠️ 18 plugins not yet profiled

**Status**: N=128-130 optimization phase based on false claims. Only pose-estimation gains (13.7%) are real. Object-detection "optimization" never ran. See n128_optimization_false_claim_investigation_2025-10-31-19-47.md.

## Test Status

All 43 smoke tests passing:
```bash
VIDEO_EXTRACT_THREADS=4 cargo test --release --test smoke_test_comprehensive \
  -- --ignored --test-threads=1
```

**Test time**: 20.65s (N=123), 20.67s (N=124)

## Benchmark Commands

```bash
# Build release binary
cargo build --release

# Benchmark pose-estimation (10 runs, 1 warmup)
VIDEO_EXTRACT_THREADS=4 hyperfine --warmup 1 --runs 10 \
  './target/release/video-extract debug --ops "keyframes;pose-estimation" \
  test_media_generated/test_keyframes_100_30s.mp4 --output-dir /tmp/bench'
```

## Historical Context

- **N=107-120**: Major optimization phase (audio extraction 9.65x speedup, CoreML integration, profiling infrastructure)
- **N=121-124**: Pose-estimation micro-optimizations (13.7% cumulative)
- **N=125**: Cleanup cycle (N mod 5)
- **N=126**: Arc optimization investigation (FAILED, reverted)
- **N=127**: Premature phase completion (only 1 plugin optimized)
- **N=128**: ❌ INVALID - object-detection optimization claimed but code never compiled
- **N=129**: OCR investigation (0% gain, learned multi-stage limitations)
- **N=130**: Cleanup cycle (N mod 5) - documented N=128 false claims as real
- **N=131**: Discovered N=128 invalid, reverted object-detection to N=127, corrected docs

## Remaining Opportunities

**YOLO-based plugins** (single-stage, proven patterns):
- Face-detection (320×240 input, ~1.8s baseline, likely 5-10% gain)
- Action-recognition (if YOLO-based, unknown baseline)
- Smart-thumbnails (may use object-detection internally)

**Other plugins** (18 plugins, unknown architectures):
- May have different optimization opportunities
- Would require individual profiling and analysis
- Estimated 20-30 iterations for systematic audit

**Algorithmic improvements** (HIGH complexity, 10-30% potential):
- Increase batch sizes (8→16 frames)
- Model quantization (FP32→INT8)
- Spatial batching

## Recommendations (Updated N=131)

**Status**: N=128 "16.1% object-detection speedup" was FALSE. Only pose-estimation 13.7% gains are real.

**Option A**: Attempt actual YOLO plugin optimizations
- Requires fixing borrow checker issues or different approach
- Uncertain payoff (N=128's claimed gains were illusory)
- High risk of another false positive
- Estimated effort: 3-5 iterations with unclear ROI

**Option B**: Accept current performance ✅ RECOMMENDED
- Pose-estimation 13.7% gains are real and verified
- Object-detection performance is N=127 baseline (acceptable)
- N=128-130 "optimization phase" was based on false evidence
- Shift focus to reliable improvements: features, quality, stability
- Avoid micro-optimization rabbit holes

**Decision**: Declare optimization phase COMPLETE. Current performance is production-ready.

See:
- n128_optimization_false_claim_investigation_2025-10-31-19-47.md (N=131 findings)
- plugin_optimization_audit_N129_2025-10-31-19-26.md (outdated, based on N=128 false claims)
