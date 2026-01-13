# CRITICAL: --threads Flag Causes Rendering Differences (N=249)

**Date**: 2025-11-22
**Worker**: WORKER0 # 249
**Severity**: CRITICAL - Correctness Issue

## Executive Summary

The `--threads` flag changes rendering output, violating the fundamental requirement that **threading must not affect correctness**. This bug caused all test failures reported in N=248.

## Evidence

### Deterministic Reproduction

```bash
# Without --threads flag (default: 8 threads)
$ out/Release/pdfium_cli --format ppm --workers 1 --quality balanced render-pages pdfs/benchmark/fax_ccitt.pdf /tmp/test1/
$ md5 /tmp/test1/page_0007.ppm
MD5 (page_0007.ppm) = d5a6c077750f037ae04936c3d77fe0a8

# With --threads 1 flag
$ out/Release/pdfium_cli --format ppm --workers 1 --threads 1 --quality balanced render-pages pdfs/benchmark/fax_ccitt.pdf /tmp/test2/
$ md5 /tmp/test2/page_0007.ppm
MD5 (page_0007.ppm) = 83e669252e14a1086be3b9e2732be886
```

**Result**: Different MD5 hashes for identical PDF, settings, and binary!

### Scope of Impact

- **All PPM baseline tests**: Tests use `--threads 1`, baselines generated without `--threads`
- **452 PDFs affected**: Every PDF in test suite has potential mismatch
- **N=248 regeneration failure**: Used `--workers 1 --quality balanced` but didn't specify `--threads`, so generated wrong baselines

### Example Test Failures

- `fax_ccitt.pdf`: 14/50 pages mismatched (28%)
- `cc_001_931p.pdf`: 730/931 pages mismatched (78%)
- Many other PDFs affected

## Root Cause Analysis

### Code Path Differences

**File**: `examples/pdfium_cli.cpp`

**Multi-threaded path** (`thread_count > 1`, line 2716-2907):
1. Pre-scan for JPEG fast path (if smart mode enabled)
2. Pre-load all pages to populate caches (lines 2795-2806)
3. Render using `FPDF_RenderPagesParallelV2` (line 2850)

**Single-threaded path** (`thread_count == 1`, line 2909-2920):
1. No pre-scan
2. No pre-loading
3. Render using `render_page_to_png()` (line 2912)

### The Bug

The two paths use **different rendering APIs**:
- Multi-threaded: `FPDF_RenderPagesParallelV2` (C++ parallel rendering wrapper)
- Single-threaded: `render_page_to_png()` → `FPDF_RenderPageBitmap` (direct C API)

These APIs produce **different pixel output** for the same PDF!

## Impact Assessment

### Test Suite
- **2,791 tests** potentially affected
- N=248 reported: 2,454 passed, 337 failed (12% failure rate)
- Root cause: Baseline/test thread count mismatch

### Baselines
- Current baselines: Generated without `--threads` flag (default 8 threads, uses parallel path)
- Tests: Run with `--threads 1` (uses single-threaded path)
- **Mismatch**: Comparing apples to oranges

### N=248 Regeneration
The regeneration script at N=248 used:
```python
cmd = ['--format', 'ppm', '--workers', 'N', '--quality', 'balanced', 'render-pages', ...]
```

Missing: `--threads 1` flag! So it used default (8 threads), generating baselines for parallel path while tests use single-threaded path.

## Workaround Options

### Option 1: Add --threads 1 to Baseline Regeneration (RECOMMENDED)
**Pros**:
- Tests already use `--threads 1`
- Single-threaded is authoritative baseline
- Minimal test changes

**Cons**:
- Doesn't fix underlying correctness bug
- Parallel rendering still produces different output

**Implementation**:
```python
# lib/regenerate_ppm_baselines.py line 79-88
cmd = [
    str(self.cli_bin),
    '--format', 'ppm',
    '--workers', str(workers),
    '--threads', '1',  # ADD THIS
    '--quality', quality,
    'render-pages',
    str(pdf_path),
    str(temp_path)
]
```

### Option 2: Fix Parallel Rendering to Match Single-Threaded
**Pros**:
- Fixes root cause
- Threading becomes performance-only (correct design)

**Cons**:
- Complex fix
- Requires deep PDFium debugging
- May require upstream investigation

### Option 3: Make Tests Use Default Threading
**Pros**:
- Tests would match production usage

**Cons**:
- Changes many test files
- Parallel rendering has stability issues at K>=4 (N=210)
- Less deterministic testing

## Recommended Action

**Immediate (N=249)**:
1. Add `--threads 1` to baseline regeneration script
2. Regenerate all 452 PPM baselines
3. Run full test suite to verify

**Follow-up (N=250+)**:
1. Investigate why parallel vs single-threaded rendering differs
2. File PDFium upstream bug if needed
3. Consider removing `--threads` flag from tests once parallel rendering matches

## Historical Context

### MANAGER Commit (af5cbcd62a)
The MANAGER commit accidentally reverted 3 baselines:
- `fax_ccitt.json`
- `japanese_008.json`
- `japanese_013.json`

These files had correct MD5s for parallel rendering (`83e669...` for fax_ccitt page 7) but were reverted to old MD5s (`d5a6c077...`).

However, this was actually CORRECT because tests use `--threads 1` (single-threaded), which produces `d5a6c077...`!

The real bug was that baselines at N=246 were generated with parallel rendering, not single-threaded.

## Expiration

**N=248 Claims** (NOW INCORRECT):
- "Only 73 files needed updating" - FALSE, all 452 files need regeneration with `--threads 1`
- "379 files already had correct MD5s" - FALSE, they had MD5s for wrong rendering path
- "Both modified and unmodified baselines are correct" - FALSE, only 3 files were correct (reverted by MANAGER)

## Next Steps

1. Modify `lib/regenerate_ppm_baselines.py` to add `--threads 1`
2. Run regeneration: `python lib/regenerate_ppm_baselines.py --all`
3. Commit all 452 updated baselines
4. Run full test suite: `pytest -v`
5. Verify 100% pass rate
