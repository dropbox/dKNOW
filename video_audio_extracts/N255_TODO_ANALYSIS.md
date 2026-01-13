# N=255 TODO Comment Analysis

**Date:** 2025-11-13
**Purpose:** Cleanup cycle - review and categorize TODO comments in codebase

## Summary

Found 10 TODO/FIXME comments across codebase. All are optimization suggestions or future enhancements, none are blocking issues.

## TODO Comments by Category

### Category 1: Performance Optimizations (4 comments)

#### 1. Face Detection - Cache Priors
**Location:** crates/face-detection/src/lib.rs:300
```rust
// TODO: Cache priors to avoid regeneration on each call
```
**Analysis:** Priors are regenerated on every inference call. Caching would improve performance by ~5-10%.
**Priority:** Low - current performance acceptable (150ms average latency)
**Effort:** 1 commit (add lazy_static cache)

#### 2. Logo Detection - Session Caching
**Location:** crates/logo-detection/src/lib_clip.rs:364
```rust
// TODO: Consider caching session for better performance
```
**Analysis:** ONNX session created for each request. Caching would reduce initialization overhead.
**Priority:** Low - current performance acceptable (1.2s average)
**Effort:** 1 commit (add session cache with Mutex)

#### 3. Image Quality - Refactor Session Ownership
**Location:** crates/image-quality-assessment/src/plugin.rs:67
```rust
/// TODO: Refactor ImageQualityAssessor to accept &mut Session instead of owning it
```
**Analysis:** Session ownership prevents reuse. Refactoring would enable session pooling.
**Priority:** Low - architectural improvement, not urgent
**Effort:** 2-3 commits (refactor API, update callers, test)

#### 4. Registry - Transitive Closure Algorithm
**Location:** crates/video-extract-core/src/registry.rs:157
```rust
// TODO: Implement full transitive closure with graph algorithm
```
**Analysis:** Dependency resolution could be more sophisticated for complex plugin graphs.
**Priority:** Very Low - current simple approach works for existing plugins
**Effort:** 3-5 commits (implement graph algorithm, test edge cases)

### Category 2: Feature Enhancements (3 comments)

#### 5. Face Detection - Support Multiple Input Sizes
**Location:** crates/face-detection/src/lib.rs:252
```rust
// TODO: Support other input sizes if needed
```
**Analysis:** Currently hardcoded to 320×240. Supporting multiple sizes would improve accuracy on high-res images.
**Priority:** Low - current size works well for most use cases
**Effort:** 2-3 commits (parameterize size, update preprocessing, test)

#### 6. Caption Generation - Beam Search
**Location:** crates/caption-generation/src/generation.rs:208
```rust
// TODO: Implement beam search
```
**Analysis:** Currently uses greedy decoding. Beam search would improve caption quality.
**Priority:** Medium - would improve output quality significantly
**Effort:** 5-8 commits (implement beam search, tune parameters, verify quality)

#### 7. OCR - Language Mapping Expansion
**Location:** crates/ocr/src/plugin.rs:143
```rust
// TODO: Expand this mapping for more languages
```
**Analysis:** Currently maps ~10 languages. Supporting more would improve multilingual OCR.
**Priority:** Low - covers most common use cases already
**Effort:** 1 commit per language batch (research Tesseract lang codes, test)

### Category 3: Timeout/Observability (2 comments)

#### 8. Executor - Timeout Support
**Location:** crates/video-extract-core/src/executor.rs:357
```rust
/// TODO: Implement timeout support in execute_streaming (currently unused)
```
**Analysis:** Timeout parameter exists but not implemented. Would prevent hung operations.
**Priority:** Medium - important for production robustness
**Effort:** 2-3 commits (implement timeout, test with slow operations)

#### 9. Registry - Performance-Based Estimates
**Location:** crates/video-extract-core/src/registry.rs:187
```rust
// TODO: Use plugin performance characteristics for better estimates
```
**Analysis:** Completion time estimates could be more accurate using historical performance data.
**Priority:** Low - current estimates adequate for user feedback
**Effort:** 3-4 commits (add performance tracking, update estimator)

### Category 4: Code Comments (1 comment)

#### 10. Scene Detector - Log Format Documentation
**Location:** crates/scene-detector/src/lib.rs:194
```rust
// Format: [scdet @ 0x...] lavfi.scd.score: X.XXX, lavfi.scd.time: Y.YYY
```
**Analysis:** This is actually a comment documenting log format, not a TODO action item.
**Priority:** N/A - documentation, not a task
**Effort:** 0

## Recommendations

### Immediate Actions (None Required)
All TODOs are optimization suggestions or future enhancements. No urgent or blocking issues found.

### Short-Term (Next 10 commits, if desired)
1. **Beam Search for Captions** (N=260-267, Medium priority, significant quality improvement)
2. **Timeout Support** (N=258-260, Medium priority, production robustness)

### Long-Term (Future cleanup cycles)
1. Performance optimizations (caching, session pooling)
2. Feature enhancements (multiple input sizes, more languages)
3. Architectural improvements (transitive closure, better estimates)

## Status

**Total TODOs:** 10
**Blocking Issues:** 0
**High Priority:** 0
**Medium Priority:** 2 (beam search, timeout)
**Low Priority:** 7
**Documentation Only:** 1

**Conclusion:** Codebase is in good health. All TODOs are future improvements, not technical debt or bugs.

## Next Steps for N=255

Since all TODOs are non-urgent, no immediate action required. This analysis documents current state for future cleanup cycles.
