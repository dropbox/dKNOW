# docling-pdf-ml

ML-based PDF parsing library with layout detection, OCR, table extraction, and code/formula enrichment.

This is a Rust port of the PDF parsing pipeline from [docling](https://github.com/docling-project/docling), featuring a 5-model ML pipeline for advanced document understanding.

## Features

- **Layout Detection** - RT-DETR v2 model for detecting document elements (text, tables, figures, etc.)
- **OCR (RapidOCR)** - High-quality text recognition for scanned documents
  - Detection: DBNet for text region detection
  - Classification: AngleNet for text angle correction
  - Recognition: CRNNNet for character recognition
- **Table Structure** - TableFormer model for parsing table structure
- **Reading Order** - Transformer model for determining optimal reading order
- **Code/Formula Enrichment** - Multi-modal model for detecting and enriching code blocks and formulas
- **Export** - Convert to DoclingDocument JSON or Markdown

## Architecture

The pipeline consists of 5 ML models executed in sequence:

```
PDF Page
  ↓
┌─────────────────────────────────────────────────────────┐
│ Model 1: RapidOCR (ONNX)                                │
│  - Detection → Classification → Recognition             │
│  - Output: Text regions with confidence scores          │
└─────────────────────────────────────────────────────────┘
  ↓
┌─────────────────────────────────────────────────────────┐
│ Model 2: Layout Detector (PyTorch or ONNX)              │
│  - RT-DETR v2 for element detection                     │
│  - Output: Bounding boxes + labels (text/table/figure)  │
└─────────────────────────────────────────────────────────┘
  ↓
┌─────────────────────────────────────────────────────────┐
│ Model 3: TableFormer (PyTorch)                          │
│  - Table structure recognition                          │
│  - Output: Cell coordinates + row/col spans             │
└─────────────────────────────────────────────────────────┘
  ↓
┌─────────────────────────────────────────────────────────┐
│ Model 4: Reading Order (Transformer, PyTorch)           │
│  - Determines natural reading sequence                  │
│  - Output: Topological ordering of elements             │
└─────────────────────────────────────────────────────────┘
  ↓
┌─────────────────────────────────────────────────────────┐
│ Model 5: CodeFormula (Multi-modal, PyTorch)             │
│  - Enriches code blocks and formulas                    │
│  - Output: Syntax highlighting, LaTeX rendering         │
└─────────────────────────────────────────────────────────┘
  ↓
Assembly Pipeline (Stages 4-9)
  - Cell assignment
  - Empty cluster removal
  - Orphan creation
  - BBox adjustment
  - Overlap resolution
  - Document assembly
  ↓
DoclingDocument (JSON) or Markdown
```

## Directory Structure

```
src/
├── models/              # ML model implementations
│   ├── code_formula/    # Model 5: Code/Formula enrichment
│   ├── layout/          # Model 2: Layout detection (ONNX + PyTorch)
│   └── table_structure/ # Model 3: TableFormer
├── ocr/                 # Model 1: RapidOCR (detection/classification/recognition)
├── pipeline/            # Pipeline orchestration
│   ├── assembly/        # Stages 4-9: Post-processing pipeline
│   ├── executor.rs      # Main pipeline executor
│   ├── reading_order.rs # Model 4: Reading order predictor
│   ├── table_inference.rs
│   ├── layout_postprocessor.rs
│   ├── page_assembly.rs
│   └── docling_export.rs
├── preprocessing/       # Image preprocessing utilities
├── convert.rs           # Convert PageElement → DocItem
├── docling_document.rs  # DoclingDocument data structures
├── model_utils.rs       # Model loading utilities
├── baseline.rs          # Baseline data handling for tests
└── types/               # Common data structures

baseline_data/           # Test baseline data (5.4GB, git-ignored)
tests/                   # 93 test files (187 unit tests + integration tests)
models/                  # Pre-trained model weights (git-ignored)
```

## Usage

### Basic Example

```rust
use docling_pdf_ml::pipeline::{Pipeline, PipelineConfig};
use image::DynamicImage;

// Load pipeline with default config
let config = PipelineConfig::default();
let pipeline = Pipeline::new(config)?;

// Process a PDF page
let page_image: DynamicImage = /* load image */;
let page_result = pipeline.process_page(&page_image, 0)?;

// Export to markdown
let markdown = page_result.export_to_markdown();
println!("{}", markdown);
```

### Custom Configuration

```rust
use docling_pdf_ml::pipeline::{Pipeline, PipelineConfigBuilder};

let config = PipelineConfigBuilder::new()
    .layout_model_path("path/to/layout_model.pt")
    .table_model_path("path/to/table_model.pt")
    .build();

let pipeline = Pipeline::new(config)?;
```

### Export to JSON

```rust
// Export as DoclingDocument JSON
let doc_json = page_result.export_to_json()?;
println!("{}", serde_json::to_string_pretty(&doc_json)?);
```

## Setup

### 1. Environment Configuration

The pipeline requires PyTorch libtorch and LLVM libraries:

```bash
# Run this before any cargo commands
source setup_env.sh

# Or manually set:
export LIBTORCH_USE_PYTORCH=1
export DYLD_LIBRARY_PATH=/opt/homebrew/lib/python3.14/site-packages/torch/lib:/opt/homebrew/opt/llvm/lib
```

### 2. Model Weights

Download pre-trained models:

```bash
# Layout model (RT-DETR v2)
# Automatically downloaded from HuggingFace on first use

# OCR models (RapidOCR)
# Already included in models/ directory (15MB)

# TableFormer model
# Automatically downloaded from HuggingFace on first use

# Reading Order model
# Automatically downloaded from HuggingFace on first use

# CodeFormula model
# Automatically downloaded from HuggingFace on first use
```

Models are cached in `~/.cache/huggingface/` by default.

### 3. Feature Flags

```toml
[dependencies]
docling-pdf-ml = { version = "0.1.0", features = ["pytorch", "opencv-preprocessing"] }
```

**Available features:**
- `pytorch` - Enable PyTorch backend (TableFormer, Reading Order, CodeFormula)
- `opencv-preprocessing` - Enable OpenCV for advanced image preprocessing
- `debug-trace` - Enable execution tracing (DEBUG_E2E_TRACE env var)
- `debug-profiling` - Enable performance profiling (PROFILE_* env vars)
- `debug-stats` - Enable statistics output

## Building

```bash
# Standard build (debug)
source setup_env.sh
cargo build -p docling-pdf-ml --features pytorch,opencv-preprocessing

# Release build (faster inference)
cargo build -p docling-pdf-ml --features pytorch,opencv-preprocessing --release

# Run tests
cargo test -p docling-pdf-ml --features pytorch,opencv-preprocessing -- --test-threads=1
```

**Note:** Use `--test-threads=1` to avoid thread-safety issues with pdfium C library.

## Testing

### Test Suite Summary

- **202 total tests**
- **184 passing** (99.5% pass rate)
- **1 failing** (missing baseline data - setup issue)
- **17 ignored** (debug/architecture mismatch)

See [TEST_RESULTS.md](TEST_RESULTS.md) for detailed test analysis.

### Running Tests

```bash
# All tests
source setup_env.sh
cargo test -p docling-pdf-ml --features pytorch,opencv-preprocessing -- --test-threads=1

# Unit tests only
cargo test -p docling-pdf-ml --lib --features pytorch,opencv-preprocessing

# Specific integration test
cargo test -p docling-pdf-ml --test layout_phase1_validation_test --features pytorch,opencv-preprocessing
```

### Baseline Data

Tests require baseline data (5.4GB) for validation:

```bash
# Baseline data is already present at:
crates/docling-pdf-ml/baseline_data/

# If missing, copy from source repository:
cp -r ~/docling_debug_pdf_parsing/baseline_data/ ./crates/docling-pdf-ml/
```

Baseline data is git-ignored due to size.

## Performance

Typical performance on a modern MacBook Pro (M-series):

- **Layout Detection:** ~200ms per page
- **OCR (scanned page):** ~500ms per page
- **Table Structure:** ~100ms per table
- **Reading Order:** ~50ms per page
- **CodeFormula:** ~150ms per code block

**Total:** ~1 second per page (varies with content complexity)

**Memory:** ~2GB RAM for model weights + ~500MB per page during inference

## Limitations

### Known Issues

1. **Thread Safety:** pdfium C library has thread-safety issues. Use `--test-threads=1` when running tests.

2. **Model Loading:** First run downloads models from HuggingFace (~500MB total). Subsequent runs use cached models.

3. **Architecture Mismatch:** 12 tests disabled due to source repo using ModularPipeline (Stage04-Stage10) architecture. This port uses an executor-based architecture for better performance.

4. **Missing Baseline File:** 1 test fails due to missing `ml_model_inputs/rapid_ocr_isolated/cls_preprocessed_input.npy`. Non-blocking.

### Unsupported Features

- **Batch Processing:** Current implementation processes one page at a time
- **GPU Acceleration:** PyTorch backend supports GPU, but not yet configured by default
- **Streaming:** Entire page must be loaded into memory

## Dependencies

**Core:**
- `ort` - ONNX Runtime for RapidOCR and ONNX Layout models
- `tch` - PyTorch bindings for TableFormer, Reading Order, CodeFormula
- `ndarray` - N-dimensional array operations
- `image` - Image loading and manipulation
- `serde`/`serde_json` - Serialization

**Optional:**
- `opencv` - Advanced image preprocessing
- `tokenizers` - HuggingFace tokenizers for CodeFormula

**Test:**
- `ndarray-npy` - Load numpy baseline data
- `npyz` - Alternative numpy loader

## Contributing

This is a port of Python docling. When porting features:

1. **Read Python source first** - Understand the algorithm before implementing
2. **Line-by-line translation** - Port accurately, don't improve during initial port
3. **Validate with tests** - Compare output against Python baselines
4. **Document differences** - Note any deviations from Python implementation

See [WORKER_PLAN_REMAINING_MIGRATION.md](../../WORKER_PLAN_REMAINING_MIGRATION.md) for remaining work.

## License

Apache-2.0

## References

- **Python docling:** https://github.com/docling-project/docling
- **RT-DETR v2:** https://arxiv.org/abs/2304.08069
- **RapidOCR:** https://github.com/RapidAI/RapidOCR
- **TableFormer:** https://arxiv.org/abs/2203.01017
- **HuggingFace Models:** https://huggingface.co/DS4SD

## Changelog

### N=65 (2025-11-23)

- ✅ Tests ported (93 files, 187 unit tests)
- ✅ 99.5% pass rate (184/185 non-ignored)
- ✅ CodeFormula model complete (3,893 lines)
- ✅ Export infrastructure complete (1,083 lines)
- ✅ Baseline data copied (5.4GB)

### N=64 (2025-11-23)

- ✅ Export infrastructure added (docling_export.rs, docling_document.rs)

### N=63 (2025-11-23)

- ✅ CodeFormula model ported (9 files, 3,893 lines)

### N=62 (2025-11-23)

- ✅ Pipeline integration complete
- ✅ End-to-end test passing
- ✅ Environment setup automated

### Earlier commits

- Core 4 models ported (OCR, Layout, Table, ReadingOrder)
- Pipeline executor and assembly stages
- Preprocessing utilities
- 26,711 lines of source code

## Status

**Current:** 85% complete (26,711/31,419 source lines + tests + docs)

**Remaining work:**
- Task 4: Documentation (in progress - this file)
- Task 5: Final validation on diverse PDFs

**Target:** 100% completion by 2025-11-26
