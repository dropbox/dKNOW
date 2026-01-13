#!/bin/bash
# Install git hooks for pdfium development

HOOK_DIR=".git/hooks"

echo "Installing git pre-commit hook..."

cat > "$HOOK_DIR/pre-commit" << 'HOOK_EOF'
#!/bin/bash
# Pre-commit validation hook
# Protects upstream baselines, smoke tests, and runs smoke tests for regressions

set -e

# 1. Protect upstream baselines from modification
if git diff --cached --name-only | grep -q "^integration_tests/baselines/upstream/images_ppm/"; then
    echo "❌ ERROR: Attempting to modify protected upstream baselines!"
    echo ""
    echo "Protected: integration_tests/baselines/upstream/images_ppm/"
    echo ""
    echo "Modified files:"
    git diff --cached --name-only | grep "^integration_tests/baselines/upstream/images_ppm/"
    echo ""
    echo "Use worker_cli baselines instead or override with --no-verify"
    exit 1
fi

# 2. Protect smoke tests from adding skips
if git diff --cached --name-only | grep -q "integration_tests/tests/test_001_smoke.py"; then
    # Check if pytest.skip() is being added to smoke tests
    if git diff --cached integration_tests/tests/test_001_smoke.py | grep -q "^+.*pytest\.skip"; then
        echo "❌ ERROR: Attempting to add pytest.skip() to smoke tests!"
        echo ""
        echo "Smoke tests must NEVER be skipped."
        echo "If a test fails, fix the underlying issue instead."
        echo ""
        echo "Changes detected:"
        git diff --cached integration_tests/tests/test_001_smoke.py | grep "^+.*pytest\.skip"
        echo ""
        echo "Override with --no-verify only if absolutely necessary (not recommended)"
        exit 1
    fi

    # Check if test functions are being commented out or removed
    BEFORE_COUNT=$(git show HEAD:integration_tests/tests/test_001_smoke.py | grep -c "^def test_" || echo 0)
    REMOVED_COUNT=$(git diff --cached integration_tests/tests/test_001_smoke.py | grep -c "^-def test_" || echo 0)

    if [ "$REMOVED_COUNT" -gt 0 ]; then
        echo "⚠️  WARNING: Smoke test functions are being removed!"
        echo ""
        echo "Removed tests: $REMOVED_COUNT"
        echo ""
        echo "Smoke tests protect against regressions."
        echo "Removing tests weakens validation coverage."
        echo ""
        read -p "Continue anyway? (y/N): " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            exit 1
        fi
    fi
fi

# 3. Run smoke tests to catch regressions
# Only run if C++ code or critical test files were modified
CHANGED_FILES=$(git diff --cached --name-only)

if echo "$CHANGED_FILES" | grep -qE "(examples/pdfium_cli\.cpp|fpdfsdk/|core/|tests/test_00|lib/baselines\.py|conftest\.py)"; then
    echo ""
    echo "🧪 Running smoke tests (critical files changed)..."
    echo ""

    cd integration_tests

    if ! pytest -m smoke -q --tb=short; then
        echo ""
        echo "❌ SMOKE TESTS FAILED!"
        echo ""
        echo "Smoke tests protect against regressions:"
        echo "  - Text extraction correctness (1w vs 4w)"
        echo "  - Image rendering functionality"
        echo "  - Basic performance sanity (4w faster than 1w)"
        echo ""
        echo "Fix the failures or use --no-verify to bypass (not recommended)"
        exit 1
    fi

    echo ""
    echo "✅ Smoke tests passed!"
    echo ""
fi

exit 0
HOOK_EOF

chmod +x "$HOOK_DIR/pre-commit"

echo "✅ Pre-commit hook installed"
echo ""
echo "Hook protects:"
echo "  1. Upstream baselines (no modifications)"
echo "  2. Smoke tests (no pytest.skip(), no removal)"
echo "  3. Runs smoke tests on critical changes"
echo ""
echo "Test the hook:"
echo "  ./install_hooks.sh  # Run this script"
echo "  # Then try to modify test_001_smoke.py"
