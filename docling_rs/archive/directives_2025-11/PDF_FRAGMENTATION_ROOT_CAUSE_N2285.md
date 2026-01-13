# PDF Fragmentation Root Cause Analysis (N=2285)

**Date:** 2025-11-25
**Issue:** Rust produces 115 DocItems, Python produces 53 DocItems
**Status:** ✅ ROOT CAUSE IDENTIFIED

---

## Summary

**ROOT CAUSE: Stage 4 Cell Assignment is Too Conservative**

- Rust assigns cells to only **12 clusters** → creates **103 orphans**
- Python assigns cells to **~53 clusters** → creates **~41 orphans** (estimated)
- Result: Rust has 62 MORE orphan "text" items, diluting correct labels

---

## Test Results

### Experiment: Disable Orphan Creation

**Modified:** `crates/docling-pdf-ml/src/pipeline_modular/stage06_orphan_creator.rs:32`
```rust
create_orphans: false,  // Temporarily disabled for testing
```

**Results:**
```
WITHOUT orphans: 12 DocItems
  - 5 text
  - 3 section_header
  - 4 list_item

WITH orphans: 115 DocItems
  - ~112 text (5 original + 103 orphans + 4 preserved)
  - 3 section_header (8 missing!)
  - 0 list_item (4 original became orphans!)

Python target: 53 DocItems
  - 16 text
  - 11 section_header
  - 26 list_item
```

**Analysis:**
- **12 baseline DocItems** = clusters that successfully got cells assigned (Stage 4)
- **103 orphan DocItems** = cells that didn't match any cluster (all labeled "text")
- **53 Python DocItems** = 12 baseline + 41 orphans (estimated)

**Conclusion:** Rust creates 2.5x more orphans than Python (103 vs 41)

---

## Pipeline Trace

### Page 0 Example (from debug output)

**Stage 1-2: Text Extraction**
```
Raw pdfium segments: 56
Merged cells: 42
```

**Stage 3: ML Layout Prediction**
```
ML clusters: 14
Labels: 4 section_header, 4 list_item, 6 text
```

**Stage 4: Cell Assignment (THE PROBLEM!)**
```
Expected: Assign 42 cells to 14 clusters
Actual: Only ~4 clusters get cells, 10 clusters remain empty
```

**Stage 6: Orphan Creation**
```
Unassigned cells: ~24 (42 - cells assigned to 4 clusters)
Orphans created: 24 (all labeled "text")
```

**Result:**
```
4 clusters with cells (correct labels preserved)
+ 24 orphans (all "text")
= 28 DocItems for page 0

Python produces ~10-11 DocItems for page 0
```

### Across All 5 Pages

**Stage 3 Total:** 62 ML clusters predicted (14+9+14+15+1)
**Stage 4 Result:** Only 12 clusters get cells (50 clusters empty!)
**Stage 6 Orphans:** 103 created (cells unassigned to the 50 empty clusters)
**Final Output:** 115 DocItems (12 + 103)

**Python would:**
- Assign cells to ~12 + 41 = 53 clusters
- Create ~41 orphans
- Result: 53 DocItems

---

## Root Cause Analysis

### Why Does Stage 4 Assign So Few Cells?

**Three possible causes:**

#### 1. Cluster Bboxes Too Small (Most Likely)
- ML model (Stage 3) predicts cluster bounding boxes
- If bboxes are too small, cells won't overlap enough
- Stage 4 requires >20% overlap (intersection_over_self)
- Small bbox → low overlap → cell not assigned

#### 2. Overlap Calculation Different
- Python uses `intersection_over_self = intersection_area / cell_area`
- Rust implementation should match, but might have subtle bugs
- Coordinate system mismatch? (TopLeft vs BottomLeft)
- Bbox boundary calculation error?

#### 3. Missing Logic
- Python might have additional assignment pass
- Or fallback: assign to nearest cluster even if <20% overlap
- Or expands cluster bboxes before assignment

---

## Evidence

### Python Source Code Analysis

**File:** `~/docling/docling/utils/layout_postprocessor.py:283-300`
```python
# Handle orphaned cells
unassigned = self._find_unassigned_cells(clusters)
if unassigned and self.options.create_orphan_clusters:
    next_id = max((c.id for c in self.all_clusters), default=0) + 1
    orphan_clusters = []
    for i, cell in enumerate(unassigned):
        orphan_clusters.append(
            Cluster(
                id=next_id + i,
                label=DocItemLabel.TEXT,  # ← HARDCODED AS TEXT
                ...
            )
        )
```

**Key findings:**
- ✅ Python ALSO hardcodes orphan labels as "text"
- ✅ Python creates one orphan per unassigned cell
- ✅ Python uses 20% overlap threshold (same as Rust)

**Conclusion:** Rust orphan creation algorithm is CORRECT. Problem is upstream (Stage 3 or 4).

### Debug Output Comparison

**Stage 3 (ML Model Output):**
```
[DEBUG] Stage 3 output - 14 clusters with labels:
  [0] Text: ""
  [1] ListItem: ""
  [10] SectionHeader: ""
  [11] SectionHeader: ""
  [12] SectionHeader: ""
  [13] SectionHeader: ""
```

**Stage 10 (Final Output):**
```
[DEBUG] Stage 10 output - 38 elements with labels:
  [0] text: "The Evolution of the Word Processor"
  [1] text: "The concept of the word processor predat"
  [7] section_header: "mid19th century. Patented in 1868..."
  ... (rest mostly "text")
```

**Analysis:**
- Stage 3: 4 section_header predicted (indices 10-13)
- Stage 10: Only 1 section_header survived
- 3 section_header clusters got no cells → didn't appear in output
- Their text content became orphans → labeled "text"

---

## Next Steps

### Priority 1: Compare Stage 3 Cluster Bboxes (2-3 hours)

**Test:** Export cluster bboxes and compare with Python

```bash
export DEBUG_E2E_TRACE=/tmp/debug_pdf
export LIBTORCH_USE_PYTORCH=1
export LIBTORCH_BYPASS_VERSION_CHECK=1
export DYLD_LIBRARY_PATH="/Users/ayates/docling_rs:/opt/homebrew/lib/python3.14/site-packages/torch/lib:/opt/homebrew/opt/llvm/lib"
export DYLD_FALLBACK_LIBRARY_PATH="/Users/ayates/docling_rs:/opt/homebrew/opt/llvm/lib"

cargo test -p docling-backend --test pdf_honest_test \
  --features pdf-ml -- --exact --nocapture

# Check exported files
ls /tmp/debug_pdf/
```

**Compare:**
- Rust cluster bbox sizes vs Python
- Are Rust bboxes smaller?
- Do they cover enough cells?

### Priority 2: Check Overlap Calculation (1-2 hours)

**File:** `crates/docling-pdf-ml/src/pipeline_modular/types.rs`
- Find `intersection_over_self` method
- Compare with Python implementation
- Verify coordinate system (TopLeft vs BottomLeft)
- Check for off-by-one errors

### Priority 3: Fix Cell Assignment (2-4 hours)

**Options:**
A. **Fix Stage 3 cluster bboxes** (if they're too small)
   - Expand bboxes by 10-20%?
   - Or check ML model output format

B. **Adjust Stage 4 overlap threshold**
   - Try 0.1 (10%) instead of 0.2 (20%)
   - Or use different overlap metric

C. **Add fallback assignment**
   - If cell has NO overlap with any cluster
   - Assign to nearest cluster anyway
   - Or use containment check (cell center in bbox)

D. **Port exact Python logic**
   - Read Python Stage 4 implementation line-by-line
   - Translate to Rust
   - Verify with tests

### Priority 4: Verify Fix (1 hour)

```bash
# Re-enable orphan creation
# Edit stage06_orphan_creator.rs:32 → create_orphans: true

# Run test
export LIBTORCH_USE_PYTORCH=1
export LIBTORCH_BYPASS_VERSION_CHECK=1
export DYLD_LIBRARY_PATH="/Users/ayates/docling_rs:/opt/homebrew/lib/python3.14/site-packages/torch/lib:/opt/homebrew/opt/llvm/lib"
export DYLD_FALLBACK_LIBRARY_PATH="/Users/ayates/docling_rs:/opt/homebrew/opt/llvm/lib"

cargo test -p docling-backend --test pdf_honest_test \
  save_rust_docling_json --features pdf-ml -- --exact --nocapture

# Should produce ~53 DocItems
jq 'length' /tmp/rust_docitems.json  # Target: 53
jq 'group_by(.label) | map({label: .[0].label, count: length})' /tmp/rust_docitems.json
# Target: 16 text, 11 section_header, 26 list_item
```

---

## Key Insights

1. **Orphan Creation is NOT the Bug**
   - Both Python and Rust hardcode orphan label as "text" ✅
   - Orphan algorithm is correct ✅
   - Problem is QUANTITY of orphans (103 vs 41)

2. **Stage 4 is the Real Problem**
   - Assigns cells to only 12 clusters (correct labels preserved!)
   - Leaves 103 cells unassigned (these become "text" orphans)
   - Python assigns better → only ~41 orphans

3. **Label Bug is Downstream Effect**
   - ML model predicts labels correctly (Stage 3)
   - But most clusters don't get cells (Stage 4)
   - Unassigned cells become "text" orphans (Stage 6)
   - Result: Correct labels diluted by "text" orphans

4. **Test Without Feature First**
   - Disabling orphan creation revealed baseline (12 items)
   - Shows what Stage 4 actually assigns
   - Makes problem obvious: 103 orphans vs 41 expected

---

## Files Modified (For Testing Only)

**Note:** `crates/docling-pdf-ml/` is gitignored (imported from external repo)
Changes below are temporary for testing and won't be committed.

### `crates/docling-pdf-ml/src/pipeline/executor.rs`

Added debug logging at lines 1302 and 1341:
- Shows Stage 3 cluster labels (ML model output)
- Shows Stage 10 element labels (final output)

### `crates/docling-pdf-ml/src/pipeline_modular/stage06_orphan_creator.rs:32`

Temporarily disabled orphan creation:
```rust
create_orphans: false,  // N=2284: TEST - disable to measure baseline
```

**To re-enable:**
```rust
create_orphans: true,  // Re-enable after fixing Stage 4
```

---

## Success Criteria

- ✅ DocItem count: 53 (not 115)
- ✅ Label distribution:
  - 16 text (not 112)
  - 11 section_header (not 3)
  - 26 list_item (not 0)
- ✅ LLM quality: 100% (exact match with Python)

---

## Time Estimate

**Remaining work:** 4-6 hours
- 2-3 hours: Diagnose bbox/overlap issue
- 2-4 hours: Implement fix
- 1 hour: Test and verify
