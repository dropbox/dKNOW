# Test Infrastructure - Final Implementation Plan

**Date**: 2025-10-31
**Status**: APPROVED - Ready for implementation
**Based on**: Q_and_A.md user responses

---

## Design Decisions Summary

| Decision | Choice | Rationale |
|----------|--------|-----------|
| **Scope** | All 452 PDFs | Comprehensive coverage |
| **Test count** | 1,356 static tests (452 × 3) | Text, JSONL, Image per PDF |
| **Test generation** | Generated + committed | LLM readability > git noise |
| **Organization** | Hierarchical (match PDF dirs) | Easier navigation |
| **Per-page text** | Yes, committed to git | Fast debugging (~50MB total) |
| **Per-page JSONL** | Only page 0, metadata only | Too large for all pages |
| **Images** | PNG + JPG, metadata only | ~100KB metadata vs ~20GB images |
| **Image comparison** | MD5 first, SSIM fallback | Fast + tolerant |
| **Markers** | Category + size + special sets | Flexible test selection |
| **Telemetry** | Pytest hooks | Standard pattern |
| **Config** | Hybrid (fixed + parametrized) | Deterministic correctness |
| **Smoke test** | 10 PDFs, < 1 minute | Git commit trigger |

---

## File Structure

```
integration_tests/
  master_test_suite/
    pdf_manifest.csv                          # 452 PDFs × 25 columns
    expected_outputs/
      arxiv/
        arxiv_001/
          manifest.json                       # Per-PDF manifest
          text/
            page_0000.txt                     # ✅ Committed
            page_0001.txt                     # ✅ Committed
            ...
            full.txt                          # ✅ Committed (concatenated)
          jsonl/
            page_0000.jsonl                   # ✅ Committed (page 0 only)
            page_0000.jsonl.md5               # ✅ Committed
          images/
            page_0000.png.md5                 # ✅ Committed (metadata)
            page_0000.jpg.md5                 # ✅ Committed (metadata)
            # Actual images NOT committed
        arxiv_005/
          ...
      cc/
        cc_001/
          ...
      edge_cases/
        344775293/
          ...
      (452 directories total)

  tests/
    test_000_infrastructure/
      test_pdf_manifest.py                    # PDF manifest integrity
      test_expected_outputs.py                # Expected output integrity
      test_baseline_generation.py             # Baseline reproducibility

    pdfs/
      arxiv/
        test_arxiv_001.py                     # 3 tests
        test_arxiv_005.py                     # 3 tests
        ...
      cc/
        test_cc_001.py                        # 3 tests
        ...
      edge_cases/
        test_344775293.py                     # 3 tests
        ...
      benchmark/
        test_0100pages_7FKQLKX273JBHXAAW5XDRT27JGMIZMCI.py
        ...
      (452 files total)

    test_batch_bulk.py                        # 4 batch tests

  lib/
    generate_expected_outputs.py              # Baseline generation
    generate_test_files.py                    # Test file generator
    manifest_generator.py                     # Manifest system
    baseline_generator.py                     # (existing)

  conftest.py                                 # Fixtures + hooks
  pytest.ini                                  # Markers
```

---

## Implementation Steps

### Phase 1: Manifest System (Updated)

**Update lib/manifest_generator.py**:

```python
class PerPDFManifest:
    """Per-PDF manifest with page-level metadata"""

    def generate(self, pdf_path, baseline_binary):
        manifest = {
            "pdf": pdf_path.name,
            "generated_by": "baseline_pdfium",
            "binary_md5": BASELINE_BINARY_MD5,
            "generated_date": datetime.now().isoformat(),
            "pages": page_count,

            "text": {
                "full": {
                    "path": "text/full.txt",
                    "md5": compute_md5(...),
                    "bytes": len(full_text),
                    "chars": count_chars(full_text)
                },
                "pages": [
                    {
                        "page": 0,
                        "path": "text/page_0000.txt",
                        "md5": "...",
                        "bytes": 1200,
                        "chars": 1150
                    },
                    # ... all pages
                ]
            },

            "jsonl": {
                "note": "Only page 0 generated (character-level is 200x larger)",
                "pages": [
                    {
                        "page": 0,
                        "path": "jsonl/page_0000.jsonl",
                        "md5": "...",
                        "bytes": 45000,
                        "lines": 1150,
                        "char_count": 1150
                    }
                ]
            },

            "images": {
                "formats": ["png", "jpg"],
                "dpi": 300,
                "pages": [
                    {
                        "page": 0,
                        "png": {
                            "path": "images/page_0000.png",
                            "md5": "...",
                            "bytes": 145000,
                            "width_px": 2550,
                            "height_px": 3300
                        },
                        "jpg": {
                            "path": "images/page_0000.jpg",
                            "md5": "...",
                            "bytes": 42000,
                            "quality": 85,
                            "width_px": 2550,
                            "height_px": 3300
                        }
                    },
                    # ... all pages (metadata only, not files)
                ]
            }
        }
```

**Main PDF Manifest** (csv):
```csv
pdf_name,pdf_path,pdf_md5,pdf_bytes,pdf_pages,pdf_category,pdf_size_class,
expected_outputs_dir,manifest_json_path,markers
arxiv_001.pdf,pdfs/benchmark/arxiv_001.pdf,8c081c...,930690,10,arxiv,small,
expected_outputs/arxiv/arxiv_001,expected_outputs/arxiv/arxiv_001/manifest.json,
"standard_60_set,smoke_fast,arxiv,small_pdf"
```

**Markers column**: Comma-separated list of markers for this PDF

---

### Phase 2: Expected Output Generation

**lib/generate_expected_outputs.py**:

```python
def generate_all_outputs(pdf_path, baseline_binary, output_dir):
    """Generate text (per-page), JSONL (page 0), images (PNG+JPG metadata)"""

    pdf_stem = pdf_path.stem
    pdf_output_dir = output_dir / pdf_stem

    # 1. Generate per-page text
    text_dir = pdf_output_dir / 'text'
    text_dir.mkdir(parents=True, exist_ok=True)

    text_pages = generate_text_per_page(pdf_path, baseline_binary)
    for page_num, page_text in enumerate(text_pages):
        page_file = text_dir / f'page_{page_num:04d}.txt'
        page_file.write_bytes(page_text)

    # Save full text
    full_text = b''.join(text_pages)
    (text_dir / 'full.txt').write_bytes(full_text)

    # 2. Generate JSONL for page 0 only
    jsonl_dir = pdf_output_dir / 'jsonl'
    jsonl_dir.mkdir(parents=True, exist_ok=True)

    page_0_jsonl = generate_jsonl_with_metadata(pdf_path, baseline_binary, page=0)
    jsonl_file = jsonl_dir / 'page_0000.jsonl'
    jsonl_file.write_text(page_0_jsonl)

    # 3. Generate images (PNG + JPG) - save metadata only
    images_dir = pdf_output_dir / 'images'
    images_dir.mkdir(parents=True, exist_ok=True)

    image_metadata = []
    for page_num in range(page_count):
        # Render PNG
        png_data = render_page_png(pdf_path, page_num, baseline_binary, dpi=300)
        png_file = images_dir / f'page_{page_num:04d}.png'
        png_file.write_bytes(png_data)  # Temp (for MD5)

        # Render JPG
        jpg_data = render_page_jpg(pdf_path, page_num, baseline_binary, dpi=300, quality=85)
        jpg_file = images_dir / f'page_{page_num:04d}.jpg'
        jpg_file.write_bytes(jpg_data)  # Temp (for MD5)

        # Get image dimensions
        from PIL import Image
        img = Image.open(png_file)
        width_px, height_px = img.size

        # Save metadata
        page_meta = {
            "page": page_num,
            "png": {
                "path": f"images/page_{page_num:04d}.png",
                "md5": compute_md5(png_file),
                "bytes": len(png_data),
                "width_px": width_px,
                "height_px": height_px
            },
            "jpg": {
                "path": f"images/page_{page_num:04d}.jpg",
                "md5": compute_md5(jpg_file),
                "bytes": len(jpg_data),
                "quality": 85,
                "width_px": width_px,
                "height_px": height_px
            }
        }
        image_metadata.append(page_meta)

        # Delete images (keep metadata only)
        png_file.unlink()
        jpg_file.unlink()

    # 4. Write manifest.json
    manifest = build_manifest(pdf_path, text_pages, page_0_jsonl, image_metadata)
    (pdf_output_dir / 'manifest.json').write_text(json.dumps(manifest, indent=2))

    return manifest
```

**What gets committed**:
- ✅ manifest.json (all metadata)
- ✅ text/page_NNNN.txt (all pages)
- ✅ text/full.txt
- ✅ jsonl/page_0000.jsonl (page 0 only)
- ✅ jsonl/page_0000.jsonl.md5
- ❌ images/*.png (NOT committed - regenerate on demand)
- ❌ images/*.jpg (NOT committed - regenerate on demand)

**.gitignore**:
```
expected_outputs/**/images/*.png
expected_outputs/**/images/*.jpg
!expected_outputs/**/manifest.json
```

---

### Phase 3: Test File Generation

**lib/generate_test_files.py**:

```python
#!/usr/bin/env python3
"""
Generate 1,356 static test files for 452 PDFs.

Output: tests/pdfs/<category>/test_<pdf_stem>.py (452 files)
Each file: 3 test functions (text, jsonl, image)
Total: 1,356 static test functions

Run: python lib/generate_test_files.py
"""

import csv
from pathlib import Path

TEMPLATE = '''"""
Tests for {pdf_name}

Generated by: lib/generate_test_files.py
DO NOT EDIT - Regenerate with: python lib/generate_test_files.py

PDF: {pdf_name}
Category: {category}
Size: {size_class} ({pages} pages)
Markers: {markers}
"""

import pytest
from pathlib import Path
import tempfile
import subprocess

PDF_NAME = "{pdf_name}"
PDF_STEM = "{pdf_stem}"
EXPECTED_DIR = Path(__file__).parent.parent.parent / "master_test_suite" / "expected_outputs" / "{category}" / "{pdf_stem}"


{markers_decorators}
def test_text_extraction_{test_name}(benchmark_pdfs, test_binary, expected_outputs):
    """Text extraction correctness for {pdf_name}"""
    pdf_path = benchmark_pdfs / "{rel_path}"
    expected_manifest = (EXPECTED_DIR / "manifest.json").read_text()

    # Act: Extract text
    result = subprocess.run(
        [str(test_binary), "--txt", str(pdf_path)],
        capture_output=True,
        timeout=600
    )
    assert result.returncode == 0, f"Extraction failed: {{result.stderr}}"

    # Assert: Compare per-page
    manifest = json.loads(expected_manifest)
    for page_meta in manifest["text"]["pages"]:
        expected_page = (EXPECTED_DIR / page_meta["path"]).read_bytes()
        actual_page = extract_page_from_result(result.stdout, page_meta["page"])
        assert actual_page == expected_page, f"Page {{page_meta['page']}} mismatch"


{markers_decorators}
def test_jsonl_extraction_{test_name}(benchmark_pdfs, test_binary, expected_outputs):
    """JSONL extraction for {pdf_name} (page 0 only)"""
    pdf_path = benchmark_pdfs / "{rel_path}"
    expected_manifest = (EXPECTED_DIR / "manifest.json").read_text()
    manifest = json.loads(expected_manifest)

    # Act: Extract JSONL for page 0
    result = subprocess.run(
        [str(test_binary), "--jsonl", "--page=0", str(pdf_path)],
        capture_output=True,
        timeout=600
    )
    assert result.returncode == 0, f"JSONL extraction failed"

    # Assert: Compare with expected
    expected_jsonl = (EXPECTED_DIR / manifest["jsonl"]["pages"][0]["path"]).read_bytes()
    assert result.stdout == expected_jsonl, "JSONL mismatch on page 0"


{markers_decorators}
def test_image_rendering_{test_name}(benchmark_pdfs, test_binary, expected_outputs):
    """Image rendering for {pdf_name}"""
    pdf_path = benchmark_pdfs / "{rel_path}"
    expected_manifest = (EXPECTED_DIR / "manifest.json").read_text()
    manifest = json.loads(expected_manifest)

    # Act: Render all pages as PNG
    with tempfile.TemporaryDirectory() as tmpdir:
        result = subprocess.run(
            [str(test_binary), "--png", str(pdf_path), tmpdir],
            capture_output=True,
            timeout=1200
        )
        assert result.returncode == 0, f"Rendering failed"

        # Assert: Compare each page
        for page_meta in manifest["images"]["pages"]:
            actual_png = Path(tmpdir) / f"page_{{page_meta['page']:04d}}.png"

            # Strategy 1: MD5
            actual_md5 = compute_md5(actual_png)
            expected_md5 = page_meta["png"]["md5"]

            if actual_md5 == expected_md5:
                continue  # Perfect match

            # Strategy 2: SSIM (perceptual diff)
            # Only if expected image exists (can regenerate for comparison)
            expected_png = regenerate_page_image_if_needed(pdf_path, page_meta["page"])
            ssim_score = compare_ssim(expected_png, actual_png)

            assert ssim_score > 0.99, f"Page {{page_meta['page']}} SSIM={{ssim_score:.4f}}"
'''

def generate_markers(pdf_row):
    """Build marker decorators for test"""
    markers_list = pdf_row['markers'].split(',')
    decorators = '\n'.join(f'@pytest.mark.{m.strip()}' for m in markers_list)
    return decorators

def generate_test_file(pdf_row, output_base_dir):
    """Generate one test file with 3 test functions"""
    category = pdf_row['pdf_category']
    pdf_stem = Path(pdf_row['pdf_name']).stem
    test_name = pdf_stem.replace('-', '_').replace(' ', '_').replace('.', '_')

    # Determine output directory
    output_dir = output_base_dir / category
    output_dir.mkdir(parents=True, exist_ok=True)

    # Generate file content
    markers_decorators = generate_markers(pdf_row)
    content = TEMPLATE.format(
        pdf_name=pdf_row['pdf_name'],
        pdf_stem=pdf_stem,
        test_name=test_name,
        category=category,
        size_class=pdf_row['pdf_size_class'],
        pages=pdf_row['pdf_pages'],
        markers=pdf_row['markers'],
        markers_decorators=markers_decorators,
        rel_path=pdf_row['pdf_path']
    )

    # Write file
    test_file = output_dir / f'test_{test_name}.py'
    test_file.write_text(content)
    print(f"Generated {test_file.relative_to(output_base_dir.parent)}")

def main():
    manifest = Path('master_test_suite/pdf_manifest.csv')
    output_dir = Path('tests/pdfs')

    with open(manifest) as f:
        reader = csv.DictReader(f)
        for row in reader:
            generate_test_file(row, output_dir)

    print(f"\n✓ Generated 452 test files (1,356 test functions)")
```

---

### Phase 4: Markers Definition

**pytest.ini** (complete markers):

```ini
markers =
    # Test levels
    smoke: Quick smoke tests (< 30s)
    smoke_fast: Ultra-fast smoke (< 1 min, 10 PDFs, git commit trigger)
    full: Full test suite (20-30m)
    extended: Extended tests with full 450 PDF corpus (2h+)

    # Test types
    infrastructure: Infrastructure verification
    text: Text extraction tests
    jsonl: JSONL extraction tests
    image: Image rendering tests
    correctness: Correctness validation
    performance: Performance benchmarks
    scaling: Worker scaling tests
    batch_bulk: Batch processing tests

    # PDF sets
    standard_60_set: Standard 60-PDF curated corpus
    smoke_fast_set: 10 PDFs for fast smoke tests (< 1 min)

    # Categories (from filename)
    arxiv: ArXiv academic papers
    cc: Common Crawl web content
    edinet: EDINET Japanese corporate filings
    web: Web-converted documents
    pages: Page-numbered benchmark PDFs
    edge_cases: Edge case and malformed PDFs

    # Size classes (by page count)
    small_pdf: < 100 pages
    medium_pdf: 100-199 pages
    large_pdf: 200+ pages
```

---

### Phase 5: Smoke Test Selection

**10 PDFs for smoke_fast (< 1 minute)**:

Criteria:
- Small size (< 50 pages)
- Diverse categories
- Known to work
- Total test time: ~60 seconds

**Proposed smoke_fast set**:
```python
SMOKE_FAST_PDFS = [
    "arxiv_001.pdf",        # 10 pages, arxiv
    "arxiv_005.pdf",        # 12 pages, arxiv
    "cc_007_101p.pdf",      # 101 pages, cc (edge of medium)
    "cc_015_101p.pdf",      # 101 pages, cc
    "web_001.pdf",          # ~20 pages, web
    "web_007.pdf",          # ~25 pages, web
    "edinet_..._E00982.pdf", # Japanese filing
    "edinet_..._E01920.pdf", # Japanese filing
    "0100pages_...pdf",     # Exactly 100 pages
    "edge_cases/small.pdf", # Edge case
]
```

**Tests**: 10 PDFs × 3 tests = 30 tests in ~60 seconds

**Usage**:
```bash
pytest -m smoke_fast  # < 1 minute, for git commit hooks
```

---

## Implementation Checklist

### ✅ Already Completed
- [x] PDF manifest with 60 PDFs (will expand to 452)
- [x] Text baselines for 60 PDFs
- [x] Infrastructure tests (240 tests passing)
- [x] Marker system (pytest.ini)
- [x] Code path documentation

### 🔲 Phase 1: Update Manifest System (1 hour)
- [ ] Update pdf_manifest.csv with all 452 PDFs
- [ ] Add `markers` column to manifest
- [ ] Add per-page metadata structure
- [ ] Update manifest_generator.py for two-level manifests

### 🔲 Phase 2: Expected Outputs (2-3 hours)
- [ ] Implement generate_expected_outputs.py:
  - [ ] Per-page text extraction
  - [ ] Page 0 JSONL with character metadata
  - [ ] PNG + JPG rendering (metadata only)
  - [ ] Per-PDF manifest.json generation
- [ ] Run for all 452 PDFs (long-running)
- [ ] Commit manifests + text files (~60MB)

### 🔲 Phase 3: Test Generation (30 min)
- [ ] Implement lib/generate_test_files.py
- [ ] Generate 452 test files in hierarchical structure
- [ ] Commit all 1,356 test functions

### 🔲 Phase 4: Smoke Test (15 min)
- [ ] Select 10 PDFs for smoke_fast
- [ ] Update manifest with smoke_fast_set marker
- [ ] Verify: `pytest -m smoke_fast` < 1 minute

### 🔲 Phase 5: Infrastructure Tests (30 min)
- [ ] Update test_000_infrastructure.py for per-page validation
- [ ] Test per-page MD5 verification
- [ ] Test manifest integrity
- [ ] Run: `pytest -m infrastructure`

### 🔲 Phase 6: Validation (1 hour)
- [ ] Run: `pytest -m smoke_fast` (must pass, < 1 min)
- [ ] Run: `pytest -m standard_60_set` (60 PDFs × 3 = 180 tests)
- [ ] Run: `pytest -m arxiv` (test category filtering)
- [ ] Run: `pytest -m large_pdf` (test size filtering)
- [ ] Verify telemetry logs all tests

### 🔲 Phase 7: Documentation (30 min)
- [ ] Update Q_and_A.md with "RESOLVED" status
- [ ] Write implementation report
- [ ] Update CLAUDE.md with test suite instructions
- [ ] Commit final documentation

**Total Estimated Time**: 8-10 hours (includes generation of expected outputs for 452 PDFs)

---

## Git Commit Strategy

**Commit 1**: Update manifest system
- lib/manifest_generator.py
- master_test_suite/pdf_manifest.csv (452 rows)

**Commit 2**: Expected output generation script
- lib/generate_expected_outputs.py

**Commit 3**: Generate expected outputs (LARGE)
- master_test_suite/expected_outputs/**/manifest.json (452 files)
- master_test_suite/expected_outputs/**/text/*.txt (~50MB)
- master_test_suite/expected_outputs/**/jsonl/page_0000.jsonl (452 files)
- .gitignore (exclude images)

**Commit 4**: Generate test files
- tests/pdfs/**/*.py (452 files, 1,356 functions)
- lib/generate_test_files.py

**Commit 5**: Markers and smoke tests
- pytest.ini (updated markers)
- Update test files with smoke_fast_set marker

**Commit 6**: Final validation
- Test results report
- Updated documentation

---

## Usage Examples (After Implementation)

```bash
# Infrastructure check
pytest -m infrastructure                    # Verify manifests

# Fast smoke test (< 1 min, git hook)
pytest -m smoke_fast                        # 10 PDFs × 3 = 30 tests

# Standard 60-PDF set
pytest -m standard_60_set                   # 60 PDFs × 3 = 180 tests

# Category testing
pytest -m arxiv                             # All arxiv PDFs
pytest -m "arxiv and text"                  # Only text tests for arxiv
pytest -m "large_pdf and image"             # Image tests for large PDFs

# Single PDF
pytest tests/pdfs/arxiv/test_arxiv_001.py   # 3 tests

# Batch bulk
pytest -m batch_bulk                        # 4 batch tests

# Full suite
pytest -m full                              # All 1,356 tests (~2 hours)
```

---

## Next Steps

**Manager (this session)**:
1. Implement Phase 1: Update manifest system
2. Implement Phase 2: Expected output generation
3. Implement Phase 3: Test file generation
4. Document progress

**Next AI**:
- Run expected output generation for all 452 PDFs (2-3 hours)
- Commit large expected outputs
- Run validation tests
- Create final report

**Blocked on**: None - Ready to implement

---

## Success Criteria

✅ **Completeness**:
- 452 PDFs with expected outputs
- 1,356 static test functions
- All committed to git

✅ **Correctness**:
- All manifests validate (MD5 checks)
- smoke_fast passes (< 1 min)
- standard_60_set passes

✅ **Maintainability**:
- LLM can read all test files
- Clear organization (hierarchical)
- Regeneration script works

✅ **Performance**:
- smoke_fast < 1 minute (git hook)
- standard_60_set < 15 minutes
- Full suite < 2 hours
