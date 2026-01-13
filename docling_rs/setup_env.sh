#!/bin/bash
# Environment setup for PDF ML backend
export LIBTORCH_USE_PYTORCH=1
export LIBTORCH_BYPASS_VERSION_CHECK=1
# LLVM/Clang paths for opencv/clang-sys build (libclang required)
export LIBCLANG_PATH=/opt/homebrew/opt/llvm/lib
export LLVM_CONFIG_PATH=/opt/homebrew/opt/llvm/bin/llvm-config
# Get the directory where this script is located (repo root)
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
# Use Python 3.12 venv's torch (Python 3.14 causes foreign exception crashes)
# PyTorch 2.9.1 from .venv_tableformer works with tch-rs 0.22
# Prepend venv bin to PATH so torch-sys finds the right python
export PATH="$SCRIPT_DIR/.venv_tableformer/bin:$PATH"
TORCH_LIB="$SCRIPT_DIR/.venv_tableformer/lib/python3.12/site-packages/torch/lib"
# pdfium-fast libraries (libpdfium_render_bridge.dylib) located in pdfium_fast build directory
PDFIUM_LIB="$HOME/pdfium_fast/out/Release"
export DYLD_LIBRARY_PATH="$SCRIPT_DIR:$PDFIUM_LIB:$TORCH_LIB:/opt/homebrew/opt/llvm/lib"
export DYLD_FALLBACK_LIBRARY_PATH="$SCRIPT_DIR:$PDFIUM_LIB:/opt/homebrew/opt/llvm/lib"
echo "Environment configured for PDF ML:"
echo "  LIBTORCH_USE_PYTORCH=$LIBTORCH_USE_PYTORCH"
echo "  DYLD_LIBRARY_PATH=$DYLD_LIBRARY_PATH"
echo "  PATH prepended with venv/bin (Python $(python --version 2>&1 | cut -d' ' -f2))"
echo "  (includes repo root for libpdfium.dylib)"
echo "  (includes pdfium_fast for libpdfium_render_bridge.dylib)"
echo "  (uses Python 3.12 venv torch for stability)"
