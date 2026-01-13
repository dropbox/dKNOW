# User Guide

Welcome to docling-rs! This guide will help you get started with converting documents to markdown using Rust.

---

## Table of Contents

1. [Installation](#installation)
2. [Quick Start](#quick-start)
3. [Basic Usage](#basic-usage)
4. [Advanced Usage](#advanced-usage)
5. [Configuration](#configuration)
6. [Output Formats](#output-formats)
7. [Batch Processing](#batch-processing)
8. [Best Practices](#best-practices)
9. [Format-Specific Guides](#format-specific-guides)

---

## Format-Specific Guides

**Comprehensive guides for each format family:**

- 📄 **[PDF Documents](formats/pdf.md)** - Text extraction, OCR, tables, performance optimization
- 📝 **[Microsoft Office](formats/office.md)** - DOCX, PPTX, XLSX, legacy DOC formats
- 🌐 **[Web Formats](formats/web.md)** - HTML, Markdown, AsciiDoc, CSV
- 🖼️ **[Extended Formats](formats/extended.md)** - Images, e-books, archives, email, multimedia, specialty formats (40+ formats)

**Additional Resources:**

- 🏗️ **[Architecture](ARCHITECTURE.md)** - System architecture, design decisions, extension points
- ⚡ **[Performance Tuning](guides/performance.md)** - Optimization techniques, profiling, benchmarking

**Quick Links:**

| Format Category | Examples | Guide Link |
|----------------|----------|------------|
| **Documents** | PDF, DOCX, PPTX, XLSX | [PDF](formats/pdf.md) • [Office](formats/office.md) |
| **Web Content** | HTML, Markdown, CSV | [Web Formats](formats/web.md) |
| **Images** | PNG, JPEG, TIFF, WebP | [Extended Formats](formats/extended.md#image-formats) |
| **E-books** | EPUB, MOBI, FB2 | [Extended Formats](formats/extended.md#e-book-formats) |
| **Archives** | ZIP, TAR, 7Z, RAR | [Extended Formats](formats/extended.md#archive-formats) |
| **Email** | EML, MBOX, MSG | [Extended Formats](formats/extended.md#email-formats) |

---

## Installation

### Prerequisites

**Required:**
- Rust 1.70+ (https://rustup.rs/)

**Optional:**
- LibreOffice (for DOC/PUB legacy format support)
- LLVM/Clang (for building with OpenCV features)

### Step 1: Install Rust

```bash
# Install Rust via rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Verify installation
rustc --version
cargo --version
```

### Step 2: Add docling-rs to Your Project

**Add to `Cargo.toml`:**
```toml
[dependencies]
docling-core = { path = "path/to/docling_rs/crates/docling-core" }
# Or once published to crates.io:
# docling-core = "0.1.0"
```

### Step 3: Verify Setup

```bash
# Clone repository
git clone https://github.com/your-org/docling_rs.git
cd docling_rs

# Run tests to verify installation
cargo test --lib

# Expected: All tests pass
```

---

## Quick Start

### Convert a Single Document

```rust
// Note: DocumentConverter is in docling-backend crate
use docling_backend::DocumentConverter;
use docling_core::Result;

fn main() -> Result<()> {
    // Create converter
    let converter = DocumentConverter::new()?;

    // Convert document
    let result = converter.convert("document.pdf")?;

    // Access markdown output
    println!("{}", result.document.markdown);

    // Check conversion time
    println!("Converted in {:?}", result.latency);

    Ok(())
}
```

### Save Output to File

```rust
// Note: DocumentConverter is in docling-backend crate
use docling_backend::DocumentConverter;
use docling_core::Result;
use std::fs;

fn main() -> Result<()> {
    let converter = DocumentConverter::new()?;
    let result = converter.convert("document.pdf")?;

    // Write markdown to file
    fs::write("output.md", &result.document.markdown)?;

    println!("Saved to output.md");
    Ok(())
}
```

---

## Basic Usage

### Converting Different Formats

docling-rs supports **65+ formats across 15+ categories**. Here are common examples:

**For detailed format-specific documentation, see [Format-Specific Guides](#format-specific-guides).**

#### PDF Documents

```rust
// Note: DocumentConverter is in docling-backend crate
use docling_backend::DocumentConverter;
use docling_core::Result;

fn convert_pdf(path: &str) -> Result<String> {
    let converter = DocumentConverter::new()?;
    let result = converter.convert(path)?;
    Ok(result.document.markdown)
}

// Usage
let markdown = convert_pdf("report.pdf")?;
```

**Features:**
- Text extraction
- Table structure detection
- Image captioning (with OCR)
- Multi-page support

#### Microsoft Office Documents

```rust
// DOCX
let markdown = converter.convert("document.docx")?;

// PPTX
let markdown = converter.convert("presentation.pptx")?;

// XLSX
let markdown = converter.convert("spreadsheet.xlsx")?;
```

**Note:** Uses native Rust/C++ ML models via PyTorch and ONNX Runtime for best accuracy.

#### Images (with OCR)

```rust
// Enable OCR for images
let converter = DocumentConverter::with_ocr(true)?;

// Convert images
let markdown = converter.convert("scan.png")?;
let markdown = converter.convert("photo.jpg")?;
```

**Supported image formats:** PNG, JPEG, TIFF, WebP, BMP, GIF, HEIF, AVIF

#### E-books

```rust
// Enable Rust backend for e-books
std::env::set_var("USE_RUST_BACKEND", "1");

let converter = DocumentConverter::new()?;
let markdown = converter.convert("book.epub")?;
let markdown = converter.convert("novel.mobi")?;
```

**Supported formats:** EPUB, FB2, MOBI

See [Format Support Matrix](FORMATS.md) for complete list, or see format-specific guides:
- **[PDF Guide](formats/pdf.md)** - Comprehensive PDF documentation
- **[Office Guide](formats/office.md)** - DOCX, PPTX, XLSX details
- **[Extended Formats Guide](formats/extended.md)** - E-books, archives, and 40+ more formats

---

## Advanced Usage

### Enabling OCR

OCR (Optical Character Recognition) extracts text from images and scanned documents.

**Auto-Detection:** Scanned PDFs are automatically detected and OCR is enabled without requiring explicit configuration. A PDF is considered scanned if its first few pages have no programmatic text and consist of single-image scans.

```rust
// Note: DocumentConverter is in docling-backend crate
use docling_backend::DocumentConverter;
use docling_core::Result;

fn main() -> Result<()> {
    let converter = DocumentConverter::new()?;

    // Scanned PDFs are auto-detected - no special config needed!
    let result = converter.convert("scanned_document.pdf")?;

    println!("Extracted text:\n{}", result.document.markdown);
    Ok(())
}
```

**Force OCR On:** If auto-detection doesn't work for your document, you can force OCR:

```rust
let converter = DocumentConverter::with_ocr(true)?;
```

**Performance Note:** OCR adds 5-15 seconds per page. Auto-detection only enables OCR when needed.

**OCR Engines:**
- **macOS:** Uses built-in `ocrmac` (non-deterministic)
- **Linux/Windows:** Requires `tesseract` or `easyocr` installation

See [Troubleshooting - OCR Issues](TROUBLESHOOTING.md#ocr-problems) for configuration.

---

### Using Rust Backend (Experimental)

For formats like e-books, archives, and email, you can use pure Rust parsers (5-10x faster):

```rust
// Note: DocumentConverter is in docling-backend crate
use docling_backend::DocumentConverter;
use docling_core::Result;

fn main() -> Result<()> {
    // Enable Rust backend
    std::env::set_var("USE_RUST_BACKEND", "1");

    let converter = DocumentConverter::new()?;

    // These use Rust parsers:
    let epub = converter.convert("book.epub")?;      // E-book
    let zip = converter.convert("archive.zip")?;     // Archive (extracts contents)
    let eml = converter.convert("email.eml")?;       // Email
    let odt = converter.convert("document.odt")?;    // OpenDocument

    Ok(())
}
```

**Supported Rust backends:**
- E-books: EPUB, FB2, MOBI
- Archives: ZIP, TAR, 7Z, RAR
- Email: EML, MBOX, MSG, VCF
- OpenDocument: ODT, ODS, ODP
- Multimedia: SRT, WebVTT, video/audio metadata
- Specialty: XPS, RTF, SVG, ICS, IPYNB, GPX, KML, etc.

See [Format Support Matrix - Rust Backend](FORMATS.md#rust-backend-formats-36) for full list.

---

### Accessing Metadata

```rust
// Note: DocumentConverter is in docling-backend crate
use docling_backend::DocumentConverter;
use docling_core::Result;

fn main() -> Result<()> {
    let converter = DocumentConverter::new()?;
    let result = converter.convert("document.pdf")?;

    let doc = &result.document;

    // Basic metadata
    println!("Format: {:?}", doc.format);
    println!("Characters: {}", doc.metadata.num_characters);

    // Timing
    println!("Conversion time: {:?}", result.latency);

    // More metadata (Phase 0: limited support)
    if let Some(title) = &doc.metadata.title {
        println!("Title: {}", title);
    }

    Ok(())
}
```

**Available metadata (Phase 0):**
- Format (InputFormat enum)
- Character count
- Conversion latency

**Future metadata (Phase 1+):**
- Title, author, creation date
- Page count
- Language detection
- Document structure (headings, sections)

---

### Error Handling

```rust
// Note: DocumentConverter is in docling-backend crate
use docling_backend::DocumentConverter;
use docling_core::{DoclingError, Result};

fn safe_convert(path: &str) -> Result<String> {
    let converter = DocumentConverter::new()?;

    match converter.convert(path) {
        Ok(result) => Ok(result.document.markdown),
        Err(DoclingError::FormatError(msg)) => {
            eprintln!("Unsupported format: {}", msg);
            Err(DoclingError::FormatError(msg))
        }
        Err(DoclingError::PythonError(msg)) => {
            eprintln!("Python conversion failed: {}", msg);
            Err(DoclingError::PythonError(msg))
        }
        Err(e) => {
            eprintln!("Conversion error: {:?}", e);
            Err(e)
        }
    }
}
```

**Error Types:**
- `FormatError`: Unsupported or unrecognized file format
- `ConversionError`: General conversion error (parsing, ML model, etc.)
- `IOError`: File read/write error
- `ModelError`: ML model loading or inference failure

---

## Configuration

### Environment Variables

docling-rs uses environment variables for runtime configuration:

| Variable | Values | Default | Purpose |
|----------|--------|---------|---------|
| `LIBTORCH_USE_PYTORCH` | `1` | not set | Use PyTorch C++ backend for PDF ML |
| `ONNX_BACKEND` | `1` | not set | Use ONNX Runtime instead of PyTorch |
| `PDF_ML_CACHE_DIR` | path | system temp | Cache directory for ML model weights |

**Example:**
```bash
# Set up PyTorch C++ backend for PDF processing
source setup_env.sh  # Sets LIBTORCH_USE_PYTORCH=1
cargo build --features pdf-ml

# Or in code:
std::env::set_var("USE_RUST_BACKEND", "1");
```

---

### OCR Configuration

```rust
// Disable OCR (default, faster)
let converter = DocumentConverter::new()?;

// Enable OCR (slower, extracts text from images)
let converter = DocumentConverter::with_ocr(true)?;
```

**When to enable OCR:**
- Scanned PDFs (no embedded text)
- Images with text (PNG, JPEG, etc.)
- Documents with important image content

**When to disable OCR:**
- Text-based PDFs (already has embedded text)
- Performance-critical applications
- Batch processing large document sets

---

## Output Formats

### Markdown (Default)

```rust
let result = converter.convert("document.pdf")?;
let markdown = &result.document.markdown;

// Contains:
// - Headings (# ## ###)
// - Paragraphs
// - Lists (bullet and numbered)
// - Tables (markdown format)
// - Code blocks
// - Links
```

**Example output:**
```markdown
# Document Title

## Section 1

This is a paragraph with **bold** and *italic* text.

### Subsection

| Column 1 | Column 2 |
|----------|----------|
| Data A   | Data B   |
| Data C   | Data D   |

- Bullet point 1
- Bullet point 2
```

### JSON Export

JSON export is fully supported via the CLI:

```bash
# Export to JSON
docling convert input.pdf --format json --output output.json

# Or use the API
// Note: DocumentConverter is in docling-backend crate
use docling_backend::DocumentConverter;

let converter = DocumentConverter::new()?;
let result = converter.convert("input.pdf")?;
let json = serde_json::to_string_pretty(&result.document)?;
```

### HTML Export

HTML export is not yet implemented. Use markdown output and convert with tools like `pandoc`:

```bash
# Export to markdown, then convert to HTML
docling convert input.pdf --format markdown | pandoc -f markdown -t html > output.html
```

---

## Batch Processing

### Processing Multiple Files

```rust
// Note: DocumentConverter is in docling-backend crate
use docling_backend::DocumentConverter;
use docling_core::Result;
use std::path::Path;
use std::fs;

fn batch_convert(input_dir: &Path, output_dir: &Path) -> Result<()> {
    // Create converter once (reuse for efficiency)
    let converter = DocumentConverter::new()?;

    // Ensure output directory exists
    fs::create_dir_all(output_dir)?;

    // Process all PDF files
    for entry in fs::read_dir(input_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|e| e.to_str()) == Some("pdf") {
            println!("Converting: {:?}", path);

            // Convert
            let result = converter.convert(&path)?;

            // Save with .md extension
            let output_name = path.file_stem()
                .unwrap()
                .to_str()
                .unwrap();
            let output_path = output_dir.join(format!("{}.md", output_name));

            fs::write(&output_path, &result.document.markdown)?;

            println!("  -> Saved to: {:?} ({:?})", output_path, result.latency);
        }
    }

    Ok(())
}

// Usage
fn main() -> Result<()> {
    batch_convert(
        Path::new("input_documents/"),
        Path::new("output_markdown/")
    )?;
    Ok(())
}
```

---

### Parallel Processing (Future)

```rust
use rayon::prelude::*;
use std::sync::Arc;

fn parallel_convert(files: Vec<PathBuf>) -> Vec<Result<String>> {
    // Create converter once (wrapped in Arc for sharing)
    let converter = Arc::new(DocumentConverter::new().unwrap());

    files.par_iter()
        .map(|path| {
            let conv = Arc::clone(&converter);
            let result = conv.convert(path)?;
            Ok(result.document.markdown)
        })
        .collect()
}
```

**Note:** Requires thread-safe Python GIL handling (future enhancement).

---

### Progress Tracking

```rust
use indicatif::{ProgressBar, ProgressStyle};

fn batch_with_progress(files: &[PathBuf]) -> Result<()> {
    let converter = DocumentConverter::new()?;

    // Create progress bar
    let pb = ProgressBar::new(files.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg}")?
            .progress_chars("##-")
    );

    for (i, path) in files.iter().enumerate() {
        pb.set_message(format!("Converting: {}", path.display()));

        let result = converter.convert(path)?;

        // Save output...

        pb.inc(1);
    }

    pb.finish_with_message("Done!");
    Ok(())
}
```

---

## Best Practices

### 1. Reuse Converter Instances

```rust
// GOOD: Create once, reuse
let converter = DocumentConverter::new()?;
for file in files {
    converter.convert(&file)?;
}

// BAD: Create per file (slow! Loads Python modules each time)
for file in files {
    let converter = DocumentConverter::new()?;
    converter.convert(&file)?;
}
```

**Why:** Creating a converter loads Python modules and initializes ML models (expensive).

---

### 2. Use Release Builds for Production

```bash
# Debug build (slow)
cargo run -- document.pdf

# Release build (5-10x faster)
cargo run --release -- document.pdf
```

**Performance difference:**
- Debug: 2.5s per PDF
- Release: 0.3-0.5s per PDF

---

### 3. Disable OCR When Not Needed

```rust
// For text PDFs (already has embedded text)
let converter = DocumentConverter::new()?; // OCR disabled

// Only enable for scanned documents
let converter = DocumentConverter::with_ocr(true)?;
```

**Impact:** OCR adds 5-15 seconds per page.

---

### 4. Handle Errors Gracefully

```rust
fn robust_convert(path: &Path) -> Option<String> {
    let converter = DocumentConverter::new().ok()?;

    match converter.convert(path) {
        Ok(result) => Some(result.document.markdown),
        Err(e) => {
            eprintln!("Failed to convert {:?}: {}", path, e);
            None
        }
    }
}
```

---

### 5. Validate Input Files

```rust
use std::path::Path;

fn is_supported_format(path: &Path) -> bool {
    let ext = path.extension()?.to_str()?.to_lowercase();
    matches!(ext.as_str(),
        "pdf" | "docx" | "pptx" | "xlsx" |
        "html" | "md" | "png" | "jpg" |
        // ... (see FORMATS.md)
    )
}

// Usage
if !is_supported_format(&path) {
    eprintln!("Unsupported format: {:?}", path);
    return Err(...);
}
```

---

### 6. Monitor Memory Usage

```rust
// For large batch processing
for chunk in files.chunks(100) {
    let converter = DocumentConverter::new()?;

    for file in chunk {
        converter.convert(file)?;
    }

    // Drop converter to free memory
    drop(converter);

    // Optional: Force garbage collection (Python side)
    // Python::with_gil(|py| py.run("import gc; gc.collect()", None, None))?;
}
```

---

## Example Applications

### CLI Tool

```rust
// Note: DocumentConverter is in docling-backend crate
use docling_backend::DocumentConverter;
use docling_core::Result;
use std::env;
use std::fs;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        eprintln!("Usage: {} <input> <output>", args[0]);
        std::process::exit(1);
    }

    let input = &args[1];
    let output = &args[2];

    println!("Converting {} to {}", input, output);

    let converter = DocumentConverter::new()?;
    let result = converter.convert(input)?;

    fs::write(output, &result.document.markdown)?;

    println!("Success! Converted in {:?}", result.latency);

    Ok(())
}
```

**Usage:**
```bash
cargo run --release -- document.pdf output.md
```

---

### Web Service

Example REST API using `axum` web framework:

```rust
use axum::{
    extract::Multipart,
    http::StatusCode,
    routing::post,
    Router,
};
// Note: DocumentConverter is in docling-backend crate
use docling_backend::DocumentConverter;

async fn convert_document(mut multipart: Multipart) -> Result<String, StatusCode> {
    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap().to_string();
        if name == "file" {
            let data = field.bytes().await.unwrap();

            // Write to temp file
            let temp_path = format!("/tmp/upload_{}", uuid::Uuid::new_v4());
            std::fs::write(&temp_path, data).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            // Convert
            let converter = DocumentConverter::new().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            let result = converter.convert(&temp_path).map_err(|_| StatusCode::BAD_REQUEST)?;

            // Cleanup
            let _ = std::fs::remove_file(&temp_path);

            return Ok(result.markdown);
        }
    }
    Err(StatusCode::BAD_REQUEST)
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/convert", post(convert_document));

    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
```

Add dependencies to `Cargo.toml`:
```toml
[dependencies]
docling-core = { path = "path/to/docling-rs/crates/docling-core" }
axum = "0.7"
tokio = { version = "1", features = ["full"] }
uuid = { version = "1", features = ["v4"] }
```

---

## Performance Benchmarks

### Expected Performance (Release Build)

| Format | File Size | Time | Throughput |
|--------|-----------|------|------------|
| PDF (text) | 1 MB | 0.3-1.0s | 1-3 MB/s |
| PDF (OCR) | 1 MB, 10 pages | 50-150s | 10-30 pages/min |
| DOCX | 500 KB | 0.01-0.05s | 10-50 MB/s |
| HTML | 100 KB | 0.002-0.01s | 10-50 MB/s |
| EPUB | 2 MB | 0.1-0.5s | 4-20 MB/s |

See [Baseline Performance Benchmarks](BASELINE_PERFORMANCE_BENCHMARKS.md) for detailed measurements.

---

## Next Steps

### Documentation

- **Format-Specific Guides:**
  - [PDF Documents](formats/pdf.md) - Text extraction, OCR, tables
  - [Office Formats](formats/office.md) - DOCX, PPTX, XLSX
  - [Web Formats](formats/web.md) - HTML, Markdown, CSV
  - [Extended Formats](formats/extended.md) - 40+ additional formats
- **Architecture & Performance:**
  - [Architecture](ARCHITECTURE.md) - System design and extension points
  - [Performance Tuning](guides/performance.md) - Optimization techniques
- **Reference:**
  - [API.md](API.md) - Complete API documentation
  - [FORMATS.md](FORMATS.md) - Format support matrix
  - [TROUBLESHOOTING.md](TROUBLESHOOTING.md) - Common issues
  - [CONTRIBUTING.md](CONTRIBUTING.md) - Contributing guide

---

## Examples

**Complete, runnable examples** demonstrating all features are available in the `examples/` directory.

### Available Examples (10)

1. **basic_conversion.rs** - Simple document conversion (start here!)
2. **ocr_processing.rs** - OCR for scanned documents and images
3. **batch_processing.rs** - Sequential and parallel batch processing
4. **error_handling.rs** - Robust error handling with retry logic
5. **metadata_extraction.rs** - Extract and analyze document metadata
6. **custom_serialization.rs** - Customize markdown output options
7. **format_detection.rs** - Automatic format detection and multi-format handling
8. **streaming_api.rs** - Stream processing with progress reporting
9. **performance_bench.rs** - Performance measurement and optimization
10. **cli_tool.rs** - Complete CLI application with subcommands

### Running Examples

```bash
# Run a specific example
cargo run --package docling-examples --example basic_conversion -- document.pdf

# Use release build for better performance
cargo run --release --package docling-examples --example batch_processing -- *.pdf
```

### Example Documentation

See `examples/README.md` for:
- Detailed descriptions of each example
- Usage instructions and command-line arguments
- Common patterns and best practices
- Troubleshooting guide

### Quick Start with Examples

**New to docling-rs?** Start with these examples in order:

1. `basic_conversion.rs` - Learn the basics
2. `ocr_processing.rs` - Handle scanned documents
3. `batch_processing.rs` - Process multiple files
4. `error_handling.rs` - Production-ready patterns

**Additional Resources:**
- API Documentation: Run `cargo doc --open` for full API reference
- API Cookbook: See `docs/API_COOKBOOK.md` for 30+ code patterns

---

**Last Updated:** 2025-12-08 (N=2893)
**Status:** Production-ready (100% Rust/C++ - no Python required)
**Architecture:** Pure Rust with C++ FFI for ML (PyTorch, ONNX Runtime, pdfium)
