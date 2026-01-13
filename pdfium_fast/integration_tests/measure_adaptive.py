#!/usr/bin/env python3
"""
Measure adaptive threading performance (N=307)
Validates that adaptive K-selection improves performance vs fixed K=8 default
"""
import subprocess
import tempfile
import time
from pathlib import Path
import statistics

def benchmark_pdf(pdf_path, explicit_k=None, runs=3):
    """
    Benchmark PDF with optional explicit thread count
    If explicit_k is None, uses adaptive selection (auto-K)
    Returns: (mean_time, stdev, times_list)
    """
    times = []

    for run in range(runs):
        with tempfile.TemporaryDirectory() as tmpdir:
            cmd = [
                "../out/Release/pdfium_cli",
                "render-pages",
                str(pdf_path),
                tmpdir
            ]

            # Add explicit --threads K if specified
            if explicit_k is not None:
                cmd.insert(1, "--threads")
                cmd.insert(2, str(explicit_k))

            start = time.time()
            result = subprocess.run(cmd, capture_output=True, timeout=1200, text=True)
            elapsed = time.time() - start

            if result.returncode != 0:
                print(f"  ERROR: Run {run+1} failed")
                print(f"  stderr: {result.stderr[:200]}")
                return None

            times.append(elapsed)

    mean_time = statistics.mean(times)
    stdev = statistics.stdev(times) if len(times) > 1 else 0
    return (mean_time, stdev, times)

def main():
    print("=" * 80)
    print("N=307: Adaptive Threading Performance Measurement")
    print("=" * 80)
    print()

    # Test PDFs organized by size category
    test_pdfs = [
        # Large (>1000p): auto-K=4 expected (avoid memory bottleneck)
        ("Large (>1000p)", [
            ("1931pages_7ZNNFJGHOEFFP6I4OARCZGH3GPPDNDXC.pdf", 1931, 4),
        ]),
        # Medium (200-1000p): auto-K=8 expected (maximize parallelism)
        ("Medium (200-1000p)", [
            ("0821pages_LUNFJFH4KWZ3ZFNRO43WSMZPLM4OLB7C.pdf", 821, 8),
            ("cc_001_931p.pdf", 931, 8),
            ("cc_002_522p.pdf", 522, 8),
            ("cc_004_291p.pdf", 291, 8),
            ("0201pages_RYDFB4ZZNNBE6LLDSY4CFGWQQ7U3KSAA.pdf", 201, 8),
            ("0255pages_OKRV2XLZ2UGGRCBKHS3I5DA5TNLNVHWC.pdf", 255, 8),
        ]),
        # Small-medium (50-199p): auto-K=4 expected (balanced)
        ("Small-medium (50-199p)", [
            ("0100pages_7FKQLKX273JBHXAAW5XDRT27JGMIZMCI.pdf", 100, 4),
            ("0106pages_IYAFRX3M262EGRMQJHT7MCODUP5ZGYRW.pdf", 106, 4),
            ("cc_007_101p.pdf", 101, 4),
            ("cc_015_101p.pdf", 101, 4),
            ("cc_020_103p.pdf", 103, 4),
        ]),
    ]

    all_results = []

    for category_name, pdfs in test_pdfs:
        print(f"\n{'=' * 80}")
        print(f"Category: {category_name}")
        print('=' * 80)

        for pdf_name, pages, expected_auto_k in pdfs:
            pdf_path = Path(f"pdfs/benchmark/{pdf_name}")

            if not pdf_path.exists():
                print(f"\nERROR: {pdf_name} not found")
                continue

            print(f"\n{pdf_name} ({pages}p)")
            print(f"  Expected auto-K: {expected_auto_k}")
            print(f"  Measuring with 3 runs each...")

            # Measure auto-K (adaptive selection)
            print(f"    Auto-K: ", end="", flush=True)
            auto_result = benchmark_pdf(pdf_path, explicit_k=None, runs=3)
            if auto_result is None:
                print("FAILED")
                continue
            auto_mean, auto_std, auto_times = auto_result
            print(f"{auto_mean:.2f}s ± {auto_std:.2f}s")

            # Measure explicit K=8 (old default)
            print(f"    K=8:    ", end="", flush=True)
            k8_result = benchmark_pdf(pdf_path, explicit_k=8, runs=3)
            if k8_result is None:
                print("FAILED")
                continue
            k8_mean, k8_std, k8_times = k8_result
            print(f"{k8_mean:.2f}s ± {k8_std:.2f}s")

            # Measure explicit K=4 for comparison
            print(f"    K=4:    ", end="", flush=True)
            k4_result = benchmark_pdf(pdf_path, explicit_k=4, runs=3)
            if k4_result is None:
                print("FAILED")
                continue
            k4_mean, k4_std, k4_times = k4_result
            print(f"{k4_mean:.2f}s ± {k4_std:.2f}s")

            # Analysis
            speedup_auto_vs_k8 = k8_mean / auto_mean
            speedup_k4_vs_k8 = k8_mean / k4_mean

            # Check if auto matches expected K
            if abs(auto_mean - k8_mean) < abs(auto_mean - k4_mean):
                detected_auto_k = 8
            else:
                detected_auto_k = 4

            print(f"  Results:")
            print(f"    Auto-K vs K=8: {speedup_auto_vs_k8:.3f}x")
            print(f"    K=4 vs K=8:    {speedup_k4_vs_k8:.3f}x")
            print(f"    Detected auto-K={detected_auto_k} (expected {expected_auto_k})")

            # Regression check
            if category_name == "Large (>1000p)":
                # Large PDFs: auto-K should beat K=8 (fix regression)
                if speedup_auto_vs_k8 > 1.10:
                    status = "✓ PASS - Regression fixed"
                elif speedup_auto_vs_k8 > 0.95:
                    status = "~ MARGINAL - Small improvement"
                else:
                    status = "✗ FAIL - Still regressed"
            else:
                # Other categories: auto-K should match or beat K=8
                if speedup_auto_vs_k8 > 0.95:
                    status = "✓ PASS - Performance maintained"
                else:
                    status = "✗ FAIL - Regression detected"

            print(f"    Status: {status}")

            all_results.append({
                "category": category_name,
                "pdf": pdf_name,
                "pages": pages,
                "expected_auto_k": expected_auto_k,
                "detected_auto_k": detected_auto_k,
                "auto_mean": auto_mean,
                "auto_std": auto_std,
                "k8_mean": k8_mean,
                "k8_std": k8_std,
                "k4_mean": k4_mean,
                "k4_std": k4_std,
                "speedup_auto_vs_k8": speedup_auto_vs_k8,
                "speedup_k4_vs_k8": speedup_k4_vs_k8,
                "status": status,
            })

    # Summary
    print("\n" + "=" * 80)
    print("SUMMARY")
    print("=" * 80)

    # Group by category
    for category_name, _ in test_pdfs:
        cat_results = [r for r in all_results if r["category"] == category_name]
        if not cat_results:
            continue

        print(f"\n{category_name}:")
        speedups = [r["speedup_auto_vs_k8"] for r in cat_results]
        print(f"  Count: {len(cat_results)}")
        print(f"  Auto-K vs K=8 speedup:")
        print(f"    Mean:   {statistics.mean(speedups):.3f}x")
        print(f"    Median: {statistics.median(speedups):.3f}x")
        print(f"    Range:  [{min(speedups):.3f}x - {max(speedups):.3f}x]")

        # Status summary
        pass_count = sum(1 for r in cat_results if "PASS" in r["status"])
        print(f"  Status: {pass_count}/{len(cat_results)} PASS")

    # Overall validation
    print("\n" + "=" * 80)
    print("VALIDATION")
    print("=" * 80)

    total_pass = sum(1 for r in all_results if "PASS" in r["status"])
    total_tests = len(all_results)

    print(f"Overall: {total_pass}/{total_tests} tests passed")

    # Key findings
    large_pdfs = [r for r in all_results if r["category"] == "Large (>1000p)"]
    if large_pdfs:
        large_speedup = large_pdfs[0]["speedup_auto_vs_k8"]
        k4_speedup = large_pdfs[0]["speedup_k4_vs_k8"]
        print(f"\nKey finding (1931p PDF):")
        print(f"  Auto-K vs K=8: {large_speedup:.3f}x")
        print(f"  K=4 vs K=8:    {k4_speedup:.3f}x")
        print(f"  Regression fix: {((large_speedup - 1) * 100):.1f}% improvement")

    if total_pass == total_tests:
        print("\n✓ All tests passed - Adaptive threading validated")
    else:
        print(f"\n✗ {total_tests - total_pass} test(s) failed - Review needed")

    return 0 if total_pass == total_tests else 1

if __name__ == "__main__":
    exit(main())
