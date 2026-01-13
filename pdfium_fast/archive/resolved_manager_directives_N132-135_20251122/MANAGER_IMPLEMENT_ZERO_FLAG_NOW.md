# MANAGER: Implement Zero-Flag Defaults NOW

**To:** WORKER0 (N=128+)
**Priority:** CRITICAL - User directive
**User quote:** "make the library behave reasonably by default"

---

## USER DIRECTIVE: Implement Smart Defaults

**Goal:** 90% of users should need ZERO flags

**Implement immediately** (not v2.0.0, do it now)

---

## Implementation: 5 Changes (N=128-132)

### N=128: Auto-Detect Directory (Remove --batch Requirement)

**File:** `examples/pdfium_cli.cpp`

**Current:** Requires --batch flag for directories

**Change:**
```cpp
// Around line 1200 (after parsing arguments)
// Auto-detect if input is directory
struct stat st;
bool is_directory = (stat(pdf_path, &st) == 0 && S_ISDIR(st.st_mode));

if (is_directory && !batch_mode) {
  // Auto-enable batch mode
  batch_mode = true;
  fprintf(stderr, "Auto-detected directory input, enabling batch mode\n");
}

// Rest of code continues...
if (batch_mode) {
  return ProcessBatch(...);
}
```

**Result:**
```bash
# NEW: Just works (no --batch flag needed)
./pdfium_cli extract-text /pdfs/ /output/

# OLD: Still works (backward compatible)
./pdfium_cli --batch extract-text /pdfs/ /output/
```

**Test:**
```bash
# Should work without --batch
./pdfium_cli extract-text integration_tests/pdfs/benchmark/ /tmp/test_auto/
# Should process all PDFs automatically
```

**Commit:**
```
[WORKER0] # 128: Auto-Detect Directory (Remove --batch Requirement)

User directive: Make library behave reasonably by default.

Change: Auto-detect if input is directory, enable batch mode automatically.

Before: pdfium_cli --batch extract-text /pdfs/ /output/
After:  pdfium_cli extract-text /pdfs/ /output/  (works!)

Backward compatible: --batch flag still works (no-op now).

Tests: Verify batch processing works without --batch flag.
```

---

### N=129: Recursive by Default

**File:** `examples/pdfium_cli.cpp`

**Current:** recursive = false (default)

**Change:**
```cpp
// Around line 735 (flag defaults)
bool recursive = true;  // Changed from false to true

// Update flag parsing
if (strcmp(argv[arg_idx], "--recursive") == 0) {
  recursive = true;  // Now redundant but keep for backward compat
  arg_idx++;
} else if (strcmp(argv[arg_idx], "--no-recursive") == 0) {
  recursive = false;  // NEW flag to disable
  arg_idx++;
}
```

**Result:**
```bash
# NEW: Recursive by default
./pdfium_cli extract-text /pdfs/ /output/
# Searches all subdirectories automatically

# Disable if needed (rare)
./pdfium_cli --no-recursive extract-text /pdfs/ /output/
```

**Commit:**
```
[WORKER0] # 129: Recursive by Default

Changed default: recursive = true (was false).

Most users want recursive directory search (who wants to skip subdirs?).

Before: pdfium_cli --batch --recursive extract-text /pdfs/ /out/
After:  pdfium_cli extract-text /pdfs/ /out/  (recursive by default)

Added --no-recursive flag for rare cases (top-level only).

Backward compatible: --recursive still works (redundant).
```

---

### N=130: JPEG Web Preset Default for render-pages

**File:** `examples/pdfium_cli.cpp`

**Current:** Defaults to PNG 300 DPI (creates TB-scale output!)

**Change:**
```cpp
// Around line 1100 (before operation dispatch)
if (operation == Operation::RENDER_PAGES) {
  // Smart defaults for image rendering (prevent TB-scale output)
  if (!user_set_format && !user_set_preset) {
    // Default to web preset (150 DPI JPEG)
    // Most users want JPEG for storage efficiency
    preset = RenderPreset::WEB;
    fprintf(stderr, "Using default: --preset web (150 DPI JPEG q85)\n");
    fprintf(stderr, "Override with --preset print for 300 DPI PNG\n");
  }
}

// Apply preset defaults
auto config = PRESET_CONFIGS[static_cast<int>(preset)];
// ... apply dpi, format, quality
```

**Result:**
```bash
# NEW: JPEG by default (37 GB not 3 TB!)
./pdfium_cli render-pages /pdfs/ /images/
# Output: 150 DPI JPEG q85 (web preset)

# Override for high quality
./pdfium_cli --preset print render-pages /pdfs/ /images/
# Output: 300 DPI PNG (original behavior)
```

**Critical:** This prevents users from accidentally creating TB-scale PNG output!

**Commit:**
```
[WORKER0] # 130: JPEG Web Preset Default for render-pages

CRITICAL UX fix: Prevent TB-scale PNG output by default.

Default: render-pages uses web preset (150 DPI JPEG q85)
Override: --preset print for 300 DPI PNG

Before: pdfium_cli render-pages /pdfs/ /images/ → 3.1 TB PNG
After:  pdfium_cli render-pages /pdfs/ /images/ → 37 GB JPEG

Shows notice: \"Using default: --preset web\"
Shows override: \"Use --preset print for 300 DPI PNG\"

Tests: Verify default creates JPEG, --preset print creates PNG.

This solves the 4.5 TB problem BY DEFAULT.
```

---

### N=131: Auto-Select Workers (Smart Parallelism)

**File:** `examples/pdfium_cli.cpp`

**Current:** Defaults to 1 worker (slow)

**Change:**
```cpp
// Around line 730 (defaults)
int worker_count = 0;  // 0 = auto-select (was 1)

// Around line 1150 (before operation)
if (worker_count == 0) {
  // Auto-select based on operation and input type
  if (batch_mode) {
    worker_count = 4;  // Good for batch processing
    fprintf(stderr, "Auto-selected 4 workers for batch processing\n");
  } else if (operation == Operation::EXTRACT_TEXT) {
    // Single file text extraction
    int page_count = get_page_count(pdf_path);
    if (page_count >= 200) {
      worker_count = 4;  // Large PDF benefits from workers
    } else {
      worker_count = 1;  // Small PDF: single-threaded
    }
  } else {
    worker_count = 1;  // Single file rendering: use threads instead
  }
}

// Similar for threads
if (thread_count == 8 && !user_set_threads) {
  // Default thread count, can be smarter
  if (operation == Operation::RENDER_PAGES && !batch_mode) {
    thread_count = 8;  // Good default for rendering
  }
}
```

**Result:**
```bash
# NEW: Automatically uses 4 workers for batch
./pdfium_cli extract-text /pdfs/ /output/
# Auto: 4 workers

# Override if needed
./pdfium_cli --workers 1 extract-text /pdfs/ /output/
```

**Commit:**
```
[WORKER0] # 131: Auto-Select Workers (Smart Parallelism)

Auto-select parallelism based on operation:
- Batch mode: 4 workers (optimal for directories)
- Single file: 1 worker (unless large PDF)
- Rendering: 8 threads (optimal for images)

Before: pdfium_cli extract-text /pdfs/ /out/  → 1 worker (slow)
After:  pdfium_cli extract-text /pdfs/ /out/  → 4 workers (optimal)

Shows notice: \"Auto-selected 4 workers for batch processing\"

User can still override: --workers N
```

---

### N=132: Update Help Text (Show New Defaults)

**File:** `examples/pdfium_cli.cpp`

**Update help to show new defaults:**

```cpp
fprintf(stderr, "Flags (all optional with smart defaults):\n");
fprintf(stderr, "  --workers N       Multi-process workers (default: auto, 1-4 based on workload)\n");
fprintf(stderr, "  --threads K       Render threads (default: 8 for images, 1 for text)\n");
fprintf(stderr, "  --preset MODE     Output preset (default: web for render-pages, none for extract-text)\n");
fprintf(stderr, "  --no-recursive    Don't search subdirectories (default: recursive)\n");
fprintf(stderr, "  --format FMT      Override preset format\n");
fprintf(stderr, "  --dpi N           Override preset DPI\n");
fprintf(stderr, "\n");
fprintf(stderr, "Smart Defaults (Zero-Configuration):\n");
fprintf(stderr, "  • Directory input: Batch mode enabled automatically\n");
fprintf(stderr, "  • Subdirectories: Recursive search by default\n");
fprintf(stderr, "  • Image rendering: JPEG web preset (150 DPI, prevents TB-scale output)\n");
fprintf(stderr, "  • Parallelism: Auto-selected (4 workers for batch, 8 threads for images)\n");
fprintf(stderr, "  • Override any default with explicit flags\n");
```

**Commit:**
```
[WORKER0] # 132: Update Help Text - Document Smart Defaults

Updated --help to show new zero-flag defaults:
- Auto-detect directory
- Recursive by default
- JPEG web preset for images
- Auto-select parallelism

Examples updated to show zero-flag usage:
  pdfium_cli extract-text /pdfs/ /output/           (just works!)
  pdfium_cli render-pages /pdfs/ /images/           (JPEG, not TB PNG!)

Help text emphasizes: \"All optional with smart defaults\"
```

---

## Result: Zero-Flag Interface

**Before (v1.9.0):**
```bash
pdfium_cli --batch --recursive --workers 4 --preset web render-pages /pdfs/ /out/
```

**After (N=128-132):**
```bash
pdfium_cli render-pages /pdfs/ /output/
# Same result, ZERO flags!
# Auto: directory, recursive, 4 workers, JPEG web
```

---

## Testing Checklist (N=133)

**Verify zero-flag usage works:**

```bash
# Test 1: Text extraction (directory)
./pdfium_cli extract-text integration_tests/pdfs/benchmark/ /tmp/test_text/
# Should: Auto-detect, recursive, 4 workers

# Test 2: Image rendering (directory)
./pdfium_cli render-pages integration_tests/pdfs/benchmark/ /tmp/test_images/
# Should: Auto-detect, recursive, JPEG web preset
# Check: Files are .jpg not .png

# Test 3: Single file
./pdfium_cli extract-text integration_tests/pdfs/benchmark/arxiv_001.pdf /tmp/test.txt
# Should: Single file mode

# Test 4: Override defaults
./pdfium_cli --preset print render-pages integration_tests/pdfs/benchmark/ /tmp/test_png/
# Should: Create PNG files (override JPEG default)

# Run smoke tests
cd integration_tests && pytest -m smoke
# Should: 96/96 pass
```

**Commit:**
```
[WORKER0] # 133: Validate Zero-Flag Defaults

Tested all auto-detection and smart defaults:
✓ Directory auto-detected
✓ Recursive by default
✓ JPEG web preset for images
✓ Auto-selected workers

Smoke tests: 96/96 pass

Ready for production.
```

---

## Total Effort

**5 commits, ~3 hours:**
- N=128: Auto-detect directory
- N=129: Recursive default
- N=130: JPEG default
- N=131: Auto-select workers
- N=132: Update help
- N=133: Test

**Result:** Dramatically simpler interface

---

## WORKER: IMPLEMENT THIS NOW

These are user-facing critical changes.

Start with N=128 (auto-detect directory).

**Make it so users can just type:**
```bash
pdfium_cli render-pages /pdfs/ /images/
```

**And get 37 GB JPEG output (not 3 TB PNG).**

This is the RIGHT default!
