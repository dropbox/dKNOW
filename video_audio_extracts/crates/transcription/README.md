# Transcription Module

Speech-to-text transcription using Whisper.cpp via Rust bindings (whisper-rs).

## Features

- **Multiple Model Sizes**: Tiny, Base, Small, Medium, Large-v3
- **Word-Level Timestamps**: Extract precise timing for each word
- **Language Detection**: Auto-detect or specify language
- **Quality Scoring**: Calculate confidence scores for transcripts
- **Hardware Acceleration**: Metal (Apple Silicon), CUDA, VAAPI support
- **Configuration Presets**: Fast, Balanced, Accurate modes

## Status

**Core API**: ✅ Implemented and tested
**Audio Loading**: ✅ Integrated with audio-extractor module (auto-converts to 16kHz mono PCM)
**Integration Tests**: ✅ 6 unit tests passing

## Usage

```rust
use transcription::{Transcriber, TranscriptionConfig, WhisperModel};

// Load model with balanced preset
let config = TranscriptionConfig::balanced();
let transcriber = Transcriber::new("models/ggml-small.bin", config)?;

// Transcribe audio file (must be 16kHz mono PCM)
let transcript = transcriber.transcribe("audio.wav")?;

println!("Text: {}", transcript.text);
println!("Quality: {:.2}", transcript.quality_score);

for segment in transcript.segments {
    println!("[{:.2}s - {:.2}s]: {}", segment.start, segment.end, segment.text);
}
```

## Configuration Presets

### Fast (Tiny model)
- Model: Tiny (39M params)
- Beam size: 1
- Compute: Int8
- Use case: Real-time transcription, quick previews

### Balanced (Small model) - Default
- Model: Small (244M params)
- Beam size: 5
- Compute: Float16
- Use case: General-purpose transcription

### Accurate (Medium model)
- Model: Medium (769M params)
- Beam size: 10
- Compute: Float16
- Use case: High-accuracy transcription, archival

## Model Files

Whisper models must be downloaded separately. Expected format: GGML binary.

**Download locations**:
- Official: https://huggingface.co/ggerganov/whisper.cpp
- Models: tiny, base, small, medium, large-v3
- English-only variants available for faster processing

**Example download**:
```bash
mkdir -p models
cd models
wget https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin
```

## Completed Features

1. ✅ **Audio Loading Integration**: `load_audio_samples()` uses audio-extractor module for:
   - Converting any audio format to 16kHz mono PCM
   - Handling stereo to mono conversion
   - Automatic resampling to 16kHz

2. ✅ **Integration Tests**: 6 unit tests passing

## Future Enhancements

1. **Model Management**: Add model download/caching utilities

2. **Language Detection**: Extract actual language probability from Whisper state

3. **No-Speech Detection**: Extract no-speech probability from Whisper state

## Hardware Acceleration

The crate is configured with Metal support for Apple Silicon. To enable other backends:

```toml
# Cargo.toml
[dependencies]
whisper-rs = { version = "0.15", features = ["cuda"] }  # NVIDIA
whisper-rs = { version = "0.15", features = ["vulkan"] }  # Vulkan
```

## API Reference

See [AI_TECHNICAL_SPEC.md](../../AI_TECHNICAL_SPEC.md) section 3.6 for complete API specification.

## Testing

Run all tests (6 unit tests):
```bash
cargo test -p transcription
```

## Dependencies

- **whisper-rs**: Rust bindings for whisper.cpp
- **video-audio-common**: Shared types and errors
- **serde**: Serialization for configuration
- **tracing**: Logging
