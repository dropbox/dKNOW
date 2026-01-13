# Release Notes: v1.9.0

**Release Date**: 2025-11-21
**Branch**: feature/v1.7.0-implementation
**Status**: Production-ready

## Summary

v1.9.0 delivers memory bandwidth optimization through BGR color format and improved user experience with smart rendering presets. 100% correctness maintained across all 2,787 tests. Speed remains at 72x baseline (unchanged from v1.6.0).

## New Features

### 1. BGR Color Format (Memory Optimization)

**What**: Automatic 3-byte BGR format for opaque pages, 4-byte BGRA only when transparency detected

**Benefits**:
- 25% less memory bandwidth (3 vs 4 bytes per pixel)
- Applies to 95%+ of typical document PDFs (opaque pages)
- No measurable performance improvement (speed neutral at 0.976x)

**Technical Details**:
- Uses `FPDFBitmap_BGR` for opaque pages (detected via `FPDFPage_HasTransparency()`)
- Falls back to `FPDFBitmap_BGRA` for transparent pages
- Transparent detection is automatic and requires no user configuration

**Implementation**: examples/pdfium_cli.cpp:2152-2168 (N=41)

### 2. Smart Rendering Presets (UX Improvement)

**What**: Simple `--preset` flag with smart defaults for common use cases

**Presets**:
- `--preset web`: 150 DPI, JPEG q85 (80% less memory, 84x smaller output)
- `--preset thumbnail`: 72 DPI, JPEG q80 (94% less memory, 280x smaller output)
- `--preset print`: 300 DPI, PNG (high quality, default)

**Benefits**:
- Simpler CLI interface (one flag vs three)
- Optimized defaults for each use case
- Explicit quality/speed tradeoffs

**Example**:
```bash
# Before (complex)
pdfium_cli --dpi 150 --format jpg --quality 85 render-pages input.pdf output/

# After (simple)
pdfium_cli --preset web render-pages input.pdf output/
```

**Implementation**: examples/pdfium_cli.cpp:197-218, 1024-1043 (N=43)

## Performance

### Speed (vs upstream PDFium)

**Baseline**: 72x speedup (unchanged from v1.6.0-v1.9.0)
**Smart mode**: 545x speedup for JPEG scanned PDFs (N=522 feature)

**v1.9.0 BGR mode**: No measurable speed improvement
- Measured: 0.976x (2.4% slower, within measurement variance)
- Reason: System is memory-bound (N=343 profiling analysis)
- Benefit: 25% less memory bandwidth (not speed)

### Memory & Disk Space Savings

**Memory savings** (at lower DPI):
- 300 → 150 DPI: 80% less memory (972 MB → 191 MB)
- 300 → 72 DPI: 94% less memory (972 MB → 60 MB)

**Disk space savings** (JPEG format):
- PNG → JPEG at 150 DPI: 84x smaller (3.1 GB → 37 MB per 100 pages)
- PNG → JPEG at 72 DPI: 280x smaller (3.1 GB → 11 MB per 100 pages)

**Real-world impact**: 100K PDFs at 100 pages each
- 300 DPI PNG: 3.1 TB storage required
- 150 DPI JPEG (web preset): 37 GB storage (84x savings)
- 72 DPI JPEG (thumbnail preset): 11 GB storage (280x savings)

See: EXTRACTING_100K_PDFS.md for complete analysis

## Test Status

**Full test suite**: 2,787/2,787 pass (100%)
**Smoke tests**: 92/92 pass (100%)
**Session**: sess_20251121_031227_5919d350
**Timestamp**: 2025-11-21T03:12:27Z

**Correctness**: 100% byte-for-byte match with upstream PDFium on 452 baseline PDFs

## Breaking Changes

None. v1.9.0 is fully backward-compatible with v1.8.0.

## Known Limitations

### jemalloc Not Included

**Original plan**: jemalloc memory allocator for 2-5% additional gain
**Status**: BLOCKED (allocator conflict with Xcode SDK 15.2)
**Impact**: Missing 2-5% potential performance gain

**Details**: See reports/feature-v1.7.0-implementation/jemalloc_blocker_2025-11-20.md

**Future work**: Revisit in v2.0.0 with LD_PRELOAD approach or SDK upgrade

## Migration Guide

No changes required. v1.9.0 is a drop-in replacement for v1.8.0.

**Optional**: Use new `--preset` flag for simpler CLI:
```bash
# Old (still works)
pdfium_cli --dpi 150 render-pages input.pdf output/

# New (recommended)
pdfium_cli --preset web render-pages input.pdf output/
```

## Build Instructions

```bash
# Build C++ CLI
ninja -C out/Release pdfium_cli

# Optional: Build Rust bindings
cd rust && cargo build --release
```

**Binary locations**:
- CLI: `out/Release/pdfium_cli`
- Library: `out/Release/libpdfium.dylib`

## Platform Support

**Validated**: macOS ARM64 (Apple Silicon M-series)
**Status**: Production-ready on validated platform only

**Future platforms**: Linux x86_64, macOS x86_64 (not yet validated)

## What's Next?

**v2.0.0 candidate features**:
1. Revisit jemalloc with LD_PRELOAD approach
2. Parallel text extraction (currently single-threaded)
3. Batch progress aggregation
4. Cross-platform validation (Linux, macOS x86_64)

## Contributors

**Development**: WORKER0 (Claude Code AI)
**Project**: Andrew Yates, Dropbox Dash
**Base**: Google PDFium (commit 7f43fd79, 2025-10-30)

## References

- **BGR implementation**: Git commit 0b5a3f85c3 (N=41)
- **Smart presets**: Git commit 1d58eca0c2 (N=43)
- **jemalloc blocker**: Git commit 5792cfe6bd (N=42)
- **Performance analysis**: reports/feature-v1.7.0-implementation/v1.9.0_performance_analysis_2025-11-21.md
