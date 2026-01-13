#!/bin/bash
#
# setup.sh - Automated PDFium Fast Build Setup
#
# This script automates the complete build setup process for pdfium_fast.
# It handles dependency downloads (~7.2GB) and compilation.
#
# Usage:
#   ./setup.sh              # Full setup: download deps + build (60-90 min)
#   ./setup.sh --deps-only  # Just download dependencies (30-60 min)
#   ./setup.sh --build-only # Just build (assumes deps already downloaded)
#
# Prerequisites:
#   - depot_tools in PATH (https://chromium.googlesource.com/chromium/tools/depot_tools.git)
#   - macOS or Linux
#   - 10GB free disk space
#
# What this script does:
#   1. Checks for depot_tools (gn, ninja, gclient)
#   2. Creates .gclient configuration in parent directory
#   3. Runs 'gclient sync' to download ~7.2GB of dependencies
#   4. Runs 'gn gen' to configure build
#   5. Runs 'ninja' to compile pdfium_cli
#

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo ""
echo "=========================================="
echo "PDFium Fast - Automated Build Setup"
echo "=========================================="
echo ""

# Parse command line arguments
MODE="full"
if [ "$1" == "--deps-only" ]; then
    MODE="deps-only"
elif [ "$1" == "--build-only" ]; then
    MODE="build-only"
fi

echo "Mode: $MODE"
echo ""

# ============================================================================
# Step 1: Check Prerequisites
# ============================================================================

echo -e "${BLUE}[1/5] Checking prerequisites...${NC}"

# Check for depot_tools
if ! command -v gn &> /dev/null; then
    echo -e "${RED}ERROR: 'gn' not found in PATH${NC}"
    echo ""
    echo "You need depot_tools installed and in your PATH."
    echo ""
    echo "Install depot_tools:"
    echo "  git clone https://chromium.googlesource.com/chromium/tools/depot_tools.git"
    echo "  export PATH=\"\$PWD/depot_tools:\$PATH\"  # Add to ~/.bashrc or ~/.zshrc"
    echo ""
    exit 1
fi

if ! command -v ninja &> /dev/null; then
    echo -e "${RED}ERROR: 'ninja' not found in PATH${NC}"
    echo "depot_tools appears incomplete. Please reinstall depot_tools."
    exit 1
fi

if ! command -v gclient &> /dev/null; then
    echo -e "${RED}ERROR: 'gclient' not found in PATH${NC}"
    echo "depot_tools appears incomplete. Please reinstall depot_tools."
    exit 1
fi

echo -e "${GREEN}✓ Found: gn, ninja, gclient${NC}"

# Check disk space (need at least 10GB)
if command -v df &> /dev/null; then
    AVAIL_KB=$(df -k . | tail -1 | awk '{print $4}')
    AVAIL_GB=$((AVAIL_KB / 1024 / 1024))
    if [ $AVAIL_GB -lt 10 ]; then
        echo -e "${YELLOW}WARNING: Only ${AVAIL_GB}GB free disk space. Need at least 10GB.${NC}"
        echo "Continue anyway? (y/n)"
        read -r response
        if [ "$response" != "y" ]; then
            exit 1
        fi
    else
        echo -e "${GREEN}✓ Sufficient disk space: ${AVAIL_GB}GB available${NC}"
    fi
fi

echo ""

# ============================================================================
# Step 2: Setup .gclient Configuration
# ============================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_NAME="$(basename "$SCRIPT_DIR")"
PARENT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
GCLIENT_FILE="$PARENT_DIR/.gclient"

echo -e "${BLUE}[2/5] Setting up workspace...${NC}"
echo "Repository: $SCRIPT_DIR"
echo "Workspace:  $PARENT_DIR"
echo ""

if [ -f "$GCLIENT_FILE" ]; then
    echo -e "${GREEN}✓ .gclient already exists: $GCLIENT_FILE${NC}"
else
    echo "Creating .gclient configuration..."
    cat > "$GCLIENT_FILE" << EOF
solutions = [
  {
    "name": "$REPO_NAME",
    "url": "https://github.com/dropbox/dKNOW/pdfium_fast.git",
    "managed": False,
    "custom_deps": {},
    "custom_vars": {
      "checkout_configuration": "minimal",
    },
  },
]
EOF
    echo -e "${GREEN}✓ Created: $GCLIENT_FILE${NC}"
fi

echo ""

# ============================================================================
# Step 3: Download Dependencies (gclient sync)
# ============================================================================

if [ "$MODE" != "build-only" ]; then
    echo -e "${BLUE}[3/5] Downloading dependencies...${NC}"
    echo ""
    echo "This will download approximately 7.2GB of build tools and libraries."
    echo "Estimated time: 30-60 minutes (depending on network speed)"
    echo ""
    echo "Downloads include:"
    echo "  - build/       (Chromium build system)"
    echo "  - buildtools/  (gn, clang)"
    echo "  - third_party/ (freetype, icu, libjpeg, libpng, zlib, etc.)"
    echo "  - v8/          (JavaScript engine - optional)"
    echo ""

    cd "$PARENT_DIR"

    # Run gclient sync
    if gclient sync; then
        echo ""
        echo -e "${GREEN}✓ Dependencies downloaded successfully${NC}"
    else
        echo ""
        echo -e "${RED}ERROR: gclient sync failed${NC}"
        echo ""
        echo "Common causes:"
        echo "  - Network timeout (try again)"
        echo "  - Insufficient disk space"
        echo "  - depot_tools out of date (update with: cd depot_tools && git pull)"
        exit 1
    fi
else
    echo -e "${BLUE}[3/5] Skipping dependency download (--build-only mode)${NC}"
fi

echo ""

# ============================================================================
# Step 4: Configure Build (gn gen)
# ============================================================================

if [ "$MODE" != "deps-only" ]; then
    cd "$SCRIPT_DIR"

    echo -e "${BLUE}[4/5] Configuring build...${NC}"
    echo ""

    BUILD_DIR="out/Release"

    if [ -d "$BUILD_DIR" ]; then
        echo "Build directory already exists: $BUILD_DIR"
        echo "Reconfiguring..."
    fi

    # Configure build with optimal settings
    # use_clang_modules=false: Prevents Xcode SDK modulemap errors on macOS
    # is_component_build=true: Creates shared library (libpdfium.dylib) needed by tests
    gn gen "$BUILD_DIR" --args='is_debug=false pdf_enable_v8=false pdf_enable_xfa=false is_component_build=true use_clang_modules=false'

    echo ""
    echo -e "${GREEN}✓ Build configured: $BUILD_DIR${NC}"
else
    echo -e "${BLUE}[4/5] Skipping build configuration (--deps-only mode)${NC}"
fi

echo ""

# ============================================================================
# Step 5: Build (ninja)
# ============================================================================

if [ "$MODE" != "deps-only" ]; then
    echo -e "${BLUE}[5/6] Building C++ components...${NC}"
    echo ""
    echo "This will compile pdfium_cli and pdfium_render_bridge."
    echo "Estimated time: 20-40 minutes (first build)"
    echo ""

    if ninja -C "$BUILD_DIR" pdfium_cli pdfium_render_bridge; then
        echo ""
        echo -e "${GREEN}✓ C++ build successful!${NC}"
    else
        echo ""
        echo -e "${RED}ERROR: C++ build failed${NC}"
        echo ""
        echo "Check the error messages above for details."
        echo "Common issues:"
        echo "  - Missing system dependencies (Xcode on macOS)"
        echo "  - Compiler errors (ensure depot_tools is up to date)"
        exit 1
    fi

    echo ""
    echo -e "${BLUE}[6/6] Building Rust components...${NC}"
    echo ""
    echo "This will compile Rust tools (render_pages, extract_text, extract_text_jsonl)."
    echo "Estimated time: 2-5 minutes (first build)"
    echo ""

    cd "$SCRIPT_DIR/rust"
    if cargo build --release --example render_pages --example extract_text --example extract_text_jsonl; then
        echo ""
        echo -e "${GREEN}✓ Rust build successful!${NC}"
    else
        echo ""
        echo -e "${RED}ERROR: Rust build failed${NC}"
        echo ""
        echo "Check the error messages above for details."
        echo "Common issues:"
        echo "  - Rust not installed (install from https://rustup.rs)"
        echo "  - Library path mismatch (check rust/pdfium-sys/build.rs)"
        exit 1
    fi
    cd "$SCRIPT_DIR"
else
    echo -e "${BLUE}[5-6/6] Skipping build (--deps-only mode)${NC}"
fi

echo ""
echo "=========================================="
echo -e "${GREEN}Setup Complete!${NC}"
echo "=========================================="
echo ""

if [ "$MODE" != "deps-only" ]; then
    echo "Binary location:"
    echo "  $SCRIPT_DIR/$BUILD_DIR/pdfium_cli"
    echo ""
    echo "Test the build:"
    echo "  ./$BUILD_DIR/pdfium_cli --help"
    echo ""
fi

echo "Next steps:"
echo ""
echo "  1. Get test PDFs:"
echo "     See integration_tests/DOWNLOAD_TEST_PDFS.md"
echo ""
echo "  2. Install Python test dependencies:"
echo "     pip install -r integration_tests/requirements.txt"
echo ""
echo "  3. Run tests (requires test PDFs):"
echo "     cd integration_tests"
echo "     pytest -m smoke    # Quick validation (63 tests, 2 min)"
echo "     pytest -m extended # Full suite (40 min)"
echo ""
echo "For build documentation, see:"
echo "  README.md         - Quick start guide"
echo "  HOW_TO_BUILD.md   - Detailed build instructions"
echo ""
