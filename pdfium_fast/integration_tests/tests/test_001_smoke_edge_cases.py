"""
Smoke Test - Edge Cases (10 tests, ~15-20 seconds)

Single-page tests covering edge cases not in main smoke suite:
- Arabic/RTL text
- Emoji/supplementary Unicode
- Image-heavy PDFs
- CJK mixed text
- Scanned PDFs (fax)
- Tiny PDFs
- Math notation
- Forms with annotations
- Transparency/alpha blending
- Rotated/transformed pages

RUN: pytest -m edge_cases tests/test_001_smoke_edge_cases.py
"""

import os
import json
import pytest
import tempfile
from pathlib import Path
import validation


# 10 edge case PDFs (mostly single pages for speed)
# Format: (test_id, pdf_name, start_page, end_page, category)
# start_page/end_page = None means full PDF
EDGE_CASE_PDFS = [
    ("arabic_rtl", "cc_012_244p.pdf", 50, 50, "arabic_text"),
    ("emoji_unicode", "web_012.pdf", 11, 11, "supplementary_unicode"),  # web_012.pdf has 12 pages (0-11)
    ("large_images", "0291pages_RGLWPWLY4JE6RPNCCHQ3LQWDFLPBGHXQ.pdf", 1, 1, "jpeg_heavy"),
    ("cjk_mixed", "edinet_2025-06-26_0914_E01057_SOFT99corporation.pdf", 75, 75, "japanese_kanji"),
    ("scanned_fax", "fax_ccitt.pdf", None, None, "scanned_document"),  # Full PDF (small)
    ("tiny_pdf", "arxiv_005.pdf", None, None, "small_pdf"),  # Full PDF (5 pages)
    ("math_notation", "arxiv_009.pdf", 3, 3, "latex_math"),
    ("forms_annotations", "0100pages_7FKQLKX273JBHXAAW5XDRT27JGMIZMCI.pdf", 50, 50, "pdf_forms"),
    ("transparency", "web_039.pdf", 1, 1, "alpha_blending"),
    ("rotated_pages", "0130pages_ZJJJ6P4UAGH7LKLACPT5P437FB5F3MYF.pdf", 65, 65, "page_rotation"),
]


def pytest_generate_tests(metafunc):
    """Parametrize tests with edge case PDFs."""
    if "edge_case_pdf_info" in metafunc.fixturenames:
        metafunc.parametrize(
            "edge_case_pdf_info",
            EDGE_CASE_PDFS,
            ids=[pdf_id for pdf_id, _, _, _, _ in EDGE_CASE_PDFS]
        )


@pytest.mark.smoke
@pytest.mark.edge_cases
@pytest.mark.text
def test_edge_case_text_extraction(
    edge_case_pdf_info,
    benchmark_pdfs,
    optimized_lib,
    extract_text_tool,
    request
):
    """
    Test text extraction on edge case PDF (single page or small PDF).

    META:
      id: smoke_edge_text_001
      category: correctness
      level: smoke
      type: text
      pdf_count: 1
      duration: 1-2s per test
      workers: [1]
      validates: Edge case text extraction correctness
      impact: high

    DESCRIPTION:
      Validates that text extraction works on diverse edge cases:
      - Arabic/RTL text
      - Emoji and supplementary Unicode
      - Image-heavy PDFs
      - CJK mixed scripts
      - Scanned documents
      - Very small PDFs
      - Math notation
      - Forms with annotations
      - Transparency/alpha blending
      - Rotated/transformed pages

    SUCCESS:
      - Extraction completes (returncode = 0)
      - Output file created
      - No crashes or hangs

    FAILURE:
      - Check if PDF is corrupted
      - Verify binary is built correctly
      - Check for memory issues with ASan
    """
    pdf_id, pdf_name, start_page, end_page, pdf_category = edge_case_pdf_info
    pdf_path = benchmark_pdfs / pdf_name

    if not pdf_path.exists():
        pytest.skip(f"PDF not found: {pdf_name}")

    # Create temp output
    with tempfile.NamedTemporaryFile(mode='w', suffix='.txt', delete=False) as tmp:
        output_file = Path(tmp.name)

    try:
        # Note: Single-page extraction not supported by current CLI tools
        # Extract full PDF (page range support is a future enhancement)
        # For now, smoke test just verifies the tool doesn't crash

        # Extract with 1 worker
        success = pytest.extract_text(pdf_path, output_file, 1, optimized_lib, extract_text_tool)

        # Capture metadata for telemetry
        request.node._report_pdf_name = pdf_name
        request.node._report_worker_count = 1

        # Verify success
        assert success, f"Edge case extraction failed for {pdf_name}"
        assert output_file.exists(), f"Output file not created: {output_file}"

    finally:
        # Cleanup
        if output_file.exists():
            output_file.unlink()


@pytest.mark.smoke
@pytest.mark.edge_cases
@pytest.mark.image
def test_edge_case_image_rendering(
    edge_case_pdf_info,
    benchmark_pdfs,
    optimized_lib,
    render_tool,
    request
):
    """
    Test image rendering on edge case PDF (single page or small PDF).

    META:
      id: smoke_edge_image_001
      category: correctness
      level: smoke
      type: image
      pdf_count: 1
      duration: 1-2s per test
      workers: [1]
      validates: Edge case image rendering correctness
      impact: high

    DESCRIPTION:
      Validates that image rendering works on diverse edge cases:
      - Arabic/RTL text rendering
      - Emoji and supplementary Unicode rendering
      - Image-heavy PDFs (JPEG decompression)
      - CJK mixed scripts rendering
      - Scanned documents (JPEG extraction)
      - Very small PDFs
      - Math notation rendering
      - Forms with annotations (FPDF_FFLDraw)
      - Transparency/alpha blending (AGG renderer)
      - Rotated/transformed pages

    SUCCESS:
      - Rendering completes (returncode = 0)
      - All pages rendered
      - No crashes

    FAILURE:
      - Check if PDF is corrupted
      - Verify binary is built correctly
      - Check for memory issues with ASan
    """
    pdf_id, pdf_name, start_page, end_page, pdf_category = edge_case_pdf_info
    pdf_path = benchmark_pdfs / pdf_name

    if not pdf_path.exists():
        pytest.skip(f"PDF not found: {pdf_name}")

    # Render specified page range only (use page range feature)
    # Render with 1 worker
    pages, seconds = pytest.render_parallel(pdf_path, 1, optimized_lib, render_tool,
                                           start_page=start_page, end_page=end_page)

    # Capture telemetry
    request.node._report_pdf_name = pdf_name
    request.node._report_worker_count = 1

    # Verify success
    assert pages is not None and seconds is not None, f"Rendering failed for {pdf_name} (binary returned error)"
    assert pages > 0, f"No pages rendered for {pdf_name}"
    assert seconds > 0, f"Invalid timing for {pdf_name}"
