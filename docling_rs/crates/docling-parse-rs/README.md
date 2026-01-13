# docling-parse-rs

Safe Rust wrapper for the docling-parse C++ library.

## Overview

`docling-parse-rs` provides safe, idiomatic Rust bindings to the docling-parse C++ library for high-performance PDF document parsing. This crate wraps the raw FFI bindings from `docling-parse-sys` with memory-safe types and error handling.

## Features

- **Safe API:** No unsafe code in user-facing API
- **Rust Idioms:** Uses Result types, ownership, and RAII
- **PDF Parsing:** Extract text with precise positioning
- **Page Processing:** Efficient page-by-page parsing
- **Text Segmentation:** Identify paragraphs, headings, and structure
- **Type Conversion:** Convert C structures to Rust types

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
docling-parse-rs = "2.58.0"
```

### System Requirements

The underlying C++ library must be installed. See [docling-parse-sys](https://crates.io/crates/docling-parse-sys) for build requirements.

## Quick Start

```rust
use docling_parse_rs::DoclingParser;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create parser
    let mut parser = DoclingParser::new("error")?;

    // Load PDF document
    parser.load_document("doc1", Path::new("example.pdf"), None)?;

    // Get page count
    let num_pages = parser.number_of_pages("doc1")?;
    println!("Document has {} pages", num_pages);

    // Parse first page (returns JSON)
    let page_json = parser.parse_page("doc1", 0)?;
    println!("Page JSON: {}", page_json);

    // Unload document (optional, happens automatically on drop)
    parser.unload_document("doc1")?;

    Ok(())
}
```

## API Documentation

### DoclingParser

The main entry point for PDF parsing.

#### Creating a Parser

```rust
use docling_parse_rs::DoclingParser;

// Create parser with log level
// Valid levels: "debug", "info", "warn", "error", "off"
let mut parser = DoclingParser::new("error")?;
```

#### Loading Documents

```rust
use std::path::Path;

// Load PDF without password
parser.load_document("doc1", Path::new("example.pdf"), None)?;

// Load encrypted PDF with password
parser.load_document("doc2", Path::new("encrypted.pdf"), Some("password123"))?;
```

#### Document Operations

```rust
// Check if document is loaded
if parser.is_loaded("doc1")? {
    println!("Document is loaded");
}

// Get page count
let num_pages = parser.number_of_pages("doc1")?;

// Unload document
parser.unload_document("doc1")?;
```

#### Parsing Pages

```rust
// Parse page as JSON string
let json = parser.parse_page("doc1", 0)?;

// Parse page as structured Rust types
let page: PdfPage = parser.parse_page_typed("doc1", 0)?;

// Parse as segmented page (docling-core types)
let segmented = parser.parse_page_segmented("doc1", 0)?;
```

### Type Conversions

The crate provides several conversion functions:

```rust
use docling_parse_rs::{convert_to_segmented_page, PdfPage};

// Convert parsed page to segmented format
let page: PdfPage = parser.parse_page_typed("doc1", 0)?;
let segmented = convert_to_segmented_page(&page)?;
```

## Types

### PdfDocument

Represents a parsed PDF document:

```rust
pub struct PdfDocument {
    pub pages: Vec<PdfPage>,
    pub metadata: Option<HashMap<String, String>>,
}
```

### PdfPage

Represents a parsed PDF page:

```rust
pub struct PdfPage {
    pub page_no: usize,
    pub width: f64,
    pub height: f64,
    pub cells: Vec<Cell>,
    pub dimensions: Vec<Dimension>,
}
```

### Cell

Represents a text cell on a page:

```rust
pub struct Cell {
    pub text: String,
    pub bbox: BBox,
    pub font: Option<FontInfo>,
}
```

## Error Handling

The crate uses a `Result<T, Error>` type for error handling:

```rust
use docling_parse_rs::{DoclingParser, Error};

match parser.load_document("doc1", Path::new("example.pdf"), None) {
    Ok(()) => println!("Document loaded"),
    Err(Error::FileNotFound(path)) => eprintln!("File not found: {}", path),
    Err(Error::InvalidPdf(msg)) => eprintln!("Invalid PDF: {}", msg),
    Err(e) => eprintln!("Error: {}", e),
}
```

### Error Types

- `FileNotFound`: PDF file does not exist
- `InvalidPdf`: File is not a valid PDF
- `NotLoaded`: Document key not loaded
- `OutOfMemory`: Memory allocation failed
- `ParseError`: Parsing failed
- `InvalidParameter`: Invalid function parameter
- `Nul`: String contains null byte (FFI error)

## Memory Management

The crate uses RAII (Resource Acquisition Is Initialization) for automatic cleanup:

```rust
{
    let mut parser = DoclingParser::new("error")?;
    parser.load_document("doc1", Path::new("example.pdf"), None)?;
    // Use parser...
} // Parser and all loaded documents automatically freed here
```

### Manual Cleanup

You can manually unload documents if needed:

```rust
parser.unload_document("doc1")?;
```

## Performance

The C++ library provides excellent performance:

- **Simple PDFs:** ~0.5-1s per document
- **Complex PDFs:** ~1-3s per document
- **Memory Efficient:** Minimal overhead
- **Scalable:** Handles 1000+ page documents

### Performance Tips

1. **Reuse Parser:** Create one parser and reuse for multiple documents
2. **Page-by-Page:** Parse pages incrementally for large documents
3. **Unload When Done:** Unload documents to free memory
4. **Log Level:** Use "error" or "off" for production (less overhead)

```rust
// Good: Reuse parser
let mut parser = DoclingParser::new("error")?;
for file in pdf_files {
    parser.load_document("current", &file, None)?;
    // Process document...
    parser.unload_document("current")?;
}

// Bad: Create new parser each time (slower)
for file in pdf_files {
    let mut parser = DoclingParser::new("error")?;
    parser.load_document("current", &file, None)?;
    // Process document...
}
```

## Thread Safety

`DoclingParser` is **not thread-safe**. Use separate parser instances per thread:

```rust
use std::thread;

let handles: Vec<_> = pdf_files
    .into_iter()
    .map(|file| {
        thread::spawn(move || {
            let mut parser = DoclingParser::new("error").unwrap();
            parser.load_document("doc", &file, None).unwrap();
            // Parse document...
        })
    })
    .collect();

for handle in handles {
    handle.join().unwrap();
}
```

## Integration with docling-core

This crate is designed to integrate with `docling-core`:

```rust
use docling_core::types::page::SegmentedPdfPage;
use docling_parse_rs::DoclingParser;

// Parse page and convert to docling-core types
let mut parser = DoclingParser::new("error")?;
parser.load_document("doc1", Path::new("example.pdf"), None)?;
let segmented: SegmentedPdfPage = parser.parse_page_segmented("doc1", 0)?;

// Use with docling serializers
use docling_core::serializers::MarkdownSerializer;
let markdown = MarkdownSerializer::new().serialize(&segmented)?;
```

## Examples

### Basic PDF Parsing

```rust
use docling_parse_rs::DoclingParser;
use std::path::Path;

fn parse_pdf(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = DoclingParser::new("error")?;
    parser.load_document("doc", path, None)?;

    let num_pages = parser.number_of_pages("doc")?;
    for page_no in 0..num_pages {
        let page_json = parser.parse_page("doc", page_no)?;
        println!("Page {}: {}", page_no, page_json);
    }

    Ok(())
}
```

### Batch Processing

```rust
use docling_parse_rs::DoclingParser;
use std::path::Path;

fn batch_process(files: &[&Path]) -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = DoclingParser::new("error")?;

    for (i, file) in files.iter().enumerate() {
        let key = format!("doc{}", i);
        parser.load_document(&key, file, None)?;

        let num_pages = parser.number_of_pages(&key)?;
        println!("File: {:?}, Pages: {}", file, num_pages);

        parser.unload_document(&key)?;
    }

    Ok(())
}
```

### Error Handling

```rust
use docling_parse_rs::{DoclingParser, Error};
use std::path::Path;

fn safe_parse(path: &Path) -> Result<String, Error> {
    let mut parser = DoclingParser::new("error")?;

    match parser.load_document("doc", path, None) {
        Ok(()) => {
            let json = parser.parse_page("doc", 0)?;
            Ok(json)
        }
        Err(Error::FileNotFound(_)) => {
            eprintln!("File not found: {:?}", path);
            Err(Error::FileNotFound(path.to_string_lossy().to_string()))
        }
        Err(Error::InvalidPdf(msg)) => {
            eprintln!("Invalid PDF: {}", msg);
            Err(Error::InvalidPdf(msg))
        }
        Err(e) => Err(e),
    }
}
```

## Building from Source

```bash
# Clone repository
git clone https://github.com/ayates_dbx/docling_rs
cd docling_rs

# Build this crate (builds docling-parse-sys as dependency)
cargo build -p docling-parse-rs

# Run tests
cargo test -p docling-parse-rs

# Build release
cargo build -p docling-parse-rs --release
```

## Related Crates

- **docling-parse-sys:** Raw FFI bindings (this crate wraps it)
- **docling-parse:** Placeholder for C++ library name
- **docling-core:** High-level document processing API
- **docling-backend:** Uses this crate for PDF parsing

## Dependencies

- **docling-parse-sys:** Raw FFI bindings to C++ library
- **docling-core:** Core types (for SegmentedPdfPage conversion)
- **serde:** For JSON parsing (optional)

## License

Licensed under the MIT License. See LICENSE file for details.

## Contributing

This crate is part of the docling-rs project. For contribution guidelines, see the main repository.

## References

- **docling-parse C++ library:** https://github.com/docling-project/docling-parse
- **docling-rs repository:** https://github.com/ayates_dbx/docling_rs
- **Python docling:** https://github.com/docling-project/docling

## Comparison to Python docling

This Rust wrapper provides similar functionality to Python docling's PDF parsing:

| Feature | Python docling | docling-parse-rs |
|---------|----------------|------------------|
| PDF Parsing | ✅ | ✅ |
| Text Extraction | ✅ | ✅ |
| Table Detection | ✅ | ✅ |
| Font Analysis | ✅ | ✅ |
| Memory Safety | ⚠️ (C++ binding) | ✅ (Safe Rust) |
| Performance | Fast | Fast (same C++ backend) |
| Type Safety | Dynamic | Static |

## Status

This crate is production-ready and actively used in docling-rs for PDF processing.

## Support

For questions and issues, see the main docling-rs repository issue tracker.
