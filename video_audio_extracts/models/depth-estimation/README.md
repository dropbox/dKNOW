# Depth Estimation Models

This directory contains ONNX models for monocular depth estimation (predicting depth from single RGB images).

## Required Files

The depth estimation plugin requires ONNX model files. Place one of the following models in this directory:

### Option 1: MiDaS v3.1 Small (Recommended for Speed)
- **File**: `midas_v3_small.onnx`
- **Input**: RGB image, 256x256 pixels
- **Output**: Depth map, 256x256 pixels (single channel, float32)
- **Size**: ~15 MB
- **Speed**: Fast (~50ms on GPU, ~200ms on CPU)
- **Quality**: Good for most applications

### Option 2: DPT Hybrid (Recommended for Quality)
- **File**: `dpt_hybrid.onnx`
- **Input**: RGB image, 384x384 pixels
- **Output**: Depth map, 384x384 pixels (single channel, float32)
- **Size**: ~400 MB
- **Speed**: Medium (~150ms on GPU, ~800ms on CPU)
- **Quality**: Excellent, balanced quality/speed

### Option 3: DPT Large (Highest Quality)
- **File**: `dpt_large.onnx`
- **Input**: RGB image, 384x384 pixels
- **Output**: Depth map, 384x384 pixels (single channel, float32)
- **Size**: ~1.3 GB
- **Speed**: Slow (~300ms on GPU, ~2s on CPU)
- **Quality**: State-of-the-art, research-grade quality

## Model Architecture

### MiDaS v3.1
- Vision Transformer (ViT) backbone
- Multi-scale depth prediction
- Trained on 12 diverse datasets (indoor, outdoor, synthetic)
- Relative depth estimation (not metric depth)
- Zero-shot generalization across domains

### DPT (Dense Prediction Transformer)
- ViT-Hybrid or ViT-Large backbone
- Dense prediction head for pixel-wise depth
- Trained on MIX 6 dataset (1.4M images)
- Superior performance on complex scenes
- Better handling of fine details and occlusions

## Obtaining Models

### From HuggingFace Hub (Recommended)

```python
# Install dependencies
pip install torch onnx huggingface_hub

# Download and export MiDaS v3.1 Small
from huggingface_hub import hf_hub_download
import torch

# Option 1: MiDaS v3.1 Small (256x256)
model = torch.hub.load("intel-isl/MiDaS", "MiDaS_small", pretrained=True)
model.eval()

# Create dummy input
dummy_input = torch.randn(1, 3, 256, 256)

# Export to ONNX
torch.onnx.export(
    model,
    dummy_input,
    "midas_v3_small.onnx",
    input_names=["input"],
    output_names=["output"],
    dynamic_axes={"input": {0: "batch_size"}, "output": {0: "batch_size"}},
    opset_version=14,
)

# Option 2: DPT Hybrid (384x384)
from transformers import DPTForDepthEstimation
import torch.onnx

model = DPTForDepthEstimation.from_pretrained("Intel/dpt-hybrid-midas")
model.eval()

dummy_input = torch.randn(1, 3, 384, 384)

torch.onnx.export(
    model,
    dummy_input,
    "dpt_hybrid.onnx",
    input_names=["pixel_values"],
    output_names=["predicted_depth"],
    dynamic_axes={
        "pixel_values": {0: "batch_size"},
        "predicted_depth": {0: "batch_size"}
    },
    opset_version=14,
)
```

### From Official MiDaS Repository

```bash
git clone https://github.com/isl-org/MiDaS.git
cd MiDaS

# Download weights
wget https://github.com/isl-org/MiDaS/releases/download/v3_1/dpt_beit_large_512.pt
wget https://github.com/isl-org/MiDaS/releases/download/v3_1/dpt_swin2_tiny_256.pt

# Export to ONNX using the export script (see MiDaS docs)
python export_to_onnx.py --model_type dpt_swin2_tiny_256 --optimize
```

## Model Specifications

### Input Format
- **Shape**: [1, 3, H, W] where H=W=256 or 384 (depending on model)
- **Data Type**: float32
- **Range**: [0, 1] (normalized RGB)
- **Normalization**: ImageNet statistics
  - Mean: [0.485, 0.456, 0.406]
  - Std: [0.229, 0.224, 0.225]
- **Color Order**: RGB (not BGR)

### Output Format
- **Shape**: [1, H, W] or [1, 1, H, W] (depends on export configuration)
- **Data Type**: float32
- **Range**: Arbitrary (relative depth, not metric)
- **Interpretation**: Higher values = farther from camera (or inverse, depends on model)

## Preprocessing

The plugin automatically handles:
1. Resizing input image to model's expected size (256x256 or 384x384)
2. Converting to RGB if grayscale
3. Normalizing with ImageNet statistics
4. Converting to CHW tensor format (channels-first)

## Postprocessing

The plugin provides:
1. **Min/Max/Mean depth statistics** for scene understanding
2. **Normalized depth map** (0-255 grayscale) for visualization
3. **Optional resizing** to original image resolution (bicubic interpolation)

## Use Cases

### 3D Reconstruction
- Estimate depth for structure-from-motion pipelines
- Generate point clouds from single images
- Create 3D models from photo collections

### AR/VR Applications
- Occlusion handling (virtual objects behind real objects)
- Physics simulation (depth-aware interactions)
- Scene understanding for spatial computing

### Cinematography
- Depth-of-field effects (bokeh, focus pulling)
- Scene composition analysis
- Shot planning and storyboarding

### Accessibility
- Object distance estimation for visually impaired
- Navigation assistance
- Obstacle detection

## Performance Expectations

| Model | Resolution | GPU (RTX 3090) | CPU (16-core) | Quality |
|-------|-----------|----------------|---------------|---------|
| MiDaS Small | 256x256 | ~50ms | ~200ms | Good |
| DPT Hybrid | 384x384 | ~150ms | ~800ms | Excellent |
| DPT Large | 384x384 | ~300ms | ~2000ms | State-of-art |

**Batch Processing**: For video keyframes, processing 100 frames:
- MiDaS Small: ~5s (GPU), ~20s (CPU)
- DPT Hybrid: ~15s (GPU), ~80s (CPU)
- DPT Large: ~30s (GPU), ~200s (CPU)

## Limitations

1. **Relative Depth Only**: Models predict relative depth (ordinal depth), not metric depth (actual distances in meters). For metric depth, calibration or scale recovery is required.

2. **Single Image**: Monocular depth estimation is inherently ambiguous. Same 2D image can correspond to multiple 3D scenes. Multi-view stereo provides more accurate depth.

3. **Domain Shift**: Models may perform poorly on domains significantly different from training data (e.g., underwater, microscopy, satellite imagery).

4. **Reflective Surfaces**: Mirrors, glass, and water can confuse depth models due to reflections and transparency.

5. **Textureless Regions**: Large uniform areas (walls, sky) have ambiguous depth. Models may produce smooth but inaccurate depth maps.

## Licensing

### MiDaS Models
- **License**: MIT License
- **Commercial Use**: Allowed
- **Attribution**: Required (cite MiDaS paper)
- **Paper**: [Towards Robust Monocular Depth Estimation: Mixing Datasets for Zero-shot Cross-dataset Transfer](https://arxiv.org/abs/1907.01341)

### DPT Models
- **License**: Apache 2.0
- **Commercial Use**: Allowed
- **Attribution**: Recommended
- **Paper**: [Vision Transformers for Dense Prediction](https://arxiv.org/abs/2103.13413)

### Important Notes
- Models trained on datasets with various licenses (NYU Depth v2, KITTI, etc.)
- Check individual dataset licenses for redistribution restrictions
- Commercial use generally allowed, but verify for your specific use case

## Troubleshooting

### Model Not Found Error
```
Error: Failed to load depth estimation model from models/depth-estimation/midas_v3_small.onnx
```
**Solution**: Download and export model using instructions above. Place in `models/depth-estimation/` directory.

### Shape Mismatch Error
```
Error: Invalid model output shape: expected [1, H, W] or [1, 1, H, W], got [...]
```
**Solution**: Verify ONNX export used correct input/output names and dynamic axes. Re-export with `opset_version=14` or higher.

### Poor Quality Results
- **Symptom**: Depth map looks noisy or incorrect
- **Solutions**:
  1. Try DPT Hybrid or DPT Large for better quality
  2. Ensure input images are well-lit and in-focus
  3. Avoid scenes with mirrors, glass, or reflective surfaces
  4. Check if scene is within model's training domain (indoor/outdoor scenes)

### Slow Performance
- **Symptom**: Processing takes longer than expected
- **Solutions**:
  1. Use MiDaS Small (256x256) instead of DPT models for speed
  2. Enable GPU acceleration (CUDA or CoreML)
  3. Process keyframes in batches (plugin samples every 2 frames by default)
  4. Reduce input resolution (resize images before processing)

## References

1. MiDaS: [https://github.com/isl-org/MiDaS](https://github.com/isl-org/MiDaS)
2. DPT Paper: [https://arxiv.org/abs/2103.13413](https://arxiv.org/abs/2103.13413)
3. HuggingFace Models: [https://huggingface.co/Intel](https://huggingface.co/Intel)
4. ONNX Runtime: [https://onnxruntime.ai/](https://onnxruntime.ai/)
