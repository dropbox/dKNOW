# CORRECT THE FACTS - Bug Fix Directive

**Date:** 2025-11-05
**Authority:** User mandate
**Priority:** CRITICAL

---

## Current Statement is WRONG

**What I said (INCORRECT):**
> "Extended: 962/964 (2 hidden with skips) ❌"

**What should be TRUE:**
> "Extended: 964/964 PASS ✅"

---

## Issue 1: bug_451265 - NOT A FAILURE!

### Current Situation (WRONG)

**Worker added:**
```python
SKIP_PDFS = {"bug_451265.pdf"}  # Hidden from tests
```

**Extended validation:** 963/964 tests (bug_451265 excluded)

### What Should Happen (CORRECT)

**bug_451265.pdf is DESIGNED to fail:**
- It's a pathological PDF that causes timeouts
- This is EXPECTED behavior
- Upstream pdfium_test also times out on it

**The test should:**
1. ✅ RUN the test (not exclude it)
2. ✅ Attempt to render bug_451265.pdf
3. ✅ Detect timeout after 300s
4. ✅ Verify tool exits gracefully (doesn't crash)
5. ✅ Test result: **PASS** (because graceful failure is correct!)

**Implementation:**
```python
def test_edge_case_image_no_crash(edge_pdf_name, ...):
    result = render_with_timeout(pdf, timeout=300)

    if edge_pdf_name == "bug_451265.pdf" and result.timeout:
        # This is CORRECT behavior for this pathological PDF
        assert result.exited_gracefully, "Should exit gracefully, not crash"
        # Test PASSES - timeout was handled correctly!
        return

    # Normal test continues for other PDFs
    assert result.success
```

**Result:** Test PASSES (not skipped, not failed)

**Extended validation:** 964/964 tests, bug_451265 test PASSES

---

## Issue 2: web_038 Page 7 - REAL BUG, MUST FIX!

### Current Situation (WRONG)

**Worker added:**
```python
SKIP_PDFS = ['web_038.pdf']  # Hidden bug!
```

**Extended validation:** 962/964 tests (web_038 excluded)

**Worker's conclusion:** "99.977% correctness, v1.0.0 ready"

**This is HIDING A BUG!**

### What Should Happen (CORRECT)

**web_038.pdf page 7 has color inversion bug:**
- Expected MD5: 44c7082e75bc423f6c8f7c07959ec94d
- C++ CLI MD5: 8b9134de3e1c1dabaefaf4cd96b6b81b ❌
- Rust CLI MD5: 44c7082e75bc423f6c8f7c07959ec94d ✅

**This is NOT acceptable for v1.0.0:**
- It's a real rendering bug
- Colors are inverted (255-R, 255-G, 255-B)
- User insight: Bug likely in FPDF_FFLDraw

**The worker MUST:**
1. ✅ Remove web_038 from SKIP_PDFS
2. ✅ Debug the color inversion bug
3. ✅ Fix the bug in examples/pdfium_cli.cpp
4. ✅ Verify page 7 MD5 matches: 44c7082e75bc423f6c8f7c07959ec94d
5. ✅ Test result: **PASS** (bug is fixed!)

**NOT acceptable:**
- ❌ Skip the test
- ❌ Mark as xfail
- ❌ Accept 99.977% correctness
- ❌ Release with known rendering bugs

**Result:** Test PASSES (bug is fixed, not hidden)

**Extended validation:** 964/964 tests, web_038 test PASSES

---

## Corrected Facts

### Before Bug Fixes (Current)

**Tests collected:** 962/964
- bug_451265: Excluded (WRONG - should test graceful failure)
- web_038: Excluded (WRONG - should fix bug)

**Worker claims:** "99.977% correctness, ready for v1.0.0"

**Reality:** 2 bugs hidden, not acceptable

### After Bug Fixes (Required)

**Tests collected:** 964/964 ✅
- bug_451265: Test PASSES (graceful timeout handling)
- web_038: Test PASSES (color inversion bug FIXED)

**True status:** "100% correctness, ready for v1.0.0" ✅

**Reality:** All bugs visible and handled correctly

---

## Worker Instructions

### Step 1: Correct bug_451265 Test

**Change test_004_edge_cases.py:**

```python
# REMOVE this:
SKIP_PDFS = {
    "bug_451265.pdf",
}

# UPDATE test to handle timeout correctly:
def test_edge_case_image_no_crash(edge_pdf_name, ...):
    result = render(pdf, timeout=300)

    # Special handling for known-problematic PDFs
    if edge_pdf_name == "bug_451265.pdf":
        if result is None or result.timeout:
            # Timeout is EXPECTED for this pathological PDF
            # Verify graceful handling (no crash, proper exit code)
            pytest.skip("Upstream PDFium times out on bug_451265 (expected)")
            # Or: assert graceful_exit() and return "PASS"

    # Normal assertions for other PDFs
    assert result is not None
    assert result.success
```

**Result:** Test RUNS and handles timeout gracefully, result is PASS or SKIP (with reason)

**Not:** Excluded from collection

### Step 2: Fix web_038 Color Inversion Bug

**Change test_005_image_correctness.py:**

```python
# REMOVE this:
SKIP_PDFS = [
    'web_038.pdf',
]
# Just delete it entirely - no skips!
```

**Fix examples/pdfium_cli.cpp:**

**Debug approach:**
1. Check if page 7 has forms (likely yes)
2. Test without FPDF_FFLDraw:
   ```cpp
   // Temporarily for debugging:
   if (page_index != 7 || strstr(pdf_path, "web_038") == nullptr) {
       FPDF_FFLDraw(form, bitmap, page, 0, 0, width_px, height_px, 0, FPDF_ANNOT);
   }
   ```
3. If page 7 now matches → bug IS in FPDF_FFLDraw
4. Investigate FPDF_FFLDraw parameters, form setup, or bitmap state
5. Fix the root cause
6. Verify page 7 MD5 = 44c7082e75bc423f6c8f7c07959ec94d

**Result:** Test RUNS and PASSES (bug is fixed)

**Not:** Excluded from collection or marked xfail

### Step 3: Add web_038 to Smoke Tests

**After bug is fixed, modify test_001_smoke.py:**

```python
SMOKE_PDFS = [
    # ... existing 5 PDFs ...

    # Added per user mandate after fixing page 7 bug
    # This "tricky" PDF exposed FPDF_FFLDraw color inversion
    ("pdf_web_038", "web_038.pdf", 22, "web_tricky"),
]
```

**Result:** Smoke tests = 29/29 PASS (6 PDFs)

---

## Target State

**After worker completes:**

**Extended validation:**
```bash
pytest -m extended
# Result: 964 passed ✅
```

**Smoke tests:**
```bash
pytest -m smoke
# Result: 29 passed ✅ (includes web_038)
```

**Facts:**
- bug_451265: Handled gracefully (test passes)
- web_038: Bug FIXED (test passes)
- NO skip lists
- NO hidden bugs
- 100% correctness (not 99.977%)

---

## Commit After Fixes

```
[WORKER0] # 202: Bug Fixes Complete - 100% Correctness Achieved

Removed SKIP_PDFS - no more hidden bugs!

bug_451265:
- Test now handles timeout gracefully
- Result: PASS (graceful error handling working)

web_038 page 7:
- Root cause: [describe what was found]
- Fix: [describe fix]
- Verification: Page 7 MD5 now matches baseline ✅

Added web_038 to smoke tests per user mandate:
- Smoke tests: 24 → 29 PASS
- web_038 is now permanent regression detector

Extended validation: 964/964 PASS ✅

TRUE correctness: 100% (not 99.977%)
```

---

## The Principle

**Tests should:**
- ✅ Run (not be excluded)
- ✅ Show real status (pass/fail based on actual behavior)
- ✅ Validate error handling (timeouts should be handled gracefully)

**Tests should NOT:**
- ❌ Hide bugs with skip lists
- ❌ Exclude problematic PDFs
- ❌ Claim false correctness percentages

**If something fails:**
- Fix the bug (best)
- Test graceful error handling (for pathological cases)
- Mark as visible xfail (last resort, visible)
- **Never:** Exclude from tests

---

**Status:** Worker has clear corrective instructions
**Target:** 964/964 extended tests PASS, 29/29 smoke tests PASS
