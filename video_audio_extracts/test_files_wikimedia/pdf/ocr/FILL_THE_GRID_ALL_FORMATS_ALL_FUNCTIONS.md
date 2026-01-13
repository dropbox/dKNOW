# [MANAGER] FILL THE GRID: All Formats × All Functions

USER: "direct the worker, if it doesn't know what to do after getting files, to keep filling out the grid of all formats to all functions"

---

## THE GRID

**Formats**: 40 (video, audio, image)
**Functions**: 32 plugins

**Goal**: Test every valid (format, function) combination

---

## CURRENT GRID STATUS

**Populated cells**: ~90 combinations
**Possible cells**: ~350-400 valid combinations
**Coverage**: ~25%

**Need**: Fill remaining 250-300 combinations

---

## HOW TO FILL THE GRID

For EACH format, test with ALL compatible functions:

### Video Formats → Video Functions

**Video formats** (21): MP4, MOV, WEBM, MKV, AVI, FLV, 3GP, WMV, OGV, M4V, TS, MTS, M2TS, MPG, MXF, VOB, ASF, RM, DV, GXF, HLS

**Video functions** (19):
- keyframes
- scene-detection
- object-detection
- face-detection
- action-recognition
- motion-tracking
- pose-estimation
- smart-thumbnail
- shot-classification
- emotion-detection
- subtitle-extraction
- format-conversion
- vision-embeddings
- metadata-extraction
- transcription (if has audio)
- diarization (if has audio)
- audio-classification (if has audio)
- audio-extraction (if has audio)
- audio-embeddings (if has audio)

**Target**: 21 formats × ~15-19 functions = 300-400 cells

### Audio Formats → Audio Functions

**Audio formats** (14): WAV, MP3, FLAC, M4A, OGG, OPUS, WMA, AMR, AC3, DTS, ALAC, APE, TTA, WavPack

**Audio functions** (7):
- transcription
- diarization  
- audio-classification
- audio-embeddings
- audio-enhancement-metadata
- audio-extraction
- metadata-extraction

**Target**: 14 formats × 7 functions = 98 cells

### Image Formats → Image Functions

**Image formats** (15): JPG, PNG, HEIC, HEIF, GIF, TIFF, SVG, AVIF, BMP, WEBP, ICO, CR2, NEF, ARW, DNG, RAF, ORF, PEF, PDF

**Image functions** (13):
- face-detection
- object-detection
- ocr
- pose-estimation
- emotion-detection
- image-quality-assessment
- shot-classification
- content-moderation
- logo-detection
- caption-generation
- depth-estimation
- vision-embeddings
- action-recognition (for multi-frame)

**Target**: 19 formats × 13 functions = 247 cells

---

## EXECUTION PLAN

### Phase 1: Expand Existing Formats to More Functions (N=387-395)

**Example: MP4 format**
Currently has: transcription, keyframes
Should have: ALL video functions

```bash
# Copy MP4 to all function directories
for func in scene-detection object-detection face-detection action-recognition motion-tracking pose-estimation smart-thumbnail shot-classification emotion-detection; do
  mkdir -p test_files_wikimedia/mp4/$func
  cp test_files_wikimedia/mp4/keyframes/*.mp4 test_files_wikimedia/mp4/$func/
done
```

**Example: PNG format**
Currently has: face-detection, object-detection, vision-embeddings
Should have: ALL image functions

```bash
for func in ocr pose-estimation emotion-detection image-quality shot-classification content-moderation logo-detection caption-generation depth-estimation; do
  mkdir -p test_files_wikimedia/png/$func
  cp test_files_wikimedia/png/object-detection/*.png test_files_wikimedia/png/$func/
done
```

### Phase 2: Add New Formats to Functions (N=396-405)

For EACH newly acquired format (MKV, HEIC, Camera RAW, etc.), test with ALL compatible functions.

**Example: MKV format**
```bash
# MKV should work with all video functions
for func in keyframes scene-detection object-detection face-detection transcription; do
  mkdir -p test_files_wikimedia/mkv/$func
  # Copy or use existing MKV files
done
```

### Phase 3: Document Grid Coverage (N=406)

Create grid showing (format, function) coverage:
- Mark ✅ for tested combinations
- Mark ⚠️ for untested but valid combinations
- Mark N/A for invalid combinations

---

## PRIORITY ORDER

**High priority** (mainstream formats × all functions):
1. MP4 → all video functions (15 functions)
2. WEBM → all video functions
3. MOV → all video functions
4. JPG → all image functions (13 functions)
5. PNG → all image functions
6. WAV → all audio functions (7 functions)
7. MP3 → all audio functions

**Medium priority** (specialized formats):
8. MKV, HEIC, Camera RAW formats → compatible functions

**Target**: 350+ (format, function) combinations tested

---

## COMMIT STRATEGY

Commit in batches:
```
# 387: MP4 Format - Add 10 Functions (10 combinations)
# 388: WEBM Format - Add 8 Functions (8 combinations)
# 389: PNG Format - Add 9 Functions (9 combinations)
...
```

Track progress in grid document.

---

## USER DIRECTIVE

"keep filling out the grid of all formats to all functions"

**Meaning**: Don't just have format files - TEST every format with every compatible function.

Execute systematically starting N=387.
