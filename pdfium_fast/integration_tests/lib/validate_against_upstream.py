#!/usr/bin/env python3
"""
Validate Rust Tools Against C++ Reference Implementation

Compares output from C++ reference tools (calling PDFium C API directly)
against Rust tools (calling PDFium through Rust bindings).

Purpose: Prove Rust bindings are correct and produce identical output.

Test strategy:
1. C++ reference → Rust single-threaded: Must match byte-for-byte
2. Rust single-threaded → Rust multi-threaded: Already tested (test_002, test_005)
3. Therefore: Rust multi-threaded matches upstream (transitive)

Usage:
    python lib/validate_against_upstream.py [--pdf PDF_NAME]
"""

import subprocess
import tempfile
import argparse
import sys
from pathlib import Path
import hashlib
import json

class UpstreamValidator:
    def __init__(self):
        self.root = Path(__file__).parent.parent
        self.pdfium_root = self.root.parent

        # C++ reference tools
        self.cpp_text_tool = self.pdfium_root / 'out' / 'Optimized-Shared' / 'reference_text_extract'
        self.cpp_jsonl_tool = self.pdfium_root / 'out' / 'Optimized-Shared' / 'reference_jsonl_extract'

        # Rust tools
        self.rust_text_tool = self.pdfium_root / 'rust' / 'target' / 'release' / 'examples' / 'extract_text'
        self.rust_jsonl_tool = self.pdfium_root / 'rust' / 'target' / 'release' / 'examples' / 'extract_text_jsonl'

        # Verify all tools exist
        for tool in [self.cpp_text_tool, self.cpp_jsonl_tool, self.rust_text_tool, self.rust_jsonl_tool]:
            if not tool.exists():
                raise FileNotFoundError(f"Tool not found: {tool}")

        # PDF directory
        self.pdf_dir = self.root / 'pdfs' / 'benchmark'

    def compute_md5_file(self, file_path):
        """Compute MD5 of file"""
        md5 = hashlib.md5()
        with open(file_path, 'rb') as f:
            while chunk := f.read(8192):
                md5.update(chunk)
        return md5.hexdigest()

    def validate_text_extraction(self, pdf_path: Path) -> tuple[bool, str, dict]:
        """Compare C++ reference vs Rust text extraction"""

        with tempfile.NamedTemporaryFile(suffix='.txt', delete=False) as cpp_out:
            cpp_output_path = cpp_out.name

        with tempfile.NamedTemporaryFile(suffix='.txt', delete=False) as rust_out:
            rust_output_path = rust_out.name

        try:
            # Set library path for both C++ and Rust tools
            import os
            env = os.environ.copy()
            env['DYLD_LIBRARY_PATH'] = str(self.pdfium_root / 'out' / 'Optimized-Shared')

            # Run C++ reference tool
            cpp_result = subprocess.run(
                [str(self.cpp_text_tool), str(pdf_path), cpp_output_path],
                capture_output=True,
                text=True,
                timeout=120,
                env=env
            )

            if cpp_result.returncode != 0:
                return False, f"C++ tool failed: {cpp_result.stderr}", {}

            # Run Rust tool (single-threaded)
            rust_result = subprocess.run(
                [str(self.rust_text_tool), str(pdf_path), rust_output_path, '1'],
                capture_output=True,
                text=True,
                timeout=120,
                env=env
            )

            if rust_result.returncode != 0:
                return False, f"Rust tool failed: {rust_result.stderr}", {}

            # Compare byte-for-byte
            cpp_data = Path(cpp_output_path).read_bytes()
            rust_data = Path(rust_output_path).read_bytes()

            cpp_md5 = self.compute_md5_file(cpp_output_path)
            rust_md5 = self.compute_md5_file(rust_output_path)

            stats = {
                'cpp_bytes': len(cpp_data),
                'rust_bytes': len(rust_data),
                'cpp_md5': cpp_md5,
                'rust_md5': rust_md5,
            }

            if cpp_data == rust_data:
                return True, "MATCH (byte-for-byte identical)", stats
            else:
                # Find first difference
                for idx in range(min(len(cpp_data), len(rust_data))):
                    if cpp_data[idx] != rust_data[idx]:
                        return False, f"DIFFER at byte {idx}: C++={cpp_data[idx]:02x} Rust={rust_data[idx]:02x}", stats
                return False, f"DIFFER in length: C++={len(cpp_data)} Rust={len(rust_data)}", stats

        finally:
            Path(cpp_output_path).unlink(missing_ok=True)
            Path(rust_output_path).unlink(missing_ok=True)

    def validate_jsonl_extraction(self, pdf_path: Path) -> tuple[bool, str, dict]:
        """Compare C++ reference vs Rust JSONL extraction (page 0 only)"""

        with tempfile.NamedTemporaryFile(suffix='.jsonl', delete=False) as cpp_out:
            cpp_output_path = cpp_out.name

        with tempfile.NamedTemporaryFile(suffix='.jsonl', delete=False) as rust_out:
            rust_output_path = rust_out.name

        try:
            # Set library path for both C++ and Rust tools
            import os
            env = os.environ.copy()
            env['DYLD_LIBRARY_PATH'] = str(self.pdfium_root / 'out' / 'Optimized-Shared')

            # Run C++ reference tool
            cpp_result = subprocess.run(
                [str(self.cpp_jsonl_tool), str(pdf_path), cpp_output_path, '0'],
                capture_output=True,
                text=True,
                timeout=120,
                env=env
            )

            if cpp_result.returncode != 0:
                return False, f"C++ tool failed: {cpp_result.stderr}", {}

            # Run Rust tool
            rust_result = subprocess.run(
                [str(self.rust_jsonl_tool), str(pdf_path), rust_output_path, '0'],
                capture_output=True,
                text=True,
                timeout=120,
                env=env
            )

            if rust_result.returncode != 0:
                return False, f"Rust tool failed: {rust_result.stderr}", {}

            # Compare byte-for-byte
            cpp_data = Path(cpp_output_path).read_bytes()
            rust_data = Path(rust_output_path).read_bytes()

            cpp_lines = len(cpp_data.split(b'\n')) - 1  # Subtract empty last line
            rust_lines = len(rust_data.split(b'\n')) - 1

            cpp_md5 = self.compute_md5_file(cpp_output_path)
            rust_md5 = self.compute_md5_file(rust_output_path)

            stats = {
                'cpp_lines': cpp_lines,
                'rust_lines': rust_lines,
                'cpp_bytes': len(cpp_data),
                'rust_bytes': len(rust_data),
                'cpp_md5': cpp_md5,
                'rust_md5': rust_md5,
            }

            if cpp_data == rust_data:
                return True, "MATCH (byte-for-byte identical)", stats
            else:
                return False, f"DIFFER: C++={len(cpp_data)}B/{cpp_lines}L Rust={len(rust_data)}B/{rust_lines}L", stats

        finally:
            Path(cpp_output_path).unlink(missing_ok=True)
            Path(rust_output_path).unlink(missing_ok=True)

    def validate_pdf(self, pdf_name: str) -> dict:
        """Validate all extraction methods for a single PDF"""
        pdf_path = self.pdf_dir / pdf_name

        if not pdf_path.exists():
            return {
                'pdf': pdf_name,
                'text': (False, f"PDF not found: {pdf_path}", {}),
                'jsonl': (False, "Skipped (PDF not found)", {})
            }

        print(f"\n{'='*70}")
        print(f"Validating: {pdf_name}")
        print(f"{'='*70}")

        # Test text extraction
        print("  Text extraction (C++ vs Rust)...", end=' ', flush=True)
        text_match, text_msg, text_stats = self.validate_text_extraction(pdf_path)
        print(f"{'✅' if text_match else '❌'} {text_msg}")
        if text_stats:
            print(f"    C++:  {text_stats['cpp_bytes']:,} bytes, MD5: {text_stats['cpp_md5'][:12]}")
            print(f"    Rust: {text_stats['rust_bytes']:,} bytes, MD5: {text_stats['rust_md5'][:12]}")

        # Test JSONL extraction
        print("  JSONL extraction (C++ vs Rust)...", end=' ', flush=True)
        jsonl_match, jsonl_msg, jsonl_stats = self.validate_jsonl_extraction(pdf_path)
        print(f"{'✅' if jsonl_match else '❌'} {jsonl_msg}")
        if jsonl_stats:
            print(f"    C++:  {jsonl_stats['cpp_lines']} chars, {jsonl_stats['cpp_bytes']:,} bytes")
            print(f"    Rust: {jsonl_stats['rust_lines']} chars, {jsonl_stats['rust_bytes']:,} bytes")

        return {
            'pdf': pdf_name,
            'text': (text_match, text_msg, text_stats),
            'jsonl': (jsonl_match, jsonl_msg, jsonl_stats)
        }


def main():
    parser = argparse.ArgumentParser(description='Validate Rust tools against C++ reference')
    parser.add_argument('--pdf', type=str, help='Single PDF to test (default: test 10 representative PDFs)')
    args = parser.parse_args()

    validator = UpstreamValidator()

    # Test PDFs (10 representative samples)
    TEST_PDFS = [
        'arxiv_001.pdf',
        'arxiv_004.pdf',
        'arxiv_010.pdf',
        'cc_007_101p.pdf',
        'cc_015_101p.pdf',
        'edinet_2025-06-24_1318_E01920_Makita Corporation.pdf',
        'edinet_2025-06-25_1608_E02628_KIMURATAN CORPORATION.pdf',
        'web_005.pdf',
        'web_011.pdf',
        '0100pages_7FKQLKX273JBHXAAW5XDRT27JGMIZMCI.pdf',
    ]

    if args.pdf:
        pdfs_to_test = [args.pdf]
    else:
        pdfs_to_test = TEST_PDFS

    print(f"\n{'='*70}")
    print(f"UPSTREAM VALIDATION")
    print(f"{'='*70}")
    print(f"Testing {len(pdfs_to_test)} PDFs")
    print(f"C++ tools: Optimized-Shared build")
    print(f"Rust tools: Release build")
    print(f"{'='*70}")

    results = []
    for pdf_name in pdfs_to_test:
        result = validator.validate_pdf(pdf_name)
        results.append(result)

    # Summary
    print(f"\n{'='*70}")
    print(f"VALIDATION SUMMARY")
    print(f"{'='*70}")

    text_pass = sum(1 for r in results if r['text'][0])
    jsonl_pass = sum(1 for r in results if r['jsonl'][0])

    print(f"Text extraction:  {text_pass}/{len(results)} PDFs match")
    print(f"JSONL extraction: {jsonl_pass}/{len(results)} PDFs match")

    if text_pass == len(results) and jsonl_pass == len(results):
        print(f"\n✅ ALL TESTS PASSED")
        print(f"   Rust tools produce identical output to C++ reference")
        print(f"   Correctness validated against upstream PDFium")
        return 0
    else:
        print(f"\n❌ FAILURES DETECTED")
        print(f"\nFailed PDFs:")
        for r in results:
            if not r['text'][0]:
                print(f"  - {r['pdf']}: Text {r['text'][1]}")
            if not r['jsonl'][0]:
                print(f"  - {r['pdf']}: JSONL {r['jsonl'][1]}")
        return 1


if __name__ == '__main__':
    sys.exit(main())
