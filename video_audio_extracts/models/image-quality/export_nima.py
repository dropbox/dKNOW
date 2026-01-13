#!/usr/bin/env python3
"""
Export NIMA (Neural Image Assessment) model to ONNX format.

NIMA uses a CNN backbone (MobileNetV2) to predict aesthetic and technical quality scores.
Output: 10-class distribution (scores 1-10), mean gives final quality score.
"""

import torch
import torch.nn as nn
import torchvision.models as models
import onnx
from pathlib import Path


class NIMAAesthetic(nn.Module):
    """
    NIMA model for aesthetic quality assessment.
    Architecture: MobileNetV2 backbone + FC layers for score distribution.
    """
    def __init__(self):
        super().__init__()
        # Use MobileNetV2 as backbone (lightweight, fast)
        # Note: Using random weights for ONNX export (structure only)
        # In production, this would be replaced with trained weights
        mobilenet = models.mobilenet_v2(weights=None)

        # Remove classifier
        self.features = mobilenet.features
        self.avgpool = nn.AdaptiveAvgPool2d((1, 1))

        # NIMA head: predict distribution over 10 quality scores (1-10)
        self.classifier = nn.Sequential(
            nn.Dropout(0.5),
            nn.Linear(1280, 10),  # MobileNetV2 has 1280 features
            nn.Softmax(dim=1)
        )

    def forward(self, x):
        x = self.features(x)
        x = self.avgpool(x)
        x = torch.flatten(x, 1)
        x = self.classifier(x)
        return x


def export_nima_onnx():
    """Export NIMA model to ONNX format."""
    print("Creating NIMA model...")
    model = NIMAAesthetic()
    model.eval()

    # Create dummy input (batch_size=1, channels=3, height=224, width=224)
    dummy_input = torch.randn(1, 3, 224, 224)

    output_path = Path(__file__).parent / "nima_mobilenetv2.onnx"

    print(f"Exporting to {output_path}...")
    torch.onnx.export(
        model,
        dummy_input,
        str(output_path),
        input_names=['image'],
        output_names=['scores'],
        dynamic_axes={
            'image': {0: 'batch_size'},
            'scores': {0: 'batch_size'}
        },
        opset_version=17,
        do_constant_folding=True
    )

    # Verify ONNX model
    print("Verifying ONNX model...")
    onnx_model = onnx.load(str(output_path))
    onnx.checker.check_model(onnx_model)

    # Get model size
    size_mb = output_path.stat().st_size / (1024 * 1024)
    print(f"✓ ONNX model exported successfully: {size_mb:.1f} MB")
    print(f"  Input: [batch, 3, 224, 224] (RGB image, normalized)")
    print(f"  Output: [batch, 10] (probability distribution over scores 1-10)")
    print(f"  Usage: mean_score = sum(scores[i] * (i+1) for i in range(10))")

    return output_path


if __name__ == "__main__":
    export_nima_onnx()
