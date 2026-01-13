#!/bin/bash
# Install git hooks for docling_rs
#
# Usage: ./scripts/install-hooks.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"
HOOKS_DIR="$REPO_ROOT/.git/hooks"

echo "Installing git hooks..."

# Install pre-commit hook
if [ -f "$HOOKS_DIR/pre-commit" ]; then
    echo "WARNING: pre-commit hook already exists, backing up to pre-commit.backup"
    mv "$HOOKS_DIR/pre-commit" "$HOOKS_DIR/pre-commit.backup"
fi

ln -sf "../../scripts/pre-commit.sh" "$HOOKS_DIR/pre-commit"
chmod +x "$HOOKS_DIR/pre-commit"

echo "Installed pre-commit hook"
echo ""
echo "Git hooks installed successfully!"
echo "The pre-commit hook will run: formatting check, clippy, compilation check"
echo ""
echo "To skip the hook temporarily, use: git commit --no-verify"
