# [MANAGER] URGENT: Fix All 3 Test Failures - 100% Pass Rate Required

**Target**: WORKER0
**Priority**: CRITICAL
**Status**: Test suite completed, 3 failures MUST be fixed

## Test Results

**Complete Suite Execution** (MANAGER-initiated):
- Command: `pytest --tb=short -q`
- Result: 2,346 passed, **3 FAILED**, 529 skipped, 1 xfailed
- Duration: 1h 24m 29s
- Session: sess_20251112_194735_01a449ad

## CLAUDE.md Requirement

"No Partial Success: Only 100% pass rate is allowed."

**Current: 99.87% pass rate is NOT acceptable.**
**Required: 100% pass rate (0 failures).**

## The 3 Failures

### 1. test_text_extraction_named_dests_old_style (CORRECTNESS BUG)
**File**: `tests/pdfs/edge_cases/test_named_dests_old_style.py:115`
**Error**: `AssertionError: Text mismatch for named_dests_old_style.pdf`
**Details**: Byte-level difference in UTF-32 output

**Action Required**:
1. Extract text from named_dests_old_style.pdf manually
2. Compare actual vs expected byte-by-byte
3. Determine if:
   - Baseline is wrong (regenerate from upstream)
   - Code is wrong (fix extraction logic)
4. Fix and verify

### 2. test_image_scaling_analysis[cc_004_291p_291p_mixed] (TEST BUG)
**File**: `tests/test_013_interface_benchmarks.py:355`
**Error**: Subprocess error in conftest.py:538

**Action Required**:
1. Run test in isolation to reproduce
2. Check conftest.py:538 - likely temp directory cleanup issue
3. Fix the test infrastructure bug
4. Re-run to verify

### 3. test_pdf_type_variation_analysis (TIMEOUT/HUNG PROCESS)
**File**: `tests/test_013_interface_benchmarks.py`
**Likely Cause**: bug_451265.pdf in test corpus causing hang

**Action Required**:
1. Identify which PDF is timing out
2. If bug_451265: Mark test as xfail or exclude that PDF
3. If other PDF: Debug why it hangs
4. Fix and verify

## The 529 Skips - MANY ARE INVALID

**Critical**: Many skips are from obsolete 500-page limit in test_005_image_correctness.py:127

```python
MAX_PAGES_FOR_PPM = 500  # Line 127 - REMOVE THIS
if page_count > MAX_PAGES_FOR_PPM:
    pytest.skip(f"PDF too large...")  # Line 129 - INVALID SKIP
```

**This limit is obsolete** if PPM cleanup system exists. Large PDFs > 500 pages should be tested.

**Action Required**:
1. Check if `pytest.render_with_md5()` cleans up PPM files after each page
2. If yes: Remove MAX_PAGES_FOR_PPM limit entirely (test all PDFs)
3. If no: Add cleanup to render_with_md5(), then remove limit
4. Verify large PDFs (931p, 522p, 594p) now run and pass

**Small File Skips (INVALID)**:
- test_013_interface_benchmarks.py:181 - Skips PDFs < 200 pages
- test_013_interface_benchmarks.py:347 - Skips PDFs < 200 pages
- Reason: "multi-process overhead dominates"
- **WRONG**: If test doesn't work for small files, test design is flawed
- **Fix**: Either make test work for all sizes OR delete the test entirely
- Do NOT skip tests based on file size

**ZERO SKIPS POLICY**:

User requirement: "0 SKIPS! If we have skips, either FIX or DELETE that test!"

For EVERY skip:
1. **0-page PDFs**: Don't skip - TEST graceful handling!
   - Test should verify: Returns empty text, 0 images, doesn't crash
   - Change test to PASS on graceful handling, not SKIP

2. **Missing Rust tools**: Build the tools OR delete the test
   - parallel_render is optional reference - DELETE tests if not needed
   - Don't skip because tool missing

3. **JSONL not generated**: Generate ALL JSONLs OR delete test
   - No "we haven't generated this yet" excuses

4. **PDF expected to fail**: Change from SKIP to xfail + verify it actually fails
   - Test the failure mode, don't skip testing

**NO FILE SIZE-BASED SKIPS ALLOWED**:
- Remove "too large" skips (add cleanup)
- Remove "too small" skips (fix test design or delete test)

**CRITICAL**:
1. Add PPM cleanup check to test BEFORE removing 500-page limit
2. Remove ALL invalid skips (file size-based, missing tools that should exist)
3. Only valid skips: Truly broken PDFs (0 pages, corrupt), expected xfails

**Goal**: 0 failures, < 50 valid skips (not 529)

## Success Criteria

Before claiming production-ready:
- ✅ 0 failures (fix all 3)
- ✅ Skips reviewed and justified
- ✅ Re-run complete suite: 100% pass rate

## Next Steps

WORKER0: Your N+1 iteration must:
1. Fix named_dests_old_style text mismatch
2. Fix test_013 benchmark issues
3. Re-run full suite
4. Achieve 0 failures

**Do not proceed until 100% pass rate achieved.**
