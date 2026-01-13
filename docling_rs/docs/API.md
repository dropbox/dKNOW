# API Reference

Complete API documentation for docling-rs library.

---

## Table of Contents

1. [Core Types](#core-types)
2. [DocumentConverter](#documentconverter)
3. [Document](#document)
4. [InputFormat](#inputformat)
5. [Error Handling](#error-handling)
6. [Advanced Types](#advanced-types)

---

## Core Types

### DocumentConverter

The main entry point for document conversion.

```rust
pub struct DocumentConverter {
    // Internal Rust converter with format backends
}
```

**Constructors:**

```rust
impl DocumentConverter {
    /// Create new converter with default settings (OCR disabled)
    pub fn new() -> Result<Self>

    /// Create converter with specific OCR configuration
    pub fn with_ocr(enable_ocr: bool) -> Result<Self>
}
```

**Methods:**

```rust
impl DocumentConverter {
    /// Convert document to markdown
    pub fn convert<P: AsRef<Path>>(&self, path: P) -> Result<ConversionResult>
}
```

**Examples:**

```rust
// Note: DocumentConverter is in docling-backend crate
use docling_backend::DocumentConverter;

// Basic usage
let converter = DocumentConverter::new()?;
let result = converter.convert("document.pdf")?;
println!("{}", result.document.markdown);

// With OCR enabled
let converter = DocumentConverter::with_ocr(true)?;
let result = converter.convert("scanned.pdf")?;
```

---

### ConversionResult

Result of a document conversion operation.

```rust
pub struct ConversionResult {
    /// The converted document
    pub document: Document,

    /// Time taken for conversion
    pub latency: Duration,
}
```

**Fields:**

- `document: Document` - Converted document with markdown and metadata
- `latency: Duration` - Conversion time (from `std::time::Duration`)

**Example:**

```rust
let result = converter.convert("doc.pdf")?;
println!("Converted in {:?}", result.latency);
println!("Characters: {}", result.document.metadata.num_characters);
```

---

### Document

Represents a converted document.

```rust
pub struct Document {
    /// Markdown representation
    pub markdown: String,

    /// Input format
    pub format: InputFormat,

    /// Document metadata
    pub metadata: DocumentMetadata,

    /// Structured content blocks (optional)
    pub content_blocks: Option<Vec<ContentBlock>>,
}
```

**Constructors:**

```rust
impl Document {
    /// Create document from markdown string
    pub fn from_markdown(markdown: String, format: InputFormat) -> Self
}
```

**Methods:**

```rust
impl Document {
    /// Get markdown representation
    pub fn to_markdown(&self) -> &str

    /// Check if document has structured content
    pub fn has_structured_content(&self) -> bool

    /// Get structured content blocks
    pub fn blocks(&self) -> Option<&[ContentBlock]>
}
```

**Examples:**

```rust
// Access markdown output
let markdown = doc.to_markdown();
std::fs::write("output.md", markdown)?;

// Check for structured content
if doc.has_structured_content() {
    for block in doc.blocks().unwrap() {
        println!("Block: {:?}", block);
    }
}

// Access format
match doc.format {
    InputFormat::Pdf => println!("PDF document"),
    InputFormat::Docx => println!("Word document"),
    _ => println!("Other format"),
}
```

---

### DocumentMetadata

Metadata about the document.

```rust
pub struct DocumentMetadata {
    /// Number of pages (if applicable)
    pub num_pages: Option<usize>,

    /// Total character count
    pub num_characters: usize,

    /// Document title
    pub title: Option<String>,

    /// Author(s)
    pub author: Option<String>,

    /// Creation date
    pub created: Option<chrono::DateTime<chrono::Utc>>,

    /// Last modified date
    pub modified: Option<chrono::DateTime<chrono::Utc>>,

    /// Language (ISO 639-1 code)
    pub language: Option<String>,
}
```

**Examples:**

```rust
let meta = &result.document.metadata;

// Always available
println!("Characters: {}", meta.num_characters);

// Optional fields
if let Some(pages) = meta.num_pages {
    println!("Pages: {}", pages);
}

if let Some(title) = &meta.title {
    println!("Title: {}", title);
}

if let Some(created) = &meta.created {
    println!("Created: {}", created.format("%Y-%m-%d"));
}
```

**Note:** Most metadata fields are populated when available from the source format:
- `num_characters` (always set)
- `num_pages` (PDF and multi-page formats)

---

## InputFormat

Enum representing supported document formats.

```rust
pub enum InputFormat {
    // Documents
    Pdf, Docx, Doc, Pptx, Xlsx, Html, Csv, Md, Asciidoc, Jats, Rtf,

    // Images
    Png, Jpeg, Tiff, Webp, Bmp, Gif, Heif, Avif,

    // E-books
    Epub, Fb2, Mobi,

    // Email
    Eml, Mbox, Vcf, Msg,

    // Archives
    Zip, Tar, SevenZ, Rar,

    // Multimedia
    Wav, Mp3, Mp4, Mkv, Mov, Avi, Srt, Webvtt,

    // OpenDocument
    Odt, Ods, Odp,

    // Specialty
    Xps, Svg, Ics, Ipynb, Gpx, Kml, Kmz, Dicom,

    // CAD/3D
    Stl, Obj, Gltf, Glb, Dxf,

    // Adobe
    Idml,
}
```

**Methods:**

```rust
impl InputFormat {
    /// Detect format from file extension
    pub fn from_extension(ext: &str) -> Option<Self>

    /// Get file extensions for this format
    pub fn extensions(&self) -> &[&str]

    /// Check if this is an image format
    pub fn is_image(&self) -> bool

    /// Check if this is a document format
    pub fn is_document(&self) -> bool

    /// Check if this is an e-book format
    pub fn is_ebook(&self) -> bool

    /// Check if this is an email format
    pub fn is_email(&self) -> bool

    /// Check if this is an archive format
    pub fn is_archive(&self) -> bool

    /// Check if this is an audio format
    pub fn is_audio(&self) -> bool

    /// Check if this is a video format
    pub fn is_video(&self) -> bool

    /// Check if this is a subtitle format
    pub fn is_subtitle(&self) -> bool

    /// Check if this is an OpenDocument format
    pub fn is_opendocument(&self) -> bool

    /// Check if this is a CAD/3D format
    pub fn is_cad(&self) -> bool
}
```

**Display:**

```rust
impl std::fmt::Display for InputFormat
```

**Serialization:**

```rust
impl serde::Serialize for InputFormat
impl serde::Deserialize for InputFormat
```

**Examples:**

```rust
use docling_core::InputFormat;
use std::path::Path;

// Detect format from extension
let path = Path::new("document.pdf");
let ext = path.extension().unwrap().to_str().unwrap();
let format = InputFormat::from_extension(ext);
assert_eq!(format, Some(InputFormat::Pdf));

// Check format category
if format.unwrap().is_document() {
    println!("This is a document format");
}

// Get extensions for a format
let extensions = InputFormat::Pdf.extensions();
assert_eq!(extensions, &["pdf"]);

// Display format
println!("Format: {}", InputFormat::Pdf); // Prints: "PDF"

// Serialize to JSON
let json = serde_json::to_string(&InputFormat::Pdf)?;
assert_eq!(json, r#""PDF""#);
```

---

## Error Handling

### DoclingError

Main error type for docling-rs operations.

```rust
pub enum DoclingError {
    /// Format not supported or unrecognized
    FormatError(String),

    /// General conversion error
    ConversionError(String),

    /// Backend-specific error
    BackendError(String),

    /// I/O error
    IOError(#[from] std::io::Error),
}
```

**Traits:**

- `impl std::fmt::Display for DoclingError`
- `impl std::error::Error for DoclingError`
- `impl From<std::io::Error> for DoclingError`

**Examples:**

```rust
// Note: DocumentConverter is in docling-backend crate
use docling_backend::DocumentConverter;
use docling_core::DoclingError;

fn convert_safe(path: &str) -> Result<String, DoclingError> {
    let converter = DocumentConverter::new()?;

    match converter.convert(path) {
        Ok(result) => Ok(result.document.markdown),
        Err(DoclingError::FormatError(msg)) => {
            eprintln!("Unsupported format: {}", msg);
            Err(DoclingError::FormatError(msg))
        }
        Err(DoclingError::BackendError(msg)) => {
            eprintln!("Backend error: {}", msg);
            Err(DoclingError::BackendError(msg))
        }
        Err(e) => Err(e),
    }
}
```

### Result Type

Convenience type alias:

```rust
pub type Result<T> = std::result::Result<T, DoclingError>;
```

**Usage:**

```rust
use docling_core::Result;

fn my_function() -> Result<Document> {
    let converter = DocumentConverter::new()?;
    let result = converter.convert("file.pdf")?;
    Ok(result.document)
}
```

---

## Advanced Types

### ContentBlock

Represents a structured content block for document elements.

```rust
pub enum ContentBlock {
    /// Section header
    SectionHeader {
        self_ref: String,
        parent: Option<ItemRef>,
        children: Vec<ItemRef>,
        content_layer: String,
        prov: Vec<Provenance>,
        orig: String,
        text: String,
        level: usize,
        formatting: Option<Vec<FormatSpan>>,
        hyperlink: Option<Vec<HyperlinkSpan>>,
    },

    /// Text paragraph
    Text {
        self_ref: String,
        parent: Option<ItemRef>,
        children: Vec<ItemRef>,
        content_layer: String,
        prov: Vec<Provenance>,
        orig: String,
        text: String,
        formatting: Option<Vec<FormatSpan>>,
        hyperlink: Option<Vec<HyperlinkSpan>>,
    },

    /// Table
    Table {
        self_ref: String,
        parent: Option<ItemRef>,
        children: Vec<ItemRef>,
        content_layer: String,
        prov: Vec<Provenance>,
        orig: String,
        text: String,
        data: TableData,
    },

    /// Picture
    Picture {
        self_ref: String,
        parent: Option<ItemRef>,
        children: Vec<ItemRef>,
        content_layer: String,
        prov: Vec<Provenance>,
        annotations: Vec<Annotation>,
        data: PictureData,
    },

    /// List
    List {
        self_ref: String,
        parent: Option<ItemRef>,
        children: Vec<ItemRef>,
        content_layer: String,
        name: String,
        enumerated: bool,
    },

    /// List item
    ListItem {
        self_ref: String,
        parent: Option<ItemRef>,
        children: Vec<ItemRef>,
        content_layer: String,
        enumerated: bool,
        marker: Option<String>,
    },

    // ... (other variants)
}
```

**Note:** ContentBlock is used for structured extraction. When available, `document.content_blocks` contains the parsed document structure.

**Usage:**

```rust
// Access structured content
if let Some(blocks) = doc.blocks() {
    for block in blocks {
        match block {
            ContentBlock::SectionHeader { text, level, .. } => {
                println!("Heading {}: {}", level, text);
            }
            ContentBlock::Text { text, .. } => {
                println!("Paragraph: {}", text);
            }
            ContentBlock::Table { data, .. } => {
                println!("Table with {} rows", data.num_rows);
            }
            _ => {}
        }
    }
}
```

---

### DoclingDocument

Docling's native JSON format (for advanced use).

```rust
pub struct DoclingDocument {
    pub schema_name: String,
    pub version: String,
    pub name: String,
    pub origin: Origin,
    pub body: GroupItem,
    pub furniture: Option<GroupItem>,
    pub texts: Vec<DocItem>,
    pub groups: Vec<DocItem>,
    pub tables: Vec<DocItem>,
    pub pictures: Vec<DocItem>,
    pub key_value_items: Vec<DocItem>,
    pub form_items: Vec<DocItem>,
    pub pages: HashMap<String, PageInfo>,
}
```

**Methods:**

```rust
impl DoclingDocument {
    /// Get all items (texts, groups, tables, pictures)
    pub fn all_items(&self) -> Vec<&DocItem>

    /// Find item by reference path
    pub fn find_item(&self, ref_path: &str) -> Option<&DocItem>
}
```

**Usage:**

```rust
// Load docling JSON
let json = std::fs::read_to_string("document.json")?;
let doc: DoclingDocument = serde_json::from_str(&json)?;

// Access items
println!("Texts: {}", doc.texts.len());
println!("Tables: {}", doc.tables.len());

// Find specific item
if let Some(item) = doc.find_item("#/texts/0") {
    println!("First text: {:?}", item.text());
}
```

---

## Environment Variables

docling-rs uses environment variables for configuration:

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `DOCLING_LOG_LEVEL` | `string` | `info` | Logging level (trace, debug, info, warn, error) |
| `LIBTORCH_USE_PYTORCH` | `bool` | `false` | Use system PyTorch for PDF ML models |

**Setting in Code:**

```rust
// Enable verbose logging
std::env::set_var("DOCLING_LOG_LEVEL", "debug");

// Or in shell:
// export DOCLING_LOG_LEVEL=debug
```

---

## Feature Flags

docling-core Cargo features:

```toml
[dependencies]
docling-core = { version = "0.1", features = ["pdf-ml"] }
```

**Available Features:**

- `pdf-ml` - PDF ML-powered parsing (requires libtorch)
- `ocr` - OCR support (uses ONNX Runtime for RapidOCR)
- `parallel` - Parallel processing support

---

## Complete Example

```rust
// Note: DocumentConverter is in docling-backend crate
use docling_backend::DocumentConverter;
use docling_core::{InputFormat, DoclingError, Result};
use std::path::Path;
use std::fs;

fn main() -> Result<()> {
    // Initialize converter
    let converter = DocumentConverter::new()?;

    // Input file
    let input = Path::new("document.pdf");

    // Convert
    println!("Converting {:?}...", input);
    let result = converter.convert(input)?;

    // Extract information
    let doc = &result.document;
    println!("Format: {}", doc.format);
    println!("Characters: {}", doc.metadata.num_characters);
    println!("Conversion time: {:?}", result.latency);

    // Save markdown
    let output = input.with_extension("md");
    fs::write(&output, &doc.markdown)?;
    println!("Saved to {:?}", output);

    // Check for structured content
    if doc.has_structured_content() {
        println!("Document has {} content blocks", doc.blocks().unwrap().len());
    }

    Ok(())
}

// Error handling example
fn safe_batch_convert(files: &[PathBuf]) -> Vec<Result<String>> {
    let converter = DocumentConverter::new()
        .expect("Failed to create converter");

    files.iter()
        .map(|path| {
            converter.convert(path)
                .map(|r| r.document.markdown)
                .or_else(|e| {
                    eprintln!("Failed to convert {:?}: {}", path, e);
                    Err(e)
                })
        })
        .collect()
}
```

---

## Backend Modules (Advanced)

For direct access to format-specific parsers:

```rust
// Archive backends
pub mod archive;  // ZIP, TAR, 7Z, RAR
pub mod ebook;    // EPUB, FB2, MOBI
pub mod email;    // EML, MBOX, MSG, VCF
pub mod opendocument;  // ODT, ODS, ODP

// Multimedia backends
pub mod audio;    // WAV, MP3
pub mod video;    // MP4, MKV, MOV, AVI, SRT, WebVTT

// Specialty backends
pub mod calendar; // ICS
pub mod notebook; // IPYNB
pub mod gps;      // GPX
pub mod kml;      // KML, KMZ
pub mod svg;      // SVG
pub mod xps;      // XPS
pub mod dicom;    // DICOM
pub mod cad;      // STL, OBJ, GLTF, GLB, DXF
pub mod adobe;    // IDML
```

**Direct Backend Usage:**

```rust
use docling_core::ebook;

// Bypass DocumentConverter, call parser directly
let markdown = ebook::process_epub("book.epub")?;
println!("{}", markdown);
```

**Note:** Direct backend usage bypasses DocumentConverter routing. Use for custom workflows only.

---

## Serializers (Advanced)

```rust
pub mod serializer;

use docling_core::serializer::markdown::DoclingMarkdownExporter;
use docling_core::DoclingDocument;

// Custom serialization
let doc: DoclingDocument = serde_json::from_str(&json)?;
let exporter = DoclingMarkdownExporter::default();
let markdown = exporter.export(&doc)?;
```

**Available Serializers:**
- `markdown` - Markdown export (default)
- `html` - HTML export (future)
- `json` - JSON export (future)

---

## Type Aliases

```rust
// Legacy compatibility
pub type DocumentFormat = InputFormat;
```

---

## Re-exports

For convenience, core types are re-exported at the crate root:

```rust
pub use converter::*;   // DocumentConverter, ConversionResult
pub use document::*;    // Document, DocumentMetadata, DoclingDocument
pub use format::*;      // InputFormat
pub use error::*;       // DoclingError, Result
pub use serializer::*;  // Markdown exporter, etc.
pub use content::*;     // ContentBlock, DocItem, etc.
```

**Usage:**

```rust
// All core types available at root
use docling_core::{
    DocumentConverter,
    Document,
    InputFormat,
    DoclingError,
    Result,
};
```

---

## Version Compatibility

**docling-rs Version:** 0.1.x (Pure Rust + C++)
**Rust Edition:** 2021
**Minimum Rust Version:** 1.70+

**Architecture:**
- 100% Rust implementation with C++ FFI for ML libraries
- PyTorch C++ (libtorch) via tch-rs for PDF ML models
- ONNX Runtime for OCR (RapidOCR)
- No Python dependencies

See [ARCHITECTURE.md](ARCHITECTURE.md) for detailed system design.

---

## See Also

- **User Guide:** [USER_GUIDE.md](USER_GUIDE.md) - Practical usage examples
- **Format Support:** [FORMATS.md](FORMATS.md) - Supported formats and limitations
- **Troubleshooting:** [TROUBLESHOOTING.md](TROUBLESHOOTING.md) - Common issues
- **Contributing:** [CONTRIBUTING.md](CONTRIBUTING.md) - Development guide

---

**Last Updated:** 2025-12-21 (N=3135)
**API Stability:** Stable - Production-ready Rust + C++ implementation
