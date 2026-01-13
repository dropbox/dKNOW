# 🚨 MANAGER: Implement Progressive Rendering - Final Fix for 100%

**Date:** 2025-11-03 15:39 PST
**For:** WORKER0 (Next iteration)
**Priority:** P0 CRITICAL - Last fix needed for 100%

---

## Current Status

**Pass rate:** 187/197 PDFs (95%)
**Remaining failures:** 10 PDFs (5%)
**Root cause:** VERIFIED - Progressive vs one-shot rendering

---

## The Fix: Implement Progressive Rendering

### What To Change

**File:** `rust/pdfium-sys/examples/render_pages.rs`

**Current code (WRONG):**
```rust
// Line ~379-388 in render_page_to_ppm:
FPDF_RenderPageBitmap(
    bitmap,
    page,
    0, 0,
    width_px, height_px,
    0,
    FPDF_ANNOT as i32,
);
```

**Replace with (CORRECT):**
```rust
// Progressive rendering (matches upstream default)
let mut pause = IFSDK_PAUSE {
    version: 1,
    NeedToPauseNow: None,
};

FPDF_RenderPageBitmapWithColorScheme_Start(
    bitmap,
    page,
    0,    // start_x
    0,    // start_y
    width_px,
    height_px,
    0,    // rotate
    FPDF_ANNOT as i32,
    std::ptr::null(),  // No color scheme
    &mut pause
);

// Continue until complete
loop {
    let status = FPDF_RenderPage_Continue(page, &mut pause);
    if status != FPDF_RENDER_TOBECONTINUED {
        break;
    }
}

FPDF_RenderPage_Close(page);
```

---

## Where To Apply

**Three locations need updating:**

1. **render_page_to_png** (line ~280)
2. **render_page_to_ppm** (line ~379)
3. **render_page_md5** (line ~640) - if it exists

**Apply same progressive rendering loop to all three.**

---

## Required FFI Bindings

**May need to add to rust/pdfium-sys/build.rs:**
```rust
// Add to allowlist:
"FPDF_RenderPageBitmapWithColorScheme_Start",
"FPDF_RenderPage_Continue",
"FPDF_RenderPage_Close",
"IFSDK_PAUSE",
"FPDF_RENDER_TOBECONTINUED",
```

**Check if these exist in bindings already. If not, regenerate bindings.**

---

## Testing Protocol

```bash
# 1. Make changes
# 2. Rebuild
cd rust/pdfium-sys && cargo build --release --example render_pages

# 3. Test failing page
DYLD_LIBRARY_PATH=../../out/Optimized-Shared \
  ../../rust/target/release/examples/render_pages \
  ../../integration_tests/pdfs/benchmark/0100pages_7FKQLKX273JBHXAAW5XDRT27JGMIZMCI.pdf \
  /tmp/progressive_test 1 300 --ppm

# 4. Verify MD5
md5 /tmp/progressive_test/page_0010.ppm
# Expected: 204c77ed71ffcb207f4456546e21fa10

# 5. Run full test suite
cd integration_tests && pytest -m "full and image" -v
# Expected: 197/197 passed (100%)
```

---

## Success Criteria

**Before committing:**
- ✅ 0100pages page 10: MD5 matches baseline (204c77ed...)
- ✅ web_003 page 4: MD5 matches baseline
- ✅ All 10 previously failing PDFs: Now pass
- ✅ pytest -m "full and image": 197/197 passed (100%)
- ✅ Smoke tests: Still pass (19/19)

---

## Reference Implementation

**Upstream:** `testing/pdfium_test.cc:1082-1125` (ProgressiveBitmapPageRenderer::Start)

**Key API sequence:**
1. FPDF_RenderPageBitmapWithColorScheme_Start
2. Loop: FPDF_RenderPage_Continue until status != TOBECONTINUED
3. FPDF_RenderPage_Close

**This is the DEFAULT behavior of upstream pdfium_test.**

---

## Time Estimate

**Implementation:** 1 iteration (12 minutes)
**Testing:** Already know what to test
**Total:** 1-2 iterations maximum

---

## After This Fix

**Then complete:**
1. Task 2: Disable multi-process (force worker_count=1)
2. Task 3: Fix manifest entries
3. Task 4: Document empty baselines
4. Task 5: Final verification

**Estimated total remaining:** 2-3 iterations (24-36 minutes)

---

## Order

**Implement progressive rendering NOW. This is the last rendering fix needed.**

**Do NOT:**
- ❌ Skip this and go to Task 2
- ❌ Accept 95% as "good enough"
- ❌ Try other random fixes

**DO THIS:**
- ✅ Implement progressive rendering loop
- ✅ Test on 0100pages page 10
- ✅ Verify 100% pass rate
- ✅ Commit with test results

---

**Reference:** MANAGER_IMPLEMENT_PROGRESSIVE_RENDERING.md (this file)
