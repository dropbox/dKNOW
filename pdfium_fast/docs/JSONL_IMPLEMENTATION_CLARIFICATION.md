# JSONL Implementation Clarification

**Date:** 2025-11-20
**Discovery:** WORKER0 N=628
**Status:** DUAL IMPLEMENTATION EXISTS

---

## Executive Summary

**User Discovery:** "Ok, it looks like you fixed the C++ CLI to also export JSONL. Is that correct?"

**Answer:** YES! Both implementations exist and work:

1. **C++ CLI**: `pdfium_cli extract-jsonl` (native, no dependencies)
2. **Rust Tool**: `rust/target/release/examples/extract_text_jsonl` (alternative API)

**Key Insight:** You CAN run everything without Rust! C++ CLI is self-contained.

**But:** Rust bindings are STILL REQUIRED for:
- Programmatic API access (library usage)
- Idiomatic Rust interface for Rust developers
- Alternative tooling (460 tests currently use Rust)

---

## JSONL Implementations

### 1. C++ CLI Implementation

**Command:** `pdfium_cli extract-jsonl input.pdf output.jsonl`

**Source:** `examples/pdfium_cli.cpp` lines 2928-3104
- `extract_jsonl_bulk()` function (177 lines)
- `extract_jsonl_debug()` function (similar)

**Features:**
- Full JSONL metadata extraction
- Character positions (x, y)
- Bounding boxes (width, height)
- Font metadata (family, size, weight, flags)
- Unicode codepoints
- Color information (fill, stroke)
- Matrix transformations
- No external dependencies (pure C++)

**Test:**
```bash
./out/Release/pdfium_cli extract-jsonl test.pdf output.jsonl
```

**Output Sample:**
```json
{"char":"表","unicode":34920,"bbox":[49.622002,791.623474,60.973999,802.819458],"origin":[49.250000,792.571472],"font_size":12.000000,"font_name":"MS-Gothic","font_flags":524293,"font_weight":460,"fill_color":[0,0,0,255],"stroke_color":[0,0,0,255],"angle":0.000000,"matrix":[1.000000,0.000000,0.000000,1.000000,37.250000,792.571472],"is_generated":false,"is_hyphen":false,"has_unicode_error":false}
```

**Status:** ✅ FULLY OPERATIONAL (verified N=628)

### 2. Rust Tool Implementation

**Command:** `rust/target/release/examples/extract_text_jsonl input.pdf output.jsonl page_num`

**Source:** `rust/pdfium-sys/examples/extract_text_jsonl.rs` (245 lines)

**Features:**
- Same JSONL output format as C++ CLI
- Idiomatic Rust API
- Requires Rust toolchain + cargo
- Requires libpdfium_render_bridge.dylib

**Test:**
```bash
./rust/target/release/examples/extract_text_jsonl test.pdf output.jsonl 0
```

**Status:** ✅ FULLY OPERATIONAL (460/460 tests pass)

---

## Current Test Suite Usage

**JSONL Tests (460 total):**
- Currently use: **Rust tool** (`extract_text_jsonl`)
- Could use: **C++ CLI** (`pdfium_cli extract-jsonl`)
- Both produce identical output

**Why tests use Rust:**
- Historical: Rust implementation came first
- Current: Tests haven't been migrated to C++ CLI
- Future: Could switch to C++ CLI (removes Rust dependency for tests)

---

## Key Insight: C++ CLI is Self-Contained

**Without Rust, you can:**
- ✅ Extract text (`pdfium_cli extract-text`)
- ✅ Extract JSONL (`pdfium_cli extract-jsonl`)
- ✅ Render images (`pdfium_cli render-pages`)
- ✅ Use all v1.6.0 features (progress, batch, errors)

**C++ CLI = Complete standalone tool** (no Rust required)

---

## Rust Bindings Value Proposition

**Even though C++ CLI can do everything, Rust bindings are REQUIRED for:**

### 1. Programmatic Library Access

**C++ CLI:** Command-line only (subprocess calls)
**Rust Bindings:** Direct library access (no subprocess overhead)

```rust
// Library usage (Rust bindings)
use pdfium_sys::*;

let doc = FPDF_LoadDocument(c"input.pdf");
let page = FPDF_LoadPage(doc, 0);
// Direct API access, no subprocess

// vs C++ CLI (subprocess)
subprocess.run(["pdfium_cli", "extract-text", "input.pdf", "output.txt"])
```

### 2. Rust Developer Experience

**Rust developers want Rust API:**
- Type safety
- Memory safety
- Idiomatic Rust patterns
- Cargo integration
- No FFI if using C++ CLI subprocess

### 3. Custom Integrations

**Rust bindings enable:**
- Custom PDF processing pipelines
- Streaming extraction (no temp files)
- In-memory processing
- Fine-grained control

**C++ CLI limitations:**
- Must write temp files
- Subprocess overhead
- Fixed command-line interface

### 4. Test Infrastructure

**Current:** 460 JSONL tests use Rust tool
**Future:** Could migrate to C++ CLI, but Rust bindings still valuable for other use cases

---

## Correct Documentation

### What to Say

✅ **"Rust bindings REQUIRED for programmatic access"**
✅ **"C++ CLI is self-contained (no Rust needed for command-line use)"**
✅ **"Both C++ and Rust implement JSONL extraction"**
✅ **"Choose C++ CLI for simplicity, Rust bindings for library usage"**

### What NOT to Say

❌ **"Rust bindings optional"** (they're required for 460 tests + library access)
❌ **"Rust only way to get JSONL"** (C++ CLI also has it)
❌ **"Must use Rust"** (C++ CLI works standalone)

---

## Dependency Matrix

| Use Case | C++ CLI Only | C++ CLI + Rust Bindings |
|----------|--------------|-------------------------|
| **Command-line text extraction** | ✅ Works | ✅ Works |
| **Command-line JSONL extraction** | ✅ Works | ✅ Works |
| **Command-line image rendering** | ✅ Works | ✅ Works |
| **Programmatic library access** | ⚠️ Subprocess only | ✅ Direct API |
| **Rust projects integration** | ⚠️ Subprocess only | ✅ Native FFI |
| **Run test suite (460 JSONL tests)** | ⚠️ Need to migrate | ✅ Works now |
| **Custom PDF pipelines** | ⚠️ Limited | ✅ Full control |

---

## Recommendation

### For End Users (Command-Line)

**Use C++ CLI only:**
```bash
# Build just C++ CLI
~/depot_tools/gn gen out/Release --args='is_debug=false pdf_enable_v8=false pdf_enable_xfa=false use_clang_modules=false'
~/depot_tools/ninja -C out/Release pdfium_cli

# Use it
./out/Release/pdfium_cli extract-text document.pdf output.txt
./out/Release/pdfium_cli extract-jsonl document.pdf output.jsonl
./out/Release/pdfium_cli render-pages document.pdf images/
```

**No Rust needed!**

### For Developers (Library Access)

**Build C++ CLI + Rust bindings:**
```bash
# Build C++ components
~/depot_tools/ninja -C out/Release pdfium_cli pdfium_render_bridge

# Build Rust bindings
cd rust && cargo build --release
```

**Use Rust API:**
```rust
use pdfium_sys::*;
// Direct library access, no subprocess
```

---

## Documentation Updates Needed

### CLAUDE.md

**Current (WRONG):**
"Rust bridge REQUIRED (460 JSONL tests depend on it)"

**Correct:**
"Rust bindings REQUIRED for:
- Programmatic library access (alternative to subprocess)
- 460 JSONL tests (currently use Rust tool, could migrate to C++ CLI)
- Rust developer experience (idiomatic API)

C++ CLI is self-contained and provides extract-jsonl command (no Rust needed for CLI use)."

### README.md

**Add section:** "Rust Bindings vs C++ CLI"

**Clarify:**
- C++ CLI: Self-contained, command-line use, no Rust required
- Rust Bindings: Programmatic access, library integration, Rust projects

---

## Build System Fix

**Issue:** SDK 15.2 missing DarwinFoundation1/2/3.modulemap

**Fix:** `use_clang_modules=false` in GN args

**Applied:**
```bash
gn gen out/Release --args='is_debug=false pdf_enable_v8=false pdf_enable_xfa=false use_clang_modules=false'
```

**Result:**
- ✅ Release build works
- ✅ libpdfium_render_bridge.dylib built (4.7 MB)
- ✅ Rust tools rebuilt successfully
- ✅ 460/460 JSONL tests pass

---

## Copyright

Copyright © 2025 Andrew Yates. All rights reserved.
