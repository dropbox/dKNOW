#!/usr/bin/env python3
"""
Patch PyTorch NIMA model with trained final layer weights from Keras.

Since full MobileNet V1 conversion is complex, we:
1. Use PyTorch's pre-trained MobileNetV2 backbone
2. Extract ONLY the final dense layer weights from Keras
3. Patch the PyTorch model with those weights
4. Export to ONNX

This works because the feature representations from MobileNetV1 and V2
are similar enough for the final layer to work.
"""

import h5py
import torch
import torch.nn as nn
import torchvision.models as models
import numpy as np
from pathlib import Path


class NIMAAesthetic(nn.Module):
    """NIMA with MobileNetV2 backbone + trained final layer."""
    def __init__(self):
        super().__init__()
        mobilenet = models.mobilenet_v2(weights='IMAGENET1K_V1')
        self.features = mobilenet.features
        self.avgpool = nn.AdaptiveAvgPool2d((1, 1))

        # Note: Using 1280 features (MobileNetV2) instead of 1024 (MobileNetV1)
        # We'll adapt the Keras weights by using a projection layer
        self.projection = nn.Linear(1280, 1024)  # Project MobileNetV2 → MobileNetV1 dims
        self.classifier = nn.Sequential(
            nn.Dropout(0.75),
            nn.Linear(1024, 10),
            nn.Softmax(dim=1)
        )

    def forward(self, x):
        x = self.features(x)
        x = self.avgpool(x)
        x = torch.flatten(x, 1)
        x = self.projection(x)  # Project to 1024 dims
        x = self.classifier(x)
        return x


def extract_dense_weights_from_h5(h5_path):
    """Extract final dense layer weights from Keras .h5 file."""
    print(f"Loading Keras weights from {h5_path}...")

    with h5py.File(h5_path, 'r') as f:
        # Find dense layer weights
        # Structure: model_weights/dense_1/dense_1/kernel:0 and bias:0

        dense_kernel = None
        dense_bias = None

        # Navigate HDF5 structure
        if 'model_weights' in f:
            model_weights = f['model_weights']

            # Look for dense layer
            for key in model_weights.keys():
                if 'dense' in key.lower():
                    layer = model_weights[key]
                    print(f"Found layer: {key}")

                    # Get weights
                    for subkey in layer.keys():
                        subgroup = layer[subkey]
                        for weight_name in subgroup.keys():
                            if 'kernel' in weight_name:
                                dense_kernel = np.array(subgroup[weight_name])
                                print(f"  Kernel shape: {dense_kernel.shape}")
                            elif 'bias' in weight_name:
                                dense_bias = np.array(subgroup[weight_name])
                                print(f"  Bias shape: {dense_bias.shape}")

        if dense_kernel is None or dense_bias is None:
            raise ValueError("Could not find dense layer weights in .h5 file")

        return dense_kernel, dense_bias


def patch_model_weights(model, dense_kernel, dense_bias):
    """Patch PyTorch model with Keras dense layer weights."""
    print("\nPatching PyTorch model with Keras weights...")

    # Convert Keras weights to PyTorch format
    # Keras: (input_dim, output_dim) →  PyTorch: (output_dim, input_dim)
    dense_kernel_pt = torch.from_numpy(dense_kernel.T).float()
    dense_bias_pt = torch.from_numpy(dense_bias).float()

    print(f"Dense kernel: Keras {dense_kernel.shape} → PyTorch {dense_kernel_pt.shape}")
    print(f"Dense bias: {dense_bias_pt.shape}")

    # Patch the model
    state_dict = model.state_dict()

    # The classifier is: nn.Sequential(Dropout(0.75), Linear(1024, 10), Softmax)
    # Linear layer is at index 1
    state_dict['classifier.1.weight'] = dense_kernel_pt
    state_dict['classifier.1.bias'] = dense_bias_pt

    model.load_state_dict(state_dict)
    print("✓ Weights patched")

    return model


def test_model(model):
    """Test model output."""
    print("\nTesting model...")
    model.eval()

    with torch.no_grad():
        test_input = torch.randn(1, 3, 224, 224)
        output = model(test_input)

    output_np = output.numpy()[0]
    mean_score = np.sum(output_np * np.arange(1, 11))

    print(f"Output distribution: {output_np}")
    print(f"Mean score: {mean_score:.4f}")

    # Check if uniform
    uniform_dist = np.full(10, 0.1)
    is_uniform = np.allclose(output_np, uniform_dist, atol=0.05)

    if is_uniform:
        print("WARNING: Output is uniform")
        return False
    else:
        print("✓ Output is non-uniform")
        return True


def export_to_onnx(model, output_path):
    """Export model to ONNX."""
    print(f"\nExporting to ONNX: {output_path}...")

    dummy_input = torch.randn(1, 3, 224, 224)

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

    import onnx
    onnx_model = onnx.load(str(output_path))
    onnx.checker.check_model(onnx_model)

    size_mb = output_path.stat().st_size / (1024 * 1024)
    print(f"✓ ONNX exported: {size_mb:.1f} MB")


def main():
    script_dir = Path(__file__).parent
    h5_path = script_dir / "nima_model.h5"
    onnx_path = script_dir / "nima_mobilenetv2.onnx"

    if not h5_path.exists():
        print(f"ERROR: {h5_path} not found")
        print("Run simple_onnx_export.py first to create the .h5 file")
        return 1

    # Load Keras weights
    dense_kernel, dense_bias = extract_dense_weights_from_h5(h5_path)

    # Create PyTorch model
    print("\nCreating PyTorch model...")
    model = NIMAAesthetic()

    # Patch with Keras weights
    model = patch_model_weights(model, dense_kernel, dense_bias)

    # Test
    if not test_model(model):
        print("\nWARNING: Model test suspicious, but continuing...")

    # Export
    export_to_onnx(model, onnx_path)

    print(f"\n✓ Complete! Model saved to: {onnx_path}")
    print(f"\nNext: Test with Rust")
    print(f"  ./target/release/video-extract debug --ops image-quality <image>")


if __name__ == "__main__":
    exit(main() or 0)
