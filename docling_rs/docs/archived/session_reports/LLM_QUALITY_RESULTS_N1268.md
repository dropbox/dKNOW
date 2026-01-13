# LLM Quality Verification Results - N=1268

**Session:** N=1268
**Date:** 2025-11-17
**Branch:** feature/phase-e-open-standards
**Previous Session:** N=1267 (Fixed HTML heading levels and list start attributes)

---

## Executive Summary

**Test Results:** 8/9 formats passing at ≥95% (89% pass rate)
**HTML Improvement:** ✅ 85-87% → 100% (N=1267 fixes successful!)
**New Issue:** ⚠️  PPTX regression: 98% (N=1257) → 94% (below threshold)

---

## Test Results by Format

### Perfect 100% (6 formats) ✅

1. **CSV: 100%**
   - Completeness: 100/100
   - Accuracy: 100/100
   - Structure: 100/100
   - Formatting: 100/100
   - Metadata: 100/100
   - Status: PERFECT

2. **DOCX: 100%**
   - Completeness: 100/100
   - Accuracy: 100/100
   - Structure: 100/100
   - Formatting: 100/100
   - Metadata: 100/100
   - Status: PERFECT

3. **XLSX: 100%**
   - Completeness: 100/100
   - Accuracy: 100/100
   - Structure: 100/100
   - Formatting: 100/100
   - Metadata: 100/100
   - Status: PERFECT (improved from 95% at N=1257!)

4. **HTML: 100%** ✅ MAJOR IMPROVEMENT
   - Completeness: 100/100
   - Accuracy: 100/100
   - Structure: 100/100
   - Formatting: 100/100
   - Metadata: 100/100
   - Status: PERFECT
   - **Previous Score:** 85-87% (N=1266 quality report)
   - **Improvement:** +13-15 points
   - **Fix:** N=1267 heading level and list marker fixes

5. **WebVTT: 100%**
   - Completeness: 100/100
   - Accuracy: 100/100
   - Structure: 100/100
   - Formatting: 100/100
   - Metadata: 100/100
   - Status: PERFECT

### Excellent 95-99% (3 formats) ✅

6. **AsciiDoc: 98%**
   - Completeness: 100/100
   - Accuracy: 100/100
   - Structure: 100/100
   - Formatting: 95/100 (minor table spacing)
   - Metadata: 100/100
   - Status: EXCELLENT

7. **Markdown: 97%**
   - Completeness: 100/100
   - Accuracy: 95/100 (table header missing "Calories")
   - Structure: 100/100
   - Formatting: 95/100 (table alignment)
   - Metadata: 100/100
   - Status: EXCELLENT

8. **JATS: 95%** (XML academic format)
   - Completeness: 100/100
   - Accuracy: 95/100 (citation spacing)
   - Structure: 100/100
   - Formatting: 95/100 (citation punctuation)
   - Metadata: 100/100
   - Status: PASSES THRESHOLD
   - Issues: Minor citation spacing/punctuation inconsistencies

### Below Threshold (1 format) ⚠️

9. **PPTX: 94%** ⚠️ REGRESSION
   - Completeness: 100/100
   - Accuracy: 95/100
   - Structure: 100/100
   - Formatting: 90/100 (list formatting issues)
   - Metadata: 100/100
   - **Previous Score:** 98% (N=1257)
   - **Change:** -4 points (regression)
   - **Issue:** "List items on second slide not consistently formatted (missing bullet points)"
   - **Likely Cause:** N=1267 list marker changes affected PPTX output
   - **Verification:** Ran test twice, consistently 94% (not LLM variance)

---

## Analysis

### N=1267 Impact Assessment

**Positive Impact:**
- ✅ HTML: 85-87% → 100% (+13-15 points) - EXCELLENT
- ✅ Fixed heading level bug (h1=#, h2=##, h3=### now correct)
- ✅ Fixed ordered list start attributes (42, 43 now preserved correctly)

**Negative Impact:**
- ⚠️  PPTX: 98% → 94% (-4 points) - REGRESSION
- Issue: List formatting on second slide (missing bullet points)
- Hypothesis: markdown_helper.rs changes affected PPTX list serialization

**Net Result:**
- 8/9 formats ≥95% (89% pass rate)
- 6/9 formats at 100% (67% perfect rate)
- 1 regression that needs investigation

### Root Cause Analysis - PPTX Regression

**N=1267 Changes:**
1. Heading level: Removed +1 offset (fixed HTML, may affect PPTX)
2. List markers: Now using `marker` field from DocItem instead of regenerating

**PPTX Issue:**
- "Missing bullet points in some instances" on second slide
- Likely: List marker field is None or empty for some PPTX list items
- Need to check: How PPTX backend populates `marker` field for lists

**Investigation Needed:**
1. Check PPTX test file (which file has "second slide"?)
2. Examine PPTX backend list parsing in `pptx.rs`
3. Compare DocItem generation: bullet vs numbered lists
4. Test if `marker` field is properly set for all PPTX list items

---

## Quality Status Summary

### Python-Compatible Formats (9 baseline-verified)

**Perfect 100% (5 formats):**
- CSV: 100%
- DOCX: 100%
- XLSX: 100%
- HTML: 100% (improved from 85-87%)
- WebVTT: 100%

**Excellent 95-99% (3 formats):**
- JATS: 95%
- AsciiDoc: 98%
- Markdown: 97%

**Good 85-94% (1 format):**
- PPTX: 94% (regression from 98%, needs investigation)

**Overall Baseline Coverage:** 8/9 passing (89%)

---

## Next Steps

### Immediate (N=1269)

1. **Investigate PPTX regression:**
   - Identify test file with "second slide" issue
   - Check PPTX list marker generation in pptx.rs
   - Compare expected vs actual output for that slide
   - Fix marker field population for PPTX lists

2. **Verify fix:**
   - Re-run PPTX LLM test after fix
   - Target: Restore to 98%+

### Optional Improvements

1. **Markdown (97%):**
   - Fix table header: "Calories per portion" vs "per portion"
   - Improve table alignment consistency

2. **AsciiDoc (98%):**
   - Fix table spacing (minor issue)

3. **JATS (95%):**
   - Fix citation spacing around semicolons/parentheses
   - Already passing, but could be improved to 98%+

---

## Test Execution Details

**Command:**
```bash
export OPENAI_API_KEY="sk-proj-..."
cargo test test_llm_verification --test llm_verification_tests -- --ignored --nocapture
```

**Execution Time:** ~6.5 seconds (all 9 formats)
**Cost:** ~$0.09 (9 tests × ~$0.01 each)
**Pass Rate:** 8/9 (89%)

**Test Infrastructure:**
- Location: `crates/docling-core/tests/llm_verification_tests.rs`
- Model: OpenAI GPT-4o
- Threshold: 95% overall score to pass

---

## Lessons Learned

### Lesson 1: Cross-Format Impact of Markdown Helper Changes

The N=1267 fix to `markdown_helper.rs` affected multiple formats:
- ✅ HTML: Improved (+13-15 points)
- ⚠️  PPTX: Regressed (-4 points)
- ✅ DOCX: Maintained 100%
- ✅ XLSX: Improved to 100%

**Takeaway:** Changes to shared serialization code (markdown_helper) affect all formats using it. Always test all baseline formats after such changes.

### Lesson 2: LLM Variance vs Real Regressions

PPTX scored 94% twice in a row, not LLM variance:
- First run: 94% ("missing bullets for ordered lists")
- Second run: 94% ("missing bullet points in some instances")
- Consistent score and consistent issue description

**Takeaway:** When LLM gives same score and similar reasoning twice, it's a real issue, not variance.

### Lesson 3: List Marker Field Is Critical

The `marker` field in DocItem must be populated correctly for all list items:
- Ordered lists: "1.", "2.", "42.", "43.", etc.
- Unordered lists: "•" or "-" or "*"
- Empty/None: Results in missing bullets in output

**Takeaway:** Every backend must set `marker` field for ListItem DocItems. Missing marker = broken list rendering.

---

## Files Changed Since N=1267

**None** - This is a verification-only session.

---

## References

- N=1267: HTML regression fix (heading levels, list markers)
- N=1257: Quality targets achieved (CSV/DOCX/XLSX/PPTX all ≥95%)
- N=1252: Quality verification with actual LLM tests (introduced variance analysis)

---

**Conclusion:** N=1267 fixes were largely successful (HTML +15 points), but introduced a PPTX regression (-4 points). PPTX list marker fix needed to restore to 98%+. Overall system health: GOOD (8/9 passing, 1 issue to fix).
