# C++ Integration Options for Document Parsers

**Date:** 2025-11-10
**Context:** Worker can use C++ (via FFI) in addition to Rust

---

## Why C++ Helps

**Mature Libraries Available:**
- OCR: Tesseract (C++)
- Office formats: LibreOffice (C++)
- Image processing: OpenCV (C++)
- Legacy formats: Various C/C++ libraries

**FFI Pattern (Already Used):**
- PDF backend uses pdfium (C++) via pdfium-render ✅
- Proven pattern, worker knows how to use it

---

## Formats That Would Benefit from C++

### 🔴 HIGH IMPACT - Current Blockers

| Format | Library | Type | Benefit |
|--------|---------|------|---------|
| **PNG** | Tesseract | OCR | 4 canonical tests, Python-only |
| **JPEG** | Tesseract | OCR | 4 canonical tests, Python-only |
| **TIFF** | Tesseract | OCR | 4 canonical tests, Python-only |
| **WEBP** | Tesseract | OCR | 1 canonical test, Python-only |

**Strategy:** Use `tesseract-sys` or `leptess` Rust wrapper
- Tesseract is C++ OCR engine
- `leptess` crate already in dependencies!
- Could unblock 13 image tests immediately

---

### 🟡 MEDIUM IMPACT - Complex Formats

| Format | Library | Type | Benefit |
|--------|---------|------|---------|
| **DOC** | libmspack or antiword | Legacy | No canonical tests, but useful |
| **RTF** | Current Rust impl | - | Already done in Rust (N=263) |
| **PUB** | LibreOffice | Conversion | No tests, low priority |
| **VSDX** | LibreOffice | Conversion | No tests, specialized |
| **OneNote** | Complex | Proprietary | Very difficult, defer |

**Strategy for legacy MS:** C++ FFI if needed, but low priority (no canonical tests)

---

### 🟢 LOW IMPACT - Already Have Rust Solutions

| Format | Status | C++ Option |
|--------|--------|------------|
| **PPTX** | Python-only, 5 tests | Could use LibreOffice C++, but Rust XML parsing sufficient |
| **JATS** | Python-only, 5 tests | Pure Rust with quick-xml sufficient |
| **Archives** | ✅ Done in Rust | No need for C++ |
| **Email** | ✅ Done in Rust | No need for C++ |
| **Ebooks** | ✅ Done in Rust | No need for C++ |

---

## Recommended C++ Integrations

### IMMEDIATE: Image OCR (Tesseract)

**Files:** png.rs, jpeg.rs, tiff.rs, webp.rs (need to create)

**Approach:**
```rust
// Use leptess crate (already in dependencies!)
use leptess::LepTess;

pub struct PngBackend {
    ocr: LepTess,
}

impl DocumentBackend for PngBackend {
    fn parse_file(&self, path: &Path, options: &BackendOptions)
        -> Result<Document, DoclingError> {
        // 1. Load image
        let img = image::open(path)?;

        // 2. OCR with Tesseract (C++)
        self.ocr.set_image(&img)?;
        let text = self.ocr.get_utf8_text()?;

        // 3. Create DocItems
        let doc_items = vec![
            DocItem::Picture {
                image: Some(ImageRef { ... }),
                ...
            },
            DocItem::Text {
                text: text,
                orig: text.clone(),
                ...
            }
        ];

        // 4. Serialize
        let markdown = serialize_to_markdown(&doc_items);

        Ok(Document {
            markdown,
            content_blocks: Some(doc_items),
            ...
        })
    }
}
```

**Benefit:** Unblocks 13 canonical image tests

**Effort:** 3-4 commits (one for PNG/JPEG/TIFF together, one for WEBP)

---

### OPTIONAL: LibreOffice for Complex Formats

**Formats:** DOC, PUB, VSDX (legacy/specialized, no canonical tests)

**Approach 1: FFI to LibreOffice SDK**
```rust
// Direct C++ bindings
#[cxx::bridge]
mod ffi {
    unsafe extern "C++" {
        include!("libreoffice/converter.h");
        fn convert_doc_to_docx(input: &str, output: &str) -> bool;
    }
}

impl DocBackend {
    fn parse_file(&self, path: &Path) -> Result<Document> {
        // Convert DOC → DOCX via LibreOffice C++
        let docx_path = PathBuf::from("/tmp/converted.docx");
        ffi::convert_doc_to_docx(path.to_str(), docx_path.to_str())?;

        // Parse DOCX with existing Rust backend
        let docx_backend = DocxBackend::new();
        docx_backend.parse_file(&docx_path, options)
    }
}
```

**Approach 2: Shell out to LibreOffice CLI**
```rust
// Simpler but requires LibreOffice installed
impl DocBackend {
    fn parse_file(&self, path: &Path) -> Result<Document> {
        // Convert via soffice --headless
        let output = Command::new("soffice")
            .args(&["--headless", "--convert-to", "docx", path.to_str()])
            .output()?;

        // Parse result with DOCX backend
        // ...
    }
}
```

**Benefit:** Could support legacy formats
**Priority:** LOW (no canonical tests exist for these)

---

## Current Dependencies Already Available

**Check Cargo.toml:**
```toml
# OCR (C++ Tesseract wrapper)
leptess = "0.13"  # ✅ Already present!

# Image processing
image = "0.25"    # ✅ Already present!
```

**This means:** Worker can use Tesseract OCR RIGHT NOW without adding dependencies!

---

## Strategic Recommendation

### IMMEDIATE (Use Modern OCR for Images)

**Priority 1:** Implement image backends with modern OCR
- PNG, JPEG, TIFF, WEBP (13 canonical tests)
- **Use PaddleOCR or RapidOCR** (more modern than Tesseract)
- OCR is primarily handled by PDF system, images just need basic text extraction
- Unblocks Python-only image tests

**OCR Solution: RapidOCR v5 with PaddleOCR Models**

**Implementation Requirements:**
1. **RapidOCR v5** - Uses PaddleOCR models
   - Modern, accurate, fast
   - v5 is latest architecture

2. **ONNX Runtime** - For GPU acceleration
   - Already in workspace deps: `ort = "1.16"`
   - ONNX inference engine with GPU support
   - Portable across platforms

3. **macOS GPU Support** - Metal acceleration
   - Use ONNX Runtime with CoreML/Metal backend
   - Leverage macOS GPU for speed
   - Feature flag: `#[cfg(target_os = "macos")]`

**Architecture: Platform-Specific Backends**

**CRITICAL:** ONNX does NOT support macOS Metal. Need two backends:

1. **Linux/Windows:** ONNX Runtime (CPU/CUDA)
2. **macOS:** Research required - PyTorch Metal, CoreML, or other GPU option

**WORKER RESEARCH TASK:** Determine best macOS GPU inference option for RapidOCR v5:
- Options: PyTorch with Metal, CoreML, MLX, or other
- Requirement: Must use macOS GPU (Apple Silicon)
- Constraint: Must support RapidOCR v5 / PaddleOCR models

```rust
// Platform-specific OCR backend selection
pub struct RapidOCRv5 {
    #[cfg(target_os = "macos")]
    backend: PyTorchBackend,  // Metal GPU on macOS

    #[cfg(not(target_os = "macos"))]
    backend: OnnxBackend,     // ONNX on Linux/Windows
}

#[cfg(target_os = "macos")]
struct PyTorchBackend {
    // PyTorch with Metal backend for macOS
    // Use tch-rs crate (Rust bindings for PyTorch)
}

#[cfg(not(target_os = "macos"))]
struct OnnxBackend {
    session: ort::Session,  // ONNX Runtime
}

impl RapidOCRv5 {
    pub fn new() -> Result<Self> {
        #[cfg(target_os = "macos")]
        {
            // macOS: Use PyTorch with Metal acceleration
            let backend = PyTorchBackend::new()?;
            Ok(Self { backend })
        }

        #[cfg(not(target_os = "macos"))]
        {
            // Linux/Windows: Use ONNX Runtime
            let session = ort::Session::builder()?
                .commit_from_file("rapid_ocr_v5.onnx")?;
            Ok(Self { backend: OnnxBackend { session } })
        }
    }

    pub fn recognize(&self, image: &DynamicImage) -> Result<String> {
        #[cfg(target_os = "macos")]
        return self.backend.recognize_metal(image);  // PyTorch Metal

        #[cfg(not(target_os = "macos"))]
        return self.backend.recognize_onnx(image);   // ONNX
    }
}
```

**Dependencies Required:**

**Linux/Windows:**
```toml
ort = "1.16"  # ✅ Already in deps - ONNX Runtime
```

**macOS:**
```toml
tch = "0.16"  # PyTorch Rust bindings (Metal support)
```

**Benefit:**
- ✅ Modern OCR (RapidOCR v5 with PaddleOCR models)
- ✅ GPU accelerated on both platforms:
  - Linux/Windows: ONNX Runtime (CUDA on Linux)
  - macOS: PyTorch Metal (Apple Silicon GPU)
- ✅ Platform-optimized performance

**Effort:** 4-5 commits (model integration + 4 image backends)
**Benefit:** 13 canonical tests move from Python to Rust+C++

**Note:** ONNX Runtime already in dependencies, just need RapidOCR v5 models

### LATER (C++ for Legacy Formats)

**Priority 2:** Legacy MS formats (DOC, PUB, etc.)
- Use LibreOffice C++ or conversion tools
- Only if customer demand exists
- No canonical tests currently

---

## Updated Worker Directive

**WORKER: C++ is approved for:**

1. **Image OCR** (HIGH PRIORITY - 13 canonical tests)
   - **Use PaddleOCR or RapidOCR** (modern solutions)
   - Do NOT use Tesseract (legacy)
   - PNG, JPEG, TIFF, WEBP backends
   - Note: OCR primarily handled by PDF system, images need basic text extraction

2. **Legacy/Complex formats** (LOW PRIORITY - 0 canonical tests)
   - LibreOffice C++ bindings
   - Only if needed (pure Rust preferred when feasible)

**Still must:**
- Fix WebVTT (pure Rust, no C++ needed)
- Implement PPTX (pure Rust XML parsing sufficient)
- Implement JATS (pure Rust XML parsing sufficient)

---

## Manager Response

**Q: Does it help that workers can use C++?**

**A: YES! Especially for:**
- **Image OCR** (13 canonical tests) - Tesseract C++ already in deps via `leptess`
- **Legacy formats** (future) - LibreOffice C++ if needed

**Impact:**
- Can implement image backends immediately (unblock 13 tests)
- Don't need to port OCR models to pure Rust
- Proven FFI pattern already exists (PDF uses C++)

**Current blocker (WebVTT) doesn't need C++** - it's a text format bug.

---

**WORKER: Use C++ (Tesseract) for image backends after fixing WebVTT.**
