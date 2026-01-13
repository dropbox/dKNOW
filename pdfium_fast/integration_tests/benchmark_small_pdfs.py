#!/usr/bin/env python3
"""
Benchmark small PDF performance (Task 2 from MANAGER directive)
Tests small PDFs (<50 pages) with K=1,2,4,8 to validate Amdahl's Law
"""
import json
import glob
import subprocess
import tempfile
import time
from pathlib import Path
import statistics

def find_small_pdfs():
    """Find PDFs by size ranges"""
    pdfs_by_size = {"1-5": [], "6-15": [], "16-30": [], "31-49": []}

    for manifest_path in glob.glob("master_test_suite/expected_outputs/**/manifest.json", recursive=True):
        try:
            with open(manifest_path) as f:
                manifest = json.load(f)
                pages = manifest.get("pdf_pages", 0)
                if pages > 0 and pages < 50:
                    parts = Path(manifest_path).parts
                    category = parts[-3]
                    pdf_stem = parts[-2]
                    pdf_path = Path(f"pdfs/{category}/{pdf_stem}.pdf")
                    if pdf_path.exists():
                        if 1 <= pages <= 5:
                            pdfs_by_size["1-5"].append((pdf_path, pages))
                        elif 6 <= pages <= 15:
                            pdfs_by_size["6-15"].append((pdf_path, pages))
                        elif 16 <= pages <= 30:
                            pdfs_by_size["16-30"].append((pdf_path, pages))
                        elif 31 <= pages <= 49:
                            pdfs_by_size["31-49"].append((pdf_path, pages))
        except:
            pass

    return pdfs_by_size

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
        result = subprocess.run(cmd, capture_output=True, timeout=60)
        elapsed = time.time() - start

        if result.returncode != 0:
            return None

        return elapsed

def main():
    print("Finding small PDFs...")
    pdfs_by_size = find_small_pdfs()

    print("\nPDF Distribution:")
    for size_range, pdfs in pdfs_by_size.items():
        print(f"  {size_range} pages: {len(pdfs)} PDFs")

    # Select sample: 5 PDFs from each range (20 total)
    sample = []
    for size_range, pdfs in pdfs_by_size.items():
        pdfs.sort(key=lambda x: x[1])  # Sort by page count
        # Take evenly distributed sample
        step = max(1, len(pdfs) // 5)
        sample.extend(pdfs[::step][:5])

    print(f"\nBenchmarking {len(sample)} PDFs with K=1,2,4,8...")

    results = []
    for pdf_path, pages in sample:
        print(f"\n{pdf_path.name} ({pages}p):", end=" ")

        times = {}
        for k in [1, 2, 4, 8]:
            elapsed = benchmark_pdf(pdf_path, k)
            if elapsed is None:
                print(f"K={k}:FAIL", end=" ")
                break
            times[k] = elapsed
            speedup = times[1] / elapsed if k > 1 else 1.0
            print(f"K={k}:{speedup:.2f}x", end=" ")

        if len(times) == 4:
            results.append({
                "pdf": pdf_path.name,
                "pages": pages,
                "times": times,
                "speedup_4w": times[1] / times[4],
                "speedup_8w": times[1] / times[8]
            })

    # Analysis
    print("\n\n=== Results by Page Range ===")
    for size_range in ["1-5", "6-15", "16-30", "31-49"]:
        range_results = [r for r in results if (
            (size_range == "1-5" and 1 <= r["pages"] <= 5) or
            (size_range == "6-15" and 6 <= r["pages"] <= 15) or
            (size_range == "16-30" and 16 <= r["pages"] <= 30) or
            (size_range == "31-49" and 31 <= r["pages"] <= 49)
        )]

        if range_results:
            speedup_4w = [r["speedup_4w"] for r in range_results]
            speedup_8w = [r["speedup_8w"] for r in range_results]
            print(f"\n{size_range} pages ({len(range_results)} PDFs):")
            print(f"  K=4: {statistics.mean(speedup_4w):.2f}x mean, {statistics.median(speedup_4w):.2f}x median, [{min(speedup_4w):.2f}x - {max(speedup_4w):.2f}x] range")
            print(f"  K=8: {statistics.mean(speedup_8w):.2f}x mean, {statistics.median(speedup_8w):.2f}x median, [{min(speedup_8w):.2f}x - {max(speedup_8w):.2f}x] range")

    # Overall summary
    if results:
        overall_4w = statistics.mean([r["speedup_4w"] for r in results])
        overall_8w = statistics.mean([r["speedup_8w"] for r in results])
        print(f"\n=== Overall Small PDFs (<50 pages) ===")
        print(f"  K=4: {overall_4w:.2f}x mean speedup")
        print(f"  K=8: {overall_8w:.2f}x mean speedup")
        print(f"  Sample size: {len(results)} PDFs")

    # Save detailed results
    with open("/tmp/small_pdf_results_n264.json", "w") as f:
        json.dump(results, f, indent=2)
    print(f"\nDetailed results saved to /tmp/small_pdf_results_n264.json")

if __name__ == "__main__":
    main()
