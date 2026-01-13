# Performance Benchmarking Guide

This guide explains how to use the Docling performance benchmarking framework to measure and analyze document conversion performance.

## Performance Overview

### Format-Specific Performance Characteristics

Based on comprehensive benchmarking with 97 canonical tests across 15 formats:

| Format Category | Avg Time | Throughput | Characteristics |
|----------------|----------|------------|-----------------|
| **Text Formats** (CSV, Markdown, AsciiDoc) | 5-30ms | High | Fastest, minimal parsing overhead |
| **Web Formats** (HTML) | 0.5-2s | Medium-High | Moderate parsing, efficient extraction |
| **Office Documents** (DOCX, XLSX, PPTX) | 0.5-3s | Medium | Complex structure, good performance |
| **PDF (text-based)** | 2-8s | Medium | Depends on page count and complexity |
| **PDF (OCR)** | 10-30s | Low | ML model overhead, non-deterministic |
| **Images (OCR)** | 2-5s | Low | OCR initialization cost per file |

**Key Insights:**
- **Text formats** (CSV, Markdown) are 100-1000x faster than PDF with OCR
- **Office documents** (DOCX, XLSX) have consistent, predictable performance
- **OCR operations** dominate runtime: 28% of test time for 13% of tests (N=107)
- **File size** is not the primary factor - document complexity and format matter more

### Backend Selection Guide

#### Current Backends

**Python Backend** (default, production-ready):
- Uses official Python docling v2.58.0
- All 55 formats supported
- 94.8% test pass rate (92/97 canonical tests)
- ML models for layout analysis and OCR
- Battle-tested algorithms

**Hybrid Backend** (experimental):
- Python ML parsing + Rust serialization
- Same format support as Python
- Enables Rust serializer testing
- Use `USE_HYBRID_SERIALIZER=1` environment variable

**Native Rust Backend** (future, Phase I):
- Planned for Phase I (6-12 months)
- Will eliminate Python dependency
- Target: 5-10x performance improvement
- Focus on common formats first (PDF, DOCX, HTML)

#### When to Use Each Backend

**Use Python Backend When:**
- Processing any of the 55 supported formats
- Maximum compatibility and accuracy needed
- OCR or ML-based layout analysis required
- Production deployment (stable, tested)
- Not performance-critical (<1000 docs/hour)

**Use Hybrid Backend When:**
- Testing Rust serializer implementations
- Developing custom output formats
- Verifying serialization accuracy
- Contributing to serializer development

**Future Native Rust Backend (When Available):**
- High-throughput batch processing (>10,000 docs/hour)
- Latency-critical applications (<100ms target)
- Embedded systems with limited Python support
- Reduced deployment complexity (no Python dependency)

### Memory Characteristics

**Current Implementation (Python Backend):**
- Memory tracking not yet implemented (TODO: performance.rs:258-259)
- Python process handles memory management
- Generally low memory usage for typical documents (<100 MB)

**Large File Handling:**
- PDF files: Memory scales with page count and embedded objects
- Office documents: Generally efficient (ZIP-based compression)
- Images with OCR: Peak memory during ML model inference
- Archives: Processed sequentially (no memory explosion)

**Best Practices:**
- Process very large files (>500 pages, >100 MB) sequentially
- Monitor system memory when batch processing OCR-heavy workloads
- Use streaming API (Phase G+) for massive documents when available

## Quick Start

### CLI Benchmarking

The simplest way to benchmark document conversion is using the `docling benchmark` command:

```bash
# Benchmark a single file (3 iterations, 1 warmup)
docling benchmark test.pdf

# Benchmark multiple files
docling benchmark file1.pdf file2.docx file3.html

# Custom iterations and warmup
docling benchmark test.pdf -n 10 -w 2

# Output formats
docling benchmark test.pdf --format json -o results.json
docling benchmark test.pdf --format csv -o results.csv
docling benchmark test.pdf --format markdown -o results.md
```

### Programmatic Benchmarking

Use the Rust API for more control:

```rust
use docling_core::performance::{BenchmarkRunner, BenchmarkConfig};
use std::path::Path;

// Create configuration
let config = BenchmarkConfig {
    iterations: 5,
    warmup_iterations: 1,
    enable_ocr: false,
    ..Default::default()
};

// Run benchmark
let runner = BenchmarkRunner::new(config);
let result = runner.run_benchmark(Path::new("test.pdf"))?;

// Access metrics
println!("Average parse time: {:.2} ms", result.average.parse_time_ms);
println!("Average total time: {:.2} ms", result.average.total_time_ms);
println!("Throughput: {:.2} B/s", result.average.throughput_bps);
```

## CLI Reference

### Commands

```bash
docling benchmark [OPTIONS] <INPUT>...
```

### Arguments

- `<INPUT>...` - One or more input files to benchmark (required)

### Options

- `-n, --iterations <N>` - Number of iterations (default: 3)
- `-w, --warmup <N>` - Warmup iterations, results discarded (default: 1)
- `-f, --format <FORMAT>` - Output format: text, json, csv, markdown (default: text)
- `-o, --output <FILE>` - Output file (default: stdout)
- `--ocr` - Enable OCR for scanned PDFs

### Examples

#### Basic Benchmark

```bash
docling benchmark document.pdf
```

Output:
```
Running benchmark with 3 iterations (1 warmup)...

Performance Benchmark Results
=============================

File: document.pdf
Format: pdf
Iterations: 3

Average:
  Parse time:        123.45 ms
  Serialize time:      0.00 ms
  Total time:        123.45 ms
  Throughput:      8100.00 bytes/s

Std Dev:
  Parse time:          5.67 ms
  Total time:          5.67 ms

Min/Max:
  Parse time:        118.00 / 129.00 ms
  Total time:        118.00 / 129.00 ms
```

#### Multiple Files

```bash
docling benchmark *.pdf -n 5
```

#### JSON Output

```bash
docling benchmark document.pdf --format json -o benchmark.json
```

Output (`benchmark.json`):
```json
[
  {
    "file_path": "document.pdf",
    "file_format": "pdf",
    "iterations": [...],
    "average": {
      "parse_time_ms": 123.45,
      "serialize_time_ms": 0.0,
      "total_time_ms": 123.45,
      "throughput_bps": 8100.0
    },
    "std_dev": { ... },
    "min": { ... },
    "max": { ... }
  }
]
```

#### CSV Output

```bash
docling benchmark *.pdf --format csv -o benchmark.csv
```

Output (`benchmark.csv`):
```csv
file,format,iterations,avg_parse_ms,avg_serialize_ms,avg_total_ms,avg_throughput_bps,std_parse_ms,std_total_ms,min_total_ms,max_total_ms
doc1.pdf,pdf,3,123.45,0.00,123.45,8100.00,5.67,5.67,118.00,129.00
doc2.pdf,pdf,3,456.78,0.00,456.78,2190.00,12.34,12.34,444.00,469.00
```

#### Markdown Table

```bash
docling benchmark *.pdf --format markdown
```

Output:
```markdown
# Performance Benchmark Results

| File | Format | Avg Parse (ms) | Avg Total (ms) | Throughput (B/s) | Std Dev (ms) |
|------|--------|----------------|----------------|------------------|---------------|
| doc1.pdf | pdf | 123.45 | 123.45 | 8100.00 | 5.67 |
| doc2.pdf | pdf | 456.78 | 456.78 | 2190.00 | 12.34 |
```

## API Reference

### Core Types

#### `PerformanceMetrics`

Metrics for a single conversion operation:

```rust
pub struct PerformanceMetrics {
    pub parse_time: Duration,           // Time spent parsing
    pub serialize_time: Duration,       // Time spent serializing
    pub total_time: Duration,           // Total time
    pub memory_used_bytes: Option<u64>, // Memory used
    pub peak_memory_bytes: Option<u64>, // Peak memory
    pub input_size_bytes: u64,          // Input file size
    pub output_size_bytes: u64,         // Output size
    pub output_size_chars: usize,       // Output character count
    pub throughput_bps: f64,            // Bytes per second
    pub file_path: PathBuf,             // File that was converted
    pub file_format: String,            // File format
    pub backend: String,                // Backend used
    pub timestamp: DateTime<Utc>,       // When measured
}
```

#### `BenchmarkConfig`

Configuration for benchmark runs:

```rust
pub struct BenchmarkConfig {
    pub iterations: usize,              // Number of iterations
    pub warmup_iterations: usize,       // Warmup iterations
    pub enable_ocr: bool,               // Enable OCR
    pub output_format: BenchmarkOutputFormat,
    pub output_path: Option<PathBuf>,
}
```

#### `BenchmarkResult`

Results for a single file:

```rust
pub struct BenchmarkResult {
    pub file_path: PathBuf,
    pub file_format: String,
    pub iterations: Vec<PerformanceMetrics>,
    pub average: PerformanceMetricsSummary,
    pub std_dev: PerformanceMetricsSummary,
    pub min: PerformanceMetricsSummary,
    pub max: PerformanceMetricsSummary,
}
```

### `BenchmarkRunner`

Main API for running benchmarks:

```rust
impl BenchmarkRunner {
    // Create runner with config
    pub fn new(config: BenchmarkConfig) -> Self;

    // Create runner with defaults
    pub fn default_config() -> Self;

    // Benchmark single file
    pub fn run_benchmark(&self, path: &Path) -> Result<BenchmarkResult>;

    // Benchmark multiple files
    pub fn run_benchmarks(&self, paths: &[PathBuf]) -> Result<Vec<BenchmarkResult>>;

    // Format results
    pub fn format_as_text(results: &[BenchmarkResult]) -> String;
    pub fn format_as_json(results: &[BenchmarkResult]) -> Result<String>;
    pub fn format_as_csv(results: &[BenchmarkResult]) -> String;
    pub fn format_as_markdown(results: &[BenchmarkResult]) -> String;
}
```

## Batch Processing Performance

### Sequential vs Parallel Processing

**Current Implementation (Sequential Only):**
- All tests run with `--test-threads=1` (sequential)
- Required for pdfium thread-safety (C library limitation)
- PDF tests cannot be parallelized safely

**Performance Impact:**
- 97 canonical tests: ~14 minutes (842s) on debug build (N=107)
- 97 canonical tests: ~7.5 minutes (450s) on release build (N=100)
- Average: 8.68 seconds per test (debug), ~4.6 seconds per test (release)

**Optimization Strategies:**

1. **Use Release Builds for Benchmarking**
   ```bash
   # 2-3x faster than debug builds
   cargo test --release -p docling-core --test integration_tests test_canon
   ```

2. **Separate OCR from Non-OCR Tests**
   ```bash
   # Run fast non-OCR tests first (10x faster)
   cargo test test_canon_docx test_canon_html test_canon_csv

   # Run slow OCR tests separately
   cargo test test_canon_pdf_.*_ocr --release -- --test-threads=1
   ```

3. **Batch Multiple Documents**
   ```rust
   // Process multiple files in one session (amortize ML model loading)
   let converter = RustDocumentConverter::new()?;
   for file in files {
       converter.convert(file)?; // Reuses loaded models
   }
   ```

4. **Future: Parallel Non-PDF Processing**
   - CSV, HTML, DOCX, Markdown can run in parallel (no pdfium dependency)
   - Potential 20% speedup for mixed workloads
   - Requires selective parallelization by format

### Batch Processing Best Practices

**For High-Throughput Workloads:**

1. **Group by Format**
   - Process similar formats together (e.g., all DOCX, then all PDF)
   - Reduces backend switching overhead
   - Enables format-specific optimizations

2. **Prioritize by Speed**
   - Process fast formats first (CSV, Markdown, HTML)
   - Schedule slow formats (OCR) during off-peak hours
   - Provides quick wins for users

3. **Monitor Progress**
   - Use progress bars for batch operations (indicatif crate)
   - Log failures separately, don't halt entire batch
   - Generate summary report at end

4. **Error Recovery**
   - Continue processing on individual file failures
   - Log errors with file paths for later retry
   - Use circuit breaker pattern for repeated failures

**Example Batch Processing Pattern:**
```rust
use docling_backend::RustDocumentConverter;  // Note: RustDocumentConverter is in docling-backend crate
use std::path::Path;

fn batch_convert(files: &[&Path]) -> Result<()> {
    let converter = RustDocumentConverter::new()?;
    let mut successes = 0;
    let mut failures = Vec::new();

    for file in files {
        match converter.convert(file) {
            Ok(_) => successes += 1,
            Err(e) => failures.push((file, e)),
        }
    }

    println!("Converted {}/{} files", successes, files.len());
    for (file, err) in failures {
        eprintln!("Failed: {}: {}", file.display(), err);
    }

    Ok(())
}
```

### OCR Performance Impact

**Benchmark Data (N=107):**
- **Non-OCR tests:** 84 tests, avg ~6s/test, 100% pass rate
- **OCR tests:** 13 tests, avg ~15s/test, 76.9% pass rate (non-determinism)
- **OCR overhead:** 10-20 seconds per document (2-3x slower than non-OCR)
- **OCR runtime share:** 28% of total test time for 13% of tests

**Performance Characteristics:**

| Operation | Time | Notes |
|-----------|------|-------|
| OCR initialization | 1-2s | Per test, one-time cost |
| ML model loading | 0.5-1s | docling_parse models |
| Per-page OCR | 0.5-2s | Depends on image complexity |
| Text extraction (non-OCR) | 0.1-0.5s | Fast, deterministic |

**OCR Performance Tips:**

1. **Avoid OCR When Possible**
   - Most PDFs have embedded text (no OCR needed)
   - Use `--ocr=false` by default, enable only for scanned documents
   - Detect text-based PDFs automatically (check for text layer)

2. **Batch OCR Operations**
   - Process multiple OCR documents in one session
   - ML models stay loaded between conversions
   - Amortizes 1-2s initialization cost across documents

3. **OCR Quality vs Speed Trade-off**
   - Higher quality OCR = longer processing time
   - Current implementation uses macOS OCR (ocrmac) for speed
   - Consider Tesseract for better accuracy (slower)

4. **Non-Deterministic Results**
   - OCR output varies slightly between runs
   - Character differences: <0.2% (e.g., "O" vs "0", "l" vs "I")
   - Not a bug - inherent to ML-based OCR
   - Use fuzzy matching for OCR output validation

**OCR Optimization Roadmap (Future):**
- GPU acceleration for OCR inference (Phase I+)
- Batch OCR API (process multiple images in one call)
- Cached OCR results (avoid re-processing same documents)
- Parallel OCR for multi-page documents

## Best Practices

### Iteration Count

- **Quick check**: 3 iterations, 1 warmup (default)
- **Development**: 5-10 iterations, 1-2 warmup
- **CI/CD**: 10-20 iterations, 2-3 warmup
- **Publication**: 30-100 iterations, 5-10 warmup

### Warmup Iterations

Warmup iterations are important because:
- First run may include cold caches
- ML model initialization and loading
- OS resource allocation
- PyTorch/ONNX runtime initialization

Recommendation: Use at least 1 warmup iteration (2-3 for OCR workloads).

### Environment

For reliable benchmarks:

1. **Close other applications** - Reduce CPU/memory competition
2. **Disable CPU throttling** - Use performance governor
3. **Consistent power state** - Plugged in for laptops
4. **Same system load** - Run at same time of day
5. **Multiple runs** - Run benchmarks multiple times, report median

### Analysis

When analyzing benchmark results:

1. **Check std dev** - High variance indicates noise
2. **Look for outliers** - Remove extreme values
3. **Compare throughput** - Normalize by file size
4. **Track over time** - Use CSV output for tracking
5. **Document environment** - Record CPU, memory, OS version

## Integration with CI/CD

### GitHub Actions Example

```yaml
name: Performance Benchmark

on:
  pull_request:
    branches: [ main ]

jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2

      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Build
        run: cargo build --release

      - name: Run Benchmark
        run: |
          cargo run --release -- benchmark \
            test-corpus/pdf/*.pdf \
            --format csv \
            -o benchmark-results.csv \
            -n 10 -w 2

      - name: Upload Results
        uses: actions/upload-artifact@v2
        with:
          name: benchmark-results
          path: benchmark-results.csv

      - name: Compare with Baseline
        run: |
          python scripts/compare_benchmarks.py \
            benchmark-results.csv \
            baseline-results.csv
```

## Troubleshooting

### High Variance

**Symptom**: Large standard deviation (>10% of mean)

**Causes**:
- Background processes
- Thermal throttling
- Swapping to disk
- Network activity (for remote files)

**Solutions**:
- Increase iteration count
- Close background applications
- Use performance CPU governor
- Add more warmup iterations

### Inconsistent Results

**Symptom**: Different results across runs

**Causes**:
- Non-deterministic operations (OCR)
- Caching effects
- Garbage collection

**Solutions**:
- Increase iteration count
- Run multiple benchmark sessions
- Report median instead of mean
- Document known sources of variance

### Memory Issues

**Symptom**: Out of memory errors

**Causes**:
- Large files
- Memory leaks
- Insufficient RAM

**Solutions**:
- Benchmark smaller files
- Run benchmarks sequentially
- Increase system memory
- Check for memory leaks

## Advanced Usage

### Custom Metrics Collection

```rust
use docling_core::performance::*;
use std::time::Instant;

// Measure specific operation
let start = Instant::now();
let document = convert_document(path)?;
let parse_time = start.elapsed();

// Create custom metrics
let metrics = PerformanceMetrics {
    parse_time,
    serialize_time: Duration::from_secs(0),
    total_time: parse_time,
    // ... other fields
};
```

### Comparing Backends

```rust
// Benchmark Rust backend
let rust_config = BenchmarkConfig {
    iterations: 10,
    warmup_iterations: 2,
    enable_ocr: false,
    ..Default::default()
};

let rust_result = BenchmarkRunner::new(rust_config)
    .run_benchmark(Path::new("test.pdf"))?;

// Compare throughput
println!("Rust backend: {:.2} MB/s",
    rust_result.average.throughput_bps / 1_000_000.0);
```

## Performance FAQ

### Q: Why are OCR tests so much slower than regular tests?

**A:** OCR requires ML model initialization (1-2s), model loading (0.5-1s), and per-page inference (0.5-2s/page). Text-based PDFs extract text directly (0.1-0.5s total) without ML overhead. OCR is 10-20x slower but necessary for scanned documents.

**Solution:** Only use OCR when needed. Most PDFs have embedded text and don't require OCR.

---

### Q: How can I speed up batch processing?

**A:** Four main strategies:

1. **Use release builds** (`--release` flag): 2-3x faster
2. **Group by format**: Process all DOCX files together, then all PDFs (reduces backend switching)
3. **Separate OCR tests**: Run fast non-OCR tests first, schedule OCR for off-peak
4. **Reuse converter instance**: Create one `RustDocumentConverter`, process multiple files (amortizes ML model loading)

Expected improvement: 3-5x faster with all optimizations.

---

### Q: Why does file size not correlate with processing time?

**A:** Document complexity matters more than size:
- **10 MB simple PDF** (text-only, 100 pages): ~2-3 seconds
- **1 MB complex PDF** (scanned, tables, images, OCR): ~15-20 seconds

Factors affecting performance:
1. Format complexity (PDF > DOCX > CSV)
2. OCR requirement (20x slowdown)
3. Number of embedded objects (images, tables)
4. Document structure complexity (nested sections, complex layouts)

---

### Q: What is the expected throughput for production workloads?

**A:** Based on N=107 benchmark data:

| Workload Type | Throughput | Notes |
|---------------|------------|-------|
| **Text formats** (CSV, Markdown) | 1000-5000 docs/hour | Very fast |
| **Office documents** (DOCX, XLSX) | 500-1500 docs/hour | Moderate |
| **PDF (text-based)** | 200-600 docs/hour | Depends on pages |
| **PDF (OCR)** | 50-200 docs/hour | Slow, ML-intensive |
| **Mixed workload** | 300-800 docs/hour | Typical production |

**Note:** Assumes sequential processing. Release build, modern hardware (8-core CPU, 16 GB RAM).

---

### Q: How do I benchmark my specific workload?

**A:** Use the CLI benchmark command with your actual documents:

```bash
# Benchmark your documents
docling benchmark my-documents/*.pdf -n 10 -w 2 --format csv -o results.csv

# Analyze results
# - Look at avg_total_ms for typical processing time
# - Check std_total_ms for consistency (low = good)
# - Calculate throughput: 3600000 / avg_total_ms = docs/hour
```

For production capacity planning:
1. Benchmark representative sample (10-20 documents)
2. Measure 95th percentile time (not just average)
3. Add 20-30% safety margin for system overhead
4. Test under realistic load (concurrent requests, system load)

---

### Q: When should I consider the native Rust backend?

**A:** Consider native Rust backend (Phase I, when available) if:

1. **High throughput required:** >10,000 documents/hour
2. **Latency-critical:** <100ms response time target
3. **Deployment constraints:** Cannot install Python dependencies
4. **Cost optimization:** Reduce compute costs for large-scale processing
5. **Edge deployment:** Embedded systems, resource-constrained environments

**Current Python backend is sufficient for:**
- <1000 documents/hour
- Response times <5 seconds acceptable
- Standard server environments with Python available
- 94.8% accuracy is adequate (native Rust will match this)

---

### Q: Why is the first test run slower than subsequent runs?

**A:** "Cold start" effects:

1. **OS file cache:** First read loads from disk, subsequent reads from RAM cache
2. **Python JIT compilation:** First run compiles bytecode
3. **ML model loading:** Models loaded on first OCR operation, cached for subsequent
4. **Library initialization:** Python docling initializes backends on first use

**Solution:** Use warmup iterations (default: 1 warmup before 3 measured iterations).

---

### Q: How accurate are the benchmark results?

**A:** Variance depends on workload:

- **Text formats:** Very low variance (<5% std dev) - highly repeatable
- **Office documents:** Low variance (5-10% std dev) - consistent
- **PDF (non-OCR):** Medium variance (10-20% std dev) - affected by OS caching
- **PDF (OCR):** High variance (20-40% std dev) - ML non-determinism

**Best practices for accurate results:**
1. Use 10+ iterations for production benchmarks
2. Close background applications during measurement
3. Use same hardware/OS configuration for comparisons
4. Report median (not mean) for OCR workloads
5. Document system configuration (CPU, RAM, OS, Python version)

---

### Q: Can I parallelize document processing?

**A:** Limited parallelization currently:

**Cannot parallelize:**
- PDF processing (pdfium C library not thread-safe)
- Requires `--test-threads=1` for PDF tests

**Can parallelize (future):**
- CSV, HTML, DOCX, Markdown (no pdfium dependency)
- Potential 20% speedup for mixed workloads
- Requires format-aware parallelization

**Workaround:**
- Run multiple docling processes (process-level parallelism)
- Each process handles one document at a time
- OS schedules across cores
- Effective for batch processing with job queue

---

### Q: What are the known performance bottlenecks?

**A:** Identified bottlenecks (N=107 analysis):

1. **OCR operations** (28% of test time for 13% of tests)
   - ML model initialization overhead
   - Cannot be eliminated (required for scanned documents)
   - Mitigation: Batch OCR operations, reuse loaded models

2. **Debug builds** (2-3x slower than release)
   - Use `--release` for benchmarking
   - Development uses debug builds for better error messages

3. **Sequential execution** (no parallelism)
   - pdfium thread-safety limitation
   - Mitigation: Process-level parallelism, future selective parallelization

4. **Per-test Python initialization**
   - Each test spawns fresh Python process
   - Mitigation: Reuse converter instance in production

**Future optimizations (Phase I):**
- Native Rust PDF parser (eliminate Python overhead)
- GPU-accelerated OCR (5-10x faster)
- Parallel processing for non-PDF formats
- ML model caching across processes

## See Also

- [User Guide](USER_GUIDE.md) - General usage documentation
- [Architecture Documentation](docs/architecture.md) - System design and components
- [Phase H Planning](reports/feature/phase-e-open-standards/n125_phase_h_planning_2025-11-08.md) - Future performance improvements
- [N=100 Benchmark Report](reports/feature/phase-e-open-standards/n100_milestone_cleanup_benchmark_2025-11-08.md) - Detailed benchmark data
- [N=107 Benchmark Report](reports/feature/phase-e-open-standards/n107_benchmark_milestone_2025-11-08.md) - 100% format integration milestone
