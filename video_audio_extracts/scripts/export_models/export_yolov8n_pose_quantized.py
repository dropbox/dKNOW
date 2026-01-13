#!/usr/bin/env python3
"""
Export YOLOv8n-Pose model to INT8 quantized ONNX format

This script takes the existing YOLOv8n-Pose ONNX model and applies INT8 dynamic
quantization using ONNX Runtime's quantization tools for faster inference.

Usage:
    python3 scripts/export_models/export_yolov8n_pose_quantized.py
"""

import os
import sys
from pathlib import Path

try:
    from onnxruntime.quantization import quantize_dynamic, QuantType
except ImportError:
    print("Error: onnxruntime package not found. Install with: pip install onnxruntime")
    sys.exit(1)


def export_quantized_pose_model():
    """Quantize existing YOLOv8n-Pose ONNX model to INT8"""

    # Paths
    repo_root = Path(__file__).parent.parent.parent
    model_dir = repo_root / "models" / "pose-estimation"
    input_path = model_dir / "yolov8n-pose.onnx"
    output_path = model_dir / "yolov8n-pose-int8.onnx"

    # Check input exists
    if not input_path.exists():
        print(f"Error: Input model not found at {input_path}")
        print("Please ensure yolov8n-pose.onnx exists in models/pose-estimation/")
        sys.exit(1)

    print(f"Quantizing YOLOv8n-Pose model...")
    print(f"Input:  {input_path}")
    print(f"Output: {output_path}")

    # Apply INT8 dynamic quantization
    # Dynamic quantization: converts weights to INT8, activations quantized at runtime
    # weight_type: INT8 for 8-bit integer weights
    # per_channel: Better accuracy with per-channel quantization
    quantize_dynamic(
        model_input=str(input_path),
        model_output=str(output_path),
        weight_type=QuantType.QInt8,
        per_channel=True,
        reduce_range=False,  # Don't reduce range (better for non-VNNI CPUs)
    )

    # Report results
    if output_path.exists():
        input_size_mb = input_path.stat().st_size / (1024 * 1024)
        output_size_mb = output_path.stat().st_size / (1024 * 1024)
        reduction = ((input_size_mb - output_size_mb) / input_size_mb) * 100

        print(f"\n✓ Successfully exported quantized model to: {output_path}")
        print(f"  Original size: {input_size_mb:.2f} MB")
        print(f"  Quantized size: {output_size_mb:.2f} MB")
        print(f"  Size reduction: {reduction:.1f}%")
        print(f"\nExpected inference speedup: 20-50% (depending on hardware)")
        print(f"Expected accuracy impact: <2% (typical for INT8 quantization)")
    else:
        print(f"Error: Output file not created at {output_path}")
        sys.exit(1)


if __name__ == "__main__":
    export_quantized_pose_model()
