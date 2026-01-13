# Test File Gap Analysis & Acquisition Plan
**Date**: 2025-10-31 (N=145)
**Purpose**: Identify test file gaps and acquire diverse duration/format/codec samples
**Goal**: 3+ duration variants (short/medium/long) per format for reliable benchmarks

---

## Current Inventory Summary

**Video Files**:
- **MP4**: 5 files (38MB-1.3GB), mostly H.264, durations: 2.2min-86min
- **MOV**: 5 files (34MB-980MB), H.264, durations: similar to MP4
- **MKV**: 5 files (~10-11MB), low-res dataset videos
- **WEBM**: 5 files (~2MB), VP9 codec, low-res
- **AVI**: 5 files (13K-891K), action dataset, very short clips

**Audio Files**:
- **WAV**: 5 files, including 5min sine wave, 3sec silence, 1sec short
- **MP3**: 5 files (375K-32MB), 64kbps audiobooks
- **FLAC**: 5 files (16MB), all same file (96kHz 24-bit)
- **M4A**: 2 files (13-19MB), Zoom meeting audio
- **AAC**: 5 files (~146K), 10s test audio

**Generated Synthetic Files** (test_media_generated/):
- Keyframe density tests (10s-60s)
- High FPS tests (120fps, 240fps)
- Codec tests (VP9, AV1, MPEG-2)
- Audio tests (5min sine, 1min noise)

---

## Gap Analysis

### Gap 1: Duration Diversity ❌

**Problem**: Most files are either very short (<30s) or very long (>30min)
- Missing: 30s-1min, 1-5min, 5-15min, 15-30min ranges

**Current duration distribution**:
| Duration | Video Files | Audio Files | Quality |
|----------|-------------|-------------|---------|
| <10s | Many (edge cases + generated) | Few (10s AAC) | Good |
| 10-30s | Few (generated synthetic) | Few (1min FLAC) | ⚠️ Sparse |
| 30s-5min | ❌ Missing | Few (5min WAV sine) | ❌ Gap |
| 5-15min | 1 (7.6min mission control) | ❌ Missing | ❌ Gap |
| 15-30min | ❌ Missing | ❌ Missing | ❌ Gap |
| 30-60min | 1 (56min braintrust) | ❌ Missing | ⚠️ Sparse |
| >60min | 1 (86min Zoom) | ❌ Missing | ⚠️ Sparse |

**Impact**: Can't reliably benchmark medium-duration files (1-5min, 5-15min)

---

### Gap 2: Video Codec Diversity ❌

**Problem**: Mostly H.264, sparse modern codecs

**Current codec distribution**:
| Codec | File Count | Quality |
|-------|------------|---------|
| H.264 | 20+ files | ✅ Excellent |
| H.265 (HEVC) | 1 file (test_edge_cases) | ❌ Sparse |
| VP9 | 2-3 files (webm, low quality) | ⚠️ Limited |
| AV1 | 1 file (5s synthetic) | ❌ Sparse |
| MPEG-2 | 1 file (10s synthetic) | ❌ Sparse |

**Impact**: Can't validate codec-specific optimizations

---

### Gap 3: Video Resolution Diversity ❌

**Problem**: Most files are either low-res or 1080p+, missing common resolutions

**Current resolution distribution**:
| Resolution | File Count | Quality |
|------------|------------|---------|
| 64×64 (tiny) | 1 file | ⚠️ Edge case |
| ~360p-480p (low-res) | Many (action dataset, Low-Res SR) | ✅ Good |
| 720p (HD) | Few | ❌ Sparse |
| 1080p (Full HD) | ~5 files | ✅ Good |
| 1440p (2K) | ❌ Missing | ❌ Gap |
| 4K (2160p) | 1 file (test_edge_cases) | ⚠️ Edge case |
| >4K | 1 file (3908×2304 screen recording) | ⚠️ Edge case |

**Impact**: Missing popular YouTube/streaming resolutions (720p, 1440p)

---

### Gap 4: Audio Format/Bitrate Diversity ❌

**Problem**: Limited bitrate variety, missing OGG Vorbis audio files

**Current audio diversity**:
| Format | Bitrates | Duration Range | Quality |
|--------|----------|----------------|---------|
| WAV (PCM) | 44.1kHz, 96kHz | 1s-5min | ✅ Good |
| MP3 | 64kbps, 16kbps | 10s-audiobooks | ⚠️ Limited |
| FLAC | 96kHz 24-bit | Same 16MB file | ❌ Sparse |
| AAC | Unknown | 10s | ❌ Sparse |
| M4A | Unknown | 13-19MB | ❌ Sparse |
| OGG | Container only (format_test_ogg.ogg) | ❌ Missing Vorbis audio |

**Impact**: Can't test audio codec optimizations reliably

---

### Gap 5: Realistic Use Case Files ❌

**Problem**: Most files are either synthetic tests or very long Zoom recordings
- Missing: Typical podcast (20-60min), music videos (3-5min), short clips (15-30s)

**Current use case coverage**:
| Use Case | Example Files | Quality |
|----------|---------------|---------|
| Social media clip (15-30s) | Few synthetic | ⚠️ Limited |
| Music video (3-5min) | ❌ Missing | ❌ Gap |
| Tutorial/demo (5-15min) | 1 (7.6min mission control) | ⚠️ Sparse |
| Podcast/interview (20-60min) | ❌ Missing | ❌ Gap |
| Full meeting (60-90min) | 2 (56min, 86min Zoom) | ⚠️ Sparse |
| Movie trailer (2min) | ❌ Missing | ❌ Gap |

---

## Acquisition Plan

### Priority 1: Fill Duration Gaps (Generate Synthetic Files)

**Goal**: Create synthetic files covering missing duration ranges

**Files to generate**:
1. **30s video** (H.264, 720p, 30fps)
   - `test_duration_30s_720p.mp4`
   - Solid color with text overlay (no copyright issues)
   - Size: ~2-3MB

2. **1min video** (H.264, 1080p, 30fps)
   - `test_duration_1min_1080p.mp4`
   - Gradient animation
   - Size: ~5-8MB

3. **2min video** (H.264, 1080p, 30fps)
   - `test_duration_2min_1080p.mp4`
   - Similar to 1min
   - Size: ~10-15MB

4. **5min video** (H.264, 1080p, 30fps)
   - `test_duration_5min_1080p.mp4`
   - Longer gradient animation
   - Size: ~25-40MB

5. **10min video** (H.264, 720p, 30fps)
   - `test_duration_10min_720p.mp4`
   - Size: ~40-60MB

6. **15min video** (H.264, 720p, 30fps)
   - `test_duration_15min_720p.mp4`
   - Size: ~60-90MB

**Audio files to generate**:
7. **30s audio** (MP3, 192kbps)
   - `test_audio_30s_music.mp3`
   - Sine wave sweep
   - Size: ~700KB

8. **2min audio** (WAV, 44.1kHz)
   - `test_audio_2min_speech.wav`
   - Synthetic speech or tone pattern
   - Size: ~20MB

9. **10min audio** (MP3, 128kbps)
   - `test_audio_10min_podcast.mp3`
   - Tone with periodic changes
   - Size: ~10MB

10. **30min audio** (M4A, 128kbps)
    - `test_audio_30min_lecture.m4a`
    - Extended tone/silence pattern
    - Size: ~30MB

**Generation commands** (FFmpeg):
```bash
# 30s 720p video
ffmpeg -f lavfi -i testsrc=duration=30:size=1280x720:rate=30 \
  -c:v libx264 -preset fast test_duration_30s_720p.mp4

# 1min 1080p video with gradient
ffmpeg -f lavfi -i "color=c=blue:s=1920x1080:d=60,format=yuv420p" \
  -f lavfi -i "sine=frequency=1000:duration=60" \
  -c:v libx264 -c:a aac test_duration_1min_1080p.mp4

# Similar for 2min, 5min, 10min, 15min variants

# Audio files
ffmpeg -f lavfi -i "sine=frequency=440:duration=30" \
  -b:a 192k test_audio_30s_music.mp3

ffmpeg -f lavfi -i "sine=frequency=440:duration=120" \
  -ar 44100 test_audio_2min_speech.wav

ffmpeg -f lavfi -i "sine=frequency=440:duration=600" \
  -b:a 128k test_audio_10min_podcast.mp3

ffmpeg -f lavfi -i "sine=frequency=440:duration=1800" \
  -c:a aac -b:a 128k test_audio_30min_lecture.m4a
```

---

### Priority 2: Modern Codec Diversity (Generate)

**Goal**: Create H.265, VP9, AV1 variants of common durations

**Files to generate**:
11. **1min H.265** (1080p, 30fps)
    - `test_1min_1080p_hevc.mp4`
    - Compare to H.264 version
    - Size: ~3-5MB (better compression)

12. **1min VP9** (1080p, 30fps)
    - `test_1min_1080p_vp9.webm`
    - WebM container
    - Size: ~3-5MB

13. **1min AV1** (1080p, 30fps)
    - `test_1min_1080p_av1.mp4`
    - Modern codec
    - Size: ~2-4MB (best compression)

14. **5min H.265** (720p, 30fps)
    - `test_5min_720p_hevc.mp4`
    - Medium duration test
    - Size: ~15-25MB

15. **5min VP9** (720p, 30fps)
    - `test_5min_720p_vp9.webm`
    - Size: ~15-25MB

**Generation commands**:
```bash
# H.265 (HEVC)
ffmpeg -f lavfi -i testsrc=duration=60:size=1920x1080:rate=30 \
  -c:v libx265 -preset fast test_1min_1080p_hevc.mp4

# VP9
ffmpeg -f lavfi -i testsrc=duration=60:size=1920x1080:rate=30 \
  -c:v libvpx-vp9 -b:v 1M test_1min_1080p_vp9.webm

# AV1 (if available)
ffmpeg -f lavfi -i testsrc=duration=60:size=1920x1080:rate=30 \
  -c:v libaom-av1 -b:v 1M test_1min_1080p_av1.mp4
```

---

### Priority 3: Resolution Diversity (Generate)

**Goal**: Create 720p and 1440p versions of common durations

**Files to generate**:
16. **1min 720p** (H.264, 30fps)
    - `test_1min_720p_h264.mp4`
    - Popular streaming resolution
    - Size: ~3-5MB

17. **1min 1440p** (H.264, 30fps)
    - `test_1min_1440p_h264.mp4`
    - 2K resolution
    - Size: ~8-12MB

18. **5min 720p** (H.264, 30fps)
    - `test_5min_720p_h264.mp4`
    - Size: ~15-25MB

19. **5min 1440p** (H.264, 30fps)
    - `test_5min_1440p_h264.mp4`
    - Size: ~40-60MB

**Generation commands**:
```bash
# 720p variants
ffmpeg -f lavfi -i testsrc=duration=60:size=1280x720:rate=30 \
  -c:v libx264 test_1min_720p_h264.mp4

# 1440p variants
ffmpeg -f lavfi -i testsrc=duration=60:size=2560x1440:rate=30 \
  -c:v libx264 test_1min_1440p_h264.mp4
```

---

### Priority 4: Audio Bitrate/Format Diversity (Generate)

**Goal**: Create varied bitrate MP3/AAC files, OGG Vorbis

**Files to generate**:
20. **1min MP3 320kbps** (high quality)
    - `test_audio_1min_320kbps.mp3`
    - Size: ~2.4MB

21. **1min MP3 128kbps** (medium quality)
    - `test_audio_1min_128kbps.mp3`
    - Size: ~1MB

22. **1min MP3 64kbps** (low quality)
    - `test_audio_1min_64kbps.mp3`
    - Size: ~480KB

23. **1min AAC 256kbps** (high quality)
    - `test_audio_1min_256kbps_aac.m4a`
    - Size: ~1.9MB

24. **1min OGG Vorbis 192kbps**
    - `test_audio_1min_192kbps.ogg`
    - Size: ~1.4MB

25. **5min OGG Vorbis 128kbps**
    - `test_audio_5min_128kbps.ogg`
    - Size: ~4.8MB

**Generation commands**:
```bash
# MP3 variants
ffmpeg -f lavfi -i "sine=frequency=440:duration=60" \
  -b:a 320k test_audio_1min_320kbps.mp3

ffmpeg -f lavfi -i "sine=frequency=440:duration=60" \
  -b:a 128k test_audio_1min_128kbps.mp3

ffmpeg -f lavfi -i "sine=frequency=440:duration=60" \
  -b:a 64k test_audio_1min_64kbps.mp3

# AAC
ffmpeg -f lavfi -i "sine=frequency=440:duration=60" \
  -c:a aac -b:a 256k test_audio_1min_256kbps_aac.m4a

# OGG Vorbis
ffmpeg -f lavfi -i "sine=frequency=440:duration=60" \
  -c:a libvorbis -b:a 192k test_audio_1min_192kbps.ogg

ffmpeg -f lavfi -i "sine=frequency=440:duration=300" \
  -c:a libvorbis -b:a 128k test_audio_5min_128kbps.ogg
```

---

## Implementation Checklist

### Phase 1: Generate Synthetic Files (25 new files)

**Duration diversity** (10 files):
- [ ] 30s 720p video
- [ ] 1min 1080p video
- [ ] 2min 1080p video
- [ ] 5min 1080p video
- [ ] 10min 720p video
- [ ] 15min 720p video
- [ ] 30s MP3 audio
- [ ] 2min WAV audio
- [ ] 10min MP3 audio
- [ ] 30min M4A audio

**Codec diversity** (5 files):
- [ ] 1min H.265 1080p
- [ ] 1min VP9 1080p
- [ ] 1min AV1 1080p
- [ ] 5min H.265 720p
- [ ] 5min VP9 720p

**Resolution diversity** (4 files):
- [ ] 1min 720p H.264
- [ ] 1min 1440p H.264
- [ ] 5min 720p H.264
- [ ] 5min 1440p H.264

**Audio diversity** (6 files):
- [ ] 1min MP3 320kbps
- [ ] 1min MP3 128kbps
- [ ] 1min MP3 64kbps
- [ ] 1min AAC 256kbps
- [ ] 1min OGG Vorbis 192kbps
- [ ] 5min OGG Vorbis 128kbps

### Phase 2: Organize and Document

**Create directory structure**:
```
test_media_generated/
├── duration_tests/          # Priority 1 files
│   ├── video/
│   │   ├── test_duration_30s_720p.mp4
│   │   ├── test_duration_1min_1080p.mp4
│   │   ├── test_duration_2min_1080p.mp4
│   │   ├── test_duration_5min_1080p.mp4
│   │   ├── test_duration_10min_720p.mp4
│   │   └── test_duration_15min_720p.mp4
│   └── audio/
│       ├── test_audio_30s_music.mp3
│       ├── test_audio_2min_speech.wav
│       ├── test_audio_10min_podcast.mp3
│       └── test_audio_30min_lecture.m4a
├── codec_tests/             # Priority 2 files
│   ├── test_1min_1080p_hevc.mp4
│   ├── test_1min_1080p_vp9.webm
│   ├── test_1min_1080p_av1.mp4
│   ├── test_5min_720p_hevc.mp4
│   └── test_5min_720p_vp9.webm
├── resolution_tests/        # Priority 3 files
│   ├── test_1min_720p_h264.mp4
│   ├── test_1min_1440p_h264.mp4
│   ├── test_5min_720p_h264.mp4
│   └── test_5min_1440p_h264.mp4
└── audio_bitrate_tests/     # Priority 4 files
    ├── test_audio_1min_320kbps.mp3
    ├── test_audio_1min_128kbps.mp3
    ├── test_audio_1min_64kbps.mp3
    ├── test_audio_1min_256kbps_aac.m4a
    ├── test_audio_1min_192kbps.ogg
    └── test_audio_5min_128kbps.ogg
```

**Update COMPLETE_TEST_FILE_INVENTORY.md**:
- Add new files to inventory
- Document file characteristics (size, duration, codec, resolution, bitrate)
- Update coverage summary

### Phase 3: Validate New Files

**Validation checklist** (per file):
- [ ] File plays correctly in VLC/QuickTime
- [ ] ffprobe reports correct duration/codec/resolution
- [ ] video-extract can process the file
- [ ] File size is reasonable (not too large for CI)

**Validation command**:
```bash
# Check file properties
ffprobe -v quiet -print_format json -show_format -show_streams file.mp4

# Test with video-extract
VIDEO_EXTRACT_THREADS=4 ./target/release/video-extract debug -o keyframes file.mp4
```

---

## Expected Results

**After implementation**:
- ✅ 25 new synthetic test files
- ✅ Duration coverage: 30s, 1min, 2min, 5min, 10min, 15min, 30min
- ✅ Codec coverage: H.264, H.265, VP9, AV1
- ✅ Resolution coverage: 720p, 1080p, 1440p
- ✅ Audio bitrate coverage: 64kbps, 128kbps, 192kbps, 256kbps, 320kbps
- ✅ Format coverage: MP4, WEBM, M4A, OGG Vorbis

**Total test files** (after addition):
- Video: 1626+ (1600+ existing + 26 new)
- Audio: 116+ (100+ existing + 16 new)
- Total: 1742+ files

**Benefits**:
1. **Reliable duration benchmarks**: 3+ samples per duration range
2. **Codec comparison**: Same duration/resolution, different codecs
3. **Resolution scaling**: Same duration/codec, different resolutions
4. **Audio quality comparison**: Same duration, different bitrates
5. **Comprehensive coverage**: No gaps in common use cases

---

## Next Steps

**N=146**: Generate Phase 1 files (10 duration diversity files)
1. Run FFmpeg generation commands
2. Validate files with ffprobe + video-extract
3. Organize into test_media_generated/duration_tests/
4. Update COMPLETE_TEST_FILE_INVENTORY.md

**N=147**: Generate Phase 2-4 files (15 codec/resolution/audio diversity files)
1. Generate remaining files
2. Validate all new files
3. Complete directory organization
4. Finalize inventory documentation

**After file generation complete**: Begin TEST_EXPANSION_BEFORE_OPTIMIZATION.md implementation
