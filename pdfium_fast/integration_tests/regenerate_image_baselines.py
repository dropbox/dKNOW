#!/usr/bin/env python3
"""
Regenerate Image Baselines After FPDFBitmap_CreateEx Fix

Old baselines were generated with broken FPDFBitmap_Create API.
This script regenerates all baselines using the fixed render_pages tool.

Usage:
    python regenerate_image_baselines.py [--dry-run] [--limit N]
"""

import argparse
import sys
import subprocess
import tempfile
import re
import json
from pathlib import Path


def get_pdf_list(manifest_path: Path):
    """Load PDF list from manifest."""
    import csv

    pdfs = []
    with open(manifest_path) as f:
        reader = csv.DictReader(f)
        for row in reader:
            pdfs.append(row['pdf_name'])

    return pdfs


def render_with_md5(pdf_path: Path, tool_path: Path, lib_dir: Path, worker_count: int = 1):
    """
    Render PDF and extract MD5 hashes per page.

    Returns:
        Dict[str, str]: {page_num: md5_hash} or None on error
    """
    import os

    env = os.environ.copy()
    env['DYLD_LIBRARY_PATH'] = str(lib_dir)

    with tempfile.TemporaryDirectory() as tmpdir:
        args = [str(tool_path), str(pdf_path), tmpdir, str(worker_count), "300", "--md5"]

        try:
            result = subprocess.run(
                args,
                env=env,
                capture_output=True,
                text=True,
                timeout=600
            )

            if result.returncode != 0:
                print(f"    ERROR: render_pages failed with code {result.returncode}")
                if result.stderr:
                    print(f"    stderr: {result.stderr[:200]}")
                return None

            # Parse MD5 output: "MD5:page_0000.png:abc123..."
            md5_pattern = re.compile(r'MD5:page_(\d+)\.png:([0-9a-f]+)')
            page_hashes = {}

            for line in result.stdout.splitlines():
                match = md5_pattern.match(line)
                if match:
                    page_num = str(int(match.group(1)))  # Convert "0000" to "0"
                    hash_value = match.group(2)
                    page_hashes[page_num] = hash_value

            return page_hashes if page_hashes else None

        except subprocess.TimeoutExpired:
            print(f"    TIMEOUT: render_pages took >600s")
            return None
        except Exception as e:
            print(f"    EXCEPTION: {e}")
            return None


def regenerate_baselines(pdf_list, tool_path, lib_dir, baselines_dir, dry_run=False, limit=None):
    """Regenerate all image baselines."""

    integration_tests_root = Path(__file__).parent
    pdfs_benchmark = integration_tests_root / 'pdfs' / 'benchmark'
    pdfs_edge_cases = integration_tests_root / 'pdfs' / 'edge_cases'

    images_dir = baselines_dir / 'images'
    images_dir.mkdir(parents=True, exist_ok=True)

    stats = {
        'total': len(pdf_list),
        'success': 0,
        'failed': 0,
        'skipped': 0
    }

    if limit:
        pdf_list = pdf_list[:limit]
        print(f"Limited to first {limit} PDFs")

    for i, pdf_name in enumerate(pdf_list, 1):
        print(f"\n[{i}/{len(pdf_list)}] {pdf_name}")

        # Find PDF path
        pdf_path = None
        if (pdfs_benchmark / pdf_name).exists():
            pdf_path = pdfs_benchmark / pdf_name
        elif (pdfs_edge_cases / pdf_name).exists():
            pdf_path = pdfs_edge_cases / pdf_name
        else:
            print(f"  SKIP: PDF not found")
            stats['skipped'] += 1
            continue

        if dry_run:
            print(f"  DRY-RUN: Would regenerate baseline")
            stats['success'] += 1
            continue

        # Render with MD5
        page_hashes = render_with_md5(pdf_path, tool_path, lib_dir, worker_count=1)

        if page_hashes is None:
            print(f"  FAILED: Could not generate hashes")
            stats['failed'] += 1
            continue

        # Save baseline
        baseline_path = images_dir / f'{pdf_path.stem}.json'
        baseline_path.write_text(json.dumps(page_hashes, indent=2))

        print(f"  SUCCESS: {len(page_hashes)} pages")
        print(f"  Saved to: {baseline_path.name}")
        stats['success'] += 1

    return stats


def main():
    parser = argparse.ArgumentParser(description='Regenerate image baselines after FPDFBitmap_CreateEx fix')
    parser.add_argument('--dry-run', action='store_true', help='Show what would be done without doing it')
    parser.add_argument('--limit', type=int, help='Limit to first N PDFs (for testing)')
    parser.add_argument('--all', action='store_true', help='Regenerate all PDFs (not just manifest)')
    args = parser.parse_args()

    # Paths
    integration_tests_root = Path(__file__).parent
    pdfium_root = integration_tests_root.parent

    tool_path = pdfium_root / 'rust' / 'target' / 'release' / 'examples' / 'render_pages'
    lib_dir = pdfium_root / 'out' / 'Optimized-Shared'
    manifest_path = integration_tests_root / 'master_test_suite' / 'file_manifest.csv'
    baselines_dir = integration_tests_root / 'baselines' / 'upstream'
    pdfs_dir = integration_tests_root / 'pdfs'

    # Validate paths
    if not tool_path.exists():
        print(f"ERROR: render_pages not found at {tool_path}")
        print("Build it with: cd rust && cargo build --release --examples")
        sys.exit(1)

    if not lib_dir.exists():
        print(f"ERROR: PDFium library not found at {lib_dir}")
        print("Build it with: ninja -C out/Optimized-Shared pdfium")
        sys.exit(1)

    print("=" * 70)
    print("IMAGE BASELINE REGENERATION")
    print("=" * 70)
    print(f"Tool: {tool_path}")
    print(f"Library: {lib_dir}")
    print(f"Output: {baselines_dir / 'images'}")

    if args.dry_run:
        print("\n*** DRY RUN MODE - No changes will be made ***")

    print()

    # Load PDF list
    if args.all:
        # Find all PDFs in pdfs/ directory
        pdf_list = []
        for pdf_path in sorted(pdfs_dir.rglob('*.pdf')):
            pdf_list.append(pdf_path.name)
        print(f"Found {len(pdf_list)} PDFs in {pdfs_dir}")
    else:
        if not manifest_path.exists():
            print(f"ERROR: Manifest not found at {manifest_path}")
            print("Use --all to regenerate all PDFs instead")
            sys.exit(1)
        pdf_list = get_pdf_list(manifest_path)
        print(f"Found {len(pdf_list)} PDFs in manifest")

    # Regenerate baselines
    stats = regenerate_baselines(
        pdf_list,
        tool_path,
        lib_dir,
        baselines_dir,
        dry_run=args.dry_run,
        limit=args.limit
    )

    # Summary
    print("\n" + "=" * 70)
    print("SUMMARY")
    print("=" * 70)
    print(f"Total PDFs: {stats['total']}")
    print(f"Success: {stats['success']}")
    print(f"Failed: {stats['failed']}")
    print(f"Skipped: {stats['skipped']}")

    if stats['failed'] > 0:
        print(f"\n⚠ {stats['failed']} PDFs failed - review errors above")
        sys.exit(1)

    if not args.dry_run:
        print("\n✓ All baselines regenerated successfully")
    else:
        print("\n✓ Dry run complete - use without --dry-run to regenerate")


if __name__ == '__main__':
    main()
