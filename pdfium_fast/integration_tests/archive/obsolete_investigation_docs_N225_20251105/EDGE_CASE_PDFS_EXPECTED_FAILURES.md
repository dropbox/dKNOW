# Edge Case PDFs - Expected Failures

**Purpose:** Document PDFs that intentionally test error handling and edge cases.
**Date:** 2025-11-03
**Status:** bug_451265.pdf causes pdfium_test to hang (infinite loop or performance issue)

---

## PDFs With Expected Failures

### bug_451265.pdf

**Location:** `pdfs/edge_cases/bug_451265.pdf`
**Size:** 1.2 KB
**Pages:** 1 (per PDF header)
**Category:** Edge case / Bug reproduction test

**Behavior:**
- **upstream pdfium_test:** Hangs indefinitely (no output after 40+ seconds)
- **Expected:** Should timeout or fail gracefully
- **Actual:** Infinite processing loop (100% CPU)

**Test Strategy:**
- Skip baseline generation for this PDF
- Mark as "expected_to_hang" or "expected_to_fail"
- Test should verify our tool handles it the SAME way as upstream
- If upstream hangs, our tool hanging is CORRECT behavior (matches upstream)

**Baseline Status:**
- No baseline generated (would timeout)
- Total baselines: 451/452 (this is the 1 missing)

---

## Other Potential Edge Cases

**Encrypted PDFs** (may also fail rendering):
- encrypted_hello_world_r2_bad_okey.pdf
- encrypted_hello_world_r3_bad_okey.pdf
- encrypted.pdf

**Note:** These may have baselines if upstream can process them. Only bug_451265.pdf confirmed to hang.

---

## Test Suite Handling

### Approach 1: Skip List (Recommended)
```python
SKIP_BASELINE_GENERATION = [
    "bug_451265.pdf",  # Causes pdfium_test to hang
]
```

### Approach 2: Timeout
```python
def generate_baseline(pdf, timeout=30):
    try:
        result = subprocess.run(..., timeout=timeout)
    except subprocess.TimeoutExpired:
        # Document as expected timeout
        return {"status": "timeout", "expected": True}
```

### Approach 3: Expected Failures
```python
EXPECTED_FAILURES = {
    "bug_451265.pdf": {
        "reason": "upstream hangs",
        "behavior": "infinite_loop",
        "test": "verify_our_tool_also_hangs"
    }
}
```

---

## Recommendation for Worker

**Don't waste time trying to generate bug_451265.pdf baseline.**

Instead:
1. Document it as expected failure (this file)
2. Update pdf_manifest.csv to mark it as "skip_image_baseline"
3. Update tests to expect 451 baselines, not 452
4. Or add timeout handling to baseline generation script

**Total valid baselines: 451/452 (99.8%)**

---

**Status:** Documented and handled appropriately.
