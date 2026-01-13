# Root Cause: Format Difference Between Our Renders and Upstream

**Date**: 2025-11-02 08:35 PST
**Investigation**: MANAGER analyzing why formats differ

---

## The Format Difference

**Upstream pdfium_test**:
- Output: RGB (3 channels, no alpha)
- Size: 2549×3299 pixels

**Our render_pages**:
- Output: RGBA (4 channels, with alpha)
- Size: 2550×3300 pixels

**Question**: Why are we different?

---

## ROOT CAUSE FOUND

### Issue 1: Unnecessary Alpha Channel

**pdfium_test code** (`testing/pdfium_test.cc`):
```cpp
bool alpha = FPDFPage_HasTransparency(page());
bitmap = FPDFBitmap_CreateEx(
    width, height,
    alpha ? FPDFBitmap_BGRA : FPDFBitmap_BGRx,  // ← BGRx = 3 bytes (no alpha)
    first_scan, stride
);
```

**Upstream logic**: Check if page has transparency, use alpha ONLY if needed.

**Our Rust code** (`rust/pdfium-sys/examples/render_pages.rs:212-261`):
```rust
let bitmap = FPDFBitmap_Create(width_px, height_px, 0);  // 0 = no alpha
// ↑ Creates BGRx bitmap (4 bytes, but alpha unused)

// Convert BGRA to RGBA
for pixel in bitmap {
    rgba_data.push(r);
    rgba_data.push(g);
    rgba_data.push(b);
    rgba_data.push(a);  // ← Always include alpha, even when unused!
}

encoder.set_color(png::ColorType::Rgba);  // ← Always 4 channels
```

**Our bug**: We ALWAYS output RGBA, even for pages without transparency.

**Upstream behavior**: Outputs RGB (3 channels) for pages without transparency.

**Fix needed**:
1. Call `FPDFPage_HasTransparency(page)` before rendering
2. If no transparency: Output RGB (3 channels)
3. If transparency: Output RGBA (4 channels)

**Impact**: Would make our PNGs byte-for-byte identical to upstream (after PPM→PNG conversion)

### Issue 2: Dimension Rounding

**Calculation**:
```
Page size: 612×792 points (US Letter)
DPI: 300
Target: 612 × (300/72) = 2550 pixels
        792 × (300/72) = 3300 pixels
```

**Upstream**: 2549×3299 (rounds down by 1)
**Ours**: 2550×3300 (exact calculation)

**Why different**:
- Upstream uses: `--scale=4.166666` (≈ 300/72)
- Upstream may round down: `(int)(612 * 4.166666) = 2549`
- Our code: `(612.0 * 300.0 / 72.0) as i32 = 2550`

**Fix needed**: Match upstream rounding exactly

---

## Why This Matters

**Current state**:
- Both tools use **same libpdfium.dylib** (same rendering engine)
- Visual output is identical
- But bytes differ due to:
  1. Extra alpha channel (we include, they don't)
  2. Dimension rounding (1 pixel)

**This prevents**:
- MD5 comparison (bytes differ)
- Exact validation (need SSIM instead)

**This does NOT mean**:
- Our rendering is wrong
- Our colors are wrong
- Our quality is wrong

**It means**: Output encoding differs (implementation choice, not correctness issue)

---

## THE FIX (Option A - User Requested)

### Fix 1: Remove Unnecessary Alpha Channel

**Edit**: `rust/pdfium-sys/examples/render_pages.rs:237-261`

```rust
// BEFORE rendering, check transparency
let has_transparency = FPDFPage_HasTransparency(page);

// After rendering...
if has_transparency {
    // Keep alpha channel (RGBA)
    encoder.set_color(png::ColorType::Rgba);
    for pixel in bitmap {
        rgba_data.push(r);
        rgba_data.push(g);
        rgba_data.push(b);
        rgba_data.push(a);
    }
} else {
    // Omit alpha channel (RGB) - MATCH UPSTREAM
    encoder.set_color(png::ColorType::Rgb);
    for pixel in bitmap {
        rgba_data.push(r);
        rgba_data.push(g);
        rgba_data.push(b);
        // Skip alpha
    }
}
```

**Impact**: PNG will be RGB (3 channels) for pages without transparency, matching upstream

### Fix 2: Match Upstream Dimension Rounding

**Current**:
```rust
let width_px = (width * dpi / 72.0) as i32;
```

**Fix** (if needed):
```rust
// Match upstream rounding behavior
let width_px = (width * dpi / 72.0).floor() as i32;
// OR explicitly round down to match pdfium_test
```

**But**: Need to verify upstream's exact calculation first

---

## After These Fixes

**Expected result**:
- Our PNG: RGB, 2549×3299 (matching upstream)
- Upstream PNG (from PPM): RGB, 2549×3299
- MD5: **Should match exactly**

**Then**: Can validate 100% with MD5 comparison (no SSIM needed)

---

## SSIM Alternative (Option B)

**If we DON'T fix formats**:

Need SSIM perceptual comparison:
1. Load both PNGs as images
2. Resize to same dimensions (handle 1-pixel diff)
3. Convert both to RGB (strip alpha)
4. Compute SSIM score
5. Threshold: SSIM > 0.99 = visually identical

**Time**: 2-3 hours to implement

**Value**: Works regardless of format, tests visual quality

---

## RECOMMENDATION

**User chose Option A: Definitely fix the format** ✅

**Implementation plan**:
1. Add FPDFPage_HasTransparency() check (5 min)
2. Conditional RGB/RGBA output (10 min)
3. Fix dimension rounding (5 min)
4. Rebuild Rust tools (2 min)
5. Test on 10 PDFs (30 min)
6. Verify MD5 now matches upstream (5 min)
7. Run full validation on 50 PDFs (1-2 hours)
8. Document: 100% MD5 match (30 min)

**Total time**: 2-3 hours

**Result**: Images will be **byte-for-byte identical** to upstream

---

## WORKER DIRECTIVE

🎯 **FIX IMAGE FORMAT TO MATCH UPSTREAM EXACTLY**

**Your task**:
1. Add `FPDFPage_HasTransparency()` check
2. Output RGB (no alpha) when page has no transparency
3. Output RGBA only when page has transparency
4. Fix dimension rounding to match upstream
5. Rebuild and test
6. Verify MD5 matches on 10 PDFs
7. Run full 50-PDF validation
8. Document: Images now byte-for-byte match upstream

**This will make our images IDENTICAL to upstream pdfium_test.**

**Time**: 2-3 hours

**See**: ROOT_CAUSE_FORMAT_DIFFERENCE.md for technical details
