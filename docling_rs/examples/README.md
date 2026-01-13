# Docling Examples

This directory contains practical examples demonstrating how to use the docling-core library for document conversion and processing.

## Running Examples

All examples can be run using `cargo run --example <name>`:

```bash
# Run with debug build
cargo run --example basic_conversion -- path/to/document.pdf

# Run with optimized release build (recommended for performance)
cargo run --release --example basic_conversion -- path/to/document.pdf
```

## Available Examples

### 1. Basic Conversion (`basic_conversion.rs`)

**Purpose:** Simple document conversion to markdown.

**Usage:**
```bash
cargo run --example basic_conversion -- document.pdf
```

**What it demonstrates:**
- Creating a DocumentConverter
- Converting a single document
- Accessing conversion results and metadata
- Basic error handling

**Best for:** Getting started with docling-core

---

### 2. OCR Processing (`ocr_processing.rs`)

**Purpose:** Text extraction from scanned documents and images using OCR.

**Usage:**
```bash
cargo run --example ocr_processing -- scanned_document.pdf
cargo run --example ocr_processing -- photo.jpg
```

**What it demonstrates:**
- Enabling OCR for scanned documents
- Format-based OCR selection
- OCR best practices
- Performance considerations

**Best for:** Working with scanned PDFs and images

---

### 3. Batch Processing (`batch_processing.rs`)

**Purpose:** Efficient processing of multiple documents with parallel execution.

**Usage:**
```bash
cargo run --example batch_processing -- document1.pdf document2.docx document3.html
cargo run --example batch_processing -- documents/*.pdf
```

**What it demonstrates:**
- Sequential vs parallel processing
- Using rayon for parallelization
- Performance comparison
- Resource management for batches

**Best for:** Processing large document collections

---

### 4. Error Handling (`error_handling.rs`)

**Purpose:** Robust error handling patterns for production use.

**Usage:**
```bash
cargo run --example error_handling -- document.pdf
cargo run --example error_handling -- nonexistent.pdf  # Test error handling
```

**What it demonstrates:**
- Comprehensive error pattern matching
- Retry logic with exponential backoff
- Pre-conversion validation
- Graceful degradation

**Best for:** Building production-ready applications

---

### 5. Metadata Extraction (`metadata_extraction.rs`)

**Purpose:** Extracting comprehensive metadata from documents.

**Usage:**
```bash
cargo run --example metadata_extraction -- document.pdf
cargo run --example metadata_extraction -- doc1.pdf doc2.docx doc3.html
```

**What it demonstrates:**
- Accessing document metadata
- Content analysis (headers, tables, links)
- Performance metrics
- Document statistics

**Best for:** Document analysis and cataloging

---

### 6. Custom Serialization (`custom_serialization.rs`)

**Purpose:** Customizing markdown output with serialization options.

**Usage:**
```bash
cargo run --example custom_serialization -- document.pdf
```

**What it demonstrates:**
- MarkdownOptions configuration
- Indentation customization
- Escape character handling
- HTML pass-through options

**Best for:** Fine-tuning markdown output

---

### 7. Format Detection (`format_detection.rs`)

**Purpose:** Automatic format detection and multi-format batch conversion.

**Usage:**
```bash
cargo run --example format_detection -- documents/*
cargo run --example format_detection -- file1.pdf file2.docx file3.html
```

**What it demonstrates:**
- Automatic format detection from extensions
- Format-specific processing tips
- Multi-format batch conversion
- Format distribution analysis

**Best for:** Handling diverse document collections

---

### 8. Streaming API (`streaming_api.rs`)

**Purpose:** Stream processing with progress reporting and error recovery.

**Usage:**
```bash
cargo run --example streaming_api -- documents/*.pdf
```

**What it demonstrates:**
- Stream-based batch processing
- Real-time progress reporting
- Comprehensive statistics tracking
- Error recovery strategies

**Best for:** Processing large batches with monitoring

---

### 9. Performance Benchmarking (`performance_bench.rs`)

**Purpose:** Measuring and optimizing conversion performance.

**Usage:**
```bash
cargo run --release --example performance_bench -- document.pdf
cargo run --release --example performance_bench -- document.pdf 10  # 10 iterations
```

**What it demonstrates:**
- Converter instance reuse benefits
- Statistical analysis (mean, median, std dev)
- Throughput measurement
- Performance optimization tips

**Best for:** Performance tuning and optimization

---

### 10. Complete CLI Tool (`cli_tool.rs`)

**Purpose:** Full-featured command-line application with subcommands.

**Usage:**
```bash
# Convert single document
cargo run --example cli_tool -- convert document.pdf

# Convert with OCR
cargo run --example cli_tool -- convert scanned.pdf --ocr -o output.md

# Batch conversion
cargo run --example cli_tool -- batch *.pdf --output ./converted

# Document info
cargo run --example cli_tool -- info report.docx

# Help
cargo run --example cli_tool -- help
```

**What it demonstrates:**
- Subcommand architecture
- Argument parsing
- Multiple output modes
- Production CLI patterns

**Best for:** Building complete CLI applications

---

## Example Categories

### Getting Started
- `basic_conversion.rs` - Start here!
- `ocr_processing.rs` - For scanned documents
- `metadata_extraction.rs` - Extract document info

### Production Use
- `error_handling.rs` - Robust error handling
- `batch_processing.rs` - Efficient batch processing
- `streaming_api.rs` - Progress tracking

### Advanced Topics
- `custom_serialization.rs` - Customize output
- `format_detection.rs` - Multi-format handling
- `performance_bench.rs` - Performance optimization

### Complete Applications
- `cli_tool.rs` - Full CLI application

---

## Common Patterns

### Pattern 1: Single Document Conversion

```rust
use docling_backend::{DocumentConverter, Result};

fn main() -> Result<()> {
    let converter = DocumentConverter::new()?;
    let result = converter.convert("document.pdf")?;
    println!("{}", result.document.markdown);
    Ok(())
}
```

### Pattern 2: Batch Processing

```rust
use docling_backend::{DocumentConverter, Result};

fn main() -> Result<()> {
    let converter = DocumentConverter::new()?;
    let files = vec!["doc1.pdf", "doc2.docx", "doc3.html"];

    for file in files {
        match converter.convert(file) {
            Ok(result) => println!("Converted: {}", file),
            Err(e) => eprintln!("Failed: {}: {}", file, e),
        }
    }
    Ok(())
}
```

### Pattern 3: OCR for Images

```rust
use docling_backend::{DocumentConverter, Result};

fn main() -> Result<()> {
    let converter = DocumentConverter::with_ocr(true)?;
    let result = converter.convert("scanned.pdf")?;
    println!("{}", result.document.markdown);
    Ok(())
}
```

### Pattern 4: Error Handling with Retry

```rust
use docling_backend::{DocumentConverter, DoclingError, Result};
use std::thread;
use std::time::Duration;

fn convert_with_retry(path: &str, max_attempts: u32) -> Result<String> {
    let mut last_error = None;

    for attempt in 1..=max_attempts {
        match DocumentConverter::new() {
            Ok(converter) => match converter.convert(path) {
                Ok(result) => return Ok(result.document.markdown),
                Err(e) => {
                    last_error = Some(e);
                    // Don't retry on certain errors
                    match &last_error {
                        Some(DoclingError::FileNotFound(_)) => break,
                        Some(DoclingError::UnsupportedFormat(_)) => break,
                        _ => {}
                    }
                    if attempt < max_attempts {
                        thread::sleep(Duration::from_secs(2u64.pow(attempt - 1)));
                    }
                }
            },
            Err(e) => last_error = Some(e),
        }
    }

    Err(last_error.unwrap())
}
```

---

## Performance Tips

1. **Reuse Converter Instances**
   - Creating a converter has initialization overhead
   - Reuse the same instance for multiple conversions
   - See: `performance_bench.rs`

2. **Use Release Build for Production**
   ```bash
   cargo run --release --example <name>
   ```
   - 10-100x faster than debug builds
   - Essential for accurate performance measurement

3. **Parallel Processing for Large Batches**
   - Use rayon for parallel processing
   - Each thread needs its own converter instance
   - See: `batch_processing.rs`

4. **Enable OCR Only When Needed**
   - OCR is slower than text extraction
   - Use `DocumentConverter::new()` for digital documents
   - Use `DocumentConverter::with_ocr(true)` only for scanned documents

5. **Monitor Memory for Large Documents**
   - Very large documents may require significant memory
   - Consider processing in chunks for 1000+ page documents
   - See: `streaming_api.rs` for streaming approaches

---

## Troubleshooting

### OCR Not Working

If OCR is not extracting text:

1. Ensure image quality is good (300+ DPI)
2. Verify pdfium library is properly installed (used for PDF rendering)
3. Check ML model weights are available in the expected location
4. Try with a known-good scanned PDF first
5. See: `ocr_processing.rs` for tips

### ML Models Not Found

If you get errors about missing ML models:

```bash
# Ensure environment is set up for PyTorch C++ backend
source setup_env.sh  # Sets LIBTORCH_USE_PYTORCH=1

# Build with PDF ML support
cargo build --features pdf-ml
```

### Slow Performance

If conversions are slow:

1. Make sure you're using release build (`--release`)
2. Reuse DocumentConverter instances
3. Check if OCR is enabled unnecessarily
4. See: `performance_bench.rs` for benchmarking

### Compilation Errors

If examples fail to compile:

```bash
# Update dependencies
cargo update

# Clean and rebuild
cargo clean
cargo build --examples
```

---

## Additional Resources

- **API Documentation:** Run `cargo doc --open` to view full API documentation
- **User Guide:** See `docs/USER_GUIDE.md` for comprehensive usage guide
- **API Cookbook:** See `docs/API_COOKBOOK.md` for more code examples
- **Format Support:** See `docs/FORMAT_SUPPORT.md` for supported formats

---

## Contributing

Found a bug or have an example suggestion? Please open an issue or submit a pull request!

---

## License

MIT License - See LICENSE file for details
