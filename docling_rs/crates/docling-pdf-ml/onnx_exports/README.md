# ONNX Model Exports

This directory contains ONNX-exported models for the PDF ML pipeline.

## Layout Model

The layout model (`layout_optimum/model.onnx`) is required for the `pdf-ml-onnx` feature.

### Generating the Model

The model is too large to store in git. Generate it with:

```bash
python3 << 'EOF'
import torch
from transformers import RTDetrV2ForObjectDetection

model_name = "ds4sd/docling-layout-heron"
print(f"Loading {model_name}...")

model = RTDetrV2ForObjectDetection.from_pretrained(model_name, trust_remote_code=True)
model.eval()
print("Model loaded")

# Create dummy input
batch_size = 1
channels = 3
height = 640
width = 640
dummy_input = torch.randn(batch_size, channels, height, width)

print("Exporting to ONNX...")
output_path = "crates/docling-pdf-ml/onnx_exports/layout_optimum/model.onnx"

# Use legacy export mode
torch.onnx.export(
    model,
    dummy_input,
    output_path,
    export_params=True,
    opset_version=17,
    do_constant_folding=True,
    input_names=['pixel_values'],
    output_names=['logits', 'pred_boxes'],
    dynamic_axes={
        'pixel_values': {0: 'batch_size', 2: 'height', 3: 'width'},
        'logits': {0: 'batch_size'},
        'pred_boxes': {0: 'batch_size'}
    },
    dynamo=False  # Use legacy export
)

print(f"Model exported to {output_path}")
EOF
```

### Requirements

- Python 3.x
- torch >= 2.0
- transformers >= 4.40

### Model Size

The exported model is approximately 164MB.
