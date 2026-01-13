#!/usr/bin/env python3
"""
Regenerate PPM baselines for specific PDFs using current pdfium_cli binary.

This script renders test PDFs to PPM format at 300 DPI using the CURRENT
pdfium_cli binary (with pattern cache fix), then computes MD5 hashes.

Usage:
    ./regenerate_ppm_baseline.py arxiv_039
    ./regenerate_ppm_baseline.py --pdf arxiv_039.pdf
    ./regenerate_ppm_baseline.py --all  # Regenerate all baselines
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

def regenerate_ppm_baseline(pdf_path: Path, pdfium_cli: Path, output_json: Path) -> dict:
    """Regenerate PPM baseline for a single PDF."""

    with tempfile.TemporaryDirectory() as tmpdir:
        tmp_path = Path(tmpdir)

        # Render PDF to PPM using current pdfium_cli
        # CRITICAL: Must match test parameters exactly (--workers 1 --threads 1 --quality balanced)
        # to ensure baseline MD5s match test execution
        cmd = [
            str(pdfium_cli),
            "--workers", "1",
            "--threads", "1",
            "--quality", "balanced",
            "--ppm",
            "render-pages",
            str(pdf_path),
            str(tmp_path)
        ]

        result = subprocess.run(cmd, capture_output=True, text=True)

        if result.returncode != 0:
            raise RuntimeError(f"pdfium_cli failed: {result.stderr}")

        # Find generated PPM files
        ppm_files = sorted(tmp_path.glob("page_*.ppm"))

        if not ppm_files:
            print(f"  WARNING: No PPM files generated for {pdf_path.name}")
            return None

        # Compute MD5 for each page
        baseline = {
            "pdf_name": pdf_path.name,
            "format": "ppm",
            "dpi": 300,
            "pages": {}
        }

        for ppm_file in ppm_files:
            # Extract page number from filename: page_0002.ppm -> 2
            page_num = int(ppm_file.stem.split('_')[1])

            # Compute MD5
            md5 = compute_md5(ppm_file)

            baseline["pages"][str(page_num)] = md5

        return baseline

def main():
    parser = argparse.ArgumentParser(description="Regenerate PPM baselines using current pdfium_cli")
    parser.add_argument("pdf_stem", nargs="?", help="PDF stem (e.g., 'arxiv_039')")
    parser.add_argument("--pdf", help="PDF filename (e.g., 'arxiv_039.pdf')")
    parser.add_argument("--all", action="store_true", help="Regenerate all PPM baselines")

    args = parser.parse_args()

    # Paths
    script_dir = Path(__file__).parent
    pdfium_cli = script_dir.parent / "out" / "Release" / "pdfium_cli"
    baselines_dir = script_dir / "baselines" / "upstream" / "images_ppm"
    pdfs_dir = script_dir / "pdfs"

    if not pdfium_cli.exists():
        print(f"Error: pdfium_cli not found at {pdfium_cli}")
        sys.exit(1)

    # Get list of PDFs to process
    pdfs_to_process = []

    if args.all:
        # Get all existing baseline JSONs
        for json_file in sorted(baselines_dir.glob("*.json")):
            pdf_stem = json_file.stem
            # Try to find PDF in various categories
            for category in ["arxiv", "benchmark", "cc", "edinet", "web", "edge_cases"]:
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
        for category in ["arxiv", "benchmark", "cc", "edinet", "web", "edge_cases"]:
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
    print(f"Regenerating {len(pdfs_to_process)} baseline(s) using current pdfium_cli")
    print(f"Binary: {pdfium_cli}")
    print()

    success_count = 0
    error_count = 0

    for i, (pdf_path, output_json) in enumerate(pdfs_to_process, 1):
        print(f"[{i}/{len(pdfs_to_process)}] {pdf_path.name}", end=" ... ", flush=True)

        try:
            baseline = regenerate_ppm_baseline(pdf_path, pdfium_cli, output_json)

            if baseline is None:
                print("SKIP (no pages)")
                continue

            # Save baseline JSON
            output_json.parent.mkdir(parents=True, exist_ok=True)

            with open(output_json, 'w') as f:
                json.dump(baseline, f, indent=2)

            print(f"OK ({len(baseline['pages'])} pages)")
            success_count += 1

        except Exception as e:
            print(f"ERROR: {e}")
            error_count += 1

    print()
    print(f"Done! {success_count} succeeded, {error_count} failed")

if __name__ == "__main__":
    main()
