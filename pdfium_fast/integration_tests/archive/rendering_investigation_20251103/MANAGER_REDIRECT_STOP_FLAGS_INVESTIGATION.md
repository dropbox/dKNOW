# 🚨 MANAGER: STOP FLAGS INVESTIGATION - WRONG PATH

**Date:** 2025-11-03 11:05 PST
**For:** WORKER0 (Iteration 8+)
**Priority:** URGENT - Stop current work, change direction

---

## STOP: Flags Investigation Is Dead End

**Worker hypothesis (Iteration #111):** "Rendering flags mismatch between tools"

**Reality:** BOTH tools use `FPDF_ANNOT` (0x01)

**Proof from upstream code (testing/pdfium_test.cc:212-213):**
```c++
int PageRenderFlagsFromOptions(const Options& options) {
  int flags = FPDF_ANNOT;  // Default, all options are false
  // ... adds conditional flags IF options enabled ...
  return flags;
}
```

**All options default to `false`**, so upstream uses `FPDF_ANNOT` (0x01).

**Your Rust code (render_pages.rs:387):**
```rust
FPDF_ANNOT as i32,  // Same value: 0x01
```

**Conclusion:** Flags are IDENTICAL. This is NOT the cause.

---

## REAL PROBLEM AREAS (Where To Look)

### Area 1: Buffer Not Properly Initialized

**Symptom:** 0xFF bytes where should be other values

**Code location:** render_pages.rs:356
```rust
let mut buffer = vec![0u8; buffer_size];  // Zero-initialized
```

**Upstream equivalent:** FPDFBitmap_CreateEx with external buffer

**Check:**
- Is buffer passed correctly to FPDFBitmap_CreateEx?
- Does PDFium write to our buffer?
- Are we reading the RIGHT buffer (line 391)?

### Area 2: Buffer Pointer After FillRect

**Critical check:**
```rust
// Line 376: Fill happens
FPDFBitmap_FillRect(bitmap, 0, 0, width_px, height_px, fill_color);

// Line 391: Get buffer AFTER fill
let buffer = FPDFBitmap_GetBuffer(bitmap) as *const u8;
```

**Question:** Does FPDFBitmap_GetBuffer return the SAME pointer we passed in?
Or does PDFium use internal buffer after FillRect?

**Test:**
```rust
// Before FillRect
let buffer_before = FPDFBitmap_GetBuffer(bitmap);
FPDFBitmap_FillRect(...);
let buffer_after = FPDFBitmap_GetBuffer(bitmap);
// Are they the same?
```

### Area 3: Stride Mismatch

**Your code (line 354):**
```rust
let stride = width_px * 4;  // You calculate stride
```

**Upstream:** PDFium calculates stride internally

**Check:**
```rust
let actual_stride = FPDFBitmap_GetStride(bitmap);  // Line 392
// Does actual_stride == width_px * 4?
// If not, you're reading wrong bytes!
```

---

## CONCRETE DEBUGGING STEPS

### Step 1: Add Debug Logging

**Modify render_page_to_ppm to print:**
```rust
eprintln!("Page {}: width={} height={}", page_index, width_px, height_px);
eprintln!("  Our stride: {}", stride);
eprintln!("  PDFium stride: {}", FPDFBitmap_GetStride(bitmap));
eprintln!("  Buffer before fill: {:?}", buffer.as_ptr());
eprintln!("  Buffer after fill: {:?}", FPDFBitmap_GetBuffer(bitmap));
eprintln!("  Format: {:?}", format);
eprintln!("  Fill color: 0x{:08X}", fill_color);
```

### Step 2: Test With Failing Page

```bash
# Render ONLY page 6 of 0100pages (known to fail)
DYLD_LIBRARY_PATH=out/Optimized-Shared rust/target/release/examples/render_pages \
  integration_tests/pdfs/benchmark/0100pages_7FKQLKX273JBHXAAW5XDRT27JGMIZMCI.pdf \
  /tmp/debug_out 1 300 --ppm 2>&1 | grep "Page 6"
```

**Look for:**
- Stride mismatch?
- Buffer pointer change?
- Format detection (BGRx vs BGRA)?

### Step 3: Use Our Original Buffer, Not GetBuffer

**Current code (WRONG?):**
```rust
let mut buffer = vec![0u8; buffer_size];  // Line 356 - we own this
let bitmap = FPDFBitmap_CreateEx(..., buffer.as_mut_ptr(), ...);  // Line 358 - pass to PDFium
// ... rendering ...
let buffer = FPDFBitmap_GetBuffer(bitmap) as *const u8;  // Line 391 - get from PDFium
```

**Try instead (CORRECT?):**
```rust
let mut buffer = vec![0u8; buffer_size];
let bitmap = FPDFBitmap_CreateEx(..., buffer.as_mut_ptr(), ...);
// ... rendering ...
// DON'T call FPDFBitmap_GetBuffer - use our original buffer!
let buffer_ptr = buffer.as_ptr();  // Use our buffer, not PDFium's

// Then read from buffer_ptr for conversion
```

**Hypothesis:** FPDFBitmap_GetBuffer might return different pointer than we passed in.

### Step 4: Compare Byte-by-Byte With Working Page

```bash
# Page 0 works, page 6 doesn't
# Compare the actual rendering call differences

# Upstream page 6:  cb8dd6f586dd8ca3daefe3c2cee1e31c
# Ours page 6:      a90e177c19b745c6ea7d370e5c6b8b93

# Check if it's a conversion bug or rendering bug
```

---

## SUCCESS CRITERIA (Before Committing)

**DO NOT commit until:**
1. ✅ 0100pages PDF: 100/100 pages match (0% failure)
2. ✅ All 5 failing PDFs: 100% pages match
3. ✅ Test command: `for i in 0 1 2 6 7 32 99; do diff page_$i.ppm upstream_page_$i.ppm || echo FAIL; done` → 0 failures

**Time limit:** If not fixed in 1 more iteration, REVERT all changes and start fresh approach.

---

## IMMEDIATE ACTIONS

1. **Stop investigating flags** (dead end - both use 0x01)
2. **Add debug logging** to render_page_to_ppm
3. **Test step 3** (use original buffer, not GetBuffer)
4. **Verify stride** matches expectations
5. **Test on page 6** specifically (known failure)

---

**References:**
- MANAGER_CRITICAL_RENDERING_BUG_FOUND.md: Detailed test results
- This file: Corrected investigation direction

**Expected completion:** 1 iteration (12 minutes)
