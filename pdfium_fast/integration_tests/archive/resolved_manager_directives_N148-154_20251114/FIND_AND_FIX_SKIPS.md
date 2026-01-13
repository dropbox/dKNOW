# FIND AND FIX ALL SKIPS - No Need to Run Tests First

## You're Right - Skips Are Static

User: "Why run all tests to find skips? Skips are statically defined."

**Correct.** Just search the code:

```bash
cd integration_tests/tests/pdfs
grep -r "pytest.skip" . | wc -l
```

This shows ALL skips without running any tests.

## Fix Process

**Step 1: Find all skips in generator**
```bash
grep -n "pytest.skip" lib/generate_test_files.py
```

**Step 2: Replace EVERY pytest.skip() with test**

Example:
```python
# BEFORE:
if condition:
    pytest.skip("reason")

# AFTER:
if condition:
    # TEST the condition
    result = run_extraction(...)
    assert result.returncode == expected
    # PASS
    return
```

**Step 3: Regenerate tests**
```bash
python3 lib/generate_test_files.py
```

**Step 4: Verify (search again)**
```bash
grep -r "pytest.skip" tests/pdfs/ | wc -l
# Should be 0
```

**Step 5: Run to confirm**
```bash
pytest
# Should show: 2819 passed, 0 skipped
```

## Much More Efficient

Don't run tests to find skips.
Just fix the generator, regenerate, verify.
