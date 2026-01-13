#!/usr/bin/env python3
"""
Generate TRUE upstream baselines using unmodified upstream pdfium_test binary.

This script renders test PDFs to PPM format at 300 DPI using the UPSTREAM
pdfium_test binary at commit 7f43fd79 (unmodified), then computes MD5 hashes.

This provides TRUE ground truth baselines for validating correctness of our
modifications (threading, bug fixes, etc.).

Usage:
    ./generate_upstream_baselines.py --all  # Generate all baselines
    ./generate_upstream_baselines.py arxiv_039  # Single PDF
"""

import subprocess
import json
import hashlib
from pathlib import Path
import sys
import argparse
import tempfile
import shutil

def compute_md5(file_path: Path) -> str:
    """Compute MD5 hash of a file."""
    md5 = hashlib.md5()
    with open(file_path, 'rb') as f:
        while chunk := f.read(8192):
            md5.update(chunk)
    return md5.hexdigest()

def generate_upstream_baseline(pdf_path: Path, pdfium_test: Path, output_json: Path) -> dict:
    """Generate upstream PPM baseline for a single PDF."""

    with tempfile.TemporaryDirectory() as tmpdir:
        tmp_path = Path(tmpdir)

        # Copy PDF to temp dir (upstream pdfium_test outputs to same dir as PDF)
        temp_pdf = tmp_path / pdf_path.name
        shutil.copy2(pdf_path, temp_pdf)

        # Render PDF to PPM using upstream pdfium_test
        # Upstream command: pdfium_test --ppm --scale=4.166666 input.pdf
        # Scale formula: 300 DPI / 72 DPI = 4.166666 (6 decimals per CLAUDE.md)
        cmd = [
            str(pdfium_test),
            "--ppm",
            "--scale=4.166666",
            str(temp_pdf)
        ]

        result = subprocess.run(cmd, capture_output=True, text=True, cwd=tmp_path)

        if result.returncode != 0:
            raise RuntimeError(f"pdfium_test failed: {result.stderr}")

        # Upstream pdfium_test outputs: basename.*.ppm (may use PDF title, not filename)
        # Look for ANY .ppm files in the temp directory
        ppm_files = sorted(tmp_path.glob("*.ppm"))

        if not ppm_files:
            print(f"  WARNING: No PPM files generated for {pdf_path.name}")
            return None

        # Compute MD5 for each page
        baseline = {
            "pdf_name": pdf_path.name,
            "format": "ppm",
            "dpi": 300,
            "source": "upstream pdfium_test 7f43fd79",
            "pages": {}
        }

        for ppm_file in ppm_files:
            # Extract page number from filename: basename.2.ppm -> 2
            # Handle various formats: "test.pdf.0.ppm", "test.0.ppm", etc.
            parts = ppm_file.stem.split('.')
            page_num = int(parts[-1])  # Last part of stem before .ppm

            # Compute MD5
            md5 = compute_md5(ppm_file)

            baseline["pages"][str(page_num)] = md5

        return baseline

def main():
    parser = argparse.ArgumentParser(description="Generate upstream PPM baselines using upstream pdfium_test")
    parser.add_argument("pdf_stem", nargs="?", help="PDF stem (e.g., 'arxiv_039')")
    parser.add_argument("--pdf", help="PDF filename (e.g., 'arxiv_039.pdf')")
    parser.add_argument("--all", action="store_true", help="Generate all upstream baselines")

    args = parser.parse_args()

    # Paths
    script_dir = Path(__file__).parent
    pdfium_test = Path.home() / "upstream-checkout" / "pdfium" / "out" / "Release" / "pdfium_test"
    baselines_dir = script_dir / "baselines" / "upstream" / "images_ppm"
    pdfs_dir = script_dir / "pdfs"

    if not pdfium_test.exists():
        print(f"Error: upstream pdfium_test not found at {pdfium_test}")
        print(f"Build it first: cd ~/upstream-checkout/pdfium && ninja -C out/Release pdfium_test")
        sys.exit(1)

    # Get list of PDFs to process
    pdfs_to_process = []

    if args.all:
        # Get all existing baseline JSONs
        for json_file in sorted(baselines_dir.glob("*.json")):
            pdf_stem = json_file.stem
            # Try to find PDF in various categories
            for category in ["arxiv", "benchmark", "cc", "edinet", "web", "edge_cases", "pages"]:
                pdf_path = pdfs_dir / category / f"{pdf_stem}.pdf"
                if pdf_path.exists():
                    pdfs_to_process.append((pdf_path, json_file))
                    break

        if not pdfs_to_process:
            print("No PDFs found to process")
            sys.exit(1)

    elif args.pdf or args.pdf_stem:
        # Single PDF
        pdf_name = args.pdf if args.pdf else f"{args.pdf_stem}.pdf"
        pdf_stem = pdf_name.replace(".pdf", "")

        # Try to find PDF in various categories
        pdf_path = None
        for category in ["arxiv", "benchmark", "cc", "edinet", "web", "edge_cases", "pages"]:
            test_path = pdfs_dir / category / pdf_name
            if test_path.exists():
                pdf_path = test_path
                break

        if not pdf_path:
            print(f"Error: PDF not found: {pdf_name}")
            sys.exit(1)

        output_json = baselines_dir / f"{pdf_stem}.json"
        pdfs_to_process = [(pdf_path, output_json)]

    else:
        print("Error: specify PDF stem, --pdf, or --all")
        parser.print_help()
        sys.exit(1)

    # Process each PDF
    print(f"Generating {len(pdfs_to_process)} upstream baseline(s)")
    print(f"Binary: {pdfium_test}")
    print(f"Commit: 7f43fd79 (2025-10-30, unmodified)")
    print()

    success_count = 0
    error_count = 0
    skip_count = 0

    for i, (pdf_path, output_json) in enumerate(pdfs_to_process, 1):
        print(f"[{i}/{len(pdfs_to_process)}] {pdf_path.name}", end=" ... ", flush=True)

        try:
            baseline = generate_upstream_baseline(pdf_path, pdfium_test, output_json)

            if baseline is None:
                print("SKIP (no pages)")
                skip_count += 1
                continue

            # Save to separate upstream baselines directory to compare
            upstream_dir = baselines_dir.parent / "images_ppm_upstream_true"
            upstream_dir.mkdir(parents=True, exist_ok=True)
            upstream_json = upstream_dir / output_json.name

            with open(upstream_json, 'w') as f:
                json.dump(baseline, f, indent=2)

            print(f"OK ({len(baseline['pages'])} pages)")
            success_count += 1

        except Exception as e:
            print(f"ERROR: {e}")
            error_count += 1

    print()
    print(f"Done! {success_count} succeeded, {error_count} failed, {skip_count} skipped")
    print(f"Baselines saved to: {baselines_dir.parent / 'images_ppm_upstream_true'}")

if __name__ == "__main__":
    main()
