"""
Extended Corpus Test - All 196 Benchmark PDFs

Tests ALL PDFs in benchmark_pdfs/ directory.

META:
  id: extended_corpus
  category: correctness
  level: extended
  type: text
  pdf_count: 196
  duration: 1h

RUN: pytest -m extended
"""

import pytest
import tempfile
from pathlib import Path
import validation


def get_all_benchmark_pdfs():
    """Get all 196 PDFs from testing/pdfs/benchmark/."""
    pdf_dir = Path(__file__).parent.parent / 'pdfs' / 'benchmark'
    return sorted([p.name for p in pdf_dir.glob('*.pdf')])


def pytest_generate_tests(metafunc):
    """Parametrize with all 196 PDFs."""
    if "pdf_name" in metafunc.fixturenames:
        # Single PDF mode
        single_pdf = metafunc.config.getoption("--pdf")
        if single_pdf:
            pdfs = [single_pdf]
        else:
            pdfs = get_all_benchmark_pdfs()

        metafunc.parametrize("pdf_name", pdfs, ids=lambda p: Path(p).stem[:50])


@pytest.mark.corpus
@pytest.mark.correctness
@pytest.mark.text
def test_extended_text_correctness(
    pdf_name,
    benchmark_pdfs,
    optimized_lib,
    extract_text_tool,
    use_llm,
    request
):
    """
    Test 1-worker vs 4-worker on all 196 benchmark PDFs.

    META:
      id: extended_corpus_001
      category: correctness
      level: extended
      type: text
      pdf_count: 196
      duration: 1h
      workers: [1, 4]
      validates: Parallel text extraction correctness on full corpus
      impact: high

    DESCRIPTION:
      Validates parallel text extraction on ALL 196 PDFs in benchmark_pdfs/.
      This is the comprehensive validation across the full corpus.

    SUCCESS:
      - Edit distance = 0 (perfect match)
      - All 196 PDFs pass

    FAILURE:
      - Identify which PDFs fail
      - Run with --llm on failing PDFs for analysis
      - Check for patterns (font types, page counts, content types)
    """
    pdf_path = benchmark_pdfs / pdf_name

    if not pdf_path.exists():
        pytest.skip(f"PDF not found: {pdf_name}")

    # Temp outputs
    with tempfile.NamedTemporaryFile(mode='w', suffix='_1w.txt', delete=False) as tmp1:
        output_1w = Path(tmp1.name)
    with tempfile.NamedTemporaryFile(mode='w', suffix='_4w.txt', delete=False) as tmp4:
        output_4w = Path(tmp4.name)

    try:
        # Extract with both worker counts
        import time

        start_1w = time.time()
        success_1w = pytest.extract_text(pdf_path, output_1w, 1, optimized_lib, extract_text_tool)
        duration_1w = time.time() - start_1w

        start_4w = time.time()
        success_4w = pytest.extract_text(pdf_path, output_4w, 4, optimized_lib, extract_text_tool)
        duration_4w = time.time() - start_4w

        assert success_1w and success_4w, "Extraction failed"

        # Compare (UTF-32 LE format from Rust tools)
        # Use surrogatepass to handle surrogate pairs if present
        text_1w = output_1w.read_text(encoding='utf-32-le', errors='surrogatepass')
        text_4w = output_4w.read_text(encoding='utf-32-le', errors='surrogatepass')

        matches = (text_1w == text_4w)
        edit_dist = 0 if matches else validation.calculate_edit_distance(text_1w, text_4w)
        similarity = 1.0 if matches else validation.calculate_similarity(text_1w, text_4w)

        # Get PDF size
        pdf_size_mb = pdf_path.stat().st_size / (1024**2)

        # Estimate page count from filename
        pdf_pages = 100  # Default
        if 'pages_' in pdf_name:
            try:
                pdf_pages = int(pdf_name.split('pages_')[0].lstrip('0'))
            except:
                pass

        pps_1w = pdf_pages / duration_1w if duration_1w > 0 else 0
        pps_4w = pdf_pages / duration_4w if duration_4w > 0 else 0
        speedup = pps_4w / pps_1w if pps_1w > 0 else 0

        # Telemetry
        request.node._report_pdf_name = pdf_name
        request.node._report_pdf_pages = pdf_pages
        request.node._report_pdf_size_mb = round(pdf_size_mb, 2)
        request.node._report_worker_count = 4
        request.node._report_text_edit_distance = edit_dist
        request.node._report_text_similarity = round(similarity, 6)
        request.node._report_pages_per_sec = round(pps_4w, 2)
        request.node._report_speedup_vs_1w = round(speedup, 2)

        # LLM (only on failures)
        if use_llm and not matches:
            import difflib
            diff_lines = list(difflib.unified_diff(
                text_1w.splitlines(keepends=True),
                text_4w.splitlines(keepends=True),
                n=3
            ))
            llm_analysis = validation.analyze_text_with_llm(text_1w, text_4w, diff_lines)

            if llm_analysis:
                request.node._report_llm_called = True
                request.node._report_llm_model = 'gpt-4o-mini'
                request.node._report_llm_cost_usd = 0.01

                print(f"\n{'='*70}")
                print(f"LLM ANALYSIS: {pdf_name}")
                print('='*70)
                print(llm_analysis)
                print('='*70)

        assert matches, f"Text mismatch: edit_distance={edit_dist}"

    finally:
        if output_1w.exists():
            output_1w.unlink()
        if output_4w.exists():
            output_4w.unlink()
