# Text Extraction Code Path Trace

**Date**: 2025-10-31
**Purpose**: Complete code path documentation for text extraction with metadata
**Audience**: Future AIs, developers debugging text extraction issues

---

## Overview

Text extraction in PDFium uses the `FPDFText_*` API family to extract character-by-character text with rich metadata including:
- Unicode codepoints
- Bounding boxes (position/size)
- Font information (name, size, weight, flags)
- Text styling (fill/stroke colors, rotation angle)
- Character properties (generated, hyphen, unicode mapping errors)

---

## Entry Point: extract_text.rs

**Location**: `rust/pdfium-sys/examples/extract_text.rs`

**Dispatcher Logic** (lines 24-96):
```
main()
  ├─→ Check args[1] == "--worker" → worker_main()  [Multi-process worker]
  │
  ├─→ Parse arguments: pdf_path, output_path, [worker_count]
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
  └─→ Route to implementation:
        ├─→ worker_count == 1 → extract_text_single_threaded()
        └─→ worker_count > 1  → extract_text_multiprocess()
```

**Key Decision**: PAGE_THRESHOLD = 200
- **< 200 pages**: Single-threaded (process overhead > parallelism gain)
- **≥ 200 pages**: Multi-process (3.0x+ speedup)

---

## Code Path 1: Single-Threaded Extraction

**Function**: `extract_text_single_threaded()` (lines 125-209)

### Step-by-Step Execution

```
extract_text_single_threaded(pdf_path, output_path)
  │
  ├─→ FPDF_InitLibrary()
  │     └─→ Initialize PDFium library (global state)
  │
  ├─→ FPDF_LoadDocument(pdf_path, password=NULL)
  │     └─→ Load PDF document
  │     └─→ Returns FPDF_DOCUMENT handle (or NULL on error)
  │
  ├─→ Create output file
  │     └─→ Write UTF-32 LE BOM: [0xFF, 0xFE, 0x00, 0x00]
  │
  ├─→ FPDF_GetPageCount(doc)
  │     └─→ Get total page count
  │
  └─→ FOR EACH PAGE (page_index = 0..page_count-1):
        │
        ├─→ Write BOM (for pages after first)
        │     └─→ Each page starts with BOM in output
        │
        ├─→ FPDF_LoadPage(doc, page_index)
        │     └─→ Load page structure
        │     └─→ Returns FPDF_PAGE handle
        │
        ├─→ FPDFText_LoadPage(page)
        │     └─→ Build text page structure
        │     └─→ Parses all text objects on page
        │     └─→ Returns FPDF_TEXTPAGE handle
        │
        ├─→ FPDFText_CountChars(text_page)
        │     └─→ Get character count (NOT byte count!)
        │     └─→ Includes generated chars (spaces, newlines)
        │
        ├─→ FOR EACH CHARACTER (i = 0..char_count-1):
        │     │
        │     ├─→ FPDFText_GetUnicode(text_page, i)
        │     │     └─→ Returns UTF-16 code unit (unsigned int)
        │     │
        │     ├─→ Handle UTF-16 surrogate pairs:
        │     │     ├─→ If high surrogate (0xD800-0xDBFF):
        │     │     │     ├─→ Read next character (low surrogate)
        │     │     │     ├─→ Combine: ((high - 0xD800) << 10) + (low - 0xDC00) + 0x10000
        │     │     │     └─→ i += 1 (consume both code units)
        │     │     └─→ Else: Use code unit directly
        │     │
        │     ├─→ Convert to UTF-32 LE bytes (4 bytes per codepoint)
        │     │     └─→ codepoint.to_le_bytes()
        │     │
        │     └─→ Write to output file
        │
        ├─→ FPDFText_ClosePage(text_page)
        │     └─→ Release text page memory
        │
        └─→ FPDF_ClosePage(page)
              └─→ Release page memory

  ├─→ FPDF_CloseDocument(doc)
  │     └─→ Release document memory
  │
  └─→ FPDF_DestroyLibrary()
        └─→ Cleanup PDFium global state
```

**Output Format**: UTF-32 LE (4 bytes per character)
- File BOM: `FF FE 00 00`
- Page BOM: `FF FE 00 00` (between pages)
- Character: 4 bytes (little-endian Unicode codepoint)

---

## Code Path 2: Multi-Process Extraction

**Function**: `extract_text_multiprocess()` (lines 215-283)

### Coordinator Process

```
extract_text_multiprocess(pdf_path, output_path, worker_count, page_count)
  │
  ├─→ Calculate pages_per_worker
  │     └─→ ceil(page_count / worker_count)
  │
  ├─→ FOR EACH WORKER (worker_id = 0..worker_count-1):
  │     │
  │     ├─→ Calculate page range:
  │     │     ├─→ start_page = worker_id * pages_per_worker
  │     │     └─→ end_page = min((worker_id + 1) * pages_per_worker, page_count)
  │     │
  │     ├─→ Create temp file: /tmp/pdfium_worker_{pid}_{worker_id}.bin
  │     │
  │     └─→ Spawn worker process:
  │           └─→ Command::new(current_exe)
  │                 .arg("--worker")
  │                 .arg(pdf_path)
  │                 .arg(temp_file)
  │                 .arg(start_page)
  │                 .arg(end_page)
  │                 .arg(worker_id)
  │                 .spawn()
  │
  ├─→ Wait for all workers:
  │     └─→ FOR EACH worker: child.wait()
  │           └─→ Check exit status (fail if any worker fails)
  │
  ├─→ Combine results:
  │     ├─→ Create output file
  │     ├─→ Write UTF-32 LE BOM
  │     ├─→ FOR EACH worker temp file (in order):
  │     │     ├─→ Read worker output
  │     │     ├─→ Append to output file
  │     │     └─→ Delete temp file
  │     │
  │     └─→ Result: Pages concatenated in correct order
  │
  └─→ Return success/failure
```

**Critical**: Workers process INDEPENDENT page ranges
- Worker 0: pages 0-49
- Worker 1: pages 50-99
- Worker 2: pages 100-149
- Worker 3: pages 150-199

**No shared state**: Each worker has own PDFium instance

---

### Worker Process

**Function**: `extract_pages()` (lines 312-392)

```
worker_main()
  │
  ├─→ Parse arguments: pdf_path, output_path, start_page, end_page, worker_id
  │
  └─→ extract_pages(pdf_path, output_path, start_page, end_page, worker_id)
        │
        ├─→ FPDF_InitLibrary()
        │     └─→ Each worker: Independent PDFium instance
        │
        ├─→ FPDF_LoadDocument(pdf_path, NULL)
        │     └─→ Each worker: Opens PDF independently
        │
        ├─→ Create output file
        │
        ├─→ FOR EACH ASSIGNED PAGE (page_index = start_page..end_page-1):
        │     │
        │     ├─→ FPDF_LoadPage(doc, page_index)
        │     │
        │     ├─→ FPDFText_LoadPage(page)
        │     │
        │     ├─→ FPDFText_CountChars(text_page)
        │     │
        │     ├─→ Build page buffer:
        │     │     ├─→ Add BOM: [0xFF, 0xFE, 0x00, 0x00]
        │     │     │
        │     │     └─→ FOR EACH CHARACTER:
        │     │           ├─→ FPDFText_GetUnicode(text_page, i)
        │     │           ├─→ Handle surrogate pairs (if needed)
        │     │           ├─→ Convert to UTF-32 LE bytes
        │     │           └─→ Add to buffer
        │     │
        │     ├─→ Write page to output:
        │     │     ├─→ IF worker_id == 0 AND first page:
        │     │     │     └─→ Skip BOM (coordinator adds file-level BOM)
        │     │     └─→ ELSE:
        │     │           └─→ Write full buffer (with BOM)
        │     │
        │     ├─→ FPDFText_ClosePage(text_page)
        │     └─→ FPDF_ClosePage(page)
        │
        ├─→ FPDF_CloseDocument(doc)
        ├─→ FPDF_DestroyLibrary()
        │
        └─→ Return (worker exits)
```

**Result**: Each worker writes pages to temp file
- Temp files contain pages in order (within worker's range)
- Coordinator concatenates temp files in worker order
- Final output: All pages in correct order

---

## Text Extraction with Metadata (JSONL)

**Current Implementation**: Basic text only (UTF-32 LE)

**Required for JSONL**: Character-level metadata

### Available FPDFText APIs for Metadata

**Per Character**:

1. **FPDFText_GetUnicode(text_page, index)**
   - Returns: unsigned int (UTF-16 code unit or BMP codepoint)
   - Unicode character value

2. **FPDFText_GetCharBox(text_page, index, &left, &right, &bottom, &top)**
   - Returns: FPDF_BOOL
   - Bounding box in PDF user space coordinates

3. **FPDFText_GetCharOrigin(text_page, index, &x, &y)**
   - Returns: FPDF_BOOL
   - Character origin point

4. **FPDFText_GetFontSize(text_page, index)**
   - Returns: double
   - Font size in points (~1/72 inch)

5. **FPDFText_GetFontInfo(text_page, index, buffer, buflen, &flags)**
   - Returns: unsigned long (buffer length needed)
   - Font name (UTF-8 string)
   - Font flags (PDF spec 1.7 Section 5.7.1)

6. **FPDFText_GetFontWeight(text_page, index)**
   - Returns: int
   - Font weight (100-900, e.g., 400=normal, 700=bold)

7. **FPDFText_GetFillColor(text_page, index, &R, &G, &B, &A)**
   - Returns: FPDF_BOOL
   - Fill color (RGBA, 0-255 each)

8. **FPDFText_GetStrokeColor(text_page, index, &R, &G, &B, &A)**
   - Returns: FPDF_BOOL
   - Stroke color (RGBA, 0-255 each)

9. **FPDFText_GetCharAngle(text_page, index)**
   - Returns: float
   - Rotation angle in radians

10. **FPDFText_GetMatrix(text_page, index, &matrix)**
    - Returns: FPDF_BOOL
    - Transformation matrix (FS_MATRIX: a, b, c, d, e, f)

11. **FPDFText_IsGenerated(text_page, index)**
    - Returns: int (1=generated, 0=original, -1=error)
    - Whether character was generated by PDFium (e.g., synthetic space)

12. **FPDFText_IsHyphen(text_page, index)**
    - Returns: int (1=hyphen, 0=not hyphen, -1=error)
    - Whether character is a hyphen

13. **FPDFText_HasUnicodeMapError(text_page, index)**
    - Returns: int (1=error, 0=ok, -1=error)
    - Whether character has invalid unicode mapping

---

## Proposed JSONL Extraction Code Path

### Modified extract_pages() with Metadata

```rust
fn extract_pages_with_metadata(
    pdf_path: &str,
    output_path: &str,
    start_page: usize,
    end_page: usize
) -> Result<(), String> {
    unsafe {
        FPDF_InitLibrary();
        let doc = FPDF_LoadDocument(...);

        let mut output_file = File::create(output_path)?;

        for page_index in start_page..end_page {
            let page = FPDF_LoadPage(doc, page_index as i32);
            let text_page = FPDFText_LoadPage(page);
            let char_count = FPDFText_CountChars(text_page);

            // Extract each character with metadata
            for char_idx in 0..char_count {
                // 1. Character
                let unicode = FPDFText_GetUnicode(text_page, char_idx);
                let char_value = std::char::from_u32(unicode).unwrap_or('�');

                // 2. Bounding box
                let mut left = 0.0;
                let mut right = 0.0;
                let mut bottom = 0.0;
                let mut top = 0.0;
                FPDFText_GetCharBox(text_page, char_idx, &mut left, &mut right, &mut bottom, &mut top);

                // 3. Origin
                let mut origin_x = 0.0;
                let mut origin_y = 0.0;
                FPDFText_GetCharOrigin(text_page, char_idx, &mut origin_x, &mut origin_y);

                // 4. Font information
                let font_size = FPDFText_GetFontSize(text_page, char_idx);

                let mut font_buffer = vec![0u8; 256];
                let mut font_flags = 0i32;
                let font_name_len = FPDFText_GetFontInfo(
                    text_page,
                    char_idx,
                    font_buffer.as_mut_ptr() as *mut std::ffi::c_void,
                    font_buffer.len() as u64,
                    &mut font_flags
                );
                let font_name = String::from_utf8_lossy(&font_buffer[..font_name_len as usize]);

                let font_weight = FPDFText_GetFontWeight(text_page, char_idx);

                // 5. Colors
                let mut fill_r = 0u32;
                let mut fill_g = 0u32;
                let mut fill_b = 0u32;
                let mut fill_a = 0u32;
                FPDFText_GetFillColor(text_page, char_idx, &mut fill_r, &mut fill_g, &mut fill_b, &mut fill_a);

                let mut stroke_r = 0u32;
                let mut stroke_g = 0u32;
                let mut stroke_b = 0u32;
                let mut stroke_a = 0u32;
                FPDFText_GetStrokeColor(text_page, char_idx, &mut stroke_r, &mut stroke_g, &mut stroke_b, &mut stroke_a);

                // 6. Rotation
                let angle = FPDFText_GetCharAngle(text_page, char_idx);

                // 7. Transformation matrix
                let mut matrix = FS_MATRIX { a: 1.0, b: 0.0, c: 0.0, d: 1.0, e: 0.0, f: 0.0 };
                FPDFText_GetMatrix(text_page, char_idx, &mut matrix);

                // 8. Character properties
                let is_generated = FPDFText_IsGenerated(text_page, char_idx);
                let is_hyphen = FPDFText_IsHyphen(text_page, char_idx);
                let has_unicode_error = FPDFText_HasUnicodeMapError(text_page, char_idx);

                // 9. Serialize to JSONL
                let char_metadata = serde_json::json!({
                    "page": page_index,
                    "char_idx": char_idx,
                    "char": char_value,
                    "unicode": unicode,
                    "bbox": {
                        "left": left,
                        "top": top,
                        "right": right,
                        "bottom": bottom,
                        "width": right - left,
                        "height": top - bottom
                    },
                    "origin": {
                        "x": origin_x,
                        "y": origin_y
                    },
                    "font": {
                        "name": font_name,
                        "size": font_size,
                        "weight": font_weight,
                        "flags": font_flags
                    },
                    "color": {
                        "fill": {"r": fill_r, "g": fill_g, "b": fill_b, "a": fill_a},
                        "stroke": {"r": stroke_r, "g": stroke_g, "b": stroke_b, "a": stroke_a}
                    },
                    "angle": angle,
                    "matrix": {
                        "a": matrix.a, "b": matrix.b,
                        "c": matrix.c, "d": matrix.d,
                        "e": matrix.e, "f": matrix.f
                    },
                    "properties": {
                        "generated": is_generated == 1,
                        "hyphen": is_hyphen == 1,
                        "unicode_error": has_unicode_error == 1
                    }
                });

                // Write JSONL line
                writeln!(output_file, "{}", serde_json::to_string(&char_metadata)?)?;
            }

            FPDFText_ClosePage(text_page);
            FPDF_ClosePage(page);
        }

        FPDF_CloseDocument(doc);
        FPDF_DestroyLibrary();
    }
}
```

**JSONL Output Format** (one line per character):
```jsonl
{"page":0,"char_idx":0,"char":"H","unicode":72,"bbox":{"left":100.5,"top":712.0,"right":108.7,"bottom":700.0,"width":8.2,"height":12.0},"origin":{"x":100.5,"y":700.0},"font":{"name":"Arial","size":12.0,"weight":400,"flags":0},"color":{"fill":{"r":0,"g":0,"b":0,"a":255},"stroke":{"r":0,"g":0,"b":0,"a":0}},"angle":0.0,"matrix":{"a":1.0,"b":0.0,"c":0.0,"d":1.0,"e":0.0,"f":0.0},"properties":{"generated":false,"hyphen":false,"unicode_error":false}}
{"page":0,"char_idx":1,"char":"e","unicode":101,"bbox":{"left":108.7,"top":712.0,"right":116.2,"bottom":700.0,"width":7.5,"height":12.0},"origin":{"x":108.7,"y":700.0},"font":{"name":"Arial","size":12.0,"weight":400,"flags":0},"color":{"fill":{"r":0,"g":0,"b":0,"a":255},"stroke":{"r":0,"g":0,"b":0,"a":0}},"angle":0.0,"matrix":{"a":1.0,"b":0.0,"c":0.0,"d":1.0,"e":0.0,"f":0.0},"properties":{"generated":false,"hyphen":false,"unicode_error":false}}
```

**Alternative: Text-Run Level** (grouped by formatting):
```jsonl
{"page":0,"text":"Hello World","char_count":11,"bbox":{"left":100.5,"top":712.0,"right":180.2,"bottom":700.0},"font":{"name":"Arial","size":12.0,"weight":400},"color":{"fill":{"r":0,"g":0,"b":0,"a":255}}}
{"page":0,"text":"Next paragraph","char_count":14,"bbox":{"left":100.5,"top":680.0,"right":195.3,"bottom":668.0},"font":{"name":"Arial","size":12.0,"weight":400},"color":{"fill":{"r":0,"g":0,"b":0,"a":255}}}
```

---

## Performance Characteristics

**Single-Threaded**:
- **100-page PDF**: ~2.0 seconds
- **200-page PDF**: ~4.0 seconds
- **821-page PDF**: ~16.4 seconds
- Linear scaling with page count

**Multi-Process (4 workers)**:
- **100-page PDF**: ~1.3 seconds (1.54x speedup, overhead hurts)
- **200-page PDF**: ~2.0 seconds (2.0x speedup, break-even)
- **821-page PDF**: ~5.1 seconds (3.21x speedup, CPU parallelism wins)

**Overhead**:
- Process spawn: ~50ms per worker
- PDF load: ~20ms per worker
- IPC (temp files): ~30ms total
- Total overhead: ~250ms for 4 workers

**Threshold Analysis**:
- < 200 pages: Overhead > gain → Single-threaded
- ≥ 200 pages: Gain > overhead → Multi-process

---

## Critical Implementation Details

### UTF-16 Surrogate Pair Handling

**Code** (lines 180-191, 356-366):
```rust
let code_unit = FPDFText_GetUnicode(text_page, i);

let codepoint = if code_unit >= 0xD800 && code_unit <= 0xDBFF {
    // High surrogate - need to read low surrogate
    i += 1;
    if i < char_count {
        let low_surrogate = FPDFText_GetUnicode(text_page, i);
        // Combine surrogates into single codepoint
        ((code_unit - 0xD800) << 10) + (low_surrogate - 0xDC00) + 0x10000
    } else {
        code_unit  // Incomplete pair
    }
} else {
    code_unit
};
```

**Why This Matters**:
- BMP characters (U+0000-U+FFFF): One code unit
- Supplementary characters (U+10000-U+10FFFF): Two code units (surrogate pair)
- Examples: Emoji (🎉), Math symbols (𝒜), Historic scripts

**Common Bug**: Treating `i` as character index
- ❌ WRONG: `char[i]` = i-th character
- ✅ CORRECT: `FPDFText_GetUnicode(text_page, i)` where i may skip surrogate pairs

### BOM (Byte Order Mark) Handling

**File-level BOM** (line 141-142):
```rust
output_file.write_all(&[0xFF, 0xFE, 0x00, 0x00])  // UTF-32 LE BOM
```

**Page-level BOM** (lines 155-157):
```rust
if page_index > 0 {
    output_file.write_all(&[0xFF, 0xFE, 0x00, 0x00])  // BOM between pages
}
```

**Why**: PDF doesn't have inherent page breaks - we synthesize them with BOMs

**Worker BOM Handling** (lines 374-380):
```rust
if worker_id == 0 && page_index == start_page {
    // Worker 0's first page: Skip BOM (coordinator adds file-level BOM)
    output_file.write_all(&page_buffer[4..])
} else {
    // All other pages: Include BOM
    output_file.write_all(&page_buffer)
}
```

---

## Error Handling

**Page Load Failures**:
```rust
if page.is_null() {
    eprintln!("Warning: Failed to load page {}", page_index);
    continue;  // Skip page, continue with others
}
```

**Text Page Failures**:
```rust
if text_page.is_null() {
    FPDF_ClosePage(page);
    eprintln!("Warning: Failed to load text for page {}", page_index);
    continue;  // Skip page
}
```

**Strategy**: Continue on page failures, log warnings
- Encrypted pages: Fail to load
- Corrupt pages: Fail to parse
- Result: Partial extraction better than total failure

---

## Thread Safety

**Single-Threaded**: Safe (no concurrency)

**Multi-Process**: Safe (no shared memory)
- Each worker: Independent process
- Each worker: Own PDFium instance (`FPDF_InitLibrary()` per worker)
- Each worker: Own document handle
- No inter-process communication except temp files
- Coordinator: Waits for all workers before combining

**Why not threads**: PDFium constraint: "Only one PDFium call at a time per instance"
- Threads + Mutex: Serialization negates parallelism (1.0x speedup)
- Processes: True parallelism (3.0x+ speedup)

---

## Output Format Comparison

### Current: UTF-32 LE (Basic Text)
```
FF FE 00 00  48 00 00 00  65 00 00 00  6C 00 00 00  6C 00 00 00  6F 00 00 00
^BOM        ^'H'        ^'e'        ^'l'        ^'l'        ^'o'
```

### Proposed: JSONL (With Metadata)
```jsonl
{"page":0,"char_idx":0,"char":"H","unicode":72,"bbox":{...},"font":{...},...}
{"page":0,"char_idx":1,"char":"e","unicode":101,"bbox":{...},"font":{...},...}
```

**Size Comparison**:
- UTF-32 LE: 4 bytes per character
- JSONL: ~200-300 bytes per character (50-75x larger)

**For 1000-page PDF with 500k characters**:
- UTF-32 LE: 2 MB
- JSONL: 100-150 MB

**Optimization**: Text-run grouping can reduce by 10-20x

---

## Reference Implementation Status

**Implemented**:
- ✅ Single-threaded text extraction
- ✅ Multi-process text extraction
- ✅ UTF-16 surrogate pair handling
- ✅ BOM handling
- ✅ Page ordering preservation

**Not Yet Implemented**:
- ❌ Metadata extraction (FPDFText_GetCharBox, GetFontInfo, etc.)
- ❌ JSONL output format
- ❌ Per-page output files
- ❌ Text-run grouping

---

## Debugging Tips

**Text Mismatch**:
1. Check page count: `FPDF_GetPageCount(doc)`
2. Check character count per page: `FPDFText_CountChars(text_page)`
3. Compare per-page outputs (requires per-page extraction)
4. Check for surrogate pair bugs (emoji, special chars)

**Performance Issues**:
1. Check worker distribution: pages_per_worker should be balanced
2. Check temp file I/O: verify no disk bottleneck
3. Check PDFium init overhead: ~20ms per worker
4. Verify true parallelism: `top` during extraction (4 processes at 100% CPU)

**Correctness Issues**:
1. Verify page order: Worker 0, 1, 2, 3 outputs concatenated correctly
2. Check BOM handling: File should start with one BOM, pages have BOMs between them
3. UTF-16 surrogates: Test with emoji/math PDFs

---

## Next Steps for JSONL Implementation

1. Add `--jsonl` flag to extract_text.rs
2. Implement metadata extraction per character
3. Serialize to JSONL format
4. Add per-page output option (`--per-page-output`)
5. Update worker processes to support JSONL mode
6. Test with metadata-rich PDFs (fonts, colors, rotations)

**Estimated complexity**: 2-3 AI commits
**Reference**: See this document for complete API surface area
