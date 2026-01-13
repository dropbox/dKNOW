# Test PDF Corpus - Download Instructions

The test PDF corpus (1.4GB compressed, 1.5GB uncompressed, 462 PDFs) is NOT included in git to keep repository size manageable.

## Download from GitHub Releases (Recommended)

**Automated Download:**
```bash
cd integration_tests
python3 download_test_pdfs.py
```

This script will:
1. Download `pdfium_test_pdfs.tar.gz` from GitHub Releases
2. Extract to `integration_tests/pdfs/`
3. Verify the extraction

**Manual Download:**
1. Visit: https://github.com/ayates_dbx/pdfium_fast/releases
2. Find the `test-pdfs-v1` release
3. Download `pdfium_test_pdfs.tar.gz` (1.4GB)
4. Extract in `integration_tests/`:
   ```bash
   cd integration_tests
   tar xzf pdfium_test_pdfs.tar.gz
   ```

## What's Included

The test corpus includes:

### Benchmark PDFs (40 PDFs)
- ArXiv academic papers
- Web documents
- EDINET Japanese corporate filings
- Common Crawl web PDFs
- Large multi-page documents for performance testing

### Edge Cases (286 PDFs)
From upstream PDFium test suite:
- Unicode (Arabic RTL, CJK, emoji)
- Complex layouts (forms, annotations, transparency)
- Malformed/corrupted PDFs
- Various compression methods
- Unusual page sizes and rotations

### Scanned PDFs (6 PDFs)
- Real scanned documents for JPEG fast path testing
- CCITT fax compression samples

## Validate Downloads

After downloading, verify with:

```bash
cd integration_tests
python3 << 'EOF'
from pathlib import Path

pdfs_dir = Path('pdfs')
if not pdfs_dir.exists():
    print("❌ pdfs/ directory not found")
    exit(1)

pdf_count = len(list(pdfs_dir.rglob('*.pdf')))
print(f"✅ Found {pdf_count} PDFs")

# Check key directories exist
required_dirs = ['benchmark', 'edge_cases', 'scanned_test']
for d in required_dirs:
    if not (pdfs_dir / d).exists():
        print(f"⚠️  Missing directory: pdfs/{d}")
    else:
        count = len(list((pdfs_dir / d).rglob('*.pdf')))
        print(f"  {d}: {count} PDFs")
EOF
```

Expected output:
```
✅ Found 462 PDFs
  benchmark: 40 PDFs
  edge_cases: 286 PDFs
  scanned_test: 6 PDFs
```

## Disk Space Requirements

- **Download**: 1.4GB (compressed)
- **Extracted**: 1.5GB
- **Total needed**: 3GB during download, 1.5GB after

## Running Tests

Once PDFs are downloaded:

```bash
# Quick smoke tests (uses 5-6 PDFs, ~30 seconds)
pytest -m smoke

# Full test suite (uses all 462 PDFs, ~20 minutes)
pytest -m full

# Extended tests (comprehensive, ~2 hours)
pytest -m extended
```

## Troubleshooting

### "No PDFs found" Error

If tests fail with "No PDFs found in /path/to/pdfs/benchmark":

1. Check PDFs exist:
   ```bash
   ls integration_tests/pdfs/benchmark/*.pdf
   ```

2. If missing, run download script:
   ```bash
   cd integration_tests
   python3 download_test_pdfs.py
   ```

### Download Fails

If automated download fails:
1. Download manually from GitHub Releases (see above)
2. Ensure you have access to the repository (if private)
3. Check network/firewall settings

### Wrong PDF Count

If you have PDFs but tests still skip:
- Tests expect specific PDFs (e.g., `arxiv_001.pdf`, `cc_008_116p.pdf`)
- Ensure you downloaded the official test corpus, not individual PDFs
- Check `pdfs/benchmark/` contains the expected files

## Alternative: Minimal Test Set

For quick validation without the full corpus, you can use edge case PDFs already in the repository:

```bash
# Tests will use edge_cases/ PDFs (266 PDFs included in repo)
pytest -m smoke --pdf-source edge_cases
```

Note: This won't test performance or large document handling.

## Test PDF Contents

All test PDFs are from:
- **Public sources**: ArXiv, Common Crawl, upstream PDFium
- **Licensed content**: EDINET filings (publicly available)
- **Generated**: Synthetic PDFs for specific test cases

No proprietary or confidential content is included.
