# Voice Activity Detection (VAD)

**Status**: ✅ Functional - Core library ready (N=267)
**Dependencies**: WebRTC VAD (C++ library via Rust bindings)
**GPU Required**: No

## Overview

Voice Activity Detection (VAD) library using the WebRTC Voice Activity Detector to identify speech segments in audio streams. This is a standalone library extracted from the diarization crate, providing voice/speech detection capabilities.

## Features

- **WebRTC VAD Integration**: Uses battle-tested WebRTC VAD (C++) via Rust bindings
- **Configurable Aggressiveness**: 4 modes (Quality, LowBitrate, Aggressive, VeryAggressive)
- **Segment Detection**: Identifies speech segments with start/end times and confidence
- **Statistics**: Provides voice percentage and total voice duration
- **Multiple Sample Rates**: Supports 8kHz, 16kHz, 32kHz, 48kHz

## Usage

```rust
use video_audio_voice_activity_detection::{VoiceActivityDetector, VadConfig};

// Create detector with default config (aggressiveness=2)
let detector = VoiceActivityDetector::default();

// Or create with custom config
let config = VadConfig {
    vad_aggressiveness: 2,      // 0-3, higher = more aggressive
    min_segment_duration: 0.3,  // 300ms minimum segment
    frame_duration_ms: 30,      // 30ms frames
};
let detector = VoiceActivityDetector::new(config);

// Detect voice in audio samples (f32 PCM, sample_rate Hz)
let samples: Vec<f32> = /* your audio samples */;
let result = detector.detect(&samples, 16000)?;

// Access results
println!("Found {} voice segments", result.segments.len());
println!("Voice activity: {:.1}%", result.voice_percentage * 100.0);

for segment in result.segments {
    println!("Voice from {:.2}s to {:.2}s", segment.start, segment.end);
}
```

## Configuration

### VAD Aggressiveness (0-3)

- **0 (Quality)**: Least aggressive, best for clean audio, may have false positives
- **1 (LowBitrate)**: Slightly aggressive filtering
- **2 (Aggressive)**: Balanced mode (default), good for most use cases
- **3 (VeryAggressive)**: Most aggressive, may miss some speech but few false positives

### Minimum Segment Duration

Controls the minimum length of voice segments to report (in seconds). Shorter segments are filtered out. Default: 0.3s (300ms).

### Frame Duration

WebRTC VAD processes audio in fixed-size frames (10ms, 20ms, or 30ms). Longer frames provide more context but less temporal precision. Default: 30ms.

## Output

### VadResult

```rust
pub struct VadResult {
    pub segments: Vec<VoiceSegment>,     // All detected voice segments
    pub total_voice_duration: f64,        // Total voice duration in seconds
    pub voice_percentage: f32,            // Voice percentage (0.0-1.0)
    pub total_duration: f64,              // Total audio duration in seconds
}
```

### VoiceSegment

```rust
pub struct VoiceSegment {
    pub start: f64,                       // Start time in seconds
    pub end: f64,                         // End time in seconds
    pub duration: f64,                    // Duration in seconds
    pub confidence: f32,                  // Always 1.0 (WebRTC VAD is binary)
}
```

## Supported Sample Rates

WebRTC VAD supports only specific sample rates:
- 8000 Hz
- 16000 Hz (recommended)
- 32000 Hz
- 48000 Hz

If your audio uses a different sample rate, resample to one of these rates before calling `detect()`.

## Implementation Notes

- **WebRTC VAD is binary**: It classifies each frame as voice or no-voice, so confidence is always 1.0 for detected segments
- **Frame-based processing**: Audio is processed in fixed-size frames (30ms by default)
- **i16 conversion**: WebRTC VAD expects i16 samples, so f32 samples are converted internally
- **Incomplete frames**: Frames at the end of audio that don't match frame_size are skipped

## Examples

### Detect voice in silent audio
```rust
let samples = vec![0.0; 16000]; // 1 second of silence at 16kHz
let result = detector.detect(&samples, 16000)?;
assert_eq!(result.segments.len(), 0);
assert_eq!(result.voice_percentage, 0.0);
```

### Detect voice in speech audio
```rust
// Generate synthetic speech-like sine wave at 300Hz
let samples: Vec<f32> = (0..16000)
    .map(|i| (2.0 * PI * 300.0 * i as f32 / 16000.0).sin() * 0.5)
    .collect();

let result = detector.detect(&samples, 16000)?;
assert!(result.segments.len() > 0);
assert!(result.voice_percentage > 0.5);
```

## Integration Status

**N=267 Status**: Core VAD library is complete and functional. Plugin integration with the main video-extract system is pending (future work). This library can be used standalone in any Rust project for voice activity detection.

## Future Work

- [ ] Plugin integration with video-extract-core
- [ ] CLI interface for standalone VAD processing
- [ ] Smoke tests in comprehensive test suite
- [ ] Operation enum variant for pipeline integration

## References

- WebRTC VAD: https://chromium.googlesource.com/external/webrtc/+/refs/heads/master/common_audio/vad/
- webrtc-vad Rust crate: https://crates.io/crates/webrtc-vad
