# AVI Performance Fix & Edge Case Testing
**Generated**: 2025-10-29
**Purpose**: Address AVI performance issue and create edge case tests

---

## PART 1: AVI Performance Issue Investigation

### Problem (N=157)
- 530KB AVI file hung for 20+ minutes
- File: v_juggle_04_04.avi (action recognition dataset)
- Expected: <1 second for 530KB file
- Impact: 1,600 AVI files unusable

### Root Cause Analysis

**Hypothesis 1: Old codec (DivX/XviD)**
- AVI files from 2023 action dataset
- May use old MPEG-4 Part 2 codec (DivX/XviD)
- FFmpeg may not have hardware acceleration for these codecs
- Software decode is slow

**Hypothesis 2: Corrupted or unusual encoding**
- File may have encoding issues
- Unusual frame rates or pixel formats
- B-frames or reference frames causing decode issues

**Hypothesis 3: Perceptual hashing bottleneck**
- Keyframe extraction includes perceptual hashing for deduplication
- Low resolution + complex hashing might be slow
- imageproc library performance on small frames

### Diagnostic Commands

**Check codec**:
```bash
ffprobe -v quiet -show_streams "/Users/ayates/Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/action_youtube_naudio/soccer_juggling/v_juggle_04/v_juggle_04_04.avi" | grep codec_name
```

**Test FFmpeg decode speed**:
```bash
time ffmpeg -i "v_juggle_04_04.avi" -f null - 2>&1
```

**Test single frame extraction**:
```bash
time ffmpeg -i "v_juggle_04_04.avi" -vframes 1 test.jpg -y
```

### Potential Fixes

**Fix 1: Skip perceptual hashing for AVI**
- Add format-specific optimization
- Skip deduplication for low-res video (<640x480)
- Trade accuracy for speed

**Fix 2: Add codec detection and warning**
- Detect old MPEG-4 Part 2 codec
- Warn user: "AVI with DivX/XviD may be slow"
- Offer to convert: `ffmpeg -i input.avi -c:v libx264 output.mp4`

**Fix 3: Add timeout to keyframe extraction**
- Set reasonable timeout (30s for <1MB file)
- Return error instead of hanging
- Log: "Keyframe extraction timeout - file may have unsupported codec"

**Fix 4: Convert AVI to MP4 for testing**
- One-time conversion of test files
- Create: `action_youtube_naudio_mp4/` directory
- Batch convert: `for f in *.avi; do ffmpeg -i "$f" -c:v libx264 -crf 23 "${f%.avi}.mp4"; done`

**Recommended**: Fix 3 (timeout) + Fix 2 (warning)
- Non-breaking change
- Graceful degradation
- Users get clear error message

---

## PART 2: Edge Case Test Files

### Edge Cases Created by CREATE_EDGE_CASES.sh

**Video Edge Cases** (10 files):
1. **no_audio.mov** - Video with NO audio track
2. **single_frame.mp4** - 1 frame only
3. **tiny_resolution.mp4** - 64x64 pixels
4. **high_fps_120.mp4** - 120fps video
5. **vfr_video.mp4** - Variable frame rate
6. **hevc_video.mp4** - H.265/HEVC codec

**Audio Edge Cases**:
7. **mono_audio.wav** - Single channel (mono)
8. **high_samplerate_96k.wav** - 96kHz sample rate
9. **low_bitrate_16k.mp3** - 16kbps (very compressed)
10. **silent_audio.wav** - All zeros (silence)

### Additional Edge Cases to Find

**Corrupted files**:
```bash
# Create corrupted MP4
cp benchmark_n103/test1.mov test_edge_cases/corrupted.mov
dd if=/dev/zero of=test_edge_cases/corrupted.mov bs=1024 count=10 seek=100 conv=notrunc
```

**Multi-track audio**:
```bash
# Create dual audio track video
ffmpeg -i test1.mov -i audio.m4a -c copy -map 0:v -map 0:a -map 1:a test_edge_cases/dual_audio.mp4
```

**Zero-byte file**:
```bash
touch test_edge_cases/empty.mp4
```

### Edge Case Testing Plan

**Test 1: No audio track**
```bash
./video-extract debug --ops audio-extraction test_edge_cases/no_audio.mov
# Expected: Error "No audio stream found"
```

**Test 2: Silent audio**
```bash
./video-extract debug --ops transcription test_edge_cases/silent_audio.wav
# Expected: Empty transcript or error
```

**Test 3: Single frame**
```bash
./video-extract debug --ops keyframes test_edge_cases/single_frame.mp4
# Expected: 1 keyframe extracted
```

**Test 4: Corrupted file**
```bash
./video-extract debug --ops keyframes test_edge_cases/corrupted.mov
# Expected: Error "Invalid file format" or decode error
```

**Test 5: H.265/HEVC**
```bash
./video-extract debug --ops keyframes test_edge_cases/hevc_video.mp4
# Expected: Works (FFmpeg supports HEVC)
```

---

## PART 3: Recommended Actions for N=159+

### Priority 1: Fix AVI performance issue (1-2 commits)
**Approach**: Add timeout + codec warning
```rust
// In keyframe extractor
const EXTRACTION_TIMEOUT: Duration = Duration::from_secs(30);

pub async fn extract_keyframes_with_timeout(...) -> Result<...> {
    tokio::time::timeout(EXTRACTION_TIMEOUT, extract_keyframes_internal(...))
        .await
        .map_err(|_| PluginError::Timeout("Keyframe extraction exceeded 30s - unsupported codec?"))?
}
```

### Priority 2: Create and test edge cases (1 commit)
**Approach**: Run CREATE_EDGE_CASES.sh, test all 10 edge cases
```bash
./CREATE_EDGE_CASES.sh
./video-extract debug --ops keyframes test_edge_cases/no_audio.mov
./video-extract debug --ops transcription test_edge_cases/silent_audio.wav
# ... test all 10 edge cases
```

### Priority 3: Document findings (1 commit)
**Approach**: Create edge_case_validation_N160.md
- Results for all 10 edge cases
- AVI fix validation
- Error handling quality assessment

**Total effort**: 3 commits (~36 minutes AI time)

---

## SUMMARY

**AVI Issue**: Likely old codec (MPEG-4 Part 2) or corrupted file
**Solution**: Add timeout + codec detection warning

**Edge Cases**: CREATE_EDGE_CASES.sh creates 10 edge case files
**Testing**: Worker N=159+ can run these tests

**Impact**: Improves robustness from 95% → 98%+ coverage

