# Ambitious Wishlist for pdfium_fast from docling_rs

**Date:** 2024-12-23
**From:** docling_rs comprehensive audit
**Purpose:** Features that would make docling_rs world-class

---

## Executive Summary

Current state: **6.5 pages/sec** with existing pdfium_fast APIs.

With these improvements, targeting: **15-20 pages/sec** (2-3x speedup)

| Priority | Feature | Impact | Effort |
|----------|---------|--------|--------|
| P0 | fpdf_searchex.h binding | HIGH | 5 min |
| P1 | Batch text extraction API | HIGH | 4 hours |
| P2 | Tagged PDF (structure tree) support | HIGH | 8 hours |
| P3 | Direct image extraction API | MEDIUM | 4 hours |
| P4 | RGB output mode for parallel render | MEDIUM | 2 hours |
| P5 | Memory-mapped file loading | MEDIUM | 4 hours |
| P6 | Parallel text extraction | MEDIUM | 8 hours |
| P7 | Thumbnail extraction | LOW | 1 hour |

---

## P0: CRITICAL - fpdf_searchex.h Binding (5 minutes)

**Status:** API exists, just not exposed in Rust bindings.

See `IMPROVEMENTS_FROM_DOCLING.md` for details.

---

## P1: Batch Text Extraction API (HIGH IMPACT)

### Problem

Current per-page text extraction flow:
```
FPDFText_LoadPage()           // 1 call
FPDFText_CountRects()         // 1 call
for each rect (N = 50-200):
    FPDFText_GetRect()        // N calls
    FPDFText_GetBoundedText() // N calls
FPDFText_ClosePage()          // 1 call
```

**Total: 2N + 3 FFI calls per page** (typically 100-400 calls)

### Proposed API

New header: `public/fpdf_text_batch.h`

```c
// Text cell with bounds and content
typedef struct {
    float left;        // Left edge (PDF points, top-left origin)
    float top;         // Top edge
    float right;       // Right edge
    float bottom;      // Bottom edge
    int text_offset;   // Offset into output text buffer (in chars)
    int text_length;   // Length of text (in chars)
    float font_size;   // Font size in points
    int font_flags;    // Bold/italic/etc flags
} FPDF_TEXT_CELL_INFO;

// Extract ALL text cells from a page in ONE call
// Returns: Number of cells, or -1 on error
//
// Usage:
//   1. Call with cells=NULL to get count
//   2. Allocate buffers
//   3. Call again to fill buffers
//
FPDF_EXPORT int FPDF_CALLCONV
FPDFText_ExtractAllCells(
    FPDF_TEXTPAGE text_page,
    double page_height,           // For coordinate conversion
    FPDF_TEXT_CELL_INFO* cells,   // Output array (NULL to query count)
    int max_cells,                // Size of cells array
    unsigned short* text_buffer,  // UTF-16LE output (NULL to query size)
    int text_buffer_chars         // Size of text buffer in chars
);

// Get required buffer sizes without extraction
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFText_GetAllCellsBufferSizes(
    FPDF_TEXTPAGE text_page,
    int* out_cell_count,          // Number of cells
    int* out_text_chars           // Total text length in chars
);
```

### Implementation Strategy

Internally iterate once through PDFium's text objects, building the complete result. No external iteration needed.

### Expected Impact

- **Before:** 100-400 FFI calls per page
- **After:** 2-3 FFI calls per page
- **Speedup:** 10-20% overall (FFI overhead is ~15% of total time)

---

## P2: Tagged PDF / Structure Tree Support (HIGH IMPACT)

### Why This Matters

Tagged PDFs contain semantic structure:
- Headings (H1, H2, etc.)
- Paragraphs
- Lists
- Tables (with headers, rows, cells)
- Figures with alt text

This is **FREE** semantic information that most PDFs from Word/Google Docs have!

Currently docling_rs uses ML models to detect this structure. If the PDF is tagged, we could skip ML entirely for structure detection.

### Current API (fpdf_structtree.h - already in pdfium)

```c
FPDF_StructTree_GetForPage(page)     // Get structure tree
FPDF_StructElement_GetType(elem)      // Get "H1", "P", "Table", etc.
FPDF_StructElement_GetAltText(elem)   // Get alt text for images
FPDF_StructElement_CountChildren()    // Navigate tree
```

### Proposed High-Level API

New header: `public/fpdf_structtree_batch.h`

```c
// Flattened structure element for batch extraction
typedef struct {
    int element_id;         // Unique ID
    int parent_id;          // Parent element ID (-1 for root)
    int type;               // Element type enum
    float bbox_left, bbox_top, bbox_right, bbox_bottom;
    int text_offset;        // Offset into text buffer
    int text_length;        // Length of text content
    int alt_text_offset;    // Offset into alt text buffer
    int alt_text_length;    // Length of alt text
} FPDF_STRUCT_ELEMENT_INFO;

// Element types
#define FPDF_STRUCT_DOCUMENT 0
#define FPDF_STRUCT_PART     1
#define FPDF_STRUCT_H1       10
#define FPDF_STRUCT_H2       11
#define FPDF_STRUCT_H3       12
#define FPDF_STRUCT_P        20
#define FPDF_STRUCT_L        30  // List
#define FPDF_STRUCT_LI       31  // List item
#define FPDF_STRUCT_TABLE    40
#define FPDF_STRUCT_TR       41  // Table row
#define FPDF_STRUCT_TH       42  // Table header cell
#define FPDF_STRUCT_TD       43  // Table data cell
#define FPDF_STRUCT_FIGURE   50
// ... etc

// Check if document has structure tree (quick check)
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFDoc_HasStructTree(FPDF_DOCUMENT document);

// Extract flattened structure tree for a page
FPDF_EXPORT int FPDF_CALLCONV
FPDFPage_ExtractStructure(
    FPDF_PAGE page,
    FPDF_STRUCT_ELEMENT_INFO* elements,  // Output array
    int max_elements,
    unsigned short* text_buffer,         // Element text content
    int text_buffer_chars,
    unsigned short* alt_text_buffer,     // Alt text for figures
    int alt_text_buffer_chars
);
```

### Expected Impact

For tagged PDFs (estimated 40% of enterprise documents):
- **Skip ML layout detection** - structure is known
- **Skip ML table detection** - table structure is explicit
- **Better figure handling** - alt text available

**Speedup for tagged PDFs:** 2-5x (skip most ML inference)

---

## P3: Direct Image Extraction API (MEDIUM IMPACT)

### Problem

Scanned PDFs often contain embedded JPEG/PNG that are already compressed. Current flow:

1. PDFium decompresses JPEG to bitmap
2. We copy bitmap to Rust
3. Rust re-compresses to send to OCR

This is wasteful - we should extract the raw JPEG stream directly.

### Current Partial Support

```c
// These exist but don't give raw stream
FPDFImageObj_GetBitmap()      // Returns decompressed bitmap
FPDFImageObj_GetImageDataRaw() // Returns... something
```

### Proposed API

```c
// Image compression format
typedef enum {
    FPDF_IMAGE_UNKNOWN = 0,
    FPDF_IMAGE_JPEG = 1,
    FPDF_IMAGE_JPEG2000 = 2,
    FPDF_IMAGE_PNG = 3,      // Actually "FlateDecode with PNG predictor"
    FPDF_IMAGE_CCITT = 4,    // Fax compression (older scanners)
    FPDF_IMAGE_RAW = 5       // Uncompressed
} FPDF_IMAGE_FORMAT;

// Get image info without decompressing
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFImageObj_GetInfo(
    FPDF_PAGEOBJECT image_object,
    int* out_width,
    int* out_height,
    int* out_bits_per_component,
    FPDF_IMAGE_FORMAT* out_format,
    unsigned long* out_data_size
);

// Extract raw compressed stream (if JPEG, returns actual JPEG file)
FPDF_EXPORT unsigned long FPDF_CALLCONV
FPDFImageObj_GetCompressedData(
    FPDF_PAGEOBJECT image_object,
    void* buffer,
    unsigned long buflen
);
```

### Expected Impact

For scanned PDFs (JPEG embedded):
- **Skip decompression** on PDF side
- **Skip re-compression** on Rust side
- **Speedup:** 20-40% for image-heavy pages

---

## P4: RGB Output Mode for Parallel Render (MEDIUM IMPACT)

### Problem

`FPDF_RenderPagesParallelV2` returns BGRA. Rust side converts to RGB:

```rust
// Current: ~5% of render time
for i in 0..pixel_count {
    let b = bgra[i*4];
    let g = bgra[i*4 + 1];
    let r = bgra[i*4 + 2];
    rgb[i*3] = r;
    rgb[i*3 + 1] = g;
    rgb[i*3 + 2] = b;
}
```

### Proposed Solution

Add to `FPDF_PARALLEL_OPTIONS`:

```c
typedef struct {
    int worker_count;
    int max_queue_size;
    void* form_handle;
    double dpi;

    // NEW: Output format
    // 0 = BGRA (default, 4 bytes/pixel)
    // 1 = RGB (3 bytes/pixel, converted internally with SIMD)
    // 2 = BGR (3 bytes/pixel)
    // 3 = Grayscale (1 byte/pixel, for ML that doesn't need color)
    int output_format;

    void* reserved[1];
} FPDF_PARALLEL_OPTIONS;
```

### Implementation

Use SIMD intrinsics (SSE4/AVX on x86, NEON on ARM) for conversion. PDFium already has these available.

### Expected Impact

- Eliminate Rust-side conversion loop
- Grayscale mode reduces memory by 3x and bandwidth by 3x
- **Speedup:** 5-10%

---

## P5: Memory-Mapped File Loading (MEDIUM IMPACT)

### Problem

Current loading:
```c
FPDF_LoadMemDocument(data, size, password)  // Copies entire file into PDFium
```

For large PDFs (100MB+), this doubles memory usage.

### Proposed API

```c
// Load document from memory-mapped file (zero-copy)
// The caller must keep the mapping valid for document lifetime
FPDF_EXPORT FPDF_DOCUMENT FPDF_CALLCONV
FPDF_LoadMemMappedDocument(
    const void* mapped_data,     // Memory-mapped file pointer
    size_t size,                 // File size
    FPDF_BYTESTRING password,
    FPDF_BOOL keep_mapped        // If true, PDFium won't copy data
);
```

### Expected Impact

- **Memory reduction:** 50% for large files
- **Load time reduction:** Skip copy step
- Especially useful for batch processing many PDFs

---

## P6: Parallel Text Extraction (MEDIUM IMPACT)

### Problem

Current parallel API only handles rendering. Text extraction is still sequential:

```
Page 0: render (parallel) → text extract (sequential) → ML (sequential)
Page 1: render (parallel) → text extract (sequential) → ML (sequential)
...
```

### Proposed API

```c
// Callback for text extraction completion
typedef void (*FPDF_TEXT_CALLBACK)(
    int page_index,
    FPDF_TEXT_CELL_INFO* cells,
    int cell_count,
    const unsigned short* text,
    int text_chars,
    void* user_data,
    FPDF_BOOL success
);

// Extract text from multiple pages in parallel
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDF_ExtractTextParallel(
    FPDF_DOCUMENT document,
    int start_page,
    int page_count,
    double* page_heights,         // Array of page heights for coord conversion
    FPDF_PARALLEL_OPTIONS* options,
    FPDF_TEXT_CALLBACK callback,
    void* user_data
);
```

### Expected Impact

Text extraction is ~15% of total time. Parallelizing would reduce overall time by ~10%.

---

## P7: Thumbnail Extraction (LOW IMPACT)

### Current API (fpdf_thumbnail.h - already exists)

```c
FPDFPage_GetDecodedThumbnailData()  // Get embedded thumbnail
FPDFPage_GetThumbnailAsBitmap()     // Get as bitmap
```

### Missing in Rust Bindings

Add `fpdf_thumbnail.h` to build.rs.

### Use Case

Fast preview generation without full page render. Useful for:
- Document browser UI
- Quick document classification
- Duplicate detection

---

## P8: Font Information API (LOW IMPACT, NICE TO HAVE)

### Problem

Can't get font name/style for text. This affects:
- Code detection (monospace fonts)
- Heading detection (bold/larger fonts)
- Document styling preservation

### Proposed API

```c
typedef struct {
    char name[256];          // Font name
    int flags;               // FPDF_FONT_BOLD | FPDF_FONT_ITALIC | etc.
    int type;                // Type1, TrueType, etc.
    FPDF_BOOL is_embedded;   // Is font embedded in PDF
    FPDF_BOOL is_monospace;  // Is monospace (code font)
} FPDF_FONT_INFO;

FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFText_GetFontInfo(
    FPDF_TEXTPAGE text_page,
    int char_index,
    FPDF_FONT_INFO* out_info
);
```

---

## P9: Word Boundary Detection (NICE TO HAVE)

### Problem

PDFium gives character-level bounding boxes. Grouping into words requires heuristics in Rust.

### Proposed API

```c
// Word with bounds
typedef struct {
    float left, top, right, bottom;
    int start_char;      // Start character index
    int end_char;        // End character index (exclusive)
} FPDF_WORD_INFO;

// Extract words (uses PDFium's internal word detection)
FPDF_EXPORT int FPDF_CALLCONV
FPDFText_ExtractWords(
    FPDF_TEXTPAGE text_page,
    double page_height,
    FPDF_WORD_INFO* words,
    int max_words
);
```

---

## Summary: Expected Impact

If ALL features implemented:

| Scenario | Current | With All Features | Speedup |
|----------|---------|-------------------|---------|
| Digital PDF | 6.5 pg/s | 10 pg/s | 1.5x |
| Tagged PDF | 6.5 pg/s | 20 pg/s | 3x |
| Scanned PDF (JPEG) | 4 pg/s | 8 pg/s | 2x |
| Large PDF (100MB+) | Memory issues | Smooth | ∞ |

**Recommended implementation order:**
1. P0: fpdf_searchex.h (5 min - CRITICAL)
2. P4: RGB output mode (2 hours - easy win)
3. P1: Batch text extraction (4 hours - biggest impact)
4. P2: Tagged PDF support (8 hours - massive win for enterprise)
5. P3: Direct image extraction (4 hours - helps scanned docs)
