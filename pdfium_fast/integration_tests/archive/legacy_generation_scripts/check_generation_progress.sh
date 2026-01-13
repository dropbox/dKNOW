#!/bin/bash
# Monitor baseline generation progress

PID=66836
EXPECTED=452

echo "Baseline Generation Progress Monitor"
echo "====================================="
echo

# Check if process is running
if ! ps -p $PID > /dev/null 2>&1; then
    echo "Process $PID is not running"
    echo
    COMPLETED=$(find /Users/ayates/pdfium/integration_tests/master_test_suite/expected_outputs -name "manifest.json" 2>/dev/null | wc -l | tr -d ' ')
    echo "Final count: $COMPLETED/$EXPECTED PDFs"

    if [ "$COMPLETED" -eq "$EXPECTED" ]; then
        echo "✓ COMPLETE - All $EXPECTED manifests generated"
        echo
        echo "Next steps:"
        echo "1. cd /Users/ayates/pdfium"
        echo "2. git add integration_tests/master_test_suite/expected_outputs/"
        echo "3. git commit -m '[WORKER0] # 18: Baseline Expected Outputs for 452 PDFs'"
        echo "4. cd integration_tests && pytest -m smoke_fast -v"
    else
        MISSING=$((EXPECTED - COMPLETED))
        echo "⚠ INCOMPLETE - Missing $MISSING PDFs"
        echo
        echo "Check log: integration_tests/expected_outputs_generation.log"
    fi
    exit 0
fi

# Process is running - show progress
ELAPSED=$(ps -p $PID -o etime | tail -1 | tr -d ' ')
COMPLETED=$(find /Users/ayates/pdfium/integration_tests/master_test_suite/expected_outputs -name "manifest.json" 2>/dev/null | wc -l | tr -d ' ')
REMAINING=$((EXPECTED - COMPLETED))
PERCENT=$((COMPLETED * 100 / EXPECTED))

echo "Status: RUNNING (PID $PID)"
echo "Elapsed: $ELAPSED"
echo "Progress: $COMPLETED/$EXPECTED ($PERCENT%)"
echo "Remaining: $REMAINING PDFs"
echo

# Estimate completion
if [ $COMPLETED -gt 0 ]; then
    # Convert elapsed time to seconds (simplified)
    RATE=$(echo "scale=2; $COMPLETED / 220" | bc 2>/dev/null || echo "0.25")
    ETA_SEC=$(echo "scale=0; $REMAINING / $RATE" | bc 2>/dev/null || echo "1400")
    ETA_MIN=$((ETA_SEC / 60))

    echo "Estimated: ~$ETA_MIN minutes remaining"
fi

echo
echo "Monitor with: watch -n 10 ./check_generation_progress.sh"
