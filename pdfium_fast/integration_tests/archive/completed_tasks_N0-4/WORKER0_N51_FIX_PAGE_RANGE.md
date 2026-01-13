# WORKER0 N=51 - Fix Broken Page Range Feature

**Issue:** Page range flags exist in help but don't actually work
**Evidence:** `--start-page 1 --end-page 5` still renders all 116 pages
**Your task:** Fix the actual implementation

---

## THE BUG

**What you did (N=47):**
- ✅ Added help text
- ✅ Added argument parsing (--start-page, --end-page)
- ❌ NEVER updated the rendering loops

**Result:** Binary shows feature in --help but ignores the flags completely

---

## FIX REQUIRED

**File:** `pdfium/examples/pdfium_cli.cpp`

**Find and fix ALL rendering loops:**

1. **Bulk mode rendering** (around line 1250-1280)
2. **Fast mode worker dispatch** (around line 950-1000)
3. **Smart mode rendering** (uses bulk mode loop)
4. **Worker subprocess** (around line 1470)

**Current code (WRONG):**
```cpp
for (int page_index = 0; page_index < page_count; ++page_index) {
    // render page
}
```

**Fixed code:**
```cpp
for (int page_index = start_page; page_index <= end_page; ++page_index) {
    // render page
}
```

---

## REBUILD & TEST

```bash
cd pdfium
ninja -C out/Profile pdfium_cli
cp out/Profile/pdfium_cli ../out/Optimized-Shared/pdfium_cli

# Manual test
../out/Optimized-Shared/pdfium_cli render-pages \
  ../integration_tests/pdfs/benchmark/cc_008_116p.pdf \
  /tmp/verify --start-page 1 --end-page 5

# Verify: Should create ONLY 5 files
ls /tmp/verify/*.png | wc -l  # Must be 5, not 116!

# Run smoke tests
cd ../integration_tests
pytest -m smoke --tb=line -q  # Should be 67/67 (100%)
```

---

START NOW - Fix the actual implementation
