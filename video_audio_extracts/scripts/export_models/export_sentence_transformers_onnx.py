#!/usr/bin/env python3
"""Export Sentence-Transformers model to ONNX format."""

import torch
import torch.onnx
from sentence_transformers import SentenceTransformer
from transformers import AutoTokenizer, AutoModel
import argparse
from pathlib import Path

def export_sentence_transformer(model_name: str, output_path: str):
    """Export Sentence-Transformer to ONNX.

    Args:
        model_name: Model name (e.g., "sentence-transformers/all-MiniLM-L6-v2")
        output_path: Path to save the ONNX model
    """
    print(f"Loading Sentence-Transformer model: {model_name}")

    # Load model (force CPU to avoid MPS issues on Apple Silicon)
    model = SentenceTransformer(model_name, device='cpu')
    tokenizer = AutoTokenizer.from_pretrained(model_name)

    # Set to evaluation mode
    model.eval()

    # Get the underlying transformer model
    transformer_model = model[0].auto_model
    transformer_model = transformer_model.to('cpu')  # Ensure CPU device

    # Create dummy inputs (batch_size=1, seq_len=128)
    dummy_text = "This is a sample sentence for ONNX export."
    dummy_inputs = tokenizer(
        dummy_text,
        padding="max_length",
        max_length=128,
        truncation=True,
        return_tensors="pt"
    )

    print(f"Exporting to ONNX: {output_path}")

    # Export to ONNX
    torch.onnx.export(
        transformer_model,
        (dummy_inputs["input_ids"], dummy_inputs["attention_mask"]),
        output_path,
        input_names=["input_ids", "attention_mask"],
        output_names=["last_hidden_state"],
        dynamic_axes={
            "input_ids": {0: "batch_size", 1: "sequence_length"},
            "attention_mask": {0: "batch_size", 1: "sequence_length"},
            "last_hidden_state": {0: "batch_size", 1: "sequence_length"}
        },
        opset_version=14,
        do_constant_folding=True
    )

    print(f"Successfully exported Sentence-Transformer model to {output_path}")

    # Print model info
    import onnx
    onnx_model = onnx.load(output_path)
    print(f"Model inputs: {[inp.name for inp in onnx_model.graph.input]}")
    print(f"Model outputs: {[out.name for out in onnx_model.graph.output]}")

def main():
    parser = argparse.ArgumentParser(description="Export Sentence-Transformers to ONNX")
    parser.add_argument(
        "--model",
        type=str,
        default="sentence-transformers/all-MiniLM-L6-v2",
        help="Model name"
    )
    parser.add_argument(
        "--output",
        type=str,
        default="models/embeddings/all_minilm_l6_v2.onnx",
        help="Output path for ONNX model"
    )

    args = parser.parse_args()

    # Create output directory if needed
    Path(args.output).parent.mkdir(parents=True, exist_ok=True)

    export_sentence_transformer(args.model, args.output)

if __name__ == "__main__":
    main()
