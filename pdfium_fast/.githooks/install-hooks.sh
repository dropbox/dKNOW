#!/bin/bash
# Install git hooks for PDFium development
#
# This script copies the hooks from .githooks/ to .git/ (which is not tracked by git)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
GIT_DIR="$REPO_ROOT/.git"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo "Installing PDFium git hooks..."

# Check if .git directory exists
if [ ! -d "$GIT_DIR" ]; then
    echo "Error: .git directory not found at $GIT_DIR"
    echo "Are you in the PDFium repository root?"
    exit 1
fi

# Install pre-commit hook
if [ -f "$SCRIPT_DIR/pre-commit" ]; then
    cp "$SCRIPT_DIR/pre-commit" "$GIT_DIR/pre-commit"
    chmod +x "$GIT_DIR/pre-commit"
    echo -e "${GREEN}✓ Installed pre-commit hook${NC}"
else
    echo "Warning: pre-commit hook not found"
fi

echo ""
echo -e "${GREEN}Git hooks installed successfully!${NC}"
echo ""
echo "The pre-commit hook will:"
echo "  1. Check C++ formatting (clang-format)"
echo "  2. Lint Python files (ruff/flake8)"
echo "  3. Lint Rust files (rustfmt/clippy)"
echo "  4. Run PDFium unit tests (if built)"
echo "  5. Run integration smoke tests (if available)"
echo ""
echo -e "${YELLOW}Note: Tests are not yet built. To build:${NC}"
echo "  BUILD_ID=\"dev-\$(git log -1 --format='%h')-\$(date +%Y%m%d-%H%M)\""
echo "  BUILD_DIR=\"out/Test-\${BUILD_ID}\""
echo "  gn gen \"\$BUILD_DIR\" --args=\"is_debug=false pdf_is_standalone=true\""
echo "  ninja -C \"\$BUILD_DIR\" pdfium_unittests pdfium_embeddertests"
echo ""
echo "To bypass the hook temporarily (not recommended):"
echo "  git commit --no-verify"
