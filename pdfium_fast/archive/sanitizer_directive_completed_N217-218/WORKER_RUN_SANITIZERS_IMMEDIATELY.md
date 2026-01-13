# WORKER: Run Sanitizers IMMEDIATELY (ASan/TSan Builds Exist!)

**Current:** N=213
**Priority:** CRITICAL
**User directive:** "Run TSAN and ASAN. Make it perfect."

---

## Good News: Builds Already Exist!

**ASan build:** out/ASan/pdfium_cli ✓
**TSan build:** out/TSan/pdfium_cli ✓

**Just need to run tests with them!**

---

## N=214: Run AddressSanitizer Tests (NOW)

```bash
cd ~/pdfium_fast/integration_tests
source venv/bin/activate

# Run smoke tests with ASan
ASAN_OPTIONS=detect_leaks=1:halt_on_error=0:log_path=/tmp/asan.log \
  PDFIUM_CLI=../out/ASan/pdfium_cli \
  pytest -m smoke -v 2>&1 | tee /tmp/asan_smoke_results.txt

# Check for issues
grep -E "ERROR|Sanitizer|leak|heap-buffer-overflow|use-after-free" /tmp/asan_smoke_results.txt
cat /tmp/asan.log.* 2>/dev/null
```

**What to look for:**
- `ERROR: AddressSanitizer: heap-buffer-overflow` - Buffer overrun
- `ERROR: LeakSanitizer: detected memory leaks` - Memory leak
- `ERROR: AddressSanitizer: heap-use-after-free` - Use-after-free

**Commit:**
```
[WORKER0] # 214: ASan Smoke Tests - Memory Bug Detection

Ran smoke tests with AddressSanitizer (existing build).

Results:
- Tests: [X]/98 pass
- Memory errors: [count] (or "none detected")
- Memory leaks: [count] (or "none detected")

[If issues found:]
Issues detected:
1. [Type]: [Location] - [Description]
2. [Type]: [Location] - [Description]

[Document each issue, will fix in N=215+]

Session: [id]
ASan log: /tmp/asan.log.*
```

---

## N=215: Run ThreadSanitizer Tests

```bash
cd ~/pdfium_fast/integration_tests

# Run threading tests with TSan (SLOW, 10x slower)
TSAN_OPTIONS=halt_on_error=0:log_path=/tmp/tsan.log \
  PDFIUM_CLI=../out/TSan/pdfium_cli \
  pytest -m threading -v 2>&1 | tee /tmp/tsan_results.txt

# Check for races
grep -E "WARNING.*ThreadSanitizer|data race|lock-order" /tmp/tsan_results.txt
cat /tmp/tsan.log.* 2>/dev/null
```

**What to look for:**
- `WARNING: ThreadSanitizer: data race` - Race condition
- `WARNING: ThreadSanitizer: lock-order-inversion` - Deadlock potential

**Commit:**
```
[WORKER0] # 215: TSan Threading Tests - Race Detection

Ran threading tests with ThreadSanitizer.

Results:
- Tests: [X]/[total] pass
- Data races: [count] (or "none detected")
- Lock issues: [count] (or "none detected")

[If issues found:]
Threading issues:
1. Data race: [Location] - [Description]
2. [Type]: [Location] - [Description]

[Document, will fix in N=216+]

Session: [id]
TSan log: /tmp/tsan.log.*
```

---

## N=216+: Fix Any Issues Found

**If ASan finds issues:**
- Fix buffer overruns
- Fix memory leaks
- Fix use-after-free
- Re-run ASan smoke tests
- Verify issue gone

**If TSan finds issues:**
- Add mutex protection
- Fix data races
- Re-run TSan tests
- Verify issue gone

**Continue until:** Both sanitizers report ZERO issues

---

## N=217: Full Benchmark with Regular Build

**After all sanitizer issues fixed:**

```bash
cd ~/pdfium_fast/integration_tests

# Full suite with regular (optimized) build
pytest -v --tb=short | tee /tmp/full_suite_final.txt

# Expected: 2,791/2,791 pass (100%)
```

**Commit:**
```
[WORKER0] # 217: Full Benchmark - All Sanitizer Issues Resolved

After fixing all ASan/TSan issues, ran full benchmark.

Results:
- Total: 2,791/2,791 pass (100%)
- No crashes
- No memory bugs (ASan clean)
- No threading bugs (TSan clean)

System: Memory-safe and thread-safe
Ready for production.

Concluding session.
```

**Then STOP.**

---

## What Baseline Regeneration Means

**Simple answer:**

**Baselines = Expected MD5 hashes of rendered output**

When code changes rendering:
- Output MD5 changes
- Tests fail (actual ≠ expected)
- Must regenerate expected values
- Tests pass again

**Example:**
```
Old rendering: Blue sky (MD5: abc123)
Baseline expects: abc123
Test: PASS (abc123 == abc123)

Code change: Bug makes sky red
New rendering: Red sky (MD5: def456)
Baseline still expects: abc123
Test: FAIL (def456 != abc123)

Regenerate baseline: Now expects def456
Test: PASS (def456 == def456)
```

**Risk:** Can hide bugs by updating baselines to expect wrong output!

**Worker regenerated baselines after fixing bugs (N=197-210).**

---

## START NOW

N=214: Run ASan smoke tests (builds already exist!)
N=215: Run TSan threading tests
N=216+: Fix any issues
N=217: Full benchmark

**Make it perfect with sanitizers.**
