#!/bin/bash
# Simple end-to-end test of Rust ML from repo root

cd ~/docling_rs
source setup_env.sh

echo "=== Testing Rust ML Pipeline ==="
echo ""

# Test from pdf-ml directory (where models are)
cd crates/docling-pdf-ml
cargo run --example test_pipeline --features "pytorch,opencv-preprocessing" --release

echo ""
echo "✅ Test complete"
