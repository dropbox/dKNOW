#!/usr/bin/env python3
"""
Validate ALL PDFs Against Upstream Text Extraction (100% Validation)

MANAGER ORDER: Prove EVERY PDF - not samples, ALL 452 PDFs

Compares Rust text extraction against C++ reference implementation
for 100% of PDFs in the test suite.

Usage:
    python3 lib/validate_all_text.py [--workers N] [--continue-from N]
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

class TextValidator:
    def __init__(self):
        self.root = Path(__file__).parent.parent
        self.pdfium_root = self.root.parent

        # C++ reference tool
        self.cpp_text_tool = self.pdfium_root / 'out' / 'Optimized-Shared' / 'reference_text_extract'

        # Rust tool
        self.rust_text_tool = self.pdfium_root / 'rust' / 'target' / 'release' / 'examples' / 'extract_text'

        # Verify tools exist
        for tool in [self.cpp_text_tool, self.rust_text_tool]:
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
                    self.pdfs.append({
                        'name': row['pdf_name'],
                        'path': pdf_path,
                        'category': row.get('pdf_category', 'unknown'),
                        'pages': row.get('pdf_pages', 'unknown')
                    })

        print(f"Loaded {len(self.pdfs)} PDFs from manifest")

    def compute_md5_file(self, file_path):
        """Compute MD5 of file"""
        md5 = hashlib.md5()
        with open(file_path, 'rb') as f:
            while chunk := f.read(8192):
                md5.update(chunk)
        return md5.hexdigest()

    def validate_text_extraction(self, pdf_info: dict) -> dict:
        """Compare C++ reference vs Rust text extraction for one PDF"""

        pdf_path = pdf_info['path']
        pdf_name = pdf_info['name']

        with tempfile.NamedTemporaryFile(suffix='.txt', delete=False) as cpp_out:
            cpp_output_path = cpp_out.name

        with tempfile.NamedTemporaryFile(suffix='.txt', delete=False) as rust_out:
            rust_output_path = rust_out.name

        try:
            # Set library path for both C++ and Rust tools
            env = os.environ.copy()
            env['DYLD_LIBRARY_PATH'] = str(self.pdfium_root / 'out' / 'Optimized-Shared')

            # Run C++ reference tool
            cpp_result = subprocess.run(
                [str(self.cpp_text_tool), str(pdf_path), cpp_output_path],
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

            # Run Rust tool (single-threaded for correctness validation)
            rust_result = subprocess.run(
                [str(self.rust_text_tool), str(pdf_path), rust_output_path, '1'],
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

            # Compare byte-for-byte
            cpp_data = Path(cpp_output_path).read_bytes()
            rust_data = Path(rust_output_path).read_bytes()

            cpp_md5 = self.compute_md5_file(cpp_output_path)
            rust_md5 = self.compute_md5_file(rust_output_path)

            match = (cpp_data == rust_data)

            result = {
                'pdf': pdf_name,
                'status': 'compared',
                'match': match,
                'cpp_bytes': len(cpp_data),
                'rust_bytes': len(rust_data),
                'cpp_md5': cpp_md5,
                'rust_md5': rust_md5,
                'category': pdf_info.get('category', 'unknown'),
                'pages': pdf_info.get('pages', 'unknown')
            }

            if not match:
                # Find first difference
                for idx in range(min(len(cpp_data), len(rust_data))):
                    if cpp_data[idx] != rust_data[idx]:
                        result['diff_byte'] = idx
                        result['diff_cpp'] = f"{cpp_data[idx]:02x}"
                        result['diff_rust'] = f"{rust_data[idx]:02x}"
                        break

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
    idx, pdf_info, validator_config = args

    # Recreate validator in worker process
    validator = TextValidator()
    result = validator.validate_text_extraction(pdf_info)
    result['index'] = idx
    return result


def main():
    parser = argparse.ArgumentParser(description='Validate ALL PDFs against C++ reference (100%)')
    parser.add_argument('--workers', type=int, default=4, help='Number of parallel workers (default: 4)')
    parser.add_argument('--continue-from', type=int, default=0, help='Continue from PDF index N')
    parser.add_argument('--save-failures', action='store_true', help='Save failed comparison outputs')
    args = parser.parse_args()

    validator = TextValidator()

    # Prepare output files
    timestamp = datetime.now().strftime('%Y%m%d_%H%M%S')
    results_file = validator.root / 'telemetry' / f'text_validation_all_{timestamp}.json'
    results_csv = validator.root / 'telemetry' / f'text_validation_all_{timestamp}.csv'

    print(f"\n{'='*80}")
    print(f"TEXT VALIDATION - 100% OF PDFs")
    print(f"{'='*80}")
    print(f"Total PDFs: {len(validator.pdfs)}")
    print(f"Workers: {args.workers}")
    print(f"Continue from: {args.continue_from}")
    print(f"C++ tool: {validator.cpp_text_tool}")
    print(f"Rust tool: {validator.rust_text_tool}")
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
            result = validator.validate_text_extraction(pdf_info)
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
    else:
        # Parallel execution
        worker_args = [
            (idx, pdf_info, None)
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

    end_time = datetime.now()
    duration = (end_time - start_time).total_seconds()

    # Sort results by index for consistent ordering
    results.sort(key=lambda x: x['index'])

    # Save results
    with open(results_file, 'w') as f:
        json.dump({
            'timestamp': start_time.isoformat(),
            'duration_seconds': duration,
            'total_pdfs': len(validator.pdfs),
            'tested_pdfs': len(results),
            'continue_from': args.continue_from,
            'workers': args.workers,
            'cpp_tool': str(validator.cpp_text_tool),
            'rust_tool': str(validator.rust_text_tool),
            'results': results
        }, f, indent=2)

    # Save CSV for easy analysis
    with open(results_csv, 'w', newline='') as f:
        writer = csv.DictWriter(f, fieldnames=[
            'index', 'pdf', 'status', 'match', 'cpp_bytes', 'rust_bytes',
            'cpp_md5', 'rust_md5', 'category', 'pages', 'error'
        ])
        writer.writeheader()
        for r in results:
            writer.writerow({
                'index': r.get('index', 0),
                'pdf': r['pdf'],
                'status': r.get('status', 'unknown'),
                'match': r['match'],
                'cpp_bytes': r.get('cpp_bytes', ''),
                'rust_bytes': r.get('rust_bytes', ''),
                'cpp_md5': r.get('cpp_md5', ''),
                'rust_md5': r.get('rust_md5', ''),
                'category': r.get('category', ''),
                'pages': r.get('pages', ''),
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
                if 'cpp_bytes' in r and 'rust_bytes' in r:
                    print(f"  Size: C++={r['cpp_bytes']:,}B Rust={r['rust_bytes']:,}B")
                if 'error' in r:
                    print(f"  Error: {r['error'][:200]}")
                if 'diff_byte' in r:
                    print(f"  First diff at byte {r['diff_byte']}: C++={r['diff_cpp']} Rust={r['diff_rust']}")

                failure_count += 1
                if failure_count >= 20:
                    remaining = failed - 20
                    if remaining > 0:
                        print(f"\n... and {remaining} more failures")
                    break

        print(f"\n{'='*80}")
        print(f"100% CORRECTNESS: ❌ FAILED")
        print(f"{'='*80}")
        print(f"Required: 452/452 match")
        print(f"Actual: {matched}/452 match")
        print(f"\nSee: {results_file}")
        return 1
    else:
        print(f"\n{'='*80}")
        print(f"100% CORRECTNESS: ✅ VERIFIED")
        print(f"{'='*80}")
        print(f"All {len(results)} PDFs produce byte-for-byte identical text extraction")
        print(f"Rust implementation matches C++ reference implementation")
        print(f"\nResults saved to:")
        print(f"  JSON: {results_file}")
        print(f"  CSV:  {results_csv}")
        return 0


if __name__ == '__main__':
    sys.exit(main())
