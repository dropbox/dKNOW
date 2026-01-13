# Dash PDF Extraction v1.6.0 - macOS ARM64 Binaries

**Platform:** macOS 12+ (Apple Silicon M1/M2/M3/M4)
**Released:** 2025-11-20
**Build:** Release (optimized)

## Quick Start

```bash
# 1. Download and extract
curl -L https://github.com/dropbox/dKNOW/pdfium_fast/releases/download/v1.6.0/macos-arm64.tar.gz | tar xz
cd macos-arm64

# 2. Make executable
chmod +x pdfium_cli

# 3. Run
./pdfium_cli extract-text document.pdf output.txt
```

## Included Files

- **pdfium_cli** (5.2 MB) - Main CLI tool
- **libpdfium.dylib** (5.0 MB) - Core PDFium library
- **libpdfium_render_bridge.dylib** (4.7 MB) - Rust bindings bridge
- **SHA256SUMS.txt** - Checksums for verification

## Verify Integrity

```bash
shasum -a 256 -c SHA256SUMS.txt
```

All checksums should show "OK".

## Usage Examples

### Text Extraction
```bash
# Single-threaded (default)
./pdfium_cli extract-text document.pdf output.txt

# Multi-threaded (8 threads)
./pdfium_cli --threads 8 extract-text document.pdf output.txt
```

### Image Rendering
```bash
# Render at 300 DPI with 8 threads
./pdfium_cli --threads 8 render-pages document.pdf images/

# Progress reporting (automatic on terminal)
./pdfium_cli --threads 8 render-pages large.pdf images/
# Output:
# [████████████████████] 100% | 277 pages/sec | ETA: 0s
```

### JSONL Metadata
```bash
# Extract with character positions and font metadata
./pdfium_cli extract-jsonl document.pdf output.jsonl
```

### Batch Processing
```bash
# Process entire directory
./pdfium_cli --batch --workers 4 render-pages pdfs/ images/

# With pattern matching
./pdfium_cli --batch --pattern "*.pdf" --recursive render-pages docs/ output/
```

## Performance

- **72x faster** image rendering (vs upstream PDFium)
- **545x faster** for JPEG scanned PDFs
- **3x faster** text extraction
- **100% correctness** (byte-for-byte validated)

## System Requirements

- **OS:** macOS 12.0 or later
- **CPU:** Apple Silicon (M1/M2/M3/M4)
- **RAM:** 4GB minimum, 8GB recommended
- **Disk:** 15 MB for binaries

## Troubleshooting

### "Cannot execute binary"
```bash
# Remove quarantine attribute (macOS security)
xattr -d com.apple.quarantine pdfium_cli
xattr -d com.apple.quarantine *.dylib
```

### "Library not loaded"
```bash
# Set library path
export DYLD_LIBRARY_PATH=$(pwd)
./pdfium_cli extract-text document.pdf output.txt
```

### "Permission denied"
```bash
# Make executable
chmod +x pdfium_cli
```

## Documentation

- **Full Documentation:** https://github.com/dropbox/dKNOW/pdfium_fast
- **Performance Guide:** See PERFORMANCE_GUIDE.md in repository
- **API Reference:** Run `./pdfium_cli --help`

## License

Copyright © 2025 Andrew Yates. All rights reserved.

Based on PDFium (https://pdfium.googlesource.com/pdfium/)
- PDFium License: BSD-3-Clause
- Chromium License: BSD-3-Clause

## Support

- **Issues:** https://github.com/dropbox/dKNOW/pdfium_fast/issues
- **Discussions:** https://github.com/dropbox/dKNOW/pdfium_fast/discussions

## Version History

- **v1.6.0** (2025-11-20): UX features + 100% test pass rate
- **v1.5.0** (2025-11-19): Documentation complete
- **v1.4.0** (2025-11-18): Quality flags
- **v1.0.0** (2025-11-08): Initial release
