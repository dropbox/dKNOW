# Image Baseline Regeneration Status - #104

## Current Status (2025-11-03 16:44 UTC)

**REGENERATION IN PROGRESS**: Full 452-PDF regeneration at 300 DPI running in background

- PID: 67742
- Started: 2025-11-03 16:34 (08:34 local)
- Progress: 22/452 PDFs (4.9%) as of 16:44
- Rate: ~22 PDFs per 5 minutes
- Estimated completion: ~103 minutes from start = 10:17 local / 18:17 UTC
- Latest regenerated: 0313pages (at 08:39)

## Problem Identified

**Root Cause**: Only 60 PDFs (from `file_manifest.csv`) were regenerated at 300 DPI in #103. The remaining 392 PDFs still have baselines from #102 which were at 150 DPI.

**Evidence**:
- Test `test_image_rendering_correctness` failed with MD5 mismatches
- Failed PDFs include: arxiv_002, arxiv_003, arxiv_004, arxiv_006, arxiv_007, arxiv_008, arxiv_010
- These PDFs are NOT in `file_manifest.csv` (only 60 PDFs)
- They ARE in `pdf_manifest.csv` (all 452 PDFs)
- Their baselines were last updated in #102 (before DPI fix)

## Solution

Running: `python regenerate_image_baselines.py --all`
- Regenerates ALL 452 PDFs from `pdf_manifest.csv`
- Uses 300 DPI (fixed in #103)
- Expected duration: ~90 minutes
- Started: 2025-11-03 16:34 UTC

## Test Results Before Fix

**Smoke tests**: ✓ 19 passed (only tests 60 PDFs from file_manifest)
**Image correctness tests**: ✗ 10 failed, 287 passed (tests all PDFs, found DPI mismatches)

Sample failures:
```
arxiv_002 Page 0: expected 1edf3d6627ae6e5cfe533033ce02820e, got 90d82e3b0d9db555cf6304be805ab02d
arxiv_003 Page 0: expected 8da4fa9ae8ddeaeb1dd3ce1133bce67a, got 669b53c77ac54fe516ecb2ff1111a875
```

## Files Modified in #104

1. `master_test_suite/file_manifest.csv` - Regenerated with image baseline columns
2. `master_test_suite/pdf_manifest.csv` - Regenerated with image baseline columns
3. `baselines/upstream/images/*.json` - 35 PDFs regenerated (from file_manifest)
4. `telemetry/runs.csv` - Test run logs

## Next Steps

1. Wait for regeneration to complete (~90 min from 16:34)
2. Verify all 452 baselines regenerated at 300 DPI
3. Run image correctness tests: `pytest -m "image and not infrastructure" -v`
4. Expected: 0 failures (100% correctness)
5. Commit all regenerated baselines
6. Run full test suite: `pytest -m full`

## Commands to Resume

```bash
# Check regeneration status
ps aux | grep regenerate

# Check progress (count recently modified files)
find integration_tests/baselines/upstream/images -name "*.json" -newermt "2025-11-03 16:34" | wc -l

# When complete, verify count
ls integration_tests/baselines/upstream/images/*.json | wc -l  # Should be 452

# Run tests
cd integration_tests
pytest -m "image and not infrastructure" -v

# If tests pass, commit
git add baselines/upstream/images/ master_test_suite/*.csv telemetry/runs.csv
git commit -m "[WORKER0] # 104: Image Baseline Full Regeneration - All 452 PDFs at 300 DPI"
```

## Lessons Learned

1. **Scope Verification Critical**: When regenerating baselines, verify WHICH PDFs are being regenerated
2. **Test Coverage Gaps**: Smoke tests only check file_manifest (60 PDFs), full tests check pdf_manifest (452 PDFs)
3. **Manifest Confusion**: Two manifests exist:
   - `file_manifest.csv`: 60 curated PDFs for file_manifest tests
   - `pdf_manifest.csv`: 452 PDFs for comprehensive testing
4. **Always Regenerate All**: Use `--all` flag to regenerate complete baseline set, not just manifest subset
