# PyTorch Feature Compilation Blockers

**Date:** 2025-11-23
**Context:** Phase 12 Integration blocked by pytorch feature compilation errors
**Command:** `LIBTORCH_USE_PYTORCH=1 LIBTORCH_BYPASS_VERSION_CHECK=1 cargo check -p docling-pdf-ml --features pytorch`

## Summary

The docling-pdf-ml crate compiles successfully **without** the pytorch feature (`cargo check -p docling-pdf-ml`). However, enabling the pytorch feature reveals 13 compilation errors due to missing implementations and incorrect type references.

## Environment

- **PyTorch Version:** 2.9.0 (installed at `/opt/homebrew/lib/python3.14/site-packages/torch`)
- **tch-rs Expected Version:** 2.5.1 (bypassed with `LIBTORCH_BYPASS_VERSION_CHECK=1`)
- **Rust Toolchain:** Stable
- **Platform:** macOS ARM64

## Compilation Errors (13 total)

### 1. Reading Order Type Mismatches (2 errors)

**Location:** `crates/docling-pdf-ml/src/pipeline/executor.rs:7`

```
error[E0432]: unresolved imports `crate::pipeline::reading_order::ReadingOrderConfig`,
              `crate::pipeline::reading_order::ReadingOrderPredictor`
```

**Root Cause:**
Executor imports old type names `ReadingOrderConfig` and `ReadingOrderPredictor`, but reading_order module exports `Stage10Config` and `Stage10ReadingOrder`.

**Fix:**
```rust
// Change from:
use crate::pipeline::reading_order::{ReadingOrderConfig, ReadingOrderPredictor};

// To:
use crate::pipeline::reading_order::{Stage10Config, Stage10ReadingOrder};

// Then update all references in the file
```

**Impact:** Blocking Phase 12 integration

---

### 2. Missing code_formula Module (4 errors)

**Locations:**
- `crates/docling-pdf-ml/src/pipeline/executor.rs:697` (struct field)
- `crates/docling-pdf-ml/src/pipeline/executor.rs:1108` (model loading)
- Multiple usage sites throughout executor

```
error[E0433]: failed to resolve: could not find `code_formula` in `models`
   --> crates/docling-pdf-ml/src/pipeline/executor.rs:697:49
    |
697 |     code_formula: Option<crate::models::code_formula::CodeFormulaModel>,
    |                                         ^^^^^^^^^^^^^ could not find `code_formula` in `models`
```

**Root Cause:**
Code/formula enrichment feature (`CodeFormulaModel`) was planned but never implemented. The executor references `crate::models::code_formula::CodeFormulaModel` which doesn't exist.

**Options:**

**Option A: Stub Implementation (Quick)**
1. Create `crates/docling-pdf-ml/src/models/code_formula.rs` with stub:
   ```rust
   pub struct CodeFormulaModel;

   impl CodeFormulaModel {
       pub fn from_pretrained(_path: &Path, _device: Device) -> Result<Self> {
           unimplemented!("CodeFormula not yet implemented")
       }
   }
   ```
2. Add to `src/models/mod.rs`: `pub mod code_formula;`
3. Set `code_formula_enabled: false` by default (already done)

**Option B: Conditional Compilation (Clean)**
1. Wrap all code_formula references with `#[cfg(feature = "code-formula")]`
2. Add new Cargo feature: `code-formula = []`
3. Only compile code_formula code when feature enabled

**Option C: Remove Feature (Simplest)**
1. Remove all code_formula references from executor.rs
2. Remove from PipelineConfig
3. Document as future enhancement

**Recommendation:** Option A (stub) for now, Option B for production

**Impact:** Blocking Phase 12 integration

---

### 3. Missing RapidOcr Integration (2 errors)

**Locations:**
- `crates/docling-pdf-ml/src/pipeline/executor.rs:705` (struct field type)
- `crates/docling-pdf-ml/src/pipeline/executor.rs:1121` (model instantiation)

```
error[E0433]: failed to resolve: could not find `RapidOcr` in `ocr`
   --> crates/docling-pdf-ml/src/pipeline/executor.rs:705:32
    |
705 |     ocr: Option<crate::ocr::RapidOcr>,
    |                                ^^^^^^^^ not found in `ocr`
```

**Root Cause:**
RapidOCR module exists (`src/ocr/mod.rs`) but doesn't export `RapidOcr` type. The module has `OcrEngine` and `RapidOcrResult` but not the main `RapidOcr` struct.

**Fix:**
1. Check `src/ocr/mod.rs` - it should export `pub struct RapidOcr`
2. If missing, implement based on the Python RapidOCR wrapper
3. Ensure it has `fn new(model_dir: impl AsRef<Path>) -> Result<Self>`

**Files to Check:**
- `crates/docling-pdf-ml/src/ocr/mod.rs`
- `crates/docling-pdf-ml/src/ocr/text_cell.rs`

**Impact:** Blocking Phase 12 integration

---

### 4. Missing Assembly Pipeline Types (5 errors)

**Locations:** `crates/docling-pdf-ml/src/pipeline/executor.rs` (multiple lines)

```
error[E0412]: cannot find type `LabeledClusters` in this scope
error[E0422]: cannot find struct, variant or union type `LabeledClusters` in this scope
error[E0412]: cannot find type `OCRCells` in this scope
error[E0422]: cannot find struct, variant or union type `OCRCells` in this scope
error[E0412]: cannot find type `LabeledClusters` in this scope
```

**Root Cause:**
Executor references types `LabeledClusters`, `LabeledCluster`, `OCRCells` but they're not imported or exported correctly.

**Check:**
1. These types should be in `pipeline::assembly` module
2. Verify they're exported in `pipeline::assembly::mod.rs`
3. Add to executor imports:
   ```rust
   use crate::pipeline::assembly::{LabeledClusters, LabeledCluster, OCRCells, ClustersWithCells};
   ```

**Impact:** Blocking Phase 12 integration

---

### 5. Type Size Error (1 error)

**Location:** `crates/docling-pdf-ml/src/pipeline/executor.rs:1298`

```
error[E0277]: the size for values of type `[pipeline::data_structures::SimpleTextCell]` cannot be known at compilation time
    --> crates/docling-pdf-ml/src/pipeline/executor.rs:1298:16
     |
1298 |         if let Some(cells) = textline_cells.filter(|c| !c.is_empty()) {
     |                ^^^^^^^^^^^ doesn't have a size known at compile-time
```

**Root Cause:**
Trying to pattern match on unsized slice type. The `filter` closure takes `&[SimpleTextCell]` which is unsized.

**Fix:**
```rust
// Change from:
if let Some(cells) = textline_cells.filter(|c| !c.is_empty()) {

// To:
if let Some(cells) = textline_cells.filter(|c: &&[SimpleTextCell]| !c.is_empty()) {
// Or:
if let Some(cells) = textline_cells {
    if !cells.is_empty() {
        // ...
    }
}
```

**Impact:** Minor, easy fix

---

## Fix Priority

### Critical (Must Fix Before Phase 12)
1. **Reading Order types** - Simple rename, 5 minutes
2. **Assembly Pipeline types** - Add imports, 10 minutes
3. **RapidOcr** - Verify exports, 15 minutes
4. **Type size error** - Simple pattern match fix, 5 minutes

### Important (Can Stub Temporarily)
5. **code_formula** - Stub implementation, 30 minutes

**Total Estimated Time:** 65 minutes for all fixes

---

## Testing After Fixes

```bash
# Set environment for pytorch
export LIBTORCH_USE_PYTORCH=1
export LIBTORCH_BYPASS_VERSION_CHECK=1

# Test compilation
cargo check -p docling-pdf-ml --features pytorch

# Should succeed with 0 errors

# Then test Phase 12 integration
# (Add docling-pdf-ml to backend, wire into pdf.rs)
```

---

## Next Steps

1. Fix critical errors (1-4) in order
2. Stub code_formula (5)
3. Verify compilation succeeds
4. Proceed with Phase 12 integration
5. Test with canonical PDF tests

---

## Notes

- **Without pytorch feature:** Compilation succeeds (Phase 11 complete)
- **With pytorch feature:** 13 errors block Phase 12 integration
- **Root cause:** Phases 8-10 implementation didn't test with pytorch feature enabled
- **Lesson:** Always test with all features before marking phase complete

---

**Generated:** 2025-11-23
**Author:** Claude AI (N=12)
**Status:** BLOCKING Phase 12 Integration
