# docling-cli

Command-line document converter for docling-rs - convert 55+ document formats to Markdown, JSON, YAML, or HTML from your terminal.

[![Crates.io](https://img.shields.io/crates/v/docling-cli.svg)](https://crates.io/crates/docling-cli)
[![Documentation](https://docs.rs/docling-cli/badge.svg)](https://docs.rs/docling-cli)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Features

- **55+ Document Formats**: PDF, DOCX, PPTX, XLSX, HTML, e-books, images, archives, and more
- **Multiple Output Formats**: Markdown, JSON, YAML, HTML
- **Batch Processing**: Convert entire directories with glob patterns
- **Performance Benchmarking**: Measure conversion speed with statistical analysis
- **Progress Tracking**: Real-time progress bars for batch operations
- **Error Recovery**: Continue processing on errors with detailed error reporting
- **Shell Completions**: Tab completion for Bash, Zsh, Fish, PowerShell
- **Configuration Files**: Project and user-level defaults via `.docling.toml`

## Installation

### From crates.io

```bash
cargo install docling-cli
```

### From source

```bash
git clone https://github.com/ayates_dbx/docling_rs
cd docling_rs
cargo install --path crates/docling-cli
```

### Prerequisites

Python docling v2.58.0 is required for ML-powered parsing:

```bash
pip install docling==2.58.0
```

## Quick Start

### Convert a Single Document

```bash
# Convert PDF to markdown
docling convert document.pdf

# Convert to JSON
docling convert document.pdf --format json

# Convert with output file
docling convert document.pdf -o output.md

# Convert DOCX to HTML
docling convert report.docx --format html -o report.html
```

### Batch Conversion

```bash
# Convert all PDFs in a directory
docling batch documents/*.pdf -o output/

# Convert multiple formats
docling batch docs/**/*.{pdf,docx,pptx} -o converted/

# Continue on errors
docling batch docs/*.pdf -o output/ --continue-on-error

# Parallel processing
docling batch docs/*.pdf -o output/ --jobs 4
```

### Benchmarking

```bash
# Run benchmark (10 iterations)
docling benchmark document.pdf

# More iterations for accuracy
docling benchmark document.pdf -n 100

# Save results to CSV
docling benchmark document.pdf -n 50 --format csv -o results.csv

# Benchmark with warmup
docling benchmark document.pdf -n 20 --warmup 5

# JSON output for automation
docling benchmark document.pdf --format json -o metrics.json
```

## Commands

### `convert`

Convert a single document to the specified format.

```bash
docling convert <INPUT> [OPTIONS]

OPTIONS:
  -o, --output <FILE>        Output file path (default: stdout)
  -f, --format <FORMAT>      Output format [markdown|json|yaml|html] (default: markdown)
  -b, --backend <BACKEND>    Backend to use [rust|python|auto] (default: auto)
  --ocr                      Enable OCR for scanned documents
  --no-progress              Disable progress indicators
```

**Examples:**
```bash
# Convert to stdout
docling convert document.pdf

# Convert to file
docling convert document.pdf -o output.md

# Enable OCR for scanned PDFs
docling convert scanned.pdf --ocr -o output.md

# Force Python backend (all features)
docling convert document.pdf --backend python
```

### `batch`

Convert multiple documents with error handling and progress tracking.

```bash
docling batch <PATTERNS>... [OPTIONS]

OPTIONS:
  -o, --output-dir <DIR>     Output directory (required)
  -f, --format <FORMAT>      Output format (default: markdown)
  -j, --jobs <N>             Parallel jobs (default: num CPUs)
  --continue-on-error        Continue processing on errors
  --no-progress              Disable progress bar
```

**Examples:**
```bash
# Convert directory
docling batch documents/*.pdf -o output/

# Multiple patterns
docling batch docs/*.pdf reports/*.docx -o converted/

# Parallel processing with error recovery
docling batch large-dataset/**/*.pdf -o output/ -j 8 --continue-on-error

# Convert to JSON
docling batch *.pdf -o json_output/ --format json
```

### `benchmark`

Measure conversion performance with statistical analysis.

```bash
docling benchmark <INPUT> [OPTIONS]

OPTIONS:
  -n, --iterations <N>       Number of iterations (default: 10)
  -w, --warmup <N>           Warmup iterations (default: 2)
  -f, --format <FORMAT>      Output format [text|json|csv|markdown] (default: text)
  -o, --output <FILE>        Save results to file
  --no-outliers              Remove statistical outliers
```

**Output includes:**
- Mean, median, standard deviation
- Min/max times
- 95th percentile
- Throughput (pages/sec)
- Memory usage

**Examples:**
```bash
# Basic benchmark
docling benchmark document.pdf

# High-precision benchmark
docling benchmark document.pdf -n 100 --warmup 10

# CSV output for analysis
docling benchmark document.pdf -n 50 --format csv -o results.csv

# Markdown table
docling benchmark document.pdf --format markdown
```

**Sample output:**
```
Benchmark Results for document.pdf
===================================
Iterations: 50 (after 5 warmup runs)
Document: 10 pages, 2.3 MB

Mean:       127.5 ms
Median:     125.2 ms
Std Dev:    8.3 ms
Min:        118.1 ms
Max:        156.7 ms
95th %ile:  142.3 ms

Throughput: 78.4 pages/second
Memory:     45.2 MB peak
```

### `completion`

Generate shell completion scripts.

```bash
docling completion <SHELL>

SHELLS:
  bash, zsh, fish, powershell, elvish
```

**Installation:**

**Bash:**
```bash
docling completion bash | sudo tee /usr/local/etc/bash_completion.d/docling
```

**Zsh:**
```bash
docling completion zsh > ~/.zfunc/_docling
# Add to ~/.zshrc: fpath+=~/.zfunc
```

**Fish:**
```bash
docling completion fish > ~/.config/fish/completions/docling.fish
```

**PowerShell:**
```powershell
docling completion powershell | Out-String | Invoke-Expression
```

## Configuration Files

Create `.docling.toml` for project or user defaults:

**Location precedence:**
1. Command-line arguments (highest priority)
2. Project config (`./.docling.toml`)
3. User config (`~/.docling.toml`)
4. Built-in defaults (lowest priority)

**Example `.docling.toml`:**
```toml
[convert]
format = "markdown"
backend = "auto"
ocr = false

[batch]
format = "json"
jobs = 4
continue_on_error = true

[benchmark]
iterations = 50
warmup = 5
format = "csv"
```

**Specify custom config:**
```bash
docling --config /path/to/config.toml convert document.pdf
```

## Output Formats

### Markdown (default)
```bash
docling convert document.pdf -f markdown
```
- Clean, readable text
- Preserved headings, lists, tables
- Suitable for documentation, note-taking

### JSON
```bash
docling convert document.pdf -f json
```
- Structured document data
- Full metadata (pages, bounding boxes, labels)
- Suitable for programmatic processing

### YAML
```bash
docling convert document.pdf -f yaml
```
- Human-readable structured data
- Similar to JSON but easier to read/edit
- Suitable for configuration, data exchange

### HTML
```bash
docling convert document.pdf -f html
```
- Web-ready output
- Semantic HTML structure
- Suitable for web publishing, previews

## Supported Formats

| Category | Extensions |
|----------|------------|
| **Documents** | `.pdf`, `.docx`, `.doc`, `.pptx`, `.ppt`, `.xlsx`, `.xls`, `.rtf` |
| **Web** | `.html`, `.htm`, `.md`, `.csv`, `.adoc` |
| **Images** | `.png`, `.jpg`, `.jpeg`, `.tiff`, `.tif`, `.webp`, `.bmp`, `.gif`, `.svg`, `.ico`, `.heif`, `.heic`, `.avif` |
| **E-books** | `.epub`, `.mobi`, `.azw`, `.fb2` |
| **Archives** | `.zip`, `.tar`, `.gz`, `.bz2`, `.xz`, `.7z`, `.rar` |
| **Email** | `.eml`, `.msg` |
| **Calendar** | `.ics`, `.vcf` |
| **Scientific** | `.nxml` (JATS), `.tex` (LaTeX), `.ipynb` |
| **Multimedia** | `.mp3`, `.wav`, `.flac`, `.ogg`, `.mp4`, `.webm`, `.avi`, `.mkv`, `.srt`, `.vtt` |
| **Geospatial** | `.gpx`, `.kml` |
| **Medical** | `.dcm` (DICOM) |
| **CAD/3D** | `.dxf`, `.stl`, `.obj`, `.gltf` |

## Environment Variables

- `USE_HYBRID_SERIALIZER=1` - Use Python ML + Rust serialization
- `USE_RUST_BACKEND=1` - Force pure Rust backend (limited formats)
- `DOCLING_CONFIG` - Path to config file

## Performance Tips

1. **Use `--backend rust`** for supported formats (5-10x faster)
2. **Parallel batch processing**: `--jobs 8` (for multi-core machines)
3. **Disable progress bars** in scripts: `--no-progress`
4. **Warmup benchmarks** for accurate results: `--warmup 5`

## Error Handling

The CLI provides detailed error messages with context:

```bash
$ docling convert missing.pdf
Error: Failed to convert document
  Caused by:
    File not found: missing.pdf

$ docling batch bad/*.pdf -o output/ --continue-on-error
✓ doc1.pdf (10 pages, 127ms)
✗ corrupted.pdf: Invalid PDF structure
✓ doc2.pdf (5 pages, 89ms)

Summary: 2/3 succeeded, 1 failed
```

## Contributing

Contributions are welcome! See [CONTRIBUTING.md](../../CONTRIBUTING.md) for guidelines.

## License

Licensed under the MIT License. See [LICENSE](../../LICENSE) for details.

## Links

- **Repository**: https://github.com/ayates_dbx/docling_rs
- **Documentation**: https://docs.rs/docling-cli
- **Issues**: https://github.com/ayates_dbx/docling_rs/issues
- **Python docling**: https://github.com/docling-project/docling

## Related Crates

- `docling-core` - Core document types and serialization
- `docling-backend` - Document parsing backends
