"""
Edge Case Test - All 254 PDFs in testing/resources/

Tests unusual/malformed PDFs:
- Empty, blank, corrupted
- Encrypted (R2/R3/R5/R6)
- Malformed dictionaries, invalid annotations
- All PDFs in testing/resources/

META:
  id: edge_cases
  category: edge_cases
  level: extended
  type: both
  pdf_count: 254
  duration: 30m

RUN: pytest -m edge_cases
"""

import pytest
import tempfile
from pathlib import Path


def get_all_edge_case_pdfs():
    """Get all 256 PDFs from testing/pdfs/edge_cases/."""
    edge_dir = Path(__file__).parent.parent / 'pdfs' / 'edge_cases'
    if not edge_dir.exists():
        return []

    all_pdfs = sorted([p.name for p in edge_dir.glob('*.pdf')])
    return all_pdfs


def pytest_generate_tests(metafunc):
    """Parametrize with all edge case PDFs."""
    if "edge_pdf_name" in metafunc.fixturenames:
        pdfs = get_all_edge_case_pdfs()

        # Mark known pathological PDFs with xfail (IMAGE rendering only)
        # bug_451265.pdf was fixed in N=232 (pattern cache inheritance fix)
        KNOWN_PATHOLOGICAL_IMAGE = set()  # Empty - bug_451265 fixed

        # Create parameter list with marks
        params = []
        for pdf in pdfs:
            # Apply xfail ONLY to image rendering test
            if pdf in KNOWN_PATHOLOGICAL_IMAGE and metafunc.function.__name__ == "test_edge_case_image_no_crash":
                # Mark as xfail for known upstream bugs
                params.append(pytest.param(pdf, marks=pytest.mark.xfail(
                    reason=f"Upstream bug #451265 - infinite loop in image rendering: {pdf}",
                    strict=True
                )))
            else:
                params.append(pdf)

        metafunc.parametrize("edge_pdf_name", params, ids=lambda p: Path(getattr(p, 'values', [p])[0] if hasattr(p, 'values') else p).stem[:50])


@pytest.mark.edge_cases
@pytest.mark.corpus
@pytest.mark.text
def test_edge_case_text_no_crash(
    edge_pdf_name,
    pdfium_root,
    optimized_lib,
    extract_text_tool,
    request
):
    """
    Test edge case PDFs don't crash during text extraction.

    META:
      id: edge_text_001
      category: edge_cases
      level: extended
      type: text
      pdf_count: 254
      duration: 30m
      workers: [4]
      validates: No crashes on unusual/malformed PDFs
      impact: high

    DESCRIPTION:
      Tests that parallel text extraction gracefully handles
      all edge cases without crashing:
      - Empty PDFs
      - Encrypted PDFs (may fail extraction, that's OK)
      - Corrupted PDFs (bad dictionaries, invalid annotations)
      - Malformed structures

      Success = no crash/hang (extraction may return false)

    SUCCESS:
      - No crashes or hangs
      - Graceful error handling
      - Process completes

    FAILURE:
      - Segfault or crash
      - Infinite loop/hang
      - Unhandled exception
    """
    resources_dir = pdfium_root / 'testing' / 'resources'
    pdf_path = resources_dir / edge_pdf_name

    if not pdf_path.exists():
        pytest.skip(f"Edge case PDF not found: {edge_pdf_name}")

    # Get PDF metadata
    pdf_size_mb = pdf_path.stat().st_size / (1024**2)

    # Telemetry
    request.node._report_pdf_name = edge_pdf_name
    request.node._report_pdf_path = str(pdf_path)
    request.node._report_pdf_size_mb = round(pdf_size_mb, 2)
    request.node._report_pdf_category = 'edge_case'
    request.node._report_worker_count = 4

    with tempfile.NamedTemporaryFile(mode='w', suffix='_edge.txt', delete=False) as tmp:
        output_file = Path(tmp.name)

    try:
        # Should not crash (may return False for encrypted PDFs)
        try:
            success = pytest.extract_text(pdf_path, output_file, 4, optimized_lib, extract_text_tool)
            # Success or failure is OK - just no crash
        except Exception as e:
            pytest.fail(f"CRASHED on edge case {edge_pdf_name}: {e}")

    finally:
        if output_file.exists():
            output_file.unlink()


@pytest.mark.edge_cases
@pytest.mark.corpus
@pytest.mark.image
def test_edge_case_image_no_crash(
    edge_pdf_name,
    pdfium_root,
    optimized_lib,
    render_tool_dispatcher,
    request
):
    """
    Test edge case PDFs don't crash during rendering.

    META:
      id: edge_image_001
      category: edge_cases
      level: extended
      type: image
      pdf_count: 254
      duration: 30m
      workers: [4]
      validates: No crashes on unusual/malformed PDFs
      impact: high

    DESCRIPTION:
      Tests that parallel rendering gracefully handles all edge cases
      without crashing.

    SUCCESS:
      - No crashes or hangs

    FAILURE:
      - Crash, segfault, or hang
    """
    resources_dir = pdfium_root / 'testing' / 'resources'
    pdf_path = resources_dir / edge_pdf_name

    if not pdf_path.exists():
        pytest.skip(f"Edge case PDF not found: {edge_pdf_name}")

    # Telemetry
    request.node._report_pdf_name = edge_pdf_name
    request.node._report_pdf_category = 'edge_case'
    request.node._report_worker_count = 4

    # Known problematic PDFs that upstream PDFium cannot handle
    # These should gracefully timeout/fail, not crash
    # bug_451265.pdf was fixed in N=232 (pattern cache inheritance fix)
    KNOWN_PATHOLOGICAL = set()  # Empty - bug_451265 fixed

    # Should not crash (may return None for encrypted/pathological PDFs)
    try:
        pages, seconds = pytest.render_parallel(pdf_path, 4, optimized_lib, render_tool_dispatcher)

        # For known pathological PDFs, timeout/failure is expected and acceptable
        if edge_pdf_name in KNOWN_PATHOLOGICAL and pages is None:
            pytest.skip(f"Known pathological PDF - upstream PDFium cannot process: {edge_pdf_name}")

        # Success or failure is OK - just no crash
    except Exception as e:
        # Timeout on known pathological PDFs is expected (upstream bug) - mark as xfail
        if edge_pdf_name in KNOWN_PATHOLOGICAL and ("timeout" in str(e).lower() or "timed out" in str(e).lower()):
            pytest.xfail(f"Known upstream bug - infinite loop causes timeout: {edge_pdf_name}")
        pytest.fail(f"CRASHED on edge case {edge_pdf_name}: {e}")
