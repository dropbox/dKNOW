#!/usr/bin/env python3
"""
Analyze corpus benchmark results for N=226.
Processes telemetry data to calculate performance statistics by category.
"""

import csv
import statistics
from collections import defaultdict
from pathlib import Path

TELEMETRY_FILE = "integration_tests/telemetry/runs.csv"


def analyze_telemetry():
    """Analyze telemetry data from most recent corpus run."""

    # Read telemetry data
    rows = []
    with open(TELEMETRY_FILE, 'r') as f:
        reader = csv.DictReader(f)
        rows = list(reader)

    # Find most recent session
    if not rows:
        print("No telemetry data found")
        return

    latest_session = rows[-1]['session_id']
    print(f"Analyzing session: {latest_session}")
    print()

    # Filter to latest session, image rendering tests only
    session_rows = [
        r for r in rows
        if r['session_id'] == latest_session
        and r['test_id'].endswith('_image')
        and r['result'] == 'passed'
    ]

    if not session_rows:
        print("No image rendering tests found in latest session")
        return

    print(f"Found {len(session_rows)} image rendering tests")
    print()

    # Group by PDF category
    by_category = defaultdict(list)
    for row in session_rows:
        category = row.get('pdf_category', 'unknown')

        # Extract pages per second
        try:
            pps_4w = float(row['4w_pages_per_sec'])
            pps_1w = float(row['1w_pages_per_sec'])
            speedup = float(row['speedup_4w_vs_1w'])
            pages = int(row['pdf_pages'])

            by_category[category].append({
                'pdf_name': row['pdf_name'],
                'pages': pages,
                'pps_1w': pps_1w,
                'pps_4w': pps_4w,
                'speedup': speedup
            })
        except (ValueError, KeyError) as e:
            continue

    # Print statistics by category
    print("=" * 80)
    print("PERFORMANCE BY CATEGORY")
    print("=" * 80)
    print()

    overall_pps_1w = []
    overall_pps_4w = []
    overall_speedup = []

    for category in sorted(by_category.keys()):
        data = by_category[category]

        pps_1w_values = [d['pps_1w'] for d in data]
        pps_4w_values = [d['pps_4w'] for d in data]
        speedup_values = [d['speedup'] for d in data]

        overall_pps_1w.extend(pps_1w_values)
        overall_pps_4w.extend(pps_4w_values)
        overall_speedup.extend(speedup_values)

        print(f"{category.upper()} ({len(data)} PDFs):")
        print(f"  Single-threaded (K=1):")
        print(f"    Mean:   {statistics.mean(pps_1w_values):.1f} pages/s")
        print(f"    Median: {statistics.median(pps_1w_values):.1f} pages/s")
        print(f"    Stddev: {statistics.stdev(pps_1w_values) if len(pps_1w_values) > 1 else 0:.1f} pages/s")
        print(f"    Range:  {min(pps_1w_values):.1f} - {max(pps_1w_values):.1f} pages/s")
        print(f"  Multi-threaded (K=4):")
        print(f"    Mean:   {statistics.mean(pps_4w_values):.1f} pages/s")
        print(f"    Median: {statistics.median(pps_4w_values):.1f} pages/s")
        print(f"    Range:  {min(pps_4w_values):.1f} - {max(pps_4w_values):.1f} pages/s")
        print(f"  Speedup (K=4 vs K=1):")
        print(f"    Mean:   {statistics.mean(speedup_values):.2f}x")
        print(f"    Median: {statistics.median(speedup_values):.2f}x")
        print(f"    Range:  {min(speedup_values):.2f}x - {max(speedup_values):.2f}x")
        print()

    # Overall statistics
    print("=" * 80)
    print("OVERALL CORPUS")
    print("=" * 80)
    print(f"Total PDFs: {len(session_rows)}")
    print(f"Single-threaded (K=1):")
    print(f"  Mean:   {statistics.mean(overall_pps_1w):.1f} pages/s")
    print(f"  Median: {statistics.median(overall_pps_1w):.1f} pages/s")
    print(f"  Stddev: {statistics.stdev(overall_pps_1w):.1f} pages/s")
    print(f"Multi-threaded (K=4):")
    print(f"  Mean:   {statistics.mean(overall_pps_4w):.1f} pages/s")
    print(f"  Median: {statistics.median(overall_pps_4w):.1f} pages/s")
    print(f"Speedup (K=4 vs K=1):")
    print(f"  Mean:   {statistics.mean(overall_speedup):.2f}x")
    print(f"  Median: {statistics.median(overall_speedup):.2f}x")
    print()

    # Identify outliers (fastest and slowest)
    print("=" * 80)
    print("FASTEST 10 PDFs (K=1, pages/sec)")
    print("=" * 80)
    all_pdfs = []
    for category, data in by_category.items():
        for d in data:
            all_pdfs.append((d['pps_1w'], category, d['pdf_name'], d['pages']))

    all_pdfs.sort(reverse=True)
    for i, (pps, cat, name, pages) in enumerate(all_pdfs[:10], 1):
        print(f"{i:2d}. {pps:6.1f} pages/s - {cat}/{name} ({pages} pages)")
    print()

    print("=" * 80)
    print("SLOWEST 10 PDFs (K=1, pages/sec)")
    print("=" * 80)
    for i, (pps, cat, name, pages) in enumerate(all_pdfs[-10:][::-1], 1):
        print(f"{i:2d}. {pps:6.1f} pages/s - {cat}/{name} ({pages} pages)")
    print()


if __name__ == "__main__":
    analyze_telemetry()
