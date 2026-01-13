#!/usr/bin/env python3
"""
Generate Error Manifest for Unloadable PDFs

Creates manifest.json for PDFs that cannot be loaded by FPDF_LoadDocument.
These PDFs are intentionally malformed and test error handling.
"""

import json
from pathlib import Path
import sys

def create_error_manifest(pdf_name: str, output_dir: Path, reason: str = "FPDF_LOAD_FAILED") -> None:
    """
    Create error manifest for an unloadable PDF.

    Args:
        pdf_name: Name of the PDF file
        output_dir: Output directory for expected outputs
        reason: Reason for load failure
    """
    manifest = {
        "pdf": pdf_name,
        "load_result": reason,
        "expected_behavior": "graceful_failure",
        "error_code": 1,
        "text": {
            "available": False,
            "reason": "PDF cannot be loaded"
        },
        "images": {
            "available": False,
            "reason": "PDF cannot be loaded"
        },
        "jsonl": {
            "available": False,
            "reason": "PDF cannot be loaded"
        }
    }

    # Create output directory if needed
    output_dir.mkdir(parents=True, exist_ok=True)

    # Write manifest
    manifest_path = output_dir / "manifest.json"
    with open(manifest_path, 'w') as f:
        json.dump(manifest, f, indent=2)

    print(f"Created error manifest: {manifest_path}")

def main():
    # Known unloadable PDFs (from commit #47 and additional discovery in #53)
    unloadable_pdfs = [
        # Original 4 from commit #47
        ("bug_298.pdf", "edge_cases/bug_298"),
        ("bug_1124998.pdf", "edge_cases/bug_1124998"),
        ("bug_1324189.pdf", "edge_cases/bug_1324189"),
        ("bug_1324503.pdf", "edge_cases/bug_1324503"),
        # Additional 16 discovered in commit #53
        ("bug_325_a.pdf", "edge_cases/bug_325_a"),
        ("bug_325_b.pdf", "edge_cases/bug_325_b"),
        ("bug_343.pdf", "edge_cases/bug_343"),
        ("bug_344.pdf", "edge_cases/bug_344"),
        ("bug_355.pdf", "edge_cases/bug_355"),
        ("bug_360.pdf", "edge_cases/bug_360"),
        ("bug_424613308.pdf", "edge_cases/bug_424613308"),
        ("bug_451830.pdf", "edge_cases/bug_451830"),
        ("bug_454695.pdf", "edge_cases/bug_454695"),
        ("bug_644.pdf", "edge_cases/bug_644"),
        ("encrypted.pdf", "edge_cases/encrypted"),
        ("encrypted_hello_world_r2.pdf", "edge_cases/encrypted_hello_world_r2"),
        ("encrypted_hello_world_r2_bad_okey.pdf", "edge_cases/encrypted_hello_world_r2_bad_okey"),
        ("encrypted_hello_world_r3.pdf", "edge_cases/encrypted_hello_world_r3"),
        ("encrypted_hello_world_r3_bad_okey.pdf", "edge_cases/encrypted_hello_world_r3_bad_okey"),
        ("encrypted_hello_world_r5.pdf", "edge_cases/encrypted_hello_world_r5"),
        ("encrypted_hello_world_r6.pdf", "edge_cases/encrypted_hello_world_r6"),
        ("parser_rebuildxref_error_notrailer.pdf", "edge_cases/parser_rebuildxref_error_notrailer"),
        ("trailer_as_hexstring.pdf", "edge_cases/trailer_as_hexstring"),
        ("trailer_unterminated.pdf", "edge_cases/trailer_unterminated"),
    ]

    # Base directory
    integration_tests = Path(__file__).parent.parent
    expected_outputs = integration_tests / "master_test_suite" / "expected_outputs"

    print("Generating error manifests for unloadable PDFs...")
    print("=" * 80)

    for pdf_name, rel_path in unloadable_pdfs:
        output_dir = expected_outputs / rel_path
        create_error_manifest(pdf_name, output_dir)

    print("=" * 80)
    print(f"Generated {len(unloadable_pdfs)} error manifests")

if __name__ == "__main__":
    main()
