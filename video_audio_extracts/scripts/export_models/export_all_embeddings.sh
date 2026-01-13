#!/bin/bash
# Export all embedding models to ONNX format

set -e

echo "=== Exporting Embedding Models to ONNX ==="
echo ""

# Check if Python is available
if ! command -v python3 &> /dev/null; then
    echo "ERROR: python3 not found"
    exit 1
fi

# Check/install required packages
echo "Checking Python dependencies..."
python3 -m pip install -q --upgrade pip
python3 -m pip install -q torch transformers sentence-transformers onnx optimum

echo ""
echo "1/3 Exporting CLIP (ViT-B/32)..."
python3 scripts/export_models/export_clip_onnx.py \
    --model "openai/clip-vit-base-patch32" \
    --output "models/embeddings/clip_vit_b32.onnx"

echo ""
echo "2/3 Exporting Sentence-Transformers (all-MiniLM-L6-v2)..."
python3 scripts/export_models/export_sentence_transformers_onnx.py \
    --model "sentence-transformers/all-MiniLM-L6-v2" \
    --output "models/embeddings/all_minilm_l6_v2.onnx"

echo ""
echo "3/3 Exporting CLAP..."
echo "NOTE: CLAP export may fail if laion-clap is not installed"
echo "      This is optional - audio embeddings will use stub implementation"
python3 scripts/export_models/export_clap_onnx.py \
    --model "laion/clap-htsat-fused" \
    --output "models/embeddings/clap.onnx" || echo "CLAP export failed (optional)"

echo ""
echo "=== Export Complete ==="
echo "Models saved to: models/embeddings/"
ls -lh models/embeddings/
