# Dockerfile for Dash PDF Extraction - Linux builds
#
# This Dockerfile creates a reproducible build environment for compiling
# pdfium_cli on Linux (Ubuntu 22.04 LTS).
#
# Build time: ~60-90 minutes for full build
# Image size: ~10GB (includes all build dependencies)
#
# Usage:
#   docker build -t pdfium-fast-linux .
#   docker run -v $(pwd)/binaries:/output pdfium-fast-linux
#
# The compiled binaries will be placed in ./binaries/ on the host machine.

FROM ubuntu:22.04

# Prevent interactive prompts during package installation
ENV DEBIAN_FRONTEND=noninteractive

# Install system dependencies required for Chromium builds
RUN apt-get update && apt-get install -y \
    # Build essentials
    build-essential \
    clang \
    lld \
    # Version control
    git \
    # Python (required by depot_tools and build system)
    python3 \
    python3-pip \
    # Networking tools
    curl \
    wget \
    # Required by PDFium build
    pkg-config \
    libglib2.0-dev \
    # Rust (for optional Rust bindings)
    cargo \
    rustc \
    # Cleanup
    && rm -rf /var/lib/apt/lists/*

# Set up working directory
WORKDIR /build

# Install depot_tools (Chromium build system)
RUN git clone https://chromium.googlesource.com/chromium/tools/depot_tools.git /depot_tools
ENV PATH="/depot_tools:${PATH}"

# Clone the repository
# Note: In production, you'd typically COPY the source code instead
# For now, we clone from GitHub
ARG REPO_URL=https://github.com/dropbox/dash-pdf-extraction.git
ARG BRANCH=main
RUN git clone --branch ${BRANCH} ${REPO_URL} pdfium_fast

# Change to repository directory
WORKDIR /build/pdfium_fast

# Create .gclient configuration in parent directory
# This tells gclient how to sync dependencies
RUN cd /build && \
    cat > .gclient << 'EOF'
solutions = [
  {
    "name": "pdfium_fast",
    "url": "https://github.com/dropbox/dash-pdf-extraction.git",
    "managed": False,
    "custom_deps": {},
    "custom_vars": {
      "checkout_configuration": "minimal",
    },
  },
]
EOF

# Download dependencies (~7.2GB, 30-60 minutes depending on network)
# This is the most time-consuming step
RUN cd /build && gclient sync

# Configure build for Release mode
# - is_debug=false: Optimized build
# - pdf_enable_v8=false: Disable JavaScript engine (not needed)
# - pdf_enable_xfa=false: Disable XFA forms (not needed)
# - is_component_build=true: Create shared library (needed by tests)
# - use_clang_modules=false: Prevent modulemap errors
RUN gn gen out/Release --args='is_debug=false pdf_enable_v8=false pdf_enable_xfa=false is_component_build=true use_clang_modules=false'

# Build pdfium_cli (20-40 minutes)
# Use all available CPU cores for faster compilation
RUN ninja -C out/Release pdfium_cli

# Build Rust components (optional, 2-5 minutes)
# Note: This may fail on some systems due to library path issues
# If Rust bindings aren't needed, this step can be skipped
RUN cd rust && \
    cargo build --release --example render_pages --example extract_text --example extract_text_jsonl || \
    echo "Rust build failed (optional component)"

# Verify the build succeeded
RUN ./out/Release/pdfium_cli --help

# Create output directory for binaries
RUN mkdir -p /output

# Default command: Copy binaries to output volume
# When you run this container with -v $(pwd)/binaries:/output,
# the binaries will be copied to your host machine
CMD cp out/Release/pdfium_cli /output/ && \
    cp out/Release/libpdfium.so /output/ && \
    echo "Binaries copied to /output/" && \
    ls -lh /output/

# Build instructions:
#
# 1. Build the Docker image:
#    docker build -t pdfium-fast-linux .
#
# 2. Extract binaries to host:
#    mkdir -p binaries
#    docker run -v $(pwd)/binaries:/output pdfium-fast-linux
#
# 3. Test the binary:
#    ./binaries/pdfium_cli --help
#
# 4. (Optional) Run interactive container for development:
#    docker run -it pdfium-fast-linux /bin/bash
