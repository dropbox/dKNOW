# LLM Test Variance Analysis - N=1249

**Date:** 2025-11-17
**Context:** Investigating score "regression" from N=1247 (94%) to N=1248 (92%) after list groups implementation

---

## Executive Summary

**Finding:** The reported 92% score at N=1248 was NOT a regression but normal LLM test variance (±2%).

**Evidence:** Ran test 3 times with identical code:
- Run 1: **94.0%**
- Run 2: **93.0%**
- Run 3: **92.0%**

**Recommendation:** REVERT list groups implementation (N=1248). Current implementation doesn't improve quality and requires major architectural changes to match Python semantics.

---

## Detailed Variance Analysis

### Test Results (3 Runs, Same Code)

| Run | Overall | Text | Structure | Tables | Images | Metadata |
|-----|---------|------|-----------|--------|--------|----------|
| 1   | 94.0%   | 95   | 90        | 95     | 90     | 95       |
| 2   | 93.0%   | 95   | 90        | 95     | 90     | 85       |
| 3   | 92.0%   | 95   | 90        | 95     | 90     | 85       |

**Key Observations:**

1. **Stable Categories:** Text Content, Structure, Tables, Images (always 90-95)
2. **Variable Category:** Metadata fluctuates between 85-95 (±10 points!)
3. **Overall Variance:** 92-94% (±2% standard deviation)
4. **Mean Score:** 93.0%

### Implications

- N=1247 reported 94% → likely Run 1 result
- N=1248 reported 92% → likely Run 3 result
- **No actual regression occurred**
- LLM assessment has inherent 2-5% variability

---

## Root Cause Analysis: Why Didn't Groups Help?

### Python's Group Parent Logic

Examined `~/docling/docling/backend/msword_backend.py:1166-1190`:

```python
# Python maintains hierarchical level tracking
self.parents: dict[int, Optional[NodeItem]] = {}  # Level → parent mapping

# When creating list group:
list_gr = doc.add_list_group(name="list", parent=self.parents[level - 1])
self.parents[level] = list_gr  # Update level for children
```

**Key features:**
- **Level-based parent tracking:** Each level (0-10) has assigned parent
- **Shared parents:** Multiple groups at same level share same parent
- **Hierarchical nesting:** Indented lists create nested group structures

### Rust's Group Parent Logic (Current)

From `docx.rs:865-950` (post-processing approach):

```rust
// Post-processing after walk_body() completes
fn create_list_groups(doc_items: Vec<DocItem>) -> Vec<DocItem> {
    let mut last_non_list_ref: Option<String> = None;

    // For each list group:
    let group_parent = last_non_list_ref.clone();  // ← Sequential, not level-based
    //...
}
```

**Limitations:**
- **No level information:** walk_body() doesn't track document hierarchy levels
- **Sequential parent assignment:** Uses "most recent non-list item"
- **Can't share parents:** Each group gets different parent
- **Post-processing:** Can't access level info that was available during parsing

### Concrete Example: word_sample.docx

**Python Groups:**
```
#/groups/0: parent=#/texts/4 (section header "Let's swim!")
#/groups/1: parent=#/texts/4 (SAME parent - both at same level)
#/groups/2: parent=#/texts/14 (section header "Let's eat")
```

**Rust Groups (Current):**
```
#/groups/0: parent=#/texts/3 (preceding text)
#/groups/1: parent=#/texts/4 (different parent)
#/groups/2: parent=#/texts/9 (wrong parent)
```

**Problem:** Groups 0 and 1 should share parent `#/texts/4` but Rust assigns different parents sequentially.

---

## Why Structure Score Stayed at 90%

Despite adding list groups, Structure remained at 90% (no improvement) because:

1. **Wrong parent references:** Groups don't reflect document hierarchy
2. **Missing semantic relationships:** LLM can't recognize proper nesting
3. **No hierarchical depth:** Groups lack level/depth information
4. **Possibly not valued:** LLM may not weight list groups highly in assessment

---

## What Would Fix This Properly?

### Required Architectural Changes

1. **Add level tracking to walk_body():**
   ```rust
   struct ParserState {
       parents: HashMap<usize, Option<String>>,  // Level → parent self_ref
       current_level: usize,
       // ...
   }
   ```

2. **Create groups DURING parsing, not after:**
   ```rust
   // When encountering list item:
   if is_new_list {
       let group_ref = format!("#/groups/{}", group_idx);
       let group = DocItem::List {
           parent: state.parents.get(&(level - 1)).cloned(),
           // ...
       };
       state.parents.insert(level, Some(group_ref.clone()));
   }
   ```

3. **Track heading levels and section structure:**
   - Update `state.parents[level]` when entering/exiting headings
   - Maintain stack of open sections/headings
   - Close groups when leaving scope

### Estimated Effort

- **Complexity:** HIGH (requires refactoring 500+ line walk_body function)
- **Risk:** MEDIUM (could break existing parsing)
- **Time:** 3-5 commits (2-4 hours AI work)
- **Testing:** Must re-run all 97 canonical tests

### Alternative: Revert and Focus Elsewhere

**Cost-benefit analysis:**
- **Cost:** 3-5 commits, high risk, complex refactoring
- **Benefit:** UNCERTAIN - Structure may still stay at 90%
- **ROI:** LOW

**Better opportunities:**
- Metadata improvements (varies 85-95, easier to fix)
- Table formatting (Python has better table serialization)
- Image captions (Python extracts more image metadata)

---

## Recommendation

### Action: REVERT List Groups Implementation

```bash
git revert b8775d4  # N=1248 commit
```

**Rationale:**
1. No quality improvement (Structure still 90%)
2. Wrong semantics (doesn't match Python)
3. Architectural limitations (post-processing can't work)
4. Better ROI elsewhere (Metadata, Tables, Images)

### Focus Instead On

1. **Metadata gaps** (85-95 variance suggests missing fields):
   - Document properties
   - Style information
   - Custom properties

2. **Table formatting** (Tables: 95 → could be 100):
   - Cell alignment
   - Merged cells
   - Table styles

3. **Image metadata** (Images: 90 → could be 95+):
   - Captions
   - Alt text
   - Size/DPI

---

## Lessons Learned

### Lesson 1: LLM Tests Have Variance

- ±2-5% is normal for LLM assessments
- Single test runs can be misleading
- Always run 3+ times to measure baseline
- Use average, not single data point

### Lesson 2: Post-Processing Has Limits

- Can't reconstruct information lost during parsing
- Level/hierarchy must be tracked DURING parsing
- Document structure is context-dependent

### Lesson 3: Study Python Source First

- JSON comparison shows WHAT differs
- Python source shows WHY it differs
- Understanding semantics prevents wasted work

### Lesson 4: ROI Matters

- Not all Python features are worth porting
- Focus on high-impact, low-complexity wins
- Complex refactors need strong justification

---

## Test Commands (For Reference)

### Run LLM DocItem Quality Test

```bash
export OPENAI_API_KEY="[API_KEY_FROM_.env]"

/Users/ayates/.cargo/bin/cargo test test_llm_docitem_docx \
  --test llm_docitem_validation_tests -- --nocapture
```

**Note:** Test is NOT `#[ignore]` so don't use `--ignored` flag!

**Cost:** ~$0.02 per run (GPT-4o API call)
**Time:** 5-20 seconds per run

---

## Conclusion

The reported "regression" from 94% to 92% was **measurement variance, not a real quality drop**.

The list groups implementation:
- ✅ Technically works (creates groups)
- ❌ Doesn't match Python semantics
- ❌ Doesn't improve quality scores
- ❌ Requires major refactoring to fix properly

**Action:** Revert N=1248, focus on higher-ROI improvements (Metadata, Tables, Images).

**Next Worker (N=1250):** After revert, investigate Metadata gaps (85-95 variance) or Table formatting (Python achieves better serialization).

---

**Generated:** N=1249, 2025-11-17
**Author:** Claude AI Worker
**Status:** Complete - Ready for decision
