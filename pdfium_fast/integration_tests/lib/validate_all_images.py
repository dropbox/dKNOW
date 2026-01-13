#!/usr/bin/env python3
"""
Validate ALL PDFs Against Upstream Image Rendering (100% Validation)

MANAGER ORDER: Prove EVERY PDF - not samples, ALL 452 PDFs, ALL pages

Compares Rust image rendering against upstream pdfium_test
for 100% of PDFs and 100% of pages.

This validates ~10,000+ pages total.

Usage:
    python3 lib/validate_all_images.py [--workers N] [--continue-from N] [--ssim-threshold 0.99]
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
from typing import Tuple, Dict
import numpy as np
from skimage import io, metrics, transform
from PIL import Image

class ImageValidator:
    def __init__(self, ssim_threshold=0.99):
        self.root = Path(__file__).parent.parent
        self.pdfium_root = self.root.parent
        self.ssim_threshold = ssim_threshold

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

        print(f"Loaded {len(self.pdfs)} PDFs from manifest")

    def compute_md5_file(self, file_path):
        """Compute MD5 of file"""
        md5 = hashlib.md5()
        with open(file_path, 'rb') as f:
            while chunk := f.read(8192):
                md5.update(chunk)
        return md5.hexdigest()

    def validate_image_rendering(self, pdf_info: dict) -> dict:
        """Compare upstream vs Rust image rendering for one PDF (all pages)
        Returns per-page data with MD5s and SSIM scores."""

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

                    # Compare using SSIM
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

                    # Compare all pages - store per-page data
                    pages_data = []
                    comparison_errors = 0

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
                            # Compute MD5s first (fast - ~10ms per page)
                            upstream_md5 = self.compute_md5_file(upstream_pngs[page_num])
                            our_md5 = self.compute_md5_file(our_pngs[page_num])

                            # Get file sizes
                            upstream_bytes = upstream_pngs[page_num].stat().st_size
                            our_bytes = our_pngs[page_num].stat().st_size

                            # OPTIMIZATION: If MD5s match, skip expensive SSIM computation
                            # MD5 match = byte-for-byte identical = SSIM = 1.0
                            if upstream_md5 == our_md5:
                                # Load images only to get dimensions (fast)
                                upstream_img = Image.open(upstream_pngs[page_num])
                                our_img = Image.open(our_pngs[page_num])
                                upstream_width, upstream_height = upstream_img.size
                                our_width, our_height = our_img.size

                                pages_data.append({
                                    'page': page_num + 1,
                                    'status': 'md5_match',
                                    'match': True,
                                    'ssim': 1.0,  # Perfect match implied by MD5
                                    'upstream_md5': upstream_md5,
                                    'our_md5': our_md5,
                                    'md5_match': True,
                                    'upstream_width': int(upstream_width),
                                    'upstream_height': int(upstream_height),
                                    'our_width': int(our_width),
                                    'our_height': int(our_height),
                                    'upstream_bytes': int(upstream_bytes),
                                    'our_bytes': int(our_bytes)
                                })
                                continue

                            # MD5s differ - need SSIM to quantify difference
                            # Load images and convert to RGB for comparison
                            upstream_img = Image.open(upstream_pngs[page_num]).convert('RGB')
                            our_img = Image.open(our_pngs[page_num]).convert('RGB')

                            # Get dimensions
                            upstream_width, upstream_height = upstream_img.size
                            our_width, our_height = our_img.size

                            # Convert to numpy arrays
                            upstream_arr = np.array(upstream_img)
                            our_arr = np.array(our_img)

                            # Handle dimension differences by resizing
                            if upstream_arr.shape != our_arr.shape:
                                our_arr = transform.resize(
                                    our_arr,
                                    upstream_arr.shape,
                                    preserve_range=True,
                                    anti_aliasing=True
                                ).astype(np.uint8)

                            # Compute SSIM (expensive - 1-2 seconds per page)
                            ssim_score = metrics.structural_similarity(
                                upstream_arr,
                                our_arr,
                                channel_axis=2,
                                data_range=255
                            )

                            # Per-page result for mismatched images
                            page_match = (ssim_score >= self.ssim_threshold)
                            pages_data.append({
                                'page': page_num + 1,
                                'status': 'ssim_compared',
                                'match': bool(page_match),
                                'ssim': float(ssim_score),
                                'upstream_md5': upstream_md5,
                                'our_md5': our_md5,
                                'md5_match': False,
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

                    # Calculate statistics
                    ssim_scores = [p['ssim'] for p in pages_data if 'ssim' in p]
                    if ssim_scores:
                        mean_ssim = float(np.mean(ssim_scores))
                        min_ssim = float(np.min(ssim_scores))
                        max_ssim = float(np.max(ssim_scores))
                    else:
                        mean_ssim = min_ssim = max_ssim = 0.0

                    # Match if all pages meet threshold
                    match = all(p.get('match', False) for p in pages_data)

                    return {
                        'pdf': pdf_name,
                        'status': 'compared',
                        'match': match,
                        'total_pages': len(upstream_pngs),
                        'compared_pages': len(ssim_scores),
                        'ssim_mean': mean_ssim,
                        'ssim_min': min_ssim,
                        'ssim_max': max_ssim,
                        'comparison_errors': comparison_errors,
                        'category': pdf_info.get('category', 'unknown'),
                        'pages': pdf_info.get('pages', 'unknown'),
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
    idx, pdf_info, ssim_threshold = args

    # Recreate validator in worker process
    validator = ImageValidator(ssim_threshold=ssim_threshold)
    result = validator.validate_image_rendering(pdf_info)
    result['index'] = idx
    return result


def main():
    # CRITICAL: Set multiprocessing start method to 'spawn' on macOS
    # Fork method hangs with numpy/scikit-image due to fork safety issues
    import multiprocessing
    try:
        multiprocessing.set_start_method('spawn')
    except RuntimeError:
        # Already set, ignore
        pass

    parser = argparse.ArgumentParser(description='Validate ALL PDFs images against upstream (100%)')
    parser.add_argument('--workers', type=int, default=2, help='Number of parallel workers (default: 2)')
    parser.add_argument('--continue-from', type=int, default=0, help='Continue from PDF index N')
    parser.add_argument('--limit-pdfs', type=int, default=None, help='Limit to first N PDFs (for testing)')
    parser.add_argument('--ssim-threshold', type=float, default=0.99, help='SSIM threshold for match (default: 0.99)')
    args = parser.parse_args()

    validator = ImageValidator(ssim_threshold=args.ssim_threshold)

    # Prepare output files
    timestamp = datetime.now().strftime('%Y%m%d_%H%M%S')
    results_file = validator.root / 'telemetry' / f'image_validation_all_{timestamp}.json'
    results_csv = validator.root / 'telemetry' / f'image_validation_all_{timestamp}.csv'
    per_page_csv = validator.root / 'telemetry' / f'image_validation_all_per_page_{timestamp}.csv'

    # CRITICAL: Open CSV files NOW and write incrementally (crash-safe)
    csv_file = open(results_csv, 'w', newline='')
    csv_writer = csv.DictWriter(csv_file, fieldnames=[
        'index', 'pdf', 'status', 'match', 'total_pages', 'compared_pages',
        'ssim_mean', 'ssim_min', 'ssim_max', 'comparison_errors',
        'category', 'pages', 'error'
    ])
    csv_writer.writeheader()
    csv_file.flush()

    per_page_file = open(per_page_csv, 'w', newline='')
    per_page_writer = csv.DictWriter(per_page_file, fieldnames=[
        'pdf', 'page', 'status', 'match', 'ssim',
        'upstream_md5', 'our_md5', 'md5_match',
        'upstream_width', 'upstream_height',
        'our_width', 'our_height',
        'upstream_bytes', 'our_bytes', 'error'
    ])
    per_page_writer.writeheader()
    per_page_file.flush()

    print(f"\n{'='*80}", flush=True)
    print(f"IMAGE VALIDATION - 100% OF PDFs, ALL PAGES", flush=True)
    print(f"{'='*80}", flush=True)
    print(f"Total PDFs: {len(validator.pdfs)}", flush=True)
    print(f"SSIM threshold: {args.ssim_threshold}", flush=True)
    print(f"Workers: {args.workers}", flush=True)
    print(f"Continue from: {args.continue_from}", flush=True)
    print(f"Upstream tool: {validator.pdfium_test}", flush=True)
    print(f"Rust tool: {validator.render_pages}", flush=True)
    print(f"Results: {results_file}", flush=True)
    print(f"{'='*80}\n", flush=True)
    print(f"WARNING: This will validate ~10,000+ pages. Estimated time: 0.5-2 hours (optimized)", flush=True)
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
                'ssim_mean': f"{result['ssim_mean']:.6f}" if 'ssim_mean' in result else '',
                'ssim_min': f"{result['ssim_min']:.6f}" if 'ssim_min' in result else '',
                'ssim_max': f"{result['ssim_max']:.6f}" if 'ssim_max' in result else '',
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
                    'ssim': f"{page_data['ssim']:.6f}" if 'ssim' in page_data else '',
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

            status_icon = '✅' if result['match'] else '❌'
            ssim_str = f"SSIM: {result.get('ssim_mean', 0):.4f}" if 'ssim_mean' in result else ''
            pages_str = f"({result.get('total_pages', 0)} pages)" if 'total_pages' in result else ''
            print(f"[{completed}/{total}] {status_icon} {result['pdf']} {pages_str} {ssim_str}", flush=True)

            if not result['match']:
                failed += 1
                print(f"         MISMATCH: {result.get('status', 'unknown')}", flush=True)
                if 'error' in result:
                    print(f"         Error: {result['error'][:200]}", flush=True)
                if result.get('low_similarity_pages', 0) > 0:
                    print(f"         Low similarity: {result['low_similarity_pages']} pages", flush=True)
    else:
        # Parallel execution
        worker_args = [
            (idx, pdf_info, args.ssim_threshold)
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
                    'ssim_mean': f"{result['ssim_mean']:.6f}" if 'ssim_mean' in result else '',
                    'ssim_min': f"{result['ssim_min']:.6f}" if 'ssim_min' in result else '',
                    'ssim_max': f"{result['ssim_max']:.6f}" if 'ssim_max' in result else '',
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
                        'ssim': f"{page_data['ssim']:.6f}" if 'ssim' in page_data else '',
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

                status_icon = '✅' if result['match'] else '❌'
                ssim_str = f"SSIM: {result.get('ssim_mean', 0):.4f}" if 'ssim_mean' in result else ''
                pages_str = f"({result.get('total_pages', 0)} pages)" if 'total_pages' in result else ''
                print(f"[{completed}/{total}] {status_icon} {result['pdf']} {pages_str} {ssim_str}", flush=True)

                if not result['match']:
                    failed += 1
                    print(f"         MISMATCH: {result.get('status', 'unknown')}", flush=True)
                    if 'error' in result:
                        print(f"         Error: {result['error'][:200]}", flush=True)
                    if result.get('low_similarity_pages', 0) > 0:
                        print(f"         Low similarity: {result['low_similarity_pages']} pages", flush=True)

    end_time = datetime.now()
    duration = (end_time - start_time).total_seconds()

    # Sort results by index for consistent ordering
    results.sort(key=lambda x: x['index'])

    # Close CSV files (data already written incrementally)
    csv_file.close()
    per_page_file.close()

    # Calculate MD5 statistics
    md5_matches = 0
    ssim_computed = 0
    for r in results:
        for page_data in r.get('pages_data', []):
            if page_data.get('status') == 'md5_match':
                md5_matches += 1
            elif page_data.get('status') == 'ssim_compared':
                ssim_computed += 1

    # Save final JSON summary (optional - CSV files are complete)
    with open(results_file, 'w') as f:
        json.dump({
            'timestamp': start_time.isoformat(),
            'duration_seconds': duration,
            'total_pdfs': len(validator.pdfs),
            'tested_pdfs': len(results),
            'total_pages': total_pages,
            'md5_matches': md5_matches,
            'ssim_computed': ssim_computed,
            'ssim_threshold': args.ssim_threshold,
            'continue_from': args.continue_from,
            'workers': args.workers,
            'upstream_tool': str(validator.pdfium_test),
            'rust_tool': str(validator.render_pages),
            'results': results
        }, f, indent=2)

    # Print summary
    print(f"\n{'='*80}")
    print(f"VALIDATION COMPLETE")
    print(f"{'='*80}")
    print(f"Duration: {duration:.1f} seconds ({duration/60:.1f} minutes, {duration/3600:.1f} hours)")
    print(f"Tested: {len(results)}/{len(validator.pdfs)} PDFs")
    print(f"Total pages compared: {total_pages:,}")
    print(f"")

    matched = sum(1 for r in results if r['match'])
    if len(results) > 0:
        print(f"✅ Matched: {matched}/{len(results)} PDFs ({100*matched/len(results):.1f}%)")
        print(f"❌ Failed:  {failed}/{len(results)} PDFs ({100*failed/len(results):.1f}%)")
    else:
        print(f"✅ Matched: 0/0 (no PDFs tested)")
        print(f"❌ Failed:  0/0 (no PDFs tested)")

    if failed > 0:
        print(f"\n{'='*80}")
        print(f"FAILURES (showing first 20)")
        print(f"{'='*80}")
        failure_count = 0
        for r in results:
            if not r['match']:
                print(f"\n{r['pdf']}")
                print(f"  Status: {r.get('status', 'unknown')}")
                if 'total_pages' in r:
                    print(f"  Pages: {r['total_pages']}")
                if 'ssim_mean' in r:
                    print(f"  SSIM: mean={r['ssim_mean']:.4f}, min={r['ssim_min']:.4f}")
                if 'error' in r:
                    print(f"  Error: {r['error'][:200]}")
                if r.get('low_similarity_pages', 0) > 0:
                    print(f"  Low similarity: {r['low_similarity_pages']} pages")

                failure_count += 1
                if failure_count >= 20:
                    remaining = failed - 20
                    if remaining > 0:
                        print(f"\n... and {remaining} more failures")
                    break

        print(f"\n{'='*80}")
        print(f"100% CORRECTNESS: ❌ FAILED")
        print(f"{'='*80}")
        print(f"Required: 452/452 PDFs match (SSIM ≥ {args.ssim_threshold})")
        print(f"Actual: {matched}/452 PDFs match")
        print(f"\nSee: {results_file}")
        return 1
    else:
        print(f"\n{'='*80}")
        print(f"100% CORRECTNESS: ✅ VERIFIED")
        print(f"{'='*80}")
        print(f"All {len(results)} PDFs produce perceptually identical image rendering")
        print(f"All {total_pages:,} pages meet SSIM threshold of {args.ssim_threshold}")
        print(f"Rust implementation matches upstream pdfium_test")
        print(f"\nResults saved to:")
        print(f"  JSON: {results_file}")
        print(f"  PDF CSV:  {results_csv}")
        print(f"  Per-page CSV: {per_page_csv}")
        return 0


if __name__ == '__main__':
    sys.exit(main())
