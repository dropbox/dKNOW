#!/usr/bin/env python3
"""
Test script to collect SIMD batch size statistics.
Renders a PDF and then calls DumpSIMDBatchStats() to print histogram.
"""

import ctypes
import subprocess
import sys

def main():
    # Render a PDF to trigger SIMD operations
    pdf_path = "integration_tests/pdfs/samples/cc_001_100p.pdf"
    output_dir = "/tmp/batch_stats_test"

    print(f"Rendering {pdf_path}...")
    result = subprocess.run([
        "./rust/target/release/pdfium_cli",
        "--mode", "bulk",
        "--images",
        "--pdf", pdf_path,
        "--output", output_dir
    ], capture_output=True, text=True)

    if result.returncode != 0:
        print(f"Error rendering PDF: {result.stderr}")
        sys.exit(1)

    print("Rendering complete. Loading library to dump stats...")

    # Load the library
    lib = ctypes.CDLL("./libpdfium.dylib")

    # Call DumpSIMDBatchStats
    dump_func = lib.DumpSIMDBatchStats
    dump_func.restype = None
    dump_func()

    print("\nDone!")

if __name__ == "__main__":
    main()
