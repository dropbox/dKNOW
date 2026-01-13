# PDF Text Spacing Investigation - N=2047

**Date:** 2025-11-24
**Status:** Type conversion FIXED ✅, Text spacing BUG identified but NOT YET fixed ❌

## Summary

Successfully implemented Option A (type alignment) which fixed the JSON deserialization error. The PDF ML pipeline now runs end-to-end without type errors. However, the output text has no spaces between words, resulting in 95.4% content loss.

## What Works ✅

1. **Type Conversion:** pdf-ml `DoclingDocument` → core `DoclingDocument` ✅
   - Created `crates/docling-pdf-ml/src/convert_to_core.rs`
   - Proper enum dispatch: `TextItem` → `DocItem` variants
   - Field mappings: `enumerated` defaults, `charspan` tuple→Vec
   - NO MORE deserialization failures

2. **End-to-End Execution:** ML pipeline runs successfully ✅
   - Pdfium loads PDF
   - ML models process pages (Layout, OCR, Tables, Reading Order)
   - `to_docling_document_multi()` creates DoclingDocument
   - Converter transforms to core format
   - Markdown serializer generates output

3. **Test Infrastructure:** pdf_honest_test validates correctly ✅
   - Test compiles and runs
   - Correctly identifies the quality problem
   - Shows garbled output vs expected output

## What's Broken ❌

**Text has no spaces between words:**
- Expected: `"The Evolution of the Word Processor"`
- Actual: `"TheEvolutonoftheWordPrcr"`
- Output: 433 chars (expected 9,456) - 95.4% loss

**Sample garbled output:**
```
TheEvolutonoftheWordPrcr
severaltechlgicalmilestn
PreDigtalEt
TheModern Era(1990s -Present
```

## Root Cause Analysis

### Data Flow

```
PDF Page
  ↓ pdfium text extraction
SimpleTextCell[] (individual words/fragments)
  ↓ ML pipeline (Layout, OCR, Reading Order)
Cluster[] (grouped text cells)
  ↓ layout_postprocessor
PageElement::Text(TextElement)  ← WHERE TEXT IS ASSEMBLED
  ↓ docling_export.rs
TextItem.text  ← Missing spaces appear here
  ↓ convert_to_core.rs
DocItem::Text.text
  ↓ MarkdownSerializer
Garbled markdown output
```

### Where Text Gets Joined

The bug is likely in one of these locations:

1. **Cluster text assembly** (layout_postprocessor.rs)
   - `clusters[idx].cells.push(cell.to_text_cell())`
   - When cells are collected into clusters
   - Need to check how `cluster.text` is computed from cells

2. **TextElement creation** (document_assembler.rs or similar)
   - When Cluster → TextElement
   - `TextElement { text: ???, orig: ???, ... }`
   - This is where cells must be joined with spaces

3. **Cluster.text field** itself
   - May be computed property from cells
   - Or set during cluster creation
   - Need to find where this is populated

### Investigation Commands

```bash
# Find where Cluster.text is set
grep -rn "text:" crates/docling-pdf-ml/src/pipeline/data_structures.rs | grep Cluster

# Find where TextElement.text is assigned
grep -rn "TextElement {" crates/docling-pdf-ml/src/pipeline/

# Find text joining logic
grep -rn "join\|concat" crates/docling-pdf-ml/src/pipeline/

# Look for space insertion
grep -rn "\" \"\|push(' ')" crates/docling-pdf-ml/src/pipeline/
```

## Next Steps

### Immediate (2-4 hours)

1. **Find text assembly location**
   - Search for where `Cluster.text` or `TextElement.text` is populated
   - Look in `layout_postprocessor.rs`, `document_assembler.rs`
   - Find the code that joins cells: `cells.iter().map(|c| c.text).collect()`

2. **Add space joining**
   - Change from: `cells.iter().map(|c| c.text).collect::<String>()`
   - To: `cells.iter().map(|c| c.text.as_str()).collect::<Vec<_>>().join(" ")`
   - Or similar logic that preserves spaces

3. **Test fix**
   ```bash
   source setup_env.sh
   cargo test -p docling-backend --test pdf_honest_test --features pdf-ml -- --nocapture
   ```
   - Should show 9,000+ chars
   - Text should be clean: "The Evolution of the Word Processor"

### Alternative: Check Source Repo

The working Python implementation is at `~/docling_debug_pdf_parsing/`. Compare:
- How Python joins text cells
- Where spaces are inserted
- Any special whitespace handling

Port the correct logic to Rust.

## Test Results

```bash
cargo test -p docling-backend --test pdf_honest_test --features pdf-ml -- --nocapture

╔══════════════════════════════════════════════════════╗
║   HONEST TEST - Pure Rust ML Quality Check          ║
╚══════════════════════════════════════════════════════╝

✓ Pure Rust ML executed
Output: 433 characters  ❌ (expected 9,456)
DocItems: 28

📄 ACTUAL Pure Rust Output:
TheEvolutonoftheWordPrcr
severaltechlgicalmilestn
PreDigtalEt
...

Quality: FAIL - 95.4% content loss
```

## Files Modified (N=2047)

1. `crates/docling-pdf-ml/src/convert_to_core.rs` (NEW)
   - Type converter from pdf-ml to core types
   - Handles all DocItem variant conversions
   - 300+ lines

2. `crates/docling-pdf-ml/src/lib.rs`
   - Export convert_to_core module

3. `crates/docling-backend/src/pdf.rs`
   - Import DocItem, DoclingDocument from core
   - Use convert_to_core_docling_document() instead of JSON

## Success Criteria (Not Yet Met)

- [x] Type conversion works (no deserialization errors)
- [ ] Output: 9,000+ characters (currently 433)
- [ ] Text: Clean and readable (currently garbled)
- [ ] Test: PASSES (currently expected to fail)
- [ ] LLM judge: 100% quality

## Time Estimate

**To fix text spacing:** 2-4 hours
- Find assembly location: 30 min
- Implement fix: 30 min
- Test and iterate: 1-2 hours
- Verify with LLM judge: 30 min

**Total progress so far:** ~3 hours (type conversion)
**Remaining work:** ~3 hours (text spacing)

---

**Next AI:** Start by searching for where `Cluster.text` or `TextElement.text` is populated from cells. The bug is in text cell joining - cells are concatenated without spaces.
