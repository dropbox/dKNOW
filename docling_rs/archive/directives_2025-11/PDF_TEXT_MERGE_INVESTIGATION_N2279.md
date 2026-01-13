# PDF Text Cell Merge Investigation - N=2279

**Date:** 2025-11-25
**Status:** In Progress - Root cause identified, solution pending
**Previous:** N=2278 (Bug #1 fixed, Bug #2 identified)

## Executive Summary

Investigated why Rust text cell merging produces 42 cells on page 0 vs Python's 14 cells (target: 14).
Root cause identified: **Gap between cells (10px) exceeds merge threshold (8-9px)**, preventing merges that Python successfully performs.

**Key Finding:** Both Python and Rust use same threshold_factor (1.0), same merge algorithm logic, but **Python merges these cells and we don't**. The bug is subtle - likely coordinate system issue or pdfium API difference.

---

## Changes Made (N=2279)

###  1. Fixed merge threshold calculation

**Location:** `crates/docling-backend/src/pdf.rs:307-312`

**Before (WRONG):**
```rust
let horizontal_threshold = group_height * horizontal_threshold_factor;
// Used accumulated group height (growing with each merge)
```

**After (CORRECT):**
```rust
let prev_height = prev_cell.rect.b - prev_cell.rect.t;
let curr_height = cell.rect.b - cell.rect.t;
let avg_height = (prev_height + curr_height) / 2.0;
let horizontal_threshold = avg_height * horizontal_threshold_factor;
// Use average of two adjacent cells being compared (matches Python)
```

**Result:** No improvement (still 42 cells)

### 2. Added text re-extraction from pdfium

**Location:** `crates/docling-backend/src/pdf.rs:354-370`

**Before:**
```rust
let merged_text = group_cells.iter().map(|c| c.text.as_str()).join(" ");
// Simple concatenation with spaces
```

**After:**
```rust
let pdf_rect = PdfRect::new_from_values(bottom, left, top, right);
let page_text = page.text()?;
let merged_text = page_text.inside_rect(pdf_rect);
// Re-extract text from pdfium using merged bounding box (matches Python)
```

**Rationale:** Python calls `text_page.get_text_bounded(*merged_bbox)` to let pdfium handle spacing/ligatures correctly.

**Result:** No improvement (still 42 cells)

### 3. Added X-coordinate sorting within rows

**Location:** `crates/docling-backend/src/pdf.rs:298-302`

```rust
// Sort cells by X position (left to right) within the row
row_indices.sort_by(|&a, &b| {
    cells[a].rect.l.partial_cmp(&cells[b].rect.l).unwrap()
});
```

**Result:** No improvement (still 42 cells)

### 4. Added comprehensive debug logging

**Location:** `crates/docling-backend/src/pdf.rs:214-234, 323-328`

Shows:
- Raw pdfium segments count
- Merged cells count
- Sample cell text (first 10)
- Merge failure reasons (gap vs threshold)

---

## Root Cause Analysis

### The Merge Failure Pattern

All merge failures follow the same pattern - **numbered list items**:

```
[MERGE DEBUG] NOT merging: '1. ' + 'Undo/Redo' (gap=10.13, threshold=8.29, prev_h=8.27, curr_h=8.30, avg_h=8.29)
[MERGE DEBUG] NOT merging: '2. ' + 'Spell Check...' (gap=10.48, threshold=9.48, prev_h=8.27, curr_h=10.69, avg_h=9.48)
[MERGE DEBUG] NOT merging: '5. ' + 'Real' (gap=10.08, threshold=8.10, prev_h=8.10, curr_h=8.10, avg_h=8.10)
```

**Pattern:**
- Gap: **10.08 - 10.48 pixels** (consistently ~10px)
- Threshold: **8.10 - 9.48 pixels** (height * 1.0)
- Result: `gap > threshold` → **NO MERGE**

### Why This Matters

These are list items that should merge:
- "1. " + "Undo/Redo" → "1. Undo/Redo"
- "2. " + "Spell Check..." → "2. Spell Check..."

Python merges these successfully. We don't. This accounts for ~14 extra cells per page.

### Verified Facts

1. ✅ Both Python and Rust get **56 raw cells from pdfium** (verified N=2279)
2. ✅ Both use **threshold_factor = 1.0** (default, confirmed in code)
3. ✅ Both use **same merge algorithm logic** (ported line-by-line from Python)
4. ✅ Both use **avg_height of two adjacent cells** (fixed in N=2279)
5. ✅ Both **re-extract text from pdfium** (fixed in N=2279)
6. ✅ Both **sort cells left-to-right** (fixed in N=2279)
7. ❌ **Rust: gap=10px > threshold=8px** → NO MERGE
8. ❓ **Python: (unknown) but DOES merge** → target 14 cells

### The Mystery

**If Python and Rust both:**
- Start with 56 cells
- Use threshold_factor=1.0
- Calculate `threshold = avg_height * 1.0`
- Have cells with height ~8-10px

**Then why does Python merge them but we don't?**

### Hypotheses

**Hypothesis A: Coordinate system bug**
- Maybe our `gap = cell.l - prev_cell.r` is wrong?
- Python uses bottom-left origin, we convert to top-left
- Could conversion be affecting horizontal distances?
- **Test:** Print raw pdfium rect values (before conversion)

**Hypothesis B: Pdfium API difference**
- Python: `text_page.get_rect(i)` returns different values?
- Rust: `text.segments().iter()` via pdfium-render wrapper?
- **Test:** Compare raw pdfium rect coordinates from both

**Hypothesis C: Python uses different threshold somewhere**
- Maybe Python backend has a non-default threshold value?
- Or calls merge multiple times with different params?
- **Test:** Add logging to Python code, print actual threshold used

**Hypothesis D: Height calculation difference**
- We use `b - t` for height (top-left coords)
- Python uses `.height` property - what does that return?
- Could there be abs() or coordinate origin issue?
- **Test:** Print Python's actual height values for same cells

---

## Evidence Data

### Python Final Output (Ground Truth)
```
Python DocItems: 53
Label distribution:
  list_item: 26      ← Lists should be merged!
  section_header: 11
  text: 16
```

### Rust Current Output
```
Rust DocItems: 123
STAGE 2 Page 0: 56 → 42 cells (expected 14)
STAGE 3 Page 0: 42 → 38 ML elements
STAGE 4: 123 DocItems (all labeled as "text")
```

### Gap Analysis (Page 0)
- Rust: 56 → 42 after merge (14 merged, 42 remaining)
- Python: 56 → 14 after merge (42 merged, 14 remaining)
- **Difference:** We merge too conservatively (3x fewer merges than Python)

---

## Next Steps for Next AI

### Immediate (1-2 hours)

**Option 1: Test Hypothesis A (Coordinate Bug)**
```rust
// In merge logic, print RAW coordinates before any conversion:
eprintln!("[COORD DEBUG] prev_cell: l={}, r={}, text='{}'",
    prev_cell.rect.l, prev_cell.rect.r, prev_cell.text);
eprintln!("[COORD DEBUG] cell: l={}, r={}, text='{}'",
    cell.rect.l, cell.rect.r, cell.text);
eprintln!("[COORD DEBUG] gap = {} - {} = {}",
    cell.rect.l, prev_cell.rect.r, cell.rect.l - prev_cell.rect.r);
```

**Option 2: Compare Python's actual values**
```python
# Add to ~/docling/docling/backend/pypdfium2_backend.py merge_row():
print(f"[PY DEBUG] gap={cell.rect.l - prev_cell.rect.r:.2f}, threshold={avg_height * horizontal_threshold_factor:.2f}")
```

Run both, compare for same "1. " + "Undo/Redo" merge decision.

**Option 3: Increase threshold temporarily (diagnostic)**
```rust
let horizontal_threshold_factor = 1.5; // Was 1.0, try 1.5
```
If this fixes it → confirms threshold is the issue, but doesn't explain WHY.

### Medium Term (2-4 hours)

**Option A: Port Python's exact coordinate handling**
- Read Python's BoundingBox/BoundingRectangle classes
- Verify we're using same coordinate transformations
- Check for any rounding, truncation, or precision differences

**Option B: Use Python's merge code directly**
- Call Python's _compute_text_cells from Rust via PyO3
- Compare results cell-by-cell
- Identify exact point where outputs diverge

**Option C: Inspect pdfium-render vs pypdfium2**
- Both wrap same pdfium C library
- But wrappers might handle rects differently
- Check if `text.segments()` equals `text_page.get_rect(i)` exactly

### Alternative Approach

**If above doesn't work:** Don't fix the merge, fix the ML pipeline.

- Our 42 cells become 38 ML elements (ML does some clustering)
- Python's 14 cells become 14 ML elements (no clustering needed)
- Maybe the ML pipeline SHOULD handle fragmentation?
- Check if Python's ML has different clustering parameters

---

## Success Criteria

- ✅ EXACTLY 53 DocItems (current: 123)
- ✅ Page 0: EXACTLY 14 DocItems (current: 38)
- ✅ Merge "1. " + "Undo/Redo" → "1. Undo/Redo"
- ✅ Label distribution: 16 text, 11 section_header, 26 list_item

---

## Files Modified

- `crates/docling-backend/src/pdf.rs`
  - Lines 214-234: Added stage logging
  - Lines 226: Pass page + page_height to merge function
  - Lines 244-255: Updated function signature
  - Lines 298-302: Sort cells by X within rows
  - Lines 307-312: Use avg_height (not group_height)
  - Lines 323-328: Debug logging for merge failures
  - Lines 354-370: Re-extract text from pdfium (not concatenate)

---

## References

- **Previous Report:** PDF_DEBUG_SESSION_N2278.md
- **Python Source:** ~/docling/docling/backend/pypdfium2_backend.py:163-245
- **Test File:** test-corpus/pdf/multi_page.pdf
- **Python Baseline:** 56 raw → 14 merged → 53 DocItems
- **Rust Current:** 56 raw → 42 merged → 123 DocItems

---

**Next AI:** Focus on debugging WHY `gap=10px > threshold=8px` when Python clearly merges these cells. The algorithm logic is correct (ported from Python), so the issue is in coordinate calculation, API differences, or some hidden Python parameter we missed.
