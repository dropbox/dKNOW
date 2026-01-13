# Root Cause Analysis: Page Header Merging Bug

**Date:** 2026-01-06
**Worker:** WORKER0 (N=4417)
**Status:** Root cause identified, fix proposed

---

## Problem Statement

The MANAGER directive (commit 445698f5) asked: "Why does your PDF pipeline produce different output than Python?"

Specific symptom investigated: pg9 output differences between Rust and Python.

---

## Finding: Adjacency Merge Bug

### Evidence

**Python output (2305.03393v1-pg9.json):**
```json
{
  "texts": [
    {"label": "page_header", "text": "Optimized Table Tokenization for Table Structure Recognition"},
    {"label": "page_header", "text": "9"},
    ...
  ]
}
```

**Rust output (test-results/outputs/pdf/2305.03393v1-pg9.json):**
```json
[
  {"label": "page_header", "text": "Optimized Table Tokenization for Table Structure Recognition 9"},
  ...
]
```

### Root Cause

The Rust pipeline has an **adjacency merge** feature added in N=2280 that merges horizontally adjacent text elements. This incorrectly merges the title and page number into one element.

**Location:** `crates/docling-pdf-ml/src/pipeline/layout_postprocessor.rs` lines 1564-1594

**Code (lines 1564-1594):**
```rust
// Fix for N=2280: Merge adjacent text boxes to prevent fragmentation
// Example: "Pre" + "-" + "Digital Era" should be ONE DocItem
else if params.horizontal_gap_multiplier > 0.0
    && cluster.label.is_text_element()
    && other.label.is_text_element()
{
    let gap = other.bbox.l - cluster.bbox.r; // horizontal gap
    // ... merge if gap <= avg_height * horizontal_gap_multiplier
}
```

**Why it merges incorrectly:**
1. Both title and page number are `page_header` elements
2. Both are "text elements" (pass `is_text_element()` check)
3. They are horizontally adjacent on the same line
4. Gap between them is small relative to text height
5. They have vertical alignment

**Python behavior:** Python does NOT have this aggressive adjacency merge for page headers. Each layout detection cluster becomes a separate DocItem.

---

## Proposed Fix

### Option A: Exclude page_header from adjacency merge (Recommended)

```rust
// Only merge CONTENT text, not page furniture
else if params.horizontal_gap_multiplier > 0.0
    && cluster.label.is_text_element()
    && other.label.is_text_element()
    && !cluster.label.is_page_header()  // NEW: Don't merge page headers
    && !other.label.is_page_header()    // NEW: Don't merge page footers
{
    // adjacency merge logic...
}
```

**Rationale:** Page headers and footers are typically multiple separate elements (title, page number, author, etc.) that should remain distinct.

### Option B: Disable adjacency merge entirely

Set `horizontal_gap_multiplier: 0.0` in the default parameters.

**Risk:** May cause fragmentation in other cases (the original N=2280 bug).

### Option C: Stricter adjacency criteria

Only merge if:
- Same label type (not just "text element")
- Semantic continuity (text ends with hyphen, or continuation pattern)

---

## Impact Analysis

This fix affects:
1. **pg9:** Will produce 2 page_headers instead of 1 merged
2. **Other PDFs:** Any document where page header elements are being incorrectly merged
3. **Output length:** May slightly reduce output size (removing " 9" concatenation)

---

## Next Steps

1. Implement Option A (exclude page_header from adjacency merge)
2. Run pg9 test to verify separate page_headers
3. Run full test suite to check for regressions
4. Check other differences between Python and Rust output

---

## Verification Commands

```bash
# Before fix - shows merged page_header
cat test-results/outputs/pdf/2305.03393v1-pg9.json | jq '.[] | select(.label == "page_header") | .text'
# Expected: "Optimized Table Tokenization for Table Structure Recognition 9"

# After fix - should show separate page_headers
# Expected:
#   "Optimized Table Tokenization for Table Structure Recognition"
#   "9"
```

---

**This is an UPSTREAM fix (pipeline logic), not a DOWNSTREAM fix (text regex).**
