#!/usr/bin/env python3
"""
BGR vs BGRA Speedup Benchmark (v1.9.0)

Measures performance improvement from BGR mode (3 bytes) vs BGRA mode (4 bytes).

Usage:
    python3 benchmark_bgr_speedup.py

Output:
    - BGR baseline (default, automatic format selection)
    - BGRA baseline (--force-alpha, always 4 bytes)
    - Speedup ratio
"""

import subprocess
import time
import tempfile
import shutil
from pathlib import Path

# Configuration
CLI_PATH = Path(__file__).parent.parent / "out" / "Release" / "pdfium_cli"
PDFS_DIR = Path(__file__).parent / "pdfs" / "benchmark"
ITERATIONS = 5
THREAD_COUNT = 4  # Use multi-threaded for realistic performance

# Select test PDF (100-page for good measurement)
TEST_PDF = PDFS_DIR / "0100pages_7FKQLKX273JBHXAAW5XDRT27JGMIZMCI.pdf"

def benchmark_render(force_alpha: bool) -> float:
    """
    Benchmark rendering with BGR or BGRA mode.

    Args:
        force_alpha: If True, use --force-alpha (BGRA mode)

    Returns:
        Average time in seconds over ITERATIONS runs
    """
    times = []

    for i in range(ITERATIONS):
        with tempfile.TemporaryDirectory() as tmpdir:
            cmd = [
                str(CLI_PATH),
                "--threads", str(THREAD_COUNT),
                "--benchmark",  # Skip file writes (measure rendering only)
            ]

            if force_alpha:
                cmd.append("--force-alpha")

            cmd.extend([
                "render-pages",
                str(TEST_PDF),
                tmpdir
            ])

            start = time.time()
            result = subprocess.run(cmd, capture_output=True, text=True)
            end = time.time()

            if result.returncode != 0:
                print(f"Error: Command failed: {result.stderr}")
                return None

            elapsed = end - start
            times.append(elapsed)
            print(f"  Iteration {i+1}/{ITERATIONS}: {elapsed:.3f}s", end="\r")

    print()  # Clear line
    return sum(times) / len(times)

def main():
    print("BGR vs BGRA Speedup Benchmark (v1.9.0)")
    print("=" * 60)
    print(f"Test PDF: {TEST_PDF.name}")
    print(f"Threads: {THREAD_COUNT}")
    print(f"Iterations: {ITERATIONS}")
    print(f"Mode: benchmark (skip file writes)")
    print()

    if not TEST_PDF.exists():
        print(f"Error: Test PDF not found: {TEST_PDF}")
        return 1

    # Benchmark BGR mode (default)
    print("Benchmarking BGR mode (default, 3 bytes for opaque pages)...")
    bgr_time = benchmark_render(force_alpha=False)
    if bgr_time is None:
        return 1
    print(f"BGR mode: {bgr_time:.3f}s (average)")
    print()

    # Benchmark BGRA mode (--force-alpha)
    print("Benchmarking BGRA mode (--force-alpha, 4 bytes always)...")
    bgra_time = benchmark_render(force_alpha=True)
    if bgra_time is None:
        return 1
    print(f"BGRA mode: {bgra_time:.3f}s (average)")
    print()

    # Calculate speedup
    speedup = bgra_time / bgr_time
    improvement_pct = (speedup - 1.0) * 100

    print("Results:")
    print("=" * 60)
    print(f"BGR mode:  {bgr_time:.3f}s (3 bytes per pixel)")
    print(f"BGRA mode: {bgra_time:.3f}s (4 bytes per pixel)")
    print(f"Speedup:   {speedup:.3f}x ({improvement_pct:+.1f}%)")
    print()

    # Theoretical explanation
    memory_reduction = (1 - 3/4) * 100
    print("Theory:")
    print(f"  Memory bandwidth reduction: {memory_reduction:.1f}% (3 bytes vs 4 bytes)")
    print(f"  Expected speedup: ~1.05-1.10x (memory-bound workload)")
    print(f"  Actual speedup: {speedup:.3f}x")
    print()

    if speedup < 1.02:
        print("Note: Small speedup suggests memory bandwidth is not the bottleneck.")
        print("      Cache efficiency and computation may dominate.")
    elif speedup > 1.15:
        print("Note: High speedup suggests memory bandwidth was a significant bottleneck.")

    return 0

if __name__ == "__main__":
    exit(main())
