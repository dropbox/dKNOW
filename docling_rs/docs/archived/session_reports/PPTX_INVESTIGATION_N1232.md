# PPTX Investigation - N=1232

**Date:** 2025-11-17
**Issue:** LLM validation test reported PPTX at 76% completeness (below 95% threshold)
**Claim:** "Only one slide and a table present" - multi-slide file expected

---

## Investigation Summary

### ✅ PPTX Multi-Slide Extraction: WORKING CORRECTLY

The PPTX backend correctly extracts all slides from multi-slide presentations.

### ❌ Test Corpus Issue: Wrong File Used

The LLM validation test was using `business_presentation.pptx`, which only has **1 slide**.
The LLM report was **accurate** - there IS only one slide in that file!

---

## Evidence

### Slide Counts (Actual ZIP Contents)

```bash
$ unzip -l test-corpus/pptx/business_presentation.pptx | grep "ppt/slides/slide"
  4337  01-01-1980 00:00   ppt/slides/slide1.xml

$ unzip -l test-corpus/pptx/powerpoint_sample.pptx | grep "ppt/slides/slide"
  4337  01-01-1980 00:00   ppt/slides/slide1.xml
  4337  01-01-1980 00:00   ppt/slides/slide2.xml
  4337  01-01-1980 00:00   ppt/slides/slide3.xml
```

- **business_presentation.pptx:** 1 slide ❌ (old test file)
- **powerpoint_sample.pptx:** 3 slides ✅ (correct test file)

### Backend Test Results

Created `test_multi_slide_extraction()` in `crates/docling-backend/src/pptx.rs` (line 3629):

```
=== business_presentation.pptx (1 slide) ===
Slide count: 1
Chapter DocItems: 1

=== powerpoint_sample.pptx (3 slides) ===
Slide count: 3
Chapter DocItems: 3
Found chapter: slide-0
Found chapter: slide-1
Found chapter: slide-2
```

**Result:** Backend correctly extracts all 3 slides! ✅

### Code Analysis

**File:** `crates/docling-backend/src/pptx.rs`

**Slide Iteration Logic (lines 235-263):**

```rust
// Process each slide
for (slide_idx, slide_path) in slide_refs.iter().enumerate() {
    // Create a slide group for this slide
    let slide_group = DocItem::Chapter {
        self_ref: format!("#/groups/{}", slide_idx),
        parent: None,
        children: vec![],
        content_layer: "body".to_string(),
        name: format!("slide-{}", slide_idx),
    };
    doc_items.push(slide_group);

    // Parse slide XML and extract shapes
    let slide_items = self.parse_slide_xml(archive, slide_path, slide_idx)?;
    doc_items.extend(slide_items);

    // ... (notes processing omitted)
}
```

**Analysis:** The loop correctly iterates through ALL slides in `slide_refs`. No early return, no breaks.

---

## Fix Applied

### Changed LLM Test File

**File:** `crates/docling-core/tests/llm_docitem_validation_tests.rs`

**Changed from:**
```rust
"/../../test-corpus/pptx/business_presentation.pptx"  // 1 slide
```

**Changed to:**
```rust
"/../../test-corpus/pptx/powerpoint_sample.pptx"  // 3 slides
```

**Added comment:**
```rust
// NOTE: Using powerpoint_sample.pptx (3 slides) instead of business_presentation.pptx (1 slide)
// Previous test used single-slide file, causing false negatives about multi-slide extraction
```

---

## Results After Fix

### LLM Validation Score: 87% (+11% improvement)

**Before (business_presentation.pptx, 1 slide):**
- Overall: 76% ❌
- Completeness: 60/100 (critical gap)
- Structure: 70/100

**After (powerpoint_sample.pptx, 3 slides):**
- Overall: 87% ⚠️ (still below 95% threshold)
- Completeness: 85/100 (+25 points)
- Structure: 80/100 (+10 points)
- JSON size: 30,154 chars (vs 2,526 chars previously)

### Remaining Issues (87% → 95% gap)

LLM identified:
1. ❌ **Missing image extraction** - Images in slides not extracted to DocItems
2. ❌ **Missing shapes** - Some shapes (text boxes, other elements) not extracted
3. ⚠️ **List/table formatting inconsistencies** - Minor formatting details lost

**Analysis:** Multi-slide extraction works, but there are missing features (images, shapes).

---

## Root Cause Analysis

### Why the Confusion?

1. **Test used wrong file** - Single-slide file tested with multi-slide expectations
2. **LLM report was accurate** - "Only one slide" was correct for that file!
3. **False negative** - Backend code was correct, but test corpus was wrong

### What the LLM Actually Found

**N=1231 Report (76%, business_presentation.pptx):**
- "Only one slide and a table present" ✅ **ACCURATE** (file has 1 slide + 1 table)
- "Not all slides extracted" ✅ **MISLEADING** (file only has 1 slide!)

**N=1232 Report (87%, powerpoint_sample.pptx):**
- All 3 slides extracted ✅
- Missing images ✅ **REAL ISSUE**
- Missing shapes ✅ **REAL ISSUE**

---

## Conclusion

### ✅ No Code Bug

The PPTX backend multi-slide extraction is **working correctly**. Code review confirmed:
- `walk_linear` iterates all slides
- `parse_slide_xml` processes each slide fully
- Metadata `num_pages` is correct
- Chapter DocItems are created for each slide

### ✅ Test Corpus Fixed

Changed LLM validation test to use `powerpoint_sample.pptx` (3 slides).

### ⚠️ Still Below Threshold (87% vs 95%)

**Real issues found:**
1. Images not extracted
2. Some shapes not extracted
3. Minor formatting inconsistencies

**Recommended Next Steps (N=1233):**
1. Investigate image extraction from PPTX
2. Check shape parsing completeness
3. Compare with Python docling PPTX backend for missing features

---

## Files Modified

1. **crates/docling-backend/src/pptx.rs:**
   - Added `test_multi_slide_extraction()` test (line 3629)
   - Validates multi-slide extraction works correctly

2. **crates/docling-core/tests/llm_docitem_validation_tests.rs:**
   - Changed test file from `business_presentation.pptx` → `powerpoint_sample.pptx`
   - Added explanatory comment about the change

3. **test_pptx_slides.rs:** (temporary debug script, can delete)
   - Created for manual testing

---

## Key Learnings

### Lesson: Trust But Verify LLM Reports

The LLM report "only one slide and a table present" was factually correct - the test file DID only have one slide. Always check test inputs before assuming code bugs.

### Lesson: Test Files Matter

Using a single-slide file to test multi-slide extraction gives false negatives. Test corpus must match expected behavior.

### Lesson: Multi-Layer Validation

- **Unit tests:** Backend extracts data correctly ✅
- **Integration tests:** Markdown comparison (may miss structural issues) ⚠️
- **LLM validation:** DocItem completeness check (catches real gaps) ✅

DocItem validation methodology validated - it found real issues (images, shapes) once the test file was correct.

---

## Next AI (N=1233): Investigate PPTX Image/Shape Extraction

**Priority:** Medium (87% is decent, but below 95% threshold)

**Investigation:**
1. Check if PPTX backend has image extraction code
2. Compare with Python docling for missing features
3. Identify which shapes are being skipped

**Files to check:**
- `crates/docling-backend/src/pptx.rs` - `parse_slide_xml` method
- `~/docling/docling/backend/mspowerpoint_backend.py` - Python reference
- Test files with known images/shapes

**Expected Findings:**
- Image parsing may be stubbed out or incomplete
- Shape types (text boxes, diagrams, etc.) may not all be handled
