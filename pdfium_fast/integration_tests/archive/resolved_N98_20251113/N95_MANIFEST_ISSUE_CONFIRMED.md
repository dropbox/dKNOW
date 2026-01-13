# N=95: Manifest/PDFium Page Count Mismatch - CONFIRMED

**Date**: 2025-11-12
**Worker**: WORKER0 # 95
**Status**: Issue confirmed, ready for fix in next session

## Issue Summary

**Problem**: 77+ image rendering tests fail because manifests claim 0 pages, but PDFium reports 1 page.

**Root Cause**: Manifests were generated using a different page counting method than PDFium's FPDF_GetPageCount().

## Evidence

### Test Case: 344775293.pdf

**Manifest Claims**:
```json
{
  "pdf_pages": 0,
  "images": {
    "pages": []
  }
}
```

**PDFium Reports**:
```python
FPDF_GetPageCount() = 1
```

**Test Failure**:
```
test_image_rendering_344775293 FAILED
AssertionError: Should not generate images for 0-page PDF, found: [page_0000.ppm]
```

**Text Extraction**:
- Output: 4 bytes (UTF-32 LE BOM: `fffe 0000`)
- Characters: 0 (empty page)
- Result: Works correctly (N=93 fix)

## Analysis

### Hypothesis

The PDFs DO have 1 structural page (in /Pages tree), but the page is degenerate:
- No visible content (empty/blank)
- No text characters
- No images

Manifest generation counted "pages with content" (0), while PDFium counts "structural pages" (1).

### Impact

**Affected PDFs**: 88 PDFs with manifest `pdf_pages: 0`
**Test Failures**: 77+ image rendering tests (difference due to some being unloadable PDFs)
**Smoke Tests**: 0 (no 0-page PDFs in smoke suite, so 67/67 pass)

## Solution (N=94 Recommendation)

### Option A: Trust PDFium, Fix Manifests (RECOMMENDED)

1. Regenerate manifests using FPDF_GetPageCount() as source of truth
2. If PDFium reports 1 page, manifest should say "pdf_pages": 1
3. Generate expected outputs for that 1 page (even if blank)
4. Tests will pass (expect 1 file, get 1 file)

**Pros**:
- PDFium is authoritative source
- Tests match runtime behavior
- Clean, permanent solution
- No special case logic needed

**Cons**:
- Requires regenerating 88 manifests
- Need to generate image baselines for blank pages

### Option B: Trust Manifests, Add Content Detection

1. Check if page has actual content before rendering
2. Complex logic to determine "blank" vs "real" page
3. Manifests stay as-is

**Cons**:
- Fragile heuristics
- Subjective definition of "blank"
- Doesn't fix root issue
- Adds complexity to production code

### Option C: Update Test Template (WORKAROUND)

1. Modify test_image_rendering to accept 0 OR 1 files for manifest page_count==0
2. Quick fix, tests pass immediately

**Cons**:
- Doesn't fix root cause
- Manifests remain incorrect
- Confusing test logic

## Recommendation

**Use Option A**: Regenerate manifests with PDFium as source of truth.

## Next Steps

1. **Verify scope**: Count all PDFs with manifest `pdf_pages: 0`
2. **Test sample**: Verify 5-10 "0-page" PDFs all report 1 page via PDFium
3. **Regenerate manifests**: Use lib/generate_expected_outputs.py with FPDF_GetPageCount()
4. **Generate baselines**: Create image baselines for blank pages (likely all-white PNG/JPG)
5. **Verify tests**: Run full suite, expect 0 failures

## Files

- Evidence: This file (integration_tests/N95_MANIFEST_ISSUE_CONFIRMED.md)
- Test example: integration_tests/tests/pdfs/edge_cases/test_344775293.py
- Manifest example: master_test_suite/expected_outputs/edge_cases/344775293/manifest.json
- N=94 analysis: (in git commit efea75ed message)
