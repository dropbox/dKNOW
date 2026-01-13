# Logo Detection Models

This directory contains YOLOv8 models trained on logo datasets for brand/logo detection.

## Required Files

The logo-detection plugin requires two files:

1. **yolov8_logo.onnx** - YOLOv8 ONNX model trained on logo dataset
2. **logos.txt** - Text file with logo class names (one per line, matching model classes)

## Model Specification

### ONNX Model Requirements:
- **Architecture**: YOLOv8 (Nano, Small, Medium, Large, or XLarge)
- **Input**: RGB image tensor `[1, 3, 640, 640]` (batch, channels, height, width)
  - Normalized to [0, 1] range (divide pixel values by 255.0)
  - Letterbox resizing (preserve aspect ratio)
- **Output**: Detection tensor `[1, 4+num_classes, 8400]`
  - First 4 features: `[x_center, y_center, width, height]` (absolute pixel coordinates)
  - Remaining num_classes features: class probabilities (0-1)
- **Format**: ONNX opset 11 or higher
- **Quantization**: FP32 or FP16 (FP16 for faster inference on supported hardware)

### Class Names File Format:
```
Nike
Apple
Coca-Cola
McDonald's
Starbucks
...
```
- One brand/logo name per line
- Order must match model class IDs (line 0 = class 0, line 1 = class 1, etc.)
- No header, no empty lines (except trailing)
- UTF-8 encoding

## Available Logo Datasets

### 1. **LogoDet-3K** (3,000 logo classes)
- **Dataset**: https://github.com/Wangjing1551/LogoDet-3K-Dataset
- **Classes**: 3,000 brand logos from real-world scenarios
- **Training**: Requires custom YOLOv8 training on LogoDet-3K annotations
- **License**: Research use (check dataset license)

### 2. **FlickrLogos-32** (32 common brands)
- **Dataset**: https://www.uni-augsburg.de/en/fakultaet/fai/informatik/prof/mmc/research/datensatze/flickrlogos/
- **Classes**: 32 popular brands (Adidas, Apple, BMW, Coca-Cola, etc.)
- **Training**: Smaller dataset, easier to train YOLOv8
- **License**: Academic/research use

### 3. **QMUL-OpenLogo** (352 logo classes)
- **Dataset**: https://qmul-openlogo.github.io/
- **Classes**: 352 logo classes with 27,000+ images
- **Training**: Medium-sized dataset, good balance
- **License**: Check dataset license

### 4. **Custom Logo Datasets**
- Train YOLOv8 on your own logo dataset
- Use Roboflow, CVAT, or Label Studio for annotation
- Export annotations in YOLOv8 format

## Training YOLOv8 on Logo Dataset

### Prerequisites
```bash
pip install ultralytics onnx
```

### 1. Prepare Dataset in YOLOv8 Format

Create `dataset.yaml`:
```yaml
path: /path/to/logo-dataset
train: images/train
val: images/val
nc: 100  # number of logo classes
names: ['Nike', 'Apple', 'Coca-Cola', ...]  # class names
```

### 2. Train YOLOv8 Model

```python
from ultralytics import YOLO

# Load pretrained YOLOv8 model (start from COCO weights)
model = YOLO('yolov8n.pt')  # or yolov8s.pt, yolov8m.pt, etc.

# Train on logo dataset
results = model.train(
    data='dataset.yaml',
    epochs=100,
    imgsz=640,
    batch=16,
    device=0  # GPU device (or 'cpu')
)

# Export to ONNX
model.export(format='onnx', opset=12, simplify=True)
```

### 3. Extract Class Names to logos.txt

```python
# Extract class names from dataset.yaml
import yaml

with open('dataset.yaml', 'r') as f:
    data = yaml.safe_load(f)
    class_names = data['names']

# Write to logos.txt (one per line)
with open('logos.txt', 'w') as f:
    for name in class_names:
        f.write(f"{name}\n")
```

### 4. Move Files to models/logo-detection/

```bash
mv runs/detect/train/weights/best.onnx models/logo-detection/yolov8_logo.onnx
mv logos.txt models/logo-detection/logos.txt
```

## Using Pre-trained Models

### Option 1: Use Existing YOLOv8 Logo Models

Search for pre-trained YOLOv8 logo detection models:
- Hugging Face Hub: https://huggingface.co/models?search=yolov8+logo
- GitHub repositories with pre-trained logo detectors
- Roboflow Universe: https://universe.roboflow.com/ (search "logo detection")

Download ONNX model and class names file, then:
```bash
# Place model in models/logo-detection/
mv downloaded_model.onnx models/logo-detection/yolov8_logo.onnx
mv class_names.txt models/logo-detection/logos.txt
```

### Option 2: Convert PyTorch to ONNX

If you have a PyTorch YOLOv8 logo model:
```python
from ultralytics import YOLO

# Load your trained model
model = YOLO('path/to/best.pt')

# Export to ONNX
model.export(
    format='onnx',
    opset=12,
    simplify=True,
    dynamic=False,  # Static shapes for better optimization
    imgsz=640
)
```

## Model Size Recommendations

| Model Size | File Size | Speed | Accuracy | Use Case |
|------------|-----------|-------|----------|----------|
| YOLOv8n    | ~6 MB     | Fastest | Good | Real-time, mobile |
| YOLOv8s    | ~22 MB    | Fast | Better | General purpose |
| YOLOv8m    | ~52 MB    | Medium | High | High accuracy needed |
| YOLOv8l    | ~87 MB    | Slow | Very High | Offline processing |
| YOLOv8x    | ~136 MB   | Slowest | Highest | Maximum accuracy |

**Recommendation**: Start with **YOLOv8s** (Small) - good balance of speed and accuracy.

## License Considerations

**IMPORTANT**: Logo detection models involve intellectual property (brand logos). Usage restrictions may apply:

1. **Training Data**: Check logo dataset license (academic, commercial, research-only)
2. **Model Weights**: Derived from training data, inherits dataset license restrictions
3. **Deployment**: Some brands prohibit unauthorized logo detection/analysis
4. **Commercial Use**: May require permission from brand owners for commercial applications

**Recommended Use Cases** (generally acceptable):
- Academic research
- Content moderation (removing counterfeit/unauthorized logos)
- Brand monitoring (with brand authorization)
- Advertising analysis (aggregate statistics)

**Use Cases Requiring Legal Review**:
- Commercial logo recognition services
- Competitive analysis without authorization
- Trademark enforcement tools

**This repository does NOT include logo detection models**. Users must provide their own models and ensure compliance with applicable licenses and intellectual property laws.

## Testing Your Model

Once you have both files in place:

```bash
# Test logo detection on a single image
cargo run --release --bin video-extract -- \
  --file test_images/brands.jpg \
  --operation logo-detection \
  --confidence 0.35

# Expected output: JSON with detected logos, bounding boxes, confidence scores
```

## Troubleshooting

### Error: "Logo detection model not found"
- Ensure `yolov8_logo.onnx` exists in `models/logo-detection/`
- Check file name (must be exactly `yolov8_logo.onnx`)
- Verify file is valid ONNX format (not corrupted)

### Error: "Logo class names file not found"
- Ensure `logos.txt` exists in `models/logo-detection/`
- Check file name (must be exactly `logos.txt`)
- Verify file has one class name per line, UTF-8 encoded

### Error: "Expected X features, got Y"
- Class count in `logos.txt` doesn't match model output
- Model outputs `[1, 4+num_classes, 8400]`, check num_classes
- Verify `logos.txt` has correct number of classes (count lines)

### Low Detection Accuracy
- Try lower confidence threshold (0.20-0.30)
- Ensure input images have good resolution (logos visible)
- Verify model was trained on similar logo types/sizes
- Consider using larger model (YOLOv8m or YOLOv8l)

## Support

For model-related questions:
- YOLOv8 Documentation: https://docs.ultralytics.com/
- ONNX Runtime: https://onnxruntime.ai/docs/
- Logo Detection Plugin: See `crates/logo-detection/src/lib.rs`
