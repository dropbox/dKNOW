# Design: Pure Rust+C++ Architecture - Prevent Python Confusion

## Problem

**Risk:** Future developers might use slow Python bridge instead of fast Rust ML pipeline.

**Current Issues:**
1. Two code paths exist (python-bridge vs pdf-ml)
2. Python scripts in repo could confuse developers
3. Integration tests use python-bridge (wrong path)
4. Unclear which is the "real" implementation

## Goal

**ABSOLUTE FASTEST Rust+C++ ONLY document extraction system.**

## Design Principles

1. **Single Code Path:** Only Rust ML pipeline exists
2. **No Python Option:** Remove python-bridge completely
3. **Clear Errors:** If dependencies missing, helpful error message
4. **Fast by Default:** Rust ML is THE implementation
5. **Zero Confusion:** Impossible to accidentally use Python

## Proposed Architecture

```
┌─────────────────────────────────────────────────────┐
│ User: DocumentConverter::convert(pdf_path)          │
└──────────────────┬──────────────────────────────────┘
                   ↓
┌─────────────────────────────────────────────────────┐
│ PdfBackend::parse_bytes()                           │
│ [ONLY ONE PATH - NO ALTERNATIVES]                   │
└──────────────────┬──────────────────────────────────┘
                   ↓
┌─────────────────────────────────────────────────────┐
│ docling-pdf-ml (Pure Rust + C++ FFI)                │
│ • Pdfium C++ (PDF loading)                          │
│ • PyTorch C++ (ML models via tch-rs)                │
│ • ONNX Runtime (RapidOCR)                           │
│ • Pure Rust (DocItems, serialization)               │
└──────────────────┬──────────────────────────────────┘
                   ↓
┌─────────────────────────────────────────────────────┐
│ Output: DocItems + Markdown                         │
│ (All generated in Rust)                             │
└─────────────────────────────────────────────────────┘
```

**NO PYTHON ANYWHERE. NO ALTERNATIVE PATHS.**

## Changes Required

### 1. Remove python-bridge Feature (HIGH PRIORITY)

**Delete:**
- `crates/docling-core/src/python_bridge.rs`
- python-bridge feature from `crates/docling-core/Cargo.toml`
- pyo3 dependency

**Why:** Removes slow alternative path, prevents confusion

### 2. Archive Python Scripts (HIGH PRIORITY)

**Move to `archive/python/` or delete:**
- `scripts/python_docling_bridge.py` ❌ DELETE
- `scripts/compare_docitems.py` → archive (for debugging)
- `scripts/generate_*_test_files.py` → archive (for test gen)
- Root-level `*.py` files → archive or delete

**Add `archive/python/README.md`:**
```
⚠️  DEPRECATED - DO NOT USE

These Python scripts are archived for reference only.

The production system uses 100% Rust + C++ (docling-pdf-ml).
NO Python code is executed in the production pipeline.

If you need to generate test files, these scripts may still be useful.
For actual document conversion, use the Rust implementation ONLY.
```

### 3. Fix Integration Tests (HIGH PRIORITY)

**Current:** Tests use `USE_HYBRID_SERIALIZER=1` (Python ML + Rust serializer)

**New:** Tests should use `--features pdf-ml` (Rust ML + Rust serializer)

**Changes:**
- Update test docs to use `--features pdf-ml`
- Remove `USE_HYBRID_SERIALIZER` references
- Make tests run pure Rust by default

### 4. Make pdf-ml Default or Clear (MEDIUM PRIORITY)

**Option A: Make pdf-ml default feature**
```toml
[features]
default = ["pdf-ml"]
pdf-ml = ["docling-pdf-ml/pytorch"]
```

**Option B: Clear error without it**
```rust
#[cfg(not(feature = "pdf-ml"))]
compile_error!("PDF support requires 'pdf-ml' feature. This is the ONLY supported path. Add --features pdf-ml");
```

**Recommendation:** Option A (default feature)

### 5. Update CLAUDE.md (HIGH PRIORITY)

**Remove:**
- All references to python-bridge
- Hybrid approach documentation
- Python ML usage

**Add:**
```markdown
## PDF Processing

**ONLY ONE PATH:** Pure Rust + C++ FFI

- Rust ML pipeline (docling-pdf-ml)
- PyTorch C++ via tch-rs
- ONNX Runtime for OCR
- NO Python subprocess
- NO pyo3

**Performance:** Fastest possible (compiled ML models)
```

### 6. Update README/Docs (MEDIUM PRIORITY)

**Make it crystal clear:**
- Rust ML is THE implementation
- No Python option
- Fast by default
- C++ FFI is allowed and expected (PyTorch, Pdfium)

### 7. Remove Python Test Dependencies (LOW PRIORITY)

**Delete if not needed:**
- Python docling installation requirements
- Python environment setup
- Anything that suggests Python is needed

## Execution Plan

### Phase 1: Remove Python Bridge (Breaking Change)

1. Delete `crates/docling-core/src/python_bridge.rs`
2. Remove python-bridge feature from Cargo.toml
3. Remove pyo3 dependency
4. Update integration tests to remove python-bridge usage
5. Update CLAUDE.md

### Phase 2: Archive Python Scripts

1. Create `archive/python/` directory
2. Move Python scripts there
3. Add deprecation README
4. Update .gitignore if needed

### Phase 3: Make pdf-ml Default

1. Add `default = ["pdf-ml"]` to features
2. Update documentation
3. Update setup instructions

### Phase 4: Clean Documentation

1. Remove all Python bridge docs
2. Update performance claims
3. Add "Rust+C++ ONLY" prominently

## Performance Benefits

**Python Bridge (Old):**
- Subprocess overhead: ~500ms
- JSON serialization: ~100ms
- Process communication: slow
- Total: ~10-15 seconds per document

**Pure Rust ML:**
- No subprocess: 0ms overhead
- Direct memory: Fast
- Compiled code: Optimized
- Total: ~90 seconds (all ML execution)

**Key:** Pure Rust is faster AND more maintainable.

## Safety Measures

### 1. Compile-Time Prevention

```rust
// In pdf.rs
#[cfg(not(feature = "pdf-ml"))]
compile_error!(
    "PDF support requires 'pdf-ml' feature.\n\
     This is the ONLY supported implementation.\n\
     The Python bridge has been removed.\n\
     Build with: cargo build --features pdf-ml"
);
```

### 2. Runtime Prevention

Remove all Python code paths so there's nothing to accidentally call.

### 3. Documentation Prevention

Clear docs stating: "Rust ML ONLY. No Python option exists."

### 4. Test Prevention

All tests use `--features pdf-ml`. No tests use python-bridge.

## Migration Path

For anyone with old code using python-bridge:

**Before:**
```rust
use docling_core::python_bridge;
let markdown = python_bridge::convert_to_markdown(path, false)?;
```

**After:**
```rust
use docling_backend::{PdfBackend, BackendOptions, DocumentBackend};
let backend = PdfBackend::new()?;
let doc = backend.parse_file(path, &BackendOptions::default())?;
let markdown = doc.markdown;
```

**Migration:** Straightforward, and much faster.

## Verification

After changes:
1. ✅ No Python scripts in production code
2. ✅ No python-bridge feature
3. ✅ No pyo3 dependency in production crates
4. ✅ All tests use pure Rust
5. ✅ Docs mention only Rust+C++
6. ✅ Impossible to accidentally use Python

## Recommended Execution Order

1. **First:** Archive Python scripts (low risk)
2. **Second:** Remove python-bridge feature (breaking change, needs testing)
3. **Third:** Make pdf-ml default (convenience)
4. **Fourth:** Update all documentation (clarity)
5. **Fifth:** Add compile-time guards (prevent accidents)

## Expected Outcome

**After execution:**
- 100% Rust + C++ codebase
- ZERO Python execution paths
- Clearest possible architecture
- Fastest possible performance
- Future-proof against confusion

**Time estimate:** 2-3 hours (careful removal, testing)

## Questions for User

1. **Delete or archive Python scripts?** (I recommend archive for test generation)
2. **Make pdf-ml default feature?** (I recommend yes)
3. **Remove integration tests using python-bridge?** (I recommend yes, rewrite for pure Rust)

Shall I proceed with Phase 1 (Remove Python Bridge)?
