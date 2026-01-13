# Linux Binary Build Guide

**Dash PDF Extraction - Building Linux Binaries**

This guide explains how to build Linux binaries for Dash PDF Extraction, either locally on a Linux machine or via Docker on any platform (macOS, Linux, Windows).

---

## Quick Start (Docker - Recommended)

**Prerequisites:** Docker installed and running

```bash
# Build Linux binaries via Docker (60-90 minutes first time)
./build-linux.sh --docker

# Test the binary
./binaries/linux/pdfium_cli --help
```

**Output location:** `binaries/linux/`
- `pdfium_cli` - Main executable
- `libpdfium.so` - Shared library

---

## Method 1: Docker Build (Cross-Platform)

**Advantages:**
- Works on any platform (macOS, Linux, Windows)
- Reproducible builds (identical environment every time)
- No need to install dependencies on host
- Consistent with CI/CD pipelines

**Disadvantages:**
- Requires Docker (10GB+ disk space)
- First build is slow (60-90 minutes)
- Subsequent builds use cached layers (much faster)

### Step 1: Install Docker

**macOS:**
```bash
# Install Docker Desktop
brew install --cask docker
# Start Docker Desktop from Applications
```

**Linux:**
```bash
# Ubuntu/Debian
curl -fsSL https://get.docker.com -o get-docker.sh
sudo sh get-docker.sh
sudo usermod -aG docker $USER
# Log out and back in for group changes to take effect
```

**Windows:**
Download Docker Desktop from https://docs.docker.com/desktop/install/windows-install/

### Step 2: Build

```bash
# Clone repository
git clone https://github.com/dropbox/dash-pdf-extraction.git
cd dash-pdf-extraction

# Build via Docker (automated)
./build-linux.sh --docker

# Or manually:
docker build -t pdfium-fast-linux .
mkdir -p binaries/linux
docker run --rm -v $(pwd)/binaries/linux:/output pdfium-fast-linux
```

### Step 3: Verify

```bash
./binaries/linux/pdfium_cli --help

# Expected output:
# Usage: pdfium_cli [FLAGS] <OPERATION> <INPUT.pdf> <OUTPUT>
# ...
```

### Docker Build Details

**What the Dockerfile does:**
1. Starts with Ubuntu 22.04 LTS base image
2. Installs build dependencies (gcc, clang, python3, etc.)
3. Installs depot_tools (Chromium build system)
4. Downloads PDFium dependencies (~7.2GB)
5. Configures build with gn
6. Compiles with ninja (uses all CPU cores)
7. Copies binaries to output volume

**Build time breakdown:**
- Docker image build: 60-90 minutes (first time)
  - Dependency download: 30-60 minutes
  - Compilation: 20-40 minutes
- Subsequent builds: 2-5 minutes (cached layers)

**Disk space requirements:**
- Docker image: ~10GB
- Build artifacts: ~2GB
- Output binaries: ~50MB

---

## Method 2: Local Linux Build

**Advantages:**
- Faster builds (no Docker overhead)
- Native performance
- Direct access to build artifacts

**Disadvantages:**
- Linux-only (cannot cross-compile on macOS/Windows)
- Requires manual dependency installation
- Less reproducible (depends on system state)

### Prerequisites

**System Requirements:**
- Ubuntu 20.04+ or equivalent Linux distribution
- 10GB free disk space
- 60-90 minutes for first build

**Install Dependencies:**

```bash
# Ubuntu/Debian
sudo apt-get update
sudo apt-get install -y \
    build-essential \
    clang \
    lld \
    git \
    python3 \
    python3-pip \
    curl \
    wget \
    pkg-config \
    libglib2.0-dev \
    cargo \
    rustc
```

**Install depot_tools:**

```bash
# Clone depot_tools
git clone https://chromium.googlesource.com/chromium/tools/depot_tools.git
export PATH="$PWD/depot_tools:$PATH"

# Add to shell profile for persistence
echo 'export PATH="$HOME/depot_tools:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

### Build Steps

```bash
# Clone repository
git clone https://github.com/dropbox/dash-pdf-extraction.git
cd dash-pdf-extraction

# Option 1: Use automated script
./setup.sh

# Option 2: Manual build
# See README.md for manual build instructions

# Copy binaries to output directory
mkdir -p binaries/linux
cp out/Release/pdfium_cli binaries/linux/
cp out/Release/libpdfium.so binaries/linux/
```

### Verify Build

```bash
./binaries/linux/pdfium_cli --help

# Test text extraction
./binaries/linux/pdfium_cli extract-text sample.pdf output.txt

# Test image rendering
./binaries/linux/pdfium_cli --threads 8 render-pages sample.pdf images/
```

---

## Deployment to Linux Servers

### Option 1: Copy Binaries

```bash
# Copy to remote server
scp binaries/linux/* user@server:/usr/local/bin/

# Or use rsync
rsync -av binaries/linux/ user@server:/usr/local/bin/
```

### Option 2: Docker Image

```bash
# Save Docker image to file
docker save pdfium-fast-linux | gzip > pdfium-fast-linux.tar.gz

# Copy to server
scp pdfium-fast-linux.tar.gz user@server:~

# On server: Load image
docker load < pdfium-fast-linux.tar.gz

# Run container
docker run -v /data/pdfs:/pdfs pdfium-fast-linux \
    /build/pdfium_fast/out/Release/pdfium_cli \
    extract-text /pdfs/input.pdf /pdfs/output.txt
```

### Option 3: System Package (Advanced)

Create a `.deb` package for easy installation:

```bash
# TODO: Add packaging instructions for .deb/.rpm
# This would enable: sudo apt-get install dash-pdf-extraction
```

---

## CI/CD Integration

### GitHub Actions

```yaml
name: Build Linux Binaries

on:
  push:
    branches: [main]
  release:
    types: [created]

jobs:
  build-linux:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Build Docker image
        run: docker build -t pdfium-fast-linux .

      - name: Extract binaries
        run: |
          mkdir -p binaries/linux
          docker run --rm -v $(pwd)/binaries/linux:/output pdfium-fast-linux

      - name: Upload artifacts
        uses: actions/upload-artifact@v3
        with:
          name: linux-binaries
          path: binaries/linux/*

      - name: Create release (on tag)
        if: startsWith(github.ref, 'refs/tags/')
        uses: softprops/action-gh-release@v1
        with:
          files: binaries/linux/*
```

### GitLab CI

```yaml
build-linux:
  stage: build
  image: docker:latest
  services:
    - docker:dind
  script:
    - docker build -t pdfium-fast-linux .
    - mkdir -p binaries/linux
    - docker run --rm -v $(pwd)/binaries/linux:/output pdfium-fast-linux
  artifacts:
    paths:
      - binaries/linux/
    expire_in: 1 week
```

---

## Troubleshooting

### Docker Build Fails

**Problem:** Docker build fails with "gclient sync" errors
```
ERROR: gclient sync failed
```

**Solution:** Network timeout. Try again with longer timeout:
```bash
docker build --network=host -t pdfium-fast-linux .
```

**Problem:** Out of disk space
```
ERROR: No space left on device
```

**Solution:** Clean up Docker images:
```bash
docker system prune -a  # Warning: Removes all unused images
```

### Local Build Fails

**Problem:** Missing depot_tools
```
ERROR: 'gn' not found in PATH
```

**Solution:** Install depot_tools:
```bash
git clone https://chromium.googlesource.com/chromium/tools/depot_tools.git
export PATH="$PWD/depot_tools:$PATH"
```

**Problem:** Compiler errors
```
ERROR: Could not find clang
```

**Solution:** Install clang:
```bash
sudo apt-get install clang lld
```

### Runtime Errors

**Problem:** Binary won't run
```
./pdfium_cli: error while loading shared libraries: libpdfium.so: cannot open shared object file
```

**Solution:** Add library to LD_LIBRARY_PATH:
```bash
export LD_LIBRARY_PATH=/path/to/binaries/linux:$LD_LIBRARY_PATH
./binaries/linux/pdfium_cli --help
```

Or copy library to system location:
```bash
sudo cp binaries/linux/libpdfium.so /usr/local/lib/
sudo ldconfig
```

---

## Platform Validation Status

**Current Status:**
- ✅ macOS ARM64 (Apple Silicon) - Fully validated (v1.6.0)
- 🔄 Linux x86_64 - Build infrastructure complete, pending validation
- ❌ Linux ARM64 - Not yet tested
- ❌ Windows - Not yet tested

**Expected Performance (Linux x86_64):**
- Similar to macOS performance (~40x typical speedup)
- May vary by CPU architecture (AVX2 vs NEON)
- 10-20% variance expected vs macOS results

**Validation Plan:**
- Build Linux binary via Docker
- Run full test suite (2,780 tests)
- Measure performance vs baseline
- Document any platform-specific issues

---

## Binary Sizes

**Expected binary sizes (Linux x86_64):**
- `pdfium_cli`: ~30-40 MB (stripped)
- `libpdfium.so`: ~15-20 MB (stripped)

**To reduce binary size:**

```bash
# Strip debug symbols
strip binaries/linux/pdfium_cli
strip binaries/linux/libpdfium.so

# Or rebuild with symbol stripping
gn gen out/Release --args='is_debug=false symbol_level=0'
ninja -C out/Release pdfium_cli
```

---

## Next Steps

**After successful Linux build:**

1. **Validate correctness:** Run test suite on Linux
   ```bash
   cd integration_tests
   pytest -m smoke  # Quick validation
   pytest -m corpus # Full corpus
   ```

2. **Benchmark performance:** Compare vs macOS results
   ```bash
   pytest -m performance
   ```

3. **Document results:** Add Linux results to README.md

4. **Create release:** Tag and publish Linux binaries
   ```bash
   git tag -a v1.7.0-linux -m "Linux binary release"
   git push origin v1.7.0-linux
   ```

---

## License

See [LICENSE](LICENSE) for details.

**Copyright © 2025 Andrew Yates. All rights reserved.**
