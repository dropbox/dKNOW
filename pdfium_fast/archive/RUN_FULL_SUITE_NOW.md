# RUN FULL TEST SUITE NOW - Non-Negotiable

**WORKER0**: You are avoiding the full test suite.

**Evidence**:
- N=237: Ran 648 image tests only (`-k "test_image_rendering"`)
- N=226: Ran 964 corpus tests only (`-m corpus`)
- **NOT RUNNING**: Full suite (2,751 tests)

**Missing tests** (1,787 tests not checked!):
- JSONL extraction: 432 tests
- Edge cases: 254 tests
- Performance tests: 18 tests
- Scaling tests: 18 tests
- Infrastructure: 149 tests
- Determinism: 10 tests
- Smart mode: 10 tests
- Threading regression: 9 tests

**Risk**: PNG optimization or bug fixes could have broken JSONL, edge cases, or other categories.

---

## The Command You Must Run

```bash
cd ~/pdfium_fast/integration_tests
pytest -q
```

**NO FILTERS. NO `-k`. NO `-m`. Just `pytest -q`.**

**Expected output**:
```
....................................................
2749 passed, 2 xfailed in 6353.92s
```

**If you see anything else**:
- Failures: DEBUG before continuing
- Skipped: Investigate why
- Less than 2,749: Something is broken

---

## Why This Matters

**Scenario**: You optimized PNG encoding.

**What could break**:
- JSONL extraction (uses different code path)
- Encrypted PDFs (different handling)
- Malformed PDFs (edge cases)
- 0-page PDFs (boundary conditions)

**You WON'T find these issues** with just image tests.

**You WILL find them** with full test suite.

---

## Hard Requirement

**After EVERY code change** that touches:
- Core rendering (cpdf_*)
- Extraction (FPDFText_*)
- CLI (pdfium_cli.cpp)
- Build config (args.gn)

**Run full test suite**:
```bash
pytest -q
```

**NOT optional. Not "after 5 iterations". EVERY change.**

---

## Your N=239 Task

**STOP** whatever you're doing.

**RUN**:
```bash
cd integration_tests
pytest -q > full_suite_N239.txt 2>&1
```

**WAIT** 1h 45m for it to complete.

**CHECK**:
```
tail full_suite_N239.txt
# Must show: 2749 passed, 2 xfailed
```

**REPORT**:
```
git commit -m "[WORKER0] # 239: Full test suite validation

Ran complete test suite after PNG optimization and bug fixes.

Result: 2,749 passed, 2 xfailed, 0 failed, 0 skipped
Session: sess_xxx
Duration: XXXXs (1h XXm)

All optimizations validated. System is correct.

Next: Continue optimization work."
```

**IF ANY TESTS FAIL**: STOP. Debug. Fix. Re-run.

**THEN** continue with optimization work.

---

**This is not a suggestion. This is a requirement.**
