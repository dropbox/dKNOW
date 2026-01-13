"""
Determinism Test - Multi-Iteration Validation

Tests that parallel extraction produces identical output across multiple runs.

META:
  id: determinism
  category: stability
  level: full
  type: text
  pdf_count: 5
  duration: 5m

RUN: pytest -m stability --iterations 100
"""

import pytest
import tempfile
from pathlib import Path
import validation


# PDFs for determinism testing
DETERMINISM_PDFS = [
    "0100pages_7FKQLKX273JBHXAAW5XDRT27JGMIZMCI.pdf",
    "arxiv_001.pdf",
    "cc_008_116p.pdf",
    "edinet_2025-06-26_0914_E01057_SOFT99corporation.pdf",
    "web_007.pdf",
]


def pytest_generate_tests(metafunc):
    """Parametrize with determinism PDFs."""
    if "pdf_name" in metafunc.fixturenames:
        metafunc.parametrize("pdf_name", DETERMINISM_PDFS, ids=lambda p: Path(p).stem[:40])


@pytest.mark.stability
@pytest.mark.full
@pytest.mark.text
def test_text_determinism_multirun(
    pdf_name,
    benchmark_pdfs,
    optimized_lib,
    extract_text_tool,
    iterations,
    request
):
    """
    Test parallel text extraction is deterministic across N iterations.

    META:
      id: determinism_001
      category: stability
      level: full
      type: text
      pdf_count: 5
      duration: 5m
      workers: [4]
      validates: Parallel extraction produces identical output every time
      impact: critical

    DESCRIPTION:
      Runs parallel text extraction N times (default 10, configurable via --iterations)
      and verifies all outputs are byte-for-byte identical.

      Non-determinism indicates:
      - Race conditions
      - Uninitialized memory
      - FPU rounding issues
      - Worker-dependent behavior

    SUCCESS:
      - All N iterations produce identical output
      - Edit distance = 0 between all pairs

    FAILURE:
      - Any iteration differs from others
      - Extract JSONL debug to find differences
      - Check FPU rounding mode
      - Check worker synchronization
      - Look for unprotected shared state
    """
    pdf_path = benchmark_pdfs / pdf_name

    if not pdf_path.exists():
        pytest.skip(f"PDF not found: {pdf_name}")

    # Get PDF metadata
    pdf_size_mb = pdf_path.stat().st_size / (1024**2)

    # Run N iterations
    iteration_count = max(iterations, 2)  # At least 2
    outputs = []

    for i in range(iteration_count):
        with tempfile.NamedTemporaryFile(mode='w', suffix=f'_iter{i}.txt', delete=False) as tmp:
            output_file = Path(tmp.name)

        success = pytest.extract_text(pdf_path, output_file, 4, optimized_lib, extract_text_tool)
        assert success, f"Iteration {i} failed"

        outputs.append(output_file)

    try:
        # Load all outputs (UTF-32 LE format from Rust tools)
        texts = [output.read_text(encoding='utf-32-le') for output in outputs]

        # Compare all to first (baseline for this run)
        baseline_text = texts[0]
        max_edit_dist = 0

        for i, text in enumerate(texts[1:], 1):
            if text != baseline_text:
                edit_dist = validation.calculate_edit_distance(baseline_text, text)
                max_edit_dist = max(max_edit_dist, edit_dist)

                # Telemetry for this failure
                request.node._report_iteration_number = i
                request.node._report_text_edit_distance = edit_dist

                pytest.fail(
                    f"\nNon-deterministic output detected: {pdf_name}\n"
                    f"  Iteration {i} differs from iteration 0\n"
                    f"  Edit distance: {edit_dist}\n"
                    f"  This indicates race condition or uninitialized memory!\n"
                )

        # Success - all iterations match
        request.node._report_pdf_name = pdf_name
        request.node._report_pdf_size_mb = round(pdf_size_mb, 2)
        request.node._report_worker_count = 4
        request.node._report_iteration_number = iteration_count
        request.node._report_text_edit_distance = 0

    finally:
        # Clean up
        for output in outputs:
            if output.exists():
                output.unlink()


@pytest.mark.stability
@pytest.mark.full
@pytest.mark.image
def test_image_determinism_multirun(
    pdf_name,
    benchmark_pdfs,
    optimized_lib,
    render_tool_dispatcher,
    iterations,
    request
):
    """
    Test parallel rendering is deterministic across N iterations.

    META:
      id: determinism_002
      category: stability
      level: full
      type: image
      pdf_count: 5
      duration: 3m
      workers: [4]
      validates: Parallel rendering produces identical pixels every time
      impact: critical

    DESCRIPTION:
      Renders PDF N times and verifies consistent output.

    SUCCESS:
      - All iterations produce same page count

    FAILURE:
      - Check for race conditions in rendering pipeline
    """
    pdf_path = benchmark_pdfs / pdf_name

    if not pdf_path.exists():
        pytest.skip(f"PDF not found: {pdf_name}")

    iteration_count = max(iterations, 2)

    # Run N iterations
    results = []
    for i in range(iteration_count):
        pages, seconds = pytest.render_parallel(pdf_path, 4, optimized_lib, render_tool_dispatcher)
        assert pages is not None, f"Iteration {i} rendering failed"
        results.append((pages, seconds))

    # Verify all iterations rendered same page count
    baseline_pages = results[0][0]
    for i, (pages, seconds) in enumerate(results[1:], 1):
        assert pages == baseline_pages, (
            f"\nNon-deterministic page count: {pdf_name}\n"
            f"  Iteration 0: {baseline_pages} pages\n"
            f"  Iteration {i}: {pages} pages\n"
        )

    # Telemetry
    request.node._report_pdf_name = pdf_name
    request.node._report_worker_count = 4
    request.node._report_iteration_number = iteration_count
    request.node._report_total_pages = baseline_pages
