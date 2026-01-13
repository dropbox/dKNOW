#!/bin/bash
# Fix cargo PATH and build environment for all sessions

# Set proper PATH
export PATH="/Users/ayates/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin"

# Set PKG_CONFIG_PATH for leptonica-sys (required for OCR dependencies)
export PKG_CONFIG_PATH="/opt/homebrew/lib/pkgconfig:${PKG_CONFIG_PATH}"

# Verify cargo works
cargo --version || echo "ERROR: Cargo still not found after PATH fix"

# Verify pkg-config can find leptonica
pkg-config --exists lept && echo "✓ leptonica found" || echo "⚠ leptonica not found (required for clippy/build)"

# Use this at start of every session:
# source FIX_CARGO_PATH.sh
