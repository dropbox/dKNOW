# ⭐ CANONICAL STATUS - Single Source of Truth

**Date:** 2025-11-11
**Manager Position:** N=257
**Worker Position:** N=313
**Purpose:** SINGLE AUTHORITATIVE source. All other docs are stale.

---

## ⚠️ READ THIS ONLY

**All other status documents are STALE and INCONSISTENT:**
- ❌ FORMAT_PROCESSING_GRID.md (stale, last updated N=238)
- ❌ ALL_56_FORMATS_GRID.md (created N=252, may be outdated)
- ❌ COMPLETE_STATUS_REPORT_N256.md (my analysis, not verified)
- ❌ Various manager planning docs (historical only)

**ONLY trust:**
- ✅ **This file (CANONICAL_STATUS.md)**
- ✅ **Git log** (ground truth of what was actually done)
- ✅ **Actual code** (crates/docling-*/src/)
- ✅ **Actual tests** (integration_tests.rs)

**Worker: Ignore conflicting docs. Update THIS file only.**

---

## 📋 MANAGER WILL CHECK EVERY SESSION (N mod 10)

**Python dependency audit:**
```bash
# Count Python bridge calls in backend code
grep -r "python_bridge" crates/docling-*/src/ | grep -v "docling-core" | wc -l

# Must be: 0
# Current: 6 ❌
```

**When next AI starts:**
1. Run Python dependency audit
2. Check PYTHON_DEPENDENCY_TRACKER.md
3. Verify violations decreasing
4. If not decreasing: Block worker, demand fixes

**This will continue EVERY session until 0 Python dependencies.**

---

## CURRENT STATE (Verified from Code)

### Implementation Status

**Check actual backend files:**
```bash
ls crates/docling-backend/src/*.rs | wc -l
# Should show ~25 backend files

ls crates/docling-microsoft-extended/src/*.rs
ls crates/docling-apple/src/*.rs
ls crates/docling-latex/src/*.rs
```

**From git log (N=312):**
- MS Extended integrated: PUB, VSDX, ONE, MPP, MDB
- Apple integrated: PAGES, NUMBERS, KEY
- LaTeX integrated: TEX

**Total estimate: ~42 formats with backends**

### Python-Only Formats (NEED RUST)

**Verified:**
1. **PNG** - No Rust backend, uses Python OCR
2. **TIFF** - No Rust backend, uses Python OCR
3. **WEBP** - No Rust backend, uses Python OCR
4. **(JPEG)** - No Rust backend, 0 canonical tests

**Why still Python:** Require OCR, worker hasn't implemented yet

**Blocker:** OCR implementation (RapidOCR v5 + ONNX + macOS GPU)

### Deferred Formats

**Verified:**
1. **XPS** - Has stub, low demand
2. **IDML** - Has stub, complex Adobe format

**TEX:** Backend exists (latex.rs), need to verify complete

---

## CRITICAL WORK ITEMS

### 1. Implement LLM Mode 3 (BLOCKING 30 formats)

**Status:** NOT IMPLEMENTED
**Impact:** 30 formats can't have LLM validation
**Effort:** 2-3 commits
**Priority:** 🔴 CRITICAL

**Code needed:**
```rust
// In verifier.rs
pub async fn verify_standalone(
    &self,
    input_file: &Path,
    output: &str,
    format: InputFormat,
) -> Result<QualityReport> {
    // Implementation TODO
}
```

---

### 2. Implement Image OCR (3 canonical tests)

**Status:** NOT IMPLEMENTED
**Formats:** PNG, TIFF, WEBP
**Effort:** 10-15 commits
**Priority:** 🔴 HIGH (canonical tests!)

**Requirements:**
- RapidOCR v5 with PaddleOCR models
- ONNX Runtime (Linux) - `ort` crate exists
- macOS GPU - Research needed (PyTorch/CoreML/MLX)
- Dual-platform backend

**Blocker:** macOS GPU research not done

---

### 3. Add 30 LLM Mode 3 Tests

**Status:** TODO (after Mode 3 implemented)
**Formats:** All without Python baseline
**Effort:** 10-15 commits
**Priority:** 🔴 HIGH

**Pattern:** Copy from Mode 2 tests, call verify_standalone() instead

---

### 4. Verify Recent Integrations (N=312)

**Must verify:**
- Do TEX, PUB, VSDX, ONE, MPP, MDB, PAGES, NUMBERS, KEY generate DocItems?
- Are integration tests added?
- Do tests pass?

**Action:** Code review of N=312 changes

---

## ANSWERS TO YOUR QUESTIONS

**Q1: Need more test files?**
**A:** ❌ NO - Have adequate coverage (~48/50 formats)

**Q2: More help with LLM validation?**
**A:** ✅ YES - Mode 3 NOT IMPLEMENTED (blocks 30 formats)

**Q3: Why 3 Python and 3 deferred?**
**A:**
- **Python (3):** Image OCR not implemented (need RapidOCR v5)
- **Deferred (2):** XPS, IDML (low priority, stubs exist)
- **TEX:** Unclear - backend exists, need to verify

---

## BLOCKING PRIORITIES

**#1: Implement LLM Mode 3** (2-3 commits) - BLOCKS 30 LLM tests
**#2: Verify TEX complete** (0-2 commits) - Clarify status
**#3: Implement Image OCR** (10-15 commits) - Unblocks 3 canonical tests
**#4: Add 30 Mode 3 LLM tests** (10-15 commits) - Complete validation coverage
**#5: Decide XPS/IDML** (0-1 commits) - Mark out of scope or implement

**Total: 25-40 commits to completion**

---

## HOW TO UPDATE THIS FILE

**Worker must:**
1. Update THIS file after significant changes
2. Mark items [x] when complete
3. Add new discovered work
4. Keep this as SINGLE source of truth
5. Let other docs go stale (don't try to sync all)

**This file is authoritative. All others are historical/stale.**

---

**Next AI: Work from THIS file only. Implement Mode 3, verify TEX, tackle image OCR.**
