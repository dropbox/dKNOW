#!/usr/bin/env python3
"""
Benchmark REAL corpus performance (Task 3 from MANAGER directive)
Measures combined PNG+threading speedup on production PDFs from benchmark/ directory
"""
import json
import glob
import subprocess
import tempfile
import time
from pathlib import Path
import statistics
import random

def extract_pages_from_filename(filename):
    """Extract page count from filename like '0100pages_HASH.pdf'"""
    try:
        return int(filename.split('pages_')[0].lstrip('0') or '0')
    except:
        return None

def find_benchmark_pdfs():
    """Find all PDFs in benchmark/ directory"""
    pdfs = []
    for pdf_path in Path("pdfs/benchmark").glob("*.pdf"):
        pages = extract_pages_from_filename(pdf_path.name)
        if pages and pages > 0:
            pdfs.append((pdf_path, pages))
    return pdfs

def benchmark_pdf(pdf_path, workers):
    """Benchmark single PDF with given worker count"""
    with tempfile.TemporaryDirectory() as tmpdir:
        cmd = [
            "../out/Release/pdfium_cli",
            "--workers", str(workers),
            "render-pages",
            str(pdf_path),
            tmpdir
        ]

        start = time.time()
        result = subprocess.run(cmd, capture_output=True, timeout=600)
        elapsed = time.time() - start

        if result.returncode != 0:
            return None

        return elapsed

def main():
    print("Finding benchmark PDFs...")
    all_pdfs = find_benchmark_pdfs()
    all_pdfs.sort(key=lambda x: x[1])  # Sort by page count

    print(f"Found {len(all_pdfs)} production PDFs")

    # Stratified sampling: Select PDFs across page ranges
    ranges = [
        ("1-50", 1, 50, 10),
        ("51-100", 51, 100, 10),
        ("101-200", 101, 200, 15),
        ("201-500", 201, 500, 15),
        ("501+", 501, 10000, 10)
    ]

    sample = []
    for range_name, min_p, max_p, target_count in ranges:
        range_pdfs = [p for p in all_pdfs if min_p <= p[1] <= max_p]
        if range_pdfs:
            # Sample evenly distributed
            step = max(1, len(range_pdfs) // target_count)
            sample.extend(range_pdfs[::step][:target_count])
            print(f"  {range_name:10s}: {len(range_pdfs):3d} available, sampling {min(target_count, len(range_pdfs)):2d}")

    print(f"\nBenchmarking {len(sample)} PDFs (K=1 vs K=8)...")
    print(f"{'#':3s} {'PDF':50s} {'Pages':6s} {'K=1 (s)':9s} {'K=8 (s)':9s} {'Speedup':8s}")
    print("-" * 90)

    results = []
    for i, (pdf_path, pages) in enumerate(sample, 1):
        # Benchmark K=1
        time_k1 = benchmark_pdf(pdf_path, 1)
        if time_k1 is None:
            print(f"{i:3d} {pdf_path.name:50s} {pages:6d}p FAIL")
            continue

        # Benchmark K=8
        time_k8 = benchmark_pdf(pdf_path, 8)
        if time_k8 is None:
            print(f"{i:3d} {pdf_path.name:50s} {pages:6d}p FAIL")
            continue

        speedup = time_k1 / time_k8
        print(f"{i:3d} {pdf_path.name:50s} {pages:6d}p {time_k1:9.2f}s {time_k8:9.2f}s {speedup:7.2f}x")

        results.append({
            "pdf": pdf_path.name,
            "pages": pages,
            "time_k1": time_k1,
            "time_k8": time_k8,
            "speedup": speedup
        })

    # Analysis
    print("\n" + "=" * 90)
    print("=== COMBINED PNG+THREADING SPEEDUP ANALYSIS (Production Corpus) ===")
    print("=" * 90)

    if results:
        speedups = [r["speedup"] for r in results]
        print(f"\nOverall (n={len(results)} PDFs):")
        print(f"  Mean:   {statistics.mean(speedups):.2f}x")
        print(f"  Median: {statistics.median(speedups):.2f}x")
        print(f"  Min:    {min(speedups):.2f}x")
        print(f"  Max:    {max(speedups):.2f}x")
        print(f"  StdDev: {statistics.stdev(speedups):.2f}x")

        # By page range
        print("\nBy Page Range:")
        for range_name, min_p, max_p, _ in ranges:
            range_results = [r for r in results if min_p <= r["pages"] <= max_p]
            if range_results:
                range_speedups = [r["speedup"] for r in range_results]
                print(f"  {range_name:10s} ({len(range_results):2d} PDFs): {statistics.mean(range_speedups):5.2f}x mean, {statistics.median(range_speedups):5.2f}x median, [{min(range_speedups):5.2f}x - {max(range_speedups):5.2f}x] range")

    # Save detailed results
    with open("/tmp/real_corpus_results_n264.json", "w") as f:
        json.dump(results, f, indent=2)
    print(f"\nDetailed results saved to /tmp/real_corpus_results_n264.json")

if __name__ == "__main__":
    main()
