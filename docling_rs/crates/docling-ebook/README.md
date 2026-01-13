# docling-ebook

E-book format parsers for docling-rs, providing high-performance document extraction from popular e-book formats.

## Supported Formats

| Format | Extensions | Status | Description |
|--------|-----------|--------|-------------|
| EPUB | `.epub` | ✅ Full Support | Electronic Publication (EPUB 2.x/3.x) |
| FB2 | `.fb2`, `.fb2.zip` | ✅ Full Support | FictionBook 2.x XML format |
| MOBI | `.mobi`, `.prc`, `.azw` | ✅ Full Support | Mobipocket and Amazon Kindle (pre-KF8) |
| AZW3 | `.azw3` | 🚧 Planned | Amazon Kindle KF8 format |

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
docling-ebook = "2.58.0"
```

Or use cargo:

```bash
cargo add docling-ebook
```

## Quick Start

### Parse EPUB

```rust
use docling_ebook::{parse_epub, EbookMetadata, ParsedEbook};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new("book.epub");
    let ebook: ParsedEbook = parse_epub(path)?;

    println!("Title: {}", ebook.metadata.title.unwrap_or_default());
    println!("Author: {}", ebook.metadata.author.unwrap_or_default());
    println!("Chapters: {}", ebook.chapters.len());

    for chapter in ebook.chapters {
        println!("\n## {}", chapter.title);
        println!("{}", chapter.text);
    }

    Ok(())
}
```

### Parse FictionBook (FB2)

```rust
use docling_ebook::{parse_fb2, ParsedEbook};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new("book.fb2");
    let ebook: ParsedEbook = parse_fb2(path)?;

    println!("Title: {}", ebook.metadata.title.unwrap_or_default());

    for chapter in ebook.chapters {
        println!("\n## {}", chapter.title);
        println!("{}", chapter.text);
    }

    Ok(())
}
```

### Parse MOBI

```rust
use docling_ebook::{parse_mobi, ParsedEbook};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new("book.mobi");
    let ebook: ParsedEbook = parse_mobi(path)?;

    println!("Title: {}", ebook.metadata.title.unwrap_or_default());
    println!("Publisher: {}", ebook.metadata.publisher.unwrap_or_default());

    for chapter in ebook.chapters {
        println!("\n## {}", chapter.title);
        println!("{}", chapter.text);
    }

    Ok(())
}
```

## Data Structures

### ParsedEbook

```rust
pub struct ParsedEbook {
    pub metadata: EbookMetadata,
    pub chapters: Vec<Chapter>,
    pub toc: Vec<TocEntry>,
}
```

### EbookMetadata

```rust
pub struct EbookMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub publisher: Option<String>,
    pub date: Option<String>,
    pub language: Option<String>,
    pub identifier: Option<String>,
    pub description: Option<String>,
    pub subjects: Vec<String>,
}
```

### Chapter

```rust
pub struct Chapter {
    pub title: String,
    pub text: String,
    pub images: Vec<(String, Vec<u8>)>,
}
```

### TocEntry

```rust
pub struct TocEntry {
    pub title: String,
    pub src: String,
    pub children: Vec<TocEntry>,
}
```

## Features

### EPUB Support
- EPUB 2.x and 3.x parsing
- Table of contents (NCX and Navigation Document)
- Metadata extraction (Dublin Core)
- HTML to plain text conversion
- Image extraction (embedded images as base64)
- CSS stripping
- Reflowable and fixed-layout

### FB2 Support
- FictionBook 2.x XML parsing
- Compressed FB2 (.fb2.zip)
- Metadata extraction (title-info, document-info)
- Section hierarchy (chapters)
- Binary images (embedded, base64-encoded)
- Text styling preservation

### MOBI Support
- Mobipocket format
- Amazon Kindle pre-KF8 (.azw)
- PalmDOC (.prc)
- Metadata extraction (EXTH records)
- HTML content extraction
- Image extraction

## Advanced Usage

### Extract All Metadata

```rust
use docling_ebook::parse_epub;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ebook = parse_epub(Path::new("book.epub"))?;
    let meta = &ebook.metadata;

    println!("Title: {:?}", meta.title);
    println!("Author: {:?}", meta.author);
    println!("Publisher: {:?}", meta.publisher);
    println!("Date: {:?}", meta.date);
    println!("Language: {:?}", meta.language);
    println!("Identifier: {:?}", meta.identifier);
    println!("Description: {:?}", meta.description);
    println!("Subjects: {:?}", meta.subjects);

    Ok(())
}
```

### Extract Table of Contents

```rust
use docling_ebook::{parse_epub, TocEntry};
use std::path::Path;

fn print_toc(entries: &[TocEntry], indent: usize) {
    for entry in entries {
        println!("{}{} -> {}", "  ".repeat(indent), entry.title, entry.src);
        print_toc(&entry.children, indent + 1);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ebook = parse_epub(Path::new("book.epub"))?;
    print_toc(&ebook.toc, 0);
    Ok(())
}
```

### Extract Images

```rust
use docling_ebook::parse_epub;
use std::path::Path;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ebook = parse_epub(Path::new("book.epub"))?;

    for (chapter_idx, chapter) in ebook.chapters.iter().enumerate() {
        for (image_idx, (name, data)) in chapter.images.iter().enumerate() {
            let filename = format!("chapter_{}_image_{}.jpg", chapter_idx, image_idx);
            fs::write(&filename, data)?;
            println!("Extracted: {} ({})", filename, name);
        }
    }

    Ok(())
}
```

### Convert HTML to Plain Text

```rust
use docling_ebook::html_to_text;

fn main() {
    let html = r#"<h1>Chapter 1</h1><p>This is <b>bold</b> text.</p>"#;
    let text = html_to_text(html);
    println!("{}", text);
    // Output:
    // # Chapter 1
    // This is **bold** text.
}
```

## Error Handling

All parsing functions return `Result<ParsedEbook, EbookError>`:

```rust
use docling_ebook::{parse_epub, EbookError};
use std::path::Path;

fn main() {
    match parse_epub(Path::new("book.epub")) {
        Ok(ebook) => {
            println!("Successfully parsed: {}", ebook.metadata.title.unwrap_or_default());
        }
        Err(EbookError::Io(e)) => {
            eprintln!("IO error: {}", e);
        }
        Err(EbookError::Epub(e)) => {
            eprintln!("EPUB parsing error: {}", e);
        }
        Err(EbookError::Xml(e)) => {
            eprintln!("XML parsing error: {}", e);
        }
        Err(EbookError::InvalidFormat(msg)) => {
            eprintln!("Invalid format: {}", msg);
        }
        Err(e) => {
            eprintln!("Other error: {}", e);
        }
    }
}
```

## Performance

E-book parsing is optimized for speed and memory efficiency:

| Format | Typical File Size | Parse Time | Memory Usage |
|--------|------------------|------------|--------------|
| EPUB | 500 KB - 5 MB | 50-200 ms | 10-50 MB |
| FB2 | 200 KB - 2 MB | 20-80 ms | 5-20 MB |
| MOBI | 1 MB - 10 MB | 100-500 ms | 20-100 MB |

Benchmarked on Apple M1, 16GB RAM.

## Dependencies

- `epub` - EPUB parsing
- `mobi` - MOBI parsing
- `quick-xml` - XML parsing for FB2
- `scraper` - HTML parsing
- `html2text` - HTML to plain text conversion
- `html2md` - HTML to Markdown conversion
- `zip` - ZIP archive handling (EPUB, compressed FB2)
- `base64` - Image encoding/decoding

## Integration with docling-core

This crate is automatically used by `docling-core` when processing e-book files:

```rust
use docling_backend::{DocumentConverter, ConversionOptions};  // Note: DocumentConverter is in docling-backend crate
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let converter = DocumentConverter::new()?;
    let doc = converter.convert(Path::new("book.epub"), ConversionOptions::default())?;

    // Access structured content
    println!("Title: {}", doc.metadata.get("title").unwrap_or("Untitled"));
    println!("Chapters: {}", doc.items.len());

    Ok(())
}
```

## Testing

Run tests:

```bash
cargo test -p docling-ebook
```

Run with test files (requires test corpus):

```bash
# EPUB tests
cargo test -p docling-ebook test_epub

# FB2 tests
cargo test -p docling-ebook test_fb2

# MOBI tests
cargo test -p docling-ebook test_mobi
```

## Examples

See `examples/` directory for complete working examples:

- `examples/epub_parser.rs` - Basic EPUB parsing
- `examples/fb2_parser.rs` - FictionBook parsing
- `examples/mobi_parser.rs` - MOBI parsing
- `examples/extract_metadata.rs` - Extract all metadata
- `examples/extract_toc.rs` - Extract table of contents
- `examples/extract_images.rs` - Extract embedded images

Run examples:

```bash
cargo run --example epub_parser -- book.epub
cargo run --example fb2_parser -- book.fb2
cargo run --example mobi_parser -- book.mobi
```

## Format Specifications

### EPUB
- **Specification**: IDPF EPUB 2.x/3.x
- **Website**: http://idpf.org/epub
- **MIME Type**: `application/epub+zip`
- **Structure**: ZIP container with XHTML/HTML5 content

### FictionBook (FB2)
- **Specification**: FictionBook 2.x
- **Website**: http://fictionbook.org
- **MIME Type**: `text/xml` (application/x-fictionbook+xml)
- **Structure**: XML with embedded base64 images

### MOBI
- **Specification**: Mobipocket (proprietary, reverse-engineered)
- **Website**: N/A (format deprecated by Amazon)
- **MIME Type**: `application/x-mobipocket-ebook`
- **Structure**: PalmDOC with MOBI header and EXTH metadata

## Known Limitations

### EPUB
- Fixed-layout EPUB (FXL) support is basic (no advanced layout)
- DRM-protected EPUB files are not supported
- Scripted EPUB (EPUB3 with JavaScript) runs script-free

### FB2
- FB2.1 draft features not supported
- External binary references not resolved

### MOBI
- KF8 format (AZW3) not yet supported (use EPUB conversion)
- DRM-encrypted MOBI files are not supported
- Topaz format not supported

## Roadmap

- [ ] AZW3 (Kindle KF8) support
- [ ] DRM detection and warning messages
- [ ] Enhanced table extraction from e-books
- [ ] Comic book format support (CBZ, CBR)
- [ ] Annotation extraction (EPUB3, iBooks)
- [ ] Better fixed-layout EPUB handling

## License

MIT License - see LICENSE file for details

## Contributing

Contributions welcome! Please see the main docling-rs repository for contribution guidelines.

## Related Crates

- `docling-core` - Main document conversion library
- `docling-backend` - Backend orchestration for all formats
- `docling-cli` - Command-line interface
- `docling-archive` - Archive format support (ZIP, TAR, etc.)
- `docling-email` - Email format support (EML, MSG, MBOX)

## References

- [EPUB Specifications](http://idpf.org/epub)
- [FictionBook Specification](http://fictionbook.org/index.php/Eng:XML_Schema_Fictionbook_2.1)
- [MOBI Format Analysis](https://wiki.mobileread.com/wiki/MOBI)
- [Calibre E-book Management](https://calibre-ebook.com/)
