#!/usr/bin/env python3
"""
Basic usage examples for dash-pdf-extraction.

This script demonstrates the core functionality of the PDFProcessor class.
"""

from dash_pdf_extraction import PDFProcessor, PDFError
from pathlib import Path


def example_extract_text():
    """Extract text from a PDF file."""
    print("=" * 60)
    print("Example 1: Extract Text")
    print("=" * 60)

    processor = PDFProcessor()

    # Extract all text
    try:
        text = processor.extract_text("sample.pdf")
        print(f"Extracted {len(text)} characters")
        print(f"First 100 characters: {text[:100]}...")
    except PDFError as e:
        print(f"Error: {e}")

    print()


def example_extract_text_with_workers():
    """Extract text using multiple workers for large PDFs."""
    print("=" * 60)
    print("Example 2: Extract Text with Multiple Workers")
    print("=" * 60)

    processor = PDFProcessor()

    try:
        # Use 4 workers for faster processing
        text = processor.extract_text("large_document.pdf", workers=4)
        print(f"Extracted {len(text)} characters using 4 workers")
    except PDFError as e:
        print(f"Error: {e}")

    print()


def example_extract_page_range():
    """Extract text from specific page range."""
    print("=" * 60)
    print("Example 3: Extract Specific Page Range")
    print("=" * 60)

    processor = PDFProcessor()

    try:
        # Extract pages 1-10
        text = processor.extract_text("document.pdf", pages=(1, 10))
        print(f"Extracted pages 1-10: {len(text)} characters")

        # Extract single page (page 5)
        text_page5 = processor.extract_text("document.pdf", pages=5)
        print(f"Extracted page 5: {len(text_page5)} characters")
    except PDFError as e:
        print(f"Error: {e}")

    print()


def example_extract_with_metadata():
    """Extract text with rich metadata (JSONL format)."""
    print("=" * 60)
    print("Example 4: Extract with Metadata (JSONL)")
    print("=" * 60)

    processor = PDFProcessor()

    try:
        # Extract metadata for first page
        metadata = processor.extract_jsonl("document.pdf", page=0)

        print(f"Page metadata:")
        print(f"  - Character count: {metadata.get('char_count', 'N/A')}")
        print(f"  - Page dimensions: {metadata.get('width', 'N/A')}x{metadata.get('height', 'N/A')}")

        # Access character-level data
        if 'chars' in metadata:
            print(f"  - Characters with positions: {len(metadata['chars'])}")

    except PDFError as e:
        print(f"Error: {e}")

    print()


def example_render_pages():
    """Render PDF pages to images."""
    print("=" * 60)
    print("Example 5: Render Pages to Images")
    print("=" * 60)

    processor = PDFProcessor()

    try:
        # Render all pages to PNG
        images = processor.render_pages("document.pdf", "output/images/")
        print(f"Rendered {len(images)} pages to PNG")

        # Show first few image paths
        for i, img in enumerate(images[:3]):
            print(f"  - {img.name}")

    except PDFError as e:
        print(f"Error: {e}")

    print()


def example_render_pages_jpeg():
    """Render pages to JPEG with quality control."""
    print("=" * 60)
    print("Example 6: Render to JPEG with Quality Control")
    print("=" * 60)

    processor = PDFProcessor()

    try:
        # Render to high-quality JPEG
        images = processor.render_pages(
            "document.pdf",
            "output/jpeg/",
            format="jpg",
            jpeg_quality=95,
            workers=4
        )
        print(f"Rendered {len(images)} pages to JPEG (quality=95)")

    except PDFError as e:
        print(f"Error: {e}")

    print()


def example_render_with_adaptive():
    """Render with adaptive threading for optimal performance."""
    print("=" * 60)
    print("Example 7: Render with Adaptive Threading")
    print("=" * 60)

    processor = PDFProcessor()

    try:
        # Adaptive mode auto-selects thread count based on page count
        images = processor.render_pages(
            "large_document.pdf",
            "output/adaptive/",
            workers=4,
            adaptive=True
        )
        print(f"Rendered {len(images)} pages with adaptive threading")

    except PDFError as e:
        print(f"Error: {e}")

    print()


def example_batch_extract():
    """Batch extract text from multiple PDFs."""
    print("=" * 60)
    print("Example 8: Batch Text Extraction")
    print("=" * 60)

    processor = PDFProcessor()

    try:
        # Extract all PDFs in directory
        processor.batch_extract_text("pdfs/", "output/text/")
        print("Batch extraction complete")

        # Extract with pattern matching
        processor.batch_extract_text(
            "documents/",
            "output/reports/",
            pattern="report_*.pdf",
            workers=4
        )
        print("Batch extraction with pattern complete")

    except PDFError as e:
        print(f"Error: {e}")

    print()


def example_batch_render():
    """Batch render pages from multiple PDFs."""
    print("=" * 60)
    print("Example 9: Batch Page Rendering")
    print("=" * 60)

    processor = PDFProcessor()

    try:
        # Render all PDFs in directory
        processor.batch_render_pages(
            "pdfs/",
            "output/images/",
            workers=4,
            format="png"
        )
        print("Batch rendering complete")

        # Recursive batch rendering with JPEG output
        processor.batch_render_pages(
            "archive/",
            "output/archive_images/",
            pattern="*.pdf",
            recursive=True,
            format="jpg",
            jpeg_quality=90,
            adaptive=True
        )
        print("Recursive batch rendering complete")

    except PDFError as e:
        print(f"Error: {e}")

    print()


def example_error_handling():
    """Demonstrate error handling."""
    print("=" * 60)
    print("Example 10: Error Handling")
    print("=" * 60)

    processor = PDFProcessor()

    # Handle missing file
    try:
        text = processor.extract_text("nonexistent.pdf")
    except PDFError as e:
        print(f"Caught expected error: {e}")

    # Handle invalid page range
    try:
        text = processor.extract_text("document.pdf", pages=(100, 200))
    except PDFError as e:
        print(f"Caught expected error: {e}")

    # Handle invalid format
    try:
        images = processor.render_pages(
            "document.pdf",
            "output/",
            format="invalid"
        )
    except PDFError as e:
        print(f"Caught expected error: {e}")

    print()


def example_custom_binary_path():
    """Use custom pdfium_cli binary path."""
    print("=" * 60)
    print("Example 11: Custom Binary Path")
    print("=" * 60)

    try:
        # Specify custom binary location
        processor = PDFProcessor(cli_path="/usr/local/bin/pdfium_cli")
        text = processor.extract_text("document.pdf")
        print(f"Extracted using custom binary: {len(text)} characters")
    except PDFError as e:
        print(f"Error: {e}")

    print()


def example_debug_mode():
    """Enable debug mode for troubleshooting."""
    print("=" * 60)
    print("Example 12: Debug Mode")
    print("=" * 60)

    # Enable debug mode
    processor = PDFProcessor(debug=True)

    try:
        text = processor.extract_text("document.pdf")
        print("Debug mode enabled - check output for detailed tracing")
    except PDFError as e:
        print(f"Error: {e}")

    print()


def main():
    """Run all examples."""
    print("\n")
    print("=" * 60)
    print("Dash PDF Extraction - Usage Examples")
    print("=" * 60)
    print("\n")

    # Note: Most examples will fail without actual PDF files
    # This is just to demonstrate the API

    examples = [
        example_extract_text,
        example_extract_text_with_workers,
        example_extract_page_range,
        example_extract_with_metadata,
        example_render_pages,
        example_render_pages_jpeg,
        example_render_with_adaptive,
        example_batch_extract,
        example_batch_render,
        example_error_handling,
        example_custom_binary_path,
        example_debug_mode,
    ]

    for example in examples:
        try:
            example()
        except Exception as e:
            print(f"Example failed: {e}\n")

    print("=" * 60)
    print("Examples complete!")
    print("=" * 60)


if __name__ == "__main__":
    main()
