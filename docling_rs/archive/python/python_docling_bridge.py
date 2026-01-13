#!/usr/bin/env python3
"""
Python bridge script for docling_rs

This script runs Python docling and outputs DoclingDocument JSON to stdout.
Rust code calls this script to get structured document data.

Usage:
    python scripts/python_docling_bridge.py <input_path>
"""

import json
import sys
from pathlib import Path

try:
    # BUGFIX: docling_core doesn't include 'text/vtt' in allowed MIME types
    # but docling uses it for WebVTT files. Monkey-patch to allow it.
    from docling_core.types.doc import DocumentOrigin
    if 'text/vtt' not in DocumentOrigin._extra_mimetypes:
        DocumentOrigin._extra_mimetypes.append('text/vtt')

    from docling.document_converter import DocumentConverter
except ImportError:
    print("ERROR: docling not installed. Run: pip install docling", file=sys.stderr)
    sys.exit(1)


def convert_to_json(input_path: str, enable_ocr: bool = False) -> dict:
    """Convert a document to DoclingDocument JSON format.

    Args:
        input_path: Path to the document to convert
        enable_ocr: If True, enable OCR mode (do_ocr=True). If False, text-only mode (do_ocr=False).
    """
    from docling.datamodel.base_models import InputFormat
    from docling.datamodel.pipeline_options import PdfPipelineOptions
    from docling.document_converter import PdfFormatOption

    # Configure pipeline options to match test requirements
    # - do_table_structure=True: Enable table structure detection (default, matches upstream)
    # - do_ocr=enable_ocr: Control OCR based on parameter
    pipeline_options = PdfPipelineOptions(do_table_structure=True, do_ocr=enable_ocr)

    converter = DocumentConverter(
        format_options={
            InputFormat.PDF: PdfFormatOption(pipeline_options=pipeline_options)
        }
    )
    result = converter.convert(input_path)

    # Export to dict (DoclingDocument structure)
    # Use mode='json' to serialize pydantic types like AnyUrl to strings
    doc_dict = result.document.model_dump(
        mode='json',
        by_alias=True,
        exclude_none=False,
        exclude_unset=False,
    )

    return doc_dict


def main():
    if len(sys.argv) < 2:
        print("Usage: python_docling_bridge.py <input_path> [--ocr]", file=sys.stderr)
        sys.exit(1)

    input_path = sys.argv[1]
    enable_ocr = '--ocr' in sys.argv[2:]

    if not Path(input_path).exists():
        print(f"ERROR: File not found: {input_path}", file=sys.stderr)
        sys.exit(1)

    try:
        doc_dict = convert_to_json(input_path, enable_ocr=enable_ocr)
        # Output JSON to stdout
        print(json.dumps(doc_dict, indent=None))
    except Exception as e:
        print(f"ERROR: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
