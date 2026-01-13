#!/bin/bash
# Calculate actual generation rate

# Starting point (from beginning of session)
START_COUNT=125
START_TIME="2025-11-01 22:57:00"

# Current state
CURRENT_COUNT=$(find master_test_suite/expected_outputs -name "manifest.json" | wc -l | tr -d ' ')
CURRENT_TIME=$(date '+%Y-%m-%d %H:%M:%S')

# Convert to timestamps
START_TS=$(date -j -f "%Y-%m-%d %H:%M:%S" "$START_TIME" +%s)
CURRENT_TS=$(date +%s)

# Calculate
ELAPSED=$((CURRENT_TS - START_TS))
PROCESSED=$((CURRENT_COUNT - START_COUNT))
ELAPSED_MIN=$(awk "BEGIN {printf \"%.1f\", $ELAPSED/60}")

if [ $PROCESSED -gt 0 ]; then
    RATE=$(awk "BEGIN {printf \"%.2f\", $PROCESSED/($ELAPSED/60)}")
    SEC_PER_PDF=$(awk "BEGIN {printf \"%.1f\", $ELAPSED/$PROCESSED}")
    
    TOTAL=452
    REMAINING=$((TOTAL - CURRENT_COUNT))
    REMAINING_SEC=$((REMAINING * $ELAPSED / $PROCESSED))
    REMAINING_HOURS=$(awk "BEGIN {printf \"%.1f\", $REMAINING_SEC/3600}")
    
    echo "Actual Performance:"
    echo "  Start: $START_COUNT PDFs at $START_TIME"
    echo "  Now: $CURRENT_COUNT PDFs at $CURRENT_TIME"
    echo "  Processed: $PROCESSED PDFs in $ELAPSED_MIN minutes"
    echo "  Rate: $RATE PDFs/minute"
    echo "  Average: $SEC_PER_PDF seconds/PDF"
    echo ""
    echo "Remaining:"
    echo "  PDFs: $REMAINING"
    echo "  Estimated: $REMAINING_HOURS hours"
else
    echo "No progress yet"
fi
