# FIX 28 SKIPS NOW - Focus Only On This

## The 28 Skips

**24 graceful_failure PDFs:**
- Encrypted PDFs
- Malformed PDFs
- Unloadable PDFs

**4 0-page PDFs:**
- bug_451265
- bug_544880
- circular_viewer_ref
- repeat_viewer_ref

## The Fix (ONLY This)

**File to modify**: `lib/generate_test_files.py`

**Find this pattern** (around lines 174-262):
```python
if manifest.get("expected_behavior") == "graceful_failure":
    pytest.skip("...")
```

**Change to**:
```python
if manifest.get("expected_behavior") == "graceful_failure":
    # TEST graceful failure
    result = subprocess.run([tool, operation, pdf, output])
    assert result.returncode != 0, "Should fail gracefully"
    # PASS - proved graceful failure
    return
```

**Also find**:
```python
if manifest.get("pdf_pages") == 0:
    pytest.skip("...")
```

**Change to**:
```python
if manifest.get("pdf_pages") == 0:
    # TEST 0-page handling
    result = subprocess.run([tool, operation, pdf, output])
    assert result.returncode == 0, "Should handle gracefully"
    # PASS - proved 0-page handling
    return
```

**Then:**
```bash
cd integration_tests
python3 lib/generate_test_files.py  # Regenerate all tests
pytest  # Run ALL 2,819 tests
```

**Expected Result:**
```
2819 passed in X minutes
```

**Focus ONLY on eliminating these 28 skips. Nothing else matters until this is done.**
