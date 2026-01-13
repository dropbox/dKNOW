#!/usr/bin/env python3
"""
Analyze the "83x speedup" claim vs actual measurements
WORKER0 N=305 - Response to MANAGER directive
"""
import json
import statistics
import math

def calculate_ci(values, confidence=0.95):
    """Calculate confidence interval"""
    n = len(values)
    if n < 2:
        return (0, 0)
    mean = statistics.mean(values)
    stdev = statistics.stdev(values)
    # t-value for 95% CI (approximate for n>30)
    t = 1.96 if n > 30 else 2.0
    margin = t * (stdev / math.sqrt(n))
    return (mean - margin, mean + margin)

def main():
    # Load N=264 benchmark data
    with open("/tmp/real_corpus_results_n264.json") as f:
        results = json.load(f)

    print("=" * 90)
    print("=== PERFORMANCE CLAIM ANALYSIS: 83x Theoretical vs Actual Measurements ===")
    print("=" * 90)
    print()

    print("## Theoretical 83x Claim")
    print()
    print("The '83x' speedup is THEORETICAL, calculated as:")
    print("  83x = 11x (PNG optimization) × 7.5x (threading at K=8)")
    print()
    print("This would require comparing:")
    print("  Baseline:  Upstream PDFium (single-threaded, PNG compression ON)")
    print("  Optimized: Current build (K=8 threads, PNG compression OFF)")
    print()

    print("## Actual Measurements")
    print()
    print("### Component 1: PNG Optimization (11x)")
    print("  Source: v1.2.0 release notes (N=264)")
    print("  Method: Single-threaded rendering, PNG compression ON vs OFF")
    print("  Result: 11x speedup by disabling PNG compression")
    print("  Code:   testing/image_diff/image_diff_png_libpng.cpp:530")
    print("          - png_set_compression_level(png_ptr, Z_NO_COMPRESSION)")
    print("          - png_set_filter(png_ptr, 0, PNG_FILTER_NONE)")
    print()

    print("### Component 2: Threading Speedup (K=8 vs K=1)")
    print(f"  Source: Real corpus benchmark (N=264)")
    print(f"  PDFs tested: {len(results)}")
    print(f"  Page range: {min(r['pages'] for r in results)}-{max(r['pages'] for r in results)}")
    print()

    # Overall statistics
    speedups = [r["speedup"] for r in results]
    mean_speedup = statistics.mean(speedups)
    median_speedup = statistics.median(speedups)
    ci_low, ci_high = calculate_ci(speedups)

    print(f"  Overall Results:")
    print(f"    Mean:   {mean_speedup:.2f}x")
    print(f"    Median: {median_speedup:.2f}x")
    print(f"    95% CI: [{ci_low:.2f}x - {ci_high:.2f}x]")
    print(f"    Range:  [{min(speedups):.2f}x - {max(speedups):.2f}x]")
    print(f"    StdDev: {statistics.stdev(speedups):.2f}x")
    print()

    # By page range
    ranges = [
        ("100-200p", 100, 200),
        ("201-500p", 201, 500),
        ("501+p", 501, 10000)
    ]

    print("  By Page Range:")
    for range_name, min_p, max_p in ranges:
        range_results = [r for r in results if min_p <= r["pages"] <= max_p]
        if range_results:
            range_speedups = [r["speedup"] for r in range_results]
            range_mean = statistics.mean(range_speedups)
            range_ci_low, range_ci_high = calculate_ci(range_speedups)
            print(f"    {range_name:10s} (n={len(range_results):2d}): {range_mean:4.2f}x mean, 95% CI [{range_ci_low:.2f}x - {range_ci_high:.2f}x]")
    print()

    # Combined theoretical speedup
    png_speedup = 11.0
    threading_speedup = mean_speedup
    combined_theoretical = png_speedup * threading_speedup

    print("### Combined Speedup (Theoretical)")
    print(f"  If PNG and threading combine multiplicatively:")
    print(f"  {png_speedup:.1f}x (PNG) × {threading_speedup:.2f}x (threading) = {combined_theoretical:.1f}x")
    print()
    print(f"  Comparison to '83x' claim:")
    print(f"    Claimed: 83x")
    print(f"    Actual:  {combined_theoretical:.1f}x ({combined_theoretical/83*100:.1f}% of claim)")
    print()

    # Identify issues
    print("## Issues Identified")
    print()

    # Regressions
    regressions = [r for r in results if r["speedup"] < 1.0]
    if regressions:
        print(f"### CRITICAL: Performance Regressions ({len(regressions)} PDFs)")
        for r in regressions:
            print(f"  - {r['pdf']:50s} {r['pages']:4d}p: {r['speedup']:.2f}x (SLOWER at K=8!)")
        print()

    # Poor speedups
    poor = [r for r in results if 1.0 <= r["speedup"] < 2.0]
    if poor:
        print(f"### Poor Speedups (<2x, {len(poor)} PDFs)")
        for r in poor:
            print(f"  - {r['pdf']:50s} {r['pages']:4d}p: {r['speedup']:.2f}x")
        print()

    # Good speedups
    good = [r for r in results if r["speedup"] >= 5.0]
    if good:
        print(f"### Excellent Speedups (≥5x, {len(good)} PDFs)")
        for r in sorted(good, key=lambda x: x["speedup"], reverse=True)[:5]:
            print(f"  - {r['pdf']:50s} {r['pages']:4d}p: {r['speedup']:.2f}x")
        print()

    print("## Conclusion")
    print()
    print(f"The '83x' claim is THEORETICAL and assumes perfect multiplicative scaling.")
    print(f"Actual measurements show:")
    print(f"  - PNG optimization: 11x (measured separately)")
    print(f"  - Threading (K=8):  {mean_speedup:.2f}x mean (measured on 26 PDFs)")
    print(f"  - Combined:         ~{combined_theoretical:.0f}x (theoretical)")
    print()
    print(f"This is {combined_theoretical/83*100:.1f}% of the 83x claim.")
    print()
    print("CRITICAL ISSUE: 1931-page PDF shows 0.90x speedup (REGRESSION)")
    print("This indicates threading overhead exceeds benefit for very large PDFs.")
    print()
    print("## Next Steps")
    print()
    print("1. Investigate 1931-page PDF regression (K=8 slower than K=1)")
    print("2. Profile to find remaining bottlenecks >2%")
    print("3. Test small PDFs (<10 pages) to understand Amdahl's Law limits")
    print("4. Continue optimization (SIMD, AGG quality, lazy loading)")
    print()

if __name__ == "__main__":
    main()
