#!/usr/bin/env python3
"""
Merge LoRA adapters into base model for use with Candle/Rust.

After training with train_xtr_code.py, run this to create a merged model
that can be loaded by the sg Rust embedder.

Usage:
    python scripts/merge_lora.py checkpoints/xtr-rust --output checkpoints/xtr-rust-merged

The merged model will contain:
    - config.json
    - tokenizer.json
    - model.safetensors (merged weights)
"""

from __future__ import annotations

import argparse
import shutil
from pathlib import Path

import torch
from peft import PeftModel
from transformers import AutoTokenizer, T5EncoderModel


def merge_lora(adapter_path: Path, output_path: Path, base_model: str = "google/xtr-base-en") -> None:
    """Merge LoRA adapter weights into base model and save."""
    print(f"Loading base model (encoder only): {base_model}")
    base = T5EncoderModel.from_pretrained(base_model)
    tokenizer = AutoTokenizer.from_pretrained(base_model)

    print(f"Loading LoRA adapter: {adapter_path}")
    model = PeftModel.from_pretrained(base, str(adapter_path))

    print("Merging LoRA weights into base model...")
    merged = model.merge_and_unload()

    output_path.mkdir(parents=True, exist_ok=True)

    print(f"Saving merged model to: {output_path}")
    merged.save_pretrained(output_path, safe_serialization=True)
    tokenizer.save_pretrained(output_path)

    # Verify saved files
    expected_files = ["config.json", "tokenizer.json", "model.safetensors"]
    for fname in expected_files:
        fpath = output_path / fname
        if fpath.exists():
            size_mb = fpath.stat().st_size / (1024 * 1024)
            print(f"  {fname}: {size_mb:.1f} MB")
        else:
            print(f"  WARNING: {fname} not found")

    print()
    print("Merged model saved. Use with sg:")
    print(f"  sg index /path/to/code --model-path {output_path}")


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Merge LoRA adapters into base model for Rust/Candle inference",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
    # Merge adapter after training
    python scripts/merge_lora.py checkpoints/xtr-rust -o checkpoints/xtr-rust-merged

    # Use with custom base model
    python scripts/merge_lora.py checkpoints/xtr-rust -o merged --base checkpoints/xtr-code-base
        """,
    )
    parser.add_argument("adapter_path", type=Path, help="Path to LoRA adapter checkpoint")
    parser.add_argument("--output", "-o", type=Path, required=True, help="Output path for merged model")
    parser.add_argument("--base", type=str, default="google/xtr-base-en",
                        help="Base model to merge with (default: google/xtr-base-en)")
    args = parser.parse_args()

    if not args.adapter_path.exists():
        print(f"Error: Adapter path does not exist: {args.adapter_path}")
        return

    if args.output.exists():
        print(f"Warning: Output path exists: {args.output}")
        response = input("Overwrite? [y/N] ")
        if response.lower() != "y":
            print("Aborted.")
            return
        shutil.rmtree(args.output)

    merge_lora(args.adapter_path, args.output, args.base)


if __name__ == "__main__":
    main()
