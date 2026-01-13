#!/bin/bash
# Setup environment for PDF ML development
# Based on crates/docling-pdf-ml/README.md

# Use PyTorch from Python environment
export LIBTORCH_USE_PYTORCH=1

# Bypass version check (we have PyTorch 2.9.0, tch expects 2.5.1)
export LIBTORCH_BYPASS_VERSION_CHECK=1

# Add PyTorch and LLVM libraries to library path
export DYLD_LIBRARY_PATH=/opt/homebrew/lib/python3.14/site-packages/torch/lib:/opt/homebrew/opt/llvm/lib

echo "Environment configured for PDF ML:"
echo "  LIBTORCH_USE_PYTORCH=1"
echo "  LIBTORCH_BYPASS_VERSION_CHECK=1"
echo "  DYLD_LIBRARY_PATH=/opt/homebrew/lib/python3.14/site-packages/torch/lib:/opt/homebrew/opt/llvm/lib"
