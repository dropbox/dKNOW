# PDFium Fast - Architecture

**Updated:** 2025-11-04

---

## Layer Architecture

```
┌─────────────────────────────────────────┐
│  User Application (Rust)                │
│  - Safe API                             │
│  - Idiomatic Rust patterns              │
│  - Memory safety guarantees             │
└──────────────┬──────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────┐
│  Rust API Layer (rust/pdfium-sys)       │
│  - FFI bindings to C++ CLI              │
│  - High-level Rust abstractions         │
│  - Error handling                       │
└──────────────┬──────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────┐
│  C++ CLI (examples/pdfium_cli.cpp)      │  ← OPTIMIZATION TARGET
│  - Performance layer                    │
│  - Form rendering (FPDF_FFLDraw)        │
│  - Multi-process coordination           │
│  - --workers N, --debug modes           │
└──────────────┬──────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────┐
│  PDFium Core (Google upstream)          │
│  - Unmodified Google code               │
│  - PDF parsing and rendering            │
└─────────────────────────────────────────┘
```

---

## Current Status (v0.1.0-alpha)

### Layer 1: Rust API (NOT YET IMPLEMENTED)

**Status:** Planned for v1.0

**Purpose:** User-facing API

**Example:**
```rust
use pdfium_fast::Document;

let doc = Document::open("input.pdf")?;

// Extract text
let text = doc.extract_text()?;

// Render thumbnails
doc.render_thumbnails("output/", 150)?;
```

### Layer 2: Rust Bindings (PARTIAL)

**Status:** FFI bindings exist, need form rendering

**Location:** `rust/pdfium-sys/`

**Current:**
- ✅ Raw FFI bindings to PDFium C API
- ✅ Example implementations (render_pages.rs, extract_text.rs)
- ❌ Missing FPDF_FFLDraw (forms don't render)
- ❌ Not used in production tests

**Needs:**
- Add FPDF_FFLDraw to render_pages.rs
- Test forms render correctly from Rust
- Validate Rust → PDFium path works 100%

### Layer 3: C++ CLI (PRODUCTION)

**Status:** Complete and optimized (v0.1.0-alpha)

**Location:** `examples/pdfium_cli.cpp`

**Features:**
- ✅ Form rendering (FPDF_FFLDraw)
- ✅ Multi-process parallelism (3.8x-4.0x speedup)
- ✅ Unified --workers N API (1-16 workers, plus --debug)
- ✅ 100% upstream correctness
- ❌ Per-core speed not yet optimized (v0.2.0 target)

**This is what tests use and what we're optimizing.**

### Layer 4: PDFium Core

**Status:** Upstream Google code, unmodified

**Modifications:** 0 lines changed

---

## Why This Layering?

### Problem: Rust Can't Easily Do C Callbacks

**PDFium requires C callbacks for forms:**
```c
typedef struct FPDF_FORMFILLINFO_ {
    int version;
    FPDF_PAGE (*FFI_GetPage)(struct FPDF_FORMFILLINFO_* param,
                              FPDF_DOCUMENT document,
                              int page_index);
    // ... more callbacks
} FPDF_FORMFILLINFO;
```

**Rust FFI challenge:**
- Rust closures can't be passed as C function pointers
- Requires complex wrapper with `extern "C"` functions
- Memory management is tricky
- Error-prone

### Solution: C++ Bridge Layer

**C++ can easily provide callbacks:**
```cpp
FPDF_FORMFILLINFO form_callbacks = {};
form_callbacks.version = 1;
form_callbacks.FFI_GetPage = GetPageForIndex;  // Simple!
FPDF_FORMHANDLE form = FPDFDOC_InitFormFillEnvironment(doc, &form_callbacks);
```

**Rust calls C++ CLI:**
```rust
// Simple subprocess call
std::process::Command::new("pdfium_cli")
    .args(&["--workers", "4", "render-pages", "input.pdf", "output/"])
    .status()?;
```

---

## Development Phases

### v0.1.0-alpha (DONE)

**Focus:** Multi-threading + correctness

- ✅ C++ CLI with 3 modes
- ✅ Form rendering fix
- ✅ 100% correctness validation
- ✅ 3.8x-4.0x speedup

### v0.2.0-beta (IN PROGRESS)

**Focus:** Single-core optimization + application features

- 🔄 Optimize C++ CLI per-core performance (1.5x target)
- 🔄 Add --dpi flag
- 🔄 Add --thumbnail mode (150 DPI JPEG)
- 🔄 Smart scanned PDF detection

### v0.3.0 (FUTURE)

**Focus:** Rust API layer

- Build clean Rust API wrapping C++ CLI
- Add FPDF_FFLDraw to Rust examples
- Validate forms work from Rust
- Publish as Rust crate

### v1.0 (PRODUCTION)

**Focus:** Complete Rust library

- Production-quality Rust API
- Cross-platform (macOS, Linux, Windows)
- Documentation for Rust users
- Published to crates.io

---

## Why Optimize C++ First, Rust Later?

**Performance work happens in C++:**
- All optimizations (caching, SIMD, buffering) are in pdfium_cli.cpp
- Rust wrapper just calls the optimized C++ CLI
- No performance work needed in Rust layer

**Rust layer is thin:**
- Process management (spawn C++ CLI)
- Error handling (parse stderr/exit codes)
- API design (ergonomic Rust interface)

**Result:**
- Optimize C++ once → all languages benefit
- Rust, Python, Go can all wrap the fast C++ CLI
- Performance work is centralized

---

## Form Rendering Status

### C++ CLI: ✅ Forms Work

**Location:** `examples/pdfium_cli.cpp:919`
```cpp
FPDF_FFLDraw(form, bitmap, page, 0, 0, width_px, height_px, 0, FPDF_ANNOT);
```

**Validation:**
- 9 PDFs tested (0100pages, web_041, etc.)
- 100% match with upstream
- All form fields render correctly

### Rust Wrapper: ❌ Forms DON'T Work Yet

**Location:** `rust/pdfium-sys/examples/render_pages.rs`

**Problem:** Missing FPDF_FFLDraw call

**Status:**
- Has form initialization (FPDFDOC_InitFormFillEnvironment)
- Has form callbacks (FORM_OnAfterLoadPage)
- Missing: FPDF_FFLDraw (actual form field rendering)

**Fix needed:**
```rust
// Add after FPDF_RenderPage_Close() at line ~561
if !form_handle.is_null() {
    FPDF_FFLDraw(form_handle, bitmap, page, 0, 0, width_px, height_px, 0, FPDF_ANNOT);
}
```

**Priority:** Medium (not urgent since production uses C++ CLI, but needed for v0.3.0)

---

## Testing Strategy

### Current (v0.1.0-v0.2.0)

**Tests use C++ CLI:**
- All smoke tests: `render_tool` (C++ CLI)
- All performance tests: C++ CLI
- Form rendering validated: C++ CLI only

**Rust tools:**
- Not used in production tests
- Exist as reference implementations
- Need form fix before use

### Future (v0.3.0+)

**Add Rust API tests:**
- Test Rust wrapper directly
- Validate forms via Rust path
- Ensure Rust → C++ → PDFium works correctly

---

## Summary

**Current architecture is CORRECT:**

- ✅ C++ CLI is the performance layer (being optimized)
- ✅ Rust API will wrap C++ CLI (future work)
- ✅ Forms work in C++ CLI (100% tested)
- ❌ Forms don't work in Rust yet (needs FPDF_FFLDraw added)

**Optimization focus (v0.2.0):**
- Optimize C++ CLI only
- Rust wrapper waits until v0.3.0
- Form rendering already works (via C++ CLI)

**The Rust wrapper is NOT legacy** - it's the future API layer. Just not the current optimization target.
