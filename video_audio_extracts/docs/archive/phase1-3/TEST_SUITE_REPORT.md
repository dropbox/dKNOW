# Video & Audio Extraction System - Test Suite Report
## Media Files Inventory

**Generated**: 2024-10-27
**Total Media Files Found**: 47,860
**System Scanned**: /Users/ayates

---

## EXECUTIVE SUMMARY

This report catalogs all audio and video files available on the system for testing the video and audio extraction system. The test suite includes a diverse range of media formats, durations, resolutions, and content types—from small 10-second test clips to large multi-gigabyte recordings.

### Test Coverage

- **Total Files**: 47,860 audio and video files
- **Video Files**: ~18,500+ (primarily MP4, MOV)
- **Audio Files**: ~29,000+ (MP3, M4A, WAV, FLAC, AAC, OGG)
- **Size Range**: 88 KB (silent 1s audio) → 1.8 GB (screen recording)
- **Duration Range**: 1 second → ~90+ minutes
- **Content Types**: Screen recordings, Zoom meetings, dataset videos, test clips, music

---

## 1. TEST FILE CATEGORIES

### 1.1 Professional/Production Test Files (Desktop)

**Location**: `/Users/ayates/Desktop/stuff/`
**Count**: 18 files
**Total Size**: ~4.5 GB
**Formats**: MP4, MOV, M4A

#### Large Files (Good for Performance Testing)

| File | Size | Type | Description |
|------|------|------|-------------|
| Screen Recording 2025-09-11 at 8.49.02 AM.mov | 1.8 GB | Video | Screen recording (largest test file) |
| GMT20250520-223657_Recording_avo_1920x1080.mp4 | 1.3 GB | Video | Zoom recording, 1920x1080 |
| GMT20250516-190317_Recording_avo_1920x1080 braintrust.mp4 | 980 MB | Video | Zoom recording, 1920x1080 |
| Investor update - Calendar Agent - Oct 6.mp4 | 349 MB | Video | Presentation recording |
| Slack - Rajkumar Janakiraman (DM) - Dropbox - Slack 2024-10-20 at 9.30.23 PM.mp4 | 284 MB | Video | Screen recording |
| mission control video demo 720.mov | 277 MB | Video | Demo video, 720p |

**Test Value**: Large files for stress testing, real-world Zoom recordings with speech, screen content for OCR testing, presentation slides for visual detection.

#### Medium Files (Balanced Testing)

| File | Size | Type | Description |
|------|------|------|-------------|
| Summarize feedback from Slack This week - June 30 demo - Screen Recording 2025-06-27 at 5.50.23 PM.mov | 111 MB | Video | Demo recording |
| relevance-annotations-first-pass (1).mov | 97 MB | Video | Annotation demo |
| relevance-annotations-first-pass.mov | 97 MB | Video | Annotation demo (duplicate) |
| review existing benchmarks/gonzolo meeting aug 14/video1171640589.mp4 | 89 MB | Video | Meeting recording |
| demo videos fixing chat/show this Screen summarize slacks this week Recording 2025-06-27 at 5.25.25 PM.mov | 77 MB | Video | Product demo |
| review existing benchmarks/april meeting conv ai dashboard 2025-08-14 17.42.25 Zoom Meeting/video1509128771.mp4 | 75 MB | Video | Zoom meeting |

**Test Value**: Realistic file sizes for typical meetings and demos, speaker diarization testing, presentation content.

#### Small Files (Quick Testing)

| File | Size | Type | Description |
|------|------|------|-------------|
| Screen Recording 2025-06-02 at 11.14.26 AM.mov | 38 MB | Video | Short screen recording |
| May 5 - live labeling mocks.mp4 | 38 MB | Video | UI demo |
| editing-relevance-rubrics kg may 16 2025.mov | 34 MB | Video | Editing demo |
| demo videos fixing chat/blazers from gdrive - Screen Recording 2025-06-27 at 5.44.57 PM.mov | 32 MB | Video | Short demo |

**Test Value**: Fast iteration testing, quick validation.

#### Audio-Only Files

| File | Size | Type | Description |
|------|------|------|-------------|
| review existing benchmarks/gonzolo meeting aug 14/audio1171640589.m4a | 19 MB | Audio | Meeting audio |
| review existing benchmarks/april meeting conv ai dashboard 2025-08-14 17.42.25 Zoom Meeting/audio1509128771.m4a | 13 MB | Audio | Meeting audio |

**Test Value**: Pure audio transcription testing, speaker diarization without video.

---

### 1.2 Standardized Test Clips (Docling)

**Location**: `/Users/ayates/docling/tests/data/audio/`
**Count**: 16 files
**Total Size**: ~12 MB
**Formats**: MP3, MP4, M4A, WAV, FLAC, AAC, OGG, MOV, AVI

#### Purpose-Built Test Files (10-Second Samples)

| File | Size | Format | Description |
|------|------|--------|-------------|
| sample_10s_video-x-msvideo.avi | 2.7 MB | AVI | Video, x-msvideo codec |
| sample_10s_video-avi.avi | 2.5 MB | AVI | Video, standard AVI |
| sample_10s_audio-x-wav.wav | 1.7 MB | WAV | Audio, x-wav variant |
| sample_10s_audio-wav.wav | 1.7 MB | WAV | Audio, standard WAV |
| sample_10s_audio-x-flac.flac | 716 KB | FLAC | Audio, x-flac variant |
| sample_10s_audio-flac.flac | 716 KB | FLAC | Audio, lossless |
| sample_10s_video-quicktime.mov | 316 KB | MOV | Video, QuickTime |
| sample_10s_video-mp4.mp4 | 315 KB | MP4 | Video, standard MP4 |
| sample_10s_audio-mpeg.mp3 | 160 KB | MP3 | Audio, MPEG format |
| sample_10s_audio-mp3.mp3 | 160 KB | MP3 | Audio, standard MP3 |
| sample_10s.mp3 | 160 KB | MP3 | Audio, simple |
| sample_10s_audio-mp4.m4a | 148 KB | M4A | Audio, MP4 container |
| sample_10s_audio-m4a.m4a | 148 KB | M4A | Audio, standard M4A |
| sample_10s_audio-aac.aac | 146 KB | AAC | Audio, raw AAC |
| sample_10s_audio-ogg.ogg | 116 KB | OGG | Audio, Ogg Vorbis |
| silent_1s.wav | 88 KB | WAV | Silent audio (1 second) |

**Test Value**:
- **Format Coverage**: Tests all major audio/video formats
- **Consistency**: All 10-second clips for uniform testing
- **Codec Variants**: Tests different codec implementations (x-wav vs wav, x-flac vs flac)
- **Edge Cases**: Silent audio file for silence detection
- **Fast Execution**: Small files for rapid iteration

---

### 1.3 Kinetics-600 Dataset (5% Subset)

**Location**: `/Users/ayates/Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/Kinetics dataset (5%)/kinetics600_5per/kinetics600_5per/train/`
**Count**: 18,288 video files
**Format**: MP4
**Categories**: 600 action classes
**File Size Range**: ~200 KB - 3.2 MB per video
**Duration**: ~10 seconds per video

#### Dataset Overview

The Kinetics-600 dataset is a large-scale benchmark for human action recognition from videos. This is a 5% subset containing ~30 videos per action class.

**Sample Action Categories** (30 of 600):
- abseiling
- acting in play
- adjusting glasses
- air drumming
- alligator wrestling
- answering questions
- applauding
- applying cream
- archaeological excavation
- archery
- arguing
- arm wrestling
- arranging flowers
- assembling bicycle
- assembling computer
- attending conference
- auctioning
- backflip (human)
- baking cookies
- bandaging
- barbequing
- bartending
- base jumping
- bathing dog
- battle rope training
- beatboxing
- bee keeping
- belly dancing
- bench pressing
- bending back

**Additional Categories Found in Search**:
- carving ice (26 videos)
- picking fruit (27 videos)
- sticking tongue out (30 videos)
- home roasting coffee (24 videos)
- crossing eyes (26 videos)
- diving cliff (22 videos)

**Test Value**:
- **Scale Testing**: 18,288 videos for batch processing validation
- **Action Detection**: Ground truth for 600 human actions
- **Visual Diversity**: Diverse scenes, lighting, camera angles
- **Object Detection**: Various objects in natural contexts
- **Consistency**: Uniform 10-second duration for benchmarking
- **Throughput Testing**: Ideal for measuring videos processed per hour

**Dataset Statistics**:
```
Total Videos: 18,288
Action Categories: 600
Videos per Category: ~30
Average File Size: ~800 KB
Total Dataset Size: ~14.6 GB
```

**Recommended Use Cases**:
1. **Bulk Processing API Testing**: Process entire dataset to validate throughput claims
2. **Action Detection Evaluation**: Compare detected actions against ground truth labels
3. **Scene Detection**: Validate scene boundaries on short clips
4. **Object Detection**: Test YOLO on diverse real-world scenes
5. **Performance Benchmarking**: Measure processing speed across 18K videos
6. **GPU Batching**: Test batch sizes (32, 64, 128) for optimal throughput

---

### 1.4 Small Test Files (LangChain, pdfium, Libraries)

**Locations**: Various library test directories
**Count**: ~20-30 files
**Purpose**: Unit test fixtures

#### Audio Test Files
- `/Users/ayates/langchain_rs/libs/partners/openai/tests/integration_tests/chat_models/audio_input.wav`
- `/Users/ayates/langchain/libs/partners/openai/tests/integration_tests/chat_models/audio_input.wav`
- `/Users/ayates/pdfium/third_party/depot_tools/external_bin/gsutil/gsutil_4.68/gsutil/gslib/tests/test_data/test.mp3`

**Test Value**: Small fixtures for CI/CD integration tests.

---

### 1.5 System/Application Files

**Location**: `/Users/ayates/Library/Application Support/`
**Count**: ~29,000+ files
**Examples**:
- Zoom waiting room videos
- Chrome extension audio (Loom countdown, pause, complete sounds)
- System sounds

**Test Value**: Minimal—mostly UI sounds and app resources. Not recommended for primary testing.

---

## 2. RECOMMENDED TEST SUITE STRUCTURE

### 2.1 Unit Tests (Fast Execution)

**Files to Use**: Docling test clips (16 files, ~12 MB)

**Coverage**:
- All major audio formats (MP3, M4A, WAV, FLAC, AAC, OGG)
- All major video formats (MP4, MOV, AVI)
- Silent audio edge case
- Consistent 10-second duration

**Execution Time**: < 1 minute (CPU-only), < 30 seconds (GPU)

**Validation**:
- Format detection accuracy
- Metadata extraction completeness
- Audio extraction success rate
- Transcription (if audio contains speech)
- Keyframe extraction consistency

---

### 2.2 Integration Tests (Realistic Workloads)

**Files to Use**: Desktop production files (18 files, ~4.5 GB)

**Test Scenarios**:

#### A. Small File Tests (30-40 MB)
**Files**:
- editing-relevance-rubrics kg may 16 2025.mov (34 MB)
- May 5 - live labeling mocks.mp4 (38 MB)
- Screen Recording 2025-06-02 at 11.14.26 AM.mov (38 MB)

**Validates**: Real-time API performance, quick processing

#### B. Medium File Tests (70-120 MB)
**Files**:
- review existing benchmarks/april meeting conv ai dashboard 2025-08-14 17.42.25 Zoom Meeting/video1509128771.mp4 (75 MB)
- review existing benchmarks/gonzolo meeting aug 14/video1171640589.mp4 (89 MB)
- Summarize feedback from Slack This week - June 30 demo - Screen Recording 2025-06-27 at 5.50.23 PM.mov (111 MB)

**Validates**:
- Speaker diarization (Zoom meetings with multiple speakers)
- Screen content OCR (presentations, dashboards)
- Scene detection (meeting phases: intro, presentation, Q&A)
- Face detection (meeting participants)

#### C. Large File Tests (300MB - 1.8GB)
**Files**:
- Investor update - Calendar Agent - Oct 6.mp4 (349 MB)
- GMT20250516-190317_Recording_avo_1920x1080 braintrust.mp4 (980 MB)
- GMT20250520-223657_Recording_avo_1920x1080.mp4 (1.3 GB)
- Screen Recording 2025-09-11 at 8.49.02 AM.mov (1.8 GB)

**Validates**:
- Memory management (multi-GB files)
- Long-duration transcription accuracy
- Extended meeting diarization
- Resource efficiency

#### D. Audio-Only Tests
**Files**:
- audio1171640589.m4a (19 MB)
- audio1509128771.m4a (13 MB)

**Validates**:
- Audio-only pipeline
- Transcription without video
- Waveform generation
- Audio event detection

---

### 2.3 Performance/Stress Tests (Scale Validation)

**Files to Use**: Kinetics-600 dataset subset (18,288 videos, ~14.6 GB)

**Test Scenarios**:

#### A. Throughput Benchmarking
**Goal**: Measure videos processed per hour

**Test Sets**:
- **Small batch**: 100 videos (carving ice + picking fruit + sticking tongue out + crossing eyes)
- **Medium batch**: 1,000 videos (random selection)
- **Large batch**: 10,000 videos (full dataset processing)
- **Full dataset**: All 18,288 videos

**Metrics to Measure**:
- Total processing time
- Videos processed per hour
- CPU utilization (%)
- GPU utilization (%)
- RAM usage (peak and average)
- VRAM usage (peak and average)
- Throughput multiplier vs. real-time

#### B. GPU Batching Optimization
**Goal**: Find optimal batch sizes

**Test Configurations**:
- Object detection batch sizes: 16, 32, 64, 128
- Transcription batch sizes: 4, 8, 16, 32
- Embedding batch sizes: 64, 128, 256, 512

**Categories to Use** (consistent video characteristics):
- "carving ice" (26 videos)
- "home roasting coffee" (24 videos)
- "crossing eyes" (26 videos)

**Measure**: Throughput (videos/hour) vs. batch size

#### C. Concurrent Processing
**Goal**: Test parallel job execution

**Test**:
- Submit 100 videos simultaneously to Real-Time API
- Submit 1000 videos as bulk batch
- Mix: 50 real-time + 500 bulk

**Validate**:
- Queue management
- Resource contention
- Job prioritization
- Error handling under load

#### D. CPU-Only vs GPU Comparison
**Goal**: Quantify GPU speedup

**Test Sets**:
- 100 videos from "picking fruit" category
- Process with CPU-only mode
- Process with GPU-accelerated mode
- Compare total time and per-video time

---

### 2.4 Accuracy/Quality Tests

**Files to Use**: Desktop meetings + Kinetics samples

**Test Cases**:

#### A. Transcription Accuracy
**Files**:
- Zoom meeting recordings (clear speech)
- Screen recording demos (narration)

**Validation**:
- Manual review of 10 video transcripts
- Check for major errors
- Validate word timestamps (< 100ms drift)
- Check punctuation quality

#### B. Speaker Diarization
**Files**:
- Zoom meetings with 2+ speakers
- audio1171640589.m4a (meeting audio)

**Validation**:
- Count detected speakers vs. actual
- Check speaker change accuracy
- Validate timeline consistency

#### C. Object Detection
**Files**: Kinetics-600 subset

**Validation**:
- Compare detected actions against ground truth labels
- Calculate precision/recall for top 10 action categories
- Validate bounding boxes on sample frames

#### D. Scene Detection
**Files**:
- Long meetings (multiple scene changes)
- Kinetics videos (single scene)

**Validation**:
- Check for over-segmentation (too many scenes)
- Check for under-segmentation (missed scenes)
- Validate scene boundaries align with visual changes

---

## 3. TEST SUITE EXECUTION PLAN

### Phase 1: Smoke Tests (5 minutes)
**Files**: Docling 10-second clips (16 files)
**Goal**: Verify all formats work
**Mode**: CPU-only

**Commands**:
```bash
# Process all docling test files
for file in /Users/ayates/docling/tests/data/audio/sample_10s_*; do
    ./video-processor process --input "$file" --mode cpu_only
done
```

**Expected Results**:
- All 16 files process successfully
- No crashes or errors
- Metadata extracted for all files
- Keyframes/waveforms generated

---

### Phase 2: Integration Tests (30 minutes)
**Files**: 6 production videos (small + medium size)
**Goal**: Validate full pipeline with realistic content
**Mode**: GPU-accelerated

**Test Set**:
1. editing-relevance-rubrics kg may 16 2025.mov (34 MB)
2. May 5 - live labeling mocks.mp4 (38 MB)
3. demo videos fixing chat/blazers from gdrive - Screen Recording 2025-06-27 at 5.44.57 PM.mov (32 MB)
4. review existing benchmarks/april meeting conv ai dashboard 2025-08-14 17.42.25 Zoom Meeting/video1509128771.mp4 (75 MB)
5. review existing benchmarks/gonzolo meeting aug 14/video1171640589.mp4 (89 MB)
6. audio1171640589.m4a (19 MB)

**Commands**:
```bash
# Real-time API mode
curl -X POST http://localhost:8080/api/v1/process/realtime \
  -H "Content-Type: application/json" \
  -d '{
    "source": {"type": "file", "location": "/path/to/video.mp4"},
    "processing": {
      "priority": "realtime",
      "required_features": ["transcription", "keyframes", "scenes", "objects"],
      "quality_mode": "balanced"
    }
  }'
```

**Validation**:
- Transcription quality (manually review 2-3 transcripts)
- Speaker diarization (verify speaker count)
- OCR (check for on-screen text detection in screen recordings)
- Timeline generation (validate event synchronization)

---

### Phase 3: Performance Tests (2-4 hours)
**Files**: Kinetics-600 subset (1,000 videos)
**Goal**: Measure throughput and optimize
**Mode**: Bulk API

**Test Set**: Random 1,000 videos from Kinetics dataset

**Commands**:
```bash
# Generate batch request
python scripts/create_batch_request.py \
  --input-dir "/Users/ayates/Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/Kinetics dataset (5%)/kinetics600_5per/kinetics600_5per/train" \
  --num-files 1000 \
  --output batch_request.json

# Submit batch
curl -X POST http://localhost:8080/api/v1/process/bulk \
  -H "Content-Type: application/json" \
  -d @batch_request.json
```

**Metrics to Capture**:
- Total processing time
- Videos per hour throughput
- Average processing time per video
- CPU utilization (%)
- GPU utilization (%)
- Memory usage (RAM and VRAM)

**Baseline Targets** (to be measured, not enforced):
- Videos per hour: TBD
- CPU efficiency: TBD
- GPU efficiency: TBD

---

### Phase 4: Stress Tests (8-12 hours)
**Files**: Full Kinetics-600 subset (18,288 videos)
**Goal**: Validate scale and stability
**Mode**: Bulk API

**Test**: Process entire Kinetics dataset

**Commands**:
```bash
# Process all Kinetics videos
python scripts/process_kinetics_full.py \
  --input-dir "/Users/ayates/Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/Kinetics dataset (5%)/kinetics600_5per/kinetics600_5per/train" \
  --api-url http://localhost:8080/api/v1/process/bulk
```

**Monitoring**:
- Track progress (videos completed / total)
- Watch for memory leaks (increasing RAM over time)
- Monitor error rates
- Check for failures or crashes

**Expected Behavior**:
- No crashes
- Consistent throughput over time
- < 1% error rate
- Successful completion of all 18,288 videos

---

### Phase 5: Large File Tests (2 hours)
**Files**: 4 largest files (350MB - 1.8GB)
**Goal**: Validate memory management
**Mode**: Real-time API

**Test Set**:
1. Investor update - Calendar Agent - Oct 6.mp4 (349 MB)
2. GMT20250516-190317_Recording_avo_1920x1080 braintrust.mp4 (980 MB)
3. GMT20250520-223657_Recording_avo_1920x1080.mp4 (1.3 GB)
4. Screen Recording 2025-09-11 at 8.49.02 AM.mov (1.8 GB)

**Validation**:
- Successful processing without OOM errors
- Peak memory usage stays within system limits
- Transcription quality maintained for long videos
- Timeline generation for 60-90 minute videos

---

## 4. TEST FILE SELECTION RECOMMENDATIONS

### Quick Validation (< 5 min)
**Files**: 5 Docling clips
- sample_10s_video-mp4.mp4
- sample_10s_audio-mp3.mp3
- sample_10s_audio-wav.wav
- sample_10s_video-quicktime.mov
- silent_1s.wav (edge case)

---

### Standard Test Suite (30 min)
**Files**: 10 files covering major scenarios

**Video (6 files)**:
1. sample_10s_video-mp4.mp4 (315 KB) - Format test
2. editing-relevance-rubrics kg may 16 2025.mov (34 MB) - Screen recording
3. review existing benchmarks/gonzolo meeting aug 14/video1171640589.mp4 (89 MB) - Zoom meeting, diarization
4. Summarize feedback from Slack This week - June 30 demo - Screen Recording 2025-06-27 at 5.50.23 PM.mov (111 MB) - Demo, OCR
5. mission control video demo 720.mov (277 MB) - 720p quality
6. "carving ice" Kinetics sample (800 KB) - Action detection

**Audio (4 files)**:
7. sample_10s_audio-mp3.mp3 (160 KB) - Format test
8. sample_10s_audio-wav.wav (1.7 MB) - Lossless format
9. audio1171640589.m4a (19 MB) - Meeting audio
10. silent_1s.wav (88 KB) - Silence detection

---

### Comprehensive Test Suite (4-6 hours)
**Files**: 1,100 files

**Video**:
- 16 Docling test clips
- 18 Desktop production files
- 1,000 Kinetics-600 videos (random selection)
- 66 Kinetics videos from 6 specific categories:
  - carving ice (26)
  - picking fruit (27)
  - crossing eyes (26)
  - diving cliff (22)
  - sticking tongue out (30)
  - home roasting coffee (24)

---

### Full Stress Test (8-12 hours)
**Files**: 18,300+ files

**All available files**:
- Complete Kinetics-600 subset (18,288 videos)
- All Desktop production files (18 videos)
- All Docling test clips (16 files)

---

## 5. TEST DATA CHARACTERISTICS

### Format Distribution (Estimated)

**Video Formats**:
- MP4: ~18,300+ files (Kinetics + Desktop)
- MOV: ~8 files (Desktop + Docling)
- AVI: ~2 files (Docling)

**Audio Formats**:
- MP3: ~29,000+ files (system + test)
- M4A: ~10 files (Desktop + Docling)
- WAV: ~8 files (Docling + langchain)
- FLAC: ~2 files (Docling)
- AAC: ~1 file (Docling)
- OGG: ~1 file (Docling)

### Size Distribution

| Size Range | Count (Est.) | Use Case |
|------------|--------------|----------|
| < 1 MB | ~18,300 | Kinetics videos, test clips |
| 1-50 MB | ~20 | Small production videos, audio files |
| 50-200 MB | ~8 | Medium production videos |
| 200-500 MB | ~3 | Large production videos |
| 500MB-1GB | ~1 | Very large videos |
| > 1 GB | ~2 | Stress test files |

### Duration Distribution (Estimated)

| Duration | Count (Est.) | Files |
|----------|--------------|-------|
| 1-10 seconds | ~18,304 | Kinetics + Docling |
| 1-5 minutes | ~8 | Small Desktop videos |
| 5-15 minutes | ~6 | Medium Desktop videos |
| 15-60 minutes | ~3 | Large Desktop videos |
| > 60 minutes | ~2 | Very large Desktop videos |

### Content Type Distribution

| Content Type | Count (Est.) | Test Value |
|--------------|--------------|------------|
| Human actions | 18,288 | Kinetics dataset (action detection) |
| Screen recordings | ~10 | OCR, UI detection |
| Zoom meetings | ~4 | Speaker diarization, face detection |
| Presentation/Demos | ~4 | Scene detection, slides |
| Test clips | 16 | Format validation |
| Music/Audio | ~29,000 | Audio processing (mostly system files) |

---

## 6. RECOMMENDED TEST EXECUTION ORDER

### Day 1: Initial Validation
1. **Smoke Tests** (5 min) - Docling clips, CPU-only
2. **Format Tests** (15 min) - All Docling clips, both CPU and GPU
3. **Small Integration** (30 min) - 3 small Desktop files (30-40 MB)
4. **Audio Tests** (15 min) - 2 audio-only files

**Total**: ~1 hour
**Goal**: Verify basic functionality

---

### Day 2: Feature Validation
1. **Medium Videos** (1 hour) - 4 medium Desktop files (70-120 MB)
2. **Transcription Quality** (1 hour) - Manual review of transcripts
3. **Diarization** (30 min) - Test speaker detection on Zoom meetings
4. **OCR/Object Detection** (30 min) - Validate detections on screen recordings

**Total**: 3 hours
**Goal**: Verify feature quality

---

### Day 3: Performance Benchmarking
1. **Small Batch** (1 hour) - 100 Kinetics videos
2. **Batch Size Tuning** (2 hours) - Test different batch sizes (16, 32, 64, 128)
3. **Throughput Test** (3 hours) - 1,000 Kinetics videos

**Total**: 6 hours
**Goal**: Establish baseline performance metrics

---

### Day 4: Stress Testing
1. **Full Dataset** (10 hours) - All 18,288 Kinetics videos
2. **Monitoring** (continuous) - Track metrics, watch for issues

**Total**: 10 hours
**Goal**: Validate scale and stability

---

### Day 5: Large File Testing
1. **Large Videos** (2 hours) - 4 files (350MB - 1.8GB)
2. **Memory Profiling** (1 hour) - Analyze peak memory usage
3. **Quality Validation** (1 hour) - Check transcription quality on long videos

**Total**: 4 hours
**Goal**: Confirm memory management

---

## 7. METRICS TO CAPTURE

### Per-File Metrics
- File size (bytes)
- Duration (seconds)
- Format (codec, container)
- Resolution (video)
- Sample rate (audio)
- Processing time (milliseconds)
- CPU usage (%)
- GPU usage (%)
- RAM usage (MB)
- VRAM usage (MB)
- Success/failure status
- Error message (if failed)

### Per-Feature Metrics
- Transcription WER (if ground truth available)
- Transcription confidence score
- Speaker count detected
- Diarization error rate (if ground truth available)
- Scene count detected
- Object detection count
- Face detection count
- OCR text detected (character count)
- Embedding dimensions
- Quality scores (0.0-1.0)

### Aggregate Metrics
- Total files processed
- Success rate (%)
- Total processing time
- Average processing time per file
- Throughput (files per hour)
- Throughput multiplier (vs. real-time)
- CPU efficiency (%)
- GPU efficiency (%)
- Peak memory usage
- Error rate by file type
- Error rate by file size

---

## 8. KNOWN LIMITATIONS & EDGE CASES

### File Access Issues
- Some files in system directories may have permission restrictions
- Dropbox sync status may affect Kinetics dataset access
- Large files (> 1 GB) may have network access delays

### Content Limitations
- **System audio files** (~29K): Mostly UI sounds, not useful for transcription testing
- **Silent audio** (silent_1s.wav): Edge case, no transcription possible
- **Kinetics dataset**: 10-second clips may not fully test long-duration features (speaker diarization, extended scenes)
- **No multi-language content**: Most files appear to be English-only

### Test Coverage Gaps
- **No 4K videos**: Highest resolution is 1920x1080
- **No HDR content**: No HDR10 or Dolby Vision test files identified
- **No extremely long videos**: Longest files are ~90 minutes
- **No corrupted files**: No intentionally corrupted test files for error handling
- **No encrypted files**: No password-protected or DRM content
- **Limited codec variety**: Mostly H.264, no H.265/VP9/AV1 identified

---

## 9. TEST FILE ACCESSIBILITY

### Immediately Accessible
- ✅ Desktop production files (18 files, 4.5 GB)
- ✅ Docling test clips (16 files, 12 MB)

### May Require Dropbox Sync
- ⚠️ Kinetics-600 dataset (18,288 files, ~14.6 GB)
  - Location: Dropbox folder
  - May need to ensure sync is complete
  - Check with: `ls -la "/Users/ayates/Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/Kinetics dataset (5%)/"`

### Not Recommended
- ❌ System/Library files (~29K files)
  - Permission issues
  - Mostly UI sounds, not test-worthy
  - May trigger security warnings

---

## 10. RECOMMENDATIONS FOR ADDITIONAL TEST FILES

To improve test coverage, consider adding:

### High Priority
1. **4K video sample** (3840x2160) - Test high-resolution processing
2. **Multi-language content** - Spanish, Mandarin, French audio/video
3. **Corrupted file samples** - Test error handling
4. **HDR video** - HDR10 or Dolby Vision
5. **Long-duration video** - 3+ hours for extended stress testing
6. **Encrypted/DRM content** - Test graceful failure

### Medium Priority
7. **H.265/HEVC video** - Modern codec testing
8. **VP9/AV1 video** - WebM codec variants
9. **Multi-track audio** - Videos with multiple audio streams
10. **Subtitle files** - SRT, VTT for comparison with transcription
11. **Ultra-wide video** - 21:9 aspect ratio
12. **Vertical video** - 9:16 mobile format
13. **Very low quality** - 240p, heavily compressed
14. **Very high bitrate** - Uncompressed or ProRes

### Low Priority
15. **Ancient formats** - MPEG-1, Real Media, Windows Media
16. **Exotic codecs** - Theora, Dirac
17. **Raw video** - Uncompressed YUV
18. **Spatial audio** - Dolby Atmos, 5.1 surround

---

## 11. QUICK START COMMANDS

### Copy Small Test Set to Working Directory
```bash
# Create test directory
mkdir -p /Users/ayates/video_audio_extracts/test_files

# Copy Docling clips (fast validation)
cp /Users/ayates/docling/tests/data/audio/sample_10s_* /Users/ayates/video_audio_extracts/test_files/

# Copy 5 production videos (realistic testing)
cp "/Users/ayates/Desktop/stuff/stuff/editing-relevance-rubrics kg may 16 2025.mov" \
   "/Users/ayates/Desktop/stuff/stuff/May 5 - live labeling mocks.mp4" \
   "/Users/ayates/Desktop/stuff/stuff/review existing benchmarks/gonzolo meeting aug 14/video1171640589.mp4" \
   "/Users/ayates/Desktop/stuff/stuff/review existing benchmarks/gonzolo meeting aug 14/audio1171640589.m4a" \
   "/Users/ayates/Desktop/stuff/stuff/mission control video demo 720.mov" \
   /Users/ayates/video_audio_extracts/test_files/
```

### Generate Kinetics Test List
```bash
# List all Kinetics videos
find "/Users/ayates/Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/Kinetics dataset (5%)/kinetics600_5per/kinetics600_5per/train" \
  -type f -name "*.mp4" \
  > /Users/ayates/video_audio_extracts/kinetics_full_list.txt

# Create random sample of 1000 videos
shuf -n 1000 /Users/ayates/video_audio_extracts/kinetics_full_list.txt \
  > /Users/ayates/video_audio_extracts/kinetics_1000_sample.txt

# Create small sample of 100 videos
head -n 100 /Users/ayates/video_audio_extracts/kinetics_1000_sample.txt \
  > /Users/ayates/video_audio_extracts/kinetics_100_sample.txt
```

### Get File Statistics
```bash
# Count files by extension
find /Users/ayates/video_audio_extracts/test_files -type f | \
  sed 's/.*\.//' | sort | uniq -c | sort -rn

# Get total size
du -sh /Users/ayates/video_audio_extracts/test_files

# List with sizes
ls -lh /Users/ayates/video_audio_extracts/test_files
```

---

## 12. CONCLUSION

This system has access to **47,860 audio and video files** suitable for comprehensive testing:

**Strengths**:
- ✅ **Diverse formats**: MP4, MOV, AVI, MP3, M4A, WAV, FLAC, AAC, OGG
- ✅ **Wide size range**: 88 KB → 1.8 GB
- ✅ **Scale testing**: 18,288 Kinetics videos for throughput validation
- ✅ **Real-world content**: Zoom meetings, screen recordings, presentations
- ✅ **Standardized clips**: 16 purpose-built 10-second test files
- ✅ **Ground truth data**: Kinetics-600 action labels

**Recommended Starting Point**:
1. **Docling clips** (16 files, 5 min) - Format validation
2. **5 production videos** (30 min) - Feature validation
3. **100 Kinetics videos** (1 hour) - Performance baseline
4. **1,000 Kinetics videos** (4 hours) - Throughput measurement
5. **Full Kinetics dataset** (10 hours) - Stress testing

**Total Test Suite Execution Time**: ~15-20 hours for comprehensive validation

The test suite provides excellent coverage for validating format support, feature accuracy, and performance characteristics. The combination of small standardized clips, realistic production content, and large-scale dataset enables thorough testing across all system components.

---

**Test Suite Status**: ✅ Ready for execution
**Next Steps**: Implement system, run Phase 1 smoke tests
