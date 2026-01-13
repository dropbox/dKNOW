#!/bin/bash

# Phase 4: Stress Testing Script (P4.2) - Simplified for macOS
# Purpose: Validate production readiness under extreme conditions
# Worker: WORKER0 N=242

set -e

REPORT_DIR="reports/main"
mkdir -p "$REPORT_DIR"

LOG_FILE="$REPORT_DIR/N242_stress_test.log"
RESULTS_FILE="$REPORT_DIR/N242_stress_test_results.md"

exec > >(tee "$LOG_FILE") 2>&1

echo "=== Phase 4: Stress Testing Started ==="
echo "Time: $(date -u +"%Y-%m-%dT%H:%M:%SZ")"
echo ""

# System info
echo "System Configuration:"
echo "  CPU Cores: $(sysctl -n hw.ncpu)"
echo "  Memory: $(sysctl -n hw.memsize | awk '{printf "%.1f GB", $1/1024/1024/1024}')"
echo "  OS: $(uname -s) $(uname -r)"
echo "  Load: $(uptime | awk -F'load averages:' '{print $2}' | xargs)"
echo ""

# Test 1: Very Large PDF (931 pages)
echo "Test 1: Very Large PDF (931 pages)"
echo "==================================="

LARGE_PDF="integration_tests/pdfs/benchmark/cc_001_931p.pdf"
if [ -f "$LARGE_PDF" ]; then
    echo "Testing: $LARGE_PDF"

    # Single-threaded baseline (with millisecond timing)
    echo "  Baseline (--workers 1): text extraction"
    START=$(python3 -c "import time; print(int(time.time() * 1000))")
    out/Release/pdfium_cli --workers 1 extract-text "$LARGE_PDF" /tmp/stress_large_w1.txt
    END=$(python3 -c "import time; print(int(time.time() * 1000))")
    DURATION_MS_W1=$((END - START))
    DURATION_W1=$(echo "scale=2; $DURATION_MS_W1 / 1000" | bc)
    echo "    Duration: ${DURATION_W1}s"
    PAGES_931=931
    if [ "$DURATION_MS_W1" -gt 0 ]; then
        PPS_W1=$(echo "scale=2; $PAGES_931 * 1000 / $DURATION_MS_W1" | bc)
        echo "    Performance: ${PPS_W1} pages/sec"
    fi

    # Multi-worker (with millisecond timing)
    echo "  Optimized (--workers 4): text extraction"
    START=$(python3 -c "import time; print(int(time.time() * 1000))")
    out/Release/pdfium_cli --workers 4 extract-text "$LARGE_PDF" /tmp/stress_large_w4.txt
    END=$(python3 -c "import time; print(int(time.time() * 1000))")
    DURATION_MS_W4=$((END - START))
    DURATION_W4=$(echo "scale=2; $DURATION_MS_W4 / 1000" | bc)
    echo "    Duration: ${DURATION_W4}s"
    if [ "$DURATION_MS_W4" -gt 0 ]; then
        PPS_W4=$(echo "scale=2; $PAGES_931 * 1000 / $DURATION_MS_W4" | bc)
        echo "    Performance: ${PPS_W4} pages/sec"
    fi

    # Verify correctness
    if [ -f /tmp/stress_large_w1.txt ] && [ -f /tmp/stress_large_w4.txt ]; then
        DIFF=$(diff /tmp/stress_large_w1.txt /tmp/stress_large_w4.txt | wc -l | xargs)
        if [ "$DIFF" -eq 0 ]; then
            echo "    Correctness: PASS (byte-for-byte identical)"
            TEST1_STATUS="PASS"
        else
            echo "    Correctness: FAIL ($DIFF lines differ)"
            TEST1_STATUS="FAIL"
        fi

        # Calculate speedup
        if [ "$DURATION_MS_W1" -gt 0 ] && [ "$DURATION_MS_W4" -gt 0 ]; then
            SPEEDUP=$(echo "scale=2; $DURATION_MS_W1 / $DURATION_MS_W4" | bc)
            echo "    Speedup: ${SPEEDUP}x"
        fi
    fi

    # Cleanup
    rm -f /tmp/stress_large_*.txt
else
    echo "  SKIP: Large PDF not found"
    TEST1_STATUS="SKIP"
fi

echo ""

# Test 2: Batch Processing (50 PDFs for speed)
echo "Test 2: Batch Processing (50 PDFs)"
echo "==================================="

PDF_LIST=$(find integration_tests/pdfs -name "*.pdf" -type f | head -50)
PDF_COUNT=$(echo "$PDF_LIST" | wc -l | xargs)

echo "Processing $PDF_COUNT PDFs sequentially..."

SUCCESS_COUNT=0
FAIL_COUNT=0

START=$(date +%s)

while IFS= read -r pdf; do
    OUTPUT_DIR="/tmp/stress_batch/$(basename "$pdf" .pdf)"
    mkdir -p "$OUTPUT_DIR"

    if out/Release/pdfium_cli --workers 2 render-pages "$pdf" "$OUTPUT_DIR" > /dev/null 2>&1; then
        ((SUCCESS_COUNT++))
    else
        echo "  FAILED: $(basename "$pdf")"
        ((FAIL_COUNT++))
    fi

    # Cleanup
    rm -rf "$OUTPUT_DIR"
done <<< "$PDF_LIST"

END=$(date +%s)
DURATION=$((END - START))

echo ""
echo "Batch Results:"
echo "  Total: $PDF_COUNT"
echo "  Success: $SUCCESS_COUNT"
echo "  Failed: $FAIL_COUNT"
echo "  Duration: ${DURATION}s"
if [ "$DURATION" -gt 0 ]; then
    THROUGHPUT=$(echo "scale=2; $PDF_COUNT / $DURATION" | bc)
    echo "  Throughput: ${THROUGHPUT} PDFs/sec"
fi

if [ "$FAIL_COUNT" -eq 0 ]; then
    echo "  STATUS: PASS (zero failures)"
    TEST2_STATUS="PASS"
elif [ "$FAIL_COUNT" -lt 5 ]; then
    echo "  STATUS: PASS (acceptable failure rate: $FAIL_COUNT/$PDF_COUNT)"
    TEST2_STATUS="PASS"
else
    echo "  STATUS: FAIL (too many failures: $FAIL_COUNT/$PDF_COUNT)"
    TEST2_STATUS="FAIL"
fi

echo ""

# Test 3: Mixed Workload (rendering + text extraction simultaneously)
echo "Test 3: Mixed Workload (50% rendering + 50% text extraction)"
echo "=============================================================="

MIXED_PDF="integration_tests/pdfs/benchmark/0821pages_LUNFJFH4KWZ3ZFNRO43WSMZPLM4OLB7C.pdf"

if [ -f "$MIXED_PDF" ]; then
    echo "Testing: $MIXED_PDF (821 pages)"

    START=$(date +%s)

    # Start rendering in background
    echo "  Starting rendering (background)..."
    out/Release/pdfium_cli --workers 2 render-pages "$MIXED_PDF" /tmp/stress_mixed_render > /tmp/stress_render.log 2>&1 &
    RENDER_PID=$!

    # Start text extraction in background
    echo "  Starting text extraction (background)..."
    out/Release/pdfium_cli --workers 2 extract-text "$MIXED_PDF" /tmp/stress_mixed_text.txt > /tmp/stress_text.log 2>&1 &
    TEXT_PID=$!

    # Wait for both
    echo "  Waiting for completion..."
    wait $RENDER_PID
    RENDER_EXIT=$?
    wait $TEXT_PID
    TEXT_EXIT=$?

    END=$(date +%s)
    DURATION=$((END - START))

    echo "  Render exit code: $RENDER_EXIT"
    echo "  Text exit code: $TEXT_EXIT"
    echo "  Duration: ${DURATION}s"

    if [ $RENDER_EXIT -eq 0 ] && [ $TEXT_EXIT -eq 0 ]; then
        echo "  STATUS: PASS (both processes completed successfully)"
        TEST3_STATUS="PASS"
    else
        echo "  STATUS: FAIL (one or both processes failed)"
        TEST3_STATUS="FAIL"
    fi

    # Cleanup
    rm -rf /tmp/stress_mixed_*
    rm -f /tmp/stress_render.log /tmp/stress_text.log
else
    echo "  SKIP: Test PDF not found"
    TEST3_STATUS="SKIP"
fi

echo ""

# Test 4: Oversubscription (N=16 workers on 16 cores)
echo "Test 4: Oversubscription (N=16 workers on 16 cores)"
echo "===================================================="

OVERSUB_PDF="integration_tests/pdfs/benchmark/cc_001_931p.pdf"

if [ -f "$OVERSUB_PDF" ]; then
    echo "Testing: $OVERSUB_PDF (931 pages)"

    # Test with heavy oversubscription
    echo "  Testing N=16 workers..."
    START=$(date +%s)
    out/Release/pdfium_cli --workers 16 extract-text "$OVERSUB_PDF" /tmp/stress_oversub.txt
    END=$(date +%s)
    DURATION_N16=$((END - START))
    echo "  Duration: ${DURATION_N16}s"

    # Compare with optimal
    echo "  Testing N=4 workers (optimal)..."
    START=$(date +%s)
    out/Release/pdfium_cli --workers 4 extract-text "$OVERSUB_PDF" /tmp/stress_optimal.txt
    END=$(date +%s)
    DURATION_N4=$((END - START))
    echo "  Duration: ${DURATION_N4}s"

    # Check graceful degradation
    if [ "$DURATION_N16" -gt 0 ] && [ "$DURATION_N4" -gt 0 ]; then
        RATIO=$(echo "scale=2; $DURATION_N16 / $DURATION_N4" | bc)
        echo "  Overhead: ${RATIO}x slower than optimal"

        if (( $(echo "$RATIO <= 2.0" | bc -l) )); then
            echo "  STATUS: PASS (graceful degradation, overhead ${RATIO}x <= 2.0x)"
            TEST4_STATUS="PASS"
        else
            echo "  STATUS: FAIL (severe performance degradation, overhead ${RATIO}x > 2.0x)"
            TEST4_STATUS="FAIL"
        fi
    fi

    # Verify correctness
    if [ -f /tmp/stress_oversub.txt ] && [ -f /tmp/stress_optimal.txt ]; then
        DIFF=$(diff /tmp/stress_oversub.txt /tmp/stress_optimal.txt | wc -l | xargs)
        if [ "$DIFF" -eq 0 ]; then
            echo "  Correctness: PASS (byte-for-byte identical)"
        else
            echo "  Correctness: FAIL ($DIFF lines differ)"
            TEST4_STATUS="FAIL"
        fi
    fi

    # Cleanup
    rm -f /tmp/stress_oversub.txt /tmp/stress_optimal.txt
else
    echo "  SKIP: Test PDF not found"
    TEST4_STATUS="SKIP"
fi

echo ""
echo "=== Phase 4: Stress Testing Complete ==="
echo "Time: $(date -u +"%Y-%m-%dT%H:%M:%SZ")"
echo ""

# Summary
echo "Summary:"
echo "  Test 1 (Very Large PDF): ${TEST1_STATUS:-UNKNOWN}"
echo "  Test 2 (Batch Processing): ${TEST2_STATUS:-UNKNOWN}"
echo "  Test 3 (Mixed Workload): ${TEST3_STATUS:-UNKNOWN}"
echo "  Test 4 (Oversubscription): ${TEST4_STATUS:-UNKNOWN}"
echo ""

PASS_COUNT=0
[ "${TEST1_STATUS:-UNKNOWN}" = "PASS" ] && ((PASS_COUNT++))
[ "${TEST2_STATUS:-UNKNOWN}" = "PASS" ] && ((PASS_COUNT++))
[ "${TEST3_STATUS:-UNKNOWN}" = "PASS" ] && ((PASS_COUNT++))
[ "${TEST4_STATUS:-UNKNOWN}" = "PASS" ] && ((PASS_COUNT++))

echo "Overall: $PASS_COUNT/4 tests passed"

if [ "$PASS_COUNT" -eq 4 ]; then
    echo "STATUS: PRODUCTION-READY"
elif [ "$PASS_COUNT" -ge 3 ]; then
    echo "STATUS: ACCEPTABLE (minor issues)"
else
    echo "STATUS: NOT READY (critical failures)"
fi

echo ""
echo "Full log: $LOG_FILE"
