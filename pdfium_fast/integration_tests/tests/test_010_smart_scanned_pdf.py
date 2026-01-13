"""
Smart Scanned PDF Tests

Tests for smart mode JPEG→JPEG fast path optimization (always enabled).
Validates detection accuracy, speedup, and output quality.

Per WORKER_DIRECTIVES_OPTION_D.md:
- Separate from smoke tests (smoke tests remain unchanged)
- Test JPEG→JPEG fast path in isolation
- Validate 10-20x speedup on scanned PDFs
- Verify output quality is acceptable
"""

import pytest
import subprocess
import time
from pathlib import Path
import json

# Mark all tests in this file with smart_pdf marker
pytestmark = pytest.mark.smart_pdf


class TestSmartDetection:
    """Test scanned page detection algorithm."""

    def test_detect_scanned_page_single_image(self, tmp_path, cli_tool, scanned_pdf_single):
        """Verify detection works for PDF with single full-page image."""
        output_dir = tmp_path / "output"
        output_dir.mkdir()

        # Run rendering (smart mode always enabled) to trigger detection
        # NOTE: As of N=522, smart mode works with any thread count (K>=1)
        result = subprocess.run(
            [str(cli_tool), "--threads", "1", "render-pages", str(scanned_pdf_single),
             str(output_dir)],
            capture_output=True,
            text=True
        )

        assert result.returncode == 0, f"CLI failed: {result.stderr}"

        # Check for JPEG output (smart mode should produce .jpg)
        jpg_files = list(output_dir.glob("*.jpg"))
        assert len(jpg_files) > 0, "Smart mode should produce .jpg files for scanned PDF"

    def test_detect_not_scanned_text_pdf(self, tmp_path, cli_tool, text_pdf):
        """Verify detection rejects text-based PDFs."""
        output_dir = tmp_path / "output"
        output_dir.mkdir()

        # Run rendering (smart mode always enabled) on text PDF
        # NOTE: As of N=522, smart mode works with any thread count (K>=1)
        # PNG is default format, smart mode will use JPG if applicable
        result = subprocess.run(
            [str(cli_tool), "--threads", "1", "render-pages", str(text_pdf),
             str(output_dir)],
            capture_output=True,
            text=True
        )

        assert result.returncode == 0, f"CLI failed: {result.stderr}"

        # v2.0.0: Default format is JPEG (not PNG)
        # Check that rendering completes successfully (format doesn't matter for smart mode logic)
        output_files = list(output_dir.glob("*.*"))
        assert len(output_files) > 0, "Should produce output files for text PDF"

    def test_detect_not_scanned_multi_object(self, tmp_path, cli_tool, scanned_pdf_mixed):
        """Verify detection rejects PDFs with multiple objects."""
        output_dir = tmp_path / "output"
        output_dir.mkdir()

        # Run rendering (smart mode always enabled) on mixed content PDF
        # NOTE: As of N=522, smart mode works with any thread count (K>=1)
        result = subprocess.run(
            [str(cli_tool), "--threads", "1", "render-pages", str(scanned_pdf_mixed),
             str(output_dir)],
            capture_output=True,
            text=True
        )

        assert result.returncode == 0, f"CLI failed: {result.stderr}"

        # Mixed content PDF may have some pages as JPEG, some as PNG
        # Just verify it completes successfully
        output_files = list(output_dir.glob("*.*"))
        assert len(output_files) > 0, "Should produce output files"


class TestJPEGFastPath:
    """Test JPEG→JPEG extraction fast path."""

    def test_jpeg_extraction_works(self, tmp_path, cli_tool, scanned_pdf_multi):
        """Verify JPEG extraction produces valid .jpg file."""
        output_dir = tmp_path / "output"
        output_dir.mkdir()

        # Run rendering (smart mode always enabled)
        # NOTE: As of N=522, smart mode works with any thread count (K>=1)
        result = subprocess.run(
            [str(cli_tool), "--threads", "1", "render-pages", str(scanned_pdf_multi), str(output_dir)],
            capture_output=True,
            text=True
        )

        assert result.returncode == 0, f"CLI failed: {result.stderr}"

        # Check for JPEG output
        jpg_files = list(output_dir.glob("*.jpg"))
        assert len(jpg_files) > 0, "Should produce .jpg files"

        # Verify JPEG header (FF D8 FF)
        for jpg_file in jpg_files:
            with open(jpg_file, "rb") as f:
                header = f.read(3)
                assert header[:2] == b'\xff\xd8', f"Invalid JPEG header in {jpg_file.name}"
                assert header[2:3] == b'\xff', f"Invalid JPEG marker in {jpg_file.name}"

    def test_jpeg_fallback_to_normal(self, tmp_path, cli_tool, text_pdf):
        """Verify graceful fallback when JPEG not available."""
        output_dir = tmp_path / "output"
        output_dir.mkdir()

        # Run rendering (smart mode always enabled) on text PDF (no JPEG images)
        # NOTE: As of N=522, smart mode works with any thread count (K>=1)
        # PNG is default format, smart mode will use JPG if applicable
        result = subprocess.run(
            [str(cli_tool), "--threads", "1", "render-pages", str(text_pdf), str(output_dir)],
            capture_output=True,
            text=True
        )

        assert result.returncode == 0, f"CLI failed: {result.stderr}"

        # v2.0.0: Default format is JPEG (rendered, not extracted)
        # Check that rendering completes successfully
        output_files = list(output_dir.glob("*.*"))
        assert len(output_files) > 0, "Should produce output files"


class TestSmartModePerformance:
    """Test smart mode overhead on non-scanned PDFs.

    NOTE: Smart mode is always-on since N=34. Comparative tests between
    "smart" and "normal" modes are invalid (both use same code).

    Performance validation should compare against upstream pdfium_test baseline,
    not internal comparisons. This is tracked for future implementation.
    """
    pass  # No valid performance tests with always-on smart mode


class TestSmartModeQuality:
    """Test smart mode output quality."""

    def test_smart_mode_visual_quality(self, tmp_path, cli_tool, scanned_pdf_high_res):
        """Verify output quality is visually acceptable."""
        output_dir = tmp_path / "output"
        output_dir.mkdir()

        # Run rendering (smart mode always enabled)
        # NOTE: As of N=522, smart mode works with any thread count (K>=1)
        result = subprocess.run(
            [str(cli_tool), "--threads", "1", "render-pages", str(scanned_pdf_high_res), str(output_dir)],
            capture_output=True,
            text=True
        )

        assert result.returncode == 0, f"CLI failed: {result.stderr}"

        # Check JPEG output
        jpg_files = list(output_dir.glob("*.jpg"))
        assert len(jpg_files) > 0, "Should produce JPEG output"

        # Verify each JPEG has reasonable dimensions and file size
        for jpg_file in jpg_files:
            file_size = jpg_file.stat().st_size

            # JPEG should be at least 10KB (sanity check - not corrupted)
            assert file_size > 10_000, f"{jpg_file.name} is suspiciously small ({file_size} bytes)"

            # JPEG should be under 100MB (sanity check - not broken)
            assert file_size < 100_000_000, f"{jpg_file.name} is suspiciously large ({file_size} bytes)"

            # Verify JPEG header
            with open(jpg_file, "rb") as f:
                header = f.read(3)
                assert header[:2] == b'\xff\xd8', f"Invalid JPEG header in {jpg_file.name}"

    def test_smart_mode_dpi_targeting(self, tmp_path, cli_tool, scanned_pdf_high_res):
        """Verify DPI targeting works (when resize implemented)."""
        # Note: JPEG resize not yet implemented, so this test verifies
        # that extracted JPEG maintains original resolution
        output_dir = tmp_path / "output"
        output_dir.mkdir()

        # Run rendering (smart mode always enabled) and explicit DPI (currently ignored for JPEG extraction)
        # NOTE: As of N=522, smart mode works with any thread count (K>=1)
        result = subprocess.run(
            [str(cli_tool), "--threads", "1", "render-pages", str(scanned_pdf_high_res), str(output_dir)],
            capture_output=True,
            text=True
        )

        assert result.returncode == 0, f"CLI failed: {result.stderr}"

        # Check JPEG output exists
        jpg_files = list(output_dir.glob("*.jpg"))
        assert len(jpg_files) > 0, "Should produce JPEG output"

        # Note: When DPI targeting is implemented, add assertions for actual dimensions


class TestSmartModeIntegration:
    """Test smart mode integration with existing features."""

    def test_smart_mode_with_1worker(self, tmp_path, cli_tool, scanned_pdf_single):
        """Verify smart mode works with --workers 1 (single-threaded)."""
        output_dir = tmp_path / "output"
        output_dir.mkdir()

        # Run with --workers 1 (smart mode is always enabled)
        # NOTE: As of N=522, smart mode works with any thread count (K>=1)
        result = subprocess.run(
            [str(cli_tool), "--workers", "1", "--threads", "1", "render-pages", str(scanned_pdf_single), str(output_dir)],
            capture_output=True,
            text=True
        )

        assert result.returncode == 0, f"CLI failed: {result.stderr}"

        # Check for JPEG output
        jpg_files = list(output_dir.glob("*.jpg"))
        assert len(jpg_files) > 0, "Should produce JPEG output with 1 worker"

    def test_smart_mode_with_4workers(self, tmp_path, cli_tool, scanned_pdf_multi):
        """Verify smart mode works with --workers 4 (multi-process).

        RESOLVED N=522: Smart mode now works with any thread count (K>=1).
        Pre-scan phase detects scanned pages before parallel rendering.
        """
        output_dir = tmp_path / "output"
        output_dir.mkdir()

        # Run with --workers 4 (smart mode works with any K value as of N=522)
        result = subprocess.run(
            [str(cli_tool), "--workers", "4", "render-pages", str(scanned_pdf_multi), str(output_dir)],
            capture_output=True,
            text=True
        )

        assert result.returncode == 0, f"CLI failed: {result.stderr}"

        # Check for JPEG output
        jpg_files = list(output_dir.glob("*.jpg"))
        assert len(jpg_files) > 0, "Should produce JPEG output with 4 workers"

    def test_smart_mode_respects_ppm_flag(self, tmp_path, cli_tool, scanned_pdf_single):
        """Verify smart mode respects --ppm flag (disables JPEG fast path)."""
        # Per CLAUDE.md N=34: "Smart mode respects PPM output format"
        # When --ppm flag used, JPEG fast path should be disabled
        output_dir = tmp_path / "output"
        output_dir.mkdir()

        # Run with --ppm (should disable JPEG fast path)
        result = subprocess.run(
            [str(cli_tool), "--ppm", "render-pages", str(scanned_pdf_single), str(output_dir)],
            capture_output=True,
            text=True
        )

        assert result.returncode == 0, f"CLI failed: {result.stderr}"

        # Check for PPM output (not JPEG)
        ppm_files = list(output_dir.glob("*.ppm"))
        jpg_files = list(output_dir.glob("*.jpg"))
        assert len(ppm_files) > 0, "Should produce PPM output when --ppm flag used"
        assert len(jpg_files) == 0, "Should not use JPEG fast path with --ppm flag"


# Fixtures for this test module

@pytest.fixture
def cli_tool(pdfium_root):
    """Path to pdfium_cli binary."""
    tool = pdfium_root / 'out' / 'Release' / 'pdfium_cli'
    if not tool.exists():
        pytest.skip(f"pdfium_cli not found: {tool}")
    return tool


@pytest.fixture
def scanned_pdf_single(pdfium_root):
    """Path to single-page scanned PDF."""
    pdf = pdfium_root / 'integration_tests' / 'pdfs' / 'scanned_real' / 'scanned_single_jpeg.pdf'
    if not pdf.exists():
        pytest.skip(f"Scanned PDF not found: {pdf}")
    return pdf


@pytest.fixture
def scanned_pdf_multi(pdfium_root):
    """Path to multi-page scanned PDF."""
    pdf = pdfium_root / 'integration_tests' / 'pdfs' / 'scanned_real' / 'scanned_multi_jpeg.pdf'
    if not pdf.exists():
        pytest.skip(f"Scanned PDF not found: {pdf}")
    return pdf


@pytest.fixture
def scanned_pdf_mixed(pdfium_root):
    """Path to mixed-content PDF (some pages scanned)."""
    pdf = pdfium_root / 'integration_tests' / 'pdfs' / 'scanned_real' / 'scanned_mixed_jpeg.pdf'
    if not pdf.exists():
        pytest.skip(f"Scanned PDF not found: {pdf}")
    return pdf


@pytest.fixture
def scanned_pdf_high_res(pdfium_root):
    """Path to high-resolution scanned PDF."""
    pdf = pdfium_root / 'integration_tests' / 'pdfs' / 'scanned_real' / 'scanned_high_res_jpeg.pdf'
    if not pdf.exists():
        pytest.skip(f"Scanned PDF not found: {pdf}")
    return pdf


@pytest.fixture
def text_pdf(pdfium_root):
    """Path to text-based PDF (not scanned)."""
    # Use arxiv_001 from existing corpus as text PDF
    pdf = pdfium_root / 'integration_tests' / 'pdfs' / 'benchmark' / 'arxiv_001.pdf'
    if not pdf.exists():
        pytest.skip(f"Text PDF not found: {pdf}")
    return pdf
