# WORKER: TESTING ONLY - User Will Handle Merge

**You are at N=231**
**New directive:** Focus on testing, user will merge

---

## Your 3 Tasks (Testing Only)

### N=232: Run ASan Smoke Tests

```bash
cd ~/pdfium_fast/integration_tests
source venv/bin/activate

# Run with AddressSanitizer (build exists: ../out/ASan/pdfium_cli)
env ASAN_OPTIONS=detect_leaks=1:halt_on_error=0:log_path=/tmp/asan \
  pytest -m smoke -v 2>&1 | tee /tmp/asan_smoke.txt

# Check results
grep -i "ERROR.*Sanitizer\|leak\|buffer-overflow" /tmp/asan_smoke.txt
ls /tmp/asan.* 2>/dev/null

# Report
echo "ASan Results:" >> /tmp/asan_summary.txt
echo "Tests passed: [count from pytest]" >> /tmp/asan_summary.txt
echo "Memory errors: [count or 'none']" >> /tmp/asan_summary.txt
```

**Commit:**
```
[WORKER0] # 232: ASan Smoke Tests - Memory Safety Verified

Ran 96 smoke tests with AddressSanitizer.

Results:
- Tests: [X]/96 pass
- Memory errors: [count or "NONE"]
- Leaks: [count or "NONE"]

[If issues: list each one]

Session: [id]
ASan log: /tmp/asan.*
```

---

### N=233: Run TSan Threading Tests

```bash
cd ~/pdfium_fast

# Build TSan if needed
ls out/TSan/pdfium_cli || ninja -C out/TSan pdfium_cli

# Run threading tests with TSan (SLOW, ~30-60 min)
cd integration_tests
env TSAN_OPTIONS=halt_on_error=0:log_path=/tmp/tsan \
  pytest -m threading -v 2>&1 | tee /tmp/tsan_threading.txt

# Check for races
grep -i "WARNING.*ThreadSanitizer\|data race" /tmp/tsan_threading.txt
```

**Commit:**
```
[WORKER0] # 233: TSan Threading Tests - Race Condition Check

Ran threading tests with ThreadSanitizer.

Results:
- Tests: [X]/[total] pass
- Data races: [count or "NONE"]
- Lock issues: [count or "NONE"]

[If issues: list each one]

Session: [id]
TSan log: /tmp/tsan.*
```

---

### N=234: Full Benchmark Suite

```bash
cd ~/pdfium_fast/integration_tests
source venv/bin/activate

# Full test suite with regular build (~2 hours)
pytest -v --tb=short | tee /tmp/full_benchmark.txt

# Summary
tail -50 /tmp/full_benchmark.txt
```

**Commit:**
```
[WORKER0] # 234: Full Benchmark Complete - All Validation Done

User directive: Ran complete test suite.

Results:
- Total: [X]/2,791 pass ([Y]%)
- ASan: [clean/issues]
- TSan: [clean/issues]
- Smoke: 96/96 pass
- Corpus: [X]/964 pass

Duration: [time]
Session: [id]

System validation complete.
User will handle merge to main.
```

---

### N=235: Session Conclusion

```
[WORKER0] # 235: Session Conclusion - Testing Complete

All testing complete per user directive:
✓ ASan smoke tests
✓ TSan threading tests
✓ Full benchmark suite

Bug fixes completed (N=197-213):
✓ K=1 vs K>1 correctness
✓ Form rendering
✓ Threading races
✓ SIGBUS crashes

Waiting for user to merge feature branch to main.

Context usage: [check]
Total iterations: 235

Concluding session.
```

**THEN STOP.**

---

## You Do NOT Need To

❌ Merge to main (user will do it)
❌ Create PRs (user will do it)
❌ Clean up root directory (user will do it)

---

## You ONLY Need To

✓ Run ASan tests
✓ Run TSan tests
✓ Run full benchmark
✓ Report results
✓ Conclude session

---

## START NOW

Execute N=232 (ASan smoke tests).

Takes ~10-20 minutes with ASan overhead.

Then N=233 (TSan), N=234 (full benchmark), N=235 (conclude).

**No more maintenance loops. Just test, report, conclude.**
