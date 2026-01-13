# Upstream Validation Plan

**Date**: 2025-11-01 23:20 PST
**Context**: User correctly identified we need baseline validation against upstream before adding complexity

---

## User's Insight

> "it needs to validate against what we can get from upstream. if we do that, and have a baseline validation at the start, then that's pretty good as we add complexity"

**This is correct.** Testing strategy should be:
1. Validate single-threaded Rust tools match upstream PDFium
2. THEN add multi-threading
3. Validate multi-threaded matches single-threaded (which matches upstream)

---

## Current State

### What We Have

**Upstream PDFium Library**: `out/Optimized-Shared/libpdfium.dylib`
- Built from: Git commit 7f43fd79 (2025-10-30)
- Source: https://pdfium.googlesource.com/pdfium/
- MD5: 00cd20f999bf60b1f779249dbec8ceaa
- Build date: 2025-10-31 02:11
- **0 C++ modifications** (verified in CLAUDE.md)

**Our Rust Tools**:
- `rust/target/release/examples/extract_text` - Calls FPDFText_GetText()
- `rust/target/release/examples/extract_text_jsonl` - Calls 13 FPDFText_* APIs
- `rust/target/release/examples/render_pages` - Calls FPDF_RenderPage()

**Key fact**: Our Rust tools are **thin wrappers** around upstream libpdfium.dylib. They don't reimplement anything - they just call the C API.

### The Validation Question

**Question**: How do we validate our Rust tools match upstream?

**Answer**: Since our Rust tools call the same APIs as any C++ code would, we need to verify:
1. ✅ Library is unmodified upstream (confirmed)
2. ✅ Rust bindings are correct (calling right functions)
3. ✅ Output format matches expectations (UTF-32 LE, etc.)
4. ⚠️ No bugs in our wrapper code (need to verify)

---

## Validation Strategy

### Phase 1: Verify Library is Upstream ✅

**Already confirmed** in CLAUDE.md:
```
Upstream: Git 7f43fd79 (2025-10-30) from https://pdfium.googlesource.com/pdfium/
Binary MD5: 00cd20f999bf (libpdfium.dylib built 2025-10-31 02:11)
Verified: 0 C++ modifications vs upstream (only Rust/Python/tooling added on branch)
```

**Status**: ✅ DONE

### Phase 2: Create C++ Reference Tool

**Problem**: `pdfium_test` doesn't extract text - it only renders images.

**Solution**: Create simple C++ tool that calls same APIs as our Rust tools.

**New tool**: `examples/reference_text_extract.cpp`

```cpp
// Calls exact same API as our Rust tools
// Uses FPDFText_GetText() just like extract_text.rs
// Output: UTF-32 LE (same format)
```

**Purpose**: Prove that calling FPDFText_GetText() from C++ produces identical output to calling from Rust.

**Expected result**: Byte-for-byte identical output.

### Phase 3: Validate Rust Single-Threaded Tools

**Test matrix**:

| Tool | C++ Reference | Rust Tool | Expected |
|------|---------------|-----------|----------|
| Text extraction | reference_text_extract | extract_text (1 worker) | Identical |
| JSONL extraction | reference_jsonl_extract | extract_text_jsonl | Identical |
| Image rendering | pdfium_test | render_pages (1 worker) | MD5 match |

**Command**:
```bash
# Text
./out/Optimized-Shared/reference_text_extract input.pdf > cpp_output.txt
./rust/target/release/examples/extract_text input.pdf rust_output.txt 1
diff cpp_output.txt rust_output.txt  # Should be empty

# JSONL (just verify format - metadata correctness is inherent to FPDFText APIs)
./out/Optimized-Shared/reference_jsonl_extract input.pdf > cpp_output.jsonl
./rust/target/release/examples/extract_text_jsonl input.pdf rust_output.jsonl
diff cpp_output.jsonl rust_output.jsonl  # Should be empty

# Images
./out/Optimized-Shared/pdfium_test input.pdf  # Generates .ppm files
./rust/target/release/examples/render_pages input.pdf /tmp/rust 1 300
# Compare MD5 of ppm vs png (after format conversion)
```

**Success criteria**: 0 differences on 10-20 representative PDFs.

### Phase 4: Validate Multi-Threading Doesn't Break Correctness

**Test**: Multi-threaded Rust vs single-threaded Rust

**Current tests** (already implemented):
- test_002: 1-worker vs 4-worker text extraction
- test_005: 1-worker vs 4-worker image rendering

**What this validates**:
- Multi-threading is deterministic
- No race conditions
- No page order bugs
- No data corruption

**Expected**: Byte-for-byte identical (UTF-32 LE, PNG)

---

## Implementation Plan

### Step 1: Create C++ Reference Tools (1 hour)

**File 1**: `examples/reference_text_extract.cpp`
```cpp
#include "public/fpdfview.h"
#include "public/fpdf_text.h"

int main(int argc, char* argv[]) {
    FPDF_InitLibrary();
    FPDF_DOCUMENT doc = FPDF_LoadDocument(argv[1], NULL);

    // Write BOM for UTF-32 LE
    fwrite("\xFF\xFE\x00\x00", 4, 1, stdout);

    for (int i = 0; i < FPDF_GetPageCount(doc); i++) {
        FPDF_PAGE page = FPDF_LoadPage(doc, i);
        FPDF_TEXTPAGE text_page = FPDFText_LoadPage(page);

        // Same API as Rust tool
        int char_count = FPDFText_CountChars(text_page);
        for (int j = 0; j < char_count; j++) {
            unsigned int code = FPDFText_GetUnicode(text_page, j);
            // Handle surrogates (same logic as Rust)
            // Write UTF-32 LE
        }

        FPDFText_ClosePage(text_page);
        FPDF_ClosePage(page);

        // Write page separator BOM
        fwrite("\xFF\xFE\x00\x00", 4, 1, stdout);
    }

    FPDF_CloseDocument(doc);
    FPDF_DestroyLibrary();
    return 0;
}
```

**File 2**: `examples/reference_jsonl_extract.cpp`
- Same structure, but outputs JSONL
- Calls all 13 FPDFText_* APIs
- Validates our Rust JSONL tool is correct

**Build**:
```bash
cd /Users/ayates/pdfium
# Add to BUILD.gn
gn gen out/Reference
ninja -C out/Reference reference_text_extract reference_jsonl_extract
```

### Step 2: Run Validation Tests (30 minutes)

**Test corpus**: 10 representative PDFs
- 2 arxiv (academic)
- 2 cc (web)
- 2 edinet (Japanese)
- 2 pages (page-numbered)
- 2 edge_cases

**Script**: `integration_tests/lib/validate_against_upstream.py`

```python
def validate_text_extraction(pdf_path):
    """Compare C++ reference vs Rust tool"""

    # Generate C++ output
    cpp_output = subprocess.check_output([
        'out/Reference/reference_text_extract',
        pdf_path
    ])

    # Generate Rust output (single-threaded)
    rust_output = subprocess.check_output([
        'rust/target/release/examples/extract_text',
        pdf_path, '/tmp/rust.txt', '1'
    ])
    rust_data = open('/tmp/rust.txt', 'rb').read()

    # Compare byte-for-byte
    if cpp_output == rust_data:
        return True, "MATCH"
    else:
        return False, f"DIFFER: {len(cpp_output)} vs {len(rust_data)} bytes"

# Run for all 10 PDFs
results = []
for pdf in TEST_PDFS:
    match, msg = validate_text_extraction(pdf)
    results.append((pdf.name, match, msg))
    print(f"{pdf.name}: {msg}")

# Report
if all(r[1] for r in results):
    print("\n✅ ALL PASSED: Rust tools match C++ reference")
else:
    print("\n❌ FAILURES: Rust tools differ from C++ reference")
    for name, match, msg in results:
        if not match:
            print(f"  - {name}: {msg}")
```

### Step 3: Document Results (15 minutes)

**File**: `integration_tests/UPSTREAM_VALIDATION_RESULTS.md`

```markdown
# Upstream Validation Results

**Date**: 2025-11-XX
**Upstream**: Git 7f43fd79, libpdfium.dylib MD5 00cd20f999bf

## Test Matrix

| PDF | Text (C++ vs Rust) | JSONL (C++ vs Rust) | Image (MD5) |
|-----|-------------------|---------------------|-------------|
| arxiv_004.pdf | ✅ MATCH | ✅ MATCH | ✅ MATCH |
| arxiv_010.pdf | ✅ MATCH | ✅ MATCH | ✅ MATCH |
| ... | | | |

## Summary

**Text extraction**: 10/10 PDFs match byte-for-byte
**JSONL extraction**: 10/10 PDFs match byte-for-byte
**Image rendering**: 10/10 PDFs match MD5

**Conclusion**: Rust tools correctly call upstream PDFium APIs. Output is identical to C++ reference implementation.
```

### Step 4: Update Test Suite Claims (5 minutes)

**Update**: `CRITICAL_TESTING_GAPS.md`

Change from:
```markdown
❌ **Correctness vs upstream**: Baselines are from buggy Rust tools, not PDFium
```

To:
```markdown
✅ **Correctness vs upstream**: Validated against C++ reference tools
   - 10 PDFs tested: 100% match (text, JSONL, images)
   - See UPSTREAM_VALIDATION_RESULTS.md
```

**New grade**: B+ → A- (with upstream validation)

---

## Timeline

| Phase | Task | Time | Output |
|-------|------|------|--------|
| 1 | Create C++ reference tools | 1 hour | reference_text_extract.cpp |
| 2 | Run validation tests | 30 min | validate_against_upstream.py |
| 3 | Document results | 15 min | UPSTREAM_VALIDATION_RESULTS.md |
| 4 | Update claims | 5 min | CRITICAL_TESTING_GAPS.md update |
| **Total** | | **2 hours** | Validated test suite |

---

## Success Criteria

After completing this plan:

**✅ We can claim**:
1. Rust tools validated against upstream PDFium
2. Single-threaded output matches C++ reference (byte-for-byte)
3. Multi-threaded output matches single-threaded (deterministic)
4. Therefore: Multi-threaded output matches upstream (transitive)

**Testing grade upgrade**:
- Before: B- (circular self-validation)
- After: A- (validated against upstream reference)

**Remaining to A+**:
- Visual regression with SSIM (catch rendering quality issues)
- Extended edge case coverage (all PDF features)

---

## Next Steps

**For MANAGER** (or next WORKER if you want to delegate):

1. Create `examples/reference_text_extract.cpp`
2. Create `examples/reference_jsonl_extract.cpp`
3. Add to BUILD.gn, compile
4. Create `integration_tests/lib/validate_against_upstream.py`
5. Run validation on 10 PDFs
6. Document results
7. Update CRITICAL_TESTING_GAPS.md

**Estimated**: 2 hours work
**Value**: Upgrades testing credibility from B- to A-

**Then**: Continue with Phase 2-3 (finish expected outputs + test generation)
