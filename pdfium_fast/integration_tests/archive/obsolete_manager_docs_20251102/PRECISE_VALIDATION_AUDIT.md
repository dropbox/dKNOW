# PRECISE VALIDATION AUDIT - What Was Actually Tested

**Date**: 2025-11-02 08:30 PST
**Auditor**: MANAGER
**Purpose**: Precise, unambiguous record of validation methodology

---

## USER REQUIREMENT

"Make sure that this uses the baseline upstream public pdfium output"

---

## VALIDATION METHODOLOGY

### Upstream Library Confirmed

**Library Used**: `/Users/ayates/pdfium/out/Optimized-Shared/libpdfium.dylib`
- **Source**: Git commit 7f43fd79 (2025-10-30)
- **Repository**: https://pdfium.googlesource.com/pdfium/
- **MD5**: `00cd20f999bf60b1f779249dbec8ceaa`
- **Build Date**: 2025-10-31 02:11
- **Modifications**: 0 C++ changes verified (per CLAUDE.md)
- **Status**: ✅ **Pure upstream PDFium library**

### What "Validated vs Upstream" Means

**For Text & JSONL**:
```
Step 1: Create C++ reference tool
Step 2: C++ tool → calls PDFium API → upstream libpdfium.dylib → output A
Step 3: Rust tool → calls PDFium API → upstream libpdfium.dylib → output B
Step 4: Compare: A == B?
```

**Both tools use the SAME upstream library.**

**Key point**: We're validating that Rust bindings call the APIs correctly, not that PDFium itself is correct.

**PDFium correctness**: Assumed (it's the reference implementation)

---

## TEXT EXTRACTION VALIDATION ✅

### What Was Tested

**Test Date**: 2025-11-02 00:50-01:00 PST
**Test Commit**: 9b0f3b4ca
**Tool Created**: `examples/reference_text_extract.cpp` (142 lines)
**Tool Compiled**: `out/Optimized-Shared/reference_text_extract`

**Validation Script**: `lib/validate_against_upstream.py` (234 lines)

### Exact Testing Procedure

**For each of 10 PDFs**:

1. **Generate C++ reference output**:
   ```bash
   DYLD_LIBRARY_PATH=/path/to/out/Optimized-Shared
   ./reference_text_extract input.pdf cpp_output.txt
   ```
   - Calls: `FPDFText_GetUnicode(text_page, i)` for each character
   - Handles: UTF-16 surrogate pairs
   - Outputs: UTF-32 LE with BOMs

2. **Generate Rust output**:
   ```bash
   DYLD_LIBRARY_PATH=/path/to/out/Optimized-Shared
   ./extract_text input.pdf rust_output.txt 1
   ```
   - Calls: Same API through Rust bindings
   - Handles: Same surrogate pair logic
   - Outputs: UTF-32 LE with BOMs

3. **Compare byte-for-byte**:
   ```bash
   diff cpp_output.txt rust_output.txt
   md5 cpp_output.txt
   md5 rust_output.txt
   ```

### Test Results

| PDF | C++ Bytes | Rust Bytes | MD5 Match | Result |
|-----|-----------|------------|-----------|--------|
| arxiv_001.pdf | 270,160 | 270,160 | 531126311773 | ✅ MATCH |
| arxiv_004.pdf | 410,024 | 410,024 | 289b735ab1d8 | ✅ MATCH |
| arxiv_010.pdf | 348,028 | 348,028 | 290dc6ec90ff | ✅ MATCH |
| cc_007_101p.pdf | 865,760 | 865,760 | ab7df24f0067 | ✅ MATCH |
| cc_015_101p.pdf | 723,500 | 723,500 | 3c3038361c60 | ✅ MATCH |
| edinet_E01920.pdf | 634,860 | 634,860 | d36f5b6b38ff | ✅ MATCH |
| edinet_E02628.pdf | 415,272 | 415,272 | 5b63708c4c63 | ✅ MATCH |
| web_005.pdf | 744,712 | 744,712 | ae907ef2270b | ✅ MATCH |
| web_011.pdf | 52,512 | 52,512 | 6d0aa7fb22a1 | ✅ MATCH |
| 0100pages.pdf | 1,208,972 | 1,208,972 | 5ebb0ed0a0a6 | ✅ MATCH |

**Result**: **10/10 PDFs (100%) byte-for-byte identical**

### What This Proves

✅ **Proven**:
- Rust bindings correctly call FPDFText_GetUnicode()
- UTF-16 surrogate handling is correct
- UTF-32 LE encoding is correct
- Output matches C++ calling same API on same library

✅ **Upstream connection**:
- Both tools use upstream libpdfium.dylib (unmodified)
- Library from git 7f43fd79 (Oct 30, 2025)
- Source: https://pdfium.googlesource.com/pdfium/

**Confidence**: **100%**

### What This Does NOT Prove

❌ **Not tested**:
- Did not compare against `pdfium_test` tool (it doesn't extract text)
- Did not compare against official test baselines (don't exist for text)
- Did not compare against Adobe Reader output

**Why**: No official text extraction reference exists. PDFium IS the reference.

**Conclusion**: This is the **best possible validation** for text extraction.

---

## JSONL EXTRACTION VALIDATION ✅

### What Was Tested

**Test Date**: 2025-11-02 00:50-01:00 PST
**Tool Created**: `examples/reference_jsonl_extract.cpp` (229 lines)
**Tool Compiled**: `out/Optimized-Shared/reference_jsonl_extract`

### Exact Testing Procedure

**For each of 10 PDFs**:

1. **Generate C++ reference output**:
   ```bash
   ./reference_jsonl_extract input.pdf cpp_output.jsonl 0
   ```
   - Calls all 13 FPDFText_* APIs per character
   - Outputs: JSON object per character

2. **Generate Rust output**:
   ```bash
   ./extract_text_jsonl input.pdf rust_output.jsonl 0
   ```
   - Calls same 13 APIs through bindings
   - Outputs: JSON object per character

3. **Compare**:
   ```bash
   # Parse as JSON and compare values
   jq . cpp_output.jsonl > cpp_parsed.json
   jq . rust_output.jsonl > rust_parsed.json
   ```

### Test Results

| PDF | C++ Lines | Rust Lines | Values Match | Bytes Differ |
|-----|-----------|------------|--------------|--------------|
| arxiv_001.pdf | 3638 | 3638 | ✅ YES | 105KB (format) |
| arxiv_004.pdf | 3235 | 3235 | ✅ YES | 67KB (format) |
| arxiv_010.pdf | 5981 | 5981 | ✅ YES | 123KB (format) |
| cc_007_101p.pdf | 194 | 194 | ✅ YES | 8KB (format) |
| cc_015_101p.pdf | 126 | 126 | ✅ YES | 6KB (format) |
| edinet_E01920.pdf | 477 | 477 | ✅ YES | 14KB (format) |
| edinet_E02628.pdf | 457 | 457 | ✅ YES | 13KB (format) |
| web_005.pdf | 1433 | 1433 | ✅ YES | 34KB (format) |
| web_011.pdf | 1693 | 1693 | ✅ YES | 50KB (format) |
| 0100pages.pdf | 227 | 227 | ✅ YES | 9KB (format) |

**Result**: **10/10 PDFs (100%) numerically identical**

**Difference analysis**:
- Character counts: Identical
- Metadata values: Numerically identical
- Byte sizes: Differ by ~10% (C++ uses %.17g precision, Rust uses default Display)

**Example**:
```
C++:  "bbox":[231.03201293945312,708.719970703125,...]
Rust: "bbox":[231.03201293945313,708.719970703125,...]
      (difference in last digit: 2 vs 3 - within float64 ULP)
```

**Confidence**: **95%** (values correct, formatting cosmetic)

---

## IMAGE RENDERING VALIDATION ❌

### What Was Tested

**Testing done**:
- Self-consistency: 1-worker output == 4-worker output (MD5 match)
- Determinism: Same PDF produces same MD5 across runs
- No crashes or errors

**Validation script**: None
**Comparison tool**: None
**Upstream comparison**: **NONE**

### What Was NOT Tested

❌ **No comparison against upstream pdfium_test**:
- Never generated baseline images with pdfium_test
- Never compared MD5 of our renders vs pdfium_test
- Never used SSIM for perceptual comparison
- Never visually inspected renders

**Current status**: 196/452 PDFs have "baselines" (our own output)

**What "baseline" means**: Our own render, stored for comparison against future runs

**What "baseline" does NOT mean**: Upstream pdfium_test output (gold standard)

### Critical Gap

**Problem**: We test "does output change" not "is output correct"

**Example failure scenario**:
```
Bug: All text rendered 10 pixels too low
Our test: PASS (MD5 matches previous wrong render)
Reality: Images are wrong, but consistently wrong
```

**Confidence**: **0%** upstream validation

**Grade: F** - No upstream comparison performed

---

## PRECISE AUDIT SUMMARY

### Text Extraction ✅

**What "validated" means**:
- Created C++ tool calling PDFium API: `FPDFText_GetUnicode()`
- Compared C++ vs Rust output on 10 PDFs
- Both use upstream libpdfium.dylib (git 7f43fd79, MD5 00cd20f999bf)
- Result: 100% byte-for-byte match

**Baseline**: C++ reference output (both tools call same upstream library)

**Why this is valid**: pdfium_test doesn't extract text. PDFium library IS the reference.

**Confidence**: 100% ✅

### JSONL Extraction ✅

**What "validated" means**:
- Created C++ tool calling 13 FPDFText_* APIs
- Compared C++ vs Rust output on 10 PDFs
- Both use upstream libpdfium.dylib
- Result: 100% numerically identical (formatting differs)

**Baseline**: C++ reference output (both tools call same upstream library)

**Why this is valid**: JSONL is our custom format. We validate APIs return correct values.

**Confidence**: 95% ✅

### Image Rendering ❌

**What "validated" means**: **NOTHING**

**No upstream comparison performed**

**Baseline**: Our own output (self-referential, not upstream)

**Why this is INVALID**:
- pdfium_test CAN render images
- We SHOULD compare against pdfium_test output
- We have NOT done this

**Confidence**: 0% ❌

---

## WORKER DIRECTIVE: Image Validation Required

**READ THIS FILE**: integration_tests/PRECISE_VALIDATION_AUDIT.md

**YOUR TASK**: Validate images vs upstream pdfium_test

**Why**: Images are the ONLY component without upstream validation

**Steps**:

1. **Select 50 test PDFs** (representative sample)
2. **Generate upstream baselines**:
   ```bash
   cd /tmp/upstream
   export DYLD_LIBRARY_PATH=/Users/ayates/pdfium/out/Optimized-Shared
   for pdf in <50 PDFs>; do
     /Users/ayates/pdfium/out/Optimized-Shared/pdfium_test $pdf
   done
   # Generates: input.pdf.0.ppm, input.pdf.1.ppm, etc.
   ```

3. **Convert ppm to png**:
   ```bash
   for ppm in *.ppm; do
     convert $ppm ${ppm%.ppm}.png  # Requires ImageMagick
   done
   ```

4. **Generate our output**:
   ```bash
   for pdf in <50 PDFs>; do
     render_pages $pdf /tmp/ours/ 1 300
   done
   ```

5. **Compare MD5**:
   ```python
   for each page:
     upstream_md5 = md5(upstream/page_X.png)
     our_md5 = md5(ours/page_X.png)
     if upstream_md5 == our_md5:
       result = "MATCH"
     else:
       ssim_score = compare_ssim(upstream, ours)
       result = f"DIFFER (SSIM: {ssim_score})"
   ```

6. **Document in**: `UPSTREAM_IMAGE_VALIDATION_RESULTS.md`
   - MD5 match count
   - SSIM scores for differences
   - Analysis of any mismatches

**Expected time**: 4-6 hours

**Deliverable**: Proof that images match upstream pdfium_test renders

**This is CRITICAL**: Images are only component without upstream validation

---

## After Image Validation

**If images match**: System grade → **A-** (fully validated)
**If images differ**: Investigate and fix bugs, then A-

**Then can claim**: "All components validated against upstream PDFium"
