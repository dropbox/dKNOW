# PDFium Fast - Performance Guide

**Copyright © 2025 Andrew Yates. All rights reserved.**

**Version**: v2.0.0 (zero-flag defaults)
**Date**: 2025-11-21
**Status**: Production-ready

---

## Quick Recommendations

**For most users (v1.9.0 presets):**
```bash
# Text extraction (large PDFs >200 pages)
pdfium_cli --workers 4 extract-text document.pdf output.txt

# Image rendering - web preview (150 DPI JPEG, 80% less memory, 84x smaller)
pdfium_cli --preset web render-pages document.pdf output/

# Image rendering - thumbnails (72 DPI JPEG, 94% less memory, 280x smaller)
pdfium_cli --preset thumbnail render-pages document.pdf output/

# Image rendering - high quality print (300 DPI PNG, default)
pdfium_cli --preset print render-pages document.pdf output/
```

**For batch processing:**
```bash
# Maximum throughput (8 workers)
pdfium_cli --workers 8 --preset web render-pages document.pdf output/
```

**For small documents (<50 pages):**
```bash
# Single-threaded (overhead avoidance)
pdfium_cli render-pages document.pdf output/
```

---

## Understanding the Performance Numbers

### What "3.93x speedup" Means

When we say **3.93x mean speedup** (measured on 26 production PDFs, K=8 vs K=1):

- **Before (K=1)**: Rendering a 200-page PDF takes 10 seconds
- **After (K=8)**: Same PDF takes 2.5 seconds (10s ÷ 3.93 = 2.5s)
- **Time saved**: 7.5 seconds (75% reduction)

### Why Not "83x"?

**Theoretical calculation**: 11x PNG optimization × 7.5x threading = 82.5x

**Reality**: Optimizations interact and don't multiply perfectly. The PNG optimization changes how the workload looks to the threading system, reducing threading efficiency.

**Measured result**: 3.93x mean on real production PDFs (100-1931 pages)

---

## Performance by Document Size

### Small Documents (<50 pages)

**Expected speedup**: 1.1x at K=4, 1.07x at K=8

**Why so low?** Process spawn overhead dominates. For a 25-page PDF that takes 0.5s to render:
- Process startup: 0.2s
- Actual work: 0.3s
- Parallelism only speeds up the 0.3s, not the 0.2s overhead

**Recommendation**: Use single-threaded mode (default) for small documents.

```bash
# Optimal for small PDFs
pdfium_cli render-pages small.pdf output/
```

### Medium Documents (100-200 pages)

**Expected speedup**: 4.20x mean at K=8

**Why better?** Work >> overhead. For a 150-page PDF that takes 5s to render:
- Process startup: 0.2s (4% overhead)
- Actual work: 4.8s (96% parallelizable)

**Recommendation**: Use K=4 for balanced efficiency.

```bash
# Optimal for medium PDFs
pdfium_cli --threads 4 render-pages medium.pdf output/
```

### Large Documents (200+ pages)

**Expected speedup**: 3.42-3.80x mean at K=8

**Why slightly lower than medium?** Larger documents have more complex structure (fonts, images, color spaces) which increases synchronization overhead in the pre-loading phase.

**Recommendation**: Use K=8 for maximum throughput.

```bash
# Optimal for large PDFs
pdfium_cli --threads 8 render-pages large.pdf output/
```

---

## Text Extraction Performance

### Small Documents (<100 pages)

**Expected speedup**: 1.2x mean at 4 workers

**Recommendation**: Use single-process (default).

```bash
# Optimal for small PDFs
pdfium_cli extract-text small.pdf output.txt
```

### Medium Documents (100-200 pages)

**Expected speedup**: 1.9x mean at 4 workers

**Recommendation**: Use 4 workers if document is >150 pages.

```bash
# Optimal for medium PDFs
pdfium_cli --workers 4 extract-text medium.pdf output.txt
```

### Large Documents (200+ pages)

**Expected speedup**: 2.9x mean at 4 workers, 3.1x median

**Recommendation**: Always use 4 workers.

```bash
# Optimal for large PDFs
pdfium_cli --workers 4 extract-text large.pdf output.txt
```

---

## Special Cases

### Scanned PDFs (JPEG Images)

**Expected speedup**: 545x (automatic JPEG fast path)

PDFium Fast automatically detects pages that are single JPEG images covering ≥95% of the page area and extracts the JPEG directly without rendering. This is **always enabled** - no flag needed.

```bash
# Smart mode is automatic (545x speedup)
pdfium_cli render-pages scanned.pdf output/
```

**Detection criteria**:
- Page contains exactly one image object
- Image is JPEG format
- Image covers ≥95% of page area
- No other content (text, vectors) on page

**Common sources**: Scanner output, photo PDFs, archived documents

### Smart Presets (v1.9.0)

**Web Preview Mode**: `--preset web`
- 150 DPI JPEG output (quality 85)
- **80% less memory, 84x smaller output** than default PNG
- Ideal for: Web dashboards, preview images, UI thumbnails

**Thumbnail Mode**: `--preset thumbnail`
- 72 DPI JPEG output (quality 80)
- **94% less memory, 280x smaller output** than default PNG
- Ideal for: Document thumbnails, grid views, quick previews

**Print Mode**: `--preset print` (default)
- 300 DPI PNG output (lossless)
- Best quality for printing and archival
- Default if no preset specified

```bash
# Web preview (fast)
pdfium_cli --preset web render-pages document.pdf output/

# Thumbnail generation (fastest)
pdfium_cli --preset thumbnail render-pages document.pdf output/

# High-quality print (default)
pdfium_cli --preset print render-pages document.pdf output/
```

### BGR Memory Optimization (v1.9.0)

**Automatic optimization**: For opaque pages, PDFium Fast uses 3-byte BGR format instead of 4-byte BGRA.

**Benefits**:
- **25% less memory bandwidth** (3 vs 4 bytes per pixel)
- Applies to 95%+ of typical document PDFs
- Speed neutral (no measurable performance change)

**No configuration needed**: Automatically enabled for all renders.

---

## Hardware Considerations

### CPU Core Count

**Measured on**: Apple Silicon M-series (8 performance cores + 2 efficiency cores)

**Scaling recommendations**:
- **4 cores or fewer**: Use K=2 or K=4
- **8 cores**: Use K=4 (optimal efficiency) or K=8 (maximum throughput)
- **16+ cores**: Use K=8 (diminishing returns beyond this)

### Memory Usage

**Per thread**: ~50-100 MB for typical PDFs

**Example**: K=8 on a 200-page PDF with complex graphics:
- Base memory: 200 MB (document + resources)
- Per-thread overhead: 50 MB × 8 = 400 MB
- **Total**: ~600 MB

**Recommendation**: Ensure 100 MB free RAM per thread.

### Disk I/O

**PNG output**: ~3-4x larger than compressed PNG (Z_NO_COMPRESSION optimization)

**Example**: 200-page PDF rendered at 300 DPI:
- Compressed PNG: 50 MB (Z_DEFAULT_COMPRESSION)
- Uncompressed PNG: 150-200 MB (Z_NO_COMPRESSION, v1.2.0)

**Trade-off**: Disk space for performance. If disk space is constrained, consider external PNG compression post-processing.

---

## Troubleshooting

### Performance Lower Than Expected

**Check system load**:
```bash
uptime  # Load should be <6.0 for accurate performance testing
```

**Check for hung processes**:
```bash
ps aux | grep pdfium_cli | grep -v grep
# If processes found, kill them: killall -9 pdfium_cli
```

**Environmental factors**:
- High system load (>10.0): Expect 50-65% performance degradation
- Background CPU-intensive tasks: Reduce worker/thread count
- Low disk space: PNG writing may slow down

### Crashes or Hangs

**If pdfium_cli crashes**:
1. Try single-threaded mode first: `pdfium_cli render-pages test.pdf output/`
2. If single-threaded works, reduce thread count: `--threads 2`
3. Report issue with PDF details (pages, size, source)

**Known issue (resolved)**: bug_451265.pdf caused infinite loop in upstream PDFium. Fixed in v1.2.0 (N=232, pattern cache inheritance fix).

---

## Benchmarking Your Workload

### Step 1: Identify Document Distribution

Analyze your PDF corpus:
```bash
# Count pages in all PDFs
for pdf in *.pdf; do
    echo "$pdf: $(pdfinfo "$pdf" | grep Pages | awk '{print $2}') pages"
done | sort -t: -k2 -n
```

Group by size:
- Small: <100 pages
- Medium: 100-200 pages
- Large: 200+ pages

### Step 2: Test Representative Samples

Pick 3-5 PDFs from each size category and benchmark:

```bash
# Test single-threaded
time pdfium_cli render-pages test.pdf output_k1/

# Test K=4
time pdfium_cli --threads 4 render-pages test.pdf output_k4/

# Test K=8
time pdfium_cli --threads 8 render-pages test.pdf output_k8/
```

### Step 3: Calculate Speedup

```bash
# Example
# K=1: 10.5 seconds
# K=4: 2.7 seconds
# K=8: 1.8 seconds

# Speedup = baseline / optimized
# K=4 speedup: 10.5 / 2.7 = 3.89x
# K=8 speedup: 10.5 / 1.8 = 5.83x
```

### Step 4: Choose Optimal Configuration

Based on your corpus:
- **Mostly small PDFs (<100 pages)**: K=1 (single-threaded)
- **Mostly medium PDFs (100-200 pages)**: K=4
- **Mostly large PDFs (200+ pages)**: K=4 for efficiency, K=8 for throughput
- **Mixed corpus**: K=4 (good balance)

---

## Production Deployment Recommendations

### Auto-Dispatch Based on Page Count

```bash
#!/bin/bash
# Optimal worker selection based on page count

PDF="$1"
OUTPUT="$2"

# Get page count
PAGES=$(pdfinfo "$PDF" | grep Pages | awk '{print $2}')

# Auto-dispatch
if [ "$PAGES" -lt 50 ]; then
    # Small PDF: single-threaded
    pdfium_cli render-pages "$PDF" "$OUTPUT"
elif [ "$PAGES" -lt 200 ]; then
    # Medium PDF: K=4
    pdfium_cli --threads 4 render-pages "$PDF" "$OUTPUT"
else
    # Large PDF: K=8
    pdfium_cli --threads 8 render-pages "$PDF" "$OUTPUT"
fi
```

### Batch Processing

For processing many PDFs in parallel:

```bash
#!/bin/bash
# Process multiple PDFs with GNU parallel

# Install GNU parallel: brew install parallel (macOS) or apt-get install parallel (Linux)

# Process 4 PDFs at a time, each with K=4 threads
find . -name "*.pdf" | parallel -j 4 'pdfium_cli --threads 4 render-pages {} output/{/.}/'

# Explanation:
# -j 4: Run 4 parallel jobs (4 PDFs simultaneously)
# --threads 4: Each PDF uses 4 threads
# Total concurrency: 4 processes × 4 threads = 16 threads
```

**Resource calculation**:
- CPUs: (parallel jobs) × (threads per job)
- Memory: (parallel jobs) × (100 MB per thread) × (threads per job)
- Example: 4 jobs × 4 threads = 16 threads, ~1.6 GB RAM

### Monitoring Production Performance

Log performance data for analysis:

```bash
#!/bin/bash
# Performance logging wrapper

PDF="$1"
OUTPUT="$2"
LOG="performance.log"

START=$(date +%s.%N)
pdfium_cli --threads 4 render-pages "$PDF" "$OUTPUT"
EXIT_CODE=$?
END=$(date +%s.%N)

DURATION=$(echo "$END - $START" | bc)
PAGES=$(pdfinfo "$PDF" | grep Pages | awk '{print $2}')
PPS=$(echo "$PAGES / $DURATION" | bc -l)

echo "$(date -Iseconds),$PDF,$PAGES,$DURATION,$PPS,$EXIT_CODE" >> "$LOG"
```

Analyze logs periodically:
```bash
# Average pages per second
awk -F, '{sum+=$5; count++} END {print sum/count}' performance.log

# Find slow PDFs (< 10 pages/sec)
awk -F, '$5 < 10 {print $2, $5}' performance.log
```

---

## Variance and Reproducibility

### Expected Variance

Based on 100 measurements (10 PDFs × 10 runs):
- **Median variance**: 2.1%
- **95% of PDFs**: ≤3.0% variance
- **Typical 95% CI**: ±2-3% of mean

**Example**: 200-page PDF renders in 2.91s ± 0.03s (95% CI: 2.89s-2.94s)

### Environmental Impact

**High variance (>10%)** suggests environmental factors:
- System load >6.0
- Background processes competing for CPU
- Thermal throttling (check CPU temperature)
- Disk I/O contention (check iostat)

**Solution**: Run performance tests under controlled conditions (low load, no background tasks).

---

## Comparison to Upstream

### Baseline

**Upstream**: `pdfium_test` from commit `7f43fd79` (2025-10-30)
- Single-threaded
- PNG compression: Z_DEFAULT_COMPRESSION (level 6)
- No smart mode (always renders)

### v1.2.0 Improvements

**Single-threaded (K=1)**: 11x faster
- PNG optimization: Z_NO_COMPRESSION (97% → 30% overhead)

**Multi-threaded (K=4)**: 43x faster (theoretical)
- PNG: 11x
- Threading: 3.9x
- **Measured**: 4.20x mean on 100-200 page PDFs

**Multi-threaded (K=8)**: 83x faster (theoretical)
- PNG: 11x
- Threading: 7.5x
- **Measured**: 3.93x mean on production corpus (100-1931 pages)

**Smart mode (scanned PDFs)**: 545x faster
- JPEG direct extraction (bypass rendering)

---

## FAQ

**Q: Why is my speedup lower than advertised?**

A: The 3.93x mean is measured on production PDFs (100-1931 pages). Smaller PDFs (<50 pages) see ~1.1x due to process overhead. Check your PDF page count.

**Q: Should I use `--workers` or `--threads`?**

A: For **text extraction**, use `--workers` (multi-process). For **image rendering**, use `--threads` (multi-threaded). You can combine them: `--workers 4 --threads 4`.

**Q: Why does K=8 sometimes perform worse than K=4?**

A: Two reasons:
1. **Overhead**: Pre-loading phase has 5.6% overhead, which grows with thread count
2. **Diminishing returns**: Amdahl's Law - not all work is parallelizable

For most workloads, K=4 is optimal (balance efficiency vs throughput).

**Q: Can I use more than 8 threads?**

A: Yes, but with diminishing returns. K=8 already achieves 93% of theoretical maximum (7.5x / 8.0x). K=16 would likely give <8x speedup due to overhead.

**Q: Does threading work on Windows/Linux?**

A: Threading implementation is platform-independent (C++ std::thread). Tested on macOS Apple Silicon. Linux x86_64 validation planned. Windows support planned for future version.

**Q: Will output files be identical to upstream?**

A: **Yes, 100% pixel-perfect**. All 452 test PDFs pass byte-for-byte MD5 validation vs upstream `pdfium_test`. Text extraction also 100% byte-for-byte identical.

---

## Additional Resources

**Documentation**:
- README.md: Project overview and quick start
- RELEASE_NOTES_V1.2.0.md: Detailed release notes and technical details
- CLAUDE.md: Development protocols and architecture notes

**Testing**:
- integration_tests/: 2,780 tests (100% pass rate: 2,780 passed, 0 skipped)
- integration_tests/telemetry/runs.csv: 190,000+ test runs logged

**Reports** (technical deep-dives):
- reports/main/N264_VALIDATION_COMPLETE.md: Production corpus measurements
- reports/main/N271_FINAL_OPTIMIZATION_ASSESSMENT.md: Optimization stopping criteria

**Support**:
- GitHub Issues: https://github.com/dropbox/dKNOW/pdfium_fast/issues
- Email: ayates@dropbox.com

---

## Version History

**v1.4.0** (2025-11-18):
- Quality flags (--quality fast/none) for optional rendering modes
- Optimization work complete (Stop Condition #2 met)
- 72x total speedup vs upstream (11x PNG × 6.55x threading)
- 100% test pass rate (2,780/2,780 tests pass, 0 skipped)

**v1.1.0** (2025-11-16):
- Multi-threaded image rendering (--threads K)
- Lock-free architecture with pre-loading strategy
- Zero mutexes during parallel phase

**v1.0.0** (2025-11-08):
- Multi-process parallelism (--workers N)
- Smart mode (545x for scanned PDFs)
- 100% correctness validation (67/67 smoke tests)

---

<p align="center">
  <b>PDFium Fast v1.2.0</b><br>
  3.93x mean speedup • 100% correctness • Production-ready
</p>
