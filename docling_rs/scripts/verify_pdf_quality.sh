#!/bin/bash
# PDF Quality Verification Script
# This script generates fresh output and compares against Python groundtruth
#
# Usage: ./scripts/verify_pdf_quality.sh [test_pdf]
# Default test PDF: 2305.03393v1

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TEST_PDF="${1:-2305.03393v1}"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
OUTPUT_DIR="/tmp/pdf_quality_${TIMESTAMP}"

echo "=== PDF Quality Verification ==="
echo "Test PDF: $TEST_PDF"
echo "Timestamp: $TIMESTAMP"
echo "Output dir: $OUTPUT_DIR"
echo ""

mkdir -p "$OUTPUT_DIR"

# Find groundtruth
GROUNDTRUTH="$PROJECT_ROOT/test-corpus/groundtruth/docling_v2/${TEST_PDF}.md"
if [[ ! -f "$GROUNDTRUTH" ]]; then
    echo "ERROR: Groundtruth not found: $GROUNDTRUTH"
    exit 1
fi

echo "Groundtruth: $GROUNDTRUTH"
EXPECTED_LINES=$(wc -l < "$GROUNDTRUTH")
EXPECTED_CHARS=$(wc -c < "$GROUNDTRUTH")
echo "  Lines: $EXPECTED_LINES"
echo "  Chars: $EXPECTED_CHARS"
echo ""

# Find test PDF
TEST_PDF_PATH="$PROJECT_ROOT/test-corpus/pdf/${TEST_PDF}.pdf"
if [[ ! -f "$TEST_PDF_PATH" ]]; then
    echo "ERROR: Test PDF not found: $TEST_PDF_PATH"
    exit 1
fi

echo "Test PDF: $TEST_PDF_PATH"
echo ""

# Run Rust pipeline to generate output
echo "=== Running Rust PDF Pipeline ==="
echo "This may take a few minutes..."

# Check if the integration test exists
TEST_NAME="test_canon_pdf_${TEST_PDF//-/_}"
echo "Looking for test: $TEST_NAME"

# Run the test and capture output
cd "$PROJECT_ROOT"
RUST_OUTPUT="$OUTPUT_DIR/rust_output.md"
RUST_JSON="$OUTPUT_DIR/rust_output.json"

# Try to run the canonical test
if cargo test -p docling-core --test integration_tests "$TEST_NAME" -- --exact --nocapture 2>&1 | tee "$OUTPUT_DIR/test_output.log"; then
    echo "Test completed."
else
    echo "Test failed or not found. Check $OUTPUT_DIR/test_output.log"
fi

# Look for generated output
echo ""
echo "=== Checking for output files ==="

# Common output locations
for loc in \
    "$PROJECT_ROOT/test-results/outputs/pdf/${TEST_PDF}.txt" \
    "$PROJECT_ROOT/test-results/outputs/pdf/${TEST_PDF}.md" \
    "/tmp/${TEST_PDF}.md" \
    "/tmp/test_labels.md"; do
    if [[ -f "$loc" ]]; then
        echo "Found: $loc"
        cp "$loc" "$RUST_OUTPUT"
        break
    fi
done

if [[ ! -f "$RUST_OUTPUT" ]]; then
    echo "WARNING: Could not find Rust output file"
    echo "You may need to run the PDF pipeline manually"
    exit 1
fi

# Calculate metrics
echo ""
echo "=== Quality Metrics ==="
RUST_LINES=$(wc -l < "$RUST_OUTPUT")
RUST_CHARS=$(wc -c < "$RUST_OUTPUT")

LINE_DIFF=$(echo "scale=1; ($RUST_LINES - $EXPECTED_LINES) * 100 / $EXPECTED_LINES" | bc)
CHAR_DIFF=$(echo "scale=1; ($RUST_CHARS - $EXPECTED_CHARS) * 100 / $EXPECTED_CHARS" | bc)

echo "Python (expected):"
echo "  Lines: $EXPECTED_LINES"
echo "  Chars: $EXPECTED_CHARS"
echo ""
echo "Rust (actual):"
echo "  Lines: $RUST_LINES"
echo "  Chars: $RUST_CHARS"
echo ""
echo "Difference:"
echo "  Lines: $LINE_DIFF%"
echo "  Chars: $CHAR_DIFF%"
echo ""

# Check success criteria
echo "=== Success Criteria ==="
SUCCESS=true

# Criterion 1: First line should be title
FIRST_LINE=$(head -1 "$RUST_OUTPUT")
if [[ "$FIRST_LINE" == "## "* ]]; then
    echo "✅ First line is a heading: $FIRST_LINE"
else
    echo "❌ First line is NOT a heading: $FIRST_LINE"
    SUCCESS=false
fi

# Criterion 2: No arXiv header in first 10 lines
if head -10 "$RUST_OUTPUT" | grep -qi "arxiv:"; then
    echo "❌ arXiv identifier found in first 10 lines"
    SUCCESS=false
else
    echo "✅ No arXiv identifier in first 10 lines"
fi

# Criterion 3: Line count reasonable (<300)
if [[ $RUST_LINES -lt 300 ]]; then
    echo "✅ Line count reasonable: $RUST_LINES < 300"
else
    echo "❌ Line count too high: $RUST_LINES >= 300"
    SUCCESS=false
fi

# Criterion 4: Character difference <10%
CHAR_DIFF_ABS=$(echo "$CHAR_DIFF" | tr -d '-')
if (( $(echo "$CHAR_DIFF_ABS < 10" | bc -l) )); then
    echo "✅ Character difference acceptable: $CHAR_DIFF%"
else
    echo "❌ Character difference too high: $CHAR_DIFF%"
    SUCCESS=false
fi

# Criterion 5: No fake section headers (like "## 1873.")
if grep -E "^## [0-9]{4}\." "$RUST_OUTPUT" > /dev/null; then
    echo "❌ Fake section headers found (year patterns like '## 1873.')"
    grep -E "^## [0-9]{4}\." "$RUST_OUTPUT" | head -3
    SUCCESS=false
else
    echo "✅ No fake section headers (year patterns)"
fi

# Criterion 6: No running headers (like "4 M. Lysak")
if grep -E "^[0-9]+ [A-Z]\. [A-Z]" "$RUST_OUTPUT" | head -3 | grep -v "^1\. \|^2\. \|^3\. " > /dev/null 2>&1; then
    echo "❌ Running headers found"
    grep -E "^[0-9]+ [A-Z]\. [A-Z]" "$RUST_OUTPUT" | head -3
    SUCCESS=false
else
    echo "✅ No running headers detected"
fi

echo ""
echo "=== Final Result ==="
if $SUCCESS; then
    echo "✅ ALL CRITERIA PASSED"
else
    echo "❌ SOME CRITERIA FAILED"
fi

# Save diff for analysis
echo ""
echo "=== Generating diff ==="
diff "$GROUNDTRUTH" "$RUST_OUTPUT" > "$OUTPUT_DIR/diff.txt" 2>&1 || true
echo "Full diff saved to: $OUTPUT_DIR/diff.txt"
echo "First 50 lines of diff:"
head -50 "$OUTPUT_DIR/diff.txt"

# Summary
echo ""
echo "=== Files Generated ==="
echo "  Rust output: $RUST_OUTPUT"
echo "  Test log: $OUTPUT_DIR/test_output.log"
echo "  Diff: $OUTPUT_DIR/diff.txt"
echo ""
echo "To compare manually:"
echo "  diff $GROUNDTRUTH $RUST_OUTPUT | less"
echo ""
echo "To run LLM quality verification (requires OPENAI_API_KEY):"
echo "  cargo test -p docling-quality-verifier --test visual_quality_tests -- --nocapture"
