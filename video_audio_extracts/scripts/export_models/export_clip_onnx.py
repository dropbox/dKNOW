#!/usr/bin/env python3
"""Export CLIP vision encoder to ONNX format."""

import torch
import torch.onnx
from transformers import CLIPModel, CLIPProcessor
import argparse
from pathlib import Path

def export_clip_vision(model_name: str, output_path: str):
    """Export full CLIP model (vision + text encoders) to ONNX.

    Args:
        model_name: HuggingFace model name (e.g., "openai/clip-vit-base-patch32")
        output_path: Path to save the ONNX model
    """
    print(f"Loading CLIP model: {model_name}")
    model = CLIPModel.from_pretrained(model_name)
    processor = CLIPProcessor.from_pretrained(model_name)

    # Set to evaluation mode
    model.eval()

    # Create dummy inputs matching the Rust code expectations
    # pixel_values: [batch_size, channels=3, height=224, width=224]
    # input_ids: [batch_size, seq_len=7] - dummy text tokens
    # attention_mask: [batch_size, seq_len=7]
    dummy_pixel_values = torch.randn(1, 3, 224, 224)
    dummy_input_ids = torch.tensor([[49406, 320, 2368, 539, 320, 2368, 49407]], dtype=torch.long)
    dummy_attention_mask = torch.ones(1, 7, dtype=torch.long)

    print(f"Exporting to ONNX: {output_path}")

    # Export full CLIP model to ONNX
    # Use dynamo=False to force legacy exporter (torch 2.9 defaults to dynamo=True)
    # Legacy exporter produces ONNX format compatible with ONNX Runtime
    # IMPORTANT: kwargs must be ordered to match model.forward() signature:
    # forward(input_ids, pixel_values, attention_mask, ...)
    # input_names and dynamic_axes must match the kwargs order
    torch.onnx.export(
        model,
        (),  # Empty args tuple
        output_path,
        input_names=["input_ids", "pixel_values", "attention_mask"],
        output_names=["logits_per_image", "logits_per_text", "text_embeds", "image_embeds"],
        dynamic_axes={
            "input_ids": {0: "batch_size"},
            "pixel_values": {0: "batch_size"},
            "attention_mask": {0: "batch_size"},
            "logits_per_image": {0: "batch_size"},
            "logits_per_text": {0: "batch_size"},
            "text_embeds": {0: "batch_size"},
            "image_embeds": {0: "batch_size"}
        },
        opset_version=14,
        do_constant_folding=True,
        dynamo=False,  # Use legacy exporter for ONNX Runtime compatibility
        kwargs={
            "input_ids": dummy_input_ids,
            "pixel_values": dummy_pixel_values,
            "attention_mask": dummy_attention_mask
        }
    )

    print(f"Successfully exported full CLIP model to {output_path}")

    # Print model info
    import onnx
    onnx_model = onnx.load(output_path)
    print(f"Model input shape: {onnx_model.graph.input[0].type.tensor_type.shape}")
    print(f"Model output shape: {onnx_model.graph.output[0].type.tensor_type.shape}")

def main():
    parser = argparse.ArgumentParser(description="Export CLIP to ONNX")
    parser.add_argument(
        "--model",
        type=str,
        default="openai/clip-vit-base-patch32",
        help="HuggingFace model name"
    )
    parser.add_argument(
        "--output",
        type=str,
        default="models/embeddings/clip_vit_b32.onnx",
        help="Output path for ONNX model"
    )

    args = parser.parse_args()

    # Create output directory if needed
    Path(args.output).parent.mkdir(parents=True, exist_ok=True)

    export_clip_vision(args.model, args.output)

if __name__ == "__main__":
    main()
