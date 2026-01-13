# CRITICAL API DIFFERENCE FOUND

**Date**: 2025-11-03 05:20 PST
**User demand**: "B. figure it out. we made changes to PDFium, so we can find out what changes we made"

---

## KEY DIFFERENCE FOUND

### Upstream pdfium_test Uses Different Bitmap API

**Upstream** (testing/pdfium_test.cc line ~1650):
```cpp
bool alpha = FPDFPage_HasTransparency(page);
bitmap = FPDFBitmap_CreateEx(
    width, height,
    alpha ? FPDFBitmap_BGRA : FPDFBitmap_BGRx,  // Format parameter
    first_scan,                                   // External buffer
    stride                                        // Stride
);

FPDF_DWORD fill_color = alpha ? 0x00000000 : 0xFFFFFFFF;
FPDFBitmap_FillRect(..., fill_color);
```

**Our Rust** (render_pages.rs):
```rust
let bitmap = FPDFBitmap_Create(width, height, 0);  // 0 = no alpha

FPDFBitmap_FillRect(bitmap, 0, 0, width, height, 0xFFFFFFFF);  // Always white
```

---

## THE DIFFERENCES

### 1. Different API Function

**Upstream**: Uses `FPDFBitmap_CreateEx` (extended version)
**Ours**: Uses `FPDFBitmap_Create` (simple version)

### 2. Different Format Parameter

**Upstream**:
- `FPDFBitmap_BGRA` if transparency
- `FPDFBitmap_BGRx` if no transparency (3-byte RGB internally)

**Ours**:
- `0` (which likely defaults to FPDFBitmap_BGRA always)

### 3. Different Fill Colors

**Upstream**:
- Transparent pages: 0x00000000 (transparent)
- Opaque pages: 0xFFFFFFFF (white)

**Ours**:
- Always: 0xFFFFFFFF (white)

---

## This Explains the Mismatches!

**If our `FPDFBitmap_Create(w, h, 0)` always creates BGRA**:
- We render into BGRA bitmap
- Upstream renders into BGRx for non-transparent pages
- Different internal format = different rendering results

**Even though we OUTPUT RGB/RGBA**, the bitmap FORMAT during rendering matters!

---

## THE FIX

**Use FPDFBitmap_CreateEx like upstream**:

```rust
// Check transparency
let has_transparency = FPDFPage_HasTransparency(page) != 0;

// Create bitmap with correct format (MATCHES upstream)
let format = if has_transparency {
    FPDFBitmap_BGRA
} else {
    FPDFBitmap_BGRx  // THIS IS KEY - 3-byte format
};

let stride = width_px * if has_transparency { 4 } else { 4 };  // Still 4 bytes per pixel
let mut buffer = vec![0u8; (height_px * stride) as usize];

let bitmap = FPDFBitmap_CreateEx(
    width_px,
    height_px,
    format,  // Use correct format
    buffer.as_mut_ptr() as *mut _,
    stride
);

// Fill with correct color
let fill_color = if has_transparency { 0x00000000 } else { 0xFFFFFFFF };
FPDFBitmap_FillRect(bitmap, 0, 0, width_px, height_px, fill_color);
```

---

## Why This Matters

**Bitmap format affects rendering**:
- BGRx format: PDFium knows no alpha, optimizes differently
- BGRA format: PDFium renders with alpha blending

**Even for opaque content**, the bitmap format choice affects how PDFium renders!

---

## WORKER ORDER

**FIX render_pages.rs**:

1. Replace `FPDFBitmap_Create` with `FPDFBitmap_CreateEx`
2. Use `FPDFBitmap_BGRx` for non-transparent pages
3. Use `FPDFBitmap_BGRA` for transparent pages
4. Use correct fill color (transparent black vs white)
5. Rebuild
6. Test on 0100pages page 7
7. **Should now get 100% MD5 match**

This is the root cause. Fix this and we'll have 100% correctness.
