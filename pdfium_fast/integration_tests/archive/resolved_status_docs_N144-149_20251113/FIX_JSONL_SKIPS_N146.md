# Fix JSONL Skips to Achieve Zero Skips - N=146

## MANAGER Directive

**Target**: 2,819 passed, 0 failed, 0 skipped
**Current**: ~2,750 passed, ~60 skipped, 0 failed (estimated from N=144)

## Root Cause Analysis

**Skip Locations** (lib/generate_test_files.py template):
1. Line 157: `pytest.skip(f"PDF expected to fail loading...")` for FPDF_LOAD_FAILED
2. Line 161: `pytest.skip("JSONL not generated for this PDF")` for 0-page PDFs
3. Line 166: `pytest.skip(f"extract_text_jsonl binary not found...")` for missing Rust tool

**PDFs Affected**:
- 24 unloadable PDFs (FPDF_LOAD_FAILED, graceful_failure category)
- 4 0-page PDFs (circular_viewer_ref, repeat_viewer_ref, bug_451265, bug_544880)
- Total: 28 JSONL skips

**Contrast with Text/Image Tests**:
- Text tests (lines 56-81): TEST graceful failure for FPDF_LOAD_FAILED (PASS) ✓
- Image tests (lines 214-257): TEST graceful failure for FPDF_LOAD_FAILED (PASS) ✓
- JSONL tests: Skip instead of testing (SKIP) ✗

## Implementation Plan

### Step 1: Update Test Template (lib/generate_test_files.py)

**Change JSONL skip conditions (lines 156-161) to graceful failure tests:**

```python
# OLD (lines 156-161):
if manifest.get("load_result") == "FPDF_LOAD_FAILED":
    pytest.skip(f"PDF expected to fail loading ({manifest.get('expected_behavior')})")

# Check if JSONL is available for this PDF
if not manifest["jsonl"]["pages"]:
    pytest.skip("JSONL not generated for this PDF")

# NEW:
# Check for error manifest (unloadable PDF)
if manifest.get("load_result") == "FPDF_LOAD_FAILED":
    # JSONL extraction should fail gracefully for unloadable PDFs
    # This tests error handling, not a skip
    env = os.environ.copy()
    env['DYLD_LIBRARY_PATH'] = str(optimized_lib.parent)

    with tempfile.NamedTemporaryFile(suffix='.jsonl', delete=False) as tmp:
        tmp_path = Path(tmp.name)

    try:
        result = subprocess.run(
            [str(extract_jsonl_bin), str(pdf_path), str(tmp_path), '0'],
            capture_output=True,
            env=env,
            timeout=600
        )
        # Validate graceful failure
        assert result.returncode != 0, "Expected non-zero exit code for unloadable PDF"
        stderr_text = result.stderr.decode()
        assert ("Failed to load PDF" in stderr_text or "Failed to read page count" in stderr_text), \
            "Expected load failure message"
    finally:
        if tmp_path.exists():
            tmp_path.unlink()

    # Test passed - validated graceful failure handling
    return

# Check if JSONL is available for this PDF (0-page PDFs)
if not manifest["jsonl"]["pages"]:
    # For 0-page PDFs, JSONL should produce empty output (graceful handling)
    # This tests edge case handling, not a skip
    env = os.environ.copy()
    env['DYLD_LIBRARY_PATH'] = str(optimized_lib.parent)

    with tempfile.NamedTemporaryFile(suffix='.jsonl', delete=False) as tmp:
        tmp_path = Path(tmp.name)

    try:
        result = subprocess.run(
            [str(extract_jsonl_bin), str(pdf_path), str(tmp_path), '0'],
            capture_output=True,
            env=env,
            timeout=600
        )
        # For 0-page PDFs, extraction might fail or produce empty output
        # Either is acceptable graceful behavior
        if result.returncode != 0:
            # Failed gracefully (expected for 0-page)
            pass
        else:
            # Succeeded with empty output (also acceptable)
            assert tmp_path.stat().st_size == 0, "Should produce empty JSONL for 0-page PDF"
    finally:
        if tmp_path.exists():
            tmp_path.unlink()

    # Test passed - validated graceful 0-page handling
    return
```

**Keep Rust tool skip (line 166):**
This skip is acceptable for v1.0.0 minimal build (Rust tools optional).
For full build (v1.X+), this won't skip.

### Step 2: Regenerate All Test Files

```bash
cd integration_tests
python3 lib/generate_test_files.py
```

This will regenerate all 2,819 test files with the new graceful failure logic.

### Step 3: Verify Fix

```bash
# Run complete test suite
pytest --tb=line -q

# Expected result:
# 2,819 passed, 0 failed, 0 skipped (v1.X+ full build)
# OR
# 2,791 passed, 0 failed, 28 skipped (v1.0.0 minimal build - Rust tool missing)
```

## Expected Outcomes

**v1.0.0 Minimal Build (current)**:
- 28 skips remain (Rust tool missing for JSONL)
- All other skips (24 graceful_failure + 4 0-page) converted to PASS
- Result: 2,791 passed, 0 failed, 28 skipped

**v1.X+ Full Build (with Rust tools)**:
- 0 skips (all graceful failures tested)
- Result: 2,819 passed, 0 failed, 0 skipped ✓

## MANAGER Compliance

**Question for MANAGER**: v1.0.0 is minimal build (C++ CLI only, no Rust tools required).
Should we:
1. Accept 28 skips for v1.0.0 (Rust tool optional, JSONL tests skipped gracefully)
2. Require full build for ZERO skips (must have Rust tools)

**Recommendation**: Option 1 (accept 28 skips for v1.0.0, require ZERO for v1.X+)

## Implementation Status

- [ ] Step 1: Update test template
- [ ] Step 2: Regenerate test files
- [ ] Step 3: Run complete test suite
- [ ] Step 4: Verify ZERO skips (or document acceptable skips)
- [ ] Step 5: Commit with test results

## Context Impact

Estimated time: 2-3 AI commits (24-36 minutes)
Context usage: Currently 55K/1M (5.5%), safe for implementation
