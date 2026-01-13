#!/usr/bin/env python3
"""
Export emotion detection model to ONNX format.

This script creates a ResNet18-based emotion detection model
for 7 emotion classes: angry, disgust, fear, happy, sad, surprise, neutral.

Model architecture:
- Input: 48x48 grayscale images
- Backbone: ResNet18 (modified for grayscale input)
- Output: 7 emotion classes

Note: This exports a model with random weights for structure only.
In production, you would load pre-trained weights from FER2013 or AffectNet dataset.
"""

import torch
import torch.nn as nn
from torchvision import models

class EmotionDetectionModel(nn.Module):
    """ResNet18-based emotion detection model."""

    def __init__(self, num_classes=7):
        super(EmotionDetectionModel, self).__init__()

        # Use ResNet18 as backbone
        self.backbone = models.resnet18(weights=None)

        # Modify first conv layer for grayscale input (1 channel instead of 3)
        self.backbone.conv1 = nn.Conv2d(1, 64, kernel_size=7, stride=2, padding=3, bias=False)

        # Modify final layer for 7 emotion classes
        num_features = self.backbone.fc.in_features
        self.backbone.fc = nn.Linear(num_features, num_classes)

    def forward(self, x):
        return self.backbone(x)

def export_model():
    """Export emotion detection model to ONNX format."""

    # Create model
    model = EmotionDetectionModel(num_classes=7)
    model.eval()

    # Create dummy input (batch_size=1, channels=1, height=48, width=48)
    dummy_input = torch.randn(1, 1, 48, 48)

    # Export to ONNX
    output_path = "emotion_resnet18.onnx"
    torch.onnx.export(
        model,
        dummy_input,
        output_path,
        export_params=True,
        opset_version=17,
        do_constant_folding=True,
        input_names=['input'],
        output_names=['output'],
        dynamic_axes={
            'input': {0: 'batch_size'},
            'output': {0: 'batch_size'}
        }
    )

    print(f"✅ Model exported to {output_path}")
    print(f"   Input shape: [batch_size, 1, 48, 48]")
    print(f"   Output shape: [batch_size, 7]")
    print(f"   Classes: angry, disgust, fear, happy, sad, surprise, neutral")

    # Check file size
    import os
    size_mb = os.path.getsize(output_path) / (1024 * 1024)
    print(f"   Model size: {size_mb:.1f} MB")

if __name__ == "__main__":
    export_model()
