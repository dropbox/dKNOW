# Worker Handoff: V2 Architecture Implementation
**For**: Next Worker AI (N=85+)
**From**: Architecture design session N=84
**Date**: 2025-10-28
**Branch**: build-video-audio-extracts

## Context: What Just Happened

User requested transformation of current REST API into a **high-performance plugin-based library and CLI tool** suitable for:
1. **Laptop batch processing** - Single binary, process hundreds of files locally
2. **Dropbox cloud service** - High-volume production service (10,000+ files/hour per machine)

We researched Dropbox's Riviera plugin architecture (~/src/worktrees/riviera1) and designed V2 architecture inspired by its battle-tested patterns (millions of conversions/day).

**Design document**: `ARCHITECTURE_V2_DESIGN.md` (comprehensive, read it fully)

---

## Your Mission: Implement Phase 1 - Core Architecture

### Goal

Create the foundational plugin system that enables:
- Plugin-based extensibility (add new operations without changing orchestrator)
- Three execution modes (Debug, Performance, Bulk)
- Pipeline composition (automatic chaining: video → audio → transcription)
- Library-first design (CLI wraps library, REST server wraps library)

### Success Criteria

By end of Phase 1, you should have:
1. ✅ `crates/video-extract-core/` with Plugin trait, Registry, OutputSpec
2. ✅ One migrated plugin (transcription) working through new interface
3. ✅ Debug executor that can run: `video-extract debug video.mp4 --ops transcription`
4. ✅ All existing tests still passing
5. ✅ Zero regressions in performance

---

## Implementation Roadmap

### Step 1: Create Core Crate (1 commit)

```bash
mkdir -p crates/video-extract-core/src
cd crates/video-extract-core
```

**Files to create:**

```
crates/video-extract-core/
├── Cargo.toml
└── src/
    ├── lib.rs           # Public exports
    ├── plugin.rs        # Plugin trait definition
    ├── registry.rs      # Plugin lookup and routing
    ├── spec.rs          # OutputSpec and Operation types
    ├── pipeline.rs      # Pipeline and PipelineStage
    ├── context.rs       # Execution context
    └── error.rs         # Error types
```

**Key types to implement:**

```rust
// plugin.rs
#[async_trait]
pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;
    fn config(&self) -> &PluginConfig;
    fn supports_input(&self, input_type: &str) -> bool;
    fn produces_output(&self, output_type: &str) -> bool;

    async fn execute(
        &self,
        ctx: &Context,
        request: &PluginRequest,
    ) -> Result<PluginResponse>;
}

// spec.rs
pub struct OutputSpec {
    pub sources: Vec<OutputSpec>,
    pub operation: Operation,
}

pub enum Operation {
    DataSource(DataSource),
    Audio { sample_rate: u32, channels: u8 },
    Transcription { language: Option<String>, model: WhisperModel },
    ObjectDetection { model: ObjectDetectionModel, confidence: f32 },
    // ... all operations from ARCHITECTURE_V2_DESIGN.md
}

// registry.rs
pub struct Registry {
    plugins: HashMap<String, Arc<dyn Plugin>>,
    by_output: HashMap<String, Vec<Arc<dyn Plugin>>>,
}

impl Registry {
    pub fn lookup(&self, spec: &OutputSpec) -> Result<Pipeline> {
        // Implement recursive lookup algorithm from design doc
    }
}
```

### Step 2: Migrate Transcription Plugin (1 commit)

Create plugin wrapper around existing `crates/transcription/`:

```
crates/video-extract-plugins/
└── transcription/
    ├── Cargo.toml
    ├── plugin.yaml      # Plugin manifest
    └── src/
        └── lib.rs       # Plugin implementation wrapping existing code
```

**Plugin implementation:**

```rust
use video_extract_core::{Plugin, PluginConfig, Context, PluginRequest, PluginResponse};
use transcription::{Transcriber, TranscriptionConfig};

pub struct TranscriptionPlugin {
    config: PluginConfig,
}

#[async_trait]
impl Plugin for TranscriptionPlugin {
    fn name(&self) -> &str { "transcription" }

    fn supports_input(&self, input_type: &str) -> bool {
        matches!(input_type, "Audio" | "wav" | "mp3" | "flac" | "m4a")
    }

    fn produces_output(&self, output_type: &str) -> bool {
        output_type == "Transcription"
    }

    async fn execute(&self, ctx: &Context, req: &PluginRequest) -> Result<PluginResponse> {
        // 1. Extract audio file path from request
        let audio_path = req.input_data.as_audio_file()?;

        // 2. Get transcription config from operation spec
        let (language, model) = match &req.operation {
            Operation::Transcription { language, model } => (language, model),
            _ => return Err(PluginError::InvalidOperation),
        };

        // 3. Call existing transcription code
        let model_path = PathBuf::from("models/ggml-base.en.bin");
        let config = TranscriptionConfig {
            language: language.clone(),
            // ... map from Operation to TranscriptionConfig
        };

        let transcriber = Transcriber::new(model_path, config)?;
        let transcript = transcriber.transcribe(&audio_path)?;

        // 4. Return wrapped result
        Ok(PluginResponse {
            data: PluginData::Transcription(transcript),
            format: "Transcription".to_string(),
            metadata: PluginMetadata::default(),
        })
    }
}
```

### Step 3: Debug Executor (1 commit)

Create simplest execution mode to validate entire architecture:

```rust
// crates/video-extract-core/src/executor/debug.rs
pub struct DebugExecutor {
    registry: Arc<Registry>,
    output_dir: PathBuf,
}

impl DebugExecutor {
    pub async fn execute(&self, request: ExecuteRequest) -> Result<DebugResult> {
        info!("🔍 Building pipeline...");
        let pipeline = self.registry.lookup(&request.output_spec)?;

        info!("📋 Pipeline stages:");
        for (i, stage) in pipeline.stages.iter().enumerate() {
            info!("  {}. {} ({} → {})", i+1, stage.plugin.name(),
                  stage.input_type, stage.output_type);
        }

        let mut results = Vec::new();
        for (i, stage) in pipeline.stages.iter().enumerate() {
            info!("▶️  Stage {}/{}: {}", i+1, pipeline.stages.len(), stage.plugin.name());
            let start = Instant::now();

            let result = stage.plugin.execute(&Context::debug(), &stage.into()).await?;

            info!("   ✓ Completed in {:?}", start.elapsed());
            results.push(result);
        }

        Ok(DebugResult { results, pipeline })
    }
}
```

### Step 4: CLI Tool MVP (1 commit)

```rust
// crates/video-extract-cli/src/main.rs
use clap::{Parser, Subcommand};
use video_extract::Client;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Debug {
        #[arg(short, long)]
        input: PathBuf,

        #[arg(short, long, value_delimiter = ',')]
        ops: Vec<String>,

        #[arg(short, long, default_value = "./outputs")]
        output_dir: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Debug { input, ops, output_dir } => {
            let client = Client::builder()
                .output_dir(output_dir)
                .build()?;

            let spec = build_output_spec(&input, &ops)?;
            let result = client.debug(spec).await?;

            println!("✅ Complete!");
            println!("Results saved to: {}", result.output_dir.display());
        }
    }

    Ok(())
}
```

### Step 5: Integration (1-2 commits)

- Wire up CLI to use video-extract library
- Update existing tests to work with new architecture
- Verify no performance regressions
- Update README with new usage

---

## Threading & Performance Considerations

### When You Need Threading Optimizations

**1. Bulk Mode Executor (Phase 4)**

```rust
// You'll need this pattern:
use tokio::sync::Semaphore;

pub struct BulkExecutor {
    num_workers: usize,  // Limit concurrent files
}

impl BulkExecutor {
    pub async fn execute_bulk(&self, requests: Vec<ExecuteRequest>) -> Result<Vec<BulkResult>> {
        let semaphore = Arc::new(Semaphore::new(self.num_workers));

        let tasks: Vec<_> = requests.into_iter()
            .map(|req| {
                let sem = Arc::clone(&semaphore);
                let registry = Arc::clone(&self.registry);

                tokio::spawn(async move {
                    let _permit = sem.acquire().await.unwrap();
                    // Process file...
                })
            })
            .collect();

        // Wait for all
        join_all(tasks).await
    }
}
```

**When to use**:
- Processing multiple files concurrently (bulk mode)
- Limit to `num_cpus::get()` or user-specified worker count
- Use semaphore to prevent overloading (too many concurrent operations)

**2. Performance Mode - Parallel Plugin Execution**

```rust
// Within single file, run independent operations in parallel
pub async fn execute_parallel(&self, stages: Vec<PipelineStage>) -> Result<Vec<PluginResponse>> {
    // Group stages with no dependencies
    let parallel_groups = group_independent_stages(&stages);

    for group in parallel_groups {
        // Execute all in group concurrently
        let futures: Vec<_> = group.iter()
            .map(|stage| stage.plugin.execute(&ctx, &req))
            .collect();

        let results = try_join_all(futures).await?;
        // ...
    }
}
```

**When to use**:
- Performance mode (minimize latency for single file)
- Operations that don't depend on each other (transcription + object detection can run in parallel)
- Watch for: Memory usage (don't load too many models simultaneously)

**3. Rayon for CPU-Bound Tasks**

```rust
use rayon::prelude::*;

// For processing multiple keyframes in parallel (CPU-bound)
let detections: Vec<_> = keyframes.par_iter()
    .map(|frame| detect_objects(frame))
    .collect();
```

**When to use**:
- CPU-bound operations (image processing, text analysis)
- Data parallelism (process N items independently)
- **Warning**: Don't mix with GPU operations (causes contention)

### Library Optimization Checkpoints

**🚨 Flag these issues as you encounter them:**

#### 1. Model Loading Overhead

**Problem**: Loading ONNX models takes 1-2 seconds, dominates processing time

**Solution**:
```rust
pub struct ModelCache {
    models: Arc<RwLock<HashMap<String, Arc<Session>>>>,
}

// Load once, reuse across requests
impl ModelCache {
    pub async fn get_or_load(&self, model_path: &str) -> Arc<Session> {
        // Check cache first
        if let Some(session) = self.models.read().await.get(model_path) {
            return Arc::clone(session);
        }

        // Load and cache
        let session = Session::builder().commit_from_file(model_path)?;
        let session = Arc::new(session);
        self.models.write().await.insert(model_path.to_string(), Arc::clone(&session));
        session
    }
}
```

**When to implement**: After Phase 1, if you see model loading in benchmarks

#### 2. Intermediate File I/O

**Problem**: Writing audio.wav to disk, then reading it back wastes time

**Solution**:
```rust
pub enum PluginData {
    AudioFile(PathBuf),          // For compatibility
    AudioSamples(Vec<f32>),      // In-memory optimization
    VideoFrames(Vec<RgbImage>),  // In-memory optimization
    // ...
}

// Pass data in-memory between plugins when possible
```

**When to implement**: Phase 3 (pipeline composition) if benchmarks show I/O bottleneck

#### 3. Async vs Blocking Operations

**Problem**: Mixing tokio async with blocking code causes thread pool exhaustion

**Solution**:
```rust
// Wrong: Blocks tokio thread
async fn plugin_execute(&self) -> Result<Response> {
    let result = heavy_cpu_work();  // ❌ Blocks
}

// Right: Spawn blocking task
async fn plugin_execute(&self) -> Result<Response> {
    let result = tokio::task::spawn_blocking(|| {
        heavy_cpu_work()  // ✅ Runs on blocking thread pool
    }).await??;
}
```

**When to watch for**:
- ONNX inference (can be blocking)
- FFmpeg operations (blocking I/O)
- Image decoding/encoding

#### 4. Zero-Copy Optimizations

**Problem**: Copying large buffers (video frames, audio samples) wastes CPU and memory

**Solution**:
```rust
// Use Arc for shared ownership without copies
pub struct PluginResponse {
    data: Arc<PluginData>,  // Can be cloned cheaply
}

// Or use bytes::Bytes for immutable buffers
use bytes::Bytes;

pub struct AudioData {
    samples: Bytes,  // Zero-copy slice
}
```

**When to implement**: If profiling shows memcpy in hotpath (use `perf` on Linux, Instruments on macOS)

### Performance Measurement

**Add this to your implementation:**

```rust
// In Context
pub struct Context {
    mode: ExecutionMode,
    start_time: Instant,
    metrics: Arc<Mutex<Metrics>>,
}

pub struct Metrics {
    pub plugin_timings: HashMap<String, Duration>,
    pub cache_hits: usize,
    pub cache_misses: usize,
}

// In each plugin execution
let start = Instant::now();
let result = plugin.execute(ctx, req).await?;
ctx.metrics.lock().await.plugin_timings.insert(
    plugin.name().to_string(),
    start.elapsed()
);
```

**Report metrics** in debug mode:
```
Pipeline complete in 2.85s
  ├─ ingestion: 0.15s (5%)
  ├─ audio_extract: 0.10s (4%)
  ├─ transcription: 1.80s (63%) ⚠️ BOTTLENECK
  ├─ keyframes: 0.45s (16%)
  └─ objects: 0.35s (12%)

Cache: 2 hits, 3 misses (40% hit rate)
```

---

## Performance Targets to Hit

### Phase 1 (Debug Mode)

**Baseline**: Current system (N=83)
- Single file processing: ~5s for 10s video
- Bulk: 3.38 files/sec

**Phase 1 Target**: **No regressions**
- Same or better performance
- Main goal is correct functionality
- Performance optimizations come in Phase 4

### Phase 4 (Performance Mode)

**Target**:
- Single file: < 3s for 10s video (1.7x improvement)
- Time to first result: < 1s (5x improvement)
- CPU utilization: > 90% (currently ~60%)

**How to achieve**:
- Parallel plugin execution (independent operations)
- Streaming results (return transcription while object detection runs)
- Model caching (load once, reuse)

### Phase 5 (Bulk Mode)

**Target**:
- Throughput: 5 files/sec (1.5x improvement from 3.38)
- CPU: > 90% utilization
- GPU: > 85% utilization

**How to achieve**:
- Worker pool with optimal sizing (num_cpus::get())
- Minimize per-file overhead (model caching critical)
- Pipeline batching (process multiple files through same stage together)

---

## Red Flags - When to Ask for Help

### 🚨 Performance Issues

**If you see**:
- > 10% performance regression vs baseline
- CPU utilization < 50% in bulk mode
- Memory usage growing unbounded

**Action**:
1. Profile with `cargo flamegraph` or `perf`
2. Report findings: "Transcription taking 3s (baseline: 1.2s), profiler shows model loading in loop"
3. User will advise on threading/optimization approach

### 🚨 Architecture Issues

**If you see**:
- Lifetime issues with `Arc<dyn Plugin>` and `async`
- Can't figure out how to make Plugin trait object-safe
- Registry lookup getting too complex

**Action**:
1. Describe the problem with code snippet
2. User will help simplify design or suggest pattern

### 🚨 Integration Issues

**If you see**:
- Existing tests failing with new architecture
- Can't wrap existing modules cleanly
- Breaking changes required to current API

**Action**:
1. List incompatibilities
2. Propose migration strategy
3. User will approve or suggest alternative

---

## Testing Strategy

### Phase 1: Core Architecture

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_lookup_simple() {
        let registry = Registry::new();

        // Register mock plugin
        registry.register(Box::new(MockTranscriptionPlugin));

        // Build OutputSpec
        let spec = OutputSpec {
            sources: vec![],
            operation: Operation::Transcription { /* ... */ },
        };

        // Lookup should find plugin
        let pipeline = registry.lookup(&spec).unwrap();
        assert_eq!(pipeline.stages.len(), 1);
        assert_eq!(pipeline.stages[0].plugin.name(), "transcription");
    }

    #[test]
    fn test_pipeline_composition() {
        // Test: video → audio → transcription
        let spec = OutputSpec {
            operation: Operation::Transcription { /* ... */ },
            sources: vec![
                OutputSpec {
                    operation: Operation::Audio { sample_rate: 16000, channels: 1 },
                    sources: vec![
                        OutputSpec {
                            operation: Operation::DataSource(DataSource::LocalFile { /* ... */ }),
                            sources: vec![],
                        }
                    ],
                }
            ],
        };

        let pipeline = registry.lookup(&spec).unwrap();

        // Should have 2 stages: audio extract → transcription
        assert_eq!(pipeline.stages.len(), 2);
        assert_eq!(pipeline.stages[0].output_type, "Audio");
        assert_eq!(pipeline.stages[1].output_type, "Transcription");
    }
}
```

### Phase 1: Integration Tests

```rust
#[tokio::test]
async fn test_debug_executor_end_to_end() {
    let executor = DebugExecutor::new(
        Arc::new(registry),
        PathBuf::from("/tmp/test-outputs")
    );

    let request = ExecuteRequest {
        input: DataSource::LocalFile {
            path: PathBuf::from("test_data/sample.mp4"),
            format_hint: Some("mp4".to_string()),
        },
        operations: vec![
            Operation::Transcription { /* ... */ },
        ],
    };

    let result = executor.execute(request).await.unwrap();

    assert!(result.output_dir.join("transcription.json").exists());
}
```

### Preserve Existing Tests

**Critical**: All 137 existing tests must still pass after Phase 1

Strategy:
1. Keep existing modules unchanged initially
2. Plugin wrappers call existing code
3. Add compatibility layer if needed
4. Only refactor internals once tests pass

---

## Summary for Worker N=85

**Your immediate work**:
1. Read `ARCHITECTURE_V2_DESIGN.md` fully
2. Create `crates/video-extract-core/` with Plugin trait, Registry, OutputSpec
3. Migrate transcription plugin as proof-of-concept
4. Implement Debug executor
5. Create minimal CLI (`video-extract debug video.mp4 --ops transcription`)

**Performance considerations**:
- Phase 1: Focus on correctness, no regressions
- Phase 4+: Parallel execution, model caching, streaming
- Flag issues early: "Seeing X performance, baseline was Y, profiler shows Z"

**Success metrics**:
- All existing tests pass
- Can run: `cargo run --bin video-extract-cli -- debug test.mp4 --ops transcription`
- Performance within 10% of baseline (no major regressions)
- Clean architecture (passes user review)

**Estimated effort**: 4-5 commits for Phase 1

**User will monitor** for threading/optimization needs and provide guidance.

**Questions?** Commit your progress frequently. User is watching and will help if you get stuck.

Let's build a high-performance system! 🚀
