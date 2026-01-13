# IMPLEMENT TIER 1 FEATURES - 5 High-Value Capabilities

**Date**: 2025-10-31
**Authority**: USER directive
**Order**: "Implement all 5 of your features"

---

## THE 5 FEATURES TO IMPLEMENT

### 1. Motion Tracking (N=70-72, ~4 commits)
### 2. Action Recognition (N=73-75, ~3 commits)
### 3. Smart Thumbnail Generation (N=76-77, ~2 commits)
### 4. Subtitle Extraction (N=78-79, ~2 commits)
### 5. Audio Classification (N=80-81, ~2 commits)

**Total**: 13 commits, ~15-18 hours

---

## FEATURE 1: Motion Tracking

**What**: Track objects across video frames (persistent IDs)
**Use Case**: Follow person/car through video, trajectory analysis

**Implementation**:
```
Plugin: motion-tracking
Input: ObjectDetection (from object-detection plugin)
Output: MotionTracks (tracks with persistent IDs)
Tech: ByteTrack or BoT-SORT algorithm
Model: No new model needed (uses existing YOLOv8 detections)
```

**Algorithm**: ByteTrack (simple, effective, no extra model)
```rust
// crates/motion-tracking/src/lib.rs

pub struct Track {
    pub id: u32,
    pub class: String,
    pub detections: Vec<Detection>,  // Detections across frames
    pub start_frame: u32,
    pub end_frame: u32,
}

pub fn track_objects(detections_per_frame: Vec<Vec<Detection>>) -> Vec<Track> {
    // ByteTrack algorithm:
    // 1. High-confidence matching (IoU + Kalman filter)
    // 2. Low-confidence association
    // 3. Track lifecycle management
}
```

**Output JSON**:
```json
{
  "tracks": [
    {
      "id": 1,
      "class": "person",
      "frames": [
        {"frame": 0, "bbox": [0.1, 0.2, 0.3, 0.4], "confidence": 0.85},
        {"frame": 1, "bbox": [0.12, 0.21, 0.3, 0.4], "confidence": 0.87}
      ]
    }
  ]
}
```

**Estimated**: 4 commits, ~5 hours

---

## FEATURE 2: Action Recognition

**What**: Classify activities in video (walking, running, dancing, etc.)
**Use Case**: Sports analysis, security monitoring, content tagging

**Implementation**:
```
Plugin: action-recognition
Input: Keyframes or raw video
Output: ActionRecognition (class + confidence + temporal segments)
Tech: X3D or I3D model (3D CNN for video)
Model: kinetics-400 trained model (~50MB ONNX)
```

**Model**: X3D-M (efficient, accurate)
```
Download: https://github.com/onnx/models/tree/main/vision/body_analysis/action_recognition
Size: ~50MB
Classes: 400 action categories (Kinetics-400)
```

**Output JSON**:
```json
{
  "actions": [
    {
      "class": "walking",
      "confidence": 0.92,
      "start_time": 0.0,
      "end_time": 5.3
    },
    {
      "class": "waving",
      "confidence": 0.78,
      "start_time": 5.3,
      "end_time": 7.1
    }
  ]
}
```

**Estimated**: 3 commits, ~4 hours

---

## FEATURE 3: Smart Thumbnail Generation

**What**: Select best frame for video thumbnail (aesthetic quality)
**Use Case**: Video previews, galleries, YouTube-style thumbnails

**Implementation**:
```
Plugin: smart-thumbnail
Input: Keyframes
Output: BestThumbnail (single frame with quality score)
Tech: Aesthetic scorer + face detection + composition rules
Model: NIMA (Neural Image Assessment) ~20MB ONNX
```

**Algorithm**:
```rust
pub fn select_best_thumbnail(keyframes: &[Keyframe]) -> Keyframe {
    let mut scores = Vec::new();

    for frame in keyframes {
        let score =
            aesthetic_quality(frame) * 0.5 +      // NIMA model
            face_presence(frame) * 0.3 +          // Has faces (engaging)
            composition_score(frame) * 0.2;       // Rule of thirds, etc.

        scores.push((frame, score));
    }

    scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    scores[0].0.clone()
}
```

**Model**: NIMA or IAA (Image Aesthetic Assessment)
- Download: https://github.com/onnx/models or convert from PyTorch
- Size: ~20MB

**Estimated**: 2 commits, ~3 hours

---

## FEATURE 4: Subtitle Extraction

**What**: Extract embedded subtitles from video files
**Use Case**: Content indexing, translation, accessibility

**Implementation**:
```
Plugin: subtitle-extraction
Input: mp4, mkv, mov, avi (video files with embedded subs)
Output: Subtitles (SRT format + JSON)
Tech: FFmpeg libavformat (subtitle stream extraction)
```

**C FFI**: Use existing FFmpeg integration
```rust
pub fn extract_subtitles(video_path: &Path) -> Result<Vec<SubtitleSegment>> {
    unsafe {
        let format_ctx = FormatContext::open(video_path)?;

        // Find subtitle stream (AVMEDIA_TYPE_SUBTITLE)
        let subtitle_stream_idx = av_find_best_stream(
            format_ctx.ptr,
            AVMEDIA_TYPE_SUBTITLE,  // NEW constant
            -1, -1, ptr::null_mut(), 0
        );

        // Decode subtitle packets (ASS, SRT, VTT formats)
        while let Some(sub_packet) = read_subtitle_packet()? {
            parse_subtitle(sub_packet)?;
        }
    }
}
```

**Output JSON**:
```json
{
  "subtitles": [
    {
      "start": 0.0,
      "end": 2.5,
      "text": "Hello, world!",
      "format": "srt"
    }
  ]
}
```

**Estimated**: 2 commits, ~2-3 hours

---

## FEATURE 5: Audio Classification

**What**: Classify audio type (music, speech, silence, applause, etc.)
**Use Case**: Content tagging, audio search, highlight detection

**Implementation**:
```
Plugin: audio-classification
Input: Audio (WAV file)
Output: AudioClassification (class + confidence + temporal segments)
Tech: YAMNet or PANNs (general audio classifier)
Model: YAMNet ~4MB or PANNs ~80MB (ONNX)
```

**Model**: YAMNet (efficient, 521 audio event classes)
```
Download: https://tfhub.dev/google/yamnet/1 (convert to ONNX)
Size: ~4MB
Classes: 521 audio events (speech, music, dog bark, applause, etc.)
Input: 16kHz mono audio
```

**Output JSON**:
```json
{
  "segments": [
    {
      "start": 0.0,
      "end": 5.2,
      "class": "speech",
      "confidence": 0.94
    },
    {
      "start": 5.2,
      "end": 8.7,
      "class": "music",
      "confidence": 0.88
    },
    {
      "start": 8.7,
      "end": 10.0,
      "class": "applause",
      "confidence": 0.76
    }
  ]
}
```

**Estimated**: 2 commits, ~2-3 hours

---

## IMPLEMENTATION ORDER

**Sequential implementation** (easiest to hardest):

1. **Subtitle extraction** (N=70-71, 2-3h)
   - Uses existing FFmpeg C FFI
   - No new models needed
   - Quick win

2. **Audio classification** (N=72-73, 2-3h)
   - Small model (4MB YAMNet)
   - Standard ONNX inference pattern
   - Reuses audio extraction

3. **Smart thumbnails** (N=74-75, 3h)
   - Moderate complexity
   - Need NIMA model (~20MB)
   - Reuses keyframes and face detection

4. **Action recognition** (N=76-78, 4h)
   - Complex: video-level analysis
   - Large model (50MB X3D)
   - New input type (video clips)

5. **Motion tracking** (N=79-82, 5h)
   - Most complex: temporal state management
   - No model but algorithm is involved
   - Requires careful testing

**Total**: 13 commits, ~17 hours

---

## TESTING REQUIREMENTS

**Each feature must have:**
1. Unit tests (algorithm correctness)
2. Integration test (plugin works)
3. Smoke test entry (fast validation)
4. Full test suite entry (comprehensive)
5. Benchmark (performance measurement)

**Add to smoke_test.rs:**
```rust
#[test]
fn smoke_subtitle_extraction() {
    // Test subtitle extraction on known file
}

#[test]
fn smoke_audio_classification() {
    // Test audio classification on speech/music
}
// ... etc
```

---

## MODEL DOWNLOADS

**Worker N=70 must download:**
1. YAMNet (4MB) - Audio classification
2. NIMA or IAA (20MB) - Aesthetic quality
3. X3D-M (50MB) - Action recognition

**Total new models**: ~74MB
**Storage**: models/[feature]/*.onnx

---

## SUCCESS CRITERIA

**After N=82:**
- ✅ 5 new plugins operational
- ✅ 5 new operations in CLI
- ✅ All smoke tests include new features
- ✅ Full test suite updated
- ✅ Documentation complete
- ✅ Benchmark baselines established

**Result**: 16 total plugins (was 11), comprehensive media analysis

---

## WORKER INSTRUCTIONS

**N=70**: Start with subtitle extraction (easiest)
**N=71**: Add subtitle smoke test
**N=72-73**: Audio classification + test
**N=74-75**: Smart thumbnails + test
**N=76-78**: Action recognition + test
**N=79-82**: Motion tracking + test

**Follow plugin architecture:**
- Create crate: crates/[feature]/
- Implement plugin.yaml
- Add to registry
- Write tests
- Document in README

**Maintain standards:**
- 100% Rust/C++ (no Python)
- ONNX Runtime for models
- Production quality code
- Full test coverage
