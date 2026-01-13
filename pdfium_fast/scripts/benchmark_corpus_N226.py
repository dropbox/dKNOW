#!/usr/bin/env python3
"""
Corpus-wide benchmark for PNG optimization (N=226)
Measures rendering performance across all 462 PDFs in the test corpus.
"""

import os
import subprocess
import time
import json
import statistics
from pathlib import Path
from collections import defaultdict

# Configuration
PDFIUM_CLI = "./out/Release/pdfium_cli"
PDF_DIR = "./integration_tests/pdfs"
OUTPUT_DIR = "/tmp/benchmark_n226_output"
RESULTS_FILE = "reports/v1.2/corpus_benchmark_N226.json"

# Categories from the corpus
CATEGORIES = ["arxiv", "web", "edinet", "cc", "synthetic", "benchmark"]


def get_pdf_info(pdf_path):
    """Get page count for a PDF."""
    try:
        result = subprocess.run(
            [PDFIUM_CLI, "extract-text", pdf_path, "/dev/null"],
            capture_output=True,
            text=True,
            timeout=10
        )
        # Parse stderr for page count (from CLI output)
        for line in result.stderr.split('\n'):
            if 'pages' in line.lower():
                # Try to extract page count
                pass

        # Fallback: count actual output
        return {"path": pdf_path, "category": Path(pdf_path).parent.name}
    except Exception as e:
        return None


def benchmark_pdf(pdf_path, threads=1):
    """Benchmark rendering a single PDF."""
    os.makedirs(OUTPUT_DIR, exist_ok=True)

    start_time = time.time()
    try:
        result = subprocess.run(
            [PDFIUM_CLI, "--threads", str(threads), "render-pages", pdf_path, OUTPUT_DIR],
            capture_output=True,
            text=True,
            timeout=300  # 5 minute timeout per PDF
        )
        end_time = time.time()

        if result.returncode != 0:
            return None

        duration = end_time - start_time

        # Count generated files
        output_files = list(Path(OUTPUT_DIR).glob("page_*.png"))
        page_count = len(output_files)

        # Measure file sizes
        total_size = sum(f.stat().st_size for f in output_files)
        avg_size = total_size / page_count if page_count > 0 else 0

        # Clean up output files
        for f in output_files:
            f.unlink()

        return {
            "duration": duration,
            "page_count": page_count,
            "pages_per_sec": page_count / duration if duration > 0 else 0,
            "total_size_mb": total_size / (1024 * 1024),
            "avg_size_kb": avg_size / 1024,
            "success": True
        }
    except subprocess.TimeoutExpired:
        return {"success": False, "error": "timeout"}
    except Exception as e:
        return {"success": False, "error": str(e)}


def main():
    print("Starting corpus-wide benchmark for N=226...")
    print(f"PDFium CLI: {PDFIUM_CLI}")
    print(f"PDF Directory: {PDF_DIR}")
    print()

    # Find all PDFs
    pdf_files = sorted(Path(PDF_DIR).rglob("*.pdf"))
    print(f"Found {len(pdf_files)} PDFs")
    print()

    # Benchmark all PDFs
    results = {}
    category_results = defaultdict(list)

    for i, pdf_path in enumerate(pdf_files, 1):
        pdf_name = pdf_path.stem
        category = pdf_path.parent.name

        print(f"[{i}/{len(pdf_files)}] {category}/{pdf_name}...", end=" ", flush=True)

        result = benchmark_pdf(str(pdf_path), threads=1)

        if result and result.get("success"):
            print(f"{result['duration']:.2f}s ({result['pages_per_sec']:.1f} pages/s)")
            results[str(pdf_path)] = result
            category_results[category].append(result)
        else:
            error = result.get("error", "unknown") if result else "failed"
            print(f"FAILED ({error})")

    print()
    print("=" * 80)
    print("SUMMARY BY CATEGORY")
    print("=" * 80)
    print()

    # Calculate statistics by category
    summary = {}
    for category in sorted(category_results.keys()):
        cat_data = category_results[category]
        if not cat_data:
            continue

        pps_values = [r['pages_per_sec'] for r in cat_data]
        size_values = [r['avg_size_kb'] for r in cat_data]

        summary[category] = {
            "count": len(cat_data),
            "pages_per_sec": {
                "mean": statistics.mean(pps_values),
                "median": statistics.median(pps_values),
                "stddev": statistics.stdev(pps_values) if len(pps_values) > 1 else 0,
                "min": min(pps_values),
                "max": max(pps_values)
            },
            "avg_size_kb": {
                "mean": statistics.mean(size_values),
                "median": statistics.median(size_values),
                "stddev": statistics.stdev(size_values) if len(size_values) > 1 else 0
            }
        }

        print(f"{category.upper()}:")
        print(f"  PDFs: {len(cat_data)}")
        print(f"  Speed: {summary[category]['pages_per_sec']['mean']:.1f} ± {summary[category]['pages_per_sec']['stddev']:.1f} pages/s")
        print(f"  Range: {summary[category]['pages_per_sec']['min']:.1f} - {summary[category]['pages_per_sec']['max']:.1f} pages/s")
        print(f"  File size: {summary[category]['avg_size_kb']['mean']:.0f} ± {summary[category]['avg_size_kb']['stddev']:.0f} KB/page")
        print()

    # Overall statistics
    all_pps = [r['pages_per_sec'] for r in results.values() if r.get('success')]
    all_sizes = [r['avg_size_kb'] for r in results.values() if r.get('success')]

    print("=" * 80)
    print("OVERALL CORPUS")
    print("=" * 80)
    print(f"Total PDFs: {len(results)}")
    print(f"Mean speed: {statistics.mean(all_pps):.1f} pages/s")
    print(f"Median speed: {statistics.median(all_pps):.1f} pages/s")
    print(f"Stddev: {statistics.stdev(all_pps):.1f} pages/s")
    print(f"Range: {min(all_pps):.1f} - {max(all_pps):.1f} pages/s")
    print(f"Mean file size: {statistics.mean(all_sizes):.0f} KB/page")
    print()

    # Save detailed results
    output_data = {
        "timestamp": time.strftime("%Y-%m-%d %H:%M:%S"),
        "pdfium_cli": PDFIUM_CLI,
        "pdf_count": len(results),
        "summary": summary,
        "overall": {
            "pages_per_sec": {
                "mean": statistics.mean(all_pps),
                "median": statistics.median(all_pps),
                "stddev": statistics.stdev(all_pps),
                "min": min(all_pps),
                "max": max(all_pps)
            },
            "avg_size_kb": {
                "mean": statistics.mean(all_sizes),
                "median": statistics.median(all_sizes),
                "stddev": statistics.stdev(all_sizes)
            }
        },
        "detailed_results": results
    }

    os.makedirs(os.path.dirname(RESULTS_FILE), exist_ok=True)
    with open(RESULTS_FILE, 'w') as f:
        json.dump(output_data, f, indent=2)

    print(f"Detailed results saved to: {RESULTS_FILE}")


if __name__ == "__main__":
    main()
