# MANAGER GUIDANCE FOR WORKER - After N=44

**Date**: 2025-10-28
**Current State**: N=44 complete
**Manager Assessment**: Worker made excellent progress but left one critical issue

## Python Elimination Status: ✅ COMPLETE

Worker successfully eliminated 100% of Python dependencies through N=38-44:
- Audio embeddings: Pure Rust mel-spectrogram
- Speaker diarization: WebRTC VAD + WeSpeaker ONNX + K-means
- All verified: Zero pyo3 dependencies, all tests passing

**This work is EXCELLENT and production-ready.**

## Critical Issue Found: Bulk API Uses Broken Method

### The Problem

**File**: `crates/api-server/src/handlers.rs:246`
**Current code**:
```rust
match orchestrator.execute_bulk_staged(graphs).await {
```

**Issue**: `execute_bulk_staged()` is marked `#[deprecated]` with note:
```
"Has critical hang bug - use execute_bulk() instead"
```

See: `crates/orchestrator/src/lib.rs:1486`

### Evidence

Bulk API benchmark hangs:
- Submitted 10 jobs
- 9 completed, 1 hung forever
- After server restart: 0/10 complete, all stuck

Integration test `test_bulk_processing` PASSES because it uses realtime graph, not true bulk.

### The Fix

**Change line 246** in `crates/api-server/src/handlers.rs`:
```rust
- match orchestrator.execute_bulk_staged(graphs).await {
+ match orchestrator.execute_bulk(graphs).await {
```

But `execute_bulk()` method doesn't exist yet!

### Implementation Needed

**Worker must**:
1. Implement `execute_bulk()` in orchestrator (simple parallel execution)
2. Update handler to call `execute_bulk()` instead of `execute_bulk_staged()`
3. Test with 10-file benchmark
4. Run 100-file benchmark

**Simple implementation** (in `crates/orchestrator/src/lib.rs`):
```rust
pub async fn execute_bulk(
    &self,
    graphs: Vec<TaskGraph>,
) -> Result<Vec<TaskGraph>, ProcessingError> {
    // Execute each graph sequentially (simple, reliable)
    let mut results = Vec::new();
    for graph in graphs {
        results.push(self.execute(graph).await);
    }

    // Return results
    let completed: Result<Vec<_>, _> = results.into_iter().collect();
    completed
}
```

This is SIMPLE and RELIABLE. Not optimized for throughput but won't hang.

## Worker Instructions

**Next AI (N=45)**:

1. Add `execute_bulk()` method to orchestrator (10 lines)
2. Change handler to use it (1 line)
3. Test: `cargo test -p video-audio-api-server test_bulk_processing`
4. Run benchmark: `python3 /tmp/benchmark_bulk_corrected.py 100`
5. Commit with results

**Do NOT** try to fix `execute_bulk_staged()` - it's complex and has known issues.
**Do NOT** optimize yet - get it working first.

## System Status

**Python elimination**: ✅ DONE - Ready to merge
**Diarization**: ✅ PRODUCTION-READY (8x real-time)
**Bulk API**: ❌ BROKEN - Uses deprecated hanging method

**Priority**: Fix bulk API (1 AI commit), then branch is ready to merge.

## Assessment

Worker did EXCELLENT work on Python elimination (N=38-44).
One oversight: Bulk API still uses broken method.
Simple fix needed.

---
**Manager**: Claude (Management AI)
**For**: Next Worker AI (N=45)
