# Model Download Guide

This document explains how to download ML models for the Docling PDF ML pipeline.

## Overview

The Docling PDF ML pipeline requires several models from HuggingFace:
- **LayoutPredictor:** Document structure detection (RT-DETR)
- **RapidOCR:** Text detection and recognition (3 models)
- **TableFormer:** Table structure parsing
- **CodeFormula:** Code and formula detection (optional)

Models are downloaded to HuggingFace cache (`~/.cache/huggingface/`) using Python's `huggingface_hub` library.

## Prerequisites

```bash
pip install huggingface_hub
```

## Downloading Models

### Option 1: Python Script (Recommended)

Create a file `download_models.py`:

```python
from huggingface_hub import snapshot_download

# Download layout models
print("Downloading layout models...")
snapshot_download(repo_id="ds4sd/docling-layout-old", revision="main")

# Download OCR models (RapidOCR detection, classification, recognition)
print("Downloading OCR models...")
snapshot_download(repo_id="ds4sd/docling-models", revision="main")

# Download table structure model
print("Downloading table structure model...")
snapshot_download(repo_id="ds4sd/docling-models", revision="main")

# Download code/formula model (optional)
# print("Downloading code/formula model...")
# snapshot_download(repo_id="ds4sd/CodeFormulaV2", revision="main")

print("All models downloaded successfully!")
print(f"Location: ~/.cache/huggingface/hub/")
```

Run:
```bash
python download_models.py
```

### Option 2: Python One-Liners

```bash
# Layout model (default: docling_layout_v2)
python -c "from huggingface_hub import snapshot_download; snapshot_download(repo_id='ds4sd/docling-layout-old', revision='main')"

# OCR models
python -c "from huggingface_hub import snapshot_download; snapshot_download(repo_id='ds4sd/docling-models', revision='main')"

# Table structure model
python -c "from huggingface_hub import snapshot_download; snapshot_download(repo_id='ds4sd/docling-models', revision='main')"
```

### Option 3: Python Docling Library

If you have Python docling installed:

```python
from docling.document_converter import DocumentConverter

# Initialize converter (automatically downloads models on first run)
converter = DocumentConverter()
```

## Layout Model Variants

Multiple layout models are available with different accuracy/speed tradeoffs:

```python
from huggingface_hub import snapshot_download

# Original (default)
snapshot_download(repo_id="ds4sd/docling-layout-old", revision="main")

# Heron variants (newer architecture)
snapshot_download(repo_id="ds4sd/docling-layout-heron", revision="main")
snapshot_download(repo_id="ds4sd/docling-layout-heron-101", revision="main")

# Egret variants (latest, highest accuracy)
snapshot_download(repo_id="ds4sd/docling-layout-egret-medium", revision="main")
snapshot_download(repo_id="ds4sd/docling-layout-egret-large", revision="main")
snapshot_download(repo_id="ds4sd/docling-layout-egret-xlarge", revision="main")
```

## Using Models in Rust

Once downloaded, models are automatically discovered by the Rust code:

```rust
use docling_pdf_ml::model_utils::{find_layout_model, LayoutModelVariant};

// Find model in cache
let model_path = find_layout_model(LayoutModelVariant::DoclingLayoutV2, None)?;
println!("Model found at: {:?}", model_path);

// Find specific model file
let onnx_path = find_layout_onnx_model(LayoutModelVariant::DoclingLayoutV2, None)?;
let pytorch_path = find_layout_pytorch_model(LayoutModelVariant::DoclingLayoutV2, None)?;
```

## Cache Location

Models are stored in:
- **Linux/macOS:** `~/.cache/huggingface/hub/`
- **Windows:** `%USERPROFILE%\.cache\huggingface\hub\`
- **Custom:** Set `HF_HOME` environment variable

Structure:
```
~/.cache/huggingface/hub/
├── models--ds4sd--docling-layout-old/
│   └── snapshots/
│       └── {hash}/
│           ├── model.onnx              # ONNX backend
│           ├── model.safetensors       # PyTorch backend
│           └── config.json             # Model config
└── models--ds4sd--docling-models/
    └── snapshots/
        └── {hash}/
            ├── ocr_detection.onnx      # RapidOCR detection
            ├── ocr_classification.onnx # RapidOCR classification
            └── ocr_recognition.onnx    # RapidOCR recognition
```

## Verifying Downloads

Check if models are downloaded:

```bash
ls -lh ~/.cache/huggingface/hub/models--ds4sd--docling-layout-old/snapshots/*/model.onnx
ls -lh ~/.cache/huggingface/hub/models--ds4sd--docling-models/snapshots/*/ocr_*.onnx
```

Or use Rust:

```rust
use docling_pdf_ml::model_utils::{is_layout_model_available, LayoutModelVariant};

if is_layout_model_available(LayoutModelVariant::DoclingLayoutV2, None) {
    println!("Layout model is ready!");
} else {
    println!("Please download the layout model first");
}
```

## Model Sizes

Approximate download sizes:

| Model | ONNX | PyTorch | Total |
|-------|------|---------|-------|
| Layout (RT-DETR) | 45 MB | 110 MB | 155 MB |
| RapidOCR (3 models) | 15 MB | - | 15 MB |
| TableFormer | 20 MB | 50 MB | 70 MB |
| CodeFormula | - | 120 MB | 120 MB |
| **Total** | ~80 MB | ~280 MB | ~360 MB |

## Troubleshooting

### Model not found error

```
Error: Model 'docling_layout_v2' not found: Model not found in cache.
Download using: python -c "from huggingface_hub import snapshot_download; ..."
```

**Solution:** Download the model using Python first (see commands above).

### Permission denied

**Solution:** Check cache directory permissions:
```bash
chmod -R u+w ~/.cache/huggingface/
```

### Disk space

**Solution:** Models require ~360 MB total. Check available space:
```bash
df -h ~/.cache
```

### Offline usage

Models can be pre-downloaded and cached for offline use. Set `HF_HUB_OFFLINE=1` to prevent downloads.

## References

- [HuggingFace Hub Documentation](https://huggingface.co/docs/huggingface_hub/)
- [Docling Models Repository](https://huggingface.co/ds4sd)
- [Python Docling](https://github.com/docling-project/docling)
