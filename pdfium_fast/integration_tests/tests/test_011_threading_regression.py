"""
Threading Regression Tests - Prevent Thread-Safety Issues

Tests to ensure Phase 1 atomic singletons and Phase 2 multithreaded
rendering don't introduce crashes, races, or non-determinism.

RUN: pytest -m threading
RUN: pytest -m smoke (includes threading_smoke tests)
"""

import os
import sys
import json
import pytest
import tempfile
import subprocess
from pathlib import Path
import hashlib


# Test PDFs for threading validation
THREADING_TEST_PDFS = [
    # Small PDF (tests threading overhead doesn't hurt small docs)
    ("small_pdf", "web_007.pdf", 50),

    # Medium PDF (tests determinism)
    ("medium_pdf", "0100pages_7FKQLKX273JBHXAAW5XDRT27JGMIZMCI.pdf", 100),

    # Large PDF (tests parallel speedup)
    ("large_pdf", "cc_008_116p.pdf", 116),
]


def pytest_generate_tests(metafunc):
    """Parametrize tests with threading test PDFs."""
    if "threading_pdf_info" in metafunc.fixturenames:
        metafunc.parametrize(
            "threading_pdf_info",
            THREADING_TEST_PDFS,
            ids=[pdf_id for pdf_id, _, _ in THREADING_TEST_PDFS]
        )


# Module-level function for multiprocessing (must be picklable)
def _init_worker_for_multiprocessing_test(lib_path):
    """Worker that initializes PDFium."""
    try:
        from ctypes import CDLL
        lib = CDLL(lib_path)
        lib.FPDF_InitLibrary()
        lib.FPDF_DestroyLibrary()
        return True
    except Exception as e:
        return False


@pytest.mark.threading
@pytest.mark.smoke
class TestThreadingSafety:
    """Thread-safety regression tests from Phase 1."""

    def test_threading_smoke_init_is_thread_safe(
        self,
        optimized_lib,
        request
    ):
        """
        Test that FPDF_InitLibrary can be called safely from multiple processes.

        META:
          id: threading_001
          category: thread_safety
          level: smoke
          validates: Phase 1 atomic init flag works
          impact: critical

        DESCRIPTION:
          Validates that multiple concurrent process spawns don't cause
          init races or double-initialization. Uses Python multiprocessing
          to simulate parallel startup.

        SUCCESS:
          - All processes complete successfully
          - No crashes or assertion failures
          - Library properly initialized in each process

        RELATED:
          - Phase 1 (N=17): Atomic g_bLibraryInitialized flag
          - Phase 1 (N=21): Atomic singleton patterns
        """
        import multiprocessing

        # Spawn 4 processes that all init PDFium simultaneously
        with multiprocessing.Pool(4) as pool:
            results = pool.map(_init_worker_for_multiprocessing_test, [str(optimized_lib)] * 4)

        assert all(results), "Some processes failed to init PDFium"


    def test_threading_smoke_no_crashes_with_workers(
        self,
        threading_pdf_info,
        benchmark_pdfs,
        extract_text_tool,
        render_tool,
        request
    ):
        """
        Test that 4-worker mode doesn't crash on diverse PDFs.

        META:
          id: threading_002
          category: thread_safety
          level: smoke
          workers: [4]
          validates: Multiprocess doesn't crash
          impact: critical

        DESCRIPTION:
          Smoke test that 4-worker text extraction and image rendering
          complete without crashes. This is the basic stability check.

        SUCCESS:
          - Text extraction: exit code 0
          - Image rendering: exit code 0
          - Output files created

        FAILURE:
          - Check if specific PDF triggers crash
          - Run with ASan to find memory issues
          - Check WORKER logs for race conditions
        """
        pdf_id, pdf_name, pdf_pages = threading_pdf_info
        pdf_path = benchmark_pdfs / pdf_name

        if not pdf_path.exists():
            pytest.skip(f"PDF not found: {pdf_name}")

        # Test 4-worker text extraction using C++ CLI
        with tempfile.NamedTemporaryFile(mode='w', suffix='.txt', delete=False) as tmp:
            output_path = Path(tmp.name)

        try:
            # Use C++ CLI fixture
            pdfium_cli = extract_text_tool
            result = subprocess.run(
                [str(pdfium_cli), "--workers", "4", "extract-text", str(pdf_path), str(output_path)],
                capture_output=True,
                text=True,
                timeout=60
            )
            assert result.returncode == 0, f"Text extraction crashed: {result.stderr}"
            assert output_path.exists(), "Text output not created"
        finally:
            if output_path.exists():
                output_path.unlink()

        # Test 4-worker image rendering using C++ CLI
        pdfium_cli = render_tool
        with tempfile.TemporaryDirectory() as tmpdir:
            result = subprocess.run(
                [str(pdfium_cli), "--workers", "4", "render-pages", str(pdf_path), tmpdir],
                capture_output=True,
                text=True,
                timeout=120
            )
            assert result.returncode == 0, f"Image rendering crashed: {result.stderr}"

            # Check at least some pages rendered (v2.0.0: default is JPEG, not PNG)
            rendered_files = list(Path(tmpdir).glob("page_*.jpg")) + list(Path(tmpdir).glob("page_*.png"))
            assert len(rendered_files) > 0, "No images rendered"


@pytest.mark.threading
@pytest.mark.determinism
class TestThreadingDeterminism:
    """Determinism regression tests - multiple runs should be identical."""

    def test_threading_determinism_text_multirun(
        self,
        benchmark_pdfs,
        extract_text_tool,
        request
    ):
        """
        Test that 4-worker text extraction is deterministic across runs.

        META:
          id: threading_003
          category: determinism
          level: threading
          workers: [4]
          iterations: 3
          validates: Text output identical across runs
          impact: high

        DESCRIPTION:
          Runs 4-worker text extraction 3 times and verifies byte-for-byte
          identical output. Non-determinism indicates race conditions.

        SUCCESS:
          - All 3 runs produce identical MD5 hash
          - No variance in output

        FAILURE:
          - Phase 1 atomic singletons may have race
          - Check for unprotected global state
          - Run with ThreadSanitizer (if available)

        RELATED:
          - v1.3.0: Multi-process text is deterministic
          - Phase 1 (N=21): Atomic singletons for memory visibility
        """
        pdf_path = benchmark_pdfs / "web_007.pdf"

        if not pdf_path.exists():
            pytest.skip("Test PDF not found")

        # Use fixture instead of hardcoded path
        pdfium_cli = extract_text_tool

        # Run 3 times
        hashes = []
        for i in range(3):
            with tempfile.NamedTemporaryFile(mode='w', suffix='.txt', delete=False) as tmp:
                output_path = Path(tmp.name)

            try:
                result = subprocess.run(
                    [str(pdfium_cli), "--workers", "4", "extract-text", str(pdf_path), str(output_path)],
                    capture_output=True,
                    text=True,
                    timeout=30
                )
                assert result.returncode == 0, f"Run {i+1} failed: {result.stderr}"

                # Compute MD5
                with open(output_path, 'rb') as f:
                    md5 = hashlib.md5(f.read()).hexdigest()
                    hashes.append(md5)
            finally:
                if output_path.exists():
                    output_path.unlink()

        # All hashes must be identical
        assert len(set(hashes)) == 1, \
            f"Non-deterministic output! Hashes: {hashes}"


    @pytest.mark.smoke
    def test_threading_determinism_image_multirun(
        self,
        benchmark_pdfs,
        render_tool,
        request
    ):
        """
        Test that K=4 multi-threaded rendering is deterministic (MUST PASS).

        META:
          id: threading_004
          category: determinism
          level: smoke
          workers: [1]
          threads: [4]
          iterations: 10
          validates: K=4 deterministic rendering
          impact: CRITICAL

        DESCRIPTION:
          Renders page 0 with K=4 threads, 10 times. ALL runs must produce
          identical MD5s. This is a CRITICAL test - if it fails, there is
          a threading race bug that MUST be fixed.

        SUCCESS:
          - All 10 runs produce identical MD5
          - K=4 rendering is 100% deterministic

        FAILURE MEANS CRITICAL BUG:
          - Threading race condition exists
          - Use TSan to identify: out/TSan/pdfium_cli
          - DO NOT mark xfail - FIX THE BUG
        """
        pdf_path = benchmark_pdfs / "web_007.pdf"

        if not pdf_path.exists():
            pytest.skip("Test PDF not found")

        # Use C++ CLI fixture (pdfium_cli)
        pdfium_cli = render_tool

        # Run 10 times with K=4 threads, collect page 0 MD5
        # 10 iterations gives ~89% chance of catching 20% failure rate
        page0_hashes = []

        for i in range(10):
            with tempfile.TemporaryDirectory() as tmpdir:
                result = subprocess.run(
                    [str(pdfium_cli), "--threads", "4", "--format", "png", "render-pages", str(pdf_path), tmpdir],
                    capture_output=True,
                    text=True,
                    timeout=60
                )
                assert result.returncode == 0, f"Run {i+1} failed: {result.stderr}"

                # Get MD5 of page 0 (CLI uses %05d format)
                page0_png = Path(tmpdir) / "page_00000.png"
                if page0_png.exists():
                    with open(page0_png, 'rb') as f:
                        md5 = hashlib.md5(f.read()).hexdigest()
                        page0_hashes.append(md5)

        # All hashes must be identical - if not, CRITICAL THREADING BUG
        assert len(page0_hashes) == 10, "Not all runs produced page 0"
        unique_hashes = set(page0_hashes)
        assert len(unique_hashes) == 1, \
            f"CRITICAL THREADING RACE: K=4 produces {len(unique_hashes)} different outputs! Fix with TSan. Hashes: {page0_hashes}"

    @pytest.mark.smoke
    def test_threading_determinism_k8_default(
        self,
        benchmark_pdfs,
        render_tool,
        request
    ):
        """
        Test that K=8 (default) multi-threaded rendering is deterministic (MUST PASS).

        META:
          id: threading_005
          category: determinism
          level: smoke
          workers: [1]
          threads: [8]
          iterations: 10
          validates: K=8 deterministic rendering (DEFAULT CONFIG)
          impact: CRITICAL

        DESCRIPTION:
          Renders page 0 with K=8 threads (default), 10 times. ALL runs must
          produce identical MD5s. This tests the default configuration.

        SUCCESS:
          - All 10 runs produce identical MD5
          - K=8 rendering is 100% deterministic

        FAILURE MEANS CRITICAL BUG:
          - Default configuration has threading race
          - Users get non-deterministic output
          - Use TSan to identify and fix
        """
        pdf_path = benchmark_pdfs / "web_007.pdf"

        if not pdf_path.exists():
            pytest.skip("Test PDF not found")

        pdfium_cli = render_tool
        page0_hashes = []

        for i in range(10):
            with tempfile.TemporaryDirectory() as tmpdir:
                result = subprocess.run(
                    [str(pdfium_cli), "--threads", "8", "--format", "png", "render-pages", str(pdf_path), tmpdir],
                    capture_output=True,
                    text=True,
                    timeout=60
                )
                assert result.returncode == 0, f"Run {i+1} failed: {result.stderr}"

                # CLI uses %05d format
                page0_png = Path(tmpdir) / "page_00000.png"
                if page0_png.exists():
                    with open(page0_png, 'rb') as f:
                        md5 = hashlib.md5(f.read()).hexdigest()
                        page0_hashes.append(md5)

        assert len(page0_hashes) == 10, "Not all runs produced page 0"
        unique_hashes = set(page0_hashes)
        assert len(unique_hashes) == 1, \
            f"CRITICAL THREADING RACE IN DEFAULT CONFIG: K=8 produces {len(unique_hashes)} different outputs! Fix with TSan. Hashes: {page0_hashes}"


@pytest.mark.threading
@pytest.mark.performance
class TestThreadingPerformance:
    """Performance regression tests - ensure speedup is maintained."""

    def test_threading_performance_smoke_speedup(
        self,
        benchmark_pdfs,
        render_tool,
        request
    ):
        """
        Test that 4-worker rendering is faster than 1-worker on large PDF.

        META:
          id: threading_005
          category: performance
          level: threading
          workers: [1, 4]
          validates: Parallel speedup > 1.5x
          impact: medium

        DESCRIPTION:
          Smoke test that 4-worker mode is significantly faster than
          1-worker on a 116-page PDF. Ensures threading provides benefit.

        SUCCESS:
          - 4-worker is >= 1.5x faster than 1-worker
          - Both complete successfully

        FAILURE:
          - Check if worker overhead too high
          - Verify thread pool is actually using 4 threads
          - Profile to find bottlenecks

        RELATED:
          - v1.3.0: Achieved 3.4x image speedup with 4 workers
          - Phase 2: Should maintain similar speedup
        """
        import time

        pdf_path = benchmark_pdfs / "cc_008_116p.pdf"

        if not pdf_path.exists():
            pytest.skip("Large test PDF not found")

        # Use C++ CLI fixture (pdfium_cli)
        pdfium_cli = render_tool

        # Time 1-worker (bulk mode = single-threaded)
        # Explicitly set --threads 1 to test ONLY multi-process parallelism (not threading)
        with tempfile.TemporaryDirectory() as tmpdir:
            start = time.time()
            result = subprocess.run(
                [str(pdfium_cli), "--workers", "1", "--threads", "1", "render-pages", str(pdf_path), tmpdir],
                capture_output=True,
                text=True,
                timeout=300
            )
            time_1w = time.time() - start
            assert result.returncode == 0, f"1-worker failed: {result.stderr}"

        # Time 4-workers (fast mode)
        # Explicitly set --threads 1 to test ONLY multi-process parallelism (not threading)
        with tempfile.TemporaryDirectory() as tmpdir:
            start = time.time()
            result = subprocess.run(
                [str(pdfium_cli), "--workers", "4", "--threads", "1", "render-pages", str(pdf_path), tmpdir],
                capture_output=True,
                text=True,
                timeout=300
            )
            time_4w = time.time() - start
            assert result.returncode == 0, f"4-worker failed: {result.stderr}"

        # Calculate speedup
        speedup = time_1w / time_4w

        # Expect at least 1.5x speedup (relaxed for smoke test)
        assert speedup >= 1.5, \
            f"Insufficient speedup: {speedup:.2f}x (expected >= 1.5x). " \
            f"1w: {time_1w:.1f}s, 4w: {time_4w:.1f}s"


@pytest.mark.threading
@pytest.mark.regression
class TestThreadingRegression:
    """Specific regression tests for known threading issues."""

    def test_threading_regression_no_double_init(
        self,
        optimized_lib,
        request
    ):
        """
        Test that double FPDF_InitLibrary doesn't crash.

        META:
          id: threading_006
          category: regression
          level: threading
          validates: Atomic init flag prevents double-init
          impact: high

        DESCRIPTION:
          Calls FPDF_InitLibrary twice without destroy in between.
          Should be a no-op (second call returns immediately).

        SUCCESS:
          - Second init returns immediately
          - No crashes or assertions

        FAILURE:
          - Phase 1 atomic init flag may be broken
          - Check std::atomic<bool> implementation

        RELATED:
          - N=17: Atomic g_bLibraryInitialized flag
        """
        from ctypes import CDLL

        lib = CDLL(str(optimized_lib))

        # First init
        lib.FPDF_InitLibrary()

        # Second init (should be no-op)
        lib.FPDF_InitLibrary()

        # Cleanup
        lib.FPDF_DestroyLibrary()

        # If we get here without crash, test passed


    def test_threading_regression_init_destroy_cycle(
        self,
        optimized_lib,
        request
    ):
        """
        Test that init/destroy cycle works multiple times.

        META:
          id: threading_007
          category: regression
          level: threading
          validates: Atomic singletons can be recreated
          impact: medium

        DESCRIPTION:
          Calls FPDF_InitLibrary/FPDF_DestroyLibrary 3 times in a row.
          Verifies singletons can be properly destroyed and recreated.

        SUCCESS:
          - All 3 cycles complete
          - No crashes or memory leaks

        FAILURE:
          - Check singleton destruction code
          - Verify atomic pointer reset to nullptr

        RELATED:
          - N=21: Atomic singleton Create/Destroy
        """
        from ctypes import CDLL

        lib = CDLL(str(optimized_lib))

        # 3 init/destroy cycles
        for i in range(3):
            lib.FPDF_InitLibrary()
            lib.FPDF_DestroyLibrary()

        # If we get here without crash, test passed
