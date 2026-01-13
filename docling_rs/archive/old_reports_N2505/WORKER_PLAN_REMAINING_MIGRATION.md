# WORKER PLAN - Complete Remaining PDF ML Migration

**Date:** 2025-11-23 16:45 PT
**Status:** Core integration DONE and TESTED, remaining pieces below
**Branch:** feature/pdf-ml-migration
**Estimated:** 3-4 days remaining work

---

## What Manager Completed (Today)

### ✅ Phase 12: Integration
- Deleted old backend (350 lines)
- Wired ML into pdf.rs
- Fixed parameter bugs
- **Verified it RUNS** (end-to-end test passes)

### ✅ Setup
- Environment configuration (setup_env.sh)
- Cargo config (rpath to torch/llvm libs)
- Models copied (15MB RapidOCR)
- **Verified it BUILDS and RUNS**

---

## What's Actually Missing (NOT Optional)

**Current state:** 26,711 lines copied (85% of source's 31,419)
**Missing:** 4,708 lines (15%)

### 1. CodeFormula Model (3,893 lines) - REQUIRED

**Status:** Only stub exists (40 lines)

**Why Required:**
- Part of the 5-model pipeline
- Source has it fully implemented
- Used for code/formula enrichment
- Tests depend on it

**Files to copy:**
```bash
# From ~/docling_debug_pdf_parsing/src/models/code_formula/
config.rs          (~300 lines)
connector.rs       (~500 lines)
mod.rs            (~200 lines)
preprocessor.rs    (~800 lines)
text_decoder.rs    (~600 lines)
tokenizer.rs       (~700 lines)
vision.rs         (~793 lines)
```

**Total:** 3,893 lines

**Time estimate:** 1 day
- Copy files: 1 hour
- Fix imports: 2 hours
- Test: 2-4 hours
- Debug: 2-4 hours

### 2. LayoutPredictor Files (0 lines - MISSING ENTIRELY)

**Status:** We have PyTorch backend but missing ONNX backend files

**Current:**
- layout/pytorch_backend/*.rs ✅ EXISTS (8,285 lines)
- layout/onnx.rs ✅ EXISTS (945 lines)
- layout/mod.rs ✅ EXISTS (108 lines)

**Wait - actually we HAVE these!** Let me verify...

**Action:** Verify LayoutPredictor is complete (check against source)

### 3. Tests (214 tests) - REQUIRED

**Status:** 0 tests copied

**Why Required:**
- Verify the integration actually works correctly
- Catch bugs in type conversions
- Validate against baselines
- Quality assurance

**Source has:**
- 165 library unit tests
- 3 orchestrator tests (26 pages)
- 21 comprehensive tests
- 25 pytest tests

**Files to copy:**
```bash
cp -r ~/docling_debug_pdf_parsing/tests/*.rs ./tests/
```

**Time estimate:** 1-2 days
- Copy tests: 2 hours
- Fix imports/paths: 4 hours
- Run tests: 2 hours
- Debug failures: 4-12 hours

### 4. docling_export.rs (329 lines) - MAYBE REQUIRED

**Status:** Not copied

**Why Might Be Required:**
- Source uses it for full DoclingDocument format
- Our export_to_markdown() is simpler (may be incomplete)
- Tests might depend on it

**File to copy:**
```bash
cp ~/docling_debug_pdf_parsing/src/pipeline/docling_export.rs \
   crates/docling-pdf-ml/src/pipeline/
```

**Time estimate:** 2-4 hours
- Copy file: 15 min
- Fix imports: 1 hour
- Wire into pipeline: 1 hour
- Test: 1-2 hours

### 5. docling_document.rs (329 lines) - MAYBE REQUIRED

**Status:** Not copied

**Why Might Be Required:**
- Defines DoclingDocument struct used by docling_export
- Tests might need it for baseline comparison

**File to copy:**
```bash
cp ~/docling_debug_pdf_parsing/src/docling_document.rs \
   crates/docling-pdf-ml/src/
```

**Time estimate:** 1-2 hours

### 6. Baseline Test Infrastructure - REQUIRED FOR TESTS

**Status:** baseline.rs copied but may need data files

**Source has:**
- baseline_data/ directory (git-ignored, several GB)
- Python scripts to regenerate baselines
- Stage-by-stage validation files

**Action needed:**
- Copy baseline data OR regenerate in target
- Verify test infrastructure works

**Time estimate:** 2-4 hours

### 7. Documentation - REQUIRED

**Status:** Minimal inline docs only

**Required:**
- README.md in docling-pdf-ml/
- Architecture diagram (how 5 models connect)
- Setup guide (environment, models)
- Usage examples
- Performance characteristics
- Troubleshooting guide

**Time estimate:** 4-6 hours

---

## Worker Checklist (Complete ALL)

### Task 1: Copy CodeFormula (1 day)

**Steps:**
1. Copy 9 files from source (3,893 lines)
2. Fix imports for docling_rs
3. Wire into pipeline executor
4. Test model loading
5. Verify it works with pipeline

**Commit:** `# N: CodeFormula Model Complete`

### Task 2: Copy docling_export + docling_document (2-4 hours)

**Steps:**
1. Copy docling_document.rs
2. Copy docling_export.rs
3. Fix imports
4. Wire into export path
5. Test export functions

**Commit:** `# N+1: Export Infrastructure Complete`

### Task 3: Port Tests (1-2 days)

**Steps:**
1. Copy all test files from source
2. Copy or regenerate baseline data
3. Fix imports and paths
4. Run tests
5. Debug failures until 100% pass rate
6. Target: 214/214 tests passing

**Commit:** `# N+2: All Tests Ported and Passing (214/214)`

### Task 4: Complete Documentation (4-6 hours)

**Steps:**
1. Write README.md for docling-pdf-ml/
2. Create architecture diagram
3. Document setup process
4. Add usage examples
5. Performance benchmarks
6. Troubleshooting guide

**Commit:** `# N+3: Documentation Complete`

### Task 5: Final Validation (2-4 hours)

**Steps:**
1. Test on diverse PDFs (scanned, programmatic, complex)
2. Compare output vs source repo
3. Verify quality matches
4. Fix any discrepancies
5. Run full canonical test suite

**Commit:** `# N+4: Full Validation Complete`

---

## Success Criteria (ALL Required)

Before declaring COMPLETE:

- [ ] CodeFormula implemented (3,893 lines)
- [ ] docling_export implemented (329 lines)
- [ ] docling_document implemented (329 lines)
- [ ] All 214 tests ported and passing (100%)
- [ ] Complete documentation (README, setup guide, examples)
- [ ] Validated on diverse PDFs
- [ ] Output quality matches source repo
- [ ] Zero compilation errors
- [ ] Zero test failures

**9/9 criteria must be met**

---

## Timeline Estimate

**Task 1 (CodeFormula):** 1 day (6-8 hours)
**Task 2 (Export):** 0.5 days (2-4 hours)
**Task 3 (Tests):** 1.5 days (10-16 hours)
**Task 4 (Docs):** 0.5 days (4-6 hours)
**Task 5 (Validation):** 0.5 days (2-4 hours)

**Total:** 3-4 days (24-32 hours)

**Current completion:** Day 3.7
**Target completion:** Day 6-8 (around Nov 26-29)

---

## Why These Are NOT Optional

**User said:** "None of those pieces you listed are optional"

**User is RIGHT:**

1. **CodeFormula:** Part of the 5-model pipeline, source has it, tests use it
2. **Tests:** Need validation - can't trust untested code
3. **docling_export:** Source uses it, tests depend on it
4. **Documentation:** Users need to know how to use it
5. **Validation:** Must verify output quality matches source

**All pieces are REQUIRED for a complete port.**

---

## Current Status

**What's Done:**
- Core 4 models (OCR, Layout, Table, ReadingOrder)
- Pipeline executor and assembly
- Integration into pdf.rs
- Environment setup
- **Runtime verified (test passes)**

**What's Missing:**
- CodeFormula model (1 day)
- docling_export + docling_document (0.5 day)
- All 214 tests (1.5 days)
- Documentation (0.5 day)
- Validation (0.5 day)

**Total remaining:** 3-4 days

---

## Worker Instructions

### Start Here

1. **Read this plan completely**
2. **Set up environment:**
   ```bash
   cd ~/docling_rs
   git checkout feature/pdf-ml-migration
   source setup_env.sh
   ```

3. **Begin Task 1 (CodeFormula)**
   - Copy files from source
   - Follow checklist above
   - Commit when complete

4. **Continue through Tasks 2-5 sequentially**

5. **Commit after each task**

6. **Report blockers immediately**

### Environment

**ALWAYS run with:**
```bash
source setup_env.sh
```

**Before ANY cargo commands**

### Testing

**After each task, verify:**
```bash
cargo build -p docling-pdf-ml --features "pytorch,opencv-preprocessing" --release
cargo test -p docling-pdf-ml --features "pytorch,opencv-preprocessing"
```

**Both must succeed before moving to next task.**

---

## Manager Support

**Manager will:**
- Review each commit
- Help with blockers
- Verify quality
- Keep user informed

**Report blockers if stuck >4 hours**

---

## Final Notes

**This is NOT "optional polish"** - this is completing the port.

**Source has:**
- 31,419 lines
- 5 complete models
- 214 passing tests
- Full documentation

**We must match that** to call the port complete.

**Current:** 85% done (26,711 lines)
**Target:** 100% done (31,419 lines + tests + docs)

**Estimate:** 3-4 more days of focused work

---

**Generated by:** Manager AI
**Purpose:** Complete remaining work plan
**For:** Worker to execute
**Timeline:** 3-4 days to 100% completion
