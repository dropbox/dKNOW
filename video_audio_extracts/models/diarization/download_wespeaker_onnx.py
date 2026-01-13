#!/usr/bin/env python3
"""
Download pre-exported ONNX speaker embedding model from ModelScope/3D-Speaker.

This script is run ONCE to download the ONNX model file.
After download, Python is NOT needed for inference (uses ONNX Runtime in Rust).
"""

import sys
import os

try:
    from modelscope.hub.snapshot_download import snapshot_download
except ImportError:
    print("Error: modelscope not installed")
    print("Install with: pip install modelscope")
    sys.exit(1)


def download_wespeaker_onnx(model_id="iic/speech_campplus_sv_en_voxceleb_16k", output_path="speaker_embedding.onnx"):
    """
    Download WeSpeaker/3D-Speaker ONNX model from ModelScope.

    Available models:
    - iic/speech_campplus_sv_en_voxceleb_16k (CAM++ VoxCeleb - English)
    - iic/speech_eres2net_sv_en_voxceleb_16k (ERes2Net Base - English)
    - iic/speech_eres2net_large_sv_en_voxceleb_16k (ERes2Net Large - English)
    """

    print(f"Downloading model from ModelScope: {model_id}")
    print("This may take a few minutes...")

    # Download model snapshot
    cache_dir = snapshot_download(model_id)
    print(f"Model downloaded to: {cache_dir}")

    # Find the ONNX file
    onnx_files = []
    for root, dirs, files in os.walk(cache_dir):
        for file in files:
            if file.endswith('.onnx'):
                onnx_files.append(os.path.join(root, file))

    if not onnx_files:
        print(f"Error: No ONNX file found in {cache_dir}")
        print("Available files:")
        for root, dirs, files in os.walk(cache_dir):
            for file in files:
                print(f"  {os.path.join(root, file)}")
        sys.exit(1)

    # Use the first ONNX file found
    source_onnx = onnx_files[0]
    print(f"Found ONNX file: {source_onnx}")

    # Copy to target location
    import shutil
    shutil.copy2(source_onnx, output_path)

    print(f"\n✅ Model downloaded successfully!")
    print(f"   Saved to: {output_path}")

    # Try to verify
    try:
        verify_onnx_model(output_path)
    except ImportError:
        print("\nNote: onnxruntime not installed, skipping verification")
        print("Install with: pip install onnxruntime")

    return output_path


def verify_onnx_model(onnx_path):
    """Verify the ONNX model structure."""
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

    # Test inference with dummy input
    input_name = session.get_inputs()[0].name
    input_shape = session.get_inputs()[0].shape

    # Create dummy input (assuming audio input)
    # CAM++ expects fbank features: [batch, time, feat_dim]
    # Typically [1, 200, 80] for 2 seconds of audio
    if len(input_shape) == 3:
        test_input = np.random.randn(1, 200, 80).astype(np.float32)
    else:
        print(f"Warning: Unexpected input shape {input_shape}")
        test_input = np.random.randn(1, 48000).astype(np.float32)

    outputs = session.run(None, {input_name: test_input})

    print(f"\nTest inference:")
    print(f"  Input shape: {test_input.shape}")
    print(f"  Output shape: {outputs[0].shape}")
    print(f"  Embedding dimension: {outputs[0].shape[-1]}")
    print(f"  ✅ ONNX model verification successful!")


if __name__ == "__main__":
    try:
        # Parse arguments
        model_id = "iic/speech_campplus_sv_en_voxceleb_16k"  # Default: CAM++ VoxCeleb
        output_path = "speaker_embedding.onnx"

        if len(sys.argv) > 1:
            model_id = sys.argv[1]
        if len(sys.argv) > 2:
            output_path = sys.argv[2]

        print("=" * 70)
        print("WeSpeaker/3D-Speaker ONNX Model Downloader")
        print("=" * 70)

        # Download model
        onnx_path = download_wespeaker_onnx(model_id, output_path)

        print("\n" + "=" * 70)
        print("DOWNLOAD COMPLETE - Python no longer needed for inference!")
        print("=" * 70)
        print(f"\nModel ready at: {output_path}")
        print("Use with Rust ONNX Runtime for speaker embedding inference.")

    except Exception as e:
        print(f"❌ Download failed: {e}", file=sys.stderr)
        import traceback
        traceback.print_exc()
        print("\nMake sure you have modelscope installed:")
        print("  pip install modelscope")
        sys.exit(1)
