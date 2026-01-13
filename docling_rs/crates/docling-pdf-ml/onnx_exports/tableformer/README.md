# Table Structure Model (Microsoft Table Transformer)

This directory contains the ONNX export of Microsoft's Table Transformer model for table structure recognition.

## Model Details

- **Model:** `microsoft/table-transformer-structure-recognition`
- **Architecture:** DETR-based object detection
- **Input:** Image tensor [1, 3, H, W] (RGB, normalized)
- **Output:**
  - `logits`: [1, 125, 7] class predictions
  - `pred_boxes`: [1, 125, 4] bounding boxes (cx, cy, w, h)

## Labels

| ID | Label |
|----|-------|
| 0 | table |
| 1 | table column |
| 2 | table row |
| 3 | table column header |
| 4 | table projected row header |
| 5 | table spanning cell |
| 6 | no object |

## Generate ONNX Model

The ONNX model files are too large for git (~115MB). Generate them locally:

```bash
# Activate Python 3.12 venv with required packages
source .venv_tableformer/bin/activate

# Install dependencies if needed
pip install torch transformers onnx onnxruntime onnxscript timm

# Export model
python3 << 'PYEOF'
from transformers import TableTransformerForObjectDetection
import torch
import os
import warnings
warnings.filterwarnings('ignore')

model = TableTransformerForObjectDetection.from_pretrained(
    "microsoft/table-transformer-structure-recognition"
)
model.eval()

output_dir = "crates/docling-pdf-ml/onnx_exports/tableformer"
os.makedirs(output_dir, exist_ok=True)

dummy_input = torch.randn(1, 3, 448, 448)

torch.onnx.export(
    model,
    dummy_input,
    os.path.join(output_dir, "table_structure_model.onnx"),
    export_params=True,
    opset_version=17,
    do_constant_folding=True,
    input_names=['pixel_values'],
    output_names=['logits', 'pred_boxes'],
    dynamic_axes={
        'pixel_values': {0: 'batch_size', 2: 'height', 3: 'width'},
        'logits': {0: 'batch_size'},
        'pred_boxes': {0: 'batch_size'}
    }
)
print("Model exported successfully!")
PYEOF
```

## Output Files

After running the export script:
- `table_structure_model.onnx` (~2.24 MB) - model graph
- `table_structure_model.onnx.data` (~115 MB) - weights

## Note on IBM TableFormer

This is a **different model** from IBM's TableFormer used in the original docling:

| Feature | IBM TableFormer | Microsoft Table Transformer |
|---------|-----------------|----------------------------|
| Architecture | Autoregressive decoder | DETR object detection |
| Output | Sequence of tags | Bounding boxes |
| ONNX Support | No (beam search) | Yes |

The post-processing logic needs to be adapted to:
1. Filter detections by confidence
2. Apply NMS
3. Reconstruct cell grid from row/column intersections
