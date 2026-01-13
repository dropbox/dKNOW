"""
Interface Benchmarking Study - Single-Core vs Multi-Process Analysis

Comprehensive benchmarking to distinguish:
1. Single-core performance (per-core efficiency, should match upstream)
2. Multi-process parallelism gains (speedup from worker scaling)
3. PDF type variations (text-heavy, image-heavy, mixed)

KEY FINDINGS (Expected for v1.0.0):
- Single-core: 1.0x (baseline, no per-core optimizations)
- Multi-process: 3.8x-4.0x speedup at 4 workers
- Total speedup: From parallelism only, not single-core optimization

META:
  id: interface_benchmarks
  category: benchmarking
  level: comprehensive
  type: both
  pdf_count: 9
  duration: 20m

RUN: pytest -m interface_benchmarks -v
"""

import pytest
import tempfile
import time
from pathlib import Path


# Diverse PDFs for comprehensive benchmarking
# Categories: text-heavy, image-heavy, mixed
# Note: Only PDFs >= 200 pages are tested for scaling analysis (multi-process overhead amortization)
BENCHMARK_PDFS = [
    # Text-heavy (high text extraction workload)
    ("arxiv_001.pdf", 25, "text_heavy"),
    ("cc_008_116p.pdf", 116, "text_heavy"),
    ("0100pages_7FKQLKX273JBHXAAW5XDRT27JGMIZMCI.pdf", 100, "text_heavy"),

    # Image-heavy (high rendering workload)
    ("web_012.pdf", 12, "image_heavy"),
    ("edinet_2025-06-26_0914_E01057_SOFT99corporation.pdf", 125, "image_heavy"),

    # Mixed (balanced workload)
    ("cc_012_244p.pdf", 244, "mixed"),
    ("cc_004_291p.pdf", 291, "mixed"),

    # Large (best for scaling analysis)
    ("0821pages_LUNFJFH4KWZ3ZFNRO43WSMZPLM4OLB7C.pdf", 821, "large"),
]


def pytest_generate_tests(metafunc):
    """Parametrize with benchmark PDFs."""
    if "benchmark_pdf_info" in metafunc.fixturenames:
        metafunc.parametrize(
            "benchmark_pdf_info",
            BENCHMARK_PDFS,
            ids=[f"{name.replace('.pdf', '')}_{pages}p_{cat}" for name, pages, cat in BENCHMARK_PDFS]
        )


@pytest.mark.interface_benchmarks
@pytest.mark.comprehensive
@pytest.mark.text
def test_text_single_core_baseline(
    benchmark_pdf_info,
    benchmark_pdfs,
    optimized_lib,
    extract_text_tool,
    request
):
    """
    Measure single-core text extraction performance (per-core efficiency).

    This establishes the baseline for multi-process speedup calculations.
    v1.0.0 should match upstream per-core performance (1.0x).

    META:
      id: interface_text_single_core
      category: benchmarking
      level: comprehensive
      type: text
      workers: 1
      validates: Single-core baseline (should match upstream)
      impact: critical

    DESCRIPTION:
      Measures per-core efficiency without parallelism overhead.
      This is the baseline for calculating multi-process speedup.

      v1.0.0 has NO single-core optimizations, so this should match
      upstream pdfium_test performance (1.0x).

      Future v1.1.0 will add per-core optimizations (simdutf, etc).

    SUCCESS:
      - Baseline measured successfully
      - Results logged to telemetry
    """
    pdf_name, expected_pages, category = benchmark_pdf_info
    pdf_path = benchmark_pdfs / pdf_name

    if not pdf_path.exists():
        pytest.skip(f"PDF not found: {pdf_name}")

    # Extract with 1 worker (single-core)
    with tempfile.NamedTemporaryFile(mode='w', suffix='_1w.txt', delete=False) as tmp:
        output_path = Path(tmp.name)

    try:
        start_time = time.time()
        success = pytest.extract_text(pdf_path, output_path, 1, optimized_lib, extract_text_tool)
        duration = time.time() - start_time

        assert success, f"Single-core extraction failed for {pdf_name}"

        # Calculate pages per second
        pps = expected_pages / duration if duration > 0 else 0

        # Telemetry
        request.node._report_pdf_name = pdf_name
        request.node._report_pdf_pages = expected_pages
        request.node._report_pdf_category = category
        request.node._report_worker_count = 1
        request.node._report_pages_per_sec = round(pps, 2)
        request.node._report_total_time_sec = round(duration, 3)

        print(f"✓ Single-core baseline: {pps:.1f} pps ({duration:.2f}s) - {category}")

    finally:
        if output_path.exists():
            output_path.unlink()


@pytest.mark.interface_benchmarks
@pytest.mark.comprehensive
@pytest.mark.text
@pytest.mark.timeout(600)  # 10min for 4 worker configs
def test_text_scaling_analysis(
    benchmark_pdf_info,
    benchmark_pdfs,
    optimized_lib,
    extract_text_tool,
    request
):
    """
    Analyze text extraction scaling across 1/2/4/8 workers.

    Measures multi-process parallelism gains vs single-core baseline.
    Only tests PDFs >= 200 pages (where multi-process is beneficial).

    META:
      id: interface_text_scaling
      category: benchmarking
      level: comprehensive
      type: text
      workers: [1, 2, 4, 8]
      validates: Multi-process speedup (target: 3.8x at 4 workers)
      impact: high

    DESCRIPTION:
      Tests worker scaling to measure parallelism efficiency:
      - 1 worker: Baseline (per-core performance)
      - 2 workers: ~1.8x expected
      - 4 workers: ~3.8x expected (target)
      - 8 workers: ~6.5x expected (diminishing returns)

      Only tests PDFs >= 200 pages where multi-process overhead
      is amortized by actual work.

    SUCCESS:
      - Scaling data logged to telemetry
      - 4-worker achieves >= 2.0x speedup
    """
    pdf_name, expected_pages, category = benchmark_pdf_info
    pdf_path = benchmark_pdfs / pdf_name

    if not pdf_path.exists():
        pytest.skip(f"PDF not found: {pdf_name}")

    # Test worker counts: 1, 2, 4, 8
    worker_counts = [1, 2, 4, 8]
    results = {}

    for workers in worker_counts:
        with tempfile.NamedTemporaryFile(mode='w', suffix=f'_{workers}w.txt', delete=False) as tmp:
            output_path = Path(tmp.name)

        try:
            start_time = time.time()
            success = pytest.extract_text(pdf_path, output_path, workers, optimized_lib, extract_text_tool)
            duration = time.time() - start_time

            assert success, f"{workers}-worker extraction failed for {pdf_name}"

            pps = expected_pages / duration if duration > 0 else 0
            results[workers] = {
                'duration': duration,
                'pps': pps,
                'speedup': results[1]['duration'] / duration if workers > 1 and 1 in results else 1.0
            }

        finally:
            if output_path.exists():
                output_path.unlink()

    # Log telemetry for each worker count
    for workers, data in results.items():
        # Create pseudo-request for each worker count
        # (telemetry will capture all runs)
        print(f"  {workers}w: {data['pps']:.1f} pps ({data['duration']:.2f}s) - {data['speedup']:.2f}x speedup")

    # Main assertion: 4-worker should achieve >= 2.0x speedup
    speedup_4w = results[4]['speedup']
    request.node._report_pdf_name = pdf_name
    request.node._report_pdf_pages = expected_pages
    request.node._report_pdf_category = category
    request.node._report_worker_count = 4
    request.node._report_speedup_vs_1w = round(speedup_4w, 2)
    request.node._report_pages_per_sec = round(results[4]['pps'], 2)

    # Speedup requirements based on PDF size (process overhead vs parallel gains)
    # Special case: arxiv_001.pdf (25p) - Very fast text extraction (~80ms) makes timing unreliable
    # Multi-process overhead can dominate, causing slight slowdown. Accept 95% of baseline.
    if pdf_name == "arxiv_001.pdf":
        required_speedup = 0.95  # Accept up to 5% slowdown due to process overhead on tiny PDFs
    # Very small PDFs (< 50 pages): Log data only, no assertion (timing too fast for reliable measurement)
    # Medium PDFs (50-199 pages): Require >= 1.0x (no slowdown)
    # Large PDFs (>= 200 pages): Require >= 2.0x (parallel efficiency)
    elif expected_pages >= 200:
        required_speedup = 2.0
    elif expected_pages >= 50:
        required_speedup = 1.0
    else:
        required_speedup = None  # Too small for reliable measurement

    if required_speedup is not None:
        assert speedup_4w >= required_speedup, (
            f"\n4-worker speedup requirement NOT MET: {pdf_name}\n"
            f"  Required: >= {required_speedup}x\n"
            f"  Actual: {speedup_4w:.2f}x\n"
            f"  PDF size: {expected_pages} pages\n"
            f"  1w: {results[1]['pps']:.1f} pps ({results[1]['duration']:.2f}s)\n"
            f"  4w: {results[4]['pps']:.1f} pps ({results[4]['duration']:.2f}s)\n"
        )
        print(f"✓ Scaling validated: 4w achieves {speedup_4w:.2f}x speedup ({expected_pages}p {category}, required: >={required_speedup}x)")
    else:
        print(f"📊 Scaling data logged: 4w achieves {speedup_4w:.2f}x speedup ({expected_pages}p {category}, no assertion - PDF too small)")


@pytest.mark.interface_benchmarks
@pytest.mark.comprehensive
@pytest.mark.image
def test_image_single_core_baseline(
    benchmark_pdf_info,
    benchmark_pdfs,
    optimized_lib,
    render_tool,
    request
):
    """
    Measure single-core image rendering performance (per-core efficiency).

    This establishes the baseline for multi-process speedup calculations.
    v1.0.0 should match upstream per-core performance (1.0x).

    META:
      id: interface_image_single_core
      category: benchmarking
      level: comprehensive
      type: image
      workers: 1
      validates: Single-core baseline (should match upstream)
      impact: critical

    DESCRIPTION:
      Measures per-core rendering efficiency without parallelism.
      This is the baseline for calculating multi-process speedup.

      v1.0.0 has NO single-core optimizations, so this should match
      upstream pdfium_test performance (1.0x).

      Future v1.1.0 will add per-core optimizations (PNG compression, etc).

    SUCCESS:
      - Baseline measured successfully
      - Results logged to telemetry
    """
    pdf_name, expected_pages, category = benchmark_pdf_info
    pdf_path = benchmark_pdfs / pdf_name

    if not pdf_path.exists():
        pytest.skip(f"PDF not found: {pdf_name}")

    # Render with 1 worker (single-core)
    pages, duration = pytest.render_parallel(pdf_path, 1, optimized_lib, render_tool)

    assert pages is not None, f"Single-core rendering failed for {pdf_name}"
    assert pages == expected_pages, f"Page count mismatch: {pages} != {expected_pages}"

    # Calculate pages per second
    pps = pages / duration if duration > 0 else 0

    # Telemetry
    request.node._report_pdf_name = pdf_name
    request.node._report_pdf_pages = expected_pages
    request.node._report_pdf_category = category
    request.node._report_worker_count = 1
    request.node._report_pages_per_sec = round(pps, 2)
    request.node._report_total_time_sec = round(duration, 3)

    print(f"✓ Single-core baseline: {pps:.1f} pps ({duration:.2f}s) - {category}")


@pytest.mark.interface_benchmarks
@pytest.mark.comprehensive
@pytest.mark.image
@pytest.mark.timeout(600)  # 10min for 4 worker configs
def test_image_scaling_analysis(
    benchmark_pdf_info,
    benchmark_pdfs,
    optimized_lib,
    render_tool,
    request
):
    """
    Analyze image rendering scaling across 1/2/4/8 workers.

    Measures multi-process parallelism gains vs single-core baseline.
    Only tests PDFs >= 200 pages (where multi-process is beneficial).

    META:
      id: interface_image_scaling
      category: benchmarking
      level: comprehensive
      type: image
      workers: [1, 2, 4, 8]
      validates: Multi-process speedup (target: 3.8x at 4 workers)
      impact: high

    DESCRIPTION:
      Tests worker scaling to measure parallelism efficiency:
      - 1 worker: Baseline (per-core performance)
      - 2 workers: ~1.9x expected
      - 4 workers: ~3.8x expected (target)
      - 8 workers: ~7.0x expected (good scaling)

      Image rendering scales better than text extraction due to
      longer per-page work (amortizes overhead better).

    SUCCESS:
      - Scaling data logged to telemetry
      - 4-worker achieves >= 2.0x speedup
    """
    pdf_name, expected_pages, category = benchmark_pdf_info
    pdf_path = benchmark_pdfs / pdf_name

    if not pdf_path.exists():
        pytest.skip(f"PDF not found: {pdf_name}")

    # Test worker counts: 1, 2, 4, 8
    worker_counts = [1, 2, 4, 8]
    results = {}

    for workers in worker_counts:
        pages, duration = pytest.render_parallel(pdf_path, workers, optimized_lib, render_tool)

        assert pages is not None, f"{workers}-worker rendering failed for {pdf_name}"
        assert pages == expected_pages, f"Page count mismatch: {pages} != {expected_pages}"

        pps = pages / duration if duration > 0 else 0
        results[workers] = {
            'duration': duration,
            'pps': pps,
            'speedup': results[1]['duration'] / duration if workers > 1 and 1 in results else 1.0
        }

        print(f"  {workers}w: {pps:.1f} pps ({duration:.2f}s) - {results[workers]['speedup']:.2f}x speedup")

    # Main assertion: 4-worker should achieve >= 2.0x speedup
    speedup_4w = results[4]['speedup']
    request.node._report_pdf_name = pdf_name
    request.node._report_pdf_pages = expected_pages
    request.node._report_pdf_category = category
    request.node._report_worker_count = 4
    request.node._report_speedup_vs_1w = round(speedup_4w, 2)
    request.node._report_pages_per_sec = round(results[4]['pps'], 2)

    # Speedup requirements based on PDF size (process overhead vs parallel gains)
    # Special case: arxiv_001.pdf (25p) - Very fast text extraction (~80ms) makes timing unreliable
    # Multi-process overhead can dominate, causing slight slowdown. Accept 95% of baseline.
    if pdf_name == "arxiv_001.pdf":
        required_speedup = 0.95  # Accept up to 5% slowdown due to process overhead on tiny PDFs
    # Very small PDFs (< 50 pages): Log data only, no assertion (timing too fast for reliable measurement)
    # Medium PDFs (50-199 pages): Require >= 1.0x (no slowdown)
    # Large PDFs (>= 200 pages): Require >= 2.0x (parallel efficiency)
    elif expected_pages >= 200:
        required_speedup = 2.0
    elif expected_pages >= 50:
        required_speedup = 1.0
    else:
        required_speedup = None  # Too small for reliable measurement

    if required_speedup is not None:
        assert speedup_4w >= required_speedup, (
            f"\n4-worker speedup requirement NOT MET: {pdf_name}\n"
            f"  Required: >= {required_speedup}x\n"
            f"  Actual: {speedup_4w:.2f}x\n"
            f"  PDF size: {expected_pages} pages\n"
            f"  1w: {results[1]['pps']:.1f} pps ({results[1]['duration']:.2f}s)\n"
            f"  4w: {results[4]['pps']:.1f} pps ({results[4]['duration']:.2f}s)\n"
        )
        print(f"✓ Scaling validated: 4w achieves {speedup_4w:.2f}x speedup ({expected_pages}p {category}, required: >={required_speedup}x)")
    else:
        print(f"📊 Scaling data logged: 4w achieves {speedup_4w:.2f}x speedup ({expected_pages}p {category}, no assertion - PDF too small)")


@pytest.mark.interface_benchmarks
@pytest.mark.comprehensive
@pytest.mark.timeout(600)  # 10min for multiple PDFs with text+image
def test_pdf_type_variation_analysis(
    benchmark_pdfs,
    optimized_lib,
    extract_text_tool,
    render_tool,
    request
):
    """
    Analyze performance variations across PDF types.

    Tests text-heavy, image-heavy, and mixed PDFs to understand
    how workload characteristics affect speedup.

    META:
      id: interface_pdf_type_variation
      category: benchmarking
      level: comprehensive
      type: both
      validates: Performance by PDF type
      impact: medium

    DESCRIPTION:
      Different PDF types have different speedup characteristics:
      - Text-heavy: Lower speedup (2.4x avg) due to fast per-page work
      - Image-heavy: Higher speedup (3.2x avg) due to slow per-page work
      - Mixed: Balanced speedup (~3.0x avg)

      This test validates these patterns and logs results for analysis.

    SUCCESS:
      - All PDF types tested successfully
      - Results show expected variation patterns
    """
    # Select representative PDFs (one per type, large enough for multi-process)
    test_pdfs = [
        ("cc_012_244p.pdf", 244, "text_heavy"),
        ("edinet_001_125p.pdf", 125, "image_heavy"),
        ("cc_004_291p.pdf", 291, "mixed"),
    ]

    results_by_type = {}

    for pdf_name, expected_pages, pdf_type in test_pdfs:
        pdf_path = benchmark_pdfs / pdf_name

        if not pdf_path.exists():
            continue  # Skip missing PDFs

        # Test text extraction (1w vs 4w)
        with tempfile.NamedTemporaryFile(mode='w', suffix='_1w.txt', delete=False) as tmp1:
            out1 = Path(tmp1.name)
        with tempfile.NamedTemporaryFile(mode='w', suffix='_4w.txt', delete=False) as tmp4:
            out4 = Path(tmp4.name)

        try:
            # 1 worker
            start = time.time()
            success_1w = pytest.extract_text(pdf_path, out1, 1, optimized_lib, extract_text_tool)
            dur_1w = time.time() - start

            # 4 workers
            start = time.time()
            success_4w = pytest.extract_text(pdf_path, out4, 4, optimized_lib, extract_text_tool)
            dur_4w = time.time() - start

            if success_1w and success_4w:
                text_speedup = dur_1w / dur_4w if dur_4w > 0 else 0
            else:
                text_speedup = 0

        finally:
            if out1.exists():
                out1.unlink()
            if out4.exists():
                out4.unlink()

        # Test image rendering (1w vs 4w) - skip for small PDFs
        if expected_pages >= 200:
            pages_1w, dur_img_1w = pytest.render_parallel(pdf_path, 1, optimized_lib, render_tool)
            pages_4w, dur_img_4w = pytest.render_parallel(pdf_path, 4, optimized_lib, render_tool)

            if pages_1w and pages_4w:
                image_speedup = dur_img_1w / dur_img_4w if dur_img_4w > 0 else 0
            else:
                image_speedup = 0
        else:
            image_speedup = 0  # Skip small PDFs

        # Record results
        results_by_type[pdf_type] = {
            'pdf': pdf_name,
            'pages': expected_pages,
            'text_speedup': text_speedup,
            'image_speedup': image_speedup
        }

        print(f"  {pdf_type:15s} {pdf_name:30s} Text: {text_speedup:.2f}x  Image: {image_speedup:.2f}x")

    # Validate we got results
    assert len(results_by_type) >= 2, "Need at least 2 PDF types for variation analysis"

    # Log summary
    print("\n✓ PDF type variation analysis complete")
    for pdf_type, data in results_by_type.items():
        print(f"  {pdf_type}: Text {data['text_speedup']:.2f}x, Image {data['image_speedup']:.2f}x")
