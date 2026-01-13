#!/usr/bin/env python3
"""
Generate PPM baselines using upstream pdfium_test binary.

This script renders all test PDFs to PPM format at 300 DPI using the upstream
pdfium_test binary, then computes MD5 hashes for validation.
"""

import subprocess
import json
import hashlib
from pathlib import Path
import sys
import argparse

def generate_ppm_baseline(pdf_path: Path, pdfium_test_bin: Path, output_dir: Path) -> dict:
    """Generate PPM files and compute MD5 hashes for a PDF."""

    # Create output directory
    output_dir.mkdir(parents=True, exist_ok=True)

    # Copy PDF to output directory (pdfium_test requires local path)
    import shutil
    pdf_name = pdf_path.name
    local_pdf = output_dir / pdf_name
    shutil.copy(pdf_path, local_pdf)

    # Run pdfium_test to generate PPM files
    # Scale factor: 300 DPI / 72 DPI = 4.166666
    cmd = [
        str(pdfium_test_bin),
        "--ppm",
        "--scale=4.166666",
        pdf_name  # Use local filename
    ]

    result = subprocess.run(cmd, cwd=output_dir, capture_output=True, text=True)

    if result.returncode != 0:
        raise RuntimeError(f"pdfium_test failed: {result.stderr}")

    # Find generated PPM files (format: {pdf_name}.{page_num}.ppm)
    ppm_files = sorted(output_dir.glob(f"{pdf_name}.*.ppm"))
    
    # Compute MD5 for each page
    baseline = {
        "pdf_name": pdf_name,
        "format": "ppm",
        "dpi": 300,
        "pages": {}
    }
    
    for ppm_file in ppm_files:
        # Extract page number from filename: web_039.pdf.0.ppm -> 0
        parts = ppm_file.stem.split('.')
        page_num = int(parts[-1])
        
        # Compute MD5
        with open(ppm_file, 'rb') as f:
            md5 = hashlib.md5(f.read()).hexdigest()
        
        baseline["pages"][str(page_num)] = md5

        # Clean up PPM file (we only need the MD5)
        ppm_file.unlink()

    # Clean up copied PDF
    local_pdf.unlink()

    return baseline

def main():
    parser = argparse.ArgumentParser(description="Generate PPM baselines from upstream pdfium_test")
    parser.add_argument("--pdf", help="Single PDF to process")
    parser.add_argument("--all", action="store_true", help="Process all PDFs in pdf_manifest.csv")
    parser.add_argument("--output-dir", default="baselines/upstream/images_ppm",
                       help="Output directory for baseline JSON files")
    
    args = parser.parse_args()
    
    # Paths
    script_dir = Path(__file__).parent
    pdfium_test_bin = script_dir.parent / "out" / "Optimized-Shared" / "pdfium_test"
    output_dir = script_dir / args.output_dir
    temp_dir = Path("/tmp/ppm_baseline_gen")
    
    if not pdfium_test_bin.exists():
        print(f"Error: pdfium_test not found at {pdfium_test_bin}")
        sys.exit(1)
    
    # Get list of PDFs to process
    if args.pdf:
        pdfs = [script_dir / "pdfs" / "benchmark" / args.pdf]
    elif args.all:
        # Read from pdf_manifest.csv
        import csv
        manifest_path = script_dir / "master_test_suite" / "pdf_manifest.csv"
        with open(manifest_path) as f:
            reader = csv.DictReader(f)
            pdfs = [script_dir / row["pdf_path"] for row in reader]
    else:
        print("Error: specify --pdf or --all")
        sys.exit(1)
    
    # Process each PDF
    for i, pdf_path in enumerate(pdfs, 1):
        if not pdf_path.exists():
            print(f"[{i}/{len(pdfs)}] SKIP: {pdf_path.name} (not found)")
            continue
        
        print(f"[{i}/{len(pdfs)}] Processing: {pdf_path.name}")
        
        try:
            # Generate baseline
            baseline = generate_ppm_baseline(pdf_path, pdfium_test_bin, temp_dir)
            
            # Save baseline JSON
            output_json = output_dir / f"{pdf_path.stem}.json"
            output_json.parent.mkdir(parents=True, exist_ok=True)
            
            with open(output_json, 'w') as f:
                json.dump(baseline, f, indent=2)
            
            print(f"  -> {len(baseline['pages'])} pages, saved to {output_json.name}")
            
        except Exception as e:
            print(f"  ERROR: {e}")
    
    print("\nDone!")

if __name__ == "__main__":
    main()
