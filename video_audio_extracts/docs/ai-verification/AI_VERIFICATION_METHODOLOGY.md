# AI Verification Methodology

**Date:** 2025-11-08
**Status:** Ready for Execution
**Purpose:** Verify semantic correctness of test outputs using Claude API

---

## Overview

This methodology uses Claude Sonnet 4 with vision capabilities to verify that test outputs are semantically correct, not just structurally valid.

**Structural validation** (current): Checks JSON schema, value ranges, required fields
**Semantic verification** (this methodology): Verifies outputs match actual content

---

## Tool

**Script:** `scripts/ai_verify_outputs.py`

**Usage:**
```bash
export ANTHROPIC_API_KEY="your-api-key-here"

python scripts/ai_verify_outputs.py <input_file> <output_json> <operation>
```

**Example:**
```bash
# Generate output
./target/release/video-extract debug --ops face-detection test_edge_cases/image_test_dog.jpg

# Verify output
python scripts/ai_verify_outputs.py \
    test_edge_cases/image_test_dog.jpg \
    debug_output/stage_00_face_detection.json \
    face-detection
```

**Output format:**
```json
{
  "status": "CORRECT" | "SUSPICIOUS" | "INCORRECT",
  "confidence": 0.95,
  "findings": "Description of what matches or doesn't match",
  "errors": ["list of specific errors found"]
}
```

---

## Sampling Strategy

**Target:** Verify 100 new tests (from 275 added in N=93-109)

### Sample Distribution

**Phase 1: 50 tests (N=111-113)**

1. **RAW format tests (10 tests)**
   - ARW × 2 plugins
   - CR2 × 2 plugins
   - DNG × 2 plugins
   - NEF × 2 plugins
   - RAF × 2 plugins
   - Focus: face-detection, object-detection

2. **New video formats (10 tests)**
   - MXF × 3 plugins (face-detection, object-detection, ocr)
   - VOB × 3 plugins (face-detection, object-detection, ocr)
   - ASF × 4 plugins (face-detection, object-detection, ocr, scene-detection)

3. **Audio advanced operations (10 tests)**
   - profanity-detection × 5 formats
   - audio-enhancement-metadata × 5 formats

4. **Video advanced operations (10 tests)**
   - action-recognition × 5 formats
   - emotion-detection × 5 formats

5. **Random sampling (10 tests)**
   - Randomly selected from remaining new tests
   - Various formats and operations

**Phase 2: 50 additional tests (N=114-115)**

- Continue with remaining test categories
- Focus on any categories showing issues in Phase 1
- Ensure coverage across all new formats

---

## Verification Categories

### Vision Operations (require image input)
- face-detection
- object-detection
- ocr
- pose-estimation
- emotion-detection
- scene-detection
- action-recognition
- shot-classification
- smart-thumbnail
- duplicate-detection
- image-quality-assessment
- vision-embeddings
- keyframes

**Verification method:** Claude views image + output, verifies match

### Text/Audio Operations
- transcription
- diarization
- profanity-detection
- voice-activity-detection
- audio-classification
- acoustic-scene-classification
- audio-embeddings
- audio-enhancement-metadata

**Verification method:** Claude reviews output structure and plausibility

---

## Success Criteria

**Acceptance thresholds:**
- ≥90% confidence score on ≥95% of tests
- All INCORRECT findings require bug investigation
- All SUSPICIOUS findings require manual review

**Confidence scoring:**
- 1.0 = Perfect match
- 0.9-0.99 = Very good, minor discrepancies
- 0.7-0.89 = Good, some issues
- 0.5-0.69 = Suspicious, needs review
- <0.5 = Incorrect, requires fix

---

## Process

### 1. Select Test
From sampling strategy, choose next test to verify

### 2. Generate Output
```bash
# Run the operation in debug mode
./target/release/video-extract debug --ops <operation> <input_file>
```

### 3. AI Verification
```bash
# Verify with Claude
python scripts/ai_verify_outputs.py \
    <input_file> \
    debug_output/stage_XX_<operation>.json \
    <operation>
```

### 4. Document Result
Record in verification report:
- Test name
- Input file
- Operation
- Status (CORRECT/SUSPICIOUS/INCORRECT)
- Confidence score
- Findings
- Any errors

### 5. Investigate Issues
If status is SUSPICIOUS or INCORRECT:
- Examine output manually
- Check code for bugs
- Verify test file is valid
- Document findings

### 6. Fix Bugs
If bugs are found:
- Fix immediately
- Re-verify
- Document in commit

---

## Report Format

**Location:** `docs/ai-verification/NEW_TESTS_AI_VERIFICATION_REPORT.md`

**Template:**
```markdown
# AI Verification Report - New Tests (N=93-109)

**Date:** YYYY-MM-DD
**Tests Verified:** X/275
**Verifier:** Claude Sonnet 4

## Summary

- Total verified: X tests
- CORRECT: Y tests (Z%)
- SUSPICIOUS: A tests (B%)
- INCORRECT: C tests (D%)
- Average confidence: 0.XX

### Confidence Distribution
- ≥0.90: X tests (Y%)
- 0.70-0.89: X tests (Y%)
- 0.50-0.69: X tests (Y%)
- <0.50: X tests (Y%)

## Detailed Results

### Test 1: <test_name>
- **Input:** <file_path>
- **Operation:** <operation>
- **Status:** CORRECT
- **Confidence:** 0.95
- **Findings:** <summary>
- **Errors:** None

[... repeat for each test ...]

## Issues Found

### Bug 1: <description>
- **Tests affected:** <list>
- **Root cause:** <analysis>
- **Fix:** <description>
- **Status:** Fixed in commit <hash>

## Conclusion

<Overall assessment of test quality>
<Recommendations for next steps>
```

---

## Timeline

**N=111:** Create verification script and methodology (this commit)
**N=112:** Verify first 25 tests, document results
**N=113:** Verify next 25 tests, total 50 verified
**N=114:** Fix any bugs found, re-verify affected tests
**N=115:** Verify 50 additional tests, total 100 verified
**N=116:** Final report and recommendations

---

## Known Limitations

1. **API Key Required:** Must set `ANTHROPIC_API_KEY` environment variable
2. **API Costs:** Each verification costs ~$0.01-0.05 depending on image size
3. **Rate Limits:** May need to throttle requests to avoid rate limiting
4. **Subjective Judgments:** Claude's assessment may vary on edge cases
5. **Video Frames:** For video operations, verifies against keyframes only

---

## Alternative Verification Methods

If API access is unavailable:

1. **Manual Inspection:** Human review of sample outputs (slower, but reliable)
2. **Cross-Validation:** Compare outputs with other tools (FFmpeg, OpenCV)
3. **Ground Truth Testing:** Use labeled test datasets with known outputs
4. **Statistical Analysis:** Check distribution of outputs for anomalies

However, Claude API verification is strongly preferred as it provides:
- Scalability (100+ tests in hours vs days)
- Consistency (same evaluation criteria)
- Documentation (detailed findings in JSON)
- Multi-modal understanding (vision + text)

---

## Next Steps

1. Set `ANTHROPIC_API_KEY` environment variable
2. Run Phase 1 verification (50 tests)
3. Document results in verification report
4. Fix any bugs found
5. Run Phase 2 verification (50 additional tests)
6. Achieve ≥90% confidence on ≥95% of tests

---

**End of AI_VERIFICATION_METHODOLOGY.md**
