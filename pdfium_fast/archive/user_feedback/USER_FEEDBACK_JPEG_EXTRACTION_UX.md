# User Feedback: JPEG Extraction UX Problem

**Date**: 2025-11-20
**User**: extract_a project
**Issue**: Cannot extract pages as JPEGs, must render to PNG then convert

---

## Problem Statement

**What we wanted to do**:
Extract all pages from PDFs as JPEG images (for Dataset A corpus: 169K PDFs).

**What we had to do**:
```bash
pdfium_cli render-pages input.pdf output_dir/  # Renders to PNG
# Then manually convert:
for f in output_dir/*.png; do
    sips -s format jpeg "$f" --out "${f%.png}.jpg"
    rm "$f"
done
```

**Result**:
- Wasteful PNG→JPEG conversion
- Extra disk I/O and processing time
- Started extraction: 87 GB of PNGs generated for just 3,090 PDFs
- Estimated 4.5 TB for full corpus before conversion
- Had to kill the job

**Why this is confusing**:
- Help text mentions "Smart mode (JPEG Fast Path)" with 545x speedup
- Suggests pdfium_fast CAN work with JPEGs
- But no way to request JPEG output for regular rendering
- Only automatic for scanned PDFs with embedded JPEGs

---

## What Users Expect

**Expected interface**:
```bash
# Extract pages as JPEGs
pdfium_cli extract-pages --format jpg input.pdf output_dir/

# Or render with format flag
pdfium_cli render-pages --format jpg input.pdf output_dir/
```

**Current interface**:
```bash
pdfium_cli render-pages input.pdf output_dir/        # PNG only
pdfium_cli render-pages --ppm input.pdf output_dir/  # PPM only
# No JPEG option!
```

---

## Proposed Solution

### Option A: Add `--format` flag (RECOMMENDED)

**Implementation** (simplest):
```bash
pdfium_cli render-pages --format jpg input.pdf output_dir/
pdfium_cli render-pages --format png input.pdf output_dir/  # current default
pdfium_cli render-pages --format ppm input.pdf output_dir/  # replaces --ppm
```

**Advantages**:
- Intuitive and standard (matches other image tools)
- Minimal code change (just add JPEG encoding)
- Backwards compatible (default to PNG)
- Clear and unambiguous

**CLI Help Update**:
```
Flags:
  --format FMT      Output format: png|jpg|ppm (default png)

Examples:
  pdfium_cli render-pages --format jpg input.pdf output/
  pdfium_cli render-pages --format png input.pdf output/
```

**Effort**: 2-3 hours

---

### Option B: Add `extract-images` operation

**Implementation**:
```bash
pdfium_cli extract-images input.pdf output_dir/  # Extract embedded images as-is
pdfium_cli render-pages input.pdf output_dir/   # Render to PNG (current)
```

**When to use what**:
- `extract-images`: For scanned PDFs (preserves original JPEG, 545x faster)
- `render-pages`: For native PDFs (renders to PNG/PPM)

**Advantages**:
- Makes "JPEG Fast Path" explicit and user-accessible
- Separates extraction vs. rendering concepts
- Educational (users learn the difference)

**Disadvantages**:
- Requires user to know if PDF is scanned or native
- Doesn't help users who want JPEG output for native PDFs

**Effort**: 1-2 hours (expose existing JPEG extraction)

---

### Option C: Smart extraction with format control

**Implementation**:
```bash
# Smart mode: Uses JPEG fast path if available, else renders
pdfium_cli extract-pages input.pdf output_dir/

# Explicit format control
pdfium_cli extract-pages --format jpg input.pdf output_dir/
pdfium_cli extract-pages --format png input.pdf output_dir/

# Force rendering (bypass smart mode)
pdfium_cli render-pages --format png input.pdf output_dir/
```

**Advantages**:
- Best UX (smart by default)
- Users don't need to know PDF internals
- Explicit control when needed
- Clear naming: "extract" vs. "render"

**Disadvantages**:
- More complex implementation
- Need to explain smart mode behavior

**Effort**: 4-6 hours

---

## Recommended Implementation: Option A

**Add `--format` flag to `render-pages`**

### Code Changes Required

**1. Add format flag parsing** (src/cli.cc or equivalent):
```cpp
enum class OutputFormat {
    PNG,
    JPG,
    PPM
};

OutputFormat parse_format(const std::string& format) {
    if (format == "jpg" || format == "jpeg") return OutputFormat::JPG;
    if (format == "ppm") return OutputFormat::PPM;
    return OutputFormat::PNG;  // default
}
```

**2. Add JPEG encoding** (after rendering):
```cpp
// After FPDFBitmap_GetBuffer() generates RGBA bitmap
if (format == OutputFormat::JPG) {
    // Use libjpeg or similar to encode RGBA → JPEG
    // Quality: 90-95 (configurable via --quality flag)
    save_as_jpeg(bitmap_data, width, height, output_path, quality);
} else if (format == OutputFormat::PNG) {
    // Current PNG encoding
}
```

**3. Update help text**:
```
Flags:
  --format FMT      Output format: png|jpg|ppm (default png)
  --quality N       JPEG quality 1-100 (default 90, only for jpg format)
```

**4. Update examples**:
```
Examples:
  pdfium_cli render-pages --format jpg input.pdf output/
  pdfium_cli render-pages --format jpg --quality 95 input.pdf output/
```

### Testing

```bash
# Test PNG (existing)
pdfium_cli render-pages test.pdf output_png/
# Should output: page_001.png, page_002.png, ...

# Test JPG (new)
pdfium_cli render-pages --format jpg test.pdf output_jpg/
# Should output: page_001.jpg, page_002.jpg, ...

# Test quality
pdfium_cli render-pages --format jpg --quality 80 test.pdf output_low/
pdfium_cli render-pages --format jpg --quality 100 test.pdf output_high/
# Compare file sizes and quality
```

---

## Impact Analysis

### Disk Space Savings (for our use case)

**Current approach (PNG→JPG conversion)**:
- 3,090 PDFs → 87 GB of PNGs (before conversion)
- Full corpus: 169K PDFs → ~4.5 TB of PNGs (wasteful)

**With native JPEG output**:
- 3,090 PDFs → ~8-15 GB of JPEGs (estimated, 5-10x compression)
- Full corpus: 169K PDFs → ~450-800 GB of JPEGs (manageable)
- **Savings**: ~4 TB of disk space and conversion time

### Performance Impact

**Current**:
1. Render to PNG (~100ms per page)
2. Convert PNG→JPG (~50ms per page)
3. Delete PNG (~10ms per page)
**Total**: ~160ms per page

**With native JPEG**:
1. Render to JPG (~100ms per page)
**Total**: ~100ms per page
**Speedup**: 1.6x faster

### Real-World Impact

For our Dataset A extraction:
- **Time saved**: ~2-3 hours (out of 7-8 hour total)
- **Disk space saved**: ~3-4 TB
- **Complexity reduced**: No conversion step needed

---

## Alternative: Document Current Limitations

If adding JPEG support is too complex, at minimum document the limitation:

**Add to help text**:
```
Output Formats:
  PNG: Default format (lossless, large files ~10MB/page)
  PPM: Raw format (use --ppm flag, larger than PNG)
  JPEG: Not currently supported
        Workaround: Render to PNG, then convert with ImageMagick/sips
        Note: Smart mode extracts embedded JPEGs automatically for scanned PDFs
```

**Add to README/docs**:
```markdown
## FAQ: Why no JPEG output?

Q: Can I get JPEG output instead of PNG?
A: Not directly. Current workaround:

    pdfium_cli render-pages input.pdf output/
    for f in output/*.png; do
        convert "$f" "${f%.png}.jpg" && rm "$f"
    done

We're aware this is inefficient. JPEG output is on the roadmap.

For scanned PDFs with embedded JPEGs, use smart mode (automatic):
Smart mode extracts original JPEGs directly (545x faster).
```

---

## Questions for pdfium_fast AI

1. **Is JPEG encoding intentionally excluded?** Technical reason or just not implemented?
2. **Effort estimate**: How complex would `--format jpg` be to add?
3. **Dependencies**: Does pdfium_fast already link against libjpeg? (For smart mode)
4. **Alternative**: Could `extract-images` operation expose embedded image extraction?

---

## User Impact Statement

**Without JPEG support**:
- We cannot efficiently extract 169K PDFs to images
- PNG output would require ~4.5 TB of disk space
- Conversion step adds 2-3 hours of processing time
- Makes pdfium_fast impractical for large-scale image extraction

**With JPEG support**:
- Extraction becomes practical (~450-800 GB, manageable)
- Faster processing (no conversion step)
- Simpler code (no post-processing needed)
- We would use pdfium_fast for both text AND images

**Current decision**: Use pdfium_fast for **text-only** extraction until JPEG support is added.

---

## Suggested Priority

**High Priority** - This significantly limits pdfium_fast's usefulness for image extraction workflows.

Most users want JPEGs for:
- Web publishing (smaller file size)
- Machine learning datasets (standard format)
- Storage efficiency (10x smaller than PNG)
- Compatibility (universal support)

PNG is only preferred for:
- Lossless archival
- Images with transparency
- Screenshots with text

For PDF page rendering, JPEG is the practical choice for 90% of use cases.

---

**Submitted by**: extract_a project
**Priority**: High
**Effort**: 2-3 hours (Option A)
**Impact**: Makes image extraction practical for large corpora
