# Worker AI: PDF ML Integration Plan

**Branch:** feature/pdf-ml-migration
**Source:** ~/docling_debug_pdf_parsing (N=185, production-ready)
**Target:** ~/docling_rs/crates/docling-pdf-ml (new crate)
**Timeline:** 5-7 weeks (36-50 days)

---

## READ FIRST: Manager Documents

1. **START_HERE_PDF_MIGRATION.md** - Entry point
2. **MANAGER_REVISED_PDF_MIGRATION_PLAN.md** - Revised plan (post-cleanup)
3. **MANAGER_PDF_MIGRATION_DIRECTIVE.md** - Original directive
4. **PDF_MIGRATION_ON_HOLD.md** - Context on why deferred

---

## Source Repository Status (Current: N=185)

### Quality Metrics
- ✅ **214/214 tests passing** (167 library + 21 comprehensive + 26 orchestrator)
- ✅ **Zero warnings** (clippy + rustdoc)
- ✅ **Performance:** 66ms/page (5.16x faster than Python)
- ✅ **Production ready:** All 4 ML models working

### Code Organization
- **31,419 lines** of clean Rust source code
- **Clear module structure:** pipeline/, models/, ocr/, pipeline_modular/
- **No debug artifacts:** Removed in N=154 cleanup
- **Excellent documentation:** PROJECT_STATUS.md, CLAUDE.md, 143 reports

### What to Migrate

**Core source files (copy directly):**
- src/lib.rs (7,971 lines) - Public API
- src/pipeline/*.rs (8 files) - Main pipeline
- src/pipeline_modular/*.rs (10 files) - Assembly stages
- src/models/*.rs - ML models (Layout, Table, CodeFormula)
- src/ocr/*.rs (6 files) - RapidOCR
- src/preprocessing/*.rs - Image preprocessing
- src/baseline.rs (2,319 lines) - Test baseline loading
- tests/*.rs (167 tests + 21 comprehensive + 26 orchestrator)

**Do NOT migrate:**
- ❌ Python debugging scripts (59k files, not needed)
- ❌ Baseline .npy data (git-ignored, copy separately)
- ❌ Debug binaries (removed in cleanup)
- ❌ Profiling examples (removed in cleanup)

---

## 14-Phase Implementation Plan

### Phase 0: Foundation (2-3 days) ✅ DONE

**Status:** Complete on this branch
**Blocker:** ort 2.0 API migration (2-4 hours to fix)

**What's done:**
- Crate skeleton created
- Git LFS configured
- RapidOCR models copied (15MB)
- Planning documents created

**Before Phase 1:** Fix ort 2.0 if still needed

### Phase 1: Core Types & Baseline Loading (2-3 days)

**Copy:**
```bash
# From ~/docling_debug_pdf_parsing to ~/docling_rs/crates/docling-pdf-ml

cp src/pipeline/data_structures.rs → src/types/data_structures.rs
cp src/baseline.rs → src/baseline.rs
cp src/error.rs → src/error.rs (verify/merge)
```

**Create:**
```rust
// src/convert.rs (~300 lines)
pub fn page_element_to_doc_item(element: &PageElement) -> DocItem;
pub fn page_to_doc_items(page: &Page) -> Vec<DocItem>;
```

**Test:** Unit tests for type conversions

**Commit:** "# 1780: PDF ML Phase 1 - Core types, baseline loading, type conversions"

### Phase 2: PDF Reader Component (1-2 days)

**Extend:** `crates/docling-backend/src/pdf.rs`

**Add functions:**
```rust
pub fn render_page_to_array(page: &PdfPage, dpi: f32) -> Result<Array3<u8>> {
    // Render PDF → RGB ndarray
    // ~200 lines
}

pub fn extract_text_cells_as_simple(page: &PdfPage) -> Result<Vec<SimpleTextCell>> {
    // Convert pdfium TextCell → pipeline SimpleTextCell
    // ~100 lines
}
```

**Test:** Render test PDFs, verify dimensions/RGB values

**Commit:** "# 1781: PDF ML Phase 2 - PDF page rendering and text extraction"

### Phase 3: Preprocessing (1 day)

**Copy:**
```bash
cp -r src/preprocessing/ → src/preprocessing/
```

**Files:**
- preprocess_image.rs
- utils.rs
- mod.rs

**Test:** Port preprocessing tests

**Commit:** "# 1782: PDF ML Phase 3 - Image preprocessing"

### Phase 4: OCR Models (3 days)

**Copy:**
```bash
cp -r src/ocr/ → src/ocr/
```

**Files (6 files, ~3,000 lines):**
- detection.rs (896 lines) - DbNet
- classification.rs (~400 lines) - AngleNet
- recognition.rs (657 lines) - CrnnNet
- types.rs, utils.rs, mod.rs

**Models:** Already copied (RapidOCR, Phase 0)

**Test:** Port OCR tests from tests/test_stage*_ocr_*.rs

**Commit:** "# 1783: PDF ML Phase 4 - RapidOCR implementation (3 models)"

### Phase 5: LayoutPredictor (4-5 days)

**Copy:**
```bash
cp -r src/models/layout_predictor/ → src/models/layout/

# Models (copy from source or HuggingFace)
cp models/layout/*.{pt,onnx,safetensors} → models/layout/
```

**Files (~7,000 lines):**
- onnx.rs (945 lines)
- pytorch_backend/model.rs (1,342 lines)
- pytorch_backend/encoder.rs (1,896 lines)
- pytorch_backend/decoder.rs (1,702 lines)
- pytorch_backend/resnet.rs (625 lines)
- pytorch_backend/weights.rs (611 lines)

**Test:** Port layout tests

**Commit:** "# 1784: PDF ML Phase 5 - LayoutPredictor (PyTorch + ONNX backends)"

### Phase 6: Layout Post-Processing (3 days)

**Copy:**
```bash
cp src/pipeline/layout_postprocessor.rs → src/pipeline/postprocess.rs
```

**Single file:** 1,719 lines
- NMS (non-maximum suppression)
- Confidence filtering
- Label assignment
- Cluster preparation

**Test:** Post-processing tests

**Commit:** "# 1785: PDF ML Phase 6 - Layout post-processing (NMS, filtering)"

### Phase 7: Modular Assembly Pipeline (3-4 days)

**Copy:**
```bash
cp -r src/pipeline_modular/ → src/pipeline/modular/
```

**Files (10 files):**
- orchestrator.rs
- stage04_cell_assigner.rs (Stage 3.0)
- stage05_empty_remover.rs (Stage 3.1)
- stage06_orphan_creator.rs (Stage 3.2)
- stage07_bbox_adjuster.rs (Stage 3.3)
- stage08_overlap_resolver.rs (Stage 3.4)
- stage09_document_assembler.rs (Stage 3.5)
- stage10_reading_order.rs (889 lines)
- types.rs
- mod.rs

**Test:** Port orchestrator tests (26 tests)

**Commit:** "# 1786: PDF ML Phase 7 - Modular assembly pipeline (Stages 3.0-3.5)"

### Phase 8: TableFormer (3 days)

**Copy:**
```bash
cp -r src/models/table_structure/ → src/models/table/
cp src/pipeline/table_inference.rs → src/pipeline/table.rs

# Models
cp models/tableformer/ → models/tableformer/
```

**Files (~3,000 lines):**
- models/table/mod.rs (764 lines)
- models/table/components.rs (1,435 lines)
- models/table/helpers.rs
- pipeline/table.rs (~700 lines)

**Test:** Table structure tests

**Commit:** "# 1787: PDF ML Phase 8 - TableFormer integration"

### Phase 9: Reading Order (2 days)

**Copy:**
```bash
cp src/pipeline/reading_order.rs → src/pipeline/reading_order.rs
```

**Single file:** 1,039 lines
- Spatial graph construction
- Topological sorting
- Multi-column handling

**Note:** Use main implementation (not stage10 version)

**Test:** Reading order tests

**Commit:** "# 1788: PDF ML Phase 9 - Reading order determination"

### Phase 10: Pipeline Executor (3-4 days)

**Copy:**
```bash
cp src/pipeline/executor.rs → src/pipeline/executor.rs
cp src/pipeline/mod.rs → src/pipeline/mod.rs
cp src/lib.rs → src/lib.rs (adapt for docling-pdf-ml API)
```

**Files:**
- executor.rs (2,557 lines - LARGEST file)
- Main orchestration logic
- Stage coordination
- Device management

**Create adapter:**
```rust
// src/adapter.rs (~400 lines)
pub struct PdfMlAdapter;
impl PdfMlAdapter {
    pub fn process_pdf_bytes(&mut self, bytes: &[u8]) -> Result<Document>;
}
```

**Test:** End-to-end integration

**Commit:** "# 1789: PDF ML Phase 10 - Pipeline executor and orchestration"

### Phase 11: Export & DocItem Conversion (2-3 days)

**Copy:**
```bash
cp src/pipeline/docling_export.rs → src/pipeline/export.rs
cp src/docling_document.rs → src/schema.rs
```

**Files:**
- export.rs (754 lines) - Export to DoclingDocument
- schema.rs (9,364 lines) - DocItem schema

**Adapt:** Convert to docling-core DocItems

**Test:** Export tests, serialization validation

**Commit:** "# 1790: PDF ML Phase 11 - Export and DocItem generation"

### Phase 12: Backend Integration & Removal (2-3 days) ⭐ CRITICAL

**DELETE from pdf.rs (~1,000 lines):**
```rust
// Remove ALL heuristic functions:
fn build_markdown() { ... }
fn join_text_fragments() { ... }
fn extract_page_text() { ... }
fn filter_control_characters() { ... }
fn join_hyphenated_word() { ... }
fn append_with_space() { ... }
fn starts_with_bullet() { ... }
fn has_embedded_markers() { ... }
// ... all list/header detection heuristics
```

**REPLACE with ML pipeline (~200 lines):**
```rust
impl DocumentBackend for PdfBackend {
    fn parse_bytes(&self, data: &[u8], options: &BackendOptions) -> Result<Document> {
        // 1. Initialize ML backend (cached)
        let ml_backend = self.get_or_create_ml_backend(options)?;

        // 2. Load PDF
        let pdfium = Self::create_pdfium()?;
        let pdf_doc = pdfium.load_pdf_from_byte_slice(data, None)?;

        // 3. Process pages
        let mut all_doc_items = Vec::new();
        for (idx, page) in pdf_doc.pages().iter().enumerate() {
            let image = Self::render_page_to_array(&page, 150.0)?;
            let cells = Self::extract_text_cells_as_simple(&page)?;

            let page_result = ml_backend.process_page(idx, &image, cells)?;
            let doc_items = docling_pdf_ml::convert::page_to_doc_items(&page_result)?;

            all_doc_items.extend(doc_items);
        }

        // 4. Serialize
        let markdown = docling_core::serialize_markdown(&all_doc_items)?;

        Ok(Document {
            markdown,
            format: InputFormat::Pdf,
            metadata: Self::extract_metadata(&pdf_doc)?,
            content_blocks: Some(all_doc_items),  // Always populated
        })
    }
}
```

**Net change:** -800 lines (delete 1,000, add 200)

**Test:** All canonical PDF tests pass

**Commit:** "# 1791: PDF ML Phase 12 - Replace simple backend with ML pipeline"

### Phase 13: Test Integration (2-3 days)

**Copy all tests:**
```bash
# From ~/docling_debug_pdf_parsing/tests/
# To ~/docling_rs/crates/docling-pdf-ml/tests/

cp tests/*.rs → tests/
```

**Port 214 tests:**
- 167 library tests (unit tests)
- 21 comprehensive tests (test_e2e_*.rs)
- 26 orchestrator tests (test_orchestrator_integration.rs)

**Copy baselines:**
```bash
cp -r baseline_data/ → tests/baselines/
# Git-ignore (several GB)
```

**Verify:** All 214 tests pass in docling-pdf-ml context

**Commit:** "# 1792: PDF ML Phase 13 - Test suite migration (214 tests)"

### Phase 14: Documentation & Polish (2-3 days)

**Rustdoc:**
- Document all public APIs
- Add examples to docstrings

**Examples:**
- Copy 4 production examples
- Adapt for docling-pdf-ml API

**Final validation:**
- Canonical tests (18 PDF tests in docling_rs)
- Performance benchmark
- Memory profiling

**Commit:** "# 1793: PDF ML Phase 14 - Documentation and final validation"

---

## Critical Requirements for Worker

### 1. DELETE Simple Backend (Phase 12)

**From pdf.rs, delete these functions:**
```rust
// Line ~430-1090 (approximately 660 lines of heuristics)
fn build_markdown(pages_lines: &[Vec<TextLine>]) -> String
fn join_text_fragments(pages_lines: &[Vec<TextLine>]) -> Vec<Vec<TextLine>>
fn extract_page_text(page: &PdfPage) -> Result<Vec<TextLine>, DoclingError>
fn filter_control_characters(text: &str) -> String
fn join_hyphenated_word(current: &mut String, next: &str)
fn append_with_space(current: &mut String, text: &str)
fn append_standalone_hyphen(current: &mut String)
fn calculate_average_font_size(pages: &[Vec<TextLine>]) -> f32
fn starts_with_bullet(text: &str) -> bool
fn starts_with_numbered_marker(text: &str) -> bool
fn has_embedded_markers(trimmed: &str) -> bool
fn has_capital_colon_pattern(text: &str) -> bool
fn has_embedded_numbers(trimmed: &str) -> bool
// ... all related helper functions
```

**These are REPLACED by ML models:**
- Header detection → LayoutPredictor (ML-based)
- List detection → LayoutPredictor (ML-based)
- Structure detection → LayoutPredictor (ML-based)
- Text assembly → Assembly pipeline (ML-guided)

**Result:** Clean pdf.rs with ONLY ML pipeline integration

### 2. Maintain 100% Test Pass Rate

**At every phase:**
```bash
cargo test -p docling-pdf-ml
```

**Must pass before committing each phase.**

**If tests fail:**
- Debug and fix before proceeding
- Don't accumulate technical debt
- Don't skip phases

### 3. Copy Files Directly (No Rewrite)

**Source code is production-ready** - don't "improve" it during migration:
- Copy functions as-is
- Preserve comments
- Keep structure
- Only adapt imports/types for docling_rs

**Exception:** Type conversions (PageElement → DocItem) are new code

### 4. Use Source Documentation

**When unclear:**
- Read PROJECT_STATUS.md in source repo
- Read CLAUDE.md for stage enumeration
- Check git log in source repo (N=0-185)
- Reference reports/ directory

**Don't guess** - source has answers

---

## File Migration Map

### Complete Mapping

| Source File | Destination | Lines | Phase |
|-------------|-------------|-------|-------|
| `src/lib.rs` | `src/lib.rs` | 7,971 | 10 |
| `src/error.rs` | `src/error.rs` | 8,275 | 1 |
| `src/baseline.rs` | `src/baseline.rs` | 2,319 | 1 |
| `src/docling_document.rs` | `src/schema.rs` | 9,364 | 11 |
| `src/pipeline/data_structures.rs` | `src/types/data_structures.rs` | 817 | 1 |
| `src/pipeline/executor.rs` | `src/pipeline/executor.rs` | 2,557 | 10 |
| `src/pipeline/layout_postprocessor.rs` | `src/pipeline/postprocess.rs` | 1,719 | 6 |
| `src/pipeline/reading_order.rs` | `src/pipeline/reading_order.rs` | 1,039 | 9 |
| `src/pipeline/docling_export.rs` | `src/pipeline/export.rs` | 754 | 11 |
| `src/pipeline/table_inference.rs` | `src/pipeline/table.rs` | ~700 | 8 |
| `src/pipeline/page_assembly.rs` | `src/pipeline/assembly.rs` | ~600 | 7 |
| `src/pipeline/mod.rs` | `src/pipeline/mod.rs` | ~100 | 10 |
| `src/pipeline_modular/*.rs` (10 files) | `src/pipeline/modular/*.rs` | ~4,000 | 7 |
| `src/ocr/*.rs` (6 files) | `src/ocr/*.rs` | ~3,000 | 4 |
| `src/preprocessing/*.rs` | `src/preprocessing/*.rs` | ~1,500 | 3 |
| `src/models/layout_predictor/*.rs` | `src/models/layout/*.rs` | ~7,000 | 5 |
| `src/models/table_structure/*.rs` | `src/models/table/*.rs` | ~3,000 | 8 |
| `src/models/code_formula/*.rs` | `src/models/code_formula/*.rs` | ~4,000 | Optional |
| `tests/*.rs` (214 tests) | `tests/*.rs` | ~10,000 | 13 |

**Total:** ~59,000 lines (31k source + 10k tests + 9k schema + 9k API)

### New Code to Write

| File | Lines | Purpose |
|------|-------|---------|
| `src/convert.rs` | ~300 | Type conversions to docling-core |
| `src/adapter.rs` | ~400 | Integration adapter |
| `docling-backend/src/pdf.rs` (new functions) | ~300 | PDF rendering, text extraction |
| Test adaptations | ~500 | Adapt tests to docling-pdf-ml |

**Total new code:** ~1,500 lines

---

## Testing Strategy

### Test Categories

**1. Library Tests (167 tests):**
```bash
cargo test --lib -p docling-pdf-ml
```

**Categories:**
- Model loading
- Preprocessing correctness
- OCR pipeline
- Layout detection
- Post-processing stages
- Type conversions

**2. Comprehensive Tests (21 tests):**
```bash
cargo test --test test_e2e -p docling-pdf-ml -- --ignored
```

**End-to-end validation:**
- Process complete PDFs
- Verify DocItem generation
- Compare to baseline outputs

**3. Orchestrator Tests (26 tests):**
```bash
cargo test --test test_orchestrator_integration -p docling-pdf-ml -- --ignored
```

**Multi-page validation:**
- 26 pages across 4 PDFs
- Assembly pipeline validation

**4. Canonical Tests (18 tests):**
```bash
cargo test test_canon_pdf -- --exact
```

**docling_rs existing tests:**
- Integration with rest of framework
- Markdown output validation

### Baseline Data

**Location in source:** `baseline_data/` (git-ignored, ~5GB)

**Migration:**
```bash
# Copy to docling-pdf-ml
cp -r ~/docling_debug_pdf_parsing/baseline_data \
      ~/docling_rs/crates/docling-pdf-ml/tests/baselines/

# Add to .gitignore
echo "crates/docling-pdf-ml/tests/baselines/" >> .gitignore
```

**Loading:** Use copied baseline.rs (loads .npy/.json files)

---

## Model Files Checklist

### ✅ Already Have (Phase 0)

- RapidOCR Detection (4.5MB)
- RapidOCR Recognition (10MB)
- RapidOCR Classification (572KB)

### 📋 Need to Copy

**From source repo:**
```bash
# LayoutPredictor models
cp ~/docling_debug_pdf_parsing/models/layout_predictor/*.{pt,safetensors} \
   crates/docling-pdf-ml/models/layout/

# Or from onnx_exports/
cp ~/docling_debug_pdf_parsing/onnx_exports/*.onnx \
   crates/docling-pdf-ml/models/layout/

# TableFormer models
cp ~/docling_debug_pdf_parsing/models/tableformer/ \
   crates/docling-pdf-ml/models/tableformer/

# CodeFormula (optional)
cp ~/docling_debug_pdf_parsing/models/code_formula/ \
   crates/docling-pdf-ml/models/code_formula/
```

**Git LFS:** All model files tracked (configured in Phase 0)

---

## Dependencies Checklist

### Required in docling-pdf-ml/Cargo.toml

```toml
[dependencies]
# Core
docling-core = { path = "../docling-core" }

# ML (heavy dependencies)
ort = { version = "2.0.0-rc.10", features = ["ndarray"] }
tch = "0.18"
opencv = { version = "0.97", features = ["imgproc"] }
safetensors = "0.6"
tokenizers = "0.20"

# Data
ndarray = { workspace = true }
image = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }

# Geometry (OCR)
geo-clipper = "0.8"
geo = "0.28"
rstar = "0.12"

# Utilities
regex = "1.10"
dirs = "5.0"
log = "0.4"
rustc-hash = "2.0"
ordered-float = "4.2"
bytemuck = "1.14"
half = "2.3"
```

**Environment variables** (from source CLAUDE.md):
```bash
export DYLD_LIBRARY_PATH=/opt/homebrew/lib/python3.14/site-packages/torch/lib
export DYLD_FALLBACK_LIBRARY_PATH=/opt/homebrew/opt/llvm/lib
export LIBTORCH_USE_PYTORCH=1
export LIBTORCH_BYPASS_VERSION_CHECK=1
```

---

## Success Validation Checklist

### Per-Phase Validation

Each phase must achieve:
- [ ] Code compiles without warnings
- [ ] Tests pass (if tests ported in that phase)
- [ ] Imports resolve correctly
- [ ] No regression in existing tests

### Final Validation (Phase 14)

- [ ] **All tests:** 232 passing (214 from migration + 18 canonical)
- [ ] **Performance:** 16 pages/sec on MPS
- [ ] **Quality:** DocItems match source repo baseline
- [ ] **Integration:** Works with docling-backend
- [ ] **Documentation:** All APIs documented
- [ ] **Examples:** 4 examples working
- [ ] **Simple backend:** DELETED from pdf.rs

---

## Known Issues from Source

### Non-Blocking (Document but don't fix)

1. **CodeFormula bbox undersizing** (69% area)
   - Layout model detects smaller regions than ideal
   - Doesn't break functionality
   - Can enhance later

2. **Reading order: Two implementations**
   - `pipeline/reading_order.rs` (production)
   - `pipeline_modular/stage10_reading_order.rs` (testing)
   - Both intentional (see PROJECT_STATUS.md N=155)
   - Use main one, note the other exists

3. **Page dimensions hardcoded fallback**
   - 612x792 default in reading_order.rs
   - Should pass actual dimensions
   - Works correctly, just not perfect

**Action:** Document these, don't fix during migration

---

## Manager Checklist for Worker

### Before Starting

- [ ] Read all MANAGER documents on this branch
- [ ] Review source repo (~/docling_debug_pdf_parsing)
- [ ] Understand source structure (PROJECT_STATUS.md)
- [ ] Check ort 2.0 blocker status

### During Migration (Each Phase)

- [ ] Copy source files as documented
- [ ] Adapt imports/types minimally
- [ ] Port tests for that phase
- [ ] Verify tests pass
- [ ] Commit with # N: prefix
- [ ] Update phase progress

### After Migration

- [ ] All 232 tests passing
- [ ] Simple backend deleted
- [ ] Performance validated
- [ ] Documentation complete
- [ ] Merge to main

---

## Estimated Timeline (Revised)

**Based on cleaned source repository:**

| Week | Phases | Work |
|------|--------|------|
| **Week 1** | 0-2 | Foundation, types, PDF reader |
| **Week 2** | 3-4 | Preprocessing, OCR, start LayoutPredictor |
| **Week 3** | 5-6 | Finish LayoutPredictor, post-processing |
| **Week 4** | 7-8 | Modular pipeline, TableFormer |
| **Week 5** | 9-10 | Reading order, executor |
| **Week 6** | 11-12 | Export, backend integration |
| **Week 7** | 13-14 | Tests, documentation |

**Total: 5-7 weeks** (36-50 days AI time)

**Key milestone:** End of Week 6 (Phase 12) - ML backend integrated, simple backend deleted

---

## Success Criteria (Final)

### Must Achieve

- [ ] 232 tests passing (100%)
- [ ] Performance: 16 pages/sec (MPS)
- [ ] Zero warnings (clippy + rustdoc)
- [ ] DocItems generated (content_blocks always Some)
- [ ] Simple backend DELETED (~1,000 lines removed)
- [ ] All 4 ML models working

### Quality Metrics

- [ ] Outputs match source repo (within ML tolerance)
- [ ] Memory usage acceptable (<2GB per page)
- [ ] Error messages helpful
- [ ] Documentation complete
- [ ] Examples working

---

## Manager Sign-Off

**Analysis complete.** Source repository cleanup makes migration:
- ✅ **18% faster** (36-50 days vs 44-60 days)
- ✅ **Simpler** (no pytest, cleaner code)
- ✅ **Lower risk** (production-ready source, excellent docs)

**Worker AI:** Begin when docling_rs cleanup complete.

**Commit sequence:**
- [MANAGER] commits: Planning and directives (this commit)
- # 1780-1793: Worker implementation (14 phases)

---

**Generated by:** Manager AI
**Date:** 2025-11-22
**Source Analysis:** docling_debug_pdf_parsing N=185 (post-cleanup)
**Status:** Ready for worker implementation
