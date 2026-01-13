# Flag Simplification: Ultra-Thinking Better UX

**Problem:** Flags are confusing (--batch, --recursive, --workers, --threads, --preset, --dpi, --format)

**User asks:** "What is --recursive? Why do I need it?"

---

## Current Problems

### Problem 1: Too Many Flags

**Current complexity:**
```bash
# User must remember:
pdfium_cli --batch --recursive --workers 4 --preset web render-pages /pdfs/ /output/
#           ^^^^^^^ ^^^^^^^^^^^ ^^^^^^^^^^ ^^^^^^^^^^^
#             5 flags just to process a directory!
```

**What each flag does:**
- `--batch`: Enable directory processing (why not auto-detect?)
- `--recursive`: Search subdirectories (why not default?)
- `--workers 4`: Parallelism level (why not auto-select?)
- `--preset web`: Output format (good, but could be default for images)
- Operation: render-pages (ok)

### Problem 2: --workers vs --threads Confusion

**Current:**
- `--workers N`: Multi-process parallelism (1-16)
- `--threads K`: Multi-threading per process (1-32)
- Total parallelism: N × K

**User confusion:**
- "Which one do I use?"
- "What's the difference?"
- "How do I choose values?"

### Problem 3: --batch Flag Shouldn't Exist

**Current:** Must use --batch for directories
```bash
pdfium_cli --batch extract-text /pdfs/ /output/  # Directory
pdfium_cli extract-text input.pdf output.txt     # Single file
```

**Better:** Auto-detect input type
```bash
pdfium_cli extract-text /pdfs/ /output/     # Auto-detects directory
pdfium_cli extract-text input.pdf output.txt  # Auto-detects file
```

### Problem 4: Recursive Should Be Default

**Current:** Must specify --recursive
```bash
pdfium_cli --batch --recursive ...  # Search subdirs
pdfium_cli --batch ...              # Only top level
```

**Better:** Recursive by default
```bash
pdfium_cli extract-text /pdfs/ /output/        # Recursive (default)
pdfium_cli --no-recursive extract-text /pdfs/ /output/  # Top-level only (rare)
```

### Problem 5: Format Defaults Aren't Smart

**Current:** Always defaults to PNG
```bash
pdfium_cli render-pages /pdfs/ /output/  # Creates multi-TB PNG output!
```

**Better:** Smart defaults based on use case
```bash
# For rendering, default to JPEG (most users want this)
pdfium_cli render-pages /pdfs/ /output/  # Creates JPEG (37 GB, not 3 TB)

# For text, default to UTF-8 (not UTF-32 LE)
pdfium_cli extract-text input.pdf output.txt  # UTF-8 (not UTF-32 LE)
```

---

## Proposed Simplified Design

### Design Principle: Zero Configuration

**Goal:** Work intelligently with NO flags for 90% of use cases

### New Interface (v2.0.0)

```bash
# TEXT EXTRACTION - Just works
pdfium_cli extract-text /pdfs/ /output/
# Auto-detects: directory → batch mode, recursive, 4 workers, UTF-8 output

pdfium_cli extract-text input.pdf output.txt
# Auto-detects: single file → direct extraction, UTF-8 output

# IMAGE RENDERING - Smart defaults
pdfium_cli render-pages /pdfs/ /images/
# Auto-detects: directory → batch mode, recursive
# Auto-defaults: JPEG format (not PNG!), web quality (150 DPI)
# Output: 37 GB JPEG (not 3.1 TB PNG)

pdfium_cli render-pages input.pdf images/
# Auto-detects: single file
# Auto-defaults: JPEG web preset
```

### Simplified Flags (Only When Needed)

**Essential (5 flags):**
```
--parallel N      Total parallelism (replaces --workers and --threads)
--preset MODE     Quality preset: web|thumbnail|print|max (default: web for images)
--pages START-END Page range selection
--debug           Detailed logging
-h, --help        Show help
```

**Advanced (4 flags, rare use):**
```
--no-recursive    Don't search subdirectories (default: recursive)
--pattern GLOB    File pattern (default: *.pdf)
--dpi N           Override preset DPI (advanced)
--format FMT      Override preset format (advanced)
```

**Removed (obsolete):**
```
--batch      → Auto-detect (directory = batch mode)
--workers N  → Merged into --parallel
--threads K  → Merged into --parallel
--adaptive   → Always adaptive (auto-select based on workload)
```

---

## Comparison: Before vs After

### Before (v1.9.0 - Current)

**Confusing:**
```bash
# User must know 5+ flags
pdfium_cli --batch --recursive --workers 4 --preset web render-pages /pdfs/ /out/
```

### After (v2.0.0 - Proposed)

**Simple:**
```bash
# Just works (intelligent defaults)
pdfium_cli render-pages /pdfs/ /output/
```

**Advanced (if needed):**
```bash
# Override defaults
pdfium_cli --parallel 8 --preset thumbnail render-pages /pdfs/ /output/
```

---

## Smart Defaults

### Default Behaviors (No Flags)

**For text extraction:**
- Output: UTF-8 (not UTF-32 LE)
- Parallelism: 4 workers (auto-selected)
- Recursive: Yes (if directory)

**For image rendering:**
- Format: JPEG (not PNG!)
- Preset: web (150 DPI q85)
- Parallelism: 8 threads (auto-selected)
- Recursive: Yes (if directory)

**Rationale:**
- Most users want JPEG images (not multi-TB PNG)
- Most users want UTF-8 text (not UTF-32 LE)
- Most users want recursive directory search
- Auto-selection works better than manual tuning

---

## Migration Guide (v1.9 → v2.0)

**Old (v1.9):**
```bash
pdfium_cli --batch --recursive --workers 4 --preset web render-pages /pdfs/ /output/
```

**New (v2.0):**
```bash
pdfium_cli render-pages /pdfs/ /output/
# Exact same behavior, 5 flags removed!
```

**Backwards compatibility:**
- Old flags still work (deprecated warnings)
- New defaults can be overridden

---

## Implementation Effort

**Changes needed:**
1. Auto-detect file vs directory (5 lines)
2. Make recursive default (1 line)
3. Change default format to JPEG for render-pages (1 line)
4. Change default encoding to UTF-8 for extract-text (10 lines)
5. Add --parallel flag (merge --workers and --threads logic)
6. Update help text

**Total: 2-3 commits (~2 hours)**

---

## User Benefits

**Before:** User must understand 9+ flags
**After:** User runs with ZERO flags, gets smart defaults

**Example:**
```bash
# User wants images from 100K PDFs
# Before: Must learn --batch, --recursive, --preset
# After: Just run this
pdfium_cli render-pages /pdfs/ /images/

# Output: 37 GB JPEG (smart default!)
# Time: 2 hours
```

**This is how tools should work** - intelligent, not configurable.

---

## Recommendation

**v2.0.0 focus:** Simplify flags, smart defaults

**Priority changes:**
1. Auto-detect directory (remove --batch)
2. Recursive by default (remove --recursive)
3. JPEG default for images (not PNG)
4. UTF-8 default for text (not UTF-32 LE)
5. Merge --workers and --threads into --parallel

**Result:** 90% of users need ZERO flags

**This is the UX improvement that matters.**
