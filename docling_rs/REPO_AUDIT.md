# Docling++ Repository Audit

**Generated:** 2026-01-03
**Total Rust Lines:** 316,956

---

## Binaries & Tools

### Production Binaries (target/release/)

| Binary | Purpose | Status |
|--------|---------|--------|
| `docling` | Main CLI for document conversion | ✅ Production |
| `docling-pdf` | PDF-specific conversion tool | ✅ Production |
| `dlviz-screenshot` | Visualize PDF with ML bboxes | ✅ Working |
| `dlviz-table` | Visualize table detection | ✅ Working |
| `dlviz-crop` | Export cropped elements | ✅ Working |
| `dlviz-metrics` | Calculate extraction metrics | ⚠️ Needs testing |
| `dlviz-apply-corrections` | Apply corrections, export training data | ⚠️ Needs testing |
| `docling-llm-verify` | LLM-based quality verification | ⚠️ Needs testing |
| `docling-mcp` | MCP server for Claude integration | ⚠️ Needs testing |

### Libraries

| Library | Purpose | Status |
|---------|---------|--------|
| `libdocling_viz_bridge.dylib` | Rust FFI for DoclingViz app | ✅ Built |
| `libhtml2md.dylib` | HTML to Markdown conversion | ✅ Built |

---

## Crates by Size (Lines of Code)

### Large Crates (>5000 lines) - Production Core

| Crate | Lines | Description |
|-------|-------|-------------|
| docling-backend | ~80,000 | Format parsers (PDF, DOCX, HTML, etc.) |
| docling-core | ~45,000 | Core types, serializers, tests |
| docling-pdf-ml | ~25,000 | ML-powered PDF extraction |
| docling-viz-bridge | ~6,000 | Visualization FFI bridge |
| docling-cli | ~4,000 | Command-line interface |

### Medium Crates (1000-5000 lines) - Specialized Features

| Crate | Lines | Description | Status |
|-------|-------|-------------|--------|
| docling-microsoft-extended | ~2,000 | Visio, Publisher, etc. | ✅ Working |
| docling-ebook | ~1,500 | FB2, EPUB handlers | ✅ Working |
| docling-ocr | ~1,400 | OCR integration | ✅ Working |
| docling-quality-verifier | ~1,000 | LLM quality testing | ⚠️ Partial |

### Small Crates (<1000 lines) - Stubs or Simple

| Crate | Status | Notes |
|-------|--------|-------|
| docling-adobe | ⚠️ Stub | IDML/InDesign support |
| docling-apple | ⚠️ Stub | Pages/Numbers/Keynote |
| docling-archive | ✅ Working | ZIP, TAR, RAR extraction |
| docling-audio | ⚠️ Stub | Audio transcription (out of scope) |
| docling-cad | ✅ Working | DXF, STL, OBJ, GLTF |
| docling-calendar | ✅ Working | ICS calendar parsing |
| docling-email | ✅ Working | EML, MBOX parsing |
| docling-genomics | ⚠️ Stub | FASTA, GenBank |
| docling-gps | ✅ Working | GPX, KML parsing |
| docling-latex | ✅ Working | Pure Rust LaTeX parser |
| docling-legacy | ⚠️ Unused | Legacy code archive |
| docling-llm-verify | ⚠️ Partial | LLM quality tool |
| docling-mcp-server | ⚠️ Partial | MCP integration |
| docling-medical | ⚠️ Stub | DICOM medical imaging |
| docling-models | ⚠️ Stub | Model management |
| docling-notebook | ✅ Working | Jupyter notebook parsing |
| docling-opendocument | ✅ Working | ODF format support |
| docling-parse | ⚠️ Legacy | Old parsing code |
| docling-parse-rs | ⚠️ Legacy | Rust parsing experiments |
| docling-parse-sys | ⚠️ Legacy | FFI bindings |
| docling-pipeline | ⚠️ Stub | Pipeline abstraction |
| docling-py | ❌ Removed | Python bridge (archived) |
| docling-svg | ✅ Working | SVG parsing |
| docling-video | ⚠️ Stub | Video transcription (out of scope) |
| docling-xps | ✅ Working | XPS/OXPS parsing |

---

## DoclingViz macOS Application

**Location:** `DoclingViz/DoclingViz/`
**Status:** ✅ Built, needs Phase 4 testing

| Component | Status | Description |
|-----------|--------|-------------|
| Rust FFI Bridge | ✅ Done | 57 tests passing |
| Swift Package | ✅ Done | DoclingBridge wrapper |
| macOS App | ✅ Built | Basic functionality |
| PDF Canvas | ✅ Done | Renders pages |
| Stage Timeline | ✅ Done | 10 stages viewable |
| Overlay Rendering | ✅ Done | Bounding boxes |
| Element Selection | ⚠️ Partial | Click to select |
| Live Editing | 🔴 TODO | Edit labels/bbox |
| Correction Export | 🔴 TODO | COCO/YOLO export |

**Build:**
```bash
cd DoclingViz/DoclingViz
swift build
.build/arm64-apple-macosx/debug/DoclingViz
```

---

## Forgotten/Partial Features

### 1. docling-llm-verify

**Location:** `crates/docling-llm-verify/`
**Purpose:** Use LLMs to verify extraction quality
**Status:** Binary exists, needs testing

```bash
./target/release/docling-llm-verify --help
```

### 2. docling-mcp-server

**Location:** `crates/docling-mcp-server/`
**Purpose:** MCP server for Claude Code integration
**Status:** Partial implementation

### 3. dlviz-metrics

**Purpose:** Calculate extraction quality metrics
**Status:** Built, needs documentation

### 4. dlviz-apply-corrections

**Purpose:** Apply corrections, export to COCO/YOLO training formats
**Status:** Built, needs testing

### 5. docling-quality-verifier

**Location:** `crates/docling-quality-verifier/`
**Purpose:** LLM-based quality scoring
**Status:** 22 tests, partial implementation

### 6. Audio/Video Crates

- `docling-audio` - Stub (out of scope per CLAUDE.md)
- `docling-video` - Stub (out of scope per CLAUDE.md)

### 7. Specialized Format Stubs

- `docling-genomics` - FASTA/GenBank (stub)
- `docling-medical` - DICOM (stub, ~1900 lines but may be incomplete)
- `docling-models` - Model management (stub)

---

## Design Documents

| Document | Status | Description |
|----------|--------|-------------|
| `DESIGN_MACOS_PDF_VISUALIZER.md` | ✅ Implemented | DoclingViz app spec |
| `docs/ANNOTATION_FORMAT.md` | Active | Correction JSON spec |
| `ROADMAP.md` | Active | Current priorities |

---

## Archive Contents

| Directory | Contents |
|-----------|----------|
| `archive/directives_2025_12/` | December 2025 worker directives |
| `archive/directives_2025-11/` | November 2025 worker directives |
| `archive/old_reports_N2505/` | Historical reports |
| `archive/deprecated_code_N2867/` | Deprecated implementations |
| `archive/layout_detection_resolved_N3616/` | Resolved layout bugs |

---

## Test Coverage Summary

| Package | Tests | Status |
|---------|-------|--------|
| docling-backend | 3032 | ✅ 100% |
| docling-core | 202 | ✅ 100% |
| docling-pdf-ml | 214 | ✅ 100% |
| docling-viz-bridge | 81 | ✅ 100% |
| docling-quality-verifier | 22 | ✅ 100% |
| **Total** | **3556+** | ✅ 100% |

---

## Recommended Actions

### High Priority

1. **Test dlviz-metrics and dlviz-apply-corrections**
   - Verify they work end-to-end
   - Add to README documentation

2. **Test docling-llm-verify**
   - Run against test corpus
   - Document usage

3. **Complete DoclingViz Phase 4**
   - Test with various PDFs
   - Performance profiling
   - Memory leak testing

### Medium Priority

4. **Review docling-mcp-server**
   - Test MCP integration with Claude Code
   - Document setup

5. **Evaluate stub crates**
   - genomics, medical, models
   - Either implement or clearly mark as out of scope

### Low Priority

6. **Clean up legacy crates**
   - docling-parse, docling-parse-rs, docling-parse-sys
   - Move to archive if unused

7. **Review audio/video stubs**
   - Confirm out of scope designation
   - Remove or archive
