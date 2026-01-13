# PDF Coordinate Mismatch - Y-Axis Issue Found

**Date:** 2025-11-25 N=2288
**Status:** 🚨 ROOT CAUSE IDENTIFIED - Cells and Clusters in Different Y-Coordinate Ranges

## Problem

After implementing scale transform (N=2288), cells are STILL not being assigned to clusters on some pages.

**Current results:**
- 39 DocItems (expected 53)
- Missing: 6 section_header, 5 text, 3 list_item

## Root Cause Found

**On Page 2 (index 1), cells and clusters do NOT overlap:**

**Cells Y-range: 74-198**
```
Cell[0] bbox=(72.4, 74.7)→(516.5, 85.6)   ← Y: 74-85
Cell[1] bbox=(72.0, 88.4)→(466.9, 99.3)   ← Y: 88-99
Cell[3] bbox=(72.5, 158.7)→(268.3, 170.4) ← Y: 158-170
Cell[4] bbox=(72.2, 187.5)→(510.9, 198.4) ← Y: 187-198
```

**Clusters Y-range: 295-768**
```
Cluster[0] bbox=(71.2, 378.4)→(517.7, 445.9) ← Y: 378-445
Cluster[1] bbox=(71.2, 295.3)→(512.5, 334.9) ← Y: 295-334
Cluster[2] bbox=(71.1, 728.1)→(513.2, 768.1) ← Y: 728-768
```

**Result:** NO OVERLAP → 6 out of 9 clusters get 0 cells assigned!

## Why This Happens

**Two different coordinate transforms:**

1. **Clusters** (executor.rs:854-912):
   - Scale from IMAGE coords to PDF coords: `pdf_coord = image_coord / scale`
   - Y-flip from PDF to screen: `screen_y = page_height - pdf_y`
   - Result: Y-range 295-768

2. **Cells** (executor.rs:1307):
   - Transform: `convert_textline_coords(cells, page_height)`
   - Result: Y-range 74-198

**The two transforms produce DIFFERENT coordinate systems!**

## Investigation Needed

**Check convert_textline_coords function:**

1. Where is it defined? (Likely in executor.rs around line 914)
2. What transform does it apply?
3. Does it scale from image to PDF coordinates?
4. Does it Y-flip the same way as clusters?

**Hypothesis:**
- Cells are already in PDF coordinates (from pdfium)
- Cells only get Y-flipped
- Clusters get BOTH scaled AND Y-flipped
- But page_height used for Y-flip might be different!
- OR: Cells should ALSO be scaled but aren't

## Fix Strategy

**Option A: Cells need same scale transform as clusters**
```rust
// Current (wrong):
let textline_cells = textline_cells.map(|cells| convert_textline_coords(cells, page_height));

// Fixed:
let textline_cells = textline_cells.map(|cells| {
    convert_textline_coords_with_scale(cells, image_width, image_height, page_width, page_height)
});
```

**Option B: Cells are already in correct coords, clusters transform is wrong**
- Check if clusters should use different page_height
- Check if Y-flip formula is correct

## Test Case

**Page 0 (index 0) WORKS:**
- 12 out of 14 clusters get cells
- Only 2 clusters without cells

**Page 1 (index 1) FAILS:**
- 3 out of 9 clusters get cells
- 6 clusters without cells
- Clear Y-coordinate mismatch

**Reproduce:**
```bash
source setup_env.sh
cargo test -p docling-backend --test pdf_honest_test save_rust_docling_json \
  --features pdf-ml -- --exact --nocapture 2>&1 | grep -A 30 "Page 1"
```

## Next Steps

1. Read convert_textline_coords function (executor.rs:~914)
2. Compare cell transform with cluster transform
3. Identify which one is wrong
4. Apply consistent coordinate transform to both
5. Test that all pages get proper cell assignment
6. Verify 53 DocItems output

## Expected Outcome

After fix:
- Cells Y-range should match clusters Y-range
- All clusters should get cells assigned (or very few empty)
- Output: 53 DocItems (16 text, 11 section_header, 26 list_item)
