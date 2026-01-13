#!/bin/bash
# Scan Format Quality - Deterministic Analysis Across All Formats
#
# Runs compare_docitems.py on all available test outputs to identify
# formats with real quality gaps (not LLM noise).
#
# Usage: ./scripts/scan_format_quality.sh [--format FORMAT]
#
# Created: N=1544 (2025-11-20)
# Reference: N=1543 deterministic quality strategy

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "=========================================="
echo "Format Quality Scanner (Deterministic)"
echo "=========================================="
echo

# Check if specific format requested
TARGET_FORMAT=""
if [ $# -ge 2 ] && [ "$1" = "--format" ]; then
    TARGET_FORMAT="$2"
    echo "Scanning format: $TARGET_FORMAT"
else
    echo "Scanning all available formats"
fi
echo

# Track results
TOTAL=0
PASSED=0
FAILED=0
MISSING=0

# Function to compare a single format
compare_format() {
    local format="$1"
    local basename="$2"  # basename without any extension (e.g., "example_01")
    local extension="$3"  # format extension (e.g., "html", "csv")

    RUST_JSON="test-results/outputs/$format/$basename.json"
    PYTHON_JSON="test-corpus/groundtruth/docling_v2/$basename.$extension.json"

    # Check if files exist
    if [ ! -f "$RUST_JSON" ]; then
        echo -e "${YELLOW}⚠️  $format/$basename: Rust output missing${NC}"
        ((MISSING++)) || true
        return
    fi

    if [ ! -f "$PYTHON_JSON" ]; then
        echo -e "${YELLOW}⚠️  $format/$basename: Python baseline missing${NC}"
        ((MISSING++)) || true
        return
    fi

    ((TOTAL++)) || true

    # Run comparator (capture exit code)
    # Note: Don't exit on error - we want to scan all formats
    set +e
    if python3 scripts/compare_docitems.py \
        --rust "$RUST_JSON" \
        --python "$PYTHON_JSON" >/dev/null 2>&1; then
        echo -e "${GREEN}✅ $format/$basename: PASSED${NC}"
        ((PASSED++)) || true
    else
        echo -e "${RED}❌ $format/$basename: FAILED${NC}"
        ((FAILED++)) || true

        # Show details
        echo "   Details:"
        python3 scripts/compare_docitems.py \
            --rust "$RUST_JSON" \
            --python "$PYTHON_JSON" 2>&1 | grep -E "Missing|Outside|expected" | sed 's/^/   /'
        echo
    fi
    set -e
}

# Scan HTML files
if [ -z "$TARGET_FORMAT" ] || [ "$TARGET_FORMAT" = "html" ]; then
    echo "--- HTML Format ---"
    for file in test-results/outputs/html/*.json; do
        [ -f "$file" ] || continue
        basename=$(basename "$file" .json)
        compare_format "html" "$basename" "html"
    done
    echo
fi

# Scan CSV files
if [ -z "$TARGET_FORMAT" ] || [ "$TARGET_FORMAT" = "csv" ]; then
    echo "--- CSV Format ---"
    for file in test-results/outputs/csv/*.json; do
        [ -f "$file" ] || continue
        basename=$(basename "$file" .json)
        compare_format "csv" "$basename" "csv"
    done
    echo
fi

# Scan DOCX files
if [ -z "$TARGET_FORMAT" ] || [ "$TARGET_FORMAT" = "docx" ]; then
    echo "--- DOCX Format ---"
    for file in test-results/outputs/docx/*.json; do
        [ -f "$file" ] || continue
        basename=$(basename "$file" .json)
        compare_format "docx" "$basename" "docx"
    done
    echo
fi

# Scan XLSX files
if [ -z "$TARGET_FORMAT" ] || [ "$TARGET_FORMAT" = "xlsx" ]; then
    echo "--- XLSX Format ---"
    for file in test-results/outputs/xlsx/*.json; do
        [ -f "$file" ] || continue
        basename=$(basename "$file" .json)
        compare_format "xlsx" "$basename" "xlsx"
    done
    echo
fi

# Scan PPTX files
if [ -z "$TARGET_FORMAT" ] || [ "$TARGET_FORMAT" = "pptx" ]; then
    echo "--- PPTX Format ---"
    for file in test-results/outputs/pptx/*.json; do
        [ -f "$file" ] || continue
        basename=$(basename "$file" .json)
        compare_format "pptx" "$basename" "pptx"
    done
    echo
fi

# Scan MD files
if [ -z "$TARGET_FORMAT" ] || [ "$TARGET_FORMAT" = "md" ]; then
    echo "--- Markdown Format ---"
    for file in test-results/outputs/md/*.json; do
        [ -f "$file" ] || continue
        basename=$(basename "$file" .json)
        compare_format "md" "$basename" "md"
    done
    echo
fi

# Print summary
echo "=========================================="
echo "Summary"
echo "=========================================="
echo "Total scanned: $TOTAL"
echo -e "${GREEN}Passed: $PASSED${NC}"
echo -e "${RED}Failed: $FAILED${NC}"
echo -e "${YELLOW}Missing: $MISSING${NC}"
echo

if [ $FAILED -gt 0 ]; then
    echo -e "${RED}❌ QUALITY ISSUES DETECTED${NC}"
    echo
    echo "These are REAL quality gaps (not LLM noise)."
    echo "Fix the failing formats using deterministic workflow:"
    echo "  1. Review failure details above"
    echo "  2. Implement fix in backend code"
    echo "  3. Re-run this script to validate"
    echo "  4. Document with deterministic evidence"
    echo
    exit 1
else
    echo -e "${GREEN}✅ ALL FORMATS PASSING${NC}"
    echo
    echo "All tested formats match Python baseline."
    echo "No deterministic quality gaps detected."
    echo
    exit 0
fi
