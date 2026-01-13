#!/bin/bash
# Binary bisection testing script
# Usage: ./test_bisection_commit.sh <commit_hash> <commit_label>

set -e

COMMIT_HASH="$1"
COMMIT_LABEL="$2"

if [ -z "$COMMIT_HASH" ] || [ -z "$COMMIT_LABEL" ]; then
    echo "Usage: $0 <commit_hash> <commit_label>"
    echo "Example: $0 b2eeda53cb N=290"
    exit 1
fi

echo "════════════════════════════════════════════════════════"
echo "BISECTION TEST: ${COMMIT_LABEL}"
echo "Commit: ${COMMIT_HASH}"
echo "Date: $(date -u +"%Y-%m-%d %H:%M:%S UTC")"
echo "════════════════════════════════════════════════════════"

# Checkout commit
echo "→ Checking out commit ${COMMIT_HASH}..."
git checkout "${COMMIT_HASH}" 2>&1 | head -5

# Build binary
echo "→ Building pdfium_cli..."
BUILD_START=$(date +%s)
ninja -C out/Optimized-Shared pdfium_cli 2>&1 | tail -10
BUILD_END=$(date +%s)
BUILD_TIME=$((BUILD_END - BUILD_START))

# Get binary fingerprint
BINARY_PATH="out/Optimized-Shared/pdfium_cli"
if [ ! -f "$BINARY_PATH" ]; then
    echo "✗ Build failed - binary not found: $BINARY_PATH"
    exit 1
fi

BINARY_MD5=$(md5 -q "$BINARY_PATH")
BINARY_TIME=$(stat -f "%Sm" -t "%Y-%m-%d %H:%M:%S" "$BINARY_PATH")

echo "✓ Build complete (${BUILD_TIME}s)"
echo "  Binary: ${BINARY_PATH}"
echo "  MD5: ${BINARY_MD5}"
echo "  Built: ${BINARY_TIME}"
echo ""

# Run JSONL test on single PDF
echo "→ Running JSONL correctness test..."
cd integration_tests

TEST_START=$(date +%s)
pytest tests/pdfs/arxiv/test_arxiv_001.py::test_jsonl_extraction_arxiv_001 -v --tb=short 2>&1 | tee /tmp/bisection_test_output.txt
TEST_RESULT=${PIPESTATUS[0]}
TEST_END=$(date +%s)
TEST_TIME=$((TEST_END - TEST_START))

cd ..

echo ""
echo "════════════════════════════════════════════════════════"
if [ $TEST_RESULT -eq 0 ]; then
    echo "✓ TEST PASSED: ${COMMIT_LABEL}"
    echo "  Result: Space bbox calculation is CORRECT"
    echo "  Conclusion: Bug NOT present in this commit"
else
    echo "✗ TEST FAILED: ${COMMIT_LABEL}"
    echo "  Result: Space bbox calculation is BROKEN"
    echo "  Conclusion: Bug IS present in this commit"

    # Extract diff sample if available
    if grep -q "bbox.*250.58.*500.58" /tmp/bisection_test_output.txt; then
        echo "  Evidence: Space bbox shows 80x error pattern (3 → 250 units)"
    fi
fi
echo "  Test time: ${TEST_TIME}s"
echo "  Commit: ${COMMIT_HASH}"
echo "  Label: ${COMMIT_LABEL}"
echo "════════════════════════════════════════════════════════"

exit $TEST_RESULT
