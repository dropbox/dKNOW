#!/usr/bin/env python3
"""
Export CAM++ speaker embedding model to ONNX format.

Downloads PyTorch model from ModelScope and exports to ONNX.
This script is run ONCE to create the ONNX model file.
After export, Python is NOT needed for inference (uses ONNX Runtime in Rust).
"""

import sys
import os
import torch
import torch.nn as nn


def export_campplus_to_onnx(model_bin_path, output_path="speaker_embedding.onnx"):
    """Export CAM++ model from .bin to ONNX format."""

    print(f"Loading CAM++ model from: {model_bin_path}")

    # Load the model state dict
    checkpoint = torch.load(model_bin_path, map_location='cpu')

    print(f"Checkpoint keys: {checkpoint.keys() if isinstance(checkpoint, dict) else 'raw tensor'}")

    # The model needs to be loaded with 3D-Speaker's model architecture
    # Since we don't have the full 3D-Speaker library, we'll use the ModelScope pipeline
    try:
        from modelscope.pipelines import pipeline
        from modelscope.utils.constant import Tasks

        print("Loading model via ModelScope pipeline...")
        sv_pipeline = pipeline(
            task=Tasks.speaker_verification,
            model='iic/speech_campplus_sv_en_voxceleb_16k'
        )

        # Get the embedding model from the pipeline
        model = sv_pipeline.model
        model.eval()

        print(f"Model loaded successfully")
        print(f"Model type: {type(model)}")

        # Create dummy input - CAM++ expects fbank features
        # Input shape: [batch, time, feat_dim] = [1, 200, 80]
        # 200 frames ≈ 2 seconds of audio at 10ms frame shift
        batch_size = 1
        num_frames = 200
        feat_dim = 80
        dummy_input = torch.randn(batch_size, num_frames, feat_dim)

        print(f"Dummy input shape: {dummy_input.shape}")

        # Test forward pass
        with torch.no_grad():
            test_output = model(dummy_input)
            if isinstance(test_output, tuple):
                test_output = test_output[0]
            print(f"Output shape: {test_output.shape}")
            print(f"Embedding dimension: {test_output.shape[-1]}")

        # Export to ONNX
        print(f"\nExporting to ONNX: {output_path}")

        torch.onnx.export(
            model,
            dummy_input,
            output_path,
            export_params=True,
            opset_version=14,
            do_constant_folding=True,
            input_names=['fbank'],
            output_names=['embedding'],
            dynamic_axes={
                'fbank': {0: 'batch_size', 1: 'num_frames'},
                'embedding': {0: 'batch_size'}
            }
        )

        print(f"✅ ONNX export successful!")
        print(f"   Model saved to: {output_path}")
        print(f"   Input: [batch_size, num_frames, 80] - fbank features")
        print(f"   Output: [batch_size, 512] - speaker embeddings")
        print(f"\nYou can now delete this Python script and use pure Rust inference!")

        return output_path

    except Exception as e:
        print(f"Error loading model via ModelScope: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)


def verify_onnx_export(onnx_path):
    """Verify the exported ONNX model works."""
    import onnxruntime as ort
    import numpy as np

    print(f"\nVerifying ONNX model: {onnx_path}")

    # Load ONNX model
    session = ort.InferenceSession(onnx_path)

    # Get input/output info
    print(f"\nModel inputs:")
    for input_info in session.get_inputs():
        print(f"  - {input_info.name}: shape={input_info.shape}, type={input_info.type}")

    print(f"\nModel outputs:")
    for output_info in session.get_outputs():
        print(f"  - {output_info.name}: shape={output_info.shape}, type={output_info.type}")

    # Test inference
    input_name = session.get_inputs()[0].name
    test_fbank = np.random.randn(1, 200, 80).astype(np.float32)
    outputs = session.run(None, {input_name: test_fbank})

    print(f"\nTest inference:")
    print(f"  Input shape: {test_fbank.shape}")
    print(f"  Output shape: {outputs[0].shape}")
    print(f"  Embedding dimension: {outputs[0].shape[-1]}")
    print(f"  ✅ ONNX model verification successful!")


if __name__ == "__main__":
    try:
        # Default: use cached ModelScope model
        model_cache = os.path.expanduser("~/.cache/modelscope/hub/models/iic/speech_campplus_sv_en_voxceleb_16k")
        model_bin = os.path.join(model_cache, "campplus_voxceleb.bin")

        if not os.path.exists(model_bin):
            print(f"Error: Model not found at {model_bin}")
            print("Run download_wespeaker_onnx.py first to download the model")
            sys.exit(1)

        output_path = "speaker_embedding.onnx"
        if len(sys.argv) > 1:
            output_path = sys.argv[1]

        print("=" * 70)
        print("CAM++ Speaker Embedding ONNX Exporter")
        print("=" * 70)

        # Export model
        onnx_path = export_campplus_to_onnx(model_bin, output_path)

        # Verify export
        try:
            verify_onnx_export(onnx_path)
        except ImportError:
            print("\nNote: onnxruntime not installed, skipping verification")
            print("Install with: pip install onnxruntime")

        print("\n" + "=" * 70)
        print("EXPORT COMPLETE - Python no longer needed for inference!")
        print("=" * 70)
        print(f"\nModel ready at: {output_path}")
        print("Use with Rust ONNX Runtime for speaker embedding inference.")

    except Exception as e:
        print(f"❌ Export failed: {e}", file=sys.stderr)
        import traceback
        traceback.print_exc()
        print("\nMake sure you have required dependencies:")
        print("  pip install modelscope torch")
        sys.exit(1)
