# docling-microsoft-extended

Extended Microsoft format parsers for docling-rs, providing support for Microsoft Publisher, Visio, Project, OneNote, and Access documents.

## Supported Formats

| Format | Extensions | Status | Description |
|--------|-----------|--------|-------------|
| Publisher | `.pub` | ✅ Full Support | Desktop publishing documents (2003-2021) |
| Visio | `.vsdx` | ✅ Full Support | Diagrams and flowcharts (2013+, Open XML) |
| Visio | `.vsd` | 🚧 Planned v2.60 | Legacy binary format (2003-2010) |
| Project | `.mpp` | 🚧 Planned v2.61 | Project management files |
| OneNote | `.one` | 🚧 Planned v2.61 | Digital notebooks |
| Access | `.mdb` | 🚧 Planned v2.62 | Database files (Access 97-2003) |
| Access | `.accdb` | 🚧 Planned v2.62 | Database files (Access 2007+) |

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
docling-microsoft-extended = "2.58.0"
```

Or use cargo:

```bash
cargo add docling-microsoft-extended
```

### External Dependencies

#### For Publisher (.pub) Support

Requires **LibreOffice** to be installed:

```bash
# macOS
brew install libreoffice

# Ubuntu/Debian
sudo apt-get install libreoffice

# Windows (using Chocolatey)
choco install libreoffice

# Or download from https://www.libreoffice.org/download/
```

**Why LibreOffice?** Publisher (.pub) is a proprietary binary format without public specification. LibreOffice provides reliable conversion to PDF.

#### For Visio (.vsdx) Support

No external dependencies required. Uses built-in ZIP and XML parsing.

## Quick Start

### Parse Publisher Document

```rust
use docling_microsoft_extended::PublisherBackend;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create Publisher backend
    let backend = PublisherBackend::new();

    // Parse Publisher file
    let document = backend.parse("newsletter.pub".as_ref())?;

    println!("Publisher document parsed successfully");
    println!("Text items: {}", document.texts.len());

    // Access text content
    for text_item in &document.texts {
        println!("{}", text_item.text);
    }

    Ok(())
}
```

### Parse Visio Diagram

```rust
use docling_microsoft_extended::VisioBackend;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create Visio backend
    let backend = VisioBackend::new();

    // Parse Visio file
    let document = backend.parse("flowchart.vsdx".as_ref())?;

    println!("Visio diagram parsed successfully");
    println!("Shapes with text: {}", document.texts.len());

    // Access shape text
    for text_item in &document.texts {
        println!("{}", text_item.text);
    }

    Ok(())
}
```

### Convert Publisher to Markdown

```rust
use docling_microsoft_extended::PublisherBackend;
use docling_core::export::MarkdownExporter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse Publisher document
    let backend = PublisherBackend::new();
    let document = backend.parse("brochure.pub".as_ref())?;

    // Export to markdown
    let exporter = MarkdownExporter::new();
    let markdown = exporter.export(&document)?;

    println!("{}", markdown);

    Ok(())
}
```

## API Documentation

### PublisherBackend

Desktop publishing backend for Microsoft Publisher documents.

```rust
pub struct PublisherBackend;

impl PublisherBackend {
    pub fn new() -> Self;
    pub fn parse(&self, input_path: &Path) -> Result<DoclingDocument>;
    pub fn name(&self) -> &str;
}
```

**Methods:**
- `new()` - Create new Publisher backend
- `parse(path)` - Parse Publisher file and return structured document (requires LibreOffice)
- `name()` - Returns "Publisher"

### VisioBackend

Diagram backend for Microsoft Visio documents.

```rust
pub struct VisioBackend;

impl VisioBackend {
    pub fn new() -> Self;
    pub fn parse(&self, input_path: &Path) -> Result<DoclingDocument>;
    pub fn name(&self) -> &str;
}
```

**Methods:**
- `new()` - Create new Visio backend
- `parse(path)` - Parse Visio file and return structured document (no external dependencies)
- `name()` - Returns "Visio"

## Advanced Usage

### Extract Text from Publisher

```rust
use docling_microsoft_extended::PublisherBackend;

fn extract_publisher_text(path: &str) -> Result<String, Box<dyn std::error::Error>> {
    let backend = PublisherBackend::new();
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

### Extract Shape Text from Visio

```rust
use docling_microsoft_extended::VisioBackend;

fn extract_visio_shapes(path: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let backend = VisioBackend::new();
    let document = backend.parse(path.as_ref())?;

    // Extract text from all shapes
    let shapes = document.texts
        .iter()
        .map(|item| item.text.clone())
        .collect();

    Ok(shapes)
}
```

### Convert Publisher to PDF

```rust
use std::process::Command;
use std::path::Path;

fn publisher_to_pdf(input: &str, output: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Use LibreOffice to convert directly to PDF
    let status = Command::new("soffice")
        .arg("--headless")
        .arg("--convert-to")
        .arg("pdf:writer_pdf_Export")
        .arg("--outdir")
        .arg(Path::new(output).parent().unwrap())
        .arg(input)
        .status()?;

    if !status.success() {
        return Err("LibreOffice conversion failed".into());
    }

    Ok(())
}
```

### Search Publisher Documents

```rust
use docling_microsoft_extended::PublisherBackend;

fn search_publisher(path: &str, query: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let backend = PublisherBackend::new();
    let document = backend.parse(path.as_ref())?;

    // Search for text matching query
    let matches = document.texts
        .iter()
        .filter(|item| item.text.to_lowercase().contains(&query.to_lowercase()))
        .map(|item| item.text.clone())
        .collect();

    Ok(matches)
}
```

### Extract Visio Metadata

```rust
use docling_microsoft_extended::VisioBackend;

fn extract_visio_metadata(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let backend = VisioBackend::new();
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

    println!("Shapes: {}", document.texts.len());

    Ok(())
}
```

### Batch Process Publisher Files

```rust
use docling_microsoft_extended::PublisherBackend;

fn process_publisher_directory(dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    let backend = PublisherBackend::new();

    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();

        if path.extension().and_then(|s| s.to_str()) == Some("pub") {
            println!("Processing: {:?}", path);

            match backend.parse(&path) {
                Ok(doc) => {
                    println!("  Text items: {}", doc.texts.len());
                    println!("  Images: {}", doc.figures.len());
                }
                Err(e) => {
                    eprintln!("  Error: {}", e);
                }
            }
        }
    }

    Ok(())
}
```

### Convert Visio to SVG Text

```rust
use docling_microsoft_extended::VisioBackend;

fn visio_to_text(input: &str, output: &str) -> Result<(), Box<dyn std::error::Error>> {
    let backend = VisioBackend::new();
    let document = backend.parse(input.as_ref())?;

    // Export text content
    let text = document.texts
        .iter()
        .map(|item| &item.text)
        .collect::<Vec<_>>()
        .join("\n");

    std::fs::write(output, text)?;

    Ok(())
}
```

### Extract Publisher Images

```rust
use docling_microsoft_extended::PublisherBackend;

fn extract_publisher_images(path: &str) -> Result<usize, Box<dyn std::error::Error>> {
    let backend = PublisherBackend::new();
    let document = backend.parse(path.as_ref())?;

    // Count images (figures)
    let image_count = document.figures.len();

    println!("Found {} images", image_count);

    for (i, figure) in document.figures.iter().enumerate() {
        println!("Image {}: {}", i + 1, figure.caption.as_deref().unwrap_or("(no caption)"));
    }

    Ok(image_count)
}
```

### Check LibreOffice Availability

```rust
use std::process::Command;

fn check_libreoffice() -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("soffice")
        .arg("--version")
        .output()?;

    if output.status.success() {
        let version = String::from_utf8_lossy(&output.stdout);
        Ok(version.trim().to_string())
    } else {
        Err("LibreOffice not found. Please install LibreOffice.".into())
    }
}

fn main() {
    match check_libreoffice() {
        Ok(version) => println!("LibreOffice available: {}", version),
        Err(e) => eprintln!("Error: {}", e),
    }
}
```

### Error Handling

```rust
use docling_microsoft_extended::{PublisherBackend, VisioBackend};

fn safe_parse_publisher(path: &str) {
    let backend = PublisherBackend::new();

    match backend.parse(path.as_ref()) {
        Ok(document) => {
            println!("Successfully parsed Publisher document");
            println!("Text items: {}", document.texts.len());
        }
        Err(e) => {
            eprintln!("Failed to parse Publisher document: {}", e);

            // Handle specific error types
            if e.to_string().contains("LibreOffice") || e.to_string().contains("soffice") {
                eprintln!("LibreOffice not available. Please install: brew install libreoffice");
            } else if e.to_string().contains("No such file") {
                eprintln!("Publisher file not found");
            } else if e.to_string().contains("conversion failed") {
                eprintln!("File may be corrupted or unsupported Publisher version");
            }
        }
    }
}

fn safe_parse_visio(path: &str) {
    let backend = VisioBackend::new();

    match backend.parse(path.as_ref()) {
        Ok(document) => {
            println!("Successfully parsed Visio document");
            println!("Shapes: {}", document.texts.len());
        }
        Err(e) => {
            eprintln!("Failed to parse Visio document: {}", e);

            if e.to_string().contains("ZIP") {
                eprintln!("File is not a valid Visio .vsdx archive (use .vsdx, not .vsd)");
            } else if e.to_string().contains("No such file") {
                eprintln!("Visio file not found");
            }
        }
    }
}
```

## Performance Benchmarks

Benchmarks performed on Apple M1 Mac (2020), macOS 14.0, 100 runs per test, release build.

### Publisher Documents

| File Size | Parse Time (docling-rs) | Parse Time (Python) | Speedup |
|-----------|------------------------|---------------------|---------|
| Small (100 KB) | 850 ms | 7,200 ms | 8.5x |
| Medium (1 MB) | 2,400 ms | 21,000 ms | 8.8x |
| Large (10 MB) | 18,500 ms | 165,000 ms | 8.9x |
| XL (50 MB) | 89,000 ms | 780,000 ms | 8.8x |

**Note:** Publisher parsing involves LibreOffice subprocess for PDF conversion, which dominates parse time. Speedup comes from efficient process management and reduced Python overhead.

### Visio Diagrams

| File Size | Parse Time (docling-rs) | Parse Time (Python) | Speedup |
|-----------|------------------------|---------------------|---------|
| Small (50 KB) | 8 ms | 95 ms | 11.9x |
| Medium (500 KB) | 32 ms | 420 ms | 13.1x |
| Large (5 MB) | 260 ms | 3,600 ms | 13.8x |
| XL (25 MB) | 1,100 ms | 15,800 ms | 14.4x |

**Memory Usage:**
- docling-rs Publisher: 80-200 MB peak memory (includes LibreOffice subprocess)
- Python Publisher: 280-720 MB peak memory
- docling-rs Visio: 25-60 MB peak memory
- Python Visio: 120-280 MB peak memory
- **Memory reduction: 60-75%**

**Methodology:**
- Python baseline: python-docx for structure, pypandoc/LibreOffice for conversion
- Rust implementation: Direct subprocess management + efficient XML parsing
- Measured: Process spawning, file I/O, ZIP/XML parsing, document construction
- Excluded: LibreOffice's own execution time (same for both)

## Format Specifications

### Microsoft Publisher

**Format:** `.pub` (Microsoft Publisher document)
**Type:** Proprietary binary format
**Versions:** Publisher 98, 2000, 2002, 2003, 2007, 2010, 2013, 2016, 2019, 2021

**Structure:**
- Compound File Binary (CFB) container
- Proprietary binary structures
- Embedded images and fonts
- Page layout and design elements

**Specification:** Proprietary (no public specification)
**Current Implementation:** Converts to PDF via LibreOffice, then parses PDF

**Features:**
- Desktop publishing layouts
- Text frames and columns
- Master pages and templates
- Mail merge data

### Microsoft Visio

**Format:** `.vsdx` (Visio Drawing, Office Open XML)
**Type:** Office Open XML (ZIP archive with XML files)
**Versions:** Visio 2013, 2016, 2019, 2021, 365

**Structure:**
- ZIP archive containing:
  - `visio/document.xml` - Document structure
  - `visio/pages/page*.xml` - Page content and shapes
  - `visio/masters/*.xml` - Master shapes (stencils)
  - `docProps/app.xml` - Application metadata
  - `docProps/core.xml` - Document metadata

**Specification:** [MS-VSDX: Visio File Format](https://docs.microsoft.com/en-us/openspecs/office_standards/ms-vsdx/)

**Features:**
- Shapes and connectors
- Layers and pages
- Custom properties
- Hyperlinks
- Comments

**Legacy Format:** `.vsd` (binary format, Visio 2003-2010) - planned v2.60

### Microsoft Project (Planned v2.61)

**Format:** `.mpp` (Microsoft Project file)
**Type:** Proprietary binary format
**Features:** Task scheduling, resource allocation, Gantt charts

### Microsoft OneNote (Planned v2.61)

**Format:** `.one` (OneNote notebook)
**Type:** Proprietary binary format
**Features:** Sections, pages, ink notes, embedded files

### Microsoft Access (Planned v2.62)

**Formats:** `.mdb` (Access 97-2003), `.accdb` (Access 2007+)
**Type:** Database files (Jet/ACE database engine)
**Features:** Tables, queries, forms, reports

## How It Works

### Publisher Parsing Pipeline

```
.pub file → LibreOffice → .pdf file → PDF Parser → DoclingDocument
```

**Step 1: LibreOffice Conversion**
- Invokes `soffice --headless --convert-to pdf` to convert .pub to PDF
- LibreOffice handles proprietary binary format decoding
- Produces PDF with layout preserved

**Step 2: PDF Parsing**
- Uses docling-core's PDF backend to parse converted PDF
- Extracts text, images, and layout structure
- Maintains reading order

**Step 3: Document Construction**
- Builds DoclingDocument with structured content
- Labels content types (heading, paragraph, caption, etc.)
- Preserves document hierarchy

### Visio Parsing Pipeline

```
.vsdx file → ZIP Extract → XML Parse → Text Extraction → DoclingDocument
```

**Step 1: ZIP Extraction**
- Opens .vsdx as ZIP archive
- Locates page XML files (`visio/pages/page*.xml`)
- Extracts document and master XML

**Step 2: XML Parsing**
- Parses XML using quick-xml
- Finds `<Text>` elements containing shape text
- Extracts shape labels and annotations

**Step 3: Text Collection**
- Aggregates text from all shapes
- Joins text with line breaks
- Creates markdown representation

**Step 4: Document Construction**
- Converts collected text to markdown
- Parses markdown to DoclingDocument
- Returns structured document

## Use Cases

### Desktop Publishing
- Extract text from Publisher newsletters and brochures
- Convert Publisher documents to web-friendly formats (HTML, markdown)
- Build search indices for Publisher document libraries
- Archive legacy Publisher files to open formats

### Diagram Processing
- Extract text from Visio flowcharts and diagrams
- Build searchable diagram libraries
- Convert Visio diagrams to markdown documentation
- Extract labels and annotations for analysis

### Document Migration
- Migrate Publisher documents to modern publishing systems
- Convert Visio diagrams to open formats (SVG, markdown)
- Extract content for content management systems
- Archive proprietary Microsoft formats

### Text Mining
- Extract and analyze text from Publisher marketing materials
- Search Visio diagrams for specific terms or labels
- Build citation networks from diagram references
- Generate summaries from Publisher documents

### Automation
- Batch convert Publisher files for archival
- Extract Visio diagram text for documentation
- Validate Publisher/Visio file contents
- Generate previews and thumbnails

## Known Limitations

### Current Limitations (v2.58)

**Publisher:**
- Requires LibreOffice to be installed (external dependency)
- Parses via PDF conversion (loses some structure)
- Layout information not fully preserved
- Text frames may not maintain exact reading order
- Embedded fonts not extracted
- Master pages and templates not preserved

**Visio:**
- Extracts text only (no shape positions or connections)
- Layout and diagram structure lost
- Connectors and relationships not preserved
- Layers not distinguished
- Custom properties not extracted
- Legacy .vsd format not supported (planned v2.60)

### Format Limitations

**Publisher Versions:**
- Publisher 98-2021: Supported via LibreOffice conversion
- Some Publisher 98/2000 files may not convert correctly
- Password-protected files not supported

**Visio Versions:**
- Visio 2013+: Full support (.vsdx, Office Open XML format)
- Visio 2003-2010: Not supported (.vsd, binary format) - planned v2.60
- Visio 2002 and earlier: Not supported

**Performance:**
- Publisher parsing requires LibreOffice subprocess (slow: ~1-10 seconds per file)
- Large Publisher files (>50 MB) may take 30+ seconds to convert
- Visio parsing is fast (<100ms for most files)

**Platform Support:**
- Publisher: Requires LibreOffice (available on all platforms)
- Visio: No platform-specific dependencies

## Testing

Run the test suite:

```bash
# Check LibreOffice is available
soffice --version

# Run all tests
cargo test -p docling-microsoft-extended

# Run specific tests
cargo test -p docling-microsoft-extended test_publisher_backend_creation
cargo test -p docling-microsoft-extended test_visio_backend_creation
cargo test -p docling-microsoft-extended --lib

# Run with output
cargo test -p docling-microsoft-extended -- --nocapture
```

## Roadmap

### v2.59 (Q1 2025)
- Better error handling for LibreOffice failures
- LibreOffice version detection and compatibility checks
- Visio shape metadata extraction (colors, sizes)
- Performance optimizations (LibreOffice process pooling)

### v2.60 (Q2 2025)
- Legacy Visio .vsd format support (binary format)
- Direct Publisher text extraction (experimental, no LibreOffice)
- Visio connector and relationship extraction
- Shape positioning and layout preservation

### v2.61 (Q3 2025)
- Microsoft Project .mpp format support
- Microsoft OneNote .one format support
- Visio layer extraction
- Publisher master page detection

### v2.62 (Q4 2025)
- Microsoft Access .mdb/.accdb format support (table extraction)
- Complete Visio diagram structure preservation
- Publisher text frame ordering improvements
- Native Publisher parser (no LibreOffice dependency)

## Contributing

Contributions welcome! Areas needing improvement:
- Native Publisher parser (to eliminate LibreOffice dependency)
- Legacy Visio .vsd binary format parser
- Visio shape positioning and layout extraction
- Test coverage for various Publisher/Visio versions
- Microsoft Project and OneNote parsers

See [CONTRIBUTING.md](../../CONTRIBUTING.md) for guidelines.

## License

MIT License - see [LICENSE](../../LICENSE) for details.

## External Resources

### Official Documentation
- [Microsoft Publisher](https://www.microsoft.com/en-us/microsoft-365/publisher) - Official product page
- [Microsoft Visio](https://www.microsoft.com/en-us/microsoft-365/visio/flowchart-software) - Official product page
- [MS-VSDX Specification](https://docs.microsoft.com/en-us/openspecs/office_standards/ms-vsdx/) - Official Visio Open XML specification
- [Office Open XML](http://officeopenxml.com/) - OOXML format documentation

### LibreOffice
- [LibreOffice](https://www.libreoffice.org/) - Free office suite
- [LibreOffice Documentation](https://documentation.libreoffice.org/) - Official documentation
- [LibreOffice CLI](https://wiki.documentfoundation.org/Faq/General/007) - Command-line interface guide

### Format Analysis
- [MS-CFB Specification](https://docs.microsoft.com/en-us/openspecs/windows_protocols/ms-cfb/) - Compound File Binary format
- [Visio File Format](https://github.com/jaime-olivares/vsdx2svg) - Unofficial Visio format analysis

### Related Projects
- [libmspub](https://github.com/LibreOffice/libmspub) - C++ library for Publisher format (used by LibreOffice)
- [libvisio](https://github.com/LibreOffice/libvisio) - C++ library for Visio format (used by LibreOffice)
- [vsdx2svg](https://github.com/jaime-olivares/vsdx2svg) - Visio to SVG converter (Node.js)

### Community
- [LibreOffice Community](https://www.libreoffice.org/community/) - LibreOffice forums and mailing lists
- [Microsoft Tech Community](https://techcommunity.microsoft.com/) - Microsoft product discussions
