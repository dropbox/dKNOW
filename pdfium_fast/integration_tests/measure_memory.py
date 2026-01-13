#!/usr/bin/env python3
"""
Memory measurement script for streaming validation.

Tests memory usage during PDF processing to validate streaming architecture.
"""

import subprocess
import sys
from pathlib import Path

def measure_memory(pdf_path: Path, operation: str = "render"):
    """
    Measure peak memory usage during PDF processing.

    Uses /usr/bin/time -l to get peak RSS on macOS.
    """
    cli_path = Path(__file__).parent.parent / "out" / "Release" / "pdfium_cli"
    assert cli_path.exists(), f"pdfium_cli not found at {cli_path}"

    output_dir = Path("/tmp/memory_test_output")
    output_dir.mkdir(exist_ok=True)

    if operation == "render":
        cmd = [str(cli_path), "render-pages", str(pdf_path), str(output_dir)]
    elif operation == "text":
        output_file = output_dir / "text.txt"
        cmd = [str(cli_path), "extract-text", str(pdf_path), str(output_file)]
    else:
        raise ValueError(f"Unknown operation: {operation}")

    # Use /usr/bin/time -l to measure peak RSS (macOS)
    time_cmd = ["/usr/bin/time", "-l"] + cmd

    print(f"Running: {' '.join(cmd)}")
    print(f"PDF: {pdf_path.name} ({pdf_path.stat().st_size / 1024 / 1024:.1f} MB)")

    result = subprocess.run(
        time_cmd,
        capture_output=True,
        text=True
    )

    if result.returncode != 0:
        print(f"Error: Command failed with code {result.returncode}")
        print(f"Stderr: {result.stderr}")
        return None

    # Parse peak RSS from time output
    for line in result.stderr.split('\n'):
        line = line.strip()
        if 'maximum resident set size' in line:
            # Format: "  12345678  maximum resident set size"
            parts = line.split()
            rss_bytes = int(parts[0])
            rss_mb = rss_bytes / 1024 / 1024
            return rss_mb

    return None

def main():
    pdfs_dir = Path(__file__).parent / "pdfs" / "benchmark"

    # Test PDFs of different sizes
    test_pdfs = [
        ("small", "10pages_*"),
        ("medium", "100pages_*"),
        ("large", "291pages_*"),
        ("huge", "cc_001_931p.pdf"),
    ]

    print("=" * 80)
    print("Memory Usage Test - Streaming Validation")
    print("=" * 80)
    print()

    for size_name, pattern in test_pdfs:
        # Find PDF matching pattern
        pdfs = list(pdfs_dir.glob(pattern))
        if not pdfs:
            print(f"⚠️  {size_name}: No PDF found matching '{pattern}'")
            continue

        pdf_path = pdfs[0]
        pages = pdf_path.stem.split('_')[0].replace('pages', '').replace('p', '')

        # Test text extraction (lower memory)
        print(f"Testing {size_name} PDF ({pages} pages, {pdf_path.stat().st_size / 1024 / 1024:.1f} MB):")

        mem_text = measure_memory(pdf_path, "text")
        if mem_text:
            print(f"  Text extraction: {mem_text:.1f} MB peak RSS")

        # Skip render for huge PDF (takes too long)
        if size_name != "huge":
            mem_render = measure_memory(pdf_path, "render")
            if mem_render:
                print(f"  Image rendering:  {mem_render:.1f} MB peak RSS")

        print()

    print("=" * 80)
    print("Expected: Memory usage should be <100MB regardless of PDF size")
    print("If memory is constant (±20MB), streaming is working correctly")
    print("=" * 80)

if __name__ == "__main__":
    main()
