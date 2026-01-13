# PDF Coordinate Bug FIXED - Y-Axis Flip Removed

**Date:** 2025-11-25 N=2289
**Status:** ✅ FIXED - Cell assignment working, 53 DocItems output

## Problem (N=2288b)

After implementing scale transform (N=2288), cells were STILL not being assigned to clusters on some pages.

**Symptoms:**
- Page 1: Only 3/9 clusters got cells (6 empty clusters)
- Cells Y-range: 74-198
- Clusters Y-range: 295-768
- NO OVERLAP → Cell assignment failed
- Output: 39 DocItems (expected 53, missing 14)

## Root Cause

The `convert_clusters_to_labeled` function incorrectly applied a Y-flip transform to cluster coordinates.

**Incorrect assumption:**
- Code assumed ML model outputs bboxes in PDF coordinates with BOTTOM-LEFT origin
- Applied Y-flip: `screen_y = page_height - pdf_y`

**Reality:**
- ML model (YOLOv8) outputs bboxes in IMAGE coordinates with **TOP-LEFT origin** (like all computer graphics)
- After scaling to PDF size, coordinates REMAIN in TOP-LEFT origin
- Y-flip was inverting clusters, placing them in wrong positions

**Example:**
```
Image coords (top-left): Y=1649 (middle of 3508-pixel image)
After scale: Y=396 (middle of 842-point page) ✓ CORRECT
After wrong flip: Y=445 (378 from top) ❌ WRONG POSITION
```

## The Fix (N=2289)

**Removed the Y-flip transform from `convert_clusters_to_labeled`:**

```rust
// BEFORE (N=2288, WRONG):
let screen_t = (page_height as f64) - pdf_b;  // Y-flip
let screen_b = (page_height as f64) - pdf_t;  // Y-flip

// AFTER (N=2289, CORRECT):
let pdf_t = (c.bbox.t as f64) / (scale_y as f64);  // Scale only
let pdf_b = (c.bbox.b as f64) / (scale_y as f64);  // No flip!
```

**Why this is correct:**
- Image coords: TOP-LEFT origin (Y increases downward)
- After scale: TOP-LEFT origin preserved (just different size)
- Cells from pdfium: Also TOP-LEFT origin (converted in extract_text_cells_simple)
- Both use same coordinate system → No flip needed!

## Results

**After fix (N=2289):**
- ✅ Page 0: 14/14 clusters got cells (100%)
- ✅ Page 1: 9/9 clusters got cells (100%)
- ✅ Page 2: 14/14 clusters got cells (100%)
- ✅ Total: 53 DocItems (expected 53)
- ✅ Markdown: 9,469 chars (expected ~9,456)
- ✅ All pages working correctly

**Cell assignment results:**
```
=== STAGE 4 CELL ASSIGNMENT RESULTS ===
  ✓ Cluster[0] label=text got 6 cells
  ✓ Cluster[1] label=list_item got 4 cells
  ...
  ✓ Cluster[13] label=section_header got 1 cells
Summary: 14 clusters WITH cells, 0 clusters WITHOUT cells
```

## Files Modified

- `crates/docling-pdf-ml/src/pipeline/executor.rs`:
  - `convert_clusters_to_labeled`: Removed Y-flip transform (lines 877-905)
  - Updated function documentation to explain correct coordinate handling

## Lesson Learned

**Coordinate System Consistency:**

When integrating ML models with PDF processing:
1. **Know your origins:** ML models use top-left (computer graphics standard)
2. **Know your units:** ML models use pixels, PDFs use points
3. **Scale only:** Convert between units by scaling (divide by scale factor)
4. **Don't flip:** If both systems use same origin, no flip needed
5. **Verify:** Check that bboxes overlap after transform

**The Bug:**
- Assumed ML coords were PDF bottom-left
- Applied unnecessary Y-flip
- Created coordinate mismatch

**The Fix:**
- Recognized ML coords are image top-left
- Removed Y-flip
- Both cells and clusters now in same coordinate system

## Next Steps

1. ✅ Cell assignment working (53 DocItems)
2. ✅ Y-coordinates aligned
3. ⏭️ Compare DocItems with Python baseline (structure, not just markdown)
4. ⏭️ Verify reading order is correct
5. ⏭️ Run full quality tests

## Historical Notes

- N=2286: Identified coordinate system mismatch
- N=2287: Added scale transform (but kept wrong Y-flip)
- N=2288: Found cells/clusters in different Y-ranges
- N=2288b: Root cause analysis - cells Y=74-198, clusters Y=295-768
- **N=2289: Fixed by removing Y-flip** ✅

The coordinate bug is now resolved. All 53 DocItems are correctly generated.
