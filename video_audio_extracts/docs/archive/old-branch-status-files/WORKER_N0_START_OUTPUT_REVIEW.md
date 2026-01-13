# WORKER N=0 - AI Output Review Branch

**Branch:** ai-output-review
**Your Iteration:** N=0 (first worker on this branch)
**Goal:** Review all 363 test outputs and provide proof of correctness

---

## YOUR MISSION

User wants: **"AI review every test output and provide proof of review"**

Goal: **"Most perfect package"** - verify outputs are actually correct, not just structurally valid.

---

## WHAT YOU WILL DO

Review all 363 test outputs in `test_results/latest/outputs/` and verify correctness.

**For each test:**
1. Read the output JSON file
2. Understand what it should contain
3. Verify the output is actually correct
4. Document findings (CORRECT / SUSPICIOUS / INCORRECT)
5. Note any bugs or concerns

**Create proof of review:**
- CSV file with all 363 tests reviewed
- Summary report with quality assessment
- Document any issues found

---

## STEP-BY-STEP INSTRUCTIONS

### Step 1: Understand the Scope

```bash
# See all test outputs
ls test_results/latest/outputs/ | wc -l
# Should show ~63 test output directories

# See example output structure
ls test_results/latest/outputs/smoke_format_mp4_keyframes/
```

### Step 2: Review Tier 1 Operations (Start Here)

**Keyframes tests (~50 tests):**
```bash
find test_results/latest/outputs -name "*keyframes*" -type d | head -10
```

For each keyframes output:
- Read stage_00_keyframes.json
- Verify: frame_number, timestamp, hash, sharpness, thumbnail_paths
- Check: Are keyframes at reasonable intervals?
- Note: hash=0 and sharpness=0.0 are intentional (fast mode)

**Object Detection tests (~20 tests):**
```bash
find test_results/latest/outputs -name "*object*detection*" -type d | head -10
```

For each object detection output:
- Read stage_01_object_detection.json
- Verify: detections array, bbox coordinates, confidence, class_name
- Check: Do detections make sense? (dog in dog image?)
- Note: Empty arrays are valid (no objects in image)

**Face Detection tests (~15 tests):**
- Similar process for face-detection outputs

### Step 3: Create Review CSV

**File:** `docs/output_review_tier1.csv`

```csv
test_name,operation,input_file,status,confidence_score,findings,issues,reviewer
smoke_format_mp4_keyframes,keyframes,test.mp4,CORRECT,0.95,Keyframes at correct intervals,,N=0
smoke_format_jpg_object_detection,object-detection,dog.jpg,CORRECT,0.90,Dog detected with correct bbox,,N=0
...
```

### Step 4: Commit Tier 1 Review

```bash
git add docs/output_review_tier1.csv
git commit -m "# 0: AI Output Review Tier 1 - Keyframes, Object Detection, Face Detection

Reviewed ~100 test outputs for correctness verification.

**Operations Reviewed:**
- Keyframes: X tests reviewed, Y correct, Z suspicious
- Object Detection: X tests reviewed, Y correct, Z suspicious
- Face Detection: X tests reviewed, Y correct, Z suspicious

**Findings:**
- [List any bugs found]
- [List any suspicious outputs]
- [List any concerns]

**Quality Assessment:**
- Keyframes: [X/10] - [Brief assessment]
- Object Detection: [X/10]
- Face Detection: [X/10]

See docs/output_review_tier1.csv for detailed findings.

## Next AI: Continue with Tier 2 (Audio/Transcription/Embeddings)

Review audio operations next: transcription, audio-embeddings, diarization.
"
```

### Step 5: Continue with Tier 2 and Tier 3

**Next iterations (N=1-3):**
- N=1: Review Tier 2 (transcription, audio, embeddings)
- N=2: Review Tier 3 (remaining operations)
- N=3: Create final summary report

---

## OUTPUT FORMAT

### CSV Columns:
- test_name: e.g., "smoke_format_mp4_keyframes"
- operation: e.g., "keyframes"
- input_file: e.g., "test_edge_cases/video.mp4"
- status: CORRECT / SUSPICIOUS / INCORRECT
- confidence_score: 0.0-1.0 (your confidence in correctness)
- findings: Brief description
- issues: Any problems noted
- reviewer: "N=0" (your iteration)

### Final Report Structure:
```markdown
# AI Output Review Report

## Executive Summary
- Tests reviewed: 363
- Correct: X (Y%)
- Suspicious: X (Y%)
- Bugs found: X
- Quality score: X/10

## By Operation
[Detailed findings per operation]

## Issues Found
[Any bugs or concerns]

## Production Readiness
[Go/No-go assessment]
```

---

## CRITICAL NOTES

**This is verification work, not testing:**
- Don't run the binary
- Review existing outputs in test_results/latest/outputs/
- Focus on correctness, not performance

**Be thorough:**
- Actually read the JSON files
- Think about whether outputs make sense
- Note any suspicious patterns
- Don't just mark everything CORRECT

**Provide evidence:**
- Quote specific values from outputs
- Explain why you think it's correct/suspicious
- Note any concerns even if uncertain

---

## SUCCESS CRITERIA

- [ ] All 363 test outputs reviewed
- [ ] CSV file with detailed findings (363 rows)
- [ ] Summary report with quality assessment
- [ ] Any bugs documented
- [ ] Production readiness determination
- [ ] Proof of review (git commits with timestamps)

---

## TIME ESTIMATE

- Tier 1: ~1.5-2 hours (N=0)
- Tier 2: ~1.5-2 hours (N=1)
- Tier 3: ~2-3 hours (N=2)
- Summary: ~30 minutes (N=3)
- **Total: 5-7 hours AI work**

---

## START NOW

Read AI_OUTPUT_REVIEW_REQUIRED.md for complete methodology.

Begin with Tier 1 (keyframes, object-detection, face-detection).
