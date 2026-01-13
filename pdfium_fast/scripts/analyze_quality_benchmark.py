#!/usr/bin/env python3
"""
N=230: Analyze anti-aliasing quality benchmark results
Extracts timing data from benchmark output and calculates speedup statistics
"""

import re
import json
import sys
from collections import defaultdict
from statistics import mean, median, stdev

def parse_benchmark_file(filename):
    """Parse benchmark results file and extract timing data"""
    with open(filename, 'r') as f:
        content = f.read()

    # Extract data for each PDF in each mode
    results = defaultdict(lambda: {'balanced': {}, 'fast': {}})

    current_mode = None
    current_pdf = None

    lines = content.split('\n')
    for line in lines:
        # Detect mode
        if 'Phase 1: Balanced Mode' in line:
            current_mode = 'balanced'
        elif 'Phase 2: Fast Mode' in line:
            current_mode = 'fast'

        # Detect PDF
        match = re.match(r'Testing: (.+\.pdf) \(quality=(\w+)\)', line)
        if match:
            current_pdf = match.group(1)
            current_mode = match.group(2)
            results[current_pdf][current_mode] = {'pages': [], 'total_render_ms': 0, 'total_encode_ms': 0, 'total_time_ms': 0}

        # Extract page timing
        if current_pdf and current_mode:
            timing_match = re.search(r'Page \d+ timing: render=([\d.]+)ms \([\d.]+%\), encode=([\d.]+)ms \([\d.]+%\), write=([\d.]+)ms \([\d.]+%\), total=([\d.]+)ms', line)
            if timing_match:
                render_ms = float(timing_match.group(1))
                encode_ms = float(timing_match.group(2))
                write_ms = float(timing_match.group(3))
                total_ms = float(timing_match.group(4))

                results[current_pdf][current_mode]['pages'].append({
                    'render': render_ms,
                    'encode': encode_ms,
                    'write': write_ms,
                    'total': total_ms
                })
                results[current_pdf][current_mode]['total_render_ms'] += render_ms
                results[current_pdf][current_mode]['total_encode_ms'] += encode_ms
                results[current_pdf][current_mode]['total_time_ms'] += total_ms

    return dict(results)

def calculate_statistics(results):
    """Calculate speedup statistics"""
    stats = []

    for pdf_name, modes in results.items():
        if 'balanced' not in modes or 'fast' not in modes:
            continue

        balanced_data = modes['balanced']
        fast_data = modes['fast']

        if not balanced_data.get('pages') or not fast_data.get('pages'):
            continue

        # Per-page render time averages
        balanced_avg_render = balanced_data['total_render_ms'] / len(balanced_data['pages'])
        fast_avg_render = fast_data['total_render_ms'] / len(fast_data['pages'])

        # Total time
        balanced_total = balanced_data['total_time_ms']
        fast_total = fast_data['total_time_ms']

        # Speedup calculations
        render_speedup = balanced_avg_render / fast_avg_render if fast_avg_render > 0 else 0
        total_speedup = balanced_total / fast_total if fast_total > 0 else 0

        # Render percentage (how much rendering contributes to total time)
        balanced_render_pct = (balanced_data['total_render_ms'] / balanced_total * 100) if balanced_total > 0 else 0
        fast_render_pct = (fast_data['total_render_ms'] / fast_total * 100) if fast_total > 0 else 0

        stats.append({
            'pdf': pdf_name,
            'pages': len(balanced_data['pages']),
            'balanced_avg_render_ms': balanced_avg_render,
            'fast_avg_render_ms': fast_avg_render,
            'balanced_total_ms': balanced_total,
            'fast_total_ms': fast_total,
            'render_speedup': render_speedup,
            'total_speedup': total_speedup,
            'balanced_render_pct': balanced_render_pct,
            'fast_render_pct': fast_render_pct
        })

    return stats

def print_report(stats):
    """Print analysis report"""
    print("=" * 120)
    print("Anti-Aliasing Quality Benchmark Analysis - N=230")
    print("=" * 120)
    print()

    # Overall statistics
    render_speedups = [s['render_speedup'] for s in stats]
    total_speedups = [s['total_speedup'] for s in stats]
    balanced_render_pcts = [s['balanced_render_pct'] for s in stats]

    print("OVERALL STATISTICS:")
    print(f"  PDFs analyzed: {len(stats)}")
    print(f"  Total pages: {sum(s['pages'] for s in stats)}")
    print()
    print(f"  Render speedup (fast vs balanced):")
    print(f"    Mean:   {mean(render_speedups):.3f}x")
    print(f"    Median: {median(render_speedups):.3f}x")
    print(f"    Stdev:  {stdev(render_speedups) if len(render_speedups) > 1 else 0:.3f}x")
    print(f"    Range:  {min(render_speedups):.3f}x - {max(render_speedups):.3f}x")
    print()
    print(f"  Total speedup (end-to-end):")
    print(f"    Mean:   {mean(total_speedups):.3f}x")
    print(f"    Median: {median(total_speedups):.3f}x")
    print(f"    Stdev:  {stdev(total_speedups) if len(total_speedups) > 1 else 0:.3f}x")
    print(f"    Range:  {min(total_speedups):.3f}x - {max(total_speedups):.3f}x")
    print()
    print(f"  Rendering % of total time (balanced mode):")
    print(f"    Mean:   {mean(balanced_render_pcts):.1f}%")
    print(f"    Median: {median(balanced_render_pcts):.1f}%")
    print(f"    Range:  {min(balanced_render_pcts):.1f}% - {max(balanced_render_pcts):.1f}%")
    print()
    print("=" * 120)
    print()

    # Per-PDF breakdown
    print("PER-PDF BREAKDOWN:")
    print()
    print(f"{'PDF':<60} {'Pages':>6} {'Render%':>8} {'R-Speed':>8} {'T-Speed':>8}")
    print(f"{'='*60} {'='*6} {'='*8} {'='*8} {'='*8}")

    # Sort by rendering percentage (descending) to show rendering-heavy PDFs first
    stats_sorted = sorted(stats, key=lambda x: x['balanced_render_pct'], reverse=True)

    for s in stats_sorted:
        print(f"{s['pdf']:<60} {s['pages']:>6} {s['balanced_render_pct']:>7.1f}% {s['render_speedup']:>7.3f}x {s['total_speedup']:>7.3f}x")

    print()
    print("=" * 120)
    print()

    # Category analysis
    print("CATEGORY ANALYSIS:")
    print()

    # Categorize by rendering percentage
    high_render = [s for s in stats if s['balanced_render_pct'] >= 80]
    medium_render = [s for s in stats if 50 <= s['balanced_render_pct'] < 80]
    low_render = [s for s in stats if s['balanced_render_pct'] < 50]

    def print_category(name, pdfs):
        if not pdfs:
            print(f"  {name}: No PDFs")
            return

        total_speedups = [p['total_speedup'] for p in pdfs]
        render_speedups = [p['render_speedup'] for p in pdfs]

        print(f"  {name}: {len(pdfs)} PDFs")
        print(f"    Total speedup:  {mean(total_speedups):.3f}x (median: {median(total_speedups):.3f}x, range: {min(total_speedups):.3f}x-{max(total_speedups):.3f}x)")
        print(f"    Render speedup: {mean(render_speedups):.3f}x (median: {median(render_speedups):.3f}x, range: {min(render_speedups):.3f}x-{max(render_speedups):.3f}x)")
        print()

    print_category("Rendering-heavy (≥80% render time)", high_render)
    print_category("Medium rendering (50-80% render time)", medium_render)
    print_category("Rendering-light (<50% render time)", low_render)

    print("=" * 120)
    print()

    # Success criteria check
    print("SUCCESS CRITERIA CHECK:")
    print()
    mean_total_speedup = mean(total_speedups)
    print(f"  ✓ Mean total speedup ≥1.15x (15% improvement)? {mean_total_speedup:.3f}x {'✓ PASS' if mean_total_speedup >= 1.15 else '✗ FAIL'}")

    if high_render:
        high_render_speedup = mean([p['total_speedup'] for p in high_render])
        print(f"  ✓ Speedup ≥1.3x for rendering-heavy PDFs? {high_render_speedup:.3f}x {'✓ PASS' if high_render_speedup >= 1.3 else '✗ FAIL'}")

    print()
    print("=" * 120)

def main():
    if len(sys.argv) < 2:
        print("Usage: analyze_quality_benchmark.py <benchmark_results_file>")
        sys.exit(1)

    filename = sys.argv[1]
    results = parse_benchmark_file(filename)
    stats = calculate_statistics(results)

    # Print report
    print_report(stats)

    # Save JSON results
    json_filename = filename.replace('.txt', '.json')
    with open(json_filename, 'w') as f:
        json.dump(stats, f, indent=2)

    print(f"JSON results saved to: {json_filename}")

if __name__ == '__main__':
    main()
