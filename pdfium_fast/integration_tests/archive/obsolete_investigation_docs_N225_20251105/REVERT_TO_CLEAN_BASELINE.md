# Revert to Clean Baseline - 100% Correctness Priority

**User**: "oh no! we need to fix that PDF! if we need to restart our optimization work on pdfium, we can do that, too. I'd prefer to start from a clean baseline."

**CORRECT PRIORITY** - Correctness > Optimization

---

## Current Problem

**0100pages PDF**: 68/100 pages match (32% broken)
**Other PDFs**: 100% match

**This suggests our "fixes" broke something on certain page types.**

---

## Changes We Made (Commits #57-#63)

### Change 1: Dimension Truncation (#63)
```rust
// BEFORE:
let width_px = (width_pts * scale) as i32;

// AFTER:
let scale = ((dpi / 72.0) * 1_000_000.0).floor() / 1_000_000.0;
let width_px = (width_pts * scale).floor() as i32;
```

### Change 2: Transparency Check (#57, #62, #63)
```rust
// ADDED:
let has_transparency = FPDFPage_HasTransparency(page) != 0;
let channels = if has_transparency { 4 } else { 3 };

// Conditional RGB/RGBA output
encoder.set_color(if has_transparency {
    png::ColorType::Rgba
} else {
    png::ColorType::Rgb
});
```

### Change 3: Compression Settings (#62)
```rust
// ADDED:
encoder.set_compression(png::Compression::Default);
```

### Change 4: Removed sRGB (#62)
```rust
// REMOVED:
encoder.set_source_srgb(png::SrgbRenderingIntent::Perceptual);
```

---

## Hypothesis: Transparency Check Is Wrong

**Suspect**: FPDFPage_HasTransparency() might return wrong values on some pages

**Evidence**:
- 68 pages work (probably all text pages)
- 32 pages fail (probably have images/graphics)
- Scanned document = images on every page
- Maybe transparency detection is backwards?

---

## INVESTIGATION PLAN

### Test 1: Check Page 7 Transparency

```bash
# What does upstream use?
# Check if upstream PNG is RGB or RGBA
file /path/to/upstream_page7.png
# Should show: RGB or RGBA

# What does our code think?
# Add debug print in render_pages.rs before line 241:
eprintln!("Page 7: has_transparency = {}", has_transparency);
```

### Test 2: Force All RGBA (Revert Transparency Check)

```rust
// In render_pages.rs, line ~241:
// IGNORE transparency check, always use RGBA
let channels = 4;  // Force RGBA

encoder.set_color(png::ColorType::Rgba);  // Force RGBA
```

Re-render 0100pages page 7, check if MD5 matches.

### Test 3: If Still Fails - Revert Everything

**Revert to commit BEFORE #57** (before any RGB/RGBA changes):
```bash
git show 62de6d5893^:rust/pdfium-sys/examples/render_pages.rs > render_pages_clean.rs
cp render_pages_clean.rs rust/pdfium-sys/examples/render_pages.rs
cargo build --release --example render_pages
```

Test if that gives 100% match.

---

## CLEAN BASELINE PLAN

**User prefers**: "start from a clean baseline"

### Option A: Revert All Rendering Changes

1. Checkout render_pages.rs from before our changes
2. Use original Rust tool (no dimension tricks, no transparency checks)
3. Accept whatever MD5/SSIM we get
4. Document: "Using unmodified rendering, validated with SSIM"

### Option B: Fix Transparency Bug

1. Investigate why transparency check fails on 32 pages
2. Fix the check
3. Get 100% MD5 match

### Option C: Remove Transparency Optimization Entirely

1. Always output RGBA (simpler, safer)
2. Accept larger file sizes
3. Should get 100% match (or reveal other bugs)

---

## RECOMMENDATION

**Immediate**: Test Option C (always RGBA)
- Simple one-line change
- If fixes problem: Transparency check was the bug
- If doesn't fix: Something else is wrong

**Then**:
- If Option C works: Keep it (correctness > file size)
- If Option C fails: Revert everything (Option A)

---

## WORKER ORDER

**STOP all validation**

**Test immediately**:
1. Edit render_pages.rs: Force RGBA (remove transparency check)
2. Rebuild
3. Render 0100pages page 7
4. Compare MD5 with upstream
5. Report: Match or still differ?

**If match**: Transparency check was the bug - remove it
**If differ**: Revert all changes, start from clean baseline

**User is right**: Clean 100% baseline > optimizations that break correctness
