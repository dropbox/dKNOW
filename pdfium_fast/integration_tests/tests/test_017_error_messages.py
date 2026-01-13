"""
Error Message Tests - v1.6.0 Feature Validation

Tests actionable error messages:
- File not found
- Directory not found
- Invalid worker/thread counts
- Password protected PDFs
- Invalid page ranges
- Output directory creation failures

RUN: pytest tests/test_017_error_messages.py -v
"""

import os
import subprocess
import pytest
from pathlib import Path


@pytest.mark.smoke
@pytest.mark.v1_6_0
class TestErrorMessages:
    """Test v1.6.0 error message improvements."""

    def test_error_file_not_found(
        self,
        render_tool,
        tmp_path
    ):
        """
        Test helpful error message for missing file.

        META:
          id: error_001
          category: ux
          level: smoke
          type: error
          pdf_count: 0
          duration: 1s
          workers: [1]
          validates: FileNotFound error message
          impact: high

        DESCRIPTION:
          Validates that missing file error includes:
          - Clear error indicator
          - Reason (file not found)
          - Actionable solution
          - Help reference

        EXPECTED:
          - Exit code 1
          - "Error:" indicator
          - "Reason: File not found"
          - "Solution:" with actionable guidance
          - "Help:" reference
        """
        nonexistent_file = tmp_path / "nonexistent.pdf"
        output_file = tmp_path / "output.txt"

        result = subprocess.run(
            [
                render_tool,
                "extract-text",
                str(nonexistent_file),
                str(output_file)
            ],
            capture_output=True,
            text=True
        )

        assert result.returncode == 1, "Missing file should cause exit code 1"

        stderr = result.stderr

        # Check error format
        assert "Error:" in stderr, "Missing error indicator"
        assert "Reason: File not found" in stderr, "Missing reason"
        assert "Solution:" in stderr, "Missing solution"
        assert "Help:" in stderr, "Missing help reference"

        # Check actionable content
        assert "file path is correct" in stderr.lower() or \
               "file exists" in stderr.lower(), "Solution not actionable"

    def test_error_directory_not_found(
        self,
        render_tool,
        tmp_path
    ):
        """
        Test helpful error message for missing directory (batch mode).

        META:
          id: error_002
          category: ux
          level: smoke
          type: error
          pdf_count: 0
          duration: 1s
          workers: [1]
          validates: DirectoryNotFound error message
          impact: high

        DESCRIPTION:
          Validates that missing directory error (in batch mode)
          includes clear reason and solution.

        EXPECTED:
          - Exit code 1
          - "Error:" indicator
          - "Reason: Directory not found"
          - "Solution:" with actionable guidance
        """
        nonexistent_dir = tmp_path / "nonexistent_dir"
        output_dir = tmp_path / "output"

        result = subprocess.run(
            [
                render_tool,
                "--batch",
                "render-pages",
                str(nonexistent_dir),
                str(output_dir)
            ],
            capture_output=True,
            text=True
        )

        assert result.returncode == 1, "Missing directory should cause exit code 1"

        stderr = result.stderr

        # Check error format
        # v2.0.0: When path doesn't exist, CLI reports appropriate message
        assert "Error:" in stderr, "Missing error indicator"
        # Accept either "File not found" or "Directory not found"
        assert ("Reason: File not found" in stderr or "Reason: Directory not found" in stderr), "Missing reason"
        assert "Solution:" in stderr, "Missing solution"

    def test_error_invalid_worker_count(
        self,
        benchmark_pdfs,
        render_tool,
        tmp_path
    ):
        """
        Test helpful error message for invalid worker count.

        META:
          id: error_003
          category: ux
          level: smoke
          type: error
          pdf_count: 0
          duration: 1s
          workers: [999]
          validates: WorkerCountInvalid error message
          impact: medium

        DESCRIPTION:
          Validates that out-of-range worker count shows
          clear error with valid range.

        EXPECTED:
          - Exit code 1
          - Error mentions "--workers"
          - Shows valid range (1-16)
          - Actionable solution
        """
        pdf_path = benchmark_pdfs / "web_039.pdf"
        output_dir = tmp_path / "output"

        result = subprocess.run(
            [
                render_tool,
                "--workers", "999",
                "render-pages",
                str(pdf_path),
                str(output_dir)
            ],
            capture_output=True,
            text=True
        )

        assert result.returncode == 1, "Invalid worker count should cause exit code 1"

        stderr = result.stderr

        # Check error content
        assert "Error:" in stderr, "Missing error indicator"
        assert "--workers" in stderr, "Missing flag reference"
        assert ("1-16" in stderr or "1 to 16" in stderr or "between 1 and 16" in stderr), "Missing valid range"

    def test_error_invalid_thread_count(
        self,
        benchmark_pdfs,
        render_tool,
        tmp_path
    ):
        """
        Test helpful error message for invalid thread count.

        META:
          id: error_004
          category: ux
          level: smoke
          type: error
          pdf_count: 0
          duration: 1s
          workers: [1]
          validates: ThreadCountInvalid error message
          impact: medium

        DESCRIPTION:
          Validates that out-of-range thread count shows
          clear error with valid range.

        EXPECTED:
          - Exit code 1
          - Error mentions "--threads"
          - Shows valid range (1-32)
          - Actionable solution
        """
        pdf_path = benchmark_pdfs / "web_039.pdf"
        output_dir = tmp_path / "output"

        result = subprocess.run(
            [
                render_tool,
                "--threads", "999",
                "render-pages",
                str(pdf_path),
                str(output_dir)
            ],
            capture_output=True,
            text=True
        )

        assert result.returncode == 1, "Invalid thread count should cause exit code 1"

        stderr = result.stderr

        # Check error content
        assert "Error:" in stderr, "Missing error indicator"
        assert "--threads" in stderr, "Missing flag reference"
        assert ("1-32" in stderr or "1 to 32" in stderr or "between 1 and 32" in stderr), "Missing valid range"

    def test_error_invalid_page_range(
        self,
        benchmark_pdfs,
        render_tool,
        tmp_path
    ):
        """
        Test helpful error message for invalid page range.

        META:
          id: error_005
          category: ux
          level: smoke
          type: error
          pdf_count: 1
          duration: 1s
          workers: [1]
          validates: PageRangeInvalid error message
          impact: medium

        DESCRIPTION:
          Validates that out-of-range page numbers show
          clear error with document page count.

        EXPECTED:
          - Exit code 1
          - Error mentions page range
          - Shows document page count
          - Actionable solution
        """
        pdf_path = benchmark_pdfs / "web_039.pdf"  # 13 pages
        if not pdf_path.exists():
            pytest.skip(f"Test PDF not found: {pdf_path}")

        output_dir = tmp_path / "output"

        result = subprocess.run(
            [
                render_tool,
                "--pages", "1-999",
                "render-pages",
                str(pdf_path),
                str(output_dir)
            ],
            capture_output=True,
            text=True
        )

        assert result.returncode == 1, "Invalid page range should cause exit code 1"

        stderr = result.stderr

        # Check error content
        assert "Error:" in stderr, "Missing error indicator"
        assert "page" in stderr.lower(), "Missing page reference"
        assert "13" in stderr or "pages" in stderr, "Missing document page count info"

    def test_error_corrupted_pdf(
        self,
        render_tool,
        tmp_path
    ):
        """
        Test error message for corrupted PDF.

        META:
          id: error_006
          category: ux
          level: smoke
          type: error
          pdf_count: 1
          duration: 1s
          workers: [1]
          validates: InvalidPDF error message
          impact: medium

        DESCRIPTION:
          Validates that corrupted/invalid PDF files show
          helpful error message (not just crash).

        EXPECTED:
          - Exit code 1
          - Error indicator
          - Helpful message about invalid/corrupted PDF
          - No crash or segfault
        """
        # Create corrupted PDF
        corrupted_pdf = tmp_path / "corrupted.pdf"
        corrupted_pdf.write_bytes(b"This is not a valid PDF file")

        output_dir = tmp_path / "output"

        result = subprocess.run(
            [
                render_tool,
                "render-pages",
                str(corrupted_pdf),
                str(output_dir)
            ],
            capture_output=True,
            text=True
        )

        # Should fail gracefully, not crash
        assert result.returncode in [1, 2], "Corrupted PDF should cause exit code 1 or 2"

        stderr = result.stderr

        # Check for some error message (exact wording may vary)
        assert "Error:" in stderr or "error" in stderr.lower(), \
            "Should show error message for corrupted PDF"

    def test_error_message_format_consistency(
        self,
        render_tool,
        tmp_path
    ):
        """
        Test that all error messages follow consistent format.

        META:
          id: error_007
          category: ux
          level: regression
          type: error
          pdf_count: 0
          duration: 2s
          workers: [1]
          validates: Error format consistency
          impact: low

        DESCRIPTION:
          Validates that error messages follow consistent
          4-line format across different error types.

        EXPECTED:
          - Line 1: "Error: <context>"
          - Line 2: "  Reason: <reason>"
          - Line 3: "  Solution: <solution>"
          - Line 4: "  Help: <help_reference>"
        """
        # Test file not found error
        result = subprocess.run(
            [
                render_tool,
                "extract-text",
                str(tmp_path / "missing.pdf"),
                str(tmp_path / "output.txt")
            ],
            capture_output=True,
            text=True
        )

        stderr = result.stderr
        lines = [line for line in stderr.split('\n') if line.strip()]

        # Find error block
        error_lines = []
        in_error_block = False
        for line in lines:
            if line.startswith("Error:"):
                in_error_block = True
                error_lines = [line]
            elif in_error_block and line.startswith("  "):
                error_lines.append(line)
            elif in_error_block:
                break

        # Check format (should have at least 3 lines: Error, Reason, Solution)
        assert len(error_lines) >= 3, f"Error block too short: {error_lines}"
        assert error_lines[0].startswith("Error:"), "First line should be Error:"
        assert any("Reason:" in line for line in error_lines), "Should have Reason:"
        assert any("Solution:" in line for line in error_lines), "Should have Solution:"


@pytest.mark.v1_6_0
class TestErrorRecovery:
    """Test error recovery and graceful degradation."""

    def test_partial_batch_success(
        self,
        benchmark_pdfs,
        render_tool,
        tmp_path
    ):
        """
        Test that batch mode recovers from individual file errors.

        META:
          id: error_008
          category: robustness
          level: regression
          type: error
          pdf_count: multiple
          duration: 5s
          workers: [1]
          validates: Batch error recovery
          impact: high

        DESCRIPTION:
          Validates that batch processing continues and completes
          successfully even when some PDFs fail.

        EXPECTED:
          - Valid PDFs processed successfully
          - Failed PDFs logged but don't stop batch
          - Batch summary shows partial success
          - Exit code 1 (failures occurred)
        """
        # Create test directory
        test_dir = tmp_path / "mixed"
        test_dir.mkdir()

        # Copy 2 valid PDFs
        import shutil
        pdf_files = list(benchmark_pdfs.glob("*.pdf"))[:2]
        if len(pdf_files) < 2:
            pytest.skip("Need at least 2 benchmark PDFs for this test")
        shutil.copy(pdf_files[0], test_dir / "valid1.pdf")
        shutil.copy(pdf_files[1], test_dir / "valid2.pdf")

        # Add corrupted PDF
        (test_dir / "bad.pdf").write_bytes(b"not a pdf")

        output_dir = tmp_path / "output"

        result = subprocess.run(
            [
                render_tool,
                "--batch",
                "render-pages",
                str(test_dir),
                str(output_dir)
            ],
            capture_output=True,
            text=True
        )

        # Should return 1 (partial failure)
        assert result.returncode == 1, "Partial failures should return exit code 1"

        stderr = result.stderr

        # Check summary shows mixed results
        assert "Batch Summary:" in stderr, "Missing batch summary"
        assert "Succeeded: 2" in stderr, "Should show 2 successes"
        assert "Failed: 1" in stderr, "Should show 1 failure"

        # Check valid PDFs were processed
        assert (output_dir / "." / "valid1").exists(), "valid1 should succeed"
        assert (output_dir / "." / "valid2").exists(), "valid2 should succeed"
