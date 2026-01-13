# docling-core

Core document types and serialization for docling-rs - a powerful document conversion library that extracts structured content from 65+ document formats.

[![Crates.io](https://img.shields.io/crates/v/docling-core.svg)](https://crates.io/crates/docling-core)
[![Documentation](https://docs.rs/docling-core/badge.svg)](https://docs.rs/docling-core)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Features

- **65+ Document Formats**: PDF, DOCX, PPTX, XLSX, HTML, Markdown, LaTeX, iWork, e-books, images, archives, email, 3D/CAD, geospatial, medical imaging, and more
- **Structured Output**: Export to Markdown, HTML, JSON, or YAML with preserved document structure
- **Type-Safe Document Model**: Strong type system for DocItems (text, table, picture, section, etc.)
- **High Performance**: Optimized Rust serializers with streaming support
- **OCR Support**: Optical character recognition for scanned documents and images (optional)
- **Table Extraction**: Intelligent table detection and conversion to multiple formats

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
docling-core = "2.58.0"
```

## Quick Start

### Basic Document Conversion

```rust
use docling_backend::DocumentConverter;  // Note: DocumentConverter is in docling-backend crate
use docling_core::Result;

fn main() -> Result<()> {
    // Create a converter (text-only mode by default)
    let converter = DocumentConverter::new()?;

    // Convert a PDF to markdown
    let result = converter.convert("document.pdf")?;

    println!("Markdown output:\n{}", result.document.markdown);
    println!("Pages: {:?}", result.document.metadata.num_pages);

    Ok(())
}
```

### Choose Output Format

```rust
use docling_backend::{DocumentConverter, ConversionOptions};  // Note: DocumentConverter is in docling-backend crate
use docling_core::OutputFormat;

let converter = DocumentConverter::with_options(
    ConversionOptions::default()
        .output_format(OutputFormat::Html)
)?;

let result = converter.convert("presentation.pptx")?;
println!("{}", result.document.html.unwrap());
```

### Working with Document Structure

```rust
use docling_backend::DocumentConverter;  // Note: DocumentConverter is in docling-backend crate
use docling_core::DocItemLabel;

let converter = DocumentConverter::new()?;
let result = converter.convert("document.pdf")?;

// Access structured document items
for item in &result.document.texts {
    match item.label {
        DocItemLabel::SectionHeader => {
            println!("Section: {}", item.text);
        }
        DocItemLabel::Table => {
            println!("Found table with {} rows", item.data.rows.len());
        }
        _ => {}
    }
}
```

### Batch Processing

```rust
use docling_backend::DocumentConverter;  // Note: DocumentConverter is in docling-backend crate

let converter = DocumentConverter::new()?;

for entry in glob::glob("documents/*.pdf")? {
    let path = entry?;
    match converter.convert(&path) {
        Ok(result) => {
            println!("✓ {} ({} pages)", path.display(), result.document.metadata.num_pages);
        }
        Err(e) => {
            eprintln!("✗ {}: {}", path.display(), e);
        }
    }
}
```

## Supported Formats

| Category | Formats |
|----------|---------|
| **Documents** | PDF, DOCX, DOC, PPTX, PPT, XLSX, XLS, RTF, OpenDocument (ODT, ODS, ODP) |
| **Web** | HTML, Markdown, AsciiDoc, CSV |
| **Images** | PNG, JPEG, TIFF, WebP, BMP, GIF, SVG, ICO, HEIF, AVIF |
| **E-books** | EPUB, MOBI, AZW, FB2 |
| **Archives** | ZIP, TAR, GZ, BZ2, XZ, 7Z, RAR |
| **Email** | EML, MSG |
| **Calendar** | ICS (iCalendar), VCF (vCard) |
| **Scientific** | JATS XML, LaTeX (basic), Jupyter Notebooks |
| **Multimedia** | MP3, WAV, FLAC, OGG (metadata), MP4, AVI, WebM (metadata + subtitles) |
| **Geospatial** | GPX, KML |
| **Medical** | DICOM (metadata) |
| **CAD/3D** | DXF, STL, OBJ, GLTF |
| **Legacy** | WordPerfect, Adobe formats (XMP metadata) |

## Optional Features

```toml
[dependencies]
docling-core = { version = "2.58.0", features = ["video", "transcription"] }
```

- `video`: Enable video format support (MP4, WebM, AVI, MKV)
- `video-transcription`: Enable video transcription with Whisper models
- `transcription`: Enable audio transcription for audio files

## Architecture

docling-core provides the fundamental types and serializers:

- **Document Types**: `ConvertedDocument`, `DocItem`, `TableData`, `BoundingBox`, etc.
- **Serializers**: Markdown, HTML, JSON, YAML exporters with configurable options
- **Format Support**: Integrates with format-specific parsers from `docling-backend`
- **Hybrid Mode**: Combines Python ML parsing with Rust serialization for best performance

## Examples

See the [examples directory](../../examples/) for more:

- `basic_conversion.rs` - Simple document conversion
- `batch_processing.rs` - Process multiple files with error handling
- `custom_options.rs` - Configure OCR, output format, and features
- `benchmark.rs` - Performance testing with statistics

## Performance

Rust serializers provide 5-10x performance improvements over Python for large documents:

- **PDF (10 pages)**: ~100ms (Rust) vs ~800ms (Python)
- **DOCX (50 pages)**: ~300ms (Rust) vs ~2.5s (Python)
- **Memory**: 2-3x lower peak memory usage

## Testing

Run the test suite:

```bash
# Unit tests
cargo test --lib

# Integration tests (requires Python docling v2.58.0)
USE_HYBRID_SERIALIZER=1 cargo test

# Canonical test suite
USE_HYBRID_SERIALIZER=1 cargo test test_canon
```

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](../../CONTRIBUTING.md) for guidelines.

## License

Licensed under the MIT License. See [LICENSE](../../LICENSE) for details.

## Links

- **Repository**: https://github.com/dropbox/dKNOW/docling_rs
- **Documentation**: https://docs.rs/docling-core
- **Issues**: https://github.com/dropbox/dKNOW/docling_rs/issues
- **Python docling**: https://github.com/docling-project/docling

## Related Crates

- `docling-backend` - Document parsing backends (PDF, DOCX, HTML, etc.)
- `docling-cli` - Command-line interface for docling
- Format-specific crates: `docling-ebook`, `docling-email`, `docling-archive`, etc.
