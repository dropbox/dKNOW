# Roadmap: v1.6.0 - User Experience Improvements

**Target Release:** Q1 2025
**Focus:** Zero-overhead usability improvements
**Estimated Effort:** 3-5 AI commits (~1.5-2.5 hours)

---

## Executive Summary

v1.6.0 enhances user experience without adding computation overhead. All features are reporting, UI, or convenience improvements with <0.01% performance impact.

**Key Principle:** No new computation, only better reporting and error handling.

---

## Features

### 1. Progress Reporting (High Priority)

**Problem:** Silent execution makes users uncertain during long operations.

**Solution:** Real-time progress bars with ETA.

```bash
# Before (v1.5.0)
./pdfium_cli --threads 8 render-pages large.pdf images/
# ...silence for 10 seconds...

# After (v1.6.0)
./pdfium_cli --threads 8 render-pages large.pdf images/
Processing: [=====>    ] 547/1000 pages (54%) - 277 pps - ETA: 1.6s
```

**Implementation:**
- Add `ProgressReporter` class (fprintf to stderr)
- Update every 100ms or 10 pages (whichever is longer)
- Format: `[bar] current/total (%) - throughput - ETA`

**Overhead:** ~0.001% (one fprintf per 10 pages)

**Files:**
- `examples/pdfium_cli.cpp` (add ProgressReporter class)

**Effort:** 1 AI commit (~30 min)

---

### 2. Batch Processing (High Priority)

**Problem:** Users must script loops to process multiple PDFs.

**Solution:** Built-in directory processing.

```bash
# Process entire directory
./pdfium_cli --batch render-pages input_dir/ output_dir/

# With filters
./pdfium_cli --batch --pattern "*.pdf" render-pages docs/ images/

# Recursive
./pdfium_cli --batch --recursive render-pages project/ output/
```

**Implementation:**
- Add `--batch` flag (boolean)
- Add `--pattern` flag (glob, default "*.pdf")
- Add `--recursive` flag (boolean)
- Iterate input directory, process each PDF
- Create output subdirectories preserving structure

**Overhead:** 0% (file I/O already dominates)

**Error Handling:**
- Continue on error (don't abort entire batch)
- Log failures to stderr
- Exit code: 0 if all succeed, 1 if any fail

**Files:**
- `examples/pdfium_cli.cpp` (add batch_process function)

**Effort:** 1 AI commit (~30 min)

---

### 3. Better Error Messages (Medium Priority)

**Problem:** Generic errors require debugging.

**Solution:** Specific, actionable error messages.

```bash
# Before (v1.5.0)
Error: Cannot load document
Exit code: 1

# After (v1.6.0)
Error: Cannot open 'report.pdf'
  Reason: File is password-protected
  Solution: Use --password flag or decrypt the PDF first
  Help: ./pdfium_cli --help
Exit code: 1
```

**Common Errors to Improve:**
1. Password-protected PDFs
2. File not found
3. Invalid PDF structure
4. Out of memory
5. Unsupported features
6. Permission denied

**Implementation:**
- Create `ErrorReporter` class
- Map PDFium error codes to user messages
- Provide solutions for common issues

**Overhead:** 0% (only on error path)

**Files:**
- `examples/pdfium_cli.cpp` (add ErrorReporter class)

**Effort:** 1 AI commit (~30 min)

---

### 4. Performance Metrics Output (Medium Priority)

**Problem:** Users don't know actual performance achieved.

**Solution:** Report metrics at end of operation.

```bash
./pdfium_cli --threads 8 render-pages large.pdf images/
Processing: [==========] 1000/1000 pages (100%) - 277 pps - Done!

Performance Summary:
  Total pages: 1000
  Processing time: 3.61s
  Throughput: 277 pages/second
  Threading efficiency: 6.55x (K=8 vs K=1 baseline)
  Smart mode: 123 pages (12.3% via JPEG fast path)
  Peak memory: 487 MB (487 KB/page)
```

**Metrics to Report:**
- Total pages processed
- Wall-clock time
- Throughput (pages/second)
- Threading efficiency (actual speedup)
- Smart mode usage (% of pages)
- Peak memory (if available)

**Implementation:**
- Track start time, end time
- Count pages, smart mode hits
- Calculate metrics
- Print summary to stdout (not stderr, for easier parsing)

**Overhead:** ~0.001% (just arithmetic + one printf)

**Files:**
- `examples/pdfium_cli.cpp` (add MetricsReporter class)

**Effort:** 1 AI commit (~30 min)

---

### 5. Memory Usage Reporting (Low Priority)

**Problem:** Users need capacity planning data.

**Solution:** Track and report peak memory usage.

```bash
Peak memory: 487 MB (487 KB/page)
```

**Implementation:**
- Use `getrusage()` on macOS/Linux (RUSAGE_SELF)
- Report `ru_maxrss` (peak RSS)
- Convert bytes to human-readable (KB/MB/GB)
- Calculate per-page average

**Overhead:** ~0.01% (one system call at end)

**Files:**
- `examples/pdfium_cli.cpp` (add memory_reporter function)

**Effort:** 0.5 AI commit (~15 min)

---

### 6. Structured Logging (Low Priority)

**Problem:** Automated systems need machine-readable output.

**Solution:** JSON log output mode.

```bash
# Enable JSON logs
./pdfium_cli --log-format json render-pages test.pdf output/

# Output to stderr
{"level":"info","timestamp":"2025-01-15T10:23:45Z","message":"Starting render","pages":100}
{"level":"info","timestamp":"2025-01-15T10:23:48Z","message":"Progress","current":50,"total":100}
{"level":"info","timestamp":"2025-01-15T10:23:50Z","message":"Complete","duration":3.6,"pps":277}
```

**JSON Fields:**
- `level`: info | warn | error
- `timestamp`: ISO 8601
- `message`: Human-readable
- `...`: Context-specific fields

**Implementation:**
- Add `--log-format` flag (text | json, default text)
- Create `JSONLogger` class
- Emit JSON to stderr (keep progress on stdout)

**Overhead:** ~0.001% (JSON formatting is fast)

**Files:**
- `examples/pdfium_cli.cpp` (add JSONLogger class)

**Effort:** 1 AI commit (~30 min)

---

## Implementation Plan

### Phase 1: Core Features (2-3 AI commits)

**Commit 1: Progress Reporting + Performance Metrics**
- ProgressReporter class
- MetricsReporter class
- Test on large PDF (1000+ pages)

**Commit 2: Batch Processing**
- --batch flag implementation
- Directory iteration
- Error handling (continue on failure)
- Test on directory of 100 PDFs

**Commit 3: Better Error Messages**
- ErrorReporter class
- Map PDFium errors to user messages
- Test with malformed PDFs

### Phase 2: Polish (1-2 AI commits)

**Commit 4: Memory Reporting + Structured Logging**
- Memory usage tracking (getrusage)
- JSONLogger class (--log-format json)
- Test with monitoring scripts

### Phase 3: Testing & Documentation (1 AI commit)

**Commit 5: Integration Tests + Docs**
- Add tests for new flags
- Update README with examples
- Update --help text

---

## Testing Strategy

### Unit Tests

**Progress Reporting:**
- Test with 0 pages (no progress)
- Test with 1 page (immediate completion)
- Test with 1000 pages (full progress bar)

**Batch Processing:**
- Test with empty directory
- Test with single PDF
- Test with 100 PDFs
- Test with recursive flag
- Test error handling (malformed PDFs in batch)

**Error Messages:**
- Test all common error codes
- Verify actionable solutions provided

**Metrics:**
- Verify throughput calculation
- Verify threading efficiency calculation
- Verify smart mode percentage

### Integration Tests

Add to `integration_tests/tests/`:
- `test_012_progress_reporting.py`
- `test_013_batch_processing.py`
- `test_014_error_messages.py`
- `test_015_performance_metrics.py`

---

## Backward Compatibility

**All changes are additive:**
- Default behavior unchanged (no flags = same as v1.5.0)
- No breaking API changes
- All existing scripts continue to work

**New flags are opt-in:**
- `--batch` (off by default)
- `--log-format` (text by default)
- Progress bars (automatic for TTY, disabled for pipes)

---

## Performance Impact Analysis

| Feature | Overhead | Justification |
|---------|----------|---------------|
| Progress Reporting | 0.001% | One fprintf per 10 pages |
| Batch Processing | 0% | File I/O already dominates |
| Better Error Messages | 0% | Only on error path |
| Performance Metrics | 0.001% | One printf at end |
| Memory Reporting | 0.01% | One getrusage call |
| Structured Logging | 0.001% | JSON formatting is fast |

**Total Overhead:** <0.01%

**Validation:** Run full test suite with `--benchmark` flag to verify no regression.

---

## Documentation Updates

### README.md

**Add section:** "v1.6.0 Features - User Experience"

**Update Quick Start:**
```bash
# With progress bars and metrics
./pdfium_cli --threads 8 render-pages large.pdf images/
```

**Add Batch Processing Examples:**
```bash
# Process entire directory
./pdfium_cli --batch render-pages docs/ images/
```

### CLI Help Text

Update `--help` output:
```
Usage: pdfium_cli [FLAGS] <OPERATION> <INPUT> <OUTPUT>

New Flags (v1.6.0):
  --batch           Process directory of PDFs
  --pattern GLOB    File pattern for --batch (default: *.pdf)
  --recursive       Recursive directory search
  --log-format      Log format (text|json, default: text)
```

---

## Success Criteria

**v1.6.0 Release Checklist:**
- ✅ Progress bars work on TTY (tested with 1000-page PDF)
- ✅ Batch processing handles 100 PDFs without failure
- ✅ Error messages are actionable (tested with 10 common errors)
- ✅ Performance metrics accurate (±1% of actual)
- ✅ Memory reporting works (getrusage implemented)
- ✅ JSON logging valid (passes `jq` validation)
- ✅ All existing tests pass (2,760/2,760)
- ✅ No performance regression (<0.01% overhead measured)
- ✅ Documentation complete (README + help text)

---

## Risk Assessment

**Low Risk:**
- All features are optional (opt-in flags)
- No changes to core PDF processing
- No algorithm changes
- Extensive testing ensures stability

**Mitigation:**
- Full test suite validation before release
- Benchmark comparison vs v1.5.0
- Rollback plan: Remove new flags, keep core unchanged

---

## Post-Release

### User Feedback

Monitor for:
- Progress bar format preferences
- Batch processing use cases
- Error message clarity
- Metrics accuracy

### Future Enhancements (v1.7.0+)

Based on v1.6.0 feedback:
- Custom progress bar formats (--progress-format)
- Batch processing filters (--skip-errors, --only-errors)
- More metrics (CPU usage, disk I/O)
- Logging to file (--log-file)

---

## Copyright

**Copyright © 2025 Andrew Yates. All rights reserved.**

Dash PDF Extraction Roadmap
Version 1.6.0
Created: 2025-11-19
