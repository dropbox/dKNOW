# MANAGER: Fix Pathological PDF Handling

**Date:** 2025-11-03
**For:** WORKER0 (Next iteration)
**Priority:** HIGH - Correct test counting and skip logic

---

## Problem: bug_451265.pdf Not Properly Skipped

**Current State:**
- PDF manifest says: `image_baseline=True` for bug_451265.pdf
- Reality: PDF hangs upstream pdfium_test (infinite loop)
- Worker skipped it during generation (correct!)
- But tests will FAIL expecting 452 baselines when only 451 exist

**Impact:**
- Infrastructure tests will fail: "Image baseline JSON missing: bug_451265.pdf"
- Test count is wrong: Expects 452, should expect 451

---

## Required Fixes

### 1. Update PDF Manifest

**File:** `integration_tests/master_test_suite/pdf_manifest.csv`
**Line:** 276 (bug_451265.pdf)
**Change:** `image_baseline=True` → `image_baseline=False`

**Reason:** This PDF cannot generate image baselines (hangs upstream)

### 2. Add Skip List to Baseline Generator

**File:** `integration_tests/generate_ppm_baselines.py`

**Add at top:**
```python
# PDFs that cause upstream pdfium_test to hang/crash
SKIP_PDFS = [
    "bug_451265.pdf",  # Hangs in infinite loop (tested 2025-11-03, 6+ min timeout)
]
```

**In generation loop:**
```python
for i, pdf_path in enumerate(pdfs, 1):
    pdf_name = pdf_path.name

    # Skip pathological PDFs
    if pdf_name in SKIP_PDFS:
        print(f"[{i}/{len(pdfs)}] SKIP (pathological): {pdf_name}")
        continue
```

### 3. Update Test Expectations

**File:** `integration_tests/tests/test_000_infrastructure.py`

**Current:** Expects 452 image baselines
**Should be:** Expects 451 image baselines (bug_451265 skipped)

**Fix:**
```python
# Count PDFs that SHOULD have image baselines
pdfs_with_image_baseline = [
    row for row in manifest
    if row['image_baseline'] == 'True'
]
expected_baseline_count = len(pdfs_with_image_baseline)  # Should be 451
```

### 4. Document Skip Rationale

**File:** `CLAUDE.md` or `integration_tests/README.md`

**Add section:**
```markdown
## Pathological PDFs

Some test PDFs intentionally cause failures to test error handling:

- **bug_451265.pdf**: Causes pdfium_test to hang (infinite loop)
  - Skip image baseline generation
  - Skip image rendering tests
  - Expected: 451/452 baselines (this PDF excluded)
```

---

## Verification After Fixes

```bash
# 1. Check manifest updated
grep bug_451265 integration_tests/master_test_suite/pdf_manifest.csv | grep -q "False"

# 2. Count expected baselines
awk -F, 'NR>1 && $16=="True" {count++} END {print count}' integration_tests/master_test_suite/pdf_manifest.csv
# Should output: 451

# 3. Run infrastructure tests
cd integration_tests && pytest -m infrastructure -v
# Should pass (expects 451 baselines)

# 4. Verify skip list works
python3 generate_ppm_baselines.py --pdf bug_451265.pdf
# Should skip with message: "SKIP (pathological): bug_451265.pdf"
```

---

## Why This Matters

**Current situation:**
- Tests expect 452 baselines
- Only 451 exist
- Tests fail: "Image baseline JSON missing: bug_451265.pdf"
- False negative! System is actually correct.

**After fix:**
- Tests expect 451 baselines
- Exactly 451 exist
- Tests pass
- True positive: 100% coverage of VALID PDFs

**100% testing means:**
- 100% of PDFs that CAN be rendered
- NOT 100% of PDFs including broken/pathological ones
- bug_451265.pdf tests error handling, not rendering success

---

## Summary for Worker

**Action required:**
1. Update pdf_manifest.csv: bug_451265.pdf image_baseline=False
2. Add SKIP_PDFS list to generate_ppm_baselines.py
3. Fix test_000_infrastructure.py to count from manifest
4. Run infrastructure tests to verify fix

**Expected result:**
- Infrastructure tests pass (expect 451, find 451)
- 100% coverage claim is accurate (451/451 valid PDFs)
- Pathological PDF properly excluded from baseline validation

---

**References:**
- EDGE_CASE_PDFS_EXPECTED_FAILURES.md: Full documentation
- This file: Fix instructions
