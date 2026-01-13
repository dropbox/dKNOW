# Architecture V2: High-Performance Plugin System
**Design Date**: 2025-10-28
**Status**: IMPLEMENTED (N=84-92) - Plugin architecture, Debug/Performance/Bulk executors operational
**Implementation Status**:
- ✅ Plugin architecture complete (N=84-90)
- ✅ Debug executor operational (N=88)
- ✅ Performance/Bulk executors implemented but not optimized (N=91-92)
- ⚠️ Missing: Result caching + parallel dependency analysis (2-3 commits)
**Inspiration**: Dropbox Riviera Plugin Architecture

## Executive Summary

Transform the current REST API server into a **high-performance plugin-based library and CLI tool** inspired by Dropbox's battle-tested Riviera architecture (millions of conversions/day). This design supports three execution modes optimized for different use cases:

1. **Debug Mode** - Single file, verbose logging, intermediate outputs
2. **Performance Mode** - Single file, streaming results, minimum latency
3. **Bulk Mode** - Maximum throughput per core, batch processing

The system will work as both a local laptop utility and a high-volume Dropbox cloud service.

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                         CLI / Library API                        │
│  ┌──────────┐  ┌──────────────┐  ┌────────────────┐            │
│  │  Debug   │  │ Performance  │  │  Bulk          │            │
│  │  Mode    │  │ Mode         │  │  Mode          │            │
│  └────┬─────┘  └──────┬───────┘  └────────┬───────┘            │
│       │               │                    │                     │
│       └───────────────┴────────────────────┘                     │
│                       │                                          │
│              ┌────────▼─────────┐                                │
│              │    Frontend      │                                │
│              │   Orchestrator   │                                │
│              └────────┬─────────┘                                │
│                       │                                          │
│         ┌─────────────┼─────────────┐                            │
│         │             │             │                            │
│  ┌──────▼──────┐ ┌───▼────┐ ┌─────▼──────┐                     │
│  │   Plugin    │ │ Cache  │ │  Registry  │                     │
│  │   Executor  │ │ Layer  │ │  (Lookup)  │                     │
│  └──────┬──────┘ └────────┘ └────────────┘                     │
│         │                                                        │
│  ┌──────▼──────────────────────────────────┐                    │
│  │          Plugin Ecosystem                │                    │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐   │                    │
│  │  │ Ingest  │ │ Decode  │ │ Extract │   │                    │
│  │  ├─────────┤ ├─────────┤ ├─────────┤   │                    │
│  │  │ Analyze │ │ Embed   │ │ Fuse    │   │                    │
│  │  └─────────┘ └─────────┘ └─────────┘   │                    │
│  └──────────────────────────────────────────┘                    │
└─────────────────────────────────────────────────────────────────┘
```

---

## Core Concepts

### 1. OutputSpec - Composable Operation Specification

```rust
/// Defines what output is desired and how to produce it
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputSpec {
    /// Pipeline source(s) - nested OutputSpecs to execute first
    pub sources: Vec<OutputSpec>,

    /// The operation to perform
    pub operation: Operation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Operation {
    // ────────── Data Sources ──────────
    /// Raw input file/URL/S3
    DataSource(DataSource),

    // ────────── Extraction ──────────
    /// Extract audio track (PCM 16kHz mono for ML)
    Audio { sample_rate: u32, channels: u8 },

    /// Extract video frames at specified FPS
    Frames { fps: f32, format: PixelFormat },

    /// Extract keyframes (scene changes)
    Keyframes { max_frames: Option<u32>, min_interval_sec: f32 },

    // ────────── Analysis ──────────
    /// Transcribe speech to text
    Transcription {
        language: Option<String>,
        model: WhisperModel,  // tiny, base, small, medium, large
    },

    /// Speaker diarization (who spoke when)
    Diarization {
        num_speakers: Option<u32>,  // Auto-detect if None
    },

    /// Detect objects in frames
    ObjectDetection {
        model: ObjectDetectionModel,  // yolov8n, yolov8s, yolov8m
        confidence_threshold: f32,
        classes: Option<Vec<String>>,  // None = all 80 COCO classes
    },

    /// Detect faces in frames
    FaceDetection {
        min_size: u32,  // Minimum face size in pixels
        include_landmarks: bool,
    },

    /// Extract text via OCR
    OCR {
        languages: Vec<String>,  // ["en", "zh"], etc.
    },

    /// Detect scene changes
    SceneDetection {
        threshold: f32,
        keyframes_only: bool,  // 45.9x speedup
    },

    // ────────── Embeddings ──────────
    /// Vision embeddings (CLIP)
    VisionEmbeddings {
        model: VisionModel,  // clip-vit-b32, clip-vit-l14
    },

    /// Text embeddings (Sentence-Transformers)
    TextEmbeddings {
        model: TextModel,  // all-minilm-l6-v2, all-mpnet-base-v2
    },

    /// Audio embeddings (CLAP)
    AudioEmbeddings {
        model: AudioModel,  // clap-htsat-fused
    },

    // ────────── Fusion ──────────
    /// Cross-modal temporal fusion
    Fusion {
        align_modalities: bool,
        extract_entities: bool,
        build_relationships: bool,
    },

    // ────────── Metadata ──────────
    /// Extract video/audio metadata (duration, codec, bitrate, etc.)
    Metadata {
        include_streams: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataSource {
    /// Local file path
    LocalFile { path: PathBuf, format_hint: Option<String> },

    /// HTTP/HTTPS URL
    Url { url: String, format_hint: Option<String> },

    /// S3 object
    S3 { bucket: String, key: String, format_hint: Option<String> },

    /// Raw bytes with format hint
    Bytes { data: Vec<u8>, format_hint: String },
}
```

### 2. Plugin Manifest (YAML)

```yaml
# config/plugins/video_keyframes.yaml
name: video_keyframes
description: "Extract keyframes from video files"

inputs:
  - mp4
  - mov
  - avi
  - mkv
  - webm
  - flv

outputs:
  - Keyframes

config:
  max_file_size_mb: 10000
  requires_gpu: false
  experimental: false

performance:
  avg_processing_time_per_gb: "30s"
  memory_per_file_mb: 512
  supports_streaming: false

cache:
  enabled: true
  version: 3
  invalidate_before: "2025-10-28"

# Implementation location
implementation:
  crate: "video-audio-keyframe-extractor"
  function: "extract_keyframes"
```

```yaml
# config/plugins/transcription.yaml
name: transcription
description: "Transcribe speech to text using Whisper"

inputs:
  - Audio  # Can take output of 'audio_extract' plugin
  - wav
  - mp3
  - flac
  - m4a

outputs:
  - Transcription

config:
  max_file_size_mb: 512
  requires_gpu: false  # CPU inference is fast enough (2.9x faster than Python)
  experimental: false
  models:
    - tiny    # 39M params, 1GB VRAM, 5x real-time
    - base    # 74M params, 1.5GB VRAM, 3x real-time
    - small   # 244M params, 2.5GB VRAM, 1.5x real-time

performance:
  avg_processing_time_per_gb: "120s"
  memory_per_file_mb: 1024
  supports_streaming: true  # Can stream transcript segments as they complete

cache:
  enabled: true
  version: 1
  invalidate_before: "2025-10-01"

implementation:
  crate: "transcription"
  function: "transcribe_audio"
```

### 3. Plugin Interface

```rust
/// Core plugin trait - all plugins must implement this
#[async_trait]
pub trait Plugin: Send + Sync {
    /// Unique plugin identifier
    fn name(&self) -> &str;

    /// Get plugin configuration
    fn config(&self) -> &PluginConfig;

    /// Check if this plugin can handle the given input type
    fn supports_input(&self, input_type: &str) -> bool;

    /// Check if this plugin produces the given output type
    fn produces_output(&self, output_type: &str) -> bool;

    /// Execute the plugin operation
    async fn execute(
        &self,
        ctx: &Context,
        request: &PluginRequest,
    ) -> Result<PluginResponse, PluginError>;

    /// Validate cached result is still valid
    fn is_valid_cache_hit(
        &self,
        cached_response: &PluginResponse,
        cache_metadata: &CacheMetadata,
    ) -> bool {
        // Default: check plugin version and timestamp
        cache_metadata.plugin_version >= self.config().cache.version &&
        cache_metadata.created_at >= self.config().cache.invalidate_before
    }

    /// Optional: Streaming execution for real-time results
    async fn execute_streaming(
        &self,
        ctx: &Context,
        request: &PluginRequest,
    ) -> Result<PluginStreamingResponse, PluginError> {
        // Default: fall back to buffered execution
        let response = self.execute(ctx, request).await?;
        Ok(PluginStreamingResponse::Complete(response))
    }
}

/// Streaming response for real-time output
pub enum PluginStreamingResponse {
    /// Partial result (e.g., transcript segment, detected object)
    Partial(PartialResult),

    /// Final complete result
    Complete(PluginResponse),
}
```

### 4. Registry - Plugin Lookup and Routing

```rust
pub struct Registry {
    /// All registered plugins by name
    plugins: HashMap<String, Arc<dyn Plugin>>,

    /// Plugins indexed by output type they produce
    by_output: HashMap<String, Vec<Arc<dyn Plugin>>>,

    /// Transitive closure of supported inputs (for pipeline composition)
    transitive_inputs: HashMap<String, HashSet<String>>,
}

impl Registry {
    /// Find plugin(s) to satisfy an OutputSpec
    pub fn lookup(&self, spec: &OutputSpec) -> Result<Pipeline, RegistryError> {
        let mut stages = Vec::new();

        // 1. Recursively resolve sources
        for source in &spec.sources {
            let source_pipeline = self.lookup(source)?;
            stages.extend(source_pipeline.stages);
        }

        // 2. Determine input type (output of last source stage, or raw file)
        let input_type = if let Some(last) = stages.last() {
            last.output_type.clone()
        } else {
            // No source pipeline - must be DataSource
            match &spec.operation {
                Operation::DataSource(ds) => ds.format_hint()?,
                _ => return Err(RegistryError::NoSource),
            }
        };

        // 3. Find plugin that converts input_type → desired output
        let output_type = spec.operation.output_type_name();
        let candidates = self.by_output.get(output_type)
            .ok_or(RegistryError::NoPluginForOutput(output_type.clone()))?;

        let plugin = candidates.iter()
            .find(|p| p.supports_input(&input_type))
            .ok_or(RegistryError::NoPluginForConversion {
                from: input_type.clone(),
                to: output_type.clone(),
            })?;

        // 4. Add stage to pipeline
        stages.push(PipelineStage {
            plugin: Arc::clone(plugin),
            input_type,
            output_type: output_type.clone(),
            operation: spec.operation.clone(),
        });

        Ok(Pipeline { stages })
    }
}
```

---

## Three Execution Modes

### Mode 1: Debug Mode 🐛

**Use Case**: Development, debugging, understanding what's happening
**Priority**: Observability > Performance
**Output**: Verbose logs, intermediate files, timing breakdown

```bash
# CLI Usage
video-extract debug \
  --input video.mp4 \
  --output-dir ./debug-outputs/ \
  --operations transcription,objects,scenes \
  --save-intermediates \
  --verbose

# What happens:
# 1. Saves intermediate files: audio.wav, keyframes/*.jpg
# 2. Logs every plugin execution with timing
# 3. Outputs structured JSON with nested contexts
# 4. Preserves temp files for inspection
```

**Implementation:**

```rust
pub struct DebugExecutor {
    registry: Arc<Registry>,
    cache: Arc<Cache>,
    output_dir: PathBuf,
    save_intermediates: bool,
}

impl DebugExecutor {
    pub async fn execute(&self, request: ExecuteRequest) -> Result<DebugResult> {
        let start = Instant::now();

        // Build pipeline
        info!("🔍 Building pipeline for operations: {:?}", request.operations);
        let pipeline = self.registry.lookup(&request.output_spec)?;

        info!("📋 Pipeline has {} stages:", pipeline.stages.len());
        for (i, stage) in pipeline.stages.iter().enumerate() {
            info!("  {}. {} ({} → {})",
                i + 1, stage.plugin.name(), stage.input_type, stage.output_type);
        }

        // Execute each stage with detailed logging
        let mut results = Vec::new();
        for (i, stage) in pipeline.stages.iter().enumerate() {
            let stage_start = Instant::now();
            info!("▶️  Stage {}/{}: {}", i + 1, pipeline.stages.len(), stage.plugin.name());

            // Check cache
            if let Some(cached) = self.cache.get(&stage).await? {
                info!("   ✅ Cache hit (age: {:?})", cached.age());
                results.push(cached);
                continue;
            }

            // Execute plugin
            let result = stage.plugin.execute(&Context::debug(), &stage.into()).await?;

            let duration = stage_start.elapsed();
            info!("   ✓ Completed in {:?}", duration);

            // Save intermediate outputs if requested
            if self.save_intermediates {
                let path = self.save_intermediate(&result, i, &stage.output_type)?;
                info!("   💾 Saved intermediate: {}", path.display());
            }

            // Cache result
            self.cache.put(&stage, &result).await?;
            results.push(result);
        }

        let total_duration = start.elapsed();
        info!("🎉 Pipeline complete in {:?}", total_duration);

        Ok(DebugResult {
            results,
            pipeline,
            total_duration,
            stage_timings: /* ... */,
        })
    }
}
```

### Mode 2: Performance Mode ⚡

**Use Case**: Interactive single-file processing, real-time preview
**Priority**: Latency < Throughput
**Output**: Streaming results as they complete

```bash
# CLI Usage
video-extract perf \
  --input video.mp4 \
  --operations transcription,objects,scenes \
  --stream \
  --output results.jsonl  # One JSON object per line as results arrive

# What happens:
# 1. Starts all compatible operations in parallel
# 2. Streams results as each completes (JSONL format)
# 3. Uses all CPU/GPU resources for this one file
# 4. Optimizes for minimum time-to-first-result
```

**Implementation:**

```rust
pub struct PerformanceExecutor {
    registry: Arc<Registry>,
    cache: Arc<Cache>,
    parallelism: usize,  // Max parallel tasks
}

impl PerformanceExecutor {
    pub async fn execute_streaming(
        &self,
        request: ExecuteRequest,
    ) -> Result<impl Stream<Item = StreamingResult>> {
        let pipeline = self.registry.lookup(&request.output_spec)?;

        // Identify parallelizable stages (no dependencies between them)
        let parallelizable_groups = self.group_parallel_stages(&pipeline);

        let (tx, rx) = mpsc::channel(100);

        for group in parallelizable_groups {
            let tx_clone = tx.clone();
            let cache = Arc::clone(&self.cache);

            tokio::spawn(async move {
                // Execute all stages in group concurrently
                let futures: Vec<_> = group.into_iter()
                    .map(|stage| {
                        let cache = Arc::clone(&cache);
                        async move {
                            // Check cache first
                            if let Some(cached) = cache.get(&stage).await? {
                                return Ok((stage.output_type, cached));
                            }

                            // Execute with streaming if supported
                            let result = stage.plugin
                                .execute_streaming(&Context::performance(), &stage.into())
                                .await?;

                            // Stream partial results immediately
                            match result {
                                PluginStreamingResponse::Partial(partial) => {
                                    tx_clone.send(StreamingResult::Partial {
                                        output_type: stage.output_type.clone(),
                                        data: partial,
                                    }).await.ok();
                                }
                                PluginStreamingResponse::Complete(response) => {
                                    // Cache and return complete result
                                    cache.put(&stage, &response).await?;
                                }
                            }

                            Ok((stage.output_type, response))
                        }
                    })
                    .collect();

                // Wait for all in group
                let results = join_all(futures).await;

                // Send complete results
                for result in results {
                    if let Ok((output_type, data)) = result {
                        tx_clone.send(StreamingResult::Complete {
                            output_type,
                            data,
                        }).await.ok();
                    }
                }
            });
        }

        drop(tx);  // Close sender when all tasks spawned
        Ok(ReceiverStream::new(rx))
    }
}
```

### Mode 3: Bulk Mode 🚀

**Use Case**: Batch processing, overnight jobs, maximum files/hour
**Priority**: Throughput (files/sec) > Latency
**Output**: Efficient multi-file processing, optimal CPU/GPU utilization

```bash
# CLI Usage
video-extract bulk \
  --input-list files.txt \
  --operations transcription,objects,scenes \
  --output-dir ./batch-outputs/ \
  --workers 8 \
  --progress

# What happens:
# 1. Processes multiple files concurrently (--workers)
# 2. Each file goes through full pipeline independently
# 3. Optimizes for sustained throughput
# 4. Progress bar shows files/sec
```

**Implementation:**

```rust
pub struct BulkExecutor {
    registry: Arc<Registry>,
    cache: Arc<Cache>,
    num_workers: usize,
}

impl BulkExecutor {
    pub async fn execute_bulk(
        &self,
        requests: Vec<ExecuteRequest>,
    ) -> Result<Vec<BulkResult>> {
        // Use semaphore to limit concurrency
        let semaphore = Arc::new(Semaphore::new(self.num_workers));

        let tasks: Vec<_> = requests.into_iter()
            .map(|request| {
                let semaphore = Arc::clone(&semaphore);
                let registry = Arc::clone(&self.registry);
                let cache = Arc::clone(&self.cache);

                tokio::spawn(async move {
                    let _permit = semaphore.acquire().await.unwrap();

                    // Build and execute pipeline for this file
                    let pipeline = registry.lookup(&request.output_spec)?;

                    let mut current_input = request.input;
                    for stage in &pipeline.stages {
                        // Check cache
                        if let Some(cached) = cache.get(&stage).await? {
                            current_input = cached;
                            continue;
                        }

                        // Execute stage
                        let result = stage.plugin
                            .execute(&Context::bulk(), &stage.into())
                            .await?;

                        // Cache
                        cache.put(&stage, &result).await?;
                        current_input = result;
                    }

                    Ok::<BulkResult, Error>(BulkResult {
                        input_path: request.input_path,
                        results: current_input,
                        duration: /* ... */,
                    })
                })
            })
            .collect();

        // Wait for all tasks with progress reporting
        let mut results = Vec::with_capacity(tasks.len());
        let progress = ProgressBar::new(tasks.len() as u64);

        for task in tasks {
            let result = task.await??;
            results.push(result);
            progress.inc(1);
        }

        progress.finish();
        Ok(results)
    }
}
```

---

## Output Naming Convention

### Consistent Output Structure

```
{output_dir}/
├── {file_stem}/
│   ├── metadata.json              # Always generated
│   ├── audio.wav                  # If audio extraction requested
│   ├── keyframes/
│   │   ├── frame_0000.jpg
│   │   ├── frame_0001.jpg
│   │   └── ...
│   ├── transcription.json         # Whisper output
│   ├── diarization.json          # Speaker timeline
│   ├── objects.json              # YOLOv8 detections
│   ├── faces.json                # RetinaFace detections
│   ├── ocr.json                  # PaddleOCR text
│   ├── scenes.json               # Scene boundaries
│   ├── embeddings/
│   │   ├── vision.npy            # CLIP vectors (Nx512)
│   │   ├── text.npy              # Sentence-Transformer (Nx384)
│   │   └── audio.npy             # CLAP vectors (Nx512)
│   └── fusion.json               # Unified timeline
└── batch_summary.json            # Bulk mode only
```

### Naming Pattern Rules

1. **Deterministic**: Same input + operations = same output paths
2. **No Collisions**: File stem disambiguates multiple inputs
3. **Discoverable**: Standard names for standard operations
4. **Extensible**: Plugins can define custom output names

```rust
pub struct OutputNaming {
    base_dir: PathBuf,
    file_stem: String,
}

impl OutputNaming {
    pub fn path_for(&self, operation: &Operation) -> PathBuf {
        let file_dir = self.base_dir.join(&self.file_stem);

        match operation {
            Operation::Audio { .. } => file_dir.join("audio.wav"),
            Operation::Keyframes { .. } => file_dir.join("keyframes"),
            Operation::Transcription { .. } => file_dir.join("transcription.json"),
            Operation::Diarization { .. } => file_dir.join("diarization.json"),
            Operation::ObjectDetection { .. } => file_dir.join("objects.json"),
            Operation::FaceDetection { .. } => file_dir.join("faces.json"),
            Operation::OCR { .. } => file_dir.join("ocr.json"),
            Operation::SceneDetection { .. } => file_dir.join("scenes.json"),
            Operation::VisionEmbeddings { .. } => file_dir.join("embeddings/vision.npy"),
            Operation::TextEmbeddings { .. } => file_dir.join("embeddings/text.npy"),
            Operation::AudioEmbeddings { .. } => file_dir.join("embeddings/audio.npy"),
            Operation::Fusion { .. } => file_dir.join("fusion.json"),
            Operation::Metadata { .. } => file_dir.join("metadata.json"),
            _ => file_dir.join(format!("{}.json", operation.name())),
        }
    }
}
```

---

## Library API

### Programmatic Access

```rust
use video_extract::{Client, OutputSpec, Operation, ExecutionMode};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize client
    let client = Client::builder()
        .cache_dir("/tmp/video-extract-cache")
        .num_workers(8)
        .build()?;

    // Define operations
    let spec = OutputSpec::builder()
        .source(DataSource::local_file("video.mp4"))
        .operation(Operation::Transcription {
            language: Some("en".to_string()),
            model: WhisperModel::Base,
        })
        .operation(Operation::ObjectDetection {
            model: ObjectDetectionModel::YoloV8n,
            confidence_threshold: 0.5,
            classes: None,
        })
        .build();

    // Execute in performance mode (streaming)
    let mut stream = client
        .execute_streaming(spec, ExecutionMode::Performance)
        .await?;

    while let Some(result) = stream.next().await {
        match result {
            StreamingResult::Partial { output_type, data } => {
                println!("Partial result from {}: {:?}", output_type, data);
            }
            StreamingResult::Complete { output_type, data } => {
                println!("Complete result from {}: {:?}", output_type, data);
            }
        }
    }

    Ok(())
}
```

### Bulk Processing

```rust
use video_extract::{Client, BulkRequest, ExecutionMode};

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::builder()
        .num_workers(16)  // Process 16 files concurrently
        .build()?;

    // Read file list
    let files: Vec<PathBuf> = std::fs::read_to_string("files.txt")?
        .lines()
        .map(PathBuf::from)
        .collect();

    // Build requests
    let requests: Vec<_> = files.into_iter()
        .map(|path| {
            BulkRequest {
                input: DataSource::local_file(path),
                operations: vec![
                    Operation::Transcription { /* ... */ },
                    Operation::ObjectDetection { /* ... */ },
                ],
                output_dir: PathBuf::from("./outputs"),
            }
        })
        .collect();

    // Execute bulk
    let results = client
        .execute_bulk(requests, ExecutionMode::Bulk)
        .await?;

    // Print summary
    println!("Processed {} files", results.len());
    println!("Success: {}", results.iter().filter(|r| r.success).count());
    println!("Failed: {}", results.iter().filter(|r| !r.success).count());

    Ok(())
}
```

---

## Migration Path from Current System

### Phase 1: Plugin Extraction (2-3 AI commits)

**Goal**: Extract existing modules into plugin format

```
Current:
  crates/transcription/
  crates/object-detection/
  crates/keyframe-extractor/

New:
  crates/video-extract-core/      # Registry, Frontend, Plugin trait
  crates/video-extract-plugins/
    ├── audio-extract/
    ├── keyframes/
    ├── transcription/
    ├── object-detection/
    ├── face-detection/
    ├── ocr/
    ├── diarization/
    ├── scene-detection/
    ├── embeddings/
    └── fusion/
  crates/video-extract-cli/       # CLI tool
  crates/video-extract/           # Public API crate
```

**Changes**:
1. Create `Plugin` trait
2. Wrap existing modules with plugin implementations
3. Create YAML manifests for each plugin
4. Build registry from manifests
5. Keep existing REST API as thin wrapper over library

### Phase 2: CLI Tool (1-2 AI commits)

**Goal**: Create command-line interface with 3 modes

```bash
cargo install --path crates/video-extract-cli

# Debug mode
video-extract debug video.mp4 --ops transcription,objects

# Performance mode
video-extract perf video.mp4 --ops transcription,objects --stream

# Bulk mode
video-extract bulk files.txt --ops transcription,objects --workers 16
```

### Phase 3: Output Specification (1 AI commit)

**Goal**: Implement `OutputSpec` and pipeline composition

```rust
// Example: Transcribe audio extracted from video
let spec = OutputSpec {
    sources: vec![
        OutputSpec {
            sources: vec![],
            operation: Operation::Audio { sample_rate: 16000, channels: 1 },
        }
    ],
    operation: Operation::Transcription {
        language: None,  // Auto-detect
        model: WhisperModel::Base,
    },
};
```

### Phase 4: Performance Modes (2-3 AI commits)

**Goal**: Implement 3 execution engines

1. Debug executor with verbose logging
2. Performance executor with streaming
3. Bulk executor with worker pool

### Phase 5: Cache Layer (1-2 AI commits)

**Goal**: Implement content-addressed caching

```rust
struct CacheKey {
    input_hash: Blake3Hash,      // Hash of input file
    plugin_name: String,
    plugin_version: u32,
    operation_hash: Blake3Hash,  // Hash of operation parameters
}
```

---

## Performance Targets

### Single File (Performance Mode)

| Operation | Target Latency | Baseline (Current) | Improvement |
|-----------|---------------|-------------------|-------------|
| Transcription (10s audio) | < 0.5s | ~1.2s | 2.4x |
| Keyframe extraction (10s video) | < 0.3s | ~0.5s | 1.7x |
| Object detection (15 frames) | < 1.5s | ~2s | 1.3x |
| **Full pipeline (10s video)** | **< 3s** | **~5s** | **1.7x** |

### Bulk Mode

| Metric | Target | Baseline (Current) | Improvement |
|--------|--------|-------------------|-------------|
| Throughput | 5 files/sec | 3.38 files/sec | 1.5x |
| CPU utilization | > 90% | ~60% | 1.5x |
| GPU utilization | > 85% | ~40% | 2.1x |

### Memory Efficiency

| Mode | Target | Baseline |
|------|--------|----------|
| Single file | < 2GB | ~1.5GB |
| Bulk (16 workers) | < 16GB | ~12GB |

---

## Success Metrics

**For Laptop Use:**
- ✅ Single binary install (`cargo install video-extract`)
- ✅ Process 100 files in < 30 seconds (bulk mode)
- ✅ See first result in < 1 second (performance mode)
- ✅ Clear error messages and progress indicators

**For Dropbox Service:**
- ✅ Handle 10,000+ files/hour on single machine
- ✅ 99.9% success rate
- ✅ Horizontal scalability (add more machines = linear throughput increase)
- ✅ < 1% cache miss rate after warm-up
- ✅ Graceful degradation (continue if optional operations fail)

---

## Implementation Timeline

**Total Estimate: 12-15 AI commits (~2-3 days AI time)**

1. **Core Architecture** (4-5 commits)
   - Plugin trait and registry
   - OutputSpec and pipeline builder
   - Frontend orchestrator
   - Cache layer

2. **Plugin Migration** (3-4 commits)
   - Wrap existing modules as plugins
   - YAML manifests
   - Plugin loading and registration

3. **Execution Modes** (3-4 commits)
   - Debug executor
   - Performance executor (streaming)
   - Bulk executor (worker pool)

4. **CLI Tool** (1-2 commits)
   - Argument parsing
   - Output formatting
   - Progress reporting

5. **Testing & Validation** (1 commit)
   - Integration tests
   - Performance benchmarks
   - Documentation

---

## Next Steps for Worker

1. **Read and understand this design**
2. **Create Phase 1: Core Architecture**
   - `crates/video-extract-core/` with Plugin trait
   - Registry implementation
   - OutputSpec types
3. **Migrate one plugin as proof-of-concept**
   - Start with `transcription` (simplest, well-tested)
   - Create YAML manifest
   - Verify it works through new interface
4. **Implement Debug executor as MVP**
   - Simplest execution mode
   - Validates entire architecture
5. **Iterate from there**

---

## Appendix: Key Differences from Riviera

| Aspect | Riviera | Our Design |
|--------|---------|-----------|
| **Language** | Go | Rust |
| **Domain** | Document conversion | Video/Audio processing |
| **Cache** | Memcache + disk | Local disk (Blake3 content-addressed) |
| **Backends** | Remote RPC services | In-process native code |
| **Streaming** | Chunked HTTP | Tokio channels (mpsc) |
| **Config** | Centralized YAML | Distributed plugin manifests |
| **Frontend** | Monolithic server | Library + CLI + Optional server |

**Why these changes?**
- **Rust**: Performance, safety, async/await native
- **In-process**: Eliminate RPC overhead, use shared memory
- **Local cache**: Laptop-friendly, no infrastructure dependencies
- **Library-first**: CLI and server are thin wrappers
- **Distributed config**: Plugins self-describe, easier to extend
