# DPI Mismatch - Image Baseline Generation Issue

**Date**: 2025-11-03
**Iteration**: WORKER0 #103
**Status**: FIX IN PROGRESS

## Problem

Image rendering tests failing with MD5 mismatches across all PDFs.

**Example (arxiv_001.pdf page 0)**:
- Baseline MD5: `e632e710cb06dd35fb3f413e576810bb` (150 DPI)
- Test expected: `1e8f15ba1abad9bdb81cd94f1515bcf2` (old manifest)
- Current render: `085a0abc5b597e0fae5057dc69906146` (300 DPI)

## Root Cause

**regenerate_image_baselines.py** (created in #102) generated baselines at **150 DPI**:
```python
args = [str(tool_path), str(pdf_path), tmpdir, str(worker_count), "150", "--md5"]  # WRONG
```

But tests expect **300 DPI** renders (standard for image quality).

## Verification

Rendering arxiv_001.pdf at different DPIs with same binary (00cd20f999bf):
- 150 DPI: `e632e710cb06dd35fb3f413e576810bb` ✓ matches baseline  
- 300 DPI: `085a0abc5b597e0fae5057dc69906146` ✗ mismatches baseline

## Fix Applied

Changed regenerate_image_baselines.py line 47:
```python
args = [str(tool_path), str(pdf_path), tmpdir, str(worker_count), "300", "--md5"]  # CORRECT
```

## Status

**Regeneration started**: 2025-11-03 08:04  
**Estimated time**: ~90 minutes for 452 PDFs  
**Progress**: 23/452 files (5%)

## Files Changed

1. `integration_tests/lib/generate_full_manifest.py` - Added image baseline columns
2. `integration_tests/master_test_suite/pdf_manifest.csv` - Regenerated with baseline paths
3. `integration_tests/regenerate_image_baselines.py` - Fixed DPI 150→300

## Next Steps

1. Wait for baseline regeneration to complete
2. Verify smoke tests pass with new 300 DPI baselines
3. Run full image test suite
4. Commit baseline JSON files if tests pass

## Binary Info

- libpdfium.dylib: `00cd20f999bf60b1f779249dbec8ceaa`
- Built: 2025-11-02 00:51:46
- No code changes since #102
