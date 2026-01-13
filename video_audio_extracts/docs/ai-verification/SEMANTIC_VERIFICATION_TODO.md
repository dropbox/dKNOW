# Semantic Verification TODO (When ANTHROPIC_API_KEY Available)

**Created:** N=115 (2025-11-08)
**Status:** BLOCKED - Awaiting ANTHROPIC_API_KEY
**Purpose:** Handoff document for future AI to complete semantic verification

---

## Context

N=115 completed **structural verification** of 50 Phase 1 tests but could not perform **semantic verification** due to missing ANTHROPIC_API_KEY environment variable.

**What was completed (N=115):**
- ✅ Structural verification of 50 tests (execution + output structure)
- ✅ Created execution infrastructure
- ✅ Documented approach and results
- ✅ Confirmed all tests executable without crashes

**What remains (requires API key):**
- ❌ Semantic verification with Claude vision API
- ❌ Confidence scoring for outputs
- ❌ Investigation of suspicious results
- ❌ Bug fixes if semantic verification finds issues
- ❌ Phase 2 verification (50 more tests)
- ❌ Final verification report

---

## Quick Start (When API Key Available)

```bash
# 1. Set API key
export ANTHROPIC_API_KEY="sk-ant-..."

# 2. Verify key is set
if [ -z "$ANTHROPIC_API_KEY" ]; then
    echo "ERROR: Key not set"
else
    echo "✓ Key is set"
fi

# 3. Run Phase 1 semantic verification
bash scripts/run_phase1_verification.sh

# 4. Review report
cat docs/ai-verification/NEW_TESTS_AI_VERIFICATION_REPORT.md

# 5. Investigate suspicious results (if any)
# 6. Fix bugs (if any)
# 7. Run Phase 2 verification (50 more tests)
# 8. Write final verification report
```

---

## Files Created by N=115

All infrastructure is ready:

1. **Methodology:**
   - `docs/ai-verification/AI_VERIFICATION_METHODOLOGY.md` (N=111)
   - `docs/ai-verification/N115_STRUCTURAL_VERIFICATION_APPROACH.md` (N=115)

2. **Sampling Plan:**
   - `docs/ai-verification/PHASE_1_SAMPLING_PLAN.md` (N=112)
   - Lists all 50 tests to verify

3. **Scripts:**
   - `scripts/ai_verify_outputs.py` (N=111) - AI verification script
   - `scripts/run_phase1_verification.sh` (N=112) - Phase 1 automation
   - `scripts/structural_verify_phase1.sh` (N=115) - Structural verification

4. **Reports:**
   - `docs/ai-verification/N115_STRUCTURAL_VERIFICATION_REPORT.md` (N=115)
   - Will be created: `docs/ai-verification/NEW_TESTS_AI_VERIFICATION_REPORT.md`

---

## What Semantic Verification Does

The AI verification script (`scripts/ai_verify_outputs.py`) uses Claude's vision API to:

1. **Look at the input image/video**
2. **Read the output JSON**
3. **Verify semantic correctness:**
   - Are bounding boxes around actual faces?
   - Are object labels correct ("dog" is actually a dog)?
   - Is transcription text accurate?
   - Are emotion/action labels semantically correct?
   - Do embeddings capture semantic meaning?

4. **Produce confidence score:** 0.0-1.0 (1.0 = perfect match)
5. **Flag issues:** CORRECT / SUSPICIOUS / INCORRECT

---

## Phase 1 Test List

50 tests sampled from 275 new tests (N=93-109):

- 10 RAW format tests (ARW, CR2, DNG, NEF, RAF)
- 10 New video format tests (MXF, VOB, ASF)
- 10 Audio advanced operation tests (profanity, enhancement)
- 10 Video advanced operation tests (action, emotion)
- 10 Random sampling from other categories

**Full list:** See `docs/ai-verification/PHASE_1_SAMPLING_PLAN.md`

---

## Success Criteria

### Phase 1 (50 tests)
- [ ] AI-verified all 50 test outputs with Claude
- [ ] Confidence score ≥0.90 on ≥95% of tests (≥48/50)
- [ ] All bugs found are fixed
- [ ] Results documented

### Phase 2 (50 more tests)
- [ ] AI-verified 50 additional test outputs
- [ ] Confidence score ≥0.90 on ≥95% of tests
- [ ] Total: 100 verified tests

### Final Goal
- [ ] ≥100 new test outputs AI-verified
- [ ] Confidence score ≥0.90 on ≥95% of total
- [ ] All bugs fixed
- [ ] Final verification report published
- [ ] Can claim "outputs are real" per user requirement

---

## Timeline Estimate

**When API key is available:**

- **Phase 1 Semantic Verification:** ~2 hours (1 AI commit)
  - Run 50 tests through AI verification
  - Parse and document results
  - Identify SUSPICIOUS/INCORRECT outputs

- **Investigation:** ~1-2 hours (1 AI commit)
  - Manually review suspicious results
  - Determine if bugs exist
  - Document findings

- **Bug Fixes (if needed):** ~2-4 hours (1-2 AI commits)
  - Fix any bugs found
  - Re-verify fixed tests
  - Update results

- **Phase 2 Verification:** ~2 hours (1 AI commit)
  - Run 50 more tests through AI verification
  - Document results

- **Final Report:** ~1 hour (1 AI commit)
  - Aggregate all results
  - Write final verification report
  - Update MANAGER directive status

**Total: ~8-11 hours (~7-9 AI commits)**

---

## What N=115 Discovered (Structural Verification)

N=115 ran structural verification and produced:
- **Report:** `docs/ai-verification/N115_STRUCTURAL_VERIFICATION_REPORT.md`
- **Execution results:** All 50 tests executed successfully (or documented failures)
- **Structural validation:** All outputs passed schema validation
- **Sanity checks:** Basic reasonableness checks performed

**Key findings from N=115:**
- (Will be documented in N115_STRUCTURAL_VERIFICATION_REPORT.md)

**Read this report before starting semantic verification** to understand baseline.

---

## Comparison: Structural vs Semantic

### Structural Verification (N=115 - DONE)
- ✅ Tests execute without crashes
- ✅ JSON schema valid
- ✅ Required fields present
- ✅ Value ranges valid (0-1 confidence, etc.)
- ✅ Bounding boxes within image bounds
- ✅ Non-empty outputs when expected

### Semantic Verification (FUTURE - NEEDS API KEY)
- ❓ Are bounding boxes around actual faces?
- ❓ Are object labels semantically correct?
- ❓ Is transcription text accurate?
- ❓ Are action/emotion labels correct?
- ❓ Do embeddings capture meaning?

**Both are necessary. Structural ≠ Semantic.**

---

## Common Issues to Watch For

Based on alpha verification (N=0 - original 363 tests):

1. **Face detection false positives:**
   - Finding faces in images with no people
   - Bounding boxes on random objects

2. **Object detection label errors:**
   - Mislabeling objects (cat as "dog")
   - Detecting objects that aren't present

3. **Transcription issues:**
   - Gibberish text
   - Low confidence on clear audio
   - Wrong language detection

4. **Embeddings problems:**
   - All zeros or random noise
   - Incorrect dimensions
   - Not capturing semantic similarity

5. **Action/emotion detection:**
   - Labels don't match visual content
   - Low confidence on clear actions
   - Confusing similar emotions

**Document all issues found for investigation.**

---

## How to Investigate Suspicious Results

If AI verification finds SUSPICIOUS or INCORRECT outputs:

1. **Run test manually:**
   ```bash
   ./target/release/video-extract debug --ops <operations> <input_file>
   ```

2. **Examine input file:**
   - Open image/video in viewer
   - Note what you see (faces, objects, text, etc.)

3. **Examine output JSON:**
   ```bash
   cat debug_output/stage_XX_<operation>.json | jq
   ```

4. **Compare manually:**
   - Do bounding boxes match faces you see?
   - Do labels match objects you see?
   - Is transcription accurate?

5. **Check for patterns:**
   - Is this issue specific to one format?
   - Does it affect all tests with this operation?
   - Is it a model accuracy issue or a bug?

6. **Document findings:**
   - What's wrong
   - Why it's wrong
   - Whether it's fixable (bug) or expected (model limitation)

---

## Bug Fix Protocol

If semantic verification finds bugs:

1. **Confirm the bug:**
   - Reproduce manually
   - Understand root cause
   - Check if it affects other tests

2. **Fix the bug:**
   - Make minimal, targeted fix
   - Add test case if needed
   - Document the fix

3. **Re-verify:**
   - Run affected tests again
   - Confirm fix resolves issue
   - Update verification report

4. **Check for regressions:**
   - Run smoke tests (647 tests)
   - Ensure no new failures
   - Document any changes in behavior

---

## Integration with MANAGER Directive

This work fulfills:
- **MANAGER_CRITICAL_DIRECTIVE_AI_VERIFICATION.md**
- Phases outlined in N=110-115 plan

**Current status:**
- Phase 1A: Structural verification ✅ COMPLETE (N=115)
- Phase 1B: Semantic verification ⏳ BLOCKED (awaiting API key)
- Phase 2: Additional 50 tests ⏳ PENDING
- Final Report: ⏳ PENDING

**Once complete:**
- Can claim "outputs are real" per user requirement
- 275 new tests will be fully verified
- Total: 638 tests (363 alpha + 275 new) all AI-verified
- Production readiness milestone achieved

---

## Questions for User (If Stuck)

If API key remains unavailable:

1. **Is API key blocked indefinitely?**
   - If yes: Consider alternative verification approach
   - If no: When will it be available?

2. **Should we proceed with manual verification?**
   - Much slower (many hours)
   - Lower accuracy than AI verification
   - But doesn't require API key

3. **Should we skip verification and proceed with more tests?**
   - MANAGER directive says "STOP adding tests, START verifying"
   - Risk: Unknown number of tests may have incorrect outputs
   - Blocks production deployment

4. **Should we work on something else?**
   - What should be prioritized instead?
   - Explicit approval needed (violates MANAGER directive)

---

## Final Notes

**For Future AI:**

1. Read N=115 structural verification report first
2. Set ANTHROPIC_API_KEY environment variable
3. Run `scripts/run_phase1_verification.sh`
4. Compare structural vs semantic results
5. Investigate any discrepancies
6. Fix bugs if found
7. Complete Phase 2 (50 more tests)
8. Write final verification report
9. Update MANAGER directive status

**Cost estimate:** ~$0.50-2.50 for 50 tests (Phase 1)

**Time estimate:** ~8-11 hours AI work (~7-9 commits)

**Success criteria:** ≥90% confidence on ≥95% of tests

---

**End of SEMANTIC_VERIFICATION_TODO.md**
