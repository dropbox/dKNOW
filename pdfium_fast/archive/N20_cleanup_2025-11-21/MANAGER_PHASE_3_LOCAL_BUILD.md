# MANAGER DIRECTIVE: Phase 3 - Local Binary Builds (No GitHub Actions)

**To:** WORKER0 (N=8+)
**Alternative:** Build binaries locally, upload manually

---

## Revised Phase 3: Local Build Strategy

**Since GitHub Actions unavailable:**
- Build Linux binaries using Docker (on macOS)
- Build macOS x86_64 binary using Rosetta
- Upload binaries manually to GitHub Releases
- Document build process for future CI/CD

---

## Step 3.1: Linux x86_64 Binary via Docker (N=8-9)

### N=8: Create Docker Build Environment

**Create: `docker/Dockerfile.linux-build`**

```dockerfile
FROM ubuntu:22.04

# Install dependencies
RUN apt-get update && apt-get install -y \
    python3 \
    python3-pip \
    git \
    curl \
    pkg-config \
    lsb-release \
    sudo \
    && rm -rf /var/lib/apt/lists/*

# Install depot_tools
RUN git clone https://chromium.googlesource.com/chromium/tools/depot_tools.git /opt/depot_tools
ENV PATH="/opt/depot_tools:${PATH}"

# Create build user
RUN useradd -m -s /bin/bash builder
USER builder
WORKDIR /home/builder

# Copy source
COPY --chown=builder:builder . /home/builder/pdfium_fast/

# Build
WORKDIR /home/builder/pdfium_fast
RUN gn gen out/Release --args='is_debug=false pdf_enable_v8=false pdf_enable_xfa=false target_os="linux" target_cpu="x64"' && \
    ninja -C out/Release pdfium_cli
```

**Build script: `scripts/build_linux_docker.sh`**

```bash
#!/bin/bash
set -e

echo "Building Linux x86_64 binary via Docker..."

# Build Docker image
docker build -f docker/Dockerfile.linux-build -t pdfium-linux-builder .

# Extract binaries
docker create --name pdfium-extract pdfium-linux-builder
docker cp pdfium-extract:/home/builder/pdfium_fast/out/Release/pdfium_cli ./releases/v1.7.0/linux-x86_64/
docker cp pdfium-extract:/home/builder/pdfium_fast/out/Release/libpdfium.so ./releases/v1.7.0/linux-x86_64/
docker rm pdfium-extract

echo "Linux binaries ready at releases/v1.7.0/linux-x86_64/"
ls -lh releases/v1.7.0/linux-x86_64/
```

**Commit:**
```bash
mkdir -p docker scripts releases/v1.7.0/linux-x86_64
# Create files above
chmod +x scripts/build_linux_docker.sh
git add docker/ scripts/
git commit -m "[WORKER0] # 8: Phase 3.1 - Linux Build via Docker"
```

### N=9: Build Linux Binary

**Execute:**
```bash
./scripts/build_linux_docker.sh
# Takes ~60-90 minutes (one-time Docker build)
```

**Package:**
```bash
cd releases/v1.7.0/linux-x86_64
shasum -a 256 * > SHA256SUMS.txt
tar czf ../linux-x86_64.tar.gz .
```

**Commit:**
```bash
git add releases/v1.7.0/
git commit -m "[WORKER0] # 9: Phase 3.1 Complete - Linux x86_64 Binary Built"
```

---

## Step 3.2: macOS x86_64 Binary (N=10, OPTIONAL)

**We already have ARM64 (v1.6.0). Skip x86_64 unless needed.**

Most Macs are now ARM64. Intel Macs can use Rosetta to run ARM64 binaries.

---

## Step 3.3: Upload Binaries to GitHub Release (N=11)

**Manual upload using gh CLI:**

```bash
# Create v1.7.0 draft release
gh release create v1.7.0 \
  --draft \
  --title "v1.7.0: Feature Complete - GPU, Streaming, Binaries, Python" \
  --notes "Release notes here..." \
  releases/v1.7.0/macos-arm64.tar.gz \
  releases/v1.7.0/linux-x86_64.tar.gz

# Verify
gh release view v1.7.0
```

**Commit:**
```bash
git commit -m "[WORKER0] # 11: Phase 3 Complete - Binaries Published"
```

---

## Simplified Phase 3 Summary

**Without GitHub Actions:**
- N=8: Create Docker build setup
- N=9: Build Linux binary locally via Docker
- N=10: (Skip macOS x86_64, unnecessary)
- N=11: Upload to GitHub Release

**Total: 4 commits for Phase 3**

**Then proceed to Phase 4 (Python bindings).**

---

## Alternative: Enable GitHub Actions Later

If you enable GitHub Actions after v1.7.0:
- Worker can add CI/CD in v1.8.0
- Automated builds for every commit
- Cross-platform testing

For now: Local builds + manual upload works fine.

---

**Worker should start N=8: Create Docker build setup for Linux.**
