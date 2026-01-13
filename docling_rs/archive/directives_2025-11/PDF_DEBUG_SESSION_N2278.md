# PDF Debug Session N=2278 - Investigation Complete

**Date:** 2025-11-25
**Task:** Fix PDF to produce EXACTLY 53 DocItems (currently 123)
**Status:** 1 of 2 bugs fixed, root cause identified for second bug

## Executive Summary

Successfully identified and fixed **Bug #1 (reading order)** in commit N=2277.
Identified **Bug #2 (text over-fragmentation)** root cause - ready for next AI to fix.

Current progress: 80 → 123 DocItems (all exported correctly, but too fragmented)
Target: 53 DocItems (need 57% reduction via better text cell merging)

---

## Bug #1: FIXED ✅ - Reading Order Used Wrong Indices

**Location:** `crates/docling-backend/src/pdf.rs:1318`

**Problem:**
```rust
// BEFORE (WRONG):
assembled.elements.iter().enumerate().map(|(i, _)| i)
// Used enumerate index (0, 1, 2...) instead of cluster.id
```

**Fix:**
```rust
// AFTER (CORRECT):
assembled.elements.iter().map(|element| element.cluster().id)
// Use actual cluster IDs from PageElements
```

**Impact:**
- Before: 80 DocItems (43 missing due to wrong CID lookup)
- After: 123 DocItems (all PageElements now exported)

**Commit:** N=2277

---

## Bug #2: IDENTIFIED 🔍 - Text Cell Over-Fragmentation

**Location:** `crates/docling-backend/src/pdf.rs:extract_text_cells_simple`

### Evidence from Output Comparison

**Python (CORRECT - 53 DocItems):**
```
[0] section_header page=1: "The Evolution of the Word Processor"
[1] text page=1: "The concept of the word processor predates..."
[2] section_header page=1: "Pre-Digital Era (19th - Early 20th Century)"
[7] list_item page=1: "∞ IBM MT/ST (Magnetic Tape/Selectric Typewriter)..."
[8] list_item page=1: "∞ Wang Laboratories : In the 1970s..."
```

**Rust (WRONG - 123 DocItems):**
```
[0] text page=1: "The concept of the word processor predates..."
[8] text page=1: "·" ← Just a bullet point! Should be part of list_item
[11] text page=1: "Digital Era (19th  -  Early 20th Century)" ← Missing "Pre-"!
```

### Three Problems Observed

1. **Severe Fragmentation**
   - "Pre-Digital Era" split into separate cells, missing "Pre-"
   - Bullet points ("·", "∞") extracted as separate text cells
   - Sentence fragments scattered across different DocItems

2. **Wrong Label Classification**
   - "Digital Era..." labeled as `text`, should be `section_header`
   - List items labeled as `text`, should be `list_item`
   - Only 3 section_headers detected vs Python's 11
   - Only 4 list_items detected vs Python's 26

3. **Scrambled Reading Order**
   - Body text appears before title
   - Fragments from middle of page appear first
   - Order doesn't match visual document structure

### Stage-by-Stage Analysis

**Added logging to track counts at each stage:**

```
[STAGE 2] Merged text cells: 172 total (42+25+46+54+5 per page)
[STAGE 3] ML PageElements: 123 (38+19+35+27+4 per page)
[STAGE 4] DocItems: 123

Python baseline: 53 DocItems (14+9+14+15+1 per page)
```

**Conclusion:** The problem originates at STAGE 2 (text cell extraction)
- Rust extracts 172 cells → ML reduces to 123 (30% reduction via clustering)
- Python extracts fewer cells → ML reduces to 53 (70% better merging)

---

## Root Cause: extract_text_cells_simple()

**File:** `crates/docling-backend/src/pdf.rs:167-695`

**Function call chain:**
```
extract_text_cells_simple()
  ↓
  compute_text_cells() (lines 354-472)
    → Extracts raw text objects from pdfium
  ↓
  merge_horizontal_cells() (lines 481-687)
    → Merges horizontally adjacent cells
  ↓
  Return SimpleTextCell[]
```

**Hypothesis:** One of these is wrong:
1. `compute_text_cells` extracts too granularly (sub-word level)
2. `merge_horizontal_cells` parameters are too conservative
3. `DEFAULT_HORIZONTAL_THRESHOLD_FACTOR` / `DEFAULT_VERTICAL_THRESHOLD_FACTOR` values wrong

---

## Diagnostic Data for Next AI

### Test File
- **PDF:** `test-corpus/pdf/multi_page.pdf` (5 pages)
- **Python baseline:** `/tmp/python_docitems.json` (53 items)
- **Current Rust output:** `/tmp/rust_docitems.json` (123 items)

### Label Distribution Comparison

| Label           | Python | Rust | Difference |
|----------------|--------|------|------------|
| text           | 16     | 116  | +100 (+625%) |
| section_header | 11     | 3    | -8 (-73%)    |
| list_item      | 26     | 4    | -22 (-85%)   |
| **TOTAL**      | **53** | **123** | **+70 (+132%)** |

### Per-Page Counts

| Page | Python | Rust | Ratio |
|------|--------|------|-------|
| 1    | 14     | 38   | 2.7x  |
| 2    | 9      | 19   | 2.1x  |
| 3    | 14     | 35   | 2.5x  |
| 4    | 15     | 27   | 1.8x  |
| 5    | 1      | 4    | 4.0x  |

**Average:** Rust produces 2.3x more DocItems than Python per page

---

## Next Steps for Next AI

### 1. Add More Logging (30 minutes)

Add logging to `extract_text_cells_simple` to diagnose fragmentation point:

```rust
// In extract_text_cells_simple (around line 190)
let raw_cells = Self::compute_text_cells(page, page_height)?;
eprintln!("[STAGE 1.5] Raw pdfium text objects: {}", raw_cells.len());

// After merge_horizontal_cells (around line 218)
eprintln!("[STAGE 2] After merge_horizontal: {}", merged_cells.len());
eprintln!("[STAGE 2] Sample cells:");
for (i, cell) in merged_cells.iter().take(10).enumerate() {
    eprintln!("  [{}] '{}'", i, cell.text.chars().take(50).collect::<String>());
}
```

### 2. Compare with Python Source (1 hour)

**Python reference:**
- File: `~/docling/docling/backend/pypdfium2_backend.py`
- Function: `_compute_text_cells` (lines 158-253)
- Parameters to check:
  - `horizontal_threshold_factor` (default 0.025)
  - `vertical_threshold_factor` (default 0.13)
  - Merging logic differences

**Compare:**
```bash
# Python values
grep "threshold_factor" ~/docling/docling/backend/pypdfium2_backend.py

# Rust values
grep "THRESHOLD_FACTOR" crates/docling-backend/src/pdf.rs
```

### 3. Potential Fixes (2-4 hours)

**Option A: Adjust merge thresholds**
```rust
// Try more aggressive merging
const DEFAULT_HORIZONTAL_THRESHOLD_FACTOR: f64 = 0.05; // Was 0.025?
const DEFAULT_VERTICAL_THRESHOLD_FACTOR: f64 = 0.2;    // Was 0.13?
```

**Option B: Add second merge pass**
```rust
// Merge horizontally first, then merge vertically for multi-line paragraphs
let horizontal_merged = Self::merge_horizontal_cells(...);
let fully_merged = Self::merge_vertical_cells(horizontal_merged, ...);
```

**Option C: Check pdfium extraction mode**
```rust
// Maybe using wrong text extraction mode?
// Check PdfPageTextObject iteration vs PdfPage.text() method
```

### 4. Test and Verify (1 hour)

```bash
source setup_env.sh
cargo test -p docling-backend --test pdf_honest_test \
  save_rust_docling_json --features pdf-ml -- --nocapture

# Check if output matches Python
jq '.texts | length' /tmp/python_docitems.json  # Should be 53
jq 'length' /tmp/rust_docitems.json             # Currently 123, target 53

# Check label distribution
jq 'map(.label) | group_by(.) | map({label: .[0], count: length})' /tmp/rust_docitems.json
```

---

## Success Criteria (0% tolerance!)

As per user directive in `START_HERE_FIX_PDF_NOW.txt`:

- ✅ EXACTLY 53 DocItems (not 52, not 54, not 123)
- ✅ Types: EXACTLY 16 text, 11 section_header, 26 list_item
- ✅ Reading order: Title first, body second
- ✅ No fragmentation: "Pre-Digital Era..." = 1 DocItem (not split)
- ✅ Markdown: ~9,456 chars (derived from correct DocItems)
- ✅ LLM quality: 100% (exact match with Python)

---

## Files Modified

1. `crates/docling-backend/src/pdf.rs`
   - Line 1287: Added STAGE 2 logging (merged text cells)
   - Line 1300: Added STAGE 3 logging (ML PageElements)
   - Line 1318: FIXED - Use cluster.id instead of enumerate index
   - Line 1329: Added STAGE 4 logging (final DocItems)

---

## References

- **User Directive:** `START_HERE_FIX_PDF_NOW.txt`
- **Critical Directive:** `CRITICAL_DOCITEMS_NOT_MARKDOWN.txt`
- **Python Baseline:** `/tmp/python_docitems.json`
- **Current Rust Output:** `/tmp/rust_docitems.json`
- **Comparison Report:** `/tmp/docitems_comparison.txt`
- **Source Repo:** `~/docling_debug_pdf_parsing/` (working code)

---

**Next AI:** Focus on text cell extraction. The ML pipeline code is correct (copied from source). The bug is in how we extract and merge text cells from pdfium before feeding them to the ML pipeline.
