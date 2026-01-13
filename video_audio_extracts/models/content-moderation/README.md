# Content Moderation Models

This directory contains ONNX models for content moderation (NSFW detection).

## Required Model

**Model**: `nsfw_mobilenet.onnx`
**Type**: Image classification (5 classes: drawings, hentai, neutral, porn, sexy)
**Architecture**: MobileNetV2-based (OpenNSFW2 style)
**Input**: 224x224 RGB image
**Output**: 5-class probability distribution

## Where to Get the Model

### Option 1: ONNX Model Zoo
Download pre-trained NSFW classification models from ONNX Model Zoo or similar repositories.

### Option 2: Export from PyTorch
If you have access to a trained NSFW classifier (e.g., Yahoo OpenNSFW, OpenNSFW2):

```python
import torch
import torch.onnx

# Load pre-trained model (example)
model = torch.load('nsfw_model.pth')
model.eval()

# Export to ONNX
dummy_input = torch.randn(1, 3, 224, 224)
torch.onnx.export(
    model,
    dummy_input,
    'nsfw_mobilenet.onnx',
    input_names=['input'],
    output_names=['output'],
    dynamic_axes={'input': {0: 'batch_size'}, 'output': {0: 'batch_size'}}
)
```

### Option 3: Use Alternative Models
- **Falconsai/nsfw_image_detection** (Hugging Face) - may require ONNX conversion
- **GantMan/nsfw_model** (OpenNSFW) - TensorFlow, needs ONNX conversion
- **notAI-tech/NudeNet** - detector + classifier, needs ONNX export

## Model Specification

The plugin expects the following output format:

**Output shape**: `[batch_size, 5]`
**Output type**: `float32` (softmax probabilities)

**Categories (in order)**:
1. Drawings/illustrations (0)
2. Hentai/anime NSFW (1)
3. Neutral/safe (2)
4. Pornography (3)
5. Sexy/suggestive (4)

**NSFW score**: Computed as `hentai + porn + sexy` (categories 1, 3, 4)

## Testing Without Model

Until you have a trained model, the plugin will fail gracefully with a model loading error:

```
Failed to load content moderation plugin: cannot load model from models/content-moderation/nsfw_mobilenet.onnx
```

This is expected behavior. The plugin code is complete and ready to use once a model is provided.

## License Considerations

NSFW detection models may have specific license restrictions. Ensure compliance with:
- Model training data licenses
- Model architecture licenses (e.g., MobileNetV2 under Apache 2.0)
- Usage restrictions (commercial vs non-commercial)

## References

- **OpenNSFW2**: https://github.com/bhky/opennsfw2 (TensorFlow)
- **Yahoo OpenNSFW**: https://github.com/yahoo/open_nsfw (Caffe)
- **ONNX Model Zoo**: https://github.com/onnx/models
- **Hugging Face**: https://huggingface.co/models?pipeline_tag=image-classification&search=nsfw
