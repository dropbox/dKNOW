#!/bin/bash
# Batch AI Verification Script
#
# Systematically verifies test outputs using GPT-4 Vision API
# Usage: ./scripts/batch_verify.sh <test_list_file> <output_report>

set -e

if [ $# -lt 2 ]; then
    echo "Usage: $0 <test_list_file> <output_report>"
    echo ""
    echo "test_list_file format (CSV):"
    echo "  input_file,output_json,operation"
    echo ""
    echo "Example:"
    echo "  test_files/image.jpg,debug_output/stage_00_object_detection.json,object-detection"
    exit 1
fi

TEST_LIST="$1"
OUTPUT_REPORT="$2"

# Initialize report
cat > "$OUTPUT_REPORT" << 'EOF'
# AI Verification Report

**Date:** $(date -u +"%Y-%m-%d %H:%M:%S UTC")
**Script:** scripts/batch_verify.sh

---

## Summary

| Metric | Value |
|--------|-------|
| Tests Verified | 0 |
| CORRECT | 0 |
| SUSPICIOUS | 0 |
| INCORRECT | 0 |
| ERROR | 0 |
| Success Rate | 0.0% |

---

## Detailed Results

EOF

# Track statistics
TOTAL=0
CORRECT=0
SUSPICIOUS=0
INCORRECT=0
ERROR_COUNT=0

# Process each test
while IFS=',' read -r input_file output_json operation; do
    # Skip header line
    if [ "$input_file" = "input_file" ]; then
        continue
    fi

    TOTAL=$((TOTAL + 1))
    echo "[$TOTAL] Verifying: $input_file ($operation)"

    # Run verification
    if result=$(python3 scripts/ai_verify_openai.py "$input_file" "$output_json" "$operation" 2>&1); then
        # Parse JSON result
        status=$(echo "$result" | python3 -c "import sys, json; print(json.load(sys.stdin)['status'])" 2>/dev/null || echo "ERROR")
        confidence=$(echo "$result" | python3 -c "import sys, json; print(json.load(sys.stdin)['confidence'])" 2>/dev/null || echo "0.0")
        findings=$(echo "$result" | python3 -c "import sys, json; print(json.load(sys.stdin)['findings'])" 2>/dev/null || echo "")

        # Update counters
        case "$status" in
            CORRECT) CORRECT=$((CORRECT + 1)) ;;
            SUSPICIOUS) SUSPICIOUS=$((SUSPICIOUS + 1)) ;;
            INCORRECT) INCORRECT=$((INCORRECT + 1)) ;;
            *) ERROR_COUNT=$((ERROR_COUNT + 1)) ;;
        esac

        # Append to report
        cat >> "$OUTPUT_REPORT" << EOF

### Test #$TOTAL: $operation
- **Input:** \`$input_file\`
- **Output:** \`$output_json\`
- **Status:** $status
- **Confidence:** $confidence
- **Findings:** $findings

EOF
    else
        ERROR_COUNT=$((ERROR_COUNT + 1))
        echo "  ERROR: Verification failed"

        cat >> "$OUTPUT_REPORT" << EOF

### Test #$TOTAL: $operation (ERROR)
- **Input:** \`$input_file\`
- **Output:** \`$output_json\`
- **Status:** ERROR
- **Error:** Failed to run verification

EOF
    fi

    # Rate limit: 3 requests/minute for OpenAI API (free tier)
    sleep 20
done < "$TEST_LIST"

# Calculate success rate
if [ $TOTAL -gt 0 ]; then
    SUCCESS_RATE=$(echo "scale=1; 100.0 * $CORRECT / $TOTAL" | bc)
else
    SUCCESS_RATE="0.0"
fi

# Update summary in report
sed -i.bak "s/| Tests Verified | 0 |/| Tests Verified | $TOTAL |/" "$OUTPUT_REPORT"
sed -i.bak "s/| CORRECT | 0 |/| CORRECT | $CORRECT |/" "$OUTPUT_REPORT"
sed -i.bak "s/| SUSPICIOUS | 0 |/| SUSPICIOUS | $SUSPICIOUS |/" "$OUTPUT_REPORT"
sed -i.bak "s/| INCORRECT | 0 |/| INCORRECT | $INCORRECT |/" "$OUTPUT_REPORT"
sed -i.bak "s/| ERROR | 0 |/| ERROR | $ERROR_COUNT |/" "$OUTPUT_REPORT"
sed -i.bak "s/| Success Rate | 0.0% |/| Success Rate | ${SUCCESS_RATE}% |/" "$OUTPUT_REPORT"

# Clean up backup file
rm -f "${OUTPUT_REPORT}.bak"

echo ""
echo "Verification complete!"
echo "  Total: $TOTAL"
echo "  CORRECT: $CORRECT"
echo "  SUSPICIOUS: $SUSPICIOUS"
echo "  INCORRECT: $INCORRECT"
echo "  ERROR: $ERROR_COUNT"
echo "  Success Rate: ${SUCCESS_RATE}%"
echo ""
echo "Report saved to: $OUTPUT_REPORT"
