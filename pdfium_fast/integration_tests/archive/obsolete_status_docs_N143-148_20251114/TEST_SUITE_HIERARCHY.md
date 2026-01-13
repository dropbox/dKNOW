# Test Suite Hierarchy - DEFINITIVE

## Total Tests: 2,819

### Run ALL Tests (NO MARKER) - COMPLETE VALIDATION
```bash
pytest --tb=short -q
```
**Count**: 2,819 tests
**Includes**: EVERYTHING (smoke + extended + per-PDF + JSONL + all other tests)
**This is THE complete test suite**

### Test Markers (SUBSETS - Not Complete)

**smoke** (67 tests):
```bash
pytest -m smoke
```
- Quick validation (8 minutes)
- Core functionality only
- NOT comprehensive

**extended** (964 tests):
```bash
pytest -m extended
```
- More comprehensive
- Does NOT include per-PDF tests
- Still NOT complete (missing 1,855 tests!)

**full** (1,800 tests):
```bash
pytest -m full
```
- More tests than extended
- Still NOT all tests

## IMPORTANT

**To validate EVERYTHING**, run:
```bash
cd integration_tests
pytest --tb=short -q  # NO -m flag!
```

**This runs ALL 2,819 tests.**
**Anything else is incomplete validation.**

## Current Status

Worker only ran "extended" (964 tests).
Worker has NOT validated complete 2,819 test suite since applying fixes.

**Worker MUST run complete suite (no marker) to prove all 2,819 tests pass with 0 skips.**
