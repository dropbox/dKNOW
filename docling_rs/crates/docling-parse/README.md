# docling-parse

High-performance C++ document parsing library for docling.

## Overview

`docling-parse` is a C++ library that provides high-performance PDF document parsing with advanced text segmentation capabilities. This library is the foundation for PDF processing in docling-rs.

## Status

**Current Status:** Placeholder Crate

This crate is a placeholder for the C++ docling-parse library. The actual C++ library exists separately and is accessed through:

- **docling-parse-sys:** Raw FFI bindings to the C API
- **docling-parse-rs:** Safe Rust wrapper around the C API

## Architecture

The docling-parse integration consists of three layers:

```
┌─────────────────────────────────────┐
│     docling-backend (Rust)          │  ← High-level document processing
├─────────────────────────────────────┤
│     docling-parse-rs (Rust)         │  ← Safe Rust wrapper
├─────────────────────────────────────┤
│     docling-parse-sys (FFI)         │  ← Raw bindings (bindgen)
├─────────────────────────────────────┤
│   libdocling_parse.so (C++ lib)     │  ← C++ parsing engine
└─────────────────────────────────────┘
```

## Features

The C++ docling-parse library provides:

### PDF Processing

- **Text Extraction:** Extract text with precise positioning
- **Text Segmentation:** Identify paragraphs, headings, lists
- **Table Detection:** Locate and extract table structures
- **Font Analysis:** Extract font information (family, size, style)
- **Coordinate Mapping:** Map text to page coordinates
- **Multi-page Support:** Efficient processing of large PDF documents

### Advanced Features

- **Reading Order:** Determine logical reading order of text
- **Text Clustering:** Group related text elements
- **Layout Analysis:** Identify document structure
- **Character-level Details:** Access individual character positions

## Installation

The C++ library must be installed separately. See the main docling-rs repository for installation instructions.

For Rust integration, add to your `Cargo.toml`:

```toml
[dependencies]
docling-parse-rs = "2.58.0"   # Safe Rust wrapper
# or
docling-parse-sys = "2.58.0"  # Raw FFI bindings
```

## Usage

Do not use this crate directly. Instead use:

### docling-parse-rs (Recommended)

Safe Rust wrapper with error handling:

```rust
use docling_parse_rs::DoclingParser;

// Create parser
let mut parser = DoclingParser::new("error")?;

// Load PDF
parser.load_document("doc1", "example.pdf", None)?;

// Get page count
let num_pages = parser.number_of_pages("doc1")?;

// Parse a page (returns JSON)
let page_json = parser.parse_page("doc1", 0)?;
```

### docling-parse-sys (Advanced)

Raw FFI bindings for low-level control:

```rust
use docling_parse_sys::*;
use std::ffi::CString;

unsafe {
    // Create parser
    let loglevel = CString::new("error").unwrap();
    let parser = docling_parser_new(loglevel.as_ptr());

    // Use parser...

    // Free parser
    docling_parser_free(parser);
}
```

## Performance

The C++ library provides significant performance benefits:

- **Fast Processing:** Optimized C++ code for PDF parsing
- **Memory Efficient:** Minimal memory overhead
- **Scalable:** Handles large PDFs (1000+ pages)

### Benchmarks

Typical performance on modern hardware:

- **Simple PDFs:** ~0.5-1s per document
- **Complex PDFs:** ~1-3s per document
- **Scanned PDFs:** Dependent on OCR (handled separately)

## Build Requirements

To build docling-parse-sys (which links to the C++ library):

### System Requirements

- **C++ Compiler:** C++17 or later (GCC 7+, Clang 5+, MSVC 2017+)
- **CMake:** 3.15 or later
- **Dependencies:**
  - pdfium or poppler (PDF rendering)
  - Additional dependencies per platform

### Building

The C++ library is built automatically when building docling-parse-sys:

```bash
cargo build -p docling-parse-sys
```

This will:
1. Download/locate the C++ library
2. Build C wrapper (if needed)
3. Generate FFI bindings with bindgen
4. Link the library

## Related Crates

- **docling-parse-rs:** Safe Rust wrapper (recommended)
- **docling-parse-sys:** Raw FFI bindings
- **docling-backend:** Uses parse library for PDF processing
- **docling-core:** High-level API

## C++ Library

The underlying C++ library is part of the docling project:

- **Repository:** https://github.com/docling-project/docling-parse
- **Language:** C++17
- **License:** MIT
- **Platform Support:** Linux, macOS, Windows

## FFI Safety

The C++ library is accessed through a C API for FFI safety:

### C API Wrapper

The C wrapper (`wrapper.h`) provides:

```c
// Create/destroy parser
DoclingParser* docling_parser_new(const char* loglevel);
void docling_parser_free(DoclingParser* parser);

// Load documents
bool docling_load_document(DoclingParser* parser,
                          const char* doc_key,
                          const char* pdf_path,
                          const char* format);

// Parse pages
char* docling_parse_page_json(DoclingParser* parser,
                              const char* doc_key,
                              int page_no);
```

### Memory Management

- **Strings:** Caller must free strings returned by C API
- **Parser:** Must call `docling_parser_free()` to release resources
- **Thread Safety:** Not thread-safe, use separate parsers per thread

## Development

This crate is a placeholder. Development happens in:

1. **C++ Library:** https://github.com/docling-project/docling-parse
2. **FFI Bindings:** `docling-parse-sys` crate (this repository)
3. **Rust Wrapper:** `docling-parse-rs` crate (this repository)

## License

Licensed under the MIT License. See LICENSE file for details.

## Contributing

This crate is part of the docling-rs project. For contribution guidelines, see the main repository.

## References

- **docling-parse C++ library:** https://github.com/docling-project/docling-parse
- **Python docling:** https://github.com/docling-project/docling
- **docling-rs repository:** https://github.com/ayates_dbx/docling_rs

## Note

This placeholder crate exists to reserve the `docling-parse` name on crates.io. The actual C++ library is separate. For Rust integration, use `docling-parse-rs` or `docling-parse-sys`.
