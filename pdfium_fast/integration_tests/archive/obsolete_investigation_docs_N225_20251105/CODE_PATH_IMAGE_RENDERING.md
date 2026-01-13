# Image Rendering Code Path Trace

**Date**: 2025-10-31
**Purpose**: Complete code path documentation for image rendering
**Audience**: Future AIs, developers debugging rendering issues

---

## Overview

Image rendering in PDFium converts PDF pages to raster images (PNG/JPEG) using:
- `FPDF_RenderPageBitmap()` - Core rendering function
- `FPDFBitmap_*` APIs - Bitmap management
- PNG encoding - Output format conversion

**Rendering Pipeline**:
```
PDF Page → Load Page → Create Bitmap → Render to Bitmap → Convert BGRA→RGBA → Encode PNG → Write File
```

---

## Entry Point: render_pages.rs

**Location**: `rust/pdfium-sys/examples/render_pages.rs`

**Dispatcher Logic** (lines 25-113):
```
main()
  ├─→ Check args[1] == "--worker" → worker_main()  [Multi-process worker]
  │
  ├─→ Parse arguments: pdf_path, output_dir, [worker_count], [dpi]
  │
  ├─→ get_page_count(pdf_path)
  │     └─→ Returns page count for strategy selection
  │
  ├─→ Determine worker_count:
  │     ├─→ Explicit (from args[3]) → use specified count
  │     └─→ Auto-select:
  │           ├─→ page_count < 200 → workers = 1 (single-threaded)
  │           └─→ page_count ≥ 200 → workers = 4 (multi-process)
  │
  ├─→ Parse DPI (default: 300.0)
  │
  ├─→ Create output directory
  │
  └─→ Route to implementation:
        ├─→ worker_count == 1 → render_single_threaded()
        └─→ worker_count > 1  → render_multiprocess()
```

**Key Decision**: PAGE_THRESHOLD = 200
- **< 200 pages**: Single-threaded (process overhead ~100ms negates gain)
- **≥ 200 pages**: Multi-process (3.34x speedup at 4 workers)

**DPI Default**: 300.0
- Higher DPI = larger images, more detail
- 300 DPI: Standard print quality
- 72 DPI: Screen quality (smaller, faster)

---

## Code Path 1: Single-Threaded Rendering

**Function**: `render_single_threaded()` (lines 142-174)

### Step-by-Step Execution

```
render_single_threaded(pdf_path, output_dir, dpi)
  │
  ├─→ FPDF_InitLibrary()
  │     └─→ Initialize PDFium library (global state)
  │
  ├─→ FPDF_LoadDocument(pdf_path, password=NULL)
  │     └─→ Load PDF document
  │     └─→ Returns FPDF_DOCUMENT handle (or NULL on error)
  │
  ├─→ FPDF_GetPageCount(doc)
  │     └─→ Get total page count
  │
  └─→ FOR EACH PAGE (page_index = 0..page_count-1):
        │
        ├─→ render_page_to_png(doc, page_index, output_dir, dpi)
        │     └─→ [See detailed breakdown below]
        │     └─→ On error: Log warning, continue to next page
        │
        └─→ Continue with all pages

  ├─→ FPDF_CloseDocument(doc)
  │     └─→ Release document memory
  │
  └─→ FPDF_DestroyLibrary()
        └─→ Cleanup PDFium global state
```

### Detailed: render_page_to_png()

**Function**: `render_page_to_png()` (lines 176-256)

```
render_page_to_png(doc, page_index, output_dir, dpi)
  │
  ├─→ FPDF_LoadPage(doc, page_index)
  │     └─→ Load page structure
  │     └─→ Returns FPDF_PAGE handle
  │     └─→ NULL on error (encrypted, corrupt, etc.)
  │
  ├─→ Get page dimensions (in points, 1 point = 1/72 inch):
  │     ├─→ FPDF_GetPageWidthF(page) → width_pts
  │     └─→ FPDF_GetPageHeightF(page) → height_pts
  │
  ├─→ Calculate pixel dimensions:
  │     ├─→ scale = dpi / 72.0
  │     │     └─→ Example: 300 DPI → scale = 4.166
  │     ├─→ width_px = width_pts × scale
  │     └─→ height_px = height_pts × scale
  │           └─→ Example: 8.5" × 11" at 300 DPI = 2550 × 3300 pixels
  │
  ├─→ FPDFBitmap_Create(width_px, height_px, alpha=0)
  │     └─→ Create in-memory bitmap
  │     └─→ Format: BGRA (4 bytes per pixel)
  │     └─→ alpha=0: No alpha channel (opaque background)
  │     └─→ Returns FPDF_BITMAP handle (or NULL if out of memory)
  │
  ├─→ FPDFBitmap_FillRect(bitmap, 0, 0, width_px, height_px, 0xFFFFFFFF)
  │     └─→ Fill entire bitmap with white (0xFFFFFFFF = opaque white)
  │     └─→ Ensures background is white (not transparent/black)
  │
  ├─→ FPDF_RenderPageBitmap(
  │         bitmap,            // Destination bitmap
  │         page,              // Source page
  │         0, 0,              // Start position (x, y)
  │         width_px,          // Width to render
  │         height_px,         // Height to render
  │         rotate=0,          // No rotation
  │         flags=FPDF_ANNOT   // Include annotations
  │     )
  │     └─→ **CORE RENDERING**: Rasterizes PDF page to bitmap
  │     └─→ Renders:
  │           ├─→ Vector graphics (paths, shapes)
  │           ├─→ Text (with fonts, colors)
  │           ├─→ Images (embedded bitmaps)
  │           ├─→ Annotations (if FPDF_ANNOT flag set)
  │           └─→ Forms (if FPDF_FORMS flag set)
  │
  ├─→ Get bitmap data:
  │     ├─→ FPDFBitmap_GetBuffer(bitmap) → buffer pointer
  │     │     └─→ Returns *const u8 (raw pixel data)
  │     └─→ FPDFBitmap_GetStride(bitmap) → stride
  │           └─→ Bytes per row (may include padding)
  │           └─→ stride ≥ width_px × 4 (BGRA)
  │
  ├─→ Convert BGRA → RGBA (lines 218-233):
  │     └─→ FOR EACH ROW (y = 0..height_px-1):
  │           └─→ FOR EACH PIXEL (x = 0..width_px-1):
  │                 ├─→ pixel_offset = y × stride + x × 4
  │                 ├─→ Read BGRA: [B, G, R, A]
  │                 └─→ Write RGBA: [R, G, B, A]
  │     └─→ Why: PNG standard is RGBA, PDFium outputs BGRA
  │
  ├─→ Encode PNG (lines 236-249):
  │     ├─→ Create PNG encoder:
  │     │     ├─→ Width, height from bitmap
  │     │     ├─→ ColorType: RGBA
  │     │     └─→ BitDepth: 8 bits per channel
  │     │
  │     ├─→ Write PNG header
  │     ├─→ Write image data (RGBA buffer)
  │     └─→ PNG library handles:
  │           ├─→ Compression (DEFLATE)
  │           ├─→ Checksums (CRC)
  │           └─→ Chunking (IHDR, IDAT, IEND)
  │
  ├─→ Save to file: {output_dir}/page_{page_index:04}.png
  │     └─→ Example: page_0000.png, page_0001.png, ...
  │
  ├─→ FPDFBitmap_Destroy(bitmap)
  │     └─→ Free bitmap memory
  │
  └─→ FPDF_ClosePage(page)
        └─→ Release page memory
```

---

## Code Path 2: Multi-Process Rendering

**Function**: `render_multiprocess()` (lines 262-302)

### Coordinator Process

```
render_multiprocess(pdf_path, output_dir, worker_count, page_count, dpi)
  │
  ├─→ Calculate pages_per_worker
  │     └─→ ceil(page_count / worker_count)
  │     └─→ Example: 821 pages ÷ 4 workers = 206 pages per worker
  │
  ├─→ FOR EACH WORKER (worker_id = 0..worker_count-1):
  │     │
  │     ├─→ Calculate page range:
  │     │     ├─→ start_page = worker_id × pages_per_worker
  │     │     │     └─→ Worker 0: 0, Worker 1: 206, Worker 2: 412, Worker 3: 618
  │     │     │
  │     │     └─→ end_page = min((worker_id + 1) × pages_per_worker, page_count)
  │     │           └─→ Worker 3: min(824, 821) = 821 (handles remainder)
  │     │
  │     └─→ Spawn worker process:
  │           └─→ Command::new(current_exe())
  │                 .arg("--worker")
  │                 .arg(pdf_path)
  │                 .arg(output_dir)
  │                 .arg(start_page)     # Worker's first page
  │                 .arg(end_page)       # Worker's last page (exclusive)
  │                 .arg(dpi)
  │                 .arg(worker_id)
  │                 .spawn()
  │
  ├─→ Wait for all workers:
  │     └─→ FOR EACH worker:
  │           ├─→ child.wait() → status
  │           └─→ Check status.success()
  │                 └─→ Fail entire job if any worker fails
  │
  └─→ Return success
        └─→ All pages rendered to output_dir/page_NNNN.png
```

**Output Organization**:
```
output_dir/
  page_0000.png    # Rendered by worker 0
  page_0001.png    # Rendered by worker 0
  ...
  page_0205.png    # Rendered by worker 0
  page_0206.png    # Rendered by worker 1
  ...
  page_0411.png    # Rendered by worker 1
  page_0412.png    # Rendered by worker 2
  ...
```

**Key**: Workers write to SAME output directory
- No temp files needed (unlike text extraction)
- Filenames include page number → no conflicts
- Natural page ordering preserved

---

### Worker Process

**Function**: `render_pages_worker()` (lines 332-357)

```
worker_main()
  │
  ├─→ Parse arguments: pdf_path, output_dir, start_page, end_page, dpi, worker_id
  │
  └─→ render_pages_worker(pdf_path, output_dir, start_page, end_page, dpi)
        │
        ├─→ FPDF_InitLibrary()
        │     └─→ Each worker: Independent PDFium instance
        │
        ├─→ FPDF_LoadDocument(pdf_path, NULL)
        │     └─→ Each worker: Opens PDF independently
        │     └─→ Each worker: Loads full document (not just assigned pages)
        │
        └─→ FOR EACH ASSIGNED PAGE (page_index = start_page..end_page-1):
              │
              ├─→ render_page_to_png(doc, page_index, output_dir, dpi)
              │     └─→ [Full rendering pipeline - see above]
              │     └─→ Writes: {output_dir}/page_{page_index:04}.png
              │     └─→ On error: Log warning, continue
              │
              └─→ Continue with all assigned pages

        ├─→ FPDF_CloseDocument(doc)
        ├─→ FPDF_DestroyLibrary()
        │
        └─→ Return (worker exits)
```

**Worker Independence**:
- Each worker: Separate process
- Each worker: Own PDFium library instance
- Each worker: Own document handle
- No shared memory
- No IPC except output files (naturally ordered by filename)

---

## Rendering Details: FPDF_RenderPageBitmap

**Function**: `FPDF_RenderPageBitmap()` (PDFium internal)

**Call signature** (from render_page_to_png, lines 203-212):
```c
FPDF_RenderPageBitmap(
    bitmap,        // Destination: FPDF_BITMAP
    page,          // Source: FPDF_PAGE
    start_x = 0,   // Offset X (usually 0)
    start_y = 0,   // Offset Y (usually 0)
    size_x,        // Render width in pixels
    size_y,        // Render height in pixels
    rotate = 0,    // Rotation: 0=none, 1=90°, 2=180°, 3=270°
    flags          // Rendering flags (FPDF_ANNOT, FPDF_LCD_TEXT, etc.)
);
```

**Rendering Flags Available**:
- `FPDF_ANNOT (0x01)`: Render annotations
- `FPDF_LCD_TEXT (0x02)`: Optimize for LCD display
- `FPDF_NO_NATIVETEXT (0x04)`: Always use graphics rendering for text
- `FPDF_GRAYSCALE (0x08)`: Grayscale rendering
- `FPDF_DEBUG_INFO (0x80)`: Debug outlines
- `FPDF_NO_CATCH (0x100)`: Disable exception catching
- `FPDF_RENDER_LIMITEDIMAGE (0x200)`: Limit image cache
- `FPDF_RENDER_FORCEHALFTONE (0x400)`: Force halftone for images
- `FPDF_PRINTING (0x800)`: Optimize for printing

**Current Usage**: `FPDF_ANNOT` only (include annotations)

### Internal Rendering Stages (PDFium Engine)

```
FPDF_RenderPageBitmap()
  │
  ├─→ Parse page content stream
  │     └─→ PDF operators: m, l, c, re, f, S, etc.
  │
  ├─→ Build display list:
  │     ├─→ Text objects (Tj, TJ operators)
  │     ├─→ Path objects (stroke/fill)
  │     ├─→ Image objects (Do operator)
  │     └─→ Shading objects
  │
  ├─→ Apply transformation matrices:
  │     ├─→ Page CTM (Current Transformation Matrix)
  │     ├─→ Object-level transforms
  │     └─→ Convert to device coordinates
  │
  ├─→ Render each object to bitmap:
  │     │
  │     ├─→ TEXT RENDERING:
  │     │     ├─→ Load font (Type1, TrueType, CFF, etc.)
  │     │     ├─→ Get glyph outlines
  │     │     ├─→ Rasterize glyphs at correct size/position
  │     │     ├─→ Apply fill color
  │     │     └─→ Composite to bitmap
  │     │
  │     ├─→ PATH RENDERING:
  │     │     ├─→ Tesselate curves to line segments
  │     │     ├─→ Rasterize paths (stroke/fill)
  │     │     ├─→ Apply colors/patterns
  │     │     └─→ Composite to bitmap
  │     │
  │     ├─→ IMAGE RENDERING:
  │     │     ├─→ Decode image (JPEG, JPEG2000, JBIG2, etc.)
  │     │     ├─→ Apply color space transforms
  │     │     ├─→ Scale/rotate to fit
  │     │     └─→ Composite to bitmap
  │     │
  │     └─→ ANNOTATION RENDERING (if FPDF_ANNOT):
  │           ├─→ Render annotation appearance streams
  │           └─→ Composite over page content
  │
  └─→ Return rendered bitmap (in BGRA format)
```

---

## Bitmap Management

### Bitmap Creation

**Function**: `FPDFBitmap_Create(width, height, alpha)` (line 193)

**Parameters**:
- `width`: Width in pixels
- `height`: Height in pixels
- `alpha`: 0=no alpha (opaque), 1=has alpha (transparency)

**Memory Allocation**:
```
bytes_per_pixel = alpha ? 4 : 4  // Always 4 for BGRA
stride = width × bytes_per_pixel  // May add padding for alignment
total_bytes = stride × height

Example: 2550px × 3300px × 4 bytes = ~33MB per page
```

**Format**: BGRA (Blue, Green, Red, Alpha)
- Each pixel: 4 bytes
- Byte order: [B, G, R, A] (little-endian color)

### Bitmap Fill

**Function**: `FPDFBitmap_FillRect()` (line 200)

```c
FPDFBitmap_FillRect(
    bitmap,
    left = 0,
    top = 0,
    width = width_px,
    height = height_px,
    color = 0xFFFFFFFF  // ARGB format: 0xAARRGGBB
);
```

**Color Format**: 0xAARRGGBB (Alpha, Red, Green, Blue)
- `0xFFFFFFFF` = Opaque white (A=255, R=255, G=255, B=255)
- `0xFF000000` = Opaque black (A=255, R=0, G=0, B=0)

**Why Fill White**: PDF default background is white (not transparent)

### Bitmap Access

**Function**: `FPDFBitmap_GetBuffer()` (line 215)
- Returns: `*const u8` (pointer to raw pixel data)
- Format: BGRA
- Size: stride × height bytes

**Function**: `FPDFBitmap_GetStride()` (line 216)
- Returns: Bytes per row
- May include padding: `stride ≥ width × 4`
- Padding ensures memory alignment (performance optimization)

---

## Color Format Conversion: BGRA → RGBA

**Code** (lines 218-233):

```rust
let mut rgba_data = Vec::with_capacity((width_px * height_px * 4) as usize);

for y in 0..height_px {
    let row_offset = (y as usize) * stride;

    for x in 0..width_px {
        let pixel_offset = row_offset + (x as usize) * 4;

        // Read BGRA from PDFium bitmap
        let b = *buffer.add(pixel_offset);
        let g = *buffer.add(pixel_offset + 1);
        let r = *buffer.add(pixel_offset + 2);
        let a = *buffer.add(pixel_offset + 3);

        // Write RGBA for PNG
        rgba_data.push(r);
        rgba_data.push(g);
        rgba_data.push(b);
        rgba_data.push(a);
    }
}
```

**Why Conversion Needed**:
- PDFium bitmap: BGRA (Windows GDI format)
- PNG standard: RGBA
- Simply reorder bytes: [B,G,R,A] → [R,G,B,A]

**Performance**: ~10ms for 2550×3300 image (34M pixels)

---

## PNG Encoding

**Library**: `png` crate (Rust)

**Code** (lines 236-249):

```rust
let output_path = format!("{}/page_{:04}.png", output_dir, page_index);
let file = File::create(&output_path)?;
let w = BufWriter::new(file);

// Configure encoder
let mut encoder = png::Encoder::new(w, width_px as u32, height_px as u32);
encoder.set_color(png::ColorType::Rgba);    // RGBA format
encoder.set_depth(png::BitDepth::Eight);    // 8 bits per channel

// Write PNG
let mut writer = encoder.write_header()?;
writer.write_image_data(&rgba_data)?;
```

**PNG Structure**:
```
PNG file:
  ├─→ PNG signature (8 bytes)
  ├─→ IHDR chunk (image header): width, height, bit depth, color type
  ├─→ IDAT chunk(s) (image data): Compressed RGBA pixels (DEFLATE)
  └─→ IEND chunk (end marker)
```

**Compression**:
- DEFLATE algorithm (gzip)
- Lossless compression
- Typical ratio: 3:1 to 10:1 (varies by content)
- Text-heavy pages: Better compression
- Photo-heavy pages: Less compression

**Output Size**:
- Uncompressed RGBA: 2550 × 3300 × 4 = ~33MB
- Compressed PNG: ~3-10MB (typical)
- Highly variable by content

---

## Code Path 3: Multi-Process Coordination

**Parallelism Model**:

```
Coordinator Process
  │
  ├─→ Spawn Worker 0 (pages 0-205)
  │     └─→ Process 1234
  │
  ├─→ Spawn Worker 1 (pages 206-411)
  │     └─→ Process 1235
  │
  ├─→ Spawn Worker 2 (pages 412-617)
  │     └─→ Process 1236
  │
  ├─→ Spawn Worker 3 (pages 618-820)
  │     └─→ Process 1237
  │
  └─→ Wait for all workers
        ├─→ Workers run in parallel (true CPU parallelism)
        ├─→ Each writes directly to output_dir/page_NNNN.png
        └─→ Coordinator waits, then returns
```

**No Synchronization Needed**:
- Workers write to different files (page numbers don't overlap)
- No shared memory
- No locks/mutexes
- Natural page ordering (filename-based: page_0000, page_0001, ...)

---

## Performance Characteristics

### Single-Threaded Performance

**Benchmark** (821-page PDF at 300 DPI):
- Total time: ~164 seconds
- Per-page: ~200ms
- Output: 821 PNG files (~4GB total)

**Per-Page Breakdown**:
- Page load: ~5ms
- Rendering: ~150ms (CPU-bound)
- BGRA→RGBA: ~10ms
- PNG encode: ~35ms (compression)

### Multi-Process Performance (4 Workers)

**Benchmark** (821-page PDF at 300 DPI):
- Total time: ~49 seconds
- Speedup: **3.34x**
- Output: Same 821 PNG files

**Overhead Analysis**:
- Process spawn: ~50ms × 4 = 200ms
- PDF load: ~20ms × 4 = 80ms
- Coordination: ~20ms
- **Total overhead: ~300ms**

**Efficiency**:
- 821 pages ÷ 4 workers = ~205 pages per worker
- Per-worker time: ~49s (all workers finish ~same time)
- Linear speedup: 164s ÷ 4 = 41s (ideal)
- Actual: 49s (84% efficiency, 16% overhead)

### Small PDF Overhead

**100-page PDF**:
- Single-threaded: ~20 seconds
- Multi-process (4w): ~15 seconds
- Speedup: 1.33x (overhead dominates)

**Why Threshold at 200 Pages**:
- Overhead: ~300ms (fixed)
- Work per page: ~200ms
- Break-even: 300ms overhead ÷ (200ms × 3 workers saved) = ~2 pages of overhead
- Conservative threshold: 200 pages ensures overhead < 5% of total work

---

## DPI Scaling Impact

**72 DPI** (screen quality):
- 8.5" × 11" page = 612 × 792 pixels
- File size: ~500KB per page
- Render time: ~50ms per page

**150 DPI** (draft print):
- 8.5" × 11" page = 1275 × 1650 pixels
- File size: ~1.5MB per page
- Render time: ~100ms per page

**300 DPI** (standard print):
- 8.5" × 11" page = 2550 × 3300 pixels
- File size: ~3-5MB per page
- Render time: ~200ms per page

**600 DPI** (high quality):
- 8.5" × 11" page = 5100 × 6600 pixels
- File size: ~12-20MB per page
- Render time: ~800ms per page

**Scaling**: Time scales with pixel count (width × height)
- 2x DPI → 4x pixels → ~4x time

---

## Memory Usage

**Per-Page Memory** (at 300 DPI):

```
Bitmap (BGRA):      2550 × 3300 × 4 = ~33MB
RGBA conversion:    2550 × 3300 × 4 = ~33MB
PNG encode buffer:  ~5MB (compressed)
Peak per page:      ~70MB
```

**Multi-Process Memory** (4 workers):
- Each worker: ~70MB per page
- Overlap: Workers render different pages
- Peak: 4 × 70MB = ~280MB (if all workers rendering simultaneously)

**Total Memory Usage**:
- Coordinator: ~50MB (overhead)
- 4 Workers: 4 × 70MB = ~280MB
- **Peak: ~330MB** (for 821-page PDF at 300 DPI)

---

## Error Handling

### Page Load Failures

**Code** (lines 178-181):
```rust
let page = FPDF_LoadPage(doc, page_index);
if page.is_null() {
    return Err(format!("Failed to load page {}", page_index));
}
```

**Causes**:
- Encrypted page (no password provided)
- Corrupt page object
- Invalid page index
- Out of memory

**Strategy**: Continue on error, log warning

### Bitmap Creation Failures

**Code** (lines 193-197):
```rust
let bitmap = FPDFBitmap_Create(width_px, height_px, 0);
if bitmap.is_null() {
    FPDF_ClosePage(page);
    return Err(format!("Failed to create bitmap for page {}", page_index));
}
```

**Causes**:
- Out of memory (bitmap too large)
- Invalid dimensions (width/height too large)

**Memory Limit**: Typical max bitmap size ~100MP (megapixels)
- Example: 10,000 × 10,000 pixels × 4 bytes = 400MB per page

### PNG Encoding Failures

**Code** (lines 245-249):
```rust
let mut writer = encoder.write_header()
    .map_err(|e| format!("Failed to write PNG header: {}", e))?;

writer.write_image_data(&rgba_data)
    .map_err(|e| format!("Failed to write PNG data: {}", e))?;
```

**Causes**:
- Disk full
- Permission denied
- I/O error

---

## Thread Safety

**Single-Threaded**: Safe (no concurrency)

**Multi-Process**: Safe (no shared memory)
- Each worker: Separate process
- Each worker: Own PDFium instance
- Each worker: Writes to different filenames
- No race conditions
- No locks needed

**Why Not Threads**: Same as text extraction
- PDFium constraint: "Only one call at a time per instance"
- Threads + Mutex: Serialization (1.87x speedup, suboptimal)
- Processes: True parallelism (3.34x speedup)

---

## Output Format: PNG Specification

**PNG File Format**:

```
Offset | Size | Description
-------|------|------------
0      | 8    | PNG signature: 0x89 'P' 'N' 'G' 0x0D 0x0A 0x1A 0x0A
8      | 4    | IHDR length
12     | 4    | "IHDR" chunk type
16     | 4    | Width (big-endian)
20     | 4    | Height (big-endian)
24     | 1    | Bit depth (8)
25     | 1    | Color type (6 = RGBA)
26     | 1    | Compression (0 = DEFLATE)
27     | 1    | Filter (0 = None)
28     | 1    | Interlace (0 = None)
29     | 4    | IHDR CRC
33     | ...  | IDAT chunks (compressed image data)
...    | ...  | IEND chunk (end marker)
```

**Color Type 6**: RGBA (Truecolor with alpha)
- Red: 8 bits (0-255)
- Green: 8 bits (0-255)
- Blue: 8 bits (0-255)
- Alpha: 8 bits (0=transparent, 255=opaque)

**Compression**: DEFLATE (same as gzip)
- Lossless
- Adjustable compression level (0=none, 9=max)
- Current: Default level (6)

---

## Alternative Output Formats

### JPEG Output

**Advantages**:
- 10x smaller files (~300KB vs 3MB per page)
- Faster encoding (~20ms vs ~35ms per page)
- Total: ~2GB vs ~20GB for 452 PDFs

**Disadvantages**:
- Lossy compression (quality loss)
- No alpha channel
- Not pixel-perfect (artifacts)

**Implementation**:
```rust
// Replace PNG encoder with JPEG
let mut encoder = jpeg_encoder::Encoder::new(&file, 85);  // Quality 85%
encoder.encode(&rgb_data, width_px as u16, height_px as u16, ColorType::Rgb)?;
```

**Trade-off**:
- Correctness tests: PNG (pixel-perfect)
- Performance tests: JPEG (faster, smaller)

---

## Rendering Flags Detailed

**FPDF_ANNOT (0x01)**: Render annotations
```
Examples:
- Comments
- Highlights
- Stamps
- Form fields
```

**FPDF_LCD_TEXT (0x02)**: LCD text optimization
```
- Sub-pixel rendering (RGB stripes)
- Improves text clarity on LCD screens
- Not recommended for printing
```

**FPDF_PRINTING (0x800)**: Printing optimization
```
- Disables LCD text
- Enables print-specific hinting
- Use for print-quality output
```

**Current Usage**: Only `FPDF_ANNOT`
- Includes annotations (important for documents)
- No LCD optimization (consistent output)
- No printing flags (testing focus)

---

## Comparison with pdfium_test (Official CLI)

**pdfium_test** usage:
```bash
pdfium_test --png test.pdf
# Generates: test.pdf.0.png, test.pdf.1.png, ...
```

**Our render_pages.rs**:
```bash
render_pages test.pdf output_dir/
# Generates: output_dir/page_0000.png, output_dir/page_0001.png, ...
```

**Differences**:
- ✅ Standardized naming: `page_NNNN.png` vs `test.pdf.N.png`
- ✅ Output directory: Organized vs scattered
- ✅ Multi-process: 3.34x faster
- ✅ DPI control: Configurable vs fixed
- ✅ Worker auto-selection: Smart dispatch vs always single-threaded

---

## Debugging Tips

**Black/Blank Images**:
- Check `FPDFBitmap_FillRect()` was called (white background)
- Check `FPDF_RenderPageBitmap()` return value
- Verify page has content (not blank page)

**Color Issues**:
- Verify BGRA→RGBA conversion
- Check PNG color type (should be 6 = RGBA)
- Verify bitmap format (4 bytes per pixel)

**Size Mismatches**:
- Check DPI calculation: scale = dpi / 72.0
- Verify page dimensions: FPDF_GetPageWidthF/HeightF
- Check pixel dimensions: width_pts × scale

**Memory Issues**:
- Check bitmap creation success (not NULL)
- Monitor process memory usage
- Reduce DPI if out of memory (300 → 150 → 72)

**Performance Issues**:
- Verify multi-process is used for large PDFs
- Check worker count: `top` should show 4 processes at 100% CPU
- Profile bottleneck: rendering vs encoding vs I/O

---

## Future Enhancements

### 1. JPEG Output Option
```rust
fn render_page_to_jpeg(doc, page_index, output_dir, dpi, quality) {
    // Same rendering pipeline
    // Replace PNG encoder with JPEG encoder
    // 10x smaller files, faster encoding
}
```

### 2. Per-Page Metadata JSON
```json
{
  "page": 0,
  "width_pts": 612.0,
  "height_pts": 792.0,
  "width_px": 2550,
  "height_px": 3300,
  "dpi": 300.0,
  "render_time_ms": 150.5,
  "file_size": 3457829,
  "compression_ratio": 9.5
}
```

### 3. Render Quality Presets
```rust
enum RenderQuality {
    Draft,      // 72 DPI, JPEG 80%
    Standard,   // 150 DPI, PNG
    High,       // 300 DPI, PNG
    Print,      // 600 DPI, PNG + FPDF_PRINTING flag
}
```

### 4. Incremental Rendering
```rust
// Render only missing pages
fn render_missing_pages(pdf_path, output_dir, dpi) {
    let existing_pages = scan_output_dir(output_dir);
    let missing_pages = (0..page_count).filter(|p| !existing_pages.contains(p));
    render_pages(missing_pages, ...);
}
```

---

## Reference

**PDFium APIs Used**:
- `FPDF_InitLibrary()` / `FPDF_DestroyLibrary()`
- `FPDF_LoadDocument()` / `FPDF_CloseDocument()`
- `FPDF_GetPageCount()`
- `FPDF_LoadPage()` / `FPDF_ClosePage()`
- `FPDF_GetPageWidthF()` / `FPDF_GetPageHeightF()`
- `FPDFBitmap_Create()` / `FPDFBitmap_Destroy()`
- `FPDFBitmap_FillRect()`
- `FPDFBitmap_GetBuffer()` / `FPDFBitmap_GetStride()`
- `FPDF_RenderPageBitmap()`

**Rust Libraries**:
- `png` crate: PNG encoding/decoding
- `std::process::Command`: Multi-process spawning
- `std::fs::File` / `std::io::BufWriter`: File I/O

**Implementation Files**:
- `rust/pdfium-sys/examples/render_pages.rs` (358 lines)
- `rust/pdfium-sys/examples/parallel_render.rs` (thread-based, reference)
- `rust/pdfium-sys/examples/parallel_render_multiproc.rs` (multi-process, reference)

**Test Files**:
- `integration_tests/tests/test_005_image_correctness.py`
- `integration_tests/tests/test_009_multiprocess_benchmark.py`
