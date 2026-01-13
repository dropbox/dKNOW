# PDF ML Migration - Ready for Execution

**Date:** 2025-11-22
**Manager:** Planning complete
**Branch:** feature/pdf-ml-migration
**Status:** Awaiting user approval to begin worker execution

---

## Executive Summary

Complete migration plan created for integrating ML-based PDF parsing from `docling_debug_pdf_parsing` (production-ready, N=185) into `docling_rs`.

### Source Repository (Analyzed)

**Location:** ~/docling_debug_pdf_parsing
**Status:** Production-ready (N=185, post-cleanup)
**Quality:**
- 214/214 tests passing (100%)
- 31,419 lines of clean code (14% reduction from cleanup)
- Zero warnings
- 66ms/page performance (5.16x Python)
- All 4 ML models validated

### Target Repository (This Repo)

**Branch:** feature/pdf-ml-migration (separate from main)
**Contains:** Complete migration planning documents
**Ready for:** Worker AI execution

---

## Migration Plan Summary

### Timeline

**Revised estimate:** 5-7 weeks (36-50 days AI time)
- 18% faster than initial plan
- Cleaner source code
- No pytest complexity
- Better documentation

### Approach

**Create new crate:** `crates/docling-pdf-ml/`
- All ML models isolated in separate crate
- Optional dependency (feature flag)
- Git LFS for model files (~480MB)

**Copy source code:** 31,419 lines
- Copy directly (production-ready, don't rewrite)
- Adapt imports/types for docling_rs
- Port 214 tests

**Delete simple backend:** ~1,000 lines removed from pdf.rs
- No fallback, no heuristics
- ML pipeline is THE ONLY implementation
- Clear error if models unavailable

### Test Strategy

**232 tests total:**
- 167 library tests (unit tests)
- 21 comprehensive tests (end-to-end)
- 26 orchestrator tests (integration)
- 18 canonical tests (docling_rs existing)

**Framework:** Pure Rust (cargo test), no pytest

---

## Planning Documents (On Migration Branch)

Switch to branch to read:
```bash
git checkout feature/pdf-ml-migration
```

**Read in order:**
1. **START_HERE_PDF_MIGRATION.md** - Entry point for worker
2. **WORKER_INTEGRATION_PLAN.md** - Phase-by-phase implementation
3. **MANAGER_REVISED_PDF_MIGRATION_PLAN.md** - Post-cleanup analysis
4. **MANAGER_PDF_MIGRATION_DIRECTIVE.md** - Critical requirements
5. **MANAGER_SUMMARY_PDF_MIGRATION.md** - Complete summary

---

## Critical Requirements

### 1. Delete Simple Backend (Phase 12)

**Remove from pdf.rs:**
- build_markdown() with header detection
- join_text_fragments()
- All list detection heuristics
- All text assembly heuristics
- ~1,000 lines total

**Replace with:**
- ML pipeline integration (~200 lines)
- DocItem generation (content_blocks always Some)
- No fallback code

### 2. Full Feature Parity

**All 4 ML models:**
- RapidOCR (detection, classification, recognition)
- LayoutPredictor (document structure)
- TableFormer (table parsing)
- CodeFormula (optional enrichment)

**All tests:**
- 214 tests from source
- 100% pass rate maintained

### 3. Production Quality

- Zero warnings (clippy + rustdoc)
- Performance: 16 pages/sec (MPS)
- Memory: <2GB per page
- Error handling: Comprehensive

---

## 14-Phase Plan Overview

| Phase | Duration | Work |
|-------|----------|------|
| 0 | ✅ | Foundation (done on branch) |
| 1-2 | 3-5 days | Types, PDF reader |
| 3-6 | 11-13 days | Preprocessing, OCR, Layout, Post-processing |
| 7-9 | 7-9 days | Assembly, Table, Reading order |
| 10-11 | 5-6 days | Executor, Export |
| 12 | 2-3 days | **Integration & deletion** |
| 13-14 | 4-6 days | Tests, documentation |

**Total:** 36-50 days (5-7 weeks)

---

## What's Different from Original Plan

### Improvements (Source Cleanup)

**✅ Simpler migration:**
- No pytest infrastructure (pure Rust)
- Cleaner source (14% less code)
- Better organized (debug artifacts gone)
- Excellent documentation

**✅ Faster timeline:**
- 5-7 weeks (was 6.5-9 weeks)
- 18% time reduction
- Lower complexity

**✅ Lower risk:**
- Production-ready source
- Clear module boundaries
- Comprehensive tests

### Confirmed Decisions

**✅ Delete simple backend:**
- No fallback
- ML only
- Clear error if models missing

**✅ Full migration:**
- All 4 models
- All 214 tests
- Complete feature parity

---

## Worker AI Instructions

### When User Says "Begin Migration"

1. **Switch to migration branch:**
   ```bash
   git checkout feature/pdf-ml-migration
   ```

2. **Read START_HERE_PDF_MIGRATION.md**

3. **Execute Phase 0-14:**
   - Follow WORKER_INTEGRATION_PLAN.md
   - One commit per phase (# 1780-1793)
   - Test at every phase (100% pass rate)

4. **Phase 12 critical:**
   - Delete simple backend from pdf.rs
   - Wire ML pipeline
   - Verify all tests pass

5. **Final validation:**
   - 232 tests passing
   - Performance validated
   - Documentation complete

6. **Merge to main:**
   - Create PR
   - Final review
   - Merge

---

## Blockers

### ort 2.0 API Migration

**Status:** May still be blocking issue
**Impact:** docling-ocr needs API updates
**Fix time:** 2-4 hours
**Guide:** Will be on migration branch if needed

**Check before starting:**
```bash
cargo check -p docling-ocr
```

If fails with "ort 1.16 yanked" → fix first (2-4 hours)

---

## Success Criteria

### Must Achieve

- [ ] 232 tests passing (100%)
- [ ] 4 ML models working
- [ ] Performance: 16 pages/sec (MPS)
- [ ] Simple backend DELETED
- [ ] DocItems generated (always Some)
- [ ] Zero warnings

### Code Changes

- [ ] +32,000 lines (docling-pdf-ml crate)
- [ ] -800 lines (pdf.rs simplified)
- [ ] +480MB models (Git LFS)
- [ ] +2.3GB dependencies (optional)

---

## Manager Availability

**Manager AI (this session):**
- Created comprehensive plans
- Analyzed source repository
- Revised plan based on cleanup
- Created worker instructions
- Committed to feature/pdf-ml-migration branch

**For questions during implementation:**
- Reference planning documents
- Check source repo documentation (PROJECT_STATUS.md)
- Git log in source repo (N=0-185 history)

**Manager is NOT needed during worker execution** - plans are complete.

---

## Final Status

**✅ READY FOR WORKER EXECUTION**

**What's prepared:**
- Branch with all planning documents
- Source repository cleaned and validated
- Migration strategy defined
- Phase-by-phase instructions
- File migration map
- Test migration plan
- Success criteria

**What's needed:**
- User approval to begin
- Worker AI to execute phases
- 5-7 weeks of implementation time

**Expected result:**
- PDF backend transformed (ML-powered)
- Simple heuristics deleted
- DocItems generated for PDF
- 232 tests passing (100%)
- Production ready

---

**Manager AI sign-off:** Planning complete. Ready for execution.

**User:** Approve when ready, then worker begins Phase 0 on feature/pdf-ml-migration branch.
