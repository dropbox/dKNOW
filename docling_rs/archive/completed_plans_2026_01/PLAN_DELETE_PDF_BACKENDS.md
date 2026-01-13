# PLAN: Delete All PDF Backends Except THE ONE

**Priority:** CRITICAL
**Goal:** ONE working PDF pipeline, ZERO alternatives

---

## THE CORRECT PIPELINE: `pdfium-fast-ml`

**Components:**
| Component | Library | Purpose |
|-----------|---------|---------|
| PDF Rendering | pdfium_fast (72x faster) | Text extraction, page rendering |
| Layout Detection | ONNX Runtime | Detect text/table/figure regions |
| OCR | RapidOCR (ONNX) | Extract text from images |
| Table Structure | TableFormer (PyTorch) | Parse table rows/columns |
| Code/Formula | CodeFormula (PyTorch) | Extract code and math |

**This is the FULL EVERYTHING COMPLETE package.**

**Requirements:**
- `~/pdfium_fast` built locally
- PyTorch/libtorch libraries
- ONNX models in `models/` directory

---

## CURRENT MESS

### PDF Backend Files (docling-backend/src/)
| File | Size | Purpose | DELETE? |
|------|------|---------|---------|
| `pdf_fast.rs` | 106KB | pdfium_fast backend | **KEEP** |
| `pdf.rs` | 92KB | pdfium-render backend | **DELETE** |
| `pdfium_adapter.rs` | 269KB | Shared adapter | AUDIT |
| `pdf_constants.rs` | 1.7KB | Constants | KEEP |

### Feature Flags (docling-backend/Cargo.toml)
| Feature | What it does | DELETE? |
|---------|--------------|---------|
| `pdfium-fast` | Base fast library | **KEEP** |
| `pdfium-render` | Base slow library | **DELETE** |
| `pdf-ml` | render + pytorch + opencv | **DELETE** |
| `pdf-ml-onnx` | render + onnx + opencv | **DELETE** |
| `pdf-ml-simple` | render + basic onnx | **DELETE** |
| `pdf-ml-pytorch` | render + pytorch | **DELETE** |
| `pdfium-fast-ml` | fast + pytorch | **KEEP (make default)** |
| `pdfium-fast-ml-pytorch` | duplicate? | **DELETE** |

### Feature Flags (docling-cli/Cargo.toml)
| Feature | DELETE? |
|---------|---------|
| `pdf-ml-simple` (DEFAULT) | **DELETE** |
| `pdf-ml` | **DELETE** |
| `pdf-ml-onnx` | **DELETE** |
| `pdf-ml-pytorch` | **DELETE** |
| `pdfium-fast-ml` | **KEEP (make default)** |
| `pdfium-fast-ml-pytorch` | **DELETE** |

---

## DELETION PLAN

### Step 1: Delete pdf.rs Backend

```bash
# Remove the slow pdfium-render backend
rm crates/docling-backend/src/pdf.rs

# Remove references in mod.rs
# Edit crates/docling-backend/src/lib.rs to remove pdf module
```

### Step 2: Clean Up Cargo.toml (docling-backend)

**BEFORE:**
```toml
[features]
default = ["pdfium-fast"]
pdfium-fast = ["dep:pdfium-sys"]
pdfium-render = ["dep:pdfium-render"]
pdf-ml-onnx = ["pdfium-render", "docling-pdf-ml", "docling-pdf-ml/opencv-preprocessing"]
pdf-ml = ["pdfium-render", "docling-pdf-ml", "docling-pdf-ml/pytorch", "docling-pdf-ml/opencv-preprocessing"]
pdf-ml-simple = ["pdfium-render", "docling-pdf-ml"]
pdf-ml-pytorch = ["pdfium-render", "docling-pdf-ml", "docling-pdf-ml/pytorch"]
pdfium-fast-ml = ["pdfium-fast", "docling-pdf-ml", "docling-pdf-ml/pytorch"]
pdfium-fast-ml-pytorch = ["pdfium-fast", "docling-pdf-ml", "docling-pdf-ml/pytorch"]
```

**AFTER:**
```toml
[features]
default = ["pdf"]
pdf = ["dep:pdfium-sys", "docling-pdf-ml", "docling-pdf-ml/pytorch"]
```

### Step 3: Clean Up Cargo.toml (docling-cli)

**BEFORE:**
```toml
[features]
default = ["pdf-ml-simple"]
pdf-ml-onnx = ["docling-backend/pdf-ml-onnx"]
pdf-ml = ["docling-backend/pdf-ml"]
pdf-ml-simple = ["docling-backend/pdf-ml-simple"]
pdf-ml-pytorch = ["docling-backend/pdf-ml-pytorch"]
pdfium-fast-ml = ["docling-backend/pdfium-fast-ml"]
pdfium-fast-ml-pytorch = ["docling-backend/pdfium-fast-ml-pytorch"]
```

**AFTER:**
```toml
[features]
default = ["pdf"]
pdf = ["docling-backend/pdf"]
```

### Step 4: Clean Up Cargo.toml (docling-core)

**BEFORE:**
```toml
docling-backend = { ..., features = ["pdfium-fast-ml"] }
```

**AFTER:**
```toml
docling-backend = { ..., features = ["pdf"] }
```

### Step 5: Delete Dependencies

Remove from docling-backend/Cargo.toml:
```toml
# DELETE these lines:
pdfium-render = { version = "0.8", optional = true }
```

Remove from docling-cli/Cargo.toml:
```toml
# DELETE this line:
pdfium-render = "0.8"
```

### Step 6: Update Converter Code

Edit `crates/docling-backend/src/converter.rs`:
- Remove all `#[cfg(feature = "pdfium-render")]` blocks
- Remove all `#[cfg(feature = "pdf-ml-simple")]` etc blocks
- Keep only `pdfium-fast` code path
- Remove PdfBackend struct (only keep PdfFastBackend)

### Step 7: Update lib.rs

Edit `crates/docling-backend/src/lib.rs`:
- Remove `pub mod pdf;`
- Keep `pub mod pdf_fast;`
- Simplify exports

### Step 8: Delete Examples Using Old Backend

```bash
rm crates/docling-backend/examples/test_pdf_ml_e2e.rs
rm crates/docling-backend/examples/explore_pdfium_api.rs
```

### Step 9: Clean Up Environment Variables

**DELETE references to:**
- `USE_HYBRID_SERIALIZER` - no longer needed
- `SKIP_OUTPUT_VALIDATION` - tests should always validate
- Any PDF backend selection env vars

**KEEP:**
- `LIBTORCH_USE_PYTORCH` - needed for PyTorch
- `DYLD_LIBRARY_PATH` - needed for libraries

### Step 10: Update CLAUDE.md

Remove all documentation about:
- Multiple PDF backends
- Feature flag selection
- "Use pdf-ml-simple for easy build"

Replace with:
- "PDF uses pdfium_fast + PyTorch. Requires ~/pdfium_fast to be built."

---

## FILES TO DELETE

```
crates/docling-backend/src/pdf.rs
crates/docling-backend/examples/test_pdf_ml_e2e.rs
crates/docling-backend/examples/explore_pdfium_api.rs
```

## FILES TO EDIT

```
crates/docling-backend/Cargo.toml
crates/docling-backend/src/lib.rs
crates/docling-backend/src/converter.rs
crates/docling-cli/Cargo.toml
crates/docling-core/Cargo.toml
CLAUDE.md
```

---

## VERIFICATION

After cleanup, these commands should work:

```bash
# Build (no feature flags needed)
cargo build --release

# Test PDF
./target/release/docling convert test-corpus/pdf/2305.03393v1-pg9.pdf -o /tmp/test.md

# Output should match groundtruth
diff /tmp/test.md test-corpus/groundtruth/docling_v2/2305.03393v1-pg9.md

# Run tests (no env vars needed)
cargo test test_canon_pdf
```

---

## ROLLBACK

If something breaks:
```bash
git checkout HEAD~1 -- crates/
```

---

**EXECUTE THIS PLAN. DELETE THE MESS.**
