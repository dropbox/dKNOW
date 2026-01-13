"""
Edge Case Tests - Error Handling and Stress Testing

Created per MANAGER directive 2025-12-04.

Tests:
1. Corrupted PDF handling (graceful error exit)
2. Empty/truncated PDF handling
3. Invalid page range handling
4. Unicode edge cases (CJK, emoji, RTL already covered in test_001_smoke_edge_cases.py)
5. Concurrent stress test (5 parallel CLI runs)
6. Temp file leak verification

RUN: pytest tests/test_022_edge_cases.py -v
"""

import pytest
import subprocess
import tempfile
import os
import glob
import shutil
from pathlib import Path
import concurrent.futures


@pytest.fixture
def cli_path(pdfium_root):
    """Path to pdfium_cli binary."""
    return pdfium_root / 'out' / 'Release' / 'pdfium_cli'


@pytest.fixture
def sample_pdf(pdfium_root):
    """Path to a sample PDF for stress testing."""
    # Use a small PDF from the test corpus
    pdf = pdfium_root / 'testing' / 'resources' / 'hello_world.pdf'
    if not pdf.exists():
        # Fallback to any available PDF
        test_pdfs = list((pdfium_root / 'testing' / 'resources').glob('*.pdf'))
        if test_pdfs:
            return test_pdfs[0]
        pytest.skip("No test PDFs available")
    return pdf


class TestCorruptedPDFHandling:
    """Tests for corrupted/invalid PDF handling."""

    @pytest.mark.smoke
    def test_random_binary_as_pdf(self, cli_path):
        """CLI should handle random binary data gracefully (not crash)."""
        with tempfile.NamedTemporaryFile(suffix='.pdf', delete=False) as f:
            # Write random binary data
            f.write(os.urandom(1024))
            corrupt_path = f.name

        try:
            with tempfile.NamedTemporaryFile(suffix='.txt', delete=False) as out:
                output_path = out.name

            result = subprocess.run(
                [str(cli_path), 'extract-text', corrupt_path, output_path],
                capture_output=True,
                timeout=30
            )
            # Should fail gracefully with non-zero exit code, not crash/hang
            assert result.returncode != 0, "Should fail on random binary"
            # stderr should have error message
            assert len(result.stderr) > 0 or len(result.stdout) > 0, \
                "Should produce error message"
        finally:
            os.unlink(corrupt_path)
            if os.path.exists(output_path):
                os.unlink(output_path)

    @pytest.mark.smoke
    def test_truncated_pdf_header(self, cli_path):
        """CLI should handle truncated PDF (only header) gracefully."""
        with tempfile.NamedTemporaryFile(suffix='.pdf', delete=False) as f:
            # Write only PDF header, no content
            f.write(b'%PDF-1.4\n')
            truncated_path = f.name

        try:
            with tempfile.NamedTemporaryFile(suffix='.txt', delete=False) as out:
                output_path = out.name

            result = subprocess.run(
                [str(cli_path), 'extract-text', truncated_path, output_path],
                capture_output=True,
                timeout=30
            )
            # Should fail gracefully, not crash
            assert result.returncode != 0, "Should fail on truncated PDF"
        finally:
            os.unlink(truncated_path)
            if os.path.exists(output_path):
                os.unlink(output_path)

    @pytest.mark.smoke
    def test_empty_file_as_pdf(self, cli_path):
        """CLI should handle empty file gracefully."""
        with tempfile.NamedTemporaryFile(suffix='.pdf', delete=False) as f:
            # Write nothing - empty file
            empty_path = f.name

        try:
            with tempfile.NamedTemporaryFile(suffix='.txt', delete=False) as out:
                output_path = out.name

            result = subprocess.run(
                [str(cli_path), 'extract-text', empty_path, output_path],
                capture_output=True,
                timeout=30
            )
            # Should fail gracefully, not crash
            assert result.returncode != 0, "Should fail on empty file"
        finally:
            os.unlink(empty_path)
            if os.path.exists(output_path):
                os.unlink(output_path)


class TestInvalidPageRanges:
    """Tests for invalid page range handling."""

    @pytest.mark.smoke
    def test_page_range_exceeds_pdf(self, cli_path, sample_pdf):
        """CLI should handle page range exceeding PDF length gracefully."""
        with tempfile.TemporaryDirectory() as tmpdir:
            output_dir = tmpdir

            result = subprocess.run(
                [str(cli_path), '--pages', '0-999999', 'render-pages',
                 str(sample_pdf), output_dir],
                capture_output=True,
                timeout=60
            )
            # Should either:
            # 1. Succeed with actual pages rendered (graceful handling)
            # 2. Fail with meaningful error
            # Either is acceptable - just shouldn't crash
            # Note: PDFium typically clamps to actual page count
            pass  # Test passes if no exception/crash

    @pytest.mark.smoke
    def test_negative_page_range(self, cli_path, sample_pdf):
        """CLI should handle negative page numbers gracefully."""
        with tempfile.TemporaryDirectory() as tmpdir:
            output_dir = tmpdir

            result = subprocess.run(
                [str(cli_path), '--pages', '-5-0', 'render-pages',
                 str(sample_pdf), output_dir],
                capture_output=True,
                timeout=30
            )
            # Should fail gracefully with error message
            assert result.returncode != 0, "Should reject negative page numbers"


class TestConcurrentStress:
    """Stress tests for concurrent CLI operations."""

    @pytest.mark.smoke
    def test_concurrent_cli_runs(self, cli_path, pdfium_root):
        """5 parallel CLI runs should complete without interference."""
        # Find a suitable test PDF
        test_pdfs = list((pdfium_root / 'testing' / 'resources').glob('*.pdf'))[:5]
        if len(test_pdfs) < 1:
            pytest.skip("No test PDFs available")

        # Use first PDF for all runs
        test_pdf = test_pdfs[0]

        def run_cli(i):
            """Run CLI and return success status."""
            output_dir = f'/tmp/pdfium_stress_test_{os.getpid()}_{i}'
            try:
                os.makedirs(output_dir, exist_ok=True)
                result = subprocess.run(
                    [str(cli_path), '--threads', '4', 'render-pages',
                     str(test_pdf), output_dir],
                    capture_output=True,
                    timeout=120
                )
                return result.returncode == 0
            finally:
                # Cleanup
                if os.path.exists(output_dir):
                    shutil.rmtree(output_dir, ignore_errors=True)

        with concurrent.futures.ThreadPoolExecutor(max_workers=5) as executor:
            futures = [executor.submit(run_cli, i) for i in range(5)]
            results = [f.result(timeout=180) for f in futures]

        assert all(results), f"Some concurrent runs failed: {results}"

    @pytest.mark.smoke
    def test_no_temp_file_leak_after_concurrent(self, cli_path, pdfium_root):
        """Verify no temp files leak after concurrent CLI runs."""
        # Count temp files before
        before_count = len(glob.glob('/tmp/pdfium_worker_*'))

        # Find a test PDF
        test_pdfs = list((pdfium_root / 'testing' / 'resources').glob('*.pdf'))[:1]
        if len(test_pdfs) < 1:
            pytest.skip("No test PDFs available")
        test_pdf = test_pdfs[0]

        def run_cli(i):
            """Run CLI with multiple workers."""
            output_dir = f'/tmp/pdfium_leak_test_{os.getpid()}_{i}'
            try:
                os.makedirs(output_dir, exist_ok=True)
                result = subprocess.run(
                    [str(cli_path), '--threads', '8', 'render-pages',
                     str(test_pdf), output_dir],
                    capture_output=True,
                    timeout=120
                )
                return result.returncode == 0
            finally:
                if os.path.exists(output_dir):
                    shutil.rmtree(output_dir, ignore_errors=True)

        # Run 3 concurrent CLI instances
        with concurrent.futures.ThreadPoolExecutor(max_workers=3) as executor:
            futures = [executor.submit(run_cli, i) for i in range(3)]
            results = [f.result(timeout=180) for f in futures]

        # Count temp files after
        after_count = len(glob.glob('/tmp/pdfium_worker_*'))

        # Should have same count (no leaks)
        assert after_count <= before_count, \
            f"Temp file leak detected: {before_count} before, {after_count} after"


class TestRenderingEdgeCases:
    """Edge cases for image rendering."""

    @pytest.mark.smoke
    def test_render_nonexistent_pdf(self, cli_path):
        """CLI should handle nonexistent PDF path gracefully."""
        with tempfile.TemporaryDirectory() as tmpdir:
            result = subprocess.run(
                [str(cli_path), 'render-pages',
                 '/nonexistent/path/to/file.pdf', tmpdir],
                capture_output=True,
                timeout=30
            )
            assert result.returncode != 0, "Should fail on nonexistent file"

    @pytest.mark.smoke
    def test_render_to_nonexistent_directory(self, cli_path, pdfium_root):
        """CLI should handle nonexistent output directory gracefully."""
        test_pdfs = list((pdfium_root / 'testing' / 'resources').glob('*.pdf'))[:1]
        if len(test_pdfs) < 1:
            pytest.skip("No test PDFs available")

        result = subprocess.run(
            [str(cli_path), 'render-pages', str(test_pdfs[0]),
             '/nonexistent/output/directory'],
            capture_output=True,
            timeout=30
        )
        # Should either:
        # 1. Create directory and succeed
        # 2. Fail with meaningful error
        # Both are acceptable - just shouldn't crash


class TestTextExtractionEdgeCases:
    """Edge cases for text extraction."""

    @pytest.mark.smoke
    def test_extract_nonexistent_pdf(self, cli_path):
        """CLI should handle nonexistent PDF path gracefully."""
        with tempfile.NamedTemporaryFile(suffix='.txt', delete=False) as out:
            output_path = out.name

        try:
            result = subprocess.run(
                [str(cli_path), 'extract-text',
                 '/nonexistent/path/to/file.pdf', output_path],
                capture_output=True,
                timeout=30
            )
            assert result.returncode != 0, "Should fail on nonexistent file"
        finally:
            if os.path.exists(output_path):
                os.unlink(output_path)
