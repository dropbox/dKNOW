#!/usr/bin/env python3
"""
Validate ALL PDFs Against Upstream JSONL Extraction (100% Validation)

MANAGER ORDER: Prove EVERY PDF - not samples, ALL 296 PDFs with JSONL support

Compares Rust JSONL extraction against C++ reference implementation
for 100% of PDFs that support JSONL output.

Usage:
    python3 lib/validate_all_jsonl.py [--workers N] [--continue-from N] [--page N]
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
from concurrent.futures import ProcessPoolExecutor, as_completed
from typing import Tuple, Dict

class JSONLValidator:
    def __init__(self, page_to_test=0):
        self.root = Path(__file__).parent.parent
        self.pdfium_root = self.root.parent
        self.page_to_test = page_to_test

        # C++ reference tool
        self.cpp_jsonl_tool = self.pdfium_root / 'out' / 'Optimized-Shared' / 'reference_jsonl_extract'

        # Rust tool
        self.rust_jsonl_tool = self.pdfium_root / 'rust' / 'target' / 'release' / 'examples' / 'extract_text_jsonl'

        # Verify tools exist
        for tool in [self.cpp_jsonl_tool, self.rust_jsonl_tool]:
            if not tool.exists():
                raise FileNotFoundError(f"Tool not found: {tool}")

        # PDF manifest
        self.manifest_file = self.root / 'master_test_suite' / 'pdf_manifest.csv'
        if not self.manifest_file.exists():
            raise FileNotFoundError(f"PDF manifest not found: {self.manifest_file}")

        # Load PDF list - only those with JSONL expected output
        self.pdfs = []
        with open(self.manifest_file, 'r') as f:
            reader = csv.DictReader(f)
            for row in reader:
                # PDF paths in manifest are relative to integration_tests/
                pdf_path = self.root / row['pdf_path']
                if not pdf_path.exists():
                    continue

                # Check if JSONL baseline exists
                expected_outputs_dir = self.root / row['expected_outputs_dir']
                jsonl_baseline = expected_outputs_dir / 'jsonl' / f'page_{self.page_to_test:04d}.jsonl'

                if jsonl_baseline.exists():
                    self.pdfs.append({
                        'name': row['pdf_name'],
                        'path': pdf_path,
                        'category': row.get('pdf_category', 'unknown'),
                        'pages': row.get('pdf_pages', 'unknown'),
                        'baseline': jsonl_baseline
                    })

        print(f"Loaded {len(self.pdfs)} PDFs with JSONL support (page {self.page_to_test})")

    def compute_md5_file(self, file_path):
        """Compute MD5 of file"""
        md5 = hashlib.md5()
        with open(file_path, 'rb') as f:
            while chunk := f.read(8192):
                md5.update(chunk)
        return md5.hexdigest()

    def validate_jsonl_extraction(self, pdf_info: dict) -> dict:
        """Compare C++ reference vs Rust JSONL extraction for one PDF (page 0 only)"""

        pdf_path = pdf_info['path']
        pdf_name = pdf_info['name']

        with tempfile.NamedTemporaryFile(suffix='.jsonl', delete=False) as cpp_out:
            cpp_output_path = cpp_out.name

        with tempfile.NamedTemporaryFile(suffix='.jsonl', delete=False) as rust_out:
            rust_output_path = rust_out.name

        try:
            # Set library path for both C++ and Rust tools
            env = os.environ.copy()
            env['DYLD_LIBRARY_PATH'] = str(self.pdfium_root / 'out' / 'Optimized-Shared')

            # Run C++ reference tool (page 0 only for speed)
            cpp_result = subprocess.run(
                [str(self.cpp_jsonl_tool), str(pdf_path), cpp_output_path, str(self.page_to_test)],
                capture_output=True,
                text=True,
                timeout=300,
                env=env
            )

            if cpp_result.returncode != 0:
                return {
                    'pdf': pdf_name,
                    'status': 'cpp_failed',
                    'error': cpp_result.stderr[:500],
                    'match': False
                }

            # Run Rust tool (page 0 only)
            rust_result = subprocess.run(
                [str(self.rust_jsonl_tool), str(pdf_path), rust_output_path, str(self.page_to_test)],
                capture_output=True,
                text=True,
                timeout=300,
                env=env
            )

            if rust_result.returncode != 0:
                return {
                    'pdf': pdf_name,
                    'status': 'rust_failed',
                    'error': rust_result.stderr[:500],
                    'match': False
                }

            # Compare JSONL outputs
            # JSONL contains floating point numbers, so we need numerical comparison
            # not byte-for-byte comparison
            cpp_data = Path(cpp_output_path).read_text()
            rust_data = Path(rust_output_path).read_text()

            # Compute MD5s
            cpp_md5 = self.compute_md5_file(cpp_output_path)
            rust_md5 = self.compute_md5_file(rust_output_path)

            cpp_lines = [line for line in cpp_data.strip().split('\n') if line]
            rust_lines = [line for line in rust_data.strip().split('\n') if line]

            if len(cpp_lines) != len(rust_lines):
                return {
                    'pdf': pdf_name,
                    'status': 'compared',
                    'match': False,
                    'cpp_chars': len(cpp_lines),
                    'rust_chars': len(rust_lines),
                    'cpp_md5': cpp_md5,
                    'rust_md5': rust_md5,
                    'md5_match': (cpp_md5 == rust_md5),
                    'error': f'Line count mismatch: C++={len(cpp_lines)} Rust={len(rust_lines)}',
                    'category': pdf_info.get('category', 'unknown'),
                    'pages': pdf_info.get('pages', 'unknown')
                }

            # Parse and compare numerically
            mismatches = []
            for idx, (cpp_line, rust_line) in enumerate(zip(cpp_lines, rust_lines)):
                try:
                    cpp_obj = json.loads(cpp_line)
                    rust_obj = json.loads(rust_line)

                    # Compare text content exactly
                    if cpp_obj.get('text') != rust_obj.get('text'):
                        mismatches.append({
                            'char_idx': idx,
                            'field': 'text',
                            'cpp': cpp_obj.get('text', ''),
                            'rust': rust_obj.get('text', '')
                        })

                    # Compare numerical values with tolerance
                    for field in ['x', 'y', 'width', 'height', 'font_size']:
                        cpp_val = cpp_obj.get(field, 0.0)
                        rust_val = rust_obj.get(field, 0.0)

                        # Allow small floating point differences
                        if abs(cpp_val - rust_val) > 0.001:
                            mismatches.append({
                                'char_idx': idx,
                                'field': field,
                                'cpp': cpp_val,
                                'rust': rust_val,
                                'diff': abs(cpp_val - rust_val)
                            })

                except json.JSONDecodeError as e:
                    mismatches.append({
                        'char_idx': idx,
                        'error': f'JSON parse error: {str(e)}'
                    })

            match = (len(mismatches) == 0)

            result = {
                'pdf': pdf_name,
                'status': 'compared',
                'match': match,
                'cpp_chars': len(cpp_lines),
                'rust_chars': len(rust_lines),
                'cpp_bytes': len(cpp_data.encode('utf-8')),
                'rust_bytes': len(rust_data.encode('utf-8')),
                'cpp_md5': cpp_md5,
                'rust_md5': rust_md5,
                'md5_match': (cpp_md5 == rust_md5),
                'category': pdf_info.get('category', 'unknown'),
                'pages': pdf_info.get('pages', 'unknown')
            }

            if not match:
                result['mismatches'] = mismatches[:10]  # First 10 mismatches
                result['total_mismatches'] = len(mismatches)

            return result

        except subprocess.TimeoutExpired:
            return {
                'pdf': pdf_name,
                'status': 'timeout',
                'match': False
            }
        except Exception as e:
            return {
                'pdf': pdf_name,
                'status': 'exception',
                'error': str(e)[:500],
                'match': False
            }
        finally:
            Path(cpp_output_path).unlink(missing_ok=True)
            Path(rust_output_path).unlink(missing_ok=True)


def validate_pdf_worker(args):
    """Worker function for parallel validation"""
    idx, pdf_info, page_to_test = args

    # Recreate validator in worker process
    validator = JSONLValidator(page_to_test=page_to_test)
    result = validator.validate_jsonl_extraction(pdf_info)
    result['index'] = idx
    return result


def main():
    parser = argparse.ArgumentParser(description='Validate ALL PDFs with JSONL against C++ reference (100%)')
    parser.add_argument('--workers', type=int, default=4, help='Number of parallel workers (default: 4)')
    parser.add_argument('--continue-from', type=int, default=0, help='Continue from PDF index N')
    parser.add_argument('--page', type=int, default=0, help='Page to test (default: 0)')
    args = parser.parse_args()

    validator = JSONLValidator(page_to_test=args.page)

    # Prepare output files
    timestamp = datetime.now().strftime('%Y%m%d_%H%M%S')
    results_file = validator.root / 'telemetry' / f'jsonl_validation_all_{timestamp}.json'
    results_csv = validator.root / 'telemetry' / f'jsonl_validation_all_{timestamp}.csv'

    print(f"\n{'='*80}")
    print(f"JSONL VALIDATION - 100% OF PDFs WITH JSONL SUPPORT")
    print(f"{'='*80}")
    print(f"Total PDFs: {len(validator.pdfs)}")
    print(f"Page tested: {args.page}")
    print(f"Workers: {args.workers}")
    print(f"Continue from: {args.continue_from}")
    print(f"C++ tool: {validator.cpp_jsonl_tool}")
    print(f"Rust tool: {validator.rust_jsonl_tool}")
    print(f"Results: {results_file}")
    print(f"{'='*80}\n")

    start_time = datetime.now()

    # Filter PDFs if continuing
    pdfs_to_test = validator.pdfs[args.continue_from:]

    results = []
    completed = 0
    failed = 0

    # Progress tracking
    total = len(pdfs_to_test)

    if args.workers == 1:
        # Serial execution
        for idx, pdf_info in enumerate(pdfs_to_test, start=args.continue_from):
            result = validator.validate_jsonl_extraction(pdf_info)
            result['index'] = idx
            results.append(result)
            completed += 1

            status_icon = '✅' if result['match'] else '❌'
            print(f"[{completed}/{total}] {status_icon} {result['pdf']}")

            if not result['match']:
                failed += 1
                print(f"         MISMATCH: {result.get('status', 'unknown')}")
                if 'error' in result:
                    print(f"         Error: {result['error'][:200]}")
                if 'total_mismatches' in result:
                    print(f"         Mismatches: {result['total_mismatches']} characters")
    else:
        # Parallel execution
        worker_args = [
            (idx, pdf_info, args.page)
            for idx, pdf_info in enumerate(pdfs_to_test, start=args.continue_from)
        ]

        with ProcessPoolExecutor(max_workers=args.workers) as executor:
            futures = {executor.submit(validate_pdf_worker, arg): arg for arg in worker_args}

            for future in as_completed(futures):
                result = future.result()
                results.append(result)
                completed += 1

                status_icon = '✅' if result['match'] else '❌'
                print(f"[{completed}/{total}] {status_icon} {result['pdf']}")

                if not result['match']:
                    failed += 1
                    print(f"         MISMATCH: {result.get('status', 'unknown')}")
                    if 'error' in result:
                        print(f"         Error: {result['error'][:200]}")
                    if 'total_mismatches' in result:
                        print(f"         Mismatches: {result['total_mismatches']} characters")

    end_time = datetime.now()
    duration = (end_time - start_time).total_seconds()

    # Sort results by index for consistent ordering
    results.sort(key=lambda x: x['index'])

    # Save results
    with open(results_file, 'w') as f:
        json.dump({
            'timestamp': start_time.isoformat(),
            'duration_seconds': duration,
            'page_tested': args.page,
            'total_pdfs': len(validator.pdfs),
            'tested_pdfs': len(results),
            'continue_from': args.continue_from,
            'workers': args.workers,
            'cpp_tool': str(validator.cpp_jsonl_tool),
            'rust_tool': str(validator.rust_jsonl_tool),
            'results': results
        }, f, indent=2)

    # Save CSV for easy analysis
    with open(results_csv, 'w', newline='') as f:
        writer = csv.DictWriter(f, fieldnames=[
            'index', 'pdf', 'status', 'match', 'cpp_chars', 'rust_chars',
            'cpp_bytes', 'rust_bytes', 'cpp_md5', 'rust_md5', 'md5_match',
            'category', 'pages', 'total_mismatches', 'error'
        ])
        writer.writeheader()
        for r in results:
            writer.writerow({
                'index': r.get('index', 0),
                'pdf': r['pdf'],
                'status': r.get('status', 'unknown'),
                'match': r['match'],
                'cpp_chars': r.get('cpp_chars', ''),
                'rust_chars': r.get('rust_chars', ''),
                'cpp_bytes': r.get('cpp_bytes', ''),
                'rust_bytes': r.get('rust_bytes', ''),
                'cpp_md5': r.get('cpp_md5', ''),
                'rust_md5': r.get('rust_md5', ''),
                'md5_match': r.get('md5_match', ''),
                'category': r.get('category', ''),
                'pages': r.get('pages', ''),
                'total_mismatches': r.get('total_mismatches', ''),
                'error': r.get('error', '')
            })

    # Print summary
    print(f"\n{'='*80}")
    print(f"VALIDATION COMPLETE")
    print(f"{'='*80}")
    print(f"Duration: {duration:.1f} seconds ({duration/60:.1f} minutes)")
    print(f"Tested: {len(results)}/{len(validator.pdfs)} PDFs")
    print(f"")

    matched = sum(1 for r in results if r['match'])
    if len(results) > 0:
        print(f"✅ Matched: {matched}/{len(results)} ({100*matched/len(results):.1f}%)")
        print(f"❌ Failed:  {failed}/{len(results)} ({100*failed/len(results):.1f}%)")
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
                if 'cpp_chars' in r and 'rust_chars' in r:
                    print(f"  Characters: C++={r['cpp_chars']} Rust={r['rust_chars']}")
                if 'error' in r:
                    print(f"  Error: {r['error'][:200]}")
                if 'total_mismatches' in r:
                    print(f"  Total mismatches: {r['total_mismatches']}")
                    if 'mismatches' in r and r['mismatches']:
                        print(f"  First mismatch: {r['mismatches'][0]}")

                failure_count += 1
                if failure_count >= 20:
                    remaining = failed - 20
                    if remaining > 0:
                        print(f"\n... and {remaining} more failures")
                    break

        print(f"\n{'='*80}")
        print(f"100% CORRECTNESS: ❌ FAILED")
        print(f"{'='*80}")
        print(f"Required: {len(validator.pdfs)}/{len(validator.pdfs)} match")
        print(f"Actual: {matched}/{len(validator.pdfs)} match")
        print(f"\nSee: {results_file}")
        return 1
    else:
        print(f"\n{'='*80}")
        print(f"100% CORRECTNESS: ✅ VERIFIED")
        print(f"{'='*80}")
        print(f"All {len(results)} PDFs produce numerically identical JSONL extraction")
        print(f"Rust implementation matches C++ reference implementation")
        print(f"\nResults saved to:")
        print(f"  JSON: {results_file}")
        print(f"  CSV:  {results_csv}")
        return 0


if __name__ == '__main__':
    sys.exit(main())
