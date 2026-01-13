#!/bin/bash
#
# Setup Test Infrastructure - Complete Baseline and Manifest Generation
#
# This script performs a complete setup of the test infrastructure:
# 1. Generates text baselines for all PDFs in master list
# 2. Generates image baselines for all PDFs in master list
# 3. Creates main manifest CSV with MD5 hashes
# 4. Creates per-PDF image manifests
# 5. Runs infrastructure verification tests
#
# Usage:
#   ./setup_test_infrastructure.sh              # Full setup (1-2 hours)
#   ./setup_test_infrastructure.sh --text-only  # Text baselines only
#   ./setup_test_infrastructure.sh --manifests-only  # Manifests only (baselines must exist)
#   ./setup_test_infrastructure.sh --verify     # Verify only (no generation)
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Parse command
MODE="${1:-full}"

echo "================================================================"
echo "TEST INFRASTRUCTURE SETUP"
echo "================================================================"
echo "Mode: $MODE"
echo "Directory: $SCRIPT_DIR"
echo ""

# ============================================================================
# Step 1: Generate Baselines
# ============================================================================

if [ "$MODE" == "full" ] || [ "$MODE" == "--text-only" ]; then
    echo -e "${BLUE}Step 1: Generating Text Baselines...${NC}"
    echo "This will take 30-60 minutes for 60 PDFs."
    echo ""

    python3 lib/baseline_generator.py --text-only

    if [ $? -ne 0 ]; then
        echo -e "${RED}✗ Text baseline generation failed${NC}"
        exit 1
    fi

    echo -e "${GREEN}✓ Text baselines generated${NC}"
    echo ""
fi

if [ "$MODE" == "full" ]; then
    echo -e "${BLUE}Step 2: Generating Image Baselines...${NC}"
    echo "This will take 1-2 hours for 60 PDFs."
    echo ""

    python3 lib/baseline_generator.py --images-only

    if [ $? -ne 0 ]; then
        echo -e "${RED}✗ Image baseline generation failed${NC}"
        exit 1
    fi

    echo -e "${GREEN}✓ Image baselines generated${NC}"
    echo ""
fi

# ============================================================================
# Step 2: Generate Manifests
# ============================================================================

if [ "$MODE" != "--verify" ]; then
    echo -e "${BLUE}Step 3: Generating File Manifests...${NC}"
    echo ""

    # Generate main manifest
    echo "Generating main manifest..."
    python3 lib/manifest_generator.py generate-main

    if [ $? -ne 0 ]; then
        echo -e "${RED}✗ Main manifest generation failed${NC}"
        exit 1
    fi

    echo -e "${GREEN}✓ Main manifest generated${NC}"
    echo ""

    # Generate image manifests
    echo "Generating per-PDF image manifests..."
    python3 lib/manifest_generator.py generate-images

    if [ $? -ne 0 ]; then
        echo -e "${YELLOW}⚠ Image manifest generation had warnings (may be incomplete)${NC}"
    else
        echo -e "${GREEN}✓ Image manifests generated${NC}"
    fi
    echo ""
fi

# ============================================================================
# Step 3: Verify Infrastructure
# ============================================================================

echo -e "${BLUE}Step 4: Running Infrastructure Verification Tests...${NC}"
echo ""

# Run smoke tests first (quick)
echo "Running smoke tests..."
pytest -m "infrastructure and smoke" tests/test_000_infrastructure.py -v

if [ $? -ne 0 ]; then
    echo -e "${RED}✗ Infrastructure smoke tests failed${NC}"
    echo ""
    echo "Common issues:"
    echo "  - Manifests not generated: Run with --manifests-only"
    echo "  - Baselines missing: Run with --text-only or full mode"
    echo "  - PDFs missing: Check master_test_suite/file_manifest.csv"
    exit 1
fi

echo -e "${GREEN}✓ Smoke tests passed${NC}"
echo ""

# Run full tests
echo "Running full infrastructure tests..."
pytest -m "infrastructure and full and not images" tests/test_000_infrastructure.py -v

if [ $? -ne 0 ]; then
    echo -e "${RED}✗ Infrastructure full tests failed${NC}"
    exit 1
fi

echo -e "${GREEN}✓ Full tests passed${NC}"
echo ""

# ============================================================================
# Summary
# ============================================================================

echo "================================================================"
echo "TEST INFRASTRUCTURE SETUP COMPLETE"
echo "================================================================"
echo ""
echo "Next steps:"
echo "  1. Review manifest: master_test_suite/file_manifest.csv"
echo "  2. Run smoke tests: pytest -m smoke"
echo "  3. Run full tests: pytest -m full"
echo ""
echo "Infrastructure tests:"
echo "  - Smoke: pytest -m 'infrastructure and smoke'"
echo "  - Full:  pytest -m 'infrastructure and full'"
echo "  - All:   pytest tests/test_000_infrastructure.py"
echo ""
