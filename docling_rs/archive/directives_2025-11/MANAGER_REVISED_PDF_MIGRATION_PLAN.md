# [MANAGER] REVISED PDF ML Migration Plan - Post-Cleanup Analysis

**Date:** 2025-11-22
**Analysis:** docling_debug_pdf_parsing post-cleanup (N=185)
**Status:** Plan revised based on cleaned repository
**Previous Plan:** Created 2025-11-21 (before cleanup)

---

## What Changed: Cleanup Analysis

### Source Repository Status (After N=148-185 Cleanup)

| Metric | Before | After | Change |
|--------|--------|-------|--------|
| **Source code** | ~36,778 lines | 31,419 lines | -14.6% (cleaner!) |
| **Debug binaries** | 31 files | 2 files | -29 removed (N=154) |
| **Examples** | 17 files | 4 files | -13 removed (N=154) |
| **Rust tests** | 167 tests | 167 tests | Maintained |
| **Test status** | 100% | 100% | Maintained |
| **Code quality** | Zero warnings | Zero warnings | Maintained |
| **Performance** | 66ms/page | 66ms/page | Optimized (5.16x Python) |
| **Documentation** | Good | Excellent | Improved |

**Key Improvement:** Repository is now **production-ready and cleaner** - easier to migrate.

### What Was Cleaned (N=154-185)

**Removed (~22,000 lines, 42 files):**
- ✅ **29 debug binaries:** PyTorch validation, stage runners, debug scripts
- ✅ **13 profiling examples:** Removed development-only examples
- ✅ **Dead code:** PositionalEncoding unused code (64 lines)
- ✅ **TODO noise:** Converted 11 TODO comments to descriptive NOTE comments

**Retained (essential):**
- ✅ **2 production binaries:** docling.rs, multi_page_batch.rs
- ✅ **4 user examples:** simple_usage.rs, custom_config.rs, error_handling.rs, profile_full_pipeline.rs
- ✅ **All source code:** Pipeline, models, OCR
- ✅ **All tests:** 167 library + 21 comprehensive + 26 orchestrator = 214 tests

**Result:** Cleaner, more focused codebase - **easier to migrate**.

---

## Key Discovery: No Pytest Infrastructure!

**Important finding:** docling_debug_pdf_parsing does NOT use pytest.

**Test Framework:**
- **Rust tests only:** `tests/*.rs` files (167 library + 26 orchestrator)
- **Comprehensive tests:** 21 end-to-end tests in Rust
- **No pytest:** No conftest.py, no test_stage*.py files
- **Baselines:** Loaded via Rust code (src/baseline.rs), not pytest fixtures

**Impact on Migration Plan:**
- ❌ **Cancel:** "Port pytest infrastructure" (doesn't exist)
- ✅ **Keep:** Port Rust tests (167 library + 21 comprehensive)
- ✅ **Simpler:** No Python test framework to maintain

**This is BETTER than expected** - pure Rust testing, easier to integrate.

---

## Revised Migration Approach

### Previous Plan (Incorrect Assumptions)

**Assumed:**
1. Pytest infrastructure exists → **FALSE** (Rust tests only)
2. ~37k lines to migrate → **WRONG** (31k lines after cleanup)
3. Complex validation framework → **FALSE** (simple Rust tests)

### Revised Plan (Based on Reality)

**Actually have:**
1. ✅ **31,419 lines of clean Rust code** (14% less than estimated)
2. ✅ **167 unit tests in Rust** (tests/*.rs)
3. ✅ **Baseline loading in Rust** (src/baseline.rs)
4. ✅ **Well-documented modules** (pipeline/, models/, ocr/)
5. ✅ **Production-ready** (100% test pass rate, zero warnings)

**Migration is SIMPLER:**
- No pytest to port (pure Rust tests)
- Cleaner codebase (fewer lines)
- Better documentation
- Clear module boundaries

---

## Updated Phase Plan (Simplified)

### PHASE 0: Foundation (2-3 days) ✅ DONE

**Completed:**
- Planning documents created
- Crate skeleton (docling-pdf-ml)
- Git LFS configured
- Models copied (RapidOCR, 15MB)

**Blocker:** ort 2.0 API migration in docling-ocr (2-4 hours)

### PHASE 1: Core Types (2-3 days) ⬇️ FASTER

**Copy from docling_debug_pdf_parsing:**
- `src/pipeline/data_structures.rs` (817 lines) → `docling-pdf-ml/src/types.rs`
- `src/error.rs` (8,275 lines) → Already done (minimal wrapper)
- `src/baseline.rs` (2,319 lines) → `docling-pdf-ml/src/baseline.rs`

**Create type conversions:**
```rust
// docling-pdf-ml/src/convert.rs
pub fn page_element_to_doc_item(element: &PageElement) -> DocItem;
pub fn page_to_doc_items(page: &Page) -> Vec<DocItem>;
```

**Test:** Unit tests for conversions

**No pytest needed** - simpler than planned!

### PHASE 2: Preprocessing (1 day) ⬇️ UNCHANGED

**Copy:** `src/preprocessing/` directory
- Clean structure (already modular)

### PHASE 3: OCR Models (3-4 days) ⬇️ FASTER

**Copy:** `src/ocr/` directory (6 files)
- detection.rs (896 lines)
- classification.rs
- recognition.rs (657 lines)
- Already modular and clean

**Models:** RapidOCR already copied (Phase 0)

**Tests:** Port from tests/test_*.rs files (not pytest)

### PHASE 4: LayoutPredictor (4-5 days) ⬇️ UNCHANGED

**Copy:** `src/models/layout_predictor/` directory
- onnx.rs (945 lines)
- pytorch_backend/ (5 files: ~6,000 lines total)

**Largest files:**
- decoder.rs (1,702 lines)
- encoder.rs (1,896 lines)
- model.rs (1,342 lines)

### PHASE 5: Post-Processing & Assembly (3-4 days) ⬇️ FASTER

**Copy:**
- `src/pipeline/layout_postprocessor.rs` (1,719 lines)
- `src/pipeline/page_assembly.rs` (small file)
- `src/pipeline_modular/` directory (10 files, well-organized)

**Cleaner than expected** - good module boundaries

### PHASE 6: TableFormer (3-4 days) ⬇️ UNCHANGED

**Copy:**
- `src/models/table_structure/` (3 files)
- `src/pipeline/table_inference.rs`

### PHASE 7: Reading Order (2-3 days) ⬇️ UNCHANGED

**Copy:**
- `src/pipeline/reading_order.rs` (1,039 lines)
- `src/pipeline_modular/stage10_reading_order.rs` (889 lines)
- **Note:** Two implementations (see PROJECT_STATUS.md N=155)

### PHASE 8: Pipeline Orchestration (3-4 days) ⬇️ FASTER

**Copy:**
- `src/pipeline/executor.rs` (2,557 lines - largest file)
- `src/pipeline/mod.rs`
- `src/pipeline_modular/orchestrator.rs`

**Cleaner than expected** - good separation

### PHASE 9: Export & Serialization (2-3 days) ⬇️ UNCHANGED

**Copy:**
- `src/pipeline/docling_export.rs` (754 lines)
- `src/docling_document.rs` (9,364 lines) - DocItem schema

**Then convert to docling-core DocItems**

### PHASE 10: Testing Integration (2-3 days) ⬇️ FASTER

**Port Rust tests:**
- 167 library tests from tests/*.rs
- 21 comprehensive tests
- 26 orchestrator tests

**Total: 214 tests** (not 253 - original plan overestimated)

**No pytest to port** - simpler!

### PHASE 11: Backend Integration & Removal (2-3 days) ⬇️ UNCHANGED

**DELETE simple PDF backend:**
- Remove ~1,000 lines of heuristic code from pdf.rs
- Replace with ML pipeline calls

### PHASE 12: Performance Validation (1-2 days) ⬇️ FASTER

**Verify:**
- 16 pages/sec on MPS (baseline)
- All 214 tests pass
- Benchmark comparison

**Already optimized in source** - just validate

### PHASE 13: Documentation (2-3 days) ⬇️ UNCHANGED

**Rustdoc, examples, guides**

### PHASE 14: Final Integration (1-2 days) ⬇️ FASTER

**Canonical tests, final validation**

---

## REVISED TIMELINE

| Metric | Original Estimate | Revised Estimate | Change |
|--------|------------------|------------------|--------|
| **Phase 0** | 2-3 days | 2-3 days | Same (done) |
| **Phases 1-14** | 44-60 days | **36-50 days** | ⬇️ **-18% faster** |
| **Total** | 46-63 days | **38-53 days** | ⬇️ **5-7 weeks** (was 6.5-9) |

**Reason for speedup:**
- Cleaner codebase (-14.6% lines)
- No pytest to port (pure Rust)
- Better module boundaries
- Excellent documentation
- Fewer tests than estimated (214 not 253)

---

## Critical Revisions to Original Plan

### ❌ REMOVED: Pytest Infrastructure

**Original plan said:**
> "Port pytest infrastructure completely"

**Reality:**
- No pytest in docling_debug_pdf_parsing
- All tests are Rust (`tests/*.rs`)
- Baseline loading is Rust code (`src/baseline.rs`)

**Action:** Remove all pytest references from plan

### ✅ CONFIRMED: Delete Simple Backend

**User directive:** "remove it" (no fallback)

**Implementation:**
```rust
// Phase 11: Delete heuristic code from pdf.rs

// DELETE these functions (no fallback):
// - build_markdown()
// - join_text_fragments()
// - extract_page_text()
// - All list detection heuristics
// - All header detection heuristics

// REPLACE with:
impl DocumentBackend for PdfBackend {
    fn parse_bytes(&self, data: &[u8], options: &BackendOptions) -> Result<Document> {
        // ONLY ML pipeline
        let ml_backend = docling_pdf_ml::PdfMlBackend::new(config)?;
        ml_backend.process_pdf_bytes(data, options)
    }
}
```

### ✅ SIMPLIFIED: Test Migration

**Original:** 253 tests (overestimated)
**Actual:** 214 tests to port:
- 167 library tests (unit tests)
- 21 comprehensive tests (end-to-end)
- 26 orchestrator tests (integration)

**Plus 18 existing canonical tests in docling_rs = 232 total (not 253)**

---

## Updated Component Inventory

### Core Source Files to Migrate (31,419 lines)

**Pipeline Components:**
| File | Lines | Priority |
|------|-------|----------|
| `pipeline/executor.rs` | 2,557 | 🔴 Critical |
| `pipeline/layout_postprocessor.rs` | 1,719 | 🔴 Critical |
| `pipeline/reading_order.rs` | 1,039 | 🟡 Medium |
| `pipeline/docling_export.rs` | 754 | 🔴 Critical |
| `pipeline/data_structures.rs` | 817 | 🔴 Critical |
| `pipeline/page_assembly.rs` | ~600 | 🟡 Medium |
| `pipeline/table_inference.rs` | ~700 | 🟡 Medium |

**Modular Pipeline (10 files):**
| File | Lines | Priority |
|------|-------|----------|
| `pipeline_modular/stage10_reading_order.rs` | 889 | 🟡 Medium |
| `pipeline_modular/stage04-09_*.rs` | ~2,000 | 🔴 Critical |
| `pipeline_modular/orchestrator.rs` | ~500 | 🔴 Critical |

**ML Models:**
| Model | Lines | Priority |
|-------|-------|----------|
| `models/layout_predictor/pytorch_backend/` | ~6,000 | 🔴 Critical |
| `models/layout_predictor/onnx.rs` | 945 | 🔴 Critical |
| `models/table_structure/` | ~3,000 | 🔴 Critical |
| `models/code_formula/` | ~4,000 | 🟢 Optional |

**OCR (6 files):**
| File | Lines | Priority |
|------|-------|----------|
| `ocr/detection.rs` | 896 | 🔴 Critical |
| `ocr/recognition.rs` | 657 | 🔴 Critical |
| `ocr/classification.rs` | ~400 | 🔴 Critical |
| `ocr/utils.rs`, `ocr/types.rs`, `ocr/mod.rs` | ~500 | 🔴 Critical |

**Support Files:**
| File | Lines | Purpose |
|------|-------|---------|
| `lib.rs` | 7,971 | Public API |
| `docling_document.rs` | 9,364 | DocItem schema |
| `error.rs` | 8,275 | Error types |
| `baseline.rs` | 2,319 | Test baseline loading |
| `preprocessing/` | ~1,500 | Image preprocessing |

---

## REVISED Success Criteria

### ✅ Updated Test Target: 232 Tests (not 253)

| Test Type | Count | Source |
|-----------|-------|--------|
| **Library tests** | 167 | tests/*.rs (unit tests) |
| **Comprehensive tests** | 21 | tests/test_e2e_*.rs |
| **Orchestrator tests** | 26 | tests/test_orchestrator_integration.rs |
| **Canonical tests** | 18 | docling_rs (existing) |
| **Total** | **232** | |

**Acceptance:** All 232 tests pass (100%)

### ✅ Performance Target

- **MPS:** 16 pages/sec (66ms/page baseline achieved)
- **CUDA:** 50-60 pages/sec (estimated)
- **CPU:** 4.5 pages/sec (scanned docs with OCR)

### ✅ Code Quality

- Zero clippy warnings (already achieved in source)
- Zero rustdoc warnings (already achieved in source)
- 100% test pass rate

---

## Updated Migration Strategy

### Simplifications

**1. No Pytest Infrastructure**
- ✅ **Simpler:** Pure Rust testing
- ✅ **Faster:** No Python test framework
- ✅ **Cleaner:** One test framework (cargo test)

**2. Cleaner Codebase**
- ✅ **14% fewer lines** to migrate
- ✅ **No debug artifacts** to filter out
- ✅ **Better organized** modules

**3. Better Documentation**
- ✅ **PROJECT_STATUS.md:** Complete overview
- ✅ **CLAUDE.md:** Stage enumeration
- ✅ **143 docs:** Comprehensive reports

### What This Means for Worker

**Easier migration:**
- Copy files directly (no cleanup needed in source)
- Port Rust tests (no pytest conversion)
- Clear module boundaries
- Excellent documentation to reference

**Faster timeline:**
- 5-7 weeks (was 6.5-9 weeks)
- 36-50 days (was 44-60 days)

---

## Revised Phase Breakdown

### PHASE 0: Foundation ✅ DONE (2-3 days)

**Status:** Complete (on-hold branch)
**Blocker:** ort 2.0 migration (2-4 hours to fix)

### PHASE 1: Core Types & Baseline Loading (2-3 days) ⬇️ FASTER

**Copy (no changes needed):**
1. `src/pipeline/data_structures.rs` → `src/types/data_structures.rs`
2. `src/baseline.rs` → `src/baseline.rs` (for test baseline loading)
3. `src/error.rs` → Already done (wrapped in Phase 0)

**Create:**
1. `src/convert.rs` - Type conversions to docling-core DocItems

**Test:** Unit tests for type conversions

**Estimated:** 2-3 days (was 3-4)

### PHASE 2: PDF Reader Integration (1-2 days) ⬇️ UNCHANGED

**Extend docling-backend/src/pdf.rs:**
```rust
pub fn render_page_to_array(page: &PdfPage, dpi: f32) -> Result<Array3<u8>>;
pub fn extract_text_cells_as_simple(page: &PdfPage) -> Result<Vec<SimpleTextCell>>;
```

**Convert pdfium text cells → pipeline SimpleTextCell format**

### PHASE 3: Preprocessing (1 day) ⬇️ FASTER

**Copy:** `src/preprocessing/` directory (clean, ~1,500 lines)

**Files:**
- preprocessing/preprocess_image.rs
- preprocessing/utils.rs
- preprocessing/mod.rs

**Simple, well-organized** - fast to migrate

### PHASE 4: OCR Models (3 days) ⬇️ FASTER

**Copy:** `src/ocr/` directory (6 files, ~3,000 lines)

**Already have models** (copied in Phase 0)

**Tests:** Port OCR tests from tests/test_stage*_ocr_*.rs

### PHASE 5: LayoutPredictor (4-5 days) ⬇️ UNCHANGED

**Copy:** `src/models/layout_predictor/` (largest component, ~7,000 lines)

**Both backends:**
- ONNX backend (945 lines)
- PyTorch backend (6 files, ~6,000 lines total)

**Tests:** Port layout tests

### PHASE 6: Layout Post-Processing (3 days) ⬇️ FASTER

**Copy:**
- `src/pipeline/layout_postprocessor.rs` (1,719 lines)
- NMS, filtering, confidence thresholding

**Well-contained** - single file

### PHASE 7: Modular Assembly Pipeline (3-4 days) ⬇️ UNCHANGED

**Copy:** `src/pipeline_modular/` directory (10 files)

**Stages 3.0-3.5:**
- stage04_cell_assigner.rs
- stage05_empty_remover.rs
- stage06_orphan_creator.rs
- stage07_bbox_adjuster.rs
- stage08_overlap_resolver.rs
- stage09_document_assembler.rs

**Tests:** Port orchestrator tests (26 tests)

### PHASE 8: TableFormer (3 days) ⬇️ FASTER

**Copy:**
- `src/models/table_structure/` (3 files, ~3,000 lines)
- `src/pipeline/table_inference.rs`

### PHASE 9: Reading Order (2 days) ⬇️ FASTER

**Copy:**
- `src/pipeline/reading_order.rs` (production version)
- Note: Two implementations exist (see N=155), use main one

### PHASE 10: Pipeline Executor & Integration (3-4 days) ⬇️ UNCHANGED

**Copy:**
- `src/pipeline/executor.rs` (2,557 lines - largest file)
- `src/lib.rs` (7,971 lines - public API)

**Create adapter for docling-backend**

### PHASE 11: Export & DocItem Conversion (2-3 days) ⬇️ UNCHANGED

**Copy:**
- `src/pipeline/docling_export.rs` (754 lines)
- `src/docling_document.rs` (9,364 lines - schema)

**Map to docling-core DocItems**

### PHASE 12: Backend Integration (2-3 days) ⬇️ FASTER

**Delete simple backend from pdf.rs** (~1,000 lines removed)

**Replace with ML pipeline calls** (~200 lines new code)

**Net: -800 lines** (cleaner!)

### PHASE 13: Test Integration (2-3 days) ⬇️ FASTER

**Port 214 tests:**
- Copy tests/*.rs files
- Adapt to docling-pdf-ml structure
- Port baselines (git-ignored, ~5GB)

**No pytest** - pure Rust tests only

### PHASE 14: Documentation & Polish (2-3 days) ⬇️ UNCHANGED

**Rustdoc, examples, final validation**

---

## CodeFormula (Optional Model)

**Status in source:** Production-ready (N=318-345)

**Decision for migration:**
- ✅ **Copy infrastructure** (models/code_formula/, 4 files, ~4,000 lines)
- ✅ **Make optional** (feature flag: `code-formula`)
- ⏸️ **Don't require for Phase 1-13** (can add in Phase 15 if desired)

**Rationale:**
- CodeFormula is ~4,000 lines + large model (~200MB)
- Only enhances code/formula elements (not required for base functionality)
- Can defer to post-migration enhancement

**Revised plan:** Include in migration but as optional feature

---

## REVISED TIMELINE

### By Phase

| Phase | Original | Revised | Reason |
|-------|----------|---------|--------|
| 0 | 2-3 days | 2-3 days | Same |
| 1 | 3-4 days | 2-3 days | No pytest, cleaner types |
| 2 | 1-2 days | 1-2 days | Same |
| 3 | 4-5 days | 3 days | Cleaner OCR code |
| 4 | 5-6 days | 4-5 days | Same |
| 5 | 4-5 days | 3 days | Single file postprocessor |
| 6 | 4-5 days | 3-4 days | Same |
| 7 | 3-4 days | 3 days | Cleaner code |
| 8 | 4-5 days | 2 days | No pytest |
| 9 | 2-3 days | 2-3 days | Same |
| 10 | 2-3 days | 2-3 days | Same |
| 11 | 3-4 days | 2-3 days | Simpler removal |
| 12 | 3-4 days | 2-3 days | Fewer tests |
| 13 | 2-3 days | 2-3 days | Same |
| 14 | 3-5 days | 1-2 days | Less validation |

**Total:** 44-60 days → **36-50 days** (18% reduction)

---

## What Source Repository Provides

### ✅ Production-Ready Artifacts

**Quality:**
- ✅ 167/167 library tests passing
- ✅ 21/21 comprehensive tests passing
- ✅ 26/26 orchestrator tests passing
- ✅ Zero clippy warnings
- ✅ Zero rustdoc warnings
- ✅ Performance optimized (5.16x faster than Python)

**Organization:**
- ✅ Clean module structure (pipeline/, models/, ocr/)
- ✅ Well-documented (143 .md files)
- ✅ Clear separation of concerns
- ✅ Minimal dependencies
- ✅ No debug artifacts (removed in N=154)

**Completeness:**
- ✅ All 4 ML models (RapidOCR, Layout, Table, CodeFormula)
- ✅ Dual backends (PyTorch + ONNX)
- ✅ Full pipeline (Stages 0-9, 12)
- ✅ Export to JSON/Markdown
- ✅ Error handling
- ✅ Configuration system

### What to Migrate vs. Recreate

**COPY directly (minimal changes):**
- ✅ All src/ code (31k lines)
- ✅ All tests/*.rs (167 tests)
- ✅ All models/ (ONNX files)
- ✅ Key examples (4 files)

**ADAPT for docling_rs:**
- Type conversions (PageElement → DocItem)
- Integration with docling-backend
- Use docling-core serialization
- Follow docling_rs conventions

**DON'T migrate:**
- ❌ Debug scripts (~59k Python files)
- ❌ Baseline data (git-ignored, can regenerate)
- ❌ Debug binaries (removed)
- ❌ Profiling examples (removed)

---

## Worker Instructions (Updated)

### IMMEDIATE: Fix ort 2.0 Blocker (2-4 hours)

See `FIX_DOCLING_OCR_ORT2.md` (if exists on branch, else recreate from original plan)

### Phase 1-14: Follow Revised Timeline

**Each phase:**
1. Read source file(s) from ~/docling_debug_pdf_parsing
2. Copy to crates/docling-pdf-ml/src/
3. Adapt as needed (type system, imports)
4. Port tests
5. Verify tests pass
6. Commit with # N: prefix

**Test at every phase:**
```bash
cargo test -p docling-pdf-ml
```

**Maintain 100% pass rate** throughout migration

### Final Phase: Integration & Removal

**Phase 11 critical action:**
```rust
// DELETE from pdf.rs (lines 430-1090):
fn build_markdown()
fn join_text_fragments()
fn extract_page_text()
fn filter_control_characters()
// ... all heuristic functions

// REPLACE with ML pipeline integration:
fn parse_bytes(&self, data: &[u8], options: &BackendOptions) -> Result<Document> {
    let ml_backend = self.get_or_create_ml_backend(options)?;
    // Load PDF, render pages, run ML, return DocItems
}
```

**~800 lines net removal** (delete 1,000, add 200)

---

## Source Code Map for Worker

### File Migration Matrix

| Source (docling_debug_pdf_parsing) | Destination (docling-pdf-ml) | Phase | Lines |
|-------------------------------------|------------------------------|-------|-------|
| `src/lib.rs` | `src/lib.rs` | 10 | 7,971 |
| `src/error.rs` | `src/error.rs` | 1 | ✅ Done |
| `src/baseline.rs` | `src/baseline.rs` | 1 | 2,319 |
| `src/pipeline/data_structures.rs` | `src/types.rs` | 1 | 817 |
| `src/preprocessing/*` | `src/preprocessing/*` | 3 | ~1,500 |
| `src/ocr/*` | `src/ocr/*` | 4 | ~3,000 |
| `src/models/layout_predictor/*` | `src/models/layout/*` | 5 | ~7,000 |
| `src/pipeline/layout_postprocessor.rs` | `src/pipeline/postprocess.rs` | 6 | 1,719 |
| `src/pipeline_modular/*` | `src/pipeline/modular/*` | 7 | ~4,000 |
| `src/models/table_structure/*` | `src/models/table/*` | 8 | ~3,000 |
| `src/pipeline/reading_order.rs` | `src/pipeline/reading_order.rs` | 9 | 1,039 |
| `src/pipeline/executor.rs` | `src/pipeline/executor.rs` | 10 | 2,557 |
| `src/pipeline/docling_export.rs` | `src/pipeline/export.rs` | 11 | 754 |
| `src/docling_document.rs` | `src/schema.rs` | 11 | 9,364 |
| `tests/*.rs` (167 tests) | `tests/*.rs` | 13 | ~10,000 |

**Total:** 31,419 lines source + ~2,000 lines adaptation = **~33,000 lines**

---

## Baseline Data Strategy

### What Baselines Exist

**In source repo:**
- `baseline_data/` (git-ignored, several GB)
- 4,859 .npy files (numpy arrays)
- Stage-by-stage outputs for validation

### Migration Strategy

**Option A: Copy baselines**
```bash
# Copy from source repo
cp -r ~/docling_debug_pdf_parsing/baseline_data \
      ~/docling_rs/crates/docling-pdf-ml/tests/baselines/

# Add to .gitignore
echo "crates/docling-pdf-ml/tests/baselines/" >> .gitignore
```

**Option B: Regenerate baselines**
- Use source repo to generate fresh baselines
- Ensures consistency with migration

**Recommendation:** Option A (copy) - baselines are validated, no need to regenerate

### Baseline Loading Code

**Source has Rust baseline loader:**
- `src/baseline.rs` (2,319 lines)
- Loads .npy files via ndarray-npy
- Loads .json files via serde_json

**Migration:** Copy baseline.rs to docling-pdf-ml (Phase 1)

---

## Model Files Status

### Already Copied (Phase 0)

- ✅ RapidOCR models (15MB): detection, classification, recognition

### Still Need to Copy

| Model | Size | Source Location | Priority |
|-------|------|-----------------|----------|
| **LayoutPredictor (PyTorch)** | ~50MB | models/ or HuggingFace | 🔴 Critical |
| **LayoutPredictor (ONNX)** | ~50MB | onnx_exports/ | 🔴 Critical |
| **TableFormer** | ~80MB | models/tableformer/ | 🔴 Critical |
| **CodeFormula** | ~200MB | models/code_formula/ | 🟢 Optional |

**Phase 4-5 task:** Copy remaining model files, configure Git LFS

---

## Integration Points (Unchanged from Original Plan)

### 1. Type System Bridge

```rust
// docling-pdf-ml/src/convert.rs

use docling_core::DocItem;
use crate::pipeline::{PageElement, ElementType};

pub fn page_element_to_doc_item(element: &PageElement) -> DocItem {
    // Convert internal PageElement → docling-core DocItem
}

pub fn page_to_doc_items(page: &Page) -> Vec<DocItem> {
    page.elements.iter().map(page_element_to_doc_item).collect()
}
```

### 2. Backend Integration

```rust
// docling-backend/src/pdf.rs (Phase 12)

impl DocumentBackend for PdfBackend {
    fn parse_bytes(&self, data: &[u8], options: &BackendOptions) -> Result<Document> {
        // Initialize ML backend (cached)
        let ml_backend = self.get_or_create_ml_backend()?;

        // Load & render PDF
        let pages = self.load_and_render_pdf(data)?;

        // Process through ML
        let doc_items = ml_backend.process_pages(&pages)?;

        // Serialize
        let markdown = docling_core::serialize_markdown(&doc_items)?;

        Ok(Document {
            markdown,
            format: InputFormat::Pdf,
            metadata: self.extract_metadata(data)?,
            content_blocks: Some(doc_items),
        })
    }
}
```

### 3. Testing Integration

```rust
// docling-pdf-ml/tests/*.rs (copied from source)

#[test]
fn test_preprocessing() {
    let baseline = load_baseline("stage01", "arxiv", 0);
    let result = preprocess_image(&input);
    assert_arrays_close(&result, &baseline, 1e-5);
}

#[test]
fn test_layout_detection() {
    let baseline = load_baseline("stage18", "arxiv", 0);
    let result = layout_model.predict(&input);
    assert_clusters_match(&result, &baseline);
}

// Port all 167 tests similarly
```

---

## Risks (Updated)

### ⬇️ REDUCED RISKS

| Risk | Was | Now | Reason |
|------|-----|-----|--------|
| **Code complexity** | High | Medium | 14% fewer lines, cleaner |
| **Test framework** | High | Low | No pytest, pure Rust |
| **Documentation gaps** | Medium | Low | 143 docs, excellent |
| **Module boundaries** | Medium | Low | Clear separation |

### ⬆️ SAME RISKS

| Risk | Level | Mitigation |
|------|-------|-----------|
| **Type system mapping** | Medium | Phase 1: Careful conversions |
| **Heavy dependencies** | High | Accepted (2.3GB optional) |
| **Performance regression** | Low | Profile regularly |
| **Platform support** | Medium | Test on CI |

### ⬆️ NEW RISKS (From Cleanup Analysis)

| Risk | Level | Mitigation |
|------|-------|-----------|
| **Two reading order implementations** | Low | Use main one (pipeline/reading_order.rs) |
| **Dual pipeline systems** | Low | Both needed (executor + modular), keep both |
| **Large files** | Low | executor.rs (2.5k), docling_document.rs (9.3k) are manageable |

---

## Dependencies (Unchanged)

**Required for docling-pdf-ml:**
```toml
ort = "2.0.0-rc.10"          # ONNX Runtime (~200MB)
tch = "0.18"                 # PyTorch (~2GB with libtorch)
opencv = "0.97"              # OpenCV (~100MB)
geo-clipper = "0.8"          # Polygon operations
geo = "0.28"
ndarray = "0.15"
image = "0.25"
tokenizers = "0.20"          # For CodeFormula
safetensors = "0.6"
```

**Total:** ~2.3GB (optional dependency)

---

## Revised Success Metrics

### Must Achieve

- [ ] **232 tests passing** (167 library + 21 comprehensive + 26 orchestrator + 18 canonical)
- [ ] **Zero warnings** (clippy + rustdoc)
- [ ] **Performance: 16 pages/sec** on MPS (match source baseline)
- [ ] **DocItems generated** (content_blocks always Some)
- [ ] **Simple backend DELETED** (no heuristic code remains)
- [ ] **All 4 ML models working** (RapidOCR, Layout, Table, CodeFormula optional)

### Code Volume

- **Source to copy:** 31,419 lines (clean, no debug artifacts)
- **New adapter code:** ~2,000 lines (type conversions, integration)
- **Net change to pdf.rs:** -800 lines (delete heuristics, add ML calls)
- **Total migration:** ~33,000 lines

---

## Manager Recommendations

### For Worker AI

**1. Faster migration than originally planned**
- Source is cleaner (14% less code)
- No pytest complexity
- Better documentation
- Clear module boundaries

**Revised estimate: 5-7 weeks** (was 6.5-9 weeks)

**2. Follow phase sequence strictly**
- Don't skip phases
- Test at every phase
- Maintain 100% pass rate

**3. Use source documentation**
- PROJECT_STATUS.md has complete overview
- CLAUDE.md has stage enumeration
- Each module well-documented

**4. Baseline data handling**
- Copy baselines from source (don't regenerate)
- Git-ignore (several GB)
- Load via baseline.rs (copy from source)

**5. Delete simple backend in Phase 12**
- Don't delay, don't keep as fallback
- ML only, clear error if models missing

---

## Files for Worker to Read

**On this branch:**
1. START_HERE_PDF_MIGRATION.md - Entry point
2. MANAGER_PDF_MIGRATION_DIRECTIVE.md - Original directive
3. **THIS FILE** - Revised plan based on cleanup

**To recreate (if not on branch):**
- PDF_MIGRATION_EXECUTIVE_SUMMARY.md
- PDF_PARSING_MIGRATION_PLAN.md
- PDF_PARSING_TECHNICAL_ARCHITECTURE.md
- PDF_PARSING_GAPS_AND_COMPONENTS.md

**In source repo:**
- ~/docling_debug_pdf_parsing/PROJECT_STATUS.md
- ~/docling_debug_pdf_parsing/CLAUDE.md

---

## Manager Sign-Off

**Analysis complete.** Source repository cleanup makes migration:
- ✅ **Simpler** (no pytest, cleaner code)
- ✅ **Faster** (36-50 days vs 44-60 days)
- ✅ **Lower risk** (better documentation, clear boundaries)

**Ready for worker implementation** when repository cleanup complete in docling_rs.

**Next:** Worker AI begins Phase 0 (after ort 2.0 fix), follows revised 14-phase plan.

---

**Generated by:** Manager AI
**Date:** 2025-11-22
**Context:** Post-cleanup analysis of docling_debug_pdf_parsing (N=185)
