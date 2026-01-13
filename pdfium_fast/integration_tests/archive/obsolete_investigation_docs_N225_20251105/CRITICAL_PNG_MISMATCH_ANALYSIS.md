# 🚨 CRITICAL - PNG Mismatches Found - Investigation Required

**Date**: 2025-11-02 19:35 PST
**User demand**: "We need to find exactly what the PNG differences are. we must have 100% correctness even if that means reverting previous progress"

---

## CRITICAL FINDING

**ALL 32 mismatches are from ONE PDF: 0100pages_7FKQLKX273JBHXAAW5XDRT27JGMIZMCI.pdf**

**0100pages**:
- Pages match: 68/100 (68%)
- Pages differ: 32/100 (32%)

**Other 4 PDFs** (0106, 0109, 0124, 0130):
- Pages match: 469/469 (100%) ✅
- Pages differ: 0/469 (0%)

**This is NOT random** - Other PDFs are 100% perfect, ONE PDF has 32% mismatch.

---

## Why This Matters

**User is right**: Should be 100% exact match

**Current**: 94.4% match rate sounds good, but it's misleading
- Really: 4/5 PDFs perfect (100%)
- 1/5 PDFs broken (68%)

**Question**: What's special about 0100pages PDF that causes mismatches on 32 pages?

---

## Pages That Differ (0100pages)

**Pattern analysis**:
- Pages 1-6: Match ✅
- **Page 7-8: Differ** ❌
- Pages 9-10: Match ✅
- **Page 11: Differs** ❌
- Page 12: Match ✅
- **Page 13: Differs** ❌
- Pages 14-15: Match ✅
- **Page 16: Differs** ❌

**Pattern**: Sporadic, not sequential. Suggests certain page content types trigger difference.

---

## Hypothesis

**Possible causes**:

### 1. PNG Compression Differences
- Certain images/content compress differently
- Our Rust png crate vs upstream libpng
- Same pixels, different compression = different MD5

### 2. Transparency Handling
- FPDFPage_HasTransparency() returns different values?
- RGB vs RGBA output mismatch on specific pages?

### 3. Color Space Issues
- Certain pages have different color spaces
- sRGB vs device RGB
- Color profile embedding differences

### 4. Anti-Aliasing Differences
- Text rendering anti-aliasing
- Platform-specific rendering

### 5. Random Seed (unlikely)
- Some compression uses random seeds
- Would affect all pages, not just some

---

## INVESTIGATION REQUIRED

### Step 1: Compare ONE Mismatching Page Visually

**Test page 7 of 0100pages**:
```bash
# Generate both
pdfium_test --ppm --scale=4.166666 0100pages.pdf
sips -s format png 0100pages.pdf.6.ppm --out upstream_p7.png

render_pages 0100pages.pdf /tmp/ours 1 300
cp ours/page_0006.png our_p7.png

# Compare visually
open upstream_p7.png our_p7.png

# Check pixel difference
python3 -c "
from PIL import Image
import numpy as np
up = np.array(Image.open('upstream_p7.png'))
ours = np.array(Image.open('our_p7.png'))
print('Shapes:', up.shape, ours.shape)
print('Equal:', np.array_equal(up, ours))
if not np.array_equal(up, ours):
    diff = np.abs(up.astype(int) - ours.astype(int))
    print('Max diff:', diff.max())
    print('Mean diff:', diff.mean())
    print('Pixels differ:', (diff > 0).sum())
"
```

### Step 2: Check PNG Properties

```bash
file upstream_p7.png
file our_p7.png

# Check PNG chunks
pngcheck -v upstream_p7.png
pngcheck -v our_p7.png

# Check exact bytes
hexdump -C upstream_p7.png | head -30
hexdump -C our_p7.png | head -30
```

### Step 3: Check If Pixels Are Identical

If pixels are identical but MD5 differs:
- **Cause**: PNG encoding (compression, chunks, metadata)
- **Action**: Accept SSIM > 0.99 as proof of correctness
- **Status**: Rendering correct, encoding differs

If pixels differ:
- **Cause**: Rendering bug
- **Action**: Fix rendering issue
- **Status**: NOT correct, must fix

---

## WORKER ORDER

**STOP validation immediately**

**INVESTIGATE**:
1. Compare page 7 of 0100pages (first mismatch)
2. Check if pixels are identical
3. If pixels identical: PNG encoding issue (acceptable)
4. If pixels differ: RENDERING BUG (must fix)

**If rendering bug found**: Revert dimension fix or find root cause

**User demand**: "100% correctness even if that means reverting previous progress"

**DO NOT continue validation until we know WHY 32 pages differ.**

---

## This Is Critical

**Cannot declare 100% validation with 32 unexplained mismatches.**

Must investigate and either:
- Explain: "Pixels identical, PNG encoding differs" (acceptable)
- Fix: "Found rendering bug, fixed" (required)
- Revert: "Cannot achieve 100%, reverted to known-good state"

**User is right to demand investigation.**
