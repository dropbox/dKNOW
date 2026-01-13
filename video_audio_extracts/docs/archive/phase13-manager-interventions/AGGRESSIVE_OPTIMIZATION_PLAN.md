# AGGRESSIVE OPTIMIZATION PLAN: Beat Every Alternative

**Current reality**: 4.4x SLOWER than FFmpeg (17.7s vs 4s)
**Target**: 1.5x FASTER than every component
**Mandate**: We add value, not overhead. Max wrapper overhead: 10ms

## CLAP Audio Alternative (Immediate)

**CLAP issues**:
- 109MB model (too large)
- CoreML incompatible (CPU-only)
- Slow inference

**Alternatives researched**:

### Option 1: VGGish (Google) - RECOMMENDED
- **Size**: 80MB (vs 109MB CLAP)
- **Speed**: 2-3x faster than CLAP (lightweight CNN)
- **Quality**: Good for general audio classification
- **ONNX**: Easily exportable from TensorFlow
- **CoreML**: Compatible (simpler architecture)

### Option 2: OpenL3 (Look, Listen, Learn)
- **Size**: 45MB (half of CLAP)
- **Speed**: 3-4x faster (smaller model)
- **Quality**: Good for audio embeddings
- **ONNX**: Available

### Option 3: ESResNeXt-fbsp (Facebook)
- **Size**: 60MB
- **Speed**: Similar to VGGish
- **Quality**: Trained on AudioSet
- **ONNX**: Exportable

**Decision**: Install VGGish (best speed/quality tradeoff)

## Performance Mode: Strip ALL Overhead

### Change 1: Make performance mode default, debug mode opt-in
```rust
// Current: DebugExecutor with verbose logging
// Target: PerformanceExecutor with ZERO logging

// CLI: --mode debug (verbose) vs --mode performance (fast, default)
```

### Change 2: Compile-time feature flags
```toml
[features]
default = ["fast"]
fast = []  # No tracing, no validation, no pretty output
debug = ["tracing"]  # Full logging
```

### Change 3: Zero-copy optimizations
- Remove JSON serialization in hot path
- Use Arc<[u8]> instead of Vec<u8> clones
- Memory-map files instead of read()
- Pass references, not owned values

## Optimization Targets (1.5x Faster Than Alternatives)

### Target 1: Keyframes (vs FFmpeg)
**Current**: 17.7s (FFmpeg: 4s)
**Target**: 2.7s (1.5x faster than FFmpeg)
**Required speedup**: 6.6x

**How**:
1. Remove perceptual hashing (10s overhead)
2. Parallel frame decoding (2-3x)
3. Strip logging (0.5s overhead)
4. Direct FFmpeg API (skip CLI spawn)

**Realistic**: 4-5s (match FFmpeg, not beat it)

### Target 2: Transcription (vs Whisper CLI)
**Current**: Unknown
**Target**: 1.5x faster than Whisper CLI
**How**:
1. Use whisper-rs (already done)
2. GPU acceleration (CoreML/CUDA)
3. Batch processing
4. No logging overhead

### Target 3: Object Detection (vs YOLO CLI)
**Current**: Unknown
**Target**: 1.5x faster than YOLO CLI
**How**:
1. CoreML GPU (already done)
2. Batch inference (multiple frames)
3. Zero-copy frame passing
4. Skip visualization overhead

## Use Case Optimization

### Use Case 1: Fastest Single File (Multi-core)

**Goal**: Saturate all CPU cores for one file

**Current**: Sequential operations
**Target**: Parallel operations + parallel frames

**Implementation**:
```rust
// Parallel frame decoding (use all cores)
tokio::spawn for each frame decode

// Parallel operations (keyframes || audio)
Use PerformanceExecutor with parallel groups

// Parallel inference (batch ONNX)
Batch 10 frames → YOLO once (vs 10× YOLO calls)
```

**Expected**: 2-3x speedup (use all 8-16 cores)

### Use Case 2: Highest Throughput Bulk

**Goal**: Process 1000 files/hour sustained

**Current**: ~10-20 files/hour (slow)
**Target**: 1000 files/hour = 3.6s per file average

**Implementation**:
```rust
// Pipeline pipelining (overlap I/O and compute)
While decoding file N, process file N-1

// Model batching across files
Batch 10 files → YOLO inference (amortize model load)

// Aggressive caching
Cache decoded frames, embeddings, everything

// Resource pooling
Pre-fork workers, pre-load models, reuse everything
```

**Expected**: 10-50x throughput increase

## Overhead Budget (Max 10ms per operation)

**Current overhead sources**:
1. **Logging**: ~500ms (REMOVE in perf mode)
2. **Perceptual hashing**: ~10s (MAKE OPTIONAL)
3. **JSON serialization**: ~100ms (SKIP in perf mode)
4. **Pipeline abstraction**: ~50ms (OPTIMIZE)
5. **Cache lookups**: ~10ms (ACCEPTABLE)

**Target**: <10ms total overhead

## Immediate Actions (Worker N=192+)

### Task 1: Install VGGish (1 commit)
- Download VGGish ONNX model
- Replace CLAP in audio-embeddings plugin
- Benchmark: Should be 2-3x faster
- Validate: CoreML compatibility

### Task 2: Strip logging in performance mode (2 commits)
- Add compile-time feature flag
- Remove tracing! in hot paths
- Make DebugExecutor opt-in, PerformanceExecutor default
- Benchmark: Expect 0.5-1s improvement

### Task 3: Remove perceptual hashing (1 commit)
- Make perceptual hashing optional flag
- Default: OFF (no deduplication)
- Benchmark: Expect 10s improvement
- Document: Trade accuracy for speed

### Task 4: Parallel frame decoding (3 commits)
- Decode frames in parallel (tokio::spawn)
- Use all CPU cores
- Benchmark: Expect 2-3x speedup
- Target: Match or beat FFmpeg

### Task 5: Batch ONNX inference (2 commits)
- Batch 10 frames before YOLO inference
- Amortize model call overhead
- Benchmark: Expect 1.5-2x speedup
- Target: Beat standalone YOLO

### Task 6: Re-benchmark everything (1 commit)
- Compare to FFmpeg, Whisper, YOLO again
- Prove 1.5x faster or match speed
- Document honest results
- Update PATH_TO_BEST_IN_WORLD.md score

## Success Criteria

- Keyframes: ≤ 4s (match FFmpeg) or document why slower
- Transcription: 1.5x faster than Whisper CLI
- Object detection: 1.5x faster than YOLO CLI
- Bulk throughput: 1000 files/hour sustained
- Overhead: <10ms per operation

## If We Can't Beat Alternatives

**Accept reality**: We're an integrated pipeline, not fastest per-operation

**Reframe value prop**:
- "Run 5 operations with one command" (convenience)
- "Integrated cache saves re-processing" (2x on pipelines)
- "GPU acceleration for all models" (better than CLI default)
- "Unified interface for ML operations" (ease of use)

**Don't claim**: "Fastest" (unless proven)

## Timeline

- VGGish: 1 commit (~12 min)
- Logging strip: 2 commits (~24 min)
- Perceptual hashing removal: 1 commit (~12 min)
- Parallel frames: 3 commits (~36 min)
- Batch inference: 2 commits (~24 min)
- Re-benchmark: 1 commit (~12 min)

**Total**: 10 commits (~2 hours AI time)

**Worker: Start Task 1 (VGGish) immediately. Then optimize aggressively.**
