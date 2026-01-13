# RUN ALL TESTS - ONE COMMAND

## To Run EVERY Test (2,819 total):

```bash
cd ~/pdfium_fast/integration_tests
pytest
```

**That's it. Just `pytest` with NO flags.**

## What This Runs:
- All 2,819 tests
- Smoke tests (67)
- Extended tests (964)
- Per-PDF tests (1,356)
- JSONL tests (432)
- Everything else

## Required Result:
```
====== 2819 passed in X minutes ======
```

**0 failures. 0 skips.**

## Alternative (with output file):
```bash
cd ~/pdfium_fast/integration_tests
pytest > complete_test_results.txt 2>&1
tail -5 complete_test_results.txt
```

## DO NOT USE:
- ❌ `pytest -m smoke` (only 67 tests)
- ❌ `pytest -m extended` (only 964 tests)
- ❌ `pytest -m full` (only 1,800 tests)

## USE:
- ✅ `pytest` (all 2,819 tests)
