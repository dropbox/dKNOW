#!/bin/bash
#
# build-linux.sh - Linux Binary Builder for Dash PDF Extraction
#
# This script builds Linux binaries either locally or via Docker.
#
# Usage:
#   ./build-linux.sh [--docker|--local]
#
# Options:
#   --docker    Build inside Docker container (recommended, reproducible)
#   --local     Build on local Linux system (requires dependencies)
#   (no args)   Auto-detect (use Docker if available, otherwise local)
#
# Output:
#   binaries/linux/pdfium_cli       - Main executable
#   binaries/linux/libpdfium.so     - Shared library
#
# Prerequisites (for --local):
#   - Ubuntu 20.04+ or similar Linux distribution
#   - depot_tools in PATH
#   - Build dependencies (see Dockerfile for full list)
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
echo "Dash PDF Extraction - Linux Builder"
echo "=========================================="
echo ""

# Parse command line arguments
BUILD_MODE="auto"
if [ "$1" == "--docker" ]; then
    BUILD_MODE="docker"
elif [ "$1" == "--local" ]; then
    BUILD_MODE="local"
fi

# Auto-detect build mode
if [ "$BUILD_MODE" == "auto" ]; then
    if command -v docker &> /dev/null; then
        BUILD_MODE="docker"
        echo "Docker detected. Using Docker build (reproducible)."
    else
        BUILD_MODE="local"
        echo "Docker not found. Using local build."
    fi
fi

echo "Build mode: $BUILD_MODE"
echo ""

# ============================================================================
# Docker Build
# ============================================================================

if [ "$BUILD_MODE" == "docker" ]; then
    echo -e "${BLUE}Building via Docker...${NC}"
    echo ""

    # Check Docker is available
    if ! command -v docker &> /dev/null; then
        echo -e "${RED}ERROR: Docker not found${NC}"
        echo "Install Docker: https://docs.docker.com/get-docker/"
        exit 1
    fi

    # Check Docker daemon is running
    if ! docker info > /dev/null 2>&1; then
        echo -e "${RED}ERROR: Docker daemon not running${NC}"
        echo "Start Docker and try again."
        exit 1
    fi

    # Create output directory
    mkdir -p binaries/linux

    echo -e "${BLUE}Step 1/3: Building Docker image...${NC}"
    echo "This will take 60-90 minutes for the first build."
    echo "Subsequent builds will use cached layers and be much faster."
    echo ""

    # Build Docker image
    if docker build -t pdfium-fast-linux .; then
        echo ""
        echo -e "${GREEN}✓ Docker image built successfully${NC}"
    else
        echo ""
        echo -e "${RED}ERROR: Docker image build failed${NC}"
        exit 1
    fi

    echo ""
    echo -e "${BLUE}Step 2/3: Running container and extracting binaries...${NC}"
    echo ""

    # Run container and extract binaries
    if docker run --rm -v "$(pwd)/binaries/linux:/output" pdfium-fast-linux; then
        echo ""
        echo -e "${GREEN}✓ Binaries extracted successfully${NC}"
    else
        echo ""
        echo -e "${RED}ERROR: Binary extraction failed${NC}"
        exit 1
    fi

    echo ""
    echo -e "${BLUE}Step 3/3: Verifying binaries...${NC}"
    echo ""

    # Verify binaries exist
    if [ -f "binaries/linux/pdfium_cli" ] && [ -f "binaries/linux/libpdfium.so" ]; then
        echo -e "${GREEN}✓ Binaries verified${NC}"
        echo ""
        ls -lh binaries/linux/
    else
        echo -e "${RED}ERROR: Expected binaries not found${NC}"
        exit 1
    fi

# ============================================================================
# Local Build
# ============================================================================

elif [ "$BUILD_MODE" == "local" ]; then
    echo -e "${BLUE}Building locally...${NC}"
    echo ""

    # Check prerequisites
    echo -e "${BLUE}Checking prerequisites...${NC}"

    if ! command -v gn &> /dev/null; then
        echo -e "${RED}ERROR: 'gn' not found in PATH${NC}"
        echo "Install depot_tools: https://commondatastorage.googleapis.com/chrome-infra-docs/flat/depot_tools/docs/html/depot_tools_tutorial.html"
        exit 1
    fi

    if ! command -v ninja &> /dev/null; then
        echo -e "${RED}ERROR: 'ninja' not found in PATH${NC}"
        echo "depot_tools appears incomplete. Please reinstall."
        exit 1
    fi

    echo -e "${GREEN}✓ Prerequisites OK${NC}"
    echo ""

    # Use existing setup.sh script
    echo -e "${BLUE}Running setup.sh...${NC}"
    echo ""

    if ./setup.sh; then
        echo ""
        echo -e "${GREEN}✓ Build successful${NC}"
    else
        echo ""
        echo -e "${RED}ERROR: Build failed${NC}"
        exit 1
    fi

    # Copy binaries to output directory
    echo ""
    echo -e "${BLUE}Copying binaries...${NC}"

    mkdir -p binaries/linux
    cp out/Release/pdfium_cli binaries/linux/
    cp out/Release/libpdfium.so binaries/linux/

    echo -e "${GREEN}✓ Binaries copied to binaries/linux/${NC}"
    echo ""
    ls -lh binaries/linux/
fi

echo ""
echo "=========================================="
echo -e "${GREEN}Linux Build Complete!${NC}"
echo "=========================================="
echo ""
echo "Output location: binaries/linux/"
echo ""
echo "Files:"
echo "  - pdfium_cli       (main executable)"
echo "  - libpdfium.so     (shared library)"
echo ""
echo "Test the binary:"
echo "  ./binaries/linux/pdfium_cli --help"
echo ""
echo "Deploy to Linux server:"
echo "  scp binaries/linux/* user@server:/usr/local/bin/"
echo ""
