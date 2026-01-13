#!/usr/bin/env python3
"""Export CLAP (Contrastive Language-Audio Pretraining) audio encoder to ONNX format."""

import torch
import torch.onnx
import argparse
from pathlib import Path

def export_clap_audio(model_name: str, output_path: str, processor_path: str):
    """Export CLAP audio encoder to ONNX.

    Args:
        model_name: Model name (e.g., "laion/clap-htsat-fused")
        output_path: Path to save the ONNX model
        processor_path: Path to save the audio processor config
    """
    print(f"Loading CLAP model: {model_name}")

    try:
        from transformers import ClapModel, ClapProcessor

        # Load CLAP model (force CPU to avoid MPS issues on Apple Silicon)
        model = ClapModel.from_pretrained(model_name)
        processor = ClapProcessor.from_pretrained(model_name)

        # Save processor for later use in Rust
        processor.save_pretrained(processor_path)
        print(f"Saved processor config to {processor_path}")

        # Set to evaluation mode
        model.eval()
        model = model.to('cpu')  # Ensure CPU device

        # Get audio model (includes encoder + pooling)
        audio_model = model.audio_model

        # Create a wrapper that handles is_longer parameter
        class AudioModelWrapper(torch.nn.Module):
            def __init__(self, audio_model):
                super().__init__()
                self.audio_model = audio_model

            def forward(self, input_features):
                # is_longer is a boolean tensor indicating if audio needs padding
                # For ONNX export, we'll always pass False (assuming fixed-length inputs)
                batch_size = input_features.shape[0]
                is_longer = torch.zeros(batch_size, 1, dtype=torch.bool, device=input_features.device)
                outputs = self.audio_model(input_features=input_features, is_longer=is_longer)
                return outputs.pooler_output

        wrapped_model = AudioModelWrapper(audio_model)
        wrapped_model.eval()

        # CLAP expects mel-spectrogram features as input
        # Feature shape: [batch_size, channels, time_steps, mel_bins]
        # Standard CLAP uses 4 channels, 1001 time steps and 64 mel bins
        dummy_input = torch.randn(1, 4, 1001, 64)

        print(f"Exporting to ONNX: {output_path}")

        # Export to ONNX
        torch.onnx.export(
            wrapped_model,
            dummy_input,
            output_path,
            input_names=["input_features"],
            output_names=["pooler_output"],
            dynamic_axes={
                "input_features": {0: "batch_size"},
                "pooler_output": {0: "batch_size"}
            },
            opset_version=14,
            do_constant_folding=True
        )

        print(f"Successfully exported CLAP audio model to {output_path}")

    except ImportError as e:
        print(f"ERROR: Required package not found: {e}")
        print("Install with:")
        print("  pip install transformers")
        return

    # Print model info
    import onnx
    onnx_model = onnx.load(output_path)
    print(f"Model inputs: {[inp.name for inp in onnx_model.graph.input]}")
    print(f"Model outputs: {[out.name for out in onnx_model.graph.output]}")

def main():
    parser = argparse.ArgumentParser(description="Export CLAP to ONNX")
    parser.add_argument(
        "--model",
        type=str,
        default="laion/clap-htsat-fused",
        help="Model name"
    )
    parser.add_argument(
        "--output",
        type=str,
        default="models/embeddings/clap.onnx",
        help="Output path for ONNX model"
    )
    parser.add_argument(
        "--processor-path",
        type=str,
        default="models/embeddings/clap_processor",
        help="Output path for processor config"
    )

    args = parser.parse_args()

    # Create output directories if needed
    Path(args.output).parent.mkdir(parents=True, exist_ok=True)
    Path(args.processor_path).mkdir(parents=True, exist_ok=True)

    export_clap_audio(args.model, args.output, args.processor_path)

if __name__ == "__main__":
    main()
