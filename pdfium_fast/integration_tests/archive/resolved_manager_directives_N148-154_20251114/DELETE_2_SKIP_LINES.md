# DELETE 2 SKIP LINES - That's All That's Left

## The Reality

**904 pytest.skip() calls = 2 lines in generator × 452 test files**

## The 2 Lines to DELETE

**File**: `lib/generate_test_files.py`

**Line 177**: `pytest.skip(f"extract_text_jsonl binary not found...")`
- **DELETE THIS LINE** - Binary EXISTS at rust/target/release/examples/extract_text_jsonl
- We built it!

**Line 343**: `pytest.skip(f"PPM baseline not found...")`
- **DELETE THIS LINE** - Baselines EXIST in baselines/upstream/images_ppm/
- 452 baseline files in repo!

## The Fix (Literally 30 Seconds)

```bash
cd integration_tests

# Delete line 177
sed -i.bak '177d' lib/generate_test_files.py

# Delete line 343 (now 342 after first delete)
sed -i.bak '342d' lib/generate_test_files.py

# Regenerate all tests
python3 lib/generate_test_files.py

# Verify
grep -r "pytest.skip" tests/pdfs/ | wc -l
# Should be 0

# Run tests
pytest
# Should be: 2819 passed, 0 skipped
```

**That's it. Delete 2 lines. Done.**
