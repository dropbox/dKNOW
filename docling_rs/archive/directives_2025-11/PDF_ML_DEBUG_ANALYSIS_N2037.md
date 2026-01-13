# PDF ML Debug Analysis - Session N=2037

**Date:** 2025-11-24
**Status:** Analysis Complete - Root Cause Identified

## Problem Summary

Pure Rust ML produces **92.6% content loss** (701 chars vs 9,456 chars expected).

**Test output sample:**
```
PreDigtalEt           # Should be: "Pre-Digital Era"
TheEvolutonoftheWordPrcr   # Should be: "The Evolution of the Word Processor"
```

**Symptoms:**
- Missing spaces between words
- Missing letters/characters
- 92% of content lost

## Code Path Traced

```
crates/docling-backend/src/pdf.rs:parse_bytes()
  ↓ (line 1139)
crates/docling-pdf-ml/src/pipeline/executor.rs:Pipeline::process_page()
  ↓ (returns Page with assembled elements)
crates/docling-pdf-ml/src/convert.rs:pages_to_doc_items()
  ↓ (line 337)
crates/docling-pdf-ml/src/convert.rs:page_to_doc_items()
  ↓ (line 320-330)
crates/docling-pdf-ml/src/convert.rs:page_element_to_doc_item()
  ↓ (line 293-313)
crates/docling-pdf-ml/src/convert.rs:text_element_to_doc_item()
  ↓ (line 78, uses element.text field)
crates/docling-pdf-ml/src/convert.rs:export_to_markdown()
  ↓ (line 447-501)
Final markdown string
```

## Root Cause Hypothesis

**The TextElement already has garbled text BEFORE conversion to DocItem.**

The conversion functions in `convert.rs` are simple formatters - they just take `element.text` and create DocItem variants. The bug is EARLIER in the pipeline.

**Most likely location:** `stage09_document_assembler.rs` (text assembly stage)

**Why:** The garbled output shows:
1. **Missing spaces**: "WordProcessor" vs "Word Processor"
2. **Missing letters**: "PreDigtalEt" vs "Pre-Digital Era"
3. **Wrong concatenation**: Text cells not properly joined

This is a TEXT ASSEMBLY bug, not a serialization bug.

## Source Repo Comparison

**Source repo:** `~/docling_debug_pdf_parsing`
- Status: 165/165 tests passing
- Last commit: N=185 (cleanup cycle)
- Has same file structure (docling_export.rs, stage09_document_assembler.rs)
- **Files are 99% similar** (mostly formatting diffs)

**Current repo:** `~/docling_rs/crates/docling-pdf-ml`
- Status: 160/161 tests passing
- Output: 92.6% content loss
- Same file structure

## Investigation Steps Taken

1. ✅ Verified source repo exists and has tests passing
2. ✅ Compared `docling_export.rs` files (only formatting diffs)
3. ✅ Ran honest test confirming 92.6% loss
4. ✅ Traced code path from parse_bytes to markdown output
5. ✅ Identified conversion is simple (bug is earlier)

## Next Steps

**Option A: Copy Working Code from Source Repo** (Recommended - 2-4 hours)
1. Compare `stage09_document_assembler.rs` between repos
2. Find text assembly differences
3. Copy working logic from source repo
4. Test until output is clean

**Option B: Debug Current Code** (4-8 hours)
1. Add extensive debug logging to stage09
2. Print text cells BEFORE assembly
3. Print text cells AFTER assembly
4. Find where spaces/chars are lost
5. Fix the assembly logic

**Option C: Accept Hybrid Approach** (0 hours - already works)
- Use Python serializer with Rust ML
- Already 98% quality
- Keep for now, fix later

## Files to Investigate

**Priority 1:**
- `crates/docling-pdf-ml/src/pipeline/assembly/stage09_document_assembler.rs` (text assembly)
- Compare with: `~/docling_debug_pdf_parsing/src/pipeline_modular/stage09_document_assembler.rs`

**Priority 2:**
- `crates/docling-pdf-ml/src/pipeline/page_assembly.rs` (overall assembly)
- `crates/docling-pdf-ml/src/pipeline/executor.rs` (pipeline orchestration)

**Priority 3:**
- OCR text extraction (if using OCR mode)
- Text cell confidence filtering (may be too aggressive)

## Diagnostic Test

To verify hypothesis, add debug prints to `stage09_document_assembler.rs`:

```rust
// Before assembly
eprintln!("BEFORE ASSEMBLY: {} cells", cluster.cells.len());
for cell in &cluster.cells {
    eprintln!("  Cell: '{}' (len={})", cell.text, cell.text.len());
}

// After assembly
eprintln!("AFTER ASSEMBLY: '{}'", assembled_text);
```

Run test and check if cells have:
- Correct text BEFORE assembly? → Bug is in assembly logic
- Garbled text BEFORE assembly? → Bug is earlier (OCR or cell extraction)

## Success Criteria

**Current:** 701 chars, garbled "PreDigtalEt"
**Target:** 9,000+ chars, clean "Pre-Digital Era"
**Test:** `cargo test -p docling-backend --test pdf_honest_test --features pdf-ml`

## Recommendation

**Start with Option A** (copy from source):
1. Diff `stage09_document_assembler.rs` files
2. If substantive differences found, copy working version
3. Re-test
4. If still broken, proceed to Option B (debug)

**Time estimate:** 2-4 hours if source has working code, 8-16 hours if needs debugging from scratch.
