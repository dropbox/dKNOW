# XLSX Quality Fix Plan - N=1238

**Date:** 2025-11-17
**Current Score:** 86% (need 95%)
**Target:** 95%+

## Issues Found

### 1. Missing Sheet Names in DocItems ❌ CRITICAL
**Current:** Only Table DocItems created (lines 909-913)
**Missing:** SectionHeader DocItems for sheet names
**Python reference:** msexcel_backend.py:230
```python
self.parents[0] = doc.add_group(
    parent=None,
    label=GroupLabel.SECTION,
    name=f"sheet: {name}",
    ...
)
```

**Impact:** LLM cannot see sheet names or sheet structure
**Priority:** CRITICAL - 30% of completeness score

### 2. Missing num_pages in Metadata ❌ HIGH
**Current:** metadata uses `..Default::default()` (line 930)
**Missing:** `num_pages: Some(sheet_names.len())`
**Impact:** LLM cannot verify all sheets were extracted
**Priority:** HIGH - metadata category at 95%, could be 100%

### 3. Cell Formatting Not Captured ⚠️ KNOWN LIMITATION
**Current:** Only cell values extracted
**Missing:** Bold, italic, colors, fonts, number formatting
**Python:** Also doesn't capture full formatting in DocItems
**Impact:** Minor - formatting category expected to be ~75-80%
**Priority:** LOW - acceptable limitation (out of scope)

## Implementation Plan

### Fix 1: Add Section Headers for Sheet Names
**File:** crates/docling-backend/src/xlsx.rs
**Lines:** ~894-914

**Change:**
```rust
for (sheet_idx, name) in sheet_names.iter().enumerate() {
    let range = workbook2.worksheet_range(name)?;
    let merged_regions = workbook2.worksheet_merge_cells(name)?;

    // ADD THIS: Create SectionHeader for sheet name
    let sheet_header = DocItem::SectionHeader(SectionHeaderData {
        text: TextItem {
            text: format!("Sheet: {}", name),
            ...
        },
        level: 1,  // Top-level section
        prov: vec![ProvenanceItem {
            page_no: (sheet_idx + 1) as u32,
            ...
        }],
        ...
    });
    all_doc_items.push(sheet_header);

    // Find tables in sheet (existing code)
    let tables = self.find_data_tables(&range, &merged_regions)?;
    for table in tables {
        let doc_item = self.create_table_docitem(&table, table_index, sheet_idx + 1);
        all_doc_items.push(doc_item);
        table_index += 1;
    }
}
```

**Struct needed:** `SectionHeaderData` from docling-core

### Fix 2: Set num_pages = Sheet Count
**File:** crates/docling-backend/src/xlsx.rs
**Lines:** ~925-931

**Change:**
```rust
metadata: DocumentMetadata {
    num_characters,
    num_pages: Some(sheet_names.len()),  // ADD THIS
    author,
    created,
    modified,
    ..Default::default()
},
```

## Expected Results

**Before:**
- Completeness: 85/100 (missing sheet names)
- Structure: 80/100 (sheet structure not clear)
- Metadata: 95/100 (missing num_pages)
- **Overall: 86%**

**After:**
- Completeness: 95/100 ✅ (all sheets + names visible)
- Structure: 95/100 ✅ (clear sheet hierarchy)
- Metadata: 100/100 ✅ (num_pages = 4)
- **Overall: 95%+** ✅

## Testing

1. **Unit tests:** All existing tests should still pass
2. **DocItem validation:** `cargo test test_llm_docitem_xlsx`
3. **Expected output:** Score ≥ 95%
4. **Cost:** ~$0.02 per test run

## Time Estimate

- **Implementation:** 10-15 minutes
- **Testing:** 5 minutes
- **Documentation:** 5 minutes
- **Total:** ~30 minutes

## Next AI

After implementing these fixes:
1. Run `cargo test --package docling-backend --lib` (verify no regressions)
2. Run `OPENAI_API_KEY="..." cargo test test_llm_docitem_xlsx`
3. Verify score ≥ 95%
4. Update CURRENT_STATUS.md
5. Commit with N=1238 message

---

**Status:** Ready to implement
**Risk:** LOW (small, focused changes)
**Impact:** HIGH (86% → 95%+)
