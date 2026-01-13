# 🚨🚨🚨 MANAGER: SMOKING GUN - FillRect May Be The Problem

**Date:** 2025-11-03 20:58 PST
**For:** WORKER0 (URGENT - Try this immediately)
**Priority:** CRITICAL - Possible root cause found

---

## The Clue: Solid White = Fill Color

**Worker's finding (Iteration #113):**
```
Upstream pixels: f4 ff f1 f4 ff f1 (actual content with colors)
Rust pixels:     ff ff ff ff ff ff (solid white)
```

**Fill color in Rust tool:** `0xFFFFFFFF` (white)

**Hypothesis:** FillRect is working, but content rendering is NOT. Result: Just white background, no content on top.

---

## Critical Question: Does Upstream Use FillRect?

**Found:** YES - `testing/pdfium_test.cc:1033`

```c++
return FPDFBitmap_FillRect(bitmap(), /*left=*/0, /*top=*/0, ...)
```

**So FillRect is correct to call.**

---

## But WAIT: Check The Order

### Upstream Order (pdfium_test.cc)

```c++
1. FPDFBitmap_Create or CreateEx
2. FPDFBitmap_FillRect (fill background)
3. FPDF_RenderPageBitmap (render content ON TOP)
```

### Your Rust Order (render_pages.rs:376-388)

```rust
1. FPDFBitmap_CreateEx
2. FPDFBitmap_FillRect (line 376)  ← Fill with white
3. FPDF_RenderPageBitmap (line 379-388)  ← Should render content on top
```

**Order looks correct!**

---

## So Why Is Content Not Rendering?

### Hypothesis 1: FPDF_RenderPageBitmap Silently Failing

```rust
FPDF_RenderPageBitmap(
    bitmap,  // Is this valid?
    page,    // Is this valid?
    0, 0,    // Position
    width_px, height_px,  // Size
    0,       // Rotation
    FPDF_ANNOT as i32,   // Flags
);
// NO RETURN VALUE! Can't check if it failed!
```

**Test:** Add check AFTER render:
```rust
// Check if anything was actually rendered
let buffer = FPDFBitmap_GetBuffer(bitmap);
let first_bytes = std::slice::from_raw_parts(buffer as *const u8, 100);
eprintln!("[DEBUG] First 100 bytes after render: {:02x?}", &first_bytes[..20]);
// If all 0xFF: Rendering failed
// If varies: Rendering worked
```

### Hypothesis 2: CreateEx With Nullptr Breaks Rendering

**In iteration #113, worker changed to nullptr:**
```rust
FPDFBitmap_CreateEx(..., std::ptr::null_mut(), 0)
```

**Question:** Does this break the render?

**Test:** Try WITH custom buffer again, but DON'T call FillRect:
```rust
let mut buffer = vec![0x00u8; buffer_size];  // Start with BLACK (0x00), not white
let bitmap = FPDFBitmap_CreateEx(..., buffer.as_mut_ptr(), stride);
// SKIP FillRect entirely
FPDF_RenderPageBitmap(...);  // Render directly to black background

// If content appears on black: FillRect was overwriting!
// If still wrong: Different issue
```

---

## EXPERIMENT: Remove FillRect Temporarily

**Try this quick test:**

```rust
// Comment out FillRect:
// FPDFBitmap_FillRect(bitmap, 0, 0, width_px, height_px, fill_color);

// Render directly:
FPDF_RenderPageBitmap(...);

// Check output:
// - If content appears (even on wrong background): FillRect was blocking!
// - If still white: Rendering itself is failing
```

---

## Git History Check Needed

**User suggests:** "Check git blame history" of FillRect

**Questions:**
1. Was there EVER a version that worked without FillRect?
2. When was FillRect first added?
3. Did rendering work BEFORE FillRect was added?

**Action:** Find the commit that FIRST added FillRect, check if rendering worked before that.

---

## Immediate Test For Worker

### Test A: Remove FillRect

```rust
// In render_page_to_ppm (line 376):
// FPDFBitmap_FillRect(bitmap, 0, 0, width_px, height_px, fill_color);  // COMMENT OUT

// Rebuild and test:
cargo build --release --example render_pages
DYLD_LIBRARY_PATH=... render_pages 0100pages.pdf /tmp/test 1 300 --ppm

md5 /tmp/test/page_0006.ppm
# If matches baseline: FillRect was the problem!
# If still wrong: Different issue
```

### Test B: Fill With Black Instead

```rust
// Change fill color to black:
FPDFBitmap_FillRect(bitmap, 0, 0, width_px, height_px, 0x00000000);  // Black, not white

// If content appears on black background: Content IS rendering
// If still white: Content NOT rendering
```

---

## Priority Actions

1. **Test removing FillRect** (5 minutes)
2. **Check git history** - find first FillRect commit
3. **Compare to upstream** - does upstream call FillRect before or after render?

---

**This could be the breakthrough!**

Reference: MANAGER_SMOKING_GUN_FILLRECT.md
