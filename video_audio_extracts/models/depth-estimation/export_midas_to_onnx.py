#!/usr/bin/env python3
"""
Export MiDaS v3.1 Small depth estimation model to ONNX format.

This is a ONE-TIME SETUP script (not a runtime dependency).
Python is used only for model export, not for inference.

Usage:
    pip install torch torchvision
    python export_midas_to_onnx.py
"""

import torch
import sys

def export_midas_small():
    """Export MiDaS v3.1 Small model to ONNX format."""

    print("Loading MiDaS v3.1 Small model from torch.hub...")
    try:
        model = torch.hub.load("intel-isl/MiDaS", "MiDaS_small", pretrained=True)
    except Exception as e:
        print(f"Error loading model: {e}")
        print("\nTroubleshooting:")
        print("1. Check internet connection (downloads ~15MB)")
        print("2. Try: pip install --upgrade torch torchvision")
        print("3. Clear torch hub cache: rm -rf ~/.cache/torch/hub/")
        sys.exit(1)

    model.eval()
    print("✓ Model loaded successfully")

    # Create dummy input (256x256 RGB image)
    dummy_input = torch.randn(1, 3, 256, 256)
    print(f"✓ Created dummy input: shape={dummy_input.shape}")

    # Export to ONNX
    output_path = "midas_v3_small.onnx"
    print(f"\nExporting to ONNX: {output_path}...")

    try:
        torch.onnx.export(
            model,
            dummy_input,
            output_path,
            input_names=["input"],
            output_names=["output"],
            dynamic_axes={
                "input": {0: "batch_size"},
                "output": {0: "batch_size"}
            },
            opset_version=14,
            export_params=True,
            do_constant_folding=True,
        )
    except Exception as e:
        print(f"Error exporting to ONNX: {e}")
        sys.exit(1)

    print(f"✓ ONNX export successful: {output_path}")

    # Verify ONNX model
    try:
        import onnx
        onnx_model = onnx.load(output_path)
        onnx.checker.check_model(onnx_model)

        # Get model info
        import os
        size_mb = os.path.getsize(output_path) / (1024 * 1024)
        print(f"\n✓ ONNX model verification passed")
        print(f"  - File size: {size_mb:.1f} MB")
        print(f"  - Input: {onnx_model.graph.input[0].name}, shape: [1, 3, 256, 256]")
        print(f"  - Output: {onnx_model.graph.output[0].name}")
    except ImportError:
        print("\n✓ Export complete (install 'onnx' package to verify)")
    except Exception as e:
        print(f"\n⚠ Warning: ONNX verification failed: {e}")
        print("  Model may still work, but verification recommended")

    print(f"\n✅ SUCCESS: {output_path} ready for use")
    print(f"Move to models/depth-estimation/ directory to use with video-extract")

if __name__ == "__main__":
    print("=" * 60)
    print("MiDaS v3.1 Small → ONNX Export Script")
    print("=" * 60)
    print()

    export_midas_small()
