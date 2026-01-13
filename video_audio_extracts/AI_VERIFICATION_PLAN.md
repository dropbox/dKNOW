# AI Verification Plan - N=156

**Goal:** Verify 50-100 test outputs with GPT-4 Vision to achieve 85%+ confidence

**Date:** 2025-11-09
**Branch:** main
**MANAGER Directive:** MANAGER_FINAL_DIRECTIVE_100_PERCENT.md (Objective 3)

---

## Verification Strategy

### Target: 50 High-Value Tests

**Rationale:** Quality over quantity. Verify the most challenging and meaningful test cases.

**Categories:**

1. **Object Detection (15 tests)**
   - Complex scenes with many objects
   - Small objects
   - Challenging lighting/angles
   - Purpose: Verify ML model accuracy on challenging images

2. **Face Detection (10 tests)**
   - Multiple faces in one frame
   - Various angles and lighting
   - Partial faces/occlusions
   - Purpose: Verify face detection model accuracy

3. **OCR (10 tests)**
   - Text in images
   - Various fonts and sizes
   - Rotated/skewed text
   - Purpose: Verify text extraction accuracy

4. **RAW Format Processing (5 tests)**
   - ARW, CR2, DNG, NEF, RAF
   - Purpose: Verify RAW decoder works correctly

5. **New Format Support (5 tests)**
   - MXF, VOB, ASF, ALAC
   - Purpose: Verify newly added formats work correctly

6. **Action Recognition (5 tests)**
   - Video activity detection
   - Purpose: Verify video ML model accuracy

---

## Test Selection Criteria

**Priority 1: Challenging Real-World Cases**
- Downloaded challenging test images (N=154)
- Images with complex scenes
- Edge cases

**Priority 2: New Features**
- Formats added in N=93-155
- Operations with less verification

**Priority 3: Representative Sample**
- Mix of media types (image, video, audio)
- Mix of operations (detection, extraction, classification)

---

## Verification Process

### Step 1: Run Tests & Generate Outputs
```bash
VIDEO_EXTRACT_THREADS=4 cargo test --release --test smoke_test_comprehensive -- --ignored --test-threads=1
```

### Step 2: Create Test List (CSV)
```csv
input_file,output_json,operation
test_files_objects_challenging/fruits.jpg,debug_output/stage_00_object_detection.json,object-detection
test_files_faces/two_people.jpg,debug_output/stage_00_face_detection.json,face-detection
...
```

### Step 3: Run Batch Verification
```bash
./scripts/batch_verify.sh verification_test_list.csv AI_VERIFICATION_REPORT.md
```

### Step 4: Analyze Results
- Calculate success rate (target: ≥85% CORRECT)
- Identify common failure patterns
- Document bugs found
- Fix critical issues

### Step 5: Final Report
- Summary statistics
- Confidence by operation type
- Bugs found and fixed
- Conclusion: System readiness

---

## Success Criteria

**Minimum Requirements:**
- [ ] 50+ tests verified with GPT-4 Vision
- [ ] ≥85% success rate (CORRECT status)
- [ ] All INCORRECT cases investigated
- [ ] Critical bugs fixed or documented
- [ ] Verification report committed to repo

**Stretch Goals:**
- [ ] 100+ tests verified
- [ ] ≥90% success rate
- [ ] Representative sample across all 33 operations

---

## Expected Timeline

**N=156:** (Current)
- ✅ AI verification tool tested (works)
- ✅ Batch verification script created
- ⏳ Tests running to generate outputs
- 🔜 Create verification test list (50 tests)
- 🔜 Run batch verification
- 🔜 Analyze results and create report
- 🔜 Commit verification report

**Estimated:** 1 commit (this session)

---

## Tools

**AI Verification:**
- `scripts/ai_verify_openai.py` - GPT-4 Vision verification
- `scripts/batch_verify.sh` - Batch processing
- API Key: `OPENAI_API_KEY.txt` (loaded automatically)

**Test Execution:**
- `VIDEO_EXTRACT_THREADS=4` - Thread limiting for stability
- `--test-threads=1` - Sequential test execution
- `cargo test --release` - Release mode for performance

---

## Notes

- **API Rate Limit:** OpenAI free tier = ~3 requests/minute
  - 50 tests × 20s delay = ~17 minutes
  - 100 tests × 20s delay = ~33 minutes

- **Cost Estimate:** GPT-4o vision pricing
  - ~$0.005-0.01 per verification
  - 50 tests = $0.25-0.50
  - 100 tests = $0.50-1.00
  - Well within budget for quality assurance

- **Face Detection Issue:** Face detection on images returns empty results.
  - Tests use "keyframes;face-detection" pipeline (video-based)
  - Image-only face detection may require investigation
  - Not blocking for verification - focus on operations that work

---

## Output Location

- **Test List:** `verification_test_list.csv`
- **Report:** `AI_VERIFICATION_REPORT.md`
- **Commit:** N=156 commit message includes summary
