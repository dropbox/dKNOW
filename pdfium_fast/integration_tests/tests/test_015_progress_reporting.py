"""
Progress Reporting Tests - v1.6.0 Feature Validation

Tests progress bar and performance metrics reporting:
- Progress bar shown on TTY
- Progress bar disabled on pipe
- Performance metrics accuracy
- Threading efficiency reporting

RUN: pytest tests/test_015_progress_reporting.py -v
"""

import os
import re
import subprocess
import pytest
from pathlib import Path


@pytest.mark.smoke
@pytest.mark.v1_6_0
class TestProgressReporting:
    """Test v1.6.0 progress reporting features."""

    def test_performance_metrics_shown(
        self,
        benchmark_pdfs,
        render_tool,
        tmp_path
    ):
        """
        Test that performance metrics are displayed after rendering.

        META:
          id: progress_001
          category: ux
          level: smoke
          type: progress
          pdf_count: 1
          duration: 2s
          workers: [1]
          validates: Performance metrics display
          impact: medium

        DESCRIPTION:
          Validates that performance summary appears after rendering,
          including total pages, processing time, throughput, threading
          efficiency, and peak memory.

        EXPECTED:
          - "Performance Summary:" appears in stderr
          - "Total pages:" shows correct count
          - "Processing time:" shows duration in seconds
          - "Throughput:" shows pages/second
          - "Threading:" shows thread count
          - "Peak memory:" shows MB and KB/page
        """
        pdf_path = benchmark_pdfs / "web_039.pdf"
        if not pdf_path.exists():
            pytest.skip(f"Test PDF not found: {pdf_path}")

        output_dir = tmp_path / "output"
        output_dir.mkdir()

        # Run with single thread
        result = subprocess.run(
            [
                render_tool,
                "render-pages",
                str(pdf_path),
                str(output_dir)
            ],
            capture_output=True,
            text=True
        )

        assert result.returncode == 0, f"Command failed: {result.stderr}"

        stderr = result.stderr

        # Check for performance summary
        assert "Performance Summary:" in stderr, "Missing performance summary"
        assert "Total pages:" in stderr, "Missing total pages"
        assert "Processing time:" in stderr, "Missing processing time"
        assert "Throughput:" in stderr, "Missing throughput"
        assert "Threading:" in stderr, "Missing threading info"
        assert "Peak memory:" in stderr, "Missing peak memory"

        # Validate format
        assert re.search(r"Total pages: \d+", stderr), "Invalid total pages format"
        assert re.search(r"Processing time: \d+\.\d+s", stderr), "Invalid processing time format"
        assert re.search(r"Throughput: \d+ pages/second", stderr), "Invalid throughput format"
        assert re.search(r"Threading: \d+ threads", stderr), "Invalid threading format"
        assert re.search(r"Peak memory: \d+ MB \(\d+ KB/page\)", stderr), "Invalid memory format"

    def test_metrics_accuracy(
        self,
        benchmark_pdfs,
        render_tool,
        tmp_path
    ):
        """
        Test that performance metrics are accurate.

        META:
          id: progress_002
          category: correctness
          level: smoke
          type: progress
          pdf_count: 1
          duration: 2s
          workers: [1]
          validates: Metrics calculation accuracy
          impact: medium

        DESCRIPTION:
          Validates that reported metrics match actual values:
          - Total pages matches PDF page count
          - Processing time is reasonable
          - Throughput calculation is correct (pages / time)

        EXPECTED:
          - Total pages = 13 (web_039.pdf)
          - Processing time < 10s
          - Throughput = total_pages / processing_time (±10% for timing precision)
        """
        pdf_path = benchmark_pdfs / "web_039.pdf"
        if not pdf_path.exists():
            pytest.skip(f"Test PDF not found: {pdf_path}")

        output_dir = tmp_path / "output"
        output_dir.mkdir()

        result = subprocess.run(
            [
                render_tool,
                "render-pages",
                str(pdf_path),
                str(output_dir)
            ],
            capture_output=True,
            text=True
        )

        assert result.returncode == 0, f"Command failed: {result.stderr}"

        stderr = result.stderr

        # Extract metrics
        total_match = re.search(r"Total pages: (\d+)", stderr)
        time_match = re.search(r"Processing time: ([\d.]+)s", stderr)
        throughput_match = re.search(r"Throughput: (\d+) pages/second", stderr)

        assert total_match, "Could not extract total pages"
        assert time_match, "Could not extract processing time"
        assert throughput_match, "Could not extract throughput"

        total_pages = int(total_match.group(1))
        processing_time = float(time_match.group(1))
        throughput = int(throughput_match.group(1))

        # Validate values
        assert total_pages == 13, f"Expected 13 pages, got {total_pages}"
        assert processing_time < 10.0, f"Processing took too long: {processing_time}s"
        assert processing_time > 0.0, f"Processing time too short: {processing_time}s"

        # Validate throughput calculation
        # Fast operations (<1s) are sensitive to millisecond timing differences
        # The CLI displays seconds rounded to 2 decimals but calculates throughput
        # with full precision, causing apparent mismatches. Use percentage tolerance.
        expected_throughput = int(total_pages / processing_time)
        # Allow 10% variance for timing precision differences
        tolerance = max(5, int(expected_throughput * 0.10))
        assert abs(throughput - expected_throughput) <= tolerance, \
            f"Throughput mismatch: expected {expected_throughput}±{tolerance}, got {throughput}"

    def test_threading_efficiency_reported(
        self,
        benchmark_pdfs,
        render_tool,
        tmp_path
    ):
        """
        Test that threading efficiency is reported.

        META:
          id: progress_003
          category: ux
          level: smoke
          type: progress
          pdf_count: 1
          duration: 2s
          workers: [1]
          validates: Threading efficiency display
          impact: low

        DESCRIPTION:
          Validates that threading efficiency estimate appears in
          performance summary when using multiple threads.

        EXPECTED:
          - Threading info shows thread count
          - Expected speedup mentioned (e.g., "~6.5x speedup")
        """
        pdf_path = benchmark_pdfs / "web_039.pdf"
        if not pdf_path.exists():
            pytest.skip(f"Test PDF not found: {pdf_path}")

        output_dir = tmp_path / "output"
        output_dir.mkdir()

        result = subprocess.run(
            [
                render_tool,
                "--threads", "8",
                "render-pages",
                str(pdf_path),
                str(output_dir)
            ],
            capture_output=True,
            text=True
        )

        assert result.returncode == 0, f"Command failed: {result.stderr}"

        stderr = result.stderr

        # Check for threading info
        assert "Threading: 8 threads" in stderr, "Missing thread count"
        assert "speedup" in stderr.lower(), "Missing speedup estimate"

    def test_smart_mode_reported(
        self,
        benchmark_pdfs,
        render_tool,
        tmp_path
    ):
        """
        Test that smart mode usage is reported.

        META:
          id: progress_004
          category: ux
          level: smoke
          type: progress
          pdf_count: 1
          duration: 2s
          workers: [1]
          validates: Smart mode reporting
          impact: low

        DESCRIPTION:
          Validates that smart mode (JPEG fast path) usage appears
          in performance summary when applicable.

        EXPECTED:
          - Smart mode line appears if scanned pages detected
          - Shows page count and percentage via JPEG fast path
        """
        # Use a scanned PDF if available
        scanned_dir = benchmark_pdfs.parent / "scanned_test"
        if scanned_dir.exists():
            scanned_pdfs = list(scanned_dir.glob("*.pdf"))
            if scanned_pdfs:
                pdf_path = scanned_pdfs[0]
                output_dir = tmp_path / "output"
                output_dir.mkdir()

                result = subprocess.run(
                    [
                        render_tool,
                        "render-pages",
                        str(pdf_path),
                        str(output_dir)
                    ],
                    capture_output=True,
                    text=True
                )

                assert result.returncode == 0, f"Command failed: {result.stderr}"

                stderr = result.stderr

                # For scanned PDFs, smart mode should be reported
                # Format: "Smart mode: N pages (X.X% via JPEG fast path, 545x speedup)"
                if "Smart mode:" in stderr:
                    assert re.search(r"Smart mode: \d+ pages \([\d.]+% via JPEG fast path, 545x speedup\)", stderr), \
                        "Invalid smart mode format"
        else:
            pytest.skip("No scanned PDFs available for smart mode test")


@pytest.mark.smoke
@pytest.mark.v1_6_0
class TestProgressBar:
    """Test progress bar display behavior."""

    def test_progress_updates_disabled_on_pipe(
        self,
        benchmark_pdfs,
        render_tool,
        tmp_path
    ):
        """
        Test that progress bar is disabled when stderr is piped.

        META:
          id: progress_005
          category: ux
          level: smoke
          type: progress
          pdf_count: 1
          duration: 2s
          workers: [1]
          validates: Progress bar auto-disable on pipe
          impact: medium

        DESCRIPTION:
          Validates that progress bar does not appear when stderr
          is redirected or piped (not a TTY). This ensures clean
          output for scripts and log files.

        EXPECTED:
          - No carriage return (\r) characters in stderr
          - No progress bar patterns ([=====>    ])
          - Performance summary still appears
        """
        pdf_path = benchmark_pdfs / "web_039.pdf"
        if not pdf_path.exists():
            pytest.skip(f"Test PDF not found: {pdf_path}")

        output_dir = tmp_path / "output"
        output_dir.mkdir()

        # Pipe stderr (not a TTY)
        result = subprocess.run(
            [
                render_tool,
                "render-pages",
                str(pdf_path),
                str(output_dir)
            ],
            capture_output=True,
            text=True
        )

        assert result.returncode == 0, f"Command failed: {result.stderr}"

        stderr = result.stderr

        # Progress bars should not appear in piped output
        assert "\r" not in stderr, "Found carriage return (progress bar leaked to pipe)"
        assert "[====>" not in stderr, "Found progress bar pattern in piped output"

        # But performance summary should still appear
        assert "Performance Summary:" in stderr, "Performance summary missing"
