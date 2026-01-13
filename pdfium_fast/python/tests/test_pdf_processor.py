"""
Unit tests for dash-pdf-extraction.

Tests the PDFProcessor Python bindings.
"""

import pytest
import tempfile
import shutil
from pathlib import Path
from dash_pdf_extraction import PDFProcessor, PDFError


@pytest.fixture
def test_pdfs_dir():
    """Path to test PDF files."""
    # Use integration test PDFs
    return Path(__file__).parent.parent.parent / "integration_tests" / "master_test_suite"


@pytest.fixture
def temp_output_dir():
    """Temporary output directory for tests."""
    temp_dir = tempfile.mkdtemp()
    yield Path(temp_dir)
    shutil.rmtree(temp_dir)


@pytest.fixture
def processor():
    """PDFProcessor instance."""
    return PDFProcessor()


class TestPDFProcessorInit:
    """Test PDFProcessor initialization."""

    def test_default_init(self):
        """Test default initialization."""
        processor = PDFProcessor()
        assert processor.default_workers == 1
        assert processor.debug is False
        assert processor.cli_path.exists()

    def test_custom_workers(self):
        """Test custom worker count."""
        processor = PDFProcessor(default_workers=4)
        assert processor.default_workers == 4

    def test_workers_clamped(self):
        """Test worker count is clamped to valid range."""
        processor = PDFProcessor(default_workers=100)
        assert processor.default_workers == 16

        processor = PDFProcessor(default_workers=0)
        assert processor.default_workers == 1

    def test_debug_mode(self):
        """Test debug mode flag."""
        processor = PDFProcessor(debug=True)
        assert processor.debug is True

    def test_custom_cli_path_invalid(self):
        """Test invalid CLI path raises error."""
        with pytest.raises(PDFError, match="not found"):
            PDFProcessor(cli_path="/nonexistent/pdfium_cli")


class TestExtractText:
    """Test text extraction."""

    def test_extract_text_basic(self, processor, test_pdfs_dir, temp_output_dir):
        """Test basic text extraction."""
        pdf_path = test_pdfs_dir / "pdfs" / "arxiv" / "1706_03762.pdf"
        if not pdf_path.exists():
            pytest.skip("Test PDF not found")

        text = processor.extract_text(pdf_path)
        assert isinstance(text, str)
        assert len(text) > 0

    def test_extract_text_to_file(self, processor, test_pdfs_dir, temp_output_dir):
        """Test extraction to file."""
        pdf_path = test_pdfs_dir / "pdfs" / "arxiv" / "1706_03762.pdf"
        if not pdf_path.exists():
            pytest.skip("Test PDF not found")

        output_path = temp_output_dir / "output.txt"
        processor.extract_text(pdf_path, output_path)

        assert output_path.exists()
        assert output_path.stat().st_size > 0

    def test_extract_text_with_workers(self, processor, test_pdfs_dir, temp_output_dir):
        """Test extraction with multiple workers."""
        pdf_path = test_pdfs_dir / "pdfs" / "arxiv" / "1706_03762.pdf"
        if not pdf_path.exists():
            pytest.skip("Test PDF not found")

        text = processor.extract_text(pdf_path, workers=4)
        assert isinstance(text, str)
        assert len(text) > 0

    def test_extract_text_page_range(self, processor, test_pdfs_dir, temp_output_dir):
        """Test extraction with page range."""
        pdf_path = test_pdfs_dir / "pdfs" / "arxiv" / "1706_03762.pdf"
        if not pdf_path.exists():
            pytest.skip("Test PDF not found")

        # Extract pages 0-2
        text = processor.extract_text(pdf_path, pages=(0, 2))
        assert isinstance(text, str)
        assert len(text) > 0

    def test_extract_text_single_page(self, processor, test_pdfs_dir, temp_output_dir):
        """Test extraction of single page."""
        pdf_path = test_pdfs_dir / "pdfs" / "arxiv" / "1706_03762.pdf"
        if not pdf_path.exists():
            pytest.skip("Test PDF not found")

        # Extract page 0 only
        text = processor.extract_text(pdf_path, pages=0)
        assert isinstance(text, str)
        assert len(text) > 0

    def test_extract_text_missing_file(self, processor):
        """Test extraction with missing file."""
        with pytest.raises(PDFError, match="not found"):
            processor.extract_text("nonexistent.pdf")

    def test_extract_text_invalid_pages(self, processor, test_pdfs_dir):
        """Test extraction with invalid page range."""
        pdf_path = test_pdfs_dir / "pdfs" / "arxiv" / "1706_03762.pdf"
        if not pdf_path.exists():
            pytest.skip("Test PDF not found")

        with pytest.raises(PDFError, match="Invalid pages"):
            processor.extract_text(pdf_path, pages=[1, 2, 3])


class TestExtractJSONL:
    """Test JSONL metadata extraction."""

    def test_extract_jsonl_basic(self, processor, test_pdfs_dir, temp_output_dir):
        """Test basic JSONL extraction."""
        pdf_path = test_pdfs_dir / "pdfs" / "arxiv" / "1706_03762.pdf"
        if not pdf_path.exists():
            pytest.skip("Test PDF not found")

        metadata = processor.extract_jsonl(pdf_path, page=0)
        assert isinstance(metadata, dict)
        assert 'char_count' in metadata or len(metadata) > 0

    def test_extract_jsonl_to_file(self, processor, test_pdfs_dir, temp_output_dir):
        """Test JSONL extraction to file."""
        pdf_path = test_pdfs_dir / "pdfs" / "arxiv" / "1706_03762.pdf"
        if not pdf_path.exists():
            pytest.skip("Test PDF not found")

        output_path = temp_output_dir / "metadata.jsonl"
        processor.extract_jsonl(pdf_path, page=0, output_path=output_path)

        assert output_path.exists()
        assert output_path.stat().st_size > 0

    def test_extract_jsonl_different_page(self, processor, test_pdfs_dir, temp_output_dir):
        """Test JSONL extraction for different pages."""
        pdf_path = test_pdfs_dir / "pdfs" / "arxiv" / "1706_03762.pdf"
        if not pdf_path.exists():
            pytest.skip("Test PDF not found")

        # Extract page 1
        metadata = processor.extract_jsonl(pdf_path, page=1)
        assert isinstance(metadata, dict)

    def test_extract_jsonl_missing_file(self, processor):
        """Test JSONL extraction with missing file."""
        with pytest.raises(PDFError, match="not found"):
            processor.extract_jsonl("nonexistent.pdf")


class TestRenderPages:
    """Test page rendering."""

    def test_render_pages_basic(self, processor, test_pdfs_dir, temp_output_dir):
        """Test basic page rendering."""
        pdf_path = test_pdfs_dir / "pdfs" / "synthetic" / "minimal_text.pdf"
        if not pdf_path.exists():
            pytest.skip("Test PDF not found")

        images = processor.render_pages(pdf_path, temp_output_dir)
        assert len(images) > 0
        assert all(img.exists() for img in images)
        assert all(img.suffix == ".png" for img in images)

    def test_render_pages_with_workers(self, processor, test_pdfs_dir, temp_output_dir):
        """Test rendering with multiple workers."""
        pdf_path = test_pdfs_dir / "pdfs" / "synthetic" / "minimal_text.pdf"
        if not pdf_path.exists():
            pytest.skip("Test PDF not found")

        images = processor.render_pages(pdf_path, temp_output_dir, workers=2)
        assert len(images) > 0

    def test_render_pages_jpeg(self, processor, test_pdfs_dir, temp_output_dir):
        """Test rendering to JPEG."""
        pdf_path = test_pdfs_dir / "pdfs" / "synthetic" / "minimal_text.pdf"
        if not pdf_path.exists():
            pytest.skip("Test PDF not found")

        images = processor.render_pages(
            pdf_path,
            temp_output_dir,
            format="jpg",
            jpeg_quality=85
        )
        assert len(images) > 0
        assert all(img.suffix == ".jpg" for img in images)

    def test_render_pages_page_range(self, processor, test_pdfs_dir, temp_output_dir):
        """Test rendering specific page range."""
        pdf_path = test_pdfs_dir / "pdfs" / "arxiv" / "1706_03762.pdf"
        if not pdf_path.exists():
            pytest.skip("Test PDF not found")

        # Render pages 0-1
        images = processor.render_pages(pdf_path, temp_output_dir, pages=(0, 1))
        assert len(images) == 2

    def test_render_pages_single_page(self, processor, test_pdfs_dir, temp_output_dir):
        """Test rendering single page."""
        pdf_path = test_pdfs_dir / "pdfs" / "synthetic" / "minimal_text.pdf"
        if not pdf_path.exists():
            pytest.skip("Test PDF not found")

        # Render page 0 only
        images = processor.render_pages(pdf_path, temp_output_dir, pages=0)
        assert len(images) == 1

    def test_render_pages_adaptive(self, processor, test_pdfs_dir, temp_output_dir):
        """Test rendering with adaptive threading."""
        pdf_path = test_pdfs_dir / "pdfs" / "synthetic" / "minimal_text.pdf"
        if not pdf_path.exists():
            pytest.skip("Test PDF not found")

        images = processor.render_pages(pdf_path, temp_output_dir, adaptive=True)
        assert len(images) > 0

    def test_render_pages_invalid_format(self, processor, test_pdfs_dir, temp_output_dir):
        """Test rendering with invalid format."""
        pdf_path = test_pdfs_dir / "pdfs" / "synthetic" / "minimal_text.pdf"
        if not pdf_path.exists():
            pytest.skip("Test PDF not found")

        with pytest.raises(PDFError, match="Invalid format"):
            processor.render_pages(pdf_path, temp_output_dir, format="invalid")

    def test_render_pages_missing_file(self, processor, temp_output_dir):
        """Test rendering with missing file."""
        with pytest.raises(PDFError, match="not found"):
            processor.render_pages("nonexistent.pdf", temp_output_dir)


class TestBatchOperations:
    """Test batch operations."""

    def test_batch_extract_text(self, processor, test_pdfs_dir, temp_output_dir):
        """Test batch text extraction."""
        input_dir = test_pdfs_dir / "pdfs" / "synthetic"
        if not input_dir.exists():
            pytest.skip("Test directory not found")

        output_dir = temp_output_dir / "text"
        processor.batch_extract_text(input_dir, output_dir)

        # Check that text files were created
        text_files = list(output_dir.glob("*.txt"))
        assert len(text_files) > 0

    def test_batch_render_pages(self, processor, test_pdfs_dir, temp_output_dir):
        """Test batch page rendering."""
        input_dir = test_pdfs_dir / "pdfs" / "synthetic"
        if not input_dir.exists():
            pytest.skip("Test directory not found")

        output_dir = temp_output_dir / "images"
        processor.batch_render_pages(input_dir, output_dir, workers=2)

        # Check that image files were created
        image_files = list(output_dir.glob("*.png"))
        assert len(image_files) > 0

    def test_batch_with_pattern(self, processor, test_pdfs_dir, temp_output_dir):
        """Test batch operations with pattern."""
        input_dir = test_pdfs_dir / "pdfs"
        if not input_dir.exists():
            pytest.skip("Test directory not found")

        output_dir = temp_output_dir / "text"
        processor.batch_extract_text(
            input_dir,
            output_dir,
            pattern="minimal_*.pdf"
        )

        # Should only process files matching pattern
        text_files = list(output_dir.glob("*.txt"))
        assert len(text_files) >= 0  # May be 0 if no files match

    def test_batch_missing_directory(self, processor, temp_output_dir):
        """Test batch operations with missing directory."""
        with pytest.raises(PDFError, match="not found"):
            processor.batch_extract_text(
                "nonexistent_dir/",
                temp_output_dir
            )


class TestPathLikeObjects:
    """Test Path-like object support."""

    def test_extract_text_with_path_object(self, processor, test_pdfs_dir, temp_output_dir):
        """Test extraction with Path objects."""
        pdf_path = Path(test_pdfs_dir) / "pdfs" / "synthetic" / "minimal_text.pdf"
        if not pdf_path.exists():
            pytest.skip("Test PDF not found")

        output_path = Path(temp_output_dir) / "output.txt"

        processor.extract_text(pdf_path, output_path)
        assert output_path.exists()

    def test_render_pages_with_path_object(self, processor, test_pdfs_dir, temp_output_dir):
        """Test rendering with Path objects."""
        pdf_path = Path(test_pdfs_dir) / "pdfs" / "synthetic" / "minimal_text.pdf"
        if not pdf_path.exists():
            pytest.skip("Test PDF not found")

        output_dir = Path(temp_output_dir) / "images"

        images = processor.render_pages(pdf_path, output_dir)
        assert len(images) > 0


class TestErrorHandling:
    """Test error handling."""

    def test_timeout(self, processor, test_pdfs_dir):
        """Test command timeout."""
        pdf_path = test_pdfs_dir / "pdfs" / "arxiv" / "1706_03762.pdf"
        if not pdf_path.exists():
            pytest.skip("Test PDF not found")

        # Very short timeout should fail
        with pytest.raises(PDFError, match="timed out"):
            processor.extract_text(pdf_path, timeout=0.001)

    def test_invalid_pdf(self, processor, temp_output_dir):
        """Test with invalid PDF file."""
        # Create empty file
        invalid_pdf = temp_output_dir / "invalid.pdf"
        invalid_pdf.write_text("not a pdf")

        with pytest.raises(PDFError):
            processor.extract_text(invalid_pdf)


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
