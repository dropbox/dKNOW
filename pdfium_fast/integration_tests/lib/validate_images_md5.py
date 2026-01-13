#!/usr/bin/env python3
"""
Validate ALL PDFs Against Upstream Image Rendering - MD5 Only (FAST)

MD5-only validation for correctness. No SSIM computation.
Completes 452 PDFs in ~4-6 hours vs 51+ days with SSIM.

Compares Rust image rendering against upstream pdfium_test
using MD5 hashes for byte-for-byte correctness.

Usage:
    python3 lib/validate_images_md5.py [--workers N] [--continue-from N] [--limit-pdfs N]
"""

import subprocess
import tempfile
import argparse
import sys
import csv
import json
from pathlib import Path
from datetime import datetime
import hashlib
import os
import shutil
from concurrent.futures import ProcessPoolExecutor, as_completed
from typing import Dict
from PIL import Image

class ImageValidatorMD5:
    def __init__(self):
        self.root = Path(__file__).parent.parent
        self.pdfium_root = self.root.parent

        # Upstream tool
        self.pdfium_test = self.pdfium_root / 'out' / 'Optimized-Shared' / 'pdfium_test'

        # Rust tool
        self.render_pages = self.pdfium_root / 'rust' / 'target' / 'release' / 'examples' / 'render_pages'

        # Verify tools exist
        for tool in [self.pdfium_test, self.render_pages]:
            if not tool.exists():
                raise FileNotFoundError(f"Tool not found: {tool}")

        # PDF manifest
        self.manifest_file = self.root / 'master_test_suite' / 'pdf_manifest.csv'
        if not self.manifest_file.exists():
            raise FileNotFoundError(f"PDF manifest not found: {self.manifest_file}")

        # Load PDF list
        self.pdfs = []
        with open(self.manifest_file, 'r') as f:
            reader = csv.DictReader(f)
            for row in reader:
                # PDF paths in manifest are relative to integration_tests/
                pdf_path = self.root / row['pdf_path']
                if pdf_path.exists():
                    pages_str = row.get('pdf_pages', 'unknown')
                    try:
                        pages = int(pages_str) if pages_str != 'unknown' else None
                    except ValueError:
                        pages = None

                    self.pdfs.append({
                        'name': row['pdf_name'],
                        'path': pdf_path,
                        'category': row.get('pdf_category', 'unknown'),
                        'pages': pages
                    })

        print(f"Loaded {len(self.pdfs)} PDFs from manifest", flush=True)

    def compute_md5_file(self, file_path):
        """Compute MD5 of file (raw bytes)"""
        md5 = hashlib.md5()
        with open(file_path, 'rb') as f:
            while chunk := f.read(8192):
                md5.update(chunk)
        return md5.hexdigest()

    def compute_md5_pixels(self, image_path):
        """Compute MD5 of decoded pixel data (not PNG compression)"""
        # Load image and convert to RGB
        img = Image.open(image_path).convert('RGB')
        # Get raw pixel bytes
        pixel_bytes = img.tobytes()
        # Compute MD5 of pixel data
        return hashlib.md5(pixel_bytes).hexdigest()

    def validate_image_rendering(self, pdf_info: dict) -> dict:
        """Compare upstream vs Rust image rendering for one PDF (all pages)
        Returns per-page data with MD5s only."""

        pdf_path = pdf_info['path']
        pdf_name = pdf_info['name']

        with tempfile.TemporaryDirectory() as upstream_dir:
            with tempfile.TemporaryDirectory() as ours_dir:
                try:
                    env = os.environ.copy()
                    env['DYLD_LIBRARY_PATH'] = str(self.pdfium_test.parent)

                    # Copy PDF to upstream directory so pdfium_test writes output files there
                    upstream_pdf = Path(upstream_dir) / pdf_name
                    shutil.copy2(pdf_path, upstream_pdf)

                    # Generate upstream (pdfium_test creates .ppm files with --ppm flag)
                    # Use --scale=4.166666 to match 300 DPI (300/72 = 4.166666)
                    try:
                        result = subprocess.run(
                            [str(self.pdfium_test), '--ppm', '--scale=4.166666', pdf_name],
                            capture_output=True,
                            env=env,
                            timeout=600,  # 10 minutes max per PDF
                            cwd=upstream_dir
                        )
                    except subprocess.TimeoutExpired:
                        return {
                            'pdf': pdf_name,
                            'status': 'upstream_timeout',
                            'match': False,
                            'pages_data': []
                        }

                    if result.returncode != 0:
                        return {
                            'pdf': pdf_name,
                            'status': 'upstream_failed',
                            'error': result.stderr.decode('utf-8', errors='replace')[:500],
                            'match': False,
                            'pages_data': []
                        }

                    # Convert ppm to png (using macOS native sips)
                    ppm_files = sorted(Path(upstream_dir).glob('*.ppm'))

                    if not ppm_files:
                        return {
                            'pdf': pdf_name,
                            'status': 'no_ppm_files',
                            'match': False,
                            'pages_data': []
                        }

                    for ppm in ppm_files:
                        png_path = ppm.with_suffix('.png')
                        try:
                            subprocess.run(
                                ['sips', '-s', 'format', 'png', str(ppm), '--out', str(png_path)],
                                check=True,
                                capture_output=True,
                                timeout=60
                            )
                        except (subprocess.CalledProcessError, subprocess.TimeoutExpired) as e:
                            return {
                                'pdf': pdf_name,
                                'status': 'conversion_failed',
                                'error': str(e)[:500],
                                'match': False,
                                'pages_data': []
                            }

                    # Generate ours (single-threaded for correctness validation)
                    try:
                        result = subprocess.run(
                            [str(self.render_pages), str(pdf_path), ours_dir, '1', '300'],
                            capture_output=True,
                            env=env,
                            timeout=600,
                            check=True
                        )
                    except subprocess.TimeoutExpired:
                        return {
                            'pdf': pdf_name,
                            'status': 'our_timeout',
                            'match': False,
                            'pages_data': []
                        }
                    except subprocess.CalledProcessError as e:
                        return {
                            'pdf': pdf_name,
                            'status': 'our_failed',
                            'error': e.stderr.decode('utf-8', errors='replace')[:500],
                            'match': False,
                            'pages_data': []
                        }

                    # Compare using MD5 only (fast)
                    # Build dictionaries mapping page number to file path
                    upstream_pngs = {}
                    for png_file in Path(upstream_dir).glob('*.png'):
                        parts = png_file.stem.split('.')
                        if len(parts) >= 2:
                            try:
                                page_num = int(parts[-1])
                                upstream_pngs[page_num] = png_file
                            except ValueError:
                                pass

                    our_pngs = {}
                    for png_file in Path(ours_dir).glob('page_*.png'):
                        try:
                            page_num = int(png_file.stem.split('_')[1])
                            our_pngs[page_num] = png_file
                        except (ValueError, IndexError):
                            pass

                    if len(upstream_pngs) != len(our_pngs):
                        return {
                            'pdf': pdf_name,
                            'status': 'page_count_mismatch',
                            'upstream_pages': len(upstream_pngs),
                            'our_pages': len(our_pngs),
                            'match': False,
                            'pages_data': []
                        }

                    # Compare all pages - MD5 only
                    pages_data = []
                    comparison_errors = 0
                    md5_matches = 0
                    md5_mismatches = 0

                    for page_num in sorted(upstream_pngs.keys()):
                        if page_num not in our_pngs:
                            pages_data.append({
                                'page': page_num + 1,
                                'status': 'missing_in_our_output',
                                'match': False
                            })
                            comparison_errors += 1
                            continue

                        try:
                            # Compute MD5s of PIXEL DATA (not PNG file - compression differs)
                            # This takes ~20-30ms per page but is deterministic
                            upstream_md5 = self.compute_md5_pixels(upstream_pngs[page_num])
                            our_md5 = self.compute_md5_pixels(our_pngs[page_num])

                            # Get file sizes (for reference - these will differ due to PNG compression)
                            upstream_bytes = upstream_pngs[page_num].stat().st_size
                            our_bytes = our_pngs[page_num].stat().st_size

                            # Get dimensions from first image (already loaded in compute_md5_pixels)
                            upstream_img = Image.open(upstream_pngs[page_num])
                            our_img = Image.open(our_pngs[page_num])
                            upstream_width, upstream_height = upstream_img.size
                            our_width, our_height = our_img.size

                            # MD5 comparison of pixel data
                            md5_match = (upstream_md5 == our_md5)
                            if md5_match:
                                md5_matches += 1
                            else:
                                md5_mismatches += 1

                            pages_data.append({
                                'page': page_num + 1,
                                'status': 'md5_match' if md5_match else 'md5_mismatch',
                                'match': md5_match,
                                'upstream_md5': upstream_md5,
                                'our_md5': our_md5,
                                'md5_match': md5_match,
                                'upstream_width': int(upstream_width),
                                'upstream_height': int(upstream_height),
                                'our_width': int(our_width),
                                'our_height': int(our_height),
                                'upstream_bytes': int(upstream_bytes),
                                'our_bytes': int(our_bytes)
                            })

                        except Exception as e:
                            pages_data.append({
                                'page': page_num + 1,
                                'status': 'comparison_failed',
                                'match': False,
                                'error': str(e)[:200]
                            })
                            comparison_errors += 1

                    # Overall result
                    all_match = (md5_mismatches == 0 and comparison_errors == 0)

                    return {
                        'pdf': pdf_name,
                        'status': 'compared',
                        'match': all_match,
                        'total_pages': len(upstream_pngs),
                        'compared_pages': len(pages_data),
                        'md5_matches': md5_matches,
                        'md5_mismatches': md5_mismatches,
                        'comparison_errors': comparison_errors,
                        'category': pdf_info.get('category', 'unknown'),
                        'pages': pdf_info.get('pages'),
                        'pages_data': pages_data
                    }

                except Exception as e:
                    return {
                        'pdf': pdf_name,
                        'status': 'exception',
                        'error': str(e)[:500],
                        'match': False,
                        'pages_data': []
                    }


def validate_pdf_worker(args):
    """Worker function for parallel validation"""
    idx, pdf_info = args

    # Recreate validator in worker process
    validator = ImageValidatorMD5()
    result = validator.validate_image_rendering(pdf_info)
    result['index'] = idx
    return result


def main():
    # CRITICAL: Set multiprocessing start method to 'spawn' on macOS
    import multiprocessing
    try:
        multiprocessing.set_start_method('spawn')
    except RuntimeError:
        # Already set, ignore
        pass

    parser = argparse.ArgumentParser(description='Validate ALL PDFs images against upstream (MD5 only)')
    parser.add_argument('--workers', type=int, default=4, help='Number of parallel workers (default: 4)')
    parser.add_argument('--continue-from', type=int, default=0, help='Continue from PDF index N')
    parser.add_argument('--limit-pdfs', type=int, default=None, help='Limit to first N PDFs (for testing)')
    args = parser.parse_args()

    validator = ImageValidatorMD5()

    # Prepare output files
    timestamp = datetime.now().strftime('%Y%m%d_%H%M%S')
    results_file = validator.root / 'telemetry' / f'image_validation_md5_{timestamp}.json'
    results_csv = validator.root / 'telemetry' / f'image_validation_md5_{timestamp}.csv'
    per_page_csv = validator.root / 'telemetry' / f'image_validation_md5_per_page_{timestamp}.csv'

    # CRITICAL: Open CSV files NOW and write incrementally (crash-safe)
    csv_file = open(results_csv, 'w', newline='')
    csv_writer = csv.DictWriter(csv_file, fieldnames=[
        'index', 'pdf', 'status', 'match', 'total_pages', 'compared_pages',
        'md5_matches', 'md5_mismatches', 'comparison_errors',
        'category', 'pages', 'error'
    ])
    csv_writer.writeheader()
    csv_file.flush()

    per_page_file = open(per_page_csv, 'w', newline='')
    per_page_writer = csv.DictWriter(per_page_file, fieldnames=[
        'pdf', 'page', 'status', 'match',
        'upstream_md5', 'our_md5', 'md5_match',
        'upstream_width', 'upstream_height',
        'our_width', 'our_height',
        'upstream_bytes', 'our_bytes', 'error'
    ])
    per_page_writer.writeheader()
    per_page_file.flush()

    print(f"\n{'='*80}", flush=True)
    print(f"IMAGE VALIDATION - MD5 ONLY (FAST)", flush=True)
    print(f"{'='*80}", flush=True)
    print(f"Total PDFs: {len(validator.pdfs)}", flush=True)
    print(f"Workers: {args.workers}", flush=True)
    print(f"Continue from: {args.continue_from}", flush=True)
    print(f"Upstream tool: {validator.pdfium_test}", flush=True)
    print(f"Rust tool: {validator.render_pages}", flush=True)
    print(f"Results: {results_csv}", flush=True)
    print(f"Per-page: {per_page_csv}", flush=True)
    print(f"{'='*80}\n", flush=True)
    print(f"Estimated time: 4-6 hours for 452 PDFs, ~10,000 pages", flush=True)
    print(f"{'='*80}\n", flush=True)

    start_time = datetime.now()

    # Filter PDFs if continuing or limiting
    pdfs_to_test = validator.pdfs[args.continue_from:]
    if args.limit_pdfs is not None:
        pdfs_to_test = pdfs_to_test[:args.limit_pdfs]

    results = []
    completed = 0
    failed = 0
    total_pages = 0
    total_md5_matches = 0
    total_md5_mismatches = 0

    # Progress tracking
    total = len(pdfs_to_test)

    if args.workers == 1:
        # Serial execution
        for idx, pdf_info in enumerate(pdfs_to_test, start=args.continue_from):
            result = validator.validate_image_rendering(pdf_info)
            result['index'] = idx
            results.append(result)
            completed += 1

            # WRITE IMMEDIATELY - crash-safe incremental output
            csv_writer.writerow({
                'index': result.get('index', 0),
                'pdf': result['pdf'],
                'status': result.get('status', 'unknown'),
                'match': result['match'],
                'total_pages': result.get('total_pages', ''),
                'compared_pages': result.get('compared_pages', ''),
                'md5_matches': result.get('md5_matches', ''),
                'md5_mismatches': result.get('md5_mismatches', ''),
                'comparison_errors': result.get('comparison_errors', ''),
                'category': result.get('category', ''),
                'pages': result.get('pages', ''),
                'error': result.get('error', '')
            })
            csv_file.flush()

            # Write per-page results
            for page_data in result.get('pages_data', []):
                per_page_writer.writerow({
                    'pdf': result['pdf'],
                    'page': page_data.get('page', ''),
                    'status': page_data.get('status', ''),
                    'match': page_data.get('match', ''),
                    'upstream_md5': page_data.get('upstream_md5', ''),
                    'our_md5': page_data.get('our_md5', ''),
                    'md5_match': page_data.get('md5_match', ''),
                    'upstream_width': page_data.get('upstream_width', ''),
                    'upstream_height': page_data.get('upstream_height', ''),
                    'our_width': page_data.get('our_width', ''),
                    'our_height': page_data.get('our_height', ''),
                    'upstream_bytes': page_data.get('upstream_bytes', ''),
                    'our_bytes': page_data.get('our_bytes', ''),
                    'error': page_data.get('error', '')
                })
            per_page_file.flush()

            if 'total_pages' in result:
                total_pages += result['total_pages']
            if 'md5_matches' in result:
                total_md5_matches += result['md5_matches']
            if 'md5_mismatches' in result:
                total_md5_mismatches += result['md5_mismatches']

            status_icon = '✅' if result['match'] else '❌'
            match_str = f"MD5: {result.get('md5_matches', 0)}/{result.get('total_pages', 0)}" if 'md5_matches' in result else ''
            pages_str = f"({result.get('total_pages', 0)} pages)" if 'total_pages' in result else ''
            print(f"[{completed}/{total}] {status_icon} {result['pdf']} {pages_str} {match_str}", flush=True)

            if not result['match']:
                failed += 1
                print(f"         MISMATCH: {result.get('status', 'unknown')}", flush=True)
                if 'error' in result:
                    print(f"         Error: {result['error'][:200]}", flush=True)
                if result.get('md5_mismatches', 0) > 0:
                    print(f"         MD5 mismatches: {result['md5_mismatches']} pages", flush=True)
    else:
        # Parallel execution
        worker_args = [
            (idx, pdf_info)
            for idx, pdf_info in enumerate(pdfs_to_test, start=args.continue_from)
        ]

        with ProcessPoolExecutor(max_workers=args.workers) as executor:
            futures = {executor.submit(validate_pdf_worker, arg): arg for arg in worker_args}

            for future in as_completed(futures):
                result = future.result()
                results.append(result)
                completed += 1

                # WRITE IMMEDIATELY - crash-safe incremental output
                csv_writer.writerow({
                    'index': result.get('index', 0),
                    'pdf': result['pdf'],
                    'status': result.get('status', 'unknown'),
                    'match': result['match'],
                    'total_pages': result.get('total_pages', ''),
                    'compared_pages': result.get('compared_pages', ''),
                    'md5_matches': result.get('md5_matches', ''),
                    'md5_mismatches': result.get('md5_mismatches', ''),
                    'comparison_errors': result.get('comparison_errors', ''),
                    'category': result.get('category', ''),
                    'pages': result.get('pages', ''),
                    'error': result.get('error', '')
                })
                csv_file.flush()

                # Write per-page results
                for page_data in result.get('pages_data', []):
                    per_page_writer.writerow({
                        'pdf': result['pdf'],
                        'page': page_data.get('page', ''),
                        'status': page_data.get('status', ''),
                        'match': page_data.get('match', ''),
                        'upstream_md5': page_data.get('upstream_md5', ''),
                        'our_md5': page_data.get('our_md5', ''),
                        'md5_match': page_data.get('md5_match', ''),
                        'upstream_width': page_data.get('upstream_width', ''),
                        'upstream_height': page_data.get('upstream_height', ''),
                        'our_width': page_data.get('our_width', ''),
                        'our_height': page_data.get('our_height', ''),
                        'upstream_bytes': page_data.get('upstream_bytes', ''),
                        'our_bytes': page_data.get('our_bytes', ''),
                        'error': page_data.get('error', '')
                    })
                per_page_file.flush()

                if 'total_pages' in result:
                    total_pages += result['total_pages']
                if 'md5_matches' in result:
                    total_md5_matches += result['md5_matches']
                if 'md5_mismatches' in result:
                    total_md5_mismatches += result['md5_mismatches']

                status_icon = '✅' if result['match'] else '❌'
                match_str = f"MD5: {result.get('md5_matches', 0)}/{result.get('total_pages', 0)}" if 'md5_matches' in result else ''
                pages_str = f"({result.get('total_pages', 0)} pages)" if 'total_pages' in result else ''
                print(f"[{completed}/{total}] {status_icon} {result['pdf']} {pages_str} {match_str}", flush=True)

                if not result['match']:
                    failed += 1
                    print(f"         MISMATCH: {result.get('status', 'unknown')}", flush=True)
                    if 'error' in result:
                        print(f"         Error: {result['error'][:200]}", flush=True)
                    if result.get('md5_mismatches', 0) > 0:
                        print(f"         MD5 mismatches: {result['md5_mismatches']} pages", flush=True)

    # Close files
    csv_file.close()
    per_page_file.close()

    # Final summary
    end_time = datetime.now()
    duration = (end_time - start_time).total_seconds()

    print(f"\n{'='*80}", flush=True)
    print(f"VALIDATION COMPLETE", flush=True)
    print(f"{'='*80}", flush=True)
    print(f"Total PDFs: {completed}", flush=True)
    print(f"Passed: {completed - failed}", flush=True)
    print(f"Failed: {failed}", flush=True)
    print(f"Total pages: {total_pages}", flush=True)
    print(f"MD5 matches: {total_md5_matches}", flush=True)
    print(f"MD5 mismatches: {total_md5_mismatches}", flush=True)
    print(f"Duration: {duration:.1f}s ({duration/60:.1f} min)", flush=True)
    if total_pages > 0:
        print(f"Throughput: {total_pages/duration:.2f} pages/sec", flush=True)
    print(f"Results saved to: {results_csv}", flush=True)
    print(f"Per-page data: {per_page_csv}", flush=True)
    print(f"{'='*80}\n", flush=True)

    # Write JSON summary
    summary = {
        'timestamp': start_time.isoformat(),
        'duration_sec': duration,
        'total_pdfs': completed,
        'passed': completed - failed,
        'failed': failed,
        'total_pages': total_pages,
        'md5_matches': total_md5_matches,
        'md5_mismatches': total_md5_mismatches,
        'results': results
    }

    with open(results_file, 'w') as f:
        json.dump(summary, f, indent=2)

    # Exit code
    sys.exit(0 if failed == 0 else 1)


if __name__ == '__main__':
    main()
