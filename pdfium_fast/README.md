<div align="center">

  # pdfium_fast

  **Fast PDF processing for large-scale batch jobs**

  Part of the Dropbox **Dash** media processing system

  **Copyright © 2025 Andrew Yates. All rights reserved.**

  [![Tests](https://img.shields.io/badge/tests-2339_passed-brightgreen)]()
  [![Correctness](https://img.shields.io/badge/correctness-100%25-success)]()
  [![Version](https://img.shields.io/badge/version-v2.0.0-blue)]()
  [![Platform](https://img.shields.io/badge/tested-macOS_ARM64-lightgrey)]()

</div>

---

## What is pdfium_fast?

**pdfium_fast** is a performance-optimized fork of Google's PDFium library, designed for processing large batches of PDFs efficiently.

**The Problem**: Processing 100,000 PDFs with standard tools takes weeks and generates terabytes of output.

**The Solution**: Algorithmic optimizations (11x faster single-threaded) + multi-core parallelization (up to 6.5x additional) = **72x faster image rendering** compared to upstream PDFium single-threaded baseline on large PDFs with 8 cores.

**Testing**: Validated on macOS ARM64 (Apple Silicon). Expected to work on other platforms but not yet tested.

**Forked from**: [PDFium](https://pdfium.googlesource.com/pdfium/) commit `7f43fd79` (2025-10-30) with cherry-picked bug fixes:
- 3 JBIG2 decoder fixes (files with >4 referred-to segments, progressive template 1 decoding, huffman symbol dictionaries)
- 1 AGG dashed line rendering fix

---

## Performance Overview

**All speedups measured against upstream PDFium single-threaded (commit 7f43fd79) on macOS ARM64.**

### Image Rendering Performance

**Test case**: 201-page production PDF on Apple Silicon M-series

| Configuration | ms/page | Total Time | Speedup vs Upstream | Components |
|---------------|---------|------------|---------------------|------------|
| **Upstream PDFium** (K=1) | 211 ms | 42.4s | 1.0x | baseline |
| **pdfium_fast** (K=1) | 19 ms | 3.9s | **11x** | algorithmic only |
| **pdfium_fast** (K=4) | 5 ms | 1.1s | **40x** | 11x algo × 3.6x threading |
| **pdfium_fast** (K=8) | 3 ms | 0.6s | **72x** | 11x algo × 6.5x threading |

**Key Insight**: 11x comes from PNG compression optimization (single-threaded). Threading adds 3.6x-6.5x on top for large PDFs.

### Text Extraction Performance

**Test case**: Documents >200 pages with K=4 workers on Apple Silicon M-series

| Configuration | ms/page | Speedup vs Upstream | Components |
|---------------|---------|---------------------|------------|
| **Upstream PDFium** (K=1) | ~3.7 ms | 1.0x | baseline |
| **pdfium_fast** (K=1) | ~3.7 ms | **1.0x** | no algorithmic improvement |
| **pdfium_fast** (K=4) | ~1.2 ms | **3.1x** | threading only (large PDFs) |

**Key Insight**: Text extraction has no single-threaded optimization. Speedup comes entirely from parallelization on large documents.

### JPEG Fast Path (Special Case)

**Speedup**: 545x vs upstream single-threaded **under specific conditions**:
- PDF contains embedded JPEG images (not PNG/TIFF)
- Single JPEG per page covering ≥95% of page area
- No vector graphics overlay

**How often**: ~10-15% of scanned documents meet these strict criteria

**How it works**: Extracts original JPEG directly from PDF stream, bypassing rendering entirely

---

## Performance: Component Breakdown

Understanding what contributes to speedup:

### 1. Algorithmic Optimization (Image Rendering Only)

**Speedup**: 11x vs upstream single-threaded
**Source**: PNG compression settings
**Change**: `Z_DEFAULT_COMPRESSION` → `Z_NO_COMPRESSION` + `PNG_FILTER_NONE`
**Trade-off**: 3-4x larger PNG files (acceptable for intermediate output)
**Applies to**: All image rendering, all platforms, K=1
**Does NOT apply to**: Text extraction (no single-threaded improvement)

### 2. Multi-Core Parallelization

**Speedup**: 3.6x-6.5x additional (on top of algorithmic gains)
**Source**: Thread-based parallelism with page-level work distribution
**Scaling**: Near-linear on large PDFs (>200 pages)
**Platform**: Thread safety validated on macOS ARM64 (2,339 tests, 100% pass)

**Image rendering scaling**:
- K=4: 3.6x additional (40x total vs upstream)
- K=8: 6.5x additional (72x total vs upstream)

**Text extraction scaling** (large PDFs >200 pages):
- K=4: 3.1x vs single-threaded

**When threading doesn't help**:
- Small PDFs (<50 pages): Process overhead exceeds parallelism benefit
- High system load (>10.0): Expect 50-65% performance degradation

### 3. Output Format Optimizations

**Not a speedup** - These reduce storage/memory, not processing time:

**JPEG output** (vs PNG):
- Storage: 88x smaller (3.2 GB → 37 MB per 100 pages at 150 DPI JPEG q85)
- Speed: No change (memory-bound system)
- Use for: Web preview, thumbnails, ML datasets

**Lower DPI**:
- Memory: 80% less (300→150 DPI) or 94% less (300→72 DPI)
- Speed: No change (memory-bound system)
- Use for: Thumbnails, lower-resolution outputs

---

## When to Use pdfium_fast

### ✅ Use pdfium_fast if:

- Processing **100+ PDFs** in batch jobs
- Need **image rendering** (biggest gains: 11x-72x vs upstream)
- Have **multi-core system** (4-8 cores for best threading gains)
- Can build from source (no pre-built binaries available yet)
- Need output format flexibility (JPEG, multiple DPI options)

### 💡 Platform Status

**Tested**: macOS 15.6 (Apple Silicon M-series)
**Expected to work**: Linux x86_64, macOS Intel, other Unix-like systems
**Not tested**: Windows, Linux ARM64
**Your results may vary** based on platform, PDF complexity, system load

---

## Quick Start

### Build from Source (macOS/Linux)

**Requirements**: 10GB disk, 60-90 minutes for first build

```bash
# 1. Install Chromium's depot_tools
git clone https://chromium.googlesource.com/chromium/tools/depot_tools.git
export PATH="$PWD/depot_tools:$PATH"

# 2. Clone and build
git clone https://github.com/dropbox/dash-pdf-extraction.git
cd dash-pdf-extraction
./setup.sh  # Downloads 7.2GB dependencies + builds

# 3. Verify
./out/Release/pdfium_cli --help
```

### Docker Build (Linux Binary)

```bash
# Build Linux binary via Docker (works on any platform with Docker)
./build-linux.sh --docker

# Test
./binaries/linux/pdfium_cli --help
```

See [LINUX_BUILD.md](LINUX_BUILD.md) for details.

### Basic Usage

```bash
# Extract text (single-threaded, good for small PDFs)
./out/Release/pdfium_cli extract-text document.pdf output.txt

# Extract text (4 workers, best for large PDFs >200 pages)
./out/Release/pdfium_cli --workers 4 extract-text large.pdf output.txt

# Render images (single-threaded: 11x vs upstream)
./out/Release/pdfium_cli render-pages document.pdf images/

# Render images (8 threads: 72x vs upstream on large PDFs)
./out/Release/pdfium_cli --threads 8 render-pages document.pdf images/

# Web preview preset (150 DPI JPEG, 80% less memory)
./out/Release/pdfium_cli --preset web render-pages document.pdf images/

# Thumbnail preset (72 DPI JPEG, 94% less memory)
./out/Release/pdfium_cli --preset thumbnail render-pages document.pdf images/
```

---

## Features

### 🚀 Fast Image Rendering

**11x faster single-threaded** vs upstream PDFium through PNG compression optimization. No multi-core required.

```bash
# Single-threaded: 11x speedup vs upstream
./out/Release/pdfium_cli render-pages document.pdf images/
```

### ⚡ Multi-Core Parallelization

**Up to 72x faster** (11x algorithmic × 6.5x threading) on large PDFs with 8 cores.

```bash
--threads 1   # Single-threaded: 11x vs upstream
--threads 4   # 4 cores: 40x vs upstream (recommended)
--threads 8   # 8 cores: 72x vs upstream (large PDFs)
```

**Note**: Threading gains require large PDFs (>200 pages). Small PDFs (<50 pages) get minimal benefit due to overhead.

### 📦 Smart Presets

Simple one-flag configuration for common use cases:

```bash
--preset web        # 150 DPI JPEG q85 (web preview, 80% less memory)
--preset thumbnail  # 72 DPI JPEG q80 (thumbnails, 94% less memory)
--preset print      # 300 DPI PNG (full quality, default)
```

### 🎚️ Output Format Control

**JPEG output**: 88x smaller storage vs uncompressed PNG

```bash
--format jpg        # JPEG output (default quality 90)
--jpeg-quality 85   # Adjust quality (0-100)
--dpi 150           # Lower resolution (80% less memory vs 300 DPI)
--dpi 72            # Thumbnail resolution (94% less memory)
```

**Use case**: Processing 100K PDFs avoided 4.5 TB storage issue by using JPEG output.

### 📦 Batch Processing

Process entire directories efficiently:

```bash
--batch                      # Process directory of PDFs
--recursive                  # Recurse into subdirectories
--pattern "report_*.pdf"     # Filter by filename pattern
```

### 🔬 JPEG Fast Path (Automatic)

**545x speedup** when PDF meets strict criteria (single JPEG per page, ≥95% coverage). Automatic detection, no flags needed.

### ✅ Validated Correctness

**100% correctness** on 452-PDF test corpus:
- Byte-for-byte identical text extraction vs upstream
- Pixel-perfect image rendering (PPM MD5 validation)
- 2,339 tests, 100% pass rate

---

## Large-Scale Usage (100,000+ PDFs)

### Text Extraction

```bash
# Extract text from 100K PDFs with 4 workers
./out/Release/pdfium_cli --batch --recursive --workers 4 \
  extract-text /pdf_corpus/ /text_output/

# Expected performance (extrapolated from 169K corpus test):
# • Time: ~1-2 hours (varies with PDF size)
# • Success: ~93% (corrupt/malformed PDFs fail gracefully)
# • Output: ~22 GB text files
# • Memory: 2 GB (4 workers × 500 MB each)
```

### Image Rendering (Web Preview)

```bash
# Render 100K PDFs as JPEG with 8 threads per PDF
./out/Release/pdfium_cli --batch --recursive --preset web \
  render-pages /pdf_corpus/ /images/

# Expected performance:
# • Time: Several hours (rendering is slower than text extraction)
# • Output: ~37 GB JPEG (vs 3.1 TB for 300 DPI uncompressed PNG)
# • Storage savings: 88x smaller
# • Memory: 191 MB per PDF (vs 972 MB at 300 DPI)
```

### Thumbnails

```bash
# Generate thumbnails for 100K PDFs
./out/Release/pdfium_cli --batch --recursive --preset thumbnail \
  render-pages /pdf_corpus/ /thumbnails/

# Expected performance:
# • Output: ~11 GB JPEG
# • Storage savings: 282x smaller vs 300 DPI PNG
# • Memory: 60 MB per PDF (94% savings vs 300 DPI)
```

**Critical**: Use `--preset web` or `--format jpg` for large-scale image extraction. Default PNG settings create impractically large outputs.

---

## API & Command Reference

### C++ CLI (Primary Interface)

```bash
pdfium_cli [FLAGS] <OPERATION> <INPUT> <OUTPUT>

# Core Operations
extract-text      # Extract plain text (UTF-8)
extract-jsonl     # Extract text + metadata (positions, fonts)
render-pages      # Render to images (PNG/JPEG/PPM)

# Parallelism
--workers N       # Multi-process (1-16, default 1, for text extraction)
--threads K       # Multi-threaded (1-32, default 8, for image rendering)

# Output Control
--preset MODE     # web|thumbnail|print (simple presets)
--format FMT      # png|jpg|jpeg|ppm (default: png)
--jpeg-quality N  # JPEG quality 0-100 (default: 90)
--dpi N           # Resolution 72-600 (default: 300)

# Batch Processing
--batch           # Process directory of PDFs
--recursive       # Recurse into subdirectories
--pattern GLOB    # Filename filter (default: *.pdf)

# Utilities
--pages START-END # Process page range (e.g., --pages 10-50)
--debug           # Enable detailed tracing
--benchmark       # Skip file writes (performance testing)
```

### Rust Bindings (Optional)

For programmatic/library access (alternative to subprocess invocation of C++ CLI):

```bash
cd rust
cargo build --release

# Examples available
./target/release/examples/extract_text document.pdf output.txt
./target/release/examples/render_pages document.pdf images/
./target/release/examples/extract_text_jsonl document.pdf output.jsonl
```

**Note**: Both C++ CLI and Rust bindings produce identical output. Choose based on integration needs.

---

## Testing & Validation

### Test Coverage

**Total**: 2,339 tests, 100% pass rate (v2.0.0)

**Test breakdown**:
- **1,356 PDF tests** (452 PDFs × 3: text + JSONL + image)
- **254 edge cases** (malformed/encrypted PDFs, no-crash validation)
- **149 infrastructure tests** (baseline/binary validation)
- **70 smoke tests** (7-minute quick validation)
- **18 performance tests** (speedup requirements)
- **18 scaling tests** (1/2/4/8 worker analysis)

### Correctness Methodology

**Image rendering**:
- Method: Byte-for-byte MD5 comparison vs upstream `pdfium_test`
- Format: PPM (P6 binary RGB) for exact matching
- Coverage: 452 PDFs, pixel-perfect accuracy

**Text extraction**:
- Method: Byte-for-byte text comparison vs upstream
- Coverage: 452 PDFs, character-level accuracy

**JSONL metadata**:
- Method: Numerical comparison of character positions, fonts, bounding boxes
- Coverage: 452 PDFs (first page per PDF)

### Test Corpus

**452 benchmark PDFs**:
- arXiv papers (40) - Scientific publications
- EDINET financial (50) - Japanese financial documents
- Common Crawl (20) - Web-crawled documents
- Web documents (45) - Real-world content
- Dropbox internal (41) - Production use cases
- Edge cases (256) - Malformed, encrypted, special features

### Running Tests

```bash
# Install test dependencies
cd integration_tests
pip install -r requirements.txt

# Download test PDFs (requires repo access)
gh release download test-pdfs-v1 --pattern "*.tar.gz"
tar xzf pdfium_test_pdfs.tar.gz

# Quick validation (7 minutes)
pytest -m smoke

# Full corpus (24 minutes)
pytest -m corpus

# Complete suite (1h 46m)
pytest
```

---

## Architecture & Technical Details

### System Design

```
User Application
      ↓
C++ CLI (pdfium_cli) ← Primary interface
      ↓
Rust Bindings (optional) ← Programmatic API wrapper
      ↓
PDFium Core (fork of commit 7f43fd79)
  • PNG optimization (11x)
  • Threading infrastructure
  • JPEG fast path
```

### Key Optimizations

**1. PNG Compression (11x for image rendering)**
- Change: `Z_DEFAULT_COMPRESSION` → `Z_NO_COMPRESSION`
- Added: `PNG_FILTER_NONE` for raw pixel data
- Trade-off: 3-4x larger PNG files
- Impact: Single-threaded image rendering only
- File: Core PNG writing code

**2. Multi-Threading (3.6x-6.5x additional)**
- Pre-loading phase: Populate caches sequentially
- Parallel phase: Thread-based page rendering
- Mutex protection: Two-layer (cache access + page load)
- Stability: 100% success rate over 200 validation runs at K=4 and K=8
- File: `examples/pdfium_cli.cpp`, `fpdfsdk/fpdf_parallel.cpp`

**3. JPEG Fast Path (545x for specific PDFs)**
- Detection: Single JPEG per page, ≥95% coverage
- Method: Extract JPEG stream directly, bypass rendering
- Quality: Zero loss (original JPEG preserved)
- Activation: ~10-15% of scanned documents
- File: `examples/pdfium_cli.cpp` (smart mode detection)

**4. Multi-Process Parallelism (3.1x for text extraction)**
- Process-based with page-range splitting
- Zero contention (isolated address spaces)
- Best for: Large PDFs (>200 pages) with K=4 workers
- File: `examples/pdfium_cli.cpp` (worker pool)

**5. Streaming Architecture (Memory-Efficient)**
- On-demand page loading: `FPDF_LoadPage` → process → `FPDF_ClosePage`
- Memory scales with page complexity, not document size
- Enables processing arbitrarily large PDFs

**Performance limits**: Memory-bound system (validated via profiling). CPU optimizations yield <2% gains due to memory bandwidth bottleneck.

---

## Version History

### v2.0.0 (Current - 2025-12-26)

**Status**: Production-ready (537 iterations of continuous validation)

**Features**:
- 11x single-threaded image rendering (PNG optimization)
- Up to 72x with multi-threading (K=8)
- Smart presets (web/thumbnail/print)
- JPEG output format (88x storage savings)
- DPI control (72-600 DPI)
- Batch processing (directories)
- Progress reporting with ETA
- Multi-process text extraction (K=4: 3.1x)
- JPEG fast path (545x for scanned PDFs)

**Testing**: 2,339 tests, 100% pass rate
**Platform**: Validated on macOS 15.6 ARM64
**Correctness**: 100% on 452-PDF test corpus

### Previous Releases

- **v1.9.0** (2025-11-21): Smart presets, BGR memory optimization
- **v1.8.0** (2025-11-21): DPI control, async I/O
- **v1.7.0** (2025-11-18): JPEG output, Linux build infrastructure
- **v1.6.0** (2025-11-20): Progress reporting, batch mode, 72x speedup
- **v1.0.0** (2025-11-08): Initial production release

---

## Building from Source

### Prerequisites

- **Platform**: macOS 12+ or Linux (Ubuntu 20.04+)
- **Disk**: 10GB free space
- **Time**: 60-90 minutes for first build
- **Tools**: Chromium's depot_tools (gn, ninja, gclient)

### Automated Build (Recommended)

```bash
# 1. Install depot_tools
git clone https://chromium.googlesource.com/chromium/tools/depot_tools.git
export PATH="$PWD/depot_tools:$PATH"
# Add to ~/.bashrc or ~/.zshrc for persistence

# 2. Clone and build
git clone https://github.com/dropbox/dash-pdf-extraction.git
cd dash-pdf-extraction
./setup.sh  # Handles deps download + configure + build

# 3. Verify
./out/Release/pdfium_cli --help
```

### Manual Build

```bash
# 1. Clone repository
git clone https://github.com/dropbox/dash-pdf-extraction.git

# 2. Create .gclient in PARENT directory
cd ..  # Go to parent of dash-pdf-extraction/
cat > .gclient << 'EOF'
solutions = [{
  "name": "dash-pdf-extraction",
  "url": "https://github.com/dropbox/dash-pdf-extraction.git",
  "managed": False,
}]
EOF

# 3. Download dependencies (7.2GB, 30-60 min)
gclient sync

# 4. Build (20-40 min)
cd dash-pdf-extraction/
gn gen out/Release --args='is_debug=false pdf_enable_v8=false pdf_enable_xfa=false'
ninja -C out/Release pdfium_cli
```

### Building Library for Downstream Consumers (libpdfium_render_bridge.dylib)

**When needed**: Embedding in Rust crates (docling_rs, pdfium-render) or other programmatic consumers that need to link against libpdfium.

**Requirements**:
- Monolithic build (no @rpath dependencies)
- Exported symbols (global 'T' visibility for dlopen/dlsym)

**Build Steps**:

```bash
# 1. Create .gclient in PARENT directory with minimal checkout
cd ~/  # Parent of pdfium_fast/
cat > .gclient << 'EOF'
solutions = [
  {
    "name": "pdfium_fast",
    "url": "https://github.com/ayates_dbx/pdfium_fast.git",
    "managed": False,
    "custom_vars": {
      "checkout_configuration": "minimal",
    },
  },
]
EOF

# 2. Sync dependencies
gclient sync

# 3. Configure for monolithic build
cd pdfium_fast
gn gen out/Release --args='pdf_is_standalone=true is_debug=false is_component_build=false pdf_enable_v8=false pdf_enable_xfa=false pdf_use_skia=false'

# 4. Build the library
ninja -C out/Release libpdfium_render_bridge.dylib

# 5. Verify monolithic (only system deps, no @rpath)
otool -L out/Release/libpdfium_render_bridge.dylib

# 6. Verify symbols exported (should show 'T' for global)
nm -gU out/Release/libpdfium_render_bridge.dylib | grep -E "FPDF_Init|FPDFText_ExtractAllCells"
```

**Key Build Flags**:
- `is_component_build=false` - Monolithic build (no @rpath dependencies)
- `pdf_use_skia=false` - Avoids skia dependency issues with minimal checkout
- `checkout_configuration: "minimal"` - Reduces dependencies from 7.2GB to ~2GB

**Custom Symbols**: pdfium_fast exports additional functions not in vanilla PDFium:
- `FPDFText_ExtractAllCells` - Batch text extraction
- `FPDF_DestroyThreadPool` - Thread pool cleanup

These require the visibility fix in `public/fpdfview.h` (included since v2.1.1).

### Building Rust Bindings (Optional)

**When needed**: Programmatic/library access (not required for CLI use)

```bash
cd rust
cargo build --release
```

**Known issue (macOS SDK 15.2)**: Xcode 16.2+ ships SDK 15.2 which blocks Rust builds. Workaround: Use C++ CLI exclusively (all features available). See [SDK_COMPATIBILITY.md](SDK_COMPATIBILITY.md) for details.

---

## Usage Examples

### Single PDF Processing

```bash
# Text extraction (single-threaded)
./out/Release/pdfium_cli extract-text document.pdf output.txt

# Text extraction (4 workers, large PDFs >200 pages)
./out/Release/pdfium_cli --workers 4 extract-text large.pdf output.txt

# Image rendering (single-threaded: 11x vs upstream)
./out/Release/pdfium_cli render-pages document.pdf images/

# Image rendering (8 threads: 72x vs upstream on large PDFs)
./out/Release/pdfium_cli --threads 8 render-pages document.pdf images/

# Specific page range
./out/Release/pdfium_cli --threads 8 --pages 10-50 render-pages doc.pdf images/
```

### Output Format Options

```bash
# JPEG output (88x storage savings vs uncompressed PNG)
./out/Release/pdfium_cli --format jpg render-pages document.pdf images/

# JPEG with quality control
./out/Release/pdfium_cli --format jpg --jpeg-quality 85 render-pages doc.pdf images/

# Lower DPI for web preview (80% less memory)
./out/Release/pdfium_cli --dpi 150 render-pages document.pdf images/

# Thumbnail DPI (94% less memory)
./out/Release/pdfium_cli --dpi 72 render-pages document.pdf images/

# Smart presets (recommended)
./out/Release/pdfium_cli --preset web render-pages document.pdf images/
./out/Release/pdfium_cli --preset thumbnail render-pages document.pdf images/
./out/Release/pdfium_cli --preset print render-pages document.pdf images/
```

### Batch Processing

```bash
# Process all PDFs in directory
./out/Release/pdfium_cli --batch render-pages input_dir/ output_dir/

# Recursive processing
./out/Release/pdfium_cli --batch --recursive render-pages project/ output/

# Pattern filtering
./out/Release/pdfium_cli --batch --pattern "report_*.pdf" render-pages dir/ output/

# Batch text extraction with 4 workers
./out/Release/pdfium_cli --batch --recursive --workers 4 \
  extract-text /pdfs/ /text_output/
```

### JSONL Metadata Extraction

```bash
# Character positions, fonts, bounding boxes (C++ CLI)
./out/Release/pdfium_cli extract-jsonl document.pdf output.jsonl

# Or use Rust tool (alternative)
./rust/target/release/examples/extract_text_jsonl document.pdf output.jsonl 0
```

### Debug Mode

```bash
# Enable detailed tracing for troubleshooting
./out/Release/pdfium_cli --debug extract-text problematic.pdf output.txt
./out/Release/pdfium_cli --debug --threads 8 render-pages test.pdf images/
```

---

## Contributing

Dash PDF Extraction is an internal Dropbox project. External contributions are not currently accepted.

**For Dropbox Engineers**:
- Review [CLAUDE.md](CLAUDE.md) for development protocols
- Run full test suite before committing: `pytest` (1h 46m)
- Quick validation: `pytest -m smoke` (7 min)

**Development workflow**:
```bash
# 1. Make changes
vim examples/pdfium_cli.cpp

# 2. Build
ninja -C out/Release pdfium_cli

# 3. Test
cd integration_tests
pytest -m smoke  # Quick check
pytest -m corpus # Full corpus

# 4. Commit
git add -A
git commit -m "Description of changes"
```

---

## License & Copyright

**Copyright © 2025 Andrew Yates. All rights reserved.**

Dash PDF Extraction is an internal Dropbox project. No external license granted.

**Based on PDFium**: Copyright © 2014 The PDFium Authors, BSD-3-Clause License. See [LICENSE](LICENSE).

**Third-party dependencies**:
- PDFium (Google) - BSD-3-Clause
- FreeType - FreeType License
- libpng - PNG Reference Library License
- zlib - zlib License
- ICU - Unicode License

See [third_party/](third_party/) for complete licenses.

---

## Acknowledgments

**Primary Developer**: Andrew Yates (ayates@dropbox.com)
**Organization**: Dropbox Dash
**Development**: AI-assisted (Claude Code by Anthropic)

**Special thanks**:
- Google PDFium team for the upstream library
- Dropbox Dash team for project sponsorship
- Anthropic for Claude Code development tools

---

## Citation

If referencing this work in research or publications:

```bibtex
@software{yates2025pdfium_fast,
  title = {pdfium\_fast: High-Performance PDFium Fork for Batch Processing},
  author = {Yates, Andrew},
  organization = {Dropbox},
  year = {2025},
  version = {v2.0.0},
  url = {https://github.com/dropbox/dash-pdf-extraction},
  note = {11x single-threaded, up to 72x multi-threaded image rendering vs upstream PDFium}
}
```

---

<div align="center">

**pdfium_fast v2.0.0**

11x single-threaded • Up to 72x multi-threaded • 100% correctness validated

**Copyright © 2025 Andrew Yates • Dropbox Dash**

**Tested on**: macOS ARM64 (Apple Silicon) • Expected to work on other platforms

[Report Issues](https://github.com/dropbox/dash-pdf-extraction/issues) • [Releases](https://github.com/dropbox/dash-pdf-extraction/releases) • [Documentation](docs/)

</div>
