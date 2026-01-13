#!/bin/bash
# Run ALL LLM tests (verification + mode3) - Comprehensive Quality Assessment
# N=1021: Run all 38 LLM tests to assess true quality status
#
# Duration: ~75 minutes (38 tests × ~2 min/test)
# Cost: ~$0.02 (38 tests × $0.0006/test)

# Setup PATH for cargo
export PATH="$HOME/.cargo/bin:$PATH"

# Load OpenAI API key from .env file (gitignored)
source .env
export OPENAI_API_KEY

# Verification tests (9) - Compare against Python baseline
VERIFICATION_TESTS="csv html markdown xlsx asciidoc docx pptx webvtt jats"

# Mode3 tests (29) - Standalone validation (no Python baseline)
MODE3_TESTS="zip tar 7z rar eml mbox vcf epub fb2 mobi odt ods odp ics ipynb gpx kml kmz bmp gif heif avif stl obj gltf glb dxf svg dicom"

echo "========================================="
echo "COMPREHENSIVE LLM Quality Assessment"
echo "Date: $(date)"
echo "Tests: 38 total (9 verification + 29 mode3)"
echo "Estimated time: 75 minutes"
echo "========================================="
echo ""

PASS_COUNT=0
FAIL_COUNT=0
RESULTS_FILE="llm_comprehensive_results_$(date +%Y%m%d_%H%M%S).txt"

echo "=========================================" >> "$RESULTS_FILE"
echo "COMPREHENSIVE LLM QUALITY ASSESSMENT" >> "$RESULTS_FILE"
echo "Date: $(date)" >> "$RESULTS_FILE"
echo "Branch: $(git rev-parse --abbrev-ref HEAD)" >> "$RESULTS_FILE"
echo "Commit: $(git rev-parse --short HEAD)" >> "$RESULTS_FILE"
echo "=========================================" >> "$RESULTS_FILE"
echo "" >> "$RESULTS_FILE"

# Run verification tests (9)
echo "========================================="
echo "PART 1: VERIFICATION TESTS (9)"
echo "========================================="
echo ""

for format in $VERIFICATION_TESTS; do
    echo "Testing verification_$format..."
    echo "=========================================" >> "$RESULTS_FILE"
    echo "Test: verification_$format" >> "$RESULTS_FILE"
    echo "Type: Verification (baseline comparison)" >> "$RESULTS_FILE"
    echo "=========================================" >> "$RESULTS_FILE"

    cargo test test_llm_verification_$format --test llm_verification_tests -- --ignored --nocapture 2>&1 | tee -a "$RESULTS_FILE"

    if [ ${PIPESTATUS[0]} -eq 0 ]; then
        PASS_COUNT=$((PASS_COUNT + 1))
        echo "✅ verification_$format PASSED" | tee -a "$RESULTS_FILE"
    else
        FAIL_COUNT=$((FAIL_COUNT + 1))
        echo "❌ verification_$format FAILED" | tee -a "$RESULTS_FILE"
    fi
    echo "" >> "$RESULTS_FILE"
    echo ""
done

# Run mode3 tests (29)
echo "========================================="
echo "PART 2: MODE3 TESTS (29)"
echo "========================================="
echo ""

for format in $MODE3_TESTS; do
    echo "Testing mode3_$format..."
    echo "=========================================" >> "$RESULTS_FILE"
    echo "Test: mode3_$format" >> "$RESULTS_FILE"
    echo "Type: Mode3 (standalone validation)" >> "$RESULTS_FILE"
    echo "=========================================" >> "$RESULTS_FILE"

    cargo test test_llm_mode3_$format --test llm_verification_tests -- --ignored --nocapture 2>&1 | tee -a "$RESULTS_FILE"

    if [ ${PIPESTATUS[0]} -eq 0 ]; then
        PASS_COUNT=$((PASS_COUNT + 1))
        echo "✅ mode3_$format PASSED" | tee -a "$RESULTS_FILE"
    else
        FAIL_COUNT=$((FAIL_COUNT + 1))
        echo "❌ mode3_$format FAILED" | tee -a "$RESULTS_FILE"
    fi
    echo "" >> "$RESULTS_FILE"
    echo ""
done

# Summary
TOTAL_TESTS=$((PASS_COUNT + FAIL_COUNT))
PASS_RATE=$((PASS_COUNT * 100 / TOTAL_TESTS))

echo "=========================================" | tee -a "$RESULTS_FILE"
echo "COMPREHENSIVE SUMMARY" | tee -a "$RESULTS_FILE"
echo "=========================================" | tee -a "$RESULTS_FILE"
echo "Total Tests: $TOTAL_TESTS" | tee -a "$RESULTS_FILE"
echo "Passed: $PASS_COUNT ($PASS_RATE%)" | tee -a "$RESULTS_FILE"
echo "Failed: $FAIL_COUNT" | tee -a "$RESULTS_FILE"
echo "" | tee -a "$RESULTS_FILE"
echo "Verification Tests: 9" | tee -a "$RESULTS_FILE"
echo "Mode3 Tests: 29" | tee -a "$RESULTS_FILE"
echo "" | tee -a "$RESULTS_FILE"
echo "Results saved to: $RESULTS_FILE" | tee -a "$RESULTS_FILE"

if [ $FAIL_COUNT -eq 0 ]; then
    echo "🎉 ALL TESTS PASSED!" | tee -a "$RESULTS_FILE"
else
    echo "⚠️  FAILURES DETECTED - Review required" | tee -a "$RESULTS_FILE"
fi
