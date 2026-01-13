#!/usr/bin/env python3
"""
Export SpeechBrain ECAPA-TDNN speaker embedding model to ONNX format.

This script is run ONCE to create the ONNX model file.
After export, Python is NOT needed for inference (uses ONNX Runtime in Rust).
"""

import torch
import torch.nn as nn
from speechbrain.inference.speaker import EncoderClassifier
import sys
import os


def export_ecapa_to_onnx(output_path="speaker_embedding.onnx"):
    """Export SpeechBrain ECAPA-TDNN model to ONNX format."""

    print("Loading SpeechBrain ECAPA-TDNN model...")
    print("Source: speechbrain/spkrec-ecapa-voxceleb")

    # Load pretrained model
    classifier = EncoderClassifier.from_hparams(
        source="speechbrain/spkrec-ecapa-voxceleb",
        savedir="tmpdir",
        run_opts={"device": "cpu"}  # Export on CPU for compatibility
    )

    print(f"Model loaded successfully")
    print(f"Embedding dimension: 192")

    # Get the embedding model
    embedding_model = classifier.mods['embedding_model']
    embedding_model.eval()

    # Create dummy input
    # Input: audio waveform [batch_size, samples]
    # For 3 seconds at 16kHz = 48000 samples
    batch_size = 1
    num_samples = 48000
    dummy_input = torch.randn(batch_size, num_samples)

    print(f"Dummy input shape: {dummy_input.shape}")

    # Test forward pass
    with torch.no_grad():
        test_output = embedding_model(dummy_input)
        print(f"Output shape: {test_output.shape}")
        print(f"Output embedding dim: {test_output.shape[-1]}")

    # Export to ONNX
    print(f"\nExporting to ONNX: {output_path}")

    torch.onnx.export(
        embedding_model,
        dummy_input,
        output_path,
        export_params=True,
        opset_version=14,
        do_constant_folding=True,
        input_names=['audio'],
        output_names=['embedding'],
        dynamic_axes={
            'audio': {0: 'batch_size', 1: 'samples'},
            'embedding': {0: 'batch_size'}
        }
    )

    print(f"✅ ONNX export successful!")
    print(f"   Model saved to: {output_path}")
    print(f"   Input: [batch_size, samples] - audio samples at 16kHz")
    print(f"   Output: [batch_size, 192] - speaker embeddings")
    print(f"\nYou can now delete this Python script and use pure Rust inference!")

    return output_path


def verify_onnx_export(onnx_path):
    """Verify the exported ONNX model works."""
    import onnxruntime as ort
    import numpy as np

    print(f"\nVerifying ONNX model: {onnx_path}")

    # Load ONNX model
    session = ort.InferenceSession(onnx_path)

    # Get input/output info
    input_info = session.get_inputs()[0]
    output_info = session.get_outputs()[0]

    print(f"  Input: {input_info.name}, shape: {input_info.shape}, type: {input_info.type}")
    print(f"  Output: {output_info.name}, shape: {output_info.shape}, type: {output_info.type}")

    # Test inference
    test_audio = np.random.randn(1, 48000).astype(np.float32)
    outputs = session.run(None, {input_info.name: test_audio})

    print(f"  Test inference output shape: {outputs[0].shape}")
    print(f"  ✅ ONNX model verification successful!")


if __name__ == "__main__":
    try:
        # Export model
        output_path = "speaker_embedding.onnx"
        if len(sys.argv) > 1:
            output_path = sys.argv[1]

        onnx_path = export_ecapa_to_onnx(output_path)

        # Verify export
        try:
            verify_onnx_export(onnx_path)
        except ImportError:
            print("\nNote: onnxruntime not installed, skipping verification")
            print("Install with: pip install onnxruntime")

        print("\n" + "="*70)
        print("EXPORT COMPLETE - Python no longer needed for inference!")
        print("="*70)

    except Exception as e:
        print(f"❌ Export failed: {e}", file=sys.stderr)
        print("\nMake sure you have SpeechBrain installed:")
        print("  pip install speechbrain torch")
        sys.exit(1)
