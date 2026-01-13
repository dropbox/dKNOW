#!/bin/bash
CURRENT=$(find master_test_suite/expected_outputs -name "manifest.json" | wc -l | tr -d ' ')
TOTAL=452
REMAINING=$((TOTAL - CURRENT))
PERCENT=$(awk "BEGIN {printf \"%.1f\", ($CURRENT/$TOTAL)*100}")

# Estimate time per PDF (in seconds)
# From git history: ~40-45 seconds per PDF single-threaded
TIME_PER_PDF=42

REMAINING_SECONDS=$((REMAINING * TIME_PER_PDF))
REMAINING_HOURS=$(awk "BEGIN {printf \"%.1f\", $REMAINING_SECONDS/3600}")

echo "Progress: $CURRENT/$TOTAL PDFs ($PERCENT%)"
echo "Remaining: $REMAINING PDFs"
echo "Estimated time: $REMAINING_HOURS hours"
