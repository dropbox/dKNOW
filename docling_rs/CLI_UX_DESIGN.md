# World-Class CLI UX Design for Docling

**Created:** 2025-12-06 by MANAGER
**Status:** IMPLEMENTATION COMPLETE (N=2677-2691)
**Inspiration:** ripgrep, fd, bat, ffmpeg, pandoc, imagemagick

---

## Executive Summary

This document defines a world-class CLI experience for docling, transforming it from a functional tool into a delightful, intuitive interface that users love. The design follows principles from the best CLI tools: **zero-config excellence**, **progressive disclosure**, **safety by default**, and **beautiful feedback**.

---

## Current State Analysis

### What's Good ✅
- Clear command structure (convert, batch, benchmark)
- Shell completion support
- Multiple output formats (md, json, yaml)
- Progress bars for long operations
- Good ML device auto-detection
- Config file support (.docling.toml)

### All Features Implemented ✅
| Feature | Implementation | Commit |
|---------|----------------|--------|
| Smart output naming | Auto-generates output path | N=2678 |
| Stdin/stdout piping | `docling convert -` | N=2681 |
| `--dry-run` mode | Preview without converting | N=2680 |
| `formats` command | Lists all 55+ formats | N=2682 |
| `info` command | Inspect files before converting | N=2683 |
| `--quiet`/`--verbose` | Full verbosity control | N=2684, N=2686 |
| `--force`/`--no-clobber` | Output file safety | N=2679 |
| `--watch` mode | Continuous conversion | N=2690 |
| `config` command | Configuration management | N=2687 |
| `--profile` presets | Named configuration presets | N=2691 |
| `batch --stdin` | Pipe file lists | N=2688 |
| `batch --parallel` | Parallel processing | N=2689 |
| `--max-pages` | Limit PDF pages | N=2677 |

---

## Design Principles

### 1. 🎯 Zero-Config Excellence
**"It should just work."**
```bash
# Current (requires explicit output)
docling convert report.pdf -o report.md

# Ideal (smart defaults)
docling convert report.pdf
# → Creates report.md automatically
```

### 2. 📚 Progressive Disclosure
**Simple for beginners, powerful for experts.**
```bash
# Level 1: Beginner (just works)
docling convert file.pdf

# Level 2: Intermediate (common options)
docling convert file.pdf -f json --ocr

# Level 3: Advanced (full control)
docling convert file.pdf --ocr --device cuda --batch-size 16 --model-size accurate
```

### 3. 🛡️ Safety by Default
**Never lose data. Always recoverable.**
```bash
# Won't overwrite existing files without --force
docling convert file.pdf
# → Error: output.md already exists. Use --force to overwrite.

# Dry-run to preview what would happen
docling convert *.pdf --dry-run
# → Would convert: report.pdf → report.md
# → Would convert: thesis.pdf → thesis.md
# → Total: 2 files
```

### 4. 🎨 Beautiful Feedback
**Clear, colored, informative output.**
```
$ docling convert report.pdf
✓ Detected format: PDF (42 pages, 2.3 MB)
⠋ Converting... [████████████░░░░░░░░] 60% (page 25/42)
✓ Converted successfully in 3.2s
  → report.md (48 KB, 12,340 words)
```

### 5. 🔗 Composability
**Play well with other tools.**
```bash
# Pipe from stdin
curl https://example.com/doc.pdf | docling convert - -f json | jq '.title'

# Pipe to stdout
docling convert file.pdf | head -100

# Chain with other tools
find . -name "*.pdf" | xargs docling batch -o converted/
```

---

## Proposed Command Structure

```
docling
├── convert      Convert a single document (default command)
├── batch        Convert multiple documents efficiently
├── benchmark    Measure conversion performance
├── info         Inspect document metadata and structure
├── formats      List supported formats and features
├── config       Manage configuration
│   ├── init     Create .docling.toml with defaults
│   ├── show     Display current configuration
│   └── set      Set a configuration value
├── completion   Generate shell completions
└── help         Show help for any command
```

---

## Detailed Command Designs

### `docling convert` - The Star of the Show

```
USAGE:
    docling convert [OPTIONS] <INPUT> [OUTPUT]
    docling convert [OPTIONS] -        # Read from stdin
    command | docling convert -        # Pipe mode

ARGUMENTS:
    <INPUT>     Input file path, or '-' for stdin
    [OUTPUT]    Output file path (default: smart naming based on input)

COMMON OPTIONS:
    -f, --format <FMT>     Output format: markdown (default), json, yaml, html
    -o, --output <PATH>    Explicit output path (overrides smart naming)
    --ocr                  Enable OCR for scanned documents
    --quiet, -q            Suppress progress output
    --verbose, -v          Show detailed processing information

SAFETY OPTIONS:
    --force                Overwrite existing output files
    --no-clobber           Never overwrite (error if exists)
    --dry-run              Show what would be converted without doing it
    --backup               Create .bak before overwriting

PDF OPTIONS:
    --pages <RANGE>        Page range: "1-10", "1,3,5", "1-5,10-15"
    --max-pages <N>        Maximum pages to process
    --no-tables            Skip table structure recognition
    --device <DEV>         ML device: auto, cpu, cuda, mps
    --model-size <SIZE>    Model: fast, standard, accurate

OUTPUT CONTROL:
    --compact              Compact JSON (no pretty-printing)
    --no-color             Disable colored output
    --json-errors          Output errors as JSON (for scripting)

EXAMPLES:
    # Simple conversion (creates report.md)
    docling convert report.pdf

    # Convert to JSON with OCR
    docling convert scanned.pdf -f json --ocr

    # Convert specific pages
    docling convert thesis.pdf --pages 1-10 -o intro.md

    # Pipe from curl
    curl -s https://arxiv.org/pdf/2301.00001 | docling convert - -f json

    # Preview without converting
    docling convert *.pdf --dry-run
```

### `docling batch` - Efficient Bulk Processing

```
USAGE:
    docling batch [OPTIONS] <INPUTS>... -o <OUTPUT_DIR>
    find . -name "*.pdf" | docling batch -o out/ --stdin

ARGUMENTS:
    <INPUTS>...    Files or glob patterns to convert

OPTIONS:
    -o, --output <DIR>      Output directory (required)
    -f, --format <FMT>      Output format (default: markdown)
    --continue-on-error     Don't stop on first error
    --parallel <N>          Parallel workers (default: CPU cores)
    --max-file-size <SIZE>  Skip files larger than SIZE (e.g., 100M)
    --stdin                 Read file list from stdin
    --progress              Show progress bar (default: auto)
    --summary               Print summary at end

FILTERING:
    --include <GLOB>        Only process matching files
    --exclude <GLOB>        Skip matching files
    --newer-than <DATE>     Only files modified after DATE
    --older-than <DATE>     Only files modified before DATE

EXAMPLES:
    # Convert all PDFs in directory
    docling batch documents/*.pdf -o converted/

    # Convert with error recovery and summary
    docling batch *.pdf *.docx -o out/ --continue-on-error --summary

    # Process files from find
    find . -name "*.pdf" -mtime -7 | docling batch -o weekly/ --stdin

    # Parallel processing with 8 workers
    docling batch large_corpus/*.pdf -o out/ --parallel 8
```

### `docling info` - Document Inspector (NEW)

```
USAGE:
    docling info [OPTIONS] <INPUT>

DESCRIPTION:
    Inspect document metadata, structure, and conversion preview without
    performing full conversion. Useful for understanding documents before
    processing.

OPTIONS:
    -f, --format <FMT>    Output format: text (default), json, yaml
    --deep                Deep analysis (slower, more details)
    --structure           Show document structure tree
    --text-preview <N>    Show first N characters of text

OUTPUT INCLUDES:
    • File format and version
    • Page count, dimensions
    • Embedded fonts, images
    • Table count and locations
    • Language detection
    • OCR recommendation
    • Estimated conversion time

EXAMPLES:
    # Quick document overview
    docling info report.pdf

    # Detailed structure analysis
    docling info thesis.pdf --deep --structure

    # Machine-readable output
    docling info document.pdf -f json | jq '.page_count'
```

**Example Output:**
```
$ docling info thesis.pdf

📄 thesis.pdf
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  Format:      PDF 1.7
  Pages:       142
  Size:        8.4 MB
  Dimensions:  8.5" × 11" (Letter)

  Content:
    • Text:    98% (machine-readable)
    • Images:  23 (4 full-page figures)
    • Tables:  12 (detected)
    • Fonts:   Times New Roman, Arial, Symbol

  Analysis:
    ✓ OCR not needed (text is extractable)
    ✓ Tables detected (will be converted to markdown)
    ⚠ Some images contain text (consider --ocr)

  Estimate:
    Conversion time: ~8 seconds
    Output size:     ~450 KB (markdown)
```

### `docling formats` - Format Discovery (NEW)

```
USAGE:
    docling formats [OPTIONS] [FILTER]

DESCRIPTION:
    List all supported input formats with their capabilities.

OPTIONS:
    -f, --format <FMT>    Output format: table (default), json, list
    --features            Show feature support matrix
    --examples            Show example files for each format

EXAMPLES:
    # List all formats
    docling formats

    # Show only PDF-related formats
    docling formats pdf

    # Machine-readable format list
    docling formats -f json

    # Feature matrix
    docling formats --features
```

**Example Output:**
```
$ docling formats

Supported Formats (55 total)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Documents
  pdf      PDF documents (with ML table/layout detection)
  docx     Microsoft Word 2007+
  doc      Microsoft Word 97-2003
  odt      OpenDocument Text
  rtf      Rich Text Format
  pages    Apple Pages

Spreadsheets
  xlsx     Microsoft Excel 2007+
  xls      Microsoft Excel 97-2003
  ods      OpenDocument Spreadsheet
  csv      Comma-Separated Values

Presentations
  pptx     Microsoft PowerPoint 2007+
  ppt      Microsoft PowerPoint 97-2003
  odp      OpenDocument Presentation

Web & Markup
  html     HTML documents
  md       Markdown
  rst      reStructuredText
  asciidoc AsciiDoc
  tex      LaTeX

... (continues)

Use 'docling formats --features' for capability matrix
Use 'docling formats <name>' for format details
```

### `docling config` - Configuration Management (NEW)

```
USAGE:
    docling config <COMMAND>

COMMANDS:
    init     Create .docling.toml with sensible defaults
    show     Display current effective configuration
    set      Set a configuration value
    get      Get a configuration value
    reset    Reset to defaults

EXAMPLES:
    # Initialize config in current directory
    docling config init

    # Show all settings
    docling config show

    # Set default output format
    docling config set output.format json

    # Enable OCR by default
    docling config set pdf.ocr true
```

---

## UX Enhancements

### 1. Smart Output Naming

```rust
fn smart_output_path(input: &Path, format: OutputFormat) -> PathBuf {
    let stem = input.file_stem().unwrap();
    let ext = match format {
        OutputFormat::Markdown => "md",
        OutputFormat::Json => "json",
        OutputFormat::Yaml => "yaml",
        OutputFormat::Html => "html",
    };
    input.with_file_name(format!("{}.{}", stem.to_string_lossy(), ext))
}
```

### 2. Progress Display Styles

```
# Default (interactive terminal)
⠋ Converting report.pdf... [████████████░░░░░░░░] 60% (page 25/42)

# Quiet mode (-q)
(no output until complete)

# Verbose mode (-v)
[00:00.000] Loading document: report.pdf (2.3 MB)
[00:00.123] Detected format: PDF 1.7
[00:00.145] Initializing ML models...
[00:00.892] Processing page 1/42...
[00:01.234] Processing page 2/42...
...

# Non-interactive (piped)
Converting report.pdf... done (3.2s)
```

### 3. Helpful Error Messages

```
$ docling convert missing.pdf
Error: File not found: missing.pdf

  Did you mean one of these?
    • ./documents/missing.pdf
    • ./archive/missing_v2.pdf

  Hint: Use 'docling batch *.pdf' to convert multiple files

$ docling convert huge.pdf
Error: File too large for memory (4.2 GB)

  Suggestions:
    • Use --max-pages to process a subset
    • Use --streaming mode (coming soon)
    • Split the PDF with 'pdftk' first

$ docling convert scan.pdf
Warning: Document appears to be scanned (image-based PDF)

  OCR is recommended for best results:
    docling convert scan.pdf --ocr

  Continue without OCR? [y/N]
```

### 4. Completion Statistics

```
$ docling batch docs/*.pdf -o out/ --summary

Batch Conversion Complete
━━━━━━━━━━━━━━━━━━━━━━━━━
  Total files:    47
  Succeeded:      45 ✓
  Failed:         2 ✗
  Skipped:        0

  Time:           2m 34s
  Throughput:     0.31 files/sec
  Total input:    234 MB
  Total output:   12 MB

  Failures:
    • corrupt.pdf: Invalid PDF structure
    • encrypted.pdf: Password protected

  Output: ./out/ (45 files)
```

---

## Implementation Phases

### Phase 1: Foundation (Priority: HIGH) - ✅ COMPLETE
- [x] Smart output naming (auto-generate output path) - N=2678
- [x] `--dry-run` flag - N=2680
- [x] `--force` / `--no-clobber` flags - N=2679
- [x] `--max-pages` flag - N=2677
- [x] Stdin support (`docling convert -`) - N=2681

### Phase 2: Discovery (Priority: MEDIUM) - ✅ COMPLETE
- [x] `docling formats` command - N=2682
- [x] `docling info` command - N=2683
- [x] `--quiet` / `--verbose` flags - N=2684, N=2686
- [ ] Better error messages with suggestions (partial - key errors have hints)

### Phase 3: Power Features (Priority: LOW) - ✅ COMPLETE
- [x] `docling config` command - N=2687
- [x] `--parallel` for batch - N=2689
- [x] `--watch` mode - N=2690
- [x] `--profile` presets - N=2691
- [x] Stdin file list for batch (`--stdin`) - N=2688

---

## Migration Guide

### Breaking Changes: None
All new features are additive. Existing commands continue to work.

### Deprecations: None
No flags or behaviors are deprecated.

### New Defaults
| Behavior | Old | New |
|----------|-----|-----|
| Output naming | Required explicit -o | Smart default |
| Overwrite | Silent overwrite | Error without --force |
| Progress | Always show | Auto-detect TTY |

---

## Success Metrics

1. **Discoverability**: Users find features without reading docs
2. **Error Recovery**: Clear guidance when things go wrong
3. **Efficiency**: Common tasks require minimal typing
4. **Delight**: Users say "that's nice!" when using the CLI
5. **Composability**: Works seamlessly in shell pipelines

---

## Appendix: Comparison with Competitors

### vs. Pandoc
| Feature | Docling | Pandoc |
|---------|---------|--------|
| Auto format detection | ✅ | ❌ (must specify) |
| Progress bars | ✅ | ❌ |
| ML-powered extraction | ✅ | ❌ |
| Config file | ✅ | ❌ |
| Table detection | ✅ ML | ❌ |
| 55+ formats | ✅ | ✅ (different set) |

### vs. Apache Tika
| Feature | Docling | Tika |
|---------|---------|------|
| Pure Rust/native | ✅ | ❌ (JVM) |
| Startup time | <100ms | 2-5s |
| Memory usage | Low | High |
| ML models | ✅ | Limited |

---

**This design will make docling the most user-friendly document conversion tool available.**
