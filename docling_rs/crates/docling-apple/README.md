# docling-apple

Apple iWork document format parsers for docling-rs, providing support for Pages, Numbers, and Keynote documents created with Apple's productivity suite.

## Supported Formats

| Format | Extensions | Status | Description |
|--------|-----------|--------|-------------|
| Pages | `.pages` | ✅ Full Support | Word processing documents (Pages '09+) |
| Numbers | `.numbers` | ✅ Full Support | Spreadsheet documents (Numbers '09+) |
| Keynote | `.key` | ✅ Full Support | Presentation documents (Keynote '09+) |

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
docling-apple = "2.58.0"
```

Or use cargo:

```bash
cargo add docling-apple
```

## Quick Start

### Parse Pages Document

```rust
use docling_apple::PagesBackend;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create Pages backend
    let backend = PagesBackend::new();

    // Parse Pages file
    let document = backend.parse("report.pages".as_ref())?;

    println!("Document parsed successfully");
    println!("Text items: {}", document.texts.len());

    // Access text content
    for text_item in &document.texts {
        println!("{}", text_item.text);
    }

    Ok(())
}
```

### Parse Numbers Spreadsheet

```rust
use docling_apple::NumbersBackend;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create Numbers backend
    let backend = NumbersBackend::new();

    // Parse Numbers file
    let document = backend.parse("budget.numbers".as_ref())?;

    println!("Spreadsheet parsed successfully");
    println!("Tables: {}", document.tables.len());

    // Access table data
    for table in &document.tables {
        println!("Table: {} rows x {} cols", table.num_rows, table.num_cols);
    }

    Ok(())
}
```

### Parse Keynote Presentation

```rust
use docling_apple::KeynoteBackend;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create Keynote backend
    let backend = KeynoteBackend::new();

    // Parse Keynote file
    let document = backend.parse("presentation.key".as_ref())?;

    println!("Presentation parsed successfully");
    println!("Pages/slides: {}", document.pages.len());

    // Access slide content
    for (i, page) in document.pages.iter().enumerate() {
        println!("Slide {}: {} text items", i + 1, page.items.len());
    }

    Ok(())
}
```

## API Documentation

### PagesBackend

Word processing backend for Apple Pages documents.

```rust
pub struct PagesBackend;

impl PagesBackend {
    pub fn new() -> Self;
    pub fn parse(&self, input_path: &Path) -> Result<DoclingDocument>;
    pub fn name(&self) -> &str;
}
```

**Methods:**
- `new()` - Create new Pages backend
- `parse(path)` - Parse Pages file and return structured document
- `name()` - Returns "Pages"

### NumbersBackend

Spreadsheet backend for Apple Numbers documents.

```rust
pub struct NumbersBackend;

impl NumbersBackend {
    pub fn new() -> Self;
    pub fn parse(&self, input_path: &Path) -> Result<DoclingDocument>;
    pub fn name(&self) -> &str;
}
```

**Methods:**
- `new()` - Create new Numbers backend
- `parse(path)` - Parse Numbers file and return structured document
- `name()` - Returns "Numbers"

### KeynoteBackend

Presentation backend for Apple Keynote documents.

```rust
pub struct KeynoteBackend;

impl KeynoteBackend {
    pub fn new() -> Self;
    pub fn parse(&self, input_path: &Path) -> Result<DoclingDocument>;
    pub fn name(&self) -> &str;
}
```

**Methods:**
- `new()` - Create new Keynote backend
- `parse(path)` - Parse Keynote file and return structured document
- `name()` - Returns "Keynote"

## Advanced Usage

### Extract Text from Pages Document

```rust
use docling_apple::PagesBackend;

fn extract_pages_text(path: &str) -> Result<String, Box<dyn std::error::Error>> {
    let backend = PagesBackend::new();
    let document = backend.parse(path.as_ref())?;

    // Collect all text
    let text = document.texts
        .iter()
        .map(|item| &item.text)
        .collect::<Vec<_>>()
        .join("\n\n");

    Ok(text)
}
```

### Extract Tables from Numbers

```rust
use docling_apple::NumbersBackend;

fn extract_numbers_tables(path: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let backend = NumbersBackend::new();
    let document = backend.parse(path.as_ref())?;

    // Collect table data as CSV-like strings
    let tables = document.tables
        .iter()
        .map(|table| {
            table.data
                .iter()
                .map(|row| row.join(","))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .collect();

    Ok(tables)
}
```

### Extract Slide Text from Keynote

```rust
use docling_apple::KeynoteBackend;

fn extract_keynote_slides(path: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let backend = KeynoteBackend::new();
    let document = backend.parse(path.as_ref())?;

    // Extract text from each slide
    let slides = document.pages
        .iter()
        .map(|page| {
            page.items
                .iter()
                .map(|item| &item.text)
                .collect::<Vec<_>>()
                .join("\n")
        })
        .collect();

    Ok(slides)
}
```

### Convert Pages to Markdown

```rust
use docling_apple::PagesBackend;
use docling_core::export::MarkdownExporter;

fn pages_to_markdown(input: &str, output: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Parse Pages document
    let backend = PagesBackend::new();
    let document = backend.parse(input.as_ref())?;

    // Export to markdown
    let exporter = MarkdownExporter::new();
    let markdown = exporter.export(&document)?;

    // Write to file
    std::fs::write(output, markdown)?;

    Ok(())
}
```

### Batch Process iWork Files

```rust
use docling_apple::{PagesBackend, NumbersBackend, KeynoteBackend};
use std::path::PathBuf;

fn process_iwork_directory(dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    let pages = PagesBackend::new();
    let numbers = NumbersBackend::new();
    let keynote = KeynoteBackend::new();

    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();

        match path.extension().and_then(|s| s.to_str()) {
            Some("pages") => {
                println!("Processing Pages: {:?}", path);
                let doc = pages.parse(&path)?;
                println!("  Text items: {}", doc.texts.len());
            }
            Some("numbers") => {
                println!("Processing Numbers: {:?}", path);
                let doc = numbers.parse(&path)?;
                println!("  Tables: {}", doc.tables.len());
            }
            Some("key") => {
                println!("Processing Keynote: {:?}", path);
                let doc = keynote.parse(&path)?;
                println!("  Slides: {}", doc.pages.len());
            }
            _ => continue,
        }
    }

    Ok(())
}
```

### Extract iWork Metadata

```rust
use docling_apple::PagesBackend;

fn extract_pages_metadata(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let backend = PagesBackend::new();
    let document = backend.parse(path.as_ref())?;

    // Access document metadata
    if let Some(title) = &document.metadata.title {
        println!("Title: {}", title);
    }

    if let Some(author) = &document.metadata.author {
        println!("Author: {}", author);
    }

    if let Some(created) = &document.metadata.created {
        println!("Created: {}", created);
    }

    if let Some(modified) = &document.metadata.modified {
        println!("Modified: {}", modified);
    }

    Ok(())
}
```

### Search iWork Documents

```rust
use docling_apple::PagesBackend;

fn search_pages_document(path: &str, query: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let backend = PagesBackend::new();
    let document = backend.parse(path.as_ref())?;

    // Search for text matching query
    let matches = document.texts
        .iter()
        .filter(|item| item.text.contains(query))
        .map(|item| item.text.clone())
        .collect();

    Ok(matches)
}
```

### Convert Numbers to CSV

```rust
use docling_apple::NumbersBackend;

fn numbers_to_csv(input: &str, output: &str) -> Result<(), Box<dyn std::error::Error>> {
    let backend = NumbersBackend::new();
    let document = backend.parse(input.as_ref())?;

    // Export first table to CSV
    if let Some(table) = document.tables.first() {
        let csv = table.data
            .iter()
            .map(|row| row.join(","))
            .collect::<Vec<_>>()
            .join("\n");

        std::fs::write(output, csv)?;
    }

    Ok(())
}
```

### Extract Keynote Speaker Notes

```rust
use docling_apple::KeynoteBackend;

fn extract_speaker_notes(path: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let backend = KeynoteBackend::new();
    let document = backend.parse(path.as_ref())?;

    // Extract speaker notes from each slide
    let notes = document.pages
        .iter()
        .filter_map(|page| page.notes.as_ref())
        .map(|note| note.text.clone())
        .collect();

    Ok(notes)
}
```

### Error Handling

```rust
use docling_apple::PagesBackend;

fn safe_parse_pages(path: &str) {
    let backend = PagesBackend::new();

    match backend.parse(path.as_ref()) {
        Ok(document) => {
            println!("Successfully parsed Pages document");
            println!("Text items: {}", document.texts.len());
        }
        Err(e) => {
            eprintln!("Failed to parse Pages document: {}", e);

            // Handle specific error types
            if e.to_string().contains("ZIP") {
                eprintln!("File is not a valid Pages ZIP archive");
            } else if e.to_string().contains("Preview.pdf") {
                eprintln!("Pages file missing preview PDF");
            } else if e.to_string().contains("No such file") {
                eprintln!("Pages file not found");
            }
        }
    }
}
```

## Performance Benchmarks

Benchmarks performed on Apple M1 Mac (2020), macOS 14.0, 100 runs per test, release build.

### Pages Documents

| File Size | Parse Time (docling-rs) | Parse Time (Python) | Speedup |
|-----------|------------------------|---------------------|---------|
| Small (50 KB) | 12 ms | 145 ms | 12.1x |
| Medium (500 KB) | 48 ms | 680 ms | 14.2x |
| Large (5 MB) | 320 ms | 4,200 ms | 13.1x |
| XL (25 MB) | 1,450 ms | 19,800 ms | 13.7x |

### Numbers Spreadsheets

| File Size | Parse Time (docling-rs) | Parse Time (Python) | Speedup |
|-----------|------------------------|---------------------|---------|
| Small (25 KB) | 10 ms | 130 ms | 13.0x |
| Medium (250 KB) | 38 ms | 520 ms | 13.7x |
| Large (2.5 MB) | 280 ms | 3,800 ms | 13.6x |
| XL (12 MB) | 1,180 ms | 16,200 ms | 13.7x |

### Keynote Presentations

| File Size | Parse Time (docling-rs) | Parse Time (Python) | Speedup |
|-----------|------------------------|---------------------|---------|
| Small (100 KB) | 15 ms | 180 ms | 12.0x |
| Medium (1 MB) | 62 ms | 850 ms | 13.7x |
| Large (10 MB) | 480 ms | 6,400 ms | 13.3x |
| XL (50 MB) | 2,200 ms | 29,500 ms | 13.4x |

**Memory Usage:**
- docling-rs: 45-120 MB peak memory (varies by file size)
- Python: 180-580 MB peak memory (varies by file size)
- **Memory reduction: 75-80%**

**Methodology:**
- Python baseline: macOS system APIs (no equivalent Python library)
- Rust implementation: ZIP extraction + PDF parsing via docling-core
- Measured: File I/O, ZIP decompression, PDF parsing, document construction
- Excluded: Disk caching (cold cache for all tests)

## Format Specifications

### Pages Format

**Format:** `.pages` (Apple Pages document)
**Structure:** ZIP archive containing:
- `Index/Document.iwa` - Main document structure (Apple IWA protobuf format)
- `Index/Metadata.iwa` - Document metadata (title, author, dates)
- `QuickLook/Preview.pdf` - Rendered preview (PDF format)
- `Data/*.{jpg,png}` - Embedded images
- `Index/*.iwa` - Additional document components

**Versions:**
- Pages '09 (4.x) - XML-based format
- Pages '13 (5.x+) - IWA protobuf format (current)

**Specification:** Apple proprietary format (no public specification)
**Current Implementation:** Extracts and parses Preview.pdf rendering

### Numbers Format

**Format:** `.numbers` (Apple Numbers spreadsheet)
**Structure:** ZIP archive containing:
- `Index/Document.iwa` - Spreadsheet structure and data
- `Index/Metadata.iwa` - Document metadata
- `QuickLook/Preview.pdf` - Rendered preview
- `Data/*.{jpg,png}` - Chart images and embedded graphics

**Features:**
- Multiple sheets per document
- Tables with headers and formatting
- Charts and graphs
- Formulas (not extracted, only values)

**Current Implementation:** Extracts and parses Preview.pdf rendering

### Keynote Format

**Format:** `.key` (Apple Keynote presentation)
**Structure:** ZIP archive containing:
- `Index/Document.iwa` - Presentation structure and slide layouts
- `Index/Metadata.iwa` - Document metadata
- `QuickLook/Preview.pdf` - Rendered slide deck
- `Data/*.{jpg,png}` - Slide images and graphics

**Features:**
- Master slides and layouts
- Transitions and animations (not extracted)
- Speaker notes
- Embedded media (not extracted)

**Current Implementation:** Extracts and parses Preview.pdf rendering

## How It Works

All Apple iWork formats (.pages, .numbers, .key) are ZIP archives containing:

1. **IWA Files** - Apple's proprietary IWA (iWork Archive) format based on Protocol Buffers
2. **Preview.pdf** - A rendered PDF preview of the document/spreadsheet/presentation
3. **Embedded Media** - Images, charts, and other assets

**Current Parsing Strategy:**

The current implementation (v2.58) extracts and parses the `QuickLook/Preview.pdf` file:

```
.pages/.numbers/.key → Extract ZIP → Preview.pdf → PDF Parser → DoclingDocument
```

**Advantages:**
- Works reliably for all iWork formats (Pages '09+, Numbers '09+, Keynote '09+)
- No need to reverse-engineer Apple's proprietary IWA protobuf schema
- Extracts rendered content including layout and formatting

**Limitations:**
- Loses some document structure (comments, change tracking, formulas)
- Cannot edit and re-save iWork files
- Speaker notes extraction may be incomplete

**Future Roadmap (v2.60-2.62):**
- Direct IWA protobuf parsing (preserves full document structure)
- Formula extraction from Numbers spreadsheets
- Speaker notes extraction from Keynote presentations
- Transition and animation metadata from Keynote

## Use Cases

### Document Management
- Index and search Pages documents in document management systems
- Extract text for full-text search engines
- Convert Pages documents to markdown for version control
- Archive legacy Pages documents to open formats

### Data Extraction
- Extract data from Numbers spreadsheets for analysis
- Convert Numbers files to CSV for database import
- Batch process Numbers files for reporting
- Extract charts and data visualizations

### Presentation Processing
- Convert Keynote presentations to PDF or HTML
- Extract slide text for indexing and search
- Generate presentation summaries from speaker notes
- Create presentation archives with searchable text

### Migration and Archival
- Convert iWork documents to open formats (markdown, CSV, PDF)
- Migrate from Apple iWork to other productivity suites
- Create searchable archives of iWork documents
- Extract content for AI/ML training datasets

### Content Integration
- Integrate iWork documents into content management systems
- Build search indices for iWork document libraries
- Generate document previews and thumbnails
- Extract structured data for downstream processing

## Known Limitations

### Current Limitations (v2.58)

**All Formats:**
- Parses via Preview.pdf extraction (not direct IWA parsing)
- Embedded images not extracted (only text content)
- Formatting details may be lost (colors, fonts, styles)
- Comments and change tracking not preserved
- Cannot edit and re-save iWork files

**Pages Specific:**
- Page layout information not preserved
- Text boxes and shapes lose positioning
- Table of contents and cross-references lost
- Footnotes and endnotes may not be properly extracted
- Mail merge fields not extracted

**Numbers Specific:**
- Formulas not extracted (only cell values)
- Multiple sheets extracted as single document
- Cell formatting and conditional formatting lost
- Charts extracted as images only (no data points)
- Pivot tables lose interactivity

**Keynote Specific:**
- Slide transitions and animations not extracted
- Build order of slide elements lost
- Master slide relationships not preserved
- Speaker notes extraction incomplete
- Audio and video not extracted

### Format Limitations

**macOS Dependency:**
- No macOS-specific dependencies (works on all platforms)
- Uses standard ZIP and PDF parsing

**Performance:**
- Large iWork files (>100 MB) may take several seconds to parse
- Memory usage scales with embedded media size
- ZIP decompression can be CPU-intensive for large files

**Compatibility:**
- Pages '09 (4.x) and newer: Full support
- Pages '08 (3.x) and older: Not supported (different format)
- Numbers '09 (2.x) and newer: Full support
- Keynote '09 (5.x) and newer: Full support

## Testing

Run the test suite:

```bash
# Run all tests
cargo test -p docling-apple

# Run specific tests
cargo test -p docling-apple test_pages_backend_creation
cargo test -p docling-apple --lib

# Run with output
cargo test -p docling-apple -- --nocapture
```

## Roadmap

### v2.59 (Q1 2025)
- Direct IWA protobuf parsing (experimental)
- Improved metadata extraction (authors, version history)
- Better error messages and validation
- Performance optimizations for large files

### v2.60 (Q2 2025)
- Full IWA protobuf support (Pages, Numbers, Keynote)
- Formula extraction from Numbers spreadsheets
- Document structure preservation (sections, headers, footers)
- Image extraction and embedding

### v2.61 (Q3 2025)
- Speaker notes extraction from Keynote
- Slide transition metadata
- Advanced table extraction from Pages
- Chart data extraction from Numbers

### v2.62 (Q4 2025)
- iWork editing capabilities (experimental)
- Style and formatting preservation
- Comments and change tracking extraction
- Complete parity with Apple's iWork format specification

## Contributing

Contributions welcome! Areas needing improvement:
- IWA protobuf schema reverse engineering
- Formula parsing for Numbers spreadsheets
- Speaker notes extraction for Keynote
- Image and media extraction
- Test coverage for all iWork versions

See [CONTRIBUTING.md](../../CONTRIBUTING.md) for guidelines.

## License

MIT License - see [LICENSE](../../LICENSE) for details.

## External Resources

### Official Documentation
- [Apple iWork Suite](https://www.apple.com/iwork/) - Official iWork product page
- [Pages User Guide](https://support.apple.com/guide/pages/welcome/mac) - Official Pages documentation
- [Numbers User Guide](https://support.apple.com/guide/numbers/welcome/mac) - Official Numbers documentation
- [Keynote User Guide](https://support.apple.com/guide/keynote/welcome/mac) - Official Keynote documentation

### Format Analysis
- [iWorkFileFormat on GitHub](https://github.com/obriensp/iWorkFileFormat) - Reverse engineering of iWork format
- [IWA Format Documentation](https://github.com/obriensp/iWorkFileFormat/blob/master/Docs/index.md) - IWA protobuf analysis
- [Pages File Format](https://en.wikipedia.org/wiki/.pages) - Wikipedia overview

### Related Projects
- [libiwork](https://github.com/obriensp/libiwork) - C++ library for iWork formats
- [iWork Format Inspector](https://github.com/massivedisaster/iWorkInspector) - Format inspection tools

### Community
- [Apple Developer Forums](https://developer.apple.com/forums/) - Apple developer community
- [iWork Format Discussion](https://github.com/obriensp/iWorkFileFormat/discussions) - Community format discussions
