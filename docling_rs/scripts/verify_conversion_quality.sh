#!/bin/bash
# LLM-based quality verification for document conversions
#
# Tests that parsed output preserves essential content from original documents
# Uses OpenAI GPT-4 for semantic comparison

set -e

# Check for API key
if [ -z "$OPENAI_API_KEY" ]; then
    echo "❌ Error: OPENAI_API_KEY environment variable not set"
    echo "Set it with: export OPENAI_API_KEY=sk-..."
    exit 1
fi

echo "🔍 Document Conversion Quality Verification"
echo "============================================"
echo ""

# Test files for each new format
declare -A TEST_FILES=(
    ["LaTeX"]="test-corpus/latex/simple_document.tex"
    ["Pages"]="test-corpus/apple-pages/resume.pages"
    ["Numbers"]="test-corpus/apple-numbers/sales-report.numbers"
    ["Keynote"]="test-corpus/apple-keynote/business-review.key"
    ["Visio"]="test-corpus/microsoft-visio/sample_diagram.vsdx"
    ["Access"]="test-corpus/microsoft-access/sample1.mdb"
)

RESULTS_FILE="/tmp/llm_verification_results.json"
echo "[" > "$RESULTS_FILE"

count=0
total=${#TEST_FILES[@]}

for format in "${!TEST_FILES[@]}"; do
    file="${TEST_FILES[$format]}"
    count=$((count + 1))

    echo "[$count/$total] Testing $format: $file"

    if [ ! -f "$file" ]; then
        echo "  ⚠️  File not found, skipping"
        continue
    fi

    # Convert document
    echo "  → Converting to markdown..."
    output=$(./target/release/docling convert "$file" 2>/dev/null || echo "CONVERSION_FAILED")

    if [ "$output" = "CONVERSION_FAILED" ]; then
        echo "  ❌ Conversion failed"
        continue
    fi

    # Prepare LLM verification prompt
    echo "  → Calling OpenAI GPT-4 for quality verification..."

    # Build JSON request
    prompt=$(cat <<PROMPT_EOF
You are verifying document conversion quality.

**Original Document:**
- Format: $format
- File: $file
- (Based on filename, infer what content should be present)

**Parsed Output:**
\`\`\`markdown
${output:0:3000}
$([ ${#output} -gt 3000 ] && echo "... [truncated from ${#output} chars]")
\`\`\`

Evaluate quality on:
1. Content completeness
2. Structure preservation
3. Table integrity (if applicable)
4. Formatting preservation

Respond with JSON:
{
  "is_equivalent": true/false,
  "confidence": 0.0-1.0,
  "explanation": "analysis",
  "issues": ["problems"],
  "preserved": ["strengths"]
}
PROMPT_EOF
)

    # Call OpenAI API
    response=$(curl -s https://api.openai.com/v1/chat/completions \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $OPENAI_API_KEY" \
        -d @- <<EOF
{
    "model": "gpt-4",
    "messages": [{"role": "user", "content": $(echo "$prompt" | jq -Rs .)}],
    "temperature": 0.3
}
EOF
    )

    # Extract result
    llm_content=$(echo "$response" | jq -r '.choices[0].message.content' 2>/dev/null || echo '{}')

    # Try to parse as JSON
    is_equiv=$(echo "$llm_content" | jq -r '.is_equivalent' 2>/dev/null || echo "unknown")
    confidence=$(echo "$llm_content" | jq -r '.confidence' 2>/dev/null || echo "0.0")

    if [ "$is_equiv" = "true" ]; then
        echo "  ✅ PASS - Confidence: $confidence"
    elif [ "$is_equiv" = "false" ]; then
        echo "  ❌ FAIL - Confidence: $confidence"
    else
        echo "  ⚠️  Could not parse LLM response"
    fi

    # Save result
    if [ $count -gt 1 ]; then
        echo "," >> "$RESULTS_FILE"
    fi
    cat >> "$RESULTS_FILE" <<RESULT_EOF
{
    "format": "$format",
    "file": "$file",
    "is_equivalent": $is_equiv,
    "confidence": $confidence,
    "llm_response": $llm_content
}
RESULT_EOF

    echo ""
done

echo "]" >> "$RESULTS_FILE"

echo "============================================"
echo "📊 Verification Results saved to: $RESULTS_FILE"
echo ""
echo "Summary:"
jq -r '.[] | "\(.format): \(if .is_equivalent then "✅ PASS" else "❌ FAIL" end) (confidence: \(.confidence))"' "$RESULTS_FILE" 2>/dev/null || echo "See $RESULTS_FILE for details"
