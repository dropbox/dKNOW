#!/bin/bash

# Phase 4: Stress Testing Script (P4.2)
# Purpose: Validate production readiness under extreme conditions
# Worker: WORKER0 N=242

set -e

REPORT_DIR="reports/main"
mkdir -p "$REPORT_DIR"

LOG_FILE="$REPORT_DIR/N242_stress_test.log"
RESULTS_FILE="$REPORT_DIR/N242_stress_test_results.md"

# Initialize report
cat > "$RESULTS_FILE" << 'EOF'
# Stress Test Results - N=242

**Date**: $(date -u +"%Y-%m-%dT%H:%M:%SZ")
**Worker**: WORKER0 N=242
**Binary**: out/Release/pdfium_cli
**Purpose**: Phase 4 Scale Testing (P4.2) - Validate production readiness

---

## Test Configuration

**System**:
- CPU Cores: $(sysctl -n hw.ncpu)
- Memory: $(sysctl -n hw.memsize | awk '{print $1/1024/1024/1024 " GB"}')
- OS: $(uname -s) $(uname -r)
- Load: $(uptime | awk -F'load average:' '{print $2}')

**Test Suite**:
1. Very Large PDF (1931 pages)
2. Batch Processing (100 PDFs)
3. Mixed Workload (rendering + text extraction)
4. Oversubscription (N=16 workers on 16 cores)

---

EOF

exec > >(tee -a "$LOG_FILE") 2>&1

echo "=== Phase 4: Stress Testing Started ==="
echo "Time: $(date -u +"%Y-%m-%dT%H:%M:%SZ")"
echo ""

# Test 1: Very Large PDF (1931 pages)
echo "Test 1: Very Large PDF (1931 pages)"
echo "======================================"

LARGE_PDF="integration_tests/pdfs/benchmark/1931pages_7ZNNFJGHOEFFP6I4OARCZGH3GPPDNDXC.pdf"
if [ -f "$LARGE_PDF" ]; then
    echo "Testing: $LARGE_PDF"

    # Single-threaded baseline
    echo "  Baseline (K=1): text extraction"
    START=$(date +%s)
    timeout 600 out/Release/pdfium_cli --threads 1 extract-text "$LARGE_PDF" /tmp/stress_large_k1.txt || echo "TIMEOUT or FAILED"
    END=$(date +%s)
    DURATION_K1=$((END - START))
    echo "    Duration: ${DURATION_K1}s"

    # Multi-threaded
    echo "  Optimized (K=8): text extraction"
    START=$(date +%s)
    timeout 600 out/Release/pdfium_cli --threads 8 extract-text "$LARGE_PDF" /tmp/stress_large_k8.txt || echo "TIMEOUT or FAILED"
    END=$(date +%s)
    DURATION_K8=$((END - START))
    echo "    Duration: ${DURATION_K8}s"

    # Verify correctness
    if [ -f /tmp/stress_large_k1.txt ] && [ -f /tmp/stress_large_k8.txt ]; then
        DIFF=$(diff /tmp/stress_large_k1.txt /tmp/stress_large_k8.txt | wc -l)
        if [ "$DIFF" -eq 0 ]; then
            echo "    Correctness: PASS (byte-for-byte identical)"
        else
            echo "    Correctness: FAIL ($DIFF lines differ)"
        fi

        # Calculate speedup
        if [ "$DURATION_K1" -gt 0 ]; then
            SPEEDUP=$(echo "scale=2; $DURATION_K1 / $DURATION_K8" | bc)
            echo "    Speedup: ${SPEEDUP}x"
        fi
    fi

    # Cleanup
    rm -f /tmp/stress_large_*.txt
else
    echo "  SKIP: Large PDF not found"
fi

echo ""

# Test 2: Batch Processing (100 PDFs)
echo "Test 2: Batch Processing (100 PDFs)"
echo "===================================="

PDF_LIST=$(find integration_tests/pdfs -name "*.pdf" -type f | head -100)
PDF_COUNT=$(echo "$PDF_LIST" | wc -l)

echo "Processing $PDF_COUNT PDFs sequentially..."

SUCCESS_COUNT=0
FAIL_COUNT=0
CRASH_COUNT=0

START=$(date +%s)

while IFS= read -r pdf; do
    echo "  Processing: $(basename "$pdf")"

    OUTPUT_DIR="/tmp/stress_batch/$(basename "$pdf" .pdf)"
    mkdir -p "$OUTPUT_DIR"

    if timeout 60 out/Release/pdfium_cli --workers 4 render-pages "$pdf" "$OUTPUT_DIR" > /dev/null 2>&1; then
        ((SUCCESS_COUNT++))
    else
        EXIT_CODE=$?
        if [ $EXIT_CODE -eq 124 ]; then
            echo "    TIMEOUT"
            ((FAIL_COUNT++))
        else
            echo "    FAILED (exit $EXIT_CODE)"
            ((CRASH_COUNT++))
        fi
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
echo "  Crashed: $CRASH_COUNT"
echo "  Duration: ${DURATION}s"
echo "  Throughput: $(echo "scale=2; $PDF_COUNT / $DURATION" | bc) PDFs/sec"

if [ "$CRASH_COUNT" -gt 0 ]; then
    echo "  STATUS: FAIL (crashes detected)"
elif [ "$FAIL_COUNT" -gt 5 ]; then
    echo "  STATUS: FAIL (too many failures)"
else
    echo "  STATUS: PASS"
fi

echo ""

# Test 3: Mixed Workload (rendering + text extraction simultaneously)
echo "Test 3: Mixed Workload (50% rendering + 50% text extraction)"
echo "=============================================================="

MIXED_PDF="integration_tests/pdfs/benchmark/cc_001_931p.pdf"

if [ -f "$MIXED_PDF" ]; then
    echo "Testing: $MIXED_PDF (931 pages)"

    START=$(date +%s)

    # Start rendering in background
    echo "  Starting rendering (background)..."
    out/Release/pdfium_cli --workers 4 render-pages "$MIXED_PDF" /tmp/stress_mixed_render > /tmp/stress_render.log 2>&1 &
    RENDER_PID=$!

    # Start text extraction in background
    echo "  Starting text extraction (background)..."
    out/Release/pdfium_cli --workers 4 extract-text "$MIXED_PDF" /tmp/stress_mixed_text.txt > /tmp/stress_text.log 2>&1 &
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
        echo "  STATUS: PASS (both processes completed)"
    else
        echo "  STATUS: FAIL (one or both processes failed)"
    fi

    # Cleanup
    rm -rf /tmp/stress_mixed_*
    rm -f /tmp/stress_render.log /tmp/stress_text.log
else
    echo "  SKIP: Test PDF not found"
fi

echo ""

# Test 4: Oversubscription (N=16 workers on 16 cores)
echo "Test 4: Oversubscription (N=16 workers on 16 cores)"
echo "===================================================="

OVERSUB_PDF="integration_tests/pdfs/benchmark/0821pages_LUNFJFH4KWZ3ZFNRO43WSMZPLM4OLB7C.pdf"

if [ -f "$OVERSUB_PDF" ]; then
    echo "Testing: $OVERSUB_PDF (821 pages)"

    # Test with heavy oversubscription
    echo "  Testing N=16 workers..."
    START=$(date +%s)
    timeout 300 out/Release/pdfium_cli --workers 16 extract-text "$OVERSUB_PDF" /tmp/stress_oversub.txt || echo "TIMEOUT or FAILED"
    END=$(date +%s)
    DURATION_N16=$((END - START))

    echo "  Duration: ${DURATION_N16}s"

    # Compare with optimal
    echo "  Testing N=4 workers (optimal)..."
    START=$(date +%s)
    timeout 300 out/Release/pdfium_cli --workers 4 extract-text "$OVERSUB_PDF" /tmp/stress_optimal.txt || echo "TIMEOUT or FAILED"
    END=$(date +%s)
    DURATION_N4=$((END - START))

    echo "  Duration: ${DURATION_N4}s"

    # Check graceful degradation
    if [ "$DURATION_N16" -gt 0 ] && [ "$DURATION_N4" -gt 0 ]; then
        RATIO=$(echo "scale=2; $DURATION_N16 / $DURATION_N4" | bc)
        echo "  Overhead: ${RATIO}x slower than optimal"

        if (( $(echo "$RATIO < 2.0" | bc -l) )); then
            echo "  STATUS: PASS (graceful degradation)"
        else
            echo "  STATUS: FAIL (severe performance degradation)"
        fi
    fi

    # Cleanup
    rm -f /tmp/stress_oversub.txt /tmp/stress_optimal.txt
else
    echo "  SKIP: Test PDF not found"
fi

echo ""
echo "=== Phase 4: Stress Testing Complete ==="
echo "Time: $(date -u +"%Y-%m-%dT%H:%M:%SZ")"
echo ""
echo "Full report: $RESULTS_FILE"
echo "Log: $LOG_FILE"
