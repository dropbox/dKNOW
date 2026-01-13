#!/usr/bin/env python3
"""
Generate Python baseline outputs for formats without baselines.

This script uses Python docling v2.58.0 to generate ground truth outputs
that can be used for reliable LLM quality comparison.
"""

import sys
import os
from pathlib import Path

# Add Python docling to path
sys.path.insert(0, str(Path.home() / "docling"))

try:
    from docling.document_converter import DocumentConverter
except ImportError:
    print("ERROR: Could not import docling. Ensure ~/docling exists and is v2.58.0")
    sys.exit(1)

def generate_baseline(input_path: Path, output_dir: Path):
    """Generate baseline markdown output for a single file."""

    print(f"Processing: {input_path}")

    try:
        converter = DocumentConverter()
        result = converter.convert(str(input_path))
        markdown = result.document.export_to_markdown()

        # Create output path
        output_path = output_dir / f"{input_path.stem}.md"
        output_path.parent.mkdir(parents=True, exist_ok=True)

        # Write markdown
        with open(output_path, 'w') as f:
            f.write(markdown)

        print(f"  ✅ Generated: {output_path}")
        return True

    except Exception as e:
        print(f"  ❌ Failed: {e}")
        return False

def main():
    """Generate baselines for high-priority formats."""

    base_dir = Path(__file__).parent.parent
    test_corpus = base_dir / "test-corpus"
    baseline_dir = base_dir / "test-corpus" / "groundtruth" / "docling_v2_extended"

    # High-priority formats to process
    formats = {
        "epub": test_corpus / "ebooks" / "epub" / "simple.epub",
        "odt": test_corpus / "opendocument" / "odt" / "simple_text.odt",
        "odp": test_corpus / "opendocument" / "odp" / "simple_presentation.odp",
        "dxf": test_corpus / "cad" / "dxf" / "simple_drawing.dxf",
    }

    print("=" * 60)
    print("Python Docling Baseline Generator (v2.58.0)")
    print("=" * 60)
    print()

    success_count = 0
    total_count = len(formats)

    for format_name, input_path in formats.items():
        if not input_path.exists():
            print(f"⚠️  Skipping {format_name}: File not found: {input_path}")
            continue

        format_output_dir = baseline_dir / format_name
        if generate_baseline(input_path, format_output_dir):
            success_count += 1
        print()

    print("=" * 60)
    print(f"Results: {success_count}/{total_count} successful")
    print("=" * 60)

    if success_count < total_count:
        sys.exit(1)

if __name__ == "__main__":
    main()
