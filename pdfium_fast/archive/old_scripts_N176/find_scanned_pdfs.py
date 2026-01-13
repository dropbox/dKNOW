#!/usr/bin/env python3
"""
Quick script to scan PDF corpus and identify potential scanned documents.
Looks for PDFs where pages have single image objects covering ≥95% of page.
"""

import subprocess
import json
from pathlib import Path
import sys

def check_pdf_for_scanned_pages(pdf_path, pdfium_cli_path):
    """
    Use pdfium_cli to check if PDF contains scanned pages.
    We'll extract object info by attempting a render and checking structure.

    For now, use a simpler heuristic: Check page count and file size.
    Scanned PDFs tend to be larger and have fewer pages.
    """
    # This is a placeholder - we'd need to implement object inspection in pdfium_cli
    # For now, let's just compile a list and manually inspect a few
    return None

def main():
    pdf_dir = Path("integration_tests/pdfs/benchmark")
    pdfs = sorted(pdf_dir.glob("*.pdf"))

    print(f"Found {len(pdfs)} PDFs in benchmark directory")
    print("\nScanning for potential scanned documents...")
    print("(Manual inspection will be needed)\n")

    # List some PDFs with their sizes for manual inspection
    candidates = []
    for pdf in pdfs[:50]:  # Check first 50
        size_mb = pdf.stat().st_size / (1024 * 1024)
        if size_mb > 1.0:  # Larger files more likely to be scanned
            candidates.append((pdf.name, size_mb))

    candidates.sort(key=lambda x: x[1], reverse=True)

    print("Top 20 largest PDFs (potential scanned documents):")
    for name, size in candidates[:20]:
        print(f"  {name}: {size:.2f} MB")

    # Also check edge_cases directory
    edge_dir = Path("integration_tests/pdfs/edge_cases")
    if edge_dir.exists():
        edge_pdfs = list(edge_dir.glob("*.pdf"))
        print(f"\nFound {len(edge_pdfs)} PDFs in edge_cases directory:")
        for pdf in edge_pdfs[:10]:
            size_mb = pdf.stat().st_size / (1024 * 1024)
            print(f"  {pdf.name}: {size_mb:.2f} MB")

if __name__ == "__main__":
    main()
