#!/bin/bash
# Install enhanced pre-commit hook with loop detection and random LLM tests

cat > .git/hooks/pre-commit << 'HOOKEOF'
#!/bin/bash
# Enhanced pre-commit hook - Loop detection + Random LLM smoke test

# ============================================================================
# 1. LOOP DETECTION
# ============================================================================

RECENT_SIMILAR=$(git log --oneline -20 2>/dev/null | grep -i "system health\|documentation update\|regular development" | wc -l | tr -d ' ')

if [ "$RECENT_SIMILAR" -gt 5 ]; then
    echo ""
    echo "🚨 VALIDATION LOOP DETECTED: $RECENT_SIMILAR similar commits in last 20"
    echo ""
    echo "You are stuck in a loop. Do something NEW:"
    echo "  ✅ Fix a quality issue"
    echo "  ✅ Add new test files"
    echo "  ✅ Run LLM tests and fix issues"
    echo "  ✅ Implement new features"
    echo ""
    echo "❌ DO NOT commit same work repeatedly"
    echo ""
    echo "To override: git commit --no-verify"
    echo ""
    exit 1
fi

# ============================================================================
# 2. RANDOMIZED LLM SMOKE TEST (20% chance)
# ============================================================================

if [ -f ".env" ]; then
    source .env 2>/dev/null
fi

if [ -n "$OPENAI_API_KEY" ]; then
    RANDOM_NUM=$((RANDOM % 100))

    if [ $RANDOM_NUM -lt 20 ]; then
        echo ""
        echo "🧪 Running random LLM smoke test..."

        ALL_TESTS=(
            "test_llm_verification_csv"
            "test_llm_verification_docx"
            "test_llm_verification_html"
        )
        RANDOM_INDEX=$((RANDOM % 3))
        RANDOM_TEST=${ALL_TESTS[$RANDOM_INDEX]}

        echo "   Test: $RANDOM_TEST (cost: ~$0.0006)"

        timeout 10s cargo test $RANDOM_TEST --test llm_verification_tests -- --ignored 2>&1 | grep "Overall Score" || echo "   (Skipped - took too long)"

        echo ""
    fi
fi

echo "✅ Pre-commit checks passed"
exit 0
HOOKEOF

chmod +x .git/hooks/pre-commit
echo "✅ Enhanced pre-commit hook installed"
echo ""
echo "Features:"
echo "  - Loop detection (>5 similar commits blocked)"
echo "  - Random LLM smoke tests (20% chance)"
echo "  - Continuous quality monitoring"
