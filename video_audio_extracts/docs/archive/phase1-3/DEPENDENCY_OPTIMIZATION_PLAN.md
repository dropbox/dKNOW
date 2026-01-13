# Dependency Optimization Plan
**Date**: 2025-10-28
**Current Binary Size**: 27MB (down from 37MB)
**Status**: COMPLETED (N=100-117) - Core optimizations implemented, remaining steps have low ROI

**COMPLETION STATUS**:
- ✅ Step 1: ONNX Runtime optimization (N=100) - +15-25% inference
- ✅ Step 2: mozjpeg integration (N=101) - +3-5x JPEG decode
- ✅ Step 3: Dependency cleanup (N=102) - -27% binary (37MB→27MB)
- ✅ Step 4: FFTW integration (N=104) - +2-3x FFT speed
- ❌ Step 6-8: Custom tokenizer, simd-json, Whisper batch - NOT IMPLEMENTED (low ROI, architectural bottleneck is root cause)

**ARCHITECTURAL BOTTLENECK IDENTIFIED (N=117)**:
- Performance gap (0.01 vs 5 files/sec) is architectural, not algorithmic
- Root cause: Work duplication (keyframes extracted 3x) + no parallelism
- Further algorithmic optimizations have minimal impact (<10%)
- See N=117-119 commit messages for detailed analysis

**DOCUMENT STATUS**: Historical reference only - optimization phase complete

## High-Impact: Optimize Dependency Packages Themselves

### 🔥 **1. tokenizers (HuggingFace) - REPLACE WITH CUSTOM**

**Current**: tokenizers v0.20 (~5MB compiled, pulls in 50+ dependencies)

**Problem**:
- Only used for text embeddings (Sentence-Transformers)
- Full HuggingFace tokenizer is overkill (supports 100+ tokenizer types)
- We only need WordPiece tokenization for MiniLM models
- Heavy compile-time cost (1-2 minutes)

**Optimization**: **Build custom lightweight tokenizer**

```rust
// Custom implementation (crates/simple-tokenizer/):
pub struct SimpleWordPieceTokenizer {
    vocab: HashMap<String, u32>,      // 30K tokens
    max_length: usize,
}

impl SimpleWordPieceTokenizer {
    // Load vocab.txt (2MB file)
    pub fn from_vocab_file(path: &Path) -> Result<Self> {
        let vocab: HashMap<String, u32> = std::fs::read_to_string(path)?
            .lines()
            .enumerate()
            .map(|(i, word)| (word.to_string(), i as u32))
            .collect();

        Ok(Self { vocab, max_length: 256 })
    }

    // Basic tokenization (sufficient for sentence embeddings)
    pub fn encode(&self, text: &str) -> Vec<u32> {
        // 1. Lowercase and split on whitespace
        // 2. Subword tokenization (WordPiece algorithm)
        // 3. Add [CLS] and [SEP] tokens
        // 4. Pad to max_length
        // ~50 lines of code
    }
}
```

**Benefits**:
- **-5MB binary size** (remove tokenizers + deps)
- **-90s compile time** (tokenizers is slow to compile)
- **-0 runtime performance impact** (tokenization is <1% of inference time)
- **Simpler code** (100 lines vs importing 50K line library)

**Effort**: 1 commit (~2 hours)
**Risk**: Medium (need to validate tokenization matches HuggingFace output)

**Validation Strategy**:
```rust
#[test]
fn test_tokenization_matches_huggingface() {
    let text = "people walking on beach";

    let hf_tokens = tokenizers::Tokenizer::from_pretrained("all-MiniLM-L6-v2")
        .encode(text)?;

    let custom_tokens = SimpleWordPieceTokenizer::from_vocab_file(
        "models/embeddings/vocab.txt"
    ).encode(text);

    assert_eq!(hf_tokens.get_ids(), custom_tokens);
}
```

---

### 🔥 **2. image Crate - FORK AND STRIP**

**Current**: image v0.25 (all codecs, ~2MB)

**Problem**:
- Includes 15+ image codecs (AVIF, TIFF, HDR, ICO, TGA, etc.)
- We only use JPEG decode + PNG decode/encode
- JPEG decoder is pure Rust (slower than libjpeg-turbo)

**Optimization 1**: **Use mozjpeg (C library, SIMD-optimized)**

```toml
# Replace image's JPEG with mozjpeg
mozjpeg = "0.10"  # Mozilla's optimized JPEG codec
```

```rust
// Custom image loading with mozjpeg
pub fn load_image_fast(path: &Path) -> Result<RgbImage> {
    let extension = path.extension()?.to_str()?;

    match extension {
        "jpg" | "jpeg" => {
            // Use mozjpeg (3-5x faster decode than image::jpeg)
            let data = std::fs::read(path)?;
            let decompressor = mozjpeg::Decompress::new_mem(&data)?;
            let rgb = decompressor.rgb()?;
            let (width, height) = (rgb.width(), rgb.height());
            RgbImage::from_raw(width as u32, height as u32, rgb.to_vec())
        }
        "png" => {
            // Keep image::png (already fast)
            image::open(path)?.to_rgb8()
        }
        _ => Err("Unsupported format")
    }
}
```

**Benefits**:
- **+3-5x JPEG decode speed** (mozjpeg uses libjpeg-turbo)
- **-1MB binary size** (remove unused codecs)
- **Critical path impact**: Keyframe extraction, object detection all decode JPEGs

**Effort**: 1 commit
**Risk**: Low (mozjpeg is production-proven, used by Firefox)

**Optimization 2**: **Feature flags to disable unused codecs**

```toml
[dependencies]
image = { version = "0.25", default-features = false, features = [
    "jpeg",     # ✅ Need for keyframes
    "png",      # ✅ Need for outputs
    # Remove: gif, ico, tiff, webp, avif, hdr, pnm, tga, dds, farbfeld
]}
```

**Benefits**: **-500KB binary size**
**Effort**: 1 line change
**Risk**: None

---

### 🔥 **3. rustfft - REPLACE WITH FFTW**

**Current**: rustfft v6.2 (pure Rust FFT)

**Problem**:
- Pure Rust implementation is 2-3x slower than FFTW (C library, SIMD hand-tuned)
- Used for mel-spectrogram generation (critical path for audio embeddings)
- FFTW is THE industry standard (30 years of optimization)

**Optimization**: **Use fftw-rs (Rust bindings to FFTW C library)**

```toml
[dependencies]
# Replace rustfft with FFTW
fftw = "0.8"  # Rust bindings to FFTW3
```

```rust
// In audio embeddings mel-spectrogram generation:
use fftw::array::AlignedVec;
use fftw::plan::*;
use fftw::types::*;

pub fn compute_mel_spectrogram_fftw(audio: &[f32]) -> Array2<f32> {
    // Use FFTW instead of rustfft
    let mut plan: C2CPlan64 = C2CPlan::aligned(
        &[audio.len()],
        Sign::Forward,
        Flag::MEASURE,  // FFTW auto-optimizes for hardware
    ).unwrap();

    let mut input = AlignedVec::new(audio.len());
    let mut output = AlignedVec::new(audio.len());

    // Copy input
    for (i, &sample) in audio.iter().enumerate() {
        input[i] = c64::new(sample as f64, 0.0);
    }

    // Execute FFT (SIMD-optimized)
    plan.c2c(&mut input, &mut output).unwrap();

    // Convert to mel-spectrogram...
}
```

**Benefits**:
- **+2-3x FFT speed** (FFTW's SIMD hand-tuning beats Rust)
- **Audio embeddings**: Mel-spectrogram generation is ~30% of processing time
- **Bulk mode**: 4.5 → **4.8 files/sec** (+7% when processing audio-heavy workloads)

**Effort**: 1 commit
**Risk**: Medium (need to link FFTW C library, system dependency)

**System Requirements**:
```bash
# macOS
brew install fftw

# Linux
apt-get install libfftw3-dev

# Rust links automatically via fftw-sys
```

---

### 🔥 **4. ort (ONNX Runtime) - OPTIMIZE CONFIGURATION**

**Current**: ort v2.0.0-rc.10 with default config

**Problem**:
- Not using all available optimizations
- Could enable graph optimizations
- Could use TensorRT on NVIDIA GPUs (not just CUDA)

**Optimization**: **Configure ONNX Runtime for maximum performance**

```rust
use ort::{
    session::{Session, SessionBuilder},
    GraphOptimizationLevel,
    ExecutionProviderDispatch,
};

pub fn create_optimized_session(model_path: &Path) -> Result<Session> {
    let session = SessionBuilder::new()?
        // Enable all graph optimizations
        .with_optimization_level(GraphOptimizationLevel::All)?

        // Enable parallel execution within ops
        .with_intra_threads(num_cpus::get_physical())?

        // Try execution providers in order of preference
        .with_execution_providers([
            // 1. TensorRT (NVIDIA GPUs, fastest)
            ExecutionProviderDispatch::TensorRT(TensorRTExecutionProvider::default()
                .with_fp16(true)              // Use FP16 for 2x speed
                .with_int8(true)              // Use INT8 if calibrated
                .with_dla_core(0)             // Deep Learning Accelerator
            ),

            // 2. CUDA (NVIDIA GPUs, fallback)
            ExecutionProviderDispatch::CUDA(CUDAExecutionProvider::default()
                .with_device_id(0)
                .with_gpu_mem_limit(2 * 1024 * 1024 * 1024)  // 2GB limit
            ),

            // 3. CoreML (Apple Silicon, Mac)
            ExecutionProviderDispatch::CoreML(CoreMLExecutionProvider::default()
                .with_ane_only(false)         // Use ANE + GPU
            ),

            // 4. CPU (fallback)
            ExecutionProviderDispatch::CPU(Default::default()),
        ])?

        // Enable memory pattern optimization
        .with_memory_pattern(true)?

        // Allocator arena optimization
        .with_allocator_config(AllocatorConfig::default()
            .with_arena_extend_strategy(ArenaExtendStrategy::NextPowerOfTwo)
        )?

        .commit_from_file(model_path)?;

    Ok(session)
}
```

**Benefits**:
- **+30-50% inference speed** on NVIDIA GPUs (TensorRT FP16)
- **+20-30% inference speed** on Apple Silicon (CoreML ANE)
- **+10-20% CPU inference** (graph optimizations)
- **Object detection**: 2s → **1.3s** (-35%)
- **Face detection**: 1.5s → **1s** (-33%)
- **Embeddings**: 2s → **1.3s** (-35%)

**Effort**: 1 commit (update all ONNX-using plugins)
**Risk**: Low (ort supports all these features, graceful fallback)

---

### 🟡 **5. whisper-rs - OPTIMIZE MODEL CONFIGURATION**

**Current**: Using default Whisper model loading

**Problem**:
- Not using quantization (INT8 models available)
- Not using Metal GPU acceleration on macOS (already enabled via features, good!)
- Not using OpenVINO on Intel CPUs

**Optimization**: **Enable INT8 quantization + OpenVINO**

```rust
use whisper_rs::{WhisperContext, WhisperContextParameters, WhisperSamplingStrategy};

pub fn create_optimized_whisper(model_path: &Path) -> Result<WhisperContext> {
    let mut params = WhisperContextParameters::default();

    // Enable Metal GPU on macOS (already doing this ✅)
    // Cargo.toml: whisper-rs = { version = "0.15", features = ["metal"] }

    // Use quantized model (INT8 for 2x speed, minimal accuracy loss)
    let model_path = PathBuf::from("models/whisper/ggml-base.en-q5_0.bin");  // 5-bit quantized

    let ctx = WhisperContext::new_with_params(
        model_path.to_str().unwrap(),
        params,
    )?;

    Ok(ctx)
}

// During transcription:
let mut params = FullParams::new(WhisperSamplingStrategy::Greedy { best_of: 1 });

// Enable optimizations
params.set_n_threads(num_cpus::get_physical() as i32);  // Use all cores
params.set_single_segment(false);  // Enable segmentation
params.set_token_timestamps(true);  // For diarization alignment
params.set_speed_up(true);         // Enable speedup flag

let state = ctx.create_state()?;
state.full(params, &audio_data)?;
```

**Benefits**:
- **+50% transcription speed** with quantized models (q5_0 or q8_0)
- **-50% VRAM usage** (important for GPU-heavy workloads)
- **Bulk mode**: 4.5 → **5.2 files/sec** (+15%) when transcription is primary task

**Effort**: 30 minutes (download quantized models, update config)
**Risk**: Low (quantization quality loss <2% WER for q5_0)

**Model Options**:
```
ggml-base.en.bin        # 142MB, 100% quality, 3x real-time
ggml-base.en-q5_0.bin   # 54MB, 98% quality, 6x real-time ⭐ RECOMMENDED
ggml-base.en-q8_0.bin   # 76MB, 99% quality, 4.5x real-time
```

---

### 🔥 **6. FFmpeg (ffmpeg-next bindings) - OPTIMIZE USAGE**

**Current**: Using default FFmpeg flags

**Problem**:
- Not using hardware-accelerated color space conversion
- Not using multithreading for decoding
- Not using optimal preset flags

**Optimization**: **Optimize FFmpeg command construction**

```rust
// In crates/audio-extractor/src/lib.rs
pub fn extract_audio_optimized(input: &Path, output: &Path, config: AudioConfig) -> Result<PathBuf> {
    ffmpeg::init()?;

    let mut ictx = ffmpeg::format::input(input)?;

    // Find best audio stream
    let audio_stream = ictx.streams().best(ffmpeg::media::Type::Audio)
        .ok_or("No audio stream")?;
    let stream_index = audio_stream.index();

    // Create output with optimized settings
    let mut octx = ffmpeg::format::output(output)?;

    // Optimization: Use multithreading
    let mut output_stream = octx.add_stream(ffmpeg::encoder::find(ffmpeg::codec::Id::PCM_S16LE))?;
    output_stream.set_parameters(/* ... */);

    // Key optimizations:
    let dict = ffmpeg::Dictionary::from_iter([
        ("threads", "auto"),              // Multi-threaded decoding
        ("thread_type", "frame+slice"),   // Parallel frame + slice
        ("lowres", "0"),                  // Full resolution (no lowres)
    ]);

    // Use format context options for performance
    let mut decoder = audio_stream.codec().decoder().audio()?;
    decoder.set_parameters(ictx.metadata())?;

    // Optimization: Process in larger chunks (reduce syscalls)
    let mut resampler = software::resampling::Context::get(
        decoder.format(),
        decoder.channel_layout(),
        decoder.rate(),
        config.format,
        config.channels,
        config.sample_rate,
    )?;

    // Process with optimal buffer sizes
    for (stream, packet) in ictx.packets().filter(|(s, _)| s.index() == stream_index) {
        decoder.send_packet(&packet)?;

        // Larger frame buffers reduce overhead
        while let Ok(mut decoded) = decoder.receive_frame() {
            let resampled = resampler.run(&decoded)?;
            output_stream.send_frame(&resampled)?;
        }
    }
}
```

**Benefits**:
- **+20-30% audio extraction speed** (multithreading)
- **+15-25% video decoding speed** (hardware color conversion)
- **Bulk mode**: Audio-heavy: 4.5 → **5.0 files/sec** (+11%)

**Effort**: 1 commit
**Risk**: Low (FFmpeg flags well-documented)

---

### 🟡 **7. ndarray - OPTIMIZE WITH BLAS**

**Current**: ndarray v0.16 (pure Rust linear algebra)

**Problem**:
- Not using BLAS (Basic Linear Algebra Subprograms)
- Matrix operations in pure Rust (slower than BLAS)
- Used for embedding normalization, NMS calculations

**Optimization**: **Link against OpenBLAS or Intel MKL**

```toml
[dependencies]
ndarray = { version = "0.16", features = ["blas"] }
blas-src = { version = "0.10", features = ["openblas"] }
# OR for Intel CPUs:
# blas-src = { version = "0.10", features = ["intel-mkl"] }
```

**Benefits**:
- **+50-100% speed** for matrix operations (dot products, normalization)
- **Embedding extraction**: CLIP/CLAP normalization 2x faster
- **Impact**: Small (~2% of total pipeline), but free performance

**Effort**: 1 line change + system dependency
**Risk**: Medium (requires system BLAS library)

**System Setup**:
```bash
# macOS
brew install openblas

# Linux
apt-get install libopenblas-dev

# Links automatically via blas-src
```

---

### 🟢 **8. serde_json - REPLACE WITH simd-json**

**Current**: serde_json v1.0 (standard JSON parser)

**Problem**:
- We serialize/deserialize JSON frequently (every plugin response)
- serde_json is not SIMD-optimized
- Alternative exists: simd-json (2-3x faster parsing)

**Optimization**: **Use simd-json for hot paths**

```toml
[dependencies]
simd-json = "0.14"
```

```rust
// For hot path JSON operations:
use simd_json;

// Parsing (2-3x faster)
let mut bytes = response_json.as_bytes().to_vec();  // Need mutable
let parsed: Value = simd_json::to_borrowed_value(&mut bytes)?;

// Serialization (1.5-2x faster)
let json_bytes = simd_json::to_vec(&data)?;
```

**Benefits**:
- **+50-150% JSON parsing speed** (SIMD accelerated)
- **Impact**: Small (~1-2% of pipeline), but affects every plugin
- **Bulk mode**: +2-3% throughput (cumulative across all plugin I/O)

**Effort**: 1 commit
**Risk**: Low (simd-json is API-compatible drop-in)

**When to use**:
- Hot paths: Plugin response serialization, bulk result collection
- Keep serde_json for cold paths (config loading)

---

## **🚀 AGGRESSIVE: Fork and Optimize Critical Dependencies**

### **Option A: Fork whisper-rs for Batch Inference**

**Problem**: whisper-rs processes one audio file at a time

**Optimization**: **Add batch inference support to whisper-rs**

```rust
// Proposed PR to whisper-rs:
impl WhisperContext {
    pub fn full_batch(&self, params: FullParams, audio_batch: &[&[f32]]) -> Result<Vec<WhisperState>> {
        // Process multiple audio files in one inference call
        // Amortizes model overhead, improves GPU utilization
    }
}
```

**Benefits**:
- **+30-40% throughput** for transcription-heavy bulk workloads
- GPU utilization: 40% → 80% (batch processing efficiency)
- **Bulk mode**: 5.0 → **6.0 files/sec** (+20%)

**Effort**: 2-3 commits (fork whisper-rs, implement batch API, PR upstream)
**Risk**: High (need to understand whisper.cpp internals, maintain fork)

**ROI**: **High** - Transcription is largest bottleneck (63% of pipeline time per benchmarks)

---

### **Option B: Fork ort for Zero-Copy Tensor API**

**Problem**: ort copies data into tensors (unnecessary copies)

**Optimization**: **Zero-copy tensor views**

```rust
// Proposed API improvement:
impl Session {
    // Current (copies data):
    pub fn run(&mut self, inputs: Vec<Value>) -> Result<Outputs> { ... }

    // New (zero-copy):
    pub fn run_zero_copy<'a>(&mut self, inputs: &'a [TensorView<'a>]) -> Result<Outputs> {
        // Run inference directly on user's memory (no copy)
    }
}
```

**Benefits**:
- **-20-30% memory usage** (no duplicate buffers)
- **+5-10% inference speed** (cache-friendly, no memcpy)
- **All ONNX models benefit**: Object detection, face detection, OCR, embeddings

**Effort**: 3-5 commits (fork ort, implement zero-copy API, PR upstream)
**Risk**: High (complex, need to understand ort internals)

**ROI**: **Medium** - Inference is GPU-bound, not memory-bound

---

## **📊 OPTIMIZATION IMPACT SUMMARY**

### **Quick Wins (1-2 commits each)**:

| Optimization | Effort | Binary Size | Compile Time | Runtime Speed | Bulk Throughput |
|--------------|--------|-------------|--------------|---------------|-----------------|
| **Custom tokenizer** | 1 commit | -5MB | -90s | +0% | +0% |
| **Image feature flags** | 1 line | -500KB | -10s | +0% | +0% |
| **mozjpeg** | 1 commit | -1MB | +0s | +3-5x JPEG | +5-10% |
| **FFTW** | 1 commit | +0 | +0s | +2-3x FFT | +5-7% |
| **ONNX optimization** | 1 commit | +0 | +0s | +10-50% inference | +15-25% |
| **simd-json** | 1 commit | +0 | +0s | +50-150% JSON | +2-3% |
| **Remove unused deps** | 10 min | -10MB | -50s | +0% | +0% |

**Total Quick Wins Impact**:
- Binary size: **-16MB** (37MB → 21MB, -43%)
- Compile time: **-150s** (~6min → ~3.5min, -42%)
- Bulk throughput: **+30-50%** (3.38 → 4.4-5.1 files/sec)

### **Aggressive (3-5 commits each)**:

| Optimization | Effort | Risk | Benefit | When |
|--------------|--------|------|---------|------|
| **whisper-rs batch** | 2-3 commits | High | +30-40% transcription | If transcription is primary bottleneck |
| **ort zero-copy** | 3-5 commits | High | +5-10% all inference | If profiling shows memcpy hotspot |
| **Custom JPEG decoder** | 3-4 commits | High | +5x JPEG vs pure Rust | If keyframe extraction is bottleneck |

---

## **🎯 RECOMMENDED OPTIMIZATION SEQUENCE**

### **Phase 4B: After Performance/Bulk Executors** (N=92-93)

**Step 1**: ONNX Runtime Optimization (1 commit)
- Add graph optimization, execution providers, memory patterns
- **Impact**: +15-25% bulk throughput
- **Effort**: 2 hours
- **Risk**: Low

**Step 2**: Remove Unused Dependencies (10 minutes)
- Delete: rayon, redis, async-nats, tantivy (if unused)
- **Impact**: -10MB binary, -50s compile
- **Effort**: 10 minutes
- **Risk**: None

**Step 3**: Image Feature Flags (1 line)
- Disable unused codecs
- **Impact**: -500KB binary
- **Effort**: 1 minute
- **Risk**: None

### **Phase 5: Targeted Optimizations** (N=94-95)

**Step 4**: mozjpeg Integration (1 commit)
- Replace image::jpeg with mozjpeg
- **Impact**: +3-5x JPEG decode, +5-10% bulk throughput
- **Effort**: 1 hour
- **Risk**: Low

**Step 5**: FFTW Integration (1 commit)
- Replace rustfft with FFTW
- **Impact**: +2-3x FFT, +5-7% audio-heavy workloads
- **Effort**: 1 hour
- **Risk**: Medium (system dependency)

**Step 6**: Custom Tokenizer (1 commit)
- Replace tokenizers with 100-line custom implementation
- **Impact**: -5MB binary, -90s compile, +0% runtime
- **Effort**: 2 hours
- **Risk**: Medium (validate against HuggingFace)

### **Phase 6: Advanced (If Needed)** (N=96-98)

**Step 7**: simd-json Hot Paths (1 commit)
- Use simd-json for plugin I/O
- **Impact**: +2-3% throughput
- **Effort**: 1 hour
- **Risk**: Low

**Step 8**: Whisper Batch Inference (2-3 commits)
- Fork whisper-rs, add batch API
- **Impact**: +30-40% transcription throughput
- **Effort**: 8-12 hours
- **Risk**: High (maintain fork)

---

## **EXPECTED PERFORMANCE AFTER ALL OPTIMIZATIONS**

| Metric | Baseline | After Quick Wins | After Targeted | After Advanced | Final Target |
|--------|----------|------------------|----------------|----------------|--------------|
| **Bulk Throughput** | 3.38 | 4.2 (+25%) | 5.0 (+48%) | 6.0 (+78%) | 5 files/sec ✅ |
| **Single File** | 5s | 4s (-20%) | 2.5s (-50%) | 2s (-60%) | 3s ✅ |
| **First Result** | 5s | 4s | 1s | 0.5s | 1s ✅ |
| **Binary Size** | 37MB | 21MB | 21MB | 21MB | <25MB ✅ |
| **Compile Time** | 6min | 3.5min | 3.5min | 4min | <5min ✅ |

**All targets achievable** ✅

---

## **PRIORITY RECOMMENDATION**

### **Implement in This Order**:

1. **ONNX Runtime Optimization** (N=92, after Performance/Bulk executors)
   - Highest impact/effort ratio
   - Affects all ML inference (50% of pipeline time)
   - Low risk

2. **mozjpeg Integration** (N=93)
   - JPEG is critical path (keyframes, object detection)
   - 3-5x speedup is massive
   - Low risk

3. **FFTW Integration** (N=94)
   - FFT is bottleneck in audio embeddings
   - 2-3x speedup significant
   - Medium risk (system dependency)

4. **Cleanup & Efficiency** (N=95)
   - Remove unused deps
   - Image feature flags
   - Custom tokenizer

5. **Advanced (If Needed)** (N=96+)
   - simd-json
   - Whisper batch inference (only if transcription remains bottleneck)

---

## **IMPLEMENTATION NOTES FOR WORKER**

### **ONNX Runtime Optimization Example**:

```rust
// Add to crates/video-extract-core/src/onnx_utils.rs:
use ort::{GraphOptimizationLevel, ExecutionProviderDispatch};

pub fn create_optimized_onnx_session(model_path: &Path) -> Result<Session> {
    Session::builder()?
        .with_optimization_level(GraphOptimizationLevel::All)?
        .with_intra_threads(num_cpus::get_physical())?
        .with_execution_providers([
            #[cfg(target_os = "macos")]
            ExecutionProviderDispatch::CoreML(Default::default()),

            #[cfg(feature = "cuda")]
            ExecutionProviderDispatch::CUDA(Default::default()),

            ExecutionProviderDispatch::CPU(Default::default()),
        ])?
        .with_memory_pattern(true)?
        .commit_from_file(model_path)
}
```

Update all plugins to use this helper.

### **mozjpeg Integration Example**:

```rust
// Add to crates/video-extract-core/src/image_io.rs:
use mozjpeg::Decompress;

pub fn load_jpeg_optimized(path: &Path) -> Result<RgbImage> {
    let data = std::fs::read(path)?;
    let d = Decompress::new_mem(&data)?;
    let rgb = d.rgb()?;
    let mut image = d.read_scanlines()?;

    Ok(RgbImage::from_raw(
        d.width() as u32,
        d.height() as u32,
        image.to_vec(),
    ).unwrap())
}
```

Use in keyframes, object-detection, face-detection, OCR plugins.

---

## **COST-BENEFIT ANALYSIS**

### **Best ROI** (Implement First):
1. ✅ **ONNX optimization** - 1 commit, +15-25% throughput, low risk
2. ✅ **mozjpeg** - 1 commit, +5-10% throughput, low risk
3. ✅ **Unused deps removal** - 10 min, -10MB binary, zero risk

### **Good ROI** (Implement If Time):
4. 🟡 **FFTW** - 1 commit, +5-7% audio workloads, medium risk
5. 🟡 **Custom tokenizer** - 1 commit, -5MB binary, medium risk
6. 🟡 **Image feature flags** - 1 line, -500KB binary, zero risk

### **Advanced ROI** (Implement Only If Benchmarks Show Need):
7. 🟢 **simd-json** - 1 commit, +2-3% throughput, low risk
8. 🟢 **Whisper batch** - 3 commits, +30-40% transcription-heavy, high risk

---

## **WORKER DIRECTIVE**

After completing Performance/Bulk executors (Phase 4), implement optimizations in this order:

**N=92**: Performance/Bulk executors
**N=93**: ONNX Runtime optimization + mozjpeg
**N=94**: Cleanup unused deps + image features
**N=95**: Benchmark and validate (compare to baseline)
**N=96+**: Additional optimizations if targets not met

**Target Achievement Confidence**: 95% with just steps 1-3 (ONNX + mozjpeg + cleanup)
