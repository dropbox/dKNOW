# PDF Fragmentation Root Cause Analysis - N=2280

**Date:** 2025-11-25 01:45 PST
**Issue:** PDF produces 115 DocItems instead of 53 (2.17x over-fragmentation)
**Test file:** test-corpus/pdf/multi_page.pdf

## Executive Summary

**ROOT CAUSE IDENTIFIED:** The ML pipeline Stage 3 (layout detection + post-processing) is producing **115 PageElements** instead of **53**. The conversion from PageElements → DocItems is 1:1, so fixing Stage 3 will fix the final count.

**Stage-by-Stage Breakdown (Page 0 of multi_page.pdf):**
```
STAGE 1.5: Pdfium extraction         → 56 raw segments
STAGE 2:   merge_simple_text_cells   → 42 merged cells
STAGE 3:   ML PageElements            → 38 PageElements ❌ (should be ~14)
STAGE 4:   DocItems                   → 115 total (sum all pages) ❌ (should be 53)
```

**Key Finding:** Stage 3 → Stage 4 is **perfectly 1:1**. The bug is NOT in `to_docling_document_multi()` - that function works correctly. The bug is in the **layout post-processing** (stages 4-8 in ModularPipeline).

## Evidence

### 1. Diagnostic Output

Running `pdf_honest_test` with logging enabled shows:

**Page 0:**
- Raw pdfium: 56 segments
- After merge: 42 cells
- ML PageElements: 38
- Expected: ~14 PageElements (53 total / 5 pages ≈ 10-15 per page)

**Total across 5 pages:**
- ML PageElements: 38+19+30+24+4 = **115**
- DocItems: **115** (exact 1:1 match)
- Expected: **53**

### 2. Python Baseline Verification

Expected output from `test-corpus/groundtruth/docling_v2/multi_page.json`:
```json
Total DocItems: 53
Types: {
  "section_header": 11,
  "text": 16,
  "list_item": 26
}

First DocItem:
[0] section_header: "The Evolution of the Word Processor"
```

### 3. Source Repo Status

The source repo (`~/docling_debug_pdf_parsing`) was tested by previous AI and confirmed to work correctly on arxiv, code_and_formula, edinet, jfk PDFs. However:
- Source repo tests validate **PageElements**, not **DocItems**
- Source repo's binary requires page numbers (single-page testing tool)
- Cannot easily verify multi_page.pdf with source repo

## Analysis

### What We Know

1. **✅ Text merging works:** `merge_simple_text_cells` reduces 56→42 cells correctly
2. **✅ DocItem conversion works:** `to_docling_document_multi()` does 1:1 conversion (verified by 115=115)
3. **❌ ML post-processing fails:** Stages 4-8 produce too many PageElements

### Why ML Post-Processing Over-Fragments

The layout post-processor (stages 4-8) has these steps:
- **Stage 3:** Assign text cells to clusters (20% overlap threshold)
- **Stage 4:** Remove empty clusters
- **Stage 5:** Create orphan clusters (unassigned cells → TEXT clusters)
- **Stage 6:** Iterative refinement (bbox adjustment + overlap removal)
- **Stages 7-8:** (reading order, table processing)

**HYPOTHESIS:** Stage 6's `remove_overlapping_clusters()` only merges clusters with **80% overlap** (line 1397 in layout_postprocessor.rs). Adjacent text boxes with NO overlap are kept separate.

**Example fragmentation:**
```
Python:  [section_header: "Pre-Digital Era (19th - Early 20th Century)"]
Rust:    [Text: "Pre"]  [Text: "-"]  [Text: "Digital Era (19th - Early 20th Century)"]
```

These 3 boxes are horizontally adjacent but don't overlap 80%, so they're NOT merged.

### Why Python Works

Python docling must either:
1. Have more aggressive text consolidation in layout post-processing
2. Have additional merging logic after Stage 8
3. Use different layout model parameters that produce fewer bounding boxes
4. Have text-specific merging rules (e.g., merge TEXT clusters if horizontally adjacent)

## Next Steps

### Immediate Actions (2-4 hours)

1. **Add horizontal adjacency merging** to `remove_overlapping_clusters()`:
   - Current: Only merge if overlap ≥ 80%
   - Proposed: ALSO merge TEXT clusters if horizontally adjacent (gap < avg_height)
   - Location: `crates/docling-pdf-ml/src/pipeline/layout_postprocessor.rs:1391`

2. **Verify with logging:**
   ```rust
   // Before remove_overlapping_clusters
   eprintln!("[STAGE 3.1] Before overlap removal: {} clusters", clusters.len());

   // After remove_overlapping_clusters
   eprintln!("[STAGE 3.2] After overlap removal: {} clusters", result.len());
   ```

3. **Test the fix:**
   ```bash
   source setup_env.sh
   cargo test -p docling-backend --test pdf_honest_test --features pdf-ml -- --nocapture
   # Should see: DocItems: 53 (not 115)
   ```

### Alternative Approaches (if above fails)

1. **Check layout model output count:**
   - Add logging right after `layout_predictor.infer()` (executor.rs:1261)
   - If layout model produces 115 bboxes (not 53), problem is in model config

2. **Compare Python layout post-processor:**
   - Port `~/docling/docling/utils/layout_postprocessor.py` line-by-line
   - Check for text-specific merging rules we missed

3. **Reduce merge threshold:**
   - Change `horizontal_threshold_factor` from 1.3 to 2.0 or 3.0
   - More aggressive merging in Stage 2 might help Stage 3

## Files to Modify

1. **`crates/docling-pdf-ml/src/pipeline/layout_postprocessor.rs`**
   - Function: `remove_overlapping_clusters()` (line 1391)
   - Add: Horizontal adjacency check for TEXT clusters

2. **`crates/docling-backend/src/pdf.rs`**
   - Keep existing logging (STAGE 1.5, 2, 3, 4)
   - Add: More detailed Stage 3 substage logging

## Success Criteria

```
✅ EXACTLY 53 DocItems
✅ Types: 16 text, 11 section_header, 26 list_item
✅ Reading order: Title first ([0] section_header)
✅ No fragmentation: "Pre-Digital Era..." is ONE item
✅ Markdown: ~9,456 chars
✅ LLM quality: 100%
```

## References

- Last commit: N=2279 (attempted fixes to merge thresholds - no improvement)
- Directive: `START_HERE_FIX_PDF_NOW.txt`
- Expected output: `test-corpus/groundtruth/docling_v2/multi_page.json`
- Python source: `~/docling/docling/utils/layout_postprocessor.py`
- Rust implementation: `crates/docling-pdf-ml/src/pipeline/layout_postprocessor.rs`
