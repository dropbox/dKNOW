# docling-py

Python bridge for docling-rs, enabling hybrid Rust/Python document processing.

## Overview

`docling-py` provides a bridge between Rust and Python, enabling docling-rs to leverage the battle-tested ML models and processing capabilities of the Python docling library while maintaining Rust's performance for serialization and I/O.

## Status

**Current Status:** Placeholder

This crate is reserved for future implementation of Python integration via PyO3. Currently, Python bridge functionality in docling-rs is handled through:

- **Integration tests:** Use Python subprocess for testing
- **Hybrid serializer:** Python parsing + Rust serialization (test mode only)

## Architecture

### Hybrid Approach

docling-rs uses a **hybrid approach** combining the strengths of both ecosystems:

```
┌─────────────────────────────────────────┐
│         User Application                 │
│              (Rust)                      │
├─────────────────────────────────────────┤
│        docling-core (Rust)              │  ← High-level API
├─────────────────────────────────────────┤
│       docling-backend (Rust)            │  ← Format dispatch
├─────────────────────────────────────────┤
│    ┌────────────┬───────────────────┐   │
│    │ Rust       │ Python Bridge     │   │
│    │ Backends   │ (docling-py)      │   │
│    │            │                   │   │
│    │ - Simple   │ - PDF (ML)        │   │
│    │ - DOCX     │ - OCR             │   │
│    │ - HTML     │ - Tables (ML)     │   │
│    │ - Images   │ - Advanced        │   │
│    └────────────┴───────────────────┘   │
└─────────────────────────────────────────┘
           ↓                    ↓
    Pure Rust Output    Python docling
    (Serialization)     (ML Inference)
```

### Benefits

- **Best of Both Worlds:** Battle-tested Python ML + Rust performance
- **Gradual Migration:** Port backends from Python to Rust incrementally
- **Compatibility:** Maintain compatibility with Python docling outputs
- **Performance:** Rust handles I/O, serialization, and simple parsing

## Future Roadmap

When Python bridge is implemented (via PyO3), this crate will provide:

### Planned Features

- **PyO3 Integration:** Safe Python interop from Rust
- **Python Docling Access:** Call Python docling from Rust
- **GIL Management:** Handle Python Global Interpreter Lock
- **Type Conversion:** Convert between Rust and Python types
- **Error Handling:** Bridge Python exceptions to Rust errors
- **Resource Management:** Automatic cleanup of Python objects

### Example (Future API)

```rust
use docling_py::{PythonDocling, PythonBackendOptions};

// Initialize Python bridge
let py_docling = PythonDocling::new()?;

// Convert document using Python backend
let options = PythonBackendOptions {
    enable_ocr: true,
    enable_table_extraction: true,
    ..Default::default()
};

let result = py_docling.convert("document.pdf", options)?;
```

## PyO3 Integration

### What is PyO3?

[PyO3](https://pyo3.rs/) is a Rust library for Python interop:

- Call Python from Rust
- Call Rust from Python
- Create Python modules in Rust
- Share data between Rust and Python

### PyO3 Example

```rust
use pyo3::prelude::*;
use pyo3::types::PyDict;

fn call_python_docling(pdf_path: &str) -> PyResult<String> {
    Python::with_gil(|py| {
        // Import Python docling
        let docling = py.import("docling.document_converter")?;
        let converter_cls = docling.getattr("DocumentConverter")?;

        // Create converter
        let converter = converter_cls.call0()?;

        // Convert document
        let result = converter.call_method1("convert", (pdf_path,))?;

        // Extract markdown
        let markdown: String = result
            .call_method0("export_to_markdown")?
            .extract()?;

        Ok(markdown)
    })
}
```

## Current Hybrid Mode

Currently, hybrid mode works through integration tests:

```bash
# Run tests with hybrid serializer (Python parsing + Rust serialization)
USE_HYBRID_SERIALIZER=1 cargo test test_canon
```

### How It Works

1. **Integration test** spawns Python subprocess
2. **Python docling** parses document to JSON
3. **Rust serializer** converts JSON to markdown
4. **Test** compares output to expected results

This validates that Rust serializers produce the same output as Python.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
docling-py = "2.58.0"
```

### Python Requirements

When implemented, this crate will require:

```bash
# Install Python docling
pip install docling==2.58.0

# Or use conda
conda install -c conda-forge docling=2.58.0
```

## Use Cases

### Gradual Rust Migration

Port backends from Python to Rust incrementally:

```rust
// Start with Python bridge for complex formats
let pdf_backend = PythonBackend::new("pdf")?;

// Later, replace with native Rust backend
let pdf_backend = RustPdfBackend::new()?;
```

### Access Python ML Models

Use Python ML models until native Rust alternatives are ready:

```rust
// Use Python OCR
let ocr_result = py_docling.ocr_image("scanned_page.png")?;

// Use Python table detection
let tables = py_docling.extract_tables("document.pdf")?;
```

### Compatibility Testing

Verify Rust implementations match Python behavior:

```rust
// Run both backends and compare
let python_result = py_backend.convert("doc.pdf")?;
let rust_result = rust_backend.convert("doc.pdf")?;
assert_eq!(python_result, rust_result);
```

## Performance Considerations

### Overhead

Python bridge introduces overhead:

- **GIL Contention:** Python Global Interpreter Lock serializes execution
- **Type Conversion:** Converting data between Rust and Python
- **FFI Boundary:** Crossing language boundary has cost

### When to Use

**Use Python Bridge:**
- For ML-heavy tasks (OCR, table detection)
- When Python implementation is battle-tested
- For complex document formats

**Use Pure Rust:**
- For simple parsing (HTML, CSV, plain text)
- For serialization (markdown, JSON)
- When performance is critical
- When Python installation is undesirable

### Benchmark Results

Example performance comparison (estimated):

| Task | Pure Python | Hybrid (Py+Rust) | Pure Rust |
|------|-------------|------------------|-----------|
| PDF (no OCR) | 1.0s | 0.8s | 0.5s |
| PDF (with OCR) | 15.0s | 14.5s | N/A |
| DOCX | 0.5s | 0.3s | 0.1s |
| Markdown export | 0.2s | 0.05s | 0.05s |

## GIL Management

The Python Global Interpreter Lock (GIL) must be handled carefully:

### GIL Basics

- Only one thread can execute Python code at a time
- Acquiring GIL blocks other threads
- Must release GIL when doing Rust work

### PyO3 GIL API

```rust
use pyo3::prelude::*;

// Acquire GIL
Python::with_gil(|py| {
    // Python code here
    let result = py.eval("1 + 1", None, None)?;
    Ok::<_, PyErr>(result)
})?;

// GIL automatically released when closure ends
```

### Threading Considerations

```rust
// Bad: Hold GIL across threads
let result = Python::with_gil(|py| {
    // ❌ This blocks all other threads
    expensive_python_operation(py)
});

// Good: Release GIL when possible
Python::with_gil(|py| {
    let data = extract_data_from_python(py)?;
    py.allow_threads(|| {
        // ✅ GIL released, other threads can run
        expensive_rust_operation(data)
    })
});
```

## Error Handling

Bridge Python exceptions to Rust errors:

```rust
use pyo3::exceptions::PyException;

fn convert_document(path: &str) -> Result<String, Error> {
    Python::with_gil(|py| {
        match call_python_docling(py, path) {
            Ok(result) => Ok(result),
            Err(py_err) => {
                // Convert Python exception to Rust error
                Err(Error::PythonError(py_err.to_string()))
            }
        }
    })
}
```

## Related Crates

- **docling-core:** High-level document processing API
- **docling-backend:** Uses Python bridge for some formats
- **docling-models:** Will use Python ML models (current implementation)
- **pyo3:** Rust-Python interop library (will be a dependency)

## Dependencies

When implemented, this crate will depend on:

- **pyo3:** Rust-Python bindings
- **docling (Python):** Python docling library (runtime dependency)

## License

Licensed under the MIT License. See LICENSE file for details.

## Contributing

This crate is part of the docling-rs project. For contribution guidelines, see the main repository.

## References

- **PyO3:** https://pyo3.rs/
- **Python docling:** https://github.com/docling-project/docling
- **docling-rs repository:** https://github.com/ayates_dbx/docling_rs

## Migration Path: Python → Rust

The long-term goal is to eliminate the Python dependency:

### Phase H (Current): Hybrid Testing

- Integration tests use Python subprocess
- Validates Rust serializers against Python outputs
- No runtime Python dependency in production code

### Phase I: PyO3 Integration

- Implement docling-py crate with PyO3
- Runtime Python bridge for ML features
- Optional Python dependency (feature-gated)

### Phase J+: Native Rust Backends

- Port ML models to Rust (ONNX, tract)
- Eliminate Python dependency entirely
- Pure Rust document processing stack

## Why Start with Hybrid?

The hybrid approach provides:

1. **Correctness:** Leverage battle-tested Python implementation
2. **Compatibility:** Match Python docling outputs exactly
3. **Pragmatism:** Focus on Rust serializers first
4. **Incremental:** Port backends one at a time
5. **Risk Management:** Avoid "big bang" rewrite

## Testing

To test hybrid mode today:

```bash
# Run canonical tests with hybrid serializer
USE_HYBRID_SERIALIZER=1 cargo test test_canon -- --test-threads=1

# Run specific format
USE_HYBRID_SERIALIZER=1 cargo test test_canon_pdf -- --exact
```

This validates that Rust serialization produces the same output as Python.

## Support

For questions and issues, see the main docling-rs repository issue tracker.
