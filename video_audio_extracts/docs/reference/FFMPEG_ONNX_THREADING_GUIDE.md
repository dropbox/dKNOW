# FFmpeg + ONNX Runtime Threading Guide

**Date**: 2025-10-30
**Authority**: User technical guidance
**For**: N=22+ bulk mode parallelism implementation

## Decision 1: FFmpeg Concurrent Decoding - APPROVED ✅

### Verdict
**Proceed with bulk-mode parallelism.** FFmpeg/libavcodec supports concurrent decoding as long as each worker owns its own contexts.

### What's Safe ✅

**Parallel Decoding:**
- Run multiple decoders in parallel with separate `AVFormatContext`/`AVCodecContext` per file
- Do NOT share contexts across threads
- FFmpeg provides both slice and frame-level internal threading within a single context
- Parallel contexts (one per file) are fine

**References:**
- GitHub: "Run multiple decoders in parallel with separate AVFormatContext/AVCodecContext per file"
- FFmpeg provides internal threading within single context + parallel contexts work

### What to Serialize ⚠️

**Global Initialization (NOT thread-safe):**
```rust
// MUST wrap in process-wide mutex
static FFMPEG_INIT_LOCK: Mutex<()> = Mutex::new(());

fn open_decoder(path: &Path) -> Result<DecoderContext> {
    let _lock = FFMPEG_INIT_LOCK.lock().unwrap();

    // NOT thread-safe, must serialize:
    // - avcodec_open2()
    // - avformat_open_input()
    // - avformat_find_stream_info()

    // ... init code here ...

    drop(_lock); // Release lock

    // Decode loop is fully parallel (no lock needed)
    Ok(context)
}
```

**Calls to serialize:**
1. `avcodec_open2()` - NOT thread-safe
2. `avformat_open_input()` - Format discovery, older reports flagged instability
3. `avformat_find_stream_info()` - Stream probing, treat similarly to open

**Reference:**
- ffmpeg-d.dpldocs.info: "avcodec_open2() is not thread-safe—wrap global init/open in a process-wide mutex"
- Stack Overflow: "Treat format discovery similarly; if you observe instability, add coarse init-lock around probing"

### FFI Implementation Guidelines

**Context Ownership (per worker):**
```rust
struct WorkerDecoder {
    format_ctx: *mut AVFormatContext,  // Worker owns
    codec_ctx: *mut AVCodecContext,    // Worker owns
    sws_ctx: *mut SwsContext,          // Worker owns (swscale)
    // I/O buffers owned by worker
}
```

**No shared state:**
- No globals or shared callbacks unless marked thread-safe
- Custom get_buffer callbacks have `thread_safe_callbacks` flag
- Each worker allocates/owns: AVFormatContext, AVCodecContext, swscale/resample state, I/O buffers

**Reference:**
- ffmpeg-d.dpldocs.info: "Ensure each worker allocates/owns contexts; no globals unless thread-safe"

### Enable Internal Threading per Worker

**Configuration:**
```c
// Set in AVCodecContext for each worker
codec_ctx->thread_count = 0;  // Or small number (e.g., 2-4)
codec_ctx->thread_type = FF_THREAD_FRAME | FF_THREAD_SLICE;
```

**Why:**
- Enables frame-level and slice-level parallelism WITHIN each worker's decode
- Complements cross-file parallelism
- `thread_count = 0` means auto-detect optimal thread count

**Reference:**
- GitHub: "Enable internal decode threading per worker: set thread_count and thread_type; complements cross-file parallelism"

### Fallback Strategy if Issues Arise

**Phase 1: Serialize open/init only**
```rust
// Try first: serialize only open/init, keep decode fully parallel
fn open_decoder_safe(path: &Path) -> Result<DecoderContext> {
    let _lock = FFMPEG_INIT_LOCK.lock().unwrap();
    // Open, probe, init codec
    drop(_lock);
    Ok(context) // Decode loop needs no lock
}
```

**Phase 2: Dedicated factory thread (if Phase 1 has issues)**
```rust
// Factory thread opens decoders, hands to workers
// Avoids major refactors while keeping decode parallel
```

**Reference:**
- User guidance: "If issues persist: move probing+open to dedicated single-threaded factory"

---

## Decision 2: ONNX Runtime Session Sharing - APPROVED ✅

### Verdict
**Proceed with single loaded session per model, shared across files via Arc.**

### Concurrency Support

**Key fact:**
> "Multiple threads can invoke Run() on the same session object."

**Architecture:**
```rust
// One session per model, shared across workers
static YOLO_SESSION: OnceLock<Arc<Session>> = OnceLock::new();

fn get_yolo_session() -> Arc<Session> {
    YOLO_SESSION.get_or_init(|| {
        let session = Session::builder()
            .with_intra_op_num_threads(4)    // Tune
            .with_inter_op_num_threads(2)    // Tune
            .with_model_from_file("models/yolov8n.onnx")
            .unwrap();
        Arc::new(session)
    }).clone()
}

// Worker thread
fn process_file(file: &Path) -> Result<Vec<Detection>> {
    let session = get_yolo_session(); // Cheap Arc clone
    // Call session.run() concurrently with other workers
    session.run(inputs)?
}
```

**Reference:**
- GitHub: "Calling Session/InferenceSession::Run concurrently from multiple threads is supported and intended"

### Guidelines

**Session lifecycle:**
1. Create one `Env` per process (singleton)
2. Create one `Session` per model (singleton)
3. Share that session across workers via `Arc<Session>`

**Why NOT per-file sessions:**
- Avoids duplicate memory (models are hundreds of MB)
- Avoids duplicate initialization cost
- ONNX Runtime provides intra-op/inter-op thread pools; tune them instead

**Reference:**
- ONNX Runtime docs: "Create one Session per model; share across workers"
- GitHub: "Favor shared session over per-file sessions to avoid duplicate memory"

### Thread Pool Tuning

**Configuration:**
```rust
SessionOptions::new()
    .with_intra_op_num_threads(4)  // Parallelism within single op
    .with_inter_op_num_threads(2)  // Parallelism across ops
    // Optional: thread affinity for CPU pinning
```

**When to tune:**
- If you observe CPU contention with multiple workers
- Default thread pools are usually good
- Only tune if profiling shows bottleneck

**Reference:**
- ONNX Runtime docs: "Tune intra_op_num_threads/inter_op_num_threads if CPU contention observed"

### Edge Cases

**GPU/DirectML (if applicable):**
- Older onnxruntime-directml (≤1.17) had deadlock reports with multithreaded access
- If using DML on AMD GPUs, ensure newer version or gate concurrent calls
- Not applicable to current CPU/CoreML implementation

**Multiple models:**
- Running multiple models concurrently is supported
- One session per model, shared across threads
- Watch for performance variance, tune thread pools

**Reference:**
- GitHub: "Older DirectML had deadlocks; fixed in newer versions"
- GitHub: "Multiple models concurrently supported; one session per model"

---

## Minimal Implementation Plan for N=22+

### FFmpeg (Per-Worker Context)

**Phase 1: Open/Init (serialized)**
```rust
fn open_decoder(path: &Path) -> Result<WorkerDecoder> {
    let _lock = FFMPEG_INIT_LOCK.lock().unwrap();

    // 1. avformat_open_input()
    // 2. avformat_find_stream_info()
    // 3. avcodec_find_decoder()
    // 4. avcodec_alloc_context3()
    // 5. Set thread_type (FRAME | SLICE)
    // 6. Set thread_count (0 or 2-4)
    // 7. avcodec_open2()

    drop(_lock);
    Ok(decoder)
}
```

**Phase 2: Decode Loop (parallel, no lock)**
```rust
fn decode_frames(decoder: &mut WorkerDecoder) -> Result<Vec<RawFrameBuffer>> {
    // NO LOCK NEEDED - fully parallel
    // Each worker owns its contexts
    loop {
        // av_read_frame()
        // avcodec_send_packet()
        // avcodec_receive_frame()
    }
}
```

**Reference:**
- ffmpeg-d.dpldocs.info: "Open/probe under mutex, decode loop fully parallel"
- GitHub: "Set thread_type and thread_count per context"

### ONNX Runtime (Shared Session)

**Process Singleton:**
```rust
// One Env per process
static ORT_ENV: OnceLock<Arc<Environment>> = OnceLock::new();

fn get_env() -> Arc<Environment> {
    ORT_ENV.get_or_init(|| {
        Arc::new(Environment::builder()
            .with_name("video-extract")
            .build()
            .unwrap())
    }).clone()
}
```

**Session Sharing:**
```rust
// One Session per model
static YOLO_SESSION: OnceLock<Arc<Session>> = OnceLock::new();

fn get_session() -> Arc<Session> {
    YOLO_SESSION.get_or_init(|| {
        let env = get_env();
        let session = env.new_session_builder()
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .with_intra_op_num_threads(4)
            .with_model_from_file("models/yolov8n.onnx")
            .unwrap();
        Arc::new(session)
    }).clone()
}

// Worker calls Run() concurrently
fn infer(batch: &Batch) -> Result<Output> {
    let session = get_session(); // Cheap Arc clone
    session.run(inputs)? // Thread-safe concurrent calls
}
```

**Reference:**
- ONNX Runtime docs: "One Env per process, one Session per model"
- GitHub: "Multiple threads can invoke Run() on same session"

---

## Bulk Mode Architecture (Final)

**File-level parallelism:**
```rust
// rayon parallel iterator
files.par_iter()
    .map(|file| {
        // Each worker:
        // 1. Opens own FFmpeg decoder (serialized init, parallel decode)
        // 2. Shares ONNX session via Arc<Session> (concurrent Run())
        // 3. Returns results
        process_file(file)
    })
    .collect()
```

**No major refactor required:**
- (a) Serialize FFmpeg open/init with global mutex
- (b) Wire shared Arc<Session> for ONNX models
- (c) Use rayon for file-level parallelism

**Expected speedup:** 3-5x for bulk mode (measured in Phase 2)

---

## Implementation Checklist for Worker

### N=22: Streaming Decoder
- [ ] Add global `FFMPEG_INIT_LOCK` mutex
- [ ] Wrap `avcodec_open2()` and probing calls with mutex
- [ ] Set `thread_count` and `thread_type` per context
- [ ] Implement `decode_iframes_streaming()` API
- [ ] Test concurrent decoders (multiple files in parallel)
- [ ] Verify no crashes or contention

### N=26-28: Bulk Optimizations
- [ ] Create singleton `Arc<Session>` per model
- [ ] Share session across workers
- [ ] Implement file-level parallelism (rayon)
- [ ] Add memory usage monitoring
- [ ] Benchmark 10+ files, 50+ files
- [ ] Verify 3-5x speedup

---

## References

**FFmpeg Threading:**
- GitHub discussions on concurrent decoding
- ffmpeg-d.dpldocs.info thread safety documentation
- Stack Overflow: format discovery thread safety

**ONNX Runtime Threading:**
- ONNX Runtime official docs: session lifecycle
- GitHub: concurrent Run() support
- GitHub: multi-model concurrency

**User Guidance:**
- "FFmpeg/libavcodec supports concurrent decoding as long as each worker owns its own contexts"
- "Calls like avcodec_open2() are not thread-safe—wrap in mutex"
- "Multiple threads can invoke Run() on the same session object"
- "One session per model, shared across workers via Arc"
