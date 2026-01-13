#!/usr/bin/env python3
"""
Select representative sample of PDFs for benchmarking.
Strategy: Select diverse PDFs by size, category, and page count.
"""

import os
import subprocess
import json
from pathlib import Path
from collections import defaultdict

PDF_DIR = "./integration_tests/pdfs"
PDFIUM_CLI = "./out/Release/pdfium_cli"


def get_pdf_metadata(pdf_path):
    """Get PDF metadata (size, page count)."""
    try:
        file_size = os.path.getsize(pdf_path)

        # Get page count by trying to extract text
        result = subprocess.run(
            [PDFIUM_CLI, "extract-text", pdf_path, "/dev/null"],
            capture_output=True,
            text=True,
            timeout=30
        )

        # Count output lines (rough proxy for pages) or parse stderr
        # For now, just use file size as proxy
        return {
            "path": str(pdf_path),
            "name": pdf_path.stem,
            "category": pdf_path.parent.name,
            "size_mb": file_size / (1024 * 1024),
            "size_bytes": file_size
        }
    except Exception as e:
        return None


def select_sample():
    """Select 50 representative PDFs."""
    # Find all PDFs
    pdf_files = list(Path(PDF_DIR).rglob("*.pdf"))
    print(f"Found {len(pdf_files)} total PDFs")

    # Get metadata for all
    all_metadata = []
    for pdf in pdf_files:
        meta = get_pdf_metadata(pdf)
        if meta:
            all_metadata.append(meta)

    # Group by category
    by_category = defaultdict(list)
    for meta in all_metadata:
        by_category[meta['category']].append(meta)

    # Select from each category
    sample = []

    for category, pdfs in by_category.items():
        # Sort by size
        pdfs_sorted = sorted(pdfs, key=lambda x: x['size_bytes'])

        # Select: smallest, median, largest, plus 2 random middle ones
        n = len(pdfs_sorted)
        if n >= 5:
            indices = [0, n//4, n//2, 3*n//4, n-1]
        elif n >= 3:
            indices = [0, n//2, n-1]
        elif n >= 1:
            indices = [0]
        else:
            continue

        for idx in indices:
            sample.append(pdfs_sorted[idx])

    # Print summary
    print(f"\nSelected {len(sample)} PDFs:")
    for category in sorted(set(p['category'] for p in sample)):
        cat_pdfs = [p for p in sample if p['category'] == category]
        print(f"  {category}: {len(cat_pdfs)} PDFs")

    # Save to file
    output = {
        "total_corpus": len(all_metadata),
        "sample_size": len(sample),
        "pdfs": sample
    }

    with open("scripts/representative_sample_N226.json", 'w') as f:
        json.dump(output, f, indent=2)

    print(f"\nSample saved to: scripts/representative_sample_N226.json")

    # Also print list for easy copy-paste
    print("\nPDF paths:")
    for p in sample:
        print(p['path'])


if __name__ == "__main__":
    select_sample()
