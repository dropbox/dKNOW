# Phase 1 Corrected Test Paths (50 tests)

**Generated:** N=116 (2025-11-08)
**Source:** tests/smoke_test_comprehensive.rs (actual paths extracted from code)
**Purpose:** Corrected version of PHASE_1_SAMPLING_PLAN.md with real file paths

---

## Note on N=112 Sampling Plan

N=112's PHASE_1_SAMPLING_PLAN.md used hypothetical file paths that don't exist:
- **Hypothetical:** `test_files_camera_raw_samples/arw/sample.arw`
- **Actual:** `test_files_camera_raw/sony_a55.arw`

This document provides the corrected paths extracted from actual test code.

---

## Category 1: RAW Format Tests (10 tests)

### 1. smoke_format_arw_face_detection
- **File:** `test_files_camera_raw/sony_a55.arw`
- **Operations:** `keyframes;face-detection`
- **Format:** ARW (Sony RAW)

### 2. smoke_format_arw_object_detection
- **File:** `test_files_camera_raw/sony_a55.arw`
- **Operations:** `keyframes;object-detection`
- **Format:** ARW (Sony RAW)

### 3. smoke_format_cr2_face_detection
- **File:** `test_files_camera_raw/canon_eos_m.cr2`
- **Operations:** `keyframes;face-detection`
- **Format:** CR2 (Canon RAW)

### 4. smoke_format_cr2_object_detection
- **File:** `test_files_camera_raw/canon_eos_m.cr2`
- **Operations:** `keyframes;object-detection`
- **Format:** CR2 (Canon RAW)

### 5. smoke_format_dng_face_detection
- **File:** `test_files_camera_raw/iphone7_plus.dng`
- **Operations:** `keyframes;face-detection`
- **Format:** DNG (Adobe Digital Negative)

### 6. smoke_format_dng_ocr
- **File:** `test_files_camera_raw/iphone7_plus.dng`
- **Operations:** `keyframes;ocr`
- **Format:** DNG (Adobe Digital Negative)

### 7. smoke_format_nef_face_detection
- **File:** `test_files_camera_raw/nikon_z7.nef`
- **Operations:** `keyframes;face-detection`
- **Format:** NEF (Nikon RAW)

### 8. smoke_format_nef_pose_estimation
- **File:** `test_files_camera_raw/nikon_z7.nef`
- **Operations:** `keyframes;pose-estimation`
- **Format:** NEF (Nikon RAW)

### 9. smoke_format_raf_face_detection
- **File:** `test_files_camera_raw/fuji_xa3.raf`
- **Operations:** `keyframes;face-detection`
- **Format:** RAF (Fujifilm RAW)

### 10. smoke_format_raf_object_detection
- **File:** `test_files_camera_raw/fuji_xa3.raf`
- **Operations:** `keyframes;object-detection`
- **Format:** RAF (Fujifilm RAW)

---

## Category 2: New Video Formats (10 tests)

### 11. smoke_format_mxf_face_detection
- **File:** `test_files_wikimedia/mxf/keyframes/C0023S01.mxf`
- **Operations:** `keyframes;face-detection`
- **Format:** MXF (Material Exchange Format)

### 12. smoke_format_mxf_object_detection
- **File:** `test_files_wikimedia/mxf/keyframes/C0023S01.mxf`
- **Operations:** `keyframes;object-detection`
- **Format:** MXF (Material Exchange Format)

### 13. smoke_format_vob_face_detection
- **File:** `test_files_wikimedia/vob/emotion-detection/03_test.vob`
- **Operations:** `keyframes;face-detection`
- **Format:** VOB (DVD Video Object)

### 14. smoke_format_vob_emotion_detection
- **File:** `test_files_wikimedia/vob/emotion-detection/03_test.vob`
- **Operations:** `keyframes;emotion-detection`
- **Format:** VOB (DVD Video Object)

### 15. smoke_format_asf_face_detection
- **File:** `test_files_wikimedia/asf/emotion-detection/03_test.asf`
- **Operations:** `keyframes;face-detection`
- **Format:** ASF (Advanced Systems Format)

### 16. smoke_format_asf_emotion_detection
- **File:** `test_files_wikimedia/asf/emotion-detection/03_test.asf`
- **Operations:** `keyframes;emotion-detection`
- **Format:** ASF (Advanced Systems Format)

### 17. smoke_format_alac_transcription
- **File:** `test_files_wikimedia/alac/transcription/03_acompanyament_tema.m4a`
- **Operations:** `transcription`
- **Format:** ALAC (Apple Lossless Audio Codec)

### 18. smoke_format_alac_profanity_detection
- **File:** `test_files_wikimedia/alac/transcription/03_acompanyament_tema.m4a`
- **Operations:** `transcription;profanity-detection`
- **Format:** ALAC (Apple Lossless Audio Codec)

### 19. smoke_format_alac_audio_enhancement_metadata
- **File:** `test_files_wikimedia/alac/audio-enhancement-metadata/03_acompanyament_tema.m4a`
- **Operations:** `audio-extraction;audio-enhancement-metadata`
- **Format:** ALAC (Apple Lossless Audio Codec)

### 20. smoke_format_mkv_transcription
- **File:** `test_edge_cases/video_tiny_64x64_resolution__scaling_test.mp4`
- **Operations:** `audio-extraction;transcription`
- **Format:** MKV (Matroska)

---

## Category 3: Audio Advanced Operations (10 tests)

### 21. smoke_format_mp3_profanity_detection
- **File:** `test_files_wikimedia/mp3/transcription/02_Carla_Scaletti_on_natural_sounds_and_physical_modeling_(1999).mp3`
- **Operations:** `transcription;profanity-detection`
- **Format:** MP3

### 22. smoke_format_mp3_audio_enhancement_metadata
- **File:** `test_files_wikimedia/mp3/transcription/02_Carla_Scaletti_on_natural_sounds_and_physical_modeling_(1999).mp3`
- **Operations:** `audio-extraction;audio-enhancement-metadata`
- **Format:** MP3

### 23. smoke_format_m4a_profanity_detection
- **File:** `test_files_wikimedia/m4a/transcription/zoom_audio_sept18.m4a`
- **Operations:** `transcription;profanity-detection`
- **Format:** M4A

### 24. smoke_format_m4a_audio_enhancement_metadata
- **File:** `test_files_wikimedia/m4a/transcription/zoom_audio_sept18.m4a`
- **Operations:** `audio-extraction;audio-enhancement-metadata`
- **Format:** M4A

### 25. smoke_format_ogg_profanity_detection
- **File:** `test_edge_cases/format_test_ogg.ogg`
- **Operations:** `transcription;profanity-detection`
- **Format:** OGG

### 26. smoke_format_ogg_audio_enhancement_metadata
- **File:** `test_edge_cases/format_test_ogg.ogg`
- **Operations:** `audio-extraction;audio-enhancement-metadata`
- **Format:** OGG

### 27. smoke_format_flac_profanity_detection
- **File:** `test_files_wikimedia/flac/transcription/04_Aina_zilizo_hatarini.flac.flac`
- **Operations:** `transcription;profanity-detection`
- **Format:** FLAC

### 28. smoke_format_flac_audio_enhancement_metadata
- **File:** `test_files_wikimedia/flac/transcription/04_Aina_zilizo_hatarini.flac.flac`
- **Operations:** `audio-extraction;audio-enhancement-metadata`
- **Format:** FLAC

### 29. smoke_format_wav_profanity_detection
- **File:** `test_files_wikimedia/wav/audio-enhancement-metadata/03_LL-Q150_(fra)-WikiLucas00-audio.wav`
- **Operations:** `transcription;profanity-detection`
- **Format:** WAV

### 30. smoke_format_wav_audio_enhancement_metadata
- **File:** `test_files_wikimedia/wav/audio-enhancement-metadata/03_LL-Q150_(fra)-WikiLucas00-audio.wav`
- **Operations:** `audio-extraction;audio-enhancement-metadata`
- **Format:** WAV

---

## Category 4: Video Advanced Operations (10 tests)

### 31. smoke_format_mp4_emotion_detection
- **File:** `test_edge_cases/video_hevc_h265_modern_codec__compatibility.mp4`
- **Operations:** `keyframes;emotion-detection`
- **Format:** MP4

### 32. smoke_format_mp4_action_recognition
- **File:** `test_edge_cases/video_high_fps_120__temporal_test.mp4`
- **Operations:** `keyframes;action-recognition`
- **Format:** MP4

### 33. smoke_format_mov_emotion_detection
- **File:** `test_edge_cases/video_no_audio_stream__error_test.mov`
- **Operations:** `keyframes;emotion-detection`
- **Format:** MOV

### 34. smoke_format_mov_action_recognition
- **File:** `test_edge_cases/video_no_audio_stream__error_test.mov`
- **Operations:** `keyframes;action-recognition`
- **Format:** MOV

### 35. smoke_format_webm_emotion_detection
- **File:** `test_edge_cases/video_single_frame_only__minimal.mp4`
- **Operations:** `keyframes;emotion-detection`
- **Format:** WEBM

### 36. smoke_format_webm_action_recognition
- **File:** `test_edge_cases/video_high_fps_120__temporal_test.mp4`
- **Operations:** `keyframes;action-recognition`
- **Format:** WEBM

### 37. smoke_format_mkv_emotion_detection
- **File:** `test_edge_cases/video_tiny_64x64_resolution__scaling_test.mp4`
- **Operations:** `keyframes;emotion-detection`
- **Format:** MKV

### 38. smoke_format_mkv_action_recognition
- **File:** `test_edge_cases/video_high_fps_120__temporal_test.mp4`
- **Operations:** `keyframes;action-recognition`
- **Format:** MKV

### 39. smoke_format_avi_emotion_detection
- **File:** `test_files_wikimedia/avi/keyframes/04_generated_from_webm.avi`
- **Operations:** `keyframes;emotion-detection`
- **Format:** AVI

### 40. smoke_format_avi_action_recognition
- **File:** `test_files_wikimedia/avi/keyframes/04_generated_from_webm.avi`
- **Operations:** `keyframes;action-recognition`
- **Format:** AVI

---

## Category 5: Random Sampling (10 tests)

### 41. smoke_format_jpg_face_detection
- **File:** `test_files_wikimedia/jpg/object-detection/01_0butterfly1_up-08316.jpg`
- **Operations:** `face-detection`
- **Format:** JPG

### 42. smoke_format_jpg_ocr
- **File:** `test_files_wikimedia/jpg/object-detection/01_0butterfly1_up-08316.jpg`
- **Operations:** `ocr`
- **Format:** JPG

### 43. smoke_format_png_object_detection
- **File:** `test_files_wikimedia/png/emotion-detection/02_123inkt_logo_transparent_bg_small.png`
- **Operations:** `object-detection`
- **Format:** PNG

### 44. smoke_format_png_ocr
- **File:** `test_files_wikimedia/png/emotion-detection/02_123inkt_logo_transparent_bg_small.png`
- **Operations:** `ocr`
- **Format:** PNG

### 45. smoke_format_bmp_object_detection
- **File:** `test_files_wikimedia/bmp/emotion-detection/01_bmp_24bit.bmp`
- **Operations:** `object-detection`
- **Format:** BMP

### 46. smoke_format_heic_face_detection
- **File:** `test_edge_cases/image_iphone_photo.heic`
- **Operations:** `keyframes;face-detection`
- **Format:** HEIC

### 47. smoke_format_webp_object_detection
- **File:** `test_files_wikimedia/webp/emotion-detection/01_webp_lossy.webp`
- **Operations:** `object-detection`
- **Format:** WEBP

### 48. smoke_format_mp4_transcription
- **File:** `test_edge_cases/video_hevc_h265_modern_codec__compatibility.mp4`
- **Operations:** `audio-extraction;transcription`
- **Format:** MP4

### 49. smoke_format_webm_transcription
- **File:** `test_media_generated/test_vp9_opus_10s.webm`
- **Operations:** `audio-extraction;transcription`
- **Format:** WEBM

### 50. smoke_format_flv_transcription
- **File:** `test_edge_cases/format_test_flv.flv`
- **Operations:** `audio-extraction;transcription`
- **Format:** FLV

---

## Summary

- **Total tests:** 50
- **All paths extracted from actual test code** (not hypothetical)
- **Verified:** All test functions exist in smoke_test_comprehensive.rs
- **Verified:** All file paths match test code

---

## Usage

### For Structural Verification (without API key)

```bash
# Run single test
./target/release/video-extract debug --ops keyframes;face-detection test_files_camera_raw/sony_a55.arw

# Or use test framework
VIDEO_EXTRACT_THREADS=4 cargo test --release --test smoke_test_comprehensive smoke_format_arw_face_detection -- --ignored
```

### For Semantic Verification (with ANTHROPIC_API_KEY)

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
bash scripts/run_phase1_verification.sh
```

---

**End of PHASE_1_CORRECTED_PATHS.md**
