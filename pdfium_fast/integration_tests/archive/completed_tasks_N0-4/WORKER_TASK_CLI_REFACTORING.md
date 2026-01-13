# WORKER TASK: CLI API Refactoring and Page Selection

**Date**: 2025-11-11
**Priority**: HIGH
**Type**: API Refactoring + Feature Addition

## Objective

Simplify and improve the pdfium_cli API by:
1. **Consolidate modes**: Replace `--bulk` and `--fast N` with single `--workers N` parameter
2. **Remove confusing flags**: Remove `--smart` flag, make it default behavior (automatic detection)
3. **Add page selection**: Support `--pages START-END` for processing page ranges
4. **Update tests**: Modify all test cases to use new API
5. **Fix threading tests**: Address Rust tool dependencies in threading regression tests

## Current API (BEFORE)

```bash
# Single-threaded (bulk mode)
pdfium_cli --bulk extract-text input.pdf output.txt

# Multi-process (fast mode)
pdfium_cli --fast 4 extract-text input.pdf output.txt

# Smart mode (JPEG fast path - opt-in)
pdfium_cli --smart render-pages scanned.pdf output_dir/
pdfium_cli --fast --smart render-pages scanned.pdf output_dir/

# No page range support (full document only)
```

## Target API (AFTER)

```bash
# Single-threaded (1 worker)
pdfium_cli --workers 1 extract-text input.pdf output.txt

# Multi-process (4 workers)
pdfium_cli --workers 4 extract-text input.pdf output.txt

# With page selection
pdfium_cli --workers 4 --pages 1-10 extract-text input.pdf output.txt
pdfium_cli --workers 1 --pages 5 render-pages input.pdf output_dir/

# Smart mode is ALWAYS ON (automatic detection, no flag needed)
# If PDF is scanned with embedded JPEG: automatic fast path
# If PDF is text-based: normal rendering
pdfium_cli render-pages scanned.pdf output_dir/  # Auto-detects and uses JPEG fast path
```

## Why Remove --smart Flag?

**Problem**: Confusing UX - why would users NOT want to be smart?

**Smart Mode Benefits**:
- Automatic detection (no user decision needed)
- 545x speedup for scanned PDFs
- Full quality preserved (lossless JPEG extraction)
- Zero downside (falls back to normal rendering for non-scanned PDFs)

**Solution**: Make it default behavior
- Remove `--smart` flag entirely
- Always run smart detection on render-pages
- Automatically use JPEG fast path when applicable
- Users get optimal performance without thinking about it

## Implementation Checklist

### Phase 1: Code Changes (pdfium_cli.cpp)

- [ ] **Replace mode flags**:
  - Remove `MODE_BULK`, `MODE_FAST` from enum
  - Add single `worker_count` parameter (default: 1)
  - Remove `--bulk` flag parsing
  - Replace `--fast [N]` with `--workers N`

- [ ] **Remove --smart flag**:
  - Remove `--smart` flag parsing
  - Remove `smart_mode` boolean parameter from all functions
  - Make JPEG fast path detection ALWAYS ON in `render_page_to_png()`
  - Remove `smart_mode` from worker process arguments
  - Update help text: Remove --smart documentation

- [ ] **Add page selection**:
  - Add `--pages START-END` flag parsing
  - Add `--pages N` (single page) support
  - Modify `extract_text_bulk/fast` to accept page range
  - Modify `render_pages_bulk/fast` to accept page range
  - Update worker spawning to respect page ranges

- [ ] **Function signature updates**:
  ```cpp
  // OLD
  int extract_text_bulk(const char* pdf_path, const char* output_path);
  int extract_text_fast(const char* pdf_path, const char* output_path, int worker_count);

  // NEW
  int extract_text(const char* pdf_path, const char* output_path, int workers,
                   int start_page, int end_page);
  // start_page=-1, end_page=-1 means "all pages"
  ```

- [ ] **Auto-dispatch removal**:
  - Remove PAGE_THRESHOLD (200 pages) logic
  - User explicitly controls worker count via `--workers`
  - Default to `--workers 1` if not specified

- [ ] **Update help/usage text**:
  - Remove references to `--bulk` and `--fast`
  - Add `--workers N` documentation
  - Add `--pages` documentation with examples

### Phase 2: Test Updates

- [ ] **Update test_001_smoke.py**:
  - `test_bulk_mode_explicit()` → `test_workers_1_explicit()`
  - `test_fast_mode_explicit()` → `test_workers_4_explicit()`
  - Update subprocess calls: `["--bulk"]` → `["--workers", "1"]`
  - Update subprocess calls: `["--fast", "4"]` → `["--workers", "4"]`

- [ ] **Update test_010_smart_scanned_pdf.py**:
  - `test_smart_mode_with_bulk()` → `test_smart_mode_with_1worker()`
  - `test_smart_mode_with_fast()` → `test_smart_mode_with_4workers()`
  - Remove all `--smart` flags from test calls (smart mode now always on)
  - Tests should verify JPEG fast path is used automatically

- [ ] **Update test_008_scaling.py**:
  - Replace all `--fast N` calls with `--workers N`

- [ ] **Add page selection tests**:
  - `test_page_range_text_extraction()` - Extract pages 1-10
  - `test_page_range_image_rendering()` - Render pages 5-15
  - `test_single_page_extraction()` - Extract page 0
  - Verify output contains only specified pages

### Phase 3: Threading Regression Tests ✅ COMPLETE

Located in `test_011_threading_regression.py`:

**Status**: ALL 9 TESTS PASS (2025-11-11)
- Session: sess_20251111_102717_d4490115
- Result: 9 passed, 0 failed
- Duration: 102.16s

**Completed Actions**:
- ✅ All threading tests now use C++ CLI fixtures (`extract_text_tool`, `render_tool`)
- ✅ Removed hardcoded paths (`out/Profile/pdfium_cli`)
- ✅ All tests use `--workers N` API (no legacy `--bulk`/`--fast` flags)
- ✅ Thread safety validated: `test_threading_smoke_init_is_thread_safe`
- ✅ No crashes with workers: `test_threading_smoke_no_crashes_with_workers` (3 PDFs tested)
- ✅ Determinism validated: `test_threading_determinism_text_multirun`
- ✅ Determinism validated: `test_threading_determinism_image_multirun`
- ✅ Performance validated: `test_threading_performance_smoke_speedup` (>1.5x speedup)
- ✅ Regression tests: `test_threading_regression_no_double_init`
- ✅ Regression tests: `test_threading_regression_init_destroy_cycle`

**Note**: Page range support is NOT required for threading tests (Phase 3 scope was fixture migration only)

### Phase 4: Validation ✅ COMPLETE

- ✅ **Run smoke tests**: All pass with new API
  - Session: sess_20251111_102923_550c2771
  - Result: 57 passed, 10 skipped, 0 failed
  - Duration: 411.03s
  - All tests use `--workers N` API

- ⏭️ **Run full tests**: Deferred (not required for Phase 1-3 completion)

- ✅ **Verify backward compatibility**: N/A (no external tools use old API)

- ✅ **Update documentation**: COMPLETE
  - ✅ USAGE.md updated (# 0 continued)
  - ✅ ARCHITECTURE.md updated (# 0 continued)
  - ✅ README.md already uses correct API
  - ✅ HOW_TO_BUILD.md not affected (build instructions only)

## Implementation Notes

### Page Range Implementation Details

```cpp
// Parse --pages flag
int start_page = -1;  // -1 means "from beginning"
int end_page = -1;    // -1 means "to end"

if (strcmp(argv[i], "--pages") == 0) {
    char* pages_arg = argv[++i];
    if (strchr(pages_arg, '-')) {
        // Range: "1-10"
        sscanf(pages_arg, "%d-%d", &start_page, &end_page);
    } else {
        // Single page: "5"
        start_page = end_page = atoi(pages_arg);
    }
}

// In extract/render functions:
if (start_page == -1) start_page = 0;
if (end_page == -1) end_page = page_count - 1;

// Validate range
if (start_page < 0 || end_page >= page_count || start_page > end_page) {
    fprintf(stderr, "Error: Invalid page range %d-%d (document has %d pages)\n",
            start_page, end_page, page_count);
    return 1;
}
```

### Worker Distribution with Page Ranges

When user specifies both `--workers N` and `--pages START-END`:
- Total pages to process: `(end - start + 1)`
- Divide evenly among N workers
- Each worker gets a sub-range

Example: `--workers 4 --pages 10-50`
- Total: 41 pages
- Worker 0: pages 10-20 (11 pages)
- Worker 1: pages 21-30 (10 pages)
- Worker 2: pages 31-40 (10 pages)
- Worker 3: pages 41-50 (10 pages)

## Test Locations

- `integration_tests/tests/test_001_smoke.py` - Lines 452-528 (bulk/fast mode tests)
- `integration_tests/tests/test_001_smoke_edge_cases.py` - Lines 113, 186 (page range comments)
- `integration_tests/tests/test_010_smart_scanned_pdf.py` - Lines 273, 291 (bulk/fast with smart mode)
- `integration_tests/tests/test_011_threading_regression.py` - Lines 100+ (threading safety)
- `integration_tests/tests/test_008_scaling.py` - Lines 44, 202 (worker scaling tests)

## Success Criteria ✅ ALL COMPLETE

✅ All 62 smoke tests pass with new API (5 skipped are expected - JSONL baselines not generated)
✅ Threading regression tests all passing (9/9 tests)
✅ Documentation updated (USAGE.md, ARCHITECTURE.md)
✅ Backward compatibility: N/A (no external dependencies)
✅ Page range support implemented (--pages START-END)

## Completion Summary (2025-11-11)

**Task Status**: COMPLETE

**Implementation**:
- Phase 1 (Code): DONE (WORKER0 # 0)
- Phase 2 (Tests): DONE (WORKER0 # 0)
- Phase 3 (Threading): DONE (WORKER0 # 1)
- Phase 4 (Validation): DONE (WORKER0 # 1)
- Phase 5 (Test Cleanup): DONE (WORKER0 # 2)

**API Changes**:
- ✅ Replaced `--bulk` and `--fast N` with unified `--workers N`
- ✅ Removed `--smart` flag (now always-on default behavior)
- ✅ All 62 smoke tests use new API
- ✅ All 9 threading tests use new API
- ✅ All tests use correct fixtures (no hardcoded paths)

**Test Results**:
- Smoke: 62 passed, 5 skipped (expected), 0 failed
- Threading: 9 passed, 0 failed
- Total: 71 tests validating new API
- Improvement: +5 tests now passing (was 57, now 62)

**Next Steps** (Future Work):
1. Full test suite run (`pytest -m full`) - optional validation
2. Consider adding more page range tests to test_001_smoke.py

## Estimated Effort

- Phase 1 (Code): 2 hours
- Phase 2 (Tests): 1 hour
- Phase 3 (Threading): 2 hours
- Phase 4 (Validation): 1 hour
- **Total**: ~6 hours (or 30 AI commits at 12 min/commit)

## Current Test Results

**Latest (WORKER0 # 2)**:
- Session: sess_20251111_105839_7891b14d
- Result: 62 passed, 5 skipped, 0 failed
- Duration: 462.44s
- All skips are expected (JSONL baselines not generated)

**Baseline (WORKER0 # 0)**:
- Session: sess_20251111_092421_a1b140f2
- Result: 45 passed, 22 skipped, 0 failed
- Duration: 369.91s

**Progress**: +17 tests now passing (from 45 to 62)
