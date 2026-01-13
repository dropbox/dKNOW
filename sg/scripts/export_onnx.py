#!/usr/bin/env python3
"""Export XTR T5 encoder to ONNX format.

This script exports the T5 encoder from google/xtr-base-en to ONNX format
for use with the Rust ONNX Runtime backend.

Usage:
    python scripts/export_onnx.py [--output OUTPUT_DIR] [--opset OPSET_VERSION]

Requirements:
    pip install torch transformers onnx onnxruntime

The output model will be placed in:
    - OUTPUT_DIR/xtr_encoder.onnx (default: ~/.cache/sg/models/onnx/)
"""

import argparse
import os
from pathlib import Path

import torch
from transformers import T5EncoderModel, AutoTokenizer


def export_t5_encoder(output_dir: Path, opset_version: int = 14):
    """Export T5 encoder to ONNX format."""

    print(f"Loading model google/xtr-base-en...")
    model = T5EncoderModel.from_pretrained("google/xtr-base-en")
    tokenizer = AutoTokenizer.from_pretrained("google/xtr-base-en")

    model.eval()

    # Create output directory
    output_dir.mkdir(parents=True, exist_ok=True)
    output_path = output_dir / "xtr_encoder.onnx"

    # Create dummy input for tracing
    # Max sequence length 512 for documents
    dummy_text = "This is a sample text for ONNX export."
    inputs = tokenizer(
        dummy_text,
        return_tensors="pt",
        padding="max_length",
        max_length=64,  # Use shorter length for export, dynamic axes handle longer
        truncation=True
    )

    input_ids = inputs["input_ids"]
    attention_mask = inputs["attention_mask"]

    print(f"Input shape: {input_ids.shape}")

    # Export to ONNX with dynamic axes for batch size and sequence length
    print(f"Exporting to {output_path}...")

    torch.onnx.export(
        model,
        (input_ids, attention_mask),
        str(output_path),
        opset_version=opset_version,
        input_names=["input_ids", "attention_mask"],
        output_names=["last_hidden_state"],
        dynamic_axes={
            "input_ids": {0: "batch_size", 1: "sequence_length"},
            "attention_mask": {0: "batch_size", 1: "sequence_length"},
            "last_hidden_state": {0: "batch_size", 1: "sequence_length"},
        },
        do_constant_folding=True,
    )

    print(f"Exported model to {output_path}")
    print(f"Model size: {output_path.stat().st_size / 1024 / 1024:.2f} MB")

    # Validate export
    print("\nValidating ONNX model...")
    import onnx
    onnx_model = onnx.load(str(output_path))
    onnx.checker.check_model(onnx_model)
    print("ONNX model validation passed!")

    # Test inference with ONNX Runtime
    print("\nTesting inference with ONNX Runtime...")
    import onnxruntime as ort

    session = ort.InferenceSession(str(output_path))

    # Run inference
    ort_inputs = {
        "input_ids": input_ids.numpy(),
        "attention_mask": attention_mask.numpy(),
    }
    ort_outputs = session.run(None, ort_inputs)

    print(f"ONNX output shape: {ort_outputs[0].shape}")

    # Compare with PyTorch output
    with torch.no_grad():
        pt_output = model(input_ids, attention_mask=attention_mask).last_hidden_state

    pt_output_np = pt_output.numpy()
    onnx_output_np = ort_outputs[0]

    # Check outputs are close
    max_diff = abs(pt_output_np - onnx_output_np).max()
    mean_diff = abs(pt_output_np - onnx_output_np).mean()

    print(f"Max difference: {max_diff:.6f}")
    print(f"Mean difference: {mean_diff:.6f}")

    if max_diff < 1e-4:
        print("Output validation PASSED!")
    else:
        print(f"WARNING: Output difference ({max_diff}) exceeds threshold (1e-4)")

    # Copy tokenizer files
    tokenizer_path = output_dir / "tokenizer.json"
    if not tokenizer_path.exists():
        tokenizer.save_pretrained(str(output_dir))
        print(f"\nTokenizer files saved to {output_dir}")

    print(f"\nExport complete! ONNX model: {output_path}")
    return output_path


def main():
    parser = argparse.ArgumentParser(description="Export XTR T5 encoder to ONNX")
    parser.add_argument(
        "--output",
        type=Path,
        default=Path.home() / ".cache" / "sg" / "models" / "onnx",
        help="Output directory for ONNX model"
    )
    parser.add_argument(
        "--opset",
        type=int,
        default=14,
        help="ONNX opset version (default: 14)"
    )

    args = parser.parse_args()
    export_t5_encoder(args.output, args.opset)


if __name__ == "__main__":
    main()
