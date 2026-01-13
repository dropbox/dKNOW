# N248 Test Failure Analysis
**Date**: 2025-11-22
**Worker**: WORKER0 # 248
**Status**: In Progress - Tests still running

## Context

After regenerating PPM baselines (73/452 files modified), running full test suite revealed unexpected failures in PDFs that were NOT modified during regeneration.

## Baseline Regeneration Summary

- **Script execution**: 20 minutes, 424/452 PDFs succeeded, 28 failed (corrupt)
- **Files modified in git**: 73/452 (16.2%)
- **Files unchanged**: 379/452 (83.8%)
- **Interpretation**: Unchanged files already had correct MD5s for current binary

## Test Failure Pattern (Observed at 12% completion)

### Failures in UNMODIFIED baseline files:
- `fax_ccitt`: NOT in git diff, but test FAILED
- `cc_001` through `cc_005`: NOT in git diff, but tests FAILED
- `cc_008`, `cc_009`, `cc_011`, `cc_012`: NOT in git diff, but tests FAILED
- `cc_014`, `cc_015`, `cc_016`: NOT in git diff, but tests FAILED
- `cc_019`, `cc_020`: NOT in git diff, but tests FAILED
- `bad_page_type`: NOT in git diff, but test FAILED
- `bug_1506`: NOT in git diff, but test FAILED

### Pattern Analysis:
- Failures concentrated in CC category (scanned PDFs)
- Files that were NOT modified by regeneration script are failing
- This suggests either:
  1. Regeneration script wrote WRONG MD5s (but didn't detect change)
  2. Current binary produces non-deterministic output
  3. Test harness is comparing against wrong baseline values

### Tests PASSING:
- All arxiv tests (40/40) - these WERE modified in git
- Most edge_cases tests - mix of modified/unmodified
- Some CC tests (cc_006, cc_007, cc_010, cc_013, cc_017, cc_018)

## Hypothesis

The regeneration script successfully rendered all PDFs and computed MD5s. However:
- The script's comparison logic may have issue with detecting "no change"
- OR the script wrote new MD5s that matched OLD state but not CURRENT rendering
- OR there's non-determinism in rendering (workers=1 should prevent this)

##Investigation Needed

1. **Verify regeneration script logic**: Does it correctly detect when MD5s are unchanged?
2. **Manual render test**: Render one failing PDF (e.g., fax_ccitt) multiple times, check MD5 consistency
3. **Compare baseline vs actual**: For failing test, extract actual MD5 from test output
4. **Check test harness**: Verify it's loading correct baseline file

## Files for Next AI

- `/Users/ayates/pdfium_fast/integration_tests/baselines/upstream/images_ppm/fax_ccitt.json` - Failing test baseline
- `/Users/ayates/pdfium_fast/integration_tests/baselines/upstream/images_ppm/cc_001_931p.json` - Failing test baseline
- Full test output will be available in pytest output (currently running)

## Next Steps

1. Wait for full test suite completion (~30 more minutes)
2. Extract final failure count and specific failure details
3. Investigate root cause of failures in unmodified files
4. Possibly need to re-regenerate ALL baselines with different approach
