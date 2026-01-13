#!/usr/bin/env python3
"""
Export NIMA Keras model directly to ONNX.

This script loads the pre-trained Keras model from idealo and exports to ONNX.
Uses the simpler approach of loading the .hdf5 weights into a Keras model.
"""

import sys
import numpy as np
from pathlib import Path

try:
    import tensorflow as tf
    from tensorflow.keras.models import Model
    from tensorflow.keras.layers import Dropout, Dense
    from tensorflow.keras.applications import MobileNet
except ImportError:
    print("ERROR: TensorFlow not installed.")
    print("Install with: pip install tensorflow")
    sys.exit(1)


def build_and_load_model(weights_path):
    """
    Build NIMA model and load pre-trained weights.
    """
    print(f"Building NIMA model...")

    # Build model (matches idealo architecture exactly)
    base_model = MobileNet(
        input_shape=(224, 224, 3),
        include_top=False,
        weights=None,  # We'll load our own weights
        pooling='avg'
    )

    # Add NIMA head
    x = Dropout(0.75)(base_model.output)
    x = Dense(units=10, activation='softmax', name='dense_1')(x)

    model = Model(base_model.inputs, x)

    # Load pre-trained weights
    print(f"Loading weights from {weights_path}...")
    model.load_weights(str(weights_path))

    print("✓ Model built and weights loaded")
    return model


def test_model(model):
    """
    Test that model produces non-uniform distribution.
    """
    print("\nTesting model output...")

    # Create random test image (preprocessed for MobileNet)
    test_image = np.random.rand(1, 224, 224, 3).astype(np.float32)
    test_image = (test_image * 2) - 1  # MobileNet expects [-1, 1]

    # Run inference
    output = model.predict(test_image, verbose=0)

    output_np = output[0]
    mean_score = np.sum(output_np * np.arange(1, 11))

    print(f"Output distribution: {output_np}")
    print(f"Mean score: {mean_score:.4f}")

    # Check if distribution is uniform
    uniform_dist = np.full(10, 0.1)
    is_uniform = np.allclose(output_np, uniform_dist, atol=0.05)

    if is_uniform:
        print("WARNING: Model output is uniform! Weights may not be loaded correctly.")
        return False
    else:
        print("✓ Model output is non-uniform (weights loaded correctly)")
        return True


def export_to_saved_model(model, output_dir):
    """
    Export model to SavedModel format first.
    """
    print(f"\nExporting to SavedModel format: {output_dir}...")
    model.export(str(output_dir))
    print("✓ SavedModel exported")
    return output_dir


def convert_saved_model_to_onnx(saved_model_path, onnx_path):
    """
    Convert SavedModel to ONNX using python -m tf2onnx.convert command.
    """
    import subprocess

    print(f"\nConverting SavedModel to ONNX: {onnx_path}...")

    cmd = [
        sys.executable, '-m', 'tf2onnx.convert',
        '--saved-model', str(saved_model_path),
        '--output', str(onnx_path),
        '--opset', '17'
    ]

    result = subprocess.run(cmd, capture_output=True, text=True)

    if result.returncode != 0:
        print(f"ERROR: Conversion failed")
        print(f"STDOUT: {result.stdout}")
        print(f"STDERR: {result.stderr}")
        return False

    print(result.stdout)

    # Get model size
    size_mb = onnx_path.stat().st_size / (1024 * 1024)
    print(f"✓ ONNX model exported: {size_mb:.1f} MB")

    return True


def main():
    """
    Main export pipeline.
    """
    # Paths
    script_dir = Path(__file__).parent
    weights_path = script_dir / "temp_idealo/models/MobileNet/weights_mobilenet_aesthetic_0.07.hdf5"
    saved_model_dir = script_dir / "nima_saved_model"
    onnx_path = script_dir / "nima_mobilenetv2.onnx"

    # Check if weights exist
    if not weights_path.exists():
        print(f"ERROR: Pre-trained weights not found at {weights_path}")
        print("\nPlease run: git clone https://github.com/idealo/image-quality-assessment.git temp_idealo")
        sys.exit(1)

    # Build model and load weights
    model = build_and_load_model(weights_path)

    # Test model
    test_model(model)

    # Export to SavedModel
    export_to_saved_model(model, saved_model_dir)

    # Convert to ONNX
    success = convert_saved_model_to_onnx(saved_model_dir, onnx_path)

    if success:
        print("\n✓ Export complete!")
        print(f"\nONNX model: {onnx_path}")
        print(f"\nNext steps:")
        print(f"1. Test: ./target/release/video-extract debug --ops image-quality <image>")
        print(f"2. Verify non-uniform scores on different images")
    else:
        print("\n✗ Export failed")
        sys.exit(1)


if __name__ == "__main__":
    main()
