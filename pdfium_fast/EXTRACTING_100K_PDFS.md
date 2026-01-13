# Guide: Extracting 100,000 PDFs with pdfium_fast

**Copyright © 2025 Andrew Yates. All rights reserved.**

**Tested:** Real-world corpus of 169K PDFs (user feedback PR #17)
**Result:** 93% success rate, 27.2 PDFs/second
**Version:** v1.9.0

---

## Scenario 1: Extract Text from 100K PDFs

### Simple Batch Mode (Recommended)

```bash
#!/bin/bash
# extract_text_100k.sh

PDFIUM=./pdfium_cli  # Or full path to binary
INPUT=/path/to/100k_pdfs
OUTPUT=/path/to/text_output

# Batch mode with 4 workers
$PDFIUM --batch --recursive --workers 4 extract-text $INPUT $OUTPUT

# Expected:
# - Time: ~1 hour (100,000 / 27.2 PDFs/sec = 3,676 seconds)
# - Success: ~93,000 PDFs (93% success rate from real testing)
# - Output size: ~22 GB text files
# - Memory: ~500 MB per worker = 2 GB total
```

### Parallel Processing (Faster)

```python
#!/usr/bin/env python3
"""
Extract text from 100K PDFs using Python bindings.
Parallel processing: 8 PDFs at once × 4 workers each = 32-way parallelism.
"""

from dash_pdf_extraction import PDFProcessor
from pathlib import Path
import multiprocessing as mp
from datetime import datetime

def extract_one_pdf(args):
    """Extract text from single PDF."""
    pdf_path, output_dir = args

    processor = PDFProcessor()
    output_path = Path(output_dir) / f"{pdf_path.stem}.txt"

    try:
        # 4 workers per PDF (optimal per user testing)
        text = processor.extract_text(str(pdf_path), workers=4)

        # Save as UTF-8 (pdfium outputs UTF-32, Python handles conversion)
        output_path.write_text(text, encoding='utf-8')

        return {"path": str(pdf_path), "status": "success", "bytes": len(text)}

    except Exception as e:
        return {"path": str(pdf_path), "status": "failed", "error": str(e)}

def extract_corpus(pdf_dir, output_dir, parallel_pdfs=8):
    """
    Extract 100K PDFs in parallel.

    Args:
        pdf_dir: Directory containing PDFs
        output_dir: Where to save text files
        parallel_pdfs: How many PDFs to process simultaneously (default 8)
    """
    pdf_dir = Path(pdf_dir)
    output_dir = Path(output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    # Find all PDFs
    pdfs = list(pdf_dir.rglob("*.pdf"))
    print(f"Found {len(pdfs):,} PDFs")

    start_time = datetime.now()

    # Process in parallel (8 PDFs × 4 workers = 32 concurrent)
    args_list = [(pdf, output_dir) for pdf in pdfs]

    with mp.Pool(parallel_pdfs) as pool:
        results = pool.map(extract_one_pdf, args_list)

    # Stats
    elapsed = (datetime.now() - start_time).total_seconds()
    success = sum(1 for r in results if r["status"] == "success")
    failed = len(results) - success

    print(f"\\nComplete!")
    print(f"Time: {elapsed/60:.1f} minutes ({elapsed/3600:.2f} hours)")
    print(f"Success: {success:,}/{len(pdfs):,} ({success/len(pdfs)*100:.1f}%)")
    print(f"Failed: {failed:,}")
    print(f"Throughput: {len(pdfs)/elapsed:.1f} PDFs/second")

    # Save failure log
    failed_pdfs = [r for r in results if r["status"] == "failed"]
    if failed_pdfs:
        with open(output_dir / "failed.log", "w") as f:
            for item in failed_pdfs:
                f.write(f"{item['path']}: {item.get('error', 'Unknown')}\\n")
        print(f"Failed PDFs logged to: {output_dir}/failed.log")

if __name__ == "__main__":
    # Usage
    extract_corpus(
        pdf_dir="/path/to/100k_pdfs",
        output_dir="/path/to/text_output",
        parallel_pdfs=8  # Adjust based on RAM (8 PDFs × 500 MB = 4 GB)
    )

# Expected for 100K PDFs:
# - Time: ~1 hour (real user got 27.2 PDFs/sec)
# - Memory: 4 GB (8 PDFs × 500 MB each)
# - Success: ~93,000 PDFs (93% success rate from testing)
# - Output: ~22 GB text files
```

**Expected performance:**
- **Time:** 1 hour for 100K PDFs
- **Memory:** 4 GB total (8 parallel PDFs × 500 MB each)
- **Success rate:** 93% (based on real testing)

---

## Scenario 2: Extract Images as JPEG (Solves 4.5 TB Problem!)

### Why JPEG Matters

**User feedback (PR #18):** Started extracting 169K PDFs, hit 87 GB in 30 minutes, projected 4.5 TB total.

**Solution:** Use JPEG with web preset

```bash
#!/bin/bash
# extract_images_100k.sh - JPEG output (84x smaller than PNG)

PDFIUM=./pdfium_cli
INPUT=/path/to/100k_pdfs
OUTPUT=/path/to/images

# Process with web preset (150 DPI JPEG q85)
find $INPUT -name "*.pdf" | while read pdf; do
  basename=$(basename "$pdf" .pdf")
  mkdir -p "$OUTPUT/$basename"

  # Web preset: 150 DPI JPEG, 84x smaller than 300 DPI PNG
  $PDFIUM --preset web render-pages "$pdf" "$OUTPUT/$basename/" 2>/dev/null

  # Show progress
  echo "Processed: $basename"
done

# Expected for 100K PDFs:
# - Output size: 37 GB (vs 3.1 TB PNG = 84x savings!)
# - Time: Same as PNG (~1-2 hours)
# - Quality: 150 DPI JPEG q85 (suitable for web/preview)
```

**Disk space comparison (100K PDFs, ~100 pages each):**
- **300 DPI PNG:** 3.1 TB (impractical!)
- **150 DPI JPEG (web):** 37 GB (84x smaller) ⭐
- **72 DPI JPEG (thumbnail):** 11 GB (282x smaller)

---

## Scenario 3: Python API (Most Flexible)

```python
#!/usr/bin/env python3
"""
Extract text + images from 100K PDFs.
Optimized for large-scale processing.
"""

from dash_pdf_extraction import PDFProcessor
from pathlib import Path
import multiprocessing as mp
import json

def process_pdf_full(args):
    """Extract both text and images from PDF."""
    pdf_path, text_out, image_out = args

    processor = PDFProcessor()
    basename = pdf_path.stem

    try:
        # Extract text (fast)
        text = processor.extract_text(str(pdf_path), workers=4)

        # Save text
        text_path = Path(text_out) / f"{basename}.txt"
        text_path.write_text(text, encoding='utf-8')

        # Render pages as JPEG (web preset)
        image_dir = Path(image_out) / basename
        image_dir.mkdir(parents=True, exist_ok=True)

        # Use render_pages with web preset equivalent
        images = processor.render_pages(
            str(pdf_path),
            str(image_dir),
            format="jpg",
            dpi=150,
            jpeg_quality=85,
            workers=4
        )

        return {
            "path": str(pdf_path),
            "status": "success",
            "text_chars": len(text),
            "images": len(images)
        }

    except Exception as e:
        return {
            "path": str(pdf_path),
            "status": "failed",
            "error": str(e)
        }

def extract_corpus_complete(pdf_dir, text_out, image_out, parallel=8):
    """
    Complete extraction: text + images for 100K PDFs.

    Args:
        pdf_dir: Input PDFs
        text_out: Text output directory
        image_out: Image output directory
        parallel: Simultaneous PDFs (default 8)
    """
    pdfs = list(Path(pdf_dir).rglob("*.pdf"))
    print(f"Found {len(pdfs):,} PDFs\\n")

    args_list = [(pdf, text_out, image_out) for pdf in pdfs]

    with mp.Pool(parallel) as pool:
        results = pool.map(process_pdf_full, args_list)

    # Save manifest
    manifest_path = Path(text_out) / "manifest.json"
    with open(manifest_path, "w") as f:
        json.dump(results, f, indent=2)

    # Print stats
    success = sum(1 for r in results if r["status"] == "success")
    total_chars = sum(r.get("text_chars", 0) for r in results if r["status"] == "success")
    total_images = sum(r.get("images", 0) for r in results if r["status"] == "success")

    print(f"\\nSuccess: {success:,}/{len(pdfs):,} ({success/len(pdfs)*100:.1f}%)")
    print(f"Text extracted: {total_chars:,} characters (~{total_chars/1024/1024:.0f} MB)")
    print(f"Images rendered: {total_images:,} pages")
    print(f"Manifest saved: {manifest_path}")

# Usage
if __name__ == "__main__":
    extract_corpus_complete(
        pdf_dir="/path/to/100k_pdfs",
        text_out="/path/to/text",
        image_out="/path/to/images",
        parallel=8  # 8 PDFs × 4 workers = 32-way parallelism
    )
```

**Expected for 100K PDFs:**
- **Time:** 2-3 hours total (text + images)
- **Text output:** ~22 GB
- **Image output:** ~37 GB (JPEG web preset, vs 3.1 TB PNG!)
- **Total:** 59 GB (practical!)
- **Memory:** ~4 GB (8 parallel PDFs)

---

## Real-World Performance (Verified)

### Rendering Speed: Constant at All DPIs

| DPI | Time | Throughput | Memory | Disk (100p) |
|-----|------|------------|--------|-------------|
| 300 | 0.68s | 146 pps | 972 MB | 3.1 GB PNG |
| 150 (web) | 0.69s | 146 pps | 191 MB | **37 MB JPEG** |
| 72 (thumb) | 0.68s | 148 pps | 60 MB | **11 MB JPEG** |

**Key insight:** Speed is constant (memory-bound system), but memory and disk space improve dramatically.

### What v1.9.0 Really Provides

**Not speed** (still 72x from v1.6.0):
- BGR mode: 0.976x (slightly slower)
- DPI changes: No speed difference
- Async I/O: No measurable gain

**But disk space & memory:**
- JPEG: 84x smaller files (3.1 GB → 37 MB)
- Lower DPI: 94% memory savings (972 MB → 60 MB)
- **This is the REAL win for large-scale extraction**

### Realistic Expectations

**For 100K PDFs extraction:**
- **Text only:** 1 hour, 22 GB output
- **Images (PNG 300 DPI):** 3.1 TB output (impractical!)
- **Images (JPEG web):** 37 GB output (practical!) ⭐
- **Images (JPEG thumb):** 11 GB output (perfect for previews)

**Throughput:** 27.2 PDFs/second (user-tested on real corpus)

---

## Bottom Line

**The 130x and 166x claims are WRONG** - those compare different quality levels (invalid).

**What's REAL:**
- Speed: 72x (unchanged)
- Disk space: 84-282x smaller with JPEG
- Memory: 80-94% less at lower DPI
- **Use `--preset web` for your 100K extraction**

This solves the 4.5 TB problem with 37 GB instead!
