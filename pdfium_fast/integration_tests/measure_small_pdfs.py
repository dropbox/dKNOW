#!/usr/bin/env python3
"""
Benchmark small PDFs to validate Amdahl's Law and optimal thread counts.

Goal: Determine if K=1 is optimal for very small PDFs (<10 pages) due to
process overhead dominating actual rendering time.

Measures:
- 1-page PDFs (n=20): Expect K=1 optimal (overhead >> work)
- 2-9 page PDFs (n=20): Expect K=1 or K=2 optimal
- 10-49 page PDFs (n=4): Transition zone, K=4 may become beneficial
"""

import os
import subprocess
import tempfile
import time
import json
import statistics
from pathlib import Path

# Configuration
BINARY = '../out/Release/pdfium_cli'
RUNS = 3  # Runs per configuration
K_VALUES = [1, 2, 4, 8]  # Thread counts to test

def get_page_count(pdf_path):
    """Get page count by extracting text and counting form feeds."""
    with tempfile.NamedTemporaryFile(mode='w', delete=True) as tmp:
        result = subprocess.run(
            [BINARY, 'extract-text', pdf_path, tmp.name],
            capture_output=True,
            timeout=10
        )
        if result.returncode != 0:
            return None
        with open(tmp.name, 'rb') as f:
            content = f.read()
            return content.count(b'\f') + 1

def find_pdfs_by_size():
    """Find PDFs grouped by size category."""
    pdfs = {'1p': [], '2-9p': [], '10-49p': []}

    for root, dirs, files in os.walk('./pdfs'):
        for f in files:
            if not f.endswith('.pdf'):
                continue
            pdf_path = os.path.join(root, f)
            try:
                pages = get_page_count(pdf_path)
                if pages is None:
                    continue

                if pages == 1:
                    pdfs['1p'].append((f, pdf_path, pages))
                elif 2 <= pages <= 9:
                    pdfs['2-9p'].append((f, pdf_path, pages))
                elif 10 <= pages <= 49:
                    pdfs['10-49p'].append((f, pdf_path, pages))
            except:
                continue

    # Limit to reasonable sample sizes
    pdfs['1p'] = pdfs['1p'][:20]
    pdfs['2-9p'] = pdfs['2-9p'][:20]
    pdfs['10-49p'] = pdfs['10-49p'][:4]

    return pdfs

def benchmark_render(pdf_path, k, runs=3):
    """Benchmark rendering with K threads."""
    times = []

    with tempfile.TemporaryDirectory() as tmpdir:
        for run in range(runs):
            start = time.time()
            result = subprocess.run(
                [BINARY, '--threads', str(k), 'render-pages', pdf_path, tmpdir],
                capture_output=True,
                timeout=30
            )
            elapsed = time.time() - start

            if result.returncode != 0:
                return None

            times.append(elapsed)

    return {
        'mean': statistics.mean(times),
        'stdev': statistics.stdev(times) if len(times) > 1 else 0,
        'min': min(times),
        'max': max(times),
        'runs': times
    }

def main():
    print("Small PDF Benchmark - Amdahl's Law Validation")
    print("=" * 60)
    print()

    # Find PDFs
    print("Finding PDFs by size category...")
    pdfs = find_pdfs_by_size()
    print(f"  1-page PDFs: {len(pdfs['1p'])}")
    print(f"  2-9 page PDFs: {len(pdfs['2-9p'])}")
    print(f"  10-49 page PDFs: {len(pdfs['10-49p'])}")
    print()

    results = {}

    # Benchmark each category
    for category in ['1p', '2-9p', '10-49p']:
        print(f"\n{category} PDFs:")
        print("-" * 60)

        category_results = []

        for pdf_name, pdf_path, pages in pdfs[category]:
            print(f"\n{pdf_name} ({pages}p):")

            pdf_results = {'name': pdf_name, 'pages': pages, 'k_results': {}}

            # Test each K value
            for k in K_VALUES:
                print(f"  K={k}: ", end='', flush=True)
                result = benchmark_render(pdf_path, k, RUNS)

                if result is None:
                    print("FAILED")
                    continue

                pdf_results['k_results'][k] = result
                print(f"{result['mean']:.3f}s ± {result['stdev']:.3f}s")

            # Calculate speedups relative to K=1
            if 1 in pdf_results['k_results']:
                baseline = pdf_results['k_results'][1]['mean']
                for k in K_VALUES:
                    if k in pdf_results['k_results']:
                        speedup = baseline / pdf_results['k_results'][k]['mean']
                        pdf_results['k_results'][k]['speedup'] = speedup

                # Show speedups
                print("  Speedups vs K=1:", end='')
                for k in [2, 4, 8]:
                    if k in pdf_results['k_results']:
                        speedup = pdf_results['k_results'][k].get('speedup', 0)
                        print(f"  K={k}: {speedup:.2f}x", end='')
                print()

            category_results.append(pdf_results)

        results[category] = category_results

    # Summary statistics
    print("\n" + "=" * 60)
    print("SUMMARY")
    print("=" * 60)

    for category in ['1p', '2-9p', '10-49p']:
        if not results[category]:
            continue

        print(f"\n{category} PDFs (n={len(results[category])}):")

        # Aggregate speedups
        for k in [2, 4, 8]:
            speedups = []
            for pdf in results[category]:
                if k in pdf['k_results'] and 'speedup' in pdf['k_results'][k]:
                    speedups.append(pdf['k_results'][k]['speedup'])

            if speedups:
                mean_speedup = statistics.mean(speedups)
                median_speedup = statistics.median(speedups)
                print(f"  K={k}: {mean_speedup:.2f}x mean, {median_speedup:.2f}x median")

    # Save results
    output_file = 'small_pdf_benchmark_results.json'
    with open(output_file, 'w') as f:
        json.dump(results, f, indent=2)
    print(f"\nResults saved to {output_file}")

    # Recommendations
    print("\n" + "=" * 60)
    print("RECOMMENDATIONS")
    print("=" * 60)

    for category in ['1p', '2-9p', '10-49p']:
        if not results[category]:
            continue

        speedups_k2 = []
        speedups_k4 = []
        speedups_k8 = []

        for pdf in results[category]:
            if 2 in pdf['k_results'] and 'speedup' in pdf['k_results'][2]:
                speedups_k2.append(pdf['k_results'][2]['speedup'])
            if 4 in pdf['k_results'] and 'speedup' in pdf['k_results'][4]:
                speedups_k4.append(pdf['k_results'][4]['speedup'])
            if 8 in pdf['k_results'] and 'speedup' in pdf['k_results'][8]:
                speedups_k8.append(pdf['k_results'][8]['speedup'])

        mean_k2 = statistics.mean(speedups_k2) if speedups_k2 else 0
        mean_k4 = statistics.mean(speedups_k4) if speedups_k4 else 0
        mean_k8 = statistics.mean(speedups_k8) if speedups_k8 else 0

        # Determine optimal K
        if mean_k2 < 1.15 and mean_k4 < 1.15 and mean_k8 < 1.15:
            optimal = "K=1 (no speedup from threading)"
        elif mean_k2 >= max(mean_k4, mean_k8):
            optimal = f"K=2 ({mean_k2:.2f}x speedup)"
        elif mean_k4 >= mean_k8:
            optimal = f"K=4 ({mean_k4:.2f}x speedup)"
        else:
            optimal = f"K=8 ({mean_k8:.2f}x speedup)"

        print(f"{category}: {optimal}")

if __name__ == '__main__':
    main()
