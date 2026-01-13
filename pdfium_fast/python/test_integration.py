#!/usr/bin/env python3
"""
Quick integration test for dash-pdf-extraction.

Tests basic functionality with real PDF files.
"""

import sys
import tempfile
import shutil
from pathlib import Path

# Import the package
try:
    from dash_pdf_extraction import PDFProcessor, PDFError
    print("✓ Package imported successfully")
except ImportError as e:
    print(f"✗ Failed to import package: {e}")
    sys.exit(1)


def test_basic_text_extraction():
    """Test basic text extraction."""
    print("\n[Test 1] Basic text extraction...")

    processor = PDFProcessor()
    # Use first available PDF
    pdf_dir = Path(__file__).parent.parent / "integration_tests/pdfs/benchmark"
    test_pdf = next(pdf_dir.glob("*.pdf"), None)

    if not test_pdf or not test_pdf.exists():
        print(f"  ⚠ Test PDF not found in: {pdf_dir}")
        return False

    try:
        text = processor.extract_text(test_pdf)
        assert len(text) > 0, "Extracted text is empty"
        print(f"  ✓ Extracted {len(text)} characters")
        print(f"  ✓ First 50 chars: {text[:50]}")
        return True
    except Exception as e:
        print(f"  ✗ Failed: {e}")
        return False


def test_text_with_workers():
    """Test text extraction with multiple workers."""
    print("\n[Test 2] Text extraction with 4 workers...")

    processor = PDFProcessor()
    # Use first available PDF
    pdf_dir = Path(__file__).parent.parent / "integration_tests/pdfs/benchmark"
    test_pdf = next(pdf_dir.glob("*.pdf"), None)

    if not test_pdf or not test_pdf.exists():
        print(f"  ⚠ Test PDF not found in: {pdf_dir}")
        return False

    try:
        text = processor.extract_text(test_pdf, workers=4)
        assert len(text) > 0, "Extracted text is empty"
        print(f"  ✓ Extracted {len(text)} characters with 4 workers")
        return True
    except Exception as e:
        print(f"  ✗ Failed: {e}")
        return False


def test_page_range():
    """Test extraction with page range."""
    print("\n[Test 3] Text extraction with page range...")

    processor = PDFProcessor()
    # Use first available PDF
    pdf_dir = Path(__file__).parent.parent / "integration_tests/pdfs/benchmark"
    test_pdf = next(pdf_dir.glob("*.pdf"), None)

    if not test_pdf or not test_pdf.exists():
        print(f"  ⚠ Test PDF not found in: {pdf_dir}")
        return False

    try:
        text = processor.extract_text(test_pdf, pages=(0, 2))
        assert len(text) > 0, "Extracted text is empty"
        print(f"  ✓ Extracted pages 0-2: {len(text)} characters")
        return True
    except Exception as e:
        print(f"  ✗ Failed: {e}")
        return False


def test_jsonl_extraction():
    """Test JSONL metadata extraction."""
    print("\n[Test 4] JSONL metadata extraction...")

    processor = PDFProcessor()
    # Use first available PDF
    pdf_dir = Path(__file__).parent.parent / "integration_tests/pdfs/benchmark"
    test_pdf = next(pdf_dir.glob("*.pdf"), None)

    if not test_pdf or not test_pdf.exists():
        print(f"  ⚠ Test PDF not found in: {pdf_dir}")
        return False

    try:
        metadata = processor.extract_jsonl(test_pdf, page=0)
        assert isinstance(metadata, dict), "Metadata is not a dict"
        print(f"  ✓ Extracted metadata: {len(metadata)} fields")
        if 'char_count' in metadata:
            print(f"  ✓ Character count: {metadata['char_count']}")
        return True
    except Exception as e:
        print(f"  ✗ Failed: {e}")
        return False


def test_image_rendering():
    """Test image rendering."""
    print("\n[Test 5] Image rendering to PNG...")

    processor = PDFProcessor()
    # Use first available PDF
    pdf_dir = Path(__file__).parent.parent / "integration_tests/pdfs/benchmark"
    test_pdf = next(pdf_dir.glob("*.pdf"), None)

    if not test_pdf or not test_pdf.exists():
        print(f"  ⚠ Test PDF not found in: {pdf_dir}")
        return False

    temp_dir = tempfile.mkdtemp()
    try:
        images = processor.render_pages(test_pdf, temp_dir)
        assert len(images) > 0, "No images rendered"
        assert all(img.exists() for img in images), "Some images missing"
        print(f"  ✓ Rendered {len(images)} pages to PNG")
        return True
    except Exception as e:
        print(f"  ✗ Failed: {e}")
        return False
    finally:
        shutil.rmtree(temp_dir, ignore_errors=True)


def test_jpeg_rendering():
    """Test JPEG rendering."""
    print("\n[Test 6] Image rendering to JPEG...")

    processor = PDFProcessor()
    # Use first available PDF
    pdf_dir = Path(__file__).parent.parent / "integration_tests/pdfs/benchmark"
    test_pdf = next(pdf_dir.glob("*.pdf"), None)

    if not test_pdf or not test_pdf.exists():
        print(f"  ⚠ Test PDF not found in: {pdf_dir}")
        return False

    temp_dir = tempfile.mkdtemp()
    try:
        images = processor.render_pages(
            test_pdf,
            temp_dir,
            format="jpg",
            jpeg_quality=90
        )
        assert len(images) > 0, "No images rendered"
        assert all(img.suffix == ".jpg" for img in images), "Wrong format"
        print(f"  ✓ Rendered {len(images)} pages to JPEG")
        return True
    except Exception as e:
        print(f"  ✗ Failed: {e}")
        return False
    finally:
        shutil.rmtree(temp_dir, ignore_errors=True)


def test_error_handling():
    """Test error handling."""
    print("\n[Test 7] Error handling...")

    processor = PDFProcessor()

    try:
        # This should raise PDFError
        processor.extract_text("nonexistent.pdf")
        print("  ✗ Should have raised PDFError")
        return False
    except PDFError as e:
        print(f"  ✓ Correctly raised PDFError: {e}")
        return True
    except Exception as e:
        print(f"  ✗ Wrong exception type: {e}")
        return False


def test_cli_detection():
    """Test CLI binary detection."""
    print("\n[Test 8] CLI binary detection...")

    try:
        processor = PDFProcessor()
        assert processor.cli_path.exists(), "CLI path doesn't exist"
        print(f"  ✓ Found CLI at: {processor.cli_path}")
        return True
    except Exception as e:
        print(f"  ✗ Failed: {e}")
        return False


def main():
    """Run all tests."""
    print("=" * 60)
    print("Dash PDF Extraction - Integration Tests")
    print("=" * 60)

    tests = [
        test_cli_detection,
        test_basic_text_extraction,
        test_text_with_workers,
        test_page_range,
        test_jsonl_extraction,
        test_image_rendering,
        test_jpeg_rendering,
        test_error_handling,
    ]

    results = []
    for test in tests:
        try:
            result = test()
            results.append(result)
        except Exception as e:
            print(f"  ✗ Test crashed: {e}")
            results.append(False)

    print("\n" + "=" * 60)
    print(f"Results: {sum(results)}/{len(results)} tests passed")
    print("=" * 60)

    if all(results):
        print("✓ All tests passed!")
        return 0
    else:
        print("✗ Some tests failed")
        return 1


if __name__ == "__main__":
    sys.exit(main())
