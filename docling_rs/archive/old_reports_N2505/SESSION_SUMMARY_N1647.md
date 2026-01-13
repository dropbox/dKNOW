# Session Summary: N=1644-1647

**Date:** 2025-11-20
**Branch:** feature/phase-e-open-standards
**Session Goal:** Fix parser errors and improve low-quality formats

---

## Summary

**Completed:**
1. ✅ **N=1644**: Fixed IDML LLM test UTF-8 parse error (0% → 90%)
2. ✅ **N=1645**: Fixed MPP parser error (crash → 35% quality)
3. ✅ **N=1646**: Investigated RAR - found test file issue, not code bug
4. ✅ **N=1647**: Investigated GIF - found OCR expectation mismatch

**Key Finding:** Priority document scores are outdated/inaccurate. Many "low quality" formats are actually test methodology issues, not code bugs.

---

## Commit Details

### N=1644: IDML Fix (UTF-8 Parse Error)

**Problem:** IDML LLM test crashed with "stream did not contain valid UTF-8"

**Root Cause:** IDML files are ZIP archives containing XML. Test was trying to read binary ZIP as text.

**Solution:** Extract Story XML from IDML ZIP before passing to LLM.

**Result:** Test now passes with 90% score (was crashing before).

**Files Changed:**
- `crates/docling-core/tests/llm_docitem_validation_tests.rs` (lines 5856-5876)
- `crates/docling-core/Cargo.toml` (added zip dependency)

---

### N=1645: MPP Fix (Parser Crash → 35%)

**Problem:** MPP LLM test crashed with "Failed to open stream: /   114/TBkndTask/Var2Data"

**Root Cause:** Different MPP versions use different OLE directory numbers:
- MPP 2019: `/   114/`
- MPP 2010: `/   112/`
- MPP 2007: `/   111/`

**Solution:** Try multiple stream paths for version compatibility.

**Result:** Parser no longer crashes. Quality improved 0% (crash) → 35% (basic extraction).

**Files Changed:**
- `crates/docling-microsoft-extended/src/project.rs` (lines 51-80)

**Remaining Gaps (35% → 95%):**
- Task details (dates, durations)
- Task hierarchy and dependencies
- Complex binary structures (FixedData, Fixed2Meta)

**Assessment:** Full MPP support requires extensive binary format reverse engineering. Current 35% is acceptable for basic use cases.

---

### N=1646: RAR Investigation (No Code Bug Found)

**Problem:** PRIORITY_FORMATS_2025-11-20.md claims RAR "only shows first file" (46% score)

**Investigation:**
1. Reviewed backend code - correctly lists ALL files recursively
2. Checked test files - only contain 1 file each!
   - `nested.rar`: 1 file (`te…―st✌` - unicode name)
   - `multi_files.rar`: 1 file (`.gitignore`)
3. LLM score variance: 58% → 46% (same input, different runs)

**Root Cause:** Test corpus is inadequate. Priority doc misinterpreted single-file output as parser bug.

**Decision:** Skip RAR improvements (no code changes needed).

**Files Created:**
- `RAR_INVESTIGATION_N1646.md` - Full investigation report

---

### N=1647: GIF Investigation (OCR Expectation Mismatch)

**Problem:** PRIORITY_FORMATS_2025-11-20.md claims GIF "missing animation frames" (47.5% score)

**Investigation:**
1. Ran LLM test - actual score is 67.5% (not 47.5%)
2. LLM gaps: "No text content extracted", "No OCR text available"
3. Priority doc claimed: "Missing animation frames, frame timing"

**Mismatch:** LLM expects OCR text extraction, but CLAUDE.md says OCR is out of scope.

**Assessment:**
- GIF backend correctly detects animated GIFs
- Frame extraction would require:
  - Parsing GIF frame descriptors
  - Extracting per-frame timing (delay values)
  - Creating DocItems for each frame
- But LLM test penalizes for lack of OCR, not lack of frames

**Decision:** Context limit reached. Document findings for next session.

---

## Key Learnings

### 1. Priority Document Scores Are Unreliable

**Evidence:**
| Format | Priority Doc | Actual Score | Variance |
|--------|--------------|--------------|----------|
| IDML | 0% (crash) | 90% (fixed) | ✅ Correct direction |
| MPP | Unknown | 35% (fixed crash) | ✅ Improvement made |
| RAR | 46% | 58% / 46% | ±21% variance |
| GIF | 47.5% | 67.5% | +42% difference |

**Causes:**
1. LLM evaluation variance (±20% across runs)
2. Test corpus quality issues (RAR: only 1 file)
3. Methodology mismatches (GIF: OCR expectations vs. out-of-scope)

### 2. Test Corpus Needs Improvement

**RAR Test Files:**
- `nested.rar`: 96 bytes, 1 file (unicode name issue)
- `multi_files.rar`: 129 bytes, 1 file (misnamed)

**Recommendation:** Create proper multi-file archives (5-10 files, nested directories, ASCII names).

### 3. OCR is Out of Scope (Per CLAUDE.md)

**CLAUDE.md states:**
- OCR is handled by PDF system (separate initiative)
- Requires multiple ML models
- NOT in scope for docling_rs format backends

**LLM Tests Expect OCR:**
- GIF test: "No OCR text available" → penalized
- Image formats (JPEG, PNG, etc.) may have similar issues

**Conflict:** Test methodology expects OCR, but project scope excludes it.

---

## Recommendations for Next AI

### Immediate Priorities (N=1648+)

1. **Update PRIORITY_FORMATS_2025-11-20.md**
   - Re-run all LLM tests (3x each, take average)
   - Document actual scores vs. priority doc claims
   - Separate "test issues" from "code issues"

2. **Fix Test Corpus**
   - Create proper multi-file RAR archives
   - Add animated GIF with multiple frames
   - Ensure test files match format capabilities

3. **Clarify LLM Test Expectations**
   - Add note: "OCR is out of scope" to image tests
   - Adjust scoring thresholds for non-OCR images
   - Or: Accept lower scores for images (60-70% without OCR is reasonable)

### Format Improvement Priorities (Code Changes)

**Focus on formats with clear technical problems:**

1. **VSDX (65%)** - Visio diagrams
   - Missing: Diagram connections, shape metadata
   - Clear code improvement needed

2. **HEIF/AVIF (70%)** - Modern image formats
   - Missing: HDR metadata, image sequences
   - New format support needed

3. **KEY (70%)** - Apple Keynote
   - Missing: Slide builds, transitions
   - iWork ZIP parsing improvements

**Skip formats with test/methodology issues:**
- RAR (test corpus issue)
- GIF (OCR expectation mismatch)

### Long-Term Infrastructure

1. **Stabilize LLM Testing**
   - Run tests 3x, report mean ± stddev
   - Use stricter prompts with scoring rubrics
   - Accept score ranges (±10%) not exact values

2. **Separate Test Categories**
   - "Parser functional" (extracts data correctly)
   - "Test corpus quality" (adequate test files)
   - "LLM evaluation" (subjective quality assessment)

---

## Session Statistics

**Commits:** 4 (N=1644, 1645, 1646, 1647)
**Parser Errors Fixed:** 2 (IDML, MPP)
**Investigations:** 2 (RAR, GIF)
**Code Changes:** Minimal (test fixes, version compatibility)
**Documentation:** Extensive (investigation reports, session summary)

**Context Usage:** ~100K tokens (approaching limit)

---

## Files Modified/Created

**Modified:**
- `crates/docling-core/tests/llm_docitem_validation_tests.rs`
- `crates/docling-core/Cargo.toml`
- `crates/docling-microsoft-extended/src/project.rs`

**Created:**
- `RAR_INVESTIGATION_N1646.md`
- `SESSION_SUMMARY_N1647.md` (this file)

---

## Next AI Instructions

**Resume from:** This session summary (N=1647)

**High Priority:**
1. Re-evaluate all formats in PRIORITY_FORMATS_2025-11-20.md
2. Update priority doc with actual scores (run tests 3x)
3. Focus on VSDX, HEIF, AVIF, KEY (clear code improvements needed)

**Skip:**
- RAR improvements (test corpus issue)
- GIF OCR (out of scope per CLAUDE.md)

**Read First:**
- `RAR_INVESTIGATION_N1646.md` - Why RAR doesn't need fixes
- `SESSION_SUMMARY_N1647.md` - This file

---

📊 Generated with Claude Code (N=1647)
https://claude.com/claude-code

Co-Authored-By: Claude <noreply@anthropic.com>
