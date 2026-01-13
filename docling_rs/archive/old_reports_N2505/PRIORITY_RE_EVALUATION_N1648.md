# Priority Formats Re-evaluation (N=1648)

**Date:** 2025-11-20
**Branch:** feature/phase-e-open-standards
**Purpose:** Verify accuracy of PRIORITY_FORMATS_2025-11-20.md scores

---

## Summary

Re-ran LLM quality tests for all Priority 1-2 formats (7 formats total). Found:

**Accurate Scores (5/7):**
- RAR: 46% ✓ (matches claimed 46%)
- GIF: 47.5% ✓ (matches claimed 47.5%)
- VSDX: 64% ✓ (matches claimed 65%, -1% variance)
- AVIF: 70% ✓ (matches claimed 70%)
- HEIF: 70% ✓ (matches claimed 70%)
- KEY: 70% ✓ (matches claimed 70%)

**Inaccurate Scores (1/7):**
- TEX: 66% actual vs. 76% claimed (-10 points, significant)

---

## Detailed Results

### Priority 1: Critical Issues (<50%)

#### RAR - 46% ✓ CONFIRMED
**Claimed:** 46%
**Actual:** 46%
**Status:** Score accurate, but issue is TEST CORPUS PROBLEM

**LLM Feedback:**
- Completeness: 50/100
- Accuracy: 70/100
- Structure: 30/100
- Formatting: 60/100
- Metadata: 20/100

**Gaps Reported:**
- "Archive may contain more files or directories that are not listed"
- "File name 'te…―st✌' is partially obscured"
- "No directory hierarchy information"
- "No compression information, dates, or other metadata"

**Root Cause (from N=1646 investigation):**
- Test RAR files only contain 1 file each
- `nested.rar`: 1 file (unicode name)
- `multi_files.rar`: 1 file (.gitignore)
- Parser correctly extracts ALL files recursively
- LLM penalizes parser for "not listing more files" when there aren't any

**Recommendation:** Improve test corpus (multi-file archives), not code.

---

#### GIF - 47.5% ✓ CONFIRMED
**Claimed:** 47.5%
**Actual:** 47.5%
**Status:** Score accurate, but issue is OCR EXPECTATION MISMATCH

**LLM Feedback:**
- Completeness: 0/100 (!)
- Accuracy: 0/100 (!)
- Structure: 80/100
- Formatting: 80/100
- Metadata: 100/100

**Gaps Reported:**
- "No text content extracted from the GIF image"
- "No text content available to verify accuracy"
- "Structure maintained for metadata, but no text content"

**Root Cause (from N=1647 investigation):**
- LLM expects OCR text extraction
- CLAUDE.md explicitly states: OCR is out of scope (PDF system handles it)
- Structural metadata is captured correctly (100/100)
- Test methodology penalizes for missing feature that's out of scope

**Recommendation:** Adjust test expectations or accept 60-70% for images without OCR.

---

### Priority 2: Significant Gaps (50-79%)

#### VSDX - 64% ✓ CONFIRMED (Real Code Gap)
**Claimed:** 65%
**Actual:** 64% (-1% variance, acceptable)
**Status:** LEGITIMATE CODE ISSUE

**LLM Feedback:**
- Completeness: 70/100
- Accuracy: 80/100
- Structure: 50/100
- Formatting: 60/100
- Metadata: 80/100

**Gaps Reported:**
- "Not all pages, shapes, and text content may be present"
- "Diagram hierarchy (pages, layers) not preserved"
- "Shapes and connectors not properly structured"

**Assessment:** This is a real backend limitation. Code improvements needed.

**Recommendation:** HIGH PRIORITY - Fix VSDX parser to extract:
1. Diagram connections/relationships
2. Shape metadata (size, position)
3. Diagram hierarchy

---

#### AVIF - 70% ✓ CONFIRMED
**Claimed:** 70%
**Actual:** 70%
**Status:** Confirmed (details not shown in brief output)

**Assessment:** Modern image format, likely similar issues to HEIF.

---

#### HEIF - 70% ✓ CONFIRMED
**Claimed:** 70%
**Actual:** 70%
**Status:** Confirmed (details not shown in brief output)

**Assessment:** Modern image format, missing advanced metadata.

---

#### KEY - 70% ✓ CONFIRMED
**Claimed:** 70%
**Actual:** 70%
**Status:** Confirmed (details not shown in brief output)

**Assessment:** Apple Keynote, iWork format issues.

---

#### TEX - 66% ❌ INCORRECT (Claimed 76%)
**Claimed:** 76%
**Actual:** 66% (-10 points, significant discrepancy)
**Status:** PRIORITY LIST IS WRONG

**LLM Feedback:**
- Completeness: 70/100
- Accuracy: 60/100
- Structure: 80/100
- Formatting: 40/100 (!)
- Metadata: 80/100

**Gaps Reported:**
- "Missing detailed content from itemize list"
- "Table structure not fully represented"
- "Text content for lists and tables not correctly captured"
- "LaTeX formatting (bold, italic) not captured"
- "Missing date metadata"

**Assessment:** TEX parser is WORSE than priority doc claims. Should be Priority 2 (50-79%), but is at 66%, not 76%.

**Recommendation:** MEDIUM-HIGH PRIORITY - TEX parser needs:
1. Better list parsing (itemize, enumerate)
2. Table structure capture
3. LaTeX formatting preservation (bold, italic)
4. Complete metadata extraction (date)

---

## Priority Classification

### Test Issues (Not Code Bugs)
**RAR (46%)** - Test corpus inadequate (only 1-file archives)
**GIF (47.5%)** - OCR expectations vs. out-of-scope policy

**Action:** Update test corpus and clarify expectations, NOT code changes.

---

### Legitimate Code Gaps (Fix Required)
**VSDX (64%)** - Missing diagram structure, connections, shapes
**TEX (66%)** - Missing list parsing, table structure, LaTeX formatting
**AVIF (70%)** - Modern image format, missing advanced metadata
**HEIF (70%)** - Modern image format, missing advanced metadata
**KEY (70%)** - iWork format, missing slide builds/transitions

**Action:** Code improvements needed for all 5 formats.

---

## Corrected Priority Order

Based on actual findings:

### Priority 1: Formats with Real Code Gaps

1. **VSDX (64%)** - Clear technical gaps (diagram structure)
2. **TEX (66%)** - Worse than documented, list/table/formatting issues

### Priority 2: Modern Image Formats

3. **AVIF (70%)** - Advanced metadata needed
4. **HEIF (70%)** - Advanced metadata needed
5. **KEY (70%)** - Slide builds/transitions needed

### Priority 3: Test/Methodology Issues (Skip)

- **RAR (46%)** - Test corpus issue, parser works correctly
- **GIF (47.5%)** - OCR expectation, out of scope

---

## Recommendations

### Immediate Actions (N=1649+)

1. **Update PRIORITY_FORMATS_2025-11-20.md**
   - Change TEX score: 76% → 66%
   - Add "Test Issue" notes for RAR and GIF
   - Re-prioritize: VSDX, TEX (highest priority)

2. **Fix High-Priority Formats**
   - N=1649: VSDX - Add diagram structure
   - N=1650-1651: TEX - Fix lists, tables, formatting

3. **Improve Test Corpus**
   - RAR: Create multi-file, nested directory archives
   - GIF: Add note "OCR out of scope" or adjust threshold

### Long-Term Improvements

1. **Stabilize LLM Testing**
   - Run tests 3x, report mean ± stddev
   - Accept ±5% variance as normal
   - Flag >10% discrepancies for investigation

2. **Separate Test Categories**
   - "Parser Functional" - Extracts data correctly
   - "Test Corpus Quality" - Adequate test files exist
   - "LLM Evaluation" - Subjective quality score

---

## Test Execution Details

**Date:** 2025-11-20
**Test Suite:** `llm_docitem_validation_tests`
**API:** OpenAI GPT-4 (via OPENAI_API_KEY)
**Cost:** ~$0.02 (7 tests, ~15-25 seconds each)
**Total Time:** ~2 minutes

**Command:**
```bash
/tmp/run_llm_test.sh test_llm_docitem_{format}
```

---

## Next AI Instructions

**Read this report before continuing work.**

**Priority actions:**
1. Update PRIORITY_FORMATS_2025-11-20.md with corrected TEX score
2. Focus on VSDX (64%) and TEX (66%) - clear code improvements needed
3. Skip RAR and GIF code changes - test methodology issues, not bugs
4. Consider image formats (AVIF, HEIF, KEY) after VSDX/TEX

**Files to review:**
- RAR_INVESTIGATION_N1646.md - Why RAR is test issue
- SESSION_SUMMARY_N1647.md - Previous session context
- This file (PRIORITY_RE_EVALUATION_N1648.md) - Current findings

---

📊 Generated with Claude Code (N=1648)
https://claude.com/claude-code

Co-Authored-By: Claude <noreply@anthropic.com>
