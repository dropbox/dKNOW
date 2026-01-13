# docling-backend

Document parsing backends for docling-rs - provides format-specific parsers that convert 55+ document formats into structured documents.

[![Crates.io](https://img.shields.io/crates/v/docling-backend.svg)](https://crates.io/crates/docling-backend)
[![Documentation](https://docs.rs/docling-backend/badge.svg)](https://docs.rs/docling-backend)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Overview

docling-backend provides the parsing implementations for all supported document formats. Each backend implements the `Backend` trait, converting format-specific data into docling's unified `Document` structure.

## Supported Backends

### Core Document Formats
- **PDF**: Pure Rust/C++ parser with ML-based layout analysis and OCR (via `docling-pdf-ml`)
- **Microsoft Office**: DOCX, PPTX, XLSX (via `docling-microsoft-extended`)
- **HTML**: Web page parsing with semantic structure extraction
- **Markdown**: CommonMark and GitHub-flavored markdown
- **CSV**: Comma-separated values with table detection

### E-books & Publishing
- **EPUB/MOBI/AZW**: E-book formats (via `docling-ebook`)
- **OpenDocument**: ODT, ODS, ODP (via `docling-opendocument`)
- **XPS**: XML Paper Specification (via `docling-xps`)
- **JATS XML**: Scientific article publishing format

### Images
- **PNG, JPEG, TIFF**: Standard image formats with OCR
- **WebP, BMP, GIF**: Modern and legacy image formats
- **HEIF, AVIF**: High-efficiency image formats
- **SVG**: Scalable vector graphics (via `docling-svg`)
- **ICO**: Icon files

### Archives
- **ZIP, TAR, GZ, BZ2, XZ**: Common archive formats
- **7Z, RAR**: Advanced compression formats
- Via `docling-archive` - recursive extraction and conversion

### Email & Calendar
- **EML, MSG**: Email message formats (via `docling-email`)
- **ICS**: iCalendar events and meetings
- **VCF**: vCard contacts

### Multimedia
- **Audio**: MP3, WAV, FLAC, OGG, AAC (metadata + optional transcription)
- **Video**: MP4, WebM, AVI, MKV (metadata + subtitles + optional transcription)
- **SRT, WebVTT**: Subtitle/caption files

### Scientific & Technical
- **JATS XML**: Journal article markup (scientific publishing format)
- **LaTeX**: Basic LaTeX document support (via `docling-latex`)
- **Jupyter Notebooks**: .ipynb files with code and output

### Geospatial & Specialized
- **GPX**: GPS tracks and waypoints (via `docling-gps`)
- **KML**: Keyhole Markup Language for geographic data
- **DICOM**: Medical imaging metadata (via `docling-medical`)
- **CAD/3D**: DXF, STL, OBJ, GLTF (via `docling-cad`)

### Legacy Formats
- **RTF**: Rich Text Format
- **WordPerfect**: Legacy word processor (via `docling-legacy`)
- **Adobe**: XMP metadata, InDesign (via `docling-adobe`)

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
docling-backend = "2.58.0"
```

## Usage

### Using the Unified Converter

The easiest way to use backends is through the `DocumentConverter`:

```rust
use docling_backend::{DocumentConverter, ConversionOptions};

let converter = DocumentConverter::new()?;

// Automatically selects appropriate backend based on file extension
let result = converter.convert("document.pdf")?;
println!("{}", result.document.markdown);
```

### Using Backends Directly

For format-specific control, use backends directly:

```rust
use docling_backend::{PdfBackend, Backend};
use std::path::Path;

let backend = PdfBackend::new();
let document = backend.convert(Path::new("document.pdf"))?;

// Access structured content
for item in &document.texts {
    println!("{}: {}", item.label, item.text);
}
```

### Implementing Custom Backends

Create custom parsers by implementing the `Backend` trait:

```rust
use docling_backend::{Backend, Document};
use std::path::Path;

pub struct MyFormatBackend;

impl Backend for MyFormatBackend {
    fn convert(&self, path: &Path) -> anyhow::Result<Document> {
        // Parse your format
        let content = std::fs::read_to_string(path)?;

        // Build Document structure
        let mut doc = Document::new();
        doc.add_text_item("Title", "section-header");
        doc.add_text_item(&content, "paragraph");

        Ok(doc)
    }

    fn supports_file(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|s| s.to_str())
            .map(|s| s == "myformat")
            .unwrap_or(false)
    }
}
```

## Backend Features

### ML-Powered PDF Parsing

PDF parsing uses pure Rust/C++ ML models for layout analysis:
- **Layout Detection**: PyTorch-based model identifies text, tables, figures, headers
- **OCR**: RapidOCR via ONNX Runtime for scanned documents
- **TableFormer**: Neural network for table structure extraction
- **Reading Order**: ML-based reading order determination

All ML inference runs natively via libtorch (C++ FFI) - no Python required.

### OCR Support

Enable OCR for scanned documents and images:

```rust
use docling_backend::{ConversionOptions, OcrOptions};

let options = ConversionOptions::default()
    .with_ocr(OcrOptions {
        enabled: true,
        languages: vec!["eng".to_string(), "fra".to_string()],
        dpi: 300,
    });

let converter = DocumentConverter::with_options(options)?;
let result = converter.convert("scanned.pdf")?;
```

### Table Extraction

Backends automatically detect and extract tables:

```rust
let result = converter.convert("spreadsheet.xlsx")?;

for item in &result.document.texts {
    if item.label == DocItemLabel::Table {
        println!("Found table:");
        for row in &item.data.rows {
            println!("  {:?}", row);
        }
    }
}
```

## Architecture

```
docling-backend
├── traits.rs          - Backend trait definition
├── converter.rs       - Unified DocumentConverter
└── formats/
    ├── pdf.rs         - PDF parsing
    ├── docx.rs        - Microsoft Word
    ├── html.rs        - HTML parsing
    ├── epub.rs        - E-book formats
    └── ...            - 50+ format parsers
```

Dependencies on format-specific crates:
- `docling-ebook` - EPUB, MOBI, AZW
- `docling-email` - EML, MSG
- `docling-archive` - ZIP, TAR, 7Z, RAR
- `docling-opendocument` - ODT, ODS, ODP
- `docling-xps` - XPS documents
- `docling-svg` - SVG graphics
- And 15+ more specialized format crates

## Testing

```bash
# Unit tests (3000+ tests)
cargo test --lib

# Canonical tests (compare against groundtruth)
cargo test test_canon

# Test specific format
cargo test test_canon_pdf
```

## Performance

Backend performance varies by format:

| Format | Parse Time (avg) | OCR Time (if needed) |
|--------|------------------|----------------------|
| PDF (10 pages) | 50ms | +500ms per page |
| DOCX (50 pages) | 150ms | N/A |
| HTML | 20ms | N/A |
| EPUB | 100ms | N/A |
| XLSX (10 sheets) | 80ms | N/A |

Rust backends are 5-10x faster than pure Python implementations.

## Error Handling

All backends return `anyhow::Result<Document>`:

```rust
match converter.convert("document.pdf") {
    Ok(result) => {
        println!("Success: {} items", result.document.texts.len());
    }
    Err(e) => {
        eprintln!("Conversion failed: {}", e);
        // e contains full error chain for debugging
    }
}
```

## Contributing

Contributions are welcome! To add a new backend:

1. Create a new module in `src/`
2. Implement the `Backend` trait
3. Add format detection logic
4. Register in `converter.rs`
5. Add integration tests

See [CONTRIBUTING.md](../../CONTRIBUTING.md) for details.

## License

Licensed under the MIT License. See [LICENSE](../../LICENSE) for details.

## Links

- **Repository**: https://github.com/ayates_dbx/docling_rs
- **Documentation**: https://docs.rs/docling-backend
- **Issues**: https://github.com/ayates_dbx/docling_rs/issues
- **Python docling**: https://github.com/docling-project/docling

## Related Crates

- `docling-core` - Core document types and serialization
- `docling-cli` - Command-line interface
- Format-specific crates: `docling-ebook`, `docling-email`, `docling-archive`, etc.
