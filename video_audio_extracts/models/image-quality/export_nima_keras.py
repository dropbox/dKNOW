#!/usr/bin/env python3
"""
Export NIMA (Neural Image Assessment) model to ONNX format from Keras weights.

This script loads pre-trained NIMA MobileNet weights from the idealo repository
and exports them to ONNX format for use in the Rust inference engine.
"""

import sys
import numpy as np
from pathlib import Path

try:
    import tensorflow as tf
    from tensorflow.keras.models import Model
    from tensorflow.keras.layers import Dropout, Dense
    import tf2onnx
except ImportError:
    print("ERROR: Required packages not installed.")
    print("Install with: pip install tensorflow tf2onnx")
    sys.exit(1)


def build_nima_model(weights_path, dropout_rate=0.75):
    """
    Build NIMA model with MobileNet backbone and load pre-trained weights.

    Architecture matches idealo implementation:
    - MobileNet backbone (ImageNet pre-trained)
    - Average pooling
    - Dropout (0.75)
    - Dense(10) with softmax (quality score distribution)
    """
    print(f"Building NIMA model with weights from {weights_path}...")

    # Load MobileNet backbone (same as idealo uses)
    base_model = tf.keras.applications.MobileNet(
        input_shape=(224, 224, 3),
        include_top=False,
        weights='imagenet',
        pooling='avg'
    )

    # Add NIMA head
    x = Dropout(dropout_rate)(base_model.output)
    x = Dense(units=10, activation='softmax', name='predictions')(x)

    model = Model(base_model.inputs, x)

    # Load pre-trained NIMA weights
    print(f"Loading pre-trained weights from {weights_path}...")
    model.load_weights(str(weights_path))

    return model


def test_model_output(model):
    """
    Test that model produces non-uniform distribution (not random weights).
    """
    print("\nTesting model output distribution...")

    # Create random test image
    test_image = np.random.rand(1, 224, 224, 3).astype(np.float32)
    test_image = tf.keras.applications.mobilenet.preprocess_input(test_image)

    # Run inference
    output = model.predict(test_image, verbose=0)

    print(f"Output distribution: {output[0]}")
    print(f"Mean score: {np.sum(output[0] * np.arange(1, 11)):.4f}")

    # Check if distribution is uniform (indicating untrained weights)
    uniform_dist = np.full(10, 0.1)
    is_uniform = np.allclose(output[0], uniform_dist, atol=0.05)

    if is_uniform:
        print("WARNING: Model output is uniform! Weights may not be loaded correctly.")
        return False
    else:
        print("✓ Model output is non-uniform (weights loaded correctly)")
        return True


def export_to_onnx(model, output_path):
    """
    Export Keras model to ONNX format.
    """
    print(f"\nExporting to ONNX: {output_path}...")

    # Convert to ONNX
    input_signature = [tf.TensorSpec([1, 224, 224, 3], tf.float32, name='image')]

    onnx_model, _ = tf2onnx.convert.from_keras(
        model,
        input_signature=input_signature,
        opset=17,
        output_path=str(output_path)
    )

    # Get model size
    size_mb = output_path.stat().st_size / (1024 * 1024)
    print(f"✓ ONNX model exported successfully: {size_mb:.1f} MB")
    print(f"  Input: [batch, 224, 224, 3] (RGB image, MobileNet preprocessed)")
    print(f"  Output: [batch, 10] (probability distribution over scores 1-10)")
    print(f"  Usage: mean_score = sum(scores[i] * (i+1) for i in range(10))")

    return output_path


def main():
    """
    Main export pipeline.
    """
    # Paths
    script_dir = Path(__file__).parent
    weights_path = script_dir / "temp_idealo/models/MobileNet/weights_mobilenet_aesthetic_0.07.hdf5"
    output_path = script_dir / "nima_mobilenetv2.onnx"

    # Check if weights exist
    if not weights_path.exists():
        print(f"ERROR: Pre-trained weights not found at {weights_path}")
        print("\nPlease download weights first:")
        print("  git clone https://github.com/idealo/image-quality-assessment.git temp_idealo")
        sys.exit(1)

    # Build model and load weights
    model = build_nima_model(weights_path)

    # Test model output
    if not test_model_output(model):
        print("\nWARNING: Model test failed, but continuing with export...")

    # Export to ONNX
    export_to_onnx(model, output_path)

    print("\n✓ Export complete!")
    print(f"\nNext steps:")
    print(f"1. Test with Rust inference: ./target/release/video-extract debug --ops image-quality <image>")
    print(f"2. Verify non-uniform scores on different images")
    print(f"3. Run AI verification with GPT-4 Vision")


if __name__ == "__main__":
    main()
