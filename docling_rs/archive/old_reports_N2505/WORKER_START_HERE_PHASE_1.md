# WORKER: START HERE - Phase 1 Instructions

**Date:** 2025-11-22 13:50 PT
**Authorization:** USER APPROVED - Begin Phase 1
**Branch:** feature/pdf-ml-migration
**Your Mission:** Merge ML-based PDF parser from ~/docling_debug_pdf_parsing

---

## ⚡ CRITICAL: Read These First

**Before writing ANY code, read these 3 documents:**

1. **MANAGER_EXECUTIVE_BRIEFING_PDF_MERGE.md** (Overview, decisions, timeline)
2. **MANAGER_PDF_ML_MERGE_DIRECTIVE_2025-11-22.md** (Complete 14-phase plan)
3. **This file** (Phase 1 specific instructions)

---

## Pre-Flight Checklist ✅

### Environment Verified

```bash
cd ~/docling_rs
git checkout feature/pdf-ml-migration
git status  # Should be clean
cargo check -p docling-pdf-ml  # Should compile (empty skeleton)
cargo check -p docling-ocr     # Should compile (ort 2.0 fixed)
```

### Source Repository Verified

```bash
cd ~/docling_debug_pdf_parsing
git status  # Should show N=185 (Cleanup cycle)
cargo test  # Should show 165+3+21 = 189 tests passing
```

### User Approvals Obtained

- ✅ Complete replacement (delete simple backend)
- ✅ 5-7 week timeline
- ✅ 207 test requirement (100% pass rate)
- ✅ Rust tests only (no pytest port needed)

---

## Phase 1: Core Types & Conversions (2-3 days)

### Goal

Copy core data structures from source and create type conversions to docling-core types.

### Step 1: Copy Source Files (1 hour)

```bash
cd ~/docling_rs/crates/docling-pdf-ml

# Copy data structures
cp ~/docling_debug_pdf_parsing/src/pipeline/data_structures.rs \
   src/types/data_structures.rs

# Copy baseline loading infrastructure (for tests)
cp ~/docling_debug_pdf_parsing/src/baseline.rs \
   src/baseline.rs

# Review error types (may need to merge with existing)
# Source: ~/docling_debug_pdf_parsing/src/error.rs
# Target: Already exists in docling-core
# Action: Add PDF-specific errors to docling-core if needed
```

### Step 2: Review Copied Files (30 min)

Open `src/types/data_structures.rs` and understand:
- `PageElement` struct (represents detected elements)
- `Cluster` struct (layout detection results)
- `TextCell` struct (OCR text boxes)
- `BBox` struct (bounding boxes)
- Other core types

**Key types to understand:**
```rust
pub struct PageElement {
    pub label: String,        // "Text", "Title", "Table", etc.
    pub bbox: BBox,           // Coordinates
    pub cluster_id: Option<usize>,
    pub text: String,
    pub children: Vec<PageElement>,
    // ... more fields
}

pub struct Cluster {
    pub label: ClusterLabel,
    pub bbox: BBox,
    pub confidence: f32,
    pub cells: Vec<TextCell>,
    // ... more fields
}
```

### Step 3: Create Type Conversions (4-6 hours)

Create `src/convert.rs`:

```rust
//! Type conversions from PDF ML types to docling-core DocItem types

use crate::types::data_structures::{PageElement, Cluster, BBox};
use docling_core::{DocItem, DocItemLabel, BoundingBox, /* ... */};

/// Convert PDF ML PageElement to docling-core DocItem
pub fn page_element_to_doc_item(element: &PageElement) -> DocItem {
    DocItem {
        label: convert_label(&element.label),
        bbox: convert_bbox(&element.bbox),
        text: element.text.clone(),
        children: element.children.iter()
            .map(|child| page_element_to_doc_item(child))
            .collect(),
        // ... map all fields
    }
}

/// Convert PDF ML label to DocItemLabel
fn convert_label(label: &str) -> DocItemLabel {
    match label {
        "Text" => DocItemLabel::Text,
        "Title" => DocItemLabel::Title,
        "Table" => DocItemLabel::Table,
        "Picture" => DocItemLabel::Picture,
        "Caption" => DocItemLabel::Caption,
        // ... map all labels
        _ => DocItemLabel::Text, // fallback
    }
}

/// Convert PDF ML BBox to docling-core BoundingBox
fn convert_bbox(bbox: &BBox) -> BoundingBox {
    BoundingBox {
        l: bbox.x_min as f64,
        t: bbox.y_min as f64,
        r: bbox.x_max as f64,
        b: bbox.y_max as f64,
        coord_origin: CoordOrigin::TopLeft,
    }
}

/// Convert Cluster to DocItem
pub fn cluster_to_doc_item(cluster: &Cluster) -> DocItem {
    // Similar conversion logic
    // ...
}

/// Convert list of PageElements to Vec<DocItem>
pub fn page_to_doc_items(elements: &[PageElement]) -> Vec<DocItem> {
    elements.iter()
        .map(|e| page_element_to_doc_item(e))
        .collect()
}
```

**Important mapping considerations:**
- PDF ML uses `x_min, y_min, x_max, y_max` → docling-core uses `l, t, r, b`
- PDF ML label strings → docling-core `DocItemLabel` enum
- PDF ML `PageElement` hierarchy → docling-core `DocItem` hierarchy

### Step 4: Write Unit Tests (2-3 hours)

Create `tests/test_conversions.rs`:

```rust
use docling_pdf_ml::convert::*;
use docling_pdf_ml::types::data_structures::*;

#[test]
fn test_page_element_to_doc_item() {
    let element = PageElement {
        label: "Title".to_string(),
        bbox: BBox { x_min: 10.0, y_min: 20.0, x_max: 100.0, y_max: 30.0 },
        text: "Test Title".to_string(),
        children: vec![],
        // ... other fields
    };

    let doc_item = page_element_to_doc_item(&element);

    assert_eq!(doc_item.label, DocItemLabel::Title);
    assert_eq!(doc_item.text, "Test Title");
    assert_eq!(doc_item.bbox.l, 10.0);
    // ... more assertions
}

#[test]
fn test_label_conversion() {
    // Test all label mappings
    assert_eq!(convert_label("Text"), DocItemLabel::Text);
    assert_eq!(convert_label("Title"), DocItemLabel::Title);
    // ... test all labels
}

#[test]
fn test_bbox_conversion() {
    let bbox = BBox { x_min: 0.0, y_min: 10.0, x_max: 100.0, y_max: 50.0 };
    let bb = convert_bbox(&bbox);

    assert_eq!(bb.l, 0.0);
    assert_eq!(bb.t, 10.0);
    assert_eq!(bb.r, 100.0);
    assert_eq!(bb.b, 50.0);
}
```

### Step 5: Update Module Structure (30 min)

Update `src/lib.rs`:

```rust
//! PDF ML-based parsing library
//!
//! Complete ML pipeline for PDF document analysis with 5 models:
//! - RapidOCR (detection, classification, recognition)
//! - LayoutPredictor (document structure)
//! - TableFormer (table parsing)
//! - CodeFormula (optional enrichment)
//! - ReadingOrder (spatial ordering)

pub mod types;
pub mod convert;
pub mod baseline; // For tests

// Re-exports
pub use convert::{page_element_to_doc_item, page_to_doc_items};
pub use types::data_structures::{PageElement, Cluster, BBox};
```

### Step 6: Run Tests (30 min)

```bash
cargo test -p docling-pdf-ml
# Should see: test result: ok. X passed; 0 failed
```

**Expected:** All conversion tests passing

### Step 7: Commit Phase 1 (15 min)

```bash
git add -A
git commit -m "# 0: PDF ML Phase 1 - Core types, baseline loading, type conversions

**Current Plan**: PDF ML Migration (Phases 1-14, 5-7 weeks)
**Checklist**: Phase 1/14 complete - Core types and conversions ready

## Changes

**Copied from source:**
- src/types/data_structures.rs (PageElement, Cluster, BBox, etc.)
- src/baseline.rs (test infrastructure)

**Created:**
- src/convert.rs (~300 lines) - Type conversions to docling-core
  - page_element_to_doc_item()
  - cluster_to_doc_item()
  - Label mapping (PDF ML strings → DocItemLabel enum)
  - BBox conversion (x_min/y_min/x_max/y_max → l/t/r/b)

**Tests:**
- tests/test_conversions.rs - Unit tests for all conversions

## Tests
X/X tests passing (all conversion tests)

## Next AI
Continue to Phase 2: PDF Reader Component
- Extend crates/docling-backend/src/pdf.rs
- Add render_page_to_array() for ML models
- Add extract_text_cells_simple() for OCR
"
```

---

## Success Criteria for Phase 1

- [ ] `src/types/data_structures.rs` copied and compiling
- [ ] `src/baseline.rs` copied and compiling
- [ ] `src/convert.rs` created with all conversion functions
- [ ] Unit tests written and passing
- [ ] Zero compiler warnings
- [ ] Code documented
- [ ] Commit created: `# 0: PDF ML Phase 1`

---

## Time Budget

**Estimated:** 2-3 days (16-24 hours AI time)

**Breakdown:**
- Copy files: 1 hour
- Review: 0.5 hour
- Create conversions: 4-6 hours
- Write tests: 2-3 hours
- Debug/fix: 2-4 hours
- Documentation: 1-2 hours
- Commit: 0.25 hour

**If stuck:** Review source implementation in ~/docling_debug_pdf_parsing

---

## Common Issues & Solutions

### Issue 1: Type Mismatches

**Problem:** Source types don't match docling-core exactly

**Solution:** Create adapter types if needed, focus on semantic mapping not exact field matching

### Issue 2: Missing DocItemLabel Variants

**Problem:** PDF ML has labels not in docling-core

**Solution:** Map to closest existing label, document discrepancies

### Issue 3: Baseline Loading Errors

**Problem:** baseline.rs depends on specific file paths

**Solution:** This is test infrastructure only, can adapt paths later in test setup

### Issue 4: Complex Nested Structures

**Problem:** PageElement has recursive children

**Solution:** Use recursive conversion (already shown in example), test with simple cases first

---

## After Phase 1 Complete

**Next Phase:** Phase 2 - PDF Reader Component (1-2 days)

Read: MANAGER_PDF_ML_MERGE_DIRECTIVE_2025-11-22.md, Phase 2 section

**Your role:** Continue sequentially through phases, commit after each, maintain 100% test pass rate

---

## Manager Monitoring

Manager will check:
- Phase 1 commit exists with proper format
- Tests passing (X/X shown in commit)
- No warnings
- Code quality
- Timeline on track (2-3 days for Phase 1)

**Report blockers immediately** via commit message or pause work

---

## Quick Reference

**Source repo:** ~/docling_debug_pdf_parsing (N=185)
**Target crate:** ~/docling_rs/crates/docling-pdf-ml
**Branch:** feature/pdf-ml-migration
**Your commit:** # 0 (first worker commit)
**Timeline:** 2-3 days for Phase 1

---

**AUTHORIZATION: BEGIN PHASE 1** ✅

Start with: Copy src/types/data_structures.rs

**Good luck!** 🚀

---

**Generated by:** Manager AI
**For:** Worker AI starting Phase 1
**Date:** 2025-11-22 13:50 PT
