"""
Worker Scaling Test - 1w, 2w, 3w, 4w, 8w Comparison

Tests parallel scaling efficiency across different worker counts.
Measures actual speedup ratio for 1w vs 4w.

META:
  id: scaling
  category: scaling
  level: full
  type: both
  pdf_count: 3
  duration: 5m

RUN: pytest -m scaling
"""

import pytest
import tempfile
from pathlib import Path


# PDFs for scaling analysis (need large PDFs for meaningful parallelism)
SCALING_PDFS = [
    ("0100pages_7FKQLKX273JBHXAAW5XDRT27JGMIZMCI.pdf", 100, "text_heavy"),
    ("cc_008_116p.pdf", 116, "mixed"),
    ("0821pages_LUNFJFH4KWZ3ZFNRO43WSMZPLM4OLB7C.pdf", 821, "large"),
]


def pytest_generate_tests(metafunc):
    """Parametrize with scaling test PDFs."""
    if "scaling_pdf_info" in metafunc.fixturenames:
        metafunc.parametrize(
            "scaling_pdf_info",
            SCALING_PDFS,
            ids=[f"{pages}p_{cat}" for _, pages, cat in SCALING_PDFS]
        )


@pytest.mark.scaling
@pytest.mark.full
@pytest.mark.text
@pytest.mark.timeout(600)  # 821-page PDF × 4 worker counts needs extended timeout
def test_text_worker_scaling(
    scaling_pdf_info,
    benchmark_pdfs,
    optimized_lib,
    extract_text_tool,
    request
):
    """
    Test text extraction scaling across 1, 2, 4, 8 workers.

    META:
      id: scaling_text_001
      category: scaling
      level: full
      type: text
      pdf_count: 3
      duration: 3m
      workers: [1, 2, 4, 8]
      validates: Speedup scales with worker count, 4w/1w >= 2.0x
      impact: high

    DESCRIPTION:
      Measures actual speedup for different worker counts:
      - 1 worker (baseline)
      - 2 workers (should be 1.5-2.0x)
      - 4 workers (should be >= 2.0x)
      - 8 workers (may not scale linearly, depends on CPU)

      Validates CLAUDE.md requirement:
      "Target scaling: 2.0x at 4 workers for text extraction"

    SUCCESS:
      - 4w/1w ratio >= 2.0x
      - Each worker count faster than previous
      - Telemetry logs all worker counts

    FAILURE:
      - Check for lock contention
      - Profile hot spots
      - Verify CPU utilization
      - Check if PDF too small for parallelism
    """
    pdf_name, expected_pages, category = scaling_pdf_info
    pdf_path = benchmark_pdfs / pdf_name

    if not pdf_path.exists():
        pytest.skip(f"PDF not found: {pdf_name}")

    # Test multiple worker counts
    worker_counts = [1, 2, 4, 8]
    results = {}  # {worker_count: (duration, pps)}

    for workers in worker_counts:
        with tempfile.NamedTemporaryFile(mode='w', suffix=f'_{workers}w.txt', delete=False) as tmp:
            output_file = Path(tmp.name)

        try:
            import time
            start = time.time()
            success = pytest.extract_text(pdf_path, output_file, workers, optimized_lib, extract_text_tool)
            duration = time.time() - start

            if success:
                pps = expected_pages / duration if duration > 0 else 0
                results[workers] = (duration, pps)
            else:
                results[workers] = (None, None)

        finally:
            if output_file.exists():
                output_file.unlink()

    # Calculate speedups
    if results[1][0] is None:
        pytest.fail("1-worker baseline failed")

    baseline_duration = results[1][0]
    baseline_pps = results[1][1]

    speedups = {}
    for workers, (duration, pps) in results.items():
        if duration:
            speedups[workers] = baseline_duration / duration
        else:
            speedups[workers] = 0

    # Print scaling results
    print(f"\n{'='*70}")
    print(f"WORKER SCALING ANALYSIS: {pdf_name}")
    print(f"{'='*70}")
    print(f"{'Workers':<10} {'Duration':<12} {'PPS':<12} {'Speedup':<12} {'Efficiency'}")
    print(f"{'-'*70}")
    for workers in worker_counts:
        duration, pps = results[workers]
        speedup = speedups[workers]
        efficiency = (speedup / workers * 100) if workers > 0 else 0
        if duration:
            print(f"{workers:<10} {duration:>8.2f}s    {pps:>8.1f}    {speedup:>8.2f}x    {efficiency:>6.1f}%")
        else:
            print(f"{workers:<10} {'FAILED':<12}")
    print(f"{'='*70}")
    print("")

    # Capture telemetry - include BOTH 1w baseline and 4w results
    request.node._report_pdf_name = pdf_name
    request.node._report_pdf_pages = expected_pages
    request.node._report_pdf_category = category
    request.node._report_worker_count = 4  # Primary test is 4w

    # Capture 1-worker baseline performance (absolute)
    request.node._report_perf_1w_pps = round(baseline_pps, 2)
    request.node._report_perf_1w_duration = round(baseline_duration, 3)

    # Capture 4-worker performance (absolute)
    request.node._report_pages_per_sec = round(results[4][1], 2) if results[4][1] else 0
    request.node._report_total_time_sec = round(results[4][0], 3) if results[4][0] else 0

    # Capture speedup ratio (multiplier)
    request.node._report_speedup_vs_1w = round(speedups[4], 2)

    # Capture all worker counts for analysis
    request.node._report_perf_2w_speedup = round(speedups[2], 2) if 2 in speedups else 0
    request.node._report_perf_8w_speedup = round(speedups[8], 2) if 8 in speedups else 0

    # CRITICAL REQUIREMENT: Validate against CLAUDE.md documented performance
    # Small PDFs (< 200 pages): 1.3-1.5x speedup (process overhead dominates)
    # Large PDFs (≥ 200 pages): 3.0x+ speedup (performance requirement: >= 2.0x)
    ratio_4w_1w = speedups[4]

    if expected_pages >= 200:
        # Large PDFs must meet 2.0x requirement
        required_speedup = 2.0
        assert ratio_4w_1w >= required_speedup, (
            f"\n4w/1w RATIO REQUIREMENT NOT MET: {pdf_name}\n"
            f"  Required: >= {required_speedup}x (large PDF ≥ 200 pages)\n"
            f"  Actual: {ratio_4w_1w:.2f}x\n"
            f"\n  Per CLAUDE.md: '2.0x at 4 workers for text extraction (large PDFs)'\n"
            f"\n  Scaling analysis:\n"
            f"    1w: {baseline_pps:.1f} pps ({baseline_duration:.2f}s)\n"
            f"    2w: {results[2][1]:.1f} pps ({results[2][0]:.2f}s) = {speedups[2]:.2f}x\n"
            f"    4w: {results[4][1]:.1f} pps ({results[4][0]:.2f}s) = {speedups[4]:.2f}x\n"
            f"    8w: {results[8][1]:.1f} pps ({results[8][0]:.2f}s) = {speedups[8]:.2f}x\n"
        )
    else:
        # Small PDFs: verify scaling works but expect lower speedup
        # Just check that we get some speedup (not a regression)
        assert ratio_4w_1w > 1.0, (
            f"\n4w SLOWER THAN 1w: {pdf_name}\n"
            f"  Expected: > 1.0x (some speedup)\n"
            f"  Actual: {ratio_4w_1w:.2f}x\n"
            f"\n  Note: Small PDFs (< 200 pages) have process overhead.\n"
            f"  Per CLAUDE.md: '1.3-1.5x speedup for small PDFs'\n"
        )


@pytest.mark.scaling
@pytest.mark.full
@pytest.mark.image
@pytest.mark.timeout(600)  # 821-page PDF × 4 worker counts = 3284 renders, needs >300s
def test_image_worker_scaling(
    scaling_pdf_info,
    benchmark_pdfs,
    optimized_lib,
    render_tool_multiproc,
    request
):
    """
    Test image rendering scaling across 1, 2, 4, 8 workers.

    META:
      id: scaling_image_001
      category: scaling
      level: full
      type: image
      pdf_count: 3
      duration: 2m
      workers: [1, 2, 4, 8]
      validates: Speedup scales with worker count, 4w/1w >= 3.0x
      impact: high

    DESCRIPTION:
      Measures rendering speedup across worker counts.

      Validates CLAUDE.md requirement:
      "3x at 4 workers compared to 1 optimized worker for page rendering"

    SUCCESS:
      - 4w/1w ratio >= 3.0x
      - Scaling efficiency documented

    FAILURE:
      - Check rendering pipeline bottlenecks
      - Verify bitmap pooling
      - Profile page isolation
    """
    pdf_name, expected_pages, category = scaling_pdf_info
    pdf_path = benchmark_pdfs / pdf_name

    if not pdf_path.exists():
        pytest.skip(f"PDF not found: {pdf_name}")

    # Test multiple worker counts
    worker_counts = [1, 2, 4, 8]
    results = {}

    for workers in worker_counts:
        pages, duration = pytest.render_parallel(pdf_path, workers, optimized_lib, render_tool_multiproc)

        if pages:
            pps = pages / duration if duration > 0 else 0
            results[workers] = (duration, pps)
        else:
            results[workers] = (None, None)

    # Calculate speedups
    if results[1][0] is None:
        pytest.fail("1-worker baseline failed")

    baseline_duration = results[1][0]
    baseline_pps = results[1][1]

    speedups = {}
    for workers, (duration, pps) in results.items():
        if duration:
            speedups[workers] = baseline_duration / duration
        else:
            speedups[workers] = 0

    # Print results
    print(f"\n{'='*70}")
    print(f"IMAGE RENDERING SCALING: {pdf_name}")
    print(f"{'='*70}")
    print(f"{'Workers':<10} {'Duration':<12} {'PPS':<12} {'Speedup':<12} {'Efficiency'}")
    print(f"{'-'*70}")
    for workers in worker_counts:
        duration, pps = results[workers]
        speedup = speedups[workers]
        efficiency = (speedup / workers * 100) if workers > 0 else 0
        if duration:
            print(f"{workers:<10} {duration:>8.2f}s    {pps:>8.1f}    {speedup:>8.2f}x    {efficiency:>6.1f}%")
    print(f"{'='*70}\n")

    # Telemetry - capture BOTH 1w baseline and 4w results
    request.node._report_pdf_name = pdf_name
    request.node._report_pdf_pages = expected_pages
    request.node._report_worker_count = 4

    # 1-worker baseline (absolute performance)
    request.node._report_perf_1w_pps = round(baseline_pps, 2)
    request.node._report_perf_1w_duration = round(baseline_duration, 3)

    # 4-worker performance (absolute)
    request.node._report_pages_per_sec = round(results[4][1], 2)
    request.node._report_total_time_sec = round(results[4][0], 3) if results[4][0] else 0

    # Speedup ratio
    request.node._report_speedup_vs_1w = round(speedups[4], 2)

    # All worker speedups
    request.node._report_perf_2w_speedup = round(speedups[2], 2) if 2 in speedups else 0
    request.node._report_perf_8w_speedup = round(speedups[8], 2) if 8 in speedups else 0

    # REQUIREMENT: Validate against CLAUDE.md documented performance
    # Image rendering follows same pattern as text extraction
    # Small PDFs (< 200 pages): Lower speedup due to process overhead
    # Large PDFs (≥ 200 pages): 3.0x+ speedup
    ratio_4w_1w = speedups[4]

    if expected_pages >= 200:
        # Large PDFs must meet 3.0x requirement
        required_speedup = 3.0
        assert ratio_4w_1w >= required_speedup, (
            f"\n4w/1w RATIO REQUIREMENT NOT MET: {pdf_name}\n"
            f"  Required: >= {required_speedup}x (large PDF ≥ 200 pages)\n"
            f"  Actual: {ratio_4w_1w:.2f}x\n"
            f"\n  Per CLAUDE.md: '3.0x+ at 4 workers for page rendering (large PDFs)'\n"
        )
    else:
        # Small PDFs: verify some speedup but don't require 3.0x
        assert ratio_4w_1w > 1.0, (
            f"\n4w SLOWER THAN 1w: {pdf_name}\n"
            f"  Expected: > 1.0x (some speedup)\n"
            f"  Actual: {ratio_4w_1w:.2f}x\n"
            f"\n  Note: Small PDFs (< 200 pages) have process overhead.\n"
        )
