# docling-xps

XPS (XML Paper Specification) parser for docling-rs, supporting Microsoft's XML-based fixed-layout document format.

## Supported Formats

| Format | Extensions | Status | Description |
|--------|-----------|--------|-------------|
| XPS | `.xps` | ✅ Full Support | XML Paper Specification (Microsoft) |
| OXPS | `.oxps` | ✅ Full Support | Open XPS (ECMA-388 standard) |

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
docling-xps = "2.58.0"
```

Or use cargo:

```bash
cargo add docling-xps
```

## Quick Start

### Parse XPS Document

```rust
use docling_xps::{parse_xps, XpsDocument};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let doc: XpsDocument = parse_xps(Path::new("document.xps"))?;

    println!("Title: {:?}", doc.metadata.title);
    println!("Author: {:?}", doc.metadata.author);
    println!("Pages: {}", doc.pages.len());

    // Extract text from all pages
    for (idx, page) in doc.pages.iter().enumerate() {
        println!("\n--- Page {} ---", idx + 1);
        println!("{}", page.text());
    }

    Ok(())
}
```

### Extract Text from Specific Page

```rust
use docling_xps::parse_xps;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let doc = parse_xps(Path::new("document.xps"))?;

    if let Some(first_page) = doc.pages.first() {
        let text = first_page.text();
        println!("First page text:\n{}", text);
    }

    Ok(())
}
```

### Extract Metadata

```rust
use docling_xps::parse_xps;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let doc = parse_xps(Path::new("document.xps"))?;
    let meta = &doc.metadata;

    println!("Title: {:?}", meta.title);
    println!("Author: {:?}", meta.author);
    println!("Subject: {:?}", meta.subject);
    println!("Keywords: {:?}", meta.keywords);
    println!("Description: {:?}", meta.description);
    println!("Created: {:?}", meta.created);
    println!("Modified: {:?}", meta.modified);

    Ok(())
}
```

## Data Structures

### XpsDocument

```rust
pub struct XpsDocument {
    pub metadata: XpsMetadata,
    pub pages: Vec<XpsPage>,
}
```

### XpsMetadata

```rust
pub struct XpsMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub subject: Option<String>,
    pub keywords: Option<String>,
    pub description: Option<String>,
    pub created: Option<String>,
    pub modified: Option<String>,
    pub version: Option<String>,
}
```

### XpsPage

```rust
pub struct XpsPage {
    pub width: f64,
    pub height: f64,
    pub elements: Vec<XpsTextElement>,
}

impl XpsPage {
    pub fn text(&self) -> String {
        // Returns page text as plain string
    }
}
```

### XpsTextElement

```rust
pub struct XpsTextElement {
    pub text: String,
    pub x: f64,
    pub y: f64,
    pub font_size: f64,
    pub font_family: Option<String>,
}
```

## Features

### XPS Support
- XML-based document parsing
- Multi-page document handling
- Text extraction with positioning
- Metadata extraction (Dublin Core)
- Embedded fonts detection
- Image references extraction
- Fixed-layout page dimensions

### Open XPS (OXPS) Support
- ECMA-388 compliant parsing
- Same features as XPS
- Namespace handling differences

## Advanced Usage

### Convert to Markdown

```rust
use docling_xps::parse_xps;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let doc = parse_xps(Path::new("document.xps"))?;

    // Generate Markdown with page breaks
    let mut markdown = String::new();

    if let Some(title) = &doc.metadata.title {
        markdown.push_str(&format!("# {}\n\n", title));
    }

    if let Some(author) = &doc.metadata.author {
        markdown.push_str(&format!("**Author:** {}\n\n", author));
    }

    for (idx, page) in doc.pages.iter().enumerate() {
        markdown.push_str(&format!("## Page {}\n\n", idx + 1));
        markdown.push_str(&page.text());
        markdown.push_str("\n\n---\n\n");
    }

    println!("{}", markdown);

    Ok(())
}
```

### Extract Text with Positioning

```rust
use docling_xps::parse_xps;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let doc = parse_xps(Path::new("document.xps"))?;

    for (page_idx, page) in doc.pages.iter().enumerate() {
        println!("Page {} ({}x{})", page_idx + 1, page.width, page.height);

        for element in &page.elements {
            println!(
                "  Text at ({:.1}, {:.1}): '{}' (font: {:?}, size: {:.1})",
                element.x,
                element.y,
                element.text,
                element.font_family,
                element.font_size
            );
        }
    }

    Ok(())
}
```

### Filter by Font Size

```rust
use docling_xps::parse_xps;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let doc = parse_xps(Path::new("document.xps"))?;

    // Extract only large text (likely headings)
    for page in &doc.pages {
        let headings: Vec<_> = page
            .elements
            .iter()
            .filter(|elem| elem.font_size > 16.0)
            .collect();

        for heading in headings {
            println!("Heading ({}pt): {}", heading.font_size, heading.text);
        }
    }

    Ok(())
}
```

### Calculate Reading Order

```rust
use docling_xps::parse_xps;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let doc = parse_xps(Path::new("document.xps"))?;

    for page in &doc.pages {
        // Sort elements by Y position (top to bottom), then X position (left to right)
        let mut sorted_elements = page.elements.clone();
        sorted_elements.sort_by(|a, b| {
            a.y.partial_cmp(&b.y)
                .unwrap()
                .then(a.x.partial_cmp(&b.x).unwrap())
        });

        println!("=== Reading Order ===");
        for element in sorted_elements {
            println!("{}", element.text);
        }
    }

    Ok(())
}
```

## Error Handling

```rust
use docling_xps::{parse_xps, XpsError};
use std::path::Path;

fn main() {
    match parse_xps(Path::new("document.xps")) {
        Ok(doc) => {
            println!("Parsed {} pages", doc.pages.len());
        }
        Err(XpsError::Io(e)) => {
            eprintln!("IO error: {}", e);
        }
        Err(XpsError::Xml(e)) => {
            eprintln!("XML parsing error: {}", e);
        }
        Err(XpsError::InvalidFormat(msg)) => {
            eprintln!("Invalid XPS format: {}", msg);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }
}
```

## Performance

XPS parsing is optimized for speed and memory efficiency:

| Document Size | Pages | Parse Time | Memory Usage |
|---------------|-------|------------|--------------|
| 100 KB | 1-5 | 20-50 ms | 5-15 MB |
| 1 MB | 10-50 | 100-300 ms | 20-60 MB |
| 10 MB | 100-500 | 1-3 seconds | 50-200 MB |

Benchmarked on Apple M1, 16GB RAM.

**Memory Efficiency:**
- XPS files are ZIP archives, extracted to memory
- XML parsing uses streaming parser (low memory)
- Large documents (>50 MB) may require streaming API (future)

## Dependencies

- `zip` - ZIP archive extraction (XPS container)
- `quick-xml` - Fast XML parsing
- `anyhow` - Error handling
- `thiserror` - Error type definitions
- `log` - Logging

## Integration with docling-core

This crate is automatically used by `docling-core` when processing XPS files:

```rust
use docling_backend::{DocumentConverter, ConversionOptions};  // Note: DocumentConverter is in docling-backend crate
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let converter = DocumentConverter::new()?;

    // Automatically detects and parses XPS/OXPS
    let doc = converter.convert(Path::new("document.xps"), ConversionOptions::default())?;

    println!("Title: {}", doc.metadata.get("title").unwrap_or("Untitled"));
    println!("Pages: {}", doc.items.len());

    Ok(())
}
```

## Testing

Run tests:

```bash
cargo test -p docling-xps
```

Run with test files (requires test corpus):

```bash
# XPS tests
cargo test -p docling-xps test_xps

# OXPS tests
cargo test -p docling-xps test_oxps
```

## Examples

See `examples/` directory for complete working examples:

- `examples/xps_parser.rs` - Basic XPS parsing
- `examples/extract_metadata.rs` - Extract all metadata
- `examples/extract_text.rs` - Extract text with positioning
- `examples/convert_to_markdown.rs` - Convert to Markdown

Run examples:

```bash
cargo run --example xps_parser -- document.xps
cargo run --example extract_metadata -- document.xps
```

## Format Specifications

### XPS (XML Paper Specification)
- **Specification**: Microsoft XPS Specification 1.0
- **Website**: https://www.microsoft.com/whdc/xps/default.mspx
- **MIME Type**: `application/vnd.ms-xpsdocument`
- **Structure**: ZIP archive containing XML pages and resources

### Open XPS (OXPS)
- **Specification**: ECMA-388 (Open XML Paper Specification)
- **Website**: https://www.ecma-international.org/publications/standards/Ecma-388.htm
- **MIME Type**: `application/oxps`
- **Structure**: ZIP archive with ECMA-388 compliant XML

### File Structure

```
document.xps
├── _rels/
│   └── .rels                # Package relationships
├── docProps/
│   ├── core.xml             # Core metadata (Dublin Core)
│   └── app.xml              # Application-specific metadata
├── Documents/
│   ├── 1/
│   │   ├── Pages/
│   │   │   ├── 1.fpage      # Page 1 content (XAML/XML)
│   │   │   └── 2.fpage      # Page 2 content
│   │   ├── Resources/
│   │   │   ├── Fonts/       # Embedded fonts
│   │   │   └── Images/      # Embedded images
│   │   └── FixedDocument.fdoc  # Document structure
└── [Content_Types].xml      # Content type definitions
```

## Known Limitations

### Text Extraction
- Text elements extracted in XML order (may not match visual reading order)
- No automatic paragraph detection (whitespace-based heuristics)
- Rotated text not handled
- Text in images not extracted (no OCR)

### Formatting
- Font styles (bold, italic) not detected
- Text colors not extracted
- Background colors not extracted
- Hyperlinks not parsed

### Graphics
- Vector graphics (paths, shapes) not converted to text
- Embedded images extracted as references only (binary data not exported)
- Image positions available but not rendered

### Encryption
- Password-protected XPS files not supported
- Digital signatures not verified

## Roadmap

- [ ] Automatic reading order detection (top-to-bottom, left-to-right)
- [ ] Paragraph and heading detection
- [ ] Font style preservation (bold, italic)
- [ ] Hyperlink extraction
- [ ] Table detection and extraction
- [ ] Image binary data export
- [ ] Password-protected XPS support
- [ ] Digital signature verification

## License

MIT License - see LICENSE file for details

## Contributing

Contributions welcome! Please see the main docling-rs repository for contribution guidelines.

## Related Crates

- `docling-core` - Main document conversion library
- `docling-backend` - Backend orchestration for all formats
- `docling-cli` - Command-line interface
- `docling-svg` - SVG graphics support
- `docling-ebook` - E-book format support

## References

- [Microsoft XPS Specification](https://www.ecma-international.org/publications-and-standards/standards/ecma-388/)
- [ECMA-388: Open XML Paper Specification](https://www.ecma-international.org/publications-and-standards/standards/ecma-388/)
- [XPS Document Structure](https://docs.microsoft.com/en-us/windows/win32/printdocs/xps-document-structure)

## Compatibility

This crate supports XPS files created by:
- Microsoft XPS Document Writer (Windows 7+)
- Microsoft Edge (Print to XPS)
- Microsoft Office (2007+, Save as XPS)
- LibreOffice (Export as XPS extension)
- Google Chrome (Print to XPS, Windows)

Tested with XPS 1.0 and Open XPS (ECMA-388) formats.
