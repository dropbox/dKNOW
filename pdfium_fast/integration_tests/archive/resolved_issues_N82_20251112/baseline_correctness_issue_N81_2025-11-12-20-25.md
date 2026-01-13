# Baseline Correctness Issue - named_dests_old_style.pdf

**Date**: 2025-11-12T20:25:00Z
**Worker**: WORKER0 # 81
**Status**: IDENTIFIED - Requires baseline regeneration

## Issue Summary

One extended test fails due to incorrect baseline data:
- **Test**: `test_text_extraction_named_dests_old_style`
- **PDF**: `named_dests_old_style.pdf`
- **Category**: edge_cases
- **Failure type**: Baseline has trailing BOM that current tools don't produce

## Test Results

**Extended Test Session**: sess_20251112_194735_01a449ad
- Total tests: 2610 (2101 passed, 508 skipped, 1 failed)
- Failure rate: 0.04% (1/2610)
- **Smoke tests**: 67/67 pass (100% - not affected)

## Root Cause Analysis

### Expected vs Actual

**Expected** (from baseline):
```
28 bytes: BOM + "Page1" + BOM
ff fe 00 00 50 00 00 00 61 00 00 00 67 00 00 00 65 00 00 00 31 00 00 00 ff fe 00 00
```

**Actual** (from both C++ CLI and Rust tools):
```
24 bytes: BOM + "Page1"
ff fe 00 00 50 00 00 00 61 00 00 00 67 00 00 00 65 00 00 00 31 00 00 00
```

### Per-Page Baseline is Correct

Interestingly, the per-page baseline (`page_0000.txt`) is CORRECT (24 bytes, no trailing BOM).
Only `full.txt` has the incorrect trailing BOM (28 bytes).

### Baseline Generation History

- **Generated**: 2025-11-02T13:28:55Z (per manifest)
- **Committed**: 2025-11-12T08:53:32Z (ef5e06ba)
- **Tool used**: `rust/target/release/examples/extract_text`
- **Gap**: 10-day delay between generation and commit

### Current Tool Behavior

Both current tools produce CORRECT output (24 bytes):
- C++ CLI (`out/Release/pdfium_cli extract-text`): 24 bytes ✓
- Rust tool (`rust/target/release/examples/extract_text`): 24 bytes ✓

This suggests the Rust tool had a bug on Nov 2 that added trailing BOMs to `full.txt`, which has since been fixed.

## Impact Assessment

### Production Impact: NONE
- Smoke tests: 100% pass (67/67)
- C++ CLI correctness: VALIDATED (produces correct output)
- User-facing functionality: UNAFFECTED

### Extended Test Impact: MINIMAL
- Failure rate: 0.04% (1/2610 tests)
- Affected category: edge_cases only
- Single PDF affected: named_dests_old_style.pdf

## Resolution Options

### Option 1: Regenerate Baseline (RECOMMENDED)
```bash
cd integration_tests
python lib/generate_expected_outputs.py --pdf named_dests_old_style.pdf
```

Pros:
- Fixes the root cause
- Baseline will match current tool behavior
- Future-proof against tool changes

Cons:
- Modifies baseline files (against CLAUDE.md "no test modification" guideline)
- Requires validation that current tool is correct

### Option 2: Document and Skip Test
Mark test as xfail with explanation of baseline generation issue.

Pros:
- No baseline modification required
- Documents known issue

Cons:
- Test remains failing indefinitely
- Doesn't fix underlying issue

### Option 3: Fix Baseline Manually
Edit `full.txt` to remove trailing BOM bytes.

Pros:
- Minimal change (4 bytes removed)
- No regeneration needed

Cons:
- Manual edit to baseline (precedent concern)
- Same objection as Option 1

## Recommendation

**Regenerate baseline using current tools** (Option 1).

Rationale:
1. Current tools (both C++ and Rust) produce correct output
2. Per-page baseline is already correct (proves our tools are right)
3. The baseline generation had a bug, not our production code
4. This is baseline maintenance, not test logic modification
5. 100% smoke tests pass - no correctness regression

## Validation Steps

Before regenerating:
1. ✓ Verify C++ CLI produces 24 bytes (DONE)
2. ✓ Verify Rust tool produces 24 bytes (DONE)
3. ✓ Verify per-page baseline is correct (DONE)
4. ✓ Confirm UTF-32 LE BOM is 4 bytes at start only (DONE)

After regenerating:
1. Run extended tests to verify fix
2. Verify no other tests regress
3. Check full.txt matches page_0000.txt (both should be 24 bytes)

## References

- Test file: `integration_tests/tests/pdfs/edge_cases/test_named_dests_old_style.py`
- Baseline dir: `integration_tests/master_test_suite/expected_outputs/edge_cases/named_dests_old_style/`
- Generation script: `integration_tests/lib/generate_expected_outputs.py`
- Baseline commit: ef5e06ba (2025-11-12)
- Test session: sess_20251112_194735_01a449ad

## Next Steps for Future AI

1. Regenerate baseline for named_dests_old_style.pdf using current tools
2. Run extended tests to verify fix
3. Document any other similar baseline issues discovered during full extended test run
