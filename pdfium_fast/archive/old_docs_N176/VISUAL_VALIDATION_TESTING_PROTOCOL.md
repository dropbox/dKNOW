# Visual Validation Testing Protocol

**Date:** 2025-11-03
**Purpose:** Easy visual inspection for form rendering validation

---

## Problem with MD5-Only Testing

**Current approach:**
```
MD5 mismatch → FAIL
```

**Issue:** Can't tell if difference is:
- Critical rendering bug
- Minor form field difference (acceptable)
- Tiny pixel variation (< 1%)

---

## Solution: Visual Inspection Protocol

### For Worker: When MD5 Doesn't Match

**Step 1: Generate comparison images**
```bash
# After getting MD5 mismatch on page 10:

# Convert upstream baseline
sips -s format png /tmp/upstream/page.ppm --out /tmp/upstream_page10.png

# Convert Rust output
sips -s format png /tmp/rust/page_0010.ppm --out /tmp/rust_page10.png

# Open both
open /tmp/upstream_page10.png /tmp/rust_page10.png
```

**Step 2: Visual inspection**

**Ask:**
1. Can you see ANY difference? (look carefully!)
2. If yes, WHERE is the difference?
   - Form field/checkbox?
   - Text content?
   - Background?
   - Entire page?

3. HOW MUCH of the page differs?
   - Tiny spot (< 1%)?
   - Large area (> 10%)?
   - Whole page?

**Decision tree:**
- **No visible difference:** Accept (< 0.01% pixels, imperceptible)
- **Tiny form field:** Accept (expected without FFI callbacks)
- **Large form area:** Document (may be acceptable)
- **Text/content differs:** FAIL (must fix)
- **Whole page wrong:** FAIL (serious bug)

---

## Automated Visual Testing

**For CI/test suite:**

```python
# After rendering
def compare_images(upstream_path, rust_path):
    from PIL import Image
    import numpy as np

    up = np.array(Image.open(upstream_path).convert('RGB'))
    rust = np.array(Image.open(rust_path).convert('RGB'))

    diff = np.abs(up.astype(int) - rust.astype(int))
    diff_pixels = np.sum(np.any(diff > 0, axis=2))
    total = up.shape[0] * up.shape[1]

    diff_pct = 100 * diff_pixels / total

    # Thresholds
    if diff_pct < 0.01:
        return "PASS", "Imperceptible difference"
    elif diff_pct < 1.0:
        return "WARN", f"Minor difference ({diff_pct:.2f}% pixels) - likely form fields"
    else:
        return "FAIL", f"Significant difference ({diff_pct:.2f}% pixels)"
```

---

## Test Cases for Form Rendering

### Known Form PDFs (Should Pass After C++ Bridge)

**Test these specifically:**
1. **0100pages page 10** (0.86% diff → 0%)
   - Has web form signature field
   - Currently renders white instead of grey
   - After fix: Should render grey box

2. **0309pages page 10** (0.04% diff → 0%)
   - Has 2 form checkboxes
   - Currently renders white
   - After fix: Should render checkbox outlines

3. **web_003 page 4** (28% diff → 0%)
   - Large web form
   - Currently renders mostly white
   - After fix: Should render full form structure

4. **web_041 page 0** (44% diff → 0%)
   - Huge form document
   - Currently renders white
   - After fix: Should render complete form

**Validation:**
```bash
# After implementing C++ bridge:
for pdf_page in "0100pages:10" "0309pages:10" "web_003:4" "web_041:0"; do
  pdf="${pdf_page%:*}"
  page="${pdf_page#*:}"

  # Render
  render_pages_bridge $pdf /tmp/test 1 300 --ppm

  # Convert to PNG
  sips -s format png /tmp/test/page_$(printf "%04d" $page).ppm \
       --out /tmp/${pdf}_${page}_after_fix.png

  # Compare with baseline (manual inspection)
  open /tmp/${pdf}_${page}_after_fix.png

  # Should see:
  # - Grey/blue form fields (not white)
  # - Form structure visible
  # - Matches what you saw in upstream images
done
```

---

## Quick Visual Test Script

**For worker to use:**

```bash
#!/bin/bash
# quick_visual_test.sh

PDF=$1
PAGE=$2

# Generate both versions
echo "Generating upstream..."
cd /tmp && mkdir -p visual_test && cd visual_test
cp /Users/ayates/pdfium/integration_tests/pdfs/benchmark/$PDF .
DYLD_LIBRARY_PATH=/Users/ayates/pdfium/out/Optimized-Shared \
  /Users/ayates/pdfium/out/Optimized-Shared/pdfium_test \
  --ppm --scale=4.166666 --pages=$PAGE $PDF

echo "Generating with bridge..."
DYLD_LIBRARY_PATH=/Users/ayates/pdfium/out/Optimized-Shared \
  /Users/ayates/pdfium/rust/target/release/examples/render_pages_bridge \
  /Users/ayates/pdfium/integration_tests/pdfs/benchmark/$PDF \
  /tmp/visual_test/rust 1 300 --ppm

# Convert both
sips -s format png ${PDF}.${PAGE}.ppm --out upstream.png
sips -s format png rust/page_$(printf "%04d" $PAGE).ppm --out rust.png

# Open for comparison
open upstream.png rust.png

# Check MD5
echo "Upstream MD5: $(md5 -q ${PDF}.${PAGE}.ppm)"
echo "Rust MD5:     $(md5 -q rust/page_$(printf "%04d" $PAGE).ppm)"
```

**Usage:**
```bash
./quick_visual_test.sh 0100pages_7FKQLKX273JBHXAAW5XDRT27JGMIZMCI.pdf 10
# Visually compare: Do form fields look the same?
```

---

## Acceptance Criteria

### For Each Form PDF

**After C++ bridge implementation:**

1. **Visual inspection:** Form fields should be visible
   - Grey/blue boxes (not white)
   - Checkbox outlines (not empty)
   - Form structure rendered

2. **MD5 check:** Should match baseline
   - If matches: ✓ Perfect!
   - If doesn't match but visually identical: Measure pixel diff
   - If < 0.1%: Accept (sub-pixel differences)

3. **Pixel analysis:** If MD5 doesn't match
   ```python
   diff_pct = analyze_pixel_difference(upstream, rust)
   if diff_pct < 0.1:
       PASS  # Imperceptible
   else:
       INVESTIGATE  # Something still wrong
   ```

---

## For MANAGER Review

**After worker implements C++ bridge:**

Worker should commit with:
```
[WORKER0] # XXX: C++ Bridge Complete - Form Rendering Results

## Visual Validation

Inspected 9 form PDFs:
- 0100pages page 10: ✓ Grey form field visible
- 0309pages page 10: ✓ Checkboxes rendered
- web_003 page 4: ✓ Form structure visible
...

## MD5 Results

197 PDFs tested:
- XXX PDFs: 100% MD5 match
- YYY PDFs: < 0.1% pixel diff (visually identical)
- ZZZ PDFs: Still have issues (investigate)

## Images

Attached comparison images for review:
- /tmp/comparisons/*.png
```

---

## Summary

**Visual inspection = Easy validation**
- Form differences are OBVIOUS (grey vs white boxes)
- Can confirm fix worked immediately
- No need to debug MD5 mismatches
- Clear pass/fail

**For worker:** Use visual tests during development
**For validation:** SSIM/pixel analysis for quantification
**For acceptance:** Both visual + MD5 match

---

**This makes testing much faster and clearer!**
