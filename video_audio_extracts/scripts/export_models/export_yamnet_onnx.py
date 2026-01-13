#!/usr/bin/env python3
"""Export YAMNet audio classification model to ONNX format

YAMNet is Google's audio event classification model trained on AudioSet (521 classes).
TensorFlow Hub model: https://tfhub.dev/google/yamnet/1

Requirements:
    pip install tensorflow tensorflow-hub tf2onnx onnx

Input: waveform (float32, shape [N]) - mono audio at 16kHz, variable length
Outputs:
    - scores: [num_patches, 521] - class probabilities per patch (0.96s each)
    - embeddings: [num_patches, 1024] - feature vectors
    - spectrogram: [num_patches, 64] - log mel spectrogram

Usage:
    python3 scripts/export_models/export_yamnet_onnx.py

Output: models/audio-classification/yamnet.onnx (~15 MB)
"""

import sys
import os
import tensorflow as tf
import tensorflow_hub as hub
import tf2onnx
import onnx

def export_yamnet():
    print("Loading YAMNet from TensorFlow Hub...")
    print("URL: https://tfhub.dev/google/yamnet/1")

    # Load YAMNet from TensorFlow Hub
    yamnet_model = hub.load('https://tfhub.dev/google/yamnet/1')

    print("Model loaded successfully")
    print(f"Signature keys: {list(yamnet_model.signatures.keys())}")

    # YAMNet expects mono audio at 16kHz, variable length
    # Use a reasonable fixed length for ONNX export (3 seconds = 48000 samples)
    # Note: Model works with variable length but ONNX export requires fixed shape
    input_shape = (48000,)

    print(f"Using input shape: {input_shape} (3 seconds at 16kHz)")

    # Get the inference function
    inference_fn = yamnet_model.signatures['serving_default']

    # Create output directory if it doesn't exist
    output_dir = "models/audio-classification"
    os.makedirs(output_dir, exist_ok=True)
    output_path = os.path.join(output_dir, "yamnet.onnx")

    print("Converting to ONNX...")

    # Convert to ONNX using tf2onnx
    # Use opset 13 for better compatibility
    model_proto, external_tensor_storage = tf2onnx.convert.from_function(
        inference_fn,
        input_signature=[tf.TensorSpec(shape=input_shape, dtype=tf.float32, name="waveform")],
        opset=13,
        output_path=output_path
    )

    print(f"✅ YAMNet exported successfully: {output_path}")

    # Verify the exported model
    print("\nVerifying ONNX model...")
    onnx_model = onnx.load(output_path)
    onnx.checker.check_model(onnx_model)

    # Print input/output info
    print("\nModel inputs:")
    for input_tensor in onnx_model.graph.input:
        print(f"  - {input_tensor.name}: {input_tensor.type}")

    print("\nModel outputs:")
    for output_tensor in onnx_model.graph.output:
        print(f"  - {output_tensor.name}: {output_tensor.type}")

    # Get file size
    file_size = os.path.getsize(output_path) / (1024 * 1024)
    print(f"\nModel size: {file_size:.2f} MB")

    print("\n✅ Export complete and verified!")
    print(f"Output: {output_path}")

if __name__ == "__main__":
    try:
        export_yamnet()
    except Exception as e:
        print(f"❌ Export failed: {e}", file=sys.stderr)
        import traceback
        traceback.print_exc()
        sys.exit(1)
