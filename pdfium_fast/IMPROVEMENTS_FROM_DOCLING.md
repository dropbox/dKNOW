# Improvements Needed from docling_rs Integration

**Source:** Integration work in `~/docling_rs/`
**Purpose:** Track API gaps and feature requests discovered during docling_rs integration

---

## Priority 1: High-Level Rust API

### Issue
pdfium-sys provides raw FFI bindings only. docling_rs uses `pdfium-render` which has a high-level Rust API. Porting requires manually wrapping every FFI call.

### Suggestion
Create `pdfium-render-fast` crate that provides `pdfium-render`-compatible API:

```rust
// Goal: drop-in replacement for pdfium-render
use pdfium_render_fast::prelude::*;  // Same API as pdfium-render

let pdfium = Pdfium::new(...)?;
let doc = pdfium.load_pdf_from_file(path, None)?;
for page in doc.pages() {
    let bitmap = page.render_with_config(&config)?;
    let text = page.text()?;
}
```

### Benefit
- docling_rs integration becomes trivial (swap one dependency)
- Other pdfium-render users can benefit from 72x speedup
- Proper Rust ownership/lifetime management

---

## Priority 2: Missing FFI Bindings

These functions are used by docling_rs but may need verification in pdfium-sys:

| Function | Used For | Status |
|----------|----------|--------|
| `FPDF_GetMetaText` | Document metadata | Verify |
| `FPDFText_GetFontSize` | Font size extraction | Verify |
| `FPDFText_GetCharAngle` | Text rotation | Verify |
| `FPDF_GetPageLabel` | Page labels | Verify |

---

## Priority 3: Text Extraction Improvements

### Current
Character-by-character extraction via `FPDFText_GetUnicode` + `FPDFText_GetCharBox`

### Suggestion
Add batch text extraction API:
```rust
// Get all text segments with bounds in one call
fn extract_text_segments(page: FPDF_PAGE) -> Vec<TextSegment>;
```

### Benefit
- Reduce FFI call overhead
- Match pdfium-render's `page.text().segments()` API

---

---

## Priority 4: Batch Parallel Render API (✅ IMPLEMENTED - 2024-12-22)

### Status: ✅ COMPLETE

**Implementation:** Native C++ multi-threading via `FPDF_RenderPagesParallelV2()`

**Performance Results (14-page PDF at 150 DPI):**
- Sequential rendering: 290ms (48.2 pages/sec)
- Parallel rendering (4 threads): 74ms (189.8 pages/sec)
- **Speedup: 3.94x faster**

### What We Discovered

The initial attempt with Rayon failed because we tried to call `FPDF_InitLibrary()` multiple times:
```rust
// WRONG: Tried to create new pdfium instances per thread
let pdfium = PdfiumFast::new()?;  // Calls FPDF_InitLibrary() - SIGSEGV!
```

**Solution:** pdfium_fast already HAS native C++ multi-threading in `fpdf_parallel.h`:
- `FPDF_RenderPagesParallelV2()` - parallel render with bitmap pooling
- `FPDF_GetOptimalWorkerCountForDocument()` - auto-detect thread count
- `FPDF_DestroyThreadPool()` - cleanup

The API was there all along - we just weren't exposing it in the Rust bindings!

### Fixed in pdfium-sys

Added to `build.rs`:
```rust
.header(pdfium_root.join("public/fpdf_parallel.h"))
```

### Rust API

```rust
impl PdfDocumentFast {
    /// Get optimal thread count for this document
    pub fn optimal_thread_count(&self) -> i32;

    /// Render all pages in parallel using native C++ thread pool
    pub fn render_pages_parallel(
        &self,
        dpi: f64,
        thread_count: i32,
    ) -> Result<Vec<RenderedPage>, DoclingError>;
}
```

### Integration in docling_rs

The ML pipeline (`pdf_fast.rs`) now pre-renders ALL pages in parallel before ML processing:
```rust
// Parallel render ALL pages FIRST using pdfium_fast's native C++ thread pool
let rendered_pages = pdf_doc.render_pages_parallel(300.0, optimal_threads)?;

// Then process each page through ML sequentially
for page_idx in 0..num_pages {
    let page_image = &rendered_pages[page_idx];
    // ... ML inference ...
}
```

---

## Priority 5: Scanned PDF Detection

### Issue
Scanned PDFs are 30%+ of enterprise documents. The JPEG fast path (545x speedup) is CLI-only.

### Suggestion

```rust
impl PdfPage {
    /// Check if page is single JPEG image (scanned document)
    fn is_scanned_jpeg(&self) -> bool;

    /// If scanned, extract JPEG directly (545x faster)
    fn extract_jpeg_fast(&self) -> Option<Vec<u8>>;
}
```

---

## Priority 6: ML-Optimized DPI

### Issue
300 DPI is overkill for ML. Layout models resize to 640x640 anyway.
- 300 DPI: 2480x3508 pixels (wasteful)
- 150 DPI: 1240x1754 pixels (still overkill)
- ML-optimal: ~100 DPI or target 1000px height

### Suggestion
Add ML-friendly render options that target output dimensions rather than DPI.

---

---

## 🚨 CRITICAL: Missing Header in Rust Bindings (2024-12-23)

### Issue

The file `public/fpdf_searchex.h` exists with useful APIs but is NOT included in Rust bindings:

```c
// In fpdf_searchex.h - THESE EXIST BUT ARE NOT EXPOSED
int FPDFText_GetCharIndexFromTextIndex(FPDF_TEXTPAGE text_page, int nTextIndex);
int FPDFText_GetTextIndexFromCharIndex(FPDF_TEXTPAGE text_page, int nCharIndex);
```

These APIs map between:
- **Text index**: Position in string returned by `FPDFText_GetText()`
- **Character index**: Position used by `FPDFText_GetCharBox()`, `FPDFText_GetFontSize()`, etc.

Essential for implementing text selection in UI applications.

### Fix (5 minutes)

Edit `rust/pdfium-sys/build.rs`, add after line 91:

```rust
// Extended search APIs (text index mapping)
.header(pdfium_root.join("public/fpdf_searchex.h").to_str().unwrap())
```

### Verification

```bash
cd ~/pdfium_fast/rust/pdfium-sys
cargo build
grep "GetCharIndexFromTextIndex" target/*/build/pdfium-sys-*/out/bindings.rs
```

---

## Priority 7: Batch Text Extraction API (NEW)

### Problem

Current workflow per page:
1. `FPDFText_CountRects()` - count text rectangles
2. For each rect: `FPDFText_GetRect()` + `FPDFText_GetBoundedText()`
3. **N×2 FFI calls per page** (N = number of text cells, typically 50-200)

### Proposed API

Add to `public/fpdf_text_batch.h`:

```c
typedef struct {
    float left, top, right, bottom;  // Bounding box (top-left origin after conversion)
    int text_start;                   // Offset into text buffer
    int text_length;                  // Length in UTF-16 code units
} FPDF_TEXT_CELL;

// Extract all text cells from a page in one call
// Returns: Number of cells extracted, or -1 on error
FPDF_EXPORT int FPDF_CALLCONV
FPDFText_GetAllTextCells(FPDF_TEXTPAGE text_page,
                         void* text_buffer,        // UTF-16 output
                         int text_buffer_size,     // Size in bytes
                         FPDF_TEXT_CELL* cells,    // Output array
                         int max_cells,
                         double page_height);      // For coordinate conversion
```

### Impact

Would reduce FFI overhead from O(N) to O(1) per page. Estimated 10-20% speedup.

### Priority

MEDIUM - Current performance (6.5 pages/sec) is acceptable.

---

## Priority 8: SIMD Color Conversion Option (NEW)

### Problem

`FPDF_RenderPagesParallelV2` callback returns BGRA. Callers convert to RGB:

```rust
// Rust side - not SIMD optimized
for pixel in pixels.chunks_mut(4) {
    let (b, g, r) = (pixel[0], pixel[1], pixel[2]);
    // Swap to RGB...
}
```

### Proposed Solution

Add output format option to parallel render:

```c
typedef struct {
    int worker_count;
    int max_queue_size;
    void* form_handle;
    double dpi;
    int output_format;  // NEW: 0=BGRA (default), 1=RGB, 2=BGR
    void* reserved[1];
} FPDF_PARALLEL_OPTIONS;
```

Library performs BGRA→RGB conversion with SIMD (SSE4/AVX/NEON).

### Impact

Estimated 5-10% speedup by eliminating Rust-side conversion.

### Priority

LOW - Current performance is acceptable.

---

## Log

| Date | Issue | Reporter |
|------|-------|----------|
| 2025-12-17 | Initial integration analysis | MANAGER |
| 2025-12-17 | Missing type equivalents identified | N=2926 |
| 2025-12-22 | Batch parallel render API request | N=3017 |
| 2025-12-22 | Scanned PDF detection API request | N=3017 |
| 2025-12-22 | ML-optimized DPI suggestion | N=3017 |
| 2025-12-22 | Rayon parallel rendering attempt - FPDF_InitLibrary not thread-safe | N=3018 |
| 2025-12-22 | Updated with multi-process API suggestion | N=3018 |
| 2025-12-23 | **CRITICAL: fpdf_searchex.h missing from Rust bindings** | MANAGER |
| 2025-12-23 | Batch text extraction API proposal | MANAGER |
| 2025-12-23 | SIMD color conversion option proposal | MANAGER |
