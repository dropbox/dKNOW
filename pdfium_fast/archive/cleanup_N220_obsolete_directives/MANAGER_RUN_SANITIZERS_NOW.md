# MANAGER: Run ASAN and TSAN - Find ALL Bugs

**User directive:** "Run TSAN and ASAN. Make it perfect."
**Priority:** CRITICAL - Find memory and threading bugs

---

## What are Sanitizers?

**AddressSanitizer (ASan):** Finds memory bugs
- Buffer overruns (like N=182 SIGBUS)
- Use-after-free
- Memory leaks
- Stack overflow

**ThreadSanitizer (TSan):** Finds threading bugs
- Data races
- Deadlocks
- Race conditions (like N=209-210)

**These catch bugs that tests might miss.**

---

## TASK 1: Build with AddressSanitizer (N=214)

```bash
cd ~/pdfium_fast

# Configure ASan build
gn gen out/ASan --args='
  is_debug=false
  pdf_enable_v8=false
  pdf_enable_xfa=false
  use_clang_modules=false
  is_asan=true
'

# Build (will take 20-30 minutes)
ninja -C out/ASan pdfium_cli

# Test
cd integration_tests
ASAN_OPTIONS=detect_leaks=1:halt_on_error=0 pytest -m smoke 2>&1 | tee /tmp/asan_results.txt

# Check for any issues
grep -i "ERROR:\|leak\|buffer-overflow" /tmp/asan_results.txt
```

**Commit:**
```
[WORKER0] # 214: ASan Build - Memory Bug Detection

Built with AddressSanitizer and ran smoke tests.

ASan findings:
- Buffer overruns: [count]
- Memory leaks: [count]
- Use-after-free: [count]

[If issues found: document each, fix in next commits]
[If clean: "No memory bugs detected"]

Tests: 96/98 pass under ASan
Session: [id]

Next: Build with TSan for threading bugs.
```

---

## TASK 2: Build with ThreadSanitizer (N=215)

```bash
cd ~/pdfium_fast

# Configure TSan build
gn gen out/TSan --args='
  is_debug=false
  pdf_enable_v8=false
  pdf_enable_xfa=false
  use_clang_modules=false
  is_tsan=true
'

# Build (will take 20-30 minutes)
ninja -C out/TSan pdfium_cli

# Test threading (TSan is SLOW, ~10x slower)
cd integration_tests
pytest -m threading -v 2>&1 | tee /tmp/tsan_results.txt

# Check for data races
grep -i "WARNING.*ThreadSanitizer\|data race" /tmp/tsan_results.txt
```

**Commit:**
```
[WORKER0] # 215: TSan Build - Threading Bug Detection

Built with ThreadSanitizer and ran threading tests.

TSan findings:
- Data races: [count]
- Deadlocks: [count]

[If races found: document location, fix in next commits]
[If clean: "No threading bugs detected"]

Tests: Threading tests pass under TSan
Session: [id]

All sanitizers clean: System is memory-safe and thread-safe.
```

---

## TASK 3: Fix Any Found Issues (N=216+)

**If ASan or TSan find bugs:**

Document EVERY issue:
```
Issue #1: Buffer overrun in write_jpeg
Location: examples/pdfium_cli.cpp:2138
Type: heap-buffer-overflow
Fix: [describe fix]

Issue #2: Data race in cache access
Location: core/fpdfapi/page/cpdf_docpagedata.cpp
Type: data race (read/write conflict)
Fix: Add mutex protection
```

**Fix each issue:**
- One commit per bug fix
- Re-run sanitizer
- Verify issue gone

**Continue until:** ASan and TSan report zero issues

---

## Expected Findings

### ASan (Likely)

**BGR buffer issues:**
- N=182 fixed SIGBUS in JPEG
- But may still have issues in PNG encoder
- Check: All format conversion code

**Possible:** Small leaks in form handling

### TSan (Possible)

**Data races:**
- N=210 expanded mutex, should be clean
- But TSan may find more subtle races

**Cache access:**
- N=316-317 added cache_mutex_
- Should be clean, but verify

---

## Why This is Critical

**Sanitizers find bugs that tests miss:**
- Tests check correctness (output matches expected)
- Sanitizers check safety (no memory corruption, no races)

**You can have passing tests with memory bugs:**
```bash
# Test passes (output correct)
pytest test_xxx.py  # PASS ✓

# But has memory leak
ASan: "Direct leak of 1024 bytes"  # BUG! ✗
```

**Run both:** Tests (correctness) + Sanitizers (safety)

---

## Expected Timeline

**N=214:** Build ASan, run smoke tests (~1 hour)
**N=215:** Build TSan, run threading tests (~2 hours, TSan is slow)
**N=216+:** Fix any issues found
**N=217:** Full benchmark with regular build
**N=218:** Conclude session

**Total: 3-6 hours depending on issues found**

---

## WORKER: START with N=214 (ASan Build)

Build with AddressSanitizer and run smoke tests.

Report ANY memory issues found.

**Make it perfect with sanitizers.**
