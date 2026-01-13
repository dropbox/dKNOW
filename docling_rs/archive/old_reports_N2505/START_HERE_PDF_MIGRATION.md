# PDF ML Migration - START HERE

**Branch:** feature/pdf-ml-migration
**Status:** Ready for implementation
**Prepared:** 2025-11-21

---

## For Worker AI: Read These Documents in Order

### 1. MANAGER_PDF_MIGRATION_DIRECTIVE.md
**Purpose:** High-level directives and requirements
**Key points:**
- REMOVE simple PDF backend (no fallback)
- Full migration (all 5 ML models)
- Port pytest infrastructure
- Target: 253 tests, 100% pass rate

### 2. PDF_MIGRATION_EXECUTIVE_SUMMARY.md
**Purpose:** Strategy, timeline, decisions
**Key points:**
- 14-phase plan (6.5-9 weeks)
- Architecture: Separate docling-pdf-ml crate
- Git LFS for models
- Risk assessment

### 3. PDF_PARSING_MIGRATION_PLAN.md
**Purpose:** Detailed phase-by-phase implementation plan
**Key points:**
- Phase 0: Foundation (2-3 days)
- Phase 1-14: Implementation (44-60 days)
- Acceptance criteria for each phase
- Testing strategy

### 4. PDF_PARSING_TECHNICAL_ARCHITECTURE.md
**Purpose:** Component wiring and integration details
**Key points:**
- Data flow diagrams
- API designs
- Type conversions
- Integration points

### 5. PDF_PARSING_GAPS_AND_COMPONENTS.md
**Purpose:** Missing components and implementation details
**Key points:**
- 8 components to build
- Code volume estimates
- Dependencies and environment setup

### 6. PDF_MIGRATION_ON_HOLD.md
**Purpose:** Why deferred, when to resume
**Context:** Migration planned but deferred for repository cleanup

---

## Source Repository

**Location:** `~/docling_debug_pdf_parsing`
**Status:** Production-ready (214/214 tests passing, 16 pages/sec)
**Size:** 30GB (with models, baselines, test data)

**What to migrate:**
- ~36,778 lines of Rust code
- 5 ML models (RapidOCR, LayoutPredictor, TableFormer, CodeFormula)
- 214 unit tests
- 21 comprehensive end-to-end tests
- Pytest stage-by-stage validation infrastructure
- Baseline data (git-ignored, several GB)

---

## Target: docling_rs

**Branch:** feature/pdf-ml-migration (this branch)
**Structure:** Create `crates/docling-pdf-ml/` (new crate)
**Integration:** Wire into `crates/docling-backend/src/pdf.rs`

---

## First Steps for Worker

1. **Fix ort 2.0 blocker** (if still exists)
   - Check if ort 1.16 still yanked: `cargo check -p docling-ocr`
   - If yes: See `FIX_DOCLING_OCR_ORT2.md` (may need to recreate)
   - Estimate: 2-4 hours

2. **Begin Phase 0: Foundation**
   - Initialize Git LFS
   - Create `crates/docling-pdf-ml/` skeleton
   - Copy RapidOCR models (~15MB)
   - Set up pytest infrastructure
   - Estimate: 2-3 days

3. **Execute Phases 1-14**
   - Follow `PDF_PARSING_MIGRATION_PLAN.md`
   - Test at every phase (maintain 100%)
   - Commit after each phase: # 1780, # 1781, ...

---

## Critical Reminders

⚠️ **REMOVE simple backend** - Do not keep as fallback
⚠️ **Full migration** - All 5 models, all tests
⚠️ **Git LFS** - Track all model files
⚠️ **100% pass rate** - All 253 tests must pass

---

## Timeline

- Phase 0: 2-3 days (+ ort fix if needed)
- Phases 1-14: 44-60 days
- **Total: 46-63 days (6.5-9 weeks AI time)**

---

## Success Criteria

- [ ] 5 ML models integrated (RapidOCR, LayoutPredictor, TableFormer, CodeFormula, Reading Order)
- [ ] 214 Rust unit tests passing
- [ ] 21 pytest comprehensive tests passing
- [ ] 18 canonical PDF tests passing
- [ ] PDF generates DocItems (content_blocks always Some)
- [ ] Simple PDF backend code DELETED
- [ ] Performance: 16 pages/sec on MPS

---

**This branch is ready for Worker AI to begin implementation.**
