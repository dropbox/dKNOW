# Test Markers Reference

**Status**: v2.0.0 (Git b7943f33, 2025-11-22)
**Total Tests**: 2,791
**WARNING**: Always run `pytest` to get latest results. This document shows historical data only.
**Change**: Eliminated per-PDF image tests (N=254). Image correctness validated in test_005_image_correctness.py.

---

## Quick Reference

| Marker | Tests | Duration | Purpose |
|--------|-------|----------|---------|
| `smoke` | 96 | ~7 min | Quick validation - Run before every commit |
| `corpus` | 964 | ~24 min | Full PDF corpus (all 452 PDFs, 3 tests each) |
| (none) | 2,791 | ~1h 46m | **Complete suite - Run before release** |
| `performance` | 18 | ~9 min | Speedup validation (>= 2.0x for large PDFs) |
| `scaling` | 18 | ~12 min | Worker scaling analysis (1/2/4/8 workers) |

---

## Recommended Usage

```bash
# Before committing code
pytest -m smoke                    # 7 minutes - must pass

# Before creating PR
pytest -m corpus                   # 24 minutes - full PDF coverage

# Before tagging release
pytest                             # 1h 46m - complete validation

# Performance regression check
pytest -m performance              # 9 minutes - speedup requirements

# Scaling analysis
pytest -m scaling                  # 12 minutes - multi-worker efficiency
```

---

## Detailed Marker Descriptions

### Primary Markers (Use These)

**`smoke`** - Quick Smoke Tests (~7 minutes, 96 tests)
- Purpose: Fast validation of core functionality
- Coverage:
  - Basic text extraction and image rendering
  - Workers API (1w, 4w tests)
  - CLI help command
  - Form rendering
  - Smart mode (JPEG fast path)
  - UTF-8 handling (CJK, Arabic, emoji)
- When: Run before every commit
- Pass requirement: 100% must pass

**`corpus` (formerly "extended")** - Full PDF Corpus (~18 minutes, 708 tests)
- Purpose: Comprehensive validation across all PDFs
- Coverage:
  - All 452 benchmark PDFs (text + jsonl extraction)
  - 254 edge case PDFs (no-crash validation)
  - Infrastructure validation
  - Note: Image correctness validated separately in test_005_image_correctness.py (196 tests)
- When: Run before creating PR
- Pass requirement: 100% pass rate (0 skips, 2 xfails for upstream bug)

**(no marker)** - Complete Test Suite (~1h 30m, 2,339 tests)
- Purpose: Full system validation including performance/scaling tests
- Coverage: All tests in the suite (text, jsonl, image correctness, performance, scaling)
- When: Run before tagging release
- Pass requirement: 100% pass rate (0 skips, 0 failures, 2 xfails)

### Secondary Markers (Subset Selection)

**`performance`** - Performance Tests (~9 minutes, 18 tests)
- Validates speedup requirements:
  - Large PDFs (>= 200p): >= 2.0x speedup at 4 workers
  - Medium PDFs (50-199p): >= 1.0x speedup at 4 workers
  - Small PDFs (< 50p): no assertion (log data only)
- Tests both text extraction and image rendering
- Run to verify performance hasn't regressed

**`scaling`** - Scaling Analysis (~12 minutes, 18 tests)
- Tests worker counts: 1, 2, 4, 8
- Measures parallelism efficiency
- Only runs on large PDFs (>= 200 pages)
- Logs data to telemetry for analysis

**`text`** - Text Extraction Only (~1,000 tests)
- All text extraction tests across PDFs
- Validates byte-for-byte correctness vs upstream
- Use to isolate text issues

**`image`** - Image Rendering Only (~1,000 tests)
- All image rendering tests across PDFs
- Validates pixel-perfect correctness (PPM MD5)
- Use to isolate rendering issues

**`jsonl`** - JSONL Metadata Only (~452 tests)
- Character-level metadata extraction
- Validates positions, fonts, bounding boxes
- Requires Rust bridge (extract_text_jsonl)

### Specialized Markers

**`infrastructure`** - Infrastructure Tests (149 tests, < 1 min)
- Binary existence checks
- Baseline file validation
- Manifest integrity checks
- Must pass for any other tests to run

**`edge_cases`** - Edge Case Tests (254 tests, ~30 min)
- Malformed PDFs
- Encrypted PDFs
- Unusual features (XFA forms, attachments, etc.)
- Validates graceful failure (no crashes)

**`determinism`** - Determinism Tests (10 tests, ~3 min)
- Runs same PDF multiple times
- Validates identical output (MD5 match)
- Detects race conditions

**`threading`** - Threading Regression Tests (9 tests, ~2 min)
- Validates thread-safety
- Catches multi-process coordination bugs
- Run after changing worker code

**`smart`** / `smart_pdf`** - Smart Mode Tests (10 tests, ~1 min)
- Validates JPEG fast path (545x speedup)
- Tests scanned PDF detection
- Validates mixed PNG/JPEG output

---

## Deprecated Markers (Removed)

**`extended`** → **Renamed to `corpus`** (clearer purpose)
**`full`** → Use `corpus` or no marker instead
**`batch_bulk`** → Deprecated (always-on smart mode)
**`api`** → Deprecated (unified `--workers N` API)

---

## Test Marker Cleanup (MANAGER, 2025-11-14)

**Changes made:**
- Renamed `extended` → `corpus` (clearer name for "all 452 PDFs")
- Removed 62 deprecated tests (CSV manifest checks, Rust comparisons)
- Consolidated test markers
- Added clear duration/purpose for each marker

**Result**: 2,760 tests, 100% pass rate (0 skips, 0 failures, 0 xfails)

---

## Telemetry Notes

**Grep False Positives:**

When searching telemetry for failures, be careful with test names containing "error" or "failed":

```bash
# WRONG - catches PDF names with "error" in them
grep "failed\|error" telemetry/runs.csv

# CORRECT - check the result column (field 20)
awk -F',' '$20 == "failed" {print}' telemetry/runs.csv
```

**Examples of misleading test names:**
- `parser_rebuildxref_error_notrailer.pdf` - Tests malformed PDF (passes normally)
- `bug_451265.pdf` - Upstream infinite loop (xfailed, shows as "skipped" in telemetry)

**XFail vs Skip in Telemetry:**
- XFailed tests are logged as `result="skipped"` in CSV (pytest behavior)
- Check pytest output for true status (shows "XFAIL" vs "SKIP")
- Only 2 expected xfails: bug_451265 image rendering tests

