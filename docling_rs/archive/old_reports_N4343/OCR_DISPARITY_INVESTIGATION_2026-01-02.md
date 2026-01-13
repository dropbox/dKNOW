# OCR Output Disparity Investigation Report

**Date:** 2026-01-02
**Worker:** WORKER0, Iteration N=4320
**Status:** In Progress (test running)

## Issue Summary

PDF OCR output shows inconsistent results vs Python docling:
- **amt_handbook_sample.pdf**: 105% of expected (3,792 vs 3,619 bytes) ✓ MORE text
- **edinet_sample.pdf**: 113% of expected (56,738 vs 49,989 bytes) ✓ MORE text
- **jfk_scanned.pdf**: 47% of expected (~49,001 vs 104,833 chars) ✗ LESS text

## Investigation Findings

### 1. OCR Detection/Recognition - VERIFIED CORRECT
- 15-page validation tests show Rust produces **identical** boxes to Python
- 642 boxes detected for 15 pages (avg ~43/page)
- Max box height: 64 pts (well below 100.0 threshold)
- text_score threshold: 0.5 (same as Python)

### 2. OCR Cell Flow Traced
```
Detection (RapidOCR DbNet) → Recognition (RapidOCR CRNN) →
SimpleTextCell → ModularTextCell →
Stage 4 (Cell Assignment, skip Picture clusters) →
Stage 6 (Add OCR cells to Picture clusters if >50% overlap) →
Stage 9 (Create DocumentElement with cells) →
convert_stage10_to_clusters (preserve cells) →
PageAssembler.assemble_page → FigureElement (with cluster.cells) →
docling_export.process_figure_element → Picture.ocr_text →
Markdown serialization
```

### 3. Key Code Points Verified
- Stage 6: `picture_ocr_cells` collection at lines 374-378
- Stage 9: `cluster_info.cells` populated at lines 571-580
- Executor: `convert_cluster_info_to_cluster` preserves cells at lines 1417-1422
- docling_export: `fig_elem.cluster.cells` iterated at line 228

### 4. Threshold Parameters Checked
- `text_score`: 0.5 (matches Python default)
- `inside_container_threshold`: 0.5 (50% overlap for Picture inclusion)
- `max_cell_height`: 100.0 pts (OCR boxes max 64 pts, not filtering)
- `min_overlap` (Stage 4): 0.2 (20%)

### 5. Difference Between Working and Failing PDFs
| File | Pages | Rust/Python Ratio | Note |
|------|-------|-------------------|------|
| amt_handbook | 2 | 105% | Small, Rust produces MORE |
| edinet_sample | ~30 | 113% | Medium, Rust produces MORE |
| jfk_scanned | 270 | 47% | Large, Rust produces LESS |

## Hypotheses to Test

### H1: Scale Issue with Large Documents
270 pages may trigger different behavior than smaller documents.
Check: Processing logs for page count, memory issues.

### H2: OCR Engine Difference
Python's `OcrAutoOptions` on macOS defaults to `ocrmac` (Apple's native OCR), not RapidOCR.
Check: What OCR engine generated the groundtruth?

### H3: Picture Cluster Coverage
For scanned pages, layout model might detect full-page Picture clusters.
If OCR cells don't have >50% overlap, they may not be assigned.
Check: Stage 6 cell assignment logs for jfk_scanned.

### H4: Modular Pipeline to PageAssembler Flow
Cells might be lost in the conversion between Stage 10 output and PageAssembler.
Check: Verify `cluster.cells` at PageAssembler input.

## Test Running

Background test started: `test_canon_pdf_jfk_scanned_ocr`
- Task ID: b8e3acd
- Status: Running (270 pages, debug mode)
- CPU time: ~220+ minutes

## Next Steps

1. Wait for test to complete and check actual output
2. Compare output character count with previous commits
3. Add diagnostic logging to count cells at each pipeline stage
4. If still 47%, investigate specific differences between small/large PDFs
5. Consider lowering `inside_container_threshold` from 0.5 to 0.2

## Files Modified

None - this was a read-only investigation.

## Commands to Check Test Status

```bash
# Check if test is still running
ps aux | grep integration_tests

# Check test output
tail -50 /tmp/claude/-Users-ayates-docling-rs/tasks/b8e3acd.output

# Get task output (blocking)
# Use TaskOutput tool with task_id=b8e3acd
```
