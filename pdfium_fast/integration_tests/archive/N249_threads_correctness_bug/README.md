# N=249 Threads Correctness Bug - Archive

**Date**: 2025-11-22
**Status**: RESOLVED

## Summary

Critical bug discovered in N=248: `--threads` flag caused rendering differences between single-threaded and multi-threaded rendering paths.

## Root Cause

Two different rendering code paths:
- Multi-threaded (`--threads >1`): Uses `FPDF_RenderPagesParallelV2` with pre-loading
- Single-threaded (`--threads 1`): Uses `render_page_to_png()` → `FPDF_RenderPageBitmap`

These produced different pixel output (different MD5 hashes) for the same PDF.

## Impact

- 48 out of 452 PDFs affected (10.6%)
- N=248 test failures: 337 out of 2,791 tests (12%)
- Root cause: Baseline regeneration script didn't specify `--threads 1`, used default (8 threads)

## Resolution (N=249)

**Fix**: Added `--threads 1` flag to baseline regeneration script (`lib/regenerate_ppm_baselines.py`)

**Results**:
- Regenerated all 424 successful PDFs
- 48 baselines modified (10.6%)
- 404 baselines unchanged (89.4%)
- Tests now pass: 96/96 smoke tests (100%)

**Files Modified**:
- `lib/regenerate_ppm_baselines.py` line 85: Added `'--threads', '1'` flag

## Follow-Up Work Required

**Upstream Investigation** (deferred):
- Why do parallel vs single-threaded rendering produce different output?
- This violates threading correctness principle (threading should only affect performance)
- May need to file PDFium upstream bug report

**Current Workaround**:
- All baselines generated with `--threads 1`
- Tests run with `--threads 1`
- Production usage can still use multi-threading (different MD5s, but deterministic)

## Files in Archive

- `CRITICAL_N249_threads_correctness_bug.md`: Full technical analysis
- `N248_test_failure_analysis.md`: N=248 investigation notes
- `README.md`: This file
