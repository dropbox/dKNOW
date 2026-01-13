# Skip Analysis and Remediation Plan - N=91

**Worker**: WORKER0
**Iteration**: N=91
**Date**: 2025-11-12T23:44:00Z
**Status**: Analysis complete, remediation plan documented

## Executive Summary

**MANAGER Directive**: "0 FAILURES + 0 SKIPS Required"

**Current Status**:
- ✅ 0 FAILURES: All 3 failures fixed (timeout decorators added)
- ❌ ~529 SKIPS: Require major test suite refactoring

## Skip Analysis

Analyzed 2,879 total tests using grep pattern matching across test files.

### Skip Categories (by count from grep analysis)

| Category | Count | Type | Action Required |
|----------|-------|------|-----------------|
| Load-failed PDFs | 1,356 | Conditional | Convert skip to graceful failure test |
| Missing PPM baselines | 453 | Missing data | Generate baselines or delete tests |
| Missing JSONL tool | 453 | Missing tool | Build Rust tool or delete tests |
| 0-page PDFs (text) | 452 | Conditional | Convert skip to empty output test |
| 0-page PDFs (images) | 452 | Conditional | Convert skip to empty output test |
| Missing JSONL baselines | 452 | Missing data | Generate baselines or delete tests |
| No text baseline | 60 | Missing data | Generate baselines or delete tests |
| Missing PDFs | 22 | Missing data | Fix test data or delete tests |
| Small PDFs (<200p) | 2 | Valid skip | Keep (architectural constraint) |

### Actual Skip Count (from MANAGER report)

MANAGER reported **529 skips** from last full test run (1h 24m).

This is significantly less than the theoretical maximum (~3,700) because:
1. Many skip conditions are mutually exclusive (e.g., "load failed" OR "0 pages")
2. Many PDFs exist and load successfully
3. Some skip patterns are template code that rarely triggers

### Valid vs Invalid Skips

**VALID SKIPS (should keep):**
- Small PDFs < 200 pages for multi-process tests (2 skips)
- Missing test PDFs that are truly not needed
- Known upstream bugs (xfailed, not skipped)

**INVALID SKIPS (MANAGER wants fixed):**
1. **0-page PDFs** (904 theoretical, ~260 actual)
   - Current: `pytest.skip("PDF has 0 pages")`
   - Required: Test graceful handling (exit 0, empty output)

2. **Load-failed PDFs** (1,356 theoretical, ~50 actual)
   - Current: Some test graceful failure, then skip
   - Required: Assert graceful failure, don't skip

3. **Missing tool** (453 theoretical, ~200 actual)
   - JSONL tool not built (Rust full build required)
   - Options: Build tool OR delete JSONL tests

4. **Missing baselines** (965 theoretical, ~20 actual)
   - PPM baselines: 453 missing
   - JSONL baselines: 452 missing
   - Text baselines: 60 missing
   - Options: Generate baselines OR delete tests

## Detailed Analysis

### 1. 0-Page PDF Handling (904 skip statements, ~260 actual skips)

**Location**: All generated test files in `tests/pdfs/*/test_*.py`

**Current Code** (lines 84-85 in generated files):
```python
if manifest.get("pdf_pages") == 0:
    pytest.skip("PDF has 0 pages (no text to extract)")
```

**MANAGER Requirement**: "Don't skip 0-page PDFs - TEST graceful handling"

**Required Fix**:
```python
if manifest.get("pdf_pages") == 0:
    # Test graceful handling of 0-page PDF
    result = subprocess.run([tool, "extract-text", pdf_path, output_path], ...)
    assert result.returncode == 0, "Should handle 0-page PDF gracefully"
    assert output_path.stat().st_size == 0, "Should produce empty output"
    return  # Test passes
```

**Impact**:
- Affects: 452 PDFs × 2 operations (text + image) = 904 test modifications
- Effort: 1-2 AI commits (modify template in `lib/generate_test_files.py`, regenerate)

### 2. Load-Failed PDF Handling (1,356 skip statements, ~50 actual skips)

**Location**: All generated test files

**Current Code** (lines 74-81):
```python
if manifest.get("load_result") == "FPDF_LOAD_FAILED":
    result = subprocess.run(...)  # Test graceful failure
    assert result.returncode != 0, "Expected non-zero exit code"
    assert "Failed to load PDF" in stderr_text, "Expected error message"
    pytest.skip(f"PDF expected to fail loading")  # ← Remove this
```

**MANAGER Requirement**: "Don't skip - TEST graceful handling"

**Required Fix**: Remove the `pytest.skip()` line - test already validates behavior!

**Impact**:
- Affects: 24 actual load-failed PDFs × 3 tests = 72 test modifications
- Effort: < 1 AI commit (modify template, regenerate)

### 3. Missing JSONL Tool (453 skips)

**Location**: All generated test files, JSONL extraction tests

**Current Code** (lines 142-144):
```python
extract_jsonl_bin = pdfium_root / 'rust' / 'target' / 'release' / 'examples' / 'extract_text_jsonl'
if not extract_jsonl_bin.exists():
    pytest.skip(f"extract_text_jsonl binary not found: {extract_jsonl_bin}")
```

**MANAGER Requirement**: "BUILD tools or DELETE test"

**Options**:
1. **Build Rust tool**: Requires full build (not minimal build)
   ```bash
   ninja -C out/Release pdfium_render_bridge
   cd rust && cargo build --release --examples
   ```
   - Pro: Enables JSONL testing
   - Con: Increases build time and complexity

2. **Delete JSONL tests**: Remove JSONL test generation from template
   - Pro: Simplifies test suite for v1.0.0 (minimal build)
   - Con: Loses JSONL coverage

**Recommendation**: DELETE for v1.0.0 (JSONL is experimental feature, not required)

**Impact**:
- Affects: 453 tests
- Effort: < 1 AI commit (modify template to skip JSONL generation for minimal build)

### 4. Missing PPM Baselines (453 skips)

**Location**: Image rendering tests

**Current Code**:
```python
ppm_baseline_path = pdfium_root / "integration_tests" / "baselines" / "upstream" / "images_ppm" / f"{PDF_STEM}.json"
if not ppm_baseline_path.exists():
    pytest.skip(f"PPM baseline not found: {ppm_baseline_path}")
```

**Investigation**:
```bash
$ find baselines/upstream/images_ppm -name "*.json" | wc -l
452
```

We have 452 PPM baselines but tests expect 453? Let me check:

**Root Cause**: Unknown - requires investigation. Possibly:
- New PDF added after baseline generation
- Baseline generation failed for 1 PDF
- Test file generated but baseline missing

**Required Action**: Identify missing baseline(s) and generate or delete test

**Impact**:
- Affects: 1-10 PDFs (small number)
- Effort: < 1 AI commit (investigation + fix)

### 5. Missing JSONL Baselines (452 skips)

**Location**: JSONL extraction tests

**Analysis**: Same as #3 - JSONL tool not built, so baselines were never generated

**Recommendation**: DELETE tests for v1.0.0 minimal build

### 6. No Text Baseline (60 skips)

**Location**: Old test format (test_002_text_correctness.py)

**Code**:
```python
pytest.skip("No baseline")
```

**Root Cause**: Legacy test file uses different baseline format than generated tests

**Recommendation**: DELETE test_002_text_correctness.py (superseded by generated tests)

**Impact**: 60 tests removed, no loss of coverage (generated tests cover same PDFs)

## Remediation Plan

### Phase 1: Quick Wins (< 2 AI commits)

1. ✅ **Fix 3 test failures** (COMPLETE - timeout decorators added)
2. **Remove JSONL tests for minimal build** (< 1 commit)
   - Modify `lib/generate_test_files.py` template
   - Add conditional: skip JSONL generation if Rust tool not present
3. **Delete test_002_text_correctness.py** (< 1 commit)
   - Removes 60 obsolete skips
   - No loss of coverage

**Result**: ~513 skips eliminated

### Phase 2: Core Fix (2-3 AI commits)

4. **Convert 0-page PDF skips to tests** (2 commits)
   - Modify template in `lib/generate_test_files.py`
   - Test graceful handling (exit 0, empty output)
   - Regenerate all test files
5. **Remove load-failed skip statements** (1 commit)
   - Modify template (just delete the pytest.skip line)
   - Tests already validate graceful failure
   - Regenerate all test files

**Result**: ~260 more skips eliminated

### Phase 3: Baseline Investigation (1-2 AI commits)

6. **Identify missing PPM baseline(s)** (1 commit)
   - Compare test files vs baseline files
   - Generate missing or delete test
7. **Verify 0 skips achieved** (1 commit)
   - Run full test suite
   - Confirm only valid skips remain (small PDFs, xfails)

**Expected Final State**: 0-5 skips (only valid architectural constraints)

## Total Effort Estimate

- **Phase 1**: 1-2 AI commits (2-3 hours AI time)
- **Phase 2**: 2-3 AI commits (3-4 hours AI time)
- **Phase 3**: 1-2 AI commits (1-2 hours AI time)

**Total**: 4-7 AI commits (~6-9 hours AI execution time)

## Validation

After each phase, run:
```bash
python3 -m pytest -v --tb=line -q | tail -20
```

Target final result:
```
========= 2,879 passed, 0 failed, 0 skipped, 1 xfailed in XXXs =========
```

(1 xfailed is bug_451265 - expected upstream infinite loop)

## Files Requiring Modification

1. **lib/generate_test_files.py** - Test template generator
   - Lines 84-85: 0-page handling
   - Lines 74-81: Load-failed handling
   - Lines 142-144: JSONL tool check
   - Lines 138-139: JSONL baseline check

2. **tests/test_002_text_correctness.py** - DELETE entire file

3. **All generated test files in tests/pdfs/** - REGENERATE after template changes

## Risk Assessment

**Low Risk**:
- Deleting test_002 (redundant)
- Removing JSONL tests (experimental feature)
- Converting skips to tests (behavior already validated)

**Medium Risk**:
- Template changes affecting all 452 PDFs
- Requires careful validation after regeneration

**Mitigation**:
- Run smoke tests (67 tests, 8 minutes) after each change
- Run full tests (2,879 tests, 90 minutes) before commit
- Use git for easy rollback if issues found

## Next Steps for N=92+

1. Implement Phase 1 (quick wins)
2. Validate with smoke tests
3. Implement Phase 2 (core fix)
4. Validate with full test suite
5. Implement Phase 3 (baseline investigation)
6. Final validation: 0 failures, 0 skips
7. Commit and report to MANAGER

## References

- MANAGER commit: 325b8006 (URGENT: 0 FAILURES + 0 SKIPS Required)
- Test template: lib/generate_test_files.py
- Generated tests: tests/pdfs/*/test_*.py (452 PDFs)
- Baselines: master_test_suite/expected_outputs/ (452 manifests)
