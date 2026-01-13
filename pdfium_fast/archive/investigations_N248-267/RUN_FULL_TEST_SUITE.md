# WORKER: Run FULL Test Suite (All 2,781 Tests)

**User directive:** "run full test checks not just smoke"

**Current:** You've been running smoke tests only (96 tests)
**Required:** Run COMPLETE test suite (2,781 tests)

---

## Execute Full Test Suite NOW

```bash
cd ~/pdfium_fast/integration_tests
source venv/bin/activate

# Full test suite (all 2,781 tests, ~1.5-2 hours)
time pytest -v --tb=short 2>&1 | tee /tmp/full_test_suite_results.txt

# Summary
tail -100 /tmp/full_test_suite_results.txt
```

---

## What This Tests

**All 2,781 tests:**
- **1,356 PDF tests:** Text + JSONL + image rendering (452 PDFs)
- **254 edge cases:** Malformed, encrypted, corrupted PDFs
- **149 infrastructure:** Baseline validation
- **89 smoke tests:** Quick validation
- **18 performance tests:** Speedup validation
- **18 scaling tests:** Worker scaling (K=1/2/4/8)
- **Plus:** Memory, threading, determinism, presets, etc.

---

## Expected Results

**Best case:** 2,781/2,781 pass (100%)

**Likely case:** Some failures possible
- Image rendering tests: May fail if baselines not regenerated after bug fixes
- Determinism tests: May fail if K=1 vs K>1 still differ
- Platform-specific tests: May vary

---

## Commit Results

```
[WORKER0] # [N]: Full Test Suite Complete

User directive: Ran complete 2,781-test suite.

Results:
- Total: [X]/2,781 pass ([Y]%)
- Smoke: 89/89 pass
- Corpus: [X]/964 pass
- Performance: [X]/18 pass
- Edge cases: [X]/254 pass
- Infrastructure: [X]/149 pass

Duration: [time]
Session: [session_id]

Failures (if any):
[List each failure with test name and reason]

System status: [Production-ready OR issues documented]
```

---

## If Tests Fail

**Document each failure:**
- Test name
- Error message
- Why it failed
- How to fix (or if acceptable)

**Common failure reasons:**
- Baselines outdated (image MD5 mismatch)
- Platform differences (macOS vs Linux)
- Timing variance (performance tests)

**Fix critical failures, document acceptable ones.**

---

## After Full Suite

**If 100% pass:**
- System is production-ready
- No issues
- Conclude session

**If failures:**
- Document each one
- Fix critical issues
- Accept minor issues (document why)
- Conclude session

---

## START NOW

Run: `pytest -v --tb=short`

This will take 1.5-2 hours.

Report complete results.
