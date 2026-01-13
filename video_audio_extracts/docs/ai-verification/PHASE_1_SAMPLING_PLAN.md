# Phase 1 AI Verification Sampling Plan

**Date:** 2025-11-08
**N:** 112
**Status:** READY TO EXECUTE (requires ANTHROPIC_API_KEY)
**Target:** 50 tests from 275 new tests added in N=93-109

---

## Prerequisites

**CRITICAL:** Set the ANTHROPIC_API_KEY environment variable:

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
```

Without this, the verification script will fail with:
```
ValueError: ANTHROPIC_API_KEY environment variable not set
```

---

## Sampling Strategy Overview

Distribution of 50 Phase 1 tests:
- 10 RAW format tests (camera raw formats)
- 10 New video format tests (MXF, VOB, ASF)
- 10 Audio advanced operation tests
- 10 Video advanced operation tests
- 10 Random sampling from other categories

---

## Phase 1 Test List (50 tests)

### Category 1: RAW Format Tests (10 tests)

Vision operations on camera RAW formats to verify RAW decode + ML pipeline:

1. **ARW + face-detection**
   - Test: `smoke_format_arw_face_detection`
   - File: `test_files_camera_raw_samples/arw/sample.arw`
   - Operations: `face-detection`

2. **ARW + object-detection**
   - Test: `smoke_format_arw_object_detection`
   - File: `test_files_camera_raw_samples/arw/sample.arw`
   - Operations: `object-detection`

3. **CR2 + face-detection**
   - Test: `smoke_format_cr2_face_detection`
   - File: `test_files_camera_raw_samples/cr2/sample.cr2`
   - Operations: `face-detection`

4. **CR2 + object-detection**
   - Test: `smoke_format_cr2_object_detection`
   - File: `test_files_camera_raw_samples/cr2/sample.cr2`
   - Operations: `object-detection`

5. **DNG + face-detection**
   - Test: `smoke_format_dng_face_detection`
   - File: `test_files_camera_raw_samples/dng/sample.dng`
   - Operations: `face-detection`

6. **DNG + ocr**
   - Test: `smoke_format_dng_ocr`
   - File: `test_files_camera_raw_samples/dng/sample.dng`
   - Operations: `ocr`

7. **NEF + face-detection**
   - Test: `smoke_format_nef_face_detection`
   - File: `test_files_camera_raw_samples/nef/sample.nef`
   - Operations: `face-detection`

8. **NEF + pose-estimation**
   - Test: `smoke_format_nef_pose_estimation`
   - File: `test_files_camera_raw_samples/nef/sample.nef`
   - Operations: `pose-estimation`

9. **RAF + face-detection**
   - Test: `smoke_format_raf_face_detection`
   - File: `test_files_camera_raw_samples/raf/sample.raf`
   - Operations: `face-detection`

10. **RAF + object-detection**
    - Test: `smoke_format_raf_object_detection`
    - File: `test_files_camera_raw_samples/raf/sample.raf`
    - Operations: `object-detection`

---

### Category 2: New Video Formats (10 tests)

MXF, VOB, ASF with vision operations:

11. **MXF + face-detection**
    - Test: `smoke_format_mxf_face_detection`
    - File: `test_files_wikimedia/mxf/keyframes/C0023S01.mxf`
    - Operations: `keyframes;face-detection`

12. **MXF + object-detection**
    - Test: `smoke_format_mxf_object_detection`
    - File: `test_files_wikimedia/mxf/keyframes/C0023S01.mxf`
    - Operations: `keyframes;object-detection`

13. **MXF + ocr**
    - Test: `smoke_format_mxf_ocr`
    - File: `test_files_wikimedia/mxf/keyframes/C0023S01.mxf`
    - Operations: `keyframes;ocr`

14. **VOB + face-detection**
    - Test: `smoke_format_vob_face_detection`
    - File: `test_files_wikimedia/vob/Carrie Fisher Tribute at the Saturn Awards.vob`
    - Operations: `keyframes;face-detection`

15. **VOB + object-detection**
    - Test: `smoke_format_vob_object_detection`
    - File: `test_files_wikimedia/vob/Carrie Fisher Tribute at the Saturn Awards.vob`
    - Operations: `keyframes;object-detection`

16. **VOB + scene-detection**
    - Test: `smoke_format_vob_scene_detection`
    - File: `test_files_wikimedia/vob/Carrie Fisher Tribute at the Saturn Awards.vob`
    - Operations: `keyframes;scene-detection`

17. **ASF + face-detection**
    - Test: `smoke_format_asf_face_detection`
    - File: `test_files_wikimedia/asf/Carrie Fisher Tribute at the Saturn Awards.asf`
    - Operations: `keyframes;face-detection`

18. **ASF + object-detection**
    - Test: `smoke_format_asf_object_detection`
    - File: `test_files_wikimedia/asf/Carrie Fisher Tribute at the Saturn Awards.asf`
    - Operations: `keyframes;object-detection`

19. **ASF + ocr**
    - Test: `smoke_format_asf_ocr`
    - File: `test_files_wikimedia/asf/Carrie Fisher Tribute at the Saturn Awards.asf`
    - Operations: `keyframes;ocr`

20. **ASF + scene-detection**
    - Test: `smoke_format_asf_scene_detection`
    - File: `test_files_wikimedia/asf/Carrie Fisher Tribute at the Saturn Awards.asf`
    - Operations: `keyframes;scene-detection`

---

### Category 3: Audio Advanced Operations (10 tests)

Profanity detection and audio enhancement across formats:

21. **MP4 + profanity-detection**
    - Test: `smoke_format_mp4_profanity_detection`
    - File: `test_edge_cases/video_test_av1.mp4`
    - Operations: `profanity-detection`

22. **MKV + profanity-detection**
    - Test: `smoke_format_mkv_profanity_detection`
    - File: `test_edge_cases/video_test_vp9.mkv`
    - Operations: `profanity-detection`

23. **MXF + profanity-detection**
    - Test: `smoke_format_mxf_profanity_detection`
    - File: `test_files_wikimedia/mxf/audio_extraction/C0023S01.mxf`
    - Operations: `profanity-detection`

24. **FLAC + profanity-detection**
    - Test: `smoke_format_flac_profanity_detection`
    - File: `test_files_audio/flac/sample.flac`
    - Operations: `profanity-detection`

25. **ALAC + profanity-detection**
    - Test: `smoke_format_alac_profanity_detection`
    - File: `test_files_audio/alac/sample.m4a`
    - Operations: `profanity-detection`

26. **MP4 + audio-enhancement-metadata**
    - Test: `smoke_format_mp4_audio_enhancement_metadata`
    - File: `test_edge_cases/video_test_av1.mp4`
    - Operations: `audio-enhancement-metadata`

27. **MKV + audio-enhancement-metadata**
    - Test: `smoke_format_mkv_audio_enhancement_metadata`
    - File: `test_edge_cases/video_test_vp9.mkv`
    - Operations: `audio-enhancement-metadata`

28. **MXF + audio-enhancement-metadata**
    - Test: `smoke_format_mxf_audio_enhancement_metadata`
    - File: `test_files_wikimedia/mxf/audio_extraction/C0023S01.mxf`
    - Operations: `audio-enhancement-metadata`

29. **ALAC + audio-enhancement-metadata**
    - Test: `smoke_format_alac_audio_enhancement_metadata`
    - File: `test_files_audio/alac/sample.m4a`
    - Operations: `audio-enhancement-metadata`

30. **WAV + audio-enhancement-metadata**
    - Test: `smoke_format_wav_audio_enhancement_metadata`
    - File: `test_files_audio/wav/sample.wav`
    - Operations: `audio-enhancement-metadata`

---

### Category 4: Video Advanced Operations (10 tests)

Action recognition and emotion detection across formats:

31. **MP4 + action-recognition**
    - Test: `smoke_format_mp4_action_recognition`
    - File: `test_edge_cases/video_test_av1.mp4`
    - Operations: `keyframes;action-recognition`

32. **MKV + action-recognition**
    - Test: `smoke_format_mkv_action_recognition`
    - File: `test_edge_cases/video_test_vp9.mkv`
    - Operations: `keyframes;action-recognition`

33. **MXF + action-recognition**
    - Test: `smoke_format_mxf_action_recognition`
    - File: `test_files_wikimedia/mxf/keyframes/C0023S01.mxf`
    - Operations: `keyframes;action-recognition`

34. **VOB + action-recognition**
    - Test: `smoke_format_vob_action_recognition`
    - File: `test_files_wikimedia/vob/Carrie Fisher Tribute at the Saturn Awards.vob`
    - Operations: `keyframes;action-recognition`

35. **ASF + action-recognition**
    - Test: `smoke_format_asf_action_recognition`
    - File: `test_files_wikimedia/asf/Carrie Fisher Tribute at the Saturn Awards.asf`
    - Operations: `keyframes;action-recognition`

36. **MP4 + emotion-detection**
    - Test: `smoke_format_mp4_emotion_detection`
    - File: `test_edge_cases/video_test_av1.mp4`
    - Operations: `keyframes;emotion-detection`

37. **MKV + emotion-detection**
    - Test: `smoke_format_mkv_emotion_detection`
    - File: `test_edge_cases/video_test_vp9.mkv`
    - Operations: `keyframes;emotion-detection`

38. **MXF + emotion-detection**
    - Test: `smoke_format_mxf_emotion_detection`
    - File: `test_files_wikimedia/mxf/keyframes/C0023S01.mxf`
    - Operations: `keyframes;emotion-detection`

39. **VOB + emotion-detection**
    - Test: `smoke_format_vob_emotion_detection`
    - File: `test_files_wikimedia/vob/Carrie Fisher Tribute at the Saturn Awards.vob`
    - Operations: `keyframes;emotion-detection`

40. **ASF + emotion-detection**
    - Test: `smoke_format_asf_emotion_detection`
    - File: `test_files_wikimedia/asf/Carrie Fisher Tribute at the Saturn Awards.asf`
    - Operations: `keyframes;emotion-detection`

---

### Category 5: Random Sampling (10 tests)

Diverse operations across various formats:

41. **ARW + vision-embeddings**
    - Test: `smoke_format_arw_vision_embeddings`
    - File: `test_files_camera_raw_samples/arw/sample.arw`
    - Operations: `vision-embeddings`

42. **DNG + image-quality-assessment**
    - Test: `smoke_format_dng_image_quality_assessment`
    - File: `test_files_camera_raw_samples/dng/sample.dng`
    - Operations: `image-quality-assessment`

43. **MXF + pose-estimation**
    - Test: `smoke_format_mxf_pose_estimation`
    - File: `test_files_wikimedia/mxf/keyframes/C0023S01.mxf`
    - Operations: `keyframes;pose-estimation`

44. **VOB + vision-embeddings**
    - Test: `smoke_format_vob_vision_embeddings`
    - File: `test_files_wikimedia/vob/Carrie Fisher Tribute at the Saturn Awards.vob`
    - Operations: `keyframes;vision-embeddings`

45. **ASF + vision-embeddings**
    - Test: `smoke_format_asf_vision_embeddings`
    - File: `test_files_wikimedia/asf/Carrie Fisher Tribute at the Saturn Awards.asf`
    - Operations: `keyframes;vision-embeddings`

46. **ALAC + audio-embeddings**
    - Test: `smoke_format_alac_audio_embeddings`
    - File: `test_files_audio/alac/sample.m4a`
    - Operations: `audio-embeddings`

47. **ALAC + diarization**
    - Test: `smoke_format_alac_diarization`
    - File: `test_files_audio/alac/sample.m4a`
    - Operations: `diarization`

48. **MXF + smart-thumbnail**
    - Test: `smoke_format_mxf_smart_thumbnail`
    - File: `test_files_wikimedia/mxf/keyframes/C0023S01.mxf`
    - Operations: `keyframes;smart-thumbnail`

49. **VOB + shot-classification**
    - Test: `smoke_format_vob_shot_classification`
    - File: `test_files_wikimedia/vob/Carrie Fisher Tribute at the Saturn Awards.vob`
    - Operations: `keyframes;shot-classification`

50. **ASF + shot-classification**
    - Test: `smoke_format_asf_shot_classification`
    - File: `test_files_wikimedia/asf/Carrie Fisher Tribute at the Saturn Awards.asf`
    - Operations: `keyframes;shot-classification`

---

## Verification Workflow

For each test:

1. **Generate Output:**
   ```bash
   ./target/release/video-extract debug --ops <operations> <input_file>
   ```

2. **AI Verification:**
   ```bash
   python scripts/ai_verify_outputs.py \
       <input_file> \
       debug_output/stage_XX_<operation>.json \
       <operation>
   ```

3. **Parse JSON Response:**
   - Extract status (CORRECT/SUSPICIOUS/INCORRECT)
   - Extract confidence score
   - Extract findings
   - Extract errors

4. **Document Result:**
   - Add to verification report
   - Track confidence distribution
   - Flag issues for investigation

---

## Success Criteria

**Phase 1 Goals:**
- Complete 50 test verifications
- Achieve ≥90% confidence on ≥95% of tests (≥48 tests with confidence ≥0.90)
- Document all SUSPICIOUS and INCORRECT findings
- Investigate and fix any bugs discovered

---

## Output Location

**Verification Report:** `docs/ai-verification/NEW_TESTS_AI_VERIFICATION_REPORT.md`

**Debug Output:** `debug_output/` (created per-test, not committed)

---

## Timeline Estimate

- N=112: Create sampling plan and execution script (current)
- N=113: Execute first 25 verifications, document results
- N=114: Execute remaining 25 verifications, document results
- N=115: Investigate SUSPICIOUS/INCORRECT findings, fix bugs if found

**Per-test time estimate:**
- Generate output: 5-30 seconds (depending on operation)
- AI verification: 5-15 seconds (API call + parsing)
- Documentation: 1-2 minutes
- **Total per test: ~2-3 minutes**
- **50 tests: ~2 hours of AI work**

---

## Next Steps

1. Set `ANTHROPIC_API_KEY` environment variable
2. Run execution script (to be created)
3. Monitor progress and document results
4. Investigate any issues found
5. Fix bugs if discovered

---

**End of PHASE_1_SAMPLING_PLAN.md**
