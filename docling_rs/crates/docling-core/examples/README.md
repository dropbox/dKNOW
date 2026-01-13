# Docling-Core Examples

This directory contains working examples demonstrating how to use the docling-core library.

Each example is a standalone Rust program that compiles and runs independently.

## Quick Start

Run any example with:
```bash
cargo run --example <example_name> -- [arguments]
```

## Examples

### 1. `basic_conversion.rs` - Single Document Conversion

**Level:** Beginner
**Purpose:** Convert a single document to markdown

**Usage:**
```bash
cargo run --example basic_conversion -- path/to/document.pdf
```

**What it demonstrates:**
- Creating a DocumentConverter
- Converting a single file
- Accessing markdown output
- Reading document metadata (pages, characters, title)
- Measuring conversion performance

**Best for:** Quick document conversion, learning the basics

---

### 2. `batch_processing.rs` - Multi-Document Processing

**Level:** Intermediate
**Purpose:** Process multiple documents and collect statistics

**Usage:**
```bash
cargo run --example batch_processing -- file1.pdf file2.docx file3.html
```

**What it demonstrates:**
- Batch processing pattern
- Error handling (continue on failure)
- Statistics collection (success rate, throughput)
- Performance monitoring
- Reusing converter for multiple files

**Output example:**
```
Processing 3 documents...

[1/3] Processing: file1.pdf
  ✓ Success: 15234 chars in 2.3s
[2/3] Processing: file2.docx
  ✓ Success: 8421 chars in 1.2s
[3/3] Processing: file3.html
  ✗ Failed: Unsupported format

=== Conversion Summary ===
Total documents: 3
Successful: 2
Failed: 1
Success rate: 66.7%
Total time: 3.50s
Average time: 1.17s/doc
Total characters extracted: 23655
Throughput: 6759 chars/sec
```

**Best for:** Production workloads, folder processing, monitoring

---

### 3. `structured_extraction.rs` - Working with DocItems

**Level:** Advanced
**Purpose:** Extract and analyze structured content

**Usage:**
```bash
cargo run --example structured_extraction -- path/to/document.docx
```

**What it demonstrates:**
- Accessing DocItems (structured content)
- Filtering by content type (headings, tables, lists, pictures)
- Analyzing document structure
- Building custom processors
- Extracting specific elements

**Output example:**
```
=== Document Structure ===

1. Heading (level 1): Introduction to Rust
2. Text: Rust is a systems programming language...
3. Table: 5 rows × 3 cols
   First cell: Feature
4. List Item (•): Memory safety without garbage collection...
5. Picture/Image

=== Statistics ===
Total items: 47
Headings: 8
Text blocks: 32
Tables: 3
List items: 4
Pictures: 0

=== All Headings ===
Introduction to Rust
  Memory Safety
  Performance
  Concurrency
Advanced Topics
  Macros
  Unsafe Code
Conclusion
```

**Best for:**
- Custom document analysis
- Building table of contents
- Extracting tables for data processing
- Content filtering and transformation

---

### 4. `json_roundtrip.rs` - JSON Serialization and Round-Trip

**Level:** Intermediate
**Purpose:** Demonstrate JSON backend for round-trip workflows

**Usage:**
```bash
cargo run --example json_roundtrip
```

**What it demonstrates:**
- Converting documents using Rust backend
- Serializing Document to JSON
- Loading JSON back into Document
- Verifying round-trip integrity
- Using JSON for intermediate storage

**Output example:**
```
=== JSON Round-Trip Example ===

Step 1: Converting markdown document...
  Original markdown length: 136 chars
  Original format: Md
  Has structured content: true
  Content blocks: 8

Step 2: Serializing to JSON...
  JSON size: 3461 bytes
  Saved to: /tmp/test_roundtrip.json

Step 3: Loading JSON back into Document...
  Loaded markdown length: 136 chars
  Loaded format: Md
  Has structured content: true
  Content blocks: 8

Step 4: Verifying round-trip integrity...
  Markdown content matches: ✓
  Original format preserved: ✓
  Character count matches: ✓
  Structured content matches: ✓

✓ Round-trip successful! Document integrity preserved.
```

**Best for:**
- Testing document processing pipelines
- Saving/loading intermediate conversion results
- Debugging document structure and content
- Interoperability between Python and Rust docling
- Building custom document workflows

---

## Supported Formats

All examples work with 60+ document formats including:

- **Documents:** PDF, DOCX, PPTX, XLSX, ODT, ODS, ODP, RTF
- **Web:** HTML, MHTML, XML
- **Text:** Markdown, AsciiDoc, LaTeX, reStructuredText
- **E-books:** EPUB, MOBI, FB2
- **Images:** PNG, JPEG, TIFF, WEBP, BMP, GIF, HEIF, AVIF, SVG
- **Archives:** ZIP, TAR, 7Z, RAR
- **Email:** EML, MBOX, MSG, VCF
- **Apple:** Pages, Numbers, Keynote
- **Scientific:** JATS (medical/scientific papers)
- **Data:** CSV, JSON_DOCLING (native format), YAML
- **Specialized:** ICS (calendar), IPYNB (Jupyter), GPX (GPS), KML (maps), DICOM (medical), CAD formats

See [FORMAT_PROCESSING_GRID.md](../../../FORMAT_PROCESSING_GRID.md) for complete list.

---

## Common Patterns

### Error Handling

All examples demonstrate proper error handling with `Result<T, DoclingError>`:

```rust
match converter.convert(file_path) {
    Ok(result) => {
        // Process successful conversion
        println!("{}", result.document.markdown);
    }
    Err(e) => {
        // Handle error gracefully
        eprintln!("Conversion failed: {}", e);
    }
}
```

### Reusing Converter

Create the converter once, reuse for multiple files:

```rust
let converter = DocumentConverter::new()?;

for file in files {
    let result = converter.convert(file)?;
    // Process result...
}
```

### Working with Metadata

Access document metadata:

```rust
let result = converter.convert(file_path)?;

if let Some(pages) = result.document.metadata.num_pages {
    println!("Pages: {}", pages);
}
println!("Characters: {}", result.document.metadata.num_characters);
if let Some(title) = &result.document.metadata.title {
    println!("Title: {}", title);
}
```

### Performance Monitoring

Track conversion performance:

```rust
let result = converter.convert(file_path)?;
println!("Converted in {:?}", result.latency);
```

---

## Building Examples

Build all examples:
```bash
cargo build --package docling-core --examples
```

Build specific example:
```bash
cargo build --package docling-core --example basic_conversion
```

---

## Next Steps

1. **Try the examples** - Run them with your own documents
2. **Modify them** - Adapt to your use case
3. **Read the API docs** - `cargo doc --package docling-core --open`
4. **Check the CLI** - See `crates/docling-cli/` for command-line tool
5. **Explore formats** - See `FORMAT_PROCESSING_GRID.md` for supported formats

---

## Need Help?

- **Issues:** https://github.com/dropbox/dKNOW/docling_rs/issues
- **API Docs:** Run `cargo doc --package docling-core --open`
- **Main README:** See repository root for installation and setup
- **Python Reference:** https://github.com/docling-project/docling

---

## Contributing Examples

Want to add an example? Follow these guidelines:

1. **Standalone:** Each example should be self-contained
2. **Documented:** Include doc comments explaining purpose and usage
3. **Tested:** Must compile with `cargo build --examples`
4. **Practical:** Demonstrate real-world use case
5. **Progressive:** Start simple, add complexity gradually

Example template:
```rust
//! Brief description of what this example does
//!
//! Usage:
//! ```bash
//! cargo run --example my_example -- arguments
//! ```

use docling_backend::DocumentConverter;  // Note: DocumentConverter is in docling-backend crate
use docling_core::DoclingError;

fn main() -> Result<(), DoclingError> {
    // Your example code here
    Ok(())
}
```
