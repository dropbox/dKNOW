# V1.4.0 Baseline - Run Full Tests BEFORE Optimization

**WORKER0**: Before ANY v1.4.0 optimization work, establish clean baseline.

**Your N=405 task**: Run full test suite to verify v1.3.0 is stable on v1.4.0 branch

---

## Why This Matters

**Before you start optimizing**:
- Verify: v1.3.0 code works correctly on new branch
- Establish: Clean baseline (2,757/2,757 pass)
- Document: Starting point for v1.4.0

**If tests fail NOW**: Issue from v1.3.0, not v1.4.0 work
**If tests fail AFTER optimization**: Your change broke something

**This gives you a clean starting point.**

---

## N=405: Full Test Suite Baseline

**Run**:
```bash
cd ~/pdfium_fast/integration_tests
pytest -q > v1.4.0_baseline_tests.txt 2>&1

# Wait ~1h 45m

# Check results
tail v1.4.0_baseline_tests.txt
```

**Expected**:
```
2757 passed in 6353.92s
```

**Or acceptable**:
```
2755 passed, 2 xfailed in 6353.92s
```

**Commit**:
```
[WORKER0] # 405: v1.4.0 Baseline - Full Test Suite

Ran complete test suite on feature/v1.4.0-optimizations branch
to establish clean baseline before optimization work.

Result: 2,757 passed, 0 failed
Session: sess_xxx
Duration: XXXXs

v1.3.0 code is stable. Ready for v1.4.0 optimizations.

Next: AGG quality none optimization.
```

---

## After Baseline Passes

**THEN** (N=406+):
- Start AGG quality none
- Then remaining optimizations
- Each with validation

**NOT BEFORE** baseline is established.

---

## This Is Standard Practice

**Every new development phase**:
1. Establish baseline (tests pass)
2. Make changes
3. Validate (tests still pass)
4. Measure (performance improved)

**Without baseline**: Can't tell if new issues are from your changes or pre-existing

---

**N=405: Run full test suite. Document baseline. THEN start optimization.**
