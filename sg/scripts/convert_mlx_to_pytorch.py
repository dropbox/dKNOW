#!/usr/bin/env python3
"""
Convert MLX LoRA weights to PyTorch PEFT format for use with sg eval.

Usage:
    python scripts/convert_mlx_to_pytorch.py checkpoints/xtr-mlx/checkpoint-1000 \
        --output checkpoints/xtr-mlx-merged
"""

from __future__ import annotations

import argparse
import json
import shutil
from pathlib import Path

import numpy as np
import torch
from transformers import AutoTokenizer, T5EncoderModel
from safetensors.torch import save_file


def convert_mlx_to_pytorch(mlx_path: Path, output_path: Path, base_model: str = "google/xtr-base-en"):
    """Convert MLX LoRA weights and merge into base model."""
    print(f"Loading MLX LoRA weights from: {mlx_path}")

    # Load MLX weights
    weights_path = mlx_path / "lora_weights.npz"
    config_path = mlx_path / "lora_config.json"

    if not weights_path.exists():
        print(f"Error: {weights_path} not found")
        return False

    # Load config
    lora_config = {"lora_r": 8, "lora_alpha": 16}  # defaults
    if config_path.exists():
        with open(config_path) as f:
            lora_config = json.load(f)

    lora_r = lora_config.get("lora_r", 8)
    lora_alpha = lora_config.get("lora_alpha", 16)
    scale = lora_alpha / lora_r

    print(f"LoRA config: r={lora_r}, alpha={lora_alpha}, scale={scale:.2f}")

    # Load MLX weights
    mlx_weights = dict(np.load(weights_path))
    print(f"Loaded {len(mlx_weights)} MLX weight tensors")

    for name, arr in list(mlx_weights.items())[:5]:
        print(f"  {name}: {arr.shape}")

    # Load base PyTorch model
    print(f"\nLoading base model: {base_model}")
    model = T5EncoderModel.from_pretrained(base_model)
    tokenizer = AutoTokenizer.from_pretrained(base_model)

    # Map MLX weight names to PyTorch layer indices
    # MLX format: layers.{i}.attention.{query,value}_proj.lora_{A,B}
    # PyTorch format: encoder.block.{i}.layer.0.SelfAttention.{q,v}

    num_merged = 0

    # First pass: collect all lora_A weights (handles any iteration order)
    lora_A_cache = {}
    for name, mlx_arr in mlx_weights.items():
        if "lora_A" not in name:
            continue
        parts = name.split(".")
        if len(parts) < 5:
            continue
        try:
            layer_idx = int(parts[1])
            proj_type = parts[3]  # query_proj or value_proj
        except (ValueError, IndexError):
            continue
        if "query" in proj_type:
            pt_proj = "q"
        elif "value" in proj_type:
            pt_proj = "v"
        else:
            continue
        key = f"{layer_idx}.{pt_proj}"
        lora_A_cache[key] = torch.from_numpy(mlx_arr.copy())

    print(f"  Cached {len(lora_A_cache)} lora_A weights")

    # Second pass: merge using lora_B and cached lora_A
    for name, mlx_arr in mlx_weights.items():
        if "lora_B" not in name:
            continue
        parts = name.split(".")
        if len(parts) < 5:
            print(f"  Skipping unexpected format: {name}")
            continue

        try:
            layer_idx = int(parts[1])
            proj_type = parts[3]  # query_proj or value_proj
        except (ValueError, IndexError):
            print(f"  Skipping: {name}")
            continue

        # Map to PyTorch names
        if "query" in proj_type:
            pt_proj = "q"
        elif "value" in proj_type:
            pt_proj = "v"
        else:
            print(f"  Skipping non-QV projection: {name}")
            continue

        key = f"{layer_idx}.{pt_proj}"
        if key not in lora_A_cache:
            print(f"  Missing lora_A for {key}")
            continue

        # Get PyTorch weight
        pt_weight = model.encoder.block[layer_idx].layer[0].SelfAttention
        if pt_proj == "q":
            base_weight = pt_weight.q.weight.data
        else:
            base_weight = pt_weight.v.weight.data

        # Merge: W' = W + scale * B @ A
        # MLX stores: lora_A [r, in], lora_B [out, r]
        # Merged: [out, in] + scale * [out, r] @ [r, in]

        lora_A = lora_A_cache[key]  # [r, in]
        lora_B = torch.from_numpy(mlx_arr.copy())  # [out, r]

        # Compute delta: scale * B @ A
        delta = scale * (lora_B @ lora_A)

        # Merge into base weight
        base_weight.add_(delta.to(base_weight.dtype))
        num_merged += 1

        if num_merged <= 3:
            print(f"  Merged layer {layer_idx} {pt_proj}: delta norm = {delta.norm():.4f}")

    print(f"\nMerged {num_merged} LoRA projections")

    # Save merged model
    output_path.mkdir(parents=True, exist_ok=True)
    print(f"Saving merged model to: {output_path}")

    model.save_pretrained(output_path, safe_serialization=True)
    tokenizer.save_pretrained(output_path)

    # Verify
    expected = ["config.json", "model.safetensors"]
    for fname in expected:
        fpath = output_path / fname
        if fpath.exists():
            size_mb = fpath.stat().st_size / (1024 * 1024)
            print(f"  {fname}: {size_mb:.1f} MB")
        else:
            print(f"  WARNING: {fname} not found")

    print("\nMerged model saved. Use with sg:")
    print(f"  sg eval --model-path {output_path} --spec eval/code_queries.json")

    return True


def main():
    parser = argparse.ArgumentParser(description="Convert MLX LoRA to PyTorch merged model")
    parser.add_argument("mlx_path", type=Path, help="Path to MLX checkpoint directory")
    parser.add_argument("--output", type=Path, required=True, help="Output path for merged model")
    parser.add_argument("--base-model", default="google/xtr-base-en", help="Base model name")
    args = parser.parse_args()

    convert_mlx_to_pytorch(args.mlx_path, args.output, args.base_model)


if __name__ == "__main__":
    main()
