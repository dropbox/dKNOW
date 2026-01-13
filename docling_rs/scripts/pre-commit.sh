#!/bin/bash
# Pre-commit hook for docling_rs
# Runs quick validation before commits
#
# Install with: scripts/install-hooks.sh
# Or manually: ln -sf ../../scripts/pre-commit.sh .git/hooks/pre-commit

set -e

# Required for torch-sys crate to find PyTorch
export LIBTORCH_USE_PYTORCH=1

echo "Running pre-commit checks..."

# Check formatting (fast, catches common issues)
echo "Checking code formatting..."
if ! cargo fmt --check 2>/dev/null; then
    echo "ERROR: Code formatting issues found. Run 'cargo fmt' to fix."
    exit 1
fi
echo "  Formatting: OK"

# Run clippy (catches common mistakes)
echo "Running clippy..."
if ! cargo clippy --workspace --exclude docling-pdf-ml --quiet -- -D warnings 2>/dev/null; then
    echo "ERROR: Clippy found issues. Fix them before committing."
    exit 1
fi
echo "  Clippy: OK"

# Quick compile check (catches basic errors)
echo "Checking compilation..."
if ! cargo check --workspace --exclude docling-pdf-ml --quiet 2>/dev/null; then
    echo "ERROR: Compilation failed. Fix errors before committing."
    exit 1
fi
echo "  Compilation: OK"

echo "Pre-commit checks passed!"
