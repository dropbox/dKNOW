#!/bin/bash
# Monitor baseline generation progress

cd "$(dirname "$0")"

if [ ! -f .generation_pid ]; then
    echo "No generation running (.generation_pid not found)"
    exit 1
fi

PID=$(cat .generation_pid)

if ! ps -p $PID > /dev/null 2>&1; then
    echo "Process $PID not running"
    rm -f .generation_pid
    exit 1
fi

COUNT=$(find master_test_suite/expected_outputs -name "manifest.json" | wc -l | tr -d ' ')
TOTAL=452

PERCENT=$(echo "scale=1; $COUNT * 100 / $TOTAL" | bc)

echo "=================================="
echo "Baseline Generation Progress"
echo "=================================="
echo "PID: $PID"
echo "Complete: $COUNT/$TOTAL ($PERCENT%)"
echo "Remaining: $((TOTAL - COUNT))"
echo ""
echo "To check completion:"
echo "  ./monitor_generation.sh"
echo ""
echo "To view output:"
echo "  tail -f generation_output.log"
echo "=================================="

if [ "$COUNT" -eq "$TOTAL" ]; then
    echo "✓ GENERATION COMPLETE"
    echo "Next: Commit outputs with git add master_test_suite/expected_outputs/"
fi
