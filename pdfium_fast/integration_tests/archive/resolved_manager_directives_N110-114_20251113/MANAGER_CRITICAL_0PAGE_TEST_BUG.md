# [MANAGER] CRITICAL: 266 Tests Skip Instead of Testing 0-Page PDFs

**Target**: WORKER0
**Priority**: BLOCKER
**Impact**: 266/529 skips (50%) are from this single bug

## Problem

**All 452 auto-generated per-PDF tests** (tests/pdfs/*) have this code:

```python
# Line ~84-85 in every test file
if manifest.get("pdf_pages") == 0:
    pytest.skip("PDF has 0 pages (no text to extract)")
```

**This is WRONG.** User verified:
- 0-page PDFs extract successfully (exit code 0)
- Produce valid output (4-byte BOM for empty UTF-32 text)
- This is **graceful edge case handling** that MUST be tested

**Current behavior**: 266 tests skip (1 PDF × 3 tests each = ~88 PDFs with 0 pages)
**Required behavior**: Tests should PASS by verifying graceful handling

## The Fix

**Option A: Fix Test Generator (RECOMMENDED)**

Modify `lib/generate_test_files.py` template to NOT skip 0-page PDFs:

Find template code generating the skip, change to:
```python
if manifest.get("pdf_pages") == 0:
    # 0-page PDF - verify graceful handling (should not crash)
    result = subprocess.run([...extract...], ...)
    assert result.returncode == 0, "Should handle 0-page PDF without crash"
    assert output_file.exists(), "Should create output file"
    # Empty or BOM-only is correct for 0-page PDF
    pytest.skip("0-page PDF handled gracefully - no content to validate")
    return
```

Then regenerate all 452 tests:
```bash
cd integration_tests
python3 lib/generate_test_files.py
```

**Option B: Manual Fix (If generator can't be easily modified)**

Remove skip from all test files:
```bash
cd integration_tests/tests/pdfs
for f in */test_*.py; do
  sed -i.bak '/if manifest.get("pdf_pages") == 0:/,/pytest.skip.*no text to extract/d' "$f"
  sed -i.bak '/if manifest.get("pdf_pages") == 0:/,/pytest.skip.*no images to render/d' "$f"
done
```

Then manually add graceful handling verification.

## Verification

After fix, re-run:
```bash
pytest tests/pdfs/edge_cases/test_344775293.py -v
```

Should show: **3 passed** (not 3 skipped)

Then re-run complete suite:
```bash
pytest --tb=short -q
```

Should show: **~2,600+ passed** (266 fewer skips)

## Expected Result

**Before**: 2,346 passed, 529 skipped
**After**: 2,612+ passed, <263 skipped

This single fix eliminates HALF the skips.
