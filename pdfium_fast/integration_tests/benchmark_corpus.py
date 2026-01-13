#!/usr/bin/env python3
"""
Benchmark corpus performance (Task 3 from MANAGER directive)
Measures combined PNG+threading speedup on 50+ PDFs with K=1 and K=8
"""
import json
import glob
import subprocess
import tempfile
import time
from pathlib import Path
import statistics
import random

def find_corpus_pdfs():
    """Find PDFs across all categories and sizes (excluding trivial 1-page edge cases)"""
    all_pdfs = []

    for manifest_path in glob.glob("master_test_suite/expected_outputs/**/manifest.json", recursive=True):
        try:
            with open(manifest_path) as f:
                manifest = json.load(f)
                pages = manifest.get("pdf_pages", 0)
                parts = Path(manifest_path).parts
                category = parts[-3]

                # Include multi-page PDFs OR meaningful single-page PDFs from production categories
                is_production_category = category in ["arxiv", "cc", "edinet", "japanese", "pages", "web", "benchmark"]
                is_multipage = pages >= 2

                if pages > 0 and (is_multipage or is_production_category):
                    pdf_stem = parts[-2]
                    pdf_path = Path(f"pdfs/{category}/{pdf_stem}.pdf")
                    if pdf_path.exists():
                        all_pdfs.append((pdf_path, pages, category))
        except:
            pass

    return all_pdfs

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
        result = subprocess.run(cmd, capture_output=True, timeout=300)
        elapsed = time.time() - start

        if result.returncode != 0:
            return None

        return elapsed

def main():
    print("Finding corpus PDFs...")
    all_pdfs = find_corpus_pdfs()
    print(f"Found {len(all_pdfs)} total PDFs")

    # Stratified sampling: 50 PDFs across categories and sizes
    # Group by category
    by_category = {}
    for pdf_path, pages, category in all_pdfs:
        if category not in by_category:
            by_category[category] = []
        by_category[category].append((pdf_path, pages, category))

    print("\nPDF Distribution by Category:")
    for cat, pdfs in sorted(by_category.items()):
        print(f"  {cat}: {len(pdfs)} PDFs")

    # Sample 50 PDFs: proportional to category size, stratified by page count
    target_count = 50
    sample = []

    for category, pdfs in by_category.items():
        # Number to sample from this category (proportional)
        n = max(1, int(len(pdfs) / len(all_pdfs) * target_count))

        # Sort by pages and take evenly distributed sample
        pdfs.sort(key=lambda x: x[1])
        step = max(1, len(pdfs) // n)
        sample.extend(pdfs[::step][:n])

    # Trim to exactly 50
    sample = sample[:target_count]

    print(f"\nBenchmarking {len(sample)} PDFs (K=1 vs K=8)...")
    print("PDF | Pages | Category | K=1 (s) | K=8 (s) | Speedup")
    print("-" * 70)

    results = []
    for i, (pdf_path, pages, category) in enumerate(sample, 1):
        # Benchmark K=1
        time_k1 = benchmark_pdf(pdf_path, 1)
        if time_k1 is None:
            print(f"{i:2d}. {pdf_path.name:40s} | {pages:5d}p | {category:12s} | FAIL")
            continue

        # Benchmark K=8
        time_k8 = benchmark_pdf(pdf_path, 8)
        if time_k8 is None:
            print(f"{i:2d}. {pdf_path.name:40s} | {pages:5d}p | {category:12s} | FAIL")
            continue

        speedup = time_k1 / time_k8
        print(f"{i:2d}. {pdf_path.name:40s} | {pages:5d}p | {category:12s} | {time_k1:6.2f}s | {time_k8:6.2f}s | {speedup:5.2f}x")

        results.append({
            "pdf": pdf_path.name,
            "pages": pages,
            "category": category,
            "time_k1": time_k1,
            "time_k8": time_k8,
            "speedup": speedup
        })

    # Analysis
    print("\n" + "=" * 70)
    print("=== COMBINED PNG+THREADING SPEEDUP ANALYSIS ===")
    print("=" * 70)

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
        ranges = [
            ("1-10", 1, 10),
            ("11-50", 11, 50),
            ("51-100", 51, 100),
            ("101-200", 101, 200),
            ("201+", 201, 10000)
        ]

        for range_name, min_p, max_p in ranges:
            range_results = [r for r in results if min_p <= r["pages"] <= max_p]
            if range_results:
                range_speedups = [r["speedup"] for r in range_results]
                print(f"  {range_name:10s} ({len(range_results):2d} PDFs): {statistics.mean(range_speedups):.2f}x mean, {statistics.median(range_speedups):.2f}x median")

        # By category
        print("\nBy Category:")
        for category in sorted(set(r["category"] for r in results)):
            cat_results = [r for r in results if r["category"] == category]
            if cat_results:
                cat_speedups = [r["speedup"] for r in cat_results]
                print(f"  {category:12s} ({len(cat_results):2d} PDFs): {statistics.mean(cat_speedups):.2f}x mean")

    # Save detailed results
    with open("/tmp/corpus_results_n264.json", "w") as f:
        json.dump(results, f, indent=2)
    print(f"\nDetailed results saved to /tmp/corpus_results_n264.json")

if __name__ == "__main__":
    main()
