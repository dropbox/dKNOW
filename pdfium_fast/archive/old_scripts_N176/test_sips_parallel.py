#!/usr/bin/env python3
"""Test if sips command serializes when run in parallel"""
import subprocess
import tempfile
import time
from pathlib import Path
import shutil
from concurrent.futures import ProcessPoolExecutor
import os
import sys

def convert_one(ppm_path):
    """Convert one PPM to PNG"""
    subprocess.run(
        ['sips', '-s', 'format', 'png', str(ppm_path), '--out', str(ppm_path.with_suffix('.png'))],
        check=True,
        capture_output=True,
        timeout=60
    )

def main():
    root = Path('/Users/ayates/pdfium')
    pdfium_test = root / 'out/Optimized-Shared/pdfium_test'
    pdf_path = root / 'integration_tests/pdfs/benchmark/0100pages_7FKQLKX273JBHXAAW5XDRT27JGMIZMCI.pdf'

    with tempfile.TemporaryDirectory() as tmpdir:
        tmpdir = Path(tmpdir)
        test_pdf = tmpdir / pdf_path.name
        shutil.copy2(pdf_path, test_pdf)

        # Generate PPMs
        env = os.environ.copy()
        env['DYLD_LIBRARY_PATH'] = str(pdfium_test.parent)
        subprocess.run(
            [str(pdfium_test), '--ppm', '--scale=4.166666', pdf_path.name],
            capture_output=True,
            env=env,
            timeout=60,
            cwd=tmpdir
        )

        ppm_files = sorted(tmpdir.glob('*.ppm'))[:20]
        print(f"Created {len(ppm_files)} PPM files for testing")

        # Test 1: Sequential
        print(f"\n=== Sequential (1 at a time) ===")
        t0 = time.time()
        for ppm in ppm_files:
            subprocess.run(
                ['sips', '-s', 'format', 'png', str(ppm), '--out', str(ppm.with_suffix('.png'))],
                check=True,
                capture_output=True,
                timeout=60
            )
        t1 = time.time()
        seq_time = t1-t0
        print(f"20 files: {seq_time:.2f}s ({seq_time/20:.3f}s per file)")

        # Clean up PNGs
        for png in tmpdir.glob('*.png'):
            png.unlink()

        # Test 2: Parallel (4 workers)
        print(f"\n=== Parallel (4 workers) ===")
        t0 = time.time()
        with ProcessPoolExecutor(max_workers=4) as executor:
            list(executor.map(convert_one, ppm_files))
        t1 = time.time()
        par_time = t1-t0
        print(f"20 files: {par_time:.2f}s ({par_time/20:.3f}s per file)")
        print(f"Speedup: {seq_time/par_time:.2f}x")

        if par_time >= seq_time * 0.8:
            print(f"\n⚠️  WARNING: Parallel sips shows NO speedup!")
            print(f"This explains the 86 sec/page validation bottleneck.")

if __name__ == '__main__':
    main()
