#!/usr/bin/env python3
"""
Compare current baselines against true upstream baselines.

This script compares MD5 hashes from:
1. Current baselines (integration_tests/baselines/upstream/images_ppm/*.json)
2. True upstream baselines (integration_tests/baselines/upstream/images_ppm_upstream_true/*.json)

Reports differences to identify rendering divergence from upstream.
"""

import json
from pathlib import Path
from collections import defaultdict

def load_baseline(json_path: Path) -> dict:
    """Load baseline JSON file."""
    with open(json_path) as f:
        return json.load(f)

def compare_baselines():
    """Compare all baselines and report differences."""

    script_dir = Path(__file__).parent
    current_dir = script_dir / "baselines" / "upstream" / "images_ppm"
    upstream_dir = script_dir / "baselines" / "upstream" / "images_ppm_upstream_true"

    if not upstream_dir.exists():
        print(f"Error: Upstream baselines not found at {upstream_dir}")
        print("Run generate_upstream_baselines.py first")
        return

    # Get all PDFs that have both baselines
    current_files = {f.stem: f for f in current_dir.glob("*.json")}
    upstream_files = {f.stem: f for f in upstream_dir.glob("*.json")}

    common_pdfs = set(current_files.keys()) & set(upstream_files.keys())

    if not common_pdfs:
        print("No common PDFs found between current and upstream baselines")
        return

    print(f"Comparing {len(common_pdfs)} PDFs")
    print(f"Current baselines: {current_dir}")
    print(f"Upstream baselines: {upstream_dir}")
    print()

    # Track differences
    identical_pdfs = []
    different_pdfs = []
    differences_by_pdf = {}

    for pdf_stem in sorted(common_pdfs):
        current_baseline = load_baseline(current_files[pdf_stem])
        upstream_baseline = load_baseline(upstream_files[pdf_stem])

        current_pages = current_baseline.get("pages", {})
        upstream_pages = upstream_baseline.get("pages", {})

        # Compare page counts
        if set(current_pages.keys()) != set(upstream_pages.keys()):
            different_pdfs.append(pdf_stem)
            differences_by_pdf[pdf_stem] = {
                "type": "page_count_mismatch",
                "current_pages": len(current_pages),
                "upstream_pages": len(upstream_pages)
            }
            continue

        # Compare MD5s for each page
        diff_pages = []
        for page_num in current_pages.keys():
            current_md5 = current_pages[page_num]
            upstream_md5 = upstream_pages.get(page_num)

            if current_md5 != upstream_md5:
                diff_pages.append({
                    "page": int(page_num),
                    "current_md5": current_md5,
                    "upstream_md5": upstream_md5
                })

        if diff_pages:
            different_pdfs.append(pdf_stem)
            differences_by_pdf[pdf_stem] = {
                "type": "md5_mismatch",
                "total_pages": len(current_pages),
                "different_pages": len(diff_pages),
                "pages": diff_pages
            }
        else:
            identical_pdfs.append(pdf_stem)

    # Print summary
    print("=" * 80)
    print("COMPARISON SUMMARY")
    print("=" * 80)
    print(f"Total PDFs compared: {len(common_pdfs)}")
    print(f"Identical (100% match): {len(identical_pdfs)} ({100*len(identical_pdfs)/len(common_pdfs):.1f}%)")
    print(f"Different: {len(different_pdfs)} ({100*len(different_pdfs)/len(common_pdfs):.1f}%)")
    print()

    if identical_pdfs:
        print(f"✓ {len(identical_pdfs)} PDFs match upstream exactly")
        print()

    if different_pdfs:
        print(f"✗ {len(different_pdfs)} PDFs differ from upstream:")
        print()

        # Group by difference type
        md5_mismatches = [p for p in different_pdfs if differences_by_pdf[p]["type"] == "md5_mismatch"]
        page_count_mismatches = [p for p in different_pdfs if differences_by_pdf[p]["type"] == "page_count_mismatch"]

        if md5_mismatches:
            print(f"  MD5 Mismatches: {len(md5_mismatches)} PDFs")
            print()

            # Count total affected pages
            total_pages = 0
            affected_pages = 0
            for pdf_stem in md5_mismatches:
                diff = differences_by_pdf[pdf_stem]
                total_pages += diff["total_pages"]
                affected_pages += diff["different_pages"]

            print(f"  Total pages: {total_pages}")
            print(f"  Affected pages: {affected_pages} ({100*affected_pages/total_pages:.1f}%)")
            print()

            # Show top 10 most affected PDFs
            top_affected = sorted(md5_mismatches,
                                 key=lambda p: differences_by_pdf[p]["different_pages"],
                                 reverse=True)[:10]

            print(f"  Top 10 most affected PDFs:")
            for i, pdf_stem in enumerate(top_affected, 1):
                diff = differences_by_pdf[pdf_stem]
                pct = 100 * diff["different_pages"] / diff["total_pages"]
                print(f"    {i:2d}. {pdf_stem}: {diff['different_pages']}/{diff['total_pages']} pages ({pct:.1f}%)")
            print()

        if page_count_mismatches:
            print(f"  Page Count Mismatches: {len(page_count_mismatches)} PDFs")
            for pdf_stem in page_count_mismatches[:10]:
                diff = differences_by_pdf[pdf_stem]
                print(f"    - {pdf_stem}: current={diff['current_pages']}, upstream={diff['upstream_pages']}")
            if len(page_count_mismatches) > 10:
                print(f"    ... and {len(page_count_mismatches)-10} more")
            print()

    # Save detailed report
    report_path = Path("/tmp/baseline_comparison_report.json")
    report = {
        "total_pdfs": len(common_pdfs),
        "identical": len(identical_pdfs),
        "different": len(different_pdfs),
        "identical_pdfs": identical_pdfs,
        "different_pdfs": different_pdfs,
        "differences": differences_by_pdf
    }

    with open(report_path, 'w') as f:
        json.dump(report, f, indent=2)

    print(f"Detailed report saved to: {report_path}")

if __name__ == "__main__":
    compare_baselines()
