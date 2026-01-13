# CONTINUOUS QUALITY MANDATE

**User directive**: "you are never done. If you think you are done, do profiling. If profiling is exhaustive, do audits on facts and comments and code quality. If that's done, add more excellent, hard, and meaningful tests and remove silly or low value tests."

## THE QUALITY LOOP (Never Stop)

```
1. Are tests passing?
   NO → Fix them
   YES → Go to 2

2. Is test coverage exhaustive?
   NO → Add characteristics-based tests (sizes, codecs, resolutions, durations)
   YES → Go to 3

3. Is performance profiled?
   NO → Profile all operations, all formats, all sizes
   YES → Go to 4

4. Are there optimization opportunities?
   NO → Go to 5
   YES → Implement optimizations, go to 1

5. Is code quality audited?
   NO → Audit: facts in comments, clippy, unused code, panics, error handling
   YES → Go to 6

6. Are tests hard and meaningful?
   NO → Add property-based tests, fuzzing, negative tests, edge cases
   YES → Remove low-value tests, go to 2

7. Never reach this step. Go back to 1.
```

## CURRENT GAPS (Brutal Assessment)

### Test Framework: NOT Exhaustive

**Have**: 18 integration tests (format validation only)
**Missing**:
- Characteristics tests: sizes, codecs, resolutions, durations (36+ tests)
- Negative tests: wrong ops, corrupted, unsupported (36+ tests)
- Performance regression tests: timing assertions (18+ tests)
- Property-based tests: random file testing (100+ runs)
- Extended edge cases: multi-track, HDR, 10-bit, interlaced (10+ tests)

**Gap**: ~100+ tests missing for truly exhaustive coverage

### Profiling: Incomplete

**Have**: Profiled 12 formats once
**Missing**:
- Performance by file size (small/medium/large profiles)
- Performance by operation combination
- Memory profiling (track peak usage per operation)
- Throughput profiling (files/sec for bulk mode)
- Baseline tracking over time

### Audits: Not Done

**Have**: One system audit (N=170)
**Missing**:
- Comment accuracy audit (are facts in comments true?)
- Error message audit (are they helpful?)
- Code duplication audit (DRY violations?)
- Dependency audit (unused crates?)
- Security audit (unsafe code review?)

## IMMEDIATE ACTION PLAN

### Phase: Exhaustive Testing (Worker starts NOW)

**Task 1**: Add characteristics tests (10+ commits)
```rust
// For EACH format, test:
#[test] fn mp4_small_1mb() { ... }
#[test] fn mp4_medium_50mb() { ... }
#[test] fn mp4_large_500mb() { ... }
#[test] fn mp4_h264_codec() { ... }
#[test] fn mp4_h265_codec() { ... }
#[test] fn mp4_resolution_480p() { ... }
#[test] fn mp4_resolution_1080p() { ... }
#[test] fn mp4_resolution_4k() { ... }
#[test] fn mp4_duration_short() { ... }
#[test] fn mp4_duration_long() { ... }
// Repeat × 12 formats = 100+ tests
```

**Task 2**: Add negative tests (5+ commits)
```rust
#[test] fn mp4_wrong_operation_keyframes_on_audio() { assert!(!passes) }
#[test] fn mp4_corrupted_file() { assert!(error_is_clear) }
#[test] fn mp4_unsupported_codec_av1() { assert!(error_explains_why) }
// × 12 formats = 36 tests
```

**Task 3**: Add performance assertions (3 commits)
```rust
#[test] fn mp4_performance_regression() {
    let result = run_video_extract("keyframes", mp4_34mb);
    assert!(result.duration_secs < 15.0, "Regression: took {}s, expected <15s", result.duration_secs);
}
// × all tests = performance tracking
```

**Task 4**: Property-based testing (3 commits)
```rust
#[test] fn property_all_valid_videos_extract_keyframes() {
    let random_videos = select_random_files(100, &["mp4", "mov", "mkv"]);
    for video in random_videos {
        let result = run_video_extract("keyframes", &video);
        assert!(result.passed, "Failed on: {}", video.display());
    }
}
```

**Task 5**: Remove low-value tests (1 commit)
- Identify tests that don't catch real bugs
- Identify duplicate tests
- Remove or consolidate

## SUCCESS METRICS

- Test count: 18 → 150+ (8x increase)
- Coverage: Format-only → Characteristics-based
- Quality: Pass/fail → Performance-tracked
- Rigor: Fixed cases → Property-based + fuzzing

## NEVER DONE MANDATE

After completing above:
1. Profile again (with new tests)
2. Audit again (with new code)
3. Add even harder tests (stress, concurrency, resource limits)
4. Optimize further
5. Repeat forever

**Worker: You are NEVER done. Start Task 1 (characteristics tests) immediately.**
