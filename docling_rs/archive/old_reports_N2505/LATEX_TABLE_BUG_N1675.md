# LaTeX Table Parsing Bug (Discovered N=1675, Fixed N=1676)

**Status:** ✅ FIXED - Tables now appear in output
**Severity:** MEDIUM - Affects LaTeX quality score (66% → likely due to this)
**Discovered:** N=1675 during post-cleanup testing
**Fixed:** N=1676

---

## Problem

LaTeX files with tables produce output **missing tables entirely**.

**Test Case:** `test-corpus/latex/simple_document.tex`

**Expected Output:**
```markdown
# Tables

| Left | Center | Right |
|------|--------|-------|
| A    | B      | C     |
| 1    | 2      | 3     |
```

**Actual Output:**
```markdown
# Tables

(empty section - table missing!)
```

**JSON Test:** `jq '.content_blocks | map(select(.type == "table")) | length'` → **0 tables**

---

## Investigation

**Location:** `crates/docling-latex/src/latex.rs:186-204`

**Code Analysis:**

1. **Table extraction method exists** (lines 186-204):
   ```rust
   fn extract_tables(&self, source: &str) -> Vec<(Vec<Vec<String>>, usize)>
   ```

2. **Regex pattern** (line 191):
   ```rust
   let tabular_re = Regex::new(r"(?s)\\begin\{tabular\}\{[^}]*\}(.*?)\\end\{tabular\}").unwrap();
   ```

3. **DocItem::Table creation** (lines 303-341):
   - Tables ARE converted to DocItem::Table
   - Grid structure populated correctly

4. **JSON output check:**
   - 0 tables in content_blocks
   - This means the **regex is not matching**

---

## Root Cause (Hypothesis)

The regex expects:
```
\begin{tabular}{...}...\end{tabular}
```

But LaTeX file has:
```latex
\begin{table}[h]
\centering
\begin{tabular}{|l|c|r|}
...
\end{tabular}
\caption{A simple table}
\end{table}
```

**Possible Issues:**
1. Regex doesn't match column spec with pipes: `{|l|c|r|}`
   - Pattern: `\{[^}]*\}` should match, but verify
2. Paragraph extraction (line 242) might be removing tables before table extraction runs
   - Pattern removes: `\\begin\{table\}.*?\\end\{table\}`
   - This would delete the entire table environment including tabular!

---

## Fix Strategy

**Priority 1: Check execution order**
- If `extract_paragraphs()` runs before `extract_tables()`, it removes tables
- Solution: Extract tables FIRST, then remove from source before paragraph extraction

**Priority 2: Fix regex patterns**
- Update paragraph regex to NOT remove tabular (only remove table environment wrapper)
- Or: Extract tabular from table environments before cleaning

**Code Location:** `parse_latex_to_doc_items()` (line 257)

**Current Order:**
1. Sections (line 262)
2. Lists (line 282)
3. Tables (line 304)
4. Paragraphs (line 344)

**Correct Order:** Tables BEFORE paragraphs ✓ (already correct!)

---

## Fix (N=1676)

**Root Cause:** Serializer bug, not parser bug
- LaTeX parser correctly extracted tables (confirmed with debug logging)
- Table appeared in `content_blocks` JSON (verified with jq)
- BUT: LaTeX backend's markdown generation only serialized `DocItem::Text`
- Other DocItem types (Table, Picture, etc.) were filtered out

**Problem Code (latex.rs:428-438):**
```rust
let markdown = doc_items
    .iter()
    .filter_map(|item| {
        if let DocItem::Text { text, .. } = item {
            Some(text.clone())
        } else {
            None  // ← This dropped ALL non-Text items!
        }
    })
    .collect::<Vec<_>>()
    .join("\n\n");
```

**Solution:**
1. Changed `filter_map` to handle multiple DocItem types
2. Added `DocItem::Table` case that calls `serialize_table_simple()`
3. Implemented `serialize_table_simple()` helper (GitHub-style markdown tables)

**Changes:**
- `crates/docling-latex/src/latex.rs:428-444` - Match on DocItem types
- `crates/docling-latex/src/latex.rs:483-529` - Add `serialize_table_simple()` method

**Verification:**
```bash
cargo run --bin docling -- convert test-corpus/latex/simple_document.tex -o /tmp/latex_test.md
# Output now contains:
# | Left | Center | Right |
# |---|---|---|
# | A | B | C |
# | 1 | 2 | 3 |
```

---

## Impact

**LaTeX Quality Score:** 66% → Expected 75-80% (with table fix)

**Issues from PRIORITY_FORMATS_2025-11-20.md:**
- ✅ Lists - Fixed N=1672
- ✅ **Tables - FIXED N=1676**
- ✅ Formatting (bold/italic) - Fixed N=1672
- ✅ **Metadata (date) - FIXED N=1677**

---

✅ Tables Fixed N=1676
✅ Metadata (date) Fixed N=1677
**All LaTeX priority issues resolved!** Expected quality: 66% → 80-85%
Next AI: Run LLM tests to verify LaTeX quality improvement, then move to VSDX or other priority work
