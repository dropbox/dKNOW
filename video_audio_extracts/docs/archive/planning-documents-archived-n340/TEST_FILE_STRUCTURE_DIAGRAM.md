# Test File Structure - Inputs and Outputs

**Date**: 2025-10-30
**Purpose**: Visual diagram of test files and expected outputs

---

## Input Test Files

### test_edge_cases/ (14 files, 4.3MB total)
```
test_edge_cases/
├── audio_complete_silence_3sec__silence_detection.wav      (517K)
├── audio_hifi_96khz_24bit__quality_test.wav                (0B - empty)
├── audio_lowquality_16kbps__compression_test.mp3           (20K)
├── audio_mono_single_channel__channel_test.wav             (469K)
├── audio_very_short_1sec__duration_min.wav                 (94K)
├── corrupted_truncated_file__error_handling.mp4            (50K)
├── silent_audio.wav                                        (517K)
├── video_4k_ultra_hd_3840x2160__stress_test.mp4           (153K)
├── video_hevc_h265_modern_codec__compatibility.mp4        (157K)
├── video_high_fps_120__temporal_test.mp4                  (280K)
├── video_no_audio_stream__error_test.mov                  (1.7M)
├── video_single_frame_only__minimal.mp4                   (43K)
├── video_tiny_64x64_resolution__scaling_test.mp4          (36K)
└── video_variable_framerate_vfr__timing_test.mp4          (313K)
```

### test_files_local/ (3 files, 69MB total)
```
test_files_local/
├── audio_test.m4a                          (13M)
├── sample_10s_audio-aac.aac                (146K)
└── State of Affairs_ROUGHMIX.wav           (56M)
```

### ~/Desktop/stuff/stuff/ (Production files)
```
~/Desktop/stuff/stuff/
├── GMT20250520-223657_Recording_avo_1920x1080.mp4           (1.3GB)
├── Investor update - Calendar Agent - Oct 6.mp4             (349MB)
├── mission control video demo 720.mov                       (277MB)
├── relevance-annotations-first-pass (1).mov                 (97MB)
└── ... (more production videos)
```

---

## Output Structure

### For Single Test Run:

```
Input: test_edge_cases/video_variable_framerate_vfr__timing_test.mp4
Operation: keyframes,object-detection

Outputs Created:
├── debug_output/                           # Staged outputs (copied from /tmp)
│   ├── stage_00_keyframes.json            # 259B - Keyframe metadata
│   └── stage_01_object_detection.json     # 488B - Detection results
│
└── /tmp/video-extract/                    # Temp working directory
    ├── keyframes/
    │   └── video_variable_framerate_vfr__timing_test/
    │       └── keyframes/
    │           ├── frame_00000001.jpg      # 640x480 thumbnail
    │           ├── frame_00000002.jpg
    │           └── ...
    ├── audio/                              # If audio extraction
    │   └── *.wav
    └── object-detection/                   # If detection
        └── *.json
```

---

## Output File Formats

### 1. Keyframes JSON (stage_00_keyframes.json)
```json
[
  {
    "frame_number": 1,
    "timestamp": 0.033,
    "hash": 0,
    "sharpness": 0.0,
    "thumbnail_paths": {
      "640x480": "/tmp/video-extract/keyframes/.../frame_00000001.jpg"
    }
  },
  {
    "frame_number": 31,
    "timestamp": 1.033,
    "hash": 0,
    "sharpness": 0.0,
    "thumbnail_paths": {
      "640x480": "/tmp/video-extract/keyframes/.../frame_00000031.jpg"
    }
  }
]
```

### 2. Object Detection JSON (stage_01_object_detection.json)
```json
[
  {
    "frame_path": "/tmp/video-extract/keyframes/.../frame_00000001.jpg",
    "detections": [
      {
        "class": "person",
        "confidence": 0.85,
        "bbox": {
          "x": 0.1,
          "y": 0.2,
          "width": 0.3,
          "height": 0.4
        }
      }
    ]
  }
]
```

### 3. Transcription JSON (stage_00_transcription.json)
```json
{
  "language": "en",
  "language_probability": 0.984,
  "segments": [
    {
      "start": 0.0,
      "end": 0.9,
      "text": "Shakespeare on Scenery by Oscar Wilde...",
      "words": [
        {
          "word": "Shakespeare",
          "start": 0.99,
          "end": 1.64,
          "probability": 0.732
        }
      ]
    }
  ]
}
```

### 4. Audio Embeddings JSON (stage_01_audio_embeddings.json)
```json
{
  "embeddings": [
    {
      "start_time": 0.0,
      "end_time": 10.0,
      "embedding": [0.123, -0.456, 0.789, ...],  // 512-dim vector
      "dimension": 512
    }
  ]
}
```

### 5. Diarization JSON (stage_01_diarization.json)
```json
{
  "segments": [
    {
      "start": 0.0,
      "end": 3.5,
      "speaker": "Speaker 1"
    },
    {
      "start": 3.5,
      "end": 7.8,
      "speaker": "Speaker 2"
    }
  ]
}
```

### 6. Face Detection JSON (stage_01_face_detection.json)
```json
[
  {
    "frame_path": ".../frame_00000001.jpg",
    "faces": [
      {
        "confidence": 0.99,
        "bbox": {"x": 0.2, "y": 0.1, "width": 0.15, "height": 0.2},
        "landmarks": {
          "left_eye": [0.22, 0.15],
          "right_eye": [0.28, 0.15],
          "nose": [0.25, 0.20],
          "left_mouth": [0.23, 0.25],
          "right_mouth": [0.27, 0.25]
        }
      }
    ]
  }
]
```

### 7. OCR JSON (stage_00_ocr.json)
```json
[
  {
    "frame_path": ".../frame_00000001.jpg",
    "text_regions": [
      {
        "text": "Hello World",
        "confidence": 0.95,
        "bbox": {"x": 0.1, "y": 0.5, "width": 0.3, "height": 0.05}
      }
    ]
  }
]
```

### 8. Scene Detection JSON (stage_00_scene_detection.json)
```json
{
  "scenes": [
    {
      "scene_number": 1,
      "start_time": 0.0,
      "end_time": 5.2,
      "start_frame": 0,
      "end_frame": 156,
      "score": 0.45
    }
  ],
  "duration": 30.0
}
```

### 9. Vision Embeddings JSON (stage_01_vision_embeddings.json)
```json
[
  {
    "frame_path": ".../frame_00000001.jpg",
    "embedding": [0.012, -0.345, ...],  // 512-dim CLIP vector
    "dimension": 512
  }
]
```

### 10. Text Embeddings JSON (stage_01_text_embeddings.json)
```json
{
  "embeddings": [
    {
      "text": "Shakespeare on Scenery by Oscar Wilde...",
      "embedding": [0.234, -0.567, ...],  // 384-dim vector
      "dimension": 384,
      "start_time": 0.0,
      "end_time": 0.9
    }
  ]
}
```

### 11. Audio Files (stage_00_audio_extraction.wav)
```
Format: WAV (PCM)
Sample rate: 16000 Hz (configurable)
Channels: 1 (mono, configurable)
Bit depth: 16-bit
Size: Varies (159MB for 56MB input in example)
```

### 12. JPEG Files (/tmp/video-extract/keyframes/.../frame_*.jpg)
```
Format: JPEG
Resolution: 640x480 (default, configurable)
Quality: 95 (mozjpeg)
Naming: frame_00000001.jpg, frame_00000002.jpg, ...
Size: ~250-300KB per frame (4K video)
```

---

## Test Execution Flow

### Example: Keyframes + Object Detection Test

**Input:**
```
test_edge_cases/video_variable_framerate_vfr__timing_test.mp4 (313KB)
```

**Command:**
```bash
./target/release/video-extract debug --ops keyframes,object-detection input.mp4
```

**Process:**
```
1. Stage 0: Keyframe Extraction
   ├─> Decode I-frames (C FFI)
   ├─> Save to /tmp/video-extract/keyframes/.../frame_*.jpg
   └─> Write stage_00_keyframes.json (metadata)

2. Stage 1: Object Detection
   ├─> Load YOLOv8 model
   ├─> Run inference on each keyframe
   └─> Write stage_01_object_detection.json (detections)

3. Copy outputs to debug_output/
   ├─> stage_00_keyframes.json
   └─> stage_01_object_detection.json
```

**Outputs:**
```
debug_output/
├── stage_00_keyframes.json          (259B)
└── stage_01_object_detection.json   (488B)

/tmp/video-extract/keyframes/.../keyframes/
├── frame_00000001.jpg               (~300KB)
└── frame_00000002.jpg               (~300KB)
```

---

## Test Result Validation

**What tests check:**
```rust
let result = run_video_extract("keyframes,object-detection", &file);

// ✅ Checks:
assert!(result.passed);              // Process succeeded
assert!(result.duration_secs < 10.0); // Performance threshold (some tests)

// ❌ Doesn't check:
// - stage_00_keyframes.json contains 2 frames
// - frame_00000001.jpg is 640x480
// - stage_01_object_detection.json has expected detections
```

**With snapshot testing (your directive), will check:**
- ✅ JSON content matches baseline
- ✅ JPEG checksums match baseline
- ✅ Detect any output changes

---

## File Size Summary

| Output Type | Typical Size | Location | Format |
|-------------|--------------|----------|--------|
| Keyframes JSON | 200-500B | debug_output/ | JSON metadata |
| Object Detection JSON | 100B-5KB | debug_output/ | JSON (detections array) |
| Transcription JSON | 1-10KB | debug_output/ | JSON (segments + words) |
| Audio Embeddings JSON | 10-50KB | debug_output/ | JSON (512-dim vectors) |
| Vision Embeddings JSON | 5-20KB | debug_output/ | JSON (512-dim vectors) |
| Text Embeddings JSON | 5-15KB | debug_output/ | JSON (384-dim vectors) |
| Diarization JSON | 1-300KB | debug_output/ | JSON (speaker segments) |
| Face Detection JSON | 1-50KB | debug_output/ | JSON (faces + landmarks) |
| OCR JSON | 1-20KB | debug_output/ | JSON (text regions) |
| Scene Detection JSON | 200B-2KB | debug_output/ | JSON (scene timestamps) |
| Audio WAV | 10MB-200MB | debug_output/ | WAV file (16kHz mono) |
| Keyframe JPEGs | 100KB-500KB each | /tmp/video-extract/ | JPEG (640x480) |

**Total per test**: Typically 10-50MB (mostly audio WAV + JPEGs)

---

## Example: Full Pipeline Test

**Input:**
```
~/Desktop/stuff/stuff/Investor update - Calendar Agent - Oct 6.mp4 (349MB)
```

**Operations:**
```
keyframes → object-detection → vision-embeddings
audio → transcription → text-embeddings → diarization
```

**Outputs Created:**
```
debug_output/
├── stage_00_keyframes.json              # 20 keyframes metadata
├── stage_00_audio_extraction.wav        # 53MB (16kHz mono)
├── stage_01_object_detection.json       # Detections on 20 frames
├── stage_01_transcription.json          # Speech-to-text
├── stage_02_vision_embeddings.json      # 20 × 512-dim vectors
├── stage_02_text_embeddings.json        # Transcript embeddings
└── stage_03_diarization.json            # Speaker segments

/tmp/video-extract/keyframes/.../keyframes/
├── frame_00000001.jpg                   # 250KB
├── frame_00000002.jpg                   # 250KB
└── ... (20 frames)

Total output: ~58MB (53MB audio + 5MB JPEGs + metadata)
```

---

## Snapshot Testing: What Gets Captured

**Per test, capture:**
```
test_results/2025-10-30_22-45-13_6a8f2e1/outputs/format_mp4_quick_pipeline/
├── stage_00_keyframes.json              # COPY (small)
├── stage_01_object_detection.json       # COPY (small)
├── stage_00_audio_extraction.wav        # CHECKSUM ONLY (large)
├── keyframes/
│   ├── checksums.txt                    # SHA256 of each JPEG
│   └── count.txt                        # Number of frames
└── metadata.txt                         # Test-specific metadata
```

**Storage per test**: ~10-50KB (JSONs + checksums, not full files)
**Storage per run (98 tests)**: ~1-5MB
**Full files**: Kept in /tmp or debug_output temporarily, not archived

---

## Comparison Between Runs

**Baseline run:**
```
test_results/baseline/outputs/format_mp4_quick_pipeline/
└── stage_01_object_detection.json
    → {"frame_path": "...", "detections": [{"class": "person", "confidence": 0.85}]}
```

**Current run:**
```
test_results/latest/outputs/format_mp4_quick_pipeline/
└── stage_01_object_detection.json
    → {"frame_path": "...", "detections": [{"class": "person", "confidence": 0.82}]}
```

**Diff detected:**
```
⚠️  OUTPUT CHANGED:
  - stage_01_object_detection.json:
    detections[0].confidence: 0.85 → 0.82 (-0.03)
```

**You review:** Is this acceptable variance or a regression?

---

## Summary

**Input files:**
- test_edge_cases/: 14 files, 4.3MB (curated edge cases)
- test_files_local/: 3 files, 69MB (local copies)
- ~/Desktop/stuff/: Production videos (100MB-1.3GB)

**Output per test:**
- JSON files: 10-15 files, 10-50KB total (metadata)
- Audio WAV: 10-200MB (extracted audio)
- JPEG frames: 100-500KB each (keyframes)

**Snapshot captures:**
- JSON content (exact)
- File checksums (detect binary changes)
- Metadata (test context)

**Total snapshot storage:**
- Per test: ~10-50KB
- Per run: ~1-5MB (98 tests)
- Historical: Grows linearly with runs

**This enables tracking changes across commits without storing full outputs.**
