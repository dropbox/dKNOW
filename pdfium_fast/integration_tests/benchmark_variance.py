#!/usr/bin/env python3
"""
Variance analysis (Task 4 from MANAGER directive)
Run 10 PDFs × 10 runs each = 100 measurements, calculate 95% CI
"""
import json
import subprocess
import tempfile
import time
from pathlib import Path
import statistics
import scipy.stats as stats

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
    # Select 10 PDFs covering range of sizes
    test_pdfs = [
        ("pdfs/benchmark/0100pages_7FKQLKX273JBHXAAW5XDRT27JGMIZMCI.pdf", 100),
        ("pdfs/benchmark/0109pages_NRCGAGBLTXTXEKXNLQ4L4NFA6ZQPCWE5.pdf", 109),
        ("pdfs/benchmark/0130pages_ZJJJ6P4UAGH7LKLACPT5P437FB5F3MYF.pdf", 130),
        ("pdfs/benchmark/0150pages_44LQBJ56XNS2C6VVWKOP2YUNKCODKTEA.pdf", 150),
        ("pdfs/benchmark/0172pages_DNJIAKOPLDZDQCXT2PDA6V6DGL2MQL2H.pdf", 172),
        ("pdfs/benchmark/0192pages_ULFRJCOWYIDOOTY7KWUPC6NU7UTTCLSO.pdf", 192),
        ("pdfs/benchmark/0201pages_RYDFB4ZZNNBE6LLDSY4CFGWQQ7U3KSAA.pdf", 201),
        ("pdfs/benchmark/0255pages_OKRV2XLZ2UGGRCBKHS3I5DA5TNLNVHWC.pdf", 255),
        ("pdfs/benchmark/0309pages_7LD3RVJDZGTXF53CDLCI67YPWZQ5POOA.pdf", 309),
        ("pdfs/benchmark/0496pages_E3474JUEVRWQ3P2J2I3XBFKMMVZLLKWZ.pdf", 496),
    ]

    print(f"Running variance analysis: 10 PDFs × 10 runs = 100 measurements")
    print(f"This will measure reproducibility of performance (±% variance, 95% CI)")
    print()

    all_results = []

    for pdf_path, pages in test_pdfs:
        print(f"{Path(pdf_path).name} ({pages}p):")

        # Run 10 times with K=8
        times = []
        for run in range(1, 11):
            elapsed = benchmark_pdf(pdf_path, 8)
            if elapsed is None:
                print(f"  Run {run:2d}: FAIL")
                break
            times.append(elapsed)
            print(f"  Run {run:2d}: {elapsed:.3f}s")

        if len(times) == 10:
            mean = statistics.mean(times)
            stdev = statistics.stdev(times)
            variance_pct = (stdev / mean) * 100

            # 95% confidence interval
            confidence = 0.95
            degrees_freedom = len(times) - 1
            t_value = stats.t.ppf((1 + confidence) / 2, degrees_freedom)
            margin_error = t_value * (stdev / (len(times) ** 0.5))
            ci_lower = mean - margin_error
            ci_upper = mean + margin_error

            all_results.append({
                "pdf": Path(pdf_path).name,
                "pages": pages,
                "times": times,
                "mean": mean,
                "stdev": stdev,
                "variance_pct": variance_pct,
                "ci_95_lower": ci_lower,
                "ci_95_upper": ci_upper
            })

            print(f"  Mean: {mean:.3f}s, StdDev: {stdev:.3f}s ({variance_pct:.1f}%), 95% CI: [{ci_lower:.3f}s, {ci_upper:.3f}s]")
        print()

    # Overall analysis
    print("=" * 80)
    print("=== VARIANCE ANALYSIS SUMMARY ===")
    print("=" * 80)

    if all_results:
        all_variances = [r["variance_pct"] for r in all_results]
        print(f"\nOverall Variance (n={len(all_results)} PDFs, 10 runs each):")
        print(f"  Mean variance: {statistics.mean(all_variances):.1f}%")
        print(f"  Median variance: {statistics.median(all_variances):.1f}%")
        print(f"  Min variance: {min(all_variances):.1f}%")
        print(f"  Max variance: {max(all_variances):.1f}%")

        # Check reproducibility requirement
        max_acceptable_variance = 15.0  # ±15% is acceptable
        reproducible = [r for r in all_results if r["variance_pct"] <= max_acceptable_variance]
        print(f"\nReproducibility Assessment:")
        print(f"  {len(reproducible)}/{len(all_results)} PDFs have ≤{max_acceptable_variance}% variance")
        print(f"  Conclusion: {'REPRODUCIBLE' if len(reproducible) == len(all_results) else 'VARIABLE'}")

        # Detail by PDF
        print(f"\n{'PDF':50s} {'Pages':6s} {'Variance':10s} {'95% CI':20s}")
        print("-" * 90)
        for r in all_results:
            ci_range = f"[{r['ci_95_lower']:.2f}s, {r['ci_95_upper']:.2f}s]"
            print(f"{r['pdf']:50s} {r['pages']:6d}p {r['variance_pct']:9.1f}% {ci_range:>20s}")

    # Save detailed results
    with open("/tmp/variance_results_n264.json", "w") as f:
        json.dump(all_results, f, indent=2)
    print(f"\nDetailed results saved to /tmp/variance_results_n264.json")

if __name__ == "__main__":
    main()
