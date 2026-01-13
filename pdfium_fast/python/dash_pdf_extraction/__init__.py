"""
Dash PDF Extraction - Fast, multi-threaded PDF text extraction and rendering.

Python bindings for pdfium_cli, the optimized PDFium-based PDF processor.

Example:
    >>> from dash_pdf_extraction import PDFProcessor
    >>>
    >>> # Extract text
    >>> processor = PDFProcessor()
    >>> text = processor.extract_text("document.pdf")
    >>>
    >>> # Render pages to images
    >>> processor.render_pages("document.pdf", "output/", workers=4)
    >>>
    >>> # Extract with metadata
    >>> metadata = processor.extract_jsonl("document.pdf", page=0)
"""

from .core import PDFProcessor, PDFError
from .version import __version__

__all__ = ['PDFProcessor', 'PDFError', '__version__']
