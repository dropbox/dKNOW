#!/usr/bin/env python3
"""
Simple ONNX export using TF native converter.

Skip SavedModel intermediate step and convert directly to ONNX.
"""

import sys
import os
import numpy as np
from pathlib import Path

# Add idealo src to path
sys.path.insert(0, str(Path(__file__).parent / 'temp_idealo/src'))

try:
    from handlers.model_builder import Nima
except ImportError:
    print("ERROR: Could not import idealo NIMA model_builder")
    print("Make sure temp_idealo repository is cloned")
    sys.exit(1)


def main():
    """
    Load NIMA model and export to ONNX.
    """
    # Paths
    script_dir = Path(__file__).parent
    weights_path = script_dir / "temp_idealo/models/MobileNet/weights_mobilenet_aesthetic_0.07.hdf5"
    h5_path = script_dir / "nima_model.h5"
    onnx_path = script_dir / "nima_mobilenetv2.onnx"

    # Check if weights exist
    if not weights_path.exists():
        print(f"ERROR: Weights not found at {weights_path}")
        sys.exit(1)

    print("Building NIMA model...")
    nima = Nima('MobileNet', weights=None)
    nima.build()

    print(f"Loading weights from {weights_path}...")
    nima.nima_model.load_weights(str(weights_path))

    # Test model
    print("\nTesting model...")
    test_image = np.random.rand(1, 224, 224, 3).astype(np.float32)
    test_image = (test_image * 2) - 1  # MobileNet preprocessing
    output = nima.nima_model.predict(test_image, verbose=0)

    print(f"Output distribution: {output[0]}")
    mean_score = np.sum(output[0] * np.arange(1, 11))
    print(f"Mean score: {mean_score:.4f}")

    # Check if uniform
    uniform_dist = np.full(10, 0.1)
    is_uniform = np.allclose(output[0], uniform_dist, atol=0.05)

    if is_uniform:
        print("WARNING: Output is uniform!")
    else:
        print("✓ Output is non-uniform (weights loaded correctly)")

    # Save as .h5
    print(f"\nSaving model to {h5_path}...")
    nima.nima_model.save(str(h5_path))

    # Convert to ONNX using command line
    print(f"\nConverting to ONNX...")
    import subprocess

    # Use tf2onnx command line tool
    cmd = [
        sys.executable, '-m', 'tf2onnx.convert',
        '--keras', str(h5_path),
        '--output', str(onnx_path),
        '--opset', '17',
        '--verbose'
    ]

    print(f"Running: {' '.join(cmd)}")
    result = subprocess.run(cmd, capture_output=True, text=True)

    if result.returncode != 0:
        print(f"ERROR: Conversion failed")
        print(f"STDOUT: {result.stdout}")
        print(f"STDERR: {result.stderr}")
        sys.exit(1)

    print(result.stdout[-500:])  # Print last 500 chars to avoid too much output

    # Check ONNX file
    if onnx_path.exists():
        size_mb = onnx_path.stat().st_size / (1024 * 1024)
        print(f"\n✓ ONNX model exported successfully: {size_mb:.1f} MB")
        print(f"Location: {onnx_path}")
    else:
        print("\n✗ ONNX file not created")
        sys.exit(1)


if __name__ == "__main__":
    main()
