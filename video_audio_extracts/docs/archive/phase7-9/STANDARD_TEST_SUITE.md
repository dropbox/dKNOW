# Standard Test Suite - Curated Test Files
**Generated**: 2025-10-29
**Purpose**: Definitive list of standard tests for regular validation
**Total**: 24 standard tests (12 formats × 2 tests each)

---

## STANDARD TEST TABLE

| # | Format | File | Size | Test Purpose | Command |
|---|--------|------|------|--------------|---------|
| **VIDEO FORMATS** |
| 1 | MP4 | editing-relevance-rubrics kg may 16 2025.mov | 34MB | Quick video pipeline | `--ops keyframes,object-detection` |
| 2 | MP4 | GMT20250520-223657_Recording_avo_1920x1080.mp4 | 1.3GB | Stress test (large) | `--ops keyframes` |
| 3 | MOV | Screen Recording 2025-06-02 at 11.14.26 AM.mov | 38MB | Screen recording | `--ops keyframes,object-detection` |
| 4 | MOV | GMT20250516-190317_Recording_avo_1920x1080 braintrust.mp4 | 980MB | Stress test (large) | `--ops keyframes,audio-extraction` |
| 5 | AVI | v_juggle_04_04.avi | 530KB | Codec detection test | `--ops keyframes` (expects error) |
| 6 | AVI | v_golf_01_05.avi | 891KB | Codec detection test | `--ops keyframes` (expects error) |
| 7 | MKV | eb9jznQnhK8_raw.mkv | 11MB | Kinetics dataset | `--ops keyframes` |
| 8 | WEBM | JKAWup5iKho_raw.f251.webm | 2.2MB | Kinetics dataset | `--ops keyframes` |
| **AUDIO FORMATS** |
| 9 | M4A | audio1509128771.m4a | 13MB | Zoom meeting (speech) | `--ops transcription,diarization` |
| 10 | M4A | audio1171640589.m4a | 19MB | Zoom meeting (speech) | `--ops transcription` |
| 11 | WAV | State of Affairs_ROUGHMIX.wav | 56MB | Music (instruments) | `--ops transcription` |
| 12 | MP3 | fabula_01_018_esopo_64kb.mp3 | 1.1MB | Audiobook (clean speech) | `--ops transcription,audio-embeddings` |
| 13 | MP3 | count_of_monte_cristo_037_dumas_64kb.mp3 | ~10MB | Audiobook (longer) | `--ops transcription` |
| 14 | FLAC | Sample_BeeMoved_96kHz24bit.flac | 16MB | High-quality audio | `--ops transcription` |
| 15 | AAC | sample_10s_audio-aac.aac | 146KB | Short test audio | `--ops transcription` |
| **IMAGE FORMATS** |
| 16 | WEBP | stoplight.webp | Small | Object detection | `--ops object-detection,ocr` |
| 17 | BMP | rle.bmp | 39KB | Object detection | `--ops object-detection` |
| **EDGE CASES** |
| 18 | MOV | video_no_audio_stream__error_test.mov | 1.7MB | No audio error | `--ops audio-extraction` (expects error) |
| 19 | MP4 | video_single_frame_only__minimal.mp4 | 43KB | Minimal video | `--ops keyframes` |
| 20 | MP4 | video_hevc_h265_modern_codec__compatibility.mp4 | 157KB | HEVC codec | `--ops keyframes` |
| 21 | MP4 | video_4k_ultra_hd_3840x2160__stress_test.mp4 | 153KB | 4K resolution | `--ops keyframes` |
| 22 | WAV | audio_complete_silence_3sec__silence_detection.wav | 517KB | Silent audio | `--ops transcription` |
| 23 | MP3 | audio_lowquality_16kbps__compression_test.mp3 | 20KB | Low bitrate | `--ops transcription` |
| 24 | MP4 | corrupted_truncated_file__error_handling.mp4 | 50KB | Corrupted file | `--ops keyframes` (expects error) |

---

## FILE PATHS (Copy-Paste Ready)

### Video Files
```bash
# MP4
"$HOME/Desktop/stuff/stuff/editing-relevance-rubrics kg may 16 2025.mov"
"$HOME/Desktop/stuff/stuff/GMT20250520-223657_Recording_avo_1920x1080.mp4"

# MOV
"$HOME/Desktop/stuff/stuff/Screen Recording 2025-06-02 at 11.14.26 AM.mov"
"$HOME/Desktop/stuff/stuff/GMT20250516-190317_Recording_avo_1920x1080 braintrust.mp4"

# AVI
"$HOME/Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/action_youtube_naudio/soccer_juggling/v_juggle_04/v_juggle_04_04.avi"
"$HOME/Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/action_youtube_naudio/golf_swing/v_golf_01/v_golf_01_05.avi"

# MKV
"$HOME/Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/Kinetics dataset (5%)/kinetics600_5per/kinetics600_5per/train/ice climbing/eb9jznQnhK8_raw.mkv"

# WEBM
"$HOME/Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/Kinetics dataset (5%)/kinetics600_5per/kinetics600_5per/train/zumba/JKAWup5iKho_raw.f251.webm"
```

### Audio Files
```bash
# M4A
"$HOME/Desktop/stuff/stuff/review existing benchmarks/april meeting conv ai dashboard 2025-08-14 17.42.25 Zoom Meeting/audio1509128771.m4a"
"$HOME/Desktop/stuff/stuff/review existing benchmarks/gonzolo meeting aug 14 /audio1171640589.m4a"

# WAV
"$HOME/Music/Music/Media.localized/Music/Unknown Artist/Unknown Album/State of Affairs_ROUGHMIX.wav"

# MP3
"$HOME/Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/public/librivox/fabula_01_018_esopo_64kb.mp3"
"$HOME/Library/CloudStorage/Dropbox-BrandcraftSolutions/a.test/public/librivox/count_of_monte_cristo_037_dumas_64kb.mp3"

# FLAC
"$HOME/src/server/dropbox/tests/static/audios/Sample_BeeMoved_96kHz24bit.flac"

# AAC  
"$HOME/docling/tests/data/audio/sample_10s_audio-aac.aac"
```

### Image Files
```bash
# WEBP
"$HOME/pdfium/third_party/skia/resources/images/stoplight.webp"

# BMP
"$HOME/pdfium/third_party/skia/resources/images/rle.bmp"
```

### Edge Cases
```bash
# All in test_edge_cases/ directory
"$HOME/video_audio_extracts/test_edge_cases/video_no_audio_stream__error_test.mov"
"$HOME/video_audio_extracts/test_edge_cases/video_single_frame_only__minimal.mp4"
"$HOME/video_audio_extracts/test_edge_cases/video_hevc_h265_modern_codec__compatibility.mp4"
"$HOME/video_audio_extracts/test_edge_cases/video_4k_ultra_hd_3840x2160__stress_test.mp4"
"$HOME/video_audio_extracts/test_edge_cases/audio_complete_silence_3sec__silence_detection.wav"
"$HOME/video_audio_extracts/test_edge_cases/audio_lowquality_16kbps__compression_test.mp3"
"$HOME/video_audio_extracts/test_edge_cases/corrupted_truncated_file__error_handling.mp4"
```

---

## TEST CATEGORIES

### Quick Tests (<5 seconds)
- video_single_frame_only__minimal.mp4 (43KB)
- audio_very_short_1sec__duration_min.wav (94KB)
- stoplight.webp, rle.bmp (images)

### Medium Tests (10-60 seconds)
- editing-relevance-rubrics kg may 16 2025.mov (34MB)
- Screen Recording 2025-06-02 at 11.14.26 AM.mov (38MB)
- eb9jznQnhK8_raw.mkv (11MB)
- audio1509128771.m4a (13MB)
- fabula_01_018_esopo_64kb.mp3 (1.1MB)

### Stress Tests (>60 seconds)
- GMT20250520-223657_Recording_avo_1920x1080.mp4 (1.3GB)
- GMT20250516-190317_Recording_avo_1920x1080 braintrust.mp4 (980MB)
- State of Affairs_ROUGHMIX.wav (56MB)

### Error Tests (expect failure)
- v_juggle_04_04.avi (AVI codec issue)
- video_no_audio_stream__error_test.mov (no audio)
- corrupted_truncated_file__error_handling.mp4 (corrupted)

---

## VALIDATION SUITES

### Suite 1: Format Validation (12 tests, ~5 min)
Tests one file per format to ensure format compatibility:
- MP4, MOV, AVI (error), MKV, WEBM
- M4A, WAV, MP3, FLAC, AAC
- WEBP, BMP

### Suite 2: Performance Validation (6 tests, ~3 min)
Tests cache and parallelism optimizations:
- keyframes → object-detection (cache validation)
- keyframes → audio-extraction (parallel potential)
- Full 7-operation pipeline

### Suite 3: Edge Case Validation (7 tests, ~2 min)
Tests error handling and edge cases:
- No audio, single frame, HEVC, 4K
- Silent audio, low bitrate, corrupted

### Suite 4: Stress Testing (2 tests, ~10 min)
Tests large files:
- 1.3GB video (full pipeline)
- 980MB video (full pipeline)

**Total runtime**: ~20 minutes for complete validation

---

## COVERAGE SUMMARY

**Standard tests**: 24 files
**Total available**: 1,826 files
**Format coverage**: 100% (12/12)
**Size coverage**: 20KB → 1.3GB
**Duration coverage**: 1s → 90min
**Content coverage**: Speech, action, UI, music, silence, corrupted

**Assessment**: Production-ready test suite

