#!/bin/bash
set -e

# Test script to validate Linux build workflow logic locally
# (without actually building for Linux on macOS)

echo "=== Testing Linux Build Workflow Logic ==="
echo ""

# Determine version
VERSION="${1:-v1.7.0-test}"
echo "Version: $VERSION"

# Create release directory structure (simulating package step)
echo ""
echo "Step 1: Creating release directory structure..."
mkdir -p "releases/${VERSION}/linux-x86_64"

# Simulate copying binary (use macOS binary as placeholder)
echo ""
echo "Step 2: Simulating binary copy..."
if [ -f "out/Release/pdfium_cli" ]; then
    cp out/Release/pdfium_cli "releases/${VERSION}/linux-x86_64/"
    chmod +x "releases/${VERSION}/linux-x86_64/pdfium_cli"
    echo "✓ Binary copied (using macOS binary as placeholder)"
else
    echo "✗ ERROR: pdfium_cli not found in out/Release/"
    exit 1
fi

# Create README
echo ""
echo "Step 3: Creating README..."
cat > "releases/${VERSION}/linux-x86_64/README.md" << 'EOF'
# PDFium CLI - Linux x86_64

## System Requirements
- Ubuntu 20.04+ (glibc 2.31+)
- Other Linux distributions with compatible glibc

## Installation
```bash
# Extract archive
tar xzf linux-x86_64.tar.gz
cd linux-x86_64

# Make executable
chmod +x pdfium_cli

# Test installation
./pdfium_cli --help
```

## Usage
```bash
# Extract text
./pdfium_cli extract-text document.pdf output.txt

# Extract JSONL metadata
./pdfium_cli extract-jsonl document.pdf output.jsonl

# Render pages to images
./pdfium_cli render-pages document.pdf images/

# Multi-threaded rendering (8 workers)
./pdfium_cli --workers 8 render-pages document.pdf images/
```

## Verification
Run SHA256 checksum to verify binary integrity:
```bash
sha256sum -c SHA256SUMS
```
EOF
echo "✓ README created"

# Generate checksums
echo ""
echo "Step 4: Generating checksums..."
cd "releases/${VERSION}/linux-x86_64"
shasum -a 256 pdfium_cli > SHA256SUMS
echo "✓ Checksums generated:"
cat SHA256SUMS
cd ../../..

# Create tarball
echo ""
echo "Step 5: Creating tarball..."
tar czf "linux-x86_64-${VERSION}.tar.gz" -C "releases/${VERSION}" linux-x86_64
echo "✓ Tarball created: linux-x86_64-${VERSION}.tar.gz"

# Show contents
echo ""
echo "Step 6: Verifying tarball contents..."
tar tzf "linux-x86_64-${VERSION}.tar.gz"

# Show size
echo ""
echo "Step 7: Package size:"
ls -lh "linux-x86_64-${VERSION}.tar.gz"

# Verify extraction
echo ""
echo "Step 8: Testing tarball extraction..."
rm -rf test_extract
mkdir test_extract
tar xzf "linux-x86_64-${VERSION}.tar.gz" -C test_extract
echo "✓ Extraction successful"

echo ""
echo "Step 9: Verifying extracted files..."
if [ -f "test_extract/linux-x86_64/pdfium_cli" ]; then
    echo "✓ pdfium_cli present"
fi
if [ -f "test_extract/linux-x86_64/README.md" ]; then
    echo "✓ README.md present"
fi
if [ -f "test_extract/linux-x86_64/SHA256SUMS" ]; then
    echo "✓ SHA256SUMS present"
fi

echo ""
echo "=== Local Test Complete ==="
echo ""
echo "Artifacts created:"
echo "  - releases/${VERSION}/linux-x86_64/ (directory)"
echo "  - linux-x86_64-${VERSION}.tar.gz (tarball)"
echo ""
echo "Note: This test uses macOS binary as placeholder."
echo "Actual Linux binary will be built by GitHub Actions."

# Cleanup test extraction
rm -rf test_extract
