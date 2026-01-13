# PDFium Rust Architecture - v1.1.0

**Status:** Production Ready (as of 2025-11-05, commit # 252)
**Version:** v1.1.0
**Upstream PDFium:** 7f43fd79 (2025-10-30)

---

## Overview

Direct Rust → PDFium integration with multi-process parallelism for high-performance text extraction and image rendering.

## Architecture

```
┌──────────────────────────────────────┐
│  Python Test Suite                   │  ← Integration tests, baseline validation
│  (integration_tests/)                │
└────────────┬─────────────────────────┘
             │ subprocess
             ▼
┌──────────────────────────────────────┐
│  Rust CLI Tools                      │  ← Production binaries
│  - extract_text (text extraction)    │
│  - render_pages (image rendering)    │
│  - Multi-process coordination        │
└────────────┬─────────────────────────┘
             │ FFI (bindgen)
             ▼
┌──────────────────────────────────────┐
│  PDFium Core (Google upstream)       │  ← 7f43fd79, no modifications
│  - libpdfium.dylib (shared)          │
└──────────────────────────────────────┘
```

---

## Core Components

### 1. Rust CLI Tools (Production)

**Location:** `rust/pdfium-sys/examples/`

#### extract_text.rs
- **Purpose:** High-performance text extraction
- **Features:**
  - UTF-32 LE text output (FPDFText_* APIs)
  - JSONL rich annotation output (bounding boxes, fonts, etc.)
  - Smart mode (auto-selects single/multi-process based on page count)
  - Explicit worker count (1-16 workers)
- **Threshold:** < 200 pages = single-threaded, ≥ 200 pages = multi-process
- **Performance:** 3.75x speedup at 4 workers (verified on 821-page PDF)
- **Correctness:** 100% byte-for-byte matching with upstream pdfium_test

#### render_pages.rs
- **Purpose:** High-performance image rendering
- **Features:**
  - PNG output (RGBA, compressed)
  - PPM output (RGB, uncompressed, for validation)
  - Thumbnail mode (150 DPI)
  - Custom DPI (default 300)
  - MD5 hashing for correctness validation
  - Smart mode (auto-selects single/multi-process)
- **Threshold:** < 200 pages = single-threaded, ≥ 200 pages = multi-process
- **Performance:** 3.95x speedup at 4 workers (verified on 821-page PDF)
- **Correctness:** 100% PPM MD5 matching with upstream pdfium_test (196/196 PDFs)

### 2. Multi-Process Parallelism

**Why Multi-Process (not threads)?**
- PDFium thread safety constraint: "only a single PDFium call can be made at a time per instance"
- Thread-based parallelism with mutex: 1.87x speedup (serialization bottleneck)
- Multi-process parallelism: 3.34x - 3.95x speedup (true parallelism)

**Implementation:**
1. Main process loads PDF, counts pages
2. Spawns N worker processes (default 4)
3. Distributes pages across workers
4. Workers process pages independently
5. Main process collects results

**Verified:** 100% correctness maintained (byte-for-byte identical to single-threaded)

### 3. PDFium Integration

**Binding Generation:** `rust/pdfium-sys/build.rs`
- Uses `bindgen` to generate Rust FFI bindings from PDFium C headers
- No manual FFI declarations

**PDFium Headers Used:**
- fpdf_text.h (text extraction)
- fpdf_edit.h (bitmap creation)
- fpdf_formfill.h (not currently used)
- fpdf_progressive.h (not currently used)

**Upstream Compatibility:**
- Zero modifications to PDFium source
- Binary: libpdfium.dylib (shared library)
- Build: GN + Ninja (standard PDFium build)

---

## Production Validation

### Test Coverage (356 tests total)

1. **Smoke Tests:** 38 tests (text + image + JSONL)
2. **Performance Tests:** 8 tests (speedup validation)
3. **Text Correctness:** 60 tests (byte-for-byte vs upstream)
4. **Image Correctness:** 196 tests (PPM MD5 vs upstream)
5. **Infrastructure:** 54 tests (baselines, manifests)

### Correctness Validation

**Text Extraction:**
- Method: `diff` command on UTF-32 LE output
- Baseline: Upstream pdfium_test
- Result: 60/60 PDFs byte-for-byte identical

**Image Rendering:**
- Method: PPM MD5 matching
- Baseline: Upstream pdfium_test (300 DPI, PPM format)
- Result: 196/196 PDFs byte-for-byte identical
- Note: PNG format not used for validation (compression/metadata differences)

**JSONL Rich Annotation:**
- Included in smoke tests
- No upstream baseline (custom feature)

### Performance Validation

**Requirements:**
- Large PDFs (≥ 200 pages): ≥ 2.0x speedup at 4 workers
- Small PDFs (< 200 pages): No requirement (overhead dominates)

**Verified Results:**
- Text extraction: 3.75x speedup at 4 workers (821-page PDF)
- Image rendering: 3.95x speedup at 4 workers (821-page PDF)

**Environmental Factors:**
- System load > 10.0 can cause performance variance
- Historical telemetry used to distinguish variance from regression

---

## Build System

### PDFium Build (C++)
```bash
# From upstream: https://pdfium.googlesource.com/pdfium/
# Commit: 7f43fd79 (2025-10-30)
# Binary MD5: 00cd20f999bf60b1f779249dbec8ceaa

gn gen out/Optimized-Shared
ninja -C out/Optimized-Shared pdfium
```

**Build Config (args.gn):**
```gn
is_debug = false
symbol_level = 0
optimize_for_size = false
is_component_build = true  # Shared library
pdf_enable_xfa = false
pdf_enable_v8 = false
```

### Rust Build
```bash
cd rust/pdfium-sys
cargo build --release --examples

# Produces:
# target/release/examples/extract_text
# target/release/examples/render_pages
```

---

## API Modes

### Bulk Mode (Default)
- Single-threaded execution
- Safe for parallel document processing
- Use when processing many small PDFs in parallel

### Fast Mode
- Multi-process execution (up to 16 workers)
- Use for single large PDF
- Explicit: `--workers N`
- Auto: Smart mode selects based on page count

### Smart Mode (Recommended)
- Automatically selects bulk or fast based on page count
- Threshold: 200 pages
- Balances performance vs overhead

---

## File Formats

### Text Extraction

**UTF-32 LE (default):**
- FPDFText_GetTextUTF8() → UTF-8 → UTF-32 LE
- 4 bytes per character (fixed width)
- BOM: FF FE 00 00
- Validation: `diff` command

**JSONL (--jsonl):**
- One JSON object per line
- Fields: text, bbox, font, size, page
- Rich annotation for downstream processing

### Image Rendering

**PNG (production):**
- RGBA format (32-bit)
- Compressed
- Standard tooling support

**PPM (validation):**
- RGB format (24-bit)
- Uncompressed
- Byte-for-byte matching with upstream

---

## Known Limitations

1. **Thread Safety:** PDFium constraint prevents true multi-threaded parallelism within single process
2. **Small PDFs:** Process spawn overhead (50-100ms) makes single-threaded faster for < 200 pages
3. **Forms:** Not yet tested/validated (form-heavy PDFs work but not explicitly validated)

---

## Future Work (Deferred)

### Debug Mode (Designed but not implemented)
- Detailed tracing for development
- Performance profiling
- Error diagnostics
- See: `reports/multi-thread-and-optimize/debug_mode_design.md`
- Current workaround: `RUST_LOG=debug`, `--workers 1`, `pytest -v`

### C++ CLI Wrapper (Not Implemented)
- Old design (rust/archive/API_DESIGN_OBSOLETE_2024-11-04.md) proposed Rust → C++ CLI → PDFium
- Never implemented
- Current approach (Rust → PDFium FFI) is production-ready and validated

---

## References

**Upstream:**
- PDFium: https://pdfium.googlesource.com/pdfium/
- Commit: 7f43fd79 (2025-10-30)

**Documentation:**
- CLAUDE.md: Project instructions and protocols
- reports/multi-thread-and-optimize/: Design decisions and investigations

**Test Results:**
- integration_tests/telemetry/runs.csv: 37796+ test runs logged
- Session IDs include timestamp, binary MD5, full traceability

**Git Tags:**
- v1.1.0: Production release (commit # 252, 2025-11-05)
