#!/bin/bash
# End-to-end test for AI PDF Review System
#
# This script validates the complete AI review workflow:
# 1. dlviz-screenshot renders PDF with ML detection and outputs JSON sidecar
# 2. AI writes corrections.json (simulated here)
# 3. dlviz-apply-corrections applies corrections and exports to COCO/YOLO
#
# Prerequisites:
# - libpdfium.dylib in repo root (symlink to pdfium_fast/out/Release)
# - cargo build --features "cli,pdf-ml" completed

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"
TEST_DIR="/tmp/ai_review_e2e_test"
TEST_PDF="$REPO_ROOT/test-corpus/pdf/2305.03393v1-pg9.pdf"

echo "=== AI PDF Review System E2E Test ==="
echo

# Setup
rm -rf "$TEST_DIR"
mkdir -p "$TEST_DIR"

# Check prerequisites
if [ ! -f "$TEST_PDF" ]; then
    echo "ERROR: Test PDF not found: $TEST_PDF"
    exit 1
fi

if [ ! -f "$REPO_ROOT/target/debug/dlviz-screenshot" ]; then
    echo "ERROR: dlviz-screenshot not built. Run: cargo build --features cli,pdf-ml"
    exit 1
fi

# Set library path
export DYLD_LIBRARY_PATH="$HOME/pdfium_fast/out/Release:$DYLD_LIBRARY_PATH"

echo "Step 1: Render PDF with ML detection..."
"$REPO_ROOT/target/debug/dlviz-screenshot" "$TEST_PDF" \
    --page 0 \
    --stage reading-order \
    --output-dir "$TEST_DIR/"

# Verify outputs
if [ ! -f "$TEST_DIR/2305.03393v1-pg9_page_000_ReadingOrder.png" ]; then
    echo "ERROR: PNG not generated"
    exit 1
fi

if [ ! -f "$TEST_DIR/2305.03393v1-pg9_page_000_ReadingOrder.json" ]; then
    echo "ERROR: JSON sidecar not generated"
    exit 1
fi

echo "  ✓ PNG generated"
echo "  ✓ JSON sidecar generated"

# Verify JSON structure
if ! grep -q '"width":' "$TEST_DIR/2305.03393v1-pg9_page_000_ReadingOrder.json"; then
    echo "ERROR: JSON missing 'width' field"
    exit 1
fi

if ! grep -q '"page_size":' "$TEST_DIR/2305.03393v1-pg9_page_000_ReadingOrder.json"; then
    echo "ERROR: JSON missing 'page_size' field"
    exit 1
fi

echo "  ✓ JSON structure correct"
echo

echo "Step 2: Create simulated AI corrections..."
cat > "$TEST_DIR/corrections.json" << 'EOF'
{
  "document": "2305.03393v1-pg9.pdf",
  "reviewed_by": "test-script",
  "timestamp": "2025-12-26T00:00:00Z",
  "corrections": [
    {
      "type": "bbox",
      "page": 0,
      "element_id": 0,
      "original": {"x": 127.2, "y": 285.48, "width": 353.27, "height": 180.72},
      "corrected": {"x": 125.0, "y": 283.0, "width": 358.0, "height": 185.0},
      "reason": "Table bbox expanded"
    },
    {
      "type": "label",
      "page": 0,
      "element_id": 15,
      "original": "text",
      "corrected": "caption",
      "reason": "This is a table caption"
    },
    {
      "type": "add",
      "page": 0,
      "label": "page_footer",
      "bbox": {"x": 280.0, "y": 750.0, "width": 50.0, "height": 15.0},
      "text": "9",
      "reason": "Page number missed"
    },
    {
      "type": "delete",
      "page": 0,
      "element_id": 13,
      "reason": "False positive"
    }
  ],
  "summary": {
    "pages_reviewed": 1,
    "total_corrections": 4,
    "bbox_corrections": 1,
    "label_corrections": 1,
    "additions": 1,
    "deletions": 1
  }
}
EOF

echo "  ✓ corrections.json written"
echo

echo "Step 3: Apply corrections and export..."
"$REPO_ROOT/target/debug/dlviz-apply-corrections" "$TEST_DIR" \
    --output "$TEST_DIR/golden" \
    --format both

# Verify outputs
if [ ! -f "$TEST_DIR/golden/annotations.json" ]; then
    echo "ERROR: COCO annotations not generated"
    exit 1
fi

if [ ! -d "$TEST_DIR/golden/labels" ]; then
    echo "ERROR: YOLO labels not generated"
    exit 1
fi

if [ ! -f "$TEST_DIR/golden/classes.txt" ]; then
    echo "ERROR: classes.txt not generated"
    exit 1
fi

echo "  ✓ COCO annotations exported"
echo "  ✓ YOLO labels exported"
echo "  ✓ classes.txt written"
echo

echo "Step 4: Verify corrections applied..."

# Check bbox correction
if ! grep -q '"x": 125.0' "$TEST_DIR/golden/corrected/page_000.json"; then
    echo "ERROR: BBox correction not applied"
    exit 1
fi
echo "  ✓ BBox correction applied"

# Check label correction
if ! grep -q '"label": "caption"' "$TEST_DIR/golden/corrected/page_000.json"; then
    echo "ERROR: Label correction not applied"
    exit 1
fi
echo "  ✓ Label correction applied"

# Check element addition
if ! grep -q '"label": "page_footer"' "$TEST_DIR/golden/corrected/page_000.json"; then
    echo "ERROR: Element addition not applied"
    exit 1
fi
echo "  ✓ Element addition applied"

# Check element deletion
if grep -q '"id": 13,' "$TEST_DIR/golden/corrected/page_000.json"; then
    echo "ERROR: Element deletion not applied"
    exit 1
fi
echo "  ✓ Element deletion applied"

echo
echo "=== E2E Test PASSED ==="
echo
echo "Output files in $TEST_DIR:"
ls -la "$TEST_DIR"
echo
echo "Golden training data in $TEST_DIR/golden:"
ls -la "$TEST_DIR/golden"
