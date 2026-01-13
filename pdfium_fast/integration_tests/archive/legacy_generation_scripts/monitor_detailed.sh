#!/bin/bash
# Detailed generation progress monitor

EXPECTED_OUTPUT_DIR="master_test_suite/expected_outputs"
TOTAL_PDFS=452
PID_FILE=".gen_pid"

echo "═══════════════════════════════════════════════════════════"
echo "Baseline Generation Progress Monitor"
echo "═══════════════════════════════════════════════════════════"
echo "Time: $(date '+%Y-%m-%d %H:%M:%S')"
echo

# Check if process is running
if [ -f "$PID_FILE" ]; then
    PID=$(cat "$PID_FILE")
    if ps -p "$PID" > /dev/null 2>&1; then
        CPU_TIME=$(ps -p "$PID" -o time= | tr -d ' ')
        MEM=$(ps -p "$PID" -o rss= | awk '{printf "%.1f MB", $1/1024}')
        echo "Process Status: RUNNING (PID: $PID)"
        echo "  CPU Time: $CPU_TIME"
        echo "  Memory: $MEM"
    else
        echo "Process Status: STOPPED (PID: $PID not found)"
    fi
else
    echo "Process Status: UNKNOWN (no PID file)"
fi
echo

# Count completed PDFs
CURRENT=$(find "$EXPECTED_OUTPUT_DIR" -name "manifest.json" 2>/dev/null | wc -l | tr -d ' ')
REMAINING=$((TOTAL_PDFS - CURRENT))
PERCENT=$(awk "BEGIN {printf \"%.1f\", ($CURRENT/$TOTAL_PDFS)*100}")

echo "PDF Progress:"
echo "  Complete: $CURRENT/$TOTAL_PDFS ($PERCENT%)"
echo "  Remaining: $REMAINING PDFs"
echo

# Estimate time remaining (42 seconds per PDF average)
TIME_PER_PDF=42
REMAINING_SECONDS=$((REMAINING * TIME_PER_PDF))
REMAINING_HOURS=$(awk "BEGIN {printf \"%.1f\", $REMAINING_SECONDS/3600}")
REMAINING_MINUTES=$(awk "BEGIN {printf \"%.0f\", $REMAINING_SECONDS/60}")

echo "Time Estimate:"
echo "  Average: $TIME_PER_PDF seconds/PDF"
echo "  Remaining: $REMAINING_HOURS hours ($REMAINING_MINUTES minutes)"
echo

# Show recent completions
echo "Recent Completions (last 5):"
find "$EXPECTED_OUTPUT_DIR" -name "manifest.json" -mmin -10 | \
    xargs ls -lt 2>/dev/null | \
    head -5 | \
    awk '{print "  " $9}' | \
    sed 's|master_test_suite/expected_outputs/||' | \
    sed 's|/manifest.json||'

echo
echo "═══════════════════════════════════════════════════════════"
