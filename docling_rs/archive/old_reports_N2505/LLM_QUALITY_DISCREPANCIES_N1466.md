# LLM Quality Test Discrepancies - N=1466

**Date:** 2025-11-19
**Measured By:** AI Worker N=1466
**Previous Report:** LLM_QUALITY_TEST_RESULTS.md (dated 2025-11-18)

## Executive Summary

**Critical Finding:** LLM quality scores in LLM_QUALITY_TEST_RESULTS.md are significantly outdated and inaccurate.

**Verification Method:** Re-ran LLM quality tests on 4 formats with OPENAI_API_KEY

## Measured Discrepancies

| Format | Reported Score | Actual Score | Delta | Status |
|--------|----------------|--------------|-------|--------|
| GPX | 94% | **0%** | -94% | ❌ WRONG |
| RTF | 93% | **78%** | -15% | ❌ WRONG |
| DOCX | 95-100% | **92%** | -3 to -8% | ❌ WRONG |
| HTML | 100% | ✅ (not re-tested, N=1463 verified) | N/A | ✅ CORRECT |

## Format-Specific Findings

### GPX: 94% → 0% (-94 points)

**Root Causes:**
1. ✅ **Fixed in N=1466:** Author metadata was "Gpx11" instead of "Trail Mapper"
2. ❌ **Still Broken:** DocItems are flat markdown text, not semantic GPS structures
   - LLM expects Waypoint DocItems with lat/lon/elevation fields
   - Current: Text DocItems with "Lat: 37.9050, Lon: -122.5962"
   - Need: Waypoint/Track/Route DocItems with typed geographic fields

**Effort to Fix:** Major refactor (5-10 commits to create proper GPS DocItems)

### RTF: 93% → 78% (-15 points)

**Root Cause:** Paragraphs not separated in DocItem text fields
- Issue: "Paragraphs are not separated in the 'orig' and 'text' fields"
- Text is concatenated without newlines between paragraphs
- Document structure not preserved

**Effort to Fix:** Medium (2-4 commits to fix paragraph separation)

### DOCX: 95-100% → 92% (-3 to -8 points)

**Root Causes:**
1. List hierarchy incomplete: `parent: None`, `depth: 0` for all list items
2. N=1464 attempted to fix list references, but issue persists
3. Headings may not be hierarchically structured

**Debug Output:**
```
DEBUG: ListItem #/texts/0, parent: None, depth: 0
DEBUG: ListItem #/texts/1, parent: None, depth: 0
...
```

**Effort to Fix:** Small (1-2 commits to fix list parent/depth fields)

## Impact Assessment

**Documentation Trustworthiness:** LOW
- LLM_QUALITY_TEST_RESULTS.md cannot be trusted for prioritization
- Scores are 0-94 points off from reality
- Unknown when these scores were last verified

**Prioritization Strategy:** INVALID
- Cannot use "formats near 95%" strategy
- Must re-measure all 53 formats to know actual scores
- Cost: ~$0.02 × 53 formats = ~$1.06 for full re-measurement

**Work Completed vs. Remaining:**
- Formats reported as "passing" (95%+) may actually be failing
- Formats reported as "close" (90-94%) may be much farther
- Unknown how many formats genuinely pass the 95% threshold

## Recommendations

### Option A: Full Re-Measurement (Recommended for Accuracy)

**Action:** Run all 53 LLM quality tests with current codebase

**Cost:** ~$1.06 (53 formats × $0.02 each)

**Time:** ~6 minutes (53 tests × ~7 seconds each)

**Benefit:** Accurate baseline for prioritization

**Command:**
```bash
export OPENAI_API_KEY="..."
cargo test --test llm_docitem_validation_tests -- --nocapture > llm_results_n1466.txt 2>&1
```

### Option B: Incremental Verification (Lower Cost)

**Action:** Re-test only "near threshold" formats (10-15 formats)

**Cost:** ~$0.20-$0.30

**Time:** ~2 minutes

**Formats to Test:**
- All formats reported as 90-94% (need verification)
- All formats reported as 95-99% (verify they actually pass)

**Risk:** May still have inaccurate data for lower-scoring formats

### Option C: Continue Without Re-Measurement (Not Recommended)

**Action:** Use docling-backend unit test results as proxy for quality

**Cost:** $0

**Risk:** HIGH
- Backend tests check structure, not semantic correctness
- LLM tests check completeness vs. original document
- Cannot prioritize quality work effectively

## Next Steps for N=1467-1469

**Before N=1470 Benchmark:**

1. **Option A (if budget allows):** Full LLM re-measurement
2. **Option B (if cost-constrained):** Test 10-15 near-threshold formats
3. **Document results** in new file: `LLM_QUALITY_RESULTS_VERIFIED_N1466.md`
4. **Archive old file:** Rename `LLM_QUALITY_TEST_RESULTS.md` to `..._OUTDATED_PRE_N1466.md`

**At N=1470 Benchmark:**
- Run backend tests (as usual)
- Compare backend test results vs. LLM scores (correlation analysis)
- Determine if LLM tests add value or are too expensive to maintain

## Lessons Learned

**LLM Score Stability:** Very low
- Scores change significantly between code revisions
- "94%" can become "0%" after backend refactor
- Need continuous re-measurement, not one-time baseline

**Cost of Quality Metrics:** Higher than expected
- $0.02 per format × 53 formats × multiple runs = $3-5 per full audit
- Backend tests (free) may be sufficient for quality tracking

**Documentation Decay:** Fast
- LLM_QUALITY_TEST_RESULTS.md dated 2025-11-18 (1 day old)
- Already 0-94 points inaccurate
- Commit velocity (1465 commits) causes rapid obsolescence

## Conclusion

**Current State:** LLM quality documentation is unreliable for prioritization.

**Required Action:** Re-measure baseline before continuing quality work.

**Alternative:** Focus on backend test coverage (free, stable) instead of LLM scoring (expensive, volatile).

**Decision Point:** User should decide if LLM quality testing is worth ongoing cost vs. backend unit tests.
