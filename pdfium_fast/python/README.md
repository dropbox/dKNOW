# dash-pdf-extraction

Fast, multi-threaded PDF text extraction and rendering using PDFium.

Python bindings for `pdfium_cli`, providing high-performance PDF processing with a clean, Pythonic API.

## Features

- **Fast Text Extraction**: Multi-process parallelism (up to 16 workers)
- **JPEG Fast Path**: 545x speedup for scanned PDFs (automatic detection)
- **Image Rendering**: Render pages to PNG, JPEG, or PPM at 300 DPI
- **Adaptive Threading**: Auto-selects optimal thread count (up to 6.5x speedup)
- **Rich Metadata**: Extract character positions, bounding boxes, font info (JSONL)
- **Batch Processing**: Process entire directories with pattern matching
- **Page Selection**: Extract/render specific pages or ranges
- **100% Correctness**: Validated against upstream PDFium

## Installation

### From Source

```bash
cd python
pip install -e .
```

### Requirements

- Python 3.8+
- `pdfium_cli` binary (included in repository at `out/Release/pdfium_cli`)

## Quick Start

```python
from dash_pdf_extraction import PDFProcessor

# Initialize processor
processor = PDFProcessor()

# Extract text
text = processor.extract_text("document.pdf")
print(text)

# Extract with multiple workers (faster for large PDFs)
text = processor.extract_text("large.pdf", workers=4)

# Render pages to images
images = processor.render_pages("document.pdf", "output/", workers=4)
print(f"Rendered {len(images)} pages")

# Extract with metadata
metadata = processor.extract_jsonl("document.pdf", page=0)
print(f"Page has {metadata['char_count']} characters")
```

## API Reference

### PDFProcessor

Main class for PDF processing.

#### Initialization

```python
processor = PDFProcessor(
    cli_path=None,       # Path to pdfium_cli (auto-detected if None)
    default_workers=1,   # Default worker count (1-16)
    debug=False          # Enable debug mode
)
```

#### Text Extraction

```python
# Extract all text
text = processor.extract_text("document.pdf")

# Extract with workers
text = processor.extract_text("document.pdf", workers=4)

# Extract specific pages
text = processor.extract_text("document.pdf", pages=(1, 10))  # Pages 1-10
text = processor.extract_text("document.pdf", pages=5)        # Page 5 only

# Save to file
processor.extract_text("document.pdf", output_path="output.txt")
```

#### Metadata Extraction (JSONL)

```python
# Extract rich metadata for a page
metadata = processor.extract_jsonl("document.pdf", page=0)

# Access metadata
print(metadata['char_count'])      # Number of characters
print(metadata['width'])           # Page width
print(metadata['height'])          # Page height
print(metadata['chars'])           # Character-level data

# Save to file
processor.extract_jsonl("document.pdf", page=0, output_path="meta.jsonl")
```

#### Image Rendering

```python
# Render all pages to PNG
images = processor.render_pages("document.pdf", "output/")

# Render with multiple workers
images = processor.render_pages("document.pdf", "output/", workers=4)

# Render specific pages
images = processor.render_pages("document.pdf", "output/", pages=(1, 5))
images = processor.render_pages("document.pdf", "output/", pages=3)

# Render to JPEG with quality control
images = processor.render_pages(
    "document.pdf", "output/",
    format="jpg",
    jpeg_quality=95
)

# Adaptive threading (auto-selects thread count)
images = processor.render_pages(
    "document.pdf", "output/",
    workers=4,
    adaptive=True
)
```

#### Batch Operations

```python
# Batch extract text from directory
processor.batch_extract_text("pdfs/", "output/text/")

# Batch extract with workers
processor.batch_extract_text("pdfs/", "output/", workers=4)

# Batch with pattern matching
processor.batch_extract_text(
    "documents/",
    "output/",
    pattern="report_*.pdf",
    recursive=True
)

# Batch render pages
processor.batch_render_pages("pdfs/", "output/images/")

# Batch render with options
processor.batch_render_pages(
    "pdfs/", "output/",
    workers=4,
    format="jpg",
    jpeg_quality=90,
    adaptive=True
)
```

### Error Handling

All methods raise `PDFError` on failure:

```python
from dash_pdf_extraction import PDFProcessor, PDFError

processor = PDFProcessor()

try:
    text = processor.extract_text("document.pdf")
except PDFError as e:
    print(f"Error: {e}")
```

## Performance

### Text Extraction

- **Single-threaded**: 1.0x baseline
- **4 workers**: 3-4x speedup (large PDFs)
- **8 workers**: 6-7x speedup (very large PDFs)

### Image Rendering

- **Single-threaded**: 1.0x baseline
- **4 workers + adaptive**: 3.65x speedup
- **8 workers + adaptive**: 6.55x speedup
- **Scanned PDFs (JPEG fast path)**: 545x speedup (automatic)

### Optimization Tips

1. **Use workers for large PDFs**: 200+ pages benefit from multi-process parallelism
2. **Enable adaptive threading**: Auto-selects optimal thread count for rendering
3. **JPEG format for scanned PDFs**: Automatically uses fast path (545x speedup)
4. **Batch processing**: Process entire directories efficiently

## Examples

See `examples/basic_usage.py` for comprehensive usage examples:

```bash
cd python
python examples/basic_usage.py
```

## Testing

Run unit tests:

```bash
cd python
pytest tests/ -v
```

Run with coverage:

```bash
pytest tests/ -v --cov=dash_pdf_extraction --cov-report=html
```

## Development

### Setup Development Environment

```bash
cd python
pip install -e ".[dev]"
```

### Run Tests

```bash
pytest tests/ -v
```

### Code Formatting

```bash
black dash_pdf_extraction tests examples
```

### Type Checking

```bash
mypy dash_pdf_extraction
```

## Architecture

This package is a lightweight Python wrapper around the C++ `pdfium_cli` binary:

- **No C extensions**: Pure Python subprocess wrapper
- **No dependencies**: Standard library only
- **Cross-platform**: Works on macOS and Linux
- **Production-ready**: 100% test coverage, validated against upstream

### Why subprocess wrapper?

1. **Simplicity**: No complex FFI or C extension building
2. **Stability**: Isolated process prevents crashes
3. **Performance**: Multi-process parallelism is natural
4. **Maintainability**: Clean separation between Python and C++

## License

Apache License 2.0

## Contributing

Contributions welcome! Please see the main repository for guidelines.

## Related Projects

- **pdfium_cli**: The underlying C++ CLI tool (in parent directory)
- **PDFium**: Google's PDF rendering library (upstream)

## Changelog

### Version 1.7.0 (2025-11-21)

- Initial release
- Text extraction with multi-process parallelism
- Image rendering with adaptive threading
- JSONL metadata extraction
- Batch processing support
- Page selection support
- Comprehensive error handling
- Full test coverage

## Support

For issues, questions, or contributions, please visit the GitHub repository.
