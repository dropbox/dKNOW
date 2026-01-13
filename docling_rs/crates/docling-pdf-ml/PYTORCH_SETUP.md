# PyTorch Setup for LayoutPredictor

## Current Status

The LayoutPredictor module has been copied from the source repository (~8,283 lines) and is **feature-gated** behind the `pytorch` feature flag.

## Building Without PyTorch (Default)

```bash
# Builds successfully without PyTorch/libtorch
cargo build --package docling-pdf-ml
```

The layout predictor module is not compiled by default, keeping dependencies minimal.

## Building With PyTorch Backend

To enable the PyTorch backend for LayoutPredictor:

### 1. Install libtorch

Choose one of these options:

**Option A: Use system-wide PyTorch install**
```bash
# macOS with Homebrew (if available)
brew install pytorch

# Or download from https://pytorch.org/
# Extract to /usr/local/lib/libtorch
```

**Option B: Use Python PyTorch**
```bash
# Set environment variable to use Python's PyTorch
export LIBTORCH_USE_PYTORCH=1

# Requires PyTorch installed in Python:
pip install torch
```

**Option C: Manual download**
```bash
# Download libtorch from https://pytorch.org/
# Extract and set LIBTORCH environment variable:
export LIBTORCH=/path/to/libtorch
```

### 2. Build with pytorch feature

```bash
# With Python PyTorch
LIBTORCH_USE_PYTORCH=1 cargo build --package docling-pdf-ml --features pytorch

# Or with manual libtorch
LIBTORCH=/path/to/libtorch cargo build --package docling-pdf-ml --features pytorch
```

## Model Files

The LayoutPredictor requires model files from HuggingFace:

**Repository:** `ds4sd/docling-models`

**Required files:**
- ONNX backend: `model.onnx` (layout detection)
- PyTorch backend: `model.safetensors` + `config.json`

**Download location:** `~/.cache/huggingface/hub/models--ds4sd--docling-models/`

## Implementation Details

### Architecture

The layout predictor implements **RT-DETR v2** (Real-Time Detection Transformer):
- **Backbone:** ResNet-50 (feature extraction)
- **Encoder:** Hybrid FPN+PAN encoder (multi-scale fusion)
- **Decoder:** 6-layer Transformer decoder with deformable attention
- **Heads:** Classification + bbox regression

### Modules

- `onnx.rs` (944 lines) - ONNX and PyTorch backend orchestrator
- `pytorch_backend/model.rs` (1,702 lines) - RT-DETR model
- `pytorch_backend/encoder.rs` (2,087 lines) - ResNet + hybrid encoder
- `pytorch_backend/decoder.rs` (1,909 lines) - Transformer decoder
- `pytorch_backend/resnet.rs` (572 lines) - ResNet-50 backbone
- `pytorch_backend/deformable_attention.rs` (649 lines) - Deformable attention mechanism
- `pytorch_backend/transformer.rs` (528 lines) - Transformer layers
- `pytorch_backend/weights.rs` (681 lines) - SafeTensors weight loading

### Performance (from source repo N=485)

- **ONNX backend:** 239.35 ms/page (4.18 pages/sec)
- **PyTorch backend:** 153.43 ms/page (6.52 pages/sec)
- **Speedup:** 1.56x faster (35.9% improvement)

## Next Steps (Future Work)

1. **Phase 6:** Download/verify model files
2. **Phase 7:** Test ONNX backend (no libtorch required)
3. **Phase 8:** Test PyTorch backend (requires libtorch)
4. **Phase 9:** Integrate with PDF backend

## References

- [tch-rs README](https://github.com/LaurentMazare/tch-rs/blob/main/README.md)
- [PyTorch Downloads](https://pytorch.org/get-started/locally/)
- [HuggingFace docling-models](https://huggingface.co/ds4sd/docling-models)
