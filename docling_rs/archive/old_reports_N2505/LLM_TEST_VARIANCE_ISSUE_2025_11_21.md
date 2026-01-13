# LLM Test Variance Issue - 2025-11-21 (N=1835)

**Date:** 2025-11-21
**Session:** N=1835
**Issue:** LLM quality tests showing significant variance, making iterative improvements unreliable

---

## Summary

Attempted to improve format quality per BLOCKING_QUALITY_ISSUES.txt directive. Made valid improvements to 3 formats (OBJ, ICS, ZIP), but LLM tests showed:
1. **Score variance**: Same issues fixed, but scores vary or drop between runs
2. **Conflicting feedback**: LLM complaints change between test runs
3. **Regression paradox**: Fixing identified issues can lower overall score

---

## Changes Made (Valid, But Cannot Commit Due to Blocking File)

### 1. OBJ Format (crates/docling-cad/src/obj/serializer.rs)
**Changes:**
- Changed title from `# 3D Model: {name}` to `# 3D Model - {name}`
- Prefer model name from OBJ file (`o Name`) over filename for single-object files
- Falls back to formatted filename if model name empty/unnamed/default

**Results:**
- Structure: 90 → 100 (+10%)
- Formatting: 95 → 100 (+5%)
- Overall: 93% (still below 95% threshold)
- LLM feedback varies between runs

### 2. ICS Format (crates/docling-backend/src/ics.rs)
**Changes:**
- Added `## Calendar Metadata` section header
- Separates calendar properties from event content
- Updated both markdown and DocItems generation

**Results:**
- Structure: 90 → 95 (+5%)
- Overall: 91% → 93% (+2%)
- Still below 95% threshold
- LLM now complains about attendee list formatting (which IS proper markdown)

### 3. ZIP Format (crates/docling-backend/src/archive.rs)
**Changes:**
- Added format indicator: "ZIP Archive" before title
- Changed from `# Archive Contents: {name}` to "ZIP Archive\n\n# {name}"
- Matches pattern used in other formats (ICS, CSV, etc.)

**Results:**
- Metadata: 80 → 100 (+20%) ✅ **Fixed the identified issue**
- **BUT Overall Score DROPPED: 90% → 85%** ❌
- New complaints: Structure (90), Formatting (90)
- Complaints about list format (which uses proper markdown `- item` syntax)

---

## Evidence of LLM Test Variance

### Example 1: ZIP Test Regression
**Test 1 (Before changes):**
- Overall: 90%
- Metadata: 80/100 (complaint: "archive title not explicitly stated")
- Structure: 100/100
- Formatting: 100/100

**Test 2 (After fixing title issue):**
- Overall: 85% ⬇ **DROPPED 5%**
- Metadata: 100/100 ✅ **Issue fixed**
- Structure: 90/100 ❌ **New complaint: "section header doesn't indicate list format"**
- Formatting: 90/100 ❌ **New complaint: "lacks bullet point indentation"**

**Analysis:** Fixed the identified Metadata issue (title), but LLM now complains about Structure and Formatting that were previously scored 100/100. The list format uses proper markdown (`- filename (size)`), so the complaints are questionable.

### Example 2: OBJ Test Variance
**Multiple test runs showed different feedback:**
- Run 1: "Title 'Cube' not as descriptive as 'Simple cube'" (Completeness issue)
- Run 2: "Comment style differs" (Structure issue)
- Run 3: Different complaints each time

**Analysis:** Same code, different LLM feedback each run.

---

## Root Cause Analysis

### Why LLM Tests Are Unreliable for Iterative Development

1. **Non-Deterministic Evaluation**
   - LLM assigns scores based on subjective interpretation
   - Same output gets different scores on different runs
   - Temperature/sampling causes variance

2. **Moving Target**
   - Fixing one issue exposes or creates new complaints
   - LLM finds different things to criticize each time
   - Cannot systematically work toward 95% threshold

3. **Questionable Feedback**
   - ZIP: Complains about missing bullet points when code clearly uses `- ` markers
   - ICS: Complains about attendee list format that IS proper markdown
   - OBJ: Feedback changes between runs

4. **Scoring Paradox**
   - ZIP: Fixed Metadata (+20%), but overall score dropped (-5%)
   - Suggests scoring is not purely additive
   - May have hidden weighting or holistic evaluation

---

## Comparison: Verification Tests vs Mode3 Tests

### Verification Tests (Python vs Rust comparison)
- **Status:** 8/9 passing (88.9%) ✅
- **Reliability:** HIGH - deterministic comparison
- **Method:** Compare Rust output to Python baseline
- **Variance:** Low - same inputs produce same results

### Mode3 Tests (Standalone quality evaluation)
- **Status:** 13/38 passing (34.2%) ❌
- **Reliability:** LOW - non-deterministic evaluation
- **Method:** LLM judges quality standalone
- **Variance:** HIGH - scores vary between runs

**Key Insight from N=1602 Analysis:** "Verification tests perform well (8/9), but Mode3 tests struggle. This suggests Rust is faithful to Python, but BOTH may have quality issues that become apparent in LLM evaluation."

---

## Recommendations

### Option A: Focus on Verification Tests Only
- **Rationale:** Verification tests are reliable and deterministic
- **Goal:** Ensure Rust matches Python output exactly
- **Status:** Already 88.9% passing (8/9 formats)
- **Remaining:** Fix JATS (93%, minor italics issue)

### Option B: Use Deterministic Quality Metrics
- **Method:** `scripts/scan_format_quality.sh` (mentioned in CONTINUOUS_WORK_QUEUE.md)
- **Advantages:** Deterministic, reproducible, specific issues
- **Comparison:** Compare DocItem JSON structure with Python baseline
- **Identify:** Missing DocItem types, incorrect labels, structure issues

### Option C: Multiple LLM Test Runs for Stable Average
- **Method:** Run each format test 3-5 times, average the scores
- **Cost:** ~$0.005 per test × 5 runs × 25 formats = ~$0.625
- **Advantage:** Reduces variance impact
- **Disadvantage:** Still non-deterministic, expensive

### Option D: Remove Blocking File, Allow Incremental Commits
- **Rationale:** Current blocking prevents saving any progress
- **Issue:** LLM test variance makes "fix all 25 formats" impractical in single session
- **Solution:** Allow commits for verified improvements (tests pass, clippy clean)
- **Requirement:** Update BLOCKING_QUALITY_ISSUES.txt approach

---

## Current Code Status

**All changes are valid and improve code quality:**
- ✅ All unit tests pass
- ✅ Zero clippy warnings
- ✅ Code compiles cleanly
- ✅ Improvements are logical (add titles, sections, clarity)

**Cannot commit due to:**
- ❌ BLOCKING_QUALITY_ISSUES.txt exists
- ❌ Pre-commit hook blocks ALL commits while file exists
- ❌ File says "cannot delete until 38/38 formats pass at 95%"
- ❌ LLM variance makes this goal unreliable

---

## Session Metrics

- **Time invested:** ~2 hours AI execution
- **LLM test cost:** ~$0.04 (8 tests run)
- **Formats attempted:** 3 (OBJ, ICS, ZIP)
- **Formats improved:** 3 (all showed category improvements)
- **Formats passing 95%:** 0 (due to variance and threshold strictness)
- **Tests passing:** 100% (3419/3419)
- **Clippy warnings:** 0

---

## Next AI Recommendations

### Immediate Actions

1. **Read this report** - Understand LLM test variance issue
2. **Check if BLOCKING_QUALITY_ISSUES.txt still exists**
3. **Review changes in crates/docling-backend/src/ics.rs, crates/docling-cad/src/obj/serializer.rs, crates/docling-backend/src/archive.rs**
4. **Decide on approach:**
   - Option A: Focus on verification tests (more reliable)
   - Option B: Use deterministic quality scripts
   - Option C: Multiple LLM runs for averaging
   - Option D: Remove blocking file, allow incremental progress

### Strategic Decision Needed

**Question for user/project owner:**
- Should quality improvements be blocked by LLM test variance?
- Is 88.9% verification test pass rate (8/9 formats matching Python) sufficient?
- Should we use deterministic quality metrics instead of LLM evaluation?

### If Continuing with LLM Tests

1. **Run each test 3 times minimum, average scores**
2. **Focus on formats with clear, consistent feedback:**
   - EML (88%): Missing "Subject:" label (clear fix)
   - BMP (85%): File size inaccuracy (deterministic fix)
   - AVIF/HEIF (85/84%): Missing dimensions (clear fix)

3. **Avoid formats with vague/varying feedback:**
   - ZIP: Complaints contradict proper markdown syntax
   - OBJ: Feedback changes between runs
   - ICS: Near 95% but stuck due to subjective preferences

---

## Files Modified (Uncommitted)

1. `crates/docling-cad/src/obj/serializer.rs` - OBJ title format improvements
2. `crates/docling-backend/src/ics.rs` - ICS metadata section separation
3. `crates/docling-backend/src/archive.rs` - ZIP format indicator addition

**All changes are valid improvements. Blocked by pre-commit hook.**

---

## Conclusion

LLM Mode3 quality tests are showing significant variance that makes iterative improvement impractical. The blocking file philosophy (no commits until all 25 formats pass) combined with test variance creates an impossible situation.

**Recommendation:** Switch to deterministic quality metrics (verification tests, DocItem structure comparison) or remove the blocking file to allow incremental progress.

The changes made in this session are valid improvements that enhance output clarity and consistency. They should be committed even though LLM scores haven't reached 95% threshold.
