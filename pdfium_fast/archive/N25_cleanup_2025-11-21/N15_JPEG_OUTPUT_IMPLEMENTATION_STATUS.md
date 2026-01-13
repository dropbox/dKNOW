# N=15: JPEG Output Implementation Status

**Date**: 2025-11-20
**Worker**: WORKER0
**Status**: INCOMPLETE - Build system broken, partial implementation complete

---

## Summary

Started implementing JPEG output format per PR #18 user feedback (4.5 TB PNG problem). Made significant progress on implementation but encountered build system issue (missing expat dependency). Core JPEG writing infrastructure is complete, but integration into rendering pipeline is incomplete.

---

## User Problem (PR #18)

**Critical UX Issue:**
- User attempted to extract 169K PDFs as images
- pdfium_cli only outputs PNG format
- Generated 4.5 TB of PNGs for 3,090 PDFs
- Had to kill job due to storage constraints
- Manual PNG→JPEG conversion required

**Expected Solution:**
```bash
pdfium_cli --format jpg render-pages input.pdf output_dir/
```

**Storage Impact:**
- PNG: 4.5 TB (for full corpus)
- JPEG: ~450 GB (10x reduction)
- Blocks production deployment

---

## Implementation Progress

### ✅ Completed

1. **Added Skia JPEG Encoder Headers**
   - Location: examples/pdfium_cli.cpp:49-54
   - Includes: SkData, SkPixmap, SkImageInfo, SkStream, SkJpegEncoder

2. **Implemented write_jpeg() Function**
   - Location: examples/pdfium_cli.cpp:1730-1778
   - Uses Skia's SkJpegEncoder with quality control
   - Handles BGRA→JPEG conversion
   - 4:2:0 chroma subsampling (standard JPEG)
   - Quality range: 0-100 (default 90)

3. **Added CLI Flags**
   - `--format [png|jpg|jpeg|ppm]` - Output format selection
   - `--jpeg-quality N` - Quality control (0-100, default 90)
   - Backward compatible: `--ppm` flag still works

4. **Updated Help Text**
   - Documented new flags
   - Marked `--ppm` as deprecated

5. **Added Variables**
   - `bool use_jpeg` (line 740)
   - `int jpeg_quality` (line 741)

### ❌ Incomplete

1. **Function Signature Updates**
   - All render functions need `use_jpeg` and `jpeg_quality` parameters
   - Affected functions:
     - `render_pages_bulk()` (line 502)
     - `render_pages_fast()` (line 503)
     - `render_pages_debug()` (line 504)
     - `render_pages_worker()` (line 505)
     - `render_page_to_png()` (line 515)
     - `ProcessBatch()` (line 490)

2. **Rendering Logic Updates**
   - Need to call `write_jpeg()` instead of PNG when `use_jpeg == true`
   - Location: render_page_to_png() function (~line 1950)
   - File extension changes: ".png" → ".jpg"

3. **Worker Communication**
   - Worker subprocess argument passing needs format info
   - Location: render_pages_fast() spawns workers (~line 2500)

4. **Build System**
   - Broken: expat dependency missing
   - Error: `Unable to load "//third_party/expat/BUILD.gn"`
   - Cannot compile to test changes

---

## Technical Implementation Details

### JPEG Writer Function

```cpp
bool write_jpeg(const char* filename, void* buffer, int stride,
                int width, int height, int quality) {
    // Validate inputs
    if (stride < 0 || width < 0 || height < 0) return false;
    if (quality < 0 || quality > 100) quality = 90;

    // Create SkPixmap from BGRA buffer (PDFium format)
    SkImageInfo info = SkImageInfo::Make(
        width, height,
        kBGRA_8888_SkColorType,  // PDFium uses BGRA
        kOpaque_SkAlphaType      // JPEG doesn't support alpha
    );
    SkPixmap pixmap(info, buffer, stride);

    // Configure encoder
    SkJpegEncoder::Options options;
    options.fQuality = quality;                           // User-specified quality
    options.fDownsample = SkJpegEncoder::Downsample::k420; // 4:2:0 chroma subsampling
    options.fAlphaOption = SkJpegEncoder::AlphaOption::kIgnore;

    // Encode to memory
    sk_sp<SkData> jpeg_data = SkJpegEncoder::Encode(nullptr, pixmap, options);
    if (!jpeg_data) {
        fprintf(stderr, "Error: Failed to encode JPEG data\n");
        return false;
    }

    // Write to file
    FILE* fp = fopen(filename, "wb");
    if (!fp) return false;

    size_t written = fwrite(jpeg_data->data(), 1, jpeg_data->size(), fp);
    fclose(fp);

    return (written == jpeg_data->size());
}
```

### Flag Parsing

```cpp
// In main(), line 808-846
else if (strcmp(argv[arg_idx], "--format") == 0) {
    arg_idx++;
    if (argc <= arg_idx) {
        fprintf(stderr, "Error: --format requires png|jpg|jpeg|ppm\n");
        return 1;
    }
    if (strcmp(argv[arg_idx], "jpg") == 0 || strcmp(argv[arg_idx], "jpeg") == 0) {
        use_jpeg = true;
        use_ppm = false;
    } else if (strcmp(argv[arg_idx], "png") == 0) {
        use_jpeg = false;
        use_ppm = false;
    } else if (strcmp(argv[arg_idx], "ppm") == 0) {
        use_jpeg = false;
        use_ppm = true;
    } else {
        fprintf(stderr, "Error: Invalid format\n");
        return 1;
    }
    arg_idx++;
}
```

---

## Remaining Work

### Phase 1: Fix Build System

**Priority**: CRITICAL
**Estimate**: 1 commit

The build is broken due to missing expat dependency. Options:

1. **Run gclient sync** (if available in environment)
2. **Manually add expat** to third_party/
3. **Disable expat dependency** in BUILD.gn (if not needed)

Without a working build, cannot test JPEG implementation.

### Phase 2: Thread Parameters Through Function Calls

**Priority**: HIGH
**Estimate**: 2-3 commits

Update function signatures to pass `use_jpeg` and `jpeg_quality`:

```cpp
// Current (using use_ppm as model)
int render_pages_bulk(..., bool use_ppm, bool use_raw, ...)

// Update to
int render_pages_bulk(..., bool use_ppm, bool use_jpeg, int jpeg_quality, bool use_raw, ...)
```

**Files to modify:**
1. Function declarations (lines 490-515)
2. Function definitions throughout file
3. All call sites (~15-20 locations)

### Phase 3: Update Rendering Logic

**Priority**: HIGH
**Estimate**: 1 commit

In `render_page_to_png()` (~line 1950):

```cpp
// Current
if (use_ppm) {
    write_ppm(filename, buffer, stride, width, height);
} else if (use_raw) {
    write_bgra(filename, buffer, stride, width, height);
} else {
    std::vector<uint8_t> png_data = image_diff_png::EncodeBGRAPNG(...);
    write_png(filename, png_data);
}

// Update to
if (use_jpeg) {
    write_jpeg(filename, buffer, stride, width, height, jpeg_quality);
} else if (use_ppm) {
    write_ppm(filename, buffer, stride, width, height);
} else if (use_raw) {
    write_bgra(filename, buffer, stride, width, height);
} else {
    std::vector<uint8_t> png_data = image_diff_png::EncodeBGRAPNG(...);
    write_png(filename, png_data);
}
```

Also update file extension logic:
```cpp
// Current
const char* ext = use_ppm ? ".ppm" : (use_raw ? ".bgra" : ".png");

// Update to
const char* ext = use_jpeg ? ".jpg" : (use_ppm ? ".ppm" : (use_raw ? ".bgra" : ".png"));
```

### Phase 4: Worker Subprocess Integration

**Priority**: MEDIUM
**Estimate**: 1 commit

Update worker argument passing in `render_pages_fast()`:

```cpp
// Current (line ~2507)
const char* format_str = use_raw ? "bgra" : (use_ppm ? "ppm" : "png");

// Update to
const char* format_str = use_jpeg ? "jpeg" :
                        (use_raw ? "bgra" : (use_ppm ? "ppm" : "png"));

// Worker receives format string and must parse it
// Also need to pass jpeg_quality as additional argument
```

### Phase 5: Testing

**Priority**: HIGH
**Estimate**: 1-2 commits

1. **Basic functionality test**
   ```bash
   pdfium_cli --format jpg render-pages test.pdf output/
   # Verify: .jpg files created, valid JPEG format
   ```

2. **Quality control test**
   ```bash
   pdfium_cli --format jpg --jpeg-quality 50 render-pages test.pdf output_low/
   pdfium_cli --format jpg --jpeg-quality 95 render-pages test.pdf output_high/
   # Verify: Different file sizes, visual quality difference
   ```

3. **Smoke tests**
   - Add JPEG format to test suite
   - Verify correctness (visual comparison)
   - Measure file sizes (should be 5-10x smaller than PNG)

4. **Integration tests**
   - Multi-threaded rendering with JPEG
   - Batch processing with JPEG
   - Smart mode (JPEG fast path) compatibility

---

## File Size Expectations

Based on typical PDF rendering:

| Format | Bytes/Pixel | 2550x3300 Page | Ratio vs PNG |
|--------|-------------|----------------|--------------|
| PNG    | ~2.5        | 21 MB         | 1.0x         |
| JPEG Q=90 | ~0.25    | 2.1 MB        | 10x smaller  |
| JPEG Q=75 | ~0.15    | 1.3 MB        | 16x smaller  |
| JPEG Q=50 | ~0.10    | 850 KB        | 25x smaller  |

**User's expected savings:**
- 169K PDFs, ~4.5 TB PNG → ~450 GB JPEG Q=90
- 10x storage reduction
- Faster uploads/downloads

---

## Testing Strategy

### Unit Tests

Add to `integration_tests/tests/test_001_smoke.py`:

```python
def test_jpeg_output():
    """Test JPEG format output"""
    result = subprocess.run([
        CLI_PATH, '--format', 'jpg',
        'render-pages', TEST_PDF, OUTPUT_DIR
    ], capture_output=True)

    assert result.returncode == 0
    jpeg_files = list(Path(OUTPUT_DIR).glob('*.jpg'))
    assert len(jpeg_files) > 0

    # Verify valid JPEG
    from PIL import Image
    img = Image.open(jpeg_files[0])
    assert img.format == 'JPEG'

def test_jpeg_quality():
    """Test JPEG quality parameter"""
    # Low quality
    subprocess.run([CLI_PATH, '--format', 'jpg', '--jpeg-quality', '50',
                    'render-pages', TEST_PDF, 'output_q50/'])
    size_q50 = Path('output_q50/page_0.jpg').stat().st_size

    # High quality
    subprocess.run([CLI_PATH, '--format', 'jpg', '--jpeg-quality', '95',
                    'render-pages', TEST_PDF, 'output_q95/'])
    size_q95 = Path('output_q95/page_0.jpg').stat().st_size

    # Higher quality should be larger
    assert size_q95 > size_q50
```

### Manual Verification

```bash
# Test basic JPEG output
out/Release/pdfium_cli --format jpg render-pages \
    integration_tests/pdfs/simple.pdf test_output/

# Verify files
ls -lh test_output/
file test_output/*.jpg
open test_output/page_0.jpg  # Visual check

# Test quality control
out/Release/pdfium_cli --format jpg --jpeg-quality 50 render-pages \
    integration_tests/pdfs/simple.pdf test_q50/

out/Release/pdfium_cli --format jpg --jpeg-quality 95 render-pages \
    integration_tests/pdfs/simple.pdf test_q95/

# Compare file sizes
du -sh test_q50/ test_q95/

# Test multi-threading
out/Release/pdfium_cli --workers 4 --format jpg render-pages \
    integration_tests/pdfs/simple.pdf test_workers/
```

---

## Risks and Considerations

### 1. Build System Fragility

The expat dependency issue suggests the build might be fragile. Need to:
- Document exact build procedure
- Verify all dependencies present
- Test build from clean state

### 2. Color Space Handling

PDFium uses BGRA format internally, SkJpegEncoder expects proper color space info:
- BGRA_8888 color type is correct for PDFium bitmaps
- Alpha channel is discarded (JPEG doesn't support transparency)
- No color space conversion needed (assumes sRGB)

### 3. Performance Impact

JPEG encoding is more expensive than PNG:
- PNG: ~50 MB/s (zlib compression)
- JPEG: ~100-200 MB/s (libjpeg-turbo)
- Net benefit: I/O savings outweigh CPU cost
- For 300 DPI page: PNG=21MB, JPEG=2MB → 10x less I/O

### 4. Quality Defaults

Chose quality=90 as default:
- libjpeg-turbo default is 75 (too low for document images)
- Quality 90 provides good balance (visually lossless, 10x compression)
- Quality 95+ shows diminishing returns (minimal quality gain, 2x larger)
- Users can override with `--jpeg-quality`

### 5. Smart Mode Interaction

Smart mode (JPEG fast path) already extracts embedded JPEGs:
- This adds JPEG *encoding* for rendered pages
- Both features should work together
- Smart mode extracts original JPEG (no re-encoding)
- New feature encodes rendered bitmaps to JPEG

---

## Lessons Learned

### 1. Skia Integration is Clean

Using Skia's JPEG encoder (already in build) is much cleaner than:
- Directly using libjpeg-turbo (low-level API)
- Adding new dependencies
- Writing custom JPEG encoding

### 2. Backward Compatibility Matters

Kept `--ppm` flag working while adding `--format`:
- Existing scripts won't break
- Clear migration path (deprecation warning)
- Consistent with Unix philosophy (accept old, emit new)

### 3. Build System is Fragile

The build broke before I could test:
- Should have tested incremental build first
- Need better dependency management
- Consider isolating changes to test compilation

### 4. Scope Management

JPEG implementation touches many functions:
- 6 function signatures
- 15-20 call sites
- Multiple rendering paths
- Worker subprocess communication

Should have estimated scope before starting.

---

## Next AI Instructions

### Immediate Tasks (Priority Order)

1. **Fix Build System** (CRITICAL)
   ```bash
   # Try these in order:
   cd pdfium && git status  # Check if expat is a submodule
   ls -la third_party/expat  # Verify if directory exists
   # If missing, may need to manually add or disable in BUILD.gn
   ```

2. **Thread JPEG Parameters** (HIGH)
   - Update all 6 function signatures (see "Remaining Work" section)
   - Update all call sites (~15-20 locations)
   - Use grep to find all: `grep -n "render_pages_bulk\|render_pages_fast" examples/pdfium_cli.cpp`

3. **Update Rendering Logic** (HIGH)
   - Modify render_page_to_png() to call write_jpeg()
   - Update file extension logic
   - Handle JPEG-specific error cases

4. **Compile and Test** (CRITICAL)
   ```bash
   ninja -C out/Release pdfium_cli
   ./out/Release/pdfium_cli --format jpg render-pages test.pdf output/
   ls -lh output/  # Verify .jpg files
   ```

5. **Run Test Suite** (HIGH)
   ```bash
   cd integration_tests
   python3 -m pytest -m smoke -v
   # Should still pass (no behavior change for default PNG)
   ```

6. **Add JPEG Tests** (MEDIUM)
   - Add test_jpeg_output() to test suite
   - Add test_jpeg_quality()
   - Verify file sizes are ~10x smaller

### If Build Cannot Be Fixed

If expat issue persists, options:

1. **Revert Skia JPEG Encoder**
   - Use libjpeg-turbo directly
   - More code but fewer dependencies

2. **Wait for User**
   - Document issue in commit
   - Let user fix build environment

3. **Alternative Implementation**
   - Use stb_image_write.h (single-file library)
   - Already used in some Chromium projects

---

## Git Commit Message (When Complete)

```
[WORKER0] # 15: JPEG Output Format Implementation

**Status**: INCOMPLETE - Build broken, core logic ready

## Problem

User blocked on storage (PR #18): 169K PDFs → 4.5 TB PNG output.
Needed JPEG format to reduce to 450 GB (10x savings).

## Changes

### Completed
1. Added write_jpeg() using Skia's SkJpegEncoder
   - Quality control (0-100, default 90)
   - 4:2:0 chroma subsampling
   - BGRA→JPEG conversion
2. Added CLI flags:
   - --format [png|jpg|jpeg|ppm]
   - --jpeg-quality N
3. Updated help text
4. Added use_jpeg and jpeg_quality variables

### Incomplete
1. Function signatures not updated (6 functions)
2. Rendering logic not integrated (render_page_to_png)
3. Worker subprocess communication not updated
4. Build system broken (expat dependency missing)
5. Not tested (cannot compile)

## Build Issue

Error: Unable to load "//third_party/expat/BUILD.gn"
Cannot compile to verify changes.

## Next AI

1. Fix build (check expat dependency)
2. Thread use_jpeg/jpeg_quality through render functions
3. Update render_page_to_png() to call write_jpeg()
4. Test: --format jpg should create .jpg files
5. Verify file sizes ~10x smaller than PNG

See N15_JPEG_OUTPUT_IMPLEMENTATION_STATUS.md for complete details.
```

---

## Files Modified

- `examples/pdfium_cli.cpp`:
  - Lines 49-54: Added Skia JPEG encoder includes
  - Line 512: Added write_jpeg() declaration
  - Lines 740-741: Added use_jpeg and jpeg_quality variables
  - Lines 808-846: Added --format and --jpeg-quality flag parsing
  - Lines 561-563: Updated help text
  - Lines 1730-1778: Implemented write_jpeg() function

---

## References

- PR #18: JPEG extraction UX issue
- USER_FEEDBACK_JPEG_EXTRACTION_UX.md: User problem statement
- N=14 commit: Recommended Path B (JPEG + UTF-8)
- Skia JPEG Encoder: pdfium/third_party/skia/include/encode/SkJpegEncoder.h

---

**Report Date**: 2025-11-20 23:30 PST
**Worker**: WORKER0
**Context Usage**: ~57K / 1M tokens
