# docling-opendocument

OpenDocument Format (ODF) parsers for docling-rs, supporting the open standard for office documents.

## Supported Formats

| Format | Extensions | Status | Description |
|--------|-----------|--------|-------------|
| ODT | `.odt` | ✅ Full Support | OpenDocument Text (word processor) |
| ODS | `.ods` | ✅ Full Support | OpenDocument Spreadsheet |
| ODP | `.odp` | ✅ Full Support | OpenDocument Presentation |
| OTT | `.ott` | ✅ Full Support | OpenDocument Text Template |
| OTS | `.ots` | ✅ Full Support | OpenDocument Spreadsheet Template |
| OTP | `.otp` | ✅ Full Support | OpenDocument Presentation Template |

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
docling-opendocument = "2.58.0"
```

Or use cargo:

```bash
cargo add docling-opendocument
```

## Quick Start

### Parse ODT (Text Document)

```rust
use docling_opendocument::odt::parse_odt_file;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let doc = parse_odt_file("document.odt")?;

    println!("Title: {:?}", doc.title);
    println!("Author: {:?}", doc.author);
    println!("Page count: {:?}", doc.page_count);
    println!("\nContent:\n{}", doc.text);

    Ok(())
}
```

### Parse ODS (Spreadsheet)

```rust
use docling_opendocument::ods::parse_ods_file;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let doc = parse_ods_file("spreadsheet.ods")?;

    println!("Sheets: {:?}", doc.sheet_names);
    println!("Total rows: {}", doc.text.lines().count());

    for (idx, sheet_name) in doc.sheet_names.iter().enumerate() {
        println!("\n## Sheet {}: {}", idx + 1, sheet_name);
    }

    println!("\nContent:\n{}", doc.text);

    Ok(())
}
```

### Parse ODP (Presentation)

```rust
use docling_opendocument::odp::parse_odp_file;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let doc = parse_odp_file("presentation.odp")?;

    println!("Title: {:?}", doc.title);
    println!("Slide count: {}", doc.slide_count);

    println!("\nContent:\n{}", doc.text);

    Ok(())
}
```

## Data Structures

### OdtDocument

```rust
pub struct OdtDocument {
    pub title: Option<String>,
    pub author: Option<String>,
    pub subject: Option<String>,
    pub keywords: Option<String>,
    pub description: Option<String>,
    pub page_count: Option<u32>,
    pub text: String,
}
```

### OdsDocument

```rust
pub struct OdsDocument {
    pub title: Option<String>,
    pub author: Option<String>,
    pub sheet_names: Vec<String>,
    pub text: String,
}
```

### OdpDocument

```rust
pub struct OdpDocument {
    pub title: Option<String>,
    pub author: Option<String>,
    pub slide_count: usize,
    pub text: String,
}
```

## Features

### ODT (Text Documents)
- Paragraph extraction with style preservation
- Heading hierarchy (H1-H6)
- Lists (ordered and unordered)
- Tables (converted to Markdown tables)
- Footnotes and endnotes
- Metadata extraction (title, author, subject, keywords, description)
- Page count
- Embedded images (references extracted)
- Text formatting (bold, italic, underline) preserved in Markdown

### ODS (Spreadsheets)
- Multiple sheet support
- Cell data extraction (text, numbers, formulas)
- Table formatting preserved
- Sheet names extraction
- Metadata extraction
- Empty cell handling
- Merged cell detection

### ODP (Presentations)
- Slide extraction
- Slide titles and content
- Bullet points and text boxes
- Speaker notes
- Slide count
- Metadata extraction

## Advanced Usage

### Convert to Markdown

```rust
use docling_opendocument::odt::{parse_odt_file, odt_to_markdown};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Method 1: Parse then access text field (already markdown)
    let doc = parse_odt_file("document.odt")?;
    let markdown = doc.text;

    // Method 2: Direct conversion
    let markdown = odt_to_markdown("document.odt")?;

    println!("{}", markdown);

    Ok(())
}
```

### Extract Metadata Only

```rust
use docling_opendocument::odt::parse_odt_file;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let doc = parse_odt_file("document.odt")?;

    println!("=== Document Metadata ===");
    if let Some(title) = doc.title {
        println!("Title: {}", title);
    }
    if let Some(author) = doc.author {
        println!("Author: {}", author);
    }
    if let Some(subject) = doc.subject {
        println!("Subject: {}", subject);
    }
    if let Some(keywords) = doc.keywords {
        println!("Keywords: {}", keywords);
    }
    if let Some(description) = doc.description {
        println!("Description: {}", description);
    }
    if let Some(page_count) = doc.page_count {
        println!("Pages: {}", page_count);
    }

    Ok(())
}
```

### Process Individual Sheets (ODS)

```rust
use docling_opendocument::ods::parse_ods_file;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let doc = parse_ods_file("spreadsheet.ods")?;

    // The text field contains all sheets formatted as Markdown
    // Sheets are separated by headers: "# Sheet Name"

    let sheets: Vec<&str> = doc.text.split("\n# ").collect();

    for (idx, sheet) in sheets.iter().enumerate() {
        println!("=== Sheet {} ===", idx);
        println!("{}", sheet);
        println!();
    }

    Ok(())
}
```

### Extract Table Data (ODT)

```rust
use docling_opendocument::odt::parse_odt_file;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let doc = parse_odt_file("document.odt")?;

    // Tables are formatted as Markdown tables in the text field
    // Example:
    // | Column 1 | Column 2 |
    // |----------|----------|
    // | Data 1   | Data 2   |

    for line in doc.text.lines() {
        if line.starts_with('|') {
            println!("Table row: {}", line);
        }
    }

    Ok(())
}
```

## Error Handling

```rust
use docling_opendocument::odt::parse_odt_file;
use docling_opendocument::error::OdfError;

fn main() {
    match parse_odt_file("document.odt") {
        Ok(doc) => {
            println!("Parsed successfully: {}", doc.text.len());
        }
        Err(e) => {
            eprintln!("Failed to parse ODT: {}", e);
        }
    }
}
```

## Performance

ODF parsing is optimized for speed and memory efficiency:

| Format | Typical Size | Parse Time | Memory Usage |
|--------|--------------|------------|--------------|
| ODT | 50 KB - 5 MB | 20-100 ms | 10-50 MB |
| ODS | 100 KB - 10 MB | 50-200 ms | 20-100 MB |
| ODP | 500 KB - 20 MB | 100-500 ms | 50-200 MB |

Benchmarked on Apple M1, 16GB RAM.

**Memory Efficiency:**
- ODF files are ZIP archives, extracted to memory
- XML parsing uses streaming SAX-style parser (low memory)
- Large documents (>50 MB) may require streaming API (future)

## Dependencies

- `zip` - ZIP archive extraction (ODF container)
- `quick-xml` - Fast XML parsing
- `calamine` - Spreadsheet parsing (ODS)
- `anyhow` - Error handling
- `thiserror` - Error type definitions

## Integration with docling-core

This crate is automatically used by `docling-core` when processing ODF files:

```rust
use docling_backend::{DocumentConverter, ConversionOptions};  // Note: DocumentConverter is in docling-backend crate
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let converter = DocumentConverter::new()?;

    // Automatically detects and parses ODT/ODS/ODP
    let doc = converter.convert(Path::new("document.odt"), ConversionOptions::default())?;

    println!("Title: {}", doc.metadata.get("title").unwrap_or("Untitled"));
    println!("Content: {}", doc.to_markdown());

    Ok(())
}
```

## Testing

Run tests:

```bash
cargo test -p docling-opendocument
```

Run with test files (requires test corpus):

```bash
# ODT tests
cargo test -p docling-opendocument test_odt

# ODS tests
cargo test -p docling-opendocument test_ods

# ODP tests
cargo test -p docling-opendocument test_odp
```

## Examples

See `examples/` directory for complete working examples:

- `examples/odt_parser.rs` - Basic ODT parsing
- `examples/ods_parser.rs` - Spreadsheet parsing
- `examples/odp_parser.rs` - Presentation parsing
- `examples/extract_metadata.rs` - Extract all metadata
- `examples/convert_to_markdown.rs` - Convert to Markdown

Run examples:

```bash
cargo run --example odt_parser -- document.odt
cargo run --example ods_parser -- spreadsheet.ods
cargo run --example odp_parser -- presentation.odp
```

## Format Specifications

### OpenDocument Format
- **Specification**: ISO/IEC 26300:2015 (ODF 1.2)
- **Website**: https://www.oasis-open.org/committees/tc_home.php?wg_abbrev=office
- **MIME Types**:
  - ODT: `application/vnd.oasis.opendocument.text`
  - ODS: `application/vnd.oasis.opendocument.spreadsheet`
  - ODP: `application/vnd.oasis.opendocument.presentation`
- **Structure**: ZIP archive containing XML files

### File Structure

```
document.odt
├── mimetype                 # MIME type declaration
├── META-INF/
│   └── manifest.xml         # File manifest
├── content.xml              # Document content (main)
├── styles.xml               # Style definitions
├── meta.xml                 # Document metadata
└── Pictures/                # Embedded images (if any)
    └── *.png, *.jpg
```

## Known Limitations

### ODT
- Advanced text formatting (columns, text boxes) converted to basic format
- Embedded images extracted as references only (base64 encoding not implemented)
- Complex tables with merged cells may lose formatting
- Comments and tracked changes are skipped
- Drawing objects (shapes, diagrams) are not parsed

### ODS
- Formula evaluation not implemented (formulas shown as text)
- Charts and graphs not extracted
- Conditional formatting not preserved
- Data validation rules not parsed
- Pivot tables not supported

### ODP
- Slide animations not parsed
- Slide transitions not extracted
- Embedded videos/audio not extracted
- Complex layouts simplified to linear text
- Master slides not applied

## Roadmap

- [ ] Embedded image extraction (base64-encoded)
- [ ] Drawing object support (shapes, diagrams, charts)
- [ ] Formula evaluation for ODS
- [ ] Chart extraction from ODS and ODP
- [ ] Comment and tracked change extraction (ODT)
- [ ] ODF 1.3 support (current: 1.2)
- [ ] Master slide application for ODP
- [ ] Better table cell merging for ODT/ODS

## License

MIT License - see LICENSE file for details

## Contributing

Contributions welcome! Please see the main docling-rs repository for contribution guidelines.

## Related Crates

- `docling-core` - Main document conversion library
- `docling-backend` - Backend orchestration for all formats
- `docling-cli` - Command-line interface
- `docling-ebook` - E-book format support
- `docling-email` - Email format support
- `docling-archive` - Archive format support

## References

- [OpenDocument Format Specification (ODF 1.2)](https://docs.oasis-open.org/office/v1.2/OpenDocument-v1.2.html)
- [OASIS OpenDocument TC](https://www.oasis-open.org/committees/tc_home.php?wg_abbrev=office)
- [LibreOffice ODF Import/Export](https://wiki.documentfoundation.org/Development/ODF_Implementer_Notes)
- [Apache OpenOffice ODF Guide](https://www.openoffice.org/xml/)

## Compatibility

This crate supports files created by:
- LibreOffice (4.x - 7.x)
- Apache OpenOffice (3.x - 4.x)
- Google Docs (exported as ODF)
- Microsoft Office 2010+ (ODF support)
- AbiWord
- Calligra Suite
- OnlyOffice

Tested with ODF 1.0, 1.1, and 1.2 formats.
