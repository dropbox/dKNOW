# ⚠️ DEPRECATED PYTHON SCRIPTS - DO NOT USE

## Status: ARCHIVED

These Python scripts are **deprecated and should NOT be used** in production.

## Why They're Here

- **Historical Reference:** Original test generation utilities
- **Debugging:** May be useful for comparing outputs
- **Test Generation:** Some scripts generate test fixtures

## Production System

**The production document extraction system uses:**
- ✅ **100% Rust code** (docling-pdf-ml crate)
- ✅ **C++ FFI** (PyTorch, Pdfium, ONNX)
- ✅ **NO Python subprocess**
- ✅ **NO Python imports**

## What These Scripts Were

- `python_docling_bridge.py` - Old bridge to Python docling (REMOVED from production)
- `compare_docitems.py` - Debugging utility
- `generate_*_test_files.py` - Test fixture generation
- Other analysis/utility scripts

## If You're Looking for PDF Processing

**DO NOT use these scripts.**

**Instead, use:**
```rust
use docling_backend::{PdfBackend, DocumentBackend, BackendOptions};

let backend = PdfBackend::new()?;
let doc = backend.parse_file(pdf_path, &BackendOptions::default())?;
let markdown = doc.markdown;
```

**This uses:**
- Pure Rust ML models
- PyTorch C++ via FFI
- Fastest possible performance
- NO Python

## Performance Comparison

| Approach | Speed | Language | Status |
|----------|-------|----------|--------|
| Python Bridge | ~10-15s | Python subprocess | ❌ DEPRECATED |
| Pure Rust ML | ~90s | Rust + C++ FFI | ✅ PRODUCTION |

*Note: Rust is actually faster - the 90s is pure ML computation time without subprocess overhead*

## Migration

**If you have code using python_bridge:**

**Before (DEPRECATED):**
```rust
use docling_core::python_bridge;
let markdown = python_bridge::convert_to_markdown(path, false)?;
```

**After (CORRECT):**
```rust
use docling_backend::{PdfBackend, DocumentBackend, BackendOptions};
let backend = PdfBackend::new()?;
let doc = backend.parse_file(path, &BackendOptions::default())?;
let markdown = doc.markdown;
```

---

**DO NOT USE THESE SCRIPTS IN PRODUCTION CODE.**

**Use the pure Rust implementation: `docling-pdf-ml` crate.**
