# N=119: Phase 2 AI Verification Findings

**Date:** 2025-11-09
**Worker:** N=119 (continuation of N=119 investigation work)
**Tests Attempted:** 30
**Tests Successfully Executed:** 6/30 (20%)
**Verification Success Rate:** 2/6 correct (33%)

---

## Summary

Phase 2 verification script created but revealed major test file selection issue. Only 6/30 tests executed successfully, 24 failed due to incorrect file paths. Of the 6 successful tests:
- ✅ 2 CORRECT (33%)
- ⚠️ 2 SUSPICIOUS (33%)
- ❌ 2 INCORRECT (33%)

**Key Findings:**
1. Object detection misses obvious objects (tulips, praying mantis) - **Confidence threshold issue suspected**
2. Emotion detection runs on non-face images (landscape) - **Plugin validation needed**
3. Whisper "Blaine" hallucination persists (already documented in N=118/N=119)
4. File path methodology needs improvement for Phase 3

---

## Successful Tests (6/30)

### ✅ CORRECT (2 tests)

1. **webp_face_1** (confidence=1.0)
   - File: `test_files_wikimedia/webp/emotion-detection/01_webp_lossy.webp`
   - Operation: face-detection
   - Result: Correctly detected no faces in landscape image

2. **wav_transcript_3** (confidence=0.95)
   - File: `test_files_wikimedia/wav/audio-enhancement-metadata/03_LL-Q150_(fra)-WikiLucas00-audio.wav`
   - Operation: transcription
   - Result: Correctly transcribed "Good job." with acceptable language_probability
   - Note: Metadata markers `[_BEG_]` and `[_TT_50]` no longer present (N=119 fix working)

### ⚠️ SUSPICIOUS (2 tests)

1. **jpg_object_1** (confidence=0.8)
   - File: `test_files_wikimedia/jpg/object-detection/01_2012_Tulip_Festival.jpg`
   - Operation: object-detection
   - Issue: "The image contains clearly visible tulips, but no objects have been detected."
   - Analysis: YOLOv8n may have confidence threshold too high or flowers not in COCO classes

2. **mp3_transcript_1** (confidence=0.85)
   - File: `test_files_wikimedia/mp3/transcription/02_Carla_Scaletti_on_natural_sounds_and_physical_modeling_(1999).mp3`
   - Operation: transcription
   - Issue: "Blaine" mishearing (should be "which explains") - Already documented in N=118/N=119
   - Analysis: Whisper model limitation, not new issue

### ❌ INCORRECT (2 tests)

1. **jpg_object_2** (confidence=0.7)
   - File: `test_files_wikimedia/jpg/object-detection/01_139235_green_praying_mantis_PikiWiki_Israel.jpg`
   - Operation: object-detection
   - Issue: "The image clearly shows a green praying mantis, but no objects were detected."
   - Analysis: YOLOv8n confidence threshold or COCO classes don't include insects

2. **webp_emotion_1** (confidence=1.0)
   - File: `test_files_wikimedia/webp/emotion-detection/01_webp_lossy.webp`
   - Operation: emotion-detection
   - Issue: "The image is of a natural landscape with mountains and a river, not of a person or face. Emotion-detection does not apply to this image."
   - Analysis: Plugin should validate presence of faces before running emotion detection

---

## Failed Tests (24/30)

All 24 failures were due to incorrect file paths (files don't exist at specified paths). Examples:
- `test_files_wikimedia/jpg/face-detection/02_123Makossa.jpg` - doesn't exist
- `test_files_wikimedia/png/face-detection/02_1st_Lt._Ryan_Mierau,_Colorado_Springs,_Colo.png` - doesn't exist
- `test_files_wikimedia/mp3/transcription/file_example_MP3_1MG.mp3` - doesn't exist

**Root Cause:** Verification script used guessed filenames instead of actual file inventory.

---

## New Issues Identified

### 1. Object Detection Missing Obvious Objects (HIGH PRIORITY)

**Observations:**
- Tulips in "2012 Tulip Festival" image not detected
- Green praying mantis clearly visible but not detected

**Suspected Root Causes:**
- Confidence threshold too high (current: 0.5 in config/plugins/object_detection.yaml)
- COCO dataset taxonomy limitations (no "flower" or "insect" classes)
- YOLOv8n model size too small for diverse objects

**Recommended Actions:**
1. Lower confidence threshold from 0.5 to 0.25-0.3
2. Test with YOLOv8m or YOLOv8l (larger models)
3. Consider fine-tuning on plant/insect detection dataset
4. Document COCO class taxonomy limitations for users

### 2. Emotion Detection Runs on Non-Face Images (MEDIUM PRIORITY)

**Observation:**
Emotion detection operation ran on landscape image (no faces) and returned emotion="angry" with confidence=0.253

**Root Cause:**
Plugin doesn't validate presence of faces before running emotion detection

**Recommended Actions:**
1. Add face detection prerequisite to emotion-detection plugin
2. Return error or empty result if no faces detected
3. Update plugin config to require face_detection dependency

### 3. Whisper Hallucination Persists

**Observation:**
"Blaine why" mishearing in Carla Scaletti audio (already documented in N=118/N=119)

**Status:**
Known Whisper limitation, no new information

---

## Test File Path Methodology Issue

**Problem:**
Guessing filenames led to 80% test failure rate (24/30 files didn't exist)

**Solution for Phase 3:**
1. Use `find` or `ls` to enumerate actual files in test directories
2. Create verified file inventory (JSON or CSV)
3. Sample from inventory instead of guessing filenames
4. Prioritize files used in existing smoke tests (known to exist)

---

## Cost Analysis

**Phase 2 Verification:**
- API calls: 6 successful (4 vision, 2 transcription) + 24 failed = 30 attempted
- Actual API cost: ~6 calls (only successful tests make API calls)
- Model: gpt-4o (vision)
- Estimated cost: ~$0.10-0.30
- Time: ~1 minute (most tests failed at binary execution)

---

## Recommendations for N=120

### Immediate (N=120)

1. **Fix Object Detection Confidence Threshold**
   - Lower threshold from 0.5 to 0.3 in config/plugins/object_detection.yaml
   - Re-run verification on tulip and praying mantis tests
   - Document if objects detected with lower threshold

2. **Add Face Detection Validation to Emotion Detection**
   - Modify emotion-detection plugin to check for faces first
   - Return error if no faces detected
   - Test with landscape image to verify fix

3. **Create Verified Test File Inventory**
   - Use `find` or `ls` to create actual file list
   - Save to `docs/ai-verification/VERIFIED_TEST_FILES.json`
   - Use this inventory for Phase 3 verification

### Short-term (N=121-125)

1. **Re-run Phase 2 with Correct File Paths**
   - Fix all 24 file path errors
   - Target 30 successful executions
   - Aim for ≥90% CORRECT rate

2. **Expand to 50-100 Tests (Phase 3)**
   - Use verified file inventory
   - Broader format coverage (GIF, HEIC, MP4 video)
   - More diverse operations (action-recognition, scene-detection)

### Long-term (N=126+)

1. **Object Detection Model Upgrade**
   - Evaluate YOLOv8m, YOLOv8l for better accuracy
   - Consider fine-tuning for underrepresented classes
   - Document tradeoffs (accuracy vs. speed vs. memory)

2. **Automated Quality Monitoring**
   - Integrate GPT-4 Vision verification into CI/CD
   - Sample 10-20 random tests per commit
   - Alert on SUSPICIOUS/INCORRECT rate >10%

---

## Files Created

- `scripts/verify_phase2.sh`: Phase 2 verification script (30 tests, 80% file path errors)
- `docs/ai-verification/PHASE2_GPT4_VERIFICATION_20251108_203200.csv`: Results CSV (6 successful, 24 failed)
- `docs/ai-verification/N119_PHASE2_FINDINGS.md`: This report

---

## Next Steps

N=120 should focus on fixing the two actionable issues:
1. Object detection confidence threshold (lower from 0.5 to 0.3)
2. Emotion detection face validation (add prerequisite check)

Then create verified test file inventory for Phase 3 verification.

---

**Context for Next AI:**

- N=118: Phase 1 minimal verification (10 tests, 60% CORRECT, 4 issues found)
- N=119: Investigation complete (1 fixed, 2 ML limitations, 2 false positives)
- N=119: Phase 2 verification attempted (6 successful, 2 new actionable issues found)
- Current: Object detection confidence threshold and emotion detection validation need fixes
