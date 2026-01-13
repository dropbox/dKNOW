#!/usr/bin/env python3
"""
Test optimized validation script on small subset
Tests MD5-first optimization and progress output
"""

import subprocess
import sys
import json
from pathlib import Path
import time

def main():
    root = Path(__file__).parent
    validator_script = root / 'lib' / 'validate_all_images.py'

    print("=" * 80)
    print("TESTING OPTIMIZED VALIDATION SCRIPT")
    print("=" * 80)
    print()
    print("Test Parameters:")
    print("  - 3 small PDFs (100, 106, 109 pages = 315 pages total)")
    print("  - Workers: 2")
    print("  - Expected: MD5-first optimization active")
    print()

    # Run validation on first 3 PDFs only
    # We'll modify the script to accept --limit-pdfs option
    cmd = [
        sys.executable,
        str(validator_script),
        '--workers', '2',
        '--limit-pdfs', '3'  # Only process first 3 PDFs
    ]

    print(f"Command: {' '.join(cmd)}")
    print()
    print("=" * 80)
    print()

    start_time = time.time()

    # Run with real-time output
    proc = subprocess.Popen(
        cmd,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        bufsize=1  # Line buffered
    )

    # Stream output in real-time
    for line in proc.stdout:
        print(line, end='', flush=True)

    proc.wait()
    elapsed = time.time() - start_time

    print()
    print("=" * 80)
    print(f"Test completed in {elapsed:.1f} seconds")
    print(f"Exit code: {proc.returncode}")

    if proc.returncode == 0:
        print("✅ VALIDATION TEST PASSED")

        # Check for output files
        output_files = list(root.glob('telemetry/image_validation_all_*.json'))
        if output_files:
            latest = max(output_files, key=lambda p: p.stat().st_mtime)
            print(f"\nOutput file: {latest}")

            # Parse and show summary
            with open(latest, 'r') as f:
                data = json.load(f)

            print(f"\nValidation Summary:")
            print(f"  PDFs processed: {data.get('pdfs_processed', 0)}")
            print(f"  Total pages: {data.get('total_pages', 0)}")
            print(f"  MD5 matches: {data.get('md5_matches', 0)}")
            print(f"  SSIM computed: {data.get('ssim_computed', 0)}")
            print(f"  All match: {data.get('all_match', False)}")

            # Show MD5-first effectiveness
            md5_matches = data.get('md5_matches', 0)
            total_pages = data.get('total_pages', 0)
            if total_pages > 0:
                md5_rate = 100.0 * md5_matches / total_pages
                print(f"\n  MD5-first optimization: {md5_rate:.1f}% pages skipped SSIM")

        return 0
    else:
        print("❌ VALIDATION TEST FAILED")
        return 1

if __name__ == '__main__':
    sys.exit(main())
