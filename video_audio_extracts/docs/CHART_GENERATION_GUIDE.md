# Chart Generation Guide

**Date:** 2025-11-07 (N=66)
**Status:** Phase 5.3 Complete - Performance Comparison Charts Generated

---

## Overview

This guide documents the methodology for generating performance comparison charts from benchmark data. All charts are created using pure Python with SVG output, requiring no external plotting libraries (matplotlib-free).

**Generated Charts:**
1. **Throughput Comparison** - Operations sorted by MB/s throughput
2. **Latency Distribution** - Operations sorted by milliseconds latency
3. **Memory Usage by Category** - Average memory per operation category with min/max ranges
4. **Concurrency Scaling Efficiency** - Speedup and parallel efficiency vs worker count

**Output Locations:**
- Interactive HTML dashboard: `docs/charts/performance_charts.html`
- Individual SVG files: `docs/charts/*.svg`
- Chart generation script: `scripts/generate_performance_charts.py`

---

## Chart Generation Methodology

### Data Source

All chart data is extracted from `docs/PERFORMANCE_BENCHMARKS.md` (Phase 5.2, N=149, charts updated N=148):

**Benchmark Configuration:**
- Hardware: Apple M2 Max, 64 GB RAM, macOS Darwin 24.6.0
- Binary: video-extract v0.3.0 (release build)
- Environment: VIDEO_EXTRACT_THREADS=4
- Execution mode: performance mode (no debug overhead)
- Test files: Small (30-450 KB) from test_edge_cases/ directory

**Operations Benchmarked:** 23 operations across 6 categories (updated N=149)
- Core Extraction (3): metadata_extraction, keyframes, audio_extraction
- Speech & Audio (3): transcription, voice_activity_detection, audio_classification
- Vision Analysis (3): object_detection, face_detection, ocr
- Intelligence & Content (3): image_quality_assessment, smart_thumbnail, scene_detection
- Embeddings (2): vision_embeddings, audio_embeddings
- Utility (2): duplicate_detection, format_conversion

**Concurrency Data:** 5 concurrency levels tested (1, 2, 4, 8, 16 workers)

---

## Chart Descriptions

### 1. Throughput Comparison (throughput_comparison.svg)

**Purpose:** Compare throughput (MB/s) across operations that process significant data volumes.

**Data:**
- X-axis: Throughput (MB/s)
- Y-axis: Operation names
- Bars: Colored by category, sorted by throughput (highest to lowest)
- Excluded: Operations without meaningful throughput (image operations on 8.3 KB files)

**Key Insights:**
- Audio operations achieve highest throughput (7-8 MB/s)
  - audio_embeddings: 8.00 MB/s
  - audio_classification: 7.85 MB/s
  - voice_activity_detection: 7.62 MB/s
- Scene detection optimized (2.80 MB/s)
- Transcription slower (1.58 MB/s) due to ML inference complexity

**Visual Design:**
- Bar chart with horizontal bars
- Color-coded by operation category (6 colors)
- Value labels on bars for precise reading
- Grid lines for easy reference
- Interactive tooltips on hover

---

### 2. Latency Distribution (latency_distribution.svg)

**Purpose:** Show latency (milliseconds) for all operations, revealing startup overhead impact.

**Data:**
- X-axis: Latency (milliseconds)
- Y-axis: Operation names
- Bars: Colored by category, sorted by latency (lowest to highest)
- All 16 operations included

**Key Insights:**
- Narrow latency range: 53-86ms (only 33ms spread)
- Fastest: scene_detection (53ms)
- Slowest: metadata_extraction (86ms)
- Most operations: 55-66ms (dominated by ~50ms startup overhead)
- For small files (<1 MB), startup overhead accounts for 88% of latency

**Visual Design:**
- Bar chart with horizontal bars
- Color-coded by operation category
- Value labels showing exact milliseconds
- Grid lines every 20ms

**Production Implications:**
- For files >1 MB, processing time will dominate
- Bulk mode amortizes startup overhead across multiple files
- Single-file latency <100ms suitable for API workloads

---

### 3. Memory Usage by Category (memory_usage.svg)

**Purpose:** Show consistent memory usage across operation categories with variance ranges.

**Data:**
- X-axis: Operation categories (6 categories)
- Y-axis: Memory usage (MB)
- Bars: Average memory per category
- Error bars: Min/max range within category

**Key Insights:**
- Remarkably consistent: 14.65 MB ± 0.25 MB average
- All categories within 14.51-15.15 MB range (±2% variance)
- Memory dominated by binary base overhead (~14 MB)
- Low footprint suitable for embedded/constrained environments

**Visual Design:**
- Bar chart with error bars (min/max)
- Fixed Y-axis scale (0-16 MB) for clarity
- Color-coded by category
- Average value labels on bars

**Production Memory (Large Files):**
- Light operations: 16-128 MB
- Medium operations: 256-512 MB
- Heavy operations: 500-1024 MB

---

### 4. Concurrency Scaling Efficiency (concurrency_scaling.svg)

**Purpose:** Show speedup and parallel efficiency vs number of workers.

**Data:**
- X-axis: Number of workers (1, 2, 4, 8, 16)
- Y-axis (left): Speedup (vs 1 worker baseline)
- Y-axis (right): Parallel efficiency (%)
- Two lines: Speedup (solid blue), Efficiency (dashed green)

**Key Insights:**
- Best efficiency: 2 workers (86% efficiency, 1.72x speedup)
- Best speedup: 8 workers (2.10x speedup, 26% efficiency)
- Diminishing returns: Beyond 8 workers, speedup plateaus
- Recommendation: Use 4-8 workers for production (optimal trade-off)

**Visual Design:**
- Dual-axis line chart
- Speedup: Solid blue line with circular markers
- Efficiency: Dashed green line with circular markers
- Value labels on each data point
- Grid lines for reference

**Parallel Efficiency Formula:**
```
Parallel Efficiency = Speedup / Number of Workers
```

**Production Recommendation:**
- 2 workers: Best for CPU-limited systems (86% efficiency)
- 4 workers: Balanced throughput/efficiency (48% efficiency, 1.93x speedup)
- 8 workers: Maximum throughput (26% efficiency, 2.10x speedup)
- 16+ workers: Not recommended (efficiency <13%, no speedup gain)

---

## Reproducing Charts

### Requirements

- Python 3.6+ (no external libraries required)
- Benchmark data in `docs/PERFORMANCE_BENCHMARKS.md`

### Generate All Charts

```bash
# From project root
python3 scripts/generate_performance_charts.py

# Output:
# ✅ Performance charts generated: docs/charts/performance_charts.html
# ✅ Generated: docs/charts/throughput_comparison.svg
# ✅ Generated: docs/charts/latency_distribution.svg
# ✅ Generated: docs/charts/memory_usage.svg
# ✅ Generated: docs/charts/concurrency_scaling.svg
```

### View Charts

**Interactive HTML Dashboard:**
```bash
open docs/charts/performance_charts.html
```

**Individual SVG Files:**
```bash
open docs/charts/throughput_comparison.svg
open docs/charts/latency_distribution.svg
open docs/charts/memory_usage.svg
open docs/charts/concurrency_scaling.svg
```

---

## Chart Data Extraction

### Throughput Data

Extracted from PERFORMANCE_BENCHMARKS.md operation tables:

```python
{
    "name": "audio_embeddings",
    "category": "Embeddings",
    "latency_ms": 56,
    "memory_mb": 15.15,
    "throughput_mbs": 8.00
}
```

**Exclusions:**
- Operations without throughput metrics (image operations on 8.3 KB files)
- Operations marked as "N/A" for throughput

### Latency Data

All 16 benchmarked operations included:

```python
{
    "name": "scene_detection",
    "category": "Intelligence",
    "latency_ms": 53,
    "memory_mb": 14.65,
    "throughput_mbs": 2.80
}
```

### Memory Data

Grouped by category, calculated statistics:

```python
categories = {
    "Core": [14.59, 14.57, 14.67],  # metadata, keyframes, audio
    "Audio": [14.84, 14.57, 14.60],  # transcription, VAD, classification
    # ... etc
}

# Calculate avg, min, max per category
```

### Concurrency Data

Extracted from "Concurrency Scaling Efficiency" table:

```python
{
    "workers": 8,
    "time_s": 0.92,
    "throughput_fps": 8.71,
    "speedup": 2.10,
    "efficiency_pct": 26.2
}
```

---

## Chart Styling

### Color Palette

Category colors (consistent across all charts):

```python
colors = {
    "Core": "#007AFF",          # Blue (Apple system blue)
    "Audio": "#34C759",         # Green
    "Vision": "#FF9500",        # Orange
    "Intelligence": "#AF52DE",  # Purple
    "Embeddings": "#FF2D55",    # Pink
    "Utility": "#5AC8FA"        # Light blue
}
```

### Typography

- Font: -apple-system, BlinkMacSystemFont, 'Segoe UI', Arial, sans-serif
- Title: 18px bold
- Axis labels: 14px
- Value labels: 11-12px
- Grid: #eee (light gray)

### Layout

- Chart dimensions: 1000x500 to 1200x600 pixels
- Margins: 40-150px (accommodates labels)
- Bar opacity: 0.85 (subtle transparency)
- Hover effects: Opacity 0.8, cursor pointer

---

## Interactive Features

### HTML Dashboard

**Features:**
- All 4 charts in single scrollable page
- Responsive layout (max-width: 1400px)
- Hover tooltips on chart elements
- Clean white card design with shadows

**Metadata Display:**
- Benchmark date, hardware, binary version
- Test configuration details
- Important notes about startup overhead and file sizes

### SVG Charts

**Interactivity:**
- Hover tooltips with precise values
- Title attributes on all chart elements
- Clickable bars (opacity change on hover)

**Accessibility:**
- Clear axis labels
- Grid lines for easy reading
- High-contrast colors
- Large fonts (12-18px)

---

## Chart Validation

### Data Accuracy

All chart data matches PERFORMANCE_BENCHMARKS.md exactly:

**Verified:**
- ✅ 16 operations with correct latency values
- ✅ 10 operations with throughput metrics
- ✅ 16 operations with memory values
- ✅ 5 concurrency levels with correct speedup/efficiency

**Cross-checks:**
- Throughput sorted correctly (8.00 → 0.51 MB/s)
- Latency sorted correctly (53 → 86 ms)
- Memory ranges match calculated stats
- Concurrency speedup matches raw benchmark results

### Visual Correctness

**Checked:**
- ✅ Bar lengths proportional to values
- ✅ Grid lines aligned with values
- ✅ Color consistency across charts
- ✅ Labels readable and non-overlapping
- ✅ Tooltips match bar values

---

## Future Enhancements

### Phase 5.2: Hardware Configuration Charts (Optional)

When hardware configuration benchmarks available (Phase 5.2):
- Add hardware comparison charts (low-end, mid-range, high-end)
- Multi-line charts showing scaling across configurations
- Hardware recommendation flowchart

### Additional Chart Ideas

1. **Operation Category Heatmap** - All 33 operations × 3 metrics (latency, throughput, memory)
2. **File Size Scaling** - Throughput vs file size for select operations
3. **ML Model Performance** - ONNX Runtime acceleration comparisons
4. **Format-Specific Performance** - Throughput by video/audio/image formats

### Chart Export Formats

Currently: SVG + HTML
Future: PNG, PDF (requires external libraries or conversion tools)

---

## Troubleshooting

### Charts Not Displaying

**Issue:** HTML file shows no charts
**Solution:** Ensure SVG content is embedded in HTML, check browser console

### Incorrect Data

**Issue:** Chart values don't match benchmark document
**Solution:** Re-run chart generation script, verify BENCHMARK_DATA in script

### Missing Charts

**Issue:** SVG files not generated
**Solution:** Check write permissions on docs/charts/ directory

```bash
mkdir -p docs/charts
chmod 755 docs/charts
python3 scripts/generate_performance_charts.py
```

---

## References

**Source Documents:**
- `docs/PERFORMANCE_BENCHMARKS.md` - Phase 5.1 benchmark data (N=57)
- `docs/PERFORMANCE_OPTIMIZATION_GUIDE.md` - Phase 5.4 user-facing guide (N=65)
- `PRODUCTION_READINESS_PLAN.md` - Phase 5.3 specification

**Chart Files:**
- `docs/charts/performance_charts.html` - Interactive dashboard
- `docs/charts/throughput_comparison.svg` - Throughput bar chart
- `docs/charts/latency_distribution.svg` - Latency bar chart
- `docs/charts/memory_usage.svg` - Memory category chart
- `docs/charts/concurrency_scaling.svg` - Concurrency line chart

**Generation Script:**
- `scripts/generate_performance_charts.py` - Chart generation script (726 lines, no external dependencies)

---

## Changelog

**N=66 (2025-11-07):** Initial chart generation
- Created 4 performance comparison charts
- Generated interactive HTML dashboard
- Documented chart generation methodology
- Phase 5.3 complete

---

**End of Chart Generation Guide**
