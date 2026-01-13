# Phase 0 Architecture - Python Bridge

## Overview

Phase 0 uses a **Python bridge** via PyO3. **All conversion happens in Python** - Rust just wraps Python docling.

```
┌─────────────────────────────────────────┐
│  Rust Integration Tests                │
│  test_canon_pdf_code_and_formula_text() │
└─────────────────────────────────────────┘
              ↓
┌─────────────────────────────────────────┐
│  Rust DocumentConverter (thin wrapper)  │
│  - Wraps Python objects via PyO3        │
│  - Configures Python options            │
│  - Calls Python methods                 │
└─────────────────────────────────────────┘
              ↓ PyO3 bridge
┌─────────────────────────────────────────┐
│  Python docling (100% of work)          │
│  - Reads files                          │
│  - Parses documents                     │
│  - Runs OCR (if do_ocr=True)            │
│  - Exports to markdown                  │
└─────────────────────────────────────────┘
```

## Code Flow

### 1. Test Calls Rust API
```rust
// In tests/integration_tests.rs
let converter = DocumentConverter::with_ocr(false);  // Rust struct
let result = converter.convert("file.pdf")?;         // Rust method call
```

### 2. Rust Creates Python Objects (PyO3)
```rust
// In crates/docling-core/src/converter.rs
Python::with_gil(|py| {
    // Import Python module
    let docling = PyModule::import(py, "docling.document_converter")?;

    // Create Python PdfPipelineOptions object
    let pdf_options = pipeline_options_class.call0()?;
    pdf_options.setattr("do_ocr", false)?;  // Configure Python object

    // Create Python DocumentConverter object
    let py_converter = converter_class.call((), Some(kwargs))?;

    // Store Python object in Rust struct
    Ok(Self { py_converter: py_converter.into() })
})
```

### 3. Rust Calls Python Method
```rust
// In DocumentConverter::convert()
Python::with_gil(|py| {
    // Call Python: result = converter.convert(path)
    let result = self.py_converter.call_method1(py, "convert", (path,))?;

    // This executes 100% Python code:
    // - Python docling reads the file
    // - Python docling parses the document
    // - Python docling runs OCR (if enabled)
    // - Python docling exports to markdown
```

### 4. Rust Extracts Result
```rust
    // Get Python document object
    let py_document = result.getattr(py, "document")?;

    // Call Python: markdown = document.export_to_markdown()
    let markdown_obj = py_document.call_method0(py, "export_to_markdown")?;

    // Extract Python string to Rust String
    let markdown: String = markdown_obj.extract(py)?;

    Ok::<_, DoclingError>(markdown)
})
```

## Configuration: Text vs OCR Mode

### Text-Only Mode (`do_ocr=False`)
```rust
let converter = DocumentConverter::with_ocr(false);
```

**Python configuration:**
```python
# What happens in Python
pipeline_options = PdfPipelineOptions()
pipeline_options.do_ocr = False              # No OCR
pipeline_options.do_table_structure = True   # Parse tables

converter = DocumentConverter(
    format_options={
        InputFormat.PDF: PdfFormatOption(pipeline_options=pipeline_options)
    }
)
```

**Used for:**
- All canonical text tests (`test_canon_*_text`)
- All non-canonical text tests (`test_more_*_text`)
- Matches upstream docling default test configuration

### OCR Mode (`do_ocr=True`)
```rust
let converter = DocumentConverter::with_ocr(true);
```

**Python configuration:**
```python
# What happens in Python
pipeline_options = PdfPipelineOptions()
pipeline_options.do_ocr = True               # Enable OCR
pipeline_options.do_table_structure = True   # Parse tables

converter = DocumentConverter(...)
```

**Used for:**
- All OCR tests (`test_canon_*_ocr`, `test_more_*_ocr`)
- Images (PNG, JPEG, TIFF) always use OCR mode
- Scanned PDFs

## Phase 0 Limitations

**100% Python execution:**
- ❌ No Rust parsing/extraction (yet)
- ❌ No Rust OCR (yet)
- ❌ No Rust layout analysis (yet)
- ✅ Validates Python bridge works
- ✅ Establishes test baseline

**Performance:**
- Same as Python docling (no speedup)
- ~90 minutes for full 914 test suite
- Baseline for future Rust implementations

## Dependencies

**Required at runtime:**
- Python 3.11+ with docling 2.58.0 installed
- PyO3 links to system Python
- All Python docling dependencies (OCR engines, ML models, etc.)

**Cargo.toml:**
```toml
[dependencies]
pyo3 = { version = "0.20", features = ["auto-initialize"] }
```

## Next Phase

**Phase 1** will start replacing Python components with Rust:
- PDF text extraction → Rust (`pdfium-render`)
- Tests still pass (same API)
- Performance improves incrementally
- Python remains for other formats

Tests never change - only implementation underneath.
