#!/usr/bin/env python3
"""
DocItem JSON Comparator - Deterministic Quality Analysis Tool

Compares Rust DocItem JSON output with Python baseline to identify:
- Missing DocItem types (e.g., no Captions, no Footnotes)
- Incorrect label assignments
- Missing metadata fields
- Character count differences

This tool provides OBJECTIVE, REPEATABLE metrics (not subjective LLM scores).

Usage:
    ./scripts/compare_docitems.py \\
        --rust target/rust_output.json \\
        --python test-corpus/groundtruth/docling_v2/file.json \\
        --tolerance 0.02  # ±2% for floating point values

Created: N=1544 (2025-11-20)
Reference: N=1543 deterministic quality strategy
"""

import argparse
import json
import sys
from collections import Counter
from pathlib import Path
from typing import Any, Dict, List, Tuple


def load_json(path: Path) -> Dict[str, Any]:
    """Load JSON file."""
    try:
        with open(path, 'r', encoding='utf-8') as f:
            return json.load(f)
    except Exception as e:
        print(f"ERROR: Failed to load {path}: {e}", file=sys.stderr)
        sys.exit(1)


def extract_docitem_labels(data: Dict[str, Any]) -> Counter:
    """Extract DocItem labels (types) from JSON."""
    labels = []

    # Handle different JSON structures
    if 'texts' in data:
        # DoclingDocument format
        for item in data.get('texts', []):
            labels.append(item.get('label', 'Unknown'))
    elif 'pages' in data:
        # Page-based format
        for page in data.get('pages', []):
            for item in page.get('items', []):
                labels.append(item.get('type', item.get('label', 'Unknown')))

    return Counter(labels)


def extract_metadata_fields(data: Dict[str, Any]) -> set:
    """Extract metadata field names from JSON."""
    if 'metadata' in data:
        return set(data['metadata'].keys())
    return set()


def compare_docitem_labels(rust_labels: Counter, python_labels: Counter) -> Tuple[bool, List[str]]:
    """
    Compare DocItem label distributions.

    Returns:
        (passed, issues): Whether comparison passed and list of issue descriptions
    """
    issues = []
    passed = True

    # Check for missing labels in Rust output
    missing = python_labels - rust_labels
    if missing:
        passed = False
        for label in sorted(missing.keys()):
            count = python_labels[label]
            issues.append(f"  ❌ Missing label '{label}' (expected {count}, got 0)")

    # Check for extra labels in Rust output (may not be an error)
    extra = rust_labels - python_labels
    if extra:
        for label in sorted(extra.keys()):
            count = rust_labels[label]
            issues.append(f"  ℹ️  Extra label '{label}' (got {count}, expected 0) - may be ok")

    # Check for count differences
    for label in rust_labels.keys() & python_labels.keys():
        rust_count = rust_labels[label]
        python_count = python_labels[label]
        if rust_count != python_count:
            diff = rust_count - python_count
            percent = abs(diff) * 100.0 / python_count if python_count > 0 else 0
            if percent > 5:  # More than 5% difference is concerning
                passed = False
                issues.append(
                    f"  ❌ Label '{label}' count mismatch: "
                    f"expected {python_count}, got {rust_count} ({diff:+d}, {percent:.1f}%)"
                )
            else:
                issues.append(
                    f"  ⚠️  Label '{label}' count diff: "
                    f"expected {python_count}, got {rust_count} ({diff:+d}, {percent:.1f}%)"
                )

    return passed, issues


def compare_metadata_fields(rust_fields: set, python_fields: set) -> Tuple[bool, List[str]]:
    """
    Compare metadata fields.

    Returns:
        (passed, issues): Whether comparison passed and list of issue descriptions
    """
    issues = []
    passed = True

    # Check for missing fields in Rust output
    missing = python_fields - rust_fields
    if missing:
        passed = False
        issues.append(f"  ❌ Missing metadata fields ({len(missing)}): {', '.join(sorted(missing))}")

    # Check for extra fields in Rust output (may not be an error)
    extra = rust_fields - python_fields
    if extra:
        issues.append(f"  ℹ️  Extra metadata fields ({len(extra)}): {', '.join(sorted(extra))} - may be ok")

    # Report matching fields
    matching = rust_fields & python_fields
    if matching:
        issues.append(f"  ✅ Matching metadata fields ({len(matching)}): {', '.join(sorted(matching))}")

    return passed, issues


def count_characters_in_markdown(data: Dict[str, Any]) -> int:
    """
    Estimate character count from DocItem JSON (approximates markdown length).

    This is a rough estimate - actual markdown may differ slightly.
    """
    char_count = 0

    # Count text content
    if 'texts' in data:
        for item in data.get('texts', []):
            text = item.get('text', '')
            char_count += len(text)
            # Add label prefix (approximate)
            label = item.get('label', '')
            if label:
                char_count += len(label) + 3  # "## " for title, etc.

    # Count metadata
    if 'metadata' in data:
        for key, value in data['metadata'].items():
            char_count += len(str(key)) + len(str(value)) + 4  # "Key: Value\n"

    return char_count


def main():
    parser = argparse.ArgumentParser(
        description='Compare Rust DocItem JSON with Python baseline'
    )
    parser.add_argument(
        '--rust',
        type=Path,
        required=True,
        help='Path to Rust-generated JSON output'
    )
    parser.add_argument(
        '--python',
        type=Path,
        required=True,
        help='Path to Python baseline JSON (from test-corpus/groundtruth/docling_v2/)'
    )
    parser.add_argument(
        '--tolerance',
        type=float,
        default=0.02,
        help='Tolerance for floating point comparisons (default: 0.02 = ±2%%)'
    )
    parser.add_argument(
        '--verbose',
        action='store_true',
        help='Show detailed comparison output'
    )

    args = parser.parse_args()

    # Load JSON files
    print(f"Loading Rust output: {args.rust}")
    rust_data = load_json(args.rust)

    print(f"Loading Python baseline: {args.python}")
    python_data = load_json(args.python)

    print()

    # Extract DocItem labels
    rust_labels = extract_docitem_labels(rust_data)
    python_labels = extract_docitem_labels(python_data)

    # Extract metadata fields
    rust_metadata = extract_metadata_fields(rust_data)
    python_metadata = extract_metadata_fields(python_data)

    # Compare DocItem labels
    print("=" * 60)
    print("DOCITEM LABEL COMPARISON")
    print("=" * 60)
    print(f"Rust labels: {dict(rust_labels)}")
    print(f"Python labels: {dict(python_labels)}")
    print()

    labels_passed, label_issues = compare_docitem_labels(rust_labels, python_labels)
    if label_issues:
        for issue in label_issues:
            print(issue)
    else:
        print("  ✅ All DocItem labels match perfectly")
    print()

    # Compare metadata fields
    print("=" * 60)
    print("METADATA FIELD COMPARISON")
    print("=" * 60)
    print(f"Rust fields ({len(rust_metadata)}): {sorted(rust_metadata)}")
    print(f"Python fields ({len(python_metadata)}): {sorted(python_metadata)}")
    print()

    metadata_passed, metadata_issues = compare_metadata_fields(rust_metadata, python_metadata)
    if metadata_issues:
        for issue in metadata_issues:
            print(issue)
    else:
        print("  ✅ All metadata fields match perfectly")
    print()

    # Estimate character counts (rough approximation)
    print("=" * 60)
    print("CHARACTER COUNT ESTIMATE (FROM JSON)")
    print("=" * 60)
    rust_chars = count_characters_in_markdown(rust_data)
    python_chars = count_characters_in_markdown(python_data)
    diff = rust_chars - python_chars
    percent = abs(diff) * 100.0 / python_chars if python_chars > 0 else 0

    print(f"Rust (estimated): {rust_chars} chars")
    print(f"Python (estimated): {python_chars} chars")
    print(f"Difference: {diff:+d} chars ({percent:.1f}%)")

    char_passed = percent <= args.tolerance * 100
    if char_passed:
        print(f"  ✅ Within tolerance (±{args.tolerance * 100}%)")
    else:
        print(f"  ❌ Outside tolerance (±{args.tolerance * 100}%)")
    print()

    # Overall result
    print("=" * 60)
    print("OVERALL RESULT")
    print("=" * 60)

    overall_passed = labels_passed and metadata_passed and char_passed

    if overall_passed:
        print("  ✅ PASSED - Rust output matches Python baseline")
        print()
        print("  All deterministic metrics within acceptable tolerance:")
        print("    - DocItem label distribution matches")
        print("    - Metadata fields match")
        print("    - Character count within tolerance")
        sys.exit(0)
    else:
        print("  ❌ FAILED - Rust output differs from Python baseline")
        print()
        print("  Issues detected:")
        if not labels_passed:
            print("    - DocItem label mismatches")
        if not metadata_passed:
            print("    - Missing metadata fields")
        if not char_passed:
            print("    - Character count outside tolerance")
        print()
        print("  These are REAL quality issues (not LLM noise).")
        print("  Fix these deterministic gaps to improve quality.")
        sys.exit(1)


if __name__ == '__main__':
    main()
