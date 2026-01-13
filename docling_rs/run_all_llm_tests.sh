#!/bin/bash
# Run all LLM verification tests and capture results
# N=1019: Comprehensive quality assessment

# Load OpenAI API key from .env file (gitignored)
source .env

FORMATS="csv html markdown docx xlsx pptx asciidoc webvtt jats"

echo "========================================="
echo "LLM Quality Verification - All Formats"
echo "Date: $(date)"
echo "========================================="
echo ""

PASS_COUNT=0
FAIL_COUNT=0
RESULTS_FILE="llm_test_results_$(date +%Y%m%d_%H%M%S).txt"

for format in $FORMATS; do
    echo "Testing $format..."
    echo "=========================================" >> "$RESULTS_FILE"
    echo "Format: $format" >> "$RESULTS_FILE"
    echo "=========================================" >> "$RESULTS_FILE"

    cargo test test_llm_verification_$format --test llm_verification_tests -- --ignored --nocapture 2>&1 | tee -a "$RESULTS_FILE"

    if [ ${PIPESTATUS[0]} -eq 0 ]; then
        PASS_COUNT=$((PASS_COUNT + 1))
        echo "✅ $format PASSED" | tee -a "$RESULTS_FILE"
    else
        FAIL_COUNT=$((FAIL_COUNT + 1))
        echo "❌ $format FAILED" | tee -a "$RESULTS_FILE"
    fi
    echo "" >> "$RESULTS_FILE"
    echo ""
done

echo "========================================="
echo "SUMMARY"
echo "========================================="
echo "Passed: $PASS_COUNT / $(echo $FORMATS | wc -w | xargs)"
echo "Failed: $FAIL_COUNT"
echo ""
echo "Results saved to: $RESULTS_FILE"
