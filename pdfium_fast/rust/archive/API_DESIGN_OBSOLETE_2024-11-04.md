# Rust API Design - Wrapping the C++ CLI

**Date:** 2025-11-04
**Status:** Design for v0.3.0+

---

## The Right Approach

**Rust should call the C++ CLI binary, NOT PDFium directly.**

### Why?

**C++ CLI has everything working:**
- ✅ Form rendering (FPDF_FFLDraw)
- ✅ Multi-process coordination
- ✅ 100% correctness (451 PDFs validated)
- ✅ Optimized performance
- ✅ All callbacks implemented

**Rust calling PDFium directly is problematic:**
- ❌ C callbacks are hard in Rust
- ❌ Have to duplicate all C++ CLI logic
- ❌ Forms don't work (proven - page 10 MD5 mismatch)
- ❌ Maintenance burden (two implementations)

---

## Architecture

```
┌──────────────────────────────────────┐
│  User Rust Application               │
└────────────┬─────────────────────────┘
             │
             ▼
┌──────────────────────────────────────┐
│  Rust API Crate (pdfium_fast)        │  ← v0.3.0 work
│  - Spawns pdfium_cli subprocess      │
│  - Parses stdout/stderr              │
│  - Error handling                    │
│  - Idiomatic Rust interface          │
└────────────┬─────────────────────────┘
             │ subprocess
             ▼
┌──────────────────────────────────────┐
│  C++ CLI Binary (pdfium_cli)         │  ← Optimized in v0.2.0
│  - Form rendering (FPDF_FFLDraw)     │
│  - Multi-process coordination        │
│  - Optimized performance             │
└────────────┬─────────────────────────┘
             │ C API
             ▼
┌──────────────────────────────────────┐
│  PDFium Core (Google upstream)       │
└──────────────────────────────────────┘
```

---

## Example Rust API (v0.3.0)

```rust
use pdfium_fast::Document;

// High-level API
let doc = Document::open("input.pdf")?;

// Extract text
let text = doc.extract_text()?;

// Render thumbnails (calls: pdfium_cli --thumbnail render-pages)
doc.render_thumbnails("output/", 150)?;

// Render high quality (calls: pdfium_cli --dpi 300 render-pages)
doc.render_pages("output/", 300)?;

// Fast mode for large documents
doc.render_thumbnails_fast("output/", 8)?;  // 8 workers
```

### Implementation

```rust
pub struct Document {
    pdf_path: PathBuf,
    cli_path: PathBuf,
}

impl Document {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        Ok(Document {
            pdf_path: path.as_ref().to_path_buf(),
            cli_path: find_pdfium_cli()?,
        })
    }

    pub fn render_thumbnails(&self, output_dir: &str, dpi: u32) -> Result<()> {
        let output = Command::new(&self.cli_path)
            .args(&[
                "--thumbnail",
                "--dpi", &dpi.to_string(),
                "render-pages",
                self.pdf_path.to_str().unwrap(),
                output_dir
            ])
            .output()?;

        if !output.status.success() {
            return Err(Error::from_stderr(output.stderr));
        }

        Ok(())
    }

    pub fn extract_text(&self) -> Result<String> {
        let temp_file = tempfile::NamedTempFile::new()?;

        let output = Command::new(&self.cli_path)
            .args(&[
                "extract-text",
                self.pdf_path.to_str().unwrap(),
                temp_file.path().to_str().unwrap()
            ])
            .output()?;

        if !output.status.success() {
            return Err(Error::from_stderr(output.stderr));
        }

        // Read UTF-32 LE output
        let bytes = std::fs::read(temp_file.path())?;
        let text = decode_utf32le(&bytes)?;
        Ok(text)
    }
}
```

---

## Benefits of This Approach

### 1. Forms Work (Proven)

C++ CLI renders forms 100% correctly:
- 0100pages page 10: ✅ MD5 matches
- web_041: ✅ MD5 matches
- All 9 form PDFs: ✅ validated

Rust just calls the working tool → forms work automatically!

### 2. No Duplicate Logic

**Single source of truth:** C++ CLI
- All optimizations happen once
- All bug fixes happen once
- One codebase to maintain

### 3. Performance

C++ CLI is being optimized (v0.2.0):
- Single-core improvements
- Fast thumbnail mode
- Smart scanned PDF detection

Rust gets all optimizations for free!

### 4. Safety

**Rust layer provides:**
- Type safety
- Error handling
- Resource cleanup (temp files, etc.)
- Async/await support (spawn CLI in background)

**C++ layer provides:**
- Raw performance
- PDFium integration
- Form rendering
- Multi-process coordination

### 5. Cross-Platform

**C++ CLI compiles to:**
- macOS: pdfium_cli
- Linux: pdfium_cli
- Windows: pdfium_cli.exe

**Rust API detects platform:**
```rust
fn find_pdfium_cli() -> Result<PathBuf> {
    let binary_name = if cfg!(windows) {
        "pdfium_cli.exe"
    } else {
        "pdfium_cli"
    };

    // Search in: current dir, system path, package resources
    locate_binary(binary_name)
}
```

---

## What NOT to Do

### ❌ Don't Call PDFium Directly from Rust

**Problems we discovered:**
- Forms don't render (missing FPDF_FFLDraw)
- C callbacks are complex in Rust
- Duplicate all C++ CLI logic
- Hard to maintain

**Evidence:**
```
Rust library (render_pages.rs):
  page 10 MD5: 7739836e58d8462b8366ac5a471771f7 ❌

C++ CLI (pdfium_cli):
  page 10 MD5: 204c77ed71ffcb207f4456546e21fa10 ✅

Upstream (pdfium_test):
  page 10 MD5: 204c77ed71ffcb207f4456546e21fa10 ✅
```

---

## Implementation Plan (v0.3.0)

### Phase 1: Basic Rust API

1. Create `rust/pdfium-fast/` crate (new)
2. Implement Document struct
3. Add extract_text() method (calls pdfium_cli)
4. Add render_pages() method (calls pdfium_cli)
5. Test forms work

### Phase 2: Advanced Features

1. Async API (tokio support)
2. Streaming text extraction
3. Progress callbacks
4. Error types and handling

### Phase 3: Packaging

1. Publish to crates.io
2. Bundle pdfium_cli binary
3. Documentation and examples
4. CI/CD for releases

---

## Current Status

**v0.1.0-alpha:**
- C++ CLI works perfectly (forms, performance, correctness)
- Rust library exists but not used in production
- Tests call C++ CLI via subprocess

**v0.2.0-beta (in progress):**
- Optimize C++ CLI
- Add thumbnail mode
- Add DPI control
- Rust wrapper waits

**v0.3.0 (planned):**
- Create clean Rust API wrapping C++ CLI
- Publish to crates.io
- Forms work (via C++ CLI)

---

## Summary

**Correct approach:** Rust → subprocess → C++ CLI → PDFium

**Why it works:**
- ✅ Forms render (C++ has FPDF_FFLDraw)
- ✅ All optimizations benefit Rust users
- ✅ Single source of truth
- ✅ Simpler Rust code (just subprocess management)

**The Rust library code (render_pages.rs) is NOT production** - it's just reference code showing how to call PDFium. The real Rust API will wrap the C++ CLI binary.
