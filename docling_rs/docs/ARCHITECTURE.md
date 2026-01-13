# Docling-rs Architecture

**Version:** 0.1.x (Pure Rust + C++)
**Last Updated:** 2025-12-08
**Status:** Production-ready - 100% Rust with C++ FFI for ML libraries

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [System Architecture](#system-architecture)
3. [Component Architecture](#component-architecture)
4. [Data Flow](#data-flow)
5. [Design Decisions](#design-decisions)
6. [Extension Points](#extension-points)
7. [Performance Characteristics](#performance-characteristics)
8. [Future Architecture](#future-architecture)
9. [References](#references)

---

## Executive Summary

Docling-rs is a pure Rust + C++ document conversion system that supports **60+ document formats** with **100% test pass rate**. All ML models run natively via C++ FFI (PyTorch/libtorch and ONNX Runtime).

### Architecture Layers

```
┌─────────────────────────────────────────────────────────────┐
│  Layer 1: User Interface                                    │
│  - CLI (docling-cli)                                        │
│  - Rust API (DocumentConverter)                             │
└─────────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────┐
│  Layer 2: Conversion Orchestration                          │
│  - DocumentConverter (crates/docling-core)                  │
│  - Format detection and routing                             │
│  - Batch processing with streaming API                      │
└─────────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────┐
│  Layer 3: Rust Backends (60+ formats)                       │
│  - PDF: ML models via PyTorch C++ (layout, OCR, tables)     │
│  - Office: DOCX, PPTX, XLSX (native Rust)                   │
│  - Web: HTML, Markdown, AsciiDoc, JATS                      │
│  - Ebooks: EPUB, MOBI, FB2                                  │
│  - Archives: ZIP, TAR, 7Z, RAR                              │
│  - Email: EML, MBOX, MSG, VCF                               │
│  - Media: Images (OCR), Video (metadata)                    │
│  - Specialized: CAD, Medical, GPS, Calendar, etc.           │
└─────────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────┐
│  Layer 4: Serialization (Rust)                              │
│  - Markdown                                                 │
│  - HTML                                                     │
│  - JSON                                                     │
│  - YAML                                                     │
└─────────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────┐
│  Layer 5: Output                                            │
│  - File I/O, stdout, structured data                        │
└─────────────────────────────────────────────────────────────┘
```

### Key Characteristics

- **Pure Rust + C++:** No Python dependencies - all backends native Rust or C++ FFI
- **ML via C++ FFI:** PDF ML models via PyTorch C++ (libtorch), OCR via ONNX Runtime
- **Streaming API:** Memory-efficient batch processing with iterator pattern
- **Format Routing:** Automatic backend selection based on file extension/MIME type
- **Performance Framework:** Built-in benchmarking with statistical analysis
- **Production Ready:** 3700+ tests passing (100% pass rate)

---

## System Architecture

### 1. Crate Structure

```
docling_rs/
├── crates/
│   ├── docling-core/              # Main library (public API)
│   │   ├── src/
│   │   │   ├── lib.rs             # Public API surface
│   │   │   ├── converter.rs       # DocumentConverter (orchestration)
│   │   │   ├── backend.rs         # Backend routing and dispatch
│   │   │   ├── format.rs          # InputFormat enum (65 variants)
│   │   │   ├── error.rs           # Error types
│   │   │   ├── types/             # Document data structures
│   │   │   │   ├── mod.rs
│   │   │   │   └── page.rs
│   │   │   ├── serializer/        # Output format serializers
│   │   │   │   ├── mod.rs
│   │   │   │   ├── markdown.rs    # Markdown serializer
│   │   │   │   ├── html.rs        # Rust HTML serializer
│   │   │   │   ├── json.rs        # Rust JSON serializer
│   │   │   │   └── yaml.rs        # Rust YAML serializer
│   │   │   └── [format modules]   # Rust backend implementations
│   │   │       ├── ebook.rs
│   │   │       ├── opendocument.rs
│   │   │       ├── video.rs
│   │   │       ├── audio.rs
│   │   │       ├── adobe.rs
│   │   │       ├── gps.rs
│   │   │       ├── kml.rs
│   │   │       ├── xps.rs
│   │   │       └── legacy.rs
│   │   └── Cargo.toml
│   │
│   ├── docling-backend/           # Specialized backends
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── converter.rs       # Backend converter trait
│   │   │   ├── traits.rs          # Backend traits
│   │   │   ├── utils.rs           # Shared utilities
│   │   │   ├── pdf.rs             # PDF backend (pdfium + ML)
│   │   │   ├── archive.rs         # ZIP, TAR, RAR, 7z
│   │   │   ├── ebooks.rs          # EPUB, MOBI, AZW, FB2
│   │   │   ├── email.rs           # EML, MSG
│   │   │   ├── opendocument.rs    # ODP, ODS
│   │   │   ├── video.rs           # MP4, MKV (metadata)
│   │   │   ├── cad.rs             # DWG, DXF, STL, OBJ
│   │   │   ├── gpx.rs             # GPS tracks
│   │   │   ├── ics.rs             # Calendar
│   │   │   ├── ipynb.rs           # Jupyter notebooks
│   │   │   ├── rtf.rs             # Rich Text Format
│   │   │   ├── svg.rs             # SVG (text extraction)
│   │   │   ├── srt.rs             # Subtitles
│   │   │   ├── webvtt.rs          # Web subtitles
│   │   │   ├── xps.rs             # Microsoft XPS
│   │   │   ├── avif.rs            # AVIF image format
│   │   │   ├── bmp.rs             # BMP images
│   │   │   ├── gif.rs             # GIF images
│   │   │   ├── heif.rs            # HEIF/HEIC images
│   │   │   ├── kml.rs             # Google Earth KML
│   │   │   ├── dicom.rs           # Medical imaging
│   │   │   └── idml.rs            # Adobe InDesign
│   │   └── Cargo.toml
│   │
│   ├── docling-cli/               # Command-line interface
│   │   ├── src/
│   │   │   ├── main.rs            # CLI entry point
│   │   │   ├── commands/          # Subcommands
│   │   │   │   ├── convert.rs     # Single file conversion
│   │   │   │   ├── batch.rs       # Batch conversion with glob
│   │   │   │   └── benchmark.rs   # Performance benchmarking
│   │   │   └── performance/       # Performance analysis
│   │   │       ├── runner.rs      # Benchmark runner
│   │   │       ├── stats.rs       # Statistical analysis
│   │   │       └── report.rs      # Report generation
│   │   └── Cargo.toml
│   │
│   ├── docling-examples/          # Runnable examples
│   │   ├── Cargo.toml
│   │   └── (see examples/ directory)
│   │
│   └── [specialized crates]       # Domain-specific functionality
│       ├── docling-archive/       # Archive handling
│       ├── docling-ebook/         # E-book parsing
│       ├── docling-email/         # Email parsing
│       ├── docling-video/         # Video metadata
│       ├── docling-genomics/      # Scientific formats
│       ├── docling-medical/       # Medical imaging
│       ├── docling-cad/           # CAD formats
│       ├── docling-gps/           # GPS formats
│       ├── docling-calendar/      # Calendar formats
│       ├── docling-notebook/      # Jupyter notebooks
│       ├── docling-svg/           # SVG handling
│       ├── docling-opendocument/  # OpenDocument formats
│       ├── docling-adobe/         # Adobe formats
│       ├── docling-xps/           # XPS format
│       └── docling-legacy/        # Legacy formats
```

### 2. Format Support Matrix

| Format Category | Formats | Backend | Status |
|----------------|---------|---------|--------|
| **PDF** | PDF | Rust + PyTorch C++ (layout, tables, OCR) | ✅ Production |
| **Office** | DOCX, PPTX, XLSX | Native Rust (zip+xml) | ✅ Production |
| **Web** | HTML, CSV, Markdown, AsciiDoc, JATS, WebVTT | Native Rust | ✅ Production |
| **Images** | PNG, JPEG, TIFF, WebP, BMP, GIF, AVIF, HEIF | ONNX Runtime (RapidOCR) | ✅ Production |
| **E-books** | EPUB, MOBI, AZW, AZW3, FB2 | `epub`, `mobi` crates | ✅ Production |
| **Archives** | ZIP, TAR, RAR, 7z | `zip`, `tar`, `sevenz-rust` | ✅ Production |
| **OpenDocument** | ODP, ODS, ODT | Custom ZIP+XML parser | ✅ Production |
| **Email** | EML, MSG, MBOX, VCF | `mailparse`, `msg-parser` | ✅ Production |
| **Video** | MP4, MKV, AVI, WebM | `mp4parse`, `matroska` | ✅ Production |
| **Specialized** | RTF, SVG, SRT, ICS, IPYNB, XPS, GPX, KML | Custom parsers | ✅ Production |
| **CAD** | DWG, DXF, STL, OBJ, IFC, GLTF, GLB | `dxf`, `stl_io`, `gltf` | ✅ Production |
| **Medical** | DICOM | `dicom` crate | ✅ Production |
| **Adobe** | IDML | Custom XML parser | ✅ Production |
| **Apple** | Pages, Numbers, Keynote | Native Rust (protobuf) | ✅ Production |
| **Legacy** | RTF, DOC, WPD, WPS | Rust + external tools | ✅ Production |

**Total:** 60+ formats (all native Rust or C++ FFI - NO Python)

---

## Component Architecture

### Layer 1: User Interface

#### CLI (docling-cli)

**Entry Point:** `crates/docling-cli/src/main.rs`

```
CLI Commands:
├── docling convert <file>              # Single file conversion
│   ├── --format <markdown|html|json|yaml>
│   ├── --output <path>
│   └── --ocr                           # Enable OCR
├── docling batch <pattern>...          # Batch conversion
│   ├── --output <dir>
│   ├── --continue-on-error             # Error recovery
│   └── --glob                          # Glob pattern support
└── docling benchmark <file>            # Performance testing
    ├── --iterations <n>
    ├── --warmup <n>
    └── --format <csv|json|text>
```

**Implementation:**
- Argument parsing (no heavy dependency like `clap` - lightweight custom parser)
- Format selection and validation
- Error reporting with user-friendly messages
- Progress reporting for batch operations

#### Rust API (docling-core)

**Public API Surface:** `crates/docling-core/src/lib.rs`

```rust
// Main entry point
pub struct DocumentConverter { ... }

impl DocumentConverter {
    // Create new converter
    pub fn new() -> Result<Self>;
    pub fn with_ocr(enable_ocr: bool) -> Result<Self>;

    // Convert single document
    pub fn convert<P: AsRef<Path>>(&self, path: P) -> Result<ConversionResult>;

    // Batch conversion (streaming API)
    pub fn convert_all<P: AsRef<Path>>(
        &self,
        paths: impl IntoIterator<Item = P>,
        config: Option<ConversionConfig>,
    ) -> impl Iterator<Item = Result<ConversionResult>>;
}

// Conversion result
pub struct ConversionResult {
    pub document: Document,      // Parsed document
    pub latency: Duration,       // Conversion time
    pub metadata: Metadata,      // Document metadata
}

// Document representation
pub struct Document {
    pub markdown: String,        // Markdown output
    pub pages: Vec<Page>,        // Page metadata
    pub num_pages: usize,
    pub num_characters: usize,
}

impl Document {
    // Export to different formats
    pub fn html(&self) -> Result<String>;
    pub fn to_json(&self) -> Result<String>;
    pub fn to_yaml(&self) -> Result<String>;
}
```

---

### Layer 2: Conversion Orchestration

#### DocumentConverter

**File:** `crates/docling-core/src/converter.rs`

**Responsibilities:**
1. **Format Detection:** Determine InputFormat from file extension
2. **Backend Routing:** Select appropriate Rust/C++ backend
3. **Configuration:** Apply OCR settings, conversion limits
4. **Error Handling:** Catch and wrap errors with context
5. **Result Assembly:** Combine backend output with metadata

**Flow:**
```rust
pub fn convert<P: AsRef<Path>>(&self, path: P) -> Result<ConversionResult> {
    let path = path.as_ref();
    let start = Instant::now();

    // 1. Detect format from extension
    let format = InputFormat::from_path(path)?;

    // 2. Route to appropriate backend (all Rust/C++)
    let document = match format {
        // PDF with ML models (PyTorch C++)
        InputFormat::Pdf => {
            self.convert_pdf_ml(path)?
        }

        // Office formats (native Rust)
        InputFormat::Docx | InputFormat::Pptx | InputFormat::Xlsx => {
            self.convert_office(path, format)?
        }

        // Images with OCR (ONNX Runtime)
        InputFormat::Png | InputFormat::Jpeg | InputFormat::Tiff
        | InputFormat::Webp | InputFormat::Bmp => {
            self.convert_image_ocr(path, format)?
        }

        // All other formats (native Rust backends)
        _ => {
            self.convert_with_backend(path, format)?
        }
    };

    // 3. Assemble result with metadata
    Ok(ConversionResult {
        document,
        latency: start.elapsed(),
        metadata: Metadata::from_path(path)?,
    })
}
```

---

### Layer 3a: PDF ML Backend

**Crate:** `crates/docling-pdf-ml/`

**Purpose:** ML-powered PDF parsing with layout detection, table structure, and OCR

**Technology:** PyTorch C++ (libtorch) via tch-rs, ONNX Runtime for OCR

**Architecture:**
```
┌──────────────────────────────────────────────┐
│  Rust PDF Backend                            │
│  - Page rendering via pdfium                 │
│  - ML inference via tch-rs (PyTorch C++)     │
│  - DocItem generation in pure Rust           │
└──────────────────────────────────────────────┘
                    ↓ C++ FFI
┌──────────────────────────────────────────────┐
│  PyTorch C++ (libtorch)                      │
│  - Layout detection model                    │
│  - Table structure model (TableFormer)       │
│  - Reading order model                       │
│  - Code/formula detection                    │
└──────────────────────────────────────────────┘
                    ↓
┌──────────────────────────────────────────────┐
│  ONNX Runtime                                │
│  - RapidOCR for text recognition             │
│  - Optimized CPU inference                   │
└──────────────────────────────────────────────┘
```

**Implementation Pattern:**
```rust
pub fn convert_pdf_ml(&self, path: &Path) -> Result<Document> {
    // 1. Load PDF and render pages via pdfium
    let pages = self.renderer.render_pages(path)?;

    // 2. Run layout detection (PyTorch C++)
    let layout_results = self.layout_model.detect(&pages)?;

    // 3. Run table structure detection if tables found
    let tables = self.table_model.extract_tables(&layout_results)?;

    // 4. Run OCR on text regions (ONNX Runtime)
    let text_blocks = self.ocr_engine.recognize(&pages, &layout_results)?;

    // 5. Determine reading order
    let ordered_items = self.reading_order.sort(&layout_results)?;

    // 6. Generate DocItems in Rust
    let doc_items = self.generate_doc_items(ordered_items, tables, text_blocks)?;

    // 7. Serialize to markdown
    let markdown = self.serializer.to_markdown(&doc_items)?;

    Ok(Document {
        markdown,
        content_blocks: Some(doc_items),
        num_pages: pages.len(),
        num_characters: markdown.len(),
    })
}
```

**Performance (N=2833 Benchmark):**
- **PyTorch backend (default):** ~153 ms/page (6.5 pages/sec)
- **ONNX backend (fallback):** ~239 ms/page (4.2 pages/sec)
- **Layout detection:** 98.9% of total processing time
- **Batch inference:** 1.5-2x throughput improvement

**ML Models (ported from Python docling):**
- **LayoutPredictor** - Document layout detection (text, tables, figures, headers)
- **TableFormer** - Table structure recognition (rows, columns, spans)
- **RapidOCR** - Text recognition via ONNX Runtime
- **ReadingOrder** - Determine text flow order
- **CodeFormula** - Detect code blocks and mathematical formulas

---

### Layer 3b: Rust Backends

**File:** `crates/docling-backend/src/converter.rs`

**Backend Trait:**
```rust
pub trait BackendConverter {
    /// Convert document to markdown
    fn convert(&self, path: &Path) -> Result<String>;

    /// Extract metadata
    fn metadata(&self, path: &Path) -> Result<Metadata> {
        Ok(Metadata::default())
    }

    /// Supported formats
    fn supported_formats() -> &'static [InputFormat];
}
```

**Backend Implementations:**

#### 1. E-book Backend (`ebooks.rs`)

**Formats:** EPUB, MOBI, AZW, AZW3, FB2

**Libraries:**
- `epub` - EPUB parsing
- `mobi` - MOBI/AZW parsing

**Implementation:**
```rust
pub struct EbookConverter;

impl BackendConverter for EbookConverter {
    fn convert(&self, path: &Path) -> Result<String> {
        match InputFormat::from_path(path)? {
            InputFormat::Epub => self.convert_epub(path),
            InputFormat::Mobi | InputFormat::Azw | InputFormat::Azw3 => {
                self.convert_mobi(path)
            }
            InputFormat::Fb2 => self.convert_fb2(path),
            _ => Err(DoclingError::FormatError("Unsupported e-book format".into())),
        }
    }
}

fn convert_epub(&self, path: &Path) -> Result<String> {
    let doc = EpubDoc::new(path)?;
    let mut markdown = String::new();

    // Extract metadata
    markdown.push_str(&format!("# {}\n\n", doc.mdata("title").unwrap_or_default()));
    markdown.push_str(&format!("**Author:** {}\n\n", doc.mdata("creator").unwrap_or_default()));

    // Extract chapters
    for i in 0..doc.get_num_pages() {
        doc.set_current_page(i);
        let content = doc.get_current_str()?;
        let text = html2text::from_read(content.as_bytes(), 80);
        markdown.push_str(&text);
        markdown.push_str("\n\n---\n\n");
    }

    Ok(markdown)
}
```

#### 2. Archive Backend (`archive.rs`)

**Formats:** ZIP, TAR, RAR, 7z

**Libraries:**
- `zip` - ZIP extraction
- `tar` - TAR extraction
- `sevenz-rust` - 7z extraction

**Implementation:**
```rust
pub struct ArchiveConverter;

impl BackendConverter for ArchiveConverter {
    fn convert(&self, path: &Path) -> Result<String> {
        let format = InputFormat::from_path(path)?;
        let file_list = self.extract_file_list(path, format)?;

        let mut markdown = String::new();
        markdown.push_str(&format!("# Archive: {}\n\n", path.display()));
        markdown.push_str(&format!("**Format:** {}\n", format));
        markdown.push_str(&format!("**Files:** {}\n\n", file_list.len()));

        markdown.push_str("## Contents\n\n");
        for (name, size) in file_list {
            markdown.push_str(&format!("- `{}` ({} bytes)\n", name, size));
        }

        Ok(markdown)
    }
}
```

#### 3. Email Backend (`email.rs`)

**Formats:** EML, MSG

**Libraries:**
- `mailparse` - EML parsing
- `msg-parser` - Outlook MSG parsing

**Implementation:**
```rust
pub struct EmailConverter;

impl BackendConverter for EmailConverter {
    fn convert(&self, path: &Path) -> Result<String> {
        let format = InputFormat::from_path(path)?;
        match format {
            InputFormat::Eml => self.convert_eml(path),
            InputFormat::Msg => self.convert_msg(path),
            _ => Err(DoclingError::FormatError("Unsupported email format".into())),
        }
    }
}

fn convert_eml(&self, path: &Path) -> Result<String> {
    let contents = fs::read(path)?;
    let parsed = parse_mail(&contents)?;

    let mut markdown = String::new();
    markdown.push_str(&format!("# Email: {}\n\n",
        parsed.headers.get_first_value("Subject").unwrap_or_default()));
    markdown.push_str(&format!("**From:** {}\n",
        parsed.headers.get_first_value("From").unwrap_or_default()));
    markdown.push_str(&format!("**To:** {}\n",
        parsed.headers.get_first_value("To").unwrap_or_default()));
    markdown.push_str(&format!("**Date:** {}\n\n",
        parsed.headers.get_first_value("Date").unwrap_or_default()));

    markdown.push_str("## Body\n\n");
    markdown.push_str(&parsed.get_body()?);

    // Attachments
    if !parsed.subparts.is_empty() {
        markdown.push_str("\n\n## Attachments\n\n");
        for part in &parsed.subparts {
            if let Some(filename) = part.get_content_disposition().params.get("filename") {
                markdown.push_str(&format!("- {}\n", filename));
            }
        }
    }

    Ok(markdown)
}
```

**See:** `crates/docling-backend/src/` for all backend implementations.

---

### Layer 4: Serialization

#### Markdown Serializer

**File:** `crates/docling-core/src/serializer/markdown.rs`

**Purpose:** Converts DocItem tree to markdown format

**Serializer Implementation:**
```rust
pub fn serialize_to_markdown(doc: &DoclingDocument) -> String {
    let mut output = String::new();

    // Iterate document items
    for (item, level) in doc.iterate_items() {
        match item {
            DocItem::Text(text) => {
                output.push_str(&text.text);
                output.push_str("\n\n");
            }
            DocItem::SectionHeader(header) => {
                let prefix = "#".repeat(header.level as usize);
                output.push_str(&format!("{} {}\n\n", prefix, header.text));
            }
            DocItem::Table(table) => {
                output.push_str(&serialize_table(table));
                output.push_str("\n\n");
            }
            DocItem::List(list) => {
                output.push_str(&serialize_list(list, level));
                output.push_str("\n\n");
            }
            DocItem::Code(code) => {
                output.push_str("```\n");
                output.push_str(&code.text);
                output.push_str("\n```\n\n");
            }
            // ... other DocItem types
        }
    }

    output
}
```

#### HTML Serializer

**File:** `crates/docling-core/src/serializer/html.rs`

**Implementation:**
```rust
pub fn to_html(document: &Document) -> Result<String> {
    let mut html = String::new();

    html.push_str("<!DOCTYPE html>\n");
    html.push_str("<html>\n<head>\n");
    html.push_str("  <meta charset=\"UTF-8\">\n");
    html.push_str("  <title>Document</title>\n");
    html.push_str("</head>\n<body>\n");

    // Convert markdown to HTML (using pulldown-cmark)
    let parser = Parser::new(&document.markdown);
    let mut html_buf = String::new();
    html::push_html(&mut html_buf, parser);
    html.push_str(&html_buf);

    html.push_str("</body>\n</html>\n");

    Ok(html)
}
```

#### JSON Serializer

**File:** `crates/docling-core/src/serializer/json.rs`

**Implementation:**
```rust
use serde_json;

pub fn to_json(document: &Document) -> Result<String> {
    #[derive(Serialize)]
    struct JsonOutput {
        markdown: String,
        num_pages: usize,
        num_characters: usize,
        pages: Vec<PageMetadata>,
    }

    let output = JsonOutput {
        markdown: document.markdown.clone(),
        num_pages: document.num_pages,
        num_characters: document.num_characters,
        pages: document.pages.clone(),
    };

    serde_json::to_string_pretty(&output)
        .map_err(|e| DoclingError::JsonError(e.to_string()))
}
```

#### YAML Serializer

**File:** `crates/docling-core/src/serializer/yaml.rs`

**Implementation:**
```rust
use serde_yaml;

pub fn to_yaml(document: &Document) -> Result<String> {
    // Same structure as JSON, different serializer
    serde_yaml::to_string(&document)
        .map_err(|e| DoclingError::JsonError(e.to_string()))
}
```

---

## Data Flow

### Single Document Conversion

```
User API Call
    ↓
converter.convert("document.pdf")
    ↓
┌─────────────────────────────────────────────┐
│ DocumentConverter::convert()                │
│ 1. Detect format from extension             │
│ 2. Route to backend                         │
└─────────────────────────────────────────────┘
    ↓
    ├─→ PDF ML Backend (PDF with ML models)
    │   ↓
    │   Rust + PyTorch C++:
    │   - Renders pages via pdfium
    │   - Runs ML models (Layout, OCR, Tables) via libtorch
    │   - Generates DocItems in Rust
    │   - Serializes to markdown
    │   ↓
    │   Return Document with markdown
    │
    └─→ Native Rust Backend (DOCX, EPUB, ZIP, etc.)
        ↓
        Rust native parser:
        - Loads file with format-specific library
        - Extracts text/metadata
        - Formats as markdown
        ↓
        Return markdown String
    ↓
┌─────────────────────────────────────────────┐
│ Assemble ConversionResult                   │
│ - markdown: String                          │
│ - latency: Duration                         │
│ - metadata: Metadata                        │
└─────────────────────────────────────────────┘
    ↓
Return Result<ConversionResult>
```

### Batch Conversion (Streaming API)

```
converter.convert_all(paths, config)
    ↓
Returns Iterator<Item = Result<ConversionResult>>
    ↓
┌─────────────────────────────────────────────┐
│ For each path in paths:                     │
│   1. Validate path exists                   │
│   2. Check file size limits                 │
│   3. Convert document (same as single)      │
│   4. Yield result                           │
│   5. Continue to next (lazy evaluation)     │
└─────────────────────────────────────────────┘
    ↓
User iterates:
    for result in converter.convert_all(...) {
        match result {
            Ok(conv) => process(conv),
            Err(e) => handle_error(e),
        }
    }
```

**Memory Characteristics:**
- **Lazy Evaluation:** Only one document in memory at a time
- **Error Recovery:** Iterator continues on errors (if `raises_on_error=false`)
- **Progress Reporting:** Real-time statistics (files processed, errors, duration)

---

## Design Decisions

### 1. Pure Rust + C++ Architecture

**Decision:** 100% Rust implementation with C++ FFI for ML models

**Rationale:**
- **Maximum Performance:** No Python GIL, native compiled code
- **Minimal Dependencies:** No Python runtime required
- **Production Quality:** ML models via PyTorch C++ (libtorch) and ONNX Runtime
- **Broad Format Support:** 60+ formats all native Rust or C++ FFI

**Trade-offs:**
- ✅ **Pros:** Fastest possible performance, single binary deployment, no Python dependency
- ✅ **Pros:** Full parallelism (no GIL), lower memory footprint
- ⚠️ **Consideration:** Requires libtorch for PDF ML features

**Result:** Successfully migrated from Python hybrid to 100% Rust/C++

---

### 2. Streaming API for Batch Processing

**Decision:** Use iterator pattern for `convert_all()`

**Rationale:**
- **Memory Efficiency:** Process one document at a time, not all in memory
- **Error Recovery:** Iterator continues on errors
- **Lazy Evaluation:** Only processes documents as consumed
- **Composability:** Users can chain with other iterators (`.filter()`, `.map()`, etc.)

**Implementation:**
```rust
pub fn convert_all<P: AsRef<Path>>(
    &self,
    paths: impl IntoIterator<Item = P>,
    config: Option<ConversionConfig>,
) -> impl Iterator<Item = Result<ConversionResult>> {
    paths.into_iter().map(move |path| {
        self.convert_single_with_config(path.as_ref(), &config)
    })
}
```

**Alternative Considered:** Return `Vec<ConversionResult>`
- **Rejected:** Would load all documents in memory, fails on large batches

---

### 3. ML Model Integration via C++ FFI

**Decision:** Use PyTorch C++ (libtorch) and ONNX Runtime for ML inference

**Rationale:**
- **Production Quality Models:** Ported from Python docling's proven ML pipeline
- **Native Performance:** C++ inference without Python overhead
- **Flexible Backends:** PyTorch for full feature support, ONNX for portability

**ML Stack:**
- **PyTorch C++ (libtorch):** Layout detection, table structure (default)
- **ONNX Runtime:** RapidOCR for text recognition

**When to Use:**
- PDF with ML features: Set `LIBTORCH_USE_PYTORCH=1` for system PyTorch
- OCR-only: ONNX Runtime handles RapidOCR automatically

**Performance:** ~153 ms/page with PyTorch backend (6.5 pages/sec)

---

### 4. Crate Organization (Workspace)

**Decision:** Separate crates for core, backend, CLI, examples

**Rationale:**
- **Modularity:** Users can depend on `docling-core` without CLI
- **Compilation Speed:** Backend changes don't rebuild CLI
- **Domain Separation:** Specialized backends in separate crates
- **Testing:** Test backends independently

**Structure:**
```
docling_rs/
├── crates/
│   ├── docling-core/       # Public API (users depend on this)
│   ├── docling-backend/    # Backend implementations
│   ├── docling-cli/        # CLI binary
│   ├── docling-examples/   # Examples package
│   └── [specialized]/      # Domain-specific crates
```

---

### 5. Error Handling Strategy

**Decision:** Custom error type with context

**Implementation:**
```rust
#[derive(Debug, thiserror::Error)]
pub enum DoclingError {
    #[error("Backend error: {0}")]
    BackendError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Conversion failed: {0}")]
    ConversionError(String),

    #[error("Format error: {0}")]
    FormatError(String),

    #[error("JSON error: {0}")]
    JsonError(String),
}
```

**Rationale:**
- **Type Safety:** Compile-time error handling
- **Context:** Detailed error messages
- **Ergonomics:** `?` operator works seamlessly
- **Interop:** Converts from standard errors (`std::io::Error`, etc.)

---

### 6. Format Detection from Extension

**Decision:** Detect format from file extension, not magic bytes

**Implementation:**
```rust
impl InputFormat {
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let extension = path.as_ref()
            .extension()
            .and_then(|s| s.to_str())
            .ok_or_else(|| DoclingError::FormatError("No extension".into()))?;

        match extension.to_lowercase().as_str() {
            "pdf" => Ok(InputFormat::Pdf),
            "docx" => Ok(InputFormat::Docx),
            "epub" => Ok(InputFormat::Epub),
            // ... 52 more formats
            _ => Err(DoclingError::FormatError(format!("Unsupported: {}", extension))),
        }
    }
}
```

**Rationale:**
- **Fast:** No need to open file to detect format
- **Simple:** Clear mapping extension → format
- **Sufficient:** Users know their file types

**Alternative Considered:** Magic byte detection
- **Rejected:** Slower (requires file I/O), complex (many formats share bytes)
- **Future:** Add as fallback for extensionless files

---

## Extension Points

### Adding a New Format

**Steps:**

1. **Add to InputFormat enum** (`crates/docling-core/src/format.rs`)
```rust
pub enum InputFormat {
    // Existing formats...

    /// Your new format
    YourFormat,
}
```

2. **Update from_path()** (same file)
```rust
match extension.to_lowercase().as_str() {
    // Existing mappings...

    "yourext" => Ok(InputFormat::YourFormat),
}
```

3. **Create backend implementation** (`crates/docling-backend/src/your_format.rs`)
```rust
use crate::traits::BackendConverter;

pub struct YourFormatConverter;

impl BackendConverter for YourFormatConverter {
    fn convert(&self, path: &Path) -> Result<String> {
        // 1. Parse file
        let contents = std::fs::read_to_string(path)?;

        // 2. Extract text/structure
        let text = your_parsing_logic(&contents)?;

        // 3. Format as markdown
        let mut markdown = String::new();
        markdown.push_str("# Your Format Document\n\n");
        markdown.push_str(&text);

        Ok(markdown)
    }

    fn supported_formats() -> &'static [InputFormat] {
        &[InputFormat::YourFormat]
    }
}
```

4. **Register backend** (`crates/docling-core/src/converter.rs`)
```rust
match format {
    // Existing backends...

    InputFormat::YourFormat => {
        let converter = YourFormatConverter;
        converter.convert(path)?
    }
}
```

5. **Add tests** (`crates/docling-backend/tests/your_format_tests.rs`)
```rust
#[test]
fn test_your_format_basic() {
    let converter = DocumentConverter::new().unwrap();
    let result = converter.convert("test-files/sample.yourext").unwrap();
    assert!(!result.document.markdown.is_empty());
}
```

**See:** [CONTRIBUTING.md](CONTRIBUTING.md) for detailed guide

---

### Adding a New Output Format

**Steps:**

1. **Create serializer** (`crates/docling-core/src/serializer/your_format.rs`)
```rust
pub fn to_your_format(document: &Document) -> Result<String> {
    // Your serialization logic
    Ok(formatted_output)
}
```

2. **Add to Document** (`crates/docling-core/src/types/mod.rs`)
```rust
impl Document {
    // Existing methods...

    pub fn to_your_format(&self) -> Result<String> {
        serializer::your_format::to_your_format(self)
    }
}
```

3. **Update CLI** (`crates/docling-cli/src/commands/convert.rs`)
```rust
match output_format {
    // Existing formats...

    "yourformat" => {
        let output = result.document.to_your_format()?;
        fs::write(output_path, output)?;
    }
}
```

---

### Custom Backend Trait

**For specialized backends:**

```rust
pub trait CustomBackend: BackendConverter {
    /// Extract structured data (not just markdown)
    fn extract_structure(&self, path: &Path) -> Result<StructuredData>;

    /// Validate file before conversion
    fn validate(&self, path: &Path) -> Result<()>;

    /// Get detailed metadata
    fn detailed_metadata(&self, path: &Path) -> Result<DetailedMetadata>;
}
```

**Use case:** Backends that need more than simple text extraction (e.g., CAD files with geometry, medical images with DICOM tags)

---

## Performance Characteristics

### Latency Breakdown (Actual - N=2833 Benchmark)

| Format | Size | Latency | Throughput |
|--------|------|---------|------------|
| PDF (ML) | 5 pages | ~770ms | 6.5 pages/sec |
| PDF (ONNX fallback) | 5 pages | ~1200ms | 4.2 pages/sec |
| DOCX | 500 KB | ~30ms | High |
| EPUB | 2 MB | ~200ms | High |
| ZIP | 10 files | ~50ms | Very High |
| HTML | 100 KB | ~5ms | Very High |

**PDF ML Backend Performance:**
- **PyTorch backend (default):** ~153 ms/page (6.5 pages/sec)
- **ONNX backend (fallback):** ~239 ms/page (4.2 pages/sec)
- **Layout detection:** 98.9% of total processing time
- **Batch inference:** 1.5-2x throughput improvement

**Rust Backend Advantages:**
1. **Zero-cost Abstractions:** No runtime overhead
2. **Parallel Processing:** Can use `rayon` for multi-core
3. **Memory Efficiency:** No GC pauses
4. **No GIL:** Full parallelism for all operations

---

### Memory Usage

| Component | Memory |
|-----------|--------|
| **Rust Runtime** | 5-10 MB |
| **libtorch (ML models)** | ~400 MB (Layout + Tables) |
| **ONNX Runtime (OCR)** | ~100 MB |
| **Per Document** | 1-5 MB (minimal overhead) |

**Streaming API Memory:**
- **Peak:** Single largest document in batch (not total)
- **Average:** 10-50 MB during processing
- **Advantage:** Can process unlimited documents in constant memory

---

## Future Architecture

### Phase I: Native Rust PDF Backend - COMPLETE

**Status:** ✅ Implemented in `crates/docling-pdf-ml/`

**Components Implemented:**
1. **PDF Parser:** `pdfium-render` for page rendering
2. **Text Extraction:** Via pdfium or OCR
3. **Layout Analysis:** ML models via PyTorch C++ (libtorch)
4. **Table Detection:** TableFormer model via PyTorch C++
5. **OCR:** RapidOCR via ONNX Runtime

**Achieved Benefits:**
- Native performance (~153 ms/page)
- True parallel page processing
- No Python dependency

---

### Phase II: Structured Document Representation

**Goal:** Return structured DocItem tree, not just markdown

**API:**
```rust
pub struct DocumentTree {
    pub items: Vec<DocItem>,
}

pub enum DocItem {
    Text { text: String, bbox: BoundingBox },
    SectionHeader { text: String, level: u8, bbox: BoundingBox },
    Table { data: TableData, bbox: BoundingBox },
    Picture { image: Vec<u8>, bbox: BoundingBox },
    List { items: Vec<ListItem>, bbox: BoundingBox },
    Code { code: String, language: Option<String> },
}

impl DocumentConverter {
    pub fn convert_structured(&self, path: &Path) -> Result<DocumentTree>;
}
```

**Use Cases:**
- Custom serializers (user-defined output)
- Semantic search (query by item type)
- Layout analysis (bounding boxes)
- Document comparison (structural diff)

---

### Phase III: Plugin System

**Goal:** Allow third-party backends without modifying core

**API:**
```rust
pub trait PluginBackend: BackendConverter {
    fn name(&self) -> &str;
    fn version(&self) -> &str;
}

impl DocumentConverter {
    pub fn register_plugin<B: PluginBackend>(&mut self, backend: B);
}
```

**Use Cases:**
- Proprietary formats (company-internal)
- Experimental backends (community contributions)
- Domain-specific formats (scientific, legal, etc.)

---

## References

### Documentation

- [User Guide](USER_GUIDE.md) - Installation, usage, best practices
- [API Reference](API.md) - Complete API documentation
- [Format Support](FORMATS.md) - Supported formats, test coverage
- [Benchmarking](../BENCHMARKING.md) - Performance measurement
- [Troubleshooting](TROUBLESHOOTING.md) - Common issues, solutions
- [Contributing](CONTRIBUTING.md) - Development guide, adding formats

### Historical Reference

- [DOCLING_ARCHITECTURE.md](../DOCLING_ARCHITECTURE.md) - Original Python docling v2.58.0 analysis (archived)
- [PHASE_0_ARCHITECTURE.md](../PHASE_0_ARCHITECTURE.md) - Original Python bridge details (archived)

### Project Planning

- [MASTER_PLAN.md](../MASTER_PLAN.md) - Long-term roadmap
- [FORMAT_PROCESSING_GRID.md](../FORMAT_PROCESSING_GRID.md) - Format coverage tracking

### Source Code

- Main Library: `crates/docling-core/src/`
- Backends: `crates/docling-backend/src/`
- PDF ML: `crates/docling-pdf-ml/src/`
- CLI: `crates/docling-cli/src/`
- Examples: `examples/`

---

**Document Version:** 2.0
**Architecture:** Pure Rust + C++ (No Python)
**Status:** Production-ready (3700+ tests passing)

---

**End of Architecture Document**
