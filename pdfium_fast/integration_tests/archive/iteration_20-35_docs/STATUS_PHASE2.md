# Phase 2 Status: Expected Output Generation

**Date**: 2025-11-01 22:18 PST
**Branch**: multi-thread-and-optimize
**Commit**: e05d45534

## Current State

### Completed
- ✅ pytest.ini: Added `unknown_pdf` marker (unblocked test execution)
- ✅ lib/generate_expected_outputs.py: Fixed PDF path bug (was looking in wrong directory)
- ✅ Script verified working with --dry-run (finds all 452 PDFs)
- ✅ 1/452 PDFs generated: 0100pages_7FKQLKX273JBHXAAW5XDRT27JGMIZMCI (2.5MB)

### Blocked/Incomplete
- ⏳ 451/452 PDFs still need expected outputs generated
- ⏳ Phase 3 (test file generation) not started
- ⏳ Phase 4 (infrastructure updates) not started
- ⏳ Phase 5 (validation) not started

## Issues Fixed This Session

### Issue 1: Unknown Marker Registration
**Problem**: pytest collection failed with "unknown_pdf marker not found"
**Cause**: 35 tests use `@pytest.mark.unknown_pdf` but marker not registered in pytest.ini
**Fix**: Added marker to pytest.ini:L46
**Status**: RESOLVED

### Issue 2: PDF Path Bug
**Problem**: generate_expected_outputs.py couldn't find PDFs (404 errors for all 452 PDFs)
**Root cause**:
- Script looked in `pdfium_root/pdfs/benchmark/`
- PDFs actually in `pdfium_root/integration_tests/pdfs/benchmark/`
**Fix**: Changed Line 321 from `self.pdfium_root / pdf_row['pdf_path']` to `self.root / 'pdfs' / 'benchmark' / pdf_name`
**Status**: RESOLVED
**Verification**: `--dry-run` now finds all 452 PDFs successfully

## Test Results

**Smoke tests**: 19 passed in 21.89s
- Command: `pytest -m smoke -v`
- Session: sess_20251102_051532_50f7452e
- All tests green

## Next WORKER Tasks

**Priority 1**: Run expected output generation for all 452 PDFs

```bash
cd integration_tests
python lib/generate_expected_outputs.py
```

**Expected**:
- Runtime: 2-3 hours (depends on PDF complexity)
- Output size: ~60MB committed to git
- Generates for each PDF:
  - text/ directory with per-page files + full.txt
  - jsonl/page_0000.jsonl (page 0 only)
  - manifest.json with image metadata (images NOT committed)

**Priority 2**: Implement lib/generate_test_files.py

See IMPLEMENTATION_PLAN.md:L293-L454 for template and requirements.

**Priority 3**: Run validation

```bash
pytest -m smoke_fast  # Must complete < 1 min
pytest -m standard_60_set  # Must pass all 180 tests
```

## Context Budget

Current session: ~6% (67k/1000k tokens used)
Remaining capacity: Good for Phase 2 + Phase 3 implementation

## Files for Next WORKER

**Read First**:
- integration_tests/STATUS_PHASE2.md (this file)
- integration_tests/IMPLEMENTATION_PLAN.md (complete checklist)
- integration_tests/Q_and_A.md (all user decisions)

**Execute**:
- `python lib/generate_expected_outputs.py` (long-running, 2-3 hours)
- Implement lib/generate_test_files.py
- Run validation tests

## Critical Notes

**JSONL Generation**:
- Currently returns placeholder `{"page": 0, "note": "JSONL generation not yet implemented"}`
- FPDFText metadata APIs need to be called from Rust tool
- See CODE_PATH_TEXT_EXTRACTION.md:L77-L103 for API details

**Image Handling**:
- Script generates PNG + JPG, extracts metadata, then DELETES images
- Only MD5 + dimensions saved to manifest.json
- Images regenerated on-demand during testing

**Git Commit Size**:
- Expected outputs: ~60MB (text files + JSONL + manifests)
- Images NOT committed (would be ~20GB)
- .gitignore already configured to exclude images

## Expiration

None - all information current as of this commit.
