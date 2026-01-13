# N=118: Phase 1 AI Verification Findings

**Date:** 2025-11-08
**Worker:** N=118
**Method:** OpenAI GPT-4 Vision API
**Tests Executed:** 10 minimal tests (JPG/PNG/WebP + Transcription)
**Baseline System Status:** 647/647 smoke tests passing (100%)

---

## Summary

**Overall Results:**
- ✅ CORRECT: 6/10 (60%)
- ⚠️ SUSPICIOUS: 2/10 (20%)
- ❌ INCORRECT: 2/10 (20%)
- ❓ ERROR: 0/10 (0%)

**Status:** Verification infrastructure working. Found 4 legitimate issues.

---

## Issues Found

### 1. Object Detection: Inaccurate Label (INCORRECT)
- **Test:** jpg_object
- **File:** `test_files_wikimedia/jpg/object-detection/01_0butterfly1_up-08316.jpg`
- **Issue:** Object detection labeled "flower with spider" as "potted plant"
- **GPT-4 Assessment:** confidence=0.2, INCORRECT
- **Findings:** "The object detected is labeled as a 'potted plant', but the image shows a flower with a spider on it. The label 'potted plant' is inaccurate."
- **Action Required:** Investigate object detection model accuracy

### 2. OCR: Missed Text in Logo (INCORRECT)
- **Test:** png_ocr
- **File:** `test_files_wikimedia/png/emotion-detection/02_123inkt.nl_logo_transparent_bg_small.png`
- **Issue:** OCR failed to detect text "123inkt.nl" visible in logo
- **GPT-4 Assessment:** confidence=0.95, INCORRECT
- **Findings:** "The OCR output is incorrect as it does not include any detected text. The image clearly shows the text '123inkt.nl' on a yellow ink droplet shape."
- **Action Required:** Investigate OCR text detection for logos/stylized text

### 3. Transcription: Nonsensical Phrase (SUSPICIOUS)
- **Test:** mp3_transcript
- **File:** `test_files_wikimedia/mp3/transcription/02_Carla_Scaletti_on_natural_sounds_and_physical_modeling_(1999).mp3`
- **Issue:** Transcription contains nonsensical phrase "Blaine why I don't like the sound of bees"
- **GPT-4 Assessment:** confidence=0.75, SUSPICIOUS
- **Findings:** "There is a notable error around 'which makes Blaine why I don't like the sound of bees,' which is nonsensical as part of the speech. Additionally, there are some minor timing overlaps in words."
- **Action Required:** Investigate transcription model quality (Whisper)

### 4. Transcription: Low Language Confidence + Metadata Markers (SUSPICIOUS)
- **Test:** wav_transcript
- **File:** `test_files_wikimedia/wav/audio-enhancement-metadata/03_LL-Q150_(fra)-WikiLucas00-audio.wav`
- **Issue:** Very low language_probability + metadata markers `[_BEG_]` and `[_TT_50]` in transcription
- **GPT-4 Assessment:** confidence=0.7, SUSPICIOUS
- **Findings:** "The 'language_probability' seems very low for a straightforward 'en' transcription, suggesting low confidence in language identification. Additionally, '[_BEG_]' and '[_TT_50]' appear as metadata markers, which are not typical meaningful components of clean transcriptions."
- **Action Required:** Investigate language detection + clean up metadata markers in transcription output

---

## Tests Passed (6/10)

1. **jpg_face** - Face detection correctly returned empty list (no faces in flower/spider image)
2. **jpg_ocr** - OCR correctly returned no text (no text in flower/spider image)
3. **png_face** - Face detection correctly returned empty list (logo has no faces)
4. **png_object** - Object detection correctly returned empty list (logo not a typical object)
5. **webp_face** - Face detection correctly returned empty list (landscape has no faces)
6. **webp_object** - Object detection correctly returned empty list (landscape has no distinct objects)

---

## Infrastructure Status

### What Works
- ✅ OpenAI GPT-4 Vision API integration
- ✅ Markdown code block stripping (GPT-4 wraps JSON in ```json ... ```)
- ✅ Vision verification for standard formats (JPG, PNG, WebP)
- ✅ Text-based verification (transcription)
- ✅ Semantic correctness assessment
- ✅ Confidence scoring (0.0-1.0)
- ✅ Detailed findings reporting

### Known Limitations
- ❌ RAW format support (ARW, CR2, DNG, NEF, RAF) - GPT-4 Vision doesn't support these formats
- ❌ BMP format support - GPT-4 Vision doesn't support BMP
- ⚠️ HEIC format - Binary execution failing (need to investigate)
- ⚠️ Emotion-detection operation - Binary execution failing for all images (need to investigate)
- ⚠️ Keyframe-based verification - Complex multi-stage operations (keyframes;face-detection) require extracting keyframe image path from stage_00_keyframes.json

### Scripts Created
- `scripts/ai_verify_outputs_openai.py` - Core verification script (fixed markdown stripping)
- `scripts/verify_phase1_openai.sh` - Original 50-test batch script (failed due to RAW format issues)
- `scripts/verify_phase1_simple.sh` - Simplified 30-test batch script (failed due to markdown parsing)
- `scripts/verify_phase1_minimal.sh` - Minimal 10-test script (✅ WORKS, 60% success rate)

---

## Cost Analysis

**Minimal verification (10 tests):**
- API calls: 10
- Model: gpt-4o (vision)
- Estimated cost: ~$0.10-0.50 (depends on image sizes)
- Time: ~3-4 minutes

**Full 50-test verification estimate:**
- API calls: ~50-100 (some tests will fail on unsupported formats)
- Estimated cost: ~$2-5
- Time: ~15-20 minutes

---

## Next Steps

### Immediate (N=119)
1. **Fix HEIC support** - Investigate why binary fails on HEIC images
2. **Fix emotion-detection** - Investigate why emotion-detection operation fails on all images
3. **Investigate object detection accuracy** - "potted plant" mislabeling
4. **Investigate OCR for stylized text** - Missing "123inkt.nl" in logo
5. **Investigate transcription quality** - Nonsensical phrases + metadata markers

### Medium-term (N=120-122)
1. **Expand verification to 30-50 tests** - Focus on supported formats (JPG, PNG, WebP, GIF, MP3, WAV, MP4, etc.)
2. **Create keyframe-based verification** - Handle multi-stage operations (keyframes;face-detection) by extracting keyframe image paths
3. **Document format support matrix** - Which image formats work with GPT-4 Vision (JPG, PNG, WebP, GIF) vs. which don't (RAW, BMP, HEIC)

### Long-term (N=123+)
1. **Scale to 100-200 test verification** - Full semantic validation
2. **Automate issue tracking** - Create GitHub issues for SUSPICIOUS/INCORRECT findings
3. **Continuous verification** - Integrate into CI/CD pipeline

---

## Conclusion

**Phase 1 AI verification infrastructure is functional.** GPT-4 Vision successfully identified 4 legitimate issues across 10 tests:
- 2 INCORRECT (object detection mislabel, OCR missing text)
- 2 SUSPICIOUS (transcription quality issues)

The verification system works as intended. Issues found are real quality problems that need investigation.

**Next worker (N=119) should:**
1. Fix HEIC and emotion-detection binary failures
2. Investigate the 4 issues found
3. Expand verification to 30-50 supported-format tests

---

**Files:**
- Verification script: `scripts/ai_verify_outputs_openai.py` (✅ Working)
- Minimal test script: `scripts/verify_phase1_minimal.sh` (✅ Working)
- Results CSV: `docs/ai-verification/PHASE1_MINIMAL_GPT4_VERIFICATION_20251108_191902.csv`
- This report: `docs/ai-verification/N118_PHASE1_VERIFICATION_FINDINGS.md`
