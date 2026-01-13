# N=96: CRITICAL - 133 Manifests Have Wrong Page Counts

**Date**: 2025-11-12
**Worker**: WORKER0 # 96
**Status**: CRITICAL BUG - 133 manifests need regeneration

## Critical Finding

**ALL 10 sample PDFs with manifest "pdf_pages": 0 actually have 1-3 pages in PDFium.**

This is not a "0-page vs 1-page" issue. The manifests are completely wrong.

## Test Results

```python
web_031                       : FPDF_GetPageCount() = 1  # Manifest says 0
web_021                       : FPDF_GetPageCount() = 2  # Manifest says 0
web_042                       : FPDF_GetPageCount() = 2  # Manifest says 0
web_033                       : FPDF_GetPageCount() = 1  # Manifest says 0
web_004                       : FPDF_GetPageCount() = 3  # Manifest says 0
363015187                     : FPDF_GetPageCount() = 1  # Manifest says 0
bug_709793                    : FPDF_GetPageCount() = 1  # Manifest says 0
two_signatures                : FPDF_GetPageCount() = 1  # Manifest says 0
text_form_color               : FPDF_GetPageCount() = 1  # Manifest says 0
version_in_catalog            : FPDF_GetPageCount() = 1  # Manifest says 0
```

**10/10 PDFs tested have wrong page counts in manifests (100% failure rate)**

## Impact

- **133 manifests** have `"pdf_pages": 0`
- **100% of tested manifests are wrong** (extrapolated: all 133 likely wrong)
- **Missing baselines**: Text pages, image pages, JSONL pages for all missing pages
- **Test failures**: Image rendering tests expect 0 files, get 1-3 files per PDF

## Root Cause

The manifest generator (lib/generate_expected_outputs.py) did NOT use FPDF_GetPageCount().

Likely used one of:
- Page iteration that failed silently
- Text extraction page count (wrong for blank pages)
- Manual page count from corrupted metadata
- Wrong API (e.g., FPDFAvail_Is* instead of FPDF_GetPageCount)

## Solution

**REGENERATE ALL 133 MANIFESTS** using FPDF_GetPageCount() as source of truth.

### Steps

1. **Update lib/generate_expected_outputs.py**:
   - Use FPDF_GetPageCount(doc) for page count
   - Verify: Check against 10 sample PDFs above

2. **Regenerate manifests**:
   ```bash
   python lib/generate_expected_outputs.py --regenerate --filter pdf_pages==0
   ```

3. **Generate missing baselines**:
   - Text baselines: Already exist (4 bytes UTF-32 LE BOM for empty pages)
   - Image baselines: Generate PNG/JPG for all pages (including blank)
   - JSONL baselines: Generate for pages with text (skip blank pages)

4. **Regenerate tests**:
   ```bash
   python lib/generate_test_files.py
   ```

5. **Verify**:
   ```bash
   pytest -m smoke  # Expect 67/67 PASS
   pytest -m full   # Expect 0 failures related to 0-page PDFs
   ```

## Files

- Critical finding: integration_tests/N96_CRITICAL_133_MANIFESTS_WRONG.md (this file)
- Generator: lib/generate_expected_outputs.py (needs fix)
- Manifests: master_test_suite/expected_outputs/*/manifest.json (133 files to regenerate)

## Next AI: Fix Manifest Generator and Regenerate

**CRITICAL PRIORITY**: This blocks 133+ tests from passing.

**Action**:
1. Read lib/generate_expected_outputs.py
2. Find page count generation code
3. Replace with FPDF_GetPageCount() call
4. Regenerate 133 manifests
5. Generate missing baselines (text already exists, need images)
6. Run full test suite

**Estimated effort**: 2-3 AI commits (fix generator, regenerate manifests, verify)
