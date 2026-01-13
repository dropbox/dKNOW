#!/usr/bin/env python3
"""
Regenerate PPM Image Baselines

Regenerates PPM baselines for all test PDFs using the current pdfium_cli binary.
This is used when the rendering implementation changes (e.g., threading fixes)
and we need to update baselines to match the new output.

Usage:
    python lib/regenerate_ppm_baselines.py --all          # Regenerate all 452 baselines
    python lib/regenerate_ppm_baselines.py arxiv_001.pdf  # Regenerate single PDF
"""

import argparse
import csv
import hashlib
import json
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Dict, List, Optional


class PPMBaselineRegenerator:
    """Regenerates PPM baselines using current pdfium_cli binary."""

    def __init__(self, integration_tests_root: Path):
        self.root = Path(integration_tests_root)
        self.pdfium_root = self.root.parent

        # Find pdfium_cli binary
        self.cli_bin = self.pdfium_root / 'out' / 'Release' / 'pdfium_cli'
        if not self.cli_bin.exists():
            raise FileNotFoundError(
                f"pdfium_cli binary not found: {self.cli_bin}\n"
                f"Build it with: ninja -C out/Release pdfium_cli"
            )

        print(f"Using binary: {self.cli_bin}")

        # Baseline directory
        self.baselines_dir = self.root / 'baselines' / 'upstream' / 'images_ppm'
        self.baselines_dir.mkdir(parents=True, exist_ok=True)

    def compute_md5(self, filepath: Path) -> str:
        """Compute MD5 hash of file."""
        md5_hash = hashlib.md5()
        with open(filepath, 'rb') as f:
            for chunk in iter(lambda: f.read(8192), b""):
                md5_hash.update(chunk)
        return md5_hash.hexdigest()

    def regenerate_baseline(self, pdf_path: Path, workers: int = 1,
                           threads: int = 1, quality: str = 'balanced') -> Dict:
        """
        Regenerate PPM baseline for a single PDF.

        Args:
            pdf_path: Path to PDF file
            workers: Number of worker processes (1-16)
            threads: Number of threads per worker (1-8)
            quality: Rendering quality ('balanced', 'fast', 'none')

        Returns:
            Dict with statistics
        """
        pdf_stem = pdf_path.stem
        baseline_path = self.baselines_dir / f'{pdf_stem}.json'

        print(f"  Regenerating: {pdf_path.name}")

        # Create temp directory for PPM output
        with tempfile.TemporaryDirectory() as temp_dir:
            temp_path = Path(temp_dir)

            try:
                # Render to PPM format using pdfium_cli
                # Command: pdfium_cli --format ppm --workers N --threads 1 --quality Q render-pages input.pdf output/
                # N=249: CRITICAL - Must use --threads 1 to match test harness (see CRITICAL_N249_threads_correctness_bug.md)
                cmd = [
                    str(self.cli_bin),
                    '--format', 'ppm',
                    '--workers', str(workers),
                    '--threads', '1',  # N=249: Match test harness (tests use --threads 1)
                    '--quality', quality,
                    'render-pages',
                    str(pdf_path),
                    str(temp_path)
                ]

                result = subprocess.run(
                    cmd,
                    capture_output=True,
                    text=True,
                    timeout=600,
                    check=False
                )

                if result.returncode != 0:
                    print(f"    ⚠ pdfium_cli returned {result.returncode}")
                    print(f"    stderr: {result.stderr[:200]}")
                    baseline_path.write_text('{}')
                    return {'success': False, 'pages': 0, 'error': 'pdfium_cli_failed'}

                # Find all generated PPM files
                # pdfium_cli creates files named page_NNNN.ppm
                ppm_files = sorted(temp_path.glob('page_*.ppm'))

                if not ppm_files:
                    print(f"    ⚠ No PPM files generated")
                    baseline_path.write_text('{}')
                    return {'success': False, 'pages': 0, 'error': 'no_images'}

                # Compute MD5 for each page
                page_hashes = {}
                for ppm_file in ppm_files:
                    # Extract page number from filename (e.g., page_0000.ppm -> 0)
                    page_num_str = ppm_file.stem.replace('page_', '')
                    page_num = int(page_num_str)

                    # Compute MD5
                    md5 = self.compute_md5(ppm_file)
                    page_hashes[str(page_num)] = md5

                # Create baseline JSON
                baseline_data = {
                    'pdf_name': pdf_path.name,
                    'format': 'ppm',
                    'dpi': 300,
                    'pages': page_hashes
                }

                # Write baseline
                with open(baseline_path, 'w') as f:
                    json.dump(baseline_data, f, indent=2)

                print(f"    ✓ Generated: {len(page_hashes)} pages")

                return {
                    'success': True,
                    'pages': len(page_hashes)
                }

            except subprocess.TimeoutExpired:
                print(f"    ✗ Timeout (>600s)")
                baseline_path.write_text('{}')
                return {'success': False, 'pages': 0, 'error': 'timeout'}

            except Exception as e:
                print(f"    ✗ Error: {e}")
                baseline_path.write_text('{}')
                return {'success': False, 'pages': 0, 'error': str(e)}

    def regenerate_all(self, workers: int = 1, threads: int = 1,
                      quality: str = 'balanced') -> Dict:
        """
        Regenerate all PPM baselines from manifest.

        Returns:
            Dict with aggregate statistics
        """
        # Load PDFs from manifest (use pdf_manifest.csv which has all 452 PDFs)
        manifest_file = self.root / 'master_test_suite' / 'pdf_manifest.csv'
        if not manifest_file.exists():
            print(f"Error: Manifest not found: {manifest_file}")
            print("Generate manifest with: python lib/manifest_generator.py generate-main")
            sys.exit(1)

        pdf_paths = []
        with open(manifest_file, 'r') as f:
            reader = csv.DictReader(f)
            for row in reader:
                # pdf_manifest.csv has relative paths in pdf_path column
                pdf_path = self.root / row['pdf_path']
                if pdf_path.exists():
                    pdf_paths.append(pdf_path)

        print(f"Regenerating baselines for {len(pdf_paths)} PDFs...")
        print(f"Configuration: workers={workers}, threads={threads}, quality={quality}")
        print(f"This will take 20-30 minutes.\n")

        stats = {
            'total': len(pdf_paths),
            'success': 0,
            'failed': 0,
            'total_pages': 0
        }

        for i, pdf_path in enumerate(pdf_paths, 1):
            print(f"\n[{i}/{len(pdf_paths)}] {pdf_path.name}")

            result = self.regenerate_baseline(pdf_path, workers, threads, quality)

            if result.get('success'):
                stats['success'] += 1
                stats['total_pages'] += result.get('pages', 0)
            else:
                stats['failed'] += 1

        return stats


def main():
    """CLI for baseline regeneration."""
    parser = argparse.ArgumentParser(
        description='Regenerate PPM image baselines using current pdfium_cli binary'
    )
    parser.add_argument(
        '--all',
        action='store_true',
        help='Regenerate all baselines from manifest'
    )
    parser.add_argument(
        'pdf_name',
        nargs='?',
        help='PDF filename to regenerate (if not using --all)'
    )
    parser.add_argument(
        '--workers',
        type=int,
        default=1,
        help='Number of worker processes (1-16, default: 1)'
    )
    parser.add_argument(
        '--threads',
        type=int,
        default=1,
        help='Number of threads per worker (1-8, default: 1)'
    )
    parser.add_argument(
        '--quality',
        choices=['balanced', 'fast', 'none'],
        default='balanced',
        help='Rendering quality (default: balanced)'
    )

    args = parser.parse_args()

    integration_tests_root = Path(__file__).parent.parent
    regenerator = PPMBaselineRegenerator(integration_tests_root)

    if args.all:
        # Regenerate all baselines
        stats = regenerator.regenerate_all(args.workers, args.threads, args.quality)

        print("\n" + "=" * 60)
        print("BASELINE REGENERATION COMPLETE")
        print("=" * 60)
        print(f"Total PDFs: {stats['total']}")
        print(f"Success: {stats['success']}")
        print(f"Failed: {stats['failed']}")
        print(f"Total pages: {stats['total_pages']}")

    elif args.pdf_name:
        # Regenerate single PDF
        pdf_path = None
        for subdir in ['benchmark', 'edge_cases']:
            candidate = integration_tests_root / 'pdfs' / subdir / args.pdf_name
            if candidate.exists():
                pdf_path = candidate
                break

        if not pdf_path:
            print(f"Error: PDF not found: {args.pdf_name}")
            sys.exit(1)

        result = regenerator.regenerate_baseline(pdf_path, args.workers, args.threads, args.quality)

        if result.get('success'):
            print(f"\n✓ Success: {result['pages']} pages")
        else:
            print(f"\n✗ Failed: {result.get('error', 'unknown error')}")
            sys.exit(1)

    else:
        parser.print_help()
        sys.exit(1)


if __name__ == '__main__':
    main()
