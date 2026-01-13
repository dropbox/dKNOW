#!/usr/bin/env python3
"""
Profile PNG encoding overhead across diverse PDF corpus.

Runs pdfium_cli on 30+ PDFs and parses timing output to calculate
PNG encoding overhead percentage. Categorizes PDFs by size and content type.

Usage:
    python3 scripts/profile_png_overhead.py
"""

import subprocess
import re
import tempfile
import shutil
from pathlib import Path
import sys

def get_pdf_info(pdf_path):
    """Get basic PDF info (page count)."""
    result = subprocess.run(
        ['out/Release/pdfium_cli', 'extract-text', pdf_path, '/dev/null'],
        capture_output=True,
        text=True
    )
    # Look for "Processing N pages"
    match = re.search(r'Processing (\d+) page', result.stderr)
    if match:
        return int(match.group(1))
    return 0

def profile_pdf(pdf_path, output_dir):
    """
    Run pdfium_cli and extract timing data.
    Returns list of (page, render_ms, encode_ms, write_ms, encode_pct) tuples.
    """
    result = subprocess.run(
        ['out/Release/pdfium_cli', '--threads', '1', 'render-pages', pdf_path, output_dir],
        capture_output=True,
        text=True
    )

    # Parse timing lines: "Page N timing: render=X.XXms (X.X%), encode=X.XXms (X.X%), write=X.XXms (X.X%), total=X.XXms"
    pattern = r'Page (\d+) timing: render=([\d.]+)ms \(([\d.]+)%\), encode=([\d.]+)ms \(([\d.]+)%\), write=([\d.]+)ms \(([\d.]+)%\), total=([\d.]+)ms'

    timing_data = []
    for line in result.stderr.split('\n'):
        match = re.search(pattern, line)
        if match:
            page = int(match.group(1))
            render_ms = float(match.group(2))
            render_pct = float(match.group(3))
            encode_ms = float(match.group(4))
            encode_pct = float(match.group(5))
            write_ms = float(match.group(6))
            write_pct = float(match.group(7))
            total_ms = float(match.group(8))

            timing_data.append((page, render_ms, encode_ms, write_ms, encode_pct))

    return timing_data

def select_diverse_pdfs(pdf_root, count=30):
    """
    Select diverse PDFs from different categories.
    Simple approach: just select first N PDFs we find.
    """
    pdf_paths = list(Path(pdf_root).rglob('*.pdf'))

    # Take a sample
    selected = []
    for pdf_path in pdf_paths[:count]:
        category = pdf_path.parent.name
        # Use placeholder, will determine actual page count during profiling
        selected.append((pdf_path, -1, category))

    return selected

def main():
    print("PNG Encoding Overhead Profiling")
    print("=" * 80)

    # Select diverse PDFs
    print("\nSelecting diverse PDF sample...")
    pdfs = select_diverse_pdfs('integration_tests/pdfs', count=30)
    print(f"Selected {len(pdfs)} PDFs")

    # Create temp directory for output
    with tempfile.TemporaryDirectory() as temp_dir:
        results = []

        for i, (pdf_path, placeholder_page_count, category) in enumerate(pdfs):
            print(f"\n[{i+1}/{len(pdfs)}] Profiling: {pdf_path.name} ({category})")

            # Profile PDF
            timing_data = profile_pdf(str(pdf_path), temp_dir)

            if not timing_data:
                print(f"  Warning: No timing data extracted")
                continue

            # Calculate statistics
            page_count = len(timing_data)
            encode_percentages = [t[4] for t in timing_data]
            avg_encode_pct = sum(encode_percentages) / len(encode_percentages)
            min_encode_pct = min(encode_percentages)
            max_encode_pct = max(encode_percentages)

            print(f"  {page_count} pages, PNG encode overhead: {avg_encode_pct:.1f}% (min={min_encode_pct:.1f}%, max={max_encode_pct:.1f}%)")

            results.append({
                'pdf': pdf_path.name,
                'pages': page_count,
                'category': category,
                'avg_encode_pct': avg_encode_pct,
                'min_encode_pct': min_encode_pct,
                'max_encode_pct': max_encode_pct,
                'page_data': timing_data
            })

            # Clean temp directory
            for f in Path(temp_dir).glob('*'):
                f.unlink()

    # Print summary
    print("\n" + "=" * 80)
    print("SUMMARY")
    print("=" * 80)

    # Overall statistics
    all_encode_pcts = [r['avg_encode_pct'] for r in results]
    overall_avg = sum(all_encode_pcts) / len(all_encode_pcts)
    overall_min = min(all_encode_pcts)
    overall_max = max(all_encode_pcts)

    print(f"\nOverall PNG encoding overhead: {overall_avg:.1f}% ± {max(abs(overall_avg - overall_min), abs(overall_max - overall_avg)):.1f}%")
    print(f"  Range: {overall_min:.1f}% - {overall_max:.1f}%")
    print(f"  Median: {sorted(all_encode_pcts)[len(all_encode_pcts)//2]:.1f}%")

    # By page count
    small_results = [r for r in results if r['pages'] <= 5]
    medium_results = [r for r in results if 6 <= r['pages'] <= 50]
    large_results = [r for r in results if r['pages'] > 50]

    print(f"\nBy PDF size:")
    if small_results:
        avg = sum(r['avg_encode_pct'] for r in small_results) / len(small_results)
        print(f"  Small (1-5 pages): {avg:.1f}% ({len(small_results)} PDFs)")
    if medium_results:
        avg = sum(r['avg_encode_pct'] for r in medium_results) / len(medium_results)
        print(f"  Medium (6-50 pages): {avg:.1f}% ({len(medium_results)} PDFs)")
    if large_results:
        avg = sum(r['avg_encode_pct'] for r in large_results) / len(large_results)
        print(f"  Large (51+ pages): {avg:.1f}% ({len(large_results)} PDFs)")

    # Decision criteria
    print(f"\n" + "=" * 80)
    print("DECISION")
    print("=" * 80)

    if overall_avg >= 50:
        print(f"✅ PNG encoding is {overall_avg:.1f}% bottleneck (≥50% threshold)")
        print("   Recommendation: Implement Z_NO_COMPRESSION (N=246)")
        print(f"   Expected gain: {100 / (100 - overall_avg):.2f}x speedup if eliminated")
    elif overall_avg >= 30:
        print(f"⚠️  PNG encoding is {overall_avg:.1f}% bottleneck (30-50% range)")
        print("   Recommendation: Consider optimization, measure trade-offs")
        print(f"   Expected gain: {100 / (100 - overall_avg):.2f}x speedup if eliminated")
    else:
        print(f"❌ PNG encoding is only {overall_avg:.1f}% bottleneck (<30% threshold)")
        print("   Recommendation: REJECT 74% claim, find real bottleneck with profiling")
        print("   Focus optimization elsewhere")

    # Detailed results table
    print(f"\n" + "=" * 80)
    print("DETAILED RESULTS")
    print("=" * 80)
    print(f"\n{'PDF':<40} {'Pages':>6} {'Category':<15} {'Encode %':>10}")
    print("-" * 80)
    for r in sorted(results, key=lambda x: x['avg_encode_pct'], reverse=True):
        print(f"{r['pdf']:<40} {r['pages']:>6} {r['category']:<15} {r['avg_encode_pct']:>9.1f}%")

    return 0

if __name__ == '__main__':
    sys.exit(main())
